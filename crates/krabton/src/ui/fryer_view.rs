//! The Fryer's face, in Analog Princess's colours as its FRY section: Deep
//! Fry big, the 808s the sound goes through, the ring and the crusher, the
//! clip into the ceiling, the subliminal shadow, and the time effects with
//! their buttons to hold (Stutter, Tape Stop, Surf: on while held down,
//! right-click to leave one on).

use crate::theme::*;
use crate::ui::faces::{draw, Ctl, Ctx, Face};
use crate::ui::modulation::ModView;
use egui::{vec2, Align2, Color32, FontId, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use krabton_engine::dsp::fx::fryer as fr;

pub(crate) fn face() -> Face {
    Face::new(56.0, 12.0)
        .screened(&["Stutter Hold", "Tape Hold", "Surf Hold"])
        .section(0.0, 0.0, 7.4, 12.0, "Fry")
        .knob(0.2, 0.3, 7.0, 6.8, "Deep Fry")
        .knob(0.2, 7.3, 3.5, 3.8, "Mix")
        .knob(3.7, 7.3, 3.5, 3.8, "Output")
        .section(7.6, 0.0, 9.2, 12.0, "808 Into 808")
        .knob(7.8, 0.3, 4.4, 5.6, "Heat")
        .named(12.3, 0.3, 4.3, 5.6, "808s", Ctl::Buttons, "808s")
        .named(7.8, 6.2, 8.8, 4.9, "Distortion", Ctl::Buttons, "Distortion")
        .section(17.0, 0.0, 10.2, 12.0, "Ring & Crush")
        .knob(17.2, 0.3, 3.3, 5.0, "Ring")
        .knob(17.2, 5.6, 3.3, 5.5, "Ring Hz")
        .named(20.5, 0.3, 3.3, 10.8, "Ring Sync", Ctl::Buttons, "Sync")
        .knob(23.8, 0.3, 3.2, 5.0, "Bits")
        .named(23.8, 5.6, 3.2, 5.5, "Downsample", Ctl::Knob, "Down")
        .section(27.4, 0.0, 6.4, 12.0, "Clip")
        .knob(27.6, 0.3, 3.0, 5.4, "Clip")
        .knob(30.6, 0.3, 3.0, 5.4, "Ceiling")
        .named(27.6, 6.0, 6.0, 5.1, "Clip Mode", Ctl::Switch, "Mode")
        .section(34.0, 0.0, 7.6, 12.0, "Subliminal")
        .named(34.2, 0.3, 3.6, 5.4, "Subliminal", Ctl::Knob, "Shadow")
        .named(37.8, 0.3, 3.6, 5.4, "Sub Tone", Ctl::Knob, "Tone")
        .named(34.2, 6.0, 3.6, 5.1, "Sub Pitch", Ctl::Buttons, "Down")
        .named(37.8, 6.0, 3.6, 5.1, "Sub Reverse", Ctl::Switch, "Reverse")
        .section(41.8, 0.0, 14.2, 12.0, "Time")
        .knob(42.0, 0.3, 2.9, 4.2, "Stutter")
        .named(44.9, 0.3, 2.9, 4.2, "Stutter Rate", Ctl::Buttons, "Rate")
        .knob(47.8, 0.3, 2.9, 4.2, "Tape Stop")
        .named(50.7, 0.3, 2.6, 4.2, "Tape Time", Ctl::Buttons, "Time")
        .named(53.3, 0.3, 2.6, 4.2, "Channel Surf", Ctl::Knob, "Surf")
        .screen(42.0, 4.8, 13.9, 6.3, "holds")
}

/// The buttons' colours without Analog Princess's look.
const PADS: [Color32; 4] = [Color32::from_rgb(0xff, 0x4f, 0xb0), Color32::from_rgb(0xb0, 0x4f, 0xff), Color32::from_rgb(0x4f, 0xe0, 0xff), Color32::from_rgb(0xff, 0xe1, 0x4f)];

/// What the face remembers: the hold buttons left on.
#[derive(Clone, Copy, Default)]
struct Latched([bool; 3]);

/// The Fryer's panel. Returns whether a parameter changed.
pub(crate) fn fryer_panel(ui: &mut Ui, id: Id, p: &mut [f32], h: f32, mv: Option<&mut ModView>) -> bool {
    if p.len() < fr::SPECS.len() {
        return false;
    }
    let face = face();
    let lid = id.with("latched");
    let mut latched: Latched = ui.ctx().data(|d| d.get_temp(lid)).unwrap_or_default();
    let mut cx = Ctx { id, keys: fr::SPECS, specs: fr::SPECS, params: p, mv, presets: None };
    let changed = draw(ui, &face, (h - 24.0).max(100.0), &mut cx, &mut |ui, which, rect, p| if which == "holds" { holds(ui, id.with("holds"), rect, p, &mut latched) } else { false });
    ui.ctx().data_mut(|d| d.insert_temp(lid, latched));
    changed
}

/// Three buttons to hold down: the time effects while they are.
fn holds(ui: &mut Ui, id: Id, r: Rect, p: &mut [f32], latched: &mut Latched) -> bool {
    let painter = ui.painter().clone();
    let pads = crate::skin::current().map(|s| s.pads).filter(|p| p.len() >= 4).unwrap_or(&PADS);
    let (well, well_ink) = crate::skin::well();
    let w = r.width() / 3.0;
    let mut changed = false;
    for (k, (pi, label)) in [(fr::STUTTER_HOLD, "Stutter"), (fr::TAPE_HOLD, "Tape Stop"), (fr::SURF_HOLD, "Surf")].into_iter().enumerate() {
        let b = Rect::from_min_size(r.min + vec2(k as f32 * w, 0.0), vec2(w, r.height())).shrink(4.0);
        let resp = ui.interact(b, id.with(k), Sense::click_and_drag());
        if resp.secondary_clicked() {
            latched.0[k] = !latched.0[k];
        }
        let down = resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down());
        let want = if down || latched.0[k] { 1.0 } else { 0.0 };
        if p[pi] != want {
            p[pi] = want;
            changed = true;
        }
        let c = pads[k % pads.len()];
        let active = want > 0.5;
        painter.rect_filled(b.translate(vec2(0.0, 2.5)), 6.0, Color32::from_black_alpha(130));
        painter.rect_filled(b, 6.0, if active { c } else if resp.hovered() { mix(well, c, 0.25) } else { mix(well, c, 0.12) });
        if latched.0[k] {
            painter.rect_stroke(b.expand(1.5), 6.0, Stroke::new(2.0_f32, Color32::WHITE), StrokeKind::Outside);
        }
        let text = crate::skin::label_text(label).to_uppercase();
        let f = crate::ui::faces::fit(&painter, &text, FontId::proportional((b.height() * 0.22).clamp(10.0, 18.0)), b.width() - 6.0);
        painter.text(b.center(), Align2::CENTER_CENTER, text, f, if active { text_on(c) } else { well_ink });
        resp.on_hover_text(format!("{label}: hold it down, or right-click to leave it on"));
    }
    changed
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_face_holds_every_parameter() {
        crate::ui::faces::check(&super::face(), krabton_engine::dsp::fx::fryer::SPECS);
    }
}
