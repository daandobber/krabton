//! Top transport bar, menus and the bottom status bar.

use crate::app::{App, View};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, RichText, Sense, Ui};
use krabton_engine::{Command, Instrument, Quantize};
use std::sync::atomic::Ordering;
use std::time::Instant;

/// Bars.beats.sixteenths, 1-based like Live.
pub fn format_pos(beats: f64, bpb: u32) -> String {
    let beats = beats.max(0.0);
    let bpb = bpb.max(1) as f64;
    let bar = (beats / bpb).floor();
    let beat = (beats - bar * bpb).floor();
    let six = ((beats - beats.floor()) * 4.0).floor();
    format!("{}. {}. {}", bar as i64 + 1, beat as i64 + 1, six as i64 + 1)
}

impl App {
    pub(crate) fn ui_transport(&mut self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            self.ui_menus(ui);
            ui.separator();

            // Tempo, tap and meter.
            ui.label(RichText::new("BPM").color(TEXT_DIM).small());
            ui.add(egui::DragValue::new(&mut self.project.bpm).range(20.0..=999.0).speed(0.2).fixed_decimals(2))
                .on_hover_text("Tempo. Drag or double-click to type.");
            if ui.button("TAP").on_hover_text("Tap tempo").clicked() {
                self.tap_tempo();
            }
            ui.add(egui::DragValue::new(&mut self.project.beats_per_bar).range(1..=16).suffix(" / 4")).on_hover_text("Beats per bar");
            let metro = self.project.metronome;
            if ui.add(egui::Button::new(RichText::new("Click").color(if metro { TEXT_DARK } else { TEXT })).fill(if metro { ACCENT } else { PANEL3 })).on_hover_text("Metronome").clicked() {
                self.project.metronome = !metro;
            }
            egui::ComboBox::from_id_salt("quant")
                .width(64.0)
                .selected_text(self.project.launch_quantize.name())
                .show_ui(ui, |ui| {
                    for q in Quantize::ALL {
                        ui.selectable_value(&mut self.project.launch_quantize, q, q.name());
                    }
                })
                .response
                .on_hover_text("Global launch quantization");
            let scale_on = self.project.scale_mode;
            // The key it plays in (the Key drawer's, with a star).
            let moved = self.project.play_key.filter(|k| *k != self.project.scale);
            let key_name = match moved {
                Some(k) => format!("{}*", k.name()),
                None => self.project.scale.name(),
            };
            let scale_btn = egui::Button::new(RichText::new(key_name).color(if scale_on { TEXT_DARK } else { TEXT })).fill(if scale_on { ACCENT } else { PANEL3 });
            let tip = match moved {
                Some(k) => format!("The song is in {}; the Key drawer plays it in {}. Click to change or convert the whole project; right-click toggles scale mode.", self.project.scale.name(), k.name()),
                None => "Key & scale of the project. Click to change or convert the whole project; right-click toggles scale mode.".to_string(),
            };
            let resp = ui.add(scale_btn).on_hover_text(tip);
            if resp.clicked() {
                self.open_scale_dialog();
            }
            if resp.secondary_clicked() {
                self.project.scale_mode = !scale_on;
            }
            ui.separator();

            // Position + transport buttons.
            let status = self.status();
            let playing = status.playing.load(Ordering::Relaxed);
            let pos = status.song_pos.load();
            let (r, _) = ui.allocate_exact_size(vec2(84.0, 24.0), Sense::hover());
            rect_fill(ui.painter(), r, 2.0, BG);
            ui.painter().text(r.center(), Align2::CENTER_CENTER, format_pos(pos, self.project.beats_per_bar), mono(), if playing { PLAY } else { TEXT });

            let (r, resp) = ui.allocate_exact_size(vec2(28.0, 24.0), Sense::click());
            rect_fill(ui.painter(), r, 2.0, if playing { shade(PLAY, 0.35) } else { PANEL3 });
            play_icon(ui.painter(), r.center(), 12.0, if playing { PLAY } else { TEXT });
            if resp.on_hover_text("Play (Space)").clicked() {
                self.tape_before_play();
                self.cmd(Command::Play);
            }
            let (r, resp) = ui.allocate_exact_size(vec2(28.0, 24.0), Sense::click());
            rect_fill(ui.painter(), r, 2.0, PANEL3);
            stop_icon(ui.painter(), r.center(), 10.0, TEXT);
            if resp.on_hover_text("Stop (press twice to return to the start)").clicked() {
                self.cmd(Command::Stop);
            }
            let (r, resp) = ui.allocate_exact_size(vec2(28.0, 24.0), Sense::click());
            let rec = self.recording || !self.takes.is_empty();
            rect_fill(ui.painter(), r, 2.0, if rec { shade(REC, 0.4) } else { PANEL3 });
            rec_icon(ui.painter(), r.center(), 11.0, if rec { REC } else { TEXT_DIM });
            if resp.on_hover_text("Record (F9): armed audio tracks record into session slots, or onto the arrangement in Arrangement view; armed MIDI tracks overdub notes").clicked() {
                self.toggle_record();
            }
            // Automation Arm: knob moves go into the clips that play.
            let (r, resp) = ui.allocate_exact_size(vec2(28.0, 24.0), Sense::click());
            let arm = self.auto_arm;
            rect_fill(ui.painter(), r, 2.0, if arm { shade(REC, 0.4) } else { PANEL3 });
            let c = r.center();
            let col = if arm { REC } else { TEXT_DIM };
            ui.painter().add(egui::Shape::line(vec![c + vec2(-9.0, 5.0), c + vec2(-4.0, -1.0), c + vec2(0.0, 3.0), c + vec2(5.0, -5.0), c + vec2(9.0, -2.0)], egui::Stroke::new(2.0_f32, col)));
            ui.painter().circle_filled(c + vec2(9.0, 6.0), 2.5, col);
            if resp.on_hover_text("Automation Arm: while clips play, turning a knob of their track writes its moves into the clip (see Envelopes in the clip view)").clicked() {
                self.auto_arm = !arm;
            }
            let overridden = self.project.tracks.iter().enumerate().any(|(i, _)| status.tracks.get(i).map(|t| t.overridden.load(Ordering::Relaxed)).unwrap_or(false));
            let has_arr = self.project.tracks.iter().any(|t| !t.arrangement.is_empty());
            if overridden && has_arr {
                if ui.add(egui::Button::new(RichText::new("Back to Arr.").color(TEXT_DARK)).fill(ACCENT)).on_hover_text("Session clips are overriding the arrangement. Click to return.").clicked() {
                    self.cmd(Command::BackToArrangement);
                }
            }
            ui.separator();

            // Loop.
            let lp = self.project.loop_enabled;
            if ui.add(egui::Button::new(RichText::new("Loop").color(if lp { TEXT_DARK } else { TEXT })).fill(if lp { ACCENT } else { PANEL3 })).clicked() {
                self.project.loop_enabled = !lp;
            }
            let bar = self.project.beats_per_bar_f();
            let mut start_bar = self.project.loop_start / bar + 1.0;
            let mut len_bars = (self.project.loop_end - self.project.loop_start) / bar;
            let a = ui.add(egui::DragValue::new(&mut start_bar).range(1.0..=999.0).speed(0.05).fixed_decimals(0).prefix("start "));
            let b = ui.add(egui::DragValue::new(&mut len_bars).range(1.0..=256.0).speed(0.05).fixed_decimals(0).suffix(" bars"));
            if a.changed() || b.changed() {
                self.project.loop_start = (start_bar.round() - 1.0).max(0.0) * bar;
                self.project.loop_end = self.project.loop_start + len_bars.round().max(1.0) * bar;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // In a narrow window the CPU readout gives way and the views get short names.
                let room = ui.available_width();
                if ui.add(egui::Button::new(RichText::new("Big Picture").color(TEXT)).fill(PANEL3)).on_hover_text("For controllers and touch screens (Start on a controller opens it too)").clicked() {
                    self.enter_big_picture();
                }
                ui.separator();
                if room > 560.0 {
                    let cpu = status.cpu_load.load();
                    ui.label(RichText::new(format!("CPU {:>3.0}%", cpu * 100.0)).color(if cpu > 0.8 { REC } else { TEXT_DIM }).small());
                    ui.separator();
                }
                let dragging_clip = egui::DragAndDrop::payload::<crate::app::SlotDrag>(ui.ctx()).is_some() || egui::DragAndDrop::payload::<crate::app::ArrClipDrag>(ui.ctx()).is_some();
                let short = room < 470.0;
                for (v, label) in [(View::Tape, "Tape"), (View::Rack, "Rack"), (View::Arrangement, if short { "Arr." } else { "Arrangement" }), (View::Session, if short { "Sess." } else { "Session" })] {
                    let on = self.view == v;
                    let tip = match v {
                        View::Rack => "Every track's devices at once (Ctrl+Shift+R)",
                        View::Tape => "The four-track tape: record what you play, at any speed, backwards, looped",
                        _ => "Tab switches views (also while dragging a clip)",
                    };
                    let resp = ui.add(egui::Button::new(RichText::new(label).color(if on { TEXT_DARK } else { TEXT })).fill(if on { ACCENT } else { PANEL3 })).on_hover_text(tip);
                    if resp.clicked() {
                        self.view = v;
                    }
                    // Dragging a clip over the other view's button switches to it.
                    if dragging_clip && !on && resp.contains_pointer() {
                        self.view = v;
                    }
                }
            });
        });
    }

    fn tap_tempo(&mut self) {
        let now = Instant::now();
        let taps = &mut self.session.taps;
        if let Some(last) = taps.last() {
            if now.duration_since(*last).as_secs_f64() > 2.0 {
                taps.clear();
            }
        }
        taps.push(now);
        if taps.len() > 6 {
            taps.remove(0);
        }
        if taps.len() >= 3 {
            let span = taps.last().unwrap().duration_since(taps[0]).as_secs_f64();
            let bpm = 60.0 * (taps.len() - 1) as f64 / span;
            if (20.0..=999.0).contains(&bpm) {
                self.project.bpm = (bpm * 100.0).round() / 100.0;
            }
        }
    }

    fn ui_menus(&mut self, ui: &mut Ui) {
        ui.menu_button(RichText::new("Krabton").strong().color(ACCENT), |ui| {
            if ui.button("New Set        Ctrl+N").clicked() {
                self.new_project();
                ui.close();
            }
            if ui.button("Open...        Ctrl+O").clicked() {
                ui.close();
                self.open();
            }
            if ui.button("Save           Ctrl+S").clicked() {
                ui.close();
                self.save(false);
            }
            if ui.button("Save As...     Ctrl+Shift+S").clicked() {
                ui.close();
                self.save(true);
            }
            ui.separator();
            if ui.button("Export Audio... Ctrl+R").clicked() {
                self.export.open = true;
                ui.close();
            }
            if ui.button("Load Demo Set").clicked() {
                self.load_demo();
                ui.close();
            }
            if ui.button("Load Ambient Set").on_hover_text("Forest Morning: a brook, birds, wind chimes, a drone, a glass bowl and a pan flute").clicked() {
                self.load_ambient();
                ui.close();
            }
            ui.separator();
            if ui.button("About Krabton").clicked() {
                self.show_about = true;
                ui.close();
            }
        });
        ui.menu_button("Edit", |ui| {
            if ui.button("Undo   Ctrl+Z").clicked() {
                self.undo();
                ui.close();
            }
            if ui.button("Redo   Ctrl+Y").clicked() {
                self.redo();
                ui.close();
            }
            if ui.button("History...   Ctrl+H").on_hover_text("Every step you can undo, by name; click one to go there").clicked() {
                self.show_history = true;
                ui.close();
            }
            ui.separator();
            if ui.button("Scenes to Arrangement").on_hover_text("Lay every scene out on the timeline, one after another").clicked() {
                let n = self.project.scenes_to_arrangement();
                self.toast(format!("Placed {n} scene(s) in the arrangement"));
                self.view = View::Arrangement;
                ui.close();
            }
        });
        ui.menu_button("Create", |ui| {
            if ui.button("MIDI Track    Ctrl+Shift+T").clicked() {
                self.add_midi_track(Some(Instrument::synth("Init")));
                ui.close();
            }
            if ui.button("Audio Track   Ctrl+T").clicked() {
                self.add_audio_track();
                ui.close();
            }
            if ui.button("Return Track").clicked() {
                let n = self.project.returns.len();
                if n < krabton_engine::engine::MAX_RETURNS {
                    let name = format!("{} Return", (b'A' + n as u8) as char);
                    self.project.add_return(&name, Vec::new());
                }
                ui.close();
            }
            if ui.button("Scene         Ctrl+I").clicked() {
                let at = self.sel_scene + 1;
                self.project.add_scene(at);
                ui.close();
            }
        });
        ui.menu_button("Options", |ui| {
            if ui.checkbox(&mut self.prefs.big_picture, "Start in Big Picture").on_hover_text("For a Steam Deck, a ClockworkPi or a touch screen. --big-picture or KRABTON_BIG_PICTURE=1 does it once").changed() {
                self.prefs.save();
            }
            ui.separator();
            ui.label(RichText::new("MIDI input").color(TEXT_DIM).small());
            let ports = self.midi.ports.clone();
            if ports.is_empty() {
                ui.label("No MIDI inputs found");
            }
            for p in ports {
                let on = self.midi.connected.as_deref() == Some(p.as_str());
                if ui.selectable_label(on, &p).clicked() {
                    if on {
                        self.midi.disconnect();
                    } else if let Err(e) = self.midi.connect(&p) {
                        self.toast(e);
                    }
                    ui.close();
                }
            }
            if ui.button("Rescan MIDI ports").clicked() {
                self.midi.refresh();
            }
            ui.separator();
            ui.label(RichText::new("Audio output").color(TEXT_DIM).small());
            ui.label(RichText::new(&self.audio_desc).small());
            if self.audio_devices.is_none() {
                self.audio_devices = Some(crate::audio::output_device_names());
            }
            let chosen = self.prefs.audio_output.clone();
            egui::ComboBox::from_id_salt("audio_output").width(260.0).selected_text(chosen.clone().unwrap_or_else(|| "System default".into())).show_ui(ui, |ui| {
                let mut pick: Option<Option<String>> = None;
                if ui.selectable_label(chosen.is_none(), "System default").clicked() {
                    pick = Some(None);
                }
                for name in self.audio_devices.clone().unwrap_or_default() {
                    if ui.selectable_label(chosen.as_deref() == Some(name.as_str()), &name).clicked() {
                        pick = Some(Some(name));
                    }
                }
                if let Some(p) = pick {
                    self.prefs.audio_output = p;
                    self.prefs.save();
                    self.toast("Audio output saved: restart Krabton to switch to it");
                }
            });
            if ui.small_button("Rescan devices").clicked() {
                self.audio_devices = Some(crate::audio::output_device_names());
            }
            if self.audio_desc.starts_with("No audio") {
                ui.scope(|ui| {
                    ui.set_max_width(300.0);
                    ui.add(egui::Label::new(RichText::new(crate::audio::no_audio_hint()).small().color(ACCENT)).wrap());
                });
            }
            ui.separator();
            ui.label(RichText::new("Size of everything").color(TEXT_DIM).small());
            ui.horizontal(|ui| {
                let ctx = ui.ctx().clone();
                if ui.button(" - ").on_hover_text("Smaller (Ctrl -)").clicked() {
                    egui::gui_zoom::zoom_out(&ctx);
                }
                ui.label(format!("{:.0}%", ctx.zoom_factor() * 100.0));
                if ui.button(" + ").on_hover_text("Bigger (Ctrl +)").clicked() {
                    egui::gui_zoom::zoom_in(&ctx);
                }
                if ui.button("Reset").on_hover_text("Normal size (Ctrl 0)").clicked() {
                    ctx.set_zoom_factor(1.0);
                }
            });
            ui.separator();
            ui.checkbox(&mut self.kb_enabled, "Computer MIDI keyboard (M)");
            ui.checkbox(&mut self.rec_quantize, "Record quantization 1/16");
            ui.separator();
            ui.label(RichText::new("Scale").color(TEXT_DIM).small());
            ui.checkbox(&mut self.snap_input, "Snap played notes to scale");
            ui.checkbox(&mut self.scale_keys, "Keyboard plays scale steps");
            if ui.button("Key & Scale...").clicked() {
                self.open_scale_dialog();
                ui.close();
            }
        });
    }

    /// The Key drawer: the whole song in another key at the press of a key,
    /// on a keyboard (the root, the scale kept) or on chord buttons laid out
    /// round the circle of fifths as on an omnichord or an accordion's bass
    /// side (major, minor and seventh rows). Drums stay; what you play
    /// sounds as you play it.
    pub(crate) fn ui_transpose_drawer(&mut self, ui: &mut Ui) {
        use krabton_engine::{Scale, ScaleKind};
        let song = self.project.scale;
        let now = self.project.sounding_key();
        let moved = self.project.play_key.filter(|k| *k != song).is_some();
        // What a press asks for this frame: a key (the song's own: back).
        let mut want: Option<Scale> = None;
        let mut released = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Key").strong().size(16.0).color(TEXT));
            ui.add_space(8.0);
            let waiting = self.status().key_waiting.load(Ordering::Relaxed);
            let txt = if waiting {
                format!("{}  from the next bar", now.name())
            } else if moved {
                format!("{}  plays in  {}", song.name(), now.name())
            } else {
                format!("{}, the song's own", song.name())
            };
            let col = if waiting && !super::session::blink() { TEXT_DIM } else if moved || waiting { ACCENT } else { TEXT_DIM };
            ui.label(RichText::new(txt).size(14.0).color(col));
            if waiting {
                ui.ctx().request_repaint();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("x").on_hover_text("Close").clicked() {
                    self.transpose_open = false;
                }
                let on_bar = self.project.play_key_on_bar;
                if ui.selectable_label(on_bar, "Next bar").on_hover_text("On: a new key starts on the first beat of the next bar while the song plays. Off: at once").clicked() {
                    self.project.play_key_on_bar = !on_bar;
                }
                let latch = self.key_latch;
                if ui.selectable_label(latch, "Hold").on_hover_text("On: a press keeps the key. Off: the key lasts while you hold it, like a chord button").clicked() {
                    self.key_latch = !latch;
                }
                if moved {
                    if ui.button("Make it the key").on_hover_text("Write it into the set: every clip's notes move into this key and it becomes the song's key. It sounds the same").clicked() {
                        self.project.bake_play_key();
                        self.toast(format!("The song is in {} now", self.project.scale.name()));
                    }
                    if ui.button(format!("Back to {}", song.name())).clicked() {
                        self.project.play_key = None;
                    }
                }
            });
        });
        ui.add_space(6.0);
        ui.horizontal_top(|ui| {
            // The keyboard: an octave; a key is the root, the scale stays.
            const NAMES: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];
            let kw = 34.0;
            let kh = 96.0;
            let (rect, resp) = ui.allocate_exact_size(vec2(7.0 * kw, kh), Sense::click_and_drag());
            let p = ui.painter().clone();
            let whites = [0u8, 2, 4, 5, 7, 9, 11];
            let blacks = [(1u8, 0.0f32), (3, 1.0), (6, 3.0), (8, 4.0), (10, 5.0)];
            let white_r = |k: usize| Rect::from_min_size(pos2(rect.left() + k as f32 * kw, rect.top()), vec2(kw - 2.0, kh));
            let black_r = |after: f32| Rect::from_min_size(pos2(rect.left() + (after + 1.0) * kw - kw * 0.32, rect.top()), vec2(kw * 0.62, kh * 0.6));
            let hit = resp.interact_pointer_pos().or(resp.hover_pos()).and_then(|pt| {
                blacks.iter().find(|(_, a)| black_r(*a).contains(pt)).map(|b| b.0).or_else(|| (0..7).find(|&k| white_r(k).contains(pt)).map(|k| whites[k]))
            });
            for (k, &pc) in whites.iter().enumerate() {
                let r = white_r(k);
                let lit = now.root % 12 == pc;
                p.rect_filled(r, 3.0, if lit { ACCENT } else if hit == Some(pc) { Color32::from_gray(225) } else { Color32::from_gray(200) });
                text(&p, pos2(r.center().x, r.bottom() - 12.0), Align2::CENTER_CENTER, NAMES[pc as usize], FontId::proportional(13.0), TEXT_DARK);
                if song.root % 12 == pc {
                    p.circle_filled(pos2(r.center().x, r.bottom() - 28.0), 3.5, if lit { TEXT_DARK } else { ACCENT });
                }
            }
            for &(pc, after) in &blacks {
                let r = black_r(after);
                let lit = now.root % 12 == pc;
                p.rect_filled(r, 3.0, if lit { ACCENT } else if hit == Some(pc) { Color32::from_gray(70) } else { Color32::from_gray(35) });
                text(&p, pos2(r.center().x, r.bottom() - 10.0), Align2::CENTER_CENTER, NAMES[pc as usize], FontId::proportional(10.0), if lit { TEXT_DARK } else { TEXT_DIM });
                if song.root % 12 == pc {
                    p.circle_filled(pos2(r.center().x, r.bottom() - 22.0), 3.0, ACCENT);
                }
            }
            let pressed = ui.input(|i| i.pointer.primary_pressed()) && resp.hovered();
            if let (true, Some(pc)) = (pressed, hit) {
                want = Some(Scale::new(pc, now.kind));
            }
            if resp.drag_stopped() || (resp.clicked() && !self.key_latch) {
                released = true;
            }
            resp.on_hover_text("Press a key: the song plays from that root, in the same scale. The dot is the song's own key");
            ui.add_space(18.0);
            // The chord buttons: round the circle of fifths, a row per kind.
            let order: [u8; 12] = [1, 8, 3, 10, 5, 0, 7, 2, 9, 4, 11, 6];
            let rows: [(&str, ScaleKind); 3] = [("", ScaleKind::Major), ("m", ScaleKind::Minor), ("7", ScaleKind::Mixolydian)];
            let (bw, bh) = (44.0, 28.0);
            let (grid, gresp) = ui.allocate_exact_size(vec2(12.0 * bw, 3.0 * bh + 8.0), Sense::click_and_drag());
            let cell = |c: usize, r: usize| Rect::from_min_size(pos2(grid.left() + c as f32 * bw, grid.top() + r as f32 * (bh + 4.0)), vec2(bw - 4.0, bh));
            let ghit = gresp.interact_pointer_pos().or(gresp.hover_pos()).and_then(|pt| {
                (0..3).flat_map(|r| (0..12).map(move |c| (c, r))).find(|&(c, r)| cell(c, r).contains(pt))
            });
            for (r, (suffix, kind)) in rows.iter().enumerate() {
                for (c, &pc) in order.iter().enumerate() {
                    let rr = cell(c, r);
                    let key = Scale::new(pc, *kind);
                    let on = now == key;
                    let own = song == key;
                    let hot = ghit == Some((c, r));
                    p.rect_filled(rr, 4.0, if on { ACCENT } else if hot { PANEL3.gamma_multiply(1.4) } else { PANEL3 });
                    if own {
                        p.rect_stroke(rr, 4.0, egui::Stroke::new(1.5_f32, ACCENT), egui::StrokeKind::Inside);
                    }
                    text(&p, rr.center(), Align2::CENTER_CENTER, &format!("{}{}", NAMES[pc as usize], suffix), FontId::proportional(13.0), if on { TEXT_DARK } else { TEXT });
                }
            }
            let gpressed = ui.input(|i| i.pointer.primary_pressed()) && gresp.hovered();
            if let (true, Some((c, r))) = (gpressed, ghit) {
                want = Some(Scale::new(order[c], rows[r].1));
            }
            if gresp.drag_stopped() || (gresp.clicked() && !self.key_latch) {
                released = true;
            }
            gresp.on_hover_text("Chord buttons, as on an omnichord: major, minor and seventh keys round the circle of fifths. The outlined one is the song's own");
        });
        if let Some(k) = want {
            self.project.play_key = if k == song { None } else { Some(k) };
            self.key_held = !self.key_latch;
        }
        // Without Hold, letting go goes back to the song's key.
        if released && self.key_held && !self.key_latch {
            self.project.play_key = None;
            self.key_held = false;
        }
    }

    pub(crate) fn ui_statusbar(&mut self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            if let Some((msg, at)) = &self.message {
                let age = at.elapsed().as_secs_f32();
                if age < 8.0 {
                    let fade = (1.0 - (age - 6.0).max(0.0) / 2.0).clamp(0.0, 1.0);
                    ui.label(RichText::new(msg).color(mix(BG, TEXT, fade)).small());
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(&self.audio_desc).color(TEXT_DIM).small());
                ui.separator();
                let midi = self.midi.connected.clone().unwrap_or_else(|| "no MIDI in".into());
                ui.label(RichText::new(midi).color(TEXT_DIM).small());
                ui.separator();
                if self.kb_enabled {
                    ui.label(
                        RichText::new(format!("Keys: C{} vel {}", self.kb_octave, (self.kb_velocity * 127.0).round() as i32))
                            .color(TEXT_DIM)
                            .small(),
                    );
                }
                // What you play goes to more than one track.
                let targets = self.note_targets_count();
                if targets > 1 {
                    ui.separator();
                    ui.label(RichText::new(format!("Playing into {targets} tracks")).color(ACCENT).small());
                }
                ui.separator();
                let moved = self.project.play_key.filter(|k| *k != self.project.scale);
                let label = match moved {
                    Some(k) => format!("Key: {}", k.name()),
                    None => "Key".to_string(),
                };
                let open = self.transpose_open;
                let btn = egui::Button::new(RichText::new(format!("{} {}", if open { "v" } else { "^" }, label)).small().color(if moved.is_some() { TEXT_DARK } else { TEXT })).fill(if moved.is_some() { ACCENT } else { PANEL3 });
                if ui.add(btn).on_hover_text("Play the whole song in another key: a keyboard and chord buttons").clicked() {
                    self.transpose_open = !open;
                }
                if let Some(job) = &self.export_job {
                    ui.separator();
                    ui.add(egui::ProgressBar::new(job.progress.fraction()).desired_width(120.0).text("Exporting"));
                }
            });
        });
    }
}
