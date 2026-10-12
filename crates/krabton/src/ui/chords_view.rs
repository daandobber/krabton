//! The Guitar Chords editor: a card per chord with its name and chord
//! diagram (click the name for the next chord, right-click for the
//! previous; the buttons set the chord type and where on the neck it is
//! played), and the strum pattern underneath, a stroke per sixteenth
//! (click to change it, right-click to go back).

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Painter, Rect, Sense, Stroke, Ui};
use krabton_engine::classic_seq::{self as cs, PROG_BYTES, PROG_SLOTS};
use krabton_engine::sequencers::strum::{self as gs, Shape};

/// Progressions that suit the guitar: (name, chords as degree, type, neck).
const PROGRESSIONS: &[(&str, &[(u8, u8, u8)])] = &[
    ("I - V - vi - IV", &[(1, 0, 0), (5, 0, 0), (6, 0, 0), (4, 0, 0)]),
    ("I - IV - V", &[(1, 0, 0), (4, 0, 0), (5, 0, 0), (5, 1, 0)]),
    ("vi - IV - I - V", &[(6, 0, 0), (4, 0, 0), (1, 0, 0), (5, 0, 0)]),
    ("I - vi - IV - V", &[(1, 0, 0), (6, 0, 0), (4, 0, 0), (5, 0, 0)]),
    ("ii - V - I", &[(2, 1, 0), (5, 1, 0), (1, 1, 0), (1, 1, 0)]),
    ("Twelve-bar blues", &[(1, 1, 0), (1, 1, 0), (1, 1, 0), (1, 1, 0), (4, 1, 0), (4, 1, 0), (1, 1, 0), (5, 1, 0)]),
    ("Power chords", &[(1, 4, 0), (6, 4, 0), (4, 4, 0), (5, 4, 0)]),
    ("Barre ballad", &[(1, 0, 1), (3, 0, 1), (6, 0, 1), (4, 0, 1)]),
    ("Sus shimmer", &[(1, 2, 0), (4, 5, 0), (6, 3, 0), (5, 3, 0)]),
];

/// A chord diagram: strings upright (low E on the left), the nut or the
/// first fret's number, dots on the frets, x and o above.
pub(crate) fn diagram(p: &Painter, r: Rect, shape: &Shape, color: Color32) {
    let fretted: Vec<u8> = shape.iter().flatten().copied().filter(|&f| f > 0).collect();
    let lowest = fretted.iter().copied().min().unwrap_or(1);
    let highest = fretted.iter().copied().max().unwrap_or(1);
    // Four frets from the first fret, or from the lowest one up the neck.
    let first = if highest <= 4 { 1 } else { lowest };
    let top = r.top() + 8.0;
    let gap_x = r.width() / 5.0;
    let gap_y = (r.bottom() - top) / 4.0;
    for s in 0..6 {
        let x = r.left() + s as f32 * gap_x;
        p.line_segment([pos2(x, top), pos2(x, r.bottom())], Stroke::new(1.0_f32, TEXT_DIM));
    }
    for k in 0..=4 {
        let y = top + k as f32 * gap_y;
        let w = if k == 0 && first == 1 { 2.5_f32 } else { 1.0 };
        p.line_segment([pos2(r.left(), y), pos2(r.right(), y)], Stroke::new(w, TEXT_DIM));
    }
    if first > 1 {
        text(p, pos2(r.left() - 3.0, top + gap_y * 0.5), Align2::RIGHT_CENTER, first.to_string(), small(), TEXT_DIM);
    }
    for (s, f) in shape.iter().enumerate() {
        let x = r.left() + s as f32 * gap_x;
        match f {
            None => {
                text(p, pos2(x, r.top() + 3.0), Align2::CENTER_CENTER, "x", FontId::monospace(9.0), TEXT_DIM);
            }
            Some(0) => {
                p.circle_stroke(pos2(x, r.top() + 3.0), 2.2, Stroke::new(1.0_f32, color));
            }
            Some(f) => {
                let k = (*f as i32 - first as i32).clamp(0, 3) as f32;
                p.circle_filled(pos2(x, top + (k + 0.5) * gap_y), (gap_x * 0.38).min(3.5), color);
            }
        }
    }
}

pub(crate) fn guitar_chords(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    if data.len() < gs::DATA_BYTES {
        let mut d = gs::strum_default();
        let n = data.len().min(d.len());
        d[..n].copy_from_slice(&data[..n]);
        *data = d;
        changed = true;
    }
    let (cw, gap) = (88.0, 4.0);
    let strip_h = 34.0;
    let ch = ((height - strip_h - gap) / 2.0 - gap).clamp(44.0, 74.0);
    let w = 4.0 * (cw + gap);
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    let chords = params[gs::CHORDS].round().clamp(1.0, PROG_SLOTS as f32) as usize;
    let vis = live.on();
    let current = vis.map(|v| (v[0] & 0xff) as usize);
    for slot in 0..PROG_SLOTS {
        let card = Rect::from_min_size(pos2(rect.left() + (slot % 4) as f32 * (cw + gap), rect.top() + (slot / 4) as f32 * (ch + gap)), vec2(cw, ch));
        let (degree, quality, neck) = cs::prog_slot(data, slot);
        let used = slot < chords;
        let playing = current == Some(slot);
        rect_fill(&p, card, 4.0, if playing { shade(accent, 0.45) } else if used { PANEL3 } else { PANEL2 });
        if playing {
            rect_line(&p, card, 4.0, 1.5, accent);
        }
        let ink = if used { TEXT } else { TEXT_DIM };
        // The name: click for the next chord, right-click for the previous.
        let name_r = Rect::from_min_size(card.min, vec2(cw * 0.5, ch - 16.0));
        let o = slot * PROG_BYTES;
        let tresp = ui.interact(name_r, id.with(("deg", slot)), Sense::click());
        if tresp.clicked() {
            data[o] = (data[o] + 1) % 8;
            changed = true;
        }
        if tresp.secondary_clicked() {
            data[o] = (data[o] + 7) % 8;
            changed = true;
        }
        let (letter, roman) = if degree == 0 { ("rest".to_string(), String::new()) } else { (gs::chord_letter_name(&live.scale, degree, quality), cs::chord_name(&live.scale, degree, quality)) };
        text(&p, pos2(card.left() + 5.0, card.top() + 4.0), Align2::LEFT_TOP, &letter, FontId::proportional(if letter.len() > 5 { 12.0 } else { 16.0 }), ink);
        text(&p, pos2(card.left() + 5.0, card.top() + 23.0), Align2::LEFT_TOP, &roman, small(), TEXT_DIM);
        let shape = if playing { vis.map(|v| v[1]).filter(|&v| v != u64::MAX).map(gs::unpack_shape) } else { None }.or_else(|| gs::slot_shape(&live.scale, data, slot, 0).map(|s| s.0));
        if let Some(shape) = shape {
            let notes: Vec<String> = gs::shape_notes(&shape).iter().map(|n| krabton_engine::note_name(*n as u8)).collect();
            tresp.on_hover_text(format!("{} · click: next chord, right-click: previous", notes.join(" ")));
            let dr = Rect::from_min_max(pos2(card.left() + cw * 0.5 + 4.0, card.top() + 4.0), pos2(card.right() - 6.0, card.bottom() - 19.0));
            diagram(&p, dr, &shape, if playing { Color32::WHITE } else { accent });
        } else {
            tresp.on_hover_text("A rest · click: next chord, right-click: previous");
        }
        let q = Rect::from_min_size(pos2(card.left() + 2.0, card.bottom() - 16.0), vec2(cw * 0.5 - 3.0, 14.0));
        let nk = Rect::from_min_max(pos2(q.right() + 2.0, q.top()), pos2(card.right() - 2.0, q.bottom()));
        if button(ui, q, id.with(("q", slot)), cs::QUALITIES[quality as usize], false, accent).on_hover_text("Chord type").clicked() {
            data[o + 1] = (quality + 1) % cs::QUALITIES.len() as u8;
            changed = true;
        }
        if button(ui, nk, id.with(("neck", slot)), gs::NECKS[neck.min(2) as usize], false, accent).on_hover_text("Where on the neck: open chords, barre chords, or high up").clicked() {
            data[o + 2] = (neck.min(2) + 1) % gs::NECKS.len() as u8;
            changed = true;
        }
    }
    // The pattern: a stroke per sixteenth.
    let strip = Rect::from_min_size(pos2(rect.left(), rect.bottom() - strip_h), vec2(w, strip_h));
    let menu_w = 74.0;
    let cell = (w - menu_w - 6.0) / gs::PATTERN_STEPS as f32;
    let step_now = vis.map(|v| ((v[0] >> 8) & 0xff) as usize);
    for i in 0..gs::PATTERN_STEPS {
        let r = Rect::from_min_size(pos2(strip.left() + i as f32 * cell, strip.top() + 2.0), vec2(cell - 2.0, strip_h - 4.0));
        let s = gs::pattern_at(data, i);
        let here = step_now == Some(i);
        let on_beat = i % 4 == 0;
        rect_fill(&p, r, 2.0, if here { accent } else if s != gs::REST { SLOT } else if on_beat { PANEL3 } else { PANEL2 });
        let ink = if here { TEXT_DARK } else if s == gs::REST { TEXT_DIM } else { TEXT };
        if matches!(s, gs::DOWN | gs::UP | gs::SOFT_DOWN | gs::SOFT_UP) {
            // An arrow: down strokes point down, soft ones are thinner.
            let down = matches!(s, gs::DOWN | gs::SOFT_DOWN);
            let soft = matches!(s, gs::SOFT_DOWN | gs::SOFT_UP);
            let (w, half) = (if soft { 1.2_f32 } else { 2.0 }, r.height() * 0.3);
            let c = r.center();
            let (tail, tip) = if down { (c.y - half, c.y + half) } else { (c.y + half, c.y - half) };
            let back = if down { -5.0 } else { 5.0 };
            let col = if soft { ink.gamma_multiply(0.7) } else { ink };
            p.line_segment([pos2(c.x, tail), pos2(c.x, tip)], Stroke::new(w, col));
            p.line_segment([pos2(c.x, tip), pos2(c.x - 4.0, tip + back)], Stroke::new(w, col));
            p.line_segment([pos2(c.x, tip), pos2(c.x + 4.0, tip + back)], Stroke::new(w, col));
        } else {
            text(&p, r.center(), Align2::CENTER_CENTER, gs::stroke_label(s), FontId::proportional(12.0), ink);
        }
        let resp = ui.interact(r, id.with(("stroke", i)), Sense::click());
        let o = PROG_SLOTS * PROG_BYTES + i;
        if resp.clicked() {
            data[o] = (s + 1) % gs::STROKES;
            changed = true;
        }
        if resp.secondary_clicked() {
            data[o] = (s + gs::STROKES - 1) % gs::STROKES;
            changed = true;
        }
        resp.on_hover_text(format!("{}: {} · click: next stroke, right-click: previous", i / 4 + 1, gs::stroke_name(s)));
    }
    // Presets: patterns and progressions.
    let mr = Rect::from_min_size(pos2(strip.right() - menu_w, strip.top() + 2.0), vec2(menu_w, (strip_h - 6.0) / 2.0));
    let pr = Rect::from_min_size(pos2(mr.left(), mr.bottom() + 2.0), mr.size());
    let pmenu = button(ui, mr, id.with("patterns"), "Patterns", false, accent);
    let mut picked_pattern = None;
    egui::Popup::menu(&pmenu).show(|ui| {
        for (name, steps) in gs::PATTERNS {
            if ui.button(*name).clicked() {
                picked_pattern = Some(*steps);
                ui.close();
            }
        }
    });
    if let Some(steps) = picked_pattern {
        let o = PROG_SLOTS * PROG_BYTES;
        data[o..o + gs::PATTERN_STEPS].copy_from_slice(&steps);
        changed = true;
    }
    let cmenu = button(ui, pr, id.with("progressions"), "Chords", false, accent);
    let mut picked = None;
    egui::Popup::menu(&cmenu).show(|ui| {
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
        params[gs::CHORDS] = slots.len() as f32;
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput};
    use krabton_engine::midi_fx::MidiFxKind;
    use krabton_engine::params::defaults;

    #[test]
    fn clicking_a_stroke_changes_it_and_cards_draw() {
        let mut params = defaults(MidiFxKind::GuitarChords.specs());
        let mut data = gs::strum_default();
        let ctx = egui::Context::default();
        let mut origin = Pos2::ZERO;
        let height = 190.0;
        for f in 0..3 {
            let mut input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() };
            if f > 0 {
                // The second stroke cell (a rest in Old Faithful).
                let cell = (4.0 * 92.0 - 74.0 - 6.0) / 16.0;
                let at = origin + vec2(1.5 * cell, height - 17.0);
                input.events.push(Event::PointerMoved(at));
                input.events.push(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: f == 1, modifiers: Modifiers::NONE });
            }
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    origin = ui.cursor().min;
                    let live = Live { vis: Some([0 | 1 << 8, gs::pack_shape(&[None, Some(3), Some(2), Some(0), Some(1), Some(0)]), 0, 0]), playing: true, scale: krabton_engine::Scale::default(), kit: 0 };
                    super::super::seq_view::editor(ui, Id::new("t"), MidiFxKind::GuitarChords, &mut params, &mut data, live, &mut 0, accent_now(), height);
                });
            });
        }
        assert_eq!(gs::pattern_at(&data, 1), gs::DOWN);
    }
}
