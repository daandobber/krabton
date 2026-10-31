//! Stretch: a sample stretched 2 to 1000 times its length, after Paul
//! Nasca's Paulstretch. The sample is read a window at a time (0.04 to 0.7
//! s), each window's spectrum kept and its phases thrown to chance, and the
//! windows laid over each other a quarter apart while the place read moves
//! on slowly: a smeared, singing, still version of the sound, every
//! frequency in it held. Onset keeps the attacks (a window where something
//! starts moves at the sample's own speed, its phases kept), Freeze stops
//! the place (the sustain pedal too), Shimmer adds the spectrum an octave
//! up, and the keys move the whole spectrum to their pitch.
//!
//! As an instrument every key held is a voice (up to four) reading from
//! Position on; Position can be moved while they play, a scrub. The big
//! transforms are done a step at a time (the bit reversal, a pass, the
//! spectrum, a pass...) spread over the blocks before their window is due,
//! so no block does more than a few steps; every buffer is made when the
//! instrument is. The same windows render a clip offline ([`render`]): a
//! stretch to a given length, or a drone frozen at one moment.

use super::fft::Plan;
use super::reverb::Reverb;
use super::{db_to_gain, OnePole, Rng};
use crate::params::{ParamSpec, Unit};
use crate::sample::{SampleData, SampleRef};
use std::sync::OnceLock;

pub const STRETCH: usize = 0;
pub const WINDOW: usize = 1;
pub const POSITION: usize = 2;
pub const FREEZE: usize = 3;
pub const ONSET: usize = 4;
pub const SHIMMER: usize = 5;
pub const ROOT: usize = 6;
pub const PITCH: usize = 7;
pub const KEYS: usize = 8;
pub const ATTACK: usize = 9;
pub const RELEASE: usize = 10;
pub const TONE: usize = 11;
pub const SPACE: usize = 12;
pub const VOICES: usize = 13;
pub const VOLUME: usize = 14;
pub const VELOCITY: usize = 15;

/// The window sizes, in samples, and what they are called (their length at 48 kHz).
pub const SIZES: [usize; 5] = [2048, 4096, 8192, 16384, 32768];
pub const WINDOWS: &[&str] = &["0.04 s", "0.09 s", "0.17 s", "0.34 s", "0.68 s"];
pub const MAX_WINDOW: usize = 32768;
pub const MAX_VOICES: usize = 4;
/// The level of windows laid a quarter apart with their phases at random
/// (Hann in and out: the sum of their powers is 3/8 · 1.5).
const NORM: f32 = 4.0 / 3.0;
/// Bands of the spectrum the screen is shown.
pub const BANDS: usize = 48;
/// Where they start in the screen's values.
pub const BANDS_AT: usize = 12;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::log("Stretch", "Stretch", 2.0, 1000.0, 8.0, Unit::Ratio),
    ParamSpec::choice("Window", "Stretch", WINDOWS, 3),
    ParamSpec::lin("Position", "Stretch", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Freeze", "Stretch", false),
    ParamSpec::lin("Onset", "Stretch", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Shimmer", "Color", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Root", "Pitch", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::int("Pitch", "Pitch", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::toggle("Keys", "Pitch", true),
    ParamSpec::log("Attack", "Amp", 0.01, 10.0, 1.0, Unit::Sec),
    ParamSpec::log("Release", "Amp", 0.05, 20.0, 3.0, Unit::Sec),
    ParamSpec::log("Tone", "Color", 200.0, 20000.0, 20000.0, Unit::Hz),
    ParamSpec::lin("Space", "Color", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::int("Voices", "Output", 1.0, 4.0, 2.0, Unit::None),
    ParamSpec::lin("Volume", "Output", -30.0, 6.0, -3.0, Unit::Db),
    ParamSpec::lin("Velocity", "Amp", 0.0, 1.0, 0.5, Unit::Percent),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Endless", "Glacier", "Frozen Moment", "Slow Bloom", "Shimmer Drone", "Keep the Hits", "Dark Hall", "Wide Pad"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Glacier" => &[(STRETCH, 200.0), (WINDOW, 4.0), (ATTACK, 3.0), (RELEASE, 8.0), (SPACE, 0.5)],
        "Frozen Moment" => &[(FREEZE, 1.0), (POSITION, 0.3), (WINDOW, 3.0), (SPACE, 0.4)],
        "Slow Bloom" => &[(STRETCH, 50.0), (ATTACK, 6.0), (RELEASE, 10.0), (TONE, 6000.0), (SPACE, 0.45)],
        "Shimmer Drone" => &[(STRETCH, 100.0), (SHIMMER, 0.5), (SPACE, 0.6), (ATTACK, 2.0)],
        "Keep the Hits" => &[(STRETCH, 4.0), (WINDOW, 1.0), (ONSET, 0.8), (ATTACK, 0.01), (RELEASE, 0.5), (SPACE, 0.1)],
        "Dark Hall" => &[(STRETCH, 30.0), (TONE, 1500.0), (SPACE, 0.8), (VOLUME, 0.0)],
        "Wide Pad" => &[(STRETCH, 20.0), (WINDOW, 2.0), (VOICES, 4.0), (ATTACK, 1.5), (SPACE, 0.35)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

/// A window size's transform and window.
struct Size {
    plan: Plan,
    hann: Vec<f32>,
}

struct Tables {
    sizes: Vec<Size>,
    /// Phases to pick from: cos and sin.
    phases: Vec<(f32, f32)>,
    /// The sound it stretches without a sample.
    own: SampleRef,
}

static TABLES: OnceLock<Tables> = OnceLock::new();

/// Makes the transforms and its own sound (once). Call it before the audio
/// thread needs them.
pub fn warm() {
    tables();
}

fn tables() -> &'static Tables {
    TABLES.get_or_init(|| Tables {
        sizes: SIZES.iter().map(|&n| Size { plan: Plan::new(n), hann: (0..n).map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n as f32).cos()).collect() }).collect(),
        phases: (0..4096).map(|k| {
            let a = std::f32::consts::TAU * k as f32 / 4096.0;
            (a.cos(), a.sin())
        }).collect(),
        own: own_sound(),
    })
}

/// Its own sound: a chord on a soft organ-like tone that opens and closes
/// over six seconds, its notes coming in one by one.
fn own_sound() -> SampleRef {
    let sr = 48000u32;
    let n = 6 * sr as usize;
    let notes = [48.0f32, 55.0, 60.0, 64.0, 67.0, 71.0, 74.0];
    let mut l = vec![0.0f32; n];
    let mut r = vec![0.0f32; n];
    for (k, &note) in notes.iter().enumerate() {
        let f = 440.0 * 2f32.powf((note - 69.0) / 12.0);
        let start = k as f32 * 0.5;
        let pan = (k as f32 / (notes.len() - 1) as f32) * 0.8 + 0.1;
        for i in 0..n {
            let t = i as f32 / sr as f32;
            if t < start {
                continue;
            }
            let age = t - start;
            let env = (age / 0.4).min(1.0) * (-(age * 0.18)).exp();
            let bright = 0.5 + 0.5 * (t * 0.7).sin();
            let ph = std::f32::consts::TAU * f * t;
            let v = (ph.sin() + 0.5 * bright * (2.0 * ph).sin() + 0.25 * bright * (3.0 * ph).sin() + 0.12 * (5.0 * ph).sin()) * env * 0.09;
            l[i] += v * (1.0 - pan);
            r[i] += v * pan;
        }
    }
    SampleRef::new(SampleData::from_buffers("Stretch Chord", None, sr, l, r))
}

/// What it stretches: its sample, or its own sound.
pub fn source(sample: Option<&SampleRef>) -> &SampleData {
    match sample {
        Some(s) => s,
        None => &tables().own,
    }
}

fn size_index(p: &[f32]) -> usize {
    (p[WINDOW].round().max(0.0) as usize).min(SIZES.len() - 1)
}

#[inline]
fn interp(m: &[f32], x: f32) -> f32 {
    let i = x as usize;
    if i + 1 < m.len() {
        let t = x - i as f32;
        m[i] + (m[i + 1] - m[i]) * t
    } else if i < m.len() {
        m[i]
    } else {
        0.0
    }
}

/// One window of both channels, worked out a step at a time: read and
/// reorder, the passes of the transform, the spectrum (kept, moved to the
/// pitch, its phases at chance), the passes back. The result is in `re`
/// (left) and `im` (right, negated), not yet windowed or scaled.
pub struct Frame {
    re: Vec<f32>,
    im: Vec<f32>,
    ml: Vec<f32>,
    mr: Vec<f32>,
    pl: Vec<f32>,
    pr: Vec<f32>,
    prev: Vec<f32>,
    rng: Rng,
    /// Size index, the next step (None: idle or done).
    size: usize,
    step: Option<usize>,
    /// Where in the source it reads (frames), how the spectrum moves, how
    /// much octave goes in, how keen it is on onsets.
    at: f64,
    ratio: f32,
    shimmer: f32,
    onset: f32,
    /// Read past the ends: round again (true) or silence.
    wrap: bool,
    /// The window had something start in it (it kept its phases).
    pub hit: bool,
}

impl Frame {
    pub fn new(seed: u32) -> Self {
        let half = MAX_WINDOW / 2 + 1;
        Self {
            re: vec![0.0; MAX_WINDOW],
            im: vec![0.0; MAX_WINDOW],
            ml: vec![0.0; half],
            mr: vec![0.0; half],
            pl: vec![0.0; half],
            pr: vec![0.0; half],
            prev: vec![0.0; half],
            rng: Rng::new(seed),
            size: 0,
            step: None,
            at: 0.0,
            ratio: 1.0,
            shimmer: 0.0,
            onset: 0.0,
            wrap: true,
            hit: false,
        }
    }

    pub fn n(&self) -> usize {
        SIZES[self.size]
    }

    /// The steps a window of this size takes.
    pub fn steps(size: usize) -> usize {
        2 * SIZES[size].trailing_zeros() as usize + 2
    }

    pub fn busy(&self) -> bool {
        self.step.is_some()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start(&mut self, size: usize, at: f64, ratio: f32, shimmer: f32, onset: f32, wrap: bool) {
        self.size = size.min(SIZES.len() - 1);
        self.at = at;
        self.ratio = ratio.clamp(1.0 / 64.0, 64.0);
        self.shimmer = shimmer.clamp(0.0, 1.0);
        self.onset = onset.clamp(0.0, 1.0);
        self.wrap = wrap;
        self.hit = false;
        self.step = Some(0);
    }

    /// Does the next step; true when the window is done.
    pub fn work(&mut self, src: &SampleData) -> bool {
        let Some(step) = self.step else { return true };
        let t = tables();
        let size = &t.sizes[self.size];
        let n = size.plan.len();
        let passes = size.plan.passes();
        let (re, im) = (&mut self.re[..n], &mut self.im[..n]);
        if step == 0 {
            // The window read from the source, left as real and right as imaginary.
            let frames = src.frames() as i64;
            let first = self.at.floor() as i64;
            for i in 0..n {
                let mut k = first + i as i64;
                if self.wrap && frames > 0 {
                    k = k.rem_euclid(frames);
                }
                let (a, b) = if k >= 0 && k < frames { (src.left[k as usize], src.right[k as usize]) } else { (0.0, 0.0) };
                re[i] = a * size.hann[i];
                im[i] = b * size.hann[i];
            }
            size.plan.reorder(re, im);
        } else if step <= passes {
            size.plan.pass(step - 1, re, im);
        } else if step == passes + 1 {
            self.spectrum(n);
            let (re, im) = (&mut self.re[..n], &mut self.im[..n]);
            size.plan.reorder(re, im);
        } else {
            size.plan.pass(step - passes - 2, re, im);
        }
        let next = step + 1;
        if next >= 2 * passes + 2 {
            self.step = None;
            true
        } else {
            self.step = Some(next);
            false
        }
    }

    /// The whole window at once.
    pub fn finish(&mut self, src: &SampleData) {
        while !self.work(src) {}
    }

    /// From the transform of the window to the transform of the new one:
    /// the two channels apart, their levels kept (moved by the pitch, an
    /// octave up added), their phases at chance unless something starts here.
    fn spectrum(&mut self, n: usize) {
        let half = n / 2;
        let (re, im) = (&mut self.re[..n], &mut self.im[..n]);
        let mut flux = 0.0f32;
        let mut total = 0.0f32;
        for k in 0..=half {
            let (xr, xi) = (re[k], im[k]);
            let j = (n - k) % n;
            let (yr, yi) = (re[j], im[j]);
            // Left = (X[k] + conj X[n-k]) / 2, right = (X[k] - conj X[n-k]) / 2i.
            let (lr, li) = ((xr + yr) * 0.5, (xi - yi) * 0.5);
            let (rr, ri) = ((xi + yi) * 0.5, -(xr - yr) * 0.5);
            self.ml[k] = (lr * lr + li * li).sqrt();
            self.mr[k] = (rr * rr + ri * ri).sqrt();
            self.pl[k] = li.atan2(lr);
            self.pr[k] = ri.atan2(rr);
            let m = self.ml[k] + self.mr[k];
            flux += (m - self.prev[k]).max(0.0);
            total += m;
            self.prev[k] = m;
        }
        let flux = flux / total.max(1e-9);
        self.hit = self.onset > 0.0 && flux > 0.55 - 0.4 * self.onset;
        let phases = &tables().phases;
        let inv = 1.0 / self.ratio;
        let (ml, mr) = (&self.ml[..=half], &self.mr[..=half]);
        for j in 0..=half {
            let x = j as f32 * inv;
            let mut al = interp(ml, x);
            let mut ar = interp(mr, x);
            if self.shimmer > 0.0 {
                al += self.shimmer * interp(ml, x * 0.5);
                ar += self.shimmer * interp(mr, x * 0.5);
            }
            let ((cl, sl), (cr, sr)) = if self.hit {
                let k = (x.round() as usize).min(half);
                let (a, b) = (self.pl[k], self.pr[k]);
                ((a.cos(), a.sin()), (b.cos(), b.sin()))
            } else {
                (phases[(self.rng.next_u32() >> 20) as usize], phases[(self.rng.next_u32() >> 20) as usize])
            };
            let (alr, ali, arr, ari) = if j == 0 || j == half { (al * cl, 0.0, ar * cr, 0.0) } else { (al * cl, al * sl, ar * cr, ar * sr) };
            // Y = L + i R, both conjugate-symmetric; then conjugated for the way back.
            re[j] = alr - ari;
            im[j] = -(ali + arr);
            if j != 0 && j != half {
                re[n - j] = alr + ari;
                im[n - j] = -(arr - ali);
            }
        }
    }

    /// Adds the finished window into `out_l`/`out_r` from `at` on (round
    /// the end of the buffers), windowed and scaled.
    pub fn add_to(&self, out_l: &mut [f32], out_r: &mut [f32], at: usize, gain: f32) {
        let t = tables();
        let size = &t.sizes[self.size];
        let n = size.plan.len();
        let len = out_l.len().min(out_r.len());
        let g = gain * NORM / n as f32;
        for i in 0..n {
            let w = size.hann[i] * g;
            let k = (at + i) % len;
            out_l[k] += self.re[i] * w;
            out_r[k] -= self.im[i] * w;
        }
    }
}

/// Stretches stereo audio offline: `frames` frames out, the place read
/// starting at `from` (frames of the source) and moving `1 / factor` as fast
/// as the output (0: a drone held at `from`). `window` is an index into
/// [`SIZES`]. Thrown phases repeat with `seed`.
#[allow(clippy::too_many_arguments)]
pub fn render(src: &SampleData, from: f64, factor: f64, frames: usize, window: usize, onset: f32, seed: u32) -> (Vec<f32>, Vec<f32>) {
    warm();
    let n = SIZES[window.min(SIZES.len() - 1)];
    let hop = n / 4;
    let mut l = vec![0.0f32; frames + 2 * n];
    let mut r = vec![0.0f32; frames + 2 * n];
    let mut frame = Frame::new(seed.max(1));
    let speed = if factor > 0.0 { 1.0 / factor } else { 0.0 };
    // Each window is centred on the place it stands for: the first on `from`.
    let mut at = from - (n / 2) as f64;
    let mut out = 0usize;
    while out < frames + n / 2 {
        frame.start(window, at, 1.0, 0.0, onset, false);
        frame.finish(src);
        frame.add_to(&mut l, &mut r, out, 1.0);
        let step = if frame.hit { hop as f64 } else { hop as f64 * speed };
        at += step;
        out += hop;
    }
    // Centred: the output's middle window lines up with the source.
    let skip = n / 2;
    let l = l[skip.min(l.len())..].iter().copied().chain(std::iter::repeat(0.0)).take(frames).collect();
    let r = r[skip.min(r.len())..].iter().copied().chain(std::iter::repeat(0.0)).take(frames).collect();
    (l, r)
}

#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Off,
    Held,
    Released,
}

/// A key being stretched.
struct Voice {
    frame: Frame,
    /// The windows laid over each other, round and round.
    ola: [Vec<f32>; 2],
    read: usize,
    /// Samples until the next window starts (it must be in by then), and
    /// where in `ola` it goes.
    until: usize,
    due_at: usize,
    hop: usize,
    /// How far the place has moved since the key went down (frames of the source).
    run: f64,
    note: u8,
    vel: f32,
    stage: Stage,
    env: f32,
    age: u64,
}

const OLA: usize = 2 * MAX_WINDOW;

impl Voice {
    fn new(seed: u32) -> Self {
        Self {
            frame: Frame::new(seed),
            ola: [vec![0.0; OLA], vec![0.0; OLA]],
            read: 0,
            until: 0,
            due_at: 0,
            hop: 0,
            run: 0.0,
            note: 60,
            vel: 1.0,
            stage: Stage::Off,
            env: 0.0,
            age: 0,
        }
    }
}

/// The Stretch instrument on the audio thread.
pub struct StretchState {
    sr: f32,
    voices: Vec<Voice>,
    pub pitch_bend: f32,
    sustain: bool,
    tone: [OnePole; 2],
    reverb: Box<Reverb>,
    bl: Vec<f32>,
    br: Vec<f32>,
    counter: u64,
    frames: usize,
    /// The spectrum the screen shows (0..1 a band), and where the voices read.
    bands: [f32; BANDS],
    /// Windows that were not ready when due (finished then at once), and
    /// the most steps done in one call: what the tests watch.
    pub late: u32,
    pub most_steps: usize,
}

impl StretchState {
    pub fn new(sr: f32) -> Self {
        warm();
        Self {
            sr,
            voices: (0..MAX_VOICES).map(|k| Voice::new(0x5172_E7C4 ^ (k as u32).wrapping_mul(0x9E37_79B9))).collect(),
            pitch_bend: 0.0,
            sustain: false,
            tone: [OnePole::default(); 2],
            reverb: Box::new(Reverb::new(sr)),
            bl: vec![0.0; 256],
            br: vec![0.0; 256],
            counter: 0,
            frames: 0,
            bands: [0.0; BANDS],
            late: 0,
            most_steps: 0,
        }
    }

    /// The sustain pedal holds the place where it is (Freeze).
    pub fn sustain(&mut self, down: bool) {
        self.sustain = down;
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = (p[VOICES].round() as usize).clamp(1, MAX_VOICES);
        let pool = &self.voices[..n];
        let pick = pool
            .iter()
            .position(|v| v.stage != Stage::Off && v.note == note)
            .or_else(|| pool.iter().position(|v| v.stage == Stage::Off))
            .or_else(|| pool.iter().enumerate().filter(|(_, v)| v.stage == Stage::Released).min_by_key(|(_, v)| v.age).map(|(i, _)| i))
            .unwrap_or_else(|| pool.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0));
        self.counter += 1;
        let v = &mut self.voices[pick];
        let fresh = v.stage == Stage::Off;
        v.note = note;
        v.vel = 1.0 - p[VELOCITY].clamp(0.0, 1.0) * (1.0 - vel.clamp(0.0, 1.0));
        v.stage = Stage::Held;
        v.age = self.counter;
        if fresh {
            for b in v.ola.iter_mut() {
                b.iter_mut().for_each(|x| *x = 0.0);
            }
            v.env = 0.0;
            v.run = 0.0;
            v.read = 0;
            v.frame.step = None;
            v.until = 0;
            v.hop = 0;
        }
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        for v in self.voices.iter_mut().filter(|v| v.stage == Stage::Held && v.note == note) {
            v.stage = Stage::Released;
        }
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut().filter(|v| v.stage == Stage::Held) {
            v.stage = Stage::Released;
        }
    }

    pub fn voices_on(&self) -> usize {
        self.voices.iter().filter(|v| v.stage != Stage::Off).count()
    }

    /// For the screen: voices sounding, the sample's length, each voice's
    /// place (0..1, -1: none), whether it is frozen, and the spectrum.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.voices_on() as f32;
        out[1] = self.frames as f32;
        for (k, v) in self.voices.iter().enumerate() {
            out[2 + k] = if v.stage != Stage::Off { v.frame.at as f32 } else { -1.0 };
        }
        out[6] = self.sustain as u8 as f32;
        for (k, b) in self.bands.iter().enumerate() {
            out[BANDS_AT + k] = *b;
        }
    }

    pub fn render(&mut self, p: &[f32], sample: Option<&SampleRef>, l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let src = source(sample);
        let frames = src.frames();
        if frames < 16 {
            return;
        }
        self.frames = frames;
        let n = l.len().min(r.len());
        let sr = self.sr;
        let size = size_index(p);
        let win = SIZES[size];
        let steps_per = Frame::steps(size);
        let rate = src.sample_rate as f64 / sr as f64;
        let factor = p[STRETCH].clamp(1.0, 1000.0) as f64;
        let frozen = p[FREEZE] >= 0.5 || self.sustain;
        let base = p[POSITION].clamp(0.0, 1.0) as f64 * frames as f64;
        let shimmer = p[SHIMMER];
        let onset = p[ONSET];
        let attack = 1.0 / (p[ATTACK].max(0.005) * sr);
        let release = 1.0 / (p[RELEASE].max(0.01) * sr);
        let gain = db_to_gain(p[VOLUME]);
        let mut steps_done = 0;
        let mut done = 0;
        while done < n {
            let m = (n - done).min(self.bl.len());
            self.bl[..m].iter_mut().for_each(|x| *x = 0.0);
            self.br[..m].iter_mut().for_each(|x| *x = 0.0);
            for v in self.voices.iter_mut().filter(|v| v.stage != Stage::Off) {
                let semis = if p[KEYS] >= 0.5 { v.note as f32 - p[ROOT].round() } else { 0.0 } + p[PITCH].round() + self.pitch_bend;
                let ratio = 2f32.powf(semis / 12.0) / rate as f32;
                let mut i = 0;
                while i < m {
                    if v.until == 0 {
                        // The window for here must be in: finish it if it is not.
                        if v.frame.busy() {
                            while !v.frame.work(src) {
                                steps_done += 1;
                            }
                            steps_done += 1;
                            if v.hop > 0 {
                                self.late += 1;
                            }
                            let [ol, or] = &mut v.ola;
                            v.frame.add_to(ol, or, v.due_at, 1.0);
                            Self::note_bands(&mut self.bands, &v.frame);
                            let step = if v.frame.hit { v.hop as f64 * rate } else { v.hop as f64 * rate / factor };
                            if !frozen {
                                v.run += step;
                            }
                        }
                        // The next window: a quarter on.
                        v.hop = win / 4;
                        v.until = v.hop;
                        v.due_at = (v.read + v.until) % OLA;
                        let at = (base + v.run).rem_euclid(frames as f64);
                        v.frame.start(size, at, ratio, shimmer, onset, true);
                    }
                    let k = (m - i).min(v.until);
                    // Work on the window coming, enough to be ready in time (twice as fast).
                    if v.frame.busy() {
                        let budget = ((k * steps_per * 2).div_ceil(v.hop.max(1))).max(1);
                        for _ in 0..budget {
                            steps_done += 1;
                            if v.frame.work(src) {
                                let [ol, or] = &mut v.ola;
                            v.frame.add_to(ol, or, v.due_at, 1.0);
                                Self::note_bands(&mut self.bands, &v.frame);
                                let step = if v.frame.hit { v.hop as f64 * rate } else { v.hop as f64 * rate / factor };
                                if !frozen {
                                    v.run += step;
                                }
                                break;
                            }
                        }
                    }
                    for j in 0..k {
                        match v.stage {
                            Stage::Held => v.env = (v.env + attack).min(1.0),
                            _ => v.env = (v.env - release).max(0.0),
                        }
                        let g = v.env * v.env * v.vel;
                        let a = v.read;
                        self.bl[i + j] += v.ola[0][a] * g;
                        self.br[i + j] += v.ola[1][a] * g;
                        v.ola[0][a] = 0.0;
                        v.ola[1][a] = 0.0;
                        v.read = (a + 1) % OLA;
                    }
                    v.until -= k;
                    i += k;
                    if v.stage == Stage::Released && v.env <= 0.0 {
                        v.stage = Stage::Off;
                        v.frame.step = None;
                        break;
                    }
                }
            }
            // The tone and the space after the voices.
            let cutoff = p[TONE].clamp(200.0, 20000.0);
            if cutoff < 19_000.0 {
                for f in self.tone.iter_mut() {
                    f.set(cutoff, sr);
                }
                for i in 0..m {
                    self.bl[i] = self.tone[0].lp(self.bl[i]);
                    self.br[i] = self.tone[1].lp(self.br[i]);
                }
            }
            let space = p[SPACE].clamp(0.0, 1.0);
            if space > 0.001 {
                self.reverb.process(0.6 + 0.35 * space, 0.5, 1.0, 20.0, 150.0, space * 0.55, &mut self.bl[..m], &mut self.br[..m]);
            }
            for i in 0..m {
                l[done + i] += self.bl[i] * gain;
                r[done + i] += self.br[i] * gain;
            }
            done += m;
        }
        self.most_steps = self.most_steps.max(steps_done);
        for b in self.bands.iter_mut() {
            *b *= 0.995;
        }
    }

    /// The finished window's spectrum into the screen's bands (log spaced,
    /// 40 Hz to 16 kHz at 48 kHz), as levels from 0 to 1 (60 dB).
    fn note_bands(bands: &mut [f32; BANDS], f: &Frame) {
        let n = f.n();
        let half = n / 2;
        for (b, out) in bands.iter_mut().enumerate() {
            let lo = (40.0 * (400.0f32).powf(b as f32 / BANDS as f32) / 24000.0 * half as f32) as usize;
            let hi = ((40.0 * (400.0f32).powf((b + 1) as f32 / BANDS as f32) / 24000.0 * half as f32) as usize).max(lo + 1).min(half);
            let m = f.ml[lo.min(half)..hi].iter().zip(&f.mr[lo.min(half)..hi]).map(|(a, b)| a + b).fold(0.0f32, f32::max);
            let db = 20.0 * (m * 4.0 / n as f32).max(1e-6).log10();
            let v = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
            *out = out.max(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::fx::test_util::strongest;

    fn sine(freq: f32, secs: f32, amp: f32) -> SampleData {
        let n = (secs * 48000.0) as usize;
        let l: Vec<f32> = (0..n).map(|i| amp * (std::f32::consts::TAU * freq * i as f32 / 48000.0).sin()).collect();
        SampleData::from_buffers("sine", None, 48000, l.clone(), l)
    }

    fn rms(v: &[f32]) -> f32 {
        (v.iter().map(|x| x * x).sum::<f32>() / v.len().max(1) as f32).sqrt()
    }

    #[test]
    fn an_offline_stretch_is_factor_times_as_long_and_keeps_the_pitch() {
        let s = sine(440.0, 1.0, 0.5);
        let (l, r) = render(&s, 0.0, 4.0, 4 * s.frames(), 2, 0.0, 7);
        assert_eq!(l.len(), 4 * s.frames());
        assert_eq!(r.len(), l.len());
        assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 4.0));
        let mid = &l[48000..96000];
        let f = strongest(mid, 300.0, 600.0, 2.0);
        assert!((f - 440.0).abs() < 6.0, "the sine stays at its frequency: {f}");
        // About as loud as it was (within a few dB).
        let level = rms(mid) / rms(&s.left);
        assert!((0.5..2.0).contains(&level), "level {level}");
    }

    #[test]
    fn a_frozen_moment_holds_a_steady_spectrum() {
        // Two tones, one after the other: frozen in the first, only it sounds.
        let mut s = sine(330.0, 1.0, 0.4);
        let later = sine(880.0, 1.0, 0.4);
        let mut l = s.left.clone();
        l.extend_from_slice(&later.left);
        s = SampleData::from_buffers("two", None, 48000, l.clone(), l);
        let (out, _) = render(&s, 24000.0, 0.0, 48000 * 4, 2, 0.0, 3);
        let a = &out[24000..72000];
        let b = &out[120000..168000];
        for part in [a, b] {
            let f = strongest(part, 200.0, 1000.0, 5.0);
            assert!((f - 330.0).abs() < 8.0, "frozen at the first tone: {f}");
        }
        let (ra, rb) = (rms(a), rms(b));
        assert!((ra / rb - 1.0).abs() < 0.35, "steady: {ra} against {rb}");
    }

    #[test]
    fn the_instrument_plays_in_time_without_blowing_up() {
        let mut st = StretchState::new(48000.0);
        let mut p = preset("Endless");
        p[WINDOW] = 4.0; // the biggest window
        p[VOICES] = 4.0;
        p[SPACE] = 0.0;
        p[ATTACK] = 0.01;
        let s = SampleRef::new(sine(220.0, 2.0, 0.5));
        for note in [60, 64, 67, 72] {
            st.note_on(&p, note, 1.0);
        }
        let mut out = Vec::new();
        let started = std::time::Instant::now();
        for _ in 0..(48000 * 4 / 256) {
            let mut l = vec![0.0; 256];
            let mut r = vec![0.0; 256];
            st.render(&p, Some(&s), &mut l, &mut r);
            out.extend_from_slice(&l);
        }
        let took = started.elapsed();
        assert!(out.iter().all(|v| v.is_finite() && v.abs() < 8.0));
        assert!(rms(&out[96000..]) > 0.01, "it sounds");
        assert_eq!(st.late, 0, "every window ready in time");
        // A block of 256 never does more than a few steps a voice: here 2 each.
        assert!(st.most_steps <= 4 * 3, "steps in one block: {}", st.most_steps);
        eprintln!("four voices of 0.68 s windows, 4 s: {took:?} (debug build)");
        // The fourth key (an octave up from the root) plays an octave above the first.
        let mut solo = StretchState::new(48000.0);
        solo.note_on(&p, 72, 1.0);
        let mut one = Vec::new();
        for _ in 0..(48000 * 3 / 256) {
            let mut l = vec![0.0; 256];
            let mut r = vec![0.0; 256];
            solo.render(&p, Some(&s), &mut l, &mut r);
            one.extend_from_slice(&l);
        }
        let f = strongest(&one[96000..], 300.0, 600.0, 2.0);
        assert!((f - 440.0).abs() < 8.0, "C5 over a C4 root moves 220 Hz to 440: {f}");
        st.all_off();
    }
}
