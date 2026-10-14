//! The players' editor (Piano Player, Bass Player, Banjo Roll, String
//! Section): the chord cards of the progression, as in Progression, what
//! the player plays in a bar (click the style to change it), and keys that
//! light up with the notes it struck last.

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Ui};
use krabton_engine::midi_fx::MidiFxKind;
use krabton_engine::params::ParamKind;
use krabton_engine::sequencers::players::{self, STYLE};

pub(crate) fn player(ui: &mut Ui, id: Id, kind: MidiFxKind, params: &mut [f32], data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let strip_h = 26.0;
    let keys_h = 30.0;
    let cards_h = (height - strip_h - keys_h - 8.0).max(90.0);
    ui.vertical(|ui| {
        let mut changed = super::classic_view::progression(ui, id, params, data, live, accent, cards_h);
        ui.add_space(4.0);
        changed |= style_strip(ui, id, kind, params, live, accent, strip_h);
        ui.add_space(4.0);
        keys(ui, kind, params, live, accent, keys_h);
        changed
    })
    .inner
}

/// The style's name (click: next, right-click: previous) and its bar.
fn style_strip(ui: &mut Ui, id: Id, kind: MidiFxKind, params: &mut [f32], live: Live, accent: Color32, h: f32) -> bool {
    let w = 280.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let p = ui.painter().clone();
    let opts = match kind.specs()[STYLE].kind {
        ParamKind::Choice(o) => o,
        _ => &[],
    };
    let n = opts.len().max(1);
    let style = (params[STYLE].round().max(0.0) as usize).min(n - 1);
    let name_w = 92.0;
    let nr = Rect::from_min_size(rect.min, vec2(name_w, h));
    let resp = ui.interact(nr, id.with("style"), Sense::click());
    rect_fill(&p, nr, 3.0, if resp.hovered() { SLOT } else { PANEL3 });
    text(&p, pos2(nr.left() + 5.0, nr.center().y), Align2::LEFT_CENTER, "‹", small(), TEXT_DIM);
    text(&p, pos2(nr.right() - 5.0, nr.center().y), Align2::RIGHT_CENTER, "›", small(), TEXT_DIM);
    let label = opts.get(style).copied().unwrap_or("");
    text(&p, nr.center(), Align2::CENTER_CENTER, label, FontId::proportional(if label.len() > 11 { 10.5 } else { 12.5 }), TEXT);
    let mut changed = false;
    if resp.clicked() {
        params[STYLE] = ((style + 1) % n) as f32;
        changed = true;
    }
    if resp.secondary_clicked() {
        params[STYLE] = ((style + n - 1) % n) as f32;
        changed = true;
    }
    resp.on_hover_text("The style: click for the next, right-click for the previous");
    // A cell per sixteenth: what the player plays there.
    let steps = players::style_steps(kind, style);
    let vis = live.on();
    let now = vis.filter(|v| ((v[0] >> 16) & 0xff) as usize == style).map(|v| ((v[0] >> 8) & 0xff) as usize);
    let left = nr.right() + 6.0;
    let cell = (rect.right() - left) / 16.0;
    for (i, &s) in steps.iter().enumerate() {
        let r = Rect::from_min_size(pos2(left + i as f32 * cell, rect.top() + 3.0), vec2(cell - 2.0, h - 6.0));
        let here = now == Some(i);
        let fill = if here {
            accent
        } else if s > 0 {
            shade(accent, if s > 1 { 0.75 } else { 0.5 })
        } else if i % 4 == 0 {
            Color32::from_gray(0x4c)
        } else {
            PANEL3
        };
        rect_fill(&p, r, 2.0, fill);
    }
    changed
}

/// Two octaves and a half of keys around the player's range, lit where it
/// struck last.
fn keys(ui: &mut Ui, kind: MidiFxKind, params: &[f32], live: Live, accent: Color32, h: f32) {
    let w = 280.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let p = ui.painter().clone();
    let struck = live.on().map(|v| v[1] as u128 | (v[2] as u128) << 64).unwrap_or(0);
    let base = params[players::BASE].round() as i32;
    // From the C under the lowest note it plays (the piano's left hand is an octave down).
    let low = base - if kind == MidiFxKind::PianoPlayer { 12 } else { 0 };
    let first = low - low.rem_euclid(12);
    let mut lowest = first;
    if struck != 0 {
        let lo = struck.trailing_zeros() as i32;
        if lo < lowest {
            lowest = lo - lo.rem_euclid(12);
        }
    }
    let first = lowest;
    let octaves = 3;
    let whites = 7 * octaves;
    let kw = w / whites as f32;
    let is_black = |n: i32| matches!(n.rem_euclid(12), 1 | 3 | 6 | 8 | 10);
    let lit = |n: i32| (0..128).contains(&n) && struck >> n & 1 == 1;
    let mut x = rect.left();
    let mut white_x = Vec::new();
    for n in first..first + 12 * octaves {
        if is_black(n) {
            continue;
        }
        let r = Rect::from_min_size(pos2(x, rect.top()), vec2(kw - 1.0, h));
        rect_fill(&p, r, 2.0, if lit(n) { accent } else { Color32::from_gray(205) });
        if n.rem_euclid(12) == 0 {
            text(&p, pos2(r.center().x, r.bottom() - 6.0), Align2::CENTER_CENTER, &krabton_engine::note_name(n.clamp(0, 127) as u8), FontId::proportional(8.0), TEXT_DARK);
        }
        white_x.push((n, x));
        x += kw;
    }
    for &(n, x) in &white_x {
        let b = n + 1;
        if b < first + 12 * octaves && is_black(b) {
            let r = Rect::from_min_size(pos2(x + kw * 0.65, rect.top()), vec2(kw * 0.7, h * 0.6));
            rect_fill(&p, r, 2.0, if lit(b) { accent } else { Color32::from_gray(40) });
        }
    }
}
