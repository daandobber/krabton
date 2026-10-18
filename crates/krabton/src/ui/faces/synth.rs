//! Synth laid out as the slider polysynths of the eighties: a slider for
//! everything, in sections from the LFO through the oscillators and the
//! filter to the two envelopes, the waves and modes on lit buttons.

use super::{draw, Ctl, Ctx, Face, Presets};
use crate::ui::inst_screen::{synth_screen, InstLive};
use crate::ui::modulation::ModView;
use egui::{Id, Ui};
use krabton_engine::dsp::synth;

pub(crate) fn face() -> Face {
    Face::new(53.6, 12.0)
        .section(0.0, 0.0, 6.6, 12.0, "Memory")
        .preset(0.3, 0.4, 6.0, 1.4)
        .named(0.3, 2.1, 6.0, 3.6, "Voices", Ctl::Buttons, "Assign")
        .knob(0.3, 6.0, 3.0, 5.1, "Glide")
        .knob(3.3, 6.0, 3.0, 5.1, "Volume")
        .section(6.8, 0.0, 5.6, 12.0, "LFO")
        .named(6.95, 0.3, 1.75, 10.8, "Rate", Ctl::Fader, "Rate")
        .named(8.7, 0.3, 1.75, 10.8, "To Cutoff", Ctl::Fader, "VCF")
        .named(10.45, 0.3, 1.75, 10.8, "To Pitch", Ctl::Fader, "DCO")
        .section(12.6, 0.0, 15.4, 12.0, "DCO")
        .named(12.7, 0.3, 3.2, 5.3, "Osc 1", Ctl::Buttons, "Osc 1")
        .named(12.7, 5.8, 3.2, 5.3, "Osc 2", Ctl::Buttons, "Osc 2")
        .named(16.1, 0.3, 1.95, 10.8, "Osc 2 Pitch", Ctl::Fader, "Pitch")
        .named(18.05, 0.3, 1.95, 10.8, "Osc 2 Fine", Ctl::Fader, "Fine")
        .named(20.0, 0.3, 1.95, 10.8, "Osc Mix", Ctl::Fader, "Mix")
        .named(21.95, 0.3, 1.95, 10.8, "Sub", Ctl::Fader, "Sub")
        .named(23.9, 0.3, 1.95, 10.8, "Noise", Ctl::Fader, "Noise")
        .named(25.85, 0.3, 1.95, 10.8, "Unison", Ctl::Fader, "Unison")
        .section(28.2, 0.0, 12.0, 12.0, "VCF")
        .named(28.3, 0.3, 3.0, 6.0, "Type", Ctl::Buttons, "Mode")
        .screen(28.4, 6.6, 2.8, 4.4, "filter")
        .named(31.5, 0.3, 1.75, 10.8, "Cutoff", Ctl::Fader, "Freq")
        .named(33.25, 0.3, 1.75, 10.8, "Resonance", Ctl::Fader, "Res")
        .named(35.0, 0.3, 1.75, 10.8, "Env Amt", Ctl::Fader, "Env")
        .named(36.75, 0.3, 1.75, 10.8, "Key Track", Ctl::Fader, "Kybd")
        .named(38.5, 0.3, 1.6, 10.8, "Drive", Ctl::Fader, "Drive")
        .section(40.4, 0.0, 6.4, 12.0, "Filter Env")
        .named(40.5, 0.3, 1.55, 7.6, "Filter Env:Attack", Ctl::Fader, "A")
        .named(42.05, 0.3, 1.55, 7.6, "Filter Env:Decay", Ctl::Fader, "D")
        .named(43.6, 0.3, 1.55, 7.6, "Filter Env:Sustain", Ctl::Fader, "S")
        .named(45.15, 0.3, 1.55, 7.6, "Filter Env:Release", Ctl::Fader, "R")
        .screen(40.6, 8.1, 6.0, 3.0, "fenv")
        .section(47.0, 0.0, 6.6, 12.0, "Amp Env")
        .named(47.1, 0.3, 1.6, 7.6, "Amp Env:Attack", Ctl::Fader, "A")
        .named(48.7, 0.3, 1.6, 7.6, "Amp Env:Decay", Ctl::Fader, "D")
        .named(50.3, 0.3, 1.6, 7.6, "Amp Env:Sustain", Ctl::Fader, "S")
        .named(51.9, 0.3, 1.6, 7.6, "Amp Env:Release", Ctl::Fader, "R")
        .screen(47.2, 8.1, 6.2, 3.0, "aenv")
}

pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut [f32], live: &InstLive, height: f32, mv: Option<&mut ModView>) -> bool {
    let face = face();
    let mut cx = Ctx { id, keys: synth::SPECS, specs: synth::SPECS, params, mv, presets: Some(Presets { current: preset, names: synth::preset_names(), load: synth::preset }) };
    let changed = draw(ui, &face, height, &mut cx, &mut |ui, which, rect, p| synth_screen(ui, id.with(which), rect, which, p, live));
    if live.vals[0] > 0.0 {
        ui.ctx().request_repaint();
    }
    changed
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_slider_of_the_synth_is_on_its_face() {
        super::super::check(&super::face(), krabton_engine::dsp::synth::SPECS);
    }
}
