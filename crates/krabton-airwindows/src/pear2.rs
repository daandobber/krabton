//! Pear2: the Pear filter, a stack of up to sixteen slew-tracking lowpass
//! poles, with non-linearity and an inverted (highpass) mode (`Pear2Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Pear2",
    name: "Pear2",
    category: "EQ",
    description: "A smooth lowpass filter of up to sixteen poles with optional non-linearity that turns into a highpass below the middle of Inv/Wet.",
    params: &[
        ParamInfo { name: "Freq", default: 1.0 },
        ParamInfo { name: "NonLin", default: 0.5 },
        ParamInfo { name: "Poles", default: 0.5 },
        ParamInfo { name: "Inv/Wet", default: 1.0 },
    ],
};

/// `pear_total`: per pole a sample and a slew, for left and right.
const PEAR_TOTAL: usize = 64;

pub(crate) struct Pear2 {
    /// `prevSampL`, `prevSlewL`, `prevSampR`, `prevSlewR` for each pole in turn.
    pear: [f64; PEAR_TOTAL],
    freq_a: f64,
    freq_b: f64,
    non_lin_a: f64,
    non_lin_b: f64,
    wet_a: f64,
    wet_b: f64,
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Pear2 {
    const NUM_PARAMS: usize = 4;

    fn new(_sample_rate: f64) -> Self {
        let mut s = Self {
            pear: [0.0; PEAR_TOTAL],
            freq_a: 0.0,
            freq_b: 0.0,
            non_lin_a: 0.0,
            non_lin_b: 0.0,
            wet_a: 0.0,
            wet_b: 0.0,
            fpd_l: crate::dsp::seed(),
            fpd_r: crate::dsp::seed(),
        };
        s.reset();
        s
    }

    fn reset(&mut self) {
        self.pear = [0.0; PEAR_TOTAL];
        self.freq_a = 0.5;
        self.freq_b = 0.5;
        self.non_lin_a = 0.0;
        self.non_lin_b = 0.0;
        self.wet_a = 0.5;
        self.wet_b = 0.5;
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let frames = left.len();
        if frames == 0 {
            // nothing to glide over; keep the previous end point for the next block
            return;
        }

        self.freq_a = self.freq_b;
        self.freq_b = p[0].powf(2.0);
        if self.freq_b < 0.0001 {
            self.freq_b = 0.0001;
        }
        self.non_lin_a = self.non_lin_b;
        self.non_lin_b = p[1].powf(2.0);
        let max_poles = p[2].powf(3.0) * PEAR_TOTAL as f64;
        // `for (int x = 0; x < maxPoles; x += 4)`: how many poles that runs
        let poles = ((max_poles / 4.0).ceil() as usize).min(PEAR_TOTAL / 4);
        self.wet_a = self.wet_b;
        self.wet_b = p[3]; //inv-dry-wet for highpass

        for (i, (out_l, out_r)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
            let mut input_l = noise_floor(*out_l as f64, self.fpd_l);
            let mut input_r = noise_floor(*out_r as f64, self.fpd_r);
            let mut dry_l = input_l;
            let mut dry_r = input_r;

            // glides from the previous block's settings to this block's
            let temp = (frames - 1 - i) as f64 / frames as f64;
            let freq = (self.freq_a * temp) + (self.freq_b * (1.0 - temp));
            let non_lin = (self.non_lin_a * temp) + (self.non_lin_b * (1.0 - temp));
            let mut wet = (self.wet_a * temp) + (self.wet_b * (1.0 - temp));
            let mut dry = 2.0 - (wet * 2.0);
            if dry > 1.0 {
                dry = 1.0; //full dry for use with inv, to 0.0 at full wet
            }
            wet = (wet * 2.0) - 1.0; //inv-dry-wet for highpass

            for pole in self.pear.chunks_exact_mut(4).take(poles) {
                let mut di = (freq * (1.0 + (input_l * non_lin))).abs().min(1.0);
                let mut slew = ((input_l - pole[0]) + pole[1]) * di * 0.5;
                input_l = (di * input_l) + ((1.0 - di) * (pole[0] + pole[1]));
                pole[0] = input_l;
                pole[1] = slew;
                di = (freq * (1.0 + (input_r * non_lin))).abs().min(1.0);
                slew = ((input_r - pole[2]) + pole[3]) * di * 0.5;
                input_r = (di * input_r) + ((1.0 - di) * (pole[2] + pole[3]));
                pole[2] = input_r;
                pole[3] = slew;
            }

            input_l *= wet;
            input_r *= wet;
            dry_l *= dry;
            dry_r *= dry;
            input_l += dry_l;
            input_r += dry_r;

            *out_l = dither32(input_l, &mut self.fpd_l) as f32;
            *out_r = dither32(input_r, &mut self.fpd_r) as f32;
        }
    }
}
