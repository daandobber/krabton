//! `Saxofony`: a faux conical bore reed instrument (`include/Saxofony.h`,
//! `src/Saxofony.cpp`).
//!
//! A "hybrid" digital waveguide: a reed table excites a string-like pair of
//! delay lines (one-zero loss filter) split at the blow position, which
//! gives the conical bore spectrum, after Scavone's waveguide work.
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.

use crate::ugens::*;
use crate::{max_delay_samples, release_amplitude, Model};

pub struct Saxofony {
    sr: f64,
    delays: [DelayL; 2],
    reed_table: ReedTable,
    filter: OneZero,
    envelope: Envelope,
    noise: Noise,
    vibrato: SineWave,
    output_gain: f64,
    noise_gain: f64,
    vibrato_gain: f64,
    position: f64,
    active: bool,
}

impl Saxofony {
    pub fn new(sr: f64) -> Self {
        let n = max_delay_samples(sr, 1.0);
        let mut vibrato = SineWave::new(sr);
        vibrato.set_frequency(5.735);
        let mut s = Self {
            sr,
            delays: [DelayL::new(0.0, n), DelayL::new(0.0, n)],
            reed_table: ReedTable { offset: 0.7, slope: 0.3 },
            filter: OneZero::new(-1.0),
            envelope: Envelope::default(),
            noise: Noise::new(0x5A40),
            vibrato,
            output_gain: 0.3,
            noise_gain: 0.2,
            vibrato_gain: 0.1,
            position: 0.2,
            active: false,
        };
        s.set_frequency(220.0);
        s.clear();
        s
    }

    fn set_frequency(&mut self, frequency: f64) {
        let delay = self.sr / frequency - self.filter.phase_delay(frequency, self.sr) - 1.0;
        self.delays[0].set_delay((1.0 - self.position) * delay);
        self.delays[1].set_delay(self.position * delay);
    }

    fn set_blow_position(&mut self, position: f64) {
        if self.position == position {
            return;
        }
        self.position = position.clamp(0.0, 1.0);
        let total = self.delays[0].delay() + self.delays[1].delay();
        self.delays[0].set_delay((1.0 - self.position) * total);
        self.delays[1].set_delay(self.position * total);
    }
}

impl Model for Saxofony {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        self.set_frequency(frequency);
        let r = rate(amplitude * 0.005, self.sr);
        let a = 0.55 + amplitude * 0.30;
        if r > 0.0 {
            self.envelope.set_rate(r);
            self.envelope.set_target(a);
        }
        self.output_gain = amplitude + 0.001;
        self.active = true;
    }

    fn note_off(&mut self, amplitude: f64) {
        self.envelope.set_rate(rate(release_amplitude(amplitude) * 0.01, self.sr));
        self.envelope.set_target(0.0);
        self.active = false;
    }

    fn control_change(&mut self, number: u8, value: f64) {
        let n = value * ONE_OVER_128;
        match number {
            2 => self.reed_table.slope = 0.1 + 0.4 * n,
            4 => self.noise_gain = n * 0.4,
            29 => self.vibrato.set_frequency(n * 12.0),
            1 => self.vibrato_gain = n * 0.5,
            128 if self.active => self.envelope.set_value(n),
            11 => self.set_blow_position(n),
            26 => self.reed_table.offset = 0.4 + n * 0.6,
            _ => {}
        }
    }

    fn tick(&mut self) -> f64 {
        let mut breath_pressure = self.envelope.tick();
        breath_pressure += breath_pressure * self.noise_gain * self.noise.tick();
        breath_pressure += breath_pressure * self.vibrato_gain * self.vibrato.tick();
        let temp = -0.95 * self.filter.tick(self.delays[0].last_out());
        let out = temp - self.delays[1].last_out();
        let pressure_diff = breath_pressure - out;
        self.delays[1].tick(temp);
        self.delays[0].tick(breath_pressure - pressure_diff * self.reed_table.tick(pressure_diff) - temp);
        out * self.output_gain
    }

    fn clear(&mut self) {
        self.delays[0].clear();
        self.delays[1].clear();
        self.filter.clear();
    }
}
