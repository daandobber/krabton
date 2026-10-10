//! The FM voice of the bonus models (`rings/dsp/fm_voice.h`,
//! `fm_voice.cc`) and its three-band envelope follower
//! (`rings/dsp/follower.h`).
//!
//! Copyright 2015 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::{interpolate, one_pole, semitones_to_ratio, slew, slope, SAMPLE_RATE};
use crate::filter::{NaiveSvf, Ramp};
use crate::resources::{LUT_FM_FREQUENCY_QUANTIZER, LUT_SINE};

/// Splits the signal in three bands and follows their level and spectral
/// centroid.
#[derive(Clone, Copy, Debug, Default)]
pub struct Follower {
    low_mid_filter: NaiveSvf,
    mid_high_filter: NaiveSvf,
    attack: [f32; 3],
    decay: [f32; 3],
    detector: [f32; 3],
    centroid: f32,
}

impl Follower {
    pub fn init(&mut self, low: f32, low_mid: f32, mid_high: f32) {
        self.low_mid_filter = NaiveSvf::default();
        self.mid_high_filter = NaiveSvf::default();
        self.low_mid_filter.set_f_q(low_mid, 0.5);
        self.mid_high_filter.set_f_q(mid_high, 0.5);
        self.attack[0] = low_mid;
        self.decay[0] = (low_mid * low).sqrt();
        self.attack[1] = (low_mid * mid_high).sqrt();
        self.decay[1] = low_mid;
        self.attack[2] = (mid_high * 0.5).sqrt();
        self.decay[2] = (mid_high * low_mid).sqrt();
        self.detector = [0.0; 3];
        self.centroid = 0.0;
    }

    /// Returns (envelope, centroid).
    #[inline]
    pub fn process(&mut self, sample: f32) -> (f32, f32) {
        let mut bands = [0.0f32; 3];
        bands[2] = self.mid_high_filter.process_hp(sample);
        bands[1] = self.low_mid_filter.process_hp(self.mid_high_filter.lp());
        bands[0] = self.low_mid_filter.lp();

        let mut weighted = 0.0;
        let mut total = 0.0;
        let mut frequency = 0.0;
        for i in 0..3 {
            slope(&mut self.detector[i], bands[i].abs(), self.attack[i], self.decay[i]);
            weighted += self.detector[i] * frequency;
            total += self.detector[i];
            frequency += 0.5;
        }

        let error = weighted / (total + 0.001) - self.centroid;
        let coefficient = if error > 0.0 { 0.05 } else { 0.001 };
        self.centroid += error * coefficient;
        (total, self.centroid)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FmVoice {
    carrier_frequency: f32,
    ratio: f32,
    brightness: f32,
    damping: f32,
    /// Set but unused, as in the original.
    #[allow(dead_code)]
    position: f32,
    feedback_amount: f32,

    previous_carrier_frequency: f32,
    previous_modulator_frequency: f32,
    previous_brightness: f32,
    #[allow(dead_code)]
    previous_damping: f32,
    previous_feedback_amount: f32,

    amplitude_envelope: f32,
    brightness_envelope: f32,
    gain: f32,
    fm_amount: f32,
    carrier_phase: u32,
    modulator_phase: u32,
    previous_sample: f32,

    follower: Follower,
}

#[inline]
fn sine_fm(phase: u32, fm: f32) -> f32 {
    let phase = phase.wrapping_add((((fm + 4.0) * 536870912.0) as u32) << 3);
    let integral = (phase >> 20) as usize;
    let fractional = (phase << 12) as f32 / 4294967296.0;
    let a = LUT_SINE[integral];
    let b = LUT_SINE[integral + 1];
    a + (b - a) * fractional
}

impl FmVoice {
    pub fn init(&mut self) {
        self.carrier_frequency = 220.0 / SAMPLE_RATE;
        self.ratio = 0.5;
        self.brightness = 0.5;
        self.damping = 0.5;
        self.position = 0.5;
        self.feedback_amount = 0.0;

        self.previous_carrier_frequency = self.carrier_frequency;
        self.previous_modulator_frequency = self.carrier_frequency;
        self.previous_brightness = self.brightness;
        self.previous_damping = self.damping;
        self.previous_feedback_amount = self.feedback_amount;

        self.amplitude_envelope = 0.0;
        self.brightness_envelope = 0.0;

        self.carrier_phase = 0;
        self.modulator_phase = 0;
        self.gain = 0.0;
        self.fm_amount = 0.0;

        self.follower.init(8.0 / SAMPLE_RATE, 160.0 / SAMPLE_RATE, 1600.0 / SAMPLE_RATE);
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.carrier_frequency = frequency;
    }

    pub fn set_ratio(&mut self, ratio: f32) {
        self.ratio = ratio;
    }

    pub fn set_brightness(&mut self, brightness: f32) {
        self.brightness = brightness;
    }

    pub fn set_damping(&mut self, damping: f32) {
        self.damping = damping;
    }

    pub fn set_position(&mut self, position: f32) {
        self.position = position;
    }

    pub fn set_feedback_amount(&mut self, feedback_amount: f32) {
        self.feedback_amount = feedback_amount;
    }

    /// Starts the internal envelopes. The module always strikes at 1; the
    /// port strikes at the note velocity.
    pub fn trigger_internal_envelope(&mut self, level: f32) {
        self.amplitude_envelope = level;
        self.brightness_envelope = level;
    }

    pub fn process(&mut self, input: &[f32], out: &mut [f32], aux: &mut [f32]) {
        let size = input.len();
        // Interpolate between the "oscillator" behaviour and the "FMLPGed
        // thing" behaviour.
        let envelope_amount = if self.damping < 0.9 { 1.0 } else { (1.0 - self.damping) * 10.0 };
        let amplitude_rt60 = 0.1 * semitones_to_ratio(self.damping * 96.0) * SAMPLE_RATE;
        let amplitude_decay = 1.0 - 0.001f32.powf(1.0 / amplitude_rt60);

        let brightness_rt60 = 0.1 * semitones_to_ratio(self.damping * 84.0) * SAMPLE_RATE;
        let brightness_decay = 1.0 - 0.001f32.powf(1.0 / brightness_rt60);

        let ratio = interpolate(&LUT_FM_FREQUENCY_QUANTIZER, self.ratio, 128.0);
        let mut modulator_frequency = self.carrier_frequency * semitones_to_ratio(ratio);
        if modulator_frequency > 0.5 {
            modulator_frequency = 0.5;
        }

        let feedback = (self.feedback_amount - 0.5) * 2.0;

        let mut carrier_increment = Ramp::new(self.previous_carrier_frequency, self.carrier_frequency, size);
        let mut modulator_increment = Ramp::new(self.previous_modulator_frequency, modulator_frequency, size);
        let mut brightness = Ramp::new(self.previous_brightness, self.brightness, size);
        let mut feedback_amount = Ramp::new(self.previous_feedback_amount, feedback, size);

        let mut carrier_phase = self.carrier_phase;
        let mut modulator_phase = self.modulator_phase;
        let mut previous_sample = self.previous_sample;

        for n in 0..size {
            // Envelope follower and internal envelope.
            let (amplitude_envelope, mut brightness_envelope) = self.follower.process(input[n]);
            brightness_envelope *= 2.0 * amplitude_envelope * (2.0 - amplitude_envelope);

            slope(&mut self.amplitude_envelope, amplitude_envelope, 0.05, amplitude_decay);
            slope(&mut self.brightness_envelope, brightness_envelope, 0.01, brightness_decay);

            // Compute envelopes.
            let mut brightness_value = brightness.next();
            brightness_value *= brightness_value;
            let fm_amount_min = if brightness_value < 0.5 { 0.0 } else { brightness_value * 2.0 - 1.0 };
            let fm_amount_max = if brightness_value < 0.5 { 2.0 * brightness_value } else { 1.0 };
            let fm_envelope = 0.5 + envelope_amount * (self.brightness_envelope - 0.5);
            let fm_amount = (fm_amount_min + fm_amount_max * fm_envelope) * 2.0;
            slew(&mut self.fm_amount, fm_amount, 0.005 + fm_amount_max * 0.015);

            // FM synthesis in itself.
            let phase_feedback = if feedback < 0.0 { 0.5 * feedback * feedback } else { 0.0 };
            modulator_phase = modulator_phase.wrapping_add(
                (4294967296.0 * modulator_increment.next() * (1.0 + previous_sample * phase_feedback)) as u32,
            );
            carrier_phase = carrier_phase.wrapping_add((4294967296.0 * carrier_increment.next()) as u32);

            let feedback = feedback_amount.next();
            let modulator_fb = if feedback > 0.0 { 0.25 * feedback * feedback } else { 0.0 };
            let modulator = sine_fm(modulator_phase, modulator_fb * previous_sample);
            let carrier = sine_fm(carrier_phase, self.fm_amount * modulator);
            one_pole(&mut previous_sample, carrier, 0.1);

            // Compute amplitude envelope.
            let gain = 1.0 + envelope_amount * (self.amplitude_envelope - 1.0);
            one_pole(&mut self.gain, gain, 0.005 + 0.045 * self.fm_amount);

            out[n] = (carrier + 0.5 * modulator) * self.gain;
            aux[n] = 0.5 * modulator * self.gain;
        }
        self.previous_carrier_frequency = carrier_increment.value;
        self.previous_modulator_frequency = modulator_increment.value;
        self.previous_brightness = brightness.value;
        self.previous_feedback_amount = feedback_amount.value;
        self.carrier_phase = carrier_phase;
        self.modulator_phase = modulator_phase;
        self.previous_sample = previous_sample;
    }
}
