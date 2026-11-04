//! Not losing work: the set is written beside the settings every so often
//! while it has changes (on a thread of its own, so nothing stutters), and
//! that copy is removed when Krabton closes normally. Finding one at the
//! start means it did not close normally: it offers to bring the set back.
//! Closing, or replacing a set, with changes that were not saved asks first.

use crate::app::App;
use crate::prefs::Prefs;
use crate::theme::*;
use egui::{Context, RichText};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const EVERY: Duration = Duration::from_secs(60);

/// What was going to happen when the question about unsaved changes came.
#[derive(Clone, Debug, PartialEq)]
pub enum After {
    Quit,
    New,
    Open,
    OpenPath(PathBuf),
    Demo,
    Ambient,
    Gallop,
}

impl After {
    fn doing(&self) -> &'static str {
        match self {
            After::Quit => "before closing",
            _ => "first",
        }
    }
}

pub struct Safety {
    /// The question being asked.
    pub asking: Option<After>,
    /// Closing is decided: no more questions.
    pub closing: bool,
    /// A copy from a session that did not end normally: when it was written.
    pub found: Option<(PathBuf, String)>,
    started: bool,
    last: Instant,
    writing: Arc<AtomicBool>,
}

impl Default for Safety {
    fn default() -> Self {
        Self { asking: None, closing: false, found: None, started: false, last: Instant::now(), writing: Arc::new(AtomicBool::new(false)) }
    }
}

/// Where the copy is kept (none in test windows, unless a test asks for it).
fn copy_path() -> Option<PathBuf> {
    if std::env::var_os("KRABTON_TEST_WINDOW").is_some() && std::env::var_os("KRABTON_TEST_AUTOSAVE").is_none() {
        return None;
    }
    Prefs::beside("autosave.krab")
}

fn every() -> Duration {
    std::env::var("KRABTON_TEST_AUTOSAVE").ok().and_then(|s| s.parse().ok()).map_or(EVERY, Duration::from_secs)
}

/// Writes to a temporary file first: a crash halfway leaves the last good copy.
fn write_copy(project: &krabton_engine::project::Project, to: &Path) -> Result<(), String> {
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = to.with_extension("tmp");
    project.save(&tmp)?;
    std::fs::rename(&tmp, to).map_err(|e| e.to_string())
}

fn age_of(path: &Path) -> String {
    let secs = std::fs::metadata(path).ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).map_or(0, |d| d.as_secs());
    match secs {
        0..=89 => "just now".to_string(),
        90..=5399 => format!("{} minutes ago", (secs + 30) / 60),
        5400..=129_599 => format!("{} hours ago", (secs + 1800) / 3600),
        _ => format!("{} days ago", (secs + 43200) / 86400),
    }
}

impl App {
    /// Once a frame: the copy, every so often, while there are changes.
    pub(crate) fn safety_frame(&mut self, ctx: &Context) {
        if !self.safety.started {
            self.safety.started = true;
            self.safety.last = Instant::now();
            if let Some(path) = copy_path().filter(|p| p.is_file()) {
                let age = age_of(&path);
                self.safety.found = Some((path, age));
            }
        }
        if self.dirty && self.safety.found.is_none() && self.safety.last.elapsed() >= every() && !self.safety.writing.load(Ordering::Acquire) {
            if let Some(path) = copy_path() {
                self.safety.last = Instant::now();
                let mut project = self.project.clone();
                self.plugins.store_states(&mut project);
                self.prefs.autosave_of = self.file_path.clone();
                self.prefs.save();
                let busy = self.safety.writing.clone();
                busy.store(true, Ordering::Release);
                std::thread::spawn(move || {
                    if let Err(e) = write_copy(&project, &path) {
                        eprintln!("autosave: {e}");
                    }
                    busy.store(false, Ordering::Release);
                });
            }
        }
        self.poll_close(ctx);
    }

    /// The window's close button and every other way of quitting come here.
    fn poll_close(&mut self, ctx: &Context) {
        let button = ctx.input(|i| i.viewport().close_requested());
        let asked = std::mem::take(&mut self.quit_requested);
        if !button && !asked {
            return;
        }
        if self.safety.closing || !self.dirty {
            self.safety.closing = true;
            if asked {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            return;
        }
        if button {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        self.safety.asking = Some(After::Quit);
    }

    /// With unsaved changes: asks first, and true means wait for the answer.
    pub(crate) fn ask_first(&mut self, after: After) -> bool {
        if !self.dirty || std::env::var_os("KRABTON_TEST_WINDOW").is_some() {
            return false;
        }
        self.safety.asking = Some(after);
        true
    }

    fn carry_on(&mut self, after: After) {
        match after {
            After::Quit => {
                self.safety.closing = true;
                self.quit_requested = true;
            }
            After::New => self.new_project(),
            After::Open => self.open(),
            After::OpenPath(p) => self.open_path(p),
            After::Demo => self.load_demo(),
            After::Ambient => self.load_ambient(),
            After::Gallop => self.load_gallop(),
        }
    }

    /// Closing normally: the copy is no longer needed.
    pub(crate) fn safety_exit(&mut self) {
        if let Some(path) = copy_path() {
            let _ = std::fs::remove_file(path);
        }
    }

    /// Saved by hand: nothing to bring back.
    pub(crate) fn safety_saved(&mut self) {
        if let Some(path) = copy_path() {
            let _ = std::fs::remove_file(path);
        }
    }

    pub(crate) fn ui_safety(&mut self, ctx: &Context) {
        if let Some(after) = self.safety.asking.clone() {
            let name = self.file_path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| self.project.name.clone());
            let mut answer = None;
            egui::Window::new("Unsaved changes").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).show(ctx, |ui| {
                ui.label(RichText::new(format!("Save the changes to {name} {}?", after.doing())).strong());
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Save").color(ACCENT)).clicked() {
                        answer = Some(0);
                    }
                    if ui.button("Don't Save").clicked() {
                        answer = Some(1);
                    }
                    if ui.button("Cancel").clicked() {
                        answer = Some(2);
                    }
                });
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                answer = Some(2);
            }
            match answer {
                Some(0) => {
                    self.safety.asking = None;
                    self.save(false);
                    // Its file dialog can be cancelled: then nothing goes on.
                    if !self.dirty {
                        self.carry_on(after);
                    }
                }
                Some(1) => {
                    self.safety.asking = None;
                    self.dirty = false;
                    self.carry_on(after);
                }
                Some(_) => self.safety.asking = None,
                None => {}
            }
        }
        if let Some((path, age)) = self.safety.found.clone() {
            let mut answer = None;
            egui::Window::new("Bring back your set?").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).show(ctx, |ui| {
                ui.label("Krabton did not close normally last time.");
                ui.label(RichText::new(format!("A copy of the set was kept, {age}.")).color(TEXT_DIM));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Bring It Back").color(ACCENT)).clicked() {
                        answer = Some(true);
                    }
                    if ui.button("Throw It Away").clicked() {
                        answer = Some(false);
                    }
                });
            });
            match answer {
                Some(true) => {
                    self.safety.found = None;
                    let original = self.prefs.autosave_of.clone();
                    self.open_path(path.clone());
                    // Saving goes where the set belongs, not over the copy.
                    self.file_path = original;
                    self.dirty = true;
                    self.toast("Your set is back; save it when you can");
                }
                Some(false) => {
                    self.safety.found = None;
                    let _ = std::fs::remove_file(&path);
                }
                None => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_replaces_the_last_only_when_it_is_whole() {
        let dir = std::env::temp_dir().join(format!("krabton-safety-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("autosave.krab");
        let p = krabton_engine::project::Project::new_default();
        write_copy(&p, &path).unwrap();
        assert!(path.is_file() && !path.with_extension("tmp").exists());
        let (back, missing) = krabton_engine::project::Project::load(&path).unwrap();
        assert!(missing.is_empty());
        assert_eq!(back.tracks.len(), p.tracks.len());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_says_how_long_ago_in_words() {
        let dir = std::env::temp_dir().join(format!("krabton-age-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("x");
        std::fs::write(&path, b"x").unwrap();
        assert_eq!(age_of(&path), "just now");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
