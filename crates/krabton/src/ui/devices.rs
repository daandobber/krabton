//! Device view: the instrument and effect chain of the selected track.

use crate::app::{App, BrowserItem, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, RichText, Sense, Stroke, Ui, UiBuilder};
use krabton_engine::dsp::drums::{self, DrumVoice};
use krabton_engine::dsp::effects::EffectKind;
use krabton_engine::midi_fx::MidiFxKind;
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
    fn chain_mut(&mut self, sel: TrackSel) -> Option<&mut Vec<Device>> {
        match sel {
            TrackSel::Track(i) => self.project.tracks.get_mut(i).map(|t| &mut t.devices),
            TrackSel::Return(j) => self.project.returns.get_mut(j).map(|t| &mut t.devices),
            TrackSel::Master => Some(&mut self.project.master.devices),
        }
    }

    pub(crate) fn ui_devices(&mut self, ui: &mut Ui, body: Rect) {
        let sel = self.sel_track;
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
        let id = Id::new(("device", dev.id));
        let mut enabled = dev.enabled;
        let mut params = dev.params.clone();
        let accent = if enabled { ACCENT } else { TEXT_DIM };
        let mut mv = self.mod_view(ModOwner::Device(dev_id));
        let (changed, action) = device_frame(ui, kind.name(), Some(&mut enabled), h, |ui| param_grid_mod(ui, id, kind.specs(), &mut params, accent, h - 24.0, mv.as_mut()));
        self.apply_mod_view(mv);
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
                    for k in EffectKind::ALL {
                        if ui.button(k.name()).clicked() {
                            let dev = self.project.make_device(k);
                            if let Some(c) = self.chain_mut(sel) {
                                c.push(dev);
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
        let (_, action) = device_frame(ui, title, None, h, |ui| match &mut new_inst {
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
            Instrument::Sampler { params, sample } => {
                ui.horizontal(|ui| {
                    if ui.button("Load...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("WAV", &["wav"]).pick_file() {
                            match SampleRef::load(&path) {
                                Ok(s) => *sample = Some(s),
                                Err(e) => eprintln!("{e}"),
                            }
                        }
                    }
                    let name = sample.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "No sample".into());
                    ui.label(RichText::new(name).color(TEXT_DIM));
                });
                ui.horizontal_top(|ui| {
                    let (r, resp) = ui.allocate_exact_size(vec2(180.0, h - 56.0), Sense::click());
                    draw_waveform(ui, r, sample.as_ref(), params[sampler::START]);
                    if let Some(item) = drop_payload::<BrowserItem>(&resp) {
                        if let BrowserItem::Sample(path) = &*item {
                            if let Ok(s) = SampleRef::load(path) {
                                *sample = Some(s);
                            }
                        }
                    }
                    if resp.dnd_hover_payload::<BrowserItem>().is_some() {
                        rect_line(ui.painter(), r, 2.0, 2.0, ACCENT);
                    }
                    param_grid_mod(ui, Id::new(("sampler", track_id)), sampler::SPECS, params, ACCENT, h - 48.0, mv.as_mut());
                });
            }
            Instrument::Drums { kit, pads } => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Kit").small().color(TEXT_DIM));
                    egui::ComboBox::from_id_salt(("drum_kit", track_id)).width(110.0).selected_text(kit.as_str()).show_ui(ui, |ui| {
                        for name in drums::kit_names() {
                            if ui.selectable_label(kit == name, *name).clicked() {
                                *kit = name.to_string();
                                *pads = drums::kit(name);
                            }
                        }
                    });
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
                                egui::ComboBox::from_id_salt(("pad_voice", track_id, k)).width(86.0).selected_text(pad.voice.name()).show_ui(ui, |ui| {
                                    for v in DrumVoice::ALL {
                                        if ui.selectable_label(pad.voice == v, v.name()).clicked() {
                                            pad.voice = v;
                                            if pad.sample.is_none() {
                                                pad.name = v.name().to_string();
                                            }
                                        }
                                    }
                                });
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
                            let mut vals = [pad.level_db, pad.tune, pad.decay, pad.tone, pad.pan];
                            if param_grid(ui, Id::new(("pad_params", track_id, k)), PAD_SPECS, &mut vals, ACCENT, h - 60.0) {
                                pad.level_db = vals[0];
                                pad.tune = vals[1];
                                pad.decay = vals[2];
                                pad.tone = vals[3];
                                pad.pan = vals[4];
                            }
                        }
                    });
                });
            }
        });
        self.apply_mod_view(mv);
        if matches!(action, Some(ChainActionKind::Remove)) {
            remove = true;
        }
        let t = &mut self.project.tracks[i];
        if remove {
            t.instrument = None;
        } else if new_inst != inst {
            if let (Instrument::Synth { preset: old, .. }, Instrument::Synth { preset: new, .. }) = (&inst, &new_inst) {
                if old != new && t.name == *old {
                    t.name = new.clone();
                }
            }
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

fn effect_menu(app: &mut App, ui: &mut Ui, sel: TrackSel) {
    for k in EffectKind::ALL {
        if ui.button(k.name()).on_hover_text(k.description()).clicked() {
            let dev = app.project.make_device(k);
            if let Some(c) = app.chain_mut(sel) {
                c.push(dev);
            }
            ui.close();
        }
    }
}

fn draw_waveform(ui: &mut Ui, r: Rect, sample: Option<&SampleRef>, start: f32) {
    let p = ui.painter();
    rect_fill(p, r, 3.0, BG);
    let Some(s) = sample else {
        p.text(r.center(), Align2::CENTER_CENTER, "Drop a WAV here", small(), TEXT_DIM);
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
    let sx = r.left() + start * r.width();
    p.line_segment([pos2(sx, r.top()), pos2(sx, r.bottom())], Stroke::new(1.5_f32, Color32::WHITE));
}
