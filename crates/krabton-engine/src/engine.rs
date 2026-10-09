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
use crate::dsp::braids_synth::BraidsSynthState;
use crate::dsp::elements_synth::ElementsSynthState;
use crate::dsp::macro_osc::MacroState;
use crate::dsp::sampler::SamplerState;
use crate::dsp::synth::SynthState;
use crate::dsp::pan_gains;
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
        }
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
    /// Gives the audio half of a hosted plugin to the track whose instrument
    /// is `Instrument::Plugin` with this instance id.
    AttachProcessor { instance: u64, processor: Box<dyn AudioProcessor> },
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
}

/// The UI side of the engine.
pub struct EngineHandle {
    pub commands: Sender<Command>,
    pub garbage: Receiver<Garbage>,
    pub status: Arc<EngineStatus>,
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
    Plugin { instance: u64, proc: Option<Box<dyn AudioProcessor>>, pending: Vec<PluginEvent> },
}

impl InstState {
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
        ) || matches!((self, i), (InstState::Plugin { instance, .. }, Some(Instrument::Plugin(p))) if *instance == p.instance)
    }

    /// `ov` replaces the instrument's parameters (modulation).
    fn note_on(&mut self, inst: &Instrument, ov: Option<&[f32]>, pitch: u8, vel: f32, sr: f32) {
        match (self, inst) {
            (InstState::Synth(s), Instrument::Synth { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Drums(s), Instrument::Drums { pads, .. }) => s.note_on(pads, pitch, vel, sr),
            (InstState::Sampler(s), Instrument::Sampler { params, sample }) => s.note_on(ov.unwrap_or(params), sample.as_ref(), pitch, vel),
            (st @ InstState::Plugin { .. }, Instrument::Plugin(_)) => st.push_plugin_event(pitch, PluginEventKind::NoteOn { velocity: vel }),
            (InstState::Fm(s), Instrument::Fm { voice, op_mask, params, .. }) => s.note_on(ov.unwrap_or(params), voice, *op_mask, pitch, vel),
            (InstState::Macro(s), Instrument::Macro { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Braids(s), Instrument::Braids { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Elements(s), Instrument::Elements { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            (InstState::Analog(s), Instrument::Analog { params, .. }) => s.note_on(ov.unwrap_or(params), pitch, vel),
            _ => {}
        }
    }

    fn note_off(&mut self, inst: &Instrument, ov: Option<&[f32]>, pitch: u8) {
        match (self, inst) {
            (InstState::Synth(s), Instrument::Synth { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Sampler(s), Instrument::Sampler { .. }) => s.note_off(pitch),
            (st @ InstState::Plugin { .. }, Instrument::Plugin(_)) => st.push_plugin_event(pitch, PluginEventKind::NoteOff),
            (InstState::Fm(s), Instrument::Fm { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Macro(s), Instrument::Macro { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Braids(s), Instrument::Braids { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Elements(s), Instrument::Elements { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
            (InstState::Analog(s), Instrument::Analog { params, .. }) => s.note_off(ov.unwrap_or(params), pitch),
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
            InstState::None => {}
        }
    }

    fn render(&mut self, inst: &Instrument, ov: Option<&[f32]>, l: &mut [f32], r: &mut [f32], sr: f32) {
        match (self, inst) {
            (InstState::Synth(s), Instrument::Synth { params, .. }) => s.render(ov.unwrap_or(params), l, r, sr),
            (InstState::Drums(s), Instrument::Drums { pads, .. }) => s.render(pads, l, r, sr),
            (InstState::Sampler(s), Instrument::Sampler { params, sample }) => s.render(ov.unwrap_or(params), sample.as_ref(), l, r, sr),
            (InstState::Fm(s), Instrument::Fm { voice, params, .. }) => s.render(ov.unwrap_or(params), voice, l, r, sr),
            (InstState::Macro(s), Instrument::Macro { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Braids(s), Instrument::Braids { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Elements(s), Instrument::Elements { params, .. }) => s.render(ov.unwrap_or(params), l, r),
            (InstState::Analog(s), Instrument::Analog { params, .. }) => s.render(ov.unwrap_or(params), l, r),
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
    /// Notes and level for the modulators.
    signal: TrackSignal,
    l: Vec<f32>,
    r: Vec<f32>,
    gain_l: f32,
    gain_r: f32,
    sends: [f32; MAX_RETURNS],
}

impl TrackRt {
    fn new(id: u64) -> Self {
        Self {
            id,
            instrument: InstState::None,
            devices: Vec::new(),
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
            signal: TrackSignal::default(),
            l: vec![0.0; MAX_BLOCK],
            r: vec![0.0; MAX_BLOCK],
            gain_l: 0.0,
            gain_r: 0.0,
            sends: [0.0; MAX_RETURNS],
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
    /// Per-track signals handed to the modulators each control block.
    signals: Vec<TrackSignal>,
}

impl Engine {
    pub fn new(sample_rate: f32, project: Arc<Project>) -> (Engine, EngineHandle) {
        let (cmd_tx, cmd_rx) = bounded(8192);
        let (garbage_tx, garbage_rx) = bounded(8192);
        let status = Arc::new(EngineStatus::new());
        status.sample_rate.store(sample_rate);
        let mut e = Engine {
            sr: sample_rate,
            project,
            cmd_rx,
            garbage_tx,
            status: status.clone(),
            playing: false,
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
            signals: Vec::with_capacity(MAX_TRACKS),
        };
        e.reconcile();
        (e, EngineHandle { commands: cmd_tx, garbage: garbage_rx, status })
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
                    for rt in self.tracks.iter_mut() {
                        rt.playing = None;
                        rt.queued = None;
                        rt.release(Clock::Session);
                        rt.release(Clock::Arrangement);
                        rt.reset_midi_fx();
                    }
                    self.song_pos = self.start_marker;
                } else {
                    self.start_marker = 0.0;
                    self.song_pos = 0.0;
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
        }
    }

    fn start_transport(&mut self) {
        self.playing = true;
        self.song_pos = self.start_marker;
        self.session_pos = self.song_pos;
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
                self.trash(GarbageInner::Track(rt));
            }
            self.tracks = fresh;
        }

        let mut trash = Vec::new();
        for (t, rt) in project.tracks.iter().zip(self.tracks.iter_mut()) {
            if !rt.instrument.matches(t.instrument.as_ref()) {
                let mut old = std::mem::replace(&mut rt.instrument, InstState::for_instrument(t.instrument.as_ref(), sr));
                old.suspend();
                trash.push(GarbageInner::Instrument(old));
                if let InstState::Plugin { instance, proc, .. } = &mut rt.instrument {
                    if let Some(pos) = self.parked.iter().position(|(i, _)| i == instance) {
                        *proc = Some(self.parked.swap_remove(pos).1);
                    }
                }
            }
            sync_devices(&t.devices, &mut rt.devices, sr, &mut trash);
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
                for (_, st) in b.devices {
                    trash.push(GarbageInner::Effect(st));
                }
            }
        }
        for (ret, rt) in project.returns.iter().zip(self.returns.iter_mut()) {
            sync_devices(&ret.devices, &mut rt.devices, sr, &mut trash);
        }
        sync_devices(&project.master.devices, &mut self.master.devices, sr, &mut trash);
        for g in trash {
            self.trash(g);
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
        let ctx = FxCtx { sr, bpm: project.bpm as f32 };
        let any_solo = project.tracks.iter().any(|t| t.mixer.solo);
        let status = &self.status;
        let clock = self.clock;

        // Modulators run once per block, from what the tracks did last block.
        self.signals.clear();
        for rt in self.tracks.iter_mut() {
            if self.signals.len() < self.signals.capacity() {
                self.signals.push(rt.signal);
            }
            rt.signal.ons = 0;
        }
        self.mods.tick(&project, &self.signals, ModClock { frames: seg, sr, samples_per_beat: spb, beat: s0 }, playing);
        let mods = &self.mods;

        for b in self.returns.iter_mut() {
            b.l[..seg].fill(0.0);
            b.r[..seg].fill(0.0);
        }
        self.master.l[..seg].fill(0.0);
        self.master.r[..seg].fill(0.0);

        for (ti, track) in project.tracks.iter().enumerate() {
            let Some(rt) = self.tracks.get_mut(ti) else { break };
            let TrackRt { instrument, devices, playing: pc, overridden, offs, events, fx_buf, live, midi_fx, fx_sig, sounding, signal, l, r, gain_l, gain_r, sends, .. } = &mut **rt;
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
            events.sort_unstable_by_key(|e| (e.offset, e.on));
            // MIDI effects transform the events before the instrument hears them.
            if track.midi_devices.iter().any(|d| d.enabled) {
                let ctx = MidiCtx { now: clock, len: seg, samples_per_beat: spb, scale: project.scale, scale_on: project.scale_mode, beat: s0, playing };
                let mut shown = false;
                for (dev, (_, st)) in track.midi_devices.iter().zip(midi_fx.iter_mut()) {
                    if !dev.enabled {
                        continue;
                    }
                    fx_buf.clear();
                    let params = mods.params(ModOwner::MidiDevice(dev.id)).unwrap_or(&dev.params);
                    st.process(dev.kind, params, &dev.data, events, fx_buf, &ctx);
                    fx_buf.sort_by_key(|e| (e.offset, e.on));
                    std::mem::swap(events, fx_buf);
                    if !shown {
                        if let (Some(v), Some(ts)) = (st.visual(), status.tracks.get(ti)) {
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
            let sig = fx_signature(&track.midi_devices);
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

            if let Some(inst) = &track.instrument {
                let ov = mods.params(ModOwner::Instrument(track.id));
                let mut pos = 0;
                for ev in events.iter() {
                    let o = (ev.offset as usize).min(seg);
                    if o > pos {
                        instrument.render(inst, ov, &mut l[pos..o], &mut r[pos..o], sr);
                        pos = o;
                    }
                    let n = &mut sounding[(ev.pitch & 127) as usize];
                    if ev.on {
                        *n = n.saturating_add(1).min(8);
                        instrument.note_on(inst, ov, ev.pitch, ev.vel, sr);
                    } else {
                        *n = n.saturating_sub(1);
                        instrument.note_off(inst, ov, ev.pitch);
                    }
                }
                if pos < seg {
                    instrument.render(inst, ov, &mut l[pos..seg], &mut r[pos..seg], sr);
                }
            }

            for (dev, (_, st)) in track.devices.iter().zip(devices.iter_mut()) {
                if dev.enabled {
                    let params = mods.params(ModOwner::Device(dev.id)).unwrap_or(&dev.params);
                    st.process(params, &mut l[..seg], &mut r[..seg], &ctx);
                }
            }
            signal.level = l[..seg].iter().chain(&r[..seg]).fold(0.0f32, |m, x| m.max(x.abs()));

            let mm = mods.track_mix(ti);
            let muted = track.mixer.mute || (any_solo && !track.mixer.solo);
            let vol = if muted { 0.0 } else { fader_gain(track.mixer.volume_db) * mm.gain };
            let (pl, pr) = pan_gains((track.mixer.pan + mm.pan).clamp(-1.0, 1.0));
            let (pk_l, pk_r) = apply_gain_ramp(&mut l[..seg], &mut r[..seg], gain_l, gain_r, vol * pl, vol * pr);
            if let Some(ts) = status.tracks.get(ti) {
                ts.peak_l.store_max(pk_l);
                ts.peak_r.store_max(pk_r);
            }
            for i in 0..seg {
                self.master.l[i] += l[i];
                self.master.r[i] += r[i];
            }
            for (j, ret) in self.returns.iter_mut().enumerate().take(MAX_RETURNS) {
                let send = track.mixer.sends.get(j).copied().unwrap_or(0.0) + mm.sends.get(j).copied().unwrap_or(0.0);
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

        for (j, (ret, rt)) in project.returns.iter().zip(self.returns.iter_mut()).enumerate() {
            for (dev, (_, st)) in ret.devices.iter().zip(rt.devices.iter_mut()) {
                if dev.enabled {
                    let params = mods.params(ModOwner::Device(dev.id)).unwrap_or(&dev.params);
                    st.process(params, &mut rt.l[..seg], &mut rt.r[..seg], &ctx);
                }
            }
            let mm = mods.return_mix(j);
            let vol = if ret.mixer.mute { 0.0 } else { fader_gain(ret.mixer.volume_db) * mm.gain };
            let (pl, pr) = pan_gains((ret.mixer.pan + mm.pan).clamp(-1.0, 1.0));
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
                    st.process(params, &mut m.l[..seg], &mut m.r[..seg], &ctx);
                }
            }
            let vol = fader_gain(project.master.volume_db);
            let (pk_l, pk_r) = apply_gain_ramp(&mut m.l[..seg], &mut m.r[..seg], &mut m.gain_l, &mut m.gain_r, vol, vol);
            status.master.peak_l.store_max(pk_l);
            status.master.peak_r.store_max(pk_r);
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

fn sync_devices(devs: &[Device], rts: &mut Vec<(u64, EffectState)>, sr: f32, trash: &mut Vec<GarbageInner>) {
    let same = devs.len() == rts.len() && devs.iter().zip(rts.iter()).all(|(d, (id, st))| d.id == *id && st.kind() == d.kind);
    if same {
        return;
    }
    let mut old = std::mem::take(rts);
    for d in devs {
        match old.iter().position(|(id, st)| *id == d.id && st.kind() == d.kind) {
            Some(pos) => rts.push(old.swap_remove(pos)),
            None => rts.push((d.id, EffectState::new(d.kind, sr))),
        }
    }
    for (_, st) in old {
        trash.push(GarbageInner::Effect(st));
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
        for n in notes {
            if n.start >= len || n.start < 0.0 {
                continue;
            }
            let t = base + n.start;
            if t < lo || t >= hi {
                continue;
            }
            let at = (((t - w0) * spb) as usize).min(seg.saturating_sub(1));
            events.push(Ev { offset: at as u32, on: true, pitch: n.pitch, vel: n.velocity });
            let end = (t + n.length.min(len - n.start).max(0.001)).min(region_end);
            offs.push(PendingOff { at: end, pitch: n.pitch, clock });
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
        assert_eq!(h.status.tracks[0].playing_slot.load(RELAXED), -1);
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
