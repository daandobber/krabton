//! Breakline's face, as the trackers drill'n'bass was written in: the four
//! manners as tiles (each drawn as the line it plays), the whole phrase as
//! a roll (rolls split, octave jumps lit, bars marked, where it plays now),
//! and the bar playing as a tracker column (step, note, roll), its row lit.
//! Dice throws another line.

use super::seq_view::Live;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::sequencers::breakline as bl;

const CYAN: Color32 = Color32::from_rgb(0x39, 0xe6, 0xff);
const AMBER: Color32 = Color32::from_rgb(0xff, 0xb0, 0x2e);
const PINK: Color32 = Color32::from_rgb(0xff, 0x4f, 0xa8);
const GRID: Color32 = Color32::from_rgb(0x12, 0x1a, 0x2a);
const BG: Color32 = Color32::from_rgb(0x05, 0x08, 0x10);
const DIM: Color32 = Color32::from_rgb(0x4a, 0x5a, 0x78);

const NAMES: [&str; 12] = ["C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-"];

fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

/// A step's pitch in the song's key (as the engine plays it, before the band moves it).
fn pitch_of(p: &[f32], n: &bl::Note, scale: &krabton_engine::Scale) -> i32 {
    let (home, _) = scale.locate(p[bl::CENTER].round() as i32);
    scale.pitch_at(home + n.degree) + 12 * n.octave
}

/// A manner drawn as the line it plays.
fn glyph(p: &egui::Painter, r: Rect, style: usize, c: Color32) {
    let pts: Vec<egui::Pos2> = match style {
        // A slow tune, wandering.
        0 => (0..=24).map(|i| i as f32 / 24.0).map(|t| pos2(r.left() + t * r.width(), r.center().y - (t * 9.0).sin() * 0.3 * r.height() - (t * 3.0).sin() * 0.12 * r.height())).collect(),
        // Runs: stairs up, then down.
        1 => (0..=12)
            .flat_map(|i| {
                let t0 = i as f32 / 13.0;
                let t1 = (i + 1) as f32 / 13.0;
                let lvl = if i < 7 { i as f32 / 6.0 } else { (12 - i) as f32 / 6.0 };
                let y = r.bottom() - lvl * r.height() * 0.8 - r.height() * 0.1;
                [pos2(r.left() + t0 * r.width(), y), pos2(r.left() + t1 * r.width(), y)]
            })
            .collect(),
        // Arps: a zigzag over a chord.
        2 => (0..=12).map(|i| pos2(r.left() + i as f32 / 12.0 * r.width(), r.bottom() - r.height() * (0.15 + 0.7 * [0.0, 0.5, 1.0, 0.5][i % 4]))).collect(),
        // Acid: the root with spikes.
        _ => (0..=16).map(|i| pos2(r.left() + i as f32 / 16.0 * r.width(), r.center().y + r.height() * 0.3 - if [3, 7, 10, 14].contains(&i) { r.height() * 0.7 } else { 0.0 })).collect(),
    };
    p.add(Shape::line(pts, Stroke::new(1.6_f32, c)));
}

pub(crate) fn breakline(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, height: f32) -> bool {
    let mut changed = false;
    let h = height.max(120.0);
    let tile_w = 92.0;
    let roll_w = 420.0;
    let col_w = 132.0;
    let (rect, _) = ui.allocate_exact_size(vec2(tile_w + roll_w + col_w + 16.0, h), Sense::hover());
    let painter = ui.painter_at(rect);
    rect_fill(&painter, rect, 4.0, BG);
    let style = (params[bl::STYLE].round().max(0.0) as usize).min(bl::STYLES.len() - 1);

    // The manners.
    let th = ((h - 8.0 - 26.0) / 4.0).clamp(24.0, 52.0);
    for (k, name) in bl::STYLES.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + 4.0, rect.top() + 4.0 + k as f32 * (th + 2.0)), vec2(tile_w - 6.0, th));
        let resp = ui.interact(r, id.with(("style", k)), Sense::click());
        let on = k == style;
        rect_fill(&painter, r, 3.0, if on { Color32::from_rgb(0x0c, 0x2a, 0x36) } else { GRID });
        if on || resp.hovered() {
            rect_line(&painter, r, 3.0, 1.0, if on { CYAN } else { DIM });
        }
        let g = Rect::from_min_max(pos2(r.left() + 6.0, r.top() + 4.0), pos2(r.right() - 6.0, r.bottom() - 15.0));
        glyph(&painter, g, k, if on { CYAN } else { DIM });
        text(&painter, pos2(r.left() + 6.0, r.bottom() - 7.0), Align2::LEFT_CENTER, name.to_uppercase(), mono(10.0), if on { AMBER } else { DIM });
        let tips = [
            "A slow, sad tune wandering by steps and leaps over the breaks, held long, home at the end of its phrase",
            "Scales up and down at speed: a run of three to eight notes one way, then another",
            "A minor chord broken up fast, up, down or both ways, the chord changing every bar",
            "The root mostly, an octave up or down, hard accents",
        ];
        if resp.on_hover_text(tips[k]).clicked() {
            params[bl::STYLE] = k as f32;
            changed = true;
        }
    }
    // Dice under them.
    let dr = Rect::from_min_size(pos2(rect.left() + 4.0, rect.bottom() - 26.0), vec2(tile_w - 6.0, 22.0));
    let dice = ui.interact(dr, id.with("dice"), Sense::click());
    rect_fill(&painter, dr, 3.0, if dice.hovered() { Color32::from_rgb(0x2a, 0x1a, 0x06) } else { GRID });
    rect_line(&painter, dr, 3.0, 1.0, AMBER);
    let seed = params[bl::SEED].round() as i32;
    text(&painter, dr.center(), Align2::CENTER_CENTER, format!("DICE {seed:03}"), mono(11.0), AMBER);
    let dice = dice.on_hover_text("Another line from the same settings (right-click: the one before)");
    if dice.clicked() {
        params[bl::SEED] = ((seed + 1) % 256) as f32;
        changed = true;
    } else if dice.secondary_clicked() {
        params[bl::SEED] = ((seed + 255) % 256) as f32;
        changed = true;
    }

    // The phrase, as a roll.
    let per_bar = bl::per_bar(params);
    let bars = [1i64, 2, 4, 8][(params[bl::PHRASE].round().max(0.0) as usize).min(3)];
    let steps = per_bar * bars;
    let now = live.on().map(|v| v[0] as i64);
    let phrase_start = now.map_or(0, |k| k - k.rem_euclid(steps));
    let notes: Vec<(i64, Option<bl::Note>)> = (0..steps).map(|i| (i, bl::note_at(params, phrase_start + i))).collect();
    let pitches: Vec<i32> = notes.iter().filter_map(|(_, n)| n.as_ref().map(|n| pitch_of(params, n, &live.scale))).collect();
    let (lo, hi) = (pitches.iter().copied().min().unwrap_or(60) - 2, pitches.iter().copied().max().unwrap_or(72) + 4);
    let roll = Rect::from_min_size(pos2(rect.left() + tile_w + 4.0, rect.top() + 18.0), vec2(roll_w, h - 24.0));
    rect_fill(&painter, roll, 2.0, GRID);
    let sw = roll.width() / steps.max(1) as f32;
    let span = (hi - lo).max(1) as f32;
    let y_of = |pitch: i32| roll.bottom() - (pitch - lo) as f32 / span * roll.height();
    // The octaves' C, faintly; the bars.
    for c in (lo..=hi).filter(|n| n.rem_euclid(12) == 0) {
        let y = y_of(c);
        painter.line_segment([pos2(roll.left(), y), pos2(roll.right(), y)], Stroke::new(1.0_f32, Color32::from_rgb(0x1c, 0x28, 0x40)));
        text(&painter, pos2(roll.left() + 2.0, y - 6.0), Align2::LEFT_CENTER, format!("C{}", c / 12 - 1), mono(9.0), DIM);
    }
    for b in 0..=bars {
        let x = roll.left() + (b * per_bar) as f32 * sw;
        painter.line_segment([pos2(x, roll.top()), pos2(x, roll.bottom())], Stroke::new(1.0_f32, if b % 4 == 0 { DIM } else { Color32::from_rgb(0x24, 0x30, 0x48) }));
    }
    let head = now.map(|k| k.rem_euclid(steps));
    for (i, n) in &notes {
        let Some(n) = n else { continue };
        let pitch = pitch_of(params, n, &live.scale);
        let x = roll.left() + *i as f32 * sw;
        let y = y_of(pitch);
        let playing = head == Some(*i);
        let col = if playing {
            Color32::WHITE
        } else if n.octave != 0 {
            PINK
        } else if n.hits > 1 {
            AMBER
        } else {
            CYAN
        };
        let a = (0.35 + 0.65 * n.vel.clamp(0.0, 1.0)) * if playing { 1.0 } else { 0.9 };
        let hits = n.hits.max(1) as usize;
        let each = (sw - 1.0).max(1.0) / hits as f32;
        for hidx in 0..hits {
            let hy = y - (n.climb * hidx as i32) as f32 * roll.height() / span * 1.5;
            let r = Rect::from_min_size(pos2(x + hidx as f32 * each, hy - 2.5), vec2((each - 1.0).max(1.0), 5.0));
            painter.rect_filled(r, 1.0, col.gamma_multiply(a));
        }
    }
    if let Some(hd) = head {
        let x = roll.left() + (hd as f32 + 0.5) * sw;
        painter.line_segment([pos2(x, roll.top()), pos2(x, roll.bottom())], Stroke::new(1.0_f32, Color32::from_white_alpha(90)));
    }
    let bar_now = head.map_or(0, |hd| hd / per_bar);
    text(
        &painter,
        pos2(roll.left(), rect.top() + 9.0),
        Align2::LEFT_CENTER,
        format!("{}  ·  {} BAR{}  ·  BAR {}/{}", bl::STYLES[style].to_uppercase(), bars, if bars == 1 { "" } else { "S" }, bar_now + 1, bars),
        mono(10.0),
        AMBER,
    );

    // The bar playing, as a tracker column.
    let col = Rect::from_min_size(pos2(roll.right() + 8.0, rect.top() + 4.0), vec2(col_w - 4.0, h - 8.0));
    rect_fill(&painter, col, 2.0, Color32::from_rgb(0x03, 0x05, 0x0a));
    let rows = per_bar.min(32) as usize;
    let rh = ((col.height() - 4.0) / rows as f32).min(14.0);
    let fs = (rh - 2.0).clamp(6.0, 11.0);
    for row in 0..rows {
        let i = bar_now * per_bar + row as i64;
        let y = col.top() + 2.0 + row as f32 * rh;
        let lit = head == Some(i);
        if lit {
            painter.rect_filled(Rect::from_min_size(pos2(col.left(), y), vec2(col.width(), rh)), 0.0, Color32::from_rgb(0x10, 0x3a, 0x4a));
        } else if row % 4 == 0 {
            painter.rect_filled(Rect::from_min_size(pos2(col.left(), y), vec2(col.width(), rh)), 0.0, Color32::from_rgb(0x08, 0x0e, 0x18));
        }
        let line = match notes.get(i as usize).and_then(|(_, n)| n.as_ref()) {
            Some(n) => {
                let pitch = pitch_of(params, n, &live.scale);
                let roll = if n.hits > 1 { format!("x{}", n.hits) } else { "..".to_string() };
                format!("{row:02} {}{} {roll} {:02X}", NAMES[pitch.rem_euclid(12) as usize], pitch / 12 - 1, (n.vel * 127.0) as u8)
            }
            None => format!("{row:02} --- .. .."),
        };
        text(&painter, pos2(col.left() + 4.0, y + rh * 0.5), Align2::LEFT_CENTER, line, mono(fs), if lit { Color32::WHITE } else if row % 4 == 0 { AMBER } else { CYAN.gamma_multiply(0.85) });
    }
    if live.on().is_some() {
        ui.ctx().request_repaint();
    }
    changed
}
