//! `StifKarp`: a plucked stiff string (`include/StifKarp.h`,
//! `src/StifKarp.cpp`).
//!
//! Karplus-Strong with four allpass biquads in the loop for the dispersion
//! (inharmonic stretching) of a stiff string, and a pickup-position comb
//! filter, after Jaffe and Smith's string extensions.
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.

use crate::ugens::*;
use crate::{max_delay_samples, Model};

pub struct StifKarp {
    sr: f64,
    delay_line: DelayA,
    comb_delay: DelayL,
    filter: OneZero,
    noise: Noise,
    biquad: [BiQuad; 4],
    loop_gain: f64,
    base_loop_gain: f64,
    last_frequency: f64,
    last_length: f64,
    stretching: f64,
    pickup_position: f64,
}

impl StifKarp {
    pub fn new(sr: f64) -> Self {
        let n = max_delay_samples(sr, 1.0);
        let mut s = Self {
            sr,
            delay_line: DelayA::new(0.5, n),
            comb_delay: DelayL::new(0.0, n),
            filter: OneZero::new(-1.0),
            noise: Noise::new(0x571F),
            biquad: Default::default(),
            loop_gain: 0.999,
            base_loop_gain: 0.995,
            last_frequency: 220.0,
            last_length: sr / 220.0,
            stretching: 0.9999,
            pickup_position: 0.4,
        };
        s.clear();
        s.set_frequency(220.0);
        s
    }

    fn set_frequency(&mut self, frequency: f64) {
        self.last_frequency = frequency;
        self.last_length = self.sr / frequency;
        let delay = self.last_length - 0.5;
        self.delay_line.set_delay(delay);
        self.loop_gain = self.base_loop_gain + frequency * 0.000005;
        if self.loop_gain >= 1.0 {
            self.loop_gain = 0.99999;
        }
        self.set_stretch(self.stretching);
        self.comb_delay.set_delay(0.5 * self.pickup_position * self.last_length);
    }

    fn set_stretch(&mut self, stretch: f64) {
        self.stretching = stretch;
        let mut freq = self.last_frequency * 2.0;
        let d_freq = (0.5 * self.sr - freq) * 0.25;
        let mut temp = 0.5 + stretch * 0.5;
        if temp > 0.99999 {
            temp = 0.99999;
        }
        for bq in &mut self.biquad {
            let coefficient = temp * temp;
            bq.a2 = coefficient;
            bq.b0 = coefficient;
            bq.b2 = 1.0;
            let coefficient = -2.0 * temp * (TWO_PI * freq / self.sr).cos();
            bq.a1 = coefficient;
            bq.b1 = coefficient;
            freq += d_freq;
        }
    }

    fn set_pickup_position(&mut self, position: f64) {
        self.pickup_position = position.clamp(0.0, 1.0);
        // Set the pickup position, which puts zeroes at position * length.
        self.comb_delay.set_delay(0.5 * self.pickup_position * self.last_length);
    }

    fn set_base_loop_gain(&mut self, gain: f64) {
        self.base_loop_gain = gain;
        self.loop_gain = self.base_loop_gain + self.last_frequency * 0.000005;
        if self.loop_gain > 0.99999 {
            self.loop_gain = 0.99999;
        }
    }

    fn pluck(&mut self, amplitude: f64) {
        let n = self.last_length.ceil() as usize;
        for _ in 0..n {
            let x = self.delay_line.last_out() * 0.6 + 0.4 * self.noise.tick() * amplitude;
            self.delay_line.tick(x);
        }
    }
}

impl Model for StifKarp {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        self.set_frequency(frequency);
        self.pluck(amplitude);
    }

    fn note_off(&mut self, amplitude: f64) {
        self.loop_gain = (1.0 - amplitude) * 0.5;
    }

    fn control_change(&mut self, number: u8, value: f64) {
        let n = value * ONE_OVER_128;
        match number {
            4 => self.set_pickup_position(n),
            11 => self.set_base_loop_gain(0.97 + n * 0.03),
            1 => self.set_stretch(0.9 + 0.1 * (1.0 - n)),
            _ => {}
        }
    }

    fn tick(&mut self) -> f64 {
        let mut temp = self.delay_line.last_out() * self.loop_gain;
        // Calculate allpass stretching.
        for bq in &mut self.biquad {
            temp = bq.tick(temp);
        }
        // Moving average filter.
        temp = self.filter.tick(temp);
        let y = self.delay_line.tick(temp);
        y - self.comb_delay.tick(y)
    }

    fn clear(&mut self) {
        self.delay_line.clear();
        self.comb_delay.clear();
        self.filter.clear();
        for bq in &mut self.biquad {
            bq.clear();
        }
    }
}
