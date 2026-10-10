//! The spectral processing of the "spectral madness" mode: magnitudes are
//! recorded into a few "textures", replayed, warped, shifted, quantized and
//! glitched; phases are advanced or randomized
//! (`clouds/dsp/pvoc/frame_transformation.h`, `frame_transformation.cc`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::{crossfade, fast_atan2r, integral_fractional, interpolate, semitones_to_ratio, Random};
use crate::parameters::Parameters;
use crate::resources::LUT_SIN;

pub const MAX_NUM_TEXTURES: usize = 7;
pub const HIGH_FREQUENCY_TRUNCATION: usize = 16;

const WARP_POLYNOMIALS: [[f32; 4]; 6] = [
    [10.5882, -14.8824, 5.29412, 0.0],
    [-7.3333, 9.0, -1.79167, 0.125],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.5, 0.5, 0.0],
    [-7.3333, 9.5, -2.416667, 0.25],
    [-7.3333, 9.5, -2.416667, 0.25],
];

#[inline]
fn fast_p2r(magnitude: f32, angle: u16) -> (f32, f32) {
    let angle = (angle >> 6) as usize;
    (magnitude * LUT_SIN[angle + 256], magnitude * LUT_SIN[angle])
}

pub struct FrameTransformation {
    fft_size: usize,
    /// Magnitude textures in use (the module gives the last block of its
    /// memory to the phase arrays).
    num_textures: usize,
    size: usize,
    textures: Box<[f32]>,
    phases: Box<[u16]>,
    phases_delta: Box<[u16]>,
    synthesis_phase: Box<[u32]>,
    shift_temp: Box<[f32]>,
    glitch_algorithm: i32,
}

impl FrameTransformation {
    pub fn new(max_fft_size: usize) -> Self {
        let max_size = (max_fft_size >> 1) - HIGH_FREQUENCY_TRUNCATION;
        Self {
            fft_size: max_fft_size,
            num_textures: MAX_NUM_TEXTURES - 1,
            size: max_size,
            textures: vec![0.0; MAX_NUM_TEXTURES * max_size].into_boxed_slice(),
            phases: vec![0; max_size].into_boxed_slice(),
            phases_delta: vec![0; max_size].into_boxed_slice(),
            synthesis_phase: vec![0; max_size].into_boxed_slice(),
            shift_temp: vec![0.0; max_size + 2].into_boxed_slice(),
            glitch_algorithm: 0,
        }
    }

    /// `num_textures` counts the block used for the phases, like the module.
    pub fn init(&mut self, fft_size: usize, num_textures: usize) {
        let size = ((fft_size >> 1) - HIGH_FREQUENCY_TRUNCATION).min(self.phases.len());
        self.fft_size = fft_size;
        self.size = size;
        self.num_textures = num_textures.clamp(2, MAX_NUM_TEXTURES) - 1;
        self.glitch_algorithm = 0;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.textures[..self.num_textures * self.size].fill(0.0);
    }

    #[inline]
    fn texture_range(&self, index: usize) -> std::ops::Range<usize> {
        let index = index.min(self.num_textures - 1);
        index * self.size..(index + 1) * self.size
    }

    pub fn process(&mut self, parameters: &Parameters, fft_out: &mut [f32], ifft_in: &mut [f32], rng: &mut Random) {
        let half = self.fft_size >> 1;
        fft_out[0] = 0.0;
        fft_out[half] = 0.0;

        let freeze = parameters.freeze;
        let glitch = parameters.gate;
        let pitch_ratio = semitones_to_ratio(parameters.pitch);

        if !freeze {
            self.rectangular_to_polar(fft_out);
            self.store_magnitudes(fft_out, parameters.position, parameters.spectral.refresh_rate, rng);
        }
        self.replay_magnitudes(ifft_in, parameters.position);
        // The module reads one or two values past the magnitudes here (left
        // over from the FFT); read zeros instead.
        ifft_in[self.size] = 0.0;
        ifft_in[self.size + 1] = 0.0;
        self.warp_magnitudes(ifft_in, fft_out, parameters.spectral.warp);
        self.shift_magnitudes(fft_out, ifft_in, pitch_ratio);
        if glitch {
            self.add_glitch(ifft_in, rng);
        }
        self.quantize_magnitudes(ifft_in, parameters.spectral.quantization);
        self.set_phases(parameters.spectral.phase_randomization, pitch_ratio, rng);
        self.polar_to_rectangular(ifft_in);

        if !glitch {
            // Decide on which glitch algorithm will be used next time... if
            // glitch is enabled on the next frame!
            self.glitch_algorithm = (rng.get_sample() & 3) as i32;
        }

        ifft_in[0] = 0.0;
        ifft_in[half] = 0.0;
    }

    fn rectangular_to_polar(&mut self, fft_data: &mut [f32]) {
        let half = self.fft_size >> 1;
        for i in 1..self.size {
            let (angle, magnitude) = fast_atan2r(fft_data[half + i], fft_data[i]);
            fft_data[i] = magnitude;
            self.phases_delta[i] = angle.wrapping_sub(self.phases[i]);
            self.phases[i] = angle;
        }
    }

    fn set_phases(&mut self, phase_randomization: f32, pitch_ratio: f32, rng: &mut Random) {
        let size = self.size;
        for i in 0..size {
            self.synthesis_phase[i] = self.phases[i] as u32;
            // float -> uint16 as the ARM core does it: to 32 bits, truncated.
            let advance = (self.phases_delta[i] as f32 * pitch_ratio) as u32 as u16;
            self.phases[i] = self.phases[i].wrapping_add(advance);
        }
        let mut r = phase_randomization;
        r = ((r - 0.05) * 1.06).clamp(0.0, 1.0);
        r *= r;
        let amount = (r * 32768.0) as i32;
        for phase in self.synthesis_phase[..size].iter_mut() {
            let offset = (rng.get_sample() as i32 * amount) >> 14;
            *phase = phase.wrapping_add(offset as u32);
        }
    }

    fn polar_to_rectangular(&mut self, fft_data: &mut [f32]) {
        let half = self.fft_size >> 1;
        for i in 1..self.size {
            let (re, im) = fast_p2r(fft_data[i], self.synthesis_phase[i] as u16);
            fft_data[i] = re;
            fft_data[half + i] = im;
        }
        for i in self.size..half {
            fft_data[i] = 0.0;
            fft_data[half + i] = 0.0;
        }
    }

    fn add_glitch(&mut self, x: &mut [f32], rng: &mut Random) {
        let size = self.size;
        match self.glitch_algorithm {
            0 => {
                // Spectral hold and blow: create trails.
                let mut held = 0.0;
                for v in x[..size].iter_mut() {
                    if (rng.get_sample() as i32) & 15 == 0 {
                        held = *v;
                    }
                    *v = held;
                    held *= 1.01;
                }
            }
            1 => {
                // Spectral shift up with aliasing.
                let factor = 1.0 + ((rng.get_sample() as i32) & 7) as f32 / 4.0;
                let mut source = 0.0f32;
                for i in 0..size {
                    source += factor;
                    if source >= size as f32 {
                        source = 0.0;
                    }
                    x[i] = x[source as usize];
                }
            }
            2 => {
                // Kill largest harmonic and boost second largest.
                let i = argmax(&x[..size]);
                x[i] = 0.0;
                let i = argmax(&x[..size]);
                x[i] *= 8.0;
            }
            3 => {
                // Nasty high-pass.
                for (i, v) in x[..size].iter_mut().enumerate() {
                    if (rng.get_sample() as i32) & 15 == 0 {
                        *v *= i as f32 / 16.0;
                    }
                }
            }
            _ => {}
        }
    }

    fn quantize_magnitudes(&mut self, xf_polar: &mut [f32], amount: f32) {
        let size = self.size;
        if amount <= 0.48 {
            let amount = amount * 2.0;
            let scale_down =
                0.5 * semitones_to_ratio(-108.0 * (1.0 - amount * amount)) / self.fft_size as f32;
            let scale_up = 1.0 / scale_down;
            for v in xf_polar[..size].iter_mut() {
                *v = scale_up * ((scale_down * *v) as i32) as f32;
            }
        } else if amount >= 0.52 {
            let amount = (amount - 0.52) * 2.0;
            let norm = xf_polar[..size].iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let inv_norm = 1.0 / (norm + 0.0001);
            for v in xf_polar[1..size].iter_mut() {
                let x = *v * inv_norm;
                let warped = 4.0 * x * (1.0 - x) * (1.0 - x) * (1.0 - x);
                *v = (x + (warped - x) * amount) * norm;
            }
        }
    }

    fn warp_magnitudes(&self, source: &[f32], xf_polar: &mut [f32], amount: f32) {
        let size = self.size;
        let bin_width = 1.0 / size as f32;
        let mut f = 0.0;

        let (amount_integral, amount_fractional) = integral_fractional((amount * 4.0).clamp(0.0, 4.0));
        let row = amount_integral.clamp(0, 4) as usize;
        let mut coefficients = [0.0; 4];
        for (i, c) in coefficients.iter_mut().enumerate() {
            *c = crossfade(WARP_POLYNOMIALS[row][i], WARP_POLYNOMIALS[row + 1][i], amount_fractional);
        }
        let [a, b, c, d] = coefficients;
        let source = &source[..size + 2];
        for v in xf_polar[1..size].iter_mut() {
            f += bin_width;
            let wf = (d + f * (c + f * (b + a * f))) * size as f32;
            *v = interpolate(source, wf, 1.0);
        }
    }

    fn shift_magnitudes(&mut self, source: &[f32], xf_polar: &mut [f32], pitch_ratio: f32) {
        let size = self.size;
        let temp = &mut self.shift_temp[..size + 2];
        if pitch_ratio == 1.0 {
            temp[..size].copy_from_slice(&source[..size]);
        } else if pitch_ratio > 1.0 {
            // The module leaves the first bin as it was in its scratch
            // memory; it is never used.
            temp[0] = 0.0;
            let mut index = 1.0;
            let increment = 1.0 / pitch_ratio;
            for t in temp[1..size].iter_mut() {
                *t = interpolate(&source[..size + 1], index, 1.0);
                index += increment;
            }
        } else {
            temp.fill(0.0);
            let mut index = 1.0;
            let increment = pitch_ratio;
            for &s in source[1..size].iter() {
                let (i, f) = integral_fractional(index);
                let i = (i.max(0) as usize).min(size);
                temp[i] += (1.0 - f) * s;
                temp[i + 1] += f * s;
                index += increment;
            }
        }
        xf_polar[..size].copy_from_slice(&temp[..size]);
    }

    fn store_magnitudes(&mut self, xf_polar: &[f32], position: f32, feedback: f32, rng: &mut Random) {
        let size = self.size;
        // Write into magnitude buffers.
        let index_float = position * (self.num_textures - 1) as f32;
        let index_int = (index_float as i32).max(0) as usize;
        let index_fractional = index_float - index_int as f32;
        let mut gain_a = 1.0 - index_fractional;
        let mut gain_b = index_fractional;

        let a_range = self.texture_range(index_int);
        let b_range = self.texture_range(index_int + if position == 1.0 { 0 } else { 1 });
        let xf = &xf_polar[..size];

        if feedback >= 0.5 {
            let feedback = 2.0 * (feedback - 0.5);
            if feedback < 0.5 {
                gain_a *= 1.0 - feedback;
                gain_b *= 1.0 - feedback;
                for (i, &x) in xf.iter().enumerate() {
                    let a = &mut self.textures[a_range.start + i];
                    *a = crossfade(*a, x, gain_a);
                    let b = &mut self.textures[b_range.start + i];
                    *b = crossfade(*b, x, gain_b);
                }
            } else {
                let t = (feedback - 0.5) * 0.7 + 0.5;
                let mut gain_new = t - 0.5;
                gain_new = gain_new * gain_new * 2.0 + 0.5;
                let gain_new_a = gain_a * gain_new;
                let gain_new_b = gain_b * gain_new;
                let gain_old_a = 1.0 - gain_a * (1.0 - t);
                let gain_old_b = 1.0 - gain_b * (1.0 - t);
                for (i, &x) in xf.iter().enumerate() {
                    let a = &mut self.textures[a_range.start + i];
                    *a = *a * gain_old_a + x * gain_new_a;
                    let b = &mut self.textures[b_range.start + i];
                    *b = *b * gain_old_b + x * gain_new_b;
                }
            }
        } else {
            let mut feedback = feedback * 2.0;
            feedback *= feedback;
            let threshold = (feedback * 65535.0) as u16;
            for (i, &x) in xf.iter().enumerate() {
                let gain = if (rng.get_sample() as u16) <= threshold { 1.0 } else { 0.0 };
                let a = &mut self.textures[a_range.start + i];
                *a = crossfade(*a, x, gain_a * gain);
                let b = &mut self.textures[b_range.start + i];
                *b = crossfade(*b, x, gain_b * gain);
            }
        }
    }

    fn replay_magnitudes(&self, xf_polar: &mut [f32], position: f32) {
        let size = self.size;
        let index_float = position * (self.num_textures - 1) as f32;
        let index_int = (index_float as i32).max(0) as usize;
        let index_fractional = index_float - index_int as f32;
        let a = &self.textures[self.texture_range(index_int)];
        let b = &self.textures[self.texture_range(index_int + if position == 1.0 { 0 } else { 1 })];
        for i in 0..size {
            xf_polar[i] = crossfade(a[i], b[i], index_fractional);
        }
    }
}

/// Index of the first largest value (`std::max_element`).
fn argmax(x: &[f32]) -> usize {
    let mut best = 0;
    for i in 1..x.len() {
        if x[i] > x[best] {
            best = i;
        }
    }
    best
}
