//! A Rust port of the DSP code of the Mutable Instruments Braids
//! macro-oscillator, by Emilie Gillet (MIT license, see the headers of each
//! module and <https://github.com/pichenettes/eurorack>).
//!
//! Braids renders 16-bit fixed-point audio at 96 kHz in blocks of up to 24
//! samples. Pitch is in 1/128 semitones (MIDI note 60 = `60 << 7`); the two
//! parameters TIMBRE and COLOR go from 0 to 32767.
//!
//! The port keeps the C++ integer semantics. Where the original relies on
//! two's-complement wrapping, so does this code: the workspace disables
//! overflow checks for this crate in debug builds as well.

#![allow(clippy::all)]

pub mod analog;
pub mod digital;
mod digital_a;
mod digital_b;
pub mod dsp;
pub mod excitation;
pub mod jitter;
pub mod macro_osc;
pub mod resources;
pub mod svf;

pub use macro_osc::{MacroOscillator, SHAPES, SHAPE_COUNT};

/// The sample rate the tables and algorithms were made for.
pub const SAMPLE_RATE: f32 = 96000.0;
/// Largest block `MacroOscillator::render` accepts.
pub const BLOCK_SIZE: usize = 24;
