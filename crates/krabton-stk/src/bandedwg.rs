//! `BandedWG`: banded waveguide modelling by Georg Essl (1999--2004)
//! (`include/BandedWG.h`, `src/BandedWG.cpp`).
//!
//! One delay line plus bandpass filter per mode of a bar or bowl, struck
//! ("plucked") or bowed through a friction table, with four presets: a
//! uniform bar, a tuned bar, a glass harmonica and a Tibetan bowl. After
//! Essl and Cook, "Banded Waveguides: Towards Physical Modelling of Bar
//! Percussion Instruments" (ICMC 1999).
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.
//!
//! Krabton changes: the number of active modes is recomputed at every note
//! (STK never restores modes dropped by a high note), the Gain control (1)
//! is remembered across notes (STK resets the mode gains at every note),
//! in struck mode `note_off` damps the modes (STK ignores it there), and
//! the struck excitation is scaled with the sample rate so the level does
//! not drop at higher rates.

#![allow(clippy::excessive_precision)]

use crate::ugens::*;
use crate::{max_delay_samples, release_amplitude, Model};

const MAX_BANDED_MODES: usize = 20;

pub const PRESET_NAMES: &[&str] = &["Uniform Bar", "Tuned Bar", "Glass Harmonica", "Tibetan Bowl"];

pub struct BandedWG {
    sr: f64,
    do_pluck: bool,
    track_velocity: bool,
    n_modes: usize,
    preset_modes: usize,
    bow_table: BowTable,
    adsr: Adsr,
    bandpass: [BiQuad; MAX_BANDED_MODES],
    delay: Vec<DelayL>,
    max_velocity: f64,
    modes: [f64; MAX_BANDED_MODES],
    frequency: f64,
    base_gain: f64,
    gains: [f64; MAX_BANDED_MODES],
    basegains: [f64; MAX_BANDED_MODES],
    excitation: [f64; MAX_BANDED_MODES],
    integration_constant: f64,
    velocity_input: f64,
    bow_velocity: f64,
    bow_target: f64,
    bow_position: f64,
    strike_position: usize,
    active: bool,
}

impl BandedWG {
    pub fn new(sr: f64) -> Self {
        let n = max_delay_samples(sr, 1.01);
        let mut adsr = Adsr::new(sr);
        adsr.set_all_times(0.02, 0.005, 0.9, 0.01);
        let mut b = Self {
            sr,
            do_pluck: true,
            track_velocity: false,
            n_modes: 0,
            preset_modes: 0,
            bow_table: BowTable { slope: 3.0, ..BowTable::default() },
            adsr,
            bandpass: Default::default(),
            delay: (0..MAX_BANDED_MODES).map(|_| DelayL::new(0.0, n)).collect(),
            max_velocity: 0.0,
            modes: [0.0; MAX_BANDED_MODES],
            frequency: 220.0,
            base_gain: 0.999,
            gains: [0.0; MAX_BANDED_MODES],
            basegains: [0.0; MAX_BANDED_MODES],
            excitation: [0.0; MAX_BANDED_MODES],
            integration_constant: 0.0,
            velocity_input: 0.0,
            bow_velocity: 0.0,
            bow_target: 0.0,
            bow_position: 0.0,
            strike_position: 0,
            active: false,
        };
        b.apply_preset(0);
        b
    }

    fn apply_preset(&mut self, preset: usize) {
        match preset {
            // Tuned bar
            1 => {
                self.preset_modes = 4;
                self.modes[..4].copy_from_slice(&[1.0, 4.0198391420, 10.7184986595, 18.0697050938]);
                for i in 0..4 {
                    self.basegains[i] = 0.999f64.powi(i as i32 + 1);
                    self.excitation[i] = 1.0;
                }
            }
            // Glass harmonica
            2 => {
                self.preset_modes = 5;
                self.modes[..5].copy_from_slice(&[1.0, 2.32, 4.25, 6.63, 9.38]);
                for i in 0..5 {
                    self.basegains[i] = 0.999f64.powi(i as i32 + 1);
                    self.excitation[i] = 1.0;
                }
            }
            // Tibetan prayer bowl (ICMC'02)
            3 => {
                self.preset_modes = 12;
                let modes = [
                    0.996108344,
                    1.0038916562,
                    2.979178,
                    2.99329767,
                    5.704452,
                    5.704452,
                    8.9982,
                    9.01549726,
                    12.83303,
                    12.807382,
                    17.2808219,
                    21.97602739726,
                ];
                let basegains = [
                    0.999925960128219,
                    0.999925960128219,
                    0.999982774366897,
                    0.999982774366897,
                    1.0,
                    1.0,
                    1.0,
                    1.0,
                    0.999965497558225,
                    0.999965497558225,
                    0.9999999999999999999965497558225,
                    0.999999999999999965497558225,
                ];
                let excitation = [
                    11.900357 / 10.0,
                    11.900357 / 10.0,
                    10.914886 / 10.0,
                    10.914886 / 10.0,
                    42.995041 / 10.0,
                    42.995041 / 10.0,
                    40.063034 / 10.0,
                    40.063034 / 10.0,
                    7.063034 / 10.0,
                    7.063034 / 10.0,
                    57.063034 / 10.0,
                    57.063034 / 10.0,
                ];
                self.modes[..12].copy_from_slice(&modes);
                self.basegains[..12].copy_from_slice(&basegains);
                self.excitation[..12].copy_from_slice(&excitation);
            }
            // Uniform bar
            _ => {
                self.preset_modes = 4;
                self.modes[..4].copy_from_slice(&[1.0, 2.756, 5.404, 8.933]);
                for i in 0..4 {
                    self.basegains[i] = 0.9f64.powi(i as i32 + 1);
                    self.excitation[i] = 1.0;
                }
            }
        }
        self.n_modes = self.preset_modes;
        self.set_frequency(self.frequency);
    }

    fn set_frequency(&mut self, frequency: f64) {
        let frequency = frequency.min(1568.0);
        self.frequency = frequency;
        self.n_modes = self.preset_modes;
        let base = self.sr / frequency;
        for i in 0..self.preset_modes {
            // Calculate the delay line lengths for each mode.
            let length = (base / self.modes[i]).trunc();
            if length > 2.0 {
                self.delay[i].set_delay(length);
                self.gains[i] = self.basegains[i] * self.base_gain;
            } else {
                self.n_modes = i;
                break;
            }
            // Set the bandpass filter resonances.
            let radius = (1.0 - PI * 32.0 / self.sr).max(0.0);
            self.bandpass[i].set_resonance(frequency * self.modes[i], radius, true, self.sr);
            self.delay[i].clear();
            self.bandpass[i].clear();
        }
    }

    fn set_strike_position(&mut self, position: f64) {
        self.strike_position = (self.delay[0].delay() * position / 2.0) as usize;
    }

    fn pluck(&mut self, amplitude: f64) {
        if self.n_modes == 0 {
            return;
        }
        let min_len = self.delay[self.n_modes - 1].delay();
        // The excitation lasts fewer milliseconds at higher rates; keep its
        // level the same as at 44.1 kHz.
        let amplitude = amplitude * self.sr / REFERENCE_RATE;
        for i in 0..self.n_modes {
            let count = (self.delay[i].delay() / min_len) as usize;
            for _ in 0..count {
                self.delay[i].tick(self.excitation[i] * amplitude / self.n_modes as f64);
            }
        }
    }
}

impl Model for BandedWG {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        self.set_frequency(frequency);
        if self.do_pluck {
            self.pluck(amplitude);
        } else {
            self.adsr.set_attack_rate(rate(amplitude * 0.001, self.sr));
            self.adsr.key_on();
            self.max_velocity = 0.03 + 0.1 * amplitude;
        }
        self.active = true;
    }

    fn note_off(&mut self, amplitude: f64) {
        if !self.do_pluck {
            let r = rate((1.0 - amplitude.min(0.9)) * 0.005, self.sr);
            self.adsr.set_release_rate(r);
            self.adsr.key_off();
        } else {
            let damping = (1.0 - release_amplitude(amplitude)) * 0.5;
            for g in &mut self.gains[..self.n_modes] {
                *g *= damping;
            }
        }
        self.active = false;
    }

    fn control_change(&mut self, number: u8, value: f64) {
        let n = value * ONE_OVER_128;
        match number {
            // Bow pressure; zero means striking.
            2 => {
                if n == 0.0 {
                    self.do_pluck = true;
                } else {
                    self.do_pluck = false;
                    self.bow_table.slope = 10.0 - 9.0 * n;
                }
            }
            // Bow motion: bowing by the change of this control.
            4 => {
                self.track_velocity = true;
                self.bow_target += 0.005 * (n - self.bow_position);
                self.bow_position = n;
            }
            8 => self.set_strike_position(n),
            // Bow velocity.
            128 if self.active => {
                self.track_velocity = false;
                self.max_velocity = 0.13 * n;
                self.adsr.set_target(n);
            }
            1 => {
                self.base_gain = 0.8999999999999999 + 0.1 * n;
                for i in 0..self.n_modes {
                    self.gains[i] = self.basegains[i] * self.base_gain;
                }
            }
            11 => self.integration_constant = n,
            64 => self.do_pluck = value < 65.0,
            65 => self.track_velocity = value >= 65.0,
            16 => self.apply_preset(value as usize),
            _ => {}
        }
    }

    fn set_preset(&mut self, index: usize) {
        self.apply_preset(index);
    }

    fn tick(&mut self) -> f64 {
        let mut input = 0.0;
        if !self.do_pluck {
            if self.integration_constant == 0.0 {
                self.velocity_input = 0.0;
            } else {
                self.velocity_input *= self.integration_constant;
            }
            for k in 0..self.n_modes {
                self.velocity_input += self.base_gain * self.delay[k].last_out();
            }
            if self.track_velocity {
                self.bow_velocity *= 0.9995;
                self.bow_velocity += self.bow_target;
                self.bow_target *= 0.995;
            } else {
                self.bow_velocity = self.adsr.tick() * self.max_velocity;
            }
            input = self.bow_velocity - self.velocity_input;
            input *= self.bow_table.tick(input);
            input /= self.n_modes.max(1) as f64;
        }

        let mut data = 0.0;
        for k in 0..self.n_modes {
            let y = self.bandpass[k].tick(input + self.gains[k] * self.delay[k].last_out());
            self.delay[k].tick(y);
            data += y;
        }
        data * 4.0
    }

    fn clear(&mut self) {
        for k in 0..MAX_BANDED_MODES {
            self.delay[k].clear();
            self.bandpass[k].clear();
        }
        self.velocity_input = 0.0;
        self.bow_velocity = 0.0;
    }
}
