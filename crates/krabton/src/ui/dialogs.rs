//! Small modal-ish windows: rename, export, about.

use crate::app::{App, ExportSource, RenameTarget};
use crate::theme::*;
use egui::{pos2, vec2, Context, Rect, RichText, Sense, Ui};
use krabton_engine::scale::NOTE_NAMES;
use krabton_engine::{Scale, ScaleKind};

/// One octave of piano keys with the scale notes lit.
fn mini_keyboard(ui: &mut Ui, scale: &Scale) {
    let (r, _) = ui.allocate_exact_size(vec2(7.0 * 26.0, 54.0), Sense::hover());
    let p = ui.painter();
    let white = [0u8, 2, 4, 5, 7, 9, 11];
    let w = r.width() / 7.0;
    let lit = |pc: u8| {
        if scale.is_root(pc) {
            ACCENT
        } else if scale.contains(pc) {
            shade(ACCENT, 0.6)
        } else {
            egui::Color32::TRANSPARENT
        }
    };
    for (i, &pc) in white.iter().enumerate() {
        let k = Rect::from_min_size(pos2(r.left() + i as f32 * w, r.top()), vec2(w - 1.0, r.height()));
        p.rect_filled(k, 2.0, egui::Color32::from_gray(210));
        let c = lit(pc);
        if c != egui::Color32::TRANSPARENT {
            p.rect_filled(Rect::from_min_size(pos2(k.left() + 2.0, k.bottom() - 16.0), vec2(k.width() - 4.0, 12.0)), 2.0, c);
        }
        p.text(pos2(k.center().x, k.bottom() - 10.0), egui::Align2::CENTER_CENTER, NOTE_NAMES[pc as usize], small(), TEXT_DARK);
    }
    for (after, pc) in [(0usize, 1u8), (1, 3), (3, 6), (4, 8), (5, 10)] {
        let x = r.left() + (after as f32 + 1.0) * w - w * 0.3;
        let k = Rect::from_min_size(pos2(x, r.top()), vec2(w * 0.6, r.height() * 0.58));
        p.rect_filled(k, 2.0, egui::Color32::from_gray(35));
        let c = lit(pc);
        if c != egui::Color32::TRANSPARENT {
            p.rect_filled(Rect::from_min_size(pos2(k.left() + 2.0, k.bottom() - 10.0), vec2(k.width() - 4.0, 7.0)), 2.0, c);
        }
    }
}

impl App {
    pub(crate) fn ui_dialogs(&mut self, ctx: &Context) {
        self.ui_rename(ctx);
        self.ui_export(ctx);
        self.ui_scale_dialog(ctx);
        self.ui_history(ctx);
        if self.show_about {
            let mut open = true;
            egui::Window::new("About Krabton").collapsible(false).resizable(false).open(&mut open).show(ctx, |ui| {
                ui.heading(RichText::new("Krabton").color(ACCENT));
                ui.label("An open-source, Live-inspired DAW written in Rust.");
                ui.label("Session view, arrangement, synths, drums, sampler and effects.");
                ui.add_space(6.0);
                ui.label(RichText::new("Shortcuts").strong());
                for (k, v) in [
                    ("Space", "Play / Stop"),
                    ("Tab", "Session / Arrangement"),
                    ("Shift+Tab", "Clip / Device view"),
                    ("A W S E D F T G Y H U J K", "Play notes"),
                    ("Z / X", "Octave down / up"),
                    ("C / V", "Velocity down / up"),
                    ("F9", "Record (session, or arrangement in Arrangement view)"),
                    ("Enter", "Launch selected scene"),
                    ("Ctrl+Alt+I", "Show / hide In/Out"),
                    ("Ctrl+D / Del", "Duplicate / delete"),
                    ("Ctrl+C / Ctrl+V", "Copy / paste clip"),
                    ("B", "Draw mode in the piano roll"),
                    ("Ctrl+U", "Quantize notes"),
                    ("Ctrl+Z / Ctrl+Y", "Undo / Redo"),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(k).monospace().color(ACCENT));
                        ui.label(v);
                    });
                }
                ui.add_space(4.0);
                ui.label(RichText::new("Plaits uses the Plaits DSP code by Emilie Gillet (Mutable Instruments, MIT) via mi-plaits-dsp by Oliver Rockstedt (MIT). Braids and Elements are Rust ports of Emilie Gillet's Braids and Elements code (MIT).").small().color(TEXT_DIM));
                ui.label(RichText::new("Not affiliated with Ableton or Mutable Instruments. MIT licensed.").small().color(TEXT_DIM));
            });
            self.show_about = open;
        }
    }

    fn ui_rename(&mut self, ctx: &Context) {
        let Some((target, mut text)) = self.renaming.take() else { return };
        let mut done = false;
        let mut cancel = false;
        egui::Window::new("Rename").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            let resp = ui.text_edit_singleline(&mut text);
            resp.request_focus();
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                done = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    done = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
        if cancel {
            return;
        }
        if !done {
            self.renaming = Some((target, text));
            return;
        }
        let name = text.trim().to_string();
        if name.is_empty() {
            return;
        }
        match target {
            RenameTarget::Track(i) => {
                if let Some(t) = self.project.tracks.get_mut(i) {
                    t.name = name;
                }
            }
            RenameTarget::Return(i) => {
                if let Some(t) = self.project.returns.get_mut(i) {
                    t.name = name;
                }
            }
            RenameTarget::Scene(i) => {
                if let Some(s) = self.project.scenes.get_mut(i) {
                    s.name = name;
                }
            }
            RenameTarget::Clip(sel) => {
                if let Some(c) = self.clip_mut(sel) {
                    c.name = name;
                }
            }
            RenameTarget::Section(id) => {
                if let Some(s) = self.project.sections.iter_mut().find(|s| s.id == id) {
                    s.name = name;
                }
            }
        }
    }

    /// The undo history: every step by name, the current one lit; click one
    /// to go back (or forward) to it.
    fn ui_history(&mut self, ctx: &Context) {
        if !self.show_history {
            return;
        }
        let mut open = true;
        let mut go: Option<usize> = None;
        let n = self.undo_labels.len();
        egui::Window::new("History").open(&mut open).default_width(280.0).default_height(360.0).resizable(true).show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
                if ui.selectable_label(n == 0, "Start").clicked() {
                    go = Some(0);
                }
                for (j, label) in self.undo_labels.iter().enumerate() {
                    if ui.selectable_label(j + 1 == n, format!("{}  {label}", j + 1)).clicked() {
                        go = Some(j + 1);
                    }
                }
                // Undone steps, still there to redo.
                for (k, label) in self.redo_labels.iter().rev().enumerate() {
                    let text = egui::RichText::new(format!("{}  {label}", n + k + 1)).color(crate::theme::TEXT_DIM).italics();
                    if ui.selectable_label(false, text).clicked() {
                        go = Some(n + k + 1);
                    }
                }
            });
        });
        if let Some(steps) = go {
            self.history_go(steps);
        }
        if !open {
            self.show_history = false;
        }
    }

    fn ui_scale_dialog(&mut self, ctx: &Context) {
        let Some(mut d) = self.scale_dialog else { return };
        let current = self.project.scale;
        let mut open = true;
        let mut close = false;
        egui::Window::new("Key & Scale").collapsible(false).resizable(false).open(&mut open).show(ctx, |ui| {
            ui.set_max_width(380.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Project key").color(TEXT_DIM));
                ui.label(RichText::new(current.name()).strong().color(ACCENT));
                ui.label(RichText::new(current.note_names()).small().color(TEXT_DIM));
            });
            ui.checkbox(&mut self.project.scale_mode, "Scale mode: highlight the scale in the piano roll");
            ui.checkbox(&mut self.piano.fold, "Fold the piano roll to the scale");
            ui.checkbox(&mut self.snap_input, "Snap played notes (keyboard and MIDI) to the scale");
            ui.checkbox(&mut self.scale_keys, "Computer keyboard plays scale steps (A S D F ...)");
            ui.separator();

            ui.label(RichText::new("Root").small().color(TEXT_DIM));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                for (i, name) in NOTE_NAMES.iter().enumerate() {
                    let on = d.root as usize == i;
                    let b = egui::Button::new(RichText::new(*name).color(if on { TEXT_DARK } else { TEXT })).fill(if on { ACCENT } else { PANEL3 }).min_size(vec2(30.0, 22.0));
                    if ui.add(b).clicked() {
                        d.root = i as u8;
                    }
                }
            });
            ui.label(RichText::new("Scale").small().color(TEXT_DIM));
            egui::Grid::new("scale_kinds").num_columns(3).spacing(vec2(2.0, 2.0)).show(ui, |ui| {
                for (i, kind) in ScaleKind::ALL.iter().enumerate() {
                    let on = d.kind == *kind;
                    let b = egui::Button::new(RichText::new(kind.name()).color(if on { TEXT_DARK } else { TEXT })).fill(if on { ACCENT } else { PANEL3 }).min_size(vec2(120.0, 22.0));
                    if ui.add(b).clicked() {
                        d.kind = *kind;
                    }
                    if i % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
            ui.add_space(4.0);
            let target = Scale::new(d.root, d.kind);
            ui.horizontal(|ui| {
                mini_keyboard(ui, &target);
                ui.vertical(|ui| {
                    ui.label(RichText::new(target.name()).strong().size(16.0));
                    ui.label(RichText::new(target.note_names()).color(TEXT_DIM));
                });
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Set Scale").on_hover_text("Change the project scale without touching any notes").clicked() {
                    self.project.scale = target;
                    self.project.scale_mode = true;
                    self.toast(format!("Project scale is now {}", target.name()));
                    close = true;
                }
                let convert = egui::Button::new(RichText::new(format!("Convert Project to {}", target.name())).color(TEXT_DARK)).fill(ACCENT);
                if ui.add_enabled(target != current, convert).clicked() {
                    let n = self.project.convert_scale(target);
                    self.project.scale_mode = true;
                    self.toast(format!("Converted the project from {} to {} ({n} notes moved)", current.name(), target.name()));
                    close = true;
                }
            });
            ui.label(
                RichText::new(
                    "Convert moves every note of every MIDI clip (session and arrangement) to the same scale degree in the new key, e.g. A minor's fifth becomes C major's fifth. Drum tracks are left alone. Ctrl+Z undoes it.",
                )
                .small()
                .color(TEXT_DIM),
            );
        });
        self.scale_dialog = if open && !close { Some(d) } else { None };
    }

    fn ui_export(&mut self, ctx: &Context) {
        if !self.export.open {
            return;
        }
        let mut open = true;
        let has_arr = self.project.arrangement_end() > 0.0;
        egui::Window::new("Export Audio").collapsible(false).resizable(false).open(&mut open).show(ctx, |ui| {
            ui.radio_value(&mut self.export.source, ExportSource::Arrangement, "Arrangement").on_hover_text("Renders the whole arrangement");
            if !has_arr {
                ui.label(RichText::new("   (arrangement is empty)").small().color(TEXT_DIM));
            }
            ui.radio_value(&mut self.export.source, ExportSource::Scene, "Scene");
            ui.add_enabled_ui(self.export.source == ExportSource::Scene, |ui| {
                ui.horizontal(|ui| {
                    let name = self.project.scenes.get(self.export.scene).map(|s| s.name.clone()).unwrap_or_default();
                    egui::ComboBox::from_id_salt("export_scene").selected_text(name).show_ui(ui, |ui| {
                        for (i, s) in self.project.scenes.iter().enumerate() {
                            ui.selectable_value(&mut self.export.scene, i, &s.name);
                        }
                    });
                    ui.add(egui::DragValue::new(&mut self.export.bars).range(1..=512).suffix(" bars"));
                });
            });
            ui.horizontal(|ui| {
                ui.label("Sample rate");
                for sr in [44100, 48000] {
                    ui.selectable_value(&mut self.export.sample_rate, sr, format!("{sr}"));
                }
            });
            ui.label(RichText::new("24-bit WAV, one bar of tail for reverbs and delays.").small().color(TEXT_DIM));
            ui.add_space(4.0);
            if ui.button(RichText::new("Export...").strong()).clicked() {
                self.start_export();
            }
        });
        if !open {
            self.export.open = false;
        }
    }
}
