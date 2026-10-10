//! Galactic: a huge, drifting stereo reverb (`GalacticProc.cpp`).
//!
//! Copyright (c) 2016 Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{cycle_end, dither32, fill_last_ref, noise_floor, others, Line};
use crate::{EffectInfo, Kernel, ParamInfo};
use std::f64::consts::PI;

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Galactic",
    name: "Galactic",
    category: "Reverb",
    description: "A huge, lush, slowly drifting stereo reverb for pads and ambient spaces.",
    params: &[
        ParamInfo { name: "Replace", default: 0.5 },
        ParamInfo { name: "Brightness", default: 0.5 },
        ParamInfo { name: "Detune", default: 0.5 },
        ParamInfo { name: "Bigness", default: 1.0 },
        ParamInfo { name: "Dry/Wet", default: 1.0 },
    ],
};

/// Buffer sizes of I, J, K, L (first block), A, B, C, D (second), E, F, G, H (third).
const SIZES: [[usize; 4]; 3] = [[6480, 3660, 1720, 680], [9700, 6000, 2320, 940], [15220, 8460, 4540, 3200]];
/// Delay times at size 1.
const PRIMES: [[f64; 4]; 3] = [[3407.0, 1823.0, 859.0, 331.0], [4801.0, 2909.0, 1153.0, 461.0], [7607.0, 4217.0, 2269.0, 1597.0]];
const DELAY_M: usize = 256;

pub(crate) struct Galactic {
    overallscale: f64,
    iir_a: [f64; 2],
    iir_b: [f64; 2],
    lines: [[Line; 4]; 3],
    a_m: [Box<[f64]>; 2],
    count_m: usize,
    vib_m: f64,
    oldfpd: f64,
    feedback: [[f64; 4]; 2],
    last_ref: [[f64; 7]; 2],
    cycle: usize,
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Galactic {
    const NUM_PARAMS: usize = 5;

    fn new(sample_rate: f64) -> Self {
        let mut s = Self {
            overallscale: sample_rate / 44100.0,
            iir_a: [0.0; 2],
            iir_b: [0.0; 2],
            lines: SIZES.map(|block| block.map(Line::new)),
            a_m: [vec![0.0; 3111].into_boxed_slice(), vec![0.0; 3111].into_boxed_slice()],
            count_m: 1,
            vib_m: 3.0,
            oldfpd: 429496.7295,
            feedback: [[0.0; 4]; 2],
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
        self.a_m[0].fill(0.0);
        self.a_m[1].fill(0.0);
        self.count_m = 1;
        self.vib_m = 3.0;
        self.oldfpd = 429496.7295;
        self.feedback = [[0.0; 4]; 2];
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

        let regen = 0.0625 + ((1.0 - p[0]) * 0.0625);
        let attenuate = (1.0 - (regen / 0.125)) * 1.333;
        let lowpass = (1.00001 - (1.0 - p[1])).powf(2.0) / overallscale.sqrt();
        let drift = p[2].powi(3) * 0.001;
        let size = (p[3] * 1.77) + 0.1;
        let wet = 1.0 - (1.0 - p[4]).powi(3);

        for (block, primes) in self.lines.iter_mut().zip(PRIMES.iter()) {
            for (line, &prime) in block.iter_mut().zip(primes.iter()) {
                line.set_delay(prime * size);
            }
        }

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input_l = noise_floor(*out_l as f64, self.fpd_l);
            let mut input_r = noise_floor(*out_r as f64, self.fpd_r);
            let dry_l = input_l;
            let dry_r = input_r;

            self.vib_m += self.oldfpd * drift;
            if self.vib_m > (PI * 2.0) {
                self.vib_m = 0.0;
                self.oldfpd = 0.4294967295 + (self.fpd_l as f64 * 0.0000000000618);
            }

            self.a_m[0][self.count_m] = input_l * attenuate;
            self.a_m[1][self.count_m] = input_r * attenuate;
            self.count_m += 1;
            if self.count_m > DELAY_M {
                self.count_m = 0;
            }

            let offset_l = (self.vib_m.sin() + 1.0) * 127.0;
            let offset_r = ((self.vib_m + (PI / 2.0)).sin() + 1.0) * 127.0;
            input_l = vibrato_tap(&self.a_m[0], self.count_m, offset_l);
            input_r = vibrato_tap(&self.a_m[1], self.count_m, offset_r);
            //predelay that applies vibrato

            self.iir_a[0] = (self.iir_a[0] * (1.0 - lowpass)) + (input_l * lowpass);
            input_l = self.iir_a[0];
            self.iir_a[1] = (self.iir_a[1] * (1.0 - lowpass)) + (input_r * lowpass);
            input_r = self.iir_a[1];
            //initial filter

            self.cycle += 1;
            if self.cycle == cycle_end {
                //hit the end point and we do a reverb sample
                let mut outs = [[0.0f64; 4]; 2];
                for j in 0..4 {
                    // the left tank takes the right feedback and vice versa
                    let (l, r) = self.lines[0][j].tick(input_l + (self.feedback[1][j] * regen), input_r + (self.feedback[0][j] * regen));
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
                //which we need to feed back into the input again, a bit

                input_l = (outs[0][0] + outs[0][1] + outs[0][2] + outs[0][3]) / 8.0;
                input_r = (outs[1][0] + outs[1][1] + outs[1][2] + outs[1][3]) / 8.0;
                //and take the final combined sum of outputs
                fill_last_ref(&mut self.last_ref[0], cycle_end, input_l);
                fill_last_ref(&mut self.last_ref[1], cycle_end, input_r);
                self.cycle = 0; //reset
            }
            input_l = self.last_ref[0][self.cycle];
            input_r = self.last_ref[1][self.cycle];

            self.iir_b[0] = (self.iir_b[0] * (1.0 - lowpass)) + (input_l * lowpass);
            input_l = self.iir_b[0];
            self.iir_b[1] = (self.iir_b[1] * (1.0 - lowpass)) + (input_r * lowpass);
            input_r = self.iir_b[1];
            //end filter

            if wet < 1.0 {
                input_l = (input_l * wet) + (dry_l * (1.0 - wet));
                input_r = (input_r * wet) + (dry_r * (1.0 - wet));
            }

            *out_l = dither32(input_l, &mut self.fpd_l) as f32;
            *out_r = dither32(input_r, &mut self.fpd_r) as f32;
        }
    }
}

/// The interpolated read of the vibrato predelay (`aML`, `countM`, `delayM`).
#[inline]
fn vibrato_tap(buf: &[f64], count: usize, offset: f64) -> f64 {
    let working = count + offset as usize;
    let frac = offset - offset.floor();
    let wrap = |i: usize| if i > DELAY_M { i - (DELAY_M + 1) } else { i };
    let a = buf[wrap(working).min(buf.len() - 1)] * (1.0 - frac);
    a + buf[wrap(working + 1).min(buf.len() - 1)] * frac
}
