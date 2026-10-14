//! Convolution: the sound played in a space through the space's answer to
//! a click (its impulse response). Krabton makes the answers itself: a
//! room, a studio, a wooden chamber, a concert hall, a cathedral, a plate, a
//! spring tank, a stairwell, a parking garage, a cave, a tunnel, a forest
//! and a guitar speaker cabinet, each built from its early reflections (an
//! image-source model of the room), a tail of noise that thickens from
//! sparse echoes to a wash and dies away faster in the highs than in the
//! lows, and what makes it itself (a spring's chirps, a stairwell's
//! flutter, a cave's late echoes). *Your File* plays a WAV or FLAC you
//! load. Size stretches the answer, Decay shortens or lengthens it, Reverse
//! turns it around.
//!
//! The convolution itself is partitioned: the first 3968 samples of the
//! answer in blocks of 128 (so it answers within 128 samples), the rest in
//! blocks of 2048 whose sums are spread over the small blocks, so no single
//! moment carries the load. Building a new answer is spread over many
//! blocks too, while the old one keeps playing; then it crossfades.

use crate::dsp::biquad::{self, Coefs, State};
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};
use crate::sample::SampleRef;

pub const SPACE: usize = 0;
pub const SIZE: usize = 1;
pub const DECAY: usize = 2;
pub const PREDELAY: usize = 3;
pub const LOW_CUT: usize = 4;
pub const HIGH_CUT: usize = 5;
pub const WIDTH: usize = 6;
pub const REVERSE: usize = 7;
pub const MIX: usize = 8;

pub const SPACES: &[&str] = &[
    "Small Room",
    "Studio",
    "Wooden Chamber",
    "Concert Hall",
    "Cathedral",
    "Plate",
    "Spring Tank",
    "Stairwell",
    "Parking Garage",
    "Cave",
    "Tunnel",
    "Forest",
    "Speaker Cab",
    "Your File",
];
pub const FILE: usize = 13;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Space", "Convolution", SPACES, 3),
    ParamSpec::lin("Size", "Convolution", 0.5, 2.0, 1.0, Unit::None),
    ParamSpec::lin("Decay", "Convolution", 0.25, 2.0, 1.0, Unit::None),
    ParamSpec::lin("Pre-Delay", "Convolution", 0.0, 300.0, 0.0, Unit::Ms),
    ParamSpec::log("Low Cut", "Tone", 20.0, 1000.0, 30.0, Unit::Hz),
    ParamSpec::log("High Cut", "Tone", 1000.0, 20000.0, 16000.0, Unit::Hz),
    ParamSpec::lin("Width", "Output", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::toggle("Reverse", "Convolution", false),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.35, Unit::Percent),
];

const B1: usize = 128;
const N1: usize = 2 * B1;
const BINS1: usize = N1 / 2 + 1;
const B2: usize = 2048;
const N2: usize = 2 * B2;
const BINS2: usize = N2 / 2 + 1;
/// The part of the answer the small blocks play (the big ones are late by
/// one small block more, so they line up).
const HEAD: usize = 2 * B2 - B1;
const P1: usize = HEAD / B1;
/// Small blocks in a big one.
const STEPS: usize = B2 / B1;
/// The longest answer.
pub const MAX_S: f32 = 5.0;

// ---------------------------------------------------------------- FFT

struct CFft {
    n: usize,
    rev: Vec<usize>,
    cos: Vec<f32>,
    sin: Vec<f32>,
}

impl CFft {
    fn new(n: usize) -> Self {
        let bits = n.trailing_zeros();
        let tau = std::f64::consts::TAU;
        Self {
            n,
            rev: (0..n).map(|i| i.reverse_bits() >> (usize::BITS - bits)).collect(),
            cos: (0..n / 2).map(|k| (tau * k as f64 / n as f64).cos() as f32).collect(),
            sin: (0..n / 2).map(|k| (tau * k as f64 / n as f64).sin() as f32).collect(),
        }
    }

    fn run(&self, re: &mut [f32], im: &mut [f32], inverse: bool) {
        let n = self.n;
        for i in 0..n {
            let j = self.rev[i];
            if j > i {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let sign = if inverse { 1.0 } else { -1.0 };
        let mut size = 2;
        while size <= n {
            let half = size / 2;
            let step = n / size;
            for start in (0..n).step_by(size) {
                for k in 0..half {
                    let (wr, wi) = (self.cos[k * step], sign * self.sin[k * step]);
                    let (a, b) = (start + k, start + k + half);
                    let tr = re[b] * wr - im[b] * wi;
                    let ti = re[b] * wi + im[b] * wr;
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
            }
            size *= 2;
        }
        if inverse {
            let s = 1.0 / n as f32;
            re.iter_mut().for_each(|v| *v *= s);
            im.iter_mut().for_each(|v| *v *= s);
        }
    }
}

/// The transform of a + ib, taken apart into the half spectra of a and b
/// (interleaved re, im).
fn split(re: &[f32], im: &[f32], a: &mut [f32], b: &mut [f32]) {
    let n = re.len();
    for k in 0..=n / 2 {
        let j = (n - k) % n;
        let (zr, zi) = (re[k], im[k]);
        let (cr, ci) = (re[j], -im[j]);
        a[2 * k] = 0.5 * (zr + cr);
        a[2 * k + 1] = 0.5 * (zi + ci);
        let (dr, di) = (0.5 * (zr - cr), 0.5 * (zi - ci));
        b[2 * k] = di;
        b[2 * k + 1] = -dr;
    }
}

/// Half spectra of two real signals put together for one inverse transform.
fn join(a: &[f32], b: &[f32], re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    for k in 0..=n / 2 {
        let (ar, ai, br, bi) = (a[2 * k], a[2 * k + 1], b[2 * k], b[2 * k + 1]);
        re[k] = ar - bi;
        im[k] = ai + br;
        if k > 0 && k < n / 2 {
            // conj(A) + i conj(B)
            re[n - k] = ar + bi;
            im[n - k] = -ai + br;
        }
    }
}

/// acc += x * h, complex, over `bins` bins.
#[inline]
fn mac(acc: &mut [f32], x: &[f32], h: &[f32]) {
    for ((a, x), h) in acc.chunks_exact_mut(2).zip(x.chunks_exact(2)).zip(h.chunks_exact(2)) {
        a[0] += x[0] * h[0] - x[1] * h[1];
        a[1] += x[0] * h[1] + x[1] * h[0];
    }
}

// ---------------------------------------------------------------- spaces

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Room,
    Plate,
    Spring,
    Cab,
    Flutter { period_ms: f32, gain: f32 },
    Cave,
    Forest,
}

#[derive(Clone, Copy)]
struct SpaceDef {
    /// Decay times (s) of the lows, the mids and the highs.
    t60: [f32; 3],
    pre_ms: f32,
    /// How long the tail takes to come up, and to turn from echoes into a wash.
    build_ms: f32,
    mixing_ms: f32,
    /// Room size (m) for the early reflections (0: none), and how loud they are.
    room: [f32; 3],
    early: f32,
    /// How loud the tail is against the early part.
    tail: f32,
    kind: Kind,
}

const fn def(t60: [f32; 3], pre_ms: f32, build_ms: f32, mixing_ms: f32, room: [f32; 3], early: f32, tail: f32, kind: Kind) -> SpaceDef {
    SpaceDef { t60, pre_ms, build_ms, mixing_ms, room, early, tail, kind }
}

const DEFS: [SpaceDef; 13] = [
    def([0.45, 0.4, 0.25], 2.0, 4.0, 25.0, [4.5, 3.8, 2.6], 1.0, 0.6, Kind::Room),
    def([0.7, 0.6, 0.35], 4.0, 8.0, 35.0, [7.0, 5.5, 3.2], 0.8, 0.7, Kind::Room),
    def([1.6, 1.3, 0.6], 6.0, 12.0, 45.0, [9.0, 6.0, 4.0], 0.8, 0.8, Kind::Room),
    def([3.0, 2.3, 1.3], 18.0, 40.0, 80.0, [32.0, 22.0, 14.0], 0.6, 1.0, Kind::Room),
    def([8.0, 6.0, 2.6], 30.0, 90.0, 140.0, [60.0, 28.0, 30.0], 0.5, 1.0, Kind::Room),
    def([2.4, 2.7, 2.1], 0.5, 1.0, 4.0, [0.0; 3], 0.0, 1.0, Kind::Plate),
    def([2.4, 2.0, 0.9], 1.0, 2.0, 10.0, [0.0; 3], 0.0, 0.25, Kind::Spring),
    def([3.2, 2.6, 1.4], 6.0, 10.0, 60.0, [4.0, 3.0, 18.0], 0.8, 0.8, Kind::Flutter { period_ms: 11.0, gain: 0.7 }),
    def([3.8, 3.2, 1.6], 10.0, 30.0, 90.0, [40.0, 30.0, 2.8], 0.9, 0.9, Kind::Flutter { period_ms: 17.0, gain: 0.5 }),
    def([6.5, 4.5, 1.6], 25.0, 60.0, 120.0, [30.0, 20.0, 15.0], 0.6, 0.9, Kind::Cave),
    def([4.5, 3.5, 1.4], 8.0, 25.0, 100.0, [80.0, 6.0, 5.0], 0.7, 0.8, Kind::Flutter { period_ms: 17.5, gain: 0.8 }),
    def([1.1, 0.8, 0.35], 15.0, 20.0, 200.0, [0.0; 3], 0.0, 0.35, Kind::Forest),
    def([0.0; 3], 0.0, 0.0, 0.0, [0.0; 3], 0.0, 0.0, Kind::Cab),
];

struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }

    fn bipolar(&mut self) -> f32 {
        self.next() * 2.0 - 1.0
    }
}

/// How long the answer of a space is (samples).
fn ir_len(space: usize, size: f32, decay: f32, sr: f32, max: usize) -> usize {
    let d = &DEFS[space.min(DEFS.len() - 1)];
    let n = match d.kind {
        Kind::Cab => 0.045 * size * sr,
        _ => (d.pre_ms * 0.001 * size + 1.1 * d.t60.iter().copied().fold(0.0, f32::max) * size * decay) * sr,
    };
    (n as usize).clamp(256, max)
}

/// Writes the tail of a space, samples `from..to` of it.
struct TailGen {
    rng: [Rng; 2],
    low: [f32; 2],
    high: [f32; 2],
}

impl TailGen {
    fn new() -> Self {
        Self { rng: [Rng(0x1234_5677), Rng(0x7654_3211)], low: [0.0; 2], high: [0.0; 2] }
    }

    #[allow(clippy::too_many_arguments)]
    fn run(&mut self, space: usize, size: f32, decay: f32, sr: f32, out: [&mut [f32]; 2], from: usize, to: usize) {
        let d = DEFS[space.min(DEFS.len() - 1)];
        if d.kind == Kind::Cab {
            for o in out {
                o[from..to].fill(0.0);
            }
            return;
        }
        let pre = d.pre_ms * 0.001 * size;
        let build = (d.build_ms * 0.001 * size).max(1e-4);
        let mixing = (d.mixing_ms * 0.001 * size).max(1e-3);
        let t60 = d.t60.map(|t| (t * size * decay).max(0.02));
        let lo_k = 1.0 - (-std::f32::consts::TAU * 300.0 / sr).exp();
        let hi_k = 1.0 - (-std::f32::consts::TAU * 3000.0f32.min(0.4 * sr) / sr).exp();
        let [ol, or] = out;
        for i in from..to {
            let t = i as f32 / sr - pre;
            for c in 0..2 {
                let o = if c == 0 { &mut ol[i] } else { &mut or[i] };
                if t < 0.0 {
                    *o = 0.0;
                    continue;
                }
                // Sparse echoes at first, denser by the square of the time.
                let p = ((t / mixing) * (t / mixing)).clamp(2e-3, 1.0);
                let rng = &mut self.rng[c];
                let n = if rng.next() < p { rng.bipolar() * 1.7 / p.sqrt() } else { 0.0 };
                self.low[c] += lo_k * (n - self.low[c]);
                self.high[c] += hi_k * (n - self.high[c]);
                let (low, high) = (self.low[c], n - self.high[c]);
                let mid = n - low - high;
                let env = |k: usize| (-6.91 * t / t60[k]).exp();
                let onset = 1.0 - (-t / build).exp();
                *o = d.tail * onset * (low * env(0) + mid * env(1) + high * env(2));
            }
        }
    }
}

/// Everything that is not the tail: the early reflections, a spring's
/// chirps, flutter, a cave's late echoes, a cabinet.
fn add_character(space: usize, size: f32, decay: f32, sr: f32, l: &mut [f32], r: &mut [f32]) {
    let d = DEFS[space.min(DEFS.len() - 1)];
    let n = l.len();
    let put = |buf: &mut [f32], at: f32, g: f32| {
        // A reflection is not a click: a little rounded.
        let i = at as usize;
        for (k, w) in [0.25f32, 0.5, 0.25].iter().enumerate() {
            if let Some(v) = buf.get_mut(i + k) {
                *v += g * w;
            }
        }
    };
    if d.room[0] > 0.0 {
        // Image sources in a shoebox, up to the third order; two ears.
        let dims = d.room.map(|m| m * size);
        let src = [0.3 * dims[0], 0.45 * dims[1], 0.4 * dims[2]];
        for (c, ear) in [-0.12f32, 0.12].iter().enumerate() {
            let lis = [0.68 * dims[0], 0.5 * dims[1] + ear, 0.45 * dims[2]];
            for nx in -3i32..=3 {
                for ny in -3i32..=3 {
                    for nz in -3i32..=3 {
                        let order = nx.abs() + ny.abs() + nz.abs();
                        if order == 0 || order > 3 {
                            continue;
                        }
                        let img = |n: i32, k: usize| if n % 2 == 0 { n as f32 * dims[k] + src[k] } else { (n + 1) as f32 * dims[k] - src[k] };
                        let p = [img(nx, 0), img(ny, 1), img(nz, 2)];
                        let dist = ((p[0] - lis[0]).powi(2) + (p[1] - lis[1]).powi(2) + (p[2] - lis[2]).powi(2)).sqrt();
                        let at = (dist / 343.0 + d.pre_ms * 0.0003) * sr;
                        let g = d.early * 0.8f32.powi(order) * (2.0 / dist.max(1.0)) * if (nx + ny + nz) % 2 == 0 { 1.0 } else { -1.0 };
                        put(if c == 0 { &mut *l } else { &mut *r }, at, g);
                    }
                }
            }
        }
    }
    let fall = |t: f32, t60: f32| (-6.91 * t / (t60 * size * decay).max(0.02)).exp();
    match d.kind {
        Kind::Flutter { period_ms, gain } => {
            // Echoes between two parallel walls.
            let period = period_ms * 0.001 * size * sr;
            let mut k = 1;
            loop {
                let at = k as f32 * period + d.pre_ms * 0.001 * size * sr;
                if at as usize >= n {
                    break;
                }
                let g = gain * 0.5 * fall(at / sr, d.t60[1] * 0.6) * if k % 2 == 0 { 1.0 } else { -1.0 };
                put(l, at, g);
                put(r, at + 0.0004 * sr, g);
                k += 1;
            }
        }
        Kind::Cave => {
            // The far walls answer late and dark.
            for (i, (ms, g)) in [(110.0f32, 0.35f32), (230.0, 0.28), (370.0, 0.2), (520.0, 0.14)].iter().enumerate() {
                let at = ms * 0.001 * size * sr;
                let width = (6.0 + 4.0 * i as f32) * 0.001 * sr;
                let mut rng = Rng(0x2468_ace1 + i as u32);
                for j in 0..width as usize {
                    let w = (1.0 - j as f32 / width).powi(2);
                    let v = g * w * rng.bipolar() * 0.3;
                    if let Some(x) = l.get_mut(at as usize + j) {
                        *x += v;
                    }
                    if let Some(x) = r.get_mut(at as usize + j + (0.003 * sr) as usize) {
                        *x -= v;
                    }
                }
            }
        }
        Kind::Spring => {
            // A chirp, highs first, coming round again and again.
            let period = 0.033 * size * sr;
            let len = (0.03 * sr) as usize;
            let mut k = 0;
            loop {
                let at = 0.002 * sr + k as f32 * period;
                if at as usize >= n {
                    break;
                }
                let g = 0.6 * fall(at / sr, 2.0);
                let mut ph = 0.0f32;
                for j in 0..len {
                    let x = j as f32 / len as f32;
                    let f = 3500.0 * (300.0f32 / 3500.0).powf(x);
                    ph += std::f32::consts::TAU * f.min(0.45 * sr) / sr;
                    let v = g * ph.sin() * (std::f32::consts::PI * x).sin() * 0.35;
                    if let Some(s) = l.get_mut(at as usize + j) {
                        *s += v;
                    }
                    if let Some(s) = r.get_mut(at as usize + j + (0.0013 * sr) as usize) {
                        *s += v;
                    }
                }
                k += 1;
            }
        }
        Kind::Forest => {
            // Trees here and there, each a small, dull reflection.
            let mut rng = Rng(0x0f0e_57ee);
            for _ in 0..60 {
                let at = (0.008 + 0.25 * rng.next() * rng.next()) * size * sr;
                let g = 0.25 * (1.0 - rng.next() * 0.7) * fall(at / sr, 1.2);
                let side = rng.next();
                put(l, at, g * side);
                put(r, at + 0.0007 * sr, g * (1.0 - side));
            }
        }
        Kind::Cab => {
            // A closed back cabinet: a resonance below, the cone's presence
            // peak, nothing above 6 kHz. Bigger is lower.
            let s = 1.0 / size;
            let top = 0.45 * sr as f64;
            for (c, buf) in [&mut *l, &mut *r].into_iter().enumerate() {
                let f = |hz: f32| ((hz * s) as f64).min(top);
                let chain: [Coefs; 6] = [
                    biquad::highpass(f(75.0), sr as f64, 0.7),
                    biquad::bell(f(110.0), sr as f64, 1.2, 6.0),
                    biquad::bell(f(420.0), sr as f64, 1.0, -3.0),
                    biquad::bell(f(if c == 0 { 2400.0 } else { 2750.0 }), sr as f64, 1.4, 5.0),
                    biquad::lowpass(f(5200.0), sr as f64, 0.75),
                    biquad::lowpass(f(6400.0), sr as f64, 0.7),
                ];
                let mut st = [State::default(); 6];
                for (i, v) in buf.iter_mut().enumerate() {
                    let mut x = if i == 0 { 1.0f64 } else { 0.0 };
                    for (k, cf) in chain.iter().enumerate() {
                        x = st[k].run(cf, x);
                    }
                    *v = x as f32;
                }
            }
        }
        _ => {}
    }
}

/// The answer of a space at a low rate, for a display: its loudness over
/// time, `points` values from 0 to 1.
pub fn sketch(space: usize, size: f32, decay: f32, reverse: bool, points: usize) -> Vec<f32> {
    let sr = 4000.0;
    let n = ir_len(space.min(DEFS.len() - 1), size, decay, sr, (MAX_S * sr) as usize);
    let mut l = vec![0.0; n];
    let mut r = vec![0.0; n];
    if space < DEFS.len() {
        TailGen::new().run(space, size, decay, sr, [&mut l, &mut r], 0, n);
        add_character(space, size, decay, sr, &mut l, &mut r);
    }
    let mut out = vec![0.0f32; points.max(1)];
    for i in 0..n {
        let k = i * out.len() / n;
        out[k] = out[k].max(0.5 * (l[i].abs() + r[i].abs()));
    }
    if reverse {
        out.reverse();
    }
    let top = out.iter().copied().fold(1e-9, f32::max);
    out.iter_mut().for_each(|v| *v = (*v / top).sqrt());
    out
}

// ---------------------------------------------------------------- engine

#[derive(Clone, Copy, PartialEq)]
struct Key {
    space: usize,
    size: i32,
    decay: i32,
    reverse: bool,
    file: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Build {
    Generate(usize),
    Finish,
    Head,
    Tail(usize),
    Ready,
}

/// The transformed partitions of an answer.
struct Parts {
    head: Vec<f32>,
    tail: Vec<f32>,
    n_tail: usize,
}

impl Parts {
    fn new(max_tail: usize) -> Self {
        Self { head: vec![0.0; P1 * 2 * 2 * BINS1], tail: vec![0.0; max_tail * 2 * 2 * BINS2], n_tail: 0 }
    }
}

pub struct Convolution {
    sr: f32,
    fft1: CFft,
    fft2: CFft,
    max_len: usize,
    max_tail: usize,
    // The answer being built.
    ir: [Vec<f32>; 2],
    ir_len: usize,
    gen: TailGen,
    build: Build,
    building: Option<Key>,
    active: Option<Key>,
    next: Parts,
    parts: Parts,
    file: Option<SampleRef>,
    // Playing.
    pre: [Vec<f32>; 2],
    pre_w: usize,
    in1: [[f32; N1]; 2],
    pos1: usize,
    out1: [[f32; B1]; 2],
    fdl1: Vec<f32>,
    cur1: usize,
    in2: [Vec<f32>; 2],
    pos2: usize,
    fdl2: Vec<f32>,
    cur2: usize,
    acc2: [Vec<f32>; 2],
    done2: usize,
    step: usize,
    play2: [Vec<f32>; 2],
    next2: [Vec<f32>; 2],
    re1: Vec<f32>,
    im1: Vec<f32>,
    re2: Vec<f32>,
    im2: Vec<f32>,
    ya: Vec<f32>,
    yb: Vec<f32>,
    // The crossfade to a new answer: going down, then up.
    gain: f32,
    swap: bool,
    cut_lo: [State; 2],
    cut_hi: [State; 2],
    cut_c: (f32, f32, Coefs, Coefs),
}

impl Convolution {
    pub fn new(sr: f32) -> Self {
        let max_len = (MAX_S * sr) as usize + HEAD;
        let max_tail = (max_len - HEAD).div_ceil(B2) + 1;
        let pre_len = (0.31 * sr) as usize + 2;
        Self {
            sr,
            fft1: CFft::new(N1),
            fft2: CFft::new(N2),
            max_len,
            max_tail,
            ir: [vec![0.0; max_len], vec![0.0; max_len]],
            ir_len: 0,
            gen: TailGen::new(),
            build: Build::Ready,
            building: None,
            active: None,
            next: Parts::new(max_tail),
            parts: Parts::new(max_tail),
            file: None,
            pre: [vec![0.0; pre_len], vec![0.0; pre_len]],
            pre_w: 0,
            in1: [[0.0; N1]; 2],
            pos1: 0,
            out1: [[0.0; B1]; 2],
            fdl1: vec![0.0; P1 * 2 * 2 * BINS1],
            cur1: 0,
            in2: [vec![0.0; N2], vec![0.0; N2]],
            pos2: 0,
            fdl2: vec![0.0; max_tail * 2 * 2 * BINS2],
            cur2: 0,
            acc2: [vec![0.0; 2 * BINS2], vec![0.0; 2 * BINS2]],
            done2: 0,
            step: 0,
            play2: [vec![0.0; B2], vec![0.0; B2]],
            next2: [vec![0.0; B2], vec![0.0; B2]],
            re1: vec![0.0; N1],
            im1: vec![0.0; N1],
            re2: vec![0.0; N2],
            im2: vec![0.0; N2],
            ya: vec![0.0; 2 * BINS2],
            yb: vec![0.0; 2 * BINS2],
            gain: 0.0,
            swap: false,
            cut_lo: [State::default(); 2],
            cut_hi: [State::default(); 2],
            cut_c: (0.0, 0.0, Coefs::IDENTITY, Coefs::IDENTITY),
        }
    }

    /// The file for Your File (from the device; the engine hands it over).
    pub fn set_file(&mut self, file: Option<&SampleRef>) {
        let same = match (&self.file, file) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false,
        };
        if !same {
            self.file = file.cloned();
        }
    }

    fn key(&self, p: &[f32]) -> Key {
        let space = (p[SPACE].round().max(0.0) as usize).min(SPACES.len() - 1);
        Key {
            space,
            size: (p[SIZE].clamp(0.5, 2.0) * 100.0).round() as i32,
            decay: (p[DECAY].clamp(0.25, 2.0) * 100.0).round() as i32,
            reverse: p[REVERSE] >= 0.5,
            file: if space == FILE { self.file.as_ref().map(|f| std::sync::Arc::as_ptr(&f.0) as usize).unwrap_or(0) } else { 0 },
        }
    }

    /// One piece of building the next answer.
    fn build_step(&mut self) {
        let Some(key) = self.building else { return };
        let sr = self.sr;
        let size = key.size as f32 * 0.01;
        let decay = key.decay as f32 * 0.01;
        match self.build {
            Build::Generate(from) => {
                if from == 0 {
                    self.gen = TailGen::new();
                    self.ir_len = if key.space == FILE {
                        match &self.file {
                            Some(f) => ((f.frames() as f32 * sr / f.sample_rate as f32 * size) as usize).clamp(1, self.max_len),
                            None => 0,
                        }
                    } else {
                        ir_len(key.space, size, decay, sr, self.max_len - HEAD)
                    };
                }
                let to = (from + 32768).min(self.ir_len);
                if key.space == FILE {
                    if let Some(f) = &self.file {
                        let ratio = f.sample_rate as f64 / sr as f64 / size as f64;
                        let len_s = self.ir_len as f32 / sr;
                        let k = if decay < 1.0 { 6.91 * (1.0 / decay - 1.0) / len_s } else { 0.0 };
                        for i in from..to {
                            let (a, b) = f.read(i as f64 * ratio);
                            let env = (-k * i as f32 / sr).exp();
                            self.ir[0][i] = a * env;
                            self.ir[1][i] = b * env;
                        }
                    }
                } else {
                    let [l, r] = &mut self.ir;
                    self.gen.run(key.space, size, decay, sr, [l, r], from, to);
                }
                self.build = if to >= self.ir_len { Build::Finish } else { Build::Generate(to) };
            }
            Build::Finish => {
                let n = self.ir_len;
                let [l, r] = &mut self.ir;
                if key.space != FILE {
                    add_character(key.space, size, decay, sr, &mut l[..n], &mut r[..n]);
                }
                // Fade out the very end, so nothing is cut off.
                let fade = if n > 0 { (n / 20).max(1) } else { 0 };
                for i in 0..fade {
                    let g = 0.5 + 0.5 * (std::f32::consts::PI * i as f32 / fade as f32).cos();
                    l[n - fade + i] *= g;
                    r[n - fade + i] *= g;
                }
                if key.reverse && n > 0 {
                    l[..n].reverse();
                    r[..n].reverse();
                    // Start where it starts.
                    let first = (0..n).find(|&i| l[i].abs() > 1e-5 || r[i].abs() > 1e-5).unwrap_or(0);
                    l.copy_within(first..n, 0);
                    r.copy_within(first..n, 0);
                    l[n - first..n].fill(0.0);
                    r[n - first..n].fill(0.0);
                }
                // As loud as any other: the energy of the answer set.
                let e: f32 = (0..n).map(|i| l[i] * l[i] + r[i] * r[i]).sum::<f32>() * 0.5;
                let g = if e > 1e-12 { (0.25 / e).sqrt() } else { 0.0 };
                l[..n].iter_mut().for_each(|v| *v *= g);
                r[..n].iter_mut().for_each(|v| *v *= g);
                l[n..].fill(0.0);
                r[n..].fill(0.0);
                self.build = Build::Head;
            }
            Build::Head => {
                for part in 0..P1 {
                    let o = part * B1;
                    self.re1.fill(0.0);
                    self.im1.fill(0.0);
                    self.re1[..B1].copy_from_slice(&self.ir[0][o..o + B1]);
                    self.im1[..B1].copy_from_slice(&self.ir[1][o..o + B1]);
                    self.fft1.run(&mut self.re1, &mut self.im1, false);
                    let at = part * 4 * BINS1;
                    let (a, b) = self.next.head[at..at + 4 * BINS1].split_at_mut(2 * BINS1);
                    split(&self.re1, &self.im1, a, b);
                }
                self.next.n_tail = self.ir_len.saturating_sub(HEAD).div_ceil(B2).min(self.max_tail);
                self.build = Build::Tail(0);
            }
            Build::Tail(from) => {
                let to = (from + 6).min(self.next.n_tail);
                for m in from..to {
                    let o = HEAD + m * B2;
                    self.re2.fill(0.0);
                    self.im2.fill(0.0);
                    let end = (o + B2).min(self.ir[0].len());
                    self.re2[..end - o].copy_from_slice(&self.ir[0][o..end]);
                    self.im2[..end - o].copy_from_slice(&self.ir[1][o..end]);
                    self.fft2.run(&mut self.re2, &mut self.im2, false);
                    let at = m * 4 * BINS2;
                    let (a, b) = self.next.tail[at..at + 4 * BINS2].split_at_mut(2 * BINS2);
                    split(&self.re2, &self.im2, a, b);
                }
                self.build = if to >= self.next.n_tail { Build::Ready } else { Build::Tail(to) };
                if self.build == Build::Ready {
                    self.swap = true;
                }
            }
            Build::Ready => {}
        }
    }

    /// One small block: its transform, the head's sum, and a share of the tail's.
    fn small_block(&mut self) {
        // The input of this block and the one before.
        for c in 0..2 {
            let (re, im) = (&mut self.re1, &mut self.im1);
            let dst = if c == 0 { re } else { im };
            dst.copy_from_slice(&self.in1[c]);
        }
        self.fft1.run(&mut self.re1, &mut self.im1, false);
        let at = self.cur1 * 4 * BINS1;
        {
            let (a, b) = self.fdl1[at..at + 4 * BINS1].split_at_mut(2 * BINS1);
            split(&self.re1, &self.im1, a, b);
        }
        let ya = &mut self.ya[..2 * BINS1];
        let yb = &mut self.yb[..2 * BINS1];
        ya.fill(0.0);
        yb.fill(0.0);
        for part in 0..P1 {
            let slot = (self.cur1 + P1 - part) % P1;
            let x = &self.fdl1[slot * 4 * BINS1..(slot + 1) * 4 * BINS1];
            let h = &self.parts.head[part * 4 * BINS1..(part + 1) * 4 * BINS1];
            mac(ya, &x[..2 * BINS1], &h[..2 * BINS1]);
            mac(yb, &x[2 * BINS1..], &h[2 * BINS1..]);
        }
        join(ya, yb, &mut self.re1, &mut self.im1);
        self.fft1.run(&mut self.re1, &mut self.im1, true);
        self.out1[0].copy_from_slice(&self.re1[B1..]);
        self.out1[1].copy_from_slice(&self.im1[B1..]);
        self.cur1 = (self.cur1 + 1) % P1;
        for c in 0..2 {
            self.in1[c].copy_within(B1.., 0);
        }
        // The tail: this step's share of the sum for the block after next.
        let n_tail = self.parts.n_tail;
        let target = (self.step + 1) * n_tail / STEPS;
        for m in self.done2..target {
            let slot = (self.cur2 + self.max_tail - m) % self.max_tail;
            let x = &self.fdl2[slot * 4 * BINS2..(slot + 1) * 4 * BINS2];
            let h = &self.parts.tail[m * 4 * BINS2..(m + 1) * 4 * BINS2];
            let (a0, a1) = self.acc2.split_at_mut(1);
            mac(&mut a0[0], &x[..2 * BINS2], &h[..2 * BINS2]);
            mac(&mut a1[0], &x[2 * BINS2..], &h[2 * BINS2..]);
        }
        self.done2 = target.max(self.done2);
        self.step += 1;
        if self.step == STEPS {
            join(&self.acc2[0], &self.acc2[1], &mut self.re2, &mut self.im2);
            self.fft2.run(&mut self.re2, &mut self.im2, true);
            self.next2[0].copy_from_slice(&self.re2[B2..]);
            self.next2[1].copy_from_slice(&self.im2[B2..]);
        }
    }

    /// A big block is full: the tail moves on.
    fn big_block(&mut self) {
        std::mem::swap(&mut self.play2, &mut self.next2);
        self.re2.copy_from_slice(&self.in2[0]);
        self.im2.copy_from_slice(&self.in2[1]);
        self.fft2.run(&mut self.re2, &mut self.im2, false);
        self.cur2 = (self.cur2 + 1) % self.max_tail;
        let at = self.cur2 * 4 * BINS2;
        {
            let (a, b) = self.fdl2[at..at + 4 * BINS2].split_at_mut(2 * BINS2);
            split(&self.re2, &self.im2, a, b);
        }
        for c in 0..2 {
            self.in2[c].copy_within(B2.., 0);
            self.acc2[c].fill(0.0);
        }
        self.done2 = 0;
        self.step = 0;
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let key = self.key(p);
        if Some(key) != self.building && (Some(key) != self.active || self.building.is_some()) {
            self.building = Some(key);
            self.build = Build::Generate(0);
            // A crossfade on its way is called off: the old answer plays on.
            self.swap = false;
        }
        if self.building.is_some() && self.build != Build::Ready {
            self.build_step();
        }
        let (lo, hi) = (p[LOW_CUT].clamp(10.0, 2000.0), p[HIGH_CUT].clamp(500.0, 0.45 * sr));
        if (self.cut_c.0 - lo).abs() > 1e-3 || (self.cut_c.1 - hi).abs() > 1e-3 {
            self.cut_c = (lo, hi, biquad::highpass(lo as f64, sr as f64, 0.7), biquad::lowpass(hi as f64, sr as f64, 0.7));
        }
        let pre = ((p[PREDELAY].clamp(0.0, 300.0) * 0.001 * sr) as usize).min(self.pre[0].len() - 1);
        let width = p[WIDTH].clamp(0.0, 1.0);
        let mix = p[MIX].clamp(0.0, 1.0);
        let fade = 1.0 / (0.01 * sr);
        let pl = self.pre[0].len();
        for i in 0..n {
            let (xl, xr) = (l[i], r[i]);
            self.pre[0][self.pre_w] = xl;
            self.pre[1][self.pre_w] = xr;
            let rd = (self.pre_w + pl - pre) % pl;
            self.pre_w = (self.pre_w + 1) % pl;
            self.in1[0][B1 + self.pos1] = self.pre[0][rd];
            self.in1[1][B1 + self.pos1] = self.pre[1][rd];
            self.in2[0][B2 + self.pos2] = self.pre[0][rd];
            self.in2[1][B2 + self.pos2] = self.pre[1][rd];
            let mut wl = self.out1[0][self.pos1] + self.play2[0][self.pos2];
            let mut wr = self.out1[1][self.pos1] + self.play2[1][self.pos2];
            self.pos1 += 1;
            self.pos2 += 1;
            if self.pos1 == B1 {
                self.small_block();
                self.pos1 = 0;
            }
            if self.pos2 == B2 {
                self.big_block();
                self.pos2 = 0;
            }
            // A new answer: down, change, up.
            if self.swap {
                self.gain -= fade;
                if self.gain <= 0.0 {
                    self.gain = 0.0;
                    std::mem::swap(&mut self.parts, &mut self.next);
                    self.active = self.building.take();
                    self.swap = false;
                    // The tail's sum so far was with the old answer.
                    for c in 0..2 {
                        self.acc2[c].fill(0.0);
                    }
                    self.done2 = 0;
                }
            } else if self.gain < 1.0 && self.active.is_some() {
                self.gain = (self.gain + fade).min(1.0);
            }
            let g = self.gain;
            wl = self.cut_lo[0].run(&self.cut_c.2, (wl * g) as f64) as f32;
            wr = self.cut_lo[1].run(&self.cut_c.2, (wr * g) as f64) as f32;
            wl = self.cut_hi[0].run(&self.cut_c.3, wl as f64) as f32;
            wr = self.cut_hi[1].run(&self.cut_c.3, wr as f64) as f32;
            let mid = 0.5 * (wl + wr);
            let side = 0.5 * (wl - wr) * width;
            l[i] = xl * (1.0 - mix) + (mid + side) * mix;
            r[i] = xr * (1.0 - mix) + (mid - side) * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    /// Runs until the answer is built, then a click through it.
    fn response(p: &[f32], file: Option<&SampleRef>, secs: f32) -> (Vec<f32>, Vec<f32>) {
        let mut cv = Convolution::new(48000.0);
        cv.set_file(file);
        let mut l = vec![0.0f32; 256];
        let mut r = vec![0.0f32; 256];
        for _ in 0..400 {
            l.fill(0.0);
            r.fill(0.0);
            cv.process(p, &mut l, &mut r, &ctx());
        }
        assert!(cv.active.is_some() && cv.gain > 0.99, "not built");
        let n = (secs * 48000.0) as usize;
        let mut l = vec![0.0; n];
        let mut r = vec![0.0; n];
        l[0] = 1.0;
        r[0] = 1.0;
        for (cl, cr) in l.chunks_mut(100).zip(r.chunks_mut(100)) {
            cv.process(p, cl, cr, &ctx());
        }
        (l, r)
    }

    #[test]
    fn transforms_of_two_real_signals_come_apart_and_back() {
        let f = CFft::new(16);
        let a: Vec<f32> = (0..16).map(|i| (i as f32 * 0.7).sin()).collect();
        let b: Vec<f32> = (0..16).map(|i| (i as f32 * 0.3).cos() * 0.5).collect();
        let (mut re, mut im) = (a.clone(), b.clone());
        f.run(&mut re, &mut im, false);
        let mut sa = vec![0.0; 18];
        let mut sb = vec![0.0; 18];
        split(&re, &im, &mut sa, &mut sb);
        join(&sa, &sb, &mut re, &mut im);
        f.run(&mut re, &mut im, true);
        for i in 0..16 {
            assert!((re[i] - a[i]).abs() < 1e-4 && (im[i] - b[i]).abs() < 1e-4, "{i}");
        }
    }

    #[test]
    fn a_click_plays_the_answer_itself() {
        // A short made-up answer: the convolution of a click must be it,
        // late by one small block, head and tail in line.
        let mut p = crate::params::defaults(SPECS);
        p[SPACE] = FILE as f32;
        p[MIX] = 1.0;
        p[LOW_CUT] = 20.0;
        p[HIGH_CUT] = 20000.0;
        // Smooth bumps (no clicks, so the tone filters leave them be).
        let n = 12000;
        let bump = |i: usize| {
            let (k, j) = (i / 997, i % 997);
            if j < 32 {
                (1.0 - k as f32 * 0.08) * (std::f32::consts::PI * j as f32 / 32.0).sin().powi(2)
            } else {
                0.0
            }
        };
        let ir: Vec<f32> = (0..n).map(bump).collect();
        let file = SampleRef::new(crate::sample::SampleData::from_buffers("ir", None, 48000, ir.clone(), ir.clone()));
        let (l, _) = response(&p, Some(&file), 0.5);
        let e: f32 = ir.iter().map(|v| v * v).sum::<f32>();
        let g = (0.25 / e).sqrt();
        for k in [16usize, 997 + 16, 2991 + 16, 3988 + 16, 4985 + 16, 7976 + 16, 10967 + 16] {
            let want = ir[k] * g;
            let got = l[B1 + k];
            assert!((got - want).abs() < 0.03 * want.abs().max(0.05), "at {k}: {got}, wants {want}");
        }
        // Silence between the clicks.
        assert!(l[B1 + 500].abs() < 0.01 && l[B1 + 5500].abs() < 0.01);
    }

    #[test]
    fn every_space_answers_and_dies_away() {
        for space in 0..FILE {
            let mut p = crate::params::defaults(SPECS);
            p[SPACE] = space as f32;
            p[MIX] = 1.0;
            let (l, r) = response(&p, None, 6.0);
            assert!(l.iter().chain(&r).all(|v| v.is_finite()), "{}", SPACES[space]);
            let early = rms(&l[..24000]);
            assert!(early > 1e-4, "{}: {early}", SPACES[space]);
            assert!(rms(&l[(5.6 * 48000.0) as usize..]) < 1e-3, "{} rings past its end", SPACES[space]);
        }
    }

    #[test]
    fn your_file_without_a_file_is_quiet() {
        let mut p = crate::params::defaults(SPECS);
        p[SPACE] = FILE as f32;
        p[MIX] = 1.0;
        let mut cv = Convolution::new(48000.0);
        let mut l = tone(300.0, 1.0, 0.5);
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            cv.process(&p, cl, cr, &ctx());
        }
        assert!(peak(&l) < 1e-4);
    }

    /// How long each reverb takes for ten seconds of sound (run with
    /// --ignored --nocapture in release).
    #[test]
    #[ignore]
    fn cost() {
        use crate::dsp::effects::{EffectKind, EffectState};
        for kind in [EffectKind::Sky, EffectKind::Massive, EffectKind::Smear, EffectKind::Convolution, EffectKind::Hall] {
            let mut st = EffectState::new(kind, 48000.0);
            let mut p = crate::params::defaults(kind.specs());
            if kind == EffectKind::Convolution {
                p[SPACE] = 4.0;
            }
            let mut l = vec![0.1f32; 256];
            let mut r = vec![0.1f32; 256];
            for _ in 0..200 {
                st.process(&p, &mut l, &mut r, &ctx());
            }
            let t = std::time::Instant::now();
            for _ in 0..(10 * 48000 / 256) {
                st.process(&p, &mut l, &mut r, &ctx());
            }
            eprintln!("{:?}: {:.1} ms for 10 s", kind, t.elapsed().as_secs_f64() * 1000.0);
        }
    }

    #[test]
    fn the_sketch_falls() {
        let s = sketch(3, 1.0, 1.0, false, 100);
        assert!(s[5] > s[90], "{} {}", s[5], s[90]);
        let r = sketch(3, 1.0, 1.0, true, 100);
        assert!(r[95] > r[10]);
    }
}
