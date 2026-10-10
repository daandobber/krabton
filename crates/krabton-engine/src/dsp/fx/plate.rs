//! Plate: Jon Dattorro's plate reverb ("Effect Design, Part 1", JAES 1997):
//! four input diffusers, then a figure-eight tank of modulated all-passes and
//! delays whose outputs are tapped at many points. Dense, smooth and long;
//! with Freeze it holds forever.

use super::{Allpass, DelayLine};
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

pub const PREDELAY: usize = 0;
pub const SIZE: usize = 1;
pub const DECAY: usize = 2;
pub const DAMPING: usize = 3;
pub const BANDWIDTH: usize = 4;
pub const DIFFUSION: usize = 5;
pub const MODULATION: usize = 6;
pub const FREEZE: usize = 7;
pub const WIDTH: usize = 8;
pub const MIX: usize = 9;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Pre-Delay", "Plate", 0.0, 500.0, 10.0, Unit::Ms),
    ParamSpec::lin("Size", "Plate", 0.0, 1.0, 0.45, Unit::Percent),
    ParamSpec::lin("Decay", "Plate", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::log("Damping", "Tone", 1000.0, 20000.0, 8000.0, Unit::Hz),
    ParamSpec::log("Bandwidth", "Tone", 1000.0, 20000.0, 12000.0, Unit::Hz),
    ParamSpec::lin("Diffusion", "Plate", 0.0, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Modulation", "Plate", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::toggle("Freeze", "Plate", false),
    ParamSpec::lin("Width", "Output", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.3, Unit::Percent),
];

/// The paper's lengths are in samples at this rate.
const RATE: f32 = 29761.0;
const MAX_SCALE: f32 = 1.6;

pub struct Plate {
    pre: DelayLine,
    bw: f32,
    input: [Allpass; 4],
    ap_mod: [Allpass; 2],
    del1: [DelayLine; 2],
    damp: [f32; 2],
    ap2: [Allpass; 2],
    del2: [DelayLine; 2],
    feed: [f32; 2],
    lfo: f32,
}

impl Plate {
    pub fn new(sr: f32) -> Self {
        let k = sr / RATE * MAX_SCALE;
        let len = |n: f32| (n * k) as usize + 64;
        Self {
            pre: DelayLine::new((sr * 0.55) as usize),
            bw: 0.0,
            input: [Allpass::new(len(142.0)), Allpass::new(len(107.0)), Allpass::new(len(379.0)), Allpass::new(len(277.0))],
            ap_mod: [Allpass::new(len(672.0 + 40.0)), Allpass::new(len(908.0 + 40.0))],
            del1: [DelayLine::new(len(4453.0)), DelayLine::new(len(4217.0))],
            damp: [0.0; 2],
            ap2: [Allpass::new(len(1800.0)), Allpass::new(len(2656.0))],
            del2: [DelayLine::new(len(3720.0)), DelayLine::new(len(3163.0))],
            feed: [0.0; 2],
            lfo: 0.0,
        }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = ctx.sr;
        let k = sr / RATE * (0.5 + 1.1 * p[SIZE].clamp(0.0, 1.0));
        let freeze = p[FREEZE] >= 0.5;
        let decay = if freeze { 1.0 } else { 0.2 + 0.79 * p[DECAY].clamp(0.0, 1.0) };
        // One-pole coefficients from cutoffs.
        let pole = |hz: f32| 1.0 - (-TAU * hz.clamp(20.0, 0.45 * sr) / sr).exp();
        let damp = if freeze { 1.0 } else { pole(p[DAMPING]) };
        let bw = pole(p[BANDWIDTH]);
        let diff = p[DIFFUSION].clamp(0.0, 1.0);
        let (d1, d2) = (diff, diff * 0.833);
        let decay_diff2 = (decay + 0.15).clamp(0.25, 0.5);
        let excursion = p[MODULATION].clamp(0.0, 1.0) * 32.0 * sr / RATE;
        let pre = (p[PREDELAY].clamp(0.0, 500.0) * 0.001 * sr).max(1.0);
        let width = p[WIDTH].clamp(0.0, 1.0);
        let mix = p[MIX].clamp(0.0, 1.0);
        let lfo_step = 1.0 / sr;
        let t = |x: f32| x * k;
        for i in 0..n {
            let (xl, xr) = (l[i], r[i]);
            self.pre.write(if freeze { 0.0 } else { 0.5 * (xl + xr) });
            let mut x = self.pre.read(pre);
            self.bw += bw * (x - self.bw);
            x = self.bw;
            x = self.input[0].tick(x, t(142.0), 0.75 * d1);
            x = self.input[1].tick(x, t(107.0), 0.75 * d1);
            x = self.input[2].tick(x, t(379.0), 0.75 * d2);
            x = self.input[3].tick(x, t(277.0), 0.75 * d2);
            self.lfo = super::wrap(self.lfo + lfo_step);
            for side in 0..2 {
                let inp = x + self.feed[1 - side];
                let (ap_len, d1_len, ap2_len, d2_len) = if side == 0 { (672.0, 4453.0, 1800.0, 3720.0) } else { (908.0, 4217.0, 2656.0, 3163.0) };
                let wobble = excursion * (TAU * (self.lfo + 0.25 * side as f32)).sin();
                let a = self.ap_mod[side].tick(inp, t(ap_len) + wobble, -0.7);
                self.del1[side].write(a);
                let b = self.del1[side].read(t(d1_len));
                self.damp[side] += damp * (b - self.damp[side]);
                let c = self.damp[side] * decay;
                let e = self.ap2[side].tick(c, t(ap2_len), decay_diff2);
                self.del2[side].write(e);
                let f = self.del2[side].read(t(d2_len)) * decay;
                self.feed[side] = if f.is_finite() { f } else { 0.0 };
            }
            let (dl, dr) = (&self.del1, &self.del2);
            let wl = dl[1].read(t(266.0)) + dl[1].read(t(2974.0)) - self.ap2[1].line.read(t(1913.0)) + dr[1].read(t(1996.0)) - dl[0].read(t(1990.0)) - self.ap2[0].line.read(t(187.0)) - dr[0].read(t(1066.0));
            let wr = dl[0].read(t(353.0)) + dl[0].read(t(3627.0)) - self.ap2[0].line.read(t(1228.0)) + dr[0].read(t(2673.0)) - dl[1].read(t(2111.0)) - self.ap2[1].line.read(t(335.0)) - dr[1].read(t(121.0));
            let (wl, wr) = (wl * 0.6, wr * 0.6);
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

    fn tail(p: &[f32]) -> (Vec<f32>, f32) {
        let mut pl = Plate::new(48000.0);
        let mut l = vec![0.0f32; 48000 * 6];
        l[100] = 1.0;
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            pl.process(p, cl, cr, &ctx());
        }
        let late = rms(&l[48000 * 2..48000 * 3]);
        (l, late)
    }

    #[test]
    fn a_click_becomes_a_long_smooth_tail() {
        let mut p = crate::params::defaults(SPECS);
        p[MIX] = 1.0;
        let (l, _) = tail(&p);
        assert!(l.iter().all(|x| x.is_finite()) && peak(&l) < 2.0);
        // Longer decay rings longer; the end of a normal tail is quiet.
        let short = tail(&p).1;
        p[DECAY] = 1.0;
        let long = tail(&p).1;
        assert!(long > short * 3.0, "{long} {short}");
        p[DECAY] = 0.3;
        let (l, _) = tail(&p);
        assert!(peak(&l[48000 * 5..]) < 1e-3);
    }

    #[test]
    fn freeze_holds_the_sound() {
        let mut pl = Plate::new(48000.0);
        let mut p = crate::params::defaults(SPECS);
        p[MIX] = 1.0;
        let mut l = tone(300.0, 0.5, 0.5);
        let mut r = l.clone();
        pl.process(&p, &mut l, &mut r, &ctx());
        p[FREEZE] = 1.0;
        let mut l = vec![0.0f32; 48000 * 4];
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            pl.process(&p, cl, cr, &ctx());
        }
        assert!(rms(&l[48000 * 3..]) > 0.01, "{}", rms(&l[48000 * 3..]));
        assert!(peak(&l) < 4.0);
    }
}
