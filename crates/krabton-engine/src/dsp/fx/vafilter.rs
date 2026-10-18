//! VA Filter: the filters of analog synthesizers, modelled as their
//! circuits are (topology-preserving transforms, zero-delay feedback),
//! ported from the Faust virtual analog filter library (vaeffects.lib):
//! the four-pole transistor ladder (after Zavalishin, by Dario Sanfilippo),
//! the half ladder, the diode ladder of the little bass machines, the
//! Sallen-Key filter with the screaming feedback of the 35 chip (low and
//! high pass), the multimode state-variable filter with its saturating
//! band (low, band, high, notch) and the clean Sallen-Key (low, band,
//! high), all by Eric Tarr after Will Pirkle's application notes. On top:
//! a drive into the filter, the cutoff moved by an envelope follower and an
//! LFO (the right side's LFO a little later for width), a mix and a level.
//!
//! Changes from the library: the cutoff in hertz (the library's normalized
//! 0..1 is 20 Hz to 20 kHz), resonance 0..1 mapped onto each filter's own
//! range, the ladder's input stage and the 35 filter's feedback saturate so
//! they can self-oscillate without running away, the diode ladder takes the
//! drive in place of the library's hard input clip.
//!
//! Faust library code: Copyright (C) 2003-2019 Julius O. Smith III, Eric
//! Tarr, Dario Sanfilippo, under the MIT-style STK-4.3 license
//! (<https://github.com/grame-cncm/faustlibraries>, vaeffects.lib): the
//! software may be used, copied, modified, merged, published, distributed,
//! sublicensed and sold, provided the copyright notice and permission
//! notice are included in all copies or substantial portions of it; it is
//! provided "as is", without warranty of any kind.

use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};
use std::f32::consts::{PI, TAU};

pub const MODEL: usize = 0;
pub const CUTOFF: usize = 1;
pub const RESONANCE: usize = 2;
pub const DRIVE: usize = 3;
pub const ENV: usize = 4;
pub const ENV_RELEASE: usize = 5;
pub const LFO: usize = 6;
pub const LFO_RATE: usize = 7;
pub const LFO_SHAPE: usize = 8;
pub const STEREO: usize = 9;
pub const MIX: usize = 10;
pub const OUTPUT: usize = 11;

pub const MODELS: &[&str] = &["Ladder", "Half Ladder", "Diode Ladder", "Scream LP", "Scream HP", "Multimode LP", "Multimode BP", "Multimode HP", "Multimode Notch", "Clean LP", "Clean BP", "Clean HP"];
pub const LFO_SHAPES: &[&str] = &["Sine", "Triangle", "Saw Up", "Saw Down", "Square", "Random"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Model", "Filter", MODELS, 0),
    ParamSpec::log("Cutoff", "Filter", 20.0, 20000.0, 1200.0, Unit::Hz),
    ParamSpec::lin("Resonance", "Filter", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Drive", "Filter", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::lin("Env Amount", "Envelope", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Env Release", "Envelope", 5.0, 2000.0, 150.0, Unit::Ms),
    ParamSpec::lin("LFO Amount", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("LFO Rate", "LFO", 0.02, 20.0, 1.0, Unit::Hz),
    ParamSpec::choice("LFO Shape", "LFO", LFO_SHAPES, 0),
    ParamSpec::lin("Stereo", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Output", "Output", -24.0, 12.0, 0.0, Unit::Db),
];

/// The library's cubic soft clip (cubicnl with no drive): clipped to ±1, then x - x³/3.
fn cubic(x: f32) -> f32 {
    let x = x.clamp(-1.0, 1.0);
    x - x * x * x / 3.0
}

/// A fast tanh (Padé), for the saturating stages.
fn tanh(x: f32) -> f32 {
    let x = x.clamp(-3.0, 3.0);
    x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
}

/// The prewarped integrator gain for a cutoff.
fn warp(fc: f32, sr: f32) -> f32 {
    (PI * fc.clamp(10.0, 0.45 * sr) / sr).tan()
}

/// One channel of every filter's state (only one runs at a time).
#[derive(Clone, Copy, Default)]
struct Channel {
    s: [f32; 4],
}

impl Channel {
    /// The four-pole ladder (Sanfilippo's TPT form, Zavalishin figure 3.30).
    fn ladder(&mut self, x: f32, w: f32, k: f32) -> f32 {
        let g = w / (1.0 + w);
        let [s1, s2, s3, s4] = self.s;
        let big_g = g * g * g * g;
        let big_s = g * g * g * (s1 * (1.0 - g)) + g * g * (s2 * (1.0 - g)) + g * (s3 * (1.0 - g)) + s4 * (1.0 - g);
        // (the input stage saturates: the self-oscillation stays bounded)
        let u = tanh((x - k * big_s) / (1.0 + k * big_g));
        let v1 = g * (u - s1);
        let lp1 = v1 + s1;
        let v2 = g * (lp1 - s2);
        let lp2 = v2 + s2;
        let v3 = g * (lp2 - s3);
        let lp3 = v3 + s3;
        let v4 = g * (lp3 - s4);
        let lp4 = v4 + s4;
        self.s = [lp1 + v1, lp2 + v2, lp3 + v3, lp4 + v4];
        lp4
    }

    /// Tarr's half ladder: two poles and an all-pass in the loop.
    fn half_ladder(&mut self, x: f32, g: f32, k: f32) -> f32 {
        let [s1, s2, s3, _] = self.s;
        let big_g = g / (1.0 + g);
        let alpha = big_g;
        let ga = 2.0 * big_g - 1.0;
        let b1 = ga * big_g / (1.0 + g);
        let b2 = ga / (1.0 + g);
        let b3 = 2.0 / (1.0 + g);
        let alpha0 = 1.0 / (1.0 + k * ga * big_g * big_g);
        let u = tanh((x - s3 * b3 * k - s2 * b2 * k - s1 * b1 * k) * alpha0);
        let y1 = (u - s1) * alpha + s1;
        let y2 = (y1 - s2) * alpha + s2;
        let y = -y2 + 2.0 * ((y2 - s3) * alpha + s3);
        self.s[0] = (u - s1) * alpha * 2.0 + s1;
        self.s[1] = (y1 - s2) * alpha * 2.0 + s2;
        self.s[2] = (y2 - s3) * alpha * 2.0 + s3;
        y
    }

    /// Tarr's diode ladder: four stages that load each other.
    fn diode(&mut self, x: f32, g: f32, k: f32) -> f32 {
        let [s1, s2, s3, s4] = self.s;
        let g4 = 0.5 * g / (1.0 + g);
        let g3 = 0.5 * g / (1.0 + g - 0.5 * g * g4);
        let g2 = 0.5 * g / (1.0 + g - 0.5 * g * g3);
        let g1 = g / (1.0 + g - g * g2);
        let gamma = g1 * g2 * g3 * g4;
        let (sg1, sg3, sg4) = (g4 * g3 * g2, g4, 1.0);
        let alpha = g / (1.0 + g);
        let alpha0 = 1.0 / (1.0 + k * gamma);
        let (gam1, gam2, gam3) = (1.0 + g1 * g2, 1.0 + g2 * g3, 1.0 + g3 * g4);
        let b1 = 1.0 / (1.0 + g - g * g2);
        let b2 = 1.0 / (1.0 + g - 0.5 * g * g3);
        let b3 = 1.0 / (1.0 + g - 0.5 * g * g4);
        let b4 = 1.0 / (1.0 + g);
        let (d1, d2, d3) = (g, 0.5 * g, 0.5 * g);
        let f4 = s4 * b4;
        let f3 = (f4 * d3 + s3) * b3;
        let f2 = (f3 * d2 + s2) * b2;
        let f1 = (f2 * d1 + s1) * b1;
        // (the library feeds the third stage's gain to the second's term too)
        let u = (x - f4 * sg4 * k - f3 * sg3 * k - f2 * sg3 * k - f1 * sg1 * k) * alpha0;
        let x1 = u * gam1 + f2 + f1 * g2;
        let y1 = (x1 - s1) * alpha + s1;
        let x2 = (y1 * gam2 + f3 + f2 * g3) * 0.5;
        let y2 = (x2 - s2) * alpha + s2;
        let x3 = (y2 * gam3 + f4 + f3 * g4) * 0.5;
        let y3 = (x3 - s3) * alpha + s3;
        let x4 = y3 * 0.5;
        let y = (x4 - s4) * alpha + s4;
        self.s = [(x1 - s1) * alpha * 2.0 + s1, (x2 - s2) * alpha * 2.0 + s2, (x3 - s3) * alpha * 2.0 + s3, (x4 - s4) * alpha * 2.0 + s4];
        y
    }

    /// The 35 chip's low pass: a Sallen-Key with the feedback through a saturator.
    fn scream_lp(&mut self, x: f32, g: f32, k: f32) -> f32 {
        let [s1, s2, s3, _] = self.s;
        let big_g = g / (1.0 + g);
        let alpha = big_g;
        let b3 = (k - k * big_g) / (1.0 + g);
        let b2 = -1.0 / (1.0 + g);
        let alpha0 = 1.0 / (1.0 - k * big_g + k * big_g * big_g);
        let y1 = (x - s1) * alpha + s1;
        let u = (y1 + s3 * b3 + s2 * b2) * alpha0;
        let y = (u - s3) * alpha + s3;
        self.s[0] = (x - s1) * alpha * 2.0 + s1;
        self.s[2] = (u - s3) * alpha * 2.0 + s3;
        self.s[1] = (tanh(y) * k - s2) * alpha * 2.0 + s2;
        y
    }

    /// The 35 chip's high pass.
    fn scream_hp(&mut self, x: f32, g: f32, k: f32) -> f32 {
        let [s1, s2, s3, _] = self.s;
        let big_g = g / (1.0 + g);
        let alpha = big_g;
        let b3 = 1.0 / (1.0 + g);
        let b2 = -big_g / (1.0 + g);
        let alpha0 = 1.0 / (1.0 - k * big_g + k * big_g * big_g);
        let lp1 = (x - s1) * alpha + s1;
        let hp1 = x - lp1;
        let u = (hp1 + s3 * b3 + s2 * b2) * alpha0;
        let fb = tanh(u) * k;
        let lp2 = (fb - s2) * alpha + s2;
        let hp2 = fb - lp2;
        self.s[0] = (x - s1) * alpha * 2.0 + s1;
        self.s[1] = (fb - s2) * alpha * 2.0 + s2;
        self.s[2] = (hp2 - s3) * alpha * 2.0 + s3;
        u
    }

    /// The multimode state-variable filter, its band saturating: (lp, bp, hp, notch).
    fn multimode(&mut self, x: f32, g: f32, q: f32) -> (f32, f32, f32, f32) {
        let [s1, s2, _, _] = self.s;
        let r = 1.0 / (2.0 * q);
        let fbs1 = 2.0 * r + g;
        let alpha0 = 1.0 / (1.0 + 2.0 * r * g + g * g);
        let hp = (x - s2 - s1 * fbs1) * alpha0;
        let bp = cubic(hp * g + s1);
        let lp = bp * g + s2;
        self.s[0] = hp * g + bp;
        self.s[1] = bp * g * 2.0 + s2;
        (lp, bp, hp, lp + hp)
    }

    /// The clean Sallen-Key: (lp, bp, hp).
    fn clean(&mut self, x: f32, g: f32, q: f32) -> (f32, f32, f32) {
        let [s1, s2, _, _] = self.s;
        let r = 1.0 / (2.0 * q);
        let fbs1 = 2.0 * r + g;
        let alpha0 = 1.0 / (1.0 + 2.0 * r * g + g * g);
        let hp = (x - s2 - s1 * fbs1) * alpha0;
        let bp = hp * g + s1;
        let s1n = hp * 2.0 * g + s1;
        let lp = s1n * g + s2;
        self.s[0] = s1n;
        self.s[1] = bp * 2.0 * g + s2;
        (lp, bp, hp)
    }

    fn run(&mut self, model: usize, x: f32, fc: f32, res: f32, sr: f32) -> f32 {
        let w = warp(fc, sr);
        match model {
            0 => self.ladder(x, w, 3.98 * res) * (1.0 + 1.6 * res),
            1 => self.half_ladder(x, w, 1.98 * res) * (1.0 + 0.6 * res),
            2 => {
                let nf = ((fc / 2.0).log10() - 1.0) / 3.0;
                let k = (17.0 - nf.clamp(0.0, 1.0).powi(10) * 9.7) * res * 0.97;
                self.diode(x, w, k) * (2.2 + 1.4 * res)
            }
            3 => self.scream_lp(x, w, 1.99 * res),
            4 => self.scream_hp(x, w, 1.99 * res),
            5..=8 => {
                let (lp, bp, hp, notch) = self.multimode(x, w, 0.5 + 24.0 * res * res);
                [lp, bp, hp, notch][model - 5]
            }
            _ => {
                let (lp, bp, hp) = self.clean(x, w, 0.5 + 24.0 * res * res);
                [lp, bp, hp][(model - 9).min(2)]
            }
        }
    }
}

pub struct VaFilter {
    sr: f32,
    ch: [Channel; 2],
    model: usize,
    env: f32,
    phase: f32,
    hold: [f32; 2],
    last_phase: [f32; 2],
    rng: u32,
    /// The cutoff the filter is at now (with the envelope and LFO), for the screen.
    pub cutoff_now: f32,
}

impl VaFilter {
    pub fn new(sr: f32) -> Self {
        Self { sr, ch: [Channel::default(); 2], model: 0, env: 0.0, phase: 0.0, hold: [0.0; 2], last_phase: [0.0; 2], rng: 0x5eed_f11e, cutoff_now: 1200.0 }
    }

    fn lfo(&mut self, shape: usize, ph: f32, side: usize) -> f32 {
        match shape {
            0 => (ph * TAU).sin(),
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
            _ => {
                if ph < self.last_phase[side] {
                    self.rng ^= self.rng << 13;
                    self.rng ^= self.rng >> 17;
                    self.rng ^= self.rng << 5;
                    self.hold[side] = (self.rng >> 8) as f32 / (1u32 << 23) as f32 - 1.0;
                }
                self.last_phase[side] = ph;
                self.hold[side]
            }
        }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
        let model = (get(MODEL).round().max(0.0) as usize).min(MODELS.len() - 1);
        if model != self.model {
            self.ch = [Channel::default(); 2];
            self.model = model;
        }
        let cutoff = get(CUTOFF).clamp(20.0, 20000.0);
        let res = get(RESONANCE).clamp(0.0, 1.0);
        let drive = get(DRIVE).clamp(0.0, 1.0);
        let env_amt = get(ENV).clamp(-1.0, 1.0);
        let attack = 1.0 - (-1.0 / (0.004 * sr)).exp();
        let release = 1.0 - (-1.0 / (get(ENV_RELEASE).max(1.0) / 1000.0 * sr)).exp();
        let lfo_amt = get(LFO).clamp(0.0, 1.0);
        let lfo_rate = get(LFO_RATE).max(0.0);
        let shape = (get(LFO_SHAPE).round().max(0.0) as usize).min(LFO_SHAPES.len() - 1);
        let spread = get(STEREO).clamp(0.0, 1.0) * 0.5;
        let mix = get(MIX).clamp(0.0, 1.0);
        let out = 10f32.powf(get(OUTPUT) / 20.0);
        let pre = 1.0 + drive * drive * 15.0;
        // (driven harder, the filter's output is brought back down)
        let post = 1.0 / (1.0 + drive * 1.5);
        let mut i = 0;
        while i < n {
            // The cutoff moves every 16 samples.
            let end = (i + 16).min(n);
            let len = (end - i) as f32;
            let level = (i..end).map(|j| (l[j].abs() + r[j].abs()) * 0.5).fold(0.0, f32::max);
            let k = if level > self.env { 1.0 - (1.0 - attack).powf(len) } else { 1.0 - (1.0 - release).powf(len) };
            self.env += (level - self.env) * k;
            let ph = self.phase;
            self.phase = (self.phase + lfo_rate * len / sr).fract();
            let lfo_l = self.lfo(shape, ph, 0);
            let lfo_r = self.lfo(shape, (ph + spread).fract(), 1);
            let env_oct = env_amt * 6.0 * self.env.min(1.5);
            let fc = [cutoff * 2f32.powf(env_oct + lfo_l * lfo_amt * 4.0), cutoff * 2f32.powf(env_oct + lfo_r * lfo_amt * 4.0)];
            self.cutoff_now = fc[0];
            for j in i..end {
                for (c, buf) in [&mut *l, &mut *r].into_iter().enumerate() {
                    let dry = buf[j];
                    let x = if drive > 0.0 { tanh(dry * pre) / pre.sqrt() } else { dry };
                    let y = self.ch[c].run(model, x, fc[c], res, sr) * post;
                    let y = if y.is_finite() { y } else { 0.0 };
                    buf[j] = (dry + (y - dry) * mix) * out;
                }
            }
            i = end;
        }
        // Never left stuck after something non-finite.
        for c in self.ch.iter_mut() {
            if c.s.iter().any(|v| !v.is_finite()) {
                *c = Channel::default();
            }
        }
    }
}

/// The filter's response (gain) at `f` Hz, roughly, for the screen.
pub fn response(model: usize, cutoff: f32, res: f32, f: f32) -> f32 {
    let x = f / cutoff.max(1.0);
    let q = 0.5 + 24.0 * res * res;
    let pole2 = |q: f32| 1.0 / ((1.0 - x * x).powi(2) + (x / q).powi(2)).sqrt();
    match model {
        0 | 2 => {
            // Four poles with feedback k: |1 / ((1 + jx)^4 + k)|.
            let k = if model == 0 { 3.98 * res } else { 3.6 * res };
            let (re, im) = {
                // (1 + jx)^4
                let (a, b) = (1.0 - x * x, 2.0 * x);
                (a * a - b * b, 2.0 * a * b)
            };
            (1.0 + k * 0.4) / ((re + k).powi(2) + im * im).sqrt()
        }
        1 => pole2(0.5 + 6.0 * res * res),
        3 => pole2(0.6 + 10.0 * res * res),
        4 => x * x * pole2(0.6 + 10.0 * res * res),
        5 | 9 => pole2(q),
        6 | 10 => (x / q) * pole2(q),
        7 | 11 => x * x * pole2(q),
        _ => (1.0 - x * x).abs() * pole2(q),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> FxCtx {
        FxCtx { sr: 48000.0, bpm: 120.0, beat: 0.0, playing: false }
    }

    fn sine(f: f32, sr: f32, n: usize, amp: f32) -> Vec<f32> {
        (0..n).map(|i| (TAU * f * i as f32 / sr).sin() * amp).collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// The gain of a sine at `f` through `model` (after it settles).
    fn gain(model: usize, f: f32, cutoff: f32, res: f32) -> f32 {
        let sr = 48000.0;
        let mut fx = VaFilter::new(sr);
        let mut p = crate::params::defaults(SPECS);
        p[MODEL] = model as f32;
        p[CUTOFF] = cutoff;
        p[RESONANCE] = res;
        p[DRIVE] = 0.0;
        let x = sine(f, sr, 24000, 0.1);
        let (mut l, mut r) = (x.clone(), x.clone());
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            fx.process(&p, a, b, &ctx());
        }
        rms(&l[12000..]) / rms(&x[12000..])
    }

    #[test]
    fn low_passes_pass_lows_and_cut_highs() {
        for model in [0, 1, 2, 3, 5, 9] {
            let low = gain(model, 100.0, 2000.0, 0.1);
            let high = gain(model, 12000.0, 1000.0, 0.1);
            assert!(low > 0.4, "{}: low {low}", MODELS[model]);
            assert!(high < 0.12, "{}: high {high}", MODELS[model]);
        }
    }

    #[test]
    fn high_passes_and_band_passes_do_their_job() {
        for model in [4, 7, 11] {
            assert!(gain(model, 8000.0, 500.0, 0.1) > 0.4, "{}", MODELS[model]);
            assert!(gain(model, 60.0, 2000.0, 0.1) < 0.1, "{}", MODELS[model]);
        }
        for model in [6, 10] {
            let center = gain(model, 1000.0, 1000.0, 0.3);
            assert!(center > 3.0 * gain(model, 100.0, 1000.0, 0.3), "{}", MODELS[model]);
            assert!(center > 3.0 * gain(model, 10000.0, 1000.0, 0.3), "{}", MODELS[model]);
        }
        // The notch takes out its frequency.
        assert!(gain(8, 1000.0, 1000.0, 0.2) < 0.3 * gain(8, 100.0, 1000.0, 0.2));
    }

    #[test]
    fn resonance_peaks_and_full_resonance_stays_bounded() {
        for model in [0, 1, 2, 3, 5, 9] {
            let flat = gain(model, 1000.0, 1000.0, 0.0);
            let peak = gain(model, 1000.0, 1000.0, 0.8);
            assert!(peak > 1.3 * flat, "{}: {flat} -> {peak}", MODELS[model]);
        }
        // Full resonance on silence and on a burst: finite, not running away.
        let sr = 48000.0;
        for model in 0..MODELS.len() {
            let mut fx = VaFilter::new(sr);
            let mut p = crate::params::defaults(SPECS);
            p[MODEL] = model as f32;
            p[RESONANCE] = 1.0;
            p[DRIVE] = 1.0;
            p[LFO] = 1.0;
            p[LFO_RATE] = 5.0;
            p[ENV] = 1.0;
            let mut peak = 0.0f32;
            for b in 0..200 {
                let (mut l, mut r) = (vec![0.0f32; 256], vec![0.0f32; 256]);
                if b < 10 {
                    for (i, v) in l.iter_mut().enumerate() {
                        *v = if i % 50 < 25 { 0.8 } else { -0.8 };
                    }
                    r.copy_from_slice(&l);
                }
                fx.process(&p, &mut l, &mut r, &ctx());
                peak = l.iter().chain(&r).fold(peak, |m, x| m.max(x.abs()));
                assert!(l.iter().chain(&r).all(|x| x.is_finite()), "{}", MODELS[model]);
            }
            assert!(peak < 8.0, "{}: {peak}", MODELS[model]);
        }
    }

    #[test]
    fn the_envelope_opens_the_filter() {
        let sr = 48000.0;
        let run = |env: f32| {
            let mut fx = VaFilter::new(sr);
            let mut p = crate::params::defaults(SPECS);
            p[CUTOFF] = 200.0;
            p[RESONANCE] = 0.0;
            p[ENV] = env;
            let x = sine(3000.0, sr, 12000, 0.5);
            let (mut l, mut r) = (x.clone(), x.clone());
            for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
                fx.process(&p, a, b, &ctx());
            }
            rms(&l[6000..])
        };
        assert!(run(1.0) > 3.0 * run(0.0));
    }
}
