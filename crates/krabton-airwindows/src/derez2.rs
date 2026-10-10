//! DeRez2: sample rate and bit depth reduction with a soft/hard character
//! (`DeRez2Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "DeRez2",
    name: "DeRez2",
    category: "Lo-Fi",
    description: "Lowers the sample rate and bit depth for crunchy, retro digital lo-fi.",
    params: &[
        ParamInfo { name: "Rate", default: 1.0 },
        ParamInfo { name: "Rez", default: 1.0 },
        ParamInfo { name: "Hard", default: 1.0 },
        ParamInfo { name: "Dry/Wet", default: 1.0 },
    ],
};

#[derive(Clone, Copy, Default)]
struct Channel {
    last_sample: f64,
    held_sample: f64,
    last_dry_sample: f64,
    last_output_sample: f64,
}

pub(crate) struct DeRez2 {
    overallscale: f64,
    ch: [Channel; 2],
    position: f64,
    increment_a: f64,
    increment_b: f64,
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for DeRez2 {
    const NUM_PARAMS: usize = 4;

    fn new(sample_rate: f64) -> Self {
        Self { overallscale: sample_rate / 44100.0, ch: [Channel::default(); 2], position: 0.0, increment_a: 0.0, increment_b: 0.0, fpd_l: crate::dsp::seed(), fpd_r: crate::dsp::seed() }
    }

    fn reset(&mut self) {
        self.ch = [Channel::default(); 2];
        self.position = 0.0;
        self.increment_a = 0.0;
        self.increment_b = 0.0;
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let mut target_a = p[0].powf(3.0) + 0.0005;
        if target_a > 1.0 {
            target_a = 1.0;
        }
        let soften = (1.0 + target_a) / 2.0;
        let target_b = (1.0 - p[1]).powf(3.0) / 3.0;
        let hard = p[2];
        let wet = p[3];
        target_a /= self.overallscale;
        let log256 = 256.0f64.ln();

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input = [noise_floor(*out_l as f64, self.fpd_l), noise_floor(*out_r as f64, self.fpd_r)];
            let dry = input;

            self.increment_a = ((self.increment_a * 999.0) + target_a) / 1000.0;
            self.increment_b = ((self.increment_b * 999.0) + target_b) / 1000.0;
            //incrementA is the frequency derez
            //incrementB is the bit depth derez
            self.position += self.increment_a;
            let wrapped = self.position > 1.0;
            if wrapped {
                self.position -= 1.0;
            }
            let position = self.position;
            let increment_b = self.increment_b;

            for c in 0..2 {
                let ch = &mut self.ch[c];
                let mut output_sample = ch.held_sample;
                if wrapped {
                    ch.held_sample = (ch.last_sample * position) + (input[c] * (1.0 - position));
                    output_sample = (output_sample * (1.0 - soften)) + (ch.held_sample * soften);
                    //softens the edge of the derez
                }
                let mut x = output_sample;

                if x != ch.last_output_sample {
                    let temp = x;
                    x = (x * hard) + (ch.last_dry_sample * (1.0 - hard));
                    //transitions get an intermediate dry sample
                    ch.last_output_sample = temp; //only one intermediate sample
                } else {
                    ch.last_output_sample = x;
                }
                ch.last_dry_sample = dry[c];
                //freq section of soft/hard interpolates dry samples

                let temp = x;
                x = x.clamp(-1.0, 1.0);
                if x > 0.0 {
                    x = (1.0 + (255.0 * x.abs())).ln() / log256;
                }
                if x < 0.0 {
                    x = -(1.0 + (255.0 * x.abs())).ln() / log256;
                }
                x = (temp * hard) + (x * (1.0 - hard)); //uLaw encode as part of soft/hard

                if increment_b > 0.0005 {
                    if x > 0.0 {
                        x -= quantize_rest(x, increment_b);
                    }
                    if x < 0.0 {
                        x -= quantize_rest(x, increment_b);
                    }
                    x *= 1.0 - increment_b;
                }

                let temp = x;
                x = x.clamp(-1.0, 1.0);
                if x > 0.0 {
                    x = (256.0f64.powf(x.abs()) - 1.0) / 255.0;
                }
                if x < 0.0 {
                    x = -(256.0f64.powf(x.abs()) - 1.0) / 255.0;
                }
                x = (temp * hard) + (x * (1.0 - hard)); //uLaw decode as part of soft/hard

                if wet != 1.0 {
                    x = (x * wet) + (dry[c] * (1.0 - wet));
                }
                ch.last_sample = dry[c];
                input[c] = x;
            }

            *out_l = dither32(input[0], &mut self.fpd_l) as f32;
            *out_r = dither32(input[1], &mut self.fpd_r) as f32;
        }
    }
}

/// `temp = x; while (temp > 0) temp -= step;` (or the mirror image for
/// negative `x`): what is left after stepping past zero. The loop runs at
/// most `|x| / step` times, which is a few thousand for audio; absurdly loud
/// input takes a closed form instead of spinning.
#[inline]
fn quantize_rest(x: f64, step: f64) -> f64 {
    if (x / step).abs() > 100_000.0 {
        let n = (x.abs() / step).ceil();
        return if x > 0.0 { x - n * step } else { x + n * step };
    }
    let mut temp = x;
    if x > 0.0 {
        while temp > 0.0 {
            temp -= step;
        }
    } else {
        while temp < 0.0 {
            temp += step;
        }
    }
    temp
}
