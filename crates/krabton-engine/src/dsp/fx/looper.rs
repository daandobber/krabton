//! Looper: a loop pedal on any track. One button records a loop, then
//! overdubs onto it, then plays it (and overdubs again, plays again...), as
//! a pedal under the foot does. Stop, Clear, Undo and Redo of the last
//! overdub, Multiply (twice as long, the loop played twice), Speed in
//! semitones (half speed is an octave down and twice as long), Reverse, and
//! Feedback: how much of the loop is left after each pass of an overdub.
//!
//! With the song (Tempo: Follow) a recording starts and ends on the next bar
//! or beat (Quantize), and the loop then follows the song's tempo, faster or
//! slower. Set & Follow lets the first loop, recorded with the transport
//! stopped, set the song's tempo: a whole number of bars. Free is a loop of
//! its own length. Length records so many bars and then goes on by itself.
//!
//! The loop's memory ([`LoopMemory`]) is made by the UI (two minutes of
//! stereo, and as much again for undo) and shared with the engine as atomics,
//! as the tape is: the engine plays and records it, the UI draws it, saves it
//! next to the set and makes clips of it, and nothing is allocated on the
//! audio thread. Long jobs (undo, multiply) are done a slice per block.

use crate::dsp::db_to_gain;
use crate::dsp::effects::FxCtx;
use crate::engine::{AtomicF32, AtomicF64};
use crate::params::{ParamSpec, Unit};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;

const RELAXED: Ordering = Ordering::Relaxed;

pub const PEDAL: usize = 0;
pub const AFTER_REC: usize = 1;
pub const TEMPO: usize = 2;
pub const QUANTIZE: usize = 3;
pub const LENGTH: usize = 4;
pub const FEEDBACK: usize = 5;
pub const SPEED: usize = 6;
pub const REVERSE: usize = 7;
pub const MONITOR: usize = 8;
pub const VOLUME: usize = 9;

pub const AFTER_RECS: &[&str] = &["Overdub", "Play"];
pub const TEMPOS: &[&str] = &["Free", "Follow", "Set & Follow"];
pub const QUANTIZES: &[&str] = &["None", "1/16", "1/8", "1/4", "1/2", "1 Bar"];
const QUANTIZE_BEATS: [f64; 6] = [0.0, 0.25, 0.5, 1.0, 2.0, 0.0];
pub const LENGTHS: &[&str] = &["Free", "1 Bar", "2 Bars", "4 Bars", "8 Bars", "16 Bars"];
const LENGTH_BARS: [f64; 6] = [0.0, 1.0, 2.0, 4.0, 8.0, 16.0];
pub const MONITORS: &[&str] = &["Always", "Never", "Rec/OD Only"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::toggle("Pedal", "Control", false),
    ParamSpec::choice("After Rec", "Control", AFTER_RECS, 0),
    ParamSpec::choice("Tempo", "Tempo", TEMPOS, 1),
    ParamSpec::choice("Quantize", "Tempo", QUANTIZES, 5),
    ParamSpec::choice("Length", "Tempo", LENGTHS, 0),
    ParamSpec::lin("Feedback", "Loop", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::int("Speed", "Loop", -36.0, 36.0, 0.0, Unit::Semi),
    ParamSpec::toggle("Reverse", "Loop", false),
    ParamSpec::choice("Monitor", "Output", MONITORS, 0),
    ParamSpec::lin("Volume", "Output", -36.0, 6.0, 0.0, Unit::Db),
];

/// The longest loop, in seconds.
pub const MAX_SECONDS: f64 = 120.0;
/// Frames an undo or a multiply gets through per block.
const JOB_SLICE: usize = 65_536;
/// How fast playing fades in and out on Play and Stop (per sample, from 0 to 1).
const FADE: f32 = 1.0 / 240.0;
const QUEUE: usize = 32;

/// What the looper is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LoopState {
    Empty = 0,
    Recording = 1,
    Overdubbing = 2,
    Playing = 3,
    Stopped = 4,
}

impl LoopState {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => LoopState::Recording,
            2 => LoopState::Overdubbing,
            3 => LoopState::Playing,
            4 => LoopState::Stopped,
            _ => LoopState::Empty,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            LoopState::Empty => "Empty",
            LoopState::Recording => "Recording",
            LoopState::Overdubbing => "Overdub",
            LoopState::Playing => "Playing",
            LoopState::Stopped => "Stopped",
        }
    }
}

/// What the UI asks of the looper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LoopAction {
    /// The big button: record, overdub, play, overdub, play...
    Pedal = 1,
    Record = 2,
    Overdub = 3,
    Play = 4,
    Stop = 5,
    Clear = 6,
    Undo = 7,
    Redo = 8,
    Multiply = 9,
}

impl LoopAction {
    fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            1 => LoopAction::Pedal,
            2 => LoopAction::Record,
            3 => LoopAction::Overdub,
            4 => LoopAction::Play,
            5 => LoopAction::Stop,
            6 => LoopAction::Clear,
            7 => LoopAction::Undo,
            8 => LoopAction::Redo,
            9 => LoopAction::Multiply,
            _ => return None,
        })
    }
}

/// Zeroed atomics, from memory the system hands out already zero (so the
/// pages of a long loop are only taken once something is recorded there).
fn zeroed<T>(n: usize) -> Box<[T]> {
    // Only for atomics of 32 and 64 bits: the same size, alignment and bits
    // as the integers, and zero is a valid value for them.
    assert!(std::mem::size_of::<T>() == 4 || std::mem::size_of::<T>() == 8);
    if std::mem::size_of::<T>() == 4 {
        let v = std::mem::ManuallyDrop::new(vec![0u32; n]);
        unsafe { Vec::from_raw_parts(v.as_ptr() as *mut T, v.len(), v.capacity()) }.into_boxed_slice()
    } else {
        let v = std::mem::ManuallyDrop::new(vec![0u64; n]);
        unsafe { Vec::from_raw_parts(v.as_ptr() as *mut T, v.len(), v.capacity()) }.into_boxed_slice()
    }
}

/// A looper's audio and what it tells the UI. Made by the UI (never on the
/// audio thread), shared with the engine.
pub struct LoopMemory {
    rate: f32,
    l: Box<[AtomicU32]>,
    r: Box<[AtomicU32]>,
    /// The frames the last overdub changed, as they were (and after an
    /// undo, as the overdub left them, for redo).
    undo_l: Box<[AtomicU32]>,
    undo_r: Box<[AtomicU32]>,
    /// Which frames the last overdub changed, a bit each.
    touched: Box<[AtomicU64]>,
    /// What the engine says: its state, the loop's length (frames; while
    /// recording, what is recorded so far), where it plays (frames), the
    /// layers, the tempo it was recorded at (0: free), whether a press waits
    /// for its beat, whether undo (bit 0) and redo (bit 1) can be done, and
    /// a count that goes up whenever the audio changes.
    pub state: AtomicU8,
    pub len: AtomicUsize,
    pub pos: AtomicF64,
    pub layers: AtomicU32,
    pub loop_bpm: AtomicF32,
    pub waiting: AtomicBool,
    pub undo: AtomicU8,
    pub generation: AtomicU32,
    /// The song's tempo as the first loop under Set & Follow sets it (0:
    /// none); the UI takes it.
    pub set_tempo: AtomicF32,
    /// What the UI says: beats in a bar.
    pub bar_beats: AtomicF32,
    /// The generation last saved next to the set (the UI's).
    pub saved: AtomicU32,
    queue: [AtomicU8; QUEUE],
    head: AtomicU32,
    tail: AtomicU32,
}

impl LoopMemory {
    /// Room for `seconds` of stereo at `rate`.
    pub fn new(rate: f32, seconds: f64) -> Self {
        let frames = (seconds * rate as f64).max(1.0) as usize;
        Self {
            rate,
            l: zeroed(frames),
            r: zeroed(frames),
            undo_l: zeroed(frames),
            undo_r: zeroed(frames),
            touched: zeroed(frames.div_ceil(64)),
            state: AtomicU8::new(LoopState::Empty as u8),
            len: AtomicUsize::new(0),
            pos: AtomicF64::new(0.0),
            layers: AtomicU32::new(0),
            loop_bpm: AtomicF32::new(0.0),
            waiting: AtomicBool::new(false),
            undo: AtomicU8::new(0),
            generation: AtomicU32::new(0),
            set_tempo: AtomicF32::new(0.0),
            bar_beats: AtomicF32::new(4.0),
            saved: AtomicU32::new(0),
            queue: std::array::from_fn(|_| AtomicU8::new(0)),
            head: AtomicU32::new(0),
            tail: AtomicU32::new(0),
        }
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }

    /// The most frames a loop can have.
    pub fn capacity(&self) -> usize {
        self.l.len()
    }

    pub fn state(&self) -> LoopState {
        LoopState::from_u8(self.state.load(RELAXED))
    }

    #[inline]
    pub fn read(&self, i: usize) -> (f32, f32) {
        match (self.l.get(i), self.r.get(i)) {
            (Some(a), Some(b)) => (f32::from_bits(a.load(RELAXED)), f32::from_bits(b.load(RELAXED))),
            _ => (0.0, 0.0),
        }
    }

    #[inline]
    fn write(&self, i: usize, l: f32, r: f32) {
        if let (Some(a), Some(b)) = (self.l.get(i), self.r.get(i)) {
            a.store(l.to_bits(), RELAXED);
            b.store(r.to_bits(), RELAXED);
        }
    }

    fn is_touched(&self, i: usize) -> bool {
        self.touched.get(i / 64).is_some_and(|w| w.load(RELAXED) & (1 << (i % 64)) != 0)
    }

    /// Keeps frame `i` as it is for undo, the first time the layer changes it.
    fn keep(&self, i: usize) {
        let Some(w) = self.touched.get(i / 64) else { return };
        let bit = 1u64 << (i % 64);
        let v = w.load(RELAXED);
        if v & bit == 0 {
            w.store(v | bit, RELAXED);
            self.undo_l[i].store(self.l[i].load(RELAXED), RELAXED);
            self.undo_r[i].store(self.r[i].load(RELAXED), RELAXED);
        }
    }

    /// Swaps frame `i` with its copy for undo, if the last layer changed it.
    fn swap(&self, i: usize) {
        if self.is_touched(i) {
            let a = self.l[i].swap(self.undo_l[i].load(RELAXED), RELAXED);
            self.undo_l[i].store(a, RELAXED);
            let b = self.r[i].swap(self.undo_r[i].load(RELAXED), RELAXED);
            self.undo_r[i].store(b, RELAXED);
        }
    }

    fn forget_layer(&self, frames: usize) {
        for w in self.touched.iter().take(frames.div_ceil(64)) {
            w.store(0, RELAXED);
        }
    }

    /// Asks the engine for something (it is done at the start of its next
    /// block). False when too much is waiting already.
    pub fn press(&self, a: LoopAction) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        if head.wrapping_sub(tail) as usize >= QUEUE {
            return false;
        }
        self.queue[head as usize % QUEUE].store(a as u8, RELAXED);
        self.head.store(head.wrapping_add(1), Ordering::Release);
        true
    }

    fn take(&self) -> Option<LoopAction> {
        let tail = self.tail.load(Ordering::Acquire);
        if tail == self.head.load(Ordering::Acquire) {
            return None;
        }
        let a = self.queue[tail as usize % QUEUE].load(RELAXED);
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        LoopAction::from_u8(a)
    }

    /// The loop as it is now (left and right), for a clip or a file.
    pub fn frames(&self) -> (Vec<f32>, Vec<f32>) {
        let n = self.len.load(RELAXED).min(self.capacity());
        if self.state() == LoopState::Recording || self.state() == LoopState::Empty {
            return (Vec::new(), Vec::new());
        }
        (0..n).map(|i| self.read(i)).unzip()
    }

    /// The loudest of `n` stretches of the loop (0..1, a few frames read in
    /// each), for drawing.
    pub fn peaks(&self, n: usize) -> Vec<f32> {
        let len = self.len.load(RELAXED).min(self.capacity());
        if len == 0 || n == 0 {
            return vec![0.0; n];
        }
        (0..n)
            .map(|k| {
                let a = k * len / n;
                let b = ((k + 1) * len / n).max(a + 1).min(len);
                let step = ((b - a) / 48).max(1);
                (a..b).step_by(step).map(|i| {
                    let (l, r) = self.read(i);
                    l.abs().max(r.abs())
                }).fold(0.0f32, f32::max).min(1.0)
            })
            .collect()
    }

    /// Fills an unused memory with a loop (from a saved set): stopped, at
    /// `bpm` (0: free). Before the engine has it.
    pub fn load(&self, l: &[f32], r: &[f32], bpm: f32) {
        let n = l.len().min(r.len()).min(self.capacity());
        for i in 0..n {
            self.write(i, l[i], r[i]);
        }
        self.len.store(n, RELAXED);
        self.loop_bpm.store(bpm);
        self.layers.store(if n > 0 { 1 } else { 0 }, RELAXED);
        self.state.store(if n > 0 { LoopState::Stopped } else { LoopState::Empty } as u8, RELAXED);
        let g = self.generation.fetch_add(1, RELAXED).wrapping_add(1);
        self.saved.store(g, RELAXED);
    }
}

/// A looper's memory, carried by its device (never saved, never compared).
#[derive(Clone, Default)]
pub struct LoopHandle(pub Option<Arc<LoopMemory>>);

impl PartialEq for LoopHandle {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl std::fmt::Debug for LoopHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LoopHandle({})", if self.0.is_some() { "memory" } else { "none" })
    }
}

/// A press that waits for its beat.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Pending {
    StartRecording,
    EndRecording,
    Play,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Job {
    None,
    /// Undo or redo: frames swapped with their copies, from `at` on.
    Swap { at: usize, done: usize },
    /// Multiply: the first `half` frames copied after themselves.
    Copy { half: usize, done: usize },
}

/// The looper on the audio thread.
pub struct Looper {
    mem: Option<Arc<LoopMemory>>,
    /// A memory replaced, waiting to be handed away (never dropped here).
    retired: Option<Arc<LoopMemory>>,
    state: LoopState,
    len: usize,
    pos: f64,
    /// Frames recorded so far, and how many to record (0: until pressed).
    rec: usize,
    rec_target: usize,
    pending: Option<(Pending, f64)>,
    loop_bpm: f32,
    layers: u32,
    last_write: Option<usize>,
    pedal: bool,
    job: Job,
    can_undo: bool,
    can_redo: bool,
    gain: f32,
}

impl Default for Looper {
    fn default() -> Self {
        Self::new()
    }
}

impl Looper {
    pub fn new() -> Self {
        Self {
            mem: None,
            retired: None,
            state: LoopState::Empty,
            len: 0,
            pos: 0.0,
            rec: 0,
            rec_target: 0,
            pending: None,
            loop_bpm: 0.0,
            layers: 0,
            last_write: None,
            pedal: false,
            job: Job::None,
            can_undo: false,
            can_redo: false,
            gain: 0.0,
        }
    }

    /// Takes the memory the device carries (on a new snapshot): a new one
    /// starts from what it holds, stopped.
    pub fn attach(&mut self, mem: Option<&Arc<LoopMemory>>) {
        let same = match (&self.mem, mem) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        if let Some(old) = std::mem::replace(&mut self.mem, mem.cloned()) {
            self.retired = Some(old);
        }
        let Some(m) = &self.mem else {
            *self = Self { retired: self.retired.take(), ..Self::new() };
            return;
        };
        let len = m.len.load(RELAXED).min(m.capacity());
        let has = len > 0 && !matches!(m.state(), LoopState::Empty | LoopState::Recording);
        self.state = if has { LoopState::Stopped } else { LoopState::Empty };
        self.len = if has { len } else { 0 };
        self.pos = 0.0;
        self.rec = 0;
        self.pending = None;
        self.loop_bpm = m.loop_bpm.load();
        self.layers = if has { m.layers.load(RELAXED).max(1) } else { 0 };
        self.job = Job::None;
        self.can_undo = false;
        self.can_redo = false;
        self.gain = 0.0;
        self.publish();
    }

    /// A memory let go of, for the engine to drop elsewhere.
    pub fn take_retired(&mut self) -> Option<Arc<LoopMemory>> {
        self.retired.take()
    }

    pub fn state(&self) -> LoopState {
        self.state
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn publish(&self) {
        let Some(m) = &self.mem else { return };
        m.state.store(self.state as u8, RELAXED);
        m.len.store(if self.state == LoopState::Recording { self.rec } else { self.len }, RELAXED);
        m.pos.store(self.pos);
        m.layers.store(self.layers, RELAXED);
        m.loop_bpm.store(self.loop_bpm);
        m.waiting.store(self.pending.is_some(), RELAXED);
        m.undo.store(self.can_undo as u8 | (self.can_redo as u8) << 1, RELAXED);
    }

    fn changed(&self) {
        if let Some(m) = &self.mem {
            m.generation.fetch_add(1, RELAXED);
        }
    }

    fn bar_beats(&self) -> f64 {
        self.mem.as_ref().map(|m| m.bar_beats.load() as f64).filter(|b| *b > 0.0).unwrap_or(4.0)
    }

    /// The beat a quantized press waits for (None: at once).
    fn quantized(&self, p: &[f32], ctx: &FxCtx) -> Option<f64> {
        let synced = p[TEMPO].round() as usize > 0;
        let q = (p[QUANTIZE].round().max(0.0) as usize).min(QUANTIZE_BEATS.len() - 1);
        if !synced || !ctx.playing || q == 0 {
            return None;
        }
        let step = if q == QUANTIZE_BEATS.len() - 1 { self.bar_beats() } else { QUANTIZE_BEATS[q] };
        let next = (ctx.beat / step - 1e-6).ceil() * step;
        Some(next.max(ctx.beat))
    }

    fn press(&mut self, a: LoopAction, p: &[f32], ctx: &FxCtx) {
        use LoopState::*;
        // A press waiting for its beat is taken back by the next one (Stop,
        // Clear and the rest just go ahead).
        if self.pending.take().is_some() && matches!(a, LoopAction::Pedal | LoopAction::Record | LoopAction::Play) {
            return;
        }
        match a {
            LoopAction::Pedal => match self.state {
                Empty => self.press(LoopAction::Record, p, ctx),
                Recording => {
                    let next = if p[AFTER_REC].round() as usize == 1 { Playing } else { Overdubbing };
                    match self.quantized(p, ctx) {
                        Some(at) => self.pending = Some((Pending::EndRecording, at)),
                        None => self.end_recording(next, p, ctx),
                    }
                }
                Overdubbing => self.state = Playing,
                Playing => self.begin_layer(),
                Stopped => self.press(LoopAction::Play, p, ctx),
            },
            LoopAction::Record => {
                if self.state == Recording {
                    return self.press(LoopAction::Pedal, p, ctx);
                }
                match self.quantized(p, ctx) {
                    Some(at) => {
                        if self.state != Empty {
                            self.clear();
                        }
                        self.pending = Some((Pending::StartRecording, at));
                    }
                    None => self.start_recording(p, ctx),
                }
            }
            LoopAction::Overdub => match self.state {
                Playing | Stopped if self.len > 0 => {
                    if self.state == Stopped {
                        self.pos = self.start_pos(p);
                    }
                    self.begin_layer();
                }
                Recording => self.end_recording(Overdubbing, p, ctx),
                Empty => self.press(LoopAction::Record, p, ctx),
                _ => {}
            },
            LoopAction::Play => match self.state {
                Stopped if self.len > 0 => match self.quantized(p, ctx) {
                    Some(at) => self.pending = Some((Pending::Play, at)),
                    None => self.play_from_start(p),
                },
                Overdubbing => self.state = Playing,
                Recording => self.end_recording(Playing, p, ctx),
                _ => {}
            },
            LoopAction::Stop => match self.state {
                Recording => {
                    self.end_recording(Stopped, p, ctx);
                }
                Playing | Overdubbing => self.state = Stopped,
                _ => {}
            },
            LoopAction::Clear => self.clear(),
            LoopAction::Undo => self.undo(),
            LoopAction::Redo => self.redo(),
            LoopAction::Multiply => self.multiply(),
        }
    }

    fn start_pos(&self, p: &[f32]) -> f64 {
        if p[REVERSE] >= 0.5 && self.len > 0 {
            (self.len - 1) as f64
        } else {
            0.0
        }
    }

    fn play_from_start(&mut self, p: &[f32]) {
        self.pos = self.start_pos(p);
        self.last_write = None;
        self.state = LoopState::Playing;
    }

    fn clear(&mut self) {
        self.finish_job();
        self.state = LoopState::Empty;
        self.len = 0;
        self.rec = 0;
        self.pos = 0.0;
        self.layers = 0;
        self.loop_bpm = 0.0;
        self.pending = None;
        self.can_undo = false;
        self.can_redo = false;
        self.changed();
    }

    fn start_recording(&mut self, p: &[f32], ctx: &FxCtx) {
        self.finish_job();
        let Some(m) = &self.mem else { return };
        self.state = LoopState::Recording;
        self.rec = 0;
        self.len = 0;
        self.pos = 0.0;
        self.layers = 0;
        self.can_undo = false;
        self.can_redo = false;
        let bars = LENGTH_BARS[(p[LENGTH].round().max(0.0) as usize).min(LENGTH_BARS.len() - 1)];
        let spb = ctx.sr as f64 * 60.0 / (ctx.bpm as f64).max(1.0);
        self.rec_target = if bars > 0.0 { ((bars * self.bar_beats() * spb).round() as usize).min(m.capacity()) } else { 0 };
    }

    fn end_recording(&mut self, next: LoopState, p: &[f32], ctx: &FxCtx) {
        self.len = self.rec;
        self.rec_target = 0;
        if self.len == 0 {
            self.state = LoopState::Empty;
            return;
        }
        self.layers = 1;
        self.loop_bpm = match p[TEMPO].round() as usize {
            0 => 0.0,
            1 => ctx.bpm,
            _ if ctx.playing => ctx.bpm,
            _ => {
                // The first loop sets the song's tempo: the whole number of
                // bars that comes nearest the tempo it has.
                let secs = self.len as f64 / ctx.sr as f64;
                let bar = self.bar_beats();
                let now = ctx.bpm as f64;
                let mut best = (f64::MAX, now);
                for bars in 1..=64 {
                    let bpm = 60.0 * bars as f64 * bar / secs;
                    if !(40.0..=250.0).contains(&bpm) {
                        continue;
                    }
                    let off = (bpm / now).ln().abs();
                    if off < best.0 {
                        best = (off, bpm);
                    }
                }
                let bpm = best.1 as f32;
                if let Some(m) = &self.mem {
                    m.set_tempo.store(bpm);
                }
                bpm
            }
        };
        self.changed();
        self.pos = self.start_pos(p);
        self.last_write = None;
        if next == LoopState::Overdubbing {
            self.state = LoopState::Playing;
            self.begin_layer();
        } else {
            self.state = next;
        }
    }

    /// An overdub starts: a new layer, what it changes kept for undo.
    fn begin_layer(&mut self) {
        if self.len == 0 {
            return;
        }
        self.finish_job();
        if let Some(m) = &self.mem {
            m.forget_layer(self.len);
        }
        self.layers += 1;
        self.can_undo = true;
        self.can_redo = false;
        self.last_write = None;
        self.state = LoopState::Overdubbing;
    }

    fn undo(&mut self) {
        if !self.can_undo || self.len == 0 {
            return;
        }
        self.finish_job();
        if self.state == LoopState::Overdubbing {
            self.state = LoopState::Playing;
        }
        self.job = Job::Swap { at: (self.pos.max(0.0) as usize).min(self.len - 1), done: 0 };
        self.can_undo = false;
        self.can_redo = true;
        self.layers = self.layers.saturating_sub(1).max(1);
        self.changed();
    }

    fn redo(&mut self) {
        if !self.can_redo || self.len == 0 {
            return;
        }
        self.finish_job();
        self.job = Job::Swap { at: (self.pos.max(0.0) as usize).min(self.len - 1), done: 0 };
        self.can_undo = true;
        self.can_redo = false;
        self.layers += 1;
        self.changed();
    }

    fn multiply(&mut self) {
        let Some(m) = &self.mem else { return };
        if self.len == 0 || self.len * 2 > m.capacity() || matches!(self.state, LoopState::Recording | LoopState::Empty) {
            return;
        }
        self.finish_job();
        self.job = Job::Copy { half: self.len, done: 0 };
        self.len *= 2;
        self.can_undo = false;
        self.can_redo = false;
        self.changed();
    }

    /// Does up to `budget` frames of the job.
    fn run_job(&mut self, budget: usize) {
        let Some(m) = &self.mem else {
            self.job = Job::None;
            return;
        };
        match &mut self.job {
            Job::None => {}
            Job::Swap { at, done } => {
                let len = self.len.max(1);
                let end = (*done + budget).min(len);
                for k in *done..end {
                    m.swap((*at + k) % len);
                }
                *done = end;
                if end >= len {
                    self.job = Job::None;
                }
            }
            Job::Copy { half, done } => {
                let end = (*done + budget).min(*half);
                for k in *done..end {
                    let (l, r) = m.read(k);
                    m.write(*half + k, l, r);
                }
                *done = end;
                if end >= *half {
                    self.job = Job::None;
                }
            }
        }
    }

    fn finish_job(&mut self) {
        self.run_job(usize::MAX);
    }

    /// Writes the input onto frame `i` of the loop (an overdub).
    #[inline]
    fn dub(m: &LoopMemory, i: usize, fb: f32, l: f32, r: f32) {
        m.keep(i);
        let (a, b) = m.read(i);
        m.write(i, a * fb + l, b * fb + r);
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        if let Some(m) = self.mem.clone() {
            while let Some(a) = m.take() {
                self.press(a, p, ctx);
            }
        }
        // The Pedal parameter presses the button when it goes on (automation,
        // a modulator, a controller).
        let pedal = p[PEDAL] >= 0.5;
        if pedal && !self.pedal {
            self.press(LoopAction::Pedal, p, ctx);
        }
        self.pedal = pedal;
        self.run_job(JOB_SLICE);
        let Some(m) = self.mem.clone() else { return };

        let monitor = p[MONITOR].round() as usize;
        let fb = p[FEEDBACK].clamp(0.0, 1.0);
        let vol = if p[VOLUME].abs() < 1e-6 { 1.0 } else { db_to_gain(p[VOLUME]) };
        let follow = p[TEMPO].round() as usize > 0 && self.loop_bpm > 0.0;
        let rate = 2f64.powf(p[SPEED].round() as f64 / 12.0) * if follow { ctx.bpm as f64 / self.loop_bpm as f64 } else { 1.0 };
        let dir = if p[REVERSE] >= 0.5 { -1.0 } else { 1.0 };
        let spb = ctx.sr as f64 * 60.0 / (ctx.bpm as f64).max(1.0);
        for i in 0..n {
            if let Some((what, at)) = self.pending {
                let beat = ctx.beat + i as f64 / spb;
                if !ctx.playing || beat + 1e-9 >= at {
                    self.pending = None;
                    match what {
                        Pending::StartRecording => self.start_recording(p, ctx),
                        Pending::EndRecording => {
                            let next = if p[AFTER_REC].round() as usize == 1 { LoopState::Playing } else { LoopState::Overdubbing };
                            self.end_recording(next, p, ctx);
                        }
                        Pending::Play => self.play_from_start(p),
                    }
                }
            }
            let (xl, xr) = (l[i], r[i]);
            let target = if matches!(self.state, LoopState::Empty | LoopState::Stopped) { 0.0 } else { 1.0 };
            if self.gain != target {
                self.gain = if target > self.gain { (self.gain + FADE).min(1.0) } else { (self.gain - FADE).max(0.0) };
                if self.gain == 0.0 && self.state == LoopState::Stopped {
                    self.pos = 0.0;
                }
            }
            let (mut yl, mut yr) = (0.0, 0.0);
            match self.state {
                LoopState::Recording => {
                    if self.rec < m.capacity() {
                        m.write(self.rec, xl, xr);
                        self.rec += 1;
                    }
                    if self.rec >= m.capacity() || (self.rec_target > 0 && self.rec >= self.rec_target) {
                        let next = if p[AFTER_REC].round() as usize == 1 { LoopState::Playing } else { LoopState::Overdubbing };
                        self.end_recording(next, p, ctx);
                    }
                }
                LoopState::Playing | LoopState::Overdubbing | LoopState::Stopped if self.len > 0 && self.gain > 0.0 => {
                    let len = self.len;
                    let i0 = (self.pos.max(0.0) as usize).min(len - 1);
                    let t = (self.pos - i0 as f64) as f32;
                    let (a, b) = m.read(i0);
                    (yl, yr) = if t > 0.0 {
                        let (c, d) = m.read((i0 + 1) % len);
                        (a + (c - a) * t, b + (d - b) * t)
                    } else {
                        (a, b)
                    };
                    yl *= self.gain * vol;
                    yr *= self.gain * vol;
                    if self.state == LoopState::Overdubbing {
                        // Every frame passed since the last one written gets the input.
                        let steps = match self.last_write {
                            None => 1,
                            Some(w) => {
                                let d = if dir > 0.0 { (i0 + len - w) % len } else { (w + len - i0) % len };
                                if d > 16 {
                                    1
                                } else {
                                    d
                                }
                            }
                        };
                        for s in (0..steps).rev() {
                            let j = if dir > 0.0 { (i0 + len - s) % len } else { (i0 + s) % len };
                            Self::dub(&m, j, fb, xl, xr);
                        }
                        self.last_write = Some(i0);
                    }
                    // (Stopped, it plays on while it fades out.)
                    self.pos += rate * dir;
                    if self.pos >= len as f64 {
                        self.pos -= len as f64;
                    } else if self.pos < 0.0 {
                        self.pos += len as f64;
                    }
                    self.pos = self.pos.clamp(0.0, len as f64 - 1e-9);
                }
                _ => {}
            }
            let hear = match monitor {
                1 => false,
                2 => matches!(self.state, LoopState::Recording | LoopState::Overdubbing),
                _ => true,
            };
            l[i] = yl + if hear { xl } else { 0.0 };
            r[i] = yr + if hear { xr } else { 0.0 };
        }
        if self.state == LoopState::Overdubbing {
            self.changed();
        }
        self.publish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::fx::test_util::ctx;

    const SR: f32 = 48000.0;

    fn looper(secs: f64) -> (Looper, Arc<LoopMemory>) {
        let mem = Arc::new(LoopMemory::new(SR, secs));
        let mut lp = Looper::new();
        lp.attach(Some(&mem));
        (lp, mem)
    }

    fn params() -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        p[MONITOR] = 1.0;
        p[TEMPO] = 0.0;
        p
    }

    /// Runs `input` through in blocks of 256, the song at `beat0` (moving on
    /// when `playing`), and returns what came out.
    fn run(lp: &mut Looper, p: &[f32], input: &[f32], c: &mut FxCtx) -> Vec<f32> {
        let mut out = Vec::with_capacity(input.len());
        for chunk in input.chunks(256) {
            let mut l = chunk.to_vec();
            let mut r = chunk.to_vec();
            lp.process(p, &mut l, &mut r, c);
            if c.playing {
                c.beat += chunk.len() as f64 * c.bpm as f64 / 60.0 / c.sr as f64;
            }
            out.extend_from_slice(&l);
        }
        out
    }

    fn noise(n: usize, seed: u32) -> Vec<f32> {
        let mut rng = crate::dsp::Rng::new(seed);
        (0..n).map(|_| rng.bipolar() * 0.5).collect()
    }

    fn loop_left(mem: &LoopMemory) -> Vec<f32> {
        mem.frames().0
    }

    #[test]
    fn records_bars_and_plays_them_back_exactly() {
        let (mut lp, mem) = looper(10.0);
        let mut p = params();
        p[LENGTH] = 2.0; // two bars
        p[AFTER_REC] = 1.0; // then play
        let mut c = ctx();
        let bars2 = (2.0 * 4.0 * 60.0 / 120.0 * SR as f64) as usize;
        let take = noise(bars2, 5);
        mem.press(LoopAction::Pedal);
        run(&mut lp, &p, &take, &mut c);
        assert_eq!(lp.len(), bars2, "two bars at 120");
        assert_eq!(lp.state(), LoopState::Playing);
        let out = run(&mut lp, &p, &vec![0.0; bars2 * 2], &mut c);
        assert_eq!(&out[..bars2], &take[..], "the loop plays back bit for bit");
        assert_eq!(&out[bars2..], &take[..], "and again");
        assert_eq!(mem.len.load(RELAXED), bars2);
        assert_eq!(mem.state(), LoopState::Playing);
    }

    #[test]
    fn overdubs_add_with_feedback_and_undo_and_redo() {
        let (mut lp, mem) = looper(10.0);
        let mut p = params();
        p[FEEDBACK] = 0.5;
        let mut c = ctx();
        let n = 4800;
        let x = noise(n, 1);
        let y = noise(n, 2);
        mem.press(LoopAction::Pedal);
        run(&mut lp, &p, &x, &mut c);
        mem.press(LoopAction::Pedal); // to overdub
        run(&mut lp, &p, &y, &mut c);
        mem.press(LoopAction::Pedal); // to play
        run(&mut lp, &p, &[0.0; 256], &mut c);
        assert_eq!(lp.state(), LoopState::Playing);
        assert_eq!(mem.layers.load(RELAXED), 2);
        let dubbed = loop_left(&mem);
        for k in 0..n {
            let want = x[k] * 0.5 + y[k];
            assert!((dubbed[k] - want).abs() < 1e-6, "frame {k}: {} against {want}", dubbed[k]);
        }
        mem.press(LoopAction::Undo);
        run(&mut lp, &p, &[0.0; 512], &mut c);
        assert_eq!(loop_left(&mem), x, "undo gives back the loop before the overdub");
        assert_eq!(mem.layers.load(RELAXED), 1);
        mem.press(LoopAction::Redo);
        run(&mut lp, &p, &[0.0; 512], &mut c);
        assert_eq!(loop_left(&mem), dubbed, "redo puts the overdub back");
    }

    #[test]
    fn multiply_doubles_the_loop() {
        let (mut lp, mem) = looper(10.0);
        let mut p = params();
        p[AFTER_REC] = 1.0;
        let mut c = ctx();
        let x = noise(3000, 9);
        mem.press(LoopAction::Pedal);
        run(&mut lp, &p, &x, &mut c);
        mem.press(LoopAction::Pedal);
        mem.press(LoopAction::Multiply);
        run(&mut lp, &p, &[0.0; 256], &mut c);
        assert_eq!(lp.len(), 6000);
        let both = loop_left(&mem);
        assert_eq!(&both[..3000], &x[..]);
        assert_eq!(&both[3000..], &x[..]);
    }

    #[test]
    fn half_speed_is_an_octave_down_and_twice_as_long() {
        let (mut lp, mem) = looper(10.0);
        let mut p = params();
        p[AFTER_REC] = 1.0;
        let mut c = ctx();
        let n = 48000;
        let tone: Vec<f32> = (0..n).map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / SR).sin() * 0.5).collect();
        mem.press(LoopAction::Pedal);
        run(&mut lp, &p, &tone, &mut c);
        mem.press(LoopAction::Pedal); // play
        mem.press(LoopAction::Stop);
        run(&mut lp, &p, &[0.0; 512], &mut c);
        p[SPEED] = -12.0;
        mem.press(LoopAction::Play);
        let out = run(&mut lp, &p, &vec![0.0; 2 * n], &mut c);
        // Where the loop is once twice its length has been played: at its start.
        assert!(mem.pos.load() < 4.0, "back at the start after 2N: {}", mem.pos.load());
        let mid = &out[24000..72000];
        let f = crate::dsp::fx::test_util::strongest(mid, 150.0, 500.0, 2.0);
        assert!((f - 220.0).abs() < 4.0, "an octave down: {f}");
    }

    #[test]
    fn a_recording_waits_for_the_bar() {
        let (mut lp, mem) = looper(10.0);
        let mut p = params();
        p[TEMPO] = 1.0; // follow
        p[QUANTIZE] = 5.0; // 1 bar
        let mut c = ctx();
        c.playing = true;
        c.beat = 1.25;
        // The input counts the frames, so the loop says when it started.
        let spb = 24000.0;
        let input: Vec<f32> = (0..96000).map(|i| i as f32).collect();
        mem.press(LoopAction::Pedal);
        let first = run(&mut lp, &p, &input[..256], &mut c);
        assert!(mem.waiting.load(RELAXED), "waits for its bar");
        let _ = first;
        run(&mut lp, &p, &input[256..], &mut c);
        let start = ((4.0 - 1.25) * spb) as usize;
        assert_eq!(mem.read(0).0, start as f32, "the first frame recorded is the bar's");
        // Pressed again: it ends on the next bar, two bars after it began... here one bar.
        mem.press(LoopAction::Pedal);
        let more: Vec<f32> = (96000..300000).map(|i| i as f32).collect();
        run(&mut lp, &p, &more, &mut c);
        assert_eq!(lp.len() % (4 * 24000), 0, "a whole number of bars: {}", lp.len());
        assert_eq!(mem.loop_bpm.load(), 120.0);
    }

    #[test]
    fn the_first_loop_sets_the_tempo() {
        let (mut lp, mem) = looper(10.0);
        let mut p = params();
        p[TEMPO] = 2.0; // set & follow
        let mut c = ctx();
        mem.press(LoopAction::Pedal);
        run(&mut lp, &p, &noise((2.1 * SR as f64) as usize, 4), &mut c);
        mem.press(LoopAction::Pedal);
        run(&mut lp, &p, &[0.0; 256], &mut c);
        let bpm = mem.set_tempo.load();
        // One bar of four beats in 2.1 s: 114.3 BPM (two bars would be 228).
        assert!((bpm - 114.2857).abs() < 0.01, "{bpm}");
        // And it goes on to overdub, at that tempo.
        assert_eq!(mem.loop_bpm.load(), bpm);
        assert_eq!(lp.state(), LoopState::Overdubbing);
    }

    #[test]
    fn no_memory_passes_the_sound_through() {
        let mut lp = Looper::new();
        let p = crate::params::defaults(SPECS);
        let mut l = vec![0.25; 64];
        let mut r = vec![0.25; 64];
        lp.process(&p, &mut l, &mut r, &ctx());
        assert!(l.iter().all(|&x| x == 0.25));
    }
}
