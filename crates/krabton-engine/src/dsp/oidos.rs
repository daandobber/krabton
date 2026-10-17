//! Oidos: an additive synthesizer. A Rust port of the synth of Oidos by
//! Aske Simon Christensen (Blueberry), made for the music of 4k and 8k
//! intros: <https://github.com/askeksa/Oidos>.
//!
//! A note is hundreds or thousands of sine waves (partials) in groups
//! (modes): each mode a randomly chosen overtone of the note, pulled
//! towards the true overtones as far as Harmonicity says, loud or soft by
//! its height (Sharpness), dying away at its own rate (between Decay Low
//! and Decay High); each partial of a mode a little off its centre (Width).
//! A filter on the partials (its edges sloped, and sweeping) and a soft
//! saturation (Gain) on their sum. The Seed decides every random choice,
//! so the same settings always make the same sound.
//!
//! Altered from the original (as its license asks to be said): it plays in
//! real time, every partial worked out when its note starts, in single
//! precision (each partial's level put right every 128 samples); partials
//! the filter shuts out for good, those above the highest frequency the
//! sample rate can hold, and those that have died away are left out; notes
//! sound at the keys' pitch (the original plays an octave up); the
//! partials can spread across the stereo field; attack and release in
//! milliseconds; voices, transpose, velocity and volume added.
//!
//! The original's license:
//!
//! The zlib/libpng License. Copyright (c) 2016-2017 Aske Simon Christensen
//!
//! This software is provided 'as-is', without any express or implied
//! warranty. In no event will the authors be held liable for any damages
//! arising from the use of this software.
//!
//! Permission is granted to anyone to use this software for any purpose,
//! including commercial applications, and to alter it and redistribute it
//! freely, subject to the following restrictions:
//!
//! 1. The origin of this software must not be misrepresented; you must not
//!    claim that you wrote the original software. If you use this software
//!    in a product, an acknowledgment in the product documentation would be
//!    appreciated but is not required.
//! 2. Altered source versions must be plainly marked as such, and must not
//!    be misrepresented as being the original software.
//! 3. This notice may not be removed or altered from any source
//!    distribution.

use crate::params::{ParamSpec, Unit};
use std::sync::OnceLock;

pub const SEED: usize = 0;
pub const MODES: usize = 1;
pub const FAT: usize = 2;
pub const WIDTH: usize = 3;
pub const OVERTONES: usize = 4;
pub const SHARPNESS: usize = 5;
pub const HARMONICITY: usize = 6;
pub const DECAY_LOW: usize = 7;
pub const DECAY_HIGH: usize = 8;
pub const FILTER_LOW: usize = 9;
pub const SLOPE_LOW: usize = 10;
pub const SWEEP_LOW: usize = 11;
pub const FILTER_HIGH: usize = 12;
pub const SLOPE_HIGH: usize = 13;
pub const SWEEP_HIGH: usize = 14;
pub const GAIN: usize = 15;
pub const ATTACK: usize = 16;
pub const RELEASE: usize = 17;
pub const SPREAD: usize = 18;
pub const VOICES: usize = 19;
pub const TRANSPOSE: usize = 20;
pub const VELOCITY: usize = 21;
pub const VOLUME: usize = 22;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::int("Seed", "Partials", 0.0, 100.0, 50.0, Unit::None),
    ParamSpec::int("Modes", "Partials", 1.0, 100.0, 40.0, Unit::None),
    ParamSpec::int("Fat", "Partials", 1.0, 100.0, 10.0, Unit::None),
    ParamSpec::lin("Width", "Partials", 0.0, 1.0, 0.34, Unit::Percent),
    ParamSpec::int("Overtones", "Partials", 0.0, 100.0, 27.0, Unit::Semi),
    ParamSpec::lin("Sharpness", "Partials", 0.0, 1.0, 0.9, Unit::Percent),
    ParamSpec::lin("Harmonicity", "Partials", -1.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Decay Low", "Decay", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Decay High", "Decay", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Filter Low", "Filter", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Slope Low", "Filter", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Sweep Low", "Filter", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Filter High", "Filter", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Slope High", "Filter", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Sweep High", "Filter", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Gain", "Amp", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::log("Attack", "Amp", 1.0, 4000.0, 62.0, Unit::Ms),
    ParamSpec::log("Release", "Amp", 1.0, 8000.0, 500.0, Unit::Ms),
    ParamSpec::lin("Spread", "Play", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Voices", "Play", 1.0, 12.0, 8.0, Unit::None),
    ParamSpec::int("Transpose", "Play", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Velocity", "Amp", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Volume", "Amp", -24.0, 6.0, 0.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Init", "Glass Pad", "Choir", "Opening Pad", "Deep Drone", "Organ Cloud", "Breath", "Shimmer", "Sub Swell", "Bell Cluster", "Gong", "Metal Rain", "Pluck", "Snare", "Hi Hat", "Fat Saw"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Glass Pad" => &[(SEED, 12.0), (MODES, 30.0), (FAT, 6.0), (WIDTH, 0.25), (OVERTONES, 36.0), (SHARPNESS, 0.75), (FILTER_HIGH, 0.62), (SLOPE_HIGH, 0.3), (ATTACK, 400.0), (RELEASE, 2500.0)],
        "Choir" => &[(SEED, 33.0), (MODES, 20.0), (FAT, 20.0), (WIDTH, 0.45), (OVERTONES, 48.0), (SHARPNESS, 0.7), (FILTER_LOW, 0.48), (SLOPE_LOW, 0.4), (ATTACK, 300.0), (RELEASE, 1500.0)],
        "Opening Pad" => &[(SEED, 7.0), (MODES, 36.0), (FAT, 8.0), (WIDTH, 0.33), (OVERTONES, 60.0), (SHARPNESS, 0.85), (FILTER_HIGH, 0.5), (SLOPE_HIGH, 0.5), (SWEEP_HIGH, 0.6), (ATTACK, 800.0), (RELEASE, 3000.0)],
        "Deep Drone" => &[(SEED, 71.0), (MODES, 12.0), (FAT, 30.0), (WIDTH, 0.5), (OVERTONES, 24.0), (SHARPNESS, 0.6), (GAIN, 0.4), (ATTACK, 2000.0), (RELEASE, 4000.0), (TRANSPOSE, -12.0)],
        "Organ Cloud" => &[(SEED, 2.0), (MODES, 8.0), (FAT, 8.0), (WIDTH, 0.2), (OVERTONES, 36.0), (SHARPNESS, 0.7), (ATTACK, 50.0), (RELEASE, 400.0)],
        "Breath" => &[(SEED, 45.0), (MODES, 30.0), (FAT, 8.0), (WIDTH, 0.7), (OVERTONES, 40.0), (SHARPNESS, 0.8), (HARMONICITY, 0.2), (FILTER_LOW, 0.5), (SLOPE_LOW, 0.6), (FILTER_HIGH, 0.62), (SLOPE_HIGH, 0.6), (ATTACK, 300.0), (RELEASE, 900.0)],
        "Shimmer" => &[(SEED, 88.0), (MODES, 50.0), (FAT, 4.0), (WIDTH, 0.3), (OVERTONES, 72.0), (SHARPNESS, 1.0), (FILTER_LOW, 0.6), (SLOPE_LOW, 0.5), (DECAY_HIGH, 0.98), (ATTACK, 600.0), (RELEASE, 3000.0), (SPREAD, 1.0)],
        "Sub Swell" => &[(SEED, 5.0), (MODES, 4.0), (FAT, 12.0), (WIDTH, 0.4), (OVERTONES, 12.0), (SHARPNESS, 0.4), (GAIN, 0.45), (ATTACK, 1500.0), (RELEASE, 2000.0), (SPREAD, 0.2)],
        "Bell Cluster" => &[(SEED, 24.0), (MODES, 24.0), (FAT, 3.0), (WIDTH, 0.15), (OVERTONES, 60.0), (SHARPNESS, 0.85), (HARMONICITY, -0.4), (DECAY_LOW, 0.97), (DECAY_HIGH, 0.9), (ATTACK, 1.0), (RELEASE, 3000.0)],
        "Gong" => &[(SEED, 61.0), (MODES, 60.0), (FAT, 4.0), (WIDTH, 0.3), (OVERTONES, 70.0), (SHARPNESS, 0.85), (HARMONICITY, -0.6), (DECAY_LOW, 0.995), (DECAY_HIGH, 0.95), (GAIN, 0.35), (ATTACK, 5.0), (RELEASE, 4000.0), (TRANSPOSE, -12.0)],
        "Metal Rain" => &[(SEED, 17.0), (MODES, 40.0), (FAT, 3.0), (WIDTH, 0.2), (OVERTONES, 80.0), (SHARPNESS, 0.9), (HARMONICITY, -1.0), (DECAY_LOW, 0.85), (DECAY_HIGH, 0.6), (ATTACK, 1.0), (RELEASE, 300.0)],
        "Pluck" => &[(SEED, 40.0), (MODES, 24.0), (FAT, 4.0), (WIDTH, 0.2), (OVERTONES, 48.0), (SHARPNESS, 0.8), (DECAY_LOW, 0.9), (DECAY_HIGH, 0.55), (ATTACK, 2.0), (RELEASE, 250.0), (VELOCITY, 1.0)],
        "Snare" => &[(SEED, 55.0), (MODES, 60.0), (FAT, 5.0), (WIDTH, 0.6), (OVERTONES, 80.0), (SHARPNESS, 0.9), (HARMONICITY, 0.0), (DECAY_LOW, 0.6), (DECAY_HIGH, 0.5), (GAIN, 0.4), (ATTACK, 1.0), (RELEASE, 120.0), (VELOCITY, 1.0)],
        "Hi Hat" => &[(SEED, 80.0), (MODES, 80.0), (FAT, 3.0), (WIDTH, 0.6), (OVERTONES, 100.0), (SHARPNESS, 1.0), (HARMONICITY, 0.0), (DECAY_LOW, 0.5), (DECAY_HIGH, 0.3), (FILTER_LOW, 0.65), (SLOPE_LOW, 0.5), (ATTACK, 1.0), (RELEASE, 80.0), (TRANSPOSE, 12.0), (VELOCITY, 1.0)],
        "Fat Saw" => &[(SEED, 9.0), (MODES, 48.0), (FAT, 7.0), (WIDTH, 0.4), (OVERTONES, 64.0), (SHARPNESS, 0.8), (HARMONICITY, 1.0), (GAIN, 0.5), (ATTACK, 10.0), (RELEASE, 300.0)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

fn get(p: &[f32], i: usize) -> f32 {
    p.get(i).copied().unwrap_or(SPECS[i].default)
}

// ---------------------------------------------------------------- the partials

const TOTAL_SEMITONES: f64 = 120.0;
const NOISESIZE: usize = 64;
/// The original's decay is per this many seconds (4096 samples at 44.1 kHz).
const DECAY_TIME: f64 = 4096.0 / 44100.0;
/// No note has more partials than this.
pub const MAX_PARTIALS: usize = 4000;

/// The original's random numbers, the same on every run.
fn random_data() -> &'static [u32] {
    static DATA: OnceLock<Vec<u32>> = OnceLock::new();
    DATA.get_or_init(|| {
        let mut data = Vec::with_capacity(NOISESIZE * NOISESIZE * NOISESIZE);
        let mut state: [u32; 4] = [0x6F15AAF2, 0x4E89D208, 0x9548B49A, 0x9C4FD335];
        for _ in 0..NOISESIZE * NOISESIZE * NOISESIZE {
            let mut r = 0u32;
            for s in 0..3 {
                let rs = state[s].rotate_right(state[s]).wrapping_add(state[s + 1]);
                state[s] = rs;
                r ^= rs;
            }
            data.push(r);
        }
        data
    })
}

/// One partial as a note starts.
#[derive(Clone, Copy, Debug)]
pub struct Partial {
    /// Its pitch (MIDI note, fractional) and its mode.
    pub tone: f32,
    pub mode: u16,
    /// Its level and phase at the start, and how much it keeps a sample.
    pub amp: f64,
    pub phase: f64,
    pub keep: f64,
    /// The filter's weight on it at the start (0 to 1: shut to open; beyond
    /// either, it takes time for a sweep to reach the edge) and its change a second.
    pub low: f64,
    pub high: f64,
    /// Its place in the stereo field (-1 to 1).
    pub pan: f32,
}

/// The settings that shape a note's partials.
#[derive(Clone, Copy, Debug)]
struct Shape {
    modes: usize,
    fat: usize,
    seed: usize,
    overtones: f64,
    decay_low: f64,
    decay_diff: f64,
    harmonicity: f64,
    sharpness: f64,
    width: f64,
    f_low: f64,
    slope_low: f64,
    sweep_low: f64,
    f_high: f64,
    slope_high: f64,
    sweep_high: f64,
}

impl Shape {
    fn of(p: &[f32]) -> Self {
        let modes = get(p, MODES).round().clamp(1.0, 100.0) as usize;
        let fat = (get(p, FAT).round().clamp(1.0, 100.0) as usize).min((MAX_PARTIALS / modes).max(1));
        let decay_low = get(p, DECAY_LOW).clamp(0.0, 1.0) as f64;
        Self {
            modes,
            fat,
            seed: get(p, SEED).round().clamp(0.0, 100.0) as usize,
            overtones: get(p, OVERTONES).round().clamp(0.0, 100.0) as f64,
            decay_low,
            decay_diff: get(p, DECAY_HIGH).clamp(0.0, 1.0) as f64 - decay_low,
            harmonicity: get(p, HARMONICITY).clamp(-1.0, 1.0) as f64,
            sharpness: get(p, SHARPNESS).clamp(0.0, 1.0) as f64 * 5.0 - 4.0,
            width: (get(p, WIDTH).clamp(0.0, 1.0) as f64).powi(5) * 100.0,
            f_low: (get(p, FILTER_LOW).clamp(0.0, 1.0) as f64 * 2.0 - 1.0) * TOTAL_SEMITONES,
            slope_low: (1.0 - get(p, SLOPE_LOW).clamp(0.0, 1.0) as f64).powi(3),
            sweep_low: (get(p, SWEEP_LOW).clamp(0.0, 1.0) as f64 - 0.5).powi(3) * TOTAL_SEMITONES * 100.0,
            f_high: (get(p, FILTER_HIGH).clamp(0.0, 1.0) as f64 * 2.0 - 1.0) * TOTAL_SEMITONES,
            slope_high: (1.0 - get(p, SLOPE_HIGH).clamp(0.0, 1.0) as f64).powi(3),
            sweep_high: (get(p, SWEEP_HIGH).clamp(0.0, 1.0) as f64 - 0.5).powi(3) * TOTAL_SEMITONES * 100.0,
        }
    }

    /// How fast the filter's edges move: their weight's change a second.
    fn sweeps(&self) -> (f64, f64) {
        (-self.sweep_low * self.slope_low, self.sweep_high * self.slope_high)
    }
}

/// The partials of a note (MIDI `tone`) at sample rate `sr`, as the
/// original makes them; `all` keeps those that would be left out.
pub fn partials(p: &[f32], tone: f32, sr: f32, all: bool) -> Vec<Partial> {
    partials_and_energy(p, tone, sr, all).0
}

/// The partials, and the sum of every partial's level squared (those left
/// out too): the loudness is set against it.
fn partials_and_energy(p: &[f32], tone: f32, sr: f32, all: bool) -> (Vec<Partial>, f64) {
    let s = Shape::of(p);
    let random = random_data();
    let (add_low, add_high) = s.sweeps();
    let f_lowlimit = s.f_low + tone as f64;
    let f_highlimit = s.f_high + tone as f64;
    let nyquist_tone = 69.0 + 12.0 * (0.45 * sr as f64 / 440.0).log2();
    let spread_seed = (s.seed as u32).wrapping_mul(0x9E37_79B9) ^ 0x5bd1_e995;
    let mut out = Vec::with_capacity(s.modes * s.fat);
    let mut energy = 0.0;
    for m in 0..s.modes {
        let mut index = m * 256 + s.seed;
        let mut rnd = || {
            let r = random[index % random.len()];
            index += 1;
            r as i32 as f64 / 0x8000_0000u32 as f64
        };
        let subtone = rnd().abs();
        let reltone = subtone * s.overtones;
        let decay = (s.decay_low + subtone * s.decay_diff).clamp(0.0, 1.0);
        let keep = decay.powf(1.0 / DECAY_TIME / sr as f64);
        let relfreq = 2f64.powf(reltone / 12.0);
        let relfreq_ot = (relfreq + 0.5).floor();
        let relfreq_h = relfreq + (relfreq_ot - relfreq) * s.harmonicity;
        let reltone = relfreq_h.max(1e-6).log2() * 12.0;
        let mtone = tone as f64 + reltone;
        let mamp = rnd() * 2f64.powf(reltone * s.sharpness / 12.0);
        for k in 0..s.fat {
            energy += mamp * mamp;
            let ptone = mtone + rnd() * s.width;
            let phase = rnd() * std::f64::consts::PI;
            let low = 1.0 - (f_lowlimit - ptone) * s.slope_low;
            let high = 1.0 - (ptone - f_highlimit) * s.slope_high;
            if !all {
                // Above what the sample rate holds; shut out by the filter for good; silent.
                let shut = (low <= 0.0 && add_low <= 0.0) || (high <= 0.0 && add_high <= 0.0);
                if ptone >= nyquist_tone || shut || mamp == 0.0 || keep == 0.0 {
                    continue;
                }
            }
            let h = (spread_seed ^ ((m * 131 + k) as u32).wrapping_mul(0x85eb_ca6b)).wrapping_mul(0xc2b2_ae35);
            let pan = ((h >> 8) as f32 / (1u32 << 24) as f32) * 2.0 - 1.0;
            out.push(Partial { tone: ptone as f32, mode: m as u16, amp: mamp, phase, keep, low, high, pan });
        }
    }
    (out, energy)
}

// ---------------------------------------------------------------- the player

/// Samples between putting each partial's level right.
const BLOCK: usize = 128;

/// Eight partials side by side (the inner loop works on them together).
#[derive(Clone, Copy, Default)]
struct Lanes {
    re: [f32; 8],
    im: [f32; 8],
    c: [f32; 8],
    d: [f32; 8],
    wl: [f32; 8],
    wr: [f32; 8],
}

#[derive(Default)]
struct Voice {
    note: u8,
    vel: f32,
    gate: bool,
    active: bool,
    age: u64,
    /// The attack's ramp (0 to 1) and the release's (1 to 0).
    env: f32,
    rel: f32,
    t: usize,
    /// The partials, eight to a lane: each as a rotating pair (where it is,
    /// its turn a sample) and its stereo weights with the filter's weight
    /// on them (set once a block: the filter moves slowly).
    lanes: Vec<Lanes>,
    n: usize,
    /// Per partial: the filter's weights at the start, its stereo weights,
    /// its level as it should be and what it keeps a block.
    low0: Vec<f32>,
    high0: Vec<f32>,
    left: Vec<f32>,
    right: Vec<f32>,
    level: Vec<f32>,
    keep_block: Vec<f32>,
    add_low: f32,
    add_high: f32,
    /// What the loudness is set against: the original counts the partials;
    /// here their levels count too (three times the sum of their squares:
    /// the count, for partials of random level), so sharp sounds of high
    /// overtones are not many times louder than soft ones.
    norm: f32,
}

impl Voice {
    fn extras(&mut self) -> [&mut Vec<f32>; 6] {
        [&mut self.low0, &mut self.high0, &mut self.left, &mut self.right, &mut self.level, &mut self.keep_block]
    }

    fn start(&mut self, p: &[f32], note: u8, tone: f32, vel: f32, sr: f32) {
        let (parts, energy) = partials_and_energy(p, tone, sr, false);
        let spread = get(p, SPREAD).clamp(0.0, 1.0);
        let base = std::f64::consts::TAU * 440.0 / sr as f64;
        let s = Shape::of(p);
        let (add_low, add_high) = s.sweeps();
        let n = parts.len();
        for v in self.extras() {
            v.clear();
            v.reserve(n);
        }
        self.lanes.clear();
        self.lanes.resize(n.div_ceil(8), Lanes::default());
        for (i, q) in parts.iter().enumerate() {
            let w = base * 2f64.powf((q.tone as f64 - 69.0) / 12.0);
            let ln = &mut self.lanes[i / 8];
            let j = i % 8;
            ln.c[j] = (q.keep * w.cos()) as f32;
            ln.d[j] = (q.keep * w.sin()) as f32;
            ln.re[j] = (q.amp * q.phase.cos()) as f32;
            ln.im[j] = (q.amp * q.phase.sin()) as f32;
            self.low0.push(q.low as f32);
            self.high0.push(q.high as f32);
            self.left.push(1.0 - spread * q.pan);
            self.right.push(1.0 + spread * q.pan);
            self.level.push(q.amp.abs() as f32);
            self.keep_block.push(q.keep.powi(BLOCK as i32) as f32);
        }
        self.n = n;
        self.add_low = (add_low / sr as f64) as f32;
        self.add_high = (add_high / sr as f64) as f32;
        self.norm = (3.0 * energy) as f32;
        self.note = note;
        self.vel = vel;
        self.gate = true;
        self.active = true;
        self.env = 0.0;
        self.rel = 1.0;
        self.t = 0;
        self.weigh();
    }

    fn partials(&self) -> usize {
        self.n
    }

    /// The filter's weights for the block ahead (at its middle), on the
    /// stereo weights; partials it has shut out for good dropped.
    fn weigh(&mut self) {
        let t = self.t as f32 + BLOCK as f32 * 0.5;
        let mut i = 0;
        while i < self.n {
            let low = self.low0[i] + self.add_low * t;
            let high = self.high0[i] + self.add_high * t;
            if (low <= 0.0 && self.add_low <= 0.0) || (high <= 0.0 && self.add_high <= 0.0) {
                self.drop_partial(i);
                continue;
            }
            let f = low.min(high).clamp(0.0, 1.0);
            let ln = &mut self.lanes[i / 8];
            ln.wl[i % 8] = self.left[i] * f;
            ln.wr[i % 8] = self.right[i] * f;
            i += 1;
        }
    }

    /// After a block: every partial's level put right, the silent ones
    /// dropped, the filter's weights for the next.
    fn settle(&mut self) {
        let mut i = 0;
        while i < self.n {
            self.level[i] *= self.keep_block[i];
            let target = self.level[i];
            let ln = &mut self.lanes[i / 8];
            let j = i % 8;
            let now = (ln.re[j] * ln.re[j] + ln.im[j] * ln.im[j]).sqrt();
            if target < 1e-6 || now < 1e-12 {
                self.drop_partial(i);
                continue;
            }
            let k = target / now;
            ln.re[j] *= k;
            ln.im[j] *= k;
            i += 1;
        }
        self.weigh();
    }

    /// The last partial takes partial `i`'s place.
    fn drop_partial(&mut self, i: usize) {
        let last = self.n - 1;
        let (lo, lj) = (last / 8, last % 8);
        let src = self.lanes[lo];
        let ln = &mut self.lanes[i / 8];
        let j = i % 8;
        ln.re[j] = src.re[lj];
        ln.im[j] = src.im[lj];
        ln.c[j] = src.c[lj];
        ln.d[j] = src.d[lj];
        ln.wl[j] = src.wl[lj];
        ln.wr[j] = src.wr[lj];
        let end = &mut self.lanes[lo];
        end.re[lj] = 0.0;
        end.im[lj] = 0.0;
        end.wl[lj] = 0.0;
        end.wr[lj] = 0.0;
        for v in self.extras() {
            v.swap_remove(i);
        }
        self.n = last;
        self.lanes.truncate(self.n.div_ceil(8));
    }

    /// The partials' sums for as many samples as `l` holds (at most to the block's end).
    fn run(&mut self, l: &mut [f32], r: &mut [f32]) {
        for (ol, or) in l.iter_mut().zip(r.iter_mut()) {
            // (eight running sums, so it vectorizes)
            let mut acc_l = [0.0f32; 8];
            let mut acc_r = [0.0f32; 8];
            for ln in self.lanes.iter_mut() {
                for j in 0..8 {
                    let (a, b) = (ln.re[j], ln.im[j]);
                    let re = a * ln.c[j] - b * ln.d[j];
                    ln.im[j] = a * ln.d[j] + b * ln.c[j];
                    ln.re[j] = re;
                    acc_l[j] += re * ln.wl[j];
                    acc_r[j] += re * ln.wr[j];
                }
            }
            *ol = acc_l.iter().sum();
            *or = acc_r.iter().sum();
        }
    }
}

/// All voices together keep to this many partials (a new note ends the
/// oldest notes, released ones first, to make room).
pub const BUDGET: usize = 16000;

pub struct OidosState {
    sr: f32,
    voices: Vec<Voice>,
    clock: u64,
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    pub pitch_bend: f32,
}

impl OidosState {
    pub fn new(sr: f32) -> Self {
        Self { sr, voices: (0..12).map(|_| Voice::default()).collect(), clock: 0, buf_l: vec![0.0; BLOCK], buf_r: vec![0.0; BLOCK], pitch_bend: 0.0 }
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        let n = get(p, VOICES).round().clamp(1.0, 12.0) as usize;
        self.clock += 1;
        let k = (0..n).find(|&i| !self.voices[i].active).unwrap_or_else(|| (0..n).min_by_key(|&i| (self.voices[i].gate, self.voices[i].age)).unwrap_or(0));
        let tone = note as f32 + get(p, TRANSPOSE).round() + self.pitch_bend;
        self.voices[k].active = false;
        self.voices[k].start(p, note, tone, vel, self.sr);
        self.voices[k].age = self.clock;
        // Over the budget: the oldest notes end, released ones first.
        loop {
            let total: usize = self.voices.iter().filter(|v| v.active).map(|v| v.partials()).sum();
            if total <= BUDGET {
                break;
            }
            let Some(old) = (0..self.voices.len()).filter(|&i| i != k && self.voices[i].active).min_by_key(|&i| (self.voices[i].gate, self.voices[i].age)) else { break };
            self.voices[old].active = false;
        }
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        for v in self.voices.iter_mut() {
            if v.active && v.gate && v.note == note {
                v.gate = false;
            }
        }
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.active = false;
            v.gate = false;
        }
    }

    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        let live: Vec<&Voice> = self.voices.iter().filter(|v| v.active).collect();
        out[0] = live.len() as f32;
        out[1] = live.iter().map(|v| v.partials()).sum::<usize>() as f32;
        if let Some(v) = live.iter().max_by_key(|v| v.age) {
            out[2] = v.note as f32;
            out[3] = v.env.min(v.rel);
            out[4] = v.t as f32 / self.sr;
            out[6] = 1.0;
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        l[..n].fill(0.0);
        r[..n].fill(0.0);
        let sr = self.sr;
        let attack = 1.0 / (get(p, ATTACK).max(1.0) / 1000.0 * sr);
        let release = 1.0 / (get(p, RELEASE).max(1.0) / 1000.0 * sr);
        let gain = 4096f32.powf(get(p, GAIN).clamp(0.0, 1.0) - 0.25);
        let vel_amt = get(p, VELOCITY).clamp(0.0, 1.0);
        let volume = 10f32.powf(get(p, VOLUME) / 20.0) * 0.5;
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let mut done = 0;
            let vel = 1.0 - vel_amt + vel_amt * v.vel;
            while done < n && v.active {
                // Up to the end of this block of the voice's.
                let left = BLOCK - v.t % BLOCK;
                let m = left.min(n - done);
                let (bl, br) = (&mut self.buf_l[..m], &mut self.buf_r[..m]);
                v.run(bl, br);
                let norm = v.norm.max(1.0);
                for k in 0..m {
                    // The original's saturation, and the envelope.
                    let (sl, sr_) = (bl[k], br[k]);
                    let ol = sl * (gain / (norm + (gain - 1.0) * sl * sl)).max(0.0).sqrt();
                    let or = sr_ * (gain / (norm + (gain - 1.0) * sr_ * sr_)).max(0.0).sqrt();
                    v.env = (v.env + attack).min(1.0);
                    if !v.gate {
                        v.rel = (v.rel - release).max(0.0);
                    }
                    let a = v.env.min(v.rel) * vel * volume;
                    l[done + k] += ol * a;
                    r[done + k] += or * a;
                }
                v.t += m;
                done += m;
                if v.t % BLOCK == 0 {
                    v.settle();
                }
                if (!v.gate && v.rel <= 0.0) || v.partials() == 0 {
                    v.active = false;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_random_numbers_are_the_originals() {
        let d = random_data();
        assert_eq!(d.len(), NOISESIZE * NOISESIZE * NOISESIZE);
        assert_eq!(d[0], 0xCAADAA7B);
        assert_eq!(*d.last().unwrap(), 0xB08A4BA7);
    }

    fn run(p: &[f32], note: u8, secs: f32, release_at: f32) -> (Vec<f32>, Vec<f32>) {
        let sr = 48000.0;
        let mut st = OidosState::new(sr);
        st.note_on(p, note, 0.8);
        let (mut ol, mut or) = (Vec::new(), Vec::new());
        let blocks = (secs * sr / 256.0) as usize;
        for b in 0..blocks {
            if b == (release_at * sr / 256.0) as usize {
                st.note_off(p, note);
            }
            let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
            st.render(p, &mut l, &mut r);
            ol.extend(l);
            or.extend(r);
        }
        (ol, or)
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    #[test]
    fn every_preset_sounds_and_stays_in_range() {
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let (l, r) = run(&p, 60, 2.0, 1.0);
            let level = rms(&l[1000..14400]).max(rms(&r[1000..14400]));
            eprintln!("{name:<14} level {level:.3} peak {:.3}", l.iter().chain(&r).fold(0.0f32, |m, x| m.max(x.abs())));
            let peak = l.iter().chain(&r).fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(l.iter().chain(&r).all(|x| x.is_finite()), "{name}");
            assert!(level > 0.005, "{name}: {level}");
            assert!(peak < 1.5, "{name}: {peak}");
        }
    }

    #[test]
    fn a_harmonic_sound_has_the_notes_pitch() {
        // Few modes, all on true overtones: the period is the note's.
        let mut p = preset("Init");
        p[MODES] = 6.0;
        p[FAT] = 1.0;
        p[WIDTH] = 0.0;
        p[HARMONICITY] = 1.0;
        p[SPREAD] = 0.0;
        let (l, _) = run(&p, 57, 1.0, 1.0);
        let x = &l[12000..36000];
        let sr = 48000.0;
        let lag_of = |lag: usize| x.iter().zip(&x[lag..]).map(|(a, b)| a * b).sum::<f32>();
        let want = (sr / 220.0) as usize;
        let best = (want - 20..want + 20).max_by(|a, b| lag_of(*a).total_cmp(&lag_of(*b))).unwrap();
        assert!((best as i32 - want as i32).abs() <= 1, "{best} vs {want}");
    }

    #[test]
    fn decay_and_release_end_the_note_and_long_notes_hold_their_level() {
        let mut p = preset("Init");
        p[RELEASE] = 100.0;
        let (l, _) = run(&p, 60, 8.0, 6.0);
        let sr = 48000;
        let early = rms(&l[sr..2 * sr]);
        let late = rms(&l[5 * sr..6 * sr]);
        assert!((late / early - 1.0).abs() < 0.25, "a held note keeps its level: {early} {late}");
        assert!(rms(&l[7 * sr..]) < 1e-4, "released, it ends");
        // Decay: quieter as it goes.
        p[DECAY_LOW] = 0.8;
        p[DECAY_HIGH] = 0.8;
        let (l, _) = run(&p, 60, 3.0, 3.0);
        assert!(rms(&l[2 * sr..]) < 0.1 * rms(&l[sr / 10..sr / 2]));
    }

    #[test]
    fn the_partials_follow_the_settings() {
        let p = preset("Init");
        let all = partials(&p, 60.0, 48000.0, true);
        assert_eq!(all.len(), 400);
        // The same seed, the same partials; another, others.
        let again = partials(&p, 60.0, 48000.0, true);
        assert!(all.iter().zip(&again).all(|(a, b)| a.tone == b.tone && a.amp == b.amp));
        let mut q = p.clone();
        q[SEED] = 51.0;
        assert!(partials(&q, 60.0, 48000.0, true).iter().zip(&all).any(|(a, b)| a.tone != b.tone));
        // A low filter edge leaves the low partials out.
        q[FILTER_LOW] = 0.6;
        let kept = partials(&q, 60.0, 48000.0, false);
        assert!(kept.iter().all(|x| x.tone > 60.0 + 23.0), "{:?}", kept.iter().map(|x| x.tone).fold(f32::MAX, f32::min));
        // Nothing above what the sample rate holds.
        let mut hi = preset("Hi Hat");
        hi[TRANSPOSE] = 24.0;
        assert!(partials(&hi, 96.0, 48000.0, false).iter().all(|x| 440.0 * 2f32.powf((x.tone - 69.0) / 12.0) < 0.45 * 48000.0));
    }

    #[test]
    fn it_keeps_up_with_a_chord() {
        // Eight voices of the default sound: how long a second of it takes (printed).
        let p = preset("Init");
        let mut st = OidosState::new(48000.0);
        for k in 0..8 {
            st.note_on(&p, 48 + k * 3, 0.8);
        }
        let t = std::time::Instant::now();
        for _ in 0..(48000 / 256) {
            let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
            st.render(&p, &mut l, &mut r);
        }
        eprintln!("oidos: 8 voices x 400 partials, 1 s in {:.0} ms", t.elapsed().as_secs_f32() * 1000.0);
    }
}
