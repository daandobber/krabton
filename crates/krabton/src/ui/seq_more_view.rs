//! Editors for the playful sequencers: Bounce (a ball per column: drag its
//! drop height and its note), Orbits, Chaos, Automaton, Ant, Pong, Markov,
//! Rain, Words and Lorenz. Like the others they show what the engine
//! reports and edit the device's params and data.

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Mesh, Rect, Response, Sense, Shape, Stroke, Ui, UiBuilder};
use krabton_engine::sequencers::{self, more, MAX_BALLS};
use std::f32::consts::TAU;

fn ball_color(i: usize) -> Color32 {
    let c = [ACCENT, SELECT, PLAY, Color32::from_rgb(0xe0, 0x84, 0xff)][i % 4];
    if i >= 4 {
        mix(c, Color32::WHITE, 0.35)
    } else {
        c
    }
}

/// The name of scale degree `degree` counted from the base note `base`.
pub(super) fn degree_name(live: &Live, base: f32, degree: i32) -> String {
    let (b, _) = live.scale.locate(base.round() as i32);
    krabton_engine::note_name(live.scale.pitch_at(b + degree).clamp(0, 127) as u8)
}

/// Drag up or down on `rect` to step an integer parameter by one per `px` pixels.
pub(super) fn drag_steps(ui: &mut Ui, rect: Rect, id: Id, value: &mut f32, min: f32, max: f32, px: f32) -> (Response, bool) {
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let acc_id = id.with("acc");
    let mut changed = false;
    if resp.drag_started() {
        ui.ctx().memory_mut(|m| m.data.insert_temp(acc_id, 0.0_f32));
    }
    if resp.dragged() {
        let mut acc: f32 = ui.ctx().memory(|m| m.data.get_temp(acc_id).unwrap_or(0.0)) - resp.drag_delta().y;
        let steps = (acc / px).trunc();
        acc -= steps * px;
        ui.ctx().memory_mut(|m| m.data.insert_temp(acc_id, acc));
        if steps != 0.0 {
            let v = (value.round() + steps).clamp(min, max);
            if v != *value {
                *value = v;
                changed = true;
            }
        }
    }
    (resp, changed)
}

/// A small box with a value in it that steps when dragged up or down.
#[allow(clippy::too_many_arguments)]
pub(super) fn chip(ui: &mut Ui, rect: Rect, id: Id, label: &str, color: Color32, value: &mut f32, min: f32, max: f32, tip: &str) -> bool {
    let (resp, changed) = drag_steps(ui, rect, id, value, min, max, 7.0);
    let p = ui.painter();
    let hot = resp.hovered() || resp.dragged();
    rect_fill(p, rect, 3.0, if hot { mix(SLOT, color, 0.25) } else { SLOT });
    text(p, rect.center(), Align2::CENTER_CENTER, label, small(), if hot { Color32::WHITE } else { color });
    if hot {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    resp.on_hover_text(tip);
    changed
}

/// A trail of recent values kept in the UI's memory while the transport runs.
pub(super) fn trail<T: Clone + Send + Sync + 'static>(ui: &Ui, id: Id, new: Option<T>, max: usize) -> Vec<T> {
    let mut v: Vec<T> = ui.ctx().memory(|m| m.data.get_temp(id).unwrap_or_default());
    if let Some(x) = new {
        v.push(x);
        if v.len() > max {
            v.drain(..v.len() - max);
        }
        ui.ctx().memory_mut(|m| m.data.insert_temp(id, v.clone()));
    }
    v
}

// ---------------------------------------------------------------- bounce

/// One column per ball: drag in the column to set how high it drops from,
/// drag its note at the bottom; columns past the ball count are dimmed
/// (click one to add balls up to it).
pub(crate) fn bounce(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, height: f32) -> bool {
    let col_w = 31.0;
    let w = col_w * MAX_BALLS as f32 + 12.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    rect_fill(ui.painter(), rect, 3.0, KNOB);
    let mut changed = false;
    let n = params[0].round().clamp(1.0, MAX_BALLS as f32) as usize;
    let note_h = 18.0;
    let top = rect.top() + 14.0;
    let floor = rect.bottom() - note_h - 12.0;
    let drops = sequencers::DROP_BEATS.len();
    let level_y = |k: usize| floor - (k + 1) as f32 / drops as f32 * (floor - top);
    {
        let p = ui.painter();
        p.line_segment([pos2(rect.left() + 4.0, floor + 4.0), pos2(rect.right() - 4.0, floor + 4.0)], Stroke::new(2.0_f32, LINE));
        for k in 0..drops {
            let y = level_y(k);
            p.line_segment([pos2(rect.left() + 4.0, y), pos2(rect.right() - 4.0, y)], Stroke::new(1.0_f32, LINE_SOFT));
        }
    }
    for i in 0..MAX_BALLS {
        let x0 = rect.left() + 6.0 + i as f32 * col_w;
        let col = Rect::from_min_max(pos2(x0, top - 8.0), pos2(x0 + col_w, floor + 6.0));
        let active = i < n;
        let color = if active { ball_color(i) } else { shade(ball_color(i), 0.35) };
        let drop = params[16 + i].round().clamp(0.0, (drops - 1) as f32) as usize;
        let resp = ui.interact(col, id.with(("col", i)), Sense::click_and_drag());
        if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked() || resp.is_pointer_button_down_on()) {
            let k = (((floor - pos.y) / (floor - top) * drops as f32).ceil() as i32 - 1).clamp(0, drops as i32 - 1) as f32;
            if params[16 + i] != k {
                params[16 + i] = k;
                changed = true;
            }
            if !active {
                params[0] = (i + 1) as f32;
                changed = true;
            }
        }
        let drop_y = level_y(drop);
        let p = ui.painter();
        if resp.hovered() || resp.dragged() {
            rect_fill(p, col.shrink(1.0), 2.0, Color32::from_white_alpha(8));
        }
        let cx = col.center().x;
        p.line_segment([pos2(cx - 9.0, drop_y), pos2(cx + 9.0, drop_y)], Stroke::new(2.0_f32, shade(color, 0.7)));
        text(p, pos2(cx, drop_y - 2.0), Align2::CENTER_BOTTOM, sequencers::DROPS[drop], small(), if active { TEXT_DIM } else { shade(TEXT_DIM, 0.5) });
        let y = match live.on() {
            Some(v) if active => ((v[0] >> (i * 8)) & 0xff) as f32 / 255.0,
            _ => 1.0,
        };
        let cy = floor - y * (floor - drop_y);
        let r = 7.0;
        if active {
            // The shadow grows as the ball comes down.
            p.circle_filled(pos2(cx, floor + 4.0), 2.0 + (1.0 - y) * 4.0, shade(color, 0.4));
            p.circle_filled(pos2(cx, cy.min(floor - r + 2.0)), r, color);
        } else {
            p.circle_stroke(pos2(cx, drop_y + r + 2.0), r - 1.0, Stroke::new(1.0_f32, color));
        }
        resp.on_hover_text(format!("Ball {}: drag up or down to set how high it drops from ({})", i + 1, sequencers::DROPS[drop]));
        let chip_r = Rect::from_min_size(pos2(x0 + 1.0, rect.bottom() - note_h - 4.0), vec2(col_w - 2.0, note_h));
        let name = degree_name(&live, params[3], params[8 + i].round() as i32);
        changed |= chip(ui, chip_r, id.with(("note", i)), &name, color, &mut params[8 + i], -14.0, 14.0, &format!("Ball {}'s note: drag up or down", i + 1));
    }
    text(ui.painter(), pos2(rect.left() + 6.0, rect.top() + 2.0), Align2::LEFT_TOP, format!("{n} balls"), small(), TEXT_DIM);
    // Dice: new notes (in the scale, leaning on the chord) or new drop heights for every ball.
    let bw = 46.0;
    let nb = Rect::from_min_size(pos2(rect.right() - 2.0 * bw - 8.0, rect.top() + 1.0), vec2(bw, 14.0));
    let db = Rect::from_min_size(pos2(rect.right() - bw - 4.0, rect.top() + 1.0), vec2(bw, 14.0));
    if button(ui, nb, id.with("dice_notes"), "notes", false, ACCENT).on_hover_text("Roll new notes for every ball: in the scale, mostly chord tones").clicked() {
        let mut seed = dice_seed();
        for i in 0..MAX_BALLS {
            params[8 + i] = dice_note(&mut seed);
        }
        changed = true;
    }
    if button(ui, db, id.with("dice_drops"), "drops", false, ACCENT).on_hover_text("Roll new drop heights for every ball").clicked() {
        let mut seed = dice_seed();
        for i in 0..MAX_BALLS {
            params[16 + i] = (rnd(&mut seed) * sequencers::DROPS.len() as f32).floor().min((sequencers::DROPS.len() - 1) as f32);
        }
        changed = true;
    }
    changed
}

fn dice_seed() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1) | 1
}

fn rnd(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed >> 8) as f32 / 16_777_216.0
}

/// A scale degree from a fifth below to two octaves up, usually a chord
/// tone (root, third, fifth), sometimes any step.
fn dice_note(seed: &mut u32) -> f32 {
    let octave = [-1.0, 0.0, 0.0, 1.0, 1.0, 2.0][(rnd(seed) * 6.0) as usize % 6];
    let degree = if rnd(seed) < 0.7 { [0.0, 2.0, 4.0][(rnd(seed) * 3.0) as usize % 3] } else { (rnd(seed) * 7.0).floor() };
    (octave * 7.0 + degree).clamp(-5.0, 14.0)
}

// ---------------------------------------------------------------- orbits

pub(crate) fn orbits(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, height: f32) -> bool {
    let size = height;
    let list_w = 92.0;
    let (rect, _) = ui.allocate_exact_size(vec2(size + list_w, height), Sense::hover());
    rect_fill(ui.painter(), rect, 3.0, KNOB);
    let n = params[0].round().clamp(1.0, 8.0) as usize;
    let c = pos2(rect.left() + size / 2.0, rect.center().y);
    let r_max = size / 2.0 - 8.0;
    let r_min = 14.0;
    let ring_r = |i: usize| if n == 1 { r_max } else { r_min + (r_max - r_min) * i as f32 / (n - 1) as f32 };
    let p = ui.painter().clone();
    // The line every planet plays on.
    p.line_segment([c, pos2(c.x, c.y - r_max - 6.0)], Stroke::new(1.0_f32, LINE));
    let vis = live.on();
    for i in 0..n {
        let r = ring_r(i);
        let color = ball_color(i);
        p.circle_stroke(c, r, Stroke::new(1.0_f32, shade(color, 0.35)));
        let phase = vis.map(|v| ((v[0] >> (i * 8)) & 0xff) as f32 / 255.0).unwrap_or(0.0);
        let a = phase * TAU;
        let at = c + vec2(a.sin(), -a.cos()) * r;
        // Just past the top: a flash.
        let flash = vis.is_some() && phase < 0.08;
        p.circle_filled(at, if flash { 6.0 } else { 4.5 }, if flash { Color32::WHITE } else { color });
    }
    let mut changed = false;
    let row_h = ((height - 8.0) / 8.0).min(18.0);
    for i in 0..n {
        let y = rect.top() + 4.0 + i as f32 * row_h;
        let color = ball_color(i);
        let sr = Rect::from_min_size(pos2(rect.left() + size + 2.0, y), vec2(38.0, row_h - 2.0));
        let label = format!("x{}", params[5 + i].round() as i32);
        changed |= chip(ui, sr, id.with(("speed", i)), &label, color, &mut params[5 + i], 1.0, 16.0, "Passes per period: drag up or down");
        let nr = Rect::from_min_size(pos2(sr.right() + 3.0, y), vec2(44.0, row_h - 2.0));
        let name = degree_name(&live, params[2], params[13 + i].round() as i32);
        changed |= chip(ui, nr, id.with(("note", i)), &name, color, &mut params[13 + i], -14.0, 14.0, "This planet's note: drag up or down");
    }
    changed
}

// ---------------------------------------------------------------- chaos

/// The bifurcation diagram of the logistic map, with the chosen setting as
/// a line on it; drag sideways to move along it.
pub(crate) fn chaos(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, height: f32) -> bool {
    let w = 230.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, height), Sense::click_and_drag());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let plot = rect.shrink2(vec2(6.0, 14.0)).translate(vec2(0.0, 6.0));
    let (r0, r1) = (2.8_f32, 4.0_f32);
    let mut changed = false;
    if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        let r = (r0 + (pos.x - plot.left()) / plot.width() * (r1 - r0)).clamp(r0, r1);
        let r = (r * 100.0).round() / 100.0;
        if params[1] != r {
            params[1] = r;
            changed = true;
        }
    }
    let mut mesh = Mesh::default();
    let cols = plot.width() as usize / 2;
    for k in 0..cols {
        let r = r0 + (k as f32 + 0.5) / cols as f32 * (r1 - r0);
        let mut x = 0.5_f32;
        for _ in 0..200 {
            x = r * x * (1.0 - x);
        }
        for _ in 0..48 {
            x = r * x * (1.0 - x);
            let px = plot.left() + k as f32 * 2.0;
            let py = plot.bottom() - x * plot.height();
            mesh.add_colored_rect(Rect::from_min_size(pos2(px, py), vec2(1.5, 1.5)), Color32::from_rgba_unmultiplied(0x9f, 0xd8, 0xff, 70));
        }
    }
    p.add(Shape::mesh(mesh));
    let r = params[1].clamp(r0, r1);
    let lx = plot.left() + (r - r0) / (r1 - r0) * plot.width();
    p.line_segment([pos2(lx, plot.top()), pos2(lx, plot.bottom())], Stroke::new(1.5_f32, ACCENT));
    let x_now = live.on().map(|v| (v[0] & 0xffff) as f32 / 65535.0);
    let hist = trail(ui, id.with("hist"), x_now, 12);
    if x_now.is_some() {
        for (k, x) in hist.iter().enumerate() {
            let a = (k + 1) as f32 / hist.len() as f32;
            p.circle_filled(pos2(lx, plot.bottom() - x * plot.height()), 1.5 + 2.5 * a, mix(ACCENT, Color32::WHITE, a).gamma_multiply(0.3 + 0.7 * a));
        }
    }
    let state = if r < 3.0 {
        "steady"
    } else if r < 3.449 {
        "cycle of 2"
    } else if r < 3.544 {
        "cycle of 4"
    } else if r < 3.5688 {
        "cycle of 8"
    } else if (3.8284..3.8415).contains(&r) {
        "window: cycle of 3"
    } else {
        "chaos"
    };
    text(&p, pos2(rect.left() + 6.0, rect.top() + 2.0), Align2::LEFT_TOP, format!("{r:.2} · {state}"), small(), TEXT_DIM);
    resp.on_hover_text("Drag sideways: left settles into cycles, right is chaos");
    changed
}

// ---------------------------------------------------------------- automaton

/// The rule as eight little pictures (click to flip what a pattern turns
/// into) above the last sixteen generations.
pub(crate) fn automaton(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let cell = ((height - 36.0) / 16.0).clamp(4.0, 9.0);
    let w = (16.0 * cell + 12.0).max(184.0);
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    rect_fill(ui.painter(), rect, 3.0, KNOB);
    let mut changed = false;
    let mut rule = params[1].round().clamp(0.0, 255.0) as u32;
    let icon = 5.0;
    for k in 0..8u32 {
        // Wolfram's order: 111 on the left down to 000.
        let pat = 7 - k;
        let x0 = rect.left() + 6.0 + k as f32 * 22.0;
        let r = Rect::from_min_size(pos2(x0, rect.top() + 4.0), vec2(3.0 * icon + 4.0, 2.0 * icon + 6.0));
        let resp = ui.interact(r, id.with(("rule", k)), Sense::click());
        if resp.clicked() {
            rule ^= 1 << pat;
            params[1] = rule as f32;
            changed = true;
        }
        let p = ui.painter();
        if resp.hovered() {
            rect_fill(p, r, 2.0, SLOT);
        }
        for b in 0..3 {
            let on = (pat >> (2 - b)) & 1 == 1;
            let c = Rect::from_min_size(pos2(r.left() + 2.0 + b as f32 * icon, r.top() + 2.0), vec2(icon - 0.5, icon - 0.5));
            rect_fill(p, c, 0.0, if on { TEXT_DIM } else { LINE_SOFT });
        }
        let out = (rule >> pat) & 1 == 1;
        let c = Rect::from_min_size(pos2(r.left() + 2.0 + icon, r.top() + 3.0 + icon), vec2(icon - 0.5, icon - 0.5));
        rect_fill(p, c, 0.0, if out { accent } else { LINE });
        resp.on_hover_text("Click to flip what this neighbourhood becomes");
    }
    // Generations: from the engine while playing, otherwise a preview.
    let rows: Vec<u16> = match live.on() {
        Some(v) => (0..16).map(|r| ((v[r / 4] >> ((r % 4) * 16)) & 0xffff) as u16).collect(),
        None => {
            let mut row = 1u16 << 8;
            let mut out = Vec::with_capacity(16);
            for _ in 0..16 {
                out.push(row);
                let mut next = 0u16;
                for i in 0..16 {
                    let l = (row as u32 >> ((i + 1) % 16)) & 1;
                    let c = (row as u32 >> i) & 1;
                    let r = (row as u32 >> ((i + 15) % 16)) & 1;
                    if (rule >> (l << 2 | c << 1 | r)) & 1 == 1 {
                        next |= 1 << i;
                    }
                }
                row = next;
            }
            out
        }
    };
    let voices = params[2].round().clamp(1.0, 8.0) as usize;
    let start = 8 - voices / 2;
    let board = Rect::from_min_size(pos2(rect.left() + 6.0, rect.top() + 28.0), vec2(16.0 * cell, 16.0 * cell));
    let p = ui.painter();
    for i in 0..16usize {
        // Bit i is drawn from the right, like the numbers.
        let x = board.right() - (i + 1) as f32 * cell;
        if (start..start + voices).contains(&i) {
            rect_fill(p, Rect::from_min_size(pos2(x, board.top()), vec2(cell, board.height())), 0.0, SLOT);
        }
        for (y, row) in rows.iter().enumerate() {
            if (row >> i) & 1 == 1 {
                let last = y == 15;
                let plays = last && (start..start + voices).contains(&i);
                let c = if plays { Color32::WHITE } else if last { accent } else { mix(shade(accent, 0.35), accent, y as f32 / 15.0) };
                rect_fill(p, Rect::from_min_size(pos2(x, board.top() + y as f32 * cell), vec2(cell - 1.0, cell - 1.0)), 1.0, c);
            }
        }
    }
    text(p, pos2(board.right() + 4.0, board.top()), Align2::LEFT_TOP, format!("Rule\n{rule}"), small(), TEXT_DIM);
    changed
}

// ---------------------------------------------------------------- ant

pub(crate) fn ant(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let cell = ((height - 10.0) / 8.0).floor();
    let w = cell * 8.0 + 12.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter();
    rect_fill(p, rect, 3.0, KNOB);
    let board = Rect::from_min_size(pos2(rect.left() + 6.0, rect.top() + 5.0), vec2(cell * 8.0, cell * 8.0));
    let (cells, ants, n) = match live.on() {
        Some(v) => {
            let n = ((v[1] >> 32) & 0xff).clamp(1, 3) as usize;
            let ants: Vec<(u8, u8, u8)> = (0..n).map(|i| {
                let b = (v[1] >> (i * 8)) as u8;
                (b & 7, (b >> 3) & 7, (b >> 6) & 3)
            }).collect();
            (v[0], ants, n)
        }
        None => {
            let n = params[1].round().clamp(1.0, 3.0) as usize;
            (0, vec![(3, 3, 0), (5, 2, 1), (1, 6, 2)], n)
        }
    };
    for y in 0..8 {
        for x in 0..8 {
            let r = Rect::from_min_size(board.min + vec2(x as f32 * cell, y as f32 * cell), vec2(cell - 1.0, cell - 1.0));
            let black = (cells >> (y * 8 + x)) & 1 == 1;
            rect_fill(p, r, 1.0, if black { shade(accent, 0.75) } else { SLOT });
        }
    }
    for (i, &(x, y, dir)) in ants.iter().take(n).enumerate() {
        let c = board.min + vec2((x as f32 + 0.5) * cell, (y as f32 + 0.5) * cell);
        let d = match dir {
            0 => vec2(0.0, -1.0),
            1 => vec2(1.0, 0.0),
            2 => vec2(0.0, 1.0),
            _ => vec2(-1.0, 0.0),
        };
        let side = vec2(-d.y, d.x);
        let s = cell * 0.36;
        let col = [Color32::WHITE, SELECT, Color32::from_rgb(0xe0, 0x84, 0xff)][i];
        p.add(Shape::convex_polygon(vec![c + d * s, c - d * s + side * s * 0.8, c - d * s - side * s * 0.8], col, Stroke::NONE));
    }
    false
}

// ---------------------------------------------------------------- pong

pub(crate) fn pong(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, height: f32) -> bool {
    let side = 46.0;
    let size = height - 40.0;
    let w = size + side * 2.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    rect_fill(ui.painter(), rect, 3.0, KNOB);
    let bx = Rect::from_min_size(pos2(rect.left() + side, rect.top() + 20.0), vec2(size, size));
    let n = params[2].round().clamp(1.0, 2.0) as usize;
    let vis = live.on();
    let balls: Vec<(f32, f32)> = match vis {
        Some(v) => (0..n).map(|i| (((v[0] >> (i * 16)) & 0xff) as f32 / 255.0, ((v[0] >> (i * 16 + 8)) & 0xff) as f32 / 255.0)).collect(),
        None => vec![(0.2, 0.3), (0.7, 0.6)].into_iter().take(n).collect(),
    };
    // Walls light up as a ball touches them: top 4, right 5, bottom 6, left 7.
    let near = |w: usize| balls.iter().any(|&(x, y)| match w {
        4 => y > 0.96,
        5 => x > 0.96,
        6 => y < 0.04,
        _ => x < 0.04,
    });
    let p = ui.painter().clone();
    let walls = [
        (4, [bx.left_top(), bx.right_top()]),
        (5, [bx.right_top(), bx.right_bottom()]),
        (6, [bx.left_bottom(), bx.right_bottom()]),
        (7, [bx.left_top(), bx.left_bottom()]),
    ];
    for (k, seg) in walls {
        let hot = vis.is_some() && near(k);
        p.line_segment(seg, Stroke::new(if hot { 3.0_f32 } else { 1.5 }, if hot { Color32::WHITE } else { LINE }));
    }
    for (i, &(x, y)) in balls.iter().enumerate() {
        let pos = pos2(bx.left() + x * bx.width(), bx.bottom() - y * bx.height());
        let tr = trail(ui, id.with(("trail", i)), vis.map(|_| (pos.x - bx.left(), pos.y - bx.top())), 14);
        if vis.is_some() {
            for (k, q) in tr.iter().enumerate() {
                let a = (k + 1) as f32 / tr.len() as f32;
                p.circle_filled(bx.min + vec2(q.0, q.1), 1.0 + 2.0 * a, ball_color(i).gamma_multiply(0.15 + 0.4 * a));
            }
        }
        p.circle_filled(pos, 5.0, ball_color(i));
    }
    let mut changed = false;
    let cw = 42.0;
    let chips = [
        (4, Rect::from_center_size(pos2(bx.center().x, rect.top() + 9.0), vec2(cw, 16.0))),
        (5, Rect::from_center_size(pos2(bx.right() + side / 2.0, bx.center().y), vec2(cw, 16.0))),
        (6, Rect::from_center_size(pos2(bx.center().x, bx.bottom() + 10.0), vec2(cw, 16.0))),
        (7, Rect::from_center_size(pos2(bx.left() - side / 2.0, bx.center().y), vec2(cw, 16.0))),
    ];
    for (k, r) in chips {
        let name = degree_name(&live, params[8], params[k].round() as i32);
        changed |= chip(ui, r, id.with(("wall", k)), &name, TEXT, &mut params[k], -14.0, 14.0, "This wall's note: drag up or down");
    }
    changed
}

// ---------------------------------------------------------------- markov

pub(crate) fn markov(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let w = 250.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    rect_fill(ui.painter(), rect, 3.0, KNOB);
    let range = params[7].round().max(2.0) as usize;
    let pos = live.on().map(|v| (v[0] & 0xff) as usize);
    let ladder = Rect::from_min_size(rect.min + vec2(6.0, 6.0), vec2(18.0, rect.height() - 12.0));
    let y_of = |k: f32| ladder.bottom() - k / range as f32 * ladder.height();
    let p = ui.painter().clone();
    for k in 0..=range {
        let on = pos == Some(k);
        p.line_segment([pos2(ladder.left(), y_of(k as f32)), pos2(ladder.right(), y_of(k as f32))], Stroke::new(if on { 3.0_f32 } else { 1.0 }, if on { accent } else { LINE }));
    }
    let hist = trail(ui, id.with("trail"), pos.map(|k| k as u8), 120);
    let plot = Rect::from_min_max(pos2(ladder.right() + 6.0, ladder.top()), pos2(rect.right() - 108.0, ladder.bottom()));
    if hist.len() >= 2 {
        let dx = plot.width() / 119.0;
        let x0 = plot.right() - (hist.len() - 1) as f32 * dx;
        let pts: Vec<_> = hist.iter().enumerate().map(|(i, &k)| pos2(x0 + i as f32 * dx, y_of(k.min(range as u8) as f32))).collect();
        p.add(Shape::line(pts, Stroke::new(1.5_f32, accent)));
    }
    // The four chances as bars to drag.
    let mut changed = false;
    let names = ["Repeat", "Step", "Leap", "Rest"];
    let short = ["Rep", "Step", "Leap", "Rest"];
    let total: f32 = params[1..5].iter().map(|v| v.max(0.0)).sum::<f32>().max(1e-6);
    for (k, name) in names.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.right() - 104.0 + k as f32 * 25.0, rect.top() + 6.0), vec2(22.0, rect.height() - 26.0));
        let resp = ui.interact(r, id.with(("bar", k)), Sense::click_and_drag());
        if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            let v = ((r.bottom() - pt.y) / r.height()).clamp(0.0, 1.0);
            let v = (v * 100.0).round() / 100.0;
            if params[1 + k] != v {
                params[1 + k] = v;
                changed = true;
            }
        }
        let p = ui.painter();
        rect_fill(p, r, 2.0, SLOT);
        let v = params[1 + k].clamp(0.0, 1.0);
        let fill = Rect::from_min_max(pos2(r.left(), r.bottom() - v * r.height()), r.max);
        rect_fill(p, fill, 2.0, if k == 3 { shade(accent, 0.45) } else { accent });
        text(p, pos2(r.center().x, r.top() + 2.0), Align2::CENTER_TOP, format!("{:.0}", params[1 + k].max(0.0) / total * 100.0), small(), TEXT_DARK);
        text(p, pos2(r.center().x, r.bottom() + 2.0), Align2::CENTER_TOP, short[k], small(), TEXT_DIM);
        resp.on_hover_text(format!("Chance to {}: drag up or down (the number is its share in %)", name.to_lowercase()));
    }
    changed
}

// ---------------------------------------------------------------- rain

pub(crate) fn rain(ui: &mut Ui, id: Id, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let w = 220.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let now = ui.input(|i| i.time);
    let range = params[3].round().max(2.0);
    // New drops: the engine shifts each new degree into the low byte.
    let last_id = id.with("last");
    let drops_id = id.with("drops");
    let mut drops: Vec<(f32, f64)> = ui.ctx().memory(|m| m.data.get_temp(drops_id).unwrap_or_default());
    if let Some(v) = live.on() {
        let last: u64 = ui.ctx().memory(|m| m.data.get_temp(last_id).unwrap_or(0));
        if v[0] != last {
            let fresh = (1..8).find(|&k| v[0] >> (8 * k) == last & (u64::MAX >> (8 * k))).unwrap_or(8);
            for k in 0..fresh {
                let d = ((v[0] >> (8 * k)) & 0xff) as f32;
                drops.push((d / range as f32, now));
            }
            ui.ctx().memory_mut(|m| m.data.insert_temp(last_id, v[0]));
        }
    }
    drops.retain(|d| now - d.1 < 1.2);
    if drops.len() > 64 {
        drops.drain(..drops.len() - 64);
    }
    let ground = rect.bottom() - 24.0;
    p.line_segment([pos2(rect.left() + 4.0, ground), pos2(rect.right() - 4.0, ground)], Stroke::new(1.0_f32, LINE));
    for &(x, t) in &drops {
        let age = (now - t) as f32;
        let px = rect.left() + 10.0 + x.clamp(0.0, 1.0) * (rect.width() - 20.0);
        if age < 0.12 {
            // The drop still falling into its splash.
            let y = ground - (1.0 - age / 0.12) * 40.0;
            p.line_segment([pos2(px, y - 8.0), pos2(px, y)], Stroke::new(1.5_f32, SELECT));
        }
        let a = (1.0 - age / 1.2).max(0.0);
        let rr = 3.0 + age * 26.0;
        p.add(Shape::ellipse_stroke(pos2(px, ground), vec2(rr, rr * 0.3), Stroke::new(1.5_f32, accent.gamma_multiply(a))));
    }
    // A steady drizzle in the background, as dense as the setting.
    let density = params[0].clamp(0.1, 16.0);
    let streaks = (density * 3.0).min(40.0) as usize;
    for k in 0..streaks {
        let seed = (k as f32 * 12.9898).sin() * 43758.545;
        let fx = seed - seed.floor();
        let speed = 0.6 + fx * 0.6;
        let fy = ((now as f32 * speed + fx * 7.0) % 1.0).abs();
        let px = rect.left() + 6.0 + fx * (rect.width() - 12.0);
        let py = rect.top() + 6.0 + fy * (ground - rect.top() - 14.0);
        p.line_segment([pos2(px, py), pos2(px, py + 5.0)], Stroke::new(1.0_f32, Color32::from_white_alpha(if live.on().is_some() { 40 } else { 16 })));
    }
    text(&p, pos2(rect.left() + 6.0, rect.bottom() - 2.0), Align2::LEFT_BOTTOM, format!("{density:.1} drops per beat"), small(), TEXT_DIM);
    if !drops.is_empty() || live.on().is_some() {
        ui.ctx().request_repaint();
    }
    ui.ctx().memory_mut(|m| m.data.insert_temp(drops_id, drops));
    false
}

// ---------------------------------------------------------------- words

pub(crate) fn words(ui: &mut Ui, id: Id, params: &[f32], data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let w = 280.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    rect_fill(ui.painter(), rect, 3.0, KNOB);
    let mut changed = false;
    let mut s: String = data.iter().map(|&b| b as char).collect();
    let field = Rect::from_min_size(rect.min + vec2(6.0, 6.0), vec2(w - 12.0, 22.0));
    let mut child = ui.new_child(UiBuilder::new().max_rect(field));
    let resp = child.add_sized(field.size(), egui::TextEdit::singleline(&mut s).id(id.with("text")).hint_text("type something").char_limit(256));
    if resp.changed() {
        let bytes: Vec<u8> = s.bytes().filter(|b| (0x20..0x7f).contains(b)).take(256).collect();
        if bytes != *data {
            *data = bytes;
            changed = true;
        }
    }
    let morse = params[1] >= 0.5;
    let accents = params[2] >= 0.5;
    let cur = live.on().map(|v| v[0] as usize);
    let p = ui.painter();
    let area = Rect::from_min_max(pos2(rect.left() + 6.0, field.bottom() + 6.0), pos2(rect.right() - 6.0, rect.bottom() - 20.0));
    let cw = 11.0;
    let lh = 16.0;
    let per_line = (area.width() / cw).floor() as usize;
    let lines = ((area.height() / lh).floor() as usize).max(1);
    // Scroll the text so the current letter stays in view.
    let first_line = cur.map(|c| (c / per_line.max(1)).saturating_sub(lines - 1)).unwrap_or(0);
    for (i, &b) in data.iter().enumerate().skip(first_line * per_line).take(per_line * lines) {
        let k = i - first_line * per_line;
        let r = Rect::from_min_size(pos2(area.left() + (k % per_line) as f32 * cw, area.top() + (k / per_line) as f32 * lh), vec2(cw - 1.0, lh - 2.0));
        let note = more::letter_degree(b);
        let here = cur == Some(i);
        if here {
            rect_fill(p, r, 2.0, accent);
        } else if note.is_some() {
            rect_fill(p, r, 2.0, SLOT);
        }
        let vowel = note.map(|n| n.1).unwrap_or(false);
        let c = if here { TEXT_DARK } else if note.is_none() { TEXT_DIM } else if accents && vowel { mix(TEXT, accent, 0.5) } else { TEXT };
        text(p, r.center(), Align2::CENTER_CENTER, (b as char).to_string(), small(), c);
    }
    let foot = match cur.and_then(|i| data.get(i)) {
        Some(&b) if morse => {
            let code: String = more::morse_steps(&[b]).iter().filter(|s| s.note.is_some()).map(|s| if s.steps > 1 { '-' } else { '.' }).collect();
            format!("{}  {}", (b as char).to_ascii_uppercase(), code)
        }
        Some(&b) => match more::letter_degree(b) {
            Some((d, _)) => format!("{} plays {}", b as char, degree_name(&live, params[3], d)),
            None => format!("'{}' rests", b as char),
        },
        None if morse => "Morse: dots, dashes and gaps".to_string(),
        None => "Every letter is a note, a to z climbing the scale".to_string(),
    };
    text(p, pos2(rect.left() + 6.0, rect.bottom() - 3.0), Align2::LEFT_BOTTOM, foot, small(), TEXT_DIM);
    changed
}

// ---------------------------------------------------------------- lorenz

/// The butterfly, side on (x across, z up). Below the dashed line it rests;
/// drag up or down to move the line.
pub(crate) fn lorenz(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let w = 230.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, height), Sense::click_and_drag());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let plot = rect.shrink(6.0);
    let to = |x: f32, z: f32| pos2(plot.left() + (x + 25.0) / 50.0 * plot.width(), plot.bottom() - z / 60.0 * plot.height());
    let mut changed = false;
    if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        let z = (plot.bottom() - pos.y) / plot.height() * 60.0;
        let v = ((z / 50.0 / 0.6) * 100.0).round().clamp(0.0, 100.0) / 100.0;
        if params[3] != v {
            params[3] = v;
            changed = true;
        }
    }
    // A preview of the attractor at this rho.
    let rho = params[2].clamp(10.0, 60.0);
    let (mut x, mut y, mut z) = (0.1_f32, 0.0_f32, 20.0_f32);
    let mut pts = Vec::with_capacity(1500);
    for k in 0..2000 {
        let (dx, dy, dz) = (10.0 * (y - x), x * (rho - z) - y, x * y - 8.0 / 3.0 * z);
        x += dx * 0.008;
        y += dy * 0.008;
        z += dz * 0.008;
        if k >= 500 {
            pts.push(to(x, z));
        }
    }
    p.add(Shape::line(pts, Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0x9f, 0xd8, 0xff, 50))));
    let rest_z = params[3] * 0.6 * 50.0;
    let ry = to(0.0, rest_z).y;
    let mut xx = plot.left();
    while xx < plot.right() {
        p.line_segment([pos2(xx, ry), pos2((xx + 5.0).min(plot.right()), ry)], Stroke::new(1.0_f32, TEXT_DIM));
        xx += 9.0;
    }
    text(&p, pos2(plot.right(), ry + 2.0), Align2::RIGHT_TOP, "rest", small(), TEXT_DIM);
    let now = live.on().map(|v| (((v[0] & 0xffff) as f32 / 65535.0) * 50.0 - 25.0, ((v[0] >> 16) & 0xffff) as f32 / 65535.0 * 60.0));
    let tr = trail(ui, id.with("trail"), now, 40);
    if now.is_some() && tr.len() >= 2 {
        let pts: Vec<_> = tr.iter().map(|&(x, z)| to(x, z)).collect();
        p.add(Shape::line(pts, Stroke::new(1.5_f32, accent)));
        let (x, z) = tr[tr.len() - 1];
        p.circle_filled(to(x, z), 4.0, Color32::WHITE);
    }
    resp.on_hover_text("Drag up or down: below the dashed line the butterfly rests");
    changed
}
