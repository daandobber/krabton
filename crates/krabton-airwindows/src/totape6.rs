//! ToTape6: tape machine emulation (`ToTape6Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor, unit};
use crate::{EffectInfo, Kernel, ParamInfo};
use std::f64::consts::PI;

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "ToTape6",
    name: "ToTape6",
    category: "Tape",
    description: "Tape machine emulation with saturation, head bump, softened highs and flutter.",
    params: &[
        ParamInfo { name: "Input", default: 0.5 },
        ParamInfo { name: "Soften", default: 0.5 },
        ParamInfo { name: "Head B", default: 0.5 },
        ParamInfo { name: "Flutter", default: 0.5 },
        ParamInfo { name: "Output", default: 0.5 },
        ParamInfo { name: "Dry/Wet", default: 1.0 },
    ],
};

/// The flutter delay is 500 samples, read with `count-((count > 499)?500:0)`.
const FLUTTER_LEN: usize = 500;

/// One channel's state; `a` and `b` are the two interleaved sets the
/// original alternates between with `flip`.
#[derive(Clone, Copy, Default)]
struct Channel {
    mid_roller: [f64; 2],
    head_bump: [f64; 2],
    biquad_head: [[f64; 9]; 2],
    biquad_main: [[f64; 9]; 2],
    last_sample: f64,
}

pub(crate) struct ToTape6 {
    overallscale: f64,
    d: [Box<[f64]>; 2],
    gcount: i64,
    rateof: f64,
    sweep: f64,
    nextmax: f64,
    ch: [Channel; 2],
    flip: bool,
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for ToTape6 {
    const NUM_PARAMS: usize = 6;

    fn new(sample_rate: f64) -> Self {
        let mut s = Self {
            overallscale: sample_rate / 44100.0,
            d: [vec![0.0; 502].into_boxed_slice(), vec![0.0; 502].into_boxed_slice()],
            gcount: 0,
            rateof: 0.5,
            sweep: PI,
            nextmax: 0.5,
            ch: [Channel::default(); 2],
            flip: false,
            fpd_l: crate::dsp::seed(),
            fpd_r: crate::dsp::seed(),
        };
        s.reset();
        s
    }

    fn reset(&mut self) {
        self.d[0].fill(0.0);
        self.d[1].fill(0.0);
        self.gcount = 0;
        self.rateof = 0.5;
        self.sweep = PI;
        self.nextmax = 0.5;
        self.ch = [Channel::default(); 2];
        self.flip = false;
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let overallscale = self.overallscale;
        let inputgain = 10.0f64.powf(((p[0] - 0.5) * 24.0) / 20.0);
        let soften_control = p[1].powf(2.0);
        let roll_amount = (1.0 - (soften_control * 0.45)) / overallscale;
        let head_bump_control = p[2] * 0.25 * inputgain;
        let head_bump_freq = 0.12 / overallscale;

        // head bump bandpass (biquadA/B) and the main bandpass (biquadC/D)
        let head = bandpass(0.007 / overallscale, 0.0009);
        let main = bandpass(0.032 / overallscale, 0.0007);
        for ch in self.ch.iter_mut() {
            for set in 0..2 {
                ch.biquad_head[set][..7].copy_from_slice(&head[..7]);
                ch.biquad_main[set][..7].copy_from_slice(&main[..7]);
            }
        }

        let depth = p[3].powf(2.0) * overallscale * 70.0;
        let fluttertrim = (0.0024 * p[3].powf(2.0)) / overallscale;
        let outputgain = 10.0f64.powf(((p[4] - 0.5) * 24.0) / 20.0);
        let refclip = 0.99;
        let softness = 0.618033988749894848204586;
        let wet = p[5];

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input = [noise_floor(*out_l as f64, self.fpd_l), noise_floor(*out_r as f64, self.fpd_r)];
            let dry = input;

            if inputgain < 1.0 {
                input[0] *= inputgain;
                input[1] *= inputgain;
            } //gain cut before plugin

            let flutterrandy = unit(self.fpd_l);
            if self.gcount < 0 || self.gcount > 499 {
                self.gcount = 499;
            }
            let g = self.gcount as usize;
            self.d[0][g] = input[0];
            self.d[1][g] = input[1];
            if depth != 0.0 {
                let offset = depth + (depth * self.rateof.powf(2.0) * self.sweep.sin());
                let count = g as i64 + offset.floor() as i64;
                let frac = offset - offset.floor();
                for c in 0..2 {
                    input[c] = self.d[c][wrap(count)] * (1.0 - frac);
                    input[c] += self.d[c][wrap(count + 1)] * frac;
                }
                self.rateof = (self.rateof * (1.0 - fluttertrim)) + (self.nextmax * fluttertrim);
                self.sweep += self.rateof * fluttertrim;
                if self.sweep >= (PI * 2.0) {
                    self.sweep -= PI;
                    self.nextmax = 0.24 + (flutterrandy * 0.74);
                }
            }
            self.gcount -= 1;

            let vib_dry = input;
            let set = if self.flip { 0 } else { 1 };
            let mut highs = [0.0f64; 2];
            for c in 0..2 {
                let ch = &mut self.ch[c];
                ch.mid_roller[set] = (ch.mid_roller[set] * (1.0 - roll_amount)) + (input[c] * roll_amount);
                highs[c] = input[c] - ch.mid_roller[set];

                let mut bump = ch.head_bump[set];
                bump += input[c] * 0.05;
                bump -= bump * bump * bump * head_bump_freq;
                bump = bump.sin();
                bump = biquad_tick(&mut ch.biquad_head[set], bump);
                ch.head_bump[set] = bump.clamp(-1.0, 1.0).asin();

                let x = biquad_tick(&mut ch.biquad_main[set], input[c].sin());
                input[c] = x.clamp(-1.0, 1.0).asin();
            }
            self.flip = !self.flip;

            for c in 0..2 {
                let ch = &mut self.ch[c];
                let ground = vib_dry[c] - input[c]; //set up UnBox on fluttered audio
                let mut x = input[c];
                if inputgain > 1.0 {
                    x *= inputgain;
                }

                let mut apply_soften = highs[c].abs() * 1.57079633;
                if apply_soften > 1.57079633 {
                    apply_soften = 1.57079633;
                }
                apply_soften = 1.0 - apply_soften.cos();
                if highs[c] > 0.0 {
                    x -= apply_soften;
                }
                if highs[c] < 0.0 {
                    x += apply_soften;
                }

                let suppress = (1.0 - x.abs()) * 0.00013;
                for bump in ch.head_bump.iter_mut() {
                    if *bump > suppress {
                        *bump -= suppress;
                    }
                    if *bump < -suppress {
                        *bump += suppress;
                    }
                }
                x += (ch.head_bump[0] + ch.head_bump[1]) * head_bump_control;

                x = x.clamp(-1.0, 1.0);
                let mojo = x.abs().powf(0.25);
                if mojo > 0.0 {
                    x = (x * mojo * PI * 0.5).sin() / mojo;
                }

                x += ground; //apply UnBox processing
                if outputgain != 1.0 {
                    x *= outputgain;
                }

                // ADClip
                let last = &mut ch.last_sample;
                if *last >= refclip {
                    if x < refclip {
                        *last = (refclip * softness) + (x * (1.0 - softness));
                    } else {
                        *last = refclip;
                    }
                }
                if *last <= -refclip {
                    if x > -refclip {
                        *last = (-refclip * softness) + (x * (1.0 - softness));
                    } else {
                        *last = -refclip;
                    }
                }
                if x > refclip {
                    if *last < refclip {
                        x = (refclip * softness) + (*last * (1.0 - softness));
                    } else {
                        x = refclip;
                    }
                }
                if x < -refclip {
                    if *last > -refclip {
                        x = (-refclip * softness) + (*last * (1.0 - softness));
                    } else {
                        x = -refclip;
                    }
                }
                *last = x; //end ADClip
                input[c] = x.clamp(-refclip, refclip);
            }

            if wet != 1.0 {
                input[0] = (input[0] * wet) + (dry[0] * (1.0 - wet));
                input[1] = (input[1] * wet) + (dry[1] * (1.0 - wet));
            }

            *out_l = dither32(input[0], &mut self.fpd_l) as f32;
            *out_r = dither32(input[1], &mut self.fpd_r) as f32;
        }
    }
}

/// The original reads `d[count-((count > 499)?500:0)]`, which only stays in
/// the buffer while the flutter depth fits; wrapping is the same there and
/// keeps very high sample rates in bounds.
#[inline]
fn wrap(count: i64) -> usize {
    count.rem_euclid(FLUTTER_LEN as i64) as usize
}

/// Bandpass coefficients in the Airwindows biquad layout
/// (`[freq, reso, a0, a1, a2, b1, b2, state, state]`).
fn bandpass(freq: f64, reso: f64) -> [f64; 9] {
    let mut b = [0.0; 9];
    b[0] = freq;
    b[1] = reso;
    let k = (PI * freq).tan();
    let norm = 1.0 / (1.0 + k / reso + k * k);
    b[2] = k / reso * norm;
    b[3] = 0.0; // never set in the original, so it stays at its startup 0
    b[4] = -b[2];
    b[5] = 2.0 * (k * k - 1.0) * norm;
    b[6] = (1.0 - k / reso + k * k) * norm;
    b
}

/// The "interleaved biquad" (transposed direct form II).
#[inline]
fn biquad_tick(b: &mut [f64; 9], x: f64) -> f64 {
    let y = (x * b[2]) + b[7];
    b[7] = (x * b[3]) - (y * b[5]) + b[8];
    b[8] = (x * b[4]) - (y * b[6]);
    y
}
