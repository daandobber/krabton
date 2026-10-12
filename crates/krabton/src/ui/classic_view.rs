//! Editors for the deterministic sequencers: Step, Acid, Drum Grid and
//! Progression.

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Stroke, Ui};
use krabton_engine::classic_seq::{self as cs, ACID_BYTES, DRUM_LANES, PROG_BYTES, PROG_SLOTS, STEP_BYTES};
use krabton_engine::note_name;

fn small_btn(ui: &mut Ui, r: Rect, id: Id, label: &str) -> bool {
    button(ui, r, id, label, false, accent_now()).clicked()
}

/// Press-and-drag painting (the value is chosen on press).
fn paint<T: Clone + Send + Sync + 'static>(ui: &Ui, resp: &egui::Response, id: Id, on_press: impl FnOnce() -> T) -> Option<T> {
    let pressed = resp.hovered() && ui.input(|i| i.pointer.primary_pressed());
    if pressed {
        let v = on_press();
        ui.ctx().memory_mut(|m| m.data.insert_temp(id, v));
    }
    if resp.is_pointer_button_down_on() || pressed {
        ui.ctx().memory(|m| m.data.get_temp::<T>(id))
    } else {
        None
    }
}

fn rotate(data: &mut [u8], stride: usize, steps: usize, right: bool) {
    let n = steps * stride;
    if data.len() < n || steps < 2 {
        return;
    }
    if right {
        data[..n].rotate_right(stride);
    } else {
        data[..n].rotate_left(stride);
    }
}

fn seed() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() ^ d.as_secs() as u32).unwrap_or(7)
}

// ---------------------------------------------------------------- Step

const STEP_LANES: [(&str, usize, u8, u8); 4] = [("Note", 1, 0, cs::STEP_MAX_DEGREE), ("Vel", 2, 1, 127), ("Len", 3, 1, 8), ("Ratch", 4, 1, 4)];

pub(crate) fn step(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, lane: &mut usize, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    if data.len() < 16 * STEP_BYTES {
        *data = cs::step_default();
        changed = true;
    }
    *lane = (*lane).min(STEP_LANES.len() - 1);
    let cw = 15.0;
    let w = 16.0 * cw;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    let tab_w = w / STEP_LANES.len() as f32;
    for (l, (name, ..)) in STEP_LANES.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + l as f32 * tab_w, rect.top()), vec2(tab_w - 2.0, 16.0));
        if button(ui, r, id.with(("lane", l)), name, *lane == l, accent).clicked() {
            *lane = l;
        }
    }
    let (_, field, min, max) = STEP_LANES[*lane];
    let steps = params[1].round().clamp(1.0, 16.0) as usize;
    let pos = live.on().map(|v| (v[0] & 0xff) as usize);
    let gate_h = 14.0;
    let ed = Rect::from_min_max(pos2(rect.left(), rect.top() + 20.0), pos2(rect.right(), rect.bottom() - 22.0 - gate_h - 4.0));
    let gates = Rect::from_min_size(pos2(rect.left(), ed.bottom() + 3.0), vec2(w, gate_h));
    rect_fill(&p, ed, 3.0, KNOB);
    for i in 0..16 {
        let s = cs::step_at(data, i);
        let c = Rect::from_min_size(pos2(ed.left() + i as f32 * cw + 1.0, ed.top() + 2.0), vec2(cw - 2.0, ed.height() - 4.0));
        if pos == Some(i) {
            rect_fill(&p, c, 2.0, PANEL3);
        } else if i % 4 == 0 {
            rect_fill(&p, c, 2.0, shade(PANEL2, 0.9));
        }
        let v = data[i * STEP_BYTES + field].clamp(min, max);
        let t = (v - min) as f32 / (max - min).max(1) as f32;
        let bh = 3.0 + t * (c.height() - 3.0);
        let active = i < steps && s.on;
        let col = if !active { shade(accent, 0.25) } else if pos == Some(i) { mix(accent, Color32::WHITE, 0.45) } else { accent };
        let bar = Rect::from_min_max(pos2(c.left() + 1.0, c.bottom() - bh), pos2(c.right() - 1.0, c.bottom()));
        rect_fill(&p, bar, 1.5, shade(col, 0.75));
        p.line_segment([bar.left_top(), bar.right_top()], Stroke::new(2.0_f32, col));
        // Gate row.
        let g = Rect::from_center_size(pos2(c.center().x, gates.center().y), vec2(cw - 4.0, gate_h - 3.0));
        if s.on {
            rect_fill(&p, g, 2.0, if i < steps { accent } else { shade(accent, 0.3) });
        } else {
            rect_line(&p, g, 2.0, 1.0, shade(accent, 0.5));
        }
    }
    let resp = ui.interact(ed, id.with("values"), Sense::click_and_drag());
    if let (Some(pt), true) = (resp.interact_pointer_pos(), resp.is_pointer_button_down_on()) {
        let i = ((pt.x - ed.left()) / cw).floor();
        if (0.0..16.0).contains(&i) {
            let t = ((ed.bottom() - 2.0 - pt.y) / (ed.height() - 4.0)).clamp(0.0, 1.0);
            let v = min + (t * (max - min) as f32).round() as u8;
            let o = i as usize * STEP_BYTES;
            if data[o + field] != v || data[o] == 0 {
                data[o + field] = v;
                if field == 1 {
                    data[o] = 1; // drawing a note turns the step on
                }
                changed = true;
            }
        }
    }
    let hover = resp.hover_pos().map(|pt| ((pt.x - ed.left()) / cw).floor() as usize).filter(|&i| i < 16);
    let tip = match hover {
        Some(i) => {
            let s = cs::step_at(data, i);
            let (base, _) = live.scale.locate(params[4].round() as i32);
            let pitch = live.scale.pitch_at(base + s.degree as i32).clamp(0, 127) as u8;
            format!("Step {}: {} · vel {} · len {}/4 · x{}", i + 1, note_name(pitch), s.velocity, s.length, s.ratchet)
        }
        None => "Draw values; drawing a note turns the step on".into(),
    };
    resp.on_hover_text(tip);
    let gresp = ui.interact(gates, id.with("gates"), Sense::click_and_drag());
    if let Some(pt) = gresp.interact_pointer_pos().or_else(|| gresp.hover_pos()) {
        let i = ((pt.x - gates.left()) / cw).floor();
        if (0.0..16.0).contains(&i) {
            let o = i as usize * STEP_BYTES;
            if let Some(v) = paint(ui, &gresp, id.with("gpaint"), || (data[o] == 0) as u8) {
                if data[o] != v {
                    data[o] = v;
                    changed = true;
                }
            }
        }
    }
    let foot = Rect::from_min_size(pos2(rect.left(), rect.bottom() - 18.0), vec2(w, 18.0));
    if small_btn(ui, Rect::from_min_size(foot.min, vec2(26.0, 18.0)), id.with("rl"), "<") {
        rotate(data, STEP_BYTES, steps, false);
        changed = true;
    }
    if small_btn(ui, Rect::from_min_size(pos2(foot.left() + 30.0, foot.top()), vec2(26.0, 18.0)), id.with("rr"), ">") {
        rotate(data, STEP_BYTES, steps, true);
        changed = true;
    }
    if small_btn(ui, Rect::from_min_size(pos2(foot.right() - 64.0, foot.top()), vec2(64.0, 18.0)), id.with("rand"), "New Notes") {
        let mut r = krabton_engine::dsp::Rng::new(seed());
        let shapes = [0u8, 2, 4, 7, 4, 2, 9, 7, 11, 4];
        for i in 0..16 {
            data[i * STEP_BYTES + 1] = shapes[(r.next_u32() % shapes.len() as u32) as usize];
        }
        changed = true;
    }
    changed
}

// ---------------------------------------------------------------- Acid

pub(crate) fn acid(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    if data.len() < 16 * ACID_BYTES {
        *data = cs::acid_default();
        changed = true;
    }
    let label_w = 30.0;
    let cw = 15.0;
    let w = label_w + 16.0 * cw;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    let steps = params[1].round().clamp(1.0, 16.0) as usize;
    let base = params[3].round() as i32;
    let pos = live.on().map(|v| (v[0] & 0xff) as usize);
    let row_h = 11.0;
    let toggles = 4.0 * row_h;
    let roll_h = (height - toggles - 26.0).max(13.0 * 5.0);
    let rh = roll_h / 13.0;
    let roll = Rect::from_min_size(pos2(rect.left() + label_w, rect.top()), vec2(16.0 * cw, roll_h));
    rect_fill(&p, roll, 3.0, KNOB);
    // Keys: in-scale rows lighter, note names on the left for C and the base.
    for semi in 0..=12 {
        let y = roll.bottom() - (semi as f32 + 1.0) * rh;
        let pitch = (base + semi).clamp(0, 127) as u8;
        let row = Rect::from_min_size(pos2(roll.left(), y), vec2(roll.width(), rh));
        if live.scale.contains(pitch) {
            rect_fill(&p, row.shrink2(vec2(0.0, 0.5)), 0.0, shade(PANEL2, 1.05));
        }
        if semi == 0 || semi == 12 || pitch % 12 == 0 {
            text(&p, pos2(rect.left() + 2.0, row.center().y), Align2::LEFT_CENTER, note_name(pitch), small(), TEXT_DIM);
        }
    }
    let mut prev_on: Option<(usize, u8)> = None;
    for i in 0..16 {
        let s = cs::acid_at(data, i);
        let x = roll.left() + i as f32 * cw;
        if pos == Some(i) {
            rect_fill(&p, Rect::from_min_size(pos2(x, roll.top()), vec2(cw, roll.height())), 0.0, Color32::from_white_alpha(18));
        }
        let active = i < steps;
        let col = if !active { shade(accent, 0.3) } else if s.accent { mix(accent, Color32::WHITE, 0.35) } else { accent };
        match s.gate {
            1 => {
                let y = roll.bottom() - (s.semitone as f32 + 1.0) * rh;
                let r = Rect::from_min_size(pos2(x + 1.0, y + 0.5), vec2(cw - 2.0, rh - 1.0));
                rect_fill(&p, r, 2.0, col);
                if s.octave != 1 {
                    text(&p, r.center(), Align2::CENTER_CENTER, if s.octave == 2 { "+" } else { "-" }, small(), TEXT_DARK);
                }
                prev_on = Some((i, s.semitone));
            }
            2 => {
                if let Some((_, semi)) = prev_on {
                    let y = roll.bottom() - (semi as f32 + 0.5) * rh;
                    p.line_segment([pos2(x - 1.0, y), pos2(x + cw - 1.0, y)], Stroke::new(rh * 0.6, shade(col, 0.7)));
                }
            }
            _ => prev_on = None,
        }
        if s.slide && s.gate != 0 {
            // A little ramp toward the next step.
            let y = roll.bottom() - (s.semitone as f32 + 0.5) * rh;
            p.line_segment([pos2(x + cw - 3.0, y), pos2(x + cw + 2.0, y - 2.0)], Stroke::new(1.5_f32, Color32::WHITE));
        }
    }
    let rresp = ui.interact(roll, id.with("roll"), Sense::click());
    if let Some(pt) = rresp.interact_pointer_pos().filter(|_| rresp.clicked()) {
        let i = ((pt.x - roll.left()) / cw).floor() as usize;
        let semi = ((roll.bottom() - pt.y) / rh).floor().clamp(0.0, 12.0) as u8;
        if i < 16 {
            let o = i * ACID_BYTES;
            if data[o] == 1 && data[o + 1] == semi {
                data[o] = 0;
            } else {
                data[o] = 1;
                data[o + 1] = semi;
            }
            changed = true;
        }
    }
    rresp.on_hover_text("Click to place a note (click it again for a rest)");
    // Toggle rows: octave, accent, slide, gate type.
    let rows: [(&str, &str); 4] = [("Oct", "Octave: down / middle / up"), ("Acc", "Accent"), ("Sld", "Slide into the next step"), ("Gate", "Note, rest or tie")];
    for (ri, (label, tip)) in rows.iter().enumerate() {
        let y = roll.bottom() + 2.0 + ri as f32 * row_h;
        text(&p, pos2(rect.left() + 2.0, y + row_h * 0.5), Align2::LEFT_CENTER, *label, small(), TEXT_DIM);
        for i in 0..16 {
            let s = cs::acid_at(data, i);
            let c = Rect::from_min_size(pos2(roll.left() + i as f32 * cw + 1.0, y + 1.0), vec2(cw - 2.0, row_h - 2.0));
            let (on, glyph) = match ri {
                0 => (s.octave != 1, ["v", "", "^"][s.octave as usize]),
                1 => (s.accent, ""),
                2 => (s.slide, ""),
                _ => (s.gate != 0, ["", "", "~"][s.gate as usize]),
            };
            if on {
                rect_fill(&p, c, 2.0, if i < steps { shade(accent, 0.85) } else { shade(accent, 0.3) });
            } else {
                rect_line(&p, c, 2.0, 1.0, LINE_SOFT);
            }
            if !glyph.is_empty() {
                text(&p, c.center(), Align2::CENTER_CENTER, glyph, small(), TEXT_DARK);
            }
            let resp = ui.interact(c, id.with(("row", ri, i)), Sense::click());
            if resp.clicked() {
                let o = i * ACID_BYTES;
                match ri {
                    0 => data[o + 2] = (data[o + 2] + 1) % 3,
                    1 => data[o + 3] ^= 1,
                    2 => data[o + 4] ^= 1,
                    _ => data[o] = (data[o] + 1) % 3,
                }
                changed = true;
            }
            if i == 0 {
                resp.on_hover_text(*tip);
            }
        }
    }
    let foot = Rect::from_min_size(pos2(rect.left(), rect.bottom() - 18.0), vec2(w, 18.0));
    if small_btn(ui, Rect::from_min_size(foot.min, vec2(70.0, 18.0)), id.with("gen"), "Generate") {
        *data = cs::acid_generate(seed());
        changed = true;
    }
    if small_btn(ui, Rect::from_min_size(pos2(foot.left() + 74.0, foot.top()), vec2(26.0, 18.0)), id.with("rl"), "<") {
        rotate(data, ACID_BYTES, steps, false);
        changed = true;
    }
    if small_btn(ui, Rect::from_min_size(pos2(foot.left() + 104.0, foot.top()), vec2(26.0, 18.0)), id.with("rr"), ">") {
        rotate(data, ACID_BYTES, steps, true);
        changed = true;
    }
    if small_btn(ui, Rect::from_min_size(pos2(foot.right() - 48.0, foot.top()), vec2(48.0, 18.0)), id.with("clr"), "Clear") {
        for i in 0..16 {
            data[i * ACID_BYTES..(i + 1) * ACID_BYTES].copy_from_slice(&[0, 0, 1, 0, 0]);
        }
        changed = true;
    }
    changed
}

// ---------------------------------------------------------------- Drum Grid

pub(crate) fn drum_grid(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    if data.len() < cs::DRUM_BYTES {
        *data = cs::drum_default();
        changed = true;
    }
    let label_w = 88.0;
    let cw = 14.0;
    let rh = ((height - 26.0) / DRUM_LANES as f32).clamp(10.0, 18.0);
    let w = label_w + 16.0 * cw;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    let steps = params[1].round().clamp(1.0, 16.0) as usize;
    let pos = live.on().map(|v| (v[0] & 0xff) as usize);
    let grid = Rect::from_min_size(pos2(rect.left() + label_w, rect.top()), vec2(16.0 * cw, rh * DRUM_LANES as f32));
    rect_fill(&p, grid, 3.0, KNOB);
    for lane in 0..DRUM_LANES {
        let y = grid.top() + lane as f32 * rh;
        let note = cs::drum_note(data, lane);
        let lr = Rect::from_min_size(pos2(rect.left(), y), vec2(label_w - 4.0, rh));
        text(&p, pos2(lr.left() + 2.0, lr.center().y), Align2::LEFT_CENTER, cs::gm_drum_name(note), small(), TEXT);
        text(&p, pos2(lr.right(), lr.center().y), Align2::RIGHT_CENTER, note_name(note), small(), TEXT_DIM);
        let lresp = ui.interact(lr, id.with(("lane", lane)), Sense::hover());
        let scroll = if lresp.hovered() { ui.input(|i| i.smooth_scroll_delta.y) } else { 0.0 };
        if scroll.abs() > 0.5 {
            let acc_id = id.with(("scroll", lane));
            let acc = ui.ctx().memory(|m| m.data.get_temp::<f32>(acc_id).unwrap_or(0.0)) + scroll;
            let n = (acc / 20.0).trunc();
            ui.ctx().memory_mut(|m| m.data.insert_temp(acc_id, acc - n * 20.0));
            if n != 0.0 {
                let o = DRUM_LANES * 16 + lane;
                data[o] = (data[o] as f32 + n).clamp(0.0, 127.0) as u8;
                changed = true;
            }
        }
        lresp.on_hover_text("Scroll to change this lane's note");
        for i in 0..16 {
            let c = Rect::from_min_size(pos2(grid.left() + i as f32 * cw + 1.0, y + 1.0), vec2(cw - 2.0, rh - 2.0));
            let v = cs::drum_cell(data, lane, i);
            let active = i < steps;
            let fill = match v {
                2 => mix(accent, Color32::WHITE, 0.4),
                1 => accent,
                _ if pos == Some(i) => PANEL3,
                _ if i % 4 == 0 => shade(PANEL3, 0.8),
                _ => PANEL2,
            };
            rect_fill(&p, c, 2.0, if active { fill } else { shade(fill, 0.35) });
            if pos == Some(i) && v > 0 {
                rect_line(&p, c, 2.0, 1.0, Color32::WHITE);
            }
        }
    }
    let resp = ui.interact(grid, id.with("cells"), Sense::click_and_drag());
    if let Some(pt) = resp.interact_pointer_pos().or_else(|| resp.hover_pos()) {
        let i = ((pt.x - grid.left()) / cw).floor();
        let lane = ((pt.y - grid.top()) / rh).floor();
        if (0.0..16.0).contains(&i) && (0.0..DRUM_LANES as f32).contains(&lane) {
            let o = lane as usize * 16 + i as usize;
            // Press cycles off > hit > accent; dragging paints that value.
            if let Some(v) = paint(ui, &resp, id.with("paint"), || (data[o] + 1) % 3) {
                if data[o] != v {
                    data[o] = v;
                    changed = true;
                }
            }
        }
    }
    resp.on_hover_text("Click: off > hit > accent. Drag to paint.");
    let foot = Rect::from_min_size(pos2(rect.left(), rect.bottom() - 18.0), vec2(w, 18.0));
    let pat = button(ui, Rect::from_min_size(foot.min, vec2(80.0, 18.0)), id.with("pat"), "Patterns", false, accent);
    let mut picked = None;
    egui::Popup::menu(&pat).show(|ui| {
        for name in cs::DRUM_PATTERNS {
            if ui.button(*name).clicked() {
                picked = Some(*name);
                ui.close();
            }
        }
    });
    if let Some(name) = picked {
        cs::drum_pattern(name, data);
        changed = true;
    }
    if small_btn(ui, Rect::from_min_size(pos2(foot.left() + 84.0, foot.top()), vec2(26.0, 18.0)), id.with("rl"), "<") {
        for lane in 0..DRUM_LANES {
            data[lane * 16..lane * 16 + steps].rotate_left(1);
        }
        changed = true;
    }
    if small_btn(ui, Rect::from_min_size(pos2(foot.left() + 114.0, foot.top()), vec2(26.0, 18.0)), id.with("rr"), ">") {
        for lane in 0..DRUM_LANES {
            data[lane * 16..lane * 16 + steps].rotate_right(1);
        }
        changed = true;
    }
    changed
}

// ---------------------------------------------------------------- Progression

const PROGRESSIONS: &[(&str, &[(u8, u8, u8)])] = &[
    ("I - V - vi - IV", &[(1, 0, 0), (5, 0, 1), (6, 0, 1), (4, 0, 1)]),
    ("vi - IV - I - V", &[(6, 0, 0), (4, 0, 1), (1, 0, 1), (5, 0, 1)]),
    ("ii - V - I", &[(2, 1, 0), (5, 1, 1), (1, 1, 0), (1, 1, 0)]),
    ("I - vi - IV - V", &[(1, 0, 0), (6, 0, 1), (4, 0, 1), (5, 0, 1)]),
    ("i - VI - III - VII", &[(1, 0, 0), (6, 0, 1), (3, 0, 1), (7, 0, 1)]),
    ("Andalusian", &[(1, 0, 0), (7, 0, 1), (6, 0, 1), (5, 0, 1)]),
    ("Blues I - IV - I - V", &[(1, 1, 0), (4, 1, 1), (1, 1, 0), (5, 1, 1)]),
    ("Dreamy Sus", &[(1, 5, 0), (4, 2, 1), (6, 3, 1), (5, 5, 1)]),
];

pub(crate) fn progression(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    if data.len() < PROG_SLOTS * PROG_BYTES {
        *data = cs::progression_default();
        changed = true;
    }
    let (cw, ch) = (66.0, ((height - 30.0) / 2.0).clamp(40.0, 70.0));
    let w = 4.0 * (cw + 4.0);
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    let chords = params[1].round().clamp(1.0, PROG_SLOTS as f32) as usize;
    let current = live.on().map(|v| (v[0] & 0xff) as usize);
    for slot in 0..PROG_SLOTS {
        let card = Rect::from_min_size(pos2(rect.left() + (slot % 4) as f32 * (cw + 4.0), rect.top() + (slot / 4) as f32 * (ch + 4.0)), vec2(cw, ch));
        let (degree, quality, inversion) = cs::prog_slot(data, slot);
        let used = slot < chords;
        let playing = current == Some(slot);
        rect_fill(&p, card, 4.0, if playing { shade(accent, 0.55) } else if used { PANEL3 } else { PANEL2 });
        if playing {
            rect_line(&p, card, 4.0, 1.5, accent);
        }
        let name = cs::chord_name(&live.scale, degree, quality);
        let top = Rect::from_min_size(card.min, vec2(cw, ch - 18.0));
        text(&p, top.center(), Align2::CENTER_CENTER, &name, egui::FontId::proportional(17.0), if used { TEXT } else { TEXT_DIM });
        text(&p, pos2(card.left() + 4.0, card.top() + 7.0), Align2::LEFT_CENTER, (slot + 1).to_string(), small(), TEXT_DIM);
        let tresp = ui.interact(top, id.with(("deg", slot)), Sense::click());
        let o = slot * PROG_BYTES;
        if tresp.clicked() {
            data[o] = (data[o] + 1) % 8;
            changed = true;
        }
        if tresp.secondary_clicked() {
            data[o] = (data[o] + 7) % 8;
            changed = true;
        }
        let notes = cs::chord_pitches(&live.scale, params[3].round() as i32, degree, quality, inversion);
        let names: Vec<String> = notes.iter().map(|n| note_name((*n).clamp(0, 127) as u8)).collect();
        tresp.on_hover_text(format!("{} · click: next chord, right-click: previous", if names.is_empty() { "rest".into() } else { names.join(" ") }));
        let q = Rect::from_min_size(pos2(card.left() + 2.0, card.bottom() - 17.0), vec2(cw * 0.55, 15.0));
        let inv = Rect::from_min_max(pos2(q.right() + 2.0, q.top()), pos2(card.right() - 2.0, q.bottom()));
        if button(ui, q, id.with(("q", slot)), cs::QUALITIES[quality as usize], false, accent).on_hover_text("Chord type").clicked() {
            data[o + 1] = (quality + 1) % cs::QUALITIES.len() as u8;
            changed = true;
        }
        let inv_label = ["root", "inv 1", "inv 2", "inv 3"][inversion as usize];
        if button(ui, inv, id.with(("inv", slot)), inv_label, false, accent).on_hover_text("Inversion").clicked() {
            data[o + 2] = (inversion + 1) % 4;
            changed = true;
        }
    }
    let foot = Rect::from_min_size(pos2(rect.left(), rect.bottom() - 18.0), vec2(w, 18.0));
    let menu = button(ui, Rect::from_min_size(foot.min, vec2(96.0, 18.0)), id.with("presets"), "Progressions", false, accent);
    let mut picked = None;
    egui::Popup::menu(&menu).show(|ui| {
        for (name, slots) in PROGRESSIONS {
            if ui.button(*name).clicked() {
                picked = Some(*slots);
                ui.close();
            }
        }
    });
    if let Some(slots) = picked {
        for (i, s) in slots.iter().enumerate() {
            data[i * PROG_BYTES..(i + 1) * PROG_BYTES].copy_from_slice(&[s.0, s.1, s.2]);
        }
        params[1] = slots.len() as f32;
        changed = true;
    }
    text(&p, pos2(foot.right(), foot.center().y), Align2::RIGHT_CENTER, live.scale.name(), small(), TEXT_DIM);
    changed
}
