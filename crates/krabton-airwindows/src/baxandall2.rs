//! Baxandall2: a Baxandall-style treble and bass tone control made of
//! interleaved biquad lowpasses (`Baxandall2Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};
use std::f64::consts::PI;

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Baxandall2",
    name: "Baxandall2",
    category: "EQ",
    description: "Broad, musical treble and bass shelving, like the tone controls on a classic hi-fi amplifier.",
    params: &[ParamInfo { name: "Treble", default: 0.5 }, ParamInfo { name: "Bass", default: 0.5 }],
};

/// One channel's filters in the Airwindows biquad layout
/// (`[freq, reso, a0, a1, a2, b1, b2, state, state]`); set 0 is the `A`
/// filter, set 1 the `B` filter, and they take turns sample by sample.
#[derive(Clone, Copy, Default)]
struct Channel {
    treble: [[f64; 9]; 2],
    bass: [[f64; 9]; 2],
}

pub(crate) struct Baxandall2 {
    sample_rate: f64,
    ch: [Channel; 2],
    flip: bool,
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Baxandall2 {
    const NUM_PARAMS: usize = 2;

    fn new(sample_rate: f64) -> Self {
        let mut s = Self { sample_rate, ch: [Channel::default(); 2], flip: false, fpd_l: crate::dsp::seed(), fpd_r: crate::dsp::seed() };
        s.reset();
        s
    }

    fn reset(&mut self) {
        self.ch = [Channel::default(); 2];
        self.flip = false;
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let treble_gain = 10.0f64.powf(((p[0] * 48.0) - 24.0) / 20.0);
        let mut treble_freq = (4410.0 * treble_gain) / self.sample_rate;
        if treble_freq > 0.45 {
            treble_freq = 0.45;
        }
        let bass_gain = 10.0f64.powf(((p[1] * 48.0) - 24.0) / 20.0);
        let mut bass_freq = 10.0f64.powf(-((p[1] * 48.0) - 24.0) / 20.0);
        bass_freq = (8820.0 * bass_freq) / self.sample_rate;
        if bass_freq > 0.45 {
            bass_freq = 0.45;
        }
        let treble = lowpass(treble_freq, 0.4);
        let bass = lowpass(bass_freq, 0.2);
        for ch in self.ch.iter_mut() {
            for set in 0..2 {
                ch.treble[set][..7].copy_from_slice(&treble);
                ch.bass[set][..7].copy_from_slice(&bass);
            }
        }

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input = [noise_floor(*out_l as f64, self.fpd_l), noise_floor(*out_r as f64, self.fpd_r)];

            let set = if self.flip { 0 } else { 1 };
            for c in 0..2 {
                let ch = &mut self.ch[c];
                let x = input[c];
                let treble_sample = x - biquad_tick(&mut ch.treble[set], x);
                let bass_sample = biquad_tick(&mut ch.bass[set], x);
                input[c] = (bass_sample * bass_gain) + (treble_sample * treble_gain); //interleaved biquad
            }
            self.flip = !self.flip;

            *out_l = dither32(input[0], &mut self.fpd_l) as f32;
            *out_r = dither32(input[1], &mut self.fpd_r) as f32;
        }
    }
}

/// Lowpass coefficients `[freq, reso, a0, a1, a2, b1, b2]`.
fn lowpass(freq: f64, reso: f64) -> [f64; 7] {
    let k = (PI * freq).tan();
    let norm = 1.0 / (1.0 + k / reso + k * k);
    let a0 = k * k * norm;
    [freq, reso, a0, 2.0 * a0, a0, 2.0 * (k * k - 1.0) * norm, (1.0 - k / reso + k * k) * norm]
}

/// The "interleaved biquad" (transposed direct form II).
#[inline]
fn biquad_tick(b: &mut [f64; 9], x: f64) -> f64 {
    let y = (x * b[2]) + b[7];
    b[7] = (x * b[3]) - (y * b[5]) + b[8];
    b[8] = (x * b[4]) - (y * b[6]);
    y
}
