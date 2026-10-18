//! Effects laid out as the boxes they come from: the tape echo with its
//! repeat rate and intensity, a delay with its time in a display, a room
//! with its size big, a compressor's threshold and output as on a studio
//! unit, the chorus, the saturator, the three-band EQ, the
//! filter with its big cutoff and the crusher.

use super::{draw, Ctl, Ctx, Face};
use crate::ui::modulation::ModView;
use egui::{Id, Ui};
use krabton_engine::dsp::effects::EffectKind;

/// The face of an effect, if it has one.
pub(crate) fn face(kind: EffectKind) -> Option<Face> {
    use EffectKind::*;
    Some(match kind {
        Echo => Face::new(28.0, 12.0)
            .section(0.0, 0.0, 10.0, 12.0, "Repeat")
            .named(0.2, 0.3, 4.9, 6.4, "Time", Ctl::Knob, "Repeat Rate")
            .named(5.0, 0.3, 4.8, 6.4, "Feedback", Ctl::Knob, "Intensity")
            .named(0.3, 7.0, 9.4, 4.1, "Sync", Ctl::Lcd, "Rate")
            .section(10.2, 0.0, 9.6, 12.0, "Tape")
            .row(10.35, 0.3, 3.1, 5.4, &["Wow", "Flutter", "Saturation"], Ctl::Knob)
            .named(10.35, 5.8, 4.65, 5.3, "Tone", Ctl::Knob, "Treble")
            .named(15.0, 5.8, 4.65, 5.3, "Low Cut", Ctl::Knob, "Bass")
            .section(20.0, 0.0, 8.0, 12.0, "Output")
            .named(20.2, 0.3, 7.6, 6.2, "Mix", Ctl::Knob, "Echo Volume")
            .knob(20.2, 6.6, 7.6, 4.5, "Spread"),
        Delay => Face::new(22.0, 12.0)
            .section(0.0, 0.0, 10.0, 12.0, "Time")
            .named(0.3, 0.3, 9.4, 3.2, "Time", Ctl::Lcd, "Delay Time")
            .knob(0.3, 3.7, 6.0, 7.4, "Feedback")
            .named(6.4, 3.7, 3.3, 7.4, "Ping Pong", Ctl::Switch, "Ping Pong")
            .section(10.2, 0.0, 6.0, 12.0, "Filter")
            .knob(10.4, 0.3, 5.6, 5.3, "Low Cut")
            .knob(10.4, 5.8, 5.6, 5.3, "High Cut")
            .section(16.4, 0.0, 5.6, 12.0, "Output")
            .knob(16.6, 0.3, 5.2, 10.8, "Mix"),
        Reverb => Face::new(22.4, 12.0)
            .section(0.0, 0.0, 12.0, 12.0, "Room")
            .knob(0.2, 0.3, 6.2, 7.2, "Size")
            .knob(6.4, 0.3, 5.4, 3.6, "Damping")
            .knob(6.4, 3.9, 5.4, 3.6, "Width")
            .section(12.2, 0.0, 5.2, 12.0, "Input")
            .knob(12.4, 0.3, 4.8, 5.3, "Predelay")
            .knob(12.4, 5.8, 4.8, 5.3, "Low Cut")
            .section(17.6, 0.0, 4.8, 12.0, "Output")
            .knob(17.8, 0.3, 4.4, 10.8, "Mix"),
        Compressor => Face::new(22.4, 12.0)
            .section(0.0, 0.0, 14.0, 12.0, "Dynamics")
            .knob(0.2, 0.3, 6.8, 7.2, "Threshold")
            .knob(7.0, 0.3, 6.8, 7.2, "Ratio")
            .knob(0.3, 7.6, 6.6, 3.5, "Attack")
            .knob(7.1, 7.6, 6.6, 3.5, "Release")
            .section(14.2, 0.0, 8.2, 12.0, "Output")
            .named(14.4, 0.3, 7.8, 7.2, "Makeup", Ctl::Knob, "Output")
            .knob(14.4, 7.6, 7.8, 3.5, "Mix"),
        Chorus => Face::new(22.4, 12.0)
            .section(0.0, 0.0, 16.0, 12.0, "Ensemble")
            .knob(0.2, 0.3, 7.8, 6.4, "Rate")
            .knob(8.0, 0.3, 7.8, 6.4, "Depth")
            .knob(0.3, 7.0, 7.6, 4.1, "Delay")
            .knob(8.1, 7.0, 7.6, 4.1, "Feedback")
            .section(16.2, 0.0, 6.2, 12.0, "Output")
            .knob(16.4, 0.3, 5.8, 10.8, "Mix"),
        Saturator => Face::new(22.4, 12.0)
            .section(0.0, 0.0, 13.6, 12.0, "Shape")
            .knob(0.2, 0.3, 7.0, 7.4, "Drive")
            .named(7.3, 0.3, 6.1, 7.4, "Type", Ctl::Buttons, "Curve")
            .knob(0.3, 7.9, 13.0, 3.2, "Color")
            .section(13.8, 0.0, 8.6, 12.0, "Output")
            .knob(14.0, 0.3, 8.2, 5.4, "Output")
            .knob(14.0, 5.9, 8.2, 5.2, "Mix"),
        Eq3 => Face::new(15.0, 12.0)
            .section(0.0, 0.0, 15.0, 12.0, "Three Band")
            .row(0.2, 0.3, 4.87, 7.0, &["Low", "Mid", "High"], Ctl::Knob)
            .named(0.3, 7.5, 7.2, 3.6, "Freq Lo", Ctl::Knob, "Low · Mid")
            .named(7.5, 7.5, 7.2, 3.6, "Freq Hi", Ctl::Knob, "Mid · High"),
        AutoFilter => Face::new(28.0, 12.0)
            .section(0.0, 0.0, 17.0, 12.0, "Filter")
            .named(0.2, 0.3, 4.0, 10.8, "Type", Ctl::Buttons, "Mode")
            .knob(4.3, 0.2, 7.0, 7.4, "Cutoff")
            .knob(11.3, 0.4, 5.5, 5.4, "Resonance")
            .knob(11.3, 6.0, 5.5, 5.1, "Drive")
            .section(17.2, 0.0, 6.0, 12.0, "LFO")
            .named(17.4, 0.3, 5.6, 5.3, "LFO Amt", Ctl::Knob, "Amount")
            .named(17.4, 5.8, 5.6, 5.3, "LFO Rate", Ctl::Lcd, "Rate")
            .section(23.4, 0.0, 4.6, 12.0, "Output")
            .knob(23.6, 0.3, 4.2, 10.8, "Mix"),
        Redux => Face::new(18.0, 12.0)
            .section(0.0, 0.0, 12.4, 12.0, "Crush")
            .knob(0.2, 0.3, 6.0, 10.8, "Bits")
            .knob(6.2, 0.3, 6.0, 10.8, "Downsample")
            .section(12.6, 0.0, 5.4, 12.0, "Output")
            .knob(12.8, 0.3, 5.0, 10.8, "Mix"),
        _ => return None,
    })
}

/// An effect's face. Returns whether a value changed.
pub(crate) fn panel(ui: &mut Ui, id: Id, kind: EffectKind, face: &Face, params: &mut [f32], height: f32, mv: Option<&mut ModView>) -> bool {
    let specs = kind.specs();
    let mut cx = Ctx { id, keys: specs, specs, params, mv, presets: None };
    draw(ui, face, height, &mut cx, &mut |_, _, _, _| false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_knob_of_an_effect_with_a_face_is_on_it() {
        let mut n = 0;
        for kind in EffectKind::ALL {
            if let Some(f) = face(kind) {
                super::super::check(&f, kind.specs());
                n += 1;
            }
        }
        assert!(n >= 9);
    }
}
