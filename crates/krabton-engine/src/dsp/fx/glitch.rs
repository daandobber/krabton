//! Glitch Sequencer: a step sequencer of glitch effects. Every step of the
//! pattern (a 1/8, 1/16 or 1/32 note) picks a module that takes hold of the
//! audio for that step: Retrigger (the start of the step repeated, each
//! repeat pitched and fading), Reverse (the step before, backwards),
//! Shuffle (an earlier piece of the music), Tape Stop (the tape slowing to a
//! halt, or starting up), Stretch (played at half or a quarter speed, or
//! doubled), Crush (fewer bits, a lower rate), Gate (chopped in a rhythm),
//! Filter (a resonant sweep over the step) and Delay (the step sent into an
//! echo that rings on). Four slots hold any other effect, sequenced the
//! same way: they hear the music only during their steps, so a reverb's
//! tail rings on after it. Each step has its own chance; Variation shakes
//! the modules' settings from step to step. Four patterns, A to D.
//!
//! Everything reads from a few seconds of what came in, so the glitches
//! follow the song; edges are faded so nothing clicks. Original code.

use super::super::effects::{EffectState, FxCtx};
use crate::dsp::{db_to_gain, Svf};
use crate::params::{ParamSpec, Unit};
use crate::project::Device;
use crate::scope::{DeviceTaps, MeterTap, ScopeTap};

pub const STEP: usize = 0;
pub const LENGTH: usize = 1;
pub const PATTERN: usize = 2;
pub const VARIATION: usize = 3;
pub const CHANCE: usize = 4;
pub const MIX: usize = 5;
pub const OUTPUT: usize = 6;
pub const REPEAT: usize = 7;
pub const PITCH: usize = 8;
pub const DECAY: usize = 9;
pub const FADE: usize = 10;
pub const RANGE: usize = 11;
pub const CURVE: usize = 12;
pub const TAPE: usize = 13;
pub const SPEED: usize = 14;
pub const BITS: usize = 15;
pub const DOWNSAMPLE: usize = 16;
pub const GATE_RATE: usize = 17;
pub const DUTY: usize = 18;
pub const FILTER: usize = 19;
pub const FROM: usize = 20;
pub const TO: usize = 21;
pub const RESONANCE: usize = 22;
pub const TIME: usize = 23;
pub const FEEDBACK: usize = 24;

pub const STEP_SIZES: &[&str] = &["1/8", "1/16", "1/32"];
const STEP_BEATS: [f64; 3] = [0.5, 0.25, 0.125];
pub const PATTERN_NAMES: &[&str] = &["A", "B", "C", "D"];
pub const REPEATS: &[&str] = &["1/4", "1/8", "1/16", "1/32", "1/64", "1/128"];
const REPEAT_BEATS: [f64; 6] = [1.0, 0.5, 0.25, 0.125, 0.0625, 0.03125];
pub const RANGES: &[&str] = &["1 Beat", "2 Beats", "1 Bar", "2 Bars"];
const RANGE_BEATS: [f64; 4] = [1.0, 2.0, 4.0, 8.0];
pub const TAPE_MODES: &[&str] = &["Stop", "Start", "Stop & Start"];
pub const SPEEDS: &[&str] = &["1/4", "1/2", "2x", "4x"];
const SPEED_VALUES: [f64; 4] = [0.25, 0.5, 2.0, 4.0];
pub const GATE_RATES: &[&str] = &["1/16", "1/32", "1/64", "1/128"];
const GATE_BEATS: [f64; 4] = [0.25, 0.125, 0.0625, 0.03125];
pub const FILTERS: &[&str] = &["Low Pass", "High Pass", "Band Pass"];
pub const TIMES: &[&str] = &["1/16", "1/8", "3/16", "1/4", "3/8"];
const TIME_BEATS: [f64; 5] = [0.25, 0.5, 0.75, 1.0, 1.5];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Step", "Sequencer", STEP_SIZES, 1),
    ParamSpec::int("Length", "Sequencer", 1.0, 32.0, 16.0, Unit::None),
    ParamSpec::choice("Pattern", "Sequencer", PATTERN_NAMES, 0),
    ParamSpec::lin("Variation", "Sequencer", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Chance", "Sequencer", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Output", "Output", -24.0, 12.0, 0.0, Unit::Db),
    ParamSpec::choice("Repeat", "Retrigger", REPEATS, 3),
    ParamSpec::int("Pitch", "Retrigger", -12.0, 12.0, 0.0, Unit::None),
    ParamSpec::lin("Decay", "Retrigger", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Fade", "Reverse", 1.0, 50.0, 6.0, Unit::Ms),
    ParamSpec::choice("Range", "Shuffle", RANGES, 2),
    ParamSpec::lin("Curve", "Tape Stop", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::choice("Mode", "Tape Stop", TAPE_MODES, 0),
    ParamSpec::choice("Speed", "Stretch", SPEEDS, 1),
    ParamSpec::int("Bits", "Crush", 1.0, 16.0, 6.0, Unit::None),
    ParamSpec::int("Downsample", "Crush", 1.0, 40.0, 6.0, Unit::None),
    ParamSpec::choice("Rate", "Gate", GATE_RATES, 1),
    ParamSpec::lin("Duty", "Gate", 0.05, 0.95, 0.5, Unit::Percent),
    ParamSpec::choice("Type", "Filter", FILTERS, 0),
    ParamSpec::log("From", "Filter", 40.0, 18000.0, 9000.0, Unit::Hz),
    ParamSpec::log("To", "Filter", 40.0, 18000.0, 300.0, Unit::Hz),
    ParamSpec::lin("Resonance", "Filter", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::choice("Time", "Delay", TIMES, 2),
    ParamSpec::lin("Feedback", "Delay", 0.0, 0.95, 0.55, Unit::Percent),
];

/// The modules a step can pick (0 is "nothing": the music as it is).
pub const MODULES: &[&str] = &["Retrigger", "Reverse", "Shuffle", "Tape Stop", "Stretch", "Crush", "Gate", "Filter", "Delay", "Slot 1", "Slot 2", "Slot 3", "Slot 4"];
pub const BUILT_IN: usize = 9;
pub const SLOTS: usize = 4;
pub const STEPS: usize = 32;
pub const PATTERNS: usize = 4;
/// Per step: the module (0 = none, 1..=13) and its chance (0..=100).
pub const DATA_BYTES: usize = PATTERNS * STEPS * 2;

/// The first slot's module number.
pub const FIRST_SLOT: u8 = BUILT_IN as u8 + 1;

pub fn step_at(data: &[u8], pattern: usize, i: usize) -> (u8, u8) {
    let o = (pattern.min(PATTERNS - 1) * STEPS + i % STEPS) * 2;
    match data.get(o..o + 2) {
        Some(s) => (s[0].min(MODULES.len() as u8), s[1].min(100)),
        None => (0, 100),
    }
}

pub fn set_step(data: &mut Vec<u8>, pattern: usize, i: usize, module: u8, chance: u8) {
    if data.len() < DATA_BYTES {
        data.resize(DATA_BYTES, 0);
    }
    let o = (pattern.min(PATTERNS - 1) * STEPS + i % STEPS) * 2;
    data[o] = module.min(MODULES.len() as u8);
    data[o + 1] = chance.min(100);
}

pub fn default_data() -> Vec<u8> {
    let mut d = vec![0u8; DATA_BYTES];
    let patterns: [[u8; 16]; 3] = [
        [0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 2, 0, 7, 7, 4, 0],
        [6, 0, 6, 0, 3, 0, 0, 8, 6, 0, 6, 0, 5, 5, 1, 1],
        [1, 0, 0, 3, 0, 2, 0, 0, 9, 0, 0, 1, 0, 4, 0, 7],
    ];
    for (p, steps) in patterns.iter().enumerate() {
        for (i, &m) in steps.iter().enumerate() {
            set_step(&mut d, p, i, m, if p == 2 && m != 0 { 60 } else { 100 });
        }
    }
    d
}

/// Slot `s` was taken out: its steps play nothing, the slots after it move up.
pub fn remove_slot(data: &mut [u8], s: usize) {
    let gone = FIRST_SLOT + s as u8;
    for step in data.chunks_exact_mut(2) {
        if step[0] == gone {
            step[0] = 0;
        } else if step[0] > gone {
            step[0] -= 1;
        }
    }
}

/// A new random pattern: `density` of the steps get a built-in module (or
/// a filled slot), often repeated so they stutter in pairs.
pub fn random_pattern(data: &mut Vec<u8>, pattern: usize, length: usize, density: f32, slots_used: usize, mut seed: u32) {
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed >> 8) as f32 / 16_777_216.0
    };
    let choices = BUILT_IN + slots_used;
    let mut last = 0u8;
    for i in 0..STEPS {
        let m = if i < length && rnd() < density {
            if last != 0 && rnd() < 0.4 {
                last
            } else {
                1 + (rnd() * choices as f32) as u8 % choices as u8
            }
        } else {
            0
        };
        let chance = if m != 0 && rnd() < 0.25 { 50 + (rnd() * 40.0) as u8 } else { 100 };
        set_step(data, pattern, i, m, chance);
        last = m;
    }
}

/// The settings of a module for one step (Variation shakes them).
#[derive(Clone, Copy, Default)]
struct Vars {
    repeat: usize,
    pitch: f32,
    decay: f32,
    fade: usize,
    offset: usize,
    curve: f32,
    tape: u8,
    speed: f64,
    bits: f32,
    down: u32,
    gate: usize,
    duty: f32,
    filter: u8,
    from: f32,
    to: f32,
    res: f32,
}

/// The buffer: about 22 s at 48 kHz.
const BUF: usize = 1 << 20;
const DELAY_BUF: usize = 1 << 18;
const CHUNK: usize = 256;

pub struct Glitch {
    sr: f32,
    buf: [Vec<f32>; 2],
    w: usize,
    step_k: i64,
    module: u8,
    step_start: usize,
    step_len: usize,
    pos: usize,
    v: Vars,
    tape_p: f64,
    hold: [f32; 2],
    hold_n: u32,
    svf: [Svf; 2],
    delay: [Vec<f32>; 2],
    dw: usize,
    gate_env: f32,
    free_beat: f64,
    rng: u32,
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
            module: 0,
            step_start: BUF,
            step_len: 1,
            pos: 0,
            v: Vars::default(),
            tape_p: 0.0,
            hold: [0.0; 2],
            hold_n: 0,
            svf: [Svf::new(), Svf::new()],
            delay: [vec![0.0; DELAY_BUF], vec![0.0; DELAY_BUF]],
            dw: 0,
            gate_env: 1.0,
            free_beat: 0.0,
            rng: 0x6C17_C4ED,
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

    #[inline]
    fn read(&self, ch: usize, pos: f64) -> f32 {
        let i = pos.floor();
        let f = (pos - i) as f32;
        let i = i as i64;
        let a = self.buf[ch][i.rem_euclid(BUF as i64) as usize];
        let b = self.buf[ch][(i + 1).rem_euclid(BUF as i64) as usize];
        a + (b - a) * f
    }

    /// A new step begins: which module plays it, and with what settings.
    fn begin_step(&mut self, k: i64, p: &[f32], data: &[u8], spb: f64) {
        self.step_k = k;
        let length = p[LENGTH].round().clamp(1.0, STEPS as f32) as i64;
        let pattern = (p[PATTERN].round().max(0.0) as usize).min(PATTERNS - 1);
        let (m, chance) = step_at(data, pattern, k.rem_euclid(length) as usize);
        let fire = m != 0 && self.unit() < chance as f32 / 100.0 * p[CHANCE].clamp(0.0, 1.0);
        self.module = if fire { m } else { 0 };
        let step_beats = STEP_BEATS[(p[STEP].round().max(0.0) as usize).min(2)];
        self.step_len = ((step_beats * spb) as usize).max(16);
        self.step_start = self.w;
        self.pos = 0;
        self.tape_p = 0.0;
        // The settings, shaken by Variation.
        let vary = self.unit() < p[VARIATION].clamp(0.0, 1.0);
        let pick = |s: &mut Self, base: usize, n: usize| if vary { (s.unit() * n as f32) as usize % n } else { base.min(n - 1) };
        let ri = pick(self, p[REPEAT].round().max(0.0) as usize, REPEATS.len());
        let ri = if vary { ri.max(1) } else { ri };
        self.v.repeat = ((REPEAT_BEATS[ri] * spb) as usize).clamp(16, self.step_len.max(16));
        self.v.pitch = if vary { [-12.0, -7.0, -5.0, 0.0, 0.0, 5.0, 7.0, 12.0][(self.unit() * 8.0) as usize % 8] } else { p[PITCH].round() };
        self.v.decay = if vary { self.unit() * 0.6 } else { p[DECAY].clamp(0.0, 1.0) };
        self.v.fade = ((p[FADE].clamp(1.0, 50.0) * 0.001 * self.sr) as usize).clamp(8, self.step_len / 2);
        let range = (RANGE_BEATS[(p[RANGE].round().max(0.0) as usize).min(3)] * spb) as usize;
        let steps_back = (range / self.step_len).max(1);
        self.v.offset = self.step_len * (1 + (self.unit() * steps_back as f32) as usize % steps_back);
        self.v.curve = if vary { self.unit() } else { p[CURVE].clamp(0.0, 1.0) };
        self.v.tape = if vary { (self.unit() * 3.0) as u8 % 3 } else { (p[TAPE].round().max(0.0) as u8).min(2) };
        self.v.speed = SPEED_VALUES[pick(self, p[SPEED].round().max(0.0) as usize, SPEEDS.len())];
        self.v.bits = if vary { 2.0 + (self.unit() * 8.0).floor() } else { p[BITS].round().clamp(1.0, 16.0) };
        self.v.down = if vary { 1 + (self.unit() * 20.0) as u32 } else { p[DOWNSAMPLE].round().clamp(1.0, 40.0) as u32 };
        self.v.gate = ((GATE_BEATS[pick(self, p[GATE_RATE].round().max(0.0) as usize, GATE_RATES.len())] * spb) as usize).max(8);
        self.v.duty = if vary { 0.2 + 0.6 * self.unit() } else { p[DUTY].clamp(0.05, 0.95) };
        self.v.filter = (p[FILTER].round().max(0.0) as u8).min(2);
        let (from, to) = if vary {
            let a = 80.0 * 200f32.powf(self.unit());
            let b = 80.0 * 200f32.powf(self.unit());
            (a, b)
        } else {
            (p[FROM].clamp(20.0, 20000.0), p[TO].clamp(20.0, 20000.0))
        };
        self.v.from = from;
        self.v.to = to;
        self.v.res = p[RESONANCE].clamp(0.0, 1.0);
    }

    /// How much of the effect plays at `pos` of a step of `len`: faded in and
    /// out over `f` samples so nothing clicks.
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
            m.values[2] = self.pos as f32 / self.step_len.max(1) as f32;
            m.values[3] = 1.0;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn chunk(&mut self, p: &[f32], children: &[Device], data: &[u8], l: &mut [f32], r: &mut [f32], ctx: &FxCtx, offset: usize) {
        let n = l.len();
        let sr = ctx.sr.max(1000.0) as f64;
        let bpm = ctx.bpm.max(20.0) as f64;
        let spb = sr * 60.0 / bpm;
        let bps = 1.0 / spb;
        let step_beats = STEP_BEATS[(p[STEP].round().max(0.0) as usize).min(2)];
        let mix = p[MIX].clamp(0.0, 1.0);
        let out_g = db_to_gain(p[OUTPUT]);
        let fb = p[FEEDBACK].clamp(0.0, 0.95);
        let dtime = ((TIME_BEATS[(p[TIME].round().max(0.0) as usize).min(TIMES.len() - 1)] * spb) as usize).clamp(1, DELAY_BUF - 1);
        let gate_k = 1.0 - (-1.0 / (0.0015 * self.sr)).exp();
        let mut any_slot = false;
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
            let (pos, len, start) = (self.pos, self.step_len, self.step_start as f64);
            let edge = Self::edge(pos, len, self.v.fade);
            let mut out = input;
            let mut weights = [0.0f32; SLOTS];
            match self.module {
                1 => {
                    // Retrigger: the start of the step again and again.
                    let seg = self.v.repeat.min(len);
                    let rep = pos / seg;
                    let j = pos % seg;
                    let rate = 2f64.powf((self.v.pitch * rep as f32 / 12.0) as f64);
                    let at = start + ((j as f64 * rate) % seg as f64);
                    let g = (1.0 - self.v.decay).powi(rep as i32) * if rep > 0 { Self::edge(j, seg, 48) } else { 1.0 };
                    for ch in 0..2 {
                        let y = self.read(ch, at) * g;
                        out[ch] = input[ch] + (y - input[ch]) * Self::edge(pos, len, 48).max(if rep == 0 { 1.0 } else { 0.0 });
                    }
                }
                2 | 3 | 5 => {
                    let at = match self.module {
                        // Reverse: the step before, backwards.
                        2 => start - 1.0 - pos as f64,
                        // Shuffle: an earlier piece.
                        3 => start - self.v.offset as f64 + pos as f64,
                        // Stretch: slower (behind) or faster (from further back).
                        _ => {
                            let s = self.v.speed;
                            if s < 1.0 {
                                start + pos as f64 * s
                            } else {
                                start - len as f64 * (s - 1.0) + pos as f64 * s
                            }
                        }
                    };
                    for ch in 0..2 {
                        let y = self.read(ch, at);
                        out[ch] = input[ch] + (y - input[ch]) * edge;
                    }
                }
                4 => {
                    // Tape Stop: the speed falls to nothing (or rises from it).
                    let t = pos as f32 / len as f32;
                    let e = 0.4 + 2.0 * self.v.curve;
                    let rate = match self.v.tape {
                        0 => (1.0 - t).max(0.0).powf(e),
                        1 => t.powf(e),
                        _ => {
                            if t < 0.5 {
                                (1.0 - 2.0 * t).powf(e)
                            } else {
                                (2.0 * t - 1.0).powf(e)
                            }
                        }
                    };
                    self.tape_p += rate as f64;
                    let at = start + self.tape_p;
                    let fade = Self::edge(pos, len, 96);
                    for ch in 0..2 {
                        let y = self.read(ch, at);
                        out[ch] = input[ch] + (y - input[ch]) * fade;
                    }
                }
                6 => {
                    // Crush: hold every few samples, fewer steps of level.
                    if self.hold_n == 0 {
                        self.hold = input;
                    }
                    self.hold_n = (self.hold_n + 1) % self.v.down.max(1);
                    let q = 2f32.powf(self.v.bits - 1.0);
                    for ch in 0..2 {
                        out[ch] = (self.hold[ch] * q).round() / q;
                    }
                }
                7 => {
                    // Gate: open for Duty of every beat of the rate.
                    let open = (pos % self.v.gate) as f32 / (self.v.gate as f32) < self.v.duty;
                    let target = if open { 1.0 } else { 0.0 };
                    self.gate_env += (target - self.gate_env) * gate_k;
                    for ch in 0..2 {
                        out[ch] = input[ch] * self.gate_env;
                    }
                }
                8 => {
                    // Filter: a sweep from From to To over the step.
                    if pos % 16 == 0 {
                        let t = pos as f32 / len as f32;
                        let fc = self.v.from * (self.v.to / self.v.from).powf(t);
                        for f in self.svf.iter_mut() {
                            f.set(fc, self.v.res * 0.95, self.sr);
                        }
                    }
                    for ch in 0..2 {
                        let o = self.svf[ch].tick(input[ch]);
                        let y = match self.v.filter {
                            1 => o.high,
                            2 => o.band,
                            _ => o.low,
                        };
                        out[ch] = input[ch] + (y - input[ch]) * Self::edge(pos, len, 48);
                    }
                }
                m if m >= FIRST_SLOT => {
                    let s = (m - FIRST_SLOT) as usize;
                    if s < SLOTS {
                        weights[s] = Self::edge(pos, len, 96);
                        any_slot = true;
                    }
                }
                _ => {}
            }
            if self.module != 7 {
                self.gate_env += (1.0 - self.gate_env) * gate_k;
            }
            if self.module != 6 {
                self.hold_n = 0;
            }
            // The delay rings on; it hears only its steps.
            let send = if self.module == 9 { edge } else { 0.0 };
            let rd = (self.dw + DELAY_BUF - dtime) & (DELAY_BUF - 1);
            for ch in 0..2 {
                let d = self.delay[ch][rd];
                self.delay[ch][self.dw] = input[ch] * send + d * fb;
                out[ch] += d;
            }
            self.dw = (self.dw + 1) & (DELAY_BUF - 1);
            l[i] = out[0];
            r[i] = out[1];
            self.weights[i] = weights;
            self.w = self.w.wrapping_add(1);
            self.pos += 1;
        }
        // The slots: each hears the music during its steps and adds what it
        // makes; the music itself gives way while a slot has the step.
        let _ = any_slot;
        for s in 0..SLOTS.min(children.len()).min(self.children.len()) {
            let dev = &children[s];
            if !dev.enabled {
                continue;
            }
            for i in 0..n {
                let w = self.weights[i][s];
                let dry = [self.buf[0][(self.w.wrapping_sub(n - i)) & (BUF - 1)], self.buf[1][(self.w.wrapping_sub(n - i)) & (BUF - 1)]];
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

    /// Runs `secs` of `input` through a pattern of one module on every step.
    fn run(module: u8, p: &mut Vec<f32>, input: &[f32]) -> Vec<f32> {
        let mut g = Glitch::new(SR);
        let mut data = vec![0u8; DATA_BYTES];
        for i in 0..STEPS {
            set_step(&mut data, 0, i, module, 100);
        }
        p[VARIATION] = 0.0;
        let mut out = Vec::new();
        let spb = SR as f64 / 2.0;
        for (b, chunk) in input.chunks(256).enumerate() {
            let mut l = chunk.to_vec();
            let mut r = chunk.to_vec();
            g.process(p, &[], &data, &mut l, &mut r, &ctx(b as f64 * 256.0 / spb));
            assert!(l.iter().all(|v| v.is_finite() && v.abs() < 4.0));
            out.extend_from_slice(&l);
        }
        out
    }

    fn ramp(secs: f32) -> Vec<f32> {
        (0..(secs * SR) as usize).map(|i| ((i as f32 / SR) * 0.37).fract() - 0.5).collect()
    }

    #[test]
    fn nothing_set_is_the_music_itself() {
        let input = ramp(1.0);
        let mut p = crate::params::defaults(SPECS);
        let out = run(0, &mut p, &input);
        assert!(out.iter().zip(&input).all(|(a, b)| (a - b).abs() < 1e-6));
    }

    #[test]
    fn retrigger_repeats_the_start_of_the_step() {
        // A rising ramp: repeats bring it back down every 1/32 (3000 samples at 120 BPM).
        let input = ramp(1.0);
        let mut p = crate::params::defaults(SPECS);
        p[DECAY] = 0.0;
        let out = run(1, &mut p, &input);
        // Inside one 1/16 step (6000 samples): the second half equals the first.
        let s = 12000;
        for j in [100usize, 1000, 2500] {
            assert!((out[s + 3000 + j] - out[s + j]).abs() < 1e-3, "{j}: {} vs {}", out[s + 3000 + j], out[s + j]);
        }
    }

    #[test]
    fn reverse_plays_the_step_before_backwards() {
        let input = ramp(1.0);
        let mut p = crate::params::defaults(SPECS);
        let out = run(2, &mut p, &input);
        let s = 12000;
        // Mid-step: the input from just before the step, going back in time.
        let j = 3000;
        assert!((out[s + j] - input[s - 1 - j]).abs() < 1e-3);
    }

    #[test]
    fn tape_stop_slows_to_a_halt() {
        let input: Vec<f32> = (0..48000).map(|i| (std::f32::consts::TAU * 440.0 * i as f32 / SR).sin() * 0.5).collect();
        let mut p = crate::params::defaults(SPECS);
        p[STEP] = 0.0; // 1/8: 12000 samples
        let out = run(4, &mut p, &input);
        // Near the end of a step the tape barely moves: almost no change between samples.
        let s = 24000;
        let early: f32 = (s + 200..s + 400).map(|i| (out[i + 1] - out[i]).abs()).sum();
        let late: f32 = (s + 11000..s + 11200).map(|i| (out[i + 1] - out[i]).abs()).sum();
        assert!(late < 0.2 * early, "{early} {late}");
    }

    #[test]
    fn gate_chops_and_crush_quantizes() {
        let input = vec![0.5f32; 48000];
        let mut p = crate::params::defaults(SPECS);
        let gated = run(7, &mut p, &input);
        let lo = gated[12000..18000].iter().fold(1.0f32, |m, v| m.min(*v));
        assert!(lo < 0.05, "{lo}");
        let mut p = crate::params::defaults(SPECS);
        p[BITS] = 2.0;
        let input: Vec<f32> = (0..48000).map(|i| (i as f32 / 48000.0) - 0.5).collect();
        let crushed = run(6, &mut p, &input);
        let mut levels: Vec<i32> = crushed[12000..30000].iter().map(|v| (v * 1000.0).round() as i32).collect();
        levels.sort();
        levels.dedup();
        assert!(levels.len() <= 5, "{levels:?}");
    }

    #[test]
    fn delay_rings_on_after_its_step() {
        // A click on beat 0 with the delay on step 0 only: echoes follow.
        let mut input = vec![0.0f32; 96000];
        input[1000] = 1.0;
        let mut g = Glitch::new(SR);
        let mut data = vec![0u8; DATA_BYTES];
        set_step(&mut data, 0, 0, 9, 100);
        let mut p = crate::params::defaults(SPECS);
        p[VARIATION] = 0.0;
        p[LENGTH] = 32.0;
        let mut out = Vec::new();
        for (b, chunk) in input.chunks(256).enumerate() {
            let mut l = chunk.to_vec();
            let mut r = chunk.to_vec();
            g.process(&p, &[], &data, &mut l, &mut r, &ctx(b as f64 * 256.0 / 24000.0));
            out.extend_from_slice(&l);
        }
        // A 3/16 note at 120 BPM: 0.75 beats, 18000 samples.
        let echo = out[18900..19300].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        let second = out[36900..37300].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(echo > 0.3 && second > 0.1 && second < echo, "{echo} {second}");
    }

    #[test]
    fn a_slot_effect_hears_only_its_steps() {
        let input = vec![0.5f32; 48000];
        let mut g = Glitch::new(SR);
        // Slot 1: a Utility at -inf... use Utility with gain -60 dB: silence on its steps.
        let mut util = crate::params::defaults(super::super::super::effects::EffectKind::Utility.specs());
        util[0] = -60.0;
        let child = Device { id: 7, kind: super::super::super::effects::EffectKind::Utility, enabled: true, params: util, plugin: None, data: Vec::new(), children: Vec::new() };
        g.children.push((7, EffectState::new(child.kind, SR)));
        let mut data = vec![0u8; DATA_BYTES];
        set_step(&mut data, 0, 1, FIRST_SLOT, 100);
        let mut p = crate::params::defaults(SPECS);
        p[VARIATION] = 0.0;
        let mut out = Vec::new();
        for (b, chunk) in input.chunks(256).enumerate() {
            let mut l = chunk.to_vec();
            let mut r = chunk.to_vec();
            g.process(&p, std::slice::from_ref(&child), &data, &mut l, &mut r, &ctx(b as f64 * 256.0 / 24000.0));
            out.extend_from_slice(&l);
        }
        // Step 1 (samples 6000..12000) goes through the Utility: nearly silent mid-step.
        assert!(out[9000].abs() < 0.01, "{}", out[9000]);
        // Step 0 and step 2 are the music.
        assert!((out[3000] - 0.5).abs() < 1e-3 && (out[15000] - 0.5).abs() < 1e-3);
    }
}
