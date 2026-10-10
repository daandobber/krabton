//! Application state, frame loop, commands, undo and keyboard handling.

use crate::audio::{self, AudioOutput};
use crate::midi::{Midi, MidiMsg};
use crate::plugins::PluginManager;
use crate::theme;
use crate::ui::arrangement::ArrState;
use crate::ui::browser::BrowserState;
use crate::ui::piano_roll::PianoState;
use crate::ui::session::SessionState;
use egui::{Context, Key, Modifiers};
use krabton_engine::dsp::effects::EffectKind;
use krabton_engine::engine::EngineStatus;
use krabton_engine::render::{self, RenderProgress, RenderSource};
use krabton_engine::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Session,
    Arrangement,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DetailTab {
    Clip,
    Devices,
    Mod,
    Grid,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrackSel {
    Track(usize),
    Return(usize),
    Master,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClipSel {
    Slot { track: usize, scene: usize },
    Arr { track: usize, id: u64 },
}

impl ClipSel {
    pub fn track(&self) -> usize {
        match *self {
            ClipSel::Slot { track, .. } | ClipSel::Arr { track, .. } => track,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenameTarget {
    Track(usize),
    Return(usize),
    Scene(usize),
    Clip(ClipSel),
}

/// Something dragged out of the browser.
#[derive(Clone, Debug)]
pub enum BrowserItem {
    Synth(&'static str),
    Drums(&'static str),
    Sampler,
    Effect(EffectKind),
    Sample(PathBuf),
    Plugin(krabton_clap::PluginInfo),
    /// An FM voice (155-byte DX7 voice) from a bank.
    FmVoice { bank: String, voice: Vec<u8> },
    MidiFx(krabton_engine::midi_fx::MidiFxKind),
    /// A Plaits preset.
    Macro(&'static str),
    /// A Braids preset.
    Braids(&'static str),
    /// An Elements preset.
    Elements(&'static str),
    /// An Analog preset.
    Analog(&'static str),
    /// A copy of a clip in this project.
    Clip { id: u64, name: String },
    /// An SFZ instrument, played by the Sampler.
    SamplerPatch(PathBuf),
}

/// A bank of DX7 voices for FM.
#[derive(Clone)]
pub struct FmBank {
    pub name: String,
    pub path: Option<PathBuf>,
    pub voices: Vec<krabton_engine::dsp::dx7::Dx7Voice>,
}

impl BrowserItem {
    pub fn label(&self) -> String {
        match self {
            BrowserItem::Synth(p) => format!("Synth: {p}"),
            BrowserItem::Macro(p) => format!("Plaits: {p}"),
            BrowserItem::Braids(p) => format!("Braids: {p}"),
            BrowserItem::Elements(p) => format!("Elements: {p}"),
            BrowserItem::Analog(p) => format!("Analog: {p}"),
            BrowserItem::Drums(k) => k.to_string(),
            BrowserItem::Sampler => "Sampler".into(),
            BrowserItem::Effect(k) => k.name().into(),
            BrowserItem::Sample(p) => p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(),
            BrowserItem::Plugin(p) => p.name.clone(),
            BrowserItem::MidiFx(k) => k.name().into(),
            BrowserItem::FmVoice { voice, .. } => {
                krabton_engine::dsp::dx7::Dx7Voice::from_slice(voice).map(|v| format!("FM: {}", v.name())).unwrap_or_default()
            }
            BrowserItem::Clip { name, .. } => name.clone(),
            BrowserItem::SamplerPatch(p) => p.file_stem().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(),
        }
    }
}

/// Where a browser item was dropped.
#[derive(Clone, Copy, Debug)]
pub enum DropTarget {
    Track(usize),
    Slot { track: usize, scene: usize },
    ArrLane { track: usize, beat: f64 },
    Chain(TrackSel),
    NewTrack,
}

/// Structural edits requested from inside the UI. They change the number
/// or order of tracks/scenes/returns, so they run after the frame is drawn
/// (views loop over indices while drawing).
#[derive(Clone, Copy, Debug)]
pub enum Deferred {
    DeleteTrack(usize),
    DuplicateTrack(usize),
    MoveTrack { from: usize, to: usize },
    InsertScene(usize),
    DuplicateScene(usize),
    DeleteScene(usize),
    DeleteReturn(usize),
    /// A new armed audio track listening to this track.
    RecordIntoAudio(usize),
}

/// Drag payload: an arrangement clip being dragged (it can be dropped on a
/// session slot after switching views).
#[derive(Clone, Copy, Debug)]
pub struct ArrClipDrag {
    pub id: u64,
}

/// A session clip being dragged to another slot.
#[derive(Clone, Copy, Debug)]
pub struct SlotDrag {
    pub track: usize,
    pub scene: usize,
}

struct RecNote {
    scene: usize,
    start: f64,
    vel: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExportSource {
    Arrangement,
    Scene,
}

pub struct ExportDialog {
    pub open: bool,
    pub source: ExportSource,
    pub scene: usize,
    pub bars: u32,
    pub sample_rate: u32,
}

type ExportResult = (Result<String, String>, Vec<Box<dyn krabton_engine::plugin::AudioProcessor>>);

pub struct ExportJob {
    pub progress: Arc<RenderProgress>,
    pub handle: Option<JoinHandle<ExportResult>>,
    /// Plugin instances used by the render; dropped on this thread afterwards.
    plugin_instances: Vec<krabton_clap::PluginInstance>,
}

/// Target of the Key & Scale dialog.
#[derive(Clone, Copy, Debug)]
pub struct ScaleDialog {
    pub root: u8,
    pub kind: ScaleKind,
}

/// In "scale keys" mode the home row plays consecutive scale steps and the
/// row above continues an octave (7 steps) higher.
const SCALE_KEYMAP: [(Key, i32); 20] = [
    (Key::A, 0),
    (Key::S, 1),
    (Key::D, 2),
    (Key::F, 3),
    (Key::G, 4),
    (Key::H, 5),
    (Key::J, 6),
    (Key::K, 7),
    (Key::L, 8),
    (Key::Semicolon, 9),
    (Key::Quote, 10),
    (Key::W, 7),
    (Key::E, 8),
    (Key::R, 9),
    (Key::T, 10),
    (Key::Y, 11),
    (Key::U, 12),
    (Key::I, 13),
    (Key::O, 14),
    (Key::P, 15),
];

const KEYMAP: [(Key, i32); 18] = [
    (Key::A, 0),
    (Key::W, 1),
    (Key::S, 2),
    (Key::E, 3),
    (Key::D, 4),
    (Key::F, 5),
    (Key::T, 6),
    (Key::G, 7),
    (Key::Y, 8),
    (Key::H, 9),
    (Key::U, 10),
    (Key::J, 11),
    (Key::K, 12),
    (Key::O, 13),
    (Key::L, 14),
    (Key::P, 15),
    (Key::Semicolon, 16),
    (Key::Quote, 17),
];

pub struct App {
    pub project: Project,
    /// Structural edits to apply at the end of the frame.
    pub deferred: Vec<Deferred>,
    pub plugins: PluginManager,
    pub fm_banks: Vec<FmBank>,
    /// monome grid/arc and the virtual grid.
    pub grid: crate::grid::GridState,
    /// Modulator in map mode: clicking or dragging knobs connects it.
    pub mod_learn: Option<u64>,
    /// Modulator shown in the Mod tab editor.
    pub sel_mod: Option<u64>,
    /// Recent outputs per modulator id, for the scopes.
    pub mod_history: HashMap<u64, std::collections::VecDeque<f32>>,
    last_sent: Arc<Project>,
    committed: Project,
    undo: Vec<Project>,
    redo: Vec<Project>,
    pub dirty: bool,
    pub engine: EngineHandle,
    _audio: AudioOutput,
    pub audio_desc: String,
    pub midi: Midi,
    pending: Vec<Command>,

    pub view: View,
    pub detail: DetailTab,
    pub sel_track: TrackSel,
    pub sel_clip: Option<ClipSel>,
    pub sel_scene: usize,
    pub file_path: Option<PathBuf>,
    pub message: Option<(String, Instant)>,

    pub kb_enabled: bool,
    pub kb_octave: i32,
    pub kb_velocity: f32,
    held_keys: HashMap<Key, (u8, Vec<u64>)>,
    midi_held: HashMap<u8, (u8, Vec<u64>)>,
    /// Snap played notes (keyboard and MIDI) into the project scale.
    pub snap_input: bool,
    /// Computer keyboard plays scale steps instead of chromatic notes.
    pub scale_keys: bool,
    pub scale_dialog: Option<ScaleDialog>,
    pub recording: bool,
    /// Audio recordings in progress.
    pub takes: Vec<crate::recording::Take>,
    /// The audio interface input, opened when a track needs it.
    pub input_dev: Option<crate::audio::AudioInputDevice>,
    pub input_error: Option<String>,
    pub rec_counter: u32,
    pub rec_quantize: bool,
    rec_notes: HashMap<(u64, u8), RecNote>,

    pub meters: Vec<(f32, f32)>,
    pub return_meters: Vec<(f32, f32)>,
    pub master_meter: (f32, f32),
    last_frame: Instant,

    pub session: SessionState,
    pub arr: ArrState,
    pub piano: PianoState,
    pub browser: BrowserState,
    /// Settings kept between sessions.
    pub prefs: crate::prefs::Prefs,
    pub sel_pad: usize,
    /// The selected zone of a Sampler.
    pub sel_zone: usize,
    pub pending_pad_hit: Option<u8>,
    sample_cache: HashMap<PathBuf, SampleRef>,

    pub renaming: Option<(RenameTarget, String)>,
    pub export: ExportDialog,
    pub export_job: Option<ExportJob>,
    pub show_about: bool,
    pub clipboard: Option<crate::editing::Clipboard>,
    title: String,
    /// Debug actions from `KRABTON_SCRIPT`, as (frame, action).
    script: Vec<(u64, String)>,
    /// Pointer events from script actions, one batch per frame. They are fed
    /// to this window only (never to the desktop).
    script_input: std::collections::VecDeque<Vec<egui::Event>>,
    pub(crate) frame: u64,
    /// Close the window at the end of this frame (script action `quit`).
    quit_requested: bool,
    /// A size changed: save the layout once the mouse is up.
    pub layout_dirty: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let project = krabton_engine::demo::demo_project();
        let snapshot = Arc::new(project.clone());

        let probe = if std::env::var_os("KRABTON_NO_AUDIO").is_some() { Err("disabled by KRABTON_NO_AUDIO".to_string()) } else { audio::probe() };
        let (audio_out, engine, audio_desc) = match probe {
            Ok((device, config)) => {
                let sr = config.sample_rate().0 as f32;
                let (engine, handle) = Engine::new(sr, snapshot.clone());
                match audio::start(device, config, engine) {
                    Ok(out) => {
                        let d = out.description();
                        (out, handle, d)
                    }
                    Err(e) => {
                        let (engine, handle) = Engine::new(48000.0, snapshot.clone());
                        (audio::start_null(engine), handle, format!("Audio error: {e}"))
                    }
                }
            }
            Err(e) => {
                let (engine, handle) = Engine::new(48000.0, snapshot.clone());
                (audio::start_null(engine), handle, format!("No audio: {e}"))
            }
        };

        let mut midi = Midi::new();
        midi.auto_connect();

        let mut plugins = PluginManager::new();
        plugins.scan();
        let mut app = Self {
            deferred: Vec::new(),
            plugins,
            fm_banks: vec![FmBank::factory()],
            grid: crate::grid::GridState::new(),
            mod_learn: None,
            sel_mod: None,
            mod_history: HashMap::new(),
            committed: project.clone(),
            project,
            last_sent: snapshot,
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: false,
            engine,
            _audio: audio_out,
            audio_desc,
            midi,
            pending: Vec::new(),
            view: View::Session,
            detail: DetailTab::Clip,
            sel_track: TrackSel::Track(0),
            sel_clip: Some(ClipSel::Slot { track: 0, scene: 1 }),
            sel_scene: 0,
            file_path: None,
            message: None,
            kb_enabled: true,
            kb_octave: 3,
            kb_velocity: 100.0 / 127.0,
            held_keys: HashMap::new(),
            midi_held: HashMap::new(),
            snap_input: false,
            scale_keys: false,
            scale_dialog: None,
            recording: false,
            takes: Vec::new(),
            input_dev: None,
            input_error: None,
            rec_counter: 0,
            rec_quantize: true,
            rec_notes: HashMap::new(),
            meters: Vec::new(),
            return_meters: Vec::new(),
            master_meter: (0.0, 0.0),
            last_frame: Instant::now(),
            session: SessionState::default(),
            arr: ArrState::default(),
            piano: PianoState::default(),
            browser: BrowserState::new(),
            prefs: crate::prefs::Prefs::load(),
            sel_pad: 0,
            sel_zone: 0,
            pending_pad_hit: None,
            sample_cache: HashMap::new(),
            renaming: None,
            export: ExportDialog { open: false, source: ExportSource::Scene, scene: 0, bars: 8, sample_rate: 44100 },
            export_job: None,
            show_about: false,
            clipboard: None,
            title: String::new(),
            script: parse_script(std::env::var("KRABTON_SCRIPT").unwrap_or_default().as_str()),
            script_input: std::collections::VecDeque::new(),
            frame: 0,
            quit_requested: false,
            layout_dirty: false,
        };
        app.apply_prefs();
        if app.prefs.ui_zoom > 0.0 {
            cc.egui_ctx.set_zoom_factor(app.prefs.ui_zoom.clamp(0.5, 3.0));
        }
        let hint = if app.midi.connected.is_some() { "" } else { " Play notes with A-W-S-E-D... (Z/X = octave)." };
        app.toast(format!("Welcome to Krabton! Press Space or launch a scene.{hint}"));
        app
    }

    // ------------------------------------------------------------------
    // Engine plumbing
    // ------------------------------------------------------------------

    pub fn cmd(&mut self, c: Command) {
        self.pending.push(c);
    }

    pub fn status(&self) -> Arc<EngineStatus> {
        self.engine.status.clone()
    }

    pub fn is_playing(&self) -> bool {
        self.engine.status.playing.load(Ordering::Relaxed)
    }

    pub fn toast(&mut self, msg: impl Into<String>) {
        self.message = Some((msg.into(), Instant::now()));
    }

    /// Sends the project snapshot (if it changed) followed by queued commands.
    fn sync(&mut self, ctx: &Context) {
        if *self.last_sent != self.project {
            // Connections to deleted tracks, devices or modulators go away.
            self.project.prune_mod_routes();
            let snap = Arc::new(self.project.clone());
            self.engine.send(Command::SetProject(snap.clone()));
            self.last_sent = snap;
        }
        for c in self.pending.drain(..) {
            self.engine.send(c);
        }
        let pointer_down = ctx.input(|i| i.pointer.any_down());
        if !pointer_down && self.project != self.committed {
            let prev = std::mem::replace(&mut self.committed, self.project.clone());
            self.undo.push(prev);
            if self.undo.len() > 200 {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.dirty = true;
        }
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            let cur = std::mem::replace(&mut self.project, prev);
            self.redo.push(cur);
            self.committed = self.project.clone();
            self.dirty = true;
            self.toast("Undo");
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            let cur = std::mem::replace(&mut self.project, next);
            self.undo.push(cur);
            self.committed = self.project.clone();
            self.dirty = true;
            self.toast("Redo");
        }
    }

    fn validate_selection(&mut self) {
        let nt = self.project.tracks.len();
        let ns = self.project.scenes.len();
        self.sel_track = match self.sel_track {
            TrackSel::Track(i) if i >= nt => {
                if nt > 0 {
                    TrackSel::Track(nt - 1)
                } else {
                    TrackSel::Master
                }
            }
            TrackSel::Return(i) if i >= self.project.returns.len() => TrackSel::Master,
            s => s,
        };
        self.sel_scene = self.sel_scene.min(ns.saturating_sub(1));
        if let Some(sel) = self.sel_clip {
            if self.clip_ref(sel).is_none() {
                self.sel_clip = None;
            }
        }
    }

    // ------------------------------------------------------------------
    // Clips
    // ------------------------------------------------------------------

    pub fn clip_ref(&self, sel: ClipSel) -> Option<&Clip> {
        match sel {
            ClipSel::Slot { track, scene } => self.project.tracks.get(track)?.slots.get(scene)?.as_ref(),
            ClipSel::Arr { track, id } => self.project.tracks.get(track)?.arrangement.iter().find(|a| a.id == id).map(|a| &a.clip),
        }
    }

    pub fn clip_mut(&mut self, sel: ClipSel) -> Option<&mut Clip> {
        match sel {
            ClipSel::Slot { track, scene } => self.project.tracks.get_mut(track)?.slots.get_mut(scene)?.as_mut(),
            ClipSel::Arr { track, id } => self.project.tracks.get_mut(track)?.arrangement.iter_mut().find(|a| a.id == id).map(|a| &mut a.clip),
        }
    }

    /// Playback position (beats) inside the clip if it is currently playing.
    pub fn clip_play_pos(&self, sel: ClipSel) -> Option<f64> {
        let st = &self.engine.status;
        let ts = st.tracks.get(sel.track())?;
        let playing = match sel {
            ClipSel::Slot { scene, .. } => ts.playing_slot.load(Ordering::Relaxed) == scene as i32,
            ClipSel::Arr { id, .. } => ts.arr_clip.load(Ordering::Relaxed) == id,
        };
        let pos = ts.clip_pos.load();
        (playing && pos >= 0.0).then_some(pos)
    }

    pub fn select_clip(&mut self, sel: ClipSel) {
        self.sel_clip = Some(sel);
        self.sel_track = TrackSel::Track(sel.track());
        if let ClipSel::Slot { scene, .. } = sel {
            self.sel_scene = scene;
        }
        self.detail = DetailTab::Clip;
    }

    pub fn create_midi_clip(&mut self, track: usize, scene: usize) {
        let bpb = self.project.beats_per_bar_f();
        let id = self.project.alloc_id();
        let Some(t) = self.project.tracks.get_mut(track) else { return };
        if t.kind != TrackKind::Midi || t.slots.get(scene).map_or(true, |s| s.is_some()) {
            return;
        }
        t.slots[scene] = Some(Clip::new_midi(id, "MIDI Clip", t.color, bpb));
        self.select_clip(ClipSel::Slot { track, scene });
    }

    pub fn delete_clip(&mut self, sel: ClipSel) {
        match sel {
            ClipSel::Slot { track, scene } => {
                if let Some(Some(_)) = self.project.tracks.get(track).and_then(|t| t.slots.get(scene)) {
                    self.project.tracks[track].slots[scene] = None;
                }
            }
            ClipSel::Arr { track, id } => {
                if let Some(t) = self.project.tracks.get_mut(track) {
                    t.arrangement.retain(|a| a.id != id);
                }
            }
        }
        if self.sel_clip == Some(sel) {
            self.sel_clip = None;
        }
    }

    pub fn duplicate_clip(&mut self, sel: ClipSel) {
        match sel {
            ClipSel::Slot { track, scene } => {
                let Some(clip) = self.clip_ref(sel).cloned() else { return };
                let ns = self.project.scenes.len();
                let target = (scene + 1..ns).find(|&s| self.project.tracks[track].slots[s].is_none());
                let target = match target {
                    Some(s) => s,
                    None => {
                        self.project.add_scene(scene + 1);
                        scene + 1
                    }
                };
                let c = self.project.fresh_clip(&clip);
                self.project.tracks[track].slots[target] = Some(c);
                self.select_clip(ClipSel::Slot { track, scene: target });
            }
            ClipSel::Arr { track, id } => {
                let Some(ac) = self.project.tracks.get(track).and_then(|t| t.arrangement.iter().find(|a| a.id == id)).cloned() else { return };
                let new_id = self.project.alloc_id();
                let clip = self.project.fresh_clip(&ac.clip);
                let copy = ArrangementClip { id: new_id, start: ac.end(), clip, ..ac };
                self.project.tracks[track].arrangement.push(copy);
                self.select_clip(ClipSel::Arr { track, id: new_id });
            }
        }
    }

    pub fn copy_clip_to_arrangement(&mut self, sel: ClipSel) {
        let Some(clip) = self.clip_ref(sel).cloned() else { return };
        let track = sel.track();
        let start = self.engine.status.song_pos.load().max(0.0);
        let bar = self.project.beats_per_bar_f();
        let start = (start / bar).floor() * bar;
        let id = self.project.alloc_id();
        let clip = self.project.fresh_clip(&clip);
        let length = clip.length;
        self.project.tracks[track].arrangement.push(ArrangementClip { id, start, length, offset: 0.0, clip });
        self.toast(format!("Copied to arrangement at bar {}", (start / bar) as i64 + 1));
    }

    pub fn move_slot_clip(&mut self, from: SlotDrag, to: (usize, usize), copy: bool) {
        if (from.track, from.scene) == to {
            return;
        }
        let Some(clip) = self.clip_ref(ClipSel::Slot { track: from.track, scene: from.scene }).cloned() else { return };
        let (tt, ts) = to;
        let Some(target) = self.project.tracks.get(tt) else { return };
        if target.kind != self.project.tracks[from.track].kind || target.slots.get(ts).is_none() {
            self.toast("Clips can only move between tracks of the same type");
            return;
        }
        let clip = if copy { self.project.fresh_clip(&clip) } else { clip };
        if !copy {
            self.project.tracks[from.track].slots[from.scene] = None;
        }
        self.project.tracks[tt].slots[ts] = Some(clip);
        self.select_clip(ClipSel::Slot { track: tt, scene: ts });
    }

    // ------------------------------------------------------------------
    // Tracks
    // ------------------------------------------------------------------

    pub fn add_midi_track(&mut self, inst: Option<Instrument>) -> usize {
        let i = self.project.add_midi_track(inst);
        self.sel_track = TrackSel::Track(i);
        self.detail = DetailTab::Devices;
        i
    }

    pub fn add_audio_track(&mut self) -> usize {
        let i = self.project.add_audio_track();
        self.sel_track = TrackSel::Track(i);
        i
    }

    pub fn delete_track(&mut self, i: usize) {
        if i < self.project.tracks.len() {
            self.project.tracks.remove(i);
            self.sel_clip = None;
            self.validate_selection();
        }
    }

    pub fn load_sample(&mut self, path: &Path) -> Option<SampleRef> {
        if let Some(s) = self.sample_cache.get(path) {
            return Some(s.clone());
        }
        match SampleRef::load(path) {
            Ok(s) => {
                self.sample_cache.insert(path.to_path_buf(), s.clone());
                Some(s)
            }
            Err(e) => {
                self.toast(format!("Could not load sample: {e}"));
                None
            }
        }
    }

    /// The default drop target for double-clicks in the browser.
    pub fn default_target(&self) -> DropTarget {
        match (self.sel_track, self.sel_clip) {
            (TrackSel::Track(t), Some(ClipSel::Slot { track, scene })) if t == track => DropTarget::Slot { track, scene },
            (TrackSel::Track(t), _) if self.view == View::Session => DropTarget::Slot { track: t, scene: self.sel_scene },
            (TrackSel::Track(t), _) => DropTarget::Track(t),
            (sel, _) => DropTarget::Chain(sel),
        }
    }

    pub fn apply_item(&mut self, item: &BrowserItem, target: DropTarget) {
        let target_track = match target {
            DropTarget::Track(t) | DropTarget::Slot { track: t, .. } | DropTarget::ArrLane { track: t, .. } => Some(t),
            DropTarget::Chain(TrackSel::Track(t)) => Some(t),
            _ => None,
        };
        let is_midi = |app: &App, t: usize| app.project.tracks.get(t).map(|t| t.kind == TrackKind::Midi).unwrap_or(false);
        match item {
            BrowserItem::MidiFx(kind) => {
                let sel = match self.sel_track {
                    TrackSel::Track(t) if is_midi(self, t) => Some(t),
                    _ => None,
                };
                let track = match target {
                    DropTarget::NewTrack => {
                        use krabton_engine::midi_fx::MidiFxKind as K;
                        // A fitting instrument for each sequencer.
                        let inst = match kind {
                            K::Euclid | K::DrumGrid => Instrument::drums("909 Classic"),
                            K::Acid => Instrument::synth("Acid"),
                            K::Progression => Instrument::synth("Soft Pad"),
                            _ => Instrument::synth("Pluck"),
                        };
                        let t = self.add_midi_track(Some(inst));
                        Some(t)
                    }
                    _ => target_track.filter(|&t| is_midi(self, t)).or(sel),
                };
                match track {
                    Some(t) => {
                        let dev = self.project.make_midi_device(*kind);
                        let chain = &mut self.project.tracks[t].midi_devices;
                        if kind.is_sequencer() {
                            // Sequencers go first so arpeggiators, chords etc. can shape their notes.
                            let at = chain.iter().take_while(|d| d.kind.is_sequencer()).count();
                            chain.insert(at, dev);
                            if !self.is_playing() {
                                self.toast(format!("{} added. Press Space to hear it.", kind.name()));
                            }
                        } else {
                            chain.push(dev);
                        }
                        self.sel_track = TrackSel::Track(t);
                        self.show_devices();
                    }
                    None => self.toast("MIDI effects go on MIDI tracks"),
                }
            }
            BrowserItem::FmVoice { bank, voice } => {
                let Some(v) = krabton_engine::dsp::dx7::Dx7Voice::from_slice(voice) else { return };
                let name = v.name();
                match target_track {
                    Some(t) if is_midi(self, t) => {
                        let tr = &mut self.project.tracks[t];
                        let auto = tr.name.ends_with("MIDI") || tr.instrument.as_ref().map(|i| i.track_name() == tr.name || i.display_name() == tr.name).unwrap_or(true);
                        if let Some(Instrument::Fm { voice: cur, bank: b, .. }) = &mut tr.instrument {
                            // Keep the engine settings, swap the voice.
                            *cur = v.0.to_vec();
                            *b = bank.clone();
                        } else {
                            tr.instrument = Some(Instrument::fm(&v, bank));
                        }
                        if auto {
                            tr.name = "FM".into();
                        }
                        let _ = name;
                        self.sel_track = TrackSel::Track(t);
                        self.detail = DetailTab::Devices;
                    }
                    _ => {
                        self.add_midi_track(Some(Instrument::fm(&v, bank)));
                        let _ = name;
                    }
                }
            }
            BrowserItem::Plugin(info) => {
                let instance = self.project.alloc_id();
                let inst = Instrument::Plugin(PluginManager::make_instrument(info, instance));
                match target_track {
                    Some(t) if is_midi(self, t) => {
                        let tr = &mut self.project.tracks[t];
                        tr.name = info.name.clone();
                        tr.instrument = Some(inst);
                        self.sel_track = TrackSel::Track(t);
                        self.detail = DetailTab::Devices;
                    }
                    _ => {
                        let t = self.add_midi_track(Some(inst));
                        self.project.tracks[t].name = info.name.clone();
                    }
                }
                self.toast(format!("Loading {} ...", info.name));
            }
            BrowserItem::Synth(_) | BrowserItem::Drums(_) | BrowserItem::Sampler | BrowserItem::Macro(_) | BrowserItem::Braids(_) | BrowserItem::Elements(_) | BrowserItem::Analog(_) => {
                let inst = match item {
                    BrowserItem::Synth(p) => Instrument::synth(p),
                    BrowserItem::Macro(p) => Instrument::macro_osc(p),
                    BrowserItem::Braids(p) => Instrument::braids(p),
                    BrowserItem::Elements(p) => Instrument::elements(p),
                    BrowserItem::Analog(p) => Instrument::analog(p),
                    BrowserItem::Drums(k) => Instrument::drums(k),
                    _ => Instrument::sampler(None),
                };
                match target_track {
                    Some(t) if is_midi(self, t) => {
                        let tr = &mut self.project.tracks[t];
                        // A name Krabton gave follows the new instrument; one you typed stays.
                        let generic = tr.name.ends_with("MIDI") || tr.instrument.as_ref().map(|i| i.track_name() == tr.name || i.display_name() == tr.name).unwrap_or(true);
                        if generic {
                            tr.name = inst.track_name();
                        }
                        tr.instrument = Some(inst);
                        self.sel_track = TrackSel::Track(t);
                        self.detail = DetailTab::Devices;
                    }
                    _ => {
                        self.add_midi_track(Some(inst));
                    }
                }
            }
            BrowserItem::Effect(kind) => {
                let dev = self.project.make_device(*kind);
                match target {
                    DropTarget::Chain(TrackSel::Return(r)) => {
                        if let Some(ret) = self.project.returns.get_mut(r) {
                            ret.devices.push(dev);
                        }
                    }
                    DropTarget::Chain(TrackSel::Master) => self.project.master.devices.push(dev),
                    DropTarget::NewTrack => {
                        let t = self.add_audio_track();
                        self.project.tracks[t].devices.push(dev);
                    }
                    _ => {
                        if let Some(t) = target_track.filter(|&t| t < self.project.tracks.len()) {
                            self.project.tracks[t].devices.push(dev);
                            self.sel_track = TrackSel::Track(t);
                        }
                    }
                }
                self.detail = DetailTab::Devices;
            }
            BrowserItem::SamplerPatch(path) => {
                let lz = match krabton_engine::sample_map::load_zones(std::slice::from_ref(path)) {
                    Ok(lz) => lz,
                    Err(e) => {
                        self.toast(e);
                        return;
                    }
                };
                let mut params = krabton_engine::params::defaults(krabton_engine::dsp::sampler::SPECS);
                use krabton_engine::dsp::sampler as smp;
                for (k, v) in [smp::ATTACK, smp::DECAY, smp::SUSTAIN, smp::RELEASE].into_iter().zip(lz.envelope) {
                    if let Some(v) = v {
                        params[k] = smp::SPECS[k].clamp(v);
                    }
                }
                if lz.zones.iter().any(|z| z.loop_range.is_some()) {
                    params[smp::LOOP] = 1.0;
                }
                let n = lz.zones.len();
                let name = item.label();
                let inst = Instrument::Sampler { params, sample: None, zones: lz.zones };
                match target_track {
                    Some(t) if is_midi(self, t) => {
                        let tr = &mut self.project.tracks[t];
                        tr.instrument = Some(inst);
                        tr.name = name.clone();
                        self.sel_track = TrackSel::Track(t);
                        self.detail = DetailTab::Devices;
                    }
                    _ => {
                        let t = self.add_midi_track(Some(inst));
                        self.project.tracks[t].name = name.clone();
                    }
                }
                self.sel_zone = 0;
                let failed = if lz.failed.is_empty() { String::new() } else { format!(", {} file(s) could not be read", lz.failed.len()) };
                self.toast(format!("{name}: {n} zone(s){failed}"));
            }
            BrowserItem::Clip { id, .. } => {
                let found = self.project.tracks.iter().flat_map(|t| t.slots.iter().flatten()).find(|c| c.id == *id).cloned();
                let Some(mut clip) = found else { return };
                let midi = matches!(clip.content, ClipContent::Midi { .. });
                clip.id = self.project.alloc_id();
                let fits = |app: &App, t: usize| is_midi(app, t) == midi;
                match target {
                    DropTarget::ArrLane { track, beat } if fits(self, track) => {
                        let aid = self.project.alloc_id();
                        let length = clip.length;
                        self.project.tracks[track].arrangement.push(ArrangementClip { id: aid, start: beat, length, offset: 0.0, clip });
                        self.select_clip(ClipSel::Arr { track, id: aid });
                    }
                    _ => {
                        let (track, scene) = match (target, target_track) {
                            (DropTarget::Slot { track, scene }, _) => (track, scene),
                            (_, Some(t)) => (t, self.sel_scene),
                            (_, None) => (if midi { self.add_midi_track(None) } else { self.add_audio_track() }, self.sel_scene),
                        };
                        if !fits(self, track) {
                            self.toast(if midi { "MIDI clips go on MIDI tracks" } else { "Audio clips go on audio tracks" });
                            return;
                        }
                        if let Some(slot) = self.project.tracks[track].slots.get_mut(scene) {
                            *slot = Some(clip);
                            self.select_clip(ClipSel::Slot { track, scene });
                        }
                    }
                }
            }
            BrowserItem::Sample(path) => {
                let Some(sample) = self.load_sample(path) else { return };
                let bpm = self.project.bpm;
                match target {
                    DropTarget::Slot { track, scene } if !is_midi(self, track) => {
                        let id = self.project.alloc_id();
                        let color = self.project.tracks[track].color;
                        self.project.tracks[track].slots[scene] = Some(Clip::new_audio(id, sample, color, bpm));
                        self.select_clip(ClipSel::Slot { track, scene });
                    }
                    DropTarget::ArrLane { track, beat } if !is_midi(self, track) => {
                        let id = self.project.alloc_id();
                        let cid = self.project.alloc_id();
                        let color = self.project.tracks[track].color;
                        let clip = Clip::new_audio(cid, sample, color, bpm);
                        let length = clip.length;
                        self.project.tracks[track].arrangement.push(ArrangementClip { id, start: beat, length, offset: 0.0, clip });
                        self.select_clip(ClipSel::Arr { track, id });
                    }
                    _ => match target_track {
                        Some(t) if is_midi(self, t) => {
                            let tr = &mut self.project.tracks[t];
                            match &mut tr.instrument {
                                Some(Instrument::Sampler { sample: s, zones, .. }) => {
                                    *s = Some(sample);
                                    zones.clear();
                                }
                                Some(Instrument::Drums { pads, .. }) => {
                                    if let Some(pad) = pads.get_mut(self.sel_pad) {
                                        pad.name = sample.name.clone();
                                        pad.sample = Some(sample);
                                    }
                                }
                                _ => {
                                    tr.instrument = Some(Instrument::sampler(Some(sample.clone())));
                                    tr.name = sample.name.clone();
                                }
                            }
                            self.sel_track = TrackSel::Track(t);
                            self.detail = DetailTab::Devices;
                        }
                        Some(t) => {
                            // Audio track without a specific slot: use the selected scene.
                            let scene = self.sel_scene;
                            let id = self.project.alloc_id();
                            let color = self.project.tracks[t].color;
                            self.project.tracks[t].slots[scene] = Some(Clip::new_audio(id, sample, color, bpm));
                            self.select_clip(ClipSel::Slot { track: t, scene });
                        }
                        None => {
                            let t = self.add_audio_track();
                            let id = self.project.alloc_id();
                            let color = self.project.tracks[t].color;
                            self.project.tracks[t].name = sample.name.clone();
                            if self.view == View::Arrangement {
                                let start = self.engine.status.song_pos.load();
                                let cid = self.project.alloc_id();
                                let clip = Clip::new_audio(cid, sample, color, bpm);
                                let length = clip.length;
                                self.project.tracks[t].arrangement.push(ArrangementClip { id, start, length, offset: 0.0, clip });
                                self.select_clip(ClipSel::Arr { track: t, id });
                            } else {
                                let scene = self.sel_scene;
                                self.project.tracks[t].slots[scene] = Some(Clip::new_audio(id, sample, color, bpm));
                                self.select_clip(ClipSel::Slot { track: t, scene });
                            }
                        }
                    },
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Transport & notes
    // ------------------------------------------------------------------

    pub fn toggle_play(&mut self) {
        if self.is_playing() {
            self.cmd(Command::Stop);
        } else {
            self.cmd(Command::Play);
        }
    }

    /// The MIDI tracks that hear a live note (see `Project::live_note_targets`).
    fn note_targets_from(&self, channel: Option<u8>) -> Vec<usize> {
        let selected = match self.sel_track {
            TrackSel::Track(i) => Some(i),
            _ => None,
        };
        self.project.live_note_targets(selected, channel)
    }

    fn note_targets(&self) -> Vec<usize> {
        self.note_targets_from(None)
    }

    /// Auditions a note on one specific track (piano roll keys, drum pads).
    pub fn preview_note_on(&mut self, track: usize, pitch: u8, vel: f32) -> Vec<u64> {
        let Some(id) = self.project.tracks.get(track).map(|t| t.id) else { return Vec::new() };
        self.cmd(Command::NoteOn { track: id, pitch, velocity: vel });
        vec![id]
    }

    /// Plays a note on the armed/selected tracks. Returns the pitch that was
    /// actually played (it may be snapped to the scale) and the track ids.
    pub fn note_on(&mut self, pitch: u8, vel: f32) -> (u8, Vec<u64>) {
        self.note_on_from(None, pitch, vel)
    }

    /// A live note from the computer keyboard (`channel` None) or a MIDI
    /// device.
    pub fn note_on_from(&mut self, channel: Option<u8>, pitch: u8, vel: f32) -> (u8, Vec<u64>) {
        let pitch = if self.snap_input && self.project.scale_mode { self.project.scale.snap(pitch) } else { pitch };
        let targets = self.note_targets_from(channel);
        let status = self.status();
        let playing = self.is_playing();
        let mut ids = Vec::new();
        for ti in targets {
            let id = self.project.tracks[ti].id;
            ids.push(id);
            self.cmd(Command::NoteOn { track: id, pitch, velocity: vel });
            if self.recording && playing {
                let ts = &status.tracks[ti];
                let slot = ts.playing_slot.load(Ordering::Relaxed);
                let pos = ts.clip_pos.load();
                if slot >= 0 && pos >= 0.0 {
                    self.rec_notes.insert((id, pitch), RecNote { scene: slot as usize, start: pos, vel });
                }
            }
        }
        (pitch, ids)
    }

    /// Creates/drops plugin instances to match the project and runs their
    /// main-thread housekeeping.
    fn sync_plugins(&mut self) {
        let sr = self.engine.status.sample_rate.load() as f64;
        let (cmds, msgs) = self.plugins.sync(&self.project, sr);
        for c in cmds {
            self.cmd(c);
        }
        if let Some(m) = msgs.into_iter().next() {
            self.toast(m);
        }
        if self.plugins.idle() {
            self.dirty = true;
        }
    }

    /// Opens or closes the editor of the plugin on track `i`.
    pub fn toggle_plugin_editor(&mut self, i: usize) {
        let Some(t) = self.project.tracks.get(i) else { return };
        let Some(Instrument::Plugin(p)) = &t.instrument else { return };
        let title = format!("{} - {} - Krabton", t.name, p.name);
        if let Err(e) = self.plugins.toggle_editor(p.instance, &title) {
            self.toast(e);
        }
    }

    /// Keeps a panel size the user dragged (saved once the mouse is up).
    fn remember_size(&mut self, field: impl Fn(&mut crate::prefs::Layout) -> &mut f32, v: f32) {
        let slot = field(&mut self.prefs.layout);
        if (*slot - v).abs() > 0.5 {
            *slot = v;
            self.layout_dirty = true;
        }
    }

    /// Settings from the last session: grid rotation, plugin folders, FM banks.
    fn apply_prefs(&mut self) {
        let rotation = self.prefs.grid_rotation;
        self.set_grid_rotation(rotation);
        self.plugins.extra_folders = self.prefs.plugin_folders.clone();
        for path in self.prefs.fm_banks.clone() {
            if path.is_file() {
                let _ = self.load_fm_bank(&path);
            }
        }
    }

    /// Turns the monome grid (0, 90, 180 or 270 degrees) and remembers it.
    pub fn set_grid_rotation(&mut self, degrees: u32) {
        if let Some(m) = self.grid.monome.as_mut() {
            m.set_rotation(degrees);
            if m.grid_id().is_some() {
                let (w, h) = m.grid_dims();
                self.grid.width = w.clamp(8, 16);
                self.grid.height = h.clamp(8, 16);
            }
        }
        if self.prefs.grid_rotation != degrees {
            self.prefs.grid_rotation = degrees;
            self.prefs.save();
        }
    }

    /// Loads a DX7 .syx bank (or single voice) for FM. Returns its index.
    pub fn load_fm_bank(&mut self, path: &Path) -> Result<usize, String> {
        if let Some(i) = self.fm_banks.iter().position(|b| b.path.as_deref() == Some(path)) {
            return Ok(i);
        }
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let voices = krabton_engine::dsp::dx7::parse_sysex(&data).map_err(|e| format!("{}: {e}", path.display()))?;
        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Bank".into());
        self.fm_banks.push(FmBank { name, path: Some(path.to_path_buf()), voices });
        // Remembered for next time (saved by the caller).
        if !self.prefs.fm_banks.iter().any(|p| p == path) {
            self.prefs.fm_banks.push(path.to_path_buf());
        }
        Ok(self.fm_banks.len() - 1)
    }

    /// Loads every .syx file in a folder (and its subfolders).
    pub fn load_fm_folder(&mut self, dir: &Path) -> usize {
        fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
            let Ok(rd) = std::fs::read_dir(dir) else { return };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() && depth < 4 {
                    walk(&p, depth + 1, out);
                } else if p.extension().map(|x| x.eq_ignore_ascii_case("syx")).unwrap_or(false) {
                    out.push(p);
                }
            }
        }
        let mut files = Vec::new();
        walk(dir, 0, &mut files);
        files.sort();
        let n = files.into_iter().take(1000).filter(|f| self.load_fm_bank(f).is_ok()).count();
        self.prefs.save();
        n
    }

    /// Applies the structural edits queued during this frame.
    fn apply_deferred(&mut self) {
        for d in std::mem::take(&mut self.deferred) {
            match d {
                Deferred::DeleteTrack(i) => {
                    self.delete_track(i);
                    // Tracks that listened to it lose that input.
                    self.project.fix_routing();
                }
                Deferred::DuplicateTrack(i) => self.duplicate_track(i),
                Deferred::RecordIntoAudio(i) => {
                    self.route_into_new_audio_track(i);
                }
                Deferred::MoveTrack { from, to } => {
                    let n = self.project.tracks.len();
                    if from < n && to < n {
                        self.project.tracks.swap(from, to);
                        self.sel_track = TrackSel::Track(to);
                    }
                }
                Deferred::InsertScene(at) => self.project.add_scene(at),
                Deferred::DuplicateScene(s) => self.project.duplicate_scene(s),
                Deferred::DeleteScene(s) => {
                    self.project.remove_scene(s);
                    self.sel_clip = None;
                }
                Deferred::DeleteReturn(j) => {
                    if j < self.project.returns.len() {
                        self.project.returns.remove(j);
                        for t in &mut self.project.tracks {
                            if j < t.mixer.sends.len() {
                                t.mixer.sends.remove(j);
                            }
                        }
                        self.sel_track = TrackSel::Master;
                    }
                }
            }
        }
        self.validate_selection();
    }

    pub fn duplicate_track(&mut self, i: usize) {
        // Copy the live plugin state into the project so the copy sounds the same.
        self.plugins.store_states(&mut self.project);
        self.project.duplicate_track(i);
    }

    /// Shows the device chain, unless the Grid tab is open (selecting a
    /// track there picks what the grid edits).
    pub fn show_devices(&mut self) {
        if self.detail != DetailTab::Grid {
            self.detail = DetailTab::Devices;
        }
    }

    pub fn open_scale_dialog(&mut self) {
        let s = self.project.scale;
        self.scale_dialog = Some(ScaleDialog { root: s.root, kind: s.kind });
    }

    pub fn note_off(&mut self, pitch: u8, ids: &[u64]) {
        let status = self.status();
        for &id in ids {
            self.cmd(Command::NoteOff { track: id, pitch });
            let Some(rn) = self.rec_notes.remove(&(id, pitch)) else { continue };
            let Some(ti) = self.project.track_index(id) else { continue };
            let ts = &status.tracks[ti];
            let now = if ts.playing_slot.load(Ordering::Relaxed) == rn.scene as i32 { ts.clip_pos.load() } else { rn.start + 0.25 };
            let quant = self.rec_quantize;
            let Some(Some(clip)) = self.project.tracks[ti].slots.get_mut(rn.scene) else { continue };
            let len = clip.length.max(0.0625);
            let mut length = now - rn.start;
            if length <= 0.0 {
                length += len;
            }
            let length = length.clamp(1.0 / 32.0, len);
            let mut start = rn.start;
            if quant {
                start = ((start * 4.0).round() / 4.0) % len;
            }
            if let Some(notes) = clip.notes_mut() {
                notes.push(Note::new(pitch, start, length, rn.vel));
            }
        }
    }

    pub fn toggle_record(&mut self) {
        if self.recording || !self.takes.is_empty() {
            self.recording = false;
            self.stop_takes();
            return;
        }
        self.recording = true;
        let audio = if self.view == View::Arrangement { self.start_arrangement_takes() } else { self.start_session_takes() };
        if audio > 0 {
            self.recording = true;
            let place = if self.view == View::Arrangement { "on the arrangement" } else { "into session slots" };
            self.toast(format!("Recording {audio} audio track{} {place}", if audio == 1 { "" } else { "s" }));
        }
        let status = self.status();
        let bars = self.project.beats_per_bar_f() * 4.0;
        for ti in self.note_targets() {
            let ts = &status.tracks[ti];
            if ts.playing_slot.load(Ordering::Relaxed) >= 0 || ts.queued_slot.load(Ordering::Relaxed) >= 0 {
                continue;
            }
            let ns = self.project.scenes.len();
            let start = self.sel_scene.min(ns - 1);
            let slot = match (start..ns).find(|&s| self.project.tracks[ti].slots[s].is_none()) {
                Some(s) => s,
                None => {
                    self.project.add_scene(ns);
                    ns
                }
            };
            let id = self.project.alloc_id();
            let t = &mut self.project.tracks[ti];
            t.slots[slot] = Some(Clip::new_midi(id, "Recording", t.color, bars));
            let tid = t.id;
            self.cmd(Command::LaunchClip { track: tid, slot });
            self.select_clip(ClipSel::Slot { track: ti, scene: slot });
        }
        if self.takes.is_empty() {
            self.toast("Session record: play notes to overdub into the playing clip");
        }
    }

    fn poll_midi(&mut self) {
        while let Ok(msg) = self.midi.rx.try_recv() {
            match msg {
                MidiMsg::NoteOn { pitch, velocity, channel } => {
                    let played = self.note_on_from(Some(channel), pitch, velocity);
                    self.midi_held.insert(pitch, played);
                }
                MidiMsg::NoteOff { pitch } => {
                    if let Some((played, ids)) = self.midi_held.remove(&pitch) {
                        self.note_off(played, &ids);
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Keyboard
    // ------------------------------------------------------------------

    fn handle_keys(&mut self, ctx: &Context) {
        let typing = ctx.wants_keyboard_input() || self.renaming.is_some();
        let events = ctx.input(|i| i.events.clone());
        for ev in &events {
            match ev {
                egui::Event::Copy if !typing => self.edit_copy(),
                egui::Event::Paste(_) if !typing => self.edit_paste(),
                egui::Event::Cut if !typing => self.edit_cut(),
                egui::Event::Key { key, pressed, repeat, modifiers, .. } => {
                    if *pressed && *repeat && !typing && matches!(key, Key::ArrowUp | Key::ArrowDown) && modifiers.is_none() {
                        // Holding an arrow walks through the browser.
                        self.browser_key(*key);
                    } else if *pressed && !*repeat && !typing {
                        self.key_down(*key, *modifiers);
                    } else if !*pressed {
                        if let Some((pitch, ids)) = self.held_keys.remove(key) {
                            self.note_off(pitch, &ids);
                        }
                    }
                }
                _ => {}
            }
        }
        if !typing {
            ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab));
            // A focused button would also "click" on Space/Enter; transport keys win.
            if ctx.input(|i| i.key_pressed(Key::Space) || i.key_pressed(Key::Enter)) {
                ctx.memory_mut(|m| {
                    if let Some(id) = m.focused() {
                        m.surrender_focus(id);
                    }
                });
            }
        }
    }

    fn key_down(&mut self, key: Key, m: Modifiers) {
        if m.is_none() && self.browser_key(key) {
            return;
        }
        if m.command || m.ctrl {
            match key {
                Key::B if m.alt => self.browser.visible = !self.browser.visible,
                Key::F => {
                    self.browser.visible = true;
                    self.browser.focus_search = true;
                }
                Key::Z if m.shift => self.redo(),
                Key::Z => self.undo(),
                Key::Y => self.redo(),
                Key::S => self.save(m.shift),
                Key::O => self.open(),
                Key::N => self.new_project(),
                Key::D => self.edit_duplicate(),
                Key::T if m.shift => {
                    self.add_midi_track(Some(Instrument::synth("Init")));
                }
                Key::T => {
                    self.add_audio_track();
                }
                Key::I if m.alt => self.session.hide_io = !self.session.hide_io,
                Key::I => {
                    let at = self.sel_scene + 1;
                    self.project.add_scene(at);
                }
                Key::A => self.piano_select_all(),
                Key::U => self.piano_quantize(),
                Key::R => self.export.open = true,
                _ => {}
            }
            return;
        }
        match key {
            Key::Escape if self.mod_learn.is_some() => self.mod_learn = None,
            Key::Space => self.toggle_play(),
            Key::Tab if m.shift => {
                self.detail = if self.detail == DetailTab::Clip { DetailTab::Devices } else { DetailTab::Clip };
            }
            Key::Tab => self.view = if self.view == View::Session { View::Arrangement } else { View::Session },
            Key::Delete | Key::Backspace => {
                if self.detail == DetailTab::Clip && !self.piano.selected.is_empty() {
                    self.piano_delete_selected();
                } else if let Some(sel) = self.sel_clip {
                    self.delete_clip(sel);
                }
            }
            Key::F9 => self.toggle_record(),
            Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight => {
                self.edit_arrow(key, m);
            }
            Key::Enter => {
                let s = self.sel_scene;
                self.cmd(Command::LaunchScene(s));
            }
            Key::B => self.piano.draw_mode = !self.piano.draw_mode,
            Key::M => {
                self.kb_enabled = !self.kb_enabled;
                let state = if self.kb_enabled { "on" } else { "off" };
                self.toast(format!("Computer MIDI keyboard {state}"));
            }
            Key::Z if self.kb_enabled => {
                self.kb_octave = (self.kb_octave - 1).max(-2);
                self.toast(format!("Keyboard octave C{}", self.kb_octave));
            }
            Key::X if self.kb_enabled => {
                self.kb_octave = (self.kb_octave + 1).min(8);
                self.toast(format!("Keyboard octave C{}", self.kb_octave));
            }
            Key::C if self.kb_enabled => {
                self.kb_velocity = ((self.kb_velocity * 127.0 - 20.0).max(1.0)) / 127.0;
                self.toast(format!("Velocity {}", (self.kb_velocity * 127.0).round()));
            }
            Key::V if self.kb_enabled => {
                self.kb_velocity = ((self.kb_velocity * 127.0 + 20.0).min(127.0)) / 127.0;
                self.toast(format!("Velocity {}", (self.kb_velocity * 127.0).round()));
            }
            _ => {
                if !self.kb_enabled || self.held_keys.contains_key(&key) {
                    return;
                }
                let pitch = if self.scale_keys && self.project.scale_mode {
                    SCALE_KEYMAP.iter().find(|(k, _)| *k == key).map(|&(_, step)| {
                        let scale = self.project.scale;
                        let (base, _) = scale.locate((self.kb_octave + 2) * 12 + scale.root as i32);
                        scale.pitch_at(base + step)
                    })
                } else {
                    KEYMAP.iter().find(|(k, _)| *k == key).map(|&(_, offset)| (self.kb_octave + 2) * 12 + offset)
                };
                if let Some(pitch) = pitch.filter(|p| (0..=127).contains(p)) {
                    let vel = self.kb_velocity;
                    let played = self.note_on(pitch as u8, vel);
                    self.held_keys.insert(key, played);
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Files
    // ------------------------------------------------------------------

    fn replace_project(&mut self, p: Project) {
        self.cmd(Command::Stop);
        self.cmd(Command::Stop);
        self.cmd(Command::BackToArrangement);
        self.cmd(Command::AllNotesOff);
        self.project = p;
        self.committed = self.project.clone();
        self.undo.clear();
        self.redo.clear();
        self.dirty = false;
        self.sel_clip = None;
        self.sel_track = if self.project.tracks.is_empty() { TrackSel::Master } else { TrackSel::Track(0) };
        self.sel_scene = 0;
        self.recording = false;
    }

    pub fn new_project(&mut self) {
        self.replace_project(Project::new_default());
        self.file_path = None;
        self.toast("New set");
    }

    pub fn load_demo(&mut self) {
        self.replace_project(krabton_engine::demo::demo_project());
        self.file_path = None;
        self.toast("Demo set loaded");
    }

    pub fn open(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("Krabton set", &[FILE_EXTENSION]).pick_file() else { return };
        match Project::load(&path) {
            Ok((p, missing)) => {
                self.replace_project(p);
                self.file_path = Some(path.clone());
                if missing.is_empty() {
                    self.toast(format!("Opened {}", path.display()));
                } else {
                    self.toast(format!("Opened with {} missing sample(s): {}", missing.len(), missing[0]));
                }
            }
            Err(e) => self.toast(format!("Could not open: {e}")),
        }
    }

    pub fn save(&mut self, save_as: bool) {
        let path = match (&self.file_path, save_as) {
            (Some(p), false) => p.clone(),
            _ => {
                let name = format!("{}.{}", self.project.name, FILE_EXTENSION);
                match rfd::FileDialog::new().add_filter("Krabton set", &[FILE_EXTENSION]).set_file_name(name).save_file() {
                    Some(p) => p,
                    None => return,
                }
            }
        };
        if let Some(stem) = path.file_stem() {
            self.project.name = stem.to_string_lossy().to_string();
        }
        self.plugins.store_states(&mut self.project);
        match self.project.save(&path) {
            Ok(()) => {
                self.dirty = false;
                self.committed = self.project.clone();
                self.toast(format!("Saved {}", path.display()));
                self.file_path = Some(path);
            }
            Err(e) => self.toast(format!("Save failed: {e}")),
        }
    }

    pub fn start_export(&mut self) {
        if self.export_job.is_some() {
            return;
        }
        let name = format!("{}.wav", self.project.name);
        let Some(path) = rfd::FileDialog::new().add_filter("WAV audio", &["wav"]).set_file_name(name).save_file() else { return };
        let project = self.project.clone();
        let bar = project.beats_per_bar_f();
        let source = match self.export.source {
            ExportSource::Arrangement => {
                let end = project.arrangement_end();
                if end <= 0.0 {
                    self.toast("The arrangement is empty. Use 'Scenes to Arrangement' or export a scene.");
                    return;
                }
                RenderSource::Arrangement { start: 0.0, end }
            }
            ExportSource::Scene => RenderSource::Scene { scene: self.export.scene, beats: self.export.bars as f64 * bar },
        };
        let sr = self.export.sample_rate;
        let (plugin_instances, processors, errors) = self.plugins.export_instances(&project, sr as f64);
        if let Some(e) = errors.first() {
            self.toast(format!("Exporting without a plugin: {e}"));
        }
        let progress = Arc::new(RenderProgress::default());
        let pr = progress.clone();
        let handle = std::thread::spawn(move || {
            let (audio, leftovers) = render::render_with_plugins(&project, source, sr, bar, Some(&pr), processors);
            let result = match audio {
                Some(buf) => render::write_wav(&path, sr, &buf).map(|_| path.display().to_string()),
                None => Err("Export cancelled".into()),
            };
            (result, leftovers)
        });
        self.export_job = Some(ExportJob { progress, handle: Some(handle), plugin_instances });
        self.export.open = false;
    }

    fn poll_export(&mut self) {
        let finished = self.export_job.as_ref().and_then(|j| j.handle.as_ref()).map(|h| h.is_finished()).unwrap_or(false);
        if finished {
            if let Some(mut job) = self.export_job.take() {
                // Plugin processors come back here so they are dropped on the UI thread.
                match job.handle.take().map(|h| h.join()) {
                    Some(Ok((Ok(path), _leftovers))) => self.toast(format!("Exported {path}")),
                    Some(Ok((Err(e), _leftovers))) => self.toast(e),
                    _ => self.toast("Export failed"),
                }
                job.plugin_instances.clear();
            }
        }
    }

    fn update_meters(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.2);
        self.last_frame = now;
        let fall = 10f32.powf(-dt * 1.2);
        let st = self.status();
        let upd = |m: &mut (f32, f32), ts: &krabton_engine::engine::TrackStatus| {
            let (l, r) = ts.take_peaks();
            m.0 = l.max(m.0 * fall);
            m.1 = r.max(m.1 * fall);
        };
        self.meters.resize(self.project.tracks.len(), (0.0, 0.0));
        for (i, m) in self.meters.iter_mut().enumerate() {
            if let Some(ts) = st.tracks.get(i) {
                upd(m, ts);
            }
        }
        self.return_meters.resize(self.project.returns.len(), (0.0, 0.0));
        for (i, m) in self.return_meters.iter_mut().enumerate() {
            if let Some(ts) = st.returns.get(i) {
                upd(m, ts);
            }
        }
        upd(&mut self.master_meter, &st.master);
    }

    /// Executes scripted debug actions that are due (see `KRABTON_SCRIPT`).
    fn run_script(&mut self) {
        while let Some((at, _)) = self.script.first() {
            if *at > self.frame {
                break;
            }
            let (_, action) = self.script.remove(0);
            let (name, arg) = action.split_once('=').unwrap_or((action.as_str(), ""));
            let nums: Vec<usize> = arg.split(',').filter_map(|v| v.trim().parse().ok()).collect();
            match name {
                "scene" => self.cmd(Command::LaunchScene(nums.first().copied().unwrap_or(0))),
                "play" => self.cmd(Command::Play),
                "stop" => self.cmd(Command::Stop),
                "arrangement" => self.view = View::Arrangement,
                "session" => self.view = View::Session,
                "devices" => self.detail = DetailTab::Devices,
                "grid" => self.detail = DetailTab::Grid,
                "press" => {
                    // Keyboard input for this window only, e.g. press=ctrl+c, press=shift+up.
                    let mut mods = Modifiers::NONE;
                    let mut key_name = "";
                    for part in arg.split('+') {
                        match part.trim().to_lowercase().as_str() {
                            "ctrl" | "cmd" => {
                                mods.ctrl = true;
                                mods.command = true;
                            }
                            "shift" => mods.shift = true,
                            "alt" => mods.alt = true,
                            _ => key_name = part.trim(),
                        }
                    }
                    let lower = key_name.to_lowercase();
                    let named = match lower.as_str() {
                        "up" => Some(Key::ArrowUp),
                        "down" => Some(Key::ArrowDown),
                        "left" => Some(Key::ArrowLeft),
                        "right" => Some(Key::ArrowRight),
                        "tab" => Some(Key::Tab),
                        "del" | "delete" => Some(Key::Delete),
                        _ => Key::from_name(&key_name.to_uppercase()).or_else(|| Key::from_name(key_name)),
                    };
                    let events = match (mods.ctrl, lower.as_str()) {
                        (true, "c") => vec![vec![egui::Event::Copy]],
                        (true, "x") => vec![vec![egui::Event::Cut]],
                        (true, "v") => vec![vec![egui::Event::Paste(String::new())]],
                        _ => match named {
                            Some(key) => vec![
                                vec![egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: mods }],
                                vec![egui::Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: mods }],
                            ],
                            None => Vec::new(),
                        },
                    };
                    self.script_input.extend(events);
                }
                "analog" => {
                    let names = krabton_engine::dsp::analog::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Analog(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "elements" => {
                    let names = krabton_engine::dsp::elements_synth::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Elements(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "braids" => {
                    let names = krabton_engine::dsp::braids_synth::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Braids(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "macro" => {
                    let names = krabton_engine::dsp::macro_osc::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Macro(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "param" if nums.len() == 2 => {
                    // Sets an instrument parameter of the selected track (value in percent of its range).
                    if let TrackSel::Track(t) = self.sel_track {
                        if let Some(Instrument::Macro { params, .. }) = self.project.tracks.get_mut(t).and_then(|t| t.instrument.as_mut()) {
                            if let (Some(spec), Some(v)) = (krabton_engine::dsp::macro_osc::SPECS.get(nums[0]), params.get_mut(nums[0])) {
                                *v = spec.from_norm(nums[1] as f32 / 100.0);
                            }
                        }
                    }
                }
                "rotate" => self.set_grid_rotation(nums.first().copied().unwrap_or(0) as u32),
                // Text typed into the focused field of this window.
                "type" => self.script_input.push_back(vec![egui::Event::Text(arg.to_string())]),
                "rclick" | "dclick" => {
                    let f: Vec<f32> = arg.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                    if f.len() >= 2 {
                        let pos = egui::pos2(f[0], f[1]);
                        let button = if name == "rclick" { egui::PointerButton::Secondary } else { egui::PointerButton::Primary };
                        let ev = |pressed| egui::Event::PointerButton { pos, button, pressed, modifiers: Modifiers::NONE };
                        self.script_input.push_back(vec![egui::Event::PointerMoved(pos)]);
                        for _ in 0..if name == "dclick" { 2 } else { 1 } {
                            self.script_input.push_back(vec![ev(true)]);
                            self.script_input.push_back(vec![ev(false)]);
                        }
                    }
                }
                "click" | "drag" => {
                    let f: Vec<f32> = arg.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                    if f.len() >= 2 {
                        // drag=x1,y1,x2,y2[,x3,y3...] follows the points in order.
                        let points: Vec<egui::Pos2> = if name == "drag" { f.chunks_exact(2).map(|c| egui::pos2(c[0], c[1])).collect() } else { vec![egui::pos2(f[0], f[1])] };
                        let from = points[0];
                        let to = *points.last().unwrap_or(&from);
                        let button = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
                        self.script_input.push_back(vec![egui::Event::PointerMoved(from)]);
                        self.script_input.push_back(vec![button(from, true)]);
                        for w in points.windows(2) {
                            for k in 1..=8 {
                                self.script_input.push_back(vec![egui::Event::PointerMoved(w[0] + (w[1] - w[0]) * (k as f32 / 8.0))]);
                            }
                        }
                        self.script_input.push_back(vec![button(to, false)]);
                    }
                }
                "modtab" => self.detail = DetailTab::Mod,
                "mod" => {
                    let k = krabton_engine::modulation::ModKind::ALL[nums.first().copied().unwrap_or(0).min(7)];
                    self.add_modulator(k);
                }
                "map" => self.mod_learn = self.project.modulators.last().map(|m| m.id),
                "selmod" => self.sel_mod = nums.first().and_then(|&i| self.project.modulators.get(i)).map(|m| m.id),
                "routeinst" | "routedev" | "routemidi" | "routevol" | "routemod" => {
                    use krabton_engine::modulation::{ModOwner, ModTarget};
                    let src = self.project.modulators.last().map(|m| m.id);
                    let t = match self.sel_track {
                        TrackSel::Track(t) => self.project.tracks.get(t),
                        _ => None,
                    };
                    let n = |k: usize| nums.get(k).copied().unwrap_or(0);
                    let target = match (name, t) {
                        ("routeinst", Some(t)) => Some(ModTarget::Param { owner: ModOwner::Instrument(t.id), param: n(0) as u16 }),
                        ("routedev", Some(t)) => t.devices.get(n(0)).map(|d| ModTarget::Param { owner: ModOwner::Device(d.id), param: n(1) as u16 }),
                        ("routemidi", Some(t)) => t.midi_devices.get(n(0)).map(|d| ModTarget::Param { owner: ModOwner::MidiDevice(d.id), param: n(1) as u16 }),
                        ("routevol", _) => self.project.tracks.get(n(0)).map(|t| ModTarget::Volume(t.id)),
                        ("routemod", _) => self.project.modulators.get(n(0)).map(|m| ModTarget::Param { owner: ModOwner::Modulator(m.id), param: n(1) as u16 }),
                        _ => None,
                    };
                    let amount = nums.get(if name == "routevol" { 1 } else { 2 }).map(|&a| a as f32 / 100.0).unwrap_or(0.5);
                    let amount = if name == "routevol" && nums.len() < 2 { -1.0 } else { amount };
                    if let (Some(src), Some(target)) = (src, target) {
                        self.project.add_route(src, target, amount);
                    }
                }
                "modtrack" => {
                    let t = self.project.tracks.get(nums.first().copied().unwrap_or(0)).map(|t| t.id);
                    if let Some(m) = self.project.modulators.last_mut() {
                        m.track = t;
                    }
                }
                "gridmode" => self.grid.mode = crate::grid::GridMode::ALL[nums.first().copied().unwrap_or(0).min(2)],
                "gridkey" if nums.len() == 2 => {
                    self.grid_key(nums[0] as u32, nums[1] as u32, true);
                    self.grid_key(nums[0] as u32, nums[1] as u32, false);
                }
                "griddown" if nums.len() == 2 => self.grid_key(nums[0] as u32, nums[1] as u32, true),
                "gridup" if nums.len() == 2 => self.grid_key(nums[0] as u32, nums[1] as u32, false),
                "seq" | "seqnew" => {
                    use krabton_engine::midi_fx::MidiFxKind;
                    let k = MidiFxKind::SEQUENCERS[nums.first().copied().unwrap_or(0).min(MidiFxKind::SEQUENCERS.len() - 1)];
                    let target = match (name, self.sel_track) {
                        ("seq", TrackSel::Track(t)) => DropTarget::Track(t),
                        _ => DropTarget::NewTrack,
                    };
                    self.apply_item(&BrowserItem::MidiFx(k), target);
                }
                "clipview" => self.detail = DetailTab::Clip,
                "track" => self.sel_track = TrackSel::Track(nums.first().copied().unwrap_or(0)),
                "routetrack" => {
                    self.route_into_new_audio_track(nums.first().copied().unwrap_or(0));
                }
                "recslot" if nums.len() == 2 => {
                    let (t, s) = (nums[0], nums[1]);
                    let id = self.project.tracks.get(t).map(|t| t.id).unwrap_or(0);
                    if self.takes.iter().any(|k| k.track == id && k.end.is_none()) {
                        self.stop_slot_take(t);
                    } else {
                        self.start_slot_take(t, s);
                    }
                }
                "quit" => self.quit_requested = true,
                "fxparam" => {
                    // fxparam=device,param,value sets a parameter of a device in the selected chain.
                    let f: Vec<f32> = arg.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                    if f.len() == 3 {
                        if let Some(v) = self.chain_mut(self.sel_track).and_then(|c| c.get_mut(f[0] as usize)).and_then(|d| d.params.get_mut(f[1] as usize)) {
                            *v = f[2];
                        }
                    }
                }
                "fx" => {
                    // Adds an effect (by its index in the effect list) to the selected chain.
                    let all = krabton_engine::dsp::effects::EffectKind::ALL;
                    let dev = self.project.make_device(all[nums.first().copied().unwrap_or(0).min(all.len() - 1)]);
                    if let Some(chain) = self.chain_mut(self.sel_track) {
                        chain.push(dev);
                    }
                    self.detail = DetailTab::Devices;
                }
                "clip" if nums.len() == 2 => self.select_clip(ClipSel::Slot { track: nums[0], scene: nums[1] }),
                "s2a" => {
                    self.project.scenes_to_arrangement();
                }
                "export" => self.export.open = true,
                "about" => self.show_about = true,
                "record" => self.toggle_record(),
                "arm" => {
                    if let Some(t) = nums.first().and_then(|&i| self.project.tracks.get_mut(i)) {
                        t.arm = true;
                    }
                }
                "note" => {
                    let pitch = nums.first().copied().unwrap_or(60) as u8;
                    let played = self.note_on(pitch, 0.8);
                    self.held_keys.insert(Key::F20, played);
                }
                "scale" if nums.len() == 2 => {
                    self.project.scale = Scale::new(nums[0] as u8, ScaleKind::ALL[nums[1].min(ScaleKind::ALL.len() - 1)]);
                }
                "convert" if nums.len() == 2 => {
                    let to = Scale::new(nums[0] as u8, ScaleKind::ALL[nums[1].min(ScaleKind::ALL.len() - 1)]);
                    self.project.convert_scale(to);
                }
                "fold" => self.piano.fold = !self.piano.fold,
                "plugin" => {
                    let n = nums.first().copied().unwrap_or(0);
                    let info = self.plugins.instruments().nth(n).cloned();
                    if let Some(info) = info {
                        self.apply_item(&BrowserItem::Plugin(info), DropTarget::NewTrack);
                    } else {
                        eprintln!("no plugin #{n}");
                    }
                }
                "savepath" => match self.project.save(std::path::Path::new(arg)) {
                    // Recordings then go next to this set.
                    Ok(()) => self.file_path = Some(std::path::PathBuf::from(arg)),
                    Err(e) => eprintln!("save failed: {e}"),
                },
                "savestates" => {
                    self.plugins.store_states(&mut self.project);
                }
                "openpath" => match Project::load(std::path::Path::new(arg)) {
                    Ok((p, _)) => self.replace_project(p),
                    Err(e) => eprintln!("open failed: {e}"),
                },
                "midifx" => {
                    let k = nums.first().copied().unwrap_or(0).min(krabton_engine::midi_fx::MidiFxKind::ALL.len() - 1);
                    let t = match self.sel_track {
                        TrackSel::Track(t) => t,
                        _ => 0,
                    };
                    self.apply_item(&BrowserItem::MidiFx(krabton_engine::midi_fx::MidiFxKind::ALL[k]), DropTarget::Track(t));
                }
                "deltrack" => self.deferred.push(Deferred::DeleteTrack(nums.first().copied().unwrap_or(0))),
                "delscene" => self.deferred.push(Deferred::DeleteScene(nums.first().copied().unwrap_or(0))),
                "delreturn" => self.deferred.push(Deferred::DeleteReturn(nums.first().copied().unwrap_or(0))),
                "fmbank" => {
                    if let Err(e) = self.load_fm_bank(std::path::Path::new(arg)) {
                        eprintln!("{e}");
                    }
                }
                "fm" => {
                    let n = nums.first().copied().unwrap_or(0);
                    let voice = self.fm_banks[0].voices.get(n).cloned();
                    if let Some(v) = voice {
                        self.apply_item(&BrowserItem::FmVoice { bank: self.fm_banks[0].name.clone(), voice: v.0.to_vec() }, DropTarget::NewTrack);
                    }
                }
                "editor" => {
                    if let TrackSel::Track(i) = self.sel_track {
                        self.toggle_plugin_editor(i);
                    }
                }
                "scaledialog" => self.open_scale_dialog(),
                "noteoff" => {
                    if let Some((pitch, ids)) = self.held_keys.remove(&Key::F20) {
                        self.note_off(pitch, &ids);
                    }
                }
                _ => eprintln!("unknown script action {action}"),
            }
        }
    }

    fn update_title(&mut self, ctx: &Context) {
        let name = self.file_path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| self.project.name.clone());
        let title = format!("{}{} - Krabton", name, if self.dirty { "*" } else { "" });
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }
}

/// Parses "60:scene=1;200:arrangement" into frame-stamped actions.
fn parse_script(s: &str) -> Vec<(u64, String)> {
    let mut v: Vec<(u64, String)> = s
        .split(';')
        .filter_map(|part| {
            let (frame, action) = part.split_once(':')?;
            Some((frame.trim().parse().ok()?, action.trim().to_string()))
        })
        .collect();
    v.sort_by_key(|x| x.0);
    v
}

impl eframe::App for App {
    fn raw_input_hook(&mut self, _ctx: &Context, raw_input: &mut egui::RawInput) {
        if let Some(events) = self.script_input.pop_front() {
            raw_input.events.extend(events);
        }
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.engine.collect_garbage();
        self.sync_plugins();
        self.frame += 1;
        self.run_script();
        self.poll_midi();
        self.poll_recording();
        self.poll_export();
        self.handle_keys(ctx);
        self.validate_selection();
        self.update_meters();
        self.grid_frame();
        self.mod_frame();

        egui::TopBottomPanel::top("transport")
            .exact_height(38.0)
            .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(8, 4)))
            .show(ctx, |ui| self.ui_transport(ui));
        egui::TopBottomPanel::bottom("statusbar")
            .exact_height(22.0)
            .frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin::symmetric(8, 2)))
            .show(ctx, |ui| self.ui_statusbar(ui));
        // The browser runs from top to bottom, next to the detail view, as in Live.
        let fg = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("panel_grips")));
        if self.browser.visible {
            let r = egui::SidePanel::left("browser")
                .resizable(true)
                .default_width(crate::prefs::Layout::or(self.prefs.layout.browser_w, 330.0))
                .width_range(240.0..=900.0)
                .frame(egui::Frame::new().fill(theme::PANEL))
                .show(ctx, |ui| self.ui_browser(ui))
                .response
                .rect;
            crate::widgets::grip_dots(&fg, egui::pos2(r.right(), r.center().y), true, theme::TEXT_DIM);
            self.remember_size(|l| &mut l.browser_w, r.width());
        } else {
            egui::SidePanel::left("browser_hidden")
                .resizable(false)
                .exact_width(16.0)
                .frame(egui::Frame::new().fill(theme::PANEL))
                .show(ctx, |ui| {
                    let r = ui.max_rect();
                    let resp = ui.interact(r, ui.id().with("show_browser"), egui::Sense::click()).on_hover_text("Show the browser (Ctrl+Alt+B)");
                    let c = egui::pos2(r.center().x, r.top() + 14.0);
                    let col = if resp.hovered() { theme::TEXT } else { theme::TEXT_DIM };
                    ui.painter().add(egui::Shape::convex_polygon(vec![c + egui::vec2(-2.0, -4.0), c + egui::vec2(3.0, 0.0), c + egui::vec2(-2.0, 4.0)], col, egui::Stroke::NONE));
                    if resp.clicked() {
                        self.browser.visible = true;
                    }
                });
        }
        let r = egui::TopBottomPanel::bottom("detail")
            .resizable(true)
            .default_height(crate::prefs::Layout::or(self.prefs.layout.detail_h, 290.0))
            .height_range(120.0..=2000.0)
            .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::same(0)))
            .show(ctx, |ui| self.ui_detail(ui))
            .response
            .rect;
        crate::widgets::grip_dots(&fg, egui::pos2(r.center().x, r.top()), false, theme::TEXT_DIM);
        self.remember_size(|l| &mut l.detail_h, r.height());
        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::BG)).show(ctx, |ui| match self.view {
            View::Session => self.ui_session(ui),
            View::Arrangement => self.ui_arrangement(ui),
        });
        self.ui_dialogs(ctx);
        self.draw_drag_ghost(ctx);
        self.apply_deferred();

        self.sync(ctx);
        self.update_title(ctx);
        if self.layout_dirty && !ctx.input(|i| i.pointer.any_down()) {
            self.layout_dirty = false;
            self.prefs.save();
        }
        // Remember the size of everything (Ctrl + / - / 0, or the Options menu).
        let zoom = ctx.zoom_factor();
        let stored = if self.prefs.ui_zoom > 0.0 { self.prefs.ui_zoom } else { 1.0 };
        if (zoom - stored).abs() > 1e-3 {
            self.prefs.ui_zoom = zoom;
            self.prefs.save();
        }
        if self.quit_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}
