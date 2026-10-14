//! Glitch Sequencer: a sequencer of glitch effects. A lane of steps (1/8 to
//! 1/64 notes) is painted with blocks of any length; each block is one of
//! nine modules that takes hold of the audio while it lasts:
//!
//! 1. Tape Stop: the tape slows to a halt (after a wait) and spins up again.
//! 2. Modulator: ring modulation by a sine or square, with FM and stereo spread.
//! 3. Retrigger: the start of the block repeated, each repeat pitched, faster
//!    or slower, gated and fading.
//! 4. Shuffler: the block cut into pieces, each from an earlier spot (some backwards).
//! 5. Reverser: the block before backwards, ping-pong, or mirrored.
//! 6. Crusher: fewer bits, a lower rate, smoothed.
//! 7. Gater: chopped in a rhythm, with stereo offset.
//! 8. Stretcher: half, quarter or double speed, on tape (the pitch follows)
//!    or in grains (the pitch stays).
//! 9. Delay: the block sent into a ping-pong echo that rings on.
//!
//! Four slots hold any other effect, sequenced the same way: they hear the
//! music only during their blocks, so a reverb's tail rings on after.
//!
//! Every block has a chance; Amount scales them all, Seed makes the dice
//! repeat each time round, Variation shakes a module's knobs per block. What
//! the modules make goes through a Step Envelope, an Overdrive and a Master
//! Filter. Eight patterns. Edges are faded (De-Click). Original code.

use super::super::effects::{EffectState, FxCtx};
use crate::dsp::{db_to_gain, Svf};
use crate::params::{ParamSpec, Unit};
use crate::project::Device;
use crate::scope::{DeviceTaps, MeterTap, ScopeTap};

// The sequencer and the master section.
pub const STEP: usize = 0;
pub const LENGTH: usize = 1;
pub const PATTERN: usize = 2;
pub const AMOUNT: usize = 3;
pub const SEED: usize = 4;
pub const DECLICK: usize = 5;
pub const ENV_LENGTH: usize = 6;
pub const ENV_VOLUME: usize = 7;
pub const OD_MIX: usize = 8;
pub const OD_GAIN: usize = 9;
pub const FILTER_MODE: usize = 10;
pub const CUTOFF: usize = 11;
pub const RESONANCE: usize = 12;
pub const VARIATION: usize = 13;
pub const MIX: usize = 14;
pub const OUTPUT: usize = 15;
pub const SOLO: usize = 16;
// 1 Tape Stop
pub const TS_ON: usize = 17;
pub const TS_SLOW: usize = 18;
pub const TS_DELAY: usize = 19;
pub const TS_UP: usize = 20;
pub const TS_CURVE: usize = 21;
// 2 Modulator
pub const MOD_ON: usize = 22;
pub const MOD_FREQ: usize = 23;
pub const MOD_SPREAD: usize = 24;
pub const MOD_FM: usize = 25;
pub const MOD_FM_SPEED: usize = 26;
pub const MOD_SHAPE: usize = 27;
pub const MOD_MIX: usize = 28;
// 3 Retrigger
pub const RT_ON: usize = 29;
pub const RT_RATE: usize = 30;
pub const RT_LENGTH: usize = 31;
pub const RT_SPEED: usize = 32;
pub const RT_PITCH: usize = 33;
pub const RT_DECAY: usize = 34;
// 4 Shuffler
pub const SH_ON: usize = 35;
pub const SH_MIN: usize = 36;
pub const SH_MAX: usize = 37;
pub const SH_RANGE: usize = 38;
pub const SH_AMOUNT: usize = 39;
pub const SH_REVERSE: usize = 40;
// 5 Reverser
pub const RV_ON: usize = 41;
pub const RV_MODE: usize = 42;
pub const RV_CHUNK: usize = 43;
pub const RV_SPREAD: usize = 44;
// 6 Crusher
pub const CR_ON: usize = 45;
pub const CR_BITS: usize = 46;
pub const CR_DOWN: usize = 47;
pub const CR_SMOOTH: usize = 48;
// 7 Gater
pub const GT_ON: usize = 49;
pub const GT_RATE: usize = 50;
pub const GT_DUTY: usize = 51;
pub const GT_SMOOTH: usize = 52;
pub const GT_STEREO: usize = 53;
// 8 Stretcher
pub const ST_ON: usize = 54;
pub const ST_SPEED: usize = 55;
pub const ST_MODE: usize = 56;
pub const ST_GRAIN: usize = 57;
// 9 Delay
pub const DL_ON: usize = 58;
pub const DL_TIME: usize = 59;
pub const DL_FEEDBACK: usize = 60;
pub const DL_SPREAD: usize = 61;
pub const DL_TONE: usize = 62;

pub const STEP_SIZES: &[&str] = &["1/8", "1/16", "1/32", "1/64"];
const STEP_BEATS: [f64; 4] = [0.5, 0.25, 0.125, 0.0625];
pub const PATTERN_NAMES: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8"];
pub const FILTER_MODES: &[&str] = &["Off", "Low Pass", "High Pass", "Band Pass"];
pub const SOLOS: &[&str] = &["Off", "Tape Stop", "Modulator", "Retrigger", "Shuffler", "Reverser", "Crusher", "Gater", "Stretcher", "Delay"];
pub const REPEATS: &[&str] = &["1/4", "1/8", "1/16", "1/32", "1/64", "1/128"];
const REPEAT_BEATS: [f64; 6] = [1.0, 0.5, 0.25, 0.125, 0.0625, 0.03125];
pub const RANGES: &[&str] = &["1 Beat", "2 Beats", "1 Bar", "2 Bars", "4 Bars"];
const RANGE_BEATS: [f64; 5] = [1.0, 2.0, 4.0, 8.0, 16.0];
pub const REVERSE_MODES: &[&str] = &["Backwards", "Ping Pong", "Mirror"];
pub const CHUNKS: &[&str] = &["1/64", "1/32", "1/16", "1/8", "1/4"];
const CHUNK_BEATS: [f64; 5] = [0.0625, 0.125, 0.25, 0.5, 1.0];
pub const GATE_RATES: &[&str] = &["1/8", "1/16", "1/32", "1/64", "1/128"];
const GATE_BEATS: [f64; 5] = [0.5, 0.25, 0.125, 0.0625, 0.03125];
pub const SPEEDS: &[&str] = &["1/4", "1/3", "1/2", "2/3", "1.5x", "2x", "3x", "4x"];
const SPEED_VALUES: [f64; 8] = [0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 1.5, 2.0, 3.0, 4.0];
pub const STRETCH_MODES: &[&str] = &["Tape", "Grains"];
pub const TIMES: &[&str] = &["1/32", "1/16", "1/8 T", "1/8", "3/16", "1/4", "3/8", "1/2"];
const TIME_BEATS: [f64; 8] = [0.125, 0.25, 1.0 / 3.0, 0.5, 0.75, 1.0, 1.5, 2.0];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Step", "Sequencer", STEP_SIZES, 2),
    ParamSpec::int("Length", "Sequencer", 1.0, 64.0, 32.0, Unit::None),
    ParamSpec::choice("Pattern", "Sequencer", PATTERN_NAMES, 0),
    ParamSpec::lin("Amount", "Sequencer", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::int("Seed", "Sequencer", 0.0, 99.0, 0.0, Unit::None),
    ParamSpec::lin("De-Click", "Sequencer", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::log("Env Length", "Step Envelope", 0.25, 16.0, 2.0, Unit::None),
    ParamSpec::lin("Env Volume", "Step Envelope", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Drive Mix", "Overdrive", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Drive Gain", "Overdrive", 0.0, 36.0, 12.0, Unit::Db),
    ParamSpec::choice("Filter", "Master Filter", FILTER_MODES, 0),
    ParamSpec::log("Cutoff", "Master Filter", 40.0, 18000.0, 2500.0, Unit::Hz),
    ParamSpec::lin("Resonance", "Master Filter", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Variation", "Output", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Output", "Output", -24.0, 12.0, 0.0, Unit::Db),
    ParamSpec::choice("Solo", "Output", SOLOS, 0),
    ParamSpec::toggle("Tape Stop", "Tape Stop", true),
    ParamSpec::lin("Slow Down", "Tape Stop", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Wait", "Tape Stop", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Speed Up", "Tape Stop", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::lin("Curve", "Tape Stop", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::toggle("Modulator", "Modulator", true),
    ParamSpec::log("Frequency", "Modulator", 1.0, 5000.0, 125.0, Unit::Hz),
    ParamSpec::lin("Spread", "Modulator", 0.0, 180.0, 0.0, Unit::None),
    ParamSpec::lin("FM Amount", "Modulator", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::log("FM Speed", "Modulator", 0.1, 50.0, 4.0, Unit::Hz),
    ParamSpec::lin("Square", "Modulator", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Ring Mix", "Modulator", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::toggle("Retrigger", "Retrigger", true),
    ParamSpec::choice("Rate", "Retrigger", REPEATS, 3),
    ParamSpec::lin("Gate", "Retrigger", 0.1, 1.0, 0.85, Unit::Percent),
    ParamSpec::lin("Speed Change", "Retrigger", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Pitch", "Retrigger", -12.0, 12.0, 0.0, Unit::Semi),
    ParamSpec::lin("Decay", "Retrigger", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::toggle("Shuffler", "Shuffler", true),
    ParamSpec::int("Minimum", "Shuffler", 1.0, 8.0, 1.0, Unit::None),
    ParamSpec::int("Maximum", "Shuffler", 1.0, 8.0, 2.0, Unit::None),
    ParamSpec::choice("Range", "Shuffler", RANGES, 2),
    ParamSpec::lin("Shuffle", "Shuffler", 0.0, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Backwards", "Shuffler", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::toggle("Reverser", "Reverser", true),
    ParamSpec::choice("Mode", "Reverser", REVERSE_MODES, 0),
    ParamSpec::choice("Chunk", "Reverser", CHUNKS, 2),
    ParamSpec::lin("Stereo", "Reverser", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Crusher", "Crusher", true),
    ParamSpec::int("Bits", "Crusher", 1.0, 16.0, 6.0, Unit::None),
    ParamSpec::int("Downsample", "Crusher", 1.0, 40.0, 6.0, Unit::None),
    ParamSpec::lin("Smooth", "Crusher", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Gater", "Gater", true),
    ParamSpec::choice("Rate", "Gater", GATE_RATES, 2),
    ParamSpec::lin("Duty", "Gater", 0.05, 0.95, 0.5, Unit::Percent),
    ParamSpec::lin("Smooth", "Gater", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Stereo", "Gater", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Stretcher", "Stretcher", true),
    ParamSpec::choice("Speed", "Stretcher", SPEEDS, 2),
    ParamSpec::choice("Mode", "Stretcher", STRETCH_MODES, 0),
    ParamSpec::log("Grain", "Stretcher", 10.0, 200.0, 60.0, Unit::Ms),
    ParamSpec::toggle("Delay", "Delay", true),
    ParamSpec::choice("Time", "Delay", TIMES, 4),
    ParamSpec::lin("Feedback", "Delay", 0.0, 0.95, 0.55, Unit::Percent),
    ParamSpec::lin("Ping Pong", "Delay", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::log("Tone", "Delay", 500.0, 18000.0, 6000.0, Unit::Hz),
];

/// The modules a block can be (0 is nothing: the music as it is).
pub const MODULES: &[&str] = &["Tape Stop", "Modulator", "Retrigger", "Shuffler", "Reverser", "Crusher", "Gater", "Stretcher", "Delay", "Slot 1", "Slot 2", "Slot 3", "Slot 4"];
pub const BUILT_IN: usize = 9;
pub const SLOTS: usize = 4;
pub const STEPS: usize = 64;
pub const PATTERNS: usize = 8;
/// The first slot's module number.
pub const FIRST_SLOT: u8 = BUILT_IN as u8 + 1;

/// Each module's parameters, its on switch first.
pub const MODULE_PARAMS: [&[usize]; BUILT_IN] = [
    &[TS_ON, TS_SLOW, TS_DELAY, TS_UP, TS_CURVE],
    &[MOD_ON, MOD_FREQ, MOD_SPREAD, MOD_FM, MOD_FM_SPEED, MOD_SHAPE, MOD_MIX],
    &[RT_ON, RT_RATE, RT_LENGTH, RT_SPEED, RT_PITCH, RT_DECAY],
    &[SH_ON, SH_MIN, SH_MAX, SH_RANGE, SH_AMOUNT, SH_REVERSE],
    &[RV_ON, RV_MODE, RV_CHUNK, RV_SPREAD],
    &[CR_ON, CR_BITS, CR_DOWN, CR_SMOOTH],
    &[GT_ON, GT_RATE, GT_DUTY, GT_SMOOTH, GT_STEREO],
    &[ST_ON, ST_SPEED, ST_MODE, ST_GRAIN],
    &[DL_ON, DL_TIME, DL_FEEDBACK, DL_SPREAD, DL_TONE],
];

// ------------------------------------------------------------------ the pattern data
//
// `b"G2"`, then per pattern and step two bytes: the module (0 = none,
// 1..=13) with START set where a block begins, and the block's chance
// (0..=100, kept on its first step).

const MAGIC: [u8; 2] = *b"G2";
const START: u8 = 0x80;
pub const DATA_BYTES: usize = 2 + PATTERNS * STEPS * 2;

fn at(pattern: usize, i: usize) -> usize {
    2 + (pattern.min(PATTERNS - 1) * STEPS + i % STEPS) * 2
}

pub fn is_current(data: &[u8]) -> bool {
    data.len() >= DATA_BYTES && data[..2] == MAGIC
}

/// One step: its module, whether a block starts there, and the chance.
pub fn cell(data: &[u8], pattern: usize, i: usize) -> (u8, bool, u8) {
    if !is_current(data) {
        return (0, false, 100);
    }
    let o = at(pattern, i);
    let m = (data[o] & !START).min(MODULES.len() as u8);
    (m, data[o] & START != 0 || m == 0, data[o + 1].min(100))
}

fn set_cell(data: &mut [u8], pattern: usize, i: usize, module: u8, start: bool, chance: u8) {
    let o = at(pattern, i);
    data[o] = module.min(MODULES.len() as u8) | if start && module != 0 { START } else { 0 };
    data[o + 1] = chance.min(100);
}

/// A block of the lane: where it starts, how many steps, which module, its chance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    pub start: usize,
    pub len: usize,
    pub module: u8,
    pub chance: u8,
}

/// The blocks of a pattern, within its first `length` steps.
pub fn blocks(data: &[u8], pattern: usize, length: usize) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    let length = length.min(STEPS);
    let mut i = 0;
    while i < length {
        let (m, _, chance) = cell(data, pattern, i);
        if m == 0 {
            i += 1;
            continue;
        }
        let mut len = 1;
        while i + len < length {
            let (n, start, _) = cell(data, pattern, i + len);
            if n != m || start {
                break;
            }
            len += 1;
        }
        out.push(Block { start: i, len, module: m, chance });
        i += len;
    }
    out
}

/// The block covering step `i`, if any (without allocating: the engine asks).
pub fn block_at(data: &[u8], pattern: usize, length: usize, i: usize) -> Option<Block> {
    let length = length.min(STEPS);
    if i >= length {
        return None;
    }
    let (m, _, _) = cell(data, pattern, i);
    if m == 0 {
        return None;
    }
    let mut s = i;
    while s > 0 && !cell(data, pattern, s).1 && cell(data, pattern, s - 1).0 == m {
        s -= 1;
    }
    let mut len = i - s + 1;
    while s + len < length {
        let (n, start, _) = cell(data, pattern, s + len);
        if n != m || start {
            break;
        }
        len += 1;
    }
    Some(Block { start: s, len, module: m, chance: cell(data, pattern, s).2 })
}

/// Ensures the data is in the current layout.
fn ensure(data: &mut Vec<u8>) {
    if !is_current(data) {
        *data = empty_data();
    }
}

fn empty_data() -> Vec<u8> {
    let mut d = vec![0u8; DATA_BYTES];
    d[..2].copy_from_slice(&MAGIC);
    for p in 0..PATTERNS {
        for i in 0..STEPS {
            d[at(p, i) + 1] = 100;
        }
    }
    d
}

/// Paints steps `a..=b` as one block of `module` (0 erases). A block cut in
/// two keeps both halves.
pub fn paint(data: &mut Vec<u8>, pattern: usize, a: usize, b: usize, module: u8) {
    ensure(data);
    let (a, b) = (a.min(b).min(STEPS - 1), a.max(b).min(STEPS - 1));
    for i in a..=b {
        set_cell(data, pattern, i, module, i == a, 100);
    }
    // What follows was in the middle of a block: it starts its own.
    if b + 1 < STEPS {
        let (m, start, c) = cell(data, pattern, b + 1);
        if m != 0 && !start {
            set_cell(data, pattern, b + 1, m, true, c);
        }
    }
}

/// The chance of the block starting at step `start`.
pub fn set_chance(data: &mut Vec<u8>, pattern: usize, start: usize, chance: u8) {
    ensure(data);
    let o = at(pattern, start);
    data[o + 1] = chance.min(100);
}

pub fn clear(data: &mut Vec<u8>, pattern: usize) {
    ensure(data);
    for i in 0..STEPS {
        set_cell(data, pattern, i, 0, true, 100);
    }
}

pub fn copy_pattern(data: &mut Vec<u8>, from: usize, to: usize) {
    ensure(data);
    for i in 0..STEPS {
        let (m, start, c) = cell(data, from, i);
        set_cell(data, to, i, m, start, c);
    }
}

pub fn default_data() -> Vec<u8> {
    let mut d = empty_data();
    // (start, steps, module) on a lane of 32 thirty-seconds.
    let patterns: [&[(usize, usize, u8)]; 3] = [
        &[(0, 6, 3), (8, 4, 5), (12, 2, 6), (16, 6, 1), (24, 2, 3), (26, 2, 4), (28, 4, 7)],
        &[(0, 4, 6), (4, 4, 2), (12, 4, 3), (20, 2, 5), (22, 2, 5), (24, 8, 8)],
        &[(4, 2, 3), (10, 6, 4), (18, 2, 7), (20, 4, 9), (28, 4, 1)],
    ];
    for (p, list) in patterns.iter().enumerate() {
        for &(s, n, m) in list.iter() {
            paint(&mut d, p, s, s + n - 1, m);
        }
    }
    d
}

/// Old data (the first Glitch Sequencer: 4 patterns of 32 single steps, other
/// module numbers) in the current layout. True when it was converted.
pub fn migrate(data: &mut Vec<u8>) -> bool {
    if is_current(data) || data.is_empty() {
        if data.is_empty() {
            *data = default_data();
        }
        return false;
    }
    let old = std::mem::replace(data, empty_data());
    // Retrigger, Reverse, Shuffle, Tape Stop, Stretch, Crush, Gate, Filter, Delay, slots.
    const MAP: [u8; 14] = [0, 3, 5, 4, 1, 8, 6, 7, 0, 9, 10, 11, 12, 13];
    for p in 0..4 {
        for i in 0..32 {
            let o = (p * 32 + i) * 2;
            if let Some(s) = old.get(o..o + 2) {
                let m = MAP.get(s[0] as usize).copied().unwrap_or(0);
                if m != 0 {
                    // Every old step was its own block.
                    set_cell(data, p, i, m, true, s[1].min(100));
                }
            }
        }
    }
    true
}

/// Slot `s` was taken out: its blocks play nothing, the slots after it move up.
pub fn remove_slot(data: &mut Vec<u8>, s: usize) {
    ensure(data);
    let gone = FIRST_SLOT + s as u8;
    for step in data[2..].chunks_exact_mut(2) {
        let m = step[0] & !START;
        if m == gone {
            step[0] = 0;
        } else if m > gone {
            step[0] = (m - 1) | (step[0] & START);
        }
    }
}

/// Rhythms for Randomise: the block lengths each one picks from (in steps).
pub const TEMPLATES: [&[usize]; 4] = [&[2], &[4], &[1, 2, 3, 4, 6], &[4, 8, 12, 16]];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Randomise {
    /// New block lengths (from the template); the modules stay where they were.
    Steps,
    /// New modules in the blocks there are.
    Fx,
    Both,
}

struct Dice(u32);

impl Dice {
    fn unit(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / 16_777_216.0
    }

    fn below(&mut self, n: usize) -> usize {
        ((self.unit() * n as f32) as usize).min(n.saturating_sub(1))
    }
}

/// Throws the dice over a pattern. `modules` are those to pick from.
pub fn randomise(data: &mut Vec<u8>, pattern: usize, length: usize, what: Randomise, template: usize, modules: &[u8], seed: u32) {
    ensure(data);
    let mut d = Dice(seed | 1);
    let length = length.clamp(1, STEPS);
    let pick = |d: &mut Dice| if modules.is_empty() || d.unit() < 0.25 { 0 } else { modules[d.below(modules.len())] };
    match what {
        Randomise::Fx => {
            for b in blocks(data, pattern, length) {
                let m = pick(&mut d);
                for i in b.start..b.start + b.len {
                    set_cell(data, pattern, i, m, i == b.start, b.chance);
                }
            }
        }
        Randomise::Steps | Randomise::Both => {
            let old: Vec<u8> = (0..STEPS).map(|i| cell(data, pattern, i).0).collect();
            let any = old[..length].iter().any(|m| *m != 0);
            let lens = TEMPLATES[template.min(TEMPLATES.len() - 1)];
            let mut i = 0;
            while i < STEPS {
                let n = lens[d.below(lens.len())].max(1);
                let m = if i >= length {
                    0
                } else if what == Randomise::Steps && any {
                    old[i]
                } else {
                    pick(&mut d)
                };
                for j in i..(i + n).min(STEPS) {
                    set_cell(data, pattern, j, m, j == i, 100);
                }
                i += n;
            }
        }
    }
}

// ------------------------------------------------------------------ the sound

/// The buffer: about 22 s at 48 kHz.
const BUF: usize = 1 << 20;
const DELAY_BUF: usize = 1 << 18;
const CHUNK: usize = 256;

pub struct Glitch {
    sr: f32,
    buf: [Vec<f32>; 2],
    w: usize,
    step_k: i64,
    /// The block playing: (pass round the pattern, first step).
    block_key: (i64, usize),
    module: u8,
    seg_start: usize,
    seg_len: usize,
    pos: usize,
    step_len: usize,
    /// Knobs Variation changed for this block (else the live ones count).
    varied: Vec<Option<f32>>,
    rng: u32,
    // Tape Stop
    tape_p: f64,
    // Modulator
    phase: f64,
    fm_phase: f64,
    // Retrigger
    rt_rep: u32,
    rt_start: usize,
    rt_len: f64,
    rt_base: usize,
    // Shuffler
    sh_start: usize,
    sh_len: usize,
    sh_src: f64,
    sh_on: bool,
    sh_rev: bool,
    // Crusher
    hold: [f32; 2],
    hold_n: u32,
    smooth: [f32; 2],
    // Gater
    gate_env: [f32; 2],
    // Delay
    delay: [Vec<f32>; 2],
    dw: usize,
    tone: [f32; 2],
    // Master
    svf: [Svf; 2],
    filter_set: (f32, f32, u8),
    free_beat: f64,
    /// The slots' effects (kept by the engine, in step with the device's children).
    pub children: Vec<(u64, EffectState)>,
    pub taps: DeviceTaps,
    sl: Vec<f32>,
    sr_buf: Vec<f32>,
    weights: Vec<[f32; SLOTS]>,
}

impl Glitch {
    pub fn new(sr: f32) -> Self {
        Self {
            sr: sr.max(1000.0),
            buf: [vec![0.0; BUF], vec![0.0; BUF]],
            w: BUF,
            step_k: i64::MIN,
            block_key: (i64::MIN, 0),
            module: 0,
            seg_start: BUF,
            seg_len: 1,
            pos: 0,
            step_len: 1,
            varied: vec![None; SPECS.len()],
            rng: 0x6C17_C4ED,
            tape_p: 0.0,
            phase: 0.0,
            fm_phase: 0.0,
            rt_rep: 0,
            rt_start: 0,
            rt_len: 1.0,
            rt_base: 1,
            sh_start: 0,
            sh_len: 0,
            sh_src: 0.0,
            sh_on: false,
            sh_rev: false,
            hold: [0.0; 2],
            hold_n: 0,
            smooth: [0.0; 2],
            gate_env: [1.0; 2],
            delay: [vec![0.0; DELAY_BUF], vec![0.0; DELAY_BUF]],
            dw: 0,
            tone: [0.0; 2],
            svf: [Svf::new(), Svf::new()],
            filter_set: (0.0, -1.0, 0),
            free_beat: 0.0,
            children: Vec::new(),
            taps: DeviceTaps { pre: None, post: ScopeTap::default(), meters: Some(MeterTap::default()) },
            sl: vec![0.0; CHUNK],
            sr_buf: vec![0.0; CHUNK],
            weights: vec![[0.0; SLOTS]; CHUNK],
        }
    }

    fn unit(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / 16_777_216.0
    }

    /// A knob's value for this block.
    #[inline]
    fn v(&self, p: &[f32], k: usize) -> f32 {
        self.varied[k].unwrap_or(p[k])
    }

    #[inline]
    fn choice(&self, p: &[f32], k: usize, n: usize) -> usize {
        (self.v(p, k).round().max(0.0) as usize).min(n - 1)
    }

    #[inline]
    fn read(&self, ch: usize, pos: f64) -> f32 {
        let i = pos.floor();
        let f = (pos - i) as f32;
        let i = i as i64;
        let a = self.buf[ch][i.rem_euclid(BUF as i64) as usize];
        let b = self.buf[ch][(i + 1).rem_euclid(BUF as i64) as usize];
        a + (b - a) * f
    }

    /// A new step: perhaps a new block, which module plays it, and how.
    fn begin_step(&mut self, k: i64, p: &[f32], data: &[u8], spb: f64) {
        self.step_k = k;
        let length = p[LENGTH].round().clamp(1.0, STEPS as f32) as i64;
        let pattern = (p[PATTERN].round().max(0.0) as usize).min(PATTERNS - 1);
        let i = k.rem_euclid(length) as usize;
        let pass = k.div_euclid(length);
        let step_beats = STEP_BEATS[(p[STEP].round().max(0.0) as usize).min(STEP_BEATS.len() - 1)];
        self.step_len = ((step_beats * spb) as usize).max(16);
        // With a seed, the dice fall the same way every time round.
        let seed = p[SEED].round().clamp(0.0, 99.0) as u32;
        if seed > 0 && i == 0 {
            self.rng = seed.wrapping_mul(0x9E37_79B9) ^ 0x5EED_1234;
            self.unit();
        }
        let Some(b) = block_at(data, pattern, length as usize, i) else {
            self.module = 0;
            self.block_key = (pass, usize::MAX);
            return;
        };
        if self.block_key == (pass, b.start) {
            return;
        }
        self.block_key = (pass, b.start);
        let built_in_on = |m: u8| m as usize > BUILT_IN || p[MODULE_PARAMS[m as usize - 1][0]] >= 0.5;
        let solo = p[SOLO].round().max(0.0) as u8;
        let allowed = built_in_on(b.module) && (solo == 0 || solo == b.module);
        let fire = allowed && self.unit() < b.chance as f32 / 100.0 * p[AMOUNT].clamp(0.0, 1.0);
        self.module = if fire { b.module } else { 0 };
        let into = i - b.start;
        self.seg_len = b.len * self.step_len;
        self.pos = into * self.step_len;
        self.seg_start = self.w.wrapping_sub(self.pos);
        // Variation: some of this module's knobs take a random turn.
        self.varied.iter_mut().for_each(|v| *v = None);
        if fire && (b.module as usize) <= BUILT_IN && self.unit() < p[VARIATION].clamp(0.0, 1.0) {
            for &k in &MODULE_PARAMS[b.module as usize - 1][1..] {
                if self.unit() < 0.5 {
                    let n = self.unit();
                    self.varied[k] = Some(SPECS[k].from_norm(n));
                }
            }
        }
        // Module state for the new block.
        self.tape_p = self.pos as f64;
        self.rt_rep = 0;
        self.rt_start = 0;
        self.rt_base = ((REPEAT_BEATS[self.choice(p, RT_RATE, REPEATS.len())] * spb) as usize).clamp(32, self.seg_len.max(32));
        self.rt_len = self.rt_base as f64;
        self.sh_start = self.pos;
        self.sh_len = 0;
        self.hold_n = 0;
        self.gate_env = [1.0; 2];
    }

    /// A new piece for the Shuffler.
    fn shuffle_piece(&mut self, p: &[f32], spb: f64) {
        let (a, b) = (self.v(p, SH_MIN).round().max(1.0) as usize, self.v(p, SH_MAX).round().max(1.0) as usize);
        let (lo, hi) = (a.min(b), a.max(b));
        let steps = lo + (self.unit() * (hi - lo + 1) as f32) as usize % (hi - lo + 1);
        let left = self.seg_len.saturating_sub(self.pos).max(64);
        self.sh_start = self.pos;
        self.sh_len = (steps * self.step_len).min(left).max(64);
        self.sh_on = self.unit() < self.v(p, SH_AMOUNT);
        self.sh_rev = self.unit() < self.v(p, SH_REVERSE);
        let range = (RANGE_BEATS[self.choice(p, SH_RANGE, RANGES.len())] * spb) as usize;
        let options = (range / self.step_len).max(1);
        let back = (self.step_len * (1 + (self.unit() * options as f32) as usize % options)).max(self.sh_len);
        self.sh_src = self.w as f64 - back as f64;
    }

    /// How much of something plays at `pos` of `len`: faded in and out over `f`.
    #[inline]
    fn edge(pos: usize, len: usize, f: usize) -> f32 {
        let f = f.max(1) as f32;
        ((pos as f32 / f).min((len.saturating_sub(pos)) as f32 / f)).clamp(0.0, 1.0)
    }

    pub fn process(&mut self, p: &[f32], children: &[Device], data: &[u8], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            self.chunk(p, children, data, &mut l[done..done + len], &mut r[done..done + len], ctx, done);
            done += len;
        }
        if let Some(m) = &mut self.taps.meters {
            let length = p[LENGTH].round().clamp(1.0, STEPS as f32) as i64;
            m.values = [0.0; crate::scope::METER_VALUES];
            m.values[0] = if self.step_k == i64::MIN { -1.0 } else { self.step_k.rem_euclid(length) as f32 };
            m.values[1] = self.module as f32;
            m.values[2] = self.pos as f32 / self.seg_len.max(1) as f32;
            m.values[3] = 1.0;
            m.values[4] = if self.block_key.1 == usize::MAX { -1.0 } else { self.block_key.1 as f32 };
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn chunk(&mut self, p: &[f32], children: &[Device], data: &[u8], l: &mut [f32], r: &mut [f32], ctx: &FxCtx, offset: usize) {
        let n = l.len();
        let sr = self.sr as f64;
        let bpm = ctx.bpm.max(20.0) as f64;
        let spb = sr * 60.0 / bpm;
        let bps = 1.0 / spb;
        let step_beats = STEP_BEATS[(p[STEP].round().max(0.0) as usize).min(STEP_BEATS.len() - 1)];
        let mix = p[MIX].clamp(0.0, 1.0);
        let out_g = db_to_gain(p[OUTPUT]);
        let fade = ((0.3 + 20.0 * p[DECLICK].clamp(0.0, 1.0)) * 0.001 * self.sr) as usize;
        let env_vol = p[ENV_VOLUME].clamp(0.0, 1.0);
        let od_mix = p[OD_MIX].clamp(0.0, 1.0);
        let od_g = db_to_gain(p[OD_GAIN].clamp(0.0, 36.0));
        let od_norm = 1.0 / od_g.sqrt();
        let fmode = (p[FILTER_MODE].round().max(0.0) as u8).min(3);
        if fmode != 0 && self.filter_set != (p[CUTOFF], p[RESONANCE], fmode) {
            self.filter_set = (p[CUTOFF], p[RESONANCE], fmode);
            for f in self.svf.iter_mut() {
                f.set(p[CUTOFF].clamp(20.0, 20000.0), p[RESONANCE].clamp(0.0, 1.0) * 0.95, self.sr);
            }
        }
        // The delay's settings (it rings on between its blocks).
        let fb = self.v(p, DL_FEEDBACK).clamp(0.0, 0.95);
        let pp = self.v(p, DL_SPREAD).clamp(0.0, 1.0);
        let dtime = ((TIME_BEATS[self.choice(p, DL_TIME, TIMES.len())] * spb) as usize).clamp(1, DELAY_BUF - 1);
        let tone_k = 1.0 - (-std::f32::consts::TAU * self.v(p, DL_TONE).clamp(200.0, 20000.0) / self.sr).exp();
        for i in 0..n {
            let beat = if ctx.playing { ctx.beat + (offset + i) as f64 * bps } else { self.free_beat };
            if !ctx.playing {
                self.free_beat += bps;
            }
            let k = (beat / step_beats + 1e-9).floor() as i64;
            if k != self.step_k {
                self.begin_step(k, p, data, spb);
            }
            let input = [l[i], r[i]];
            for ch in 0..2 {
                self.buf[ch][self.w & (BUF - 1)] = input[ch];
            }
            let (pos, len, start) = (self.pos.min(self.seg_len), self.seg_len, self.seg_start as f64);
            let mut y = input;
            let mut weights = [0.0f32; SLOTS];
            let mut send = 0.0;
            let mut w = if self.module != 0 { Self::edge(pos, len, fade) } else { 0.0 };
            match self.module {
                1 => {
                    // Tape Stop: wait, slow to a halt, stay, spin up at the end.
                    let t = pos as f32 / len.max(1) as f32;
                    let up = self.v(p, TS_UP).clamp(0.0, 1.0);
                    let wait = self.v(p, TS_DELAY).clamp(0.0, 1.0) * (1.0 - up);
                    let slow = (self.v(p, TS_SLOW).clamp(0.02, 1.0) * (1.0 - up - wait)).max(0.01);
                    let e = 0.4 + 2.0 * self.v(p, TS_CURVE).clamp(0.0, 1.0);
                    let rate = if t < wait {
                        1.0
                    } else if t < wait + slow {
                        (1.0 - (t - wait) / slow).max(0.0).powf(e)
                    } else if t < 1.0 - up {
                        0.0
                    } else {
                        ((t - (1.0 - up)) / up.max(1e-3)).min(1.0).powf(e)
                    };
                    self.tape_p += rate as f64;
                    for ch in 0..2 {
                        y[ch] = self.read(ch, start + self.tape_p.min(pos as f64));
                    }
                    w = w.min(Self::edge(pos, len, fade.max(96)));
                }
                2 => {
                    // Modulator: ring modulation, FM'd, spread across the sides.
                    let f = self.v(p, MOD_FREQ).clamp(0.5, 8000.0) as f64;
                    let fm = self.v(p, MOD_FM).clamp(0.0, 1.0) as f64;
                    self.fm_phase = (self.fm_phase + self.v(p, MOD_FM_SPEED).clamp(0.01, 100.0) as f64 / sr).fract();
                    let inst = f * 2f64.powf(2.0 * fm * (std::f64::consts::TAU * self.fm_phase).sin());
                    self.phase = (self.phase + inst / sr).fract();
                    let sq = self.v(p, MOD_SHAPE).clamp(0.0, 1.0);
                    let spread = self.v(p, MOD_SPREAD).clamp(0.0, 180.0) as f64 / 360.0;
                    let ring = self.v(p, MOD_MIX).clamp(0.0, 1.0);
                    for ch in 0..2 {
                        let s = (std::f64::consts::TAU * (self.phase + if ch == 1 { spread } else { 0.0 })).sin() as f32;
                        let c = s * (1.0 - sq) + (6.0 * s).tanh() * sq;
                        y[ch] = input[ch] + (input[ch] * c - input[ch]) * ring;
                    }
                }
                3 => {
                    // Retrigger: the start of the block again and again.
                    let change = 2f64.powf(-0.5 * self.v(p, RT_SPEED).clamp(-1.0, 1.0) as f64);
                    while (pos - self.rt_start) as f64 >= self.rt_len {
                        self.rt_rep += 1;
                        self.rt_start += self.rt_len as usize;
                        self.rt_len = (self.rt_len * change).clamp(32.0, (self.rt_base * 4) as f64);
                    }
                    let rep = self.rt_rep;
                    let j = pos - self.rt_start;
                    let rate = 2f64.powf((self.v(p, RT_PITCH).round() * rep as f32 / 12.0) as f64).clamp(0.125, 8.0);
                    let src = if rep == 0 { j as f64 } else { (j as f64 * rate) % self.rt_base as f64 };
                    let gate_len = (self.rt_len * self.v(p, RT_LENGTH).clamp(0.1, 1.0) as f64) as usize;
                    let g = if j >= gate_len {
                        0.0
                    } else if rep == 0 {
                        Self::edge(j + gate_len, 2 * gate_len, 48)
                    } else {
                        Self::edge(j, gate_len, 48)
                    };
                    let g = g * (1.0 - self.v(p, RT_DECAY).clamp(0.0, 1.0)).powi(rep as i32);
                    for ch in 0..2 {
                        y[ch] = self.read(ch, start + src) * g;
                    }
                }
                4 => {
                    // Shuffler: pieces from earlier on (some backwards).
                    if pos >= self.sh_start + self.sh_len {
                        self.shuffle_piece(p, spb);
                    }
                    if self.sh_on {
                        let j = (pos - self.sh_start) as f64;
                        let at = if self.sh_rev { self.sh_src + self.sh_len as f64 - 1.0 - j } else { self.sh_src + j };
                        let e = Self::edge(pos - self.sh_start, self.sh_len, 64);
                        for ch in 0..2 {
                            y[ch] = input[ch] + (self.read(ch, at) - input[ch]) * e;
                        }
                    }
                }
                5 => {
                    // Reverser.
                    let chunk = ((CHUNK_BEATS[self.choice(p, RV_CHUNK, CHUNKS.len())] * spb) as usize).clamp(64, len.max(64));
                    let spread = self.v(p, RV_SPREAD).clamp(0.0, 1.0) as f64 * chunk as f64 * 0.5;
                    let (at, e) = match self.choice(p, RV_MODE, REVERSE_MODES.len()) {
                        // Ping Pong: every other chunk is the one before, backwards.
                        1 => {
                            let (c, j) = (pos / chunk, pos % chunk);
                            if c % 2 == 1 {
                                (Some(start + (c * chunk) as f64 - 1.0 - j as f64), Self::edge(j, chunk, 48))
                            } else {
                                (None, 0.0)
                            }
                        }
                        // Mirror: the first half, then it backwards.
                        2 => {
                            let half = len / 2;
                            if pos >= half {
                                (Some(start + half as f64 - 1.0 - (pos - half) as f64), Self::edge(pos - half, len - half, 48))
                            } else {
                                (None, 0.0)
                            }
                        }
                        _ => (Some(start - 1.0 - pos as f64), 1.0),
                    };
                    if let Some(at) = at {
                        for ch in 0..2 {
                            let a = if ch == 1 { at - spread } else { at };
                            y[ch] = input[ch] + (self.read(ch, a) - input[ch]) * e;
                        }
                    }
                }
                6 => {
                    // Crusher: hold every few samples, fewer steps of level, smoothed.
                    if self.hold_n == 0 {
                        self.hold = input;
                    }
                    self.hold_n = (self.hold_n + 1) % (self.v(p, CR_DOWN).round().clamp(1.0, 40.0) as u32);
                    let q = 2f32.powf(self.v(p, CR_BITS).round().clamp(1.0, 16.0) - 1.0);
                    let smooth = self.v(p, CR_SMOOTH).clamp(0.0, 1.0);
                    let kk = 1.0 - (-std::f32::consts::TAU * 18000.0 * (1.0f32 / 18.0).powf(smooth) / self.sr).exp();
                    for ch in 0..2 {
                        let c = (self.hold[ch] * q).round() / q;
                        self.smooth[ch] += (c - self.smooth[ch]) * kk;
                        y[ch] = if smooth < 0.01 { c } else { self.smooth[ch] };
                    }
                }
                7 => {
                    // Gater: open for Duty of every beat of the rate.
                    let g = ((GATE_BEATS[self.choice(p, GT_RATE, GATE_RATES.len())] * spb) as usize).max(8);
                    let duty = self.v(p, GT_DUTY).clamp(0.05, 0.95);
                    let t = 0.0003 + 0.03 * self.v(p, GT_SMOOTH).clamp(0.0, 1.0);
                    let kk = 1.0 - (-1.0 / (t * self.sr)).exp();
                    let off = (self.v(p, GT_STEREO).clamp(0.0, 1.0) * g as f32 * 0.5) as usize;
                    for ch in 0..2 {
                        let at = pos + if ch == 1 { off } else { 0 };
                        let open = (at % g) as f32 / (g as f32) < duty;
                        self.gate_env[ch] += (if open { 1.0 } else { 0.0 } - self.gate_env[ch]) * kk;
                        y[ch] = input[ch] * self.gate_env[ch];
                    }
                }
                8 => {
                    // Stretcher: slower (from the block's start) or faster
                    // (from further back), on tape or in grains.
                    let s = SPEED_VALUES[self.choice(p, ST_SPEED, SPEEDS.len())];
                    let rp = |t: f64| if s < 1.0 { start + t * s } else { start - len as f64 * (s - 1.0) + t * s };
                    if self.choice(p, ST_MODE, 2) == 1 {
                        let g = ((self.v(p, ST_GRAIN).clamp(10.0, 200.0) * 0.001 * self.sr) as usize).clamp(64, len.max(64));
                        y = [0.0; 2];
                        for gi in 0..2 {
                            let tg = (pos + gi * g / 2) % g;
                            let gs = pos as f64 - tg as f64;
                            let at = rp(gs) + tg as f64;
                            let win = (std::f32::consts::PI * tg as f32 / g as f32).sin().powi(2);
                            for (ch, yc) in y.iter_mut().enumerate() {
                                *yc += self.read(ch, at) * win;
                            }
                        }
                    } else {
                        for ch in 0..2 {
                            y[ch] = self.read(ch, rp(pos as f64));
                        }
                    }
                }
                9 => {
                    // Delay: this block goes into the echo.
                    send = w;
                }
                m if m >= FIRST_SLOT => {
                    let s = (m - FIRST_SLOT) as usize;
                    if s < SLOTS {
                        weights[s] = Self::edge(pos, len, fade.max(96));
                    }
                    w = 0.0;
                }
                _ => {}
            }
            if self.module != 7 {
                for e in self.gate_env.iter_mut() {
                    *e = 1.0;
                }
            }
            if w > 0.0 {
                // The Step Envelope, the Overdrive and the Master Filter shape
                // what the modules make.
                let env = 1.0 - (1.0 - env_vol) * (pos as f32 / (p[ENV_LENGTH].clamp(0.25, 16.0) * self.step_len as f32)).min(1.0);
                for ch in 0..2 {
                    let mut v = y[ch] * env;
                    if od_mix > 0.0 {
                        v += ((v * od_g).tanh() * od_norm - v) * od_mix;
                    }
                    y[ch] = v;
                }
            }
            if fmode != 0 {
                for ch in 0..2 {
                    let o = self.svf[ch].tick(if w > 0.0 { y[ch] } else { input[ch] });
                    if w > 0.0 {
                        y[ch] = match fmode {
                            2 => o.high,
                            3 => o.band,
                            _ => o.low,
                        };
                    }
                }
            }
            let mut out = [input[0] + (y[0] - input[0]) * w, input[1] + (y[1] - input[1]) * w];
            // The delay rings on; it hears only its blocks.
            let rd = (self.dw + DELAY_BUF - dtime) & (DELAY_BUF - 1);
            let d = [self.delay[0][rd], self.delay[1][rd]];
            for ch in 0..2 {
                let fbk = d[ch] * (1.0 - pp) + d[1 - ch] * pp;
                self.tone[ch] += (fbk - self.tone[ch]) * tone_k;
                let fresh = if ch == 1 { input[ch] * send * (1.0 - pp) } else { input[ch] * send };
                self.delay[ch][self.dw] = fresh + self.tone[ch] * fb;
                out[ch] += d[ch];
            }
            self.dw = (self.dw + 1) & (DELAY_BUF - 1);
            l[i] = out[0];
            r[i] = out[1];
            self.weights[i] = weights;
            self.w = self.w.wrapping_add(1);
            self.pos += 1;
        }
        // The slots: each hears the music during its blocks and adds what it
        // makes; the music itself gives way while a slot has the block.
        for s in 0..SLOTS.min(children.len()).min(self.children.len()) {
            let dev = &children[s];
            if !dev.enabled {
                continue;
            }
            for i in 0..n {
                let w = self.weights[i][s];
                let at = self.w.wrapping_sub(n - i) & (BUF - 1);
                let dry = [self.buf[0][at], self.buf[1][at]];
                self.sl[i] = dry[0] * w;
                self.sr_buf[i] = dry[1] * w;
                l[i] -= dry[0] * w;
                r[i] -= dry[1] * w;
            }
            let (_, st) = &mut self.children[s];
            st.process(&dev.params, &mut self.sl[..n], &mut self.sr_buf[..n], ctx);
            for i in 0..n {
                l[i] += self.sl[i];
                r[i] += self.sr_buf[i];
            }
        }
        // Mix with the music as it came in.
        for i in 0..n {
            let at = self.w.wrapping_sub(n - i) & (BUF - 1);
            let (dl, dr) = (self.buf[0][at], self.buf[1][at]);
            l[i] = (dl * (1.0 - mix) + l[i] * mix) * out_g;
            r[i] = (dr * (1.0 - mix) + r[i] * mix) * out_g;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn ctx(beat: f64) -> FxCtx {
        FxCtx { sr: SR, bpm: 120.0, beat, playing: true }
    }

    fn params() -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        p[STEP] = 1.0; // 1/16: 6000 samples at 120 BPM
        p[LENGTH] = 16.0;
        p
    }

    /// One block of `module` on every step (each its own block).
    fn every_step(module: u8) -> Vec<u8> {
        let mut data = empty_data();
        for i in 0..STEPS {
            paint(&mut data, 0, i, i, module);
        }
        data
    }

    fn run_with(p: &[f32], data: &[u8], input: &[f32]) -> Vec<f32> {
        let mut g = Glitch::new(SR);
        let mut out = Vec::new();
        let spb = SR as f64 / 2.0;
        for (b, chunk) in input.chunks(256).enumerate() {
            let mut l = chunk.to_vec();
            let mut r = chunk.to_vec();
            g.process(p, &[], data, &mut l, &mut r, &ctx(b as f64 * 256.0 / spb));
            assert!(l.iter().all(|v| v.is_finite() && v.abs() < 4.0));
            out.extend_from_slice(&l);
        }
        out
    }

    fn run(module: u8, p: &[f32], input: &[f32]) -> Vec<f32> {
        run_with(p, &every_step(module), input)
    }

    fn ramp(secs: f32) -> Vec<f32> {
        (0..(secs * SR) as usize).map(|i| ((i as f32 / SR) * 0.37).fract() - 0.5).collect()
    }

    fn sine(secs: f32, hz: f32) -> Vec<f32> {
        (0..(secs * SR) as usize).map(|i| (std::f32::consts::TAU * hz * i as f32 / SR).sin() * 0.5).collect()
    }

    #[test]
    fn painting_makes_blocks_and_cuts_them() {
        let mut d = empty_data();
        paint(&mut d, 0, 2, 7, 3);
        assert_eq!(blocks(&d, 0, 32), vec![Block { start: 2, len: 6, module: 3, chance: 100 }]);
        // Painting over the middle cuts it in three.
        paint(&mut d, 0, 4, 5, 6);
        let b = blocks(&d, 0, 32);
        assert_eq!(b.iter().map(|b| (b.start, b.len, b.module)).collect::<Vec<_>>(), vec![(2, 2, 3), (4, 2, 6), (6, 2, 3)]);
        // Two blocks of the same module side by side stay two.
        paint(&mut d, 0, 10, 11, 1);
        paint(&mut d, 0, 12, 13, 1);
        assert_eq!(blocks(&d, 0, 32).iter().filter(|b| b.module == 1).count(), 2);
        // Erasing.
        paint(&mut d, 0, 0, 31, 0);
        assert!(blocks(&d, 0, 32).is_empty());
    }

    #[test]
    fn old_patterns_are_converted() {
        let mut old = vec![0u8; 256];
        old[0] = 1; // Retrigger
        old[1] = 100;
        old[2] = 4; // Tape Stop
        old[3] = 50;
        assert!(migrate(&mut old));
        let b = blocks(&old, 0, 32);
        assert_eq!(b[0], Block { start: 0, len: 1, module: 3, chance: 100 });
        assert_eq!(b[1], Block { start: 1, len: 1, module: 1, chance: 50 });
    }

    #[test]
    fn randomise_keeps_within_the_length_and_the_modules() {
        let mut d = default_data();
        for (what, t) in [(Randomise::Steps, 0), (Randomise::Fx, 1), (Randomise::Both, 2), (Randomise::Both, 3)] {
            randomise(&mut d, 1, 24, what, t, &[2, 3, 7], 1234 + t as u32);
            let b = blocks(&d, 1, 64);
            assert!(b.iter().all(|b| b.start < 24), "{what:?}");
            if what != Randomise::Steps {
                assert!(b.iter().all(|b| [2, 3, 7].contains(&b.module)), "{what:?} {b:?}");
            }
        }
    }

    #[test]
    fn nothing_set_is_the_music_itself() {
        let input = ramp(1.0);
        let out = run(0, &params(), &input);
        assert!(out.iter().zip(&input).all(|(a, b)| (a - b).abs() < 1e-6));
    }

    #[test]
    fn a_block_lasts_its_steps() {
        // Gater on steps 2..=5 of 1/16: chopped from 12000 to 36000, music around it.
        let input = vec![0.5f32; 48000];
        let mut d = empty_data();
        paint(&mut d, 0, 2, 5, 7);
        let out = run_with(&params(), &d, &input);
        let low = |a: usize, b: usize| out[a..b].iter().fold(1.0f32, |m, v| m.min(*v));
        assert!((low(1000, 11000) - 0.5).abs() < 1e-4);
        assert!(low(14000, 35000) < 0.05);
        assert!((low(37000, 47000) - 0.5).abs() < 1e-4);
    }

    #[test]
    fn retrigger_repeats_the_start_of_the_block() {
        // Repeats of 1/32 (3000 samples) inside a 1/16 block (6000).
        let input = ramp(1.0);
        let mut p = params();
        p[RT_DECAY] = 0.0;
        p[RT_LENGTH] = 1.0;
        let out = run(3, &p, &input);
        let s = 12000;
        for j in [100usize, 1000, 2500] {
            assert!((out[s + 3000 + j] - out[s + j]).abs() < 1e-3, "{j}: {} vs {}", out[s + 3000 + j], out[s + j]);
        }
    }

    #[test]
    fn reverse_plays_the_block_before_backwards() {
        let input = ramp(1.0);
        let out = run(5, &params(), &input);
        let s = 12000;
        let j = 3000;
        assert!((out[s + j] - input[s - 1 - j]).abs() < 1e-3);
    }

    #[test]
    fn tape_stop_slows_to_a_halt() {
        let input = sine(1.0, 440.0);
        let mut p = params();
        p[STEP] = 0.0; // 1/8: 12000 samples
        p[TS_UP] = 0.0;
        let out = run(1, &p, &input);
        let s = 24000;
        let early: f32 = (s + 200..s + 400).map(|i| (out[i + 1] - out[i]).abs()).sum();
        let late: f32 = (s + 11000..s + 11200).map(|i| (out[i + 1] - out[i]).abs()).sum();
        assert!(late < 0.2 * early, "{early} {late}");
    }

    #[test]
    fn gate_chops_and_crush_quantizes() {
        let input = vec![0.5f32; 48000];
        let gated = run(7, &params(), &input);
        let lo = gated[12000..18000].iter().fold(1.0f32, |m, v| m.min(*v));
        assert!(lo < 0.05, "{lo}");
        let mut p = params();
        p[CR_BITS] = 2.0;
        let input: Vec<f32> = (0..48000).map(|i| (i as f32 / 48000.0) - 0.5).collect();
        let crushed = run(6, &p, &input);
        let mut levels: Vec<i32> = crushed[12500..17500].iter().map(|v| (v * 1000.0).round() as i32).collect();
        levels.sort();
        levels.dedup();
        assert!(levels.len() <= 5, "{levels:?}");
    }

    #[test]
    fn modulator_rings_the_sound() {
        // A steady level times a 125 Hz sine: it swings both ways.
        let input = vec![0.5f32; 48000];
        let mut p = params();
        p[MOD_FM] = 0.0;
        let out = run(2, &p, &input);
        let (lo, hi) = out[13000..17000].iter().fold((1.0f32, -1.0f32), |(a, b), v| (a.min(*v), b.max(*v)));
        assert!(lo < -0.4 && hi > 0.4, "{lo} {hi}");
    }

    #[test]
    fn grains_keep_the_pitch_and_tape_drops_it() {
        // Half speed: on tape 440 Hz becomes 220 Hz; in grains it stays near 440.
        let input = sine(1.0, 440.0);
        let crossings = |v: &[f32]| v.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        let mut p = params();
        p[STEP] = 0.0;
        p[ST_SPEED] = 2.0;
        let tape = run(8, &p, &input);
        p[ST_MODE] = 1.0;
        let grains = run(8, &p, &input);
        // 4800 samples (0.1 s) mid-block.
        let (t, g) = (crossings(&tape[26000..30800]), crossings(&grains[26000..30800]));
        assert!((18..=26).contains(&t), "tape {t}");
        assert!((38..=50).contains(&g), "grains {g}");
    }

    #[test]
    fn shuffler_plays_earlier_music() {
        let input = ramp(2.0);
        let mut p = params();
        p[SH_AMOUNT] = 1.0;
        p[SH_REVERSE] = 0.0;
        let out = run(4, &p, &input);
        // Late in the run, the output often differs from the input (taken from earlier).
        let differ = (60000..90000).filter(|&i| (out[i] - input[i]).abs() > 0.01).count();
        assert!(differ > 15000, "{differ}");
    }

    #[test]
    fn delay_rings_on_after_its_block() {
        let mut input = vec![0.0f32; 96000];
        input[1000] = 1.0;
        let mut d = empty_data();
        paint(&mut d, 0, 0, 0, 9);
        let mut p = params();
        p[LENGTH] = 64.0;
        p[DL_SPREAD] = 0.0;
        p[DL_TONE] = 18000.0;
        let out = run_with(&p, &d, &input);
        // A 3/16 note at 120 BPM: 0.75 beats, 18000 samples.
        let echo = out[18900..19300].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        let second = out[36900..37300].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(echo > 0.3 && second > 0.1 && second < echo, "{echo} {second}");
    }

    #[test]
    fn seed_repeats_the_dice_each_time_round() {
        // Chance 50 on every step: with a seed, the second pass equals the first.
        let input = vec![0.5f32; 96000];
        let mut d = empty_data();
        for i in 0..STEPS {
            paint(&mut d, 0, i, i, 7);
            set_chance(&mut d, 0, i, 50);
        }
        let mut p = params();
        p[SEED] = 7.0;
        p[LENGTH] = 8.0; // 48000 samples a pass
        let out = run_with(&p, &d, &input);
        let diff: f32 = (0..48000).map(|i| (out[i] - out[i + 48000]).abs()).sum();
        assert!(diff < 1.0, "{diff}");
        // And some steps fired, some did not.
        let gated = (0..8).filter(|s| out[s * 6000 + 2000..s * 6000 + 4000].iter().any(|v| *v < 0.05)).count();
        assert!(gated > 0 && gated < 8, "{gated}");
    }

    #[test]
    fn a_switched_off_module_or_another_solo_plays_the_music() {
        let input = vec![0.5f32; 48000];
        let mut p = params();
        p[GT_ON] = 0.0;
        let out = run(7, &p, &input);
        assert!(out.iter().all(|v| (v - 0.5).abs() < 1e-4));
        let mut p = params();
        p[SOLO] = 3.0;
        let out = run(7, &p, &input);
        assert!(out.iter().all(|v| (v - 0.5).abs() < 1e-4));
    }

    #[test]
    fn a_slot_effect_hears_only_its_blocks() {
        let input = vec![0.5f32; 48000];
        let mut g = Glitch::new(SR);
        let mut util = crate::params::defaults(super::super::super::effects::EffectKind::Utility.specs());
        util[0] = -60.0;
        let child = Device { id: 7, kind: super::super::super::effects::EffectKind::Utility, enabled: true, params: util, plugin: None, data: Vec::new(), children: Vec::new() , preset: String::new(), sample: None };
        g.children.push((7, EffectState::new(child.kind, SR)));
        let mut data = empty_data();
        paint(&mut data, 0, 1, 1, FIRST_SLOT);
        let p = params();
        let mut out = Vec::new();
        for (b, chunk) in input.chunks(256).enumerate() {
            let mut l = chunk.to_vec();
            let mut r = chunk.to_vec();
            g.process(&p, std::slice::from_ref(&child), &data, &mut l, &mut r, &ctx(b as f64 * 256.0 / 24000.0));
            out.extend_from_slice(&l);
        }
        assert!(out[9000].abs() < 0.01, "{}", out[9000]);
        assert!((out[3000] - 0.5).abs() < 1e-3 && (out[15000] - 0.5).abs() < 1e-3);
    }
}
