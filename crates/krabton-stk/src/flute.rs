//! `Flute`: a simple flute physical model (`include/Flute.h`,
//! `src/Flute.cpp`).
//!
//! A jet delay and the cubic jet table drive a bore delay line with a
//! one-pole loss filter and a DC blocker, after Karjalainen, Smith and
//! Waryznyk.
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.

use crate::ugens::*;
use crate::{max_delay_samples, release_amplitude, Model};

pub struct Flute {
    sr: f64,
    jet_delay: DelayL,
    bore_delay: DelayL,
    filter: OnePole,
    dc_block: PoleZero,
    noise: Noise,
    adsr: Adsr,
    vibrato: SineWave,
    max_pressure: f64,
    jet_reflection: f64,
    end_reflection: f64,
    noise_gain: f64,
    vibrato_gain: f64,
    output_gain: f64,
    jet_ratio: f64,
    active: bool,
}

impl Flute {
    pub fn new(sr: f64) -> Self {
        // The bore is tuned to 2/3 of the note frequency.
        let n = max_delay_samples(sr, 1.6);
        let mut jet_delay = DelayL::new(0.0, n);
        jet_delay.set_delay(49.0);
        let mut vibrato = SineWave::new(sr);
        vibrato.set_frequency(5.925);
        let mut adsr = Adsr::new(sr);
        adsr.set_all_times(0.005, 0.01, 0.8, 0.010);
        let mut f = Self {
            sr,
            jet_delay,
            bore_delay: DelayL::new(0.0, n),
            filter: OnePole::new(0.7 - 0.1 * 22050.0 / sr),
            dc_block: PoleZero::dc_blocker(sr),
            noise: Noise::new(0xF107),
            adsr,
            vibrato,
            max_pressure: 0.0,
            jet_reflection: 0.5,
            end_reflection: 0.5,
            noise_gain: 0.15,
            vibrato_gain: 0.05,
            output_gain: 0.0,
            jet_ratio: 0.32,
            active: false,
        };
        f.clear();
        f.set_frequency(220.0);
        f
    }

    fn set_frequency(&mut self, frequency: f64) {
        // The flute bore is tuned an octave and a fifth below the note.
        let last_frequency = frequency * 0.66666;
        let delay = self.sr / last_frequency - self.filter.phase_delay(last_frequency, self.sr) - 1.0;
        self.bore_delay.set_delay(delay);
        self.jet_delay.set_delay(delay * self.jet_ratio);
    }

    fn set_jet_delay(&mut self, ratio: f64) {
        self.jet_ratio = ratio;
        self.jet_delay.set_delay(self.bore_delay.delay() * ratio);
    }

    fn start_blowing(&mut self, amplitude: f64, rate: f64) {
        if amplitude <= 0.0 || rate <= 0.0 {
            return;
        }
        self.adsr.set_attack_rate(rate);
        self.max_pressure = amplitude / 0.8;
        self.adsr.key_on();
    }

    fn stop_blowing(&mut self, rate: f64) {
        self.adsr.set_release_rate(rate);
        self.adsr.key_off();
    }
}

impl Model for Flute {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        self.set_frequency(frequency);
        self.start_blowing(1.1 + amplitude * 0.20, rate(amplitude * 0.02, self.sr));
        self.output_gain = amplitude + 0.001;
        self.active = true;
    }

    fn note_off(&mut self, amplitude: f64) {
        self.stop_blowing(rate(release_amplitude(amplitude) * 0.02, self.sr));
        self.active = false;
    }

    fn control_change(&mut self, number: u8, value: f64) {
        let n = value * ONE_OVER_128;
        match number {
            2 => self.set_jet_delay(0.08 + 0.48 * n),
            4 => self.noise_gain = n * 0.4,
            11 => self.vibrato.set_frequency(n * 12.0),
            1 => self.vibrato_gain = n * 0.4,
            128 if self.active => self.adsr.set_target(n),
            _ => {}
        }
    }

    fn tick(&mut self) -> f64 {
        let mut breath_pressure = self.max_pressure * self.adsr.tick();
        breath_pressure +=
            breath_pressure * (self.noise_gain * self.noise.tick() + self.vibrato_gain * self.vibrato.tick());
        let temp = -self.filter.tick(self.bore_delay.last_out());
        let mut pressure_diff = breath_pressure - self.jet_reflection * temp;
        pressure_diff = self.jet_delay.tick(pressure_diff);
        pressure_diff = self.dc_block.tick(jet_table(pressure_diff)) + self.end_reflection * temp;
        0.3 * self.bore_delay.tick(pressure_diff) * self.output_gain
    }

    fn clear(&mut self) {
        self.jet_delay.clear();
        self.bore_delay.clear();
        self.filter.clear();
        self.dc_block.clear();
    }
}
