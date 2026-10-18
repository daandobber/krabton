//! The organ laid out as the tonewheel console: nine drawbars to pull out
//! (brown for the sub-octaves, white for the octaves, black for the rest),
//! the four percussion tabs, the vibrato and chorus on a chicken head, and
//! the rotating speaker with its speed switch, seen from above.

use super::{draw, Ctl, Ctx, Face, Presets};
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::organ;
use std::f32::consts::TAU;

const BROWN: Color32 = Color32::from_rgb(0x7a, 0x43, 0x22);
const WHITE: Color32 = Color32::from_rgb(0xee, 0xea, 0xdf);
const BLACK: Color32 = Color32::from_rgb(0x1e, 0x1c, 0x1a);

pub(crate) fn face() -> Face {
    let bars = [("16'", BROWN), ("5 1/3'", BROWN), ("8'", WHITE), ("4'", WHITE), ("2 2/3'", BLACK), ("2'", WHITE), ("1 3/5'", BLACK), ("1 1/3'", BLACK), ("1'", WHITE)];
    let mut f = Face::new(38.0, 12.0)
        .section(0.0, 0.0, 7.2, 12.0, "Console")
        .preset(0.3, 0.4, 6.6, 1.4)
        .named(0.3, 2.0, 6.6, 5.4, "Vibrato", Ctl::Selector, "Vibrato & Chorus")
        .knob(0.3, 7.5, 3.3, 3.6, "Key Click")
        .knob(3.6, 7.5, 3.3, 3.6, "Drive")
        .section(7.4, 0.0, 16.6, 12.0, "Drawbars");
    for (k, (name, color)) in bars.into_iter().enumerate() {
        f = f.ctl(7.6 + k as f32 * 1.8, 0.3, 1.8, 10.8, name, Ctl::Drawbar(color));
    }
    f.section(24.2, 0.0, 6.6, 12.0, "Percussion")
        .named(24.4, 0.3, 3.1, 5.3, "Percussion", Ctl::Switch, "Perc")
        .named(27.5, 0.3, 3.1, 5.3, "Percussion:Volume", Ctl::Switch, "Volume")
        .named(24.4, 5.8, 3.1, 5.3, "Percussion:Decay", Ctl::Switch, "Decay")
        .named(27.5, 5.8, 3.1, 5.3, "Harmonic", Ctl::Switch, "Harmonic")
        .section(31.0, 0.0, 7.0, 12.0, "Rotary Speaker")
        .named(31.2, 0.3, 3.2, 5.3, "Rotary", Ctl::Switch, "Speed")
        .named(34.6, 0.3, 3.2, 5.3, "Output:Volume", Ctl::Knob, "Volume")
        .screen(31.3, 5.9, 6.4, 5.2, "rotor")
}

pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut [f32], live: &InstLive, height: f32, mv: Option<&mut ModView>) -> bool {
    let face = face();
    let mut cx = Ctx { id, keys: organ::SPECS, specs: organ::SPECS, params, mv, presets: Some(Presets { current: preset, names: organ::preset_names(), load: organ::preset }) };
    let changed = draw(ui, &face, height, &mut cx, &mut |ui, _, rect, p| rotor(ui, id.with("rotor"), rect, p, live));
    let speed = params[organ::ROTARY].round();
    if speed > 0.0 && live.level() > 1e-4 {
        ui.ctx().request_repaint();
    }
    changed
}

/// The rotating speaker from above: the horn and the drum turning at their
/// speeds. Click it for off, slow, fast.
fn rotor(ui: &mut Ui, id: Id, rect: Rect, p: &mut [f32], live: &InstLive) -> bool {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect.expand(1.5), 4.0, Color32::from_black_alpha(140));
    painter.rect_filled(rect, 4.0, crate::ui::inst_screen::SCREEN_BG);
    let resp = ui.interact(rect, id, Sense::click());
    let speed = p[organ::ROTARY].round().clamp(0.0, 2.0) as usize;
    let mut changed = false;
    if resp.clicked() {
        p[organ::ROTARY] = ((speed + 1) % 3) as f32;
        changed = true;
    }
    let c = pos2(rect.center().x, rect.center().y - 4.0);
    let rad = (rect.height() * 0.36).min(rect.width() * 0.3);
    let dim = Color32::from_rgb(0x4a, 0x4e, 0x58);
    painter.circle_stroke(c, rad, Stroke::new(1.5_f32, dim));
    let drum = live.vals[1] * TAU;
    let arc: Vec<egui::Pos2> = (0..=16).map(|i| c + vec2((drum - 0.7 + 1.4 * i as f32 / 16.0).cos(), (drum - 0.7 + 1.4 * i as f32 / 16.0).sin()) * rad * 0.62).collect();
    painter.add(Shape::line(arc, Stroke::new((rad * 0.14).max(3.0), accent_now().gamma_multiply(0.8))));
    let horn = live.vals[0] * TAU;
    let d = vec2(horn.cos(), horn.sin());
    let side = vec2(-d.y, d.x);
    let mouth = c + d * rad * 0.88;
    let white = Color32::from_rgb(0xf0, 0xf0, 0xf0);
    painter.line_segment([c - d * rad * 0.45, mouth], Stroke::new(2.0_f32, white));
    painter.add(Shape::convex_polygon(vec![c + d * rad * 0.55 + side * 2.0, mouth + side * rad * 0.18, mouth - side * rad * 0.18, c + d * rad * 0.55 - side * 2.0], white, Stroke::NONE));
    painter.circle_filled(c, 3.0, white);
    painter.text(pos2(c.x, rect.bottom() - 4.0), Align2::CENTER_BOTTOM, ["STOP", "CHORALE", "TREMOLO"][speed], FontId::monospace(10.0), if speed == 2 { accent_now() } else { dim });
    resp.on_hover_text("The rotating speaker. Click: stop, slow (chorale), fast (tremolo)");
    changed
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_control_of_the_organ_is_on_its_face() {
        super::super::check(&super::face(), krabton_engine::dsp::organ::SPECS);
    }
}
