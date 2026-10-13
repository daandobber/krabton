//! Big Picture's Browse page: what you can add, in three columns: the kind
//! (instruments, drum kits, sequencers, MIDI effects, audio effects), the
//! thing, and for instruments their presets. A loads it onto the selected
//! track (an instrument replaces the one there), X onto a new track.

use super::input::{Frame, Nav};
use super::{fit_text, tile, Glyph, Sz, TILE, TILE_HI};
use crate::app::{App, BrowserItem, DropTarget, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Id, Rect, Sense, Ui};
use krabton_engine::midi_fx::MidiFxKind;

const KINDS: [&str; 5] = ["Instruments", "Drum Kits", "Sequencers", "MIDI Effects", "Audio Effects"];

#[derive(Default)]
pub struct BrowseState {
    pub col: usize,
    pub kind: usize,
    pub item: usize,
    pub preset: usize,
}

struct Entry {
    name: String,
    sub: String,
    item: Option<BrowserItem>,
    presets: Option<(&'static [&'static str], fn(&'static str) -> BrowserItem)>,
}

fn entries(kind: usize) -> Vec<Entry> {
    match kind {
        0 => {
            let mut v: Vec<Entry> = crate::ui::browser::instrument_catalog()
                .into_iter()
                .map(|(name, hint, presets, make)| Entry { name: name.to_string(), sub: hint.to_string(), item: presets.first().map(|p| make(p)), presets: Some((presets, make)) })
                .collect();
            v.push(Entry { name: "Sampler".into(), sub: "Plays a sample across the keys (load one in the desktop view)".into(), item: Some(BrowserItem::Sampler), presets: None });
            v
        }
        1 => krabton_engine::dsp::drums::kit_names().into_iter().map(|k| Entry { name: k.to_string(), sub: "Drum kit".into(), item: Some(BrowserItem::Drums(k)), presets: None }).collect(),
        2 => MidiFxKind::SEQUENCERS.iter().map(|k| Entry { name: k.name().to_string(), sub: format!("{} · {}", k.group(), k.description()), item: Some(BrowserItem::MidiFx(*k)), presets: None }).collect(),
        3 => MidiFxKind::ALL.iter().map(|k| Entry { name: k.name().to_string(), sub: k.description().to_string(), item: Some(BrowserItem::MidiFx(*k)), presets: None }).collect(),
        _ => krabton_engine::dsp::effects::EffectKind::ALL.iter().map(|k| Entry { name: k.name().to_string(), sub: k.category().to_string(), item: Some(BrowserItem::Effect(*k)), presets: None }).collect(),
    }
}

/// A list of rows that keeps `cur` in view and scrolls with a finger or
/// the wheel: (the row tapped, where the focus is after scrolling).
#[allow(clippy::too_many_arguments)]
fn list(ui: &mut Ui, id: &str, r: Rect, s: &Sz, rows: &[(String, String)], cur: usize, focused: bool, chosen: bool) -> (Option<usize>, usize) {
    let p = ui.painter().clone();
    let row_h = s.u * 1.3;
    let shown = ((r.height() / row_h).floor() as usize).max(1);
    let max_first = rows.len().saturating_sub(shown);
    let first_id = Id::new((id, "first"));
    let mut first = ui.ctx().memory(|m| m.data.get_temp::<usize>(first_id).unwrap_or(0)).min(max_first);
    let mut cur = cur;
    if cur < first {
        first = cur;
    } else if cur >= first + shown {
        first = cur + 1 - shown;
    }
    let scroll = ui.interact(r, Id::new((id, "scroll")), Sense::drag());
    let k = super::scrolled_rows(ui, &scroll, r, row_h, Id::new((id, "acc")));
    if k != 0 {
        first = (first as i64 + k).clamp(0, max_first as i64) as usize;
        cur = cur.clamp(first, (first + shown).min(rows.len()).saturating_sub(1).max(first));
    }
    ui.ctx().memory_mut(|m| m.data.insert_temp(first_id, first));
    let mut tapped = None;
    for (k, i) in (first..rows.len().min(first + shown)).enumerate() {
        let rr = Rect::from_min_size(pos2(r.left(), r.top() + k as f32 * row_h), vec2(r.width(), row_h - s.u * 0.12));
        let resp = ui.interact(rr, Id::new((id, i)), Sense::click());
        if resp.clicked() {
            tapped = Some(i);
        }
        let on = i == cur;
        tile(&p, rr, if on && (focused || chosen) { TILE_HI } else { TILE }, on && focused);
        let (name, sub) = &rows[i];
        if sub.is_empty() {
            fit_text(&p, pos2(rr.left() + s.u * 0.35, rr.center().y), Align2::LEFT_CENTER, name, s.font, rr.width() - s.u * 0.6, TEXT);
        } else {
            fit_text(&p, pos2(rr.left() + s.u * 0.35, rr.top() + rr.height() * 0.36), Align2::LEFT_CENTER, name, s.font, rr.width() - s.u * 0.6, TEXT);
            fit_text(&p, pos2(rr.left() + s.u * 0.35, rr.top() + rr.height() * 0.74), Align2::LEFT_CENTER, sub, s.small, rr.width() - s.u * 0.6, TEXT_DIM);
        }
    }
    if rows.len() > shown {
        let frac = first as f32 / (rows.len() - shown) as f32;
        let bar_h = r.height() * shown as f32 / rows.len() as f32;
        let y = r.top() + (r.height() - bar_h) * frac;
        rect_fill(&p, Rect::from_min_size(pos2(r.right() + 2.0, y), vec2(3.0, bar_h)), 2.0, LINE);
    }
    (tapped, cur)
}

impl App {
    fn load_item(&mut self, item: &BrowserItem, new_track: bool) {
        let target = match (new_track, self.sel_track) {
            (false, TrackSel::Track(t)) if t < self.project.tracks.len() => DropTarget::Track(t),
            _ => DropTarget::NewTrack,
        };
        let before = self.project.tracks.len();
        self.apply_item(item, target);
        let t = match self.sel_track {
            TrackSel::Track(t) => t,
            _ => 0,
        };
        if self.project.tracks.len() > before {
            // A new track: it is the one to play and edit now.
            let t = self.project.tracks.len() - 1;
            self.sel_track = TrackSel::Track(t);
            self.big.clip_cursor.0 = t;
        }
        let name = self.project.tracks.get(t).map(|tr| tr.name.clone()).unwrap_or_default();
        self.toast(format!("Loaded on {name}"));
    }

    pub(crate) fn big_browse(&mut self, ui: &mut Ui, r: Rect, s: &Sz, mut inp: Frame) -> Vec<(Glyph, &'static str)> {
        let mut st = std::mem::take(&mut self.big.browse);
        st.kind = st.kind.min(KINDS.len() - 1);
        let list_items = entries(st.kind);
        st.item = st.item.min(list_items.len().saturating_sub(1));
        let presets = list_items.get(st.item).and_then(|e| e.presets);
        let n_presets = presets.map(|p| p.0.len()).unwrap_or(0);
        st.preset = st.preset.min(n_presets.saturating_sub(1));
        let mut load: Option<(BrowserItem, bool)> = None;
        for nav in inp.pressed.drain(..) {
            match (st.col, nav) {
                (0, Nav::Up) => st.kind = st.kind.saturating_sub(1),
                (0, Nav::Down) => st.kind = (st.kind + 1).min(KINDS.len() - 1),
                (0, Nav::Right | Nav::A) => st.col = 1,
                (1, Nav::Up) => st.item = st.item.saturating_sub(1),
                (1, Nav::Down) => st.item = (st.item + 1).min(list_items.len().saturating_sub(1)),
                (1, Nav::Left | Nav::B) => st.col = 0,
                (1, Nav::Right) if n_presets > 0 => st.col = 2,
                (1, Nav::A) if n_presets > 0 => st.col = 2,
                (1, Nav::A | Nav::X) => {
                    if let Some(item) = list_items.get(st.item).and_then(|e| e.item.clone()) {
                        load = Some((item, nav == Nav::X));
                    }
                }
                (2, Nav::Up) => st.preset = st.preset.saturating_sub(1),
                (2, Nav::Down) => st.preset = (st.preset + 1).min(n_presets.saturating_sub(1)),
                (2, Nav::Left | Nav::B) => st.col = 1,
                (2, Nav::A | Nav::X) => {
                    if let Some((names, make)) = presets {
                        load = Some((make(names[st.preset]), nav == Nav::X));
                    }
                }
                _ => {}
            }
        }
        // Where it goes.
        let p = ui.painter().clone();
        let target = match self.sel_track {
            TrackSel::Track(t) => self.project.tracks.get(t).map(|tr| tr.name.clone()),
            _ => None,
        };
        let head = Rect::from_min_size(r.min, vec2(r.width(), s.u * 1.1));
        let here = Rect::from_min_size(pos2(head.right() - s.u * 10.4, head.top()), vec2(s.u * 5.0, head.height() - s.u * 0.1));
        let new = Rect::from_min_size(pos2(head.right() - s.u * 5.2, head.top()), vec2(s.u * 5.2, head.height() - s.u * 0.1));
        fit_text(&p, pos2(head.left(), head.center().y), Align2::LEFT_CENTER, &format!("Loads onto: {}", target.clone().unwrap_or_else(|| "a new track".into())), s.font, here.left() - head.left() - s.u * 0.3, TEXT_DIM);
        for (br, label, new_track) in [(here, "A  Load here", false), (new, "X  New track", true)] {
            let resp = ui.interact(br, Id::new(("big_load", new_track)), Sense::click());
            rect_fill(&p, br, 8.0, if resp.hovered() { TILE_HI } else { TILE });
            fit_text(&p, br.center(), Align2::CENTER_CENTER, label, s.font, br.width() - 8.0, TEXT);
            if resp.clicked() {
                let item = if st.col == 2 { presets.map(|(names, make)| make(names[st.preset])) } else { list_items.get(st.item).and_then(|e| e.item.clone()) };
                if let Some(item) = item {
                    load = Some((item, new_track));
                }
            }
        }
        // Three columns.
        let body = Rect::from_min_max(pos2(r.left(), head.bottom() + s.u * 0.25), r.max);
        let gap = s.u * 0.4;
        let w0 = (body.width() * 0.22).clamp(s.u * 5.0, s.u * 8.0);
        let w2 = if n_presets > 0 { (body.width() * 0.3).max(s.u * 6.0) } else { 0.0 };
        let w1 = body.width() - w0 - w2 - gap * 2.0;
        let c0 = Rect::from_min_size(body.min, vec2(w0, body.height()));
        let c1 = Rect::from_min_size(pos2(c0.right() + gap, body.top()), vec2(w1, body.height()));
        let c2 = Rect::from_min_size(pos2(c1.right() + gap, body.top()), vec2(w2, body.height()));
        let kinds: Vec<(String, String)> = KINDS.iter().map(|k| (k.to_string(), String::new())).collect();
        let (tap, kind) = list(ui, "big_kind", c0, s, &kinds, st.kind, st.col == 0, true);
        if kind != st.kind {
            st.kind = kind;
            st.item = 0;
        }
        if let Some(i) = tap {
            st.kind = i;
            st.col = 0;
            st.item = 0;
        }
        let rows: Vec<(String, String)> = list_items.iter().map(|e| (e.name.clone(), e.sub.clone())).collect();
        let (tap, item) = list(ui, &format!("big_item{}", st.kind), c1, s, &rows, st.item, st.col == 1, st.col == 2);
        if item != st.item {
            st.item = item;
            st.preset = 0;
        }
        if let Some(i) = tap {
            st.item = i;
            st.col = 1;
            st.preset = 0;
        }
        if let Some((names, _)) = presets {
            let prow: Vec<(String, String)> = names.iter().map(|n| (n.to_string(), String::new())).collect();
            let (tap, preset) = list(ui, "big_preset", c2, s, &prow, st.preset, st.col == 2, false);
            st.preset = preset;
            if let Some(i) = tap {
                st.preset = i;
                st.col = 2;
            }
        }
        let hints = match st.col {
            0 => vec![(Glyph::Dpad, "Move"), (Glyph::Btn(Nav::A), "Open")],
            1 if n_presets > 0 => vec![(Glyph::Dpad, "Move"), (Glyph::Btn(Nav::A), "Presets"), (Glyph::Btn(Nav::X), "New track"), (Glyph::Btn(Nav::B), "Back")],
            _ => vec![(Glyph::Dpad, "Move"), (Glyph::Btn(Nav::A), "Load"), (Glyph::Btn(Nav::X), "New track"), (Glyph::Btn(Nav::B), "Back")],
        };
        self.big.browse = st;
        if let Some((item, new_track)) = load {
            self.load_item(&item, new_track);
        }
        hints
    }
}
