//! The Studio (the Rack): every track's devices at once, MIDI effects and
//! sequencers, instrument and audio effects, a track per row with its
//! devices side by side (wrapping onto more lines as wide as the window, so
//! a track is seen whole; or on one long line), or (with the axes swapped) a
//! track per column with its devices under each other. Returns and the master come last. Each
//! track starts with its mixer (volume, pan, sends, mute, solo, arm) and
//! its clock (Sync, or a tempo of its own with Tap) and buttons to add a
//! sequencer or MIDI effect, an audio effect or an instrument; right-click
//! its name to duplicate or delete the track.

use crate::app::{App, Deferred, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, RichText, Sense, Ui};
use krabton_engine::modulation::ModTarget;
use krabton_engine::{fader_gain, TrackKind};

/// The mixer strip's width in rows, and its height on top of a column.
const STRIP_W: f32 = 258.0;
const STRIP_H: f32 = 236.0;
/// The column of a track's patterns (its clips) in its strip.
const PADS_W: f32 = 88.0;

impl App {
    pub(crate) fn ui_device_rack(&mut self, ui: &mut Ui) {
        let columns = self.prefs.layout.rack_columns;
        let wrap = !columns && !self.prefs.layout.rack_one_line;
        let h = crate::prefs::Layout::or(self.prefs.layout.rack_h, 230.0);
        self.studio_bar(ui, columns, h);
        let mut sels: Vec<TrackSel> = (0..self.project.tracks.len()).map(TrackSel::Track).collect();
        sels.extend((0..self.project.returns.len()).map(TrackSel::Return));
        sels.push(TrackSel::Master);
        // Where a wrapped row's devices end: the window's right edge.
        let view_right = ui.max_rect().right() - 14.0;
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
                            if wrap {
                                // Devices side by side, onto the next line where the window ends.
                                let wrap_w = (view_right - ui.cursor().left()).max(300.0);
                                ui.allocate_ui_with_layout(vec2(wrap_w, 0.0), egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true), |ui| {
                                    ui.set_max_width(wrap_w);
                                    self.device_chain(ui, sel, h, true);
                                });
                            } else {
                                self.device_chain(ui, sel, h, true);
                            }
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
        // A clean channel: its colour along the top and in the name's band.
        let name_r = Rect::from_min_size(r.min, vec2(r.width(), 28.0));
        rect_fill(&p, r, 6.0, if selected { shade(PANEL2, 1.12) } else { PANEL2 });
        p.rect_filled(name_r, egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 }, mix(PANEL2, color, if selected { 0.3 } else { 0.18 }));
        p.rect_filled(Rect::from_min_size(r.min, vec2(r.width(), 3.0)), egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 }, color);
        if selected {
            rect_line(&p, r, 6.0, 1.0, shade(color, 0.9));
        }
        // The name: click to select, right-click for the track itself; what plays it beside.
        let nresp = ui.interact(name_r, Id::new(("rack_name", key)), Sense::click());
        let kind_name = match sel {
            TrackSel::Track(i) => self.project.tracks[i].instrument.as_ref().map(|x| x.kind_name()).unwrap_or(if self.project.tracks[i].kind == TrackKind::Audio { "Audio" } else { "" }),
            TrackSel::Return(_) => "Return",
            TrackSel::Master => "",
        };
        let kind_w = p.layout_no_wrap(kind_name.to_string(), small(), TEXT_DIM).size().x;
        p.text(pos2(r.right() - 10.0, name_r.center().y + 1.0), Align2::RIGHT_CENTER, kind_name, small(), TEXT_DIM);
        let name_clip = Rect::from_min_max(name_r.min, pos2(r.right() - kind_w - 18.0, name_r.bottom()));
        p.with_clip_rect(name_clip).text(pos2(r.left() + 10.0, name_r.center().y + 1.0), Align2::LEFT_CENTER, &name, FontId::proportional(15.0), if selected { Color32::WHITE } else { TEXT });
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
        // A track's patterns: its clips, a pad per scene; the mixer beside them.
        let m = if let TrackSel::Track(i) = sel {
            let pads = Rect::from_min_max(pos2(r.left() + 6.0, r.top() + 34.0), pos2(r.left() + 6.0 + PADS_W, r.bottom() - 8.0));
            self.pattern_pads(ui, i, pads);
            p.line_segment([pos2(pads.right() + 5.0, pads.top()), pos2(pads.right() + 5.0, pads.bottom())], egui::Stroke::new(1.0_f32, LINE_SOFT));
            Rect::from_min_max(pos2(pads.right() + 6.0, r.top()), r.max)
        } else {
            Rect::from_min_max(pos2(r.left() + PADS_W + 10.0, r.top()), r.max)
        };
        // Mute (the activator), solo, arm.
        let by = r.top() + 30.0;
        let bw = (m.width() - 20.0) / 3.0;
        let b = |k: f32| Rect::from_min_size(pos2(m.left() + 6.0 + k * (bw + 4.0), by), vec2(bw, 18.0));
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
                    let resp = button(ui, b(2.0), Id::new(("rack_arm", key)), "", arm, REC).on_hover_text("Arm for recording (Ctrl+click for more)");
                    rec_icon(ui.painter(), b(2.0).center(), 8.0, if arm { TEXT_DARK } else { REC });
                    if resp.clicked() {
                        let multi = ui.input(|k| k.modifiers.ctrl || k.modifiers.command);
                        self.click_arm(i, multi);
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
        let buttons_h = if matches!(sel, TrackSel::Track(_)) { 72.0 } else { 26.0 };
        let area = Rect::from_min_max(pos2(m.left() + 6.0, by + 26.0), pos2(m.left() + 52.0, r.bottom() - buttons_h - 18.0));
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
        let kx = m.left() + 66.0;
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
        let row1 = Rect::from_min_max(pos2(m.left() + 6.0, r.bottom() - buttons_h), pos2(m.right() - 6.0, r.bottom() - buttons_h + 20.0));
        let at = pos2(m.right() + 4.0, r.top());
        match sel {
            TrackSel::Track(i) => {
                // Its clock, under the buttons.
                self.track_clock(ui, i, Rect::from_min_size(pos2(row1.left(), r.bottom() - 24.0), vec2(row1.width(), 18.0)));
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

    /// The studio's bar: the song (a button per scene, lit while it plays,
    /// blinking while it waits for its bar), stop, and the settings.
    fn studio_bar(&mut self, ui: &mut Ui, columns: bool, h: f32) {
        let status = self.engine.status.clone();
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("SONG").strong().color(TEXT_DIM)).on_hover_text("A button per scene: click to play it on every track");
            ui.add_space(4.0);
            let scenes: Vec<String> = self.project.scenes.iter().map(|s| s.name.clone()).collect();
            for (s, name) in scenes.iter().enumerate() {
                let (playing, queued) = scene_state(&self.project, &status, s);
                let (r, resp) = ui.allocate_exact_size(vec2(74.0, 34.0), Sense::click());
                let p = ui.painter();
                let lit = playing || (queued && super::session::blink());
                let fill = if lit { ACCENT } else if resp.hovered() { shade(PANEL3, 1.3) } else { PANEL3 };
                p.rect_filled(r.translate(vec2(0.0, 2.0)), 4.0, Color32::from_black_alpha(110));
                p.rect_filled(r, 4.0, fill);
                p.rect_filled(Rect::from_min_size(r.min + vec2(2.0, 2.0), vec2(r.width() - 4.0, 9.0)), 3.0, Color32::from_white_alpha(18));
                let ink = text_on(fill);
                p.text(pos2(r.left() + 8.0, r.center().y), Align2::LEFT_CENTER, format!("{}", s + 1), crate::skin::Font::Block.id(17.0), ink);
                let nr = Rect::from_min_max(pos2(r.left() + 26.0, r.top()), pos2(r.right() - 4.0, r.bottom()));
                p.with_clip_rect(nr).text(nr.left_center(), Align2::LEFT_CENTER, name, small(), ink);
                if resp.clicked() {
                    self.cmd(krabton_engine::Command::LaunchScene(s));
                    self.sel_scene = s;
                }
                resp.on_hover_text(format!("Scene {}: {name}. Click to play it on every track", s + 1));
            }
            ui.add_space(6.0);
            let (r, resp) = ui.allocate_exact_size(vec2(40.0, 34.0), Sense::click());
            let p = ui.painter();
            p.rect_filled(r, 4.0, if resp.hovered() { shade(PANEL3, 1.3) } else { PANEL3 });
            stop_icon(p, r.center(), 12.0, TEXT);
            if resp.on_hover_text("Stop all clips").clicked() {
                self.cmd(krabton_engine::Command::StopAllClips);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                let mut hh = h;
                if ui.add(egui::DragValue::new(&mut hh).range(140.0..=600.0).speed(1.0).suffix(" px")).on_hover_text("For devices you did not resize: drag the grip at the bottom of a device for its own height").changed() {
                    self.prefs.layout.rack_h = hh;
                    self.layout_dirty = true;
                }
                ui.label(RichText::new("Height").small().color(TEXT_DIM));
                let shown = self.prefs.layout.rack_detail;
                if ui.add(egui::Button::new(RichText::new(if shown { "Hide clip view" } else { "Show clip view" }).color(TEXT)).fill(PANEL3)).on_hover_text("The clip editor, devices, modulation and grid at the bottom (Ctrl+Alt+L)").clicked() {
                    self.toggle_detail();
                }
                // Three ways round: rows that wrap, rows on one line, columns.
                let one_line = self.prefs.layout.rack_one_line;
                let label = match (columns, one_line) {
                    (true, _) => "Tracks in columns",
                    (false, false) => "Tracks in rows, wrapped",
                    (false, true) => "Tracks in rows, one line",
                };
                let tip = "How the tracks lie: a row each with its devices side by side wrapping onto more lines (a track seen whole), a row each on one long line (scroll sideways), or a column each with its devices under each other. Click for the next";
                if ui.add(egui::Button::new(RichText::new(label).color(TEXT)).fill(PANEL3)).on_hover_text(tip).clicked() {
                    match (columns, one_line) {
                        (false, false) => self.prefs.layout.rack_one_line = true,
                        (false, true) => self.prefs.layout.rack_columns = true,
                        (true, _) => {
                            self.prefs.layout.rack_columns = false;
                            self.prefs.layout.rack_one_line = false;
                        }
                    }
                    self.layout_dirty = true;
                }
            });
        });
        ui.add_space(4.0);
    }

    /// A track's patterns: a pad per scene with the clip in it (lit while it
    /// plays, blinking while it waits). Click a pad to play it (an empty one
    /// stops the track), double-click to edit it (an empty one gets a clip).
    fn pattern_pads(&mut self, ui: &mut Ui, i: usize, area: Rect) {
        let status = self.engine.status.clone();
        let Some(ts) = status.tracks.get(i) else { return };
        let playing = ts.playing_slot.load(std::sync::atomic::Ordering::Relaxed);
        let queued = ts.queued_slot.load(std::sync::atomic::Ordering::Relaxed);
        let n = self.project.scenes.len().max(1);
        // One column where they fit (the names read), else two.
        let cols = if n as f32 * 21.0 <= area.height() - 16.0 { 1usize } else { 2 };
        let rows = n.div_ceil(cols);
        let ph = ((area.height() - 16.0) / rows as f32 - 3.0).clamp(14.0, 26.0);
        let pw = (area.width() - 3.0) / cols as f32;
        let p = ui.painter().clone();
        p.text(pos2(area.left() + 1.0, area.top() + 5.0), Align2::LEFT_CENTER, "Patterns", small(), TEXT_DIM);
        let track_id = self.project.tracks[i].id;
        let midi = self.project.tracks[i].kind == TrackKind::Midi;
        for s in 0..n {
            let r = Rect::from_min_size(pos2(area.left() + (s % cols) as f32 * (pw + 3.0), area.top() + 14.0 + (s / cols) as f32 * (ph + 3.0)), vec2(pw, ph));
            if r.bottom() > area.bottom() + 1.0 {
                break;
            }
            let clip = self.project.tracks[i].slots.get(s).and_then(|c| c.as_ref()).map(|c| (c.name.clone(), c.color, c.sound.is_some()));
            let resp = ui.interact(r, Id::new(("pattern", track_id, s)), Sense::click());
            let on = playing == s as i32;
            let waiting = queued == s as i32;
            match &clip {
                Some((name, color, keeps)) => {
                    let base = rgb(*color);
                    let lit = on || (waiting && super::session::blink());
                    let fill = if lit { base } else { shade(base, if resp.hovered() { 0.85 } else { 0.72 }) };
                    rect_fill(&p, r, 3.0, fill);
                    let ink = text_on(fill);
                    // The playing one shows its triangle, the others their scene.
                    let lead = Rect::from_min_size(r.min, vec2(16.0, r.height()));
                    if on || waiting {
                        rect_fill(&p, lead.shrink(2.0), 2.0, shade(base, 0.3));
                        play_icon(&p, lead.center(), 8.0, if on || super::session::blink() { PLAY } else { TEXT_DARK });
                    } else {
                        p.text(lead.center(), Align2::CENTER_CENTER, format!("{}", s + 1), egui::FontId::proportional(9.5), shade(ink, 0.75));
                    }
                    let tr = Rect::from_min_max(pos2(lead.right() + 1.0, r.top()), pos2(r.right() - if *keeps { 13.0 } else { 3.0 }, r.bottom()));
                    p.with_clip_rect(tr).text(tr.left_center(), Align2::LEFT_CENTER, name, egui::FontId::proportional((ph * 0.46).clamp(9.0, 11.5)), ink);
                    if *keeps {
                        super::session::sound_mark(&p, pos2(r.right() - 7.0, r.center().y), ink);
                    }
                    if resp.clicked() {
                        self.cmd(krabton_engine::Command::LaunchClip { track: track_id, slot: s });
                        self.select_clip(crate::app::ClipSel::Slot { track: i, scene: s });
                        self.detail = crate::app::DetailTab::Devices;
                    }
                    if resp.double_clicked() {
                        self.select_clip(crate::app::ClipSel::Slot { track: i, scene: s });
                        if !self.prefs.layout.rack_detail {
                            self.toggle_detail();
                        }
                    }
                    resp.on_hover_text(format!("{name} (scene {}): click to play it, double-click to edit it", s + 1));
                }
                None => {
                    rect_fill(&p, r, 3.0, if resp.hovered() { shade(SLOT, 1.2) } else { SLOT });
                    p.text(pos2(r.left() + 8.0, r.center().y), Align2::CENTER_CENTER, format!("{}", s + 1), egui::FontId::proportional(9.5), TEXT_DIM);
                    if resp.clicked() {
                        self.cmd(krabton_engine::Command::StopTrack { track: track_id });
                    }
                    if resp.double_clicked() && midi {
                        self.create_midi_clip(i, s);
                        self.select_clip(crate::app::ClipSel::Slot { track: i, scene: s });
                        if !self.prefs.layout.rack_detail {
                            self.toggle_detail();
                        }
                    }
                    resp.on_hover_text(if midi { "Empty: click to stop the track, double-click for a new clip" } else { "Empty: click to stop the track" });
                }
            }
        }
    }
}

/// Whether scene `s` plays (every track with a clip in it plays that clip)
/// and whether it waits for its bar.
fn scene_state(project: &krabton_engine::Project, status: &krabton_engine::engine::EngineStatus, s: usize) -> (bool, bool) {
    let mut any = false;
    let mut all = true;
    let mut queued = false;
    for (i, t) in project.tracks.iter().enumerate() {
        let Some(ts) = status.tracks.get(i) else { break };
        if matches!(t.slots.get(s), Some(Some(_))) {
            any = true;
            all &= ts.playing_slot.load(std::sync::atomic::Ordering::Relaxed) == s as i32;
            queued |= ts.queued_slot.load(std::sync::atomic::Ordering::Relaxed) == s as i32;
        }
    }
    (any && all, queued)
}
