//! The Drummer's face: the style to start from (a menu by group), the
//! groove as a grid of two bars by fourteen lanes, where a click cycles a
//! step through hit, accent and ghost (Shift-click: a roll, right-click:
//! off), and what the drummer plays now lit up, its fills and ghost notes
//! included. A lane the kit has no drum for says what it plays instead.

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, RichText, Sense, Ui};
use krabton_engine::classic_seq::gm_drum_name;
use krabton_engine::sequencers::drummer::{self as dr, LANES, STEPS};

const LABEL_W: f32 = 74.0;
const CELL_W: f32 = 13.0;
const BAR_GAP: f32 = 6.0;
const TOP_H: f32 = 24.0;

/// Background bands: drums, hats, cymbals, toms, percussion.
fn band(lane: usize) -> Color32 {
    match lane {
        0..=3 => Color32::from_rgb(0x1d, 0x1d, 0x21),
        4..=6 => Color32::from_rgb(0x22, 0x22, 0x26),
        7..=8 => Color32::from_rgb(0x1d, 0x1d, 0x21),
        9..=11 => Color32::from_rgb(0x22, 0x22, 0x26),
        _ => Color32::from_rgb(0x1d, 0x1d, 0x21),
    }
}

fn rand_seed() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(7) | 1
}

pub(crate) fn drummer(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    if !dr::is_current(data) {
        *data = dr::drummer_default();
        changed = true;
    }
    let rh = ((height - TOP_H - 4.0) / LANES as f32).clamp(8.0, 15.0);
    let grid_w = 2.0 * STEPS as f32 * CELL_W + BAR_GAP;
    let w = LABEL_W + grid_w;
    let (rect, _) = ui.allocate_exact_size(vec2(w, TOP_H + 4.0 + rh * LANES as f32), Sense::hover());
    let p = ui.painter().clone();
    let vis = live.on();
    let two_bars = params[dr::BARS].round() >= 1.0;

    // ---------------------------------------------------------------- the bar on top
    let top = Rect::from_min_size(rect.min, vec2(w, TOP_H - 2.0));
    let mut tui = ui.new_child(egui::UiBuilder::new().max_rect(top).layout(egui::Layout::left_to_right(egui::Align::Center)));
    let style_name = dr::style_of(data).map(|s| dr::STYLES[s].name).unwrap_or("Your Groove");
    let resp = tui.add(egui::Button::new(RichText::new(format!("Style: {style_name}")).color(TEXT_DARK)).fill(accent)).on_hover_text("Start from a style (it replaces the groove)");
    egui::Popup::menu(&resp).show(|ui| {
        for g in dr::STYLE_GROUPS {
            ui.menu_button(*g, |ui| {
                for (i, s) in dr::STYLES.iter().enumerate().filter(|(_, s)| s.group == *g) {
                    if ui.button(s.name).clicked() {
                        *data = dr::style_data(i);
                        params[dr::SWING] = s.swing;
                        params[dr::SWING_ON] = if s.eighths { 1.0 } else { 0.0 };
                        changed = true;
                        ui.close();
                    }
                }
            });
        }
    });
    if tui.small_button("A to B").on_hover_text("Copy bar A over bar B").clicked() {
        for s in 0..STEPS {
            for l in 0..LANES {
                let v = dr::cell(data, 0, s, l);
                dr::set_cell(data, 1, s, l, v);
            }
        }
        changed = true;
    }
    if tui.small_button("Vary B").on_hover_text("Bar B becomes a variation of bar A").clicked() {
        dr::vary(data, rand_seed());
        changed = true;
    }
    if tui.small_button("Clear").on_hover_text("Clear the groove").clicked() {
        for b in 0..2 {
            for s in 0..STEPS {
                for l in 0..LANES {
                    dr::set_cell(data, b, s, l, dr::OFF);
                }
            }
        }
        dr::forget_style(data);
        changed = true;
    }
    if let Some(v) = vis {
        if (v[0] >> 9) & 1 == 1 {
            tui.label(RichText::new("fill").strong().color(accent));
        }
    }

    // ---------------------------------------------------------------- the grid
    let grid = Rect::from_min_size(pos2(rect.left() + LABEL_W, rect.top() + TOP_H + 2.0), vec2(grid_w, rh * LANES as f32));
    let col_x = |bar: usize, step: usize| grid.left() + (bar * STEPS + step) as f32 * CELL_W + if bar == 1 { BAR_GAP } else { 0.0 };
    let now = vis.map(|v| ((v[0] >> 8) & 1) as usize * STEPS + (v[0] & 0xff) as usize);
    let lit = vis.map(|v| v[1]).unwrap_or(0);
    for lane in 0..LANES {
        let y = grid.top() + lane as f32 * rh;
        let row = Rect::from_min_size(pos2(rect.left(), y), vec2(w, rh));
        rect_fill(&p, row, 0.0, band(lane));
        // The label, and what the kit plays for it.
        let note = dr::lane_note(lane, live.kit);
        let stand_in = live.kit != 0 && note.is_some_and(|n| n != dr::LANE_NOTES[lane][0]);
        let ink = if note.is_none() {
            TEXT_DIM.gamma_multiply(0.5)
        } else if stand_in {
            TEXT_DIM
        } else {
            TEXT
        };
        let label = Rect::from_min_size(pos2(rect.left(), y), vec2(LABEL_W - 4.0, rh));
        if lit >> lane & 1 == 1 {
            p.circle_filled(pos2(label.left() + 4.0, label.center().y), 2.5, accent);
        }
        p.with_clip_rect(label).text(pos2(label.left() + 10.0, label.center().y), Align2::LEFT_CENTER, dr::LANE_NAMES[lane], FontId::proportional((rh * 0.78).clamp(8.0, 11.5)), ink);
        let tip = match note {
            None => format!("This kit has no {}: it stays silent", dr::LANE_NAMES[lane].to_lowercase()),
            Some(n) if stand_in => format!("This kit has no {}: it plays the {} ({})", dr::LANE_NAMES[lane].to_lowercase(), gm_drum_name(n).to_lowercase(), n),
            Some(n) => format!("{} (note {n})", dr::LANE_NAMES[lane]),
        };
        ui.interact(label, id.with(("lane", lane)), Sense::hover()).on_hover_text(tip);
        for bar in 0..2 {
            for step in 0..STEPS {
                let c = Rect::from_min_size(pos2(col_x(bar, step), y), vec2(CELL_W, rh)).shrink(1.0);
                let v = dr::cell(data, bar, step, lane);
                let dim = if bar == 1 && !two_bars { 0.35 } else { 1.0 };
                let beat = step % 4 == 0;
                if v == dr::OFF {
                    rect_fill(&p, c, 1.5, Color32::from_white_alpha(if beat { 16 } else { 8 }));
                } else {
                    let (f, inset) = match v {
                        dr::GHOST => (0.38, 2.5),
                        dr::ACCENT => (1.0, 0.0),
                        _ => (0.72, 0.0),
                    };
                    let col = accent.gamma_multiply(f * dim);
                    if v == dr::ROLL {
                        let half = c.width() * 0.5;
                        rect_fill(&p, Rect::from_min_size(c.min, vec2(half - 0.5, c.height())), 1.0, accent.gamma_multiply(0.8 * dim));
                        rect_fill(&p, Rect::from_min_max(pos2(c.left() + half + 0.5, c.top()), c.max), 1.0, accent.gamma_multiply(0.55 * dim));
                    } else {
                        rect_fill(&p, c.shrink(inset), 1.5, col);
                    }
                }
            }
        }
    }
    if let Some(col) = now {
        let x = col_x(col / STEPS, col % STEPS);
        rect_fill(&p, Rect::from_min_size(pos2(x, grid.top()), vec2(CELL_W, grid.height())), 0.0, Color32::from_white_alpha(26));
        ui.ctx().request_repaint();
    }
    p.text(pos2(col_x(0, 0) + 1.0, grid.top() - 1.0), Align2::LEFT_BOTTOM, "A", FontId::proportional(9.0), TEXT_DIM);
    p.text(pos2(col_x(1, 0) + 1.0, grid.top() - 1.0), Align2::LEFT_BOTTOM, "B", FontId::proportional(9.0), TEXT_DIM);

    // Clicks and drags: the press decides what the drag paints.
    let resp = ui.interact(grid, id.with("grid"), Sense::click_and_drag());
    let paint_id = id.with("paint");
    let at = |pos: egui::Pos2| -> Option<(usize, usize, usize)> {
        let lane = ((pos.y - grid.top()) / rh).floor();
        if lane < 0.0 || lane >= LANES as f32 {
            return None;
        }
        let x = pos.x - grid.left();
        let (bar, x) = if x >= STEPS as f32 * CELL_W + BAR_GAP * 0.5 { (1, x - STEPS as f32 * CELL_W - BAR_GAP) } else { (0, x) };
        let step = (x / CELL_W).floor();
        (step >= 0.0 && step < STEPS as f32).then_some((bar, step as usize, lane as usize))
    };
    if let Some(pos) = resp.interact_pointer_pos() {
        if let Some((bar, step, lane)) = at(pos) {
            let cur = dr::cell(data, bar, step, lane);
            if resp.drag_started() || resp.clicked() {
                let next = if ui.input(|i| i.modifiers.shift) {
                    if cur == dr::ROLL { dr::OFF } else { dr::ROLL }
                } else {
                    match cur {
                        dr::OFF => dr::HIT,
                        dr::HIT => dr::ACCENT,
                        dr::ACCENT => dr::GHOST,
                        _ => dr::OFF,
                    }
                };
                ui.ctx().memory_mut(|m| m.data.insert_temp(paint_id, next));
            }
            if resp.dragged() || resp.clicked() || resp.drag_started() {
                let v: u8 = ui.ctx().memory(|m| m.data.get_temp(paint_id).unwrap_or(dr::HIT));
                if cur != v {
                    dr::set_cell(data, bar, step, lane, v);
                    changed = true;
                }
            }
        }
    }
    if resp.secondary_clicked() {
        if let Some((bar, step, lane)) = resp.interact_pointer_pos().or(resp.hover_pos()).and_then(at) {
            dr::set_cell(data, bar, step, lane, dr::OFF);
            changed = true;
        }
    }
    resp.on_hover_text("Click: hit, accent, ghost, off · Shift-click: a roll · right-click: off · drag to paint");
    changed
}
