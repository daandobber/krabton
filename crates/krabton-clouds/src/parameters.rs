//! The parameters seen by the processor (`clouds/dsp/parameters.h`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

#[derive(Clone, Copy, Debug, Default)]
pub struct Granular {
    pub overlap: f32,
    pub window_shape: f32,
    pub stereo_spread: f32,
    pub use_deterministic_seed: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Spectral {
    pub quantization: f32,
    pub refresh_rate: f32,
    pub phase_randomization: f32,
    pub warp: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Parameters {
    pub position: f32,
    pub size: f32,
    pub pitch: f32,
    pub density: f32,
    pub texture: f32,
    pub dry_wet: f32,
    pub stereo_spread: f32,
    pub feedback: f32,
    pub reverb: f32,

    pub freeze: bool,
    /// Rising edge of the trigger input, for one block.
    pub trigger: bool,
    /// Level of the trigger input.
    pub gate: bool,

    pub granular: Granular,
    pub spectral: Spectral,
}
