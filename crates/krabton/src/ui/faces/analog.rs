//! Analog laid out as the classic analog synth of the seventies: from left
//! to right the controllers, the modulation, the oscillator bank with its
//! range and waveform selectors, the mixer, the modifiers (the filter with
//! its big cutoff, its contour and the loudness contour) and the output.

use super::{draw, Ctl, Ctx, Face, Presets};
use crate::ui::inst_screen::{analog_screen, InstLive};
use crate::ui::modulation::ModView;
use egui::{Id, Ui};
use krabton_engine::dsp::analog;

pub(crate) fn face() -> Face {
    Face::new(55.0, 12.0)
        // Controllers: the preset, glide and how the voices play.
        .section(0.0, 0.0, 6.6, 12.0, "Controllers")
        .preset(0.3, 0.4, 6.0, 1.4)
        .named(0.3, 2.1, 3.2, 4.4, "Glide", Ctl::Knob, "Glide")
        .knob(3.6, 2.1, 2.7, 2.2, "Vintage")
        .knob(3.6, 4.3, 2.7, 2.2, "Spread")
        .row(0.3, 7.0, 2.0, 4.0, &["Voices", "Unison", "Uni Detune"], Ctl::Knob)
        // Modulation: the LFO.
        .section(6.8, 0.0, 6.6, 12.0, "Modulation")
        .named(7.0, 0.4, 3.7, 4.8, "LFO Wave", Ctl::Selector, "Wave")
        .named(10.7, 0.4, 2.6, 4.8, "LFO Rate", Ctl::Knob, "Rate")
        .screen(7.1, 5.4, 6.0, 1.8, "lfo")
        .named(7.0, 7.4, 1.55, 3.7, "LFO Delay", Ctl::Knob, "Delay")
        .named(8.55, 7.4, 1.55, 3.7, "LFO Pitch", Ctl::Knob, "Pitch")
        .named(10.1, 7.4, 1.55, 3.7, "LFO Filter", Ctl::Knob, "Filter")
        .named(11.65, 7.4, 1.55, 3.7, "LFO PWM", Ctl::Knob, "PWM")
        // The oscillator bank: two rows, range and waveform on selectors.
        .section(13.6, 0.0, 13.4, 12.0, "Oscillator Bank")
        .named(13.8, 0.3, 3.4, 5.3, "Octave 1", Ctl::Selector, "Range")
        .named(17.2, 0.3, 3.4, 5.3, "Wave 1", Ctl::Selector, "Waveform")
        .named(20.6, 0.3, 2.1, 2.65, "PW 1", Ctl::Knob, "Width")
        .named(20.6, 2.95, 2.1, 2.65, "Cross Mod", Ctl::Knob, "Cross Mod")
        .screen(22.9, 0.6, 3.9, 4.8, "osc1")
        .named(13.8, 5.7, 3.4, 5.4, "Octave 2", Ctl::Selector, "Range")
        .named(17.2, 5.7, 3.4, 5.4, "Wave 2", Ctl::Selector, "Waveform")
        .named(20.6, 5.7, 2.1, 2.7, "PW 2", Ctl::Knob, "Width")
        .named(20.6, 8.4, 2.1, 2.7, "Sync", Ctl::Switch, "Sync")
        .named(22.7, 5.7, 2.1, 2.7, "Semi 2", Ctl::Knob, "Semi")
        .named(22.7, 8.4, 2.1, 2.7, "Detune 2", Ctl::Knob, "Frequency")
        .named(24.8, 5.7, 2.1, 2.7, "Env Pitch 2", Ctl::Knob, "Env Pitch")
        .screen(24.9, 8.6, 1.9, 2.4, "osc2")
        // The mixer: a level per source.
        .section(27.2, 0.0, 3.6, 12.0, "Mixer")
        .col(27.4, 0.3, 3.2, 2.7, &["Osc 1", "Osc 2", "Sub", "Noise"], Ctl::Knob)
        // The modifiers: the filter on top, its contour and the loudness contour below.
        .section(31.0, 0.0, 19.4, 12.0, "Modifiers")
        .named(31.2, 0.3, 4.4, 5.4, "Cutoff", Ctl::Knob, "Cutoff Frequency")
        .named(35.6, 0.6, 3.0, 4.8, "Resonance", Ctl::Knob, "Emphasis")
        .named(38.6, 0.6, 3.0, 4.8, "Env Amount", Ctl::Knob, "Contour")
        .named(41.6, 0.3, 3.4, 5.4, "Filter", Ctl::Selector, "Mode")
        .knob(45.0, 0.3, 2.1, 2.7, "Drive")
        .named(45.0, 3.0, 2.1, 2.7, "Key Track", Ctl::Knob, "Keyboard")
        .screen(47.3, 0.6, 2.9, 4.8, "filter")
        .text(31.2, 5.8, 1.9, 2.65, "Filter")
        .named(33.1, 5.8, 2.15, 2.65, "Flt Attack", Ctl::Knob, "Attack")
        .named(35.25, 5.8, 2.15, 2.65, "Flt Decay", Ctl::Knob, "Decay")
        .named(37.4, 5.8, 2.15, 2.65, "Flt Sustain", Ctl::Knob, "Sustain")
        .named(39.55, 5.8, 2.15, 2.65, "Flt Release", Ctl::Knob, "Release")
        .named(41.7, 5.8, 2.15, 2.65, "Flt Velocity", Ctl::Knob, "Velocity")
        .screen(44.0, 5.95, 6.2, 2.4, "fenv")
        .text(31.2, 8.45, 1.9, 2.65, "Loudness")
        .row(33.1, 8.45, 2.15, 2.65, &["Attack", "Decay", "Sustain", "Release", "Velocity"], Ctl::Knob)
        .screen(44.0, 8.6, 6.2, 2.4, "aenv")
        // The output.
        .section(50.6, 0.0, 4.4, 12.0, "Output")
        .knob(50.8, 0.3, 4.0, 5.4, "Volume")
        .ctl(50.8, 5.9, 4.0, 5.2, "Chorus", Ctl::Buttons)
}

/// Analog's face, its screens live. Returns whether a value changed.
pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut [f32], live: &InstLive, height: f32, mv: Option<&mut ModView>) -> bool {
    let face = face();
    let mut cx = Ctx { id, keys: analog::SPECS, specs: analog::SPECS, params, mv, presets: Some(Presets { current: preset, names: analog::preset_names(), load: analog::preset }) };
    let changed = draw(ui, &face, height, &mut cx, &mut |ui, which, rect, p| analog_screen(ui, id.with(which), rect, which, p, live));
    if live.vals[0] > 0.0 {
        ui.ctx().request_repaint();
    }
    changed
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_knob_of_analog_is_on_its_face() {
        super::super::check(&super::face(), krabton_engine::dsp::analog::SPECS);
    }
}
