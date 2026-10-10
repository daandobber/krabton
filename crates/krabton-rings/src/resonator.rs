//! Modal resonator (`rings/dsp/resonator.h`, `resonator.cc`): a bank of up
//! to 64 band-pass filters tuned to the partials of a bar, plate or string;
//! odd and even modes go to separate outputs.
//!
//! Copyright 2015 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::{interpolate, SAMPLE_RATE};
use crate::filter::{Approx, CosineOscillator, Ramp, Svf};
use crate::resources::{LUT_4_DECADES, LUT_STIFFNESS};

pub const MAX_MODES: usize = 64;

#[derive(Clone, Debug)]
pub struct Resonator {
    frequency: f32,
    structure: f32,
    brightness: f32,
    position: f32,
    previous_position: f32,
    damping: f32,
    resolution: usize,
    f: [Svf; MAX_MODES],
}

impl Default for Resonator {
    fn default() -> Self {
        let mut r = Self {
            frequency: 0.0,
            structure: 0.0,
            brightness: 0.0,
            position: 0.0,
            previous_position: 0.0,
            damping: 0.0,
            resolution: MAX_MODES,
            f: [Svf::default(); MAX_MODES],
        };
        r.init();
        r
    }
}

impl Resonator {
    pub fn init(&mut self) {
        self.f = [Svf::default(); MAX_MODES];
        self.frequency = 220.0 / SAMPLE_RATE;
        self.structure = 0.25;
        self.brightness = 0.5;
        self.damping = 0.3;
        self.position = 0.999;
        self.previous_position = 0.0;
        self.set_resolution(MAX_MODES);
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
    }

    pub fn set_structure(&mut self, structure: f32) {
        self.structure = structure;
    }

    pub fn set_brightness(&mut self, brightness: f32) {
        self.brightness = brightness;
    }

    pub fn set_damping(&mut self, damping: f32) {
        self.damping = damping;
    }

    pub fn set_position(&mut self, position: f32) {
        self.position = position;
    }

    /// The number of modes; must be even (rounded down).
    pub fn set_resolution(&mut self, resolution: usize) {
        let resolution = resolution - (resolution & 1);
        self.resolution = resolution.min(MAX_MODES);
    }

    fn compute_filters(&mut self) -> usize {
        let mut stiffness = interpolate(&LUT_STIFFNESS, self.structure, 256.0);
        let mut harmonic = self.frequency;
        let mut stretch_factor = 1.0;
        let mut q = 500.0 * interpolate(&LUT_4_DECADES, self.damping, 256.0);
        let mut brightness_attenuation = 1.0 - self.structure;
        // Reduces the range of brightness when structure is very low, to
        // prevent clipping.
        brightness_attenuation *= brightness_attenuation;
        brightness_attenuation *= brightness_attenuation;
        brightness_attenuation *= brightness_attenuation;
        let brightness = self.brightness * (1.0 - 0.2 * brightness_attenuation);
        let mut q_loss = brightness * (2.0 - brightness) * 0.85 + 0.15;
        let q_loss_damping_rate = self.structure * (2.0 - self.structure) * 0.1;
        let mut num_modes = 0;
        for i in 0..MAX_MODES.min(self.resolution) {
            let mut partial_frequency = harmonic * stretch_factor;
            if partial_frequency >= 0.49 {
                partial_frequency = 0.49;
            } else {
                num_modes = i + 1;
            }
            self.f[i].set_f_q(partial_frequency, 1.0 + partial_frequency * q, Approx::Fast);
            stretch_factor += stiffness;
            if stiffness < 0.0 {
                // Make sure that the partials do not fold back into negative
                // frequencies.
                stiffness *= 0.93;
            } else {
                // This helps adding a few extra partials in the highest
                // frequencies.
                stiffness *= 0.98;
            }
            // This prevents the highest partials from decaying too fast.
            q_loss += q_loss_damping_rate * (1.0 - q_loss);
            harmonic += self.frequency;
            q *= q_loss;
        }
        num_modes
    }

    /// Writes the odd modes to `out` and the even ones to `aux`.
    pub fn process(&mut self, input: &[f32], out: &mut [f32], aux: &mut [f32]) {
        let size = input.len();
        let num_modes = self.compute_filters();
        let mut position = Ramp::new(self.previous_position, self.position, size);
        for n in 0..size {
            let mut amplitudes = CosineOscillator::approximate(position.next());
            let x = input[n] * 0.125;
            let mut odd = 0.0;
            let mut even = 0.0;
            amplitudes.start();
            // The C++ code steps two modes at a time: with an odd count, the
            // last pair still runs (its second filter is above Nyquist and
            // clamped, but it is set up).
            let mut i = 0;
            while i < num_modes {
                odd += amplitudes.next() * self.f[i].process_bp(x);
                even += amplitudes.next() * self.f[(i + 1).min(MAX_MODES - 1)].process_bp(x);
                i += 2;
            }
            out[n] = odd;
            aux[n] = even;
        }
        self.previous_position = position.value;
    }
}
