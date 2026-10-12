//! The Rack view: every track's devices at once, MIDI effects and
//! sequencers, instrument and audio effects, a track per row with its
//! devices side by side, or (with the axes swapped) a track per column with
//! its devices under each other. Returns and the master come last. Each
//! track starts with its mixer (volume, pan, sends, mute, solo, arm) and
//! buttons to add a sequencer or MIDI effect, an audio effect or an
//! instrument; right-click its name to duplicate or delete the track.

use crate::app::{App, Deferred, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, RichText, Sense, Ui};
use krabton_engine::modulation::ModTarget;
use krabton_engine::{fader_gain, TrackKind};

/// The mixer strip's width in rows, and its height on top of a column.
const STRIP_W: f32 = 168.0;
const STRIP_H: f32 = 236.0;

impl App {
    pub(crate) fn ui_device_rack(&mut self, ui: &mut Ui) {
        let columns = self.prefs.layout.rack_columns;
        let h = crate::prefs::Layout::or(self.prefs.layout.rack_h, 230.0);
        // The bar on top: the axes and the height of a row of devices.
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            ui.label(RichText::new("Rack").strong().color(TEXT));
            ui.add_space(8.0);
            let label = if columns { "Tracks in columns" } else { "Tracks in rows" };
            if ui.add(egui::Button::new(RichText::new(label).color(TEXT)).fill(PANEL3)).on_hover_text("Swap the axes: a track per row with its devices side by side, or a track per column with its devices under each other").clicked() {
                self.prefs.layout.rack_columns = !columns;
                self.layout_dirty = true;
            }
            ui.add_space(8.0);
            let shown = self.prefs.layout.rack_detail;
            let label = if shown { "Hide clip view" } else { "Show clip view" };
            if ui.add(egui::Button::new(RichText::new(label).color(TEXT)).fill(PANEL3)).on_hover_text("The clip editor, devices, modulation and grid at the bottom (Ctrl+Alt+L)").clicked() {
                self.toggle_detail();
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Device height").small().color(TEXT_DIM)).on_hover_text("For devices you did not resize: drag the grip at the bottom of a device for its own height");
            let mut hh = h;
            if ui.add(egui::DragValue::new(&mut hh).range(140.0..=600.0).speed(1.0).suffix(" px")).changed() {
                self.prefs.layout.rack_h = hh;
                self.layout_dirty = true;
            }
        });
        let mut sels: Vec<TrackSel> = (0..self.project.tracks.len()).map(TrackSel::Track).collect();
        sels.extend((0..self.project.returns.len()).map(TrackSel::Return));
        sels.push(TrackSel::Master);
        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            if columns {
                ui.horizontal_top(|ui| {
                    for sel in sels {
                        ui.vertical(|ui| {
                            self.rack_strip(ui, sel, vec2(STRIP_W + 20.0, STRIP_H));
                            self.device_chain(ui, sel, h, true);
                        });
                        ui.add_space(10.0);
                    }
                });
            } else {
                ui.vertical(|ui| {
                    for sel in sels {
                        ui.horizontal_top(|ui| {
                            let row = self.rack_row_height(sel, h);
                            self.rack_strip(ui, sel, vec2(STRIP_W, (row + 30.0).max(STRIP_H)));
                            self.device_chain(ui, sel, h, true);
                        });
                        ui.add_space(6.0);
                    }
                    // New tracks.
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(RichText::new("+ MIDI track").color(TEXT)).fill(PANEL3)).on_hover_text("A new MIDI track with a synth (Ctrl+Shift+T)").clicked() {
                            let t = self.add_midi_track(Some(krabton_engine::Instrument::synth("Init")));
                            self.sel_track = TrackSel::Track(t);
                        }
                        if ui.add(egui::Button::new(RichText::new("+ Audio track").color(TEXT)).fill(PANEL3)).on_hover_text("A new audio track (Ctrl+T)").clicked() {
                            let t = self.add_audio_track();
                            self.sel_track = TrackSel::Track(t);
                        }
                    });
                });
            }
        });
    }

    /// The height of a track's row: its tallest device.
    fn rack_row_height(&self, sel: TrackSel, h: f32) -> f32 {
        let mut keys: Vec<u64> = Vec::new();
        match sel {
            TrackSel::Track(i) => {
                let t = &self.project.tracks[i];
                keys.extend(t.midi_devices.iter().map(|d| d.id));
                if t.instrument.is_some() {
                    keys.push(t.id);
                }
                keys.extend(t.devices.iter().map(|d| d.id));
            }
            TrackSel::Return(j) => keys.extend(self.project.returns[j].devices.iter().map(|d| d.id)),
            TrackSel::Master => keys.extend(self.project.master.devices.iter().map(|d| d.id)),
        }
        keys.iter().map(|k| self.rack_height(*k, h)).fold(h, f32::max)
    }

    /// A track's mixer at the start of its row (or on top of its column):
    /// name, mute, solo and arm, volume with its meter, pan, the sends, and
    /// buttons to add devices.
    fn rack_strip(&mut self, ui: &mut Ui, sel: TrackSel, size: egui::Vec2) {
        let _accent = AccentScope::new(self.track_accent(sel));
        let (r, _) = ui.allocate_exact_size(size, Sense::hover());
        let (name, color, key) = match sel {
            TrackSel::Track(i) => (self.project.tracks[i].name.clone(), rgb(self.project.tracks[i].color), self.project.tracks[i].id),
            TrackSel::Return(j) => (self.project.returns[j].name.clone(), rgb(self.project.returns[j].color), self.project.returns[j].id),
            TrackSel::Master => ("Master".to_string(), accent_now(), u64::MAX),
        };
        let selected = self.sel_track == sel;
        let p = ui.painter().clone();
        rect_fill(&p, r, 4.0, if selected { shade(PANEL2, 1.2) } else { PANEL2 });
        rect_fill(&p, Rect::from_min_size(r.min, vec2(r.width(), 3.0)), 1.0, color);
        if selected {
            rect_line(&p, r, 4.0, 1.0, shade(color, 0.8));
        }
        // The name: click to select, right-click for the track itself.
        let name_r = Rect::from_min_size(r.min + vec2(0.0, 3.0), vec2(r.width(), 24.0));
        let nresp = ui.interact(name_r, Id::new(("rack_name", key)), Sense::click());
        p.text(pos2(r.left() + 8.0, name_r.center().y), Align2::LEFT_CENTER, &name, FontId::proportional(14.0), if selected { Color32::WHITE } else { TEXT });
        if nresp.clicked() {
            self.sel_track = sel;
        }
        if let TrackSel::Track(i) = sel {
            nresp.context_menu(|ui| {
                if ui.button("Duplicate track").clicked() {
                    self.deferred.push(Deferred::DuplicateTrack(i));
                    ui.close();
                }
                if ui.button("Delete track").clicked() {
                    self.deferred.push(Deferred::DeleteTrack(i));
                    ui.close();
                }
            });
        }
        // Mute (the activator), solo, arm.
        let by = r.top() + 30.0;
        let bw = (r.width() - 20.0) / 3.0;
        let b = |k: f32| Rect::from_min_size(pos2(r.left() + 6.0 + k * (bw + 4.0), by), vec2(bw, 18.0));
        match sel {
            TrackSel::Track(i) => {
                let (mute, solo, arm, kind) = (self.project.tracks[i].mixer.mute, self.project.tracks[i].mixer.solo, self.project.tracks[i].arm, self.project.tracks[i].kind);
                if button(ui, b(0.0), Id::new(("rack_act", key)), &format!("{}", i + 1), !mute, accent_now()).on_hover_text("Track on/off (mute)").clicked() {
                    self.project.tracks[i].mixer.mute = !mute;
                }
                if button(ui, b(1.0), Id::new(("rack_solo", key)), "S", solo, SOLO).on_hover_text("Solo (Ctrl+click for more than one)").clicked() {
                    if !ui.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                        for (k, t) in self.project.tracks.iter_mut().enumerate() {
                            if k != i {
                                t.mixer.solo = false;
                            }
                        }
                    }
                    self.project.tracks[i].mixer.solo = !solo;
                }
                if kind == TrackKind::Midi || kind == TrackKind::Audio {
                    let resp = button(ui, b(2.0), Id::new(("rack_arm", key)), "", arm, REC).on_hover_text("Arm for recording");
                    rec_icon(ui.painter(), b(2.0).center(), 8.0, if arm { TEXT_DARK } else { REC });
                    if resp.clicked() {
                        self.project.tracks[i].arm = !arm;
                    }
                }
            }
            TrackSel::Return(j) => {
                let mute = self.project.returns[j].mixer.mute;
                if button(ui, b(0.0), Id::new(("rack_act", key)), &format!("{}", (b'A' + j as u8) as char), !mute, accent_now()).on_hover_text("Return on/off").clicked() {
                    self.project.returns[j].mixer.mute = !mute;
                }
            }
            TrackSel::Master => {}
        }
        // Volume with its meter on the left.
        let buttons_h = if matches!(sel, TrackSel::Track(_)) { 48.0 } else { 26.0 };
        let area = Rect::from_min_max(pos2(r.left() + 6.0, by + 26.0), pos2(r.left() + 52.0, r.bottom() - buttons_h - 18.0));
        let fr = Rect::from_min_size(area.min, vec2(22.0, area.height()));
        let (mut vol, meter_lr) = match sel {
            TrackSel::Track(i) => (self.project.tracks[i].mixer.volume_db, self.meters.get(i).copied().unwrap_or((0.0, 0.0))),
            TrackSel::Return(j) => (self.project.returns[j].mixer.volume_db, self.return_meters.get(j).copied().unwrap_or((0.0, 0.0))),
            TrackSel::Master => (self.project.master.volume_db, self.master_meter),
        };
        if fader(ui, fr, Id::new(("rack_vol", key)), &mut vol, color) {
            match sel {
                TrackSel::Track(i) => self.project.tracks[i].mixer.volume_db = vol,
                TrackSel::Return(j) => self.project.returns[j].mixer.volume_db = vol,
                TrackSel::Master => self.project.master.volume_db = vol,
            }
        }
        // What the playing clip's envelopes do now, in red.
        let now = |app: &App, target: fn(u64) -> ModTarget| match sel {
            TrackSel::Track(i) => app.envelope_now(i, target(app.project.tracks[i].id)),
            _ => None,
        };
        if let Some(v) = now(self, ModTarget::Volume) {
            fader_auto_mark(&p, fr, v);
        }
        meter(&p, Rect::from_min_size(pos2(fr.right() + 5.0, area.top() + 4.0), vec2(9.0, area.height() - 8.0)), meter_lr.0, meter_lr.1);
        let db = if fader_gain(vol) == 0.0 { "-inf".to_string() } else { format!("{vol:.1} dB") };
        p.text(pos2(area.left(), area.bottom() + 3.0), Align2::LEFT_TOP, db, small(), TEXT_DIM);
        // Pan and the sends on the right.
        let kx = r.left() + 66.0;
        let pan = match sel {
            TrackSel::Track(i) => Some(self.project.tracks[i].mixer.pan),
            TrackSel::Return(j) => Some(self.project.returns[j].mixer.pan),
            TrackSel::Master => None,
        };
        if let Some(pan) = pan {
            let pr = Rect::from_min_size(pos2(kx, by + 26.0), vec2(28.0, 28.0));
            let (resp, new) = knob(ui, pr, Id::new(("rack_pan", key)), (pan + 1.0) * 0.5, 0.5, true, accent_now());
            if let Some(n) = new {
                let v = (n * 2.0 - 1.0).clamp(-1.0, 1.0);
                match sel {
                    TrackSel::Track(i) => self.project.tracks[i].mixer.pan = v,
                    TrackSel::Return(j) => self.project.returns[j].mixer.pan = v,
                    TrackSel::Master => {}
                }
            }
            if let Some(v) = now(self, ModTarget::Pan) {
                knob_auto_mark(&p, pr, v);
            }
            resp.on_hover_text("Pan");
            p.text(pos2(pr.right() + 6.0, pr.center().y - 6.0), Align2::LEFT_CENTER, "Pan", small(), TEXT_DIM);
            p.text(pos2(pr.right() + 6.0, pr.center().y + 7.0), Align2::LEFT_CENTER, super::session::pan_text(pan), small(), TEXT);
        }
        if let TrackSel::Track(i) = sel {
            let n = self.project.returns.len().min(6);
            for j in 0..n {
                let c = pos2(kx + 13.0 + (j % 3) as f32 * 32.0, by + 82.0 + (j / 3) as f32 * 40.0);
                let kr = Rect::from_center_size(c, vec2(24.0, 24.0));
                if kr.bottom() + 12.0 > r.bottom() - buttons_h {
                    break;
                }
                let v = self.project.tracks[i].mixer.sends.get(j).copied().unwrap_or(0.0);
                let (resp, new) = knob(ui, kr, Id::new(("rack_send", key, j)), v, 0.0, false, shade(rgb(self.project.returns[j].color), 0.9));
                if let Some(nv) = new {
                    if let Some(s) = self.project.tracks[i].mixer.sends.get_mut(j) {
                        *s = nv;
                    }
                }
                if let Some(now) = self.envelope_now(i, ModTarget::Send { track: self.project.tracks[i].id, send: j as u16 }) {
                    knob_auto_mark(&p, kr, now);
                }
                resp.on_hover_text(format!("Send to {}: {:.0} %", self.project.returns[j].name, v * 100.0));
                p.text(pos2(c.x, kr.bottom() + 6.0), Align2::CENTER_CENTER, (b'A' + j as u8) as char, small(), TEXT_DIM);
            }
        }
        // Adding devices.
        let row1 = Rect::from_min_max(pos2(r.left() + 6.0, r.bottom() - buttons_h), pos2(r.right() - 6.0, r.bottom() - buttons_h + 20.0));
        let at = pos2(r.right() + 4.0, r.top());
        match sel {
            TrackSel::Track(i) => {
                let half = (row1.width() - 4.0) / 2.0;
                let midi_menu = Id::new(("rack_midi_menu", key));
                let fx_menu = Id::new(("rack_fx_menu", key));
                let inst_menu = Id::new(("rack_inst_menu", key));
                let midi = self.project.tracks[i].kind == TrackKind::Midi;
                if midi {
                    let mr = Rect::from_min_size(row1.min, vec2(half, 20.0));
                    if button(ui, mr, Id::new(("rack_add_midi", key)), "+ MIDI", false, SELECT).on_hover_text("Add a sequencer or MIDI effect").clicked() {
                        ui.ctx().memory_mut(|m| m.data.insert_temp(midi_menu, true));
                    }
                }
                let fr = Rect::from_min_size(pos2(row1.left() + half + 4.0, row1.top()), vec2(half, 20.0));
                if button(ui, fr, Id::new(("rack_add_fx", key)), "+ Effect", false, accent_now()).on_hover_text("Add an audio effect").clicked() {
                    ui.ctx().memory_mut(|m| m.data.insert_temp(fx_menu, true));
                }
                if midi {
                    let ir = Rect::from_min_size(pos2(row1.left(), row1.bottom() + 4.0), vec2(row1.width(), 20.0));
                    let label = if self.project.tracks[i].instrument.is_some() { "Change instrument" } else { "+ Instrument" };
                    if button(ui, ir, Id::new(("rack_add_inst", key)), label, false, PLAY).on_hover_text("Pick an instrument, or one of its presets").clicked() {
                        ui.ctx().memory_mut(|m| m.data.insert_temp(inst_menu, true));
                    }
                }
                self.midi_fx_popup(ui, i, midi_menu, at);
                self.effect_popup(ui, sel, fx_menu, at);
                self.instrument_popup(ui, i, inst_menu, at);
            }
            _ => {
                let fx_menu = Id::new(("rack_fx_menu", key));
                if button(ui, row1, Id::new(("rack_add_fx", key)), "+ Effect", false, accent_now()).on_hover_text("Add an audio effect").clicked() {
                    ui.ctx().memory_mut(|m| m.data.insert_temp(fx_menu, true));
                }
                self.effect_popup(ui, sel, fx_menu, at);
            }
        }
    }
}
