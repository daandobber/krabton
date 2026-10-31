//! Warping audio clips, as in Live: which second of a sample falls on which
//! beat (the warp markers), where a clip starts, ends and loops, and how it
//! is played at any tempo (the warp modes).
//!
//! Everything here runs on the audio thread without allocating: the stretch
//! modes read the sample in memory directly, and what they remember between
//! blocks (where their last grains read) is a small fixed table per track.
//!
//! - *Re-Pitch* reads the sample faster or slower: tempo and pitch together.
//! - *Beats* cuts the audio at its transients (or on a grid) and plays each
//!   piece at its own speed from the beat it belongs on; a piece that ends
//!   before the next begins stops, or loops its tail (forwards or back and
//!   forth). The hits stay exactly as sharp as they were.
//! - *Tones* and *Complex* are WSOLA: grains of 10 to 160 ms, each one
//!   placed where it lines up best with the one before (by cross
//!   correlation), so pitch stays put while time stretches. Complex also
//!   starts afresh on every transient, so drums in a song keep their attack
//!   and are never heard twice.
//! - *Texture* lines its grains up too, then lets them wander (Flux; for pads,
//!   noise and soundscapes).
//!
//! Warped, Transpose and Detune move the pitch through the grains and leave
//! the tempo alone; in Re-Pitch they do nothing (as in Live), unwarped they
//! play the sample faster or slower.

use crate::project::AudioClip;
use crate::sample::SampleData;
use serde::{Deserialize, Serialize};
use std::f64::consts::{FRAC_PI_2, PI};

/// How a warped clip is stretched to the tempo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarpMode {
    /// Cut at the hits, every piece at its own speed on its own beat.
    Beats,
    /// Grains lined up on the waveform: for notes and voices.
    Tones,
    /// Grains that wander: for pads, noise and soundscapes.
    Texture,
    /// Faster and higher together, like a record.
    #[default]
    RePitch,
    /// Lined-up grains between the hits, which stay as they were: for songs.
    Complex,
}

impl WarpMode {
    pub const ALL: [WarpMode; 5] = [WarpMode::Beats, WarpMode::Tones, WarpMode::Texture, WarpMode::RePitch, WarpMode::Complex];

    pub fn name(self) -> &'static str {
        match self {
            WarpMode::Beats => "Beats",
            WarpMode::Tones => "Tones",
            WarpMode::Texture => "Texture",
            WarpMode::RePitch => "Re-Pitch",
            WarpMode::Complex => "Complex",
        }
    }

    /// What it is good for, for a tooltip.
    pub fn about(self) -> &'static str {
        match self {
            WarpMode::Beats => "Cut at the hits, each piece played at its own speed on its beat: drums and rhythmic loops stay sharp",
            WarpMode::Tones => "Grains lined up on the waveform: notes, bass lines and voices keep their pitch",
            WarpMode::Texture => "Grains that wander: pads, noise and soundscapes",
            WarpMode::RePitch => "Faster and higher together, like a record (no stretching)",
            WarpMode::Complex => "Lined-up grains between the hits, which stay as they were: whole songs",
        }
    }

    /// The next one (or the one before), for a button clicked round.
    pub fn step(self, by: i32) -> Self {
        let i = Self::ALL.iter().position(|&m| m == self).unwrap_or(0) as i32;
        Self::ALL[(i + by).rem_euclid(Self::ALL.len() as i32) as usize]
    }
}

/// Where Beats cuts the audio.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Preserve {
    /// At its transients.
    #[default]
    Transients,
    Quarter,
    Eighth,
    Sixteenth,
    ThirtySecond,
}

impl Preserve {
    pub const ALL: [Preserve; 5] = [Preserve::Transients, Preserve::Quarter, Preserve::Eighth, Preserve::Sixteenth, Preserve::ThirtySecond];

    pub fn name(self) -> &'static str {
        match self {
            Preserve::Transients => "Transients",
            Preserve::Quarter => "1/4",
            Preserve::Eighth => "1/8",
            Preserve::Sixteenth => "1/16",
            Preserve::ThirtySecond => "1/32",
        }
    }

    /// The grid in beats (None: the transients).
    pub fn grid(self) -> Option<f64> {
        match self {
            Preserve::Transients => None,
            Preserve::Quarter => Some(1.0),
            Preserve::Eighth => Some(0.5),
            Preserve::Sixteenth => Some(0.25),
            Preserve::ThirtySecond => Some(0.125),
        }
    }

    pub fn step(self, by: i32) -> Self {
        let i = Self::ALL.iter().position(|&m| m == self).unwrap_or(0) as i32;
        Self::ALL[(i + by).rem_euclid(Self::ALL.len() as i32) as usize]
    }
}

/// What a Beats piece does when the tempo is slower than the audio's and it
/// runs out before the next piece begins.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransientLoop {
    /// It stops.
    Off,
    /// Its second half loops.
    #[default]
    Forward,
    /// Its second half plays back and forth.
    BackForth,
}

impl TransientLoop {
    pub const ALL: [TransientLoop; 3] = [TransientLoop::Off, TransientLoop::Forward, TransientLoop::BackForth];

    pub fn name(self) -> &'static str {
        match self {
            TransientLoop::Off => "Loop Off",
            TransientLoop::Forward => "Loop Forward",
            TransientLoop::BackForth => "Loop Back-and-Forth",
        }
    }

    pub fn step(self, by: i32) -> Self {
        let i = Self::ALL.iter().position(|&m| m == self).unwrap_or(0) as i32;
        Self::ALL[(i + by).rem_euclid(Self::ALL.len() as i32) as usize]
    }
}

/// Where a clip is in its playing: clip time `c`, inside the stretch from
/// `c0` to `c1` it runs straight through (the way from the Start marker to
/// the loop's end, a time round the loop, or Start to End). `round` counts
/// the times round the loop (0: the first way through).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Piece {
    pub c: f64,
    pub c0: f64,
    pub c1: f64,
    pub round: u32,
}

/// Where a clip is, `u` into its playing: from `start` to the loop's end,
/// then round the loop, or once from `start` to `end`. The units are the
/// caller's (beats or seconds).
pub fn piece_of(u: f64, start: f64, end: f64, loop_start: f64, loop_end: f64, looping: bool) -> Option<Piece> {
    if u < -1e-9 || !u.is_finite() {
        return None;
    }
    let u = u.max(0.0);
    if !looping {
        let c = start + u;
        return (c < end).then_some(Piece { c, c0: start, c1: end, round: 0 });
    }
    let len = loop_end - loop_start;
    if len <= 1e-9 {
        return None;
    }
    // A Start marker after the loop's end starts at the loop.
    let start = if start >= loop_end - 1e-9 { loop_start } else { start };
    let first = loop_end - start;
    if u < first {
        return Some(Piece { c: start + u, c0: start, c1: loop_end, round: 0 });
    }
    let v = u - first;
    let k = (v / len).floor();
    let c = (loop_start + v - k * len).min(loop_end - 1e-12);
    Some(Piece { c, c0: loop_start, c1: loop_end, round: k as u32 + 1 })
}

impl AudioClip {
    /// The second in the sample that falls on clip beat `beat`.
    pub fn secs_at(&self, beat: f64) -> f64 {
        let spb = 60.0 / self.bpm.max(1.0);
        let m = &self.markers;
        let (Some(&first), Some(&last)) = (m.first(), m.last()) else { return beat * spb };
        if beat <= first.0 {
            return first.1 + (beat - first.0) * spb;
        }
        if beat >= last.0 {
            return last.1 + (beat - last.0) * spb;
        }
        let i = m.partition_point(|p| p.0 <= beat).clamp(1, m.len() - 1);
        let (a, b) = (m[i - 1], m[i]);
        a.1 + (beat - a.0) * (b.1 - a.1) / (b.0 - a.0).max(1e-12)
    }

    /// The clip beat that second `secs` of the sample falls on.
    pub fn beat_at(&self, secs: f64) -> f64 {
        let bps = self.bpm.max(1.0) / 60.0;
        let m = &self.markers;
        let (Some(&first), Some(&last)) = (m.first(), m.last()) else { return secs * bps };
        if secs <= first.1 {
            return first.0 + (secs - first.1) * bps;
        }
        if secs >= last.1 {
            return last.0 + (secs - last.1) * bps;
        }
        let i = m.partition_point(|p| p.1 <= secs).clamp(1, m.len() - 1);
        let (a, b) = (m[i - 1], m[i]);
        a.0 + (secs - a.1) * (b.0 - a.0) / (b.1 - a.1).max(1e-12)
    }

    /// The tempo of the material around clip beat `beat` (Live's Seg. BPM).
    pub fn bpm_at(&self, beat: f64) -> f64 {
        let m = &self.markers;
        if m.len() < 2 || beat < m[0].0 || beat >= m[m.len() - 1].0 {
            return self.bpm;
        }
        let i = m.partition_point(|p| p.0 <= beat).clamp(1, m.len() - 1);
        let (a, b) = (m[i - 1], m[i]);
        60.0 * (b.0 - a.0) / (b.1 - a.1).max(1e-9)
    }

    /// The clip beat of the sample's first frame, and of its end.
    pub fn sample_start_beat(&self) -> f64 {
        self.beat_at(0.0)
    }

    pub fn sample_end_beat(&self) -> f64 {
        self.beat_at(self.sample.duration())
    }

    /// The End marker (the sample's end when it has none).
    pub fn end_beat(&self) -> f64 {
        self.end.unwrap_or_else(|| self.sample_end_beat())
    }

    /// How much faster than written it reads, from Transpose and Detune.
    pub fn rate(&self) -> f64 {
        2f64.powf((self.transpose as f64 + self.detune as f64 / 100.0) / 12.0)
    }

    /// Whether Transpose and Detune do anything (not in Re-Pitch).
    pub fn pitch_applies(&self) -> bool {
        !(self.warp && self.mode == WarpMode::RePitch)
    }

    /// The audio it plays: the sample, or the sample backwards.
    pub fn source(&self) -> &SampleData {
        if self.reverse {
            self.sample.reversed_ready().unwrap_or(&self.sample)
        } else {
            &self.sample
        }
    }

    /// Where it is, `u` beats into its playing, in clip beats (a loop of
    /// `len` beats from `loop_start`).
    pub fn piece(&self, u: f64, len: f64) -> Option<Piece> {
        piece_of(u, self.start, self.end_beat(), self.loop_start, self.loop_start + len, self.looping)
    }

    /// The same for an unwarped clip, in seconds of the sample, `secs`
    /// seconds into its playing (at its own speed, after Transpose).
    fn piece_secs(&self, secs: f64, len: f64) -> Option<Piece> {
        piece_of(secs, self.secs_at(self.start), self.secs_at(self.end_beat()), self.secs_at(self.loop_start), self.secs_at(self.loop_start + len), self.looping)
    }

    /// The clip beat it plays, `u` song beats into its playing (a beat
    /// lasting `secs_per_beat`), for a playhead: None once it has ended.
    pub fn content_beat(&self, u: f64, len: f64, secs_per_beat: f64) -> Option<f64> {
        if self.warp {
            self.piece(u, len).map(|p| p.c)
        } else {
            self.piece_secs(u * secs_per_beat * self.rate(), len).map(|p| self.beat_at(p.c))
        }
    }

    /// How many song beats one way through takes when it does not loop.
    pub fn once_beats(&self, secs_per_beat: f64) -> f64 {
        if self.warp {
            self.end_beat() - self.start
        } else {
            (self.secs_at(self.end_beat()) - self.secs_at(self.start)) / self.rate() / secs_per_beat.max(1e-9)
        }
    }

    /// Puts a warp marker on clip beat `beat`, where the audio is now (so
    /// nothing moves until it is dragged). Returns its index.
    pub fn add_marker(&mut self, beat: f64) -> usize {
        if let Some(i) = self.markers.iter().position(|m| (m.0 - beat).abs() < 1e-3) {
            return i;
        }
        let secs = self.secs_at(beat);
        let i = self.markers.partition_point(|m| m.0 < beat);
        self.markers.insert(i, (beat, secs));
        i
    }

    /// Moves warp marker `i` to clip beat `beat` (between its neighbours):
    /// the audio under it goes along, the audio around stretches.
    pub fn move_marker(&mut self, i: usize, beat: f64) {
        let lo = if i > 0 { self.markers[i - 1].0 + 1e-3 } else { f64::NEG_INFINITY };
        let hi = self.markers.get(i + 1).map_or(f64::INFINITY, |m| m.0 - 1e-3);
        if let Some(m) = self.markers.get_mut(i) {
            m.0 = beat.clamp(lo, hi.max(lo));
        }
    }

    /// Moves which second of the sample warp marker `i` holds (dragging the
    /// audio under a marker), between its neighbours.
    pub fn move_marker_secs(&mut self, i: usize, secs: f64) {
        let lo = if i > 0 { self.markers[i - 1].1 + 1e-4 } else { f64::NEG_INFINITY };
        let hi = self.markers.get(i + 1).map_or(f64::INFINITY, |m| m.1 - 1e-4);
        if let Some(m) = self.markers.get_mut(i) {
            m.1 = secs.clamp(lo, hi.max(lo));
        }
    }

    /// Live's "Warp From Here": the markers after `i` go and the audio
    /// from it on runs straight at `bpm`.
    pub fn warp_from(&mut self, i: usize, bpm: f64) {
        if i < self.markers.len() {
            self.markers.truncate(i + 1);
            self.bpm = bpm.clamp(20.0, 999.0);
        }
    }

    /// The tempo times `f` (x2, /2): the markers' beats spread or squeezed
    /// around the first one, the seconds staying where they are.
    pub fn scale_tempo(&mut self, f: f64) {
        let new = (self.bpm * f).clamp(20.0, 999.0);
        let f = new / self.bpm.max(1e-9);
        let pivot = self.markers.first().map_or(0.0, |m| m.0);
        for m in &mut self.markers {
            m.0 = pivot + (m.0 - pivot) * f;
        }
        self.bpm = new;
    }

    /// Plays the sample backwards (or forwards again): the reversed audio
    /// with its markers, its loop and its Start and End mirrored, so the
    /// same stretch of sound plays the other way round.
    pub fn set_reverse(&mut self, on: bool, len: f64) {
        if on == self.reverse {
            return;
        }
        if on {
            self.sample.reversed();
        }
        // Mirrored around the middle of the sample's beats, so the sample
        // spans the same beats either way round (and twice is no change).
        let d = self.sample.duration();
        let pivot = self.sample_start_beat() + self.sample_end_beat();
        self.markers = self.markers.iter().rev().map(|&(b, s)| (pivot - b, d - s)).collect();
        if self.looping {
            self.loop_start = pivot - (self.loop_start + len);
            self.start = self.loop_start;
        } else {
            let (s0, e0) = (self.start, self.end_beat());
            self.start = pivot - e0;
            self.end = self.end.map(|_| pivot - s0);
        }
        self.reverse = on;
    }
}

/// The gain of a fade `x` of the way in (0..1): equal power, or quicker or
/// slower with `curve`.
pub fn fade_shape(x: f64, curve: f32) -> f32 {
    let p = 2f64.powf(-2.0 * curve as f64);
    (x.clamp(0.0, 1.0).powf(p) * FRAC_PI_2).sin() as f32
}

/// How long the cut edges fade, in samples (Live's Clip Fade).
const FADE: f64 = 48.0;
/// Most grains a track keeps track of at once.
const SLOTS: usize = 8;

#[derive(Clone, Copy, Default)]
struct Grains {
    key: u64,
    /// The grain it is on (counted from where its piece starts), the source
    /// frame that grain started on, and the one before.
    k: i64,
    cur: f64,
    prev: f64,
    used: u64,
}

/// What a track's stretching remembers between blocks: where the grains of
/// the pieces it plays started (a fixed table, nothing allocated).
#[derive(Clone, Default)]
pub struct Stretch {
    slots: [Grains; SLOTS],
    clock: u64,
}

impl Stretch {
    fn slot(&mut self, key: u64) -> &mut Grains {
        self.clock += 1;
        let i = match self.slots.iter().position(|s| s.key == key) {
            Some(i) => i,
            None => {
                let i = (0..SLOTS).min_by_key(|&i| self.slots[i].used).unwrap_or(0);
                self.slots[i] = Grains { key, k: i64::MIN, cur: 0.0, prev: 0.0, used: 0 };
                i
            }
        };
        self.slots[i].used = self.clock;
        &mut self.slots[i]
    }
}

/// Where a clip lies on the song: the beat its playing began on (`origin`),
/// the stretch of song it sounds in, the fades there (beats), and a number
/// that tells it apart from the track's other clips.
#[derive(Clone, Copy, Debug)]
pub struct Place {
    pub origin: f64,
    pub region: (f64, f64),
    pub fades: (f64, f64),
    pub key: u64,
}

fn mix(a: u64, b: u64) -> u64 {
    let mut x = a ^ b.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 31;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 29;
    x | 1
}

/// A stretch of the source between cuts: from clip beat `c0` (source frame
/// `src0`) to `c1`; `next` is the frame of the next cut it must not reach.
#[derive(Clone, Copy, Debug)]
struct Seg {
    c0: f64,
    c1: f64,
    src0: f64,
    next: f64,
    /// It starts on a transient (a sharp cut) rather than on a grid line.
    hit: bool,
}

enum Cuts<'a> {
    None,
    Grid(f64),
    Hits(&'a [usize]),
}

/// One clip's playing for one block: the clip and what follows from it.
struct Voice<'a> {
    a: &'a AudioClip,
    src: &'a SampleData,
    frames: f64,
    /// Source frames per second, output samples per song beat and per second.
    ssr: f64,
    spb: f64,
    sr: f64,
    len: f64,
    /// Source frames per output sample from Transpose (and the rates).
    rho: f64,
    cuts: Cuts<'a>,
    /// The grains' hop in output samples and how far a grain may move to
    /// line up, in source frames.
    hop: f64,
    reach: f64,
    key: u64,
}

impl<'a> Voice<'a> {
    fn new(a: &'a AudioClip, len: f64, spb: f64, sr: f64, key: u64) -> Self {
        let src = a.source();
        let ssr = src.sample_rate as f64;
        let hits = || src.analysis_ready().map(|an| an.transients.as_slice());
        let cuts = match a.mode {
            WarpMode::Beats => match a.preserve.grid() {
                Some(g) => Cuts::Grid(g),
                None => hits().map_or(Cuts::Grid(0.25), Cuts::Hits),
            },
            WarpMode::Complex => hits().map_or(Cuts::None, Cuts::Hits),
            _ => Cuts::None,
        };
        let grain = 0.005 * 16f64.powf(a.grain.clamp(0.0, 1.0) as f64);
        let (hop, reach) = match a.mode {
            WarpMode::Complex => (0.021, 0.012),
            _ => (grain, (grain * 0.5).clamp(0.006, 0.02)),
        };
        Voice {
            a,
            src,
            frames: src.frames() as f64,
            ssr,
            spb,
            sr,
            len,
            rho: ssr / sr * a.rate(),
            cuts,
            hop: (hop * sr).max(16.0),
            reach: reach * ssr,
            key,
        }
    }

    /// The clip's sound `u` song beats into its playing, and how far that
    /// is from a cut edge (to fade it there): None where it is silent.
    fn at(&self, u: f64, st: &mut Stretch) -> Option<(f32, f32, f64)> {
        let a = self.a;
        let xf = (a.loop_xfade as f64 * 0.001).max(0.0);
        if !a.warp {
            // Seconds of the sample from seconds of the song, at its own speed.
            let p = a.piece_secs(u * self.spb / self.sr * a.rate(), self.len)?;
            return self.straight(p, |c| c * self.ssr, xf * self.ssr);
        }
        if a.mode == WarpMode::RePitch {
            let p = a.piece(u, self.len)?;
            return self.straight(p, |c| a.secs_at(c) * self.ssr, xf * self.ssr);
        }
        let p = a.piece(u, self.len)?;
        let s = self.segment(&p, p.c);
        let n = (p.c - s.c0) * self.spb;
        let key = self.seg_key(u - (p.c - s.c0));
        let (mut x, mut y) = self.play(&s, n, key, st);
        // The piece before it rings on for a moment, crossfading.
        if let Some((ps, cp, len, power)) = self.before(&p, &s) {
            if n < len {
                let t = n / len;
                let (wi, wo) = if power { ((t * FRAC_PI_2).sin(), (t * FRAC_PI_2).cos()) } else { (t, 1.0 - t) };
                let np = (cp - ps.c0) * self.spb;
                let (px, py) = self.play(&ps, np, self.seg_key(u - (cp - ps.c0)), st);
                x = x * wi as f32 + px * wo as f32;
                y = y * wi as f32 + py * wo as f32;
            }
        }
        let edge = if a.looping { f64::INFINITY } else { (p.c1 - p.c) * self.spb };
        Some((x, y, edge))
    }

    /// Re-Pitch and unwarped: the sample read straight through; `pos` turns
    /// clip time into a source frame. The edges fade as they always did; a
    /// loop crossfade (`xf` frames) lets the end ring into the start.
    fn straight(&self, p: Piece, pos: impl Fn(f64) -> f64, xf: f64) -> Option<(f32, f32, f64)> {
        let at = pos(p.c);
        if at < 0.0 || at >= self.frames {
            return None;
        }
        let (p0, p1) = (pos(p.c0), pos(p.c1));
        let crossfade = xf > 0.0 && self.a.looping;
        let mut edge = (self.frames - at).min(at);
        if !crossfade || p.round == 0 {
            edge = edge.min(at - p0);
        }
        if !crossfade {
            edge = edge.min(p1 - at);
        }
        let (mut x, mut y) = self.src.read(at);
        if crossfade && p.round >= 1 && at - p0 < xf {
            // The way round before goes on past the loop's end, fading out.
            let t = (at - p0) / xf;
            let (wi, wo) = ((t * FRAC_PI_2).sin() as f32, (t * FRAC_PI_2).cos() as f32);
            let (px, py) = self.src.read(p1 + (at - p0));
            x = x * wi + px * wo;
            y = y * wi + py * wo;
        }
        Some((x, y, edge.max(0.0)))
    }

    fn seg_key(&self, u_seg: f64) -> u64 {
        mix(self.key, (u_seg * 1e6).round() as i64 as u64)
    }

    /// The stretch between cuts that clip time `c` (in piece `p`) is in.
    fn segment(&self, p: &Piece, c: f64) -> Seg {
        let a = self.a;
        let (b0, b1, hit) = match self.cuts {
            Cuts::None => (f64::NEG_INFINITY, f64::INFINITY, false),
            Cuts::Grid(g) => {
                let b = ((c / g) + 1e-9).floor() * g;
                (b, b + g, false)
            }
            Cuts::Hits(ts) => {
                let x = a.secs_at(c) * self.ssr;
                let i = ts.partition_point(|&t| (t as f64) <= x + 1e-6);
                let b0 = if i > 0 { a.beat_at(ts[i - 1] as f64 / self.ssr) } else { f64::NEG_INFINITY };
                let b1 = ts.get(i).map_or(f64::INFINITY, |&t| a.beat_at(t as f64 / self.ssr));
                (b0, b1, i > 0)
            }
        };
        let c0 = b0.max(p.c0);
        let next = if b1.is_finite() { a.secs_at(b1) * self.ssr } else { f64::INFINITY };
        Seg { c0, c1: b1.min(p.c1), src0: a.secs_at(c0) * self.ssr, next, hit: hit && b0 >= p.c0 }
    }

    /// The stretch played just before `s` began (and how far into it clip
    /// time has gone on), with how long the two crossfade (output samples)
    /// and whether at equal power: None where `s` is where playing began.
    fn before(&self, p: &Piece, s: &Seg) -> Option<(Seg, f64, f64, bool)> {
        let a = self.a;
        let ms = |t: f64| t * self.sr;
        if s.c0 > p.c0 + 1e-9 {
            let ps = self.segment(p, s.c0 - 1e-4);
            let len = if s.hit { ms(0.0007) } else { ms(0.003) };
            return Some((ps, p.c, len, false));
        }
        if p.round == 0 {
            return None;
        }
        // Round the loop: the last stretch before its end, gone on past it.
        let le = p.c1;
        let c0 = if p.round == 1 { if a.start >= le - 1e-9 { a.loop_start } else { a.start } } else { a.loop_start };
        let pp = Piece { c: le + (p.c - p.c0), c0, c1: le, round: p.round - 1 };
        let ps = self.segment(&pp, le - 1e-4);
        let xf = a.loop_xfade as f64 * 0.001;
        Some((ps, pp.c, ms(xf.max(0.002)), xf > 0.0))
    }

    /// Stretch `s`, `n` output samples after it began.
    fn play(&self, s: &Seg, n: f64, key: u64, st: &mut Stretch) -> (f32, f32) {
        match self.a.mode {
            WarpMode::Beats => self.beats(s, n),
            _ => self.grains(s, n, key, st),
        }
    }

    /// A source frame, faded where the sample ends and before `next`.
    fn tap(&self, pos: f64, next: f64) -> (f32, f32) {
        if pos < 0.0 || pos >= self.frames {
            return (0.0, 0.0);
        }
        let mut g = 1.0f64;
        if self.a.declick {
            g = ((self.frames - pos) / FADE).min(1.0);
        }
        if next.is_finite() {
            g = g.min((next - pos) / (0.002 * self.ssr));
        }
        if g <= 0.0 {
            return (0.0, 0.0);
        }
        let (x, y) = self.src.read(pos);
        let g = g.min(1.0) as f32;
        (x * g, y * g)
    }

    /// Beats: the stretch at its own speed from its start, then (when it
    /// runs out before the next) silence or a loop of its second half.
    fn beats(&self, s: &Seg, n: f64) -> (f32, f32) {
        let a = self.a;
        let p = n * self.rho;
        let avail = if s.next.is_finite() { s.next - s.src0 } else { self.frames - s.src0 };
        if avail <= 1.0 {
            return (0.0, 0.0);
        }
        let mut g = 1.0f32;
        if a.transient_env < 0.999 && s.c1.is_finite() {
            let t = n / ((s.c1 - s.c0) * self.spb).max(1.0);
            g = ((1.0 - t).clamp(0.0, 1.0) as f32).powf((1.0 - a.transient_env.clamp(0.0, 1.0)) * 4.0);
        }
        // Whether it will run out before the next stretch takes over.
        let runs_out = !s.c1.is_finite() || (s.c1 - s.c0) * self.spb * self.rho > avail + 1.0;
        let half = avail * 0.5;
        let xf = (0.004 * self.ssr).min(half * 0.5);
        let (x, y) = match a.transient_loop {
            _ if p < avail - xf || !runs_out => self.src.read(s.src0 + p),
            TransientLoop::Off => {
                if p >= avail {
                    return (0.0, 0.0);
                }
                let (x, y) = self.src.read(s.src0 + p);
                let f = ((avail - p) / xf) as f32;
                (x * f, y * f)
            }
            TransientLoop::Forward => {
                // The second half round and round, its end crossfading into its start.
                let q = (p - half).rem_euclid(half);
                let at = s.src0 + half + q;
                if q > half - xf {
                    let t = ((q - (half - xf)) / xf) as f32;
                    let (x0, y0) = self.src.read(at);
                    let (x1, y1) = self.src.read(at - half);
                    (x0 * (1.0 - t) + x1 * t, y0 * (1.0 - t) + y1 * t)
                } else {
                    self.src.read(at)
                }
            }
            TransientLoop::BackForth => {
                if p < avail {
                    self.src.read(s.src0 + p)
                } else {
                    let q = (p - avail).rem_euclid(2.0 * half);
                    let back = if q < half { q } else { 2.0 * half - q };
                    self.src.read(s.src0 + avail - back)
                }
            }
        };
        (x * g, y * g)
    }

    /// Where grain `k` of stretch `s` would read if nothing lined up.
    fn ideal(&self, s: &Seg, k: i64) -> f64 {
        self.a.secs_at(s.c0 + k as f64 * self.hop / self.spb) * self.ssr
    }

    /// Tones, Texture, Complex: two grains crossfading at every moment, each
    /// lined up with the one before.
    fn grains(&self, s: &Seg, n: f64, key: u64, st: &mut Stretch) -> (f32, f32) {
        let h = self.hop;
        let k = (n / h).floor() as i64;
        let x = n - k as f64 * h;
        let slot = st.slot(key);
        if slot.k != k {
            if k == 0 {
                slot.cur = s.src0;
                slot.prev = s.src0;
            } else if slot.k == k - 1 {
                slot.prev = slot.cur;
                slot.cur = self.next_grain(s, k, slot.prev);
            } else {
                // Come in from elsewhere: the grain before as it would be.
                let prev = if k == 1 { s.src0 } else { self.limit(s, self.ideal(s, k - 1)) };
                slot.prev = prev;
                slot.cur = self.next_grain(s, k, prev);
            }
            slot.k = k;
        }
        let (cur, prev) = (slot.cur, slot.prev);
        let r = self.rho;
        if k == 0 {
            return self.tap(cur + x * r, s.next);
        }
        let w = (0.5 - 0.5 * (PI * x / h).cos()) as f32;
        let (x0, y0) = self.tap(cur + x * r, s.next);
        let (x1, y1) = self.tap(prev + (x + h) * r, s.next);
        (x0 * w + x1 * (1.0 - w), y0 * w + y1 * (1.0 - w))
    }

    /// Keeps a grain inside its stretch: it starts no earlier than the cut
    /// and late enough to end before the next one (near the end of a slowed
    /// stretch its last moment is heard again rather than the next hit too
    /// soon). A stretch shorter than a few grains only stops at the cut.
    fn limit(&self, s: &Seg, pos: f64) -> f64 {
        let span = self.hop * self.rho;
        let hi = match s.next {
            n if !n.is_finite() => f64::INFINITY,
            n if n - 2.0 * span >= s.src0 + span => n - 2.0 * span,
            n => pos.min(n),
        };
        pos.clamp(s.src0, hi.max(s.src0))
    }

    fn next_grain(&self, s: &Seg, k: i64, prev: f64) -> f64 {
        let ideal = self.ideal(s, k);
        let centre = self.limit(s, ideal);
        let hi = self.limit(s, centre + self.reach);
        let lo = (centre - self.reach).max(s.src0);
        let at = self.align(prev + self.hop * self.rho, lo, hi.max(lo));
        if self.a.mode == WarpMode::Texture && self.a.flux > 0.0 {
            // Texture: then it wanders off by up to two grains.
            let r = (mix(self.key ^ 0x7E57, k as u64) >> 11) as f64 / (1u64 << 53) as f64;
            let wander = (r - 0.5) * 2.0 * self.a.flux as f64 * self.hop * self.rho * 2.0;
            return self.limit(s, at + wander);
        }
        at
    }

    /// The start between `lo` and `hi` whose next `hop` output samples look
    /// most like those from `target` (normalised cross correlation of the
    /// mono sum, every few frames, then to the frame).
    fn align(&self, target: f64, lo: f64, hi: f64) -> f64 {
        if hi - lo < 1.0 {
            return lo;
        }
        let l = &self.src.left;
        let rr = &self.src.right;
        let mono = |p: f64| -> f32 {
            if p < 0.0 {
                return 0.0;
            }
            let i = p as usize;
            match (l.get(i), rr.get(i)) {
                (Some(a), Some(b)) => a + b,
                _ => 0.0,
            }
        };
        const TERMS: usize = 128;
        let span = self.hop * self.rho;
        let step = (span / TERMS as f64).max(1.0);
        let terms = ((span / step) as usize).clamp(4, TERMS);
        let mut tgt = [0.0f32; TERMS];
        for (j, t) in tgt.iter_mut().enumerate().take(terms) {
            *t = mono(target + j as f64 * step);
        }
        let score = |c: f64| -> f32 {
            let (mut xy, mut yy) = (0.0f32, 1e-9f32);
            for (j, t) in tgt.iter().enumerate().take(terms) {
                let v = mono(c + j as f64 * step);
                xy += t * v;
                yy += v * v;
            }
            xy / yy.sqrt()
        };
        let coarse = (self.ssr / 11025.0).round().max(1.0);
        let mut best = (lo, f32::MIN);
        let mut c = lo;
        while c <= hi {
            let v = score(c);
            if v > best.1 {
                best = (c, v);
            }
            c += coarse;
        }
        let around = best.0;
        let mut c = (around - coarse).max(lo);
        while c <= (around + coarse).min(hi) {
            let v = score(c);
            if v > best.1 {
                best = (c, v);
            }
            c += 1.0;
        }
        best.0
    }
}

/// Mixes an audio clip into `l`/`r` for one block: song beat `w0` at the
/// first sample, `spb` samples per song beat at `sr`. `len` is its loop.
#[allow(clippy::too_many_arguments)]
pub fn render(a: &AudioClip, len: f64, at: Place, w0: f64, spb: f64, sr: f32, seg: usize, l: &mut [f32], r: &mut [f32], st: &mut Stretch) {
    let v = Voice::new(a, len.max(1.0 / 64.0), spb, sr as f64, at.key);
    if v.frames < 2.0 {
        return;
    }
    let gain = crate::dsp::db_to_gain(a.gain_db);
    let pan = a.pan.clamp(-1.0, 1.0);
    let (gl, gr) = (gain * (1.0 - pan).min(1.0), gain * (1.0 + pan).min(1.0));
    let (region_start, region_end) = at.region;
    let (fade_in, fade_out) = at.fades;
    for i in 0..seg.min(l.len()).min(r.len()) {
        let g = w0 + i as f64 / spb;
        if g < region_start || g >= region_end {
            continue;
        }
        let Some((x, y, edge)) = v.at(g - at.origin, st) else { continue };
        // No click where a cut clip starts or ends either; and its fades.
        let mut e = 1.0f32;
        if a.declick {
            let region = ((g - region_start) * spb).min((region_end - g) * spb);
            e = (edge.min(region) / FADE).clamp(0.0, 1.0) as f32;
        }
        if fade_in > 1e-9 {
            e *= fade_shape((g - region_start) / fade_in, a.fade_curve);
        }
        if fade_out > 1e-9 {
            e *= fade_shape((region_end - g) / fade_out, a.fade_curve);
        }
        l[i] += x * gl * e;
        r[i] += y * gr * e;
    }
}

/// A number that tells the clips of a track apart for [`Stretch`].
pub fn key_of(a: u64, b: u64) -> u64 {
    mix(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::SampleRef;

    fn sine(hz: f64, secs: f64, sr: u32) -> SampleRef {
        let n = (secs * sr as f64) as usize;
        let x: Vec<f32> = (0..n).map(|i| (0.5 * (std::f64::consts::TAU * hz * i as f64 / sr as f64).sin()) as f32).collect();
        SampleRef::new(SampleData::from_buffers("sine", None, sr, x.clone(), x))
    }

    /// Renders `beats` song beats of a clip that starts at beat 0, at `bpm`.
    fn play(a: &AudioClip, len: f64, beats: f64, bpm: f64, sr: f32) -> Vec<f32> {
        let spb = sr as f64 * 60.0 / bpm;
        let n = (beats * spb) as usize;
        let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
        let mut st = Stretch::default();
        let at = Place { origin: 0.0, region: (0.0, f64::INFINITY), fades: (0.0, 0.0), key: 7 };
        let mut i = 0;
        while i < n {
            let seg = (n - i).min(256);
            render(a, len, at, i as f64 / spb, spb, sr, seg, &mut l[i..], &mut r[i..], &mut st);
            i += seg;
        }
        l
    }

    /// The frequency of a steady tone, from its zero crossings.
    fn frequency(x: &[f32], sr: f32) -> f64 {
        let ups: Vec<f64> = x.windows(2).enumerate().filter(|(_, w)| w[0] <= 0.0 && w[1] > 0.0).map(|(i, w)| i as f64 + (-w[0] / (w[1] - w[0])) as f64).collect();
        let (Some(a), Some(b)) = (ups.first(), ups.last()) else { return 0.0 };
        (ups.len() - 1) as f64 / ((b - a) / sr as f64)
    }

    /// The strongest frequency between 400 and 480 Hz, to half a hertz.
    fn spectral_peak(x: &[f32], sr: f32) -> f64 {
        let power = |f: f64| {
            let w = std::f64::consts::TAU * f / sr as f64;
            let (mut re, mut im) = (0.0, 0.0);
            for (i, &v) in x.iter().enumerate() {
                re += v as f64 * (w * i as f64).cos();
                im += v as f64 * (w * i as f64).sin();
            }
            re * re + im * im
        };
        (0..=160).map(|k| 400.0 + k as f64 * 0.5).max_by(|a, b| power(*a).total_cmp(&power(*b))).unwrap_or(0.0)
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    #[test]
    fn markers_map_beats_and_seconds_both_ways() {
        let mut a = AudioClip::new(sine(440.0, 8.0, 8000), 120.0);
        assert!((a.secs_at(4.0) - 2.0).abs() < 1e-12);
        a.markers = vec![(0.0, 0.1), (4.0, 2.6), (8.0, 4.6)];
        assert!((a.secs_at(2.0) - 1.35).abs() < 1e-12);
        assert!((a.secs_at(-1.0) - (0.1 - 0.5)).abs() < 1e-12, "the tempo before the first");
        assert!((a.secs_at(10.0) - 5.6).abs() < 1e-12, "and after the last");
        for b in [-3.0, -0.2, 0.0, 1.0, 3.9, 4.0, 6.5, 12.0] {
            assert!((a.beat_at(a.secs_at(b)) - b).abs() < 1e-9, "{b}");
        }
        assert!((a.bpm_at(1.0) - 60.0 * 4.0 / 2.5).abs() < 1e-9);
        // A marker added where the audio is changes nothing; moved, the audio follows.
        let before = a.secs_at(5.0);
        let i = a.add_marker(5.0);
        assert_eq!(i, 2);
        assert!((a.secs_at(5.0) - before).abs() < 1e-12);
        a.move_marker(i, 6.0);
        assert!((a.secs_at(6.0) - before).abs() < 1e-12);
        a.move_marker(i, 100.0);
        assert!(a.markers[i].0 < 8.0, "stays before its neighbour");
        a.warp_from(1, 100.0);
        assert_eq!(a.markers.len(), 2);
        assert_eq!(a.bpm, 100.0);
    }

    #[test]
    fn start_end_and_loop() {
        // From beat 1 to the loop's end at 4, then round the loop 2..4.
        let p = |u| piece_of(u, 1.0, 9.0, 2.0, 4.0, true).map(|p| (p.c, p.round));
        assert_eq!(p(0.0), Some((1.0, 0)));
        assert_eq!(p(2.5), Some((3.5, 0)));
        assert_eq!(p(3.0), Some((2.0, 1)));
        assert_eq!(p(5.5), Some((2.5, 2)));
        // Not looping: from the Start to the End marker, then nothing.
        let q = |u| piece_of(u, 1.0, 3.0, 2.0, 4.0, false).map(|p| p.c);
        assert_eq!(q(1.5), Some(2.5));
        assert_eq!(q(2.0), None);
        // The arrangement folds an offset into the first time round.
        let mut c = crate::project::Clip::new_audio(1, sine(100.0, 4.0, 8000), [0, 0, 0], 120.0);
        c.length = 2.0;
        if let crate::project::ClipContent::Audio(a) = &mut c.content {
            a.start = 1.0;
            a.loop_start = 2.0;
            a.mode = WarpMode::RePitch;
        }
        assert!((c.fold_offset(2.5) - 2.5).abs() < 1e-12);
        assert!((c.fold_offset(5.5) - 3.5).abs() < 1e-12);
    }

    #[test]
    fn stretching_keeps_the_pitch_and_the_length() {
        let sr = 44100.0f32;
        for mode in [WarpMode::Complex, WarpMode::Tones, WarpMode::Texture] {
            for bpm in [84.0, 120.0, 150.0] {
                // A 440 Hz tone, four beats at 120 BPM, played at another tempo.
                let mut a = AudioClip::new(sine(440.0, 2.0, 44100), 120.0);
                a.mode = mode;
                a.looping = false;
                let out = play(&a, 4.0, 6.0, bpm, sr);
                let spb = sr as f64 * 60.0 / bpm;
                let body = &out[(0.3 * spb) as usize..(3.7 * spb) as usize];
                // Unaligned grains blur the waveform, so Texture is measured by its spectrum.
                let (f, tol) = if mode == WarpMode::Texture { (spectral_peak(body, sr), 3.0) } else { (frequency(body, sr), 1.0) };
                assert!((f - 440.0).abs() < tol, "{mode:?} at {bpm}: {f:.2} Hz");
                // Four beats long at the new tempo, then silence.
                let (inside, after) = (rms(&out[(3.5 * spb) as usize..(3.9 * spb) as usize]), rms(&out[(4.05 * spb) as usize..]));
                assert!(inside > 0.3 && after < 1e-4, "{mode:?} at {bpm}: {inside} {after}");
                // Lined-up grains keep the level steady.
                if mode != WarpMode::Texture {
                    let w = (0.05 * sr) as usize;
                    let levels: Vec<f32> = body.chunks(w).map(rms).collect();
                    let (lo, hi) = levels.iter().fold((f32::MAX, 0.0f32), |(a, b), &v| (a.min(v), b.max(v)));
                    assert!(hi / lo < 1.06, "{mode:?} at {bpm}: level {lo}..{hi}");
                }
            }
        }
    }

    #[test]
    fn transposing_moves_the_pitch_but_not_the_time() {
        let sr = 44100.0f32;
        let mut a = AudioClip::new(sine(440.0, 2.0, 44100), 120.0);
        a.mode = WarpMode::Tones;
        a.transpose = 7.0;
        let out = play(&a, 4.0, 4.0, 100.0, sr);
        let spb = sr as f64 * 60.0 / 100.0;
        let f = frequency(&out[(0.3 * spb) as usize..(3.5 * spb) as usize], sr);
        let want = 440.0 * 2f64.powf(7.0 / 12.0);
        assert!((f - want).abs() < 2.0, "{f} vs {want}");
        // Re-Pitch ignores it: speed and pitch together from the tempo.
        a.mode = WarpMode::RePitch;
        let out = play(&a, 4.0, 4.0, 100.0, sr);
        let f = frequency(&out[(0.3 * spb) as usize..(3.5 * spb) as usize], sr);
        assert!((f - 440.0 * 100.0 / 120.0).abs() < 1.0, "{f}");
    }

    /// A decaying 800 Hz click on every beat (`decay` per second, cut
    /// after `secs`).
    fn clicks(bpm: f64, beats: usize, sr: u32) -> SampleRef {
        clicks_with(bpm, beats, sr, 30.0, 0.12)
    }

    fn clicks_with(bpm: f64, beats: usize, sr: u32, decay: f64, secs: f64) -> SampleRef {
        let spb = 60.0 / bpm * sr as f64;
        let n = (beats as f64 * spb) as usize;
        let mut x = vec![0.0f32; n];
        for b in 0..beats {
            let at = (b as f64 * spb) as usize;
            for i in 0..(secs * sr as f64) as usize {
                if let Some(s) = x.get_mut(at + i) {
                    let t = i as f64 / sr as f64;
                    *s = (0.8 * (std::f64::consts::TAU * 800.0 * t).sin() * (-t * decay).exp()) as f32;
                }
            }
        }
        let s = SampleData::from_buffers("clicks", None, sr, x.clone(), x);
        s.analysis();
        SampleRef::new(s)
    }

    /// Where the hits start: the first sample over a tenth after silence.
    fn hits(x: &[f32], gap: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut quiet = gap;
        for (i, v) in x.iter().enumerate() {
            if v.abs() > 0.1 {
                if quiet >= gap {
                    out.push(i);
                }
                quiet = 0;
            } else {
                quiet += 1;
            }
        }
        out
    }

    #[test]
    fn beats_keeps_hits_sharp_and_on_the_beat() {
        let sr = 44100.0f32;
        for (mode, bpm) in [(WarpMode::Beats, 70.0), (WarpMode::Beats, 140.0), (WarpMode::Complex, 75.0), (WarpMode::Complex, 135.0)] {
            let mut a = AudioClip::new(clicks(100.0, 8, 44100), 100.0);
            a.mode = mode;
            a.transient_loop = TransientLoop::Off;
            let out = play(&a, 8.0, 8.0, bpm, sr);
            let spb = sr as f64 * 60.0 / bpm;
            let got = hits(&out, (0.02 * sr) as usize);
            assert_eq!(got.len(), 8, "{mode:?} at {bpm}: {got:?}");
            for (k, &h) in got.iter().enumerate() {
                let want = k as f64 * spb;
                assert!((h as f64 - want).abs() < 0.002 * sr as f64, "{mode:?} at {bpm}: hit {k} at {h}, beat at {want}");
                // As sharp as the original: full level within a millisecond.
                let peak = out[h..h + (0.002 * sr) as usize].iter().fold(0.0f32, |m, v| m.max(v.abs()));
                assert!(peak > 0.5, "{mode:?} at {bpm}: hit {k} peaks at {peak}");
            }
            // Nothing of a hit is heard before its beat (no pre-echo).
            for k in 1..8 {
                let at = (k as f64 * spb) as usize;
                let pre = &out[at - (0.01 * sr) as usize..at - (0.001 * sr) as usize];
                let base = &out[at - (0.03 * sr) as usize..at - (0.02 * sr) as usize];
                assert!(rms(pre) <= rms(base) * 1.5 + 1e-3, "{mode:?} at {bpm}: before hit {k}");
            }
        }
    }

    #[test]
    fn beats_loops_the_tail_when_slower() {
        let sr = 44100.0f32;
        let mut a = AudioClip::new(clicks_with(100.0, 4, 44100, 4.0, 0.6), 100.0);
        a.mode = WarpMode::Beats;
        let spb = sr as f64 * 60.0 / 50.0;
        a.transient_loop = TransientLoop::Off;
        let off = play(&a, 4.0, 4.0, 50.0, sr);
        a.transient_loop = TransientLoop::Forward;
        let fwd = play(&a, 4.0, 4.0, 50.0, sr);
        // Late in a beat at half speed: silent when off, the tail looping when forward.
        let late = (0.8 * spb) as usize..(0.95 * spb) as usize;
        assert!(rms(&off[late.clone()]) < 1e-4);
        assert!(rms(&fwd[late]) > 1e-4);
    }

    #[test]
    fn reverse_plays_backwards_and_back_again() {
        let ramp: Vec<f32> = (0..1000).map(|i| i as f32 / 1000.0).collect();
        let s = SampleRef::new(SampleData::from_buffers("ramp", None, 1000, ramp.clone(), ramp));
        let mut a = AudioClip::new(s, 120.0);
        a.declick = false;
        a.set_reverse(true, 2.0);
        let out = play(&a, 2.0, 2.0, 120.0, 1000.0);
        assert!((out[100] - 0.899).abs() < 0.01, "{}", out[100]);
        assert!((out[900] - 0.099).abs() < 0.01, "{}", out[900]);
        a.markers = vec![(0.5, 0.2)];
        let m = a.markers.clone();
        a.set_reverse(false, 2.0);
        a.set_reverse(true, 2.0);
        assert!((a.markers[0].0 - m[0].0).abs() < 1e-9 && (a.markers[0].1 - m[0].1).abs() < 1e-9);
    }

    #[test]
    fn old_sets_load_as_warped_re_pitch_loops() {
        let old = r#"{"sample":{"name":"x","path":null},"bpm":96.0,"gain_db":-3.0}"#;
        let a: AudioClip = serde_json::from_str(old).unwrap();
        assert!(a.warp && a.looping && a.declick && !a.reverse);
        assert_eq!((a.mode, a.start, a.loop_start, a.end, a.transpose, a.gain_db), (WarpMode::RePitch, 0.0, 0.0, None, 0.0, -3.0));
        assert!(a.markers.is_empty());
        // And a clip that has nothing new writes nothing new.
        let back = serde_json::to_string(&a).unwrap();
        assert_eq!(back, r#"{"sample":{"name":"x","path":null},"bpm":96.0,"gain_db":-3.0}"#);
        let mut b = a.clone();
        b.mode = WarpMode::Beats;
        b.markers = vec![(0.0, 0.25)];
        b.looping = false;
        b.end = Some(6.0);
        let c: AudioClip = serde_json::from_str(&serde_json::to_string(&b).unwrap()).unwrap();
        assert_eq!((c.mode, c.markers.clone(), c.looping, c.end), (WarpMode::Beats, vec![(0.0, 0.25)], false, Some(6.0)));
    }

    #[test]
    fn an_old_clip_sounds_as_it_did() {
        // Re-Pitch from the start, looping: the sample read at the tempo's speed,
        // with the 48-sample fades at the loop's ends, as before warping came.
        let ramp: Vec<f32> = (0..1000).map(|i| i as f32 / 1000.0).collect();
        let s = SampleRef::new(SampleData::from_buffers("ramp", None, 1000, ramp.clone(), ramp));
        let a = AudioClip::new(s, 120.0);
        let out = play(&a, 2.0, 4.0, 60.0, 1000.0);
        // At 60 BPM a beat is a second here: half a second of the ramp per second.
        assert!((out[1000] - 0.5).abs() < 1e-3, "{}", out[1000]);
        assert!((out[2000 + 500] - 0.25).abs() < 1e-3, "round again: {}", out[2500]);
        assert!(out[2000] < 0.01, "the loop's seam fades");
    }

    #[test]
    fn a_warped_clip_renders_on_the_songs_beats() {
        use crate::project::{ArrangementClip, Clip, Project};
        use crate::render::{render, RenderSource};
        for (mode, bpm) in [(WarpMode::Beats, 128.0), (WarpMode::Complex, 84.0), (WarpMode::Tones, 132.0), (WarpMode::RePitch, 90.0)] {
            let mut p = Project::blank();
            p.bpm = bpm;
            let t = p.add_audio_track();
            let id = p.alloc_id();
            let cid = p.alloc_id();
            // Eight clicks at 100 BPM, warped into the song from bar 2.
            let mut clip = Clip::new_audio(cid, clicks(100.0, 8, 44100), [200, 100, 50], 120.0);
            if let crate::project::ClipContent::Audio(a) = &mut clip.content {
                assert!((a.bpm - 100.0).abs() < 1e-6, "an 8-beat loop at 100: {}", a.bpm);
                a.mode = mode;
                a.transient_loop = TransientLoop::Off;
            }
            p.tracks[t].arrangement.push(ArrangementClip::new(id, 4.0, 8.0, clip));
            p.prepare_audio();
            let out = render(&p, RenderSource::Arrangement { start: 0.0, end: 12.0 }, 44100, 0.0, None).unwrap();
            let left: Vec<f32> = out.iter().step_by(2).copied().collect();
            let peak = left.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            let norm: Vec<f32> = left.iter().map(|v| v / peak.max(1e-9)).collect();
            let got = hits(&norm, (0.02 * 44100.0) as usize);
            let spb = 44100.0 * 60.0 / bpm;
            assert_eq!(got.len(), 8, "{mode:?} at {bpm}: {got:?}");
            for (k, &h) in got.iter().enumerate() {
                let want = (4.0 + k as f64) * spb;
                // Tones lines grains up without minding the hits: they may move by up to its reach.
                let tol = if mode == WarpMode::Tones { 0.012 } else { 0.003 };
                assert!((h as f64 - want).abs() < tol * 44100.0, "{mode:?} at {bpm}: hit {k} at {h}, beat at {want}");
            }
        }
    }

    #[test]
    fn fades_and_pan() {
        assert!((fade_shape(0.5, 0.0) - (0.25 * PI).sin() as f32).abs() < 1e-6);
        assert!(fade_shape(0.5, 1.0) > fade_shape(0.5, 0.0));
        assert!(fade_shape(0.5, -1.0) < fade_shape(0.5, 0.0));
        let one: Vec<f32> = vec![0.5; 4000];
        let s = SampleRef::new(SampleData::from_buffers("dc", None, 1000, one.clone(), one));
        let mut a = AudioClip::new(s, 120.0);
        a.fade_in = 1.0;
        a.pan = -1.0;
        let spb = 500.0;
        let (mut l, mut r) = (vec![0.0f32; 1000], vec![0.0f32; 1000]);
        let at = Place { origin: 0.0, region: (0.0, 8.0), fades: (a.fade_in, 0.0), key: 1 };
        render(&a, 8.0, at, 0.0, spb, 1000.0, 1000, &mut l, &mut r, &mut Stretch::default());
        assert!(l[100] < l[400] && (l[700] - 0.5).abs() < 1e-3, "{} {} {}", l[100], l[400], l[700]);
        assert!(r.iter().all(|v| v.abs() < 1e-6), "all the way left");
    }
}
