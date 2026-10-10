//! ButterComp2: a smooth bipolar, interleaved compressor
//! (`ButterComp2Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::dither32;
use crate::{EffectInfo, Kernel, ParamInfo};

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "ButterComp2",
    name: "ButterComp2",
    category: "Dynamics",
    description: "A soft, glue-like compressor that smooths dynamics without obvious pumping.",
    params: &[
        ParamInfo { name: "Compress", default: 0.0 },
        ParamInfo { name: "Output", default: 0.5 },
        ParamInfo { name: "Dry/Wet", default: 1.0 },
    ],
};

#[derive(Clone, Copy)]
struct Channel {
    control_a_pos: f64,
    control_a_neg: f64,
    control_b_pos: f64,
    control_b_neg: f64,
    target_pos: f64,
    target_neg: f64,
    last_output: f64,
    noise_source: i32,
}

impl Channel {
    fn new(noise_source: i32) -> Self {
        Self { control_a_pos: 1.0, control_a_neg: 1.0, control_b_pos: 1.0, control_b_neg: 1.0, target_pos: 1.0, target_neg: 1.0, last_output: 0.0, noise_source }
    }

    /// The "residue" noise the original adds to keep away from denormals.
    #[inline]
    fn residue(&mut self) -> f64 {
        self.noise_source %= 1700021;
        self.noise_source += 1;
        let mut residue = self.noise_source.wrapping_mul(self.noise_source);
        residue %= 170003;
        residue = residue.wrapping_mul(residue);
        residue %= 17011;
        residue = residue.wrapping_mul(residue);
        residue %= 1709;
        residue = residue.wrapping_mul(residue);
        residue %= 173;
        residue = residue.wrapping_mul(residue);
        residue %= 17;
        let mut apply = residue as f64;
        apply *= 0.00000001;
        apply *= 0.00000001;
        apply
    }
}

pub(crate) struct ButterComp2 {
    overallscale: f64,
    ch: [Channel; 2],
    flip: bool,
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for ButterComp2 {
    const NUM_PARAMS: usize = 3;

    fn new(sample_rate: f64) -> Self {
        Self { overallscale: sample_rate / 44100.0, ch: [Channel::new(0), Channel::new(850010)], flip: false, fpd_l: crate::dsp::seed(), fpd_r: crate::dsp::seed() }
    }

    fn reset(&mut self) {
        self.ch = [Channel::new(0), Channel::new(850010)];
        self.flip = false;
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let overallscale = self.overallscale;
        let inputgain = 10.0f64.powf((p[0] * 14.0) / 20.0);
        let compfactor = 0.012 * (p[0] / 135.0);
        let output = p[1] * 2.0;
        let wet = p[2];
        let mut outputgain = inputgain;
        outputgain -= 1.0;
        outputgain /= 1.5;
        outputgain += 1.0;

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input = [*out_l as f64, *out_r as f64];
            let flip = self.flip;
            for c in 0..2 {
                let ch = &mut self.ch[c];
                let apply_residue = ch.residue();
                let mut x = input[c] + apply_residue;
                if x < 1.2e-38 && -x < 1.2e-38 {
                    x -= apply_residue;
                }
                //for live air, we always apply the dither noise. Then, if our result is
                //effectively digital black, we'll subtract it again.
                let dry = x;

                x *= inputgain;

                let mut divisor = compfactor / (1.0 + ch.last_output.abs());
                divisor /= overallscale;
                let remainder = divisor;
                divisor = 1.0 - divisor;
                //recalculate divisor every sample

                let mut inputpos = x + 1.0;
                if inputpos < 0.0 {
                    inputpos = 0.0;
                }
                let mut outputpos = inputpos / 2.0;
                if outputpos > 1.0 {
                    outputpos = 1.0;
                }
                inputpos *= inputpos;
                ch.target_pos *= divisor;
                ch.target_pos += inputpos * remainder;
                let calcpos = (1.0 / ch.target_pos).powf(2.0);

                let mut inputneg = (-x) + 1.0;
                if inputneg < 0.0 {
                    inputneg = 0.0;
                }
                let mut outputneg = inputneg / 2.0;
                if outputneg > 1.0 {
                    outputneg = 1.0;
                }
                inputneg *= inputneg;
                ch.target_neg *= divisor;
                ch.target_neg += inputneg * remainder;
                let calcneg = (1.0 / ch.target_neg).powf(2.0);
                //now we have mirrored targets for comp

                if x > 0.0 {
                    //working on pos
                    let control = if flip { &mut ch.control_a_pos } else { &mut ch.control_b_pos };
                    *control *= divisor;
                    *control += calcpos * remainder;
                } else {
                    //working on neg
                    let control = if flip { &mut ch.control_a_neg } else { &mut ch.control_b_neg };
                    *control *= divisor;
                    *control += calcneg * remainder;
                }
                //this causes each of the four to update only when active and in the correct 'flip'

                let totalmultiplier = if flip { (ch.control_a_pos * outputpos) + (ch.control_a_neg * outputneg) } else { (ch.control_b_pos * outputpos) + (ch.control_b_neg * outputneg) };
                //this combines the sides according to flip, blending relative to the input value

                x *= totalmultiplier;
                x /= outputgain;
                if output != 1.0 {
                    x *= output;
                }
                if wet != 1.0 {
                    x = (x * wet) + (dry * (1.0 - wet));
                }
                ch.last_output = x;
                input[c] = x;
            }
            self.flip = !self.flip;

            *out_l = dither32(input[0], &mut self.fpd_l) as f32;
            *out_r = dither32(input[1], &mut self.fpd_r) as f32;
        }
    }
}
