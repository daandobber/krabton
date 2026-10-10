//! TapeDelay2: a tape echo with a resonant filter in the loop and flutter
//! (`TapeDelay2Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{cycle_end, dither32, fill_last_ref, multipole_average, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};
use std::f64::consts::PI;

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "TapeDelay2",
    name: "TapeDelay2",
    category: "Delay",
    description: "A warm tape echo with a resonant filter on the repeats and optional flutter.",
    params: &[
        ParamInfo { name: "Time", default: 1.0 },
        ParamInfo { name: "Regen", default: 0.0 },
        ParamInfo { name: "Freq", default: 0.5 },
        ParamInfo { name: "Reso", default: 0.0 },
        ParamInfo { name: "Flutter", default: 0.0 },
        ParamInfo { name: "Dry/Wet", default: 1.0 },
    ],
};

/// The tape loop is 88200 samples long (of the tank rate, 44.1/48 kHz).
const LOOP: usize = 88200;

struct Channel {
    d: Box<[f64]>,
    prev_sample: f64,
    delay: f64,
    sweep: f64,
    regen_filter: [f64; 9],
    out_filter: [f64; 9],
    last_ref: [f64; 10],
}

impl Channel {
    fn new() -> Self {
        Self { d: vec![0.0; 88211].into_boxed_slice(), prev_sample: 0.0, delay: 0.0, sweep: 0.0, regen_filter: [0.0; 9], out_filter: [0.0; 9], last_ref: [0.0; 10] }
    }

    fn reset(&mut self) {
        self.d.fill(0.0);
        self.prev_sample = 0.0;
        self.delay = 0.0;
        self.sweep = 0.0;
        self.regen_filter = [0.0; 9];
        self.out_filter = [0.0; 9];
        self.last_ref = [0.0; 10];
    }

    /// One tank sample: write the new sample (input plus filtered feedback)
    /// onto the "tape" over the stretch the head moved, then read it back.
    #[inline]
    fn tick(&mut self, input: f64, base_speed: f64, vib_speed: f64, feedback: f64) -> f64 {
        let speed = base_speed + (vib_speed * (self.sweep.sin() + 1.0));
        self.sweep += 0.05 * input * input;
        if self.sweep > 6.283185307179586 {
            self.sweep -= 6.283185307179586;
        }
        if !self.sweep.is_finite() {
            self.sweep = 0.0;
        }
        let mut pos = head(self.delay);
        let mut new_sample = input + self.d[pos] * feedback;
        new_sample = biquad_tick(&mut self.regen_filter, new_sample);
        self.delay -= speed;
        if self.delay < 0.0 {
            self.delay += LOOP as f64;
        }
        // keep the head on the tape (the original could land on exactly 88200.0)
        if !(0.0..LOOP as f64).contains(&self.delay) {
            self.delay = self.delay.rem_euclid(LOOP as f64);
            if !(0.0..LOOP as f64).contains(&self.delay) {
                self.delay = 0.0;
            }
        }
        let increment = (new_sample - self.prev_sample) / speed;
        self.d[pos] = self.prev_sample;
        let target = head(self.delay);
        let mut steps = 0;
        while pos != target && steps < LOOP {
            self.d[pos] = self.prev_sample;
            self.prev_sample += increment;
            pos = if pos == 0 { LOOP - 1 } else { pos - 1 };
            steps += 1;
        }
        self.prev_sample = new_sample;
        let out = self.d[target];
        biquad_tick(&mut self.out_filter, out)
    }
}

pub(crate) struct TapeDelay2 {
    overallscale: f64,
    ch: [Channel; 2],
    cycle: usize,
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for TapeDelay2 {
    const NUM_PARAMS: usize = 6;

    fn new(sample_rate: f64) -> Self {
        Self { overallscale: sample_rate / 44100.0, ch: [Channel::new(), Channel::new()], cycle: 0, fpd_l: crate::dsp::seed(), fpd_r: crate::dsp::seed() }
    }

    fn reset(&mut self) {
        for ch in self.ch.iter_mut() {
            ch.reset();
        }
        self.cycle = 0;
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let overallscale = self.overallscale;
        let cycle_end = cycle_end(overallscale);
        if self.cycle > cycle_end - 1 {
            self.cycle = cycle_end - 1; //sanity check
        }

        let base_speed = (p[0].powf(4.0) * 25.0) + 1.0;
        let feedback = p[1].powf(2.0);
        let freq = (p[2].powf(3.0) * 0.4) + 0.0001;
        let reso = p[3].powf(2.0) + 0.01; //resonance
        for ch in self.ch.iter_mut() {
            bandpass(&mut ch.regen_filter, freq, reso);
            bandpass(&mut ch.out_filter, freq, reso * 1.618033988749894848204586);
        }
        let vib_speed = p[4].powf(5.0) * base_speed * ((freq * 0.09) + 0.025); //0.05
        let wet = (p[5] * 2.0).clamp(0.0, 1.0);
        let dry = (2.0 - p[5] * 2.0).clamp(0.0, 1.0);

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input = [noise_floor(*out_l as f64, self.fpd_l), noise_floor(*out_r as f64, self.fpd_r)];
            let mut dry_sample = input;

            self.cycle += 1;
            if self.cycle == cycle_end {
                //hit the end point and we do a delay sample
                for c in 0..2 {
                    let out = self.ch[c].tick(input[c], base_speed, vib_speed, feedback);
                    fill_last_ref(&mut self.ch[c].last_ref, cycle_end, out);
                }
                self.cycle = 0; //reset
            }
            for c in 0..2 {
                let ch = &mut self.ch[c];
                input[c] = ch.last_ref[self.cycle];
                input[c] = multipole_average(&mut ch.last_ref, cycle_end, input[c]);
            }

            if wet < 1.0 {
                input[0] *= wet;
                input[1] *= wet;
            }
            if dry < 1.0 {
                dry_sample[0] *= dry;
                dry_sample[1] *= dry;
            }
            input[0] += dry_sample[0];
            input[1] += dry_sample[1];

            *out_l = dither32(input[0], &mut self.fpd_l) as f32;
            *out_r = dither32(input[1], &mut self.fpd_r) as f32;
        }
    }
}

/// `int pos = floor(delay);` on the tape.
#[inline]
fn head(delay: f64) -> usize {
    (delay.floor() as usize).min(LOOP - 1)
}

/// Bandpass coefficients in the Airwindows biquad layout (`a1` stays 0).
fn bandpass(b: &mut [f64; 9], freq: f64, reso: f64) {
    b[0] = freq;
    b[1] = reso;
    let k = (PI * freq).tan();
    let norm = 1.0 / (1.0 + k / reso + k * k);
    b[2] = k / reso * norm;
    b[4] = -b[2];
    b[5] = 2.0 * (k * k - 1.0) * norm;
    b[6] = (1.0 - k / reso + k * k) * norm;
}

/// The loop filters' transposed direct form II, with the zero `a1` term
/// left out as in the original.
#[inline]
fn biquad_tick(b: &mut [f64; 9], x: f64) -> f64 {
    let y = (x * b[2]) + b[7];
    b[7] = -(y * b[5]) + b[8];
    b[8] = (x * b[4]) - (y * b[6]);
    y
}
