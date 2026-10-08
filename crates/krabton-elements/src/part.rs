//! The voice(s), the stereo mix and the reverb (`elements/dsp/part.h`,
//! `part.cc`).
//!
//! The module plays a single voice. This port runs any number of voices,
//! each with its own gate, note and strength, into the shared reverb, and
//! skips voices that have been silent for a while.
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::soft_limit;
use crate::fx::Reverb;
use crate::resources::{LUT_MIDI_TO_F_HIGH, LUT_MIDI_TO_F_LOW};
use crate::voice::{Patch, ResonatorModel, Voice};
use crate::{BLOCK_SIZE, SAMPLE_RATE};

/// What a voice is told to play.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PerformanceState {
    pub gate: bool,
    /// MIDI note (fractional).
    pub note: f32,
    /// Pitch offset in semitones.
    pub modulation: f32,
    /// Accent, 0 to 1; 0.5 is neutral.
    pub strength: f32,
}

impl Default for PerformanceState {
    fn default() -> Self {
        Self { gate: false, note: 69.0, modulation: 0.0, strength: 0.5 }
    }
}

/// Blocks of silence after which a released voice is no longer rendered
/// (a quarter second).
const QUIET_BLOCKS: u32 = (SAMPLE_RATE as u32 / BLOCK_SIZE as u32) / 4;
const QUIET_LEVEL: f32 = 3e-5;

pub struct Part {
    pub patch: Patch,
    voices: Vec<Voice>,
    quiet: Vec<u32>,
    panic: bool,
    resonator_model: ResonatorModel,
    silence: [f32; BLOCK_SIZE],
    raw_buffer: [f32; BLOCK_SIZE],
    center_buffer: [f32; BLOCK_SIZE],
    sides_buffer: [f32; BLOCK_SIZE],
    resonator_level: f32,
    reverb: Reverb,
}

/// Normalized frequency of a MIDI note, from the module's tables (capped
/// at 12 kHz).
pub fn note_to_frequency(midi_pitch: f32) -> f32 {
    let pitch = (((midi_pitch + 48.0) * 256.0) as i32).clamp(0, 65535) as usize;
    LUT_MIDI_TO_F_HIGH[pitch >> 8] * LUT_MIDI_TO_F_LOW[pitch & 0xff]
}

impl Part {
    pub fn new(num_voices: usize) -> Self {
        let n = num_voices.max(1);
        Self {
            patch: Patch::default(),
            voices: (0..n).map(|_| Voice::new()).collect(),
            quiet: vec![QUIET_BLOCKS + 1; n],
            panic: false,
            resonator_model: ResonatorModel::Modal,
            silence: [0.0; BLOCK_SIZE],
            raw_buffer: [0.0; BLOCK_SIZE],
            center_buffer: [0.0; BLOCK_SIZE],
            sides_buffer: [0.0; BLOCK_SIZE],
            resonator_level: 0.0,
            reverb: Reverb::default(),
        }
    }

    pub fn num_voices(&self) -> usize {
        self.voices.len()
    }

    pub fn resonator_model(&self) -> ResonatorModel {
        self.resonator_model
    }

    pub fn set_resonator_model(&mut self, model: ResonatorModel) {
        self.resonator_model = model;
    }

    /// Whether voice `i` is still being rendered.
    pub fn voice_active(&self, i: usize) -> bool {
        self.quiet.get(i).is_some_and(|&q| q <= QUIET_BLOCKS)
    }

    /// Silences all resonators and the reverb.
    pub fn panic(&mut self) {
        self.panic = true;
    }

    /// The module seeds a few hidden settings (the pickup LFO, reverb
    /// colour and exciter signature) from its serial number; this does the
    /// same from any words.
    pub fn seed(&mut self, seed: &[u32]) {
        let mut signature: u32 = 0xf0cacc1a;
        for &s in seed {
            signature ^= s;
            signature = signature.wrapping_mul(1664525).wrapping_add(1013904223);
        }
        let mut next = || {
            let x = (signature & 7) as f32 / 8.0;
            signature >>= 3;
            x
        };
        let p = &mut self.patch;
        p.resonator_modulation_frequency = (0.4 + 0.8 * next()) / SAMPLE_RATE;
        p.resonator_modulation_offset = 0.05 + 0.1 * next();
        p.reverb_diffusion = 0.55 + 0.15 * next();
        p.reverb_lp = 0.7 + 0.2 * next();
        p.exciter_signature = next();
    }

    /// Renders one block (at most [`BLOCK_SIZE`] samples). `states[i]`
    /// drives voice `i`; missing states mean a closed gate.
    pub fn process(&mut self, states: &[PerformanceState], main: &mut [f32], aux: &mut [f32]) {
        let size = main.len().min(aux.len()).min(BLOCK_SIZE);
        let (main, aux) = (&mut main[..size], &mut aux[..size]);
        main.fill(0.0);
        aux.fill(0.0);
        if self.panic {
            for v in self.voices.iter_mut() {
                v.panic();
            }
            self.reverb.clear();
            self.resonator_level = 0.0;
            self.panic = false;
            return;
        }

        let space = self.patch.space.min(1.0);
        let raw_gain = if space <= 0.05 {
            1.0
        } else if space <= 0.1 {
            2.0 - space * 20.0
        } else {
            0.0
        };
        let space = if space >= 0.1 { space - 0.1 } else { 0.0 };
        let spread = space.min(0.7);
        let reverb_amount = if space >= 0.5 { space - 0.5 } else { 0.0 };
        let reverb_time = 0.35 + 1.2 * reverb_amount;

        let patch = self.patch;
        for (i, voice) in self.voices.iter_mut().enumerate() {
            let state = states.get(i).copied().unwrap_or(PerformanceState { gate: false, ..Default::default() });
            if !state.gate && self.quiet[i] > QUIET_BLOCKS {
                continue;
            }
            voice.set_resonator_model(self.resonator_model);
            voice.process(
                &patch,
                note_to_frequency(state.note + state.modulation),
                state.strength,
                state.gate,
                &self.silence[..size],
                &self.silence[..size],
                &mut self.raw_buffer[..size],
                &mut self.center_buffer[..size],
                &mut self.sides_buffer[..size],
            );
            let mut peak = 0.0f32;
            for j in 0..size {
                let side = self.sides_buffer[j] * spread;
                let r = self.center_buffer[j] - side;
                let l = self.center_buffer[j] + side;
                main[j] += r;
                aux[j] += l + (self.raw_buffer[j] - l) * raw_gain;
                peak = peak.max(r.abs()).max(l.abs()).max(self.raw_buffer[j].abs());
            }
            if state.gate || peak > QUIET_LEVEL {
                self.quiet[i] = 0;
            } else {
                self.quiet[i] += 1;
            }
        }

        for (m, a) in main.iter_mut().zip(aux.iter_mut()) {
            *m = soft_limit(*m);
            *a = soft_limit(*a);
        }

        // Detect whether the resonator went haywire.
        let mut resonator_level = self.resonator_level;
        for &m in main.iter() {
            let error = m * m - resonator_level;
            resonator_level += error * if error > 0.0 { 0.05 } else { 0.0005 };
        }
        self.resonator_level = resonator_level;
        if resonator_level >= 200.0 || !resonator_level.is_finite() {
            self.panic = true;
        }

        self.reverb.set_amount(reverb_amount);
        self.reverb.set_diffusion(self.patch.reverb_diffusion);
        let freeze = self.patch.space >= 1.75;
        if freeze {
            self.reverb.set_time(1.0);
            self.reverb.set_input_gain(0.0);
            self.reverb.set_lp(1.0);
        } else {
            self.reverb.set_time(reverb_time);
            self.reverb.set_input_gain(0.2);
            self.reverb.set_lp(self.patch.reverb_lp);
        }
        self.reverb.process(main, aux);
    }
}
