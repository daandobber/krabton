//! Hybrid: two oscillators that can each be something else, through two
//! filters that can each be something else. The oscillators: phase
//! distortion (see `warp`), a saw that leans into a triangle, a square of
//! any width, a triangle that folds, a sine that feeds back into itself,
//! vector phaseshaping (a cosine read fast and then slow, the bend point
//! moved up: formants), VOSIM (trains of sine pulses that die away, a
//! voice's formant), Walsh functions (the square waves of a digital
//! sequency) and a plucked string. The second can be synced to the first,
//! ringed by it or phase-modulated by it; noise beside them. The filters:
//! a ladder that growls, a state variable (low, band, high, notch), a comb,
//! a formant filter that speaks vowels, a phaser; one after the other or
//! side by side, a drive before them. Two envelopes (the amp, and one for
//! the filters, the shapes and the pitch) and an LFO. Krabton's own, after
//! the list of what pd-hybrid-synth does; none of its code is used.

use crate::params::{ParamSpec, Unit};
use std::f32::consts::{PI, TAU};

pub const ENGINE1: usize = 0;
pub const SHAPE1: usize = 1;
pub const TUNE1: usize = 2;
pub const FINE1: usize = 3;
pub const LEVEL1: usize = 4;
pub const ENGINE2: usize = 5;
pub const SHAPE2: usize = 6;
pub const TUNE2: usize = 7;
pub const FINE2: usize = 8;
pub const LEVEL2: usize = 9;
pub const MOD2: usize = 10;
pub const MOD_AMOUNT: usize = 11;
pub const NOISE: usize = 12;
pub const F1_TYPE: usize = 13;
pub const F1_CUTOFF: usize = 14;
pub const F1_RESO: usize = 15;
pub const F1_ENV: usize = 16;
pub const F1_KEYS: usize = 17;
pub const F2_TYPE: usize = 18;
pub const F2_CUTOFF: usize = 19;
pub const F2_RESO: usize = 20;
pub const F2_ENV: usize = 21;
pub const ROUTING: usize = 22;
pub const DRIVE: usize = 23;
pub const ATTACK: usize = 24;
pub const DECAY: usize = 25;
pub const SUSTAIN: usize = 26;
pub const RELEASE: usize = 27;
pub const M_ATTACK: usize = 28;
pub const M_DECAY: usize = 29;
pub const M_SUSTAIN: usize = 30;
pub const M_RELEASE: usize = 31;
pub const M_SHAPE: usize = 32;
pub const M_PITCH: usize = 33;
pub const LFO_WAVE: usize = 34;
pub const LFO_RATE: usize = 35;
pub const LFO_PITCH: usize = 36;
pub const LFO_CUTOFF: usize = 37;
pub const LFO_SHAPE: usize = 38;
pub const VOICES: usize = 39;
pub const GLIDE: usize = 40;
pub const VELOCITY: usize = 41;
pub const VOLUME: usize = 42;

pub const ENGINES: &[&str] = &["Phase", "Saw", "Square", "Triangle", "Sine", "Vector", "Vosim", "Walsh", "Pluck"];
pub const MODS: &[&str] = &["Off", "Sync", "Ring", "Phase Mod"];
pub const FILTERS: &[&str] = &["Off", "Ladder", "Low", "Band", "High", "Notch", "Comb", "Formant", "Phaser"];
pub const ROUTINGS: &[&str] = &["Series", "Parallel"];
pub const LFO_WAVES: &[&str] = &["Sine", "Triangle", "Saw", "Square", "Random"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Engine 1", "Osc 1", ENGINES, 1),
    ParamSpec::lin("Shape 1", "Osc 1", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Tune 1", "Osc 1", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Fine 1", "Osc 1", -100.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::lin("Level 1", "Osc 1", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::choice("Engine 2", "Osc 2", ENGINES, 2),
    ParamSpec::lin("Shape 2", "Osc 2", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::int("Tune 2", "Osc 2", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Fine 2", "Osc 2", -100.0, 100.0, 7.0, Unit::Cents),
    ParamSpec::lin("Level 2", "Osc 2", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::choice("Mod 2", "Osc 2", MODS, 0),
    ParamSpec::lin("Mod Amount", "Osc 2", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Noise", "Osc 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Filter 1", "Filter 1", FILTERS, 1),
    ParamSpec::log("Cutoff 1", "Filter 1", 20.0, 20000.0, 2400.0, Unit::Hz),
    ParamSpec::lin("Reso 1", "Filter 1", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Env 1", "Filter 1", -1.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Keys 1", "Filter 1", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::choice("Filter 2", "Filter 2", FILTERS, 0),
    ParamSpec::log("Cutoff 2", "Filter 2", 20.0, 20000.0, 1200.0, Unit::Hz),
    ParamSpec::lin("Reso 2", "Filter 2", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Env 2", "Filter 2", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Routing", "Filter 2", ROUTINGS, 0),
    ParamSpec::lin("Drive", "Filter 1", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::log("Attack", "Amp", 1.0, 5000.0, 3.0, Unit::Ms),
    ParamSpec::log("Decay", "Amp", 1.0, 5000.0, 400.0, Unit::Ms),
    ParamSpec::lin("Sustain", "Amp", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::log("Release", "Amp", 1.0, 8000.0, 300.0, Unit::Ms),
    ParamSpec::log("Mod Attack", "Mod Env", 1.0, 5000.0, 2.0, Unit::Ms),
    ParamSpec::log("Mod Decay", "Mod Env", 1.0, 5000.0, 500.0, Unit::Ms),
    ParamSpec::lin("Mod Sustain", "Mod Env", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::log("Mod Release", "Mod Env", 1.0, 8000.0, 400.0, Unit::Ms),
    ParamSpec::lin("Env Shape", "Mod Env", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Env Pitch", "Mod Env", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::choice("LFO Wave", "LFO", LFO_WAVES, 0),
    ParamSpec::log("LFO Rate", "LFO", 0.05, 20.0, 4.0, Unit::Hz),
    ParamSpec::lin("LFO Pitch", "LFO", 0.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::lin("LFO Cutoff", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("LFO Shape", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Voices", "Play", 1.0, 8.0, 8.0, Unit::None),
    ParamSpec::log("Glide", "Play", 1.0, 2000.0, 1.0, Unit::Ms),
    ParamSpec::lin("Velocity", "Play", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Volume", "Play", 0.0, 1.0, 0.7, Unit::Percent),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Init", "Fat Bass", "Vector Lead", "Vosim Voice", "Walsh Bells", "Pluck Comb", "Phase Pad", "Sync Scream", "Ring Metal", "Fold Bass", "Formant Talk", "Phaser Strings", "PM Keys", "Acid Ladder", "Noise Sweep", "Glass Pluck"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Fat Bass" => &[(ENGINE2, 1.0), (TUNE2, -12.0), (FINE2, 5.0), (LEVEL2, 0.7), (F1_CUTOFF, 380.0), (F1_RESO, 0.4), (F1_ENV, 0.55), (DRIVE, 0.45), (M_DECAY, 260.0), (M_SUSTAIN, 0.0), (DECAY, 600.0), (SUSTAIN, 0.8), (RELEASE, 120.0), (VOICES, 1.0), (GLIDE, 30.0)],
        "Vector Lead" => &[(ENGINE1, 5.0), (SHAPE1, 0.55), (ENGINE2, 5.0), (SHAPE2, 0.6), (FINE2, 12.0), (LEVEL2, 0.6), (F1_CUTOFF, 4200.0), (F1_RESO, 0.2), (LFO_SHAPE, 0.3), (LFO_RATE, 0.4), (LFO_PITCH, 8.0), (VOICES, 1.0), (GLIDE, 70.0), (M_SHAPE, 0.3)],
        "Vosim Voice" => &[(ENGINE1, 6.0), (SHAPE1, 0.35), (ENGINE2, 6.0), (SHAPE2, 0.5), (FINE2, 9.0), (LEVEL2, 0.5), (F1_TYPE, 7.0), (F1_CUTOFF, 900.0), (F1_RESO, 0.5), (F1_ENV, 0.0), (LFO_CUTOFF, 0.3), (LFO_RATE, 0.25), (ATTACK, 120.0), (RELEASE, 800.0), (LFO_PITCH, 10.0)],
        "Walsh Bells" => &[(ENGINE1, 7.0), (SHAPE1, 0.5), (ENGINE2, 4.0), (TUNE2, 19.0), (LEVEL2, 0.35), (F1_TYPE, 2.0), (F1_CUTOFF, 5000.0), (DECAY, 1800.0), (SUSTAIN, 0.0), (RELEASE, 1500.0), (M_SHAPE, 0.5), (M_DECAY, 900.0)],
        "Pluck Comb" => &[(ENGINE1, 8.0), (SHAPE1, 0.6), (LEVEL2, 0.0), (F1_TYPE, 6.0), (F1_CUTOFF, 900.0), (F1_RESO, 0.5), (F1_ENV, 0.2), (F1_KEYS, 1.0), (DECAY, 2500.0), (SUSTAIN, 0.0), (RELEASE, 900.0)],
        "Phase Pad" => &[(ENGINE1, 0.0), (SHAPE1, 0.5), (ENGINE2, 0.0), (SHAPE2, 0.6), (FINE2, 10.0), (LEVEL2, 0.7), (F1_CUTOFF, 3000.0), (F1_ENV, 0.1), (ATTACK, 900.0), (SUSTAIN, 0.9), (RELEASE, 2200.0), (LFO_SHAPE, 0.4), (LFO_RATE, 0.3), (M_SHAPE, 0.3), (M_ATTACK, 1500.0), (M_SUSTAIN, 0.6)],
        "Sync Scream" => &[(ENGINE2, 1.0), (TUNE2, 7.0), (MOD2, 1.0), (LEVEL1, 0.3), (LEVEL2, 0.8), (M_PITCH, 14.0), (M_DECAY, 500.0), (M_SUSTAIN, 0.0), (F1_CUTOFF, 5000.0), (DRIVE, 0.4), (VOICES, 1.0), (GLIDE, 40.0)],
        "Ring Metal" => &[(ENGINE1, 4.0), (ENGINE2, 2.0), (TUNE2, 13.0), (FINE2, 30.0), (MOD2, 2.0), (LEVEL1, 0.2), (LEVEL2, 0.9), (F1_TYPE, 3.0), (F1_CUTOFF, 2500.0), (F1_RESO, 0.4), (DECAY, 1200.0), (SUSTAIN, 0.0), (RELEASE, 900.0)],
        "Fold Bass" => &[(ENGINE1, 3.0), (SHAPE1, 0.2), (LEVEL2, 0.0), (M_SHAPE, 0.7), (M_DECAY, 300.0), (M_SUSTAIN, 0.1), (F1_CUTOFF, 1800.0), (F1_ENV, 0.2), (TUNE1, -12.0), (VOICES, 1.0), (GLIDE, 20.0)],
        "Formant Talk" => &[(ENGINE1, 1.0), (ENGINE2, 2.0), (SHAPE2, 0.4), (F1_TYPE, 7.0), (F1_CUTOFF, 600.0), (F1_RESO, 0.6), (F1_ENV, 0.6), (M_DECAY, 700.0), (M_SUSTAIN, 0.3), (LFO_CUTOFF, 0.25), (LFO_RATE, 1.5)],
        "Phaser Strings" => &[(ENGINE1, 1.0), (SHAPE1, 0.15), (ENGINE2, 1.0), (FINE2, 11.0), (LEVEL2, 0.7), (F1_TYPE, 2.0), (F1_CUTOFF, 5000.0), (F2_TYPE, 8.0), (F2_CUTOFF, 900.0), (F2_RESO, 0.6), (LFO_CUTOFF, 0.0), (ATTACK, 450.0), (SUSTAIN, 0.9), (RELEASE, 1600.0)],
        "PM Keys" => &[(ENGINE1, 4.0), (ENGINE2, 4.0), (TUNE2, 12.0), (MOD2, 3.0), (MOD_AMOUNT, 0.6), (LEVEL1, 0.2), (LEVEL2, 0.8), (F1_TYPE, 0.0), (DECAY, 1400.0), (SUSTAIN, 0.2), (RELEASE, 500.0), (M_SHAPE, 0.0), (VELOCITY, 0.9)],
        "Acid Ladder" => &[(ENGINE1, 1.0), (LEVEL2, 0.0), (F1_CUTOFF, 300.0), (F1_RESO, 0.8), (F1_ENV, 0.7), (DRIVE, 0.5), (M_DECAY, 180.0), (M_SUSTAIN, 0.0), (DECAY, 300.0), (SUSTAIN, 0.6), (VOICES, 1.0), (GLIDE, 60.0)],
        "Noise Sweep" => &[(LEVEL1, 0.0), (LEVEL2, 0.0), (NOISE, 0.9), (F1_TYPE, 3.0), (F1_CUTOFF, 300.0), (F1_RESO, 0.8), (F1_ENV, 0.9), (M_ATTACK, 2000.0), (M_DECAY, 2000.0), (ATTACK, 400.0), (RELEASE, 1500.0)],
        "Glass Pluck" => &[(ENGINE1, 8.0), (SHAPE1, 0.85), (ENGINE2, 8.0), (SHAPE2, 0.9), (TUNE2, 12.0), (LEVEL2, 0.4), (F1_TYPE, 8.0), (F1_CUTOFF, 1500.0), (F1_RESO, 0.5), (DECAY, 3000.0), (SUSTAIN, 0.0), (RELEASE, 1500.0)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

/// An oscillator's wave at phase `ph` (0 to 1), `dt` its step a sample.
/// (The plucked string is not a wave: it shows as a saw that dies away.)
pub fn osc(engine: usize, shape: f32, ph: f32, dt: f32) -> f32 {
    let s = shape.clamp(0.0, 1.0);
    match engine {
        0 => super::warp::wave(0, ph, s, dt),
        // A saw that leans into a triangle.
        1 => {
            let saw = 2.0 * ph - 1.0 - poly_blep(ph, dt);
            let peak = 1.0 - 0.5 * s;
            let tri = if ph < peak { 2.0 * ph / peak - 1.0 } else { 1.0 - 2.0 * (ph - peak) / (1.0 - peak) };
            if s < 0.02 {
                saw
            } else {
                saw + (tri - saw) * (s * 3.0).min(1.0)
            }
        }
        // A square of any width.
        2 => {
            let w = 0.5 + 0.45 * s;
            let mut y = if ph < w { 1.0 } else { -1.0 };
            y += poly_blep(ph, dt);
            y -= poly_blep((ph - w).rem_euclid(1.0), dt);
            y
        }
        // A triangle that folds back on itself.
        3 => {
            let tri = 1.0 - 4.0 * (ph - 0.5).abs();
            let g = 1.0 + 4.0 * s;
            (0.5 * PI * tri * g).sin()
        }
        // A sine that feeds back into itself (a little FM of its own).
        4 => {
            let mut y = (TAU * ph).sin();
            for _ in 0..3 {
                y = (TAU * ph + 1.4 * s * y).sin();
            }
            y
        }
        // Vector phaseshaping: up to the bend point fast, then the rest.
        5 => {
            let v = 0.5 + 2.5 * s;
            let d = 0.5;
            let w = if ph < d { v * ph / d } else { v + (1.0 - v) * (ph - d) / (1.0 - d) };
            -(TAU * w).cos()
        }
        // VOSIM: sine pulses at the formant, each smaller, in every cycle.
        6 => {
            let r = 1.0 + 7.0 * s;
            let x = ph * r;
            let k = x.floor();
            let pulse = (PI * x.fract()).sin().powi(2) * 0.72f32.powf(k);
            // (the mean of the train, taken away)
            let mean = (1.0 - 0.72f32.powf(r.floor().max(1.0))) / (1.0 - 0.72) * 0.5 / r;
            2.0 * (pulse - mean)
        }
        // Walsh functions, sequency 1 to 15, crossfaded.
        7 => {
            let pos = s * 14.0;
            let k = pos.floor() as u32;
            let f = pos - pos.floor();
            // (in sequency order: 1 the slowest square, up from there)
            let walsh = |n: u32| {
                let idx = ((ph * 16.0) as u32).min(15);
                let rev = n.reverse_bits() >> 28;
                if (rev & idx).count_ones() % 2 == 0 {
                    1.0
                } else {
                    -1.0
                }
            };
            walsh(k + 1) * (1.0 - f) + walsh((k + 2).min(15)) * f
        }
        _ => (2.0 * ph - 1.0) * (1.0 - ph),
    }
}

#[derive(Clone, Copy, Default)]
struct Adsr {
    level: f32,
    stage: u8,
}

impl Adsr {
    fn gate(&mut self, on: bool) {
        if on {
            self.stage = 1;
        } else if self.stage != 0 {
            self.stage = 4;
        }
    }

    /// Times in ms.
    fn tick(&mut self, a: f32, d: f32, s: f32, r: f32, dt: f32) -> f32 {
        let k = |ms: f32| 1.0 - (-dt * 1000.0 / ms.max(0.5) * 4.0).exp();
        match self.stage {
            1 => {
                self.level += dt * 1000.0 / a.max(0.5);
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = 2;
                }
            }
            2 => {
                self.level += (s - self.level) * k(d);
                if (self.level - s).abs() < 1e-4 {
                    self.stage = 3;
                }
            }
            3 => self.level = s,
            4 => {
                self.level -= self.level * k(r);
                if self.level < 1e-4 {
                    self.level = 0.0;
                    self.stage = 0;
                }
            }
            _ => self.level = 0.0,
        }
        self.level
    }
}

/// One filter, any of the kinds.
#[derive(Clone)]
struct Filter {
    s: [f32; 4],
    comb: Vec<f32>,
    pos: usize,
    form: [[f32; 2]; 3],
    ap: [f32; 4],
    fb: f32,
}

impl Filter {
    fn new(sr: f32) -> Self {
        Self { s: [0.0; 4], comb: vec![0.0; (sr / 20.0) as usize + 4], pos: 0, form: [[0.0; 2]; 3], ap: [0.0; 4], fb: 0.0 }
    }

    fn svf(s: &mut [f32; 4], x: f32, g: f32, k: f32) -> (f32, f32, f32) {
        let a1 = 1.0 / (1.0 + g * (g + k));
        let v3 = x - s[1];
        let v1 = a1 * s[0] + g * a1 * v3;
        let v2 = s[1] + g * v1;
        s[0] = 2.0 * v1 - s[0];
        s[1] = 2.0 * v2 - s[1];
        (v2, v1, x - k * v1 - v2)
    }

    fn run(&mut self, kind: usize, x: f32, fc: f32, reso: f32, sr: f32) -> f32 {
        let fc = fc.clamp(20.0, sr * 0.45);
        let g = (PI * fc / sr).tan();
        let y = match kind {
            // A ladder: four poles, its feedback through a soft clip.
            1 => {
                let k = 4.2 * reso;
                let gg = g / (1.0 + g);
                let mut u = (x - k * self.fb).tanh();
                for st in self.s.iter_mut() {
                    let v = (u - *st) * gg;
                    let y = v + *st;
                    *st = y + v;
                    u = y;
                }
                self.fb = u;
                u * (1.0 + 0.6 * reso)
            }
            2..=5 => {
                let k = 2.0 - 1.9 * reso;
                let (lp, bp, hp) = Self::svf(&mut self.s, x, g, k);
                match kind {
                    2 => lp,
                    3 => bp * k * (1.0 + reso),
                    4 => hp,
                    _ => lp + hp,
                }
            }
            // A comb at the cutoff's pitch.
            6 => {
                let n = self.comb.len();
                let d = (sr / fc).clamp(2.0, (n - 2) as f32);
                let at = self.pos as f32 + n as f32 - d;
                let i0 = at.floor() as usize % n;
                let f = at - at.floor();
                let delayed = self.comb[i0] * (1.0 - f) + self.comb[(i0 + 1) % n] * f;
                let y = x + 0.97 * reso * delayed;
                self.comb[self.pos] = y.clamp(-4.0, 4.0);
                self.pos = (self.pos + 1) % n;
                y * (1.0 - 0.9 * reso)
            }
            // Vowels: three formants, the cutoff walking from u to i.
            7 => {
                const VOWELS: [[f32; 3]; 5] = [[300.0, 870.0, 2240.0], [400.0, 800.0, 2600.0], [730.0, 1090.0, 2440.0], [530.0, 1840.0, 2480.0], [270.0, 2290.0, 3010.0]];
                let pos = ((fc / 200.0).ln() / (4000.0f32 / 200.0).ln()).clamp(0.0, 1.0) * 4.0;
                let i = (pos.floor() as usize).min(3);
                let f = pos - i as f32;
                let mut y = 0.0;
                for b in 0..3 {
                    let ff = VOWELS[i][b] + (VOWELS[i + 1][b] - VOWELS[i][b]) * f;
                    let g = (PI * ff.min(sr * 0.45) / sr).tan();
                    let k = 1.0 / (3.0 + 12.0 * reso);
                    let mut st = [self.form[b][0], self.form[b][1], 0.0, 0.0];
                    let (_, bp, _) = Self::svf(&mut st, x, g, k);
                    self.form[b] = [st[0], st[1]];
                    y += bp * [1.0, 0.7, 0.4][b] * k;
                }
                y * 1.5
            }
            // A phaser: four allpasses at the cutoff, fed back.
            8 => {
                let a = (g - 1.0) / (g + 1.0);
                let mut u = x + self.fb * 0.8 * reso;
                for st in self.ap.iter_mut() {
                    let y = a * u + *st;
                    *st = u - a * y;
                    u = y;
                }
                self.fb = u;
                0.5 * (x + u)
            }
            _ => x,
        };
        if y.is_finite() {
            y
        } else {
            *self = Self::new(sr);
            0.0
        }
    }
}

#[derive(Clone)]
struct Voice {
    note: u8,
    vel: f32,
    gate: bool,
    age: u64,
    pitch: f32,
    target: f32,
    ph: [f32; 2],
    pm_last: f32,
    amp: Adsr,
    menv: Adsr,
    f: [Filter; 2],
    string: [Vec<f32>; 2],
    spos: [usize; 2],
    slast: [f32; 2],
    rng: u32,
}

impl Voice {
    fn new(sr: f32) -> Self {
        let n = (sr / 20.0) as usize + 4;
        Self {
            note: 0,
            vel: 0.0,
            gate: false,
            age: 0,
            pitch: 60.0,
            target: 60.0,
            ph: [0.0; 2],
            pm_last: 0.0,
            amp: Adsr::default(),
            menv: Adsr::default(),
            f: [Filter::new(sr), Filter::new(sr)],
            string: [vec![0.0; n], vec![0.0; n]],
            spos: [0; 2],
            slast: [0.0; 2],
            rng: 0x2545_F491,
        }
    }

    fn active(&self) -> bool {
        self.amp.stage != 0
    }

    fn noise(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / (1u32 << 23) as f32 - 1.0
    }

    /// A string struck: its loop filled with a burst, brighter or darker.
    fn pluck(&mut self, o: usize, bright: f32) {
        let mut last = 0.0;
        for i in 0..self.string[o].len() {
            let n = self.noise();
            last += (n - last) * (0.25 + 0.75 * bright);
            self.string[o][i] = last;
        }
        self.slast[o] = 0.0;
    }
}

pub struct HybridState {
    sr: f32,
    voices: Vec<Voice>,
    clock: u64,
    held: Vec<(u8, f32)>,
    lfo_ph: f32,
    lfo_hold: f32,
    lfo_rng: u32,
    lfo_now: f32,
    pub pitch_bend: f32,
}

fn get(p: &[f32], i: usize, or: f32) -> f32 {
    p.get(i).copied().unwrap_or(or)
}

fn choice(p: &[f32], i: usize, n: usize) -> usize {
    (get(p, i, 0.0).round().max(0.0) as usize).min(n - 1)
}

impl HybridState {
    pub fn new(sr: f32) -> Self {
        Self { sr, voices: (0..8).map(|_| Voice::new(sr)).collect(), clock: 0, held: Vec::new(), lfo_ph: 0.0, lfo_hold: 0.0, lfo_rng: 0x1234_5679, lfo_now: 0.0, pitch_bend: 0.0 }
    }

    fn voices(p: &[f32]) -> usize {
        get(p, VOICES, 8.0).round().clamp(1.0, 8.0) as usize
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        self.held.retain(|k| k.0 != note);
        self.held.push((note, vel));
        self.clock += 1;
        let n = Self::voices(p);
        let mono = n == 1;
        let k = if mono {
            0
        } else {
            (0..n).find(|&i| !self.voices[i].active()).unwrap_or_else(|| (0..n).min_by_key(|&i| self.voices[i].age).unwrap_or(0))
        };
        let glide = get(p, GLIDE, 1.0) >= 2.0;
        let v = &mut self.voices[k];
        let legato = mono && v.gate && self.held.len() > 1;
        v.target = note as f32;
        if !legato {
            if !v.active() || !mono || !glide {
                v.pitch = note as f32;
            }
            if !v.active() {
                v.ph = [0.0; 2];
            }
            v.amp.gate(true);
            v.menv.gate(true);
            v.rng ^= (self.clock as u32).wrapping_mul(2_654_435_761) | 1;
            for o in 0..2 {
                if choice(p, if o == 0 { ENGINE1 } else { ENGINE2 }, ENGINES.len()) == 8 {
                    let bright = get(p, if o == 0 { SHAPE1 } else { SHAPE2 }, 0.5) * (0.6 + 0.4 * vel);
                    v.pluck(o, bright);
                }
            }
        }
        v.note = note;
        v.vel = vel;
        v.gate = true;
        v.age = self.clock;
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.held.retain(|k| k.0 != note);
        if Self::voices(p) == 1 {
            let v = &mut self.voices[0];
            if v.note == note {
                if let Some(&(n, vel)) = self.held.last() {
                    v.note = n;
                    v.target = n as f32;
                    v.vel = vel;
                    return;
                }
                v.gate = false;
                v.amp.gate(false);
                v.menv.gate(false);
            }
            return;
        }
        for v in self.voices.iter_mut() {
            if v.gate && v.note == note {
                v.gate = false;
                v.amp.gate(false);
                v.menv.gate(false);
            }
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in self.voices.iter_mut() {
            v.gate = false;
            v.amp = Adsr::default();
            v.menv = Adsr::default();
        }
    }

    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        let Some(v) = self.voices.iter().filter(|v| v.active()).max_by_key(|v| v.age) else {
            out[3] = self.lfo_now;
            return;
        };
        out[0] = v.note as f32;
        out[1] = v.amp.level;
        out[2] = v.menv.level;
        out[3] = self.lfo_now;
        out[6] = 1.0;
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let dt = 1.0 / sr;
        let eng = [choice(p, ENGINE1, ENGINES.len()), choice(p, ENGINE2, ENGINES.len())];
        let shape = [get(p, SHAPE1, 0.0), get(p, SHAPE2, 0.3)];
        let tune = [get(p, TUNE1, 0.0).round() + get(p, FINE1, 0.0) / 100.0, get(p, TUNE2, 0.0).round() + get(p, FINE2, 7.0) / 100.0];
        let level = [get(p, LEVEL1, 0.8), get(p, LEVEL2, 0.5)];
        let modk = choice(p, MOD2, MODS.len());
        let mod_amt = get(p, MOD_AMOUNT, 0.5);
        let noise = get(p, NOISE, 0.0);
        let ft = [choice(p, F1_TYPE, FILTERS.len()), choice(p, F2_TYPE, FILTERS.len())];
        let fc = [get(p, F1_CUTOFF, 2400.0), get(p, F2_CUTOFF, 1200.0)];
        let fr = [get(p, F1_RESO, 0.3), get(p, F2_RESO, 0.3)];
        let fe = [get(p, F1_ENV, 0.35), get(p, F2_ENV, 0.0)];
        let keys = get(p, F1_KEYS, 0.5);
        let parallel = choice(p, ROUTING, ROUTINGS.len()) == 1;
        let drive = get(p, DRIVE, 0.1);
        let (a, d, s, rl) = (get(p, ATTACK, 3.0), get(p, DECAY, 400.0), get(p, SUSTAIN, 0.7), get(p, RELEASE, 300.0));
        let (ma, md, ms, mr) = (get(p, M_ATTACK, 2.0), get(p, M_DECAY, 500.0), get(p, M_SUSTAIN, 0.2), get(p, M_RELEASE, 400.0));
        let (m_shape, m_pitch) = (get(p, M_SHAPE, 0.0), get(p, M_PITCH, 0.0));
        let lfo_wave = choice(p, LFO_WAVE, LFO_WAVES.len());
        let lfo_rate = get(p, LFO_RATE, 4.0);
        let (lfo_pitch, lfo_cut, lfo_shape) = (get(p, LFO_PITCH, 0.0) / 100.0, get(p, LFO_CUTOFF, 0.0), get(p, LFO_SHAPE, 0.0));
        let glide = get(p, GLIDE, 1.0).max(1.0) / 1000.0;
        let glide_k = 1.0 - (-dt / glide).exp();
        let vel_amt = get(p, VELOCITY, 0.6);
        let volume = get(p, VOLUME, 0.7);
        let bend = self.pitch_bend;
        let drive_g = 1.0 + 8.0 * drive;
        let drive_n = drive_g.tanh();
        for i in 0..n {
            // The LFO, one for all voices.
            let before = self.lfo_ph;
            self.lfo_ph = (self.lfo_ph + lfo_rate * dt).fract();
            if self.lfo_ph < before {
                self.lfo_rng ^= self.lfo_rng << 13;
                self.lfo_rng ^= self.lfo_rng >> 17;
                self.lfo_rng ^= self.lfo_rng << 5;
                self.lfo_hold = (self.lfo_rng >> 8) as f32 / (1u32 << 23) as f32 - 1.0;
            }
            let x = self.lfo_ph;
            let lfo = match lfo_wave {
                0 => (TAU * x).sin(),
                1 => 1.0 - 4.0 * (x - 0.5).abs(),
                2 => 2.0 * x - 1.0,
                3 => {
                    if x < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                _ => self.lfo_hold,
            };
            self.lfo_now = lfo;
            let mut out = 0.0;
            for v in self.voices.iter_mut() {
                if !v.active() {
                    continue;
                }
                v.pitch += (v.target - v.pitch) * glide_k;
                let amp = v.amp.tick(a, d, s, rl, dt);
                let menv = v.menv.tick(ma, md, ms, mr, dt);
                let vel = 1.0 - vel_amt + vel_amt * v.vel;
                let base = v.pitch + bend + lfo * lfo_pitch + menv * m_pitch;
                let mut y = [0.0f32; 2];
                let mut wrapped = false;
                let mut wrap_at = 0.0;
                for o in 0..2 {
                    let f = 440.0 * 2f32.powf((base + tune[o] - 69.0) / 12.0);
                    let dph = (f / sr).min(0.45);
                    let sh = (shape[o] + menv * m_shape + lfo * lfo_shape * 0.5).clamp(0.0, 1.0);
                    if eng[o] == 8 {
                        // The string: its loop as long as the pitch, a little darker each time round.
                        let len = (sr / f).clamp(2.0, (v.string[o].len() - 2) as f32);
                        let nb = v.string[o].len();
                        let pos = v.spos[o];
                        let at = pos as f32 + nb as f32 - len;
                        let i0 = at.floor() as usize % nb;
                        let fr_ = at - at.floor();
                        let delayed = v.string[o][i0] * (1.0 - fr_) + v.string[o][(i0 + 1) % nb] * fr_;
                        let damp = 0.25 + 0.7 * sh;
                        let next = delayed * damp + v.slast[o] * (1.0 - damp);
                        v.slast[o] = next;
                        v.string[o][pos] = next * 0.998;
                        v.spos[o] = (pos + 1) % nb;
                        y[o] = delayed * 2.0;
                        continue;
                    }
                    let mut ph = v.ph[o];
                    if o == 1 && modk == 3 {
                        ph = (ph + 0.5 * mod_amt * v.pm_last).rem_euclid(1.0);
                    }
                    y[o] = osc(eng[o], sh, ph, dph);
                    v.ph[o] += dph;
                    if v.ph[o] >= 1.0 {
                        v.ph[o] -= 1.0;
                        if o == 0 {
                            wrapped = true;
                            wrap_at = v.ph[0] / dph;
                        }
                    }
                    if o == 0 {
                        v.pm_last = y[0];
                    }
                    if o == 1 && modk == 1 && wrapped {
                        // Hard sync: the second starts again with the first.
                        let f2 = 440.0 * 2f32.powf((base + tune[1] - 69.0) / 12.0);
                        v.ph[1] = (wrap_at * f2 / sr).fract();
                    }
                }
                if modk == 2 {
                    y[1] = y[1] * (1.0 - mod_amt) + y[1] * y[0] * mod_amt;
                }
                let nz = if noise > 0.0 { v.noise() * noise } else { 0.0 };
                let mut x = y[0] * level[0] + y[1] * level[1] + nz;
                if drive > 0.0 {
                    x = (x * drive_g).tanh() / drive_n;
                }
                // The filters: their cutoffs moved by the keys, the envelope and the LFO.
                let key_oct = (v.note as f32 - 60.0) / 12.0;
                let cut = |c: usize| {
                    let oct = fe[c] * menv * 6.0 + lfo * lfo_cut * 2.0 + if c == 0 { keys * key_oct } else { 0.0 };
                    fc[c] * 2f32.powf(oct)
                };
                let yv = if parallel && ft[1] != 0 {
                    let a1 = v.f[0].run(ft[0], x, cut(0), fr[0], sr);
                    let a2 = v.f[1].run(ft[1], x, cut(1), fr[1], sr);
                    0.5 * (a1 + a2)
                } else {
                    let a1 = v.f[0].run(ft[0], x, cut(0), fr[0], sr);
                    v.f[1].run(ft[1], a1, cut(1), fr[1], sr)
                };
                out += yv * amp * vel;
            }
            let y = out * 0.4 * volume;
            l[i] = y;
            r[i] = y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn run(st: &mut HybridState, p: &[f32], secs: f32) -> Vec<f32> {
        let n = (secs * SR) as usize;
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            let mut l = vec![0.0; 256];
            let mut r = vec![0.0; 256];
            st.render(p, &mut l, &mut r);
            out.extend(l);
        }
        out
    }

    fn rms(v: &[f32]) -> f32 {
        (v.iter().map(|x| x * x).sum::<f32>() / v.len().max(1) as f32).sqrt()
    }

    #[test]
    fn every_engine_plays_its_pitch() {
        for (e, name) in ENGINES.iter().enumerate() {
            let mut p = preset("Init");
            p[ENGINE1] = e as f32;
            p[SHAPE1] = if e == 8 { 0.5 } else { 0.0 };
            p[LEVEL2] = 0.0;
            p[F1_TYPE] = 0.0;
            p[SUSTAIN] = 1.0;
            let mut st = HybridState::new(SR);
            st.note_on(&p, 57, 1.0);
            let out = run(&mut st, &p, 0.5);
            assert!(rms(&out[4800..]) > 0.01, "{name}");
            // The fundamental, by autocorrelation (220 Hz: 218 samples).
            let x = &out[9600..19200];
            let lag_of = |lag: usize| x.iter().zip(&x[lag..]).map(|(a, b)| a * b).sum::<f32>();
            let best = (150..300).max_by(|a, b| lag_of(*a).total_cmp(&lag_of(*b))).unwrap();
            assert!((best as f32 - 218.2).abs() < 3.0, "{name}: {best}");
        }
    }

    #[test]
    fn every_filter_passes_and_stays_sane() {
        for (k, name) in FILTERS.iter().enumerate() {
            let mut p = preset("Init");
            p[F1_TYPE] = k as f32;
            p[F1_RESO] = 1.0;
            let mut st = HybridState::new(SR);
            st.note_on(&p, 48, 1.0);
            let out = run(&mut st, &p, 0.5);
            assert!(out.iter().all(|x| x.is_finite() && x.abs() < 1.5), "{name}");
            assert!(rms(&out[4800..]) > 0.005, "{name}: {}", rms(&out[4800..]));
        }
    }

    #[test]
    fn sync_follows_the_first_oscillator() {
        let mut p = preset("Sync Scream");
        p[M_PITCH] = 0.0;
        p[LEVEL1] = 0.0;
        p[F1_TYPE] = 0.0;
        p[DRIVE] = 0.0;
        p[SUSTAIN] = 1.0;
        let mut st = HybridState::new(SR);
        st.note_on(&p, 57, 1.0);
        let out = run(&mut st, &p, 0.5);
        // Synced, the second repeats at the first's period (218 samples at 220 Hz).
        let x = &out[9600..19200];
        let lag_of = |lag: usize| x.iter().zip(&x[lag..]).map(|(a, b)| a * b).sum::<f32>() / x.iter().map(|a| a * a).sum::<f32>();
        let c = (216..=220).map(lag_of).fold(f32::MIN, f32::max);
        assert!(c > 0.9, "{c}");
    }

    #[test]
    fn every_preset_sounds_and_lets_go() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = HybridState::new(SR);
            for n in [48, 55, 60] {
                st.note_on(&p, n, 0.9);
            }
            let out = run(&mut st, &p, 0.5);
            assert!(out.iter().all(|x| x.is_finite() && x.abs() < 1.5), "{name}");
            assert!(rms(&out) > 0.005, "{name}: {}", rms(&out));
            for n in [48, 55, 60] {
                st.note_off(&p, n);
            }
            run(&mut st, &p, 12.0);
            assert!(st.voices.iter().all(|v| !v.active()), "{name} rings on");
        }
    }
}

