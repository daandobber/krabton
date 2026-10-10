//! Onset detection on the audio input, used to strum when nothing else does
//! (`rings/dsp/onset_detector.h`).
//!
//! Copyright 2015 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::{slope, BLOCK_SIZE};
use crate::filter::NaiveSvf;

#[derive(Clone, Copy, Debug, Default)]
struct ZScorer {
    coefficient: f32,
    mean: f32,
    variance: f32,
}

impl ZScorer {
    fn init(&mut self, cutoff: f32) {
        self.coefficient = cutoff;
        self.mean = 0.0;
        self.variance = 0.0;
    }

    #[inline]
    fn update(&mut self, sample: f32) -> f32 {
        let centered = sample - self.mean;
        self.mean += self.coefficient * centered;
        self.variance += self.coefficient * (centered * centered - self.variance);
        centered
    }

    #[inline]
    fn test(&mut self, sample: f32, threshold: f32, absolute_threshold: f32) -> bool {
        let value = self.update(sample);
        value > self.variance.sqrt() * threshold && value > absolute_threshold
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Compressor {
    attack: f32,
    decay: f32,
    level: f32,
    skew: f32,
}

impl Compressor {
    fn init(&mut self, attack: f32, decay: f32, max_gain: f32) {
        self.attack = attack;
        self.decay = decay;
        self.level = 0.0;
        self.skew = 1.0 / max_gain;
    }

    fn process(&mut self, input: &[f32], out: &mut [f32]) {
        let mut level = self.level;
        for (o, &x) in out.iter_mut().zip(input) {
            slope(&mut level, x.abs(), self.attack, self.decay);
            *o = x / (self.skew + level);
        }
        self.level = level;
    }
}

#[derive(Clone, Debug, Default)]
pub struct OnsetDetector {
    compressor: Compressor,
    low_mid_filter: NaiveSvf,
    mid_high_filter: NaiveSvf,
    attack: [f32; 3],
    decay: [f32; 3],
    energy: [f32; 3],
    envelope: [f32; 3],
    onset_df: f32,
    bands: [[f32; BLOCK_SIZE]; 3],
    z_df: ZScorer,
    inhibit_threshold: f32,
    inhibit_decay: f32,
    inhibit_time: i32,
    inhibit_counter: i32,
}

impl OnsetDetector {
    pub fn init(&mut self, low: f32, low_mid: f32, mid_high: f32, decimated_sr: f32, ioi_time: f32) {
        let ioi_f = 1.0 / (ioi_time * decimated_sr);
        self.compressor.init(ioi_f * 10.0, ioi_f * 0.05, 40.0);

        self.low_mid_filter = NaiveSvf::default();
        self.mid_high_filter = NaiveSvf::default();
        self.low_mid_filter.set_f_q(low_mid, 0.5);
        self.mid_high_filter.set_f_q(mid_high, 0.5);

        self.attack = [low_mid; 3];
        self.decay = [low * 0.25; 3];
        self.envelope = [0.0; 3];
        self.energy = [0.0; 3];

        self.z_df.init(ioi_f * 0.05);

        self.inhibit_time = (ioi_time * decimated_sr) as i32;
        self.inhibit_decay = 1.0 / (ioi_time * decimated_sr);
        self.inhibit_threshold = 0.0;
        self.inhibit_counter = 0;
        self.onset_df = 0.0;
    }

    /// One block of input (at most [`BLOCK_SIZE`] samples).
    pub fn process(&mut self, samples: &[f32]) -> bool {
        let size = samples.len().min(BLOCK_SIZE);
        let samples = &samples[..size];

        // Automatic gain control.
        let [b0, b1, b2] = &mut self.bands;
        self.compressor.process(samples, &mut b0[..size]);

        // Quick and dirty filter bank - split the signal in three bands.
        for j in 0..size {
            let (low, high) = self.mid_high_filter.split(b0[j]);
            b1[j] = low;
            b2[j] = high;
        }
        for j in 0..size {
            let (low, high) = self.low_mid_filter.split(b1[j]);
            b0[j] = low;
            b1[j] = high;
        }

        // Compute low-pass energy and onset detection function
        // (derivative of energy) in each band.
        let mut onset_df = 0.0;
        let mut total_energy = 0.0;
        for i in 0..3 {
            let s = &self.bands[i];
            let mut energy = 0.0;
            let mut envelope = self.envelope[i];
            let increment = 4 >> i;
            let mut j = 0;
            while j < size {
                slope(&mut envelope, s[j] * s[j], self.attack[i], self.decay[i]);
                energy += envelope;
                j += increment;
            }
            energy = energy.sqrt() * increment as f32;
            self.envelope[i] = envelope;

            let derivative = energy - self.energy[i];
            onset_df += derivative + derivative.abs();
            self.energy[i] = energy;
            total_energy += energy;
        }

        self.onset_df += 0.05 * (onset_df - self.onset_df);
        let outlier_in_df = self.z_df.test(self.onset_df, 1.0, 0.01);
        let exceeds_energy_threshold = total_energy >= self.inhibit_threshold;
        let not_inhibited = self.inhibit_counter == 0;
        let has_onset = outlier_in_df && exceeds_energy_threshold && not_inhibited;

        if has_onset {
            self.inhibit_threshold = total_energy * 1.5;
            self.inhibit_counter = self.inhibit_time;
        } else {
            self.inhibit_threshold -= self.inhibit_decay * self.inhibit_threshold;
            if self.inhibit_counter > 0 {
                self.inhibit_counter -= 1;
            }
        }
        has_onset
    }
}
