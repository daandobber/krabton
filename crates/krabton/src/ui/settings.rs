//! The settings window: the audio output and input devices (switched at
//! once) and the MIDI inputs.

use crate::app::App;
use crate::theme::*;
use egui::{Context, RichText, Ui};

/// A device menu: the system default, then every device by name. Returns
/// the one picked (Some(None): the default).
fn device_combo(ui: &mut Ui, id: &str, chosen: Option<&str>, names: &[String]) -> Option<Option<String>> {
    let mut pick = None;
    egui::ComboBox::from_id_salt(id).width(300.0).selected_text(chosen.unwrap_or("System default")).show_ui(ui, |ui| {
        if ui.selectable_label(chosen.is_none(), "System default").clicked() {
            pick = Some(None);
        }
        for name in names {
            if ui.selectable_label(chosen == Some(name.as_str()), name).clicked() {
                pick = Some(Some(name.clone()));
            }
        }
        if let Some(c) = chosen.filter(|c| !names.iter().any(|n| n == c)) {
            ui.label(RichText::new(format!("{c} (not connected)")).small().color(TEXT_DIM));
        }
    });
    pick
}

impl App {
    pub(crate) fn ui_settings(&mut self, ctx: &Context) {
        if !self.show_settings {
            return;
        }
        if self.audio_devices.is_none() {
            self.audio_devices = Some(crate::audio::output_device_names());
        }
        if self.audio_in_devices.is_none() {
            self.audio_in_devices = Some(crate::audio::input_device_names());
        }
        let mut out_pick = None;
        let mut in_pick = None;
        let mut rescan = false;
        let (open, _) = super::windows::tool_window(ctx, "settings", "Settings", egui::vec2(460.0, 400.0), |ui| {
            ui.label(RichText::new("Audio").strong().color(ACCENT));
            egui::Grid::new("audio_settings").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                ui.label("Output");
                out_pick = device_combo(ui, "settings_audio_out", self.prefs.audio_output.as_deref(), self.audio_devices.as_deref().unwrap_or_default());
                ui.end_row();
                ui.label("");
                let rate = self.engine.status.sample_rate.load();
                let now = if self.audio_desc.contains(" Hz") { self.audio_desc.clone() } else { format!("{} · {rate:.0} Hz", self.audio_desc) };
                ui.label(RichText::new(format!("Playing to {now}")).small().color(TEXT_DIM));
                ui.end_row();
                ui.label("Input");
                in_pick = device_combo(ui, "settings_audio_in", self.prefs.audio_input.as_deref(), self.audio_in_devices.as_deref().unwrap_or_default());
                ui.end_row();
                ui.label("");
                let status = match (&self.input_dev, &self.input_error) {
                    (Some(d), _) => format!("Recording from {} ({} channels)", d.name, d.channels),
                    (None, Some(e)) => format!("Could not open it: {e}"),
                    (None, None) => "Opens when an audio track listens to its input".to_string(),
                };
                ui.label(RichText::new(status).small().color(if self.input_error.is_some() { REC } else { TEXT_DIM }));
                ui.end_row();
            });
            ui.horizontal(|ui| {
                if ui.small_button("Rescan devices").on_hover_text("Look again for devices plugged in since").clicked() {
                    rescan = true;
                }
                ui.label(RichText::new("A device switches at once.").small().color(TEXT_DIM));
            });
            if self.audio_desc.starts_with("No audio") {
                ui.add(egui::Label::new(RichText::new(crate::audio::no_audio_hint()).small().color(ACCENT)).wrap());
            }
            ui.separator();
            ui.label(RichText::new("MIDI input").strong().color(ACCENT));
            let ports = self.midi.ports.clone();
            if ports.is_empty() {
                ui.label(RichText::new("No MIDI inputs found").color(TEXT_DIM));
            }
            for p in ports {
                let on = self.midi.connected.as_deref() == Some(p.as_str());
                if ui.selectable_label(on, &p).on_hover_text(if on { "Click to disconnect" } else { "Click to play Krabton from it" }).clicked() {
                    if on {
                        self.midi.disconnect();
                    } else if let Err(e) = self.midi.connect(&p) {
                        self.toast(e);
                    }
                }
            }
            if ui.small_button("Rescan MIDI ports").clicked() {
                self.midi.refresh();
            }
        });
        if rescan {
            self.audio_devices = Some(crate::audio::output_device_names());
            self.audio_in_devices = Some(crate::audio::input_device_names());
        }
        if let Some(p) = out_pick {
            self.switch_audio_output(p);
        }
        if let Some(p) = in_pick {
            self.switch_audio_input(p);
        }
        if !open {
            self.show_settings = false;
        }
    }
}
