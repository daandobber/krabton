//! The Orchestra's (and the Soloists') screen: the instrument and its articulations (click one),
//! the Dynamics fader (drag it, or move it from a modulator: the sound
//! swells through its recorded layers), the keys the articulation plays with
//! the notes held lit, and, until the instrument is on this computer, its
//! download.

use super::inst_screen::{caption, InstLive, SCREEN_BG};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Stroke, Ui};
use krabton_engine::orchestra::{self, ARTICULATION, DYNAMICS, INSTRUMENT, INSTRUMENTS};

/// Where the instrument's samples are.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LibState {
    Ready,
    Missing,
    Installing(String, f32),
    Failed(String),
}

const FAINT: Color32 = Color32::from_rgb(0x1e, 0x21, 0x28);

/// Returns (changed, the download asked for).
pub(crate) fn orchestra_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32, lib: LibState) -> (bool, bool) {
    let w = 440.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let painter = ui.painter_at(rect);
    rect_fill(&painter, rect, 6.0, SCREEN_BG);
    rect_line(&painter, rect, 6.0, 1.0, Color32::from_rgb(0x26, 0x29, 0x31));
    let accent = accent_now();
    let mut changed = false;
    let mut install = false;
    let inst = (p.get(INSTRUMENT).copied().unwrap_or(0.0).round().max(0.0) as usize).min(INSTRUMENTS.len() - 1);
    let def = &INSTRUMENTS[inst];
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, def.name, accent);
    painter.text(rect.right_top() + vec2(-10.0, 6.0), Align2::RIGHT_TOP, format!("{:?}", def.family).to_uppercase(), FontId::monospace(9.0), TEXT_DIM);
    // The articulations.
    let art = (p.get(ARTICULATION).copied().unwrap_or(0.0).round().max(0.0) as usize).min(def.arts.len() - 1);
    let mut x = rect.left() + 10.0;
    let mut y = rect.top() + 24.0;
    for (i, a) in def.arts.iter().enumerate() {
        let tw = 9.0 + a.label.len() as f32 * 6.6;
        if x + tw > rect.right() - 10.0 {
            x = rect.left() + 10.0;
            y += 20.0;
        }
        let r = Rect::from_min_size(pos2(x, y), vec2(tw, 17.0));
        let resp = ui.interact(r, id.with(("art", i)), Sense::click());
        let on = i == art;
        painter.rect_filled(r, 4.0, if on { accent } else if resp.hovered() { Color32::from_rgb(0x2c, 0x30, 0x3a) } else { FAINT });
        painter.text(r.center(), Align2::CENTER_CENTER, a.label, FontId::proportional(11.0), if on { TEXT_DARK } else { TEXT });
        if resp.clicked() && !on {
            p[ARTICULATION] = i as f32;
            changed = true;
        }
        x += tw + 5.0;
    }
    let body = Rect::from_min_max(pos2(rect.left() + 10.0, y + 26.0), pos2(rect.right() - 10.0, rect.bottom() - 10.0));
    // The Dynamics fader.
    let fr = Rect::from_min_size(body.min, vec2(34.0, body.height()));
    let resp = ui.interact(fr, id.with("dyn"), Sense::click_and_drag());
    if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        let v = ((fr.bottom() - pos.y) / fr.height()).clamp(0.0, 1.0);
        let v = (v * 100.0).round() / 100.0;
        if p[DYNAMICS] != v {
            p[DYNAMICS] = v;
            changed = true;
        }
    }
    painter.rect_filled(fr, 4.0, FAINT);
    let d = p[DYNAMICS].clamp(0.0, 1.0);
    painter.rect_filled(Rect::from_min_max(pos2(fr.left(), fr.bottom() - d * fr.height()), fr.max), 4.0, accent.gamma_multiply(0.55));
    // Where the sound is now (it glides there).
    if lib == LibState::Ready {
        let now = live.vals[1].clamp(0.0, 1.0);
        let yy = fr.bottom() - now * fr.height();
        painter.line_segment([pos2(fr.left(), yy), pos2(fr.right(), yy)], Stroke::new(2.0_f32, YELLOW_DYN));
    }
    for (k, mark) in ["pp", "p", "mp", "mf", "f", "ff"].iter().enumerate() {
        let yy = fr.bottom() - (k as f32 + 0.5) / 6.0 * fr.height();
        painter.text(pos2(fr.right() + 4.0, yy), Align2::LEFT_CENTER, *mark, FontId::proportional(9.0), TEXT_DIM);
    }
    // The keys it plays.
    let kr = Rect::from_min_max(pos2(fr.right() + 26.0, body.top() + body.height() * 0.45), body.max);
    let range = orchestra::library(inst).map(|l| l.range(art));
    let (lo, hi) = (21u8, 108u8);
    let whites: Vec<u8> = (lo..=hi).filter(|n| !matches!(n % 12, 1 | 3 | 6 | 8 | 10)).collect();
    let kw = kr.width() / whites.len() as f32;
    for (i, &n) in whites.iter().enumerate() {
        let r = Rect::from_min_size(pos2(kr.left() + i as f32 * kw, kr.top()), vec2(kw - 1.0, kr.height()));
        let playable = range.is_some_and(|(a, b)| a <= n && n <= b);
        let lit = live.notes & (1u128 << n) != 0;
        painter.rect_filled(r, 1.0, if lit { accent } else if playable { Color32::from_rgb(0xc8, 0xc8, 0xcc) } else { Color32::from_rgb(0x48, 0x4a, 0x50) });
    }
    for n in lo..=hi {
        if !matches!(n % 12, 1 | 3 | 6 | 8 | 10) {
            continue;
        }
        let left = whites.iter().position(|&w| w > n).unwrap_or(whites.len()) as f32;
        let r = Rect::from_center_size(pos2(kr.left() + left * kw, kr.top() + kr.height() * 0.3), vec2(kw * 0.6, kr.height() * 0.6));
        let playable = range.is_some_and(|(a, b)| a <= n && n <= b);
        let lit = live.notes & (1u128 << n) != 0;
        painter.rect_filled(r, 1.0, if lit { accent } else if playable { Color32::from_rgb(0x20, 0x22, 0x26) } else { Color32::from_rgb(0x30, 0x32, 0x38) });
    }
    // Above the keys: what it is, or getting it.
    let info = Rect::from_min_max(pos2(kr.left(), body.top()), pos2(kr.right(), kr.top() - 6.0));
    match &lib {
        LibState::Ready => {
            let (a, b) = range.unwrap_or((0, 0));
            let held = if def.arts[art].held { "held notes loop; Legato joins them" } else { "short notes play out" };
            painter.text(info.left_top(), Align2::LEFT_TOP, format!("{}  ·  {} to {}  ·  {}", def.arts[art].label, krabton_engine::note_name(a), krabton_engine::note_name(b), held), FontId::proportional(11.0), TEXT);
            painter.text(info.left_bottom(), Align2::LEFT_BOTTOM, def.src.short, FontId::proportional(9.5), TEXT_DIM);
        }
        LibState::Missing => {
            let r = Rect::from_min_size(info.left_top(), vec2(230.0, 26.0));
            let resp = ui.interact(r, id.with("get"), Sense::click());
            painter.rect_filled(r, 5.0, if resp.hovered() { accent } else { accent.gamma_multiply(0.75) });
            painter.text(r.center(), Align2::CENTER_CENTER, format!("Download {} ({} MB)", def.name, def.mb), FontId::proportional(12.0), TEXT_DARK);
            painter.text(pos2(info.left(), r.bottom() + 4.0), Align2::LEFT_TOP, "free recordings, once; then it plays at once", FontId::proportional(9.5), TEXT_DIM);
            if resp.clicked() {
                install = true;
            }
        }
        LibState::Installing(what, frac) => {
            painter.text(info.left_top(), Align2::LEFT_TOP, what, FontId::proportional(11.0), TEXT);
            let bar = Rect::from_min_size(info.left_top() + vec2(0.0, 18.0), vec2(info.width(), 6.0));
            painter.rect_filled(bar, 3.0, FAINT);
            painter.rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * frac.clamp(0.0, 1.0), bar.height())), 3.0, accent);
            ui.ctx().request_repaint();
        }
        LibState::Failed(e) => {
            let r = Rect::from_min_size(info.left_top(), vec2(120.0, 22.0));
            let resp = ui.interact(r, id.with("retry"), Sense::click());
            painter.rect_filled(r, 5.0, if resp.hovered() { accent } else { FAINT });
            painter.text(r.center(), Align2::CENTER_CENTER, "Try again", FontId::proportional(11.0), if resp.hovered() { TEXT_DARK } else { TEXT });
            painter.text(pos2(info.left(), r.bottom() + 4.0), Align2::LEFT_TOP, e, FontId::proportional(9.5), REC);
            if resp.clicked() {
                install = true;
            }
        }
    }
    if live.vals[2] > 0.0 {
        ui.ctx().request_repaint();
    }
    (changed, install)
}

const YELLOW_DYN: Color32 = Color32::from_rgb(0xff, 0xd4, 0x3b);

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, RawInput};

    #[test]
    fn the_orchestra_screen_draws_in_every_state() {
        let ctx = egui::Context::default();
        let live = InstLive { wave: vec![0.0; 64], notes: 1 << 60, vals: [0.5; krabton_engine::engine::INST_VALS] };
        for state in [LibState::Ready, LibState::Missing, LibState::Installing("Downloading 3 of 40".into(), 0.3), LibState::Failed("downloading failed".into())] {
            for name in orchestra::NAMES {
                let mut p = orchestra::preset(name);
                let _ = ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() }, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        orchestra_screen(ui, Id::new("o"), &mut p, &live, 200.0, state.clone());
                    });
                });
            }
        }
    }
}
