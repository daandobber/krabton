//! Isolator3: a 14-pole highpass (dry minus a lowpass) into a 14-pole
//! lowpass, with coefficients that glide across each block, then ClipOnly2
//! (`Isolator3Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};
use std::f64::consts::PI;

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Isolator3",
    name: "Isolator3",
    category: "EQ",
    description: "A very steep filter that isolates the lows or the highs with one Iso knob, or a narrow band around it as Q goes up.",
    params: &[ParamInfo { name: "Iso", default: 0.5 }, ParamInfo { name: "Q", default: 0.0 }],
};

/// `biq_reso` of the seven stages A..G: together a 14-pole Butterworth.
const RESO: [f64; 7] = [4.46570214, 1.51387132, 0.93979296, 0.70710678, 0.59051105, 0.52972649, 0.50316379];

/// One stage of the "coefficient interpolating biquad filter, stereo".
#[derive(Clone, Copy, Default)]
struct Biquad {
    /// a0, a1, a2, b1, b2 at the start of the block (`biq_aA0` ... `biq_bA2`).
    from: [f64; 5],
    /// The same at the end of the block (`biq_aB0` ... `biq_bB2`).
    to: [f64; 5],
    /// `biq_sL1`, `biq_sL2` and `biq_sR1`, `biq_sR2`.
    s: [[f64; 2]; 2],
}

impl Biquad {
    /// The previous end point becomes the new start point, and the end point
    /// becomes a lowpass at `freq` (a fraction of the sample rate).
    fn retarget(&mut self, freq: f64, reso: f64) {
        self.from = self.to;
        let k = (PI * freq).tan();
        let norm = 1.0 / (1.0 + k / reso + k * k);
        let a0 = k * k * norm;
        self.to = [a0, 2.0 * a0, a0, 2.0 * (k * k - 1.0) * norm, (1.0 - k / reso + k * k) * norm];
    }

    /// The coefficients `temp` of the way back from the end point.
    #[inline]
    fn coefficients(&self, temp: f64) -> [f64; 5] {
        let mut c = [0.0; 5];
        for j in 0..5 {
            c[j] = (self.from[j] * temp) + (self.to[j] * (1.0 - temp));
        }
        c
    }

    /// Transposed direct form II, channel `ch` (0 left, 1 right).
    #[inline]
    fn tick(&mut self, c: &[f64; 5], ch: usize, x: f64) -> f64 {
        let s = &mut self.s[ch];
        let out = (x * c[0]) + s[0];
        s[0] = (x * c[1]) - (out * c[3]) + s[1];
        s[1] = (x * c[2]) - (out * c[4]);
        out
    }
}

/// ClipOnly2 state for one channel.
#[derive(Clone, Copy, Default)]
struct Clip {
    last_sample: f64,
    /// One longer than the original's 16, which its `spacing` clamp of 16
    /// would overrun (here `spacing` stays at 4 or below anyway).
    intermediate: [f64; 17],
    was_pos_clip: bool,
    was_neg_clip: bool,
}

pub(crate) struct Isolator3 {
    overallscale: f64,
    /// `biquadA` ... `biquadG`: the lowpass.
    biquad: [Biquad; 7],
    /// `hiquadA` ... `hiquadG`: the lowpass that is taken away from the dry signal.
    hiquad: [Biquad; 7],
    clip: [Clip; 2],
    /// No block has run since the reset.
    fresh: bool,
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Isolator3 {
    const NUM_PARAMS: usize = 2;

    fn new(sample_rate: f64) -> Self {
        let mut s = Self {
            overallscale: sample_rate / 44100.0,
            biquad: [Biquad::default(); 7],
            hiquad: [Biquad::default(); 7],
            clip: [Clip::default(); 2],
            fresh: true,
            fpd_l: crate::dsp::seed(),
            fpd_r: crate::dsp::seed(),
        };
        s.reset();
        s
    }

    fn reset(&mut self) {
        self.biquad = [Biquad::default(); 7];
        self.hiquad = [Biquad::default(); 7];
        self.clip = [Clip::default(); 2];
        self.fresh = true;
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let frames = left.len();
        if frames == 0 {
            // nothing to glide over; keep the previous end point for the next block
            return;
        }

        let overallscale = self.overallscale;
        //should give us working basic scaling, usually 2 or 4
        let spacing = (overallscale.floor() as i64).clamp(1, 16) as usize;

        let f = p[0];
        let q = p[1];
        let mut iso_freq = ((f * 2.0) * (1.0 - q)) + (f * q);
        let mut hiso_freq = (((f * 2.0) - 1.0) * (1.0 - q)) + (f * q);
        if iso_freq > 1.0 {
            iso_freq = 1.0;
        }
        if hiso_freq < 0.0 {
            hiso_freq = 0.0;
        }

        let mut lowpass = iso_freq.powf(2.0 * overallscale.sqrt()) * 0.4999;
        if lowpass < 0.00025 {
            lowpass = 0.00025;
        }
        let mut highpass = hiso_freq.powf(2.0 * overallscale.sqrt()) * 0.4999;
        if highpass < 0.00025 {
            highpass = 0.00025;
        }
        //previous run through the buffer is still in the filter, so we move it
        //to the A section and now it's the new starting point.
        for (stage, &reso) in self.biquad.iter_mut().zip(RESO.iter()) {
            stage.retarget(lowpass, reso);
        }
        for (stage, &reso) in self.hiquad.iter_mut().zip(RESO.iter()) {
            stage.retarget(highpass, reso);
        }
        if self.fresh {
            // The original glides the first block up from all-zero
            // coefficients, which sets the lowpass's near-Nyquist resonances
            // ringing for a second or more; start at the settings instead.
            for stage in self.biquad.iter_mut().chain(self.hiquad.iter_mut()) {
                stage.from = stage.to;
            }
            self.fresh = false;
        }

        for (i, (out_l, out_r)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
            let input_l = noise_floor(*out_l as f64, self.fpd_l);
            let input_r = noise_floor(*out_r as f64, self.fpd_r);
            let dry_l = input_l;
            let dry_r = input_r;

            let temp = (frames - 1 - i) as f64 / frames as f64;

            let mut x = [input_l, input_r];
            for stage in self.hiquad.iter_mut() {
                let c = stage.coefficients(temp);
                x[0] = stage.tick(&c, 0, x[0]);
                x[1] = stage.tick(&c, 1, x[1]);
            }
            x = [dry_l - x[0], dry_r - x[1]];
            for stage in self.biquad.iter_mut() {
                let c = stage.coefficients(temp);
                x[0] = stage.tick(&c, 0, x[0]);
                x[1] = stage.tick(&c, 1, x[1]);
            }

            let output_l = clip_only2(&mut self.clip[0], x[0], spacing);
            let output_r = clip_only2(&mut self.clip[1], x[1], spacing);

            *out_l = dither32(output_l, &mut self.fpd_l) as f32;
            *out_r = dither32(output_r, &mut self.fpd_r) as f32;
        }
    }
}

/// ClipOnly2, "a little, compressed chunk that can be dropped into code": a
/// clipper that rounds off the samples going into and out of the clip, at
/// the cost of one sample of latency.
#[inline]
fn clip_only2(c: &mut Clip, mut x: f64, spacing: usize) -> f64 {
    x = x.clamp(-4.0, 4.0);
    if c.was_pos_clip {
        //current will be over
        if x < c.last_sample {
            c.last_sample = 0.7058208 + (x * 0.2609148);
        } else {
            c.last_sample = 0.2491717 + (c.last_sample * 0.7390851);
        }
    }
    c.was_pos_clip = false;
    if x > 0.9549925859 {
        c.was_pos_clip = true;
        x = 0.7058208 + (c.last_sample * 0.2609148);
    }
    if c.was_neg_clip {
        //current will be -over
        if x > c.last_sample {
            c.last_sample = -0.7058208 + (x * 0.2609148);
        } else {
            c.last_sample = -0.2491717 + (c.last_sample * 0.7390851);
        }
    }
    c.was_neg_clip = false;
    if x < -0.9549925859 {
        c.was_neg_clip = true;
        x = -0.7058208 + (c.last_sample * 0.2609148);
    }
    c.intermediate[spacing] = x;
    x = c.last_sample; //Latency is however many samples equals one 44.1k sample
    for i in (1..=spacing).rev() {
        c.intermediate[i - 1] = c.intermediate[i];
    }
    c.last_sample = c.intermediate[0]; //run a little buffer to handle this
    x
}
