//! Silicon: a synth that sounds on the emulated DSP56300 (the chip Hyper's
//! Virus runs on) with Krabton's own firmware, `silicon.asm`: no Virus
//! firmware, nothing of Access's. Every sample is worked out on the chip:
//! two LFOs; per voice two oscillators that are each a classic wave (a saw
//! turning into a pulse of any width), a hypersaw (up to nine saws) or a
//! wavetable (Waves' tables, put into the chip's memory), the second
//! frequency modulated by the first or synced to it, a third oscillator, a
//! sub, coloured noise, ring modulation, a drive, filter 1 (a state
//! variable filter or a four pole ladder) with saturation after it, filter 2
//! after it, beside it or each oscillator through its own, a filter and an
//! amplifier envelope; then distortion, a chorus or flanger, a phaser, a
//! delay going from side to side, a reverb and a boost of the low end.
//!
//! This side is the chip's host, as a microcontroller would be: it takes the
//! notes and picks the voices (unison too), glides their pitch, moves the
//! envelopes from attack to decay to sustain (sloping, if asked) to release,
//! works out the knobs, writes it all into the chip's X memory, runs the
//! firmware a block at a time and reads the samples back from Y memory.

use super::waves::{BANKS, WAVE_COUNT, WAVE_LEN};
use super::{db_to_gain, midi_to_freq, pan_gains};
use crate::params::{ParamSpec, Unit};
use krabton_virus::chip::{self, frac, to_frac, Area, Chip, Program};
use std::sync::OnceLock;

// The first knobs, as the first Silicon had them (their places kept).
pub const SHAPE: usize = 0;
pub const OSC2: usize = 1;
pub const DETUNE: usize = 2;
pub const SEMITONE: usize = 3;
pub const SUB: usize = 4;
pub const CUTOFF: usize = 5;
pub const RESONANCE: usize = 6;
pub const FILTER: usize = 7;
pub const ENV_AMT: usize = 8;
pub const F_DECAY: usize = 9;
pub const KEY_TRACK: usize = 10;
pub const ATTACK: usize = 11;
pub const DECAY: usize = 12;
pub const SUSTAIN: usize = 13;
pub const RELEASE: usize = 14;
pub const LFO_RATE: usize = 15;
pub const LFO_PITCH: usize = 16;
pub const LFO_CUTOFF: usize = 17;
pub const GLIDE: usize = 18;
pub const VOICES: usize = 19;
pub const VELOCITY: usize = 20;
pub const VOLUME: usize = 21;
// The second firmware's.
pub const PW1: usize = 22;
pub const SHAPE2: usize = 23;
pub const PW2: usize = 24;
pub const FM: usize = 25;
pub const SYNC: usize = 26;
pub const RING: usize = 27;
pub const NOISE: usize = 28;
pub const DRIVE: usize = 29;
pub const ROUTING: usize = 30;
pub const CUTOFF2: usize = 31;
pub const RES2: usize = 32;
pub const FILTER2: usize = 33;
pub const F_ATTACK: usize = 34;
pub const F_SUSTAIN: usize = 35;
pub const F_RELEASE: usize = 36;
pub const VEL_CUTOFF: usize = 37;
pub const LFO_SHAPE: usize = 38;
pub const LFO_PW: usize = 39;
pub const LFO_AMP: usize = 40;
pub const UNISON: usize = 41;
pub const UNI_DETUNE: usize = 42;
pub const UNI_SPREAD: usize = 43;
pub const OCTAVE: usize = 44;
pub const CHORUS: usize = 45;
pub const CHORUS_DEPTH: usize = 46;
pub const DELAY: usize = 47;
pub const DELAY_TIME: usize = 48;
pub const FEEDBACK: usize = 49;
pub const REVERB: usize = 50;
pub const SIZE: usize = 51;
pub const DAMP: usize = 52;
// The third's.
pub const OSC1_MODE: usize = 53;
pub const OSC2_MODE: usize = 54;
pub const DENSITY: usize = 55;
pub const SAW_SPREAD: usize = 56;
pub const TABLE1: usize = 57;
pub const TABLE2: usize = 58;
pub const WAVE1: usize = 59;
pub const WAVE2: usize = 60;
pub const OSC3: usize = 61;
pub const SEMI3: usize = 62;
pub const SHAPE3: usize = 63;
pub const SUB_WAVE: usize = 64;
pub const NOISE_COLOR: usize = 65;
pub const ENV_OSC2: usize = 66;
pub const ENV_FM: usize = 67;
pub const SATURATION: usize = 68;
pub const SAT_AMOUNT: usize = 69;
pub const BALANCE: usize = 70;
pub const F_SLOPE: usize = 71;
pub const SLOPE: usize = 72;
pub const LFO2_SHAPE: usize = 73;
pub const LFO2_RATE: usize = 74;
pub const LFO2_CUTOFF2: usize = 75;
pub const LFO2_PAN: usize = 76;
pub const LFO2_FM: usize = 77;
pub const LFO2_WAVE: usize = 78;
pub const DISTORTION: usize = 79;
pub const DIST_AMOUNT: usize = 80;
pub const PHASER: usize = 81;
pub const PHASER_RATE: usize = 82;
pub const PHASER_FB: usize = 83;
pub const CHORUS_MODE: usize = 84;
pub const BOOST: usize = 85;
pub const BOOST_FREQ: usize = 86;

pub const FILTERS: &[&str] = &["LP", "BP", "HP", "Notch", "Ladder 24", "Ladder 12"];
pub const FILTERS2: &[&str] = &["LP", "BP", "HP", "Notch"];
pub const ROUTINGS: &[&str] = &["Off", "Serial", "Parallel", "Split"];
pub const LFO_SHAPES: &[&str] = &["Tri", "Sine", "Saw", "Square", "S&H", "S&G"];
pub const OSC_MODES: &[&str] = &["Classic", "HyperSaw", "Wavetable"];
pub const SUB_WAVES: &[&str] = &["Square", "Triangle"];
pub const SATURATIONS: &[&str] = &["Off", "Light", "Soft", "Hard", "Digital", "Shaper", "Rectifier"];
pub const DISTORTIONS: &[&str] = &["Off", "Light", "Soft", "Hard", "Bits", "Rate"];
pub const CHORUS_MODES: &[&str] = &["Chorus", "Flanger"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Shape", "Osc 1", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Osc 2", "Osc 2", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Detune", "Osc 2", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::int("Semitone", "Osc 2", -12.0, 12.0, 0.0, Unit::Semi),
    ParamSpec::lin("Sub", "Osc 3", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Cutoff", "Filter", 40.0, 15_000.0, 1800.0, Unit::Hz),
    ParamSpec::lin("Resonance", "Filter", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::choice("Filter", "Filter", FILTERS, 0),
    ParamSpec::lin("Env Amount", "Filter", -1.0, 1.0, 0.5, Unit::None),
    ParamSpec::log("Env Decay", "Filter Env", 0.01, 5.0, 0.3, Unit::Sec),
    ParamSpec::lin("Key Track", "Filter", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::log("Attack", "Amp", 0.001, 5.0, 0.005, Unit::Sec),
    ParamSpec::log("Decay", "Amp", 0.01, 5.0, 0.4, Unit::Sec),
    ParamSpec::lin("Sustain", "Amp", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::log("Release", "Amp", 0.01, 5.0, 0.3, Unit::Sec),
    ParamSpec::log("LFO Rate", "LFO 1", 0.05, 20.0, 4.0, Unit::Hz),
    ParamSpec::lin("LFO Pitch", "LFO 1", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("LFO Cutoff", "LFO 1", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Glide", "Play", 0.001, 1.0, 0.001, Unit::Sec),
    ParamSpec::int("Voices", "Play", 1.0, 16.0, 8.0, Unit::None),
    ParamSpec::lin("Velocity", "Play", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Volume", "Play", -30.0, 6.0, -4.0, Unit::Db),
    ParamSpec::lin("Pulse Width", "Osc 1", 0.05, 0.5, 0.5, Unit::Percent),
    ParamSpec::lin("Shape 2", "Osc 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Pulse Width 2", "Osc 2", 0.05, 0.5, 0.5, Unit::Percent),
    ParamSpec::lin("FM", "Osc 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Sync", "Osc 2", false),
    ParamSpec::lin("Ring", "Osc 3", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Noise", "Osc 3", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Drive", "Filter", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Routing", "Filter", ROUTINGS, 0),
    ParamSpec::lin("Cutoff 2", "Filter", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Resonance 2", "Filter", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::choice("Filter 2", "Filter", FILTERS2, 0),
    ParamSpec::log("Env Attack", "Filter Env", 0.001, 5.0, 0.001, Unit::Sec),
    ParamSpec::lin("Env Sustain", "Filter Env", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Env Release", "Filter Env", 0.01, 5.0, 0.3, Unit::Sec),
    ParamSpec::lin("Vel > Cutoff", "Filter", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::choice("LFO Shape", "LFO 1", LFO_SHAPES, 1),
    ParamSpec::lin("LFO PW", "LFO 1", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("LFO Amp", "LFO 1", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Unison", "Play", 1.0, 8.0, 1.0, Unit::None),
    ParamSpec::lin("Unison Detune", "Play", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Spread", "Play", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::int("Octave", "Play", -2.0, 2.0, 0.0, Unit::None),
    ParamSpec::lin("Chorus", "Effects", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Chorus Depth", "Effects", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Delay", "Effects", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Delay Time", "Effects", 0.02, 0.66, 0.33, Unit::Sec),
    ParamSpec::lin("Feedback", "Effects", 0.0, 0.9, 0.4, Unit::Percent),
    ParamSpec::lin("Reverb", "Effects", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Reverb Size", "Effects", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Reverb Damp", "Effects", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::choice("Osc 1 Mode", "Osc 1", OSC_MODES, 0),
    ParamSpec::choice("Osc 2 Mode", "Osc 2", OSC_MODES, 0),
    ParamSpec::int("Density", "Osc 1", 1.0, 9.0, 7.0, Unit::None),
    ParamSpec::lin("Saw Spread", "Osc 1", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::choice("Table 1", "Osc 1", BANKS, 0),
    ParamSpec::choice("Table 2", "Osc 2", BANKS, 0),
    ParamSpec::lin("Wave 1", "Osc 1", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Wave 2", "Osc 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Osc 3", "Osc 3", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Semitone 3", "Osc 3", -24.0, 24.0, -12.0, Unit::Semi),
    ParamSpec::lin("Shape 3", "Osc 3", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Sub Wave", "Osc 3", SUB_WAVES, 0),
    ParamSpec::lin("Noise Color", "Osc 3", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Env > Osc 2", "Osc 2", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Env > FM", "Osc 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Saturation", "Filter", SATURATIONS, 0),
    ParamSpec::lin("Sat Amount", "Filter", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Filter Balance", "Filter", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Env Slope", "Filter Env", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Amp Slope", "Amp", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::choice("LFO 2 Shape", "LFO 2", LFO_SHAPES, 0),
    ParamSpec::log("LFO 2 Rate", "LFO 2", 0.05, 20.0, 0.5, Unit::Hz),
    ParamSpec::lin("LFO 2 Cutoff 2", "LFO 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("LFO 2 Pan", "LFO 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("LFO 2 FM", "LFO 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("LFO 2 Wave", "LFO 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Distortion", "Effects", DISTORTIONS, 0),
    ParamSpec::lin("Dist Amount", "Effects", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Phaser", "Effects", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Phaser Rate", "Effects", 0.05, 5.0, 0.3, Unit::Hz),
    ParamSpec::lin("Phaser Feedback", "Effects", 0.0, 0.9, 0.5, Unit::Percent),
    ParamSpec::choice("Chorus Mode", "Effects", CHORUS_MODES, 0),
    ParamSpec::lin("Boost", "Effects", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Boost Freq", "Effects", 40.0, 400.0, 120.0, Unit::Hz),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Init",
        "Chip Bass",
        "Square Lead",
        "Silicon Pad",
        "Resonant Pluck",
        "Hollow",
        "Sub Drone",
        "Sync Sweep",
        "Bright Brass",
        "Wobble",
        "PWM Strings",
        "FM Bell",
        "Ring Clang",
        "Noise Sweep",
        "Unison Lead",
        "Dual Filter Bass",
        "Echo Pluck",
        "Space Choir",
        "Driven Acid",
        "Tremolo Keys",
        "Hyper Lead",
        "Hyper Pad",
        "Trance Pluck",
        "Wavetable Pad",
        "Wave Sweep",
        "Analog Bass",
        "Acid Ladder",
        "Split Stack",
        "Phaser Pad",
        "Flanger Lead",
        "Bitcrush Lead",
        "Rate Bass",
        "S&H Bubbles",
        "Env FM Bell",
        "Three Osc Stack",
        "Rectified Lead",
    ]
}

fn bank(name: &str) -> f32 {
    BANKS.iter().position(|&b| b == name).unwrap_or(0) as f32
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: Vec<(usize, f32)> = match name {
        "Chip Bass" => vec![(SHAPE, 0.0), (OSC2, 0.4), (DETUNE, 0.1), (SUB, 0.5), (CUTOFF, 300.0), (RESONANCE, 0.55), (ENV_AMT, 0.6), (F_DECAY, 0.2), (DECAY, 0.3), (SUSTAIN, 0.6), (RELEASE, 0.08), (VOICES, 1.0), (GLIDE, 0.05)],
        "Square Lead" => vec![(SHAPE, 1.0), (OSC2, 0.3), (DETUNE, 0.2), (SEMITONE, 12.0), (CUTOFF, 3500.0), (RESONANCE, 0.25), (ENV_AMT, 0.25), (F_DECAY, 0.5), (SUSTAIN, 0.9), (RELEASE, 0.2), (LFO_RATE, 5.5), (LFO_PITCH, 0.12), (VOICES, 1.0), (GLIDE, 0.04), (DELAY, 0.25), (DELAY_TIME, 0.3)],
        "Silicon Pad" => vec![(SHAPE, 0.3), (OSC2, 0.8), (DETUNE, 0.35), (CUTOFF, 1400.0), (RESONANCE, 0.2), (ENV_AMT, 0.15), (F_DECAY, 1.5), (ATTACK, 0.6), (DECAY, 1.0), (SUSTAIN, 0.85), (RELEASE, 1.5), (LFO_RATE, 0.3), (LFO_CUTOFF, 0.25), (CHORUS, 0.5), (REVERB, 0.4)],
        "Resonant Pluck" => vec![(SHAPE, 0.0), (OSC2, 0.3), (CUTOFF, 250.0), (RESONANCE, 0.8), (ENV_AMT, 0.8), (F_DECAY, 0.18), (DECAY, 0.5), (SUSTAIN, 0.0), (RELEASE, 0.3), (REVERB, 0.2)],
        "Hollow" => vec![(SHAPE, 0.8), (OSC2, 0.0), (FILTER, 1.0), (CUTOFF, 900.0), (RESONANCE, 0.5), (ENV_AMT, 0.3), (F_DECAY, 0.6), (SUSTAIN, 0.8), (LFO_RATE, 0.5), (LFO_CUTOFF, 0.3)],
        "Sub Drone" => vec![(SHAPE, 0.5), (OSC2, 0.6), (DETUNE, 0.5), (SEMITONE, -12.0), (SUB, 0.8), (CUTOFF, 500.0), (RESONANCE, 0.4), (ENV_AMT, 0.0), (ATTACK, 1.5), (SUSTAIN, 1.0), (RELEASE, 2.5), (LFO_RATE, 0.15), (LFO_CUTOFF, 0.4), (REVERB, 0.3)],
        "Sync Sweep" => vec![(SHAPE, 0.0), (OSC2, 0.9), (SEMITONE, 7.0), (DETUNE, 0.05), (SYNC, 1.0), (ENV_OSC2, 0.6), (CUTOFF, 2500.0), (RESONANCE, 0.3), (ENV_AMT, 0.4), (F_DECAY, 1.2), (F_SUSTAIN, 0.3), (SUSTAIN, 0.9), (VOICES, 1.0), (GLIDE, 0.03)],
        "Bright Brass" => vec![(SHAPE, 0.0), (OSC2, 0.7), (DETUNE, 0.25), (CUTOFF, 700.0), (RESONANCE, 0.15), (ENV_AMT, 0.55), (F_ATTACK, 0.06), (F_DECAY, 0.5), (F_SUSTAIN, 0.4), (ATTACK, 0.04), (SUSTAIN, 0.85), (RELEASE, 0.25), (CHORUS, 0.25), (REVERB, 0.2)],
        "Wobble" => vec![(SHAPE, 0.6), (OSC2, 0.5), (SEMITONE, -12.0), (SUB, 0.4), (CUTOFF, 350.0), (RESONANCE, 0.6), (ENV_AMT, 0.0), (SUSTAIN, 1.0), (RELEASE, 0.1), (LFO_RATE, 3.0), (LFO_SHAPE, 0.0), (LFO_CUTOFF, 0.8), (VOICES, 1.0), (DRIVE, 0.4)],
        "PWM Strings" => vec![(SHAPE, 1.0), (PW1, 0.35), (SHAPE2, 1.0), (PW2, 0.25), (OSC2, 0.7), (DETUNE, 0.2), (LFO_RATE, 0.6), (LFO_SHAPE, 0.0), (LFO_PW, 0.6), (CUTOFF, 3000.0), (RESONANCE, 0.1), (ENV_AMT, 0.1), (ATTACK, 0.35), (SUSTAIN, 0.9), (RELEASE, 0.9), (CHORUS, 0.6), (REVERB, 0.35)],
        "FM Bell" => vec![(SHAPE, 0.0), (OSC2, 0.9), (SEMITONE, 7.0), (DETUNE, 0.0), (FM, 0.6), (CUTOFF, 9000.0), (RESONANCE, 0.0), (ENV_AMT, 0.2), (DECAY, 1.8), (SUSTAIN, 0.0), (RELEASE, 1.2), (REVERB, 0.45), (DELAY, 0.15)],
        "Ring Clang" => vec![(SHAPE, 0.5), (OSC2, 0.2), (SEMITONE, 5.0), (DETUNE, 0.6), (RING, 1.0), (CUTOFF, 5000.0), (RESONANCE, 0.3), (ENV_AMT, 0.3), (F_DECAY, 0.4), (DECAY, 0.9), (SUSTAIN, 0.0), (RELEASE, 0.6), (REVERB, 0.3)],
        "Noise Sweep" => vec![(SHAPE, 0.0), (OSC2, 0.0), (NOISE, 0.9), (FILTER, 1.0), (CUTOFF, 300.0), (RESONANCE, 0.85), (ENV_AMT, 0.8), (F_ATTACK, 0.8), (F_DECAY, 1.5), (F_SUSTAIN, 0.2), (ATTACK, 0.2), (SUSTAIN, 0.8), (RELEASE, 1.0), (REVERB, 0.5), (KEY_TRACK, 1.0)],
        "Unison Lead" => vec![(SHAPE, 0.0), (OSC2, 0.5), (DETUNE, 0.1), (UNISON, 4.0), (UNI_DETUNE, 0.35), (UNI_SPREAD, 0.8), (CUTOFF, 4000.0), (RESONANCE, 0.2), (ENV_AMT, 0.2), (SUSTAIN, 0.9), (RELEASE, 0.25), (VOICES, 4.0), (GLIDE, 0.02), (DELAY, 0.3), (DELAY_TIME, 0.375), (REVERB, 0.25)],
        "Dual Filter Bass" => vec![(SHAPE, 0.3), (OSC2, 0.6), (DETUNE, 0.15), (SUB, 0.3), (CUTOFF, 600.0), (RESONANCE, 0.5), (ENV_AMT, 0.5), (F_DECAY, 0.25), (ROUTING, 1.0), (CUTOFF2, 0.4), (RES2, 0.6), (FILTER2, 1.0), (DRIVE, 0.3), (SUSTAIN, 0.8), (RELEASE, 0.1), (VOICES, 1.0), (OCTAVE, -1.0)],
        "Echo Pluck" => vec![(SHAPE, 0.7), (PW1, 0.3), (OSC2, 0.3), (CUTOFF, 400.0), (RESONANCE, 0.4), (ENV_AMT, 0.7), (F_DECAY, 0.2), (DECAY, 0.35), (SUSTAIN, 0.0), (RELEASE, 0.2), (DELAY, 0.5), (DELAY_TIME, 0.375), (FEEDBACK, 0.55), (REVERB, 0.2)],
        "Space Choir" => vec![(SHAPE, 0.4), (OSC2, 0.7), (DETUNE, 0.3), (NOISE, 0.08), (FILTER, 1.0), (CUTOFF, 1200.0), (RESONANCE, 0.45), (ROUTING, 2.0), (CUTOFF2, 0.5), (FILTER2, 1.0), (RES2, 0.5), (ATTACK, 0.8), (SUSTAIN, 1.0), (RELEASE, 2.0), (UNISON, 2.0), (VOICES, 6.0), (CHORUS, 0.7), (REVERB, 0.7), (SIZE, 0.85)],
        "Driven Acid" => vec![(SHAPE, 0.0), (OSC2, 0.0), (CUTOFF, 280.0), (RESONANCE, 0.85), (ENV_AMT, 0.75), (F_DECAY, 0.22), (DRIVE, 0.7), (DECAY, 0.4), (SUSTAIN, 0.7), (RELEASE, 0.06), (VOICES, 1.0), (GLIDE, 0.06), (OCTAVE, -1.0), (VEL_CUTOFF, 0.6), (DELAY, 0.2), (DELAY_TIME, 0.28)],
        "Tremolo Keys" => vec![(SHAPE, 0.6), (PW1, 0.4), (OSC2, 0.3), (SEMITONE, 12.0), (CUTOFF, 2200.0), (RESONANCE, 0.1), (ENV_AMT, 0.3), (F_DECAY, 0.8), (DECAY, 1.4), (SUSTAIN, 0.3), (RELEASE, 0.4), (LFO_RATE, 5.0), (LFO_SHAPE, 1.0), (LFO_AMP, 0.6), (CHORUS, 0.3), (REVERB, 0.3)],
        "Hyper Lead" => vec![(OSC1_MODE, 1.0), (DENSITY, 9.0), (SAW_SPREAD, 0.45), (OSC2, 0.3), (SEMITONE, 12.0), (UNISON, 2.0), (UNI_DETUNE, 0.15), (CUTOFF, 5000.0), (RESONANCE, 0.2), (ENV_AMT, 0.3), (F_DECAY, 0.5), (F_SUSTAIN, 0.4), (SUSTAIN, 1.0), (RELEASE, 0.3), (GLIDE, 0.02), (LFO_RATE, 5.5), (LFO_PITCH, 0.08), (CHORUS, 0.2), (DELAY, 0.3), (DELAY_TIME, 0.375), (REVERB, 0.25)],
        "Hyper Pad" => vec![(OSC1_MODE, 1.0), (DENSITY, 9.0), (SAW_SPREAD, 0.6), (OSC2, 0.0), (UNISON, 2.0), (UNI_DETUNE, 0.25), (CUTOFF, 2500.0), (RESONANCE, 0.15), (ENV_AMT, 0.2), (F_ATTACK, 1.2), (F_DECAY, 2.0), (F_SUSTAIN, 0.6), (ATTACK, 0.7), (SUSTAIN, 0.9), (RELEASE, 2.0), (LFO_RATE, 0.2), (LFO_CUTOFF, 0.15), (CHORUS, 0.5), (REVERB, 0.6)],
        "Trance Pluck" => vec![(OSC1_MODE, 1.0), (DENSITY, 7.0), (SAW_SPREAD, 0.35), (OSC2, 0.0), (CUTOFF, 400.0), (RESONANCE, 0.3), (ENV_AMT, 0.7), (F_DECAY, 0.25), (DECAY, 0.4), (SUSTAIN, 0.0), (RELEASE, 0.3), (DELAY, 0.35), (DELAY_TIME, 0.375), (FEEDBACK, 0.4), (REVERB, 0.25)],
        "Wavetable Pad" => vec![(OSC1_MODE, 2.0), (TABLE1, bank("Vocal")), (WAVE1, 0.2), (OSC2_MODE, 2.0), (TABLE2, bank("Strings")), (WAVE2, 0.6), (OSC2, 0.7), (DETUNE, 0.15), (LFO2_SHAPE, 1.0), (LFO2_RATE, 0.15), (LFO2_WAVE, 0.5), (CUTOFF, 3500.0), (ENV_AMT, 0.1), (ATTACK, 0.6), (SUSTAIN, 0.9), (RELEASE, 1.8), (CHORUS, 0.3), (REVERB, 0.5)],
        "Wave Sweep" => vec![(OSC1_MODE, 2.0), (TABLE1, bank("Bright Saws")), (WAVE1, 0.0), (OSC2, 0.0), (LFO2_SHAPE, 0.0), (LFO2_RATE, 0.25), (LFO2_WAVE, 0.9), (CUTOFF, 6000.0), (RESONANCE, 0.2), (ENV_AMT, 0.1), (SUSTAIN, 1.0), (RELEASE, 0.3), (VOICES, 1.0), (GLIDE, 0.03), (DELAY, 0.25)],
        "Analog Bass" => vec![(FILTER, 4.0), (SHAPE, 0.0), (OSC2, 0.5), (SHAPE2, 1.0), (SEMITONE, -12.0), (SUB, 0.3), (CUTOFF, 220.0), (RESONANCE, 0.5), (ENV_AMT, 0.7), (F_DECAY, 0.25), (F_SUSTAIN, 0.2), (SUSTAIN, 0.8), (RELEASE, 0.1), (VOICES, 1.0), (GLIDE, 0.04), (BOOST, 0.4)],
        "Acid Ladder" => vec![(FILTER, 4.0), (OSC2, 0.0), (CUTOFF, 250.0), (RESONANCE, 0.85), (ENV_AMT, 0.8), (F_DECAY, 0.2), (DRIVE, 0.5), (SATURATION, 2.0), (SAT_AMOUNT, 0.5), (SUSTAIN, 0.7), (RELEASE, 0.06), (VOICES, 1.0), (GLIDE, 0.06), (OCTAVE, -1.0), (DELAY, 0.2), (DELAY_TIME, 0.28)],
        "Split Stack" => vec![(OSC1_MODE, 1.0), (DENSITY, 5.0), (SAW_SPREAD, 0.3), (OSC2, 0.8), (SHAPE2, 1.0), (SEMITONE, -12.0), (ROUTING, 3.0), (CUTOFF, 1800.0), (RESONANCE, 0.3), (CUTOFF2, -0.5), (RES2, 0.5), (FILTER2, 0.0), (BALANCE, 0.5), (ENV_AMT, 0.3), (SUSTAIN, 0.9), (CHORUS, 0.3), (REVERB, 0.3)],
        "Phaser Pad" => vec![(SHAPE, 0.0), (OSC2, 0.8), (DETUNE, 0.3), (CUTOFF, 3000.0), (RESONANCE, 0.1), (ENV_AMT, 0.1), (ATTACK, 0.4), (SUSTAIN, 0.9), (RELEASE, 1.5), (PHASER, 0.8), (PHASER_RATE, 0.2), (PHASER_FB, 0.6), (REVERB, 0.4)],
        "Flanger Lead" => vec![(SHAPE, 1.0), (PW1, 0.3), (OSC2, 0.4), (DETUNE, 0.2), (CUTOFF, 4000.0), (ENV_AMT, 0.2), (SUSTAIN, 0.9), (VOICES, 1.0), (GLIDE, 0.03), (CHORUS, 0.8), (CHORUS_MODE, 1.0), (CHORUS_DEPTH, 0.7), (DELAY, 0.2)],
        "Bitcrush Lead" => vec![(SHAPE, 1.0), (OSC2, 0.4), (SEMITONE, 7.0), (CUTOFF, 6000.0), (SUSTAIN, 1.0), (VOICES, 1.0), (DISTORTION, 4.0), (DIST_AMOUNT, 0.65), (DELAY, 0.3), (DELAY_TIME, 0.33)],
        "Rate Bass" => vec![(SHAPE, 0.0), (OSC2, 0.5), (SEMITONE, -12.0), (CUTOFF, 900.0), (RESONANCE, 0.4), (ENV_AMT, 0.5), (F_DECAY, 0.3), (SUSTAIN, 0.8), (VOICES, 1.0), (OCTAVE, -1.0), (DISTORTION, 5.0), (DIST_AMOUNT, 0.6)],
        "S&H Bubbles" => vec![(SHAPE, 0.0), (OSC2, 0.4), (CUTOFF, 800.0), (RESONANCE, 0.75), (ENV_AMT, 0.1), (LFO_SHAPE, 4.0), (LFO_RATE, 8.0), (LFO_CUTOFF, 0.7), (DECAY, 0.3), (SUSTAIN, 0.5), (RELEASE, 0.3), (DELAY, 0.35), (DELAY_TIME, 0.25), (FEEDBACK, 0.5), (REVERB, 0.3)],
        "Env FM Bell" => vec![(SHAPE, 0.0), (OSC2, 0.9), (SEMITONE, 12.0), (DETUNE, 0.02), (FM, 0.1), (ENV_FM, 0.8), (F_DECAY, 0.6), (CUTOFF, 8000.0), (ENV_AMT, 0.1), (DECAY, 2.0), (SUSTAIN, 0.0), (RELEASE, 1.5), (REVERB, 0.4)],
        "Three Osc Stack" => vec![(SHAPE, 0.0), (OSC2, 0.7), (DETUNE, 0.2), (OSC3, 0.7), (SEMI3, -12.0), (SHAPE3, 1.0), (CUTOFF, 1500.0), (RESONANCE, 0.25), (ENV_AMT, 0.4), (F_DECAY, 0.5), (SUSTAIN, 0.9), (RELEASE, 0.3), (CHORUS, 0.2)],
        "Rectified Lead" => vec![(SHAPE, 0.4), (OSC2, 0.3), (CUTOFF, 1200.0), (RESONANCE, 0.4), (SATURATION, 6.0), (SAT_AMOUNT, 0.7), (ROUTING, 1.0), (CUTOFF2, 0.6), (FILTER2, 0.0), (ENV_AMT, 0.3), (SUSTAIN, 0.9), (VOICES, 1.0), (GLIDE, 0.03), (DELAY, 0.25)],
        _ => Vec::new(),
    };
    for (i, v) in set {
        p[i] = v;
    }
    p
}

/// The firmware's source.
pub static FIRMWARE: &str = include_str!("silicon.asm");
/// Where it is put in P memory.
const ORIGIN: u32 = 0x100;
/// The chip's memories, each this many words (the delay's lines need it).
const MEMORY: u32 = 0x20000;
/// Voices on the chip (a note takes as many as its unison).
pub const MAX_VOICES: usize = 16;
/// The most samples the firmware makes in one go.
const MAX_BLOCK: usize = 256;
/// A wavetable's samples, and how many harmonics are kept in it.
const TABLE_LEN: usize = 256;
const TABLE_HARMONICS: usize = 48;

// Where the firmware reads the knobs (X memory).
const G_SAW1: u32 = 0x00;
const G_N: u32 = 0x40;
const CH_MIX: u32 = 0x48;
const CH_INC: u32 = 0x4c;
const CH_POS: u32 = 0x4d;
const DL_MIX: u32 = 0x50;
const DL_LP_L: u32 = 0x54;
const RV_MIX: u32 = 0x58;
const RV_POS_0: u32 = 0x5c;
const CH_FB: u32 = 0x69;
const LFO1: u32 = 0x70;
const LFO2: u32 = 0x7b;
const RNG: u32 = 0x86;
const G_SPLIT: u32 = 0x88;
const G_HS_TAB: u32 = 0xa0;
const FX_FLAGS: u32 = 0x100;
const PH_MIX: u32 = 0x108;
const BO_AMT: u32 = 0x130;
// The lines and the tables in Y memory.
const CH_LINE_L: u32 = 0x1000;
const DL_LINE_L: u32 = 0x10000;
const DL_LINE_R: u32 = 0x18000;
const RV_LINES: [u32; 6] = [0x2000, 0x2800, 0x3000, 0x3800, 0x4000, 0x4400];
const TABLES: u32 = 0x8000;
const OUT_L: u32 = 0x000;
const OUT_R: u32 = 0x100;
// A voice's 64 words, in X from V_BASE on.
const V_BASE: u32 = 0x200;
const V_WORDS: u32 = 64;
const W_ACTIVE: u32 = 0;
const W_PH1: u32 = 1;
const W_INC1: u32 = 2;
const W_PH2: u32 = 3;
const W_INC2: u32 = 4;
const W_PHS: u32 = 5;
const W_INCS: u32 = 6;
const W_ENV: u32 = 7;
/// Its envelopes' targets, and (next to each) their rates.
const W_TARGET: u32 = 9;
const W_FENV: u32 = 11;
const W_FTARGET: u32 = 13;
const W_LOW: u32 = 15;
const W_VEL: u32 = 19;
const W_CUT: u32 = 20;
const W_PAN_L: u32 = 21;
const W_NOISE: u32 = 23;
const W_HS: u32 = 32;
const W_PH3: u32 = 50;
const W_INC3: u32 = 51;
const W_LADDER: u32 = 52;

/// The firmware assembled (once): its words, and where it starts and waits.
pub fn firmware() -> Result<&'static Program, &'static str> {
    static PROGRAM: OnceLock<Result<Program, String>> = OnceLock::new();
    PROGRAM.get_or_init(|| chip::assemble(FIRMWARE, ORIGIN)).as_ref().map_err(|e| e.as_str())
}

/// Waves' 128 waves as the chip's tables: 256 samples a cycle, the first
/// harmonics kept (no aliasing in the middle of the keyboard), half loud.
fn tables() -> &'static [u32] {
    static T: OnceLock<Vec<u32>> = OnceLock::new();
    T.get_or_init(|| {
        let mut out = Vec::with_capacity(WAVE_COUNT * TABLE_LEN);
        let tau = std::f64::consts::TAU;
        let cos: Vec<f64> = (0..WAVE_LEN).map(|k| (tau * k as f64 / WAVE_LEN as f64).cos()).collect();
        let sin: Vec<f64> = (0..WAVE_LEN).map(|k| (tau * k as f64 / WAVE_LEN as f64).sin()).collect();
        for w in 0..WAVE_COUNT {
            let raw = super::waves::wave(w);
            let mut wave = vec![0.0f64; TABLE_LEN];
            for h in 1..=TABLE_HARMONICS {
                let (mut a, mut b, mut k) = (0.0f64, 0.0f64, 0usize);
                for &v in raw {
                    a += v as f64 * cos[k];
                    b += v as f64 * sin[k];
                    k = (k + h) % WAVE_LEN;
                }
                let (a, b) = (a * 2.0 / WAVE_LEN as f64, b * 2.0 / WAVE_LEN as f64);
                for (j, s) in wave.iter_mut().enumerate() {
                    let ph = tau * (h * j) as f64 / TABLE_LEN as f64;
                    *s += a * ph.cos() + b * ph.sin();
                }
            }
            let peak = wave.iter().fold(1e-9f64, |m, v| m.max(v.abs()));
            out.extend(wave.iter().map(|v| frac((v / peak * 0.5) as f32)));
        }
        out
    })
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Stage {
    Off,
    Attack,
    Decay,
    /// At its sustain (and sloping from there, if asked).
    Sustain,
    Release,
}

#[derive(Clone, Copy)]
struct Voice {
    note: u8,
    gate: bool,
    age: u64,
    pitch: f32,
    target: f32,
    /// Unison: semitones off the note.
    detune: f32,
    /// Octaves the velocity moves its cutoff.
    vel_cut: f32,
    stage: Stage,
    fstage: Stage,
}

impl Voice {
    const OFF: Voice = Voice { note: 0, gate: false, age: 0, pitch: 60.0, target: 60.0, detune: 0.0, vel_cut: 0.0, stage: Stage::Off, fstage: Stage::Off };
}

/// One-pole coefficient for a time constant of `tau` seconds.
fn pole(tau: f32, sr: f32) -> f32 {
    1.0 - (-1.0 / (tau.max(0.0002) * sr)).exp()
}

/// Where an envelope's sustain goes, and how fast: still (slope 0), down, or up.
fn slope(sustain: f32, slope: f32, sr: f32) -> (f32, f32) {
    if slope.abs() < 0.01 {
        (sustain, pole(0.01, sr))
    } else {
        (if slope > 0.0 { 0.999 } else { 0.0 }, pole(0.3 + (1.0 - slope.abs()) * 12.0, sr))
    }
}

/// One of the shapes, all of it (the weights the firmware mixes them with).
fn one_hot<const N: usize>(i: usize) -> [u32; N] {
    let mut w = [0u32; N];
    if i < N {
        w[i] = frac(0.999);
    }
    w
}

pub struct SiliconState {
    pub pitch_bend: f32,
    sr: f32,
    chip: Option<Chip>,
    start: u32,
    done: u32,
    voices: [Voice; MAX_VOICES],
    held: Vec<(u8, f32)>,
    counter: u64,
    last_pitch: Option<f32>,
    seed: u32,
    out_l: Vec<u32>,
    out_r: Vec<u32>,
    /// What the face shows: the chip's millions of instruction cycles a second of sound.
    mips: f32,
}

impl SiliconState {
    pub fn new(sr: f32) -> Self {
        let mut st = Self {
            pitch_bend: 0.0,
            sr,
            chip: None,
            start: 0,
            done: 0,
            voices: [Voice::OFF; MAX_VOICES],
            held: Vec::with_capacity(32),
            counter: 0,
            last_pitch: None,
            seed: 0x5111_C0DE,
            out_l: vec![0; MAX_BLOCK],
            out_r: vec![0; MAX_BLOCK],
            mips: 0.0,
        };
        if let (Ok(program), Ok(mut chip)) = (firmware(), Chip::new(MEMORY)) {
            chip.load(program);
            st.start = program.at("render_block");
            st.done = program.at("render_done");
            // The tables, the effects' lines, the chip's randomness.
            chip.write(Area::Y, TABLES, tables());
            chip.write(Area::X, CH_POS, &[CH_LINE_L]);
            chip.write(Area::X, DL_LP_L, &[0, 0, DL_LINE_L, DL_LINE_R]);
            chip.write(Area::X, RV_POS_0, &RV_LINES);
            chip.write(Area::X, RNG, &[0x2b_7e15]);
            // Every voice, oscillator mode and effect once, so the chip compiles its code now and not at the first note.
            st.warm(&mut chip);
            st.chip = Some(chip);
        }
        st
    }

    fn warm(&self, chip: &mut Chip) {
        for v in 0..MAX_VOICES as u32 {
            let mut w = [0u32; V_WORDS as usize];
            w[W_ACTIVE as usize] = 1;
            w[W_INC1 as usize] = frac(0.01);
            w[W_INC2 as usize] = frac(0.01);
            w[W_INCS as usize] = frac(0.005);
            w[W_INC3 as usize] = frac(0.01);
            w[W_TARGET as usize] = frac(0.5);
            w[W_TARGET as usize + 1] = frac(0.01);
            w[W_VEL as usize] = frac(0.1);
            w[W_CUT as usize] = frac(0.1);
            w[W_NOISE as usize] = 0x1234 + v;
            chip.write(Area::X, V_BASE + v * V_WORDS, &w);
        }
        let mut p = preset("Init");
        (p[CHORUS], p[DELAY], p[REVERB], p[PHASER], p[BOOST], p[DISTORTION], p[ROUTING], p[SATURATION]) = (0.5, 0.5, 0.5, 0.5, 0.5, 5.0, 3.0, 1.0);
        for (m1, m2, filter) in [(1.0, 2.0, 4.0), (2.0, 1.0, 0.0), (0.0, 0.0, 5.0)] {
            (p[OSC1_MODE], p[OSC2_MODE], p[FILTER]) = (m1, m2, filter);
            self.globals(chip, &p, 4);
            chip.run(self.start, self.done, 1_000_000);
        }
        for v in 0..MAX_VOICES as u32 {
            chip.write(Area::X, V_BASE + v * V_WORDS, &[0; V_WORDS as usize]);
        }
    }

    /// Whether the chip is there (with its firmware).
    pub fn ready(&self) -> bool {
        self.chip.is_some()
    }

    fn random(&mut self) -> u32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed & 0xFF_FFFF
    }

    fn write(&mut self, v: usize, word: u32, values: &[u32]) {
        if let Some(chip) = self.chip.as_mut() {
            chip.write(Area::X, V_BASE + v as u32 * V_WORDS + word, values);
        }
    }

    fn pitch_of(p: &[f32], note: u8) -> f32 {
        note as f32 + 12.0 * p[OCTAVE].round()
    }

    fn unison(p: &[f32]) -> usize {
        (p[UNISON].round() as usize).clamp(1, 8)
    }

    /// Voice `v` starts a note as unison voice `u` of `n` (from where it is, if it sounds).
    #[allow(clippy::too_many_arguments)]
    fn start(&mut self, p: &[f32], v: usize, u: usize, n: usize, note: u8, vel: f32, from: Option<f32>, retrigger: bool) {
        self.counter += 1;
        let fresh = self.voices[v].stage == Stage::Off;
        let pitch = Self::pitch_of(p, note);
        let c = if n == 1 { 0.0 } else { -1.0 + 2.0 * u as f32 / (n - 1) as f32 };
        let (stage, fstage) = if retrigger || fresh { (Stage::Attack, Stage::Attack) } else { (self.voices[v].stage, self.voices[v].fstage) };
        let vel_cut = p[VEL_CUTOFF].clamp(0.0, 1.0) * (vel.clamp(0.0, 1.0) - 0.6) * 3.0;
        self.voices[v] = Voice { note, gate: true, age: self.counter, pitch: from.unwrap_or(pitch), target: pitch, detune: c * p[UNI_DETUNE].clamp(0.0, 1.0) * 0.4, vel_cut, stage, fstage };
        if fresh {
            // Free running: each from a phase of its own; the envelopes and the filters from rest.
            let phases: Vec<u32> = (0..3).map(|_| self.random()).collect();
            let saws: Vec<u32> = (0..18).map(|_| self.random()).collect();
            let noise = self.random() | 1;
            let p3 = self.random();
            self.write(v, W_PH1, &[phases[0]]);
            self.write(v, W_PH2, &[phases[1]]);
            self.write(v, W_PHS, &[phases[2]]);
            self.write(v, W_ENV, &[0, 0]);
            self.write(v, W_FENV, &[0, 0]);
            self.write(v, W_LOW, &[0, 0, 0, 0]);
            self.write(v, W_NOISE, &[noise]);
            self.write(v, W_HS, &saws);
            self.write(v, W_PH3, &[p3]);
            self.write(v, W_LADDER, &[0, 0, 0, 0, 0]);
        }
        if retrigger || fresh {
            let sr = self.sr;
            self.write(v, W_TARGET, &[frac(0.999), frac(pole(p[ATTACK] / 3.0, sr))]);
            self.write(v, W_FTARGET, &[frac(0.999), frac(pole(p[F_ATTACK] / 3.0, sr))]);
        }
        let amount = 1.0 - p[VELOCITY].clamp(0.0, 1.0) * (1.0 - vel.clamp(0.0, 1.0));
        let (l, r) = pan_gains(c * p[UNI_SPREAD].clamp(0.0, 1.0));
        let scale = 1.0 / (n as f32).sqrt();
        self.write(v, W_VEL, &[frac(amount * 0.5 * scale)]);
        self.write(v, W_PAN_L, &[frac(l * 0.7), frac(r * 0.7)]);
        self.write(v, W_ACTIVE, &[1]);
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() || self.chip.is_none() {
            return;
        }
        let poly = (p[VOICES].round() as usize).clamp(1, 16);
        let uni = Self::unison(p);
        let glide = p[GLIDE] > 0.002;
        let legato = poly == 1 && !self.held.is_empty();
        self.held.retain(|h| h.0 != note);
        self.held.push((note, vel));
        if poly == 1 {
            // Mono: the first voices; held into the next note (legato), gliding there.
            let from = (glide && self.voices[0].stage != Stage::Off).then_some(self.voices[0].pitch);
            for u in 0..uni {
                self.start(p, u, u, uni, note, vel, from, !legato);
            }
            for v in uni..MAX_VOICES {
                if self.voices[v].gate {
                    self.release(p, v);
                }
            }
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
            for v in 0..MAX_VOICES {
                if self.voices[v].gate && self.voices[v].note == old {
                    self.release(p, v);
                }
            }
        }
        // Its voices: the note's own again, free ones, the oldest let go, the oldest.
        let mut order: Vec<(u8, u64, usize)> = (0..MAX_VOICES)
            .map(|v| {
                let x = &self.voices[v];
                let rank = if x.gate && x.note == note {
                    0
                } else if x.stage == Stage::Off {
                    1
                } else if !x.gate {
                    2
                } else {
                    3
                };
                (rank, x.age, v)
            })
            .collect();
        order.sort_unstable();
        let from = if glide { self.last_pitch } else { None };
        let picks: Vec<usize> = order.iter().take(uni).map(|o| o.2).collect();
        for (u, &v) in picks.iter().enumerate() {
            self.start(p, v, u, uni, note, vel, from, true);
        }
        self.last_pitch = Some(Self::pitch_of(p, note));
    }

    fn release(&mut self, p: &[f32], v: usize) {
        self.voices[v].gate = false;
        self.voices[v].stage = Stage::Release;
        self.voices[v].fstage = Stage::Release;
        let sr = self.sr;
        self.write(v, W_TARGET, &[0, frac(pole(p[RELEASE] / 4.6, sr))]);
        self.write(v, W_FTARGET, &[0, frac(pole(p[F_RELEASE] / 4.6, sr))]);
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        if p.len() < SPECS.len() {
            return;
        }
        self.held.retain(|h| h.0 != note);
        let poly = (p[VOICES].round() as usize).clamp(1, 16);
        if poly == 1 && self.voices[0].gate && self.voices[0].note == note {
            // Back to a key still held, gliding there.
            if let Some(&(prev, _)) = self.held.last() {
                let pitch = Self::pitch_of(p, prev);
                for v in self.voices.iter_mut().filter(|v| v.gate) {
                    v.note = prev;
                    v.target = pitch;
                }
                return;
            }
        }
        for v in 0..MAX_VOICES {
            if self.voices[v].gate && self.voices[v].note == note {
                self.release(p, v);
            }
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        let sr = self.sr;
        for v in 0..MAX_VOICES {
            if self.voices[v].stage != Stage::Off {
                self.voices[v].gate = false;
                self.voices[v].stage = Stage::Release;
                self.voices[v].fstage = Stage::Release;
                self.write(v, W_TARGET, &[0, frac(pole(0.02, sr))]);
            }
        }
    }

    /// For the face: voices sounding on the chip, its MIPS, whether it is there.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.voices.iter().filter(|v| v.stage != Stage::Off).count() as f32;
        out[1] = self.mips;
        out[2] = self.ready() as u8 as f32;
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() || self.chip.is_none() {
            return;
        }
        let n = l.len().min(r.len());
        let mut at = 0;
        while at < n {
            let len = MAX_BLOCK.min(n - at);
            self.block(p, len);
            let gain = db_to_gain(p[VOLUME]);
            for i in 0..len {
                l[at + i] += to_frac(self.out_l[i]) * gain;
                r[at + i] += to_frac(self.out_r[i]) * gain;
            }
            at += len;
        }
    }

    /// Every knob as the firmware reads it, into the chip's memory.
    fn globals(&self, chip: &mut Chip, p: &[f32], len: usize) {
        let sr = self.sr;
        let c = |i: usize, lo: f32, hi: f32| p[i].clamp(lo, hi);
        let pick = |i: usize, n: usize| (p[i].round().max(0.0) as usize).min(n - 1);
        let (shape1, shape2, shape3) = (c(SHAPE, 0.0, 1.0), c(SHAPE2, 0.0, 1.0), c(SHAPE3, 0.0, 1.0));
        let filter = pick(FILTER, FILTERS.len());
        let mut f1 = [0.0f32; 3];
        match filter {
            1 => f1[1] = 0.9,
            2 => f1[2] = 0.9,
            3 => (f1[0], f1[2]) = (0.9, 0.9),
            _ => f1[0] = 0.9,
        }
        let mut f2 = [0.0f32; 3];
        match pick(FILTER2, FILTERS2.len()) {
            1 => f2[1] = 0.9,
            2 => f2[2] = 0.9,
            3 => (f2[0], f2[2]) = (0.9, 0.9),
            _ => f2[0] = 0.9,
        }
        let routing = pick(ROUTING, ROUTINGS.len());
        let bal = c(BALANCE, 0.0, 1.0);
        let (ser, par, split, out1, out2) = match routing {
            1 => (0.999, 0.0, 0.0, 0.0, 0.999),
            2 => (0.0, 0.999, 0.0, ((1.0 - bal) * 1.2).min(0.999), (bal * 1.2).min(0.999)),
            3 => (0.0, 0.0, 0.999, ((1.0 - bal) * 1.2).min(0.999), (bal * 1.2).min(0.999)),
            _ => (0.0, 0.0, 0.0, 0.999, 0.0),
        };
        let (mode1, mode2) = (pick(OSC1_MODE, OSC_MODES.len()), pick(OSC2_MODE, OSC_MODES.len()));
        let flags = (p[SYNC] >= 0.5) as u32
            | ((routing > 0) as u32) << 1
            | ((mode1 == 1) as u32) << 2
            | ((mode1 == 2) as u32) << 3
            | ((mode2 == 1) as u32) << 4
            | ((mode2 == 2) as u32) << 5
            | ((pick(SUB_WAVE, SUB_WAVES.len()) == 1) as u32) << 6
            | ((filter >= 4) as u32) << 7
            | ((routing == 3) as u32) << 8;
        let sat = pick(SATURATION, SATURATIONS.len());
        let flags2 = if sat == 0 { 0 } else { 1u32 << (sat - 1) };
        // The noise: white, and low passed (darker) or less the low pass (brighter).
        let (noise, color) = (c(NOISE, 0.0, 1.0) * 0.35, c(NOISE_COLOR, -1.0, 1.0));
        let (white, dark) = if color < 0.0 { (noise * (1.0 + color), noise * -color * 1.5) } else { (noise, -noise * color) };
        let every = [
            frac((1.0 - shape1) * 0.5),
            frac(shape1 * 0.4),
            frac(1.0 - 2.0 * c(PW1, 0.05, 0.5)),
            frac((1.0 - shape2) * 0.5),
            frac(shape2 * 0.4),
            frac(1.0 - 2.0 * c(PW2, 0.05, 0.5)),
            frac(c(OSC2, 0.0, 1.0) * 0.8),
            frac(c(SUB, 0.0, 1.0) * 0.4),
            frac(white),
            frac(c(RING, 0.0, 1.0)),
            frac(c(FM, 0.0, 1.0) * 0.6),
            frac((1.0 + 15.0 * c(DRIVE, 0.0, 1.0)) / 16.0 * (2.0 / 3.0)),
            frac(c(ENV_AMT, -1.0, 1.0) * 0.45),
            frac(1.0 - 0.985 * c(RESONANCE, 0.0, 1.0)),
            frac(f1[0]),
            frac(f1[1]),
            frac(f1[2]),
            frac(1.0 - 0.985 * c(RES2, 0.0, 1.0)),
            frac(f2[0]),
            frac(f2[1]),
            frac(f2[2]),
            frac(ser),
            frac(par),
            frac(out1),
            frac(out2),
            frac(c(LFO_PITCH, 0.0, 1.0).powi(2) * 0.12),
            frac(c(LFO_CUTOFF, 0.0, 1.0) * 0.25),
            frac(c(LFO_PW, 0.0, 1.0) * 0.8),
            frac(c(LFO_AMP, 0.0, 1.0) * 0.5),
            flags,
            frac((2f32.powf(c(CUTOFF2, -1.0, 1.0) * 3.0) / 8.0).min(0.999)),
            flags2,
        ];
        let base = |b: usize| TABLES + (b as u32) * 8 * TABLE_LEN as u32;
        let long = [
            frac(split),
            frac(c(OSC3, 0.0, 1.0) * 0.8),
            frac((1.0 - shape3) * 0.5),
            frac(shape3 * 0.4),
            frac(0.0),
            frac(pole(1.0 / (std::f32::consts::TAU * 600.0), sr)),
            frac(dark),
            frac(c(ENV_OSC2, -1.0, 1.0) * 0.9),
            frac(c(ENV_FM, 0.0, 1.0) * 0.6),
            frac((1.0 + 15.0 * c(SAT_AMOUNT, 0.0, 1.0)) / 16.0),
            frac(c(RESONANCE, 0.0, 1.0) * 0.97),
            frac(if filter == 5 { 0.9 } else { 0.0 }),
            frac(if filter == 4 { 0.9 } else { 0.0 }),
            frac(c(LFO2_CUTOFF2, 0.0, 1.0) * 0.25),
            frac(c(LFO2_PAN, 0.0, 1.0) * 0.9),
            frac(c(LFO2_FM, 0.0, 1.0) * 0.6),
            frac(c(LFO2_WAVE, 0.0, 1.0)),
            frac(c(WAVE1, 0.0, 1.0)),
            frac(c(WAVE2, 0.0, 1.0)),
            base(pick(TABLE1, BANKS.len())),
            base(pick(TABLE2, BANKS.len())),
            (p[DENSITY].round() as u32).clamp(1, 9),
        ];
        // The hypersaw: each saw's detune (against the note) and level, the louder in the middle.
        let d = (p[DENSITY].round() as usize).clamp(1, 9);
        let spread = c(SAW_SPREAD, 0.0, 1.0).powf(1.5) * 1.2;
        let middle = if d == 1 { 1.0 } else { 1.0 / (d - 1) as f32 + 1e-3 };
        let mut saws = [0u32; 18];
        let mut levels = [0.0f32; 9];
        for (k, level) in levels.iter_mut().enumerate().take(d) {
            let x = if d == 1 { 0.0 } else { -1.0 + 2.0 * k as f32 / (d - 1) as f32 };
            saws[2 * k] = frac(2f32.powf(x * spread / 12.0) - 1.0);
            *level = if x.abs() < middle { 1.0 } else { 0.8 };
        }
        let norm = 0.7 / levels.iter().map(|l| l * l).sum::<f32>().sqrt().max(0.1);
        for k in 0..9 {
            saws[2 * k + 1] = frac(levels[k] * norm);
        }
        chip.write(Area::X, G_SAW1, &every);
        chip.write(Area::X, G_SPLIT, &long);
        chip.write(Area::X, G_HS_TAB, &saws);
        chip.write(Area::X, G_N, &[len as u32, MAX_VOICES as u32]);
        // The LFOs: how far they move a sample, their shapes, how fast S&G glides.
        for (at, rate, shape) in [(LFO1, LFO_RATE, LFO_SHAPE), (LFO2, LFO2_RATE, LFO2_SHAPE)] {
            let hz = c(rate, 0.01, 40.0);
            chip.write(Area::X, at + 1, &[frac(2.0 * hz / sr)]);
            chip.write(Area::X, at + 2, &one_hot::<6>(pick(shape, LFO_SHAPES.len())));
            chip.write(Area::X, at + 10, &[frac(pole(0.25 / hz, sr))]);
        }
        // The effects.
        let flanger = pick(CHORUS_MODE, CHORUS_MODES.len()) == 1;
        let depth = c(CHORUS_DEPTH, 0.0, 1.0);
        let (base_s, depth_s, rate, fb, mix) = if flanger { (0.0025, depth * 0.002, 0.15, 0.7, 0.6) } else { (0.012, depth * 0.005, 0.5, 0.0, 0.8) };
        chip.write(Area::X, CH_MIX, &[frac(c(CHORUS, 0.0, 1.0) * mix), frac(base_s * sr / 2048.0), frac(depth_s * sr / 2048.0)]);
        chip.write(Area::X, CH_INC, &[frac(2.0 * rate / sr)]);
        chip.write(Area::X, CH_FB, &[frac(fb)]);
        let delay_back = (-((c(DELAY_TIME, 0.02, 0.66) * sr).clamp(2.0, 32_000.0) as i32)) as u32 & 0xFF_FFFF;
        chip.write(Area::X, DL_MIX, &[frac(c(DELAY, 0.0, 1.0) * 0.6), delay_back, frac(c(FEEDBACK, 0.0, 0.9)), frac(0.35)]);
        let rv_damp = c(DAMP, 0.0, 1.0) * 0.45;
        chip.write(Area::X, RV_MIX, &[frac(c(REVERB, 0.0, 1.0) * 0.7), frac(0.7 + c(SIZE, 0.0, 1.0) * 0.27), frac(rv_damp), frac(1.0 - rv_damp)]);
        let dist = pick(DISTORTION, DISTORTIONS.len());
        let amount = c(DIST_AMOUNT, 0.0, 1.0);
        let bits = ((12.0 - amount * 10.0).round() as u32).clamp(2, 23);
        let step = !((1u32 << (24 - bits)) - 1) & 0xFF_FFFF;
        let gain = if matches!(dist, 1..=3) { (1.0 + 15.0 * amount) / 16.0 } else { 1.0 / 16.0 };
        chip.write(Area::X, FX_FLAGS, &[if dist == 0 { 0 } else { 1 << (dist - 1) }, frac(gain), step, frac(2.0 * (0.5 - amount * 0.48))]);
        chip.write(Area::X, PH_MIX, &[frac(c(PHASER, 0.0, 1.0) * 0.8)]);
        chip.write(Area::X, PH_MIX + 2, &[frac(2.0 * c(PHASER_RATE, 0.01, 10.0) / sr), frac(c(PHASER_FB, 0.0, 0.9)), frac(-0.82), frac(0.15)]);
        chip.write(Area::X, BO_AMT, &[frac(c(BOOST, 0.0, 1.0) * 0.6), frac(pole(1.0 / (std::f32::consts::TAU * c(BOOST_FREQ, 20.0, 1000.0)), sr))]);
    }

    /// One block on the chip: the knobs and the voices into its memory,
    /// the firmware run, the samples out.
    fn block(&mut self, p: &[f32], len: usize) {
        let sr = self.sr;
        let dt = len as f32 / sr;
        let glide = if p[GLIDE] > 0.002 { 1.0 - (-dt / (p[GLIDE] * 0.3)).exp() } else { 1.0 };
        let detune = p[DETUNE].clamp(0.0, 1.0) * 0.5 + p[SEMITONE].round();
        let semi3 = p[SEMI3].round();
        let (decay, sustain) = (pole(p[DECAY] / 4.6, sr), p[SUSTAIN].clamp(0.0, 0.999));
        let (fdecay, fsustain) = (pole(p[F_DECAY] / 4.6, sr), p[F_SUSTAIN].clamp(0.0, 0.999));
        let (held, held_rate) = slope(sustain, p[SLOPE].clamp(-1.0, 1.0), sr);
        let (fheld, fheld_rate) = slope(fsustain, p[F_SLOPE].clamp(-1.0, 1.0), sr);
        let bend = self.pitch_bend;
        let Some(mut chip) = self.chip.take() else { return };
        self.globals(&mut chip, p, len);
        for v in 0..MAX_VOICES {
            let base = V_BASE + v as u32 * V_WORDS;
            let voice = &mut self.voices[v];
            if voice.stage == Stage::Off {
                continue;
            }
            // The envelopes: from their attack to their decay, then their sustain (sloping), and silent after the release.
            let env = to_frac(chip.read_one(Area::X, base + W_ENV));
            let fenv = to_frac(chip.read_one(Area::X, base + W_FENV));
            match voice.fstage {
                Stage::Attack if fenv >= 0.95 => {
                    voice.fstage = Stage::Decay;
                    chip.write(Area::X, base + W_FTARGET, &[frac(fsustain), frac(fdecay)]);
                }
                Stage::Decay if (fenv - fsustain).abs() < 0.02 => {
                    voice.fstage = Stage::Sustain;
                    chip.write(Area::X, base + W_FTARGET, &[frac(fheld), frac(fheld_rate)]);
                }
                _ => {}
            }
            match voice.stage {
                Stage::Attack if env >= 0.95 => {
                    voice.stage = Stage::Decay;
                    chip.write(Area::X, base + W_TARGET, &[frac(sustain), frac(decay)]);
                }
                Stage::Decay if (env - sustain).abs() < 0.02 => {
                    voice.stage = Stage::Sustain;
                    chip.write(Area::X, base + W_TARGET, &[frac(held), frac(held_rate)]);
                }
                _ => {}
            }
            // Silent: let go of (or sloped all the way down while held).
            let gone = env < 0.0003 && (voice.stage == Stage::Release || (voice.stage == Stage::Sustain && held == 0.0));
            if gone {
                *voice = Voice { stage: Stage::Off, fstage: Stage::Off, gate: false, ..*voice };
                chip.write(Area::X, base + W_ACTIVE, &[0]);
                continue;
            }
            voice.pitch += (voice.target - voice.pitch) * glide;
            let pitch = voice.pitch + bend + voice.detune;
            let (f1, f2, f3) = (midi_to_freq(pitch), midi_to_freq(pitch + detune), midi_to_freq(pitch + semi3));
            let inc = |f: f32| frac((2.0 * f / sr).min(0.95));
            // The cutoff, followed by the key and the velocity; as f/2 for a filter run twice a sample.
            let hz = p[CUTOFF] * 2f32.powf(p[KEY_TRACK] * (voice.pitch - 60.0) / 12.0 + voice.vel_cut);
            let f = (std::f32::consts::PI * hz.clamp(20.0, sr * 0.45) / (2.0 * sr)).sin();
            chip.write(Area::X, base + W_INC1, &[inc(f1)]);
            chip.write(Area::X, base + W_INC2, &[inc(f2)]);
            chip.write(Area::X, base + W_INCS, &[inc(f1 * 0.5)]);
            chip.write(Area::X, base + W_INC3, &[inc(f3)]);
            chip.write(Area::X, base + W_CUT, &[frac(f.clamp(0.0005, 0.48))]);
        }
        let before = chip.cycles();
        chip.run(self.start, self.done, 16_000_000);
        let cycles = chip.cycles().saturating_sub(before) as f32;
        self.mips = self.mips * 0.9 + cycles / dt / 1e6 * 0.1;
        chip.read(Area::Y, OUT_L, &mut self.out_l[..len]);
        chip.read(Area::Y, OUT_R, &mut self.out_r[..len]);
        self.chip = Some(chip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    /// Plays `notes` for `hold` seconds of `secs`: left and right.
    fn play(p: &[f32], notes: &[u8], secs: f32, hold: f32) -> (Vec<f32>, Vec<f32>, SiliconState) {
        let mut st = SiliconState::new(SR);
        assert!(st.ready(), "the chip and its firmware: {:?}", firmware().err());
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

    fn sharp(x: &[f32]) -> f32 {
        rms(&x.windows(2).map(|w| w[1] - w[0]).collect::<Vec<f32>>()) / rms(x).max(1e-9)
    }

    fn differs(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum::<f32>() / a.len().max(1) as f32
    }

    /// A plain saw, the filter open, nothing else.
    fn plain() -> Vec<f32> {
        let mut p = preset("Init");
        (p[OSC2], p[CUTOFF], p[ENV_AMT], p[SUSTAIN], p[RESONANCE], p[KEY_TRACK], p[VEL_CUTOFF]) = (0.0, 12_000.0, 0.0, 0.9, 0.0, 0.0, 0.0);
        p
    }

    #[test]
    fn the_firmware_assembles() {
        let p = firmware().unwrap_or_else(|e| panic!("{e}"));
        assert!(p.words.len() > 300, "{} words", p.words.len());
        assert!(p.at("render_done") > p.at("render_block"));
    }

    #[test]
    fn a_note_sounds_at_its_pitch_from_the_chip() {
        let (out, right, st) = play(&plain(), &[57], 1.0, 1.0);
        let level = rms(&out[4800..]);
        assert!(level > 0.02, "it sounds: {level}");
        // A saw at 220 Hz: about 220 upward crossings a second (the first part left out).
        let crossings = out[4800..].windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count() as f32 / (out.len() - 4800) as f32 * SR;
        assert!((crossings - 220.0).abs() < 12.0, "{crossings}");
        assert!(differs(&out, &right) < 1e-6, "one voice in the middle: both sides alike");
        assert!(st.mips > 1.0, "the chip did the work: {} MIPS", st.mips);
    }

    #[test]
    fn its_release_ends_it_and_the_chip_lets_the_voices_go() {
        let mut p = plain();
        p[RELEASE] = 0.05;
        let (out, _, mut st) = play(&p, &[60, 64, 67], 1.0, 0.3);
        assert!(rms(&out[4800..12_000]) > 0.02);
        assert!(rms(&out[out.len() - 4800..]) < 1e-4, "silent after the release");
        assert!(st.voices.iter().all(|v| v.stage == Stage::Off));
        let chip = st.chip.as_mut().unwrap();
        assert!((0..MAX_VOICES as u32).all(|v| chip.read_one(Area::X, V_BASE + v * V_WORDS + W_ACTIVE) == 0));
    }

    #[test]
    fn unison_takes_voices_and_spreads_them() {
        let mut p = plain();
        (p[UNISON], p[UNI_SPREAD], p[UNI_DETUNE]) = (2.0, 1.0, 0.5);
        let (l, r, st) = play(&p, &[48, 55, 60], 0.3, 1.0);
        assert_eq!(st.voices.iter().filter(|v| v.gate).count(), 6, "three notes, two voices each");
        assert!(differs(&l, &r) > 0.01, "spread across the sides");
    }

    #[test]
    fn the_filter_envelope_closes_the_filter() {
        let mut p = preset("Resonant Pluck");
        (p[SUSTAIN], p[DECAY], p[RESONANCE], p[REVERB]) = (0.9, 1.0, 0.2, 0.0);
        let (out, _, _) = play(&p, &[48], 1.2, 1.2);
        let (early, late) = (sharp(&out[480..4800]), sharp(&out[48_000..52_800]));
        assert!(early > late * 2.0, "{early} {late}");
    }

    #[test]
    fn pulse_width_sync_fm_ring_noise_and_drive_each_change_the_sound() {
        let base = plain();
        let (a, _, _) = play(&base, &[45], 0.4, 0.4);
        let mut changes: Vec<(&str, Vec<f32>)> = Vec::new();
        for (name, k, v) in [("pulse", SHAPE, 1.0), ("noise", NOISE, 1.0), ("drive", DRIVE, 1.0), ("sub", SUB, 1.0)] {
            let mut p = base.clone();
            p[k] = v;
            changes.push((name, play(&p, &[45], 0.4, 0.4).0));
        }
        // A narrow pulse against a square.
        let mut narrow = base.clone();
        (narrow[SHAPE], narrow[PW1]) = (1.0, 0.1);
        let narrow = play(&narrow, &[45], 0.4, 0.4).0;
        assert!(differs(&narrow, &changes[0].1) > 0.005, "width: {}", differs(&narrow, &changes[0].1));
        for (name, k, v) in [("sync", SYNC, 1.0), ("fm", FM, 0.8), ("ring", RING, 1.0)] {
            let mut p = base.clone();
            (p[OSC2], p[SEMITONE], p[k]) = (0.8, 7.0, v);
            let mut q = base.clone();
            (q[OSC2], q[SEMITONE]) = (0.8, 7.0);
            let with = play(&p, &[45], 0.4, 0.4).0;
            let without = play(&q, &[45], 0.4, 0.4).0;
            assert!(differs(&with, &without) > 0.005, "{name}: {}", differs(&with, &without));
        }
        for (name, b) in &changes {
            assert!(differs(&a, b) > 0.005, "{name}: {}", differs(&a, b));
        }
        let noisy = &changes[1].1;
        assert!(sharp(noisy) > sharp(&a) * 2.0, "noise is bright");
    }

    #[test]
    fn filter_two_after_filter_one_darkens_it() {
        let mut p = plain();
        p[CUTOFF] = 6000.0;
        let open = play(&p, &[45], 0.5, 0.5).0;
        (p[ROUTING], p[CUTOFF2], p[FILTER2]) = (1.0, -1.0, 0.0);
        let dark = play(&p, &[45], 0.5, 0.5).0;
        assert!(sharp(&open[4800..]) > sharp(&dark[4800..]) * 2.0, "{} {}", sharp(&open[4800..]), sharp(&dark[4800..]));
        p[ROUTING] = 2.0;
        let beside = play(&p, &[45], 0.5, 0.5).0;
        assert!(rms(&beside[4800..]) > 0.01);
    }

    #[test]
    fn the_lfo_moves_the_pitch_and_the_level() {
        let mut p = plain();
        (p[LFO_RATE], p[LFO_PITCH], p[LFO_SHAPE]) = (4.0, 1.0, 3.0);
        let (out, _, _) = play(&p, &[57], 1.0, 1.0);
        // A square LFO: two pitches a quarter second each.
        let rate = |x: &[f32]| x.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        let parts: Vec<usize> = out[4800..].chunks(3000).map(rate).collect();
        let (lo, hi) = (parts.iter().min().unwrap(), parts.iter().max().unwrap());
        assert!(*hi as f32 > *lo as f32 * 1.15, "{parts:?}");
        let mut q = plain();
        (q[LFO_RATE], q[LFO_AMP], q[LFO_SHAPE]) = (4.0, 1.0, 0.0);
        let (out, _, _) = play(&q, &[57], 1.0, 1.0);
        let levels: Vec<f32> = out[4800..].chunks(2400).map(rms).collect();
        let (lo, hi) = levels.iter().fold((f32::MAX, 0f32), |(a, b), &x| (a.min(x), b.max(x)));
        assert!(hi > lo * 2.0, "tremolo: {levels:?}");
    }

    #[test]
    fn the_chorus_widens_the_delay_echoes_and_the_reverb_rings_on() {
        let mut p = plain();
        p[CHORUS] = 1.0;
        let (l, r, _) = play(&p, &[57], 0.6, 0.6);
        assert!(differs(&l[9600..], &r[9600..]) > 0.005, "the chorus: the sides apart");
        let mut dry = plain();
        (dry[DECAY], dry[SUSTAIN], dry[RELEASE]) = (0.1, 0.0, 0.05);
        let tail = |x: &[f32]| rms(&x[48_000..72_000]);
        let (d, _, _) = play(&dry, &[57], 1.6, 0.1);
        let mut echo = dry.clone();
        (echo[DELAY], echo[DELAY_TIME], echo[FEEDBACK]) = (0.8, 0.25, 0.7);
        let (e, er, _) = play(&echo, &[57], 1.6, 0.1);
        assert!(tail(&e) + tail(&er) > tail(&d) * 10.0 + 1e-4, "echoes: {} {}", tail(&e), tail(&d));
        let mut space = dry.clone();
        (space[REVERB], space[SIZE]) = (1.0, 0.9);
        let (v, _, _) = play(&space, &[57], 1.6, 0.1);
        assert!(rms(&v[24_000..48_000]) > rms(&d[24_000..48_000]) * 10.0 + 1e-4, "the reverb rings on");
    }

    #[test]
    fn every_preset_sounds_and_stays_in_bounds() {
        for name in preset_names() {
            let (l, r, _) = play(&preset(name), &[45, 57], 1.0, 0.6);
            assert!(rms(&l) > 1e-3, "{name} sounds");
            assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() <= 2.0), "{name} stays in bounds");
        }
    }

    #[test]
    fn the_first_silicons_knobs_keep_their_places() {
        // A set saved with the first Silicon: its 22 knobs, the rest as they start.
        let mut old = preset("Init")[..22].to_vec();
        crate::params::fix_len(&mut old, SPECS);
        assert_eq!(old.len(), SPECS.len());
        assert_eq!((SPECS[CUTOFF].name, SPECS[VOLUME].name), ("Cutoff", "Volume"));
        let (out, _, _) = play(&old, &[57], 0.3, 0.3);
        assert!(rms(&out) > 0.01);
    }

    #[test]
    #[ignore]
    fn speed() {
        let mut p = preset("Space Choir");
        p[UNISON] = 2.0;
        let mut st = SiliconState::new(SR);
        for k in [48, 52, 55, 59, 60, 64, 67, 71] {
            st.note_on(&p, k, 0.9);
        }
        let t = std::time::Instant::now();
        let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
        for _ in 0..(SR as usize / 256) {
            st.render(&p, &mut l, &mut r);
        }
        eprintln!("one second of sixteen voices and the effects on the chip: {:?}, {:.1} MIPS", t.elapsed(), st.mips);
    }

    /// How much one rendering differs from another with one knob moved.
    fn moved(base: &[f32], set: &[(usize, f32)], notes: &[u8]) -> f32 {
        let mut p = base.to_vec();
        for &(k, v) in set {
            p[k] = v;
        }
        let (a, _, _) = play(base, notes, 0.4, 0.4);
        let (b, _, _) = play(&p, notes, 0.4, 0.4);
        differs(&a, &b)
    }

    #[test]
    fn the_hypersaw_beats_where_one_saw_does_not() {
        let wobble = |mode: f32| {
            let mut p = plain();
            (p[OSC1_MODE], p[DENSITY], p[SAW_SPREAD]) = (mode, 9.0, 0.8);
            let (l, _, _) = play(&p, &[45], 1.0, 1.0);
            let parts: Vec<f32> = l[4800..].chunks(2400).map(rms).collect();
            let (lo, hi) = parts.iter().fold((f32::MAX, 0f32), |(a, b), &x| (a.min(x), b.max(x)));
            hi / lo.max(1e-6)
        };
        assert!(wobble(1.0) > wobble(0.0) * 1.05, "{} {}", wobble(1.0), wobble(0.0));
    }

    #[test]
    fn wavetables_sound_and_each_table_is_its_own() {
        let mut p = plain();
        p[OSC1_MODE] = 2.0;
        let (a, _, _) = play(&p, &[57], 0.5, 0.5);
        assert!(rms(&a[4800..]) > 0.02, "a wavetable sounds");
        let crossings = a[4800..].windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count() as f32 / (a.len() - 4800) as f32 * SR;
        assert!(crossings > 150.0, "at about its pitch: {crossings}");
        assert!(moved(&p, &[(TABLE1, 1.0)], &[57]) > 0.005, "another table");
        assert!(moved(&p, &[(WAVE1, 0.9)], &[57]) > 0.005, "another wave in it");
        let mut q = plain();
        (q[OSC2_MODE], q[OSC2]) = (2.0, 0.9);
        assert!(moved(&q, &[(TABLE2, 7.0)], &[57]) > 0.005, "osc 2's table");
    }

    #[test]
    fn the_ladder_darkens_and_rings() {
        let mut p = plain();
        (p[FILTER], p[CUTOFF], p[RESONANCE]) = (4.0, 300.0, 0.0);
        let (dark, _, _) = play(&p, &[45], 0.5, 0.5);
        let (open, _, _) = play(&plain(), &[45], 0.5, 0.5);
        assert!(sharp(&open[4800..]) > sharp(&dark[4800..]) * 2.0);
        assert!(rms(&dark[4800..]) > 0.005, "it still sounds");
        p[RESONANCE] = 0.9;
        let (ringing, _, _) = play(&p, &[45], 0.5, 0.5);
        assert!(differs(&dark, &ringing) > 0.003);
        assert!(ringing.iter().all(|v| v.is_finite() && v.abs() <= 2.0));
        p[FILTER] = 5.0;
        assert!(rms(&play(&p, &[45], 0.4, 0.4).0) > 0.005, "12 dB");
    }

    #[test]
    fn every_saturation_distortion_and_routing_changes_the_sound_and_stays_in_bounds() {
        let mut base = plain();
        (base[OSC2], base[SEMITONE]) = (0.6, 7.0);
        for sat in 1..SATURATIONS.len() {
            assert!(moved(&base, &[(SATURATION, sat as f32), (SAT_AMOUNT, 0.8)], &[45]) > 0.003, "saturation {sat}");
        }
        for dist in 1..DISTORTIONS.len() {
            assert!(moved(&base, &[(DISTORTION, dist as f32), (DIST_AMOUNT, 0.8)], &[45]) > 0.003, "distortion {dist}");
        }
        for routing in 1..ROUTINGS.len() {
            let mut p = base.clone();
            (p[ROUTING], p[CUTOFF2], p[RES2], p[FILTER2]) = (routing as f32, -0.6, 0.7, 2.0);
            let (l, r, _) = play(&p, &[45, 52], 0.5, 0.3);
            assert!(rms(&l) > 1e-3, "routing {routing} sounds");
            assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() <= 2.0), "routing {routing} in bounds");
        }
        // Split: filter 2 a high pass for osc 2 only, against both through it.
        assert!(moved(&base, &[(ROUTING, 3.0), (FILTER2, 2.0), (CUTOFF2, 0.8)], &[45]) > 0.003);
    }

    #[test]
    fn the_third_oscillator_the_sub_noise_colour_and_the_envelope_on_osc_2() {
        let base = plain();
        assert!(moved(&base, &[(OSC3, 0.8), (SEMI3, 7.0)], &[45]) > 0.005, "osc 3");
        assert!(moved(&base, &[(SUB, 0.8), (SUB_WAVE, 1.0)], &[45]) > 0.0, "the sub");
        let mut sub = base.clone();
        sub[SUB] = 0.8;
        assert!(moved(&sub, &[(SUB_WAVE, 1.0)], &[45]) > 0.003, "a triangle sub is not a square one");
        let mut noise = base.clone();
        (noise[SHAPE], noise[NOISE], noise[OSC2]) = (0.0, 1.0, 0.0);
        let dark = { let mut p = noise.clone(); p[NOISE_COLOR] = -1.0; play(&p, &[45], 0.4, 0.4).0 };
        let bright = { let mut p = noise.clone(); p[NOISE_COLOR] = 1.0; play(&p, &[45], 0.4, 0.4).0 };
        assert!(sharp(&bright[4800..]) > sharp(&dark[4800..]) * 1.5, "noise colour");
        let mut two = base.clone();
        (two[OSC2], two[SEMITONE], two[F_DECAY], two[F_SUSTAIN]) = (0.9, 7.0, 1.5, 0.5);
        assert!(moved(&two, &[(ENV_OSC2, 0.8)], &[45]) > 0.003, "the envelope on osc 2's pitch");
        assert!(moved(&two, &[(ENV_FM, 0.8)], &[45]) > 0.003, "the envelope on the FM");
    }

    #[test]
    fn the_effects_phaser_flanger_and_boost() {
        let base = plain();
        assert!(moved(&base, &[(PHASER, 1.0), (PHASER_RATE, 1.0)], &[45]) > 0.003, "phaser");
        let chorus = { let mut p = base.clone(); p[CHORUS] = 1.0; play(&p, &[45], 0.4, 0.4).0 };
        let flanger = { let mut p = base.clone(); (p[CHORUS], p[CHORUS_MODE]) = (1.0, 1.0); play(&p, &[45], 0.4, 0.4).0 };
        assert!(differs(&chorus, &flanger) > 0.003, "a flanger is not a chorus");
        let mut low = base.clone();
        (low[CUTOFF], low[OCTAVE]) = (400.0, -1.0);
        let (plainly, _, _) = play(&low, &[36], 0.5, 0.5);
        low[BOOST] = 1.0;
        let (boosted, _, _) = play(&low, &[36], 0.5, 0.5);
        assert!(rms(&boosted[4800..]) > rms(&plainly[4800..]) * 1.2, "the low end boosted");
    }

    #[test]
    fn lfo_two_pans_and_sample_and_hold_steps() {
        let mut p = plain();
        (p[LFO2_PAN], p[LFO2_RATE], p[LFO2_SHAPE]) = (1.0, 3.0, 3.0);
        let (l, r, _) = play(&p, &[57], 1.0, 1.0);
        let side = |x: &[f32], y: &[f32]| x.chunks(2400).zip(y.chunks(2400)).map(|(a, b)| rms(a) - rms(b)).collect::<Vec<f32>>();
        let s = side(&l[4800..], &r[4800..]);
        assert!(s.iter().any(|&d| d > 0.005) && s.iter().any(|&d| d < -0.005), "from side to side: {s:?}");
        let mut q = plain();
        (q[LFO_SHAPE], q[LFO_RATE], q[LFO_PITCH]) = (4.0, 6.0, 1.0);
        let (out, _, _) = play(&q, &[57], 1.0, 1.0);
        let rate = |x: &[f32]| x.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        let parts: Vec<usize> = out[4800..].chunks(1600).map(rate).collect();
        let distinct: std::collections::BTreeSet<usize> = parts.iter().copied().collect();
        assert!(distinct.len() >= 3, "held pitches: {parts:?}");
    }

    #[test]
    fn the_sustain_slopes_up_or_down() {
        let level_at = |slope: f32, sustain: f32| {
            let mut p = plain();
            (p[SUSTAIN], p[DECAY], p[SLOPE]) = (sustain, 0.05, slope);
            let (out, _, _) = play(&p, &[57], 2.0, 2.0);
            (rms(&out[19_200..24_000]), rms(&out[90_000..96_000]))
        };
        let (early, late) = level_at(-0.9, 0.7);
        assert!(late < early * 0.6, "falling: {early} {late}");
        let (early, late) = level_at(0.9, 0.3);
        assert!(late > early * 1.4, "rising: {early} {late}");
        let (early, late) = level_at(0.0, 0.5);
        assert!((late / early - 1.0).abs() < 0.15, "level: {early} {late}");
    }

    #[test]
    fn sets_from_the_second_silicon_keep_their_knobs() {
        let mut old = preset("Unison Lead")[..53].to_vec();
        crate::params::fix_len(&mut old, SPECS);
        assert_eq!(old.len(), SPECS.len());
        assert_eq!((SPECS[REVERB].name, SPECS[DAMP].name, SPECS[OSC1_MODE].name), ("Reverb", "Reverb Damp", "Osc 1 Mode"));
        let (out, _, _) = play(&old, &[57], 0.3, 0.3);
        assert!(rms(&out) > 0.01);
    }
}
