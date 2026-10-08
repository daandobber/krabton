//! One Elements voice: exciters, tube, diffuser, modal resonator or strings
//! (`elements/dsp/voice.h`, `voice.cc`, `patch.h`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::{integral_fractional, semitones_to_ratio};
use crate::envelope::MultistageEnvelope;
use crate::exciter::{Exciter, ExciterModel, FLAG_FALLING_EDGE, FLAG_GATE, FLAG_RISING_EDGE};
use crate::filter::DcBlocker;
use crate::fx::Diffuser;
use crate::resonator::Resonator;
use crate::resources::{LUT_ACCENT_GAIN_COARSE, LUT_ACCENT_GAIN_FINE};
use crate::string::KsString;
use crate::tube::Tube;
use crate::{BLOCK_SIZE, SAMPLE_RATE};

pub const NUM_STRINGS: usize = 5;

/// The front panel of the module, all from 0 to 1 unless noted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Patch {
    pub exciter_envelope_shape: f32,
    pub exciter_bow_level: f32,
    pub exciter_bow_timbre: f32,
    pub exciter_blow_level: f32,
    pub exciter_blow_meta: f32,
    pub exciter_blow_timbre: f32,
    pub exciter_strike_level: f32,
    pub exciter_strike_meta: f32,
    pub exciter_strike_timbre: f32,
    pub exciter_signature: f32,
    pub resonator_geometry: f32,
    pub resonator_brightness: f32,
    pub resonator_damping: f32,
    pub resonator_position: f32,
    /// Normalized frequency (cycles per sample) of the stereo pickup LFO.
    pub resonator_modulation_frequency: f32,
    pub resonator_modulation_offset: f32,
    pub reverb_diffusion: f32,
    pub reverb_lp: f32,
    /// 0 to 2; at 1.75 and above the reverb freezes.
    pub space: f32,
    pub modulation_frequency: f32,
}

impl Default for Patch {
    fn default() -> Self {
        Self {
            exciter_envelope_shape: 1.0,
            exciter_bow_level: 0.0,
            exciter_bow_timbre: 0.5,
            exciter_blow_level: 0.0,
            exciter_blow_meta: 0.5,
            exciter_blow_timbre: 0.5,
            exciter_strike_level: 0.8,
            exciter_strike_meta: 0.5,
            exciter_strike_timbre: 0.5,
            exciter_signature: 0.0,
            resonator_geometry: 0.2,
            resonator_brightness: 0.5,
            resonator_damping: 0.25,
            resonator_position: 0.3,
            resonator_modulation_frequency: 0.5 / SAMPLE_RATE,
            resonator_modulation_offset: 0.1,
            reverb_diffusion: 0.625,
            reverb_lp: 0.7,
            space: 0.5,
            modulation_frequency: 0.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ResonatorModel {
    #[default]
    Modal,
    String,
    Strings,
}

const CHORDS: [[f32; NUM_STRINGS]; 11] = [
    [0.0, -12.0, 0.0, 0.01, 12.0],
    [0.0, -12.0, 3.0, 7.0, 10.0],
    [0.0, -12.0, 3.0, 7.0, 12.0],
    [0.0, -12.0, 3.0, 7.0, 14.0],
    [0.0, -12.0, 3.0, 7.0, 17.0],
    [0.0, -12.0, 7.0, 12.0, 19.0],
    [0.0, -12.0, 4.0, 7.0, 17.0],
    [0.0, -12.0, 4.0, 7.0, 14.0],
    [0.0, -12.0, 4.0, 7.0, 12.0],
    [0.0, -12.0, 4.0, 7.0, 11.0],
    [0.0, -12.0, 5.0, 7.0, 12.0],
];

pub struct Voice {
    envelope: MultistageEnvelope,
    tube: Tube,
    bow: Exciter,
    blow: Exciter,
    strike: Exciter,
    diffuser: Diffuser,
    resonator: Resonator,
    strings: Vec<KsString>,
    dc_blocker: DcBlocker,
    strength: f32,
    envelope_value: f32,
    exciter_level: f32,
    bow_buffer: [f32; BLOCK_SIZE],
    bow_strength_buffer: [f32; BLOCK_SIZE],
    blow_buffer: [f32; BLOCK_SIZE],
    strike_buffer: [f32; BLOCK_SIZE],
    previous_gate: bool,
    resonator_model: ResonatorModel,
    chord_index: f32,
}

impl Default for Voice {
    fn default() -> Self {
        Self::new()
    }
}

impl Voice {
    pub fn new() -> Self {
        let mut v = Self {
            envelope: MultistageEnvelope::default(),
            tube: Tube::default(),
            bow: Exciter::default(),
            blow: Exciter::default(),
            strike: Exciter::default(),
            diffuser: Diffuser::default(),
            resonator: Resonator::default(),
            strings: (0..NUM_STRINGS).map(|_| KsString::new(true)).collect(),
            dc_blocker: DcBlocker::new(1.0 - 10.0 / SAMPLE_RATE),
            strength: 0.0,
            envelope_value: 0.0,
            exciter_level: 0.0,
            bow_buffer: [0.0; BLOCK_SIZE],
            bow_strength_buffer: [0.0; BLOCK_SIZE],
            blow_buffer: [0.0; BLOCK_SIZE],
            strike_buffer: [0.0; BLOCK_SIZE],
            previous_gate: false,
            resonator_model: ResonatorModel::Modal,
            chord_index: 0.0,
        };
        v.resonator.set_resolution(52); // Runs with 56 extremely tightly.
        v.bow.set_model(ExciterModel::Flow);
        v.bow.set_parameter(0.7);
        v.bow.set_timbre(0.5);
        v.blow.set_model(ExciterModel::GranularSamplePlayer);
        v.envelope.set_adsr(0.5, 0.5, 0.5, 0.5);
        v
    }

    /// Silences the resonator (and strings) at once.
    pub fn panic(&mut self) {
        self.resonator.reset();
        self.resonator.set_resolution(52);
        for s in self.strings.iter_mut() {
            s.reset();
        }
        self.dc_blocker = DcBlocker::new(1.0 - 10.0 / SAMPLE_RATE);
    }

    pub fn exciter_level(&self) -> f32 {
        self.exciter_level
    }

    pub fn set_resonator_model(&mut self, model: ResonatorModel) {
        self.resonator_model = model;
    }

    fn gate_flags(&mut self, gate_in: bool) -> u8 {
        let mut flags = 0;
        if gate_in {
            if !self.previous_gate {
                flags |= FLAG_RISING_EDGE;
            }
            flags |= FLAG_GATE;
        } else if self.previous_gate {
            flags = FLAG_FALLING_EDGE;
        }
        self.previous_gate = gate_in;
        flags
    }

    /// Renders one block (at most [`BLOCK_SIZE`] samples). `frequency` is
    /// normalized (cycles per sample at 32 kHz), `strength` goes from 0 to
    /// 1 (0.5 is neutral). `blow_in` and `strike_in` are external exciter
    /// signals.
    pub fn process(
        &mut self,
        patch: &Patch,
        frequency: f32,
        strength: f32,
        gate_in: bool,
        blow_in: &[f32],
        strike_in: &[f32],
        raw: &mut [f32],
        center: &mut [f32],
        sides: &mut [f32],
    ) {
        let size = raw.len().min(BLOCK_SIZE);
        let (raw, center, sides) = (&mut raw[..size], &mut center[..size], &mut sides[..size]);
        let flags = self.gate_flags(gate_in);

        // Compute the envelope.
        let mut envelope_gain = 1.0;
        let shape = patch.exciter_envelope_shape;
        if shape < 0.4 {
            let a = shape * 0.75 + 0.15;
            let dr = a * 1.8;
            self.envelope.set_adsr(a, dr, 0.0, dr);
            envelope_gain = 5.0 - shape * 10.0;
        } else if shape < 0.6 {
            let s = (shape - 0.4) * 5.0;
            self.envelope.set_adsr(0.45, 0.81, s, 0.81);
        } else {
            let a = (1.0 - shape) * 0.75 + 0.15;
            let dr = a * 1.8;
            self.envelope.set_adsr(a, dr, 1.0, dr);
        }
        let envelope_value = self.envelope.process(flags) * envelope_gain;
        let envelope_increment = (envelope_value - self.envelope_value) / size as f32;

        // Configure and evaluate exciters.
        let brightness_factor = 0.4 + 0.6 * patch.resonator_brightness;
        self.bow.set_timbre(patch.exciter_bow_timbre * brightness_factor);

        self.blow.set_parameter(patch.exciter_blow_meta);
        self.blow.set_timbre(patch.exciter_blow_timbre);
        self.blow.set_signature(patch.exciter_signature);

        let strike_meta = patch.exciter_strike_meta;
        self.strike.set_meta(
            if strike_meta <= 0.4 { strike_meta * 0.625 } else { strike_meta * 1.25 - 0.25 },
            ExciterModel::SamplePlayer,
            ExciterModel::Particles,
        );
        self.strike.set_timbre(patch.exciter_strike_timbre);
        self.strike.set_signature(patch.exciter_signature);

        self.bow.process(flags, &mut self.bow_buffer[..size]);

        let mut blow_level = patch.exciter_blow_level * 1.5;
        let tube_level = if blow_level > 1.0 { (blow_level - 1.0) * 2.0 } else { 0.0 };
        blow_level = if blow_level < 1.0 { blow_level * 0.4 } else { 0.4 };
        self.blow.process(flags, &mut self.blow_buffer[..size]);
        self.tube.process(frequency, envelope_value, patch.resonator_damping, tube_level, &mut self.blow_buffer[..size], tube_level * 0.5);

        for i in 0..size {
            self.blow_buffer[i] = self.blow_buffer[i] * blow_level + blow_in.get(i).copied().unwrap_or(0.0);
        }
        self.diffuser.process(&mut self.blow_buffer[..size]);
        self.strike.process(flags, &mut self.strike_buffer[..size]);

        // The strike exciter is implemented in such a way that raising the
        // level beyond a certain point doesn't change the exciter amplitude,
        // but instead, increasingly mixes the raw exciter signal into the
        // resonator output.
        let mut strike_level = patch.exciter_strike_level * 1.25;
        let strike_bleed = if strike_level > 1.0 { (strike_level - 1.0) * 2.0 } else { 0.0 };
        strike_level = strike_level.min(1.0) * 1.5;

        // The strength parameter is very sensitive to zipper noise.
        let strength = strength.clamp(0.0, 1.0) * 256.0;
        let strength_increment = (strength - self.strength) / size as f32;

        // Sum all sources of excitation.
        for i in 0..size {
            self.strength += strength_increment;
            self.envelope_value += envelope_increment;
            let mut input_sample = 0.0;
            let mut e = self.envelope_value;
            let (si, sf) = integral_fractional(self.strength.clamp(0.0, 256.0));
            let accent = LUT_ACCENT_GAIN_COARSE[si as usize] * LUT_ACCENT_GAIN_FINE[((256.0 * sf) as usize).min(256)];
            self.bow_strength_buffer[i] = e * patch.exciter_bow_level;

            self.strike_buffer[i] *= accent;
            e *= accent;

            input_sample += self.bow_buffer[i] * self.bow_strength_buffer[i] * 0.125 * accent;
            input_sample += self.blow_buffer[i] * e;
            input_sample += self.strike_buffer[i] * strike_level;
            input_sample += strike_in.get(i).copied().unwrap_or(0.0);
            raw[i] = input_sample * 0.5;
        }

        // Update meter for exciter.
        for &x in raw.iter() {
            let error = x * x - self.exciter_level;
            self.exciter_level += error * if error > 0.0 { 0.5 } else { 0.001 };
        }

        // Some exciters can cause palm mutes on release.
        let mut damping = patch.resonator_damping;
        damping -= self.strike.damping() * strike_level * 0.125;
        damping -= (1.0 - self.bow_strength_buffer[0]) * patch.exciter_bow_level * 0.0625;
        damping = damping.max(0.0);

        // Configure resonator.
        if self.resonator_model == ResonatorModel::Modal {
            self.resonator.set_frequency(frequency);
            self.resonator.set_geometry(patch.resonator_geometry);
            self.resonator.set_brightness(patch.resonator_brightness);
            self.resonator.set_position(patch.resonator_position);
            self.resonator.set_damping(damping);
            self.resonator.set_modulation_frequency(patch.resonator_modulation_frequency);
            self.resonator.set_modulation_offset(patch.resonator_modulation_offset);
            self.resonator.process(&self.bow_strength_buffer[..size], raw, center, sides);
        } else {
            let num_notes = if self.resonator_model == ResonatorModel::String { 1 } else { NUM_STRINGS };
            let normalization = 1.0 / num_notes as f32;
            self.dc_blocker.process(raw);
            for x in raw.iter_mut() {
                *x *= normalization;
            }
            let chord = patch.resonator_geometry * 10.0;
            let hysteresis = if chord > self.chord_index { -0.1 } else { 0.1 };
            let chord_index = ((chord + hysteresis + 0.5) as i32).clamp(0, 10);
            self.chord_index = chord_index as f32;
            center.fill(0.0);
            sides.fill(0.0);
            for i in 0..num_notes {
                let transpose = CHORDS[chord_index as usize][i];
                let s = &mut self.strings[i];
                s.set_frequency(frequency * semitones_to_ratio(transpose));
                s.set_brightness(patch.resonator_brightness);
                s.set_position(patch.resonator_position);
                s.set_damping(damping);
                if num_notes == 1 {
                    s.set_dispersion(patch.resonator_geometry);
                } else {
                    let b = patch.resonator_brightness;
                    s.set_dispersion(if b < 0.5 { 0.0 } else { (b - 0.5) * -0.4 });
                }
                s.process(raw, center, sides);
            }
            for i in 0..size {
                let left = center[i];
                let right = sides[i];
                center[i] = left - right;
                sides[i] = left + right;
            }
        }

        // Mix the raw strike signal into the outputs.
        for i in 0..size {
            center[i] += strike_bleed * self.strike_buffer[i];
        }
    }
}
