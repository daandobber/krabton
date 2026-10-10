//! Transient: makes the attack of sounds harder or softer and their
//! sustain longer or shorter, from the difference between fast and slow
//! envelope followers (level independent, no threshold).

use super::Follower;
use crate::dsp::db_to_gain;
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};

pub const ATTACK: usize = 0;
pub const SUSTAIN: usize = 1;
pub const OUTPUT: usize = 2;
pub const MIX: usize = 3;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Attack", "Transient", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Sustain", "Transient", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Output", "Output", -24.0, 24.0, 0.0, Unit::Db),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];

#[derive(Default)]
pub struct Transient {
    fast: Follower,
    slow: Follower,
    fast2: Follower,
    slow2: Follower,
    sr: f32,
    gain: f32,
}

impl Transient {
    pub fn new() -> Self {
        Self { gain: 1.0, ..Default::default() }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        if self.sr != ctx.sr {
            self.sr = ctx.sr;
            self.fast.set(0.0005, 0.05, ctx.sr);
            self.slow.set(0.015, 0.05, ctx.sr);
            self.fast2.set(0.001, 0.02, ctx.sr);
            self.slow2.set(0.001, 0.3, ctx.sr);
        }
        let n = l.len().min(r.len());
        let (att, sus) = (p[ATTACK].clamp(-1.0, 1.0), p[SUSTAIN].clamp(-1.0, 1.0));
        let out = db_to_gain(p[OUTPUT]);
        let mix = p[MIX].clamp(0.0, 1.0);
        for i in 0..n {
            let x = l[i].abs().max(r[i].abs()) + 1e-6;
            let (f, s) = (self.fast.tick(x), self.slow.tick(x));
            let (f2, s2) = (self.fast2.tick(x), self.slow2.tick(x));
            // Ratios above 1 mark an attack, or a tail.
            let g = (f / s).powf(2.0 * att) * (s2 / f2).powf(2.0 * sus);
            let g = g.clamp(0.06, 16.0);
            self.gain += (g - self.gain) * 0.05;
            let k = (1.0 - mix) + mix * self.gain;
            l[i] *= k * out;
            r[i] *= k * out;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    /// Decaying 200 Hz hits, 4 per second.
    fn hits() -> Vec<f32> {
        (0..48000).map(|i| {
            let t = (i % 12000) as f32 / 48000.0;
            (-t * 20.0).exp() * (i as f32 * 200.0 * std::f32::consts::TAU / 48000.0).sin() * 0.5
        }).collect()
    }

    #[test]
    fn more_sustain_lifts_the_tails() {
        let run = |sus: f32| {
            let mut t = Transient::new();
            let mut p = crate::params::defaults(SPECS);
            p[SUSTAIN] = sus;
            let mut l = hits();
            let mut r = l.clone();
            t.process(&p, &mut l, &mut r, &ctx());
            // The end of the last hit.
            rms(&l[36000 + 6000..48000])
        };
        assert!(run(1.0) > run(0.0) * 1.5, "{} {}", run(1.0), run(0.0));
        assert!(run(-1.0) < run(0.0) * 0.7);
    }
}
