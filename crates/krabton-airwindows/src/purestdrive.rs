//! PurestDrive: sine-based saturation that backs off on transients
//! (`PurestDriveProc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "PurestDrive",
    name: "PurestDrive",
    category: "Saturation",
    description: "Subtle, musical saturation that thickens the sound while letting the highs through.",
    params: &[ParamInfo { name: "Drive", default: 0.0 }],
};

pub(crate) struct PurestDrive {
    previous: [f64; 2],
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for PurestDrive {
    const NUM_PARAMS: usize = 1;

    fn new(_sample_rate: f64) -> Self {
        Self { previous: [0.0; 2], fpd_l: crate::dsp::seed(), fpd_r: crate::dsp::seed() }
    }

    fn reset(&mut self) {
        self.previous = [0.0; 2];
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let intensity = p[0];
        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input = [noise_floor(*out_l as f64, self.fpd_l), noise_floor(*out_r as f64, self.fpd_r)];
            for c in 0..2 {
                let dry = input[c];
                let x = dry.sin();
                //basic distortion factor
                let apply = ((self.previous[c] + x).abs() / 2.0) * intensity;
                //saturate less if previous sample was undistorted and low level, or if it was
                //inverse polarity. Lets through highs and brightness more.
                input[c] = (dry * (1.0 - apply)) + (x * apply);
                //dry-wet control for intensity also has FM modulation to clean up highs
                self.previous[c] = dry.sin();
            }
            *out_l = dither32(input[0], &mut self.fpd_l) as f32;
            *out_r = dither32(input[1], &mut self.fpd_r) as f32;
        }
    }
}
