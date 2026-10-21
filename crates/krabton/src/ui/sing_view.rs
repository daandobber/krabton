//! The Sing sequencer's editor: a line to type the words or sounds in, and
//! under it the song they make, as a lyric sheet: each syllable written under
//! its note, the notes as long as they are sung and as high as they go, the
//! melody drawn through them, and the one sung now lit. The sheet can be
//! drawn on: sweep over the notes to draw the melody (it becomes the Drawn
//! melody), drag a note's end to make it longer or shorter, double-click a
//! note to give its length back to the rhythm.

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Id, Rect, Sense, Shape, Stroke, Ui, UiBuilder};
use krabton_engine::sequencers::sing;

/// What a drag on the sheet does: draws notes (from where the pointer was
/// last), or stretches one (which, its length and where the drag began).
#[derive(Clone, Copy)]
enum Drag {
    Draw(egui::Pos2),
    Stretch(usize, u32, f32),
}

pub(crate) fn sing(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let w = 320.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    rect_fill(ui.painter(), rect, 3.0, KNOB);
    let mut changed = false;
    // The words, in anything: romaji, kana, other words (what was drawn stays).
    let mut s = String::from_utf8_lossy(sing::text(data)).into_owned();
    let field = Rect::from_min_size(rect.min + vec2(6.0, 6.0), vec2(w - 12.0, 22.0));
    let mut child = ui.new_child(UiBuilder::new().max_rect(field));
    let tip = "What it sings: words split into syllables (sakura: sa ku ra; hap-py), kana into morae (さくら), each syllable a note with it as its lyric, for the Singer";
    let resp = child.add_sized(field.size(), egui::TextEdit::singleline(&mut s).id(id.with("text")).hint_text("sa ku ra / さくら / la la la").char_limit(200)).on_hover_text(tip);
    if resp.changed() {
        let mut end = s.len().min(400);
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        let new = sing::with_drawn(&s.as_bytes()[..end], &sing::drawn(data));
        if new != *data {
            *data = new;
            changed = true;
        }
    }

    // The sheet: this pass's notes, the melody through them, the syllables under them.
    let on = live.on();
    let pass = on.map(|v| v[1]).unwrap_or(0);
    let now = on.and_then(|v| (v[0] as usize).checked_sub(1));
    let plan = sing::plan(params, data, pass);
    let sheet = Rect::from_min_max(pos2(rect.left() + 8.0, field.bottom() + 8.0), pos2(rect.right() - 8.0, rect.bottom() - 20.0));
    let mut foot = None;
    if plan.sung.is_empty() || plan.length == 0 {
        text(ui.painter(), sheet.center(), Align2::CENTER_CENTER, "type something to sing".to_string(), small(), TEXT_DIM);
    } else {
        let words_h = 14.0;
        let notes = Rect::from_min_max(sheet.min, pos2(sheet.right(), sheet.bottom() - words_h));
        let step_w = notes.width() / plan.length as f32;
        // The heights shown: what is sung and some room around it, kept while a drag lasts.
        let range = params.get(sing::RANGE).map(|r| r.round() as i32).unwrap_or(5);
        let lo_hi = (plan.sung.iter().map(|s| s.degree).min().unwrap_or(0).min(-2), plan.sung.iter().map(|s| s.degree).max().unwrap_or(0).max(range.max(4) + 2));
        let held = ui.ctx().memory(|m| m.data.get_temp::<(i32, i32)>(id.with("lohi")));
        let (lo, hi) = held.unwrap_or(lo_hi);
        let bar_h = (notes.height() / (hi - lo + 1) as f32).clamp(3.0, 8.0);
        let span = (notes.height() - bar_h).max(1.0);
        let y_of = |d: i32| notes.bottom() - bar_h * 0.5 - (d - lo) as f32 / (hi - lo).max(1) as f32 * span;
        let deg_of = |y: f32| (lo + ((notes.bottom() - bar_h * 0.5 - y) / span * (hi - lo) as f32).round() as i32).clamp(lo, hi);
        let bars: Vec<Rect> = plan.sung.iter().map(|s| Rect::from_min_size(pos2(notes.left() + s.start as f32 * step_w + 1.0, y_of(s.degree) - bar_h * 0.5), vec2((s.steps as f32 * step_w - 2.0).max(2.0), bar_h))).collect();
        // Which note is under x, and whether the pointer is on a note's end.
        let note_at = |x: f32| plan.sung.iter().position(|s| x < notes.left() + (s.start + s.steps) as f32 * step_w).filter(|_| x >= notes.left());
        let end_at = |p: egui::Pos2| bars.iter().position(|b| (p.x - b.right()).abs() < 5.0 && (p.y - b.center().y).abs() < bar_h + 4.0);

        let resp = ui.interact(notes, id.with("sheet"), Sense::click_and_drag()).on_hover_text("Sweep over the notes to draw the melody; drag a note's end for its length; double-click a note: its length back to the rhythm's");
        if let Some(hp) = resp.hover_pos() {
            ui.ctx().set_cursor_icon(if end_at(hp).is_some() { CursorIcon::ResizeHorizontal } else { CursorIcon::Crosshair });
        }
        let drag_id = id.with("drag");
        if resp.drag_started() {
            // From where it was pressed (a drag begins a few pixels later).
            if let Some(p) = ui.input(|i| i.pointer.press_origin()).or(resp.interact_pointer_pos()) {
                let d = match end_at(p) {
                    Some(i) => Drag::Stretch(i, plan.sung[i].steps, p.x),
                    None => Drag::Draw(p),
                };
                ui.ctx().memory_mut(|m| {
                    m.data.insert_temp(drag_id, d);
                    m.data.insert_temp(id.with("lohi"), (lo, hi));
                });
            }
        }
        // What is drawn so far: from the melody as it is (the first time round) where nothing was.
        let start_drawing = |params: &[f32], data: &[u8]| {
            let mut p0 = params.to_vec();
            if let Some(v) = p0.get_mut(sing::VARIATION) {
                *v = 0.0;
            }
            let first = sing::plan(&p0, data, 0);
            let mut d = sing::drawn(data);
            d.resize(first.sung.len(), (0, 0));
            if params.get(sing::MELODY).is_none_or(|m| m.round() as usize != sing::DRAWN) {
                for (k, s) in first.sung.iter().enumerate() {
                    d[k].0 = s.degree.clamp(-100, 100) as i8;
                }
            }
            d
        };
        let drag = ui.ctx().memory(|m| m.data.get_temp::<Drag>(drag_id));
        // A click draws the one note under it.
        let drawing = match (drag, resp.clicked()) {
            (_, true) => resp.interact_pointer_pos().map(|p| (p, p)),
            (Some(Drag::Draw(last)), _) if resp.dragged() => resp.interact_pointer_pos().map(|p| (last, p)),
            _ => None,
        };
        if let Some((from, to)) = drawing {
            // Every note from where the pointer was to where it is, at the height between them.
            let mut d = start_drawing(params, data);
            let was_drawn = params.get(sing::MELODY).is_some_and(|m| m.round() as usize == sing::DRAWN);
            let (x0, x1) = (from.x.min(to.x), from.x.max(to.x));
            let mut touched = false;
            for (i, s) in plan.sung.iter().enumerate().take(d.len()) {
                let (l, r) = (notes.left() + s.start as f32 * step_w, notes.left() + (s.start + s.steps) as f32 * step_w);
                if r < x0 || l > x1 {
                    continue;
                }
                let cx = ((l + r) * 0.5).clamp(x0, x1);
                let t = if x1 - x0 > 0.5 { (cx - from.x) / (to.x - from.x) } else { 1.0 };
                let deg = deg_of(from.y + (to.y - from.y) * t.clamp(0.0, 1.0)) as i8;
                touched |= d[i].0 != deg;
                d[i].0 = deg;
                foot = Some(format!("{}: {}", plan.syllables[s.syllable], super::seq_more_view::degree_name(&live, params[sing::BASE], deg as i32)));
            }
            if touched || (!was_drawn && foot.is_some()) {
                if let Some(m) = params.get_mut(sing::MELODY) {
                    *m = sing::DRAWN as f32;
                }
                *data = sing::with_drawn(sing::text(data), &d);
                changed = true;
            }
            if drag.is_some() {
                ui.ctx().memory_mut(|m| m.data.insert_temp(drag_id, Drag::Draw(to)));
            }
        }
        if resp.dragged() {
            if let (Some(p), Some(drag)) = (resp.interact_pointer_pos(), drag) {
                let mut d = start_drawing(params, data);
                match drag {
                    Drag::Draw(_) => {}
                    Drag::Stretch(i, steps, x0) => {
                        let l = (steps as i32 + ((p.x - x0) / step_w).round() as i32).clamp(1, 32) as u8;
                        if i < d.len() && d[i].1 != l {
                            d[i].1 = l;
                            *data = sing::with_drawn(sing::text(data), &d);
                            changed = true;
                        }
                        foot = Some(format!("{}: {} steps", plan.syllables[plan.sung[i.min(plan.sung.len() - 1)].syllable], l));
                    }
                }
            }
        }
        if resp.drag_stopped() {
            ui.ctx().memory_mut(|m| {
                m.data.remove::<Drag>(drag_id);
                m.data.remove::<(i32, i32)>(id.with("lohi"));
            });
        }
        if resp.double_clicked() {
            if let Some(i) = resp.interact_pointer_pos().and_then(|p| note_at(p.x)) {
                let mut d = sing::drawn(data);
                if let Some(n) = d.get_mut(i).filter(|n| n.1 > 0) {
                    n.1 = 0;
                    *data = sing::with_drawn(sing::text(data), &d);
                    changed = true;
                }
            }
        }

        let p = ui.painter();
        // A faint line for each scale step, the base note a little stronger.
        for d in lo..=hi {
            p.line_segment([pos2(notes.left(), y_of(d)), pos2(notes.right(), y_of(d))], Stroke::new(1.0_f32, mix(KNOB, TEXT_DIM, if d == 0 { 0.35 } else { 0.12 })));
        }
        let drawn = sing::drawn(data);
        let mut line = Vec::with_capacity(plan.sung.len());
        let label_font = FontId::proportional(if step_w >= 18.0 { 11.0 } else { 9.0 });
        let mut last_label_end = f32::NEG_INFINITY;
        for (i, (s, bar)) in plan.sung.iter().zip(&bars).enumerate() {
            let here = now == Some(i);
            p.rect_filled(*bar, 2.0, if here { accent } else if s.word_start { mix(SLOT, accent, 0.35) } else { SLOT });
            // A length of its own: its end marked.
            if drawn.get(i).is_some_and(|d| d.1 > 0) {
                p.line_segment([pos2(bar.right() - 1.0, bar.top() - 1.0), pos2(bar.right() - 1.0, bar.bottom() + 1.0)], Stroke::new(1.5_f32, mix(TEXT, accent, 0.5)));
            }
            line.push(pos2(bar.left() + 1.0, bar.center().y));
            // Its syllable under the sheet, where it fits.
            let word = &plan.syllables[s.syllable];
            let x0 = notes.left() + s.start as f32 * step_w;
            if x0 >= last_label_end || here {
                let galley = p.layout_no_wrap(word.clone(), label_font.clone(), TEXT);
                let c = if here { accent } else if s.word_start { TEXT } else { TEXT_DIM };
                p.text(pos2(x0 + 1.0, sheet.bottom()), Align2::LEFT_BOTTOM, word, label_font.clone(), c);
                last_label_end = x0 + galley.size().x + 3.0;
            }
        }
        if line.len() > 1 {
            p.add(Shape::line(line, Stroke::new(1.0_f32, mix(TEXT_DIM, accent, 0.4))));
        }
    }
    let foot = foot.unwrap_or_else(|| match now.and_then(|i| plan.sung.get(i)) {
        Some(s) => format!("sings \u{201c}{}\u{201d}  ·  pass {}", plan.syllables[s.syllable], pass + 1),
        None if plan.sung.is_empty() => "Each syllable a note, with it as the lyric".to_string(),
        None => format!("{} syllables  ·  {} steps a pass  ·  draw on it", plan.sung.len(), plan.length),
    });
    text(ui.painter(), pos2(rect.left() + 6.0, rect.bottom() - 3.0), Align2::LEFT_BOTTOM, foot, small(), TEXT_DIM);
    // What was drawn can be wiped.
    if !sing::drawn(data).is_empty() {
        let r = Rect::from_min_size(pos2(rect.right() - 52.0, rect.bottom() - 17.0), vec2(46.0, 14.0));
        let resp = ui.interact(r, id.with("clear"), Sense::click()).on_hover_text("Clear what was drawn: the melody and the lengths as the rhythm and the melody shape go");
        text(ui.painter(), r.right_center(), Align2::RIGHT_CENTER, "clear".to_string(), small(), if resp.hovered() { accent } else { TEXT_DIM });
        if resp.clicked() {
            *data = sing::text(data).to_vec();
            if let Some(m) = params.get_mut(sing::MELODY).filter(|m| m.round() as usize == sing::DRAWN) {
                *m = 0.0;
            }
            changed = true;
        }
    }
    changed
}
