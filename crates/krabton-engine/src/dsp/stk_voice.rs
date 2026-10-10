//! Voice: A singing voice from formant filters: 32 vowels and consonants, from the Synthesis ToolKit (see `stk_synth`).

use super::stk_synth::{self, Family, StkState};
use crate::params::{ParamSpec, Unit};

pub const MODELS: &[&str] = &["Eee", "Ihh", "Ehh", "Aaa", "Ahh", "Aww", "Ohh", "Uhh", "Uuu", "Ooo", "Rrr", "Lll", "Mmm", "Nnn", "Nng", "Ngg", "Fff", "Sss", "Thh", "Shh", "Xxx", "Hee", "Hoo", "Hah", "Bbb", "Ddd", "Jjj", "Ggg", "Vvv", "Zzz", "Thz", "Zhh"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::int("Voices", "Voice", 1.0, 8.0, 1.0, Unit::None),
    ParamSpec::lin("Volume", "Voice", -24.0, 12.0, 0.0, Unit::Db),
    ParamSpec::lin("Control 1", "Controls", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Control 2", "Controls", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Control 3", "Controls", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Control 4", "Controls", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Control 5", "Controls", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Control 6", "Controls", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Control 7", "Controls", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::choice("Model", "Model", MODELS, 0),
];

pub fn preset_names() -> &'static [&'static str] {
    stk_synth::preset_names(Family::Voice)
}

pub fn preset(name: &str) -> Vec<f32> {
    stk_synth::preset(Family::Voice, name)
}

pub struct VoiceState(StkState);

impl VoiceState {
    pub fn new(sr: f32) -> Self {
        Self(StkState::new(Family::Voice, sr))
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        self.0.note_on(p, note, vel);
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.0.note_off(p, note);
    }

    pub fn all_off(&mut self) {
        self.0.all_off();
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        self.0.render(p, l, r);
    }
}
