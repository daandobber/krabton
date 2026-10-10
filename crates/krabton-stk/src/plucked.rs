//! `Plucked`: a plucked string (`include/Plucked.h`, `src/Plucked.cpp`).
//!
//! The basic Karplus-Strong algorithm: an allpass-interpolated delay line
//! with a one-zero loss filter, filled with lowpassed noise on each pluck.
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.
//!
//! Krabton change: `note_off` sets the loop gain to 1 - amplitude like STK,
//! but with the amplitude at least 0.1 and never raising the loop gain, so
//! a note-off with amplitude 0 damps the string instead of letting it ring
//! (STK's loop gain of 1 would sustain it indefinitely).

use crate::ugens::*;
use crate::{max_delay_samples, release_amplitude, Model};

pub struct Plucked {
    sr: f64,
    delay_line: DelayA,
    loop_filter: OneZero,
    pick_filter: OnePole,
    noise: Noise,
    loop_gain: f64,
}

impl Plucked {
    pub fn new(sr: f64) -> Self {
        let mut p = Self {
            sr,
            delay_line: DelayA::new(0.5, max_delay_samples(sr, 1.0)),
            loop_filter: OneZero::new(-1.0),
            pick_filter: OnePole::new(0.9),
            noise: Noise::new(0x9C4E),
            loop_gain: 0.995,
        };
        p.set_frequency(220.0);
        p
    }

    fn set_frequency(&mut self, frequency: f64) {
        // Delay = length - filter delay.
        let delay = self.sr / frequency - self.loop_filter.phase_delay(frequency, self.sr);
        self.delay_line.set_delay(delay);
        self.loop_gain = 0.995 + frequency * 0.000005;
        if self.loop_gain >= 1.0 {
            self.loop_gain = 0.99999;
        }
    }

    fn pluck(&mut self, amplitude: f64) {
        self.pick_filter.set_pole(0.999 - amplitude * 0.15);
        self.pick_filter.gain = amplitude * 0.5;
        let n = self.delay_line.delay().ceil() as usize;
        for _ in 0..n {
            // Fill the delay line with shaped noise.
            let x = 0.6 * self.delay_line.last_out() + self.pick_filter.tick(self.noise.tick());
            self.delay_line.tick(x);
        }
    }
}

impl Model for Plucked {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        self.set_frequency(frequency);
        self.pluck(amplitude);
    }

    fn note_off(&mut self, amplitude: f64) {
        // STK sets the loop gain to 1 - amplitude; never let a release
        // ring longer than the note itself.
        self.loop_gain = self.loop_gain.min(1.0 - release_amplitude(amplitude));
    }

    fn control_change(&mut self, _number: u8, _value: f64) {}

    fn tick(&mut self) -> f64 {
        let x = self.loop_filter.tick(self.delay_line.last_out() * self.loop_gain);
        3.0 * self.delay_line.tick(x)
    }

    fn clear(&mut self) {
        self.delay_line.clear();
        self.loop_filter.clear();
        self.pick_filter.clear();
    }
}
