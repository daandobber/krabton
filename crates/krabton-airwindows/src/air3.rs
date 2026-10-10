//! Air3: high-end "air" and low-end "ground" shelving from a
//! fourth-order slope predictor (`Air3Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Air3",
    name: "Air3",
    category: "EQ",
    description: "Boosts or cuts the airy top end and the grounded low end with a natural, transparent tone.",
    params: &[ParamInfo { name: "Air", default: 0.5 }, ParamInfo { name: "Gnd", default: 0.5 }],
};

#[derive(Clone, Copy, Default)]
struct Channel {
    pv_a1: f64,
    pv_a2: f64,
    pv_a3: f64,
    pv_a4: f64,
    gndavg: f64,
    out_a: f64,
    gain_a: f64,
}

pub(crate) struct Air3 {
    overallscale: f64,
    ch: [Channel; 2],
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Air3 {
    const NUM_PARAMS: usize = 2;

    fn new(sample_rate: f64) -> Self {
        Self { overallscale: sample_rate / 44100.0, ch: [Channel::default(); 2], fpd_l: crate::dsp::seed(), fpd_r: crate::dsp::seed() }
    }

    fn reset(&mut self) {
        self.ch = [Channel::default(); 2];
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let overallscale = self.overallscale;
        let mut air_gain = p[0] * 2.0;
        if air_gain > 1.0 {
            air_gain = air_gain.powf(3.0 + overallscale.sqrt());
        }
        let gnd_gain = p[1] * 2.0;
        let gain_limit = 0.3 * overallscale.sqrt();

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input = [noise_floor(*out_l as f64, self.fpd_l), noise_floor(*out_r as f64, self.fpd_r)];
            for c in 0..2 {
                let ch = &mut self.ch[c];
                let dry = input[c];
                // the slope (pvS) and acceleration (accS, acc2S) terms are
                // recomputed from the history every sample
                let pv_s4 = ch.pv_a4 - ch.pv_a3;
                let pv_s3 = ch.pv_a3 - ch.pv_a2;
                let pv_s2 = ch.pv_a2 - ch.pv_a1;
                let pv_s1 = ch.pv_a1 - input[c];
                let acc_s3 = pv_s4 - pv_s3;
                let acc_s2 = pv_s3 - pv_s2;
                let acc_s1 = pv_s2 - pv_s1;
                let acc2_s2 = acc_s3 - acc_s2;
                let acc2_s1 = acc_s2 - acc_s1;
                ch.out_a = -(ch.pv_a1 + pv_s3 + acc2_s2 - ((acc2_s2 + acc2_s1) * 0.5));
                ch.gain_a *= 0.5;
                ch.gain_a += (dry - ch.out_a).abs() * 0.5;
                if ch.gain_a > gain_limit {
                    ch.gain_a = gain_limit;
                }
                ch.pv_a4 = ch.pv_a3;
                ch.pv_a3 = ch.pv_a2;
                ch.pv_a2 = ch.pv_a1;
                ch.pv_a1 = (ch.gain_a * ch.out_a) + dry;
                let mut gnd = dry - ((ch.out_a * 0.5) + (dry * (0.457 - (0.017 * overallscale))));
                let temp = (gnd + ch.gndavg) * 0.5;
                ch.gndavg = gnd;
                gnd = temp;
                input[c] = ((dry - gnd) * air_gain) + (gnd * gnd_gain);
            }
            *out_l = dither32(input[0], &mut self.fpd_l) as f32;
            *out_r = dither32(input[1], &mut self.fpd_r) as f32;
        }
    }
}
