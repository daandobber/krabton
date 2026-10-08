//! Hand-painted widgets: knobs, faders, meters and small toggle buttons.
//! They take an explicit rect so the views can lay things out on a grid.

use crate::theme::*;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui};
use krabton_engine::params::{ParamKind, ParamSpec};
use std::f32::consts::PI;

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
    let fill = if on {
        on_color
    } else if resp.hovered() {
        shade(PANEL3, 1.3)
    } else {
        PANEL3
    };
    rect_fill(p, rect, 2.0, fill);
    let tc = if on { text_on(on_color) } else { TEXT };
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
fn drag_position(ui: &Ui, resp: &Response, id: Id, current: f32, delta: f32) -> Option<f32> {
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
fn shown_position(ui: &Ui, resp: &Response, id: Id, current: f32) -> f32 {
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
    let p = ui.painter();
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.5 - 2.0;
    p.circle_filled(c, r, KNOB);
    p.add(Shape::line(arc_points(c, r - 2.0, KNOB_START, KNOB_START + KNOB_RANGE), Stroke::new(2.5_f32, LINE)));
    let (from, to): (f32, f32) = if bipolar { (0.5, v) } else { (0.0, v) };
    let (a, b) = (KNOB_START + KNOB_RANGE * from.min(to), KNOB_START + KNOB_RANGE * from.max(to));
    if b - a > 0.001 {
        p.add(Shape::line(arc_points(c, r - 2.0, a, b), Stroke::new(2.5_f32, color)));
    }
    let ang = KNOB_START + KNOB_RANGE * v;
    let tip = pos2(c.x + (r - 3.0) * ang.cos(), c.y + (r - 3.0) * ang.sin());
    p.line_segment([c, tip], Stroke::new(2.0_f32, if resp.hovered() || resp.dragged() { Color32::WHITE } else { TEXT }));
    (resp, new)
}

/// Height of a parameter cell made by [`param_cell`].
pub const CELL_W: f32 = 54.0;
pub const CELL_H: f32 = 60.0;

/// A labelled control for any engine parameter: knob, choice box or toggle.
/// Returns true when the value changed.
pub fn param_cell(ui: &mut Ui, rect: Rect, id: Id, spec: &ParamSpec, value: &mut f32, accent: Color32) -> bool {
    let p = ui.painter().clone();
    let label_rect = Rect::from_min_size(rect.min, vec2(rect.width(), 12.0));
    // Long names get a smaller font rather than being cut off.
    let mut font = small();
    let width = p.layout_no_wrap(spec.name.to_string(), font.clone(), TEXT_DIM).size().x;
    if width > rect.width() {
        font.size = (font.size * rect.width() / width).max(8.0);
    }
    p.with_clip_rect(label_rect).text(label_rect.center(), Align2::CENTER_CENTER, spec.name, font, TEXT_DIM);
    let value_rect = Rect::from_min_size(pos2(rect.left(), rect.bottom() - 12.0), vec2(rect.width(), 12.0));
    let ctrl = Rect::from_min_max(pos2(rect.left(), label_rect.bottom() + 1.0), pos2(rect.right(), value_rect.top() - 1.0));
    let mut changed = false;
    match spec.kind {
        ParamKind::Choice(opts) => {
            let b = Rect::from_center_size(ctrl.center(), vec2(rect.width() - 4.0, 18.0));
            let resp = ui.interact(b, id, Sense::click());
            let idx = value.round() as usize;
            rect_fill(&p, b, 2.0, if resp.hovered() { PANEL3 } else { KNOB });
            rect_line(&p, b, 2.0, 1.0, shade(accent, 0.6));
            p.with_clip_rect(b).text(b.center(), Align2::CENTER_CENTER, opts.get(idx).copied().unwrap_or("?"), small(), TEXT);
            if resp.clicked() {
                *value = ((idx + 1) % opts.len()) as f32;
                changed = true;
            }
            if resp.secondary_clicked() {
                *value = ((idx + opts.len() - 1) % opts.len()) as f32;
                changed = true;
            }
            resp.on_hover_text(format!("{}: click for next, right-click for previous", spec.name));
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
            let size = ctrl.height().min(rect.width()).min(36.0);
            let k = Rect::from_center_size(ctrl.center(), vec2(size, size));
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
    p.with_clip_rect(value_rect).text(value_rect.center(), Align2::CENTER_CENTER, spec.format(*value), small(), TEXT);
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
