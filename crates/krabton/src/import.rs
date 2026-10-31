//! Audio files coming in from outside: dropped from the desktop (Explorer,
//! Finder, a file manager), picked with *Import Audio...* or an empty audio
//! slot's *Load Audio File...*, or a long one dragged out of the browser.
//! They are decoded on a thread of their own (a song as MP3 takes a moment),
//! with the status bar saying so, and then become clips the way samples from
//! the browser do: beat-locked to the project's tempo, named after the file.

use crate::app::{App, BrowserItem, DropTarget, TrackSel, View};
use crate::theme::{self, ACCENT};
use egui::{Context, Id, Pos2, Rect};
use krabton_engine::sample::{is_audio_file, is_external_audio_file, AUDIO_EXTENSIONS};
use krabton_engine::{SampleRef, TrackKind};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};

/// Files over this size are read in the background, from the browser too.
const SLOW_BYTES: u64 = 2_000_000;

/// Where files from outside go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum FileTarget {
    /// This slot and the ones below it, one file per scene.
    Slot { track: usize, scene: usize },
    /// A track's lane in the arrangement, one file after another from this beat.
    Lane { track: usize, beat: f64 },
    /// The track: from the selected scene, or the playhead's bar in the arrangement.
    Track(usize),
    /// A new audio track (in the arrangement from this beat, when given).
    NewTrack { beat: Option<f64> },
}

/// Files dragged in from outside, this frame.
#[derive(Default)]
pub(crate) struct FileDrag {
    /// The files while they hover over the window, or in the frame they drop.
    pub files: Vec<PathBuf>,
    pub dropped: bool,
    /// Where the pointer is; None when the system does not say.
    pub pos: Option<Pos2>,
    /// What the view under the pointer offers (set while drawing).
    pub target: Option<FileTarget>,
}

/// What happens once a file is read.
enum After {
    /// A browser item is applied (the target's track known by id, in case
    /// tracks move meanwhile).
    Apply { item: BrowserItem, target: DropTarget, track_id: Option<u64> },
    /// It is shown and played in the browser's preview, if still selected.
    Preview,
    /// It becomes a clip, the next one after the one before.
    Place(Placement),
}

/// Where the next clip of an import goes.
struct Placement {
    /// The track, by id; None makes a new audio track for the first clip.
    track: Option<u64>,
    /// In the session from this scene, or in the arrangement from this beat.
    scene: usize,
    beat: Option<f64>,
}

/// Files being read on their thread, in order.
struct Load {
    files: Vec<PathBuf>,
    rx: Receiver<(PathBuf, Result<SampleRef, String>)>,
    after: After,
    got: usize,
    placed: usize,
    failed: Vec<String>,
}

#[derive(Default)]
pub(crate) struct Imports {
    pub drag: FileDrag,
    /// Files a script drops (or holds over the window) at a point: see `run_script`.
    pub script: Option<(Vec<PathBuf>, Pos2, bool)>,
    loads: Vec<Load>,
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| p.display().to_string())
}

fn is_midi_file(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mid") || e.eq_ignore_ascii_case("midi"))
}

/// A file dialog for the audio files Krabton reads (WAV, FLAC, MP3, AIFF),
/// upper case too for the file systems that care.
pub(crate) fn audio_file_dialog() -> rfd::FileDialog {
    let exts: Vec<String> = AUDIO_EXTENSIONS.iter().flat_map(|e| [e.to_string(), e.to_uppercase()]).collect();
    rfd::FileDialog::new().add_filter("Audio files (WAV, FLAC, MP3, AIFF)", &exts)
}

/// The same, with SFZ instruments as well (for the Sampler).
pub(crate) fn audio_and_sfz_dialog() -> rfd::FileDialog {
    let exts: Vec<String> = AUDIO_EXTENSIONS.iter().chain(&["sfz"]).flat_map(|e| [e.to_string(), e.to_uppercase()]).collect();
    rfd::FileDialog::new().add_filter("Samples and SFZ", &exts)
}

/// Where the pointer is while files are dragged in from outside. Windows
/// tells the window nothing of the mouse during such a drag, so it is asked
/// where the cursor is on the screen; elsewhere the last pointer position
/// egui knows is used.
fn file_drag_pointer(ctx: &Context) -> Option<Pos2> {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct Point {
            x: i32,
            y: i32,
        }
        #[link(name = "user32")]
        extern "system" {
            fn GetCursorPos(p: *mut Point) -> i32;
        }
        let mut pt = Point { x: 0, y: 0 };
        // SAFETY: GetCursorPos writes one POINT to the pointer it is given.
        if unsafe { GetCursorPos(&mut pt) } != 0 {
            let inner = ctx.input(|i| i.viewport().inner_rect);
            if let Some(inner) = inner {
                let ppp = ctx.pixels_per_point();
                return Some(Pos2::new(pt.x as f32 / ppp, pt.y as f32 / ppp) - inner.min.to_vec2());
            }
        }
    }
    ctx.input(|i| i.pointer.latest_pos())
}

/// Where files dragged in from outside are this frame, for views that take
/// them themselves (the Granulator); see [`take_file_drop`].
pub(crate) fn file_drop_pos(ctx: &Context) -> Option<Pos2> {
    ctx.data(|d| d.get_temp::<Option<Pos2>>(Id::new("file_drag_pos"))).flatten()
}

/// A view took the files dropped this frame: they go nowhere else.
pub(crate) fn take_file_drop(ctx: &Context) {
    ctx.data_mut(|d| d.insert_temp(Id::new("file_drop_taken"), true));
}

impl App {
    /// Start of a frame: the files dragged in from outside, and where.
    pub(crate) fn file_drag_frame(&mut self, ctx: &Context) {
        // A script's files hover until they drop.
        let script = if self.imports.script.as_ref().is_some_and(|s| s.2) { self.imports.script.take() } else { self.imports.script.clone() };
        let (files, dropped, pos) = match script {
            Some((files, pos, dropped)) => (files, dropped, Some(pos)),
            None => {
                let (files, dropped) = ctx.input(|i| {
                    let dropped: Vec<PathBuf> = i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect();
                    if dropped.is_empty() {
                        (i.raw.hovered_files.iter().filter_map(|f| f.path.clone()).collect(), false)
                    } else {
                        (dropped, true)
                    }
                });
                let pos = if files.is_empty() { None } else { file_drag_pointer(ctx) };
                (files, dropped, pos)
            }
        };
        ctx.data_mut(|d| {
            d.insert_temp(Id::new("file_drag_pos"), pos);
            d.remove_temp::<bool>(Id::new("file_drop_taken"));
        });
        self.imports.drag = FileDrag { files, dropped, pos, target: None };
    }

    /// Whether files dragged in from outside are over `r`, which takes them
    /// as `target`: it gets a ring, and the files go there when dropped.
    pub(crate) fn file_over(&mut self, p: &egui::Painter, r: Rect, target: FileTarget) -> bool {
        let drag = &self.imports.drag;
        if drag.files.is_empty() || !drag.pos.is_some_and(|pos| r.contains(pos) && p.clip_rect().contains(pos)) {
            return false;
        }
        self.imports.drag.target = Some(target);
        p.rect_stroke(r.shrink(1.0), 2.0, egui::Stroke::new(2.0_f32, ACCENT), egui::StrokeKind::Inside);
        true
    }

    /// End of a frame: files dropped go where the view under the pointer
    /// said, or (dropped elsewhere, or where the system does not say) to the
    /// selected audio track or a new one. While they hover, a note at the
    /// pointer says what will happen.
    pub(crate) fn file_drop_frame(&mut self, ctx: &Context) {
        if self.imports.drag.files.is_empty() {
            return;
        }
        let target = self.imports.drag.target.unwrap_or_else(|| self.default_file_target());
        if !self.imports.drag.dropped {
            let files = &self.imports.drag.files;
            let what = match files.len() {
                1 => file_name(&files[0]),
                n => format!("{n} files"),
            };
            let note = format!("{what} {}", self.describe_target(target, files));
            let at = self.imports.drag.pos.unwrap_or_else(|| ctx.screen_rect().center_top() + egui::vec2(0.0, 60.0));
            let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, Id::new("file_drag_note")));
            let galley = p.layout_no_wrap(note, theme::small(), theme::TEXT);
            let r = Rect::from_min_size(at + egui::vec2(16.0, 18.0), galley.size() + egui::vec2(12.0, 8.0));
            p.rect_filled(r, 3.0, theme::PANEL);
            p.rect_stroke(r, 3.0, egui::Stroke::new(1.0_f32, ACCENT), egui::StrokeKind::Inside);
            p.galley(r.min + egui::vec2(6.0, 4.0), galley, theme::TEXT);
            return;
        }
        let taken = ctx.data(|d| d.get_temp::<bool>(Id::new("file_drop_taken"))).unwrap_or(false);
        let files = std::mem::take(&mut self.imports.drag.files);
        if !taken {
            self.import_files(files, target);
        }
    }

    /// Where files go that were not dropped on anything in particular.
    pub(crate) fn default_file_target(&self) -> FileTarget {
        match self.sel_track {
            TrackSel::Track(t) if self.project.tracks.get(t).is_some_and(|t| t.kind == TrackKind::Audio) => {
                if self.view == View::Arrangement {
                    FileTarget::Track(t)
                } else {
                    FileTarget::Slot { track: t, scene: self.sel_scene }
                }
            }
            _ => FileTarget::NewTrack { beat: None },
        }
    }

    /// What dropping these files there does, in a few words.
    fn describe_target(&self, target: FileTarget, files: &[PathBuf]) -> String {
        if files.iter().all(|p| is_midi_file(p)) {
            return "as MIDI".into();
        }
        if !files.iter().any(|p| is_audio_file(p) || is_external_audio_file(p) || is_midi_file(p)) {
            return "cannot be opened here".into();
        }
        let p = self.placement(target);
        let track = p.track.and_then(|id| self.project.tracks.iter().find(|t| t.id == id)).map(|t| format!("onto {}", t.name)).unwrap_or_else(|| "onto a new audio track".into());
        match p.beat {
            Some(beat) => format!("{track}, from bar {}", (beat / self.project.beats_per_bar_f()).floor() as i64 + 1),
            None => format!("{track}, from scene {}", p.scene + 1),
        }
    }

    /// Opens audio files with a file dialog and brings them in at `target`.
    pub(crate) fn load_audio_dialog(&mut self, target: FileTarget) {
        if let Some(files) = audio_file_dialog().pick_files() {
            self.import_files(files, target);
        }
    }

    /// *Import Audio...*: at the selected audio track's slot (or the
    /// playhead), or on a new audio track.
    pub(crate) fn import_audio_dialog(&mut self) {
        let target = self.default_file_target();
        self.load_audio_dialog(target);
    }

    /// Files from outside: audio becomes clips at `target` (read in the
    /// background), MIDI files open as from the browser.
    pub(crate) fn import_files(&mut self, files: Vec<PathBuf>, target: FileTarget) {
        let (midi, rest): (Vec<PathBuf>, Vec<PathBuf>) = files.into_iter().partition(|p| is_midi_file(p));
        for path in midi {
            match target {
                FileTarget::Slot { track, scene } => self.apply_item(&BrowserItem::MidiFile(path), DropTarget::Slot { track, scene }),
                FileTarget::Lane { track, beat } => self.apply_item(&BrowserItem::MidiFile(path), DropTarget::ArrLane { track, beat }),
                FileTarget::Track(t) => self.apply_item(&BrowserItem::MidiFile(path), DropTarget::Track(t)),
                FileTarget::NewTrack { beat: Some(beat) } => {
                    // Its tracks start on the bar it was dropped at.
                    let bpb = self.project.beats_per_bar_f();
                    let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    if let Some(song) = self.read_midi_file(&path) {
                        self.midi_file_tracks(&name, &song, Some((beat / bpb).round() * bpb));
                    }
                }
                FileTarget::NewTrack { beat: None } => self.apply_item(&BrowserItem::MidiFile(path), DropTarget::NewTrack),
            }
        }
        let (audio, other): (Vec<PathBuf>, Vec<PathBuf>) = rest.into_iter().partition(|p| is_audio_file(p) || is_external_audio_file(p) || p.extension().is_none());
        if let Some(p) = other.first() {
            self.toast(format!("Krabton cannot open {} here (audio files are WAV, FLAC, MP3 and AIFF)", file_name(p)));
        }
        if !audio.is_empty() {
            let placement = self.placement(target);
            self.start_load(audio, After::Place(placement));
        }
    }

    fn placement(&self, target: FileTarget) -> Placement {
        let audio = |t: usize| self.project.tracks.get(t).filter(|t| t.kind == TrackKind::Audio).map(|t| t.id);
        let bpb = self.project.beats_per_bar_f();
        let playhead_bar = (self.engine.status.song_pos.load() / bpb).floor() * bpb;
        let arr = self.view == View::Arrangement;
        match target {
            FileTarget::Slot { track, scene } => Placement { track: audio(track), scene, beat: None },
            FileTarget::Lane { track, beat } => Placement { track: audio(track), scene: self.sel_scene, beat: Some(beat.max(0.0)) },
            FileTarget::Track(t) => Placement { track: audio(t), scene: self.sel_scene, beat: arr.then_some(playhead_bar) },
            FileTarget::NewTrack { beat } => Placement { track: None, scene: self.sel_scene, beat: beat.map(|b| b.max(0.0)).or(arr.then_some(playhead_bar)) },
        }
    }

    /// Whether reading this file would hold the screen up (and so is done
    /// in the background).
    pub(crate) fn slow_to_load(&self, path: &Path) -> bool {
        !self.sample_cached(path) && std::fs::metadata(path).is_ok_and(|m| m.len() > SLOW_BYTES)
    }

    /// A browser sample applied once it is read (see `apply_item`).
    pub(crate) fn load_then_apply(&mut self, item: &BrowserItem, target: DropTarget) {
        let BrowserItem::Sample(path) = item else { return };
        let track = match target {
            DropTarget::Track(t) | DropTarget::Slot { track: t, .. } | DropTarget::ArrLane { track: t, .. } | DropTarget::Chain(TrackSel::Track(t)) => Some(t),
            _ => None,
        };
        let track_id = track.and_then(|t| self.project.tracks.get(t)).map(|t| t.id);
        self.start_load(vec![path.clone()], After::Apply { item: item.clone(), target, track_id });
    }

    /// A browser sample read for the preview in the background.
    pub(crate) fn load_for_preview(&mut self, path: &Path) {
        if !self.imports.loads.iter().any(|l| matches!(l.after, After::Preview) && l.files.iter().any(|f| f == path)) {
            self.start_load(vec![path.to_path_buf()], After::Preview);
        }
    }

    fn start_load(&mut self, files: Vec<PathBuf>, after: After) {
        let (tx, rx) = std::sync::mpsc::channel();
        let jobs: Vec<(PathBuf, Option<SampleRef>)> = files.iter().map(|p| (p.clone(), self.cached_sample(p))).collect();
        let spawned = std::thread::Builder::new().name("audio import".into()).spawn(move || {
            for (path, have) in jobs {
                let r = match have {
                    Some(s) => Ok(s),
                    None => SampleRef::load(&path),
                };
                if tx.send((path, r)).is_err() {
                    break;
                }
            }
        });
        match spawned {
            Ok(_) => self.imports.loads.push(Load { files, rx, after, got: 0, placed: 0, failed: Vec::new() }),
            Err(e) => self.toast(format!("Could not start reading the files: {e}")),
        }
    }

    /// Each frame: what the reading threads have brought, put in place.
    pub(crate) fn poll_imports(&mut self) {
        if self.imports.loads.is_empty() {
            return;
        }
        let mut loads = std::mem::take(&mut self.imports.loads);
        let mut busy = None;
        loads.retain_mut(|load| {
            loop {
                match load.rx.try_recv() {
                    Ok((path, Ok(sample))) => {
                        load.got += 1;
                        self.remember_sample(&path, &sample);
                        if self.file_loaded(&mut load.after, path, &sample) {
                            load.placed += 1;
                        }
                    }
                    Ok((_, Err(e))) => {
                        load.got += 1;
                        load.failed.push(e);
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        for f in &load.files[load.got.min(load.files.len())..] {
                            load.failed.push(format!("{}: could not be read", f.display()));
                        }
                        load.got = load.files.len();
                        break;
                    }
                }
            }
            if load.got < load.files.len() {
                if !matches!(load.after, After::Preview) || busy.is_none() {
                    busy = Some(file_name(&load.files[load.got]));
                }
                true
            } else {
                self.import_done(load);
                false
            }
        });
        // Loads started meanwhile come after these.
        loads.append(&mut self.imports.loads);
        self.imports.loads = loads;
        if let Some(name) = busy {
            self.toast(format!("Loading {name}..."));
        }
    }

    /// One file read: true when it went where it was meant to.
    fn file_loaded(&mut self, after: &mut After, path: PathBuf, sample: &SampleRef) -> bool {
        match after {
            After::Apply { item, target, track_id } => {
                // The track may have moved (or gone) while the file was read.
                let at = track_id.and_then(|id| self.project.tracks.iter().position(|t| t.id == id));
                let target = match (*target, at) {
                    (DropTarget::Track(_), Some(t)) => DropTarget::Track(t),
                    (DropTarget::Slot { scene, .. }, Some(t)) => DropTarget::Slot { track: t, scene },
                    (DropTarget::ArrLane { beat, .. }, Some(t)) => DropTarget::ArrLane { track: t, beat },
                    (DropTarget::Chain(TrackSel::Track(_)), Some(t)) => DropTarget::Chain(TrackSel::Track(t)),
                    (_, None) if track_id.is_some() => return false,
                    (other, _) => other,
                };
                let item = item.clone();
                self.apply_item(&item, target);
                true
            }
            After::Preview => {
                self.browser_sample_ready(&path);
                true
            }
            After::Place(p) => {
                self.place_clip(p, path, sample);
                true
            }
        }
    }

    /// The next clip of an import, after the one before it.
    fn place_clip(&mut self, p: &mut Placement, path: PathBuf, sample: &SampleRef) {
        let t = match p.track.and_then(|id| self.project.tracks.iter().position(|t| t.id == id)) {
            Some(t) => t,
            None => {
                let t = self.add_audio_track();
                self.project.tracks[t].name = sample.name.clone();
                p.track = Some(self.project.tracks[t].id);
                t
            }
        };
        let item = BrowserItem::Sample(path);
        match p.beat {
            Some(beat) => {
                self.apply_item(&item, DropTarget::ArrLane { track: t, beat });
                p.beat = Some(self.project.tracks[t].arrangement.last().map_or(beat, |a| a.end()));
            }
            None => {
                while p.scene >= self.project.scenes.len() {
                    let n = self.project.scenes.len();
                    self.project.add_scene(n);
                }
                self.apply_item(&item, DropTarget::Slot { track: t, scene: p.scene });
                p.scene += 1;
            }
        }
    }

    fn import_done(&mut self, load: &Load) {
        if let Some(e) = load.failed.first() {
            let more = if load.failed.len() > 1 { format!(" (and {} more)", load.failed.len() - 1) } else { String::new() };
            self.toast(format!("Could not load {e}{more}"));
        } else if let After::Place(_) = load.after {
            match load.placed {
                0 => {}
                1 => self.toast(format!("Imported {}", file_name(&load.files[0]))),
                n => self.toast(format!("Imported {n} audio files")),
            }
        } else if self.message.as_ref().is_some_and(|(m, _)| m.starts_with("Loading ")) {
            self.message = None;
        }
    }
}
