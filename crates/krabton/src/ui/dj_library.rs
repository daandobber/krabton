//! The DJ view's records: the folders on the left, the music of the chosen one
//! in the middle with its tempo, key and length, what is selected on the right.
//! A background thread listens to the records in the folder one by one; what it
//! finds is kept on disk, so it only has to hear a record once.

use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, RichText, Sense, Ui, UiBuilder};
use krabton_engine::dsp::{dj, turntable as tt};
use krabton_engine::SampleRef;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;

const ROW_H: f32 = 19.0;

/// What is known of a record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Info {
    pub bpm: f64,
    pub key: Option<(u8, bool)>,
    pub secs: f64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Column {
    Title,
    Artist,
    Bpm,
    Key,
    Length,
}

const COLUMNS: [(Column, &str, f32); 5] = [(Column::Title, "Title", 0.42), (Column::Artist, "Artist", 0.28), (Column::Bpm, "BPM", 0.1), (Column::Key, "Key", 0.08), (Column::Length, "Length", 0.12)];

struct Scan {
    stop: Arc<AtomicBool>,
    found: Receiver<(PathBuf, Option<Info>)>,
    left: usize,
}

pub struct Library {
    listed: Option<PathBuf>,
    pub files: Vec<PathBuf>,
    pub info: HashMap<PathBuf, Info>,
    loaded: bool,
    unsaved: usize,
    /// Records that would not open: not tried again.
    failed: HashSet<PathBuf>,
    scan: Option<Scan>,
    open: HashSet<PathBuf>,
    children: HashMap<PathBuf, Vec<PathBuf>>,
    sort: Column,
    down: bool,
    search: String,
    pub selected: Option<PathBuf>,
}

impl Default for Library {
    fn default() -> Self {
        Self { listed: None, files: Vec::new(), info: HashMap::new(), loaded: false, unsaved: 0, failed: HashSet::new(), scan: None, open: HashSet::new(), children: HashMap::new(), sort: Column::Title, down: false, search: String::new(), selected: None }
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        if let Some(s) = &self.scan {
            s.stop.store(true, Ordering::Relaxed);
        }
    }
}

/// A record's artist and title, from "Artist - Title" in its file name.
pub(crate) fn artist_title(path: &Path) -> (String, String) {
    let stem = path.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    split_name(&stem)
}

pub(crate) fn split_name(stem: &str) -> (String, String) {
    match stem.split_once(" - ") {
        Some((a, t)) => (a.trim().to_string(), t.trim().to_string()),
        None => (String::new(), stem.trim().to_string()),
    }
}

pub(crate) fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

/// The audio files of a folder.
pub(crate) fn audio_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.is_file() && krabton_engine::sample::is_audio_file(p)).collect()).unwrap_or_default();
    v.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    v
}

fn folders_in(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|d| d.flatten().map(|e| e.path()).filter(|p| p.is_dir() && !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'))).collect())
        .unwrap_or_default();
    v.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    v
}

/// The places to start from: your music and what was fetched by links.
fn places() -> Vec<(String, PathBuf)> {
    let mut v = Vec::new();
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from) {
        for name in ["Music", "Downloads", "Desktop"] {
            let p = home.join(name);
            if p.is_dir() {
                v.push((name.to_string(), p));
            }
        }
    }
    for (name, dir) in [("Films", crate::video::ytdl::films_dir()), ("From links (old)", crate::video::ytdl::old_films_dir())] {
        if dir.is_dir() {
            v.push((name.to_string(), dir));
        }
    }
    v
}

fn cache_path() -> PathBuf {
    krabton_engine::ffmpeg::library_dir().with_file_name("DJ").join("records.json")
}

/// The order of the records: `key` of two records, the unknown last.
pub(crate) fn sorted(files: &[PathBuf], info: &HashMap<PathBuf, Info>, by: Column, down: bool, search: &str) -> Vec<PathBuf> {
    let needle = search.trim().to_lowercase();
    let mut v: Vec<PathBuf> = files.iter().filter(|p| needle.is_empty() || p.file_stem().is_some_and(|n| n.to_string_lossy().to_lowercase().contains(&needle))).cloned().collect();
    let num = |p: &PathBuf| -> f64 {
        let i = info.get(p);
        match by {
            Column::Bpm => i.map_or(f64::INFINITY, |i| if i.bpm > 0.0 { i.bpm } else { f64::INFINITY }),
            Column::Length => i.map_or(f64::INFINITY, |i| i.secs),
            Column::Key => i.and_then(|i| i.key).map_or(f64::INFINITY, |k| {
                let (n, minor) = dj::wheel(k.0, k.1);
                n as f64 * 2.0 + if minor { 0.0 } else { 1.0 }
            }),
            _ => 0.0,
        }
    };
    let text = |p: &PathBuf| {
        let (a, t) = artist_title(p);
        (if by == Column::Artist { a } else { t }).to_lowercase()
    };
    v.sort_by(|a, b| {
        let order = match by {
            Column::Title | Column::Artist => text(a).cmp(&text(b)),
            _ => num(a).total_cmp(&num(b)),
        };
        if down {
            order.reverse()
        } else {
            order
        }
    });
    v
}

impl Library {
    fn load_cache(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        if let Ok(text) = std::fs::read_to_string(cache_path()) {
            if let Ok(saved) = serde_json::from_str::<HashMap<String, Info>>(&text) {
                self.info = saved.into_iter().map(|(k, v)| (PathBuf::from(k), v)).collect();
            }
        }
    }

    fn save_cache(&mut self) {
        self.unsaved = 0;
        let out: HashMap<String, Info> = self.info.iter().map(|(k, v)| (k.to_string_lossy().to_string(), *v)).collect();
        let path = cache_path();
        if let (Some(dir), Ok(text)) = (path.parent(), serde_json::to_string(&out)) {
            let _ = std::fs::create_dir_all(dir);
            let _ = std::fs::write(&path, text);
        }
    }

    /// A record that was put on a deck: known from now on.
    pub(crate) fn learn(&mut self, path: &Path, info: Info) {
        self.info.insert(path.to_path_buf(), info);
        self.save_cache();
    }

    /// Lists a folder and starts listening to what is not known yet.
    fn list(&mut self, folder: Option<PathBuf>) {
        if let Some(s) = self.scan.take() {
            s.stop.store(true, Ordering::Relaxed);
        }
        self.files = folder.as_deref().map(audio_files).unwrap_or_default();
        self.listed = folder;
        let todo: Vec<PathBuf> = self.files.iter().filter(|p| !self.info.contains_key(*p) && !self.failed.contains(*p)).cloned().collect();
        if todo.is_empty() {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();
        let (st, left) = (stop.clone(), todo.len());
        let spawned = std::thread::Builder::new().name("dj records".into()).spawn(move || {
            for path in todo {
                if st.load(Ordering::Relaxed) {
                    return;
                }
                let info = SampleRef::load(&path).ok().map(|s| {
                    let rec = tt::listen(&s);
                    Info { bpm: rec.bpm, key: krabton_engine::analysis::detect_key(&s).map(|k| (k.root, k.minor)), secs: s.duration() }
                });
                if tx.send((path, info)).is_err() {
                    return;
                }
            }
        });
        if spawned.is_ok() {
            self.scan = Some(Scan { stop, found: rx, left });
        }
    }

    fn poll(&mut self) {
        let mut done = false;
        if let Some(s) = &mut self.scan {
            while let Ok((path, info)) = s.found.try_recv() {
                s.left = s.left.saturating_sub(1);
                match info {
                    Some(i) => {
                        self.info.insert(path, i);
                        self.unsaved += 1;
                    }
                    None => {
                        self.failed.insert(path);
                    }
                }
            }
            done = s.left == 0;
        }
        if done {
            self.scan = None;
        }
        if self.unsaved > 0 && (done || self.unsaved >= 5) {
            self.save_cache();
        }
    }

    fn children_of(&mut self, dir: &Path) -> Vec<PathBuf> {
        self.children.entry(dir.to_path_buf()).or_insert_with(|| folders_in(dir)).clone()
    }
}

impl App {
    /// The records, across the bottom of the DJ view.
    pub(crate) fn dj_library(&mut self, ui: &mut Ui, rect: Rect) {
        self.dj.library.load_cache();
        let folder = self.prefs.dj_folder.clone();
        if self.dj.library.listed != folder {
            self.dj.library.list(folder.clone());
        }
        self.dj.library.poll();
        rect_fill(ui.painter(), rect, 3.0, PANEL2);
        let tree_w = (rect.width() * 0.18).clamp(130.0, 240.0);
        let info_w = (rect.width() * 0.2).clamp(150.0, 260.0);
        let tree = Rect::from_min_size(rect.min, vec2(tree_w, rect.height()));
        let info = Rect::from_min_max(pos2(rect.right() - info_w, rect.top()), rect.max);
        let list = Rect::from_min_max(pos2(tree.right() + 4.0, rect.top()), pos2(info.left() - 4.0, rect.bottom()));
        for (r, f) in [(tree, 0), (list, 1), (info, 2)] {
            let mut sub = ui.new_child(UiBuilder::new().max_rect(r.shrink(4.0)));
            sub.set_clip_rect(r);
            match f {
                0 => self.dj_tree(&mut sub),
                1 => self.dj_records(&mut sub),
                _ => self.dj_selected(&mut sub),
            }
        }
    }

    fn dj_tree(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Folders").small().strong().color(TEXT));
            if ui.small_button("Add...").on_hover_text("Choose the folder your music is in").clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    self.dj.library.open.insert(dir.clone());
                    self.prefs.dj_folder = Some(dir);
                    self.prefs.save();
                }
            }
        });
        egui::ScrollArea::vertical().id_salt("dj_folders").auto_shrink([false, false]).show(ui, |ui| {
            let mut roots = places();
            if let Some(f) = &self.prefs.dj_folder {
                if !roots.iter().any(|(_, p)| f.starts_with(p)) {
                    roots.insert(0, (f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| f.display().to_string()), f.clone()));
                }
            }
            for (name, path) in roots {
                self.dj_folder_row(ui, &name, &path, 0);
            }
        });
    }

    fn dj_folder_row(&mut self, ui: &mut Ui, name: &str, path: &Path, depth: usize) {
        let open = self.dj.library.open.contains(path);
        let chosen = self.prefs.dj_folder.as_deref() == Some(path);
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 10.0);
            if ui.add(egui::Label::new(RichText::new(if open { "v" } else { ">" }).small().color(TEXT_DIM)).sense(Sense::click())).clicked() {
                if open {
                    self.dj.library.open.remove(path);
                } else {
                    self.dj.library.open.insert(path.to_path_buf());
                    self.dj.library.children.remove(path);
                }
            }
            if ui.selectable_label(chosen, RichText::new(name).small()).clicked() {
                self.prefs.dj_folder = Some(path.to_path_buf());
                self.prefs.save();
            }
        });
        if open && depth < 8 {
            for child in self.dj.library.children_of(path) {
                let n = child.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                self.dj_folder_row(ui, &n, &child, depth + 1);
            }
        }
    }

    fn dj_records(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.dj.library.search).hint_text("Search").desired_width(160.0));
            let lib = &self.dj.library;
            ui.label(RichText::new(format!("{} files", lib.files.len())).small().color(TEXT_DIM));
            if let Some(s) = &lib.scan {
                ui.spinner();
                ui.label(RichText::new(format!("Listening: {} to go", s.left)).small().color(TEXT_DIM));
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
            }
        });
        let width = ui.available_width();
        let x_of = |k: usize| COLUMNS[..k].iter().map(|c| c.2).sum::<f32>() * width;
        // The heads: click to sort by it, again the other way round.
        let (head, _) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::hover());
        rect_fill(ui.painter(), head, 0.0, PANEL3);
        for (k, (col, name, part)) in COLUMNS.iter().enumerate() {
            let r = Rect::from_min_size(pos2(head.left() + x_of(k), head.top()), vec2(part * width, ROW_H));
            let resp = ui.interact(r, Id::new(("dj_head", k)), Sense::click());
            if resp.clicked() {
                let lib = &mut self.dj.library;
                if lib.sort == *col {
                    lib.down = !lib.down;
                } else {
                    (lib.sort, lib.down) = (*col, false);
                }
            }
            let arrow = if self.dj.library.sort == *col { if self.dj.library.down { " v" } else { " ^" } } else { "" };
            ui.painter().text(r.left_center() + vec2(3.0, 0.0), Align2::LEFT_CENTER, format!("{name}{arrow}"), small(), TEXT_DIM);
        }
        let lib = &self.dj.library;
        let rows = sorted(&lib.files, &lib.info, lib.sort, lib.down, &lib.search);
        if rows.is_empty() {
            ui.add_space(12.0);
            ui.label(RichText::new("Choose a folder on the left, or drag music onto a deck").small().color(TEXT_DIM));
            return;
        }
        let mut load = None;
        egui::ScrollArea::vertical().id_salt("dj_records").auto_shrink([false, false]).show_rows(ui, ROW_H, rows.len(), |ui, range| {
            for k in range {
                let path = &rows[k];
                let (r, _) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::hover());
                let resp = ui.interact(r, Id::new(("dj_row", path)), Sense::click_and_drag());
                if resp.clicked() {
                    self.dj.library.selected = Some(path.clone());
                }
                if resp.double_clicked() {
                    load = Some(path.clone());
                }
                resp.dnd_set_drag_payload(path.clone());
                let selected = self.dj.library.selected.as_ref() == Some(path);
                if selected {
                    rect_fill(ui.painter(), r, 0.0, Color32::from_gray(0x4a));
                } else if resp.hovered() {
                    rect_fill(ui.painter(), r, 0.0, PANEL3);
                }
                let (artist, title) = artist_title(path);
                let i = self.dj.library.info.get(path).copied();
                let cells = [
                    title,
                    artist,
                    i.filter(|i| i.bpm > 0.0).map_or(String::new(), |i| format!("{:.1}", i.bpm)),
                    i.and_then(|i| i.key).map_or(String::new(), |k| dj::wheel_name(k.0, k.1)),
                    i.map_or(String::new(), |i| clock(i.secs)),
                ];
                let p = ui.painter().with_clip_rect(r);
                for (c, text) in cells.iter().enumerate() {
                    let cell = Rect::from_min_size(pos2(r.left() + x_of(c), r.top()), vec2(COLUMNS[c].2 * width - 4.0, ROW_H));
                    p.with_clip_rect(cell).text(cell.left_center() + vec2(3.0, 0.0), Align2::LEFT_CENTER, text, small(), TEXT);
                }
            }
        });
        if let Some(path) = load {
            // A double click goes to the deck that is not playing.
            let free = if self.dj_deck_running(0) { 1 } else { 0 };
            self.dj_load(free, &path);
        }
    }

    fn dj_selected(&mut self, ui: &mut Ui) {
        let Some(path) = self.dj.library.selected.clone() else {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| ui.label(RichText::new("Drag a record onto a deck, or double-click it").small().color(TEXT_DIM)));
            return;
        };
        let (artist, title) = artist_title(&path);
        ui.label(RichText::new(title).strong().color(TEXT));
        if !artist.is_empty() {
            ui.label(RichText::new(artist).small().color(TEXT_DIM));
        }
        if let Some(i) = self.dj.library.info.get(&path).copied() {
            if i.bpm > 0.0 {
                ui.label(RichText::new(format!("BPM  {:.2}", i.bpm)).small().color(TEXT));
            }
            if let Some(k) = i.key {
                let name = krabton_engine::analysis::Key { root: k.0, minor: k.1, confidence: 1.0 }.name();
                ui.label(RichText::new(format!("Key  {} ({})", dj::wheel_name(k.0, k.1), name)).small().color(TEXT));
            }
            ui.label(RichText::new(format!("Length  {}", clock(i.secs))).small().color(TEXT));
        }
        ui.horizontal(|ui| {
            if ui.button("Load A").clicked() {
                self.dj_load(0, &path);
            }
            if ui.button("Load B").clicked() {
                self.dj_load(1, &path);
            }
        });
        ui.add(egui::Label::new(RichText::new(path.display().to_string()).small().color(TEXT_DIM)).wrap());
        let _ = FontId::proportional(10.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_name_tells_artist_and_title() {
        assert_eq!(split_name("David Bowie - Space Oddity"), ("David Bowie".to_string(), "Space Oddity".to_string()));
        assert_eq!(split_name("Just A Title"), (String::new(), "Just A Title".to_string()));
        assert_eq!(split_name("A - B - C"), ("A".to_string(), "B - C".to_string()));
    }

    #[test]
    fn the_clock_reads_minutes_and_seconds() {
        assert_eq!(clock(65.4), "1:05");
        assert_eq!(clock(-3.0), "0:00");
    }

    #[test]
    fn a_folder_lists_its_audio_by_name_and_its_folders_apart() {
        let dir = std::env::temp_dir().join(format!("krabton_dj_lib_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Sub")).unwrap();
        std::fs::create_dir_all(dir.join(".hidden")).unwrap();
        for f in ["b.MP3", "a.wav", "notes.txt", "c.flac"] {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        let names: Vec<String> = audio_files(&dir).iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect();
        let folders = folders_in(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(names, ["a.wav", "b.MP3", "c.flac"]);
        assert_eq!(folders.len(), 1, "{folders:?}");
    }

    #[test]
    fn records_sort_by_any_column_the_unknown_last_and_search_narrows() {
        let (a, b, c) = (PathBuf::from("Zed - One.wav"), PathBuf::from("Abe - Two.wav"), PathBuf::from("Mia - Three.wav"));
        let files = vec![a.clone(), b.clone(), c.clone()];
        let mut info = HashMap::new();
        info.insert(a.clone(), Info { bpm: 128.0, key: Some((9, true)), secs: 200.0 });
        info.insert(b.clone(), Info { bpm: 90.0, key: Some((0, false)), secs: 100.0 });
        assert_eq!(sorted(&files, &info, Column::Bpm, false, ""), vec![b.clone(), a.clone(), c.clone()]);
        assert_eq!(sorted(&files, &info, Column::Artist, false, ""), vec![b.clone(), c.clone(), a.clone()]);
        assert_eq!(sorted(&files, &info, Column::Title, true, ""), vec![b.clone(), c.clone(), a.clone()]);
        assert_eq!(sorted(&files, &info, Column::Title, false, "mia"), vec![c.clone()]);
        // 8A sits just before 8B on the wheel.
        assert_eq!(sorted(&files[..2], &info, Column::Key, false, ""), vec![a.clone(), b.clone()]);
    }
}
