//! The grand piano's panel beside its screen: touch, the lid and the
//! pedal, its noises, the room it stands in and its tuning.

use super::{draw, Ctl, Ctx, Face, Presets};
use crate::ui::modulation::ModView;
use egui::{Id, Ui};
use krabton_engine::piano;

pub(crate) fn face() -> Face {
    Face::new(32.4, 12.0)
        .section(0.0, 0.0, 6.0, 12.0, "Grand")
        .preset(0.3, 0.4, 5.4, 1.4)
        .named(0.3, 2.0, 5.4, 5.6, "Dynamics", Ctl::Knob, "Touch")
        .knob(0.3, 7.8, 2.7, 3.3, "Una Corda")
        .knob(3.0, 7.8, 2.7, 3.3, "Dampers")
        .section(6.2, 0.0, 6.4, 12.0, "Lid & Pedal")
        .knob(6.4, 0.3, 6.0, 5.6, "Lid")
        .named(6.4, 6.0, 2.9, 5.1, "Sustain", Ctl::Switch, "Pedal")
        .knob(9.4, 6.0, 3.0, 5.1, "Tone")
        .section(12.8, 0.0, 6.2, 12.0, "Noises")
        .knob(12.9, 0.3, 3.0, 5.3, "Key Noise")
        .knob(15.9, 0.3, 3.0, 5.3, "Resonance")
        .knob(12.9, 5.8, 3.0, 5.3, "Pedal Noise")
        .knob(15.9, 5.8, 3.0, 5.3, "Action")
        .section(19.2, 0.0, 6.4, 12.0, "Room")
        .knob(19.3, 0.3, 3.1, 5.3, "Room")
        .knob(22.4, 0.3, 3.1, 5.3, "Room Size")
        .knob(19.3, 5.8, 3.1, 5.3, "Width")
        .named(22.4, 5.8, 3.1, 5.3, "Side", Ctl::Switch, "Heard from")
        .section(25.8, 0.0, 6.6, 12.0, "Tuning")
        .ctl(25.9, 0.3, 3.2, 5.3, "Tuning", Ctl::Switch)
        .knob(29.1, 0.3, 3.2, 5.3, "Fine")
        .knob(25.9, 5.8, 3.2, 5.3, "Transpose")
        .knob(29.1, 5.8, 3.2, 5.3, "Volume")
}

pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut [f32], height: f32, mv: Option<&mut ModView>) -> bool {
    let face = face();
    let mut cx = Ctx { id, keys: piano::SPECS, specs: piano::SPECS, params, mv, presets: Some(Presets { current: preset, names: piano::preset_names(), load: piano::preset }) };
    draw(ui, &face, height, &mut cx, &mut |_, _, _, _| false)
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_knob_of_the_piano_is_on_its_face() {
        super::super::check(&super::face(), krabton_engine::piano::SPECS);
    }
}
