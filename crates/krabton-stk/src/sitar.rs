//! `Sitar`: a sitar string (`include/Sitar.h`, `src/Sitar.cpp`).
//!
//! A Karplus-Strong string whose delay length glides from a randomly
//! detuned start to the note (the buzzing bridge), excited by enveloped
//! noise, after Cook's "Real Sound Synthesis for Interactive Applications".
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.
//!
//! Krabton change: `note_off` sets the loop gain to 1 - amplitude like STK,
//! but with the amplitude at least 0.1 and never raising the loop gain, so
//! a note-off with amplitude 0 damps the string instead of letting it ring
//! (with STK's loop gain of 1 it rings longer than without a note-off).

use crate::ugens::*;
use crate::{max_delay_samples, release_amplitude, Model};

pub struct Sitar {
    sr: f64,
    delay_line: DelayA,
    loop_filter: OneZero,
    noise: Noise,
    envelope: Adsr,
    loop_gain: f64,
    am_gain: f64,
    delay: f64,
    target_delay: f64,
    glide_down: f64,
    glide_up: f64,
}

impl Sitar {
    pub fn new(sr: f64) -> Self {
        // The start of a note is detuned by up to 5% (longer delay).
        let n = max_delay_samples(sr, 1.06);
        let delay = 0.5 * n as f64;
        let mut delay_line = DelayA::new(0.5, n);
        delay_line.set_delay(delay);
        let mut envelope = Adsr::new(sr);
        envelope.set_all_times(0.001, 0.04, 0.0, 0.5);
        let mut s = Self {
            sr,
            delay_line,
            loop_filter: OneZero::new(0.01),
            noise: Noise::new(0x5174),
            envelope,
            loop_gain: 0.999,
            am_gain: 0.0,
            delay,
            target_delay: delay,
            glide_down: radius(0.99999, sr),
            glide_up: 1.0 / radius(0.99999, sr),
        };
        s.clear();
        s
    }

    fn set_frequency(&mut self, frequency: f64) {
        self.target_delay = self.sr / frequency;
        self.delay = self.target_delay * (1.0 + 0.05 * self.noise.tick());
        self.delay_line.set_delay(self.delay);
        self.loop_gain = 0.995 + frequency * 0.0000005;
        if self.loop_gain > 0.9995 {
            self.loop_gain = 0.9995;
        }
    }
}

impl Model for Sitar {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        self.set_frequency(frequency);
        self.envelope.key_on();
        self.am_gain = 0.1 * amplitude;
    }

    fn note_off(&mut self, amplitude: f64) {
        self.loop_gain = self.loop_gain.min(1.0 - release_amplitude(amplitude));
    }

    fn control_change(&mut self, _number: u8, _value: f64) {}

    fn tick(&mut self) -> f64 {
        if (self.target_delay - self.delay).abs() > 0.001 {
            if self.target_delay < self.delay {
                self.delay *= self.glide_down;
            } else {
                self.delay *= self.glide_up;
            }
            self.delay_line.set_delay(self.delay);
        }
        let x = self.loop_filter.tick(self.delay_line.last_out() * self.loop_gain)
            + self.am_gain * self.envelope.tick() * self.noise.tick();
        self.delay_line.tick(x)
    }

    fn clear(&mut self) {
        self.delay_line.clear();
        self.loop_filter.clear();
    }
}
