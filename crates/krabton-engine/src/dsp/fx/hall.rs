//! Hall and Shimmer: a large feedback delay network. Eight modulated delay
//! lines mixed by a Householder matrix, each losing highs and lows at its
//! own rate for decays from a room to a minute, after a pre-delay and input
//! diffusion. Shimmer puts a pitch shifter in the loop, so every pass comes
//! back an interval higher (or lower): rising, glittering clouds.

use super::{Allpass, DelayLine, PitchShifter};
use crate::dsp::effects::FxCtx;
use crate::dsp::fast_tanh;
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

pub const PREDELAY: usize = 0;
pub const SIZE: usize = 1;
pub const DECAY: usize = 2;
pub const LOW_DECAY: usize = 3;
pub const DAMPING: usize = 4;
pub const DIFFUSION: usize = 5;
pub const MODULATION: usize = 6;
pub const FREEZE: usize = 7;
pub const WIDTH: usize = 8;
pub const MIX: usize = 9;
/// Shimmer only.
pub const PITCH: usize = 10;
pub const SHIMMER: usize = 11;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Pre-Delay", "Hall", 0.0, 500.0, 20.0, Unit::Ms),
    ParamSpec::lin("Size", "Hall", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::log("Decay", "Hall", 0.3, 60.0, 4.0, Unit::Sec),
    ParamSpec::lin("Low Decay", "Hall", 0.25, 2.0, 1.0, Unit::None),
    ParamSpec::log("Damping", "Tone", 800.0, 20000.0, 6000.0, Unit::Hz),
    ParamSpec::lin("Diffusion", "Hall", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Modulation", "Hall", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::toggle("Freeze", "Hall", false),
    ParamSpec::lin("Width", "Output", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.3, Unit::Percent),
];

pub static SHIMMER_SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Pre-Delay", "Hall", 0.0, 500.0, 30.0, Unit::Ms),
    ParamSpec::lin("Size", "Hall", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::log("Decay", "Hall", 0.3, 60.0, 8.0, Unit::Sec),
    ParamSpec::lin("Low Decay", "Hall", 0.25, 2.0, 0.8, Unit::None),
    ParamSpec::log("Damping", "Tone", 800.0, 20000.0, 7000.0, Unit::Hz),
    ParamSpec::lin("Diffusion", "Hall", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Modulation", "Hall", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::toggle("Freeze", "Hall", false),
    ParamSpec::lin("Width", "Output", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::int("Pitch", "Shimmer", -24.0, 24.0, 12.0, Unit::Semi),
    ParamSpec::lin("Shimmer", "Shimmer", 0.0, 1.0, 0.5, Unit::Percent),
];

const N: usize = 8;
/// Line lengths in ms at size 1 (mutually prime-ish).
const LENGTHS: [f32; N] = [59.3, 71.9, 83.1, 89.7, 103.9, 117.1, 131.3, 149.9];
const MOD_RATES: [f32; N] = [0.11, 0.17, 0.23, 0.29, 0.31, 0.37, 0.41, 0.47];
const DIFFUSERS: [f32; 4] = [4.7, 3.6, 12.7, 9.3];

pub struct Hall {
    pre: [DelayLine; 2],
    diff: [[Allpass; 4]; 2],
    lines: Vec<DelayLine>,
    low: [f32; N],
    damp: [f32; N],
    phases: [f32; N],
    shift: PitchShifter,
    shimmer_lp: f32,
    last: [f32; N],
}

impl Hall {
    pub fn new(sr: f32) -> Self {
        let ms = |m: f32| (m * 0.001 * sr) as usize + 16;
        Self {
            pre: [DelayLine::new(ms(510.0)), DelayLine::new(ms(510.0))],
            diff: [DIFFUSERS.map(|d| Allpass::new(ms(d))), DIFFUSERS.map(|d| Allpass::new(ms(d * 1.13)))],
            // Size 1 is twice the lengths, plus room for the modulation.
            lines: LENGTHS.iter().map(|&m| DelayLine::new(ms(m * 2.0 + 6.0))).collect(),
            low: [0.0; N],
            damp: [0.0; N],
            phases: std::array::from_fn(|i| i as f32 / N as f32),
            shift: PitchShifter::new(sr, 0.2),
            shimmer_lp: 0.0,
            last: [0.0; N],
        }
    }

    /// Runs the hall; `shimmer` is (pitch ratio, amount) for Shimmer.
    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx, shimmer: Option<(f32, f32)>) {
        let n = l.len().min(r.len());
        let sr = ctx.sr;
        let size = 0.35 + 1.65 * p[SIZE].clamp(0.0, 1.0);
        let freeze = p[FREEZE] >= 0.5;
        let t60 = p[DECAY].clamp(0.1, 120.0);
        let low_t60 = t60 * p[LOW_DECAY].clamp(0.1, 4.0);
        let pole = |hz: f32| 1.0 - (-TAU * hz.clamp(20.0, 0.45 * sr) / sr).exp();
        let damp = if freeze { 1.0 } else { pole(p[DAMPING]) };
        let split = pole(250.0);
        let diffusion = p[DIFFUSION].clamp(0.0, 1.0) * 0.7;
        let depth = p[MODULATION].clamp(0.0, 1.0) * 0.0012 * sr;
        let pre = (p[PREDELAY].clamp(0.0, 500.0) * 0.001 * sr).max(1.0);
        let width = p[WIDTH].clamp(0.0, 1.0);
        let mix = p[MIX].clamp(0.0, 1.0);
        let mut lens = [0.0f32; N];
        let mut gains = [(1.0f32, 1.0f32); N];
        for k in 0..N {
            lens[k] = LENGTHS[k] * 0.001 * sr * size;
            // Gain per pass for a 60 dB fall in T60 (highs and lows apart).
            gains[k] = if freeze { (1.0, 1.0) } else { (10f32.powf(-3.0 * lens[k] / (low_t60 * sr)), 10f32.powf(-3.0 * lens[k] / (t60 * sr))) };
        }
        let (ratio, amount) = shimmer.unwrap_or((1.0, 0.0));
        let window = 0.08 * sr;
        let shimmer_tone = pole(5000.0);
        let diff_ms = |d: f32, c: usize| d * 0.001 * sr * if c == 1 { 1.13 } else { 1.0 };
        for i in 0..n {
            let (xl, xr) = (l[i], r[i]);
            let mut inp = [0.0f32; 2];
            for c in 0..2 {
                self.pre[c].write(if freeze { 0.0 } else if c == 0 { xl } else { xr });
                let mut x = self.pre[c].read(pre);
                for (k, ap) in self.diff[c].iter_mut().enumerate() {
                    x = ap.tick(x, diff_ms(DIFFUSERS[k], c), diffusion);
                }
                inp[c] = x;
            }
            // Shimmer: the hall's sound, shifted, goes back in.
            if amount > 0.0 {
                let mono = (self.last[0] + self.last[1] + self.last[2] + self.last[3]) * 0.25;
                let shifted = self.shift.tick(mono, ratio, window);
                self.shimmer_lp += shimmer_tone * (shifted - self.shimmer_lp);
                let s = self.shimmer_lp * amount * 0.7;
                inp[0] += s;
                inp[1] += s;
            }
            // Read, filter and mix the lines (Householder: x - 2/N * sum).
            let mut out = [0.0f32; N];
            let mut sum = 0.0;
            for k in 0..N {
                self.phases[k] = super::wrap(self.phases[k] + MOD_RATES[k] / sr);
                let d = lens[k] + depth * (1.0 + (self.phases[k] * TAU).sin());
                let y = self.lines[k].read(d);
                self.low[k] += split * (y - self.low[k]);
                let (gl, gh) = gains[k];
                let y = self.low[k] * gl + (y - self.low[k]) * gh;
                self.damp[k] += damp * (y - self.damp[k]);
                out[k] = self.damp[k];
                sum += out[k];
            }
            let h = sum * (2.0 / N as f32);
            for k in 0..N {
                let feed = out[k] - h + if k % 2 == 0 { inp[0] } else { inp[1] } * 0.5;
                // A soft limit keeps a frozen or shimmering hall in check.
                let feed = if freeze || amount > 0.0 { fast_tanh(feed) } else { feed };
                self.lines[k].write(if feed.is_finite() { feed } else { 0.0 });
            }
            self.last = out;
            let wl = (out[0] - out[2] + out[4] - out[6]) * 0.6;
            let wr = (out[1] - out[3] + out[5] - out[7]) * 0.6;
            let mid = 0.5 * (wl + wr);
            let side = 0.5 * (wl - wr) * width;
            l[i] = xl * (1.0 - mix) + (mid + side) * mix;
            r[i] = xr * (1.0 - mix) + (mid - side) * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    fn run(p: &[f32], secs: f32, shimmer: Option<(f32, f32)>) -> Vec<f32> {
        let mut h = Hall::new(48000.0);
        let n = (48000.0 * secs) as usize;
        let mut l = vec![0.0f32; n];
        l[..4800].copy_from_slice(&tone(440.0, 0.1, 0.5));
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            h.process(p, cl, cr, &ctx(), shimmer);
        }
        l
    }

    #[test]
    fn decay_sets_how_long_it_rings() {
        let mut p = crate::params::defaults(SPECS);
        p[MIX] = 1.0;
        p[DECAY] = 1.0;
        let short = run(&p, 4.0, None);
        assert!(short.iter().all(|x| x.is_finite()) && peak(&short) < 2.0);
        // About 60 dB down two seconds after the burst.
        let level = |v: &[f32], s: f32| rms(&v[(48000.0 * s) as usize..(48000.0 * (s + 0.25)) as usize]);
        assert!(level(&short, 3.0) < level(&short, 0.2) * 0.01);
        p[DECAY] = 20.0;
        let long = run(&p, 4.0, None);
        assert!(level(&long, 3.0) > level(&long, 0.2) * 0.3, "{} {}", level(&long, 3.0), level(&long, 0.2));
    }

    #[test]
    fn shimmer_rises_an_octave() {
        let mut p = crate::params::defaults(SHIMMER_SPECS);
        p[MIX] = 1.0;
        let out = run(&p, 3.0, Some((2.0, 1.0)));
        assert!(out.iter().all(|x| x.is_finite()) && peak(&out) < 4.0);
        // The octave above grows into the tail.
        let tail = &out[48000 * 2..48000 * 2 + 9600];
        let corr = |f: f32| {
            let w = f * TAU / 48000.0;
            let (s, c) = tail.iter().enumerate().fold((0.0f32, 0.0f32), |(s, c), (i, x)| (s + x * (w * i as f32).sin(), c + x * (w * i as f32).cos()));
            (s * s + c * c).sqrt()
        };
        assert!(corr(880.0) > corr(600.0) * 3.0, "{} {}", corr(880.0), corr(600.0));
    }

    #[test]
    fn frozen_it_stays_bounded() {
        let mut p = crate::params::defaults(SPECS);
        p[MIX] = 1.0;
        p[FREEZE] = 1.0;
        let out = run(&p, 6.0, None);
        assert!(out.iter().all(|x| x.is_finite()) && peak(&out) < 4.0);
    }
}
