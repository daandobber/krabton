//! Bassline laid out as the little silver acid box: its six knobs in a row
//! (tuning, cutoff, resonance, envelope, decay, accent) with the waveform
//! at the start and the volume at the end, and under them the filter screen
//! and the trimmers that sit inside the box.

use super::{draw, Ctl, Ctx, Face, Presets};
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use egui::{Id, Ui, UiBuilder};
use krabton_engine::dsp::bassline;

pub(crate) fn face() -> Face {
    Face::new(31.0, 13.4)
        // The strip along the top, with the memory's display.
        .text(0.3, 0.1, 12.0, 1.4, "Computer controlled · Bass line")
        .preset(20.4, 0.1, 10.3, 1.4)
        .section(0.0, 1.65, 31.0, 5.9, "Bass Line")
        .named(0.3, 1.8, 3.2, 4.8, "Waveform", Ctl::Knob, "Waveform")
        .named(3.6, 1.8, 3.6, 4.8, "Tuning", Ctl::Knob, "Tuning")
        .named(7.2, 1.65, 4.6, 5.0, "Cutoff", Ctl::Knob, "Cut Off Freq")
        .named(11.8, 1.65, 4.6, 5.0, "Resonance", Ctl::Knob, "Resonance")
        .named(16.4, 1.8, 3.6, 4.8, "Env Mod", Ctl::Knob, "Env Mod")
        .named(20.0, 1.8, 3.6, 4.8, "Decay", Ctl::Knob, "Decay")
        .named(23.6, 1.8, 3.6, 4.8, "Accent", Ctl::Knob, "Accent")
        .named(27.3, 1.8, 3.4, 4.8, "Volume", Ctl::Knob, "Volume")
        .section(0.0, 7.7, 9.4, 5.7, "Filter")
        .screen(0.3, 7.9, 8.8, 4.5, "screen")
        .section(9.6, 7.7, 21.4, 5.7, "Inside")
        .row(9.8, 7.8, 3.0, 2.3, &["Slide Time", "Square Phase", "Normal Attack", "Accent At", "Accent Attack", "Accent Decay", "Drive"], Ctl::Knob)
        .row(9.8, 10.1, 3.0, 2.3, &["Amp Decay", "Amp Sustain", "Amp Release", "Pre HPF", "Feedback HPF", "Post HPF"], Ctl::Knob)
}

pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut [f32], live: &InstLive, height: f32, mv: Option<&mut ModView>) -> bool {
    let face = face();
    let mut cx = Ctx { id, keys: bassline::SPECS, specs: bassline::SPECS, params, mv, presets: Some(Presets { current: preset, names: bassline::preset_names(), load: bassline::preset }) };
    let mut changed = false;
    ui.horizontal_top(|ui| {
        changed = draw(ui, &face, height, &mut cx, &mut |ui, _, rect, p| {
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| crate::ui::engine_screens::bassline_screen_w(ui, id.with("screen"), p, live, rect.width(), rect.height())).inner
        });
    });
    changed
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_knob_of_the_bassline_is_on_its_face() {
        super::super::check(&super::face(), krabton_engine::dsp::bassline::SPECS);
    }
}
