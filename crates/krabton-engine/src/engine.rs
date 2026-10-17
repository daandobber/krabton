//! The real-time audio engine.
//!
//! The UI thread talks to the engine through a lock-free command channel and
//! reads its state back through atomics in [`EngineStatus`]. Project edits
//! arrive as whole immutable snapshots; old snapshots are handed back through
//! a garbage channel so that freeing them never happens on the audio thread.

use crate::dsp::drums::DrumState;
use crate::dsp::effects::{EffectState, FxCtx};
use crate::dsp::fm::FmState;
use crate::dsp::analog::AnalogState;
use crate::dsp::shaker_synth::ShakerSynthState;
use crate::modular::runtime::ModularState;
use crate::dsp::stk_voice::VoiceState;
use crate::dsp::stk_mallets::MalletsState;
use crate::dsp::stk_strings::StringsState;
use crate::dsp::stk_winds::WindsState;
use crate::dsp::organ::OrganState;
use crate::dsp::monk::MonkState;
use crate::dsp::bassline::BasslineState;
use crate::dsp::mono101::Mono101State;
use crate::dsp::drone::DroneState;
use crate::dsp::waves::WavesState;
use crate::dsp::guitar::GuitarState;
use crate::dsp::wind_chimes::WindChimesState;
use crate::dsp::beads_synth::BeadsSynthState;
use crate::dsp::birds::BirdsState;
use crate::dsp::nature::NatureState;
use crate::dsp::facet::FacetState;
use crate::dsp::pan_flute::PanFluteState;
use crate::dsp::melodica::MelodicaState;
use crate::dsp::harmonica::HarmonicaState;
use crate::piano::{library::PianoLibrary, PianoState};
use crate::dsp::rings_synth::RingsState;
use crate::dsp::braids_synth::BraidsSynthState;
use crate::dsp::elements_synth::ElementsSynthState;
use crate::dsp::macro_osc::MacroState;
use crate::dsp::sampler::SamplerState;
use crate::dsp::synth::SynthState;
use crate::audio_in::{InputRing, MAX_INPUTS};
use crate::dsp::pan_gains;
use crate::routing::{AudioInput, AudioOutput, MidiInput, Monitor, Tap};
use crate::midi_fx::{MidiCtx, MidiEvent, MidiFxState};
use crate::modulation::{ModClock, ModOwner, ModRuntime, TrackSignal, MAX_MODS, MOD_BLOCK};
use crate::plugin::{AudioProcessor, PluginEvent, PluginEventKind};
use crate::project::*;
use crate::sample::SampleRef;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Internal processing block size; also bounds MIDI event latency.
pub const MAX_BLOCK: usize = 256;
pub const MAX_TRACKS: usize = 128;
pub const MAX_RETURNS: usize = 8;
/// Values an instrument reports for its display (see `InstState::visual`).
pub const INST_VALS: usize = 64;

/// Samples of the shown instrument's sound kept for its display.
pub const INST_SCOPE_LEN: usize = 8192;

const RELAXED: Ordering = Ordering::Relaxed;

pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn new(v: f32) -> Self {
        Self(AtomicU32::new(v.to_bits()))
    }
    pub fn load(&self) -> f32 {
        f32::from_bits(self.0.load(RELAXED))
    }
    pub fn store(&self, v: f32) {
        self.0.store(v.to_bits(), RELAXED)
    }
    pub fn swap(&self, v: f32) -> f32 {
        f32::from_bits(self.0.swap(v.to_bits(), RELAXED))
    }
    pub fn store_max(&self, v: f32) {
        if v > self.load() {
            self.store(v);
        }
    }
}

pub struct AtomicF64(AtomicU64);

impl AtomicF64 {
    pub fn new(v: f64) -> Self {
        Self(AtomicU64::new(v.to_bits()))
    }
    pub fn load(&self) -> f64 {
        f64::from_bits(self.0.load(RELAXED))
    }
    pub fn store(&self, v: f64) {
        self.0.store(v.to_bits(), RELAXED)
    }
}

pub const QUEUE_NONE: i32 = -2;
pub const QUEUE_STOP: i32 = -1;

pub struct TrackStatus {
    /// Session slot currently playing, or -1.
    pub playing_slot: AtomicI32,
    /// Slot waiting for the launch quantization point, `QUEUE_STOP`, or `QUEUE_NONE`.
    pub queued_slot: AtomicI32,
    /// Position inside the playing clip in beats, or -1.
    pub clip_pos: AtomicF64,
    /// Id of the arrangement clip under the playhead (0 = none).
    pub arr_clip: AtomicU64,
    /// True when session clips override the arrangement on this track.
    pub overridden: AtomicBool,
    pub peak_l: AtomicF32,
    pub peak_r: AtomicF32,
    /// Visual state of the track's first sequencer (see `sequencers`).
    pub seq: [AtomicU64; 4],
    /// Peak of the track's audio input since the UI last took it.
    pub in_peak: AtomicF32,
    /// The instrument's sound, the notes it holds and the values of
    /// `InstState::visual`, for its screen (every track's, so all of them
    /// move in the Rack).
    pub inst_scope: crate::scope::ScopeBuffer,
    pub inst_held: [AtomicU64; 2],
    pub inst_vals: Vec<AtomicF32>,
}

impl TrackStatus {
    fn new() -> Self {
        Self {
            playing_slot: AtomicI32::new(-1),
            queued_slot: AtomicI32::new(QUEUE_NONE),
            clip_pos: AtomicF64::new(-1.0),
            arr_clip: AtomicU64::new(0),
            overridden: AtomicBool::new(false),
            peak_l: AtomicF32::new(0.0),
            peak_r: AtomicF32::new(0.0),
            seq: [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)],
            in_peak: AtomicF32::new(0.0),
            inst_scope: crate::scope::ScopeBuffer::new(INST_SCOPE_LEN, false),
            inst_held: [AtomicU64::new(0), AtomicU64::new(0)],
            inst_vals: (0..INST_VALS).map(|_| AtomicF32::new(0.0)).collect(),
        }
    }

    /// UI: the notes the instrument holds, as a 128-bit mask.
    pub fn inst_notes(&self) -> u128 {
        self.inst_held[0].load(RELAXED) as u128 | (self.inst_held[1].load(RELAXED) as u128) << 64
    }

    /// UI: the instrument's display values.
    pub fn inst_values(&self) -> [f32; INST_VALS] {
        std::array::from_fn(|i| self.inst_vals[i].load())
    }

    /// Peak level since the last call.
    pub fn take_peaks(&self) -> (f32, f32) {
        (self.peak_l.swap(0.0), self.peak_r.swap(0.0))
    }
}

pub struct EngineStatus {
    pub playing: AtomicBool,
    pub song_pos: AtomicF64,
    pub session_pos: AtomicF64,
    pub sample_rate: AtomicF32,
    pub cpu_load: AtomicF32,
    pub tracks: Vec<TrackStatus>,
    pub returns: Vec<TrackStatus>,
    pub master: TrackStatus,
    pub preview_active: AtomicBool,
    /// Output of each modulator (-1..1), by position in `Project::modulators`.
    pub mods: Vec<AtomicF32>,
    /// Audio taps of devices with a display.
    pub scopes: crate::scope::ScopeBank,
    /// Numbers those devices publish (gain reduction, loudness, ...).
    pub meters: crate::scope::MeterBank,
    /// Peak of each audio interface input since the UI last took it.
    pub input_peaks: Vec<AtomicF32>,
    /// Inputs the audio interface delivers (0 = no input open).
    pub input_channels: AtomicU32,
    /// True while armed audio tracks are being captured.
    pub capturing: AtomicBool,
    /// Recorded blocks that did not fit in the channel to the UI.
    pub rec_dropped: AtomicU64,
    /// Each sequencer's state for its editor, by device id.
    pub seqs: SeqBank,
    /// The tape's audio and where it is.
    pub reel: crate::tape::Reel,
    /// A new play key waits for the next bar.
    pub key_waiting: AtomicBool,
}

/// The visual state of every sequencer (see `MidiFxState::visual`), by
/// device id, so each editor moves, not only a track's first.
pub struct SeqBank {
    owners: Vec<AtomicU64>,
    vals: Vec<AtomicU64>,
}

pub const MAX_SEQ_VIS: usize = 128;

impl SeqBank {
    fn new() -> Self {
        Self { owners: (0..MAX_SEQ_VIS).map(|_| AtomicU64::new(0)).collect(), vals: (0..MAX_SEQ_VIS * 4).map(|_| AtomicU64::new(0)).collect() }
    }

    fn slot(&self, id: u64) -> Option<usize> {
        self.owners.iter().position(|o| o.load(RELAXED) == id)
    }

    /// Audio thread: stores device `id`'s state, claiming a slot if needed.
    pub fn write(&self, id: u64, v: [u64; 4]) {
        if id == 0 {
            return;
        }
        let s = self.slot(id).or_else(|| self.owners.iter().position(|o| o.compare_exchange(0, id, Ordering::AcqRel, RELAXED).is_ok()));
        if let Some(s) = s {
            for (a, x) in self.vals[s * 4..s * 4 + 4].iter().zip(v) {
                a.store(x, RELAXED);
            }
        }
    }

    /// UI: device `id`'s last state, if it reported one.
    pub fn read(&self, id: u64) -> Option<[u64; 4]> {
        let s = self.slot(id)?;
        Some(std::array::from_fn(|i| self.vals[s * 4 + i].load(RELAXED)))
    }

    /// UI: frees the slots of devices that no longer exist.
    pub fn release_unused(&self, alive: impl Fn(u64) -> bool) {
        for o in &self.owners {
            let id = o.load(RELAXED);
            if id != 0 && !alive(id) {
                let _ = o.compare_exchange(id, 0, Ordering::AcqRel, RELAXED);
            }
        }
    }
}

impl EngineStatus {
    pub fn new() -> Self {
        Self {
            playing: AtomicBool::new(false),
            song_pos: AtomicF64::new(0.0),
            session_pos: AtomicF64::new(0.0),
            sample_rate: AtomicF32::new(48000.0),
            cpu_load: AtomicF32::new(0.0),
            tracks: (0..MAX_TRACKS).map(|_| TrackStatus::new()).collect(),
            returns: (0..MAX_RETURNS).map(|_| TrackStatus::new()).collect(),
            master: TrackStatus::new(),
            preview_active: AtomicBool::new(false),
            mods: (0..MAX_MODS).map(|_| AtomicF32::new(0.0)).collect(),
            scopes: crate::scope::ScopeBank::new(),
            meters: crate::scope::MeterBank::new(),
            input_peaks: (0..MAX_INPUTS).map(|_| AtomicF32::new(0.0)).collect(),
            input_channels: AtomicU32::new(0),
            capturing: AtomicBool::new(false),
            rec_dropped: AtomicU64::new(0),
            seqs: SeqBank::new(),
            reel: crate::tape::Reel::new(),
            key_waiting: AtomicBool::new(false),
        }
    }
}

impl Default for EngineStatus {
    fn default() -> Self {
        Self::new()
    }
}

pub enum Command {
    SetProject(Arc<Project>),
    Play,
    Stop,
    SetPosition(f64),
    LaunchClip { track: u64, slot: usize },
    StopTrack { track: u64 },
    LaunchScene(usize),
    StopAllClips,
    BackToArrangement,
    NoteOn { track: u64, pitch: u8, velocity: f32 },
    NoteOff { track: u64, pitch: u8 },
    AllNotesOff,
    Preview(Option<SampleRef>),
    /// Gives the audio half of a hosted plugin to the instrument or effect
    /// device with this instance id.
    AttachProcessor { instance: u64, processor: Box<dyn AudioProcessor> },
    /// The audio interface input (or none).
    SetInput(Option<Arc<InputRing>>),
    /// Start or stop sending the input of armed audio tracks to the UI.
    SetCapture(bool),
    /// The sustain pedal (MIDI CC 64) for a track's instrument.
    Sustain { track: u64, down: bool },
    /// Bends the instrument of a track by `semis` semitones (0: none).
    PitchBend { track: u64, semis: f32 },
    /// The mod wheel of a track (0..1), for its Mod Wheel modulators.
    ModWheel { track: u64, value: f32 },
    /// Puts the tape at a place (seconds).
    TapeLocate(f64),
    /// The track being played, which the tape records from by default.
    TapeFollow(Option<u64>),
    /// Brakes the tape to a stop while true.
    TapeBrake(bool),
    /// Repeats the last `beats` the tape played until None.
    TapeStutter(Option<f64>),
    /// Moves the tape by hand at this speed (also when stopped) until None.
    TapeScrub(Option<f32>),
    /// Bends the tape's speed by a ratio for a moment (1: as set).
    TapeBend(f32),
    /// A joystick on a track (-1..1 each way), for its Joystick modulators.
    Joystick { track: u64, x: f32, y: f32 },
    /// The grand piano's samples (installed, or none).
    SetPianoLibrary(Option<Arc<PianoLibrary>>),
}

/// Frames per recorded block.
pub const REC_FRAMES: usize = MAX_BLOCK;

/// A block of the input of an armed audio track, captured while recording.
/// Blocks of one track arrive in order and without gaps.
#[derive(Clone)]
pub struct RecChunk {
    pub track: u64,
    /// Session and arrangement position of the first frame, in beats.
    pub session_beat: f64,
    pub song_beat: f64,
    pub frames: usize,
    pub l: [f32; REC_FRAMES],
    pub r: [f32; REC_FRAMES],
}

/// A note an armed MIDI track got from another track ("MIDI From"), for the
/// UI to record: where it falls in the track's playing session clip.
#[derive(Clone, Copy, Debug)]
pub struct MidiRec {
    pub track: u64,
    pub pitch: u8,
    pub vel: f32,
    pub on: bool,
    /// The playing slot (-1: none) and the position in its clip (beats).
    pub slot: i32,
    pub pos: f64,
}

/// Something the audio thread no longer needs. Drop it on another thread.
pub struct Garbage(#[allow(dead_code)] GarbageInner);

#[allow(dead_code)]
enum GarbageInner {
    Project(Arc<Project>),
    Track(Box<TrackRt>),
    Effect(EffectState),
    Instrument(InstState),
    Sample(SampleRef),
    Processor(Box<dyn AudioProcessor>),
    Input(Arc<InputRing>),
    PianoLibrary(Arc<PianoLibrary>),
}

/// The UI side of the engine.
pub struct EngineHandle {
    pub commands: Sender<Command>,
    pub garbage: Receiver<Garbage>,
    pub status: Arc<EngineStatus>,
    /// Recorded audio of armed tracks (see `Command::SetCapture`).
    pub recorded: Receiver<RecChunk>,
    /// Notes armed tracks got from other tracks.
    pub midi_recorded: Receiver<MidiRec>,
}

impl EngineHandle {
    pub fn send(&self, cmd: Command) -> bool {
        self.commands.try_send(cmd).is_ok()
    }

    pub fn collect_garbage(&self) {
        while self.garbage.try_recv().is_ok() {}
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Clock {
    Session,
    Arrangement,
}

#[derive(Clone, Copy)]
struct PendingOff {
    at: f64,
    pitch: u8,
    clock: Clock,
}

type Ev = MidiEvent;

#[derive(Clone, Copy)]
struct PlayingClip {
    slot: usize,
    /// Session-clock beat at which the clip started.
    start: f64,
}

#[derive(Clone, Copy)]
struct Queued {
    slot: Option<usize>,
    at: f64,
}

enum InstState {
    None,
    Synth(Box<SynthState>),
    Drums(Box<DrumState>),
    Sampler(Box<SamplerState>),
    Fm(Box<FmState>),
    Macro(Box<MacroState>),
    Braids(Box<BraidsSynthState>),
    Elements(Box<ElementsSynthState>),
    Analog(Box<AnalogState>),
    Shakers(Box<ShakerSynthState>),
    Modular(Box<ModularState>),
    Voice(Box<VoiceState>),
    Mallets(Box<MalletsState>),
    Strings(Box<StringsState>),
    Winds(Box<WindsState>),
    Organ(Box<OrganState>),
    Rings(Box<RingsState>),
    Monk(Box<MonkState>),
    Piano(Box<PianoState>),
    Bassline(Box<BasslineState>),
    Mono101(Box<Mono101State>),
    Drone(Box<DroneState>),
    Waves(Box<WavesState>),
    Guitar(Box<GuitarState>),
    WindChimes(Box<WindChimesState>),
    BeadsSynth(Box<BeadsSynthState>),
    Birds(Box<BirdsState>),
    Nature(Box<NatureState>),
    PanFlute(Box<PanFluteState>),
    Melodica(Box<MelodicaState>),
    Harmonica(Box<HarmonicaState>),
    Facet(Box<FacetState>),
    Plugin { instance: u64, proc: Option<Box<dyn AudioProcessor>>, pending: Vec<PluginEvent> },
}

impl InstState {
    /// Bends the instruments that can be bent (by `semis` semitones).
    fn set_bend(&mut self, semis: f32) {
        match self {
            InstState::Synth(s) => s.pitch_bend = semis,
            InstState::Analog(s) => s.pitch_bend = semis,
            InstState::Waves(s) => s.pitch_bend = semis,
            InstState::Mono101(s) => s.pitch_bend = semis,
            InstState::Bassline(s) => s.pitch_bend = semis,
            InstState::Macro(s) => s.pitch_bend = semis,
            InstState::Braids(s) => s.pitch_bend = semis,
            InstState::Elements(s) => s.pitch_bend = semis,
            InstState::Fm(s) => s.pitch_bend = semis,
            InstState::Sampler(s) => s.pitch_bend = semis,
            InstState::Drone(s) => s.pitch_bend = semis,
            InstState::Organ(s) => s.pitch_bend = semis,
            InstState::Guitar(s) => s.pitch_bend = semis,
            InstState::BeadsSynth(s) => s.pitch_bend = semis,
            InstState::PanFlute(s) => s.pitch_bend = semis,
            InstState::Melodica(s) => s.pitch_bend = semis,
            InstState::Harmonica(s) => s.pitch_bend = semis,
            InstState::Facet(s) => s.pitch_bend = semis,
            InstState::Voice(s) => s.pitch_bend = semis,
            InstState::Monk(s) => s.pitch_bend = semis,
            _ => {}
        }
    }

    fn for_instrument(i: Option<&Instrument>, sr: f32) -> Self {
        match i {
            None => InstState::None,
            Some(Instrument::Synth { .. }) => InstState::Synth(Box::default()),
            Some(Instrument::Drums { .. }) => InstState::Drums(Box::default()),
            Some(Instrument::Sampler { .. }) => InstState::Sampler(Box::default()),
            Some(Instrument::Plugin(p)) => InstState::Plugin { instance: p.instance, proc: None, pending: Vec::with_capacity(256) },
            Some(Instrument::Fm { .. }) => InstState::Fm(Box::default()),
            Some(Instrument::Macro { .. }) => InstState::Macro(Box::new(MacroState::new(sr))),
            Some(Instrument::Braids { .. }) => InstState::Braids(Box::new(BraidsSynthState::new(sr))),
            Some(Instrument::Elements { .. }) => InstState::Elements(Box::new(ElementsSynthState::new(sr))),
            Some(Instrument::Analog { .. }) => InstState::Analog(Box::new(AnalogState::new(sr))),
            Some(Instrument::Shakers { .. }) => InstState::Shakers(Box::new(ShakerSynthState::new(sr))),
            Some(Instrument::Modular { .. }) => InstState::Modular(Box::new(ModularState::new(sr))),
            Some(Instrument::Voice { .. }) => InstState::Voice(Box::new(VoiceState::new(sr))),
            Some(Instrument::Mallets { .. }) => InstState::Mallets(Box::new(MalletsState::new(sr))),
            Some(Instrument::Strings { .. }) => InstState::Strings(Box::new(StringsState::new(sr))),
            Some(Instrument::Winds { .. }) => InstState::Winds(Box::new(WindsState::new(sr))),
            Some(Instrument::Organ { .. }) => InstState::Organ(Box::new(OrganState::new(sr))),
            Some(Instrument::Rings { .. }) => InstState::Rings(Box::new(RingsState::new(sr))),
            Some(Instrument::Monk { .. }) => InstState::Monk(Box::new(MonkState::new(sr))),
            Some(Instrument::Piano { .. }) => InstState::Piano(Box::new(PianoState::new(sr))),
            Some(Instrument::Bassline { .. }) => InstState::Bassline(Box::new(BasslineState::new(sr))),
            Some(Instrument::Mono101 { .. }) => InstState::Mono101(Box::new(Mono101State::new(sr))),
            Some(Instrument::Drone { .. }) => InstState::Drone(Box::new(DroneState::new(sr))),
            Some(Instrument::Waves { .. }) => InstState::Waves(Box::new(WavesState::new(sr))),
            Some(Instrument::Guitar { .. }) => InstState::Guitar(Box::new(GuitarState::new(sr))),
            Some(Instrument::WindChimes { .. }) => InstState::WindChimes(Box::new(WindChimesState::new(sr))),
            Some(Instrument::BeadsSynth { .. }) => InstState::BeadsSynth(Box::new(BeadsSynthState::new(sr))),
            Some(Instrument::Birds { .. }) => InstState::Birds(Box::new(BirdsState::new(sr))),
            Some(Instrument::Nature { .. }) => InstState::Nature(Box::new(NatureState::new(sr))),
            Some(Instrument::PanFlute { .. }) => InstState::PanFlute(Box::new(PanFluteState::new(sr))),
            Some(Instrument::Melodica { .. }) => InstState::Melodica(Box::new(MelodicaState::new(sr))),
            Some(Instrument::Harmonica { .. }) => InstState::Harmonica(Box::new(HarmonicaState::new(sr))),
            Some(Instrument::Facet { .. }) => InstState::Facet(Box::new(FacetState::new(sr))),
        }
    }

    /// Stops a plugin processor (audio thread) before the state is discarded.
    fn suspend(&mut self) {
        if let InstState::Plugin { proc: Some(p), .. } = self {
            p.suspend();
        }
    }

    fn push_plugin_event(&mut self, pitch: u8, kind: PluginEventKind) {
        if let InstState::Plugin { pending, .. } = self {
            if pending.len() < pending.capacity() {
                pending.push(PluginEvent { offset: 0, pitch, kind });
            }
        }
    }

    fn matches(&self, i: Option<&Instrument>) -> bool {
        matches!(
            (self, i),
            (InstState::None, None)
                | (InstState::Synth(_), Some(Instrument::Synth { .. }))
                | (InstState::Drums(_), Some(Instrument::Drums { .. }))
                | (InstState::Sampler(_), Some(Instrument::Sampler { .. }))
                | (InstState::Fm(_), Some(Instrument::Fm { .. }))
                | (InstState::Macro(_), Some(Instrument::Macro { .. }))
                | (InstState::Braids(_), Some(Instrument::Braids { .. }))
                | (InstState::Elements(_), Some(Instrument::Elements { .. }))
                | (InstState::Analog(_), Some(Instrument::Analog { .. }))
                | (InstState::Shakers(_), Some(Instrument::Shakers { .. }))
                | (InstState::Modular(_), Some(Instrument::Modular { .. }))
                | (InstState::Voice(_), Some(Instrument::Voice { .. }))
                | (InstState::Mallets(_), Some(Instrument::Mallets { .. }))
                | (InstState::Strings(_), Some(Instrument::Strings { .. }))
                | (InstState::Winds(_), Some(Instrument::Winds { .. }))
                | (InstState::Organ(_), Some(Instrument::Organ { .. }))
                | (InstState::Rings(_), Some(Instrument::Rings { .. }))
                | (InstState::Monk(_), Some(Instrument::Monk { .. }))
                | (InstState::Piano(_), Some(Instrument::Piano { .. }))
                | (InstState::Bassline(_), Some(Instrument::Bassline { .. }))
                | (InstState::Mono101(_), Some(Instrument::Mono101 { .. }))
                | (InstState::Drone(_), Some(Instrument::Drone { .. }))
                | (InstState::Waves(_), Some(Instrument::Waves { .. }))
                | (InstState::Guitar(_), Some(Instrument::Guitar { .. }))
                | (InstState::WindChimes(_), Some(Instrument::WindChimes { .. }))
                | (InstState::BeadsSynth(_), Some(Instrument::BeadsSynth { .. }))
                | (InstState::Birds(_), Some(Instrument::Birds { .. }))
                | (InstState::Nature(_), Some(Instrument::Nature { .. }))
                | (InstState::PanFlute(_), Some(Instrument::PanFlute { .. }))
                | (InstState::Melodica(_), Some(Instrument::Melodica { .. }))
                | (InstState::Harmonica(_), Some(Instrument::Harmonica { .. }))
                | (InstState::Facet(_), Some(Instrument::Facet { .. }))
        ) || matches!((self, i), (InstState::Plugin { instance, .. }, Some(Instrument::Plugin(p))) if *instance == p.instance)
    }

    /// `ov` replaces the instrument's parameters (modulation).
    fn note_on(&mut self, inst: &Instrument, ov: Option<&[f32]>, pitch: u8, vel: f32, sr: f32) {
        match (self, inst) {
            (InstState::Synth(s), Instrument::Synth { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Drums(s), Instrument::Drums { pads, .. }) => s.note_on(pads, pitch, vel, sr),
            (InstState::Sampler(s), Instrument::Sampler { params, sample, zones }) => s.note_on(ov.unwrap_or(params), sample.as_ref(), zones, pitch, vel),
            (st @ InstState::Plugin { .. }, Instrument::Plugin(_)) => st.push_plugin_event(pitch, PluginEventKind::NoteOn { velocity: vel }),
            (InstState::Fm(s), Instrument::Fm { voice, op_mask, params, .. }) => s.note_on(ov.unwrap_or(params), voice, *op_mask, pitch, vel),
            (InstState::Macro(s), Instrument::Macro { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Braids(s), Instrument::Braids { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Elements(s), Instrument::Elements { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Analog(s), Instrument::Analog { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Shakers(s), Instrument::Shakers { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Modular(s), Instrument::Modular { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Voice(s), Instrument::Voice { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Mallets(s), Instrument::Mallets { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Strings(s), Instrument::Strings { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Winds(s), Instrument::Winds { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Organ(s), Instrument::Organ { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Rings(s), Instrument::Rings { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Monk(s), Instrument::Monk { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Piano(s), Instrument::Piano { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Bassline(s), Instrument::Bassline { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Mono101(s), Instrument::Mono101 { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Drone(s), Instrument::Drone { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Waves(s), Instrument::Waves { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Guitar(s), Instrument::Guitar { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::WindChimes(s), Instrument::WindChimes { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::BeadsSynth(s), Instrument::BeadsSynth { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Birds(s), Instrument::Birds { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Nature(s), Instrument::Nature { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::PanFlute(s), Instrument::PanFlute { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Melodica(s), Instrument::Melodica { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Harmonica(s), Instrument::Harmonica { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Facet(s), Instrument::Facet { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            _ => {}
        }
    }

    fn note_off(&mut self, inst: &Instrument, ov: Option<&[f32]>, pitch: u8) {
        match (self, inst) {
            (InstState::Synth(s), Instrument::Synth { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Sampler(s), Instrument::Sampler { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (st @ InstState::Plugin { .. }, Instrument::Plugin(_)) => st.push_plugin_event(pitch, PluginEventKind::NoteOff),
            (InstState::Fm(s), Instrument::Fm { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Macro(s), Instrument::Macro { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Braids(s), Instrument::Braids { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Elements(s), Instrument::Elements { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Analog(s), Instrument::Analog { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Shakers(s), Instrument::Shakers { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Modular(s), Instrument::Modular { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Voice(s), Instrument::Voice { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Mallets(s), Instrument::Mallets { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Strings(s), Instrument::Strings { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Winds(s), Instrument::Winds { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Organ(s), Instrument::Organ { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Rings(s), Instrument::Rings { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Monk(s), Instrument::Monk { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Piano(s), Instrument::Piano { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Bassline(s), Instrument::Bassline { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Mono101(s), Instrument::Mono101 { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Drone(s), Instrument::Drone { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Waves(s), Instrument::Waves { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Guitar(s), Instrument::Guitar { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::WindChimes(s), Instrument::WindChimes { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::BeadsSynth(s), Instrument::BeadsSynth { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Birds(s), Instrument::Birds { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Nature(s), Instrument::Nature { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::PanFlute(s), Instrument::PanFlute { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Melodica(s), Instrument::Melodica { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Harmonica(s), Instrument::Harmonica { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Facet(s), Instrument::Facet { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            _ => {}
        }
    }

    fn all_off(&mut self) {
        match self {
            InstState::Synth(s) => s.all_off(),
            InstState::Drums(s) => s.all_off(),
            InstState::Sampler(s) => s.all_off(),
            InstState::Plugin { .. } => self.push_plugin_event(0, PluginEventKind::AllNotesOff),
            InstState::Fm(s) => s.all_off(),
            InstState::Macro(s) => s.all_off(),
            InstState::Braids(s) => s.all_off(),
            InstState::Elements(s) => s.all_off(),
            InstState::Analog(s) => s.all_off(),
            InstState::Shakers(s) => s.all_off(),
            InstState::Modular(s) => s.all_off(),
            InstState::Voice(s) => s.all_off(),
            InstState::Mallets(s) => s.all_off(),
            InstState::Strings(s) => s.all_off(),
            InstState::Winds(s) => s.all_off(),
            InstState::Organ(s) => s.all_off(),
            InstState::Rings(s) => s.all_off(),
            InstState::Monk(s) => s.all_off(),
            InstState::Piano(s) => s.all_off(),
            InstState::Bassline(s) => s.all_off(),
            InstState::Mono101(s) => s.all_off(),
            InstState::Drone(s) => s.all_off(),
            InstState::Waves(s) => s.all_off(),
            InstState::Guitar(s) => s.all_off(),
            InstState::WindChimes(s) => s.all_off(),
            InstState::BeadsSynth(s) => s.all_off(),
            InstState::Birds(s) => s.all_off(),
            InstState::Nature(s) => s.all_off(),
            InstState::PanFlute(s) => s.all_off(),
            InstState::Melodica(s) => s.all_off(),
            InstState::Harmonica(s) => s.all_off(),
            InstState::Facet(s) => s.all_off(),
            InstState::None => {}
        }
    }

    /// Fills in what this instrument shows on its display (zeros where it has
    /// nothing to say). Returns false for instruments without such values.
    fn visual(&self, out: &mut [f32; INST_VALS]) -> bool {
        match self {
            InstState::Shakers(s) => s.visual(out),
            InstState::Organ(s) => s.visual(out),
            InstState::Analog(s) => s.visual(out),
            InstState::Synth(s) => s.visual(out),
            InstState::Voice(s) => s.visual(out),
            InstState::Monk(s) => s.visual(out),
            InstState::Piano(s) => s.visual(out),
            InstState::Bassline(s) => s.visual(out),
            InstState::Mono101(s) => s.visual(out),
            InstState::Drone(s) => s.visual(out),
            InstState::Waves(s) => s.visual(out),
            InstState::Guitar(s) => s.visual(out),
            InstState::WindChimes(s) => s.visual(out),
            InstState::BeadsSynth(s) => s.visual(out),
            InstState::Birds(s) => s.visual(out),
            InstState::Nature(s) => s.visual(out),
            InstState::PanFlute(s) => s.visual(out),
            InstState::Melodica(s) => s.visual(out),
            InstState::Harmonica(s) => s.visual(out),
            InstState::Facet(s) => s.visual(out),
            _ => return false,
        }
        true
    }

    fn render(&mut self, inst: &Instrument, ov: Option<&[f32]>, l: &mut [f32], r: &mut [f32], sr: f32) {
        match (self, inst) {
            (InstState::Synth(s), Instrument::Synth { params, .. }) => s.render(ov.unwrap_or(params), l, r, sr),
            (InstState::Drums(s), Instrument::Drums { pads, .. }) => s.render(pads, l, r, sr),
            (InstState::Sampler(s), Instrument::Sampler { params, sample, zones }) => s.render(ov.unwrap_or(params), sample.as_ref(), zones, l, r, sr),
            (InstState::Fm(s), Instrument::Fm { voice, params, .. }) => s.render(ov.unwrap_or(params), voice, l, r, sr),
            (InstState::Macro(s), Instrument::Macro { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Braids(s), Instrument::Braids { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Elements(s), Instrument::Elements { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Analog(s), Instrument::Analog { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Shakers(s), Instrument::Shakers { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Modular(s), Instrument::Modular { params, patch, .. }) => s.render(ov.unwrap_or(params), patch, l, r),
            (InstState::Voice(s), Instrument::Voice { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Mallets(s), Instrument::Mallets { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Strings(s), Instrument::Strings { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Winds(s), Instrument::Winds { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Organ(s), Instrument::Organ { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Rings(s), Instrument::Rings { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Monk(s), Instrument::Monk { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Piano(s), Instrument::Piano { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Bassline(s), Instrument::Bassline { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Mono101(s), Instrument::Mono101 { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Drone(s), Instrument::Drone { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Waves(s), Instrument::Waves { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Guitar(s), Instrument::Guitar { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::WindChimes(s), Instrument::WindChimes { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::BeadsSynth(s), Instrument::BeadsSynth { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Birds(s), Instrument::Birds { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Nature(s), Instrument::Nature { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::PanFlute(s), Instrument::PanFlute { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Melodica(s), Instrument::Melodica { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Harmonica(s), Instrument::Harmonica { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Facet(s), Instrument::Facet { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Plugin { proc, pending, .. }, Instrument::Plugin(_)) => {
                if let Some(p) = proc {
                    p.process(pending, l, r);
                }
                pending.clear();
            }
            _ => {}
        }
    }
}

struct TrackRt {
    id: u64,
    instrument: InstState,
    devices: Vec<(u64, EffectState)>,
    /// Where the Map puts it (made when it first goes on the Map).
    placer: Option<Box<crate::dsp::fx::stage::Stage>>,
    /// It was off the Map: start afresh when it comes back.
    placer_idle: bool,
    playing: Option<PlayingClip>,
    queued: Option<Queued>,
    overridden: bool,
    offs: Vec<PendingOff>,
    events: Vec<Ev>,
    /// Scratch buffer for the MIDI effect chain.
    fx_buf: Vec<Ev>,
    /// Live notes (keyboard / MIDI input) for the next block.
    live: Vec<Ev>,
    midi_fx: Vec<(u64, MidiFxState)>,
    /// Signature of the enabled MIDI effects. When it changes (or after a
    /// stop) the notes the old chain started are released.
    fx_sig: u64,
    /// Note-ons minus note-offs the instrument received, per pitch.
    sounding: [u8; 128],
    /// How far the Key drawer moved each pitch (its note-off goes there too).
    shifted: [i8; 128],
    /// Notes and level for the modulators.
    signal: TrackSignal,
    /// The pitch bend in semitones.
    bend: f32,
    l: Vec<f32>,
    r: Vec<f32>,
    gain_l: f32,
    gain_r: f32,
    sends: [f32; MAX_RETURNS],
    /// The track's audio before the effects, after them, and after the mixer,
    /// for tracks that listen to it.
    pre_l: Vec<f32>,
    pre_r: Vec<f32>,
    post_l: Vec<f32>,
    post_r: Vec<f32>,
    out_l: Vec<f32>,
    out_r: Vec<f32>,
    /// What this (audio) track hears: its input and tracks sent to it.
    sources: Vec<Source>,
    /// The notes this track played in the current segment (for tracks that
    /// take them) and the track it takes notes from.
    midi_out: Vec<Ev>,
    midi_src: Option<usize>,
    /// False when the track's output goes into another track.
    to_master: bool,
}

/// One thing an audio track hears.
#[derive(Clone, Copy, Debug)]
enum Source {
    Ext { channel: usize, stereo: bool },
    Track { index: usize, tap: Tap },
    Resampling,
}

impl TrackRt {
    fn new(id: u64) -> Self {
        Self {
            id,
            instrument: InstState::None,
            devices: Vec::new(),
            placer: None,
            placer_idle: false,
            playing: None,
            queued: None,
            overridden: false,
            offs: Vec::with_capacity(512),
            events: Vec::with_capacity(1024),
            fx_buf: Vec::with_capacity(1024),
            live: Vec::with_capacity(256),
            midi_fx: Vec::new(),
            fx_sig: FX_SIG_EMPTY,
            sounding: [0; 128],
            shifted: [0; 128],
            bend: 0.0,
            signal: TrackSignal::default(),
            l: vec![0.0; MAX_BLOCK],
            r: vec![0.0; MAX_BLOCK],
            gain_l: 0.0,
            gain_r: 0.0,
            sends: [0.0; MAX_RETURNS],
            pre_l: vec![0.0; MAX_BLOCK],
            pre_r: vec![0.0; MAX_BLOCK],
            post_l: vec![0.0; MAX_BLOCK],
            post_r: vec![0.0; MAX_BLOCK],
            out_l: vec![0.0; MAX_BLOCK],
            out_r: vec![0.0; MAX_BLOCK],
            sources: Vec::with_capacity(8),
            midi_out: Vec::with_capacity(512),
            midi_src: None,
            to_master: true,
        }
    }

    fn release(&mut self, clock: Clock) {
        for o in self.offs.iter_mut() {
            if o.clock == clock {
                o.at = f64::NEG_INFINITY;
            }
        }
    }

    fn reset_midi_fx(&mut self) {
        self.live.clear();
        for (_, st) in self.midi_fx.iter_mut() {
            st.reset();
        }
        // Release whatever the effects left sounding in the next block.
        self.fx_sig = FX_SIG_FLUSH;
        self.signal.held = 0;
    }
}

const FX_SIG_EMPTY: u64 = 0xcbf2_9ce4_8422_2325;
const FX_SIG_FLUSH: u64 = 0;

fn fx_signature(devices: &[crate::project::MidiDevice]) -> u64 {
    devices.iter().fold(FX_SIG_EMPTY, |h, d| (h ^ (d.id << 1 | d.enabled as u64)).wrapping_mul(0x100_0000_01b3))
}

struct BusRt {
    id: u64,
    devices: Vec<(u64, EffectState)>,
    l: Vec<f32>,
    r: Vec<f32>,
    gain_l: f32,
    gain_r: f32,
}

impl BusRt {
    fn new(id: u64) -> Self {
        Self { id, devices: Vec::new(), l: vec![0.0; MAX_BLOCK], r: vec![0.0; MAX_BLOCK], gain_l: 0.0, gain_r: 0.0 }
    }
}

/// The master output, kept for tracks that record it ("Resampling"). It is
/// read one block late, so a track can listen while the master is mixed.
struct ResampleRing {
    l: Vec<f32>,
    r: Vec<f32>,
    pos: usize,
}

const RESAMPLE_LEN: usize = 4 * MAX_BLOCK;

impl ResampleRing {
    fn new() -> Self {
        Self { l: vec![0.0; RESAMPLE_LEN], r: vec![0.0; RESAMPLE_LEN], pos: 0 }
    }

    fn write(&mut self, l: &[f32], r: &[f32]) {
        for (a, b) in l.iter().zip(r) {
            self.l[self.pos] = *a;
            self.r[self.pos] = *b;
            self.pos = (self.pos + 1) % RESAMPLE_LEN;
        }
    }

    /// Frame `i` of the current block, one block late.
    #[inline]
    fn read(&self, i: usize) -> (f32, f32) {
        let k = (self.pos + RESAMPLE_LEN - MAX_BLOCK + i) % RESAMPLE_LEN;
        (self.l[k], self.r[k])
    }
}

/// Sums what audio track `ti` hears into `out_l`/`out_r`.
fn gather_input(tracks: &[Box<TrackRt>], ti: usize, ext: &[f32], resample: &ResampleRing, seg: usize, out_l: &mut [f32], out_r: &mut [f32]) {
    out_l[..seg].fill(0.0);
    out_r[..seg].fill(0.0);
    let Some(rt) = tracks.get(ti) else { return };
    for src in &rt.sources {
        match *src {
            Source::Ext { channel, stereo } => {
                let a = &ext[channel * MAX_BLOCK..channel * MAX_BLOCK + seg];
                let b = if stereo && channel + 1 < MAX_INPUTS { &ext[(channel + 1) * MAX_BLOCK..(channel + 1) * MAX_BLOCK + seg] } else { a };
                for i in 0..seg {
                    out_l[i] += a[i];
                    out_r[i] += b[i];
                }
            }
            Source::Track { index, tap } => {
                let Some(s) = tracks.get(index) else { continue };
                let (sl, sr) = match tap {
                    Tap::PreFx => (&s.pre_l, &s.pre_r),
                    Tap::PostFx => (&s.post_l, &s.post_r),
                    Tap::PostMixer => (&s.out_l, &s.out_r),
                };
                for i in 0..seg {
                    out_l[i] += sl[i];
                    out_r[i] += sr[i];
                }
            }
            Source::Resampling => {
                for i in 0..seg {
                    let (a, b) = resample.read(i);
                    out_l[i] += a;
                    out_r[i] += b;
                }
            }
        }
    }
}

/// Metronome click.
struct Click {
    phase: f32,
    freq: f32,
    amp: f32,
    coef: f32,
}

impl Click {
    fn trigger(&mut self, accent: bool, sr: f32) {
        self.phase = 0.0;
        self.freq = if accent { 1568.0 } else { 1046.0 };
        self.amp = if accent { 0.5 } else { 0.32 };
        self.coef = crate::dsp::decay_coef(0.04, sr);
    }

    #[inline]
    fn tick(&mut self, sr: f32) -> f32 {
        if self.amp < 1e-4 {
            return 0.0;
        }
        self.phase = crate::dsp::wrap_phase(self.phase + self.freq / sr);
        let y = (self.phase * crate::dsp::TAU).sin() * self.amp;
        self.amp *= self.coef;
        y
    }
}

/// Flushes denormals to zero while alive (x86-64 only).
struct DenormalGuard {
    #[cfg(target_arch = "x86_64")]
    old: u32,
}

impl DenormalGuard {
    #[cfg(target_arch = "x86_64")]
    fn new() -> Self {
        let mut old: u32 = 0;
        // SAFETY: reads/writes the MXCSR control register through valid pointers.
        unsafe {
            std::arch::asm!("stmxcsr [{}]", in(reg) &mut old as *mut u32, options(nostack, preserves_flags));
            let new = old | 0x8040; // FTZ | DAZ
            std::arch::asm!("ldmxcsr [{}]", in(reg) &new as *const u32, options(nostack, preserves_flags, readonly));
        }
        Self { old }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn new() -> Self {
        Self {}
    }
}

impl Drop for DenormalGuard {
    fn drop(&mut self) {
        #[cfg(target_arch = "x86_64")]
        // SAFETY: restores the previously saved MXCSR value.
        unsafe {
            std::arch::asm!("ldmxcsr [{}]", in(reg) &self.old as *const u32, options(nostack, preserves_flags, readonly));
        }
    }
}

pub struct Engine {
    sr: f32,
    /// The play key the music is in now (it may wait for a bar to follow the project's).
    key_now: Option<crate::scale::Scale>,
    project: Arc<Project>,
    cmd_rx: Receiver<Command>,
    garbage_tx: Sender<Garbage>,
    status: Arc<EngineStatus>,
    playing: bool,
    song_pos: f64,
    session_pos: f64,
    start_marker: f64,
    tracks: Vec<Box<TrackRt>>,
    returns: Vec<BusRt>,
    master: BusRt,
    click: Click,
    preview: Option<(SampleRef, f64)>,
    mix_l: Vec<f32>,
    mix_r: Vec<f32>,
    /// Processors that arrived before their track's instrument did.
    parked: Vec<(u64, Box<dyn AudioProcessor>)>,
    /// Frames rendered since the engine started (drives MIDI effect timing).
    clock: u64,
    mods: ModRuntime,
    /// Clip envelope values this block (target, 0..1).
    envs: Vec<(crate::modulation::ModTarget, f32)>,
    /// Per-track signals handed to the modulators each control block.
    signals: Vec<TrackSignal>,
    /// Track indices in processing order (sources before listeners).
    order: Vec<usize>,
    input: Option<Arc<InputRing>>,
    /// The interface inputs of the current block, channel-major.
    in_buf: Vec<f32>,
    /// What the current track hears.
    in_l: Vec<f32>,
    in_r: Vec<f32>,
    resample: ResampleRing,
    capture: bool,
    rec_tx: Sender<RecChunk>,
    midi_rec_tx: Sender<MidiRec>,
    /// The grand piano's samples, for every Grand Piano.
    piano_lib: Option<Arc<PianoLibrary>>,
    /// The notes the current track takes from another track.
    midi_scratch: Vec<Ev>,
    tape: crate::tape::TapeRt,
}

impl Engine {
    pub fn new(sample_rate: f32, project: Arc<Project>) -> (Engine, EngineHandle) {
        let (cmd_tx, cmd_rx) = bounded(8192);
        let (garbage_tx, garbage_rx) = bounded(8192);
        // About 20 seconds of one recording track; the UI drains it every frame.
        let (rec_tx, rec_rx) = bounded(4096);
        let (midi_rec_tx, midi_rec_rx) = bounded(4096);
        let status = Arc::new(EngineStatus::new());
        status.sample_rate.store(sample_rate);
        status.reel.set_rate(sample_rate);
        let mut e = Engine {
            sr: sample_rate,
            project,
            cmd_rx,
            garbage_tx,
            status: status.clone(),
            playing: false,
            key_now: None,
            song_pos: 0.0,
            session_pos: 0.0,
            start_marker: 0.0,
            tracks: Vec::new(),
            returns: Vec::new(),
            master: BusRt::new(0),
            click: Click { phase: 0.0, freq: 1000.0, amp: 0.0, coef: 0.0 },
            preview: None,
            mix_l: vec![0.0; MAX_BLOCK],
            mix_r: vec![0.0; MAX_BLOCK],
            parked: Vec::with_capacity(16),
            clock: 0,
            mods: ModRuntime::new(),
            envs: Vec::with_capacity(256),
            signals: Vec::with_capacity(MAX_TRACKS),
            order: Vec::with_capacity(MAX_TRACKS),
            input: None,
            in_buf: vec![0.0; MAX_INPUTS * MAX_BLOCK],
            in_l: vec![0.0; MAX_BLOCK],
            in_r: vec![0.0; MAX_BLOCK],
            resample: ResampleRing::new(),
            capture: false,
            rec_tx,
            midi_rec_tx,
            midi_scratch: Vec::with_capacity(512),
            piano_lib: crate::piano::library::shared(),
            tape: crate::tape::TapeRt::new(),
        };
        e.reconcile();
        (e, EngineHandle { commands: cmd_tx, garbage: garbage_rx, status, recorded: rec_rx, midi_recorded: midi_rec_rx })
    }

    pub fn sample_rate(&self) -> f32 {
        self.sr
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn song_pos(&self) -> f64 {
        self.song_pos
    }

    /// Stops and removes every plugin processor (call on the thread that ran
    /// `process`), so they can be dropped on the UI thread.
    pub fn take_processors(&mut self) -> Vec<Box<dyn AudioProcessor>> {
        let mut out = Vec::new();
        for rt in self.tracks.iter_mut() {
            if let InstState::Plugin { proc, .. } = &mut rt.instrument {
                if let Some(mut p) = proc.take() {
                    p.suspend();
                    out.push(p);
                }
            }
        }
        let chains = self.tracks.iter_mut().map(|t| &mut t.devices).chain(self.returns.iter_mut().map(|r| &mut r.devices)).chain(std::iter::once(&mut self.master.devices));
        for devs in chains {
            for (_, st) in devs.iter_mut() {
                if let EffectState::Plugin { proc, .. } = st {
                    if let Some(mut p) = proc.take() {
                        p.suspend();
                        out.push(p);
                    }
                }
            }
        }
        for (_, mut p) in self.parked.drain(..) {
            p.suspend();
            out.push(p);
        }
        out
    }

    fn samples_per_beat(&self) -> f64 {
        self.sr as f64 * 60.0 / self.project.bpm.max(1.0)
    }

    fn trash(&self, g: GarbageInner) {
        // If the channel is full the value is simply dropped here.
        let _ = self.garbage_tx.try_send(Garbage(g));
    }

    /// Renders interleaved stereo audio. `out.len()` must be even.
    pub fn process(&mut self, out: &mut [f32]) {
        let _guard = DenormalGuard::new();
        let started = Instant::now();
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            self.handle(cmd);
        }
        let frames = out.len() / 2;
        let mut done = 0;
        while done < frames {
            let n = (frames - done).min(MAX_BLOCK);
            self.process_block(n);
            for i in 0..n {
                out[(done + i) * 2] = self.mix_l[i].clamp(-1.0, 1.0);
                out[(done + i) * 2 + 1] = self.mix_r[i].clamp(-1.0, 1.0);
            }
            done += n;
        }
        self.publish_status();
        if frames > 0 {
            let budget = frames as f32 / self.sr;
            let load = started.elapsed().as_secs_f32() / budget;
            let prev = self.status.cpu_load.load();
            self.status.cpu_load.store(prev * 0.9 + load * 0.1);
        }
    }

    pub fn handle(&mut self, cmd: Command) {
        match cmd {
            Command::SetProject(p) => {
                let old = std::mem::replace(&mut self.project, p);
                self.trash(GarbageInner::Project(old));
                self.reconcile();
            }
            Command::Play => {
                if !self.playing {
                    self.start_transport();
                }
            }
            Command::Stop => {
                if self.playing {
                    self.playing = false;
                    // Like Live, session clips stay launched: Play starts
                    // them again. Stop All Clips (or a track's stop) clears them.
                    for rt in self.tracks.iter_mut() {
                        rt.queued = None;
                        rt.release(Clock::Session);
                        rt.release(Clock::Arrangement);
                        rt.reset_midi_fx();
                    }
                    self.song_pos = self.start_marker;
                } else {
                    self.start_marker = 0.0;
                    self.song_pos = 0.0;
                    // Stopped twice: the tape winds back too.
                    let t = &self.project.tape;
                    self.tape.locate(if t.loop_on { t.loop_in * self.sr as f64 } else { 0.0 });
                }
            }
            Command::SetPosition(p) => {
                let p = p.max(0.0);
                self.start_marker = p;
                self.song_pos = p;
                for rt in self.tracks.iter_mut() {
                    rt.release(Clock::Arrangement);
                }
            }
            Command::LaunchClip { track, slot } => {
                if let Some(ti) = self.project.track_index(track) {
                    let immediate = !self.playing;
                    if immediate && self.slot_has_clip(ti, slot) {
                        self.start_transport();
                    }
                    self.launch(ti, Some(slot), immediate);
                }
            }
            Command::StopTrack { track } => {
                if let Some(ti) = self.project.track_index(track) {
                    self.launch(ti, None, !self.playing);
                }
            }
            Command::LaunchScene(scene) => {
                let immediate = !self.playing;
                if immediate {
                    let any = (0..self.tracks.len()).any(|ti| self.slot_has_clip(ti, scene));
                    if !any {
                        return;
                    }
                    self.start_transport();
                }
                for ti in 0..self.tracks.len() {
                    let slot = if self.slot_has_clip(ti, scene) { Some(scene) } else { None };
                    self.launch(ti, slot, immediate);
                }
            }
            Command::StopAllClips => {
                let immediate = !self.playing;
                for ti in 0..self.tracks.len() {
                    self.launch(ti, None, immediate);
                }
            }
            Command::BackToArrangement => {
                for rt in self.tracks.iter_mut() {
                    rt.overridden = false;
                    rt.queued = None;
                    if rt.playing.take().is_some() {
                        rt.release(Clock::Session);
                    }
                }
            }
            Command::NoteOn { track, pitch, velocity } => {
                if let Some(rt) = self.project.track_index(track).and_then(|ti| self.tracks.get_mut(ti)) {
                    if rt.live.len() < rt.live.capacity() {
                        rt.live.push(Ev::on(0, pitch, velocity));
                    }
                }
            }
            Command::NoteOff { track, pitch } => {
                if let Some(rt) = self.project.track_index(track).and_then(|ti| self.tracks.get_mut(ti)) {
                    if rt.live.len() < rt.live.capacity() {
                        rt.live.push(Ev::off(0, pitch));
                    }
                }
            }
            Command::AllNotesOff => {
                for rt in self.tracks.iter_mut() {
                    rt.instrument.all_off();
                    rt.release(Clock::Session);
                    rt.release(Clock::Arrangement);
                    rt.reset_midi_fx();
                }
            }
            Command::AttachProcessor { instance, processor } => {
                let mut slot = Some(processor);
                for rt in self.tracks.iter_mut() {
                    if let InstState::Plugin { instance: i, proc, .. } = &mut rt.instrument {
                        if *i == instance {
                            if let Some(mut old) = proc.take() {
                                old.suspend();
                                let _ = self.garbage_tx.try_send(Garbage(GarbageInner::Processor(old)));
                            }
                            *proc = slot.take();
                            break;
                        }
                    }
                }
                if slot.is_some() {
                    let chains = self.tracks.iter_mut().map(|t| &mut t.devices).chain(self.returns.iter_mut().map(|r| &mut r.devices)).chain(std::iter::once(&mut self.master.devices));
                    'found: for devs in chains {
                        for (_, st) in devs.iter_mut() {
                            if let EffectState::Plugin { instance: i, proc } = st {
                                if *i == instance {
                                    if let Some(mut old) = proc.take() {
                                        old.suspend();
                                        let _ = self.garbage_tx.try_send(Garbage(GarbageInner::Processor(old)));
                                    }
                                    *proc = slot.take();
                                    break 'found;
                                }
                            }
                        }
                    }
                }
                if let Some(p) = slot {
                    if self.parked.len() >= 16 {
                        let (_, mut old) = self.parked.remove(0);
                        old.suspend();
                        self.trash(GarbageInner::Processor(old));
                    }
                    self.parked.push((instance, p));
                }
            }
            Command::Preview(s) => {
                if let Some((old, _)) = self.preview.take() {
                    self.trash(GarbageInner::Sample(old));
                }
                self.preview = s.map(|s| (s, 0.0));
            }
            Command::SetInput(input) => {
                self.status.input_channels.store(input.as_ref().map(|i| i.channels() as u32).unwrap_or(0), RELAXED);
                if let Some(old) = std::mem::replace(&mut self.input, input) {
                    self.trash(GarbageInner::Input(old));
                }
            }
            Command::SetCapture(on) => {
                self.capture = on;
                self.status.capturing.store(on, RELAXED);
            }
            Command::TapeLocate(secs) => self.tape.locate(secs * self.sr as f64),
            Command::TapeFollow(track) => self.tape.follow = track,
            Command::TapeBrake(on) => self.tape.brake = on,
            Command::TapeStutter(beats) => {
                let spb = self.samples_per_beat();
                self.tape.stutter(beats.map(|b| b * spb));
            }
            Command::TapeScrub(v) => self.tape.scrub = v,
            Command::TapeBend(ratio) => self.tape.bend = if ratio.is_finite() { ratio } else { 1.0 },
            Command::PitchBend { track, semis } => {
                if let Some(rt) = self.project.track_index(track).and_then(|ti| self.tracks.get_mut(ti)) {
                    rt.bend = semis;
                }
            }
            Command::ModWheel { track, value } => {
                if let Some(rt) = self.project.track_index(track).and_then(|ti| self.tracks.get_mut(ti)) {
                    rt.signal.modwheel = value.clamp(0.0, 1.0);
                }
            }
            Command::Joystick { track, x, y } => {
                if let Some(rt) = self.project.track_index(track).and_then(|ti| self.tracks.get_mut(ti)) {
                    rt.signal.joy = [x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0)];
                }
            }
            Command::Sustain { track, down } => {
                if let Some(InstState::Piano(s)) = self.project.track_index(track).and_then(|ti| self.tracks.get_mut(ti)).map(|rt| &mut rt.instrument) {
                    s.sustain(down);
                }
            }
            Command::SetPianoLibrary(lib) => {
                let mut old = Vec::new();
                for rt in self.tracks.iter_mut() {
                    if let InstState::Piano(s) = &mut rt.instrument {
                        old.extend(s.set_library(lib.clone()));
                    }
                }
                old.extend(std::mem::replace(&mut self.piano_lib, lib));
                for o in old {
                    self.trash(GarbageInner::PianoLibrary(o));
                }
            }
        }
    }

    fn start_transport(&mut self) {
        self.playing = true;
        self.song_pos = self.start_marker;
        self.session_pos = self.song_pos;
        // Clips that were playing when the transport stopped start over.
        let start = self.session_pos;
        for rt in self.tracks.iter_mut() {
            if let Some(pc) = &mut rt.playing {
                pc.start = start;
            }
        }
    }

    fn slot_has_clip(&self, ti: usize, slot: usize) -> bool {
        matches!(self.project.tracks.get(ti).and_then(|t| t.slots.get(slot)), Some(Some(_)))
    }

    fn launch(&mut self, ti: usize, slot: Option<usize>, immediate: bool) {
        let slot = slot.filter(|&s| self.slot_has_clip(ti, s));
        let q = self.project.launch_quantize.beats(self.project.beats_per_bar);
        let session_pos = self.session_pos;
        let Some(rt) = self.tracks.get_mut(ti) else { return };
        if immediate {
            rt.queued = None;
            if rt.playing.is_some() {
                rt.release(Clock::Session);
            }
            rt.playing = slot.map(|s| PlayingClip { slot: s, start: session_pos });
            if slot.is_some() {
                rt.overridden = true;
                rt.release(Clock::Arrangement);
            }
        } else {
            if slot.is_none() && rt.playing.is_none() {
                rt.queued = None;
                return;
            }
            let at = match q {
                None => session_pos,
                Some(g) => (session_pos / g - 1e-9).ceil() * g,
            };
            rt.queued = Some(Queued { slot, at });
        }
    }

    fn fire_due_launches(&mut self) {
        let pos = self.session_pos;
        for rt in self.tracks.iter_mut() {
            let Some(q) = rt.queued else { continue };
            if q.at > pos + 1e-9 {
                continue;
            }
            rt.release(Clock::Session);
            rt.playing = q.slot.map(|s| PlayingClip { slot: s, start: pos });
            if q.slot.is_some() {
                if !rt.overridden {
                    rt.release(Clock::Arrangement);
                }
                rt.overridden = true;
            }
            rt.queued = None;
        }
    }

    fn next_launch(&self) -> Option<f64> {
        self.tracks.iter().filter_map(|rt| rt.queued.map(|q| q.at)).reduce(f64::min)
    }

    /// Brings runtime state (voices, effect states, ...) in line with the
    /// current project snapshot, keyed by ids so state survives reordering.
    fn reconcile(&mut self) {
        let project = self.project.clone();
        let sr = self.sr;

        let same = project.tracks.len() == self.tracks.len() && project.tracks.iter().zip(&self.tracks).all(|(t, r)| t.id == r.id);
        if !same {
            let mut old = std::mem::take(&mut self.tracks);
            let mut fresh = Vec::with_capacity(project.tracks.len());
            for t in &project.tracks {
                match old.iter().position(|r| r.id == t.id) {
                    Some(pos) => fresh.push(old.swap_remove(pos)),
                    None => fresh.push(Box::new(TrackRt::new(t.id))),
                }
            }
            for mut rt in old {
                rt.instrument.suspend();
                for (_, st) in rt.devices.iter_mut() {
                    st.suspend();
                }
                self.trash(GarbageInner::Track(rt));
            }
            self.tracks = fresh;
        }

        let mut trash = Vec::new();
        for (t, rt) in project.tracks.iter().zip(self.tracks.iter_mut()) {
            if !rt.instrument.matches(t.instrument.as_ref()) {
                let mut old = std::mem::replace(&mut rt.instrument, InstState::for_instrument(t.instrument.as_ref(), sr));
                if let InstState::Piano(s) = &mut rt.instrument {
                    s.set_library(self.piano_lib.clone());
                }
                old.suspend();
                trash.push(GarbageInner::Instrument(old));
                if let InstState::Plugin { instance, proc, .. } = &mut rt.instrument {
                    if let Some(pos) = self.parked.iter().position(|(i, _)| i == instance) {
                        *proc = Some(self.parked.swap_remove(pos).1);
                    }
                }
            }
            sync_devices(&t.devices, &mut rt.devices, sr, &mut trash, &mut self.parked);
            let same_fx = t.midi_devices.len() == rt.midi_fx.len() && t.midi_devices.iter().zip(&rt.midi_fx).all(|(d, (id, st))| d.id == *id && st.matches(d.kind));
            if !same_fx {
                let mut old = std::mem::take(&mut rt.midi_fx);
                for d in &t.midi_devices {
                    match old.iter().position(|(id, st)| *id == d.id && st.matches(d.kind)) {
                        Some(pos) => rt.midi_fx.push(old.swap_remove(pos)),
                        None => rt.midi_fx.push((d.id, MidiFxState::new(d.kind))),
                    }
                }
            }
            if let Some(pc) = rt.playing {
                if !matches!(t.slots.get(pc.slot), Some(Some(_))) {
                    rt.playing = None;
                    rt.release(Clock::Session);
                }
            }
            if let Some(Queued { slot: Some(s), .. }) = rt.queued {
                if !matches!(t.slots.get(s), Some(Some(_))) {
                    rt.queued = None;
                }
            }
        }

        let same = project.returns.len() == self.returns.len() && project.returns.iter().zip(&self.returns).all(|(t, r)| t.id == r.id);
        if !same {
            let mut old = std::mem::take(&mut self.returns);
            for ret in &project.returns {
                match old.iter().position(|r| r.id == ret.id) {
                    Some(pos) => self.returns.push(old.swap_remove(pos)),
                    None => self.returns.push(BusRt::new(ret.id)),
                }
            }
            for b in old {
                for (_, mut st) in b.devices {
                    st.suspend();
                    trash.push(GarbageInner::Effect(st));
                }
            }
        }
        for (ret, rt) in project.returns.iter().zip(self.returns.iter_mut()) {
            sync_devices(&ret.devices, &mut rt.devices, sr, &mut trash, &mut self.parked);
        }
        sync_devices(&project.master.devices, &mut self.master.devices, sr, &mut trash, &mut self.parked);
        for g in trash {
            self.trash(g);
        }
        // Routing: processing order, what each audio track hears, where it goes.
        self.order.clear();
        self.order.extend(project.routing_order());
        for (ti, (t, rt)) in project.tracks.iter().zip(self.tracks.iter_mut()).enumerate() {
            rt.midi_src = project.midi_source(ti);
            rt.sources.clear();
            if t.kind == TrackKind::Audio {
                match t.input {
                    AudioInput::Ext { channel, stereo } => rt.sources.push(Source::Ext { channel: (channel as usize).min(MAX_INPUTS - 1), stereo }),
                    AudioInput::Resampling => rt.sources.push(Source::Resampling),
                    AudioInput::None | AudioInput::Track { .. } => {}
                }
                for (index, tap) in project.audio_sources(ti) {
                    rt.sources.push(Source::Track { index, tap });
                }
            }
            rt.to_master = match t.output {
                AudioOutput::Master => true,
                AudioOutput::Track(id) => !project.tracks.iter().any(|d| d.id == id && d.id != t.id && d.kind == TrackKind::Audio),
            };
        }
        self.mods.rebuild(&project);
    }

    fn process_block(&mut self, n: usize) {
        let mut done = 0;
        while done < n {
            let mut seg = n - done;
            if self.mods.active() {
                seg = seg.min(MOD_BLOCK);
            }
            let spb = self.samples_per_beat();
            if self.playing {
                self.fire_due_launches();
                if let Some(t) = self.next_launch() {
                    let k = ((t - self.session_pos) * spb).ceil();
                    if k >= 1.0 {
                        seg = seg.min(k as usize);
                    }
                }
                let p = &self.project;
                if p.loop_enabled && p.loop_end > p.loop_start + 0.01 && self.song_pos < p.loop_end {
                    let k = ((p.loop_end - self.song_pos) * spb).ceil().max(1.0) as usize;
                    seg = seg.min(k);
                }
            }
            self.render_segment(done, seg);
            self.clock += seg as u64;
            if self.playing {
                let adv = seg as f64 / spb;
                let before = self.song_pos;
                self.session_pos += adv;
                self.song_pos += adv;
                let p = &self.project;
                if p.loop_enabled && p.loop_end > p.loop_start + 0.01 && before < p.loop_end && self.song_pos >= p.loop_end - 1e-9 {
                    self.song_pos = p.loop_start + (self.song_pos - p.loop_end).max(0.0);
                    for rt in self.tracks.iter_mut() {
                        rt.release(Clock::Arrangement);
                    }
                }
            }
            done += seg;
        }
    }

    fn render_segment(&mut self, off: usize, seg: usize) {
        let project = self.project.clone();
        let sr = self.sr;
        let spb = sr as f64 * 60.0 / project.bpm.max(1.0);
        let playing = self.playing;
        let adv = if playing { seg as f64 / spb } else { 0.0 };
        let (s0, s1) = (self.session_pos, self.session_pos + adv);
        let (a0, a1) = (self.song_pos, self.song_pos + adv);
        let any_solo = project.tracks.iter().any(|t| t.mixer.solo);
        let status = &self.status;
        let clock = self.clock;
        // Effects that keep time follow the session clock (it starts from the song position).
        let ctx = FxCtx { sr, bpm: project.bpm as f32, beat: s0, playing };

        // Modulators run once per block, from what the tracks did last block.
        self.signals.clear();
        for rt in self.tracks.iter_mut() {
            if self.signals.len() < self.signals.capacity() {
                self.signals.push(rt.signal);
            }
            rt.signal.ons = 0;
        }
        // Clip envelopes: the value of each where its clip is now.
        self.envs.clear();
        if playing {
            for (track, rt) in project.tracks.iter().zip(self.tracks.iter()) {
                let at = match rt.playing {
                    Some(pc) => track.slots.get(pc.slot).and_then(|s| s.as_ref()).map(|c| (c, (s0 - pc.start).max(0.0).rem_euclid(c.length.max(1e-6)))),
                    None if !rt.overridden => track.arrangement.iter().find(|a| a.start <= a0 && a0 < a.end()).map(|a| (&a.clip, (a0 - a.start + a.offset).rem_euclid(a.clip.length.max(1e-6)))),
                    None => None,
                };
                if let Some((clip, pos)) = at {
                    for e in &clip.envelopes {
                        if let Some(v) = e.value_at(pos) {
                            if self.envs.len() < self.envs.capacity() {
                                self.envs.push((e.target, v));
                            }
                        }
                    }
                }
                // The arrangement's automation, while the track follows the arrangement.
                if rt.playing.is_none() && !rt.overridden {
                    for e in &track.automation {
                        if let Some(v) = e.value_at(a0) {
                            if self.envs.len() < self.envs.capacity() {
                                self.envs.push((e.target, v));
                            }
                        }
                    }
                }
            }
        }
        self.mods.tick(&project, &self.signals, ModClock { frames: seg, sr, samples_per_beat: spb, beat: s0 }, playing, &self.envs);
        let mods = &self.mods;

        for b in self.returns.iter_mut() {
            b.l[..seg].fill(0.0);
            b.r[..seg].fill(0.0);
        }
        self.master.l[..seg].fill(0.0);
        self.master.r[..seg].fill(0.0);

        // The audio interface input of this block.
        match &self.input {
            Some(input) => {
                input.pop(seg, &mut self.in_buf, MAX_BLOCK);
                for (c, pk) in status.input_peaks.iter().enumerate().take(input.channels()) {
                    pk.store_max(self.in_buf[c * MAX_BLOCK..c * MAX_BLOCK + seg].iter().fold(0.0f32, |m, x| m.max(x.abs())));
                }
            }
            None => self.in_buf.fill(0.0),
        }

        // The Key drawer's key: at once, or on the next bar's first beat
        // (notes before it in this block keep the old key).
        let (key_old, key_new, key_at) = if project.key_seq_on && playing && !project.key_seq.is_empty() {
            // The key sequence: a step per so many bars, changing on the bar.
            let step = project.key_seq_step_beats();
            let key_of = |pos: f64| project.key_seq_step(pos).and_then(|k| project.key_seq[k]);
            let next = (s0 / step - 1e-9).ceil() * step;
            let at = ((next - s0) * spb).round().max(0.0);
            if at < seg as f64 {
                (key_of(s0 - 1e-6), key_of(next + 1e-6), at as usize)
            } else {
                let k = key_of(s0);
                (k, k, usize::MAX)
            }
        } else {
            let want = project.play_key;
            if want == self.key_now {
                (want, want, usize::MAX)
            } else if !project.play_key_on_bar || !playing {
                self.key_now = want;
                (want, want, usize::MAX)
            } else {
                let bar = project.beats_per_bar_f().max(1.0);
                let next = (s0 / bar - 1e-9).ceil() * bar;
                let at = ((next - s0) * spb).round().max(0.0);
                if at < seg as f64 {
                    (self.key_now, want, at as usize)
                } else {
                    (self.key_now, self.key_now, usize::MAX)
                }
            }
        };
        if key_at == usize::MAX {
            self.key_now = key_new;
        }
        let sequencing = project.key_seq_on && playing && !project.key_seq.is_empty();
        status.key_waiting.store(!sequencing && project.play_key != self.key_now && key_at == usize::MAX, RELAXED);

        for oi in 0..self.order.len() {
            let ti = self.order[oi];
            let Some(track) = project.tracks.get(ti) else { continue };
            // What an audio track hears, from tracks processed before it.
            let listens = self.tracks.get(ti).is_some_and(|rt| !rt.sources.is_empty());
            if listens {
                gather_input(&self.tracks, ti, &self.in_buf, &self.resample, seg, &mut self.in_l, &mut self.in_r);
            }
            // The notes it takes from another track (processed before it).
            self.midi_scratch.clear();
            if let Some(src) = self.tracks.get(ti).and_then(|rt| rt.midi_src).and_then(|si| self.tracks.get(si)) {
                self.midi_scratch.extend_from_slice(&src.midi_out);
            }
            let Some(rt) = self.tracks.get_mut(ti) else { continue };
            let TrackRt { instrument, devices, placer, placer_idle, playing: pc, overridden, offs, events, fx_buf, live, midi_fx, fx_sig, sounding, shifted, signal, bend, l, r, gain_l, gain_r, sends, pre_l, pre_r, post_l, post_r, out_l, out_r, to_master, midi_out, .. } = &mut **rt;
            l[..seg].fill(0.0);
            r[..seg].fill(0.0);
            events.clear();

            if playing {
                let session_clip = pc.and_then(|pc| track.slots.get(pc.slot).and_then(|s| s.as_ref()).map(|c| (c, pc.start)));
                if let Some((clip, start)) = session_clip {
                    match &clip.content {
                        ClipContent::Midi { notes } => {
                            collect_notes(notes, clip.length, start, start, f64::INFINITY, s0, s1, spb, seg, Clock::Session, events, offs)
                        }
                        ClipContent::Audio(a) => render_audio(clip.length, a, start, start, f64::INFINITY, s0, spb, seg, l, r),
                    }
                } else if !*overridden {
                    for ac in &track.arrangement {
                        if ac.start >= a1 || ac.end() <= a0 {
                            continue;
                        }
                        let origin = ac.start - ac.offset;
                        match &ac.clip.content {
                            ClipContent::Midi { notes } => collect_notes(
                                notes,
                                ac.clip.length,
                                origin,
                                ac.start,
                                ac.end(),
                                a0,
                                a1,
                                spb,
                                seg,
                                Clock::Arrangement,
                                events,
                                offs,
                            ),
                            ClipContent::Audio(a) => render_audio(ac.clip.length, a, origin, ac.start, ac.end(), a0, spb, seg, l, r),
                        }
                    }
                }
            }

            // Note-offs that fall inside this segment (including ones just scheduled).
            let mut i = 0;
            while i < offs.len() {
                let o = offs[i];
                let (c0, c1) = match o.clock {
                    Clock::Session => (s0, s1),
                    Clock::Arrangement => (a0, a1),
                };
                if o.at < c1 || !playing {
                    let at = if o.at <= c0 { 0 } else { ((o.at - c0) * spb) as usize };
                    events.push(Ev { offset: at.min(seg.saturating_sub(1)) as u32, on: false, pitch: o.pitch, vel: 0.0 });
                    offs.swap_remove(i);
                } else {
                    i += 1;
                }
            }

            events.append(live);
            if !self.midi_scratch.is_empty() {
                // Like live notes: heard with Monitor In, or Auto when armed.
                let hears = match track.monitor {
                    Monitor::In => true,
                    Monitor::Auto => track.arm,
                    Monitor::Off => false,
                };
                if hears {
                    events.extend_from_slice(&self.midi_scratch);
                }
                // An armed track records them (the UI writes them into its clip).
                if track.arm && playing {
                    let clip = pc.and_then(|pc| track.slots.get(pc.slot).and_then(|s| s.as_ref()).map(|c| (pc.slot, pc.start, c.length.max(1.0 / 64.0))));
                    for e in &self.midi_scratch {
                        let (slot, pos) = match clip {
                            Some((slot, start, len)) => (slot as i32, (s0 + e.offset as f64 / spb - start).rem_euclid(len)),
                            None => (-1, 0.0),
                        };
                        let _ = self.midi_rec_tx.try_send(MidiRec { track: track.id, pitch: e.pitch, vel: e.vel, on: e.on, slot, pos });
                    }
                }
            }
            events.sort_unstable_by_key(|e| (e.offset, e.on));
            // MIDI effects transform the events before the instrument hears them.
            if track.midi_devices.iter().any(|d| d.enabled) {
                let drum_notes = track.instrument.as_ref().map(|i| i.drum_notes()).unwrap_or(0);
                let ctx = MidiCtx { now: clock, len: seg, samples_per_beat: spb, scale: project.scale, scale_on: project.scale_mode, beat: s0, playing, sr, drum_notes };
                // All notes free (scale mode off): the twelve of them, but for
                // what is in a key by nature.
                let free = MidiCtx { scale: crate::scale::Scale::new(project.scale.root, crate::scale::ScaleKind::Chromatic), ..ctx };
                let mut shown = false;
                for (dev, (_, st)) in track.midi_devices.iter().zip(midi_fx.iter_mut()) {
                    if !dev.enabled {
                        continue;
                    }
                    fx_buf.clear();
                    let params = mods.params(ModOwner::MidiDevice(dev.id)).unwrap_or(&dev.params);
                    let dctx = if !ctx.scale_on && !dev.kind.keeps_key() { &free } else { &ctx };
                    st.process(dev.kind, params, &dev.data, events, fx_buf, dctx);
                    fx_buf.sort_by_key(|e| (e.offset, e.on));
                    std::mem::swap(events, fx_buf);
                    if let Some(v) = st.visual() {
                        status.seqs.write(dev.id, v);
                        // The track's first one also for the grid.
                        if let (false, Some(ts)) = (shown, status.tracks.get(ti)) {
                            for (slot, val) in ts.seq.iter().zip(v) {
                                slot.store(val, RELAXED);
                            }
                            shown = true;
                        }
                    }
                }
            }
            // The chain changed or the transport stopped: notes started by
            // effects that are gone or bypassed would never get their note-off.
            // Changing where the notes come from also lets go of what is held.
            let route = match track.midi_in {
                MidiInput::Track(id) => id.wrapping_mul(0x9E37_79B9_7F4A_7C15),
                _ => 0,
            };
            let sig = fx_signature(&track.midi_devices) ^ route;
            if sig != *fx_sig {
                *fx_sig = sig;
                fx_buf.clear();
                for (pitch, n) in sounding.iter_mut().enumerate() {
                    for _ in 0..*n {
                        fx_buf.push(Ev::off(0, pitch as u8));
                    }
                    *n = 0;
                }
                if !fx_buf.is_empty() {
                    fx_buf.append(events);
                    std::mem::swap(events, fx_buf);
                }
            }

            for ev in events.iter() {
                if ev.on {
                    signal.ons += 1;
                    signal.pitch = ev.pitch;
                    signal.velocity = ev.vel;
                    signal.held = signal.held.saturating_add(1);
                } else {
                    signal.held = signal.held.saturating_sub(1);
                }
            }

            midi_out.clear();
            midi_out.extend_from_slice(events);

            if let Some(inst) = &track.instrument {
                let ov = mods.params(ModOwner::Instrument(track.id));
                instrument.set_bend(*bend);
                // Clocks in a modular patch keep time with the song; wind,
                // birds and nature play while it plays.
                match instrument {
                    InstState::Modular(m) => m.set_transport(ctx.bpm, ctx.beat, ctx.playing),
                    InstState::WindChimes(s) => s.playing = ctx.playing,
                    InstState::BeadsSynth(s) => {
                        s.playing = ctx.playing;
                        s.beat = ctx.beat;
                        s.bpm = ctx.bpm;
                    }
                    InstState::Melodica(s) => {
                        // Its echo keeps the song's time.
                        s.playing = ctx.playing;
                        s.beat = ctx.beat;
                        s.bpm = ctx.bpm;
                    }
                    InstState::Birds(s) => s.playing = ctx.playing,
                    InstState::Nature(s) => s.playing = ctx.playing,
                    // The pipes of a pan flute, a harp's key: the song's key now.
                    InstState::PanFlute(s) => s.key = key_new.filter(|_| !track.key_fixed).unwrap_or(project.scale).mask(),
                    InstState::Harmonica(s) => s.song_key = key_new.filter(|_| !track.key_fixed).unwrap_or(project.scale),
                    _ => {}
                }
                // The Key drawer moves what it plays into its key (not drums).
                let melodic = inst.transposes() && !track.key_fixed;
                let moved = |p: u8, s: i8| (p as i32 + s as i32).clamp(0, 127) as u8;
                let mut pos = 0;
                for ev in events.iter() {
                    let o = (ev.offset as usize).min(seg);
                    if o > pos {
                        instrument.render(inst, ov, &mut l[pos..o], &mut r[pos..o], sr);
                        pos = o;
                    }
                    let k = (ev.pitch & 127) as usize;
                    let n = &mut sounding[k];
                    if ev.on {
                        *n = n.saturating_add(1).min(8);
                        let key = if (ev.offset as usize) >= key_at { key_new } else { key_old };
                        let np = match key.filter(|k| melodic && *k != project.scale) {
                            Some(key) => crate::scale::convert_pitch(ev.pitch, &project.scale, &key),
                            None => ev.pitch,
                        };
                        shifted[k] = (np as i32 - ev.pitch as i32).clamp(-127, 127) as i8;
                        instrument.note_on(inst, ov, np, ev.vel, sr);
                    } else {
                        *n = n.saturating_sub(1);
                        instrument.note_off(inst, ov, moved(ev.pitch, shifted[k]));
                    }
                }
                if pos < seg {
                    instrument.render(inst, ov, &mut l[pos..seg], &mut r[pos..seg], sr);
                }
                if let Some(ts) = status.tracks.get(ti) {
                    ts.inst_scope.write(&l[..seg], &r[..seg]);
                    let mut held = [0u64; 2];
                    for (p, &n) in sounding.iter().enumerate() {
                        if n > 0 {
                            held[p / 64] |= 1 << (p % 64);
                        }
                    }
                    for (a, h) in ts.inst_held.iter().zip(held) {
                        a.store(h, RELAXED);
                    }
                    let mut vals = [0.0; INST_VALS];
                    if instrument.visual(&mut vals) {
                        for (a, v) in ts.inst_vals.iter().zip(vals) {
                            a.store(v);
                        }
                    }
                }
            }

            if listens {
                let (in_l, in_r) = (&self.in_l[..seg], &self.in_r[..seg]);
                if let Some(ts) = status.tracks.get(ti) {
                    ts.in_peak.store_max(in_l.iter().chain(in_r).fold(0.0f32, |m, x| m.max(x.abs())));
                }
                // Hearing the master inside the master would feed back.
                let monitor = track.input != AudioInput::Resampling
                    && match track.monitor {
                        Monitor::In => true,
                        Monitor::Auto => track.arm && pc.is_none(),
                        Monitor::Off => false,
                    };
                if monitor {
                    l[..seg].copy_from_slice(in_l);
                    r[..seg].copy_from_slice(in_r);
                }
                if self.capture && playing && track.arm {
                    let mut chunk = RecChunk { track: track.id, session_beat: s0, song_beat: a0, frames: seg, l: [0.0; REC_FRAMES], r: [0.0; REC_FRAMES] };
                    chunk.l[..seg].copy_from_slice(in_l);
                    chunk.r[..seg].copy_from_slice(in_r);
                    if self.rec_tx.try_send(chunk).is_err() {
                        status.rec_dropped.fetch_add(1, RELAXED);
                    }
                }
            }
            pre_l[..seg].copy_from_slice(&l[..seg]);
            pre_r[..seg].copy_from_slice(&r[..seg]);
            for (dev, (_, st)) in track.devices.iter().zip(devices.iter_mut()) {
                if dev.enabled {
                    let params = mods.params(ModOwner::Device(dev.id)).unwrap_or(&dev.params);
                    run_device(st, dev, params, &mut l[..seg], &mut r[..seg], &ctx, status);
                }
            }
            // The Map: where it stands, after its effects, before its fader.
            match &track.place {
                Some(place) => {
                    let st = placer.get_or_insert_with(|| Box::new(crate::dsp::fx::stage::Stage::new(sr)));
                    if *placer_idle {
                        *placer_idle = false;
                        st.clear();
                    }
                    let params = crate::dsp::fx::stage::map_params(&project.map, place);
                    st.process(&params, &mut l[..seg], &mut r[..seg], &ctx);
                }
                None => *placer_idle = true,
            }
            signal.level = l[..seg].iter().chain(&r[..seg]).fold(0.0f32, |m, x| m.max(x.abs()));
            post_l[..seg].copy_from_slice(&l[..seg]);
            post_r[..seg].copy_from_slice(&r[..seg]);

            let mm = mods.track_mix(ti);
            let muted = track.mixer.mute || (any_solo && !track.mixer.solo);
            let vol = if muted { 0.0 } else { fader_gain(mm.volume(track.mixer.volume_db)) * mm.gain };
            let (pl, pr) = pan_gains((mm.pan_base(track.mixer.pan) + mm.pan).clamp(-1.0, 1.0));
            let (pk_l, pk_r) = apply_gain_ramp(&mut l[..seg], &mut r[..seg], gain_l, gain_r, vol * pl, vol * pr);
            if let Some(ts) = status.tracks.get(ti) {
                ts.peak_l.store_max(pk_l);
                ts.peak_r.store_max(pk_r);
            }
            out_l[..seg].copy_from_slice(&l[..seg]);
            out_r[..seg].copy_from_slice(&r[..seg]);
            // A track sent to another track reaches the master through it.
            if *to_master {
                for i in 0..seg {
                    self.master.l[i] += l[i];
                    self.master.r[i] += r[i];
                }
            }
            for (j, ret) in self.returns.iter_mut().enumerate().take(MAX_RETURNS) {
                let send = mm.send_base(j, track.mixer.sends.get(j).copied().unwrap_or(0.0)) + mm.sends.get(j).copied().unwrap_or(0.0);
                let target = if muted { 0.0 } else { send_gain(send.clamp(0.0, 1.0)) };
                let cur = sends[j];
                if cur == 0.0 && target == 0.0 {
                    continue;
                }
                let step = (target - cur) / seg as f32;
                let mut g = cur;
                for i in 0..seg {
                    g += step;
                    ret.l[i] += l[i] * g;
                    ret.r[i] += r[i] * g;
                }
                sends[j] = target;
            }
        }

        // The tape plays into the master and records the armed track's source.
        {
            use crate::tape::TapeSource;
            let set = &project.tape;
            let source = set.armed.and_then(|k| set.tracks.get(k)).map(|t| t.source);
            let rec: Option<(&[f32], &[f32])> = match source {
                Some(TapeSource::Played) => self.tape.follow.and_then(|id| project.track_index(id)).and_then(|ti| self.tracks.get(ti)).map(|rt| (&rt.out_l[..seg], &rt.out_r[..seg])),
                Some(TapeSource::Track(id)) => project.track_index(id).and_then(|ti| self.tracks.get(ti)).map(|rt| (&rt.out_l[..seg], &rt.out_r[..seg])),
                Some(TapeSource::Input) => {
                    let right = if self.input.as_ref().is_some_and(|i| i.channels() > 1) { MAX_BLOCK } else { 0 };
                    Some((&self.in_buf[..seg], &self.in_buf[right..right + seg]))
                }
                None => None,
            };
            self.tape.process(&status.reel, set, sr, playing, rec, &mut self.master.l[..seg], &mut self.master.r[..seg]);
        }

        for (j, (ret, rt)) in project.returns.iter().zip(self.returns.iter_mut()).enumerate() {
            for (dev, (_, st)) in ret.devices.iter().zip(rt.devices.iter_mut()) {
                if dev.enabled {
                    let params = mods.params(ModOwner::Device(dev.id)).unwrap_or(&dev.params);
                    run_device(st, dev, params, &mut rt.l[..seg], &mut rt.r[..seg], &ctx, status);
                }
            }
            let mm = mods.return_mix(j);
            let vol = if ret.mixer.mute { 0.0 } else { fader_gain(mm.volume(ret.mixer.volume_db)) * mm.gain };
            let (pl, pr) = pan_gains((mm.pan_base(ret.mixer.pan) + mm.pan).clamp(-1.0, 1.0));
            let (pk_l, pk_r) = apply_gain_ramp(&mut rt.l[..seg], &mut rt.r[..seg], &mut rt.gain_l, &mut rt.gain_r, vol * pl, vol * pr);
            if let Some(ts) = status.returns.get(j) {
                ts.peak_l.store_max(pk_l);
                ts.peak_r.store_max(pk_r);
            }
            for i in 0..seg {
                self.master.l[i] += rt.l[i];
                self.master.r[i] += rt.r[i];
            }
        }

        {
            let m = &mut self.master;
            for (dev, (_, st)) in project.master.devices.iter().zip(m.devices.iter_mut()) {
                if dev.enabled {
                    let params = mods.params(ModOwner::Device(dev.id)).unwrap_or(&dev.params);
                    run_device(st, dev, params, &mut m.l[..seg], &mut m.r[..seg], &ctx, status);
                }
            }
            let vol = fader_gain(project.master.volume_db);
            let (pk_l, pk_r) = apply_gain_ramp(&mut m.l[..seg], &mut m.r[..seg], &mut m.gain_l, &mut m.gain_r, vol, vol);
            status.master.peak_l.store_max(pk_l);
            status.master.peak_r.store_max(pk_r);
            self.resample.write(&m.l[..seg], &m.r[..seg]);
        }

        // Metronome and preview go straight to the output, after the master.
        let mut click_at = None;
        if playing && project.metronome {
            let b = a0.ceil();
            if b < a1 {
                let at = ((b - a0) * spb) as usize;
                let accent = (b as i64).rem_euclid(project.beats_per_bar.max(1) as i64) == 0;
                click_at = Some((at.min(seg.saturating_sub(1)), accent));
            }
        }
        let mut preview_done = false;
        for i in 0..seg {
            if let Some((at, accent)) = click_at {
                if at == i {
                    self.click.trigger(accent, sr);
                }
            }
            let c = self.click.tick(sr);
            let mut pl = 0.0;
            let mut pr = 0.0;
            if let Some((s, pos)) = &mut self.preview {
                if *pos >= s.frames() as f64 {
                    preview_done = true;
                } else {
                    let (x, y) = s.read(*pos);
                    pl = x * 0.7;
                    pr = y * 0.7;
                    *pos += s.sample_rate as f64 / sr as f64;
                }
            }
            self.mix_l[off + i] = self.master.l[i] + c + pl;
            self.mix_r[off + i] = self.master.r[i] + c + pr;
        }
        if preview_done {
            if let Some((s, _)) = self.preview.take() {
                self.trash(GarbageInner::Sample(s));
            }
        }
        // The bar came: the new key is the key now.
        if key_at != usize::MAX {
            self.key_now = key_new;
        }
    }

    fn publish_status(&self) {
        let st = &self.status;
        st.playing.store(self.playing, RELAXED);
        st.song_pos.store(self.song_pos);
        st.session_pos.store(self.session_pos);
        st.preview_active.store(self.preview.is_some(), RELAXED);
        for (slot, v) in st.mods.iter().zip(self.mods.outputs.iter()) {
            slot.store(*v);
        }
        for (ti, (track, rt)) in self.project.tracks.iter().zip(&self.tracks).enumerate().take(MAX_TRACKS) {
            let ts = &st.tracks[ti];
            ts.playing_slot.store(rt.playing.map_or(-1, |p| p.slot as i32), RELAXED);
            ts.queued_slot.store(
                match rt.queued {
                    None => QUEUE_NONE,
                    Some(q) => q.slot.map_or(QUEUE_STOP, |s| s as i32),
                },
                RELAXED,
            );
            ts.overridden.store(rt.overridden, RELAXED);
            let mut pos = -1.0;
            let mut arr_id = 0;
            if let Some(pc) = rt.playing {
                if let Some(Some(clip)) = track.slots.get(pc.slot) {
                    pos = (self.session_pos - pc.start).max(0.0).rem_euclid(clip.length.max(1e-6));
                }
            } else if self.playing && !rt.overridden {
                if let Some(ac) = track.arrangement.iter().find(|a| a.start <= self.song_pos && self.song_pos < a.end()) {
                    pos = (self.song_pos - ac.start + ac.offset).rem_euclid(ac.clip.length.max(1e-6));
                    arr_id = ac.id;
                }
            }
            ts.clip_pos.store(pos);
            ts.arr_clip.store(arr_id, RELAXED);
        }
        for ts in st.tracks.iter().skip(self.tracks.len()) {
            if ts.playing_slot.load(RELAXED) == -1 && ts.queued_slot.load(RELAXED) == QUEUE_NONE {
                break;
            }
            ts.playing_slot.store(-1, RELAXED);
            ts.queued_slot.store(QUEUE_NONE, RELAXED);
        }
    }
}

fn sync_devices(devs: &[Device], rts: &mut Vec<(u64, EffectState)>, sr: f32, trash: &mut Vec<GarbageInner>, parked: &mut Vec<(u64, Box<dyn AudioProcessor>)>) {
    sync_chain(devs, rts, sr, trash, parked);
    // A convolution's file comes with the device.
    for (d, (_, st)) in devs.iter().zip(rts.iter_mut()) {
        if let EffectState::Convolution(c) = st {
            c.set_file(d.sample.as_ref());
        }
    }
    // Effects inside effects (the Glitch Sequencer's slots).
    for (d, (_, st)) in devs.iter().zip(rts.iter_mut()) {
        if let EffectState::Glitch(g) = st {
            sync_chain(&d.children, &mut g.children, sr, trash, parked);
        }
    }
}

fn sync_chain(devs: &[Device], rts: &mut Vec<(u64, EffectState)>, sr: f32, trash: &mut Vec<GarbageInner>, parked: &mut Vec<(u64, Box<dyn AudioProcessor>)>) {
    let same = devs.len() == rts.len() && devs.iter().zip(rts.iter()).all(|(d, (id, st))| d.id == *id && st.kind() == d.kind);
    if same {
        return;
    }
    let mut old = std::mem::take(rts);
    for d in devs {
        match old.iter().position(|(id, st)| *id == d.id && st.kind() == d.kind) {
            Some(pos) => rts.push(old.swap_remove(pos)),
            None => {
                let mut st = EffectState::new(d.kind, sr);
                if let (EffectState::Plugin { instance, proc }, Some(p)) = (&mut st, &d.plugin) {
                    // Its processor may have arrived before the device did.
                    *instance = p.instance;
                    if let Some(pos) = parked.iter().position(|(i, _)| *i == p.instance) {
                        *proc = Some(parked.swap_remove(pos).1);
                    }
                }
                rts.push((d.id, st));
            }
        }
    }
    for (_, mut st) in old {
        st.suspend();
        trash.push(GarbageInner::Effect(st));
    }
}

/// Runs an audio effect, sending what goes in and out of it to its display.
fn run_device(st: &mut EffectState, dev: &Device, params: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx, status: &EngineStatus) {
    let id = dev.id;
    if let Some(t) = st.taps() {
        t.before(&status.scopes, id, l, r);
    }
    match st {
        // The Glitch Sequencer plays its pattern and the effects in its slots.
        EffectState::Glitch(g) => g.process(params, &dev.children, &dev.data, l, r, ctx),
        _ => st.process(params, l, r, ctx),
    }
    if let Some(t) = st.taps() {
        t.after(&status.scopes, &status.meters, id, l, r);
    }
}

/// Applies a linearly ramped stereo gain and returns the peak levels.
fn apply_gain_ramp(l: &mut [f32], r: &mut [f32], gl: &mut f32, gr: &mut f32, tl: f32, tr: f32) -> (f32, f32) {
    let n = l.len().max(1) as f32;
    let (dl, dr) = ((tl - *gl) / n, (tr - *gr) / n);
    let (mut pl, mut pr) = (0.0f32, 0.0f32);
    for (a, b) in l.iter_mut().zip(r.iter_mut()) {
        *gl += dl;
        *gr += dr;
        *a *= *gl;
        *b *= *gr;
        pl = pl.max(a.abs());
        pr = pr.max(b.abs());
    }
    *gl = tl;
    *gr = tr;
    (pl, pr)
}

/// Schedules note-ons (and their note-offs) of a looping MIDI clip that fall
/// inside the window `[w0, w1)` on the given clock.
#[allow(clippy::too_many_arguments)]
fn collect_notes(
    notes: &[Note],
    len: f64,
    origin: f64,
    region_start: f64,
    region_end: f64,
    w0: f64,
    w1: f64,
    spb: f64,
    seg: usize,
    clock: Clock,
    events: &mut Vec<Ev>,
    offs: &mut Vec<PendingOff>,
) {
    let len = len.max(1.0 / 64.0);
    let lo = w0.max(region_start);
    let hi = w1.min(region_end);
    if hi <= lo {
        return;
    }
    let k0 = ((lo - origin) / len).floor().max(0.0) as i64;
    let k1 = ((hi - origin) / len).floor() as i64;
    for k in k0..=k1 {
        let base = origin + k as f64 * len;
        for (idx, n) in notes.iter().enumerate() {
            if n.start >= len || n.start < 0.0 {
                continue;
            }
            // Note operators: recurrence, chance, velocity spread, repeats.
            if !n.plays_on_loop(k) || (n.chance < 1.0 && crate::project::note_random(k, idx, n, 1) >= n.chance) {
                continue;
            }
            let vel = if n.vel_spread > 0.0 {
                (n.velocity + n.vel_spread * (crate::project::note_random(k, idx, n, 2) * 2.0 - 1.0)).clamp(0.02, 1.0)
            } else {
                n.velocity
            };
            let full = n.length.min(len - n.start).max(0.001);
            let reps = n.repeats.max(1) as usize;
            let step = full / reps as f64;
            for r in 0..reps {
                let t = base + n.start + r as f64 * step;
                if t < lo || t >= hi {
                    continue;
                }
                let at = (((t - w0) * spb) as usize).min(seg.saturating_sub(1));
                events.push(Ev { offset: at as u32, on: true, pitch: n.pitch, vel });
                let gate = if reps > 1 { step * 0.6 } else { full };
                let end = (t + gate.max(0.001)).min(region_end);
                offs.push(PendingOff { at: end, pitch: n.pitch, clock });
            }
        }
    }
}

/// Mixes a looping, beat-locked audio clip into `l`/`r`.
#[allow(clippy::too_many_arguments)]
fn render_audio(
    len: f64,
    a: &AudioClip,
    origin: f64,
    region_start: f64,
    region_end: f64,
    w0: f64,
    spb: f64,
    seg: usize,
    l: &mut [f32],
    r: &mut [f32],
) {
    let s = &a.sample;
    let src_fpb = s.sample_rate as f64 * 60.0 / a.bpm.max(1.0);
    let gain = crate::dsp::db_to_gain(a.gain_db);
    let len = len.max(1.0 / 64.0);
    let frames = s.frames() as f64;
    let loop_frames = (len * src_fpb).min(frames);
    const FADE: f64 = 48.0;
    for i in 0..seg {
        let g = w0 + i as f64 / spb;
        if g < region_start || g >= region_end {
            continue;
        }
        let local = (g - origin).rem_euclid(len);
        let pos = local * src_fpb;
        if pos >= frames {
            continue;
        }
        let edge = pos.min(loop_frames - pos).min(frames - pos);
        let e = (edge / FADE).clamp(0.0, 1.0) as f32;
        let (x, y) = s.read(pos);
        l[i] += x * gain * e;
        r[i] += y * gain * e;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(engine: &mut Engine, seconds: f32) -> Vec<f32> {
        let frames = (seconds * engine.sample_rate()) as usize;
        let mut out = vec![0.0; frames * 2];
        for chunk in out.chunks_mut(1024) {
            engine.process(chunk);
        }
        out
    }

    fn peak(buf: &[f32]) -> f32 {
        buf.iter().fold(0.0f32, |m, v| m.max(v.abs()))
    }

    /// Note-ons of a one-note clip over `loops` loops.
    fn note_ons(n: Note, loops: i64) -> Vec<(f64, f32)> {
        let len = 1.0;
        let (mut events, mut offs) = (Vec::new(), Vec::new());
        let mut out = Vec::new();
        // One window per sixteenth, 1 beat = 4800 samples.
        let w = 0.0625;
        let mut t = 0.0;
        while t < loops as f64 * len {
            events.clear();
            collect_notes(std::slice::from_ref(&n), len, 0.0, 0.0, 1e9, t, t + w, 4800.0, 300, Clock::Session, &mut events, &mut offs);
            for e in &events {
                out.push((t + e.offset as f64 / 4800.0, e.vel));
            }
            t += w;
        }
        out
    }

    #[test]
    fn note_operators_shape_what_plays() {
        let plain = Note::new(60, 0.0, 0.5, 0.8);
        assert_eq!(note_ons(plain.clone(), 8).len(), 8);
        // Ratchets: four quick hits per loop.
        let rep = Note { repeats: 4, ..plain.clone() };
        let ons = note_ons(rep, 2);
        assert_eq!(ons.len(), 8);
        assert!((ons[1].0 - 0.125).abs() < 1e-3, "{ons:?}");
        // Recurrence: the first of every four loops.
        let rec = Note { recur_len: 4, recur_mask: 0b0001, ..plain.clone() };
        let ons = note_ons(rec, 8);
        assert_eq!(ons.iter().map(|o| o.0.round() as i64).collect::<Vec<_>>(), vec![0, 4]);
        // Chance: about half of many loops, and the same each time.
        let half = Note { chance: 0.5, ..plain.clone() };
        let a = note_ons(half.clone(), 400).len();
        assert!((150..250).contains(&a), "{a}");
        assert_eq!(note_ons(half, 400).len(), a, "repeatable");
        // Velocity spread wanders around the velocity.
        let spread = Note { vel_spread: 0.3, ..plain };
        let vels: Vec<f32> = note_ons(spread, 50).iter().map(|o| o.1).collect();
        assert!(vels.iter().any(|v| *v > 0.85) && vels.iter().any(|v| *v < 0.75), "{vels:?}");
        assert!(vels.iter().all(|v| (0.5..=1.0).contains(v)));
    }

    #[test]
    fn notes_without_operators_save_as_before() {
        let n = Note::new(60, 0.0, 1.0, 0.8);
        let json = serde_json::to_string(&n).unwrap();
        assert!(!json.contains("chance") && !json.contains("repeats") && !json.contains("recur_len"), "{json}");
        let back: Note = serde_json::from_str(r#"{"pitch":60,"start":0.0,"length":1.0,"velocity":0.8}"#).unwrap();
        assert_eq!(back, n);
    }

    #[test]
    fn silent_when_stopped() {
        let (mut e, _h) = Engine::new(48000.0, Arc::new(crate::demo::demo_project()));
        let out = run(&mut e, 0.5);
        assert_eq!(peak(&out), 0.0);
    }

    #[test]
    fn scene_launch_plays_and_stops() {
        let (mut e, h) = Engine::new(48000.0, Arc::new(crate::demo::demo_project()));
        h.send(Command::LaunchScene(1));
        let out = run(&mut e, 2.0);
        assert!(e.is_playing());
        assert!(peak(&out) > 0.05, "demo scene should be audible");
        assert!(h.status.tracks[0].playing_slot.load(RELAXED) == 1);
        h.send(Command::Stop);
        run(&mut e, 0.1);
        assert!(!e.is_playing());
        // The clip stays launched for the next Play, like in Live.
        assert_eq!(h.status.tracks[0].playing_slot.load(RELAXED), 1);
    }

    #[test]
    fn the_ambient_set_plays_outside_and_in() {
        let (mut e, h) = Engine::new(48000.0, Arc::new(crate::demo::ambient_project()));
        // Silent until the song plays, the brook and birds included.
        assert_eq!(peak(&run(&mut e, 0.5)), 0.0);
        h.send(Command::LaunchScene(2));
        let out = run(&mut e, 6.0);
        let p = peak(&out);
        assert!(p > 0.05 && p < 1.5, "{p}");
        // Every track sounds at some point: the flute, the drone and the brook at once.
        for name in ["Brook", "Drone", "Pan Flute"] {
            let i = e.project.tracks.iter().position(|t| t.name == name).unwrap();
            assert!(h.status.tracks[i].peak_l.load() > 0.0 || h.status.tracks[i].peak_r.load() > 0.0, "{name}");
        }
    }

    #[test]
    fn launches_are_quantized_to_the_bar() {
        let (mut e, h) = Engine::new(48000.0, Arc::new(crate::demo::demo_project()));
        let drums = e.project.tracks[0].id;
        h.send(Command::LaunchClip { track: drums, slot: 0 });
        run(&mut e, 0.25);
        h.send(Command::LaunchClip { track: drums, slot: 1 });
        run(&mut e, 0.05);
        assert_eq!(h.status.tracks[0].playing_slot.load(RELAXED), 0);
        assert_eq!(h.status.tracks[0].queued_slot.load(RELAXED), 1);
        // One bar at the demo tempo is under two seconds.
        run(&mut e, 2.0);
        assert_eq!(h.status.tracks[0].playing_slot.load(RELAXED), 1);
    }

    #[test]
    fn arrangement_loops() {
        let mut p = crate::demo::demo_project();
        p.scenes_to_arrangement();
        p.loop_enabled = true;
        p.loop_start = 0.0;
        p.loop_end = 4.0;
        let (mut e, h) = Engine::new(48000.0, Arc::new(p));
        h.send(Command::Play);
        let out = run(&mut e, 5.0);
        assert!(peak(&out) > 0.05);
        assert!(e.song_pos() < 4.0 + 1e-6);
    }

    struct FakePlugin {
        notes: Arc<AtomicU32>,
        level: f32,
    }

    impl AudioProcessor for FakePlugin {
        fn process(&mut self, events: &[PluginEvent], left: &mut [f32], right: &mut [f32]) {
            for e in events {
                match e.kind {
                    PluginEventKind::NoteOn { .. } => {
                        self.notes.fetch_add(1, RELAXED);
                        self.level = 0.25;
                    }
                    _ => self.level = 0.0,
                }
            }
            for (l, r) in left.iter_mut().zip(right.iter_mut()) {
                *l += self.level;
                *r += self.level;
            }
        }
    }

    #[test]
    fn plugin_instruments_receive_notes_and_make_sound() {
        let mut p = crate::demo::demo_project();
        let instance = p.alloc_id();
        p.tracks[1].instrument = Some(Instrument::Plugin(PluginInstrument {
            instance,
            format: "clap".into(),
            plugin_id: "test.fake".into(),
            name: "Fake".into(),
            vendor: "Test".into(),
            path: "fake.clap".into(),
            state: None,
        }));
        for (i, t) in p.tracks.iter_mut().enumerate() {
            t.mixer.mute = i != 1;
        }
        let (mut e, h) = Engine::new(48000.0, Arc::new(p));
        let notes = Arc::new(AtomicU32::new(0));
        h.send(Command::AttachProcessor { instance, processor: Box::new(FakePlugin { notes: notes.clone(), level: 0.0 }) });
        h.send(Command::LaunchScene(1));
        let out = run(&mut e, 2.0);
        assert!(notes.load(RELAXED) >= 4, "bass notes should reach the plugin");
        assert!(peak(&out) > 0.1);
        assert_eq!(e.take_processors().len(), 1);
    }

    /// An effect plug-in that scales what goes through it.
    struct FakeEffect {
        gain: f32,
        calls: Arc<AtomicU32>,
    }

    impl AudioProcessor for FakeEffect {
        fn process(&mut self, _: &[PluginEvent], _: &mut [f32], _: &mut [f32]) {}

        fn process_effect(&mut self, left: &mut [f32], right: &mut [f32]) {
            self.calls.fetch_add(1, RELAXED);
            for (l, r) in left.iter_mut().zip(right.iter_mut()) {
                *l *= self.gain;
                *r *= self.gain;
            }
        }
    }

    #[test]
    fn effect_plugins_process_the_master() {
        let mut p = crate::demo::demo_project();
        let fake = PluginInstrument { instance: 0, format: "clap".into(), plugin_id: "test.fx".into(), name: "Fake FX".into(), vendor: "Test".into(), path: "fake.clap".into(), state: None };
        let (mut e, h) = Engine::new(48000.0, Arc::new(p.clone()));
        h.send(Command::LaunchScene(0));
        assert!(peak(&run(&mut e, 0.5)) > 0.05, "the demo plays");
        // The processor arrives before the device does (it waits, parked).
        let dev = p.make_plugin_device(fake);
        let instance = dev.plugin.as_ref().unwrap().instance;
        let calls = Arc::new(AtomicU32::new(0));
        h.send(Command::AttachProcessor { instance, processor: Box::new(FakeEffect { gain: 0.0, calls: calls.clone() }) });
        run(&mut e, 0.05);
        p.master.devices.push(dev);
        h.send(Command::SetProject(Arc::new(p.clone())));
        run(&mut e, 0.05);
        assert_eq!(peak(&run(&mut e, 0.5)), 0.0, "the effect silences the master");
        assert!(calls.load(RELAXED) > 10);
        // Taken out of the chain, its processor goes to the bin and the sound comes back.
        p.master.devices.clear();
        h.send(Command::SetProject(Arc::new(p.clone())));
        assert!(peak(&run(&mut e, 0.5)) > 0.05);
        assert_eq!(e.take_processors().len(), 0);
        assert_eq!(p.plugins().len(), 0);
    }

    #[test]
    fn midi_effects_run_before_the_instrument() {
        use crate::midi_fx::MidiFxKind;
        let mut p = crate::demo::demo_project();
        let instance = p.alloc_id();
        p.tracks[2].instrument = Some(Instrument::Plugin(PluginInstrument {
            instance,
            format: "clap".into(),
            plugin_id: "test.fake".into(),
            name: "Fake".into(),
            vendor: "Test".into(),
            path: "fake.clap".into(),
            state: None,
        }));
        // The chords clip holds 3-note chords for a bar each; an arpeggiator
        // at 1/16 turns every chord into 16 single notes per bar.
        let arp = p.make_midi_device(MidiFxKind::Arpeggiator);
        p.tracks[2].midi_devices.push(arp);
        let (mut e, h) = Engine::new(48000.0, Arc::new(p));
        let notes = Arc::new(AtomicU32::new(0));
        h.send(Command::AttachProcessor { instance, processor: Box::new(FakePlugin { notes: notes.clone(), level: 0.0 }) });
        h.send(Command::LaunchScene(1));
        // One bar at 124 BPM is ~1.94 s.
        run(&mut e, 1.9);
        let n = notes.load(RELAXED);
        assert!((14..=17).contains(&n), "arp notes in one bar: {n}");
        // Live notes go through the chain too.
        let before = notes.load(RELAXED);
        let track = e.project.tracks[2].id;
        h.send(Command::Stop);
        run(&mut e, 0.1);
        h.send(Command::NoteOn { track, pitch: 60, velocity: 0.8 });
        run(&mut e, 0.5);
        assert!(notes.load(RELAXED) > before + 2, "live arp");
    }

    #[test]
    fn sequencer_notes_are_released_on_bypass_and_stop() {
        use crate::midi_fx::MidiFxKind;
        let mut p = crate::demo::demo_project();
        let ti = 2;
        p.tracks[ti].arrangement.clear();
        let mut seq = p.make_midi_device(MidiFxKind::Drunk);
        seq.params[3] = 0.0; // never rest
        seq.params[5] = 1.0; // full gate: a note is (nearly) always sounding
        p.tracks[ti].midi_devices.push(seq);
        let (mut e, h) = Engine::new(48000.0, Arc::new(p.clone()));
        let sounding = |e: &Engine| e.tracks[ti].sounding.iter().map(|&n| n as u32).sum::<u32>();
        h.send(Command::Play);
        let mut seen = 0;
        for _ in 0..50 {
            run(&mut e, 0.01);
            seen += (sounding(&e) > 0) as u32;
        }
        assert!(seen > 40, "drunk notes sounding in {seen} of 50 checks");
        // Bypass mid-note: whatever it started must be released.
        p.tracks[ti].midi_devices[0].enabled = false;
        h.send(Command::SetProject(Arc::new(p.clone())));
        run(&mut e, 0.05);
        assert_eq!(sounding(&e), 0, "bypassed sequencer left notes on");
        p.tracks[ti].midi_devices[0].enabled = true;
        h.send(Command::SetProject(Arc::new(p.clone())));
        run(&mut e, 0.3);
        assert!(sounding(&e) > 0);
        h.send(Command::Stop);
        run(&mut e, 0.05);
        assert_eq!(sounding(&e), 0, "stop left notes on");
    }

    #[test]
    fn the_tape_records_what_the_played_track_plays() {
        let mut p = Project::new_default();
        let ti = p.tracks.iter().position(|t| matches!(t.instrument, Some(Instrument::Synth { .. }))).expect("a synth track");
        let id = p.tracks[ti].id;
        p.tape.armed = Some(2);
        let (mut e, h) = Engine::new(48000.0, Arc::new(p));
        h.status.reel.spool_or_make(2);
        h.send(Command::TapeFollow(Some(id)));
        h.send(Command::Play);
        h.send(Command::NoteOn { track: id, pitch: 48, velocity: 0.9 });
        run(&mut e, 0.5);
        let s = h.status.reel.spool(2).unwrap();
        assert!(h.status.reel.recording.load(RELAXED));
        assert!((20000..24001).contains(&s.end()), "half a second on tape: {}", s.end());
        let loudest = (0..s.overview_len()).map(|k| s.peak(k)).fold(0.0f32, f32::max);
        assert!(loudest > 0.05, "{loudest}");
        assert!(h.status.reel.spool(0).is_none(), "other tracks take no memory");
    }

    /// The period (in samples) of the left channel, by autocorrelation.
    fn period(out: &[f32]) -> f32 {
        let x: Vec<f32> = out.chunks(2).map(|c| c[0]).skip(4800).take(9600).collect();
        let corr = |lag: usize| -> f32 {
            let (mut s, mut e) = (0.0f32, 0.0f32);
            for i in 0..x.len() - lag {
                s += x[i] * x[i + lag];
                e += x[i] * x[i];
            }
            s / e.max(1e-9)
        };
        let c: Vec<f32> = (0..1400).map(corr).collect();
        // Past the first dip below zero, the first peak near the highest.
        let start = (1..1400).find(|&l| c[l] < 0.0).unwrap_or(20);
        let best = c[start..].iter().copied().fold(f32::MIN, f32::max);
        (start + 1..1399).find(|&l| c[l] > 0.85 * best && c[l] >= c[l - 1] && c[l] >= c[l + 1]).map(|l| l as f32).unwrap_or(0.0)
    }

    #[test]
    fn the_play_key_moves_what_plays_and_lets_go_where_it_played() {
        let mut p = Project::new_default();
        p.scale = crate::scale::Scale::new(0, crate::scale::ScaleKind::Major);
        let ti = p.tracks.iter().position(|t| matches!(t.instrument, Some(Instrument::Synth { .. }))).expect("a synth track");
        let id = p.tracks[ti].id;
        for t in p.tracks.iter_mut() {
            t.arrangement.clear();
        }
        let play = |p: &Project| {
            let (mut e, h) = Engine::new(48000.0, Arc::new(p.clone()));
            h.send(Command::NoteOn { track: id, pitch: 48, velocity: 0.9 });
            let out = run(&mut e, 0.4);
            (e, h, out)
        };
        // C (48) in C major; played in G major it is G (a fifth up, 3/2).
        let (_, _, low) = play(&p);
        p.play_key = Some(crate::scale::Scale::new(7, crate::scale::ScaleKind::Major));
        let (mut e, h, high) = play(&p);
        let (a, b) = (period(&low), period(&high));
        // G is the nearest root: a fourth down (or a fifth up).
        let ratio = a / b;
        assert!((ratio - 2f32.powf(-5.0 / 12.0)).abs() < 0.03 || (ratio - 2f32.powf(7.0 / 12.0)).abs() < 0.04, "G against C: periods {a} and {b}");
        // The key changes while the note is held: its note-off still finds it.
        let mut q = p.clone();
        q.play_key = None;
        h.send(Command::SetProject(Arc::new(q)));
        h.send(Command::NoteOff { track: id, pitch: 48 });
        run(&mut e, 2.0);
        let tail = run(&mut e, 0.2);
        assert!(peak(&tail) < 1e-3, "the note was let go: {}", peak(&tail));
    }

    #[test]
    fn a_track_can_stay_where_it_is_when_the_key_moves() {
        let mut p = Project::new_default();
        p.scale = crate::scale::Scale::new(0, crate::scale::ScaleKind::Major);
        let ti = p.tracks.iter().position(|t| matches!(t.instrument, Some(Instrument::Synth { .. }))).expect("a synth track");
        let id = p.tracks[ti].id;
        for t in p.tracks.iter_mut() {
            t.arrangement.clear();
        }
        let play = |p: &Project| {
            let (mut e, h) = Engine::new(48000.0, Arc::new(p.clone()));
            h.send(Command::NoteOn { track: id, pitch: 48, velocity: 0.9 });
            period(&run(&mut e, 0.4))
        };
        let home = play(&p);
        p.play_key = Some(crate::scale::Scale::new(7, crate::scale::ScaleKind::Major));
        p.tracks[ti].key_fixed = true;
        let fixed = play(&p);
        assert!((home / fixed - 1.0).abs() < 0.02, "it stays: {home} {fixed}");
    }

    #[test]
    fn with_all_notes_free_the_sequencers_that_pick_notes_take_all_twelve() {
        // A Turing machine on the synth, playing; which notes the synth holds.
        let held_classes = |scale_mode: bool, kind: crate::midi_fx::MidiFxKind| {
            let mut p = Project::new_default();
            p.scale = crate::scale::Scale::new(0, crate::scale::ScaleKind::Major);
            p.scale_mode = scale_mode;
            let ti = p.tracks.iter().position(|t| matches!(t.instrument, Some(Instrument::Synth { .. }))).expect("a synth track");
            for t in p.tracks.iter_mut() {
                t.arrangement.clear();
            }
            let dev = p.make_midi_device(kind);
            p.tracks[ti].midi_devices.push(dev);
            let (mut e, h) = Engine::new(48000.0, Arc::new(p));
            h.send(Command::Play);
            let mut classes = 0u16;
            for _ in 0..160 {
                run(&mut e, 0.05);
                let held = &h.status.tracks[ti].inst_held;
                for (w, a) in held.iter().enumerate() {
                    let bits = a.load(RELAXED);
                    for b in 0..64 {
                        if bits >> b & 1 == 1 {
                            classes |= 1 << ((w * 64 + b) % 12);
                        }
                    }
                }
            }
            classes
        };
        let c_major: u16 = [0, 2, 4, 5, 7, 9, 11].iter().fold(0, |m, &pc| m | 1 << pc);
        use crate::midi_fx::MidiFxKind;
        let in_key = held_classes(true, MidiFxKind::Turing);
        assert!(in_key != 0 && in_key & !c_major == 0, "{in_key:012b}");
        let free = held_classes(false, MidiFxKind::Turing);
        assert!(free & !c_major != 0, "some note out of the key: {free:012b}");
        // A composer stays in the key, notes free or not.
        let glass = held_classes(false, MidiFxKind::Glass);
        assert!(glass != 0 && glass & !c_major == 0, "{glass:012b}");
    }

    #[test]
    fn arrangement_automation_moves_the_volume() {
        // A synth note held; the volume line falls to silence at beat 2.
        let mut p = Project::new_default();
        let ti = p.tracks.iter().position(|t| matches!(t.instrument, Some(Instrument::Synth { .. }))).expect("a synth track");
        let id = p.tracks[ti].id;
        for t in p.tracks.iter_mut() {
            t.arrangement.clear();
        }
        let env = p.tracks[ti].automation_mut(crate::modulation::ModTarget::Volume(id));
        env.points = vec![(0.0, crate::project::db_to_fader_pos(0.0)), (2.0, crate::project::db_to_fader_pos(0.0)), (2.01, 0.0)];
        let spb = 60.0 / p.bpm as f32;
        let (mut e, h) = Engine::new(48000.0, Arc::new(p));
        h.send(Command::Play);
        h.send(Command::NoteOn { track: id, pitch: 60, velocity: 0.9 });
        run(&mut e, spb * 0.5);
        let before = track_level(&mut e, &h, ti, spb);
        run(&mut e, spb * 1.0);
        let after = track_level(&mut e, &h, ti, spb);
        assert!(before > 0.01, "playing before: {before}");
        assert!(after < 0.05 * before, "silent after: {after} (before {before})");
    }

    #[test]
    fn the_key_sequence_steps_on_the_bars() {
        use crate::scale::{Scale, ScaleKind};
        let mut p = Project::new_default();
        p.scale = Scale::new(0, ScaleKind::Major);
        let f = p.scale.degree_key(3);
        p.key_seq = vec![None, Some(f)];
        p.key_seq_bars = 1;
        p.key_seq_on = true;
        let bar_s = (p.beats_per_bar_f() * 60.0 / p.bpm) as f32;
        let (mut e, h) = Engine::new(48000.0, Arc::new(p));
        h.send(Command::Play);
        run(&mut e, bar_s * 0.5);
        assert_eq!(e.key_now, None, "bar 1: the song's key");
        run(&mut e, bar_s);
        assert_eq!(e.key_now, Some(f), "bar 2: F Lydian");
        run(&mut e, bar_s);
        assert_eq!(e.key_now, None, "bar 3: round again");
    }

    #[test]
    fn a_play_key_on_the_bar_waits_for_the_bar() {
        let mut p = Project::new_default();
        p.scale = crate::scale::Scale::new(0, crate::scale::ScaleKind::Major);
        p.play_key_on_bar = true;
        let (mut e, h) = Engine::new(48000.0, Arc::new(p.clone()));
        h.send(Command::Play);
        // A bar in, the key is asked for: it waits until the next bar.
        let bar_s = p.beats_per_bar_f() * 60.0 / p.bpm;
        run(&mut e, (bar_s * 1.25) as f32);
        p.play_key = Some(crate::scale::Scale::new(7, crate::scale::ScaleKind::Major));
        h.send(Command::SetProject(Arc::new(p.clone())));
        run(&mut e, 0.05);
        assert!(h.status.key_waiting.load(RELAXED), "it waits");
        assert_eq!(e.key_now, None);
        run(&mut e, (bar_s * 0.8) as f32);
        assert!(!h.status.key_waiting.load(RELAXED));
        assert_eq!(e.key_now, p.play_key, "the next bar brought it");
        // Stopped, a key comes at once.
        h.send(Command::Stop);
        run(&mut e, 0.05);
        p.play_key = None;
        h.send(Command::SetProject(Arc::new(p.clone())));
        run(&mut e, 0.05);
        assert_eq!(e.key_now, None);
    }

    #[test]
    fn baking_the_play_key_keeps_the_sound_and_leaves_drums() {
        let mut p = crate::demo::demo_project();
        let root = p.scale.root;
        let notes = |p: &Project, i: usize| -> Vec<u8> {
            p.tracks[i].slots.iter().flatten().filter_map(|c| if let crate::project::ClipContent::Midi { notes } = &c.content { Some(notes.iter().map(|n| n.pitch).collect::<Vec<_>>()) } else { None }).flatten().collect()
        };
        let drums = p.tracks.iter().position(|t| t.instrument.as_ref().is_some_and(|i| i.is_drums())).expect("drums");
        let keys = (0..p.tracks.len()).find(|&i| p.tracks[i].instrument.as_ref().is_some_and(|x| x.transposes()) && !notes(&p, i).is_empty()).expect("a melodic track with notes");
        let (d0, k0) = (notes(&p, drums), notes(&p, keys));
        let kind = p.scale.kind;
        p.play_key = Some(crate::scale::Scale::new((root + 3) % 12, kind));
        p.bake_play_key();
        assert_eq!(p.play_key, None);
        assert_eq!(p.scale.root, (root + 3) % 12);
        assert_eq!(notes(&p, drums), d0, "drums stay");
        assert_eq!(notes(&p, keys), k0.iter().map(|n| n + 3).collect::<Vec<_>>());
    }

    #[test]
    fn a_track_takes_the_notes_another_track_plays() {
        use crate::midi_fx::MidiFxKind;
        let mut p = crate::demo::demo_project();
        let (src, dst) = (2, 3);
        for ti in [src, dst] {
            p.tracks[ti].arrangement.clear();
        }
        let mut seq = p.make_midi_device(MidiFxKind::Drunk);
        seq.params[3] = 0.0; // never rest
        seq.params[5] = 1.0; // full gate
        p.tracks[src].midi_devices.push(seq);
        p.tracks[dst].midi_in = MidiInput::Track(p.tracks[src].id);
        p.tracks[dst].monitor = Monitor::In;
        assert_eq!(p.midi_source(dst), Some(src));
        assert!(!p.can_route_midi(dst, src), "no loops");
        assert!(p.routing_order().iter().position(|&t| t == src) < p.routing_order().iter().position(|&t| t == dst));
        let (mut e, h) = Engine::new(48000.0, Arc::new(p.clone()));
        let sounding = |e: &Engine, ti: usize| e.tracks[ti].sounding.iter().map(|&n| n as u32).sum::<u32>();
        h.send(Command::Play);
        let mut both = 0;
        for _ in 0..50 {
            run(&mut e, 0.01);
            both += (sounding(&e, src) > 0 && sounding(&e, dst) > 0) as u32;
        }
        assert!(both > 40, "the second track plays along in {both} of 50 checks");
        assert!(h.midi_recorded.try_recv().is_err(), "not armed: nothing to record");
        // Armed with a clip playing: the notes go to the UI to record, at their place in the clip.
        let clip_id = p.alloc_id();
        p.tracks[dst].slots[0] = Some(Clip::new_midi(clip_id, "Rec", p.tracks[dst].color, 4.0));
        p.tracks[dst].arm = true;
        h.send(Command::SetProject(Arc::new(p.clone())));
        h.send(Command::LaunchClip { track: p.tracks[dst].id, slot: 0 });
        // The launch waits for the next bar.
        run(&mut e, 4.0);
        let got: Vec<MidiRec> = std::iter::from_fn(|| h.midi_recorded.try_recv().ok()).collect();
        assert!(got.iter().filter(|m| m.on).count() > 3, "{}", got.len());
        assert!(got.iter().filter(|m| m.slot == 0).all(|m| (0.0..4.0).contains(&m.pos)));
        assert!(got.iter().any(|m| m.slot == 0));
        // Another source: what was held is let go.
        p.tracks[dst].midi_in = MidiInput::None;
        h.send(Command::SetProject(Arc::new(p.clone())));
        run(&mut e, 0.05);
        assert_eq!(sounding(&e, dst), 0, "rerouting left notes on");
        assert!(sounding(&e, src) > 0);
    }

    /// Average post-fader peak of a track over `seconds`, sampled every 10 ms.
    fn track_level(e: &mut Engine, h: &EngineHandle, ti: usize, seconds: f32) -> f32 {
        let n = (seconds * 100.0) as usize;
        let mut sum = 0.0;
        for _ in 0..n {
            run(e, 0.01);
            let (l, r) = h.status.tracks[ti].take_peaks();
            sum += l.max(r);
        }
        sum / n as f32
    }

    #[test]
    fn follower_sidechain_ducks_the_bass() {
        use crate::modulation::{ModKind, ModTarget};
        let plain = crate::demo::demo_project();
        let mut ducked = plain.clone();
        let mut f = ducked.make_modulator(ModKind::Follower);
        f.track = Some(ducked.tracks[0].id);
        f.params[0] = 18.0; // gain
        let fid = f.id;
        ducked.modulators.push(f);
        let bass = ducked.tracks[1].id;
        ducked.add_route(fid, ModTarget::Volume(bass), -1.0);
        let mut levels = Vec::new();
        for p in [plain, ducked] {
            let (mut e, h) = Engine::new(48000.0, Arc::new(p));
            h.send(Command::LaunchScene(1));
            run(&mut e, 0.5);
            levels.push(track_level(&mut e, &h, 1, 2.0));
        }
        assert!(levels[0] > 0.05, "bass plays: {levels:?}");
        assert!(levels[1] < levels[0] * 0.75, "ducked bass is quieter: {levels:?}");
    }

    #[test]
    fn lfo_modulates_an_instrument() {
        use crate::modulation::{ModKind, ModOwner, ModTarget};
        let plain = crate::demo::demo_project();
        let mut wobbly = plain.clone();
        let lfo = wobbly.make_modulator(ModKind::Lfo);
        let lid = lfo.id;
        wobbly.modulators.push(lfo);
        let bass = wobbly.tracks[1].id;
        let cutoff = crate::dsp::synth::SPECS.iter().position(|s| s.name == "Cutoff").unwrap() as u16;
        wobbly.add_route(lid, ModTarget::Param { owner: ModOwner::Instrument(bass), param: cutoff }, 0.8);
        let mut outs = Vec::new();
        for p in [plain, wobbly] {
            let (mut e, h) = Engine::new(48000.0, Arc::new(p));
            h.send(Command::LaunchScene(1));
            outs.push(run(&mut e, 1.5));
        }
        let diff: f32 = outs[0].iter().zip(&outs[1]).map(|(a, b)| (a - b).abs()).sum::<f32>() / outs[0].len() as f32;
        assert!(diff > 1e-3, "modulated cutoff changes the sound: {diff}");
        assert!(peak(&outs[1]) <= 1.0);
    }

    #[test]
    fn every_track_and_sequencer_reports_for_its_screen() {
        // In the Rack every device shows at once, so not only the selected
        // track's instrument may report what it does.
        let mut p = crate::demo::demo_project();
        let (a, b) = (p.make_midi_device(crate::midi_fx::MidiFxKind::Euclid), p.make_midi_device(crate::midi_fx::MidiFxKind::Euclid));
        let ids = (a.id, b.id);
        p.tracks[2].midi_devices.push(a);
        p.tracks[2].midi_devices.push(b);
        let (mut e, h) = Engine::new(48000.0, Arc::new(p));
        h.send(Command::LaunchScene(1));
        h.send(Command::Play);
        run(&mut e, 1.5);
        let sounding = (0..5)
            .filter(|&t| {
                let (mut l, mut r) = (Vec::new(), Vec::new());
                h.status.tracks[t].inst_scope.read(4096, &mut l, &mut r);
                l.iter().any(|v| v.abs() > 1e-4)
            })
            .count();
        assert!(sounding >= 3, "{sounding} tracks report their sound");
        // Both sequencers of a track, not only the first.
        assert!(h.status.seqs.read(ids.0).is_some() && h.status.seqs.read(ids.1).is_some());
    }

    #[test]
    fn clip_envelopes_move_parameters_while_the_clip_plays() {
        use crate::modulation::{ModOwner, ModTarget};
        let plain = crate::demo::demo_project();
        let bass = plain.tracks[1].id;
        // The bass clip of scene 2 with its volume all the way down...
        let mut quiet = plain.clone();
        let c = quiet.tracks[1].slots[1].as_mut().expect("a bass clip");
        c.envelopes.push(Envelope { target: ModTarget::Volume(bass), points: vec![(0.0, 0.0)] });
        // ...and one sweeping the synth's cutoff.
        let mut swept = plain.clone();
        let cutoff = crate::dsp::synth::SPECS.iter().position(|s| s.name == "Cutoff").unwrap() as u16;
        let c = swept.tracks[1].slots[1].as_mut().unwrap();
        c.envelopes.push(Envelope { target: ModTarget::Param { owner: ModOwner::Instrument(bass), param: cutoff }, points: vec![(0.0, 0.0), (2.0, 1.0), (4.0, 0.0)] });
        let mut levels = Vec::new();
        let mut outs = Vec::new();
        for p in [plain, quiet, swept] {
            let (mut e, h) = Engine::new(48000.0, Arc::new(p));
            h.send(Command::LaunchScene(1));
            run(&mut e, 0.3);
            levels.push(track_level(&mut e, &h, 1, 1.0));
            outs.push(run(&mut e, 1.0));
        }
        assert!(levels[0] > 0.02, "the bass plays: {}", levels[0]);
        assert!(levels[1] < 0.001, "its volume envelope at the bottom silences it: {}", levels[1]);
        let diff: f32 = outs[0].iter().zip(&outs[2]).map(|(a, b)| (a - b).abs()).sum::<f32>() / outs[0].len() as f32;
        assert!(diff > 1e-3, "the cutoff envelope changes the sound: {diff}");
        // An envelope's line: flat before and after its points, straight between.
        let e = Envelope { target: ModTarget::Volume(bass), points: vec![(1.0, 0.2), (3.0, 0.6)] };
        assert_eq!(e.value_at(0.0), Some(0.2));
        assert!((e.value_at(2.0).unwrap() - 0.4).abs() < 1e-6);
        assert_eq!(e.value_at(5.0), Some(0.6));
    }

    #[test]
    fn krab_macro_plays_clips_and_can_be_modulated() {
        use crate::modulation::{ModKind, ModOwner, ModTarget};
        let mut p = crate::demo::demo_project();
        p.tracks[1].instrument = Some(Instrument::macro_osc("Analog Bass"));
        let lfo = p.make_modulator(ModKind::Lfo);
        let lid = lfo.id;
        p.modulators.push(lfo);
        let bass = p.tracks[1].id;
        p.add_route(lid, ModTarget::Param { owner: ModOwner::Instrument(bass), param: crate::dsp::macro_osc::TIMBRE as u16 }, 0.5);
        let (mut e, h) = Engine::new(48000.0, Arc::new(p));
        h.send(Command::LaunchScene(1));
        run(&mut e, 0.3);
        let level = track_level(&mut e, &h, 1, 1.0);
        assert!(level > 0.02, "Plaits bass plays the clip: {level}");
        assert!(e.mods.params(ModOwner::Instrument(bass)).is_some(), "its parameters are modulated");
    }

    #[test]
    fn krab_elements_plays_clips_and_can_be_modulated() {
        use crate::modulation::{ModKind, ModOwner, ModTarget};
        let mut p = crate::demo::demo_project();
        p.tracks[2].instrument = Some(Instrument::elements("Marimba"));
        let lfo = p.make_modulator(ModKind::Lfo);
        let lid = lfo.id;
        p.modulators.push(lfo);
        let chords = p.tracks[2].id;
        p.add_route(lid, ModTarget::Param { owner: ModOwner::Instrument(chords), param: crate::dsp::elements_synth::BRIGHTNESS as u16 }, 0.5);
        let (mut e, h) = Engine::new(48000.0, Arc::new(p));
        h.send(Command::LaunchScene(1));
        run(&mut e, 0.3);
        let level = track_level(&mut e, &h, 2, 1.0);
        assert!(level > 0.003, "Elements plays the chords: {level}");
        assert!(e.mods.params(ModOwner::Instrument(chords)).is_some(), "its parameters are modulated");
    }

    fn drain(h: &EngineHandle, track: u64) -> Vec<RecChunk> {
        let mut v = Vec::new();
        while let Ok(c) = h.recorded.try_recv() {
            if c.track == track {
                v.push(c);
            }
        }
        v
    }

    fn rms(chunks: &[RecChunk]) -> f32 {
        let (mut s, mut n) = (0.0f32, 0usize);
        for c in chunks {
            for i in 0..c.frames {
                s += c.l[i] * c.l[i] + c.r[i] * c.r[i];
                n += 2;
            }
        }
        (s / n.max(1) as f32).sqrt()
    }

    /// The demo with an extra audio track listening to the bass.
    fn bass_into_audio_track(tap: Tap, monitor: Monitor) -> (Project, usize, u64) {
        let mut p = crate::demo::demo_project();
        let bass = p.tracks[1].id;
        let ai = p.add_audio_track();
        let t = &mut p.tracks[ai];
        t.input = AudioInput::Track { id: bass, tap };
        t.monitor = monitor;
        t.arm = true;
        let id = t.id;
        (p, ai, id)
    }

    #[test]
    fn a_track_records_another_track_without_gaps() {
        let (p, ai, id) = bass_into_audio_track(Tap::PostFx, Monitor::Off);
        let (mut e, h) = Engine::new(48000.0, Arc::new(p));
        h.send(Command::SetCapture(true));
        h.send(Command::LaunchScene(1));
        run(&mut e, 1.0);
        let chunks = drain(&h, id);
        let frames: usize = chunks.iter().map(|c| c.frames).sum();
        assert!((47000..=48100).contains(&frames), "one second recorded: {frames}");
        assert!(rms(&chunks) > 0.01, "the bass is in the recording: {}", rms(&chunks));
        // Blocks follow each other on the session clock.
        let spb = 48000.0 * 60.0 / e.project.bpm;
        for w in chunks.windows(2) {
            let expect = w[0].session_beat + w[0].frames as f64 / spb;
            assert!((w[1].session_beat - expect).abs() < 1e-6, "gap between blocks");
        }
        // Monitoring Off: the recording track itself stays silent.
        let lvl = track_level(&mut e, &h, ai, 0.5);
        assert!(lvl < 1e-6, "monitor off is silent: {lvl}");
        // Nothing is captured once recording stops.
        h.send(Command::SetCapture(false));
        run(&mut e, 0.1);
        drain(&h, id);
        run(&mut e, 0.2);
        assert!(drain(&h, id).is_empty());
    }

    #[test]
    fn monitoring_follows_in_auto_and_arm() {
        let (mut p, ai, _) = bass_into_audio_track(Tap::PostMixer, Monitor::In);
        p.tracks[ai].arm = false;
        let (mut e, h) = Engine::new(48000.0, Arc::new(p.clone()));
        h.send(Command::LaunchScene(1));
        run(&mut e, 0.3);
        assert!(track_level(&mut e, &h, ai, 1.0) > 0.02, "monitor In hears the bass");
        // Auto, not armed: silent; armed: hears it.
        p.tracks[ai].monitor = Monitor::Auto;
        h.send(Command::SetProject(Arc::new(p.clone())));
        run(&mut e, 0.1);
        assert!(track_level(&mut e, &h, ai, 0.5) < 1e-6);
        p.tracks[ai].arm = true;
        h.send(Command::SetProject(Arc::new(p.clone())));
        run(&mut e, 0.1);
        assert!(track_level(&mut e, &h, ai, 1.0) > 0.02);
        h.collect_garbage();
    }

    #[test]
    fn audio_to_sends_a_track_through_another() {
        let mut p = crate::demo::demo_project();
        let ai = p.add_audio_track();
        let dest = p.tracks[ai].id;
        p.tracks[ai].input = AudioInput::None;
        p.tracks[ai].monitor = Monitor::In;
        p.tracks[1].output = AudioOutput::Track(dest);
        // Only the bass plays, so the master hears exactly what goes through.
        for (i, t) in p.tracks.iter_mut().enumerate() {
            if i != 1 {
                t.mixer.mute = i != ai;
            }
        }
        let (mut e, h) = Engine::new(48000.0, Arc::new(p.clone()));
        h.send(Command::LaunchScene(1));
        run(&mut e, 0.3);
        assert!(track_level(&mut e, &h, ai, 1.0) > 0.02, "the bass comes out of the audio track");
        let master = |e: &mut Engine| {
            let out = run(e, 1.0);
            out.iter().fold(0.0f32, |m, x| m.max(x.abs()))
        };
        assert!(master(&mut e) > 0.02);
        // Turning the destination off silences the bass: it no longer goes
        // to the master directly.
        p.tracks[ai].mixer.mute = true;
        h.send(Command::SetProject(Arc::new(p.clone())));
        run(&mut e, 0.1);
        assert!(master(&mut e) < 1e-4);
        h.collect_garbage();
    }

    #[test]
    fn interface_input_is_monitored_and_recorded() {
        let mut p = Project::blank();
        let ai = p.add_audio_track();
        p.tracks[ai].arm = true;
        let id = p.tracks[ai].id;
        p.tracks[ai].input = AudioInput::Ext { channel: 1, stereo: false };
        let ring = Arc::new(InputRing::new(1 << 16, 2));
        let (mut e, h) = Engine::new(48000.0, Arc::new(p));
        h.send(Command::SetInput(Some(ring.clone())));
        h.send(Command::SetCapture(true));
        h.send(Command::Play);
        // Input 2 carries a 440 Hz tone, input 1 silence.
        let mut t = 0usize;
        let mut feed = |frames: usize| {
            let x: Vec<f32> = (0..frames).flat_map(|_| {
                t += 1;
                [0.0, 0.5 * (2.0 * std::f32::consts::PI * 440.0 * t as f32 / 48000.0).sin()]
            }).collect();
            ring.push(&x, 2);
        };
        let mut out = vec![0.0; 2048];
        for _ in 0..40 {
            feed(1024);
            e.process(&mut out);
        }
        assert_eq!(h.status.input_channels.load(RELAXED), 2);
        let chunks = drain(&h, id);
        assert!(rms(&chunks) > 0.2, "the tone is recorded: {}", rms(&chunks));
        // Mono input lands on both sides; the armed track monitors it (Auto).
        let c = chunks.last().unwrap();
        assert_eq!(c.l[..c.frames], c.r[..c.frames]);
        assert!(out.iter().any(|x| x.abs() > 0.1), "monitored to the output");
        h.collect_garbage();
    }

    #[test]
    fn resampling_records_the_master_without_feedback() {
        let mut p = crate::demo::demo_project();
        let ai = p.add_audio_track();
        p.tracks[ai].input = AudioInput::Resampling;
        p.tracks[ai].monitor = Monitor::In;
        p.tracks[ai].arm = true;
        let id = p.tracks[ai].id;
        let (mut e, h) = Engine::new(48000.0, Arc::new(p));
        h.send(Command::SetCapture(true));
        h.send(Command::LaunchScene(1));
        run(&mut e, 1.0);
        assert!(rms(&drain(&h, id)) > 0.02, "the mix is recorded");
        assert!(track_level(&mut e, &h, ai, 0.3) < 1e-6, "but not played back into itself");
    }

    #[test]
    fn scopes_receive_audio() {
        let mut p = crate::demo::demo_project();
        let scope = p.make_device(crate::dsp::effects::EffectKind::Scope);
        let scope_id = scope.id;
        p.tracks[1].devices.push(scope);
        let (mut e, h) = Engine::new(48000.0, Arc::new(p));
        h.send(Command::LaunchScene(1));
        run(&mut e, 0.5);
        // The Oscilloscope passes audio through and fills its buffer.
        let buf = h.status.scopes.find(scope_id).expect("the scope claimed a buffer");
        let (mut l, mut r) = (Vec::new(), Vec::new());
        buf.read(4800, &mut l, &mut r);
        assert_eq!(l.len(), 4800);
        assert!(l.iter().chain(&r).any(|x| x.abs() > 0.01), "the scope sees the bass");
        assert!(track_level(&mut e, &h, 1, 1.0) > 0.02, "and the bass still plays");
    }

    #[test]
    fn stop_and_play_resume_the_session_clips() {
        let p = crate::demo::demo_project();
        let (mut e, h) = Engine::new(48000.0, Arc::new(p));
        h.send(Command::LaunchScene(1));
        run(&mut e, 0.5);
        h.send(Command::Stop);
        run(&mut e, 2.0);
        // Still launched, but silent while stopped (once the release is over).
        assert_eq!(h.status.tracks[1].playing_slot.load(RELAXED), 1);
        h.status.tracks[1].take_peaks();
        assert!(track_level(&mut e, &h, 1, 0.3) < 1e-6);
        h.send(Command::Play);
        run(&mut e, 0.1);
        assert!(track_level(&mut e, &h, 1, 1.0) > 0.02, "Play brings the clips back");
        // Stop All Clips (while stopped) does clear them.
        h.send(Command::Stop);
        h.send(Command::StopAllClips);
        run(&mut e, 2.0);
        assert_eq!(h.status.tracks[1].playing_slot.load(RELAXED), -1);
        h.send(Command::Play);
        run(&mut e, 0.1);
        h.status.tracks[1].take_peaks();
        assert!(track_level(&mut e, &h, 1, 0.5) < 1e-6);
    }

    #[test]
    fn analog_plays_clips_and_can_be_modulated() {
        use crate::modulation::{ModKind, ModOwner, ModTarget};
        let mut p = crate::demo::demo_project();
        p.tracks[1].instrument = Some(Instrument::analog("Moog Bass"));
        let lfo = p.make_modulator(ModKind::Lfo);
        let lid = lfo.id;
        p.modulators.push(lfo);
        let bass = p.tracks[1].id;
        p.add_route(lid, ModTarget::Param { owner: ModOwner::Instrument(bass), param: crate::dsp::analog::CUTOFF as u16 }, 0.5);
        let (mut e, h) = Engine::new(48000.0, Arc::new(p));
        h.send(Command::LaunchScene(1));
        run(&mut e, 0.3);
        let level = track_level(&mut e, &h, 1, 1.0);
        assert!(level > 0.02, "Analog bass plays the clip: {level}");
        assert!(e.mods.params(ModOwner::Instrument(bass)).is_some(), "its parameters are modulated");
    }

    #[test]
    fn project_swap_keeps_running_state() {
        let p = crate::demo::demo_project();
        let (mut e, h) = Engine::new(48000.0, Arc::new(p.clone()));
        h.send(Command::LaunchScene(0));
        run(&mut e, 0.2);
        let mut q = p.clone();
        q.tracks[1].mixer.volume_db = -12.0;
        let id = q.alloc_id();
        let mut t = q.tracks[0].clone();
        t.id = id;
        q.tracks.insert(0, t);
        h.send(Command::SetProject(Arc::new(q)));
        run(&mut e, 0.2);
        // The original drum track moved to index 1 and must still be playing.
        assert_eq!(h.status.tracks[1].playing_slot.load(RELAXED), 0);
        assert_eq!(h.status.tracks[0].playing_slot.load(RELAXED), -1);
        h.collect_garbage();
    }
}
