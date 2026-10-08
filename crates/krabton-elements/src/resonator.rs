//! Modal resonator (`elements/dsp/resonator.h`, `resonator.cc`): a bank of
//! band-pass filters tuned to the partials of a bar, plate or string, plus a
//! few banded waveguides for the bow.
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::interpolate;
use crate::filter::{Approx, CosineOscillator, DelayLine, Svf};
use crate::resources::{LUT_4_DECADES, LUT_STIFFNESS};
use crate::SAMPLE_RATE;

pub const MAX_MODES: usize = 64;
pub const MAX_BOWED_MODES: usize = 8;
pub const MAX_DELAY_LINE_SIZE: usize = 1024;

#[derive(Clone, Debug)]
pub struct Resonator {
    frequency: f32,
    geometry: f32,
    brightness: f32,
    position: f32,
    previous_position: f32,
    damping: f32,
    modulation_frequency: f32,
    modulation_offset: f32,
    lfo_phase: f32,
    bow_signal: f32,
    resolution: usize,
    f: [Svf; MAX_MODES],
    f_bow: [Svf; MAX_BOWED_MODES],
    d_bow: Vec<DelayLine<MAX_DELAY_LINE_SIZE>>,
    clock_divider: usize,
}

impl Default for Resonator {
    fn default() -> Self {
        Self {
            frequency: 220.0 / SAMPLE_RATE,
            geometry: 0.25,
            brightness: 0.5,
            position: 0.999,
            previous_position: 0.0,
            damping: 0.3,
            modulation_frequency: 0.0,
            modulation_offset: 0.0,
            lfo_phase: 0.0,
            bow_signal: 0.0,
            resolution: MAX_MODES,
            f: [Svf::default(); MAX_MODES],
            f_bow: [Svf::default(); MAX_BOWED_MODES],
            d_bow: (0..MAX_BOWED_MODES).map(|_| DelayLine::default()).collect(),
            clock_divider: 0,
        }
    }
}

impl Resonator {
    /// Back to the power-on state, keeping the allocations.
    pub fn reset(&mut self) {
        for f in self.f.iter_mut().chain(self.f_bow.iter_mut()) {
            *f = Svf::default();
        }
        for d in self.d_bow.iter_mut() {
            d.reset();
        }
        self.frequency = 220.0 / SAMPLE_RATE;
        self.geometry = 0.25;
        self.brightness = 0.5;
        self.damping = 0.3;
        self.position = 0.999;
        self.previous_position = 0.0;
        self.resolution = MAX_MODES;
        self.bow_signal = 0.0;
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
    }

    pub fn set_geometry(&mut self, geometry: f32) {
        self.geometry = geometry;
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

    pub fn set_resolution(&mut self, resolution: usize) {
        self.resolution = resolution.min(MAX_MODES);
    }

    pub fn set_modulation_frequency(&mut self, f: f32) {
        self.modulation_frequency = f;
    }

    pub fn set_modulation_offset(&mut self, offset: f32) {
        self.modulation_offset = offset;
    }

    #[inline]
    fn bow_table(x: f32, velocity: f32) -> f32 {
        let x = 0.13 * velocity - x;
        let mut bow = x * 6.0;
        bow = bow.abs() + 0.75;
        bow *= bow;
        bow *= bow;
        bow = (0.25 / bow).clamp(0.0025, 0.245);
        x * bow
    }

    fn compute_filters(&mut self) -> usize {
        self.clock_divider = self.clock_divider.wrapping_add(1);
        let mut stiffness = interpolate(&LUT_STIFFNESS, self.geometry, 256.0);
        let mut harmonic = self.frequency;
        let mut stretch_factor = 1.0;
        let mut q = 500.0 * interpolate(&LUT_4_DECADES, self.damping * 0.8, 256.0);
        let mut brightness_attenuation = 1.0 - self.geometry;
        // Reduces the range of brightness when geometry is very low, to
        // prevent clipping.
        brightness_attenuation *= brightness_attenuation;
        brightness_attenuation *= brightness_attenuation;
        brightness_attenuation *= brightness_attenuation;
        let brightness = self.brightness * (1.0 - 0.2 * brightness_attenuation);
        let mut q_loss = brightness * (2.0 - brightness) * 0.85 + 0.15;
        let q_loss_damping_rate = self.geometry * (2.0 - self.geometry) * 0.1;
        let mut num_modes = 0;
        for i in 0..MAX_MODES.min(self.resolution) {
            // Update the first 24 modes every time (2kHz). The higher modes
            // are refreshed every other block (1kHz).
            let update = i <= 24 || (i & 1) == (self.clock_divider & 1);
            let mut partial_frequency = harmonic * stretch_factor;
            if partial_frequency >= 0.49 {
                partial_frequency = 0.49;
            } else {
                num_modes = i + 1;
            }
            if update {
                self.f[i].set_f_q(partial_frequency, 1.0 + partial_frequency * q, Approx::Fast);
                if i < MAX_BOWED_MODES {
                    let mut period = (1.0 / partial_frequency) as usize;
                    while period >= MAX_DELAY_LINE_SIZE {
                        period >>= 1;
                    }
                    self.d_bow[i].set_delay(period);
                    let g = self.f[i].g();
                    self.f_bow[i].set_g_q(g, 1.0 + partial_frequency * 1500.0);
                }
            }
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

    pub fn process(&mut self, bow_strength: &[f32], input: &[f32], center: &mut [f32], sides: &mut [f32]) {
        let size = input.len();
        let num_modes = self.compute_filters();
        let num_banded_wg = MAX_BOWED_MODES.min(num_modes);
        let position_increment = (self.position - self.previous_position) / size as f32;
        for n in 0..size {
            self.lfo_phase += self.modulation_frequency;
            if self.lfo_phase >= 1.0 {
                self.lfo_phase -= 1.0;
            }
            self.previous_position += position_increment;
            let lfo = if self.lfo_phase > 0.5 { 1.0 - self.lfo_phase } else { self.lfo_phase };
            let mut amplitudes = CosineOscillator::approximate(self.previous_position);
            let mut aux_amplitudes = CosineOscillator::approximate(self.modulation_offset + lfo);
            let mut x = input[n] * 0.125;
            let mut sum_center = 0.0;
            let mut sum_side = 0.0;
            for f in self.f[..num_modes].iter_mut() {
                let s = f.process_bp(x);
                sum_center += s * amplitudes.next();
                sum_side += s * aux_amplitudes.next();
            }
            sides[n] = sum_side - sum_center;

            // Bowed modes.
            let mut bow_signal = 0.0;
            x += self.bow_signal;
            amplitudes.start();
            for i in 0..num_banded_wg {
                let mut s = 0.99 * self.d_bow[i].read();
                bow_signal += s;
                s = self.f_bow[i].process_bp_normalized(x + s);
                self.d_bow[i].write(s);
                sum_center += s * amplitudes.next() * 8.0;
            }
            self.bow_signal = Self::bow_table(bow_signal, bow_strength[n]);
            center[n] = sum_center;
        }
    }
}
