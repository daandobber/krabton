//! Hand-painted widgets: knobs, faders, meters and small toggle buttons.
//! They take an explicit rect so the views can lay things out on a grid.

use crate::theme::*;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui};
use krabton_engine::params::{ParamKind, ParamSpec};
use std::f32::consts::PI;

/// A divider to drag, grabbed anywhere in `rect`. A `vertical` divider
/// moves left and right. It lights up under the pointer.
pub fn splitter(ui: &mut Ui, id: Id, rect: Rect, vertical: bool) -> Response {
    let resp = ui.interact(rect, id, Sense::drag());
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(if vertical { egui::CursorIcon::ResizeHorizontal } else { egui::CursorIcon::ResizeVertical });
        let p = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, id));
        let c = rect.center();
        let s = Stroke::new(2.0_f32, ACCENT);
        if vertical {
            p.line_segment([pos2(c.x, rect.top()), pos2(c.x, rect.bottom())], s);
        } else {
            p.line_segment([pos2(rect.left(), c.y), pos2(rect.right(), c.y)], s);
        }
    }
    resp
}

/// Three dots that show where something can be dragged to resize it.
pub fn grip_dots(p: &Painter, c: Pos2, vertical: bool, color: Color32) {
    for k in [-1.0f32, 0.0, 1.0] {
        let at = if vertical { c + vec2(0.0, 4.0 * k) } else { c + vec2(4.0 * k, 0.0) };
        p.circle_filled(at, 1.2, color);
    }
}

/// Drag the right edge of a display (`rect`) to change its `width`.
pub fn width_grip(ui: &mut Ui, id: Id, rect: Rect, width: &mut f32, min: f32, max: f32) {
    let strip = Rect::from_min_max(pos2(rect.right() - 4.0, rect.top()), pos2(rect.right() + 4.0, rect.bottom()));
    let resp = splitter(ui, id, strip, true);
    if !(resp.hovered() || resp.dragged()) {
        grip_dots(ui.painter(), pos2(rect.right() - 3.0, rect.center().y), true, TEXT_DIM.gamma_multiply(0.7));
    }
    if resp.dragged() {
        *width = (*width + resp.drag_delta().x).clamp(min, max);
    }
}

pub fn rect_fill(p: &Painter, r: Rect, radius: f32, c: Color32) {
    p.rect_filled(r, CornerRadius::same(radius as u8), c);
}

pub fn rect_line(p: &Painter, r: Rect, radius: f32, width: f32, c: Color32) {
    p.rect_stroke(r, CornerRadius::same(radius as u8), Stroke::new(width, c), StrokeKind::Inside);
}

pub fn text(p: &Painter, pos: Pos2, anchor: Align2, s: impl ToString, font: egui::FontId, c: Color32) -> Rect {
    p.text(pos, anchor, s.to_string(), font, c)
}

/// Text clipped to `rect`, left-aligned and vertically centered.
/// Text whose digits all take the same width (as tabular figures do), so a
/// number that changes does not make its neighbours jump.
pub fn tabular_text(p: &Painter, pos: Pos2, anchor: Align2, s: &str, font: egui::FontId, c: Color32) -> Rect {
    let digit = p.layout_no_wrap("0".to_owned(), font.clone(), c).size();
    let widths: Vec<f32> = s.chars().map(|ch| if ch.is_ascii_digit() { digit.x } else { p.layout_no_wrap(ch.to_string(), font.clone(), c).size().x }).collect();
    let total: f32 = widths.iter().sum();
    let rect = anchor.anchor_size(pos, vec2(total, digit.y));
    let mut x = rect.left();
    for (ch, w) in s.chars().zip(widths) {
        if !ch.is_whitespace() {
            p.text(pos2(x + w * 0.5, rect.center().y), Align2::CENTER_CENTER, ch, font.clone(), c);
        }
        x += w;
    }
    rect
}

pub fn text_in(p: &Painter, rect: Rect, s: impl ToString, font: egui::FontId, c: Color32) {
    let p = p.with_clip_rect(rect.intersect(p.clip_rect()));
    p.text(pos2(rect.left() + 4.0, rect.center().y), Align2::LEFT_CENTER, s.to_string(), font, c);
}

pub fn play_icon(p: &Painter, center: Pos2, size: f32, c: Color32) {
    let h = size * 0.5;
    let pts = vec![pos2(center.x - h * 0.7, center.y - h), pos2(center.x + h * 0.9, center.y), pos2(center.x - h * 0.7, center.y + h)];
    p.add(Shape::convex_polygon(pts, c, Stroke::NONE));
}

pub fn stop_icon(p: &Painter, center: Pos2, size: f32, c: Color32) {
    p.rect_filled(Rect::from_center_size(center, vec2(size, size)), CornerRadius::same(1), c);
}

pub fn rec_icon(p: &Painter, center: Pos2, size: f32, c: Color32) {
    p.circle_filled(center, size * 0.5, c);
}

/// Flat button with a label. Returns the response (check `.clicked()`).
pub fn button(ui: &mut Ui, rect: Rect, id: Id, label: &str, on: bool, on_color: Color32) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    let p = ui.painter();
    let off = crate::skin::switch_off();
    let fill = if on {
        on_color
    } else if resp.hovered() {
        mix(off, Color32::WHITE, 0.12)
    } else {
        off
    };
    rect_fill(p, rect, 2.0, fill);
    let tc = if on { text_on(on_color) } else { crate::skin::well().1 };
    p.with_clip_rect(rect).text(rect.center(), Align2::CENTER_CENTER, label, small(), tc);
    resp
}

pub(crate) fn arc_points(center: Pos2, r: f32, a0: f32, a1: f32) -> Vec<Pos2> {
    let steps = (((a1 - a0).abs() / (PI / 24.0)).ceil() as usize).max(2);
    (0..=steps)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / steps as f32;
            pos2(center.x + r * a.cos(), center.y + r * a.sin())
        })
        .collect()
}

/// Tracks the unrounded position (0..1) of a control while it is dragged.
/// Without this, controls whose value is rounded (integer parameters) or
/// clamped (faders at -inf) lose every small mouse movement between frames
/// and feel stuck.
pub(crate) fn drag_position(ui: &Ui, resp: &Response, id: Id, current: f32, delta: f32) -> Option<f32> {
    let key = id.with("drag_pos");
    if resp.drag_started() {
        ui.ctx().memory_mut(|m| m.data.insert_temp(key, current));
    }
    let mut out = None;
    if resp.dragged() && delta != 0.0 {
        let base = ui.ctx().memory(|m| m.data.get_temp::<f32>(key)).unwrap_or(current);
        let v = (base + delta).clamp(0.0, 1.0);
        ui.ctx().memory_mut(|m| m.data.insert_temp(key, v));
        out = Some(v);
    }
    if resp.drag_stopped() {
        ui.ctx().memory_mut(|m| m.data.remove::<f32>(key));
    }
    out
}

/// The position to draw while dragging (follows the mouse smoothly even
/// when the value itself moves in steps).
pub(crate) fn shown_position(ui: &Ui, resp: &Response, id: Id, current: f32) -> f32 {
    if resp.dragged() {
        ui.ctx().memory(|m| m.data.get_temp::<f32>(id.with("drag_pos"))).unwrap_or(current)
    } else {
        current
    }
}

pub(crate) const KNOB_START: f32 = 0.75 * PI;
pub(crate) const KNOB_RANGE: f32 = 1.5 * PI;

/// Draws a rotary knob in `rect`. Returns the response and the new
/// normalized value if it changed. Drag vertically; Shift for fine; double
/// click resets to `default_norm`.
pub fn knob(ui: &mut Ui, rect: Rect, id: Id, norm: f32, default_norm: f32, bipolar: bool, color: Color32) -> (Response, Option<f32>) {
    knob_with_speed(ui, rect, id, norm, default_norm, bipolar, color, 1.0)
}

/// Like [`knob`], with a drag sensitivity multiplier.
#[allow(clippy::too_many_arguments)]
pub fn knob_with_speed(ui: &mut Ui, rect: Rect, id: Id, norm: f32, default_norm: f32, bipolar: bool, color: Color32, speed_scale: f32) -> (Response, Option<f32>) {
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let fine = ui.input(|i| i.modifiers.shift);
    let speed = if fine { 0.0015 } else { 0.006 } * speed_scale;
    let d = -resp.drag_delta().y * speed + resp.drag_delta().x * speed * 0.5;
    let mut new = drag_position(ui, &resp, id, norm, d);
    if resp.double_clicked() {
        new = Some(default_norm);
    }
    let v = new.unwrap_or_else(|| shown_position(ui, &resp, id, norm));
    ring_face(ui.painter(), rect, v, bipolar, color, resp.hovered() || resp.dragged());
    (resp, new)
}

/// The ring under a knob's arc.
pub(crate) const RING_TRACK: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3f);

/// Krabton's knob: a thick ring on a dark disc, the value as a light arc
/// (white under the pointer), and a short tick in the accent colour.
pub(crate) fn ring_face(p: &Painter, rect: Rect, v: f32, bipolar: bool, color: Color32, hot: bool) {
    match crate::skin::current() {
        Some(s) => crate::skin::knob_face(p, rect, v, bipolar, color, hot, &s),
        None => ring_face_plain(p, rect, v, bipolar, color, hot),
    }
}

/// [`ring_face`] in the plain look.
pub(crate) fn ring_face_plain(p: &Painter, rect: Rect, v: f32, bipolar: bool, color: Color32, hot: bool) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.5 - 3.0;
    let w = (r * 0.36).clamp(3.0, 5.0);
    p.circle_filled(c, r + 3.0, Color32::from_rgb(0x12, 0x12, 0x14));
    p.add(Shape::line(arc_points(c, r, KNOB_START, KNOB_START + KNOB_RANGE), Stroke::new(w, RING_TRACK)));
    let (a, b) = if bipolar { (0.5f32.min(v), 0.5f32.max(v)) } else { (0.0, v) };
    let fill = if hot { Color32::WHITE } else { mix(Color32::from_gray(228), color, 0.18) };
    if b - a > 0.004 {
        p.add(Shape::line(arc_points(c, r, KNOB_START + KNOB_RANGE * a, KNOB_START + KNOB_RANGE * b), Stroke::new(w, fill)));
    }
    let ang = KNOB_START + KNOB_RANGE * v;
    let dir = vec2(ang.cos(), ang.sin());
    p.line_segment([c + dir * (r - w - 2.0), c + dir * (r + w * 0.4)], Stroke::new(2.0_f32, color));
}

/// A vertical slider for any parameter, laid out like [`param_cell`]: the
/// name on top, the value below; drawn like the mixer's faders. Returns
/// whether it changed and the slot (for overlays).
pub fn param_slider(ui: &mut Ui, rect: Rect, id: Id, spec: &ParamSpec, value: &mut f32, accent: Color32) -> (bool, Rect) {
    let p = ui.painter().clone();
    let label = Rect::from_min_size(rect.min, vec2(rect.width(), 12.0));
    let name = crate::skin::label_text(spec.name);
    let mut font = small();
    let width = p.layout_no_wrap(name.clone(), font.clone(), TEXT_DIM).size().x;
    if width > rect.width() {
        font.size = (font.size * rect.width() / width).max(8.0);
    }
    p.with_clip_rect(label).text(label.center(), Align2::CENTER_CENTER, name, font, crate::skin::ink());
    let value_rect = Rect::from_min_size(pos2(rect.left(), rect.bottom() - 12.0), vec2(rect.width(), 12.0));
    let track = Rect::from_center_size(pos2(rect.center().x, (label.bottom() + value_rect.top()) * 0.5), vec2(4.0, value_rect.top() - label.bottom() - 14.0));
    let hit = Rect::from_min_max(pos2(rect.left(), track.top() - 6.0), pos2(rect.right(), track.bottom() + 6.0));
    let resp = ui.interact(hit, id, Sense::click_and_drag());
    let fine = ui.input(|i| i.modifiers.shift);
    let d = -resp.drag_delta().y / track.height().max(1.0) * if fine { 0.2 } else { 1.0 };
    let mut changed = false;
    if let Some(n) = drag_position(ui, &resp, id, spec.to_norm(*value), d) {
        let v = spec.from_norm(n);
        if v != *value {
            *value = v;
            changed = true;
        }
    }
    if resp.double_clicked() {
        *value = spec.default;
        changed = true;
    }
    let shown = shown_position(ui, &resp, id, spec.to_norm(*value));
    rect_fill(&p, track, 2.0, BG);
    let y = track.bottom() - shown * track.height();
    let bipolar = spec.min < 0.0 && spec.max > 0.0;
    let from = if bipolar { track.center().y } else { track.bottom() };
    rect_fill(&p, Rect::from_min_max(pos2(track.left(), y.min(from)), pos2(track.right(), y.max(from))), 2.0, shade(accent, 0.8));
    let thumb = Rect::from_center_size(pos2(track.center().x, y), vec2((rect.width() - 10.0).min(22.0), 8.0));
    rect_fill(&p, thumb, 2.0, if resp.hovered() || resp.dragged() { Color32::WHITE } else { TEXT });
    tabular_text(&p.with_clip_rect(value_rect), value_rect.center(), Align2::CENTER_CENTER, &spec.format(*value), small(), crate::skin::ink_dim());
    resp.on_hover_text(format!("{}: {}\nDrag (Shift: fine), double-click to reset", spec.name, spec.format(*value)));
    (changed, track)
}

/// Where a clip envelope has a knob now, as a red dot on its rim.
pub fn knob_auto_mark(p: &Painter, rect: Rect, norm: f32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.5 - 2.0;
    let ang = KNOB_START + KNOB_RANGE * norm.clamp(0.0, 1.0);
    p.circle_filled(pos2(c.x + (r - 2.0) * ang.cos(), c.y + (r - 2.0) * ang.sin()), 3.0, REC);
}

/// Where a clip envelope has a fader now (0..1 of its travel), as a red line.
pub fn fader_auto_mark(p: &Painter, rect: Rect, pos: f32) {
    let track = Rect::from_center_size(rect.center(), vec2(4.0, rect.height() - 8.0));
    let y = track.bottom() - pos.clamp(0.0, 1.0) * track.height();
    p.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(2.0_f32, REC));
}

/// Height of a parameter cell made by [`param_cell`].
pub const CELL_W: f32 = 54.0;
pub const CELL_H: f32 = 60.0;

/// A labelled control for any engine parameter: knob, choice box or toggle.
/// Returns true when the value changed.
pub fn param_cell(ui: &mut Ui, rect: Rect, id: Id, spec: &ParamSpec, value: &mut f32, accent: Color32) -> bool {
    let p = ui.painter().clone();
    let label_rect = Rect::from_min_size(rect.min, vec2(rect.width(), 12.0));
    let hero = is_hero_cell(rect);
    let name = crate::skin::label_text(spec.name);
    // Long names get a smaller font rather than being cut off.
    let mut font = if hero { body() } else { small() };
    let width = p.layout_no_wrap(name.clone(), font.clone(), TEXT_DIM).size().x;
    if width > rect.width() {
        font.size = (font.size * rect.width() / width).max(if name.chars().any(|c| c.is_lowercase()) { 8.0 } else { 7.0 });
    }
    let skinned = crate::skin::current().is_some();
    p.with_clip_rect(label_rect).text(label_rect.center(), Align2::CENTER_CENTER, name, font, if skinned { crate::skin::ink() } else { TEXT_DIM });
    let value_rect = Rect::from_min_size(pos2(rect.left(), rect.bottom() - 12.0), vec2(rect.width(), 12.0));
    let ctrl = Rect::from_min_max(pos2(rect.left(), label_rect.bottom() + 1.0), pos2(rect.right(), value_rect.top() - 1.0));
    let mut changed = false;
    match spec.kind {
        ParamKind::Choice(opts) => {
            changed |= choice_slider(ui, ctrl, id, spec, opts, value, accent);
        }
        ParamKind::Toggle => {
            let b = Rect::from_center_size(ctrl.center(), vec2(rect.width() - 10.0, 18.0));
            let on = *value >= 0.5;
            let resp = button(ui, b, id, if on { "On" } else { "Off" }, on, accent);
            if resp.clicked() {
                *value = if on { 0.0 } else { 1.0 };
                changed = true;
            }
        }
        _ => {
            let k = cell_knob_rect(rect);
            let bipolar = spec.min < 0.0 && spec.max > 0.0;
            let norm = spec.to_norm(*value);
            // Stepped parameters with few values: about 24 px of drag per step.
            let steps = if spec.is_discrete() { (spec.max - spec.min).round().max(1.0) } else { 0.0 };
            let speed = if steps > 0.0 { (167.0 / (steps * 24.0)).max(1.0) } else { 1.0 };
            let (resp, new) = knob_with_speed(ui, k, id, norm, spec.to_norm(spec.default), bipolar, accent, speed);
            if let Some(n) = new {
                let v = spec.from_norm(n);
                if v != *value {
                    *value = v;
                    changed = true;
                }
            }
            resp.on_hover_text(format!("{}: {}\nDrag to change, Shift for fine, double-click to reset", spec.name, spec.format(*value)));
        }
    }
    tabular_text(&p.with_clip_rect(value_rect), value_rect.center(), Align2::CENTER_CENTER, &spec.format(*value), if hero { body() } else { small() }, if skinned { crate::skin::ink_dim() } else { TEXT });
    changed
}

/// Whether a cell is a big one (two by two).
pub fn is_hero_cell(cell: Rect) -> bool {
    cell.width() > CELL_W * 1.5 && cell.height() > CELL_H * 1.5
}

/// Where the knob sits in a cell made by [`param_cell`].
pub fn cell_knob_rect(cell: Rect) -> Rect {
    let ctrl = Rect::from_min_max(pos2(cell.left(), cell.top() + 13.0), pos2(cell.right(), cell.bottom() - 13.0));
    let size = ctrl.height().min(cell.width()).min(if is_hero_cell(cell) { 84.0 } else { 36.0 });
    Rect::from_center_size(ctrl.center(), vec2(size, size))
}

/// A choice as a horizontal slider: drag left and right through the options
/// (each a notch on the track), click for the list, double-click for the
/// default.
fn choice_slider(ui: &mut Ui, ctrl: Rect, id: Id, spec: &ParamSpec, opts: &[&str], value: &mut f32, accent: Color32) -> bool {
    let p = ui.painter().clone();
    let n = opts.len().max(1);
    let b = Rect::from_center_size(ctrl.center(), vec2(ctrl.width() - 4.0, 22.0));
    let resp = ui.interact(b, id, Sense::click_and_drag());
    let idx = (value.round().max(0.0) as usize).min(n - 1);
    let mut changed = false;
    // Dragging: a notch every few pixels, at least across the box for short lists.
    let step_px = (b.width() / n as f32).clamp(7.0, 16.0);
    let acc_id = id.with("choice_drag");
    if resp.drag_started() {
        ui.ctx().memory_mut(|m| m.data.insert_temp(acc_id, 0.0f32));
    }
    if resp.dragged() {
        let acc = ui.ctx().memory(|m| m.data.get_temp::<f32>(acc_id).unwrap_or(0.0)) + resp.drag_delta().x;
        let k = (acc / step_px).trunc();
        ui.ctx().memory_mut(|m| m.data.insert_temp(acc_id, acc - k * step_px));
        if k != 0.0 {
            let next = (idx as f32 + k).clamp(0.0, (n - 1) as f32);
            if next != *value {
                *value = next;
                changed = true;
            }
        }
    }
    if resp.double_clicked() {
        *value = spec.default;
        changed = true;
    }
    let hot = resp.hovered() || resp.dragged();
    let (well, well_ink) = crate::skin::well();
    rect_fill(&p, b, 2.0, if hot { mix(well, Color32::WHITE, 0.1) } else { well });
    // The track: a notch per option, filled up to this one.
    let track = Rect::from_min_max(pos2(b.left() + 4.0, b.bottom() - 5.0), pos2(b.right() - 4.0, b.bottom() - 3.0));
    let shown = (value.round().max(0.0) as usize).min(n - 1);
    let at = |i: usize| if n > 1 { track.left() + track.width() * i as f32 / (n - 1) as f32 } else { track.center().x };
    rect_fill(&p, track, 1.0, BG);
    rect_fill(&p, Rect::from_min_max(track.min, pos2(at(shown).max(track.left() + 2.0), track.max.y)), 1.0, shade(accent, 0.8));
    if n <= 24 {
        for i in 0..n {
            p.line_segment([pos2(at(i), track.top() - 1.5), pos2(at(i), track.top())], Stroke::new(1.0_f32, LINE));
        }
    }
    p.circle_filled(pos2(at(shown), track.center().y), 3.0, if hot { Color32::WHITE } else { TEXT });
    let text_rect = Rect::from_min_max(b.min, pos2(b.right(), track.top() - 1.0));
    p.with_clip_rect(b).text(text_rect.center(), Align2::CENTER_CENTER, opts.get(shown).copied().unwrap_or("?"), small(), well_ink);
    // A click opens the list.
    egui::Popup::menu(&resp).show(|ui| {
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            for (i, o) in opts.iter().enumerate() {
                if ui.selectable_label(i == shown, *o).clicked() {
                    *value = i as f32;
                    changed = true;
                    ui.close();
                }
            }
        });
    });
    resp.on_hover_text(format!("{}: {}\nDrag left or right, click for the list, double-click to reset", spec.name, opts.get(shown).copied().unwrap_or("?")));
    changed
}

/// dB value to 0..1 fader position (0 dB sits at ~84%).
pub fn db_to_fader(db: f32) -> f32 {
    use krabton_engine::project::{FADER_MAX_DB, FADER_MIN_DB};
    if db <= FADER_MIN_DB + 0.05 {
        return 0.0;
    }
    let g = 10f32.powf(db / 20.0);
    let gmax = 10f32.powf(FADER_MAX_DB / 20.0);
    (g.powf(0.25) / gmax.powf(0.25)).clamp(0.0, 1.0)
}

pub fn fader_to_db(pos: f32) -> f32 {
    use krabton_engine::project::{FADER_MAX_DB, FADER_MIN_DB};
    let gmax = 10f32.powf(FADER_MAX_DB / 20.0);
    let g = (pos.clamp(0.0, 1.0) * gmax.powf(0.25)).powi(4);
    if g <= 1e-6 {
        return FADER_MIN_DB;
    }
    (20.0 * g.log10()).clamp(FADER_MIN_DB, FADER_MAX_DB)
}

/// Vertical volume fader. Returns true when changed.
pub fn fader(ui: &mut Ui, rect: Rect, id: Id, db: &mut f32, color: Color32) -> bool {
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let mut changed = false;
    let fine = ui.input(|i| i.modifiers.shift);
    let d = -resp.drag_delta().y / rect.height() * if fine { 0.2 } else { 1.0 };
    if let Some(pos) = drag_position(ui, &resp, id, db_to_fader(*db), d) {
        let v = fader_to_db(pos);
        if v != *db {
            *db = v;
            changed = true;
        }
    }
    if resp.double_clicked() {
        *db = 0.0;
        changed = true;
    }
    let p = ui.painter();
    let track = Rect::from_center_size(rect.center(), vec2(4.0, rect.height() - 8.0));
    rect_fill(p, track, 2.0, BG);
    let pos = db_to_fader(*db);
    let y = track.bottom() - pos * track.height();
    let fill = Rect::from_min_max(pos2(track.left(), y), track.max);
    rect_fill(p, fill, 2.0, shade(color, 0.8));
    let zero = track.bottom() - db_to_fader(0.0) * track.height();
    p.line_segment([pos2(rect.left() + 2.0, zero), pos2(rect.left() + 6.0, zero)], Stroke::new(1.0_f32, TEXT_DIM));
    let thumb = Rect::from_center_size(pos2(rect.center().x, y), vec2(rect.width() - 4.0, 8.0));
    rect_fill(p, thumb, 2.0, if resp.hovered() || resp.dragged() { Color32::WHITE } else { TEXT });
    let txt = if *db <= krabton_engine::project::FADER_MIN_DB + 0.05 { "-inf dB".to_string() } else { format!("{:.1} dB", db) };
    resp.on_hover_text(format!("{txt}\nDrag, Shift for fine, double-click for 0 dB"));
    changed
}

fn meter_pos(level: f32) -> f32 {
    if level <= 1e-5 {
        0.0
    } else {
        let db = 20.0 * level.log10();
        db_to_fader(db.min(6.0))
    }
}

/// Stereo peak meter.
pub fn meter(p: &Painter, rect: Rect, l: f32, r: f32) {
    rect_fill(p, rect, 1.0, BG);
    let w = (rect.width() - 1.0) / 2.0;
    let zero = meter_pos(1.0);
    let warn = meter_pos(0.5);
    for (i, lv) in [l, r].into_iter().enumerate() {
        let x0 = rect.left() + i as f32 * (w + 1.0);
        let pos = meter_pos(lv);
        let seg = |a: f32, b: f32, c: Color32| {
            if b > a {
                let y0 = rect.bottom() - b * rect.height();
                let y1 = rect.bottom() - a * rect.height();
                p.rect_filled(Rect::from_min_max(pos2(x0, y0), pos2(x0 + w, y1)), CornerRadius::ZERO, c);
            }
        };
        seg(0.0, pos.min(warn), PLAY);
        seg(warn, pos.min(zero), Color32::from_rgb(0xff, 0xd6, 0x0a));
        seg(zero, pos, REC);
    }
}

/// Drag and drop: the payload of type `T` released over `resp`, if any.
///
/// Unlike `Response::dnd_release_payload`, a payload of another type is left
/// alone (egui's version takes whatever is being dragged, so checking for one
/// type first would make a drop of another type disappear).
pub fn drop_payload<T: std::any::Any + Send + Sync>(resp: &Response) -> Option<std::sync::Arc<T>> {
    if egui::DragAndDrop::has_payload_of_type::<T>(&resp.ctx) {
        resp.dnd_release_payload::<T>()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use krabton_engine::params::ParamSpec;

    static RATES: &[&str] = &["1/32", "1/16 T", "1/16", "1/8 T", "1/8", "1/4", "1/2"];
    static SPEC: ParamSpec = ParamSpec::choice("Rate", "Clock", RATES, 2);

    /// Runs frames of a choice cell with these pointer events, one list per frame.
    fn frames(value: &mut f32, events: Vec<Vec<egui::Event>>) -> Rect {
        let ctx = egui::Context::default();
        let cell = Rect::from_min_size(pos2(20.0, 20.0), vec2(CELL_W - 2.0, CELL_H - 2.0));
        for ev in std::iter::once(Vec::new()).chain(events) {
            let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))), events: ev, ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    param_cell(ui, cell, Id::new("rate"), &SPEC, value, Color32::WHITE);
                });
            });
        }
        cell
    }

    #[test]
    fn a_choice_slides_left_and_right() {
        let mut v = 2.0;
        let c = pos2(20.0 + (CELL_W - 2.0) / 2.0, 20.0 + (CELL_H - 2.0) / 2.0);
        let button = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
        let mut ev = vec![vec![egui::Event::PointerMoved(c)], vec![button(c, true)]];
        for k in 1..=10 {
            ev.push(vec![egui::Event::PointerMoved(c + vec2(3.0 * k as f32, 0.0))]);
        }
        ev.push(vec![button(c + vec2(30.0, 0.0), false)]);
        frames(&mut v, ev);
        // 30 px at a notch every 7 px (less the few a drag needs to start).
        assert!((4.0..=6.0).contains(&v), "{v}");
        let mut v = 2.0;
        let mut ev = vec![vec![egui::Event::PointerMoved(c)], vec![button(c, true)]];
        for k in 1..=5 {
            ev.push(vec![egui::Event::PointerMoved(c - vec2(3.0 * k as f32, 0.0))]);
        }
        ev.push(vec![button(c - vec2(15.0, 0.0), false)]);
        frames(&mut v, ev);
        assert!(v < 2.0, "{v}");
    }
}
