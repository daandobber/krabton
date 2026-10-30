//! Editors for the sequencers after nature: Fireflies (the meadow, each
//! firefly glowing as it flashes, and how together they are), Sandpile (the
//! board, the grains on each cell and the avalanche), Pendulums (the row of
//! pendulums swinging), Flock (the birds over the lines) and Plants (the
//! plant, lit as far as it has played). Beside each picture the settings
//! that matter most: big targets to click (the next) or right-click (the
//! one before), and faders.

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::params::ParamSpec;
use krabton_engine::sequencers::nature as na;
use std::f32::consts::TAU;

/// Width of the settings column beside a picture.
pub(super) const SIDE: f32 = 112.0;
/// Height of a setting in it.
pub(super) const ROW: f32 = 22.0;

pub(super) fn frame(ui: &mut Ui, w: f32, height: f32) -> (Rect, egui::Painter) {
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    (rect, p)
}

/// The picture on the left of `rect` and the settings column on the right.
pub(super) fn split(rect: Rect) -> (Rect, Rect) {
    let side = Rect::from_min_max(pos2(rect.right() - SIDE - 4.0, rect.top() + 4.0), pos2(rect.right() - 4.0, rect.bottom() - 4.0));
    let pic = Rect::from_min_max(rect.min + vec2(4.0, 4.0), pos2(side.left() - 6.0, rect.bottom() - 4.0));
    (pic, side)
}

/// The `i`th setting's place in the settings column.
pub(super) fn slot(side: Rect, i: usize) -> Rect {
    Rect::from_min_size(pos2(side.left(), side.top() + i as f32 * (ROW + 3.0)), vec2(side.width(), ROW))
}

/// A big target for a choice or a count: its name small, its value large;
/// click for the next, right-click for the one before (round the ends).
#[allow(clippy::too_many_arguments)]
pub(super) fn pick(ui: &mut Ui, r: Rect, id: Id, value: &mut f32, min: f32, max: f32, name: &str, label: &str, accent: Color32) -> bool {
    let resp = ui.interact(r, id, Sense::click());
    let p = ui.painter();
    let hot = resp.hovered();
    rect_fill(p, r, 3.0, if hot { mix(PANEL3, accent, 0.25) } else { PANEL3 });
    text(p, pos2(r.left() + 5.0, r.center().y), Align2::LEFT_CENTER, name, small(), TEXT_DIM);
    text(p, pos2(r.right() - 5.0, r.center().y), Align2::RIGHT_CENTER, label, FontId::proportional(12.0), if hot { Color32::WHITE } else { TEXT });
    let step = if resp.clicked() {
        1.0
    } else if resp.secondary_clicked() {
        -1.0
    } else {
        0.0
    };
    resp.on_hover_text(format!("{name}: {label}\nClick: the next, right-click: the one before"));
    if step == 0.0 {
        return false;
    }
    let span = max - min + 1.0;
    *value = min + (value.round() - min + step).rem_euclid(span);
    true
}

/// `pick` for a choice parameter (its labels).
#[allow(clippy::too_many_arguments)]
pub(super) fn pick_choice(ui: &mut Ui, r: Rect, id: Id, params: &mut [f32], i: usize, labels: &[&str], name: &str, accent: Color32) -> bool {
    let k = (params[i].round().max(0.0) as usize).min(labels.len() - 1);
    pick(ui, r, id, &mut params[i], 0.0, (labels.len() - 1) as f32, name, labels[k], accent)
}

/// `pick` for a whole number between `min` and `max`.
#[allow(clippy::too_many_arguments)]
pub(super) fn pick_count(ui: &mut Ui, r: Rect, id: Id, params: &mut [f32], i: usize, min: f32, max: f32, name: &str, accent: Color32) -> bool {
    let label = format!("{}", params[i].round() as i32);
    pick(ui, r, id, &mut params[i], min, max, name, &label, accent)
}

/// A fader lying down: drag or click along it, double-click for its default.
pub(super) fn fader(ui: &mut Ui, r: Rect, id: Id, value: &mut f32, spec: &ParamSpec, color: Color32) -> bool {
    let resp = ui.interact(r, id, Sense::click_and_drag());
    let p = ui.painter();
    let hot = resp.hovered() || resp.dragged();
    rect_fill(p, r, 3.0, PANEL3);
    let mut changed = false;
    if resp.double_clicked() {
        *value = spec.default;
        changed = true;
    } else if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        let v = spec.from_norm(((pt.x - r.left()) / r.width()).clamp(0.0, 1.0));
        if v != *value {
            *value = v;
            changed = true;
        }
    }
    let n = spec.to_norm(*value).clamp(0.0, 1.0);
    let fill = Rect::from_min_size(r.min, vec2(r.width() * n, r.height()));
    rect_fill(p, fill, 3.0, shade(color, if hot { 0.75 } else { 0.55 }));
    p.line_segment([pos2(fill.right(), r.top() + 2.0), pos2(fill.right(), r.bottom() - 2.0)], Stroke::new(2.0_f32, color));
    text(p, pos2(r.left() + 5.0, r.center().y), Align2::LEFT_CENTER, spec.name, small(), TEXT);
    text(p, pos2(r.right() - 5.0, r.center().y), Align2::RIGHT_CENTER, spec.format(*value), small(), TEXT);
    resp.on_hover_text(format!("{}: {}\nDrag or click along it, double-click to reset", spec.name, spec.format(*value)));
    changed
}

/// A fader for parameter `i` of `specs`.
pub(super) fn fader_at(ui: &mut Ui, r: Rect, id: Id, params: &mut [f32], specs: &[ParamSpec], i: usize, color: Color32) -> bool {
    fader(ui, r, id.with(("fader", i)), &mut params[i], &specs[i], color)
}

/// A button that rolls a seed on (a new scatter, a new row...).
#[allow(clippy::too_many_arguments)]
pub(super) fn roll(ui: &mut Ui, r: Rect, id: Id, params: &mut [f32], i: usize, max: f32, label: &str, tip: &str, accent: Color32) -> bool {
    if button(ui, r, id, label, false, accent).on_hover_text(tip).clicked() {
        params[i] = (params[i].round() + 1.0).rem_euclid(max + 1.0);
        return true;
    }
    false
}

/// A fixed place for item `i` from `salt` (0..1 each way).
pub(super) fn spot(salt: u32, i: u32) -> (f32, f32) {
    let h = |a: u32| {
        let mut x = a.wrapping_mul(0x9E37_79B9) ^ i.wrapping_mul(0x85EB_CA6B) ^ salt;
        x ^= x >> 15;
        x = x.wrapping_mul(0x2C1B_3C6D);
        x ^= x >> 12;
        (x >> 8) as f32 / 16_777_216.0
    };
    (h(1), h(2))
}

fn choice(params: &[f32], i: usize, n: usize) -> usize {
    (params[i].round().max(0.0) as usize).min(n - 1)
}

// ---------------------------------------------------------------- fireflies

/// The meadow: each firefly glows as it flashes and fades; the bar below is
/// how together they are.
pub(crate) fn fireflies(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 330.0, height);
    let (pic, side) = split(rect);
    let n = (params[na::FF_COUNT].round() as usize).clamp(2, na::MAX_FIREFLIES);
    let vis = live.on().map(|v| na::unpack(&v));
    let night = Rect::from_min_max(pic.min, pos2(pic.right(), pic.bottom() - 16.0));
    rect_fill(&p, night, 3.0, Color32::from_rgb(0x10, 0x14, 0x1c));
    // Grass along the bottom of the night.
    for g in 0..24 {
        let x = night.left() + (g as f32 + 0.5) / 24.0 * night.width();
        let h = 4.0 + 6.0 * spot(5, g).0;
        p.line_segment([pos2(x, night.bottom()), pos2(x + 2.0, night.bottom() - h)], Stroke::new(1.0_f32, Color32::from_rgb(0x24, 0x3a, 0x2a)));
    }
    let glow_col = Color32::from_rgb(0xd8, 0xff, 0x6a);
    for i in 0..n {
        let (sx, sy) = spot(params[na::FF_SEED].round() as u32 + 3, i as u32);
        let c = pos2(night.left() + 10.0 + sx * (night.width() - 20.0), night.top() + 10.0 + sy * (night.height() - 26.0));
        let phase = vis.map(|b| b[i] as f32 / 256.0).unwrap_or(0.5);
        // Bright just after a flash, then dimming.
        let glow = if vis.is_some() { (-phase * 9.0).exp() } else { 0.0 };
        if glow > 0.05 {
            p.circle_filled(c, 4.0 + 9.0 * glow, glow_col.gamma_multiply(0.18 * glow));
        }
        p.circle_filled(c, 2.5, mix(Color32::from_rgb(0x40, 0x48, 0x30), glow_col, glow.max(0.15)));
    }
    let order = vis.map(|b| b[13] as f32 / 255.0).unwrap_or(0.0);
    let bar = Rect::from_min_max(pos2(pic.left(), pic.bottom() - 10.0), pic.max);
    rect_fill(&p, bar, 2.0, PANEL3);
    rect_fill(&p, Rect::from_min_size(bar.min, vec2(bar.width() * order, bar.height())), 2.0, glow_col.gamma_multiply(0.7));
    text(&p, pos2(bar.left() + 3.0, bar.center().y), Align2::LEFT_CENTER, if vis.is_some() { format!("together {:.0}%", order * 100.0) } else { "together".into() }, small(), TEXT_DARK);
    let specs = na::FIREFLIES;
    let mut changed = pick_count(ui, slot(side, 0), id.with("ff_n"), params, na::FF_COUNT, 2.0, na::MAX_FIREFLIES as f32, "Fireflies", accent);
    changed |= pick_choice(ui, slot(side, 1), id.with("ff_pulse"), params, na::FF_PULSE, na::PULSES, "Pulse", accent);
    changed |= fader_at(ui, slot(side, 2), id, params, specs, na::FF_COUPLING, accent);
    changed |= fader_at(ui, slot(side, 3), id, params, specs, na::FF_SPREAD, accent);
    changed |= fader_at(ui, slot(side, 4), id, params, specs, na::FF_NUDGE, accent);
    changed |= pick_choice(ui, slot(side, 5), id.with("ff_notes"), params, na::FF_LAYOUT, na::LAYOUTS, "Notes", accent);
    changed |= roll(ui, slot(side, 6), id.with("ff_seed"), params, na::FF_SEED, 99.0, "Scatter", "Scatter them over the meadow again (Seed)", accent);
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    changed
}

// ---------------------------------------------------------------- sandpile

/// The board: the more grains on a cell the brighter; cells toppling now
/// white; the spot where grains fall marked.
pub(crate) fn sandpile(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 330.0, height);
    let (pic, side) = split(rect);
    let vis = live.on();
    let side_n = vis.map(|v| (v[3] >> 8 & 0xf) as usize).unwrap_or(params[na::SP_SIZE].round() as usize).clamp(4, na::MAX_SIDE);
    let cs = (pic.width().min(pic.height() - 14.0) / side_n as f32).floor();
    let board = Rect::from_min_size(pos2(pic.center().x - cs * side_n as f32 / 2.0, pic.top()), vec2(cs * side_n as f32, cs * side_n as f32));
    let sand = Color32::from_rgb(0xe8, 0xc0, 0x7a);
    for y in 0..side_n {
        for x in 0..side_n {
            let i = y * na::MAX_SIDE + x;
            let c = Rect::from_min_size(pos2(board.left() + x as f32 * cs, board.top() + y as f32 * cs), vec2(cs, cs)).shrink(1.0);
            let h = vis.map(|v| (v[i / 32] >> ((i % 32) * 2) & 3) as f32).unwrap_or(0.0);
            let toppled = vis.is_some_and(|v| v[2] >> i & 1 == 1);
            let fill = if toppled { Color32::WHITE } else { mix(PANEL3, sand, h / 3.0 * 0.85) };
            rect_fill(&p, c, 2.0, fill);
            if h > 0.0 && !toppled {
                // Grains as dots.
                for g in 0..h as usize {
                    let q = c.left_top() + vec2(c.width() * (0.3 + 0.2 * g as f32), c.height() * 0.5);
                    p.circle_filled(q, (cs * 0.07).max(1.0), shade(sand, 0.45));
                }
            }
        }
    }
    if let Some(v) = vis {
        let (sx, sy) = ((v[3] & 0xf) as f32, (v[3] >> 4 & 0xf) as f32);
        let c = pos2(board.left() + (sx + 0.5) * cs, board.top() + (sy + 0.5) * cs);
        p.circle_stroke(c, cs * 0.42, Stroke::new(1.5_f32, accent));
        let wave = v[3] >> 12 & 0xff;
        let size = v[3] >> 20 & 0xfff;
        let what = if wave > 0 { format!("avalanche · wave {wave} · {size} cells") } else { "still".to_string() };
        text(&p, pos2(pic.left(), pic.bottom()), Align2::LEFT_BOTTOM, what, small(), TEXT_DIM);
        ui.ctx().request_repaint();
    }
    let specs = na::SANDPILE;
    let mut changed = pick_count(ui, slot(side, 0), id.with("sp_size"), params, na::SP_SIZE, 4.0, na::MAX_SIDE as f32, "Size", accent);
    changed |= fader_at(ui, slot(side, 1), id, params, specs, na::SP_DROP, accent);
    changed |= pick_choice(ui, slot(side, 2), id.with("sp_spot"), params, na::SP_SPOT, na::SPOTS, "Spot", accent);
    changed |= pick_choice(ui, slot(side, 3), id.with("sp_play"), params, na::SP_PLAY, na::SAND_PLAYS, "Play", accent);
    changed |= pick_count(ui, slot(side, 4), id.with("sp_voices"), params, na::SP_VOICES, 1.0, 6.0, "Voices", accent);
    changed |= pick_choice(ui, slot(side, 5), id.with("sp_start"), params, na::SP_START, na::STARTS, "Start", accent);
    changed |= roll(ui, slot(side, 6), id.with("sp_seed"), params, na::SP_SEED, 99.0, "New Pile", "Start the pile again (Seed)", accent);
    changed
}

// ---------------------------------------------------------------- pendulums

/// The pendulums from above, a row each, longest at the top: each bob swings
/// across the middle line. A bob that plays flashes white and swells, a ring
/// goes out from where it played and its string glows, fading in under half
/// a second. Where in the cycle it is under them; two columns of settings:
/// the wave and the notes.
pub(crate) fn pendulums(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 480.0, height);
    let col_w = 128.0;
    let cols = [
        Rect::from_min_max(pos2(rect.right() - 2.0 * col_w - 8.0, rect.top() + 4.0), pos2(rect.right() - col_w - 8.0, rect.bottom() - 4.0)),
        Rect::from_min_max(pos2(rect.right() - col_w - 4.0, rect.top() + 4.0), pos2(rect.right() - 4.0, rect.bottom() - 4.0)),
    ];
    let pic = Rect::from_min_max(rect.min + vec2(4.0, 4.0), pos2(cols[0].left() - 6.0, rect.bottom() - 4.0));
    // As many rows of settings as the height holds (nine at most).
    let row = ((cols[0].height() + 3.0) / 9.0 - 3.0).clamp(14.0, ROW);
    let at = |c: usize, k: usize| Rect::from_min_size(pos2(cols[c].left(), cols[c].top() + k as f32 * (row + 3.0)), vec2(cols[c].width(), row));
    let n = (params[na::PE_COUNT].round() as usize).clamp(2, na::MAX_PENDULUMS);
    let vis = live.on();
    let pos = vis.map(|v| (v[0] & 0xffff) as f32 / 65536.0).unwrap_or(0.0);
    // Which played since the screen last looked: their counts moved on. Each
    // keeps when it last played, and where.
    let now = ui.input(|i| i.time);
    let mem = id.with("pe_flash");
    let (mut counts, mut flash) = ui.ctx().memory(|m| m.data.get_temp::<([u8; 16], [(f64, f32); 16])>(mem)).unwrap_or(([0; 16], [(f64::NEG_INFINITY, 0.0); 16]));
    let area = Rect::from_min_max(pic.min, pos2(pic.right(), pic.bottom() - 16.0));
    let mid = area.center().x;
    p.line_segment([pos2(mid, area.top()), pos2(mid, area.bottom())], Stroke::new(1.0_f32, LINE));
    let rh = area.height() / n as f32;
    let amp = area.width() * 0.44;
    const GLOW: f64 = 0.45;
    let mut glowing = false;
    for i in 0..n {
        let swings = na::pendulum_swings(params, i) as f32;
        let x = mid - amp * (TAU * swings * pos).cos();
        let y = area.top() + (i as f32 + 0.5) * rh;
        if let Some(v) = vis {
            let c = ((v[1] >> (4 * i)) & 0xf) as u8;
            if c != counts[i] {
                counts[i] = c;
                flash[i] = (now, x);
            }
        }
        // How far into its flash (1 just played, 0 done).
        let f = (1.0 - (now - flash[i].0) / GLOW).clamp(0.0, 1.0) as f32;
        glowing |= f > 0.0;
        let col = mix(accent, SELECT, i as f32 / n as f32);
        let r = (rh * 0.38).clamp(2.5, 6.0);
        // The string glows as it plays.
        p.line_segment([pos2(mid, y), pos2(x, y)], Stroke::new(1.0 + 1.5 * f, mix(LINE_SOFT, col, f)));
        if f > 0.0 {
            // A ring goes out from where it played.
            let at = pos2(flash[i].1, y);
            let grow = 1.0 - f;
            p.circle_stroke(at, r + grow * (rh * 1.4).clamp(8.0, 22.0), Stroke::new(1.5 * f + 0.5, Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), (200.0 * f) as u8)));
            p.circle_filled(pos2(x, y), r * (1.0 + 1.2 * f), Color32::from_rgba_unmultiplied(255, 255, 255, (90.0 * f) as u8));
        }
        p.circle_filled(pos2(x, y), r * (1.0 + 0.5 * f * f), mix(col, Color32::WHITE, f));
    }
    if vis.is_none() {
        counts = [0; 16];
    }
    ui.ctx().memory_mut(|m| m.data.insert_temp(mem, (counts, flash)));
    if glowing {
        ui.ctx().request_repaint();
    }
    // Where in the cycle: they are all together again at its end.
    let bar = Rect::from_min_max(pos2(pic.left(), pic.bottom() - 8.0), pic.max);
    rect_fill(&p, bar, 2.0, PANEL3);
    rect_fill(&p, Rect::from_min_size(bar.min, vec2(bar.width() * pos, bar.height())), 2.0, shade(accent, 0.7));
    let bars = na::pendulum_cycle(params) / 4.0;
    let label = format!("bar {} of {}", (pos as f64 * bars).floor() as i64 + 1, bars.round() as i64);
    text(&p, pos2(pic.right(), bar.top() - 2.0), Align2::RIGHT_BOTTOM, label, small(), TEXT_DIM);
    let specs = na::PENDULUMS;
    // The wave.
    let mut changed = pick_count(ui, at(0, 0), id.with("pe_n"), params, na::PE_COUNT, 2.0, na::MAX_PENDULUMS as f32, "Pendulums", accent);
    changed |= pick_choice(ui, at(0, 1), id.with("pe_cycle"), params, na::PE_CYCLE, na::CYCLES, "Cycle", accent);
    changed |= pick_count(ui, at(0, 2), id.with("pe_slow"), params, na::PE_SLOW, 1.0, 8.0, "Slower ×", accent);
    changed |= pick_count(ui, at(0, 3), id.with("pe_swings"), params, na::PE_SWINGS, 1.0, 24.0, "Swings", accent);
    changed |= pick_choice(ui, at(0, 4), id.with("pe_play"), params, na::PE_PLAY, na::PENDULUM_PLAYS, "Play", accent);
    changed |= pick_choice(ui, at(0, 5), id.with("pe_sync"), params, na::PE_SYNC, krabton_engine::sequencers::more::SYNCS, "Sync", accent);
    changed |= fader_at(ui, at(0, 6), id, params, specs, na::PE_SPREAD, accent);
    changed |= fader_at(ui, at(0, 7), id, params, specs, na::PE_WOBBLE, accent);
    changed |= fader_at(ui, at(0, 8), id, params, specs, na::PE_FADE, accent);
    // The notes.
    changed |= pick_choice(ui, at(1, 0), id.with("pe_notes"), params, na::PE_LAYOUT, na::LAYOUTS, "Notes", accent);
    changed |= pick_count(ui, at(1, 1), id.with("pe_steps"), params, na::PE_STEPS, 1.0, 4.0, "Steps", accent);
    changed |= pick_count(ui, at(1, 2), id.with("pe_other"), params, na::PE_OTHER, -7.0, 7.0, "Other Side", accent);
    changed |= pick_choice(ui, at(1, 3), id.with("pe_hold"), params, na::PE_HOLD, &["Off", "On"], "Hold Swing", accent);
    changed |= fader_at(ui, at(1, 4), id, params, specs, na::PE_TOGETHER, accent);
    changed |= fader_at(ui, at(1, 5), id, params, specs, na::PE_CHANCE, accent);
    changed |= fader_at(ui, at(1, 6), id, params, specs, na::PE_TILT, accent);
    changed |= fader_at(ui, at(1, 7), id, params, specs, na::PE_GATE, accent);
    changed |= fader_at(ui, at(1, 8), id, params, specs, na::PE_VELOCITY, accent);
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    changed
}

// ---------------------------------------------------------------- flock

/// The birds over the field, higher is higher, and the lines they play on.
pub(crate) fn flock(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 330.0, height);
    let (pic, side) = split(rect);
    rect_fill(&p, pic, 3.0, Color32::from_rgb(0x16, 0x1c, 0x26));
    let lines = (params[na::FL_LINES].round() as usize).clamp(1, 8);
    for l in 0..lines {
        let x = pic.left() + (l as f32 + 0.5) / lines as f32 * pic.width();
        p.line_segment([pos2(x, pic.top() + 2.0), pos2(x, pic.bottom() - 2.0)], Stroke::new(1.0_f32, shade(accent, 0.45)));
    }
    if let Some(v) = live.on() {
        let (_, _, birds) = na::birds_from_visual(&v);
        for (x, y, h) in birds {
            let c = pos2(pic.left() + x * pic.width(), pic.bottom() - y * pic.height());
            let a = h * TAU;
            let (dx, dy) = (a.cos(), -a.sin());
            let tip = c + vec2(dx, dy) * 6.0;
            let l = c + vec2(-dx * 4.0 - dy * 3.0, -dy * 4.0 + dx * 3.0);
            let r = c + vec2(-dx * 4.0 + dy * 3.0, -dy * 4.0 - dx * 3.0);
            p.add(Shape::convex_polygon(vec![tip, l, r], Color32::from_rgb(0xe8, 0xec, 0xf4), Stroke::NONE));
        }
        ui.ctx().request_repaint();
    } else {
        text(&p, pic.center(), Align2::CENTER_CENTER, "the flock flies while it plays", small(), TEXT_DIM);
    }
    let specs = na::FLOCK;
    let mut changed = pick_count(ui, slot(side, 0), id.with("fl_n"), params, na::FL_BIRDS, 2.0, na::MAX_BIRDS as f32, "Birds", accent);
    changed |= pick_count(ui, slot(side, 1), id.with("fl_lines"), params, na::FL_LINES, 1.0, 8.0, "Lines", accent);
    for (r, i) in [na::FL_SEPARATION, na::FL_ALIGNMENT, na::FL_COHESION, na::FL_WANDER, na::FL_SPEED].into_iter().enumerate() {
        changed |= fader_at(ui, slot(side, 2 + r), id, params, specs, i, accent);
    }
    changed
}

// ---------------------------------------------------------------- plants

/// The plant drawn as its rules grow it (a turtle: F forward, + and - a
/// turn, [ ] a branch), lit as far as it has played.
pub(crate) fn plants(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 330.0, height);
    let (pic, side) = split(rect);
    let which = choice(params, na::PL_PLANT, na::PLANTS.len());
    let sys = &na::LSYSTEMS[which];
    let vis = live.on();
    let gens = vis.map(|v| (v[1] & 0xff) as usize).filter(|g| *g > 0).unwrap_or(params[na::PL_GENERATIONS].round() as usize).clamp(1, 7);
    let at = vis.map(|v| v[0] as usize).unwrap_or(usize::MAX);
    let s = sys.grow(gens, 6000);
    let green = Color32::from_rgb(0x7c, 0xd0, 0x6a);
    if sys.angle == 0.0 {
        // Algae: its cells in a row, wrapped.
        let per = 16usize;
        let cw = pic.width() / per as f32;
        for (i, &c) in s.iter().enumerate().take(per * 8) {
            let r = Rect::from_min_size(pos2(pic.left() + (i % per) as f32 * cw, pic.top() + (i / per) as f32 * cw), vec2(cw - 2.0, cw - 2.0));
            let col = if i + 1 == at { Color32::WHITE } else if i < at { green } else { shade(green, 0.35) };
            if c == b'A' {
                rect_fill(&p, r, 2.0, col);
            } else {
                rect_line(&p, r, 2.0, 1.5, col);
            }
        }
    } else {
        let mut segs: Vec<(egui::Pos2, egui::Pos2, usize)> = Vec::new();
        let (mut x, mut y, mut a) = (0.0f32, 0.0f32, 90.0f32);
        let mut stack = Vec::new();
        for (i, &c) in s.iter().enumerate() {
            match c {
                b'F' | b'G' | b'A' | b'B' => {
                    let (nx, ny) = (x + a.to_radians().cos(), y + a.to_radians().sin());
                    segs.push((pos2(x, y), pos2(nx, ny), i));
                    (x, y) = (nx, ny);
                }
                b'+' => a += sys.angle,
                b'-' => a -= sys.angle,
                b'[' => stack.push((x, y, a)),
                b']' => {
                    if let Some(t) = stack.pop() {
                        (x, y, a) = t;
                    }
                }
                _ => {}
            }
        }
        if !segs.is_empty() {
            let (mut lo, mut hi) = (pos2(f32::MAX, f32::MAX), pos2(f32::MIN, f32::MIN));
            for (a, b, _) in &segs {
                lo = lo.min(a.min(*b));
                hi = hi.max(a.max(*b));
            }
            let scale = (pic.width() / (hi.x - lo.x).max(1e-3)).min((pic.height() - 12.0) / (hi.y - lo.y).max(1e-3));
            let off = vec2(pic.center().x - (lo.x + hi.x) / 2.0 * scale, 0.0);
            let to = |q: egui::Pos2| pos2(q.x * scale + off.x, pic.bottom() - 12.0 - (q.y - lo.y) * scale);
            for (a, b, i) in &segs {
                let col = if *i + 1 == at { Color32::WHITE } else if *i < at { green } else { shade(green, 0.3) };
                p.line_segment([to(*a), to(*b)], Stroke::new(if *i + 1 == at { 2.5_f32 } else { 1.2 }, col));
            }
        }
    }
    let total = na::plant_length(which, gens);
    text(&p, pos2(pic.left(), pic.bottom()), Align2::LEFT_BOTTOM, format!("generation {gens} · {total} symbols"), small(), TEXT_DIM);
    let mut changed = pick_choice(ui, slot(side, 0), id.with("pl_plant"), params, na::PL_PLANT, na::PLANTS, "Plant", accent);
    changed |= pick_count(ui, slot(side, 1), id.with("pl_gen"), params, na::PL_GENERATIONS, 1.0, 7.0, "Grown", accent);
    let on = if params[na::PL_GROW] >= 0.5 { "On" } else { "Off" };
    changed |= pick(ui, slot(side, 2), id.with("pl_grow"), &mut params[na::PL_GROW], 0.0, 1.0, "Grow", on, accent);
    changed |= pick_count(ui, slot(side, 3), id.with("pl_turn"), params, na::PL_TURN, 1.0, 4.0, "Turn", accent);
    changed |= pick_choice(ui, slot(side, 4), id.with("pl_x"), params, na::PL_X, na::X_MODES, "X", accent);
    changed |= fader_at(ui, slot(side, 5), id, params, na::PLANT, na::PL_BRANCH, accent);
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    changed
}
