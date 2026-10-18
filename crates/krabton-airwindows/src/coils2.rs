//! Coils2: a transformer saturation with a "cheapness" control that makes
//! the coil smaller and narrower (`Coils2Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};
use std::f64::consts::PI;

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Coils2",
    name: "Coils2",
    category: "Saturation",
    description: "Emulates audio transformer saturation, from subtle iron coloration to cheap, small-transformer grit.",
    params: &[
        ParamInfo { name: "Saturate", default: 0.0 },
        ParamInfo { name: "Cheapness", default: 0.0 },
        ParamInfo { name: "Dry/Wet", default: 1.0 },
    ],
};

pub(crate) struct Coils2 {
    sample_rate: f64,
    /// The clean-tone bandpass; left state in `[7..9]`, right in `[9..11]`.
    biquad_a: [f64; 15],
    /// The lowpass on the notched-out part; same layout.
    biquad_b: [f64; 15],
    hysteresis: [f64; 2],
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Coils2 {
    const NUM_PARAMS: usize = 3;

    fn new(sample_rate: f64) -> Self {
        Self { sample_rate, biquad_a: [0.0; 15], biquad_b: [0.0; 15], hysteresis: [0.0; 2], fpd_l: crate::dsp::seed(), fpd_r: crate::dsp::seed() }
    }

    fn reset(&mut self) {
        self.biquad_a = [0.0; 15];
        self.biquad_b = [0.0; 15];
        self.hysteresis = [0.0; 2];
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let overallscale = self.sample_rate / 44100.0;

        let mut dist_scaling = (1.0 - p[0]).powf(2.0);
        if dist_scaling < 0.0001 {
            dist_scaling = 0.0001;
        }
        let a = &mut self.biquad_a;
        a[0] = 600.0 / self.sample_rate;
        a[1] = 0.01 + (p[1].powf(2.0) * 0.5);
        let iir_amount = a[1] / overallscale;
        let k = (PI * a[0]).tan();
        let norm = 1.0 / (1.0 + k / a[1] + k * k);
        a[2] = k / a[1] * norm;
        a[4] = -a[2];
        a[5] = 2.0 * (k * k - 1.0) * norm;
        a[6] = (1.0 - k / a[1] + k * k) * norm;
        let b = &mut self.biquad_b;
        b[0] = (21890.0 - (a[1] * 890.0)) / self.sample_rate;
        b[1] = 0.89;
        let k = (PI * b[0]).tan();
        let norm = 1.0 / (1.0 + k / b[1] + k * k);
        b[2] = k * k * norm;
        b[3] = 2.0 * b[2];
        b[4] = b[2];
        b[5] = 2.0 * (k * k - 1.0) * norm;
        b[6] = (1.0 - k / b[1] + k * k) * norm;
        let wet = p[2];

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input = [noise_floor(*out_l as f64, self.fpd_l), noise_floor(*out_r as f64, self.fpd_r)];
            let dry = input;

            for c in 0..2 {
                let s = 7 + c * 2;
                let a = &mut self.biquad_a;
                if a[0] < 0.49999 {
                    let temp = (input[c] * a[2]) + a[s];
                    a[s] = -(temp * a[5]) + a[s + 1];
                    a[s + 1] = (input[c] * a[4]) - (temp * a[6]);
                    input[c] = temp; //create bandpass of clean tone
                }
                let mut diff = (dry[c] - input[c]) / dist_scaling; //mids notched out
                let b = &mut self.biquad_b;
                if b[0] < 0.49999 {
                    let temp = (diff * b[2]) + b[s];
                    b[s] = (diff * b[3]) - (temp * b[5]) + b[s + 1];
                    b[s + 1] = (diff * b[4]) - (temp * b[6]);
                    diff = temp;
                }
                let h = &mut self.hysteresis[c];
                *h = (*h * (1.0 - iir_amount)) + (diff * iir_amount);
                if h.abs() < 1.18e-37 {
                    *h = 0.0;
                } else {
                    diff -= *h;
                }
                diff = diff.clamp(-1.571, 1.571);
                *h = h.clamp(-1.571, 1.571);
                input[c] += (diff.sin() - h.sin()) * dist_scaling; //apply transformer distortions
            }

            if wet != 1.0 {
                input[0] = (input[0] * wet) + (dry[0] * (1.0 - wet));
                input[1] = (input[1] * wet) + (dry[1] * (1.0 - wet));
            }

            *out_l = dither32(input[0], &mut self.fpd_l) as f32;
            *out_r = dither32(input[1], &mut self.fpd_r) as f32;
        }
    }
}
