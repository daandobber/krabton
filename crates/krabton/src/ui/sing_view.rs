//! The Sing sequencer's editor: a line to type the words or sounds in, and
//! under it the song they make, as a lyric sheet: each syllable written under
//! its note, the notes as long as they are sung and as high as they go, the
//! melody drawn through them, and the one sung now lit.

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Shape, Stroke, Ui, UiBuilder};
use krabton_engine::sequencers::sing;

pub(crate) fn sing(ui: &mut Ui, id: Id, params: &[f32], data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let w = 320.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    rect_fill(ui.painter(), rect, 3.0, KNOB);
    let mut changed = false;
    // The words, in anything: romaji, kana, other words.
    let mut s = String::from_utf8_lossy(data).into_owned();
    let field = Rect::from_min_size(rect.min + vec2(6.0, 6.0), vec2(w - 12.0, 22.0));
    let mut child = ui.new_child(UiBuilder::new().max_rect(field));
    let tip = "What it sings: words split into syllables (sakura: sa ku ra; hap-py), kana into morae (さくら), each syllable a note with it as its lyric, for the Singer";
    let resp = child.add_sized(field.size(), egui::TextEdit::singleline(&mut s).id(id.with("text")).hint_text("sa ku ra / さくら / la la la").char_limit(200)).on_hover_text(tip);
    if resp.changed() {
        let mut end = s.len().min(400);
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        let bytes = s.as_bytes()[..end].to_vec();
        if bytes != *data {
            *data = bytes;
            changed = true;
        }
    }

    // The sheet: this pass's notes, the melody through them, the syllables under them.
    let on = live.on();
    let pass = on.map(|v| v[1]).unwrap_or(0);
    let now = on.and_then(|v| (v[0] as usize).checked_sub(1));
    let plan = sing::plan(params, data, pass);
    let p = ui.painter();
    let sheet = Rect::from_min_max(pos2(rect.left() + 8.0, field.bottom() + 8.0), pos2(rect.right() - 8.0, rect.bottom() - 20.0));
    if plan.sung.is_empty() || plan.length == 0 {
        text(p, sheet.center(), Align2::CENTER_CENTER, "type something to sing".to_string(), small(), TEXT_DIM);
    } else {
        let lo = plan.sung.iter().map(|s| s.degree).min().unwrap_or(0);
        let hi = plan.sung.iter().map(|s| s.degree).max().unwrap_or(0).max(lo + 4);
        let words_h = 14.0;
        let notes = Rect::from_min_max(sheet.min, pos2(sheet.right(), sheet.bottom() - words_h));
        let step_w = notes.width() / plan.length as f32;
        let bar_h = (notes.height() / (hi - lo + 1) as f32).clamp(3.0, 8.0);
        let y_of = |d: i32| notes.bottom() - bar_h * 0.5 - (d - lo) as f32 / (hi - lo).max(1) as f32 * (notes.height() - bar_h);
        // A faint line for each scale step.
        for d in lo..=hi {
            p.line_segment([pos2(notes.left(), y_of(d)), pos2(notes.right(), y_of(d))], Stroke::new(1.0_f32, mix(KNOB, TEXT_DIM, if d == 0 { 0.35 } else { 0.12 })));
        }
        let mut line = Vec::with_capacity(plan.sung.len());
        let label_font = FontId::proportional(if step_w >= 18.0 { 11.0 } else { 9.0 });
        let mut last_label_end = f32::NEG_INFINITY;
        for (i, s) in plan.sung.iter().enumerate() {
            let x0 = notes.left() + s.start as f32 * step_w;
            let bar = Rect::from_min_size(pos2(x0 + 1.0, y_of(s.degree) - bar_h * 0.5), vec2((s.steps as f32 * step_w - 2.0).max(2.0), bar_h));
            let here = now == Some(i);
            p.rect_filled(bar, 2.0, if here { accent } else if s.word_start { mix(SLOT, accent, 0.35) } else { SLOT });
            line.push(pos2(bar.left() + 1.0, bar.center().y));
            // Its syllable under the sheet, where it fits.
            let word = &plan.syllables[s.syllable];
            let galley = p.layout_no_wrap(word.clone(), label_font.clone(), TEXT);
            if x0 >= last_label_end || here {
                let c = if here { accent } else if s.word_start { TEXT } else { TEXT_DIM };
                p.text(pos2(x0 + 1.0, sheet.bottom()), Align2::LEFT_BOTTOM, word, label_font.clone(), c);
                last_label_end = x0 + galley.size().x + 3.0;
            }
        }
        if line.len() > 1 {
            p.add(Shape::line(line, Stroke::new(1.0_f32, mix(TEXT_DIM, accent, 0.4))));
        }
    }
    let foot = match now.and_then(|i| plan.sung.get(i)) {
        Some(s) => format!("sings \u{201c}{}\u{201d}  ·  pass {}", plan.syllables[s.syllable], pass + 1),
        None if plan.sung.is_empty() => "Each syllable a note, with it as the lyric".to_string(),
        None => format!("{} syllables  ·  {} steps a pass", plan.sung.len(), plan.length),
    };
    text(p, pos2(rect.left() + 6.0, rect.bottom() - 3.0), Align2::LEFT_BOTTOM, foot, small(), TEXT_DIM);
    changed
}
