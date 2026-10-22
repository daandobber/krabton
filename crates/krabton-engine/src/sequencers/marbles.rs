//! Marbles: a random sampler, after Mutable Instruments Marbles. Its T
//! section rolls the dice on every tick of a clock: T2 is the clock itself,
//! T1 and T3 are triggers decided by a model (a coin toss between them,
//! clusters, drum patterns, two independent coins, a divider, three states
//! or a Markov chain), with Bias leaning towards one or the other and Jitter
//! loosening the clock. Its X section draws three random values from a
//! distribution (Spread and Bias), quantized to the scale with fewer notes
//! the more Steps; each changes when its T fires. Déjà Vu makes both loop:
//! at noon a loop of Length values repeats, to the left it slowly changes,
//! to the right it jumps around inside it.
//!
//! As drums, T1, T2 and T3 play three drums and the X values their
//! dynamics; as notes, each T plays its X value.
//!
//! The T and X generators are ported from Marbles' firmware:
//!
//! Copyright 2015 Emilie Gillet.
//!
//! Permission is hereby granted, free of charge, to any person obtaining a
//! copy of this software and associated documentation files (the
//! "Software"), to deal in the Software without restriction, including
//! without limitation the rights to use, copy, modify, merge, publish,
//! distribute, sublicense, and/or sell copies of the Software, and to permit
//! persons to whom the Software is furnished to do so, subject to the
//! following conditions:
//!
//! The above copyright notice and this permission notice shall be included
//! in all copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
//! OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
//! MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
//! NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
//! DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
//! OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
//! USE OR OTHER DEALINGS IN THE SOFTWARE.

use super::SeqState;
use crate::dsp::Rng;
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

pub const RATE: usize = 0;
pub const MODEL: usize = 1;
pub const BIAS: usize = 2;
pub const JITTER: usize = 3;
pub const GATE: usize = 4;
pub const GATE_RANDOM: usize = 5;
pub const DEJA_VU: usize = 6;
pub const LENGTH: usize = 7;
pub const T_LOOP: usize = 8;
pub const X_LOOP: usize = 9;
pub const SPREAD: usize = 10;
pub const X_BIAS: usize = 11;
pub const STEPS: usize = 12;
pub const CONTROL: usize = 13;
pub const RANGE: usize = 14;
pub const BASE: usize = 15;
pub const PLAY: usize = 16;
pub const T_ON: usize = 17; // T1, T2, T3: 17..=19.
pub const T_NOTE: usize = 20; // Their drums: 20..=22.
pub const VELOCITY: usize = 23;
pub const ACCENTS: usize = 24;

pub const MODELS: &[&str] = &["Coin Toss", "Clusters", "Drums", "Two Coins", "Divider", "Three States", "Markov"];
pub const LOOPS: &[&str] = &["Off", "On", "Locked"];
pub const CONTROLS: &[&str] = &["Identical", "Bump", "Tilt"];
pub const PLAYS: &[&str] = &["Drums", "Notes"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "T", super::RATES, 2),
    ParamSpec::choice("Model", "T", MODELS, 0),
    ParamSpec::lin("Bias", "T", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Jitter", "T", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Gate", "T", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Gate Random", "T", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Deja Vu", "Loop", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::int("Length", "Loop", 1.0, 16.0, 8.0, Unit::None),
    ParamSpec::choice("T Loop", "Loop", LOOPS, 1),
    ParamSpec::choice("X Loop", "Loop", LOOPS, 1),
    ParamSpec::lin("Spread", "X", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("X Bias", "X", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Steps", "X", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::choice("Control", "X", CONTROLS, 0),
    ParamSpec::int("Range", "X", 1.0, 4.0, 2.0, Unit::None),
    ParamSpec::int("Base Note", "X", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::choice("Play", "Out", PLAYS, 0),
    ParamSpec::toggle("T1", "Out", true),
    ParamSpec::toggle("T2", "Out", true),
    ParamSpec::toggle("T3", "Out", true),
    ParamSpec::int("T1 Drum", "Out", 0.0, 127.0, 36.0, Unit::None),
    ParamSpec::int("T2 Drum", "Out", 0.0, 127.0, 42.0, Unit::None),
    ParamSpec::int("T3 Drum", "Out", 0.0, 127.0, 38.0, Unit::None),
    ParamSpec::lin("Velocity", "Out", 0.05, 1.0, 0.85, Unit::Percent),
    ParamSpec::lin("Accents", "Out", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

// ---------------------------------------------------------------- distributions

const TABLE: usize = 387;
const NUM_BIAS: usize = 5;
const NUM_RANGE: usize = 9;

fn tables() -> &'static [f32] {
    static T: std::sync::OnceLock<Vec<f32>> = std::sync::OnceLock::new();
    T.get_or_init(|| include_bytes!("marbles_beta.bin").chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect())
}

fn interpolate(t: &[f32], x: f32, size: f32) -> f32 {
    let xi = (x * size).max(0.0);
    let i = (xi as usize).min(t.len().saturating_sub(2));
    let f = xi - i as f32;
    t[i] + (t[i + 1] - t[i]) * f
}

/// A sample of a beta distribution from a uniform one: `spread` from
/// certain (0) to all over (1), `bias` where it leans.
pub fn beta_sample(mut uniform: f32, spread: f32, mut bias: f32) -> f32 {
    let t = tables();
    let flip = bias > 0.5;
    if flip {
        uniform = 1.0 - uniform;
        bias = 1.0 - bias;
    }
    let bias = bias.clamp(0.0, 0.5) * (NUM_BIAS as f32 - 1.0) * 2.0;
    let spread = spread.clamp(0.0, 1.0) * (NUM_RANGE as f32 - 1.0);
    let (bi, bf) = (bias as usize, bias.fract());
    let (si, sf) = (spread as usize, spread.fract());
    let cell = bi * (NUM_RANGE + 1) + si;
    let mut offset = 0;
    if uniform <= 0.05 {
        offset = 129;
        uniform *= 20.0;
    } else if uniform >= 0.95 {
        offset = 258;
        uniform = (uniform - 0.95) * 20.0;
    }
    let at = |c: usize| interpolate(&t[c * TABLE + offset..(c + 1) * TABLE], uniform, 128.0);
    let x1y1 = at(cell);
    let x2y1 = at(cell + 1);
    let x1y2 = at(cell + NUM_RANGE + 1);
    let x2y2 = at(cell + NUM_RANGE + 2);
    let y1 = x1y1 + (x2y1 - x1y1) * sf;
    let y2 = x1y2 + (x2y2 - x1y2) * sf;
    let y = y1 + (y2 - y1) * bf;
    if flip {
        1.0 - y
    } else {
        y
    }
}

/// Beta(3, 3) with a fatter tail.
fn fast_beta_sample(u: f32) -> f32 {
    let c = 4 * (NUM_RANGE + 1) + 3;
    interpolate(&tables()[c * TABLE..c * TABLE + 129], u, 128.0)
}

fn semitones_to_ratio(s: f32) -> f32 {
    (s / 12.0).exp2()
}

/// The X value of a channel: degenerate to its bias at no spread, a coin at
/// full spread, a beta distribution between.
pub fn x_value(u: f32, spread: f32, bias: f32) -> f32 {
    let degenerate = (1.25 - spread * 25.0).clamp(0.0, 1.0);
    let bernoulli = (spread * 25.0 - 23.75).clamp(0.0, 1.0);
    let mut value = beta_sample(u, spread, bias);
    let coin = if u >= 1.0 - bias { 0.999_999 } else { 0.0 };
    value += degenerate * (bias - value);
    value += bernoulli * (coin - value);
    value
}

// ---------------------------------------------------------------- random sequence

const LOOP_SIZE: usize = 16;

/// The déjà vu loop: values are drawn new, replayed or jumped through.
#[derive(Clone, Debug)]
pub struct RandomSequence {
    lp: [f32; LOOP_SIZE],
    write: usize,
    length: usize,
    step: usize,
    deja_vu: f32,
}

impl RandomSequence {
    fn new(rng: &mut Rng) -> Self {
        Self { lp: std::array::from_fn(|_| rng.unipolar()), write: 0, length: 8, step: 0, deja_vu: 0.0 }
    }

    fn set_length(&mut self, length: usize) {
        if (1..=LOOP_SIZE).contains(&length) {
            self.length = length;
            self.step %= length;
        }
    }

    fn reset(&mut self) {
        self.step = self.length - 1;
    }

    fn next_value(&mut self, rng: &mut Rng) -> f32 {
        let p_sqrt = 2.0 * self.deja_vu - 1.0;
        let p = p_sqrt * p_sqrt;
        let mutate = rng.unipolar() < p;
        if mutate && self.deja_vu <= 0.5 {
            // A new value at the end of the loop.
            self.lp[self.write] = rng.unipolar();
            self.write = (self.write + 1) % LOOP_SIZE;
            self.step = self.length - 1;
        } else if mutate {
            // Jump somewhere in the loop.
            self.step = ((rng.unipolar() * self.length as f32) as usize).min(self.length - 1);
        } else {
            self.step += 1;
            if self.step >= self.length {
                self.step = 0;
            }
        }
        self.lp[(self.write + LOOP_SIZE - self.length + self.step) % LOOP_SIZE]
    }

    fn next_vector(&mut self, rng: &mut Rng, out: &mut [f32]) {
        let seed = self.next_value(rng);
        let mut word = (seed * 4_294_967_296.0) as u32;
        for o in out {
            *o = word as f32 / 4_294_967_296.0;
            word = word.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        }
    }

    /// Where in the loop it plays.
    pub fn step(&self) -> usize {
        self.step
    }
}

// ---------------------------------------------------------------- slave ramps

const MAX_RAMP: f32 = 0.9999;

#[derive(Clone, Copy, Debug)]
struct SlaveRamp {
    phase: f32,
    max_phase: f32,
    ratio: f32,
    pulse_width: f32,
    target: f32,
    pulse_length: u32,
    bernoulli: bool,
    must_complete: bool,
}

impl SlaveRamp {
    fn new() -> Self {
        Self { phase: 0.0, max_phase: MAX_RAMP, ratio: 1.0, pulse_width: 0.0, target: 1.0, pulse_length: 0, bernoulli: false, must_complete: false }
    }

    fn reset(&mut self) {
        *self = Self::new();
        self.phase = 1.0;
    }

    /// A multiple or division of the master clock.
    fn init_divided(&mut self, length: i32, ratio: (i32, i32), pulse_width: f32) {
        self.bernoulli = false;
        self.phase = 0.0;
        self.max_phase = length as f32 * MAX_RAMP;
        self.ratio = ratio.0 as f32 / ratio.1 as f32;
        self.pulse_width = pulse_width;
        self.target = 1.0;
        self.pulse_length = 0;
    }

    /// A pulse at the next tick, or a slope halving towards the top while
    /// there is none.
    fn init_bernoulli(&mut self, must_complete: bool, pulse_width: f32, expected: f32) {
        self.bernoulli = true;
        if self.must_complete {
            self.phase = 0.0;
            self.pulse_width = pulse_width;
            self.ratio = 1.0;
            self.pulse_length = 0;
        }
        self.ratio = if must_complete { 1.0 - self.phase } else { (1.0 - self.phase) * expected };
        self.must_complete = must_complete;
    }

    /// Advances one sample: (phase, gate).
    fn process(&mut self, frequency: f32) -> (f32, bool) {
        let out;
        if self.bernoulli {
            self.phase += frequency * self.ratio;
            out = self.phase.min(1.0);
        } else {
            self.phase = (self.phase + frequency).min(self.max_phase);
            let mut o = self.phase * self.ratio;
            if o > self.target {
                self.pulse_length = 0;
                self.target += 1.0;
            }
            o -= o.trunc();
            out = o;
        }
        let gate = if self.pulse_width == 0.0 { self.pulse_length < 32 && out <= 0.5 } else { out < self.pulse_width };
        self.pulse_length = self.pulse_length.saturating_add(1);
        (out, gate)
    }
}

// ---------------------------------------------------------------- T generator

type Ratio = (i32, i32);

const DIVIDER_PATTERNS: [([Ratio; 2], i32); 17] = [
    ([(1, 1), (1, 1)], 1),
    ([(1, 1), (2, 1)], 1),
    ([(1, 2), (1, 1)], 2),
    ([(1, 1), (4, 1)], 1),
    ([(1, 2), (2, 1)], 2),
    ([(1, 1), (3, 2)], 2),
    ([(1, 4), (4, 1)], 4),
    ([(1, 4), (2, 1)], 4),
    ([(1, 2), (3, 2)], 2),
    ([(1, 1), (8, 1)], 1),
    ([(1, 1), (3, 1)], 1),
    ([(1, 3), (1, 1)], 3),
    ([(1, 1), (5, 4)], 4),
    ([(1, 2), (5, 4)], 4),
    ([(1, 1), (6, 1)], 1),
    ([(1, 3), (2, 1)], 3),
    ([(1, 1), (16, 1)], 1),
];

const FIXED_DIVIDER_PATTERNS: [([Ratio; 2], i32); 17] = [
    ([(8, 1), (1, 8)], 8),
    ([(6, 1), (1, 6)], 6),
    ([(4, 1), (1, 4)], 4),
    ([(3, 1), (1, 3)], 3),
    ([(2, 1), (1, 2)], 2),
    ([(3, 2), (2, 3)], 6),
    ([(4, 3), (3, 4)], 12),
    ([(5, 4), (4, 5)], 20),
    ([(1, 1), (1, 1)], 1),
    ([(4, 5), (5, 4)], 20),
    ([(3, 4), (4, 3)], 12),
    ([(2, 2), (3, 2)], 6),
    ([(1, 2), (2, 1)], 2),
    ([(1, 3), (3, 1)], 3),
    ([(1, 4), (4, 1)], 4),
    ([(1, 6), (6, 1)], 6),
    ([(1, 8), (8, 1)], 8),
];

const DRUM_PATTERNS: [[u8; 8]; 18] = [
    [1, 0, 0, 0, 2, 0, 0, 0],
    [0, 0, 1, 0, 2, 0, 0, 0],
    [1, 0, 1, 0, 2, 0, 0, 0],
    [0, 0, 1, 0, 2, 0, 0, 2],
    [1, 0, 1, 0, 2, 0, 1, 0],
    [0, 2, 1, 0, 2, 0, 0, 2],
    [1, 0, 0, 0, 2, 0, 1, 0],
    [0, 2, 1, 0, 2, 0, 1, 2],
    [1, 0, 0, 1, 2, 0, 0, 0],
    [0, 2, 1, 1, 2, 0, 1, 2],
    [1, 0, 0, 1, 2, 0, 1, 0],
    [0, 2, 1, 1, 2, 2, 1, 2],
    [1, 0, 0, 1, 2, 0, 1, 2],
    [0, 2, 0, 1, 2, 0, 1, 2],
    [1, 0, 1, 1, 2, 0, 1, 2],
    [2, 0, 1, 2, 0, 1, 2, 0],
    [1, 2, 1, 1, 2, 0, 1, 2],
    [2, 0, 1, 2, 0, 1, 2, 2],
];

const MARKOV_HISTORY: usize = 16;

/// The random vector of a tick: pulse widths, two uniforms, the Markov
/// déjà vu draw and the jitter.
#[derive(Clone, Copy, Default)]
struct Vector {
    x: [f32; 6],
}

impl Vector {
    fn pulse_width(&self, i: usize) -> f32 {
        self.x[i]
    }
    fn u(&self, i: usize) -> f32 {
        self.x[2 + i]
    }
    fn p(&self) -> f32 {
        self.x[4]
    }
    fn jitter(&self) -> f32 {
        self.x[5]
    }
}

/// stmlib's hysteresis quantizer (not symmetric): an index that does not
/// flicker between neighbours.
#[derive(Clone, Copy, Debug, Default)]
struct Hysteresis {
    value: i32,
}

impl Hysteresis {
    fn process(&mut self, x: f32, steps: i32, hysteresis: f32) -> usize {
        let v = x * steps as f32 - 0.5;
        let sign = if v > self.value as f32 { -1.0 } else { 1.0 };
        let q = ((v + sign * hysteresis + 0.5) as i32).clamp(0, steps - 1);
        self.value = q;
        q as usize
    }
}

#[derive(Clone, Debug)]
pub struct TGenerator {
    pub model: usize,
    pub bias: f32,
    pub jitter: f32,
    pub pulse_width_mean: f32,
    pub pulse_width_std: f32,
    jitter_multiplier: f32,
    divider_pattern_length: i32,
    streak: [i32; 2],
    markov_history: [i32; MARKOV_HISTORY],
    markov_ptr: usize,
    drum_step: usize,
    drum_index: usize,
    pub sequence: RandomSequence,
    slaves: [SlaveRamp; 2],
    bias_quantizer: Hysteresis,
}

impl TGenerator {
    fn new(rng: &mut Rng) -> Self {
        Self {
            model: 0,
            bias: 0.5,
            jitter: 0.0,
            pulse_width_mean: 0.5,
            pulse_width_std: 0.0,
            jitter_multiplier: 1.0,
            divider_pattern_length: 0,
            streak: [0; 2],
            markov_history: [0; MARKOV_HISTORY],
            markov_ptr: 0,
            drum_step: 0,
            drum_index: 0,
            sequence: RandomSequence::new(rng),
            slaves: [SlaveRamp::new(); 2],
            bias_quantizer: Hysteresis::default(),
        }
    }

    fn random_pulse_width(&self, u: f32) -> f32 {
        if self.pulse_width_std == 0.0 {
            0.05 + 0.9 * self.pulse_width_mean
        } else {
            0.05 + 0.9 * beta_sample(u, self.pulse_width_std, self.pulse_width_mean)
        }
    }

    fn complementary_bernoulli(&self, x: &Vector) -> u8 {
        (0..2).filter(|&i| (x.u(0) > self.bias) ^ (i & 1 == 1)).fold(0, |m, i| m | 1 << i)
    }

    fn independent_bernoulli(&self, x: &Vector) -> u8 {
        (0..2).filter(|&i| (x.u(i) > self.bias) ^ (i & 1 == 1)).fold(0, |m, i| m | 1 << i)
    }

    fn three_states(&self, x: &Vector) -> u8 {
        let p_none = 0.75 - (self.bias - 0.5).abs();
        let threshold = p_none + (1.0 - p_none) * (0.25 + self.bias * 0.5);
        let u = x.u(0);
        (0..2).filter(|&i| u > p_none && ((u > threshold) ^ (i & 1 == 1))).fold(0, |m, i| m | 1 << i)
    }

    fn drums(&mut self, x: &Vector) -> u8 {
        self.drum_step += 1;
        if self.drum_step >= 8 {
            self.drum_step = 0;
            let u = x.u(0) * 2.0 * (self.bias - 0.5).abs();
            self.drum_index = ((DRUM_PATTERNS.len() as f32 * u) as usize).min(DRUM_PATTERNS.len() - 1);
            if self.bias <= 0.5 {
                self.drum_index -= self.drum_index % 2;
            }
        }
        DRUM_PATTERNS[self.drum_index][self.drum_step]
    }

    fn markov(&mut self, x: &Vector) -> u8 {
        let mut bitmask = 0;
        let b = 1.5 * self.bias - 0.5;
        let p = self.markov_ptr;
        self.markov_history[p] = 0;
        let h = |k: usize| self.markov_history[(p + k) % MARKOV_HISTORY];
        for i in 0..2 {
            let mask = 1 << i;
            // Repeat what played 8 ticks ago, not both at once, sparse,
            // and one echoing what the other played 4 ticks before.
            let periodic = h(8) & mask != 0;
            let simultaneous = h(8) & !mask != 0;
            let dense = h(1) & mask != 0;
            let alternate = h(4) & !mask != 0;
            let mut logit = -1.5;
            logit += if self.streak[i] > 24 { 10.0 } else { 0.0 };
            logit += 8.0 * b.abs() * if periodic { b } else { -b };
            logit -= 2.0 * if simultaneous { b } else { -b };
            logit -= if dense { b } else { 0.0 };
            logit += if alternate { b } else { 0.0 };
            let logit: f32 = logit.clamp(-10.0, 10.0);
            // Marbles' logit table: 257 steps of log2-odds from -10 to 10.
            let idx = (logit * 12.8 + 128.0) as i32;
            let odds = (idx as f32 / 256.0 * 20.0 - 10.0).exp2();
            let probability = odds / (1.0 + odds);
            let mut state = x.u(i) < probability;
            if self.sequence.deja_vu >= x.p() {
                state = h(self.sequence.length) & mask != 0;
            }
            if state {
                bitmask |= mask;
                self.streak[i] = 0;
            } else {
                self.streak[i] += 1;
            }
        }
        self.markov_history[p] |= bitmask;
        self.markov_ptr = (p + MARKOV_HISTORY - 1) % MARKOV_HISTORY;
        bitmask as u8
    }

    fn schedule(&mut self, x: &Vector, mut bitmask: u8) {
        for i in 0..2 {
            let pw = self.random_pulse_width(x.pulse_width(i));
            self.slaves[i].init_bernoulli(bitmask & 1 == 1, pw, 0.5);
            bitmask >>= 1;
        }
    }

    fn configure(&mut self, x: &Vector) {
        match self.model {
            0 => {
                let m = self.complementary_bernoulli(x);
                self.schedule(x, m);
            }
            3 => {
                let m = self.independent_bernoulli(x);
                self.schedule(x, m);
            }
            5 => {
                let m = self.three_states(x);
                self.schedule(x, m);
            }
            2 => {
                let m = self.drums(x);
                self.schedule(x, m);
            }
            6 => {
                let m = self.markov(x);
                self.schedule(x, m);
            }
            _ => {
                // Clusters (1) and Divider (4).
                self.divider_pattern_length -= 1;
                if self.divider_pattern_length <= 0 {
                    let (mut ratios, length) = if self.model == 4 {
                        FIXED_DIVIDER_PATTERNS[self.bias_quantizer.process(self.bias, 17, 0.1)]
                    } else {
                        let strength = (self.bias - 0.5).abs() * 2.0;
                        let mut u = x.u(0);
                        u *= u + strength * strength * (1.0 - u);
                        u *= strength;
                        let pattern = DIVIDER_PATTERNS[((u * 17.0) as usize).min(16)];
                        pattern
                    };
                    if self.model == 1 && self.bias < 0.5 {
                        ratios.swap(0, 1);
                    }
                    for i in 0..2 {
                        let pw = self.random_pulse_width(x.pulse_width(i));
                        self.slaves[i].init_divided(length, ratios[i], pw);
                    }
                    self.divider_pattern_length = length;
                }
            }
        }
    }

    /// Starts over (a reset): the first tick plays what is decided now.
    fn reset(&mut self, rng: &mut Rng) {
        for s in self.slaves.iter_mut() {
            s.reset();
        }
        self.sequence.reset();
        self.divider_pattern_length = 0;
        self.drum_step = 8;
        if self.model != 4 {
            let mut v = Vector::default();
            self.sequence.next_vector(rng, &mut v.x);
            self.configure(&v);
        }
    }

    /// A tick of the master clock: the dice for the next one, the jitter of
    /// the next period (pulled back when it drifts from the straight clock).
    fn tick(&mut self, rng: &mut Rng, phase_difference: f32) {
        let mut v = Vector::default();
        self.sequence.next_vector(rng, &mut v.x);
        let jitter_amount = self.jitter.powi(4) * 36.0;
        let x = fast_beta_sample(v.jitter());
        let mut multiplier = semitones_to_ratio((x * 2.0 - 1.0) * jitter_amount);
        multiplier *= if phase_difference > 0.0 { 1.0 + phase_difference } else { 1.0 / (1.0 - phase_difference) };
        self.jitter_multiplier = multiplier;
        self.configure(&v);
    }
}

// ---------------------------------------------------------------- X section

/// Marbles' weights of the twelve notes from the root (its C major scale):
/// the higher, the longer a note survives fewer Steps.
const MAJOR_WEIGHTS: [u8; 12] = [255, 16, 96, 24, 128, 64, 8, 192, 16, 96, 24, 128];
/// The thresholds Steps walks through.
const THRESHOLDS: [u8; 7] = [0, 16, 32, 64, 128, 192, 255];

/// The weight of a note `interval` semitones above the root, in the
/// project's scale (or Marbles' major one without it).
fn weight(interval: usize, scale: &crate::scale::Scale, scale_on: bool) -> u8 {
    let i = interval % 12;
    if !scale_on {
        return MAJOR_WEIGHTS[i];
    }
    if scale.contains((scale.root as usize + i) as u8 % 12 + 60) {
        [255, 48, 96, 128, 128, 64, 48, 192, 96, 96, 128, 128][i]
    } else if i == 6 {
        8
    } else {
        16
    }
}

/// Quantizes `pitch` (fractional semitones) to the notes that weigh at least
/// the threshold that `steps` (0..1) picks: all twelve, the scale, its
/// triad, its fifth and root, the root.
pub fn quantize(pitch: f32, steps: f32, scale: &crate::scale::Scale, scale_on: bool) -> i32 {
    let level = (steps.clamp(0.0, 1.0) * 6.0).round() as usize;
    let mut threshold = THRESHOLDS[level];
    let second = (1..12).map(|i| weight(i, scale, scale_on)).filter(|&w| w != 255).max().unwrap_or(0);
    if level == 5 && second > 192 {
        threshold = second;
    }
    let root = scale.root as f32;
    let allowed: Vec<usize> = (0..12).filter(|&i| weight(i, scale, scale_on) >= threshold).collect();
    let allowed = if allowed.is_empty() { vec![0] } else { allowed };
    let rel = pitch - root;
    let octave = (rel / 12.0).floor();
    let frac = rel - octave * 12.0;
    // The tightest notes below and above, and the nearer of the two.
    let mut below = allowed[allowed.len() - 1] as f32 - 12.0;
    let mut above = allowed[0] as f32 + 12.0;
    for &a in &allowed {
        let a = a as f32;
        if frac > a {
            below = a;
        } else {
            above = a;
            break;
        }
    }
    let q = if frac < (below + above) * 0.5 { below } else { above };
    (root + octave * 12.0 + q).round() as i32
}

// ---------------------------------------------------------------- runtime

/// What Marbles shows: per T the last ticks, and the X values.
#[derive(Clone, Debug)]
pub struct MarblesState {
    pub t: TGenerator,
    x_seq: [RandomSequence; 3],
    /// The X values (0..1) and the notes they became.
    pub x: [f32; 3],
    pub notes: [u8; 3],
    gates: [bool; 3],
    phases: [f32; 3],
    /// What each T plays now (to end it at the end of its gate).
    sounding: [Option<u8>; 3],
    /// The song's position in ticks, the jittered clock's, and the tick to come.
    straight: f64,
    jittered: f64,
    next_tick: i64,
    next_beat: f64,
    synced: bool,
    rate: f64,
    /// A bit per tick (newest lowest) for T1, T2 and T3.
    pub history: [u16; 3],
    fired: [bool; 3],
    ticks: u64,
    rng: Rng,
}

impl Default for MarblesState {
    fn default() -> Self {
        let mut rng = Rng::new(0x3A7B_1E55);
        let t = TGenerator::new(&mut rng);
        let x_seq = std::array::from_fn(|_| RandomSequence::new(&mut rng));
        Self {
            t,
            x_seq,
            x: [0.5; 3],
            notes: [60; 3],
            gates: [false; 3],
            phases: [0.0; 3],
            sounding: [None; 3],
            straight: 0.0,
            jittered: 0.0,
            next_tick: 0,
            next_beat: f64::NEG_INFINITY,
            synced: false,
            rate: 0.25,
            history: [0; 3],
            fired: [false; 3],
            ticks: 0,
            rng,
        }
    }
}

impl MarblesState {
    pub fn restart(&mut self) {
        self.synced = false;
        self.sounding = [None; 3];
        self.gates = [false; 3];
    }
}

/// The déjà vu knob with a dead band at noon (a locked loop).
pub fn deja_vu_amount(knob: f32) -> f32 {
    if knob < 0.47 {
        knob * 1.063_829_8
    } else if knob > 0.53 {
        0.5 + (knob - 0.53) * 1.063_829_8
    } else {
        0.5
    }
}

impl SeqState {
    pub(crate) fn marbles(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let shift = self.shift;
        let m = &mut *self.marbles;
        let rate = super::rate_beats(p[RATE]);
        // Settings.
        m.t.model = (p[MODEL].round().max(0.0) as usize).min(MODELS.len() - 1);
        m.t.bias = p[BIAS].clamp(0.0, 1.0);
        m.t.jitter = p[JITTER].clamp(0.0, 1.0);
        m.t.pulse_width_mean = p[GATE].clamp(0.0, 1.0);
        m.t.pulse_width_std = p[GATE_RANDOM].clamp(0.0, 1.0);
        let dv = deja_vu_amount(p[DEJA_VU].clamp(0.0, 1.0));
        let loop_dv = |mode: f32| match mode.round() as i32 {
            0 => 0.0,
            2 => 0.5,
            _ => dv,
        };
        let length = p[LENGTH].round().clamp(1.0, 16.0) as usize;
        m.t.sequence.deja_vu = loop_dv(p[T_LOOP]);
        m.t.sequence.set_length(length);
        let x_dv = loop_dv(p[X_LOOP]);
        for s in m.x_seq.iter_mut() {
            s.deja_vu = x_dv;
            s.set_length(length);
        }
        let control = p[CONTROL].round() as i32;
        let amount = |i: usize| match control {
            1 => {
                if i == 1 {
                    1.0
                } else {
                    -1.0
                }
            }
            2 => i as f32 - 1.0,
            _ => 1.0,
        };
        let spread = p[SPREAD].clamp(0.0, 1.0);
        let x_bias = p[X_BIAS].clamp(0.0, 1.0);
        let steps = p[STEPS].clamp(0.0, 1.0);
        let range = p[RANGE].round().clamp(1.0, 4.0);
        let base = p[BASE].round();
        let notes_mode = p[PLAY] >= 0.5;
        let vel = p[VELOCITY].clamp(0.05, 1.0);
        let accents = p[ACCENTS].clamp(0.0, 1.0);

        // Keep time with the song: start over after a jump or a new rate.
        if !m.synced || (ctx.beat - m.next_beat).abs() > 1e-3 || (rate - m.rate).abs() > 1e-9 {
            m.synced = true;
            m.rate = rate;
            m.straight = ctx.beat / rate;
            m.jittered = m.straight;
            m.next_tick = (m.straight - 1e-6).ceil() as i64;
            m.t.jitter_multiplier = 1.0;
            let rng = &mut m.rng;
            m.t.reset(rng);
            for s in m.x_seq.iter_mut() {
                s.reset();
            }
            m.phases = [1.0; 3];
        }
        m.next_beat = ctx.beat + ctx.len as f64 / ctx.samples_per_beat;
        m.straight = ctx.beat / rate;
        if m.t.jitter <= 0.0 {
            m.jittered = m.straight;
        }
        let freq = 1.0 / (rate * ctx.samples_per_beat);
        for i in 0..ctx.len {
            if m.jittered >= m.next_tick as f64 - 1e-9 {
                m.next_tick += 1;
                let difference = (m.straight - m.jittered) as f32;
                let rng = &mut m.rng;
                m.t.tick(rng, difference);
                m.ticks += 1;
                for (h, f) in m.history.iter_mut().zip(m.fired.iter_mut()) {
                    *h = *h << 1 | *f as u16;
                    *f = false;
                }
            }
            let master_phase = (m.jittered - (m.next_tick - 1) as f64).clamp(0.0, 0.999_9) as f32;
            let f = (freq * m.t.jitter_multiplier as f64) as f32;
            let (p1, g1) = m.t.slaves[0].process(f);
            let (p3, g3) = m.t.slaves[1].process(f);
            let phases = [p1, master_phase, p3];
            let gates = [g1, master_phase < 0.5, g3];
            for c in 0..3 {
                // A new X value when its T's ramp starts again.
                if phases[c] < m.phases[c] {
                    let a = amount(c);
                    let u = m.x_seq[c].next_value(&mut m.rng);
                    let s = 0.5 + (spread - 0.5) * a;
                    let b = 0.5 + (x_bias - 0.5) * a;
                    let st = 0.5 + (steps - 0.5) * a;
                    m.x[c] = x_value(u, s, b);
                    let pitch = base + (m.x[c] - 0.5) * range * 12.0;
                    m.notes[c] = quantize(pitch, st, &ctx.scale, ctx.scale_on).clamp(0, 127) as u8;
                }
                m.phases[c] = phases[c];
                let on = p.get(T_ON + c).is_none_or(|v| *v >= 0.5);
                if gates[c] && !m.gates[c] {
                    m.fired[c] = true;
                    if on && ctx.playing {
                        let note = if notes_mode {
                            shifted_by(shift, ctx, m.notes[c] as i32)
                        } else {
                            p[T_NOTE + c].round().clamp(0.0, 127.0) as u8
                        };
                        let v = vel * (1.0 - accents + accents * (0.3 + 0.7 * m.x[c]));
                        if let Some(n) = m.sounding[c].take() {
                            out.push(MidiEvent::off(i as u32, n));
                        }
                        for o in m.sounding.iter_mut() {
                            if *o == Some(note) {
                                *o = None;
                                out.push(MidiEvent::off(i as u32, note));
                            }
                        }
                        out.push(MidiEvent::on(i as u32, note, v.clamp(0.01, 1.0)));
                        m.sounding[c] = Some(note);
                    }
                } else if !gates[c] && m.gates[c] {
                    if let Some(n) = m.sounding[c].take() {
                        out.push(MidiEvent::off(i as u32, n));
                    }
                }
                m.gates[c] = gates[c];
            }
            m.straight += freq;
            m.jittered += freq * m.t.jitter_multiplier as f64;
        }
        // What the screen shows.
        let fired_now = m.fired.iter().enumerate().fold(0u64, |a, (i, f)| a | (*f as u64) << i);
        self.visual[0] = m.history[0] as u64 | (m.history[1] as u64) << 16 | (m.history[2] as u64) << 32 | (m.t.sequence.step() as u64 & 0xff) << 48 | (m.ticks & 0xff) << 56;
        self.visual[1] = m.x.iter().enumerate().fold(0u64, |a, (i, x)| a | (((x.clamp(0.0, 1.0) * 65535.0) as u64) << (16 * i))) | ((m.jittered.fract() * 65535.0) as u64) << 48;
        self.visual[2] = m.notes[0] as u64 | (m.notes[1] as u64) << 8 | (m.notes[2] as u64) << 16 | fired_now << 24 | (m.gates.iter().enumerate().fold(0u64, |a, (i, g)| a | (*g as u64) << i)) << 28;
    }
}

/// `pitch` moved by the held key (Transpose), as `SeqState::shifted` does.
fn shifted_by(shift: Option<(i32, i32)>, ctx: &MidiCtx, pitch: i32) -> u8 {
    let p = match shift {
        None => pitch,
        Some((key, reference)) if ctx.scale_on => {
            let (d, rest) = ctx.scale.locate(pitch);
            let (dk, _) = ctx.scale.locate(key);
            let (dr, _) = ctx.scale.locate(reference);
            ctx.scale.pitch_at(d + dk - dr) + rest
        }
        Some((key, reference)) => pitch + key - reference,
    };
    p.clamp(0, 127) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::MidiFxKind;
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};

    fn ctx(beat: f64, len: usize) -> MidiCtx {
        MidiCtx { now: (beat * 24_000.0) as u64, len, samples_per_beat: 24_000.0, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat, playing: true, sr: 48_000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 2] }
    }

    /// Runs `bars` bars and returns (frame, pitch, velocity) of every note.
    fn run(p: &[f32], bars: usize) -> Vec<(u64, u8, f32)> {
        let mut s = SeqState::new();
        let mut notes = Vec::new();
        let block = 512;
        let total = bars * 4 * 24_000;
        let mut frame = 0;
        while frame < total {
            let mut out = Vec::new();
            let c = ctx(frame as f64 / 24_000.0, block);
            s.process(MidiFxKind::Marbles, p, &[], &mut out, &c);
            for e in out.iter().filter(|e| e.on) {
                notes.push((frame as u64 + e.offset as u64, e.pitch, e.vel));
            }
            frame += block;
        }
        notes
    }

    #[test]
    fn the_tables_are_inverse_cdfs() {
        let t = tables();
        assert_eq!(t.len(), 60 * TABLE);
        // Every table rises from 0 to 1.
        for c in 0..60 {
            let body = &t[c * TABLE..c * TABLE + 129];
            assert!(body[0].abs() < 1e-3 && (body[128] - 1.0).abs() < 1e-3, "{c}");
            assert!(body.windows(2).all(|w| w[1] >= w[0] - 1e-6), "{c}");
        }
        // Bias moves the middle, spread widens it.
        assert!(beta_sample(0.5, 0.5, 0.2) < 0.4);
        assert!(beta_sample(0.5, 0.5, 0.8) > 0.6);
        assert!((beta_sample(0.5, 0.5, 0.5) - 0.5).abs() < 0.02);
        assert!(beta_sample(0.9, 0.1, 0.5) - 0.5 < beta_sample(0.9, 0.8, 0.5) - 0.5);
        assert!((x_value(0.9, 0.0, 0.3) - 0.3).abs() < 1e-4);
    }

    #[test]
    fn t2_is_the_clock_and_t1_and_t3_take_turns() {
        let p = defaults(SPECS);
        let notes = run(&p, 2);
        let clock: Vec<u64> = notes.iter().filter(|n| n.1 == 42).map(|n| n.0).collect();
        // Sixteenths for two bars, right on time.
        assert_eq!(clock.len(), 32);
        assert!(clock.iter().enumerate().all(|(i, &f)| f == i as u64 * 6_000));
        // The coin toss: every tick one of T1 and T3, never both.
        let kicks = notes.iter().filter(|n| n.1 == 36).count();
        let snares = notes.iter().filter(|n| n.1 == 38).count();
        assert_eq!(kicks + snares, 32);
        assert!(kicks > 4 && snares > 4, "{kicks} {snares}");
    }

    #[test]
    fn bias_leans_and_the_drums_model_plays_grooves() {
        let mut p = defaults(SPECS);
        p[DEJA_VU] = 0.0;
        let kicks = |p: &[f32]| run(p, 4).iter().filter(|n| n.1 == 36).count();
        p[BIAS] = 0.1;
        let many = kicks(&p);
        p[BIAS] = 0.9;
        let few = kicks(&p);
        assert!(many > 48 && few < 16, "{many} {few}");
        p[DEJA_VU] = 0.35;
        p[MODEL] = 2.0;
        p[BIAS] = 0.5;
        let notes = run(&p, 2);
        // The plainest pattern: T1 on the first and T3 on the fifth of eight.
        let at = |pitch: u8| notes.iter().filter(|n| n.1 == pitch).map(|n| n.0 / 6_000).collect::<Vec<_>>();
        assert_eq!(at(36), vec![0, 8, 16, 24]);
        assert_eq!(at(38), vec![4, 12, 20, 28]);
    }

    #[test]
    fn a_locked_loop_repeats() {
        let mut p = defaults(SPECS);
        p[DEJA_VU] = 0.5;
        p[LENGTH] = 4.0;
        let notes = run(&p, 4);
        let seq: Vec<u8> = notes.iter().filter(|n| n.1 != 42).map(|n| n.1).collect();
        // After the first loop, every four ticks repeat.
        assert!(seq.len() >= 40);
        for i in 8..seq.len() {
            assert_eq!(seq[i], seq[i - 4], "{i}");
        }
    }

    #[test]
    fn notes_stay_in_the_scale_and_steps_thin_it() {
        let mut p = defaults(SPECS);
        p[PLAY] = 1.0;
        p[SPREAD] = 0.8;
        let notes = run(&p, 4);
        let scale = Scale::new(0, ScaleKind::Major);
        assert!(notes.len() > 40);
        assert!(notes.iter().all(|n| scale.contains(n.1)));
        let distinct = |ns: &[(u64, u8, f32)]| {
            let mut v: Vec<u8> = ns.iter().map(|n| n.1 % 12).collect();
            v.sort();
            v.dedup();
            v.len()
        };
        assert!(distinct(&notes) >= 5);
        p[STEPS] = 1.0;
        let roots = run(&p, 4);
        assert!(roots.iter().all(|n| n.1 % 12 == 0));
        p[STEPS] = 5.0 / 6.0;
        let fifths = run(&p, 4);
        assert!(fifths.iter().all(|n| n.1 % 12 == 0 || n.1 % 12 == 7));
    }

    #[test]
    fn jitter_loosens_but_does_not_drift() {
        let mut p = defaults(SPECS);
        p[JITTER] = 0.8;
        let notes = run(&p, 8);
        let clock: Vec<u64> = notes.iter().filter(|n| n.1 == 42).map(|n| n.0).collect();
        let off_grid = clock.iter().filter(|&&f| f % 6_000 != 0).count();
        assert!(off_grid > clock.len() / 2);
        // Still about one tick per sixteenth after eight bars.
        assert!((clock.len() as i64 - 128).abs() <= 4, "{}", clock.len());
    }

    #[test]
    fn dividers_and_the_others_play() {
        for model in 0..MODELS.len() {
            let mut p = defaults(SPECS);
            p[MODEL] = model as f32;
            p[BIAS] = 0.8;
            let notes = run(&p, 2);
            assert!(notes.iter().any(|n| n.1 == 36 || n.1 == 38), "{}", MODELS[model]);
        }
    }
}
