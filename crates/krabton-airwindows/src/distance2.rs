//! Distance2: makes things sound far away, with a chain of slew limiters
//! for the softening of the air and an IIR stage that darkens
//! (`Distance2Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Distance2",
    name: "Distance2",
    category: "EQ",
    description: "Pushes a sound into the distance by softening and darkening it like air does.",
    params: &[
        ParamInfo { name: "Atmosph", default: 0.85 },
        ParamInfo { name: "Darken", default: 0.618 },
        ParamInfo { name: "Dry/Wet", default: 0.618 },
    ],
};

/// `thresholdA` .. `thresholdM` at 44.1 kHz.
const THRESHOLDS: [f64; 13] = [
    0.618033988749894,
    0.679837387624884,
    0.747821126387373,
    0.82260323902611,
    0.904863562928721,
    0.995349919221593,
    1.094884911143752,
    1.204373402258128,
    1.32481074248394,
    1.457291816732335,
    1.603020998405568,
    1.763323098246125,
    1.939655408070737,
];

#[derive(Clone, Copy, Default)]
struct Channel {
    /// `lastSampleA` .. `lastSampleM`: the last 13 dry samples, newest first.
    slew: [f64; 13],
    third_sample: f64,
    last_sample: f64,
}

pub(crate) struct Distance2 {
    overallscale: f64,
    ch: [Channel; 2],
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Distance2 {
    const NUM_PARAMS: usize = 3;

    fn new(sample_rate: f64) -> Self {
        let mut s = Self { overallscale: sample_rate / 44100.0, ch: [Channel::default(); 2], fpd_l: crate::dsp::seed(), fpd_r: crate::dsp::seed() };
        s.reset();
        s
    }

    fn reset(&mut self) {
        self.ch = [Channel::default(); 2];
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let overallscale = self.overallscale;
        let thresholds = THRESHOLDS.map(|t| t / overallscale);
        let mut softslew = (p[0].powf(3.0) * 24.0) + 0.6;
        softslew *= overallscale;
        let filter = softslew * p[1];
        let secondfilter = filter / 3.0;
        let thirdfilter = filter / 5.0;
        let offset_scale = p[0] * 0.1618;
        let levelcorrect = 1.0 + ((filter / 12.0) * p[0]);
        //bring in top slider again to manage boost level for lower settings
        let wet = p[2];
        let silence = silence_offset(offset_scale, wet, softslew, levelcorrect, thresholds[0]);

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let input = [noise_floor(*out_l as f64, self.fpd_l), noise_floor(*out_r as f64, self.fpd_r)];
            let mut output = [0.0f64; 2];
            for c in 0..2 {
                let ch = &mut self.ch[c];
                let dry = input[c];
                let mut x = dry;

                let offset = offset_scale - (ch.last_sample - x);
                x += offset * offset_scale; //extra bit from Loud: offset air compression
                x *= wet; //clean up w. dry introduced
                x *= softslew; //scale into Atmosphere algorithm

                for (&last, &threshold) in ch.slew.iter().zip(thresholds.iter()) {
                    let clamp = x - last;
                    if clamp > threshold {
                        x = last + threshold;
                    }
                    if -clamp > threshold {
                        x = last - threshold;
                    }
                }
                ch.slew.copy_within(0..12, 1);
                ch.slew[0] = dry;
                //store the raw input sample again for use next time

                x *= levelcorrect;
                x /= softslew;
                x -= offset * offset_scale;
                x -= silence; // see silence_offset
                //begin IIR stage
                x += ch.third_sample * thirdfilter;
                x /= thirdfilter + 1.0;
                x += ch.last_sample * secondfilter;
                x /= secondfilter + 1.0;
                //do an IIR like thing to further squish superdistant stuff
                ch.third_sample = ch.last_sample;
                ch.last_sample = x;
                x *= levelcorrect;

                if wet != 1.0 {
                    x = (x * wet) + (dry * (1.0 - wet));
                }
                output[c] = x;
            }

            *out_l = dither32(output[0], &mut self.fpd_l) as f32;
            *out_r = dither32(output[1], &mut self.fpd_r) as f32;
        }
    }
}

/// What digital silence (no input, no history) leaves behind where the
/// offset is taken out again. The offset goes in before `wet * softslew` and
/// the slew clamps and comes out after `levelcorrect / softslew`, so in the
/// original its constant part `offsetScale * offsetScale` does not cancel and
/// silence comes out as a DC offset (about -64 dB at the defaults at 44.1 kHz,
/// -49 dB at 96 kHz, up to -18 dB at full settings). It is subtracted as well,
/// so silence stays silent; where the clamps do not act, that takes out
/// exactly this DC and nothing else.
fn silence_offset(offset_scale: f64, wet: f64, softslew: f64, levelcorrect: f64, threshold_a: f64) -> f64 {
    let offset = offset_scale;
    let mut x = offset * offset_scale;
    x *= wet;
    x *= softslew;
    if x > threshold_a {
        x = threshold_a; // the first clamp; against silence the wider ones never act
    }
    x *= levelcorrect;
    x /= softslew;
    x - (offset * offset_scale)
}
