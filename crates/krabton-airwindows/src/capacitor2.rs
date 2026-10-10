//! Capacitor2: interleaved one-pole lowpass and highpass filters with a
//! capacitor-style non-linearity (`Capacitor2Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Capacitor2",
    name: "Capacitor2",
    category: "EQ",
    description: "Soft analog-style lowpass and highpass filters with a touch of capacitor non-linearity.",
    params: &[
        ParamInfo { name: "Lowpass", default: 1.0 },
        ParamInfo { name: "Highpass", default: 0.0 },
        ParamInfo { name: "NonLin", default: 0.0 },
        ParamInfo { name: "Dry/Wet", default: 1.0 },
    ],
};

/// Which of the six filter pairs (A..F) run on each step of the 0..5 count;
/// pair A runs every sample.
const SCHEDULE: [[usize; 3]; 6] = [[0, 1, 3], [0, 2, 4], [0, 1, 5], [0, 2, 3], [0, 1, 4], [0, 2, 5]];

#[derive(Clone, Copy, Default)]
struct Channel {
    highpass: [f64; 6],
    lowpass: [f64; 6],
}

pub(crate) struct Capacitor2 {
    ch: [Channel; 2],
    count: usize,
    lowpass_base_amount: f64,
    highpass_base_amount: f64,
    wet: f64,
    last_lowpass: f64,
    last_highpass: f64,
    last_wet: f64,
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Capacitor2 {
    const NUM_PARAMS: usize = 4;

    fn new(_sample_rate: f64) -> Self {
        let mut s = Self {
            ch: [Channel::default(); 2],
            count: 0,
            lowpass_base_amount: 0.0,
            highpass_base_amount: 0.0,
            wet: 0.0,
            last_lowpass: 0.0,
            last_highpass: 0.0,
            last_wet: 0.0,
            fpd_l: crate::dsp::seed(),
            fpd_r: crate::dsp::seed(),
        };
        s.reset();
        s
    }

    fn reset(&mut self) {
        self.ch = [Channel::default(); 2];
        self.count = 0;
        self.lowpass_base_amount = 1.0;
        self.highpass_base_amount = 0.0;
        self.wet = 1.0;
        self.last_lowpass = 1000.0;
        self.last_highpass = 1000.0;
        self.last_wet = 1000.0;
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let lowpass_chase = p[0].powf(2.0);
        let highpass_chase = p[1].powf(2.0);
        let non_lin = 1.0 + ((1.0 - p[2]) * 6.0);
        let non_lin_trim = 1.5 / non_lin.cbrt();
        let wet_chase = p[3];
        //should not scale with sample rate, because values reaching 1 are important
        //to its ability to bypass when set to max
        let lowpass_speed = 300.0 / ((self.last_lowpass - lowpass_chase).abs() + 1.0);
        let highpass_speed = 300.0 / ((self.last_highpass - highpass_chase).abs() + 1.0);
        let wet_speed = 300.0 / ((self.last_wet - wet_chase).abs() + 1.0);
        self.last_lowpass = lowpass_chase;
        self.last_highpass = highpass_chase;
        self.last_wet = wet_chase;

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input = [noise_floor(*out_l as f64, self.fpd_l), noise_floor(*out_r as f64, self.fpd_r)];
            let dry = input;

            // capped at 2, where the one-poles are still stable; it only gets
            // there for samples beyond +-NonLin, so beyond full scale
            let dielectric = [(2.0 - ((input[0] + non_lin) / non_lin)).abs().min(2.0), (2.0 - ((input[1] + non_lin) / non_lin)).abs().min(2.0)];
            self.lowpass_base_amount = ((self.lowpass_base_amount * lowpass_speed) + lowpass_chase) / (lowpass_speed + 1.0);
            self.highpass_base_amount = ((self.highpass_base_amount * highpass_speed) + highpass_chase) / (highpass_speed + 1.0);
            self.wet = ((self.wet * wet_speed) + wet_chase) / (wet_speed + 1.0);

            self.count += 1;
            if self.count > 5 {
                self.count = 0;
            }
            let pairs = SCHEDULE[self.count];
            for c in 0..2 {
                let lowpass_amount = self.lowpass_base_amount * dielectric[c];
                let inv_lowpass = 1.0 - lowpass_amount;
                let highpass_amount = self.highpass_base_amount * dielectric[c];
                let inv_highpass = 1.0 - highpass_amount;
                let ch = &mut self.ch[c];
                let mut x = input[c];
                for &k in pairs.iter() {
                    ch.highpass[k] = (ch.highpass[k] * inv_highpass) + (x * highpass_amount);
                    x -= ch.highpass[k];
                    ch.lowpass[k] = (ch.lowpass[k] * inv_lowpass) + (x * lowpass_amount);
                    x = ch.lowpass[k];
                }
                input[c] = x;
            }

            let wet = self.wet;
            input[0] = (dry[0] * (1.0 - wet)) + (input[0] * non_lin_trim * wet);
            input[1] = (dry[1] * (1.0 - wet)) + (input[1] * non_lin_trim * wet);

            *out_l = dither32(input[0], &mut self.fpd_l) as f32;
            *out_r = dither32(input[1], &mut self.fpd_r) as f32;
        }
    }
}
