//! The easter egg: "Disastrous Peace", a polyphonic string synth with
//! formant filter, chorus, ensemble and reverb (`rings/dsp/
//! string_synth_part.h`, `string_synth_part.cc`, `string_synth_voice.h`,
//! `string_synth_oscillator.h`, `string_synth_envelope.h`).
//!
//! Copyright 2015 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::{integral_fractional, semitones_to_ratio, A3, BLOCK_SIZE, SAMPLE_RATE};
use crate::filter::{Approx, Ramp, Svf};
use crate::fx::{Chorus, Ensemble, Reverb};
use crate::limiter::Limiter;
use crate::note_filter::NoteFilter;
use crate::part::{Patch, PerformanceState, NUM_CHORDS};
use crate::EggFx;

pub const MAX_STRING_SYNTH_POLYPHONY: usize = 4;
pub const STRING_SYNTH_VOICES: usize = 12;
pub const MAX_CHORD_SIZE: usize = 8;
pub const NUM_HARMONICS: usize = 3;
pub const NUM_FORMANTS: usize = 3;

const FLAG_RISING_EDGE: u8 = 1;
const FLAG_FALLING_EDGE: u8 = 2;
const FLAG_GATE: u8 = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum EnvelopeShape {
    #[default]
    Linear,
    Quartic,
}

/// Multi-segment envelope, used as an AD here.
#[derive(Clone, Copy, Debug, Default)]
struct StringSynthEnvelope {
    level: [f32; 4],
    rate: [f32; 4],
    shape: [EnvelopeShape; 4],
    segment: usize,
    start_value: f32,
    value: f32,
    phase: f32,
    num_segments: usize,
    sustain_point: usize,
}

impl StringSynthEnvelope {
    fn init(&mut self) {
        self.set_ad(0.1, 0.001);
        self.segment = self.num_segments;
        self.phase = 0.0;
        self.start_value = 0.0;
        self.value = 0.0;
    }

    fn process(&mut self, flags: u8) -> f32 {
        if flags & FLAG_RISING_EDGE != 0 {
            self.start_value = if self.segment == self.num_segments { self.level[0] } else { self.value };
            self.segment = 0;
            self.phase = 0.0;
        } else if flags & FLAG_FALLING_EDGE != 0 && self.sustain_point != 0 {
            self.start_value = self.value;
            self.segment = self.sustain_point;
            self.phase = 0.0;
        } else if self.phase >= 1.0 && self.segment < self.num_segments {
            self.start_value = self.level[self.segment + 1];
            self.segment += 1;
            self.phase = 0.0;
        }

        let done = self.segment == self.num_segments;
        let sustained = self.sustain_point != 0 && self.segment == self.sustain_point && flags & FLAG_GATE != 0;

        let mut phase_increment = 0.0;
        if !sustained && !done {
            phase_increment = self.rate[self.segment];
        }
        let mut t = self.phase;
        if self.shape[self.segment] == EnvelopeShape::Quartic {
            t = 1.0 - t;
            t *= t;
            t *= t;
            t = 1.0 - t;
        }

        self.phase += phase_increment;
        self.value = self.start_value + (self.level[self.segment + 1] - self.start_value) * t;
        self.value
    }

    fn set_ad(&mut self, attack: f32, decay: f32) {
        self.num_segments = 2;
        self.sustain_point = 0;

        self.level[0] = 0.0;
        self.level[1] = 1.0;
        self.level[2] = 0.0;

        self.rate[0] = attack;
        self.rate[1] = decay;

        self.shape[0] = EnvelopeShape::Linear;
        self.shape[1] = EnvelopeShape::Quartic;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OscillatorShape {
    BrightSquare,
    DarkSquare,
}

/// Band-limited (polyBLEP) square and saw pair.
#[derive(Clone, Copy, Debug)]
struct StringSynthOscillator {
    high: bool,
    phase: f32,
    phase_increment: f32,
    next_sample: f32,
    next_sample_saw: f32,
    filter_state: f32,
    gain: f32,
    gain_saw: f32,
}

impl Default for StringSynthOscillator {
    fn default() -> Self {
        Self {
            high: false,
            phase: 0.0,
            phase_increment: 0.01,
            next_sample: 0.0,
            next_sample_saw: 0.0,
            filter_state: 0.0,
            gain: 0.0,
            gain_saw: 0.0,
        }
    }
}

#[inline]
fn this_blep_sample(t: f32) -> f32 {
    0.5 * t * t
}

#[inline]
fn next_blep_sample(t: f32) -> f32 {
    let t = 1.0 - t;
    -0.5 * t * t
}

impl StringSynthOscillator {
    fn render(
        &mut self,
        shape: OscillatorShape,
        interpolate_pitch: bool,
        target_increment: f32,
        mut target_gain: f32,
        target_gain_saw: f32,
        out: &mut [f32],
    ) {
        let size = out.len();
        // Cut harmonics above 12kHz, and low-pass harmonics above 8kHz to
        // clear highs.
        if target_increment >= 0.17 {
            target_gain *= 1.0 - (target_increment - 0.17) * 12.5;
            if target_increment >= 0.25 {
                return;
            }
        }
        let mut phase = self.phase;
        let mut phase_increment = Ramp::new(self.phase_increment, target_increment, size);
        let mut gain = Ramp::new(self.gain, target_gain, size);
        let mut gain_saw = Ramp::new(self.gain_saw, target_gain_saw, size);

        let mut next_sample = self.next_sample;
        let mut next_sample_saw = self.next_sample_saw;
        let mut filter_state = self.filter_state;
        let mut high = self.high;

        for o in out.iter_mut() {
            let mut this_sample = next_sample;
            let mut this_sample_saw = next_sample_saw;
            next_sample = 0.0;
            next_sample_saw = 0.0;

            let increment = if interpolate_pitch { phase_increment.next() } else { target_increment };
            phase += increment;

            const PW: f32 = 0.5;

            if !high && phase >= PW {
                let t = (phase - PW) / increment;
                this_sample += this_blep_sample(t);
                next_sample += next_blep_sample(t);
                high = true;
            }
            if phase >= 1.0 {
                phase -= 1.0;
                let t = phase / increment;
                let a = this_blep_sample(t);
                let b = next_blep_sample(t);
                this_sample -= a;
                next_sample -= b;
                this_sample_saw -= a;
                next_sample_saw -= b;
                high = false;
            }

            next_sample += if phase < PW { 0.0 } else { 1.0 };
            next_sample_saw += phase;

            let sample = match shape {
                OscillatorShape::DarkSquare => {
                    let integrator_coefficient = increment * 2.0;
                    this_sample = 4.0 * (this_sample - 0.5);
                    filter_state += integrator_coefficient * (this_sample - filter_state);
                    filter_state
                }
                OscillatorShape::BrightSquare => {
                    let integrator_coefficient = increment * 2.0;
                    this_sample = 2.0 * this_sample - 1.0;
                    filter_state += integrator_coefficient * (this_sample - filter_state);
                    (this_sample - filter_state) * 0.5
                }
            };
            this_sample_saw = 2.0 * this_sample_saw - 1.0;

            *o += sample * gain.next() + this_sample_saw * gain_saw.next();
        }
        self.high = high;
        self.phase = phase;
        self.next_sample = next_sample;
        self.next_sample_saw = next_sample_saw;
        self.filter_state = filter_state;
        if interpolate_pitch {
            self.phase_increment = phase_increment.value;
        }
        self.gain = gain.value;
        self.gain_saw = gain_saw.value;
    }
}

/// A note of the chord: an octave-stacked set of square/saw oscillators.
#[derive(Clone, Copy, Debug, Default)]
struct StringSynthVoice {
    oscillator: [StringSynthOscillator; NUM_HARMONICS],
}

impl StringSynthVoice {
    fn render(&mut self, mut frequency: f32, amplitudes: &[f32; NUM_HARMONICS * 2], summed_harmonics: usize, out: &mut [f32]) {
        self.oscillator[0].render(OscillatorShape::DarkSquare, true, frequency, amplitudes[0], amplitudes[1], out);
        for i in 1..summed_harmonics.min(NUM_HARMONICS) {
            frequency *= 2.0;
            self.oscillator[i].render(
                OscillatorShape::BrightSquare,
                false,
                frequency,
                amplitudes[2 * i],
                amplitudes[2 * i + 1],
                out,
            );
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct VoiceGroup {
    tonic: f32,
    envelope: StringSynthEnvelope,
    chord: i32,
    /// Stored but unused, as in the original.
    #[allow(dead_code)]
    structure: f32,
    /// Port addition: the velocity of the strum that started the group.
    velocity: f32,
}

const REGISTRATION_TABLE_SIZE: usize = 11;
const REGISTRATIONS: [[f32; NUM_HARMONICS * 2]; REGISTRATION_TABLE_SIZE] = [
    [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [1.0, 1.0, 0.0, 0.0, 0.0, 0.0],
    [1.0, 0.0, 1.0, 0.0, 0.0, 0.0],
    [1.0, 0.1, 0.0, 0.0, 1.0, 0.0],
    [1.0, 0.5, 1.0, 0.0, 1.0, 0.0],
    [1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
    [0.0, 1.0, 1.0, 1.0, 1.0, 0.0],
    [0.0, 0.5, 1.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
];

/// Chord table by Bryan Noll (`BRYAN_CHORDS`): more compact, leaving room
/// for a bass; more frequent note changes between adjacent chords; dropped
/// fifth.
const CHORDS: [[[f32; MAX_CHORD_SIZE]; NUM_CHORDS]; MAX_STRING_SYNTH_POLYPHONY] = [
    [
        [-12.0, -0.01, 0.0, 0.01, 0.02, 11.99, 12.0, 24.0], // OCT
        [-12.0, -5.01, -5.0, 0.0, 7.0, 12.0, 19.0, 24.0],   // 5
        [-12.0, -5.0, 0.0, 5.0, 7.0, 12.0, 17.0, 24.0],     // sus4
        [-12.0, -5.0, 0.0, 0.01, 3.0, 12.0, 19.0, 24.0],    // m
        [-12.0, -5.01, -5.0, 0.0, 3.0, 10.0, 19.0, 24.0],   // m7
        [-12.0, -5.0, 0.0, 3.0, 10.0, 14.0, 19.0, 24.0],    // m9
        [-12.0, -5.01, -5.0, 0.0, 3.0, 10.0, 17.0, 24.0],   // m11
        [-12.0, -5.0, 0.0, 2.0, 9.0, 16.0, 19.0, 24.0],     // 69
        [-12.0, -5.0, 0.0, 4.0, 11.0, 14.0, 19.0, 24.0],    // M9
        [-12.0, -5.0, 0.0, 4.0, 7.0, 11.0, 19.0, 24.0],     // M7
        [-12.0, -5.0, 0.0, 4.0, 7.0, 12.0, 19.0, 24.0],     // M
    ],
    [
        [-12.0, -0.01, 0.0, 0.01, 12.0, 12.01, 0.0, 0.0], // OCT
        [-12.0, -5.01, -5.0, 0.0, 7.0, 12.0, 0.0, 0.0],   // 5
        [-12.0, -5.0, 0.0, 5.0, 7.0, 12.0, 0.0, 0.0],     // sus4
        [-12.0, -5.0, 0.0, 0.01, 3.0, 12.0, 0.0, 0.0],    // m
        [-12.0, -5.01, -5.0, 0.0, 3.0, 10.0, 0.0, 0.0],   // m7
        [-12.0, -5.0, 0.0, 3.0, 10.0, 14.0, 0.0, 0.0],    // m9
        [-12.0, -5.0, 0.0, 3.0, 10.0, 17.0, 0.0, 0.0],    // m11
        [-12.0, -5.0, 0.0, 2.0, 9.0, 16.0, 0.0, 0.0],     // 69
        [-12.0, -5.0, 0.0, 4.0, 11.0, 14.0, 0.0, 0.0],    // M9
        [-12.0, -5.0, 0.0, 4.0, 7.0, 11.0, 0.0, 0.0],     // M7
        [-12.0, -5.0, 0.0, 4.0, 7.0, 12.0, 0.0, 0.0],     // M
    ],
    [
        [-12.0, 0.0, 0.01, 12.0, 0.0, 0.0, 0.0, 0.0],  // OCT
        [-12.0, 6.99, 7.0, 12.0, 0.0, 0.0, 0.0, 0.0],  // 5
        [-12.0, 5.0, 7.0, 12.0, 0.0, 0.0, 0.0, 0.0],   // sus4
        [-12.0, 3.0, 11.99, 12.0, 0.0, 0.0, 0.0, 0.0], // m
        [-12.0, 3.0, 9.99, 10.0, 0.0, 0.0, 0.0, 0.0],  // m7
        [-12.0, 3.0, 10.0, 14.0, 0.0, 0.0, 0.0, 0.0],  // m9
        [-12.0, 3.0, 10.0, 17.0, 0.0, 0.0, 0.0, 0.0],  // m11
        [-12.0, 2.0, 9.0, 16.0, 0.0, 0.0, 0.0, 0.0],   // 69
        [-12.0, 4.0, 11.0, 14.0, 0.0, 0.0, 0.0, 0.0],  // M9
        [-12.0, 4.0, 7.0, 11.0, 0.0, 0.0, 0.0, 0.0],   // M7
        [-12.0, 4.0, 7.0, 12.0, 0.0, 0.0, 0.0, 0.0],   // M
    ],
    [
        [0.0, 0.01, 12.0, 0.0, 0.0, 0.0, 0.0, 0.0], // OCT
        [0.0, 7.0, 12.0, 0.0, 0.0, 0.0, 0.0, 0.0],  // 5
        [5.0, 7.0, 12.0, 0.0, 0.0, 0.0, 0.0, 0.0],  // sus4
        [0.0, 3.0, 12.0, 0.0, 0.0, 0.0, 0.0, 0.0],  // m
        [0.0, 3.0, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0],  // m7
        [3.0, 10.0, 14.0, 0.0, 0.0, 0.0, 0.0, 0.0], // m9
        [3.0, 10.0, 17.0, 0.0, 0.0, 0.0, 0.0, 0.0], // m11
        [2.0, 9.0, 16.0, 0.0, 0.0, 0.0, 0.0, 0.0],  // 69
        [4.0, 11.0, 14.0, 0.0, 0.0, 0.0, 0.0, 0.0], // M9
        [4.0, 7.0, 11.0, 0.0, 0.0, 0.0, 0.0, 0.0],  // M7
        [4.0, 7.0, 12.0, 0.0, 0.0, 0.0, 0.0, 0.0],  // M
    ],
];

const FORMANT_TABLE_SIZE: usize = 5;
const FORMANTS: [[f32; NUM_FORMANTS]; FORMANT_TABLE_SIZE] = [
    [700.0, 1100.0, 2400.0],
    [500.0, 1300.0, 1700.0],
    [400.0, 2000.0, 2500.0],
    [600.0, 800.0, 2400.0],
    [300.0, 900.0, 2200.0],
];

fn fx_index(fx: EggFx) -> usize {
    match fx {
        EggFx::Formant => 0,
        EggFx::Chorus => 1,
        EggFx::Reverb => 2,
        EggFx::Formant2 => 3,
        EggFx::Ensemble => 4,
        EggFx::Reverb2 => 5,
    }
}

pub struct StringSynthPart {
    voice: [StringSynthVoice; STRING_SYNTH_VOICES],
    group: [VoiceGroup; MAX_STRING_SYNTH_POLYPHONY],
    formant_filter: [Svf; NUM_FORMANTS],
    ensemble: Ensemble,
    reverb: Reverb,
    chorus: Chorus,
    limiter: Limiter,
    active_group: usize,
    polyphony: usize,
    acquisition_delay: i32,
    fx_type: EggFx,
    note_filter: NoteFilter,
    filter_in_buffer: [f32; BLOCK_SIZE],
    filter_out_buffer: [f32; BLOCK_SIZE],
    clear_fx: bool,
}

impl Default for StringSynthPart {
    fn default() -> Self {
        Self::new()
    }
}

impl StringSynthPart {
    pub fn new() -> Self {
        let mut part = Self {
            voice: [StringSynthVoice::default(); STRING_SYNTH_VOICES],
            group: [VoiceGroup { velocity: 1.0, ..VoiceGroup::default() }; MAX_STRING_SYNTH_POLYPHONY],
            formant_filter: [Svf::default(); NUM_FORMANTS],
            ensemble: Ensemble::default(),
            reverb: Reverb::default(),
            chorus: Chorus::default(),
            limiter: Limiter::default(),
            active_group: 0,
            polyphony: 1,
            acquisition_delay: 0,
            fx_type: EggFx::Ensemble,
            note_filter: NoteFilter::default(),
            filter_in_buffer: [0.0; BLOCK_SIZE],
            filter_out_buffer: [0.0; BLOCK_SIZE],
            clear_fx: false,
        };
        part.reset();
        part
    }

    /// Back to the power-on state (keeping the polyphony and effect),
    /// without allocating.
    pub fn reset(&mut self) {
        self.voice = [StringSynthVoice::default(); STRING_SYNTH_VOICES];
        for g in self.group.iter_mut() {
            *g = VoiceGroup { velocity: 1.0, ..VoiceGroup::default() };
            g.envelope.init();
        }
        self.formant_filter = [Svf::default(); NUM_FORMANTS];
        self.reverb.reset();
        self.chorus.clear();
        self.ensemble.clear();
        self.limiter = Limiter::default();
        self.active_group = 0;
        self.acquisition_delay = 0;
        self.note_filter.init(
            SAMPLE_RATE / BLOCK_SIZE as f32,
            0.001, // Lag time with a sharp edge on the V/Oct input or trigger.
            0.005, // Lag time after the trigger has been received.
            0.050, // Time to transition from reactive to filtered.
            0.004, // Prevent a sharp edge to partly leak on the previous voice.
        );
        self.filter_in_buffer = [0.0; BLOCK_SIZE];
        self.filter_out_buffer = [0.0; BLOCK_SIZE];
        self.clear_fx = false;
    }

    pub fn polyphony(&self) -> usize {
        self.polyphony
    }

    pub fn set_polyphony(&mut self, polyphony: usize) {
        let old_polyphony = self.polyphony;
        self.polyphony = polyphony.clamp(1, MAX_STRING_SYNTH_POLYPHONY);
        for i in old_polyphony..self.polyphony {
            self.group[i].tonic = self.group[0].tonic + i as f32 * 0.01;
        }
        if self.active_group >= self.polyphony {
            self.active_group = 0;
        }
    }

    pub fn fx(&self) -> EggFx {
        self.fx_type
    }

    pub fn set_fx(&mut self, fx: EggFx) {
        if fx_index(fx) % 3 != fx_index(self.fx_type) % 3 {
            self.clear_fx = true;
        } else if fx != self.fx_type && matches!(fx, EggFx::Chorus | EggFx::Ensemble) {
            // On the module the chorus and the ensemble share their memory;
            // here each has its own, so the one switched to starts empty
            // instead of replaying an old tail.
            self.chorus.clear();
            self.ensemble.clear();
        }
        self.fx_type = fx;
    }

    fn process_envelopes(&mut self, shape: f32, flags: &[u8; MAX_STRING_SYNTH_POLYPHONY], values: &mut [f32; MAX_STRING_SYNTH_POLYPHONY]) {
        let decay = shape;
        let attack = if shape < 0.5 { 0.0 } else { (shape - 0.5) * 2.0 };

        // Convert the arbitrary values to actual units.
        let period = SAMPLE_RATE / BLOCK_SIZE as f32;
        let attack_time = semitones_to_ratio(attack * 96.0) * 0.005 * period;
        let decay_time = semitones_to_ratio(decay * 84.0) * 0.180 * period;
        let attack_rate = 1.0 / attack_time;
        let decay_rate = 1.0 / decay_time;

        for i in 0..self.polyphony {
            let mut drone = if shape < 0.98 { 0.0 } else { (shape - 0.98) * 55.0 };
            if drone >= 1.0 {
                drone = 1.0;
            }
            self.group[i].envelope.set_ad(attack_rate, decay_rate);
            let value = self.group[i].envelope.process(flags[i]);
            values[i] = value + (1.0 - value) * drone;
        }
    }

    fn compute_registration(gain: f32, registration: f32, amplitudes: &mut [f32; NUM_HARMONICS * 2]) {
        let registration = registration.clamp(0.0, 1.0) * (REGISTRATION_TABLE_SIZE as f32 - 1.001);
        let (integral, fractional) = integral_fractional(registration);
        let integral = (integral.max(0) as usize).min(REGISTRATION_TABLE_SIZE - 2);
        let mut total = 0.0;
        for i in 0..NUM_HARMONICS * 2 {
            let a = REGISTRATIONS[integral][i];
            let b = REGISTRATIONS[integral + 1][i];
            amplitudes[i] = a + (b - a) * fractional;
            total += amplitudes[i];
        }
        for a in amplitudes.iter_mut() {
            *a = gain * *a / total;
        }
    }

    fn process_formant_filter(&mut self, vowel: f32, shift: f32, resonance: f32, out: &mut [f32], aux: &mut [f32]) {
        let size = out.len();
        for i in 0..size {
            self.filter_in_buffer[i] = out[i] + aux[i];
        }
        out.fill(0.0);
        aux.fill(0.0);

        let vowel = vowel.clamp(0.0, 1.0) * (FORMANT_TABLE_SIZE as f32 - 1.001);
        let (vowel_integral, vowel_fractional) = integral_fractional(vowel);
        let vowel_integral = (vowel_integral.max(0) as usize).min(FORMANT_TABLE_SIZE - 2);

        for i in 0..NUM_FORMANTS {
            let a = FORMANTS[vowel_integral][i];
            let b = FORMANTS[vowel_integral + 1][i];
            let mut f = a + (b - a) * vowel_fractional;
            f *= shift;
            self.formant_filter[i].set_f_q(f / SAMPLE_RATE, resonance, Approx::Dirty);
            self.formant_filter[i].process_bp_buffer(&self.filter_in_buffer[..size], &mut self.filter_out_buffer[..size]);
            let pan = i as f32 * 0.3 + 0.2;
            for j in 0..size {
                out[j] += self.filter_out_buffer[j] * pan * 0.5;
                aux[j] += self.filter_out_buffer[j] * (1.0 - pan) * 0.5;
            }
        }
    }

    /// Renders one block (at most [`BLOCK_SIZE`] samples): odd to `out`,
    /// even to `aux`. The input is mixed in dry.
    pub fn process(&mut self, ps: &PerformanceState, patch: &Patch, input: &[f32], out: &mut [f32], aux: &mut [f32]) {
        let size = input.len().min(out.len()).min(aux.len()).min(BLOCK_SIZE);
        let (input, out, aux) = (&input[..size], &mut out[..size], &mut aux[..size]);

        // Assign note to a voice.
        let mut envelope_flags = [0u8; MAX_STRING_SYNTH_POLYPHONY];
        self.note_filter.process(ps.note, ps.strum);
        if ps.strum {
            self.group[self.active_group].tonic = self.note_filter.stable_note();
            self.note_filter.settle();
            envelope_flags[self.active_group] = FLAG_FALLING_EDGE;
            self.active_group = (self.active_group + 1) % self.polyphony;
            envelope_flags[self.active_group] = FLAG_RISING_EDGE;
            self.group[self.active_group].velocity = ps.velocity;
            self.acquisition_delay = 3;
        }
        if self.acquisition_delay > 0 {
            self.acquisition_delay -= 1;
        } else {
            let g = &mut self.group[self.active_group];
            g.tonic = self.note_filter.note();
            g.chord = ps.chord;
            g.structure = patch.structure;
            envelope_flags[self.active_group] |= FLAG_GATE;
        }

        // Process envelopes.
        let mut envelope_values = [0.0f32; MAX_STRING_SYNTH_POLYPHONY];
        self.process_envelopes(patch.damping, &envelope_flags, &mut envelope_values);

        aux.copy_from_slice(input);
        out.copy_from_slice(input);
        let chord_size = (STRING_SYNTH_VOICES / self.polyphony).min(MAX_CHORD_SIZE);
        for group in 0..self.polyphony {
            let mut harmonics = [0.0f32; NUM_HARMONICS * 2];
            Self::compute_registration(
                envelope_values[group] * 0.25 * self.group[group].velocity,
                patch.brightness,
                &mut harmonics,
            );

            let chord = (self.group[group].chord.clamp(0, NUM_CHORDS as i32 - 1)) as usize;
            for chord_note in 0..chord_size {
                let n = CHORDS[self.polyphony - 1][chord][chord_note];
                let amplitude = if (0.0..=17.0).contains(&n) { 1.0 } else { 0.7 };

                let mut note = 0.0;
                note += self.group[group].tonic;
                note += ps.tonic;
                note += ps.fm;
                note += n;

                let mut amplitudes = [0.0f32; NUM_HARMONICS * 2];
                for i in 0..NUM_HARMONICS * 2 {
                    amplitudes[i] = amplitude * harmonics[i];
                }

                // Fold truncated harmonics.
                let num_harmonics =
                    if self.polyphony >= 2 && chord_note < 2 { NUM_HARMONICS - 1 } else { NUM_HARMONICS };
                for i in num_harmonics..NUM_HARMONICS {
                    amplitudes[2 * (num_harmonics - 1)] += amplitudes[2 * i];
                    amplitudes[2 * (num_harmonics - 1) + 1] += amplitudes[2 * i + 1];
                }

                let frequency = semitones_to_ratio(note - 69.0) * A3;
                let destination = if (group + chord_note) & 1 != 0 { &mut *out } else { &mut *aux };
                self.voice[group * chord_size + chord_note].render(frequency, &amplitudes, num_harmonics, destination);
            }
        }

        if self.clear_fx {
            self.reverb.clear();
            self.chorus.clear();
            self.ensemble.clear();
            self.clear_fx = false;
        }

        match self.fx_type {
            EggFx::Formant | EggFx::Formant2 => {
                let formant = self.fx_type == EggFx::Formant;
                self.process_formant_filter(
                    patch.position,
                    if formant { 1.0 } else { 1.1 },
                    if formant { 25.0 } else { 10.0 },
                    out,
                    aux,
                );
            }
            EggFx::Chorus => {
                self.chorus.set_amount(patch.position);
                self.chorus.set_depth(0.15 + 0.5 * patch.position);
                self.chorus.process(out, aux);
            }
            EggFx::Ensemble => {
                self.ensemble.set_amount(patch.position * (2.0 - patch.position));
                self.ensemble.set_depth(0.2 + 0.8 * patch.position * patch.position);
                self.ensemble.process(out, aux);
            }
            EggFx::Reverb | EggFx::Reverb2 => {
                let first = self.fx_type == EggFx::Reverb;
                self.reverb.set_amount(patch.position * 0.5);
                self.reverb.set_diffusion(0.625);
                self.reverb.set_time(if first { 0.5 + 0.49 * patch.position } else { 0.3 + 0.6 * patch.position });
                self.reverb.set_input_gain(0.2);
                self.reverb.set_lp(if first { 0.3 } else { 0.6 });
                self.reverb.process(out, aux);
            }
        }

        // Prevent main signal cancellation when EVEN gets summed with ODD
        // through normalization.
        for a in aux.iter_mut() {
            *a = -*a;
        }
        self.limiter.process(out, aux, 1.0);
    }
}
