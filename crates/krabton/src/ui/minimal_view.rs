//! Editors for the minimal sequencers: Glass (the round of chords and the
//! figure, its notes as dots, how often it repeats), Satie (the bar with its
//! bass and chord, the chord, the melody's last notes on a staff) and Riley
//! (the series of figures with each player on it, the pulse; or the
//! Rainbow's loops as rings). They show what the engine reports.

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::sequencers::minimal as mn;
use std::f32::consts::{FRAC_PI_2, TAU};

fn player_color(i: usize) -> Color32 {
    [accent_now(), SELECT, PLAY, Color32::from_rgb(0xe0, 0x84, 0xff), Color32::from_rgb(0xff, 0x7a, 0x9c)][i % 5]
}

fn choice(params: &[f32], i: usize, n: usize) -> usize {
    (params[i].round().max(0.0) as usize).min(n - 1)
}

/// A chord's name in the key: its root's chord, as a player would say it.
fn chord(live: &Live, degree: i32) -> String {
    live.scale.chord_name(degree.rem_euclid(7) as usize)
}

// ---------------------------------------------------------------- glass

/// The round of chords along the top (the one now lit), the figure below as
/// dots by height (the note now big; triplets above for three over two), and
/// a bar per repeat that fills as the figure comes round.
pub(crate) fn glass(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let w = 260.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let vis = live.on();
    let roots = mn::GLASS_ROOTS[choice(params, mn::G_CHORDS, mn::GLASS_CHORDS.len())];
    let figure = choice(params, mn::G_FIGURE, mn::FIGURES.len());
    let repeats = params[mn::G_REPEATS].round().clamp(1.0, 16.0) as u64;
    let (cur, rep, note, len) = match vis {
        Some(v) => ((v[0] & 0xf) as usize, (v[0] >> 12) & 0x1f, ((v[0] >> 22) & 0x1f) as usize, ((v[0] >> 27) & 0x1f) as usize),
        None => (0, 0, usize::MAX, if figure == 2 { 6 } else if figure == 3 { 2 } else { 4 }),
    };
    // The chords.
    let cw = (w - 16.0) / roots.len() as f32;
    for (i, &r) in roots.iter().enumerate() {
        let c = Rect::from_min_size(pos2(rect.left() + 8.0 + i as f32 * cw, rect.top() + 8.0), vec2(cw - 4.0, 22.0));
        let on = i == cur % roots.len();
        rect_fill(&p, c, 3.0, if on { accent } else { PANEL3 });
        text(&p, c.center(), Align2::CENTER_CENTER, chord(&live, r), small(), if on { TEXT_DARK } else { TEXT });
    }
    // The figure: which chord note each pulse plays.
    let pattern: Vec<usize> = match figure {
        0 => vec![0, 1, 2, 1],
        1 => vec![0, 2, 1, 2],
        2 => vec![0, 1, 2, 3, 2, 1],
        3 => (0..len.max(1)).collect(),
        _ => vec![0; 4],
    };
    let area = Rect::from_min_max(pos2(rect.left() + 12.0, rect.top() + 40.0), pos2(rect.right() - 12.0, rect.bottom() - 30.0));
    let n = pattern.len();
    let dx = area.width() / n as f32;
    let y_of = |i: usize| area.bottom() - 6.0 - i as f32 * (area.height() - 12.0) / 6.0;
    for (j, &i) in pattern.iter().enumerate() {
        let x = area.left() + (j as f32 + 0.5) * dx;
        let now = vis.is_some() && j == note.min(n.saturating_sub(1)) && note != usize::MAX;
        let col = if now { accent } else { TEXT_DIM.gamma_multiply(0.7) };
        if figure == 4 {
            for s in 0..3 {
                p.circle_filled(pos2(x, y_of(s * 2)), if now { 5.0 } else { 3.5 }, col);
            }
        } else {
            p.circle_filled(pos2(x, y_of(i)), if now { 5.5 } else { 3.5 }, col);
        }
    }
    // Three over two: the triplets above.
    if figure == 0 {
        let t = (3 * n / 2).max(1);
        let tx = area.width() / t as f32;
        for j in 0..t {
            p.circle_stroke(pos2(area.left() + (j as f32 + 0.5) * tx, area.top() + 4.0), 3.0, Stroke::new(1.2_f32, SELECT.gamma_multiply(0.8)));
        }
    }
    // How often it has come round.
    let bars = if figure == 3 { 2 } else { repeats.max(1) };
    let bw = (w - 16.0) / bars as f32;
    for i in 0..bars {
        let r = Rect::from_min_size(pos2(rect.left() + 8.0 + i as f32 * bw, rect.bottom() - 22.0), vec2(bw - 3.0, 5.0));
        rect_fill(&p, r, 2.0, if vis.is_some() && i <= rep { mix(PANEL3, accent, 0.7) } else { PANEL3 });
    }
    let what = format!("{}  ·  {}x a chord", mn::FIGURES[figure].to_lowercase(), if figure == 3 { "grows and shrinks".to_string() } else { repeats.to_string() });
    text(&p, pos2(rect.left() + 8.0, rect.bottom() - 4.0), Align2::LEFT_BOTTOM, what, small(), TEXT_DIM);
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    false
}

// ---------------------------------------------------------------- satie

/// The bar (three beats, four for a Gnossienne) with what each beat plays
/// and where it is now; the chord; the melody's last notes on a staff.
pub(crate) fn satie(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let w = 260.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let vis = live.on();
    let style = choice(params, mn::S_STYLE, mn::SATIE_STYLES.len());
    let beats = if style == 1 { 4 } else { 3 };
    let (t, root, recent) = match vis {
        Some(v) => (((v[0] >> 4) & 0xff) as f32, ((v[0] >> 20) & 0xff) as i32 - 64, v[1]),
        None => (-1.0, mn::SATIE_ROOTS[choice(params, mn::S_HARMONY, mn::SATIE_HARMONIES.len())][0], 0),
    };
    // The bar.
    let bar = Rect::from_min_size(pos2(rect.left() + 8.0, rect.top() + 8.0), vec2(w - 16.0, 30.0));
    let bw = bar.width() / beats as f32;
    for b in 0..beats {
        let r = Rect::from_min_size(pos2(bar.left() + b as f32 * bw, bar.top()), vec2(bw - 3.0, bar.height()));
        let now = t >= 0.0 && (t / 12.0).floor() as usize == b;
        rect_fill(&p, r, 3.0, if now { mix(PANEL3, accent, 0.5) } else { PANEL3 });
        let label = match (style, b) {
            (2, 0) => "chord",
            (2, 2) => "slide",
            (_, 0) | (1, 2) => "bass",
            (_, 1) | (1, 3) => "chord",
            _ => "·",
        };
        text(&p, r.center(), Align2::CENTER_CENTER, label, small(), if now { TEXT } else { TEXT_DIM });
    }
    if t >= 0.0 {
        let x = bar.left() + t / (12.0 * beats as f32) * bar.width();
        p.line_segment([pos2(x, bar.top() - 2.0), pos2(x, bar.bottom() + 2.0)], Stroke::new(1.5_f32, accent));
    }
    // The chord now.
    let name = if style == 2 { format!("{}9", chord(&live, root)) } else { format!("{}7", chord(&live, root)) };
    text(&p, pos2(rect.right() - 10.0, bar.bottom() + 14.0), Align2::RIGHT_CENTER, name, small(), TEXT);
    // The melody's last notes, newest on the right.
    let staff = Rect::from_min_max(pos2(rect.left() + 10.0, bar.bottom() + 26.0), pos2(rect.right() - 10.0, rect.bottom() - 20.0));
    for l in 0..5 {
        let y = staff.top() + l as f32 * staff.height() / 4.0;
        p.line_segment([pos2(staff.left(), y), pos2(staff.right(), y)], Stroke::new(1.0_f32, LINE_SOFT));
    }
    let notes: Vec<i32> = (0..8).rev().map(|i| ((recent >> (8 * i)) & 0xff) as i32).filter(|&d| d > 0).map(|d| d - 64).collect();
    let y_of = |d: i32| staff.bottom() - (d + 3) as f32 * staff.height() / 14.0;
    let dx = staff.width() / 9.0;
    let pts: Vec<egui::Pos2> = notes.iter().enumerate().map(|(i, &d)| pos2(staff.left() + (i as f32 + 1.0) * dx, y_of(d))).collect();
    if pts.len() > 1 {
        p.add(Shape::line(pts.clone(), Stroke::new(1.0_f32, accent.gamma_multiply(0.4))));
    }
    for (i, q) in pts.iter().enumerate() {
        let newest = i + 1 == pts.len();
        p.add(Shape::ellipse_filled(*q, vec2(4.5, 3.5), if newest { accent } else { accent.gamma_multiply(0.55) }));
    }
    text(&p, pos2(rect.left() + 8.0, rect.bottom() - 4.0), Align2::LEFT_BOTTOM, mn::SATIE_STYLES[style], small(), TEXT_DIM);
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    false
}

// ---------------------------------------------------------------- riley

/// In C: the series of figures from left to right, each player a dot on its
/// own row at the figure it plays (dim while it rests), the pulse a light.
/// Rainbow: each loop a ring, as big as it is long, its head going round.
pub(crate) fn riley(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let w = 260.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let vis = live.on();
    let style = choice(params, mn::R_STYLE, mn::RILEY_STYLES.len());
    let players = params[mn::R_PLAYERS].round().clamp(1.0, mn::MAX_PLAYERS as f32) as usize;
    let total = params[mn::R_FIGURES].round().clamp(8.0, 48.0) as usize;
    let pulse = params[mn::R_PULSE] >= 0.5;
    let k = vis.map(|v| v[2]).unwrap_or(0);
    if style == 1 {
        // The loops as rings, from small to large.
        let n = players;
        let cw = (w - 16.0) / n as f32;
        for i in 0..n {
            let len = mn::RAINBOW_LENGTHS[i];
            let c = pos2(rect.left() + 8.0 + (i as f32 + 0.5) * cw, rect.center().y - 6.0);
            let r = (cw * 0.5 - 4.0).min(10.0 + len as f32 * 2.2).max(8.0);
            p.circle_stroke(c, r, Stroke::new(1.5_f32, player_color(i).gamma_multiply(0.5)));
            for s in 0..len {
                let a = s as f32 / len as f32 * TAU - FRAC_PI_2;
                p.circle_filled(c + vec2(a.cos(), a.sin()) * r, 1.5, TEXT_DIM.gamma_multiply(0.6));
            }
            if let Some(v) = vis {
                let pos = ((v[1] >> (4 * i)) & 0xf) as f32;
                let a = pos / len as f32 * TAU - FRAC_PI_2;
                p.circle_filled(c + vec2(a.cos(), a.sin()) * r, 4.0, player_color(i));
            }
            text(&p, pos2(c.x, rect.bottom() - 22.0), Align2::CENTER_CENTER, format!("{len}"), small(), TEXT_DIM);
        }
    } else {
        // The series, a row per player.
        let area = Rect::from_min_max(pos2(rect.left() + 10.0, rect.top() + 12.0), pos2(rect.right() - 10.0, rect.bottom() - 30.0));
        let fx = area.width() / total as f32;
        let row = area.height() / players as f32;
        for f in 0..total {
            let x = area.left() + f as f32 * fx;
            p.line_segment([pos2(x, area.top()), pos2(x, area.bottom())], Stroke::new(1.0_f32, LINE_SOFT));
        }
        for i in 0..players {
            let y = area.top() + (i as f32 + 0.5) * row;
            p.line_segment([pos2(area.left(), y), pos2(area.right(), y)], Stroke::new(1.0_f32, LINE_SOFT));
            if let Some(v) = vis {
                let fig = ((v[1] >> (8 * i)) & 0xff) as usize;
                let resting = (v[0] >> (10 + i)) & 1 == 1;
                if fig < total {
                    let x = area.left() + (fig as f32 + 0.5) * fx;
                    let col = if resting { player_color(i).gamma_multiply(0.3) } else { player_color(i) };
                    p.circle_filled(pos2(x, y), (row * 0.3).clamp(3.0, 6.0), col);
                }
            }
        }
    }
    // The pulse.
    if pulse {
        let lit = vis.is_some() && k % 4 < 2;
        let c = pos2(rect.right() - 14.0, rect.bottom() - 12.0);
        p.circle_filled(c, 5.0, if lit { accent } else { PANEL3 });
        text(&p, c - vec2(10.0, 0.0), Align2::RIGHT_CENTER, "pulse", small(), TEXT_DIM);
    }
    let what = if style == 1 { format!("rainbow: {players} loops") } else { format!("{players} players, {total} figures") };
    text(&p, pos2(rect.left() + 8.0, rect.bottom() - 4.0), Align2::LEFT_BOTTOM, what, small(), TEXT_DIM);
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    false
}
