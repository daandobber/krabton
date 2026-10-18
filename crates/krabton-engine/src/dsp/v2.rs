//! V2: the synthesizer of farbrausch's intros and games (the farbrausch V2
//! Synthesizer System by Tammo "kb" Hinrichs, 2000-2008, placed in the
//! public domain with the fr_public sources), ported to Rust from the C++
//! version of its synth core in fr_public (v2/synth_core.cpp).
//!
//! A voice: three oscillators (a saw that leans into a triangle by its
//! Color, a pulse of any width, a sine, filtered noise, and FM: a sine
//! phase-modulated by what the oscillators before it make; each can ring
//! instead of add), two filters (the state-variable low, band, high, notch
//! and all-pass, twice oversampled, and a Moog-style low and high), alone,
//! one after the other or side by side, a distortion (overdrive, clip,
//! bitcrusher, decimator, or another filter), two envelopes with a
//! sustain that rises or falls, two LFOs. The channel: a distortion, a
//! chorus/flanger, a compressor and a bass boost; sends to a reverb and a
//! modulated stereo delay. Everything is worked out once a frame (128
//! samples at 44.1 kHz) as V2 does, and the modulation matrix (velocity,
//! the mod wheel, the envelopes, the LFOs, the note into any parameter) is
//! applied then. Parameters are V2's, 0 to 127.
//!
//! Changes from the original: notes sound at the keys' pitch (V2's
//! oscillators play an octave down), the FM sine's range is reduced
//! properly (the original's BUG_V2_FM_RANGE is off), the noise filter
//! keeps its state from frame to frame, the channel filters run on each
//! side, the Moog filters' feedback coefficient has the sign of the Moog
//! VCF they come from (in the C++ port the low pass is all but silent),
//! the reverb's combs keep their low-pass state; the aux buses, the sum
//! compressor and the Ronan speech synth are left out; pitch bend; eight
//! slots of the matrix.

use crate::params::{ParamSpec, Unit, SCREEN_ONLY};

// ---------------------------------------------------------------- parameters

/// The voice's parameters, in V2's order.
pub const PAN: usize = 0;
pub const TRANSPOSE: usize = 1;
/// Oscillator `i`'s parameter: Mode, Ring, Transpose, Detune, Color, Volume.
pub const fn osc(i: usize, k: usize) -> usize {
    2 + 6 * i + k
}
pub const O_MODE: usize = 0;
pub const O_RING: usize = 1;
pub const O_TXPOSE: usize = 2;
pub const O_DETUNE: usize = 3;
pub const O_COLOR: usize = 4;
pub const O_VOLUME: usize = 5;
/// Filter `i`'s parameter: Mode, Cutoff, Reso.
pub const fn flt(i: usize, k: usize) -> usize {
    20 + 3 * i + k
}
pub const ROUTING: usize = 26;
pub const BALANCE: usize = 27;
pub const VDIST: usize = 28;
/// Envelope `i`'s parameter: Attack, Decay, Sustain, Sustain Time, Release, Amplify.
pub const fn env(i: usize, k: usize) -> usize {
    32 + 6 * i + k
}
/// LFO `i`'s parameter: Mode, Key Sync, Env Mode, Rate, Phase, Polarity, Amplify.
pub const fn lfo(i: usize, k: usize) -> usize {
    44 + 7 * i + k
}
pub const OSC_SYNC: usize = 58;
pub const VOICE_N: usize = 59;
/// The channel's.
pub const CHAN_VOL: usize = 59;
pub const REVERB_SEND: usize = 60;
pub const DELAY_SEND: usize = 61;
pub const FX_ROUTE: usize = 62;
pub const BOOST: usize = 63;
pub const CDIST: usize = 64;
pub const CHORUS: usize = 68;
pub const COMP: usize = 75;
pub const CHAN_END: usize = 84;
/// The reverb and delay (V2's globals).
pub const RV_TIME: usize = 84;
pub const RV_HIGHCUT: usize = 85;
pub const RV_LOWCUT: usize = 86;
pub const RV_VOLUME: usize = 87;
pub const DELAY: usize = 88;
pub const LOWCUT: usize = 95;
pub const HIGHCUT: usize = 96;
pub const POLY: usize = 97;
/// Slot `i` of the matrix: Source, Amount (64: none), Destination.
pub const fn modslot(i: usize, k: usize) -> usize {
    98 + 3 * i + k
}
pub const MOD_SLOTS: usize = 8;
pub const MOD_WHEEL: usize = 122;
pub const VOLUME: usize = 123;

pub const OSC_MODES: &[&str] = &["Off", "Saw/Tri", "Pulse", "Sine", "Noise", "FM"];
pub const FLT_MODES: &[&str] = &["Off", "Low", "Band", "High", "Notch", "All", "Moog Low", "Moog High"];
pub const ROUTINGS: &[&str] = &["Single", "Serial", "Parallel"];
pub const DIST_MODES: &[&str] = &["Off", "Overdrive", "Clip", "Crush", "Decimate", "Low", "Band", "High", "Notch", "All", "Moog Low"];
pub const LFO_MODES: &[&str] = &["Saw", "Triangle", "Pulse", "Sine", "S&H"];
pub const POLARITIES: &[&str] = &["+", "-", "+/-"];
pub const OFF_ON: &[&str] = &["Off", "On"];
pub const SYNCS: &[&str] = &["Off", "Osc", "Full"];
pub const FX_ROUTES: &[&str] = &["Dist > Chorus", "Chorus > Dist"];
pub const COMP_MODES: &[&str] = &["Off", "Peak", "RMS"];
pub const COUPLES: &[&str] = &["Mono", "Stereo"];
pub const SOURCES: &[&str] = &["Off", "Velocity", "Mod Wheel", "Breath", "Ctl 3", "Ctl 4", "Ctl 5", "Ctl 6", "Volume", "Amp EG", "EG 2", "LFO 1", "LFO 2", "Note"];

/// The destinations of the matrix: the voice's and the channel's parameters.
pub const DESTS: &[&str] = &[
    "Pan", "Transpose",
    "Osc 1 Mode", "Osc 1 Ring", "Osc 1 Transpose", "Osc 1 Detune", "Osc 1 Color", "Osc 1 Volume",
    "Osc 2 Mode", "Osc 2 Ring", "Osc 2 Transpose", "Osc 2 Detune", "Osc 2 Color", "Osc 2 Volume",
    "Osc 3 Mode", "Osc 3 Ring", "Osc 3 Transpose", "Osc 3 Detune", "Osc 3 Color", "Osc 3 Volume",
    "VCF 1 Mode", "VCF 1 Cutoff", "VCF 1 Reso", "VCF 2 Mode", "VCF 2 Cutoff", "VCF 2 Reso",
    "Routing", "Balance", "Dist Mode", "Dist In Gain", "Dist Param 1", "Dist Param 2",
    "Amp EG Attack", "Amp EG Decay", "Amp EG Sustain", "Amp EG Sus Time", "Amp EG Release", "Amp EG Amplify",
    "EG 2 Attack", "EG 2 Decay", "EG 2 Sustain", "EG 2 Sus Time", "EG 2 Release", "EG 2 Amplify",
    "LFO 1 Mode", "LFO 1 Key Sync", "LFO 1 Env Mode", "LFO 1 Rate", "LFO 1 Phase", "LFO 1 Polarity", "LFO 1 Amplify",
    "LFO 2 Mode", "LFO 2 Key Sync", "LFO 2 Env Mode", "LFO 2 Rate", "LFO 2 Phase", "LFO 2 Polarity", "LFO 2 Amplify",
    "Osc Sync",
    "Channel Volume", "Reverb Send", "Delay Send", "FX Route", "Boost",
    "Ch Dist Mode", "Ch Dist In Gain", "Ch Dist Param 1", "Ch Dist Param 2",
    "Chorus Amount", "Chorus Feedback", "Chorus Delay L", "Chorus Delay R", "Chorus Rate", "Chorus Depth", "Chorus Phase",
    "Comp Mode", "Comp Couple", "Comp Auto Gain", "Comp Lookahead", "Comp Threshold", "Comp Ratio", "Comp Attack", "Comp Release", "Comp Out Gain",
];

const fn v(name: &'static str, group: &'static str, default: f32) -> ParamSpec {
    ParamSpec::int(name, group, 0.0, 127.0, default, Unit::None)
}

pub static SPECS: &[ParamSpec] = &[
    v("Pan", "Voice", 64.0),
    v("Transpose", "Voice", 64.0),
    ParamSpec::choice("Osc 1 Mode", "Osc 1", OSC_MODES, 1),
    ParamSpec::choice("Osc 1 Ring", SCREEN_ONLY, OFF_ON, 0),
    v("Osc 1 Transpose", "Osc 1", 64.0),
    v("Osc 1 Detune", "Osc 1", 64.0),
    v("Osc 1 Color", "Osc 1", 0.0),
    v("Osc 1 Volume", "Osc 1", 127.0),
    ParamSpec::choice("Osc 2 Mode", "Osc 2", OSC_MODES, 0),
    ParamSpec::choice("Osc 2 Ring", "Osc 2", OFF_ON, 0),
    v("Osc 2 Transpose", "Osc 2", 64.0),
    v("Osc 2 Detune", "Osc 2", 64.0),
    v("Osc 2 Color", "Osc 2", 32.0),
    v("Osc 2 Volume", "Osc 2", 127.0),
    ParamSpec::choice("Osc 3 Mode", "Osc 3", OSC_MODES, 0),
    ParamSpec::choice("Osc 3 Ring", "Osc 3", OFF_ON, 0),
    v("Osc 3 Transpose", "Osc 3", 64.0),
    v("Osc 3 Detune", "Osc 3", 64.0),
    v("Osc 3 Color", "Osc 3", 32.0),
    v("Osc 3 Volume", "Osc 3", 127.0),
    ParamSpec::choice("VCF 1 Mode", "VCF 1", FLT_MODES, 1),
    v("VCF 1 Cutoff", "VCF 1", 127.0),
    v("VCF 1 Reso", "VCF 1", 0.0),
    ParamSpec::choice("VCF 2 Mode", "VCF 2", FLT_MODES, 0),
    v("VCF 2 Cutoff", "VCF 2", 64.0),
    v("VCF 2 Reso", "VCF 2", 0.0),
    ParamSpec::choice("Routing", "Filters", ROUTINGS, 0),
    v("Balance", "Filters", 64.0),
    ParamSpec::choice("Dist Mode", "Voice Dist", DIST_MODES, 0),
    v("Dist In Gain", "Voice Dist", 32.0),
    v("Dist Param 1", "Voice Dist", 0.0),
    v("Dist Param 2", "Voice Dist", 64.0),
    v("Amp Attack", "Amp EG", 0.0),
    v("Amp Decay", "Amp EG", 64.0),
    v("Amp Sustain", "Amp EG", 127.0),
    v("Amp Sus Time", "Amp EG", 64.0),
    v("Amp Release", "Amp EG", 80.0),
    v("Amp Amplify", "Amp EG", 0.0),
    v("EG2 Attack", "EG 2", 0.0),
    v("EG2 Decay", "EG 2", 64.0),
    v("EG2 Sustain", "EG 2", 127.0),
    v("EG2 Sus Time", "EG 2", 64.0),
    v("EG2 Release", "EG 2", 80.0),
    v("EG2 Amplify", "EG 2", 64.0),
    ParamSpec::choice("LFO1 Mode", "LFO 1", LFO_MODES, 1),
    ParamSpec::choice("LFO1 Key Sync", "LFO 1", OFF_ON, 1),
    ParamSpec::choice("LFO1 Env Mode", "LFO 1", OFF_ON, 0),
    v("LFO1 Rate", "LFO 1", 64.0),
    v("LFO1 Phase", "LFO 1", 2.0),
    ParamSpec::choice("LFO1 Polarity", "LFO 1", POLARITIES, 0),
    v("LFO1 Amplify", "LFO 1", 0.0),
    ParamSpec::choice("LFO2 Mode", "LFO 2", LFO_MODES, 1),
    ParamSpec::choice("LFO2 Key Sync", "LFO 2", OFF_ON, 1),
    ParamSpec::choice("LFO2 Env Mode", "LFO 2", OFF_ON, 0),
    v("LFO2 Rate", "LFO 2", 64.0),
    v("LFO2 Phase", "LFO 2", 2.0),
    ParamSpec::choice("LFO2 Polarity", "LFO 2", POLARITIES, 0),
    v("LFO2 Amplify", "LFO 2", 127.0),
    ParamSpec::choice("Osc Sync", "Voice", SYNCS, 0),
    v("Channel Volume", "Channel", 0.0),
    v("Reverb Send", "Channel", 0.0),
    v("Delay Send", "Channel", 0.0),
    ParamSpec::choice("FX Route", "Channel", FX_ROUTES, 0),
    v("Boost", "Channel", 0.0),
    ParamSpec::choice("Ch Dist Mode", "Ch Dist", DIST_MODES, 0),
    v("Ch Dist In Gain", "Ch Dist", 32.0),
    v("Ch Dist Param 1", "Ch Dist", 100.0),
    v("Ch Dist Param 2", "Ch Dist", 64.0),
    v("Chorus Amount", "Chorus", 64.0),
    v("Chorus Feedback", "Chorus", 64.0),
    v("Chorus Delay L", "Chorus", 32.0),
    v("Chorus Delay R", "Chorus", 32.0),
    v("Chorus Rate", "Chorus", 0.0),
    v("Chorus Depth", "Chorus", 0.0),
    v("Chorus Phase", "Chorus", 64.0),
    ParamSpec::choice("Comp Mode", "Compressor", COMP_MODES, 0),
    ParamSpec::choice("Comp Couple", "Compressor", COUPLES, 0),
    ParamSpec::choice("Comp Auto Gain", "Compressor", OFF_ON, 1),
    ParamSpec::int("Comp Lookahead", "Compressor", 0.0, 10.0, 2.0, Unit::Ms),
    v("Comp Threshold", "Compressor", 90.0),
    v("Comp Ratio", "Compressor", 32.0),
    v("Comp Attack", "Compressor", 20.0),
    v("Comp Release", "Compressor", 64.0),
    v("Comp Out Gain", "Compressor", 64.0),
    v("Reverb Time", "Reverb", 64.0),
    v("Reverb High Cut", "Reverb", 64.0),
    v("Reverb Low Cut", "Reverb", 32.0),
    v("Reverb Volume", "Reverb", 127.0),
    v("Delay Volume", "Delay", 100.0),
    v("Delay Feedback", "Delay", 80.0),
    v("Delay L", "Delay", 64.0),
    v("Delay R", "Delay", 64.0),
    v("Delay Mod Rate", "Delay", 0.0),
    v("Delay Mod Depth", "Delay", 0.0),
    v("Delay Mod Phase", "Delay", 64.0),
    v("Low Cut", "Output", 0.0),
    v("High Cut", "Output", 127.0),
    ParamSpec::int("Polyphony", "Voice", 1.0, 16.0, 8.0, Unit::None),
    ParamSpec::choice("Mod 1 Source", "Matrix", SOURCES, 1),
    v("Mod 1 Amount", "Matrix", 127.0),
    ParamSpec::choice("Mod 1 Dest", "Matrix", DESTS, 37),
    ParamSpec::choice("Mod 2 Source", "Matrix", SOURCES, 2),
    v("Mod 2 Amount", "Matrix", 127.0),
    ParamSpec::choice("Mod 2 Dest", "Matrix", DESTS, 50),
    ParamSpec::choice("Mod 3 Source", "Matrix", SOURCES, 11),
    v("Mod 3 Amount", "Matrix", 65.0),
    ParamSpec::choice("Mod 3 Dest", "Matrix", DESTS, 1),
    ParamSpec::choice("Mod 4 Source", "Matrix", SOURCES, 8),
    v("Mod 4 Amount", "Matrix", 127.0),
    ParamSpec::choice("Mod 4 Dest", "Matrix", DESTS, 59),
    ParamSpec::choice("Mod 5 Source", "Matrix", SOURCES, 0),
    v("Mod 5 Amount", "Matrix", 64.0),
    ParamSpec::choice("Mod 5 Dest", "Matrix", DESTS, 21),
    ParamSpec::choice("Mod 6 Source", "Matrix", SOURCES, 0),
    v("Mod 6 Amount", "Matrix", 64.0),
    ParamSpec::choice("Mod 6 Dest", "Matrix", DESTS, 21),
    ParamSpec::choice("Mod 7 Source", "Matrix", SOURCES, 0),
    v("Mod 7 Amount", "Matrix", 64.0),
    ParamSpec::choice("Mod 7 Dest", "Matrix", DESTS, 21),
    ParamSpec::choice("Mod 8 Source", "Matrix", SOURCES, 0),
    v("Mod 8 Amount", "Matrix", 64.0),
    ParamSpec::choice("Mod 8 Dest", "Matrix", DESTS, 21),
    v("Mod Wheel", "Play", 0.0),
    ParamSpec::lin("Volume", "Play", -24.0, 6.0, 0.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Init", "Fat Lead", "Super Pad", "Moog Bass", "Acid", "Pluck", "Brass", "Strings", "Wobble Bass", "FM Bell", "Chip Lead", "Organ", "Choir", "Kick", "Snare", "Hat", "Sweep FX"]
}

/// (slot, source, amount, destination)
type Mod = (usize, usize, f32, usize);

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let o = |i, k| osc(i, k);
    let (set, mods): (Vec<(usize, f32)>, Vec<Mod>) = match name {
        "Fat Lead" => (
            vec![(o(1, O_MODE), 1.0), (o(1, O_DETUNE), 72.0), (o(2, O_MODE), 1.0), (o(2, O_DETUNE), 57.0), (o(2, O_TXPOSE), 52.0), (o(0, O_VOLUME), 100.0), (o(1, O_VOLUME), 100.0), (o(2, O_VOLUME), 90.0), (flt(0, 0), 6.0), (flt(0, 1), 80.0), (flt(0, 2), 50.0), (env(1, 0), 0.0), (env(1, 1), 70.0), (env(1, 2), 40.0), (CHORUS, 90.0), (CHORUS + 4, 20.0), (CHORUS + 5, 30.0), (DELAY_SEND, 40.0), (POLY, 1.0)],
            vec![(4, 10, 100.0, flt(0, 1))],
        ),
        "Super Pad" => (
            vec![(o(0, O_COLOR), 10.0), (o(1, O_MODE), 1.0), (o(1, O_COLOR), 10.0), (o(1, O_DETUNE), 76.0), (o(2, O_MODE), 1.0), (o(2, O_COLOR), 10.0), (o(2, O_DETUNE), 52.0), (o(0, O_VOLUME), 90.0), (o(1, O_VOLUME), 90.0), (o(2, O_VOLUME), 90.0), (flt(0, 1), 70.0), (flt(0, 2), 30.0), (env(0, 0), 60.0), (env(0, 4), 100.0), (env(1, 0), 70.0), (env(1, 1), 90.0), (env(1, 2), 60.0), (CHORUS, 100.0), (CHORUS + 4, 15.0), (CHORUS + 5, 50.0), (REVERB_SEND, 70.0), (lfo(1, 3), 30.0)],
            vec![(4, 10, 90.0, flt(0, 1)), (5, 12, 70.0, flt(0, 1))],
        ),
        "Moog Bass" => (
            vec![(TRANSPOSE, 52.0), (o(1, O_MODE), 2.0), (o(1, O_COLOR), 64.0), (o(1, O_TXPOSE), 52.0), (o(1, O_VOLUME), 90.0), (flt(0, 0), 6.0), (flt(0, 1), 40.0), (flt(0, 2), 70.0), (env(0, 1), 70.0), (env(0, 2), 100.0), (env(0, 4), 40.0), (env(1, 1), 50.0), (env(1, 2), 0.0), (VDIST, 1.0), (VDIST + 1, 50.0), (VDIST + 2, 100.0), (POLY, 1.0), (BOOST, 40.0)],
            vec![(4, 10, 110.0, flt(0, 1))],
        ),
        "Acid" => (
            vec![(TRANSPOSE, 52.0), (o(0, O_COLOR), 0.0), (flt(0, 0), 6.0), (flt(0, 1), 30.0), (flt(0, 2), 110.0), (env(0, 1), 60.0), (env(0, 2), 90.0), (env(1, 1), 45.0), (env(1, 2), 0.0), (VDIST, 1.0), (VDIST + 1, 70.0), (VDIST + 2, 90.0), (POLY, 1.0), (DELAY_SEND, 30.0)],
            vec![(4, 10, 115.0, flt(0, 1)), (5, 1, 90.0, flt(0, 1))],
        ),
        "Pluck" => (
            vec![(o(0, O_COLOR), 20.0), (o(1, O_MODE), 2.0), (o(1, O_COLOR), 40.0), (o(1, O_TXPOSE), 76.0), (o(1, O_VOLUME), 60.0), (flt(0, 1), 30.0), (flt(0, 2), 40.0), (env(0, 1), 55.0), (env(0, 2), 0.0), (env(0, 4), 60.0), (env(1, 1), 40.0), (env(1, 2), 0.0), (REVERB_SEND, 40.0), (DELAY_SEND, 40.0)],
            vec![(4, 10, 120.0, flt(0, 1))],
        ),
        "Brass" => (
            vec![(o(0, O_VOLUME), 90.0), (o(1, O_MODE), 1.0), (o(1, O_DETUNE), 70.0), (o(1, O_VOLUME), 80.0), (flt(0, 1), 50.0), (flt(0, 2), 20.0), (env(0, 0), 20.0), (env(0, 2), 110.0), (env(1, 0), 35.0), (env(1, 1), 70.0), (env(1, 2), 50.0), (REVERB_SEND, 40.0), (CHORUS, 80.0), (CHORUS + 5, 20.0), (CHORUS + 4, 10.0)],
            vec![(4, 10, 105.0, flt(0, 1))],
        ),
        "Strings" => (
            vec![(o(0, O_VOLUME), 90.0), (o(0, O_COLOR), 40.0), (o(1, O_MODE), 1.0), (o(1, O_COLOR), 40.0), (o(1, O_DETUNE), 68.0), (o(1, O_TXPOSE), 76.0), (o(1, O_VOLUME), 55.0), (flt(0, 1), 80.0), (env(0, 0), 50.0), (env(0, 4), 90.0), (CHORUS, 110.0), (CHORUS + 4, 25.0), (CHORUS + 5, 60.0), (REVERB_SEND, 80.0), (lfo(0, 3), 70.0)],
            vec![(2, 11, 66.0, TRANSPOSE)],
        ),
        "Wobble Bass" => (
            vec![(TRANSPOSE, 52.0), (o(1, O_MODE), 1.0), (o(1, O_DETUNE), 70.0), (o(1, O_TXPOSE), 52.0), (flt(0, 0), 6.0), (flt(0, 1), 20.0), (flt(0, 2), 90.0), (lfo(1, 3), 72.0), (lfo(1, 0), 3.0), (VDIST, 2.0), (VDIST + 1, 60.0), (VDIST + 2, 100.0), (POLY, 1.0), (BOOST, 50.0)],
            vec![(4, 12, 120.0, flt(0, 1))],
        ),
        "FM Bell" => (
            vec![(o(0, O_MODE), 3.0), (o(0, O_TXPOSE), 83.0), (o(0, O_VOLUME), 50.0), (o(1, O_MODE), 5.0), (o(1, O_VOLUME), 120.0), (flt(0, 0), 0.0), (env(0, 1), 85.0), (env(0, 2), 0.0), (env(0, 4), 85.0), (REVERB_SEND, 70.0)],
            vec![(4, 10, 40.0, o(0, O_VOLUME))],
        ),
        "Chip Lead" => (
            vec![(o(0, O_MODE), 2.0), (o(0, O_COLOR), 32.0), (flt(0, 0), 0.0), (VDIST, 3.0), (VDIST + 1, 40.0), (VDIST + 2, 6.0), (VDIST + 3, 0.0), (POLY, 1.0), (DELAY_SEND, 50.0), (lfo(0, 3), 76.0), (lfo(0, 0), 3.0)],
            vec![(4, 2, 80.0, o(0, O_COLOR))],
        ),
        "Organ" => (
            vec![(o(0, O_MODE), 3.0), (o(1, O_MODE), 3.0), (o(1, O_TXPOSE), 76.0), (o(1, O_VOLUME), 90.0), (o(2, O_MODE), 3.0), (o(2, O_TXPOSE), 83.0), (o(2, O_VOLUME), 70.0), (flt(0, 0), 0.0), (env(0, 4), 30.0), (CHORUS, 100.0), (CHORUS + 4, 50.0), (CHORUS + 5, 30.0), (REVERB_SEND, 50.0)],
            vec![],
        ),
        "Choir" => (
            vec![(o(0, O_COLOR), 64.0), (o(1, O_MODE), 1.0), (o(1, O_COLOR), 64.0), (o(1, O_DETUNE), 70.0), (o(2, O_MODE), 4.0), (o(2, O_VOLUME), 25.0), (o(2, O_COLOR), 90.0), (flt(0, 0), 2.0), (flt(0, 1), 70.0), (flt(0, 2), 80.0), (flt(1, 0), 2.0), (flt(1, 1), 92.0), (flt(1, 2), 80.0), (ROUTING, 2.0), (env(0, 0), 60.0), (env(0, 4), 95.0), (CHORUS, 110.0), (CHORUS + 4, 20.0), (CHORUS + 5, 60.0), (REVERB_SEND, 90.0)],
            vec![(4, 12, 70.0, flt(0, 1))],
        ),
        "Kick" => (
            vec![(o(0, O_MODE), 3.0), (TRANSPOSE, 40.0), (flt(0, 0), 0.0), (env(0, 1), 62.0), (env(0, 2), 0.0), (env(0, 4), 50.0), (env(1, 1), 40.0), (env(1, 2), 0.0), (VDIST, 1.0), (VDIST + 1, 50.0), (VDIST + 2, 110.0), (POLY, 1.0)],
            vec![(4, 10, 100.0, TRANSPOSE)],
        ),
        "Snare" => (
            vec![(o(0, O_MODE), 3.0), (o(0, O_VOLUME), 80.0), (o(1, O_MODE), 4.0), (o(1, O_COLOR), 90.0), (o(1, O_TXPOSE), 90.0), (flt(0, 0), 3.0), (flt(0, 1), 60.0), (env(0, 1), 50.0), (env(0, 2), 0.0), (env(0, 4), 40.0), (env(1, 1), 30.0), (env(1, 2), 0.0), (REVERB_SEND, 30.0)],
            vec![(4, 10, 90.0, TRANSPOSE)],
        ),
        "Hat" => (
            vec![(o(0, O_MODE), 4.0), (o(0, O_COLOR), 100.0), (o(0, O_TXPOSE), 110.0), (flt(0, 0), 3.0), (flt(0, 1), 88.0), (env(0, 1), 48.0), (env(0, 2), 0.0), (env(0, 4), 40.0)],
            vec![],
        ),
        "Sweep FX" => (
            vec![(o(0, O_MODE), 4.0), (o(0, O_COLOR), 60.0), (o(0, O_VOLUME), 90.0), (o(1, O_MODE), 1.0), (o(1, O_VOLUME), 80.0), (flt(0, 0), 2.0), (flt(0, 1), 20.0), (flt(0, 2), 80.0), (env(0, 0), 70.0), (env(0, 4), 110.0), (lfo(1, 3), 40.0), (lfo(1, 0), 3.0), (REVERB_SEND, 70.0), (DELAY_SEND, 60.0)],
            vec![(4, 12, 127.0, flt(0, 1))],
        ),
        _ => (vec![], vec![]),
    };
    for (i, x) in set {
        p[i] = x;
    }
    for (slot, src, amount, dest) in mods {
        p[modslot(slot, 0)] = src as f32;
        p[modslot(slot, 1)] = amount;
        p[modslot(slot, 2)] = dest as f32;
    }
    p
}

// ---------------------------------------------------------------- constants

const FCLOWEST: f32 = 1.220_703_1e-4;
const FCPI_2: f32 = std::f32::consts::FRAC_PI_2;
const FCPI: f32 = std::f32::consts::PI;
const FC1P5PI: f32 = 4.712_389;
const FC2PI: f32 = std::f32::consts::TAU;
const FC32BIT: f32 = 2_147_483_648.0;
/// The oscillators' base: middle C.
const FCOSCBASE: f32 = 261.625_58;
const FCSRBASE: f32 = 44100.0;
const FCBOOSTFREQ: f32 = 150.0;
const FCFRAMEBASE: f32 = 128.0;
const FCDCFLT: f32 = 126.0;
const FCCFFRAME: f32 = 11.0;
const FCFMMAX: f32 = 2.0;
const FCATTACKMUL: f32 = -0.093_75;
const FCATTACKADD: f32 = 7.0;
const FCSUSMUL: f32 = 0.001_937_5;
const FCGAIN: f32 = 0.6;
const FCGAINH: f32 = 0.6;
const FCMDLFOMUL: f32 = 1_973_915.5;
const FCCPDFALLOFF: f32 = 0.9998;
const FCDCOFFSET: f32 = 3.814_697_3e-6;

// ---------------------------------------------------------------- helpers

fn fastatan(x: f32) -> f32 {
    let (sign, x) = if x < 0.0 { (-1.0, -x) } else { (1.0, x) };
    const C: [[f32; 6]; 2] = [[1.0, 0.431_579_74, 1.0, 0.058_319_38, 0.764_439_45, 0.0], [-0.431_597_97, -1.0, 0.058_319_38, 1.0, 0.764_439_45, 1.570_796_3]];
    let c = &C[(x >= 1.0) as usize];
    let x2 = x * x;
    let r = (c[1] * x2 + c[0]) * x / ((c[4] * x2 + c[3]) * x2 + c[2]) + c[5];
    r * sign
}

/// Sine for x in [-pi/2, pi/2] (an odd minimax polynomial).
fn fastsin(x: f32) -> f32 {
    let x2 = x * x;
    (((-0.000_185_42 * x2 + 0.008_314_3) * x2 - 0.166_66) * x2 + 1.0) * x
}

/// Sine of any x (reduced to [-pi/2, pi/2]; the original's bug with
/// negative x is not kept).
fn fastsinrc(x: f32) -> f32 {
    let mut x = x % FC2PI;
    if x < 0.0 {
        x += FC2PI;
    }
    if x > FC1P5PI {
        x -= FC2PI;
    } else if x > FCPI_2 {
        x = FCPI - x;
    }
    fastsin(x)
}

fn calcfreq(x: f32) -> f32 {
    2f32.powf((x - 1.0) * 10.0)
}

fn calcfreq2(x: f32) -> f32 {
    2f32.powf((x - 1.0) * FCCFFRAME)
}

fn urandom(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(196_314_165).wrapping_add(907_633_515);
    *seed
}

/// Uniform in [-1, 1).
fn frandom(seed: &mut u32) -> f32 {
    let bits = urandom(seed);
    f32::from_bits((bits >> 9) | 0x4000_0000) - 3.0
}

/// A 32-bit counter as a float in [0, 1).
fn utof23(x: u32) -> f32 {
    f32::from_bits((x >> 9) | 0x3f80_0000) - 1.0
}

/// [0, 1) into 0.32 fixed point.
fn ftou32(v: f32) -> u32 {
    2u32.wrapping_mul((v * FC32BIT) as i32 as u32)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

// ---------------------------------------------------------------- building blocks

/// The sample-rate-dependent constants.
#[derive(Clone, Copy)]
struct Inst {
    samples_per_ms: f32,
    obasefrq: f32,
    linfreq: f32,
    boost_cos: f32,
    boost_sin: f32,
    dcfilter: f32,
    frame: usize,
    iframe: f32,
}

impl Inst {
    fn new(sr: f32) -> Self {
        let frame = ((FCFRAMEBASE * sr / FCSRBASE + 0.5) as usize).max(1);
        let boost = FCBOOSTFREQ * FC2PI / sr;
        Self {
            samples_per_ms: sr / 1000.0,
            // (2^32: notes at the keys' pitch; V2 has 2^31, an octave down)
            obasefrq: FCOSCBASE * FC32BIT * 2.0 / sr,
            linfreq: FCSRBASE / sr,
            boost_cos: boost.cos(),
            boost_sin: boost.sin(),
            dcfilter: 1.0 - FCDCFLT / sr,
            frame,
            iframe: 1.0 / frame as f32,
        }
    }
}

/// The state-variable filter: low and band as its state, high returned.
#[derive(Clone, Copy, Default)]
struct Lrc {
    l: f32,
    b: f32,
}

impl Lrc {
    fn step(&mut self, input: f32, freq: f32, reso: f32) -> f32 {
        self.l += freq * self.b;
        let h = input - self.b * reso - self.l;
        self.b += freq * h;
        h
    }

    fn step_2x(&mut self, input: f32, freq: f32, reso: f32) -> f32 {
        let input = input + FCDCOFFSET;
        self.l += freq * self.b - FCDCOFFSET;
        self.b += freq * (input - self.b * reso - self.l);
        self.l += freq * self.b;
        let h = input - self.b * reso - self.l;
        self.b += freq * h;
        h
    }
}

#[derive(Clone, Copy, Default)]
struct Moog {
    b: [f32; 5],
}

impl Moog {
    fn step(&mut self, realin: f32, f: f32, p: f32, q: f32) -> f32 {
        let mut input = realin + FCDCOFFSET;
        input -= q * self.b[4];
        let t1 = self.b[1];
        self.b[1] = (input + self.b[0]) * p - self.b[1] * f;
        let t2 = self.b[2];
        self.b[2] = (t1 + self.b[1]) * p - self.b[2] * f;
        let t3 = self.b[3];
        self.b[3] = (t2 + self.b[2]) * p - self.b[3] * f;
        let mut b4 = (t3 + self.b[3]) * p - self.b[4] * f;
        b4 -= b4 * b4 * b4 * (1.0 / 6.0);
        b4 -= FCDCOFFSET;
        self.b[4] = b4 - FCDCOFFSET;
        self.b[0] = realin;
        b4
    }
}

#[derive(Clone, Copy, Default)]
struct Dcf {
    xm1: f32,
    ym1: f32,
}

impl Dcf {
    fn step(&mut self, input: f32, r: f32) -> f32 {
        let y = (FCDCOFFSET + r * self.ym1 - self.xm1 + input) - FCDCOFFSET;
        self.xm1 = input;
        self.ym1 = y;
        y
    }
}

// ---------------------------------------------------------------- oscillator

#[derive(Clone, Copy, Default)]
struct Osc {
    mode: usize,
    ring: bool,
    cnt: u32,
    freq: u32,
    brpt: u32,
    nffrq: f32,
    nfres: f32,
    nseed: u32,
    gain: f32,
    nf: Lrc,
    note: f32,
    pitch: f32,
}

const SEEDS: [u32; 3] = [0xdead_beef, 0xbaad_f00d, 0xd3ad_c0de];

impl Osc {
    fn init(&mut self, idx: usize) {
        self.cnt = 0;
        self.nf = Lrc::default();
        self.nseed = SEEDS[idx];
    }

    fn chg_pitch(&mut self, inst: &Inst) {
        self.nffrq = inst.linfreq * calcfreq((self.pitch + 64.0) / 128.0);
        let f = inst.obasefrq * 2f32.powf((self.pitch + self.note - 60.0) / 12.0);
        // (below half the sample rate)
        self.freq = f.clamp(0.0, 2_147_483_000.0) as u32;
    }

    fn set(&mut self, vals: &[f32], inst: &Inst) {
        self.mode = vals[O_MODE] as usize;
        self.ring = (vals[O_RING] as i32 & 1) != 0;
        self.pitch = (vals[O_TXPOSE] - 64.0) + (vals[O_DETUNE] - 64.0) / 128.0;
        self.chg_pitch(inst);
        self.gain = vals[O_VOLUME] / 128.0;
        let col = vals[O_COLOR] / 128.0;
        self.brpt = ftou32(col);
        self.nfres = 1.0 - col.sqrt();
    }

    fn render(&mut self, dest: &mut [f32]) {
        match self.mode {
            1 => self.render_tri_saw(dest),
            2 => self.render_pulse(dest),
            3 => self.render_sin(dest),
            4 => self.render_noise(dest),
            5 => self.render_fm(dest),
            _ => {}
        }
    }

    #[inline]
    fn output(&self, d: &mut f32, x: f32) {
        if self.ring {
            *d *= x;
        } else {
            *d += x;
        }
    }

    // The state machine: was the wave rising, is it now, and did the counter wrap.
    fn osm_init(&self) -> u32 {
        if self.cnt.wrapping_sub(self.freq) < self.brpt {
            3
        } else {
            0
        }
    }

    fn osm_tick(&mut self, state: &mut u32) -> u32 {
        *state = ((*state << 1) | (self.cnt < self.brpt) as u32) & 3;
        let code = *state | if self.cnt < self.freq { 4 } else { 0 };
        self.cnt = self.cnt.wrapping_add(self.freq);
        code
    }

    /// A triangle bent towards a saw by Color, box-filtered (each sample
    /// the wave's average over it: little aliasing).
    fn render_tri_saw(&mut self, dest: &mut [f32]) {
        let f = utof23(self.freq).max(1e-9);
        let omf = 1.0 - f;
        let rcpf = 1.0 / f;
        let col = utof23(self.brpt).clamp(1e-6, 1.0 - 1e-6);
        let gain = self.gain;
        let c1 = gain / col;
        let c2 = -gain / (1.0 - col);
        let mut state = self.osm_init();
        for d in dest.iter_mut() {
            let p = utof23(self.cnt) - col;
            let y = match self.osm_tick(&mut state) {
                3 => c1 * (p + p - f),
                0 => c2 * (p + p - f),
                2 => rcpf * (c2 * p * p - c1 * (p - f) * (p - f)),
                5 => -rcpf * (gain + c2 * (p + omf) * (p + omf) - c1 * p * p),
                7 => -rcpf * (gain + c1 * omf * (p + p + omf)),
                4 => -rcpf * (gain + c2 * omf * (p + p + omf)),
                _ => 0.0,
            };
            self.output(d, y + gain);
        }
    }

    fn render_pulse(&mut self, dest: &mut [f32]) {
        let f = utof23(self.freq).max(1e-9);
        let gain = self.gain;
        let gdf = gain / f;
        let col = utof23(self.brpt);
        let cc121 = gdf * 2.0 * (col - 1.0) + gain;
        let cc212 = gdf * 2.0 * col - gain;
        let mut state = self.osm_init();
        for d in dest.iter_mut() {
            let p = utof23(self.cnt);
            let out = match self.osm_tick(&mut state) {
                3 => gain,
                0 => -gain,
                2 => gdf * 2.0 * (col - p) + gain,
                5 => gdf * 2.0 * p - gain,
                7 => cc121,
                4 => cc212,
                _ => 0.0,
            };
            self.output(d, out);
        }
    }

    fn render_sin(&mut self, dest: &mut [f32]) {
        for d in dest.iter_mut() {
            // (a cosine, as V2's is)
            let mut phase = self.cnt.wrapping_add(0x4000_0000);
            self.cnt = self.cnt.wrapping_add(self.freq);
            if phase & 0x8000_0000 != 0 {
                phase = !phase;
            }
            let t = f32::from_bits((phase >> 8) | 0x3f80_0000) * FCPI - FC1P5PI;
            let x = self.gain * fastsin(t);
            self.output(d, x);
        }
    }

    fn render_noise(&mut self, dest: &mut [f32]) {
        let mut flt = self.nf;
        let mut seed = self.nseed;
        for d in dest.iter_mut() {
            let n = frandom(&mut seed);
            let h = flt.step(n, self.nffrq, self.nfres);
            let x = self.nfres * (flt.l + h) + flt.b;
            self.output(d, self.gain * x);
        }
        self.nf = flt;
        self.nseed = seed;
    }

    /// A sine phase-modulated by what is in the voice so far.
    fn render_fm(&mut self, dest: &mut [f32]) {
        for d in dest.iter_mut() {
            let m = *d * FCFMMAX;
            let t = (utof23(self.cnt) + m) * FC2PI;
            self.cnt = self.cnt.wrapping_add(self.freq);
            let out = self.gain * fastsinrc(t);
            if self.ring {
                *d *= out;
            } else {
                *d = out;
            }
        }
    }
}

// ---------------------------------------------------------------- envelope

#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
enum EnvState {
    #[default]
    Off,
    Release,
    Attack,
    Decay,
    Sustain,
}

#[derive(Clone, Copy, Default)]
struct Env {
    out: f32,
    state: EnvState,
    val: f32,
    atd: f32,
    dcf: f32,
    sul: f32,
    suf: f32,
    rf: f32,
    gain: f32,
}

impl Env {
    fn set(&mut self, e: &[f32]) {
        self.atd = 2f32.powf(e[0] * FCATTACKMUL + FCATTACKADD);
        self.dcf = 1.0 - calcfreq2(1.0 - e[1] / 128.0);
        self.sul = e[2];
        self.suf = 2f32.powf(FCSUSMUL * (e[3] - 64.0));
        self.rf = 1.0 - calcfreq2(1.0 - e[4] / 128.0);
        self.gain = e[5] / 128.0;
    }

    fn tick(&mut self, gate: bool) {
        if self.state <= EnvState::Release && gate {
            self.state = EnvState::Attack;
        } else if self.state >= EnvState::Attack && !gate {
            self.state = EnvState::Release;
        }
        match self.state {
            EnvState::Off => self.val = 0.0,
            EnvState::Attack => {
                self.val += self.atd;
                if self.val >= 128.0 {
                    self.val = 128.0;
                    self.state = EnvState::Decay;
                }
            }
            EnvState::Decay => {
                self.val *= self.dcf;
                if self.val <= self.sul {
                    self.val = self.sul;
                    self.state = EnvState::Sustain;
                }
            }
            EnvState::Sustain => {
                self.val = (self.val * self.suf).min(128.0);
            }
            EnvState::Release => self.val *= self.rf,
        }
        if self.val <= FCLOWEST {
            self.val = 0.0;
            self.state = EnvState::Off;
        }
        self.out = self.val * self.gain;
    }
}

// ---------------------------------------------------------------- filter

#[derive(Clone, Copy, Default)]
struct Flt {
    mode: usize,
    cfreq: f32,
    res: f32,
    moogf: f32,
    moogp: f32,
    moogq: f32,
    lrc: Lrc,
    moog: Moog,
}

impl Flt {
    fn init(&mut self) {
        self.lrc = Lrc::default();
        self.moog = Moog::default();
    }

    fn set(&mut self, mode: f32, cutoff: f32, reso: f32, inst: &Inst) {
        self.mode = mode as usize;
        let f = calcfreq(cutoff / 128.0) * inst.linfreq;
        let r = reso / 128.0;
        if self.mode < 6 {
            self.res = 1.0 - r;
            self.cfreq = f;
        } else {
            let f = f * 0.25;
            let t = 1.0 - f;
            self.moogp = f + 0.8 * f * t;
            // (the Moog VCF of musicdsp.org has 2p - 1 here; the C++ port's
            // 1 - 2p, which its author doubted, leaves the low pass all but silent)
            self.moogf = self.moogp + self.moogp - 1.0;
            self.moogq = 4.0 * r * (1.0 + 0.5 * t * (1.0 - t + 5.6 * t * t));
        }
    }

    fn render(&mut self, buf: &mut [f32]) {
        let (cf, res) = (self.cfreq, self.res);
        match self.mode & 7 {
            1 => buf.iter_mut().for_each(|x| {
                self.lrc.step_2x(*x, cf, res);
                *x = self.lrc.l;
            }),
            2 => buf.iter_mut().for_each(|x| {
                self.lrc.step_2x(*x, cf, res);
                *x = self.lrc.b;
            }),
            3 => buf.iter_mut().for_each(|x| *x = self.lrc.step_2x(*x, cf, res)),
            4 => buf.iter_mut().for_each(|x| {
                let h = self.lrc.step_2x(*x, cf, res);
                *x = self.lrc.l + h;
            }),
            5 => buf.iter_mut().for_each(|x| {
                let h = self.lrc.step_2x(*x, cf, res);
                *x = self.lrc.l + self.lrc.b + h;
            }),
            6 => {
                let (f, p, q) = (self.moogf, self.moogp, self.moogq);
                buf.iter_mut().for_each(|x| {
                    self.moog.step(*x, f, p, q);
                    *x = self.moog.step(*x, f, p, q);
                })
            }
            7 => {
                let (f, p, q) = (self.moogf, self.moogp, self.moogq);
                buf.iter_mut().for_each(|x| {
                    self.moog.step(*x, f, p, q);
                    *x -= self.moog.step(*x, f, p, q);
                })
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------- LFO

#[derive(Clone, Copy, Default)]
struct Lfo {
    out: f32,
    mode: usize,
    sync: bool,
    eg: bool,
    freq: u32,
    cntr: u32,
    cphase: u32,
    gain: f32,
    dc: f32,
    nseed: u32,
    last: u32,
}

impl Lfo {
    fn init(&mut self, seed: u32) {
        self.cntr = 0;
        self.last = 0;
        self.nseed = seed;
    }

    fn set(&mut self, l: &[f32]) {
        self.mode = l[0] as usize;
        self.sync = l[1] as i32 != 0;
        self.eg = l[2] as i32 != 0;
        self.freq = (0.5 * FC32BIT * calcfreq(l[3] / 128.0)) as u32;
        self.cphase = ftou32(l[4] / 128.0);
        match l[5] as i32 {
            1 => {
                self.gain = -l[6];
                self.dc = 0.0;
            }
            2 => {
                self.gain = l[6];
                self.dc = -0.5 * l[6];
            }
            _ => {
                self.gain = l[6];
                self.dc = 0.0;
            }
        }
    }

    fn key_on(&mut self) {
        if self.sync {
            self.cntr = self.cphase;
            self.last = !0;
        }
    }

    fn tick(&mut self) {
        let v = match self.mode & 7 {
            1 => utof23((self.cntr << 1) ^ ((self.cntr as i32 >> 31) as u32)),
            2 => utof23((self.cntr as i32 >> 31) as u32),
            3 => fastsinrc(utof23(self.cntr) * FC2PI) * 0.5 + 0.5,
            4 => {
                if self.cntr < self.last {
                    self.nseed = urandom(&mut self.nseed);
                }
                self.last = self.cntr;
                utof23(self.nseed)
            }
            _ => utof23(self.cntr),
        };
        self.out = v * self.gain + self.dc;
        self.cntr = self.cntr.wrapping_add(self.freq);
        if self.cntr < self.freq && self.eg {
            self.cntr = !0;
        }
    }
}

// ---------------------------------------------------------------- distortion

#[derive(Clone, Copy, Default)]
struct Dist {
    mode: usize,
    gain1: f32,
    gain2: f32,
    offs: f32,
    crush1: f32,
    crush2: i32,
    crxor: i32,
    dcount: u32,
    dfreq: u32,
    dvall: f32,
    dvalr: f32,
    fltl: Flt,
    fltr: Flt,
}

impl Dist {
    fn init(&mut self) {
        self.dcount = 0;
        self.dvall = 0.0;
        self.dvalr = 0.0;
        self.fltl.init();
        self.fltr.init();
    }

    fn set(&mut self, d: &[f32], inst: &Inst) {
        self.mode = d[0] as usize;
        self.gain1 = 2f32.powf((d[1] - 32.0) / 16.0);
        match self.mode {
            0 => {}
            1 => {
                self.gain2 = (d[2] / 128.0) / self.gain1.atan();
                self.offs = self.gain1 * 2.0 * ((d[3] / 128.0) - 0.5);
            }
            2 => {
                self.gain2 = d[2] / 128.0;
                self.offs = self.gain1 * 2.0 * ((d[3] / 128.0) - 0.5);
            }
            3 => {
                let x = d[2] * 256.0 + 1.0;
                self.crush2 = x as i32;
                self.crush1 = self.gain1 * (32768.0 / x);
                self.crxor = (d[3] as i32) << 9;
            }
            4 => self.dfreq = ftou32(calcfreq(d[2] / 128.0)),
            m => {
                let fm = (m - 4) as f32;
                self.fltl.set(fm, d[2], d[3], inst);
                self.fltr.set(fm, d[2], d[3], inst);
            }
        }
    }

    #[inline]
    fn shape(&self, x: f32) -> f32 {
        match self.mode {
            1 => self.gain2 * fastatan(x * self.gain1 + self.offs),
            2 => self.gain2 * (x * self.gain1 + self.offs).clamp(-1.0, 1.0),
            3 => {
                let t = (x * self.crush1) as i32;
                let t = t.wrapping_mul(self.crush2).clamp(-0x7fff, 0x7fff) ^ self.crxor;
                t as f32 / 32768.0
            }
            _ => x,
        }
    }

    fn render_mono(&mut self, buf: &mut [f32]) {
        match self.mode {
            0 => {}
            1..=3 => buf.iter_mut().for_each(|x| *x = self.shape(*x)),
            4 => {
                for x in buf.iter_mut() {
                    self.dcount = self.dcount.wrapping_add(self.dfreq);
                    if self.dcount < self.dfreq {
                        self.dvall = *x;
                    }
                    *x = self.dvall;
                }
            }
            _ => self.fltl.render(buf),
        }
    }

    fn render_stereo(&mut self, l: &mut [f32], r: &mut [f32]) {
        match self.mode {
            0 => {}
            1..=3 => {
                l.iter_mut().for_each(|x| *x = self.shape(*x));
                r.iter_mut().for_each(|x| *x = self.shape(*x));
            }
            4 => {
                for (a, b) in l.iter_mut().zip(r.iter_mut()) {
                    self.dcount = self.dcount.wrapping_add(self.dfreq);
                    if self.dcount < self.dfreq {
                        self.dvall = *a;
                        self.dvalr = *b;
                    }
                    *a = self.dvall;
                    *b = self.dvalr;
                }
            }
            _ => {
                self.fltl.render(l);
                self.fltr.render(r);
            }
        }
    }
}

// ---------------------------------------------------------------- voice

#[derive(Clone, Copy, Default)]
struct Voice {
    note: u8,
    velo: f32,
    gate: bool,
    active: bool,
    alloc: u64,
    curvol: f32,
    volramp: f32,
    xpose: f32,
    fmode: usize,
    lvol: f32,
    rvol: f32,
    f1gain: f32,
    f2gain: f32,
    keysync: usize,
    osc: [Osc; 3],
    vcf: [Flt; 2],
    env: [Env; 2],
    lfo: [Lfo; 2],
    dist: Dist,
    dcf: Dcf,
}

impl Voice {
    fn init(&mut self, seed: u32) {
        for (i, o) in self.osc.iter_mut().enumerate() {
            o.init(i);
        }
        for f in self.vcf.iter_mut() {
            f.init();
        }
        for e in self.env.iter_mut() {
            *e = Env::default();
        }
        for (i, l) in self.lfo.iter_mut().enumerate() {
            l.init(seed.wrapping_mul(2_654_435_761).wrapping_add(i as u32 * 7919));
        }
        self.dist.init();
        self.dcf = Dcf::default();
    }

    fn tick(&mut self, inst: &Inst) {
        let gate = self.gate;
        for e in self.env.iter_mut() {
            e.tick(gate);
        }
        for l in self.lfo.iter_mut() {
            l.tick();
        }
        self.volramp = (self.env[0].out / 128.0 - self.curvol) * inst.iframe;
    }

    fn update_note(&mut self, bend: f32) {
        let n = self.xpose + self.note as f32 + bend;
        for o in self.osc.iter_mut() {
            o.note = n;
        }
    }

    fn set(&mut self, vals: &[f32], inst: &Inst, bend: f32) {
        self.xpose = vals[TRANSPOSE] - 64.0;
        self.update_note(bend);
        self.fmode = vals[ROUTING] as usize;
        self.keysync = vals[OSC_SYNC] as usize;
        let p = vals[PAN] / 128.0;
        self.lvol = (1.0 - p).max(0.0).sqrt();
        self.rvol = p.max(0.0).sqrt();
        let x = (vals[BALANCE] - 64.0) / 64.0;
        if x >= 0.0 {
            self.f2gain = 1.0;
            self.f1gain = 1.0 - x;
        } else {
            self.f1gain = 1.0;
            self.f2gain = 1.0 + x;
        }
        for i in 0..3 {
            self.osc[i].set(&vals[osc(i, 0)..osc(i, 6)], inst);
        }
        for i in 0..2 {
            self.env[i].set(&vals[env(i, 0)..env(i, 6)]);
            self.vcf[i].set(vals[flt(i, 0)], vals[flt(i, 1)], vals[flt(i, 2)], inst);
            self.lfo[i].set(&vals[lfo(i, 0)..lfo(i, 7)]);
        }
        self.dist.set(&vals[VDIST..VDIST + 4], inst);
    }

    fn note_on(&mut self, note: u8, vel: f32, inst: &Inst, voice_index: usize) {
        self.note = note;
        self.velo = vel;
        self.gate = true;
        for e in self.env.iter_mut() {
            e.state = EnvState::Attack;
        }
        if self.keysync >= 2 {
            for e in self.env.iter_mut() {
                e.val = 0.0;
            }
            self.curvol = 0.0;
            for (i, o) in self.osc.iter_mut().enumerate() {
                o.init(i);
            }
            for f in self.vcf.iter_mut() {
                f.init();
            }
            self.dist.init();
        }
        if self.keysync >= 1 {
            for o in self.osc.iter_mut() {
                o.cnt = 0;
            }
        }
        for o in self.osc.iter_mut() {
            o.chg_pitch(inst);
        }
        for l in self.lfo.iter_mut() {
            l.key_on();
        }
        self.dcf = Dcf::default();
        let _ = voice_index;
    }

    fn render(&mut self, voice: &mut [f32], voice2: &mut [f32], l: &mut [f32], r: &mut [f32], inst: &Inst) {
        let n = voice.len();
        voice.fill(0.0);
        for o in self.osc.iter_mut() {
            o.render(voice);
        }
        match self.fmode {
            0 => self.vcf[0].render(voice),
            2 => {
                voice2[..n].copy_from_slice(voice);
                self.vcf[1].render(&mut voice2[..n]);
                self.vcf[0].render(voice);
                for (a, b) in voice.iter_mut().zip(voice2.iter()) {
                    *a = *a * self.f1gain + *b * self.f2gain;
                }
            }
            _ => {
                self.vcf[0].render(voice);
                self.vcf[1].render(voice);
            }
        }
        self.dist.render_mono(voice);
        let rr = inst.dcfilter;
        for x in voice.iter_mut() {
            *x = self.dcf.step(*x, rr);
        }
        let mut cv = self.curvol;
        for i in 0..n {
            let out = voice[i] * cv;
            cv += self.volramp;
            l[i] += self.lvol * out + FCDCOFFSET;
            r[i] += self.rvol * out + FCDCOFFSET;
        }
        self.curvol = cv;
    }
}

// ---------------------------------------------------------------- channel effects

#[derive(Clone, Copy, Default)]
struct Boost {
    enabled: bool,
    a1: f32,
    a2: f32,
    b0: f32,
    b1: f32,
    b2: f32,
    x1: [f32; 2],
    x2: [f32; 2],
    y1: [f32; 2],
    y2: [f32; 2],
}

impl Boost {
    fn set(&mut self, amount: f32, inst: &Inst) {
        self.enabled = amount as i32 != 0;
        if !self.enabled {
            return;
        }
        let a = 2f32.powf(amount / 128.0);
        let beta = (2.0 * a).sqrt();
        let bs = beta * inst.boost_sin;
        let (am1, ap1) = (a - 1.0, a + 1.0);
        let (cam1, cap1) = (am1 * inst.boost_cos, ap1 * inst.boost_cos);
        let ia0 = 1.0 / (ap1 + cam1 + bs);
        self.b1 = 2.0 * a * (am1 - cap1) * ia0;
        self.a1 = -2.0 * (am1 + cap1) * ia0;
        self.a2 = (ap1 + cam1 - bs) * ia0;
        self.b0 = a * (ap1 - cam1 + bs) * ia0;
        self.b2 = a * (ap1 - cam1 - bs) * ia0;
    }

    fn render(&mut self, l: &mut [f32], r: &mut [f32]) {
        if !self.enabled {
            return;
        }
        for (ch, buf) in [l, r].into_iter().enumerate() {
            let (mut xm1, mut xm2, mut ym1, mut ym2) = (self.x1[ch], self.x2[ch], self.y1[ch], self.y2[ch]);
            for s in buf.iter_mut() {
                let x = *s + FCDCOFFSET;
                let y = self.b0 * x + self.b1 * xm1 + self.b2 * xm2 - self.a1 * ym1 - self.a2 * ym2;
                ym2 = ym1;
                ym1 = y;
                xm2 = xm1;
                xm1 = x;
                *s = y;
            }
            (self.x1[ch], self.x2[ch], self.y1[ch], self.y2[ch]) = (xm1, xm2, ym1, ym2);
        }
    }
}

/// The modulated delay: the channel's chorus/flanger, and the delay.
struct ModDel {
    db: [Vec<f32>; 2],
    mask: u32,
    ptr: u32,
    offs: [u32; 2],
    mcnt: u32,
    mfreq: u32,
    mphase: u32,
    mmaxoffs: u32,
    fbval: f32,
    dryout: f32,
    wetout: f32,
}

impl ModDel {
    fn new(len: usize) -> Self {
        Self { db: [vec![0.0; len], vec![0.0; len]], mask: len as u32 - 1, ptr: 0, offs: [0; 2], mcnt: 0, mfreq: 0, mphase: 0, mmaxoffs: 0, fbval: 0.0, dryout: 1.0, wetout: 0.0 }
    }

    fn set(&mut self, m: &[f32], inst: &Inst) {
        self.wetout = (m[0] - 64.0) / 64.0;
        self.dryout = 1.0 - self.wetout.abs();
        self.fbval = (m[1] - 64.0) / 64.0;
        let lenscale = (self.mask as f32 - 1023.0) / 128.0;
        self.offs = [(m[2] * lenscale) as u32, (m[3] * lenscale) as u32];
        self.mfreq = (inst.linfreq * FCMDLFOMUL * calcfreq(m[4] / 128.0)) as u32;
        self.mmaxoffs = (m[5] * 1023.0 / 128.0) as u32;
        self.mphase = ftou32((m[6] - 64.0) / 128.0);
    }

    #[inline]
    fn chan_sample(&mut self, input: f32, ch: usize, dry: f32) -> f32 {
        let mut counter = self.mcnt.wrapping_add(if ch == 1 { self.mphase } else { 0 });
        counter = if counter < 0x8000_0000 { counter.wrapping_mul(2) } else { 0xffff_ffffu32.wrapping_sub(counter.wrapping_mul(2)) };
        let o = counter as u64 * self.mmaxoffs as u64;
        let offs_int = ((o >> 32) as u32).wrapping_add(self.offs[ch]);
        let index = self.ptr.wrapping_sub(offs_int);
        let x = utof23((o & 0xffff_ffff) as u32);
        let buf = &mut self.db[ch];
        let delayed = lerp(buf[(index & self.mask) as usize], buf[(index.wrapping_sub(1) & self.mask) as usize], x);
        buf[self.ptr as usize] = input + delayed * self.fbval;
        input * dry + delayed * self.wetout
    }

    fn sample(&mut self, l: f32, r: f32, dry: f32) -> (f32, f32) {
        let a = self.chan_sample(l, 0, dry);
        let b = self.chan_sample(r, 1, dry);
        self.mcnt = self.mcnt.wrapping_add(self.mfreq);
        self.ptr = (self.ptr + 1) & self.mask;
        (a, b)
    }

    fn render_chan(&mut self, l: &mut [f32], r: &mut [f32]) {
        if self.wetout == 0.0 {
            return;
        }
        let dry = self.dryout;
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            (*a, *b) = self.sample(*a + FCDCOFFSET, *b + FCDCOFFSET, dry);
        }
    }

    /// The send into the output (wet only).
    fn render_aux(&mut self, aux: &[f32], l: &mut [f32], r: &mut [f32]) {
        if self.wetout == 0.0 {
            return;
        }
        for i in 0..aux.len() {
            let x = aux[i] + FCDCOFFSET;
            let (a, b) = self.sample(x, x, 0.0);
            l[i] += a;
            r[i] += b;
        }
    }
}

const COMPDLEN: usize = 5700;
const RMSLEN: usize = 8192;

struct Comp {
    /// bit 0: RMS, bit 1: stereo, bit 2: off.
    mode: u32,
    invol: f32,
    ratio: f32,
    outvol: f32,
    attack: f32,
    release: f32,
    dblen: usize,
    dbcnt: usize,
    curgain: [f32; 2],
    peakval: [f32; 2],
    rmsval: [f32; 2],
    rmscnt: usize,
    dbuf: Vec<[f32; 2]>,
    rmsbuf: Vec<[f32; 2]>,
    levels: Vec<[f32; 2]>,
}

impl Comp {
    fn new() -> Self {
        let mut c = Self { mode: 4, invol: 1.0, ratio: 0.0, outvol: 1.0, attack: 0.0, release: 0.0, dblen: 1, dbcnt: 0, curgain: [1.0; 2], peakval: [0.0; 2], rmsval: [0.0; 2], rmscnt: 0, dbuf: vec![[0.0; 2]; COMPDLEN], rmsbuf: vec![[0.0; 2]; RMSLEN], levels: Vec::new() };
        c.reset();
        c
    }

    fn reset(&mut self) {
        self.peakval = [0.0; 2];
        self.rmsval = [0.0; 2];
        self.curgain = [1.0; 2];
        self.rmsbuf.iter_mut().for_each(|x| *x = [0.0; 2]);
        self.rmscnt = 0;
    }

    fn set(&mut self, c: &[f32], inst: &Inst) {
        let old = self.mode;
        self.mode = match c[0] as i32 {
            1 => 0,
            2 => 1,
            _ => 4,
        };
        if c[1] != 0.0 {
            self.mode |= 2;
        }
        if self.mode != old {
            self.reset();
        }
        self.dblen = ((c[3] * inst.samples_per_ms) as usize).clamp(1, COMPDLEN);
        if self.dbcnt >= self.dblen {
            self.dbcnt = 0;
        }
        let mut thresh = 8.0 * calcfreq(c[4] / 128.0);
        self.invol = 1.0 / thresh;
        if c[2] != 0.0 {
            thresh = 1.0;
        }
        self.outvol = thresh * 2f32.powf((c[8] - 64.0) / 16.0);
        self.ratio = c[5] / 128.0;
        self.attack = 2f32.powf(-c[6] * 12.0 / 128.0);
        self.release = 2f32.powf(-c[7] * 16.0 / 128.0);
    }

    fn peak(&mut self, x: f32, ch: usize) -> f32 {
        self.peakval[ch] = (self.peakval[ch] * FCCPDFALLOFF + FCDCOFFSET).max(x.abs());
        self.peakval[ch]
    }

    fn rms(&mut self, x: f32, ch: usize) -> f32 {
        let sq = (x + FCDCOFFSET) * (x + FCDCOFFSET);
        self.rmsval[ch] += sq - self.rmsbuf[self.rmscnt][ch];
        self.rmsbuf[self.rmscnt][ch] = sq;
        (self.rmsval[ch].max(0.0) / RMSLEN as f32).sqrt()
    }

    fn render(&mut self, l: &mut [f32], r: &mut [f32]) {
        if self.mode & 4 != 0 {
            return;
        }
        let n = l.len();
        let mut levels = std::mem::take(&mut self.levels);
        levels.resize(n, [0.0; 2]);
        for i in 0..n {
            levels[i] = match self.mode & 3 {
                0 => {
                    let v = self.invol * self.peak(0.5 * (l[i] + r[i]), 0);
                    [v, v]
                }
                1 => {
                    let v = self.invol * self.rms(0.5 * (l[i] + r[i]), 0);
                    self.rmscnt = (self.rmscnt + 1) & (RMSLEN - 1);
                    [v, v]
                }
                2 => [self.invol * self.peak(l[i], 0), self.invol * self.peak(r[i], 1)],
                _ => {
                    let v = [self.invol * self.rms(l[i], 0), self.invol * self.rms(r[i], 1)];
                    self.rmscnt = (self.rmscnt + 1) & (RMSLEN - 1);
                    v
                }
            };
        }
        for (ch, buf) in [l, r].into_iter().enumerate() {
            let mut gain = self.curgain[ch];
            let mut ind = self.dbcnt;
            for i in 0..n {
                let v = self.outvol * self.dbuf[ind][ch];
                self.dbuf[ind][ch] = self.invol * buf[i];
                ind += 1;
                if ind >= self.dblen {
                    ind = 0;
                }
                let lvl = levels[i][ch];
                let dgain = if lvl >= 1.0 { 1.0 / (1.0 + self.ratio * (lvl - 1.0)) } else { 1.0 };
                gain += if dgain < gain { self.attack } else { self.release } * (dgain - gain);
                buf[i] = v * gain;
            }
            self.curgain[ch] = gain;
            if ch == 1 {
                self.dbcnt = ind;
            }
        }
        self.levels = levels;
    }
}

struct Delay {
    buf: Vec<f32>,
    pos: usize,
}

impl Delay {
    fn new(len: usize) -> Self {
        Self { buf: vec![0.0; len], pos: 0 }
    }

    fn fetch(&self) -> f32 {
        self.buf[self.pos]
    }

    fn feed(&mut self, v: f32) {
        self.buf[self.pos] = v;
        self.pos += 1;
        if self.pos == self.buf.len() {
            self.pos = 0;
        }
    }
}

struct Reverb {
    gainc: [f32; 4],
    gaina: [f32; 2],
    gainin: f32,
    damp: f32,
    lowcut: f32,
    combd: [[Delay; 4]; 2],
    combl: [[f32; 4]; 2],
    alld: [[Delay; 2]; 2],
    hpf: [f32; 2],
}

impl Reverb {
    fn new() -> Self {
        let d = Delay::new;
        Self {
            gainc: [0.0; 4],
            gaina: [0.0; 2],
            gainin: 0.0,
            damp: 0.0,
            lowcut: 0.0,
            combd: [[d(1309), d(1635), d(1811), d(1926)], [d(1327), d(1631), d(1833), d(1901)]],
            combl: [[0.0; 4]; 2],
            alld: [[d(220), d(74)], [d(205), d(77)]],
            hpf: [0.0; 2],
        }
    }

    fn set(&mut self, rv: &[f32], inst: &Inst) {
        const GAINC: [f32; 4] = [0.966_384_6, 0.958_186_36, 0.953_783_9, 0.950_933_2];
        const GAINA: [f32; 2] = [0.994_260_1, 0.998_044_7];
        let e = inst.linfreq * (64.0 / (rv[0] + 1.0)).powi(2);
        for i in 0..4 {
            self.gainc[i] = GAINC[i].powf(e);
        }
        for i in 0..2 {
            self.gaina[i] = GAINA[i].powf(e);
        }
        self.damp = inst.linfreq * (rv[1] / 128.0);
        self.gainin = rv[3] / 128.0;
        self.lowcut = inst.linfreq * (rv[2] / 128.0).powi(4);
    }

    fn render(&mut self, aux: &[f32], l: &mut [f32], r: &mut [f32]) {
        for i in 0..aux.len() {
            let input = aux[i] * self.gainin + FCDCOFFSET;
            for ch in 0..2 {
                let mut cur = 0.0;
                for j in 0..4 {
                    let dv = self.gainc[j] * self.combd[ch][j].fetch();
                    let nv = if j & 1 == 1 { dv - input } else { dv + input };
                    let lp = self.combl[ch][j] + self.damp * (nv - self.combl[ch][j]);
                    self.combl[ch][j] = lp;
                    self.combd[ch][j].feed(lp);
                    cur += lp;
                }
                for j in 0..2 {
                    let dv = self.alld[ch][j].fetch();
                    let dz = cur + self.gaina[j] * dv;
                    self.alld[ch][j].feed(dz);
                    cur = dv - self.gaina[j] * dz;
                }
                self.hpf[ch] += self.lowcut * (cur - self.hpf[ch]);
                let out = cur - self.hpf[ch];
                if ch == 0 {
                    l[i] += out;
                } else {
                    r[i] += out;
                }
            }
        }
    }
}

// ---------------------------------------------------------------- the synth

const VOICES: usize = 16;

pub struct V2State {
    inst: Inst,
    voices: Vec<Voice>,
    clock: u64,
    last_voice: usize,
    // the channel
    chgain: f32,
    a1gain: f32,
    a2gain: f32,
    fxr: usize,
    dcf1: [Dcf; 2],
    boost: Boost,
    cdist: Dist,
    dcf2: [Dcf; 2],
    chorus: ModDel,
    comp: Comp,
    // the globals
    reverb: Reverb,
    delay: ModDel,
    dcf: [Dcf; 2],
    lcfreq: f32,
    lcbuf: [f32; 2],
    hcfreq: f32,
    hcbuf: [f32; 2],
    // buffers (a frame)
    vce: Vec<f32>,
    vce2: Vec<f32>,
    chan: [Vec<f32>; 2],
    aux1: Vec<f32>,
    aux2: Vec<f32>,
    mix: [Vec<f32>; 2],
    tickd: usize,
    vals: Vec<f32>,
    pub pitch_bend: f32,
}

fn get(p: &[f32], i: usize) -> f32 {
    p.get(i).copied().unwrap_or(SPECS[i].default)
}

impl V2State {
    pub fn new(sr: f32) -> Self {
        let inst = Inst::new(sr);
        let n = inst.frame;
        let mut voices = vec![Voice::default(); VOICES];
        for (i, v) in voices.iter_mut().enumerate() {
            v.init(0x1234_5678 ^ i as u32);
        }
        Self {
            inst,
            voices,
            clock: 0,
            last_voice: 0,
            chgain: 0.0,
            a1gain: 0.0,
            a2gain: 0.0,
            fxr: 0,
            dcf1: [Dcf::default(); 2],
            boost: Boost::default(),
            cdist: Dist::default(),
            dcf2: [Dcf::default(); 2],
            chorus: ModDel::new(2048),
            comp: Comp::new(),
            reverb: Reverb::new(),
            delay: ModDel::new(32768),
            dcf: [Dcf::default(); 2],
            lcfreq: 0.0,
            lcbuf: [0.0; 2],
            hcfreq: 1.0,
            hcbuf: [0.0; 2],
            vce: vec![0.0; n],
            vce2: vec![0.0; n],
            chan: [vec![0.0; n], vec![0.0; n]],
            aux1: vec![0.0; n],
            aux2: vec![0.0; n],
            mix: [vec![0.0; n], vec![0.0; n]],
            tickd: 0,
            vals: vec![0.0; CHAN_END],
            pitch_bend: 0.0,
        }
    }

    /// A modulation source's value for a voice.
    fn source(&self, p: &[f32], v: &Voice, source: usize) -> f32 {
        match source {
            1 => v.velo,
            2 => get(p, MOD_WHEEL),
            3..=7 => 0.0,
            8 => 127.0,
            9 => v.env[0].out,
            10 => v.env[1].out,
            11 => v.lfo[0].out,
            12 => v.lfo[1].out,
            13 => 2.0 * (v.note as f32 - 48.0),
            _ => 0.0,
        }
    }

    /// The parameters with the matrix applied, for voice `k` (the voice's
    /// part, or the channel's).
    fn values(&mut self, p: &[f32], k: usize, from: usize, to: usize) {
        for i in from..to {
            self.vals[i] = get(p, i).clamp(SPECS[i].min, SPECS[i].max.max(127.0));
        }
        let voice = self.voices[k];
        for s in 0..MOD_SLOTS {
            let src = get(p, modslot(s, 0)).round() as usize;
            if src == 0 {
                continue;
            }
            let dest = get(p, modslot(s, 2)).round().max(0.0) as usize;
            if dest < from || dest >= to {
                continue;
            }
            let scale = (get(p, modslot(s, 1)) - 64.0) / 64.0;
            let x = self.vals[dest] + scale * self.source(p, &voice, src);
            self.vals[dest] = x.clamp(0.0, 128.0);
        }
        // Choices stay within their options.
        for i in from..to {
            if let crate::params::ParamKind::Choice(opts) = SPECS[i].kind {
                self.vals[i] = self.vals[i].floor().min(opts.len() as f32 - 1.0);
            }
        }
    }

    fn store_voice(&mut self, p: &[f32], k: usize) {
        self.values(p, k, 0, VOICE_N);
        let inst = self.inst;
        let bend = self.pitch_bend;
        let vals = std::mem::take(&mut self.vals);
        self.voices[k].set(&vals, &inst, bend);
        self.vals = vals;
    }

    fn store_chan(&mut self, p: &[f32]) {
        let k = self.last_voice;
        self.values(p, k, VOICE_N, CHAN_END);
        let v = &self.vals;
        let inst = self.inst;
        self.chgain = FCGAIN * (v[CHAN_VOL] / 128.0);
        self.a1gain = self.chgain * FCGAINH * (v[REVERB_SEND] / 128.0);
        self.a2gain = self.chgain * FCGAINH * (v[DELAY_SEND] / 128.0);
        self.fxr = v[FX_ROUTE] as usize;
        let (cd, ch, cm, b) = ([v[CDIST], v[CDIST + 1], v[CDIST + 2], v[CDIST + 3]], [v[CHORUS], v[CHORUS + 1], v[CHORUS + 2], v[CHORUS + 3], v[CHORUS + 4], v[CHORUS + 5], v[CHORUS + 6]], [v[COMP], v[COMP + 1], v[COMP + 2], v[COMP + 3], v[COMP + 4], v[COMP + 5], v[COMP + 6], v[COMP + 7], v[COMP + 8]], v[BOOST]);
        self.cdist.set(&cd, &inst);
        self.chorus.set(&ch, &inst);
        self.comp.set(&cm, &inst);
        self.boost.set(b, &inst);
    }

    fn store_globals(&mut self, p: &[f32]) {
        let inst = self.inst;
        let rv = [get(p, RV_TIME), get(p, RV_HIGHCUT), get(p, RV_LOWCUT), get(p, RV_VOLUME)];
        self.reverb.set(&rv, &inst);
        let dl: Vec<f32> = (0..7).map(|i| get(p, DELAY + i)).collect();
        self.delay.set(&dl, &inst);
        self.lcfreq = ((get(p, LOWCUT) + 1.0) / 128.0).powi(2);
        self.hcfreq = ((get(p, HIGHCUT) + 1.0) / 128.0).powi(2);
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        let poly = get(p, POLY).round().clamp(1.0, VOICES as f32) as usize;
        let used = self.voices.iter().filter(|v| v.active).count();
        let mut pick = None;
        if used < poly {
            pick = self.voices.iter().position(|v| !v.active);
        }
        if pick.is_none() {
            // The oldest with its key up, else the oldest.
            pick = (0..VOICES).filter(|&i| self.voices[i].active && !self.voices[i].gate).min_by_key(|&i| self.voices[i].alloc);
        }
        let k = pick.or_else(|| (0..VOICES).filter(|&i| self.voices[i].active).min_by_key(|&i| self.voices[i].alloc)).unwrap_or(0);
        self.clock += 1;
        self.voices[k].active = true;
        self.voices[k].alloc = self.clock;
        self.voices[k].note = note;
        self.voices[k].velo = vel * 127.0;
        self.last_voice = k;
        self.store_voice(p, k);
        let inst = self.inst;
        self.voices[k].note_on(note, vel * 127.0, &inst, k);
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        for v in self.voices.iter_mut() {
            if v.active && v.gate && v.note == note {
                v.gate = false;
            }
        }
    }

    pub fn all_off(&mut self) {
        for (i, v) in self.voices.iter_mut().enumerate() {
            v.active = false;
            v.gate = false;
            v.init(0x1234_5678 ^ i as u32);
        }
    }

    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.voices.iter().filter(|v| v.active).count() as f32;
        let v = &self.voices[self.last_voice];
        if v.active {
            out[1] = v.env[0].val / 128.0;
            out[2] = v.env[1].val / 128.0;
            out[3] = v.lfo[0].out / 128.0;
            out[4] = v.lfo[1].out / 128.0;
            out[5] = v.note as f32;
            out[6] = 1.0;
        }
    }

    /// A frame: the matrix, the envelopes and LFOs, the voices, the channel, the effects.
    fn tick(&mut self, p: &[f32]) {
        let inst = self.inst;
        for k in 0..VOICES {
            if !self.voices[k].active {
                continue;
            }
            self.store_voice(p, k);
            self.voices[k].tick(&inst);
            if self.voices[k].env[0].state == EnvState::Off {
                self.voices[k].active = false;
            }
        }
        self.store_chan(p);
        self.store_globals(p);
        let n = inst.frame;
        let [ml, mr] = &mut self.mix;
        for b in [ml, mr, &mut self.aux1, &mut self.aux2] {
            b.fill(0.0);
        }
        if self.voices.iter().any(|v| v.active) {
            let [cl, cr] = &mut self.chan;
            cl.fill(0.0);
            cr.fill(0.0);
            for v in self.voices.iter_mut().filter(|v| v.active) {
                v.render(&mut self.vce, &mut self.vce2, cl, cr, &inst);
            }
            // The channel: DC filter, compressor, boost, distortion and chorus.
            let r = inst.dcfilter;
            for i in 0..n {
                cl[i] = self.dcf1[0].step(cl[i], r);
                cr[i] = self.dcf1[1].step(cr[i], r);
            }
            self.comp.render(cl, cr);
            self.boost.render(cl, cr);
            if self.fxr == 0 {
                self.cdist.render_stereo(cl, cr);
                for i in 0..n {
                    cl[i] = self.dcf2[0].step(cl[i], r);
                    cr[i] = self.dcf2[1].step(cr[i], r);
                }
                self.chorus.render_chan(cl, cr);
            } else {
                self.chorus.render_chan(cl, cr);
                self.cdist.render_stereo(cl, cr);
                for i in 0..n {
                    cl[i] = self.dcf2[0].step(cl[i], r);
                    cr[i] = self.dcf2[1].step(cr[i], r);
                }
            }
            for i in 0..n {
                let m = cl[i] + cr[i];
                self.aux1[i] += self.a1gain * m;
                self.aux2[i] += self.a2gain * m;
                self.mix[0][i] += self.chgain * cl[i];
                self.mix[1][i] += self.chgain * cr[i];
            }
        }
        // The reverb and the delay, a DC filter, the low and high cuts.
        let [ml, mr] = &mut self.mix;
        self.reverb.render(&self.aux1, ml, mr);
        self.delay.render_aux(&self.aux2, ml, mr);
        let r = inst.dcfilter;
        let (lcf, hcf) = (self.lcfreq, self.hcfreq);
        for i in 0..n {
            for (ch, buf) in [&mut *ml, &mut *mr].into_iter().enumerate() {
                let y = self.dcf[ch].step(buf[i], r);
                let mut x = y - self.lcbuf[ch];
                self.lcbuf[ch] += lcf * x;
                if hcf != 1.0 {
                    self.hcbuf[ch] += hcf * (x - self.hcbuf[ch]);
                    x = self.hcbuf[ch];
                }
                buf[i] = x;
            }
        }
        self.tickd = n;
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        // (V2's channel at full volume is loud: half, to sit with the other instruments)
        let volume = 10f32.powf(get(p, VOLUME) / 20.0) * 0.5;
        let mut done = 0;
        while done < n {
            if self.tickd == 0 {
                self.tick(p);
            }
            let frame = self.inst.frame;
            let from = frame - self.tickd;
            let m = self.tickd.min(n - done);
            for k in 0..m {
                l[done + k] = self.mix[0][from + k] * volume;
                r[done + k] = self.mix[1][from + k] * volume;
            }
            done += m;
            self.tickd -= m;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(p: &[f32], note: u8, secs: f32, off_at: f32) -> (Vec<f32>, Vec<f32>) {
        let sr = 48000.0;
        let mut st = V2State::new(sr);
        st.note_on(p, note, 0.8);
        let (mut ol, mut or) = (Vec::new(), Vec::new());
        for b in 0..(secs * sr / 256.0) as usize {
            if b == (off_at * sr / 256.0) as usize {
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
    fn the_tables_line_up() {
        assert_eq!(SPECS.len(), VOLUME + 1);
        assert_eq!(DESTS.len(), CHAN_END);
        assert_eq!(SPECS[osc(2, O_VOLUME)].name, "Osc 3 Volume");
        assert_eq!(SPECS[env(1, 5)].name, "EG2 Amplify");
        assert_eq!(SPECS[lfo(1, 6)].name, "LFO2 Amplify");
        assert_eq!(SPECS[OSC_SYNC].name, "Osc Sync");
        assert_eq!(SPECS[COMP + 8].name, "Comp Out Gain");
        assert_eq!(SPECS[RV_VOLUME].name, "Reverb Volume");
        assert_eq!(SPECS[HIGHCUT].name, "High Cut");
        assert_eq!(SPECS[modslot(7, 2)].name, "Mod 8 Dest");
        assert_eq!(DESTS[37], "Amp EG Amplify");
        assert_eq!(DESTS[50], "LFO 1 Amplify");
        assert_eq!(DESTS[59], "Channel Volume");
    }

    #[test]
    fn a_saw_plays_at_the_keys_pitch() {
        let mut p = preset("Init");
        p[flt(0, 0)] = 0.0;
        let (l, _) = run(&p, 57, 1.0, 1.0);
        let x = &l[9600..33600];
        let sr = 48000.0;
        let lag_of = |lag: usize| x.iter().zip(&x[lag..]).map(|(a, b)| a * b).sum::<f32>();
        let want = (sr / 220.0) as usize;
        let best = (want - 30..want + 30).max_by(|a, b| lag_of(*a).total_cmp(&lag_of(*b))).unwrap();
        assert!((best as i32 - want as i32).abs() <= 1, "{best} vs {want}");
    }

    #[test]
    fn every_preset_sounds_and_stays_in_range() {
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let (l, r) = run(&p, if matches!(*name, "Kick" | "Snare" | "Hat") { 48 } else { 60 }, 2.5, 1.0);
            let level = rms(&l[480..24000]).max(rms(&r[480..24000])).max(rms(&l[24000..48000]));
            let peak = l.iter().chain(&r).fold(0.0f32, |m, x| m.max(x.abs()));
            eprintln!("{name:<12} level {level:.3} peak {peak:.3}");
            assert!(l.iter().chain(&r).all(|x| x.is_finite()), "{name}");
            assert!(level > 0.01, "{name}: {level}");
            assert!(peak < 2.0, "{name}: {peak}");
        }
    }

    #[test]
    fn released_notes_end_and_voices_free_up() {
        let p = preset("Init");
        let sr = 48000.0;
        let mut st = V2State::new(sr);
        for k in 0..20 {
            st.note_on(&p, 40 + k, 0.8);
        }
        assert!(st.voices.iter().filter(|v| v.active).count() <= VOICES);
        for k in 0..20 {
            st.note_off(&p, 40 + k);
        }
        for _ in 0..(4.0 * sr / 256.0) as usize {
            let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
            st.render(&p, &mut l, &mut r);
        }
        assert_eq!(st.voices.iter().filter(|v| v.active).count(), 0);
    }

    #[test]
    fn the_matrix_moves_parameters() {
        // Velocity into the amp envelope's level: louder notes are louder.
        let p = preset("Init");
        let sr = 48000.0;
        let level = |vel: f32| {
            let mut st = V2State::new(sr);
            st.note_on(&p, 60, vel);
            let mut out = Vec::new();
            for _ in 0..40 {
                let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
                st.render(&p, &mut l, &mut r);
                out.extend(l);
            }
            rms(&out[2000..])
        };
        assert!(level(1.0) > 1.6 * level(0.4), "{} {}", level(1.0), level(0.4));
    }
}

