//! Big Picture's Rack: every track at once, a row per track. The row starts
//! with the track's mixer (volume with its meter, pan, sends, mute, solo,
//! arm), then a card per device with its live screen (the one it has on the
//! desktop, or its sound and notes) and its first four knobs. Move between
//! the cards with the D-pad; A goes into one (up and down choose, left and
//! right or the triggers turn, B comes out), X turns a device on or off (or
//! mutes the track), Y opens all its knobs on the Devices page (or the
//! Mixer). A finger drags the knobs and faders.

use super::devices::{step_value, Link};
use super::input::{Frame, Nav};
use super::mixer::Ctl;
use super::{fit_text, tile, value_bar, Glyph, Page, Sz, TILE, TILE_HI};
use crate::app::{App, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Ui};
use krabton_engine::params::ParamKind;

const KNOBS: usize = 4;

#[derive(Default)]
pub struct RackState {
    pub track: usize,
    /// 0: the mixer, then the devices.
    pub card: usize,
    /// In a card: turning its knobs.
    pub inside: bool,
    pub knob: usize,
    pub first_track: usize,
    pub first_card: usize,
    pub turn_acc: f32,
}

impl App {
    /// The mixer's controls as the Rack shows them (two sends at most).
    fn rack_mix_controls(&self, t: usize) -> Vec<Ctl> {
        self.strip_controls(t).into_iter().filter(|c| !matches!(c, Ctl::Send(k) if *k >= 2)).collect()
    }

    /// The device behind card `card` of track `t` (card 0 is the mixer).
    fn rack_link(chains: &[Vec<(Link, String, String, bool)>], t: usize, card: usize) -> Option<Link> {
        if card == 0 {
            None
        } else {
            chains[t].get(card - 1).map(|c| c.0)
        }
    }

    /// Turns the focused knob of a card by `amount` of its range (stepped
    /// ones a step at a time as `acc` fills up).
    fn rack_turn(&mut self, t: usize, link: Option<Link>, knob: usize, amount: f32, acc: &mut f32) {
        match link {
            Some(l) => {
                let specs = self.link_specs(t, l);
                let Some(spec) = specs.get(knob) else { return };
                let Some(v) = self.link_value(t, l, knob) else { return };
                if spec.is_discrete() {
                    *acc += amount * 10.0;
                    let k = acc.trunc();
                    if k != 0.0 {
                        *acc -= k;
                        if !matches!(spec.kind, ParamKind::Toggle) {
                            self.set_link_value(t, l, knob, (v.round() + k).clamp(spec.min, spec.max));
                        }
                    }
                } else {
                    self.set_link_value(t, l, knob, spec.from_norm(spec.to_norm(v) + amount));
                }
            }
            None => {
                if let Some(c) = self.rack_mix_controls(t).get(knob) {
                    self.turn_control(t, *c, amount);
                }
            }
        }
    }

    pub(crate) fn big_rack(&mut self, ui: &mut Ui, r: Rect, s: &Sz, mut inp: Frame) -> Vec<(Glyph, &'static str)> {
        let n_tracks = self.project.tracks.len();
        if n_tracks == 0 {
            return Vec::new();
        }
        let chains: Vec<Vec<(Link, String, String, bool)>> = (0..n_tracks).map(|t| self.chain(t)).collect();
        let mut st = std::mem::take(&mut self.big.rack);
        st.track = st.track.min(n_tracks - 1);
        let cards = |t: usize| chains[t].len() + 1;
        st.card = st.card.min(cards(st.track) - 1);
        let link = Self::rack_link(&chains, st.track, st.card);
        let mix_ctls = self.rack_mix_controls(st.track);
        let n_knobs = match link {
            Some(l) => self.link_specs(st.track, l).len().min(KNOBS),
            None => mix_ctls.len(),
        };
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
                (false, Nav::Right) => st.card = (st.card + 1).min(cards(st.track) - 1),
                (false, Nav::A) if n_knobs > 0 => {
                    st.inside = true;
                    st.knob = 0;
                }
                (true, Nav::Up) => st.knob = st.knob.saturating_sub(1),
                (true, Nav::Down) => st.knob = (st.knob + 1).min(n_knobs.saturating_sub(1)),
                (true, Nav::B) => st.inside = false,
                (true, Nav::Left | Nav::Right | Nav::A) => {
                    let step = if nav == Nav::Left { -1 } else { 1 };
                    match link {
                        Some(l) => {
                            let spec = &self.link_specs(st.track, l)[st.knob];
                            if nav != Nav::A || matches!(spec.kind, ParamKind::Toggle) {
                                if let Some(v) = self.link_value(st.track, l, st.knob) {
                                    self.set_link_value(st.track, l, st.knob, step_value(spec, v, step));
                                }
                            }
                        }
                        None => {
                            let c = mix_ctls[st.knob];
                            match (c, nav) {
                                (Ctl::Mute | Ctl::Solo | Ctl::Arm, _) | (_, Nav::A) => self.press_control(st.track, c),
                                _ => self.turn_control(st.track, c, step as f32 * 0.025),
                            }
                        }
                    }
                }
                (_, Nav::X) => match link {
                    Some(l) => self.toggle_link(st.track, l),
                    None => self.press_control(st.track, Ctl::Mute),
                },
                (_, Nav::Y) => open_all = true,
                _ => {}
            }
        }
        st.card = st.card.min(cards(st.track) - 1);
        // The triggers (or the right stick) turn the knob with the focus.
        if st.inside && inp.turn.abs() > 0.01 {
            let link = Self::rack_link(&chains, st.track, st.card);
            let mut acc = st.turn_acc;
            self.rack_turn(st.track, link, st.knob, inp.turn * inp.dt * 0.6, &mut acc);
            st.turn_acc = acc;
        }

        // Layout: the mixer, then wide cards with the screen on the left.
        let p = ui.painter().clone();
        let mix_w = (s.u * 5.0).min(r.width() * 0.3);
        let vis_rows = ((r.height() / (s.u * 4.6)).floor() as usize).clamp(1, n_tracks);
        let row_h = (r.height() / vis_rows as f32).min(s.u * 6.6);
        let card_w = (s.u * 10.5).min(r.width() - mix_w);
        let gap = s.u * 0.2;
        let vis_cards = (((r.width() - mix_w) / card_w).floor() as usize).max(1);
        // Screens draw over the page: not while a menu is open.
        let screens = self.big.menu.is_none() && self.big.clip_menu.is_none();
        if st.track < st.first_track {
            st.first_track = st.track;
        } else if st.track >= st.first_track + vis_rows {
            st.first_track = st.track + 1 - vis_rows;
        }
        // The device cards scroll; the mixer stays.
        if st.card > 0 {
            let dev_card = st.card - 1;
            if dev_card < st.first_card {
                st.first_card = dev_card;
            } else if dev_card >= st.first_card + vis_cards {
                st.first_card = dev_card + 1 - vis_cards;
            }
        }
        let max_cards = chains.iter().map(|c| c.len()).max().unwrap_or(0);
        st.first_card = st.first_card.min(max_cards.saturating_sub(vis_cards));
        st.first_track = st.first_track.min(n_tracks.saturating_sub(vis_rows));
        let mut tapped: Option<(usize, usize)> = None;
        let small = Sz { font: s.small, ..*s };
        for (k, t) in (st.first_track..(st.first_track + vis_rows).min(n_tracks)).enumerate() {
            let y = r.top() + k as f32 * row_h;
            let color = rgb(self.project.tracks[t].color);

            // ---------------------------------------------------------------- the mixer
            let mr = Rect::from_min_size(pos2(r.left(), y), vec2(mix_w - gap, row_h - gap));
            let focus_mix = t == st.track && st.card == 0;
            tile(&p, mr, shade(color, if t == st.track { 0.42 } else { 0.28 }), focus_mix && !st.inside);
            let name = self.project.tracks[t].name.clone();
            let nresp = ui.interact(Rect::from_min_size(mr.min, vec2(mr.width(), s.u * 0.85)), Id::new(("big_rack_mixname", t)), Sense::click());
            if nresp.clicked() {
                tapped = Some((t, 0));
            }
            fit_text(&p, pos2(mr.left() + s.u * 0.3, mr.top() + s.u * 0.45), Align2::LEFT_CENTER, &name, s.font, mr.width() - s.u * 0.5, TEXT);
            let ctls = self.rack_mix_controls(t);
            let faders: Vec<Ctl> = ctls.iter().copied().filter(|c| matches!(c, Ctl::Volume | Ctl::Pan | Ctl::Send(_))).collect();
            let body_top = mr.top() + s.u * 0.9;
            let btn_h = (s.u * 0.75).min((mr.bottom() - body_top) * 0.3);
            let fader_h = ((mr.bottom() - body_top - btn_h - s.u * 0.25) / faders.len().max(1) as f32).min(s.u * 0.6);
            let meter = self.meters.get(t).copied().unwrap_or((0.0, 0.0));
            for (q, c) in faders.iter().enumerate() {
                let fr = Rect::from_min_size(pos2(mr.left() + s.u * 0.2, body_top + q as f32 * fader_h), vec2(mr.width() - s.u * 0.4, fader_h - s.u * 0.08));
                let ci = ctls.iter().position(|x| x == c).unwrap_or(0);
                let knob_focus = focus_mix && st.inside && st.knob == ci;
                let mixer = &self.project.tracks[t].mixer;
                let (label, norm, bipolar) = match c {
                    Ctl::Volume => (format!("{:+.1} dB", mixer.volume_db), db_to_fader(mixer.volume_db), false),
                    Ctl::Pan => {
                        let pan = mixer.pan;
                        let txt = if pan.abs() < 0.01 {
                            "C".to_string()
                        } else if pan < 0.0 {
                            format!("{:.0}L", -pan * 50.0)
                        } else {
                            format!("{:.0}R", pan * 50.0)
                        };
                        (txt, pan * 0.5 + 0.5, true)
                    }
                    Ctl::Send(k) => {
                        let v = mixer.sends.get(*k).copied().unwrap_or(0.0);
                        (format!("{} {:.0}%", (b'A' + *k as u8) as char, v * 100.0), v, false)
                    }
                    _ => (String::new(), 0.0, false),
                };
                value_bar(&p, fr, norm, bipolar, &label, &small, color);
                if *c == Ctl::Volume {
                    // The meter along the volume.
                    let lvl = meter.0.max(meter.1).clamp(0.0, 1.0).sqrt();
                    p.rect_filled(Rect::from_min_size(pos2(fr.left(), fr.bottom() - 3.0), vec2(fr.width() * lvl, 3.0)), 1.0, if lvl >= 0.995 { REC } else { PLAY });
                }
                if knob_focus {
                    rect_line(&p, fr.expand(1.5), 6.0, 2.0, super::FOCUS);
                }
                let fresp = ui.interact(fr, Id::new(("big_rack_fader", t, q)), Sense::click_and_drag());
                if let Some(pt) = fresp.interact_pointer_pos().filter(|_| fresp.dragged() || fresp.clicked()) {
                    let n = ((pt.x - fr.left()) / fr.width()).clamp(0.0, 1.0);
                    match c {
                        Ctl::Volume => *self.volume_db(t) = fader_to_db(n),
                        Ctl::Pan => self.project.tracks[t].mixer.pan = n * 2.0 - 1.0,
                        Ctl::Send(k) => {
                            let m = &mut self.project.tracks[t].mixer;
                            if m.sends.len() <= *k {
                                m.sends.resize(*k + 1, 0.0);
                            }
                            m.sends[*k] = n;
                        }
                        _ => {}
                    }
                    st.track = t;
                    st.card = 0;
                    st.inside = true;
                    st.knob = ci;
                }
                if fresp.double_clicked() {
                    self.press_control(t, *c);
                }
            }
            // Mute, solo, arm.
            let (mute, solo, arm) = {
                let tr = &self.project.tracks[t];
                (tr.mixer.mute, tr.mixer.solo, tr.arm)
            };
            let bw = (mr.width() - s.u * 0.6) / 3.0;
            for (b, (c, label, on, on_col)) in [(Ctl::Mute, "M", mute, Color32::from_rgb(0xf2, 0xa0, 0x3c)), (Ctl::Solo, "S", solo, SOLO), (Ctl::Arm, "R", arm, REC)].into_iter().enumerate() {
                let br = Rect::from_min_size(pos2(mr.left() + s.u * 0.2 + b as f32 * (bw + s.u * 0.1), mr.bottom() - s.u * 0.15 - btn_h), vec2(bw, btn_h));
                let ci = ctls.iter().position(|x| *x == c).unwrap_or(0);
                tile(&p, br, if on { on_col } else { TILE_HI }, focus_mix && st.inside && st.knob == ci);
                text(&p, br.center(), Align2::CENTER_CENTER, label, s.s(), if on { TEXT_DARK } else { TEXT });
                if ui.interact(br, Id::new(("big_rack_btn", t, b)), Sense::click()).clicked() {
                    self.press_control(t, c);
                }
            }

            // ---------------------------------------------------------------- the devices
            if chains[t].is_empty() {
                text(&p, pos2(mr.right() + s.u * 0.4, y + row_h * 0.5), Align2::LEFT_CENTER, "No devices: Y adds one", s.f(), TEXT_DIM);
            }
            for (j, c) in (st.first_card..(st.first_card + vis_cards).min(chains[t].len())).enumerate() {
                let (l, title, sub, on) = &chains[t][c];
                let cr = Rect::from_min_size(pos2(r.left() + mix_w + j as f32 * card_w, y), vec2(card_w - gap, row_h - gap));
                let focus = t == st.track && st.card == c + 1;
                tile(&p, cr, if focus { TILE_HI } else { TILE }, focus && !st.inside);
                let cresp = ui.interact(Rect::from_min_size(cr.min, vec2(cr.width(), s.u * 0.95)), Id::new(("big_rack_card", t, c)), Sense::click());
                if cresp.clicked() {
                    tapped = Some((t, c + 1));
                }
                p.circle_filled(pos2(cr.left() + s.u * 0.35, cr.top() + s.u * 0.48), s.u * 0.14, if *on { color } else { LINE });
                fit_text(&p, pos2(cr.left() + s.u * 0.6, cr.top() + s.u * 0.32), Align2::LEFT_CENTER, title, s.font * 0.9, cr.width() - s.u * 0.8, if *on { TEXT } else { TEXT_DIM });
                fit_text(&p, pos2(cr.left() + s.u * 0.6, cr.top() + s.u * 0.72), Align2::LEFT_CENTER, sub, s.small * 0.9, cr.width() - s.u * 0.8, TEXT_DIM);
                // The live screen on the left, the knobs on the right.
                let body = Rect::from_min_max(pos2(cr.left() + s.u * 0.2, cr.top() + s.u * 1.0), pos2(cr.right() - s.u * 0.2, cr.bottom() - s.u * 0.15));
                let screen = Rect::from_min_size(body.min, vec2(body.width() * 0.56, body.height()));
                let knobs = Rect::from_min_max(pos2(screen.right() + s.u * 0.25, body.top()), body.max);
                if screens {
                    self.rack_screen(ui.ctx(), &p, (t, c), t, *l, screen, color);
                } else {
                    p.rect_filled(screen, 6.0, Color32::from_rgb(0x0a, 0x0b, 0x0e));
                }
                let specs = self.link_specs(t, *l);
                let knob_h = (knobs.height() / KNOBS as f32).min(s.u * 0.85);
                for (q, spec) in specs.iter().take(KNOBS).enumerate() {
                    let kr = Rect::from_min_size(pos2(knobs.left(), knobs.top() + q as f32 * knob_h), vec2(knobs.width(), knob_h - s.u * 0.08));
                    let v = self.link_value(t, *l, q).unwrap_or(spec.default);
                    let knob_focus = focus && st.inside && q == st.knob;
                    // The name above its bar.
                    let name_h = kr.height() * 0.42;
                    fit_text(&p, pos2(kr.left(), kr.top() + name_h * 0.5), Align2::LEFT_CENTER, spec.name, s.small * 0.9, kr.width(), if knob_focus { super::FOCUS } else { TEXT_DIM });
                    let bar = Rect::from_min_max(pos2(kr.left(), kr.top() + name_h), kr.max);
                    let label = match spec.kind {
                        ParamKind::Choice(opts) => opts.get(v.round().max(0.0) as usize).copied().unwrap_or("?").to_string(),
                        ParamKind::Toggle => {
                            if v >= 0.5 {
                                "On".to_string()
                            } else {
                                "Off".to_string()
                            }
                        }
                        _ => spec.format(v),
                    };
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
                        st.card = c + 1;
                        st.inside = true;
                        st.knob = q;
                    }
                }
                if specs.is_empty() {
                    let what = if matches!(l, Link::Instrument) && matches!(self.project.tracks[t].instrument, Some(krabton_engine::Instrument::Drums { .. })) { "Pads: on the desktop view" } else { "Its own window: on the desktop view" };
                    fit_text(&p, knobs.center(), Align2::CENTER_CENTER, what, s.small, knobs.width(), TEXT_DIM);
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
            if card == 0 {
                self.big.mix.strip = t;
                self.big.page = Page::Mixer;
            } else {
                self.big.dev.link = card - 1;
                self.big.dev.col = 1;
                self.big.dev.row = 0;
                self.big.page = Page::Devices;
            }
        }
        let mixer = card == 0;
        if inside {
            vec![(Glyph::Dpad, "Choose, change"), (Glyph::Triggers, "Turn"), (Glyph::Btn(Nav::B), "Out"), (Glyph::Btn(Nav::X), if mixer { "Mute" } else { "On/Off" }), (Glyph::Btn(Nav::Y), if mixer { "Mixer page" } else { "All knobs" })]
        } else {
            vec![(Glyph::Dpad, "Move"), (Glyph::Btn(Nav::A), if mixer { "Mixer" } else { "Knobs" }), (Glyph::Btn(Nav::X), if mixer { "Mute" } else { "On/Off" }), (Glyph::Btn(Nav::Y), if mixer { "Mixer page" } else { "All knobs" })]
        }
    }
}
