//! Verbity2: a dense 5x5 Householder-matrix reverb (`Verbity2Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{cycle_end, dither32, fill_last_ref, noise_floor, others, unit, Line};
use crate::{EffectInfo, Kernel, ParamInfo};

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Verbity2",
    name: "Verbity2",
    category: "Reverb",
    description: "A smooth, dense room-to-hall reverb with size, sustain and darkness controls.",
    params: &[
        ParamInfo { name: "RmSize", default: 0.5 },
        ParamInfo { name: "Sustain", default: 0.5 },
        ParamInfo { name: "Mulch", default: 0.5 },
        ParamInfo { name: "Wetness", default: 1.0 },
    ],
};

/// Delay times at size 1, per stage: A..E, F..J, K..O, P..T, U..Y
/// ("balanced primes of order two"). The buffers are two longer.
const PRIMES: [[usize; 5]; 5] = [
    [5003, 4349, 3323, 2141, 677],
    [4951, 4157, 2791, 1811, 643],
    [4919, 3929, 2767, 1733, 439],
    [4799, 3529, 2389, 1171, 349],
    [4751, 3329, 2347, 787, 281],
];

pub(crate) struct Verbity2 {
    overallscale: f64,
    iir_a: [f64; 2],
    iir_b: [f64; 2],
    lines: [[Line; 5]; 5],
    feedback: [[f64; 5]; 2],
    previous: [[f64; 5]; 2],
    last_ref: [[f64; 7]; 2],
    cycle: usize,
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Verbity2 {
    const NUM_PARAMS: usize = 4;

    fn new(sample_rate: f64) -> Self {
        let mut s = Self {
            overallscale: sample_rate / 44100.0,
            iir_a: [0.0; 2],
            iir_b: [0.0; 2],
            lines: PRIMES.map(|stage| stage.map(|prime| Line::new(prime + 2))),
            feedback: [[0.0; 5]; 2],
            previous: [[0.0; 5]; 2],
            last_ref: [[0.0; 7]; 2],
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
        for line in self.lines.iter_mut().flatten() {
            line.reset();
        }
        self.feedback = [[0.0; 5]; 2];
        self.previous = [[0.0; 5]; 2];
        self.last_ref = [[0.0; 7]; 2];
        self.cycle = 0;
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let overallscale = self.overallscale;
        let cycle_end = cycle_end(overallscale);
        //this is going to be 2 for 88.1 or 96k, 3 for silly people, 4 for 176 or 192k
        if self.cycle > cycle_end - 1 {
            self.cycle = cycle_end - 1; //sanity check
        }

        //mulch is tone control, darken to obscure the Chrome Oxide, not as much highpass
        let size = (p[0].powf(2.0) * 0.9) + 0.1;
        let regen = (1.0 - (1.0 - p[1]).powf(3.0)) * 0.00032;
        let mulch_setting = p[2];
        let lowpass = (1.0 - (mulch_setting * 0.75)) / overallscale.sqrt();
        let highpass = (0.007 + (mulch_setting * 0.022)) / overallscale.sqrt();
        let interpolate_max = 0.07 + (mulch_setting * 0.4);
        let wet = (p[3] * 2.0).clamp(0.0, 1.0);
        let dry = (2.0 - p[3] * 2.0).clamp(0.0, 1.0);
        //this reverb makes 50% full dry AND full wet, not crossfaded.

        for (stage, primes) in self.lines.iter_mut().zip(PRIMES.iter()) {
            for (line, &prime) in stage.iter_mut().zip(primes.iter()) {
                line.set_delay(prime as f64 * size);
            }
        }

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input_l = noise_floor(*out_l as f64, self.fpd_l);
            let mut input_r = noise_floor(*out_r as f64, self.fpd_r);
            let mut dry_l = input_l;
            let mut dry_r = input_r;

            if self.iir_a[0].abs() < 1.18e-37 {
                self.iir_a[0] = 0.0;
            }
            self.iir_a[0] = (self.iir_a[0] * (1.0 - highpass)) + (input_l * highpass);
            input_l -= self.iir_a[0];
            if self.iir_a[1].abs() < 1.18e-37 {
                self.iir_a[1] = 0.0;
            }
            self.iir_a[1] = (self.iir_a[1] * (1.0 - highpass)) + (input_r * highpass);
            input_r -= self.iir_a[1];
            //first filter is highpass

            let interpolate = [interpolate_max + (interpolate_max * unit(self.fpd_l)), interpolate_max + (interpolate_max * unit(self.fpd_r))];
            //we have our feedback soften also working as Chrome Oxide soften/noise

            self.cycle += 1;
            if self.cycle == cycle_end {
                //hit the end point and we do a reverb sample
                for ch in 0..2 {
                    for j in 0..5 {
                        let fb = (self.feedback[ch][j] * (1.0 - interpolate[ch])) + (self.previous[ch][j] * interpolate[ch]);
                        self.feedback[ch][j] = fb;
                        self.previous[ch][j] = fb;
                    }
                }

                let mut outs = [[0.0f64; 5]; 2];
                for j in 0..5 {
                    let fl = self.feedback[0][j];
                    let fr = self.feedback[1][j];
                    let l = input_l + (fl * (regen * (1.0 - (fl * regen).abs())));
                    let r = input_r + (fr * (regen * (1.0 - (fr * regen).abs())));
                    let (l, r) = self.lines[0][j].tick(l, r);
                    outs[0][j] = l;
                    outs[1][j] = r;
                }
                for stage in 1..5 {
                    let mut next = [[0.0f64; 5]; 2];
                    for j in 0..5 {
                        let l = (outs[0][j] * 3.0) - (others(&outs[0], j) * 2.0);
                        let r = (outs[1][j] * 3.0) - (others(&outs[1], j) * 2.0);
                        let (l, r) = self.lines[stage][j].tick(l, r);
                        next[0][j] = l;
                        next[1][j] = r;
                    }
                    outs = next;
                }
                for j in 0..5 {
                    let from_l = (outs[0][j] * 3.0) - (others(&outs[0], j) * 2.0);
                    let from_r = (outs[1][j] * 3.0) - (others(&outs[1], j) * 2.0);
                    // A, C and E cross over to the other side, B and D stay
                    if j % 2 == 0 {
                        self.feedback[1][j] = from_l;
                        self.feedback[0][j] = from_r;
                    } else {
                        self.feedback[0][j] = from_l;
                        self.feedback[1][j] = from_r;
                    }
                }
                //which we need to feed back into the input again, a bit

                input_l = (outs[0][0] + outs[0][1] + outs[0][2] + outs[0][3] + outs[0][4]) * 0.0016;
                input_r = (outs[1][0] + outs[1][1] + outs[1][2] + outs[1][3] + outs[1][4]) * 0.0016;
                //and take the final combined sum of outputs, corrected for Householder gain
                fill_last_ref(&mut self.last_ref[0], cycle_end, input_l);
                fill_last_ref(&mut self.last_ref[1], cycle_end, input_r);
                self.cycle = 0; //reset
            }
            input_l = self.last_ref[0][self.cycle];
            input_r = self.last_ref[1][self.cycle];

            if self.iir_b[0].abs() < 1.18e-37 {
                self.iir_b[0] = 0.0;
            }
            self.iir_b[0] = (self.iir_b[0] * (1.0 - lowpass)) + (input_l * lowpass);
            input_l = self.iir_b[0];
            if self.iir_b[1].abs() < 1.18e-37 {
                self.iir_b[1] = 0.0;
            }
            self.iir_b[1] = (self.iir_b[1] * (1.0 - lowpass)) + (input_r * lowpass);
            input_r = self.iir_b[1];
            //second filter

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
            //this is our submix verb dry/wet: 0.5 is BOTH at FULL VOLUME

            *out_l = dither32(input_l, &mut self.fpd_l) as f32;
            *out_r = dither32(input_r, &mut self.fpd_r) as f32;
        }
    }
}
