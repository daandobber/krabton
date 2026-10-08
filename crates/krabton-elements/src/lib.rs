//! A Rust port of the DSP code of the Mutable Instruments Elements modal
//! synthesis voice, by Emilie Gillet (MIT license, see the headers of each
//! module and <https://github.com/pichenettes/eurorack>).
//!
//! Elements renders floating-point audio at 32 kHz in blocks of up to 16
//! samples. An exciter section (bow, blow, strike) feeds a resonator (a bank
//! of 52 modal filters, or Karplus-Strong strings), followed by a reverb.
//! [`Part`] ties it together. Unlike the module, which plays one voice, a
//! `Part` can run several voices into the shared reverb.

#![allow(clippy::too_many_arguments, clippy::needless_range_loop)]

pub mod dsp;
pub mod envelope;
pub mod exciter;
pub mod filter;
pub mod fx;
pub mod part;
pub mod resonator;
pub mod resources;
pub mod string;
pub mod tube;
pub mod voice;

pub use part::{Part, PerformanceState};
pub use voice::{Patch, ResonatorModel, Voice};

/// The sample rate the tables and algorithms were made for.
pub const SAMPLE_RATE: f32 = 32000.0;
/// Block size: the exciter envelope and controls update once per block.
pub const BLOCK_SIZE: usize = 16;

#[cfg(test)]
mod tests;
