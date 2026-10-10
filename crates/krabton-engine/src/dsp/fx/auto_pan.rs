//! Auto Pan: an LFO on the level of each side. In phase it is a tremolo;
//! half a cycle apart it pans from side to side.

use super::{Lfo, LFO_SYNC};
use crate::dsp::effects::FxCtx;
use crate::dsp::Rng;
use crate::params::{ParamSpec, Unit};

pub const AMOUNT: usize = 0;
pub const RATE: usize = 1;
pub const SYNC: usize = 2;
pub const SHAPE: usize = 3;
pub const PHASE: usize = 4;

pub const SHAPES: &[&str] = &["Sine", "Triangle", "Square", "Saw Down", "Random"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Amount", "Auto Pan", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::log("Rate", "LFO", 0.05, 30.0, 4.0, Unit::Hz),
    ParamSpec::choice("Sync", "LFO", LFO_SYNC, 0),
    ParamSpec::choice("Shape", "LFO", SHAPES, 0),
    ParamSpec::int("Phase", "Auto Pan", 0.0, 360.0, 180.0, Unit::None),
];

pub struct AutoPan {
    lfo: Lfo,
    phases: Vec<f32>,
    /// Smoothed gains (squares and random steps without clicks).
    gain: [f32; 2],
    rng: Rng,
    held: [f32; 2],
    last_phase: [f32; 2],
}

impl AutoPan {
    pub fn new() -> Self {
        Self { lfo: Lfo::default(), phases: vec![0.0; 4096], gain: [1.0; 2], rng: Rng::new(0x9A11), held: [0.0; 2], last_phase: [0.0; 2] }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len()).min(self.phases.len());
        let amount = p[AMOUNT].clamp(0.0, 1.0);
        let shape = p[SHAPE].round().max(0.0) as usize;
        let offset = p[PHASE].clamp(0.0, 360.0) / 360.0;
        let smooth = 1.0 - (-1.0 / (0.002 * ctx.sr)).exp();
        self.lfo.run(p[RATE], p[SYNC], ctx, &mut self.phases[..n]);
        for i in 0..n {
            for ch in 0..2 {
                let ph = super::wrap(self.phases[i] + if ch == 1 { offset } else { 0.0 });
                // A new random level each cycle.
                if ph < self.last_phase[ch] {
                    self.held[ch] = self.rng.bipolar();
                }
                self.last_phase[ch] = ph;
                let v = match shape {
                    1 => 1.0 - 4.0 * (ph - 0.5).abs(),
                    2 => {
                        if ph < 0.5 {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    3 => 1.0 - 2.0 * ph,
                    4 => self.held[ch],
                    _ => super::sine(ph),
                };
                let target = 1.0 - amount * (0.5 - 0.5 * v);
                self.gain[ch] += (target - self.gain[ch]) * smooth;
            }
            l[i] *= self.gain[0];
            r[i] *= self.gain[1];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    #[test]
    fn half_a_cycle_apart_it_pans() {
        let mut a = AutoPan::new();
        let p = crate::params::defaults(SPECS);
        let mut l = vec![0.5; 4800];
        let mut r = vec![0.5; 4800];
        a.process(&p, &mut l, &mut r, &ctx());
        // At 4 Hz, a quarter cycle in: one side up, the other down.
        let i = 3000;
        assert!((l[i] - r[i]).abs() > 0.3, "{} {}", l[i], r[i]);
        assert!(peak(&l) <= 0.5001 && peak(&r) <= 0.5001);
    }
}
