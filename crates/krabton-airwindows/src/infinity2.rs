//! Infinity2: an endless Householder reverb with tone shaping
//! (`Infinity2Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor, others, Line};
use crate::{EffectInfo, Kernel, ParamInfo};
use std::f64::consts::PI;

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Infinity2",
    name: "Infinity2",
    category: "Reverb",
    description: "A big reverb that can sustain forever, with filter, size, damping and feedback controls.",
    params: &[
        ParamInfo { name: "Filter", default: 1.0 },
        ParamInfo { name: "Size", default: 0.5 },
        ParamInfo { name: "Damping", default: 0.0 },
        ParamInfo { name: "Allpass", default: 1.0 },
        ParamInfo { name: "Feedback", default: 1.0 },
        ParamInfo { name: "Dry/Wet", default: 1.0 },
    ],
};

/// Buffer sizes of A..H, the Householder delays (not sample-rate scaled in
/// the original either).
const TANK_SIZES: [usize; 8] = [8111, 7511, 7311, 6911, 6311, 6111, 5511, 4911];
/// Their delay times at size 1.
const TANK_PRIMES: [f64; 8] = [79.0, 73.0, 71.0, 67.0, 61.0, 59.0, 53.0, 47.0];
/// Buffer sizes of I..L, the allpasses in front.
const ALLPASS_SIZES: [usize; 4] = [4511, 4311, 3911, 3311];
const ALLPASS_PRIMES: [f64; 4] = [43.0, 41.0, 37.0, 31.0];

pub(crate) struct Infinity2 {
    sample_rate: f64,
    biquad_a: [f64; 11],
    biquad_b: [f64; 11],
    biquad_c: [f64; 11],
    tank: [Line; 8],
    allpass: [Line; 4],
    feedback: [[f64; 8]; 2],
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Infinity2 {
    const NUM_PARAMS: usize = 6;

    fn new(sample_rate: f64) -> Self {
        let mut s = Self {
            sample_rate,
            biquad_a: [0.0; 11],
            biquad_b: [0.0; 11],
            biquad_c: [0.0; 11],
            tank: TANK_SIZES.map(Line::new),
            allpass: ALLPASS_SIZES.map(Line::new),
            feedback: [[0.0; 8]; 2],
            fpd_l: crate::dsp::seed(),
            fpd_r: crate::dsp::seed(),
        };
        s.reset();
        s
    }

    fn reset(&mut self) {
        self.biquad_a = [0.0; 11];
        self.biquad_b = [0.0; 11];
        self.biquad_c = [0.0; 11];
        for line in self.tank.iter_mut().chain(self.allpass.iter_mut()) {
            line.reset();
        }
        self.feedback = [[0.0; 8]; 2];
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let freq = ((p[0].powf(2.0) * 9900.0) + 100.0) / self.sample_rate;
        self.biquad_a[0] = freq;
        self.biquad_b[0] = freq;
        self.biquad_c[0] = freq;
        self.biquad_a[1] = 0.618033988749894848204586;
        self.biquad_b[1] = (p[0] * 0.5) + 0.118033988749894848204586;
        self.biquad_c[1] = 0.5;
        let k = (PI * freq).tan(); //lowpass
        lowpass(&mut self.biquad_a, k);
        lowpass(&mut self.biquad_b, k);
        lowpass(&mut self.biquad_c, k);

        let size = (p[1].powf(2.0) * 99.0) + 1.0;
        let damping = p[2].powf(2.0) * 0.5;
        let raw_pass = p[3];
        let feedback = 1.0 - (1.0 - p[4]).powf(4.0);
        let wet = p[5];
        let frac = damping - damping.floor();

        for (line, &prime) in self.tank.iter_mut().zip(TANK_PRIMES.iter()) {
            line.set_delay(prime * size);
        }
        for (line, &prime) in self.allpass.iter_mut().zip(ALLPASS_PRIMES.iter()) {
            line.set_delay(prime * size);
        }

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input_l = noise_floor(*out_l as f64, self.fpd_l);
            let mut input_r = noise_floor(*out_r as f64, self.fpd_r);
            let dry_l = input_l;
            let dry_r = input_r;

            input_l = biquad(&mut self.biquad_a, input_l, 7); //7 and 8 store the L channel
            input_r = biquad(&mut self.biquad_a, input_r, 9); //9 and 10 store the R channel

            let mut pass_l = [input_l; 4];
            let mut pass_r = [input_r; 4];
            for (i, line) in self.allpass.iter_mut().enumerate() {
                let mut temp = line.count + 1;
                if temp > line.delay {
                    temp = 0;
                }
                pass_l[i] -= line.l[temp] * 0.5;
                line.l[line.count] = pass_l[i];
                pass_l[i] *= 0.5;
                pass_r[i] -= line.r[temp] * 0.5;
                line.r[line.count] = pass_r[i];
                pass_r[i] *= 0.5;
                line.count += 1;
                if line.count > line.delay {
                    line.count = 0;
                }
                pass_l[i] += line.l[line.count];
                pass_r[i] += line.r[line.count];
            }
            //the big allpass in front of everything

            if raw_pass != 1.0 {
                for i in 0..4 {
                    pass_l[i] = (pass_l[i] * raw_pass) + (dry_l * (1.0 - raw_pass));
                    pass_r[i] = (pass_r[i] * raw_pass) + (dry_r * (1.0 - raw_pass));
                }
            }

            // A..D and E..H both take allpasses I..L
            let mut infinite = [[0.0f64; 8]; 2];
            for (j, line) in self.tank.iter_mut().enumerate() {
                let (l, r) = line.tick(pass_l[j % 4] + (self.feedback[0][j] * feedback), pass_r[j % 4] + (self.feedback[1][j] * feedback));
                let next = if line.count + 1 > line.delay { 0 } else { line.count + 1 };
                infinite[0][j] = l * (1.0 - frac);
                infinite[0][j] += line.l[next] * frac;
                infinite[1][j] = r * (1.0 - frac);
                infinite[1][j] += line.r[next] * frac;
            }
            //the Householder matrices (shared between channels, offset is stereo)

            for (ch, dry) in [dry_l, dry_r].into_iter().enumerate() {
                let inf = &infinite[ch];
                let dial_a = dial_back(inf[0]);
                let dial_e = dial_back(inf[4]);
                let dial_dry = dial_back(dry);
                //we're compressing things subtly so we can feed energy in and not overload
                let (first, second) = inf.split_at(4);
                for j in 0..4 {
                    let dial = if j == 0 { dial_a } else { dial_dry };
                    self.feedback[ch][j] = (first[j] - others(first, j)) * dial;
                    let dial = if j == 0 { dial_e } else { dial_dry };
                    self.feedback[ch][j + 4] = (second[j] - others(second, j)) * dial;
                }
                //Householder feedback matrix
            }

            let [il, ir] = &infinite;
            input_l = (il[0] + il[1] + il[2] + il[3] + il[4] + il[5] + il[6] + il[7]) / 8.0;
            input_r = (ir[0] + ir[1] + ir[2] + ir[3] + ir[4] + ir[5] + ir[6] + ir[7]) / 8.0;

            input_l = biquad(&mut self.biquad_b, input_l, 7);
            input_r = biquad(&mut self.biquad_b, input_r, 9);

            input_l = clip(input_l);
            input_r = clip(input_r);
            //without this, you can get a NaN condition where it spits out DC offset at full blast!

            input_l = input_l.asin();
            input_r = input_r.asin();

            input_l = biquad(&mut self.biquad_c, input_l, 7);
            input_r = biquad(&mut self.biquad_c, input_r, 9);

            if wet != 1.0 {
                input_l = (input_l * wet) + (dry_l * (1.0 - wet));
                input_r = (input_r * wet) + (dry_r * (1.0 - wet));
            }

            *out_l = dither32(input_l, &mut self.fpd_l) as f32;
            *out_r = dither32(input_r, &mut self.fpd_r) as f32;
        }
    }
}

/// Lowpass coefficients in the Airwindows biquad layout: `b[1]` is the Q,
/// `k = tan(PI * freq)`.
fn lowpass(b: &mut [f64; 11], k: f64) {
    let norm = 1.0 / (1.0 + k / b[1] + k * k);
    b[2] = k * k * norm;
    b[3] = 2.0 * b[2];
    b[4] = b[2];
    b[5] = 2.0 * (k * k - 1.0) * norm;
    b[6] = (1.0 - k / b[1] + k * k) * norm;
}

/// Transposed direct form II; the state lives in `b[s]`, `b[s + 1]`.
#[inline]
fn biquad(b: &mut [f64; 11], x: f64, s: usize) -> f64 {
    let y = (x * b[2]) + b[s];
    b[s] = (x * b[3]) - (y * b[5]) + b[s + 1];
    b[s + 1] = (x * b[4]) - (y * b[6]);
    y
}

/// `dialBack`: 0.5, less once the level passes 0.4. Kept at -0.5 or more:
/// in the original it keeps falling, and past a level of 5.4 the matrix
/// starts to amplify and the tank runs away to infinity (this only changes
/// anything beyond that point).
#[inline]
fn dial_back(x: f64) -> f64 {
    let mut dial = 0.5;
    if x.abs() > 0.4 {
        dial -= (x.abs() - 0.4) * 0.2;
    }
    dial.max(-0.5)
}

/// Hard clip to -1..1 before the `asin` (NaN passes, as in the original).
#[inline]
fn clip(mut x: f64) -> f64 {
    if x > 1.0 {
        x = 1.0;
    }
    if x < -1.0 {
        x = -1.0;
    }
    x
}
