//! Guitar pedals: Overdrive, Distortion, Fuzz, Octaver, Squash (a
//! compressor), Wah, Tremolo, Vibe and Amp (preamp, tone stack, power amp
//! and a speaker cabinet). Written for Krabton.
//!
//! The clipping stages use first-order antiderivative anti-aliasing (Parker,
//! Zavalishin and Le Bivic, "Reducing the aliasing of nonlinear waveshaping
//! using continuous-time convolution", DAFx 2016): each output is the
//! average of the curve over the straight line between two input samples,
//! which takes most of the fold-back out of hard clipping without
//! oversampling.

use super::{Follower, Lfo, LFO_SYNC};
use crate::dsp::biquad::{self, Coefs, State};
use crate::dsp::effects::FxCtx;
use crate::dsp::{db_to_gain, DcBlock, OnePole, Svf};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::{PI, TAU};

/// Samples per chunk for the LFOs.
const CHUNK: usize = 128;

// ---------------------------------------------------------------- clipping

#[derive(Clone, Copy)]
enum Curve {
    /// tanh: smooth, like diodes in a feedback loop.
    Soft,
    /// Hard limits at ±1, like diodes to ground.
    Hard,
    /// tanh moved off centre (and back to zero at rest): even harmonics,
    /// like a transistor or tube that is not biased in the middle.
    Bias(f64),
}

fn ln_cosh(x: f64) -> f64 {
    let a = x.abs();
    a + (-2.0 * a).exp().ln_1p() - std::f64::consts::LN_2
}

fn curve(c: Curve, x: f64) -> f64 {
    match c {
        Curve::Soft => x.tanh(),
        Curve::Hard => x.clamp(-1.0, 1.0),
        Curve::Bias(b) => (x + b).tanh() - b.tanh(),
    }
}

/// The antiderivative of `curve`.
fn anti(c: Curve, x: f64) -> f64 {
    match c {
        Curve::Soft => ln_cosh(x),
        Curve::Hard => {
            if x.abs() <= 1.0 {
                0.5 * x * x
            } else {
                x.abs() - 0.5
            }
        }
        Curve::Bias(b) => ln_cosh(x + b) - b.tanh() * x,
    }
}

/// A clipper with antiderivative anti-aliasing (half a sample late).
#[derive(Clone, Copy, Default)]
struct Clip {
    x1: f64,
    f1: f64,
}

impl Clip {
    #[inline]
    fn run(&mut self, x: f32, c: Curve) -> f32 {
        let x = x as f64;
        let fx = anti(c, x);
        let d = x - self.x1;
        let y = if d.abs() < 1e-6 { curve(c, 0.5 * (x + self.x1)) } else { (fx - self.f1) / d };
        self.x1 = x;
        self.f1 = fx;
        if y.is_finite() {
            y as f32
        } else {
            *self = Clip::default();
            0.0
        }
    }
}

fn lp(cutoff: f32, sr: f32) -> OnePole {
    let mut f = OnePole::default();
    f.set(cutoff, sr);
    f
}

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

// ---------------------------------------------------------------- overdrive

pub mod overdrive {
    use super::*;
    pub const DRIVE: usize = 0;
    pub const TONE: usize = 1;
    pub const LEVEL: usize = 2;
    pub static SPECS: &[ParamSpec] = &[
        ParamSpec::lin("Drive", "Overdrive", 0.0, 1.0, 0.5, Unit::Percent),
        ParamSpec::lin("Tone", "Overdrive", 0.0, 1.0, 0.5, Unit::Percent),
        ParamSpec::lin("Level", "Overdrive", -24.0, 12.0, 0.0, Unit::Db),
    ];
}

/// The green mid-boost overdrive: only what is above about 720 Hz is driven
/// into the soft clipping, the lows pass clean beside it, so the bass stays
/// tight and the mids push forward. Then a tone control and the level.
pub struct Overdrive {
    hp: [OnePole; 2],
    post: [OnePole; 2],
    tone: [OnePole; 2],
    clip: [Clip; 2],
    dc: [DcBlock; 2],
    sr: f32,
}

impl Overdrive {
    pub fn new(sr: f32) -> Self {
        Self { hp: [lp(720.0, sr); 2], post: [lp(6000.0, sr); 2], tone: [OnePole::default(); 2], clip: [Clip::default(); 2], dc: [DcBlock::default(); 2], sr }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        use overdrive::*;
        let g = 4.0 * 30f32.powf(p[DRIVE].clamp(0.0, 1.0));
        let level = db_to_gain(p[LEVEL]) * 0.8;
        for ch in 0..2 {
            self.tone[ch].set(700.0 * 2f32.powf(p[TONE].clamp(0.0, 1.0) * 4.0), self.sr);
            let buf: &mut [f32] = if ch == 0 { l } else { r };
            for x in buf.iter_mut() {
                let hi = self.hp[ch].hp(*x);
                let y = 0.4 * *x + 0.6 * self.clip[ch].run(g * hi, Curve::Soft);
                let y = self.tone[ch].lp(self.post[ch].lp(y));
                *x = self.dc[ch].tick(y) * level;
            }
        }
    }
}

// ---------------------------------------------------------------- distortion

pub mod distortion {
    use super::*;
    pub const GAIN: usize = 0;
    pub const FILTER: usize = 1;
    pub const LEVEL: usize = 2;
    pub const CLIP: usize = 3;
    pub const CLIPS: &[&str] = &["Diodes", "Soft", "Lopsided"];
    pub static SPECS: &[ParamSpec] = &[
        ParamSpec::lin("Gain", "Distortion", 0.0, 1.0, 0.6, Unit::Percent),
        ParamSpec::lin("Filter", "Distortion", 0.0, 1.0, 0.4, Unit::Percent),
        ParamSpec::lin("Level", "Distortion", -24.0, 12.0, 0.0, Unit::Db),
        ParamSpec::choice("Clip", "Distortion", CLIPS, 0),
    ];
}

/// The op-amp distortion: up to 66 dB of gain, and the more gain the less
/// bandwidth the op-amp has left (so at full gain only the lows and mids get
/// through it), into clipping diodes and a filter that turns the fizz down.
pub struct Distortion {
    hp: [OnePole; 2],
    amp: [OnePole; 2],
    filter: [OnePole; 2],
    clip: [Clip; 2],
    dc: [DcBlock; 2],
    sr: f32,
}

impl Distortion {
    pub fn new(sr: f32) -> Self {
        Self { hp: [lp(60.0, sr); 2], amp: [OnePole::default(); 2], filter: [OnePole::default(); 2], clip: [Clip::default(); 2], dc: [DcBlock::default(); 2], sr }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        use distortion::*;
        let g = 2.0 * 1000f32.powf(p[GAIN].clamp(0.0, 1.0));
        let curve = match (p[CLIP].round().max(0.0) as usize).min(CLIPS.len() - 1) {
            0 => Curve::Hard,
            1 => Curve::Soft,
            _ => Curve::Bias(0.4),
        };
        let level = db_to_gain(p[LEVEL]) * 0.5;
        for ch in 0..2 {
            self.amp[ch].set((1_000_000.0 / g).min(18000.0), self.sr);
            self.filter[ch].set(18000.0 * 0.0264f32.powf(p[FILTER].clamp(0.0, 1.0)), self.sr);
            let buf: &mut [f32] = if ch == 0 { l } else { r };
            for x in buf.iter_mut() {
                let v = self.amp[ch].lp(g * self.hp[ch].hp(*x));
                let y = self.filter[ch].lp(self.clip[ch].run(v, curve));
                *x = self.dc[ch].tick(y) * level;
            }
        }
    }
}

// ---------------------------------------------------------------- fuzz

pub mod fuzz {
    use super::*;
    pub const FUZZ: usize = 0;
    pub const TONE: usize = 1;
    pub const LEVEL: usize = 2;
    pub const TYPE: usize = 3;
    pub const GATE: usize = 4;
    pub const TYPES: &[&str] = &["Vintage", "Wall", "Octave Up"];
    pub static SPECS: &[ParamSpec] = &[
        ParamSpec::lin("Fuzz", "Fuzz", 0.0, 1.0, 0.7, Unit::Percent),
        ParamSpec::lin("Tone", "Fuzz", 0.0, 1.0, 0.5, Unit::Percent),
        ParamSpec::lin("Level", "Fuzz", -24.0, 12.0, 0.0, Unit::Db),
        ParamSpec::choice("Type", "Fuzz", TYPES, 0),
        ParamSpec::lin("Gate", "Fuzz", 0.0, 1.0, 0.15, Unit::Percent),
    ];
}

/// Three fuzzes. Vintage: two transistors biased off centre, round and
/// lopsided, sputtering as the note dies when the gate is up. Wall: two
/// clipping stages in a row and a tone control that scoops the mids. Octave
/// Up: the wave folded over (rectified) before it clips, so an octave above
/// rings along, sweetest high on the neck.
pub struct Fuzz {
    hp: [OnePole; 2],
    mid_hp: [OnePole; 2],
    mid_lp: [OnePole; 2],
    tone_lp: [OnePole; 2],
    tone_hp: [OnePole; 2],
    clip: [[Clip; 2]; 2],
    env: [Follower; 2],
    dc: [[DcBlock; 2]; 2],
    sr: f32,
}

impl Fuzz {
    pub fn new(sr: f32) -> Self {
        let mut env = [Follower::default(); 2];
        for e in env.iter_mut() {
            e.set(0.001, 0.06, sr);
        }
        Self {
            hp: [lp(120.0, sr); 2],
            mid_hp: [lp(200.0, sr); 2],
            mid_lp: [lp(4000.0, sr); 2],
            tone_lp: [OnePole::default(); 2],
            tone_hp: [OnePole::default(); 2],
            clip: [[Clip::default(); 2]; 2],
            env,
            dc: [[DcBlock::default(); 2]; 2],
            sr,
        }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        use fuzz::*;
        let f = p[FUZZ].clamp(0.0, 1.0);
        let kind = (p[TYPE].round().max(0.0) as usize).min(TYPES.len() - 1);
        let tone = p[TONE].clamp(0.0, 1.0);
        let gate = p[GATE].clamp(0.0, 1.0);
        let th = 0.0005 + 0.04 * gate * gate;
        let bias = Curve::Bias(0.3 + 0.4 * gate as f64);
        let level = db_to_gain(p[LEVEL]) * 0.45;
        for ch in 0..2 {
            if kind == 1 {
                self.tone_lp[ch].set(400.0, self.sr);
                self.tone_hp[ch].set(1200.0, self.sr);
            } else {
                self.tone_lp[ch].set(1000.0 * 2f32.powf(tone * 4.0), self.sr);
            }
            let buf: &mut [f32] = if ch == 0 { l } else { r };
            for x in buf.iter_mut() {
                let v = self.hp[ch].hp(*x);
                let e = self.env[ch].tick(v.abs());
                let y = match kind {
                    0 => {
                        let a = self.clip[ch][0].run(3.0 * 100f32.powf(f) * v, bias);
                        let y = self.dc[ch][0].tick(a) * smoothstep(e / th - 0.5);
                        self.tone_lp[ch].lp(y)
                    }
                    1 => {
                        let a = self.clip[ch][0].run(2.0 * 40f32.powf(f) * v, Curve::Soft);
                        let a = self.mid_lp[ch].lp(self.mid_hp[ch].hp(a));
                        let b = self.clip[ch][1].run(12.0 * a, Curve::Soft) * smoothstep(e / th - 0.5);
                        // The scooped tone: the lows of it, or the highs, or both.
                        let lo = self.tone_lp[ch].lp(b);
                        let hi = self.tone_hp[ch].hp(b);
                        lo * (1.0 - tone) + hi * tone * 1.6
                    }
                    _ => {
                        let up = self.dc[ch][1].tick(v.abs() * 2.0);
                        let a = self.clip[ch][0].run(3.0 * 60f32.powf(f) * (up + 0.3 * v), bias);
                        let y = self.dc[ch][0].tick(a) * smoothstep(e / th - 0.5);
                        self.tone_lp[ch].lp(y)
                    }
                };
                *x = y * level;
            }
        }
    }
}

// ---------------------------------------------------------------- octaver

pub mod octaver {
    use super::*;
    pub const SUB1: usize = 0;
    pub const SUB2: usize = 1;
    pub const UP: usize = 2;
    pub const DRY: usize = 3;
    pub const TONE: usize = 4;
    pub static SPECS: &[ParamSpec] = &[
        ParamSpec::lin("Octave Down", "Octaver", 0.0, 1.0, 0.8, Unit::Percent),
        ParamSpec::lin("Two Down", "Octaver", 0.0, 1.0, 0.0, Unit::Percent),
        ParamSpec::lin("Octave Up", "Octaver", 0.0, 1.0, 0.0, Unit::Percent),
        ParamSpec::lin("Dry", "Octaver", 0.0, 1.0, 0.8, Unit::Percent),
        ParamSpec::lin("Tone", "Octaver", 0.0, 1.0, 0.4, Unit::Percent),
    ];
}

/// The analog octave divider: the fundamental is filtered out, every time
/// it crosses zero going up a flip-flop turns over (and a second one at half
/// that rate), and the flip-flops flip the sign of the fundamental, which
/// leaves a wave an octave (or two) below that follows how you play. One
/// note at a time, like the pedals. Octave Up is the wave folded over.
pub struct Octaver {
    detect: [[Svf; 2]; 2],
    env: [Follower; 2],
    above: [bool; 2],
    ff: [[f32; 2]; 2],
    tone: [[OnePole; 2]; 2],
    up_hp: [OnePole; 2],
    up_dc: [DcBlock; 2],
    sr: f32,
}

impl Octaver {
    pub fn new(sr: f32) -> Self {
        let mut f = Svf::new();
        f.set_q(320.0, 0.707, sr);
        let mut env = [Follower::default(); 2];
        for e in env.iter_mut() {
            e.set(0.002, 0.08, sr);
        }
        Self { detect: [[f; 2]; 2], env, above: [false; 2], ff: [[1.0; 2]; 2], tone: [[OnePole::default(); 2]; 2], up_hp: [lp(150.0, sr); 2], up_dc: [DcBlock::default(); 2], sr }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        use octaver::*;
        let (s1, s2, up, dry) = (p[SUB1].clamp(0.0, 1.0), p[SUB2].clamp(0.0, 1.0), p[UP].clamp(0.0, 1.0), p[DRY].clamp(0.0, 1.0));
        for ch in 0..2 {
            for t in self.tone[ch].iter_mut() {
                t.set(150.0 * 2f32.powf(p[TONE].clamp(0.0, 1.0) * 4.5), self.sr);
            }
            let buf: &mut [f32] = if ch == 0 { l } else { r };
            for x in buf.iter_mut() {
                let a = self.detect[ch][0].tick(*x).low;
                let fund = self.detect[ch][1].tick(a).low;
                let e = self.env[ch].tick(fund.abs());
                // Turn over on the way up, with some hysteresis against noise.
                let h = 0.15 * e;
                if !self.above[ch] && fund > h {
                    self.above[ch] = true;
                    self.ff[ch][0] = -self.ff[ch][0];
                    if self.ff[ch][0] > 0.0 {
                        self.ff[ch][1] = -self.ff[ch][1];
                    }
                } else if self.above[ch] && fund < -h {
                    self.above[ch] = false;
                }
                let d1 = self.tone[ch][0].lp(self.ff[ch][0] * fund);
                let d2 = self.tone[ch][1].lp(self.ff[ch][1] * fund);
                let u = self.up_dc[ch].tick(self.up_hp[ch].hp(*x).abs());
                *x = dry * *x + 2.2 * (s1 * d1 + s2 * d2) + 1.5 * up * u;
            }
        }
    }
}

// ---------------------------------------------------------------- squash

pub mod squash {
    use super::*;
    pub const SUSTAIN: usize = 0;
    pub const ATTACK: usize = 1;
    pub const RELEASE: usize = 2;
    pub const LEVEL: usize = 3;
    pub const BLEND: usize = 4;
    pub static SPECS: &[ParamSpec] = &[
        ParamSpec::lin("Sustain", "Squash", 0.0, 1.0, 0.5, Unit::Percent),
        ParamSpec::log("Attack", "Squash", 0.5, 50.0, 6.0, Unit::Ms),
        ParamSpec::log("Release", "Squash", 20.0, 1000.0, 200.0, Unit::Ms),
        ParamSpec::lin("Level", "Squash", -24.0, 12.0, 0.0, Unit::Db),
        ParamSpec::lin("Blend", "Squash", 0.0, 1.0, 1.0, Unit::Percent),
    ];
}

/// The squashy pedal compressor: more Sustain is a lower threshold and a
/// higher ratio with the make-up to match, so notes ring on at one level.
/// Blend mixes the dry pick attack back in.
pub struct Squash {
    env: Follower,
    pub(crate) reduction_db: f32,
}

impl Squash {
    pub fn new() -> Self {
        Self { env: Follower::default(), reduction_db: 0.0 }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        use squash::*;
        let s = p[SUSTAIN].clamp(0.0, 1.0);
        let th = -6.0 - 34.0 * s;
        let slope = 1.0 - 1.0 / (2.0 + 10.0 * s);
        let makeup = -th * slope * 0.7;
        let out = db_to_gain(p[LEVEL]);
        let blend = p[BLEND].clamp(0.0, 1.0);
        self.env.set(p[ATTACK] * 0.001, p[RELEASE] * 0.001, ctx.sr);
        let mut most = 0.0f32;
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            let peak = a.abs().max(b.abs());
            // The detector follows at the Attack time, but a loud note
            // straight after silence is caught at once (no +20 dB pop).
            let e = self.env.tick(peak).max(0.3 * peak);
            let db = 20.0 * e.max(1e-6).log10();
            // A soft knee 6 dB wide.
            let over = db - th;
            let red = if over <= -3.0 { 0.0 } else if over >= 3.0 { over * slope } else { slope * (over + 3.0).powi(2) / 12.0 };
            most = most.max(red);
            let g = db_to_gain(makeup - red);
            *a = (*a * (blend * g + (1.0 - blend))) * out;
            *b = (*b * (blend * g + (1.0 - blend))) * out;
        }
        self.reduction_db = most;
    }
}

// ---------------------------------------------------------------- wah

pub mod wah {
    use super::*;
    pub const PEDAL: usize = 0;
    pub const MODE: usize = 1;
    pub const RESONANCE: usize = 2;
    pub const SENSITIVITY: usize = 3;
    pub const RATE: usize = 4;
    pub const SYNC: usize = 5;
    pub const MIX: usize = 6;
    pub const MODES: &[&str] = &["Pedal", "Auto", "Wobble"];
    pub static SPECS: &[ParamSpec] = &[
        ParamSpec::lin("Pedal", "Wah", 0.0, 1.0, 0.5, Unit::Percent),
        ParamSpec::choice("Mode", "Wah", MODES, 0),
        ParamSpec::lin("Resonance", "Wah", 0.0, 1.0, 0.6, Unit::Percent),
        ParamSpec::lin("Sensitivity", "Auto", 0.0, 1.0, 0.5, Unit::Percent),
        ParamSpec::log("Rate", "Wobble", 0.1, 10.0, 1.5, Unit::Hz),
        ParamSpec::choice("Sync", "Wobble", LFO_SYNC, 0),
        ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
    ];
}

/// The wah: a resonant peak swept from about 350 Hz (heel down) to 2.3 kHz
/// (toe down). Rock it with the Pedal knob (automate or modulate it), let
/// your playing open it (Auto) or let it wobble by itself.
pub struct Wah {
    f: [Svf; 2],
    pos: f32,
    env: Follower,
    lfo: Lfo,
    pub(crate) shown: f32,
}

impl Wah {
    pub fn new() -> Self {
        Self { f: [Svf::new(), Svf::new()], pos: 0.5, env: Follower::default(), lfo: Lfo::default(), shown: 0.5 }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        use wah::*;
        let n = l.len().min(r.len());
        let mode = (p[MODE].round().max(0.0) as usize).min(MODES.len() - 1);
        let q = 2.0 + 8.0 * p[RESONANCE].clamp(0.0, 1.0);
        let gain = 1.8 / q.sqrt();
        let mix = p[MIX].clamp(0.0, 1.0);
        let pedal = p[PEDAL].clamp(0.0, 1.0);
        let sens = p[SENSITIVITY].clamp(0.0, 1.0);
        self.env.set(0.004, 0.15, ctx.sr);
        let k = 1.0 - (-1.0 / (0.012 * ctx.sr)).exp();
        let mut phases = [0.0f32; CHUNK];
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            if mode == 2 {
                self.lfo.run(p[RATE], p[SYNC], ctx, &mut phases[..len]);
            }
            for j in 0..len {
                let i = done + j;
                let target = match mode {
                    1 => {
                        let e = self.env.tick(l[i].abs().max(r[i].abs()));
                        (pedal * 0.3 + e * (2.0 + 20.0 * sens)).min(1.0)
                    }
                    2 => 0.5 - 0.5 * (TAU * phases[j]).cos(),
                    _ => pedal,
                };
                self.pos += k * (target - self.pos);
                // Every 8 samples is smooth enough for the sweep.
                if j % 8 == 0 {
                    let fc = 350.0 * (2300.0f32 / 350.0).powf(self.pos);
                    for f in self.f.iter_mut() {
                        f.set_q(fc, q, ctx.sr);
                    }
                }
                for (ch, x) in [&mut l[i], &mut r[i]].into_iter().enumerate() {
                    let o = self.f[ch].tick(*x);
                    let wet = (o.band + 0.12 * o.low) * gain;
                    *x = *x * (1.0 - mix) + wet * mix;
                }
            }
            done += len;
        }
        self.shown = self.pos;
    }
}

// ---------------------------------------------------------------- tremolo

pub mod tremolo {
    use super::*;
    pub const RATE: usize = 0;
    pub const SYNC: usize = 1;
    pub const DEPTH: usize = 2;
    pub const SHAPE: usize = 3;
    pub const STEREO: usize = 4;
    pub const SHAPES: &[&str] = &["Sine", "Triangle", "Choppy", "Harmonic"];
    pub static SPECS: &[ParamSpec] = &[
        ParamSpec::log("Rate", "Tremolo", 0.5, 15.0, 5.0, Unit::Hz),
        ParamSpec::choice("Sync", "Tremolo", LFO_SYNC, 0),
        ParamSpec::lin("Depth", "Tremolo", 0.0, 1.0, 0.6, Unit::Percent),
        ParamSpec::choice("Shape", "Tremolo", SHAPES, 0),
        ParamSpec::lin("Stereo", "Tremolo", 0.0, 1.0, 0.0, Unit::Percent),
    ];
}

/// The amp's tremolo: the level rises and falls. Choppy is almost on and
/// off; Harmonic splits the sound at 800 Hz and moves the lows and the highs
/// in turn, which makes it shimmer more than throb. Stereo moves the two
/// sides apart, up to a pan.
pub struct Tremolo {
    lfo: Lfo,
    split: [OnePole; 2],
    pub(crate) shown: f32,
}

impl Tremolo {
    pub fn new(sr: f32) -> Self {
        Self { lfo: Lfo::default(), split: [lp(800.0, sr); 2], shown: 1.0 }
    }

    fn shape(kind: usize, ph: f32) -> f32 {
        match kind {
            1 => 1.0 - 4.0 * (ph - 0.5).abs(),
            2 => (6.0 * (TAU * ph).sin()).tanh(),
            _ => (TAU * ph).sin(),
        }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        use tremolo::*;
        let n = l.len().min(r.len());
        let depth = p[DEPTH].clamp(0.0, 1.0);
        let kind = (p[SHAPE].round().max(0.0) as usize).min(SHAPES.len() - 1);
        let off = 0.5 * p[STEREO].clamp(0.0, 1.0);
        let mut phases = [0.0f32; CHUNK];
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            self.lfo.run(p[RATE], p[SYNC], ctx, &mut phases[..len]);
            for j in 0..len {
                let i = done + j;
                for ch in 0..2 {
                    let ph = (phases[j] + if ch == 1 { off } else { 0.0 }).fract();
                    let x = if ch == 0 { &mut l[i] } else { &mut r[i] };
                    if kind == 3 {
                        let lo = self.split[ch].lp(*x);
                        let hi = *x - lo;
                        let g1 = 1.0 - depth * (0.5 - 0.5 * Self::shape(0, ph));
                        let g2 = 1.0 - depth * (0.5 + 0.5 * Self::shape(0, ph));
                        *x = lo * g1 + hi * g2;
                    } else {
                        *x *= 1.0 - depth * (0.5 - 0.5 * Self::shape(kind, ph));
                    }
                }
            }
            if len > 0 {
                self.shown = 0.5 + 0.5 * Self::shape(if kind == 3 { 0 } else { kind }, phases[len - 1]);
            }
            done += len;
        }
    }
}

// ---------------------------------------------------------------- vibe

pub mod vibe {
    use super::*;
    pub const SPEED: usize = 0;
    pub const SYNC: usize = 1;
    pub const INTENSITY: usize = 2;
    pub const MODE: usize = 3;
    pub const MODES: &[&str] = &["Chorus", "Vibrato"];
    pub static SPECS: &[ParamSpec] = &[
        ParamSpec::log("Speed", "Vibe", 0.3, 12.0, 2.0, Unit::Hz),
        ParamSpec::choice("Sync", "Vibe", LFO_SYNC, 0),
        ParamSpec::lin("Intensity", "Vibe", 0.0, 1.0, 0.7, Unit::Percent),
        ParamSpec::choice("Mode", "Vibe", MODES, 0),
    ];
}

/// The photocell vibe: four all-pass stages, each tuned by its own
/// capacitor, all moved by one lamp. The lamp heats up faster than it cools
/// and the cells answer unevenly, which gives the lopsided, throbbing sweep.
/// Chorus mixes it with the dry sound (moving notches), Vibrato is the pitch
/// wobble alone.
pub struct Vibe {
    lfo: Lfo,
    lamp: f32,
    coef: [f32; 4],
    z: [[f32; 4]; 2],
    pub(crate) shown: f32,
}

const VIBE_BASE: [f32; 4] = [90.0, 300.0, 1000.0, 3300.0];

impl Vibe {
    pub fn new() -> Self {
        Self { lfo: Lfo::default(), lamp: 0.0, coef: [0.0; 4], z: [[0.0; 4]; 2], shown: 0.0 }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        use vibe::*;
        let n = l.len().min(r.len());
        let depth = p[INTENSITY].clamp(0.0, 1.0);
        let vibrato = p[MODE] >= 0.5;
        let heat = 1.0 - (-1.0 / (0.004 * ctx.sr)).exp();
        let cool = 1.0 - (-1.0 / (0.025 * ctx.sr)).exp();
        let mut phases = [0.0f32; CHUNK];
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            self.lfo.run(p[SPEED], p[SYNC], ctx, &mut phases[..len]);
            for j in 0..len {
                let i = done + j;
                let drive = 0.5 - 0.5 * (TAU * phases[j]).cos();
                self.lamp += (drive - self.lamp) * if drive > self.lamp { heat } else { cool };
                // The cells: resistance falls unevenly as the lamp brightens.
                let cell = (self.lamp + 0.04).powf(0.7) / 1.04f32.powf(0.7);
                let m = 8f32.powf(depth * (cell - 0.5) * 2.0);
                if j % 4 == 0 {
                    for (k, c) in self.coef.iter_mut().enumerate() {
                        let t = (PI * (VIBE_BASE[k] * m).min(0.45 * ctx.sr) / ctx.sr).tan();
                        *c = (t - 1.0) / (t + 1.0);
                    }
                }
                for ch in 0..2 {
                    let x = if ch == 0 { &mut l[i] } else { &mut r[i] };
                    let mut v = *x;
                    for (c, z) in self.coef.iter().zip(self.z[ch].iter_mut()) {
                        let y = c * v + *z;
                        *z = v - c * y;
                        v = y;
                    }
                    *x = if vibrato { v } else { 0.5 * (*x + v) };
                }
            }
            done += len;
        }
        self.shown = self.lamp;
    }
}

// ---------------------------------------------------------------- amp

pub mod amp {
    use super::*;
    pub const TYPE: usize = 0;
    pub const GAIN: usize = 1;
    pub const BASS: usize = 2;
    pub const MID: usize = 3;
    pub const TREBLE: usize = 4;
    pub const PRESENCE: usize = 5;
    pub const MASTER: usize = 6;
    pub const CAB: usize = 7;
    pub const TYPES: &[&str] = &["Clean", "Crunch", "Lead"];
    pub const CABS: &[&str] = &["Off", "1x12 Open", "2x12", "4x12 Closed"];
    pub static SPECS: &[ParamSpec] = &[
        ParamSpec::choice("Type", "Amp", TYPES, 1),
        ParamSpec::lin("Gain", "Amp", 0.0, 1.0, 0.5, Unit::Percent),
        ParamSpec::lin("Bass", "Tone", -12.0, 12.0, 0.0, Unit::Db),
        ParamSpec::lin("Mid", "Tone", -12.0, 12.0, 0.0, Unit::Db),
        ParamSpec::lin("Treble", "Tone", -12.0, 12.0, 0.0, Unit::Db),
        ParamSpec::lin("Presence", "Tone", 0.0, 1.0, 0.5, Unit::Percent),
        ParamSpec::lin("Master", "Output", -24.0, 12.0, 0.0, Unit::Db),
        ParamSpec::choice("Cabinet", "Output", CABS, 2),
    ];
}

/// A guitar amp: a bright input, one to two tube stages (lopsided soft
/// clipping), the tone stack (bass, mid, treble), a power stage that sags a
/// little when hit hard, presence, and a speaker cabinet (the low thump, the
/// cone's mid dip and peak, and the steep roll-off above 4 to 6 kHz that
/// takes the fizz away).
pub struct Amp {
    input_hp: [OnePole; 2],
    inter_hp: [OnePole; 2],
    inter_lp: [OnePole; 2],
    clip: [[Clip; 3]; 2],
    eq: [Coefs; 5],
    eq_st: [[State; 5]; 2],
    cab: [Coefs; 6],
    cab_st: [[State; 6]; 2],
    cab_n: usize,
    sag: Follower,
    dc: [DcBlock; 2],
    key: [f32; 7],
    sr: f32,
}

impl Amp {
    pub fn new(sr: f32) -> Self {
        let mut sag = Follower::default();
        sag.set(0.01, 0.2, sr);
        Self {
            input_hp: [lp(70.0, sr); 2],
            inter_hp: [lp(40.0, sr); 2],
            inter_lp: [lp(9000.0, sr); 2],
            clip: [[Clip::default(); 3]; 2],
            eq: [Coefs::IDENTITY; 5],
            eq_st: [[State::default(); 5]; 2],
            cab: [Coefs::IDENTITY; 6],
            cab_st: [[State::default(); 6]; 2],
            cab_n: 0,
            sag,
            dc: [DcBlock::default(); 2],
            key: [f32::NAN; 7],
            sr,
        }
    }

    fn design(&mut self, p: &[f32]) {
        use amp::*;
        let key = [p[TYPE].round(), p[BASS], p[MID], p[TREBLE], p[PRESENCE], p[CAB].round(), self.sr];
        if key == self.key {
            return;
        }
        self.key = key;
        let sr = self.sr as f64;
        let kind = (p[TYPE].round().max(0.0) as usize).min(TYPES.len() - 1);
        // Bright cap on the clean channel, the tone stack, then presence.
        self.eq = [
            biquad::high_shelf(2500.0, sr, 0.7, if kind == 0 { 4.0 } else { 1.5 }),
            biquad::low_shelf(120.0, sr, 0.7, p[BASS].clamp(-12.0, 12.0) as f64),
            biquad::bell(650.0, sr, 0.8, p[MID].clamp(-12.0, 12.0) as f64 - if kind == 2 { 3.0 } else { 0.0 }),
            biquad::high_shelf(2800.0, sr, 0.7, p[TREBLE].clamp(-12.0, 12.0) as f64),
            biquad::high_shelf(4500.0, sr, 0.7, (p[PRESENCE].clamp(0.0, 1.0) * 12.0 - 4.0) as f64),
        ];
        let cab = (p[CAB].round().max(0.0) as usize).min(CABS.len() - 1);
        let (cabs, n): ([Coefs; 6], usize) = match cab {
            1 => ([biquad::highpass(90.0, sr, 0.7), biquad::bell(130.0, sr, 1.2, 3.0), biquad::bell(2200.0, sr, 1.0, 3.0), biquad::lowpass(6000.0, sr, 0.7), biquad::lowpass(7500.0, sr, 0.6), Coefs::IDENTITY], 5),
            2 => ([biquad::highpass(75.0, sr, 0.7), biquad::bell(110.0, sr, 1.3, 4.0), biquad::bell(450.0, sr, 1.0, -3.0), biquad::bell(2500.0, sr, 1.2, 4.0), biquad::lowpass(5000.0, sr, 0.7), biquad::lowpass(6500.0, sr, 0.6)], 6),
            3 => ([biquad::highpass(65.0, sr, 0.7), biquad::bell(100.0, sr, 1.4, 5.0), biquad::bell(500.0, sr, 0.9, -4.0), biquad::bell(2800.0, sr, 1.5, 5.0), biquad::lowpass(4200.0, sr, 0.75), biquad::lowpass(5200.0, sr, 0.6)], 6),
            _ => ([Coefs::IDENTITY; 6], 0),
        };
        self.cab = cabs;
        self.cab_n = n;
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        use amp::*;
        self.design(p);
        let kind = (p[TYPE].round().max(0.0) as usize).min(TYPES.len() - 1);
        let gain = p[GAIN].clamp(0.0, 1.0);
        let (g1, g2, b1, b2, out) = match kind {
            0 => (1.0 + 7.0 * gain, 0.0, 0.05f64, 0.0f64, 0.9),
            1 => (2.0 * 30f32.powf(gain), 2.5, 0.2, 0.1, 0.45),
            _ => (4.0 * 60f32.powf(gain), 8.0, 0.25, -0.15, 0.35),
        };
        let master = db_to_gain(p[MASTER]) * out * if self.cab_n > 0 { 1.5 } else { 1.0 };
        for i in 0..l.len().min(r.len()) {
            let mut v = [l[i], r[i]];
            let mut peak = 0.0f32;
            for ch in 0..2 {
                let mut x = self.input_hp[ch].hp(v[ch]);
                x = self.eq_st[ch][0].run(&self.eq[0], x as f64) as f32;
                x = self.clip[ch][0].run(g1 * x, Curve::Bias(b1)) / if kind == 0 { g1.sqrt() } else { 1.0 };
                x = self.inter_lp[ch].lp(self.inter_hp[ch].hp(x));
                if g2 > 0.0 {
                    x = self.clip[ch][1].run(g2 * x, Curve::Bias(b2));
                }
                for k in 1..4 {
                    x = self.eq_st[ch][k].run(&self.eq[k], x as f64) as f32;
                }
                v[ch] = x;
                peak = peak.max(x.abs());
            }
            // The power stage sags: the harder it is driven, the less it gives.
            let s = self.sag.tick(peak);
            let pg = 1.4 / (1.0 + 0.6 * s);
            for ch in 0..2 {
                let mut x = self.clip[ch][2].run(pg * v[ch], Curve::Soft);
                x = self.eq_st[ch][4].run(&self.eq[4], x as f64) as f32;
                for k in 0..self.cab_n {
                    x = self.cab_st[ch][k].run(&self.cab[k], x as f64) as f32;
                }
                v[ch] = self.dc[ch].tick(x) * master;
            }
            l[i] = v[0];
            r[i] = v[1];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::ctx;
    use super::*;

    const SR: f32 = 48000.0;

    fn saw(f: f32, secs: f32, amp: f32) -> Vec<f32> {
        (0..(secs * SR) as usize).map(|i| amp * (2.0 * (i as f32 * f / SR).fract() - 1.0)).collect()
    }

    fn sine(f: f32, secs: f32, amp: f32) -> Vec<f32> {
        (0..(secs * SR) as usize).map(|i| amp * (TAU * i as f32 * f / SR).sin()).collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn mag(x: &[f32], hz: f32) -> f32 {
        let w = TAU as f64 * hz as f64 / SR as f64;
        let n = x.len() as f64;
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, &s) in x.iter().enumerate() {
            let win = 0.5 - 0.5 * (TAU as f64 * i as f64 / n).cos();
            re += s as f64 * win * (w * i as f64).cos();
            im += s as f64 * win * (w * i as f64).sin();
        }
        ((re * re + im * im).sqrt() / n) as f32
    }

    /// Runs a pedal in blocks of 256 and returns the left channel.
    fn run(f: &mut dyn FnMut(&[f32], &mut [f32], &mut [f32], &FxCtx), p: &[f32], input: &[f32]) -> Vec<f32> {
        let c = ctx();
        let mut out = Vec::with_capacity(input.len());
        for chunk in input.chunks(256) {
            let mut l = chunk.to_vec();
            let mut r = chunk.to_vec();
            f(p, &mut l, &mut r, &c);
            assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 4.0), "out of bounds");
            out.extend_from_slice(&l);
        }
        out
    }

    fn defaults(s: &[ParamSpec]) -> Vec<f32> {
        crate::params::defaults(s)
    }

    #[test]
    fn clip_matches_its_curve_on_slow_input() {
        for c in [Curve::Soft, Curve::Hard, Curve::Bias(0.3)] {
            let mut k = Clip::default();
            let mut worst = 0.0f64;
            for i in 0..2000 {
                let x = 3.0 * (i as f32 / 2000.0 * TAU).sin();
                let x_prev = 3.0 * ((i as f32 - 1.0) / 2000.0 * TAU).sin();
                let y = k.run(x, c) as f64;
                if i > 0 {
                    worst = worst.max((y - curve(c, 0.5 * (x + x_prev) as f64)).abs());
                }
            }
            assert!(worst < 0.01, "{worst}");
        }
    }

    #[test]
    fn hard_clipping_aliases_less_with_adaa() {
        // A high sine clipped hard: the folded-back partial at |3f - sr| is
        // much weaker than with naive clipping.
        let f = 7000.0;
        let x = sine(f, 0.25, 4.0);
        let mut k = Clip::default();
        let adaa: Vec<f32> = x.iter().map(|&v| k.run(v, Curve::Hard)).collect();
        let naive: Vec<f32> = x.iter().map(|&v| v.clamp(-1.0, 1.0)).collect();
        let alias = SR - 7.0 * f;
        let (a, b) = (mag(&adaa, alias.abs()), mag(&naive, alias.abs()));
        assert!(a < 0.5 * b, "adaa {a} naive {b}");
    }

    #[test]
    fn every_pedal_stays_bounded_and_changes_the_sound() {
        let input = saw(110.0, 0.5, 0.5);
        let c = ctx();
        let mut od = Overdrive::new(SR);
        let mut ds = Distortion::new(SR);
        let mut fz = Fuzz::new(SR);
        let mut oc = Octaver::new(SR);
        let mut sq = Squash::new();
        let mut wa = Wah::new();
        let mut tr = Tremolo::new(SR);
        let mut vb = Vibe::new();
        let mut am = Amp::new(SR);
        let _ = c;
        let cases: Vec<(&str, Vec<f32>)> = vec![
            ("overdrive", run(&mut |p, l, r, c| od.process(p, l, r, c), &defaults(overdrive::SPECS), &input)),
            ("distortion", run(&mut |p, l, r, c| ds.process(p, l, r, c), &defaults(distortion::SPECS), &input)),
            ("fuzz", run(&mut |p, l, r, c| fz.process(p, l, r, c), &defaults(fuzz::SPECS), &input)),
            ("octaver", run(&mut |p, l, r, c| oc.process(p, l, r, c), &defaults(octaver::SPECS), &input)),
            ("squash", run(&mut |p, l, r, c| sq.process(p, l, r, c), &defaults(squash::SPECS), &input)),
            ("wah", run(&mut |p, l, r, c| wa.process(p, l, r, c), &defaults(wah::SPECS), &input)),
            ("tremolo", run(&mut |p, l, r, c| tr.process(p, l, r, c), &defaults(tremolo::SPECS), &input)),
            ("vibe", run(&mut |p, l, r, c| vb.process(p, l, r, c), &defaults(vibe::SPECS), &input)),
            ("amp", run(&mut |p, l, r, c| am.process(p, l, r, c), &defaults(amp::SPECS), &input)),
        ];
        let level_in = rms(&input);
        for (name, out) in cases {
            let level = rms(&out[4800..]);
            let diff = rms(&out.iter().zip(&input).map(|(a, b)| a - b).collect::<Vec<_>>()[4800..]);
            eprintln!("{name}: in {level_in:.3} out {level:.3} diff {diff:.3}");
            assert!(level > 0.25 * level_in && level < 4.0 * level_in, "{name}: level {level} for {level_in}");
            if name != "squash" {
                assert!(diff > 0.02, "{name} did nothing");
            }
        }
    }

    #[test]
    fn drive_adds_harmonics() {
        let input = sine(220.0, 0.3, 0.3);
        let mut p = defaults(overdrive::SPECS);
        let thd = |p: &[f32]| {
            let mut od = Overdrive::new(SR);
            let out = run(&mut |p, l, r, c| od.process(p, l, r, c), p, &input);
            let tail = &out[4800..];
            (mag(tail, 660.0) + mag(tail, 440.0)) / mag(tail, 220.0)
        };
        p[overdrive::DRIVE] = 0.0;
        let low = thd(&p);
        p[overdrive::DRIVE] = 1.0;
        let high = thd(&p);
        assert!(high > 3.0 * low, "{low} {high}");
    }

    #[test]
    fn octaver_plays_an_octave_below() {
        let input = sine(220.0, 0.6, 0.4);
        let mut p = defaults(octaver::SPECS);
        p[octaver::DRY] = 0.0;
        let mut oc = Octaver::new(SR);
        let out = run(&mut |p, l, r, c| oc.process(p, l, r, c), &p, &input);
        let tail = &out[9600..];
        assert!(mag(tail, 110.0) > 3.0 * mag(tail, 220.0), "{} {}", mag(tail, 110.0), mag(tail, 220.0));
        p[octaver::SUB1] = 0.0;
        p[octaver::SUB2] = 1.0;
        let mut oc = Octaver::new(SR);
        let out = run(&mut |p, l, r, c| oc.process(p, l, r, c), &p, &input);
        let tail = &out[9600..];
        assert!(mag(tail, 55.0) > 0.3 * mag(tail, 110.0).max(mag(tail, 220.0)), "{} {}", mag(tail, 55.0), mag(tail, 110.0));
    }

    #[test]
    fn wah_moves_its_peak_with_the_pedal() {
        let input = saw(110.0, 0.4, 0.4);
        let peak_at = |pedal: f32| {
            let mut p = defaults(wah::SPECS);
            p[wah::PEDAL] = pedal;
            let mut w = Wah::new();
            let out = run(&mut |p, l, r, c| w.process(p, l, r, c), &p, &input);
            let tail = &out[9600..];
            (1..30).max_by(|&a, &b| mag(tail, 110.0 * a as f32).total_cmp(&mag(tail, 110.0 * b as f32))).unwrap()
        };
        let heel = peak_at(0.0);
        let toe = peak_at(1.0);
        assert!(heel <= 4 && toe >= 15, "heel peaks at harmonic {heel}, toe at {toe}");
    }

    #[test]
    fn tremolo_pulses_at_its_rate() {
        let input = vec![0.5f32; (SR * 1.0) as usize];
        let mut p = defaults(tremolo::SPECS);
        p[tremolo::RATE] = 4.0;
        p[tremolo::DEPTH] = 1.0;
        let mut t = Tremolo::new(SR);
        let out = run(&mut |p, l, r, c| t.process(p, l, r, c), &p, &input);
        let lo = out.iter().fold(1.0f32, |m, v| m.min(*v));
        let hi = out.iter().fold(0.0f32, |m, v| m.max(*v));
        assert!(lo < 0.02 && hi > 0.48, "{lo} {hi}");
        assert!(mag(&out, 4.0) > 10.0 * mag(&out, 7.0));
    }

    #[test]
    fn squash_evens_out_the_level() {
        let mut p = defaults(squash::SPECS);
        p[squash::SUSTAIN] = 0.8;
        let level = |amp: f32| {
            let mut s = Squash::new();
            let out = run(&mut |p, l, r, c| s.process(p, l, r, c), &p, &sine(220.0, 0.4, amp));
            rms(&out[9600..])
        };
        let (soft, loud) = (level(0.05), level(0.8));
        // 24 dB in, much less out.
        assert!(loud / soft < 4.0, "{soft} {loud}");
    }

    #[test]
    fn the_cabinet_takes_the_fizz_away() {
        let input = sine(8000.0, 0.2, 0.05);
        let mid = sine(1000.0, 0.2, 0.05);
        let mut p = defaults(amp::SPECS);
        p[amp::TYPE] = 0.0;
        p[amp::GAIN] = 0.0;
        let through = |p: &[f32], x: &[f32]| {
            let mut a = Amp::new(SR);
            rms(&run(&mut |p, l, r, c| a.process(p, l, r, c), p, x)[2400..])
        };
        p[amp::CAB] = 3.0;
        let closed = through(&p, &input) / through(&p, &mid);
        p[amp::CAB] = 0.0;
        let open = through(&p, &input) / through(&p, &mid);
        assert!(closed < 0.2 * open, "4x12 {closed} vs none {open}");
    }
}
