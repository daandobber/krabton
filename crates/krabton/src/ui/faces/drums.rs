//! A drum machine laid out as one: a column per instrument, its name on
//! top, its knobs (level, tone, decay, tune) one under the other and its
//! step button at the foot, in the machine's colours. A button lights while
//! its drum plays; click it to hear it and to pick it (its other settings
//! show beside the columns); drop a sample on a column to put it there.

use super::knob;
use crate::app::BrowserItem;
use crate::theme::*;
use crate::widgets::drop_payload;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Stroke, Ui};
use krabton_engine::params::{ParamSpec, Unit};
use krabton_engine::project::DrumPad;
use krabton_engine::SampleRef;

static KNOBS: [ParamSpec; 4] = [
    ParamSpec::lin("Level", "Pad", -36.0, 6.0, 0.0, Unit::Db),
    ParamSpec::lin("Tone", "Pad", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::log("Decay", "Pad", 0.1, 4.0, 1.0, Unit::None),
    ParamSpec::int("Tune", "Pad", -24.0, 24.0, 0.0, Unit::Semi),
];

/// The columns, `height` high. `sel` is the picked pad, `hit` a pad to hear,
/// `notes` the notes playing now. Returns whether a pad changed.
pub(crate) fn columns(ui: &mut Ui, id: Id, pads: &mut [DrumPad], sel: &mut usize, hit: &mut Option<u8>, notes: u128, height: f32) -> bool {
    let u = (height / 12.0).clamp(10.0, 24.0);
    let mut order: Vec<usize> = (0..pads.len().min(16)).collect();
    order.sort_by_key(|&k| pads[k].note);
    let n = order.len().max(1);
    let col_w = 2.7 * u;
    let (rect, _) = ui.allocate_exact_size(vec2(col_w * n as f32, 12.0 * u), Sense::hover());
    let p = ui.painter().clone();
    let skin = crate::skin::current();
    let palette = skin.map(|s| s.pads).filter(|p| !p.is_empty());
    let mut changed = false;
    for (c, &k) in order.iter().enumerate() {
        let x = rect.left() + c as f32 * col_w;
        let col = Rect::from_min_size(pos2(x, rect.top()), vec2(col_w, rect.height()));
        let selected = *sel == k;
        // A rule between the columns, as on the panel.
        if c > 0 {
            let ink = crate::skin::ink();
            p.line_segment([pos2(x, col.top() + 4.0), pos2(x, col.bottom() - 2.0 * u)], Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), 40)));
        }
        // The name: the drum's, in the panel's print.
        let name_r = Rect::from_min_size(pos2(x + 2.0, col.top()), vec2(col_w - 4.0, 0.9 * u));
        let name = crate::skin::label_text(&pads[k].name);
        let font = FontId::proportional((u * 0.5).clamp(8.0, 12.0));
        let width = p.layout_no_wrap(name.clone(), font.clone(), Color32::WHITE).size().x;
        let font = if width > name_r.width() { FontId::proportional((font.size * name_r.width() / width).max(7.0)) } else { font };
        p.text(name_r.center(), Align2::CENTER_CENTER, &name, font, if selected { accent_now() } else { crate::skin::ink() });
        // Its knobs.
        let pad = &mut pads[k];
        let mut vals = [pad.level_db, pad.tone, pad.decay, pad.tune];
        for (j, spec) in KNOBS.iter().enumerate() {
            let r = Rect::from_min_size(pos2(x + 0.15 * u, col.top() + (1.0 + j as f32 * 2.2) * u), vec2(col_w - 0.3 * u, 2.2 * u));
            changed |= knob(ui, r, id.with((k, j)), spec, &mut vals[j], spec.name, u);
        }
        [pad.level_db, pad.tone, pad.decay, pad.tune] = vals;
        // Its step button, lit while it plays.
        let b = Rect::from_min_size(pos2(x + 0.3 * u, col.top() + 10.0 * u), vec2(col_w - 0.6 * u, 1.7 * u));
        let resp = ui.interact(b, id.with(("pad", k)), Sense::click());
        let color = match palette {
            Some(pal) => pal[(c * pal.len() / n).min(pal.len() - 1)],
            None => crate::skin::well().0,
        };
        let playing = notes >> pad.note & 1 == 1;
        let fill = if resp.is_pointer_button_down_on() { mix(color, Color32::WHITE, 0.35) } else { color };
        p.rect_filled(b.translate(vec2(0.0, 2.0)), 3.0, Color32::from_black_alpha(110));
        p.rect_filled(b, 3.0, fill);
        p.rect_filled(Rect::from_min_size(b.min + vec2(2.0, 2.0), vec2(b.width() - 4.0, b.height() * 0.3)), 2.0, Color32::from_white_alpha(30));
        if selected {
            p.rect_stroke(b.expand(1.5), 4.0, Stroke::new(1.5_f32, accent_now()), egui::StrokeKind::Outside);
        }
        let led = pos2(b.center().x, b.top() - 0.35 * u);
        p.circle_filled(led, (u * 0.16).max(2.5), if playing || resp.is_pointer_button_down_on() { REC } else { Color32::from_black_alpha(130) });
        let sub = if pad.sample.is_some() || !pad.layers.is_empty() { format!("{} ·", krabton_engine::note_name(pad.note)) } else { krabton_engine::note_name(pad.note) };
        p.text(b.center(), Align2::CENTER_CENTER, sub, FontId::proportional((u * 0.45).clamp(7.5, 10.5)), text_on(fill));
        if resp.clicked() {
            *sel = k;
            *hit = Some(pad.note);
        }
        let resp = resp.on_hover_text(format!("{}: click to hear and pick it; drop a sample here", pad.name));
        // A sample dropped on the column goes on its pad.
        let drop = ui.interact(col, id.with(("drop", k)), Sense::hover());
        for r in [&resp, &drop] {
            if r.dnd_hover_payload::<BrowserItem>().is_some() {
                p.rect_stroke(col.shrink(1.0), 3.0, Stroke::new(2.0_f32, accent_now()), egui::StrokeKind::Inside);
            }
            if let Some(item) = drop_payload::<BrowserItem>(r) {
                if let BrowserItem::Sample(path) = &*item {
                    if let Ok(s) = SampleRef::load(path) {
                        pad.name = s.name.clone();
                        pad.sample = Some(s);
                        changed = true;
                    }
                }
            }
        }
    }
    changed
}
