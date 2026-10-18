//! Fracture2: a sine wavefolder with extra stages of disruption
//! (`Fracture2Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};
use std::f64::consts::FRAC_PI_2;

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Fracture2",
    name: "Fracture2",
    category: "Saturation",
    description: "A wavefolder that bends hot signals back on themselves, from gritty folding to shattered, metallic distortion.",
    params: &[
        ParamInfo { name: "Drive", default: 0.1 },
        ParamInfo { name: "Fractre", default: 0.0 },
        ParamInfo { name: "Thresh", default: 0.5 },
        ParamInfo { name: "Output", default: 1.0 },
        ParamInfo { name: "Dry/Wet", default: 1.0 },
    ],
};

pub(crate) struct Fracture2 {
    /// Each control glides from its value at the previous block (`a`) to
    /// the new one (`b`) across the block.
    density_a: f64,
    density_b: f64,
    threshold_a: f64,
    threshold_b: f64,
    output_a: f64,
    output_b: f64,
    wet_a: f64,
    wet_b: f64,
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Fracture2 {
    const NUM_PARAMS: usize = 5;

    fn new(_sample_rate: f64) -> Self {
        let mut s = Self {
            density_a: 0.0,
            density_b: 0.0,
            threshold_a: 0.0,
            threshold_b: 0.0,
            output_a: 0.0,
            output_b: 0.0,
            wet_a: 0.0,
            wet_b: 0.0,
            fpd_l: crate::dsp::seed(),
            fpd_r: crate::dsp::seed(),
        };
        s.reset();
        s
    }

    fn reset(&mut self) {
        self.density_a = 1.0;
        self.density_b = 1.0;
        self.threshold_a = 0.5;
        self.threshold_b = 0.5;
        self.output_a = 1.0;
        self.output_b = 1.0;
        self.wet_a = 1.0;
        self.wet_b = 1.0;
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let n = left.len();
        let in_frames_to_process = n as f64;
        self.density_a = self.density_b;
        self.density_b = p[0] * 10.0; //0.0 to 10.0
        let stages = (p[1] * 8.0) as i32;
        self.threshold_a = self.threshold_b;
        self.threshold_b = p[2];
        self.output_a = self.output_b;
        self.output_b = p[3];
        self.wet_a = self.wet_b;
        self.wet_b = p[4];

        for (i, (out_l, out_r)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
            let mut input = [noise_floor(*out_l as f64, self.fpd_l), noise_floor(*out_r as f64, self.fpd_r)];
            let dry = input;

            // `sampleFrames` after the loop's pre-decrement: n-1 down to 0
            let temp = (n - 1 - i) as f64 / in_frames_to_process;
            let density = (self.density_a * temp) + (self.density_b * (1.0 - temp));
            let threshold = (self.threshold_a * temp) + (self.threshold_b * (1.0 - temp));
            let output = (self.output_a * temp) + (self.output_b * (1.0 - temp));
            let wet = (self.wet_a * temp) + (self.wet_b * (1.0 - temp));

            for x in input.iter_mut() {
                *x *= density;
                for _ in 0..stages {
                    *x *= x.abs() + 1.0;
                }
                // at full drive and fracture a hot input overflows to infinity
                // here and the original then outputs NaN; holding it at the
                // largest finite value keeps it folding (sin(f64::MAX))
                *x = x.clamp(-f64::MAX, f64::MAX);

                if *x > FRAC_PI_2 {
                    *x = (x.sin() * threshold) + (1.0 - threshold);
                } else if *x < -FRAC_PI_2 {
                    *x = (x.sin() * threshold) - (1.0 - threshold);
                } else {
                    *x = x.sin();
                }

                *x *= output;
            }

            if wet != 1.0 {
                input[0] = (input[0] * wet) + (dry[0] * (1.0 - wet));
                input[1] = (input[1] * wet) + (dry[1] * (1.0 - wet));
            }
            //Dry/Wet control, defaults to the last slider

            *out_l = dither32(input[0], &mut self.fpd_l) as f32;
            *out_r = dither32(input[1], &mut self.fpd_r) as f32;
        }
    }
}
