//! Noise burst exciter for the string models (`rings/dsp/plucker.h`).
//!
//! Copyright 2015 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::Rng;
use crate::filter::{Approx, DelayLine, Svf};

#[derive(Clone, Debug)]
pub struct Plucker {
    svf: Svf,
    comb_filter: DelayLine<256>,
    remaining_samples: usize,
    comb_filter_period: f32,
    comb_filter_gain: f32,
    level: f32,
    rng: Rng,
}

impl Plucker {
    pub fn new(seed: u32) -> Self {
        let mut p = Self {
            svf: Svf::default(),
            comb_filter: DelayLine::default(),
            remaining_samples: 0,
            comb_filter_period: 0.0,
            comb_filter_gain: 0.0,
            level: 1.0,
            rng: Rng::new(seed),
        };
        p.init();
        p
    }

    pub fn init(&mut self) {
        self.svf = Svf::default();
        self.comb_filter.reset();
        self.remaining_samples = 0;
        self.comb_filter_period = 0.0;
    }

    /// Starts a burst of noise one period long, comb-filtered by the pluck
    /// position. `level` scales the burst (the module always plucks at 1;
    /// the port plucks at the note velocity).
    pub fn trigger(&mut self, frequency: f32, cutoff: f32, position: f32, level: f32) {
        let ratio = position * 0.9 + 0.05;
        let mut comb_period = 1.0 / frequency * ratio;
        self.remaining_samples = comb_period.max(0.0) as usize;
        while comb_period >= 255.0 {
            comb_period *= 0.5;
        }
        self.comb_filter_period = comb_period;
        self.comb_filter_gain = (1.0 - position) * 0.8;
        self.svf.set_f_q(cutoff.min(0.499), 1.0, Approx::Dirty);
        self.level = level;
    }

    pub fn process(&mut self, out: &mut [f32]) {
        let comb_gain = self.comb_filter_gain;
        let comb_delay = self.comb_filter_period;
        for o in out.iter_mut() {
            let mut input = 0.0;
            if self.remaining_samples > 0 {
                input = (2.0 * self.rng.float() - 1.0) * self.level;
                self.remaining_samples -= 1;
            }
            *o = input + comb_gain * self.comb_filter.read_linear(comb_delay);
            self.comb_filter.write(*o);
        }
        self.svf.process_lp_in_place(out);
    }
}
