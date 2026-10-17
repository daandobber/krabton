//! Editors for the third round of composers' sequencers: Mozart (the dice
//! table, or the period's bars), Chopin (the chords, the line, how late the
//! right hand is; a Prelude's sinking chord), Ravel (the crescendo, the
//! layers, the drum's two bars), Bartók (the metre's groups, the night's
//! noises, the mirror), Messiaen (the mode and its chord, the bird's phrase,
//! the rhythmic characters), Ligeti (the metronomes swinging, the two hands'
//! notes, the pitch classes so far), Cage (the hexagram, the square-root
//! form, the clock) and Einaudi (the chords, the layers). They show what
//! the engine reports.

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::sequencers::masters as ms;
use std::f32::consts::{FRAC_PI_2, TAU};

const NUM: [&str; 7] = ["I", "ii", "iii", "IV", "V", "vi", "vii°"];

fn choice(params: &[f32], i: usize, n: usize) -> usize {
    (params[i].round().max(0.0) as usize).min(n - 1)
}

fn frame(ui: &mut Ui, height: f32) -> (Rect, egui::Painter) {
    let (rect, _) = ui.allocate_exact_size(vec2(260.0, height), Sense::hover());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    (rect, p)
}

fn footer(p: &egui::Painter, rect: Rect, s: impl ToString) {
    text(p, pos2(rect.left() + 8.0, rect.bottom() - 4.0), Align2::LEFT_BOTTOM, s, small(), TEXT_DIM);
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

fn staff(p: &egui::Painter, area: Rect) {
    for l in 0..5 {
        let y = area.top() + l as f32 * area.height() / 4.0;
        p.line_segment([pos2(area.left(), y), pos2(area.right(), y)], Stroke::new(1.0_f32, LINE_SOFT));
    }
}

/// A row of boxes, one lit, each with its label.
fn boxes(p: &egui::Painter, row: Rect, labels: &[String], lit: Option<usize>, accent: Color32) {
    let n = labels.len().max(1);
    let cw = row.width() / n as f32;
    for (i, l) in labels.iter().enumerate() {
        let r = Rect::from_min_size(pos2(row.left() + i as f32 * cw, row.top()), vec2(cw - 3.0, row.height()));
        let on = lit == Some(i);
        rect_fill(p, r, 3.0, if on { accent } else { PANEL3 });
        text(p, r.center(), Align2::CENTER_CENTER, l, small(), if on { TEXT_DARK } else { TEXT });
    }
}

/// Eight degrees (four bits each) as numerals.
fn numerals(packed: u64, n: usize) -> Vec<String> {
    (0..n).map(|i| NUM[((packed >> (4 * i)) & 0xf) as usize % 7].to_string()).collect()
}

fn repaint(ui: &Ui, vis: Option<[u64; 4]>) {
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
}

// ---------------------------------------------------------------- mozart

/// The dice game: its table, sixteen bars across and the eleven throws
/// down, the bar each throw chose lit, and the dice. Or the period's bars
/// and the melody.
pub(crate) fn mozart(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, height);
    let vis = live.on();
    let style = choice(params, ms::MO_STYLE, ms::MOZART_STYLES.len());
    if style == 0 {
        let seed = params[ms::MO_DICE].round().max(0.0) as u32;
        let (table, now, dice) = match vis {
            Some(v) => (v[1], ((v[0] >> 4) & 0xf) as usize, Some((((v[0] >> 12) & 0xf) as u32, ((v[0] >> 16) & 0xf) as u32))),
            None => ((0..16).fold(0u64, |acc, i| {
                let (a, b) = ms::dice(seed, 0, i);
                acc | ((a + b - 2) as u64) << (4 * i)
            }), usize::MAX, None),
        };
        let area = Rect::from_min_max(pos2(rect.left() + 8.0, rect.top() + 8.0), pos2(rect.right() - 8.0, rect.bottom() - 30.0));
        let cw = area.width() / 16.0;
        let ch = area.height() / 11.0;
        for col in 0..16 {
            let pick = ((table >> (4 * col)) & 0xf) as usize;
            for row in 0..11 {
                let r = Rect::from_min_size(pos2(area.left() + col as f32 * cw, area.top() + row as f32 * ch), vec2(cw - 1.0, ch - 1.0));
                let chosen = row == pick;
                let c = if chosen && col == now {
                    accent
                } else if chosen {
                    accent.gamma_multiply(0.45)
                } else if col == now {
                    PANEL3.gamma_multiply(1.4)
                } else {
                    PANEL3
                };
                rect_fill(&p, r, 1.0, c);
            }
        }
        // The two dice.
        if let Some((a, b)) = dice {
            for (j, d) in [a, b].iter().enumerate() {
                let r = Rect::from_min_size(pos2(rect.right() - 52.0 + j as f32 * 24.0, rect.bottom() - 26.0), vec2(20.0, 20.0));
                rect_fill(&p, r, 3.0, TEXT);
                const PIPS: [&[(f32, f32)]; 6] = [&[(0.5, 0.5)], &[(0.25, 0.25), (0.75, 0.75)], &[(0.25, 0.25), (0.5, 0.5), (0.75, 0.75)], &[(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)], &[(0.25, 0.25), (0.75, 0.25), (0.5, 0.5), (0.25, 0.75), (0.75, 0.75)], &[(0.25, 0.2), (0.75, 0.2), (0.25, 0.5), (0.75, 0.5), (0.25, 0.8), (0.75, 0.8)]];
                for &(x, y) in PIPS[(*d as usize).clamp(1, 6) - 1] {
                    p.circle_filled(pos2(r.left() + x * r.width(), r.top() + y * r.height()), 1.8, TEXT_DARK);
                }
            }
        }
        footer(&p, rect, if now < 16 { format!("bar {} of 16", now + 1) } else { "two dice for every bar".to_string() });
    } else {
        let (b, chords, refrain, hist) = match vis {
            Some(v) => (((v[0] >> 4) & 0xf) as usize, v[1], (v[0] >> 24) & 1 == 1, v[2]),
            None => (usize::MAX, if style == 1 { 0x0430_4040 } else { 0x0430_0000 }, false, 0),
        };
        let row = Rect::from_min_size(pos2(rect.left() + 8.0, rect.top() + 8.0), vec2(244.0, 22.0));
        boxes(&p, row, &numerals(chords, 8), if b < 8 { Some(b) } else { None }, accent);
        let area = Rect::from_min_max(pos2(rect.left() + 10.0, row.bottom() + 12.0), pos2(rect.right() - 10.0, rect.bottom() - 24.0));
        staff(&p, area);
        if vis.is_some() {
            trail(&p, area, hist, accent, -4, 12);
        }
        let what = match (style, b) {
            (1, 6) => "Sonata · the trill".to_string(),
            (1, 3) => "Sonata · half cadence".to_string(),
            (1, _) => "Sonata · Alberti bass".to_string(),
            (_, _) if refrain => "Alla Turca · refrain".to_string(),
            _ => "Alla Turca".to_string(),
        };
        footer(&p, rect, what);
    }
    repaint(ui, vis);
    false
}

// ---------------------------------------------------------------- chopin

/// The chords (the one now lit), the right hand's line, and how far behind
/// the left hand it is. A Prelude: its three voices sinking.
pub(crate) fn chopin(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, height);
    let vis = live.on();
    let style = choice(params, ms::CH_STYLE, ms::CHOPIN_STYLES.len());
    let roots = ms::CHOPIN_ROOTS[choice(params, ms::CH_HARMONY, ms::CHOPIN_HARMONIES.len())];
    let cur = vis.map(|v| ((v[0] >> 4) & 0xf) as usize);
    let labels: Vec<String> = roots.iter().map(|&r| live.scale.chord_name(r as usize)).collect();
    let row = Rect::from_min_size(pos2(rect.left() + 8.0, rect.top() + 8.0), vec2(244.0, 22.0));
    if style == 2 {
        // The three voices, falling semitone by semitone.
        let area = Rect::from_min_max(pos2(rect.left() + 10.0, rect.top() + 10.0), pos2(rect.right() - 10.0, rect.bottom() - 24.0));
        staff(&p, area);
        if let Some(v) = vis {
            let moves = ((v[2] >> 24) & 0xff) as f32;
            let cadence = (v[2] >> 32) & 0xff;
            for i in 0..3 {
                let pitch = ((v[2] >> (8 * i)) & 0xff) as f32;
                let y = area.bottom() - ((pitch - 36.0) / 48.0).clamp(0.0, 1.0) * area.height();
                p.circle_filled(pos2(area.left() + 30.0 + i as f32 * 14.0, y), 5.0, accent.gamma_multiply(0.85));
            }
            let x = area.left() + 90.0;
            trail(&p, Rect::from_min_max(pos2(x, area.top()), area.max), v[1], accent, -4, 10);
            footer(&p, rect, if cadence > 0 { "Prelude · cadence".to_string() } else { format!("Prelude · {moves:.0} steps down") });
        } else {
            footer(&p, rect, "Prelude: chords sinking a semitone at a time");
        }
    } else {
        boxes(&p, row, &labels, cur, accent);
        let area = Rect::from_min_max(pos2(rect.left() + 10.0, row.bottom() + 12.0), pos2(rect.right() - 10.0, rect.bottom() - 30.0));
        staff(&p, area);
        if let Some(v) = vis {
            trail(&p, area, v[1], accent, -1, 12);
            // How late the right hand is.
            let late = (v[2] & 0xff) as f32 / 255.0;
            let bar = Rect::from_min_size(pos2(rect.right() - 90.0, rect.bottom() - 16.0), vec2(80.0, 6.0));
            rect_fill(&p, bar, 2.0, PANEL3);
            rect_fill(&p, Rect::from_min_size(bar.min, vec2(bar.width() * (late * 3.0).min(1.0), bar.height())), 2.0, accent.gamma_multiply(0.7));
        }
        footer(&p, rect, if style == 0 { "Nocturne · rubato" } else { "Waltz" });
    }
    repaint(ui, vis);
    false
}

// ---------------------------------------------------------------- ravel

/// The crescendo as a hairpin with where it is, the layers in so far, the
/// side drum's two bars.
pub(crate) fn ravel(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, height);
    let vis = live.on();
    let max_layers = params[ms::RA_LAYERS].round().clamp(1.0, 5.0) as usize;
    let (x, layers, second, silent, ending, pos) = match vis {
        Some(v) => ((v[0] & 0xff) as f32 / 255.0, ((v[0] >> 8) & 0xf) as usize, (v[0] >> 12) & 1 == 1, (v[0] >> 13) & 1 == 1, (v[0] >> 14) & 1 == 1, ((v[0] >> 16) & 0xff) as u8),
        None => (0.0, 0, false, false, false, u8::MAX),
    };
    // The hairpin.
    let hp = Rect::from_min_max(pos2(rect.left() + 12.0, rect.top() + 12.0), pos2(rect.right() - 12.0, rect.top() + 54.0));
    let mid = hp.center().y;
    p.add(Shape::line(vec![pos2(hp.right(), hp.top()), pos2(hp.left(), mid), pos2(hp.right(), hp.bottom())], Stroke::new(1.5_f32, TEXT_DIM)));
    if vis.is_some() {
        let xx = hp.left() + x * hp.width();
        let half = x * hp.height() / 2.0;
        p.add(Shape::convex_polygon(vec![pos2(hp.left(), mid), pos2(xx, mid - half), pos2(xx, mid + half)], accent.gamma_multiply(0.5), Stroke::NONE));
        p.line_segment([pos2(xx, hp.top()), pos2(xx, hp.bottom())], Stroke::new(1.5_f32, accent));
    }
    // Where each layer comes in.
    for (i, &at) in ms::RAVEL_LAYER_AT.iter().enumerate().take(max_layers) {
        let xx = hp.left() + at * hp.width();
        p.circle_filled(pos2(xx + 4.0, hp.bottom() + 9.0), 3.5, if i < layers { accent } else { PANEL3 });
    }
    // The side drum's two bars.
    let dr = Rect::from_min_max(pos2(rect.left() + 12.0, hp.bottom() + 22.0), pos2(rect.right() - 12.0, hp.bottom() + 42.0));
    for half in 0..2 {
        let y = dr.top() + half as f32 * 10.0 + 4.0;
        for &b in ms::BOLERO.iter().filter(|&&b| (b / 36) as usize == half) {
            let xx = dr.left() + (b % 36) as f32 / 36.0 * dr.width();
            let now = vis.is_some() && b == pos;
            p.circle_filled(pos2(xx, y), if now { 3.5 } else { 2.2 }, if now { accent } else { TEXT_DIM });
        }
    }
    let label = match vis {
        None => "Boléro: louder and louder".to_string(),
        Some(_) if silent => "silence".to_string(),
        Some(_) if ending => "the crash".to_string(),
        Some(_) => format!("tune {} · {} {}", if second { "B" } else { "A" }, layers, if layers == 1 { "layer" } else { "layers" }),
    };
    footer(&p, rect, label);
    repaint(ui, vis);
    false
}

// ---------------------------------------------------------------- bartók

/// The dance's metre as its groups of quavers (the one now lit); the night
/// (its insects up high, the cluster low); the mirror (the line and its
/// reflection).
pub(crate) fn bartok(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, height);
    let vis = live.on();
    let style = choice(params, ms::BT_STYLE, ms::BARTOK_STYLES.len());
    match style {
        0 => {
            let groups = ms::METERS[choice(params, ms::BT_METER, ms::BARTOK_METERS.len())];
            let total: u8 = groups.iter().sum();
            let now = vis.map(|v| ((v[0] >> 4) & 0xff) as usize);
            let row = Rect::from_min_size(pos2(rect.left() + 8.0, rect.top() + 8.0), vec2(244.0, 26.0));
            let qw = row.width() / total as f32;
            let mut q = 0usize;
            for &g in groups {
                let gr = Rect::from_min_size(pos2(row.left() + q as f32 * qw, row.top()), vec2(g as f32 * qw - 4.0, row.height()));
                rect_line(&p, gr, 3.0, 1.0, accent.gamma_multiply(0.6));
                for j in 0..g as usize {
                    let c = pos2(gr.left() + (j as f32 + 0.5) * qw - 2.0, gr.center().y);
                    let lit = now == Some(q + j);
                    p.circle_filled(c, if j == 0 { 5.0 } else { 3.5 }, if lit { accent } else if j == 0 { TEXT } else { TEXT_DIM });
                }
                q += g as usize;
            }
            let area = Rect::from_min_max(pos2(rect.left() + 10.0, row.bottom() + 12.0), pos2(rect.right() - 10.0, rect.bottom() - 24.0));
            staff(&p, area);
            if let Some(v) = vis {
                trail(&p, area, v[1], accent, -3, 9);
            }
            footer(&p, rect, format!("{}  ·  {}", ms::BARTOK_METERS[choice(params, ms::BT_METER, ms::BARTOK_METERS.len())], ms::BARTOK_MODES[choice(params, ms::BT_MODE, ms::BARTOK_MODES.len())].to_lowercase()));
        }
        1 => {
            let sky = Rect::from_min_max(pos2(rect.left() + 8.0, rect.top() + 8.0), pos2(rect.right() - 8.0, rect.bottom() - 24.0));
            rect_fill(&p, sky, 3.0, Color32::from_rgb(0x14, 0x16, 0x22));
            if let Some(v) = vis {
                for i in 0..8 {
                    let d = ((v[2] >> (8 * i)) & 0xff) as i32;
                    if d == 0 {
                        continue;
                    }
                    let off = d - 64;
                    let xx = sky.left() + ((i as f32 * 0.37 + off as f32 * 0.11).fract()) * sky.width();
                    let y = sky.bottom() - ((off + 26) as f32 / 66.0).clamp(0.0, 1.0) * sky.height();
                    p.circle_filled(pos2(xx, y), if i == 0 { 3.5 } else { 2.0 }, Color32::from_rgb(0xd8, 0xf0, 0x9a).gamma_multiply(1.0 - i as f32 * 0.1));
                }
                let c = ((v[0] >> 16) & 0xff) as f32 - 8.0;
                let cl = Rect::from_min_size(pos2(sky.center().x - 30.0 + c * 6.0, sky.bottom() - 14.0), vec2(60.0, 8.0));
                rect_fill(&p, cl, 2.0, accent.gamma_multiply(0.5));
            }
            footer(&p, rect, "night music");
        }
        _ => {
            let area = Rect::from_min_max(pos2(rect.left() + 10.0, rect.top() + 10.0), pos2(rect.right() - 10.0, rect.bottom() - 24.0));
            let axis = area.center().y;
            p.line_segment([pos2(area.left(), axis), pos2(area.right(), axis)], Stroke::new(1.0_f32, TEXT_DIM));
            if let Some(v) = vis {
                let notes: Vec<i32> = (0..8).rev().map(|i| ((v[1] >> (8 * i)) & 0xff) as i32).filter(|&d| d > 0).map(|d| d - 64).collect();
                let dx = area.width() / 9.0;
                let sy = area.height() / 2.0 / 13.0;
                for (sign, col) in [(1.0, accent), (-1.0, SELECT)] {
                    let pts: Vec<egui::Pos2> = notes.iter().enumerate().map(|(i, &s)| pos2(area.left() + (i as f32 + 1.0) * dx, axis - sign * (s as f32 + 6.0) * sy)).collect();
                    if pts.len() > 1 {
                        p.add(Shape::line(pts.clone(), Stroke::new(1.0_f32, col.gamma_multiply(0.5))));
                    }
                    for q in &pts {
                        p.circle_filled(*q, 3.0, col);
                    }
                }
            }
            footer(&p, rect, "mirror: semitone for semitone");
        }
    }
    repaint(ui, vis);
    false
}

// ---------------------------------------------------------------- messiaen

/// The mode on a circle of twelve, the chord now lit; or the bird's phrase
/// (each note a dot by height, those sung lit); or the three rhythmic
/// characters, their durations as blocks.
pub(crate) fn messiaen(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, height);
    let vis = live.on();
    let style = choice(params, ms::ME_STYLE, ms::MESSIAEN_STYLES.len());
    let mode_i = choice(params, ms::ME_MODE, ms::MESSIAEN_MODES.len());
    match style {
        0 => {
            let mode = ms::MODES[mode_i];
            let c = pos2(rect.left() + 64.0, rect.center().y - 8.0);
            let r = (rect.height() * 0.32).min(50.0);
            let tonic = live.scale.root as i32;
            let mask = vis.map(|v| v[2]).unwrap_or(0);
            for pc in 0..12 {
                let a = pc as f32 / 12.0 * TAU - FRAC_PI_2;
                let q = c + vec2(a.cos(), a.sin()) * r;
                let in_mode = mode.contains(&(pc as i32));
                let sounding = (mask >> ((pc + tonic) % 12)) & 1 == 1;
                if sounding {
                    p.circle_filled(q, 6.0, accent);
                } else if in_mode {
                    p.circle_stroke(q, 5.0, Stroke::new(1.5_f32, accent.gamma_multiply(0.7)));
                } else {
                    p.circle_filled(q, 2.0, PANEL3);
                }
            }
            if let Some(v) = vis {
                let area = Rect::from_min_max(pos2(rect.left() + 130.0, rect.top() + 12.0), pos2(rect.right() - 10.0, rect.bottom() - 26.0));
                trail(&p, area, v[1], accent, 6, 30);
            }
            footer(&p, rect, format!("{} · added values", ms::MESSIAEN_MODES[mode_i]));
        }
        1 => {
            let area = Rect::from_min_max(pos2(rect.left() + 10.0, rect.top() + 10.0), pos2(rect.right() - 10.0, rect.bottom() - 24.0));
            rect_fill(&p, area, 3.0, PANEL3.gamma_multiply(0.6));
            if let Some(v) = vis {
                let n = (v[3] & 0xff) as usize;
                let played = ((v[3] >> 8) & 0xff) as usize;
                let shown = n.min(16);
                let dx = area.width() / (shown.max(1) + 1) as f32;
                for i in 0..shown {
                    let word = if i < 8 { v[1] } else { v[2] };
                    let pitch = ((word >> (8 * (i % 8))) & 0xff) as i32 - 64;
                    let y = area.bottom() - ((pitch - 4) as f32 / 40.0).clamp(0.0, 1.0) * area.height();
                    let sung = i < played;
                    p.circle_filled(pos2(area.left() + (i as f32 + 1.0) * dx, y), if sung { 4.0 } else { 2.5 }, if sung { accent } else { TEXT_DIM });
                }
            }
            footer(&p, rect, ms::MESSIAEN_BIRDS[choice(params, ms::ME_BIRD, ms::MESSIAEN_BIRDS.len())].to_lowercase());
        }
        _ => {
            let names = ["grows", "shrinks", "stays"];
            let rh = (rect.height() - 34.0) / 3.0;
            for (v_i, name) in names.iter().enumerate() {
                let row = Rect::from_min_size(pos2(rect.left() + 58.0, rect.top() + 8.0 + v_i as f32 * rh), vec2(rect.width() - 66.0, rh - 6.0));
                text(&p, pos2(rect.left() + 8.0, row.center().y), Align2::LEFT_CENTER, *name, small(), TEXT_DIM);
                if let Some(v) = vis {
                    let word = v[1 + v_i];
                    let cells: Vec<f32> = (0..5).map(|i| ((word >> (8 * i)) & 0xff) as f32).collect();
                    let pos = ((word >> 48) & 0xff) as usize;
                    let total: f32 = cells.iter().sum::<f32>().max(1.0);
                    let scale = row.width() / total.max(20.0);
                    let mut xx = row.left();
                    for (j, &c) in cells.iter().enumerate() {
                        let r = Rect::from_min_size(pos2(xx, row.top()), vec2((c * scale - 2.0).max(1.0), row.height()));
                        let now = (j + 4) % 5 == pos % 5;
                        rect_fill(&p, r, 2.0, if now { accent } else { accent.gamma_multiply(0.3) });
                        xx += c * scale;
                    }
                }
            }
            footer(&p, rect, "rhythmic characters");
        }
    }
    repaint(ui, vis);
    false
}

// ---------------------------------------------------------------- ligeti

/// The metronomes, each swinging until it has run down; or the two hands'
/// notes, the one now lit; or the twelve pitch classes, those let in lit.
pub(crate) fn ligeti(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, height);
    let vis = live.on();
    let style = choice(params, ms::LI_STYLE, ms::LIGETI_STYLES.len());
    match style {
        0 => {
            let n = params[ms::LI_VOICES].round().clamp(2.0, ms::MAX_METRONOMES as f32) as usize;
            let per_row = n.min(8);
            let rows = n.div_ceil(8);
            let cw = (rect.width() - 16.0) / per_row as f32;
            let rh = (rect.height() - 30.0) / rows as f32;
            let (alive, phases) = match vis {
                Some(v) => ((v[0] >> 4) & 0xffff, v[1]),
                None => (0xffff, 0),
            };
            for i in 0..n {
                let col = i % 8;
                let row = i / 8;
                let base = pos2(rect.left() + 8.0 + (col as f32 + 0.5) * cw, rect.top() + 8.0 + (row as f32 + 1.0) * rh - 6.0);
                let hgt = (rh - 14.0).min(54.0);
                let wid = (cw * 0.35).min(10.0);
                let on = (alive >> i) & 1 == 1;
                let col_c = if on { accent } else { TEXT_DIM.gamma_multiply(0.5) };
                p.add(Shape::convex_polygon(vec![pos2(base.x - wid, base.y), pos2(base.x + wid, base.y), pos2(base.x + wid * 0.35, base.y - hgt), pos2(base.x - wid * 0.35, base.y - hgt)], PANEL3, Stroke::new(1.0_f32, col_c)));
                let ph = ((phases >> (4 * i)) & 0xf) as f32 / 16.0;
                let ang = if on && vis.is_some() { 0.45 * (ph * TAU).cos() } else { 0.0 };
                let top = base + vec2(ang.sin() * hgt * 0.9, -ang.cos() * hgt * 0.9);
                p.line_segment([pos2(base.x, base.y - 3.0), top], Stroke::new(1.5_f32, col_c));
            }
            let left = (0..n).filter(|i| (alive >> i) & 1 == 1).count();
            footer(&p, rect, if vis.is_some() { format!("{left} of {n} still ticking") } else { format!("{n} metronomes") });
        }
        1 => {
            let rows = ["right", "left"];
            let rh = (rect.height() - 30.0) / 2.0;
            for (h, name) in rows.iter().enumerate() {
                let row = Rect::from_min_size(pos2(rect.left() + 48.0, rect.top() + 8.0 + h as f32 * rh), vec2(rect.width() - 56.0, rh - 6.0));
                rect_fill(&p, row, 3.0, PANEL3.gamma_multiply(0.6));
                text(&p, pos2(rect.left() + 8.0, row.center().y), Align2::LEFT_CENTER, *name, small(), TEXT_DIM);
                if let Some(v) = vis {
                    let n = ((v[0] >> (4 + 4 * h)) & 0xf) as usize;
                    let count = ((v[0] >> 16) & 0xffff) as usize;
                    let word = v[1 + h];
                    let dx = row.width() / (n.max(1) + 1) as f32;
                    for j in 0..n {
                        let s = ((word >> (8 * j)) & 0xff) as i32 - 64;
                        let y = row.bottom() - ((s + 8) as f32 / 28.0).clamp(0.0, 1.0) * row.height();
                        let now = j == count.wrapping_sub(1) % n.max(1);
                        p.circle_filled(pos2(row.left() + (j as f32 + 1.0) * dx, y), if now { 5.0 } else { 3.0 }, if now { accent } else { accent.gamma_multiply(0.45) });
                    }
                }
            }
            footer(&p, rect, "continuum");
        }
        _ => {
            let c = pos2(rect.center().x, rect.center().y - 8.0);
            let r = (rect.height() * 0.32).min(52.0);
            if let Some(v) = vis {
                let count = ((v[0] >> 4) & 0xff) as usize;
                let last = (v[2] & 0xff) as i32 - 64;
                for i in 0..12 {
                    let pc = ((v[1] >> (4 * i)) & 0xf) as i32;
                    let a = pc as f32 / 12.0 * TAU - FRAC_PI_2;
                    let q = c + vec2(a.cos(), a.sin()) * r;
                    let in_now = i < count;
                    let newest = i + 1 == count;
                    let col = if newest { accent } else if in_now { accent.gamma_multiply(0.55) } else { PANEL3 };
                    p.circle_filled(q, if pc == last && in_now { 7.0 } else { 5.0 }, col);
                }
                text(&p, c, Align2::CENTER_CENTER, count.to_string(), small(), TEXT);
            } else {
                for i in 0..12 {
                    let a = i as f32 / 12.0 * TAU - FRAC_PI_2;
                    p.circle_filled(c + vec2(a.cos(), a.sin()) * r, if i == 0 { 5.0 } else { 3.0 }, if i == 0 { accent } else { PANEL3 });
                }
            }
            footer(&p, rect, "musica ricercata");
        }
    }
    repaint(ui, vis);
    false
}

// ---------------------------------------------------------------- cage

/// The hexagram the coins gave (its moving lines marked) and whether its
/// sound is a silence; or the square-root form; or the clock of 4'33".
pub(crate) fn cage(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, height);
    let vis = live.on();
    let style = choice(params, ms::CA_STYLE, ms::CAGE_STYLES.len());
    match style {
        0 => {
            let lines = vis.map(|v| (v[0] >> 4) & 0xfff);
            let hex = vis.map(|v| (v[0] >> 16) & 0xff);
            let silent = vis.is_some_and(|v| (v[0] >> 24) & 1 == 1);
            let w = 90.0;
            let lh = ((rect.height() - 40.0) / 6.0).min(16.0);
            let x0 = rect.center().x - w / 2.0;
            let top = rect.top() + 10.0;
            for i in 0..6 {
                // The bottom line first.
                let y = top + (5 - i) as f32 * lh + lh * 0.5;
                let (yang, moving) = match lines {
                    Some(l) => {
                        let v = (l >> (2 * i)) & 3;
                        (v & 1 == 1, v == 0 || v == 3)
                    }
                    None => (i % 2 == 0, false),
                };
                let col = if silent { TEXT_DIM } else { accent };
                let th = (lh * 0.5).max(4.0);
                if yang {
                    rect_fill(&p, Rect::from_center_size(pos2(x0 + w / 2.0, y), vec2(w, th)), 1.0, col);
                } else {
                    rect_fill(&p, Rect::from_center_size(pos2(x0 + w * 0.22, y), vec2(w * 0.44, th)), 1.0, col);
                    rect_fill(&p, Rect::from_center_size(pos2(x0 + w * 0.78, y), vec2(w * 0.44, th)), 1.0, col);
                }
                if moving {
                    p.circle_stroke(pos2(x0 + w + 12.0, y), 3.5, Stroke::new(1.2_f32, SELECT));
                }
            }
            let label = match hex {
                Some(h) if silent => format!("{} · silence", h + 1),
                Some(h) => format!("{}", h + 1),
                None => "toss the coins".to_string(),
            };
            footer(&p, rect, label);
        }
        1 => {
            let props = ms::PROPORTIONS[params[ms::CA_COINS].round().max(0.0) as usize % ms::PROPORTIONS.len()];
            let total: f32 = props.iter().map(|&x| x as f32).sum();
            let (section, phrase) = match vis {
                Some(v) => (((v[0] >> 4) & 0xf) as usize, ((v[0] >> 8) & 0xf) as usize),
                None => (usize::MAX, usize::MAX),
            };
            for (row, now) in [(0, section), (1, phrase)] {
                let y = rect.top() + 14.0 + row as f32 * 34.0;
                let mut x = rect.left() + 10.0;
                let wsum = rect.width() - 20.0;
                for (i, &pp) in props.iter().enumerate() {
                    let wv = pp as f32 / total * wsum;
                    let r = Rect::from_min_size(pos2(x, y), vec2(wv - 3.0, 24.0));
                    rect_fill(&p, r, 2.0, if i == now { accent } else { PANEL3 });
                    text(&p, r.center(), Align2::CENTER_CENTER, pp.to_string(), small(), if i == now { TEXT_DARK } else { TEXT_DIM });
                    x += wv;
                }
            }
            let ps: Vec<String> = props.iter().map(|x| x.to_string()).collect();
            footer(&p, rect, format!("prepared piano · {}", ps.join(", ")));
        }
        _ => {
            let secs = vis.map(|v| ((v[0] >> 4) as f64) / 10.0).unwrap_or(0.0);
            let mut acc = 0.0;
            let mut mv = 0;
            for (i, &m) in ms::MOVEMENTS.iter().enumerate() {
                if secs >= acc {
                    mv = i;
                }
                acc += m;
            }
            let big = egui::FontId::proportional(28.0);
            text(&p, pos2(rect.center().x, rect.top() + 36.0), Align2::CENTER_CENTER, format!("{}:{:04.1}", (secs / 60.0) as u32, secs % 60.0), big, TEXT);
            let names = ["I · 30\"", "II · 2'23\"", "III · 1'40\""];
            let row = Rect::from_min_size(pos2(rect.left() + 8.0, rect.top() + 64.0), vec2(244.0, 22.0));
            boxes(&p, row, &names.map(|s| s.to_string()), if vis.is_some() { Some(mv) } else { None }, accent);
            footer(&p, rect, "tacet");
        }
    }
    repaint(ui, vis);
    false
}

// ---------------------------------------------------------------- einaudi

/// The four chords (the one now lit), and the melody; or the layers of the
/// build, the one now lit.
pub(crate) fn einaudi(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, height);
    let vis = live.on();
    let style = choice(params, ms::EI_STYLE, ms::EINAUDI_STYLES.len());
    let roots = ms::EINAUDI_ROOTS[choice(params, ms::EI_CHORDS, ms::EINAUDI_CHORDS.len())];
    let labels: Vec<String> = roots.iter().map(|&r| live.scale.chord_name(r as usize)).collect();
    let row = Rect::from_min_size(pos2(rect.left() + 8.0, rect.top() + 8.0), vec2(244.0, 22.0));
    boxes(&p, row, &labels, vis.map(|v| ((v[0] >> 4) & 0xf) as usize), accent);
    if style == 1 {
        let names = ["pattern", "+ bass", "+ line", "faster", "all", "fade"];
        let stage = vis.map(|v| ((v[0] >> 16) & 0xf) as usize);
        let area = Rect::from_min_max(pos2(rect.left() + 8.0, row.bottom() + 10.0), pos2(rect.right() - 8.0, rect.bottom() - 24.0));
        let cw = area.width() / 6.0;
        for (i, name) in names.iter().enumerate() {
            let hgt = area.height() * [0.3, 0.45, 0.6, 0.8, 1.0, 0.4][i];
            let r = Rect::from_min_max(pos2(area.left() + i as f32 * cw, area.bottom() - hgt), pos2(area.left() + (i as f32 + 1.0) * cw - 3.0, area.bottom()));
            let on = stage == Some(i);
            rect_fill(&p, r, 2.0, if on { accent } else { PANEL3 });
            text(&p, pos2(r.center().x, r.bottom() - 6.0), Align2::CENTER_BOTTOM, *name, small(), if on { TEXT_DARK } else { TEXT_DIM });
        }
        footer(&p, rect, "build: eight bars a layer");
    } else {
        let area = Rect::from_min_max(pos2(rect.left() + 10.0, row.bottom() + 12.0), pos2(rect.right() - 10.0, rect.bottom() - 24.0));
        staff(&p, area);
        if let Some(v) = vis {
            trail(&p, area, v[1], accent, -3, 12);
        }
        footer(&p, rect, "broken chords, a few notes over them");
    }
    repaint(ui, vis);
    false
}
