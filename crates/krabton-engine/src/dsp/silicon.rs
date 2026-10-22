//! Silicon: a synth that sounds on the emulated DSP56300 (the chip Hyper's
//! Virus runs on) with Krabton's own firmware, `silicon.asm`: no Virus
//! firmware, nothing of Access's. Every sample is worked out on the chip:
//! an LFO, then per voice two oscillators that turn from a saw into a pulse
//! of any width (the second frequency modulated by the first or synced to
//! it), ring modulation, a sub and noise, a drive, two state variable filters
//! (the second after the first or beside it), a filter and an amplifier
//! envelope; then a chorus, a delay going from side to side and a reverb.
//!
//! This side is the chip's host, as a microcontroller would be: it takes the
//! notes and picks the voices (unison too), glides their pitch, moves the
//! envelopes from attack to decay to release, works out the knobs, writes it
//! all into the chip's X memory, runs the firmware a block at a time and
//! reads the samples back from Y memory.

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
// And the ones after.
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

pub const FILTERS: &[&str] = &["LP", "BP", "HP", "Notch"];
pub const FILTERS2: &[&str] = &["LP", "BP", "HP"];
pub const ROUTINGS: &[&str] = &["Off", "Serial", "Parallel"];
pub const LFO_SHAPES: &[&str] = &["Tri", "Sine", "Saw", "Square"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Shape", "Osc 1", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Osc 2", "Osc 2", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Detune", "Osc 2", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::int("Semitone", "Osc 2", -12.0, 12.0, 0.0, Unit::Semi),
    ParamSpec::lin("Sub", "Osc 1", 0.0, 1.0, 0.0, Unit::Percent),
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
    ParamSpec::log("LFO Rate", "LFO", 0.05, 20.0, 4.0, Unit::Hz),
    ParamSpec::lin("LFO Pitch", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("LFO Cutoff", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Glide", "Play", 0.001, 1.0, 0.001, Unit::Sec),
    ParamSpec::int("Voices", "Play", 1.0, 16.0, 8.0, Unit::None),
    ParamSpec::lin("Velocity", "Play", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Volume", "Play", -30.0, 6.0, -4.0, Unit::Db),
    ParamSpec::lin("Pulse Width", "Osc 1", 0.05, 0.5, 0.5, Unit::Percent),
    ParamSpec::lin("Shape 2", "Osc 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Pulse Width 2", "Osc 2", 0.05, 0.5, 0.5, Unit::Percent),
    ParamSpec::lin("FM", "Osc 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Sync", "Osc 2", false),
    ParamSpec::lin("Ring", "Osc 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Noise", "Osc 1", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Drive", "Filter", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Routing", "Filter 2", ROUTINGS, 0),
    ParamSpec::lin("Cutoff 2", "Filter 2", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Resonance 2", "Filter 2", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::choice("Filter 2", "Filter 2", FILTERS2, 0),
    ParamSpec::log("Env Attack", "Filter Env", 0.001, 5.0, 0.001, Unit::Sec),
    ParamSpec::lin("Env Sustain", "Filter Env", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Env Release", "Filter Env", 0.01, 5.0, 0.3, Unit::Sec),
    ParamSpec::lin("Vel > Cutoff", "Filter", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::choice("LFO Shape", "LFO", LFO_SHAPES, 1),
    ParamSpec::lin("LFO PW", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("LFO Amp", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Unison", "Play", 1.0, 4.0, 1.0, Unit::None),
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
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Chip Bass" => &[(SHAPE, 0.0), (OSC2, 0.4), (DETUNE, 0.1), (SUB, 0.5), (CUTOFF, 300.0), (RESONANCE, 0.55), (ENV_AMT, 0.6), (F_DECAY, 0.2), (DECAY, 0.3), (SUSTAIN, 0.6), (RELEASE, 0.08), (VOICES, 1.0), (GLIDE, 0.05)],
        "Square Lead" => &[(SHAPE, 1.0), (OSC2, 0.3), (DETUNE, 0.2), (SEMITONE, 12.0), (CUTOFF, 3500.0), (RESONANCE, 0.25), (ENV_AMT, 0.25), (F_DECAY, 0.5), (SUSTAIN, 0.9), (RELEASE, 0.2), (LFO_RATE, 5.5), (LFO_PITCH, 0.12), (VOICES, 1.0), (GLIDE, 0.04), (DELAY, 0.25), (DELAY_TIME, 0.3)],
        "Silicon Pad" => &[(SHAPE, 0.3), (OSC2, 0.8), (DETUNE, 0.35), (CUTOFF, 1400.0), (RESONANCE, 0.2), (ENV_AMT, 0.15), (F_DECAY, 1.5), (ATTACK, 0.6), (DECAY, 1.0), (SUSTAIN, 0.85), (RELEASE, 1.5), (LFO_RATE, 0.3), (LFO_CUTOFF, 0.25), (CHORUS, 0.5), (REVERB, 0.4)],
        "Resonant Pluck" => &[(SHAPE, 0.0), (OSC2, 0.3), (CUTOFF, 250.0), (RESONANCE, 0.8), (ENV_AMT, 0.8), (F_DECAY, 0.18), (DECAY, 0.5), (SUSTAIN, 0.0), (RELEASE, 0.3), (REVERB, 0.2)],
        "Hollow" => &[(SHAPE, 0.8), (OSC2, 0.0), (FILTER, 1.0), (CUTOFF, 900.0), (RESONANCE, 0.5), (ENV_AMT, 0.3), (F_DECAY, 0.6), (SUSTAIN, 0.8), (LFO_RATE, 0.5), (LFO_CUTOFF, 0.3)],
        "Sub Drone" => &[(SHAPE, 0.5), (OSC2, 0.6), (DETUNE, 0.5), (SEMITONE, -12.0), (SUB, 0.8), (CUTOFF, 500.0), (RESONANCE, 0.4), (ENV_AMT, 0.0), (ATTACK, 1.5), (SUSTAIN, 1.0), (RELEASE, 2.5), (LFO_RATE, 0.15), (LFO_CUTOFF, 0.4), (REVERB, 0.3)],
        "Sync Sweep" => &[(SHAPE, 0.0), (OSC2, 0.9), (SEMITONE, 7.0), (DETUNE, 0.05), (SYNC, 1.0), (CUTOFF, 2500.0), (RESONANCE, 0.3), (ENV_AMT, 0.4), (F_DECAY, 1.2), (F_SUSTAIN, 0.3), (SUSTAIN, 0.9), (LFO_RATE, 0.4), (LFO_PITCH, 0.0), (VOICES, 1.0), (GLIDE, 0.03)],
        "Bright Brass" => &[(SHAPE, 0.0), (OSC2, 0.7), (DETUNE, 0.25), (CUTOFF, 700.0), (RESONANCE, 0.15), (ENV_AMT, 0.55), (F_ATTACK, 0.06), (F_DECAY, 0.5), (F_SUSTAIN, 0.4), (ATTACK, 0.04), (SUSTAIN, 0.85), (RELEASE, 0.25), (CHORUS, 0.25), (REVERB, 0.2)],
        "Wobble" => &[(SHAPE, 0.6), (OSC2, 0.5), (SEMITONE, -12.0), (SUB, 0.4), (CUTOFF, 350.0), (RESONANCE, 0.6), (ENV_AMT, 0.0), (SUSTAIN, 1.0), (RELEASE, 0.1), (LFO_RATE, 3.0), (LFO_SHAPE, 0.0), (LFO_CUTOFF, 0.8), (VOICES, 1.0), (DRIVE, 0.4)],
        "PWM Strings" => &[(SHAPE, 1.0), (PW1, 0.35), (SHAPE2, 1.0), (PW2, 0.25), (OSC2, 0.7), (DETUNE, 0.2), (LFO_RATE, 0.6), (LFO_SHAPE, 0.0), (LFO_PW, 0.6), (CUTOFF, 3000.0), (RESONANCE, 0.1), (ENV_AMT, 0.1), (ATTACK, 0.35), (SUSTAIN, 0.9), (RELEASE, 0.9), (CHORUS, 0.6), (REVERB, 0.35)],
        "FM Bell" => &[(SHAPE, 0.0), (OSC2, 0.9), (SEMITONE, 7.0), (DETUNE, 0.0), (FM, 0.6), (CUTOFF, 9000.0), (RESONANCE, 0.0), (ENV_AMT, 0.2), (DECAY, 1.8), (SUSTAIN, 0.0), (RELEASE, 1.2), (REVERB, 0.45), (DELAY, 0.15)],
        "Ring Clang" => &[(SHAPE, 0.5), (OSC2, 0.2), (SEMITONE, 5.0), (DETUNE, 0.6), (RING, 1.0), (CUTOFF, 5000.0), (RESONANCE, 0.3), (ENV_AMT, 0.3), (F_DECAY, 0.4), (DECAY, 0.9), (SUSTAIN, 0.0), (RELEASE, 0.6), (REVERB, 0.3)],
        "Noise Sweep" => &[(SHAPE, 0.0), (OSC2, 0.0), (NOISE, 0.9), (FILTER, 1.0), (CUTOFF, 300.0), (RESONANCE, 0.85), (ENV_AMT, 0.8), (F_ATTACK, 0.8), (F_DECAY, 1.5), (F_SUSTAIN, 0.2), (ATTACK, 0.2), (SUSTAIN, 0.8), (RELEASE, 1.0), (REVERB, 0.5), (KEY_TRACK, 1.0)],
        "Unison Lead" => &[(SHAPE, 0.0), (OSC2, 0.5), (DETUNE, 0.1), (UNISON, 4.0), (UNI_DETUNE, 0.35), (UNI_SPREAD, 0.8), (CUTOFF, 4000.0), (RESONANCE, 0.2), (ENV_AMT, 0.2), (SUSTAIN, 0.9), (RELEASE, 0.25), (VOICES, 4.0), (GLIDE, 0.02), (DELAY, 0.3), (DELAY_TIME, 0.375), (REVERB, 0.25)],
        "Dual Filter Bass" => &[(SHAPE, 0.3), (OSC2, 0.6), (DETUNE, 0.15), (SUB, 0.3), (CUTOFF, 600.0), (RESONANCE, 0.5), (ENV_AMT, 0.5), (F_DECAY, 0.25), (ROUTING, 1.0), (CUTOFF2, 0.4), (RES2, 0.6), (FILTER2, 1.0), (DRIVE, 0.3), (SUSTAIN, 0.8), (RELEASE, 0.1), (VOICES, 1.0), (OCTAVE, -1.0)],
        "Echo Pluck" => &[(SHAPE, 0.7), (PW1, 0.3), (OSC2, 0.3), (CUTOFF, 400.0), (RESONANCE, 0.4), (ENV_AMT, 0.7), (F_DECAY, 0.2), (DECAY, 0.35), (SUSTAIN, 0.0), (RELEASE, 0.2), (DELAY, 0.5), (DELAY_TIME, 0.375), (FEEDBACK, 0.55), (REVERB, 0.2)],
        "Space Choir" => &[(SHAPE, 0.4), (OSC2, 0.7), (DETUNE, 0.3), (NOISE, 0.08), (FILTER, 1.0), (CUTOFF, 1200.0), (RESONANCE, 0.45), (ROUTING, 2.0), (CUTOFF2, 0.5), (FILTER2, 1.0), (RES2, 0.5), (ATTACK, 0.8), (SUSTAIN, 1.0), (RELEASE, 2.0), (UNISON, 2.0), (VOICES, 6.0), (CHORUS, 0.7), (REVERB, 0.7), (SIZE, 0.85)],
        "Driven Acid" => &[(SHAPE, 0.0), (OSC2, 0.0), (CUTOFF, 280.0), (RESONANCE, 0.85), (ENV_AMT, 0.75), (F_DECAY, 0.22), (DRIVE, 0.7), (DECAY, 0.4), (SUSTAIN, 0.7), (RELEASE, 0.06), (VOICES, 1.0), (GLIDE, 0.06), (OCTAVE, -1.0), (VEL_CUTOFF, 0.6), (DELAY, 0.2), (DELAY_TIME, 0.28)],
        "Tremolo Keys" => &[(SHAPE, 0.6), (PW1, 0.4), (OSC2, 0.3), (SEMITONE, 12.0), (CUTOFF, 2200.0), (RESONANCE, 0.1), (ENV_AMT, 0.3), (F_DECAY, 0.8), (DECAY, 1.4), (SUSTAIN, 0.3), (RELEASE, 0.4), (LFO_RATE, 5.0), (LFO_SHAPE, 1.0), (LFO_AMP, 0.6), (CHORUS, 0.3), (REVERB, 0.3)],
        _ => &[],
    };
    for &(i, v) in set {
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

// Where the firmware reads the knobs (X memory): every sample from G_SAW1 on,
// once a block from G_N on.
const G_SAW1: u32 = 0x00;
const G_FLAGS: u32 = 0x1d;
const G_N: u32 = 0x40;
const LFO_INC: u32 = 0x43;
const CH_MIX: u32 = 0x48;
const CH_INC: u32 = 0x4c;
const CH_POS: u32 = 0x4d;
const DL_MIX: u32 = 0x50;
const DL_LP_L: u32 = 0x54;
const RV_MIX: u32 = 0x58;
const RV_POS_0: u32 = 0x5c;
// The lines in Y memory where the effects start.
const CH_LINE_L: u32 = 0x1000;
const DL_LINE_L: u32 = 0x10000;
const DL_LINE_R: u32 = 0x18000;
const RV_LINES: [u32; 6] = [0x2000, 0x2800, 0x3000, 0x3800, 0x4000, 0x4400];
const OUT_L: u32 = 0x000;
const OUT_R: u32 = 0x100;
// A voice's 32 words, in X from V_BASE on.
const V_BASE: u32 = 0x100;
const V_WORDS: u32 = 32;
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

/// The firmware assembled (once): its words, and where it starts and waits.
pub fn firmware() -> Result<&'static Program, &'static str> {
    static PROGRAM: OnceLock<Result<Program, String>> = OnceLock::new();
    PROGRAM.get_or_init(|| chip::assemble(FIRMWARE, ORIGIN)).as_ref().map_err(|e| e.as_str())
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Stage {
    Off,
    Attack,
    Decay,
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
            // Where the effects' lines are.
            chip.write(Area::X, CH_POS, &[CH_LINE_L]);
            chip.write(Area::X, DL_LP_L, &[0, 0, DL_LINE_L, DL_LINE_R]);
            chip.write(Area::X, RV_POS_0, &RV_LINES);
            // Every voice and effect once, so the chip compiles its code now and not at the first note.
            for v in 0..MAX_VOICES as u32 {
                let mut w = [0u32; V_WORDS as usize];
                w[W_ACTIVE as usize] = 1;
                w[W_INC1 as usize] = frac(0.01);
                w[W_INC2 as usize] = frac(0.01);
                w[W_INCS as usize] = frac(0.005);
                w[W_TARGET as usize] = frac(0.5);
                w[W_TARGET as usize + 1] = frac(0.01);
                w[W_VEL as usize] = frac(0.1);
                w[W_CUT as usize] = frac(0.1);
                w[W_NOISE as usize] = 0x1234 + v;
                chip.write(Area::X, V_BASE + v * V_WORDS, &w);
            }
            chip.write(Area::X, G_N, &[4, MAX_VOICES as u32]);
            chip.write(Area::X, G_FLAGS, &[3]);
            chip.write(Area::X, CH_MIX, &[frac(0.1), frac(0.2), frac(0.05)]);
            chip.write(Area::X, DL_MIX, &[frac(0.1), (-100i32 as u32) & 0xFF_FFFF, frac(0.3), frac(0.3)]);
            chip.write(Area::X, RV_MIX, &[frac(0.1), frac(0.8), frac(0.2), frac(0.8)]);
            chip.run(st.start, st.done, 1_000_000);
            for v in 0..MAX_VOICES as u32 {
                chip.write(Area::X, V_BASE + v * V_WORDS, &[0; V_WORDS as usize]);
            }
            st.chip = Some(chip);
        }
        st
    }

    /// Whether the chip is there (with its firmware).
    pub fn ready(&self) -> bool {
        self.chip.is_some()
    }

    fn random(&mut self) -> u32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed
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
        (p[UNISON].round() as usize).clamp(1, 4)
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
            let (a, b, s, noise) = (self.random(), self.random(), self.random(), self.random() | 1);
            self.write(v, W_PH1, &[a & 0xFF_FFFF]);
            self.write(v, W_PH2, &[b & 0xFF_FFFF]);
            self.write(v, W_PHS, &[s & 0xFF_FFFF]);
            self.write(v, W_ENV, &[0, 0]);
            self.write(v, W_FENV, &[0, 0]);
            self.write(v, W_LOW, &[0, 0, 0, 0]);
            self.write(v, W_NOISE, &[noise & 0xFF_FFFF]);
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

    /// The knobs as the firmware reads them every sample (from G_SAW1 on).
    fn every_sample(p: &[f32]) -> [u32; 31] {
        let c = |i: usize, lo: f32, hi: f32| p[i].clamp(lo, hi);
        let pick = |i: usize, n: usize| (p[i].round().max(0.0) as usize).min(n - 1);
        let (shape1, shape2) = (c(SHAPE, 0.0, 1.0), c(SHAPE2, 0.0, 1.0));
        let mut f1 = [0.0f32; 3];
        match pick(FILTER, FILTERS.len()) {
            1 => f1[1] = 0.9,
            2 => f1[2] = 0.9,
            3 => (f1[0], f1[2]) = (0.9, 0.9),
            _ => f1[0] = 0.9,
        }
        let mut f2 = [0.0f32; 3];
        f2[pick(FILTER2, FILTERS2.len())] = 0.9;
        let routing = pick(ROUTING, ROUTINGS.len());
        let (ser, par, out1, out2) = match routing {
            1 => (0.999, 0.0, 0.0, 0.999),
            2 => (0.0, 0.999, 0.6, 0.6),
            _ => (0.0, 0.0, 0.999, 0.0),
        };
        let flags = (p[SYNC] >= 0.5) as u32 | ((routing > 0) as u32) << 1;
        [
            frac((1.0 - shape1) * 0.5),
            frac(shape1 * 0.4),
            frac(1.0 - 2.0 * c(PW1, 0.05, 0.5)),
            frac((1.0 - shape2) * 0.5),
            frac(shape2 * 0.4),
            frac(1.0 - 2.0 * c(PW2, 0.05, 0.5)),
            frac(c(OSC2, 0.0, 1.0) * 0.8),
            frac(c(SUB, 0.0, 1.0) * 0.4),
            frac(c(NOISE, 0.0, 1.0) * 0.35),
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
        ]
    }

    /// One block on the chip: the knobs and the voices into its memory,
    /// the firmware run, the samples out.
    fn block(&mut self, p: &[f32], len: usize) {
        let sr = self.sr;
        let dt = len as f32 / sr;
        let every = Self::every_sample(p);
        let pick = |i: usize, n: usize| (p[i].round().max(0.0) as usize).min(n - 1);
        let mut shape = [0.0f32; 4];
        shape[pick(LFO_SHAPE, LFO_SHAPES.len())] = 0.999;
        let glide = if p[GLIDE] > 0.002 { 1.0 - (-dt / (p[GLIDE] * 0.3)).exp() } else { 1.0 };
        let detune = p[DETUNE].clamp(0.0, 1.0) * 0.5 + p[SEMITONE].round();
        let (decay, sustain) = (pole(p[DECAY] / 4.6, sr), p[SUSTAIN].clamp(0.0, 0.999));
        let (fdecay, fsustain) = (pole(p[F_DECAY] / 4.6, sr), p[F_SUSTAIN].clamp(0.0, 0.999));
        let delay_back = (-((p[DELAY_TIME].clamp(0.02, 0.66) * sr).clamp(2.0, 32_000.0) as i32)) as u32 & 0xFF_FFFF;
        let rv_damp = p[DAMP].clamp(0.0, 1.0) * 0.45;
        let bend = self.pitch_bend;
        let Some(chip) = self.chip.as_mut() else { return };
        chip.write(Area::X, G_SAW1, &every);
        // The block's own (the LFO's, the chorus's phases and the lines' places are the chip's).
        chip.write(Area::X, G_N, &[len as u32, MAX_VOICES as u32]);
        chip.write(Area::X, LFO_INC, &[frac(2.0 * p[LFO_RATE].clamp(0.01, 40.0) / sr), frac(shape[0]), frac(shape[1]), frac(shape[2]), frac(shape[3])]);
        chip.write(Area::X, CH_MIX, &[frac(p[CHORUS].clamp(0.0, 1.0) * 0.8), frac(0.012 * sr / 2048.0), frac(p[CHORUS_DEPTH].clamp(0.0, 1.0) * 0.005 * sr / 2048.0)]);
        chip.write(Area::X, CH_INC, &[frac(2.0 * 0.5 / sr)]);
        chip.write(Area::X, DL_MIX, &[frac(p[DELAY].clamp(0.0, 1.0) * 0.6), delay_back, frac(p[FEEDBACK].clamp(0.0, 0.9)), frac(0.35)]);
        chip.write(Area::X, RV_MIX, &[frac(p[REVERB].clamp(0.0, 1.0) * 0.7), frac(0.7 + p[SIZE].clamp(0.0, 1.0) * 0.27), frac(rv_damp), frac(1.0 - rv_damp)]);
        for v in 0..MAX_VOICES {
            let base = V_BASE + v as u32 * V_WORDS;
            let voice = &mut self.voices[v];
            if voice.stage == Stage::Off {
                continue;
            }
            // The envelopes: from their attack to their decay, and silent after the release.
            let env = to_frac(chip.read_one(Area::X, base + W_ENV));
            let fenv = to_frac(chip.read_one(Area::X, base + W_FENV));
            if voice.fstage == Stage::Attack && fenv >= 0.95 {
                voice.fstage = Stage::Decay;
                chip.write(Area::X, base + W_FTARGET, &[frac(fsustain), frac(fdecay)]);
            }
            match voice.stage {
                Stage::Attack if env >= 0.95 => {
                    voice.stage = Stage::Decay;
                    chip.write(Area::X, base + W_TARGET, &[frac(sustain), frac(decay)]);
                }
                Stage::Release if env < 0.0003 => {
                    *voice = Voice { stage: Stage::Off, fstage: Stage::Off, gate: false, ..*voice };
                    chip.write(Area::X, base + W_ACTIVE, &[0]);
                    continue;
                }
                _ => {}
            }
            voice.pitch += (voice.target - voice.pitch) * glide;
            let pitch = voice.pitch + bend + voice.detune;
            let (f1, f2) = (midi_to_freq(pitch), midi_to_freq(pitch + detune));
            let inc = |f: f32| frac((2.0 * f / sr).min(0.95));
            // The cutoff, followed by the key and the velocity; as f/2 for a filter run twice a sample.
            let hz = p[CUTOFF] * 2f32.powf(p[KEY_TRACK] * (voice.pitch - 60.0) / 12.0 + voice.vel_cut);
            let f = (std::f32::consts::PI * hz.clamp(20.0, sr * 0.45) / (2.0 * sr)).sin();
            chip.write(Area::X, base + W_INC1, &[inc(f1)]);
            chip.write(Area::X, base + W_INC2, &[inc(f2)]);
            chip.write(Area::X, base + W_INCS, &[inc(f1 * 0.5)]);
            chip.write(Area::X, base + W_CUT, &[frac(f.clamp(0.0005, 0.48))]);
        }
        let before = chip.cycles();
        chip.run(self.start, self.done, 8_000_000);
        let cycles = chip.cycles().saturating_sub(before) as f32;
        self.mips = self.mips * 0.9 + cycles / dt / 1e6 * 0.1;
        chip.read(Area::Y, OUT_L, &mut self.out_l[..len]);
        chip.read(Area::Y, OUT_R, &mut self.out_r[..len]);
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
}
