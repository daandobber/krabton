//! Rackverb's face: a black one-unit rack with its pinstripes, three big knobs,
//! two lights, the program in red digits and two square buttons. Up and down
//! step through the programs as on the machine, the two lists choose directly.

use crate::ui::modulation::{knob_overlay, ModView};
use crate::widgets::knob_with_speed;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Painter, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder};
use krabton_engine::dsp::fx::rackverb as rv;
use krabton_engine::engine::EngineStatus;
use krabton_rackverb::MODELS;

const BLACK: Color32 = Color32::from_rgb(0x17, 0x18, 0x1c);
const STRIPE: Color32 = Color32::from_rgb(0x4f, 0x8f, 0xc8);
const INK: Color32 = Color32::from_rgb(0xe4, 0xe6, 0xea);
const DIM: Color32 = Color32::from_rgb(0x9a, 0x9e, 0xa6);
const RED: Color32 = Color32::from_rgb(0xff, 0x2a, 0x22);
const RED_OFF: Color32 = Color32::from_rgb(0x38, 0x0c, 0x0c);
const GREEN: Color32 = Color32::from_rgb(0x46, 0xe0, 0x6a);
const WIDTH: f32 = 520.0;

/// Which of the seven segments a digit lights: top, upper right, lower right,
/// bottom, lower left, upper left, middle.
fn segments(d: u8) -> [bool; 7] {
    match d {
        0 => [true, true, true, true, true, true, false],
        1 => [false, true, true, false, false, false, false],
        2 => [true, true, false, true, true, false, true],
        3 => [true, true, true, true, false, false, true],
        4 => [false, true, true, false, false, true, true],
        5 => [true, false, true, true, false, true, true],
        6 => [true, false, true, true, true, true, true],
        7 => [true, true, true, false, false, false, false],
        8 => [true; 7],
        _ => [true, true, true, true, false, true, true],
    }
}

/// A red seven-segment digit, the unlit segments faintly there.
fn digit(p: &Painter, r: Rect, d: u8) {
    let t = (r.width() * 0.16).max(2.0);
    let (x0, x1) = (r.left(), r.right());
    let (y0, y1, y2) = (r.top(), r.center().y, r.bottom());
    let across = |x0: f32, x1: f32, y: f32| vec![pos2(x0 + t * 0.5, y), pos2(x0 + t, y - t * 0.5), pos2(x1 - t, y - t * 0.5), pos2(x1 - t * 0.5, y), pos2(x1 - t, y + t * 0.5), pos2(x0 + t, y + t * 0.5)];
    let down = |x: f32, y0: f32, y1: f32| vec![pos2(x, y0 + t * 0.5), pos2(x + t * 0.5, y0 + t), pos2(x + t * 0.5, y1 - t), pos2(x, y1 - t * 0.5), pos2(x - t * 0.5, y1 - t), pos2(x - t * 0.5, y0 + t)];
    let shapes: [Vec<Pos2>; 7] = [across(x0, x1, y0), down(x1, y0, y1), down(x1, y1, y2), across(x0, x1, y2), down(x0, y1, y2), down(x0, y0, y1), across(x0, x1, y1)];
    let lit = segments(d);
    for (pts, on) in shapes.into_iter().zip(lit) {
        p.add(Shape::convex_polygon(pts, if on { RED } else { RED_OFF }, Stroke::NONE));
    }
}

/// A hardware knob: a ring of ticks, a dark cap, a white line.
fn cap(p: &Painter, c: Pos2, r: f32, norm: f32) {
    for k in 0..=10 {
        let a = (-135.0 + 27.0 * k as f32).to_radians();
        let dir = vec2(a.sin(), -a.cos());
        p.line_segment([c + dir * (r + 2.5), c + dir * (r + 7.0)], Stroke::new(1.2_f32, INK));
    }
    p.circle_filled(c, r, Color32::from_rgb(0x0b, 0x0b, 0x0d));
    // The fluted edge.
    for k in 0..24 {
        let a = (k as f32 * 15.0).to_radians();
        let dir = vec2(a.sin(), -a.cos());
        p.line_segment([c + dir * (r * 0.8), c + dir * r], Stroke::new(1.0_f32, Color32::from_gray(0x3a)));
    }
    p.circle_filled(c, r * 0.72, Color32::from_rgb(0x1d, 0x1d, 0x21));
    let a = (-135.0 + 270.0 * norm.clamp(0.0, 1.0)).to_radians();
    let dir = vec2(a.sin(), -a.cos());
    p.line_segment([c + dir * (r * 0.25), c + dir * (r * 0.95)], Stroke::new(2.2_f32, Color32::WHITE));
}

/// A light.
fn light(p: &Painter, c: Pos2, on: bool, lit: Color32) {
    if on {
        p.circle_filled(c, 7.0, lit.gamma_multiply(0.25));
    }
    p.circle_filled(c, 4.0, if on { lit } else { lit.gamma_multiply(0.22) });
    p.circle_stroke(c, 4.0, Stroke::new(1.0_f32, Color32::from_gray(0x40)));
}

/// One step through a model's programs, round and round.
pub(crate) fn step(program: usize, by: i32, count: usize) -> usize {
    (program as i32 + by).rem_euclid(count.max(1) as i32) as usize
}

/// The number the display shows for a program.
pub(crate) fn shown(model: usize, program: usize) -> usize {
    program + MODELS[model.min(MODELS.len() - 1)].first
}

/// Rackverb's panel. Returns whether a parameter changed.
pub(crate) fn rackverb_panel(ui: &mut Ui, id: Id, dev_id: u64, p: &mut Vec<f32>, h: f32, mut mv: Option<&mut ModView>, status: &EngineStatus) -> bool {
    if p.len() < rv::SPECS.len() {
        return false;
    }
    let height = (h - 24.0).clamp(120.0, 190.0);
    let (rect, _) = ui.allocate_exact_size(vec2(WIDTH, height), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, BLACK);
    painter.rect_stroke(rect, 3.0, Stroke::new(1.0_f32, Color32::from_gray(0x33)), StrokeKind::Inside);
    let model = (p[rv::MODEL].round().max(0.0) as usize).min(MODELS.len() - 1);
    let count = MODELS[model].programs.len();
    let mut program = (p[rv::PROGRAM].round().max(0.0) as usize).min(count - 1);
    let mut changed = false;

    // The logo over its pinstripes.
    let line_y = rect.top() + height * 0.2;
    painter.text(pos2(rect.left() + 12.0, line_y - 1.0), Align2::LEFT_BOTTOM, "RACKVERB", FontId::proportional(height * 0.15), INK);
    for k in 0..3 {
        let y = line_y + 2.0 + k as f32 * 3.0;
        painter.line_segment([pos2(rect.left() + 2.0, y), pos2(rect.right() - 2.0, y)], Stroke::new(1.0_f32, STRIPE));
    }

    // Three knobs, as big as they fit.
    let row_top = line_y + 14.0;
    let bottom_row = 26.0;
    let row_h = rect.bottom() - row_top - bottom_row;
    let knob_r = (row_h * 0.3).clamp(10.0, 24.0);
    for (k, (param, label)) in [(rv::INPUT, "INPUT"), (rv::MIX, "MIX"), (rv::OUTPUT, "OUTPUT")].into_iter().enumerate() {
        let cell = Rect::from_min_size(pos2(rect.left() + 8.0 + k as f32 * 78.0, row_top), vec2(76.0, row_h));
        painter.text(pos2(cell.center().x, cell.top()), Align2::CENTER_TOP, label, FontId::proportional(10.5), INK);
        let center = pos2(cell.center().x, cell.top() + 14.0 + (row_h - 14.0) * 0.5);
        let k_rect = Rect::from_center_size(center, vec2(knob_r * 2.0, knob_r * 2.0));
        let spec = &rv::SPECS[param];
        let (resp, new) = knob_with_speed(ui, k_rect, id.with(("knob", param)), spec.to_norm(p[param]), spec.to_norm(spec.default), false, Color32::WHITE, 1.0);
        resp.on_hover_text(format!("{}: {}\nDrag to change, Shift for fine, double-click to reset", spec.name, spec.format(p[param])));
        if let Some(n) = new {
            let v = spec.from_norm(n);
            if v != p[param] {
                p[param] = v;
                changed = true;
            }
        }
        cap(ui.painter(), center, knob_r, spec.to_norm(p[param]));
        if let Some(mv) = mv.as_deref_mut() {
            knob_overlay(ui, cell, k_rect, id.with(("mod", param)), spec, p[param], param, mv);
        }
    }

    // The lights: how hard it is driven.
    let heard = status.meters.read(dev_id).map_or(0.0, |m| m[rv::M_INPUT]);
    let lights_x = rect.left() + 256.0;
    let mid_y = row_top + row_h * 0.5 + 6.0;
    let (over, signal) = (pos2(lights_x, mid_y - 14.0), pos2(lights_x, mid_y + 14.0));
    painter.text(over + vec2(-8.0, 0.0), Align2::RIGHT_CENTER, "0", FontId::proportional(10.0), DIM);
    painter.text(signal + vec2(-8.0, 0.0), Align2::RIGHT_CENTER, "-12", FontId::proportional(10.0), DIM);
    light(&painter, over, heard >= 0.9, RED);
    light(&painter, signal, heard >= 0.25, GREEN);

    // The program, in red digits.
    let win = Rect::from_center_size(pos2(rect.left() + 318.0, mid_y - 6.0), vec2(78.0, (row_h - 20.0).clamp(30.0, 58.0)));
    painter.rect_filled(win, 2.0, Color32::from_rgb(0x12, 0x03, 0x03));
    painter.rect_stroke(win, 2.0, Stroke::new(1.0_f32, Color32::from_gray(0x2c)), StrokeKind::Inside);
    let number = shown(model, program);
    let dh = win.height() - 14.0;
    let dw = dh * 0.55;
    for (k, d) in [(number / 10 % 10) as u8, (number % 10) as u8].into_iter().enumerate() {
        let x = win.center().x + if k == 0 { -dw - 6.0 } else { 6.0 };
        digit(&painter, Rect::from_min_size(pos2(x, win.center().y - dh * 0.5), vec2(dw, dh)), d);
    }
    painter.text(pos2(win.center().x, win.bottom() + 3.0), Align2::CENTER_TOP, "PROGRAM", FontId::proportional(9.5), DIM);

    // Down and up.
    for (k, (label, by)) in [("DOWN", -1), ("UP", 1)].into_iter().enumerate() {
        let b = Rect::from_center_size(pos2(rect.left() + 400.0 + k as f32 * 56.0, mid_y - 6.0), vec2(44.0, 36.0));
        let resp = ui.interact(b, id.with(("step", k)), Sense::click());
        let down = resp.is_pointer_button_down_on();
        painter.rect_filled(b, 2.0, if down { Color32::from_gray(0x16) } else { Color32::from_gray(0x2a) });
        painter.rect_stroke(b, 2.0, Stroke::new(1.0_f32, Color32::from_gray(if down { 0x22 } else { 0x4c })), StrokeKind::Inside);
        painter.text(pos2(b.center().x, b.bottom() + 3.0), Align2::CENTER_TOP, label, FontId::proportional(9.5), DIM);
        if resp.clicked() {
            program = step(program, by, count);
            p[rv::PROGRAM] = program as f32;
            changed = true;
        }
    }

    // The two lists and how much goes round again.
    let row = Rect::from_min_max(pos2(rect.left() + 8.0, rect.bottom() - bottom_row + 2.0), pos2(rect.right() - 8.0, rect.bottom() - 2.0));
    let mut sub = ui.new_child(UiBuilder::new().max_rect(row));
    sub.horizontal(|ui| {
        let mut chosen = model;
        egui::ComboBox::from_id_salt(id.with("model")).width(66.0).selected_text(MODELS[model].name).show_ui(ui, |ui| {
            for (k, m) in MODELS.iter().enumerate() {
                ui.selectable_value(&mut chosen, k, m.name);
            }
        });
        if chosen != model {
            p[rv::MODEL] = chosen as f32;
            p[rv::PROGRAM] = program.min(MODELS[chosen].programs.len() - 1) as f32;
            changed = true;
        }
        let mut pick = program;
        let now = format!("{}  {}", shown(model, program), MODELS[model].programs[program].name);
        egui::ComboBox::from_id_salt(id.with("program")).width(236.0).height(320.0).selected_text(now).show_ui(ui, |ui| {
            for (k, prog) in MODELS[model].programs.iter().enumerate() {
                ui.selectable_value(&mut pick, k, format!("{}  {}", shown(model, k), prog.name));
            }
        });
        if pick != program {
            p[rv::PROGRAM] = pick as f32;
            changed = true;
        }
        ui.label(RichText::new("Feedback").small().color(DIM));
        let mut fb = p[rv::FEEDBACK];
        if ui.add(egui::Slider::new(&mut fb, 0.0..=1.0).show_value(false)).on_hover_text("What it answers goes round again: a machine that never had this knob").changed() {
            p[rv::FEEDBACK] = fb;
            changed = true;
        }
    });
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(80));
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_buttons_go_round_and_the_display_counts_as_the_machine_did() {
        assert_eq!(step(0, -1, 64), 63);
        assert_eq!(step(63, 1, 64), 0);
        assert_eq!(step(5, 1, 100), 6);
        assert_eq!(shown(0, 0), 1, "the first models count from one");
        assert_eq!(shown(2, 0), 0, "the last from nought");
    }

    #[test]
    fn the_digits_light_the_right_segments() {
        assert_eq!(segments(8), [true; 7]);
        assert_eq!(segments(1).iter().filter(|s| **s).count(), 2);
        assert_eq!(segments(0)[6], false);
        assert_eq!(segments(7).iter().filter(|s| **s).count(), 3);
    }
}
