//! Tube2: asymmetric tube-style saturation (`Tube2Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Tube2",
    name: "Tube2",
    category: "Saturation",
    description: "Warm, asymmetric tube-style saturation with input level and tube drive controls.",
    params: &[ParamInfo { name: "Input", default: 0.5 }, ParamInfo { name: "Tube", default: 0.5 }],
};

pub(crate) struct Tube2 {
    overallscale: f64,
    /// previousSampleA/C/E (left) and B/D/F (right).
    prev_in: [f64; 2],
    prev_mid: [f64; 2],
    prev_out: [f64; 2],
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Tube2 {
    const NUM_PARAMS: usize = 2;

    fn new(sample_rate: f64) -> Self {
        Self { overallscale: sample_rate / 44100.0, prev_in: [0.0; 2], prev_mid: [0.0; 2], prev_out: [0.0; 2], fpd_l: crate::dsp::seed(), fpd_r: crate::dsp::seed() }
    }

    fn reset(&mut self) {
        self.prev_in = [0.0; 2];
        self.prev_mid = [0.0; 2];
        self.prev_out = [0.0; 2];
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let overallscale = self.overallscale;
        let input_pad = p[0];
        let iterations = 1.0 - p[1];
        let powerfactor = ((9.0 * iterations) + 1.0) as i32;
        let asym_pad = powerfactor as f64;
        let gainscaling = 1.0 / (powerfactor + 1) as f64;
        let outputscaling = 1.0 + (1.0 / powerfactor as f64);

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input = [noise_floor(*out_l as f64, self.fpd_l), noise_floor(*out_r as f64, self.fpd_r)];
            for c in 0..2 {
                let mut x = input[c];
                if input_pad < 1.0 {
                    x *= input_pad;
                }
                if overallscale > 1.9 {
                    let stored = x;
                    x += self.prev_in[c];
                    self.prev_in[c] = stored;
                    x *= 0.5;
                } //for high sample rates on this plugin we are going to do a simple average

                x = x.clamp(-1.0, 1.0);

                //flatten bottom, point top of sine waveshaper L
                x /= asym_pad;
                let mut sharpen = -x;
                if sharpen > 0.0 {
                    sharpen = 1.0 + sharpen.sqrt();
                } else {
                    sharpen = 1.0 - (-sharpen).sqrt();
                }
                x -= x * x.abs() * sharpen * 0.25;
                x *= asym_pad;

                let mut factor = x;
                for _ in 0..powerfactor {
                    factor *= x;
                }
                if (powerfactor % 2 == 1) && (x != 0.0) {
                    factor = (factor / x) * x.abs();
                }
                factor *= gainscaling;
                x -= factor;
                x *= outputscaling;

                if overallscale > 1.9 {
                    let stored = x;
                    x += self.prev_mid[c];
                    self.prev_mid[c] = stored;
                    x *= 0.5;
                } //for high sample rates on this plugin we are going to do a simple average

                let mut slew = self.prev_out[c] - x;
                if overallscale > 1.9 {
                    let stored = x;
                    x += self.prev_out[c];
                    self.prev_out[c] = stored;
                    x *= 0.5;
                } else {
                    self.prev_out[c] = x;
                }
                if slew > 0.0 {
                    slew = 1.0 + (slew.sqrt() * 0.5);
                } else {
                    slew = 1.0 - ((-slew).sqrt() * 0.5);
                }
                x -= x * x.abs() * slew * gainscaling;
                x = x.clamp(-0.52, 0.52);
                x *= 1.923076923076923;
                input[c] = x;
            }
            *out_l = dither32(input[0], &mut self.fpd_l) as f32;
            *out_r = dither32(input[1], &mut self.fpd_r) as f32;
        }
    }
}
