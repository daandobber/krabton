//! The tape: four stereo tracks on a six-minute reel. It runs with the
//! transport at any speed (a quarter to twice as fast, the pitch going with
//! it), forwards or backwards, looping between two points; tape tricks brake
//! it to a stop, stutter the bit just played, or scrub it by hand. The armed
//! track records what a track plays (or the audio input) onto the tape,
//! mixed with what is there, through a soft tape saturation. Each track has
//! its own volume, pan and mute and goes to the master.
//!
//! The reel is shared: the engine plays and records it, the UI draws it,
//! edits it (lift, drop, erase, undo) and saves it, all through atomics, so
//! neither waits for the other. Frames are 16-bit stereo (left in the low
//! half), so four full tracks take 276 MB at 48 kHz; a track's memory is
//! taken the first time it is armed or loaded.

use crate::engine::{AtomicF32, AtomicF64};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicUsize, Ordering};
use std::sync::OnceLock;

const RELAXED: Ordering = Ordering::Relaxed;

pub const TAPE_TRACKS: usize = 4;
/// How long the reel is.
pub const TAPE_SECONDS: f64 = 360.0;
/// Frames per entry of a track's overview (its peaks, for drawing).
pub const OVERVIEW: usize = 1024;
/// The slowest and fastest the tape runs.
pub const MIN_SPEED: f32 = 0.25;
pub const MAX_SPEED: f32 = 2.0;

/// What the armed tape track records.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TapeSource {
    /// The track being played (the selected one; the UI tells the engine).
    #[default]
    Played,
    Track(u64),
    /// The audio interface's first two inputs.
    Input,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TapeTrack {
    #[serde(default)]
    pub volume_db: f32,
    #[serde(default)]
    pub pan: f32,
    #[serde(default)]
    pub mute: bool,
    #[serde(default)]
    pub source: TapeSource,
    /// Where its audio is saved (next to the set).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,
}

/// The tape's settings, saved with the set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TapeSettings {
    pub tracks: [TapeTrack; TAPE_TRACKS],
    /// Speed as a ratio (1: as recorded).
    pub speed: f32,
    pub reverse: bool,
    pub loop_on: bool,
    /// The loop, in seconds of tape.
    pub loop_in: f64,
    pub loop_out: f64,
    /// The track that records while the transport runs.
    pub armed: Option<usize>,
    /// Gain on what is recorded, in dB.
    pub rec_gain_db: f32,
}

impl Default for TapeSettings {
    fn default() -> Self {
        Self { tracks: Default::default(), speed: 1.0, reverse: false, loop_on: false, loop_in: 0.0, loop_out: 8.0, armed: None, rec_gain_db: 0.0 }
    }
}

impl TapeSettings {
    /// The speed with its direction.
    pub fn velocity(&self) -> f32 {
        let s = self.speed.clamp(MIN_SPEED, MAX_SPEED);
        if self.reverse {
            -s
        } else {
            s
        }
    }
}

/// Soft tape saturation: straight for quiet sound, rounding off to full
/// scale at 1.5 (and flat beyond).
fn saturate(x: f32) -> f32 {
    let x = x.clamp(-1.5, 1.5);
    x - x * x * x / 6.75
}

fn pack(l: f32, r: f32) -> u32 {
    let q = |x: f32| (saturate(x) * 32767.0).round() as i16 as u16 as u32;
    q(l) | (q(r) << 16)
}

fn unpack(v: u32) -> (f32, f32) {
    ((v as u16 as i16) as f32 / 32767.0, ((v >> 16) as u16 as i16) as f32 / 32767.0)
}

/// One track's audio on the reel.
pub struct Spool {
    data: Box<[AtomicU32]>,
    /// The loudest of each `OVERVIEW` frames (0..65535).
    overview: Box<[AtomicU16]>,
    /// Changed since it was last saved.
    pub dirty: AtomicBool,
}

impl Spool {
    fn new(frames: usize) -> Self {
        Self {
            data: (0..frames).map(|_| AtomicU32::new(0)).collect(),
            overview: (0..frames.div_ceil(OVERVIEW)).map(|_| AtomicU16::new(0)).collect(),
            dirty: AtomicBool::new(false),
        }
    }

    pub fn frames(&self) -> usize {
        self.data.len()
    }

    pub fn read(&self, i: usize) -> (f32, f32) {
        self.data.get(i).map(|v| unpack(v.load(RELAXED))).unwrap_or((0.0, 0.0))
    }

    /// Reads between frames (linearly), anywhere on the reel.
    pub fn read_at(&self, pos: f64) -> (f32, f32) {
        if pos < 0.0 {
            return (0.0, 0.0);
        }
        let i = pos as usize;
        let t = (pos - i as f64) as f32;
        let (a, b) = self.read(i);
        let (c, d) = self.read(i + 1);
        (a + (c - a) * t, b + (d - b) * t)
    }

    fn note_peak(&self, i: usize, l: f32, r: f32) {
        let p = (l.abs().max(r.abs()).min(1.0) * 65535.0) as u16;
        if let Some(o) = self.overview.get(i / OVERVIEW) {
            if p > o.load(RELAXED) {
                o.store(p, RELAXED);
            }
        }
    }

    /// Writes a frame over what is there.
    pub fn write(&self, i: usize, l: f32, r: f32) {
        if let Some(v) = self.data.get(i) {
            let packed = pack(l, r);
            v.store(packed, RELAXED);
            let (l, r) = unpack(packed);
            self.note_peak(i, l, r);
        }
    }

    /// Records a frame onto what is there (overdub).
    pub fn add(&self, i: usize, l: f32, r: f32) {
        let (a, b) = self.read(i);
        self.write(i, a + l, b + r);
    }

    /// The loudest of the overview entry `k` (0..1).
    pub fn peak(&self, k: usize) -> f32 {
        self.overview.get(k).map(|o| o.load(RELAXED) as f32 / 65535.0).unwrap_or(0.0)
    }

    pub fn overview_len(&self) -> usize {
        self.overview.len()
    }

    /// Recounts the overview of frames `a..b` after an edit.
    pub fn refresh(&self, a: usize, b: usize) {
        let b = b.min(self.frames());
        for k in a / OVERVIEW..b.div_ceil(OVERVIEW).min(self.overview.len()) {
            let lo = k * OVERVIEW;
            let hi = (lo + OVERVIEW).min(self.frames());
            let p = (lo..hi).map(|i| {
                let (l, r) = self.read(i);
                l.abs().max(r.abs())
            });
            self.overview[k].store((p.fold(0.0f32, f32::max).min(1.0) * 65535.0) as u16, RELAXED);
        }
    }

    /// One frame past the last that is not silent.
    pub fn end(&self) -> usize {
        match self.overview.iter().rposition(|o| o.load(RELAXED) > 0) {
            Some(k) => {
                let hi = ((k + 1) * OVERVIEW).min(self.frames());
                (k * OVERVIEW..hi).rev().find(|&i| self.data[i].load(RELAXED) != 0).map(|i| i + 1).unwrap_or(k * OVERVIEW)
            }
            None => 0,
        }
    }

    /// Frames `a..b` as they are stored.
    pub fn copy_raw(&self, a: usize, b: usize) -> Vec<u32> {
        let b = b.min(self.frames());
        (a.min(b)..b).map(|i| self.data[i].load(RELAXED)).collect()
    }

    /// Puts stored frames at `at`, over what is there.
    pub fn paste_raw(&self, at: usize, raw: &[u32]) {
        for (k, v) in raw.iter().enumerate() {
            if let Some(d) = self.data.get(at + k) {
                d.store(*v, RELAXED);
            }
        }
        self.refresh(at, at + raw.len());
        self.dirty.store(true, RELAXED);
    }

    pub fn erase(&self, a: usize, b: usize) {
        let b = b.min(self.frames());
        for d in &self.data[a.min(b)..b] {
            d.store(0, RELAXED);
        }
        self.refresh(a, b);
        self.dirty.store(true, RELAXED);
    }

    /// The track as stereo frames, up to its end.
    pub fn interleaved(&self) -> Vec<f32> {
        (0..self.end()).flat_map(|i| {
            let (l, r) = self.read(i);
            [l, r]
        }).collect()
    }

    /// Fills the track from stereo frames (the rest silent).
    pub fn load_interleaved(&self, frames: &[f32]) {
        for (i, d) in self.data.iter().enumerate() {
            let v = match frames.get(i * 2..i * 2 + 2) {
                Some(f) => pack(f[0], f[1]),
                None => 0,
            };
            d.store(v, RELAXED);
        }
        self.refresh(0, self.frames());
        self.dirty.store(false, RELAXED);
    }

    pub fn clear(&self) {
        for d in self.data.iter() {
            d.store(0, RELAXED);
        }
        for o in self.overview.iter() {
            o.store(0, RELAXED);
        }
        self.dirty.store(false, RELAXED);
    }
}

/// The reel and what the engine tells the UI about it.
pub struct Reel {
    frames: AtomicUsize,
    rate: AtomicF32,
    spools: [OnceLock<Spool>; TAPE_TRACKS],
    /// A copy of one track from before the last recording or edit.
    undo: OnceLock<Spool>,
    undo_track: AtomicUsize,
    /// Where the tape is (frames) and how fast it runs now.
    pub pos: AtomicF64,
    pub speed: AtomicF32,
    pub recording: AtomicBool,
    /// Each track's peak since the UI last took it.
    pub levels: [AtomicF32; TAPE_TRACKS],
}

impl Default for Reel {
    fn default() -> Self {
        Self::new()
    }
}

impl Reel {
    pub fn new() -> Self {
        Self {
            frames: AtomicUsize::new((TAPE_SECONDS * 48000.0) as usize),
            rate: AtomicF32::new(48000.0),
            spools: Default::default(),
            undo: OnceLock::new(),
            undo_track: AtomicUsize::new(usize::MAX),
            pos: AtomicF64::new(0.0),
            speed: AtomicF32::new(0.0),
            recording: AtomicBool::new(false),
            levels: std::array::from_fn(|_| AtomicF32::new(0.0)),
        }
    }

    /// The sample rate the reel runs at (before any track is made).
    pub fn set_rate(&self, sr: f32) {
        self.rate.store(sr);
        self.frames.store((TAPE_SECONDS * sr as f64) as usize, RELAXED);
    }

    pub fn rate(&self) -> f32 {
        self.rate.load()
    }

    pub fn frames(&self) -> usize {
        self.frames.load(RELAXED)
    }

    /// A track's audio, if it was ever made.
    pub fn spool(&self, k: usize) -> Option<&Spool> {
        self.spools.get(k)?.get()
    }

    /// A track's audio, made (silent) the first time. Not on the audio thread.
    pub fn spool_or_make(&self, k: usize) -> &Spool {
        self.spools[k].get_or_init(|| Spool::new(self.frames()))
    }

    /// Keeps a copy of track `k` for undo.
    pub fn snapshot(&self, k: usize) {
        let Some(s) = self.spool(k) else { return };
        let u = self.undo.get_or_init(|| Spool::new(self.frames()));
        for (d, v) in u.data.iter().zip(s.data.iter()) {
            d.store(v.load(RELAXED), RELAXED);
        }
        self.undo_track.store(k, RELAXED);
    }

    /// Swaps track `k` with its copy: undo, and again redo. Returns the track.
    pub fn swap_undo(&self) -> Option<usize> {
        let k = self.undo_track.load(RELAXED);
        let (s, u) = (self.spool(k)?, self.undo.get()?);
        for (a, b) in s.data.iter().zip(u.data.iter()) {
            let v = a.load(RELAXED);
            a.store(b.swap(v, RELAXED), RELAXED);
        }
        s.refresh(0, s.frames());
        s.dirty.store(true, RELAXED);
        Some(k)
    }

    pub fn can_undo(&self) -> bool {
        self.undo_track.load(RELAXED) < TAPE_TRACKS
    }

    /// Silences every track and forgets undo (a new set).
    pub fn clear(&self) {
        for k in 0..TAPE_TRACKS {
            if let Some(s) = self.spool(k) {
                s.clear();
            }
        }
        self.undo_track.store(usize::MAX, RELAXED);
        self.pos.store(0.0);
    }
}

/// The tape's state on the audio thread.
pub struct TapeRt {
    pos: f64,
    /// The speed now (frames per frame, negative backwards), easing to its target.
    speed: f32,
    pub brake: bool,
    /// Repeating `len` frames from `start` while a stutter is held.
    stutter: Option<(f64, f64)>,
    pub scrub: Option<f32>,
    /// A speed the tape is bent by for a moment (1: none).
    pub bend: f32,
    /// The track being played, for `TapeSource::Played`.
    pub follow: Option<u64>,
    /// The last frame recorded in this pass.
    last_rec: Option<i64>,
    gains: [(f32, f32); TAPE_TRACKS],
}

impl Default for TapeRt {
    fn default() -> Self {
        Self::new()
    }
}

impl TapeRt {
    pub fn new() -> Self {
        Self { pos: 0.0, speed: 0.0, brake: false, stutter: None, scrub: None, bend: 1.0, follow: None, last_rec: None, gains: [(0.0, 0.0); TAPE_TRACKS] }
    }

    pub fn locate(&mut self, frames: f64) {
        self.pos = frames.max(0.0);
        self.last_rec = None;
    }

    /// Repeats the last `len` frames played until `None`.
    pub fn stutter(&mut self, len: Option<f64>) {
        self.stutter = len.filter(|l| *l >= 64.0).map(|len| if self.speed < 0.0 { (self.pos, len) } else { ((self.pos - len).max(0.0), len) });
    }

    /// Runs the tape for one block: plays every track into `out` (added) and
    /// records `rec` (the armed track's source) onto the armed track.
    #[allow(clippy::too_many_arguments)]
    pub fn process(&mut self, reel: &Reel, set: &TapeSettings, sr: f32, playing: bool, rec: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let n = out_l.len().min(out_r.len());
        let frames = reel.frames();
        let moving = playing || self.scrub.is_some();
        if !moving {
            self.speed = 0.0;
            self.last_rec = None;
            reel.speed.store(0.0);
            reel.recording.store(false, RELAXED);
            reel.pos.store(self.pos);
            return;
        }
        let target = match self.scrub {
            Some(v) => v,
            None if self.brake => 0.0,
            None => set.velocity() * self.bend.clamp(0.25, 4.0),
        };
        // The tape eases to a new speed (a brake slowly, the rest quickly).
        let secs = if self.brake && self.scrub.is_none() { 0.5 } else { 0.06 };
        let ease = 1.0 - (-1.0 / (secs * sr)).exp();
        // Each track's gain, gliding over the block.
        let mut to = [(0.0f32, 0.0f32); TAPE_TRACKS];
        for (k, t) in set.tracks.iter().enumerate() {
            if !t.mute {
                let g = 10f32.powf(t.volume_db.clamp(-70.0, 12.0) / 20.0);
                let p = (t.pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
                to[k] = (g * p.cos() * std::f32::consts::SQRT_2, g * p.sin() * std::f32::consts::SQRT_2);
            }
        }
        let spools: [Option<&Spool>; TAPE_TRACKS] = std::array::from_fn(|k| reel.spool(k));
        let armed = set.armed.filter(|&k| k < TAPE_TRACKS && playing && self.scrub.is_none()).and_then(|k| spools[k].map(|s| (k, s)));
        let recording = armed.is_some() && rec.is_some();
        let rec_gain = 10f32.powf(set.rec_gain_db.clamp(-40.0, 24.0) / 20.0);
        let mut peaks = [0.0f32; TAPE_TRACKS];
        for i in 0..n {
            self.speed += (target - self.speed) * ease;
            let t = (i + 1) as f32 / n as f32;
            for k in 0..TAPE_TRACKS {
                let Some(s) = spools[k] else { continue };
                let (l, r) = s.read_at(self.pos);
                let (g0, g1) = self.gains[k];
                let gl = g0 + (to[k].0 - g0) * t;
                let gr = g1 + (to[k].1 - g1) * t;
                out_l[i] += l * gl;
                out_r[i] += r * gr;
                peaks[k] = peaks[k].max(l.abs()).max(r.abs());
            }
            if let (Some((_, s)), Some((rl, rr))) = (armed, rec) {
                let idx = self.pos.floor() as i64;
                if self.speed.abs() > 1e-3 && Some(idx) != self.last_rec && idx >= 0 {
                    let (x, y) = (rl.get(i).copied().unwrap_or(0.0) * rec_gain, rr.get(i).copied().unwrap_or(0.0) * rec_gain);
                    // Faster than real time some frames are passed over: they get the same sound.
                    let from = match self.last_rec {
                        Some(prev) if (idx - prev).abs() <= 8 => prev,
                        _ if self.speed > 0.0 => idx - 1,
                        _ => idx + 1,
                    };
                    let step = if idx > from { 1 } else { -1 };
                    let mut j = from + step;
                    loop {
                        if j >= 0 {
                            s.add(j as usize, x, y);
                        }
                        if j == idx {
                            break;
                        }
                        j += step;
                    }
                    self.last_rec = Some(idx);
                    s.dirty.store(true, RELAXED);
                }
            }
            let before = self.pos;
            self.pos += self.speed as f64;
            // Where the tape goes on: a stutter's slice, round the loop when
            // it runs out of it (forwards past its end, backwards past its
            // start), or stops at the ends of the reel.
            if let Some((a, len)) = self.stutter {
                if self.pos >= a + len || self.pos < a {
                    self.pos = a + (self.pos - a).rem_euclid(len);
                    self.last_rec = None;
                }
            } else if set.loop_on && set.loop_out > set.loop_in + 0.01 {
                let (a, b) = ((set.loop_in * sr as f64).max(0.0), (set.loop_out * sr as f64).min(frames as f64));
                if b > a && ((before < b && self.pos >= b) || (before >= a && self.pos < a)) {
                    self.pos = a + (self.pos - a).rem_euclid(b - a);
                    self.last_rec = None;
                }
            }
            if self.pos < 0.0 {
                self.pos = 0.0;
                self.speed = self.speed.max(0.0);
            } else if self.pos > (frames.saturating_sub(2)) as f64 {
                self.pos = frames.saturating_sub(2) as f64;
                self.speed = self.speed.min(0.0);
            }
        }
        self.gains = to;
        for (lv, p) in reel.levels.iter().zip(peaks) {
            lv.store_max(p);
        }
        reel.pos.store(self.pos);
        reel.speed.store(self.speed);
        reel.recording.store(recording, RELAXED);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 8000.0;

    fn reel() -> Reel {
        let r = Reel::new();
        r.set_rate(SR);
        r
    }

    /// Runs the tape `blocks` blocks of 64 frames, recording `src` (per frame
    /// index of the run) and returning what it played.
    fn run(tape: &mut TapeRt, r: &Reel, set: &TapeSettings, playing: bool, blocks: usize, src: impl Fn(usize) -> f32) -> Vec<f32> {
        let mut out = Vec::new();
        for b in 0..blocks {
            let rec: Vec<f32> = (0..64).map(|i| src(b * 64 + i)).collect();
            let (mut l, mut rr) = (vec![0.0; 64], vec![0.0; 64]);
            tape.process(r, set, SR, playing, Some((&rec, &rec)), &mut l, &mut rr);
            out.extend_from_slice(&l);
        }
        out
    }

    #[test]
    fn frames_keep_their_sound() {
        for x in [0.0f32, 0.25, -0.5, 0.9] {
            let (l, r) = unpack(pack(x, -x));
            assert!((l - saturate(x)).abs() < 1e-4 && (r + saturate(x)).abs() < 1e-4, "{x}: {l} {r}");
        }
        // Too loud is rounded off, never wrapped.
        assert!(unpack(pack(4.0, -4.0)).0 > 0.99);
        assert!(unpack(pack(4.0, -4.0)).1 < -0.99);
    }

    #[test]
    fn what_is_recorded_plays_back() {
        let r = reel();
        r.spool_or_make(1);
        let mut set = TapeSettings { armed: Some(1), ..Default::default() };
        // Slow enough that a frame of difference in where it lands is lost in the tolerance.
        let tone = |i: usize| ((i as f32) * 0.01).sin() * 0.5;
        run(&mut TapeRt::new(), &r, &set, true, 100, tone);
        assert!(r.spool(1).unwrap().dirty.load(RELAXED));
        // Played from the start the same way, every frame is where it was recorded.
        set.armed = None;
        let back = run(&mut TapeRt::new(), &r, &set, true, 100, |_| 0.0);
        // The first frames ease in; after that the tape plays what it got.
        let err = (2000..6000).map(|i| (back[i] - saturate(tone(i))).abs()).fold(0.0f32, f32::max);
        assert!(err < 0.01, "played back off by {err}");
        // Recording again adds to what is there (every frame once).
        let (before, _) = r.spool(1).unwrap().read(3000);
        set.armed = Some(1);
        run(&mut TapeRt::new(), &r, &set, true, 100, |_| 0.1);
        let (l, _) = r.spool(1).unwrap().read(3000);
        assert!((l - saturate(before + 0.1)).abs() < 0.001, "overdub {before} + 0.1 = {l}");
    }

    #[test]
    fn speed_and_direction() {
        let r = reel();
        let s = r.spool_or_make(0);
        // A ramp: each frame tells where it is.
        for i in 0..20000 {
            s.write(i, i as f32 / 40000.0, 0.0);
        }
        let mut tape = TapeRt::new();
        let set = TapeSettings { speed: 2.0, ..Default::default() };
        run(&mut tape, &r, &set, true, 50, |_| 0.0);
        // Twice as fast, after easing in from standing still (60 ms).
        let tau = 0.06 * SR as f64;
        let want = 2.0 * (3200.0 - tau * (1.0 - (-3200.0 / tau).exp()));
        let pos = r.pos.load();
        assert!((pos - want).abs() < 20.0, "at {pos}, not {want}");
        let set = TapeSettings { reverse: true, ..Default::default() };
        let before = r.pos.load();
        let out = run(&mut tape, &r, &set, true, 50, |_| 0.0);
        assert!(r.pos.load() < before, "backwards: {before} -> {}", r.pos.load());
        assert!(out[3000] < out[1000], "the ramp plays downwards");
    }

    #[test]
    fn the_loop_and_the_stutter_go_round() {
        let r = reel();
        r.spool_or_make(0);
        let mut tape = TapeRt::new();
        let set = TapeSettings { loop_on: true, loop_in: 0.25, loop_out: 0.5, ..Default::default() };
        tape.locate(0.3 * SR as f64);
        run(&mut tape, &r, &set, true, 200, |_| 0.0);
        let pos = r.pos.load() / SR as f64;
        assert!((0.25..0.5).contains(&pos), "in the loop: {pos}");
        let set = TapeSettings::default();
        tape.locate(1.0 * SR as f64);
        run(&mut tape, &r, &set, true, 20, |_| 0.0);
        tape.stutter(Some(400.0));
        let start = r.pos.load();
        run(&mut tape, &r, &set, true, 100, |_| 0.0);
        let pos = r.pos.load();
        assert!(pos <= start + 1.0 && pos >= start - 400.0, "stutters on {start}: {pos}");
        tape.stutter(None);
        run(&mut tape, &r, &set, true, 20, |_| 0.0);
        assert!(r.pos.load() > start + 500.0, "goes on after the stutter");
    }

    #[test]
    fn a_brake_stops_the_tape() {
        let r = reel();
        r.spool_or_make(0);
        let mut tape = TapeRt::new();
        let set = TapeSettings::default();
        run(&mut tape, &r, &set, true, 20, |_| 0.0);
        tape.brake = true;
        run(&mut tape, &r, &set, true, 400, |_| 0.0);
        assert!(r.speed.load().abs() < 0.01, "stopped: {}", r.speed.load());
        tape.brake = false;
        run(&mut tape, &r, &set, true, 50, |_| 0.0);
        assert!((r.speed.load() - 1.0).abs() < 0.01, "running again: {}", r.speed.load());
    }

    #[test]
    fn edits_and_undo() {
        let r = reel();
        let s = r.spool_or_make(2);
        for i in 0..1000 {
            s.write(i, 0.5, 0.5);
        }
        assert_eq!(s.end(), 1000);
        r.snapshot(2);
        let lifted = s.copy_raw(100, 200);
        s.erase(100, 200);
        assert_eq!(s.read(150), (0.0, 0.0));
        s.paste_raw(2000, &lifted);
        assert!(s.read(2050).0 > 0.4);
        assert_eq!(s.end(), 2100);
        assert_eq!(r.swap_undo(), Some(2));
        assert!(s.read(150).0 > 0.4, "undo brings it back");
        assert_eq!(s.end(), 1000);
        r.swap_undo();
        assert_eq!(s.read(150), (0.0, 0.0), "and redo takes it away again");
        let saved = s.interleaved();
        s.clear();
        s.load_interleaved(&saved);
        assert!(s.read(2050).0 > 0.4);
    }
}
