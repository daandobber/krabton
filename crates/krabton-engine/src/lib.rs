//! Krabton engine: project model, DSP and the real-time audio engine.
//!
//! This crate has no GUI or audio-device dependencies, so it can be tested
//! and used headless (e.g. for offline rendering).

pub mod audio_in;
pub mod classic_seq;
pub mod demo;
pub mod dsp;
pub mod engine;
pub mod midi_fx;
pub mod midi_tools;
pub mod modular;
pub mod modulation;
pub mod params;
pub mod plugin;
pub mod project;
pub mod render;
pub mod sample;
pub mod sample_map;
pub mod sequencers;
pub mod routing;
pub mod scale;
pub mod scope;

pub use engine::{Command, Engine, EngineHandle, EngineStatus, RecChunk};
pub use project::*;
pub use routing::{AudioInput, AudioOutput, MidiInput, Monitor, Tap};
pub use sample::{SampleData, SampleRef};
pub use scale::{Scale, ScaleKind};
