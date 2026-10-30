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
    /// The studio: every track a machine on the shelf, its patterns and
    /// mixer beside its devices (the home screen).
    Studio,
    /// The four-track tape.
    Tape,
    /// Touch: the selected track played with plates, a pulser, steps and cables.
    Touch,
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
    /// A section of the arranger track, by id.
    Section(u64),
}

/// Something dragged out of the browser.
#[derive(Clone, Debug)]
pub enum BrowserItem {
    Synth(&'static str),
    Drums(&'static str),
    Sampler,
    Effect(EffectKind),
    /// An audio effect set to one of its presets.
    EffectPreset(EffectKind, &'static str),
    Sample(PathBuf),
    /// A Standard MIDI File: its notes become clips.
    MidiFile(PathBuf),
    /// A Street Organ preset.
    StreetOrgan(&'static str),
    /// The Singer (on the voicebank used last, or its own voice).
    Singer,
    /// Hyper (on the firmware used last).
    Hyper,
    /// A Virus firmware file, for Hyper.
    HyperFirmware(PathBuf),
    /// A UTAU voicebank's folder, for the Singer.
    Voicebank(PathBuf),
    /// The SF2 Player without a bank.
    Sf2Player,
    /// A preset of a SoundFont bank on disk.
    Sf2 { path: PathBuf, bank: u16, patch: u8, name: String },
    Plugin(krabton_clap::PluginInfo),
    /// A CLAP audio effect.
    PluginEffect(krabton_clap::PluginInfo),
    /// An FM voice (155-byte DX7 voice) from a bank.
    FmVoice { bank: String, voice: Vec<u8> },
    MidiFx(krabton_engine::midi_fx::MidiFxKind),
    /// A MIDI effect or sequencer set to one of its presets.
    MidiPreset(krabton_engine::midi_fx::MidiFxKind, &'static str),
    /// A Plaits preset.
    Macro(&'static str),
    /// A Braids preset.
    Braids(&'static str),
    /// An Elements preset.
    Elements(&'static str),
    /// An Analog preset.
    Analog(&'static str),
    /// A Shakers preset.
    Shakers(&'static str),
    /// A Modular patch.
    Modular(&'static str),
    /// A Voice preset.
    Voice(&'static str),
    /// A Mallets preset.
    Mallets(&'static str),
    /// A Strings preset.
    Strings(&'static str),
    /// A Winds preset.
    Winds(&'static str),
    /// A Organ preset.
    Organ(&'static str),
    /// A Rings preset.
    Rings(&'static str),
    /// A Monk preset.
    Monk(&'static str),
    /// A Bassline preset.
    Bassline(&'static str),
    /// A Mono preset.
    Mono101(&'static str),
    /// A Drone preset.
    Drone(&'static str),
    /// A Waves preset.
    Waves(&'static str),
    /// A Guitar preset.
    Guitar(&'static str),
    /// A Wind Chimes preset.
    WindChimes(&'static str),
    /// A Birds preset.
    Birds(&'static str),
    /// A Nature preset.
    Nature(&'static str),
    /// A Pan Flute preset.
    PanFlute(&'static str),
    Melodica(&'static str),
    Harmonica(&'static str),
    Calculator(&'static str),
    Console(&'static str),
    Handheld(&'static str),
    Micro(&'static str),
    Warp(&'static str),
    Hybrid(&'static str),
    ChipFm(&'static str),
    V2(&'static str),
    Oidos(&'static str),
    Quartet(&'static str),
    Orchestra(&'static str),
    /// A Facet preset.
    Facet(&'static str),
    /// A Coast preset.
    Coast(&'static str),
    /// A Crimson preset.
    Crimson(&'static str),
    /// A Silicon preset.
    Silicon(&'static str),
    /// A Beads (instrument) preset.
    BeadsSynth(&'static str),
    /// A Grand Piano preset.
    Piano(&'static str),
    /// A copy of a clip in this project.
    Clip { id: u64, name: String },
    /// An SFZ instrument, played by the Sampler.
    SamplerPatch(PathBuf),
    /// A Theremin preset.
    Theremin(&'static str),
    /// A Granulator preset.
    Granulator(&'static str),
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
            BrowserItem::Shakers(p) => format!("Particles: {p}"),
            BrowserItem::Modular(p) => format!("Modular: {p}"),
            BrowserItem::Voice(p) => format!("Voice: {p}"),
            BrowserItem::Mallets(p) => format!("Mallets: {p}"),
            BrowserItem::Strings(p) => format!("Strings: {p}"),
            BrowserItem::Winds(p) => format!("Winds: {p}"),
            BrowserItem::Organ(p) => format!("Organ: {p}"),
            BrowserItem::Rings(p) => format!("Rings: {p}"),
            BrowserItem::Monk(p) => format!("Monk: {p}"),
            BrowserItem::Bassline(p) => format!("Bassline: {p}"),
            BrowserItem::Mono101(p) => format!("Mono: {p}"),
            BrowserItem::Drone(p) => format!("Drone: {p}"),
            BrowserItem::Waves(p) => format!("Waves: {p}"),
            BrowserItem::Guitar(p) => format!("Guitar: {p}"),
            BrowserItem::WindChimes(p) => format!("Wind Chimes: {p}"),
            BrowserItem::BeadsSynth(p) => format!("Beads: {p}"),
            BrowserItem::Birds(p) => format!("Birds: {p}"),
            BrowserItem::Nature(p) => format!("Nature: {p}"),
            BrowserItem::PanFlute(p) => format!("Pan Flute: {p}"),
            BrowserItem::Melodica(p) => format!("Melodica: {p}"),
            BrowserItem::StreetOrgan(p) => format!("Street Organ: {p}"),
            BrowserItem::Harmonica(p) => format!("Harmonica: {p}"),
            BrowserItem::Calculator(p) => format!("Calculator: {p}"),
            BrowserItem::Console(p) => format!("Console: {p}"),
            BrowserItem::Handheld(p) => format!("Handheld: {p}"),
            BrowserItem::Micro(p) => format!("Micro: {p}"),
            BrowserItem::Warp(p) => format!("Warp: {p}"),
            BrowserItem::Hybrid(p) => format!("Hybrid: {p}"),
            BrowserItem::ChipFm(p) => format!("Chip FM: {p}"),
            BrowserItem::V2(p) => format!("V2: {p}"),
            BrowserItem::Oidos(p) => format!("Oidos: {p}"),
            BrowserItem::Quartet(p) => format!("Quartet: {p}"),
            BrowserItem::Orchestra(p) => format!("{}: {p}", if krabton_engine::orchestra::SOLOIST_NAMES.contains(p) { "Soloists" } else { "Orchestra" }),
            BrowserItem::Facet(p) => format!("Facet: {p}"),
            BrowserItem::Coast(p) => format!("Coast: {p}"),
            BrowserItem::Crimson(p) => format!("Crimson: {p}"),
            BrowserItem::Silicon(p) => format!("Silicon: {p}"),
            BrowserItem::Piano(p) => format!("Grand Piano: {p}"),
            BrowserItem::Drums(k) => k.to_string(),
            BrowserItem::Sampler => "Sampler".into(),
            BrowserItem::Effect(k) => k.name().into(),
            BrowserItem::EffectPreset(k, p) => format!("{}: {p}", k.name()),
            BrowserItem::Sample(p) => p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(),
            BrowserItem::MidiFile(p) => p.file_stem().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(),
            BrowserItem::Sf2Player => "SF2 Player".into(),
            BrowserItem::Singer => "Singer".into(),
            BrowserItem::Hyper => "Hyper".into(),
            BrowserItem::HyperFirmware(p) => p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(),
            BrowserItem::Voicebank(p) => p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(),
            BrowserItem::Sf2 { name, .. } => name.clone(),
            BrowserItem::Plugin(p) | BrowserItem::PluginEffect(p) => p.name.clone(),
            BrowserItem::MidiFx(k) => k.name().into(),
            BrowserItem::MidiPreset(k, p) => format!("{}: {p}", k.name()),
            BrowserItem::FmVoice { voice, .. } => {
                krabton_engine::dsp::dx7::Dx7Voice::from_slice(voice).map(|v| format!("FM: {}", v.name())).unwrap_or_default()
            }
            BrowserItem::Clip { name, .. } => name.clone(),
            BrowserItem::SamplerPatch(p) => p.file_stem().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(),
            BrowserItem::Theremin(p) => format!("Theremin: {p}"),
            BrowserItem::Granulator(p) => format!("Granulator: {p}"),
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
    /// Recorded into this arrangement clip (`start` in it) instead of a slot.
    arr: Option<u64>,
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
    /// SoundFont banks open (the last few), so picking from one again is instant.
    pub(crate) sf2_open: Vec<krabton_engine::dsp::sf2::Sf2Ref>,
    /// Voicebanks open (the last few), and the lyrics typed in the clip view.
    pub(crate) singer_open: Vec<krabton_engine::singer::VoicebankRef>,
    /// Virus firmware read (the last few), and the files found on this computer (looked for now and then).
    pub(crate) hyper_open: Vec<krabton_engine::hyper::FirmwareRef>,
    hyper_found: std::cell::RefCell<Option<(std::time::Instant, Vec<PathBuf>)>>,
    /// An installer being unpacked for its Virus TI firmware: what comes of it, and its name.
    firmware_unpack: Option<(std::sync::mpsc::Receiver<Result<PathBuf, String>>, String)>,
    pub(crate) lyric_text: String,
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
    /// What each undo/redo step changed (see `history`).
    pub(crate) undo_labels: Vec<String>,
    pub(crate) redo_labels: Vec<String>,
    pub show_history: bool,
    /// The Modular rack window.
    pub rack: crate::ui::rack::RackUi,
    pub dirty: bool,
    pub engine: EngineHandle,
    _audio: AudioOutput,
    /// Where the engine is, for the output to play (and to move to another).
    engine_slot: audio::EngineSlot,
    pub audio_desc: String,
    /// Output devices for the Options menu (read when it first opens).
    pub audio_devices: Option<Vec<String>>,
    pub audio_in_devices: Option<Vec<String>>,
    /// The settings window (audio and MIDI devices).
    pub show_settings: bool,
    pub midi: Midi,
    /// MIDI out to the tracks' MIDI To ports.
    pub midi_outs: crate::midi_out::MidiOutputs,
    pending: Vec<Command>,
    /// Per track (by id): the clip whose sound it has taken, while that clip plays.
    sound_from: std::collections::HashMap<u64, u64>,

    pub view: View,
    pub detail: DetailTab,
    pub sel_track: TrackSel,
    /// More tracks selected with Ctrl+click (by id), besides `sel_track`.
    pub(crate) sel_more: Vec<u64>,
    /// The Key drawer at the bottom is open.
    pub(crate) transpose_open: bool,
    /// The step of the key sequence the next press fills.
    pub(crate) key_seq_sel: Option<usize>,
    pub sel_clip: Option<ClipSel>,
    pub sel_scene: usize,
    pub file_path: Option<PathBuf>,
    pub message: Option<(String, Instant)>,

    pub kb_enabled: bool,
    pub kb_octave: i32,
    pub kb_velocity: f32,
    pub(crate) held_keys: HashMap<Key, (u8, Vec<u64>)>,
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
    /// MIDI recorded into the arrangement: per track (id), the clip it goes
    /// into and where the song was last (it ends when the loop jumps back).
    arr_takes: HashMap<u64, (u64, f64)>,
    /// The grand piano's samples being installed.
    pub piano_install: Option<crate::piano_lib::Install>,
    /// Orchestra instruments on their way, by instrument.
    pub orchestra_installs: std::collections::HashMap<usize, crate::orchestra_lib::Install>,
    /// Sampled drum packs being downloaded, and those that are here (by folder).
    pub(crate) drum_installs: std::collections::HashMap<&'static str, crate::drum_lib::Install>,
    /// The SoundFont banks downloading, by offer (see `sf2_lib::OFFERS`).
    pub(crate) sf2_installs: HashMap<usize, crate::sf2_lib::Install>,
    /// The voicebanks downloading, by offer (see `singer_lib::OFFERS`), and
    /// the tracks whose Singers asked for them.
    pub(crate) singer_installs: HashMap<usize, (crate::singer_lib::Install, Vec<u64>)>,
    /// The Touch view's plate held, cable being drawn, pulse light.
    pub(crate) touch_view: crate::ui::touch_view::TouchViewState,
    /// When the sound engine last failed and was restarted.
    engine_failed_at: Option<std::time::Instant>,
    pub(crate) packs_here: std::collections::HashSet<&'static str>,

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
    pub(crate) quit_requested: bool,
    /// Big Picture: the controller and touch view.
    pub big: crate::big::BigPicture,
    /// What was lifted off the tape, to drop elsewhere.
    pub(crate) tape_clip: Option<Vec<u32>>,
    /// The chord players that play the same chords.
    pub(crate) band: crate::band::Band,
    /// The played track the engine was last told of (for the tape).
    pub(crate) tape_follow_sent: Option<Option<u64>>,
    /// `shot=path`: where the next picture of the window goes, and whether
    /// it was asked for yet.
    shot: Option<(String, bool)>,
    /// The clip view shows envelopes (instead of notes or audio), which one,
    /// and whether dragging draws.
    pub(crate) env_view: bool,
    pub(crate) env_target: Option<krabton_engine::modulation::ModTarget>,
    pub(crate) env_draw: bool,
    /// Automation Arm: knobs turned while clips play are recorded into them.
    pub(crate) auto_arm: bool,
    /// The Glitch Sequencer shown in a window of its own (by device id).
    pub(crate) glitch_window: Option<u64>,
    /// The Map, in a window, and the track chosen on it.
    pub(crate) map_window: bool,
    pub(crate) map_selected: Option<u64>,
    /// Size of everything asked for by a script (see `run_script`).
    script_zoom: Option<f32>,
    /// A script opening a track's menu of sequencers and MIDI effects.
    script_menu: Option<usize>,
    /// The Studio: where a track's strip is in its packed row (the rest of
    /// the row is for its devices, under the strip too).
    pub(crate) pack_strip: Option<egui::Rect>,
    pub(crate) auto_prev: std::collections::HashMap<(usize, krabton_engine::modulation::ModTarget), f32>,
    /// The knob moves being recorded: their clip, where they started and
    /// last wrote, and the frame of that write.
    pub(crate) auto_last: std::collections::HashMap<(usize, krabton_engine::modulation::ModTarget), (ClipSel, f64, f64, u64)>,
    /// A size changed: save the layout once the mouse is up.
    pub layout_dirty: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        crate::skin::install_fonts(&cc.egui_ctx);
        // Test windows show widget id clashes in red, as debug builds do.
        if std::env::var_os("KRABTON_TEST_WINDOW").is_some() {
            cc.egui_ctx.options_mut(|o| o.warn_on_id_clash = true);
        }
        // The grand piano's samples, if installed (engines made from here on play them).
        krabton_engine::piano::library::set_shared(crate::piano_lib::open_installed());
        crate::orchestra_lib::open_all();
        let project = krabton_engine::demo::demo_project();
        let snapshot = Arc::new(project.clone());

        let prefs = crate::prefs::Prefs::load();
        let start_big = prefs.big_picture;
        // KRABTON_AUDIO_DEVICE (a device name) wins over the one picked in Options.
        let preferred = std::env::var("KRABTON_AUDIO_DEVICE").ok().or_else(|| prefs.audio_output.clone());
        let engine_slot = audio::new_slot();
        let opened = if std::env::var_os("KRABTON_NO_AUDIO").is_some() {
            Err("disabled by KRABTON_NO_AUDIO".to_string())
        } else {
            audio::open_output(preferred.as_deref(), &engine_slot, None)
        };
        let (audio_out, rate, audio_desc) = match opened {
            Ok((out, rate)) => {
                let d = out.description();
                (out, rate, d)
            }
            Err(e) => {
                eprintln!("No audio output: {e}\n{}", audio::no_audio_hint());
                let short = if std::env::var_os("KRABTON_NO_AUDIO").is_some() { format!("No audio: {e}") } else { "No audio output (see Options > Settings)".to_string() };
                (audio::start_null(engine_slot.clone(), 48000), 48000, short)
            }
        };
        // The engine at the output's rate, into the slot it plays from.
        let (eng, engine) = Engine::new(rate as f32, snapshot.clone());
        if let Ok(mut g) = engine_slot.lock() {
            *g = Some(eng);
        }

        let mut midi = Midi::new();
        midi.auto_connect();
        let midi_outs = crate::midi_out::MidiOutputs::start(engine.midi_out.clone(), engine.epoch);

        let mut plugins = PluginManager::new();
        plugins.scan();
        let mut app = Self {
            deferred: Vec::new(),
            plugins,
            fm_banks: vec![FmBank::factory()],
            sf2_open: Vec::new(),
            singer_open: Vec::new(),
            hyper_open: Vec::new(),
            hyper_found: std::cell::RefCell::new(None),
            firmware_unpack: None,
            lyric_text: String::new(),
            grid: crate::grid::GridState::new(),
            mod_learn: None,
            sel_mod: None,
            mod_history: HashMap::new(),
            committed: project.clone(),
            project,
            last_sent: snapshot,
            undo: Vec::new(),
            redo: Vec::new(),
            undo_labels: Vec::new(),
            redo_labels: Vec::new(),
            show_history: false,
            rack: Default::default(),
            dirty: false,
            engine,
            _audio: audio_out,
            engine_slot,
            audio_desc,
            audio_devices: None,
            audio_in_devices: None,
            show_settings: false,
            midi,
            midi_outs,
            pending: Vec::new(),
            sound_from: std::collections::HashMap::new(),
            view: View::Studio,
            detail: DetailTab::Clip,
            sel_track: TrackSel::Track(0),
            sel_more: Vec::new(),
            transpose_open: false,
            key_seq_sel: None,
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
            arr_takes: HashMap::new(),
            piano_install: None,
            orchestra_installs: std::collections::HashMap::new(),
            drum_installs: Default::default(),
            sf2_installs: HashMap::new(),
            singer_installs: HashMap::new(),
            touch_view: Default::default(),
            engine_failed_at: None,
            packs_here: krabton_engine::dsp::real_drums::PACKS.iter().filter(|p| krabton_engine::dsp::real_drums::installed(p)).map(|p| p.folder).collect(),
            meters: Vec::new(),
            return_meters: Vec::new(),
            master_meter: (0.0, 0.0),
            last_frame: Instant::now(),
            session: SessionState::default(),
            arr: ArrState::default(),
            piano: PianoState::default(),
            browser: BrowserState::new(),
            prefs,
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
            big: crate::big::BigPicture::new(crate::big::start_in_big_picture(start_big)),
            tape_clip: None,
            band: Default::default(),
            tape_follow_sent: None,
            shot: None,
            env_view: false,
            env_target: None,
            env_draw: false,
            auto_arm: false,
            glitch_window: None,
            map_window: false,
            map_selected: None,
            script_zoom: None,
            script_menu: None,
            pack_strip: None,
            auto_prev: Default::default(),
            auto_last: Default::default(),
            layout_dirty: false,
        };
        app.apply_prefs();
        // A set to open, from the file manager ("Open with Krabton").
        if let Some(path) = std::env::args_os().skip(1).map(std::path::PathBuf::from).find(|p| p.is_file()) {
            app.open_path(path);
        }
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
        self.band_frame();
        self.follow_clip_sounds();
        if *self.last_sent != self.project {
            // A clip moved to another track brings its sound with that track's ids.
            self.project.fix_clip_sound_ids();
            // Connections to deleted tracks, devices or modulators go away.
            self.project.prune_mod_routes();
            let snap = Arc::new(self.project.clone());
            self.engine.send(Command::SetProject(snap.clone()));
            self.last_sent = snap;
        }
        for c in self.pending.drain(..) {
            self.engine.send(c);
        }
        // Where the tracks' notes go out, and a port that would not open.
        let routes = self.project.tracks.iter().filter_map(|t| t.midi_to.as_ref().map(|m| (t.id, (m.port.clone(), m.channel.min(15))))).collect();
        self.midi_outs.set_routes(routes);
        if let Some(e) = self.midi_outs.error.lock().ok().and_then(|mut e| e.take()) {
            self.toast(e);
        }
        let pointer_down = ctx.input(|i| i.pointer.any_down());
        if !pointer_down && self.project != self.committed {
            // Notes of a pitch never overlap (as in Live), once an edit is done.
            self.tidy_clip_notes();
            let prev = std::mem::replace(&mut self.committed, self.project.clone());
            self.undo_labels.push(crate::history::describe(&prev, &self.project));
            self.undo.push(prev);
            if self.undo.len() > 200 {
                self.undo.remove(0);
                self.undo_labels.remove(0);
            }
            self.redo.clear();
            self.redo_labels.clear();
            self.dirty = true;
        }
    }

    /// Clips that keep a sound: the one that started gives its track that
    /// sound (no step to undo: the clip just plays), and while it plays what
    /// changes on the track is kept in it. A clip without one leaves the
    /// track as it is.
    fn follow_clip_sounds(&mut self) {
        let status = self.engine.status.clone();
        for ti in 0..self.project.tracks.len() {
            let Some(ts) = status.tracks.get(ti) else { break };
            let t = &self.project.tracks[ti];
            let slot = ts.playing_slot.load(Ordering::Relaxed);
            let clip = if slot >= 0 {
                t.slots.get(slot as usize).and_then(|c| c.as_ref()).map(|c| c.id)
            } else if !ts.overridden.load(Ordering::Relaxed) {
                let a = ts.arr_clip.load(Ordering::Relaxed);
                t.arrangement.iter().find(|x| a != 0 && x.id == a).map(|x| x.clip.id)
            } else {
                None
            };
            let track_id = t.id;
            let Some(clip) = clip.filter(|&c| t.clip_sound(c).is_some()) else {
                self.sound_from.remove(&track_id);
                continue;
            };
            if self.sound_from.get(&track_id) == Some(&clip) {
                self.project.tracks[ti].update_clip_sound(clip);
            } else {
                if self.project.tracks[ti].take_clip_sound(clip) {
                    // The same in what undo goes back to.
                    let t = &self.project.tracks[ti];
                    if let Some(c) = self.committed.tracks.iter_mut().find(|c| c.id == track_id) {
                        c.midi_devices = t.midi_devices.clone();
                        c.instrument = t.instrument.clone();
                        c.devices = t.devices.clone();
                    }
                }
                self.sound_from.insert(track_id, clip);
            }
        }
    }

    /// Keeps the track's sound in clip `sel` (anew, if it kept one).
    pub(crate) fn keep_sound_in_clip(&mut self, sel: ClipSel) {
        let Some(sound) = self.project.tracks.get(sel.track()).and_then(|t| t.sound_now()) else {
            self.toast("A track with a CLAP plug-in can't keep its sound in a clip (the plug-in keeps its own)");
            return;
        };
        if let Some(c) = self.clip_mut(sel) {
            c.sound = Some(Box::new(sound));
            let name = c.name.clone();
            self.toast(format!("{name} keeps this sound: it plays with it when it starts"));
        }
    }

    /// The clip `sel` no longer keeps a sound.
    pub(crate) fn forget_clip_sound(&mut self, sel: ClipSel) {
        if let Some(c) = self.clip_mut(sel) {
            c.sound = None;
        }
    }

    pub fn undo(&mut self) {
        if let Some(label) = self.undo_step() {
            self.toast(format!("Undo: {label}"));
        }
    }

    pub fn redo(&mut self) {
        if let Some(label) = self.redo_step() {
            self.toast(format!("Redo: {label}"));
        }
    }

    /// One step back; returns what it undid.
    fn undo_step(&mut self) -> Option<String> {
        let prev = self.undo.pop()?;
        let label = self.undo_labels.pop().unwrap_or_default();
        let cur = std::mem::replace(&mut self.project, prev);
        self.redo.push(cur);
        self.redo_labels.push(label.clone());
        self.committed = self.project.clone();
        // A clip playing with its sound gives it to its track again.
        self.sound_from.clear();
        self.dirty = true;
        Some(label)
    }

    /// One step forward; returns what it redid.
    fn redo_step(&mut self) -> Option<String> {
        let next = self.redo.pop()?;
        let label = self.redo_labels.pop().unwrap_or_default();
        let cur = std::mem::replace(&mut self.project, next);
        self.undo.push(cur);
        self.undo_labels.push(label.clone());
        self.committed = self.project.clone();
        self.sound_from.clear();
        self.dirty = true;
        Some(label)
    }

    /// Goes to the state after `steps` steps of the history (0: the start).
    pub(crate) fn history_go(&mut self, steps: usize) {
        while self.undo.len() > steps && self.undo_step().is_some() {}
        while self.undo.len() < steps && self.redo_step().is_some() {}
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
        self.project.tracks[track].arrangement.push(ArrangementClip::new(id, start, length, clip));
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
        if let BrowserItem::Orchestra(name) = item {
            if let Some(i) = krabton_engine::orchestra::index_of(name) {
                self.start_orchestra_install(i);
            }
        }
        let target_track = match target {
            DropTarget::Track(t) | DropTarget::Slot { track: t, .. } | DropTarget::ArrLane { track: t, .. } => Some(t),
            DropTarget::Chain(TrackSel::Track(t)) => Some(t),
            _ => None,
        };
        let is_midi = |app: &App, t: usize| app.project.tracks.get(t).map(|t| t.kind == TrackKind::Midi).unwrap_or(false);
        match item {
            BrowserItem::MidiFx(kind) | BrowserItem::MidiPreset(kind, _) => {
                let sel = match self.sel_track {
                    TrackSel::Track(t) if is_midi(self, t) => Some(t),
                    _ => None,
                };
                let track = match target {
                    DropTarget::NewTrack => {
                        use krabton_engine::midi_fx::MidiFxKind as K;
                        // A fitting instrument for each sequencer.
                        let inst = match kind {
                            K::Euclid | K::DrumGrid | K::DrumMap | K::Marbles | K::Chance | K::Bits => Instrument::drums("909 Classic"),
                            K::Percussion => Instrument::drums("808 Latin"),
                            K::Drummer => Instrument::drums("Studio Kit"),
                            K::Acid => Instrument::synth("Acid"),
                            K::Progression => Instrument::synth("Soft Pad"),
                            K::PianoPlayer => Instrument::synth("Keys"),
                            K::BassPlayer => Instrument::guitar("Finger Bass"),
                            K::GuitarChords => Instrument::guitar("Strummed Chords"),
                            K::Ukulele => Instrument::guitar("Ukulele"),
                            K::Banjo => Instrument::guitar("Banjo"),
                            K::Section => Instrument::strings("Bowed"),
                            K::OrganGrinder => Instrument::street_organ("Draaiorgel"),
                            K::MusicBox => self.music_box_sound(),
                            K::Sing => Instrument::singer(self.default_singer_bank()),
                            _ => Instrument::synth("Pluck"),
                        };
                        let t = self.add_midi_track(Some(inst));
                        Some(t)
                    }
                    _ => target_track.filter(|&t| is_midi(self, t)).or(sel),
                };
                match track {
                    Some(t) => {
                        let mut dev = self.project.make_midi_device(*kind);
                        if let BrowserItem::MidiPreset(_, name) = item {
                            if let Some(pr) = krabton_engine::presets::midi(*kind).iter().find(|p| p.name == *name) {
                                dev.params = krabton_engine::presets::apply_to(kind.specs(), pr, &dev.params);
                                if let Some(f) = pr.data {
                                    dev.data = f();
                                }
                                dev.preset = pr.name.to_string();
                            }
                        }
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
            BrowserItem::Synth(_) | BrowserItem::Drums(_) | BrowserItem::Sampler | BrowserItem::Macro(_) | BrowserItem::Braids(_) | BrowserItem::Elements(_) | BrowserItem::Analog(_) | BrowserItem::Modular(_) | BrowserItem::Shakers(_) | BrowserItem::Voice(_) | BrowserItem::Mallets(_) | BrowserItem::Strings(_) | BrowserItem::Winds(_) | BrowserItem::Organ(_) | BrowserItem::Rings(_) | BrowserItem::Monk(_) | BrowserItem::Piano(_) | BrowserItem::Bassline(_) | BrowserItem::Mono101(_) | BrowserItem::Drone(_) | BrowserItem::Waves(_) | BrowserItem::Guitar(_) | BrowserItem::WindChimes(_) | BrowserItem::Birds(_) | BrowserItem::Nature(_) | BrowserItem::PanFlute(_) | BrowserItem::Melodica(_) | BrowserItem::StreetOrgan(_) | BrowserItem::Harmonica(_) | BrowserItem::Calculator(_) | BrowserItem::Console(_) | BrowserItem::Handheld(_) | BrowserItem::Micro(_) | BrowserItem::Warp(_) | BrowserItem::Hybrid(_) | BrowserItem::Quartet(_) | BrowserItem::ChipFm(_) | BrowserItem::V2(_) | BrowserItem::Oidos(_) | BrowserItem::Orchestra(_) | BrowserItem::BeadsSynth(_) | BrowserItem::Facet(_) | BrowserItem::Coast(_) | BrowserItem::Crimson(_) | BrowserItem::Silicon(_) | BrowserItem::Theremin(_) | BrowserItem::Granulator(_) => {
                let inst = match item {
                    BrowserItem::Synth(p) => Instrument::synth(p),
                    BrowserItem::Macro(p) => Instrument::macro_osc(p),
                    BrowserItem::Braids(p) => Instrument::braids(p),
                    BrowserItem::Elements(p) => Instrument::elements(p),
                    BrowserItem::Analog(p) => Instrument::analog(p),
                    BrowserItem::Shakers(p) => Instrument::shakers(p),
                    BrowserItem::Modular(p) => Instrument::modular(p),
                    BrowserItem::Voice(p) => Instrument::voice(p),
                    BrowserItem::Mallets(p) => Instrument::mallets(p),
                    BrowserItem::Strings(p) => Instrument::strings(p),
                    BrowserItem::Winds(p) => Instrument::winds(p),
                    BrowserItem::Organ(p) => Instrument::organ(p),
                    BrowserItem::Rings(p) => Instrument::rings(p),
                    BrowserItem::Monk(p) => Instrument::monk(p),
                    BrowserItem::Piano(p) => Instrument::piano(p),
                    BrowserItem::Bassline(p) => Instrument::bassline(p),
                    BrowserItem::Mono101(p) => Instrument::mono101(p),
                    BrowserItem::Drone(p) => Instrument::drone(p),
                    BrowserItem::Waves(p) => Instrument::waves(p),
                    BrowserItem::Guitar(p) => Instrument::guitar(p),
                    BrowserItem::WindChimes(p) => Instrument::wind_chimes(p),
                    BrowserItem::BeadsSynth(p) => Instrument::beads_synth(p),
                    BrowserItem::Birds(p) => Instrument::birds(p),
                    BrowserItem::Nature(p) => Instrument::nature(p),
                    BrowserItem::PanFlute(p) => Instrument::pan_flute(p),
                    BrowserItem::Melodica(p) => Instrument::melodica(p),
                    BrowserItem::StreetOrgan(p) => Instrument::street_organ(p),
                    BrowserItem::Harmonica(p) => Instrument::harmonica(p),
                    BrowserItem::Calculator(p) => Instrument::calculator(p),
                    BrowserItem::Console(p) => Instrument::console(p),
                    BrowserItem::Handheld(p) => Instrument::handheld(p),
                    BrowserItem::Micro(p) => Instrument::micro(p),
                    BrowserItem::Warp(p) => Instrument::warp(p),
                    BrowserItem::Hybrid(p) => Instrument::hybrid(p),
                    BrowserItem::ChipFm(p) => Instrument::chipfm(p),
                    BrowserItem::V2(p) => Instrument::v2(p),
                    BrowserItem::Oidos(p) => Instrument::oidos(p),
                    BrowserItem::Quartet(p) => Instrument::quartet(p),
                    BrowserItem::Orchestra(p) => Instrument::orchestra(p),
                    BrowserItem::Facet(p) => Instrument::facet(p),
                    BrowserItem::Coast(p) => Instrument::coast(p),
                    BrowserItem::Crimson(p) => Instrument::crimson(p),
                    BrowserItem::Silicon(p) => Instrument::silicon(p),
                    BrowserItem::Theremin(p) => Instrument::theremin(p),
                    BrowserItem::Granulator(p) => Instrument::granulator(p),
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
            BrowserItem::Effect(_) | BrowserItem::EffectPreset(..) | BrowserItem::PluginEffect(_) => {
                let dev = match item {
                    BrowserItem::PluginEffect(info) => self.project.make_plugin_device(PluginManager::make_instrument(info, 0)),
                    BrowserItem::Effect(kind) => self.project.make_device(*kind),
                    BrowserItem::EffectPreset(kind, name) => {
                        let mut dev = self.project.make_device(*kind);
                        if let Some(pr) = krabton_engine::presets::effect(*kind).iter().find(|p| p.name == *name) {
                            dev.params = krabton_engine::presets::apply_to(kind.specs(), pr, &dev.params);
                            if let Some(f) = pr.data {
                                dev.data = f();
                            }
                            dev.preset = pr.name.to_string();
                        }
                        dev
                    }
                    _ => return,
                };
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
                        self.project.tracks[track].arrangement.push(ArrangementClip::new(aid, beat, length, clip));
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
            BrowserItem::Hyper | BrowserItem::HyperFirmware(_) => {
                let fw = match item {
                    BrowserItem::HyperFirmware(path) => match self.hyper_firmware(path) {
                        Some(f) => Some(f),
                        None => return,
                    },
                    _ => self.default_hyper_firmware(),
                };
                match target_track {
                    // A firmware on a Hyper: it plays it, from its first preset.
                    Some(t) if matches!(self.project.tracks.get(t).and_then(|tr| tr.instrument.as_ref()), Some(Instrument::Hyper { .. })) => {
                        if let (Some(f), Some(Instrument::Hyper { params, firmware, bank, program, preset, .. })) = (fw.clone(), self.project.tracks[t].instrument.as_mut()) {
                            let first = f.preset(0, 0).cloned();
                            *firmware = Some(f);
                            (*bank, *program) = (0, 0);
                            if let Some(pr) = first {
                                *preset = pr.name.clone();
                                *params = pr.knobs();
                            }
                        }
                        self.sel_track = TrackSel::Track(t);
                        self.detail = DetailTab::Devices;
                    }
                    Some(t) if is_midi(self, t) => {
                        self.project.tracks[t].instrument = Some(Instrument::hyper(fw.clone()));
                        self.sel_track = TrackSel::Track(t);
                        self.detail = DetailTab::Devices;
                    }
                    _ => {
                        self.add_midi_track(Some(Instrument::hyper(fw.clone())));
                    }
                }
                match &fw {
                    Some(f) => {
                        let n: usize = f.banks.iter().map(Vec::len).sum();
                        self.toast(format!("Hyper: {} ({}), {n} presets; it boots in a few seconds", f.name, f.model));
                    }
                    None => self.toast("Hyper needs a Virus firmware: open one on its display (a Virus TI's .bin, or an A, B or C operating system)"),
                }
            }
            BrowserItem::Singer | BrowserItem::Voicebank(_) => {
                let bank = match item {
                    BrowserItem::Voicebank(path) => match self.singer_bank(path) {
                        Some(b) => Some(b),
                        None => return,
                    },
                    _ => self.default_singer_bank(),
                };
                match target_track {
                    // A voicebank on a Singer: it sings with it, its knobs as they were.
                    Some(t) if matches!(self.project.tracks.get(t).and_then(|tr| tr.instrument.as_ref()), Some(Instrument::Singer { .. })) => {
                        if let Some(Instrument::Singer { bank: b, .. }) = self.project.tracks[t].instrument.as_mut() {
                            if bank.is_some() {
                                *b = bank.clone();
                            }
                        }
                        self.sel_track = TrackSel::Track(t);
                        self.detail = DetailTab::Devices;
                    }
                    Some(t) if is_midi(self, t) => {
                        let tr = &mut self.project.tracks[t];
                        let generic = tr.name.ends_with("MIDI") || tr.instrument.as_ref().map(|i| i.track_name() == tr.name || i.display_name() == tr.name).unwrap_or(true);
                        let inst = Instrument::singer(bank.clone());
                        if generic {
                            tr.name = bank.as_ref().map(|b| b.name.clone()).unwrap_or_else(|| inst.track_name());
                        }
                        tr.instrument = Some(inst);
                        self.sel_track = TrackSel::Track(t);
                        self.detail = DetailTab::Devices;
                    }
                    _ => {
                        let t = self.add_midi_track(Some(Instrument::singer(bank.clone())));
                        if let Some(b) = &bank {
                            self.project.tracks[t].name = b.name.clone();
                        }
                    }
                }
                if let Some(b) = &bank {
                    let kind = if b.vcv { "VCV" } else { "CV" };
                    self.toast(format!("{}: {} {kind} samples; type lyrics in the clip view", b.name, b.entries.len()));
                }
            }
            BrowserItem::Sf2Player | BrowserItem::Sf2 { .. } => {
                let inst = match item {
                    BrowserItem::Sf2 { path, bank, patch, .. } => {
                        let Some(font) = self.sf2_bank(path) else { return };
                        Instrument::sf2(Some(font), *bank, *patch)
                    }
                    _ => match self.default_sf2() {
                        Some(f) => {
                            let (b, p) = f.preset(0, 0).or_else(|| f.presets.first()).map(|p| (p.bank, p.patch)).unwrap_or((0, 0));
                            Instrument::sf2(Some(f), b, p)
                        }
                        None => Instrument::sf2(None, 0, 0),
                    },
                };
                match target_track {
                    Some(t) if is_midi(self, t) => {
                        let tr = &mut self.project.tracks[t];
                        let generic = tr.name.ends_with("MIDI") || tr.instrument.as_ref().map(|i| i.track_name() == tr.name || i.display_name() == tr.name).unwrap_or(true);
                        // Another preset of the same bank keeps the knobs where they are.
                        let inst = match (&tr.instrument, inst) {
                            (Some(Instrument::Sf2 { params, font: old, .. }), Instrument::Sf2 { font, bank, patch, preset, .. }) if *old == font => Instrument::Sf2 { params: params.clone(), font, bank, patch, preset },
                            (_, inst) => inst,
                        };
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
            BrowserItem::MidiFile(path) => {
                let Some(song) = self.read_midi_file(path) else { return };
                let name = item.label();
                match (target, target_track) {
                    (_, Some(t)) if !is_midi(self, t) => self.toast("MIDI files go on MIDI tracks"),
                    // On a track: all its notes as one clip.
                    (DropTarget::Slot { track, scene }, _) => {
                        let clip = self.midi_file_clip(&name, &song.all_notes(), song.clip_length(self.project.beats_per_bar_f()), track);
                        if let Some(slot) = self.project.tracks[track].slots.get_mut(scene) {
                            *slot = Some(clip);
                            self.select_clip(ClipSel::Slot { track, scene });
                        }
                    }
                    (DropTarget::ArrLane { track, beat }, _) => self.midi_file_on_arrangement(&name, &song, track, beat),
                    (_, Some(track)) if self.view == View::Arrangement => {
                        let bpb = self.project.beats_per_bar_f();
                        let beat = (self.engine.status.song_pos.load() / bpb).floor() * bpb;
                        self.midi_file_on_arrangement(&name, &song, track, beat);
                    }
                    (_, Some(track)) => {
                        let scene = self.sel_scene;
                        let clip = self.midi_file_clip(&name, &song.all_notes(), song.clip_length(self.project.beats_per_bar_f()), track);
                        if let Some(slot) = self.project.tracks[track].slots.get_mut(scene) {
                            *slot = Some(clip);
                            self.select_clip(ClipSel::Slot { track, scene });
                        }
                    }
                    // Anywhere else: a track for each of its tracks, as Live does.
                    (_, None) => self.midi_file_tracks(&name, &song, None),
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
                        self.project.tracks[track].arrangement.push(ArrangementClip::new(id, beat, length, clip));
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
                                Some(Instrument::Granulator { sample: s, .. }) => *s = Some(sample),
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
                                self.project.tracks[t].arrangement.push(ArrangementClip::new(id, start, length, clip));
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

    /// A SoundFont bank: read once and kept open (the last few), and
    /// remembered for the browser. A toast says why one can't be read.
    pub(crate) fn sf2_bank(&mut self, path: &Path) -> Option<krabton_engine::dsp::sf2::Sf2Ref> {
        match self.sf2_open_bank(path) {
            Ok(f) => Some(f),
            Err(e) => {
                self.toast(e);
                None
            }
        }
    }

    /// [`App::sf2_bank`], quietly.
    pub(crate) fn sf2_open_bank(&mut self, path: &Path) -> Result<krabton_engine::dsp::sf2::Sf2Ref, String> {
        if let Some(f) = self.sf2_open.iter().find(|f| f.path.as_deref() == Some(path)) {
            return Ok(f.clone());
        }
        let f = krabton_engine::dsp::sf2::Sf2Ref::load(path)?;
        if self.sf2_open.len() >= 4 {
            self.sf2_open.remove(0);
        }
        self.sf2_open.push(f.clone());
        if !self.prefs.sf2_banks.iter().any(|p| p == path) {
            self.prefs.sf2_banks.push(path.to_path_buf());
            self.prefs.save();
        }
        Ok(f)
    }

    /// A music box to play the Music Box sequencer: the General MIDI bank's
    /// music box when there is a bank, else a glockenspiel.
    fn music_box_sound(&mut self) -> Instrument {
        match self.default_sf2() {
            Some(f) if f.is_gm() => match f.gm(10, false).map(|p| (p.bank, p.patch)) {
                Some((b, p)) => Instrument::sf2(Some(f), b, p),
                None => Instrument::elements("Glockenspiel"),
            },
            _ => Instrument::elements("Glockenspiel"),
        }
    }

    /// A UTAU voicebank: read and kept open (the last few), remembered for the
    /// browser; its pitch periods are found in the background.
    pub(crate) fn singer_bank(&mut self, path: &Path) -> Option<krabton_engine::singer::VoicebankRef> {
        if let Some(b) = self.singer_open.iter().find(|b| b.path.as_deref() == Some(path)) {
            return Some(b.clone());
        }
        match krabton_engine::singer::VoicebankRef::load(path) {
            Ok(b) => {
                if self.singer_open.len() >= 3 {
                    self.singer_open.remove(0);
                }
                self.singer_open.push(b.clone());
                if !self.prefs.singer_banks.iter().any(|p| p == path) {
                    self.prefs.singer_banks.push(path.to_path_buf());
                    self.prefs.save();
                }
                Some(b)
            }
            Err(e) => {
                self.toast(e);
                None
            }
        }
    }

    /// The voicebank a new Singer starts on: the last one opened (or
    /// downloaded) that is still there.
    pub(crate) fn default_singer_bank(&mut self) -> Option<krabton_engine::singer::VoicebankRef> {
        let mut paths: Vec<PathBuf> = self.prefs.singer_banks.iter().rev().cloned().collect();
        paths.extend(crate::singer_lib::all_installed());
        let path = paths.into_iter().find(|p| p.is_dir())?;
        self.singer_bank(&path)
    }

    /// Downloads an offered voicebank (unless it is here or on its way); the
    /// Singer on track `asking` gets it when it arrives.
    pub(crate) fn start_singer_install(&mut self, i: usize, asking: Option<u64>) {
        let Some(offer) = crate::singer_lib::OFFERS.get(i) else { return };
        if let Some((_, tracks)) = self.singer_installs.get_mut(&i) {
            if let Some(t) = asking {
                tracks.push(t);
            }
            let busy = self.singer_installs.get(&i).and_then(|(s, _)| s.lock().ok().map(|s| matches!(*s, crate::singer_lib::State::Downloading { .. } | crate::singer_lib::State::Unpacking))).unwrap_or(false);
            if busy {
                return;
            }
        }
        if crate::singer_lib::installed(offer).is_some() {
            return;
        }
        self.singer_installs.insert(i, (crate::singer_lib::start(i), asking.into_iter().collect()));
        self.toast(format!("Downloading {} ({} MB)...", offer.name, offer.bytes / 1_000_000));
    }

    /// What the Singer's display offers to get, and how far each is (1.0: unpacking).
    pub(crate) fn singer_offers(&self) -> Vec<crate::ui::faces::sf2::OfferView> {
        use crate::ui::faces::sf2::{OfferState, OfferView};
        crate::singer_lib::OFFERS
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let state = match self.singer_installs.get(&i).and_then(|(s, _)| s.lock().ok().map(|s| s.clone())) {
                    Some(crate::singer_lib::State::Downloading { bytes }) => OfferState::Downloading((bytes as f32 / o.bytes.max(1) as f32).min(0.99)),
                    Some(crate::singer_lib::State::Unpacking) => OfferState::Downloading(1.0),
                    Some(crate::singer_lib::State::Failed(_)) => OfferState::Failed,
                    _ if crate::singer_lib::installed(o).is_some() => OfferState::Here,
                    _ => OfferState::Get,
                };
                OfferView { name: o.name, mb: o.bytes / 1_000_000, state }
            })
            .collect()
    }

    /// A voicebank arrived: the Singers that asked for it (and still have
    /// none) sing with it, and the browser keeps it.
    fn poll_singer_installs(&mut self) {
        let mut done = Vec::new();
        for (&i, (s, _)) in &self.singer_installs {
            match s.lock().map(|s| s.clone()) {
                Ok(crate::singer_lib::State::Ready(bank)) => done.push((i, Ok(bank))),
                Ok(crate::singer_lib::State::Failed(e)) if !e.is_empty() => done.push((i, Err(e))),
                _ => {}
            }
        }
        for (i, result) in done {
            match result {
                Ok(bank) => {
                    let asking = self.singer_installs.remove(&i).map(|(_, t)| t).unwrap_or_default();
                    let Some(path) = bank.path.clone() else { continue };
                    if self.singer_open.len() >= 3 {
                        self.singer_open.remove(0);
                    }
                    self.singer_open.push(bank.clone());
                    if !self.prefs.singer_banks.contains(&path) {
                        self.prefs.singer_banks.push(path);
                        self.prefs.save();
                    }
                    for t in self.project.tracks.iter_mut().filter(|t| asking.contains(&t.id)) {
                        if let Some(Instrument::Singer { bank: b, .. }) = t.instrument.as_mut() {
                            if b.as_ref().is_none_or(|b| b.is_missing()) {
                                *b = Some(bank.clone());
                                if t.name == "Singer" {
                                    t.name = bank.name.clone();
                                }
                            }
                        }
                    }
                    let kind = if bank.vcv { "VCV" } else { "CV" };
                    self.toast(format!("{} is here: {} {kind} samples", bank.name, bank.entries.len()));
                }
                // The display shows it failed (and offers it again); say why once.
                Err(e) => {
                    if let Some((s, _)) = self.singer_installs.get(&i) {
                        if let Ok(mut st) = s.lock() {
                            *st = crate::singer_lib::State::Failed(String::new());
                        }
                    }
                    self.toast(e);
                }
            }
        }
    }

    /// A Virus firmware: read and kept open (the last few), remembered.
    pub(crate) fn hyper_firmware(&mut self, path: &Path) -> Option<krabton_engine::hyper::FirmwareRef> {
        if let Some(f) = self.hyper_open.iter().find(|f| f.path.as_deref() == Some(path)) {
            return Some(f.clone());
        }
        match krabton_engine::hyper::FirmwareRef::load(path) {
            Ok(f) => {
                if self.hyper_open.len() >= 3 {
                    self.hyper_open.remove(0);
                }
                self.hyper_open.push(f.clone());
                if !self.prefs.hyper_firmware.iter().any(|p| p == path) {
                    self.prefs.hyper_firmware.push(path.to_path_buf());
                    self.prefs.save();
                }
                Some(f)
            }
            Err(e) => {
                self.toast(format!("{}: {e}", path.display()));
                None
            }
        }
    }

    /// Where Virus firmware may be: the ones opened before, and the folders
    /// the emulator's own plugins and Krabton's library keep them in.
    pub(crate) fn hyper_found(&self) -> Vec<PathBuf> {
        if let Some((at, found)) = &*self.hyper_found.borrow() {
            if at.elapsed().as_secs_f32() < 5.0 {
                return found.clone();
            }
        }
        let mut found: Vec<PathBuf> = self.prefs.hyper_firmware.iter().rev().filter(|p| p.is_file()).cloned().collect();
        let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default();
        let docs = if cfg!(any(windows, target_os = "macos")) { home.join("Documents") } else { std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share")) };
        let dirs = [
            docs.join("The Usual Suspects").join("OsTIrus").join("roms"),
            docs.join("The Usual Suspects").join("Osirus").join("roms"),
            crate::piano_lib::library_dir().parent().map(Path::to_path_buf).unwrap_or_default().join("Firmware"),
        ];
        for dir in dirs {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            // .bin and .mid files, and a TI firmware whatever it is called.
            let mut files: Vec<PathBuf> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .filter(|p| (p.extension().is_some_and(|e| e.eq_ignore_ascii_case("bin") || e.eq_ignore_ascii_case("mid")) && std::fs::metadata(p).map(|m| m.len() >= 100_000).unwrap_or(false)) || krabton_engine::hyper::looks_like_ti_firmware(p))
                .collect();
            files.sort();
            for f in files {
                if !found.contains(&f) {
                    found.push(f);
                }
            }
        }
        // Where Access's own Virus TI software puts the firmware when it is installed.
        let mut installed: Vec<PathBuf> = Vec::new();
        if cfg!(windows) {
            for var in ["CommonProgramFiles", "CommonProgramFiles(x86)", "CommonProgramW6432", "ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
                if let Some(d) = std::env::var_os(var) {
                    installed.push(PathBuf::from(d).join("Access Music"));
                }
            }
        } else if cfg!(target_os = "macos") {
            installed.push(PathBuf::from("/Library/Application Support/Access Music"));
            installed.push(home.join("Library/Application Support/Access Music"));
        } else {
            // Linux: where Access's Windows installer puts it, run with Wine.
            for prefix in [std::env::var_os("WINEPREFIX").map(PathBuf::from), Some(home.join(".wine"))].into_iter().flatten() {
                for sub in ["Program Files", "Program Files (x86)", "Program Files/Common Files", "Program Files (x86)/Common Files"] {
                    installed.push(prefix.join("drive_c").join(sub).join("Access Music"));
                }
            }
        }
        for dir in installed.iter().filter(|d| d.is_dir()) {
            for f in krabton_engine::hyper::find_ti_firmware(dir) {
                if !found.contains(&f) {
                    found.push(f);
                }
            }
        }
        *self.hyper_found.borrow_mut() = Some((std::time::Instant::now(), found.clone()));
        found
    }

    /// The firmware a new Hyper starts on: the last one opened, or one found.
    pub(crate) fn default_hyper_firmware(&mut self) -> Option<krabton_engine::hyper::FirmwareRef> {
        if let Some(f) = self.hyper_open.last() {
            return Some(f.clone());
        }
        let found = self.hyper_found();
        for path in found {
            if let Ok(f) = krabton_engine::hyper::FirmwareRef::load(&path) {
                self.hyper_open.push(f.clone());
                return Some(f);
            }
        }
        None
    }

    /// Opens a Virus firmware for Hyper (a TI's whatever the file is called),
    /// or Access's installer it is in (unpacked for it, in the background).
    pub(crate) fn open_firmware_dialog(&mut self) -> Option<krabton_engine::hyper::FirmwareRef> {
        let path = rfd::FileDialog::new()
            .set_title("A Virus firmware (a Virus TI's: Access's container, 6 to 9 MB; or an A, B or C operating system, .bin or .mid), or Access's Virus TI installer it is in")
            .add_filter("All files", &["*"])
            .add_filter("Virus firmware", &["bin", "BIN", "mid", "MID"])
            .add_filter("Installers and archives", &["exe", "msi", "dmg", "pkg", "zip", "7z", "cab", "EXE", "MSI", "DMG", "PKG", "ZIP"])
            .pick_file()?;
        *self.hyper_found.borrow_mut() = None;
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if ext != "bin" && ext != "mid" && !krabton_engine::hyper::looks_like_ti_firmware(&path) && crate::firmware_lib::is_container(&path) {
            self.start_firmware_unpack(path);
            return None;
        }
        self.hyper_firmware(&path)
    }

    /// Unpacks an installer for its Virus TI firmware, in the background;
    /// the Hypers without a firmware get it when it is out.
    pub(crate) fn start_firmware_unpack(&mut self, installer: PathBuf) {
        if self.firmware_unpack.is_some() {
            self.toast("Already unpacking an installer for its firmware: a moment");
            return;
        }
        let name = installer.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        self.toast(format!("Unpacking {name} for its Virus TI firmware (this can take a minute)"));
        self.firmware_unpack = Some((crate::firmware_lib::start(installer), name));
    }

    fn poll_firmware_unpack(&mut self) {
        let Some((rx, name)) = &self.firmware_unpack else { return };
        let name = name.clone();
        let result = match rx.try_recv() {
            Ok(r) => r,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(format!("Unpacking {name} stopped")),
        };
        self.firmware_unpack = None;
        let path = match result {
            Ok(path) => path,
            Err(e) => {
                self.toast(e);
                return;
            }
        };
        *self.hyper_found.borrow_mut() = None;
        let Some(fw) = self.hyper_firmware(&path) else { return };
        let mut given = 0;
        for t in self.project.tracks.iter_mut() {
            if let Some(Instrument::Hyper { params, firmware, bank, program, preset, .. }) = t.instrument.as_mut() {
                if firmware.as_ref().is_none_or(|f| f.is_missing()) {
                    *firmware = Some(fw.clone());
                    (*bank, *program) = (0, 0);
                    if let Some(pr) = fw.preset(0, 0) {
                        *preset = pr.name.clone();
                        *params = pr.knobs();
                    }
                    given += 1;
                }
            }
        }
        let dir = crate::firmware_lib::firmware_dir();
        let to = if given > 0 { ", Hyper boots on it" } else { ": drag it onto a track from the browser's Hyper folder" };
        self.toast(format!("The {} firmware is out of {name}, kept in {}{to}", fw.model, dir.display()));
    }

    /// Looks through a folder (and the folders in it) for a Virus TI's
    /// firmware: where Access's installer was unpacked or installed; or, with
    /// none there, for the installer itself to take it out of.
    pub(crate) fn find_firmware_dialog(&mut self) -> Option<krabton_engine::hyper::FirmwareRef> {
        let dir = rfd::FileDialog::new().set_title("A folder to look in for a Virus TI firmware (where Access's Virus TI software was unpacked or installed, or its installer was downloaded)").pick_folder()?;
        let found = krabton_engine::hyper::find_ti_firmware(&dir);
        if found.is_empty() {
            // Access's installer there, not unpacked: unpacked for it.
            if let Some(installer) = crate::firmware_lib::installers_in(&dir).into_iter().next() {
                self.start_firmware_unpack(installer);
                return None;
            }
            self.toast(format!("No Virus TI firmware in {} (a file of 6 to 9 MB that starts with FORM), and no Virus installer to take it out of", dir.display()));
            return None;
        }
        *self.hyper_found.borrow_mut() = None;
        let mut last_err = String::new();
        for path in &found {
            match krabton_engine::hyper::FirmwareRef::load(path) {
                Ok(_) => return self.hyper_firmware(path),
                Err(e) => last_err = e,
            }
        }
        self.toast(format!("Found {} file(s) like a Virus TI firmware, but none could be read: {last_err}", found.len()));
        None
    }

    /// Every Hyper with a firmware gets a device booted from it (on a thread
    /// of its own); one without lets its device go.
    fn boot_hypers(&mut self) {
        let rate = self.engine.status.sample_rate.load();
        if rate < 8000.0 {
            return;
        }
        for t in &self.project.tracks {
            if let Some(Instrument::Hyper { firmware, engine, .. }) = &t.instrument {
                match firmware.as_ref().filter(|f| !f.is_missing()) {
                    Some(f) => engine.boot(f, rate),
                    None if engine.status() != krabton_engine::hyper::Status::Off => engine.stop(),
                    None => {}
                }
            }
        }
    }

    /// Opens a voicebank folder for the Singer.
    pub(crate) fn open_voicebank_dialog(&mut self) -> Option<krabton_engine::singer::VoicebankRef> {
        let dir = rfd::FileDialog::new().set_title("A UTAU voicebank's folder (with its oto.ini)").pick_folder()?;
        self.singer_bank(&dir)
    }

    /// Puts typed lyrics on a clip's notes, a syllable each in time order: on
    /// the selected notes, or all of them; nothing typed takes them off.
    pub(crate) fn put_lyrics(&mut self, sel: ClipSel, text: &str) {
        let syllables = krabton_engine::singer::kana::split_lyrics(text);
        let picked: Vec<usize> = self.piano.selected.iter().copied().collect();
        let Some(notes) = self.clip_mut(sel).and_then(|c| c.notes_mut()) else { return };
        let mut order: Vec<usize> = if picked.is_empty() { (0..notes.len()).collect() } else { picked.into_iter().filter(|&i| i < notes.len()).collect() };
        // In time order, the highest of notes together first (the tune).
        order.sort_by(|&a, &b| notes[a].start.total_cmp(&notes[b].start).then(notes[b].pitch.cmp(&notes[a].pitch)));
        let n = order.len();
        if syllables.is_empty() {
            for &i in &order {
                notes[i].lyric.clear();
            }
            self.toast(format!("Lyrics off {n} note{}", if n == 1 { "" } else { "s" }));
            return;
        }
        for (&i, s) in order.iter().zip(&syllables) {
            notes[i].lyric = s.clone();
        }
        let used = syllables.len().min(n);
        self.toast(format!("{used} syllable{} on {used} note{}{}", if used == 1 { "" } else { "s" }, if used == 1 { "" } else { "s" }, if syllables.len() > n { format!(" ({} left over)", syllables.len() - n) } else { String::new() }));
    }

    /// Opens SoundFont banks from disk into the browser's SF2 Player folder.
    pub(crate) fn open_sf2_dialog(&mut self) {
        let Some(files) = rfd::FileDialog::new().add_filter("SoundFont 2 bank", &["sf2", "SF2"]).pick_files() else { return };
        for f in files {
            if let Some(bank) = self.sf2_bank(&f) {
                let gm = if bank.is_gm() { ", a General MIDI set" } else { "" };
                self.toast(format!("{}: {} presets{gm}", bank.name, bank.presets.len()));
            }
        }
    }

    /// Reads a MIDI file (a toast says why one can't be).
    pub(crate) fn read_midi_file(&mut self, path: &Path) -> Option<midi_file::MidiSong> {
        let name = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        match std::fs::read(path).map_err(|e| e.to_string()).and_then(|b| midi_file::read(&b)) {
            Ok(song) if !song.tracks.is_empty() => Some(song),
            Ok(_) => {
                self.toast(format!("{name}: there are no notes in it"));
                None
            }
            Err(e) => {
                self.toast(format!("{name}: {e}"));
                None
            }
        }
    }

    /// A MIDI clip of `notes`, `length` beats long, in track `track`'s colour.
    fn midi_file_clip(&mut self, name: &str, notes: &[Note], length: f64, track: usize) -> Clip {
        let color = self.project.tracks.get(track).map(|t| t.color).unwrap_or_default();
        let mut clip = Clip::new_midi(self.project.alloc_id(), name, color, length);
        if let Some(n) = clip.notes_mut() {
            *n = notes.to_vec();
        }
        clip
    }

    /// All of a MIDI file's notes as one clip on track `track`'s timeline at `beat`.
    fn midi_file_on_arrangement(&mut self, name: &str, song: &midi_file::MidiSong, track: usize, beat: f64) {
        let length = song.clip_length(self.project.beats_per_bar_f());
        let clip = self.midi_file_clip(name, &song.all_notes(), length, track);
        let id = self.project.alloc_id();
        self.project.tracks[track].arrangement.push(ArrangementClip::new(id, beat.max(0.0), length, clip));
        self.select_clip(ClipSel::Arr { track, id });
    }

    /// A MIDI file's tracks as new MIDI tracks, each with an instrument that
    /// fits it and its notes as a clip: on the arrangement at `at` (or the
    /// playhead's bar) in Arrangement view, else in the selected scene.
    pub(crate) fn midi_file_tracks(&mut self, name: &str, song: &midi_file::MidiSong, at: Option<f64>) {
        // The file's tempo and metre become the song's (undo brings the old ones back).
        let mut took = Vec::new();
        if let Some(bpm) = song.bpm.map(|b| (b.clamp(20.0, 999.0) * 100.0).round() / 100.0).filter(|b| (b - self.project.bpm).abs() > 0.004) {
            self.project.bpm = bpm;
            took.push(format!("at {} BPM", fmt_bpm(bpm)));
        }
        if let Some(n) = song.beats_per_bar.filter(|&n| (1..=16).contains(&n) && n != self.project.beats_per_bar) {
            self.project.beats_per_bar = n;
            took.push(format!("in {n}/4"));
        }
        let bpb = self.project.beats_per_bar_f();
        let length = song.clip_length(bpb);
        let on_arrangement = at.is_some() || self.view == View::Arrangement;
        let start = at.unwrap_or_else(|| (self.engine.status.song_pos.load() / bpb).floor() * bpb).max(0.0);
        let scene = self.sel_scene;
        let single = song.tracks.len() == 1;
        let mut first = None;
        let gm = self.prefs.gm_bank.clone().and_then(|p| self.sf2_bank(&p));
        for st in &song.tracks {
            let inst = match &gm {
                Some(bank) => midi_file_gm(bank, st),
                None => midi_file_instrument(st),
            };
            let t = self.project.add_midi_track(Some(inst));
            first.get_or_insert(t);
            let label = if single { name.to_string() } else { st.name.clone() };
            self.project.tracks[t].name = label.clone();
            let clip = self.midi_file_clip(&label, &st.notes, length, t);
            if on_arrangement {
                let id = self.project.alloc_id();
                self.project.tracks[t].arrangement.push(ArrangementClip::new(id, start, length, clip));
            } else if let Some(slot) = self.project.tracks[t].slots.get_mut(scene) {
                *slot = Some(clip);
            }
        }
        if let Some(t) = first {
            self.sel_track = TrackSel::Track(t);
            self.detail = DetailTab::Devices;
        }
        let n = song.tracks.len();
        let bars = (length / bpb).round();
        let tempo = if took.is_empty() { String::new() } else { format!("; the song is now {}", took.join(", ")) };
        self.toast(format!("{name}: {n} track{}, {bars} bar{}{tempo}", if n == 1 { "" } else { "s" }, if bars == 1.0 { "" } else { "s" }));
    }

    /// Opens a MIDI file from disk onto new tracks.
    pub fn import_midi_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().add_filter("MIDI file", &["mid", "midi", "MID"]).pick_file() {
            self.apply_item(&BrowserItem::MidiFile(path), DropTarget::NewTrack);
        }
    }

    /// Saves a MIDI clip's notes as a MIDI file.
    pub fn export_midi_clip(&mut self, sel: ClipSel) {
        let Some(clip) = self.clip_ref(sel) else { return };
        if !clip.is_midi() {
            return;
        }
        let bytes = midi_file::write(clip.notes(), clip.length, &clip.name, self.project.bpm);
        let file = clip.name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-");
        let Some(path) = rfd::FileDialog::new().add_filter("MIDI file", &["mid"]).set_file_name(format!("{file}.mid")).save_file() else { return };
        match std::fs::write(&path, bytes) {
            Ok(()) => self.toast(format!("Saved {}", path.display())),
            Err(e) => self.toast(format!("Export failed: {e}")),
        }
    }

    /// "To Clip": `bars` bars of what a track's sequencer plays, as a clip
    /// on the track: in the selected scene (or the next free slot, or a new
    /// scene), or in Arrangement view at the playhead's bar. The sequencer
    /// turns off, so what plays is the clip.
    pub fn sequencer_to_clip(&mut self, track: usize, dev_id: u64, bars: u32) {
        let Some(t) = self.project.tracks.get(track) else { return };
        let Some(dev) = t.midi_devices.iter().find(|d| d.id == dev_id).cloned() else { return };
        let bpb = self.project.beats_per_bar_f();
        let bars = bars.max(1);
        let beats = bars as f64 * bpb;
        let kit = t.instrument.as_ref().map(|i| i.drum_notes()).unwrap_or(0);
        let p = &self.project;
        let notes = midi_fx::render_notes(dev.kind, &dev.params, &dev.data, beats, p.bpm, p.scale, p.scale_mode, kit);
        let name = dev.kind.name();
        let s = if bars == 1 { "" } else { "s" };
        if notes.is_empty() {
            self.toast(format!("{name} plays nothing in {bars} bar{s}"));
            return;
        }
        let clip = self.midi_file_clip(name, &notes, beats, track);
        let place = if self.view == View::Arrangement {
            let start = (self.engine.status.song_pos.load() / bpb).floor() * bpb;
            let id = self.project.alloc_id();
            self.project.tracks[track].arrangement.push(ArrangementClip::new(id, start, beats, clip));
            self.select_clip(ClipSel::Arr { track, id });
            "on the arrangement".to_string()
        } else {
            let n = self.project.scenes.len();
            let free = (0..n).map(|k| (self.sel_scene + k) % n).find(|&s| matches!(self.project.tracks[track].slots.get(s), Some(None)));
            let scene = free.unwrap_or_else(|| {
                self.project.add_scene(n);
                n
            });
            self.project.tracks[track].slots[scene] = Some(clip);
            self.select_clip(ClipSel::Slot { track, scene });
            format!("in scene {}", scene + 1)
        };
        if let Some(d) = self.project.tracks[track].midi_devices.iter_mut().find(|d| d.id == dev_id) {
            d.enabled = false;
        }
        self.toast(format!("{name}: {bars} bar{s} as a clip {place}; the sequencer is off now, so you hear the clip"));
    }

    // ------------------------------------------------------------------
    // Transport & notes
    // ------------------------------------------------------------------

    pub fn toggle_play(&mut self) {
        if self.is_playing() {
            self.cmd(Command::Stop);
        } else {
            self.tape_before_play();
            self.cmd(Command::Play);
        }
    }

    /// The MIDI tracks that hear a live note (see `Project::live_note_targets`).
    fn note_targets_from(&self, channel: Option<u8>) -> Vec<usize> {
        self.project.live_note_targets_of(&self.selected_tracks(), channel)
    }

    /// The selected tracks: the one with the focus first, then the ones
    /// added with Ctrl+click.
    pub(crate) fn selected_tracks(&self) -> Vec<usize> {
        let mut v = Vec::new();
        if let TrackSel::Track(i) = self.sel_track {
            v.push(i);
        }
        for id in &self.sel_more {
            if let Some(i) = self.project.tracks.iter().position(|t| t.id == *id) {
                if !v.contains(&i) {
                    v.push(i);
                }
            }
        }
        v
    }

    pub(crate) fn is_selected(&self, i: usize) -> bool {
        self.sel_track == TrackSel::Track(i) || self.project.tracks.get(i).is_some_and(|t| self.sel_more.contains(&t.id))
    }

    /// A click on a track's name: it alone, or with Ctrl it joins (or
    /// leaves) the selection.
    pub(crate) fn click_track(&mut self, i: usize, ctrl: bool) {
        let Some(id) = self.project.tracks.get(i).map(|t| t.id) else { return };
        if !ctrl {
            self.sel_more.clear();
            self.sel_track = TrackSel::Track(i);
            return;
        }
        if self.sel_track == TrackSel::Track(i) {
            // The focus goes to another selected track, if there is one.
            if let Some(next) = self.sel_more.pop() {
                if let Some(k) = self.project.tracks.iter().position(|t| t.id == next) {
                    self.sel_track = TrackSel::Track(k);
                }
            }
        } else if let Some(k) = self.sel_more.iter().position(|x| *x == id) {
            self.sel_more.remove(k);
        } else {
            if let TrackSel::Track(old) = self.sel_track {
                if let Some(t) = self.project.tracks.get(old) {
                    if !self.sel_more.contains(&t.id) {
                        self.sel_more.push(t.id);
                    }
                }
            }
            self.sel_track = TrackSel::Track(i);
        }
    }

    /// A click on a track's arm button: it alone is armed (all of the
    /// selection when it is part of one); with Ctrl it is armed too.
    pub(crate) fn click_arm(&mut self, i: usize, ctrl: bool) {
        let Some(now) = self.project.tracks.get(i).map(|t| !t.arm) else { return };
        let sel = self.selected_tracks();
        let group = if sel.len() > 1 && sel.contains(&i) { sel } else { vec![i] };
        if !ctrl {
            for (k, t) in self.project.tracks.iter_mut().enumerate() {
                if !group.contains(&k) {
                    t.arm = false;
                }
            }
        }
        for &k in &group {
            if let Some(t) = self.project.tracks.get_mut(k) {
                if matches!(t.kind, TrackKind::Midi | TrackKind::Audio) {
                    t.arm = now;
                }
            }
        }
        if group.len() == 1 && !ctrl {
            self.click_track(i, false);
        }
    }

    fn note_targets(&self) -> Vec<usize> {
        self.note_targets_from(None)
    }

    /// What you play sounds as you play it, also when the Key drawer moves
    /// the song into another key: the note is sent as the song's key's
    /// note that the drawer moves back to it.
    fn unmove_live(&self, pitch: u8) -> u8 {
        let Some(to) = self.current_play_key().filter(|k| *k != self.project.scale) else { return pitch };
        let from = self.project.scale;
        let mut q = krabton_engine::scale::convert_pitch(pitch, &to, &from) as i32;
        // The way back may land an octave off (a tritone apart).
        let back = krabton_engine::scale::convert_pitch(q.clamp(0, 127) as u8, &from, &to) as i32;
        let off = pitch as i32 - back;
        if off % 12 == 0 {
            q += off;
        }
        q.clamp(0, 127) as u8
    }

    /// How many tracks the computer keyboard plays.
    pub(crate) fn note_targets_count(&self) -> usize {
        self.note_targets().len()
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
        let pitch = if self.snap_input && self.project.scale_mode { self.project.sounding_key().snap(pitch) } else { pitch };
        let pitch = self.unmove_live(pitch);
        let targets = self.note_targets_from(channel);
        let mut ids = Vec::new();
        // An Endless that records takes the note as its next step.
        for &ti in &targets {
            for dev in self.project.tracks[ti].midi_devices.iter_mut() {
                let rec = krabton_engine::sequencers::played::endless::RECORD;
                if dev.enabled && dev.kind == krabton_engine::midi_fx::MidiFxKind::Endless && dev.params.get(rec).is_some_and(|v| *v >= 0.5) {
                    krabton_engine::sequencers::played::endless_add(&mut dev.data, pitch);
                }
            }
        }
        for ti in targets {
            let id = self.project.tracks[ti].id;
            ids.push(id);
            self.cmd(Command::NoteOn { track: id, pitch, velocity: vel });
            self.rec_note_on(ti, pitch, vel);
        }
        (pitch, ids)
    }

    /// A live note played on track `ti` goes into the clip it records into
    /// (with Record on and the transport going); it is written when it ends.
    pub(crate) fn rec_note_on(&mut self, ti: usize, pitch: u8, vel: f32) {
        if !self.recording || !self.is_playing() {
            return;
        }
        let status = self.status();
        let id = self.project.tracks[ti].id;
        if let Some(&(aid, _)) = self.arr_takes.get(&id) {
            let song = status.song_pos.load();
            if let Some(a) = self.project.tracks[ti].arrangement.iter().find(|a| a.id == aid) {
                if song >= a.start {
                    self.rec_notes.insert((id, pitch), RecNote { scene: 0, start: song - a.start, vel, arr: Some(aid) });
                }
            }
            return;
        }
        let ts = &status.tracks[ti];
        let slot = ts.playing_slot.load(Ordering::Relaxed);
        let pos = ts.clip_pos.load();
        if slot >= 0 && pos >= 0.0 {
            self.rec_notes.insert((id, pitch), RecNote { scene: slot as usize, start: pos, vel, arr: None });
        }
    }

    /// Plays through another output device (None: the system default), at
    /// once, and remembers it. The engine moves along as it is when the
    /// device can run at its sample rate; else a new one takes over at the
    /// device's rate, with the set, the transport, plugins and input.
    pub(crate) fn switch_audio_output(&mut self, name: Option<String>) {
        self.prefs.audio_output = name.clone();
        self.prefs.save();
        if std::env::var_os("KRABTON_NO_AUDIO").is_some() {
            self.toast("Audio is off here (KRABTON_NO_AUDIO)");
            return;
        }
        let rate = self.engine.status.sample_rate.load().round() as u32;
        // The old output stops first; the engine waits in its slot.
        self._audio = AudioOutput::Off;
        let (out, new_rate) = match audio::open_output(name.as_deref(), &self.engine_slot, Some(rate)) {
            Ok(ok) => ok,
            Err(e) => {
                self.toast(format!("No audio output: {e}"));
                (audio::start_null(self.engine_slot.clone(), rate), rate)
            }
        };
        if new_rate != rate {
            self.rebuild_engine(new_rate);
        }
        self.audio_desc = out.description();
        self._audio = out;
        let wanted = name.as_deref().is_none_or(|n| self.audio_desc.starts_with(n));
        self.toast(if wanted { format!("Audio output: {}", self.audio_desc) } else { format!("Could not open it; playing to {}", self.audio_desc) });
    }

    /// Records from another input device (None: the system default),
    /// reopened as soon as a track listens to it.
    pub(crate) fn switch_audio_input(&mut self, name: Option<String>) {
        self.prefs.audio_input = name;
        self.prefs.save();
        let was_open = self.input_dev.take().is_some();
        self.input_error = None;
        self.cmd(Command::SetInput(None));
        if !was_open {
            self.toast("Audio input saved: it opens when a track listens to its input");
        }
    }

    /// The engine failed while playing (the audio output took it out): a new
    /// one takes over where it was, at most once every ten seconds; if it
    /// fails again that soon it stays silent until asked.
    fn recover_engine(&mut self) {
        if !crate::audio::engine_failed() {
            return;
        }
        let now = std::time::Instant::now();
        let log = crate::prefs::Prefs::crash_log().map(|p| p.display().to_string()).unwrap_or_else(|| "crash.log".into());
        if self.engine_failed_at.is_some_and(|t| now.duration_since(t).as_secs_f32() < 10.0) {
            self.toast(format!("The sound engine failed again; it stays off. Save your set and restart Krabton (what happened is in {log})"));
            return;
        }
        self.engine_failed_at = Some(now);
        let rate = self.engine.status.sample_rate.load().round() as u32;
        self.rebuild_engine(rate.max(8000));
        self.toast(format!("The sound engine hit an error and was restarted (what happened is in {log})"));
    }

    /// A new engine at `rate` in place of the running one: the set, the
    /// transport, plugins (from their current state) and the input go over.
    fn rebuild_engine(&mut self, rate: u32) {
        let playing = self.engine.status.playing.load(Ordering::Relaxed);
        let pos = self.engine.status.song_pos.load();
        // The session clips that play, to launch again.
        let launched: Vec<(u64, usize)> = self
            .project
            .tracks
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                let s = self.engine.status.tracks.get(i)?.playing_slot.load(Ordering::Relaxed);
                (s >= 0).then_some((t.id, s as usize))
            })
            .collect();
        self.plugins.store_states(&mut self.project);
        let snapshot = Arc::new(self.project.clone());
        let (eng, handle) = Engine::new(rate as f32, snapshot.clone());
        let old = self.engine_slot.lock().ok().and_then(|mut g| g.replace(eng));
        // The old engine (and the plugins' processors in it) goes here, off the audio thread.
        drop(old);
        self.midi_outs.attach(handle.midi_out.clone(), handle.epoch);
        self.engine = handle;
        self.last_sent = snapshot;
        self.plugins.forget();
        self.sync_plugins();
        self.input_dev = None;
        self.input_error = None;
        if playing {
            self.cmd(Command::SetPosition(pos));
            self.cmd(Command::Play);
        }
        for (track, slot) in launched {
            self.cmd(Command::LaunchClip { track, slot });
        }
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

    /// Opens or closes the editor of an effect plug-in.
    pub fn toggle_effect_editor(&mut self, instance: u64, title: &str) {
        if let Err(e) = self.plugins.toggle_editor(instance, &format!("{title} - Krabton")) {
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
        for &id in ids {
            self.cmd(Command::NoteOff { track: id, pitch });
            self.finish_rec_note(id, pitch, None);
            self.touch_played_off(id, pitch);
        }
    }

    /// Writes a recorded note into its clip once it ends: at `end` (the
    /// position in the clip) or where the clip is now.
    pub(crate) fn finish_rec_note(&mut self, id: u64, pitch: u8, end: Option<f64>) {
        let status = self.status();
        {
            let Some(rn) = self.rec_notes.remove(&(id, pitch)) else { return };
            let Some(ti) = self.project.track_index(id) else { return };
            if let Some(aid) = rn.arr {
                // Into the arrangement clip, which grows to hold it.
                let song = status.song_pos.load();
                let quant = self.rec_quantize;
                let Some(a) = self.project.tracks[ti].arrangement.iter_mut().find(|a| a.id == aid) else { return };
                let now = end.unwrap_or(song - a.start);
                let length = (now - rn.start).max(1.0 / 32.0);
                let start = if quant { (rn.start * 4.0).round() / 4.0 } else { rn.start };
                if start + length > a.length {
                    a.length = start + length;
                    a.clip.length = a.length;
                }
                if let Some(notes) = a.clip.notes_mut() {
                    notes.push(Note::new(pitch, start, length, rn.vel));
                }
                return;
            }
            let ts = &status.tracks[ti];
            let now = match end {
                Some(e) => e,
                None if ts.playing_slot.load(Ordering::Relaxed) == rn.scene as i32 => ts.clip_pos.load(),
                None => rn.start + 0.25,
            };
            let quant = self.rec_quantize;
            let Some(Some(clip)) = self.project.tracks[ti].slots.get_mut(rn.scene) else { return };
            let len = clip.length.max(0.0625);
            let mut length = now - rn.start;
            // Before its start: it was held across the loop's end. A tap
            // that ended before the position moved stays short.
            if length < -1e-6 {
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

    /// Removes notes hidden under others of their pitch and shortens the
    /// ones that run on into the next (see `tidy_notes`), keeping the piano
    /// roll's selection on the same notes.
    fn tidy_clip_notes(&mut self) {
        let sel = self.sel_clip;
        for (ti, t) in self.project.tracks.iter_mut().enumerate() {
            for (si, slot) in t.slots.iter_mut().enumerate() {
                let Some(map) = slot.as_mut().and_then(|c| c.notes_mut()).and_then(krabton_engine::project::tidy_notes) else { continue };
                if sel == Some(ClipSel::Slot { track: ti, scene: si }) {
                    self.piano.selected = self.piano.selected.iter().filter_map(|&i| map.get(i).copied().flatten()).collect();
                }
            }
            for a in t.arrangement.iter_mut() {
                let Some(map) = a.clip.notes_mut().and_then(krabton_engine::project::tidy_notes) else { continue };
                if sel == Some(ClipSel::Arr { track: ti, id: a.id }) {
                    self.piano.selected = self.piano.selected.iter().filter_map(|&i| map.get(i).copied().flatten()).collect();
                }
            }
        }
    }

    /// Notes armed tracks took from other tracks: recorded like played ones.
    pub fn poll_midi_recording(&mut self) {
        let playing = self.is_playing();
        while let Ok(m) = self.engine.midi_recorded.try_recv() {
            if !self.recording || !playing {
                continue;
            }
            // Into the arrangement clip the track records into, where the song is.
            if let Some(&(aid, _)) = self.arr_takes.get(&m.track) {
                let start = self.project.track_index(m.track).and_then(|ti| self.project.tracks[ti].arrangement.iter().find(|a| a.id == aid)).map(|a| a.start);
                if let Some(start) = start.filter(|&s| m.song >= s) {
                    if m.on {
                        self.rec_notes.insert((m.track, m.pitch), RecNote { scene: 0, start: m.song - start, vel: m.vel, arr: Some(aid) });
                    } else {
                        self.finish_rec_note(m.track, m.pitch, Some(m.song - start));
                    }
                }
                continue;
            }
            if m.slot < 0 {
                continue;
            }
            if m.on {
                self.rec_notes.insert((m.track, m.pitch), RecNote { scene: m.slot as usize, start: m.pos, vel: m.vel, arr: None });
            } else {
                self.finish_rec_note(m.track, m.pitch, Some(m.pos));
            }
        }
        self.grow_arr_takes();
    }

    /// The arrangement's MIDI takes grow with the song; when the loop jumps
    /// back (or the recording stops) a take ends at the beat.
    fn grow_arr_takes(&mut self) {
        if self.arr_takes.is_empty() {
            return;
        }
        let song = self.engine.status.song_pos.load();
        let mut done = Vec::new();
        for (&track, take) in self.arr_takes.iter_mut() {
            let Some(ti) = self.project.track_index(track) else {
                done.push(track);
                continue;
            };
            let Some(a) = self.project.tracks[ti].arrangement.iter_mut().find(|a| a.id == take.0) else {
                done.push(track);
                continue;
            };
            if song + 1e-6 < take.1 || !self.recording {
                done.push(track);
                continue;
            }
            take.1 = song;
            let len = (song - a.start).max(0.25);
            if len > a.length {
                a.length = len;
                a.clip.length = len;
            }
        }
        for track in done {
            self.end_arr_take(track);
        }
    }

    /// Ends the arrangement take of track `track`: held notes end now, and
    /// the clip ends on the next beat.
    fn end_arr_take(&mut self, track: u64) {
        let held: Vec<u8> = self.rec_notes.iter().filter(|((t, _), rn)| *t == track && rn.arr.is_some()).map(|((_, p), _)| *p).collect();
        for p in held {
            self.finish_rec_note(track, p, None);
        }
        if let Some((aid, _)) = self.arr_takes.remove(&track) {
            if let Some(a) = self.project.track_index(track).and_then(|ti| self.project.tracks[ti].arrangement.iter_mut().find(|a| a.id == aid)) {
                a.length = a.length.max(1.0).ceil();
                a.clip.length = a.length;
                if a.clip.name == "Recording" {
                    a.clip.name = "Take".into();
                }
            }
        }
    }

    /// The tracks a recording writes notes into: the ones that hear the
    /// keyboard, and armed tracks that take notes from another track.
    fn midi_record_targets(&self) -> Vec<usize> {
        let mut v = self.note_targets();
        for (ti, t) in self.project.tracks.iter().enumerate() {
            if t.arm && !v.contains(&ti) && self.project.midi_source(ti).is_some() {
                v.push(ti);
            }
        }
        v
    }

    pub fn toggle_record(&mut self) {
        if self.recording || !self.takes.is_empty() {
            self.recording = false;
            self.stop_takes();
            for track in self.arr_takes.keys().copied().collect::<Vec<_>>() {
                self.end_arr_take(track);
            }
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
        if self.view == View::Arrangement {
            // MIDI goes onto the arrangement: a clip from the playhead's bar that grows.
            let song = status.song_pos.load();
            let bpb = self.project.beats_per_bar_f();
            let start = (song / bpb).floor() * bpb;
            let mut n = 0;
            for ti in self.midi_record_targets() {
                if self.project.tracks[ti].kind != TrackKind::Midi {
                    continue;
                }
                let id = self.project.alloc_id();
                let cid = self.project.alloc_id();
                let t = &mut self.project.tracks[ti];
                let len = (song - start).max(0.25);
                t.arrangement.push(krabton_engine::ArrangementClip::new(id, start, len, Clip::new_midi(cid, "Recording", t.color, len)));
                self.arr_takes.insert(t.id, (id, song));
                n += 1;
            }
            if n > 0 {
                if !self.is_playing() {
                    self.cmd(Command::Play);
                }
                self.toast(format!("Recording {n} MIDI track{} on the arrangement", if n == 1 { "" } else { "s" }));
            }
            return;
        }
        for ti in self.midi_record_targets() {
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
                MidiMsg::Sustain { down, channel } => {
                    for ti in self.note_targets_from(Some(channel)) {
                        let track = self.project.tracks[ti].id;
                        self.cmd(Command::Sustain { track, down });
                    }
                }
                MidiMsg::PitchBend { value, channel } => {
                    // Two semitones each way, as most keyboards expect.
                    for ti in self.note_targets_from(Some(channel)) {
                        let track = self.project.tracks[ti].id;
                        self.cmd(Command::PitchBend { track, semis: value * 2.0 });
                    }
                }
                MidiMsg::ModWheel { value, channel } => {
                    for ti in self.note_targets_from(Some(channel)) {
                        let track = self.project.tracks[ti].id;
                        self.cmd(Command::ModWheel { track, value });
                    }
                }
            }
        }
    }

    /// Starts downloading an Orchestra instrument (unless it is here or on its way).
    pub(crate) fn start_orchestra_install(&mut self, instrument: usize) {
        let busy = self.orchestra_installs.get(&instrument).and_then(|s| s.lock().ok().map(|s| !matches!(*s, crate::orchestra_lib::State::Failed(_)))).unwrap_or(false);
        if !busy && krabton_engine::orchestra::library(instrument).is_none() && instrument < krabton_engine::orchestra::INSTRUMENTS.len() {
            self.orchestra_installs.insert(instrument, crate::orchestra_lib::start(instrument));
        }
    }

    /// An Orchestra instrument arrived: it plays.
    fn poll_orchestra_installs(&mut self) {
        let done: Vec<usize> = self.orchestra_installs.iter().filter(|(_, s)| s.lock().map(|s| *s == crate::orchestra_lib::State::Ready).unwrap_or(false)).map(|(k, _)| *k).collect();
        for i in done {
            self.orchestra_installs.remove(&i);
            let name = krabton_engine::orchestra::INSTRUMENTS[i].name;
            if crate::orchestra_lib::open_one(i) {
                self.toast(format!("{name} is here: it plays now"));
            } else {
                self.toast(format!("{name} was downloaded but could not be opened"));
            }
        }
    }

    /// Starts downloading and preparing the grand piano's samples.
    pub fn start_piano_install(&mut self) {
        let busy = self.piano_install.as_ref().and_then(|s| s.lock().ok().map(|s| !matches!(*s, crate::piano_lib::InstallState::Failed(_)))).unwrap_or(false);
        if !busy && krabton_engine::piano::library::shared().is_none() {
            self.piano_install = Some(crate::piano_lib::start_install());
        }
    }

    /// Follows the piano install; when it is done every Grand Piano gets the samples.
    /// Downloads the pack a kit comes from (unless it is here or on its way).
    pub(crate) fn start_drum_install(&mut self, kit: &str) {
        let Some(pack) = krabton_engine::dsp::real_drums::pack_of(kit) else { return };
        let busy = self.drum_installs.get(pack.folder).and_then(|s| s.lock().ok().map(|s| !matches!(*s, crate::drum_lib::State::Failed(_)))).unwrap_or(false);
        if !busy && !self.packs_here.contains(pack.folder) {
            self.drum_installs.insert(pack.folder, crate::drum_lib::start(pack));
        }
    }

    /// A pack arrived: the kits that use it load their recordings.
    fn poll_drum_install(&mut self) {
        let done: Vec<&'static str> = self.drum_installs.iter().filter(|(_, s)| s.lock().map(|s| *s == crate::drum_lib::State::Ready).unwrap_or(false)).map(|(k, _)| *k).collect();
        for folder in done {
            self.drum_installs.remove(folder);
            self.packs_here.insert(folder);
            for t in self.project.tracks.iter_mut() {
                if let Some(Instrument::Drums { kit, pads, .. }) = t.instrument.as_mut() {
                    if krabton_engine::dsp::real_drums::pack_of(kit).is_some_and(|p| p.folder == folder) {
                        *pads = krabton_engine::dsp::drums::kit(kit);
                    }
                }
            }
            self.toast(format!("{folder} is here: its kits play their recordings now"));
        }
    }

    /// Downloads an offered SoundFont bank (unless it is here or on its way).
    pub(crate) fn start_sf2_install(&mut self, i: usize) {
        let Some(offer) = crate::sf2_lib::OFFERS.get(i) else { return };
        let busy = self.sf2_installs.get(&i).and_then(|s| s.lock().ok().map(|s| matches!(*s, crate::sf2_lib::State::Downloading { .. }))).unwrap_or(false);
        if busy || crate::sf2_lib::installed(offer).is_some() {
            return;
        }
        self.sf2_installs.insert(i, crate::sf2_lib::start(i));
        self.toast(format!("Downloading {} ({} MB)...", offer.name, offer.bytes / 1_000_000));
    }

    /// What the SF2 Player's display offers to get, and how far each is.
    pub(crate) fn sf2_offers(&self) -> Vec<crate::ui::faces::sf2::OfferView> {
        use crate::ui::faces::sf2::{OfferState, OfferView};
        crate::sf2_lib::OFFERS
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let state = match self.sf2_installs.get(&i).and_then(|s| s.lock().ok().map(|s| s.clone())) {
                    Some(crate::sf2_lib::State::Downloading { bytes }) => OfferState::Downloading(bytes as f32 / o.bytes.max(1) as f32),
                    Some(crate::sf2_lib::State::Failed(_)) => OfferState::Failed,
                    _ if crate::sf2_lib::installed(o).is_some() => OfferState::Here,
                    _ => OfferState::Get,
                };
                OfferView { name: o.name, mb: o.bytes / 1_000_000, state }
            })
            .collect()
    }

    /// The bank a new SF2 Player starts on: the one MIDI files play with, a
    /// downloaded one, or one opened before.
    pub(crate) fn default_sf2(&mut self) -> Option<krabton_engine::dsp::sf2::Sf2Ref> {
        let mut paths: Vec<PathBuf> = self.prefs.gm_bank.iter().cloned().collect();
        paths.extend(crate::sf2_lib::all_installed());
        paths.extend(self.prefs.sf2_banks.iter().cloned());
        let path = paths.into_iter().find(|p| p.is_file())?;
        self.sf2_bank(&path)
    }

    /// A bank arrived: every SF2 Player without one plays it, and MIDI files
    /// play with it if they had no bank.
    fn poll_sf2_installs(&mut self) {
        let mut done = Vec::new();
        for (&i, s) in &self.sf2_installs {
            match s.lock().map(|s| s.clone()) {
                Ok(crate::sf2_lib::State::Ready(bank)) => done.push((i, Ok(bank))),
                Ok(crate::sf2_lib::State::Failed(e)) if !e.is_empty() => done.push((i, Err(e))),
                _ => {}
            }
        }
        for (i, result) in done {
            match result {
                Ok(bank) => {
                    self.sf2_installs.remove(&i);
                    let Some(path) = bank.path.clone() else { continue };
                    if self.sf2_open.len() >= 4 {
                        self.sf2_open.remove(0);
                    }
                    self.sf2_open.push(bank.clone());
                    if !self.prefs.sf2_banks.contains(&path) {
                        self.prefs.sf2_banks.push(path.clone());
                    }
                    let mut midi = "";
                    if self.prefs.gm_bank.as_ref().is_none_or(|p| !p.is_file()) && bank.is_gm() {
                        self.prefs.gm_bank = Some(path);
                        midi = "; MIDI files play with it";
                    }
                    self.prefs.save();
                    let first = bank.preset(0, 0).or_else(|| bank.presets.first()).map(|p| (p.bank, p.patch, p.name.clone()));
                    for t in self.project.tracks.iter_mut() {
                        if let Some(Instrument::Sf2 { font, bank: b, patch, preset, .. }) = t.instrument.as_mut() {
                            if font.as_ref().is_none_or(|f| f.is_missing()) {
                                if let Some((nb, np, name)) = &first {
                                    let named = t.name == "SF2 Player" || t.name == *preset;
                                    *font = Some(bank.clone());
                                    (*b, *patch, *preset) = (*nb, *np, name.clone());
                                    if named {
                                        t.name = name.clone();
                                    }
                                }
                            }
                        }
                    }
                    self.toast(format!("{} is here: {} presets{midi}", bank.name, bank.presets.len()));
                }
                // The display shows it failed (and offers it again); say why once.
                Err(e) => {
                    if let Some(s) = self.sf2_installs.get(&i) {
                        if let Ok(mut st) = s.lock() {
                            *st = crate::sf2_lib::State::Failed(String::new());
                        }
                    }
                    if !e.is_empty() {
                        self.toast(e);
                    }
                }
            }
        }
    }

    fn poll_piano_install(&mut self) {
        let Some(state) = self.piano_install.clone() else { return };
        let ready = state.lock().map(|s| *s == crate::piano_lib::InstallState::Ready).unwrap_or(false);
        if ready {
            self.piano_install = None;
            match crate::piano_lib::open_installed() {
                Some(lib) => {
                    krabton_engine::piano::library::set_shared(Some(lib.clone()));
                    self.cmd(Command::SetPianoLibrary(Some(lib)));
                    self.toast("The grand piano is installed");
                }
                None => self.toast("The grand piano was downloaded but could not be opened"),
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
                Key::L if m.alt => self.toggle_detail(),
                Key::H => self.show_history = !self.show_history,
                Key::Comma => self.show_settings = !self.show_settings,
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
                Key::D if self.view == View::Arrangement && !self.arr.picked.is_empty() => self.arr_duplicate_selection(),
                Key::D => self.edit_duplicate(),
                Key::E if self.view == View::Arrangement => self.arr_split_at_playhead(),
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
                Key::R if m.shift => self.view = if self.view == View::Studio { View::Session } else { View::Studio },
                Key::R => self.export.open = true,
                _ => {}
            }
            return;
        }
        // The Touch view: the letter rows press its plates (as the keys do in
        // key mode), the computer keyboard on or not.
        if self.view == View::Touch && m.is_none() {
            if let Some(&(_, step)) = SCALE_KEYMAP.iter().find(|(k, _)| *k == key) {
                if !self.held_keys.contains_key(&key) {
                    if let Some(played) = self.touch_key(step) {
                        self.held_keys.insert(key, played);
                    }
                }
                return;
            }
        }
        match key {
            Key::Escape if self.mod_learn.is_some() => self.mod_learn = None,
            Key::Space => self.toggle_play(),
            Key::Tab if m.shift => {
                self.detail = if self.detail == DetailTab::Clip { DetailTab::Devices } else { DetailTab::Clip };
            }
            Key::Tab => self.view = if self.view == View::Session { View::Arrangement } else { View::Session },
            Key::Delete | Key::Backspace if self.view == View::Arrangement && !self.arr.picked.is_empty() => self.arr_delete_selection(),
            Key::Delete | Key::Backspace if self.view == View::Arrangement && self.sel_clip.is_none() && self.arr.sel_section.is_some() => {
                if let Some(id) = self.arr.sel_section.take() {
                    self.project.delete_section(id, false);
                }
            }
            Key::Num0 if self.view == View::Arrangement && !self.arr_selection().is_empty() => self.arr_toggle_off(),
            Key::Home if self.view == View::Arrangement => {
                self.cmd(Command::SetPosition(0.0));
                self.arr.scroll_x = 0.0;
            }
            Key::End if self.view == View::Arrangement => {
                let end = self.song_end();
                self.cmd(Command::SetPosition(end));
            }
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
        self.sel_more.clear();
        self.band = Default::default();
        self.tape_load();
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

    /// The new age ambient set: brook, birds, chimes, drone, bowl and pan flute.
    pub fn load_ambient(&mut self) {
        self.replace_project(krabton_engine::demo::ambient_project());
        self.file_path = None;
        self.toast("Forest Morning: press Space or launch a scene");
    }

    pub fn open(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("Krabton set", &[FILE_EXTENSION]).pick_file() else { return };
        self.open_path(path);
    }

    /// Opens a set from disk (the Open dialog, or a file double-clicked in
    /// the file manager).
    pub fn open_path(&mut self, path: std::path::PathBuf) {
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
        if let Some(dir) = path.parent() {
            let name = self.project.name.clone();
            self.tape_save(dir, &name);
        }
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

    /// Saves an audio clip on its own, as it plays (its loop, offset and
    /// gain, no effects), as WAV or MP3.
    pub fn export_clip(&mut self, sel: ClipSel, mp3: bool) {
        let (loop_len, offset, length) = match sel {
            ClipSel::Slot { .. } => {
                let Some(c) = self.clip_ref(sel) else { return };
                (c.length, 0.0, c.length)
            }
            ClipSel::Arr { track, id } => {
                let Some(ac) = self.project.tracks.get(track).and_then(|t| t.arrangement.iter().find(|a| a.id == id)) else { return };
                (ac.clip.length, ac.offset, ac.length)
            }
        };
        let Some(clip) = self.clip_ref(sel) else { return };
        let ClipContent::Audio(a) = &clip.content else { return };
        let ext = if mp3 { "mp3" } else { "wav" };
        let name = format!("{}.{ext}", clip.name);
        let (sr, audio) = render::clip_audio(a, loop_len, offset, length);
        let filter = if mp3 { "MP3 audio" } else { "WAV audio" };
        let Some(path) = rfd::FileDialog::new().add_filter(filter, &[ext]).set_file_name(name).save_file() else { return };
        let result = if mp3 { crate::mp3::encode(&path, sr, &audio) } else { render::write_wav(&path, sr, &audio) };
        match result {
            Ok(()) => self.toast(format!("Saved {}", path.display())),
            Err(e) => self.toast(format!("Export failed: {e}")),
        }
    }

    pub fn start_export(&mut self) {
        if self.export_job.is_some() {
            return;
        }
        let name = format!("{}.wav", self.project.name);
        let Some(path) = rfd::FileDialog::new().add_filter("WAV audio", &["wav"]).add_filter("MP3 audio", &["mp3"]).set_file_name(name).save_file() else { return };
        let mp3 = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp3"));
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
                Some(buf) if mp3 => crate::mp3::encode(&path, sr, &buf).map(|_| path.display().to_string()),
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

    /// Whether the detail panel (clip editor, devices) is out: the Rack has
    /// its own setting.
    pub(crate) fn detail_shown(&self) -> bool {
        if self.view == View::Studio {
            self.prefs.layout.rack_detail
        } else {
            !self.prefs.layout.detail_hidden
        }
    }

    pub(crate) fn toggle_detail(&mut self) {
        if self.view == View::Studio {
            self.prefs.layout.rack_detail = !self.prefs.layout.rack_detail;
        } else {
            self.prefs.layout.detail_hidden = !self.prefs.layout.detail_hidden;
        }
        self.layout_dirty = true;
    }

    /// The `shot` script action: asks egui for the window's picture and
    /// saves it when it arrives.
    fn take_shot(&mut self, ctx: &Context) {
        let Some((path, asked)) = self.shot.clone() else { return };
        if !asked {
            // A minimized window has nothing to read back (and egui_glow panics on it).
            let gone = ctx.input(|i| i.viewport().minimized == Some(true) || i.screen_rect().width() < 1.0 || i.viewport().inner_rect.is_some_and(|r| r.width() < 1.0 || r.height() < 1.0));
            if gone {
                return;
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.shot = Some((path, true));
            return;
        }
        let image = ctx.input(|i| i.events.iter().find_map(|e| if let egui::Event::Screenshot { image, .. } = e { Some(image.clone()) } else { None }));
        if let Some(image) = image {
            if let Err(e) = crate::shot::save(&path, &image) {
                eprintln!("shot: {e}");
            }
            self.shot = None;
        }
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
                // chain=TRACK: prints its instrument and effects (to check by script).
                "chain" => {
                    if let Some(t) = self.project.tracks.get(nums.first().copied().unwrap_or(0)) {
                        let fx: Vec<&str> = t.devices.iter().map(|d| d.kind.name()).collect();
                        println!("chain {}: {} | {}", t.name, t.instrument.as_ref().map_or("-", |i| i.kind_name()), fx.join(", "));
                    }
                }
                // The arrangement's edits, to check by script: the playhead at
                // a beat, pick a track's clips, split, switch off, duplicate.
                "pos" => self.cmd(Command::SetPosition(arg.trim().parse::<f64>().unwrap_or(0.0))),
                "arrpick" => {
                    let t = nums.first().copied().unwrap_or(0);
                    if let Some(tr) = self.project.tracks.get(t) {
                        self.arr.picked = tr.arrangement.iter().map(|a| a.id).collect();
                        self.sel_clip = tr.arrangement.first().map(|a| ClipSel::Arr { track: t, id: a.id });
                    }
                }
                "arrsplit" => self.arr_split_at_playhead(),
                // sf2=PATH: a new track with the bank's first preset; sf2pick=BANK,PROGRAM on
                // the selected track; gmbank=PATH: MIDI files play with that bank.
                "sf2" => {
                    let path = PathBuf::from(arg.trim());
                    if let Some(f) = self.sf2_bank(&path) {
                        let (bank, patch) = f.presets.first().map(|p| (p.bank, p.patch)).unwrap_or((0, 0));
                        self.add_midi_track(Some(Instrument::sf2(Some(f), bank, patch)));
                    }
                }
                "sf2pick" if nums.len() == 2 => {
                    if let TrackSel::Track(t) = self.sel_track {
                        if let Some(Instrument::Sf2 { font, bank, patch, preset, .. }) = self.project.tracks.get_mut(t).and_then(|tr| tr.instrument.as_mut()) {
                            *bank = nums[0] as u16;
                            *patch = nums[1] as u8;
                            *preset = font.as_ref().and_then(|f| f.preset(*bank, *patch)).map(|p| p.name.clone()).unwrap_or_default();
                        }
                    }
                }
                "gmbank" => self.prefs.gm_bank = Some(PathBuf::from(arg.trim())),
                // sf2get=N: download offered bank N; sf2empty: a new SF2 Player as from the
                // browser (on a bank when there is one); sf2none: one without a bank.
                "sf2get" => self.start_sf2_install(nums.first().copied().unwrap_or(0)),
                "sf2empty" => self.apply_item(&BrowserItem::Sf2Player, DropTarget::NewTrack),
                // singer: a Singer on the selected track (on the last voicebank); voicebank=PATH: that voicebank there;
                // lyrics=TEXT: on the selected clip's notes.
                // hyper: Hyper on the selected track (on the last firmware); firmware=PATH: that firmware there.
                "hyper" | "firmware" => {
                    let target = match self.sel_track {
                        TrackSel::Track(t) => DropTarget::Track(t),
                        _ => DropTarget::NewTrack,
                    };
                    let item = if name == "hyper" { BrowserItem::Hyper } else { BrowserItem::HyperFirmware(PathBuf::from(arg.trim())) };
                    self.apply_item(&item, target);
                }
                "singer" => {
                    let target = match self.sel_track {
                        TrackSel::Track(t) => DropTarget::Track(t),
                        _ => DropTarget::NewTrack,
                    };
                    self.apply_item(&BrowserItem::Singer, target);
                }
                "voicebank" => {
                    let path = PathBuf::from(arg.trim());
                    let target = match self.sel_track {
                        TrackSel::Track(t) => DropTarget::Track(t),
                        _ => DropTarget::NewTrack,
                    };
                    self.apply_item(&BrowserItem::Voicebank(path), target);
                }
                // singerown: the selected track's Singer sings with its own voice (as its OWN VOICE button).
                "singerown" => {
                    if let TrackSel::Track(t) = self.sel_track {
                        if let Some(Instrument::Singer { bank, .. }) = self.project.tracks.get_mut(t).and_then(|tr| tr.instrument.as_mut()) {
                            *bank = None;
                        }
                    }
                }
                // voiceget=N: downloads offered voicebank N for the selected track's Singer.
                "voiceget" => {
                    let asking = match self.sel_track {
                        TrackSel::Track(t) => self.project.tracks.get(t).map(|t| t.id),
                        _ => None,
                    };
                    self.start_singer_install(nums.first().copied().unwrap_or(0), asking);
                }
                // slot=TRACK,SCENE (or slot=SCENE on the selected track): that clip selected, in the clip view.
                "slot" => {
                    let at = match (nums.as_slice(), self.sel_track) {
                        ([t, s], _) => Some((*t, *s)),
                        ([s], TrackSel::Track(t)) => Some((t, *s)),
                        _ => None,
                    };
                    if let Some((track, scene)) = at {
                        self.sel_clip = Some(ClipSel::Slot { track, scene });
                        self.sel_track = TrackSel::Track(track);
                        self.detail = DetailTab::Clip;
                    }
                }
                "lyrics" => {
                    if let Some(sel) = self.sel_clip {
                        self.put_lyrics(sel, arg);
                    }
                }
                "sf2none" => {
                    self.add_midi_track(Some(Instrument::sf2(None, 0, 0)));
                }
                // midifile=PATH: a MIDI file as if double-clicked in the browser.
                "midifile" => self.apply_item(&BrowserItem::MidiFile(PathBuf::from(arg.trim())), DropTarget::NewTrack),
                // toclip=DEVICE,BARS: the selected track's MIDI device's To Clip.
                "toclip" if nums.len() == 2 => {
                    if let TrackSel::Track(t) = self.sel_track {
                        if let Some(id) = self.project.tracks.get(t).and_then(|tr| tr.midi_devices.get(nums[0])).map(|d| d.id) {
                            self.sequencer_to_clip(t, id, nums[1] as u32);
                        }
                    }
                }
                "arroff" => self.arr_toggle_off(),
                "arrdup" => self.arr_duplicate_selection(),
                // keepsound=TRACK,SCENE: that clip keeps the track's sound.
                "keepsound" if nums.len() == 2 => self.keep_sound_in_clip(ClipSel::Slot { track: nums[0], scene: nums[1] }),
                "play" => self.cmd(Command::Play),
                "ambient" => self.load_ambient(),
                "stop" => self.cmd(Command::Stop),
                "arrangement" => self.view = View::Arrangement,
                // automation[=TRACK]: Automation mode, and on TRACK a volume line and a pan lane to see.
                "automation" => {
                    self.view = View::Arrangement;
                    self.arr.automation = true;
                    if let Some(&ti) = nums.first() {
                        if let Some(tr) = self.project.tracks.get_mut(ti) {
                            let id = tr.id;
                            use krabton_engine::modulation::ModTarget;
                            tr.auto_shown = Some(ModTarget::Volume(id));
                            tr.automation_mut(ModTarget::Volume(id)).points = vec![(0.0, 0.8), (4.0, 0.8), (8.0, 0.3), (12.0, 0.6), (16.0, 0.85)];
                            tr.automation_mut(ModTarget::Pan(id)).points = vec![(0.0, 0.5), (2.0, 0.1), (6.0, 0.9), (10.0, 0.5)];
                            tr.auto_lanes = vec![ModTarget::Pan(id)];
                        }
                    }
                }
                "session" => self.view = View::Session,
                "rackview" | "studio" => self.view = View::Studio,
                "tapeview" => self.view = View::Tape,
                "touchview" => self.view = View::Touch,
                // touchparam=K,V: the selected track's Touch parameter K set to V.
                "touchparam" => {
                    let f: Vec<f32> = arg.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                    if let (Some(ti), [k, v, ..]) = (self.touch_track(), f.as_slice()) {
                        let di = self.touch_device(ti);
                        if let Some(p) = self.project.tracks[ti].midi_devices[di].params.get_mut(*k as usize) {
                            *p = *v;
                        }
                    }
                }
                // touchpreset=N: Touch preset N on the selected track's Touch.
                "touchpreset" => {
                    if let Some(ti) = self.touch_track() {
                        let di = self.touch_device(ti);
                        self.touch_preset(ti, di, nums.first().copied().unwrap_or(0));
                    }
                }
                "detail" => self.toggle_detail(),
                "browser" => self.browser.visible = !self.browser.visible,
                // plain[=0]: devices in the flat look (or, with 0, each in its own).
                "plain" => self.prefs.plain_devices = nums.first() != Some(&0),
                // sliders[=0]: the Touch view and Coast with sliders (or, with 0, knobs).
                "sliders" => self.prefs.knobs = nums.first() == Some(&0),
                "envelopes" => {
                    self.env_view = true;
                    self.detail = DetailTab::Clip;
                    // envelopes=1: and a volume line on the selected clip, to see.
                    if nums.first() == Some(&1) {
                        if let Some(sel) = self.sel_clip {
                            let t = sel.track();
                            if let Some(id) = self.project.tracks.get(t).map(|tr| tr.id) {
                                let target = krabton_engine::modulation::ModTarget::Volume(id);
                                if let Some(c) = self.clip_mut(sel) {
                                    c.envelopes.retain(|e| e.target != target);
                                    c.envelopes.push(krabton_engine::project::Envelope { target, points: vec![(0.0, 0.85), (1.0, 0.3), (2.0, 0.9), (3.0, 0.5)] });
                                }
                                self.env_target = Some(target);
                            }
                        }
                    }
                }
                "autoarm" => self.auto_arm = !self.auto_arm,
                "zoom" => self.script_zoom = nums.first().map(|&z| (z as f32 / 100.0).clamp(0.5, 3.0)),
                "glitchwin" => self.glitch_window = self.project.all_devices().find(|d| d.kind == krabton_engine::dsp::effects::EffectKind::Glitch).map(|d| d.id),
                "rackcolumns" => self.prefs.layout.rack_columns = !self.prefs.layout.rack_columns,
                "devices" => self.detail = DetailTab::Devices,
                "settings" => self.show_settings = true,
                "mapwin" => self.map_window = true,
                "bandwin" => self.band.window = true,
                // bandon[=1]: Band on in every device that can be in it (=1: and parts).
                "bandon" => {
                    for dev in self.project.tracks.iter_mut().flat_map(|t| t.midi_devices.iter_mut()) {
                        let b = krabton_engine::sequencers::players::band_param(dev.kind).or(krabton_engine::band::follow_param(dev.kind));
                        if let Some(v) = b.and_then(|b| dev.params.get_mut(b)) {
                            *v = 1.0;
                        }
                    }
                    if nums.first() == Some(&1) && self.project.band.parts.is_empty() {
                        self.make_band_parts();
                    }
                }
                "mapall" => {
                    for i in 0..self.project.tracks.len() {
                        self.set_on_map(i, true);
                    }
                }
                "mapspread" => self.spread_map(),
                "mapsel" => self.map_selected = self.project.tracks.get(nums.first().copied().unwrap_or(0)).map(|t| t.id),
                "audioout" => {
                    let names = audio::output_device_names();
                    eprintln!("outputs: {names:?}");
                    let pick = nums.first().and_then(|&i| names.get(i).cloned());
                    self.switch_audio_output(pick);
                    eprintln!("now: {}", self.audio_desc);
                }
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
                "piano" => {
                    let names = krabton_engine::piano::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Piano(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "pianoinstall" => self.start_piano_install(),
                "druminstall" => {
                    let kit = arg.to_string();
                    self.start_drum_install(if kit.is_empty() { "Real Kit" } else { &kit });
                }
                "mono101" | "mono" => {
                    let names = krabton_engine::dsp::mono101::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Mono101(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "bassline" => {
                    let names = krabton_engine::dsp::bassline::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Bassline(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "drone" => {
                    let names = krabton_engine::dsp::drone::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Drone(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "waves" => {
                    let names = krabton_engine::dsp::waves::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Waves(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "windchimes" => {
                    let names = krabton_engine::dsp::wind_chimes::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::WindChimes(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "birds" => {
                    let names = krabton_engine::dsp::birds::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Birds(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "nature" => {
                    let names = krabton_engine::dsp::nature::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Nature(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "beads" => {
                    let names = krabton_engine::dsp::beads_synth::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::BeadsSynth(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "panflute" => {
                    let names = krabton_engine::dsp::pan_flute::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::PanFlute(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "streetorgan" => {
                    let names = krabton_engine::dsp::street_organ::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::StreetOrgan(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "melodica" => {
                    let names = krabton_engine::dsp::melodica::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Melodica(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "harmonica" => {
                    let names = krabton_engine::dsp::harmonica::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Harmonica(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "calculator" => {
                    let names = krabton_engine::dsp::calculator::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Calculator(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "console" => {
                    let names = krabton_engine::dsp::console::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Console(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "handheld" => {
                    let names = krabton_engine::dsp::handheld::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Handheld(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "micro" => {
                    let names = krabton_engine::dsp::micro::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Micro(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "warp" => {
                    let names = krabton_engine::dsp::warp::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Warp(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "hybrid" => {
                    let names = krabton_engine::dsp::hybrid::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Hybrid(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "quartet" => {
                    let names = krabton_engine::dsp::quartet::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Quartet(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "orchestra" => {
                    let names = krabton_engine::orchestra::NAMES;
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Orchestra(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "oidos" => {
                    let names = krabton_engine::dsp::oidos::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Oidos(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "v2" => {
                    let names = krabton_engine::dsp::v2::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::V2(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "chipfm" => {
                    let names = krabton_engine::dsp::chipfm::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::ChipFm(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "coast" => {
                    let names = krabton_engine::dsp::coast::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Coast(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "crimson" => {
                    let names = krabton_engine::dsp::crimson::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Crimson(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "silicon" => {
                    let names = krabton_engine::dsp::silicon::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Silicon(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "theremin" => {
                    let names = krabton_engine::dsp::theremin::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Theremin(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "facet" => {
                    let names = krabton_engine::dsp::facet::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Facet(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "granulator" => {
                    let names = krabton_engine::dsp::granulator::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Granulator(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "guitar" => {
                    let names = krabton_engine::dsp::guitar::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Guitar(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "monk" => {
                    let names = krabton_engine::dsp::monk::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Monk(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "rings" => {
                    let names = krabton_engine::dsp::rings_synth::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Rings(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "organ" => {
                    let names = krabton_engine::dsp::organ::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Organ(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "winds" => {
                    let names = krabton_engine::dsp::stk_winds::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Winds(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "strings" => {
                    let names = krabton_engine::dsp::stk_strings::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Strings(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "mallets" => {
                    let names = krabton_engine::dsp::stk_mallets::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Mallets(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "voice" => {
                    let names = krabton_engine::dsp::stk_voice::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Voice(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "modular" => {
                    let names = krabton_engine::modular::presets::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Modular(p), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "rack" => {
                    // Opens the rack of the selected track.
                    if let TrackSel::Track(i) = self.sel_track {
                        self.rack.open = self.project.tracks.get(i).map(|t| t.id);
                    }
                }
                "drums" => {
                    let kits = krabton_engine::dsp::drums::kit_names();
                    let kit = kits.iter().copied().find(|k| *k == arg).unwrap_or(kits[0]);
                    self.apply_item(&BrowserItem::Drums(kit), DropTarget::NewTrack);
                    self.detail = DetailTab::Devices;
                }
                "shakers" => {
                    let names = krabton_engine::dsp::shaker_synth::preset_names();
                    let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                    self.apply_item(&BrowserItem::Shakers(p), DropTarget::NewTrack);
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
                // touchdown=x1,y1,x2,y2...: fingers on a touch screen there (pressing at 60%); touchup: they let go.
                // midifxmenu=N: opens the menu of sequencers and MIDI effects of track N.
                "midifxmenu" => self.script_menu = nums.first().copied(),
                // touchcapture: the Touch view's Capture, on the selected track.
                "touchcapture" => {
                    if let Some(ti) = self.touch_track() {
                        self.touch_capture(ti);
                    }
                }
                "touchdown" | "touchmove" | "touchup" => {
                    let f: Vec<f32> = arg.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                    let phase = match name {
                        "touchdown" => egui::TouchPhase::Start,
                        "touchmove" => egui::TouchPhase::Move,
                        _ => egui::TouchPhase::End,
                    };
                    let fingers: Vec<egui::Pos2> = if f.len() >= 2 { f.chunks_exact(2).map(|c| egui::pos2(c[0], c[1])).collect() } else { vec![egui::Pos2::ZERO; 4] };
                    let evs = fingers
                        .iter()
                        .enumerate()
                        .map(|(k, &pos)| egui::Event::Touch { device_id: egui::TouchDeviceId(1), id: egui::TouchId(k as u64), phase, pos, force: Some(0.6) })
                        .collect();
                    self.script_input.push_back(evs);
                }
                "modtab" => self.detail = DetailTab::Mod,
                "mod" => {
                    let all = krabton_engine::modulation::ModKind::ALL;
                    let k = all[nums.first().copied().unwrap_or(0).min(all.len() - 1)];
                    self.add_modulator(k);
                }
                "map" => self.mod_learn = self.project.modulators.last().map(|m| m.id),
                "selmod" => self.sel_mod = nums.first().and_then(|&i| self.project.modulators.get(i)).map(|m| m.id),
                "routeinst" | "routedev" | "routemidi" | "routevol" | "routemod" | "routemap" => {
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
                        ("routemap", _) => self.project.tracks.get(n(0)).map(|t| ModTarget::Param { owner: ModOwner::Map(t.id), param: n(1) as u16 }),
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
                    // By its number in the list, or its name.
                    let k = MidiFxKind::SEQUENCERS
                        .into_iter()
                        .find(|k| k.name().eq_ignore_ascii_case(arg.trim()))
                        .unwrap_or(MidiFxKind::SEQUENCERS[nums.first().copied().unwrap_or(0).min(MidiFxKind::SEQUENCERS.len() - 1)]);
                    let target = match (name, self.sel_track) {
                        ("seq", TrackSel::Track(t)) => DropTarget::Track(t),
                        _ => DropTarget::NewTrack,
                    };
                    self.apply_item(&BrowserItem::MidiFx(k), target);
                }
                "clipview" => self.detail = DetailTab::Clip,
                "track" => self.sel_track = TrackSel::Track(nums.first().copied().unwrap_or(0)),
                "master" => self.sel_track = TrackSel::Master,
                "transposeopen" => self.transpose_open = true,
                // keyseq=D1,D2,..: a key sequence of degrees of the song's key, running.
                "keyseq" => {
                    let song = self.project.scale;
                    self.project.key_seq = nums.iter().map(|&d| if d == 0 { None } else { Some(song.degree_key(d)) }).collect();
                    self.project.key_seq_on = true;
                }
                // playkey=ROOT[,KIND]: the song plays in that key (KIND an index into the scales).
                "playkey" => {
                    let root = nums.first().copied().unwrap_or(0) as u8 % 12;
                    let kind = nums.get(1).and_then(|&k| krabton_engine::ScaleKind::ALL.get(k).copied()).unwrap_or(self.project.scale.kind);
                    self.project.play_key = Some(krabton_engine::Scale::new(root, kind));
                }
                // selmore=N: track N joins the selection (as Ctrl+click).
                "selmore" => self.click_track(nums.first().copied().unwrap_or(0), true),
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
                "shot" => self.shot = Some((arg.to_string(), false)),
                "fxparam" => {
                    // fxparam=device,param,value sets a parameter of a device in the selected chain.
                    let f: Vec<f32> = arg.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                    if f.len() == 3 {
                        if let Some(v) = self.chain_mut(self.sel_track).and_then(|c| c.get_mut(f[0] as usize)).and_then(|d| d.params.get_mut(f[1] as usize)) {
                            *v = f[2];
                        }
                    }
                }
                "midiparam" => {
                    // midiparam=device,param,value sets a parameter of a MIDI device of the selected track.
                    let f: Vec<f32> = arg.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                    if let (3, TrackSel::Track(t)) = (f.len(), self.sel_track) {
                        if let Some(v) = self.project.tracks.get_mut(t).and_then(|t| t.midi_devices.get_mut(f[0] as usize)).and_then(|d| d.params.get_mut(f[1] as usize)) {
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
                "big" => {
                    if self.big.on {
                        self.leave_big_picture();
                    } else {
                        self.enter_big_picture();
                    }
                }
                "bigstick" if nums.len() == 2 || arg.contains(',') => {
                    let f: Vec<f32> = arg.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                    self.big.script_stick = (f.len() == 2 && (f[0] != 0.0 || f[1] != 0.0)).then(|| (f[0], f[1]));
                }
                "biglstick" => {
                    let f: Vec<f32> = arg.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                    self.big.script_lstick = (f.len() == 2 && (f[0] != 0.0 || f[1] != 0.0)).then(|| (f[0], f[1]));
                }
                "bigtrig" => {
                    let v: f32 = arg.trim().parse().unwrap_or(0.0);
                    self.big.script_trig = (v != 0.0).then_some(v);
                }
                "bigpage" => self.big.page = crate::big::Page::ALL[nums.first().copied().unwrap_or(0).min(crate::big::Page::ALL.len() - 1)],
                "bignav" => {
                    // bignav=down,down,a: one press a frame.
                    use crate::big::input::Nav;
                    for w in arg.split(',') {
                        let n = match w.trim() {
                            "up" => Nav::Up,
                            "down" => Nav::Down,
                            "left" => Nav::Left,
                            "right" => Nav::Right,
                            "a" => Nav::A,
                            "b" => Nav::B,
                            "x" => Nav::X,
                            "y" => Nav::Y,
                            "lb" => Nav::PrevPage,
                            "rb" => Nav::NextPage,
                            "start" => Nav::Start,
                            "select" | "menu" => Nav::Select,
                            "rec" => Nav::Record,
                            "l3" => Nav::LeftStick,
                            _ => continue,
                        };
                        self.big.script.push_back(n);
                    }
                }
                "plugin" => {
                    let n = nums.first().copied().unwrap_or(0);
                    let info = self.plugins.instruments().nth(n).cloned();
                    if let Some(info) = info {
                        self.apply_item(&BrowserItem::Plugin(info), DropTarget::NewTrack);
                    } else {
                        eprintln!("no plugin #{n}");
                    }
                }
                "pluginfx" => {
                    // An effect plug-in on the selected track's chain.
                    let n = nums.first().copied().unwrap_or(0);
                    let info = self.plugins.effects().nth(n).cloned();
                    match info {
                        Some(info) => {
                            let sel = self.sel_track;
                            self.apply_item(&BrowserItem::PluginEffect(info), DropTarget::Chain(sel));
                        }
                        None => eprintln!("no effect plugin #{n}"),
                    }
                }
                "savepath" => match {
                    // The tape goes next to the set, as with Save.
                    let path = std::path::Path::new(arg);
                    if let (Some(dir), Some(stem)) = (path.parent(), path.file_stem()) {
                        self.tape_save(dir, &stem.to_string_lossy());
                    }
                    self.project.save(path)
                } {
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
        crate::skin::set_enabled(!self.prefs.plain_devices);
        crate::skin::set_sliders(!self.prefs.knobs);
        self.engine.collect_garbage();
        self.sync_plugins();
        self.frame += 1;
        if self.frame == 40 && !self.script.is_empty() {
            // For scripts: how points map to the pixels of a `shot`.
            println!("ppp {} screen {:?}", ctx.pixels_per_point(), ctx.screen_rect());
        }
        self.run_script();
        if let Some(z) = self.script_zoom.take() {
            ctx.set_zoom_factor(z);
        }
        if let Some(t) = self.script_menu.take() {
            // The Devices tab's menu and the Studio's (+ MIDI).
            let id = self.project.tracks.get(t).map_or(0, |t| t.id);
            ctx.memory_mut(|m| {
                m.data.insert_temp(egui::Id::new(("midi_fx_menu", t)), true);
                m.data.insert_temp(egui::Id::new(("rack_midi_menu", id)), true);
            });
        }
        self.take_shot(ctx);
        self.record_automation();
        if self.frame % 30 == 0 {
            self.release_unused_displays();
        }
        self.poll_midi();
        self.poll_piano_install();
        self.poll_orchestra_installs();
        self.poll_drum_install();
        self.poll_sf2_installs();
        self.poll_singer_installs();
        self.poll_firmware_unpack();
        self.boot_hypers();
        self.recover_engine();
        self.poll_recording();
        self.tape_frame();
        self.poll_export();
        // Controllers (and in Big Picture the keyboard) speak through the same actions.
        // While a ring plays the left stick, it does not move the cursor.
        let page = self.big.page;
        // On the desktop the controller always plays (as on the Play page).
        // (on the desktop the sticks always glide: the left one plays too)
        let lstick_plays = !self.big.on || (page == crate::big::Page::Play && self.big.play.sticks != crate::big::play::StickMode::Pads) || page == crate::big::Page::Tape;
        self.big.pads.lstick_nav = !(lstick_plays && self.big.menu.is_none());
        let mut pad_input = self.big.pads.poll(ctx, self.big.on);
        if let Some(n) = self.big.script.pop_front() {
            pad_input.pressed.push(n);
        }
        if let Some(s) = self.big.script_stick {
            pad_input.stick = s;
            pad_input.stick_turn = s.1;
        }
        if let Some(s) = self.big.script_lstick {
            pad_input.lstick = s;
        }
        if let Some(t) = self.big.script_trig {
            pad_input.trig = t;
            pad_input.rt = t.max(0.0);
            pad_input.lt = (-t).max(0.0);
        }
        if !self.big.on {
            self.handle_keys(ctx);
            // Start on a controller opens Big Picture; otherwise it plays.
            if pad_input.has(crate::big::input::Nav::Start) {
                self.enter_big_picture();
            } else {
                self.desktop_pad_play(ctx, pad_input.clone());
            }
        }
        self.validate_selection();
        self.update_meters();
        self.grid_frame();
        self.mod_frame();
        if self.big.on {
            self.ui_big(ctx, pad_input);
            self.ui_dialogs(ctx);
            self.apply_deferred();
            self.sync(ctx);
            self.update_title(ctx);
            if self.quit_requested {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint();
            return;
        }

        egui::TopBottomPanel::top("transport")
            .exact_height(38.0)
            .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(8, 4)))
            .show(ctx, |ui| self.ui_transport(ui));
        egui::TopBottomPanel::bottom("statusbar")
            .exact_height(22.0)
            .frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin::symmetric(8, 2)))
            .show(ctx, |ui| self.ui_statusbar(ui));
        // The Transpose drawer slides up above it.
        egui::TopBottomPanel::bottom("transpose_drawer")
            .resizable(false)
            .frame(egui::Frame::new().fill(theme::PANEL2).inner_margin(egui::Margin::symmetric(12, 8)))
            .show_animated(ctx, self.transpose_open, |ui| self.ui_transpose_drawer(ui));
        // The browser runs from top to bottom, next to the detail view, as in Live.
        let fg = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("panel_grips")));
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
        if self.detail_shown() {
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
        } else {
            // Folded away: a thin strip to bring it back.
            egui::TopBottomPanel::bottom("detail_hidden")
                .resizable(false)
                .exact_height(20.0)
                .frame(egui::Frame::new().fill(theme::PANEL2))
                .show(ctx, |ui| {
                    let r = ui.max_rect();
                    let resp = ui.interact(r, ui.id().with("show_detail"), egui::Sense::click()).on_hover_text("Show the clip and device view (Ctrl+Alt+L)");
                    let hot = resp.hovered();
                    let p = ui.painter();
                    if hot {
                        p.rect_filled(r, 0.0, theme::PANEL3);
                    }
                    p.line_segment([r.left_top(), r.right_top()], egui::Stroke::new(1.0_f32, theme::LINE));
                    let col = if hot { theme::ACCENT } else { theme::TEXT_DIM };
                    // An arrow and the tabs' names in the middle, at the left too.
                    for c in [egui::pos2(r.left() + 14.0, r.center().y), egui::pos2(r.center().x - 92.0, r.center().y)] {
                        p.add(egui::Shape::convex_polygon(vec![c + egui::vec2(-6.0, 3.0), c + egui::vec2(0.0, -4.0), c + egui::vec2(6.0, 3.0)], col, egui::Stroke::NONE));
                    }
                    p.text(egui::pos2(r.center().x - 80.0, r.center().y), egui::Align2::LEFT_CENTER, "Clip · Devices · Mod · Grid", theme::small(), col);
                    if resp.clicked() {
                        self.toggle_detail();
                    }
                });
        }
        // Away from the Touch view, its plate lets go.
        if self.view != View::Touch && (self.touch_view_holds()) {
            self.touch_release_all();
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::BG)).show(ctx, |ui| match self.view {
            View::Session => self.ui_session(ui),
            View::Arrangement => self.ui_arrangement(ui),
            View::Studio => self.ui_device_rack(ui),
            View::Touch => self.ui_touch(ui),
            View::Tape => self.ui_tape(ui),
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

/// An instrument that fits a MIDI file's track: by its channel (10: drums),
/// its General MIDI program, or its name.
fn midi_file_instrument(t: &midi_file::MidiSongTrack) -> Instrument {
    let name = t.name.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| name.contains(w));
    if t.drums || has(&["drum", "kit", "perc"]) {
        return Instrument::drums("Studio Kit");
    }
    match t.program {
        Some(0..=7) => Instrument::synth("Keys"),
        Some(32..=39) => Instrument::guitar("Finger Bass"),
        Some(40..=51) => Instrument::strings("Bowed"),
        Some(88..=95) => Instrument::synth("Soft Pad"),
        _ if has(&["bass"]) => Instrument::guitar("Finger Bass"),
        _ if has(&["string", "violin", "viola", "cello"]) => Instrument::strings("Bowed"),
        _ if has(&["pad"]) => Instrument::synth("Soft Pad"),
        _ if has(&["piano", "keys", "organ"]) => Instrument::synth("Keys"),
        _ => Instrument::synth("Pluck"),
    }
}

/// A tempo as people write it: 120, 98.5, 133.33.
fn fmt_bpm(bpm: f64) -> String {
    let s = format!("{bpm:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A MIDI file's track on a General MIDI bank: on its program (or one its
/// name suggests), drums on the bank's first kit.
fn midi_file_gm(bank: &krabton_engine::dsp::sf2::Sf2Ref, t: &midi_file::MidiSongTrack) -> Instrument {
    let name = t.name.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| name.contains(w));
    let drums = t.drums || has(&["drum", "kit", "perc"]);
    let program = t.program.unwrap_or(if has(&["bass"]) {
        33
    } else if has(&["string", "violin", "viola", "cello"]) {
        48
    } else if has(&["pad"]) {
        89
    } else if has(&["organ"]) {
        16
    } else if has(&["guitar"]) {
        25
    } else if has(&["brass", "trumpet", "horn"]) {
        61
    } else if has(&["sax"]) {
        65
    } else if has(&["flute"]) {
        73
    } else if has(&["choir", "voice", "vox"]) {
        52
    } else if has(&["lead"]) {
        81
    } else {
        0
    });
    match bank.gm(program, drums).or_else(|| bank.presets.first()) {
        Some(p) => Instrument::sf2(Some(bank.clone()), p.bank, p.patch),
        None => Instrument::sf2(Some(bank.clone()), 0, 0),
    }
}
