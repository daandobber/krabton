//! `Clarinet`: a simple clarinet physical model (`include/Clarinet.h`,
//! `src/Clarinet.cpp`).
//!
//! A reed table excites a single bore delay line with a one-zero loss
//! filter, after Julius Smith's waveguide reed model.
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.

use crate::ugens::*;
use crate::{max_delay_samples, release_amplitude, Model};

pub struct Clarinet {
    sr: f64,
    delay_line: DelayL,
    reed_table: ReedTable,
    filter: OneZero,
    envelope: Envelope,
    noise: Noise,
    vibrato: SineWave,
    output_gain: f64,
    noise_gain: f64,
    vibrato_gain: f64,
    active: bool,
}

impl Clarinet {
    pub fn new(sr: f64) -> Self {
        let mut vibrato = SineWave::new(sr);
        vibrato.set_frequency(5.735);
        let mut c = Self {
            sr,
            delay_line: DelayL::new(0.0, max_delay_samples(sr, 0.5)),
            reed_table: ReedTable { offset: 0.7, slope: -0.3 },
            filter: OneZero::new(-1.0),
            envelope: Envelope::default(),
            noise: Noise::new(0x0C1A),
            vibrato,
            output_gain: 1.0,
            noise_gain: 0.2,
            vibrato_gain: 0.1,
            active: false,
        };
        c.set_frequency(220.0);
        c.clear();
        c
    }

    fn set_frequency(&mut self, frequency: f64) {
        let delay = (self.sr / frequency) * 0.5 - self.filter.phase_delay(frequency, self.sr) - 1.0;
        self.delay_line.set_delay(delay);
    }

    fn start_blowing(&mut self, amplitude: f64, rate: f64) {
        if amplitude <= 0.0 || rate <= 0.0 {
            return;
        }
        self.envelope.set_rate(rate);
        self.envelope.set_target(amplitude);
    }

    fn stop_blowing(&mut self, rate: f64) {
        self.envelope.set_rate(rate);
        self.envelope.set_target(0.0);
    }
}

impl Model for Clarinet {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        self.set_frequency(frequency);
        self.start_blowing(0.55 + amplitude * 0.30, rate(amplitude * 0.005, self.sr));
        self.output_gain = amplitude + 0.001;
        self.active = true;
    }

    fn note_off(&mut self, amplitude: f64) {
        self.stop_blowing(rate(release_amplitude(amplitude) * 0.01, self.sr));
        self.active = false;
    }

    fn control_change(&mut self, number: u8, value: f64) {
        let n = value * ONE_OVER_128;
        match number {
            2 => self.reed_table.slope = -0.44 + 0.26 * n,
            4 => self.noise_gain = n * 0.4,
            11 => self.vibrato.set_frequency(n * 12.0),
            1 => self.vibrato_gain = n * 0.5,
            128 if self.active => self.envelope.set_value(n),
            _ => {}
        }
    }

    fn tick(&mut self) -> f64 {
        let mut breath_pressure = self.envelope.tick();
        breath_pressure += breath_pressure * self.noise_gain * self.noise.tick();
        breath_pressure += breath_pressure * self.vibrato_gain * self.vibrato.tick();
        let mut pressure_diff = -0.95 * self.filter.tick(self.delay_line.last_out());
        pressure_diff -= breath_pressure;
        let out = self.delay_line.tick(breath_pressure + pressure_diff * self.reed_table.tick(pressure_diff));
        out * self.output_gain
    }

    fn clear(&mut self) {
        self.delay_line.clear();
        self.filter.clear();
    }
}
