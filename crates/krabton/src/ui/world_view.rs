//! Editors for the sequencers from around the world: Gamelan (the gong
//! cycle with its core notes, the gongs marked, and the two parts locking
//! into each other), Raga (the notes going up and coming down, the one it
//! is on, the way through Alap, Jor and Jhala) and Call & Response (the
//! call and its answer).

use super::nature_seq_view::{fader_at, frame, pick, pick_choice, pick_count, roll, slot, split};
use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Shape, Stroke, Ui};
use krabton_engine::sequencers::systems::PHRASE_LEN;
use krabton_engine::sequencers::world as wo;
use std::f32::consts::{FRAC_PI_2, TAU};

fn choice(params: &[f32], i: usize, n: usize) -> usize {
    (params[i].round().max(0.0) as usize).min(n - 1)
}

const BRONZE: Color32 = Color32::from_rgb(0xd8, 0xa8, 0x50);

// ---------------------------------------------------------------- gamelan

/// The gong cycle as a circle (the gong at the top, kenong at the quarters,
/// kempul between), the core notes round it as numbers, and the last steps
/// of the two parts: polos above, sangsih below.
pub(crate) fn gamelan(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 330.0, height);
    let (pic, side) = split(rect);
    let vis = live.on();
    let n = vis.map(|v| (v[0] >> 10 & 0x3f) as usize).unwrap_or([8, 16, 32][choice(params, wo::GA_CYCLE, 3)]).clamp(8, 32);
    let j = vis.map(|v| (v[0] >> 4 & 0x3f) as usize);
    let hits = vis.map(|v| v[0] >> 18 & 7).unwrap_or(0);
    let core: Vec<i32> = match vis {
        Some(v) => (0..16).map(|i| (v[1] >> (i * 4) & 0xf) as i32 - 4).collect(),
        None => wo::balungan(params[wo::GA_MELODY].round() as u32, 0, 0.0, n)[..16].iter().map(|&d| d as i32).collect(),
    };
    let half = vis.map(|v| (v[2] >> 24 & 0xff) as usize).unwrap_or(0);
    let ring = Rect::from_min_max(pic.min, pos2(pic.right(), pic.bottom() - 30.0));
    let c = ring.center();
    let rad = ring.height().min(ring.width()) / 2.0 - 10.0;
    p.circle_stroke(c, rad, Stroke::new(1.0_f32, LINE_SOFT));
    let quarter = n / 4;
    for pos in 1..=n {
        let a = pos as f32 / n as f32 * TAU - FRAC_PI_2;
        let q = c + vec2(a.cos(), a.sin()) * rad;
        let now = j == Some(pos - 1);
        // The gongs: the big one at the end, kenong at the quarters, kempul between.
        let (size, kind) = if pos == n {
            (8.0, 1)
        } else if pos % quarter == 0 {
            (5.5, 2)
        } else if pos % quarter == quarter / 2 && pos != quarter / 2 {
            (4.0, 4)
        } else {
            (2.0, 0)
        };
        let struck = now && hits & kind != 0;
        if kind != 0 {
            p.circle_stroke(q, size, Stroke::new(1.5_f32, if struck { Color32::WHITE } else { BRONZE.gamma_multiply(0.7) }));
        }
        p.circle_filled(q, if now { 3.5 } else { 1.8 }, if now { Color32::WHITE } else { BRONZE.gamma_multiply(0.6) });
        // The core note, as a number (1 to 5, or 7), where this half of the cycle is.
        let k = pos - 1;
        if k >= half && k < half + 16 && n <= 32 {
            let d = core[k - half];
            let num = if choice(params, wo::GA_SCALE, 2) == 0 { d.rem_euclid(5) + 1 } else { d.rem_euclid(7) + 1 };
            let tq = c + vec2(a.cos(), a.sin()) * (rad - 14.0);
            text(&p, tq, Align2::CENTER_CENTER, num, small(), if now { TEXT } else { TEXT_DIM });
        }
    }
    if hits & 1 != 0 {
        p.circle_filled(c, rad * 0.35, BRONZE.gamma_multiply(0.35));
    }
    // The parts' last steps: one line made of two.
    let trail_id = id.with("parts");
    let mut steps: Vec<(u32, u8)> = ui.ctx().memory(|m| m.data.get_temp(trail_id).unwrap_or_default());
    if let Some(v) = vis {
        let key = (v[0] & 0x3ff) as u32 | ((v[0] >> 24 & 0xffff) as u32) << 10;
        if steps.last().map(|s| s.0) != Some(key) {
            steps.push((key, (v[2] >> 16 & 3) as u8));
            if steps.len() > 24 {
                steps.remove(0);
            }
            ui.ctx().memory_mut(|m| m.data.insert_temp(trail_id, steps.clone()));
        }
        ui.ctx().request_repaint();
    }
    let lanes = Rect::from_min_max(pos2(pic.left(), pic.bottom() - 24.0), pic.max);
    let cw = lanes.width() / 24.0;
    for (i, &(_, bits)) in steps.iter().enumerate() {
        for (lane, col) in [(0, accent), (1, SELECT)] {
            let r = Rect::from_min_size(pos2(lanes.left() + i as f32 * cw, lanes.top() + lane as f32 * 12.0), vec2(cw - 1.5, 10.0));
            rect_fill(&p, r, 1.5, if bits >> lane & 1 == 1 { col } else { PANEL3 });
        }
    }
    let mut changed = pick_choice(ui, slot(side, 0), id.with("ga_irama"), params, wo::GA_IRAMA, wo::IRAMAS, "Irama", accent);
    changed |= pick_choice(ui, slot(side, 1), id.with("ga_style"), params, wo::GA_STYLE, wo::KOTEKAN, "Style", accent);
    changed |= pick_choice(ui, slot(side, 2), id.with("ga_cycle"), params, wo::GA_CYCLE, wo::GONG_CYCLES, "Cycle", accent);
    changed |= pick_choice(ui, slot(side, 3), id.with("ga_scale"), params, wo::GA_SCALE, wo::GAMELAN_SCALES, "Notes", accent);
    changed |= pick_choice(ui, slot(side, 4), id.with("ga_change"), params, wo::GA_CHANGE, wo::IRAMA_CHANGES, "Change", accent);
    changed |= fader_at(ui, slot(side, 5), id, params, wo::GAMELAN, wo::GA_VARIATION, accent);
    // The three layers, each on or off.
    let s = slot(side, 6);
    let bw = (s.width() - 4.0) / 3.0;
    for (k, (i, label)) in [(wo::GA_GONGS, "Gongs"), (wo::GA_CORE, "Core"), (wo::GA_PARTS, "Parts")].into_iter().enumerate() {
        let r = Rect::from_min_size(pos2(s.left() + k as f32 * (bw + 2.0), s.top()), vec2(bw, s.height()));
        let on = params[i] >= 0.5;
        if button(ui, r, id.with(("layer", k)), label, on, accent).on_hover_text("Play this layer or leave it out").clicked() {
            params[i] = if on { 0.0 } else { 1.0 };
            changed = true;
        }
    }
    changed
}

// ---------------------------------------------------------------- raga

const SARGAM: [&str; 12] = ["Sa", "re", "Re", "ga", "Ga", "ma", "Ma", "Pa", "dha", "Dha", "ni", "Ni"];

/// The raga's notes going up (left) and coming down (right), the resting
/// notes ringed, the one it is on lit on the side it is going; the last
/// notes between; the way through Alap, Jor and Jhala below.
pub(crate) fn raga(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 330.0, height);
    let (pic, side) = split(rect);
    let which = choice(params, wo::RA_RAGA, wo::RAGAS.len());
    let r = wo::raga_notes(which, &live.scale);
    let vis = live.on();
    let notes: Vec<i32> = vis.map(|v| (0..8).rev().map(|i| (v[1] >> (8 * i) & 0xff) as i32).filter(|&b| b > 0).map(|b| b - 64).collect()).unwrap_or_default();
    let now = notes.last().copied();
    let rising = notes.len() >= 2 && notes[notes.len() - 1] > notes[notes.len() - 2];
    let ladder = Rect::from_min_max(pic.min, pos2(pic.right(), pic.bottom() - 22.0));
    let row_h = ladder.height() / 13.0;
    let y_of = |semi: i32| ladder.bottom() - (semi as f32 + 0.5) * row_h;
    let col_w = 34.0;
    for (side_x, set, n, up) in [(ladder.left(), &r.up, r.n_up, true), (ladder.right() - col_w, &r.down, r.n_down, false)] {
        text(&p, pos2(side_x + col_w / 2.0, ladder.top()), Align2::CENTER_TOP, if up { "up" } else { "down" }, small(), TEXT_DIM);
        for semi in 0..=12 {
            let pc = semi % 12;
            if !set[..n].contains(&(pc as i8)) {
                continue;
            }
            let q = pos2(side_x + col_w / 2.0, y_of(semi));
            let lit = now.is_some_and(|x| x.rem_euclid(12) == pc && rising == up);
            let r_box = Rect::from_center_size(q, vec2(col_w - 4.0, row_h - 1.0));
            rect_fill(&p, r_box, 2.0, if lit { accent } else { PANEL3 });
            if r.rests_on(pc) {
                rect_line(&p, r_box, 2.0, 1.0, shade(accent, 0.8));
            }
            text(&p, q, Align2::CENTER_CENTER, if semi == 12 { "Sa'" } else { SARGAM[pc as usize] }, small(), if lit { TEXT_DARK } else { TEXT });
        }
    }
    // The last notes, between the ladders.
    let mid = Rect::from_min_max(pos2(ladder.left() + col_w + 8.0, ladder.top()), pos2(ladder.right() - col_w - 8.0, ladder.bottom()));
    if notes.len() > 1 {
        let dx = mid.width() / 8.0;
        let pts: Vec<egui::Pos2> = notes.iter().enumerate().map(|(i, &x)| pos2(mid.left() + (i as f32 + 0.5) * dx, y_of(x.clamp(-1, 13)))).collect();
        p.add(Shape::line(pts.clone(), Stroke::new(1.2_f32, accent.gamma_multiply(0.5))));
        for (i, q) in pts.iter().enumerate() {
            p.circle_filled(*q, if i + 1 == pts.len() { 4.0 } else { 2.5 }, accent.gamma_multiply(if i + 1 == pts.len() { 1.0 } else { 0.55 }));
        }
    }
    if vis.is_some_and(|v| v[0] >> 10 & 1 == 1) {
        text(&p, mid.center_top() + vec2(0.0, 10.0), Align2::CENTER_TOP, "pakad", small(), accent);
    }
    // The way: Alap, Jor, Jhala.
    let bar = Rect::from_min_max(pos2(pic.left(), pic.bottom() - 16.0), pic.max);
    let stage_param = choice(params, wo::RA_STAGE, wo::STAGES.len());
    let stage_now = vis.map(|v| (v[0] >> 8 & 3) as usize).unwrap_or(if stage_param < 3 { stage_param } else { 0 });
    let (bounds, labels) = if stage_param == 3 { ([0.0, 0.4, 0.75, 1.0], ["Alap", "Jor", "Jhala"]) } else { ([0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0], ["Alap", "Jor", "Jhala"]) };
    for s in 0..3 {
        let r_s = Rect::from_min_max(pos2(bar.left() + bounds[s] * bar.width(), bar.top()), pos2(bar.left() + bounds[s + 1] * bar.width() - 2.0, bar.bottom()));
        let on = s == stage_now;
        rect_fill(&p, r_s, 2.0, if on { shade(accent, 0.8) } else { PANEL3 });
        text(&p, r_s.center(), Align2::CENTER_CENTER, labels[s], small(), if on { TEXT_DARK } else { TEXT_DIM });
    }
    if let (Some(v), 3) = (vis, stage_param) {
        let x = bar.left() + (v[0] >> 16 & 0xff) as f32 / 255.0 * bar.width();
        p.line_segment([pos2(x, bar.top() - 2.0), pos2(x, bar.bottom() + 2.0)], Stroke::new(2.0_f32, Color32::WHITE));
    }
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    let specs = wo::RAGA;
    let mut changed = pick_choice(ui, slot(side, 0), id.with("ra_raga"), params, wo::RA_RAGA, wo::RAGAS, "Raga", accent);
    changed |= pick_choice(ui, slot(side, 1), id.with("ra_stage"), params, wo::RA_STAGE, wo::STAGES, "Stage", accent);
    changed |= fader_at(ui, slot(side, 2), id, params, specs, wo::RA_PAKAD, accent);
    changed |= fader_at(ui, slot(side, 3), id, params, specs, wo::RA_ORNAMENTS, accent);
    changed |= fader_at(ui, slot(side, 4), id, params, specs, wo::RA_LINGER, accent);
    let on = if params[wo::RA_DRONE] >= 0.5 { "On" } else { "Off" };
    changed |= pick(ui, slot(side, 5), id.with("ra_drone"), &mut params[wo::RA_DRONE], 0.0, 1.0, "Drone", on, accent);
    changed |= pick_choice(ui, slot(side, 6), id.with("ra_dn"), params, wo::RA_DRONE_NOTE, wo::DRONE_NOTES, "With", accent);
    changed
}

// ---------------------------------------------------------------- call

/// The call and its answer, a note a dot (higher is higher), the step now lit.
pub(crate) fn call(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 330.0, height);
    let (pic, side) = split(rect);
    let vis = live.on();
    let unpack = |w: u64| -> Vec<Option<i32>> { (0..PHRASE_LEN).map(|i| (w >> (i * 4) & 0xf) as i32).map(|x| if x == 15 { None } else { Some(x - 5) }).collect() };
    let (call, answer, kind) = match vis {
        Some(v) => (unpack(v[1]), unpack(v[2]), (v[0] >> 42 & 7) as usize),
        None => {
            let (c, a, k) = wo::call_and_answer(params, 0);
            (c.iter().map(|d| d.map(|d| d as i32)).collect(), a.iter().map(|d| d.map(|d| d as i32)).collect(), k)
        }
    };
    let len = (params[wo::CR_LENGTH].round() as usize).clamp(2, PHRASE_LEN);
    let gap = params[wo::CR_GAP].round() as usize;
    let pos = vis.map(|v| (v[0] & 0xff) as usize);
    let phase = vis.map(|v| (v[0] >> 40 & 3) as usize);
    let row_h = (pic.height() - 18.0) / 2.0;
    for (r, (name, ph, col)) in [("call", &call, accent), ("answer", &answer, SELECT)].into_iter().enumerate() {
        let strip = Rect::from_min_size(pos2(pic.left(), pic.top() + r as f32 * (row_h + 6.0)), vec2(pic.width(), row_h));
        rect_fill(&p, strip, 3.0, PANEL3);
        text(&p, pos2(strip.left() + 4.0, strip.top() + 2.0), Align2::LEFT_TOP, name, small(), TEXT_DIM);
        let cw = (strip.width() - 8.0) / len as f32;
        let now = match (phase, pos) {
            (Some(0), Some(q)) if r == 0 => Some(q),
            (Some(1), Some(q)) if r == 1 => q.checked_sub(len + gap),
            _ => None,
        };
        for i in 0..len {
            let Some(d) = ph[i] else { continue };
            let q = pos2(strip.left() + 4.0 + (i as f32 + 0.5) * cw, strip.bottom() - 6.0 - ((d + 5) as f32 / 16.0).clamp(0.0, 1.0) * (strip.height() - 18.0));
            let on = now == Some(i);
            p.circle_filled(q, if on { 5.0 } else { 3.5 }, if on { Color32::WHITE } else { col.gamma_multiply(0.75) });
        }
    }
    text(&p, pos2(pic.left(), pic.bottom()), Align2::LEFT_BOTTOM, format!("answers: {}", wo::ANSWERS[kind.min(wo::ANSWERS.len() - 1)]), small(), TEXT_DIM);
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    let mut changed = pick_choice(ui, slot(side, 0), id.with("cr_answer"), params, wo::CR_ANSWER, wo::ANSWERS, "Answer", accent);
    changed |= pick_count(ui, slot(side, 1), id.with("cr_choir"), params, wo::CR_CHOIR, 1.0, 3.0, "Choir", accent);
    changed |= pick_count(ui, slot(side, 2), id.with("cr_len"), params, wo::CR_LENGTH, 2.0, 16.0, "Length", accent);
    changed |= pick_count(ui, slot(side, 3), id.with("cr_gap"), params, wo::CR_GAP, 0.0, 8.0, "Gap", accent);
    changed |= pick_choice(ui, slot(side, 4), id.with("cr_phrase"), params, wo::CR_PHRASE, krabton_engine::sequencers::systems::PHRASES, "Phrase", accent);
    changed |= fader_at(ui, slot(side, 5), id, params, wo::CALL, wo::CR_CHANGE, accent);
    changed |= roll(ui, slot(side, 6), id.with("cr_seed"), params, wo::CR_SEED, 99.0, "New Call", "A new call (Seed)", accent);
    changed
}
