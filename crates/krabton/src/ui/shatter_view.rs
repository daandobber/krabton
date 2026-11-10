//! Shatter's face: black glass, cracked. The motif is a row of shards in
//! the key (click a place to put a note there or take it away, right-click
//! a note for an accent, double-click to hold it over the rests after it),
//! each edged in cyan and magenta as a broken screen is, the step playing
//! flashing white. Under it the phrase: a tile for every bar with what it
//! does to the motif, drawn as its shape (click for the moves). Over it
//! the mood, Write, Mutate and the dice; beside it what is held down: Run,
//! Stutter, Up, Cut.

use super::seq_view::Live;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Pos2, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::project::note_name;
use krabton_engine::sequencers::shatter::{self as sh, *};

const GLASS: Color32 = Color32::from_rgb(0x06, 0x06, 0x0a);
const CRACK: Color32 = Color32::from_rgb(0x2a, 0x2c, 0x38);
const CYAN: Color32 = Color32::from_rgb(0x3e, 0xf3, 0xff);
const MAGENTA: Color32 = Color32::from_rgb(0xff, 0x2b, 0xd6);
const WHITE: Color32 = Color32::from_rgb(0xf4, 0xf6, 0xff);
const DIM: Color32 = Color32::from_rgb(0x7c, 0x80, 0x96);
/// The degrees the motif can be drawn in (from the centre).
const LOW: i32 = -5;
const HIGH: i32 = 12;

fn mono(s: f32) -> FontId {
    FontId::monospace(s)
}

fn hash(a: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65536.0
}

/// A shard of glass in `r`, the same shape every time for `seed`.
fn shard(r: Rect, seed: u32) -> Vec<Pos2> {
    let j = |k: u32, s: f32| (hash(seed * 31 + k) - 0.5) * s;
    vec![
        pos2(r.left() + r.width() * (0.1 + j(1, 0.2)), r.top() + r.height() * (0.3 + j(2, 0.3))),
        pos2(r.left() + r.width() * (0.55 + j(3, 0.3)), r.top() + r.height() * (0.0 + j(4, 0.1)).max(0.0)),
        pos2(r.right() - r.width() * (0.05 + j(5, 0.1)).max(0.0), r.top() + r.height() * (0.55 + j(6, 0.3))),
        pos2(r.left() + r.width() * (0.4 + j(7, 0.3)), r.bottom() - r.height() * (0.02 + j(8, 0.1)).max(0.0)),
    ]
}

/// Drawn as a broken screen draws it: magenta one way, cyan the other, `core` between.
fn glitched(p: &egui::Painter, pts: &[Pos2], core: Color32, by: f32) {
    let moved = |d: f32| pts.iter().map(|q| *q + vec2(d, 0.0)).collect::<Vec<_>>();
    p.add(Shape::convex_polygon(moved(-by), MAGENTA.gamma_multiply(0.55), Stroke::NONE));
    p.add(Shape::convex_polygon(moved(by), CYAN.gamma_multiply(0.55), Stroke::NONE));
    p.add(Shape::convex_polygon(pts.to_vec(), core, Stroke::new(1.0_f32, WHITE.gamma_multiply(0.5))));
}

/// The cracks across the glass.
fn cracks(p: &egui::Painter, r: Rect) {
    let c = pos2(r.left() + r.width() * 0.72, r.top() + r.height() * 0.38);
    for k in 0..9u32 {
        let a = k as f32 * 0.71 + hash(k) * 0.5;
        let mut at = c;
        let mut pts = vec![at];
        for s in 0..4u32 {
            let len = 18.0 + hash(k * 7 + s) * 40.0;
            let turn = (hash(k * 13 + s) - 0.5) * 0.6;
            at += vec2((a + turn).cos(), (a + turn).sin()) * len;
            pts.push(at);
        }
        p.add(Shape::line(pts, Stroke::new(1.0_f32, CRACK)));
    }
}

/// What a move does, drawn small in `r`.
fn move_glyph(p: &egui::Painter, r: Rect, mv: u8, col: Color32) {
    let s = Stroke::new(1.6_f32, col);
    let x = |f: f32| r.left() + r.width() * f;
    let y = |f: f32| r.top() + r.height() * f;
    let line = |pts: Vec<Pos2>| {
        p.add(Shape::line(pts, s));
    };
    match mv {
        PLAY => line(vec![pos2(x(0.1), y(0.7)), pos2(x(0.35), y(0.3)), pos2(x(0.6), y(0.55)), pos2(x(0.9), y(0.35))]),
        ANSWER => {
            line(vec![pos2(x(0.1), y(0.8)), pos2(x(0.4), y(0.6))]);
            line(vec![pos2(x(0.55), y(0.4)), pos2(x(0.9), y(0.2))]);
        }
        CLIMB => line(vec![pos2(x(0.1), y(0.85)), pos2(x(0.3), y(0.85)), pos2(x(0.3), y(0.6)), pos2(x(0.55), y(0.6)), pos2(x(0.55), y(0.35)), pos2(x(0.8), y(0.35)), pos2(x(0.8), y(0.12))]),
        INVERT => {
            line(vec![pos2(x(0.1), y(0.3)), pos2(x(0.5), y(0.7)), pos2(x(0.9), y(0.3))]);
            p.line_segment([pos2(x(0.05), y(0.5)), pos2(x(0.95), y(0.5))], Stroke::new(1.0_f32, col.gamma_multiply(0.4)));
        }
        BACKWARDS => line(vec![pos2(x(0.9), y(0.5)), pos2(x(0.15), y(0.5)), pos2(x(0.35), y(0.3)), pos2(x(0.15), y(0.5)), pos2(x(0.35), y(0.7))]),
        RUN_MOVE => {
            for k in 0..6 {
                let f = 0.1 + k as f32 * 0.14;
                p.line_segment([pos2(x(f), y(0.8 - k as f32 * 0.1)), pos2(x(f + 0.08), y(0.8 - k as f32 * 0.1))], s);
            }
        }
        STUTTER_MOVE => {
            line(vec![pos2(x(0.08), y(0.6)), pos2(x(0.4), y(0.6))]);
            for k in 0..4 {
                let f = 0.5 + k as f32 * 0.11;
                p.line_segment([pos2(x(f), y(0.35)), pos2(x(f), y(0.75))], s);
            }
        }
        LEAP => line(vec![pos2(x(0.1), y(0.8)), pos2(x(0.3), y(0.15)), pos2(x(0.5), y(0.8)), pos2(x(0.7), y(0.15)), pos2(x(0.9), y(0.8))]),
        SHATTER => {
            for k in 0..5u32 {
                let c = pos2(x(0.15 + hash(k) * 0.7), y(0.2 + hash(k + 9) * 0.6));
                p.add(Shape::convex_polygon(vec![c + vec2(-3.0, 2.0), c + vec2(1.0, -3.5), c + vec2(3.5, 2.5)], col, Stroke::NONE));
            }
        }
        _ => {
            p.line_segment([pos2(x(0.2), y(0.5)), pos2(x(0.8), y(0.5))], Stroke::new(1.0_f32, col.gamma_multiply(0.5)));
        }
    }
}

/// A button held down (or right-clicked to stay on), as Shatter lights it.
fn hold_button(ui: &mut Ui, r: Rect, id: Id, label: &str, on: bool, tip: &str) -> (bool, bool) {
    let resp = ui.interact(r, id, Sense::click());
    let down = held(ui, &resp);
    let p = ui.painter();
    let lit = on || down;
    p.rect_filled(r, 3.0, if lit { MAGENTA } else { Color32::from_rgb(0x16, 0x16, 0x20) });
    p.rect_stroke(r, 3.0, Stroke::new(1.0_f32, if lit { WHITE } else { CRACK }), egui::StrokeKind::Inside);
    text(p, r.center() + vec2(-1.0, 0.0), Align2::CENTER_CENTER, label, mono(11.0), if lit { CYAN.gamma_multiply(0.6) } else { Color32::TRANSPARENT });
    text(p, r.center(), Align2::CENTER_CENTER, label, mono(11.0), if lit { GLASS } else { WHITE });
    let latch = resp.secondary_clicked();
    resp.on_hover_text(tip);
    (down, latch)
}

/// Writes the motif and its phrase in `mood` from `seed`, into the settings.
pub(crate) fn write_into(params: &mut [f32], data: &mut Vec<u8>, mood: usize, seed: u32) {
    let w = sh::write(mood, seed);
    *data = w.data;
    params[RATE] = w.rate as f32;
    params[LENGTH] = w.length as f32;
    params[BARS] = w.bars as f32;
    for (k, m) in w.moves.iter().enumerate() {
        params[MOVE0 + k] = *m as f32;
    }
    params[STACK] = w.stack as f32;
    params[CENTER] = w.center as f32;
    params[GATE] = w.gate;
    params[MOOD] = mood as f32;
    params[SEED] = seed as f32;
}

pub(crate) fn shatter(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, height: f32) -> bool {
    if params.len() < SPECS.len() {
        return false;
    }
    if data.len() < 2 * CELLS {
        *data = shatter_default();
    }
    let mut changed = false;
    let vis = live.on();
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    let len = (params[LENGTH].round() as usize).clamp(2, CELLS);
    let bars = [1usize, 2, 4, 8][(params[BARS].round().max(0.0) as usize).min(3)];
    let w = ui.available_width().clamp(520.0, 1100.0);
    let h = height.max(200.0);
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let p = ui.painter_at(rect);
    rect_fill(&p, rect, 6.0, GLASS);
    cracks(&p, rect);

    // The head: its name, the mood, Write, Mutate, the dice.
    let head = Rect::from_min_size(rect.min + vec2(10.0, 6.0), vec2(rect.width() - 20.0, 24.0));
    text(&p, head.left_center() + vec2(-1.5, 0.0), Align2::LEFT_CENTER, "SHATTER", FontId::proportional(19.0), MAGENTA.gamma_multiply(0.8));
    text(&p, head.left_center() + vec2(1.5, 0.0), Align2::LEFT_CENTER, "SHATTER", FontId::proportional(19.0), CYAN.gamma_multiply(0.8));
    text(&p, head.left_center(), Align2::LEFT_CENTER, "SHATTER", FontId::proportional(19.0), WHITE);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(head.left() + 120.0, head.top()), head.max)).layout(egui::Layout::left_to_right(egui::Align::Center)));
    {
        let ui = &mut child;
        ui.spacing_mut().item_spacing.x = 5.0;
        let mood = (params[MOOD].round().max(0.0) as usize).min(MOODS.len() - 1);
        egui::ComboBox::from_id_salt(id.with("mood")).width(96.0).selected_text(MOODS[mood]).show_ui(ui, |ui| {
            for (k, m) in MOODS.iter().enumerate() {
                if ui.selectable_label(k == mood, *m).clicked() {
                    params[MOOD] = k as f32;
                    changed = true;
                }
            }
        });
        let write = egui::Button::new(egui::RichText::new("Write").color(GLASS).strong()).fill(CYAN);
        if ui.add(write).on_hover_text("A new motif and phrase in this mood (from a new seed)").clicked() {
            let seed = (params[SEED].round() as u32 + 1 + (ui.input(|i| i.time) * 1000.0) as u32 % 97) % 1000;
            write_into(params, data, mood, seed);
            changed = true;
        }
        if ui.button("Mutate").on_hover_text("A few of the motif's notes changed, the rest kept").clicked() {
            sh::mutate(data, len, (ui.input(|i| i.time) * 1000.0) as u32);
            changed = true;
        }
        ui.label(egui::RichText::new("Rate").small().color(DIM));
        let rates = krabton_engine::sequencers::RATES;
        let ri = (params[RATE].round().max(0.0) as usize).min(rates.len() - 1);
        egui::ComboBox::from_id_salt(id.with("rate")).width(58.0).selected_text(rates[ri]).show_ui(ui, |ui| {
            for (k, r) in rates.iter().enumerate() {
                if ui.selectable_label(k == ri, *r).clicked() {
                    params[RATE] = k as f32;
                    changed = true;
                }
            }
        });
        ui.label(egui::RichText::new("Stack").small().color(DIM));
        let si = (params[STACK].round().max(0.0) as usize).min(STACKS.len() - 1);
        egui::ComboBox::from_id_salt(id.with("stack")).width(64.0).selected_text(STACKS[si]).show_ui(ui, |ui| {
            for (k, s) in STACKS.iter().enumerate() {
                if ui.selectable_label(k == si, *s).on_hover_text("On every note: a fifth, a triad in the key, an octave or a power chord (rave stabs)").clicked() {
                    params[STACK] = k as f32;
                    changed = true;
                }
            }
        });
    }

    // The holds, on the right of the head.
    let holds = [(RUN, "RUN", "Held: the motif twice as fast (right-click: stays on)"), (STUTTER, "STUT", "Held: every step stutters four times"), (UP, "UP", "Held: an octave higher"), (CUT, "CUT", "Held: a hole (for the break)")];
    for (k, (pi, label, tip)) in holds.into_iter().enumerate() {
        let r = Rect::from_min_size(pos2(head.right() - (4 - k) as f32 * 48.0, head.top() + 1.0), vec2(44.0, 22.0));
        let latch_id = id.with(("latch", pi));
        let latched: bool = ui.data(|d| d.get_temp(latch_id)).unwrap_or(false);
        let (down, latch) = hold_button(ui, r, id.with(("hold", pi)), label, latched, tip);
        let mut l = latched;
        if latch {
            l = !l;
            ui.data_mut(|d| d.insert_temp(latch_id, l));
        }
        let want = if down || l { 1.0 } else { 0.0 };
        if params[pi] != want {
            params[pi] = want;
            changed = true;
        }
    }

    // The motif: shards in the key.
    let moves_h = 46.0;
    let foot_h = 26.0;
    let grid = Rect::from_min_max(pos2(rect.left() + 44.0, head.bottom() + 8.0), pos2(rect.right() - 10.0, rect.bottom() - moves_h - foot_h - 12.0));
    let rows = (HIGH - LOW + 1) as usize;
    let cw = grid.width() / len as f32;
    let rh = grid.height() / rows as f32;
    let row_y = |deg: i32| grid.bottom() - (deg - LOW) as f32 * rh - rh * 0.5;
    let (home, _) = live.scale.locate(params[CENTER].round() as i32);
    // Rows: the centre and its octaves lit, names on the left.
    let sl = live.scale.len() as i32;
    for d in LOW..=HIGH {
        let y = row_y(d);
        let key_row = d.rem_euclid(sl.max(1)) == 0;
        if key_row {
            p.line_segment([pos2(grid.left(), y), pos2(grid.right(), y)], Stroke::new(1.0_f32, CRACK.gamma_multiply(1.6)));
        }
        if key_row || d == LOW || d == HIGH {
            let pitch = live.scale.pitch_at(home + d).clamp(0, 127) as u8;
            text(&p, pos2(grid.left() - 6.0, y), Align2::RIGHT_CENTER, note_name(pitch), mono(9.0), if key_row { WHITE } else { DIM });
        }
    }
    let now_step = vis.map(|v| v[2] as usize);
    let sounding = vis.is_some_and(|v| v[3] == 1);
    for c in 0..len {
        let x0 = grid.left() + c as f32 * cw;
        if now_step == Some(c) {
            p.rect_filled(Rect::from_min_size(pos2(x0, grid.top()), vec2(cw, grid.height())), 0.0, Color32::from_white_alpha(10));
        }
        if c % 4 == 0 {
            p.line_segment([pos2(x0, grid.top()), pos2(x0, grid.bottom())], Stroke::new(1.0_f32, CRACK));
        }
        if let Some((d, f)) = cell(data, c) {
            let y = row_y(d.clamp(LOW, HIGH));
            let big = if f & ACCENT != 0 { 1.0 } else { 0.78 };
            let r = Rect::from_center_size(pos2(x0 + cw * 0.5, y), vec2((cw - 4.0).min(34.0) * big, rh * 2.2 * big));
            // Held: a trail of glass to the next note.
            if f & HOLD != 0 {
                let next = (1..len).find(|g| cell(data, c + g).is_some() && c + g < len).unwrap_or(len - c);
                p.line_segment([pos2(r.right(), y), pos2(x0 + cw * next as f32, y)], Stroke::new(2.0_f32, CYAN.gamma_multiply(0.5)));
            }
            let lit = now_step == Some(c) && sounding;
            glitched(&p, &shard(r, c as u32 * 7 + d as u32), if lit { WHITE } else if f & ACCENT != 0 { MAGENTA } else { Color32::from_rgb(0x8a, 0x92, 0xc8) }, if lit { 3.0 } else { 1.4 });
        }
    }
    // Clicking the glass.
    let resp = ui.interact(grid, id.with("motif"), Sense::click());
    if let Some(q) = resp.interact_pointer_pos().or(resp.hover_pos()) {
        let c = (((q.x - grid.left()) / cw).floor() as usize).min(len - 1);
        let d = (LOW + ((grid.bottom() - q.y) / rh).floor() as i32).clamp(LOW, HIGH);
        let here = cell(data, c);
        if resp.double_clicked() {
            if let Some((dd, f)) = here {
                set_cell(data, c, Some((dd, f ^ HOLD)));
                changed = true;
            }
        } else if resp.clicked() {
            set_cell(data, c, if here.is_some_and(|h| h.0 == d) { None } else { Some((d, here.map_or(0, |h| h.1))) });
            changed = true;
        } else if resp.secondary_clicked() {
            if let Some((dd, f)) = here {
                set_cell(data, c, Some((dd, f ^ ACCENT)));
                changed = true;
            }
        }
        let pitch = live.scale.pitch_at(home + d).clamp(0, 127) as u8;
        resp.on_hover_text(format!("Step {}, {}: click to put a note here (or take it away), right-click a note for an accent, double-click to hold it over the rests after it", c + 1, note_name(pitch)));
    }

    // The phrase: a tile a bar.
    let mrow = Rect::from_min_max(pos2(grid.left(), grid.bottom() + 8.0), pos2(grid.right(), grid.bottom() + 8.0 + moves_h));
    let tw = mrow.width() / 8.0;
    let now_bar = vis.map(|v| v[1] as usize);
    for b in 0..8 {
        let r = Rect::from_min_size(pos2(mrow.left() + b as f32 * tw, mrow.top()), vec2(tw - 4.0, moves_h));
        let used = b < bars;
        let mv = (params[MOVE0 + b].round().max(0.0) as u8).min(REST_MOVE);
        let playing = now_bar == Some(b) && used;
        p.rect_filled(r, 3.0, if playing { Color32::from_rgb(0x24, 0x10, 0x2a) } else { Color32::from_rgb(0x10, 0x10, 0x18) });
        p.rect_stroke(r, 3.0, Stroke::new(if playing { 1.6 } else { 1.0_f32 }, if playing { MAGENTA } else { CRACK }), egui::StrokeKind::Inside);
        let col = if !used { DIM.gamma_multiply(0.4) } else if playing { WHITE } else { CYAN };
        move_glyph(&p, Rect::from_min_max(r.min + vec2(6.0, 4.0), pos2(r.right() - 6.0, r.bottom() - 16.0)), mv, col);
        text(&p, pos2(r.center().x, r.bottom() - 7.0), Align2::CENTER_CENTER, MOVES[mv as usize], mono(9.0), if used { WHITE } else { DIM.gamma_multiply(0.5) });
        let tr = ui.interact(r, id.with(("move", b)), Sense::click());
        let tip = if used { format!("Bar {}: what it does to the motif. Click for the moves", b + 1) } else { format!("Bar {}: past the phrase (Bars: {bars})", b + 1) };
        egui::Popup::menu(&tr).show(|ui| {
            for (k, name) in MOVES.iter().enumerate() {
                let about = ["The motif as it is", "A fifth up, as an answer", "Two steps higher every time round", "Upside down", "Backwards", "Twice as fast: a run", "Its end stuttering", "Every other note an octave up", "Its notes in another order, new every time round", "A hole, for the break"][k];
                if ui.selectable_label(k as u8 == mv, *name).on_hover_text(about).clicked() {
                    params[MOVE0 + b] = k as f32;
                    changed = true;
                    ui.close();
                }
            }
        });
        tr.on_hover_text(tip);
    }

    // The foot: length, bars, centre, gate, velocity, vary.
    let foot = Rect::from_min_max(pos2(rect.left() + 10.0, rect.bottom() - foot_h - 4.0), pos2(rect.right() - 10.0, rect.bottom() - 4.0));
    let cells: [(usize, &str, &str); 6] = [
        (LENGTH, "LENGTH", "The motif's steps: click for longer, right-click for shorter"),
        (BARS, "BARS", "The phrase's bars: click for more, right-click for fewer"),
        (CENTER, "CENTRE", "The note in the middle: click a semitone up, right-click down (Shift: an octave)"),
        (GATE, "GATE", "How long each note sounds: drag up or down"),
        (VELOCITY, "VELOCITY", "How hard it plays: drag up or down"),
        (VARY, "VARY", "How much changes every time the phrase comes round: drag up or down"),
    ];
    let fw = foot.width() / cells.len() as f32;
    for (k, (pi, label, tip)) in cells.into_iter().enumerate() {
        let r = Rect::from_min_size(pos2(foot.left() + k as f32 * fw, foot.top()), vec2(fw - 6.0, foot_h));
        let spec = &SPECS[pi];
        let resp = ui.interact(r, id.with(("foot", pi)), Sense::click_and_drag());
        let shift = ui.input(|i| i.modifiers.shift);
        let mut v = params[pi];
        match pi {
            LENGTH | BARS | CENTER => {
                let by = if pi == CENTER && shift { 12.0 } else { 1.0 };
                if resp.clicked() {
                    v += by;
                }
                if resp.secondary_clicked() {
                    v -= by;
                }
            }
            _ => {
                if resp.dragged() {
                    v += -resp.drag_delta().y * (spec.max - spec.min) * if shift { 0.0008 } else { 0.006 };
                }
                if resp.double_clicked() {
                    v = spec.default;
                }
            }
        }
        let v = v.clamp(spec.min, spec.max);
        if v != params[pi] {
            params[pi] = v;
            changed = true;
        }
        p.rect_filled(r, 3.0, Color32::from_rgb(0x10, 0x10, 0x18));
        text(&p, pos2(r.left() + 6.0, r.center().y), Align2::LEFT_CENTER, label, mono(9.0), DIM);
        let shown = match pi {
            LENGTH => format!("{}", v as i32),
            BARS => BAR_COUNTS[(v.round() as usize).min(3)].to_string(),
            CENTER => note_name(v.round().clamp(0.0, 127.0) as u8),
            _ => format!("{:.0} %", v * 100.0),
        };
        text(&p, pos2(r.right() - 6.0, r.center().y), Align2::RIGHT_CENTER, shown, mono(12.0), CYAN);
        resp.on_hover_text(tip);
    }
    changed
}
