//! Rackverb: the little rack reverbs of the late eighties, gated and reversed
//! rooms, flanges, choruses and delays, run by the programs of those machines
//! (see `krabton_rackverb`). As on them it hears one channel, at their own low
//! rate and 13 bits, and answers in stereo; before and after it the filters
//! that keep that rate from being heard as anything but their darkness.

use super::super::{db_to_gain, FilterMode, Svf};
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};
use crate::scope::DeviceTaps;
use krabton_rackverb::{Machine, FULL, MODELS, RATE};

pub const MODEL: usize = 0;
pub const PROGRAM: usize = 1;
pub const INPUT: usize = 2;
pub const MIX: usize = 3;
pub const OUTPUT: usize = 4;
pub const FEEDBACK: usize = 5;

/// What the meters tell the face: how hard it is driven.
pub const M_INPUT: usize = 0;

pub const MODEL_NAMES: &[&str] = &["Rack I", "Rack Fx", "Rack II"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Model", "Rack", MODEL_NAMES, 2),
    ParamSpec::int("Program", "Rack", 0.0, 99.0, 15.0, Unit::None),
    ParamSpec::lin("Input", "Rack", -24.0, 12.0, 0.0, Unit::Db),
    ParamSpec::lin("Mix", "Rack", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Output", "Rack", -24.0, 12.0, 0.0, Unit::Db),
    ParamSpec::lin("Feedback", "Rack", 0.0, 1.0, 0.0, Unit::Percent),
];

/// Below the machine's half rate.
const CUTOFF: f32 = 10_000.0;
/// An eighth-order Butterworth in four sections.
const QS: [f32; 4] = [0.5098, 0.6013, 0.9000, 2.5629];

#[derive(Clone, Copy, Default)]
struct Steep([Svf; 4]);

impl Steep {
    fn new(sr: f32) -> Self {
        let mut s = Self([Svf::new(); 4]);
        for (f, q) in s.0.iter_mut().zip(QS) {
            f.set_q(CUTOFF.min(sr * 0.45), q, sr);
        }
        s
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        self.0.iter_mut().fold(x, |x, f| f.process(x, FilterMode::Low))
    }
}

pub struct Rackverb {
    machine: Machine,
    sr: f32,
    before: Steep,
    after: [Steep; 2],
    /// Where the machine's next sample falls between two of ours.
    phase: f64,
    input: [f32; 2],
    output: [[f32; 2]; 2],
    wet: [f32; 2],
    gains: [f32; 4],
    fresh: bool,
    pub taps: DeviceTaps,
}

impl Rackverb {
    pub fn new(sr: f32) -> Self {
        Self {
            machine: Machine::new(),
            sr,
            before: Steep::new(sr),
            after: [Steep::new(sr); 2],
            phase: 0.0,
            input: [0.0; 2],
            output: [[0.0; 2]; 2],
            wet: [0.0; 2],
            gains: [0.0; 4],
            fresh: true,
            taps: DeviceTaps::full(),
        }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let model = (p[MODEL].round().max(0.0) as usize).min(MODELS.len() - 1);
        self.machine.select(model, p[PROGRAM].round().max(0.0) as usize);
        let angle = p[MIX].clamp(0.0, 1.0) * std::f32::consts::FRAC_PI_2;
        let to = [db_to_gain(p[INPUT]), angle.cos(), angle.sin() * db_to_gain(p[OUTPUT]), p[FEEDBACK].clamp(0.0, 1.0)];
        if self.fresh {
            self.gains = to;
            self.fresh = false;
        }
        let n = l.len().min(r.len());
        let step = 1.0 / n.max(1) as f32;
        let inc = RATE / self.sr.max(1.0) as f64;
        let mut peak = 0.0f32;
        for i in 0..n {
            let t = (i + 1) as f32 * step;
            let g: [f32; 4] = std::array::from_fn(|k| self.gains[k] + (to[k] - self.gains[k]) * t);
            let (dry_l, dry_r) = (l[i], r[i]);
            let heard = (dry_l + dry_r) * 0.5 * g[0];
            peak = peak.max(heard.abs());
            let x = self.before.process(heard + (self.wet[0] + self.wet[1]) * 0.5 * g[3]);
            self.input = [self.input[1], x];
            self.phase += inc;
            while self.phase >= 1.0 {
                self.phase -= 1.0;
                // Exactly where the machine's sample falls, between our last two.
                let frac = (1.0 - self.phase / inc) as f32;
                let at = self.input[0] + (self.input[1] - self.input[0]) * frac;
                let (a, b) = self.machine.tick((at.clamp(-1.0, 1.0) * FULL) as i16);
                self.output = [[self.output[0][1], a as f32 / FULL], [self.output[1][1], b as f32 / FULL]];
            }
            let frac = self.phase as f32;
            for c in 0..2 {
                let o = self.output[c];
                self.wet[c] = self.after[c].process(o[0] + (o[1] - o[0]) * frac);
            }
            l[i] = dry_l * g[1] + self.wet[0] * g[2];
            r[i] = dry_r * g[1] + self.wet[1] * g[2];
        }
        self.gains = to;
        if let Some(m) = &mut self.taps.meters {
            m.values[M_INPUT] = peak;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn ctx() -> FxCtx {
        FxCtx { sr: SR, bpm: 120.0, beat: 0.0, playing: false, scale: crate::scale::Scale::new(0, crate::scale::ScaleKind::Major), note: None }
    }

    fn run(p: &[f32], seconds: f32, burst: f32) -> (Vec<f32>, Vec<f32>) {
        let mut fx = Rackverb::new(SR);
        let n = (seconds * SR) as usize;
        let mut seed = 7u32;
        let (mut l, mut r): (Vec<f32>, Vec<f32>) = (0..n)
            .map(|i| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let x = if (i as f32) < burst * SR { ((seed >> 16) as f32 / 32768.0 - 1.0) * 0.5 } else { 0.0 };
                (x, x)
            })
            .unzip();
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            fx.process(p, a, b, &ctx());
        }
        (l, r)
    }

    fn params(model: f32, program: f32, mix: f32) -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        (p[MODEL], p[PROGRAM], p[MIX]) = (model, program, mix);
        p
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    #[test]
    fn all_dry_it_leaves_the_sound_alone() {
        let (l, _) = run(&params(2.0, 15.0, 0.0), 0.2, 0.2);
        let (dry, _) = {
            let mut seed = 7u32;
            let v: Vec<f32> = (0..(0.2 * SR) as usize)
                .map(|i| {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    if (i as f32) < 0.2 * SR { ((seed >> 16) as f32 / 32768.0 - 1.0) * 0.5 } else { 0.0 }
                })
                .collect();
            (v.clone(), v)
        };
        for (a, b) in l.iter().zip(&dry) {
            assert!((a - b).abs() < 1e-5);
        }
    }

    #[test]
    fn a_room_rings_on_in_stereo_for_as_long_as_it_says() {
        // Medium Bright 1.1 Sec, all wet.
        let (l, r) = run(&params(2.0, 15.0, 1.0), 2.0, 0.05);
        let at = |x: &[f32], a: f32, b: f32| rms(&x[(a * SR) as usize..(b * SR) as usize]);
        assert!(at(&l, 0.2, 0.4) > 0.005, "it rings: {}", at(&l, 0.2, 0.4));
        assert!(at(&l, 1.6, 1.9) < at(&l, 0.2, 0.4) * 0.3, "and dies away");
        let apart = l.iter().zip(&r).filter(|(a, b)| (*a - *b).abs() > 1e-3).count();
        assert!(apart > 5000, "two sides: {apart}");
        // A short one is gone sooner than a long one.
        let (short, _) = run(&params(2.0, 1.0, 1.0), 2.0, 0.05);
        assert!(at(&short, 0.8, 1.0) < at(&l, 0.8, 1.0));
    }

    #[test]
    fn it_is_dark_as_the_machine_is() {
        let (l, _) = run(&params(2.0, 15.0, 1.0), 1.0, 0.3);
        // What is left above the filters: a sample-to-sample difference far below the level.
        let diff: Vec<f32> = l.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(rms(&diff) < rms(&l) * 0.9, "{} {}", rms(&diff), rms(&l));
    }

    #[test]
    fn every_program_of_every_model_is_stable_through_the_effect() {
        for (m, model) in MODELS.iter().enumerate() {
            for k in (0..model.programs.len()).step_by(7) {
                let (l, r) = run(&params(m as f32, k as f32, 1.0), 0.3, 0.1);
                let top = l.iter().chain(&r).fold(0.0f32, |a, v| a.max(v.abs()));
                assert!(top.is_finite() && top < 8.0, "{m}/{k}: {top}");
            }
        }
    }

    #[test]
    fn feedback_keeps_it_going_longer() {
        let mut fed = params(2.0, 75.0, 1.0);
        fed[FEEDBACK] = 0.7;
        let (l, _) = run(&fed, 2.0, 0.05);
        let (plain, _) = run(&params(2.0, 75.0, 1.0), 2.0, 0.05);
        let tail = (1.2 * SR) as usize;
        assert!(rms(&l[tail..]) >= rms(&plain[tail..]), "{} {}", rms(&l[tail..]), rms(&plain[tail..]));
    }
}
