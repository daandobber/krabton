//! `Bowed`: a bowed string instrument (`include/Bowed.h`, `src/Bowed.cpp`),
//! with contributions by Esteban Maestre (2011).
//!
//! The bow (a friction table) sits between a neck and a bridge delay line;
//! the bridge reflection has a one-pole loss filter and the output goes
//! through six biquads modelling the violin body.
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.

use crate::ugens::*;
use crate::{max_delay_samples, Model, MIN_FREQUENCY};

pub struct Bowed {
    sr: f64,
    neck_delay: DelayL,
    bridge_delay: DelayL,
    bow_table: BowTable,
    string_filter: OnePole,
    body_filters: [BiQuad; 6],
    vibrato: SineWave,
    adsr: Adsr,
    bow_down: bool,
    max_velocity: f64,
    base_delay: f64,
    vibrato_gain: f64,
    beta_ratio: f64,
    active: bool,
}

impl Bowed {
    pub fn new(sr: f64) -> Self {
        // Neck delay plus vibrato excursion (gain up to 0.4).
        let n = max_delay_samples(sr, 1.5);
        let mut neck_delay = DelayL::new(0.0, n);
        neck_delay.set_delay(100.0);
        let mut bridge_delay = DelayL::new(0.0, n);
        bridge_delay.set_delay(29.0);
        let bow_table = BowTable { slope: 3.0, offset: 0.001, ..BowTable::default() };
        let mut vibrato = SineWave::new(sr);
        vibrato.set_frequency(6.12723);
        let mut string_filter = OnePole::new(0.75 - 0.2 * 22050.0 / sr);
        string_filter.gain = 0.95;

        // Violin body filters (coefficients as designed in STK).
        let coefficients: [[f64; 5]; 6] = [
            [1.0, 1.5667, 0.3133, -0.5509, -0.3925],
            [1.0, -1.9537, 0.9542, -1.6357, 0.8697],
            [1.0, -1.6683, 0.8852, -1.7674, 0.8735],
            [1.0, -1.8585, 0.9653, -1.8498, 0.9516],
            [1.0, -1.9299, 0.9621, -1.9354, 0.9590],
            [1.0, -1.9800, 0.9888, -1.9867, 0.9923],
        ];
        let body_filters = coefficients.map(|c| {
            let mut f = BiQuad::default();
            f.set_coefficients(c[0], c[1], c[2], c[3], c[4]);
            f
        });

        let mut adsr = Adsr::new(sr);
        adsr.set_all_times(0.02, 0.005, 0.9, 0.01);
        let mut b = Self {
            sr,
            neck_delay,
            bridge_delay,
            bow_table,
            string_filter,
            body_filters,
            vibrato,
            adsr,
            bow_down: false,
            max_velocity: 0.25,
            base_delay: 0.0,
            vibrato_gain: 0.0,
            beta_ratio: 0.127236,
            active: false,
        };
        b.set_frequency(220.0);
        b.clear();
        b
    }

    fn set_frequency(&mut self, frequency: f64) {
        // Delay = length - approximate filter delay.
        self.base_delay = self.sr / frequency - 4.0;
        if self.base_delay <= 0.0 {
            self.base_delay = 0.3;
        }
        self.bridge_delay.set_delay(self.base_delay * self.beta_ratio);
        self.neck_delay.set_delay(self.base_delay * (1.0 - self.beta_ratio));
    }
}

impl Model for Bowed {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        let r = rate(amplitude * 0.001, self.sr);
        if amplitude > 0.0 && r > 0.0 {
            self.adsr.set_attack_rate(r);
            self.adsr.key_on();
            self.max_velocity = 0.03 + 0.2 * amplitude;
            self.bow_down = true;
        }
        self.set_frequency(frequency);
        self.active = true;
    }

    fn note_off(&mut self, amplitude: f64) {
        // STK: the release rate is (1 - amplitude) * 0.005; keep it above zero.
        let r = rate((1.0 - amplitude.min(0.9)) * 0.005, self.sr);
        self.adsr.set_release_rate(r);
        self.adsr.key_off();
        self.active = false;
    }

    fn control_change(&mut self, number: u8, value: f64) {
        let n = value * ONE_OVER_128;
        match number {
            2 => {
                self.bow_down = n > 0.0;
                self.bow_table.slope = 5.0 - 4.0 * n;
            }
            4 => {
                self.beta_ratio = n;
                self.bridge_delay.set_delay(self.base_delay * self.beta_ratio);
                self.neck_delay.set_delay(self.base_delay * (1.0 - self.beta_ratio));
            }
            11 => self.vibrato.set_frequency(n * 12.0),
            1 => self.vibrato_gain = n * 0.4,
            100 | 128 if self.active => self.adsr.set_target(n),
            // Frequency in Hz (not 0..128), as in STK.
            101 => {
                if value >= MIN_FREQUENCY {
                    self.set_frequency(value)
                }
            }
            _ => {}
        }
    }

    fn tick(&mut self) -> f64 {
        let bow_velocity = self.max_velocity * self.adsr.tick();
        let bridge_reflection = -self.string_filter.tick(self.bridge_delay.last_out());
        let nut_reflection = -self.neck_delay.last_out();
        let string_velocity = bridge_reflection + nut_reflection;
        let delta_v = bow_velocity - string_velocity;

        let new_velocity = if self.bow_down { delta_v * self.bow_table.tick(delta_v) } else { 0.0 };

        self.neck_delay.tick(bridge_reflection + new_velocity);
        self.bridge_delay.tick(nut_reflection + new_velocity);

        if self.vibrato_gain > 0.0 {
            self.neck_delay.set_delay(
                self.base_delay * (1.0 - self.beta_ratio) + self.base_delay * self.vibrato_gain * self.vibrato.tick(),
            );
        }

        let mut y = self.bridge_delay.last_out();
        for f in &mut self.body_filters {
            y = f.tick(y);
        }
        0.1248 * y
    }

    fn clear(&mut self) {
        self.neck_delay.clear();
        self.bridge_delay.clear();
        self.string_filter.clear();
        for f in &mut self.body_filters {
            f.clear();
        }
    }
}
