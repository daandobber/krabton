//! Pressure4: a stereo-linked mu-style compressor/limiter
//! (`Pressure4Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Pressure4",
    name: "Pressure4",
    category: "Dynamics",
    description: "A stereo-linked, program-dependent compressor and limiter with a soft-clipped output.",
    params: &[
        ParamInfo { name: "Pressure", default: 0.0 },
        ParamInfo { name: "Speed", default: 0.2 },
        ParamInfo { name: "Mewiness", default: 1.0 },
        ParamInfo { name: "Output Gain", default: 1.0 },
    ],
};

pub(crate) struct Pressure4 {
    overallscale: f64,
    mu_speed_a: f64,
    mu_speed_b: f64,
    mu_coefficient_a: f64,
    mu_coefficient_b: f64,
    /// The original declares and checks `flip` but never toggles it, so only
    /// the `B` follower ever runs; kept that way.
    flip: bool,
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Pressure4 {
    const NUM_PARAMS: usize = 4;

    fn new(sample_rate: f64) -> Self {
        let mut s = Self {
            overallscale: sample_rate / 44100.0,
            mu_speed_a: 0.0,
            mu_speed_b: 0.0,
            mu_coefficient_a: 0.0,
            mu_coefficient_b: 0.0,
            flip: false,
            fpd_l: crate::dsp::seed(),
            fpd_r: crate::dsp::seed(),
        };
        s.reset();
        s
    }

    fn reset(&mut self) {
        self.mu_speed_a = 10000.0;
        self.mu_speed_b = 10000.0;
        self.mu_coefficient_a = 1.0;
        self.mu_coefficient_b = 1.0;
        self.flip = false;
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let threshold = 1.0 - (p[0] * 0.95);
        let mu_makeup_gain = 1.0 / threshold;
        //gain settings around threshold
        let mut release = (1.28 - p[1]).powf(5.0) * 32768.0;
        release /= self.overallscale;
        let fastest = release.sqrt();
        //speed settings around release
        let mut mewiness = (p[2] * 2.0) - 1.0;
        let positivemu = mewiness >= 0.0;
        if !positivemu {
            mewiness = -mewiness;
        }
        let unmewiness = 1.0 - mewiness;
        let output_gain = p[3];

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input_l = noise_floor(*out_l as f64, self.fpd_l);
            let mut input_r = noise_floor(*out_r as f64, self.fpd_r);

            input_l *= mu_makeup_gain;
            input_r *= mu_makeup_gain;

            let mut input_sense = input_l.abs();
            if input_r.abs() > input_sense {
                input_sense = input_r.abs();
            }
            //we will take the greater of either channel and just use that, then apply the result
            //to both stereo channels.

            let (speed, coefficient) = if self.flip { (&mut self.mu_speed_a, &mut self.mu_coefficient_a) } else { (&mut self.mu_speed_b, &mut self.mu_coefficient_b) };
            if input_sense > threshold {
                let mu_vary = threshold / input_sense;
                let mu_attack = speed.abs().sqrt();
                *coefficient *= mu_attack - 1.0;
                if mu_vary < threshold {
                    *coefficient += threshold;
                } else {
                    *coefficient += mu_vary;
                }
                *coefficient /= mu_attack;
            } else {
                *coefficient *= (*speed * *speed) - 1.0;
                *coefficient += 1.0;
                *coefficient /= *speed * *speed;
            }
            let mut mu_new_speed = *speed * (*speed - 1.0);
            mu_new_speed = mu_new_speed + (input_sense * release).abs() + fastest;
            *speed = mu_new_speed / *speed;

            let mut gain = if positivemu { coefficient.powf(2.0) } else { coefficient.sqrt() };
            gain = (gain * mewiness) + (*coefficient * unmewiness);
            input_l *= gain;
            input_r *= gain;

            if output_gain != 1.0 {
                input_l *= output_gain;
                input_r *= output_gain;
            }

            input_l = sine_clip(input_l);
            input_r = sine_clip(input_r);
            //second stage of overdrive to prevent overs and allow bloody loud extremeness

            *out_l = dither32(input_l, &mut self.fpd_l) as f32;
            *out_r = dither32(input_r, &mut self.fpd_r) as f32;
        }
    }
}

#[inline]
fn sine_clip(x: f64) -> f64 {
    let mut b = x.abs();
    if b > 1.57079633 {
        b = 1.0;
    } else {
        b = b.sin();
    }
    if x > 0.0 {
        b
    } else {
        -b
    }
}
