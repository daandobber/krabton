//! Drone: a deep, slowly breathing drone and ambient instrument. Hold a key
//! or two and it keeps moving for minutes. Every note has four layers, each
//! with its own level:
//!
//! - Tanpura: four plucked strings (Pa, Ma or Ni, then Sa, Sa and the low
//!   Sa), plucked one after another in the classic slow cycle. Each string
//!   is a Karplus-Strong loop (Kevin Karplus and Alex Strong, 1983, with the
//!   tuning and decay refinements of David Jaffe and Julius O. Smith, 1983).
//!   The jawari, the curved bridge, is a delay that gets shorter while the
//!   string presses on the bridge: the displacement-dependent bridge of
//!   waveguide sitar and tanpura models (after C. V. Raman's 1921 account
//!   of the jawari and the sitar models of Matti Karjalainen, Vesa Valimaki
//!   and Sadjad Siddiq). A loud string buzzes brightly, and the buzz sweeps
//!   down through the harmonics as it rings out.
//! - Pad: Paul Nasca's PADsynth (from ZynAddSubFX). Every harmonic is a band
//!   of noise with a Gaussian spectrum that gets wider with the harmonic
//!   number, made once by giving random phases to a Gaussian profile and
//!   running an inverse FFT. The band is kept at baseband (as I and Q) and
//!   shifted up to each harmonic by its own carrier; reading it faster makes
//!   it wider, so Bandwidth, Harmonics and Odd/Even all work live without
//!   new tables.
//! - Swell: pairs of slightly detuned sine oscillators that phase-modulate
//!   each other, each with a slow envelope that swells and fades on its own,
//!   the way paired-voice organic drone synths (like the Lyra-8) behave.
//! - Air: noise through resonant band-passes (Andrew Simper's state-variable
//!   filter) on the note's harmonics that slowly wander, like wind or breath.
//!
//! Evolve lets everything drift by itself (pitch, brightness, the balance of
//! the layers). Space is Krabton's Hall reverb. Original code.

use super::effects::FxCtx;
use super::fx::hall::{self, Hall};
use super::{db_to_gain, fast_tanh, midi_to_freq, pan_gains, OnePole, Rng, Svf};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

pub const TANPURA: usize = 0;
pub const FIRST_STRING: usize = 1;
pub const PLUCK_RATE: usize = 2;
pub const JAWARI: usize = 3;
pub const STRING_DECAY: usize = 4;
pub const PAD: usize = 5;
pub const BANDWIDTH: usize = 6;
pub const HARMONICS: usize = 7;
pub const ODD_EVEN: usize = 8;
pub const SWELL: usize = 9;
pub const FM: usize = 10;
pub const SWELL_TIME: usize = 11;
pub const AIR: usize = 12;
pub const AIR_RESONANCE: usize = 13;
pub const EVOLVE: usize = 14;
pub const DETUNE: usize = 15;
pub const ATTACK: usize = 16;
pub const RELEASE: usize = 17;
pub const TONE: usize = 18;
pub const SPACE: usize = 19;
pub const WIDTH: usize = 20;
pub const VOICES: usize = 21;
pub const VOLUME: usize = 22;

/// The first tanpura string: the fifth, the fourth or the major seventh.
pub const FIRST_STRINGS: &[&str] = &["Pa", "Ma", "Ni"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Tanpura", "Tanpura", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::choice("First String", "Tanpura", FIRST_STRINGS, 0),
    ParamSpec::log("Pluck Rate", "Tanpura", 0.2, 4.0, 1.0, Unit::Hz),
    ParamSpec::lin("Jawari", "Tanpura", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::log("String Decay", "Tanpura", 1.0, 30.0, 8.0, Unit::Sec),
    ParamSpec::lin("Pad", "Pad", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::log("Bandwidth", "Pad", 1.0, 200.0, 40.0, Unit::Cents),
    ParamSpec::lin("Harmonics", "Pad", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Odd/Even", "Pad", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Swell", "Swell", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("FM", "Swell", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::log("Swell Time", "Swell", 0.5, 30.0, 6.0, Unit::Sec),
    ParamSpec::lin("Air", "Air", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Air Resonance", "Air", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Evolve", "Motion", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Detune", "Motion", 0.0, 30.0, 6.0, Unit::Cents),
    ParamSpec::log("Attack", "Envelope", 0.01, 20.0, 2.0, Unit::Sec),
    ParamSpec::log("Release", "Envelope", 0.05, 30.0, 6.0, Unit::Sec),
    ParamSpec::lin("Tone", "Output", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Space", "Output", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Width", "Output", 0.0, 1.5, 1.0, Unit::Percent),
    ParamSpec::int("Voices", "Output", 1.0, 6.0, 4.0, Unit::None),
    ParamSpec::lin("Volume", "Output", -24.0, 6.0, 0.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Tanpura",
        "Tanpura Ma",
        "Raga Morning",
        "Deep Space",
        "Organ Drone",
        "Choir of Ghosts",
        "Swell Field",
        "Wind Over Water",
        "Dark Matter",
        "Shimmer Fields",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    let (pa, ma, ni) = (0.0, 1.0, 2.0);
    match name {
        "Tanpura" => set(&[(TANPURA, 1.0), (FIRST_STRING, pa), (PLUCK_RATE, 1.0), (JAWARI, 0.7), (STRING_DECAY, 9.0), (PAD, 0.12), (BANDWIDTH, 20.0), (SWELL, 0.08), (AIR, 0.05), (EVOLVE, 0.35), (DETUNE, 3.0), (ATTACK, 0.01), (RELEASE, 4.0), (SPACE, 0.35), (VOICES, 2.0)]),
        "Tanpura Ma" => set(&[(TANPURA, 1.0), (FIRST_STRING, ma), (PLUCK_RATE, 0.85), (JAWARI, 0.8), (STRING_DECAY, 11.0), (PAD, 0.15), (BANDWIDTH, 15.0), (HARMONICS, 0.4), (SWELL, 0.1), (FM, 0.15), (AIR, 0.06), (EVOLVE, 0.4), (DETUNE, 4.0), (ATTACK, 0.01), (RELEASE, 5.0), (SPACE, 0.4), (VOICES, 2.0)]),
        "Raga Morning" => set(&[(TANPURA, 0.75), (FIRST_STRING, pa), (PLUCK_RATE, 1.2), (JAWARI, 0.55), (STRING_DECAY, 8.0), (PAD, 0.45), (BANDWIDTH, 25.0), (HARMONICS, 0.55), (SWELL, 0.15), (AIR, 0.15), (AIR_RESONANCE, 0.7), (EVOLVE, 0.45), (ATTACK, 1.5), (RELEASE, 6.0), (TONE, 0.15), (SPACE, 0.5)]),
        "Deep Space" => set(&[(TANPURA, 0.0), (PAD, 0.8), (BANDWIDTH, 80.0), (HARMONICS, 0.35), (ODD_EVEN, 0.45), (SWELL, 0.4), (FM, 0.2), (SWELL_TIME, 12.0), (AIR, 0.2), (AIR_RESONANCE, 0.5), (EVOLVE, 0.65), (DETUNE, 8.0), (ATTACK, 8.0), (RELEASE, 14.0), (TONE, -0.3), (SPACE, 0.9), (WIDTH, 1.3)]),
        "Organ Drone" => set(&[(TANPURA, 0.0), (PAD, 0.85), (BANDWIDTH, 5.0), (HARMONICS, 0.7), (ODD_EVEN, 0.3), (SWELL, 0.3), (FM, 0.08), (SWELL_TIME, 4.0), (AIR, 0.06), (AIR_RESONANCE, 0.4), (EVOLVE, 0.25), (DETUNE, 3.0), (ATTACK, 0.8), (RELEASE, 3.0), (SPACE, 0.45), (WIDTH, 0.9)]),
        "Choir of Ghosts" => set(&[(TANPURA, 0.0), (PAD, 0.9), (BANDWIDTH, 60.0), (HARMONICS, 0.6), (ODD_EVEN, 0.5), (SWELL, 0.15), (FM, 0.1), (SWELL_TIME, 9.0), (AIR, 0.3), (AIR_RESONANCE, 0.75), (EVOLVE, 0.6), (DETUNE, 7.0), (ATTACK, 4.0), (RELEASE, 10.0), (SPACE, 0.75), (WIDTH, 1.2)]),
        "Swell Field" => set(&[(TANPURA, 0.0), (PAD, 0.25), (BANDWIDTH, 30.0), (SWELL, 0.9), (FM, 0.45), (SWELL_TIME, 8.0), (AIR, 0.1), (EVOLVE, 0.7), (DETUNE, 9.0), (ATTACK, 3.0), (RELEASE, 10.0), (SPACE, 0.6), (WIDTH, 1.2)]),
        "Wind Over Water" => set(&[(TANPURA, 0.15), (FIRST_STRING, ni), (PLUCK_RATE, 0.3), (JAWARI, 0.3), (STRING_DECAY, 14.0), (PAD, 0.2), (BANDWIDTH, 120.0), (HARMONICS, 0.45), (SWELL, 0.15), (FM, 0.1), (SWELL_TIME, 14.0), (AIR, 0.9), (AIR_RESONANCE, 0.55), (EVOLVE, 0.8), (DETUNE, 5.0), (ATTACK, 5.0), (RELEASE, 12.0), (SPACE, 0.7), (WIDTH, 1.4)]),
        "Dark Matter" => set(&[(TANPURA, 0.0), (PAD, 0.6), (BANDWIDTH, 30.0), (HARMONICS, 0.15), (ODD_EVEN, 0.35), (SWELL, 0.6), (FM, 0.7), (SWELL_TIME, 15.0), (AIR, 0.15), (AIR_RESONANCE, 0.85), (EVOLVE, 0.5), (DETUNE, 4.0), (ATTACK, 6.0), (RELEASE, 14.0), (TONE, -0.6), (SPACE, 0.8)]),
        "Shimmer Fields" => set(&[(TANPURA, 0.35), (FIRST_STRING, pa), (PLUCK_RATE, 1.4), (JAWARI, 0.9), (STRING_DECAY, 12.0), (PAD, 0.7), (BANDWIDTH, 50.0), (HARMONICS, 0.9), (ODD_EVEN, 0.65), (SWELL, 0.2), (FM, 0.25), (AIR, 0.2), (AIR_RESONANCE, 0.8), (EVOLVE, 0.6), (ATTACK, 3.0), (RELEASE, 10.0), (TONE, 0.4), (SPACE, 0.85), (WIDTH, 1.3)]),
        _ => {}
    }
    p
}

/// Voice slots: held notes (up to six) plus release tails.
const MAX_VOICES: usize = 8;
/// Samples per control step (envelopes, drift, pad bands).
const CTRL: usize = 32;
/// Samples per output chunk (tone, width, reverb).
const CHUNK: usize = 256;
/// Pad harmonics per note.
const MAX_H: usize = 40;
/// Pad noise band table: 2^16 complex samples, Gaussian spectrum of this
/// width (standard deviation, in bins).
const PAD_BITS: u32 = 16;
const PAD_N: usize = 1 << PAD_BITS;
const PAD_SIGMA: f32 = 200.0;
const SINE_N: usize = 1024;
const AIR_BANDS: usize = 6;
const AIR_HARMONICS: [f32; AIR_BANDS] = [1.0, 2.0, 3.0, 4.0, 6.0, 8.0];
/// Tanpura plucking order: the first string, Sa, Sa, then the low Sa,
/// with a longer breath after the low Sa (in pluck periods).
const PLUCK_GAPS: [f32; 4] = [1.0, 1.0, 1.0, 1.5];
const STRING_PANS: [f32; 4] = [-0.35, -0.1, 0.15, 0.35];
const STRING_LEVELS: [f32; 4] = [0.9, 1.0, 1.0, 1.15];
/// Loss filter of the strings (one-pole, a = 1 - b) and its phase delay.
const LOSS_A: f32 = 0.9;
const LOSS_DELAY: f32 = (1.0 - LOSS_A) / LOSS_A;
/// Layer gains, so the four levels sit together.
const TANPURA_GAIN: f32 = 0.55;
const PAD_GAIN: f32 = 0.32;
const SWELL_GAIN: f32 = 0.22;
const AIR_GAIN: f32 = 0.1;

fn sine_table() -> &'static [f32] {
    static T: OnceLock<Vec<f32>> = OnceLock::new();
    T.get_or_init(|| (0..=SINE_N).map(|i| (i as f32 / SINE_N as f32 * TAU).sin()).collect())
}

/// Sine of a phase in turns (any value above -8).
#[inline]
fn sin_turns(t: &[f32], turns: f32) -> f32 {
    let x = turns + 8.0;
    let x = (x - x as i32 as f32) * SINE_N as f32;
    let i = (x as usize).min(SINE_N - 1);
    let f = x - i as f32;
    t[i] + (t[i + 1] - t[i]) * f
}

/// The PADsynth band, built once: interleaved I and Q, with one extra
/// sample at the end so reads can interpolate across the wrap.
fn pad_table() -> &'static [f32] {
    static T: OnceLock<Vec<f32>> = OnceLock::new();
    T.get_or_init(build_pad_table)
}

fn build_pad_table() -> Vec<f32> {
    let n = PAD_N;
    let mut re = vec![0.0f64; n];
    let mut im = vec![0.0f64; n];
    let mut rng = Rng::new(0x0DA0_5EED);
    // Gaussian amplitudes with random phases, around 0 Hz.
    let sigma = PAD_SIGMA as f64;
    let reach = (sigma * 4.5) as i64;
    for k in -reach..=reach {
        let a = (-0.5 * (k as f64 / sigma).powi(2)).exp();
        let ph = rng.unipolar() as f64 * std::f64::consts::TAU;
        let i = k.rem_euclid(n as i64) as usize;
        re[i] = a * ph.cos();
        im[i] = a * ph.sin();
    }
    inverse_fft(&mut re, &mut im);
    let power = re.iter().zip(&im).map(|(a, b)| a * a + b * b).sum::<f64>() / n as f64;
    let g = 1.0 / power.sqrt().max(1e-30);
    let mut t = Vec::with_capacity(2 * (n + 1));
    for i in 0..=n {
        t.push((re[i % n] * g) as f32);
        t.push((im[i % n] * g) as f32);
    }
    t
}

/// In-place radix-2 inverse FFT (no scaling); the length is a power of two.
fn inverse_fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if j > i {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut size = 2;
    while size <= n {
        let half = size / 2;
        let step = std::f64::consts::TAU / size as f64;
        for k in 0..half {
            let (wi, wr) = (step * k as f64).sin_cos();
            for start in (0..n).step_by(size) {
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
}

/// I and Q of the pad band at a 16.16 fixed-point position.
#[inline]
fn pad_read(t: &[f32], pos: u32) -> (f32, f32) {
    let i = (pos >> 16) as usize * 2;
    let f = (pos & 0xFFFF) as f32 * (1.0 / 65536.0);
    (t[i] + (t[i + 2] - t[i]) * f, t[i + 1] + (t[i + 3] - t[i + 1]) * f)
}

#[inline]
fn cents(c: f32) -> f32 {
    (c * (std::f32::consts::LN_2 / 1200.0)).exp()
}

#[inline]
fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// A slow random walk between -1 and 1: glides to a new target every so
/// often, with smooth turns.
#[derive(Clone, Copy)]
struct Wander {
    from: f32,
    to: f32,
    t: f32,
    dur: f32,
}

impl Wander {
    fn new(rng: &mut Rng) -> Self {
        Self { from: rng.bipolar(), to: rng.bipolar(), t: rng.unipolar(), dur: 0.0 }
    }

    /// Moves on by `secs`, taking `min..max` seconds per glide.
    fn step(&mut self, rng: &mut Rng, secs: f32, min: f32, max: f32) -> f32 {
        if self.dur <= 0.0 {
            self.dur = min + (max - min) * rng.unipolar();
        }
        self.t += secs / self.dur;
        if self.t >= 1.0 {
            self.t = 0.0;
            self.from = self.to;
            self.to = rng.bipolar();
            self.dur = min + (max - min) * rng.unipolar();
        }
        self.from + (self.to - self.from) * smoothstep(self.t)
    }
}

/// A swell envelope that comes and goes on its own: it glides to a new
/// level, holds there a while, then glides again (in units of Swell Time).
#[derive(Clone, Copy)]
struct SwellEnv {
    from: f32,
    to: f32,
    t: f32,
    dur: f32,
    hold: bool,
}

impl SwellEnv {
    fn new(rng: &mut Rng) -> Self {
        Self { from: 0.0, to: 0.4 + 0.6 * rng.unipolar(), t: 0.0, dur: 0.3 + 0.7 * rng.unipolar(), hold: false }
    }

    fn step(&mut self, rng: &mut Rng, secs: f32, time: f32) -> f32 {
        self.t += secs / (self.dur * time);
        if self.t >= 1.0 {
            self.t = 0.0;
            self.from = self.to;
            if self.hold {
                // Loud ones tend to fade, quiet ones to swell.
                let r = rng.unipolar();
                self.to = if self.from > 0.5 { 0.03 + 0.45 * r } else { 0.5 + 0.5 * r };
                self.dur = 0.6 + 0.8 * rng.unipolar();
            } else {
                self.dur = 0.15 + 0.7 * rng.unipolar();
            }
            self.hold = !self.hold;
        }
        if self.hold {
            self.from
        } else {
            self.from + (self.to - self.from) * smoothstep(self.t)
        }
    }
}

/// A gentle DC blocker (about 5 Hz).
#[derive(Clone, Copy, Default)]
struct Dc {
    x1: f32,
    y1: f32,
}

impl Dc {
    #[inline]
    fn tick(&mut self, x: f32, r: f32) -> f32 {
        let y = x - self.x1 + r * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }
}

/// Settings shared by all voices for one render call.
struct Glob<'a> {
    sr: f32,
    sine: &'a [f32],
    pad_tab: &'a [f32],
    ln: &'a [f32; MAX_H + 1],
    widen: &'a [f32; MAX_H + 1],
    level: [f32; 4],
    first: f32,
    rate: f32,
    jawari: f32,
    decay: f32,
    bw_rel: f32,
    harm: f32,
    odd_even: f32,
    fm: f32,
    swell_time: f32,
    air_q: f32,
    evolve: f32,
    detune: f32,
    att_inc: f32,
    rel_k: f32,
    steal_k: f32,
    bridge_a: f32,
}

/// One tanpura string: a Karplus-Strong loop with a jawari bridge.
struct KsString {
    buf: Vec<f32>,
    mask: usize,
    pos: usize,
    delay: f32,
    lp: f32,
    /// Smoothed displacement where the string meets the bridge.
    bridge: f32,
    contact_avg: f32,
    exc_left: usize,
    exc_len: usize,
    exc_amp: f32,
    exc_lp: f32,
    exc_a: f32,
    /// How strongly it rings, for the display.
    env: f32,
}

impl KsString {
    fn new(max_period: usize) -> Self {
        let len = (max_period + 8).next_power_of_two();
        Self {
            buf: vec![0.0; len],
            mask: len - 1,
            pos: 0,
            delay: 100.0,
            lp: 0.0,
            bridge: 0.0,
            contact_avg: 0.0,
            exc_left: 0,
            exc_len: 1,
            exc_amp: 0.0,
            exc_lp: 0.0,
            exc_a: 0.5,
            env: 0.0,
        }
    }

    fn reset(&mut self, period: f32) {
        self.buf.fill(0.0);
        self.delay = period;
        self.lp = 0.0;
        self.bridge = 0.0;
        self.contact_avg = 0.0;
        self.exc_left = 0;
        self.exc_lp = 0.0;
        self.env = 0.0;
    }

    /// A fingertip pluck: one period of soft noise, while the finger
    /// damps what was still ringing.
    fn pluck(&mut self, amp: f32, period: f32, bright_hz: f32, sr: f32) {
        self.exc_len = (period as usize).clamp(4, self.mask);
        self.exc_left = self.exc_len;
        self.exc_a = 1.0 - (-TAU * bright_hz.min(0.4 * sr) / sr).exp();
        // Unit-level noise after the low-pass.
        let var = self.exc_a / (2.0 - self.exc_a) / 3.0;
        self.exc_amp = amp / var.sqrt();
    }

    #[inline]
    fn read(&self, d: f32) -> f32 {
        let d = d.clamp(1.0, (self.mask - 3) as f32);
        let i = d as usize;
        let f = d - i as f32;
        let m = self.mask;
        let at = |k: usize| self.buf[(self.pos.wrapping_sub(k)) & m];
        let (xm1, x0, x1, x2) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        let c1 = 0.5 * (x1 - xm1);
        let c2 = xm1 - 2.5 * x0 + 2.0 * x1 - 0.5 * x2;
        let c3 = 0.5 * (x2 - xm1) + 1.5 * (x0 - x1);
        ((c3 * f + c2) * f + c1) * f + x0
    }

    #[inline]
    fn tick(&mut self, base: f32, jaw: f32, loss: f32, bridge_a: f32, rng: &mut Rng) -> f32 {
        // Jawari: while the string lies on the curved bridge it vibrates
        // over a shorter length (the mean is taken out to keep it in tune).
        let contact = self.bridge.max(0.0);
        self.contact_avg += 0.0005 * (contact - self.contact_avg);
        let target = base - jaw * (contact - 0.85 * self.contact_avg);
        self.delay += (target - self.delay).clamp(-0.3, 0.3);
        let y = self.read(self.delay);
        self.lp += LOSS_A * (y - self.lp);
        let mut v = self.lp * loss;
        if self.exc_left > 0 {
            let t = 1.0 - self.exc_left as f32 / self.exc_len as f32;
            let w = 4.0 * t * (1.0 - t);
            self.exc_lp += self.exc_a * (rng.bipolar() - self.exc_lp);
            v = v * (1.0 - 0.7 * w) + self.exc_amp * w * self.exc_lp;
            self.exc_left -= 1;
        }
        // Safety only: a soft ceiling far above normal levels.
        if v.abs() > 1.0 {
            v = (1.0 + 0.5 * fast_tanh(2.0 * (v.abs() - 1.0))).copysign(v);
        }
        if !v.is_finite() {
            v = 0.0;
        }
        self.pos = (self.pos + 1) & self.mask;
        self.buf[self.pos] = v;
        self.bridge += bridge_a * (v - self.bridge);
        self.env = (self.env * 0.9996).max(v.abs());
        v
    }
}

struct Tanpura {
    strings: [KsString; 4],
    /// Samples until the next pluck.
    timer: f32,
    next: usize,
    /// Small fixed mistuning per string, in cents.
    tune: [f32; 4],
    /// The string plucked last.
    last: usize,
}

impl Tanpura {
    fn new(max_period: usize) -> Self {
        Self {
            strings: std::array::from_fn(|_| KsString::new(max_period)),
            timer: 0.0,
            next: 0,
            tune: [0.0; 4],
            last: 0,
        }
    }

    fn reset(&mut self, rng: &mut Rng) {
        for s in self.strings.iter_mut() {
            s.reset(100.0);
        }
        self.timer = 0.0;
        self.next = 0;
        self.tune = std::array::from_fn(|_| rng.bipolar() * 1.5);
    }

    fn ratios(&self, g: &Glob) -> [f32; 4] {
        let d = g.detune * 0.25 + 0.5;
        [
            g.first * cents(self.tune[0]),
            cents(0.5 * d + self.tune[1]),
            cents(-0.5 * d + self.tune[2]),
            0.5 * cents(self.tune[3]),
        ]
    }

    #[allow(clippy::too_many_arguments)]
    fn block(&mut self, g: &Glob, f: f32, vel: f32, gate: bool, rng: &mut Rng, gains: (f32, f32), l: &mut [f32], r: &mut [f32]) {
        let n = l.len();
        let sr = g.sr;
        let ratios = self.ratios(g);
        if gate {
            self.timer -= n as f32;
            if self.timer <= 0.0 {
                let s = self.next;
                let period = sr / (f * ratios[s]).clamp(16.0, sr / 8.0);
                let amp = 0.3 * (0.6 + 0.4 * vel) * STRING_LEVELS[s] * (1.0 + 0.2 * g.evolve * rng.bipolar());
                self.strings[s].pluck(amp, period, 1500.0 + 2500.0 * vel, sr);
                self.last = s;
                let gap = PLUCK_GAPS[s] * (1.0 + 0.04 * g.evolve * rng.bipolar());
                self.timer = (self.timer + gap * sr / g.rate).max(0.0);
                self.next = (s + 1) % 4;
            }
        }
        let jaw = g.jawari * g.jawari.sqrt() * 0.05;
        let mut base = [0.0f32; 4];
        let mut jaws = [0.0f32; 4];
        let mut loss = [0.0f32; 4];
        let mut pans = [(0.0f32, 0.0f32); 4];
        for s in 0..4 {
            let period = sr / (f * ratios[s]).clamp(16.0, sr / 8.0);
            base[s] = period - LOSS_DELAY;
            jaws[s] = jaw * period;
            loss[s] = 10f32.powf(-3.0 * period / (g.decay * sr));
            pans[s] = pan_gains(STRING_PANS[s]);
        }
        let (g0, g1) = gains;
        let dg = (g1 - g0) / n as f32;
        let mut gn = g0;
        for i in 0..n {
            let (mut sl, mut sr) = (0.0, 0.0);
            for s in 0..4 {
                let y = self.strings[s].tick(base[s], jaws[s], loss[s], g.bridge_a, rng);
                sl += y * pans[s].0;
                sr += y * pans[s].1;
            }
            gn += dg;
            l[i] += sl * gn;
            r[i] += sr * gn;
        }
    }
}

/// PADsynth harmonics: a carrier per harmonic times its noise band (as I
/// and Q), a different stretch of the band for left and right.
struct Pad {
    count: usize,
    c: [f32; MAX_H],
    s: [f32; MAX_H],
    rc: [f32; MAX_H],
    rs: [f32; MAX_H],
    pos: [[u32; 2]; MAX_H],
    env: [[f32; 4]; MAX_H],
    step: [[f32; 4]; MAX_H],
}

impl Pad {
    fn new() -> Self {
        Self {
            count: 0,
            c: [1.0; MAX_H],
            s: [0.0; MAX_H],
            rc: [1.0; MAX_H],
            rs: [0.0; MAX_H],
            pos: [[0; 2]; MAX_H],
            env: [[0.0; 4]; MAX_H],
            step: [[0.0; 4]; MAX_H],
        }
    }

    fn reset(&mut self, rng: &mut Rng) {
        self.count = 0;
        for k in 0..MAX_H {
            let (s, c) = (rng.unipolar() * TAU).sin_cos();
            self.c[k] = c;
            self.s[k] = s;
            self.pos[k] = [rng.next_u32(), rng.next_u32()];
            self.env[k] = [0.0; 4];
            self.step[k] = [0.0; 4];
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn block(&mut self, g: &Glob, f: f32, bright: f32, formants: (f32, f32), gains: (f32, f32), l: &mut [f32], r: &mut [f32]) {
        let n = l.len();
        let sr = g.sr;
        let limit = (0.45 * sr).min(15000.0);
        let count = ((limit / f) as usize).min(MAX_H);
        // Harmonic levels: a slope set by Harmonics, odd/even balance and a
        // soft vowel-like formant; the fundamental stays the strongest.
        let slope = (2.4 - 1.9 * g.harm - 0.6 * bright).clamp(0.3, 3.2);
        let odd = (2.0 * (1.0 - g.odd_even)).min(1.0);
        let even = (2.0 * g.odd_even).min(1.0);
        let bump = |x: f32, c: f32| {
            let d = (x - c) / (0.4 * c);
            1.0 / (1.0 + d * d)
        };
        let mut amp = [0.0f32; MAX_H];
        let mut sum = 0.0;
        for (k, a) in amp.iter_mut().enumerate().take(count) {
            let h = k + 1;
            let fk = f * h as f32;
            let mut v = (-slope * g.ln[h]).exp();
            if h > 1 {
                v *= if h % 2 == 1 { odd } else { even };
                v *= 0.7 + 0.5 * bump(fk, formants.0) + 0.3 * bump(fk, formants.1);
                v = v.min(0.8);
            }
            v *= ((limit - fk) / (0.2 * limit)).clamp(0.0, 1.0);
            *a = v;
            sum += v * v;
        }
        let norm = if sum > 0.0 { 1.0 / sum.sqrt() } else { 0.0 };
        // Band width per harmonic (PADsynth: grows with the harmonic number),
        // turned into a read speed through the band table.
        let speed = PAD_N as f32 / (PAD_SIGMA * sr) * 0.3536 * 65536.0 * n as f32;
        let w = TAU * f / sr;
        let (sw1, cw1) = w.sin_cos();
        let (mut cw, mut sw) = (cw1, sw1);
        let inv_n = 1.0 / n as f32;
        for k in 0..count {
            let h = k + 1;
            let bw_hz = g.bw_rel * f * h as f32 * g.widen[h];
            let inc = (bw_hz * speed).min(1.0e9) as u32;
            self.pos[k][0] = self.pos[k][0].wrapping_add(inc);
            self.pos[k][1] = self.pos[k][1].wrapping_add(inc);
            let a = amp[k] * norm;
            let (il, ql) = pad_read(g.pad_tab, self.pos[k][0]);
            let (ir, qr) = pad_read(g.pad_tab, self.pos[k][1]);
            let target = [il * a, ql * a, ir * a, qr * a];
            for j in 0..4 {
                self.step[k][j] = (target[j] - self.env[k][j]) * inv_n;
            }
            self.rc[k] = cw;
            self.rs[k] = sw;
            // Keep the carrier on the unit circle.
            let m = 1.5 - 0.5 * (self.c[k] * self.c[k] + self.s[k] * self.s[k]);
            self.c[k] *= m;
            self.s[k] *= m;
            (cw, sw) = (cw * cw1 - sw * sw1, cw * sw1 + sw * cw1);
        }
        for k in count..self.count {
            self.env[k] = [0.0; 4];
        }
        self.count = count;
        let (g0, g1) = gains;
        let dg = (g1 - g0) * inv_n;
        let mut gn = g0;
        for i in 0..n {
            let (mut sl, mut sr) = (0.0, 0.0);
            for k in 0..count {
                let (c, s) = (self.c[k], self.s[k]);
                let nc = c * self.rc[k] - s * self.rs[k];
                let ns = c * self.rs[k] + s * self.rc[k];
                self.c[k] = nc;
                self.s[k] = ns;
                let e = &mut self.env[k];
                let st = &self.step[k];
                e[0] += st[0];
                e[1] += st[1];
                e[2] += st[2];
                e[3] += st[3];
                sl += e[0] * nc - e[1] * ns;
                sr += e[2] * nc - e[3] * ns;
            }
            gn += dg;
            l[i] += sl * gn;
            r[i] += sr * gn;
        }
    }
}

/// Two pairs of cross-modulating sines: one at the note, one an octave up.
struct Swell {
    phase: [f32; 4],
    /// The last two outputs of each oscillator (averaged for modulation).
    y: [[f32; 2]; 4],
    env: [SwellEnv; 4],
    /// Each oscillator's own small, fixed detune (cents).
    tune: [f32; 4],
    /// The swell of each oscillator now, for the display.
    now: [f32; 4],
}

impl Swell {
    fn new() -> Self {
        Self { phase: [0.0; 4], y: [[0.0; 2]; 4], env: [SwellEnv { from: 0.0, to: 0.0, t: 0.0, dur: 1.0, hold: false }; 4], tune: [0.0; 4], now: [0.0; 4] }
    }

    fn reset(&mut self, rng: &mut Rng) {
        for k in 0..4 {
            self.phase[k] = rng.unipolar();
            self.y[k] = [0.0; 2];
            self.env[k] = SwellEnv::new(rng);
            self.tune[k] = rng.bipolar();
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn block(&mut self, g: &Glob, f: f32, rng: &mut Rng, gains: (f32, f32), l: &mut [f32], r: &mut [f32]) {
        let n = l.len();
        let secs = n as f32 / g.sr;
        let mut e = [0.0f32; 4];
        for (k, v) in e.iter_mut().enumerate() {
            *v = self.env[k].step(rng, secs, g.swell_time);
        }
        self.now = e;
        let d = 0.5 * g.detune;
        let spread = 1.0 + 2.0 * g.evolve;
        let inc = [
            f * cents(d + self.tune[0] * spread) / g.sr,
            f * cents(-d + self.tune[1] * spread) / g.sr,
            2.0 * f * cents(0.8 * d + self.tune[2] * spread) / g.sr,
            2.0 * f * cents(-0.8 * d + self.tune[3] * spread) / g.sr,
        ];
        // Modulation index in turns; a little self-feedback for warmth.
        let index = g.fm * g.fm.sqrt() * 0.45;
        let fb = g.fm * 0.06;
        let level = [e[0], e[1], 0.5 * e[2], 0.5 * e[3]];
        let pans = [pan_gains(-0.6), pan_gains(0.6), pan_gains(-0.3), pan_gains(0.3)];
        let (g0, g1) = gains;
        let dg = (g1 - g0) / n as f32;
        let mut gn = g0;
        let t = g.sine;
        for i in 0..n {
            let (mut sl, mut sr) = (0.0, 0.0);
            for pair in 0..2 {
                let (a, b) = (2 * pair, 2 * pair + 1);
                let ya_prev = 0.5 * (self.y[a][0] + self.y[a][1]);
                let yb_prev = 0.5 * (self.y[b][0] + self.y[b][1]);
                let ma = index * (0.3 + 0.7 * e[b]) * yb_prev + fb * ya_prev;
                let mb = index * (0.3 + 0.7 * e[a]) * ya_prev + fb * yb_prev;
                self.phase[a] += inc[a];
                if self.phase[a] >= 1.0 {
                    self.phase[a] -= 1.0;
                }
                self.phase[b] += inc[b];
                if self.phase[b] >= 1.0 {
                    self.phase[b] -= 1.0;
                }
                let ya = sin_turns(t, self.phase[a] + ma);
                let yb = sin_turns(t, self.phase[b] + mb);
                self.y[a] = [ya, self.y[a][0]];
                self.y[b] = [yb, self.y[b][0]];
                let (va, vb) = (ya * level[a], yb * level[b]);
                sl += va * pans[a].0 + vb * pans[b].0;
                sr += va * pans[a].1 + vb * pans[b].1;
            }
            gn += dg;
            l[i] += sl * gn;
            r[i] += sr * gn;
        }
    }
}

/// Wind: noise through resonant band-passes on the harmonics, left and
/// right with their own noise.
struct Air {
    f: [[Svf; 2]; AIR_BANDS],
    pitch: [Wander; AIR_BANDS],
    level: [Wander; AIR_BANDS],
    gust: Wander,
    rng: Rng,
}

impl Air {
    fn new(seed: u32) -> Self {
        let mut rng = Rng::new(seed);
        Self {
            f: [[Svf::new(); 2]; AIR_BANDS],
            pitch: std::array::from_fn(|_| Wander::new(&mut rng)),
            level: std::array::from_fn(|_| Wander::new(&mut rng)),
            gust: Wander::new(&mut rng),
            rng,
        }
    }

    fn reset(&mut self) {
        for b in 0..AIR_BANDS {
            self.f[b][0].reset();
            self.f[b][1].reset();
            self.pitch[b] = Wander::new(&mut self.rng);
            self.level[b] = Wander::new(&mut self.rng);
        }
        self.gust = Wander::new(&mut self.rng);
    }

    fn block(&mut self, g: &Glob, f: f32, gains: (f32, f32), l: &mut [f32], r: &mut [f32]) {
        let n = l.len();
        let sr = g.sr;
        let secs = n as f32 / sr;
        let wobble = 6.0 + 30.0 * g.evolve;
        let q = g.air_q;
        let mut lv = [0.0f32; AIR_BANDS];
        let mut bands = 0;
        for b in 0..AIR_BANDS {
            let w = self.pitch[b].step(&mut self.rng, secs, 2.0, 7.0);
            let fc = f * AIR_HARMONICS[b] * cents(w * wobble);
            if fc > 0.42 * sr {
                break;
            }
            let x = 0.5 + 0.5 * self.level[b].step(&mut self.rng, secs, 1.5, 5.0);
            self.f[b][0].set_q(fc, q, sr);
            self.f[b][1].set_q(fc, q, sr);
            // Same loudness whatever the resonance; higher bands softer.
            let norm = (3.0 * sr / (q * PI * fc)).sqrt();
            lv[b] = norm * x * x / AIR_HARMONICS[b].sqrt();
            bands = b + 1;
        }
        let gust = 0.6 + 0.4 * self.gust.step(&mut self.rng, secs, 1.0, 4.0);
        let (g0, g1) = (gains.0 * gust, gains.1 * gust);
        let dg = (g1 - g0) / n as f32;
        let mut gn = g0;
        for i in 0..n {
            let (nl, nr) = (self.rng.bipolar(), self.rng.bipolar());
            let (mut sl, mut sr) = (0.0, 0.0);
            for b in 0..bands {
                sl += self.f[b][0].tick(nl).band * lv[b];
                sr += self.f[b][1].tick(nr).band * lv[b];
            }
            gn += dg;
            l[i] += sl * gn;
            r[i] += sr * gn;
        }
    }
}

/// What drifts by itself in each voice (scaled by Evolve).
const D_PITCH: usize = 0;
const D_BRIGHT: usize = 1;
const D_LEVEL: usize = 2; // four of them, one per layer
const D_F1: usize = 6;
const D_F2: usize = 7;
const DRIFTS: usize = 8;

struct Voice {
    note: u8,
    vel: f32,
    gate: bool,
    active: bool,
    age: u64,
    /// Envelope: attack progress (0..1) and the current level.
    att: f32,
    amp: f32,
    /// Stolen: fading out fast, then starting this note.
    stealing: bool,
    pending: Option<(u8, f32)>,
    freq: f32,
    /// Layer gains at the end of the last block (ramped from there).
    gains: [f32; 4],
    drift: [Wander; DRIFTS],
    rng: Rng,
    tanpura: Tanpura,
    pad: Pad,
    swell: Swell,
    air: Air,
}

impl Voice {
    fn new(sr: f32, seed: u32) -> Self {
        let mut rng = Rng::new(seed);
        Self {
            note: 0,
            vel: 0.0,
            gate: false,
            active: false,
            age: 0,
            att: 0.0,
            amp: 0.0,
            stealing: false,
            pending: None,
            freq: 440.0,
            gains: [0.0; 4],
            drift: std::array::from_fn(|_| Wander::new(&mut rng)),
            tanpura: Tanpura::new((sr / 16.0) as usize + 4),
            pad: Pad::new(),
            swell: Swell::new(),
            air: Air::new(seed ^ 0xA1A1_5EED),
            rng,
        }
    }

    fn start(&mut self, note: u8, vel: f32, age: u64) {
        self.note = note;
        self.vel = vel;
        self.gate = true;
        self.active = true;
        self.age = age;
        self.att = 0.0;
        self.amp = 0.0;
        self.stealing = false;
        self.pending = None;
        self.freq = midi_to_freq(note as f32);
        self.gains = [0.0; 4];
        for d in self.drift.iter_mut() {
            *d = Wander::new(&mut self.rng);
        }
        self.tanpura.reset(&mut self.rng);
        self.pad.reset(&mut self.rng);
        self.swell.reset(&mut self.rng);
        self.air.reset();
    }

    /// The same note again while it still sounds: attack from where it is.
    fn retrigger(&mut self, vel: f32, age: u64) {
        self.vel = vel;
        self.gate = true;
        self.age = age;
        self.att = (1.0 - 2.0 * self.amp.clamp(0.0, 1.0)).acos() / PI;
        self.tanpura.timer = 0.0;
        self.tanpura.next = 0;
    }

    /// Fades out quickly, then plays `note`.
    fn steal(&mut self, note: u8, vel: f32, age: u64) {
        self.stealing = true;
        self.gate = true;
        self.age = age;
        self.pending = Some((note, vel));
    }

    fn render(&mut self, g: &Glob, l: &mut [f32], r: &mut [f32]) {
        if !self.active {
            return;
        }
        let n = l.len();
        let secs = n as f32 / g.sr;
        // Envelope: an S-shaped rise, then an exponential fall that
        // reaches -60 dB at the release time.
        if self.stealing {
            self.amp *= g.steal_k.powi(n as i32);
        } else if self.gate {
            self.att = (self.att + g.att_inc * n as f32).min(1.0);
            self.amp = 0.5 - 0.5 * (PI * self.att).cos();
        } else {
            self.amp *= g.rel_k.powi(n as i32);
        }
        let ending = !self.gate || self.stealing;
        if ending && self.amp < 1e-3 {
            self.amp = 0.0;
        }
        // Drift.
        let ev = g.evolve;
        let rng = &mut self.rng;
        let pitch = self.drift[D_PITCH].step(rng, secs, 6.0, 16.0) * ev * 7.0;
        let bright = self.drift[D_BRIGHT].step(rng, secs, 5.0, 14.0) * ev;
        let f1 = 550.0 * cents(600.0 * self.drift[D_F1].step(rng, secs, 4.0, 12.0) * (0.3 + 0.7 * ev));
        let f2 = 1600.0 * cents(500.0 * self.drift[D_F2].step(rng, secs, 4.0, 12.0) * (0.3 + 0.7 * ev));
        let mut targets = [0.0f32; 4];
        for (k, t) in targets.iter_mut().enumerate() {
            let d = self.drift[D_LEVEL + k].step(rng, secs, 8.0, 25.0);
            *t = g.level[k] * (1.0 + 0.6 * ev * d).max(0.0) * self.amp * (0.5 + 0.5 * self.vel);
        }
        let f = self.freq * cents(pitch);
        let last = self.gains;
        self.gains = targets;
        let gains = |k: usize| (last[k], targets[k]);
        if last[0] > 0.0 || targets[0] > 0.0 {
            let (a, b) = gains(0);
            self.tanpura.block(g, f, self.vel, self.gate && !self.stealing, &mut self.rng, (a * TANPURA_GAIN, b * TANPURA_GAIN), l, r);
        }
        if last[1] > 0.0 || targets[1] > 0.0 {
            let (a, b) = gains(1);
            self.pad.block(g, f, bright, (f1, f2), (a * PAD_GAIN, b * PAD_GAIN), l, r);
        }
        if last[2] > 0.0 || targets[2] > 0.0 {
            let (a, b) = gains(2);
            self.swell.block(g, f, &mut self.rng, (a * SWELL_GAIN, b * SWELL_GAIN), l, r);
        }
        if last[3] > 0.0 || targets[3] > 0.0 {
            let (a, b) = gains(3);
            self.air.block(g, f, (a * AIR_GAIN, b * AIR_GAIN), l, r);
        }
        if ending && self.amp <= 0.0 {
            match self.pending.take() {
                Some((note, vel)) if self.stealing => self.start(note, vel, self.age),
                _ => {
                    self.active = false;
                    self.gate = false;
                    self.stealing = false;
                }
            }
        }
    }
}

pub struct DroneState {
    sr: f32,
    voices: Vec<Voice>,
    counter: u64,
    sine: &'static [f32],
    pad_tab: &'static [f32],
    ln: [f32; MAX_H + 1],
    widen: [f32; MAX_H + 1],
    hall: Box<Hall>,
    hall_p: Vec<f32>,
    wl: Vec<f32>,
    wr: Vec<f32>,
    tone_lp: [Svf; 2],
    tilt: [OnePole; 2],
    dc: [Dc; 2],
    /// Samples since the last voice stopped.
    idle: usize,
}

impl DroneState {
    pub fn new(sr: f32) -> Self {
        let mut hall_p = crate::params::defaults(hall::SPECS);
        hall_p[hall::MODULATION] = 0.12;
        hall_p[hall::WIDTH] = 1.0;
        Self {
            sr,
            voices: (0..MAX_VOICES).map(|i| Voice::new(sr, 0x0D50_0E17 ^ (i as u32 + 1).wrapping_mul(0x9E37_79B9))).collect(),
            counter: 0,
            sine: sine_table(),
            pad_tab: pad_table(),
            ln: std::array::from_fn(|k| (k.max(1) as f32).ln()),
            // PADsynth's bandwidth scale: relative width grows as h^0.3.
            widen: std::array::from_fn(|k| (k.max(1) as f32).powf(0.3)),
            hall: Box::new(Hall::new(sr)),
            hall_p,
            wl: vec![0.0; CHUNK],
            wr: vec![0.0; CHUNK],
            tone_lp: [Svf::new(); 2],
            tilt: [OnePole::default(); 2],
            dc: [Dc::default(); 2],
            idle: usize::MAX / 2,
        }
    }

    /// What the display shows, for the newest voice: [0] voices sounding,
    /// [1..5] the level of each layer (tanpura, pad, swell, air), [5..9] how
    /// strongly each tanpura string rings, [9] the string plucked last,
    /// [10..14] the swells, [14] the note, [15] its loudness.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.voices.iter().filter(|v| v.active).count() as f32;
        if let Some(v) = self.voices.iter().filter(|v| v.active).max_by_key(|v| v.age) {
            out[1..5].copy_from_slice(&v.gains);
            for s in 0..4 {
                out[5 + s] = v.tanpura.strings[s].env;
            }
            out[9] = v.tanpura.last as f32;
            out[10..14].copy_from_slice(&v.swell.now);
            out[14] = v.note as f32;
            out[15] = v.amp;
        }
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        self.counter += 1;
        let age = self.counter;
        let vel = vel.clamp(0.0, 1.0);
        let poly = (p[VOICES].round() as usize).clamp(1, 6);
        let same = self.voices.iter().position(|v| v.active && !v.stealing && v.note == note);
        // At most `poly` held notes: the oldest other one lets go.
        let held = |vs: &[Voice]| vs.iter().enumerate().filter(|(i, v)| v.active && v.gate && Some(*i) != same).count();
        while held(&self.voices) >= poly {
            let oldest = self.voices.iter_mut().enumerate().filter(|(i, v)| v.active && v.gate && Some(*i) != same).min_by_key(|(_, v)| v.age);
            match oldest {
                Some((_, v)) => {
                    v.gate = false;
                    v.pending = None;
                }
                None => break,
            }
        }
        if let Some(i) = same {
            self.voices[i].retrigger(vel, age);
            return;
        }
        if let Some(v) = self.voices.iter_mut().find(|v| !v.active) {
            v.start(note, vel, age);
            return;
        }
        // All busy: the quietest tail (or the oldest note) makes room.
        let pick = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, v)| !v.gate)
            .min_by(|a, b| a.1.amp.total_cmp(&b.1.amp))
            .or_else(|| self.voices.iter().enumerate().min_by_key(|(_, v)| v.age))
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.voices[pick].steal(note, vel, age);
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        if p.len() < SPECS.len() {
            return;
        }
        for v in self.voices.iter_mut().filter(|v| v.active && v.gate) {
            if v.stealing {
                if v.pending.map(|(n, _)| n) == Some(note) {
                    v.pending = None;
                    v.gate = false;
                }
            } else if v.note == note {
                v.gate = false;
            }
        }
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.gate = false;
            v.pending = None;
        }
    }

    /// Voices still sounding.
    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let total = l.len().min(r.len());
        let mut done = 0;
        while done < total {
            let n = CHUNK.min(total - done);
            self.render_chunk(p, &mut l[done..done + n], &mut r[done..done + n]);
            done += n;
        }
    }

    fn render_chunk(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len()).min(CHUNK);
        let sr = self.sr;
        let space = p[SPACE].clamp(0.0, 1.0);
        let any = self.voices.iter().any(|v| v.active);
        if any {
            self.idle = 0;
        } else {
            self.idle = self.idle.saturating_add(n);
            let tail = if space > 0.001 { hall_decay(space) * 1.3 + 1.0 } else { 0.1 };
            if self.idle as f32 > tail * sr {
                return;
            }
        }
        let g = Glob {
            sr,
            sine: self.sine,
            pad_tab: self.pad_tab,
            ln: &self.ln,
            widen: &self.widen,
            level: [p[TANPURA], p[PAD], p[SWELL], p[AIR]].map(|v| v.clamp(0.0, 1.0)),
            first: match p[FIRST_STRING].round() as i32 {
                1 => 2.0 / 3.0,
                2 => 15.0 / 16.0,
                _ => 0.75,
            },
            rate: p[PLUCK_RATE].clamp(0.05, 10.0),
            jawari: p[JAWARI].clamp(0.0, 1.0),
            decay: p[STRING_DECAY].clamp(0.2, 60.0),
            bw_rel: cents(p[BANDWIDTH].clamp(0.1, 1200.0)) - 1.0,
            harm: p[HARMONICS].clamp(0.0, 1.0),
            odd_even: p[ODD_EVEN].clamp(0.0, 1.0),
            fm: p[FM].clamp(0.0, 1.0),
            swell_time: p[SWELL_TIME].clamp(0.1, 60.0),
            air_q: 0.8 * 150f32.powf(p[AIR_RESONANCE].clamp(0.0, 1.0)),
            evolve: p[EVOLVE].clamp(0.0, 1.0),
            detune: p[DETUNE].clamp(0.0, 100.0),
            att_inc: 1.0 / (p[ATTACK].max(0.001) * sr),
            rel_k: 10f32.powf(-3.0 / (p[RELEASE].max(0.01) * sr)),
            steal_k: 10f32.powf(-3.0 / (0.03 * sr)),
            bridge_a: 1.0 - (-TAU * 2500.0 / sr).exp(),
        };
        let (wl, wr) = (&mut self.wl[..n], &mut self.wr[..n]);
        wl.fill(0.0);
        wr.fill(0.0);
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let mut off = 0;
            while off < n {
                let m = CTRL.min(n - off);
                v.render(&g, &mut wl[off..off + m], &mut wr[off..off + m]);
                off += m;
            }
        }
        // Tone: a low-pass when turned down, a gentle lift of the highs up.
        let tone = p[TONE].clamp(-1.0, 1.0);
        let dc_r = 1.0 - TAU * 5.0 / sr;
        let lift = tone.max(0.0) * 0.9;
        if tone < -0.01 {
            let fc = 18000.0 * (800.0f32 / 18000.0).powf(-tone);
            for f in self.tone_lp.iter_mut() {
                f.set_q(fc, 0.6, sr);
            }
        }
        for t in self.tilt.iter_mut() {
            t.set(2500.0, sr);
        }
        let width = p[WIDTH].clamp(0.0, 2.0);
        for i in 0..n {
            let mut x = [wl[i], wr[i]];
            for c in 0..2 {
                let mut y = self.dc[c].tick(x[c], dc_r);
                if tone < -0.01 {
                    y = self.tone_lp[c].tick(y).low;
                }
                if lift > 0.0 {
                    y += lift * self.tilt[c].hp(y);
                }
                x[c] = y;
            }
            let mid = 0.5 * (x[0] + x[1]);
            let side = 0.5 * (x[0] - x[1]) * width;
            wl[i] = mid + side;
            wr[i] = mid - side;
        }
        if space > 0.001 {
            let hp = &mut self.hall_p;
            hp[hall::PREDELAY] = 25.0;
            hp[hall::SIZE] = 0.5 + 0.45 * space;
            hp[hall::DECAY] = hall_decay(space);
            hp[hall::LOW_DECAY] = 1.1;
            hp[hall::DAMPING] = 5500.0;
            hp[hall::DIFFUSION] = 0.8;
            hp[hall::MODULATION] = 0.12;
            hp[hall::FREEZE] = 0.0;
            hp[hall::WIDTH] = 1.0;
            hp[hall::MIX] = 0.6 * space;
            let ctx = FxCtx { sr, bpm: 120.0, beat: 0.0, playing: false };
            self.hall.process(&self.hall_p, wl, wr, &ctx, None);
        }
        let gain = db_to_gain(p[VOLUME]);
        for i in 0..n {
            l[i] += soft_clip(wl[i] * gain);
            r[i] += soft_clip(wr[i] * gain);
        }
    }
}

/// Reverb decay (T60, seconds) for a Space setting.
fn hall_decay(space: f32) -> f32 {
    2.0 + 8.0 * space
}

/// Clean below 0.9, then a soft knee up to 1.4 at most.
#[inline]
fn soft_clip(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.9 {
        x
    } else {
        (0.9 + 0.5 * fast_tanh(2.0 * (a - 0.9))).copysign(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn run(st: &mut DroneState, p: &[f32], secs: f32) -> (Vec<f32>, Vec<f32>) {
        let n = (secs * SR) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (cl, cr) in l.chunks_mut(480).zip(r.chunks_mut(480)) {
            st.render(p, cl, cr);
        }
        (l, r)
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0f32, |m, v| m.max(v.abs()))
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// Only one layer, dry, no drift.
    fn solo(layer: usize) -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        for k in [TANPURA, PAD, SWELL, AIR] {
            p[k] = if k == layer { 1.0 } else { 0.0 };
        }
        p[EVOLVE] = 0.0;
        p[SPACE] = 0.0;
        p[ATTACK] = 0.01;
        p
    }

    /// Strength of frequency `f` in `x` (Hann-windowed correlation).
    fn level_at(x: &[f32], f: f32) -> f32 {
        let n = x.len();
        let w = TAU * f / SR;
        let (mut s, mut c) = (0.0f64, 0.0f64);
        for (i, v) in x.iter().enumerate() {
            let win = 0.5 - 0.5 * (TAU * i as f32 / n as f32).cos();
            let a = (w * i as f32) as f64;
            s += (v * win) as f64 * a.sin();
            c += (v * win) as f64 * a.cos();
        }
        ((s * s + c * c).sqrt() / n as f64) as f32
    }

    #[test]
    fn every_preset_sounds_and_fades_out() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = DroneState::new(SR);
            for n in [48, 55] {
                st.note_on(&p, n, 0.8);
            }
            let (l, r) = run(&mut st, &p, p[ATTACK] + 2.5);
            let held = rms(&l[l.len() - 24000..]).max(rms(&r[r.len() - 24000..]));
            let pk = peak(&l).max(peak(&r));
            eprintln!("{name:<16} held rms {held:.3} peak {pk:.3}");
            assert!(l.iter().chain(&r).all(|x| x.is_finite()), "{name}: finite");
            assert!(held > 0.01 && pk < 1.5, "{name}: rms {held} peak {pk}");
            for n in [48, 55] {
                st.note_off(&p, n);
            }
            let tail = hall_decay(p[SPACE]) + 1.5;
            let (l, r) = run(&mut st, &p, p[RELEASE] + 0.2);
            assert!(l.iter().chain(&r).all(|x| x.is_finite()));
            assert_eq!(st.active_voices(), 0, "{name}: voices stop at the release time");
            let (l, r) = run(&mut st, &p, tail);
            let end = rms(&l[l.len() - 12000..]).max(rms(&r[r.len() - 12000..]));
            assert!(end < held * 1e-3, "{name}: tail {end} vs {held}");
        }
    }

    #[test]
    fn tanpura_plucks_repeat_at_the_pluck_rate() {
        let mut p = solo(TANPURA);
        p[PLUCK_RATE] = 2.0;
        p[STRING_DECAY] = 3.0;
        let mut st = DroneState::new(SR);
        st.note_on(&p, 57, 0.8);
        let (l, r) = run(&mut st, &p, 6.0);
        // Energy in 10 ms frames (longer than a period of the low Sa, so the
        // wave itself does not look like a pluck); a pluck is a sudden jump.
        let frame = 480;
        let e: Vec<f32> = l.chunks(frame).zip(r.chunks(frame)).map(|(a, b)| rms(a) + rms(b)).collect();
        let mut onsets = Vec::new();
        for i in 0..e.len() {
            let before = e[i.saturating_sub(4)..i].iter().copied().reduce(f32::min).unwrap_or(0.0);
            let rise = e[i] - before;
            let last = onsets.last().copied().unwrap_or(-1000i64);
            if rise > 0.25 * before.max(0.01) && rise > 0.02 && (i as i64 - last) * frame as i64 > (0.2 * SR) as i64 {
                onsets.push(i as i64);
            }
        }
        let times: Vec<f32> = onsets.iter().map(|&i| (i * frame as i64) as f32 / SR).collect();
        eprintln!("onsets {times:?}");
        // 2 Hz: Pa, Sa, Sa half a second apart, the low Sa breathes 0.75 s.
        let expect = [0.0, 0.5, 1.0, 1.5, 2.25, 2.75, 3.25, 3.75, 4.5, 5.0, 5.5];
        assert_eq!(times.len(), expect.len(), "{times:?}");
        for (t, e) in times.iter().zip(expect) {
            assert!((t - e).abs() < 0.04, "{t} vs {e}");
        }
    }

    #[test]
    fn pad_is_centred_on_the_note() {
        let mut p = solo(PAD);
        p[WIDTH] = 1.0;
        for (bw, harm) in [(40.0, 0.5), (5.0, 0.9), (150.0, 0.2)] {
            p[BANDWIDTH] = bw;
            p[HARMONICS] = harm;
            let mut st = DroneState::new(SR);
            st.note_on(&p, 57, 0.8);
            let (l, r) = run(&mut st, &p, 3.0);
            let x: Vec<f32> = l[48000..].iter().zip(&r[48000..]).map(|(a, b)| a + b).collect();
            let at = |f: f32| level_at(&x, f);
            let fund = at(220.0);
            let others = [110.0, 165.0, 330.0, 440.0, 660.0].map(at);
            eprintln!("bw {bw} fund {fund:.4} others {others:?}");
            assert!(others.iter().all(|&o| o < fund), "bw {bw}: {fund} {others:?}");
            assert!(fund > 3.0 * others[0] && fund > 3.0 * others[1], "no sub: {fund} {others:?}");
        }
    }

    #[test]
    fn six_notes_stay_below_the_limit() {
        let mut all = crate::params::defaults(SPECS);
        for k in [TANPURA, PAD, SWELL, AIR, JAWARI, FM, EVOLVE] {
            all[k] = 1.0;
        }
        all[VOICES] = 6.0;
        let mut sets: Vec<(String, Vec<f32>)> = vec![("all layers".into(), all)];
        for name in preset_names() {
            let mut p = preset(name);
            p[VOICES] = 6.0;
            p[ATTACK] = p[ATTACK].min(1.0);
            sets.push((name.to_string(), p));
        }
        for (name, p) in sets {
            let mut st = DroneState::new(SR);
            for n in [36, 43, 48, 55, 60, 64] {
                st.note_on(&p, n, 1.0);
            }
            let (l, r) = run(&mut st, &p, 5.0);
            let pk = peak(&l).max(peak(&r));
            eprintln!("{name:<16} six notes peak {pk:.3}");
            assert!(l.iter().chain(&r).all(|x| x.is_finite()) && pk < 1.5 && pk > 0.02, "{name}: {pk}");
        }
    }

    #[test]
    fn evolve_at_max_stays_stable() {
        let mut p = crate::params::defaults(SPECS);
        for k in [TANPURA, PAD, SWELL, AIR, EVOLVE, JAWARI, FM, AIR_RESONANCE] {
            p[k] = 1.0;
        }
        p[ATTACK] = 0.5;
        p[DETUNE] = 30.0;
        p[STRING_DECAY] = 30.0;
        p[SWELL_TIME] = 0.5;
        let mut st = DroneState::new(SR);
        for n in [40, 52, 59] {
            st.note_on(&p, n, 1.0);
        }
        let (l, r) = run(&mut st, &p, 20.0);
        assert!(l.iter().chain(&r).all(|x| x.is_finite()));
        let pk = peak(&l).max(peak(&r));
        let early = rms(&l[48000 * 3..48000 * 5]);
        let late = rms(&l[48000 * 15..]);
        eprintln!("evolve max: peak {pk:.3} early {early:.3} late {late:.3}");
        assert!(pk < 1.5, "{pk}");
        assert!(late < early * 3.0 && late > early * 0.2, "no runaway or collapse: {early} {late}");
    }

    #[test]
    fn voices_param_limits_held_notes() {
        let mut p = preset("Organ Drone");
        p[VOICES] = 1.0;
        let mut st = DroneState::new(SR);
        st.note_on(&p, 48, 0.8);
        st.note_on(&p, 55, 0.8);
        let held = st.voices.iter().filter(|v| v.active && v.gate).count();
        assert_eq!(held, 1);
        // Many notes, all slots busy: still finite and bounded.
        p[VOICES] = 6.0;
        for n in 40..60 {
            st.note_on(&p, n, 0.8);
            let (l, _) = run(&mut st, &p, 0.05);
            assert!(l.iter().all(|x| x.is_finite()));
        }
        assert!(st.voices.iter().filter(|v| v.gate).count() <= 6);
    }

    #[test]
    fn cpu_six_notes_all_layers() {
        let mut p = crate::params::defaults(SPECS);
        for k in [TANPURA, PAD, SWELL, AIR] {
            p[k] = 0.8;
        }
        p[VOICES] = 6.0;
        p[ATTACK] = 0.1;
        let mut st = DroneState::new(SR);
        for n in [36, 43, 48, 55, 60, 64] {
            st.note_on(&p, n, 0.8);
        }
        let secs = 4.0;
        let n = (secs * SR) as usize;
        let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
        let t0 = std::time::Instant::now();
        for _ in 0..n / 256 {
            l.fill(0.0);
            r.fill(0.0);
            st.render(&p, &mut l, &mut r);
        }
        let used = t0.elapsed().as_secs_f32();
        eprintln!("Drone: 6 notes, all layers, {secs} s of audio in {used:.3} s = {:.1} % of one core", 100.0 * used / secs);
    }
}

