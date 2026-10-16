//! Editors for the composers' sequencers: Reich (the two players on their
//! rings, or the rhythm against itself, or the chords and the breath), Bach
//! (the chords so far and the top voice), Nancarrow (each voice's way
//! through the melody towards where they meet), Debussy (the chord shape and
//! the line) and Feldman (the pattern, its chords in time, how often it
//! comes back). They show what the engine reports.

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::sequencers::composers as cp;
use std::f32::consts::{FRAC_PI_2, TAU};

fn voice_color(i: usize) -> Color32 {
    [accent_now(), SELECT, PLAY, Color32::from_rgb(0xe0, 0x84, 0xff), Color32::from_rgb(0xff, 0x7a, 0x9c)][i % 5]
}

fn choice(params: &[f32], i: usize, n: usize) -> usize {
    (params[i].round().max(0.0) as usize).min(n - 1)
}

fn frame(ui: &mut Ui, w: f32, height: f32) -> (Rect, egui::Painter) {
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    (rect, p)
}

/// The last notes (a byte each, 64 the tonic) as a line, newest on the right.
fn trail(p: &egui::Painter, area: Rect, packed: u64, accent: Color32, lo: i32, hi: i32) {
    let notes: Vec<i32> = (0..8).rev().map(|i| ((packed >> (8 * i)) & 0xff) as i32).filter(|&d| d > 0).map(|d| d - 64).collect();
    let y_of = |d: i32| area.bottom() - ((d - lo) as f32 / (hi - lo).max(1) as f32).clamp(0.0, 1.0) * area.height();
    let dx = area.width() / 9.0;
    let pts: Vec<egui::Pos2> = notes.iter().enumerate().map(|(i, &d)| pos2(area.left() + (i as f32 + 1.0) * dx, y_of(d))).collect();
    if pts.len() > 1 {
        p.add(Shape::line(pts.clone(), Stroke::new(1.0_f32, accent.gamma_multiply(0.4))));
    }
    for (i, q) in pts.iter().enumerate() {
        p.circle_filled(*q, if i + 1 == pts.len() { 4.5 } else { 3.0 }, accent.gamma_multiply(if i + 1 == pts.len() { 1.0 } else { 0.55 }));
    }
}

// ---------------------------------------------------------------- reich

/// Phase: the figure on two rings, the inner player and the outer one,
/// how far ahead the outer one is. Shifting Rhythm: the rhythm twice, the
/// second turned. Pulses: the chords and the breath.
pub(crate) fn reich(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 260.0, height);
    let vis = live.on();
    let style = choice(params, cp::RE_STYLE, cp::REICH_STYLES.len());
    let pat = (params[cp::RE_PATTERN].round().max(0.0) as usize) % 8;
    match style {
        0 => {
            let fig = cp::reich_figure(pat);
            let c = pos2(rect.center().x, rect.center().y - 6.0);
            let r = (rect.height() * 0.36).min(70.0);
            let (a, b, offset) = match vis {
                Some(v) => (v[1] as usize % 12, v[2] as usize % 12, ((v[0] >> 4) & 0xf) as usize),
                None => (usize::MAX, usize::MAX, 0),
            };
            for (ring, rr, now, col) in [(0, r * 0.62, a, accent), (1, r, b, SELECT)] {
                p.circle_stroke(c, rr, Stroke::new(1.0_f32, LINE_SOFT));
                for (j, &d) in fig.iter().enumerate() {
                    let ang = j as f32 / 12.0 * TAU - FRAC_PI_2;
                    let q = c + vec2(ang.cos(), ang.sin()) * rr;
                    let size = 2.0 + d as f32 * 0.5;
                    p.circle_filled(q, if j == now { size + 3.0 } else { size }, if j == now { col } else { col.gamma_multiply(0.35) });
                }
                let _ = ring;
            }
            text(&p, c, Align2::CENTER_CENTER, if offset == 0 { "together".to_string() } else { format!("+{offset}") }, small(), TEXT);
        }
        1 => {
            let bits = cp::reich_rhythm(pat);
            let (step, offset) = match vis {
                Some(v) => (v[1] as usize % 12, ((v[0] >> 4) & 0xf) as usize),
                None => (usize::MAX, 0),
            };
            let cw = (rect.width() - 20.0) / 12.0;
            for (row, off) in [(0usize, 0usize), (1, offset)] {
                let y = rect.top() + 30.0 + row as f32 * 40.0;
                for j in 0..12 {
                    let hit = bits >> ((j + off) % 12) & 1 == 1;
                    let r = Rect::from_min_size(pos2(rect.left() + 10.0 + j as f32 * cw, y), vec2(cw - 3.0, 26.0));
                    let col = if row == 0 { accent } else { SELECT };
                    let now = j == step;
                    rect_fill(&p, r, 3.0, if hit { if now { col } else { col.gamma_multiply(0.45) } } else if now { PANEL3.gamma_multiply(1.5) } else { PANEL3 });
                }
            }
            text(&p, pos2(rect.left() + 10.0, rect.top() + 110.0), Align2::LEFT_TOP, format!("second player {offset} on"), small(), TEXT_DIM);
        }
        _ => {
            let (ci, bp) = match vis {
                Some(v) => (((v[0] >> 4) & 0xf) as usize, ((v[0] >> 8) & 0xf) as f32),
                None => (0, -1.0),
            };
            const CHORDS: [i32; 6] = [0, 4, 5, 3, 1, 4];
            let cw = (rect.width() - 16.0) / 6.0;
            for (i, &r) in CHORDS.iter().enumerate() {
                let b = Rect::from_min_size(pos2(rect.left() + 8.0 + i as f32 * cw, rect.top() + 8.0), vec2(cw - 4.0, 22.0));
                rect_fill(&p, b, 3.0, if i == ci { accent } else { PANEL3 });
                text(&p, b.center(), Align2::CENTER_CENTER, live.scale.chord_name(r as usize), small(), if i == ci { TEXT_DARK } else { TEXT });
            }
            // The breath: a swell, and where it is.
            let area = Rect::from_min_max(pos2(rect.left() + 12.0, rect.top() + 40.0), pos2(rect.right() - 12.0, rect.bottom() - 26.0));
            let pts: Vec<egui::Pos2> = (0..=32).map(|i| {
                let x = i as f32 / 32.0;
                pos2(area.left() + x * area.width(), area.bottom() - (std::f32::consts::PI * x).sin() * area.height())
            }).collect();
            p.add(Shape::line(pts, Stroke::new(1.5_f32, accent.gamma_multiply(0.6))));
            if bp >= 0.0 {
                let x = (bp + 0.5) / 16.0;
                p.circle_filled(pos2(area.left() + x * area.width(), area.bottom() - (std::f32::consts::PI * x).sin() * area.height()), 5.0, accent);
            }
        }
    }
    text(&p, pos2(rect.left() + 8.0, rect.bottom() - 4.0), Align2::LEFT_BOTTOM, cp::REICH_STYLES[style], small(), TEXT_DIM);
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    false
}

// ---------------------------------------------------------------- bach

/// The chords so far as numerals (the newest lit), and the top voice's line.
pub(crate) fn bach(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 260.0, height);
    let vis = live.on();
    let style = choice(params, cp::BA_STYLE, cp::BACH_STYLES.len());
    const NUM: [&str; 7] = ["I", "ii", "iii", "IV", "V", "vi", "vii°"];
    let hist = vis.map(|v| v[1]).unwrap_or(0);
    let cw = (rect.width() - 16.0) / 8.0;
    for i in 0..8 {
        let r = Rect::from_min_size(pos2(rect.left() + 8.0 + i as f32 * cw, rect.top() + 8.0), vec2(cw - 3.0, 22.0));
        let root = ((hist >> (4 * (7 - i))) & 0xf) as usize;
        let newest = i == 7 && vis.is_some();
        rect_fill(&p, r, 3.0, if newest { accent } else { PANEL3 });
        if vis.is_some() {
            text(&p, r.center(), Align2::CENTER_CENTER, NUM[root % 7], small(), if newest { TEXT_DARK } else { TEXT_DIM });
        }
    }
    let area = Rect::from_min_max(pos2(rect.left() + 10.0, rect.top() + 40.0), pos2(rect.right() - 10.0, rect.bottom() - 22.0));
    for l in 0..5 {
        let y = area.top() + l as f32 * area.height() / 4.0;
        p.line_segment([pos2(area.left(), y), pos2(area.right(), y)], Stroke::new(1.0_f32, LINE_SOFT));
    }
    if let Some(v) = vis {
        trail(&p, area, v[2], accent, -6, 12);
    }
    text(&p, pos2(rect.left() + 8.0, rect.bottom() - 4.0), Align2::LEFT_BOTTOM, cp::BACH_STYLES[style], small(), TEXT_DIM);
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    false
}

// ---------------------------------------------------------------- nancarrow

/// A lane per voice, each filling at its own speed towards the line where
/// they all arrive.
pub(crate) fn nancarrow(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 260.0, height);
    let vis = live.on();
    let ratios = cp::NANCARROW_RATIOS[choice(params, cp::NA_RATIOS, cp::NANCARROW_RATIOS.len())];
    let labels: Vec<&str> = ratios.split(':').collect();
    let nv = if ratios == "Irrational" { 2 } else { labels.len() };
    let area = Rect::from_min_max(pos2(rect.left() + 34.0, rect.top() + 12.0), pos2(rect.right() - 14.0, rect.bottom() - 26.0));
    let row = area.height() / nv as f32;
    let converge = params[cp::NA_CONVERGE] >= 0.5;
    let x_end = if converge { area.right() } else { area.left() };
    p.line_segment([pos2(x_end, area.top()), pos2(x_end, area.bottom())], Stroke::new(1.5_f32, accent.gamma_multiply(0.6)));
    for i in 0..nv {
        let y = area.top() + i as f32 * row;
        let lane = Rect::from_min_size(pos2(area.left(), y + row * 0.25), vec2(area.width(), row * 0.5));
        rect_fill(&p, lane, 3.0, PANEL3);
        let label = if ratios == "Irrational" { ["1", "√2"][i] } else { labels[i] };
        text(&p, pos2(rect.left() + 8.0, lane.center().y), Align2::LEFT_CENTER, label, small(), TEXT_DIM);
        if let Some(v) = vis {
            let pr = ((v[1] >> (8 * i)) & 0xff) as f32 / 255.0;
            if pr > 0.0 && pr < 1.0 {
                let fill = Rect::from_min_size(lane.min, vec2(lane.width() * pr, lane.height()));
                rect_fill(&p, fill, 3.0, voice_color(i).gamma_multiply(0.6));
            }
        }
    }
    text(&p, pos2(rect.left() + 8.0, rect.bottom() - 4.0), Align2::LEFT_BOTTOM, if converge { format!("{ratios}, arriving together") } else { format!("{ratios}, drifting apart") }, small(), TEXT_DIM);
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    false
}

// ---------------------------------------------------------------- debussy

/// The chord shape (its notes as a stack) and the line it follows.
pub(crate) fn debussy(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 260.0, height);
    let vis = live.on();
    let style = choice(params, cp::DE_STYLE, cp::DEBUSSY_STYLES.len());
    let shape = cp::SHAPES[choice(params, cp::DE_SHAPE, cp::DEBUSSY_SHAPES.len())];
    // The stack, on the left.
    let stack = Rect::from_min_max(pos2(rect.left() + 12.0, rect.top() + 12.0), pos2(rect.left() + 52.0, rect.bottom() - 26.0));
    let top = *shape.last().unwrap_or(&12) as f32;
    let show_stack = style == 0;
    if show_stack {
        for &s in shape {
            let y = stack.bottom() - s as f32 / top.max(1.0) * stack.height();
            p.circle_filled(pos2(stack.center().x, y), 4.5, accent.gamma_multiply(0.8));
        }
        p.line_segment([pos2(stack.center().x, stack.top()), pos2(stack.center().x, stack.bottom())], Stroke::new(1.0_f32, LINE_SOFT));
    } else if style == 1 {
        // The whole-tone scale as a ring of six.
        let c = stack.center();
        for j in 0..6 {
            let a = j as f32 / 6.0 * TAU - FRAC_PI_2;
            p.circle_filled(c + vec2(a.cos(), a.sin()) * 16.0, 3.5, accent.gamma_multiply(0.8));
        }
    }
    let area = Rect::from_min_max(pos2(stack.right() + 12.0, rect.top() + 12.0), pos2(rect.right() - 10.0, rect.bottom() - 26.0));
    if let Some(v) = vis {
        trail(&p, area, v[1], accent, -8, 14);
    }
    let what = match style {
        0 => format!("planing: {}", cp::DEBUSSY_SHAPES[choice(params, cp::DE_SHAPE, cp::DEBUSSY_SHAPES.len())].to_lowercase()),
        _ => cp::DEBUSSY_STYLES[style].to_lowercase(),
    };
    text(&p, pos2(rect.left() + 8.0, rect.bottom() - 4.0), Align2::LEFT_BOTTOM, what, small(), TEXT_DIM);
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    false
}

// ---------------------------------------------------------------- feldman

/// The pattern from left to right, its chords as blocks, the playhead, and
/// a dot for each time it will still come back.
pub(crate) fn feldman(ui: &mut Ui, live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 260.0, height);
    let vis = live.on();
    let area = Rect::from_min_max(pos2(rect.left() + 10.0, rect.top() + 20.0), pos2(rect.right() - 10.0, rect.bottom() - 34.0));
    rect_fill(&p, area, 3.0, PANEL3.gamma_multiply(0.6));
    if let Some(v) = vis {
        let len = (v[0] & 0xff).max(1) as f32;
        let pos = ((v[0] >> 8) & 0xff) as f32;
        let left = ((v[0] >> 16) & 0xff) as usize;
        let n = ((v[0] >> 24) & 0xff) as usize;
        let row = area.height() / n.max(1) as f32;
        for i in 0..n.min(cp::MAX_EVENTS) {
            let start = ((v[1] >> (16 * i)) & 0xff) as f32;
            let dur = ((v[1] >> (16 * i + 8)) & 0xff) as f32;
            let x0 = area.left() + start / len * area.width();
            let x1 = (area.left() + (start + dur) / len * area.width()).min(area.right());
            let r = Rect::from_min_max(pos2(x0, area.top() + i as f32 * row + 3.0), pos2(x1, area.top() + (i + 1) as f32 * row - 3.0));
            let sounding = pos >= start && pos < start + dur;
            rect_fill(&p, r, 3.0, if sounding { accent.gamma_multiply(0.8) } else { accent.gamma_multiply(0.3) });
        }
        let x = area.left() + pos / len * area.width();
        p.line_segment([pos2(x, area.top()), pos2(x, area.bottom())], Stroke::new(1.0_f32, TEXT_DIM));
        for i in 0..left.min(8) {
            p.circle_filled(pos2(rect.left() + 14.0 + i as f32 * 10.0, rect.bottom() - 24.0), 3.0, accent.gamma_multiply(0.5));
        }
        ui.ctx().request_repaint();
    }
    text(&p, pos2(rect.left() + 8.0, rect.bottom() - 4.0), Align2::LEFT_BOTTOM, "soft patterns, coming back changed", small(), TEXT_DIM);
    false
}
