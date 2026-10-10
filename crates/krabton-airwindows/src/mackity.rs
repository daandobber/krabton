//! Mackity: a Mackie-style console input stage (`MackityProc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};
use std::f64::consts::PI;

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Mackity",
    name: "Mackity",
    category: "Saturation",
    description: "Emulates the gritty, overdriven input stage of a classic compact mixer.",
    params: &[ParamInfo { name: "In Trim", default: 0.1 }, ParamInfo { name: "Out Pad", default: 1.0 }],
};

pub(crate) struct Mackity {
    sample_rate: f64,
    iir_a: [f64; 2],
    iir_b: [f64; 2],
    biquad_a: [f64; 15],
    biquad_b: [f64; 15],
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Mackity {
    const NUM_PARAMS: usize = 2;

    fn new(sample_rate: f64) -> Self {
        Self { sample_rate, iir_a: [0.0; 2], iir_b: [0.0; 2], biquad_a: [0.0; 15], biquad_b: [0.0; 15], fpd_l: crate::dsp::seed(), fpd_r: crate::dsp::seed() }
    }

    fn reset(&mut self) {
        self.iir_a = [0.0; 2];
        self.iir_b = [0.0; 2];
        self.biquad_a = [0.0; 15];
        self.biquad_b = [0.0; 15];
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let overallscale = self.sample_rate / 44100.0;
        let mut in_trim = p[0] * 10.0;
        let out_pad = p[1];
        in_trim *= in_trim;

        let iir_amount_a = 0.001860867 / overallscale;
        let iir_amount_b = 0.000287496 / overallscale;

        // 19160 Hz; below about 42.6 kHz that would pass Nyquist, so it is
        // held just under it there (the original only runs at 44.1k and up)
        let freq = (19160.0 / self.sample_rate).min(0.45);
        self.biquad_a[0] = freq;
        self.biquad_b[0] = freq;
        self.biquad_a[1] = 0.431684981684982;
        self.biquad_b[1] = 1.1582298;
        lowpass(&mut self.biquad_a);
        lowpass(&mut self.biquad_b);

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input = [noise_floor(*out_l as f64, self.fpd_l), noise_floor(*out_r as f64, self.fpd_r)];
            for c in 0..2 {
                let mut x = input[c];
                if self.iir_a[c].abs() < 1.18e-37 {
                    self.iir_a[c] = 0.0;
                }
                self.iir_a[c] = (self.iir_a[c] * (1.0 - iir_amount_a)) + (x * iir_amount_a);
                x -= self.iir_a[c];

                if in_trim != 1.0 {
                    x *= in_trim;
                }

                x = df1(&mut self.biquad_a, c, x);
                x = x.clamp(-1.0, 1.0);
                x -= x.powi(5) * 0.1768;
                x = df1(&mut self.biquad_b, c, x);

                if self.iir_b[c].abs() < 1.18e-37 {
                    self.iir_b[c] = 0.0;
                }
                self.iir_b[c] = (self.iir_b[c] * (1.0 - iir_amount_b)) + (x * iir_amount_b);
                x -= self.iir_b[c];

                if out_pad != 1.0 {
                    x *= out_pad;
                }
                input[c] = x;
            }
            *out_l = dither32(input[0], &mut self.fpd_l) as f32;
            *out_r = dither32(input[1], &mut self.fpd_r) as f32;
        }
    }
}

/// Lowpass coefficients into `b[2..7]` from `b[0]` (freq) and `b[1]` (Q).
fn lowpass(b: &mut [f64; 15]) {
    let k = (PI * b[0]).tan();
    let norm = 1.0 / (1.0 + k / b[1] + k * k);
    b[2] = k * k * norm;
    b[3] = 2.0 * b[2];
    b[4] = b[2];
    b[5] = 2.0 * (k * k - 1.0) * norm;
    b[6] = (1.0 - k / b[1] + k * k) * norm;
}

/// Direct form I; left state in `b[7..11]`, right in `b[11..15]`.
#[inline]
fn df1(b: &mut [f64; 15], channel: usize, x: f64) -> f64 {
    let s = 7 + channel * 4;
    let y = b[2] * x + b[3] * b[s] + b[4] * b[s + 1] - b[5] * b[s + 2] - b[6] * b[s + 3];
    b[s + 1] = b[s];
    b[s] = x;
    b[s + 3] = b[s + 2];
    b[s + 2] = y;
    y
}
