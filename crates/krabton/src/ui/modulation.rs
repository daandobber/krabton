//! Modulation matrix UI: the Mod tab (modulator list, editor and the
//! matrix), the modulator chips in the device view, and the rings drawn on
//! every modulated knob. Connecting works three ways: drag a modulator chip
//! onto a knob, press Map and click/drag knobs, or click a matrix cell.

use crate::app::{App, DetailTab, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, RichText, Sense, Shape, Stroke, Ui, UiBuilder};
use krabton_engine::modulation::{ModKind, ModOwner, ModTarget, Modulator, MAX_MODS};
use krabton_engine::params::{ParamKind, ParamSpec};
use std::collections::VecDeque;

pub(crate) const MOD_COLORS: [Color32; 8] = [
    Color32::from_rgb(0x4f, 0xd1, 0xc5),
    Color32::from_rgb(0xe0, 0x7b, 0xff),
    Color32::from_rgb(0xa3, 0xe6, 0x35),
    Color32::from_rgb(0x60, 0xa5, 0xfa),
    Color32::from_rgb(0xfb, 0xbf, 0x24),
    Color32::from_rgb(0xfb, 0x71, 0x85),
    Color32::from_rgb(0xa7, 0x8b, 0xfa),
    Color32::from_rgb(0x6e, 0xe7, 0xb7),
];

pub(crate) fn mod_color(i: usize) -> Color32 {
    MOD_COLORS[i % MOD_COLORS.len()]
}

/// Drag payload: a modulator being dragged onto a knob.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ModChip(pub u64);

#[derive(Clone, Copy)]
pub(crate) struct Link {
    pub param: usize,
    pub source: u64,
    pub amount: f32,
    pub output: f32,
    pub color: Color32,
}

/// Modulation shown on one parameter set, plus the edits made on it.
pub(crate) struct ModView {
    owner: ModOwner,
    links: Vec<Link>,
    /// Params a clip on this track has an envelope for (a red dot).
    /// Parameters a clip envelope moves, with its value now (while one plays).
    automated: Vec<(usize, Option<f32>)>,
    mapping: Option<(u64, Color32)>,
    /// (source, param, amount) to store after drawing.
    edits: Vec<(u64, usize, f32)>,
}

const HISTORY: usize = 240;

fn span(spec: &ParamSpec) -> f32 {
    if spec.min < 0.0 && (spec.max + spec.min).abs() < 1e-6 {
        0.5
    } else {
        1.0
    }
}

/// Draws modulation on a cell made by `param_cell`, and handles mapping:
/// click or drag in map mode, or a dropped modulator chip.
pub(crate) fn cell_overlay(ui: &mut Ui, cell: Rect, id: Id, spec: &ParamSpec, value: f32, param: usize, mv: &mut ModView) {
    knob_overlay(ui, cell, crate::widgets::cell_knob_rect(cell), id, spec, value, param, mv);
}

/// [`cell_overlay`] for a knob anywhere in `cell` (the area that takes
/// mapping clicks and dropped modulators).
#[allow(clippy::too_many_arguments)]
pub(crate) fn knob_overlay(ui: &mut Ui, cell: Rect, knob: Rect, id: Id, spec: &ParamSpec, value: f32, param: usize, mv: &mut ModView) {
    let p = ui.painter().clone();
    let links: Vec<Link> = mv.links.iter().filter(|l| l.param == param).copied().collect();
    let auto = mv.automated.iter().find(|a| a.0 == param).map(|a| a.1);
    // A playing envelope sets the value the modulation starts from.
    let base = auto.flatten().unwrap_or_else(|| spec.to_norm(value));
    let sp = span(spec);
    let knob_like = !matches!(spec.kind, ParamKind::Choice(_) | ParamKind::Toggle);
    if let Some(now) = auto {
        p.circle_filled(pos2(cell.right() - 4.0, cell.top() + 4.0), 3.0, REC);
        if let (Some(v), true) = (now, knob_like) {
            knob_auto_mark(&p, knob, v);
        }
    }
    if !links.is_empty() {
        let live = (base + links.iter().map(|l| l.amount * l.output * sp).sum::<f32>()).clamp(0.0, 1.0);
        if knob_like {
            let c = knob.center();
            let r = knob.width() * 0.5 - 2.0;
            for (k, l) in links.iter().enumerate() {
                let rr = r + 3.0 + k as f32 * 2.5;
                let to = (base + l.amount * sp).clamp(0.0, 1.0);
                let (a, b) = (KNOB_START + KNOB_RANGE * base.min(to), KNOB_START + KNOB_RANGE * base.max(to));
                if b - a > 0.001 {
                    p.add(Shape::line(arc_points(c, rr, a, b), Stroke::new(2.0_f32, l.color)));
                }
            }
            let ang = KNOB_START + KNOB_RANGE * live;
            p.circle_filled(pos2(c.x + (r + 3.0) * ang.cos(), c.y + (r + 3.0) * ang.sin()), 2.5, Color32::WHITE);
        } else {
            let bar = Rect::from_min_size(pos2(cell.left() + 4.0, knob.center().y + 11.0), vec2(cell.width() - 8.0, 3.0));
            rect_fill(&p, bar, 1.0, PANEL3);
            p.circle_filled(pos2(bar.left() + bar.width() * live, bar.center().y), 2.5, links[0].color);
        }
    }
    map_and_drop(ui, cell, id, spec, param, &links, mv);
}

/// [`cell_overlay`] for a vertical slider (`track` is its slot): the
/// envelope's value now in red, each modulation's range as a bar beside the
/// slot and its live value as a dot.
#[allow(clippy::too_many_arguments)]
pub(crate) fn slider_overlay(ui: &mut Ui, cell: Rect, track: Rect, id: Id, spec: &ParamSpec, value: f32, param: usize, mv: &mut ModView) {
    let p = ui.painter().clone();
    let links: Vec<Link> = mv.links.iter().filter(|l| l.param == param).copied().collect();
    let auto = mv.automated.iter().find(|a| a.0 == param).map(|a| a.1);
    let base = auto.flatten().unwrap_or_else(|| spec.to_norm(value));
    let sp = span(spec);
    let y = |v: f32| track.bottom() - v.clamp(0.0, 1.0) * track.height();
    if let Some(now) = auto {
        p.circle_filled(pos2(cell.right() - 4.0, cell.top() + 4.0), 3.0, REC);
        if let Some(v) = now {
            p.line_segment([pos2(track.left() - 4.0, y(v)), pos2(track.right() + 4.0, y(v))], Stroke::new(2.0_f32, REC));
        }
    }
    if !links.is_empty() {
        for (k, l) in links.iter().enumerate() {
            let x = track.right() + 4.0 + k as f32 * 3.0;
            let to = base + l.amount * sp;
            p.line_segment([pos2(x, y(base)), pos2(x, y(to))], Stroke::new(2.0_f32, l.color));
        }
        let live = (base + links.iter().map(|l| l.amount * l.output * sp).sum::<f32>()).clamp(0.0, 1.0);
        p.circle_filled(pos2(track.center().x, y(live)), 2.5, Color32::WHITE);
    }
    map_and_drop(ui, cell, id, spec, param, &links, mv);
}

/// Mapping (click or drag in map mode) and dropped modulator chips, on any cell.
fn map_and_drop(ui: &mut Ui, cell: Rect, id: Id, spec: &ParamSpec, param: usize, links: &[Link], mv: &mut ModView) {
    let p = ui.painter().clone();
    if let Some((src, color)) = mv.mapping {
        rect_line(&p, cell.shrink(1.0), 4.0, 1.0, color);
        let resp = ui.interact(cell, id.with("map"), Sense::click_and_drag());
        let cur = links.iter().find(|l| l.source == src).map(|l| l.amount);
        if resp.dragged() {
            let fine = if ui.input(|i| i.modifiers.shift) { 0.25 } else { 1.0 };
            let d = (-resp.drag_delta().y * 0.006 + resp.drag_delta().x * 0.003) * fine;
            mv.edits.push((src, param, (cur.unwrap_or(0.0) + d).clamp(-1.0, 1.0)));
        } else if resp.clicked() && cur.is_none() {
            mv.edits.push((src, param, 0.5));
        }
        resp.on_hover_text(format!("{}: drag up/down to set the modulation amount ({})", spec.name, cur.map(|a| format!("{:+.0}%", a * 100.0)).unwrap_or_else(|| "click to connect".into())));
    }
    if egui::DragAndDrop::payload::<ModChip>(ui.ctx()).is_some() {
        let resp = ui.interact(cell, id.with("drop"), Sense::hover());
        if resp.contains_pointer() {
            rect_line(&p, cell, 4.0, 2.0, Color32::WHITE);
        }
        if let Some(chip) = drop_payload::<ModChip>(&resp) {
            if !links.iter().any(|l| l.source == chip.0) {
                mv.edits.push((chip.0, param, 0.5));
            }
        }
    }
}

/// A bipolar amount bar for the matrix. Returns the new amount, or
/// `Some(None)` to remove the route.
fn amount_cell(ui: &mut Ui, r: Rect, id: Id, amount: Option<f32>, color: Color32) -> Option<Option<f32>> {
    let p = ui.painter().clone();
    let resp = ui.interact(r, id, Sense::click_and_drag());
    let mut out = None;
    match amount {
        Some(a) => {
            rect_fill(&p, r, 2.0, KNOB);
            let mid = r.center().x;
            let x = mid + a * (r.width() * 0.5 - 2.0);
            let bar = Rect::from_min_max(pos2(mid.min(x), r.top() + 3.0), pos2(mid.max(x), r.bottom() - 3.0));
            rect_fill(&p, bar, 1.0, color.gamma_multiply(0.75));
            p.line_segment([pos2(mid, r.top() + 2.0), pos2(mid, r.bottom() - 2.0)], Stroke::new(1.0_f32, LINE));
            text(&p, r.center(), Align2::CENTER_CENTER, format!("{:+.0}", a * 100.0), small(), TEXT);
            if resp.dragged() {
                let fine = if ui.input(|i| i.modifiers.shift) { 0.25 } else { 1.0 };
                let d = (resp.drag_delta().x * 0.008 - resp.drag_delta().y * 0.008) * fine;
                out = Some(Some((a + d).clamp(-1.0, 1.0)));
            }
            if resp.double_clicked() {
                out = Some(Some(if a.abs() > 0.99 { 0.5 } else { a.signum() }));
            }
            resp.context_menu(|ui| {
                for (label, v) in [("+100%", 1.0), ("+50%", 0.5), ("-50%", -0.5), ("-100%", -1.0)] {
                    if ui.button(label).clicked() {
                        out = Some(Some(v));
                        ui.close();
                    }
                }
                if ui.button("Invert").clicked() {
                    out = Some(Some(-a));
                    ui.close();
                }
                if ui.button("Disconnect").clicked() {
                    out = Some(None);
                    ui.close();
                }
            });
            resp.on_hover_text("Drag to set the amount (Shift = fine), double-click for full, right-click for more");
        }
        None => {
            if resp.hovered() {
                rect_line(&p, r.shrink(1.0), 2.0, 1.0, color);
                text(&p, r.center(), Align2::CENTER_CENTER, "+", body(), color);
            } else {
                p.circle_filled(r.center(), 1.5, LINE);
            }
            if resp.clicked() {
                out = Some(Some(0.5));
            }
            resp.on_hover_text("Click to connect");
        }
    }
    out
}

/// A small live meter of a modulator's output.
fn meter_bar(p: &egui::Painter, r: Rect, v: f32, bipolar: bool, color: Color32) {
    rect_fill(p, r, 1.5, KNOB);
    let (a, b) = if bipolar { (r.center().x, r.center().x + v * r.width() * 0.5) } else { (r.left(), r.left() + v.max(0.0) * r.width()) };
    let fill = Rect::from_min_max(pos2(a.min(b), r.top()), pos2(a.max(b), r.bottom()));
    rect_fill(p, fill, 1.5, color);
}

impl App {
    fn mod_output(&self, i: usize) -> f32 {
        self.engine.status.mods.get(i).map(|a| a.load()).unwrap_or(0.0)
    }

    /// Modulation for a parameter set, or None when there are no modulators.
    pub(crate) fn mod_view(&self, owner: ModOwner) -> Option<ModView> {
        // Params that clips of this owner's track move.
        let track = self.project.tracks.iter().position(|t| match owner {
            ModOwner::Instrument(id) => t.id == id,
            ModOwner::Device(id) => t.devices.iter().any(|d| d.id == id),
            ModOwner::MidiDevice(id) => t.midi_devices.iter().any(|d| d.id == id),
            ModOwner::Modulator(_) => false,
            ModOwner::Map(id) => t.id == id,
        });
        let mut automated: Vec<(usize, Option<f32>)> = Vec::new();
        if let Some(ti) = track {
            let t = &self.project.tracks[ti];
            for c in t.slots.iter().flatten().chain(t.arrangement.iter().map(|a| &a.clip)) {
                for e in &c.envelopes {
                    if let ModTarget::Param { owner: o, param } = e.target {
                        if o == owner && !e.points.is_empty() && !automated.iter().any(|a| a.0 == param as usize) {
                            automated.push((param as usize, self.envelope_now(ti, e.target)));
                        }
                    }
                }
            }
        }
        if self.project.modulators.is_empty() && automated.is_empty() {
            return None;
        }
        let links = self
            .project
            .mod_routes
            .iter()
            .filter_map(|r| match r.target {
                ModTarget::Param { owner: o, param } if o == owner => {
                    let i = self.project.modulators.iter().position(|m| m.id == r.source)?;
                    Some(Link { param: param as usize, source: r.source, amount: r.amount, output: self.mod_output(i), color: mod_color(i) })
                }
                _ => None,
            })
            .collect();
        let mapping = self.mod_learn.and_then(|id| self.project.modulators.iter().position(|m| m.id == id).map(|i| (id, mod_color(i))));
        Some(ModView { owner, links, automated, mapping, edits: Vec::new() })
    }

    /// Stores the connections made on a parameter set.
    pub(crate) fn apply_mod_view(&mut self, mv: Option<ModView>) {
        let Some(mv) = mv else { return };
        for (src, param, amount) in mv.edits {
            let target = ModTarget::Param { owner: mv.owner, param: param as u16 };
            let new = !self.project.mod_routes.iter().any(|r| r.source == src && r.target == target);
            self.project.add_route(src, target, amount);
            if new {
                let (a, b) = self.project.target_label(target);
                let name = self.project.modulators.iter().find(|m| m.id == src).map(|m| m.name.clone()).unwrap_or_default();
                self.toast(format!("{name} -> {a} {b}"));
            }
        }
    }

    pub(crate) fn add_modulator(&mut self, kind: ModKind) -> Option<u64> {
        if self.project.modulators.len() >= MAX_MODS {
            self.toast(format!("At most {MAX_MODS} modulators"));
            return None;
        }
        let mut m = self.project.make_modulator(kind);
        if kind.uses_track() {
            m.track = match self.sel_track {
                TrackSel::Track(t) => self.project.tracks.get(t).map(|t| t.id),
                _ => None,
            }
            .or_else(|| self.project.tracks.first().map(|t| t.id));
        }
        let id = m.id;
        self.project.modulators.push(m);
        self.sel_mod = Some(id);
        Some(id)
    }

    fn delete_modulator(&mut self, id: u64) {
        self.project.modulators.retain(|m| m.id != id);
        self.project.prune_mod_routes();
        if self.mod_learn == Some(id) {
            self.mod_learn = None;
        }
    }

    /// Per-frame bookkeeping: output history for the scopes, stale ids.
    pub(crate) fn mod_frame(&mut self) {
        let outs: Vec<(u64, f32)> = self.project.modulators.iter().enumerate().map(|(i, m)| (m.id, self.mod_output(i))).collect();
        self.mod_history.retain(|id, _| outs.iter().any(|o| o.0 == *id));
        for (id, v) in outs {
            let h = self.mod_history.entry(id).or_insert_with(|| VecDeque::with_capacity(HISTORY));
            if h.len() >= HISTORY {
                h.pop_front();
            }
            h.push_back(v);
        }
        if let Some(id) = self.mod_learn {
            if !self.project.modulators.iter().any(|m| m.id == id) {
                self.mod_learn = None;
            }
        }
        if self.sel_mod.map(|id| !self.project.modulators.iter().any(|m| m.id == id)).unwrap_or(true) {
            self.sel_mod = self.project.modulators.first().map(|m| m.id);
        }
    }

    fn toggle_map(&mut self, id: u64) {
        if self.mod_learn == Some(id) {
            self.mod_learn = None;
        } else {
            self.mod_learn = Some(id);
            let name = self.project.modulators.iter().find(|m| m.id == id).map(|m| m.name.clone()).unwrap_or_default();
            self.toast(format!("Mapping {name}: click or drag any knob. Esc when done."));
        }
    }

    fn add_menu(&mut self, ui: &mut Ui) {
        for k in ModKind::ALL {
            if ui.button(k.name()).on_hover_text(k.description()).clicked() {
                self.add_modulator(k);
                ui.close();
            }
        }
    }

    /// The "Mapping ..." banner in the detail tab bar.
    pub(crate) fn mod_banner(&mut self, ui: &mut Ui, bar: Rect) {
        let Some(id) = self.mod_learn else { return };
        let Some(i) = self.project.modulators.iter().position(|m| m.id == id) else { return };
        let color = mod_color(i);
        let done = Rect::from_min_size(pos2(bar.right() - 64.0, bar.top() + 3.0), vec2(58.0, bar.height() - 6.0));
        if button(ui, done, Id::new("map_done"), "Done", true, color).clicked() {
            self.mod_learn = None;
        }
        let msg = format!("Mapping {}: click or drag knobs to set amounts", self.project.modulators[i].name);
        text(ui.painter(), pos2(done.left() - 8.0, bar.center().y), Align2::RIGHT_CENTER, msg, small(), color);
    }

    // ------------------------------------------------------------ device view strip

    /// Modulator chips at the start of the device chain. Drag one onto a
    /// knob, or click it to enter map mode.
    pub(crate) fn mod_strip(&mut self, ui: &mut Ui, h: f32) {
        let mods: Vec<(u64, String, ModKind)> = self.project.modulators.iter().map(|m| (m.id, m.name.clone(), m.kind)).collect();
        if mods.is_empty() {
            let (r, resp) = ui.allocate_exact_size(vec2(58.0, h), Sense::click());
            let p = ui.painter();
            rect_line(p, r.shrink(2.0), 4.0, 1.0, if resp.hovered() { MOD_COLORS[0] } else { LINE_SOFT });
            p.text(r.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, "Mod", small(), MOD_COLORS[0]);
            p.text(r.center() + vec2(0.0, 8.0), Align2::CENTER_CENTER, "+", body(), MOD_COLORS[0]);
            let resp = resp.on_hover_text("Add a modulator (LFO, envelope, follower, ...) and connect it to any knob");
            egui::Popup::menu(&resp).show(|ui| self.add_menu(ui));
            return;
        }
        egui::Frame::new().fill(PANEL2).corner_radius(4.0).stroke(Stroke::new(1.0_f32, LINE_SOFT)).inner_margin(egui::Margin::symmetric(4, 4)).show(ui, |ui| {
            ui.set_min_height(h);
            ui.set_width(104.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("MOD").small().strong().color(MOD_COLORS[0]));
                    let add = ui.small_button("+").on_hover_text("Add a modulator");
                    egui::Popup::menu(&add).show(|ui| self.add_menu(ui));
                    if ui.small_button("Matrix").on_hover_text("Open the modulation matrix").clicked() {
                        self.detail = DetailTab::Mod;
                    }
                });
                egui::ScrollArea::vertical().id_salt("mod_strip").max_height(h - 22.0).show(ui, |ui| {
                    for (i, (id, name, kind)) in mods.iter().enumerate() {
                        let (r, resp) = ui.allocate_exact_size(vec2(100.0, 22.0), Sense::click_and_drag());
                        let color = mod_color(i);
                        let mapping = self.mod_learn == Some(*id);
                        let p = ui.painter();
                        rect_fill(p, r, 3.0, if mapping { shade(color, 0.45) } else if resp.hovered() { PANEL3 } else { KNOB });
                        rect_fill(p, Rect::from_min_size(r.min, vec2(3.0, r.height())), 1.0, color);
                        p.with_clip_rect(r.shrink(1.0)).text(pos2(r.left() + 7.0, r.top() + 8.0), Align2::LEFT_CENTER, name, small(), TEXT);
                        meter_bar(p, Rect::from_min_size(pos2(r.left() + 7.0, r.bottom() - 6.0), vec2(r.width() - 12.0, 3.0)), self.mod_output(i), kind.bipolar(), color);
                        resp.dnd_set_drag_payload(ModChip(*id));
                        if resp.clicked() {
                            self.toggle_map(*id);
                        }
                        resp.on_hover_text(format!("{name}: drag onto a knob to connect, or click to map knobs"));
                    }
                });
            });
        });
    }

    // ------------------------------------------------------------ Mod tab

    pub(crate) fn ui_mod_panel(&mut self, ui: &mut Ui, body: Rect) {
        let list_w = 210.0;
        let matrix_w = (body.width() * 0.4).clamp(300.0, 620.0);
        let list = Rect::from_min_size(body.min, vec2(list_w, body.height()));
        let matrix = Rect::from_min_max(pos2(body.right() - matrix_w, body.top()), body.max);
        let editor = Rect::from_min_max(pos2(list.right() + 1.0, body.top()), pos2(matrix.left() - 1.0, body.bottom()));
        let p = ui.painter().clone();
        rect_fill(&p, list, 0.0, PANEL);
        p.line_segment([pos2(list.right(), body.top()), pos2(list.right(), body.bottom())], Stroke::new(1.0_f32, LINE));
        p.line_segment([pos2(matrix.left(), body.top()), pos2(matrix.left(), body.bottom())], Stroke::new(1.0_f32, LINE));

        let mut lui = ui.new_child(UiBuilder::new().max_rect(list.shrink(6.0)).layout(egui::Layout::top_down(egui::Align::Min)));
        lui.set_clip_rect(list);
        self.mod_list(&mut lui);

        let mut eui = ui.new_child(UiBuilder::new().max_rect(editor.shrink(6.0)).layout(egui::Layout::top_down(egui::Align::Min)));
        eui.set_clip_rect(editor);
        self.mod_editor(&mut eui);

        let mut mui = ui.new_child(UiBuilder::new().max_rect(matrix.shrink(6.0)).layout(egui::Layout::top_down(egui::Align::Min)));
        mui.set_clip_rect(matrix);
        self.mod_matrix(&mut mui);
    }

    fn mod_list(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Modulators").strong());
            let add = ui.button("+ Add").on_hover_text("Add a modulator");
            egui::Popup::menu(&add).show(|ui| self.add_menu(ui));
        });
        if self.project.modulators.is_empty() {
            ui.label(RichText::new("Add an LFO, envelope, follower, ... then connect it: drag it onto any knob in the device view, press Map and click knobs, or use the matrix.").small().color(TEXT_DIM));
            return;
        }
        let mods: Vec<(u64, String, ModKind)> = self.project.modulators.iter().map(|m| (m.id, m.name.clone(), m.kind)).collect();
        let mut delete = None;
        let mut duplicate = None;
        egui::ScrollArea::vertical().id_salt("mod_list").auto_shrink([false, false]).show(ui, |ui| {
            for (i, (id, name, kind)) in mods.iter().enumerate() {
                let w = ui.available_width();
                let (r, resp) = ui.allocate_exact_size(vec2(w, 26.0), Sense::click_and_drag());
                let color = mod_color(i);
                let sel = self.sel_mod == Some(*id);
                let p = ui.painter().clone();
                rect_fill(&p, r, 3.0, if sel { PANEL3 } else if resp.hovered() { PANEL2 } else { PANEL });
                rect_fill(&p, Rect::from_min_size(r.min, vec2(4.0, r.height())), 1.0, color);
                p.with_clip_rect(r).text(pos2(r.left() + 10.0, r.top() + 9.0), Align2::LEFT_CENTER, name, small(), TEXT);
                p.text(pos2(r.left() + 10.0, r.bottom() - 7.0), Align2::LEFT_CENTER, kind.name(), small(), TEXT_DIM);
                meter_bar(&p, Rect::from_min_size(pos2(r.right() - 96.0, r.center().y - 2.0), vec2(42.0, 4.0)), self.mod_output(i), kind.bipolar(), color);
                let map_r = Rect::from_min_size(pos2(r.right() - 48.0, r.top() + 4.0), vec2(44.0, r.height() - 8.0));
                if button(ui, map_r, Id::new(("map", *id)), "Map", self.mod_learn == Some(*id), color).on_hover_text("Map mode: click or drag knobs anywhere to connect them").clicked() {
                    let was = self.mod_learn == Some(*id);
                    self.toggle_map(*id);
                    if !was {
                        self.detail = DetailTab::Devices;
                    }
                }
                resp.dnd_set_drag_payload(ModChip(*id));
                if resp.clicked() {
                    self.sel_mod = Some(*id);
                }
                resp.context_menu(|ui| {
                    if ui.button("Duplicate").clicked() {
                        duplicate = Some(*id);
                        ui.close();
                    }
                    if ui.button("Delete").clicked() {
                        delete = Some(*id);
                        ui.close();
                    }
                });
            }
        });
        if let Some(id) = duplicate {
            if let Some(m) = self.project.modulators.iter().find(|m| m.id == id).cloned() {
                if self.project.modulators.len() < MAX_MODS {
                    let new_id = self.project.alloc_id();
                    let routes: Vec<_> = self.project.mod_routes.iter().filter(|r| r.source == id).map(|r| (r.target, r.amount)).collect();
                    self.project.modulators.push(Modulator { id: new_id, name: format!("{} copy", m.name), ..m });
                    for (t, a) in routes {
                        self.project.add_route(new_id, t, a);
                    }
                    self.sel_mod = Some(new_id);
                }
            }
        }
        if let Some(id) = delete {
            self.delete_modulator(id);
        }
    }

    fn mod_editor(&mut self, ui: &mut Ui) {
        let Some(id) = self.sel_mod else { return };
        let Some(i) = self.project.modulators.iter().position(|m| m.id == id) else { return };
        let color = mod_color(i);
        let m = self.project.modulators[i].clone();
        let mut name = m.name.clone();
        let mut track = m.track;
        let mut params = m.params.clone();
        let mut data = m.data.clone();
        let mut delete = false;
        let mut map = false;
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
            ui.painter().circle_filled(r.center(), 5.0, color);
            ui.add(egui::TextEdit::singleline(&mut name).desired_width(120.0));
            ui.label(RichText::new(m.kind.name()).small().color(color));
            if ui.add(egui::Button::new(RichText::new("Map").color(if self.mod_learn == Some(id) { TEXT_DARK } else { TEXT })).fill(if self.mod_learn == Some(id) { color } else { PANEL3 })).on_hover_text("Map mode: click or drag knobs anywhere").clicked() {
                map = true;
            }
            if ui.small_button("Delete").clicked() {
                delete = true;
            }
        });
        ui.label(RichText::new(m.kind.description()).small().color(TEXT_DIM));
        let mut mv = self.mod_view(ModOwner::Modulator(id));
        let mut changed = false;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(186.0);
                // Scope.
                let (r, _) = ui.allocate_exact_size(vec2(184.0, 64.0), Sense::hover());
                let p = ui.painter();
                rect_fill(p, r, 3.0, KNOB);
                let mid = if m.kind.bipolar() { r.center().y } else { r.bottom() - 3.0 };
                let scale = if m.kind.bipolar() { r.height() * 0.5 - 4.0 } else { r.height() - 7.0 };
                p.line_segment([pos2(r.left(), mid), pos2(r.right(), mid)], Stroke::new(1.0_f32, LINE_SOFT));
                if let Some(h) = self.mod_history.get(&id) {
                    let dx = r.width() / (HISTORY - 1) as f32;
                    let x0 = r.right() - (h.len().saturating_sub(1)) as f32 * dx;
                    let pts: Vec<_> = h.iter().enumerate().map(|(k, v)| pos2(x0 + k as f32 * dx, mid - v * scale)).collect();
                    if pts.len() >= 2 {
                        p.add(Shape::line(pts, Stroke::new(1.5_f32, color)));
                    }
                }
                if m.kind.uses_track() {
                    let label = |t: Option<u64>| t.and_then(|t| self.project.tracks.iter().find(|x| x.id == t)).map(|t| t.name.clone()).unwrap_or_else(|| "(no track)".into());
                    let what = if m.kind == ModKind::Follower { "Listens to" } else { "Notes from" };
                    ui.label(RichText::new(what).small().color(TEXT_DIM));
                    egui::ComboBox::from_id_salt(("mod_track", id)).width(170.0).selected_text(label(track)).show_ui(ui, |ui| {
                        for t in &self.project.tracks {
                            if ui.selectable_label(track == Some(t.id), &t.name).clicked() {
                                track = Some(t.id);
                            }
                        }
                    });
                }
                if m.kind == ModKind::Steps {
                    if data.len() < 16 {
                        data.resize(16, 0);
                    }
                    let (r, resp) = ui.allocate_exact_size(vec2(184.0, 54.0), Sense::click_and_drag());
                    let p = ui.painter().clone();
                    rect_fill(&p, r, 3.0, KNOB);
                    let len = params[1].round().clamp(1.0, 16.0) as usize;
                    let cw = r.width() / 16.0;
                    for (k, v) in data.iter().enumerate().take(16) {
                        let c = Rect::from_min_size(pos2(r.left() + k as f32 * cw + 1.0, r.top() + 2.0), vec2(cw - 2.0, r.height() - 4.0));
                        let bh = 2.0 + *v as f32 / 255.0 * (c.height() - 2.0);
                        let col = if k < len { color } else { shade(color, 0.3) };
                        rect_fill(&p, Rect::from_min_max(pos2(c.left(), c.bottom() - bh), c.max), 1.0, col);
                    }
                    if let (Some(pt), true) = (resp.interact_pointer_pos(), resp.is_pointer_button_down_on()) {
                        let k = ((pt.x - r.left()) / cw).floor();
                        if (0.0..16.0).contains(&k) {
                            let v = ((r.bottom() - 2.0 - pt.y) / (r.height() - 4.0)).clamp(0.0, 1.0);
                            data[k as usize] = (v * 255.0).round() as u8;
                            changed = true;
                        }
                    }
                    resp.on_hover_text("Draw the steps");
                }
                if m.kind == ModKind::Tides {
                    tides_preview(ui, &params, color);
                }
            });
            ui.add_space(8.0);
            if crate::ui::devices::param_grid_mod(ui, Id::new(("mod_params", id)), m.kind.specs(), &mut params, color, ui.available_height() - 4.0, mv.as_mut()) {
                changed = true;
            }
        });
        self.apply_mod_view(mv.take());
        if map {
            self.toggle_map(id);
            if self.mod_learn == Some(id) {
                // The knobs to map are in the device view.
                self.detail = DetailTab::Devices;
            }
        }
        if delete {
            self.delete_modulator(id);
            return;
        }
        if let Some(m) = self.project.modulators.iter_mut().find(|m| m.id == id) {
            if name != m.name {
                m.name = name;
            }
            m.track = track;
            if changed {
                m.params = params;
                m.data = data;
            }
        }
    }

    fn mod_matrix(&mut self, ui: &mut Ui) {
        let sel = self.sel_mod;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Matrix").strong());
            ui.label(RichText::new("rows: sources · columns: targets").small().color(TEXT_DIM));
            let b = ui.small_button("+ Mixer").on_hover_text("Connect the selected modulator to a volume, pan or send");
            egui::Popup::menu(&b).show(|ui| {
                let Some(src) = sel else {
                    ui.label("Select a modulator first");
                    return;
                };
                let tracks: Vec<(u64, String)> = self.project.tracks.iter().map(|t| (t.id, t.name.clone())).chain(self.project.returns.iter().map(|r| (r.id, r.name.clone()))).collect();
                let returns: Vec<String> = self.project.returns.iter().map(|r| r.name.clone()).collect();
                for (tid, tname) in tracks {
                    let is_track = self.project.tracks.iter().any(|t| t.id == tid);
                    ui.menu_button(tname, |ui| {
                        let mut pick = None;
                        if ui.button("Volume").clicked() {
                            pick = Some((ModTarget::Volume(tid), -0.5));
                        }
                        if ui.button("Pan").clicked() {
                            pick = Some((ModTarget::Pan(tid), 0.5));
                        }
                        if is_track {
                            for (s, rn) in returns.iter().enumerate() {
                                if ui.button(format!("Send {rn}")).clicked() {
                                    pick = Some((ModTarget::Send { track: tid, send: s as u16 }, 0.5));
                                }
                            }
                        }
                        if let Some((t, a)) = pick {
                            self.project.add_route(src, t, a);
                            ui.close();
                        }
                    });
                }
            });
        });
        let mut targets: Vec<ModTarget> = Vec::new();
        for r in &self.project.mod_routes {
            if !targets.contains(&r.target) {
                targets.push(r.target);
            }
        }
        if self.project.modulators.is_empty() {
            return;
        }
        if targets.is_empty() {
            ui.label(RichText::new("Nothing connected yet. Drag a modulator onto a knob, or press Map and click knobs.").small().color(TEXT_DIM));
            return;
        }
        let labels: Vec<(String, String)> = targets.iter().map(|t| self.project.target_label(*t)).collect();
        let mods: Vec<(u64, String)> = self.project.modulators.iter().map(|m| (m.id, m.name.clone())).collect();
        let amounts: Vec<Vec<Option<f32>>> = mods
            .iter()
            .map(|(id, _)| targets.iter().map(|t| self.project.mod_routes.iter().find(|r| r.source == *id && r.target == *t).map(|r| r.amount)).collect())
            .collect();
        let (row_h, col_w, label_w, head_h) = (22.0, 66.0, 92.0, 30.0);
        let mut changes = Vec::new();
        egui::ScrollArea::both().id_salt("mod_matrix").auto_shrink([false, false]).show(ui, |ui| {
            let size = vec2(label_w + targets.len() as f32 * col_w, head_h + mods.len() as f32 * row_h);
            let (r, _) = ui.allocate_exact_size(size, Sense::hover());
            let p = ui.painter().clone();
            for (c, (a, b)) in labels.iter().enumerate() {
                let x = r.left() + label_w + c as f32 * col_w;
                let hr = Rect::from_min_size(pos2(x, r.top()), vec2(col_w - 2.0, head_h - 2.0));
                rect_fill(&p, hr, 2.0, PANEL2);
                p.with_clip_rect(hr).text(pos2(hr.center().x, hr.top() + 8.0), Align2::CENTER_CENTER, a, small(), TEXT_DIM);
                p.with_clip_rect(hr).text(pos2(hr.center().x, hr.bottom() - 8.0), Align2::CENTER_CENTER, b, small(), TEXT);
                ui.interact(hr, Id::new(("mx_head", c)), Sense::hover()).on_hover_text(format!("{a} · {b}"));
            }
            for (ri, (id, name)) in mods.iter().enumerate() {
                let y = r.top() + head_h + ri as f32 * row_h;
                let color = mod_color(ri);
                let lr = Rect::from_min_size(pos2(r.left(), y), vec2(label_w - 4.0, row_h - 2.0));
                let lresp = ui.interact(lr, Id::new(("mx_row", *id)), Sense::click());
                rect_fill(&p, lr, 2.0, if sel == Some(*id) { PANEL3 } else { PANEL2 });
                rect_fill(&p, Rect::from_min_size(lr.min, vec2(3.0, lr.height())), 1.0, color);
                p.with_clip_rect(lr).text(pos2(lr.left() + 7.0, lr.center().y), Align2::LEFT_CENTER, name, small(), TEXT);
                if lresp.clicked() {
                    self.sel_mod = Some(*id);
                }
                for c in 0..targets.len() {
                    let cr = Rect::from_min_size(pos2(r.left() + label_w + c as f32 * col_w, y), vec2(col_w - 2.0, row_h - 2.0));
                    if let Some(new) = amount_cell(ui, cr, Id::new(("mx", *id, c)), amounts[ri][c], color) {
                        changes.push((*id, targets[c], new));
                    }
                }
            }
        });
        for (src, target, new) in changes {
            match new {
                Some(a) => {
                    self.project.add_route(src, target, a);
                }
                None => self.project.mod_routes.retain(|r| !(r.source == src && r.target == target)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput};
    use krabton_engine::params::Unit;

    static SPEC: ParamSpec = ParamSpec::log("Cutoff", "Filter", 20.0, 20000.0, 1000.0, Unit::Hz);

    /// Draws one modulatable knob per frame; events are relative to its cell.
    fn drive(mv: &mut ModView, frames: &[Vec<Event>], before_frame: impl Fn(usize, &egui::Context)) {
        let ctx = egui::Context::default();
        let cell = Rect::from_min_size(pos2(100.0, 100.0), vec2(CELL_W, CELL_H));
        for (i, evs) in std::iter::once(&Vec::new()).chain(frames.iter()).enumerate() {
            before_frame(i, &ctx);
            let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), events: evs.clone(), ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut v = 1000.0;
                    param_cell(ui, cell, Id::new("k"), &SPEC, &mut v, ACCENT);
                    cell_overlay(ui, cell, Id::new("k").with("mod"), &SPEC, v, 3, mv);
                });
            });
        }
    }

    fn at(x: f32, y: f32) -> Pos2 {
        pos2(100.0 + x, 100.0 + y)
    }

    fn press(p: Pos2, pressed: bool) -> Event {
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
    }

    fn view(mapping: Option<u64>) -> ModView {
        ModView { owner: ModOwner::Device(1), links: Vec::new(), automated: Vec::new(), mapping: mapping.map(|id| (id, MOD_COLORS[0])), edits: Vec::new() }
    }

    #[test]
    fn map_mode_click_connects_and_drag_sets_amount() {
        let mut mv = view(Some(7));
        let c = at(27.0, 30.0);
        drive(&mut mv, &[vec![Event::PointerMoved(c), press(c, true)], vec![press(c, false)]], |_, _| {});
        assert_eq!(mv.edits, vec![(7, 3, 0.5)]);
        let mut mv = view(Some(7));
        let frames: Vec<Vec<Event>> = vec![
            vec![Event::PointerMoved(c), press(c, true)],
            vec![Event::PointerMoved(at(27.0, 10.0))],
            vec![Event::PointerMoved(at(27.0, -10.0))],
            vec![press(at(27.0, -10.0), false)],
        ];
        drive(&mut mv, &frames, |_, _| {});
        let last = mv.edits.last().copied().expect("dragging edits the amount");
        assert_eq!((last.0, last.1), (7, 3));
        assert!(last.2 > 0.05, "dragging up raises the amount: {:?}", mv.edits);
    }

    #[test]
    fn dropping_a_chip_connects_it() {
        let mut mv = view(None);
        let c = at(27.0, 30.0);
        let frames = vec![vec![Event::PointerMoved(c), press(c, true)], vec![Event::PointerMoved(c)], vec![press(c, false)]];
        drive(&mut mv, &frames, |i, ctx| {
            if i == 1 {
                egui::DragAndDrop::set_payload(ctx, ModChip(9));
            }
        });
        assert_eq!(mv.edits, vec![(9, 3, 0.5)]);
    }

    #[test]
    fn without_mapping_the_knob_stays_a_knob() {
        let mut mv = view(None);
        let c = at(27.0, 30.0);
        drive(&mut mv, &[vec![Event::PointerMoved(c), press(c, true)], vec![press(c, false)]], |_, _| {});
        assert!(mv.edits.is_empty());
    }
}

/// One cycle of Tides' four outputs at its settings, the one it sends bright.
fn tides_preview(ui: &mut Ui, params: &[f32], color: Color32) {
    use krabton_engine::dsp::tides as td;
    let get = |i: usize| params.get(i).copied().unwrap_or(0.0);
    let s = td::Settings {
        ramp_mode: (get(0).round().max(0.0) as usize).min(2),
        output_mode: (get(1).round().max(0.0) as usize).min(3),
        shape: get(5),
        slope: get(6),
        smoothness: get(7),
        shift: get(8),
    };
    let (r, resp) = ui.allocate_exact_size(vec2(184.0, 72.0), Sense::hover());
    let p = ui.painter();
    rect_fill(p, r, 3.0, KNOB);
    let n = 96;
    let cycle = td::preview(&s, n);
    let bipolar = s.ramp_mode == td::LOOP;
    let plot = r.shrink2(vec2(4.0, 6.0));
    let mid = if bipolar { plot.center().y } else { plot.bottom() };
    let scale = if bipolar { plot.height() * 0.5 } else { plot.height() };
    p.line_segment([pos2(plot.left(), mid), pos2(plot.right(), mid)], Stroke::new(1.0_f32, LINE_SOFT));
    let out = (get(9).round().clamp(1.0, 4.0) as usize) - 1;
    let line = |j: usize| -> Vec<egui::Pos2> { cycle.iter().enumerate().map(|(k, o)| pos2(plot.left() + k as f32 / (n - 1) as f32 * plot.width(), mid - o[j].clamp(-1.0, 1.0) * scale)).collect() };
    for j in (0..td::CHANNELS).filter(|&j| j != out) {
        p.add(Shape::line(line(j), Stroke::new(1.0_f32, shade(color, 0.32))));
    }
    p.add(Shape::line(line(out), Stroke::new(1.8_f32, color)));
    let what = if s.output_mode == td::FREQUENCY {
        let r = td::ratios(s.shift);
        format!("x{} x{} x{} x{}", trim(r[0]), trim(r[1]), trim(r[2]), trim(r[3]))
    } else {
        format!("{} · {}", td::RAMP_MODES[s.ramp_mode], td::OUTPUT_MODES[s.output_mode])
    };
    text(p, r.left_top() + vec2(5.0, 3.0), Align2::LEFT_TOP, what, small(), TEXT_DIM);
    text(p, r.right_top() + vec2(-5.0, 3.0), Align2::RIGHT_TOP, format!("out {}", out + 1), small(), color);
    resp.on_hover_text("One cycle of the four outputs (an AD after a note, an AR held for half of it); the bright one is the output this modulator sends");
}

fn trim(v: f32) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}
