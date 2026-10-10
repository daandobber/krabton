//! `Brass`: a simple brass instrument (`include/Brass.h`, `src/Brass.cpp`).
//!
//! A lip model (a biquad resonance driven by the pressure difference and
//! squared) excites an allpass-interpolated bore delay line with a DC
//! blocker, after Cook's TBone and HosePlayer.
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.
//!
//! Krabton changes, to make the model speak over the whole range:
//!
//! - STK's lip filter has a fixed radius (0.997) and gain (0.03), so its
//!   DC gain grows with 1 / f^2: below about 800 Hz normal breath pressure
//!   blows the lips wide open and the instrument stays silent (at 220 Hz
//!   only amplitudes around 0.04..0.13 sound, at 110 Hz nothing does).
//!   Here the lip filter of every note is a scaled copy of STK's lip
//!   filter at 880 Hz and 44.1 kHz: the same Q (20.9) and DC gain (1.92).
//! - The bore delay also subtracts the DC blocker's phase delay, like the
//!   other STK winds subtract their loop filters' delays.
//! - The default lip tension is 63 instead of 64, which puts the pitch
//!   within about 1% over 55 Hz..1.5 kHz.
//! - Lip tension (2) and slide length (4) are remembered and re-applied at
//!   every note (STK resets both to neutral at each note), and the vibrato
//!   term fades out with the breath envelope so a released note goes
//!   silent even with vibrato gain up.

use crate::ugens::*;
use crate::{max_delay_samples, release_amplitude, Model};

/// STK's lip filter radius and gain, and the frequency (at 44.1 kHz) whose
/// lip filter shape every note uses.
const STK_LIP_RADIUS: f64 = 0.997;
const STK_LIP_GAIN: f64 = 0.03;
const LIP_REFERENCE_FREQUENCY: f64 = 880.0;

pub struct Brass {
    sr: f64,
    lip_q: f64,
    lip_dc_gain: f64,
    delay_line: DelayA,
    lip_filter: BiQuad,
    dc_block: PoleZero,
    adsr: Adsr,
    vibrato: SineWave,
    lip_target: f64,
    slide_target: f64,
    vibrato_gain: f64,
    max_pressure: f64,
    lip_tension: f64,
    slide_length: f64,
    active: bool,
}

impl Brass {
    pub fn new(sr: f64) -> Self {
        let mut adsr = Adsr::new(sr);
        adsr.set_all_times(0.005, 0.001, 1.0, 0.010);
        let mut vibrato = SineWave::new(sr);
        vibrato.set_frequency(6.137);
        // Q and DC gain of STK's lip filter at the reference frequency.
        let w = TWO_PI * LIP_REFERENCE_FREQUENCY / REFERENCE_RATE;
        let lip_q = LIP_REFERENCE_FREQUENCY / (-STK_LIP_RADIUS.ln() * REFERENCE_RATE / PI);
        let lip_dc_gain =
            STK_LIP_GAIN / (1.0 - 2.0 * STK_LIP_RADIUS * w.cos() + STK_LIP_RADIUS * STK_LIP_RADIUS);
        let mut b = Self {
            sr,
            lip_q,
            lip_dc_gain,
            // Slide length can stretch the bore to 1.5 times twice the period.
            delay_line: DelayA::new(0.5, max_delay_samples(sr, 3.0) + 8),
            lip_filter: BiQuad::default(),
            dc_block: PoleZero::dc_blocker(sr),
            adsr,
            vibrato,
            lip_target: 0.0,
            slide_target: 0.0,
            vibrato_gain: 0.0,
            max_pressure: 0.0,
            lip_tension: 63.0 / 128.0,
            slide_length: 0.5,
            active: false,
        };
        b.clear();
        b.set_frequency(220.0);
        b
    }

    fn set_frequency(&mut self, frequency: f64) {
        // Fudge correction for filter delays; play the second harmonic.
        let d = &self.dc_block;
        self.slide_target = (self.sr / frequency * 2.0) + 3.0
            - phase_delay(&[d.b0, d.b1], &[1.0, d.a1], d.gain, frequency, self.sr);
        self.delay_line.set_delay(self.slide_target);
        self.lip_target = frequency;
        self.set_lip(frequency);
    }

    /// Tunes the lips to `frequency` with a constant Q and DC gain.
    fn set_lip(&mut self, frequency: f64) {
        let r = (-PI * frequency / self.lip_q / self.sr).exp();
        self.lip_filter.set_resonance(frequency, r, false, self.sr);
        let f = &self.lip_filter;
        self.lip_filter.gain = self.lip_dc_gain * (1.0 + f.a1 + f.a2);
    }

    fn apply_lip_tension(&mut self) {
        let f = self.lip_target * 4.0f64.powf(2.0 * self.lip_tension - 1.0);
        self.set_lip(f);
    }

    fn apply_slide_length(&mut self) {
        self.delay_line.set_delay(self.slide_target * (0.5 + self.slide_length));
    }
}

impl Model for Brass {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        self.set_frequency(frequency);
        self.apply_lip_tension();
        self.apply_slide_length();
        let r = rate(amplitude * 0.001, self.sr);
        if amplitude > 0.0 && r > 0.0 {
            self.adsr.set_attack_rate(r);
            self.max_pressure = amplitude;
            self.adsr.key_on();
        }
        self.active = true;
    }

    fn note_off(&mut self, amplitude: f64) {
        self.adsr.set_release_rate(rate(release_amplitude(amplitude) * 0.005, self.sr));
        self.adsr.key_off();
        self.active = false;
    }

    fn control_change(&mut self, number: u8, value: f64) {
        let n = value * ONE_OVER_128;
        match number {
            2 => {
                self.lip_tension = n;
                self.apply_lip_tension();
            }
            4 => {
                self.slide_length = n;
                self.apply_slide_length();
            }
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
        let mouth_pressure = 0.3 * breath_pressure;
        let bore_pressure = 0.85 * self.delay_line.last_out();
        let mut delta_pressure = mouth_pressure - bore_pressure;
        delta_pressure = self.lip_filter.tick(delta_pressure);
        delta_pressure *= delta_pressure;
        if delta_pressure > 1.0 {
            delta_pressure = 1.0;
        }
        let out = delta_pressure * mouth_pressure + (1.0 - delta_pressure) * bore_pressure;
        self.delay_line.tick(self.dc_block.tick(out))
    }

    fn clear(&mut self) {
        self.delay_line.clear();
        self.lip_filter.clear();
        self.dc_block.clear();
    }
}
