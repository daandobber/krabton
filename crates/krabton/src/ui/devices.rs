//! Device view: the instrument and effect chain of the selected track.

use crate::app::{App, BrowserItem, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, RichText, Sense, Stroke, Ui, UiBuilder};
use krabton_engine::dsp::drums::{self, DrumVoice};
use krabton_engine::dsp::effects::EffectKind;
use krabton_engine::midi_fx::MidiFxKind;
use krabton_engine::dsp::analog;
use krabton_engine::dsp::shaker_synth;
use krabton_engine::dsp::stk_voice;
use krabton_engine::dsp::stk_mallets;
use krabton_engine::dsp::stk_strings;
use krabton_engine::dsp::stk_winds;
use krabton_engine::dsp::organ;
use krabton_engine::dsp::rings_synth;
use krabton_engine::dsp::braids_synth;
use krabton_engine::dsp::elements_synth;
use krabton_engine::dsp::macro_osc::{self, HARMONICS, MODEL};
use krabton_engine::dsp::{sampler, synth};
use krabton_engine::params::{ParamSpec, Unit};
use krabton_engine::modulation::ModOwner;
use krabton_engine::{note_name, Command, Device, Instrument, SampleRef};
use super::modulation::{cell_overlay, ModView};

static PAD_SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Level", "Pad", -36.0, 6.0, 0.0, Unit::Db),
    ParamSpec::int("Tune", "Pad", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::log("Decay", "Pad", 0.1, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Tone", "Pad", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Pan", "Pad", -1.0, 1.0, 0.0, Unit::Pan),
];

enum ChainAction {
    Remove(usize),
    MoveLeft(usize),
    MoveRight(usize),
}

/// Lays out parameters in labelled groups of up to three rows.
pub(crate) fn param_grid(ui: &mut Ui, id: Id, specs: &[ParamSpec], values: &mut [f32], accent: Color32, height: f32) -> bool {
    param_grid_mod(ui, id, specs, values, accent, height, None)
}

/// [`param_grid`] that also shows and edits modulation of these parameters.
pub(crate) fn param_grid_mod(ui: &mut Ui, id: Id, specs: &[ParamSpec], values: &mut [f32], accent: Color32, height: f32, mut mv: Option<&mut ModView>) -> bool {
    let mut groups: Vec<(&str, Vec<usize>)> = Vec::new();
    for (i, s) in specs.iter().enumerate() {
        match groups.iter_mut().find(|g| g.0 == s.group) {
            Some(g) => g.1.push(i),
            None => groups.push((s.group, vec![i])),
        }
    }
    let rows = (((height - 16.0) / CELL_H).floor() as usize).clamp(1, 3);
    let widths: Vec<f32> = groups.iter().map(|g| g.1.len().div_ceil(rows) as f32 * CELL_W + 10.0).collect();
    let total: f32 = widths.iter().sum();
    let (rect, _) = ui.allocate_exact_size(vec2(total, rows as f32 * CELL_H + 16.0), Sense::hover());
    let p = ui.painter().clone();
    let mut x = rect.left();
    let mut changed = false;
    for (gi, (name, idxs)) in groups.iter().enumerate() {
        p.text(pos2(x + 5.0, rect.top() + 6.0), Align2::LEFT_CENTER, *name, small(), shade(accent, 0.85));
        if gi > 0 {
            p.line_segment([pos2(x, rect.top() + 2.0), pos2(x, rect.bottom() - 2.0)], Stroke::new(1.0_f32, LINE_SOFT));
        }
        for (k, &pi) in idxs.iter().enumerate() {
            let (col, row) = (k / rows, k % rows);
            let cell = Rect::from_min_size(pos2(x + 5.0 + col as f32 * CELL_W, rect.top() + 14.0 + row as f32 * CELL_H), vec2(CELL_W - 2.0, CELL_H - 2.0));
            if let Some(v) = values.get_mut(pi) {
                if param_cell(ui, cell, id.with(pi), &specs[pi], v, accent) {
                    changed = true;
                }
                if let Some(mv) = mv.as_deref_mut() {
                    cell_overlay(ui, cell, id.with(("mod", pi)), &specs[pi], *v, pi, mv);
                }
            }
        }
        x += widths[gi];
    }
    changed
}

/// Device frame with a title bar. Returns the body's inner response.
fn device_frame<R>(ui: &mut Ui, title: &str, enabled: Option<&mut bool>, height: f32, body: impl FnOnce(&mut Ui) -> R) -> (R, Option<ChainActionKind>) {
    let mut action = None;
    let r = egui::Frame::new()
        .fill(PANEL2)
        .corner_radius(4.0)
        .stroke(Stroke::new(1.0_f32, LINE_SOFT))
        .inner_margin(egui::Margin::symmetric(6, 4))
        .show(ui, |ui| {
            ui.set_min_height(height);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if let Some(en) = enabled {
                        let (rr, resp) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::click());
                        ui.painter().circle_filled(rr.center(), 5.5, if *en { ACCENT } else { PANEL3 });
                        if resp.on_hover_text("Device on/off").clicked() {
                            *en = !*en;
                        }
                    }
                    ui.label(RichText::new(title).strong().color(TEXT));
                    ui.add_space(10.0);
                    // Buttons sit next to the title so the panel keeps its natural width.
                    if ui.small_button("<").on_hover_text("Move left").clicked() {
                        action = Some(ChainActionKind::Left);
                    }
                    if ui.small_button(">").on_hover_text("Move right").clicked() {
                        action = Some(ChainActionKind::Right);
                    }
                    if ui.small_button("x").on_hover_text("Remove").clicked() {
                        action = Some(ChainActionKind::Remove);
                    }
                });
                body(ui)
            })
            .inner
        });
    (r.inner, action)
}

#[derive(Clone, Copy)]
enum ChainActionKind {
    Remove,
    Left,
    Right,
}

impl App {
    pub(crate) fn chain_mut(&mut self, sel: TrackSel) -> Option<&mut Vec<Device>> {
        match sel {
            TrackSel::Track(i) => self.project.tracks.get_mut(i).map(|t| &mut t.devices),
            TrackSel::Return(j) => self.project.returns.get_mut(j).map(|t| &mut t.devices),
            TrackSel::Master => Some(&mut self.project.master.devices),
        }
    }

    pub(crate) fn ui_devices(&mut self, ui: &mut Ui, body: Rect) {
        let sel = self.sel_track;
        // Free the display buffers of devices that are gone.
        let shown: Vec<u64> = self.project.all_devices().filter(|d| d.kind.has_display()).map(|d| d.id).collect();
        let status = self.status();
        status.scopes.release_unused(|key| shown.contains(&(key & !krabton_engine::scope::PRE_KEY)));
        status.meters.release_unused(|id| shown.contains(&id));
        let mut dui = ui.new_child(UiBuilder::new().max_rect(body.shrink(4.0)));
        dui.set_clip_rect(body);
        let h = (body.height() - 34.0).max(80.0);
        let mut actions = Vec::new();
        let mut midi_actions = Vec::new();
        egui::ScrollArea::horizontal().auto_shrink([false, false]).show(&mut dui, |ui| {
            ui.horizontal_top(|ui| {
                self.mod_strip(ui, h);
                if let TrackSel::Track(i) = sel {
                    // MIDI effects come first, like in Live: notes flow left to right.
                    if self.project.tracks.get(i).map(|t| t.kind == krabton_engine::TrackKind::Midi).unwrap_or(false) {
                        let n = self.project.tracks[i].midi_devices.len();
                        for d in 0..n {
                            if let Some(a) = self.midi_fx_panel(ui, i, d, h) {
                                midi_actions.push(a);
                            }
                        }
                        self.add_midi_fx_slot(ui, i, h);
                    }
                    if self.project.tracks.get(i).map(|t| t.instrument.is_some()).unwrap_or(false) {
                        self.instrument_panel(ui, i, h);
                    } else if self.project.tracks.get(i).map(|t| t.kind == krabton_engine::TrackKind::Midi).unwrap_or(false) {
                        self.empty_instrument_slot(ui, i, h);
                    }
                }
                let n = self.chain_mut(sel).map(|c| c.len()).unwrap_or(0);
                for d in 0..n {
                    if let Some(a) = self.effect_panel(ui, sel, d, h) {
                        actions.push(a);
                    }
                }
                self.add_effect_slot(ui, sel, h);
            });
        });
        if let (TrackSel::Track(i), false) = (sel, midi_actions.is_empty()) {
            if let Some(chain) = self.project.tracks.get_mut(i).map(|t| &mut t.midi_devices) {
                for a in midi_actions {
                    match a {
                        ChainAction::Remove(d) if d < chain.len() => {
                            chain.remove(d);
                        }
                        ChainAction::MoveLeft(d) if d > 0 && d < chain.len() => chain.swap(d, d - 1),
                        ChainAction::MoveRight(d) if d + 1 < chain.len() => chain.swap(d, d + 1),
                        _ => {}
                    }
                }
            }
        }
        if let Some(chain) = self.chain_mut(sel) {
            for a in actions {
                match a {
                    ChainAction::Remove(d) if d < chain.len() => {
                        chain.remove(d);
                    }
                    ChainAction::MoveLeft(d) if d > 0 && d < chain.len() => chain.swap(d, d - 1),
                    ChainAction::MoveRight(d) if d + 1 < chain.len() => chain.swap(d, d + 1),
                    _ => {}
                }
            }
        }
    }

    fn effect_panel(&mut self, ui: &mut Ui, sel: TrackSel, d: usize, h: f32) -> Option<ChainAction> {
        let chain = self.chain_mut(sel)?;
        let dev = chain.get_mut(d)?;
        let kind = dev.kind;
        let dev_id = dev.id;
        let title = dev.name().to_string();
        let plugin = dev.plugin.clone();
        let id = Id::new(("device", dev.id));
        let mut enabled = dev.enabled;
        let mut params = dev.params.clone();
        let accent = if enabled { ACCENT } else { TEXT_DIM };
        let mut mv = self.mod_view(ModOwner::Device(dev_id));
        let status = self.status();
        // Displays are as wide as the user dragged them (per kind of device).
        let default_w = match kind {
            EffectKind::Scope => 360.0,
            EffectKind::LensEq => 620.0,
            EffectKind::LensComp => 420.0,
            _ => 400.0,
        };
        let mut width = self.prefs.layout.display(kind.name(), default_w);
        let width_before = width;
        let plugin_status = plugin.as_ref().map(|p| self.plugins.status(p.instance));
        let mut toggle_editor = false;
        let (changed, action) = device_frame(ui, &title, Some(&mut enabled), h, |ui| {
            if let (Some(p), Some(st)) = (&plugin, &plugin_status) {
                ui.vertical(|ui| {
                    ui.set_min_width(240.0);
                    ui.label(RichText::new(&p.name).strong().size(15.0).color(accent));
                    ui.label(RichText::new(format!("by {}", p.vendor)).small().color(TEXT_DIM));
                    ui.add_space(6.0);
                    match st {
                        crate::plugins::PluginStatus::Loaded { has_editor: true, editor_open } => {
                            let label = if *editor_open { "Close Editor" } else { "Open Editor" };
                            if ui.add(egui::Button::new(RichText::new(label).color(TEXT_DARK)).fill(ACCENT).min_size(vec2(130.0, 26.0))).clicked() {
                                toggle_editor = true;
                            }
                        }
                        crate::plugins::PluginStatus::Loaded { .. } => {
                            ui.label("This plug-in has no editor.");
                        }
                        crate::plugins::PluginStatus::Failed(e) => {
                            ui.label(RichText::new(format!("Could not load it: {e}")).color(REC));
                            ui.label(RichText::new(p.path.display().to_string()).small().color(TEXT_DIM));
                        }
                        crate::plugins::PluginStatus::Loading => {
                            ui.label("Loading...");
                        }
                    }
                    ui.add_space(4.0);
                    ui.label(RichText::new("Its settings are saved with your set.").small().color(TEXT_DIM));
                });
                false
            } else if kind == EffectKind::Scope {
                ui.horizontal_top(|ui| {
                    let opts = crate::ui::scope::ScopeOpts::from_params(&params);
                    let (rect, _) = ui.allocate_exact_size(vec2(width, (h - 30.0).max(80.0)), Sense::hover());
                    let resp = crate::ui::scope::scope_view(ui, rect, id.with("screen"), status.scopes.find(dev_id), status.sample_rate.load(), &opts);
                    resp.on_hover_text("Wave: locked to the pitch. Time: triggered window. XY: left against right. Vector: mid up, side across. Spectrum: level per frequency. Drag the right edge to make it wider.");
                    width_grip(ui, id.with("display_width"), rect, &mut width, 160.0, 1800.0);
                    ui.add_space(6.0);
                    param_grid_mod(ui, id, kind.specs(), &mut params, accent, h - 24.0, mv.as_mut())
                })
                .inner
            } else if kind == EffectKind::LensEq {
                crate::ui::lens::eq_panel(ui, id, dev_id, &mut params, accent, h, mv.as_mut(), &status, &mut width)
            } else if kind == EffectKind::LensComp {
                crate::ui::lens::comp_panel(ui, id, dev_id, &mut params, accent, h, mv.as_mut(), &status, &mut width)
            } else if kind == EffectKind::LensLimiter {
                crate::ui::lens::limiter_panel(ui, id, dev_id, &mut params, accent, h, mv.as_mut(), &status, &mut width)
            } else {
                param_grid_mod(ui, id, kind.specs(), &mut params, accent, h - 24.0, mv.as_mut())
            }
        });
        self.apply_mod_view(mv);
        if let (true, Some(p)) = (toggle_editor, &plugin) {
            self.toggle_effect_editor(p.instance, &p.name);
        }
        if width != width_before {
            self.prefs.layout.displays.insert(kind.name().to_string(), width);
            self.layout_dirty = true;
        }
        let dev = self.chain_mut(sel)?.get_mut(d)?;
        dev.enabled = enabled;
        if changed {
            dev.params = params;
        }
        action.map(|a| match a {
            ChainActionKind::Remove => ChainAction::Remove(d),
            ChainActionKind::Left => ChainAction::MoveLeft(d),
            ChainActionKind::Right => ChainAction::MoveRight(d),
        })
    }

    fn midi_fx_panel(&mut self, ui: &mut Ui, track: usize, d: usize, h: f32) -> Option<ChainAction> {
        let tr = self.project.tracks.get(track)?;
        let dev = tr.midi_devices.get(d)?.clone();
        let mut enabled = dev.enabled;
        let mut params = dev.params.clone();
        let mut data = dev.data.clone();
        let id = Id::new(("midi_device", dev.id));
        let mut mv = self.mod_view(ModOwner::MidiDevice(dev.id));
        let (changed, action) = if dev.kind.is_sequencer() {
            // The engine reports the state of the first enabled sequencer.
            let shown = tr.midi_devices.iter().position(|m| m.enabled && m.kind.is_sequencer()) == Some(d);
            let status = self.status();
            let vis = status.tracks.get(track).filter(|_| shown).map(|ts| std::array::from_fn(|i| ts.seq[i].load(std::sync::atomic::Ordering::Relaxed)));
            let live = super::seq_view::Live { vis, playing: self.is_playing(), scale: self.project.scale };
            let accent = if enabled { PLAY } else { TEXT_DIM };
            let lane = &mut self.grid.poly_lane;
            device_frame(ui, dev.kind.name(), Some(&mut enabled), h, |ui| {
                ui.label(RichText::new(format!("Sequencer · {}", dev.kind.description())).small().color(shade(PLAY, 0.8)));
                ui.horizontal_top(|ui| {
                    let a = super::seq_view::editor(ui, id.with("editor"), dev.kind, &mut params, &mut data, live, lane, accent, h - 44.0);
                    ui.add_space(6.0);
                    let b = param_grid_mod(ui, id, dev.kind.specs(), &mut params, accent, h - 40.0, mv.as_mut());
                    a | b
                })
                .inner
            })
        } else {
            let accent = if enabled { SELECT } else { TEXT_DIM };
            device_frame(ui, dev.kind.name(), Some(&mut enabled), h, |ui| {
                ui.label(RichText::new("MIDI effect").small().color(shade(SELECT, 0.8)));
                param_grid_mod(ui, id, dev.kind.specs(), &mut params, accent, h - 40.0, mv.as_mut())
            })
        };
        self.apply_mod_view(mv);
        let target = self.project.tracks.get_mut(track)?.midi_devices.get_mut(d)?;
        target.enabled = enabled;
        if changed {
            target.params = params;
            target.data = data;
        }
        action.map(|a| match a {
            ChainActionKind::Remove => ChainAction::Remove(d),
            ChainActionKind::Left => ChainAction::MoveLeft(d),
            ChainActionKind::Right => ChainAction::MoveRight(d),
        })
    }

    fn add_midi_fx_slot(&mut self, ui: &mut Ui, track: usize, h: f32) {
        let empty = self.project.tracks.get(track).map(|t| t.midi_devices.is_empty()).unwrap_or(true);
        let (r, resp) = ui.allocate_exact_size(vec2(if empty { 104.0 } else { 34.0 }, h), Sense::click());
        let hovering = resp.dnd_hover_payload::<BrowserItem>().is_some();
        let p = ui.painter();
        rect_line(p, r.shrink(2.0), 4.0, 1.0, if hovering || resp.hovered() { SELECT } else { LINE_SOFT });
        if empty {
            p.text(r.center() - vec2(0.0, 16.0), Align2::CENTER_CENTER, "MIDI Effects", small(), shade(SELECT, 0.8));
            p.text(r.center(), Align2::CENTER_CENTER, "drop here", small(), TEXT_DIM);
            p.text(r.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "or click", small(), TEXT_DIM);
        } else {
            p.text(r.center(), Align2::CENTER_CENTER, "+", body(), shade(SELECT, 0.8));
        }
        if let Some(item) = drop_payload::<BrowserItem>(&resp) {
            self.apply_item(&item, crate::app::DropTarget::Track(track));
        }
        let menu_id = Id::new(("midi_fx_menu", track));
        if resp.clicked() {
            ui.ctx().memory_mut(|m| m.data.insert_temp(menu_id, true));
        }
        resp.on_hover_text("Add a sequencer or MIDI effect (arpeggiator, chord, ...)");
        if ui.ctx().memory(|m| m.data.get_temp::<bool>(menu_id).unwrap_or(false)) {
            let mut close = false;
            egui::Area::new(menu_id.with("area")).fixed_pos(r.left_top() + vec2(4.0, 4.0)).order(egui::Order::Foreground).show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new("Sequencers").small().color(PLAY));
                    for k in MidiFxKind::SEQUENCERS {
                        if ui.button(k.name()).on_hover_text(k.description()).clicked() {
                            self.apply_item(&BrowserItem::MidiFx(k), crate::app::DropTarget::Track(track));
                            close = true;
                        }
                    }
                    ui.label(RichText::new("MIDI effects").small().color(SELECT));
                    for k in MidiFxKind::ALL {
                        if ui.button(k.name()).on_hover_text(k.description()).clicked() {
                            let dev = self.project.make_midi_device(k);
                            if let Some(t) = self.project.tracks.get_mut(track) {
                                t.midi_devices.push(dev);
                            }
                            close = true;
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
            if close || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                ui.ctx().memory_mut(|m| m.data.insert_temp(menu_id, false));
            }
        }
    }

    fn add_effect_slot(&mut self, ui: &mut Ui, sel: TrackSel, h: f32) {
        let (r, resp) = ui.allocate_exact_size(vec2(130.0, h), Sense::click());
        let hovering = resp.dnd_hover_payload::<BrowserItem>().is_some();
        let p = ui.painter();
        rect_line(p, r.shrink(2.0), 4.0, 1.0, if hovering { ACCENT } else { LINE_SOFT });
        p.text(r.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, "Drop effects here", small(), TEXT_DIM);
        p.text(r.center() + vec2(0.0, 8.0), Align2::CENTER_CENTER, "or click to add", small(), TEXT_DIM);
        if let Some(item) = drop_payload::<BrowserItem>(&resp) {
            if let BrowserItem::Effect(k) = &*item {
                let dev = self.project.make_device(*k);
                if let Some(c) = self.chain_mut(sel) {
                    c.push(dev);
                }
            } else if let BrowserItem::PluginEffect(info) = &*item {
                let dev = self.project.make_plugin_device(crate::plugins::PluginManager::make_instrument(info, 0));
                if let Some(c) = self.chain_mut(sel) {
                    c.push(dev);
                }
            } else {
                let t = self.default_target();
                self.apply_item(&item, t);
            }
        }
        resp.context_menu(|ui| effect_menu(self, ui, sel));
        if resp.clicked() {
            // Left-click opens the same menu via a popup.
            ui.ctx().memory_mut(|m| m.data.insert_temp(Id::new("fx_menu_open"), true));
        }
        let open = ui.ctx().memory(|m| m.data.get_temp::<bool>(Id::new("fx_menu_open")).unwrap_or(false));
        if open {
            let area = egui::Area::new(Id::new("fx_menu")).fixed_pos(r.left_top() + vec2(4.0, 4.0)).order(egui::Order::Foreground);
            let mut close = false;
            area.show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    // A column per category, four to a row.
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    for row in EffectKind::CATEGORIES.chunks(4) {
                        ui.horizontal_top(|ui| {
                            for &cat in row {
                                ui.vertical(|ui| {
                                    ui.set_min_width(130.0);
                                    ui.label(RichText::new(cat).small().color(TEXT_DIM));
                                    for k in EffectKind::ALL.into_iter().filter(|k| k.category() == cat) {
                                        if ui.button(k.name()).on_hover_text(k.description()).clicked() {
                                            let dev = self.project.make_device(k);
                                            if let Some(c) = self.chain_mut(sel) {
                                                c.push(dev);
                                            }
                                            close = true;
                                        }
                                    }
                                });
                            }
                        });
                        ui.add_space(4.0);
                    }
                    let plugins: Vec<krabton_clap::PluginInfo> = self.plugins.effects().cloned().collect();
                    if !plugins.is_empty() {
                        ui.separator();
                        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                            for info in &plugins {
                                if ui.button(&info.name).on_hover_text(format!("{} (CLAP)", info.vendor)).clicked() {
                                    let dev = self.project.make_plugin_device(crate::plugins::PluginManager::make_instrument(info, 0));
                                    if let Some(c) = self.chain_mut(sel) {
                                        c.push(dev);
                                    }
                                    close = true;
                                }
                            }
                        });
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
            if close || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                ui.ctx().memory_mut(|m| m.data.insert_temp(Id::new("fx_menu_open"), false));
            }
        }
    }

    fn empty_instrument_slot(&mut self, ui: &mut Ui, i: usize, h: f32) {
        let (r, resp) = ui.allocate_exact_size(vec2(160.0, h), Sense::click());
        let p = ui.painter();
        rect_line(p, r.shrink(2.0), 4.0, 1.0, if resp.dnd_hover_payload::<BrowserItem>().is_some() { ACCENT } else { LINE_SOFT });
        p.text(r.center(), Align2::CENTER_CENTER, "Drop an instrument", small(), TEXT_DIM);
        if let Some(item) = drop_payload::<BrowserItem>(&resp) {
            self.apply_item(&item, crate::app::DropTarget::Track(i));
        }
    }

    fn instrument_panel(&mut self, ui: &mut Ui, i: usize, h: f32) {
        let track_id = self.project.tracks[i].id;
        let Some(inst) = self.project.tracks[i].instrument.clone() else { return };
        let mut new_inst = inst.clone();
        let mut remove = false;
        let title = inst.kind_name();
        let mut toggle_editor = false;
        let mut mv = self.mod_view(ModOwner::Instrument(track_id));
        let (_, action) = device_frame(ui, title, None, h, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| match &mut new_inst {
                    Instrument::Fm { voice, op_mask, bank, params } => {
                        self.fm_panel(ui, track_id, voice, op_mask, bank, params, h, mv.as_mut());
                    }
                    Instrument::Plugin(p) => {
                        ui.set_min_width(300.0);
                        ui.label(RichText::new(&p.name).strong().size(16.0).color(ACCENT));
                        ui.label(RichText::new(format!("by {}  -  {}", p.vendor, p.plugin_id)).small().color(TEXT_DIM));
                        ui.add_space(6.0);
                        match self.plugins.status(p.instance) {
                            crate::plugins::PluginStatus::Loaded { has_editor, editor_open } => {
                                if has_editor {
                                    let label = if editor_open { "Close Editor" } else { "Open Editor" };
                                    if ui.add(egui::Button::new(RichText::new(label).color(TEXT_DARK)).fill(ACCENT).min_size(vec2(140.0, 26.0))).clicked() {
                                        toggle_editor = true;
                                    }
                                } else {
                                    ui.label("This plugin has no editor.");
                                }
                                ui.add_space(4.0);
                                ui.label(RichText::new("Play it like any instrument. Its patch and settings are saved with your set.").small().color(TEXT_DIM));
                            }
                            crate::plugins::PluginStatus::Failed(e) => {
                                ui.label(RichText::new(format!("Could not load the plugin: {e}")).color(REC));
                                ui.label(RichText::new(p.path.display().to_string()).small().color(TEXT_DIM));
                            }
                            crate::plugins::PluginStatus::Loading => {
                                ui.label("Loading...");
                            }
                        }
                    }
                    Instrument::Synth { preset, params } => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Preset").small().color(TEXT_DIM));
                            egui::ComboBox::from_id_salt(("synth_preset", track_id)).width(110.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
                                for name in synth::preset_names() {
                                    if ui.selectable_label(preset == name, *name).clicked() {
                                        *preset = name.to_string();
                                        *params = synth::preset(name);
                                    }
                                }
                            });
                        });
                        param_grid_mod(ui, Id::new(("synth", track_id)), synth::SPECS, params, ACCENT, h - 48.0, mv.as_mut());
                    }
                    Instrument::Analog { preset, params } => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Preset").small().color(TEXT_DIM));
                            egui::ComboBox::from_id_salt(("analog_preset", track_id)).width(130.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
                                for name in analog::preset_names() {
                                    if ui.selectable_label(preset == name, *name).clicked() {
                                        *preset = name.to_string();
                                        *params = analog::preset(name);
                                    }
                                }
                            });
                            ui.label(RichText::new("classic analog polysynth").small().color(TEXT_DIM));
                        });
                        param_grid_mod(ui, Id::new(("analog", track_id)), analog::SPECS, params, ACCENT, h - 48.0, mv.as_mut());
                    }
                    Instrument::Rings { preset, params } => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Preset").small().color(TEXT_DIM));
                            egui::ComboBox::from_id_salt(("rings_synth_preset", track_id)).width(130.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
                                for name in rings_synth::preset_names() {
                                    if ui.selectable_label(preset == name, *name).clicked() {
                                        *preset = name.to_string();
                                        *params = rings_synth::preset(name);
                                    }
                                }
                            });
                            ui.label(RichText::new("struck resonator: modal, strings, FM, chords").small().color(TEXT_DIM));
                        });
                        param_grid_mod(ui, Id::new(("rings_synth", track_id)), rings_synth::SPECS, params, ACCENT, h - 48.0, mv.as_mut());
                    }
                    Instrument::Organ { preset, params } => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Preset").small().color(TEXT_DIM));
                            egui::ComboBox::from_id_salt(("organ_preset", track_id)).width(130.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
                                for name in organ::preset_names() {
                                    if ui.selectable_label(preset == name, *name).clicked() {
                                        *preset = name.to_string();
                                        *params = organ::preset(name);
                                    }
                                }
                            });
                            ui.label(RichText::new("tonewheel organ: drawbars, percussion, vibrato, rotary").small().color(TEXT_DIM));
                        });
                        param_grid_mod(ui, Id::new(("organ", track_id)), organ::SPECS, params, ACCENT, h - 48.0, mv.as_mut());
                    }
                    Instrument::Winds { preset, params } => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Preset").small().color(TEXT_DIM));
                            egui::ComboBox::from_id_salt(("stk_winds_preset", track_id)).width(130.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
                                for name in stk_winds::preset_names() {
                                    if ui.selectable_label(preset == name, *name).clicked() {
                                        *preset = name.to_string();
                                        *params = stk_winds::preset(name);
                                    }
                                }
                            });
                            ui.label(RichText::new("blown physical models (STK)").small().color(TEXT_DIM));
                        });
                        stk_controls(ui, track_id, krabton_engine::dsp::stk_synth::Family::Winds, params, h, mv.as_mut());
                    }
                    Instrument::Strings { preset, params } => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Preset").small().color(TEXT_DIM));
                            egui::ComboBox::from_id_salt(("stk_strings_preset", track_id)).width(130.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
                                for name in stk_strings::preset_names() {
                                    if ui.selectable_label(preset == name, *name).clicked() {
                                        *preset = name.to_string();
                                        *params = stk_strings::preset(name);
                                    }
                                }
                            });
                            ui.label(RichText::new("string physical models (STK)").small().color(TEXT_DIM));
                        });
                        stk_controls(ui, track_id, krabton_engine::dsp::stk_synth::Family::Strings, params, h, mv.as_mut());
                    }
                    Instrument::Mallets { preset, params } => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Preset").small().color(TEXT_DIM));
                            egui::ComboBox::from_id_salt(("stk_mallets_preset", track_id)).width(130.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
                                for name in stk_mallets::preset_names() {
                                    if ui.selectable_label(preset == name, *name).clicked() {
                                        *preset = name.to_string();
                                        *params = stk_mallets::preset(name);
                                    }
                                }
                            });
                            ui.label(RichText::new("struck physical models (STK)").small().color(TEXT_DIM));
                        });
                        stk_controls(ui, track_id, krabton_engine::dsp::stk_synth::Family::Mallets, params, h, mv.as_mut());
                    }
                    Instrument::Voice { preset, params } => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Preset").small().color(TEXT_DIM));
                            egui::ComboBox::from_id_salt(("stk_voice_preset", track_id)).width(130.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
                                for name in stk_voice::preset_names() {
                                    if ui.selectable_label(preset == name, *name).clicked() {
                                        *preset = name.to_string();
                                        *params = stk_voice::preset(name);
                                    }
                                }
                            });
                            ui.label(RichText::new("formant voice (STK)").small().color(TEXT_DIM));
                        });
                        stk_controls(ui, track_id, krabton_engine::dsp::stk_synth::Family::Voice, params, h, mv.as_mut());
                    }
                    Instrument::Shakers { preset, params } => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Preset").small().color(TEXT_DIM));
                            egui::ComboBox::from_id_salt(("shakers_preset", track_id)).width(130.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
                                for name in shaker_synth::preset_names() {
                                    if ui.selectable_label(preset == name, *name).clicked() {
                                        *preset = name.to_string();
                                        *params = shaker_synth::preset(name);
                                    }
                                }
                            });
                            ui.label(RichText::new("PhISEM shakers: the key tunes them, Hold keeps shaking").small().color(TEXT_DIM));
                        });
                        param_grid_mod(ui, Id::new(("shakers", track_id)), shaker_synth::SPECS, params, ACCENT, h - 48.0, mv.as_mut());
                    }
                    Instrument::Macro { preset, params } => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Preset").small().color(TEXT_DIM));
                            egui::ComboBox::from_id_salt(("macro_preset", track_id)).width(130.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
                                for name in macro_osc::preset_names() {
                                    if ui.selectable_label(preset == name, *name).clicked() {
                                        *preset = name.to_string();
                                        *params = macro_osc::preset(name);
                                    }
                                }
                            });
                            ui.label(RichText::new("models from Mutable Instruments Plaits (MIT)").small().color(TEXT_DIM));
                        });
                        ui.horizontal_top(|ui| {
                            macro_model_picker(ui, track_id, params, h - 48.0);
                            ui.add_space(6.0);
                            // The three macro knobs are named after what they do in this model.
                            let model = (params[MODEL].round().max(0.0) as usize).min(macro_osc::MODEL_KNOBS.len() - 1);
                            let mut specs = macro_osc::SPECS.to_vec();
                            for k in 0..3 {
                                specs[HARMONICS + k].name = macro_osc::MODEL_KNOBS[model][k];
                            }
                            param_grid_mod(ui, Id::new(("macro", track_id)), &specs, params, ACCENT, h - 48.0, mv.as_mut());
                        });
                    }
                    Instrument::Braids { preset, params } => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Preset").small().color(TEXT_DIM));
                            egui::ComboBox::from_id_salt(("braids_preset", track_id)).width(130.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
                                for name in braids_synth::preset_names() {
                                    if ui.selectable_label(preset == name, *name).clicked() {
                                        *preset = name.to_string();
                                        *params = braids_synth::preset(name);
                                    }
                                }
                            });
                            ui.label(RichText::new("shapes from Mutable Instruments Braids (MIT)").small().color(TEXT_DIM));
                        });
                        ui.horizontal_top(|ui| {
                            braids_shape_picker(ui, track_id, params, h - 48.0);
                            ui.add_space(6.0);
                            let shape = (params[braids_synth::SHAPE].round().max(0.0) as usize).min(braids_synth::SHAPE_COUNT - 1);
                            let mut specs = braids_synth::SPECS.to_vec();
                            specs[braids_synth::TIMBRE].name = braids_synth::SHAPE_KNOBS[shape][0];
                            specs[braids_synth::COLOR].name = braids_synth::SHAPE_KNOBS[shape][1];
                            param_grid_mod(ui, Id::new(("braids", track_id)), &specs, params, ACCENT, h - 48.0, mv.as_mut());
                        });
                    }
                    Instrument::Elements { preset, params } => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Preset").small().color(TEXT_DIM));
                            egui::ComboBox::from_id_salt(("elements_preset", track_id)).width(130.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
                                for name in elements_synth::preset_names() {
                                    if ui.selectable_label(preset == name, *name).clicked() {
                                        *preset = name.to_string();
                                        *params = elements_synth::preset(name);
                                    }
                                }
                            });
                            ui.label(RichText::new("modal synthesis from Mutable Instruments Elements (MIT)").small().color(TEXT_DIM));
                        });
                        ui.horizontal_top(|ui| {
                            elements_model_picker(ui, track_id, params, h - 48.0);
                            ui.add_space(6.0);
                            let mut specs = elements_synth::SPECS.to_vec();
                            specs[elements_synth::GEOMETRY].name = elements_synth::geometry_name(params[elements_synth::MODEL]);
                            // MODEL (the last parameter) is the buttons on the left.
                            let knobs = elements_synth::MODEL;
                            param_grid_mod(ui, Id::new(("elements", track_id)), &specs[..knobs], &mut params[..knobs], ACCENT, h - 48.0, mv.as_mut());
                        });
                    }
                    Instrument::Sampler { params, sample, zones } => {
                        let mut load: Option<Vec<std::path::PathBuf>> = None;
                        let sel = self.sel_zone.min(zones.len().saturating_sub(1));
                        ui.horizontal(|ui| {
                            if ui.button("Load...").on_hover_text("WAV or FLAC files (several are laid out by the notes in their names), or an SFZ instrument").clicked() {
                                if let Some(files) = rfd::FileDialog::new().add_filter("Samples and SFZ", &["wav", "flac", "sfz", "WAV", "FLAC", "SFZ"]).pick_files() {
                                    load = Some(files);
                                }
                            }
                            if ui.button("Folder...").on_hover_text("Every sample in a folder, laid out by the notes in their names (or the SFZ in it)").clicked() {
                                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                    load = Some(vec![dir]);
                                }
                            }
                            let source = zones.get(sel).map(|z| z.sample.clone()).or_else(|| sample.clone());
                            if ui.add_enabled(source.is_some(), egui::Button::new("Slice")).on_hover_text("Cut the sample at its hits: one slice per key from C1, played One-Shot").clicked() {
                                if let Some(s) = source {
                                    *zones = krabton_engine::sample_map::slice_zones(&s, 64);
                                    params[sampler::MODE] = 1.0;
                                    *sample = Some(s);
                                    self.sel_zone = 0;
                                    self.toast(format!("{} slices, from C1 up", zones.len()));
                                }
                            }
                            if !zones.is_empty() && ui.button("Clear").on_hover_text("Back to one sample on every key").clicked() {
                                if sample.is_none() {
                                    *sample = zones.first().map(|z| z.sample.clone());
                                }
                                zones.clear();
                            }
                            let name = match zones.len() {
                                0 => sample.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "No sample".into()),
                                1 => zones[0].sample.name.clone(),
                                n => format!("{n} zones"),
                            };
                            ui.label(RichText::new(name).color(TEXT_DIM));
                        });
                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| {
                                // Loop mode, next to the editor.
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Loop").small().color(TEXT_DIM));
                                    for (k, name) in sampler::LOOP_MODES.iter().enumerate() {
                                        if ui.selectable_label(params[sampler::LOOP].round() as usize == k, *name).clicked() {
                                            params[sampler::LOOP] = k as f32;
                                        }
                                    }
                                    ui.label(RichText::new("drag S/E and the loop; Ctrl+wheel zooms").small().color(TEXT_DIM));
                                });
                                let map_h = if zones.is_empty() { 0.0 } else { 84.0 };
                                let mut ed_w = self.prefs.layout.display("Sampler", 380.0);
                                let size = vec2(ed_w, (h - 80.0 - map_h).max(40.0));
                                let wid = Id::new(("sampler_wave", track_id, sel, zones.len()));
                                let resp = match zones.get_mut(sel) {
                                    Some(z) => {
                                        let s = z.sample.clone();
                                        wave_editor(ui, wid, size, &s, sampler::zone_region(z), params, Some(&mut z.loop_range))
                                    }
                                    None => match sample.clone() {
                                        Some(s) => {
                                            let region = (0.0, (s.frames() as f64 - 1.0).max(1.0));
                                            wave_editor(ui, wid, size, &s, region, params, None)
                                        }
                                        None => {
                                            let (r, resp) = ui.allocate_exact_size(size, Sense::click());
                                            draw_waveform(ui, r, None, -1.0);
                                            resp
                                        }
                                    },
                                };
                                let r = resp.rect;
                                let before = ed_w;
                                width_grip(ui, Id::new(("sampler_width", track_id)), r, &mut ed_w, 240.0, 1800.0);
                                if ed_w != before {
                                    self.prefs.layout.displays.insert("Sampler".into(), ed_w);
                                    self.layout_dirty = true;
                                }
                                if let Some(item) = drop_payload::<BrowserItem>(&resp) {
                                    match &*item {
                                        BrowserItem::Sample(path) => match SampleRef::load(path) {
                                            Ok(s) => {
                                                *sample = Some(s);
                                                zones.clear();
                                            }
                                            Err(e) => self.toast(e),
                                        },
                                        BrowserItem::SamplerPatch(path) => load = Some(vec![path.clone()]),
                                        _ => {}
                                    }
                                }
                                if resp.dnd_hover_payload::<BrowserItem>().is_some() {
                                    rect_line(ui.painter(), r, 2.0, 2.0, ACCENT);
                                }
                                if !zones.is_empty() {
                                    let (zr, zresp) = ui.allocate_exact_size(vec2(ed_w, 34.0), Sense::click());
                                    let hit = draw_zone_map(ui, zr, zones, sel, zresp.hover_pos());
                                    if let (true, Some(k)) = (zresp.clicked(), hit) {
                                        self.sel_zone = k;
                                    }
                                    if let Some(k) = hit {
                                        let z = &zones[k];
                                        zresp.on_hover_text(format!("{}: {} to {}, root {}, velocity {} to {}", z.sample.name, note_name(z.lo_key), note_name(z.hi_key), note_name(z.root), z.lo_vel, z.hi_vel));
                                    }
                                    if let Some(z) = zones.get_mut(sel) {
                                        zone_editor(ui, z, sel);
                                    }
                                }
                            });
                            param_grid_mod(ui, Id::new(("sampler", track_id)), sampler::SPECS, params, ACCENT, h - 48.0, mv.as_mut());
                        });
                        if let Some(paths) = load {
                            match krabton_engine::sample_map::load_zones(&paths) {
                                Ok(lz) => {
                                    let n = lz.zones.len();
                                    if n == 1 && lz.zones[0].lo_key == 0 && lz.zones[0].hi_key == 127 {
                                        // One file: the plain single sample.
                                        params[sampler::ROOT] = lz.zones[0].root as f32;
                                        *sample = Some(lz.zones[0].sample.clone());
                                        zones.clear();
                                    } else {
                                        *zones = lz.zones;
                                        *sample = None;
                                    }
                                    for (k, v) in [sampler::ATTACK, sampler::DECAY, sampler::SUSTAIN, sampler::RELEASE].into_iter().zip(lz.envelope) {
                                        if let Some(v) = v {
                                            params[k] = sampler::SPECS[k].clamp(v);
                                        }
                                    }
                                    params[sampler::MODE] = 0.0;
                                    // SFZ loops play with Loop on.
                                    params[sampler::LOOP] = if zones.iter().any(|z| z.loop_range.is_some()) { 1.0 } else { 0.0 };
                                    self.sel_zone = 0;
                                    let mut msg = format!("Loaded {n} zone(s)");
                                    if !lz.failed.is_empty() {
                                        msg += &format!(", {} file(s) could not be read (only WAV and FLAC)", lz.failed.len());
                                    }
                                    if lz.skipped > 0 {
                                        msg += &format!(", {} release or round-robin region(s) left out", lz.skipped);
                                    }
                                    self.toast(msg);
                                }
                                Err(e) => self.toast(e),
                            }
                        }
                    }
                    Instrument::Drums { kit, pads, engine } => {
                        let engine = *engine;
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Kit").small().color(TEXT_DIM));
                            egui::ComboBox::from_id_salt(("drum_kit", track_id)).width(130.0).height(360.0).selected_text(kit.as_str()).show_ui(ui, |ui| {
                                for name in engine.kits() {
                                    if ui.selectable_label(kit == name, *name).clicked() {
                                        *kit = name.to_string();
                                        *pads = drums::kit(name);
                                    }
                                }
                            })
                            .response
                            .on_hover_text(engine.description());
                            if engine == drums::DrumEngine::Rack && ui.button("From folder...").on_hover_text("A kit from a folder of drum samples: kick, snare, hats, ... go to their pads by name").clicked() {
                                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                    let files = krabton_engine::sample_map::files_in(&dir);
                                    let (new_pads, failed) = krabton_engine::sample_map::drum_kit_from(&files);
                                    let n = new_pads.iter().filter(|p| p.sample.is_some()).count();
                                    if n == 0 {
                                        self.toast("No WAV or FLAC files in that folder");
                                    } else {
                                        *pads = new_pads;
                                        *kit = dir.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Samples".into());
                                        self.toast(if failed.is_empty() { format!("{n} samples on the pads") } else { format!("{n} samples on the pads, {} could not be read", failed.len()) });
                                    }
                                }
                            }
                        });
                        ui.horizontal_top(|ui| {
                            // Pad grid: 4 x 2, lowest note bottom-left like a drum rack.
                            let cell = vec2(70.0, ((h - 60.0) / 2.0).clamp(30.0, 60.0));
                            let (grid, _) = ui.allocate_exact_size(vec2(cell.x * 4.0 + 6.0, cell.y * 2.0 + 4.0), Sense::hover());
                            let mut order: Vec<usize> = (0..pads.len().min(8)).collect();
                            order.sort_by_key(|&k| pads[k].note);
                            for (slot, &k) in order.iter().enumerate() {
                                let col = slot % 4;
                                let row = 1 - (slot / 4).min(1);
                                let r = Rect::from_min_size(pos2(grid.left() + col as f32 * (cell.x + 2.0), grid.top() + row as f32 * (cell.y + 2.0)), cell);
                                let resp = ui.interact(r, Id::new(("pad", track_id, k)), Sense::click());
                                let selected = self.sel_pad == k;
                                let pad = &mut pads[k];
                                let fill = if resp.is_pointer_button_down_on() {
                                    ACCENT
                                } else if selected {
                                    PANEL3
                                } else {
                                    KNOB
                                };
                                let p = ui.painter();
                                rect_fill(p, r, 3.0, fill);
                                if selected {
                                    rect_line(p, r, 3.0, 1.0, ACCENT);
                                }
                                let tc = if fill == ACCENT { TEXT_DARK } else { TEXT };
                                p.with_clip_rect(r).text(pos2(r.left() + 5.0, r.top() + 10.0), Align2::LEFT_CENTER, &pad.name, small(), tc);
                                let sub = if pad.sample.is_some() { format!("{}  sample", note_name(pad.note)) } else { note_name(pad.note) };
                                p.text(pos2(r.left() + 5.0, r.bottom() - 9.0), Align2::LEFT_CENTER, sub, small(), shade(tc, 0.7));
                                if resp.dnd_hover_payload::<BrowserItem>().is_some() {
                                    rect_line(p, r, 3.0, 2.0, ACCENT);
                                }
                                if let Some(item) = drop_payload::<BrowserItem>(&resp) {
                                    if let BrowserItem::Sample(path) = &*item {
                                        if let Ok(s) = SampleRef::load(path) {
                                            pad.name = s.name.clone();
                                            pad.sample = Some(s);
                                        }
                                    }
                                }
                                if resp.clicked() {
                                    self.sel_pad = k;
                                    let note = pad.note;
                                    self.pending_pad_hit = Some(note);
                                }
                            }
                            ui.vertical(|ui| {
                                let k = self.sel_pad.min(pads.len().saturating_sub(1));
                                if let Some(pad) = pads.get_mut(k) {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&pad.name).strong());
                                        egui::ComboBox::from_id_salt(("pad_voice", track_id, k)).width(96.0).height(420.0).selected_text(pad.voice.name()).show_ui(ui, |ui| {
                                            for &(group, voices) in engine.groups() {
                                                ui.label(RichText::new(group).small().color(TEXT_DIM));
                                                for &v in voices.iter() {
                                                    if ui.selectable_label(pad.voice == v, format!("  {}", v.name())).clicked() {
                                                        // A name the kit or Krabton gave follows the voice.
                                                        if pad.sample.is_none() && DrumVoice::GROUPS.iter().flat_map(|g| g.1.iter()).any(|o| o.name() == pad.name) {
                                                            pad.name = v.name().to_string();
                                                        }
                                                        pad.voice = v;
                                                    }
                                                }
                                            }
                                        })
                                        .response
                                        .on_hover_text("Classic: Krabton's own drums. Mutable: the drum models of Mutable Instruments Plaits. Shakers: Perry Cook's PhISEM shakers.");
                                        let mut note = pad.note as i32;
                                        if ui.add(egui::DragValue::new(&mut note).range(0..=127).custom_formatter(|v, _| note_name(v as u8))).on_hover_text("MIDI note").changed() {
                                            pad.note = note as u8;
                                        }
                                        if ui.small_button("Sample...").clicked() {
                                            if let Some(path) = rfd::FileDialog::new().add_filter("WAV", &["wav"]).pick_file() {
                                                if let Ok(s) = SampleRef::load(&path) {
                                                    pad.name = s.name.clone();
                                                    pad.sample = Some(s);
                                                }
                                            }
                                        }
                                        if pad.sample.is_some() && ui.small_button("Synth").on_hover_text("Remove the sample, use the synthesized voice").clicked() {
                                            pad.sample = None;
                                            pad.name = pad.voice.name().to_string();
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Choke").small().color(TEXT_DIM));
                                        let label = |g: u8| if g == 0 { "-".to_string() } else { g.to_string() };
                                        egui::ComboBox::from_id_salt(("pad_choke", track_id, k)).width(36.0).selected_text(label(pad.choke)).show_ui(ui, |ui| {
                                            for g in 0..=4u8 {
                                                ui.selectable_value(&mut pad.choke, g, label(g));
                                            }
                                        })
                                        .response
                                        .on_hover_text("Pads in the same choke group cut each other off, like a closed hat stops an open one");
                                    });
                                    // Level, Tune, Decay, Tone, Pan and the voice's own knobs.
                                    let mut specs = PAD_SPECS.to_vec();
                                    let macros = pad.voice.macros();
                                    for name in macros.iter().flatten() {
                                        specs.push(ParamSpec::lin(name, "Pad", 0.0, 1.0, 0.5, Unit::Percent));
                                    }
                                    let mut vals = vec![pad.level_db, pad.tune, pad.decay, pad.tone, pad.pan];
                                    vals.extend(macros.iter().zip([pad.a, pad.b]).filter(|(m, _)| m.is_some()).map(|(_, v)| v));
                                    if param_grid(ui, Id::new(("pad_params", track_id, k)), &specs, &mut vals, ACCENT, h - 80.0) {
                                        pad.level_db = vals[0];
                                        pad.tune = vals[1];
                                        pad.decay = vals[2];
                                        pad.tone = vals[3];
                                        pad.pan = vals[4];
                                        let mut extra = vals[5..].iter();
                                        if macros[0].is_some() {
                                            pad.a = *extra.next().unwrap_or(&pad.a);
                                        }
                                        if macros[1].is_some() {
                                            pad.b = *extra.next().unwrap_or(&pad.b);
                                        }
                                    }
                                }
                            });
                        });
                    }
                });
            });
        });
        self.apply_mod_view(mv);
        if matches!(action, Some(ChainActionKind::Remove)) {
            remove = true;
        }
        let t = &mut self.project.tracks[i];
        if remove {
            t.instrument = None;
        } else if new_inst != inst {
            t.instrument = Some(new_inst);
        }
        if let Some(note) = self.pending_pad_hit.take() {
            let ids = self.preview_note_on(i, note, 0.9);
            self.note_off(note, &ids);
        }
        if toggle_editor {
            self.toggle_plugin_editor(i);
        }
        let _ = Command::AllNotesOff;
    }
}

/// The Braids shapes by family: a row of family tabs and the shapes of
/// the open family, labelled with the module's four-letter codes.
fn braids_shape_picker(ui: &mut Ui, track_id: u64, params: &mut [f32], height: f32) {
    let current = params[braids_synth::SHAPE].round().max(0.0) as usize;
    let fam_id = Id::new(("braids_family", track_id));
    let current_family = braids_synth::FAMILIES.iter().position(|(_, first, n)| current >= *first && current < first + n).unwrap_or(0);
    let open = ui.ctx().memory(|m| m.data.get_temp::<usize>(fam_id)).unwrap_or(current_family);
    const TABS: [&str; 5] = ["Analog", "Digital", "Model", "Wave", "Noise"];
    let (cols, bw) = (3usize, 64.0);
    let bh = ((height - 22.0) / 5.0).clamp(15.0, 22.0);
    let (rect, _) = ui.allocate_exact_size(vec2(cols as f32 * (bw + 3.0), 20.0 + bh * 5.0), Sense::hover());
    let tab_w = rect.width() / braids_synth::FAMILIES.len() as f32;
    for (f, (name, ..)) in braids_synth::FAMILIES.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + f as f32 * tab_w, rect.top()), vec2(tab_w - 2.0, 16.0));
        let resp = button(ui, r, Id::new(("braids_fam", track_id, f)), TABS[f], open == f, if f == current_family { ACCENT } else { SELECT }).on_hover_text(*name);
        if resp.clicked() {
            ui.ctx().memory_mut(|m| m.data.insert_temp(fam_id, f));
        }
    }
    let (_, first, count) = braids_synth::FAMILIES[open];
    for k in 0..count {
        let shape = first + k;
        let (col, row) = (k % cols, k / cols);
        let r = Rect::from_min_size(pos2(rect.left() + col as f32 * (bw + 3.0), rect.top() + 20.0 + row as f32 * bh), vec2(bw, bh - 2.0));
        let (code, name) = krabton_braids::SHAPES[shape];
        if button(ui, r, Id::new(("braids_shape", track_id, shape)), code, current == shape, ACCENT).on_hover_text(name).clicked() {
            params[braids_synth::SHAPE] = shape as f32;
        }
    }
}

/// The three resonator models of Elements, plus what the MALLET knob and
/// (for Strings) GEOMETRY currently pick.
fn elements_model_picker(ui: &mut Ui, track_id: u64, params: &mut [f32], height: f32) {
    use elements_synth::{BLOW, BOW, GEOMETRY, MALLET, MODEL, MODEL_NAMES, STRIKE};
    let (bw, bh) = (104.0, ((height - 64.0) / 3.0).clamp(15.0, 22.0));
    let (rect, _) = ui.allocate_exact_size(vec2(bw, bh * 3.0 + 64.0), Sense::hover());
    let current = params[MODEL].round().clamp(0.0, 2.0) as usize;
    const HINTS: [&str; 3] = [
        "Modal: 52 band-pass modes, from strings and bars to plates and bells",
        "String: one Karplus-Strong string with dispersion",
        "Strings: five sympathetic strings tuned to a chord",
    ];
    for (k, name) in MODEL_NAMES.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left(), rect.top() + k as f32 * bh), vec2(bw, bh - 2.0));
        if button(ui, r, Id::new(("elements_model", track_id, k)), name, current == k, ACCENT).on_hover_text(HINTS[k]).clicked() {
            params[MODEL] = k as f32;
        }
    }
    let p = ui.painter();
    let mut y = rect.top() + bh * 3.0 + 6.0;
    let line = |y: f32, label: &str, value: &str| {
        p.text(pos2(rect.left() + 2.0, y), Align2::LEFT_TOP, label, small(), TEXT_DIM);
        p.text(pos2(rect.right() - 2.0, y), Align2::RIGHT_TOP, value, small(), TEXT);
    };
    line(y, "Strike", elements_synth::mallet_zone(params[MALLET]));
    y += 14.0;
    if current == 2 {
        line(y, "Chord", elements_synth::chord_name(params[GEOMETRY]));
    }
    // Exciter mix: bow, blow, strike.
    let my = rect.bottom() - 20.0;
    for (k, (label, idx)) in [("Bow", BOW), ("Blow", BLOW), ("Strike", STRIKE)].iter().enumerate() {
        let w = (bw - 8.0) / 3.0;
        let r = Rect::from_min_size(pos2(rect.left() + k as f32 * (w + 4.0), my), vec2(w, 6.0));
        p.rect_filled(r, 2.0, PANEL3);
        let v = params[*idx].clamp(0.0, 1.0);
        p.rect_filled(Rect::from_min_size(r.min, vec2(r.width() * v, r.height())), 2.0, ACCENT);
        p.text(pos2(r.center().x, my + 8.0), Align2::CENTER_TOP, *label, small(), TEXT_DIM);
    }
}

/// The 24 Plaits models in their three banks, as buttons.
fn macro_model_picker(ui: &mut Ui, track_id: u64, params: &mut [f32], height: f32) {
    const BANKS: [(&str, Color32); 3] = [("Synth", PLAY), ("Classic", ACCENT), ("Noise & Drums", SELECT)];
    let (bw, bh) = (84.0, ((height - 18.0) / 8.0).clamp(14.0, 19.0));
    let (rect, _) = ui.allocate_exact_size(vec2(bw * 3.0 + 8.0, 16.0 + bh * 8.0), Sense::hover());
    let current = params[MODEL].round() as usize;
    for (b, (title, color)) in BANKS.iter().enumerate() {
        let x = rect.left() + b as f32 * (bw + 4.0);
        text(ui.painter(), pos2(x + 2.0, rect.top() + 6.0), Align2::LEFT_CENTER, *title, small(), shade(*color, 0.9));
        for k in 0..8 {
            let m = b * 8 + k;
            let r = Rect::from_min_size(pos2(x, rect.top() + 14.0 + k as f32 * bh), vec2(bw, bh - 2.0));
            if button(ui, r, Id::new(("macro_model", track_id, m)), macro_osc::MODELS[m], current == m, *color).clicked() {
                params[MODEL] = m as f32;
            }
        }
    }
}

/// The model and controls of an STK instrument: the knobs carry the
/// current model's own control names.
fn stk_controls(ui: &mut Ui, track_id: u64, family: krabton_engine::dsp::stk_synth::Family, params: &mut Vec<f32>, h: f32, mv: Option<&mut ModView>) {
    use krabton_engine::dsp::stk_synth as stk;
    let specs = stk::specs(family);
    if params.len() < specs.len() {
        return;
    }
    let models = stk::models(family);
    let k = stk::model_of(family, params);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Model").small().color(TEXT_DIM));
        egui::ComboBox::from_id_salt(("stk_model", track_id)).width(130.0).selected_text(models[k].name).show_ui(ui, |ui| {
            for (i, m) in models.iter().enumerate() {
                if ui.selectable_label(i == k, m.name).clicked() && i != k {
                    // A new model starts from its own defaults; voices and volume stay.
                    let (voices, volume) = (params[stk::VOICES], params[stk::VOLUME]);
                    *params = stk::defaults_for(family, i);
                    params[stk::VOICES] = voices;
                    params[stk::VOLUME] = volume;
                }
            }
        });
    });
    let ctl = stk::controls(&models[stk::model_of(family, params)]);
    let used = ctl.iter().take_while(|c| c.is_some()).count();
    let mut view: Vec<ParamSpec> = specs[..stk::CTL0 + used].to_vec();
    for (s, c) in view[stk::CTL0..].iter_mut().zip(ctl.iter().flatten()) {
        s.name = stk::short_name(c.name);
        s.default = c.default / 128.0;
    }
    param_grid_mod(ui, Id::new(("stk", track_id)), &view, &mut params[..stk::CTL0 + used], ACCENT, h - 72.0, mv);
}

fn effect_menu(app: &mut App, ui: &mut Ui, sel: TrackSel) {
    for cat in EffectKind::CATEGORIES {
        ui.menu_button(cat, |ui| {
            for k in EffectKind::ALL.into_iter().filter(|k| k.category() == cat) {
                if ui.button(k.name()).on_hover_text(k.description()).clicked() {
                    let dev = app.project.make_device(k);
                    if let Some(c) = app.chain_mut(sel) {
                        c.push(dev);
                    }
                    ui.close();
                }
            }
        });
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Marker {
    Start,
    End,
    LoopStart,
    LoopEnd,
}

/// The nearest frame to `f` where the left channel crosses zero (within
/// about 4 ms), so loops do not click.
fn snap_to_zero(s: &SampleRef, f: f64) -> f64 {
    let x = &s.left;
    let c = (f as usize).min(x.len().saturating_sub(2));
    let reach = (s.sample_rate as usize / 250).max(8);
    (0..reach)
        .flat_map(|d| [c.saturating_sub(d), (c + d).min(x.len() - 2)])
        .find(|&i| x[i] <= 0.0 && x[i + 1] > 0.0)
        .map(|i| i as f64)
        .unwrap_or(f)
}

/// The Sampler's sample editor: the waveform of the part a zone plays (or
/// the whole sample) with Start (S), End (E) and loop markers to drag.
/// Start, End and the loop points are the fractions in `params`, or the
/// zone's own loop (`own_loop`, from an SFZ) when it has one. Shift drags
/// finely, Ctrl+wheel zooms, Shift+wheel scrolls, double-click shows all.
fn wave_editor(ui: &mut Ui, id: Id, size: egui::Vec2, s: &SampleRef, region: (f64, f64), params: &mut [f32], mut own_loop: Option<&mut Option<(u32, u32)>>) -> egui::Response {
    let (full, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    // The waveform, with a line of times under it.
    let r = Rect::from_min_max(full.min, pos2(full.right(), full.bottom() - 15.0));
    let (r0, r1) = region;
    let len = (r1 - r0).max(1.0);
    let mut view: (f64, f64) = ui.ctx().memory_mut(|m| m.data.get_temp(id)).unwrap_or((r0, r1));
    let mut drag: Option<Marker> = ui.ctx().memory_mut(|m| m.data.get_temp(id.with("drag"))).unwrap_or(None);
    // Zoom and scroll.
    if resp.hovered() {
        let (zoom, scroll, ptr) = ui.input(|i| (i.zoom_delta(), i.raw_scroll_delta, i.pointer.hover_pos()));
        let span = view.1 - view.0;
        if (zoom - 1.0).abs() > 1e-3 {
            let at = ptr.map(|p| view.0 + ((p.x - r.left()) / r.width()) as f64 * span).unwrap_or(view.0 + span * 0.5);
            let new_span = (span / zoom as f64).clamp(64.0f64.min(len), len);
            let t = (at - view.0) / span;
            view.0 = at - t * new_span;
            view.1 = view.0 + new_span;
        }
        let pan = if ui.input(|i| i.modifiers.shift) { scroll.y + scroll.x } else { scroll.x };
        if pan.abs() > 0.0 {
            let shift = -(pan / r.width()) as f64 * (view.1 - view.0);
            view.0 += shift;
            view.1 += shift;
        }
        let span = view.1 - view.0;
        if view.0 < r0 {
            view = (r0, r0 + span);
        }
        if view.1 > r1 {
            view = ((r1 - span).max(r0), r1);
        }
    }
    if resp.double_clicked() {
        view = (r0, r1);
    }
    let span = (view.1 - view.0).max(1.0);
    let fx = |f: f64| r.left() + (((f - view.0) / span) as f32) * r.width();
    let xf = |x: f32| view.0 + ((x - r.left()) / r.width()) as f64 * span;

    // The waveform: peaks when zoomed out, the samples themselves when in.
    let p = ui.painter_at(r);
    rect_fill(&p, r, 3.0, BG);
    let mid = r.center().y;
    let half = r.height() * 0.45;
    let frames = s.frames();
    let block = krabton_engine::sample::PEAK_BLOCK as f64;
    for col in 0..r.width() as usize {
        let f0 = xf(r.left() + col as f32).max(0.0);
        let f1 = xf(r.left() + col as f32 + 1.0).max(f0 + 1.0);
        let (mut mn, mut mx) = (0.0f32, 0.0f32);
        if f1 - f0 >= block {
            let (a, b) = ((f0 / block) as usize, ((f1 / block) as usize).max((f0 / block) as usize + 1));
            for pk in s.peaks.get(a.min(s.peaks.len())..b.min(s.peaks.len())).unwrap_or(&[]) {
                mn = mn.min(pk.0);
                mx = mx.max(pk.1);
            }
        } else {
            let (a, b) = (f0 as usize, (f1.ceil() as usize).min(frames));
            for i in a.min(frames)..b.max(a + 1).min(frames) {
                let v = 0.5 * (s.left[i] + s.right[i]);
                mn = mn.min(v);
                mx = mx.max(v);
            }
        }
        let x = r.left() + col as f32 + 0.5;
        p.line_segment([pos2(x, mid - mx * half), pos2(x, mid - mn * half + 0.5)], Stroke::new(1.0_f32, ACCENT));
    }

    // Markers.
    let own = own_loop.as_deref().copied().flatten();
    let (start, end, looped) = sampler::bounds(params, r0, r1, own);
    let dim = Color32::from_black_alpha(150);
    p.rect_filled(Rect::from_min_max(r.min, pos2(fx(start).max(r.left()), r.bottom())), 0.0, dim);
    p.rect_filled(Rect::from_min_max(pos2(fx(end).min(r.right()), r.top()), r.max), 0.0, dim);
    let loop_col = Color32::from_rgb(0x7f, 0xb6, 0xe8);
    if let Some((a, b)) = looped {
        let band = Rect::from_min_max(pos2(fx(a), r.bottom() - 12.0), pos2(fx(b), r.bottom()));
        p.rect_filled(band, 0.0, loop_col.gamma_multiply(0.35));
        for (x, left) in [(fx(a), true), (fx(b), false)] {
            p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], Stroke::new(1.0_f32, loop_col));
            let tip = if left { vec![pos2(x, r.bottom() - 12.0), pos2(x + 8.0, r.bottom() - 6.0), pos2(x, r.bottom())] } else { vec![pos2(x, r.bottom() - 12.0), pos2(x - 8.0, r.bottom() - 6.0), pos2(x, r.bottom())] };
            p.add(egui::Shape::convex_polygon(tip, loop_col, Stroke::NONE));
        }
    }
    for (x, label) in [(fx(start), "S"), (fx(end), "E")] {
        p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], Stroke::new(1.5_f32, Color32::WHITE));
        let flag = if label == "S" { Rect::from_min_size(pos2(x, r.top()), vec2(11.0, 11.0)) } else { Rect::from_min_size(pos2(x - 11.0, r.top()), vec2(11.0, 11.0)) };
        p.rect_filled(flag, 1.0, Color32::WHITE);
        p.text(flag.center(), Align2::CENTER_CENTER, label, small(), TEXT_DARK);
    }
    let secs = |f: f64| f / s.sample_rate.max(1) as f64;
    let mut info = format!("Start {:.3} s   End {:.3} s", secs(start), secs(end));
    if let Some((a, b)) = looped {
        info += &format!("   Loop {:.3} - {:.3} s", secs(a), secs(b));
    }
    let below = ui.painter_at(full);
    below.text(pos2(full.left() + 2.0, full.bottom()), Align2::LEFT_BOTTOM, info, small(), TEXT_DIM);
    if span < len - 1.0 {
        below.text(pos2(full.right() - 2.0, full.bottom()), Align2::RIGHT_BOTTOM, format!("zoom {:.0}x", len / span), small(), TEXT_DIM);
    }

    // Dragging the markers.
    let near = |x: f32, y: f32| -> Option<Marker> {
        let mut c = vec![(Marker::Start, fx(start)), (Marker::End, fx(end))];
        if let Some((a, b)) = looped {
            // Loop handles sit at the bottom: there they win.
            let bottom = y > r.bottom() - 18.0;
            let loops = [(Marker::LoopStart, fx(a)), (Marker::LoopEnd, fx(b))];
            if bottom {
                c.splice(0..0, loops);
            } else {
                c.extend(loops);
            }
        }
        // The closest; on a tie the first in the list (min_by keeps the first).
        c.into_iter().filter(|(_, mx)| (mx - x).abs() < 9.0).min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs())).map(|(m, _)| m)
    };
    if let Some(hp) = resp.hover_pos() {
        if drag.is_some() || near(hp.x, hp.y).is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
    }
    if resp.drag_started() {
        drag = ui.input(|i| i.pointer.press_origin()).and_then(|o| near(o.x, o.y));
    }
    if let (Some(m), true) = (drag, resp.dragged()) {
        let fine = ui.input(|i| i.modifiers.shift);
        let current = match m {
            Marker::Start => start,
            Marker::End => end,
            Marker::LoopStart => looped.map(|l| l.0).unwrap_or(start),
            Marker::LoopEnd => looped.map(|l| l.1).unwrap_or(end),
        };
        let mut f = if fine {
            current + resp.drag_delta().x as f64 * span / r.width() as f64 * 0.1
        } else {
            resp.interact_pointer_pos().map(|p| xf(p.x)).unwrap_or(current)
        };
        f = f.clamp(r0, r1);
        let frac = |f: f64| ((f - r0) / len) as f32;
        let gap = (32.0 / len) as f32;
        match m {
            Marker::Start => params[sampler::START] = frac(f).clamp(0.0, (params[sampler::END] - gap).max(0.0)).min(0.99),
            Marker::End => params[sampler::END] = frac(f).clamp((params[sampler::START] + gap).max(0.01), 1.0),
            Marker::LoopStart | Marker::LoopEnd => {
                let f = if fine { f } else { snap_to_zero(s, f) };
                match own_loop.as_deref_mut() {
                    Some(Some(range)) => {
                        if m == Marker::LoopStart {
                            range.0 = (f as u32).min(range.1.saturating_sub(32));
                        } else {
                            range.1 = (f as u32).max(range.0 + 32);
                        }
                    }
                    _ => {
                        if m == Marker::LoopStart {
                            params[sampler::LOOP_START] = frac(f).clamp(0.0, (params[sampler::LOOP_END] - gap).max(0.0)).min(0.99);
                        } else {
                            params[sampler::LOOP_END] = frac(f).clamp((params[sampler::LOOP_START] + gap).max(0.01), 1.0);
                        }
                    }
                }
            }
        }
    }
    if resp.drag_stopped() {
        drag = None;
    }
    ui.ctx().memory_mut(|m| {
        m.data.insert_temp(id, view);
        m.data.insert_temp(id.with("drag"), drag);
    });
    resp
}

/// Zones over the keys (x) and velocities (y). Returns the zone under
/// `hover`.
fn draw_zone_map(ui: &mut Ui, r: Rect, zones: &[krabton_engine::SampleZone], sel: usize, hover: Option<egui::Pos2>) -> Option<usize> {
    let p = ui.painter();
    rect_fill(p, r, 3.0, BG);
    // Show the keys in use (at least two octaves), whole octaves.
    let lo = zones.iter().map(|z| z.lo_key).min().unwrap_or(0) as f32;
    let hi = zones.iter().map(|z| z.hi_key).max().unwrap_or(127) as f32 + 1.0;
    let mut from = (lo / 12.0).floor() * 12.0;
    let mut to = ((hi / 12.0).ceil() * 12.0).max(from + 24.0);
    if to > 128.0 {
        from = (from - (to - 128.0)).max(0.0);
        to = 128.0;
    }
    let x = move |k: f32| r.left() + (k - from) / (to - from) * r.width();
    let mut c = from;
    while c <= to {
        let xx = x(c);
        p.line_segment([pos2(xx, r.top()), pos2(xx, r.bottom())], Stroke::new(1.0_f32, LINE_SOFT));
        if c < to {
            p.text(pos2(xx + 2.0, r.bottom() - 1.0), Align2::LEFT_BOTTOM, note_name(c as u8), small(), TEXT_DIM);
        }
        c += 12.0;
    }
    let rect_of = |z: &krabton_engine::SampleZone| {
        Rect::from_min_max(pos2(x(z.lo_key as f32), r.bottom() - (z.hi_vel as f32 + 1.0) / 128.0 * r.height()), pos2(x(z.hi_key as f32 + 1.0), r.bottom() - z.lo_vel as f32 / 128.0 * r.height()))
    };
    let palette = [ACCENT, SELECT, PLAY, SOLO, REC];
    for (k, z) in zones.iter().enumerate() {
        let zr = rect_of(z).shrink2(vec2(0.0, 0.5));
        let c = palette[k % palette.len()];
        p.rect_filled(zr, 1.0, if k == sel { c } else { shade(c, 0.45) });
        let rx = x(z.root as f32 + 0.5);
        p.line_segment([pos2(rx, zr.top()), pos2(rx, zr.bottom())], Stroke::new(1.0_f32, Color32::WHITE.gamma_multiply(0.7)));
    }
    if let Some(z) = zones.get(sel) {
        p.rect_stroke(rect_of(z), 1.0, Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Inside);
    }
    let hp = hover?;
    (0..zones.len()).rev().find(|&k| rect_of(&zones[k]).contains(hp))
}

/// Keys, root, velocities, tuning and gain of one zone, in two rows.
fn zone_editor(ui: &mut Ui, z: &mut krabton_engine::SampleZone, k: usize) {
    let key = |ui: &mut Ui, v: &mut u8, tip: &str| {
        let mut n = *v as i32;
        if ui.add(egui::DragValue::new(&mut n).range(0..=127).custom_formatter(|v, _| note_name(v as u8))).on_hover_text(tip).changed() {
            *v = n as u8;
        }
    };
    let dim = |ui: &mut Ui, t: &str| ui.label(RichText::new(t).small().color(TEXT_DIM));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        dim(ui, &format!("Zone {}  keys", k + 1));
        key(ui, &mut z.lo_key, "Lowest key");
        key(ui, &mut z.hi_key, "Highest key");
        dim(ui, "root");
        key(ui, &mut z.root, "The key that plays the sample at its own pitch");
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        dim(ui, "velocity");
        ui.add(egui::DragValue::new(&mut z.lo_vel).range(0..=127)).on_hover_text("Lowest velocity");
        ui.add(egui::DragValue::new(&mut z.hi_vel).range(0..=127)).on_hover_text("Highest velocity");
        ui.add(egui::DragValue::new(&mut z.tune).range(-1200.0..=1200.0).suffix(" ct").speed(1.0)).on_hover_text("Tuning in cents");
        ui.add(egui::DragValue::new(&mut z.gain_db).range(-36.0..=12.0).suffix(" dB").speed(0.1)).on_hover_text("Level");
    });
    if z.hi_key < z.lo_key {
        z.hi_key = z.lo_key;
    }
    if z.hi_vel < z.lo_vel {
        z.hi_vel = z.lo_vel;
    }
}

fn draw_waveform(ui: &mut Ui, r: Rect, sample: Option<&SampleRef>, start: f32) {
    let p = ui.painter();
    rect_fill(p, r, 3.0, BG);
    let Some(s) = sample else {
        p.text(r.center(), Align2::CENTER_CENTER, "Drop a sample or an SFZ here", small(), TEXT_DIM);
        return;
    };
    let n = s.peaks.len().max(1);
    let mid = r.center().y;
    let half = r.height() * 0.45;
    for x in 0..r.width() as usize {
        let a = x * n / r.width() as usize;
        let b = ((x + 1) * n / r.width() as usize).max(a + 1).min(n);
        let (mut mn, mut mx) = (0.0f32, 0.0f32);
        for pk in &s.peaks[a.min(n - 1)..b] {
            mn = mn.min(pk.0);
            mx = mx.max(pk.1);
        }
        let xx = r.left() + x as f32;
        p.line_segment([pos2(xx, mid - mx * half), pos2(xx, mid - mn * half + 0.5)], Stroke::new(1.0_f32, ACCENT));
    }
    if start >= 0.0 {
        let sx = r.left() + start * r.width();
        p.line_segment([pos2(sx, r.top()), pos2(sx, r.bottom())], Stroke::new(1.5_f32, Color32::WHITE));
    }
}
