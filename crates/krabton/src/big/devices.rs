//! Big Picture's Devices page: the selected track's chain on the left (its
//! instrument, its sequencers and MIDI effects, its audio effects), and
//! every parameter of the chosen one on the right, a big row each. Move with
//! the D-pad; on a parameter left and right step it and the right stick turns
//! it smoothly; the triggers go to the device before or after; the first row
//! of an instrument changes its preset (a drum track's kit). X turns a device
//! on or off, Y adds one.

use super::input::{Frame, Nav};
use super::{fit_text, tile, value_bar, Glyph, Page, Sz, TILE, TILE_HI};
use crate::app::{App, BrowserItem, DropTarget, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Id, Rect, Sense, Ui};
use krabton_engine::params::{ParamKind, ParamSpec};
use krabton_engine::Instrument;

#[derive(Default)]
pub struct DevState {
    /// 0: the chain, 1: the parameters.
    pub col: usize,
    pub link: usize,
    pub row: usize,
    pub first_row: usize,
    /// Turning a stepped value: how far the stick has gone.
    pub turn_acc: f32,
    /// The triggers' last value (pulling one goes to the next device).
    pub last_trig: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Link {
    Instrument,
    Midi(usize),
    Fx(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    Preset,
    Param(usize),
}

/// The presets of an instrument (or a drum track's kits) and the one it has.
fn presets_of(inst: &Instrument) -> Option<(Vec<&'static str>, fn(&'static str) -> BrowserItem)> {
    if let Instrument::Drums { .. } = inst {
        return Some((krabton_engine::dsp::drums::kit_names(), BrowserItem::Drums));
    }
    let kind = inst.kind_name();
    crate::ui::browser::instrument_catalog().into_iter().find(|(name, _, presets, _)| *name == kind && !presets.is_empty()).map(|(_, _, presets, make)| (presets.to_vec(), make))
}

/// One step of a value: an option, a whole number, or a fortieth of the range.
pub(super) fn step_value(spec: &ParamSpec, v: f32, step: i32) -> f32 {
    match spec.kind {
        ParamKind::Toggle => {
            if v >= 0.5 {
                0.0
            } else {
                1.0
            }
        }
        _ if spec.is_discrete() => (v.round() + step as f32).clamp(spec.min, spec.max),
        _ => spec.from_norm(spec.to_norm(v) + step as f32 * 0.025),
    }
}

impl App {
    pub(super) fn chain(&self, t: usize) -> Vec<(Link, String, String, bool)> {
        let track = &self.project.tracks[t];
        let mut v = Vec::new();
        if let Some(i) = &track.instrument {
            v.push((Link::Instrument, i.kind_name().to_string(), i.display_name(), true));
        }
        for (k, d) in track.midi_devices.iter().enumerate() {
            let what = if d.kind.is_sequencer() { "Sequencer" } else { "MIDI effect" };
            v.push((Link::Midi(k), d.kind.name().to_string(), what.to_string(), d.enabled));
        }
        for (k, d) in track.devices.iter().enumerate() {
            v.push((Link::Fx(k), d.kind.name().to_string(), d.kind.category().to_string(), d.enabled));
        }
        v
    }

    pub(super) fn link_specs(&self, t: usize, link: Link) -> &'static [ParamSpec] {
        let track = &self.project.tracks[t];
        match link {
            Link::Instrument => track.instrument.as_ref().and_then(krabton_engine::modulation::instrument_specs).unwrap_or(&[]),
            Link::Midi(k) => track.midi_devices.get(k).map(|d| d.kind.specs()).unwrap_or(&[]),
            Link::Fx(k) => track.devices.get(k).map(|d| d.kind.specs()).unwrap_or(&[]),
        }
    }

    pub(super) fn link_value(&self, t: usize, link: Link, i: usize) -> Option<f32> {
        let track = &self.project.tracks[t];
        match link {
            Link::Instrument => track.instrument.as_ref().and_then(krabton_engine::modulation::instrument_params).and_then(|p| p.get(i).copied()),
            Link::Midi(k) => track.midi_devices.get(k).and_then(|d| d.params.get(i).copied()),
            Link::Fx(k) => track.devices.get(k).and_then(|d| d.params.get(i).copied()),
        }
    }

    pub(super) fn set_link_value(&mut self, t: usize, link: Link, i: usize, v: f32) {
        let track = &mut self.project.tracks[t];
        let slot = match link {
            Link::Instrument => track.instrument.as_mut().and_then(krabton_engine::modulation::instrument_params_mut).and_then(|p| p.get_mut(i)),
            Link::Midi(k) => track.midi_devices.get_mut(k).and_then(|d| d.params.get_mut(i)),
            Link::Fx(k) => track.devices.get_mut(k).and_then(|d| d.params.get_mut(i)),
        };
        if let Some(s) = slot {
            *s = v;
        }
    }

    pub(super) fn toggle_link(&mut self, t: usize, link: Link) {
        let track = &mut self.project.tracks[t];
        match link {
            Link::Midi(k) => {
                if let Some(d) = track.midi_devices.get_mut(k) {
                    d.enabled = !d.enabled;
                }
            }
            Link::Fx(k) => {
                if let Some(d) = track.devices.get_mut(k) {
                    d.enabled = !d.enabled;
                }
            }
            Link::Instrument => {}
        }
    }

    /// The next (or previous) preset of the track's instrument.
    fn step_preset(&mut self, t: usize, step: i32) {
        let Some(inst) = self.project.tracks[t].instrument.as_ref() else { return };
        let Some((presets, make)) = presets_of(inst) else { return };
        let now = inst.display_name();
        let i = presets.iter().position(|p| *p == now).map(|i| i as i32).unwrap_or(-1);
        let n = presets.len() as i32;
        let next = presets[((i + step).rem_euclid(n)) as usize];
        self.apply_item(&make(next), DropTarget::Track(t));
        self.sel_track = TrackSel::Track(t);
    }

    pub(crate) fn big_devices(&mut self, ui: &mut Ui, r: Rect, s: &Sz, mut inp: Frame) -> Vec<(Glyph, &'static str)> {
        let n_tracks = self.project.tracks.len();
        if n_tracks == 0 {
            return Vec::new();
        }
        let t = match self.sel_track {
            TrackSel::Track(t) if t < n_tracks => t,
            _ => 0,
        };
        let chain = self.chain(t);
        let p = ui.painter().clone();
        let accent = rgb(self.project.tracks[t].color);
        let st = &mut self.big.dev;
        st.link = st.link.min(chain.len().saturating_sub(1));
        let mut col = if chain.is_empty() { 0 } else { st.col };
        let mut link_i = st.link;
        let link = chain.get(link_i).map(|c| c.0);
        // The rows of the chosen device.
        let specs = link.map(|l| self.link_specs(t, l)).unwrap_or(&[]);
        let has_preset = link == Some(Link::Instrument) && self.project.tracks[t].instrument.as_ref().and_then(presets_of).is_some();
        let mut rows: Vec<Row> = Vec::new();
        if has_preset {
            rows.push(Row::Preset);
        }
        rows.extend((0..specs.len()).map(Row::Param));
        let mut row_i = self.big.dev.row.min(rows.len().saturating_sub(1));

        // Buttons.
        let mut add = false;
        for nav in inp.pressed.drain(..) {
            match (col, nav) {
                (0, Nav::Up) => link_i = link_i.saturating_sub(1),
                (0, Nav::Down) => link_i = (link_i + 1).min(chain.len().saturating_sub(1)),
                (0, Nav::Right | Nav::A) if !rows.is_empty() => col = 1,
                (0, Nav::X) => {
                    if let Some(l) = link {
                        self.toggle_link(t, l);
                    }
                }
                (_, Nav::Y) => add = true,
                (1, Nav::Up) => row_i = row_i.saturating_sub(1),
                (1, Nav::Down) => row_i = (row_i + 1).min(rows.len().saturating_sub(1)),
                (1, Nav::B) => col = 0,
                (1, Nav::Left | Nav::Right | Nav::A) => {
                    let step = if nav == Nav::Left { -1 } else { 1 };
                    match (rows.get(row_i), link) {
                        (Some(Row::Preset), _) if nav != Nav::A => self.step_preset(t, step),
                        (Some(Row::Param(i)), Some(l)) => {
                            let spec = &specs[*i];
                            let toggle = matches!(spec.kind, ParamKind::Toggle);
                            if nav != Nav::A || toggle {
                                if let Some(v) = self.link_value(t, l, *i) {
                                    self.set_link_value(t, l, *i, step_value(spec, v, step));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        // The triggers go to the device before or after.
        let trig = inp.trig;
        let last_trig = self.big.dev.last_trig;
        self.big.dev.last_trig = trig;
        if !chain.is_empty() {
            if trig > 0.6 && last_trig <= 0.6 {
                link_i = (link_i + 1).min(chain.len() - 1);
            } else if trig < -0.6 && last_trig >= -0.6 {
                link_i = link_i.saturating_sub(1);
            }
        }
        // The right stick turns the value.
        let turn = inp.stick_turn;
        if col == 1 && turn.abs() > 0.01 && link_i == self.big.dev.link {
            if let (Some(Row::Param(i)), Some(l)) = (rows.get(row_i), link) {
                let spec = &specs[*i];
                if let Some(v) = self.link_value(t, l, *i) {
                    if spec.is_discrete() {
                        self.big.dev.turn_acc += turn * inp.dt * 6.0;
                        let k = self.big.dev.turn_acc.trunc();
                        if k != 0.0 {
                            self.big.dev.turn_acc -= k;
                            if !matches!(spec.kind, ParamKind::Toggle) {
                                self.set_link_value(t, l, *i, (v.round() + k).clamp(spec.min, spec.max));
                            }
                        }
                    } else {
                        let n = spec.to_norm(v) + turn * inp.dt * 0.6;
                        self.set_link_value(t, l, *i, spec.from_norm(n));
                    }
                }
            }
        }

        // The chain.
        let chain_w = (r.width() * 0.32).clamp(s.u * 6.0, s.u * 11.0);
        let item_h = s.u * 1.5;
        let track_name = self.project.tracks[t].name.clone();
        let head = Rect::from_min_size(r.min, vec2(chain_w, s.u * 1.1));
        rect_fill(&p, head, 8.0, shade(accent, 0.35));
        fit_text(&p, pos2(head.left() + s.u * 0.3, head.center().y), Align2::LEFT_CENTER, &track_name, s.font, chain_w - s.u * 0.6, TEXT);
        let list_top = head.bottom() + s.u * 0.2;
        let shown = (((r.bottom() - list_top) / item_h).floor() as usize).max(1);
        let first = link_i.saturating_sub(shown.saturating_sub(1)).min(chain.len().saturating_sub(shown));
        for (k, i) in (first..chain.len().min(first + shown)).enumerate() {
            let (_, name, sub, on) = &chain[i];
            let ir = Rect::from_min_size(pos2(r.left(), list_top + k as f32 * item_h), vec2(chain_w, item_h - s.u * 0.15));
            let resp = ui.interact(ir, Id::new(("big_chain", i)), Sense::click());
            if resp.clicked() {
                link_i = i;
                col = 0;
            }
            let chosen = i == link_i;
            tile(&p, ir, if chosen { TILE_HI } else { TILE }, chosen && col == 0);
            p.circle_filled(pos2(ir.left() + s.u * 0.4, ir.center().y), s.u * 0.16, if *on { accent } else { LINE });
            fit_text(&p, pos2(ir.left() + s.u * 0.75, ir.top() + ir.height() * 0.36), Align2::LEFT_CENTER, name, s.font, ir.width() - s.u, if *on { TEXT } else { TEXT_DIM });
            fit_text(&p, pos2(ir.left() + s.u * 0.75, ir.top() + ir.height() * 0.74), Align2::LEFT_CENTER, sub, s.small, ir.width() - s.u, TEXT_DIM);
        }
        if chain.is_empty() {
            text(&p, pos2(r.left() + s.u * 0.3, list_top + s.u * 0.5), Align2::LEFT_CENTER, "Nothing here yet: Y adds a device", s.f(), TEXT_DIM);
        }

        // The parameters.
        let pr = Rect::from_min_max(pos2(r.left() + chain_w + s.u * 0.5, r.top()), r.max);
        let row_h = s.u * 1.15;
        let shown_rows = ((pr.height() / row_h).floor() as usize).max(1);
        let mut first_row = self.big.dev.first_row;
        if row_i < first_row {
            first_row = row_i;
        } else if row_i >= first_row + shown_rows {
            first_row = row_i + 1 - shown_rows;
        }
        first_row = first_row.min(rows.len().saturating_sub(shown_rows));
        // A finger or the wheel scrolls (the focus comes along).
        let scroll = ui.interact(pr, Id::new(("big_param_scroll", t)), Sense::drag());
        let k = super::scrolled_rows(ui, &scroll, pr, row_h, Id::new("big_param_scroll_acc"));
        if k != 0 {
            first_row = (first_row as i64 + k).clamp(0, rows.len().saturating_sub(shown_rows) as i64) as usize;
            row_i = row_i.clamp(first_row, (first_row + shown_rows).min(rows.len()).saturating_sub(1).max(first_row));
        }
        if rows.is_empty() && link.is_some() {
            let msg = match link {
                Some(Link::Instrument) => "This instrument has its own window: open it in the desktop view",
                _ => "Nothing to set here",
            };
            text(&p, pos2(pr.left(), pr.top() + s.u * 0.5), Align2::LEFT_CENTER, msg, s.f(), TEXT_DIM);
        }
        let name_w = (pr.width() * 0.36).clamp(s.u * 4.0, s.u * 9.0);
        let mut last_group = "";
        for (k, ri) in (first_row..rows.len().min(first_row + shown_rows)).enumerate() {
            let rr = Rect::from_min_size(pos2(pr.left(), pr.top() + k as f32 * row_h), vec2(pr.width(), row_h - s.u * 0.12));
            let focus = col == 1 && ri == row_i;
            tile(&p, rr, if focus { TILE_HI } else { TILE }, focus);
            let bar = Rect::from_min_max(pos2(rr.left() + name_w, rr.top() + s.u * 0.15), pos2(rr.right() - s.u * 0.2, rr.bottom() - s.u * 0.15));
            match rows[ri] {
                Row::Preset => {
                    let inst = self.project.tracks[t].instrument.as_ref();
                    let label = if matches!(inst, Some(Instrument::Drums { .. })) { "Kit" } else { "Preset" };
                    text(&p, pos2(rr.left() + s.u * 0.35, rr.center().y), Align2::LEFT_CENTER, label, s.f(), accent);
                    let name = inst.map(|i| i.display_name()).unwrap_or_default();
                    rect_fill(&p, bar, 6.0, BG);
                    fit_text(&p, bar.center(), Align2::CENTER_CENTER, &format!("<  {name}  >"), s.font, bar.width() - 8.0, TEXT);
                    let resp = ui.interact(bar, Id::new(("big_preset", t)), Sense::click());
                    if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.clicked()) {
                        self.step_preset(t, if pt.x < bar.center().x { -1 } else { 1 });
                        col = 1;
                        row_i = ri;
                    }
                }
                Row::Param(i) => {
                    let spec = &specs[i];
                    let Some(l) = link else { continue };
                    let v = self.link_value(t, l, i).unwrap_or(spec.default);
                    if spec.group != last_group && !spec.group.is_empty() {
                        last_group = spec.group;
                    }
                    let tr = Rect::from_min_size(rr.min, vec2(name_w, rr.height()));
                    fit_text(&p, pos2(tr.left() + s.u * 0.35, tr.top() + tr.height() * 0.38), Align2::LEFT_CENTER, spec.name, s.font, name_w - s.u * 0.5, TEXT);
                    fit_text(&p, pos2(tr.left() + s.u * 0.35, tr.top() + tr.height() * 0.78), Align2::LEFT_CENTER, spec.group, s.small * 0.9, name_w - s.u * 0.5, TEXT_DIM);
                    let resp = ui.interact(bar, Id::new(("big_param", t, i)), Sense::click_and_drag());
                    match spec.kind {
                        ParamKind::Choice(opts) => {
                            rect_fill(&p, bar, 6.0, BG);
                            let o = opts.get(v.round().max(0.0) as usize).copied().unwrap_or("?");
                            fit_text(&p, bar.center(), Align2::CENTER_CENTER, &format!("<  {o}  >"), s.font, bar.width() - 8.0, TEXT);
                            if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.clicked()) {
                                self.set_link_value(t, l, i, step_value(spec, v, if pt.x < bar.center().x { -1 } else { 1 }));
                            }
                        }
                        ParamKind::Toggle => {
                            let on = v >= 0.5;
                            let pill = Rect::from_min_size(bar.min, vec2(bar.width().min(s.u * 3.0), bar.height()));
                            rect_fill(&p, pill, 6.0, if on { accent } else { BG });
                            text(&p, pill.center(), Align2::CENTER_CENTER, if on { "On" } else { "Off" }, s.f(), if on { TEXT_DARK } else { TEXT_DIM });
                            if resp.clicked() {
                                self.set_link_value(t, l, i, if on { 0.0 } else { 1.0 });
                            }
                        }
                        _ => {
                            let bipolar = spec.min < 0.0 && spec.max > 0.0;
                            value_bar(&p, bar, spec.to_norm(v), bipolar, &spec.format(v), s, accent);
                            if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
                                let n = ((pt.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
                                let nv = spec.from_norm(n);
                                self.set_link_value(t, l, i, if spec.is_discrete() { nv.round() } else { nv });
                            }
                            if resp.double_clicked() {
                                self.set_link_value(t, l, i, spec.default);
                            }
                        }
                    }
                    if resp.clicked() || resp.dragged() {
                        col = 1;
                        row_i = ri;
                    }
                }
            }
        }
        // Another device starts at its first row.
        let new_link = link_i != self.big.dev.link;
        let st = &mut self.big.dev;
        st.col = col;
        st.link = link_i;
        st.row = if new_link { 0 } else { row_i };
        st.first_row = if new_link { 0 } else { first_row };
        if add {
            self.big.page = Page::Browse;
        }
        if col == 0 {
            vec![(Glyph::Dpad, "Move"), (Glyph::Btn(Nav::A), "Open"), (Glyph::Triggers, "Device"), (Glyph::Btn(Nav::X), "On/Off"), (Glyph::Btn(Nav::Y), "Add")]
        } else {
            vec![(Glyph::Dpad, "Choose, change"), (Glyph::RightStick, "Turn"), (Glyph::Triggers, "Device"), (Glyph::Btn(Nav::B), "Back"), (Glyph::Btn(Nav::Y), "Add")]
        }
    }
}
