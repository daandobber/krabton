//! Hull2: a three-band EQ split by a Hull-style moving-average treble
//! crossover and a three-pole midbass crossover (`Hull2Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{cycle_end, dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Hull2",
    name: "Hull2",
    category: "EQ",
    description: "A clear, transparent three-band EQ for shaping treble, mids and bass.",
    params: &[
        ParamInfo { name: "Treble", default: 0.5 },
        ParamInfo { name: "Mid", default: 0.5 },
        ParamInfo { name: "Bass", default: 0.5 },
    ],
};

/// `hullL[225]`: three 20-sample stages in a 60-sample ring, each written
/// twice (at `p` and `p + 60`) so the averages can read straight through.
const HULL_LEN: usize = 225;

#[derive(Clone, Copy)]
struct Channel {
    hull: [f64; HULL_LEN],
    hullb: [f64; 5],
}

impl Default for Channel {
    fn default() -> Self {
        Self { hull: [0.0; HULL_LEN], hullb: [0.0; 5] }
    }
}

pub(crate) struct Hull2 {
    overallscale: f64,
    ch: [Channel; 2],
    hullp: usize,
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Hull2 {
    const NUM_PARAMS: usize = 3;

    fn new(sample_rate: f64) -> Self {
        let mut s = Self {
            overallscale: sample_rate / 44100.0,
            ch: [Channel::default(); 2],
            hullp: 1,
            fpd_l: crate::dsp::seed(),
            fpd_r: crate::dsp::seed(),
        };
        s.reset();
        s
    }

    fn reset(&mut self) {
        self.ch = [Channel::default(); 2];
        self.hullp = 1;
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let cycle_end = cycle_end(self.overallscale);
        //max out at 4x, 192k

        let treble = p[0] * 2.0;
        let mid = p[1] * 2.0;
        let bass = p[2] * 2.0;
        let iir_amount = 0.125 / cycle_end as f64;
        let limit = 4 * cycle_end;
        let divisor = 2.0 / limit as f64;

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let input = [noise_floor(*out_l as f64, self.fpd_l), noise_floor(*out_r as f64, self.fpd_r)];
            let mut output = [0.0f64; 2];

            //begin Hull2 treble crossover
            if self.hullp == 0 {
                self.hullp = 60;
            }
            self.hullp -= 1;
            let hullp = self.hullp;

            for c in 0..2 {
                let ch = &mut self.ch[c];
                let dry = input[c];
                let mut mid_sample = hull_stage(&mut ch.hull, hullp, dry, limit, divisor);
                mid_sample = hull_stage(&mut ch.hull, hullp + 20, mid_sample, limit, divisor);
                mid_sample = hull_stage(&mut ch.hull, hullp + 40, mid_sample, limit, divisor);
                let treble_sample = dry - mid_sample;
                //end Hull2 treble crossover

                //begin Hull2 midbass crossover
                let mut bass_sample = mid_sample;
                for x in 0..3 {
                    ch.hullb[x] = (ch.hullb[x] * (1.0 - iir_amount)) + (bass_sample * iir_amount);
                    if ch.hullb[x].abs() < 1.18e-37 {
                        ch.hullb[x] = 0.0;
                    }
                    bass_sample = ch.hullb[x];
                }
                mid_sample -= bass_sample;
                //end Hull2 midbass crossover

                output[c] = (bass_sample * bass) + (mid_sample * mid) + (treble_sample * treble);
            }

            *out_l = dither32(output[0], &mut self.fpd_l) as f32;
            *out_r = dither32(output[1], &mut self.fpd_r) as f32;
        }
    }
}

/// One averaging stage: stores `x` at `base` and `base + 60`, then sums the
/// first half of the `limit` samples from `base` (plus an eighth) and takes
/// an eighth of the second half off again.
#[inline]
fn hull_stage(hull: &mut [f64; HULL_LEN], base: usize, x: f64, limit: usize, divisor: f64) -> f64 {
    hull[base] = x;
    hull[base + 60] = x;
    let mut sum = 0.0;
    for &h in &hull[base..base + (limit / 2)] {
        sum += h * divisor;
    }
    sum += sum * 0.125;
    for &h in &hull[base + (limit / 2)..base + limit] {
        sum -= h * 0.125 * divisor;
    }
    sum
}
