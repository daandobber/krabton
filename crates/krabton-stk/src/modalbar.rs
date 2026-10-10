//! `ModalBar`: struck bars (`include/ModalBar.h`, `src/ModalBar.cpp`) on
//! top of the `Modal` resonator bank (`include/Modal.h`, `src/Modal.cpp`).
//!
//! A recorded stick strike (`marmstk1.raw`), lowpassed according to the
//! strike amplitude, excites four biquad resonances whose ratios, radii and
//! gains come from nine presets: Marimba, Vibraphone, Agogo, Wood1, Reso,
//! Wood2, Beats, Two Fixed and Clump. Negative ratios are fixed
//! frequencies in Hz.
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.
//!
//! Krabton changes: the mode radii and the strike playback rate are given
//! for 44.1 kHz and converted to the voice's sample rate, and the
//! resonator input is scaled by (44100 / rate)^2, so decay times, the
//! stick sound and the level do not change with the sample rate (in STK
//! the unnormalized resonators get louder with the square of the rate).

use crate::rawwaves;
use crate::ugens::*;
use crate::Model;

const N_MODES: usize = 4;

/// Preset table: per preset the mode ratios, radii, gains, and stick
/// hardness, strike position and direct gain.
#[rustfmt::skip]
const PRESETS: [[[f64; 4]; 4]; 9] = [
    [[1.0, 3.99, 10.65, -2443.0],          // Marimba
     [0.9996, 0.9994, 0.9994, 0.999],
     [0.04, 0.01, 0.01, 0.008],
     [0.429688, 0.445312, 0.093750, 0.0]],
    [[1.0, 2.01, 3.9, 14.37],              // Vibraphone
     [0.99995, 0.99991, 0.99992, 0.9999],
     [0.025, 0.015, 0.015, 0.015],
     [0.390625, 0.570312, 0.078125, 0.0]],
    [[1.0, 4.08, 6.669, -3725.0],          // Agogo
     [0.999, 0.999, 0.999, 0.999],
     [0.06, 0.05, 0.03, 0.02],
     [0.609375, 0.359375, 0.140625, 0.0]],
    [[1.0, 2.777, 7.378, 15.377],          // Wood1
     [0.996, 0.994, 0.994, 0.99],
     [0.04, 0.01, 0.01, 0.008],
     [0.460938, 0.375000, 0.046875, 0.0]],
    [[1.0, 2.777, 7.378, 15.377],          // Reso
     [0.99996, 0.99994, 0.99994, 0.9999],
     [0.02, 0.005, 0.005, 0.004],
     [0.453125, 0.250000, 0.101562, 0.0]],
    [[1.0, 1.777, 2.378, 3.377],           // Wood2
     [0.996, 0.994, 0.994, 0.99],
     [0.04, 0.01, 0.01, 0.008],
     [0.312500, 0.445312, 0.109375, 0.0]],
    [[1.0, 1.004, 1.013, 2.377],           // Beats
     [0.9999, 0.9999, 0.9999, 0.999],
     [0.02, 0.005, 0.005, 0.004],
     [0.398438, 0.296875, 0.070312, 0.0]],
    [[1.0, 4.0, -1320.0, -3960.0],         // 2Fix
     [0.9996, 0.999, 0.9994, 0.999],
     [0.04, 0.01, 0.01, 0.008],
     [0.453125, 0.453125, 0.070312, 0.0]],
    [[1.0, 1.217, 1.475, 1.729],           // Clump
     [0.999, 0.999, 0.999, 0.999],
     [0.03, 0.03, 0.03, 0.03],
     [0.390625, 0.570312, 0.078125, 0.0]],
];

pub const PRESET_NAMES: &[&str] =
    &["Marimba", "Vibraphone", "Agogo", "Wood1", "Reso", "Wood2", "Beats", "Two Fixed", "Clump"];

pub struct ModalBar {
    sr: f64,
    envelope: Envelope,
    wave: WvIn,
    filters: [BiQuad; N_MODES],
    onepole: OnePole,
    vibrato: SineWave,
    ratios: [f64; N_MODES],
    radii: [f64; N_MODES],
    vibrato_gain: f64,
    master_gain: f64,
    direct_gain: f64,
    /// Resonator input scaling that keeps the level independent of the
    /// sample rate (1 at 44.1 kHz).
    rate_gain: f64,
    stick_hardness: f64,
    strike_position: f64,
    base_frequency: f64,
    active: bool,
}

impl ModalBar {
    pub fn new(sr: f64) -> Self {
        let mut filters: [BiQuad; N_MODES] = Default::default();
        for f in &mut filters {
            f.set_equal_gain_zeroes();
        }
        let mut vibrato = SineWave::new(sr);
        vibrato.set_frequency(6.0);
        let mut wave = WvIn::new(rawwaves::marmstk1(), sr);
        wave.set_rate(0.5 * 22050.0 / sr);
        let mut m = Self {
            sr,
            envelope: Envelope::default(),
            wave,
            filters,
            onepole: OnePole::new(0.9),
            vibrato,
            ratios: [0.0; N_MODES],
            radii: [0.0; N_MODES],
            vibrato_gain: 0.0,
            master_gain: 1.0,
            direct_gain: 0.0,
            rate_gain: (REFERENCE_RATE / sr).powi(2),
            stick_hardness: 0.5,
            strike_position: 0.561,
            base_frequency: 440.0,
            active: false,
        };
        m.clear();
        m.set_preset(0);
        m
    }

    fn mode_frequency(&self, ratio: f64) -> f64 {
        if ratio < 0.0 {
            -ratio
        } else {
            ratio * self.base_frequency
        }
    }

    fn set_frequency(&mut self, frequency: f64) {
        self.base_frequency = frequency;
        for i in 0..N_MODES {
            self.set_ratio_and_radius(i, self.ratios[i], self.radii[i]);
        }
    }

    /// `setRatioAndRadius`; `radius` is given for 44.1 kHz.
    fn set_ratio_and_radius(&mut self, i: usize, ratio: f64, radius_44k: f64) {
        let nyquist = self.sr / 2.0;
        if ratio * self.base_frequency < nyquist {
            self.ratios[i] = ratio;
        } else {
            let mut temp = ratio;
            while temp * self.base_frequency > nyquist {
                temp *= 0.5;
            }
            self.ratios[i] = temp;
        }
        self.radii[i] = radius_44k;
        let f = self.mode_frequency(ratio);
        self.filters[i].set_resonance(f, radius(radius_44k, self.sr), false, self.sr);
    }

    fn set_mode_gain(&mut self, i: usize, gain: f64) {
        self.filters[i].gain = gain;
    }

    fn strike(&mut self, amplitude: f64) {
        self.envelope.set_rate(1.0);
        self.envelope.set_target(amplitude);
        self.onepole.set_pole(radius(1.0 - amplitude, self.sr));
        self.envelope.tick();
        self.wave.reset();
        for i in 0..N_MODES {
            let f = self.mode_frequency(self.ratios[i]);
            self.filters[i].set_resonance(f, radius(self.radii[i], self.sr), false, self.sr);
        }
    }

    fn damp(&mut self, amplitude: f64) {
        for i in 0..N_MODES {
            let f = self.mode_frequency(self.ratios[i]);
            self.filters[i].set_resonance(f, radius(self.radii[i] * amplitude, self.sr), false, self.sr);
        }
    }

    fn set_stick_hardness(&mut self, hardness: f64) {
        if !(0.0..=1.0).contains(&hardness) {
            return;
        }
        self.stick_hardness = hardness;
        self.wave.set_rate(rate(0.25 * 4.0f64.powf(hardness), self.sr));
        self.master_gain = 0.1 + 1.8 * hardness;
    }

    fn set_strike_position(&mut self, position: f64) {
        if !(0.0..=1.0).contains(&position) {
            return;
        }
        self.strike_position = position;
        // Hack only first three modes.
        let temp2 = position * PI;
        self.set_mode_gain(0, 0.12 * temp2.sin());
        self.set_mode_gain(1, -0.03 * (0.05 + 3.9 * temp2).sin());
        self.set_mode_gain(2, 0.11 * (-0.05 + 11.0 * temp2).sin());
    }

    fn apply_preset(&mut self, preset: usize) {
        let p = &PRESETS[preset % PRESETS.len()];
        for i in 0..N_MODES {
            self.set_ratio_and_radius(i, p[0][i], p[1][i]);
            self.set_mode_gain(i, p[2][i]);
        }
        self.set_stick_hardness(p[3][0]);
        self.set_strike_position(p[3][1]);
        self.direct_gain = p[3][2];
        self.vibrato_gain = if preset % PRESETS.len() == 1 { 0.2 } else { 0.0 };
    }
}

impl Model for ModalBar {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        self.strike(amplitude);
        self.set_frequency(frequency);
        self.active = true;
    }

    fn note_off(&mut self, amplitude: f64) {
        self.damp(1.0 - amplitude * 0.03);
        self.active = false;
    }

    fn control_change(&mut self, number: u8, value: f64) {
        let n = value * ONE_OVER_128;
        match number {
            2 => self.set_stick_hardness(n),
            4 => self.set_strike_position(n),
            16 => self.apply_preset(value as usize),
            8 => self.direct_gain = n,
            1 => self.vibrato_gain = n * 0.3,
            11 => self.vibrato.set_frequency(n * 12.0),
            128 if self.active => self.envelope.set_target(n),
            _ => {}
        }
    }

    fn set_preset(&mut self, index: usize) {
        self.apply_preset(index);
    }

    fn tick(&mut self) -> f64 {
        let temp = self.master_gain * self.onepole.tick(self.wave.tick() * self.envelope.tick());
        let mut temp2 = 0.0;
        for f in &mut self.filters {
            temp2 += f.tick(temp * self.rate_gain);
        }
        temp2 -= temp2 * self.direct_gain;
        temp2 += self.direct_gain * temp;
        if self.vibrato_gain != 0.0 {
            // Calculate AM and apply to master out.
            let am = 1.0 + self.vibrato.tick() * self.vibrato_gain;
            temp2 *= am;
        }
        temp2
    }

    fn clear(&mut self) {
        self.onepole.clear();
        for f in &mut self.filters {
            f.clear();
        }
    }
}
