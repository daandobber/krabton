//! `BlowBotl`: a blown bottle (`include/BlowBotl.h`, `src/BlowBotl.cpp`).
//!
//! A Helmholtz resonator (a normalized biquad resonance) driven by a jet
//! nonlinearity and breath noise.
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.
//!
//! Krabton changes: the vibrato term fades out with the breath envelope,
//! so a released note goes silent even with vibrato gain up (in STK the
//! vibrato keeps blowing, noisily, after the note), and the breath noise
//! is scaled by sqrt(rate / 44100) so its spectral density, and the
//! sound, do not change with the sample rate.

use crate::ugens::*;
use crate::{release_amplitude, Model};

/// `__BOTTLE_RADIUS_`, the resonance radius at 44.1 kHz.
const BOTTLE_RADIUS: f64 = 0.999;

pub struct BlowBotl {
    sr: f64,
    bottle_radius: f64,
    resonator: BiQuad,
    dc_block: PoleZero,
    noise: Noise,
    adsr: Adsr,
    vibrato: SineWave,
    max_pressure: f64,
    noise_gain: f64,
    /// Keeps the breath noise density the same at every sample rate.
    noise_scale: f64,
    vibrato_gain: f64,
    output_gain: f64,
    active: bool,
}

impl BlowBotl {
    pub fn new(sr: f64) -> Self {
        let bottle_radius = radius(BOTTLE_RADIUS, sr);
        let mut resonator = BiQuad::default();
        resonator.set_resonance(500.0, bottle_radius, true, sr);
        let mut adsr = Adsr::new(sr);
        adsr.set_all_times(0.005, 0.01, 0.8, 0.010);
        let mut vibrato = SineWave::new(sr);
        vibrato.set_frequency(5.925);
        Self {
            sr,
            bottle_radius,
            resonator,
            dc_block: PoleZero::dc_blocker(sr),
            noise: Noise::new(0xB077),
            adsr,
            vibrato,
            max_pressure: 0.0,
            noise_gain: 20.0,
            noise_scale: (sr / REFERENCE_RATE).sqrt(),
            vibrato_gain: 0.0,
            output_gain: 0.0,
            active: false,
        }
    }
}

impl Model for BlowBotl {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        self.resonator.set_resonance(frequency, self.bottle_radius, true, self.sr);
        let r = rate(amplitude * 0.02, self.sr);
        if r > 0.0 {
            self.adsr.set_attack_rate(r);
            self.max_pressure = 1.1 + amplitude * 0.20;
            self.adsr.key_on();
        }
        self.output_gain = amplitude + 0.001;
        self.active = true;
    }

    fn note_off(&mut self, amplitude: f64) {
        self.adsr.set_release_rate(rate(release_amplitude(amplitude) * 0.02, self.sr));
        self.adsr.key_off();
        self.active = false;
    }

    fn control_change(&mut self, number: u8, value: f64) {
        let n = value * ONE_OVER_128;
        match number {
            4 => self.noise_gain = n * 30.0,
            11 => self.vibrato.set_frequency(n * 12.0),
            1 => self.vibrato_gain = n * 0.4,
            128 if self.active => self.adsr.set_target(n),
            _ => {}
        }
    }

    fn tick(&mut self) -> f64 {
        let envelope = self.adsr.tick();
        let mut breath_pressure = self.max_pressure * envelope;
        breath_pressure += self.vibrato_gain * self.vibrato.tick() * (envelope * 4.0).min(1.0);
        let pressure_diff = breath_pressure - self.resonator.last_out();
        let mut rand_pressure = self.noise_gain * self.noise_scale * self.noise.tick();
        rand_pressure *= breath_pressure;
        rand_pressure *= 1.0 + pressure_diff;
        self.resonator.tick(breath_pressure + rand_pressure - jet_table(pressure_diff) * pressure_diff);
        0.2 * self.output_gain * self.dc_block.tick(pressure_diff)
    }

    fn clear(&mut self) {
        self.resonator.clear();
        self.dc_block.clear();
    }
}
