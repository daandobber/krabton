//! Quartet: four oscillators, each one of eleven kinds, into two filters,
//! each one of ten, with five envelopes, three LFOs and a modulation
//! matrix of eight slots, and a delay and a reverb after it. The kinds of
//! oscillator: Classic (saw, square, triangle, sine, pulses), Wave (the 128
//! single-cycle waves of Waves, morphing into the next), Chorus (three
//! copies of a wave, spread apart), Grains (short grains of a wave, now and
//! then, a little off pitch each), FM (a sine whose phase the wave moves)
//! and FM Inverse (the wave read at a pace a sine moves), Shaper (a sine
//! through a waveshaper), Noise (white, pink, crackle or steps), Pad (Paul
//! Nasca's PADsynth: every harmonic spread into a band of noise, built
//! once), and Phase (phase distortion, see `warp`). Each sends its sound to
//! filter A or B or between them, and sits left or right. Krabton's own,
//! after the list of what WhySynth (now ZedSynth) by Sean Bolton does; none
//! of its code is used.

use super::reverb::Reverb;
use super::waves::{self, TableRead};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

pub const OSC_PARAMS: usize = 8;
pub const O_MODE: usize = 0;
pub const O_WAVE: usize = 1;
pub const O_PITCH: usize = 2;
pub const O_DETUNE: usize = 3;
pub const O_PARAM: usize = 4;
pub const O_LEVEL: usize = 5;
pub const O_BUS: usize = 6;
pub const O_PAN: usize = 7;

pub const fn osc_param(o: usize, k: usize) -> usize {
    o * OSC_PARAMS + k
}

pub const FA_MODE: usize = 32;
pub const FA_CUTOFF: usize = 33;
pub const FA_RESO: usize = 34;
pub const FA_ENV: usize = 35;
pub const FA_KEYS: usize = 36;
pub const FB_MODE: usize = 37;
pub const FB_CUTOFF: usize = 38;
pub const FB_RESO: usize = 39;
pub const FB_ENV: usize = 40;
pub const FB_KEYS: usize = 41;
pub const FB_SERIES: usize = 42;
/// The envelopes: 0 the amp, 1 (Env 2) the filters', 2 to 4 for the matrix.
pub const ENV_BASE: usize = 43;
pub const ENVS: usize = 5;
pub const fn env_param(e: usize, k: usize) -> usize {
    ENV_BASE + e * 4 + k
}
pub const LFO_BASE: usize = 63;
pub const LFOS: usize = 3;
pub const fn lfo_param(l: usize, k: usize) -> usize {
    LFO_BASE + l * 3 + k
}
pub const MATRIX_BASE: usize = 72;
pub const SLOTS: usize = 8;
pub const fn slot_param(s: usize, k: usize) -> usize {
    MATRIX_BASE + s * 3 + k
}
pub const DELAY: usize = 96;
pub const DELAY_TIME: usize = 97;
pub const FEEDBACK: usize = 98;
pub const REVERB: usize = 99;
pub const REVERB_SIZE: usize = 100;
pub const VOICES: usize = 101;
pub const GLIDE: usize = 102;
pub const VELOCITY: usize = 103;
pub const VOLUME: usize = 104;
pub const COUNT: usize = 105;

pub const MODES: &[&str] = &["Off", "Classic", "Wave", "Chorus", "Grains", "FM", "FM Inverse", "Shaper", "Noise", "Pad", "Phase"];
pub const CLASSIC: &[&str] = &["Saw", "Square", "Triangle", "Sine", "Pulse", "Narrow"];
pub const SHAPERS: &[&str] = &["Soft", "Fold", "Cheby 3", "Cheby 5", "Clip", "Rectify"];
pub const NOISES: &[&str] = &["White", "Pink", "Crackle", "Steps"];
pub const PADS: &[&str] = &["Saw Pad", "Hollow Pad", "Choir Pad", "Organ Pad"];
pub const FILTERS: &[&str] = &["Off", "Low 12", "Low 24", "High 12", "High 24", "Band", "Notch", "Ladder", "Peak", "Comb"];
pub const LFO_WAVES: &[&str] = &["Sine", "Triangle", "Saw Up", "Saw Down", "Square", "S&H"];
pub const SOURCES: &[&str] = &["Off", "LFO 1", "LFO 2", "LFO 3", "Env 2", "Env 3", "Env 4", "Env 5", "Amp Env", "Velocity", "Key", "Random"];
pub const DESTS: &[&str] = &["Off", "Pitch", "Pitch 1", "Pitch 2", "Pitch 3", "Pitch 4", "Param 1", "Param 2", "Param 3", "Param 4", "Level 1", "Level 2", "Level 3", "Level 4", "Cutoff A", "Cutoff B", "Reso A", "Reso B", "Pan", "Amp"];

/// The waves a mode chooses from (Wave, Chorus, Grains and the FMs: all 128).
pub fn waves_of(mode: usize) -> usize {
    match mode {
        1 => CLASSIC.len(),
        2..=6 => waves::WAVE_COUNT,
        7 => SHAPERS.len(),
        8 => NOISES.len(),
        9 => PADS.len(),
        10 => super::warp::WAVES.len(),
        _ => 1,
    }
}

/// What an oscillator's wave is called.
pub fn wave_name(mode: usize, wave: usize) -> String {
    let w = wave % waves_of(mode).max(1);
    match mode {
        1 => CLASSIC[w].into(),
        2..=6 => format!("{} {}", waves::BANKS[w / waves::BANK_SIZE], w % waves::BANK_SIZE + 1),
        7 => SHAPERS[w].into(),
        8 => NOISES[w].into(),
        9 => PADS[w].into(),
        10 => super::warp::WAVES[w].into(),
        _ => "".into(),
    }
}

macro_rules! osc_specs {
    ($g:literal, $n:literal, $mode:expr, $level:expr, $detune:expr, $bus:expr) => {
        [
            ParamSpec::choice(concat!("Mode ", $n), $g, MODES, $mode),
            ParamSpec::int(concat!("Wave ", $n), $g, 0.0, 127.0, 0.0, Unit::None),
            ParamSpec::int(concat!("Pitch ", $n), $g, -24.0, 24.0, 0.0, Unit::Semi),
            ParamSpec::lin(concat!("Detune ", $n), $g, -50.0, 50.0, $detune, Unit::Cents),
            ParamSpec::lin(concat!("Param ", $n), $g, 0.0, 1.0, 0.3, Unit::Percent),
            ParamSpec::lin(concat!("Level ", $n), $g, 0.0, 1.0, $level, Unit::Percent),
            ParamSpec::lin(concat!("Bus ", $n), $g, 0.0, 1.0, $bus, Unit::Percent),
            ParamSpec::lin(concat!("Pan ", $n), $g, -1.0, 1.0, 0.0, Unit::Pan),
        ]
    };
}

macro_rules! env_specs {
    ($g:literal, $a:literal, $d:literal, $s:literal, $r:literal, $va:expr, $vd:expr, $vs:expr, $vr:expr) => {
        [
            ParamSpec::log($a, $g, 1.0, 8000.0, $va, Unit::Ms),
            ParamSpec::log($d, $g, 1.0, 8000.0, $vd, Unit::Ms),
            ParamSpec::lin($s, $g, 0.0, 1.0, $vs, Unit::Percent),
            ParamSpec::log($r, $g, 1.0, 10000.0, $vr, Unit::Ms),
        ]
    };
}

macro_rules! lfo_specs {
    ($n:literal, $wave:expr, $rate:expr) => {
        [
            ParamSpec::choice(concat!("LFO ", $n, " Wave"), "LFOs", LFO_WAVES, $wave),
            ParamSpec::log(concat!("LFO ", $n, " Rate"), "LFOs", 0.02, 30.0, $rate, Unit::Hz),
            ParamSpec::lin(concat!("LFO ", $n, " Delay"), "LFOs", 0.0, 3.0, 0.0, Unit::Sec),
        ]
    };
}

macro_rules! slot_specs {
    ($n:literal) => {
        [
            ParamSpec::choice(concat!("Source ", $n), "Matrix", SOURCES, 0),
            ParamSpec::choice(concat!("Dest ", $n), "Matrix", DESTS, 0),
            ParamSpec::lin(concat!("Amount ", $n), "Matrix", -1.0, 1.0, 0.0, Unit::Percent),
        ]
    };
}

const O1: [ParamSpec; 8] = osc_specs!("Osc 1", "1", 1, 0.8, 0.0, 0.0);
const O2: [ParamSpec; 8] = osc_specs!("Osc 2", "2", 1, 0.6, 7.0, 0.0);
const O3: [ParamSpec; 8] = osc_specs!("Osc 3", "3", 0, 0.5, -7.0, 0.0);
const O4: [ParamSpec; 8] = osc_specs!("Osc 4", "4", 0, 0.5, 0.0, 1.0);
const FILT: [ParamSpec; 11] = [
    ParamSpec::choice("Filter A", "Filter A", FILTERS, 2),
    ParamSpec::log("Cutoff A", "Filter A", 20.0, 20000.0, 2500.0, Unit::Hz),
    ParamSpec::lin("Reso A", "Filter A", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Env A", "Filter A", -1.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Keys A", "Filter A", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::choice("Filter B", "Filter B", FILTERS, 0),
    ParamSpec::log("Cutoff B", "Filter B", 20.0, 20000.0, 1500.0, Unit::Hz),
    ParamSpec::lin("Reso B", "Filter B", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Env B", "Filter B", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Keys B", "Filter B", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::toggle("B after A", "Filter B", false),
];
const E1: [ParamSpec; 4] = env_specs!("Amp", "Attack", "Decay", "Sustain", "Release", 4.0, 300.0, 0.8, 300.0);
const E2: [ParamSpec; 4] = env_specs!("Env 2", "Attack 2", "Decay 2", "Sustain 2", "Release 2", 2.0, 400.0, 0.3, 400.0);
const E3: [ParamSpec; 4] = env_specs!("Env 3", "Attack 3", "Decay 3", "Sustain 3", "Release 3", 10.0, 500.0, 0.5, 500.0);
const E4: [ParamSpec; 4] = env_specs!("Env 4", "Attack 4", "Decay 4", "Sustain 4", "Release 4", 10.0, 500.0, 0.5, 500.0);
const E5: [ParamSpec; 4] = env_specs!("Env 5", "Attack 5", "Decay 5", "Sustain 5", "Release 5", 10.0, 500.0, 0.5, 500.0);
const L1: [ParamSpec; 3] = lfo_specs!("1", 0, 5.0);
const L2: [ParamSpec; 3] = lfo_specs!("2", 1, 0.5);
const L3: [ParamSpec; 3] = lfo_specs!("3", 5, 4.0);
const S1: [ParamSpec; 3] = slot_specs!("1");
const S2: [ParamSpec; 3] = slot_specs!("2");
const S3: [ParamSpec; 3] = slot_specs!("3");
const S4: [ParamSpec; 3] = slot_specs!("4");
const S5: [ParamSpec; 3] = slot_specs!("5");
const S6: [ParamSpec; 3] = slot_specs!("6");
const S7: [ParamSpec; 3] = slot_specs!("7");
const S8: [ParamSpec; 3] = slot_specs!("8");
const TAIL: [ParamSpec; 9] = [
    ParamSpec::lin("Delay", "Effects", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Delay Time", "Effects", 20.0, 1500.0, 375.0, Unit::Ms),
    ParamSpec::lin("Feedback", "Effects", 0.0, 0.95, 0.35, Unit::Percent),
    ParamSpec::lin("Reverb", "Effects", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Reverb Size", "Effects", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::int("Voices", "Play", 1.0, 8.0, 8.0, Unit::None),
    ParamSpec::log("Glide", "Play", 1.0, 2000.0, 1.0, Unit::Ms),
    ParamSpec::lin("Velocity", "Play", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Volume", "Play", 0.0, 1.0, 0.7, Unit::Percent),
];

const fn build() -> [ParamSpec; COUNT] {
    let parts: [&[ParamSpec]; 23] = [&O1, &O2, &O3, &O4, &FILT, &E1, &E2, &E3, &E4, &E5, &L1, &L2, &L3, &S1, &S2, &S3, &S4, &S5, &S6, &S7, &S8, &TAIL, &[]];
    let mut out = [O1[0]; COUNT];
    let mut i = 0;
    let mut p = 0;
    while p < parts.len() {
        let part = parts[p];
        let mut k = 0;
        while k < part.len() {
            out[i] = part[k];
            i += 1;
            k += 1;
        }
        p += 1;
    }
    out
}

static ALL: [ParamSpec; COUNT] = build();
pub static SPECS: &[ParamSpec] = &ALL;

pub fn preset_names() -> &'static [&'static str] {
    &["Init", "Big Saws", "Wave Pad", "Grain Cloud", "FM Bell", "FM Bass", "Shaper Lead", "Choir Pad", "Organ Pad", "Wind", "Phase Brass", "Chorus Strings", "Sweep Matrix", "Vocal Wave", "Digital Keys", "Space Pluck"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let o = osc_param;
    let wave = |bank: usize, n: usize| (bank * waves::BANK_SIZE + n) as f32;
    let slot = |p: &mut Vec<f32>, s: usize, src: usize, dest: usize, amt: f32| {
        p[slot_param(s, 0)] = src as f32;
        p[slot_param(s, 1)] = dest as f32;
        p[slot_param(s, 2)] = amt;
    };
    let set = |p: &mut Vec<f32>, pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    match name {
        "Big Saws" => {
            set(&mut p, &[(o(2, O_MODE), 1.0), (o(2, O_DETUNE), -12.0), (o(2, O_LEVEL), 0.6), (o(0, O_PAN), -0.4), (o(1, O_PAN), 0.4), (o(3, O_MODE), 1.0), (o(3, O_PITCH), -12.0), (o(3, O_LEVEL), 0.4), (o(3, O_BUS), 0.0), (FA_CUTOFF, 4000.0), (DELAY, 0.15), (REVERB, 0.2)]);
        }
        "Wave Pad" => {
            set(&mut p, &[(o(0, O_MODE), 2.0), (o(0, O_WAVE), wave(3, 2)), (o(0, O_PARAM), 0.0), (o(1, O_MODE), 2.0), (o(1, O_WAVE), wave(3, 5)), (o(1, O_PARAM), 0.0), (o(1, O_DETUNE), 9.0), (env_param(0, 0), 900.0), (env_param(0, 3), 2000.0), (FA_CUTOFF, 3500.0), (FA_ENV, 0.1), (REVERB, 0.35)]);
            slot(&mut p, 0, 2, 6, 0.6);
            slot(&mut p, 1, 2, 7, -0.5);
            p[lfo_param(1, 1)] = 0.15;
        }
        "Grain Cloud" => {
            set(&mut p, &[(o(0, O_MODE), 4.0), (o(0, O_WAVE), wave(12, 3)), (o(0, O_PARAM), 0.6), (o(1, O_MODE), 4.0), (o(1, O_WAVE), wave(2, 1)), (o(1, O_PARAM), 0.4), (o(1, O_PITCH), 12.0), (o(0, O_PAN), -0.5), (o(1, O_PAN), 0.5), (env_param(0, 0), 600.0), (env_param(0, 3), 2500.0), (FA_MODE, 1.0), (FA_CUTOFF, 6000.0), (FA_ENV, 0.0), (REVERB, 0.5), (DELAY, 0.25)]);
        }
        "FM Bell" => {
            set(&mut p, &[(o(0, O_MODE), 5.0), (o(0, O_WAVE), wave(6, 2)), (o(0, O_PARAM), 0.5), (o(1, O_MODE), 5.0), (o(1, O_WAVE), wave(7, 0)), (o(1, O_PITCH), 19.0), (o(1, O_LEVEL), 0.3), (FA_MODE, 0.0), (env_param(0, 1), 2500.0), (env_param(0, 2), 0.0), (env_param(0, 3), 1800.0), (REVERB, 0.3)]);
            slot(&mut p, 0, 4, 6, -0.5);
        }
        "FM Bass" => {
            set(&mut p, &[(o(0, O_MODE), 5.0), (o(0, O_WAVE), wave(0, 0)), (o(0, O_PARAM), 0.7), (o(0, O_PITCH), -12.0), (o(1, O_MODE), 1.0), (o(1, O_WAVE), 3.0), (o(1, O_PITCH), -12.0), (o(1, O_LEVEL), 0.5), (FA_CUTOFF, 900.0), (FA_ENV, 0.5), (env_param(1, 1), 250.0), (env_param(1, 2), 0.0), (VOICES, 1.0), (GLIDE, 30.0)]);
            slot(&mut p, 0, 4, 6, -0.6);
        }
        "Shaper Lead" => {
            set(&mut p, &[(o(0, O_MODE), 7.0), (o(0, O_WAVE), 1.0), (o(0, O_PARAM), 0.4), (o(1, O_MODE), 7.0), (o(1, O_WAVE), 2.0), (o(1, O_DETUNE), 8.0), (o(1, O_LEVEL), 0.5), (FA_MODE, 7.0), (FA_CUTOFF, 3000.0), (FA_RESO, 0.3), (VOICES, 1.0), (GLIDE, 60.0), (DELAY, 0.2)]);
            slot(&mut p, 0, 1, 1, 0.12);
            slot(&mut p, 1, 4, 6, 0.4);
            p[lfo_param(0, 2)] = 0.4;
        }
        "Choir Pad" => {
            set(&mut p, &[(o(0, O_MODE), 9.0), (o(0, O_WAVE), 2.0), (o(0, O_PARAM), 0.7), (o(1, O_MODE), 9.0), (o(1, O_WAVE), 0.0), (o(1, O_PARAM), 0.5), (o(1, O_PITCH), 12.0), (o(1, O_LEVEL), 0.3), (env_param(0, 0), 800.0), (env_param(0, 3), 2500.0), (FA_MODE, 1.0), (FA_CUTOFF, 5000.0), (FA_ENV, 0.0), (REVERB, 0.45)]);
        }
        "Organ Pad" => {
            set(&mut p, &[(o(0, O_MODE), 9.0), (o(0, O_WAVE), 3.0), (o(0, O_PARAM), 0.3), (o(1, O_MODE), 0.0), (env_param(0, 0), 120.0), (env_param(0, 3), 900.0), (FA_MODE, 0.0), (REVERB, 0.3)]);
        }
        "Wind" => {
            set(&mut p, &[(o(0, O_MODE), 8.0), (o(0, O_WAVE), 1.0), (o(0, O_PARAM), 0.8), (o(1, O_MODE), 0.0), (FA_MODE, 5.0), (FA_CUTOFF, 900.0), (FA_RESO, 0.7), (FA_ENV, 0.0), (env_param(0, 0), 1200.0), (env_param(0, 3), 2500.0), (REVERB, 0.4)]);
            slot(&mut p, 0, 2, 14, 0.5);
            slot(&mut p, 1, 3, 18, 0.6);
            p[lfo_param(1, 1)] = 0.2;
            p[lfo_param(2, 1)] = 0.7;
        }
        "Phase Brass" => {
            set(&mut p, &[(o(0, O_MODE), 10.0), (o(0, O_WAVE), 0.0), (o(0, O_PARAM), 0.5), (o(1, O_MODE), 10.0), (o(1, O_WAVE), 0.0), (o(1, O_DETUNE), 8.0), (o(1, O_PARAM), 0.4), (FA_MODE, 1.0), (FA_CUTOFF, 6000.0), (FA_ENV, 0.0), (env_param(0, 0), 60.0), (env_param(2, 0), 80.0), (env_param(2, 1), 600.0), (env_param(2, 2), 0.6)]);
            slot(&mut p, 0, 5, 6, 0.5);
            slot(&mut p, 1, 5, 7, 0.5);
            slot(&mut p, 2, 1, 1, 0.08);
            p[lfo_param(0, 2)] = 0.5;
        }
        "Chorus Strings" => {
            set(&mut p, &[(o(0, O_MODE), 3.0), (o(0, O_WAVE), wave(3, 0)), (o(0, O_PARAM), 0.5), (o(1, O_MODE), 3.0), (o(1, O_WAVE), wave(1, 2)), (o(1, O_PARAM), 0.6), (o(1, O_LEVEL), 0.4), (o(1, O_PITCH), -12.0), (env_param(0, 0), 350.0), (env_param(0, 3), 1400.0), (FA_CUTOFF, 4500.0), (FA_ENV, 0.0), (REVERB, 0.35)]);
        }
        "Sweep Matrix" => {
            set(&mut p, &[(o(0, O_MODE), 1.0), (o(0, O_WAVE), 0.0), (o(1, O_MODE), 1.0), (o(1, O_WAVE), 4.0), (o(1, O_PARAM), 0.5), (FA_MODE, 7.0), (FA_CUTOFF, 400.0), (FA_RESO, 0.6), (FA_ENV, 0.0), (FB_MODE, 5.0), (FB_CUTOFF, 1200.0), (FB_RESO, 0.6), (o(1, O_BUS), 1.0), (o(0, O_PAN), -0.6), (o(1, O_PAN), 0.6), (DELAY, 0.3)]);
            slot(&mut p, 0, 2, 14, 0.6);
            slot(&mut p, 1, 3, 15, 0.5);
            slot(&mut p, 2, 1, 7, 0.4);
            p[lfo_param(1, 1)] = 0.12;
            p[lfo_param(2, 0)] = 0.0;
            p[lfo_param(2, 1)] = 0.21;
        }
        "Vocal Wave" => {
            set(&mut p, &[(o(0, O_MODE), 2.0), (o(0, O_WAVE), wave(2, 0)), (o(0, O_PARAM), 0.0), (o(1, O_MODE), 0.0), (FA_MODE, 1.0), (FA_CUTOFF, 7000.0), (FA_ENV, 0.0), (env_param(0, 0), 80.0)]);
            slot(&mut p, 0, 2, 6, 1.0);
            slot(&mut p, 1, 1, 1, 0.07);
            p[lfo_param(1, 1)] = 0.3;
            p[lfo_param(0, 2)] = 0.5;
        }
        "Digital Keys" => {
            set(&mut p, &[(o(0, O_MODE), 6.0), (o(0, O_WAVE), wave(4, 1)), (o(0, O_PARAM), 0.35), (o(1, O_MODE), 2.0), (o(1, O_WAVE), wave(4, 4)), (o(1, O_LEVEL), 0.4), (env_param(0, 1), 1600.0), (env_param(0, 2), 0.25), (env_param(0, 3), 500.0), (FA_CUTOFF, 3000.0), (FA_ENV, 0.3), (VELOCITY, 0.8)]);
            slot(&mut p, 0, 9, 6, 0.4);
        }
        "Space Pluck" => {
            set(&mut p, &[(o(0, O_MODE), 1.0), (o(0, O_WAVE), 5.0), (o(1, O_MODE), 2.0), (o(1, O_WAVE), wave(7, 3)), (o(1, O_PITCH), 12.0), (o(1, O_LEVEL), 0.4), (env_param(0, 1), 600.0), (env_param(0, 2), 0.0), (env_param(0, 3), 600.0), (FA_CUTOFF, 1200.0), (FA_ENV, 0.7), (env_param(1, 1), 300.0), (env_param(1, 2), 0.0), (DELAY, 0.35), (DELAY_TIME, 280.0), (FEEDBACK, 0.5), (REVERB, 0.4)]);
        }
        _ => {}
    }
    p
}

/// An oscillator's wave at `x` (0 to 1, one cycle) as the screen draws it.
pub fn preview(mode: usize, wave: usize, param: f32, x: f32) -> f32 {
    let w = wave % waves_of(mode).max(1);
    let table = |ph: f32| {
        let t = waves::wave(w);
        t[((ph.rem_euclid(1.0)) * t.len() as f32) as usize % t.len()]
    };
    match mode {
        1 => classic(w, x, 0.002, param),
        2 => {
            let next = waves::wave((w + 1).min(waves::WAVE_COUNT - 1));
            table(x) * (1.0 - param) + next[(x * next.len() as f32) as usize % next.len()] * param
        }
        3 | 4 => table(x),
        5 => (TAU * x + 5.0 * param * table(x)).sin(),
        6 => table(x + 0.25 * param * (TAU * x).sin()),
        7 => shaper(w, (TAU * x).sin() * (1.0 + 5.0 * param)) * 0.8,
        8 => {
            let s = ((x * 61.0).floor() * 12.9898).sin() * 43_758.547;
            2.0 * (s - s.floor()) - 1.0
        }
        9 => {
            let t = &pad_tables()[(w % PADS.len()) * 2];
            t[(x * (PAD_N / PAD_CYCLES) as f32) as usize % PAD_N] * 1.4
        }
        10 => super::warp::wave(w, x, param, 0.002),
        _ => 0.0,
    }
}

// ---------------------------------------------------------------- PADsynth

const PAD_N: usize = 1 << 16;
/// The table's fundamental: this many cycles fit in it.
const PAD_CYCLES: usize = 64;
const PAD_HARMONICS: usize = 32;

/// The pad tables: four spectra, each narrow and wide, built once.
fn pad_tables() -> &'static [Vec<f32>] {
    static T: OnceLock<Vec<Vec<f32>>> = OnceLock::new();
    T.get_or_init(|| {
        let mut out = Vec::new();
        for shape in 0..PADS.len() {
            for bw in [8.0f64, 50.0] {
                out.push(build_pad(shape, bw));
            }
        }
        out
    })
}

fn build_pad(shape: usize, bw_cents: f64) -> Vec<f32> {
    let n = PAD_N;
    let mut re = vec![0.0f64; n];
    let mut im = vec![0.0f64; n];
    let mut seed = 0x5EED_0000u32 + shape as u32 * 7919 + bw_cents as u32;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as f64 / u32::MAX as f64
    };
    for h in 1..=PAD_HARMONICS {
        let hf = h as f64;
        let amp = match shape {
            0 => 1.0 / hf,
            1 => {
                if h % 2 == 1 {
                    1.0 / hf
                } else {
                    0.1 / hf
                }
            }
            // A sung "ah" on 220 Hz: its formants.
            2 => {
                let f = 220.0 * hf;
                let g = |c: f64, w: f64| (-((f - c) / w).powi(2)).exp();
                (g(700.0, 180.0) + 0.7 * g(1220.0, 220.0) + 0.3 * g(2600.0, 350.0)) / hf.sqrt() + 0.02 / hf
            }
            _ => match h {
                1 => 1.0,
                2 => 0.8,
                3 => 0.6,
                4 => 0.5,
                6 => 0.3,
                8 => 0.25,
                _ => 0.0,
            },
        };
        if amp <= 0.0 {
            continue;
        }
        let centre = (PAD_CYCLES * h) as f64;
        // PADsynth: each harmonic a band, wider the higher it is.
        let width = ((2f64.powf(bw_cents / 1200.0) - 1.0) * centre).max(0.5);
        let reach = (width * 3.0) as i64 + 1;
        for k in -reach..=reach {
            let bin = centre as i64 + k;
            if bin <= 0 || bin as usize >= n / 2 {
                continue;
            }
            let a = amp * (-0.5 * (k as f64 / width).powi(2)).exp() / width.sqrt();
            let ph = rnd() * std::f64::consts::TAU;
            re[bin as usize] += a * ph.cos();
            im[bin as usize] += a * ph.sin();
            re[n - bin as usize] += a * ph.cos();
            im[n - bin as usize] -= a * ph.sin();
        }
    }
    inverse_fft(&mut re, &mut im);
    let rms = (re.iter().map(|x| x * x).sum::<f64>() / n as f64).sqrt().max(1e-30);
    let mut t: Vec<f32> = re.iter().map(|x| (x / rms * 0.5) as f32).collect();
    t.push(t[0]);
    t
}

/// In-place radix-2 inverse FFT (unscaled).
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

// ---------------------------------------------------------------- pieces

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

fn classic(w: usize, ph: f32, dt: f32, param: f32) -> f32 {
    match w {
        0 => {
            let s = 2.0 * ph - 1.0 - poly_blep(ph, dt);
            // Param drives it a little.
            (s * (1.0 + 2.0 * param)).tanh() / (1.0 + 2.0 * param).tanh()
        }
        1 | 4 | 5 => {
            let base = [0.5, 0.5, 0.5, 0.5, 0.25, 0.1][w];
            let pw = (base + param * if w == 1 { 0.4 } else { 0.2 }).clamp(0.02, 0.98);
            let mut y = if ph < pw { 1.0 } else { -1.0 };
            y += poly_blep(ph, dt);
            y -= poly_blep((ph - pw).rem_euclid(1.0), dt);
            y
        }
        2 => {
            let t = 1.0 - 4.0 * (ph - 0.5).abs();
            (t * (1.0 + 3.0 * param)).clamp(-1.0, 1.0)
        }
        _ => (TAU * ph).sin(),
    }
}

fn shaper(w: usize, x: f32) -> f32 {
    match w {
        0 => x.tanh(),
        1 => (x * 0.5 * PI).sin(),
        2 => {
            let x = x.clamp(-1.0, 1.0);
            4.0 * x * x * x - 3.0 * x
        }
        3 => {
            let x = x.clamp(-1.0, 1.0);
            16.0 * x.powi(5) - 20.0 * x.powi(3) + 5.0 * x
        }
        4 => x.clamp(-1.0, 1.0),
        _ => 2.0 * x.abs().min(1.0) - 1.0,
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

#[derive(Clone, Copy, Default)]
struct Grain {
    ph: f32,
    age: f32,
    len: f32,
    rate: f32,
    on: bool,
}

#[derive(Clone, Copy, Default)]
struct Osc {
    ph: f32,
    copies: [f32; 2],
    grains: [Grain; 6],
    spawn: f32,
    pad: f32,
    pink: [f32; 3],
    lp: f32,
    hold: f32,
}

#[derive(Clone)]
struct Filter {
    s: [f32; 4],
    fb: f32,
    comb: Vec<f32>,
    pos: usize,
}

impl Filter {
    fn new(sr: f32) -> Self {
        Self { s: [0.0; 4], fb: 0.0, comb: vec![0.0; (sr / 20.0) as usize + 4], pos: 0 }
    }

    fn svf(s: &mut [f32], x: f32, g: f32, k: f32) -> (f32, f32, f32) {
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
        let k = 2.0 - 1.9 * reso.clamp(0.0, 1.0);
        let y = match kind {
            1 => Self::svf(&mut self.s[..2], x, g, k).0,
            2 => {
                let (a, _, _) = Self::svf(&mut self.s[..2], x, g, k.max(1.0));
                Self::svf(&mut self.s[2..], a, g, k).0
            }
            3 => Self::svf(&mut self.s[..2], x, g, k).2,
            4 => {
                let (_, _, a) = Self::svf(&mut self.s[..2], x, g, k.max(1.0));
                Self::svf(&mut self.s[2..], a, g, k).2
            }
            5 => Self::svf(&mut self.s[..2], x, g, k).1 * k,
            6 => {
                let (lp, _, hp) = Self::svf(&mut self.s[..2], x, g, k);
                lp + hp
            }
            7 => {
                let kk = 4.2 * reso;
                let gg = g / (1.0 + g);
                let mut u = (x - kk * self.fb).tanh();
                for st in self.s.iter_mut() {
                    let v = (u - *st) * gg;
                    let y = v + *st;
                    *st = y + v;
                    u = y;
                }
                self.fb = u;
                u * (1.0 + 0.6 * reso)
            }
            8 => {
                let (_, bp, _) = Self::svf(&mut self.s[..2], x, g, 0.7);
                x + bp * 4.0 * reso
            }
            9 => {
                let n = self.comb.len();
                let d = (sr / fc).clamp(2.0, (n - 2) as f32);
                let at = self.pos as f32 + n as f32 - d;
                let i0 = at.floor() as usize % n;
                let f = at - at.floor();
                let delayed = self.comb[i0] * (1.0 - f) + self.comb[(i0 + 1) % n] * f;
                let y = x + 0.95 * reso * delayed;
                self.comb[self.pos] = y.clamp(-4.0, 4.0);
                self.pos = (self.pos + 1) % n;
                y * (1.0 - 0.85 * reso)
            }
            _ => x,
        };
        if y.is_finite() {
            y
        } else {
            self.s = [0.0; 4];
            self.fb = 0.0;
            self.comb.fill(0.0);
            0.0
        }
    }
}

/// What the matrix and the knobs make of a voice, worked out every few samples.
#[derive(Clone, Copy, Default)]
struct Ctl {
    pitch: [f32; 4],
    param: [f32; 4],
    level: [f32; 4],
    cut: [f32; 2],
    reso: [f32; 2],
    pan: f32,
    amp: f32,
}

struct Voice {
    note: u8,
    vel: f32,
    gate: bool,
    age: u64,
    pitch: f32,
    target: f32,
    osc: [Osc; 4],
    env: [Adsr; ENVS],
    lfo_ph: [f32; 2],
    lfo_hold: [f32; 2],
    time: f32,
    random: f32,
    filt: [Filter; 2],
    rng: u32,
    ctl: Ctl,
    reads: [Option<TableRead>; 4],
    /// Just struck: its controls are worked out on the next sample.
    fresh: bool,
}

impl Voice {
    fn new(sr: f32) -> Self {
        Self {
            note: 0,
            vel: 0.0,
            gate: false,
            age: 0,
            pitch: 60.0,
            target: 60.0,
            osc: [Osc::default(); 4],
            env: [Adsr::default(); ENVS],
            lfo_ph: [0.0; 2],
            lfo_hold: [0.0; 2],
            time: 0.0,
            random: 0.0,
            filt: [Filter::new(sr), Filter::new(sr)],
            rng: 0x6C07_8965,
            ctl: Ctl::default(),
            reads: [None, None, None, None],
            fresh: false,
        }
    }

    fn active(&self) -> bool {
        self.env[0].stage != 0
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / (1u32 << 24) as f32
    }
}

pub struct QuartetState {
    sr: f32,
    voices: Vec<Voice>,
    clock: u64,
    held: Vec<(u8, f32)>,
    lfo1: f32,
    lfo1_hold: f32,
    lfo1_now: f32,
    lfo_rng: u32,
    delay: [Vec<f32>; 2],
    delay_pos: usize,
    reverb: Reverb,
    pads: &'static [Vec<f32>],
    pub pitch_bend: f32,
}

fn get(p: &[f32], i: usize, or: f32) -> f32 {
    p.get(i).copied().unwrap_or(or)
}

fn choice(p: &[f32], i: usize, n: usize) -> usize {
    (get(p, i, 0.0).round().max(0.0) as usize).min(n.max(1) - 1)
}

fn lfo_shape(w: usize, ph: f32, hold: f32) -> f32 {
    match w {
        0 => (TAU * ph).sin(),
        1 => 1.0 - 4.0 * (ph - 0.5).abs(),
        2 => 2.0 * ph - 1.0,
        3 => 1.0 - 2.0 * ph,
        4 => {
            if ph < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        _ => hold,
    }
}

/// How far a matrix amount moves the pitch (semitones): fine near the
/// middle, two octaves at the ends.
fn pitch_amount(a: f32) -> f32 {
    a * a.abs() * 24.0
}

const CTRL: usize = 16;

impl QuartetState {
    pub fn new(sr: f32) -> Self {
        waves::warm();
        let len = (sr * 1.6) as usize;
        Self {
            sr,
            voices: (0..8).map(|_| Voice::new(sr)).collect(),
            clock: 0,
            held: Vec::new(),
            lfo1: 0.0,
            lfo1_hold: 0.0,
            lfo1_now: 0.0,
            lfo_rng: 0x9E37_79B9,
            delay: [vec![0.0; len], vec![0.0; len]],
            delay_pos: 0,
            reverb: Reverb::new(sr),
            pads: pad_tables(),
            pitch_bend: 0.0,
        }
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
        let clock = self.clock;
        let v = &mut self.voices[k];
        let legato = mono && v.gate && self.held.len() > 1;
        v.target = note as f32;
        if !legato {
            if !v.active() || !mono || !glide {
                v.pitch = note as f32;
            }
            v.rng ^= (clock as u32).wrapping_mul(2_654_435_761) | 1;
            if !v.active() {
                for o in 0..4 {
                    v.osc[o].ph = v.rand();
                    v.osc[o].copies = [v.rand(), v.rand()];
                    v.osc[o].pad = v.rand() * PAD_N as f32;
                    v.osc[o].grains = [Grain::default(); 6];
                }
            }
            for e in v.env.iter_mut() {
                e.gate(true);
            }
            v.lfo_ph = [0.0, 0.0];
            v.time = 0.0;
            v.random = v.rand() * 2.0 - 1.0;
        }
        v.note = note;
        v.vel = vel;
        v.gate = true;
        v.age = clock;
        v.fresh = true;
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.held.retain(|k| k.0 != note);
        let release = |v: &mut Voice| {
            v.gate = false;
            for e in v.env.iter_mut() {
                e.gate(false);
            }
        };
        if Self::voices(p) == 1 {
            let v = &mut self.voices[0];
            if v.note == note {
                if let Some(&(n, vel)) = self.held.last() {
                    v.note = n;
                    v.target = n as f32;
                    v.vel = vel;
                    return;
                }
                release(v);
            }
            return;
        }
        for v in self.voices.iter_mut() {
            if v.gate && v.note == note {
                release(v);
            }
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in self.voices.iter_mut() {
            v.gate = false;
            v.env = [Adsr::default(); ENVS];
        }
        for d in self.delay.iter_mut() {
            d.fill(0.0);
        }
    }

    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[6] = self.lfo1_now;
        let Some(v) = self.voices.iter().filter(|v| v.active()).max_by_key(|v| v.age) else { return };
        out[0] = v.note as f32;
        for e in 0..ENVS {
            out[1 + e] = v.env[e].level;
        }
        out[7] = 1.0;
        for o in 0..4 {
            out[8 + o] = v.ctl.level[o];
            out[12 + o] = v.ctl.param[o];
        }
    }

    /// The matrix and the knobs, for one voice.
    fn control(v: &mut Voice, p: &[f32], lfo1: f32, sr: f32, bend: f32) {
        let mut pitch_all = 0.0;
        let mut pitch = [0.0f32; 4];
        let mut param = [0.0f32; 4];
        let mut level = [0.0f32; 4];
        let mut cut = [0.0f32; 2];
        let mut reso = [0.0f32; 2];
        let mut pan = 0.0;
        let mut amp = 0.0;
        let lfo = |l: usize, v: &Voice| {
            let delay = get(p, lfo_param(l + 1, 2), 0.0);
            let fade = if delay > 0.0 { ((v.time - delay) / 0.3).clamp(0.0, 1.0) } else { 1.0 };
            lfo_shape(choice(p, lfo_param(l + 1, 0), LFO_WAVES.len()), v.lfo_ph[l], v.lfo_hold[l]) * fade
        };
        let lfo1_delay = get(p, lfo_param(0, 2), 0.0);
        let lfo1v = lfo1 * if lfo1_delay > 0.0 { ((v.time - lfo1_delay) / 0.3).clamp(0.0, 1.0) } else { 1.0 };
        for s in 0..SLOTS {
            let src = choice(p, slot_param(s, 0), SOURCES.len());
            let dest = choice(p, slot_param(s, 1), DESTS.len());
            let a = get(p, slot_param(s, 2), 0.0);
            if src == 0 || dest == 0 || a == 0.0 {
                continue;
            }
            let x = match src {
                1 => lfo1v,
                2 => lfo(0, v),
                3 => lfo(1, v),
                4..=7 => v.env[src - 3].level,
                8 => v.env[0].level,
                9 => v.vel,
                10 => (v.note as f32 - 60.0) / 60.0,
                _ => v.random,
            };
            let m = x * a;
            match dest {
                1 => pitch_all += pitch_amount(a) * x,
                2..=5 => pitch[dest - 2] += pitch_amount(a) * x,
                6..=9 => param[dest - 6] += m,
                10..=13 => level[dest - 10] += m,
                14 | 15 => cut[dest - 14] += m * 6.0,
                16 | 17 => reso[dest - 16] += m,
                18 => pan += m,
                _ => amp += m,
            }
        }
        let c = &mut v.ctl;
        for o in 0..4 {
            c.pitch[o] = v.pitch + bend + pitch_all + pitch[o] + get(p, osc_param(o, O_PITCH), 0.0).round() + get(p, osc_param(o, O_DETUNE), 0.0) / 100.0;
            c.param[o] = (get(p, osc_param(o, O_PARAM), 0.3) + param[o]).clamp(0.0, 1.0);
            c.level[o] = (get(p, osc_param(o, O_LEVEL), 0.5) * (1.0 + level[o])).clamp(0.0, 2.0);
        }
        let key = (v.note as f32 - 60.0) / 12.0;
        let env2 = v.env[1].level;
        for f in 0..2 {
            let (ci, ri, ei, ki) = if f == 0 { (FA_CUTOFF, FA_RESO, FA_ENV, FA_KEYS) } else { (FB_CUTOFF, FB_RESO, FB_ENV, FB_KEYS) };
            let oct = get(p, ei, 0.0) * env2 * 6.0 + get(p, ki, 0.5) * key + cut[f];
            c.cut[f] = (get(p, ci, 2000.0) * 2f32.powf(oct)).clamp(20.0, sr * 0.45);
            c.reso[f] = (get(p, ri, 0.2) + reso[f]).clamp(0.0, 1.0);
        }
        c.pan = pan.clamp(-1.0, 1.0);
        c.amp = (1.0 + amp).max(0.0);
        // The waves this block (band-limited for its pitch).
        for o in 0..4 {
            let mode = choice(p, osc_param(o, O_MODE), MODES.len());
            v.reads[o] = if (2..=6).contains(&mode) {
                let w = (get(p, osc_param(o, O_WAVE), 0.0).round().max(0.0) as usize) % waves::WAVE_COUNT;
                let f = 440.0 * 2f32.powf((c.pitch[o] - 69.0) / 12.0);
                // Wave morphs into the next with Param; the others read one wave.
                let mix = if mode == 2 { c.param[o] } else { 0.0 };
                let f_read = if mode == 6 { f * 2.0 } else { f };
                Some(TableRead::new(w, mix, f_read, sr))
            } else {
                None
            };
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let dt = 1.0 / sr;
        let modes: [usize; 4] = std::array::from_fn(|o| choice(p, osc_param(o, O_MODE), MODES.len()));
        let wave: [usize; 4] = std::array::from_fn(|o| get(p, osc_param(o, O_WAVE), 0.0).round().max(0.0) as usize);
        let bus: [f32; 4] = std::array::from_fn(|o| get(p, osc_param(o, O_BUS), 0.0).clamp(0.0, 1.0));
        let pans: [f32; 4] = std::array::from_fn(|o| get(p, osc_param(o, O_PAN), 0.0).clamp(-1.0, 1.0));
        let fmode = [choice(p, FA_MODE, FILTERS.len()), choice(p, FB_MODE, FILTERS.len())];
        let series = get(p, FB_SERIES, 0.0) >= 0.5;
        let env: [[f32; 4]; ENVS] = std::array::from_fn(|e| [get(p, env_param(e, 0), 10.0), get(p, env_param(e, 1), 300.0), get(p, env_param(e, 2), 0.5), get(p, env_param(e, 3), 300.0)]);
        let lfo_rates: [f32; 3] = std::array::from_fn(|l| get(p, lfo_param(l, 1), 1.0));
        let lfo1_wave = choice(p, lfo_param(0, 0), LFO_WAVES.len());
        let glide = get(p, GLIDE, 1.0).max(1.0) / 1000.0;
        let glide_k = 1.0 - (-dt / glide).exp();
        let vel_amt = get(p, VELOCITY, 0.6);
        let volume = get(p, VOLUME, 0.7);
        let bend = self.pitch_bend;
        // Where each bus sits: its oscillators' pans, by level.
        let bus_pan = |b: usize| {
            let (mut s, mut w) = (0.0, 0.0);
            for o in 0..4 {
                if modes[o] == 0 {
                    continue;
                }
                let share = if b == 0 { 1.0 - bus[o] } else { bus[o] } * get(p, osc_param(o, O_LEVEL), 0.5);
                s += pans[o] * share;
                w += share;
            }
            if w > 0.0 {
                s / w
            } else {
                0.0
            }
        };
        let bpan = [bus_pan(0), bus_pan(1)];
        let pads = self.pads;
        for i in 0..n {
            let before = self.lfo1;
            self.lfo1 = (self.lfo1 + lfo_rates[0] * dt).fract();
            if self.lfo1 < before {
                self.lfo_rng ^= self.lfo_rng << 13;
                self.lfo_rng ^= self.lfo_rng >> 17;
                self.lfo_rng ^= self.lfo_rng << 5;
                self.lfo1_hold = (self.lfo_rng >> 8) as f32 / (1u32 << 23) as f32 - 1.0;
            }
            self.lfo1_now = lfo_shape(lfo1_wave, self.lfo1, self.lfo1_hold);
            let (mut out_l, mut out_r) = (0.0f32, 0.0f32);
            for v in self.voices.iter_mut() {
                if !v.active() {
                    continue;
                }
                v.time += dt;
                v.pitch += (v.target - v.pitch) * glide_k;
                for e in 0..ENVS {
                    let [a, d, s, r] = env[e];
                    v.env[e].tick(a, d, s, r, dt);
                }
                for l2 in 0..2 {
                    let b = v.lfo_ph[l2];
                    v.lfo_ph[l2] = (b + lfo_rates[l2 + 1] * dt).fract();
                    if v.lfo_ph[l2] < b {
                        v.lfo_hold[l2] = v.rand() * 2.0 - 1.0;
                    }
                }
                if i % CTRL == 0 || v.fresh {
                    Self::control(v, p, self.lfo1_now, sr, bend);
                    v.fresh = false;
                }
                let c = v.ctl;
                let mut buses = [0.0f32; 2];
                for o in 0..4 {
                    let mode = modes[o];
                    if mode == 0 || c.level[o] <= 0.0 {
                        continue;
                    }
                    let f = 440.0 * 2f32.powf((c.pitch[o] - 69.0) / 12.0);
                    let dph = (f / sr).min(0.45);
                    let pr = c.param[o];
                    let w = wave[o] % waves_of(mode).max(1);
                    let white = if mode == 8 { v.rand() * 2.0 - 1.0 } else { 0.0 };
                    let os = &mut v.osc[o];
                    let read = v.reads[o].as_ref();
                    let tab = |ph: f32| read.map(|r| r.read(ph)).unwrap_or(0.0);
                    let y = match mode {
                        1 => classic(w, os.ph, dph, pr),
                        2 => tab(os.ph),
                        3 => {
                            // Three copies, spread by Param.
                            let spread = pr * 0.012;
                            let mut s = tab(os.ph);
                            for (k, cp) in os.copies.iter_mut().enumerate() {
                                s += tab(*cp);
                                *cp += dph * (1.0 + if k == 0 { spread } else { -spread });
                                if *cp >= 1.0 {
                                    *cp -= 1.0;
                                }
                            }
                            s * 0.55
                        }
                        4 => {
                            // Grains: Param how many a second.
                            os.spawn += (6.0 + 90.0 * pr) * dt;
                            if os.spawn >= 1.0 {
                                os.spawn -= 1.0;
                                if let Some(slot) = os.grains.iter().position(|g| !g.on) {
                                    let ph = (os.ph + 0.37 * slot as f32).fract();
                                    os.grains[slot] = Grain { ph, age: 0.0, len: (0.025 + 0.06 * (1.0 - pr)) * sr, rate: 1.0, on: true };
                                }
                            }
                            let mut s = 0.0;
                            for (k, g) in os.grains.iter_mut().enumerate() {
                                if !g.on {
                                    continue;
                                }
                                if g.rate == 1.0 {
                                    // A little off pitch each.
                                    g.rate = 1.0 + 0.004 * ((k as f32 * 1.7 + os.ph * 13.0).sin());
                                }
                                let wnd = (PI * g.age / g.len).sin().powi(2);
                                s += tab(g.ph) * wnd;
                                g.ph = (g.ph + dph * g.rate).fract();
                                g.age += 1.0;
                                if g.age >= g.len {
                                    g.on = false;
                                }
                            }
                            s * 0.7
                        }
                        5 => {
                            let m = tab(os.ph);
                            (TAU * os.ph + 5.0 * pr * m).sin()
                        }
                        6 => {
                            let m = (TAU * os.ph).sin();
                            tab((os.ph + 0.25 * pr * m).rem_euclid(1.0))
                        }
                        7 => shaper(w, (TAU * os.ph).sin() * (1.0 + 5.0 * pr)) * 0.8,
                        8 => {
                            let raw = match w {
                                0 => white,
                                1 => {
                                    os.pink[0] = 0.997 * os.pink[0] + 0.029 * white;
                                    os.pink[1] = 0.985 * os.pink[1] + 0.032 * white;
                                    os.pink[2] = 0.950 * os.pink[2] + 0.048 * white;
                                    (os.pink[0] + os.pink[1] + os.pink[2] + 0.05 * white) * 2.5
                                }
                                2 => {
                                    if white.abs() > 0.995 - 0.02 * pr {
                                        white.signum()
                                    } else {
                                        0.0
                                    }
                                }
                                _ => {
                                    if os.ph + dph >= 1.0 {
                                        os.hold = white;
                                    }
                                    os.hold
                                }
                            };
                            // Param darkens it.
                            os.lp += (raw - os.lp) * (1.0 - 0.95 * pr);
                            os.lp
                        }
                        9 => {
                            let t = &pads[(w % PADS.len()) * 2..(w % PADS.len()) * 2 + 2];
                            let x = os.pad;
                            let i0 = x as usize % PAD_N;
                            let fr = x - x.floor();
                            let rd = |tb: &[f32]| tb[i0] + (tb[i0 + 1] - tb[i0]) * fr;
                            let y = rd(&t[0]) * (1.0 - pr) + rd(&t[1]) * pr;
                            os.pad += f / sr * (PAD_N / PAD_CYCLES) as f32;
                            if os.pad >= PAD_N as f32 {
                                os.pad -= PAD_N as f32;
                            }
                            y
                        }
                        _ => super::warp::wave(w, os.ph, pr, dph),
                    };
                    os.ph += dph;
                    if os.ph >= 1.0 {
                        os.ph -= 1.0;
                    }
                    let y = y * c.level[o];
                    buses[0] += y * (1.0 - bus[o]);
                    buses[1] += y * bus[o];
                }
                let a = v.filt[0].run(fmode[0], buses[0], c.cut[0], c.reso[0], sr);
                let b_in = if series { a + buses[1] } else { buses[1] };
                let b = v.filt[1].run(fmode[1], b_in, c.cut[1], c.reso[1], sr);
                let vel = 1.0 - vel_amt + vel_amt * v.vel;
                let g = v.env[0].level * vel * c.amp;
                // A after B in series: only B is heard of the two.
                let (la, ra) = if series { (0.0, 0.0) } else { super::pan_gains((bpan[0] + c.pan).clamp(-1.0, 1.0)) };
                let (lb, rb) = super::pan_gains((bpan[1] + c.pan).clamp(-1.0, 1.0));
                out_l += (a * la + b * lb) * g;
                out_r += (a * ra + b * rb) * g;
            }
            l[i] = out_l * 0.24 * volume;
            r[i] = out_r * 0.24 * volume;
        }
        // The delay: echoes going from side to side.
        let mix = get(p, DELAY, 0.0);
        if mix > 0.0 {
            let len = self.delay[0].len();
            let d = ((get(p, DELAY_TIME, 375.0) / 1000.0 * sr) as usize).clamp(1, len - 1);
            let fb = get(p, FEEDBACK, 0.35);
            for i in 0..n {
                let at = (self.delay_pos + len - d) % len;
                let (dl, dr) = (self.delay[0][at], self.delay[1][at]);
                self.delay[0][self.delay_pos] = l[i] + dr * fb;
                self.delay[1][self.delay_pos] = r[i] * 0.3 + dl * fb;
                self.delay_pos = (self.delay_pos + 1) % len;
                l[i] += dl * mix;
                r[i] += dr * mix;
            }
        }
        let rev = get(p, REVERB, 0.0);
        if rev > 0.0 {
            self.reverb.process(get(p, REVERB_SIZE, 0.6), 0.4, 1.0, 10.0, 100.0, rev * 0.6, &mut l[..n], &mut r[..n]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn run(st: &mut QuartetState, p: &[f32], secs: f32) -> (Vec<f32>, Vec<f32>) {
        let n = (secs * SR) as usize;
        let (mut ol, mut or) = (Vec::with_capacity(n), Vec::with_capacity(n));
        while ol.len() < n {
            let mut l = vec![0.0; 256];
            let mut r = vec![0.0; 256];
            st.render(p, &mut l, &mut r);
            ol.extend(l);
            or.extend(r);
        }
        (ol, or)
    }

    fn rms(v: &[f32]) -> f32 {
        (v.iter().map(|x| x * x).sum::<f32>() / v.len().max(1) as f32).sqrt()
    }

    #[test]
    fn its_parameters_are_in_order() {
        assert_eq!(SPECS.len(), COUNT);
        assert_eq!(SPECS[osc_param(3, O_PAN)].name, "Pan 4");
        assert_eq!(SPECS[FA_MODE].name, "Filter A");
        assert_eq!(SPECS[FB_SERIES].name, "B after A");
        assert_eq!(SPECS[env_param(0, 0)].name, "Attack");
        assert_eq!(SPECS[env_param(4, 3)].name, "Release 5");
        assert_eq!(SPECS[lfo_param(2, 2)].name, "LFO 3 Delay");
        assert_eq!(SPECS[slot_param(7, 2)].name, "Amount 8");
        assert_eq!(SPECS[DELAY].name, "Delay");
        assert_eq!(SPECS[VOLUME].name, "Volume");
    }

    #[test]
    fn every_mode_sounds_at_its_pitch() {
        for (m, name) in MODES.iter().enumerate().skip(1) {
            let mut p = preset("Init");
            p[osc_param(0, O_MODE)] = m as f32;
            p[osc_param(1, O_MODE)] = 0.0;
            p[FA_MODE] = 0.0;
            p[osc_param(0, O_PARAM)] = 0.2;
            let mut st = QuartetState::new(SR);
            st.note_on(&p, 57, 1.0);
            let (l, _) = run(&mut st, &p, 0.5);
            assert!(rms(&l[4800..]) > 0.01, "{name}: {}", rms(&l[4800..]));
            assert!(l.iter().all(|x| x.is_finite() && x.abs() < 1.5), "{name}");
            if !matches!(m, 4 | 8 | 9) {
                // The fundamental by autocorrelation (220 Hz: 218 samples).
                let x = &l[9600..19200];
                let lag_of = |lag: usize| x.iter().zip(&x[lag..]).map(|(a, b)| a * b).sum::<f32>();
                let best = (150..300).max_by(|a, b| lag_of(*a).total_cmp(&lag_of(*b))).unwrap();
                assert!((best as f32 - 218.2).abs() < 3.0, "{name}: {best}");
            }
        }
    }

    #[test]
    fn the_pad_plays_its_pitch_as_a_band() {
        let t = &pad_tables()[0];
        assert_eq!(t.len(), PAD_N + 1);
        let r = rms(&t[..PAD_N]);
        assert!((r - 0.5).abs() < 0.05, "{r}");
    }

    #[test]
    fn the_matrix_moves_what_it_says() {
        let mut p = preset("Init");
        p[FA_MODE] = 0.0;
        p[osc_param(1, O_MODE)] = 0.0;
        p[env_param(0, 2)] = 1.0;
        // LFO 1, a square, on the pitch: two pitches.
        p[lfo_param(0, 0)] = 4.0;
        p[lfo_param(0, 1)] = 2.0;
        p[slot_param(0, 0)] = 1.0;
        p[slot_param(0, 1)] = 1.0;
        p[slot_param(0, 2)] = 0.5;
        let mut st = QuartetState::new(SR);
        st.note_on(&p, 57, 1.0);
        let (l, _) = run(&mut st, &p, 1.0);
        let crossings = |x: &[f32]| x.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        let a = crossings(&l[2400..12000]);
        let b = crossings(&l[14400..24000]);
        assert!((a as f32 / b as f32 - 1.0).abs() > 0.3, "{a} {b}");
    }

    #[test]
    fn pans_and_buses_go_where_they_are_sent() {
        let mut p = preset("Init");
        p[osc_param(1, O_MODE)] = 0.0;
        p[osc_param(0, O_PAN)] = -1.0;
        let mut st = QuartetState::new(SR);
        st.note_on(&p, 60, 1.0);
        let (l, r) = run(&mut st, &p, 0.3);
        assert!(rms(&l) > 10.0 * rms(&r), "{} {}", rms(&l), rms(&r));
    }

    #[test]
    fn every_preset_sounds_and_lets_go() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = QuartetState::new(SR);
            for n in [48, 55, 60] {
                st.note_on(&p, n, 0.9);
            }
            let (l, r) = run(&mut st, &p, 0.6);
            assert!(l.iter().chain(&r).all(|x| x.is_finite() && x.abs() < 1.5), "{name}");
            assert!(rms(&l) + rms(&r) > 0.01, "{name}: {}", rms(&l));
            for n in [48, 55, 60] {
                st.note_off(&p, n);
            }
            run(&mut st, &p, 14.0);
            assert!(st.voices.iter().all(|v| !v.active()), "{name} rings on");
        }
    }
}

