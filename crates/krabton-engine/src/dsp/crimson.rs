//! Crimson: a virtual analog in the manner of the red German synths of the
//! late nineties, written for Krabton (no firmware to bring). Two
//! oscillators: the first plays a classic wave (morphing from a sine through
//! a saw to a pulse), a hypersaw (up to nine saws spread around the note) or
//! a wavetable; the second a classic wave or a wavetable, a few semitones
//! away and detuned, frequency modulated by the first or synced to it. A
//! sub, noise and a ring modulator beside them. Two filters (the first a 24
//! or 12 dB low pass, a high, band pass or notch; the second after it, beside
//! it, or each oscillator its own) with saturation between, a filter and an
//! amplifier envelope, two LFOs, unison (up to eight voices a note, detuned
//! and spread), and the effects it is heard through: chorus, a delay going
//! from side to side, reverb, and a boost of the low end.

use super::fx::DelayLine;
use super::reverb::Reverb;
use super::waves::{TableRead, BANKS};
use super::{db_to_gain, midi_to_freq, pan_gains, Adsr, AdsrCoefs, DcBlock, FilterMode, OnePole, Rng, Svf};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::{FRAC_PI_2, TAU};

pub const OSC1_MODE: usize = 0;
pub const OSC1_SHAPE: usize = 1;
pub const OSC1_PW: usize = 2;
pub const OSC1_TABLE: usize = 3;
pub const DENSITY: usize = 4;
pub const SPREAD: usize = 5;
pub const OSC2_MODE: usize = 6;
pub const OSC2_SHAPE: usize = 7;
pub const OSC2_PW: usize = 8;
pub const OSC2_TABLE: usize = 9;
pub const SEMITONE: usize = 10;
pub const DETUNE: usize = 11;
pub const FM: usize = 12;
pub const SYNC: usize = 13;
pub const BALANCE: usize = 14;
pub const SUB: usize = 15;
pub const NOISE: usize = 16;
pub const RING: usize = 17;
pub const CUTOFF: usize = 18;
pub const RESONANCE: usize = 19;
pub const FILTER_MODE: usize = 20;
pub const ENV_AMT: usize = 21;
pub const KEY_TRACK: usize = 22;
pub const ROUTING: usize = 23;
pub const CUTOFF2: usize = 24;
pub const FILTER2_MODE: usize = 25;
pub const SATURATION: usize = 26;
pub const DRIVE: usize = 27;
pub const F_ATTACK: usize = 28;
pub const F_DECAY: usize = 29;
pub const F_SUSTAIN: usize = 30;
pub const F_RELEASE: usize = 31;
pub const ATTACK: usize = 32;
pub const DECAY: usize = 33;
pub const SUSTAIN: usize = 34;
pub const RELEASE: usize = 35;
pub const LFO1_RATE: usize = 36;
pub const LFO1_SHAPE: usize = 37;
pub const LFO1_PITCH: usize = 38;
pub const LFO1_CUTOFF: usize = 39;
pub const LFO1_TO_SHAPE: usize = 40;
pub const LFO2_RATE: usize = 41;
pub const LFO2_PAN: usize = 42;
pub const LFO2_AMP: usize = 43;
pub const UNISON: usize = 44;
pub const UNI_DETUNE: usize = 45;
pub const UNI_SPREAD: usize = 46;
pub const VOICES: usize = 47;
pub const GLIDE: usize = 48;
pub const OCTAVE: usize = 49;
pub const VEL_AMP: usize = 50;
pub const VEL_CUTOFF: usize = 51;
pub const CHORUS: usize = 52;
pub const DELAY: usize = 53;
pub const DELAY_TIME: usize = 54;
pub const FEEDBACK: usize = 55;
pub const REVERB: usize = 56;
pub const BOOST: usize = 57;
pub const VOLUME: usize = 58;
pub const ENV_PITCH: usize = 59;

pub const OSC1_MODES: &[&str] = &["Classic", "Hypersaw", "Wavetable"];
pub const OSC2_MODES: &[&str] = &["Classic", "Wavetable"];
pub const FILTER_MODES: &[&str] = &["LP 24", "LP 12", "HP", "BP", "Notch"];
pub const FILTER2_MODES: &[&str] = &["LP", "HP", "BP"];
pub const ROUTINGS: &[&str] = &["Single", "Serial", "Parallel", "Split"];
pub const SATURATIONS: &[&str] = &["Off", "Light", "Soft", "Hard", "Fold", "Bits"];
pub const LFO_SHAPES: &[&str] = &["Sine", "Triangle", "Saw", "Square", "S&H"];

/// Voices in all (a note takes as many as its unison).
pub const MAX_VOICES: usize = 32;
const MAX_SAWS: usize = 9;
/// Filters and envelopes move every this many samples.
const CHUNK: usize = 16;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Mode", "Osc 1", OSC1_MODES, 0),
    ParamSpec::lin("Shape", "Osc 1", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Pulse Width", "Osc 1", 0.5, 0.95, 0.5, Unit::Percent),
    ParamSpec::choice("Table", "Osc 1", BANKS, 0),
    ParamSpec::int("Density", "Osc 1", 1.0, 9.0, 7.0, Unit::None),
    ParamSpec::lin("Spread", "Osc 1", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::choice("Mode", "Osc 2", OSC2_MODES, 0),
    ParamSpec::lin("Shape", "Osc 2", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Pulse Width", "Osc 2", 0.5, 0.95, 0.5, Unit::Percent),
    ParamSpec::choice("Table", "Osc 2", BANKS, 0),
    ParamSpec::int("Semitone", "Osc 2", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Detune", "Osc 2", 0.0, 1.0, 0.08, Unit::Percent),
    ParamSpec::lin("FM", "Osc 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Sync", "Osc 2", false),
    ParamSpec::lin("Balance", "Mixer", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Sub", "Mixer", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Noise", "Mixer", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Ring", "Mixer", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Cutoff", "Filter", 20.0, 20_000.0, 2500.0, Unit::Hz),
    ParamSpec::lin("Resonance", "Filter", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::choice("Mode", "Filter", FILTER_MODES, 0),
    ParamSpec::lin("Env Amount", "Filter", -1.0, 1.0, 0.35, Unit::None),
    ParamSpec::lin("Key Track", "Filter", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::choice("Routing", "Filter", ROUTINGS, 0),
    ParamSpec::lin("Cutoff 2", "Filter", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::choice("Mode 2", "Filter", FILTER2_MODES, 0),
    ParamSpec::choice("Saturation", "Filter", SATURATIONS, 0),
    ParamSpec::lin("Drive", "Filter", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::log("Attack", "Filter Env", 0.001, 10.0, 0.005, Unit::Sec),
    ParamSpec::log("Decay", "Filter Env", 0.005, 10.0, 0.4, Unit::Sec),
    ParamSpec::lin("Sustain", "Filter Env", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::log("Release", "Filter Env", 0.005, 10.0, 0.4, Unit::Sec),
    ParamSpec::log("Attack", "Amp Env", 0.001, 10.0, 0.003, Unit::Sec),
    ParamSpec::log("Decay", "Amp Env", 0.005, 10.0, 0.5, Unit::Sec),
    ParamSpec::lin("Sustain", "Amp Env", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::log("Release", "Amp Env", 0.005, 10.0, 0.3, Unit::Sec),
    ParamSpec::log("Rate", "LFO 1", 0.05, 20.0, 2.0, Unit::Hz),
    ParamSpec::choice("Shape", "LFO 1", LFO_SHAPES, 0),
    ParamSpec::lin("To Pitch", "LFO 1", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("To Cutoff", "LFO 1", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("To Shape", "LFO 1", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Rate", "LFO 2", 0.05, 20.0, 0.3, Unit::Hz),
    ParamSpec::lin("To Pan", "LFO 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("To Amp", "LFO 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Unison", "Unison", 1.0, 8.0, 1.0, Unit::None),
    ParamSpec::lin("Detune", "Unison", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Spread", "Unison", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::int("Voices", "Play", 1.0, 16.0, 8.0, Unit::None),
    ParamSpec::log("Glide", "Play", 0.001, 2.0, 0.001, Unit::Sec),
    ParamSpec::int("Octave", "Play", -2.0, 2.0, 0.0, Unit::None),
    ParamSpec::lin("Velocity", "Play", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Vel > Cutoff", "Play", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Chorus", "Effects", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Delay", "Effects", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Delay Time", "Effects", 0.05, 1.5, 0.375, Unit::Sec),
    ParamSpec::lin("Feedback", "Effects", 0.0, 0.95, 0.35, Unit::Percent),
    ParamSpec::lin("Reverb", "Effects", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Boost", "Output", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Volume", "Output", -30.0, 6.0, -8.0, Unit::Db),
    ParamSpec::lin("Env > Pitch", "Osc 2", -1.0, 1.0, 0.0, Unit::None),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Init",
        "Trance Lead",
        "Hyper Pad",
        "Acid Bass",
        "Reese Bass",
        "Hyper Pluck",
        "Sync Lead",
        "Wave Pad",
        "Digital Bells",
        "Wobble Bass",
        "Fat Brass",
        "Dark Choir",
        "Rave Stab",
        "Sub Bass",
        "Bit Lead",
    ]
}

fn bank(name: &str) -> f32 {
    BANKS.iter().position(|&b| b == name).unwrap_or(0) as f32
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: Vec<(usize, f32)> = match name {
        "Trance Lead" => vec![(OSC1_MODE, 1.0), (DENSITY, 9.0), (SPREAD, 0.45), (OSC1_SHAPE, 0.8), (OSC2_SHAPE, 0.5), (SEMITONE, 12.0), (DETUNE, 0.12), (BALANCE, -0.4), (UNISON, 2.0), (UNI_DETUNE, 0.2), (CUTOFF, 6500.0), (RESONANCE, 0.15), (ENV_AMT, 0.25), (F_DECAY, 0.5), (F_SUSTAIN, 0.4), (SUSTAIN, 1.0), (RELEASE, 0.35), (GLIDE, 0.02), (LFO1_RATE, 5.5), (LFO1_PITCH, 0.08), (CHORUS, 0.2), (DELAY, 0.3), (DELAY_TIME, 0.375), (FEEDBACK, 0.4), (REVERB, 0.25)],
        "Hyper Pad" => vec![(OSC1_MODE, 1.0), (DENSITY, 9.0), (SPREAD, 0.6), (OSC1_SHAPE, 0.9), (BALANCE, -1.0), (UNISON, 2.0), (UNI_DETUNE, 0.3), (FILTER_MODE, 1.0), (CUTOFF, 2800.0), (RESONANCE, 0.1), (ENV_AMT, 0.2), (F_ATTACK, 1.2), (F_DECAY, 2.0), (F_SUSTAIN, 0.6), (F_RELEASE, 2.0), (ATTACK, 0.7), (DECAY, 1.0), (SUSTAIN, 0.9), (RELEASE, 2.0), (LFO1_RATE, 0.15), (LFO1_CUTOFF, 0.15), (CHORUS, 0.45), (REVERB, 0.5)],
        "Acid Bass" => vec![(OSC1_SHAPE, 0.5), (BALANCE, -1.0), (CUTOFF, 320.0), (RESONANCE, 0.78), (ENV_AMT, 0.75), (KEY_TRACK, 0.4), (F_DECAY, 0.22), (F_SUSTAIN, 0.0), (F_RELEASE, 0.15), (DECAY, 0.4), (SUSTAIN, 0.7), (RELEASE, 0.08), (SATURATION, 2.0), (DRIVE, 0.5), (VOICES, 1.0), (GLIDE, 0.06), (OCTAVE, -1.0), (VEL_CUTOFF, 0.5), (DELAY, 0.15), (DELAY_TIME, 0.28), (FEEDBACK, 0.3)],
        "Reese Bass" => vec![(OSC1_SHAPE, 0.5), (OSC2_SHAPE, 0.5), (DETUNE, 0.3), (UNISON, 2.0), (UNI_DETUNE, 0.35), (UNI_SPREAD, 0.4), (CUTOFF, 650.0), (RESONANCE, 0.2), (ENV_AMT, 0.1), (SATURATION, 1.0), (DRIVE, 0.6), (SUSTAIN, 1.0), (RELEASE, 0.2), (VOICES, 1.0), (GLIDE, 0.03), (OCTAVE, -1.0), (LFO1_RATE, 0.3), (LFO1_CUTOFF, 0.12), (BOOST, 0.4)],
        "Hyper Pluck" => vec![(OSC1_MODE, 1.0), (DENSITY, 5.0), (SPREAD, 0.3), (OSC1_SHAPE, 0.7), (BALANCE, -1.0), (CUTOFF, 380.0), (RESONANCE, 0.25), (ENV_AMT, 0.65), (F_DECAY, 0.25), (F_SUSTAIN, 0.0), (DECAY, 0.45), (SUSTAIN, 0.0), (RELEASE, 0.3), (DELAY, 0.3), (DELAY_TIME, 0.375), (FEEDBACK, 0.35), (REVERB, 0.2)],
        "Sync Lead" => vec![(OSC1_SHAPE, 0.5), (OSC2_SHAPE, 0.5), (SYNC, 1.0), (SEMITONE, 7.0), (ENV_PITCH, 0.45), (BALANCE, 0.6), (CUTOFF, 5000.0), (RESONANCE, 0.2), (ENV_AMT, 0.2), (F_DECAY, 0.6), (F_SUSTAIN, 0.2), (SUSTAIN, 1.0), (VOICES, 1.0), (GLIDE, 0.04), (LFO1_RATE, 5.0), (LFO1_PITCH, 0.06), (DELAY, 0.25), (REVERB, 0.15)],
        "Wave Pad" => vec![(OSC1_MODE, 2.0), (OSC1_TABLE, bank("Vocal")), (OSC1_SHAPE, 0.3), (OSC2_MODE, 1.0), (OSC2_TABLE, bank("Strings")), (OSC2_SHAPE, 0.6), (DETUNE, 0.15), (LFO1_RATE, 0.18), (LFO1_TO_SHAPE, 0.45), (FILTER_MODE, 1.0), (CUTOFF, 3500.0), (ENV_AMT, 0.1), (ATTACK, 0.6), (DECAY, 1.0), (SUSTAIN, 0.85), (RELEASE, 1.8), (UNISON, 2.0), (CHORUS, 0.3), (REVERB, 0.55)],
        "Digital Bells" => vec![(OSC1_MODE, 2.0), (OSC1_TABLE, bank("Bells")), (OSC1_SHAPE, 0.4), (OSC2_SHAPE, 0.0), (SEMITONE, 19.0), (FM, 0.35), (BALANCE, -0.3), (CUTOFF, 9000.0), (ENV_AMT, 0.2), (F_DECAY, 0.8), (F_SUSTAIN, 0.1), (DECAY, 1.6), (SUSTAIN, 0.0), (RELEASE, 1.2), (REVERB, 0.4), (DELAY, 0.15)],
        "Wobble Bass" => vec![(OSC1_SHAPE, 0.6), (OSC2_SHAPE, 0.9), (SEMITONE, -12.0), (BALANCE, -0.2), (CUTOFF, 380.0), (RESONANCE, 0.45), (ENV_AMT, 0.1), (LFO1_RATE, 2.6), (LFO1_SHAPE, 1.0), (LFO1_CUTOFF, 0.7), (SATURATION, 2.0), (DRIVE, 0.55), (SUSTAIN, 1.0), (RELEASE, 0.1), (VOICES, 1.0), (OCTAVE, -1.0), (BOOST, 0.35)],
        "Fat Brass" => vec![(OSC1_SHAPE, 0.5), (OSC2_SHAPE, 0.5), (DETUNE, 0.14), (UNISON, 2.0), (UNI_DETUNE, 0.2), (CUTOFF, 1100.0), (RESONANCE, 0.1), (ENV_AMT, 0.5), (F_ATTACK, 0.08), (F_DECAY, 0.6), (F_SUSTAIN, 0.4), (ATTACK, 0.03), (SUSTAIN, 0.9), (RELEASE, 0.25), (CHORUS, 0.25), (REVERB, 0.2)],
        "Dark Choir" => vec![(OSC1_MODE, 2.0), (OSC1_TABLE, bank("Vocal")), (OSC1_SHAPE, 0.65), (OSC2_MODE, 1.0), (OSC2_TABLE, bank("Vocal")), (OSC2_SHAPE, 0.2), (DETUNE, 0.2), (FILTER_MODE, 1.0), (CUTOFF, 1800.0), (ENV_AMT, 0.05), (ATTACK, 0.5), (SUSTAIN, 1.0), (RELEASE, 1.5), (LFO1_RATE, 0.25), (LFO1_TO_SHAPE, 0.25), (UNISON, 3.0), (UNI_DETUNE, 0.18), (CHORUS, 0.5), (REVERB, 0.6)],
        "Rave Stab" => vec![(OSC1_MODE, 1.0), (DENSITY, 7.0), (SPREAD, 0.6), (OSC1_SHAPE, 0.8), (OSC2_SHAPE, 0.9), (SEMITONE, 7.0), (BALANCE, -0.2), (CUTOFF, 1600.0), (RESONANCE, 0.3), (ENV_AMT, 0.5), (F_DECAY, 0.3), (F_SUSTAIN, 0.1), (DECAY, 0.35), (SUSTAIN, 0.0), (RELEASE, 0.25), (REVERB, 0.35), (DELAY, 0.2), (DELAY_TIME, 0.28)],
        "Sub Bass" => vec![(OSC1_SHAPE, 0.0), (BALANCE, -1.0), (SUB, 0.5), (CUTOFF, 300.0), (ENV_AMT, 0.05), (SUSTAIN, 1.0), (RELEASE, 0.12), (VOICES, 1.0), (OCTAVE, -1.0), (BOOST, 0.3)],
        "Bit Lead" => vec![(OSC1_SHAPE, 1.0), (OSC1_PW, 0.7), (BALANCE, -1.0), (SATURATION, 5.0), (DRIVE, 0.6), (CUTOFF, 7000.0), (ENV_AMT, 0.15), (SUSTAIN, 1.0), (VOICES, 1.0), (GLIDE, 0.03), (LFO1_RATE, 6.0), (LFO1_TO_SHAPE, 0.3), (DELAY, 0.3), (DELAY_TIME, 0.33)],
        _ => Vec::new(),
    };
    for (i, v) in set {
        p[i] = v;
    }
    p
}

/// A saw's or a pulse's edges smoothed for a step `dt` (PolyBLEP).
#[inline]
fn blep(t: f32, dt: f32) -> f32 {
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

/// The classic wave at `ph`: a sine (shape 0) through a saw (0.5) to a
/// pulse of width `pw` (1).
#[inline]
fn classic(ph: f32, dt: f32, shape: f32, pw: f32) -> f32 {
    let saw = 2.0 * ph - 1.0 - blep(ph, dt);
    if shape <= 0.5 {
        let sine = (TAU * ph).sin();
        sine + (saw - sine) * shape * 2.0
    } else {
        let naive = if ph < pw { 1.0 } else { -1.0 };
        let pulse = naive + blep(ph, dt) - blep((ph + 1.0 - pw).fract(), dt);
        saw + (pulse - saw) * (shape - 0.5) * 2.0
    }
}

/// Oscillator `osc` (1 or 2) as the face draws it: one cycle, at `ph`.
pub fn wave_at(p: &[f32], osc: usize, ph: f32) -> f32 {
    let get = |i: usize| p.get(i).copied().unwrap_or(0.0);
    let (mode, shape, pw, table) = if osc == 1 {
        (get(OSC1_MODE).round().max(0.0) as usize, get(OSC1_SHAPE), get(OSC1_PW), get(OSC1_TABLE))
    } else {
        (if get(OSC2_MODE) >= 0.5 { 2 } else { 0 }, get(OSC2_SHAPE), get(OSC2_PW), get(OSC2_TABLE))
    };
    let shape = shape.clamp(0.0, 1.0);
    match mode {
        1 => {
            // The saws drawn a little apart, as they drift.
            let d = (get(DENSITY).round() as usize).clamp(1, MAX_SAWS);
            let spread = get(SPREAD).clamp(0.0, 1.0);
            let (mut sum, mut norm) = (0.0, 0.0);
            for k in 0..d {
                let c = if d == 1 { 0.0 } else { -1.0 + 2.0 * k as f32 / (d - 1) as f32 };
                let level = if c.abs() < 1.0 / (d.max(2) - 1) as f32 + 1e-3 { 1.0 - 0.4 * shape } else { shape };
                sum += (2.0 * (ph + c * spread * 0.3).rem_euclid(1.0) - 1.0) * level;
                norm += level;
            }
            sum / norm.max(1e-3)
        }
        2 => {
            let x = shape * 7.0;
            let base = (table.round().max(0.0) as usize).min(BANKS.len() - 1) * 8;
            let i = x.floor().min(6.0);
            let (a, b) = (super::waves::wave(base + i as usize), super::waves::wave(base + i as usize + 1));
            let at = |w: &[f32]| w[((ph.rem_euclid(1.0) * w.len() as f32) as usize).min(w.len() - 1)];
            at(a) + (at(b) - at(a)) * (x - i).min(1.0)
        }
        _ => classic(ph.rem_euclid(1.0), 0.0, shape, pw.clamp(0.5, 0.97)),
    }
}

/// Saturation of one kind, as hard as `drive`.
#[inline]
fn saturate(x: f32, kind: usize, drive: f32) -> f32 {
    match kind {
        1 => {
            let g = 1.0 + drive * 2.0;
            (x * g).tanh() / g.tanh()
        }
        2 => {
            let g = 1.0 + drive * 6.0;
            let y = x * g;
            y / (1.0 + y.abs()) * (1.0 + g) / g
        }
        3 => (x * (1.0 + drive * 8.0)).clamp(-1.0, 1.0),
        4 => (FRAC_PI_2 * x * (1.0 + drive * 5.0)).sin(),
        5 => {
            let steps = 2f32.powf(12.0 - drive * 10.0);
            (x.clamp(-1.0, 1.0) * steps).round() / steps
        }
        _ => x,
    }
}

/// An LFO of a shape at `ph` (0..1), held value `held` for S&H.
#[inline]
fn lfo(shape: usize, ph: f32, held: f32) -> f32 {
    match shape {
        1 => 1.0 - 4.0 * (ph - 0.5).abs(),
        2 => 2.0 * ph - 1.0,
        3 => {
            if ph < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        4 => held,
        _ => (TAU * ph).sin(),
    }
}

#[derive(Clone)]
struct Voice {
    note: u8,
    gate: bool,
    vel: f32,
    age: u64,
    pitch: f32,
    target: f32,
    /// Unison: semitones off the note, and where in the stereo field.
    detune: f32,
    pan: f32,
    phase1: f32,
    saws: [f32; MAX_SAWS],
    sub: f32,
    phase2: f32,
    amp: Adsr,
    fenv: Adsr,
    f1a: Svf,
    f1b: Svf,
    f2: Svf,
    dc: DcBlock,
}

impl Voice {
    fn new() -> Self {
        Self {
            note: 0,
            gate: false,
            vel: 0.0,
            age: 0,
            pitch: 60.0,
            target: 60.0,
            detune: 0.0,
            pan: 0.0,
            phase1: 0.0,
            saws: [0.0; MAX_SAWS],
            sub: 0.0,
            phase2: 0.0,
            amp: Adsr::new(),
            fenv: Adsr::new(),
            f1a: Svf::new(),
            f1b: Svf::new(),
            f2: Svf::new(),
            dc: DcBlock::default(),
        }
    }

    fn active(&self) -> bool {
        self.amp.is_active()
    }
}

pub struct CrimsonState {
    pub pitch_bend: f32,
    sr: f32,
    voices: Vec<Voice>,
    held: Vec<(u8, f32)>,
    counter: u64,
    last_pitch: Option<f32>,
    rng: Rng,
    lfo1: f32,
    lfo2: f32,
    held1: f32,
    chorus: [DelayLine; 2],
    chorus_ph: f32,
    delay: [DelayLine; 2],
    delay_lp: [OnePole; 2],
    reverb: Reverb,
    boost: [OnePole; 2],
    bl: Vec<f32>,
    br: Vec<f32>,
    /// For the face: the filter envelope and how far the cutoff is open.
    shown: (f32, f32),
}

impl CrimsonState {
    pub fn new(sr: f32) -> Self {
        super::waves::warm();
        let line = |secs: f32| DelayLine::new((sr * secs) as usize + 8);
        Self {
            pitch_bend: 0.0,
            sr,
            voices: (0..MAX_VOICES).map(|_| Voice::new()).collect(),
            held: Vec::with_capacity(32),
            counter: 0,
            last_pitch: None,
            rng: Rng::new(0xC21_5011),
            lfo1: 0.0,
            lfo2: 0.0,
            held1: 0.0,
            chorus: [line(0.05), line(0.05)],
            chorus_ph: 0.0,
            delay: [line(1.6), line(1.6)],
            delay_lp: [OnePole::default(); 2],
            reverb: Reverb::new(sr),
            boost: [OnePole::default(); 2],
            bl: vec![0.0; 4096],
            br: vec![0.0; 4096],
            shown: (0.0, 0.0),
        }
    }

    fn pitch_of(p: &[f32], note: u8) -> f32 {
        note as f32 + 12.0 * p[OCTAVE].round()
    }

    fn unison(p: &[f32]) -> usize {
        (p[UNISON].round() as usize).clamp(1, 8)
    }

    /// Starts voice `i` as unison voice `u` of `n` for a note.
    #[allow(clippy::too_many_arguments)]
    fn start(&mut self, p: &[f32], i: usize, u: usize, n: usize, note: u8, pitch: f32, from: Option<f32>, vel: f32, retrigger: bool) {
        let c = if n == 1 { 0.0 } else { -1.0 + 2.0 * u as f32 / (n - 1) as f32 };
        let detune = c * p[UNI_DETUNE].clamp(0.0, 1.0).powf(1.5) * 0.5;
        let pan = c * p[UNI_SPREAD].clamp(0.0, 1.0);
        let fresh = !self.voices[i].active();
        let mut phases = [0.0f32; MAX_SAWS + 2];
        for ph in phases.iter_mut() {
            *ph = self.rng.unipolar();
        }
        let v = &mut self.voices[i];
        v.note = note;
        v.gate = true;
        v.vel = vel;
        v.age = self.counter;
        v.target = pitch;
        v.pitch = from.unwrap_or(pitch);
        v.detune = detune;
        v.pan = pan;
        if fresh {
            // Free running: every voice and saw from a phase of its own.
            v.phase1 = phases[0];
            v.phase2 = phases[1];
            v.saws.copy_from_slice(&phases[2..]);
            v.f1a.reset();
            v.f1b.reset();
            v.f2.reset();
        }
        if retrigger || fresh {
            v.amp.trigger();
            v.fenv.trigger();
        }
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let pitch = Self::pitch_of(p, note);
        let glide = p[GLIDE] > 0.002;
        let poly = (p[VOICES].round() as usize).clamp(1, 16);
        let uni = Self::unison(p);
        self.counter += 1;
        let legato = poly == 1 && !self.held.is_empty();
        self.held.retain(|h| h.0 != note);
        self.held.push((note, vel));
        if poly == 1 {
            // Mono: the first voices; held into the next note (legato), gliding there.
            let from = if glide && self.voices[0].active() { Some(self.voices[0].pitch) } else { None };
            for u in 0..uni {
                self.start(p, u, u, uni, note, pitch, from, vel, !legato);
            }
            for v in self.voices[uni..].iter_mut().filter(|v| v.gate) {
                v.gate = false;
                v.amp.release();
                v.fenv.release();
            }
            self.last_pitch = Some(pitch);
            return;
        }
        // At most `poly` notes held: the oldest lets go.
        let mut notes: Vec<(u64, u8)> = Vec::with_capacity(MAX_VOICES);
        for v in self.voices.iter().filter(|v| v.gate && v.note != note) {
            if !notes.iter().any(|n| n.1 == v.note) {
                notes.push((v.age, v.note));
            }
        }
        notes.sort_unstable();
        while notes.len() >= poly {
            let (_, old) = notes.remove(0);
            for v in self.voices.iter_mut().filter(|v| v.gate && v.note == old) {
                v.gate = false;
                v.amp.release();
                v.fenv.release();
            }
        }
        // Its voices: the note's own again, free ones, the oldest let go, the oldest.
        let mut order: Vec<(u8, u64, usize)> = self
            .voices
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let rank = if v.gate && v.note == note {
                    0
                } else if !v.active() {
                    1
                } else if !v.gate {
                    2
                } else {
                    3
                };
                (rank, v.age, i)
            })
            .collect();
        order.sort_unstable();
        let from = if glide { self.last_pitch } else { None };
        let picks: Vec<usize> = order.iter().take(uni.min(MAX_VOICES)).map(|o| o.2).collect();
        for (u, &i) in picks.iter().enumerate() {
            self.start(p, i, u, uni, note, pitch, from, vel, true);
        }
        self.last_pitch = Some(pitch);
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.held.retain(|h| h.0 != note);
        let poly = p.get(VOICES).map(|v| (v.round() as usize).clamp(1, 16)).unwrap_or(8);
        if poly == 1 && self.voices[0].note == note {
            // Back to a key still held, gliding there.
            if let Some(&(prev, vel)) = self.held.last() {
                let pitch = Self::pitch_of(p, prev);
                for v in self.voices.iter_mut().filter(|v| v.gate) {
                    v.note = prev;
                    v.target = pitch;
                    v.vel = vel;
                }
                return;
            }
        }
        for v in self.voices.iter_mut().filter(|v| v.gate && v.note == note) {
            v.gate = false;
            v.amp.release();
            v.fenv.release();
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in self.voices.iter_mut() {
            v.gate = false;
            v.amp.release();
            v.fenv.release();
        }
    }

    /// For the face: voices sounding, the filter envelope, the cutoff (0..1),
    /// the first LFO (0..1).
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.voices.iter().filter(|v| v.active()).count() as f32;
        out[1] = self.shown.0;
        out[2] = self.shown.1;
        out[3] = self.lfo1;
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        if self.bl.len() < n {
            self.bl.resize(n, 0.0);
            self.br.resize(n, 0.0);
        }
        self.bl[..n].fill(0.0);
        self.br[..n].fill(0.0);
        let sr = self.sr;
        let pick = |i: usize, k: usize| (p[i].round().max(0.0) as usize).min(k - 1);
        let mode1 = pick(OSC1_MODE, OSC1_MODES.len());
        let mode2 = pick(OSC2_MODE, OSC2_MODES.len());
        let table1 = pick(OSC1_TABLE, BANKS.len());
        let table2 = pick(OSC2_TABLE, BANKS.len());
        let fmode = pick(FILTER_MODE, FILTER_MODES.len());
        let f2mode = match pick(FILTER2_MODE, FILTER2_MODES.len()) {
            1 => FilterMode::High,
            2 => FilterMode::Band,
            _ => FilterMode::Low,
        };
        let routing = pick(ROUTING, ROUTINGS.len());
        let sat = pick(SATURATION, SATURATIONS.len());
        let drive = p[DRIVE].clamp(0.0, 1.0);
        let lfo1_shape = pick(LFO1_SHAPE, LFO_SHAPES.len());
        let density = (p[DENSITY].round() as usize).clamp(1, MAX_SAWS);
        let sync = p[SYNC] >= 0.5;
        let amp_c = AdsrCoefs::new(p[ATTACK], p[DECAY], p[SUSTAIN], p[RELEASE], sr);
        let fenv_c = AdsrCoefs::new(p[F_ATTACK], p[F_DECAY], p[F_SUSTAIN], p[F_RELEASE], sr / CHUNK as f32);
        let glide = if p[GLIDE] > 0.002 { 1.0 - (-(CHUNK as f32) / (p[GLIDE] * 0.3 * sr)).exp() } else { 1.0 };
        let gain = db_to_gain(p[VOLUME]) * 0.35;
        let g1 = (1.0 - p[BALANCE]).min(1.0);
        let g2 = (1.0 + p[BALANCE]).min(1.0);
        let res = p[RESONANCE].clamp(0.0, 1.0);
        // The hypersaw: saws spread around the note, the side ones as loud as Shape.
        let spread = p[SPREAD].clamp(0.0, 1.0).powf(1.6) * 1.5;
        let mut ratios = [1.0f32; MAX_SAWS];
        let mut levels = [0.0f32; MAX_SAWS];
        // The middle one (or two) as loud as can be, the others as Shape sets them.
        let middle = if density == 1 { 1.0 } else { 1.0 / (density - 1) as f32 + 1e-3 };
        for k in 0..density {
            let c = if density == 1 { 0.0 } else { -1.0 + 2.0 * k as f32 / (density - 1) as f32 };
            ratios[k] = 2f32.powf(c * spread / 12.0);
            levels[k] = if c.abs() < middle { 1.0 - 0.4 * p[OSC1_SHAPE] } else { p[OSC1_SHAPE] };
        }
        let norm = 1.0 / levels[..density].iter().map(|a| a * a).sum::<f32>().sqrt().max(0.3);
        let mut shown = (0.0f32, 0.0f32);
        let lfo1_inc = p[LFO1_RATE] / sr;
        let lfo2_inc = p[LFO2_RATE] / sr;
        let mut at = 0;
        while at < n {
            let len = CHUNK.min(n - at);
            // The LFOs, once a chunk.
            let before = self.lfo1;
            self.lfo1 = (self.lfo1 + lfo1_inc * len as f32).fract();
            if self.lfo1 < before {
                self.held1 = self.rng.bipolar();
            }
            self.lfo2 = (self.lfo2 + lfo2_inc * len as f32).fract();
            let l1 = lfo(lfo1_shape, self.lfo1, self.held1);
            let l2 = (TAU * self.lfo2).sin();
            let shape_mod = l1 * p[LFO1_TO_SHAPE] * 0.5;
            let shape1 = (p[OSC1_SHAPE] + shape_mod).clamp(0.0, 1.0);
            let shape2 = (p[OSC2_SHAPE] + shape_mod).clamp(0.0, 1.0);
            let pw1 = (p[OSC1_PW] + shape_mod * 0.4).clamp(0.5, 0.97);
            let pw2 = (p[OSC2_PW] + shape_mod * 0.4).clamp(0.5, 0.97);
            let vib = l1 * p[LFO1_PITCH] * p[LFO1_PITCH] * 12.0;
            let trem = 1.0 - p[LFO2_AMP] * (0.5 + 0.5 * l2);
            for v in self.voices.iter_mut() {
                if !v.active() {
                    continue;
                }
                let fe = v.fenv.next(&fenv_c);
                v.pitch += (v.target - v.pitch) * glide;
                let pitch = v.pitch + self.pitch_bend + v.detune + vib;
                let f1 = midi_to_freq(pitch);
                let f2 = midi_to_freq(pitch + p[SEMITONE].round() + p[DETUNE] * 0.5 + p[ENV_PITCH] * fe * 24.0);
                let (dt1, dt2) = ((f1 / sr).min(0.45), (f2 / sr).min(0.45));
                let vel = v.vel.clamp(0.0, 1.0);
                let octaves = p[ENV_AMT] * fe * 7.0 + p[KEY_TRACK] * (v.pitch - 60.0) / 12.0 + l1 * p[LFO1_CUTOFF] * 3.0 + p[VEL_CUTOFF] * (vel - 0.6) * 3.0;
                let cutoff = (p[CUTOFF] * 2f32.powf(octaves)).clamp(20.0, sr * 0.45);
                let cutoff2 = (cutoff * 2f32.powf(p[CUTOFF2] * 4.0)).clamp(20.0, sr * 0.45);
                v.f1a.set(cutoff, res * 0.3, sr);
                v.f1b.set(cutoff, res, sr);
                v.f2.set(cutoff2, res * 0.8, sr);
                let wt1 = (mode1 == 2).then(|| {
                    let x = (shape1 * 7.0).min(7.0);
                    TableRead::new(table1 * 8 + x.floor().min(6.0) as usize, x - x.floor().min(6.0), f1, sr)
                });
                let wt2 = (mode2 == 1).then(|| {
                    let x = (shape2 * 7.0).min(7.0);
                    TableRead::new(table2 * 8 + x.floor().min(6.0) as usize, x - x.floor().min(6.0), f2, sr)
                });
                let strength = 1.0 - p[VEL_AMP] * (1.0 - vel);
                let (pl, pr) = pan_gains(v.pan + l2 * p[LFO2_PAN]);
                if fe > shown.0 {
                    shown = (fe, (cutoff / 20.0).log2() / 10.0);
                }
                for i in at..at + len {
                    // Osc 1: classic, hypersaw or wavetable.
                    let wrapped = v.phase1 + dt1 >= 1.0;
                    v.phase1 = (v.phase1 + dt1).fract();
                    let o1 = match mode1 {
                        1 => {
                            let mut s = 0.0;
                            for k in 0..density {
                                let d = dt1 * ratios[k];
                                v.saws[k] = (v.saws[k] + d).fract();
                                s += (2.0 * v.saws[k] - 1.0 - blep(v.saws[k], d)) * levels[k];
                            }
                            s * norm
                        }
                        2 => wt1.as_ref().map(|t| t.read(v.phase1)).unwrap_or(0.0),
                        _ => classic(v.phase1, dt1, shape1, pw1),
                    };
                    // The sub, an octave under.
                    v.sub = (v.sub + dt1 * 0.5).fract();
                    let sub = if v.sub < 0.5 { 1.0 } else { -1.0 } + blep(v.sub, dt1 * 0.5) - blep((v.sub + 0.5).fract(), dt1 * 0.5);
                    // Osc 2: synced to the first, frequency modulated by it.
                    if sync && wrapped {
                        v.phase2 = v.phase1 * dt2 / dt1.max(1e-6);
                    } else {
                        v.phase2 = (v.phase2 + dt2).fract();
                    }
                    let ph2 = (v.phase2 + p[FM] * 1.5 * o1).rem_euclid(1.0);
                    let o2 = match &wt2 {
                        Some(t) => t.read(ph2),
                        None => classic(ph2, dt2, shape2, pw2),
                    };
                    let noise = self.rng.bipolar();
                    let a = o1 * g1 + sub * p[SUB] * 0.8;
                    let b = o2 * g2 + noise * p[NOISE] * 0.6 + o1 * o2 * p[RING];
                    // The filters as routed, saturation after the first.
                    let first = |v: &mut Voice, x: f32| match fmode {
                        0 => v.f1b.process(v.f1a.process(x, FilterMode::Low), FilterMode::Low),
                        1 => v.f1b.process(x, FilterMode::Low),
                        2 => v.f1b.process(x, FilterMode::High),
                        3 => v.f1b.process(x, FilterMode::Band),
                        _ => v.f1b.process(x, FilterMode::Notch),
                    };
                    let y = match routing {
                        1 => {
                            let y = saturate(first(v, a + b), sat, drive);
                            v.f2.process(y, f2mode)
                        }
                        2 => {
                            let x = a + b;
                            saturate(first(v, x) * 0.7 + v.f2.process(x, f2mode) * 0.7, sat, drive)
                        }
                        3 => saturate(first(v, a) + v.f2.process(b, f2mode), sat, drive),
                        _ => saturate(first(v, a + b), sat, drive),
                    };
                    let y = v.dc.tick(y);
                    let env = v.amp.next(&amp_c);
                    let out = y * env * strength * trem * gain;
                    self.bl[i] += out * pl;
                    self.br[i] += out * pr;
                }
            }
            at += len;
        }
        self.shown = shown;
        self.effects(p, n);
        for i in 0..n {
            l[i] += self.bl[i];
            r[i] += self.br[i];
        }
    }

    /// Chorus, delay, reverb and boost on what the voices played.
    fn effects(&mut self, p: &[f32], n: usize) {
        let sr = self.sr;
        let (bl, br) = (&mut self.bl[..n], &mut self.br[..n]);
        let chorus = p[CHORUS].clamp(0.0, 1.0);
        if chorus > 0.0 {
            let inc = 0.45 / sr;
            for i in 0..n {
                self.chorus_ph = (self.chorus_ph + inc).fract();
                for (c, x) in [&mut bl[i], &mut br[i]].into_iter().enumerate() {
                    let ph = self.chorus_ph + c as f32 * 0.25;
                    let d = (0.011 + 0.004 * (TAU * ph).sin()) * sr;
                    self.chorus[c].write(*x);
                    let wet = self.chorus[c].read(d);
                    *x = *x * (1.0 - chorus * 0.4) + wet * chorus * 0.8;
                }
            }
        }
        let delay = p[DELAY].clamp(0.0, 1.0);
        if delay > 0.0 {
            let t = (p[DELAY_TIME].clamp(0.05, 1.5) * sr).max(2.0);
            let fb = p[FEEDBACK].clamp(0.0, 0.95);
            for lp in self.delay_lp.iter_mut() {
                lp.set(5000.0, sr);
            }
            for i in 0..n {
                // Going from side to side: each side feeds the other.
                let (dl, dr) = (self.delay[0].read(t), self.delay[1].read(t));
                let to_l = self.delay_lp[0].lp(bl[i] * 0.5 + br[i] * 0.5 + dr * fb);
                let to_r = self.delay_lp[1].lp(dl * fb);
                self.delay[0].write(to_l);
                self.delay[1].write(to_r);
                bl[i] += dl * delay;
                br[i] += dr * delay;
            }
        }
        let reverb = p[REVERB].clamp(0.0, 1.0);
        if reverb > 0.0 {
            self.reverb.process(0.75, 0.45, 1.0, 12.0, 150.0, reverb * 0.6, bl, br);
        }
        let boost = p[BOOST].clamp(0.0, 1.0);
        if boost > 0.0 {
            for b in self.boost.iter_mut() {
                b.set(140.0, sr);
            }
            for i in 0..n {
                bl[i] += self.boost[0].lp(bl[i]) * boost * 1.5;
                br[i] += self.boost[1].lp(br[i]) * boost * 1.5;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    /// Plays `notes` (all at once) for `hold` seconds of `secs`: left and right.
    fn play(p: &[f32], notes: &[u8], secs: f32, hold: f32) -> (Vec<f32>, Vec<f32>, CrimsonState) {
        let mut st = CrimsonState::new(SR);
        for &k in notes {
            st.note_on(p, k, 0.9);
        }
        let (mut ol, mut or) = (Vec::new(), Vec::new());
        let blocks = (secs * SR / 256.0) as usize;
        let mut released = false;
        for b in 0..blocks {
            if !released && b as f32 * 256.0 / SR >= hold {
                for &k in notes {
                    st.note_off(p, k);
                }
                released = true;
            }
            let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
            st.render(p, &mut l, &mut r);
            ol.extend(l);
            or.extend(r);
        }
        (ol, or, st)
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn crossings(x: &[f32]) -> usize {
        x.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count()
    }

    #[test]
    fn every_preset_sounds_and_stays_in_bounds() {
        for name in preset_names() {
            let (l, r, _) = play(&preset(name), &[57, 64], 1.2, 0.6);
            assert!(rms(&l) > 1e-3, "{name} sounds");
            assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 2.5), "{name} stays in bounds");
        }
    }

    #[test]
    fn the_hypersaw_beats_where_one_saw_does_not() {
        // Its saws drift against each other: the loudness comes and goes.
        let wobble = |density: f32| {
            let mut p = preset("Init");
            (p[OSC1_MODE], p[DENSITY], p[SPREAD], p[OSC1_SHAPE], p[BALANCE], p[CUTOFF], p[ENV_AMT], p[SUSTAIN]) = (1.0, density, 0.8, 1.0, -1.0, 20_000.0, 0.0, 1.0);
            let (l, _, _) = play(&p, &[45], 1.0, 1.0);
            let parts: Vec<f32> = l[4800..].chunks(2400).map(rms).collect();
            let (lo, hi) = parts.iter().fold((f32::MAX, 0f32), |(a, b), &x| (a.min(x), b.max(x)));
            hi / lo.max(1e-6)
        };
        assert!(wobble(9.0) > wobble(1.0) * 1.05, "{} {}", wobble(9.0), wobble(1.0));
    }

    #[test]
    fn unison_spreads_the_voices_across_the_sides() {
        let mut p = preset("Init");
        p[SUSTAIN] = 1.0;
        let (l, r, _) = play(&p, &[60], 0.5, 0.5);
        let diff: f32 = l.iter().zip(&r).map(|(a, b)| (a - b).abs()).sum();
        assert!(diff < 1e-3, "one voice: the middle");
        (p[UNISON], p[UNI_SPREAD], p[UNI_DETUNE]) = (4.0, 1.0, 0.5);
        let (l, r, st) = play(&p, &[60], 0.5, 0.5);
        let diff: f32 = l.iter().zip(&r).map(|(a, b)| (a - b).abs()).sum();
        assert!(diff > 1.0, "four, spread: {diff}");
        assert_eq!(st.voices.iter().filter(|v| v.active()).count(), 4);
    }

    #[test]
    fn the_filter_envelope_opens_and_closes_the_cutoff() {
        let mut p = preset("Init");
        (p[OSC1_SHAPE], p[CUTOFF], p[RESONANCE], p[ENV_AMT], p[F_ATTACK], p[F_DECAY], p[F_SUSTAIN], p[SUSTAIN]) = (0.5, 150.0, 0.0, 1.0, 0.001, 0.5, 0.0, 1.0);
        let (l, _, _) = play(&p, &[48], 1.2, 1.2);
        // How sharp: the size of its steps against its own size.
        let sharp = |x: &[f32]| rms(&x.windows(2).map(|w| w[1] - w[0]).collect::<Vec<f32>>()) / rms(x).max(1e-9);
        let (bright, dark) = (sharp(&l[240..2640]), sharp(&l[43_200..48_000]));
        assert!(bright > dark * 3.0, "{bright} {dark}");
    }

    #[test]
    fn mono_glides_to_the_next_note_and_back() {
        let mut p = preset("Init");
        (p[VOICES], p[GLIDE], p[OSC1_SHAPE], p[CUTOFF], p[ENV_AMT], p[SUSTAIN]) = (1.0, 0.15, 0.0, 20_000.0, 0.0, 1.0);
        let mut st = CrimsonState::new(SR);
        st.note_on(&p, 48, 0.9);
        let run = |st: &mut CrimsonState, blocks: usize| {
            let mut out = Vec::new();
            for _ in 0..blocks {
                let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
                st.render(&p, &mut l, &mut r);
                out.extend(l);
            }
            out
        };
        let low = crossings(&run(&mut st, 40));
        st.note_on(&p, 60, 0.9);
        let early = crossings(&run(&mut st, 4));
        let late = crossings(&run(&mut st, 40));
        assert!(st.voices[0].gate && st.voices.iter().filter(|v| v.active()).count() == 1, "one voice, held into the next");
        // 40 blocks at 130.8 Hz then 261.6 Hz: twice the crossings once there.
        assert!(late > low * 18 / 10, "{low} {late}");
        assert!(early * 10 < late * 9 / 10, "gliding at first: {early} {late}");
        st.note_off(&p, 60);
        assert_eq!(st.voices[0].note, 48, "back to the key still held");
    }

    #[test]
    fn at_most_as_many_notes_as_voices() {
        let mut p = preset("Init");
        (p[VOICES], p[UNISON], p[SUSTAIN], p[RELEASE]) = (2.0, 2.0, 1.0, 0.005);
        let (_, _, st) = play(&p, &[60, 64, 67], 0.3, 1.0);
        let notes: std::collections::HashSet<u8> = st.voices.iter().filter(|v| v.gate).map(|v| v.note).collect();
        assert_eq!(notes, [64u8, 67].into_iter().collect(), "the oldest let go");
        assert_eq!(st.voices.iter().filter(|v| v.gate).count(), 4, "two voices each");
    }

    #[test]
    fn the_saturations_and_routings_stay_in_bounds() {
        for sat in 0..SATURATIONS.len() {
            for routing in 0..ROUTINGS.len() {
                let mut p = preset("Rave Stab");
                (p[SATURATION], p[ROUTING], p[DRIVE], p[RESONANCE]) = (sat as f32, routing as f32, 1.0, 0.95);
                let (l, r, _) = play(&p, &[40, 52, 59], 0.6, 0.3);
                assert!(rms(&l) > 1e-3, "{sat} {routing} sounds");
                assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 3.0), "{sat} {routing} in bounds");
            }
        }
    }

    #[test]
    fn sync_and_fm_change_the_second_oscillator() {
        let osc2 = |sync: f32, fm: f32| {
            let mut p = preset("Init");
            (p[BALANCE], p[SEMITONE], p[SYNC], p[FM], p[CUTOFF], p[ENV_AMT], p[SUSTAIN]) = (1.0, 7.0, sync, fm, 20_000.0, 0.0, 1.0);
            play(&p, &[48], 0.3, 0.3).0
        };
        let plain = osc2(0.0, 0.0);
        for other in [osc2(1.0, 0.0), osc2(0.0, 0.6)] {
            let d: f32 = plain.iter().zip(&other).map(|(a, b)| (a - b).abs()).sum::<f32>() / plain.len() as f32;
            assert!(d > 0.01, "{d}");
        }
    }

    #[test]
    fn the_effects_ring_on_after_the_note() {
        let mut p = preset("Hyper Pluck");
        (p[DELAY], p[REVERB]) = (0.0, 0.0);
        let (dry, _, _) = play(&p, &[60], 2.0, 0.1);
        (p[DELAY], p[REVERB], p[FEEDBACK]) = (0.6, 0.5, 0.5);
        let (wet, _, _) = play(&p, &[60], 2.0, 0.1);
        let tail = |x: &[f32]| rms(&x[72_000..96_000]);
        assert!(tail(&wet) > tail(&dry) * 5.0 + 1e-5, "{} {}", tail(&wet), tail(&dry));
    }
}
