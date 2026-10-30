//! Home Rhythms' editor, as on a home keyboard: a display with the rhythm's
//! number and name, what plays now (Main A or B, a fill, the intro, the
//! ending) and the chord the accompaniment is on; the rhythm's pattern in
//! its lanes, the bass and the chords, the step it is on lit; and the big
//! buttons: Intro, Variation A and B, Fill In and Ending. Beside it the
//! rhythm (a menu by group), the part it plays and how it finds its chords,
//! and faders.

use super::nature_seq_view::{fader_at, frame, pick_choice, slot, split};
use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Ui};
use krabton_engine::sequencers::home as ho;

/// What the display calls each section.
const SECTIONS: [&str; 5] = ["MAIN", "FILL IN", "INTRO", "ENDING", "STOPPED"];

pub(crate) fn home(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, p) = frame(ui, 470.0, height);
    let (pic, side) = split(rect);
    let vis = live.on();
    let ri = vis.map(|v| (v[0] >> 24 & 0xff) as usize).unwrap_or(params[ho::RHYTHM].round().max(0.0) as usize).min(ho::RHYTHMS.len() - 1);
    let r = &ho::RHYTHMS[ri];
    let var = vis.map(|v| (v[0] >> 20 & 1) as usize).unwrap_or(params[ho::VARIATION].round().clamp(0.0, 1.0) as usize);
    let section = vis.map(|v| (v[0] >> 16 & 7) as usize).unwrap_or(0).min(4);
    let want = vis.map(|v| (v[0] >> 48 & 7) as u8).unwrap_or(0);
    let mut changed = false;

    // The display.
    let lcd = Rect::from_min_size(pic.min, vec2(pic.width(), 34.0));
    rect_fill(&p, lcd, 3.0, Color32::from_rgb(0x10, 0x14, 0x10));
    let ink = accent;
    text(&p, lcd.left_center() + vec2(8.0, 0.0), Align2::LEFT_CENTER, format!("{:02} {}", ri + 1, r.name.to_uppercase()), FontId::monospace(15.0), ink);
    let now = match section {
        0 => format!("{} {}", SECTIONS[0], ho::VARIATIONS[var]),
        s => SECTIONS[s].to_string(),
    };
    let next = if want & 1 != 0 {
        "  > INTRO"
    } else if want & 4 != 0 {
        "  > ENDING"
    } else if want & 2 != 0 {
        "  > FILL IN"
    } else {
        ""
    };
    text(&p, lcd.right_top() + vec2(-8.0, 4.0), Align2::RIGHT_TOP, format!("{now}{next}"), FontId::monospace(11.0), if vis.is_some() { ink } else { shade(ink, 0.5) });
    let part = (params[ho::PART].round().max(0.0) as usize).min(ho::PARTS.len() - 1);
    if part > 0 {
        // The chord it plays on.
        let chord = vis.map(|v| ((v[1] >> 16 & 0xf) as usize, (v[1] >> 20 & 0xf) as usize));
        let label = match chord {
            Some((root, q)) => format!("{}{}", krabton_engine::scale::NOTE_NAMES[root % 12], ho::Quality::ALL[q.min(7)].suffix()),
            None => "-".into(),
        };
        text(&p, lcd.right_bottom() + vec2(-8.0, -4.0), Align2::RIGHT_BOTTOM, format!("CHORD {label}"), FontId::monospace(11.0), ink);
    }

    // The pattern: the lanes it plays, the bass and the chords.
    let buttons_h = 34.0;
    let grid = Rect::from_min_max(pos2(pic.left(), lcd.bottom() + 6.0), pos2(pic.right(), pic.bottom() - buttons_h - 6.0));
    let steps = r.steps;
    let len = ho::bars(r) * steps;
    let lanes: Vec<usize> = (0..ho::LANES).filter(|&l| (0..len).any(|i| ho::drum_level(r, var, l, i) > 0)).collect();
    let rows = lanes.len() + 2;
    let name_w = 62.0;
    let row_h = (grid.height() / rows as f32).min(16.0);
    let cw = (grid.width() - name_w) / len as f32;
    let at = vis.map(|v| {
        let j = (v[0] & 0xff) as usize;
        let bar = (v[0] >> 32 & 0xffff) as usize;
        (bar % ho::bars(r)) * steps + j
    });
    let cell = |row: usize, i: usize| Rect::from_min_size(pos2(grid.left() + name_w + i as f32 * cw, grid.top() + row as f32 * row_h), vec2(cw - 1.0, row_h - 1.5));
    for i in 0..len {
        // The beats a little lighter, the step it is on lit.
        let col = Rect::from_min_max(cell(0, i).min, cell(rows - 1, i).max);
        if at == Some(i) {
            rect_fill(&p, col, 1.5, shade(accent, 0.35));
        } else if i % (steps / r.beats).max(1) == 0 {
            rect_fill(&p, col, 1.5, Color32::from_white_alpha(8));
        }
    }
    for (row, &l) in lanes.iter().enumerate() {
        text(&p, pos2(grid.left(), grid.top() + (row as f32 + 0.5) * row_h), Align2::LEFT_CENTER, ho::LANE_NAMES[l], small(), TEXT_DIM);
        for i in 0..len {
            let lv = ho::drum_level(r, var, l, i);
            let c = cell(row, i);
            let fill = match lv {
                0 => PANEL3,
                1 => shade(accent, 0.45),
                2 => shade(accent, 0.8),
                _ => accent,
            };
            rect_fill(&p, c, 1.5, if at == Some(i) && lv > 0 { Color32::WHITE } else { fill });
        }
    }
    for (k, (c, name)) in [('b', "Bass"), ('c', "Chords")].into_iter().enumerate() {
        let row = lanes.len() + k;
        text(&p, pos2(grid.left(), grid.top() + (row as f32 + 0.5) * row_h), Align2::LEFT_CENTER, name, small(), TEXT_DIM);
        let find = |rows: &'static [(char, &'static str)]| rows.iter().find(|(ch, _)| *ch == c).map(|(_, s)| s.as_bytes());
        let line = if var == 1 { find(r.b).or_else(|| find(r.a)) } else { find(r.a) };
        for i in 0..len {
            let ch = line.map(|s| s[i % s.len()]).unwrap_or(b'.');
            let cr = cell(row, i);
            if ch == b'.' || ch == b'-' {
                rect_fill(&p, cr, 1.5, PANEL3);
                continue;
            }
            rect_fill(&p, cr, 1.5, if at == Some(i) { Color32::WHITE } else { shade(SELECT, if part == 0 { 0.35 } else { 0.7 }) });
            if cw > 7.0 && c == 'b' {
                text(&p, cr.center(), Align2::CENTER_CENTER, ch as char, FontId::proportional((row_h * 0.7).min(10.0)), Color32::BLACK);
            }
        }
    }

    // The big buttons.
    let bar = Rect::from_min_max(pos2(pic.left(), pic.bottom() - buttons_h), pic.max);
    let bw = bar.width() / 5.0;
    let big = |ui: &mut Ui, k: usize, label: &str, on: bool, tip: &str| -> bool {
        let b = Rect::from_min_size(pos2(bar.left() + k as f32 * bw, bar.top()), vec2(bw, bar.height())).shrink(2.0);
        let resp = ui.interact(b, id.with(("big", k)), Sense::click());
        let fill = if on {
            accent
        } else if resp.hovered() {
            mix(PANEL3, accent, 0.3)
        } else {
            PANEL3
        };
        let painter = ui.painter();
        rect_fill(painter, b.translate(vec2(0.0, 2.0)), 4.0, Color32::from_black_alpha(100));
        rect_fill(painter, b, 4.0, fill);
        text(painter, b.center(), Align2::CENTER_CENTER, label, FontId::proportional(13.0), if on { text_on(fill) } else { TEXT });
        resp.on_hover_text(tip).clicked()
    };
    let press = |params: &mut [f32], i: usize| params[i] = (params[i].round() + 1.0).rem_euclid(100.0);
    if big(ui, 0, "INTRO", section == 2 || want & 1 != 0, "The intro: a count in and a fill, from the next bar (Start Intro: when the song starts)") {
        press(params, ho::INTRO);
        changed = true;
    }
    for (k, v) in [(1, 0.0), (2, 1.0)] {
        let label = if v == 0.0 { "VAR A" } else { "VAR B" };
        if big(ui, k, label, params[ho::VARIATION].round() == v, "Variation A, or B: busier") {
            params[ho::VARIATION] = v;
            changed = true;
        }
    }
    if big(ui, 3, "FILL IN", section == 1 || want & 2 != 0, "A fill to the end of the bar (in the bar's last beat: the next bar)") {
        press(params, ho::FILL_IN);
        changed = true;
    }
    if big(ui, 4, "ENDING", section == 3 || want & 4 != 0, "The ending, from the next bar; then it is quiet until Intro or Fill In") {
        press(params, ho::ENDING);
        changed = true;
    }
    if vis.is_some() {
        ui.ctx().request_repaint();
    }

    // The settings: the rhythm from a menu (right-click: the next one).
    let s = slot(side, 0);
    let resp = ui.interact(s, id.with("rhythm"), Sense::click());
    rect_fill(&p, s, 3.0, if resp.hovered() { mix(PANEL3, accent, 0.25) } else { PANEL3 });
    text(&p, pos2(s.left() + 5.0, s.center().y), Align2::LEFT_CENTER, "Rhythm", small(), TEXT_DIM);
    text(&p, pos2(s.right() - 5.0, s.center().y), Align2::RIGHT_CENTER, r.name, FontId::proportional(12.0), TEXT);
    if resp.secondary_clicked() {
        params[ho::RHYTHM] = ((ri + 1) % ho::RHYTHMS.len()) as f32;
        changed = true;
    }
    let resp = resp.on_hover_text("The rhythm: click for the list, right-click for the next");
    egui::Popup::menu(&resp).show(|ui| {
        egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
            for group in ho::GROUPS {
                ui.label(egui::RichText::new(*group).small().color(TEXT_DIM));
                for (i, rh) in ho::RHYTHMS.iter().enumerate().filter(|(_, rh)| rh.group == *group) {
                    if ui.selectable_label(i == ri, format!("  {:02} {}", i + 1, rh.name)).clicked() {
                        params[ho::RHYTHM] = i as f32;
                        changed = true;
                        ui.close();
                    }
                }
            }
        });
    });
    changed |= pick_choice(ui, slot(side, 1), id.with("part"), params, ho::PART, ho::PARTS, "Part", accent);
    changed |= pick_choice(ui, slot(side, 2), id.with("chords"), params, ho::CHORDS, ho::CHORD_MODES, "Chords", accent);
    changed |= pick_choice(ui, slot(side, 3), id.with("fills"), params, ho::FILL_EVERY, ho::FILL_CHOICES, "Fill Every", accent);
    let s = slot(side, 4);
    let on = params[ho::START_INTRO] >= 0.5;
    if button(ui, s, id.with("start_intro"), "Start Intro", on, accent).on_hover_text("Play the intro when the song starts from its beginning").clicked() {
        params[ho::START_INTRO] = if on { 0.0 } else { 1.0 };
        changed = true;
    }
    for (k, i) in [ho::VELOCITY, ho::SPLIT, ho::BASS_NOTE].into_iter().enumerate() {
        changed |= fader_at(ui, slot(side, 5 + k), id, params, ho::SPECS, i, accent);
    }
    changed
}
