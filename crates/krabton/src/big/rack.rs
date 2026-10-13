//! Big Picture's Rack: every track's devices at once, a row per track and a
//! card per device with its first four knobs. Move between the cards with
//! the D-pad; A goes into a card (up and down choose a knob, left and right
//! or the triggers turn it, B comes out), X turns a device on or off, Y
//! opens all its knobs on the Devices page. A finger drags the knobs.

use super::devices::{step_value, Link};
use super::input::{Frame, Nav};
use super::{fit_text, tile, value_bar, Glyph, Page, Sz, TILE, TILE_HI};
use crate::app::{App, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Id, Rect, Sense, Ui};
use krabton_engine::params::ParamKind;

const KNOBS: usize = 4;

#[derive(Default)]
pub struct RackState {
    pub track: usize,
    pub card: usize,
    /// In a card: turning its knobs.
    pub inside: bool,
    pub knob: usize,
    pub first_track: usize,
    pub first_card: usize,
    pub turn_acc: f32,
}

impl App {
    pub(crate) fn big_rack(&mut self, ui: &mut Ui, r: Rect, s: &Sz, mut inp: Frame) -> Vec<(Glyph, &'static str)> {
        let n_tracks = self.project.tracks.len();
        if n_tracks == 0 {
            return Vec::new();
        }
        let chains: Vec<Vec<(Link, String, String, bool)>> = (0..n_tracks).map(|t| self.chain(t)).collect();
        let mut st = std::mem::take(&mut self.big.rack);
        st.track = st.track.min(n_tracks - 1);
        let cards = |t: usize| chains[t].len();
        st.card = st.card.min(cards(st.track).saturating_sub(1));
        let link = chains[st.track].get(st.card).map(|c| c.0);
        let n_knobs = link.map(|l| self.link_specs(st.track, l).len().min(KNOBS)).unwrap_or(0);
        if n_knobs == 0 {
            st.inside = false;
        }
        st.knob = st.knob.min(n_knobs.saturating_sub(1));
        let mut open_all = false;
        for nav in inp.pressed.drain(..) {
            match (st.inside, nav) {
                (false, Nav::Up) => st.track = st.track.saturating_sub(1),
                (false, Nav::Down) => st.track = (st.track + 1).min(n_tracks - 1),
                (false, Nav::Left) => st.card = st.card.saturating_sub(1),
                (false, Nav::Right) => st.card = (st.card + 1).min(cards(st.track).saturating_sub(1)),
                (false, Nav::A) if n_knobs > 0 => {
                    st.inside = true;
                    st.knob = 0;
                }
                (true, Nav::Up) => st.knob = st.knob.saturating_sub(1),
                (true, Nav::Down) => st.knob = (st.knob + 1).min(n_knobs.saturating_sub(1)),
                (true, Nav::B) => st.inside = false,
                (true, Nav::Left | Nav::Right | Nav::A) => {
                    if let Some(l) = link {
                        let spec = &self.link_specs(st.track, l)[st.knob];
                        if nav != Nav::A || matches!(spec.kind, ParamKind::Toggle) {
                            if let Some(v) = self.link_value(st.track, l, st.knob) {
                                self.set_link_value(st.track, l, st.knob, step_value(spec, v, if nav == Nav::Left { -1 } else { 1 }));
                            }
                        }
                    }
                }
                (_, Nav::X) => {
                    if let Some(l) = link {
                        self.toggle_link(st.track, l);
                    }
                }
                (_, Nav::Y) => open_all = true,
                _ => {}
            }
        }
        st.card = st.card.min(cards(st.track).saturating_sub(1));
        // The triggers turn the knob with the focus.
        if st.inside && inp.turn.abs() > 0.01 {
            if let Some(l) = chains[st.track].get(st.card).map(|c| c.0) {
                let spec = &self.link_specs(st.track, l)[st.knob.min(n_knobs.saturating_sub(1))];
                if let Some(v) = self.link_value(st.track, l, st.knob) {
                    if spec.is_discrete() {
                        st.turn_acc += inp.turn * inp.dt * 6.0;
                        let k = st.turn_acc.trunc();
                        if k != 0.0 {
                            st.turn_acc -= k;
                            if !matches!(spec.kind, ParamKind::Toggle) {
                                self.set_link_value(st.track, l, st.knob, (v.round() + k).clamp(spec.min, spec.max));
                            }
                        }
                    } else {
                        self.set_link_value(st.track, l, st.knob, spec.from_norm(spec.to_norm(v) + inp.turn * inp.dt * 0.6));
                    }
                }
            }
        }

        // Layout.
        let p = ui.painter().clone();
        let head_w = (s.u * 4.2).min(r.width() * 0.25);
        let vis_rows = ((r.height() / (s.u * 3.7)).floor() as usize).clamp(1, n_tracks);
        // The rows fill the height.
        let row_h = (r.height() / vis_rows as f32).min(s.u * 4.6);
        let card_w = s.u * 6.0;
        let gap = s.u * 0.2;
        let vis_cards = (((r.width() - head_w) / card_w).floor() as usize).max(1);
        if st.track < st.first_track {
            st.first_track = st.track;
        } else if st.track >= st.first_track + vis_rows {
            st.first_track = st.track + 1 - vis_rows;
        }
        if st.card < st.first_card {
            st.first_card = st.card;
        } else if st.card >= st.first_card + vis_cards {
            st.first_card = st.card + 1 - vis_cards;
        }
        let max_cards = chains.iter().map(|c| c.len()).max().unwrap_or(0);
        st.first_card = st.first_card.min(max_cards.saturating_sub(vis_cards));
        st.first_track = st.first_track.min(n_tracks.saturating_sub(vis_rows));
        let mut tapped: Option<(usize, usize)> = None;
        for (k, t) in (st.first_track..(st.first_track + vis_rows).min(n_tracks)).enumerate() {
            let y = r.top() + k as f32 * row_h;
            let track = &self.project.tracks[t];
            let color = rgb(track.color);
            let name = track.name.clone();
            // The track.
            let hr = Rect::from_min_size(pos2(r.left(), y), vec2(head_w - gap, row_h - gap));
            rect_fill(&p, hr, 8.0, shade(color, if t == st.track { 0.5 } else { 0.3 }));
            fit_text(&p, pos2(hr.left() + s.u * 0.3, hr.top() + s.u * 0.55), Align2::LEFT_CENTER, &name, s.font, hr.width() - s.u * 0.5, TEXT);
            let hresp = ui.interact(hr, Id::new(("big_rack_head", t)), Sense::click());
            if hresp.clicked() {
                tapped = Some((t, 0));
            }
            if chains[t].is_empty() {
                text(&p, pos2(hr.right() + s.u * 0.4, y + row_h * 0.5), Align2::LEFT_CENTER, "No devices: Y adds one", s.f(), TEXT_DIM);
            }
            // Its devices.
            for (j, c) in (st.first_card..(st.first_card + vis_cards).min(chains[t].len())).enumerate() {
                let (l, title, sub, on) = &chains[t][c];
                let cr = Rect::from_min_size(pos2(r.left() + head_w + j as f32 * card_w, y), vec2(card_w - gap, row_h - gap));
                let focus = t == st.track && c == st.card;
                tile(&p, cr, if focus { TILE_HI } else { TILE }, focus && !st.inside);
                let cresp = ui.interact(Rect::from_min_size(cr.min, vec2(cr.width(), s.u * 0.95)), Id::new(("big_rack_card", t, c)), Sense::click());
                if cresp.clicked() {
                    tapped = Some((t, c));
                }
                p.circle_filled(pos2(cr.left() + s.u * 0.35, cr.top() + s.u * 0.48), s.u * 0.14, if *on { color } else { LINE });
                fit_text(&p, pos2(cr.left() + s.u * 0.6, cr.top() + s.u * 0.32), Align2::LEFT_CENTER, title, s.font * 0.9, cr.width() - s.u * 0.8, if *on { TEXT } else { TEXT_DIM });
                fit_text(&p, pos2(cr.left() + s.u * 0.6, cr.top() + s.u * 0.72), Align2::LEFT_CENTER, sub, s.small * 0.9, cr.width() - s.u * 0.8, TEXT_DIM);
                let specs = self.link_specs(t, *l);
                let knob_h = (cr.height() - s.u * 1.05) / KNOBS as f32;
                for (q, spec) in specs.iter().take(KNOBS).enumerate() {
                    let kr = Rect::from_min_size(pos2(cr.left() + s.u * 0.2, cr.top() + s.u * 1.0 + q as f32 * knob_h), vec2(cr.width() - s.u * 0.4, knob_h - s.u * 0.08));
                    let v = self.link_value(t, *l, q).unwrap_or(spec.default);
                    let knob_focus = focus && st.inside && q == st.knob;
                    let name_w = kr.width() * 0.38;
                    fit_text(&p, pos2(kr.left(), kr.center().y), Align2::LEFT_CENTER, spec.name, s.small, name_w - 4.0, if knob_focus { super::FOCUS } else { TEXT_DIM });
                    let bar = Rect::from_min_max(pos2(kr.left() + name_w, kr.top()), kr.max);
                    let label = match spec.kind {
                        ParamKind::Choice(opts) => opts.get(v.round().max(0.0) as usize).copied().unwrap_or("?").to_string(),
                        ParamKind::Toggle => if v >= 0.5 { "On".to_string() } else { "Off".to_string() },
                        _ => spec.format(v),
                    };
                    let small = Sz { font: s.small, ..*s };
                    value_bar(&p, bar, spec.to_norm(v), spec.min < 0.0 && spec.max > 0.0, &label, &small, color);
                    if knob_focus {
                        rect_line(&p, bar.expand(1.5), 6.0, 2.0, super::FOCUS);
                    }
                    let kresp = ui.interact(bar, Id::new(("big_rack_knob", t, c, q)), Sense::click_and_drag());
                    if let Some(pt) = kresp.interact_pointer_pos().filter(|_| kresp.dragged() || kresp.clicked()) {
                        let nv = match spec.kind {
                            ParamKind::Choice(_) | ParamKind::Toggle if kresp.clicked() => step_value(spec, v, if pt.x < bar.center().x { -1 } else { 1 }),
                            ParamKind::Choice(_) | ParamKind::Toggle => v,
                            _ => {
                                let n = spec.from_norm(((pt.x - bar.left()) / bar.width()).clamp(0.0, 1.0));
                                if spec.is_discrete() {
                                    n.round()
                                } else {
                                    n
                                }
                            }
                        };
                        self.set_link_value(t, *l, q, nv);
                        st.track = t;
                        st.card = c;
                        st.inside = true;
                        st.knob = q;
                    }
                }
                if specs.is_empty() {
                    let what = if matches!(l, Link::Instrument) && matches!(self.project.tracks[t].instrument, Some(krabton_engine::Instrument::Drums { .. })) { "Pads: on the desktop view" } else { "Its own window: on the desktop view" };
                    fit_text(&p, cr.center() + vec2(0.0, s.u * 0.4), Align2::CENTER_CENTER, what, s.small, cr.width() - s.u * 0.4, TEXT_DIM);
                }
            }
            if chains[t].len() > st.first_card + vis_cards {
                text(&p, pos2(r.right() - s.u * 0.2, y + row_h * 0.5), Align2::RIGHT_CENTER, ">", s.big(), TEXT_DIM);
            }
        }
        if let Some((t, c)) = tapped {
            st.track = t;
            st.card = c;
            st.inside = false;
        }
        let (t, card, inside) = (st.track, st.card, st.inside);
        self.big.rack = st;
        self.sel_track = TrackSel::Track(t);
        if open_all {
            self.big.dev.link = card;
            self.big.dev.col = if chains[t].is_empty() { 0 } else { 1 };
            self.big.dev.row = 0;
            self.big.page = if chains[t].is_empty() { Page::Browse } else { Page::Devices };
        }
        if inside {
            vec![(Glyph::Dpad, "Knob, change"), (Glyph::Triggers, "Turn"), (Glyph::Btn(Nav::B), "Out"), (Glyph::Btn(Nav::X), "On/Off"), (Glyph::Btn(Nav::Y), "All knobs")]
        } else {
            vec![(Glyph::Dpad, "Move"), (Glyph::Btn(Nav::A), "Knobs"), (Glyph::Btn(Nav::X), "On/Off"), (Glyph::Btn(Nav::Y), "All knobs")]
        }
    }
}
