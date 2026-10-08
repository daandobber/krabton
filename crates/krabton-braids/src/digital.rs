//! The digital oscillator: every Braids shape from Triple Ring Mod on.
//!
//! Port of `braids/digital_oscillator.h/.cc` (Copyright 2012 Emilie Gillet,
//! MIT). This file has the state, setup and dispatch; the render functions
//! live in `digital_a.rs` (ring mod to struck drum) and `digital_b.rs`
//! (physical models, wavetables, noises, drums, digital modulation).
//!
//! The C++ class keeps its per-shape state and its delay lines in unions.
//! Here every member is a separate field; `init` resets the state exactly
//! where the C++ code clears the union.

#![allow(dead_code)] // enum values and sizes are kept as in the C++ code

use crate::excitation::Excitation;
use crate::resources::{LUT_FM_FREQUENCY_QUANTIZER, LUT_OSCILLATOR_DELAYS, LUT_OSCILLATOR_INCREMENTS};
use crate::svf::Svf;

pub(crate) const K_WG_BRIDGE_LENGTH: usize = 1024;
pub(crate) const K_WG_NECK_LENGTH: usize = 4096;
pub(crate) const K_WG_BORE_LENGTH: usize = 2048;
pub(crate) const K_WG_JET_LENGTH: usize = 1024;
pub(crate) const K_WG_FBORE_LENGTH: usize = 4096;
pub(crate) const K_COMB_DELAY_LENGTH: usize = 8192;
pub(crate) const K_NUM_FORMANTS: usize = 5;
pub(crate) const K_NUM_PLUCK_VOICES: usize = 3;
pub(crate) const K_NUM_OVERLAPPING_FOF: usize = 3;
pub(crate) const K_NUM_BELL_PARTIALS: usize = 11;
pub(crate) const K_NUM_DRUM_PARTIALS: usize = 6;
pub(crate) const K_NUM_ADDITIVE_HARMONICS: usize = 12;

pub(crate) const K_HIGHEST_NOTE: i32 = 140 * 128;
pub(crate) const K_PITCH_TABLE_START: i32 = 128 * 128;
pub(crate) const K_OCTAVE: i32 = 12 * 128;
pub(crate) const K_FIR4_COEFFICIENTS: [u32; 4] = [10530, 14751, 16384, 14751];
pub(crate) const K_FIR4_DC_OFFSET: u32 = 28208;

// `DigitalOscillatorShape` values (the C++ enum). Note that the dispatch
// order below follows `fn_table_`, which differs from the enum for the
// physical models and drums; code comparing against these constants uses
// the enum values exactly like the C++ code.
pub(crate) const OSC_SHAPE_TRIPLE_RING_MOD: usize = 0;
pub(crate) const OSC_SHAPE_SAW_SWARM: usize = 1;
pub(crate) const OSC_SHAPE_COMB_FILTER: usize = 2;
pub(crate) const OSC_SHAPE_TOY: usize = 3;
pub(crate) const OSC_SHAPE_DIGITAL_FILTER_LP: usize = 4;
pub(crate) const OSC_SHAPE_DIGITAL_FILTER_PK: usize = 5;
pub(crate) const OSC_SHAPE_DIGITAL_FILTER_BP: usize = 6;
pub(crate) const OSC_SHAPE_DIGITAL_FILTER_HP: usize = 7;
pub(crate) const OSC_SHAPE_VOSIM: usize = 8;
pub(crate) const OSC_SHAPE_VOWEL: usize = 9;
pub(crate) const OSC_SHAPE_VOWEL_FOF: usize = 10;
pub(crate) const OSC_SHAPE_HARMONICS: usize = 11;
pub(crate) const OSC_SHAPE_FM: usize = 12;
pub(crate) const OSC_SHAPE_FEEDBACK_FM: usize = 13;
pub(crate) const OSC_SHAPE_CHAOTIC_FEEDBACK_FM: usize = 14;
pub(crate) const OSC_SHAPE_STRUCK_BELL: usize = 15;
pub(crate) const OSC_SHAPE_STRUCK_DRUM: usize = 16;
pub(crate) const OSC_SHAPE_KICK: usize = 17;
pub(crate) const OSC_SHAPE_HAT: usize = 18;
pub(crate) const OSC_SHAPE_SNARE: usize = 19;
pub(crate) const OSC_SHAPE_PLUCKED: usize = 20;
pub(crate) const OSC_SHAPE_BOWED: usize = 21;
pub(crate) const OSC_SHAPE_BLOWN: usize = 22;
pub(crate) const OSC_SHAPE_FLUTED: usize = 23;
pub(crate) const OSC_SHAPE_WAVETABLES: usize = 24;
pub(crate) const OSC_SHAPE_WAVE_MAP: usize = 25;
pub(crate) const OSC_SHAPE_WAVE_LINE: usize = 26;
pub(crate) const OSC_SHAPE_WAVE_PARAPHONIC: usize = 27;
pub(crate) const OSC_SHAPE_FILTERED_NOISE: usize = 28;
pub(crate) const OSC_SHAPE_TWIN_PEAKS_NOISE: usize = 29;
pub(crate) const OSC_SHAPE_CLOCKED_NOISE: usize = 30;
pub(crate) const OSC_SHAPE_GRANULAR_CLOUD: usize = 31;
pub(crate) const OSC_SHAPE_PARTICLE_NOISE: usize = 32;
pub(crate) const OSC_SHAPE_DIGITAL_MODULATION: usize = 33;
pub(crate) const OSC_SHAPE_QUESTION_MARK_LAST: usize = 34;

#[derive(Clone, Copy, Default)]
pub(crate) struct ResoSquareState {
    pub modulator_phase_increment: u32,
    pub modulator_phase: u32,
    pub square_modulator_phase: u32,
    pub integrator: i32,
    pub polarity: bool,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct VowelSynthesizerState {
    pub formant_increment: [u32; 3],
    pub formant_phase: [u32; 3],
    pub formant_amplitude: [u32; 3],
    pub consonant_frames: u16,
    pub noise: u16,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct SawSwarmState {
    pub phase: [u32; 6],
    pub filter_state: [[i32; 2]; 2],
    pub dc_blocked: i32,
    pub lp: i32,
    pub bp: i32,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct HarmonicsState {
    pub amplitude: [i32; K_NUM_ADDITIVE_HARMONICS],
}

#[derive(Clone, Copy, Default)]
pub(crate) struct AdditiveState {
    pub partial_phase: [u32; K_NUM_BELL_PARTIALS],
    pub partial_phase_increment: [u32; K_NUM_BELL_PARTIALS],
    pub partial_amplitude: [i32; K_NUM_BELL_PARTIALS],
    pub target_partial_amplitude: [i32; K_NUM_BELL_PARTIALS],
    pub previous_sample: i16,
    pub current_partial: usize,
    pub lp_noise: [i32; 3],
}

#[derive(Clone, Copy, Default)]
pub(crate) struct PluckState {
    pub size: usize,
    pub write_ptr: usize,
    pub shift: usize,
    pub mask: usize,
    pub pluck_position: usize,
    pub initialization_ptr: usize,
    pub phase: u32,
    pub phase_increment: u32,
    pub max_phase_increment: u32,
    pub previous_sample: i16,
    pub polyphony_assigner: u8,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct FeedbackFmState {
    pub modulator_phase: u32,
    pub previous_sample: i16,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct ParticleNoiseState {
    pub amplitude: u16,
    pub filter_state: [[i32; 2]; 3],
    pub filter_scale: [i32; 3],
    pub filter_coefficient: [i32; 3],
}

#[derive(Clone, Copy, Default)]
pub(crate) struct PhysicalModellingState {
    pub delay_ptr: u16,
    pub excitation_ptr: u16,
    pub lp_state: i32,
    pub filter_state: [i32; 2],
    pub previous_sample: i16,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Grain {
    pub phase: u32,
    pub phase_increment: u32,
    pub envelope_phase: u32,
    pub envelope_phase_increment: u32,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct FofState {
    pub next_saw_sample: i32,
    pub previous_sample: i16,
    pub svf_lp: [i32; K_NUM_FORMANTS],
    pub svf_bp: [i32; K_NUM_FORMANTS],
}

#[derive(Clone, Copy, Default)]
pub(crate) struct ToyState {
    pub held_sample: u8,
    pub decimation_counter: u16,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct SvfState {
    pub bp: i32,
    pub lp: i32,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct DigitalModulationState {
    pub symbol_phase: u32,
    pub symbol_count: u16,
    pub filter_state: i32,
    pub data_byte: u8,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct ClockedNoiseState {
    pub cycle_phase: u32,
    pub cycle_phase_increment: u32,
    pub rng_state: u32,
    pub seed: i32,
    pub sample: i16,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct HatState {
    pub phase: [u32; 6],
    pub rng_state: u32,
}

/// `union DigitalOscillatorState`, with one field per member.
#[derive(Clone, Copy, Default)]
pub(crate) struct DigitalOscillatorState {
    pub res: ResoSquareState,
    pub vow: VowelSynthesizerState,
    pub saw: SawSwarmState,
    pub plk: [PluckState; 4],
    pub ffm: FeedbackFmState,
    pub pno: ParticleNoiseState,
    pub phy: PhysicalModellingState,
    pub grain: [Grain; 4],
    pub fof: FofState,
    pub toy: ToyState,
    pub svf: SvfState,
    pub add: AdditiveState,
    pub dmd: DigitalModulationState,
    pub clk: ClockedNoiseState,
    pub hat: HatState,
    pub hrm: HarmonicsState,
    pub modulator_phase: u32,
}

/// The delay-line union, one buffer per member.
pub(crate) struct DelayLines {
    pub comb: Box<[i16]>,
    /// `ks[1025 * 4]`.
    pub ks: Box<[i16]>,
    pub bowed_bridge: Box<[i8]>,
    pub bowed_neck: Box<[i8]>,
    pub bore: Box<[i16]>,
    pub fluted_jet: Box<[i8]>,
    pub fluted_bore: Box<[i8]>,
}

pub struct DigitalOscillator {
    pub(crate) phase: u32,
    pub(crate) phase_increment: u32,
    pub(crate) delay: u32,
    pub(crate) parameter: [i16; 2],
    pub(crate) previous_parameter: [i16; 2],
    pub(crate) smoothed_parameter: i32,
    pub(crate) pitch: i16,
    pub(crate) active_voice: u8,
    pub(crate) init: bool,
    pub(crate) strike: bool,
    pub(crate) shape: usize,
    pub(crate) previous_shape: usize,
    pub(crate) state: DigitalOscillatorState,
    pub(crate) pulse: [Excitation; 4],
    pub(crate) svf: [Svf; 3],
    pub(crate) delay_lines: DelayLines,
}

impl DigitalOscillator {
    pub fn new_boxed() -> Box<Self> {
        let mut d = Box::new(Self {
            phase: 0,
            phase_increment: 0,
            delay: 0,
            parameter: [0; 2],
            previous_parameter: [0; 2],
            smoothed_parameter: 0,
            pitch: 60 << 7,
            active_voice: 0,
            init: true,
            strike: true,
            shape: 0,
            previous_shape: usize::MAX,
            state: DigitalOscillatorState::default(),
            pulse: [Excitation::default(); 4],
            svf: [Svf::default(); 3],
            delay_lines: DelayLines {
                comb: vec![0; K_COMB_DELAY_LENGTH].into_boxed_slice(),
                ks: vec![0; 1025 * 4].into_boxed_slice(),
                bowed_bridge: vec![0; K_WG_BRIDGE_LENGTH].into_boxed_slice(),
                bowed_neck: vec![0; K_WG_NECK_LENGTH].into_boxed_slice(),
                bore: vec![0; K_WG_BORE_LENGTH].into_boxed_slice(),
                fluted_jet: vec![0; K_WG_JET_LENGTH].into_boxed_slice(),
                fluted_bore: vec![0; K_WG_FBORE_LENGTH].into_boxed_slice(),
            },
        });
        d.init();
        d
    }

    /// `Init()`: clears the per-shape state (the delay lines are kept).
    pub fn init(&mut self) {
        self.state = DigitalOscillatorState::default();
        for p in self.pulse.iter_mut() {
            p.init();
        }
        for s in self.svf.iter_mut() {
            s.init();
        }
        self.phase = 0;
        self.strike = true;
        self.init = true;
    }

    /// Selects a shape by its position in `fn_table_` (= macro shape - 13).
    pub fn set_shape(&mut self, shape: usize) {
        self.shape = shape.min(OSC_SHAPE_QUESTION_MARK_LAST);
    }

    pub fn set_pitch(&mut self, pitch: i16) {
        // Smooth HF noise when the pitch CV is noisy.
        if self.pitch as i32 > (90 << 7) && pitch as i32 > (90 << 7) {
            self.pitch = ((self.pitch as i32 + pitch as i32) >> 1) as i16;
        } else {
            self.pitch = pitch;
        }
    }

    pub fn set_parameters(&mut self, parameter_1: i16, parameter_2: i16) {
        self.parameter = [parameter_1, parameter_2];
    }

    pub fn phase_increment(&self) -> u32 {
        self.phase_increment
    }

    pub fn strike(&mut self) {
        self.strike = true;
    }

    pub(crate) fn compute_phase_increment(&self, midi_pitch: i16) -> u32 {
        let mut midi_pitch = midi_pitch as i32;
        if midi_pitch >= K_PITCH_TABLE_START {
            midi_pitch = K_PITCH_TABLE_START - 1;
        }
        let mut ref_pitch = midi_pitch - K_PITCH_TABLE_START;
        let mut num_shifts = 0;
        while ref_pitch < 0 {
            ref_pitch += K_OCTAVE;
            num_shifts += 1;
        }
        let a = LUT_OSCILLATOR_INCREMENTS[(ref_pitch >> 4) as usize];
        let b = LUT_OSCILLATOR_INCREMENTS[((ref_pitch >> 4) + 1) as usize];
        let phase_increment = a.wrapping_add(((b.wrapping_sub(a) as i32).wrapping_mul(ref_pitch & 0xf) >> 4) as u32);
        phase_increment.checked_shr(num_shifts).unwrap_or(0)
    }

    pub(crate) fn compute_delay(&self, midi_pitch: i16) -> u32 {
        let mut midi_pitch = midi_pitch as i32;
        if midi_pitch >= K_HIGHEST_NOTE - K_OCTAVE {
            midi_pitch = K_HIGHEST_NOTE - K_OCTAVE;
        }
        let mut ref_pitch = midi_pitch - K_PITCH_TABLE_START;
        let mut num_shifts: i32 = 0;
        while ref_pitch < 0 {
            ref_pitch += K_OCTAVE;
            num_shifts += 1;
        }
        let a = LUT_OSCILLATOR_DELAYS[(ref_pitch >> 4) as usize];
        let b = LUT_OSCILLATOR_DELAYS[((ref_pitch >> 4) + 1) as usize];
        let delay = a.wrapping_add(((b.wrapping_sub(a) as i32).wrapping_mul(ref_pitch & 0xf) >> 4) as u32);
        let shift = 12 - num_shifts;
        if shift >= 0 {
            delay.checked_shr(shift as u32).unwrap_or(0)
        } else {
            delay.checked_shl((-shift) as u32).unwrap_or(0)
        }
    }

    /// Renders `buffer.len()` (at most 24) samples.
    pub fn render(&mut self, sync: &[u8], buffer: &mut [i16]) {
        // Quantize parameter for FM.
        if self.shape >= OSC_SHAPE_FM && self.shape <= OSC_SHAPE_CHAOTIC_FEEDBACK_FM {
            let p = self.parameter[1] as i32;
            let integral = (p >> 8).clamp(0, 127) as usize;
            let fractional = p & 255;
            let a = LUT_FM_FREQUENCY_QUANTIZER[integral] as i16 as i32;
            let b = LUT_FM_FREQUENCY_QUANTIZER[integral + 1] as i16 as i32;
            self.parameter[1] = (a + ((b - a) * fractional >> 8)) as i16;
        }

        if self.shape != self.previous_shape {
            self.init();
            self.previous_shape = self.shape;
            self.init = true;
        }

        self.phase_increment = self.compute_phase_increment(self.pitch);
        self.delay = self.compute_delay(self.pitch);

        if self.pitch as i32 > K_HIGHEST_NOTE {
            self.pitch = K_HIGHEST_NOTE as i16;
        } else if self.pitch < 0 {
            self.pitch = 0;
        }

        // `fn_table_` order.
        match self.shape {
            0 => self.render_triple_ring_mod(sync, buffer),
            1 => self.render_saw_swarm(sync, buffer),
            2 => self.render_comb(sync, buffer),
            3 => self.render_toy(sync, buffer),
            4..=7 => self.render_digital_filter(sync, buffer),
            8 => self.render_vosim(sync, buffer),
            9 => self.render_vowel(sync, buffer),
            10 => self.render_vowel_fof(sync, buffer),
            11 => self.render_harmonics(sync, buffer),
            12 => self.render_fm(sync, buffer),
            13 => self.render_feedback_fm(sync, buffer),
            14 => self.render_chaotic_feedback_fm(sync, buffer),
            15 => self.render_plucked(sync, buffer),
            16 => self.render_bowed(sync, buffer),
            17 => self.render_blown(sync, buffer),
            18 => self.render_fluted(sync, buffer),
            19 => self.render_struck_bell(sync, buffer),
            20 => self.render_struck_drum(sync, buffer),
            21 => self.render_kick(sync, buffer),
            22 => self.render_cymbal(sync, buffer),
            23 => self.render_snare(sync, buffer),
            24 => self.render_wavetables(sync, buffer),
            25 => self.render_wave_map(sync, buffer),
            26 => self.render_wave_line(sync, buffer),
            27 => self.render_wave_paraphonic(sync, buffer),
            28 => self.render_filtered_noise(sync, buffer),
            29 => self.render_twin_peaks_noise(sync, buffer),
            30 => self.render_clocked_noise(sync, buffer),
            31 => self.render_granular_cloud(sync, buffer),
            32 => self.render_particle_noise(sync, buffer),
            33 => self.render_digital_modulation(sync, buffer),
            _ => self.render_question_mark(sync, buffer),
        }
    }
}
