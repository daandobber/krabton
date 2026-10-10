//! Phaser: a chain of first-order all-pass filters swept by an LFO, mixed
//! with the dry signal for moving notches, with feedback and a stereo offset.

use super::{sine, Lfo, LFO_SYNC};
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};

pub const RATE: usize = 0;
pub const SYNC: usize = 1;
pub const DEPTH: usize = 2;
pub const CENTER: usize = 3;
pub const FEEDBACK: usize = 4;
pub const STAGES: usize = 5;
pub const STEREO: usize = 6;
pub const MIX: usize = 7;

const STAGE_COUNTS: [usize; 5] = [2, 4, 6, 8, 12];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::log("Rate", "LFO", 0.02, 10.0, 0.5, Unit::Hz),
    ParamSpec::choice("Sync", "LFO", LFO_SYNC, 0),
    ParamSpec::lin("Depth", "LFO", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::log("Center", "Phaser", 100.0, 8000.0, 800.0, Unit::Hz),
    ParamSpec::lin("Feedback", "Phaser", -0.95, 0.95, 0.5, Unit::Percent),
    ParamSpec::choice("Stages", "Phaser", &["2", "4", "6", "8", "12"], 1),
    ParamSpec::lin("Stereo", "Output", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.5, Unit::Percent),
];

#[derive(Default)]
pub struct Phaser {
    lfo: Lfo,
    /// Per channel and stage: last input and output.
    x1: [[f32; 12]; 2],
    y1: [[f32; 12]; 2],
    last: [f32; 2],
    phases: Vec<f32>,
}

impl Phaser {
    pub fn new() -> Self {
        Self { phases: vec![0.0; 4096], ..Default::default() }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len()).min(self.phases.len());
        let stages = STAGE_COUNTS[(p[STAGES].round().max(0.0) as usize).min(STAGE_COUNTS.len() - 1)];
        let (depth, center, fb, mix) = (p[DEPTH].clamp(0.0, 1.0), p[CENTER].clamp(20.0, 0.4 * ctx.sr), p[FEEDBACK].clamp(-0.95, 0.95), p[MIX].clamp(0.0, 1.0));
        let offset = 0.5 * p[STEREO].clamp(0.0, 1.0);
        self.lfo.run(p[RATE], p[SYNC], ctx, &mut self.phases[..n]);
        for i in 0..n {
            for ch in 0..2 {
                let ph = self.phases[i] + if ch == 1 { offset } else { 0.0 };
                // Three octaves either way at full depth.
                let fc = (center * 2f32.powf(3.0 * depth * sine(ph))).clamp(20.0, 0.45 * ctx.sr);
                let t = (std::f32::consts::PI * fc / ctx.sr).tan();
                let a = (t - 1.0) / (t + 1.0);
                let x = if ch == 0 { l[i] } else { r[i] };
                let mut v = x + fb * self.last[ch];
                for s in 0..stages {
                    let y = a * v + self.x1[ch][s] - a * self.y1[ch][s];
                    self.x1[ch][s] = v;
                    self.y1[ch][s] = y;
                    v = y;
                }
                if !v.is_finite() {
                    v = 0.0;
                    self.x1[ch] = [0.0; 12];
                    self.y1[ch] = [0.0; 12];
                }
                self.last[ch] = v;
                // Full mix is half dry, half shifted: the deepest notches.
                let out = x * (1.0 - 0.5 * mix) + v * 0.5 * mix;
                if ch == 0 {
                    l[i] = out;
                } else {
                    r[i] = out;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    #[test]
    fn notches_move_and_it_stays_bounded() {
        let mut ph = Phaser::new();
        let mut p = crate::params::defaults(SPECS);
        p[MIX] = 1.0;
        p[FEEDBACK] = 0.95;
        let mut rng = crate::dsp::Rng::new(3);
        let mut l: Vec<f32> = (0..48000).map(|_| rng.bipolar() * 0.5).collect();
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
            ph.process(&p, cl, cr, &ctx());
        }
        assert!(l.iter().chain(&r).all(|x| x.is_finite()));
        assert!(peak(&l) < 4.0 && rms(&l) > 0.05);
    }
}
