//! Chamber: a golden-ratio echo chamber reverb (`ChamberProc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{cycle_end, dither32, fill_last_ref, multipole_average, noise_floor, others, Line};
use crate::{EffectInfo, Kernel, ParamInfo};

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Chamber",
    name: "Chamber",
    category: "Reverb",
    description: "An echo-chamber style reverb with size, decay, highpass and lowpass controls.",
    params: &[
        ParamInfo { name: "Bigness", default: 0.35 },
        ParamInfo { name: "Longness", default: 0.35 },
        ParamInfo { name: "Liteness", default: 0.35 },
        ParamInfo { name: "Darkness", default: 0.35 },
        ParamInfo { name: "Wetness", default: 0.35 },
    ],
};

/// Buffer sizes of I, J, K, L (first block), A, B, C, D (second), E, F, G, H (third).
const SIZES: [[usize; 4]; 3] = [[426, 264, 163, 101], [2916, 1804, 1115, 689], [20000, 12361, 7640, 4722]];
const PHI: f64 = 0.618033988749894848204586;

pub(crate) struct Chamber {
    overallscale: f64,
    iir_a: [f64; 2],
    iir_b: [f64; 2],
    iir_c: [f64; 2],
    lines: [[Line; 4]; 3],
    feedback: [[f64; 4]; 2],
    previous: [[f64; 4]; 2],
    last_ref: [[f64; 10]; 2],
    cycle: usize,
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Chamber {
    const NUM_PARAMS: usize = 5;

    fn new(sample_rate: f64) -> Self {
        let mut s = Self {
            overallscale: sample_rate / 44100.0,
            iir_a: [0.0; 2],
            iir_b: [0.0; 2],
            iir_c: [0.0; 2],
            lines: SIZES.map(|block| block.map(Line::new)),
            feedback: [[0.0; 4]; 2],
            previous: [[0.0; 4]; 2],
            last_ref: [[0.0; 10]; 2],
            cycle: 0,
            fpd_l: crate::dsp::seed(),
            fpd_r: crate::dsp::seed(),
        };
        s.reset();
        s
    }

    fn reset(&mut self) {
        self.iir_a = [0.0; 2];
        self.iir_b = [0.0; 2];
        self.iir_c = [0.0; 2];
        for line in self.lines.iter_mut().flatten() {
            line.reset();
        }
        self.feedback = [[0.0; 4]; 2];
        self.previous = [[0.0; 4]; 2];
        self.last_ref = [[0.0; 10]; 2];
        self.cycle = 0;
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let overallscale = self.overallscale;
        let cycle_end = cycle_end(overallscale);
        if self.cycle > cycle_end - 1 {
            self.cycle = cycle_end - 1; //sanity check
        }

        let size = (p[0].powf(2.0) * 0.9) + 0.1;
        let regen = (1.0 - (1.0 - p[1]).powf(6.0)) * 0.123;
        let highpass = p[2].powf(2.0) / overallscale.sqrt();
        let lowpass = (1.0 - p[3].powf(2.0)) / overallscale.sqrt();
        let interpolate = size * 0.381966011250105;
        let wet = (p[4] * 2.0).clamp(0.0, 1.0);
        let dry = (2.0 - p[4] * 2.0).clamp(0.0, 1.0);

        // delayE = 19900*size; delayF = delayE*0.618...; and so on down to
        // delayL, each one truncated to an int.
        let mut delay = (19900.0 * size) as i64;
        for (block, j) in [(2, 0), (2, 1), (2, 2), (2, 3), (1, 0), (1, 1), (1, 2), (1, 3), (0, 0), (0, 1), (0, 2), (0, 3)] {
            self.lines[block][j].set_delay(delay as f64);
            delay = (delay as f64 * PHI) as i64;
        }

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input_l = noise_floor(*out_l as f64, self.fpd_l);
            let mut input_r = noise_floor(*out_r as f64, self.fpd_r);
            let mut dry_l = input_l;
            let mut dry_r = input_r;

            self.iir_c[0] = (self.iir_c[0] * (1.0 - highpass)) + (input_l * highpass);
            input_l -= self.iir_c[0];
            self.iir_c[1] = (self.iir_c[1] * (1.0 - highpass)) + (input_r * highpass);
            input_r -= self.iir_c[1];
            self.iir_a[0] = (self.iir_a[0] * (1.0 - lowpass)) + (input_l * lowpass);
            input_l = self.iir_a[0];
            self.iir_a[1] = (self.iir_a[1] * (1.0 - lowpass)) + (input_r * lowpass);
            input_r = self.iir_a[1];

            self.cycle += 1;
            if self.cycle == cycle_end {
                //hit the end point and we do a reverb sample
                for ch in 0..2 {
                    for j in 0..4 {
                        let fb = (self.feedback[ch][j] * (1.0 - interpolate)) + (self.previous[ch][j] * interpolate);
                        self.feedback[ch][j] = fb;
                        self.previous[ch][j] = fb;
                    }
                }

                let mut outs = [[0.0f64; 4]; 2];
                for j in 0..4 {
                    let (l, r) = self.lines[0][j].tick(input_l + (self.feedback[0][j] * regen), input_r + (self.feedback[1][j] * regen));
                    outs[0][j] = l;
                    outs[1][j] = r;
                }
                for block in 1..3 {
                    let mut next = [[0.0f64; 4]; 2];
                    for j in 0..4 {
                        let l = outs[0][j] - others(&outs[0], j);
                        let r = outs[1][j] - others(&outs[1], j);
                        let (l, r) = self.lines[block][j].tick(l, r);
                        next[0][j] = l;
                        next[1][j] = r;
                    }
                    outs = next;
                }
                for ch in 0..2 {
                    for j in 0..4 {
                        self.feedback[ch][j] = outs[ch][j] - others(&outs[ch], j);
                    }
                }

                input_l = (outs[0][0] + outs[0][1] + outs[0][2] + outs[0][3]) / 8.0;
                input_r = (outs[1][0] + outs[1][1] + outs[1][2] + outs[1][3]) / 8.0;
                fill_last_ref(&mut self.last_ref[0], cycle_end, input_l);
                fill_last_ref(&mut self.last_ref[1], cycle_end, input_r);
                self.cycle = 0; //reset
            }
            input_l = self.last_ref[0][self.cycle];
            input_r = self.last_ref[1][self.cycle];
            input_l = multipole_average(&mut self.last_ref[0], cycle_end, input_l);
            input_r = multipole_average(&mut self.last_ref[1], cycle_end, input_r);

            self.iir_b[0] = (self.iir_b[0] * (1.0 - lowpass)) + (input_l * lowpass);
            input_l = self.iir_b[0];
            self.iir_b[1] = (self.iir_b[1] * (1.0 - lowpass)) + (input_r * lowpass);
            input_r = self.iir_b[1];

            if wet < 1.0 {
                input_l *= wet;
                input_r *= wet;
            }
            if dry < 1.0 {
                dry_l *= dry;
                dry_r *= dry;
            }
            input_l += dry_l;
            input_r += dry_r;

            *out_l = dither32(input_l, &mut self.fpd_l) as f32;
            *out_r = dither32(input_r, &mut self.fpd_r) as f32;
        }
    }
}
