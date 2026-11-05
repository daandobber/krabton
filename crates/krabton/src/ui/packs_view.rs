//! The Packs window: what Krabton can fetch, what is here and how big it is,
//! with "Download selected", "Download all" and removing. And the Update
//! window: what a new version brings, and putting it in place.

use crate::app::App;
use crate::packs::{self, Kind, Pack, Progress};
use crate::theme::*;
use crate::update::{self, Check, Fetch, How};
use egui::{Align, Context, Layout, RichText, Ui};

fn size(mb: u64) -> String {
    if mb >= 1000 {
        format!("{:.1} GB", mb as f32 / 1000.0)
    } else {
        format!("{mb} MB")
    }
}

impl App {
    pub(crate) fn ui_packs(&mut self, ctx: &Context) {
        if !self.packs.open {
            return;
        }
        if !self.packs.surveyed() {
            self.packs.resurvey();
        }
        let list = packs::all();
        let (open, _) = super::windows::tool_window(ctx, "packs", "Packs", egui::vec2(640.0, 560.0), |ui| self.packs_body(ui, &list));
        if !open {
            self.packs.open = false;
            self.packs.welcome = false;
        }
    }

    fn packs_body(&mut self, ui: &mut Ui, list: &[Pack]) {
        if self.packs.welcome {
            ui.label(RichText::new("Welcome to Krabton").strong().color(ACCENT));
            ui.label("It plays without any of these. Tick what you want and it comes while you work; this window is in the Krabton menu whenever you want more.");
            ui.separator();
        }
        let (here, disk) = list.iter().filter_map(|p| self.packs.survey.get(&p.kind)).fold((0, 0u64), |(n, b), s| (n + s.0 as usize, b + s.1));
        ui.horizontal(|ui| {
            if self.packs.surveyed() {
                ui.label(RichText::new(format!("{here} of {} here, {} on this disk", list.len(), size(disk / 1_000_000))).color(TEXT_DIM));
            } else {
                ui.spinner();
                ui.label(RichText::new("Looking what is here").color(TEXT_DIM));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(path) = crate::piano_lib::library_dir().parent() {
                    if ui.small_button("Show folder").on_hover_text(path.display().to_string()).clicked() {
                        let _ = std::fs::create_dir_all(path);
                        let _ = std::process::Command::new(update::opener()).arg(path).spawn();
                    }
                }
            });
        });
        ui.add_space(4.0);
        let foot = 40.0;
        egui::ScrollArea::vertical().max_height((ui.available_height() - foot).max(80.0)).auto_shrink([false, false]).show(ui, |ui| {
            for (id, title) in packs::GROUPS {
                self.packs_group(ui, list, id, title);
            }
        });
        ui.separator();
        // What the two big buttons would bring.
        let picked: Vec<Kind> = list.iter().filter(|p| self.packs.picked.contains(&p.kind)).map(|p| p.kind).collect();
        let picked_mb: u64 = list.iter().filter(|p| self.packs.picked.contains(&p.kind)).map(|p| p.mb as u64).sum();
        let missing: Vec<&Pack> = list.iter().filter(|p| !self.packs.is_here(p.kind) && !self.pack_on_its_way(p.kind) && !packs::by_name_only(p.kind)).collect();
        let all: Vec<Kind> = missing.iter().map(|p| p.kind).collect();
        let all_mb: u64 = missing.iter().map(|p| p.mb as u64).sum();
        ui.horizontal(|ui| {
            let label = if picked.is_empty() { "Download selected".to_string() } else { format!("Download selected ({}, {})", picked.len(), size(picked_mb)) };
            if ui.add_enabled(!picked.is_empty(), egui::Button::new(label)).clicked() {
                self.queue_packs(picked);
            }
            if ui.add_enabled(!all.is_empty() && self.packs.surveyed(), egui::Button::new(format!("Download all ({})", size(all_mb)))).clicked() {
                self.queue_packs(all);
            }
            if self.packs.left() > 0 {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Stop").on_hover_text("The one under way finishes; the rest stay away").clicked() {
                        self.packs.queue.clear();
                    }
                    ui.label(RichText::new(format!("{} to go", self.packs.left())).color(TEXT_DIM));
                });
            }
        });
    }

    fn pack_on_its_way(&self, kind: Kind) -> bool {
        self.packs.running == Some(kind) || self.packs.queue.contains(&kind)
    }

    fn packs_group(&mut self, ui: &mut Ui, list: &[Pack], id: &str, title: &str) {
        let members: Vec<&Pack> = list.iter().filter(|p| p.group == id).collect();
        // One pack: no need to fold it open.
        if let [one] = members.as_slice() {
            self.pack_row(ui, one, Some(title));
            return;
        }
        let open_kinds: Vec<Kind> = members.iter().filter(|p| !self.packs.is_here(p.kind) && !self.pack_on_its_way(p.kind) && !packs::by_name_only(p.kind)).map(|p| p.kind).collect();
        let ticked = open_kinds.iter().filter(|k| self.packs.picked.contains(k)).count();
        let here = members.iter().filter(|p| self.packs.is_here(p.kind)).count();
        let mb: u64 = members.iter().map(|p| p.mb as u64).sum();
        let state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), ui.id().with(("pack_group", id)), false);
        state
            .show_header(ui, |ui| {
                let mut all = !open_kinds.is_empty() && ticked == open_kinds.len();
                let check = egui::Checkbox::new(&mut all, RichText::new(title).strong()).indeterminate(ticked > 0 && ticked < open_kinds.len());
                if ui.add_enabled(!open_kinds.is_empty(), check).on_hover_text("Everything in this group that is not here yet").changed() {
                    for k in &open_kinds {
                        if all {
                            self.packs.picked.insert(*k);
                        } else {
                            self.packs.picked.remove(k);
                        }
                    }
                }
                ui.label(RichText::new(format!("{here} of {} here, {} in all", members.len(), size(mb))).color(TEXT_DIM).small());
            })
            .body(|ui| {
                for p in &members {
                    self.pack_row(ui, p, None);
                }
            });
    }

    fn pack_row(&mut self, ui: &mut Ui, p: &Pack, heading: Option<&str>) {
        let kind = p.kind;
        let here = self.packs.is_here(kind);
        ui.horizontal(|ui| {
            let name = match heading {
                Some(h) => RichText::new(h).strong(),
                None => RichText::new(p.name.as_str()),
            };
            if here {
                ui.label(RichText::new("✔").color(ACCENT));
                ui.label(name).on_hover_text(&p.about);
            } else {
                let mut on = self.packs.picked.contains(&kind);
                if ui.add_enabled(!self.pack_on_its_way(kind), egui::Checkbox::new(&mut on, name)).on_hover_text(&p.about).changed() {
                    if on {
                        self.packs.picked.insert(kind);
                    } else {
                        self.packs.picked.remove(&kind);
                    }
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let bytes = self.packs.survey.get(&kind).map_or(0, |s| s.1);
                if self.packs.running == Some(kind) {
                    match self.pack_progress(kind) {
                        Progress::Busy(f, text) => {
                            ui.add(egui::ProgressBar::new(f).desired_width(190.0).text(text));
                        }
                        _ => {
                            ui.spinner();
                        }
                    }
                } else if self.packs.queue.contains(&kind) {
                    ui.label(RichText::new("Waiting").color(TEXT_DIM));
                } else if here && bytes > 0 {
                    if self.packs.removing == Some(kind) {
                        if ui.small_button("Cancel").clicked() {
                            self.packs.removing = None;
                        }
                        if ui.small_button(RichText::new("Yes, remove").color(REC)).clicked() {
                            self.packs.removing = None;
                            self.remove_pack(kind);
                        }
                    } else {
                        if ui.small_button("Remove").on_hover_text("Off this disk; you can fetch it again").clicked() {
                            self.packs.removing = Some(kind);
                        }
                        ui.label(RichText::new(size(bytes.div_ceil(1_000_000))).color(TEXT_DIM));
                    }
                } else if here {
                    ui.label(RichText::new("On this computer").color(TEXT_DIM));
                } else if let Some(why) = self.packs.failed.get(&kind).cloned() {
                    if ui.small_button("Try again").clicked() {
                        self.queue_packs([kind]);
                    }
                    ui.label(RichText::new("Did not come").color(REC)).on_hover_text(why);
                } else if p.mb == 0 {
                    ui.label(RichText::new("Made here").color(TEXT_DIM));
                } else {
                    ui.label(RichText::new(size(p.mb as u64)).color(TEXT_DIM));
                }
            });
        });
    }

    /// The status bar: a new version, and packs on their way.
    pub(crate) fn ui_status_pills(&mut self, ui: &mut Ui) {
        if let Some(r) = self.update.found().filter(|r| self.prefs.update_skip.as_deref() != Some(r.version.as_str())) {
            let label = RichText::new(format!("Update {}", r.version)).small().color(TEXT_DARK);
            if ui.add(egui::Button::new(label).fill(ACCENT).min_size(egui::vec2(0.0, 16.0))).on_hover_text("A new Krabton is out").clicked() {
                self.update.open = true;
            }
        }
        if let Some(kind) = self.packs.running {
            let percent = match self.pack_progress(kind) {
                Progress::Busy(f, _) => (f * 100.0) as u32,
                _ => 100,
            };
            let name = packs::all().into_iter().find(|p| p.kind == kind).map(|p| p.name).unwrap_or_default();
            let left = self.packs.left();
            let text = if left > 1 { format!("{name} {percent}% (+{} more)", left - 1) } else { format!("{name} {percent}%") };
            if ui.add(egui::Button::new(RichText::new(text).small()).min_size(egui::vec2(0.0, 16.0))).on_hover_text("Packs on their way: open the Packs window").clicked() {
                self.packs.open = true;
            }
        }
    }

    pub(crate) fn ui_update(&mut self, ctx: &Context) {
        if !self.update.open {
            return;
        }
        let (open, _) = super::windows::tool_window(ctx, "update", "Update", egui::vec2(520.0, 420.0), |ui| self.update_body(ui));
        if !open {
            self.update.open = false;
        }
    }

    fn update_body(&mut self, ui: &mut Ui) {
        let check = self.update.check.lock().map(|c| c.clone()).unwrap_or(Check::Idle);
        ui.label(RichText::new(format!("This is Krabton {}", update::current())).color(TEXT_DIM));
        ui.add_space(4.0);
        match check {
            Check::Idle => {
                if ui.button("Check for updates").clicked() {
                    self.update.check_now();
                }
            }
            Check::Checking => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Asking GitHub");
                });
            }
            Check::UpToDate => {
                ui.label(RichText::new("You have the newest version.").strong());
                if ui.button("Check again").clicked() {
                    self.update.check_now();
                }
            }
            Check::Failed(e) => {
                ui.label(RichText::new(e).color(REC));
                if ui.button("Try again").clicked() {
                    self.update.check_now();
                }
            }
            Check::Found(r) => self.update_found(ui, &r),
        }
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            let mut on = !self.prefs.update_check_off;
            if ui.checkbox(&mut on, "Look for updates when Krabton starts").on_hover_text("Asks GitHub which version is the newest, nothing else").changed() {
                self.prefs.update_check_off = !on;
                self.prefs.save();
            }
        });
    }

    fn update_found(&mut self, ui: &mut Ui, r: &update::Release) {
        ui.label(RichText::new(format!("Krabton {} is out", r.version)).strong().color(ACCENT));
        if r.changes.len() > 1 {
            ui.label(RichText::new(format!("What is new in the {} versions since yours (you get the newest, with all of them in it):", r.changes.len())).small().color(TEXT_DIM));
        }
        if !r.changes.is_empty() {
            egui::ScrollArea::vertical().max_height((ui.available_height() - 110.0).max(60.0)).auto_shrink([false, true]).show(ui, |ui| {
                let several = r.changes.len() > 1;
                for (version, notes) in &r.changes {
                    if several {
                        ui.add_space(4.0);
                        ui.label(RichText::new(version.as_str()).small().strong());
                    }
                    ui.label(RichText::new(notes.as_str()).small());
                }
            });
        }
        ui.add_space(6.0);
        let asset = update::asset_for(r, &self.update.how).cloned();
        let fetch = self.update.fetch.as_ref().and_then(|f| f.lock().ok().map(|f| f.clone()));
        if let Some(why) = self.update.trouble.clone() {
            ui.label(RichText::new(why).color(REC));
        }
        match (fetch, asset) {
            (_, None) => {
                let why = if self.update.how == How::ByHand { "This copy cannot replace itself (a package manager's, or in a folder Krabton may not write to)." } else { "This release has no file for this computer yet." };
                ui.label(RichText::new(why).color(TEXT_DIM));
                ui.horizontal(|ui| {
                    if ui.button("Open the download page").clicked() {
                        update::open_page(&r.page);
                    }
                    self.update_skip_button(ui, r);
                });
            }
            (None, Some(asset)) => {
                ui.horizontal(|ui| {
                    if ui.button(format!("Download ({} MB)", asset.bytes / 1_000_000)).on_hover_text("You can keep working; nothing changes until you restart").clicked() {
                        self.update.trouble = None;
                        self.update.fetch = Some(update::start_fetch(asset.clone()));
                    }
                    self.update_skip_button(ui, r);
                });
            }
            (Some(Fetch::Getting { bytes, total }), _) => {
                ui.add(egui::ProgressBar::new((bytes as f32 / total.max(1) as f32).min(1.0)).text(format!("{} of {} MB; you can keep working", bytes / 1_000_000, total / 1_000_000)));
            }
            (Some(Fetch::Failed(e)), Some(asset)) => {
                ui.label(RichText::new(e).color(REC));
                ui.horizontal(|ui| {
                    if ui.button("Try again").clicked() {
                        self.update.fetch = Some(update::start_fetch(asset.clone()));
                    }
                    if ui.button("Open the download page").clicked() {
                        update::open_page(&r.page);
                    }
                });
            }
            (Some(Fetch::Ready(file)), _) => self.update_ready(ui, &file),
        }
    }

    fn update_skip_button(&mut self, ui: &mut Ui, r: &update::Release) {
        if ui.button("Skip this version").on_hover_text("No more telling about this one").clicked() {
            self.prefs.update_skip = Some(r.version.clone());
            self.prefs.save();
            self.update.open = false;
        }
    }

    fn update_ready(&mut self, ui: &mut Ui, file: &std::path::Path) {
        ui.label("It is here. Restarting puts it in place and brings your saved set back; the music stops for a moment.");
        ui.horizontal(|ui| {
            if self.dirty {
                if ui.button("Save and restart").clicked() {
                    self.save(false);
                    if !self.dirty {
                        self.restart_into_update(file);
                    }
                }
                if ui.button(RichText::new("Restart without saving").color(REC)).clicked() {
                    self.restart_into_update(file);
                }
            } else if ui.button("Restart now").clicked() {
                self.restart_into_update(file);
            }
            if ui.button("Later").on_hover_text("It waits; the Update button stays below").clicked() {
                self.update.open = false;
            }
        });
    }
}
