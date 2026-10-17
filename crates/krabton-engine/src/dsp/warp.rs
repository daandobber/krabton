//! Warp: phase distortion, the way the digital synths of the mid eighties
//! made their sound. A cosine read at a bent pace: the more its phase is
//! bent (*Warp*), the brighter it gets, from a sine to a saw, a square, a
//! pulse, a doubled sine, a saw that stops halfway, or one of the three
//! resonant waves (a fast cosine inside a saw, a triangle or a trapezoid
//! window, sweeping like a resonant filter). Two lines, each its own wave
//! (or two waves taking turns, a cycle each), its own bend and level, as
//! one line, the other, a line and a detuned copy of itself, or both, the
//! second ringed by the first or made noisy. And the envelopes of those
//! machines: eight steps of a rate and a level each, a step to hold on
//! while the key is down and a step to end on, for each line's warp and
//! amp and for the pitch, drawn and dragged on the screen. Then vibrato
//! (four shapes, a delay) and a chorus. Krabton's own, after the way it
//! was described; no code of any emulation is used.

use crate::params::{ParamSpec, Unit, SCREEN_ONLY};
use std::f32::consts::TAU;

pub const LINE: usize = 0;
pub const OCTAVE: usize = 1;
pub const DETUNE_NOTE: usize = 2;
pub const DETUNE: usize = 3;
pub const RING: usize = 4;
pub const NOISE: usize = 5;
pub const WAVE1_A: usize = 6;
pub const WAVE1_B: usize = 7;
pub const WARP1: usize = 8;
pub const WARP1_KEYS: usize = 9;
pub const AMP1_KEYS: usize = 10;
pub const LEVEL1: usize = 11;
pub const WAVE2_A: usize = 12;
pub const WAVE2_B: usize = 13;
pub const WARP2: usize = 14;
pub const WARP2_KEYS: usize = 15;
pub const AMP2_KEYS: usize = 16;
pub const LEVEL2: usize = 17;
pub const VIB_WAVE: usize = 18;
pub const VIB_DELAY: usize = 19;
pub const VIB_RATE: usize = 20;
pub const VIB_DEPTH: usize = 21;
pub const PITCH_ENV: usize = 22;
pub const VOICES: usize = 23;
pub const GLIDE: usize = 24;
pub const VELOCITY: usize = 25;
pub const CHORUS: usize = 26;
pub const CHORUS_RATE: usize = 27;
pub const VOLUME: usize = 28;

/// The envelopes: eight rates, eight levels, the step it holds on and the
/// step it ends on, each.
pub const ENV_BASE: usize = 29;
pub const ENV_LEN: usize = 18;
pub const ENVS: usize = 5;
pub const ENV_WARP1: usize = 0;
pub const ENV_AMP1: usize = 1;
pub const ENV_WARP2: usize = 2;
pub const ENV_AMP2: usize = 3;
pub const ENV_PITCH: usize = 4;
pub const ENV_NAMES: [&str; ENVS] = ["Warp 1", "Amp 1", "Warp 2", "Amp 2", "Pitch"];
pub const RATE: usize = 0;
pub const LEVEL: usize = 8;
pub const SUSTAIN: usize = 16;
pub const END: usize = 17;
pub const COUNT: usize = ENV_BASE + ENVS * ENV_LEN;

pub const fn env_param(env: usize, k: usize) -> usize {
    ENV_BASE + env * ENV_LEN + k
}

pub const WAVES: &[&str] = &["Saw", "Square", "Pulse", "Double", "Saw Pulse", "Reso Saw", "Reso Tri", "Reso Trap"];
pub const WAVES_B: &[&str] = &["Off", "Saw", "Square", "Pulse", "Double", "Saw Pulse", "Reso Saw", "Reso Tri", "Reso Trap"];
pub const LINES: &[&str] = &["1", "2", "1+1'", "1+2'"];
pub const VIB_WAVES: &[&str] = &["Triangle", "Saw Up", "Saw Down", "Square"];

const KNOBS: [ParamSpec; ENV_BASE] = [
    ParamSpec::choice("Line", "Line", LINES, 3),
    ParamSpec::int("Octave", "Line", -2.0, 2.0, 0.0, Unit::None),
    ParamSpec::int("Detune Note", "Line", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Detune", "Line", 0.0, 100.0, 6.0, Unit::Cents),
    ParamSpec::toggle("Ring", "Line", false),
    ParamSpec::toggle("Noise", "Line", false),
    ParamSpec::choice("Wave 1", "Line 1", WAVES, 0),
    ParamSpec::choice("Wave 1 B", "Line 1", WAVES_B, 0),
    ParamSpec::lin("Warp 1", "Line 1", 0.0, 1.0, 0.75, Unit::Percent),
    ParamSpec::int("Warp Keys 1", "Line 1", 0.0, 9.0, 2.0, Unit::None),
    ParamSpec::int("Amp Keys 1", "Line 1", 0.0, 9.0, 2.0, Unit::None),
    ParamSpec::lin("Level 1", "Line 1", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::choice("Wave 2", "Line 2", WAVES, 1),
    ParamSpec::choice("Wave 2 B", "Line 2", WAVES_B, 0),
    ParamSpec::lin("Warp 2", "Line 2", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::int("Warp Keys 2", "Line 2", 0.0, 9.0, 2.0, Unit::None),
    ParamSpec::int("Amp Keys 2", "Line 2", 0.0, 9.0, 2.0, Unit::None),
    ParamSpec::lin("Level 2", "Line 2", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::choice("Vib Wave", "Vibrato", VIB_WAVES, 0),
    ParamSpec::lin("Vib Delay", "Vibrato", 0.0, 3.0, 0.4, Unit::Sec),
    ParamSpec::lin("Vib Rate", "Vibrato", 0.5, 12.0, 5.5, Unit::Hz),
    ParamSpec::lin("Vib Depth", "Vibrato", 0.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::lin("Pitch Env", "Play", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::int("Voices", "Play", 1.0, 8.0, 8.0, Unit::None),
    ParamSpec::log("Glide", "Play", 1.0, 2000.0, 1.0, Unit::Ms),
    ParamSpec::lin("Velocity", "Play", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Chorus", "Output", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Chorus Rate", "Output", 0.1, 3.0, 0.6, Unit::Hz),
    ParamSpec::lin("Volume", "Output", 0.0, 1.0, 0.7, Unit::Percent),
];

const STEP_NAMES: [&str; ENV_LEN] = ["Rate 1", "Rate 2", "Rate 3", "Rate 4", "Rate 5", "Rate 6", "Rate 7", "Rate 8", "Level 1", "Level 2", "Level 3", "Level 4", "Level 5", "Level 6", "Level 7", "Level 8", "Sustain", "End"];

/// The envelopes it starts with: the amps up at once, down a little to
/// hold, then away; the warps open and fall back; the pitch still.
const fn env_default(e: usize, k: usize) -> f32 {
    let amp = e == ENV_AMP1 || e == ENV_AMP2;
    let pitch = e == ENV_PITCH;
    if k < LEVEL {
        let amp_rates = [99.0, 70.0, 62.0, 50.0, 50.0, 50.0, 50.0, 50.0];
        let warp_rates = [86.0, 62.0, 58.0, 50.0, 50.0, 50.0, 50.0, 50.0];
        if pitch {
            50.0
        } else if amp {
            amp_rates[k]
        } else {
            warp_rates[k]
        }
    } else if k < SUSTAIN {
        let amp_levels = [99.0, 80.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let warp_levels = [99.0, 45.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        if pitch {
            0.0
        } else if amp {
            amp_levels[k - LEVEL]
        } else {
            warp_levels[k - LEVEL]
        }
    } else if k == SUSTAIN {
        if pitch {
            1.0
        } else {
            2.0
        }
    } else if pitch {
        1.0
    } else {
        3.0
    }
}

const fn build() -> [ParamSpec; COUNT] {
    let mut out = [KNOBS[0]; COUNT];
    let mut i = 0;
    while i < ENV_BASE {
        out[i] = KNOBS[i];
        i += 1;
    }
    let mut e = 0;
    while e < ENVS {
        let mut k = 0;
        while k < ENV_LEN {
            let (min, max) = if k < SUSTAIN { (0.0, 99.0) } else { (1.0, 8.0) };
            out[env_param(e, k)] = ParamSpec::int(STEP_NAMES[k], SCREEN_ONLY, min, max, env_default(e, k), Unit::None);
            k += 1;
        }
        e += 1;
    }
    out
}

static ALL: [ParamSpec; COUNT] = build();
pub static SPECS: &[ParamSpec] = &ALL;

/// Sets an envelope: (rate, level) per step, the step it holds on and the
/// one it ends on (counting from 1).
pub fn set_env(p: &mut [f32], e: usize, steps: &[(f32, f32)], sustain: usize, end: usize) {
    for k in 0..8 {
        let (r, l) = steps.get(k).copied().unwrap_or((50.0, 0.0));
        p[env_param(e, RATE + k)] = r;
        p[env_param(e, LEVEL + k)] = l;
    }
    p[env_param(e, SUSTAIN)] = sustain as f32;
    p[env_param(e, END)] = end as f32;
}

pub fn preset_names() -> &'static [&'static str] {
    &["Init", "Brass", "Strings", "Funk Bass", "Electric Piano", "Organ", "Bell", "Reso Sweep", "Lead", "Choir", "Clav", "Pluck", "Sweep Pad", "Wind", "Ring Bell", "Harp", "Square Bass", "Glass Pad"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set = |p: &mut Vec<f32>, pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    match name {
        "Brass" => {
            set(&mut p, &[(LINE, 3.0), (WAVE1_A, 0.0), (WAVE2_A, 0.0), (DETUNE, 7.0), (WARP1, 0.85), (WARP2, 0.7), (VIB_DEPTH, 12.0), (VIB_DELAY, 0.5), (LEVEL2, 0.7)]);
            set_env(&mut p, ENV_WARP1, &[(62.0, 99.0), (45.0, 70.0), (60.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_WARP2, &[(58.0, 90.0), (45.0, 60.0), (60.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP1, &[(78.0, 99.0), (60.0, 88.0), (62.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP2, &[(74.0, 99.0), (60.0, 85.0), (62.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_PITCH, &[(85.0, 6.0), (70.0, 0.0)], 2, 2);
            p[PITCH_ENV] = -2.0;
        }
        "Strings" => {
            set(&mut p, &[(LINE, 2.0), (WAVE1_A, 0.0), (DETUNE, 9.0), (WARP1, 0.5), (CHORUS, 0.55), (VIB_DEPTH, 8.0), (VIB_DELAY, 0.6)]);
            set_env(&mut p, ENV_WARP1, &[(52.0, 80.0), (40.0, 60.0), (50.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP1, &[(58.0, 99.0), (50.0, 92.0), (52.0, 0.0)], 2, 3);
        }
        "Funk Bass" => {
            set(&mut p, &[(LINE, 0.0), (WAVE1_A, 5.0), (OCTAVE, -1.0), (WARP1, 0.9), (VOICES, 1.0), (GLIDE, 25.0), (VELOCITY, 0.8)]);
            set_env(&mut p, ENV_WARP1, &[(99.0, 99.0), (66.0, 18.0), (70.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP1, &[(99.0, 99.0), (60.0, 72.0), (80.0, 0.0)], 2, 3);
        }
        "Electric Piano" => {
            set(&mut p, &[(LINE, 3.0), (WAVE1_A, 3.0), (WAVE2_A, 2.0), (DETUNE_NOTE, 12.0), (DETUNE, 3.0), (WARP1, 0.3), (WARP2, 0.55), (LEVEL2, 0.35), (CHORUS, 0.35), (VELOCITY, 0.8)]);
            set_env(&mut p, ENV_WARP1, &[(99.0, 60.0), (45.0, 0.0), (70.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_WARP2, &[(99.0, 99.0), (60.0, 0.0), (70.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP1, &[(99.0, 99.0), (40.0, 0.0), (70.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP2, &[(99.0, 99.0), (58.0, 0.0), (70.0, 0.0)], 2, 3);
        }
        "Organ" => {
            set(&mut p, &[(LINE, 3.0), (WAVE1_A, 1.0), (WAVE2_A, 1.0), (DETUNE_NOTE, 12.0), (DETUNE, 2.0), (WARP1, 0.3), (WARP2, 0.25), (LEVEL2, 0.5), (CHORUS, 0.45), (VELOCITY, 0.0)]);
            set_env(&mut p, ENV_WARP1, &[(99.0, 99.0), (99.0, 99.0), (90.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_WARP2, &[(99.0, 99.0), (99.0, 99.0), (90.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP1, &[(99.0, 99.0), (99.0, 99.0), (82.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP2, &[(99.0, 99.0), (99.0, 99.0), (82.0, 0.0)], 2, 3);
        }
        "Bell" => {
            set(&mut p, &[(LINE, 3.0), (WAVE1_A, 3.0), (WAVE2_A, 2.0), (DETUNE_NOTE, 19.0), (DETUNE, 14.0), (WARP1, 0.4), (WARP2, 0.8), (LEVEL2, 0.5), (CHORUS, 0.3)]);
            set_env(&mut p, ENV_WARP1, &[(99.0, 70.0), (40.0, 10.0), (50.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_WARP2, &[(99.0, 99.0), (48.0, 0.0), (50.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP1, &[(99.0, 99.0), (32.0, 0.0), (48.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP2, &[(99.0, 99.0), (42.0, 0.0), (48.0, 0.0)], 2, 3);
        }
        "Reso Sweep" => {
            set(&mut p, &[(LINE, 2.0), (WAVE1_A, 5.0), (DETUNE, 5.0), (WARP1, 1.0), (WARP1_KEYS, 0.0), (CHORUS, 0.3)]);
            set_env(&mut p, ENV_WARP1, &[(38.0, 99.0), (34.0, 20.0), (45.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP1, &[(80.0, 99.0), (55.0, 85.0), (55.0, 0.0)], 2, 3);
        }
        "Lead" => {
            set(&mut p, &[(LINE, 2.0), (WAVE1_A, 1.0), (DETUNE, 11.0), (WARP1, 0.8), (VOICES, 1.0), (GLIDE, 60.0), (VIB_DEPTH, 18.0), (VIB_DELAY, 0.3), (VIB_RATE, 6.0)]);
            set_env(&mut p, ENV_WARP1, &[(90.0, 99.0), (55.0, 75.0), (60.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP1, &[(95.0, 99.0), (70.0, 92.0), (68.0, 0.0)], 2, 3);
        }
        "Choir" => {
            set(&mut p, &[(LINE, 3.0), (WAVE1_A, 6.0), (WAVE2_A, 6.0), (DETUNE, 12.0), (WARP1, 0.35), (WARP2, 0.45), (LEVEL2, 0.7), (CHORUS, 0.7), (VIB_DEPTH, 10.0), (VIB_DELAY, 0.8)]);
            set_env(&mut p, ENV_WARP1, &[(45.0, 99.0), (40.0, 80.0), (50.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_WARP2, &[(42.0, 99.0), (40.0, 70.0), (50.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP1, &[(50.0, 99.0), (50.0, 95.0), (48.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP2, &[(46.0, 99.0), (50.0, 95.0), (48.0, 0.0)], 2, 3);
        }
        "Clav" => {
            set(&mut p, &[(LINE, 3.0), (WAVE1_A, 2.0), (WAVE2_A, 4.0), (DETUNE_NOTE, 12.0), (WARP1, 0.9), (WARP2, 0.8), (LEVEL2, 0.4), (VELOCITY, 0.9)]);
            set_env(&mut p, ENV_WARP1, &[(99.0, 99.0), (62.0, 30.0), (80.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_WARP2, &[(99.0, 99.0), (66.0, 20.0), (80.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP1, &[(99.0, 99.0), (56.0, 30.0), (85.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP2, &[(99.0, 99.0), (60.0, 20.0), (85.0, 0.0)], 2, 3);
        }
        "Pluck" => {
            set(&mut p, &[(LINE, 2.0), (WAVE1_A, 4.0), (DETUNE, 6.0), (WARP1, 0.95), (CHORUS, 0.25), (VELOCITY, 0.8)]);
            set_env(&mut p, ENV_WARP1, &[(99.0, 99.0), (64.0, 0.0), (80.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP1, &[(99.0, 99.0), (52.0, 0.0), (75.0, 0.0)], 2, 3);
        }
        "Sweep Pad" => {
            set(&mut p, &[(LINE, 3.0), (WAVE1_A, 0.0), (WAVE1_B, 6.0), (WAVE2_A, 7.0), (DETUNE, 8.0), (WARP1, 0.8), (WARP2, 0.7), (LEVEL2, 0.6), (CHORUS, 0.6)]);
            set_env(&mut p, ENV_WARP1, &[(36.0, 99.0), (30.0, 30.0), (32.0, 80.0), (30.0, 40.0), (45.0, 0.0)], 4, 5);
            set_env(&mut p, ENV_WARP2, &[(30.0, 90.0), (28.0, 20.0), (30.0, 70.0), (30.0, 30.0), (45.0, 0.0)], 4, 5);
            set_env(&mut p, ENV_AMP1, &[(48.0, 99.0), (50.0, 90.0), (45.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP2, &[(44.0, 99.0), (50.0, 90.0), (45.0, 0.0)], 2, 3);
        }
        "Wind" => {
            set(&mut p, &[(LINE, 3.0), (NOISE, 1.0), (WAVE1_A, 6.0), (WAVE2_A, 5.0), (WARP1, 0.5), (WARP2, 0.9), (LEVEL1, 0.25), (LEVEL2, 0.9), (CHORUS, 0.5)]);
            set_env(&mut p, ENV_WARP2, &[(30.0, 99.0), (25.0, 30.0), (28.0, 80.0), (40.0, 0.0)], 3, 4);
            set_env(&mut p, ENV_AMP1, &[(40.0, 99.0), (50.0, 90.0), (40.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP2, &[(36.0, 99.0), (50.0, 90.0), (40.0, 0.0)], 2, 3);
        }
        "Ring Bell" => {
            set(&mut p, &[(LINE, 3.0), (RING, 1.0), (WAVE1_A, 3.0), (WAVE2_A, 0.0), (DETUNE_NOTE, 7.0), (DETUNE, 30.0), (WARP1, 0.2), (WARP2, 0.6), (LEVEL1, 0.4), (LEVEL2, 0.9)]);
            set_env(&mut p, ENV_AMP1, &[(99.0, 99.0), (36.0, 0.0), (50.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP2, &[(99.0, 99.0), (40.0, 0.0), (50.0, 0.0)], 2, 3);
        }
        "Harp" => {
            set(&mut p, &[(LINE, 2.0), (WAVE1_A, 7.0), (DETUNE, 4.0), (WARP1, 0.7), (CHORUS, 0.4), (VELOCITY, 0.7)]);
            set_env(&mut p, ENV_WARP1, &[(99.0, 99.0), (50.0, 0.0), (60.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP1, &[(99.0, 99.0), (40.0, 0.0), (55.0, 0.0)], 2, 3);
        }
        "Square Bass" => {
            set(&mut p, &[(LINE, 0.0), (WAVE1_A, 1.0), (OCTAVE, -1.0), (WARP1, 0.7), (VOICES, 1.0), (GLIDE, 15.0)]);
            set_env(&mut p, ENV_WARP1, &[(99.0, 99.0), (70.0, 40.0), (80.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP1, &[(99.0, 99.0), (75.0, 85.0), (85.0, 0.0)], 2, 3);
        }
        "Glass Pad" => {
            set(&mut p, &[(LINE, 3.0), (WAVE1_A, 3.0), (WAVE2_A, 7.0), (DETUNE_NOTE, 12.0), (DETUNE, 7.0), (WARP1, 0.3), (WARP2, 0.5), (LEVEL2, 0.45), (CHORUS, 0.7), (VIB_DEPTH, 6.0)]);
            set_env(&mut p, ENV_WARP2, &[(40.0, 99.0), (35.0, 40.0), (45.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP1, &[(46.0, 99.0), (50.0, 90.0), (42.0, 0.0)], 2, 3);
            set_env(&mut p, ENV_AMP2, &[(40.0, 99.0), (50.0, 85.0), (42.0, 0.0)], 2, 3);
        }
        // "Init": one line, a saw that opens and closes.
        _ => set(&mut p, &[(LINE, 0.0)]),
    }
    p
}

/// How long a step at this rate takes over the whole range (s): 99 is a
/// couple of milliseconds, 0 several seconds.
pub fn rate_secs(rate: f32) -> f32 {
    0.0015 * 2f32.powf((99.0 - rate.clamp(0.0, 99.0)) / 8.2)
}

/// A phase-distorted wave at phase `ph` (0 to 1), bent by `d` (0: a
/// cosine), `dt` the phase it moves a sample (so no bend gets steeper than
/// a couple of samples).
pub fn wave(w: usize, ph: f32, d: f32, dt: f32) -> f32 {
    let d = d.clamp(0.0, 1.0);
    let sharp = (2.5 * dt).clamp(0.004, 0.2);
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let saw_warp = |x: f32, m: f32| if x < m { 0.5 * x / m } else { 0.5 + 0.5 * (x - m) / (1.0 - m) };
    let resonant = |window: f32| {
        // A cosine that runs faster as the warp opens, inside a window
        // that falls to nothing at the end of each cycle.
        let r = 1.0 + 15.0 * d;
        let reso = 1.0 - (1.0 - (TAU * r * ph).cos()) * window;
        lerp((TAU * ph).cos(), reso, (3.0 * d).min(1.0))
    };
    match w {
        // Saw: up fast, down slowly.
        0 => (TAU * saw_warp(ph, lerp(0.5, sharp, d))).cos(),
        // Square: down at once and held, up at once and held.
        1 => {
            let m = lerp(0.5, sharp, d);
            let w = if ph < 0.5 { 0.5 * (ph / m).min(1.0) } else { 0.5 + 0.5 * ((ph - 0.5) / m).min(1.0) };
            (TAU * w).cos()
        }
        // Pulse: the whole cycle (down and up) in a sliver, then held.
        2 => (TAU * (ph / lerp(1.0, 2.0 * sharp, d)).min(1.0)).cos(),
        // Double: a whole cycle in the first half, then held.
        3 => {
            let w = if ph < 0.5 { lerp(ph, 2.0 * ph, d) } else { lerp(ph, 1.0, d) };
            (TAU * w).cos()
        }
        // Saw pulse: a saw squeezed into the first half, then held.
        4 => {
            let m = lerp(0.5, 2.0 * sharp, d);
            let flat = if ph < 0.5 { saw_warp(2.0 * ph, m) } else { 1.0 };
            (TAU * lerp(ph, flat, d)).cos()
        }
        5 => resonant(1.0 - ph),
        6 => resonant(1.0 - (2.0 * ph - 1.0).abs()),
        _ => resonant((2.0 * (1.0 - ph)).min(1.0)),
    }
}

/// An envelope of eight steps.
#[derive(Clone, Copy, Default, Debug)]
pub struct StepEnv {
    /// 0 to 99.
    pub level: f32,
    /// The step it is going to (8: falling away after its end).
    pub step: usize,
    pub on: bool,
    released: bool,
}

impl StepEnv {
    /// From where it is (so a key played again does not click).
    pub fn start(&mut self) {
        self.on = true;
        self.released = false;
        self.step = 0;
    }

    pub fn release(&mut self, p: &[f32], e: usize) {
        if !self.on {
            return;
        }
        self.released = true;
        let sus = Self::step_of(p, e, SUSTAIN);
        let end = Self::step_of(p, e, END);
        // Past the step it held on.
        if self.step <= sus {
            self.step = (sus + 1).min(end);
            if sus >= end {
                self.step = 8;
            }
        }
    }

    fn step_of(p: &[f32], e: usize, k: usize) -> usize {
        (p.get(env_param(e, k)).copied().unwrap_or(1.0).round().clamp(1.0, 8.0) as usize) - 1
    }

    /// One sample; `speed` makes it faster (higher keys).
    pub fn tick(&mut self, p: &[f32], e: usize, dt: f32, speed: f32) -> f32 {
        if !self.on {
            return 0.0;
        }
        let end = Self::step_of(p, e, END);
        let sus = Self::step_of(p, e, SUSTAIN);
        let get = |k: usize| p.get(env_param(e, k)).copied().unwrap_or(0.0);
        let (target, rate) = if self.step >= 8 { (0.0, get(RATE + end)) } else { (get(LEVEL + self.step).clamp(0.0, 99.0), get(RATE + self.step)) };
        let delta = 99.0 * dt * speed / rate_secs(rate);
        if self.level < target {
            self.level = (self.level + delta).min(target);
        } else {
            self.level = (self.level - delta).max(target);
        }
        if (self.level - target).abs() < 1e-4 {
            if self.step >= 8 {
                self.on = false;
                self.level = 0.0;
            } else if self.released {
                if self.step >= end {
                    if target <= 1e-4 {
                        self.on = false;
                    } else {
                        self.step = 8;
                    }
                } else {
                    self.step += 1;
                }
            } else if self.step != sus && self.step < end {
                self.step += 1;
            }
        }
        self.level
    }
}

#[derive(Clone, Copy, Default)]
struct Voice {
    note: u8,
    vel: f32,
    gate: bool,
    age: u64,
    pitch: f32,
    target: f32,
    ph: [f32; 2],
    /// Which of a line's two waves this cycle is.
    second: [bool; 2],
    env: [StepEnv; ENVS],
    time: f32,
    noise: f32,
    noise_ph: f32,
    rng: u32,
    dc: f32,
}

impl Voice {
    fn active(&self) -> bool {
        self.env[ENV_AMP1].on || self.env[ENV_AMP2].on
    }
}

pub struct WarpState {
    sr: f32,
    voices: [Voice; 8],
    clock: u64,
    held: Vec<(u8, f32)>,
    chorus: [Vec<f32>; 2],
    chorus_pos: usize,
    chorus_ph: f32,
    pub pitch_bend: f32,
}

fn get(p: &[f32], i: usize, or: f32) -> f32 {
    p.get(i).copied().unwrap_or(or)
}

fn choice(p: &[f32], i: usize, n: usize) -> usize {
    (get(p, i, 0.0).round().max(0.0) as usize).min(n - 1)
}

impl WarpState {
    pub fn new(sr: f32) -> Self {
        let len = (sr * 0.03) as usize + 4;
        Self { sr, voices: [Voice::default(); 8], clock: 0, held: Vec::new(), chorus: [vec![0.0; len], vec![0.0; len]], chorus_pos: 0, chorus_ph: 0.0, pitch_bend: 0.0 }
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
        let v = &mut self.voices[k];
        let legato = mono && v.gate && self.held.len() > 1;
        v.target = note as f32;
        if !legato {
            if !v.active() {
                v.pitch = note as f32;
                v.ph = [0.0; 2];
                v.second = [false; 2];
                v.rng = 0x9E37_79B9 ^ (self.clock as u32).wrapping_mul(2_654_435_761);
            }
            if !mono || get(p, GLIDE, 1.0) < 2.0 {
                v.pitch = note as f32;
            }
            v.time = 0.0;
            for e in v.env.iter_mut() {
                e.start();
            }
        }
        v.note = note;
        v.vel = vel;
        v.gate = true;
        v.age = self.clock;
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.held.retain(|k| k.0 != note);
        let mono = Self::voices(p) == 1;
        if mono {
            let v = &mut self.voices[0];
            if v.note == note {
                if let Some(&(n, vel)) = self.held.last() {
                    // Back to the key still held, gliding.
                    v.note = n;
                    v.target = n as f32;
                    v.vel = vel;
                    return;
                }
                v.gate = false;
                for (e, env) in v.env.iter_mut().enumerate() {
                    env.release(p, e);
                }
            }
            return;
        }
        for v in self.voices.iter_mut() {
            if v.gate && v.note == note {
                v.gate = false;
                for (e, env) in v.env.iter_mut().enumerate() {
                    env.release(p, e);
                }
            }
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in self.voices.iter_mut() {
            *v = Voice::default();
        }
    }

    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        let Some(v) = self.voices.iter().filter(|v| v.active()).max_by_key(|v| v.age) else { return };
        out[0] = v.note as f32;
        for e in 0..ENVS {
            out[1 + e] = v.env[e].level / 99.0;
            out[6 + e] = if v.env[e].on { v.env[e].step as f32 + 1.0 } else { 0.0 };
        }
        out[11] = 1.0;
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let dt = 1.0 / sr;
        let line = choice(p, LINE, LINES.len());
        let octave = get(p, OCTAVE, 0.0).round() * 12.0;
        let det = get(p, DETUNE_NOTE, 0.0).round() + get(p, DETUNE, 6.0) / 100.0;
        let ring = get(p, RING, 0.0) >= 0.5;
        let noise_on = get(p, NOISE, 0.0) >= 0.5;
        let waves = [[choice(p, WAVE1_A, WAVES.len()), choice(p, WAVE1_B, WAVES_B.len())], [choice(p, WAVE2_A, WAVES.len()), choice(p, WAVE2_B, WAVES_B.len())]];
        let warp = [get(p, WARP1, 0.75), get(p, WARP2, 0.6)];
        let warp_keys = [get(p, WARP1_KEYS, 2.0) / 9.0, get(p, WARP2_KEYS, 2.0) / 9.0];
        let amp_keys = [get(p, AMP1_KEYS, 2.0) / 9.0, get(p, AMP2_KEYS, 2.0) / 9.0];
        let level = [get(p, LEVEL1, 0.8), get(p, LEVEL2, 0.6)];
        let vib_wave = choice(p, VIB_WAVE, VIB_WAVES.len());
        let (vib_delay, vib_rate, vib_depth) = (get(p, VIB_DELAY, 0.4), get(p, VIB_RATE, 5.5), get(p, VIB_DEPTH, 0.0) / 100.0);
        let pitch_env = get(p, PITCH_ENV, 0.0);
        let glide = get(p, GLIDE, 1.0).max(1.0) / 1000.0;
        let glide_k = 1.0 - (-dt / glide).exp();
        let vel_amt = get(p, VELOCITY, 0.5).clamp(0.0, 1.0);
        let chorus = get(p, CHORUS, 0.0).clamp(0.0, 1.0);
        let chorus_rate = get(p, CHORUS_RATE, 0.6);
        let volume = get(p, VOLUME, 0.7).clamp(0.0, 1.0);
        let bend = self.pitch_bend;
        let dc_k = 1.0 - (-TAU * 10.0 / sr).exp();
        // Which line plays on each of the two oscillators, with whose settings.
        let (use_a, use_b, set_b) = match line {
            0 => (true, false, 0),
            1 => (false, true, 1),
            2 => (true, true, 0),
            _ => (true, true, 1),
        };
        let clen = self.chorus[0].len();
        for s in 0..n {
            let mut out = 0.0;
            for v in self.voices.iter_mut() {
                if !v.active() {
                    continue;
                }
                // The amp of a line that does not play does not keep a voice alive.
                if !use_a || set_b == 0 {
                    v.env[ENV_AMP2].on &= line == 1;
                }
                if line == 1 {
                    v.env[ENV_AMP1].on = false;
                }
                if !v.active() {
                    continue;
                }
                v.pitch += (v.target - v.pitch) * glide_k;
                v.time += dt;
                let note = v.note as f32;
                let up = ((note - 48.0) / 24.0).max(0.0);
                // The envelopes, faster for higher keys as the amp keys say.
                let lv = [
                    v.env[ENV_WARP1].tick(p, ENV_WARP1, dt, 1.0),
                    v.env[ENV_AMP1].tick(p, ENV_AMP1, dt, 1.0 + 2.0 * amp_keys[0] * up),
                    v.env[ENV_WARP2].tick(p, ENV_WARP2, dt, 1.0),
                    v.env[ENV_AMP2].tick(p, ENV_AMP2, dt, 1.0 + 2.0 * amp_keys[1] * up),
                    v.env[ENV_PITCH].tick(p, ENV_PITCH, dt, 1.0),
                ];
                let vib = if vib_depth > 0.0 && v.time > vib_delay {
                    let x = ((v.time - vib_delay) * vib_rate).fract();
                    let shape = match vib_wave {
                        0 => 1.0 - 4.0 * (x - 0.5).abs(),
                        1 => 2.0 * x - 1.0,
                        2 => 1.0 - 2.0 * x,
                        _ => {
                            if x < 0.5 {
                                1.0
                            } else {
                                -1.0
                            }
                        }
                    };
                    vib_depth * shape * ((v.time - vib_delay) / 0.3).min(1.0)
                } else {
                    0.0
                };
                let base = v.pitch + octave + bend + vib + pitch_env * lv[4] / 99.0;
                let f = [440.0 * 2f32.powf((base - 69.0) / 12.0), 440.0 * 2f32.powf((base + if line >= 2 { det } else { 0.0 } - 69.0) / 12.0)];
                let vel = 1.0 - vel_amt + vel_amt * v.vel;
                let mut y = [0.0f32; 2];
                for o in 0..2 {
                    if (o == 0 && !use_a) || (o == 1 && !use_b) {
                        continue;
                    }
                    // Its settings: line 1's, or line 2's.
                    let st = if o == 0 { if line == 1 { 1 } else { 0 } } else { set_b };
                    let (env_w, env_a) = if st == 0 { (lv[ENV_WARP1], lv[ENV_AMP1]) } else { (lv[ENV_WARP2], lv[ENV_AMP2]) };
                    // On its own, line 2 plays at the key's pitch.
                    let fo = if o == 1 && line == 1 { f[0] } else { f[o] };
                    let dph = (fo / sr).min(0.45);
                    let [wa, wb] = waves[st];
                    let w = if v.second[o] && wb > 0 { wb - 1 } else { wa };
                    let keys = 1.0 - warp_keys[st] * ((note - 36.0) / 60.0).clamp(0.0, 1.0);
                    let d = warp[st] * env_w / 99.0 * keys * vel;
                    let x = wave(w, v.ph[o], d, dph);
                    let amp = env_a / 99.0 * level[st] * vel * (1.0 - 0.5 * amp_keys[st] * ((note - 60.0) / 60.0).clamp(0.0, 1.0));
                    y[o] = x * amp;
                    v.ph[o] += dph;
                    if v.ph[o] >= 1.0 {
                        v.ph[o] -= 1.0;
                        v.second[o] = wb > 0 && !v.second[o];
                    }
                }
                if use_a && use_b {
                    if ring {
                        let a = wave(waves[0][0], v.ph[0], 0.0, 0.0);
                        y[1] *= a;
                    }
                    if noise_on {
                        // Noise, held for a while at a pace with the pitch.
                        v.noise_ph += f[1] * 4.0 / sr;
                        if v.noise_ph >= 1.0 {
                            v.noise_ph -= v.noise_ph.floor();
                            v.rng ^= v.rng << 13;
                            v.rng ^= v.rng >> 17;
                            v.rng ^= v.rng << 5;
                            v.noise = (v.rng >> 8) as f32 / (1u32 << 23) as f32 - 1.0;
                        }
                        y[1] *= v.noise;
                    }
                }
                let x = y[0] + y[1];
                // The resonant waves stand off zero: the middle taken out.
                v.dc += dc_k * (x - v.dc);
                out += x - v.dc;
            }
            let x = out * 0.22 * volume;
            if chorus > 0.0 {
                self.chorus_ph = (self.chorus_ph + chorus_rate / sr).fract();
                self.chorus[0][self.chorus_pos] = x;
                self.chorus[1][self.chorus_pos] = x;
                let read = |buf: &[f32], ph: f32, pos: usize| {
                    let d = (0.008 + 0.004 * (TAU * ph).sin()) * sr;
                    let at = pos as f32 + clen as f32 - d;
                    let i0 = at.floor() as usize % clen;
                    let fr = at - at.floor();
                    buf[i0] * (1.0 - fr) + buf[(i0 + 1) % clen] * fr
                };
                let wl = read(&self.chorus[0], self.chorus_ph, self.chorus_pos);
                let wr = read(&self.chorus[1], (self.chorus_ph + 0.5).fract(), self.chorus_pos);
                self.chorus_pos = (self.chorus_pos + 1) % clen;
                l[s] = x * (1.0 - 0.4 * chorus) + wl * 0.7 * chorus;
                r[s] = x * (1.0 - 0.4 * chorus) + wr * 0.7 * chorus;
            } else {
                l[s] = x;
                r[s] = x;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn run(st: &mut WarpState, p: &[f32], secs: f32) -> Vec<f32> {
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

    fn brightness(v: &[f32]) -> f32 {
        let d: f32 = v.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum();
        d / v.iter().map(|x| x * x).sum::<f32>().max(1e-9)
    }

    #[test]
    fn unbent_every_wave_is_a_cosine_and_none_jumps() {
        for w in 0..WAVES.len() {
            for k in 0..100 {
                let ph = k as f32 / 100.0;
                assert!((wave(w, ph, 0.0, 0.001) - (TAU * ph).cos()).abs() < 1e-4, "{}", WAVES[w]);
            }
            // Bent all the way, still no step anywhere (nor across the cycle).
            let dt = 0.002;
            let pts: Vec<f32> = (0..=2000).map(|k| wave(w, (k as f32 / 2000.0).fract(), 1.0, dt)).collect();
            let worst = pts.windows(2).map(|x| (x[1] - x[0]).abs()).fold(0.0, f32::max);
            assert!(worst < 0.5, "{}: {worst}", WAVES[w]);
        }
    }

    #[test]
    fn bending_brightens() {
        let mut p = preset("Init");
        set_env(&mut p, ENV_WARP1, &[(99.0, 99.0)], 1, 2);
        let mut level = |d: f32| {
            p[WARP1] = d;
            let mut st = WarpState::new(SR);
            st.note_on(&p, 48, 1.0);
            brightness(&run(&mut st, &p, 0.2)[2400..])
        };
        let (soft, hard) = (level(0.0), level(1.0));
        assert!(hard > 4.0 * soft, "{soft} {hard}");
    }

    #[test]
    fn the_envelope_walks_its_steps_holds_and_ends() {
        let mut p = preset("Init");
        set_env(&mut p, ENV_AMP1, &[(99.0, 99.0), (80.0, 40.0), (60.0, 70.0), (90.0, 0.0)], 3, 4);
        let mut e = StepEnv::default();
        e.start();
        for _ in 0..48000 {
            e.tick(&p, ENV_AMP1, 1.0 / SR, 1.0);
        }
        // Held on step 3, at its level.
        assert_eq!(e.step, 2);
        assert!((e.level - 70.0).abs() < 1e-3);
        e.release(&p, ENV_AMP1);
        for _ in 0..48000 {
            e.tick(&p, ENV_AMP1, 1.0 / SR, 1.0);
        }
        assert!(!e.on && e.level == 0.0);
    }

    #[test]
    fn every_preset_sounds_and_lets_go() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = WarpState::new(SR);
            for n in [48, 55, 60, 64] {
                st.note_on(&p, n, 0.9);
            }
            let out = run(&mut st, &p, 0.5);
            assert!(out.iter().all(|x| x.is_finite() && x.abs() < 1.5), "{name}");
            assert!(rms(&out) > 0.01, "{name}: {}", rms(&out));
            for n in [48, 55, 60, 64] {
                st.note_off(&p, n);
            }
            run(&mut st, &p, 6.0);
            assert!(st.voices.iter().all(|v| !v.active()), "{name} rings on");
        }
    }

    #[test]
    fn the_second_line_rings_and_detunes() {
        let mut p = preset("Ring Bell");
        let mut st = WarpState::new(SR);
        st.note_on(&p, 60, 1.0);
        let rung = run(&mut st, &p, 0.2);
        p[RING] = 0.0;
        let mut st = WarpState::new(SR);
        st.note_on(&p, 60, 1.0);
        let plain = run(&mut st, &p, 0.2);
        let diff: f32 = rung.iter().zip(&plain).map(|(a, b)| (a - b).abs()).sum();
        assert!(diff > 1.0, "{diff}");
    }

    #[test]
    fn its_envelopes_have_no_knobs() {
        assert_eq!(SPECS.len(), COUNT);
        assert!(SPECS[..ENV_BASE].iter().all(|s| s.shown()));
        assert!(SPECS[ENV_BASE..].iter().all(|s| !s.shown()));
        assert_eq!(SPECS[env_param(ENV_AMP1, SUSTAIN)].default, 2.0);
    }
}
