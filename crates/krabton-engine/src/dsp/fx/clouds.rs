//! Clouds: the Mutable Instruments Clouds granular processor (Emilie
//! Gillet, MIT; see `krabton_clouds`) as an effect: grains, time stretch,
//! looping delay and spectral freezes of the track's audio.

use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};
use krabton_clouds::{Clouds, Params, PlaybackMode};

pub const MODE: usize = 0;
pub const QUALITY: usize = 1;
pub const POSITION: usize = 2;
pub const SIZE: usize = 3;
pub const PITCH: usize = 4;
pub const DENSITY: usize = 5;
pub const TEXTURE: usize = 6;
pub const DRY_WET: usize = 7;
pub const SPREAD: usize = 8;
pub const FEEDBACK: usize = 9;
pub const REVERB: usize = 10;
pub const FREEZE: usize = 11;

pub const MODES: &[&str] = &["Granular", "Stretch", "Looping Delay", "Spectral"];
pub const QUALITIES: &[&str] = &["16-bit Stereo", "16-bit Mono", "Lo-Fi Stereo", "Lo-Fi Mono"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Mode", "Clouds", MODES, 0),
    ParamSpec::choice("Quality", "Clouds", QUALITIES, 0),
    ParamSpec::lin("Position", "Grains", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Size", "Grains", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Pitch", "Grains", -48.0, 48.0, 0.0, Unit::Semi),
    ParamSpec::lin("Density", "Grains", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Texture", "Grains", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Dry/Wet", "Blend", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Spread", "Blend", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Feedback", "Blend", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Reverb", "Blend", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Freeze", "Blend", false),
];

pub struct CloudsFx {
    clouds: Box<Clouds>,
}

impl CloudsFx {
    pub fn new(sr: f32) -> Self {
        Self { clouds: Box::new(Clouds::new(sr)) }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let mode = match p[MODE].round() as i32 {
            1 => PlaybackMode::Stretch,
            2 => PlaybackMode::LoopingDelay,
            3 => PlaybackMode::Spectral,
            _ => PlaybackMode::Granular,
        };
        let params = Params {
            mode,
            quality: p[QUALITY].round().clamp(0.0, 3.0) as u8,
            position: p[POSITION],
            size: p[SIZE],
            pitch: p[PITCH].round(),
            density: p[DENSITY],
            texture: p[TEXTURE],
            dry_wet: p[DRY_WET],
            stereo_spread: p[SPREAD],
            feedback: p[FEEDBACK],
            reverb: p[REVERB],
            freeze: p[FREEZE] >= 0.5,
            trigger: false,
        };
        self.clouds.process(&params, l, r);
    }
}
