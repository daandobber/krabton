//! Smear: a swarm of short echoes smeared into a cave. Eight echoes, each a
//! delay that feeds itself (Reflect: how long they go on; all the way up
//! they ring for ever), are heard through a chain of all-passes that blurs
//! them (Diffuse). Length is how long the swarm is, Drag how far apart its
//! echoes are: turn it while it rings and they slide, bending everything in
//! pitch. Dampen darkens every pass.

use super::verb::{soft, Ap, Line};
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

pub const LENGTH: usize = 0;
pub const DIFFUSE: usize = 1;
pub const DAMPEN: usize = 2;
pub const DRAG: usize = 3;
pub const REFLECT: usize = 4;
pub const MIX: usize = 5;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::log("Length", "Smear", 20.0, 1000.0, 180.0, Unit::Ms),
    ParamSpec::lin("Diffuse", "Smear", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Dampen", "Smear", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Drag", "Smear", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Reflect", "Smear", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.45, Unit::Percent),
];

const HEADS: usize = 8;
const MAX_S: f32 = 1.1;

pub struct Smear {
    sr: f32,
    combs: [[Line; HEADS]; 2],
    lp: [[f32; HEADS]; 2],
    aps: [[Ap; 6]; 2],
    dc: [f32; 2],
    /// How long each echo is (samples), gliding to where it should be.
    heads: [[f32; HEADS]; 2],
    phase: f32,
    started: bool,
}

impl Smear {
    pub fn new(sr: f32) -> Self {
        let len = (MAX_S * sr) as usize + 64;
        Self {
            sr,
            combs: std::array::from_fn(|_| std::array::from_fn(|_| Line::new(len))),
            lp: [[0.0; HEADS]; 2],
            aps: std::array::from_fn(|_| std::array::from_fn(|_| Ap::new((0.25 * sr) as usize))),
            dc: [0.0; 2],
            heads: [[0.0; HEADS]; 2],
            phase: 0.0,
            started: false,
        }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let length = (p[LENGTH].clamp(20.0, 1000.0) * 0.001 * sr).min(self.combs[0][0].max_delay() - 16.0);
        let diffuse = p[DIFFUSE].clamp(0.0, 1.0);
        let damp = {
            let hz = 18000.0 * (700.0f32 / 18000.0).powf(p[DAMPEN].clamp(0.0, 1.0));
            1.0 - (-TAU * hz.min(0.45 * sr) / sr).exp()
        };
        let drag = p[DRAG].clamp(0.0, 1.0);
        let reflect = p[REFLECT].clamp(0.0, 1.0);
        // The top of the knob is where it rings long; all the way: for ever.
        let g = if reflect >= 0.995 { 1.0 } else { 0.98 * (1.0 - (1.0 - reflect).powf(1.5)) };
        let mix = p[MIX].clamp(0.0, 1.0);
        // The echoes: from the full length down, as far apart as Drag says.
        let mut want = [[0.0f32; HEADS]; 2];
        for c in 0..2 {
            for k in 0..HEADS {
                let t = (k as f32 + if c == 1 { 0.5 } else { 0.0 }) / HEADS as f32;
                // A little irregular, so they never fall in step.
                let nudge = 1.0 + [0.0, 0.013, -0.009, 0.017, -0.013, 0.007, -0.017, 0.011][k];
                want[c][k] = (length * (1.0 - drag * 0.9 * t) * nudge).max(8.0);
            }
        }
        if !self.started {
            self.heads = want;
            self.started = true;
        }
        let glide = 1.0 - (-1.0 / (0.25 * sr)).exp();
        let ap_len = |k: usize, c: usize| (length / sr).clamp(0.02, 1.0) * [0.031, 0.047, 0.067, 0.089, 0.113, 0.149][k] * sr * if c == 1 { 1.07 } else { 1.0 };
        let ap_g = 0.3 + 0.45 * diffuse;
        let dc_k = 1.0 - (-TAU * 20.0 / sr).exp();
        let norm = 1.0 / (HEADS as f32).sqrt();
        for i in 0..n {
            let x = [l[i], r[i]];
            self.phase = (self.phase + 0.23 / sr).fract();
            let wob = (TAU * self.phase).sin() * 0.0006 * sr;
            let mut y = [0.0f32; 2];
            for c in 0..2 {
                let mut s = 0.0;
                for k in 0..HEADS {
                    self.heads[c][k] += (want[c][k] - self.heads[c][k]) * glide;
                    let w = if k % 2 == 0 { wob } else { -wob };
                    let line = &mut self.combs[c][k];
                    let v = line.read(self.heads[c][k] + w);
                    // Darker every pass.
                    self.lp[c][k] += damp * (v - self.lp[c][k]);
                    line.write(soft(x[c] + g * self.lp[c][k]));
                    s += v * if k % 3 == 0 { -1.0 } else { 1.0 };
                }
                s *= norm;
                for (k, ap) in self.aps[c].iter_mut().enumerate() {
                    s = ap.tick(s, ap_len(k, c), ap_g);
                }
                self.dc[c] += dc_k * (s - self.dc[c]);
                y[c] = s - self.dc[c];
            }
            l[i] = x[0] * (1.0 - mix) + y[0] * mix;
            r[i] = x[1] * (1.0 - mix) + y[1] * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    fn run(secs: f32, f: impl Fn(&mut Vec<f32>)) -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        p[MIX] = 1.0;
        f(&mut p);
        let mut s = Smear::new(48000.0);
        let mut l = vec![0.0; (secs * 48000.0) as usize];
        for v in l[..480].iter_mut() {
            *v = 0.5;
        }
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            s.process(&p, cl, cr, &ctx());
        }
        l
    }

    #[test]
    fn it_smears_and_dies_away() {
        let l = run(4.0, |p| p[REFLECT] = 0.5);
        assert!(l.iter().all(|v| v.is_finite()));
        assert!(rms(&l[4800..24000]) > 1e-3);
        assert!(rms(&l[3 * 48000..]) < 0.02 * rms(&l[4800..24000]));
    }

    #[test]
    fn full_reflect_rings_on_but_stays_bounded() {
        let l = run(8.0, |p| {
            p[REFLECT] = 1.0;
            p[DAMPEN] = 0.0;
        });
        assert!(l.iter().all(|v| v.is_finite() && v.abs() < 4.0));
        assert!(rms(&l[7 * 48000..]) > 1e-3);
    }
}
