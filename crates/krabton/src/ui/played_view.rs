//! Editors for the sequencers you play into: Spinner (the turning polygon
//! and its balls), Endless (the row of steps, with Rest, Back and Clear),
//! Hands (the patterns, the step that plays) and Draw (the line, drawn with
//! the mouse or a finger).

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Pos2, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::sequencers::played::{self, draw, endless, hands, spinner, DRAW_POINTS, PATTERNS, PATTERN_STEPS};
use std::f32::consts::TAU;

fn bytes(v: [u64; 4]) -> [u8; 32] {
    let mut b = [0u8; 32];
    for (k, x) in v.iter().enumerate() {
        b[k * 8..k * 8 + 8].copy_from_slice(&x.to_le_bytes());
    }
    b
}

/// Spinner: the polygon turning, the balls in it.
pub(crate) fn spinner(ui: &mut Ui, _id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let side = height.min(260.0);
    let (rect, _) = ui.allocate_exact_size(vec2(side + 150.0, height), Sense::hover());
    let p = ui.painter().clone();
    let box_r = Rect::from_min_size(rect.min, vec2(side, height));
    rect_fill(&p, box_r, 4.0, KNOB);
    let c = box_r.center();
    let rad = side * 0.44;
    let sides = (params[spinner::SIDES].round() as usize).clamp(3, 8);
    let (angle, balls) = match live.vis {
        Some(v) => {
            let b = bytes(v);
            let ang = u16::from_le_bytes([b[0], b[1]]) as f32 / 65535.0 * TAU;
            let n = (b[3] as usize).min(14);
            let balls: Vec<Pos2> = (0..n).map(|i| pos2(b[4 + i * 2] as f32 / 127.5 - 1.0, b[5 + i * 2] as f32 / 127.5 - 1.0)).collect();
            (ang, balls)
        }
        None => (0.0, Vec::new()),
    };
    let pts: Vec<Pos2> = (0..=sides).map(|s| {
        let a = angle + TAU * s as f32 / sides as f32;
        c + vec2(a.cos(), -a.sin()) * rad
    }).collect();
    p.add(Shape::line(pts, Stroke::new(2.0_f32, accent)));
    let ball_r = (params[spinner::SIZE] * rad).max(3.0);
    for b in &balls {
        p.circle_filled(c + vec2(b.x, -b.y) * rad, ball_r, TEXT);
    }
    let hint = if balls.is_empty() { "Play notes: each drops a ball" } else { "" };
    text(&p, pos2(box_r.center().x, box_r.bottom() - 10.0), Align2::CENTER_CENTER, hint, FontId::proportional(10.0), TEXT_DIM);
    let info = Rect::from_min_max(pos2(box_r.right() + 10.0, rect.top()), rect.max);
    text(&p, info.left_top() + vec2(0.0, 4.0), Align2::LEFT_TOP, format!("{} balls\n{} sides\nspin {:+.2}", balls.len(), sides, params[spinner::SPIN]), FontId::monospace(11.0), TEXT_DIM);
    false
}

/// Endless: the row; Rest, Back and Clear while it records.
pub(crate) fn endless(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    let (row, len) = played::endless_row(data);
    let row: Vec<u8> = row.iter().take(len).copied().collect();
    let recording = params[endless::RECORD] >= 0.5;
    let now = live.on().map(|v| (v[0] & 0xff) as usize);
    let cols = 16usize;
    let cw = 22.0;
    let rows = 4usize;
    let rh = ((height - 30.0) / rows as f32).clamp(12.0, 26.0);
    let (rect, _) = ui.allocate_exact_size(vec2(cols as f32 * cw + 110.0, height), Sense::hover());
    let p = ui.painter().clone();
    let grid = Rect::from_min_size(rect.min, vec2(cols as f32 * cw, rows as f32 * rh));
    rect_fill(&p, grid, 3.0, KNOB);
    for i in 0..cols * rows {
        let cell = Rect::from_min_size(pos2(grid.left() + (i % cols) as f32 * cw + 1.0, grid.top() + (i / cols) as f32 * rh + 1.0), vec2(cw - 2.0, rh - 2.0));
        let (fill, label) = match row.get(i) {
            Some(0) => (PANEL3, "·".to_string()),
            Some(&n) => (shade(accent, 0.55), krabton_engine::note_name(n)),
            None if i == len && recording => (shade(REC, 0.35), String::new()),
            None => (PANEL2, String::new()),
        };
        let fill = if now == Some(i) && !recording { accent } else { fill };
        p.rect_filled(cell, 2.0, fill);
        if !label.is_empty() {
            text(&p, cell.center(), Align2::CENTER_CENTER, label, FontId::proportional(9.0), if now == Some(i) { TEXT_DARK } else { TEXT });
        }
    }
    let side = Rect::from_min_max(pos2(grid.right() + 10.0, rect.top()), rect.max);
    let mut y = side.top();
    let mut button = |label: &str, on: bool, tip: &str| -> bool {
        let r = Rect::from_min_size(pos2(side.left(), y), vec2(side.width().min(96.0), 22.0));
        y += 26.0;
        let resp = ui.interact(r, id.with(label), Sense::click());
        rect_fill(&p, r, 3.0, if on { REC } else if resp.hovered() { PANEL3 } else { PANEL2 });
        text(&p, r.center(), Align2::CENTER_CENTER, label, FontId::proportional(11.0), TEXT);
        resp.on_hover_text(tip).clicked()
    };
    if button(if recording { "Recording" } else { "Record" }, recording, "While recording, every note you play is added to the row; switch it off to play the row") {
        params[endless::RECORD] = if recording { 0.0 } else { 1.0 };
        changed = true;
    }
    if button("Rest", false, "Add a rest") && played::endless_add(data, 0) {
        changed = true;
    }
    if button("Back", false, "Take the last step off") && !data.is_empty() && data[0] > 0 {
        data[0] -= 1;
        changed = true;
    }
    if button("Clear", false, "Start the row again") && !data.is_empty() {
        data[0] = 0;
        changed = true;
    }
    text(&p, pos2(side.left(), y + 4.0), Align2::LEFT_TOP, format!("{len} / {} steps", played::ENDLESS_STEPS), FontId::monospace(10.0), TEXT_DIM);
    changed
}

/// Hands: the patterns, the one held keys play lit, the step now.
pub(crate) fn hands(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    let cw = 14.0;
    let label_w = 76.0;
    let rh = ((height - 6.0) / PATTERNS.len() as f32).clamp(8.0, 18.0);
    let (rect, _) = ui.allocate_exact_size(vec2(label_w + 16.0 * cw, height), Sense::hover());
    let p = ui.painter().clone();
    let vis = live.on().map(|v| v[0]);
    let step = vis.map(|v| (v & 0xff) as usize);
    let held = vis.map(|v| ((v >> 8) & 0xff) as usize).unwrap_or(0);
    let per_key = params[hands::PER_KEY] >= 0.5;
    let shared = (params[hands::PATTERN].round().max(0.0) as usize).min(PATTERNS.len() - 1);
    for (k, name) in PATTERNS.iter().enumerate() {
        let y = rect.top() + k as f32 * rh;
        let lab = Rect::from_min_size(pos2(rect.left(), y), vec2(label_w - 4.0, rh - 1.0));
        let resp = ui.interact(lab, id.with(("pattern", k)), Sense::click());
        let chosen = !per_key && k == shared;
        rect_fill(&p, lab, 2.0, if chosen { shade(accent, 0.6) } else if resp.hovered() { PANEL3 } else { PANEL2 });
        text(&p, lab.left_center() + vec2(4.0, 0.0), Align2::LEFT_CENTER, *name, FontId::proportional(10.0), if chosen { TEXT_DARK } else { TEXT });
        if resp.on_hover_text("The pattern every held key plays").clicked() {
            params[hands::PATTERN] = k as f32;
            params[hands::PER_KEY] = 0.0;
            changed = true;
        }
        for (s, &hit) in PATTERN_STEPS[k].iter().enumerate() {
            let cell = Rect::from_min_size(pos2(rect.left() + label_w + s as f32 * cw + 1.0, y + 1.0), vec2(cw - 2.0, rh - 3.0));
            let lit = held > 0 && (per_key || k == shared);
            let col = match hit {
                0 => PANEL2,
                1 => shade(accent, if lit { 0.65 } else { 0.3 }),
                _ => if lit { accent } else { shade(accent, 0.45) },
            };
            p.rect_filled(cell, 1.0, if step == Some(s) && hit > 0 && lit { TEXT } else { col });
        }
    }
    if let Some(s) = step {
        let x = rect.left() + label_w + s as f32 * cw;
        p.line_segment([pos2(x, rect.top()), pos2(x, rect.top() + PATTERNS.len() as f32 * rh)], Stroke::new(1.0_f32, TEXT_DIM));
    }
    changed
}

/// Draw: the line across 32 steps, drawn by dragging.
pub(crate) fn draw(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    if data.len() < DRAW_POINTS {
        *data = played::draw_default();
        changed = true;
    }
    let w = 340.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let area = Rect::from_min_max(rect.min + vec2(0.0, 4.0), rect.max - vec2(0.0, 16.0));
    let resp = ui.interact(area, id.with("line"), Sense::click_and_drag());
    let p = ui.painter().clone();
    rect_fill(&p, area, 3.0, KNOB);
    let col_w = area.width() / DRAW_POINTS as f32;
    // Drawing: the column under the pointer takes its height (and the ones passed over).
    let last_id = id.with("last");
    if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        let i = (((pt.x - area.left()) / col_w) as usize).min(DRAW_POINTS - 1);
        let v = (((area.bottom() - pt.y) / area.height()).clamp(0.0, 1.0) * 255.0) as u8;
        let prev: Option<(usize, u8)> = ui.ctx().memory(|m| m.data.get_temp(last_id));
        let (a, va) = prev.filter(|_| resp.dragged()).unwrap_or((i, v));
        let (lo, hi) = (a.min(i), a.max(i));
        for j in lo..=hi {
            let t = if hi == lo { 1.0 } else { (j - lo) as f32 / (hi - lo) as f32 };
            let (from, to) = if a <= i { (va, v) } else { (v, va) };
            data[j] = (from as f32 + (to as f32 - from as f32) * t) as u8;
        }
        ui.ctx().memory_mut(|m| m.data.insert_temp(last_id, (i, v)));
        changed = true;
    } else {
        ui.ctx().memory_mut(|m| m.data.remove::<(usize, u8)>(last_id));
    }
    // The key's lines across, the octaves stronger.
    let range = params[draw::RANGE].round().clamp(1.0, 4.0) as usize;
    for o in 0..=range {
        let y = area.bottom() - area.height() * o as f32 / range as f32;
        p.line_segment([pos2(area.left(), y), pos2(area.right(), y)], Stroke::new(1.0_f32, LINE));
    }
    let now = live.on().map(|v| v[0] as usize % DRAW_POINTS);
    let pts: Vec<Pos2> = (0..DRAW_POINTS).map(|i| pos2(area.left() + (i as f32 + 0.5) * col_w, area.bottom() - data[i] as f32 / 255.0 * area.height())).collect();
    p.add(Shape::line(pts.clone(), Stroke::new(2.0_f32, accent)));
    for (i, pt) in pts.iter().enumerate() {
        p.circle_filled(*pt, if now == Some(i) { 5.0 } else { 2.0 }, if now == Some(i) { TEXT } else { accent });
    }
    let base = params[draw::BASE].round().clamp(0.0, 127.0) as u8;
    text(&p, pos2(area.left(), rect.bottom() - 7.0), Align2::LEFT_CENTER, format!("{} up {} octave{}  ·  drag to draw", krabton_engine::note_name(base), range, if range == 1 { "" } else { "s" }), FontId::proportional(10.0), TEXT_DIM);
    changed
}
