//! The Mutable modules' faces, beside their shape pickers and screens:
//! Braids (timbre and colour big, named for the shape), Elements (the three
//! exciters side by side, the resonator's geometry big), Rings (its models on
//! lit buttons, the structure big) and Beads (the grains with their random
//! amounts).

use super::{draw, Ctl, Ctx, Face, Presets};
use crate::ui::modulation::ModView;
use egui::{Id, Ui};
use krabton_engine::dsp::{beads_synth, braids_synth, elements_synth, rings_synth};

pub(crate) fn braids() -> Face {
    Face::new(34.0, 12.0)
        .section(0.0, 0.0, 9.6, 12.0, "Oscillator")
        .preset(0.3, 0.4, 9.0, 1.4)
        .knob(0.3, 2.0, 4.6, 5.7, "Timbre")
        .knob(4.9, 2.0, 4.6, 5.7, "Color")
        .named(0.3, 8.0, 4.5, 3.1, "Bits", Ctl::Lcd, "Bits")
        .named(4.9, 8.0, 4.5, 3.1, "Rate", Ctl::Lcd, "Sample Rate")
        .screened(&["Shape"])
        .section(9.8, 0.0, 6.6, 12.0, "Envelope")
        .named(9.95, 0.3, 3.15, 5.3, "Env Attack", Ctl::Knob, "Attack")
        .named(13.1, 0.3, 3.15, 5.3, "Env Decay", Ctl::Knob, "Decay")
        .named(9.95, 5.8, 2.1, 5.3, "Timbre Env", Ctl::Knob, "Timbre")
        .named(12.05, 5.8, 2.1, 5.3, "Color Env", Ctl::Knob, "Color")
        .named(14.15, 5.8, 2.1, 5.3, "Pitch Env", Ctl::Knob, "Pitch")
        .section(16.6, 0.0, 8.6, 12.0, "Amp")
        .row(16.7, 0.3, 2.1, 5.3, &["Attack", "Decay", "Sustain", "Release"], Ctl::Knob)
        .knob(16.7, 5.8, 3.0, 5.3, "Velocity")
        .knob(19.7, 5.8, 5.4, 5.3, "Volume")
        .section(25.4, 0.0, 8.6, 12.0, "Voice")
        .row(25.5, 0.3, 2.8, 5.3, &["Voices", "Glide", "Transpose"], Ctl::Knob)
        .row(25.5, 5.8, 4.2, 5.3, &["Spread", "Drift"], Ctl::Knob)
}

pub(crate) fn elements() -> Face {
    Face::new(38.4, 12.0)
        .section(0.0, 0.0, 12.4, 12.0, "Exciter")
        .preset(0.3, 0.4, 11.8, 1.4)
        .knob(0.3, 2.0, 4.0, 3.7, "Bow")
        .named(0.3, 5.8, 4.0, 2.7, "Bow Timbre", Ctl::Knob, "Timbre")
        .knob(4.3, 2.0, 4.0, 3.7, "Blow")
        .knob(4.3, 5.8, 4.0, 2.7, "Flow")
        .named(4.3, 8.5, 4.0, 2.65, "Blow Timbre", Ctl::Knob, "Timbre")
        .knob(8.3, 2.0, 4.0, 3.7, "Strike")
        .knob(8.3, 5.8, 4.0, 2.7, "Mallet")
        .named(8.3, 8.5, 4.0, 2.65, "Strike Timbre", Ctl::Knob, "Timbre")
        .section(12.6, 0.0, 4.6, 12.0, "Contour")
        .knob(12.7, 0.3, 4.4, 5.4, "Contour")
        .knob(12.7, 5.8, 2.2, 5.3, "Velocity")
        .knob(14.9, 5.8, 2.2, 5.3, "Signature")
        .section(17.4, 0.0, 11.8, 12.0, "Resonator")
        .knob(17.6, 0.3, 5.0, 6.6, "Geometry")
        .knob(17.6, 7.0, 5.0, 4.1, "Position")
        .knob(22.6, 0.3, 3.2, 5.4, "Brightness")
        .knob(25.8, 0.3, 3.2, 5.4, "Damping")
        .screened(&["Model"])
        .section(29.4, 0.0, 5.2, 12.0, "Space")
        .knob(29.6, 0.3, 4.8, 5.4, "Space")
        .knob(29.6, 5.9, 2.4, 5.2, "Shimmer")
        .ctl(32.0, 5.9, 2.4, 5.2, "Freeze", Ctl::Switch)
        .section(34.8, 0.0, 3.6, 12.0, "Voice")
        .col(34.9, 0.3, 3.4, 3.6, &["Voices", "Transpose", "Volume"], Ctl::Knob)
}

pub(crate) fn rings() -> Face {
    Face::new(27.4, 12.0)
        .section(0.0, 0.0, 18.0, 12.0, "Resonator")
        .preset(0.3, 0.4, 4.4, 1.4)
        .named(0.3, 2.0, 4.4, 9.1, "Model", Ctl::Buttons, "Model")
        .knob(4.9, 0.3, 5.4, 10.8, "Structure")
        .knob(10.3, 0.3, 3.8, 5.4, "Brightness")
        .knob(14.1, 0.3, 3.7, 5.4, "Damping")
        .knob(10.3, 5.9, 3.8, 5.2, "Position")
        .named(14.1, 5.9, 3.7, 5.2, "Voices", Ctl::Switch, "Polyphony")
        .section(18.2, 0.0, 4.6, 12.0, "String Synth")
        .named(18.4, 0.3, 4.2, 5.3, "Peace Mode", Ctl::Switch, "Peace")
        .named(18.4, 5.9, 4.2, 5.2, "Peace FX", Ctl::Lcd, "Effect")
        .section(23.0, 0.0, 4.4, 12.0, "Output")
        .knob(23.2, 0.3, 4.0, 5.3, "Width")
        .knob(23.2, 5.8, 4.0, 5.3, "Volume")
}

pub(crate) fn beads() -> Face {
    Face::new(36.6, 12.0)
        .section(0.0, 0.0, 8.8, 12.0, "Wave")
        .preset(0.3, 0.4, 8.2, 1.4)
        .named(0.3, 2.0, 8.2, 2.6, "Bank", Ctl::Lcd, "Bank")
        .knob(0.3, 4.8, 4.6, 6.3, "Wave")
        .knob(4.9, 4.8, 3.6, 6.3, "Transpose")
        .section(9.0, 0.0, 12.6, 12.0, "Grains")
        .row(9.1, 0.3, 4.13, 6.0, &["Size", "Shape", "Density"], Ctl::Knob)
        .named(9.1, 6.6, 3.1, 4.5, "Wave Random", Ctl::Knob, "Wave ±")
        .named(12.2, 6.6, 3.1, 4.5, "Size Random", Ctl::Knob, "Size ±")
        .named(15.3, 6.6, 3.1, 4.5, "Shape Random", Ctl::Knob, "Shape ±")
        .named(18.4, 6.6, 3.1, 4.5, "Pitch Random", Ctl::Knob, "Pitch ±")
        .section(21.8, 0.0, 6.0, 12.0, "Mix")
        .knob(21.95, 0.3, 2.9, 5.3, "Grains")
        .knob(24.85, 0.3, 2.9, 5.3, "Reverb")
        .ctl(21.95, 5.8, 5.8, 5.3, "Freeze", Ctl::Switch)
        .section(28.0, 0.0, 8.6, 12.0, "Seed & Sound")
        .named(28.2, 0.3, 4.1, 3.0, "Seed", Ctl::Lcd, "Seed")
        .named(32.3, 0.3, 4.1, 3.0, "Sync", Ctl::Lcd, "Sync")
        .named(28.2, 3.5, 8.2, 2.4, "Quality", Ctl::Buttons, "Quality")
        .row(28.2, 6.1, 2.73, 5.0, &["Attack", "Release", "Volume"], Ctl::Knob)
}

/// Braids' face, its timbre and colour named for the shape.
pub(crate) fn braids_panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut [f32], height: f32, mv: Option<&mut ModView>) -> bool {
    let shape = (params[braids_synth::SHAPE].round().max(0.0) as usize).min(braids_synth::SHAPE_COUNT - 1);
    let mut specs = braids_synth::SPECS.to_vec();
    specs[braids_synth::TIMBRE].name = braids_synth::SHAPE_KNOBS[shape][0];
    specs[braids_synth::COLOR].name = braids_synth::SHAPE_KNOBS[shape][1];
    let mut cx = Ctx { id, keys: braids_synth::SPECS, specs: &specs, params, mv, presets: Some(Presets { current: preset, names: braids_synth::preset_names(), load: braids_synth::preset }) };
    draw(ui, &braids(), height, &mut cx, &mut |_, _, _, _| false)
}

/// Elements' face, its geometry named for the model.
pub(crate) fn elements_panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut [f32], height: f32, mv: Option<&mut ModView>) -> bool {
    let mut specs = elements_synth::SPECS.to_vec();
    specs[elements_synth::GEOMETRY].name = elements_synth::geometry_name(params[elements_synth::MODEL]);
    let mut cx = Ctx { id, keys: elements_synth::SPECS, specs: &specs, params, mv, presets: Some(Presets { current: preset, names: elements_synth::preset_names(), load: elements_synth::preset }) };
    draw(ui, &elements(), height, &mut cx, &mut |_, _, _, _| false)
}

pub(crate) fn rings_panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut [f32], height: f32, mv: Option<&mut ModView>) -> bool {
    let mut cx = Ctx { id, keys: rings_synth::SPECS, specs: rings_synth::SPECS, params, mv, presets: Some(Presets { current: preset, names: rings_synth::preset_names(), load: rings_synth::preset }) };
    draw(ui, &rings(), height, &mut cx, &mut |_, _, _, _| false)
}

pub(crate) fn beads_panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut [f32], height: f32, mv: Option<&mut ModView>) -> bool {
    let mut cx = Ctx { id, keys: beads_synth::SPECS, specs: beads_synth::SPECS, params, mv, presets: Some(Presets { current: preset, names: beads_synth::preset_names(), load: beads_synth::preset }) };
    draw(ui, &beads(), height, &mut cx, &mut |_, _, _, _| false)
}

#[cfg(test)]
mod tests {
    use super::super::check;

    #[test]
    fn every_knob_of_the_modules_is_on_their_faces() {
        check(&super::braids(), krabton_engine::dsp::braids_synth::SPECS);
        check(&super::elements(), krabton_engine::dsp::elements_synth::SPECS);
        check(&super::rings(), krabton_engine::dsp::rings_synth::SPECS);
        check(&super::beads(), krabton_engine::dsp::beads_synth::SPECS);
    }
}
