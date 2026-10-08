//! The browser: instruments, effects and samples from disk. Items can be
//! double-clicked (applies to the selection) or dragged onto tracks/slots.

use crate::app::{App, BrowserItem};
use crate::theme::*;
use egui::{RichText, Sense, Ui};
use krabton_engine::dsp::effects::EffectKind;
use krabton_engine::dsp::{drums, synth};
use krabton_engine::Command;
use std::path::{Path, PathBuf};

pub struct BrowserState {
    pub folder: Option<PathBuf>,
    pub entries: Vec<(PathBuf, bool)>,
    pub filter: String,
}

impl BrowserState {
    pub fn new() -> Self {
        let mut s = Self { folder: None, entries: Vec::new(), filter: String::new() };
        let music = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(|h| PathBuf::from(h).join("Music"));
        if let Some(m) = music.filter(|m| m.is_dir()) {
            s.set_folder(m);
        }
        s
    }

    pub fn set_folder(&mut self, dir: PathBuf) {
        self.entries.clear();
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten().take(5000) {
                let path = e.path();
                let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if name.starts_with('.') {
                    continue;
                }
                if path.is_dir() {
                    self.entries.push((path, true));
                } else if is_wav(&path) {
                    self.entries.push((path, false));
                }
            }
        }
        self.entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.file_name().cmp(&b.0.file_name())));
        self.folder = Some(dir);
    }
}

fn is_wav(p: &Path) -> bool {
    p.extension().map(|e| e.eq_ignore_ascii_case("wav")).unwrap_or(false)
}

impl App {
    pub(crate) fn ui_browser(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Browser").strong());
        });
        ui.add(egui::TextEdit::singleline(&mut self.browser.filter).hint_text("Search...").desired_width(f32::INFINITY));
        ui.add_space(2.0);
        let filter = self.browser.filter.to_lowercase();
        let matches = |s: &str| filter.is_empty() || s.to_lowercase().contains(&filter);

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::CollapsingHeader::new(RichText::new("Instruments").color(ACCENT)).default_open(true).show(ui, |ui| {
                ui.label(RichText::new("Krab Synth").small().color(TEXT_DIM));
                for p in synth::preset_names() {
                    if matches(p) || matches("synth") {
                        self.browser_item(ui, BrowserItem::Synth(p), p, "Two-oscillator virtual analog synth");
                    }
                }
                ui.label(RichText::new("Krab Drums").small().color(TEXT_DIM));
                for k in drums::kit_names() {
                    if matches(k) || matches("drums") {
                        self.browser_item(ui, BrowserItem::Drums(k), k, "Synthesized drum rack (GM note mapping)");
                    }
                }
                ui.label(RichText::new("Krab Sampler").small().color(TEXT_DIM));
                if matches("sampler") {
                    self.browser_item(ui, BrowserItem::Sampler, "Empty Sampler", "Plays one sample across the keyboard. Drop a WAV on it.");
                }
            });
            egui::CollapsingHeader::new(RichText::new("Krab Macro (Plaits)").color(ACCENT)).default_open(true).show(ui, |ui| {
                for p in krabton_engine::dsp::macro_osc::preset_names() {
                    if matches(p) || matches("macro") || matches("plaits") || matches("analog") {
                        self.browser_item(ui, BrowserItem::Macro(p), p, "24 synthesis models from Mutable Instruments Plaits: analog, FM, speech, strings, drums");
                    }
                }
            });
            egui::CollapsingHeader::new(RichText::new("Krab Braids").color(ACCENT)).default_open(true).show(ui, |ui| {
                for p in krabton_engine::dsp::braids_synth::preset_names() {
                    if matches(p) || matches("braids") || matches("analog") {
                        self.browser_item(ui, BrowserItem::Braids(p), p, "47 shapes from Mutable Instruments Braids: analog, FM, physical models, wavetables, noise");
                    }
                }
            });
            egui::CollapsingHeader::new(RichText::new("Krab Elements").color(ACCENT)).default_open(true).show(ui, |ui| {
                for p in krabton_engine::dsp::elements_synth::preset_names() {
                    if matches(p) || matches("elements") || matches("modal") || matches("physical") {
                        self.browser_item(ui, BrowserItem::Elements(p), p, "Modal synthesis from Mutable Instruments Elements: bow, blow and strike a resonator or strings");
                    }
                }
            });
            egui::CollapsingHeader::new(RichText::new("Sequencers").color(ACCENT)).default_open(true).show(ui, |ui| {
                for k in krabton_engine::midi_fx::MidiFxKind::SEQUENCERS {
                    if matches(k.name()) || matches("sequencer") || matches("seq") {
                        self.browser_item(ui, BrowserItem::MidiFx(k), k.name(), k.description());
                    }
                }
            });
            egui::CollapsingHeader::new(RichText::new("MIDI Effects").color(ACCENT)).default_open(true).show(ui, |ui| {
                for k in krabton_engine::midi_fx::MidiFxKind::ALL {
                    if matches(k.name()) || matches("midi") {
                        self.browser_item(ui, BrowserItem::MidiFx(k), k.name(), k.description());
                    }
                }
            });
            egui::CollapsingHeader::new(RichText::new("Krab FM (DX7)").color(ACCENT)).default_open(true).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Load .syx...").on_hover_text("Load DX7 banks or voices (Dexed / MiniDexed .syx files)").clicked() {
                        if let Some(files) = rfd::FileDialog::new().add_filter("DX7 sysex", &["syx", "SYX"]).pick_files() {
                            let mut ok = 0;
                            for f in files {
                                match self.load_fm_bank(&f) {
                                    Ok(_) => ok += 1,
                                    Err(e) => self.toast(e),
                                }
                            }
                            if ok > 0 {
                                self.toast(format!("Loaded {ok} bank(s)"));
                            }
                        }
                    }
                    if ui.button("Folder...").on_hover_text("Load every .syx in a folder, e.g. MiniDexed's sysex/voice").clicked() {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            let n = self.load_fm_folder(&dir);
                            self.toast(format!("Loaded {n} bank(s)"));
                        }
                    }
                });
                let banks = self.fm_banks.clone();
                for (bi, bank) in banks.iter().enumerate() {
                    let show = |ui: &mut Ui, app: &mut App| {
                        for v in &bank.voices {
                            let name = v.name();
                            if matches(&name) || matches(&bank.name) || matches("fm") {
                                let item = BrowserItem::FmVoice { bank: bank.name.clone(), voice: v.0.to_vec() };
                                app.browser_item(ui, item, &name, "Krab FM voice: double-click or drag onto a track");
                            }
                        }
                    };
                    if bi == 0 {
                        show(ui, self);
                    } else {
                        egui::CollapsingHeader::new(RichText::new(&bank.name).small()).id_salt(("fm_bank", bi)).default_open(false).show(ui, |ui| show(ui, self));
                    }
                }
            });
            egui::CollapsingHeader::new(RichText::new("Plugins (CLAP)").color(ACCENT)).default_open(true).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(if self.plugins.scanned { "Rescan" } else { "Scan" }).on_hover_text("Look for CLAP plugins in the standard folders").clicked() {
                        self.plugins.scan();
                        let n = self.plugins.instruments().count();
                        self.toast(format!("Found {n} CLAP instrument(s)"));
                    }
                    if ui.button("Folder...").on_hover_text("Add a folder with .clap files").clicked() {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            self.plugins.add_folder(dir);
                        }
                    }
                });
                let instruments: Vec<_> = self.plugins.instruments().cloned().collect();
                if instruments.is_empty() {
                    ui.label(
                        RichText::new("No CLAP instruments found. Install e.g. Dexed, Surge XT or Odin 2 (CLAP version), or add a folder.")
                            .small()
                            .color(TEXT_DIM),
                    );
                }
                for info in instruments {
                    if matches(&info.name) || matches(&info.vendor) || matches("plugin") {
                        let hint = format!("{} by {}\n{}\n{}", info.name, info.vendor, info.description, info.path.display());
                        let label = info.name.clone();
                        self.browser_item(ui, BrowserItem::Plugin(info), &label, &hint);
                    }
                }
                let fx = self.plugins.effect_count();
                if fx > 0 {
                    ui.label(RichText::new(format!("{fx} effect plugin(s) found (effects not supported yet)")).small().color(TEXT_DIM));
                }
                if !self.plugins.scan_errors.is_empty() {
                    let errs = self.plugins.scan_errors.join("\n");
                    ui.label(RichText::new(format!("{} file(s) could not be loaded", self.plugins.scan_errors.len())).small().color(REC)).on_hover_text(errs);
                }
            });
            egui::CollapsingHeader::new(RichText::new("Audio Effects").color(ACCENT)).default_open(true).show(ui, |ui| {
                for k in EffectKind::ALL {
                    if matches(k.name()) {
                        self.browser_item(ui, BrowserItem::Effect(k), k.name(), k.description());
                    }
                }
            });
            egui::CollapsingHeader::new(RichText::new("Samples").color(ACCENT)).default_open(true).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Folder...").on_hover_text("Choose a folder with WAV files").clicked() {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            self.browser.set_folder(dir);
                        }
                    }
                    let parent = self.browser.folder.as_ref().and_then(|f| f.parent().map(|p| p.to_path_buf()));
                    if let Some(parent) = parent {
                        if ui.button("Up").clicked() {
                            self.browser.set_folder(parent);
                        }
                    }
                });
                match self.browser.folder.clone() {
                    None => {
                        ui.label(RichText::new("No folder chosen").small().color(TEXT_DIM));
                    }
                    Some(folder) => {
                        let name = folder.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| folder.display().to_string());
                        ui.label(RichText::new(name).small().color(TEXT_DIM)).on_hover_text(folder.display().to_string());
                        let entries = self.browser.entries.clone();
                        if entries.is_empty() {
                            ui.label(RichText::new("No WAV files here").small().color(TEXT_DIM));
                        }
                        for (path, is_dir) in entries {
                            let fname = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                            if !matches(&fname) {
                                continue;
                            }
                            if is_dir {
                                if ui.selectable_label(false, RichText::new(format!("[{fname}]")).color(TEXT_DIM)).clicked() {
                                    self.browser.set_folder(path);
                                }
                            } else {
                                self.browser_item(ui, BrowserItem::Sample(path.clone()), &fname, "Click to preview, drag onto a track or slot");
                            }
                        }
                    }
                }
            });
            ui.add_space(8.0);
            ui.label(RichText::new("Tip: double-click an item to add it to the selected track, or drag it onto a track, slot or the drop area.").small().color(TEXT_DIM));
        });
    }

    fn browser_item(&mut self, ui: &mut Ui, item: BrowserItem, label: &str, hint: &str) {
        let resp = ui.selectable_label(false, RichText::new(format!("  {label}")).color(TEXT)).interact(Sense::click_and_drag());
        resp.dnd_set_drag_payload(item.clone());
        if resp.clicked() {
            if let BrowserItem::Sample(path) = &item {
                if let Some(s) = self.load_sample(path) {
                    self.cmd(Command::Preview(Some(s)));
                }
            }
        }
        if resp.double_clicked() {
            let target = self.default_target();
            self.apply_item(&item, target);
        }
        resp.on_hover_text(hint);
    }
}
