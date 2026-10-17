//! Analog: a virtual analog polysynth in the spirit of the classic analog
//! polys and monos.
//!
//! Two oscillators (saw, pulse, triangle, sine) with hard sync, cross
//! modulation, pulse-width modulation and a sweep of oscillator 2 by the
//! filter envelope; a sub oscillator and noise; a nonlinear 24 dB ladder
//! filter or a 12 dB state-variable filter; RC-style envelopes that
//! overshoot into the attack like capacitor circuits; one LFO with delay;
//! unison; a VINTAGE control for slowly drifting oscillators and per-voice
//! spread of filters and envelopes; and a bucket-brigade-style stereo
//! chorus with the three classic modes.
//!
//! Voices run at twice the host rate with band-limited oscillators
//! (PolyBLEP steps and BLAMP corners, also at sync resets) and are
//! decimated with a half-band filter.

use super::{db_to_gain, fast_tanh, midi_to_freq, pan_gains, Rng};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::PI;

/// Voice slots (polyphony times unison).
pub const MAX_VOICES: usize = 16;
/// Oversampled samples per control step (envelope-driven pitch and cutoff).
const CTRL: usize = 16;
/// Host samples rendered per inner chunk.
const CHUNK: usize = 256;
const FIR_TAPS: usize = 47;

pub const WAVE1: usize = 0;
pub const OCTAVE1: usize = 1;
pub const PW1: usize = 2;
pub const CROSS_MOD: usize = 3;
pub const WAVE2: usize = 4;
pub const OCTAVE2: usize = 5;
pub const SEMI2: usize = 6;
pub const DETUNE2: usize = 7;
pub const PW2: usize = 8;
pub const SYNC: usize = 9;
pub const ENV_PITCH2: usize = 10;
pub const MIX1: usize = 11;
pub const MIX2: usize = 12;
pub const SUB: usize = 13;
pub const NOISE: usize = 14;
pub const FILTER: usize = 15;
pub const CUTOFF: usize = 16;
pub const RESONANCE: usize = 17;
pub const DRIVE: usize = 18;
pub const ENV_AMOUNT: usize = 19;
pub const KEY_TRACK: usize = 20;
pub const FLT_VELOCITY: usize = 21;
pub const F_ATTACK: usize = 22;
pub const F_DECAY: usize = 23;
pub const F_SUSTAIN: usize = 24;
pub const F_RELEASE: usize = 25;
pub const ATTACK: usize = 26;
pub const DECAY: usize = 27;
pub const SUSTAIN: usize = 28;
pub const RELEASE: usize = 29;
pub const VELOCITY: usize = 30;
pub const LFO_WAVE: usize = 31;
pub const LFO_RATE: usize = 32;
pub const LFO_DELAY: usize = 33;
pub const LFO_PITCH: usize = 34;
pub const LFO_FILTER: usize = 35;
pub const LFO_PWM: usize = 36;
pub const VOICES: usize = 37;
pub const UNISON: usize = 38;
pub const UNI_DETUNE: usize = 39;
pub const GLIDE: usize = 40;
pub const VINTAGE: usize = 41;
pub const SPREAD: usize = 42;
pub const CHORUS: usize = 43;
pub const VOLUME: usize = 44;

pub const WAVES: &[&str] = &["Saw", "Pulse", "Triangle", "Sine"];
pub const OCTAVES: &[&str] = &["32'", "16'", "8'", "4'", "2'"];
pub const FILTERS: &[&str] = &["Ladder 24", "SVF LP 12", "SVF BP", "SVF HP"];
pub const LFO_WAVES: &[&str] = &["Triangle", "Sine", "Saw", "Square", "S&H"];
pub const CHORUS_MODES: &[&str] = &["Off", "I", "II", "I+II"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Wave 1", "Osc 1", WAVES, 0),
    ParamSpec::choice("Octave 1", "Osc 1", OCTAVES, 2),
    ParamSpec::lin("PW 1", "Osc 1", 0.05, 0.95, 0.5, Unit::Percent),
    ParamSpec::lin("Cross Mod", "Osc 1", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Wave 2", "Osc 2", WAVES, 0),
    ParamSpec::choice("Octave 2", "Osc 2", OCTAVES, 2),
    ParamSpec::int("Semi 2", "Osc 2", -12.0, 12.0, 0.0, Unit::Semi),
    ParamSpec::lin("Detune 2", "Osc 2", -50.0, 50.0, 7.0, Unit::Cents),
    ParamSpec::lin("PW 2", "Osc 2", 0.05, 0.95, 0.5, Unit::Percent),
    ParamSpec::toggle("Sync", "Osc 2", false),
    ParamSpec::lin("Env Pitch 2", "Osc 2", -24.0, 48.0, 0.0, Unit::Semi),
    ParamSpec::lin("Osc 1", "Mixer", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Osc 2", "Mixer", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Sub", "Mixer", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Noise", "Mixer", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Filter", "Filter", FILTERS, 0),
    ParamSpec::log("Cutoff", "Filter", 20.0, 20000.0, 1500.0, Unit::Hz),
    ParamSpec::lin("Resonance", "Filter", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Drive", "Filter", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Env Amount", "Filter", -1.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Key Track", "Filter", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Flt Velocity", "Filter", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::log("Flt Attack", "Filter Env", 0.001, 10.0, 0.002, Unit::Sec),
    ParamSpec::log("Flt Decay", "Filter Env", 0.005, 10.0, 0.5, Unit::Sec),
    ParamSpec::lin("Flt Sustain", "Filter Env", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::log("Flt Release", "Filter Env", 0.005, 10.0, 0.4, Unit::Sec),
    ParamSpec::log("Attack", "Amp Env", 0.001, 10.0, 0.002, Unit::Sec),
    ParamSpec::log("Decay", "Amp Env", 0.005, 10.0, 0.6, Unit::Sec),
    ParamSpec::lin("Sustain", "Amp Env", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::log("Release", "Amp Env", 0.005, 10.0, 0.3, Unit::Sec),
    ParamSpec::lin("Velocity", "Amp Env", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::choice("LFO Wave", "LFO", LFO_WAVES, 0),
    ParamSpec::log("LFO Rate", "LFO", 0.05, 30.0, 5.0, Unit::Hz),
    ParamSpec::lin("LFO Delay", "LFO", 0.0, 5.0, 0.0, Unit::Sec),
    ParamSpec::lin("LFO Pitch", "LFO", 0.0, 12.0, 0.0, Unit::Semi),
    ParamSpec::lin("LFO Filter", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("LFO PWM", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Voices", "Voice", 1.0, 8.0, 8.0, Unit::None),
    ParamSpec::int("Unison", "Voice", 1.0, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Uni Detune", "Voice", 0.0, 50.0, 15.0, Unit::Cents),
    ParamSpec::log("Glide", "Voice", 0.001, 2.0, 0.001, Unit::Sec),
    ParamSpec::lin("Vintage", "Voice", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Spread", "Voice", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::choice("Chorus", "Output", CHORUS_MODES, 0),
    ParamSpec::lin("Volume", "Output", -24.0, 6.0, 0.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Init",
        "Ladder Bass",
        "Sub Bass",
        "Squelch Bass",
        "Wobble Bass",
        "Chorus Pad",
        "Chorus Strings",
        "Warm Pad",
        "Sweep Pad",
        "Poly Brass",
        "Rave Stab",
        "Unison Saws",
        "Poly Keys",
        "Pluck",
        "Funk Clav",
        "Organ",
        "Bell",
        "Sync Lead",
        "Mono Lead",
        "Square Lead",
        "Laser Zap",
        "Analog Kick",
        "Noise Hit",
        "Wonky Tape",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    let (saw, pulse, tri, sine) = (0.0, 1.0, 2.0, 3.0);
    let (o32, o16, o4) = (0.0, 1.0, 3.0);
    let mono = (VOICES, 1.0);
    let on = 1.0;
    match name {
        "Ladder Bass" => set(&[mono, (OCTAVE1, o16), (OCTAVE2, o16), (MIX2, 0.8), (DETUNE2, 6.0), (SUB, 0.4), (CUTOFF, 260.0), (RESONANCE, 0.35), (DRIVE, 0.45), (ENV_AMOUNT, 0.45), (F_DECAY, 0.35), (F_SUSTAIN, 0.15), (SUSTAIN, 1.0), (RELEASE, 0.08), (GLIDE, 0.03), (SPREAD, 0.0), (VOLUME, 5.0)]),
        "Sub Bass" => set(&[mono, (WAVE1, tri), (OCTAVE1, o16), (SUB, 0.6), (CUTOFF, 500.0), (RESONANCE, 0.0), (ENV_AMOUNT, 0.1), (SUSTAIN, 1.0), (RELEASE, 0.06), (SPREAD, 0.0), (VINTAGE, 0.15), (VOLUME, 6.0)]),
        "Squelch Bass" => set(&[mono, (OCTAVE1, o16), (CUTOFF, 320.0), (RESONANCE, 0.78), (DRIVE, 0.5), (ENV_AMOUNT, 0.55), (F_DECAY, 0.22), (F_SUSTAIN, 0.0), (FLT_VELOCITY, 0.5), (SUSTAIN, 0.9), (RELEASE, 0.05), (GLIDE, 0.06), (SPREAD, 0.0), (VOLUME, 5.0)]),
        "Wobble Bass" => set(&[mono, (OCTAVE1, o16), (WAVE2, pulse), (OCTAVE2, o16), (MIX2, 0.7), (SUB, 0.5), (CUTOFF, 280.0), (RESONANCE, 0.5), (DRIVE, 0.6), (ENV_AMOUNT, 0.1), (LFO_WAVE, 1.0), (LFO_RATE, 3.0), (LFO_FILTER, 0.6), (SUSTAIN, 1.0), (RELEASE, 0.1), (SPREAD, 0.0), (VOLUME, 5.0)]),
        "Chorus Pad" => set(&[(WAVE1, pulse), (LFO_RATE, 0.6), (LFO_PWM, 0.6), (SUB, 0.35), (CUTOFF, 2200.0), (RESONANCE, 0.15), (ENV_AMOUNT, 0.1), (ATTACK, 0.6), (DECAY, 2.0), (SUSTAIN, 0.9), (RELEASE, 1.5), (CHORUS, 1.0), (VOLUME, -4.0)]),
        "Chorus Strings" => set(&[(CUTOFF, 3800.0), (RESONANCE, 0.1), (ENV_AMOUNT, 0.0), (ATTACK, 0.35), (SUSTAIN, 1.0), (RELEASE, 1.2), (CHORUS, 2.0), (VELOCITY, 0.3)]),
        "Warm Pad" => set(&[(WAVE1, tri), (MIX2, 0.8), (DETUNE2, 9.0), (CUTOFF, 1400.0), (ENV_AMOUNT, 0.15), (F_ATTACK, 1.2), (F_DECAY, 3.0), (ATTACK, 0.8), (SUSTAIN, 1.0), (RELEASE, 2.5), (VINTAGE, 0.6), (CHORUS, 1.0)]),
        "Sweep Pad" => set(&[(MIX2, 0.9), (DETUNE2, 12.0), (UNISON, 2.0), (VOICES, 6.0), (CUTOFF, 450.0), (RESONANCE, 0.45), (ENV_AMOUNT, 0.0), (LFO_WAVE, 1.0), (LFO_RATE, 0.12), (LFO_FILTER, 0.55), (ATTACK, 1.0), (SUSTAIN, 1.0), (RELEASE, 2.0), (CHORUS, 1.0), (SPREAD, 0.8)]),
        "Poly Brass" => set(&[(MIX2, 0.9), (DETUNE2, 8.0), (CUTOFF, 700.0), (RESONANCE, 0.15), (ENV_AMOUNT, 0.45), (F_ATTACK, 0.08), (F_DECAY, 0.6), (F_SUSTAIN, 0.45), (FLT_VELOCITY, 0.4), (ATTACK, 0.04), (SUSTAIN, 0.9), (RELEASE, 0.3), (VELOCITY, 0.4)]),
        "Rave Stab" => set(&[(UNISON, 3.0), (UNI_DETUNE, 22.0), (VOICES, 5.0), (CUTOFF, 1800.0), (RESONANCE, 0.25), (ENV_AMOUNT, 0.4), (F_DECAY, 0.25), (F_SUSTAIN, 0.0), (DECAY, 0.5), (SUSTAIN, 0.0), (RELEASE, 0.25), (CHORUS, 2.0), (SPREAD, 0.9)]),
        "Unison Saws" => set(&[(UNISON, 4.0), (UNI_DETUNE, 25.0), (VOICES, 4.0), (CUTOFF, 5000.0), (ENV_AMOUNT, 0.1), (ATTACK, 0.02), (SUSTAIN, 1.0), (RELEASE, 0.6), (CHORUS, 2.0), (SPREAD, 1.0)]),
        "Poly Keys" => set(&[(WAVE2, pulse), (MIX2, 0.6), (PW2, 0.35), (CUTOFF, 1100.0), (ENV_AMOUNT, 0.4), (F_DECAY, 0.7), (F_SUSTAIN, 0.2), (DECAY, 1.2), (SUSTAIN, 0.5), (RELEASE, 0.4), (VELOCITY, 0.7), (FLT_VELOCITY, 0.5)]),
        "Pluck" => set(&[(CUTOFF, 450.0), (RESONANCE, 0.25), (ENV_AMOUNT, 0.6), (F_DECAY, 0.18), (F_SUSTAIN, 0.0), (DECAY, 0.45), (SUSTAIN, 0.0), (RELEASE, 0.35), (CHORUS, 1.0)]),
        "Funk Clav" => set(&[(WAVE1, pulse), (PW1, 0.2), (FILTER, 2.0), (CUTOFF, 900.0), (RESONANCE, 0.45), (ENV_AMOUNT, 0.5), (F_DECAY, 0.12), (F_SUSTAIN, 0.0), (DECAY, 0.25), (SUSTAIN, 0.1), (RELEASE, 0.08), (VELOCITY, 0.7), (VOLUME, 4.0)]),
        "Organ" => set(&[(WAVE1, sine), (WAVE2, sine), (OCTAVE2, o4), (MIX2, 0.6), (DETUNE2, 0.0), (SUB, 0.5), (CUTOFF, 12000.0), (RESONANCE, 0.0), (ENV_AMOUNT, 0.0), (KEY_TRACK, 0.0), (ATTACK, 0.004), (SUSTAIN, 1.0), (RELEASE, 0.05), (VELOCITY, 0.0), (CHORUS, 3.0), (VOLUME, -3.0)]),
        "Bell" => set(&[(WAVE1, tri), (WAVE2, sine), (SEMI2, 7.0), (OCTAVE2, o4), (DETUNE2, -17.0), (CROSS_MOD, 0.55), (MIX2, 0.25), (CUTOFF, 6000.0), (ENV_AMOUNT, 0.2), (F_DECAY, 1.5), (DECAY, 2.5), (SUSTAIN, 0.0), (RELEASE, 1.5), (VELOCITY, 0.6), (CHORUS, 1.0)]),
        "Sync Lead" => set(&[mono, (WAVE2, saw), (SYNC, on), (SEMI2, 7.0), (ENV_PITCH2, 30.0), (MIX1, 0.0), (MIX2, 1.0), (CUTOFF, 3500.0), (RESONANCE, 0.2), (ENV_AMOUNT, 0.15), (F_DECAY, 0.8), (F_SUSTAIN, 0.2), (SUSTAIN, 1.0), (RELEASE, 0.2), (GLIDE, 0.04), (LFO_WAVE, 1.0), (LFO_PITCH, 0.15), (LFO_DELAY, 0.6), (SPREAD, 0.0), (VOLUME, 6.0)]),
        "Mono Lead" => set(&[mono, (MIX2, 0.9), (DETUNE2, 10.0), (CUTOFF, 1800.0), (RESONANCE, 0.3), (DRIVE, 0.4), (ENV_AMOUNT, 0.3), (SUSTAIN, 1.0), (RELEASE, 0.15), (GLIDE, 0.08), (LFO_WAVE, 1.0), (LFO_RATE, 5.5), (LFO_PITCH, 0.2), (LFO_DELAY, 0.6), (SPREAD, 0.0), (VOLUME, 5.0)]),
        "Square Lead" => set(&[mono, (WAVE1, pulse), (WAVE2, pulse), (OCTAVE2, o16), (MIX2, 0.4), (PW2, 0.3), (CUTOFF, 2500.0), (RESONANCE, 0.1), (ENV_AMOUNT, 0.2), (SUSTAIN, 1.0), (RELEASE, 0.12), (GLIDE, 0.02), (LFO_WAVE, 1.0), (LFO_PITCH, 0.15), (LFO_DELAY, 0.5), (SPREAD, 0.0), (VOLUME, 5.0)]),
        "Laser Zap" => set(&[(VOICES, 4.0), (WAVE2, pulse), (SYNC, on), (ENV_PITCH2, 48.0), (MIX1, 0.0), (MIX2, 1.0), (F_DECAY, 0.2), (F_SUSTAIN, 0.0), (CUTOFF, 6000.0), (ENV_AMOUNT, 0.0), (DECAY, 0.3), (SUSTAIN, 0.0), (RELEASE, 0.2)]),
        "Analog Kick" => set(&[mono, (WAVE2, sine), (OCTAVE2, o32), (MIX1, 0.0), (MIX2, 1.0), (ENV_PITCH2, 36.0), (F_DECAY, 0.06), (F_SUSTAIN, 0.0), (FILTER, 1.0), (CUTOFF, 20000.0), (RESONANCE, 0.0), (ENV_AMOUNT, 0.0), (KEY_TRACK, 0.0), (FLT_VELOCITY, 0.0), (DRIVE, 0.5), (DECAY, 0.4), (SUSTAIN, 0.0), (RELEASE, 0.3), (VELOCITY, 0.6), (VINTAGE, 0.0), (SPREAD, 0.0), (DETUNE2, 0.0), (VOLUME, 6.0)]),
        "Noise Hit" => set(&[(VOICES, 4.0), (MIX1, 0.0), (WAVE2, tri), (MIX2, 0.5), (ENV_PITCH2, 12.0), (F_DECAY, 0.05), (F_SUSTAIN, 0.0), (NOISE, 1.0), (FILTER, 3.0), (CUTOFF, 900.0), (RESONANCE, 0.2), (ENV_AMOUNT, 0.0), (KEY_TRACK, 0.0), (DECAY, 0.18), (SUSTAIN, 0.0), (RELEASE, 0.15), (VELOCITY, 0.6), (VOLUME, 4.0)]),
        "Wonky Tape" => set(&[(WAVE2, tri), (MIX2, 0.7), (DETUNE2, 0.0), (CUTOFF, 1600.0), (ENV_AMOUNT, 0.3), (F_DECAY, 0.8), (DECAY, 1.5), (SUSTAIN, 0.4), (RELEASE, 0.6), (VINTAGE, 1.0), (LFO_WAVE, 4.0), (LFO_RATE, 0.4), (LFO_PITCH, 0.06), (CHORUS, 1.0)]),
        _ => {}
    }
    p
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Wave {
    Saw,
    Pulse,
    Tri,
    Sine,
}

impl Wave {
    fn from_param(v: f32) -> Self {
        match v.round() as i32 {
            1 => Wave::Pulse,
            2 => Wave::Tri,
            3 => Wave::Sine,
            _ => Wave::Saw,
        }
    }

    #[inline]
    fn naive(self, p: f32, pw: f32) -> f32 {
        match self {
            Wave::Saw => 2.0 * p - 1.0,
            Wave::Pulse => {
                if p < pw {
                    1.0
                } else {
                    -1.0
                }
            }
            Wave::Tri => 1.0 - 4.0 * (p - 0.5).abs(),
            Wave::Sine => (2.0 * PI * p).sin(),
        }
    }

    /// Slope in value per sample at phase `p`.
    #[inline]
    fn slope(self, p: f32, dt: f32) -> f32 {
        match self {
            Wave::Saw => 2.0 * dt,
            Wave::Pulse => 0.0,
            Wave::Tri => {
                if p < 0.5 {
                    4.0 * dt
                } else {
                    -4.0 * dt
                }
            }
            Wave::Sine => 2.0 * PI * dt * (2.0 * PI * p).cos(),
        }
    }
}

/// Band-limiting corrections for the previous and the current sample: two
/// sample PolyBLEP residuals for steps, and their integral (BLAMP) for
/// corners. `x` is how long ago (in samples, 0..1) the event happened.
#[derive(Default)]
struct Blep {
    prev: f32,
    now: f32,
}

impl Blep {
    #[inline]
    fn step(&mut self, height: f32, x: f32) {
        let x = x.clamp(0.0, 1.0);
        self.prev += 0.5 * height * x * x;
        let y = 1.0 - x;
        self.now -= 0.5 * height * y * y;
    }

    #[inline]
    fn corner(&mut self, slope_change: f32, x: f32) {
        let x = x.clamp(0.0, 1.0);
        self.prev += slope_change * x * x * x / 6.0;
        let y = 1.0 - x;
        self.now += slope_change * y * y * y / 6.0;
    }
}

/// Moves a phase from `a` to `b` (`b` may pass 1 once) and records the
/// waveform's steps and corners on the way. The end point lies `t_end`
/// samples before the current sample. Returns the wrapped phase.
#[inline]
fn advance(w: Wave, a: f32, b: f32, dt: f32, pw: f32, t_end: f32, level: f32, bl: &mut Blep) -> f32 {
    if level != 0.0 && dt > 0.0 {
        let since = |e: f32| t_end + (b - e) / dt;
        match w {
            Wave::Saw => {
                if b >= 1.0 {
                    bl.step(-2.0 * level, since(1.0));
                }
            }
            Wave::Pulse => {
                if a < pw && b >= pw {
                    bl.step(-2.0 * level, since(pw));
                }
                if b >= 1.0 {
                    bl.step(2.0 * level, since(1.0));
                    if b - 1.0 >= pw {
                        bl.step(-2.0 * level, since(1.0 + pw));
                    }
                }
            }
            Wave::Tri => {
                if a < 0.5 && b >= 0.5 {
                    bl.corner(-8.0 * dt * level, since(0.5));
                }
                if b >= 1.0 {
                    bl.corner(8.0 * dt * level, since(1.0));
                    if b - 1.0 >= 0.5 {
                        bl.corner(-8.0 * dt * level, since(1.5));
                    }
                }
            }
            Wave::Sine => {}
        }
    }
    if b >= 1.0 {
        b - 1.0
    } else {
        b
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Release,
}

/// RC-style envelope: the attack charges towards 1.3 and stops at 1, decay
/// and release are exponential.
#[derive(Clone, Copy, Debug)]
struct Env {
    level: f32,
    stage: Stage,
}

#[derive(Clone, Copy, Debug)]
struct EnvCoefs {
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}

impl EnvCoefs {
    fn new(a: f32, d: f32, s: f32, r: f32, rate: f32) -> Self {
        // Time to reach 1 when charging towards 1.3: ln(1.3 / 0.3).
        let k = |t: f32, c: f32| 1.0 - (-c / (t.max(0.0005) * rate)).exp();
        Self { attack: k(a, 1.466), decay: k(d, 4.6), sustain: s.clamp(0.0, 1.0), release: k(r, 4.6) }
    }
}

impl Env {
    const IDLE: Self = Self { level: 0.0, stage: Stage::Idle };

    #[inline]
    fn next(&mut self, c: &EnvCoefs) -> f32 {
        match self.stage {
            Stage::Attack => {
                self.level += (1.3 - self.level) * c.attack;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => self.level += (c.sustain - self.level) * c.decay,
            Stage::Release => {
                self.level -= self.level * c.release;
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

#[derive(Clone, Debug)]
struct Voice {
    note: u8,
    gate: bool,
    active: bool,
    age: u64,
    velocity: f32,
    pitch: f32,
    target: f32,
    /// Unison position, -1..1 (0 without unison).
    uni: f32,
    pan: f32,
    p1: f32,
    p2: f32,
    psub: f32,
    pending: f32,
    osc2_out: f32,
    ladder: [f32; 4],
    s1: f32,
    s2: f32,
    g: f32,
    fenv: Env,
    aenv: Env,
    since_on: f32,
    rng: Rng,
    drift: [f32; 2],
    drift_target: [f32; 2],
    drift_timer: u32,
    /// Fixed per-voice component spread, -1..1, scaled by VINTAGE.
    tol_cutoff: f32,
    tol_time: f32,
    tol_detune: f32,
    /// The filter's cutoff at the last control step (Hz), for displays.
    cut_hz: f32,
}

impl Voice {
    fn new(index: usize) -> Self {
        let mut rng = Rng::new(0x51ED_2701 ^ (index as u32 + 1).wrapping_mul(0x9E37_79B9));
        let (tol_cutoff, tol_time, tol_detune) = (rng.bipolar(), rng.bipolar(), rng.bipolar());
        Self {
            note: 0,
            gate: false,
            active: false,
            age: 0,
            velocity: 0.0,
            pitch: 60.0,
            target: 60.0,
            uni: 0.0,
            pan: 0.0,
            p1: 0.0,
            p2: 0.0,
            psub: 0.0,
            pending: 0.0,
            osc2_out: 0.0,
            ladder: [0.0; 4],
            s1: 0.0,
            s2: 0.0,
            g: 0.1,
            fenv: Env::IDLE,
            aenv: Env::IDLE,
            since_on: 0.0,
            rng,
            drift: [0.0; 2],
            drift_target: [0.0; 2],
            drift_timer: 0,
            tol_cutoff,
            tol_time,
            tol_detune,
            cut_hz: 1000.0,
        }
    }

    fn start(&mut self, note: u8, vel: f32, glide: bool, age: u64) {
        if !self.active {
            // Free-running oscillators: an idle voice starts anywhere.
            self.p1 = (self.rng.bipolar() + 1.0) * 0.5;
            self.p2 = (self.rng.bipolar() + 1.0) * 0.5;
            self.psub = self.p1 * 0.5;
        }
        if !glide || !self.active {
            self.pitch = note as f32;
        }
        self.note = note;
        self.target = note as f32;
        self.gate = true;
        self.active = true;
        self.velocity = vel;
        self.age = age;
        self.since_on = 0.0;
        // Like analog envelopes, a retrigger starts from the current level.
        self.fenv.stage = Stage::Attack;
        self.aenv.stage = Stage::Attack;
    }

    fn release(&mut self) {
        self.gate = false;
        if self.aenv.stage != Stage::Idle {
            self.aenv.stage = Stage::Release;
        }
        if self.fenv.stage != Stage::Idle {
            self.fenv.stage = Stage::Release;
        }
    }
}

/// Per-render settings, from the parameters.
struct Ctrl {
    fs2: f32,
    /// Pitch bend in semitones.
    bend: f32,
    wave1: Wave,
    wave2: Wave,
    oct1: f32,
    oct2: f32,
    pw1: f32,
    pw2: f32,
    xmod: f32,
    sync: bool,
    env_pitch2: f32,
    l1: f32,
    l2: f32,
    lsub: f32,
    lnoise: f32,
    filter: usize,
    cutoff_note: f32,
    k: f32,
    svf_r: f32,
    drive: f32,
    out_gain: f32,
    env_amount: f32,
    key_track: f32,
    flt_velocity: f32,
    fenv: [f32; 4],
    aenv: [f32; 4],
    velocity: f32,
    lfo_delay: f32,
    lfo_pitch: f32,
    lfo_filter: f32,
    lfo_pwm: f32,
    uni_detune: f32,
    glide_k: f32,
    vintage: f32,
    spread: f32,
    /// Unison voices are mixed at 1/sqrt(n) so stacking keeps the level.
    uni_gain: f32,
    fmax: f32,
}

/// Bucket-brigade-style stereo chorus with the three classic modes: two
/// modulated delays in antiphase, darkened like a BBD.
struct Chorus {
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    pos: usize,
    mask: usize,
    phase: f32,
    lp_l: f32,
    lp_r: f32,
    mix: f32,
    sr: f32,
}

impl Chorus {
    fn new(sr: f32) -> Self {
        let len = ((0.02 * sr) as usize).next_power_of_two();
        Self { buf_l: vec![0.0; len], buf_r: vec![0.0; len], pos: 0, mask: len - 1, phase: 0.0, lp_l: 0.0, lp_r: 0.0, mix: 0.0, sr }
    }

    #[inline]
    fn read(buf: &[f32], mask: usize, pos: usize, delay: f32) -> f32 {
        let d = delay.max(1.0);
        let i = d as usize;
        let f = d - i as f32;
        let a = buf[(pos + buf.len() - i) & mask];
        let b = buf[(pos + buf.len() - i - 1) & mask];
        a + (b - a) * f
    }

    fn process(&mut self, mode: usize, l: &mut [f32], r: &mut [f32]) {
        let target = if mode == 0 { 0.0 } else { 1.0 };
        if target == 0.0 && self.mix < 1e-4 {
            self.mix = 0.0;
            return;
        }
        // (LFO rate Hz, center delay ms, depth ms)
        let (rate, center, depth) = match mode {
            2 => (0.863, 3.5, 1.85),
            3 => (9.75, 3.3, 0.3),
            _ => (0.513, 3.5, 1.85),
        };
        let ms = self.sr / 1000.0;
        let inc = rate / self.sr;
        let lp = 1.0 - (-2.0 * PI * 9000.0 / self.sr).exp();
        let mix_k = 1.0 - (-1.0 / (0.02 * self.sr)).exp();
        for (xl, xr) in l.iter_mut().zip(r.iter_mut()) {
            self.buf_l[self.pos] = *xl;
            self.buf_r[self.pos] = *xr;
            self.phase += inc;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            let tri = 1.0 - 4.0 * (self.phase - 0.5).abs();
            let wl = Self::read(&self.buf_l, self.mask, self.pos, (center + depth * tri) * ms);
            let wr = Self::read(&self.buf_r, self.mask, self.pos, (center - depth * tri) * ms);
            self.lp_l += (wl - self.lp_l) * lp;
            self.lp_r += (wr - self.lp_r) * lp;
            self.pos = (self.pos + 1) & self.mask;
            self.mix += (target - self.mix) * mix_k;
            let m = self.mix;
            *xl += m * (0.75 * (*xl + self.lp_l) - *xl);
            *xr += m * (0.75 * (*xr + self.lp_r) - *xr);
        }
    }
}

/// A windowed-sinc half-band low-pass for 2:1 decimation.
fn halfband() -> [f32; FIR_TAPS] {
    let mut h = [0.0f32; FIR_TAPS];
    let m = (FIR_TAPS - 1) as f32 / 2.0;
    let mut sum = 0.0;
    for (i, c) in h.iter_mut().enumerate() {
        let x = i as f32 - m;
        let sinc = if x == 0.0 { 1.0 } else { (PI * x * 0.5).sin() / (PI * x * 0.5) };
        let t = 2.0 * PI * i as f32 / (FIR_TAPS - 1) as f32;
        let w = 0.35875 - 0.48829 * t.cos() + 0.14128 * (2.0 * t).cos() - 0.01168 * (3.0 * t).cos();
        *c = sinc * w;
        sum += *c;
    }
    for c in h.iter_mut() {
        *c /= sum;
    }
    h
}

pub struct AnalogState {
    /// Pitch bend in semitones (a wheel, a controller's triggers), set by the engine.
    pub pitch_bend: f32,
    voices: Vec<Voice>,
    held: Vec<u8>,
    counter: u64,
    sr: f32,
    lfo_phase: f32,
    lfo_hold: f32,
    lfo_rng: Rng,
    os_l: Vec<f32>,
    os_r: Vec<f32>,
    fir: [f32; FIR_TAPS],
    hist_l: [f32; 2 * FIR_TAPS],
    hist_r: [f32; 2 * FIR_TAPS],
    hist_pos: usize,
    chorus: Chorus,
}

const PANS: [f32; 8] = [-0.6, 0.6, -0.3, 0.3, -0.9, 0.9, -0.15, 0.15];

impl AnalogState {
    pub fn new(sr: f32) -> Self {
        Self {
            pitch_bend: 0.0,
            voices: (0..MAX_VOICES).map(Voice::new).collect(),
            held: Vec::with_capacity(32),
            counter: 0,
            sr,
            lfo_phase: 0.0,
            lfo_hold: 0.0,
            lfo_rng: Rng::new(0xA4A1_06),
            os_l: vec![0.0; 2 * CHUNK],
            os_r: vec![0.0; 2 * CHUNK],
            fir: halfband(),
            hist_l: [0.0; 2 * FIR_TAPS],
            hist_r: [0.0; 2 * FIR_TAPS],
            hist_pos: 0,
            chorus: Chorus::new(sr),
        }
    }

    /// (polyphony, unison) for the parameters.
    fn layout(p: &[f32]) -> (usize, usize) {
        let unison = (p[UNISON].round() as usize).clamp(1, 4);
        let poly = (p[VOICES].round() as usize).clamp(1, 8).min(MAX_VOICES / unison);
        (poly, unison)
    }

    fn glide_on(p: &[f32]) -> bool {
        p[GLIDE] > 0.0015
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let (poly, unison) = Self::layout(p);
        let glide = Self::glide_on(p);
        self.counter += 1;
        let age = self.counter;
        let uni_pos = |u: usize| if unison > 1 { u as f32 / (unison - 1) as f32 * 2.0 - 1.0 } else { 0.0 };
        if poly == 1 {
            // Mono: legato while a key is held, last note priority.
            let legato = !self.held.is_empty() && self.voices[0].gate;
            self.held.retain(|&h| h != note);
            if self.held.len() < self.held.capacity() {
                self.held.push(note);
            }
            for (u, v) in self.voices[..unison].iter_mut().enumerate() {
                v.uni = uni_pos(u);
                v.pan = v.uni;
                if legato && v.gate {
                    v.note = note;
                    v.target = note as f32;
                    if !glide {
                        v.pitch = note as f32;
                    }
                } else {
                    v.start(note, vel, glide, age);
                }
            }
            return;
        }
        let slots = poly * unison;
        let pool = &mut self.voices[..slots];
        // A key that is still held restarts its own voices.
        if pool.iter().any(|v| v.gate && v.note == note) {
            for v in pool.iter_mut().filter(|v| v.gate && v.note == note) {
                v.start(note, vel, glide, age);
            }
            return;
        }
        for u in 0..unison {
            let pick = pool
                .iter()
                .position(|v| !v.active)
                .or_else(|| pool.iter().enumerate().filter(|(_, v)| !v.gate && v.age != age).min_by_key(|(_, v)| v.age).map(|(i, _)| i))
                .unwrap_or_else(|| pool.iter().enumerate().filter(|(_, v)| v.age != age).min_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0));
            let v = &mut pool[pick];
            v.uni = uni_pos(u);
            v.pan = if unison > 1 { v.uni } else { PANS[pick % PANS.len()] };
            v.start(note, vel, glide, age);
        }
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        if p.len() < SPECS.len() {
            return;
        }
        let (poly, unison) = Self::layout(p);
        if poly == 1 {
            self.held.retain(|&h| h != note);
            let glide = Self::glide_on(p);
            let back = self.held.last().copied();
            for v in self.voices[..unison].iter_mut() {
                match back {
                    Some(b) if v.gate => {
                        v.note = b;
                        v.target = b as f32;
                        if !glide {
                            v.pitch = b as f32;
                        }
                    }
                    _ => {
                        if v.note == note || self.held.is_empty() {
                            v.release();
                        }
                    }
                }
            }
            return;
        }
        for v in self.voices.iter_mut() {
            if v.gate && v.note == note {
                v.release();
            }
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in self.voices.iter_mut() {
            v.release();
        }
    }

    /// For the display: [0] voices shown (newest first, at most 6), then per
    /// voice from [1] four values: amp envelope level and stage, filter
    /// envelope level and stage (stage 1 attack, 2 decay/sustain, 3 release);
    /// [25] the newest voice's cutoff (Hz) and [26] its pitch.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        // The newest voices first, without allocating (this runs on the audio thread).
        let mut newest: [Option<&Voice>; 6] = [None; 6];
        let mut below = u64::MAX;
        for slot in newest.iter_mut() {
            *slot = self.voices.iter().filter(|v| v.active && v.age < below).max_by_key(|v| v.age);
            match slot {
                Some(v) => below = v.age,
                None => break,
            }
        }
        let on = || newest.iter().map_while(|v| *v);
        let stage = |s: Stage| match s {
            Stage::Idle => 0.0,
            Stage::Attack => 1.0,
            Stage::Decay => 2.0,
            Stage::Release => 3.0,
        };
        out[0] = on().count() as f32;
        for (k, v) in on().enumerate() {
            out[1 + k * 4] = v.aenv.level;
            out[2 + k * 4] = stage(v.aenv.stage);
            out[3 + k * 4] = v.fenv.level;
            out[4 + k * 4] = stage(v.fenv.stage);
        }
        if let Some(v) = newest[0] {
            out[25] = v.cut_hz;
            out[26] = v.pitch;
        }
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    fn ctrl(&self, p: &[f32]) -> Ctrl {
        let fs2 = 2.0 * self.sr;
        let octave = |v: f32| (v.round().clamp(0.0, 4.0) - 2.0) * 12.0;
        let drive = 1.0 + p[DRIVE].clamp(0.0, 1.0) * 7.0;
        let reso = p[RESONANCE].clamp(0.0, 1.0);
        let filter = (p[FILTER].round().max(0.0) as usize).min(FILTERS.len() - 1);
        let k = reso * 4.2;
        // Make up the level lost to resonance (ladder) and to the drive.
        let out_gain = if filter == 0 { (1.0 + 0.45 * k) / drive.sqrt() } else { 1.0 / drive.sqrt() };
        let fenv = [p[F_ATTACK], p[F_DECAY], p[F_SUSTAIN], p[F_RELEASE]];
        let aenv = [p[ATTACK], p[DECAY], p[SUSTAIN], p[RELEASE]];
        let block = CTRL as f32 / fs2;
        Ctrl {
            bend: self.pitch_bend,
            fs2,
            wave1: Wave::from_param(p[WAVE1]),
            wave2: Wave::from_param(p[WAVE2]),
            oct1: octave(p[OCTAVE1]),
            oct2: octave(p[OCTAVE2]) + p[SEMI2].round() + p[DETUNE2] / 100.0,
            pw1: p[PW1].clamp(0.05, 0.95),
            pw2: p[PW2].clamp(0.05, 0.95),
            xmod: p[CROSS_MOD].clamp(0.0, 1.0),
            sync: p[SYNC] >= 0.5,
            env_pitch2: p[ENV_PITCH2],
            l1: p[MIX1].clamp(0.0, 1.0) * 0.5,
            l2: p[MIX2].clamp(0.0, 1.0) * 0.5,
            lsub: p[SUB].clamp(0.0, 1.0) * 0.5,
            lnoise: p[NOISE].clamp(0.0, 1.0) * 0.4,
            filter,
            cutoff_note: 69.0 + 12.0 * (p[CUTOFF].max(10.0) / 440.0).log2(),
            k,
            svf_r: 1.0 - reso * 0.97,
            drive,
            out_gain,
            env_amount: p[ENV_AMOUNT] * 72.0,
            key_track: p[KEY_TRACK],
            flt_velocity: p[FLT_VELOCITY] * 24.0,
            fenv,
            aenv,
            velocity: p[VELOCITY].clamp(0.0, 1.0),
            lfo_delay: p[LFO_DELAY].max(0.0),
            lfo_pitch: p[LFO_PITCH],
            lfo_filter: p[LFO_FILTER] * 36.0,
            lfo_pwm: p[LFO_PWM] * 0.45,
            uni_detune: p[UNI_DETUNE] / 100.0,
            glide_k: 1.0 - (-block / p[GLIDE].max(1e-4)).exp(),
            vintage: p[VINTAGE].clamp(0.0, 1.0),
            spread: p[SPREAD].clamp(0.0, 1.0),
            uni_gain: 1.0 / (Self::layout(p).1 as f32).sqrt(),
            fmax: (0.42 * fs2).min(24000.0),
        }
    }

    /// The shared LFO, advanced by one control step.
    fn lfo_step(&mut self, wave: usize, rate: f32, dt: f32) -> f32 {
        self.lfo_phase += rate * dt;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= self.lfo_phase.floor();
            self.lfo_hold = self.lfo_rng.bipolar();
        }
        let ph = self.lfo_phase;
        match wave {
            1 => (2.0 * PI * ph).sin(),
            2 => 1.0 - 2.0 * ph,
            3 => {
                if ph < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            4 => self.lfo_hold,
            _ => 1.0 - 4.0 * (ph - 0.5).abs(),
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let c = self.ctrl(p);
        let master = db_to_gain(p[VOLUME]) * 0.35;
        let lfo_wave = p[LFO_WAVE].round().max(0.0) as usize;
        let lfo_rate = p[LFO_RATE].max(0.01);
        let chorus = (p[CHORUS].round().max(0.0) as usize).min(3);
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            let os = 2 * len;
            self.os_l[..os].fill(0.0);
            self.os_r[..os].fill(0.0);
            // One LFO value per control step, shared by the voices.
            let steps = os.div_ceil(CTRL);
            let mut lfo = [0.0f32; 2 * CHUNK / CTRL + 1];
            for v in lfo.iter_mut().take(steps) {
                *v = self.lfo_step(lfo_wave, lfo_rate, CTRL as f32 / c.fs2);
            }
            for v in self.voices.iter_mut().filter(|v| v.active) {
                render_voice(v, &c, &lfo[..steps], &mut self.os_l[..os], &mut self.os_r[..os]);
            }
            // Back to the host rate.
            for i in 0..len {
                for k in 0..2 {
                    let (xl, xr) = (self.os_l[2 * i + k], self.os_r[2 * i + k]);
                    self.hist_l[self.hist_pos] = xl;
                    self.hist_l[self.hist_pos + FIR_TAPS] = xl;
                    self.hist_r[self.hist_pos] = xr;
                    self.hist_r[self.hist_pos + FIR_TAPS] = xr;
                    self.hist_pos = (self.hist_pos + 1) % FIR_TAPS;
                }
                let hl = &self.hist_l[self.hist_pos..self.hist_pos + FIR_TAPS];
                let hr = &self.hist_r[self.hist_pos..self.hist_pos + FIR_TAPS];
                let (mut yl, mut yr) = (0.0, 0.0);
                for t in 0..FIR_TAPS {
                    yl += self.fir[t] * hl[t];
                    yr += self.fir[t] * hr[t];
                }
                self.os_l[i] = yl * master;
                self.os_r[i] = yr * master;
            }
            self.chorus.process(chorus, &mut self.os_l[..len], &mut self.os_r[..len]);
            for i in 0..len {
                l[done + i] += self.os_l[i];
                r[done + i] += self.os_r[i];
            }
            done += len;
        }
    }
}

fn render_voice(v: &mut Voice, c: &Ctrl, lfo: &[f32], out_l: &mut [f32], out_r: &mut [f32]) {
    let n = out_l.len();
    let fs2 = c.fs2;
    let time_scale = 1.0 + 0.1 * c.vintage * v.tol_time;
    let fe = EnvCoefs::new(c.fenv[0] * time_scale, c.fenv[1] * time_scale, c.fenv[2], c.fenv[3] * time_scale, fs2);
    let ae = EnvCoefs::new(c.aenv[0] * time_scale, c.aenv[1] * time_scale, c.aenv[2], c.aenv[3] * time_scale, fs2);
    let vel_gain = (1.0 - c.velocity + c.velocity * v.velocity) * c.uni_gain;
    let (gl, gr) = pan_gains(v.pan * c.spread);
    let (gl, gr) = (gl * vel_gain * c.out_gain, gr * vel_gain * c.out_gain);
    let drift_cents = c.vintage * 0.08;
    let mut i = 0;
    let mut step = 0;
    while i < n && v.active {
        let len = CTRL.min(n - i);
        // --- Control rate: glide, drift, LFO, pitch and cutoff.
        v.pitch += (v.target - v.pitch) * c.glide_k;
        if c.vintage > 0.0 {
            if v.drift_timer == 0 {
                v.drift_target = [v.rng.bipolar(), v.rng.bipolar()];
                v.drift_timer = 1500 + (v.rng.next_u32() % 6000);
            }
            v.drift_timer -= 1;
            for k in 0..2 {
                v.drift[k] += (v.drift_target[k] - v.drift[k]) * 0.0005;
            }
        }
        let fade = if c.lfo_delay <= 0.0 { 1.0 } else { ((v.since_on - 0.5 * c.lfo_delay) / (0.5 * c.lfo_delay)).clamp(0.0, 1.0) };
        v.since_on += len as f32 / fs2;
        let mod_lfo = lfo.get(step).copied().unwrap_or(0.0) * fade;
        step += 1;
        let base = v.pitch + v.uni * c.uni_detune + mod_lfo * c.lfo_pitch + c.bend;
        let note1 = base + c.oct1 + v.drift[0] * drift_cents;
        let note2 = base + c.oct2 + v.drift[1] * drift_cents + v.tol_detune * c.vintage * 0.03 + c.env_pitch2 * v.fenv.level;
        let dt1 = (midi_to_freq(note1) / fs2).min(0.45);
        let dt2 = (midi_to_freq(note2) / fs2).min(0.45);
        let pw1 = (c.pw1 + mod_lfo * c.lfo_pwm).clamp(0.03, 0.97);
        let pw2 = (c.pw2 + mod_lfo * c.lfo_pwm).clamp(0.03, 0.97);
        let cut = c.cutoff_note
            + c.env_amount * v.fenv.level
            + c.key_track * (v.pitch - 60.0)
            + c.flt_velocity * (v.velocity * 2.0 - 1.0)
            + mod_lfo * c.lfo_filter
            + v.tol_cutoff * c.vintage * 1.5;
        let f = midi_to_freq(cut).clamp(15.0, c.fmax);
        v.cut_hz = f;
        let g_target = (PI * f / fs2).tan();
        let g_step = (g_target - v.g) / len as f32;

        for j in 0..len {
            v.g += g_step;
            v.fenv.next(&fe);
            let amp = v.aenv.next(&ae);

            // --- Oscillators, band-limited with a one-sample delay.
            let mut bl = Blep::default();
            let dt1e = (dt1 * (1.0 + c.xmod * 2.0 * v.osc2_out)).clamp(0.0, 0.45);
            let b1 = v.p1 + dt1e;
            let wrapped = b1 >= 1.0;
            v.p1 = advance(c.wave1, v.p1, b1, dt1e, pw1, 0.0, c.l1, &mut bl);
            let bs = v.psub + 0.5 * dt1e;
            v.psub = advance(Wave::Pulse, v.psub, bs, 0.5 * dt1e, 0.5, 0.0, c.lsub, &mut bl);
            if c.sync && wrapped && dt1e > 0.0 {
                // Reset osc 2 at the moment osc 1 wrapped, `x` samples ago.
                let x = (v.p1 / dt1e).min(1.0);
                let ps = advance(c.wave2, v.p2, v.p2 + dt2 * (1.0 - x), dt2, pw2, x, c.l2, &mut bl);
                if c.l2 != 0.0 {
                    bl.step((c.wave2.naive(0.0, pw2) - c.wave2.naive(ps, pw2)) * c.l2, x);
                    let m = c.wave2.slope(0.0, dt2) - c.wave2.slope(ps, dt2);
                    if m != 0.0 {
                        bl.corner(m * c.l2, x);
                    }
                }
                v.p2 = advance(c.wave2, 0.0, dt2 * x, dt2, pw2, 0.0, c.l2, &mut bl);
            } else {
                v.p2 = advance(c.wave2, v.p2, v.p2 + dt2, dt2, pw2, 0.0, c.l2, &mut bl);
            }
            let o1 = c.wave1.naive(v.p1, pw1);
            let o2 = c.wave2.naive(v.p2, pw2);
            v.osc2_out = o2;
            let sub = if v.psub < 0.5 { 1.0 } else { -1.0 };
            let noise = if c.lnoise > 0.0 { v.rng.bipolar() } else { 0.0 };
            let s = c.l1 * o1 + c.l2 * o2 + c.lsub * sub + c.lnoise * noise;
            let osc = v.pending + bl.prev;
            v.pending = s + bl.now;

            // --- Filter.
            let x = osc * c.drive;
            let g = v.g;
            let y = if c.filter == 0 {
                // Zero-delay-feedback ladder; the feedback sum is saturated.
                let gg = g / (1.0 + g);
                let b = 1.0 - gg;
                let st = &mut v.ladder;
                let s_sum = (gg * (gg * (gg * st[0] + st[1]) + st[2]) + st[3]) * b;
                let g4 = gg * gg * gg * gg;
                let y4 = (g4 * x + s_sum) / (1.0 + c.k * g4);
                let mut u = fast_tanh(x - c.k * y4);
                for s in st.iter_mut() {
                    let w = (u - *s) * gg;
                    let yy = w + *s;
                    *s = yy + w;
                    u = yy;
                }
                u
            } else {
                // State-variable filter after a saturating input stage.
                let r = c.svf_r;
                let h = 1.0 / (1.0 + 2.0 * r * g + g * g);
                let xin = fast_tanh(x);
                let hp = (xin - (2.0 * r + g) * v.s1 - v.s2) * h;
                let bp = g * hp + v.s1;
                v.s1 = g * hp + bp;
                let lp = g * bp + v.s2;
                v.s2 = g * bp + lp;
                match c.filter {
                    1 => lp,
                    2 => bp * 2.0 * r,
                    _ => hp,
                }
            };

            // --- Amplifier.
            let out = y * amp;
            out_l[i + j] += out * gl;
            out_r[i + j] += out * gr;
        }
        if v.aenv.stage == Stage::Idle {
            v.active = false;
            v.gate = false;
        }
        i += len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(st: &mut AnalogState, p: &[f32], secs: f32, sr: f32) -> (Vec<f32>, Vec<f32>) {
        let n = (secs * sr) as usize;
        let mut l = vec![0.0; n];
        let mut r = vec![0.0; n];
        for (cl, cr) in l.chunks_mut(64).zip(r.chunks_mut(64)) {
            st.render(p, cl, cr);
        }
        (l, r)
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// Pitch by normalized autocorrelation with parabolic refinement.
    fn pitch_hz(x: &[f32], sr: f32, lo: f32, hi: f32) -> f32 {
        let (min_lag, max_lag) = ((sr / hi) as usize, (sr / lo) as usize);
        let n = x.len() - max_lag - 2;
        let corr = |lag: usize| {
            let (mut xy, mut xx, mut yy) = (0.0f64, 0.0f64, 0.0f64);
            for i in 0..n {
                let (a, b) = (x[i] as f64, x[i + lag] as f64);
                xy += a * b;
                xx += a * a;
                yy += b * b;
            }
            xy / (xx * yy).sqrt().max(1e-20)
        };
        let s: Vec<f64> = (min_lag..=max_lag + 1).map(corr).collect();
        // The first peak close to the best one: multiples of the period
        // correlate just as well.
        let best = s.iter().cloned().fold(f64::MIN, f64::max);
        let b = (1..s.len() - 1).find(|&i| s[i] >= 0.97 * best && s[i] >= s[i - 1] && s[i] >= s[i + 1]).unwrap();
        let shift = 0.5 * (s[b - 1] - s[b + 1]) / (s[b - 1] - 2.0 * s[b] + s[b + 1]);
        sr / ((min_lag + b) as f64 + shift) as f32
    }

    #[test]
    fn every_preset_sounds_and_stops() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = AnalogState::new(48000.0);
            st.note_on(&p, 48, 0.9);
            st.note_on(&p, 55, 0.9);
            let (l, r) = render(&mut st, &p, 0.5, 48000.0);
            let level = rms(&l).max(rms(&r));
            assert!(level > 3e-3, "{name} is silent ({level})");
            let peak = l.iter().chain(&r).fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(l.iter().chain(&r).all(|v| v.is_finite()) && peak < 1.5, "{name} peaks at {peak}");
            st.note_off(&p, 48);
            st.note_off(&p, 55);
            let _ = render(&mut st, &p, 12.0, 48000.0);
            assert_eq!(st.active_voices(), 0, "{name} keeps a voice");
        }
    }

    #[test]
    fn in_tune_at_48k_and_44k1() {
        for sr in [48000.0f32, 44100.0] {
            let mut p = preset("Init");
            p[WAVE1] = 2.0; // triangle
            p[CUTOFF] = 20000.0;
            p[ENV_AMOUNT] = 0.0;
            p[VINTAGE] = 0.0;
            let mut st = AnalogState::new(sr);
            st.note_on(&p, 57, 0.8);
            let (l, _) = render(&mut st, &p, 0.4, sr);
            let f = pitch_hz(&l[(0.1 * sr) as usize..], sr, 100.0, 800.0);
            assert!((f - 220.0).abs() < 0.5, "{sr}: {f} Hz");
        }
    }

    #[test]
    fn oscillators_are_band_limited() {
        // A high note with an open filter: everything that is not a harmonic
        // of the note is aliasing.
        let mut p = preset("Init");
        p[CUTOFF] = 20000.0;
        p[ENV_AMOUNT] = 0.0;
        p[KEY_TRACK] = 0.0;
        p[DRIVE] = 0.0;
        p[RESONANCE] = 0.0;
        p[VINTAGE] = 0.0;
        for (wave, sync) in [(0.0, false), (1.0, false), (0.0, true)] {
            p[WAVE1] = wave;
            p[WAVE2] = wave;
            p[MIX1] = if sync { 0.0 } else { 1.0 };
            p[MIX2] = if sync { 1.0 } else { 0.0 };
            p[SYNC] = if sync { 1.0 } else { 0.0 };
            p[SEMI2] = if sync { 5.0 } else { 0.0 };
            let mut st = AnalogState::new(48000.0);
            st.note_on(&p, 96, 0.8); // ~2093 Hz
            let (l, _) = render(&mut st, &p, 0.3, 48000.0);
            // Harmonics of 2093 Hz up to 20 kHz are real; anything that is
            // not a harmonic is aliasing. Measure between harmonics 1 and 2.
            let n = 8192;
            let x = &l[l.len() - n..];
            let f0 = midi_to_freq(96.0);
            let bin = |hz: f32| {
                let w = 2.0 * std::f64::consts::PI * hz as f64 / 48000.0;
                let (mut re, mut im) = (0.0f64, 0.0f64);
                for (i, &s) in x.iter().enumerate() {
                    let win = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos();
                    re += s as f64 * win * (w * i as f64).cos();
                    im += s as f64 * win * (w * i as f64).sin();
                }
                (re * re + im * im).sqrt()
            };
            let fundamental = bin(f0);
            let worst = (1..40).map(|k| bin(f0 * (1.0 + k as f32 / 41.0))).fold(0.0f64, f64::max);
            let ratio_db = 20.0 * (worst / fundamental).log10();
            if std::env::var("ANALOG_REPORT").is_ok() {
                eprintln!("wave {wave} sync {sync}: worst non-harmonic {ratio_db:.1} dB");
            }
            assert!(ratio_db < -40.0, "wave {wave} sync {sync}: aliasing at {ratio_db:.1} dB");
        }
    }

    #[test]
    fn ladder_resonance_self_oscillates_and_stays_bounded() {
        let mut p = preset("Init");
        p[MIX1] = 0.0;
        p[NOISE] = 0.02;
        p[RESONANCE] = 1.0;
        p[CUTOFF] = 880.0;
        p[ENV_AMOUNT] = 0.0;
        p[KEY_TRACK] = 0.0;
        p[FLT_VELOCITY] = 0.0;
        p[VINTAGE] = 0.0;
        p[SUSTAIN] = 1.0;
        let mut st = AnalogState::new(48000.0);
        st.note_on(&p, 60, 1.0);
        let (l, _) = render(&mut st, &p, 1.0, 48000.0);
        let tail = &l[24000..];
        assert!(rms(tail) > 0.02, "no self-oscillation: {}", rms(tail));
        assert!(tail.iter().all(|v| v.abs() < 1.5));
        let f = pitch_hz(tail, 48000.0, 400.0, 2000.0);
        assert!((f - 880.0).abs() < 880.0 * 0.06, "self-oscillation at {f} Hz");
    }

    #[test]
    fn unison_mono_and_glide() {
        let mut p = preset("Init");
        p[VOICES] = 1.0;
        p[UNISON] = 3.0;
        let mut st = AnalogState::new(48000.0);
        st.note_on(&p, 60, 0.8);
        render(&mut st, &p, 0.05, 48000.0);
        assert_eq!(st.active_voices(), 3);
        // Legato: a second key moves the same voices.
        st.note_on(&p, 64, 0.8);
        render(&mut st, &p, 0.05, 48000.0);
        assert_eq!(st.active_voices(), 3);
        st.note_off(&p, 64);
        st.note_off(&p, 60);
        render(&mut st, &p, 3.0, 48000.0);
        assert_eq!(st.active_voices(), 0);
        // Poly with unison 2 and 8 voices uses at most 16 slots.
        p[VOICES] = 8.0;
        p[UNISON] = 2.0;
        for k in 0..10 {
            st.note_on(&p, 50 + k, 0.8);
        }
        render(&mut st, &p, 0.05, 48000.0);
        assert!(st.active_voices() <= MAX_VOICES);
    }

    #[test]
    fn chorus_widens_the_stereo_image() {
        let mut p = preset("Init");
        p[SPREAD] = 0.0;
        p[VINTAGE] = 0.0;
        let width = |chorus: f32| {
            let mut q = p.clone();
            q[CHORUS] = chorus;
            let mut st = AnalogState::new(48000.0);
            st.note_on(&q, 60, 0.8);
            let (l, r) = render(&mut st, &q, 0.5, 48000.0);
            let side: Vec<f32> = l.iter().zip(&r).map(|(a, b)| a - b).collect();
            rms(&side[12000..]) / rms(&l[12000..])
        };
        assert!(width(0.0) < 1e-3);
        assert!(width(1.0) > 0.05);
    }

    #[test]
    #[ignore]
    fn report_cpu() {
        for name in ["Init", "Unison Saws", "Chorus Pad"] {
            let p = preset(name);
            let mut st = AnalogState::new(48000.0);
            for k in 0..8 {
                st.note_on(&p, 48 + 3 * k, 0.8);
            }
            let t = std::time::Instant::now();
            let _ = render(&mut st, &p, 1.0, 48000.0);
            let el = t.elapsed().as_secs_f32();
            eprintln!("{name}: {} voices, 1 s in {:.1} ms ({:.1}% of a core)", st.active_voices(), el * 1000.0, el * 100.0);
        }
    }
}
