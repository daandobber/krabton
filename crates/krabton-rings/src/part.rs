//! The group of voices of Rings (`rings/dsp/part.h`, `part.cc`, with
//! `patch.h` and `performance_state.h`): the modal, string and FM
//! resonators, their exciters, the voice allocation, the reverb of the
//! String and Reverb model and the output limiter.
//!
//! Copyright 2015 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::{semitones_to_ratio, A3, BLOCK_SIZE, SAMPLE_RATE};
use crate::filter::{Approx, CosineOscillator, DcBlocker, Svf};
use crate::fm_voice::FmVoice;
use crate::fx::Reverb;
use crate::limiter::Limiter;
use crate::note_filter::NoteFilter;
use crate::plucker::Plucker;
use crate::resonator::Resonator;
use crate::string::KsString;
use crate::Model;

pub const MAX_POLYPHONY: usize = 4;
pub const NUM_STRINGS: usize = MAX_POLYPHONY * 2;
pub const NUM_CHORDS: usize = 11;

/// The four knobs (after CV), 0 to 1 (structure to 0.9995).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Patch {
    pub structure: f32,
    pub brightness: f32,
    pub damping: f32,
    pub position: f32,
}

/// What is played, once per block.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PerformanceState {
    /// A strum (trigger) happens in this block.
    pub strum: bool,
    /// Nothing is patched in the audio input.
    pub internal_exciter: bool,
    /// Transposition (the FREQUENCY knob), semitones.
    pub tonic: f32,
    /// V/Oct, semitones above the tonic.
    pub note: f32,
    /// FM input, semitones.
    pub fm: f32,
    /// 0 to [`NUM_CHORDS`] - 1.
    pub chord: i32,
    /// Port addition: the strength of the internal exciter for this strum
    /// (the module always strikes at 1).
    pub velocity: f32,
}

/// Chord table by Bryan Noll (`BRYAN_CHORDS` is defined in the firmware),
/// by polyphony.
const CHORDS: [[[f32; 8]; NUM_CHORDS]; MAX_POLYPHONY] = [
    [
        [-12.0, -0.01, 0.0, 0.01, 0.02, 11.98, 11.99, 12.0], // OCT
        [-12.0, -5.0, 0.0, 6.99, 7.0, 11.99, 12.0, 19.0],    // 5
        [-12.0, -5.0, 0.0, 5.0, 7.0, 11.99, 12.0, 17.0],     // sus4
        [-12.0, -5.0, 0.0, 3.0, 7.0, 3.01, 12.0, 19.0],      // m
        [-12.0, -5.0, 0.0, 3.0, 7.0, 3.01, 10.0, 19.0],      // m7
        [-12.0, -5.0, 0.0, 3.0, 14.0, 3.01, 10.0, 19.0],     // m9
        [-12.0, -5.0, 0.0, 3.0, 7.0, 3.01, 10.0, 17.0],      // m11
        [-12.0, -5.0, 0.0, 2.0, 7.0, 9.0, 16.0, 19.0],       // 69
        [-12.0, -5.0, 0.0, 4.0, 7.0, 11.0, 14.0, 19.0],      // M9
        [-12.0, -5.0, 0.0, 4.0, 7.0, 11.0, 10.99, 19.0],     // M7
        [-12.0, -5.0, 0.0, 4.0, 7.0, 11.99, 12.0, 19.0],     // M
    ],
    [
        [-12.0, 0.0, 0.01, 12.0, 0.0, 0.0, 0.0, 0.0],  // OCT
        [-12.0, 6.99, 7.0, 12.0, 0.0, 0.0, 0.0, 0.0],  // 5
        [-12.0, 5.0, 7.0, 12.0, 0.0, 0.0, 0.0, 0.0],   // sus4
        [-12.0, 3.0, 11.99, 12.0, 0.0, 0.0, 0.0, 0.0], // m
        [-12.0, 3.0, 10.0, 12.0, 0.0, 0.0, 0.0, 0.0],  // m7
        [-12.0, 3.0, 10.0, 14.0, 0.0, 0.0, 0.0, 0.0],  // m9
        [-12.0, 3.0, 10.0, 17.0, 0.0, 0.0, 0.0, 0.0],  // m11
        [-12.0, 2.0, 9.0, 16.0, 0.0, 0.0, 0.0, 0.0],   // 69
        [-12.0, 4.0, 11.0, 14.0, 0.0, 0.0, 0.0, 0.0],  // M9
        [-12.0, 4.0, 7.0, 11.0, 0.0, 0.0, 0.0, 0.0],   // M7
        [-12.0, 4.0, 7.0, 12.0, 0.0, 0.0, 0.0, 0.0],   // M
    ],
    [
        [0.0, -12.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 9.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 11.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 12.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [-12.0, 12.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ],
    [
        [0.0, -12.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 9.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 11.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 12.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [-12.0, 12.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ],
];

/// Voice order for strums with three voices.
const PING_PATTERN: [usize; 8] = [1, 0, 2, 1, 0, 2, 1, 0];

/// Output gain per model, before the limiter.
const MODEL_GAINS: [f32; 6] = [
    1.4, // Modal
    1.0, // Sympathetic string
    1.4, // String
    0.7, // FM voice
    1.0, // Sympathetic string quantized
    1.4, // String and reverb
];

fn model_index(model: Model) -> usize {
    match model {
        Model::Modal => 0,
        Model::SympatheticString => 1,
        Model::InharmonicString => 2,
        Model::FmVoice => 3,
        Model::SympatheticStringQuantized => 4,
        Model::StringAndReverb => 5,
    }
}

#[inline]
fn squash(mut x: f32) -> f32 {
    if x < 0.5 {
        x *= 2.0;
        x *= x;
        x *= x;
        x *= x;
        x *= x;
        x *= 0.5;
    } else {
        x = 2.0 - 2.0 * x;
        x *= x;
        x *= x;
        x *= x;
        x *= x;
        x = 1.0 - 0.5 * x;
    }
    x
}

pub struct Part {
    dirty: bool,
    model: Model,
    active_voice: usize,
    step_counter: usize,
    polyphony: usize,

    resonator: [Resonator; MAX_POLYPHONY],
    string: Vec<KsString>,
    lfo: [CosineOscillator; NUM_STRINGS],
    fm_voice: [FmVoice; MAX_POLYPHONY],

    excitation_filter: [Svf; MAX_POLYPHONY],
    dc_blocker: [DcBlocker; MAX_POLYPHONY],
    plucker: Vec<Plucker>,

    note: [f32; MAX_POLYPHONY],
    note_filter: NoteFilter,

    resonator_input: [f32; BLOCK_SIZE],
    sympathetic_resonator_input: [f32; BLOCK_SIZE],
    noise_burst_buffer: [f32; BLOCK_SIZE],
    out_buffer: [f32; BLOCK_SIZE],
    aux_buffer: [f32; BLOCK_SIZE],

    reverb: Reverb,
    limiter: Limiter,
}

impl Default for Part {
    fn default() -> Self {
        Self::new()
    }
}

impl Part {
    pub fn new() -> Self {
        let mut part = Self {
            dirty: true,
            model: Model::Modal,
            active_voice: 0,
            step_counter: 0,
            polyphony: 1,
            resonator: std::array::from_fn(|_| Resonator::default()),
            string: (0..NUM_STRINGS).map(|i| KsString::new(0x5eed_0000 + i as u32 * 7919)).collect(),
            lfo: [CosineOscillator::default(); NUM_STRINGS],
            fm_voice: [FmVoice::default(); MAX_POLYPHONY],
            excitation_filter: [Svf::default(); MAX_POLYPHONY],
            dc_blocker: [DcBlocker::new(1.0 - 10.0 / SAMPLE_RATE); MAX_POLYPHONY],
            plucker: (0..MAX_POLYPHONY).map(|i| Plucker::new(0x0b0e_0000 + i as u32 * 104729)).collect(),
            note: [0.0; MAX_POLYPHONY],
            note_filter: NoteFilter::default(),
            resonator_input: [0.0; BLOCK_SIZE],
            sympathetic_resonator_input: [0.0; BLOCK_SIZE],
            noise_burst_buffer: [0.0; BLOCK_SIZE],
            out_buffer: [0.0; BLOCK_SIZE],
            aux_buffer: [0.0; BLOCK_SIZE],
            reverb: Reverb::default(),
            limiter: Limiter::default(),
        };
        part.reset();
        part
    }

    /// Back to the power-on state (keeping the model and polyphony),
    /// without allocating.
    pub fn reset(&mut self) {
        for r in self.resonator.iter_mut() {
            r.init();
        }
        for s in self.string.iter_mut() {
            s.init(false);
        }
        for v in self.fm_voice.iter_mut() {
            v.init();
        }
        for p in self.plucker.iter_mut() {
            p.init();
        }
        self.excitation_filter = [Svf::default(); MAX_POLYPHONY];
        self.dc_blocker = [DcBlocker::new(1.0 - 10.0 / SAMPLE_RATE); MAX_POLYPHONY];
        self.lfo = [CosineOscillator::default(); NUM_STRINGS];
        self.note = [0.0; MAX_POLYPHONY];
        self.note_filter.init(
            SAMPLE_RATE / BLOCK_SIZE as f32,
            0.001, // Lag time with a sharp edge on the V/Oct input or trigger.
            0.010, // Lag time after the trigger has been received.
            0.050, // Time to transition from reactive to filtered.
            0.004, // Prevent a sharp edge to partly leak on the previous voice.
        );
        self.resonator_input = [0.0; BLOCK_SIZE];
        self.sympathetic_resonator_input = [0.0; BLOCK_SIZE];
        self.noise_burst_buffer = [0.0; BLOCK_SIZE];
        self.out_buffer = [0.0; BLOCK_SIZE];
        self.aux_buffer = [0.0; BLOCK_SIZE];
        self.reverb.reset();
        self.limiter = Limiter::default();
        self.active_voice = 0;
        self.step_counter = 0;
        self.dirty = true;
    }

    pub fn polyphony(&self) -> usize {
        self.polyphony
    }

    pub fn set_polyphony(&mut self, polyphony: usize) {
        let old_polyphony = self.polyphony;
        self.polyphony = polyphony.clamp(1, MAX_POLYPHONY);
        for i in old_polyphony..self.polyphony {
            self.note[i] = self.note[0] + i as f32 * 0.05;
        }
        self.dirty = true;
    }

    pub fn model(&self) -> Model {
        self.model
    }

    pub fn set_model(&mut self, model: Model) {
        if model != self.model {
            self.model = model;
            self.dirty = true;
        }
    }

    fn configure_resonators(&mut self) {
        if !self.dirty {
            return;
        }
        match self.model {
            Model::Modal => {
                let resolution = 64 / self.polyphony - 4;
                for r in self.resonator[..self.polyphony].iter_mut() {
                    r.init();
                    r.set_resolution(resolution);
                }
            }
            Model::SympatheticString | Model::InharmonicString | Model::SympatheticStringQuantized | Model::StringAndReverb => {
                // The C++ initializer has 7 values for 8 strings.
                const LFO_FREQUENCIES: [f32; NUM_STRINGS] = [0.5, 0.4, 0.35, 0.23, 0.211, 0.2, 0.171, 0.0];
                let has_dispersion = matches!(self.model, Model::InharmonicString | Model::StringAndReverb);
                for i in 0..NUM_STRINGS {
                    self.string[i].init(has_dispersion);
                    let f_lfo = BLOCK_SIZE as f32 / SAMPLE_RATE * LFO_FREQUENCIES[i];
                    self.lfo[i].init_approximate(f_lfo);
                }
                for p in self.plucker[..self.polyphony].iter_mut() {
                    p.init();
                }
            }
            Model::FmVoice => {
                for v in self.fm_voice[..self.polyphony].iter_mut() {
                    v.init();
                }
            }
        }
        if self.active_voice >= self.polyphony {
            self.active_voice = 0;
        }
        self.dirty = false;
    }

    fn compute_sympathetic_strings_notes(
        &self,
        tonic: f32,
        note: f32,
        mut parameter: f32,
        destination: &mut [f32; NUM_STRINGS],
        num_strings: usize,
    ) {
        let notes = [
            tonic,
            note - 12.0,
            note - 7.01955,
            note,
            note + 7.01955,
            note + 12.0,
            note + 19.01955,
            note + 24.0,
            note + 24.0,
        ];
        const DETUNINGS: [f32; 4] = [0.013, 0.011, 0.007, 0.017];

        if parameter >= 2.0 {
            // Quantized chords.
            let chord_index = ((parameter - 2.0) as usize).min(NUM_CHORDS - 1);
            let chord = &CHORDS[self.polyphony - 1][chord_index];
            for i in 0..num_strings {
                destination[i] = chord[i] + note;
            }
            return;
        }

        let num_detuned_strings = (num_strings - 1) >> 1;
        let first_detuned_string = num_strings - num_detuned_strings;

        for i in 0..first_detuned_string {
            let mut note = 3.0;
            if i != 0 {
                note = parameter * 7.0;
                parameter += (1.0 - parameter) * 0.2;
            }
            let note = note.clamp(0.0, 7.999);
            let integral = note as usize;
            let fractional = squash(note - integral as f32);
            let a = notes[integral];
            let b = notes[integral + 1];
            destination[i] = a + (b - a) * fractional;
            if i + first_detuned_string < num_strings {
                destination[i + first_detuned_string] = destination[i] + DETUNINGS[i & 3];
            }
        }
    }

    fn render_modal_voice(&mut self, voice: usize, ps: &PerformanceState, patch: &Patch, frequency: f32, filter_cutoff: f32, size: usize) {
        // Internal exciter is a pulse, pre-filter.
        if ps.internal_exciter && voice == self.active_voice && ps.strum {
            self.resonator_input[0] +=
                ps.velocity * 0.25 * semitones_to_ratio(filter_cutoff * filter_cutoff * 24.0) / filter_cutoff;
        }

        // Process through filter.
        self.excitation_filter[voice].process_lp_in_place(&mut self.resonator_input[..size]);

        let r = &mut self.resonator[voice];
        r.set_frequency(frequency);
        r.set_structure(patch.structure);
        r.set_brightness(patch.brightness * patch.brightness);
        r.set_position(patch.position);
        r.set_damping(patch.damping);
        r.process(&self.resonator_input[..size], &mut self.out_buffer[..size], &mut self.aux_buffer[..size]);
    }

    fn render_fm_voice(&mut self, voice: usize, ps: &PerformanceState, patch: &Patch, frequency: f32, size: usize) {
        let v = &mut self.fm_voice[voice];
        if ps.internal_exciter && voice == self.active_voice && ps.strum {
            v.trigger_internal_envelope(ps.velocity);
        }
        v.set_frequency(frequency);
        v.set_ratio(patch.structure);
        v.set_brightness(patch.brightness);
        v.set_feedback_amount(patch.position);
        v.set_position(0.0);
        v.set_damping(patch.damping);
        v.process(&self.resonator_input[..size], &mut self.out_buffer[..size], &mut self.aux_buffer[..size]);
    }

    fn render_string_voice(&mut self, voice: usize, ps: &PerformanceState, patch: &Patch, frequency: f32, filter_cutoff: f32, size: usize) {
        // Compute number of strings and frequency.
        let mut num_strings = 1;
        let mut frequencies = [0.0f32; NUM_STRINGS];

        if matches!(self.model, Model::SympatheticString | Model::SympatheticStringQuantized) {
            num_strings = 2 * MAX_POLYPHONY / self.polyphony;
            let parameter =
                if self.model == Model::SympatheticString { patch.structure } else { 2.0 + ps.chord as f32 };
            self.compute_sympathetic_strings_notes(
                ps.tonic + ps.fm,
                ps.tonic + self.note[voice] + ps.fm,
                parameter,
                &mut frequencies,
                num_strings,
            );
            for f in frequencies[..num_strings].iter_mut() {
                *f = semitones_to_ratio(*f - 69.0) * A3;
            }
        } else {
            frequencies[0] = frequency;
        }

        if voice == self.active_voice {
            let gain = 1.0 / (num_strings as f32 * 2.0).sqrt();
            for x in self.resonator_input[..size].iter_mut() {
                *x *= gain;
            }
        }

        // Process external input.
        self.excitation_filter[voice].process_lp_in_place(&mut self.resonator_input[..size]);

        // Add noise burst.
        if ps.internal_exciter {
            if voice == self.active_voice && ps.strum {
                self.plucker[voice].trigger(frequency, filter_cutoff * 8.0, patch.position, ps.velocity);
            }
            self.plucker[voice].process(&mut self.noise_burst_buffer[..size]);
            for (x, n) in self.resonator_input[..size].iter_mut().zip(&self.noise_burst_buffer[..size]) {
                *x += n;
            }
        }
        self.dc_blocker[voice].process(&mut self.resonator_input[..size]);

        self.out_buffer[..size].fill(0.0);
        self.aux_buffer[..size].fill(0.0);

        let structure = patch.structure;
        let dispersion = if structure < 0.24 {
            (structure - 0.24) * 4.166
        } else if structure > 0.26 {
            (structure - 0.26) * 1.35135
        } else {
            0.0
        };

        for string in 0..num_strings {
            let i = voice + string * self.polyphony;
            let lfo_value = self.lfo[i].next();

            let mut brightness = patch.brightness;
            let mut damping = patch.damping;
            let mut position = patch.position;
            let mut glide = 1.0;
            let string_index = string as f32 / num_strings as f32;
            let mut sympathetic = false;

            if self.model == Model::StringAndReverb {
                damping *= 2.0 - damping;
            }

            // When the internal exciter is used, string 0 is the main
            // source, the other strings are vibrating by sympathetic
            // resonance. When the internal exciter is not used, all strings
            // are vibrating by sympathetic resonance.
            if string > 0 && ps.internal_exciter {
                brightness *= 2.0 - brightness;
                brightness *= 2.0 - brightness;
                damping = 0.7 + patch.damping * 0.27;
                let amount = (0.5 - (0.5 - patch.position).abs()) * 0.9;
                position = patch.position + lfo_value * amount;
                glide = semitones_to_ratio((brightness - 1.0) * 36.0);
                sympathetic = true;
            }

            let s = &mut self.string[i];
            s.set_dispersion(dispersion);
            s.set_frequency(frequencies[string], glide);
            s.set_brightness(brightness);
            s.set_position(position);
            s.set_damping(damping + string_index * (0.95 - damping));
            let input = if sympathetic { &self.sympathetic_resonator_input[..size] } else { &self.resonator_input[..size] };
            s.process(input, &mut self.out_buffer[..size], &mut self.aux_buffer[..size]);

            if string == 0 {
                // Was 0.1f, Ben Wilson -> 0.2f
                let gain = 0.2 / num_strings as f32;
                for j in 0..size {
                    let sum = self.out_buffer[j] - self.aux_buffer[j];
                    self.sympathetic_resonator_input[j] = gain * sum;
                }
            }
        }
    }

    /// Renders one block (at most [`BLOCK_SIZE`] samples): odd to `out`,
    /// even to `aux`.
    pub fn process(&mut self, ps: &PerformanceState, patch: &Patch, input: &[f32], out: &mut [f32], aux: &mut [f32]) {
        let size = input.len().min(out.len()).min(aux.len()).min(BLOCK_SIZE);
        let (input, out, aux) = (&input[..size], &mut out[..size], &mut aux[..size]);

        self.configure_resonators();

        self.note_filter.process(ps.note, ps.strum);

        if ps.strum {
            self.note[self.active_voice] = self.note_filter.stable_note();
            self.note_filter.settle();
            if self.polyphony > 1 && self.polyphony & 1 != 0 {
                self.active_voice = PING_PATTERN[self.step_counter % 8].min(self.polyphony - 1);
                self.step_counter = (self.step_counter + 1) % 8;
            } else {
                self.active_voice = (self.active_voice + 1) % self.polyphony;
            }
        }

        self.note[self.active_voice] = self.note_filter.note();

        out.fill(0.0);
        aux.fill(0.0);
        for voice in 0..self.polyphony {
            // Compute MIDI note value, frequency, and cutoff frequency for
            // excitation filter.
            let cutoff = patch.brightness * (2.0 - patch.brightness);
            let note = self.note[voice] + ps.tonic + ps.fm;
            let frequency = semitones_to_ratio(note - 69.0) * A3;
            let filter_cutoff_range = if ps.internal_exciter {
                frequency * semitones_to_ratio((cutoff - 0.5) * 96.0)
            } else {
                0.4 * semitones_to_ratio((cutoff - 1.0) * 108.0)
            };
            let filter_cutoff =
                (if voice == self.active_voice { filter_cutoff_range } else { 10.0 / SAMPLE_RATE }).min(0.499);
            let filter_q = if ps.internal_exciter { 1.5 } else { 0.8 };

            // Process input with excitation filter. Inactive voices receive
            // silence.
            self.excitation_filter[voice].set_f_q(filter_cutoff, filter_q, Approx::Dirty);
            if voice == self.active_voice {
                self.resonator_input[..size].copy_from_slice(input);
            } else {
                self.resonator_input[..size].fill(0.0);
            }

            match self.model {
                Model::Modal => self.render_modal_voice(voice, ps, patch, frequency, filter_cutoff, size),
                Model::FmVoice => self.render_fm_voice(voice, ps, patch, frequency, size),
                _ => self.render_string_voice(voice, ps, patch, frequency, filter_cutoff, size),
            }

            if self.polyphony == 1 {
                // Send the two sets of harmonics / pickups to individual
                // outputs.
                for i in 0..size {
                    out[i] += self.out_buffer[i];
                    aux[i] += self.aux_buffer[i];
                }
            } else {
                // Dispatch odd/even voices to individual outputs.
                let destination = if voice & 1 != 0 { &mut *aux } else { &mut *out };
                for i in 0..size {
                    destination[i] += self.out_buffer[i] - self.aux_buffer[i];
                }
            }
        }

        if self.model == Model::StringAndReverb {
            for i in 0..size {
                let l = out[i];
                let r = aux[i];
                out[i] = l * patch.position + (1.0 - patch.position) * r;
                aux[i] = r * patch.position + (1.0 - patch.position) * l;
            }
            self.reverb.set_amount(0.1 + patch.damping * 0.5);
            self.reverb.set_diffusion(0.625);
            self.reverb.set_time(0.35 + 0.63 * patch.damping);
            self.reverb.set_input_gain(0.2);
            self.reverb.set_lp(0.3 + patch.brightness * 0.6);
            self.reverb.process(out, aux);
            for a in aux.iter_mut() {
                *a = -*a;
            }
        }

        // Apply limiter to string output.
        self.limiter.process(out, aux, MODEL_GAINS[model_index(self.model)]);
    }
}
