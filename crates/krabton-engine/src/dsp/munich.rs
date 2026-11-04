//! Munich: a monophonic bass and lead synth in the manner of the big American
//! modular and its famous small brother from the early seventies. What makes
//! that sound is modelled rather than imitated:
//!
//! - The ladder is Antti Huovilainen's non-linear model (DAFx 2004): each of
//!   the four transistor stages saturates by itself, not only the feedback,
//!   so a loud bass note thickens all the way down the ladder and the
//!   resonance bites into the low end as it does on the hardware. It runs at
//!   four times the host rate, as the model needs.
//! - Three oscillators that are never reset and drift apart slowly, so no two
//!   notes begin alike. Band-limited (PolyBLEP) and oversampled.
//! - The mixer can be pushed into the filter (Drive), and the output can be
//!   fed back into the mixer (Feedback), the classic cable from the headphone
//!   socket to the external input that fattens the low end.
//! - Separate contours for filter and loudness, with attacks that charge like
//!   a capacitor and snap; the low-note priority of the original keyboard.
//! - A stereo tape echo (Echo): dry on one side, the repeat on the other, in
//!   time with the song. A short repeat after every note is what turns a
//!   plain octave line into a gallop.
//!
//! The algorithm is from the paper; the code is Krabton's own.

use super::{db_to_gain, fast_tanh, Rng};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::PI;

/// Inner samples per host sample.
const OVER: usize = 4;
const FIR_TAPS: usize = 48;
const ECHO_SECONDS: f32 = 2.2;

pub const RANGE1: usize = 0;
pub const WAVE1: usize = 1;
pub const LEVEL1: usize = 2;
pub const RANGE2: usize = 3;
pub const WAVE2: usize = 4;
pub const DETUNE2: usize = 5;
pub const LEVEL2: usize = 6;
pub const RANGE3: usize = 7;
pub const WAVE3: usize = 8;
pub const DETUNE3: usize = 9;
pub const LEVEL3: usize = 10;
pub const NOISE: usize = 11;
pub const DRIVE: usize = 12;
pub const FEEDBACK: usize = 13;
pub const CUTOFF: usize = 14;
pub const EMPHASIS: usize = 15;
pub const CONTOUR: usize = 16;
pub const KEYBOARD: usize = 17;
pub const F_ATTACK: usize = 18;
pub const F_DECAY: usize = 19;
pub const F_SUSTAIN: usize = 20;
pub const F_RELEASE: usize = 21;
pub const ATTACK: usize = 22;
pub const DECAY: usize = 23;
pub const SUSTAIN: usize = 24;
pub const RELEASE: usize = 25;
pub const MOD_SOURCE: usize = 26;
pub const MOD_RATE: usize = 27;
pub const MOD_PITCH: usize = 28;
pub const MOD_FILTER: usize = 29;
pub const GLIDE: usize = 30;
pub const PRIORITY: usize = 31;
pub const TRIGGER: usize = 32;
pub const VELOCITY: usize = 33;
pub const DRIFT: usize = 34;
pub const ECHO_MIX: usize = 35;
pub const ECHO_TIME: usize = 36;
pub const ECHO_FREE: usize = 37;
pub const ECHO_FEEDBACK: usize = 38;
pub const ECHO_SPREAD: usize = 39;
pub const ECHO_TAPE: usize = 40;
pub const VOLUME: usize = 41;

pub const RANGES: &[&str] = &["32'", "16'", "8'", "4'", "2'"];
pub const WAVES: &[&str] = &["Triangle", "Tri-Saw", "Reverse Saw", "Saw", "Square", "Wide Pulse", "Narrow Pulse"];
pub const KEYBOARDS: &[&str] = &["Off", "1/3", "2/3", "Full"];
pub const MOD_SOURCES: &[&str] = &["Triangle LFO", "Square LFO", "Random LFO", "Oscillator 3", "Noise"];
pub const PRIORITIES: &[&str] = &["Low", "Last", "High"];
pub const TRIGGERS: &[&str] = &["Every Note", "Legato"];
pub const ECHO_TIMES: &[&str] = &["1/32", "1/16", "1/16 Dotted", "1/8", "1/8 Dotted", "1/4", "Free"];
/// The echo's times in beats (Free: by its milliseconds).
const ECHO_BEATS: [f32; 6] = [0.125, 0.25, 0.375, 0.5, 0.75, 1.0];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Range", "Osc 1", RANGES, 1),
    ParamSpec::choice("Wave", "Osc 1", WAVES, 3),
    ParamSpec::lin("Level", "Osc 1", 0.0, 1.0, 0.9, Unit::Percent),
    ParamSpec::choice("Range", "Osc 2", RANGES, 1),
    ParamSpec::choice("Wave", "Osc 2", WAVES, 3),
    ParamSpec::lin("Detune", "Osc 2", -100.0, 100.0, 6.0, Unit::Cents),
    ParamSpec::lin("Level", "Osc 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Range", "Osc 3", RANGES, 0),
    ParamSpec::choice("Wave", "Osc 3", WAVES, 3),
    ParamSpec::lin("Detune", "Osc 3", -100.0, 100.0, -5.0, Unit::Cents),
    ParamSpec::lin("Level", "Osc 3", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Noise", "Mixer", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Drive", "Mixer", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Feedback", "Mixer", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Cutoff", "Filter", 20.0, 18000.0, 400.0, Unit::Hz),
    ParamSpec::lin("Emphasis", "Filter", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Contour", "Filter", 0.0, 1.0, 0.45, Unit::Percent),
    ParamSpec::choice("Keyboard", "Filter", KEYBOARDS, 1),
    ParamSpec::log("Attack", "Filter Contour", 0.001, 6.0, 0.001, Unit::Sec),
    ParamSpec::log("Decay", "Filter Contour", 0.004, 8.0, 0.2, Unit::Sec),
    ParamSpec::lin("Sustain", "Filter Contour", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::log("Release", "Filter Contour", 0.004, 8.0, 0.1, Unit::Sec),
    ParamSpec::log("Attack", "Loudness", 0.001, 6.0, 0.001, Unit::Sec),
    ParamSpec::log("Decay", "Loudness", 0.004, 8.0, 0.25, Unit::Sec),
    ParamSpec::lin("Sustain", "Loudness", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::log("Release", "Loudness", 0.004, 8.0, 0.06, Unit::Sec),
    ParamSpec::choice("Source", "Modulation", MOD_SOURCES, 0),
    ParamSpec::log("Rate", "Modulation", 0.05, 40.0, 5.0, Unit::Hz),
    ParamSpec::lin("To Pitch", "Modulation", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("To Filter", "Modulation", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Glide", "Keys", 0.0005, 4.0, 0.0005, Unit::Sec),
    ParamSpec::choice("Priority", "Keys", PRIORITIES, 0),
    ParamSpec::choice("Trigger", "Keys", TRIGGERS, 0),
    ParamSpec::lin("Velocity", "Keys", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Drift", "Keys", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Mix", "Echo", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Time", "Echo", ECHO_TIMES, 1),
    ParamSpec::log("Free", "Echo", 0.02, 1.5, 0.12, Unit::Sec),
    ParamSpec::lin("Repeats", "Echo", 0.0, 0.92, 0.0, Unit::Percent),
    ParamSpec::lin("Spread", "Echo", 0.0, 1.0, 0.85, Unit::Percent),
    ParamSpec::lin("Tape", "Echo", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Volume", "Output", -24.0, 6.0, 0.0, Unit::Db),
];

/// A patch: the defaults with some knobs set.
fn patch(set: &[(usize, f32)]) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    for &(k, v) in set {
        p[k] = v;
    }
    p
}

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Init",
        "Gallop",
        "Gallop Bright",
        "Disco Octaves",
        "Three Saws",
        "Fat Square",
        "Round Sub",
        "Thumb",
        "Funk Pluck",
        "Rubber",
        "Feedback Growl",
        "Pedal Drone",
        "Squelch",
        "Wah Contour",
        "Reedy Pulse",
        "Slow Filter Sweep",
        "Prog Lead",
        "Glide Lead",
        "Whistle",
        "Sample And Hold",
        "Laser",
        "Machine Kick",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    match name {
        // One saw, tight on the filter, a sixteenth repeat on the other side.
        "Gallop" => patch(&[(CUTOFF, 240.0), (EMPHASIS, 0.22), (CONTOUR, 0.42), (KEYBOARD, 1.0), (F_DECAY, 0.13), (F_SUSTAIN, 0.06), (DECAY, 0.17), (SUSTAIN, 0.35), (RELEASE, 0.05), (DRIVE, 0.45), (ECHO_MIX, 0.8), (ECHO_TIME, 1.0), (ECHO_FEEDBACK, 0.08), (ECHO_SPREAD, 0.9), (ECHO_TAPE, 0.35)]),
        "Gallop Bright" => patch(&[(CUTOFF, 380.0), (EMPHASIS, 0.32), (CONTOUR, 0.55), (KEYBOARD, 2.0), (F_DECAY, 0.11), (F_SUSTAIN, 0.05), (DECAY, 0.15), (SUSTAIN, 0.3), (RELEASE, 0.04), (DRIVE, 0.55), (ECHO_MIX, 0.75), (ECHO_TIME, 1.0), (ECHO_FEEDBACK, 0.14), (ECHO_SPREAD, 0.9)]),
        "Disco Octaves" => patch(&[(LEVEL1, 0.9), (RANGE2, 2.0), (WAVE2, 4.0), (LEVEL2, 0.35), (CUTOFF, 300.0), (EMPHASIS, 0.2), (CONTOUR, 0.5), (KEYBOARD, 1.0), (F_DECAY, 0.16), (F_SUSTAIN, 0.1), (DECAY, 0.22), (SUSTAIN, 0.5), (ECHO_MIX, 0.4), (ECHO_TIME, 1.0), (ECHO_SPREAD, 0.7)]),
        "Three Saws" => patch(&[(LEVEL2, 0.9), (DETUNE2, 7.0), (RANGE3, 1.0), (LEVEL3, 0.9), (DETUNE3, -8.0), (CUTOFF, 420.0), (EMPHASIS, 0.18), (CONTOUR, 0.4), (KEYBOARD, 1.0), (DRIVE, 0.6), (F_DECAY, 0.35), (F_SUSTAIN, 0.2), (DECAY, 0.5), (SUSTAIN, 0.8), (RELEASE, 0.1)]),
        "Fat Square" => patch(&[(WAVE1, 4.0), (WAVE2, 3.0), (LEVEL2, 0.7), (RANGE3, 0.0), (WAVE3, 4.0), (LEVEL3, 0.5), (CUTOFF, 330.0), (EMPHASIS, 0.12), (CONTOUR, 0.35), (DRIVE, 0.7), (F_DECAY, 0.25), (F_SUSTAIN, 0.25), (SUSTAIN, 0.85)]),
        "Round Sub" => patch(&[(RANGE1, 0.0), (WAVE1, 0.0), (RANGE2, 1.0), (WAVE2, 0.0), (LEVEL2, 0.5), (CUTOFF, 170.0), (EMPHASIS, 0.0), (CONTOUR, 0.12), (KEYBOARD, 1.0), (DRIVE, 0.3), (SUSTAIN, 0.95), (RELEASE, 0.12), (DRIFT, 0.2)]),
        "Thumb" => patch(&[(WAVE1, 3.0), (WAVE2, 4.0), (LEVEL2, 0.5), (CUTOFF, 190.0), (EMPHASIS, 0.3), (CONTOUR, 0.55), (KEYBOARD, 1.0), (F_DECAY, 0.075), (F_SUSTAIN, 0.0), (DECAY, 0.28), (SUSTAIN, 0.2), (RELEASE, 0.05), (DRIVE, 0.65), (VELOCITY, 0.6)]),
        "Funk Pluck" => patch(&[(WAVE1, 5.0), (RANGE2, 2.0), (WAVE2, 3.0), (LEVEL2, 0.3), (CUTOFF, 260.0), (EMPHASIS, 0.5), (CONTOUR, 0.6), (KEYBOARD, 2.0), (F_DECAY, 0.09), (F_SUSTAIN, 0.0), (DECAY, 0.16), (SUSTAIN, 0.15), (RELEASE, 0.05), (VELOCITY, 0.7)]),
        "Rubber" => patch(&[(WAVE1, 1.0), (CUTOFF, 200.0), (EMPHASIS, 0.42), (CONTOUR, 0.5), (KEYBOARD, 1.0), (F_DECAY, 0.2), (F_SUSTAIN, 0.05), (GLIDE, 0.06), (TRIGGER, 1.0), (SUSTAIN, 0.8), (DRIVE, 0.5)]),
        "Feedback Growl" => patch(&[(LEVEL2, 0.8), (DETUNE2, 9.0), (CUTOFF, 280.0), (EMPHASIS, 0.25), (CONTOUR, 0.4), (KEYBOARD, 1.0), (DRIVE, 0.8), (FEEDBACK, 0.55), (F_DECAY, 0.3), (F_SUSTAIN, 0.3), (SUSTAIN, 0.9)]),
        "Pedal Drone" => patch(&[(RANGE1, 0.0), (LEVEL2, 0.9), (RANGE2, 0.0), (DETUNE2, 8.0), (RANGE3, 1.0), (LEVEL3, 0.5), (DETUNE3, -6.0), (CUTOFF, 130.0), (EMPHASIS, 0.1), (CONTOUR, 0.35), (F_ATTACK, 0.03), (F_DECAY, 1.2), (F_SUSTAIN, 0.2), (ATTACK, 0.01), (SUSTAIN, 1.0), (RELEASE, 0.6), (DRIVE, 0.6)]),
        "Squelch" => patch(&[(CUTOFF, 180.0), (EMPHASIS, 0.72), (CONTOUR, 0.7), (KEYBOARD, 2.0), (F_DECAY, 0.14), (F_SUSTAIN, 0.0), (DECAY, 0.2), (SUSTAIN, 0.3), (DRIVE, 0.55), (GLIDE, 0.03), (TRIGGER, 1.0)]),
        "Wah Contour" => patch(&[(LEVEL2, 0.6), (DETUNE2, 5.0), (CUTOFF, 220.0), (EMPHASIS, 0.6), (CONTOUR, 0.65), (KEYBOARD, 1.0), (F_ATTACK, 0.09), (F_DECAY, 0.3), (F_SUSTAIN, 0.1), (SUSTAIN, 0.85), (RELEASE, 0.1)]),
        "Reedy Pulse" => patch(&[(RANGE1, 2.0), (WAVE1, 6.0), (RANGE2, 1.0), (WAVE2, 5.0), (LEVEL2, 0.5), (CUTOFF, 900.0), (EMPHASIS, 0.25), (CONTOUR, 0.2), (KEYBOARD, 2.0), (SUSTAIN, 0.9), (RELEASE, 0.12), (MOD_RATE, 5.2), (MOD_PITCH, 0.03)]),
        "Slow Filter Sweep" => patch(&[(LEVEL2, 0.9), (DETUNE2, 8.0), (CUTOFF, 120.0), (EMPHASIS, 0.55), (CONTOUR, 0.75), (KEYBOARD, 1.0), (F_ATTACK, 2.0), (F_DECAY, 3.0), (F_SUSTAIN, 0.2), (F_RELEASE, 1.5), (ATTACK, 0.2), (SUSTAIN, 1.0), (RELEASE, 1.0)]),
        "Prog Lead" => patch(&[(RANGE1, 2.0), (RANGE2, 2.0), (LEVEL2, 0.9), (DETUNE2, 6.0), (RANGE3, 1.0), (WAVE3, 4.0), (LEVEL3, 0.6), (CUTOFF, 1400.0), (EMPHASIS, 0.3), (CONTOUR, 0.3), (KEYBOARD, 3.0), (F_DECAY, 0.4), (F_SUSTAIN, 0.5), (SUSTAIN, 0.95), (RELEASE, 0.2), (GLIDE, 0.04), (PRIORITY, 1.0), (TRIGGER, 1.0), (MOD_RATE, 5.5), (MOD_PITCH, 0.03), (DRIVE, 0.6), (ECHO_MIX, 0.2), (ECHO_TIME, 3.0), (ECHO_FEEDBACK, 0.3), (ECHO_SPREAD, 0.5)]),
        "Glide Lead" => patch(&[(RANGE1, 3.0), (WAVE1, 4.0), (RANGE2, 2.0), (LEVEL2, 0.7), (DETUNE2, 4.0), (CUTOFF, 1100.0), (EMPHASIS, 0.35), (CONTOUR, 0.25), (KEYBOARD, 3.0), (SUSTAIN, 1.0), (RELEASE, 0.25), (GLIDE, 0.16), (PRIORITY, 1.0), (TRIGGER, 1.0)]),
        "Whistle" => patch(&[(VOLUME, 6.0), (LEVEL1, 0.0), (CUTOFF, 440.0), (EMPHASIS, 1.0), (CONTOUR, 0.0), (KEYBOARD, 3.0), (SUSTAIN, 1.0), (RELEASE, 0.2), (GLIDE, 0.08), (PRIORITY, 1.0), (TRIGGER, 1.0), (MOD_RATE, 5.5), (MOD_PITCH, 0.02)]),
        "Sample And Hold" => patch(&[(LEVEL2, 0.6), (DETUNE2, 9.0), (CUTOFF, 500.0), (EMPHASIS, 0.7), (CONTOUR, 0.1), (KEYBOARD, 2.0), (MOD_SOURCE, 2.0), (MOD_RATE, 7.0), (MOD_FILTER, 0.55), (SUSTAIN, 1.0), (RELEASE, 0.3), (ECHO_MIX, 0.3), (ECHO_TIME, 2.0), (ECHO_FEEDBACK, 0.4)]),
        "Laser" => patch(&[(VOLUME, -10.0), (RANGE1, 3.0), (WAVE1, 4.0), (CUTOFF, 300.0), (EMPHASIS, 0.78), (CONTOUR, 1.0), (F_DECAY, 0.2), (F_SUSTAIN, 0.0), (DECAY, 0.3), (SUSTAIN, 0.0), (MOD_SOURCE, 3.0), (RANGE3, 4.0), (MOD_PITCH, 0.2)]),
        "Machine Kick" => patch(&[(VOLUME, 6.0), (LEVEL1, 0.0), (NOISE, 0.03), (CUTOFF, 55.0), (EMPHASIS, 1.0), (CONTOUR, 0.45), (KEYBOARD, 0.0), (F_DECAY, 0.05), (F_SUSTAIN, 0.0), (DECAY, 0.22), (SUSTAIN, 0.0), (RELEASE, 0.05), (DRIVE, 0.75), (DRIFT, 0.05)]),
        _ => patch(&[]),
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Release,
}

/// An envelope that charges like a capacitor: quickly at first, stopping at 1.
#[derive(Clone, Copy)]
struct Contour {
    stage: Stage,
    level: f32,
}

impl Contour {
    #[inline]
    fn step(&mut self, ka: f32, kd: f32, sus: f32, kr: f32) -> f32 {
        match self.stage {
            Stage::Attack => {
                self.level += (1.3 - self.level) * ka;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => self.level += (sus - self.level) * kd,
            Stage::Release => {
                self.level -= self.level * kr;
                if self.level < 1e-5 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
            Stage::Idle => {}
        }
        self.level
    }
}

/// The two-sample correction around a step, at phase `t` with increment `dt`.
#[inline]
fn blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

/// One cycle of a wave, band-limited where it jumps.
#[inline]
fn wave(kind: u8, phase: f32, dt: f32) -> f32 {
    let saw = || 2.0 * phase - 1.0 - blep(phase, dt);
    let pulse = |width: f32| {
        let high = if phase < width { 1.0 } else { -1.0 };
        let fall = (phase - width + 1.0).fract();
        high + blep(phase, dt) - blep(fall, dt)
    };
    let triangle = || 1.0 - 4.0 * (phase - 0.5).abs();
    match kind {
        0 => triangle(),
        1 => 0.55 * triangle() + 0.55 * saw(),
        2 => -saw(),
        3 => saw(),
        4 => pulse(0.5),
        5 => pulse(0.3),
        _ => pulse(0.12),
    }
}

/// The ladder: four transistor stages, each saturating, and the feedback with
/// its half-sample delay (Huovilainen 2004).
#[derive(Clone, Copy, Default)]
struct Ladder {
    stage: [f32; 4],
    /// tanh of each stage's output, kept for the next stage and the next sample.
    held: [f32; 3],
    last: f32,
    delayed: f32,
}

/// How hard the stages saturate: one over twice the transistors' thermal voltage.
const THERMAL: f32 = 1.15;

impl Ladder {
    /// `fc` the cutoff as a part of half the inner rate, `res` from 0 to 1.
    #[inline]
    fn tick(&mut self, input: f32, fc: f32, res: f32) -> f32 {
        let fc = fc.clamp(0.0001, 0.9);
        let (fc2, fc3) = (fc * fc, fc * fc * fc);
        // The paper's corrections for tuning and resonance at higher cutoffs.
        let fcr = 1.8730 * fc3 + 0.4955 * fc2 - 0.6490 * fc + 0.9988;
        let acr = -3.9364 * fc2 + 1.8409 * fc + 0.9968;
        let g = 1.0 - (-PI * fc * fcr).exp();
        // The paper's 4 is reached at 80 %: from there up it sings, and louder
        // the further it goes, as the hardware's Emphasis does from about 8 of 10.
        let k = 5.0 * res * acr;
        let tune = g / THERMAL;
        let x = fast_tanh((input - k * self.delayed) * THERMAL);
        let a = self.stage[0] + tune * (x - self.held[0]);
        self.held[0] = fast_tanh(a * THERMAL);
        let b = self.stage[1] + tune * (self.held[0] - self.held[1]);
        self.held[1] = fast_tanh(b * THERMAL);
        let c = self.stage[2] + tune * (self.held[1] - self.held[2]);
        self.held[2] = fast_tanh(c * THERMAL);
        let d = self.stage[3] + tune * (self.held[2] - fast_tanh(self.stage[3] * THERMAL));
        self.stage = [a, b, c, d];
        // The half-sample delay that keeps the feedback in tune.
        self.delayed = 0.5 * (d + self.last);
        self.last = d;
        d
    }
}

pub struct MunichState {
    /// Pitch bend in semitones, set by the engine.
    pub pitch_bend: f32,
    /// The song's tempo, for the echo.
    pub bpm: f32,
    sr: f32,
    held: Vec<u8>,
    gate: bool,
    velocity: f32,
    pitch: f32,
    target: f32,
    /// Free-running: never reset by a note.
    phase: [f32; 3],
    drift: [f32; 3],
    drift_to: [f32; 3],
    ladder: Ladder,
    fenv: Contour,
    aenv: Contour,
    out_prev: f32,
    dc_in: f32,
    dc_out: f32,
    lfo_phase: f32,
    lfo_held: f32,
    mod_now: f32,
    rng: Rng,
    cutoff_hz: f32,
    fir: [f32; FIR_TAPS],
    ring: [f32; FIR_TAPS],
    ring_at: usize,
    echo: Vec<f32>,
    echo_at: usize,
    echo_lp: f32,
    echo_hp: f32,
    echo_time: f32,
    wow: f32,
    /// How much is still ringing in the echo.
    echo_level: f32,
}

impl MunichState {
    pub fn new(sr: f32) -> Self {
        let sr = sr.max(1000.0);
        // A windowed sinc down to the host rate.
        let mut fir = [0.0; FIR_TAPS];
        let mid = (FIR_TAPS as f32 - 1.0) * 0.5;
        let cut = 0.92 / (2.0 * OVER as f32);
        let mut sum = 0.0;
        for (i, c) in fir.iter_mut().enumerate() {
            let x = i as f32 - mid;
            let sinc = if x.abs() < 1e-6 { 2.0 * cut } else { (2.0 * PI * cut * x).sin() / (PI * x) };
            let t = i as f32 / (FIR_TAPS as f32 - 1.0);
            let window = 0.42 - 0.5 * (2.0 * PI * t).cos() + 0.08 * (4.0 * PI * t).cos();
            *c = sinc * window;
            sum += *c;
        }
        for c in &mut fir {
            *c /= sum;
        }
        let silent = Contour { stage: Stage::Idle, level: 0.0 };
        Self {
            pitch_bend: 0.0,
            bpm: 120.0,
            sr,
            held: Vec::with_capacity(16),
            gate: false,
            velocity: 1.0,
            pitch: 36.0,
            target: 36.0,
            phase: [0.13, 0.57, 0.81],
            drift: [0.0; 3],
            drift_to: [0.0; 3],
            ladder: Ladder::default(),
            fenv: silent,
            aenv: silent,
            out_prev: 0.0,
            dc_in: 0.0,
            dc_out: 0.0,
            lfo_phase: 0.0,
            lfo_held: 0.0,
            mod_now: 0.0,
            rng: Rng::new(0x4D55_4E43),
            cutoff_hz: 400.0,
            fir,
            ring: [0.0; FIR_TAPS],
            ring_at: 0,
            echo: vec![0.0; (sr * ECHO_SECONDS) as usize + 8],
            echo_at: 0,
            echo_lp: 0.0,
            echo_hp: 0.0,
            echo_time: 0.0,
            wow: 0.0,
            echo_level: 0.0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.gate || self.aenv.stage != Stage::Idle || self.echo_level > 1e-4
    }

    /// The key that sounds among those held.
    fn chosen(&self, p: &[f32]) -> Option<u8> {
        match p[PRIORITY].round() as i32 {
            0 => self.held.iter().copied().min(),
            2 => self.held.iter().copied().max(),
            _ => self.held.last().copied(),
        }
    }

    fn trigger(&mut self) {
        // From where they are, as a capacitor charges on from what it holds.
        self.fenv.stage = Stage::Attack;
        self.aenv.stage = Stage::Attack;
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        let was_gate = self.gate;
        self.held.retain(|n| *n != note);
        if self.held.len() == self.held.capacity() {
            self.held.remove(0);
        }
        self.held.push(note);
        self.velocity = vel.clamp(0.0, 1.0);
        let Some(key) = self.chosen(p) else { return };
        self.target = key as f32;
        if !was_gate && p[GLIDE] <= 0.001 {
            self.pitch = self.target;
        }
        self.gate = true;
        if !was_gate || p[TRIGGER].round() as i32 == 0 {
            self.trigger();
        }
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.held.retain(|n| *n != note);
        match self.chosen(p) {
            Some(key) => self.target = key as f32,
            None => {
                self.gate = false;
                self.release();
            }
        }
    }

    fn release(&mut self) {
        if self.fenv.stage != Stage::Idle {
            self.fenv.stage = Stage::Release;
        }
        if self.aenv.stage != Stage::Idle {
            self.aenv.stage = Stage::Release;
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        self.gate = false;
        self.release();
    }

    /// For the screen: the filter contour, the loudness, the cutoff (0..1 of
    /// its range), whether a key is down, and the modulation.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.fenv.level;
        out[1] = self.aenv.level;
        out[2] = ((self.cutoff_hz / 20.0).max(1.0).ln() / (18000.0f32 / 20.0).ln()).clamp(0.0, 1.0);
        out[3] = self.gate as u8 as f32;
        out[4] = self.mod_now;
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let sr = self.sr;
        let inner = sr * OVER as f32;
        let k = |t: f32, c: f32| 1.0 - (-c / (t.max(0.0003) * inner)).exp();
        let (fa, fd, fr) = (k(p[F_ATTACK], 1.466), k(p[F_DECAY], 4.6), k(p[F_RELEASE], 4.6));
        let (aa, ad, ar) = (k(p[ATTACK], 1.466), k(p[DECAY], 4.6), k(p[RELEASE], 4.6));
        let (fs, ass) = (p[F_SUSTAIN].clamp(0.0, 1.0), p[SUSTAIN].clamp(0.0, 1.0));
        let glide_k = if p[GLIDE] <= 0.001 { 1.0 } else { k(p[GLIDE], 2.3) };
        let range = |i: usize| (p[i].round().clamp(0.0, 4.0) - 2.0) * 12.0;
        let semis = [range(RANGE1), range(RANGE2) + p[DETUNE2] / 100.0, range(RANGE3) + p[DETUNE3] / 100.0];
        let kinds = [p[WAVE1].round() as u8, p[WAVE2].round() as u8, p[WAVE3].round() as u8];
        let levels = [p[LEVEL1], p[LEVEL2], p[LEVEL3]].map(|v| v.clamp(0.0, 1.0));
        let noise_level = p[NOISE].clamp(0.0, 1.0);
        // The mixer into the ladder: from clean to overdriven, the level held up afterwards.
        let drive = p[DRIVE].clamp(0.0, 1.0);
        let push = 0.55 * 8f32.powf(drive);
        let makeup = 1.0 / (0.7 + 0.55 * push.sqrt());
        let feedback = p[FEEDBACK].clamp(0.0, 1.0) * 2.4;
        let cutoff = p[CUTOFF].clamp(15.0, 20000.0);
        let res = p[EMPHASIS].clamp(0.0, 1.0);
        let contour = p[CONTOUR].clamp(0.0, 1.0) * 8.0;
        let key_amount = p[KEYBOARD].round().clamp(0.0, 3.0) / 3.0;
        let vel_amount = p[VELOCITY].clamp(0.0, 1.0);
        let vel_gain = 1.0 - 0.85 * vel_amount * (1.0 - self.velocity);
        let vel_oct = -3.0 * vel_amount * (1.0 - self.velocity);
        let drift_amount = p[DRIFT].clamp(0.0, 1.0);
        let source = p[MOD_SOURCE].round() as i32;
        let rate = p[MOD_RATE].clamp(0.01, 60.0);
        let depth = |x: f32| x.clamp(0.0, 1.0).powi(2);
        let mod_pitch = depth(p[MOD_PITCH]) * 24.0;
        let mod_filter = depth(p[MOD_FILTER]) * 6.0;
        let out_gain = db_to_gain(p[VOLUME]) * vel_gain;
        // The echo.
        let echo_mix = p[ECHO_MIX].clamp(0.0, 1.0);
        let echo_repeats = p[ECHO_FEEDBACK].clamp(0.0, 0.92);
        let spread = p[ECHO_SPREAD].clamp(0.0, 1.0);
        let tape = p[ECHO_TAPE].clamp(0.0, 1.0);
        let beats = p[ECHO_TIME].round() as usize;
        let want = match ECHO_BEATS.get(beats) {
            Some(b) => b * 60.0 / self.bpm.max(20.0),
            None => p[ECHO_FREE].clamp(0.01, ECHO_SECONDS - 0.1),
        }
        .min(ECHO_SECONDS - 0.1);
        if self.echo_time <= 0.0 {
            self.echo_time = want;
        }
        let lp_k = 1.0 - (-2.0 * PI * (6500.0 - 4500.0 * tape) / sr).exp();
        let hp_k = 1.0 - (-2.0 * PI * 70.0 / sr).exp();
        let dc_k = 1.0 - (-2.0 * PI * 12.0 / inner).exp();
        let (pan_dry_l, pan_dry_r) = (1.0, 1.0 - spread);
        let (pan_echo_l, pan_echo_r) = (1.0 - spread, 1.0);
        for i in 0..n {
            for _ in 0..OVER {
                // Modulation.
                let before = self.lfo_phase;
                self.lfo_phase = (self.lfo_phase + rate / inner).fract();
                if self.lfo_phase < before {
                    self.lfo_held = self.rng.bipolar();
                }
                let white = self.rng.bipolar();
                let modulation = match source {
                    1 => {
                        if self.lfo_phase < 0.5 {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    2 => self.lfo_held,
                    3 => wave(kinds[2], self.phase[2], 0.01),
                    4 => white,
                    _ => 1.0 - 4.0 * (self.lfo_phase - 0.5).abs(),
                };
                self.mod_now = modulation;
                let fenv = self.fenv.step(fa, fd, fs, fr);
                let aenv = self.aenv.step(aa, ad, ass, ar);
                // Pitch.
                self.pitch += (self.target - self.pitch) * glide_k;
                let base = self.pitch + self.pitch_bend + modulation * mod_pitch;
                let mut mix = white * noise_level * 0.5;
                for o in 0..3 {
                    if self.rng.unipolar() < 0.00004 {
                        self.drift_to[o] = self.rng.bipolar() * 0.09 * drift_amount;
                    }
                    self.drift[o] += (self.drift_to[o] - self.drift[o]) * 0.00002;
                    let s = base + semis[o] + self.drift[o];
                    let f = 440.0 * 2f32.powf((s - 69.0) / 12.0);
                    let dt = (f / inner).min(0.45);
                    if levels[o] > 0.0 {
                        mix += levels[o] * wave(kinds[o], self.phase[o], dt);
                    }
                    self.phase[o] = (self.phase[o] + dt).fract();
                }
                // Into the ladder, the cable from the output round to the mixer with it.
                // (With the transistors' own hiss, which starts the ladder singing by itself.)
                let input = fast_tanh(mix * push + self.out_prev * feedback) + white * 0.0002;
                // The cutoff.
                let oct = contour * fenv + key_amount * (self.pitch - 48.0) / 12.0 + modulation * mod_filter + vel_oct;
                let fc = (cutoff * 2f32.powf(oct)).clamp(10.0, 0.42 * inner);
                self.cutoff_hz = fc;
                let y = self.ladder.tick(input, fc / (inner * 0.5), res);
                self.out_prev = y;
                let out = y * aenv;
                // No DC from the saturation.
                self.dc_in += (out - self.dc_in) * dc_k;
                let cleaned = out - self.dc_in;
                self.dc_out = cleaned;
                self.ring[self.ring_at] = cleaned;
                self.ring_at = (self.ring_at + 1) % FIR_TAPS;
            }
            // Down to the host rate.
            let mut dry = 0.0;
            let mut at = self.ring_at;
            for c in &self.fir {
                dry += c * self.ring[at];
                at = (at + 1) % FIR_TAPS;
            }
            dry *= 1.5 * makeup * out_gain;

            // The tape echo: slides to a new time, wavers a little.
            let len = self.echo.len();
            self.echo_time += (want - self.echo_time) * 0.0008;
            self.wow += (2.0 * PI * 0.7 / sr).min(1.0);
            if self.wow > 2.0 * PI {
                self.wow -= 2.0 * PI;
            }
            let wobble = 1.0 + tape * 0.0022 * self.wow.sin();
            let delay = (self.echo_time * wobble * sr).clamp(2.0, len as f32 - 3.0);
            let read = self.echo_at as f32 + len as f32 - delay;
            let whole = read.floor();
            let frac = read - whole;
            let a = self.echo[whole as usize % len];
            let b = self.echo[(whole as usize + 1) % len];
            let wet = a + (b - a) * frac;
            self.echo_lp += (wet - self.echo_lp) * lp_k;
            self.echo_hp += (self.echo_lp - self.echo_hp) * hp_k;
            let toned = self.echo_lp - self.echo_hp;
            let saturated = if tape > 0.0 { toned + tape * (fast_tanh(toned * 1.6) / 1.6 - toned) } else { toned };
            self.echo[self.echo_at] = dry + saturated * echo_repeats;
            self.echo_at = (self.echo_at + 1) % len;
            self.echo_level = (self.echo_level * 0.9995).max(saturated.abs());
            let echoed = saturated * echo_mix;
            // Without an echo the note stands in the middle.
            let (dl, dr) = if echo_mix > 0.0 { (pan_dry_l, pan_dry_r) } else { (1.0, 1.0) };
            let left = dry * dl + echoed * pan_echo_l;
            let right = dry * dr + echoed * pan_echo_r;
            l[i] += left;
            r[i] += right;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn play(p: &[f32], note: u8, seconds: f32) -> (Vec<f32>, Vec<f32>) {
        let mut s = MunichState::new(SR);
        s.note_on(p, note, 1.0);
        let n = (seconds * SR) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (a, b) in l.chunks_mut(128).zip(r.chunks_mut(128)) {
            s.render(p, a, b);
        }
        (l, r)
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0, |m, v| m.max(v.abs()))
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// How strong a frequency is in a signal.
    fn amount(x: &[f32], f: f32) -> f32 {
        let (mut re, mut im) = (0.0f32, 0.0f32);
        for (i, v) in x.iter().enumerate() {
            let w = 2.0 * PI * f * i as f32 / SR;
            re += v * w.cos();
            im += v * w.sin();
        }
        2.0 * (re * re + im * im).sqrt() / x.len().max(1) as f32
    }

    /// The ladder's own response at a frequency.
    fn through(fc: f32, res: f32, f: f32, level: f32) -> f32 {
        let inner = SR * OVER as f32;
        let mut ladder = Ladder::default();
        let n = (inner * 0.5) as usize;
        let mut out = Vec::with_capacity(n / 2);
        for i in 0..n {
            let x = level * (2.0 * PI * f * i as f32 / inner).sin();
            let y = ladder.tick(x, fc / (inner * 0.5), res);
            if i >= n / 2 {
                out.push(y);
            }
        }
        (out.iter().map(|v| v * v).sum::<f32>() / out.len() as f32).sqrt() * 2f32.sqrt() / level
    }

    #[test]
    fn the_ladder_falls_at_24_db_an_octave() {
        let at = |f| through(500.0, 0.0, f, 0.05);
        assert!(at(60.0) > 0.85, "passes the low end: {}", at(60.0));
        let (one, two) = (at(2000.0), at(4000.0));
        let slope = 20.0 * (one / two).log10();
        assert!((slope - 24.0).abs() < 3.5, "{slope} dB an octave");
    }

    #[test]
    fn the_cutoff_is_where_it_says_it_is() {
        // At the cutoff four poles give -12 dB, whatever the frequency.
        for fc in [100.0, 500.0, 2000.0, 8000.0] {
            let db = 20.0 * through(fc, 0.0, fc, 0.03).log10();
            assert!((-16.0..-9.0).contains(&db), "{fc}: {db} dB");
        }
    }

    #[test]
    fn emphasis_rings_at_the_cutoff_and_takes_from_the_low_end() {
        let plain = through(800.0, 0.0, 800.0, 0.03);
        let ringing = through(800.0, 0.7, 800.0, 0.03);
        assert!(ringing > plain * 3.0, "{plain} {ringing}");
        let low_plain = through(800.0, 0.0, 60.0, 0.03);
        let low_ringing = through(800.0, 0.7, 60.0, 0.03);
        assert!(low_ringing < low_plain * 0.75, "the hardware loses bass: {low_plain} {low_ringing}");
    }

    #[test]
    fn every_stage_saturates_so_loud_notes_thicken() {
        // A quiet sine stays pure; a loud one gains harmonics.
        let inner = SR * OVER as f32;
        let third = |level: f32| {
            let mut ladder = Ladder::default();
            let n = (inner * 0.5) as usize;
            let mut out = Vec::with_capacity(n / 2);
            for i in 0..n {
                let y = ladder.tick(level * (2.0 * PI * 110.0 * i as f32 / inner).sin(), 2000.0 / (inner * 0.5), 0.0);
                if i >= n / 2 {
                    out.push(y);
                }
            }
            let at = |f: f32| {
                let (mut re, mut im) = (0.0f32, 0.0f32);
                for (k, v) in out.iter().enumerate() {
                    let w = 2.0 * PI * f * k as f32 / inner;
                    re += v * w.cos();
                    im += v * w.sin();
                }
                (re * re + im * im).sqrt()
            };
            at(330.0) / at(110.0)
        };
        assert!(third(0.02) < 0.002, "{}", third(0.02));
        assert!(third(2.0) > 0.03, "{}", third(2.0));
    }

    #[test]
    fn full_emphasis_sings_by_itself_near_the_cutoff_and_stays_bounded() {
        let inner = SR * OVER as f32;
        let mut ladder = Ladder::default();
        let n = inner as usize;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let kick = if i < 8 { 0.2 } else { 0.0 };
            out.push(ladder.tick(kick, 440.0 / (inner * 0.5), 1.0));
        }
        let tail = &out[n / 2..];
        let top = tail.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(top > 0.12 && top < 4.0, "{top}");
        let crossings = tail.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f32;
        let hz = crossings / (tail.len() as f32 / inner);
        assert!((hz / 440.0 - 1.0).abs() < 0.05, "sings at {hz} Hz");
    }

    #[test]
    fn it_plays_in_tune_on_all_three_oscillators() {
        let p = patch(&[(CUTOFF, 18000.0), (CONTOUR, 0.0), (EMPHASIS, 0.0), (SUSTAIN, 1.0), (DRIFT, 0.0), (DRIVE, 0.0)]);
        let (l, _) = play(&p, 45, 0.5);
        let f = 110.0;
        // Range 16': one octave below the 8' the note's name means.
        let tail = &l[12000..];
        assert!(amount(tail, f / 2.0) > amount(tail, f * 0.75) * 10.0, "{} {}", amount(tail, f / 2.0), amount(tail, f * 0.75));
    }

    #[test]
    fn low_note_priority_like_the_original_keyboard() {
        let p = preset("Init");
        let mut s = MunichState::new(SR);
        s.note_on(&p, 48, 1.0);
        s.note_on(&p, 60, 1.0);
        assert_eq!(s.target, 48.0);
        s.note_off(&p, 48);
        assert_eq!(s.target, 60.0);
        let mut q = p.clone();
        q[PRIORITY] = 1.0;
        let mut s = MunichState::new(SR);
        s.note_on(&q, 48, 1.0);
        s.note_on(&q, 60, 1.0);
        assert_eq!(s.target, 60.0);
    }

    #[test]
    fn notes_begin_differently_because_the_oscillators_run_free() {
        let p = patch(&[(LEVEL2, 0.8), (DRIFT, 0.0)]);
        let mut s = MunichState::new(SR);
        let (mut a, mut ar) = (vec![0.0; 2048], vec![0.0; 2048]);
        s.note_on(&p, 36, 1.0);
        s.render(&p, &mut a, &mut ar);
        s.note_off(&p, 36);
        let (mut gap, mut gr) = (vec![0.0; 1777], vec![0.0; 1777]);
        s.render(&p, &mut gap, &mut gr);
        let (mut b, mut br) = (vec![0.0; 2048], vec![0.0; 2048]);
        s.note_on(&p, 36, 1.0);
        s.render(&p, &mut b, &mut br);
        let diff: f32 = a.iter().zip(&b).map(|(x, y)| (x - y).abs()).sum();
        assert!(diff > 1.0, "{diff}");
    }

    #[test]
    fn the_echo_repeats_a_sixteenth_later_on_the_other_side() {
        let mut p = preset("Gallop");
        p[ECHO_FEEDBACK] = 0.0;
        let mut s = MunichState::new(SR);
        s.bpm = 120.0;
        s.note_on(&p, 36, 1.0);
        let n = (0.6 * SR) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        s.render(&p, &mut l[..1024], &mut r[..1024]);
        s.note_off(&p, 36);
        // Its tail keeps the instrument alive after the key is up.
        assert!(s.is_active());
        for (a, b) in l[1024..].chunks_mut(128).zip(r[1024..].chunks_mut(128)) {
            s.render(&p, a, b);
        }
        // 16th at 120: 125 ms. The right is quiet until the repeat comes.
        let window = |x: &[f32], from: f32, to: f32| rms(&x[(from * SR) as usize..(to * SR) as usize]);
        assert!(window(&l, 0.0, 0.02) > window(&r, 0.0, 0.02) * 4.0);
        assert!(window(&r, 0.125, 0.15) > window(&r, 0.08, 0.11) * 2.0, "{} {}", window(&r, 0.125, 0.15), window(&r, 0.08, 0.11));
        // Then it ends.
        let (mut a, mut b) = (vec![0.0; 128], vec![0.0; 128]);
        for _ in 0..3000 {
            a.fill(0.0);
            b.fill(0.0);
            s.render(&p, &mut a, &mut b);
        }
        assert!(!s.is_active(), "{}", s.echo_level);
    }

    #[test]
    fn every_preset_sounds_and_stays_within_bounds() {
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let mut s = MunichState::new(SR);
            s.bpm = 127.0;
            let n = 256;
            let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
            let mut top = 0.0f32;
            let mut energy = 0.0f32;
            // A line of notes, low and high, for a second and a half.
            for k in 0..12 {
                let note = [36u8, 48, 43, 46][k % 4];
                s.note_on(&p, note, 0.9);
                for _ in 0..24 {
                    l.fill(0.0);
                    r.fill(0.0);
                    s.render(&p, &mut l, &mut r);
                    top = top.max(peak(&l)).max(peak(&r));
                    energy += rms(&l) + rms(&r);
                    assert!(l.iter().chain(r.iter()).all(|v| v.is_finite()), "{name}");
                }
                s.note_off(&p, note);
                for _ in 0..4 {
                    l.fill(0.0);
                    r.fill(0.0);
                    s.render(&p, &mut l, &mut r);
                }
            }
            assert!(energy > 0.3, "{name} is silent: {energy}");
            assert!(top < 3.0, "{name} peaks at {top}");
        }
    }

    #[test]
    fn a_soft_note_is_clearly_softer_and_darker_than_a_hard_one() {
        let p = preset("Init");
        let render = |vel: f32| {
            let mut s = MunichState::new(SR);
            s.note_on(&p, 48, vel);
            let n = (0.3 * SR) as usize;
            let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
            for (a, b) in l.chunks_mut(128).zip(r.chunks_mut(128)) {
                s.render(&p, a, b);
            }
            rms(&l)
        };
        let db = 20.0 * (render(1.0) / render(0.25)).log10();
        assert!(db > 4.5, "only {db} dB between hard and soft");
    }

    #[test]
    fn it_does_not_allocate_more_while_it_plays() {
        let p = preset("Gallop");
        let mut s = MunichState::new(SR);
        let cap = s.held.capacity();
        for k in 0..64u8 {
            s.note_on(&p, 20 + k, 1.0);
        }
        assert_eq!(s.held.capacity(), cap);
    }
}
