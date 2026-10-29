//! Editors for the sequencers that follow a system: Bells (the rows rung
//! so far, the treble's line and the blue line through them, the calls),
//! Canon (the phrase and each voice's copy of it), Row (the row's notes and
//! its shape round the clock of twelve), Gears (the train of gears turning,
//! their pegs), Formula (the bytes the formula gives) and Staircase (the
//! stair round and round, and the voices under the bell curve).

use super::nature_seq_view::{fader_at, frame, pick, pick_choice, pick_count, roll, slot, split};
use super::seq_more_view::{chip, degree_name};
use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::sequencers::{self, nature, systems as sy};
use std::f32::consts::{FRAC_PI_2, TAU};

fn choice(params: &[f32], i: usize, n: usize) -> usize {
    (params[i].round().max(0.0) as usize).min(n - 1)
}

fn voice_color(i: usize) -> Color32 {
    [accent_now(), SELECT, PLAY, Color32::from_rgb(0xe0, 0x84, 0xff), Color32::from_rgb(0xff, 0x7a, 0x9c), Color32::from_rgb(0x7a, 0xe0, 0xd0)][i % 6]
}

// ---------------------------------------------------------------- bells

/// The rows rung so far, newest at the bottom: each bell's number in its
/// place, the treble's path and the blue line of the second.
pub(crate) fn bells(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 330.0, height);
    let (pic, side) = split(rect);
    let vis = live.on();
    let n = vis.map(|v| (v[0] >> 36 & 0xf) as usize).unwrap_or(params[sy::BE_BELLS].round() as usize).clamp(4, sy::MAX_BELLS);
    // The rows, kept while it rings.
    let hist_id = id.with("bell_rows");
    let mut rows: Vec<(u32, u32)> = ui.ctx().memory(|m| m.data.get_temp(hist_id).unwrap_or_default());
    if let Some(v) = vis {
        let entry = (v[1] as u32, v[0] as u32);
        if rows.last().map(|r| r.0) != Some(entry.0) {
            rows.push(entry);
            if rows.len() > 24 {
                rows.remove(0);
            }
            ui.ctx().memory_mut(|m| m.data.insert_temp(hist_id, rows.clone()));
        }
    }
    let rounds: u32 = (0..n as u32).fold(0, |acc, i| acc | i << (i * 4));
    let shown: Vec<u32> = if rows.is_empty() { vec![rounds] } else { rows.iter().map(|r| r.1).collect() };
    let head = 16.0;
    let area = Rect::from_min_max(pos2(pic.left(), pic.top() + head), pic.max);
    let rh = 13.0;
    let fit = ((area.height() / rh) as usize).max(1);
    let shown = &shown[shown.len().saturating_sub(fit)..];
    let cw = area.width() / n as f32;
    let at = |r: usize, place: usize| pos2(area.left() + (place as f32 + 0.5) * cw, area.top() + (r as f32 + 0.5) * rh);
    let place_of = |row: u32, bell: u32| (0..n).find(|&i| row >> (i * 4) & 0xf == bell);
    // The treble's line and the blue line.
    for (bell, col) in [(0u32, accent), (1u32, SOLO)] {
        let pts: Vec<egui::Pos2> = shown.iter().enumerate().filter_map(|(r, &row)| place_of(row, bell).map(|i| at(r, i))).collect();
        if pts.len() > 1 {
            p.add(Shape::line(pts, Stroke::new(2.0_f32, col.gamma_multiply(0.8))));
        }
    }
    let last = shown.len() - 1;
    let place_now = vis.map(|v| (v[0] >> 32 & 0xf) as usize);
    for (r, &row) in shown.iter().enumerate() {
        for i in 0..n {
            let bell = row >> (i * 4) & 0xf;
            let now = r == last && place_now.is_some_and(|pl| pl == i + 1 || (pl == 0 && i + 1 == n));
            if now {
                p.circle_filled(at(r, i), 6.0, Color32::WHITE);
            }
            let col = if now { TEXT_DARK } else if r == last { TEXT } else { TEXT_DIM };
            text(&p, at(r, i), Align2::CENTER_CENTER, format!("{}", bell + 1), small(), col);
        }
    }
    let method = sy::METHODS[choice(params, sy::BE_METHOD, sy::METHODS.len())];
    let status = match vis.map(|v| ((v[0] >> 43 & 3) as u8, (v[0] >> 41 & 3) as u8)) {
        Some((_, 1)) => "Bob!".to_string(),
        Some((_, 2)) => "Single!".to_string(),
        Some((1, _)) => format!("lead {}", (vis.unwrap()[1] >> 32) + 1),
        Some((2, _)) => "rounds again".to_string(),
        Some(_) => "rounds".to_string(),
        None => format!("{n} bells"),
    };
    text(&p, pos2(pic.left(), pic.top()), Align2::LEFT_TOP, format!("{method} · {status}"), small(), TEXT_DIM);
    let specs = sy::BELLS;
    let mut changed = pick_count(ui, slot(side, 0), id.with("be_n"), params, sy::BE_BELLS, 4.0, sy::MAX_BELLS as f32, "Bells", accent);
    changed |= pick_choice(ui, slot(side, 1), id.with("be_method"), params, sy::BE_METHOD, sy::METHODS, "Method", accent);
    changed |= fader_at(ui, slot(side, 2), id, params, specs, sy::BE_CALLS, accent);
    changed |= fader_at(ui, slot(side, 3), id, params, specs, sy::BE_SINGLES, accent);
    changed |= pick_count(ui, slot(side, 4), id.with("be_rounds"), params, sy::BE_ROUNDS, 0.0, 8.0, "Rounds", accent);
    let on = if params[sy::BE_GAP] >= 0.5 { "On" } else { "Off" };
    changed |= pick(ui, slot(side, 5), id.with("be_gap"), &mut params[sy::BE_GAP], 0.0, 1.0, "Gap", on, accent);
    changed |= pick_choice(ui, slot(side, 6), id.with("be_notes"), params, sy::BE_NOTES, sy::BELL_NOTES, "Notes", accent);
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    changed
}

// ---------------------------------------------------------------- canon

/// The leader's phrase and under it each voice's copy (upside down,
/// backward...), where each is now; beside each voice its delay, interval,
/// form and speed.
pub(crate) fn canon(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 400.0, height);
    let mut changed = false;
    let top = Rect::from_min_size(rect.min + vec2(4.0, 4.0), vec2(rect.width() - 8.0, 22.0));
    let qw = (top.width() - 9.0) / 4.0;
    let q = |i: usize| Rect::from_min_size(pos2(top.left() + i as f32 * (qw + 3.0), top.top()), vec2(qw, top.height()));
    changed |= pick_choice(ui, q(0), id.with("ca_src"), params, sy::CA_SOURCE, sy::CANON_SOURCES, "Lead", accent);
    changed |= pick_choice(ui, q(1), id.with("ca_ph"), params, sy::CA_PHRASE, sy::PHRASES, "Phrase", accent);
    changed |= pick_count(ui, q(2), id.with("ca_len"), params, sy::CA_LENGTH, 4.0, 16.0, "Length", accent);
    changed |= pick_count(ui, q(3), id.with("ca_v"), params, sy::CA_VOICES, 1.0, sy::MAX_FOLLOWERS as f32, "Voices", accent);
    let played = params[sy::CA_SOURCE] >= 0.5;
    let voices = (params[sy::CA_VOICES].round() as usize).clamp(1, sy::MAX_FOLLOWERS);
    let len = (params[sy::CA_LENGTH].round() as usize).clamp(4, sy::PHRASE_LEN);
    let ph = sy::phrase(choice(params, sy::CA_PHRASE, sy::PHRASES.len()), len, params[sy::CA_SEED].round() as u32);
    let vis = live.on().map(|v| nature::unpack(&v));
    let body = Rect::from_min_max(pos2(rect.left() + 4.0, top.bottom() + 6.0), pos2(rect.right() - 4.0, rect.bottom() - 4.0));
    let row_h = body.height() / (voices + 1) as f32;
    let ctrl_w = 166.0;
    let strip_w = body.width() - ctrl_w - 6.0;
    for r in 0..=voices {
        let y0 = body.top() + r as f32 * row_h;
        let strip = Rect::from_min_size(pos2(body.left(), y0 + 1.0), vec2(strip_w, row_h - 3.0));
        rect_fill(&p, strip, 3.0, PANEL3);
        let col = voice_color(r);
        let label = if r == 0 { "leader".to_string() } else { format!("{:+}", params[sy::CA_INTERVAL + r - 1].round() as i32) };
        text(&p, pos2(strip.left() + 3.0, strip.top() + 1.0), Align2::LEFT_TOP, label, small(), TEXT_DIM);
        let lit = vis.is_some_and(|b| b[13] >> r & 1 == 1);
        if played {
            // Played: the last note each voice answered with.
            if let Some(b) = vis {
                let d = b[8 + r] as i32 - 64;
                let y = strip.center().y - (d as f32 / 16.0).clamp(-0.5, 0.5) * strip.height() * 0.8;
                p.circle_filled(pos2(strip.center().x, y), 4.0, col);
            } else if r == 0 {
                text(&p, strip.center(), Align2::CENTER_CENTER, "play: the voices follow you", small(), TEXT_DIM);
            }
        } else {
            let form = if r == 0 { 0 } else { choice(params, sy::CA_FORM + r - 1, sy::FORMS.len()) };
            let now = vis.map(|b| b[r] as usize);
            let cw = (strip.width() - 30.0) / len as f32;
            for i in 0..len {
                let Some(d) = sy::canon_degree(&ph, len, i as i64, form) else { continue };
                let x = strip.left() + 28.0 + (i as f32 + 0.5) * cw;
                let y = strip.bottom() - 4.0 - ((d + 6) as f32 / 18.0).clamp(0.0, 1.0) * (strip.height() - 8.0);
                let on = now == Some(i);
                let c = if on && lit { Color32::WHITE } else if on { col } else { col.gamma_multiply(0.5) };
                p.add(Shape::rect_filled(Rect::from_center_size(pos2(x, y), vec2((cw - 2.0).max(2.0), 4.0)), 1.0, c));
            }
            if now == Some(255) {
                text(&p, strip.center(), Align2::CENTER_CENTER, "waiting to come in", small(), TEXT_DIM);
            }
        }
        if r == 0 {
            continue;
        }
        let v = r - 1;
        let cx = strip.right() + 6.0;
        let h = (row_h - 3.0).min(24.0);
        let cy = y0 + 1.0;
        let delay = format!("+{}", params[sy::CA_DELAY + v].round() as i32);
        changed |= chip(ui, Rect::from_min_size(pos2(cx, cy), vec2(34.0, h)), id.with(("ca_delay", v)), &delay, col, &mut params[sy::CA_DELAY + v], 1.0, 32.0, "How many steps later it comes in: drag up or down");
        let iv = format!("{:+}", params[sy::CA_INTERVAL + v].round() as i32);
        changed |= chip(ui, Rect::from_min_size(pos2(cx + 37.0, cy), vec2(34.0, h)), id.with(("ca_iv", v)), &iv, col, &mut params[sy::CA_INTERVAL + v], -14.0, 14.0, "How many scale steps higher or lower: drag up or down");
        let form = ["Same", "Upside", "Back", "Both"][choice(params, sy::CA_FORM + v, 4)];
        changed |= pick(ui, Rect::from_min_size(pos2(cx + 74.0, cy), vec2(52.0, h)), id.with(("ca_form", v)), &mut params[sy::CA_FORM + v], 0.0, 3.0, "", form, accent);
        let speed = ["½", "1", "2"][choice(params, sy::CA_SPEED + v, 3)];
        changed |= pick(ui, Rect::from_min_size(pos2(cx + 129.0, cy), vec2(31.0, h)), id.with(("ca_speed", v)), &mut params[sy::CA_SPEED + v], 0.0, 2.0, "", speed, accent);
    }
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    changed
}

// ---------------------------------------------------------------- row

/// The row's notes in the form that plays, the one now lit, and its shape
/// round the clock of twelve (or of the key's notes).
pub(crate) fn row(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 330.0, height);
    let (pic, side) = split(rect);
    let twelve = choice(params, sy::RO_NOTES, sy::ROW_NOTES.len()) == 0;
    let vis = live.on();
    let (row, m, lit, form) = match vis {
        Some(v) => {
            let m = ((v[0] >> 56 & 0xf) as usize).clamp(1, 12);
            let row: Vec<u8> = (0..m).map(|i| (v[0] >> (i * 4) & 0xf) as u8).collect();
            (row, m, v[1] >> 16 & 0xfff, (v[0] >> 48 & 0xf) as usize)
        }
        None => {
            let m = if twelve { 12 } else { live.scale.len().clamp(1, 12) };
            let prime = sy::prime_row(choice(params, sy::RO_ROW, sy::ROWS.len()), params[sy::RO_SEED].round() as u32, m);
            (prime[..m].to_vec(), m, 0, 0)
        }
    };
    let name = |pc: u8| -> String {
        if twelve {
            krabton_engine::scale::NOTE_NAMES[(live.scale.root as usize + pc as usize) % 12].to_string()
        } else {
            let n = live.scale.pitch_at(pc as i32).rem_euclid(12) as usize;
            krabton_engine::scale::NOTE_NAMES[n].to_string()
        }
    };
    let bw = pic.width() / m as f32;
    for (i, &pc) in row.iter().enumerate() {
        let r = Rect::from_min_size(pos2(pic.left() + i as f32 * bw, pic.top()), vec2(bw - 2.0, 20.0));
        let on = lit >> i & 1 == 1;
        rect_fill(&p, r, 2.0, if on { accent } else { PANEL3 });
        text(&p, r.center(), Align2::CENTER_CENTER, name(pc), small(), if on { TEXT_DARK } else { TEXT });
    }
    // The clock: the row as a path through the twelve (or the key's) places.
    let c = pos2(pic.center().x, (pic.top() + 26.0 + pic.bottom()) / 2.0);
    let rad = ((pic.bottom() - pic.top() - 34.0) / 2.0).min(pic.width() / 2.0) - 4.0;
    let at = |pc: u8| c + vec2((pc as f32 / m as f32 * TAU - FRAC_PI_2).cos(), (pc as f32 / m as f32 * TAU - FRAC_PI_2).sin()) * rad;
    p.circle_stroke(c, rad, Stroke::new(1.0_f32, LINE_SOFT));
    let pts: Vec<egui::Pos2> = row.iter().map(|&pc| at(pc)).collect();
    p.add(Shape::line(pts, Stroke::new(1.2_f32, accent.gamma_multiply(0.55))));
    for (i, &pc) in row.iter().enumerate() {
        let on = lit >> i & 1 == 1;
        p.circle_filled(at(pc), if on { 5.0 } else { 3.0 }, if on { Color32::WHITE } else { accent });
    }
    if vis.is_some() {
        text(&p, c, Align2::CENTER_CENTER, ["P", "R", "I", "RI"][form % 4], FontId::proportional(16.0), TEXT_DIM);
        ui.ctx().request_repaint();
    }
    let mut changed = pick_choice(ui, slot(side, 0), id.with("ro_form"), params, sy::RO_FORM, sy::ROW_FORMS, "Form", accent);
    changed |= pick_choice(ui, slot(side, 1), id.with("ro_row"), params, sy::RO_ROW, sy::ROWS, "Row", accent);
    changed |= pick_choice(ui, slot(side, 2), id.with("ro_tex"), params, sy::RO_TEXTURE, sy::TEXTURES, "Texture", accent);
    changed |= pick_choice(ui, slot(side, 3), id.with("ro_rhythm"), params, sy::RO_RHYTHM, sy::ROW_RHYTHMS, "Rhythm", accent);
    changed |= pick_choice(ui, slot(side, 4), id.with("ro_move"), params, sy::RO_MOVE, sy::MOVES, "Move", accent);
    changed |= pick_choice(ui, slot(side, 5), id.with("ro_notes"), params, sy::RO_NOTES, &["Twelve", "Key"], "Notes", accent);
    if roll(ui, slot(side, 6), id.with("ro_seed"), params, sy::RO_SEED, 99.0, "Shuffle", "A new shuffled row (Seed)", accent) {
        params[sy::RO_ROW] = 0.0;
        changed = true;
    }
    changed
}

// ---------------------------------------------------------------- gears

/// The train of gears turning, their pegs on the teeth and the top where a
/// peg plays. Click a gear for a tooth more, right-click for one less.
pub(crate) fn gears(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 400.0, height);
    let (pic, side) = split(rect);
    let n = (params[sy::GE_GEARS].round() as usize).clamp(2, sy::MAX_GEARS);
    let vis = live.on().map(|v| nature::unpack(&v));
    let teeth: Vec<usize> = (0..n).map(|i| params[sy::GE_TEETH + i].round().clamp(3.0, 32.0) as usize).collect();
    // Radius in proportion to the teeth, all of them fitting side by side.
    let total: f32 = teeth.iter().map(|&t| 2.0 * t as f32).sum();
    let k = (pic.width() / total).min((pic.height() - 20.0) / (2.0 * *teeth.iter().max().unwrap() as f32));
    let mut changed = false;
    let mut x = pic.left();
    let cy = pic.top() + (pic.height() - 14.0) / 2.0;
    for (i, &t) in teeth.iter().enumerate() {
        let r = t as f32 * k;
        let c = pos2(x + r, cy);
        x += 2.0 * r;
        let col = voice_color(i);
        let top = vis.map(|b| b[i] as usize).unwrap_or(0);
        let fired = vis.is_some_and(|b| b[7] >> i & 1 == 1);
        let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
        let pegs = (params[sy::GE_PEGS + i].round().max(1.0) as usize).min(t);
        p.circle_filled(c, r * 0.82, shade(col, 0.18));
        p.circle_stroke(c, r * 0.82, Stroke::new(1.0_f32, shade(col, 0.6)));
        for tooth in 0..t {
            let a = -FRAC_PI_2 + dir * (tooth as f32 - top as f32) / t as f32 * TAU;
            let (ca, sa) = (a.cos(), a.sin());
            p.line_segment([c + vec2(ca, sa) * r * 0.82, c + vec2(ca, sa) * r], Stroke::new((k * 0.9).clamp(1.0, 3.0), shade(col, 0.7)));
            if sequencers::euclid_hit(tooth, t, pegs, 0) {
                let on = tooth == top && fired;
                p.circle_filled(c + vec2(ca, sa) * r * 0.66, (k * 0.7).clamp(1.5, 3.5), if on { Color32::WHITE } else { col });
            }
        }
        p.circle_filled(c, (r * 0.12).max(2.0), shade(col, 0.6));
        let resp = ui.interact(Rect::from_center_size(c, vec2(2.0 * r, 2.0 * r)), id.with(("gear", i)), Sense::click());
        let step = if resp.clicked() {
            1.0
        } else if resp.secondary_clicked() {
            -1.0
        } else {
            0.0
        };
        if step != 0.0 {
            params[sy::GE_TEETH + i] = (params[sy::GE_TEETH + i].round() + step).clamp(3.0, 32.0);
            changed = true;
        }
        resp.on_hover_text(format!("Gear {}: {t} teeth, {pegs} pegs. Click: a tooth more, right-click: one less", i + 1));
    }
    // The top, where pegs play.
    p.line_segment([pos2(pic.left(), cy - (pic.height() - 20.0) / 2.0), pos2(pic.right(), cy - (pic.height() - 20.0) / 2.0)], Stroke::new(1.0_f32, LINE_SOFT));
    if vis.is_some_and(|b| b[8] == 1) {
        text(&p, pos2(pic.left(), pic.bottom()), Align2::LEFT_BOTTOM, "all home", small(), accent);
    }
    changed |= pick_count(ui, slot(side, 0), id.with("ge_n"), params, sy::GE_GEARS, 2.0, sy::MAX_GEARS as f32, "Gears", accent);
    for i in 0..n {
        let s = slot(side, 1 + i);
        let col = voice_color(i);
        let pegs = format!("{} pegs", params[sy::GE_PEGS + i].round() as i32);
        changed |= chip(ui, Rect::from_min_size(s.min, vec2(s.width() * 0.5 - 2.0, s.height())), id.with(("pegs", i)), &pegs, col, &mut params[sy::GE_PEGS + i], 1.0, 8.0, "Pegs on this gear: drag up or down");
        let note = degree_name(&live, params[sy::GE_BASE], params[sy::GE_NOTE + i].round() as i32);
        changed |= chip(ui, Rect::from_min_size(pos2(s.center().x + 2.0, s.top()), vec2(s.width() * 0.5 - 2.0, s.height())), id.with(("note", i)), &note, col, &mut params[sy::GE_NOTE + i], -14.0, 14.0, "This gear's note: drag up or down");
    }
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    changed
}

// ---------------------------------------------------------------- formula

/// The bytes the formula gives from here on, as bars (dim where it rests),
/// the notes they make as dots.
pub(crate) fn formula(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 330.0, height);
    let (pic, side) = split(rect);
    let f = choice(params, sy::FO_FORMULA, sy::FORMULAS.len());
    text(&p, pos2(pic.left(), pic.top()), Align2::LEFT_TOP, sy::FORMULAS[f], FontId::monospace(12.0), TEXT);
    let t0 = live.on().map(|v| (v[0] & 0xffff_ffff) as u32).unwrap_or(sy::formula_t(0, params, 16));
    let area = Rect::from_min_max(pos2(pic.left(), pic.top() + 20.0), pic.max);
    let n = 48;
    let bw = area.width() / n as f32;
    let range = params[sy::FO_RANGE].round().max(1.0);
    let (a, b) = (params[sy::FO_A].round() as u32, params[sy::FO_B].round() as u32);
    for i in 0..n {
        let t = t0.wrapping_add((i as u32).wrapping_mul(sy::formula_stride(params)));
        let byte = sy::formula_byte(f, t, a, b) as f32 / 255.0;
        let note = sy::formula_note(params, t);
        let x = area.left() + i as f32 * bw;
        let bar = Rect::from_min_max(pos2(x + 0.5, area.bottom() - byte * area.height()), pos2(x + bw - 0.5, area.bottom()));
        let col = if i == 0 && live.on().is_some() { Color32::WHITE } else if note.is_some() { shade(accent, 0.55) } else { PANEL3 };
        rect_fill(&p, bar, 0.0, col);
        if let Some(d) = note {
            let y = area.bottom() - (d as f32 / range) * (area.height() - 6.0) - 3.0;
            p.circle_filled(pos2(x + bw / 2.0, y), 2.0, if i == 0 { Color32::WHITE } else { accent });
        }
    }
    if live.on().is_some() {
        ui.ctx().request_repaint();
    }
    let specs = sy::FORMULA;
    let mut changed = pick_choice(ui, slot(side, 0), id.with("fo_f"), params, sy::FO_FORMULA, &["1", "2", "3", "4", "5", "6", "7", "8", "9"], "Formula", accent);
    changed |= pick_count(ui, slot(side, 1), id.with("fo_a"), params, sy::FO_A, 1.0, 16.0, "A", accent);
    changed |= pick_count(ui, slot(side, 2), id.with("fo_b"), params, sy::FO_B, 1.0, 16.0, "B", accent);
    changed |= pick_choice(ui, slot(side, 3), id.with("fo_step"), params, sy::FO_STEP, sy::FORMULA_STEPS, "Step", accent);
    changed |= pick_choice(ui, slot(side, 4), id.with("fo_loop"), params, sy::FO_LOOP, sequencers::bits::LOOPS, "Loop", accent);
    changed |= fader_at(ui, slot(side, 5), id, params, specs, sy::FO_RESTS, accent);
    changed |= roll(ui, slot(side, 6), id.with("fo_seed"), params, sy::FO_SEED, 255.0, "Shift", "Start the counter one further on (Seed)", accent);
    changed
}

// ---------------------------------------------------------------- staircase

/// The stair going round (the step it is on lit) and the voices under the
/// bell curve: loud in the middle, fading at the top and the bottom.
pub(crate) fn staircase(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 330.0, height);
    let (pic, side) = split(rect);
    let n = live.scale.len().max(1);
    let x = live.on().map(|v| v[0] as u32 as i32 as i64).unwrap_or(0);
    // The stair round a circle, each step a little higher.
    let ring = Rect::from_min_size(pic.min, vec2(pic.width(), pic.height() * 0.55));
    let c = ring.center();
    let rad = ring.height().min(ring.width()) / 2.0 - 6.0;
    let now = x.rem_euclid(n as i64) as usize;
    for s in 0..n {
        let a = s as f32 / n as f32 * TAU - FRAC_PI_2;
        let q = c + vec2(a.cos(), a.sin()) * rad * 0.75;
        let size = 5.0 + 7.0 * s as f32 / n as f32;
        let on = s == now;
        rect_fill(&p, Rect::from_center_size(q, vec2(size, size)), 2.0, if on { Color32::WHITE } else { mix(PANEL3, accent, 0.3 + 0.5 * s as f32 / n as f32) });
    }
    // The bell curve over the voices.
    let curve = Rect::from_min_max(pos2(pic.left(), ring.bottom() + 6.0), pic.max);
    let (cd, _) = live.scale.locate(params[sy::SC_CENTER].round() as i32);
    let voices = (params[sy::SC_VOICES].round() as usize).clamp(2, 6);
    let span = (voices * n) as f32;
    let width = params[sy::SC_WIDTH].clamp(0.25, 4.0) * n as f32;
    let pts: Vec<egui::Pos2> = (0..=48)
        .map(|i| {
            let f = i as f32 / 48.0;
            let rel = (f - 0.5) * span / width;
            pos2(curve.left() + f * curve.width(), curve.bottom() - (-2.0 * rel * rel).exp() * (curve.height() - 4.0))
        })
        .collect();
    p.add(Shape::line(pts, Stroke::new(1.0_f32, accent.gamma_multiply(0.5))));
    let vs = sy::stair_voices(params, x, n, cd);
    let bottom = cd as f32 - span / 2.0;
    for &(d, loud) in vs.iter().take(voices) {
        let f = (d as f32 - bottom) / span;
        let q = pos2(curve.left() + f * curve.width(), curve.bottom() - loud * (curve.height() - 4.0));
        p.circle_filled(q, 2.0 + 4.0 * loud, mix(accent, Color32::WHITE, loud));
    }
    if live.on().is_some() {
        ui.ctx().request_repaint();
    }
    let specs = sy::STAIRCASE;
    let mut changed = pick_choice(ui, slot(side, 0), id.with("sc_dir"), params, sy::SC_DIRECTION, sy::DIRECTIONS, "Way", accent);
    changed |= pick_count(ui, slot(side, 1), id.with("sc_v"), params, sy::SC_VOICES, 2.0, 6.0, "Voices", accent);
    changed |= pick_choice(ui, slot(side, 2), id.with("sc_play"), params, sy::SC_PLAY, sy::STAIR_PLAYS, "Play", accent);
    changed |= pick_count(ui, slot(side, 3), id.with("sc_step"), params, sy::SC_STEP, 1.0, 4.0, "Step", accent);
    changed |= pick_count(ui, slot(side, 4), id.with("sc_every"), params, sy::SC_EVERY, 1.0, 8.0, "Every", accent);
    changed |= fader_at(ui, slot(side, 5), id, params, specs, sy::SC_WIDTH, accent);
    changed
}
