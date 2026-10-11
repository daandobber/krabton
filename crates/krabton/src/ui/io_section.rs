//! The In/Out section of the session mixer, under the clip slots, laid out
//! like Live's: Audio From (or MIDI From) with a second chooser for the
//! channel or tap point, Monitor In/Auto/Off, and Audio To.

use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Id, Rect, RichText, Ui, UiBuilder};
use krabton_engine::engine::EngineStatus;
use krabton_engine::{AudioInput, AudioOutput, MidiInput, Monitor, Tap, TrackKind};

/// Height of the section.
pub const IO_H: f32 = 132.0;
const ROW: f32 = 18.0;
const LABEL: f32 = 12.0;

/// A combo box filling `rect`.
fn combo(ui: &mut Ui, rect: Rect, id: Id, selected: &str, enabled: bool, add: impl FnOnce(&mut Ui)) {
    let mut c = ui.new_child(UiBuilder::new().max_rect(rect));
    c.set_clip_rect(rect.expand(1.0).intersect(ui.clip_rect()));
    c.add_enabled_ui(enabled, |ui| {
        egui::ComboBox::from_id_salt(id).width(rect.width()).height(320.0).selected_text(RichText::new(selected).small()).show_ui(ui, add);
    });
}

/// A chooser that cannot be opened, showing `text`.
fn fixed_box(ui: &Ui, rect: Rect, text: &str) {
    let p = ui.painter();
    rect_fill(p, rect, 2.0, PANEL2);
    p.with_clip_rect(rect).text(rect.left_center() + vec2(5.0, 0.0), Align2::LEFT_CENTER, text, small(), TEXT_DIM);
}

fn label(ui: &Ui, x: f32, y: f32, text: &str) {
    ui.painter().text(pos2(x, y), Align2::LEFT_TOP, text, small(), TEXT_DIM);
}

/// A small vertical level meter.
fn meter(ui: &Ui, rect: Rect, level: f32) {
    let p = ui.painter();
    rect_fill(p, rect, 1.0, PANEL3);
    let h = level.clamp(0.0, 1.0).sqrt() * rect.height();
    let fill = Rect::from_min_max(pos2(rect.left(), rect.bottom() - h), rect.max);
    rect_fill(p, fill, 1.0, if level >= 0.99 { REC } else { PLAY });
}

impl App {
    /// The I/O section of track `i`.
    pub(crate) fn track_io(&mut self, ui: &mut Ui, i: usize, r: Rect, status: &EngineStatus) {
        let Some(t) = self.project.tracks.get(i) else { return };
        let (tid, kind, input, monitor, output, midi_in, midi_channel) = (t.id, t.kind, t.input, t.monitor, t.output, t.midi_in, t.midi_channel);
        let tracks: Vec<(usize, String, u64)> = self.project.tracks.iter().enumerate().map(|(j, t)| (j, format!("{}-{}", j + 1, t.name), t.id)).collect();
        let sources: Vec<bool> = (0..tracks.len()).map(|j| self.project.can_route(j, i)).collect();
        let midi_sources: Vec<bool> = (0..tracks.len()).map(|j| self.project.can_route_midi(j, i)).collect();
        let targets: Vec<bool> = (0..tracks.len()).map(|j| self.project.can_route(i, j)).collect();
        let name_of = |id: u64| tracks.iter().find(|t| t.2 == id).map(|t| t.1.clone()).unwrap_or_else(|| "?".into());
        let channels = match status.input_channels.load(std::sync::atomic::Ordering::Relaxed) as usize {
            0 => 2,
            n => n,
        };
        let port = self.midi.connected.clone();

        rect_fill(ui.painter(), r, 0.0, PANEL);
        let x = r.left() + 4.0;
        let w = r.width() - 8.0;
        let row = |y: f32| Rect::from_min_size(pos2(x, y), vec2(w, ROW - 1.0));
        let mut y = r.top() + 4.0;
        let (mut new_input, mut new_monitor, mut new_output) = (input, monitor, output);
        let (mut new_midi_in, mut new_channel) = (midi_in, midi_channel);

        if kind == TrackKind::Audio {
            label(ui, x, y, "Audio From");
            y += LABEL;
            let from = match input {
                AudioInput::None => "No Input".to_string(),
                AudioInput::Ext { .. } => "Ext. In".to_string(),
                AudioInput::Track { id, .. } => name_of(id),
                AudioInput::Resampling => "Resampling".to_string(),
            };
            combo(ui, row(y), Id::new(("io_from", tid)), &from, true, |ui| {
                if ui.selectable_label(matches!(input, AudioInput::Ext { .. }), "Ext. In").clicked() && !matches!(input, AudioInput::Ext { .. }) {
                    new_input = AudioInput::Ext { channel: 0, stereo: true };
                }
                ui.selectable_value(&mut new_input, AudioInput::Resampling, "Resampling").on_hover_text("The master output");
                for (j, name, id) in &tracks {
                    if !sources[*j] {
                        continue;
                    }
                    let on = matches!(input, AudioInput::Track { id: s, .. } if s == *id);
                    if ui.selectable_label(on, name).clicked() && !on {
                        new_input = AudioInput::Track { id: *id, tap: Tap::PostFx };
                    }
                }
                ui.selectable_value(&mut new_input, AudioInput::None, "No Input");
            });
            y += ROW;
            // Second chooser: input channel or tap point, with the input meter.
            let meter_id = Id::new(("io_meter", tid));
            let pk = status.tracks.get(i).map(|ts| ts.in_peak.swap(0.0)).unwrap_or(0.0);
            let level = ui.ctx().memory_mut(|m| {
                let v = m.data.get_temp_mut_or_default::<f32>(meter_id);
                *v = pk.max(*v * 0.88);
                *v
            });
            meter(ui, Rect::from_min_size(pos2(x, y + 2.0), vec2(4.0, ROW - 5.0)), level);
            let r2 = Rect::from_min_size(pos2(x + 7.0, y), vec2(w - 7.0, ROW - 1.0));
            match input {
                AudioInput::Ext { channel, stereo } => {
                    let text = if stereo { format!("{}/{}", channel + 1, channel + 2) } else { format!("{}", channel + 1) };
                    combo(ui, r2, Id::new(("io_ch", tid)), &text, true, |ui| {
                        for c in 0..channels {
                            ui.selectable_value(&mut new_input, AudioInput::Ext { channel: c as u8, stereo: false }, format!("{}", c + 1));
                        }
                        for c in (0..channels).step_by(2).filter(|c| c + 1 < channels) {
                            ui.selectable_value(&mut new_input, AudioInput::Ext { channel: c as u8, stereo: true }, format!("{}/{}", c + 1, c + 2));
                        }
                    });
                }
                AudioInput::Track { id, tap } => {
                    combo(ui, r2, Id::new(("io_tap", tid)), tap.name(), true, |ui| {
                        for t in Tap::ALL {
                            ui.selectable_value(&mut new_input, AudioInput::Track { id, tap: t }, t.name());
                        }
                    });
                }
                AudioInput::Resampling | AudioInput::None => fixed_box(ui, r2, ""),
            }
        } else {
            label(ui, x, y, "MIDI From");
            y += LABEL;
            let device = port.clone().unwrap_or_else(|| "MIDI In".into());
            let from = match midi_in {
                MidiInput::AllIns => "All Ins".to_string(),
                MidiInput::Keyboard => "Computer Keyboard".to_string(),
                MidiInput::Device => device.clone(),
                MidiInput::None => "No Input".to_string(),
                MidiInput::Track(id) => name_of(id),
            };
            combo(ui, row(y), Id::new(("io_midi", tid)), &from, true, |ui| {
                ui.selectable_value(&mut new_midi_in, MidiInput::AllIns, "All Ins");
                ui.selectable_value(&mut new_midi_in, MidiInput::Keyboard, "Computer Keyboard");
                ui.selectable_value(&mut new_midi_in, MidiInput::Device, &device);
                ui.selectable_value(&mut new_midi_in, MidiInput::None, "No Input");
                // The notes other MIDI tracks play, sequencers included.
                if midi_sources.iter().any(|&ok| ok) {
                    ui.separator();
                    for (j, name, id) in &tracks {
                        if midi_sources[*j] {
                            ui.selectable_value(&mut new_midi_in, MidiInput::Track(*id), name).on_hover_text("The notes this track plays, after its MIDI effects");
                        }
                    }
                }
            });
            y += ROW;
            let r2 = Rect::from_min_size(pos2(x + 7.0, y), vec2(w - 7.0, ROW - 1.0));
            let text = match midi_channel {
                None => "All Channels".to_string(),
                Some(c) => format!("Ch. {}", c + 1),
            };
            let enabled = matches!(midi_in, MidiInput::AllIns | MidiInput::Device);
            combo(ui, r2, Id::new(("io_midi_ch", tid)), &text, enabled, |ui| {
                ui.selectable_value(&mut new_channel, None, "All Channels");
                for c in 0..16u8 {
                    ui.selectable_value(&mut new_channel, Some(c), format!("Ch. {}", c + 1));
                }
            });
        }
        y += ROW + 2.0;

        label(ui, x, y, "Monitor");
        y += LABEL;
        let bw = w / 3.0;
        for (k, m) in Monitor::ALL.into_iter().enumerate() {
            let b = Rect::from_min_size(pos2(x + k as f32 * bw, y), vec2(bw - 2.0, ROW - 3.0));
            let tip = match m {
                Monitor::In => "In: always hear the input; the track's clips stay silent",
                Monitor::Auto => "Auto: hear the input while the track is armed",
                Monitor::Off => "Off: never hear the input (it can still be recorded)",
            };
            if button(ui, b, Id::new(("io_mon", tid, k)), m.name(), new_monitor == m, ACCENT).on_hover_text(tip).clicked() {
                new_monitor = m;
            }
        }
        y += ROW + 1.0;

        label(ui, x, y, "Audio To");
        y += LABEL;
        let to = match output {
            AudioOutput::Master => "Master".to_string(),
            AudioOutput::Track(id) => name_of(id),
        };
        combo(ui, row(y), Id::new(("io_to", tid)), &to, true, |ui| {
            ui.selectable_value(&mut new_output, AudioOutput::Master, "Master");
            for (j, name, id) in &tracks {
                if targets[*j] {
                    ui.selectable_value(&mut new_output, AudioOutput::Track(*id), name);
                }
            }
        });
        y += ROW;
        fixed_box(ui, row(y), if matches!(output, AudioOutput::Track(_)) { "Track In" } else { "" });

        // Picking an interface input again retries a device that failed.
        if new_input != input && matches!(new_input, AudioInput::Ext { .. }) {
            self.input_error = None;
        }
        let Some(t) = self.project.tracks.get_mut(i) else { return };
        t.input = new_input;
        t.monitor = new_monitor;
        t.midi_in = new_midi_in;
        t.midi_channel = new_channel;
        if new_output != output {
            t.output = new_output;
            // Like Live, the track it goes to plays it: let that one listen.
            if let AudioOutput::Track(dest) = new_output {
                if let Some(d) = self.project.tracks.iter_mut().find(|d| d.id == dest) {
                    if d.monitor == Monitor::Auto && !d.arm {
                        d.monitor = Monitor::In;
                    }
                }
            }
        }
    }

    /// The I/O section of a return track: it goes to the master.
    pub(crate) fn return_io(&self, ui: &mut Ui, r: Rect) {
        rect_fill(ui.painter(), r, 0.0, PANEL);
        let x = r.left() + 4.0;
        let y = r.top() + 4.0 + 2.0 * (LABEL + ROW) + LABEL + ROW + 3.0;
        label(ui, x, y, "Audio To");
        fixed_box(ui, Rect::from_min_size(pos2(x, y + LABEL), vec2(r.width() - 8.0, ROW - 1.0)), "Master");
    }

    /// The I/O section of the master: where it plays.
    pub(crate) fn master_io(&self, ui: &mut Ui, r: Rect) {
        rect_fill(ui.painter(), r, 0.0, PANEL);
        let x = r.left() + 4.0;
        let y = r.top() + 4.0 + 2.0 * (LABEL + ROW) + LABEL + ROW + 3.0;
        label(ui, x, y, "Master Out");
        fixed_box(ui, Rect::from_min_size(pos2(x, y + LABEL), vec2(r.width() - 8.0, ROW - 1.0)), "1/2");
    }
}
