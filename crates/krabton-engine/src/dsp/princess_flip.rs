//! Flipside (it was Princess Flip): a sample flip for pluggnb, to sit with
//! Analog Princess. A sample goes on it (a soul or R&B record, an anime or
//! game tune, the sound of a video, anything), it is listened to for its
//! tempo and its key (see `analysis::detect_tempo` and `analysis::detect_key`;
//! a key or tempo in its name counts first) and chopped into up to 32 chops
//! laid over the keys from C3 up: at its hits (as readily as *Sensitivity*
//! says), in bars or beats of its tempo, in equal parts, or only where lines
//! were put by hand. The lines can be moved, added and taken away, and a
//! chop moved keeps its own sound. The keys below (C1 to B2) play the
//! chosen chop at their pitch, C2 its own, for a melody of one chop.
//!
//! With *Song Key* every chop is moved by the difference between its key and
//! the song's (the nearer way round, to the relative key when one is major
//! and the other minor), so a flip in A minor plays in C minor up three.
//! Every chop has its own pitch, gain, pan, direction, trim, fades, tone,
//! way of playing (one-shot, gate, loop, ping-pong), choke group and whether
//! it is stretched (its own speed at another pitch) or played at another
//! speed as a record is. Then the flip's own sound: *Pitch* (varispeed:
//! slower is lower, as the slowed records are), *Slowed + Reverb* (a little
//! slower still, in a big room), *Half-Time* (the chop at half its speed and
//! its own pitch, read in short grains), an envelope, a filter (low, high or
//! band pass) that the envelope and the velocity can open, one voice or
//! many (mono, legato, with glide), an LFO in the song's time, the *Wobble*
//! of a worn tape or a warped record, a crusher, *Choke* (a chop cuts the
//! one before), *Stutter* (a held chop starts again every eighth to
//! sixty-fourth), *Hiss* (a record's, while it plays), *Fry* (Analog
//! Princess's FRY section: drive, ring, bits, downsampling and the clip into
//! the ceiling turned up together), the room and the stereo width.
//!
//! A set made before all this (no settings of its own in its chops, the
//! parameters it had then) sounds as it did: what is new starts where it
//! changes nothing (see the tests against `princess_flip_legacy`).

use super::fry::{Bus, BusKnobs};
use super::reverb::Reverb;
use super::{db_to_gain, decay_coef, FilterMode, OnePole, Rng, Svf};
use crate::params::{ParamSpec, Unit};
use crate::project::Note;
use crate::sample::{SampleData, SampleRef};
use crate::scale::{Scale, ScaleKind};
use serde::{Deserialize, Serialize};

pub const PITCH: usize = 0;
pub const SLOWED: usize = 1;
pub const SONG_KEY: usize = 2;
pub const HALF: usize = 3;
pub const MODE: usize = 4;
pub const CHOKE: usize = 5;
pub const STUTTER: usize = 6;
pub const ATTACK: usize = 7;
pub const RELEASE: usize = 8;
pub const WOBBLE: usize = 9;
pub const LOW_PASS: usize = 10;
pub const HISS: usize = 11;
pub const FRY: usize = 12;
pub const REVERB: usize = 13;
pub const VOLUME: usize = 14;
pub const CHOP: usize = 15;
pub const DECAY: usize = 16;
pub const SUSTAIN: usize = 17;
pub const VEL_LEVEL: usize = 18;
pub const FILTER_TYPE: usize = 19;
pub const RESONANCE: usize = 20;
pub const FILTER_ENV: usize = 21;
pub const VEL_FILTER: usize = 22;
pub const VEL_PITCH: usize = 23;
pub const VOICING: usize = 24;
pub const GLIDE: usize = 25;
pub const VOICES: usize = 26;
pub const LFO_RATE: usize = 27;
pub const LFO_SHAPE: usize = 28;
pub const LFO_PITCH: usize = 29;
pub const LFO_FILTER: usize = 30;
pub const LFO_PAN: usize = 31;
pub const WIDTH: usize = 32;
pub const WOBBLE_TYPE: usize = 33;
pub const BITS: usize = 34;
pub const CRUSH_RATE: usize = 35;

pub const MODES: &[&str] = &["One-Shot", "Gate"];
pub const STUTTERS: &[&str] = &["Off", "1/8", "1/16", "1/32", "1/64"];
const STUTTER_BEATS: [f64; 5] = [0.0, 0.5, 0.25, 0.125, 0.0625];
pub const FILTERS: &[&str] = &["Low Pass", "High Pass", "Band Pass"];
pub const VOICINGS: &[&str] = &["Poly", "Mono", "Legato"];
pub const LFO_RATES: &[&str] = &["4 Bars", "2 Bars", "1 Bar", "1/2", "1/4", "1/8", "1/16", "1/4 T", "1/8 T", "1/16 T"];
/// A cycle of each rate, in beats.
const LFO_BEATS: [f64; 10] = [16.0, 8.0, 4.0, 2.0, 1.0, 0.5, 0.25, 2.0 / 3.0, 1.0 / 3.0, 1.0 / 6.0];
pub const LFO_SHAPES: &[&str] = &["Sine", "Triangle", "Square", "Saw", "Random"];
pub const WOBBLES: &[&str] = &["Tape", "Vinyl", "Worn"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Pitch", "Flip", -12.0, 12.0, 0.0, Unit::Semi),
    ParamSpec::toggle("Slowed + Reverb", "Flip", false),
    ParamSpec::toggle("Song Key", "Flip", true),
    ParamSpec::toggle("Half-Time", "Flip", false),
    ParamSpec::choice("Mode", "Chops", MODES, 0),
    ParamSpec::toggle("Choke", "Chops", true),
    ParamSpec::choice("Stutter", "Chops", STUTTERS, 0),
    ParamSpec::log("Attack", "Chops", 0.5, 200.0, 1.0, Unit::Ms),
    ParamSpec::log("Release", "Chops", 5.0, 2000.0, 40.0, Unit::Ms),
    ParamSpec::lin("Wobble", "Tape", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Low Pass", "Tape", 200.0, 20_000.0, 20_000.0, Unit::Hz),
    ParamSpec::lin("Hiss", "Tape", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Fry", "Fry", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Reverb", "Fry", 0.0, 1.0, 0.12, Unit::Percent),
    ParamSpec::lin("Volume", "Fry", -30.0, 6.0, -3.0, Unit::Db),
    ParamSpec::int("Chop", "Chops", 1.0, 32.0, 1.0, Unit::None),
    ParamSpec::log("Decay", "Chops", 5.0, 5000.0, 400.0, Unit::Ms),
    ParamSpec::lin("Sustain", "Chops", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel > Level", "Chops", 0.0, 1.0, 0.65, Unit::Percent),
    ParamSpec::choice("Filter Type", "Filter", FILTERS, 0),
    ParamSpec::lin("Resonance", "Filter", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::lin("Env Amount", "Filter", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel > Filter", "Filter", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel > Pitch", "Voice", 0.0, 12.0, 0.0, Unit::Semi),
    ParamSpec::choice("Voicing", "Voice", VOICINGS, 0),
    ParamSpec::lin("Glide", "Voice", 0.0, 1000.0, 0.0, Unit::Ms),
    ParamSpec::int("Voices", "Voice", 1.0, 16.0, 8.0, Unit::None),
    ParamSpec::choice("LFO Rate", "LFO", LFO_RATES, 4),
    ParamSpec::choice("LFO Shape", "LFO", LFO_SHAPES, 0),
    ParamSpec::lin("LFO > Pitch", "LFO", 0.0, 12.0, 0.0, Unit::Semi),
    ParamSpec::lin("LFO > Filter", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("LFO > Pan", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Width", "Fry", 0.0, 2.0, 1.0, Unit::Percent),
    ParamSpec::choice("Wobble Type", "Tape", WOBBLES, 0),
    ParamSpec::int("Bits", "Crush", 2.0, 16.0, 16.0, Unit::None),
    ParamSpec::log("Crush Rate", "Crush", 1000.0, 48_000.0, 48_000.0, Unit::Hz),
];

/// The first chop's key (C3); the others go up from it.
pub const FIRST_NOTE: u8 = 60;
/// The most chops a flip has, and the fewest it is cut in at its hits.
pub const MAX_CHOPS: usize = 32;
pub const MIN_CHOPS: usize = 8;
/// The shortest a chop is (frames): lines keep this far apart.
pub const MIN_LEN: u64 = 64;
/// The keys that play the chosen chop at their pitch (C1 to B2), and the
/// one that plays it as it is (C2).
pub const LOW_KEYS: std::ops::RangeInclusive<u8> = 36..=59;
pub const LOW_ROOT: u8 = 48;
/// How much slower Slowed + Reverb plays (about three semitones down).
const SLOWED_SPEED: f64 = 0.84;
/// The most voices, and as many as a flip had before it could choose.
const MAX_VOICES: usize = 16;
const BLOCK: usize = 128;
/// Where the face finds what plays (see [`FlipState::visual`]).
pub const VIS_AT: usize = 32;
pub const VIS_SHIFT: usize = 33;
pub const VIS_VOICES: usize = 34;
pub const VIS_LFO: usize = 35;

/// How a chop plays when its key is held and let go.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Play {
    /// As the flip's Mode says.
    #[default]
    Flip,
    /// To its end, whatever the key does.
    OneShot,
    /// While its key is held.
    Gate,
    /// Round and round while held.
    Loop,
    /// To its end and back while held.
    PingPong,
}

impl Play {
    pub const ALL: [Play; 5] = [Play::Flip, Play::OneShot, Play::Gate, Play::Loop, Play::PingPong];

    pub fn name(self) -> &'static str {
        match self {
            Play::Flip => "As Flip",
            Play::OneShot => "One-Shot",
            Play::Gate => "Gate",
            Play::Loop => "Loop",
            Play::PingPong => "Ping-Pong",
        }
    }
}

fn is_zero(v: &f32) -> bool {
    *v == 0.0
}

fn is_zero_u8(v: &u8) -> bool {
    *v == 0
}

fn not(v: &bool) -> bool {
    !*v
}

fn open() -> f32 {
    20_000.0
}

fn is_open(v: &f32) -> bool {
    *v >= 20_000.0
}

fn is_flip(p: &Play) -> bool {
    *p == Play::Flip
}

/// A part of the sample a key plays (frames `start..end`), and how: its
/// own tuning, level, place, direction, trim, fades, tone, way of playing,
/// choke group and whether it is stretched. A chop of an older set has none
/// of these and plays as it did.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlipChop {
    pub start: u64,
    pub end: u64,
    #[serde(default)]
    pub reverse: bool,
    /// Semitones and cents.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub pitch: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub fine: f32,
    /// dB.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub gain: f32,
    /// -1 (left) to 1 (right).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub pan: f32,
    /// Its start and end moved by so many ms (the line stays where it is).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub trim_in: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub trim_out: f32,
    /// Fades in and out (ms; 0: just enough not to click).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub fade_in: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub fade_out: f32,
    /// Its own low pass (Hz; 20 kHz is open).
    #[serde(default = "open", skip_serializing_if = "is_open")]
    pub cutoff: f32,
    #[serde(default, skip_serializing_if = "is_flip")]
    pub play: Play,
    /// Chops of a group (1 to 4) cut each other; 0: as the flip's Choke.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub choke: u8,
    /// Kept at its own speed when its pitch moves (read in grains), not
    /// played faster or slower as a record is.
    #[serde(default, skip_serializing_if = "not")]
    pub stretch: bool,
    /// Its line was put or moved by hand: chopping again keeps it.
    #[serde(default, skip_serializing_if = "not")]
    pub manual: bool,
}

impl Default for FlipChop {
    fn default() -> Self {
        Self { start: 0, end: 0, reverse: false, pitch: 0.0, fine: 0.0, gain: 0.0, pan: 0.0, trim_in: 0.0, trim_out: 0.0, fade_in: 0.0, fade_out: 0.0, cutoff: open(), play: Play::Flip, choke: 0, stretch: false, manual: false }
    }
}

impl FlipChop {
    pub fn new(start: u64, end: u64) -> Self {
        Self { start, end, ..Self::default() }
    }

    /// The same sound on another part of the sample.
    fn sound_of(self, other: &FlipChop) -> Self {
        Self { start: self.start, end: self.end, manual: self.manual, ..*other }
    }

    /// Whether it sounds as a chop of its own (anything set on it).
    pub fn has_sound(&self) -> bool {
        self.sound_of(&FlipChop::default()) != *self
    }
}

/// The settings of a chop the face shows, by their place in [`CHOP_SPECS`].
pub static CHOP_SPECS: &[ParamSpec] = &[
    ParamSpec::int("Pitch", "Chop", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Fine", "Chop", -100.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::lin("Gain", "Chop", -24.0, 12.0, 0.0, Unit::Db),
    ParamSpec::lin("Pan", "Chop", -1.0, 1.0, 0.0, Unit::Pan),
    ParamSpec::lin("Trim In", "Chop", -200.0, 200.0, 0.0, Unit::Ms),
    ParamSpec::lin("Trim Out", "Chop", -200.0, 200.0, 0.0, Unit::Ms),
    ParamSpec::lin("Fade In", "Chop", 0.0, 1000.0, 0.0, Unit::Ms),
    ParamSpec::lin("Fade Out", "Chop", 0.0, 1000.0, 0.0, Unit::Ms),
    ParamSpec::log("Tone", "Chop", 100.0, 20_000.0, 20_000.0, Unit::Hz),
];

impl FlipChop {
    /// Its setting `k` of [`CHOP_SPECS`].
    pub fn setting(&mut self, k: usize) -> Option<&mut f32> {
        Some(match k {
            0 => &mut self.pitch,
            1 => &mut self.fine,
            2 => &mut self.gain,
            3 => &mut self.pan,
            4 => &mut self.trim_in,
            5 => &mut self.trim_out,
            6 => &mut self.fade_in,
            7 => &mut self.fade_out,
            8 => &mut self.cutoff,
            _ => return None,
        })
    }
}

/// How a sample is chopped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChopBy {
    /// At its hits: the strongest, in order.
    #[default]
    Hits,
    /// In bars (of its tempo, from its first downbeat) or beats of them;
    /// equal parts without a tempo.
    Bars,
    /// In equal parts.
    Equal,
    /// Only where lines were put by hand.
    Manual,
}

impl ChopBy {
    pub const ALL: [ChopBy; 4] = [ChopBy::Hits, ChopBy::Bars, ChopBy::Equal, ChopBy::Manual];

    pub fn name(self) -> &'static str {
        match self {
            ChopBy::Hits => "Hits",
            ChopBy::Bars => "Bars",
            ChopBy::Equal => "Equal",
            ChopBy::Manual => "Manual",
        }
    }
}

/// The grid of chopping in bars.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Grid {
    /// Bars, or halves, beats, half beats of them till there are enough.
    #[default]
    Auto,
    Bar,
    Half,
    Beat,
    Eighth,
    Sixteenth,
}

impl Grid {
    pub const ALL: [Grid; 6] = [Grid::Auto, Grid::Bar, Grid::Half, Grid::Beat, Grid::Eighth, Grid::Sixteenth];

    pub fn name(self) -> &'static str {
        match self {
            Grid::Auto => "Auto",
            Grid::Bar => "1 Bar",
            Grid::Half => "1/2",
            Grid::Beat => "1/4",
            Grid::Eighth => "1/8",
            Grid::Sixteenth => "1/16",
        }
    }

    fn beats(self) -> Option<f64> {
        match self {
            Grid::Auto => None,
            Grid::Bar => Some(4.0),
            Grid::Half => Some(2.0),
            Grid::Beat => Some(1.0),
            Grid::Eighth => Some(0.5),
            Grid::Sixteenth => Some(0.25),
        }
    }
}

fn twelve() -> usize {
    12
}

fn half() -> f32 {
    crate::analysis::WARP_SENSITIVITY
}

/// How a flip is chopped (again): what by, into how many (the most, at its
/// hits), how readily a hit counts, on what grid.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Chopping {
    #[serde(default)]
    pub by: ChopBy,
    #[serde(default = "twelve")]
    pub count: usize,
    #[serde(default = "half")]
    pub sensitivity: f32,
    #[serde(default)]
    pub grid: Grid,
}

impl Default for Chopping {
    fn default() -> Self {
        Self { by: ChopBy::Hits, count: 12, sensitivity: half(), grid: Grid::Auto }
    }
}

/// What a flip knows of its sample: its chops, its key (root, minor) and
/// tempo as they were found, and how it was chopped (saved with the set;
/// the sample is the instrument's own).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Flip {
    #[serde(default)]
    pub chops: Vec<FlipChop>,
    #[serde(default)]
    pub key: Option<(u8, bool)>,
    #[serde(default)]
    pub bpm: Option<f64>,
    #[serde(default)]
    pub chopping: Chopping,
}

impl Flip {
    pub fn key_name(&self) -> Option<String> {
        self.key.map(|(root, minor)| crate::analysis::Key { root, minor, confidence: 1.0 }.name())
    }

    /// The lines (each chop's start) in the order they are in the sample:
    /// (chop, frame).
    pub fn lines(&self) -> Vec<(usize, u64)> {
        let mut v: Vec<(usize, u64)> = self.chops.iter().enumerate().map(|(k, c)| (k, c.start)).collect();
        v.sort_by_key(|&(k, f)| (f, k));
        v
    }

    /// How far chop `k`'s line can go (its neighbours' lines and its own
    /// end, `MIN_LEN` away) in a sample of `frames`.
    pub fn line_bounds(&self, k: usize, frames: usize) -> (u64, u64) {
        let Some(c) = self.chops.get(k) else { return (0, 0) };
        let lines = self.lines();
        let at = lines.iter().position(|&(j, _)| j == k).unwrap_or(0);
        let lo = if at > 0 { lines[at - 1].1 + MIN_LEN } else { 0 };
        let next = lines.get(at + 1).map_or(frames as u64, |l| l.1);
        let hi = next.min(c.end).saturating_sub(MIN_LEN).max(lo);
        (lo, hi)
    }

    /// Chop `k`'s line moved to `to` (as far as its neighbours let it): the
    /// chop that ended there ends there still. Returns where it went.
    pub fn move_line(&mut self, k: usize, to: u64, frames: usize) -> u64 {
        if k >= self.chops.len() {
            return to;
        }
        let (lo, hi) = self.line_bounds(k, frames);
        let old = self.chops[k].start;
        let new = to.clamp(lo, hi);
        for (j, c) in self.chops.iter_mut().enumerate() {
            if j == k || c.start >= old {
                continue;
            }
            // The chop before: its end follows the line (and is cut where
            // the line moved into it).
            if c.end == old || c.end > new && c.end < old {
                c.end = new;
            }
        }
        let c = &mut self.chops[k];
        c.start = new;
        c.manual = true;
        new
    }

    /// A line put at `at`: the chop there cut in two (the new one sounds as
    /// the one it was cut from), or a chop of its own in a gap. Returns the
    /// new chop's place (after the one it was cut from).
    pub fn add_line(&mut self, at: u64, frames: usize) -> Option<usize> {
        let n = frames as u64;
        if self.chops.len() >= MAX_CHOPS || at >= n {
            return None;
        }
        if let Some(k) = self.chops.iter().position(|c| at >= c.start && at < c.end) {
            let c = self.chops[k];
            if at < c.start + MIN_LEN || at + MIN_LEN > c.end {
                return None;
            }
            self.chops[k].end = at;
            self.chops.insert(k + 1, FlipChop { start: at, end: c.end, manual: true, ..c });
            return Some(k + 1);
        }
        // In a gap: up to the next line (or the end).
        let next = self.chops.iter().map(|c| c.start).filter(|&s| s > at).min().unwrap_or(n);
        if next < at + MIN_LEN {
            return None;
        }
        let k = self.chops.iter().position(|c| c.start > at).unwrap_or(self.chops.len());
        self.chops.insert(k, FlipChop { manual: true, ..FlipChop::new(at, next) });
        Some(k)
    }

    /// Chop `k`'s line taken away: the chop before it (in the sample) runs
    /// on over it; the first just goes.
    pub fn delete_line(&mut self, k: usize) -> bool {
        let Some(c) = self.chops.get(k).copied() else { return false };
        if let Some(before) = self.chops.iter_mut().enumerate().find(|(j, b)| *j != k && b.end == c.start && b.start < c.start).map(|(_, b)| b) {
            before.end = c.end;
        }
        self.chops.remove(k);
        true
    }

    /// The chops in the order they are in the sample again (on the keys).
    pub fn sort(&mut self) {
        self.chops.sort_by_key(|c| c.start);
    }

    /// The chops on the keys in another order.
    pub fn shuffle(&mut self, seed: u32) {
        let mut rng = Rng::new(seed | 1);
        for i in (1..self.chops.len()).rev() {
            let j = rng.next_u32() as usize % (i + 1);
            self.chops.swap(i, j);
        }
    }

    /// The chops' sound thrown about: some an octave or a fifth away, some
    /// turned round, a little louder or softer, left or right.
    pub fn randomize(&mut self, seed: u32) {
        let mut rng = Rng::new(seed | 1);
        const PITCHES: [f32; 9] = [0.0, 0.0, 0.0, 0.0, 12.0, -12.0, 7.0, -5.0, 5.0];
        for c in &mut self.chops {
            c.pitch = PITCHES[rng.next_u32() as usize % PITCHES.len()];
            c.reverse = rng.unipolar() < 0.25;
            c.gain = (-4.0 * rng.unipolar()).round();
            c.pan = (rng.bipolar() * 0.4 * 10.0).round() / 10.0;
        }
    }

    /// Chopped again as its `chopping` says. Lines put by hand stay where
    /// they are (with their chops' sound) unless `all`; a new chop where
    /// one was keeps its sound.
    pub fn rechop(&mut self, s: &SampleData, all: bool) {
        let tempo = s.analysis().tempo.filter(|t| t.confidence > 0.2);
        let downbeat = tempo.map_or(0.0, |t| t.downbeat);
        let n = s.frames() as u64;
        let near = (0.01 * s.sample_rate as f64) as u64;
        let old = std::mem::take(&mut self.chops);
        let hand: Vec<FlipChop> = if all { Vec::new() } else { old.iter().copied().filter(|c| c.manual).collect() };
        let mut auto = if self.chopping.by == ChopBy::Manual { Vec::new() } else { chop_as(s, &self.chopping, self.bpm, downbeat) };
        if auto.is_empty() && hand.is_empty() && n > MIN_LEN {
            auto.push(FlipChop::new(0, n));
        }
        // A new chop where an old one was sounds as it did.
        for c in &mut auto {
            if let Some(o) = old.iter().filter(|o| o.has_sound()).find(|o| o.start.abs_diff(c.start) <= near) {
                *c = c.sound_of(o);
            }
        }
        if hand.is_empty() {
            self.chops = auto;
            return;
        }
        // The hand's lines and the new ones not too near them, each chop up
        // to the next line.
        let mut cuts: Vec<FlipChop> = hand.clone();
        cuts.extend(auto.iter().copied().filter(|c| hand.iter().all(|h| h.start.abs_diff(c.start) > near)));
        cuts.sort_by_key(|c| c.start);
        cuts.dedup_by_key(|c| c.start);
        while cuts.len() > MAX_CHOPS {
            match cuts.iter().rposition(|c| !c.manual) {
                Some(k) => cuts.remove(k),
                None => cuts.pop().unwrap_or_default(),
            };
        }
        let last_end = auto.last().map_or(n, |c| c.end).max(hand.iter().map(|c| c.end).max().unwrap_or(0)).min(n);
        for k in 0..cuts.len() {
            cuts[k].end = cuts.get(k + 1).map_or(last_end.max(cuts[k].start + MIN_LEN).min(n), |c| c.start);
        }
        cuts.retain(|c| c.end > c.start);
        self.chops = cuts;
    }
}

/// Where a line let go at `at` goes: onto a hit within `radius` frames (a
/// millisecond before it, as chopping puts them), then onto the nearest
/// place the wave crosses zero (within 2 ms, before a hit so it stays
/// whole), so the chop starts without a click.
pub fn snap(s: &SampleData, at: u64, radius: u64) -> u64 {
    let n = s.frames();
    if n < 2 {
        return at;
    }
    let sr = s.sample_rate as f64;
    let pre = (0.001 * sr) as u64;
    let hit = s.analysis().transients.iter().map(|&t| (t as u64).saturating_sub(pre)).filter(|t| t.abs_diff(at) <= radius).min_by_key(|t| t.abs_diff(at));
    let to = hit.unwrap_or(at).min(n as u64 - 1) as usize;
    let mono = |i: usize| s.left[i] + s.right[i];
    let crosses = |i: usize| i > 0 && i < n && (mono(i) == 0.0 || (mono(i - 1) < 0.0) != (mono(i) < 0.0));
    let w = (0.002 * sr) as usize;
    for d in 0..=w {
        if crosses(to.saturating_sub(d)) {
            return to.saturating_sub(d) as u64;
        }
        if hit.is_none() && crosses(to + d) {
            return (to + d) as u64;
        }
    }
    to as u64
}

/// Listens to a sample and chops it into `count` (8 to 16) chops. Takes a
/// moment for a whole song, so never on the audio thread.
pub fn listen(s: &SampleData, by: ChopBy, count: usize) -> Flip {
    listen_as(s, Chopping { by, count, ..Chopping::default() })
}

/// Listens to a sample and chops it as `chopping` says.
pub fn listen_as(s: &SampleData, chopping: Chopping) -> Flip {
    let key = crate::analysis::key_in_name(&s.name).or_else(|| crate::analysis::detect_key(s)).map(|k| (k.root, k.minor));
    let tempo = s.analysis().tempo.filter(|t| t.confidence > 0.2);
    let bpm = crate::analysis::bpm_in_name(&s.name).or(tempo.map(|t| t.bpm));
    let mut f = Flip { chops: Vec::new(), key, bpm, chopping };
    f.rechop(s, true);
    f
}

/// The chops of a sample (see [`listen`]): `downbeat` (s) is where its bars
/// start.
pub fn chop(s: &SampleData, by: ChopBy, count: usize, bpm: Option<f64>, downbeat: f64) -> Vec<FlipChop> {
    chop_as(s, &Chopping { by, count, ..Chopping::default() }, bpm, downbeat)
}

/// The chops of a sample as `how` says.
pub fn chop_as(s: &SampleData, how: &Chopping, bpm: Option<f64>, downbeat: f64) -> Vec<FlipChop> {
    let n = s.frames();
    let count = how.count.clamp(1, MAX_CHOPS);
    let sr = s.sample_rate as f64;
    if n < 64 {
        return Vec::new();
    }
    let by = how.by;
    let mut starts: Vec<usize> = match (by, bpm) {
        (ChopBy::Manual, _) => vec![0],
        (ChopBy::Hits, _) => {
            // The analysis's hits as they were found, or found again as
            // readily as the sensitivity says.
            let mut hits: Vec<(usize, f32)> = if (how.sensitivity - half()).abs() < 1e-4 {
                let an = s.analysis();
                an.transients.iter().copied().zip(an.strengths.iter().copied().chain(std::iter::repeat(1.0))).collect()
            } else {
                crate::analysis::spectral_flux(s).hits(s, how.sensitivity).iter().map(|h| (h.frame, h.strength)).collect()
            };
            if hits.len() > count {
                hits.sort_by(|a, b| b.1.total_cmp(&a.1));
                hits.truncate(count);
            }
            // A millisecond before each, so the hit is whole.
            let pre = (0.001 * sr) as usize;
            let mut v: Vec<usize> = hits.into_iter().map(|(h, _)| h.saturating_sub(pre)).collect();
            v.sort_unstable();
            // What comes before the first hit is a chop too.
            if v.first().is_some_and(|&f| f as f64 > 0.01 * sr) && v.len() < count {
                v.insert(0, 0);
            }
            if v.is_empty() {
                (0..count).map(|k| k * n / count).collect()
            } else {
                // Few hits (a pad, a held chord): the longest chops halved
                // till there are at least eight.
                while v.len() < MIN_CHOPS.min(count) {
                    let ends: Vec<usize> = v.iter().skip(1).copied().chain(std::iter::once(n)).collect();
                    let Some((k, len)) = v.iter().zip(&ends).map(|(a, b)| b - a).enumerate().max_by_key(|(_, len)| *len) else { break };
                    if len < (0.05 * sr) as usize {
                        break;
                    }
                    v.insert(k + 1, v[k] + len / 2);
                }
                v
            }
        }
        (ChopBy::Bars, Some(bpm)) if bpm > 0.0 => {
            let beat = 60.0 / bpm * sr;
            let first = ((downbeat.max(0.0) * sr) as usize).min(n - 1);
            let unit = match how.grid.beats() {
                Some(b) => b * beat,
                None => {
                    // Bars, or halves, beats, half beats of them till there are enough.
                    let mut unit = 4.0 * beat;
                    while ((n - first) as f64 / unit).floor() < count as f64 && unit > beat * 0.5 + 1.0 {
                        unit *= 0.5;
                    }
                    unit
                }
            };
            let fit = (((n - first) as f64 / unit).floor() as usize).clamp(1, count);
            (0..fit).map(|k| first + (k as f64 * unit) as usize).collect()
        }
        _ => (0..count).map(|k| k * n / count).collect(),
    };
    starts.dedup();
    let unit_end = |k: usize, starts: &[usize]| starts.get(k + 1).copied().unwrap_or(n);
    let bars = by == ChopBy::Bars;
    (0..starts.len())
        .map(|k| {
            let end = if bars && bpm.is_some() && k + 1 == starts.len() {
                // The last bar as long as the others.
                let len = if k > 0 { starts[k] - starts[k - 1] } else { n - starts[k] };
                (starts[k] + len).min(n)
            } else {
                unit_end(k, &starts)
            };
            FlipChop::new(starts[k] as u64, end as u64)
        })
        .filter(|c| c.end > c.start + 32)
        .collect()
}

/// How far a sample in `from` (root, minor) moves to play in `to`: the
/// difference of their roots (of the relative keys when one is minor and
/// the other not), the nearer way round (-6 to +5).
pub fn key_shift(from: (u8, bool), to: Scale) -> i32 {
    let minor = matches!(to.kind, ScaleKind::Minor | ScaleKind::Dorian | ScaleKind::Phrygian | ScaleKind::Locrian | ScaleKind::HarmonicMinor | ScaleKind::MelodicMinor | ScaleKind::MinorPentatonic | ScaleKind::HungarianMinor | ScaleKind::Blues | ScaleKind::Hirajoshi | ScaleKind::InSen);
    // Both as their major (a minor key's relative major is three up).
    let a = from.0 as i32 + if from.1 { 3 } else { 0 };
    let b = to.root as i32 + if minor { 3 } else { 0 };
    let d = (b - a).rem_euclid(12);
    if d > 5 {
        d - 12
    } else {
        d
    }
}

// ------------------------------------------------------------ auto-flip

/// The feel of an auto-flip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutoStyle {
    /// The chops one after the other, each about as long as it is.
    #[default]
    Straight,
    /// On sixteenths, some chops played two to four times over.
    Stutter,
    /// The pluggnb bounce: three, three and two sixteenths, twice a bar.
    Bounce,
}

impl AutoStyle {
    pub const ALL: [AutoStyle; 3] = [AutoStyle::Straight, AutoStyle::Stutter, AutoStyle::Bounce];

    pub fn name(self) -> &'static str {
        match self {
            AutoStyle::Straight => "Straight",
            AutoStyle::Stutter => "Stutter",
            AutoStyle::Bounce => "Bounce",
        }
    }

    /// The grid its notes start on (beats).
    pub fn grid(self) -> f64 {
        match self {
            AutoStyle::Straight => 0.5,
            AutoStyle::Stutter | AutoStyle::Bounce => 0.25,
        }
    }
}

/// Where the bounce's notes start in a bar (beats): 3 + 3 + 2 sixteenths, twice.
pub const BOUNCE: [f64; 6] = [0.0, 0.75, 1.5, 2.0, 2.75, 3.5];

/// A pattern of `bars` bars (of `beats_per_bar`) that plays the chops in a
/// new order, in time with a song at `bpm`: notes on the chops' keys, on
/// the style's grid, none over the next. `seed` picks the order.
pub fn auto_flip(flip: &Flip, sample_rate: u32, bpm: f64, beats_per_bar: f64, style: AutoStyle, bars: usize, seed: u32) -> Vec<Note> {
    let n = flip.chops.len().min(MAX_CHOPS);
    if n == 0 {
        return Vec::new();
    }
    let mut rng = Rng::new(seed.wrapping_mul(2_654_435_761) | 1);
    let total = bars.max(1) as f64 * beats_per_bar.max(1.0);
    let grid = style.grid();
    // How long a chop is, in beats of the song.
    let beats = |k: usize| {
        let c = flip.chops[k];
        (c.end.saturating_sub(c.start)) as f64 / sample_rate.max(1) as f64 * bpm.max(1.0) / 60.0
    };
    let on_grid = |b: f64, lo: f64, hi: f64| ((b / grid).round() * grid).clamp(lo, hi);
    // The chops in a new order, again and again.
    let mut order: Vec<usize> = Vec::new();
    let mut next = |rng: &mut Rng| {
        if order.is_empty() {
            order = (0..n).collect();
            for i in (1..n).rev() {
                order.swap(i, rng.next_u32() as usize % (i + 1));
            }
        }
        order.pop().unwrap_or(0)
    };
    let note = |k: usize, start: f64, len: f64, vel: f32| Note::new(FIRST_NOTE + k as u8, start, len, vel);
    let mut out = Vec::new();
    match style {
        AutoStyle::Straight => {
            let mut t = 0.0;
            while t < total - 1e-9 {
                let k = next(&mut rng);
                let len = on_grid(beats(k), 0.5, 2.0).min(total - t);
                out.push(note(k, t, len, if (t % beats_per_bar).abs() < 1e-9 { 1.0 } else { 0.85 }));
                t += len;
            }
        }
        AutoStyle::Stutter => {
            let mut t = 0.0;
            while t < total - 1e-9 {
                let k = next(&mut rng);
                if rng.unipolar() < 0.35 {
                    // Two to four sixteenths of the same chop.
                    let times = 2 + rng.next_u32() as usize % 3;
                    for r in 0..times {
                        if t >= total - 1e-9 {
                            break;
                        }
                        out.push(note(k, t, grid, if r == 0 { 1.0 } else { 0.7 }));
                        t += grid;
                    }
                } else {
                    let len = on_grid(beats(k), grid, 1.0).min(total - t);
                    out.push(note(k, t, len, 0.9));
                    t += len;
                }
            }
        }
        AutoStyle::Bounce => {
            // The bar's one on the same chop each bar, the rest about it.
            let anchor = next(&mut rng);
            for bar in 0..bars.max(1) {
                let at = bar as f64 * beats_per_bar.max(1.0);
                let ons: Vec<f64> = BOUNCE.iter().map(|b| b * beats_per_bar.max(1.0) / 4.0).collect();
                for (i, &o) in ons.iter().enumerate() {
                    let end = ons.get(i + 1).copied().unwrap_or(beats_per_bar.max(1.0));
                    let k = if i == 0 { anchor } else { next(&mut rng) };
                    let vel = if i == 0 || i == 3 { 1.0 } else { 0.75 };
                    out.push(note(k, at + o, end - o, vel));
                }
            }
        }
    }
    out
}

// ------------------------------------------------------------ export

/// The chops as the pads of a Drum Rack (from C1 up, as a sliced loop's
/// are, at most sixteen), each its part of the sample with its tune (and
/// the key's move), level, pan, choke group and gate. A chop turned round
/// plays forwards there (a pad keeps the file and its part, not the sound
/// turned round).
pub fn drum_pads(flip: &Flip, sample: &SampleRef, shift: i32, gate: bool) -> Vec<crate::project::DrumPad> {
    use crate::dsp::drums::{DrumVoice, MAX_PADS, PAD_GAIN};
    flip.chops
        .iter()
        .take(MAX_PADS)
        .enumerate()
        .map(|(k, c)| {
            let mut pad = crate::project::DrumPad::new(DrumVoice::Kick);
            pad.name = format!("Chop {}", k + 1);
            pad.note = crate::slicing::FIRST_NOTE + k as u8;
            pad.level_db = c.gain;
            pad.trim_db = -crate::dsp::gain_to_db(PAD_GAIN);
            pad.tune = c.pitch + c.fine / 100.0 + shift as f32;
            pad.pan = c.pan;
            pad.choke = c.choke.min(4);
            pad.gate = match c.play {
                Play::Flip => gate,
                Play::OneShot => false,
                _ => true,
            };
            pad.sample = Some(chop_sample(sample, c, k));
            pad
        })
        .collect()
}

/// Chop `k`'s part of the sample as a sample of its own (with its fades).
pub fn chop_sample(sample: &SampleRef, c: &FlipChop, k: usize) -> SampleRef {
    let sr = sample.sample_rate as f64;
    let n = sample.frames() as i64;
    let a = (c.start as i64 + (c.trim_in as f64 * 0.001 * sr) as i64).clamp(0, n) as u64;
    let b = (c.end as i64 + (c.trim_out as f64 * 0.001 * sr) as i64).clamp(a as i64, n) as u64;
    let fade = |ms: f32, least: f64| ((ms as f64 * 0.001).max(least) * sr).round() as u32;
    let region = crate::sample::SampleRegion { start: a, end: b, fade_in: fade(c.fade_in, 0.001), fade_out: fade(c.fade_out, 0.004) };
    SampleRef::new(sample.region(format!("{} {}", sample.name, k + 1), region))
}

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Init",
        "Plugg Flip",
        "Slowed Soul",
        "Half-Time Dream",
        "Deep Fried Flip",
        "Tape Warble",
        "Stutter Chops",
        "Anime Lo-Fi",
        "Gated Chops",
        "Night Drive",
        "Legato Melody",
        "Vinyl Flip",
        "Crushed Chops",
        "Wavy Pads",
        "Cloud Bounce",
        "Glide Lead",
        "Wide Dream",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Plugg Flip" => &[(PITCH, -2.0), (REVERB, 0.22), (LOW_PASS, 9000.0), (FRY, 0.12)],
        "Slowed Soul" => &[(SLOWED, 1.0), (WOBBLE, 0.25), (LOW_PASS, 7000.0), (HISS, 0.25)],
        "Half-Time Dream" => &[(HALF, 1.0), (REVERB, 0.4), (RELEASE, 400.0), (ATTACK, 20.0), (LOW_PASS, 6000.0)],
        "Deep Fried Flip" => &[(FRY, 0.8), (PITCH, -1.0), (REVERB, 0.05)],
        "Tape Warble" => &[(WOBBLE, 0.7), (HISS, 0.4), (LOW_PASS, 5000.0), (PITCH, -1.0)],
        "Stutter Chops" => &[(STUTTER, 2.0), (MODE, 1.0), (FRY, 0.2)],
        "Anime Lo-Fi" => &[(PITCH, -3.0), (LOW_PASS, 3500.0), (HISS, 0.5), (WOBBLE, 0.35), (REVERB, 0.25)],
        "Gated Chops" => &[(MODE, 1.0), (RELEASE, 15.0)],
        // A late drive: slowed, dark, the filter opening with each chop, slow and wide.
        "Night Drive" => &[(SLOWED, 1.0), (LOW_PASS, 1800.0), (RESONANCE, 0.3), (FILTER_ENV, 0.5), (DECAY, 900.0), (SUSTAIN, 0.4), (VEL_FILTER, 0.4), (WIDTH, 1.4), (REVERB, 0.3)],
        // One chop on the keys below as a melody, sliding from note to note.
        "Legato Melody" => &[(VOICING, 2.0), (GLIDE, 120.0), (MODE, 1.0), (RELEASE, 200.0), (ATTACK, 8.0), (REVERB, 0.25), (LOW_PASS, 8000.0)],
        "Vinyl Flip" => &[(WOBBLE, 0.4), (WOBBLE_TYPE, 1.0), (HISS, 0.45), (LOW_PASS, 8000.0), (PITCH, -1.0), (REVERB, 0.15)],
        "Crushed Chops" => &[(BITS, 8.0), (CRUSH_RATE, 12_000.0), (FRY, 0.15), (MODE, 1.0), (RELEASE, 20.0)],
        // Slow in, held, the filter swaying on the bar.
        "Wavy Pads" => &[(ATTACK, 120.0), (RELEASE, 1200.0), (MODE, 1.0), (HALF, 1.0), (LOW_PASS, 4000.0), (LFO_RATE, 2.0), (LFO_FILTER, 0.5), (LFO_PAN, 0.3), (REVERB, 0.45), (WIDTH, 1.5)],
        "Cloud Bounce" => &[(PITCH, -2.0), (MODE, 1.0), (RELEASE, 60.0), (DECAY, 250.0), (SUSTAIN, 0.6), (REVERB, 0.35), (LFO_RATE, 5.0), (LFO_PAN, 0.4), (WIDTH, 1.3)],
        "Glide Lead" => &[(VOICING, 1.0), (GLIDE, 70.0), (MODE, 1.0), (LFO_RATE, 6.0), (LFO_PITCH, 0.3), (VEL_PITCH, 1.0), (RELEASE, 120.0), (FRY, 0.1)],
        "Wide Dream" => &[(WIDTH, 1.8), (REVERB, 0.5), (WOBBLE, 0.2), (LOW_PASS, 10_000.0), (HISS, 0.15), (ATTACK, 30.0)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

/// A chop as it sounds.
#[derive(Clone, Copy, Debug)]
struct Voice {
    active: bool,
    note: u8,
    chop: usize,
    /// Its part of the sample (frames) and whether backwards.
    a: f64,
    b: f64,
    reverse: bool,
    /// Which way it reads now (a ping-pong turns it round).
    dir: f64,
    /// Where it is (frames), its speed (the key's, the note's, its own).
    pos: f64,
    ratio: f64,
    /// A glide: so many samples more of multiplying the speed by this.
    glide_mul: f64,
    glide_left: u32,
    glide_to: f64,
    env: f32,
    attack: f32,
    /// Down by this each sample to the sustain, once it is all the way in.
    decay: f32,
    sustain: f32,
    peaked: bool,
    release: f32,
    held: bool,
    /// Let go when its key is.
    gate: bool,
    play: Play,
    stretch: bool,
    gain: f32,
    pan: [f32; 2],
    vel: f32,
    group: u8,
    /// Its fades at its start and end (frames of the sample).
    fin: f64,
    fout: f64,
    /// Its own tone, and the flip's filter when it moves for each chop.
    tone: bool,
    tone_f: [Svf; 2],
    filt: [Svf; 2],
    /// Output samples since it (re)started, for the stutter.
    since: u64,
    /// The grains (half-time, stretched): how far the first is (0..1),
    /// where each began.
    gph: f32,
    gfrom: [f64; 2],
    last: [f32; 2],
    tail: [f32; 2],
    tail_left: u32,
    age: u64,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            active: false,
            note: 0,
            chop: 0,
            a: 0.0,
            b: 0.0,
            reverse: false,
            dir: 1.0,
            pos: 0.0,
            ratio: 1.0,
            glide_mul: 1.0,
            glide_left: 0,
            glide_to: 1.0,
            env: 0.0,
            attack: 1.0,
            decay: 0.0,
            sustain: 1.0,
            peaked: false,
            release: 0.0,
            held: false,
            gate: false,
            play: Play::OneShot,
            stretch: false,
            gain: 1.0,
            pan: [1.0; 2],
            vel: 1.0,
            group: 0,
            fin: 1.0,
            fout: 1.0,
            tone: false,
            tone_f: [Svf::new(); 2],
            filt: [Svf::new(); 2],
            since: 0,
            gph: 0.0,
            gfrom: [0.0; 2],
            last: [0.0; 2],
            tail: [0.0; 2],
            tail_left: 0,
            age: 0,
        }
    }
}

pub struct FlipState {
    sr: f32,
    pub bpm: f32,
    /// The song's key (Song Key moves the chops into it).
    pub key: Scale,
    /// The song's beat when its transport was last told, the samples since,
    /// and whether it plays (the LFO keeps its time).
    beat: f64,
    since_beat: u64,
    playing: bool,
    voices: [Voice; MAX_VOICES],
    pending: [Option<(u8, f32)>; 32],
    releases: [Option<u8>; 32],
    age: u64,
    rng: Rng,
    /// For what is new (the LFO's random steps), so the tape's own noise
    /// stays as it was.
    rng2: Rng,
    wow: f32,
    flutter: f32,
    drift: f32,
    drift_to: f32,
    lp: [Svf; 2],
    lp_on: bool,
    hiss_lp: OnePole,
    hiss_hp: OnePole,
    crackle: f32,
    /// How much it is playing (for the hiss: on with a chop, fading after).
    act: f32,
    bus: Box<Bus>,
    room: Box<Reverb>,
    buf: Box<[[f32; BLOCK]; 2]>,
    lit: [f32; MAX_CHOPS],
    /// Where the last chop played is in the sample (0..1).
    at: f32,
    shift: i32,
    /// The pitch (speed) of the last chop started, for a glide from it.
    last_ratio: Option<f64>,
    lfo_ph: f64,
    lfo_hold: f32,
    lfo_now: f32,
    /// The crusher: where it is between held samples, what it holds.
    crush_ph: f32,
    crush_hold: [f32; 2],
}

impl FlipState {
    pub fn new(sr: f32) -> Self {
        let mut hiss_lp = OnePole::default();
        hiss_lp.set(6000.0, sr);
        let mut hiss_hp = OnePole::default();
        hiss_hp.set(400.0, sr);
        Self {
            sr,
            bpm: 120.0,
            key: Scale::new(0, ScaleKind::Minor),
            beat: 0.0,
            since_beat: 0,
            playing: false,
            voices: [Voice::default(); MAX_VOICES],
            pending: [None; 32],
            releases: [None; 32],
            age: 0,
            rng: Rng::new(0xF11F_0042),
            rng2: Rng::new(0x51DE_F11F),
            wow: 0.0,
            flutter: 0.0,
            drift: 0.0,
            drift_to: 0.0,
            lp: [Svf::new(); 2],
            lp_on: false,
            hiss_lp,
            hiss_hp,
            crackle: 0.0,
            act: 0.0,
            bus: Box::new(Bus::new(sr)),
            room: Box::new(Reverb::new(sr)),
            buf: Box::new([[0.0; BLOCK]; 2]),
            lit: [0.0; MAX_CHOPS],
            at: -1.0,
            shift: 0,
            last_ratio: None,
            lfo_ph: 0.0,
            lfo_hold: 0.0,
            lfo_now: 0.0,
            crush_ph: 1.0,
            crush_hold: [0.0; 2],
        }
    }

    /// The song's tempo, and where it is (beats) and whether it plays.
    pub fn set_transport(&mut self, bpm: f32, beat: f64, playing: bool) {
        self.bpm = bpm.clamp(20.0, 999.0);
        self.beat = beat;
        self.since_beat = 0;
        self.playing = playing;
    }

    pub fn note_on(&mut self, note: u8, vel: f32) {
        if let Some(slot) = self.pending.iter_mut().find(|s| s.is_none()) {
            *slot = Some((note, vel));
        }
    }

    pub fn note_off(&mut self, note: u8) {
        if let Some(slot) = self.releases.iter_mut().find(|s| s.is_none()) {
            *slot = Some(note);
        }
    }

    pub fn all_off(&mut self) {
        self.pending = [None; 32];
        for v in self.voices.iter_mut().filter(|v| v.active) {
            v.held = false;
            v.release = v.release.max(1.0 / (0.01 * self.sr));
        }
    }

    /// For the face: how lit each chop is, where the last one played is in
    /// the sample (-1: none), the key's move, the voices playing, the LFO.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[..MAX_CHOPS].copy_from_slice(&self.lit);
        out[VIS_AT] = self.at;
        out[VIS_SHIFT] = self.shift as f32;
        out[VIS_VOICES] = self.voices.iter().filter(|v| v.active).count() as f32;
        out[VIS_LFO] = self.lfo_now;
    }

    /// The semitones the key moves everything by now.
    pub fn shift(&self, p: &[f32], flip: &Flip) -> i32 {
        match flip.key {
            Some(k) if p.get(SONG_KEY).is_some_and(|v| *v >= 0.5) && self.key.kind != ScaleKind::Chromatic => key_shift(k, self.key),
            _ => 0,
        }
    }

    fn start(&mut self, p: &[f32], flip: &Flip, sd: &SampleData, note: u8, vel: f32) {
        let n = sd.frames();
        let (chop, semis) = if note >= FIRST_NOTE && ((note - FIRST_NOTE) as usize) < flip.chops.len().min(MAX_CHOPS) {
            ((note - FIRST_NOTE) as usize, 0.0)
        } else if LOW_KEYS.contains(&note) && !flip.chops.is_empty() {
            let c = (p[CHOP].round().max(1.0) as usize - 1).min(flip.chops.len().min(MAX_CHOPS) - 1);
            (c, note as f32 - LOW_ROOT as f32)
        } else {
            return;
        };
        let c = flip.chops[chop];
        let srs = sd.sample_rate as f64;
        let trim = |ms: f32| (ms.clamp(-1000.0, 1000.0) as f64 * 0.001 * srs).round();
        let a = ((c.start as usize).min(n) as f64 + if c.trim_in != 0.0 { trim(c.trim_in) } else { 0.0 }).clamp(0.0, n as f64);
        let b = ((c.end as usize).min(n) as f64 + if c.trim_out != 0.0 { trim(c.trim_out) } else { 0.0 }).clamp(0.0, n as f64);
        if b - a < 4.0 {
            return;
        }
        let sr = self.sr;
        let vel = vel.clamp(0.0, 1.0);
        let voicing = p[VOICING].round() as usize;
        let vel_pitch = p[VEL_PITCH].clamp(0.0, 12.0);
        let own = c.pitch.clamp(-48.0, 48.0) + c.fine.clamp(-100.0, 100.0) * 0.01 + if vel_pitch > 0.0 { vel_pitch * (vel - 1.0) } else { 0.0 };
        let ratio = 2f64.powf((semis + self.shift as f32 + own) as f64 / 12.0);
        let play = match c.play {
            Play::Flip if p[MODE] >= 0.5 => Play::Gate,
            Play::Flip => Play::OneShot,
            other => other,
        };
        let glide = (p[GLIDE].clamp(0.0, 1000.0) * 0.001 * sr) as u32;
        let glide_from = |from: Option<f64>| match from {
            Some(f) if glide > 0 && voicing > 0 && f > 0.0 => (f, (ratio / f).powf(1.0 / glide as f64), glide),
            _ => (ratio, 1.0, 0),
        };
        // Legato: the chop sounding still slides to this one, its envelope
        // going on.
        if voicing == 2 {
            if let Some(v) = self.voices.iter_mut().filter(|v| v.active && v.held && v.release <= 0.0).max_by_key(|v| v.age) {
                let (from, mul, left) = glide_from(Some(v.ratio));
                v.ratio = from;
                v.glide_mul = mul;
                v.glide_left = left;
                v.glide_to = ratio;
                v.note = note;
                if v.chop != chop {
                    v.tail = v.last;
                    v.tail_left = (0.003 * sr) as u32;
                    v.chop = chop;
                    v.a = a;
                    v.b = b;
                    v.reverse = c.reverse;
                    v.dir = if c.reverse { -1.0 } else { 1.0 };
                    v.pos = if c.reverse { b - 1.0 } else { a };
                    v.since = 0;
                }
                self.last_ratio = Some(ratio);
                self.lit[chop.min(MAX_CHOPS - 1)] = 1.0;
                return;
            }
        }
        // Mono: what plays makes way; a group's chops cut each other, or
        // with Choke every chop the one before.
        let cut = |v: &Voice| voicing > 0 || if c.choke > 0 { v.group == c.choke } else { p[CHOKE] >= 0.5 };
        for v in self.voices.iter_mut().filter(|v| v.active) {
            if cut(v) {
                v.held = false;
                v.release = v.release.max(1.0 / (0.005 * sr));
            }
        }
        let limit = (p[VOICES].round() as usize).clamp(1, MAX_VOICES);
        let slot = self.voices[..limit].iter().position(|v| !v.active).unwrap_or_else(|| (0..limit).min_by_key(|&i| self.voices[i].age).unwrap_or(0));
        self.age += 1;
        let pos = if c.reverse { b - 1.0 } else { a };
        let attack = 1.0 / (p[ATTACK].clamp(0.5, 200.0) * 0.001 * sr).max(1.0);
        let sustain = p[SUSTAIN].clamp(0.0, 1.0);
        let decay = (1.0 - sustain) / (p[DECAY].clamp(5.0, 5000.0) * 0.001 * sr).max(1.0);
        let vel_level = p[VEL_LEVEL].clamp(0.0, 1.0);
        // At its default, to the last bit as it always was.
        let gain = if vel_level == SPECS[VEL_LEVEL].default { 0.35 + 0.65 * vel } else { (1.0 - vel_level) + vel_level * vel };
        let gain = if c.gain != 0.0 { gain * db_to_gain(c.gain.clamp(-60.0, 24.0)) } else { gain };
        let pan = c.pan.clamp(-1.0, 1.0);
        let tone = c.cutoff < 19_000.0;
        let mut tone_f = [Svf::new(); 2];
        if tone {
            for f in tone_f.iter_mut() {
                f.set(c.cutoff.max(20.0), 0.1, sr);
            }
        }
        let fade = |ms: f32, least: f64| if ms > 0.0 { (ms as f64 * 0.001 * srs).max(least * srs) } else { least * srs };
        let (from, glide_mul, glide_left) = glide_from(if voicing > 0 { self.last_ratio } else { None });
        self.last_ratio = Some(ratio);
        let old = self.voices[slot];
        self.voices[slot] = Voice {
            active: true,
            note,
            chop,
            a,
            b,
            reverse: c.reverse,
            dir: if c.reverse { -1.0 } else { 1.0 },
            pos,
            ratio: from,
            glide_mul,
            glide_left,
            glide_to: ratio,
            env: 0.0,
            attack,
            decay,
            sustain,
            peaked: false,
            release: 0.0,
            held: true,
            gate: play != Play::OneShot,
            play,
            stretch: c.stretch,
            gain,
            pan: [1.0 - pan.max(0.0), 1.0 + pan.min(0.0)],
            vel,
            group: c.choke,
            fin: fade(c.fade_in, 0.002),
            fout: fade(c.fade_out, 0.004),
            tone,
            tone_f,
            filt: [Svf::new(); 2],
            since: 0,
            gph: 0.0,
            gfrom: [pos, pos],
            last: [0.0; 2],
            // A voice taken while it sounded fades out under the new one.
            tail: if old.active { old.last } else { [0.0; 2] },
            tail_left: if old.active { (0.003 * sr) as u32 } else { 0 },
            age: self.age,
        };
        self.lit[chop.min(MAX_CHOPS - 1)] = 1.0;
    }

    pub fn render(&mut self, p: &[f32], sample: Option<&SampleRef>, flip: &Flip, l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let mut pos = 0;
        while pos < n {
            let len = (n - pos).min(BLOCK);
            self.render_block(p, sample, flip, &mut l[pos..pos + len], &mut r[pos..pos + len]);
            pos += len;
        }
    }

    /// The LFO now (-1..1), a sample further on.
    fn lfo_tick(&mut self, p: &[f32]) -> f32 {
        let cycle = LFO_BEATS[(p[LFO_RATE].round().max(0.0) as usize).min(LFO_BEATS.len() - 1)];
        let was = self.lfo_ph;
        self.lfo_ph = if self.playing {
            // On the song's beat.
            let beat = self.beat + self.since_beat as f64 * self.bpm as f64 / 60.0 / self.sr as f64;
            (beat / cycle).rem_euclid(1.0)
        } else {
            (self.lfo_ph + self.bpm as f64 / 60.0 / self.sr as f64 / cycle).fract()
        };
        self.since_beat += 1;
        let ph = self.lfo_ph as f32;
        let x = match p[LFO_SHAPE].round() as usize {
            0 => (ph * std::f32::consts::TAU).sin(),
            1 => 1.0 - 4.0 * (ph - 0.5).abs(),
            2 => {
                if ph < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            3 => 1.0 - 2.0 * ph,
            _ => {
                if self.lfo_ph < was {
                    self.lfo_hold = self.rng2.bipolar();
                }
                self.lfo_hold
            }
        };
        self.lfo_now = x;
        x
    }

    fn render_block(&mut self, p: &[f32], sample: Option<&SampleRef>, flip: &Flip, l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len()).min(BLOCK);
        let sr = self.sr;
        self.shift = self.shift(p, flip);
        let frames = sample.map_or(0, |s| s.frames());
        // Notes from the block's start.
        for i in 0..self.pending.len() {
            let Some((note, vel)) = self.pending[i].take() else { continue };
            if let Some(s) = sample.filter(|_| frames > 0) {
                self.start(p, flip, s, note, vel);
            }
        }
        let release = 1.0 / (p[RELEASE].clamp(5.0, 2000.0) * 0.001 * sr).max(1.0);
        for i in 0..self.releases.len() {
            let Some(note) = self.releases[i].take() else { continue };
            for v in self.voices.iter_mut().filter(|v| v.active && v.note == note) {
                v.held = false;
                if v.gate {
                    v.release = v.release.max(release);
                }
            }
        }
        // The LFO for the block, a value a sample.
        let lfo_pitch = p[LFO_PITCH].clamp(0.0, 12.0);
        let lfo_filter = p[LFO_FILTER].clamp(0.0, 1.0);
        let lfo_pan = p[LFO_PAN].clamp(0.0, 1.0);
        let mut lfo = [0.0f32; BLOCK];
        if lfo_pitch > 0.0 || lfo_filter > 0.0 || lfo_pan > 0.0 {
            for v in lfo[..n].iter_mut() {
                *v = self.lfo_tick(p);
            }
        } else {
            self.since_beat += n as u64;
            self.lfo_now = 0.0;
        }
        let [bl, br] = &mut *self.buf;
        bl[..n].fill(0.0);
        br[..n].fill(0.0);
        // The tape: how fast it runs now.
        let wobble = p[WOBBLE].clamp(0.0, 1.0);
        let wobble_type = p[WOBBLE_TYPE].round() as usize;
        let slowed = p[SLOWED] >= 0.5;
        let half = p[HALF] >= 0.5;
        let base = 2f64.powf(p[PITCH].clamp(-12.0, 12.0) as f64 / 12.0) * if slowed { SLOWED_SPEED } else { 1.0 };
        let tape_time = if slowed { SLOWED_SPEED } else { 1.0 };
        let stutter = STUTTER_BEATS[(p[STUTTER].round().max(0.0) as usize).min(STUTTER_BEATS.len() - 1)];
        let stutter_len = (stutter * 60.0 / self.bpm.max(1.0) as f64 * sr as f64) as u64;
        // The filter: once for all (as it always was), or for each chop when
        // its envelope or velocity moves it.
        let lp_hz = p[LOW_PASS].clamp(200.0, 20_000.0);
        let ftype = match p[FILTER_TYPE].round() as usize {
            1 => FilterMode::High,
            2 => FilterMode::Band,
            _ => FilterMode::Low,
        };
        let res = p[RESONANCE].clamp(0.0, 1.0);
        let env_amt = p[FILTER_ENV].clamp(-1.0, 1.0);
        let vel_f = p[VEL_FILTER].clamp(0.0, 1.0);
        let per_voice = env_amt != 0.0 || vel_f > 0.0;
        let lp_on = !per_voice && (ftype != FilterMode::Low || lp_hz < 19_000.0 || lfo_filter > 0.0);
        if lp_on {
            if !self.lp_on {
                self.lp = [Svf::new(); 2];
            }
            for f in self.lp.iter_mut() {
                f.set(lp_hz, res, sr);
            }
        }
        self.lp_on = lp_on;
        let hiss = p[HISS].clamp(0.0, 1.0);
        let act_fall = decay_coef(1.5, sr);
        let grain = (0.07 * sr).max(16.0);
        let mut last_at = None;
        if let Some(s) = sample.filter(|_| frames > 1) {
            let sd: &SampleData = s;
            let rate = sd.sample_rate as f64 / sr as f64;
            for i in 0..n {
                // Wow (a slow drift and a sway) and flutter, in cents.
                self.wow = (self.wow + 0.55 / sr).fract();
                self.flutter = (self.flutter + 6.3 / sr).fract();
                let often = if wobble_type == 2 { 1024 } else { 4096 };
                if self.rng.next_u32() % often == 0 {
                    self.drift_to = self.rng.bipolar();
                }
                self.drift += (self.drift_to - self.drift) * if wobble_type == 2 { 0.0008 } else { 0.0002 };
                let tau = std::f32::consts::TAU;
                let mut cents = match wobble_type {
                    // A warped record: once a turn, and its second.
                    1 => wobble * (24.0 * (self.wow * tau).sin() + 7.0 * (2.0 * self.wow * tau + 1.0).sin() + 2.0 * (self.flutter * tau).sin()),
                    // A tape worn out: it wanders.
                    2 => wobble * (10.0 * (self.wow * tau).sin() + 9.0 * (self.flutter * tau).sin() + 32.0 * self.drift),
                    _ => wobble * (16.0 * (self.wow * tau).sin() + 5.0 * (self.flutter * tau).sin() + 10.0 * self.drift),
                };
                if lfo_pitch > 0.0 {
                    cents += lfo[i] * lfo_pitch * 100.0;
                }
                let tape = 2f64.powf(cents as f64 / 1200.0);
                let speed = base * tape;
                let fcut = if lfo_filter > 0.0 { 2f32.powf(lfo[i] * lfo_filter * 3.0) } else { 1.0 };
                if lp_on && lfo_filter > 0.0 && i % 16 == 0 {
                    for f in self.lp.iter_mut() {
                        f.set((lp_hz * fcut).clamp(20.0, 20_000.0), res, sr);
                    }
                }
                let (mut sl, mut sr_) = (0.0f32, 0.0f32);
                for v in self.voices.iter_mut().filter(|v| v.active) {
                    if v.glide_left > 0 {
                        v.glide_left -= 1;
                        v.ratio = if v.glide_left == 0 { v.glide_to } else { v.ratio * v.glide_mul };
                    }
                    let step = v.ratio * speed * rate;
                    let rev = v.reverse;
                    let dir = v.dir;
                    let (a, b) = (v.a, v.b);
                    let grains = half || v.stretch;
                    // A stretched chop keeps its own time, at the tape's speed.
                    let adv = if v.stretch { tape_time * tape * rate * if half { 0.5 } else { 1.0 } } else { step * 0.5 };
                    if v.since == 0 {
                        // The grains from the chop's start: the second one, half
                        // way through its own, reads from there now.
                        v.gph = 0.0;
                        v.gfrom = [v.pos, v.pos - dir * 0.5 * grain as f64 * step];
                    }
                    // In at the chop's start, out at its end.
                    let (fin, fout) = (v.fin, v.fout);
                    let read = |at: f64| -> [f32; 2] {
                        if at < a || at > b - 1.0 {
                            return [0.0; 2];
                        }
                        let (from_start, to_end) = if rev { (b - 1.0 - at, at - a) } else { (at - a, b - 1.0 - at) };
                        let e = ((from_start / fin).min(to_end / fout)).min(1.0) as f32;
                        let (x, y) = sd.read(at);
                        [x * e, y * e]
                    };
                    let y = if grains {
                        // Grains read at the chop's pitch, from a place moving
                        // at half its speed (half-time) or at its own (stretched).
                        let before = v.gph;
                        v.gph = (v.gph + 1.0 / grain).fract();
                        let mut y = [0.0f32; 2];
                        for k in 0..2 {
                            let (age, was) = ((v.gph + 0.5 * k as f32).fract(), (before + 0.5 * k as f32).fract());
                            if age < was {
                                v.gfrom[k] = v.pos;
                            }
                            let at = v.gfrom[k] + dir * age as f64 * grain as f64 * step;
                            let w = (std::f32::consts::PI * age).sin();
                            let s = read(at);
                            y[0] += s[0] * w * w;
                            y[1] += s[1] * w * w;
                        }
                        v.pos += dir * adv;
                        y
                    } else {
                        let y = read(v.pos);
                        v.pos += dir * step;
                        y
                    };
                    // The envelope: in, down to its sustain, held, out.
                    if v.release > 0.0 {
                        v.env -= v.release;
                    } else if !v.peaked {
                        v.env = (v.env + v.attack).min(1.0);
                        v.peaked = v.env >= 1.0;
                    } else if v.env > v.sustain {
                        v.env = (v.env - v.decay).max(v.sustain);
                    }
                    let mut y = y;
                    if v.tone {
                        y = [v.tone_f[0].process(y[0], FilterMode::Low), v.tone_f[1].process(y[1], FilterMode::Low)];
                    }
                    if per_voice {
                        // The filter opened (or shut) by the envelope and the velocity.
                        if i % 16 == 0 {
                            let oct = env_amt * v.env.clamp(0.0, 1.0) * 6.0 + vel_f * (v.vel - 1.0) * 5.0;
                            let hz = (lp_hz * 2f32.powf(oct) * fcut).clamp(20.0, 20_000.0);
                            for f in v.filt.iter_mut() {
                                f.set(hz, res, sr);
                            }
                        }
                        y = [v.filt[0].process(y[0], ftype), v.filt[1].process(y[1], ftype)];
                    }
                    let g = v.env.max(0.0) * v.gain;
                    let mut out = [y[0] * g * v.pan[0], y[1] * g * v.pan[1]];
                    if v.tail_left > 0 {
                        let k = v.tail_left as f32 / (0.003 * sr);
                        out[0] += v.tail[0] * k;
                        out[1] += v.tail[1] * k;
                        v.tail_left -= 1;
                    }
                    v.last = out;
                    sl += out[0];
                    sr_ += out[1];
                    v.since += 1;
                    // A held chop again from its start, every so often.
                    if stutter_len > 0 && v.held && v.since >= stutter_len {
                        v.since = 0;
                        v.tail = v.last;
                        v.tail_left = (0.003 * sr) as u32;
                        v.dir = if v.reverse { -1.0 } else { 1.0 };
                        v.pos = if v.reverse { b - 1.0 } else { a };
                    }
                    let ended = match v.play {
                        // Round again from its start, or turned round at its ends.
                        Play::Loop => {
                            if v.dir > 0.0 && v.pos > b - 1.0 {
                                v.pos = a + (v.pos - (b - 1.0)).min(b - a - 1.0);
                            } else if v.dir < 0.0 && v.pos < a {
                                v.pos = b - 1.0 - (a - v.pos).min(b - a - 1.0);
                            }
                            false
                        }
                        Play::PingPong => {
                            if v.pos > b - 1.0 {
                                v.pos = (b - 1.0 - (v.pos - (b - 1.0))).max(a);
                                v.dir = -1.0;
                            } else if v.pos < a {
                                v.pos = (a + (a - v.pos)).min(b - 1.0);
                                v.dir = 1.0;
                            }
                            false
                        }
                        _ => {
                            if v.dir < 0.0 {
                                v.pos < a
                            } else {
                                v.pos > b - 1.0
                            }
                        }
                    };
                    if ended && v.tail_left == 0 || v.env <= 0.0 && v.release > 0.0 {
                        v.active = false;
                    }
                    last_at = Some((v.age, v.pos / frames as f64));
                }
                bl[i] = sl;
                br[i] = sr_;
            }
        }
        if let Some((_, at)) = last_at {
            self.at = at as f32;
        } else if self.voices.iter().all(|v| !v.active) {
            self.at = -1.0;
        }
        // The crusher: fewer bits, held samples.
        let bits = p[BITS].round().clamp(2.0, 16.0);
        let crush_hz = p[CRUSH_RATE].clamp(1000.0, 48_000.0);
        if bits < 16.0 || crush_hz < 47_900.0 {
            let steps = 2f32.powf(bits - 1.0);
            let inc = (crush_hz / sr).min(1.0);
            for i in 0..n {
                self.crush_ph += inc;
                if self.crush_ph >= 1.0 {
                    self.crush_ph -= 1.0;
                    self.crush_hold = [bl[i], br[i]];
                    if bits < 16.0 {
                        self.crush_hold = [(self.crush_hold[0] * steps).round() / steps, (self.crush_hold[1] * steps).round() / steps];
                    }
                }
                bl[i] = self.crush_hold[0];
                br[i] = self.crush_hold[1];
            }
        }
        // The tape's tone and its hiss, while it plays and a moment after.
        let playing = self.voices.iter().any(|v| v.active);
        for i in 0..n {
            if lp_on {
                bl[i] = self.lp[0].process(bl[i], ftype);
                br[i] = self.lp[1].process(br[i], ftype);
            }
            self.act = if playing { 1.0 } else { self.act * act_fall };
            if hiss > 0.0 && self.act > 1e-3 {
                let w = self.rng.bipolar();
                let h = self.hiss_hp.hp(self.hiss_lp.lp(w));
                if self.rng.next_u32() % 9000 == 0 {
                    self.crackle = self.rng.bipolar() * 0.6;
                }
                self.crackle *= 0.7;
                let x = (h * 0.05 + self.crackle * 0.25) * hiss * self.act;
                bl[i] += x;
                br[i] += x * 0.9 + self.crackle * 0.02;
            }
        }
        // The FRY section, turned up together.
        let fry = p[FRY].clamp(0.0, 1.0);
        if fry > 0.001 {
            let bits = (16.0 - fry * 9.0).clamp(2.0, 16.0);
            let down = 1.0 + fry * 4.0;
            let k = BusKnobs {
                drive: fry * 0.6,
                steps: 2f32.powf(bits - 1.0),
                rate: 1.0 / down,
                crush: true,
                ring: fry * 0.15,
                ring_inc: 180.0 / sr,
                push: 1.0 + 4.0 * fry,
                hard: fry > 0.5,
                ceiling: db_to_gain(-0.3),
                volume: 1.0,
                lim_rel: 1.0 - (-1.0 / (0.08 * sr)).exp(),
                ..BusKnobs::default()
            };
            for i in 0..n {
                let y = self.bus.fry([bl[i], br[i]], &k);
                let y = self.bus.tick(y, &k);
                bl[i] = y[0];
                br[i] = y[1];
            }
        }
        // The room: more of it slowed.
        let verb = (p[REVERB].clamp(0.0, 1.0) + if slowed { 0.3 } else { 0.0 }).min(1.0);
        if verb > 0.001 {
            let size = if slowed { 0.92 } else { 0.7 };
            self.room.process(size, 0.55, 1.0, 12.0, 150.0, verb * 0.6, &mut bl[..n], &mut br[..n]);
        }
        // The LFO's pan, then how wide it is.
        if lfo_pan > 0.0 {
            for i in 0..n {
                let pan = lfo[i] * lfo_pan;
                bl[i] *= 1.0 - pan.max(0.0);
                br[i] *= 1.0 + pan.min(0.0);
            }
        }
        let width = p[WIDTH].clamp(0.0, 2.0);
        if (width - 1.0).abs() > 1e-4 {
            for i in 0..n {
                let (m, s) = (0.5 * (bl[i] + br[i]), 0.5 * (bl[i] - br[i]) * width);
                bl[i] = m + s;
                br[i] = m - s;
            }
        }
        let vol = db_to_gain(p[VOLUME].clamp(-30.0, 6.0));
        for i in 0..n {
            let (a, b) = (bl[i] * vol, br[i] * vol);
            if a.is_finite() && b.is_finite() {
                l[i] += a;
                r[i] += b;
            }
        }
        let fade = (-(n as f32) / (0.15 * sr)).exp();
        self.lit.iter_mut().for_each(|x| *x *= fade);
        // A chop still sounding stays lit.
        for v in self.voices.iter().filter(|v| v.active) {
            let x = &mut self.lit[v.chop.min(MAX_CHOPS - 1)];
            *x = x.max(0.6 * v.env.clamp(0.0, 1.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    const SR: f32 = 48_000.0;

    fn sample(name: &str, v: Vec<f32>) -> SampleRef {
        SampleRef::new(SampleData::from_buffers(name, None, 48_000, v.clone(), v))
    }

    fn sine(hz: f32, secs: f32) -> Vec<f32> {
        (0..(secs * SR) as usize).map(|i| 0.5 * (TAU * hz * i as f32 / SR).sin()).collect()
    }

    /// The pitch of `x` by its upward zero crossings (Hz).
    fn pitch(x: &[f32]) -> f32 {
        let ups: Vec<usize> = x.windows(2).enumerate().filter(|(_, w)| w[0] <= 0.0 && w[1] > 0.0).map(|(i, _)| i).collect();
        if ups.len() < 3 {
            return 0.0;
        }
        (ups.len() - 1) as f32 * SR / (ups[ups.len() - 1] - ups[0]) as f32
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn whole(s: &SampleRef) -> Flip {
        Flip { chops: vec![FlipChop::new(0, s.frames() as u64)], ..Flip::default() }
    }

    /// Both sides of `note` held for `hold` s of `secs`.
    fn play2(st: &mut FlipState, p: &[f32], s: &SampleRef, f: &Flip, note: u8, secs: f32, hold: f32) -> (Vec<f32>, Vec<f32>) {
        st.note_on(note, 1.0);
        let (mut ol, mut or) = (Vec::new(), Vec::new());
        let blocks = (secs * SR / 128.0) as usize;
        for b in 0..blocks {
            if b == (hold * SR / 128.0) as usize {
                st.note_off(note);
            }
            let (mut l, mut r) = ([0.0f32; 128], [0.0f32; 128]);
            st.render(p, Some(s), f, &mut l, &mut r);
            ol.extend_from_slice(&l);
            or.extend_from_slice(&r);
        }
        (ol, or)
    }

    fn play(st: &mut FlipState, p: &[f32], s: &SampleRef, f: &Flip, note: u8, secs: f32, hold: f32) -> Vec<f32> {
        play2(st, p, s, f, note, secs, hold).0
    }

    /// Notes at blocks (block, note, velocity; 0 lets go), `blocks` long.
    fn perform(st: &mut FlipState, p: &[f32], s: &SampleRef, f: &Flip, notes: &[(usize, u8, f32)], blocks: usize) -> Vec<f32> {
        let mut out = Vec::new();
        for b in 0..blocks {
            for &(at, note, vel) in notes.iter().filter(|n| n.0 == b) {
                if vel > 0.0 {
                    st.note_on(note, vel);
                } else {
                    st.note_off(note);
                }
                let _ = at;
            }
            let (mut l, mut r) = ([0.0f32; 128], [0.0f32; 128]);
            st.render(p, Some(s), f, &mut l, &mut r);
            out.extend_from_slice(&l);
        }
        out
    }

    fn clean() -> Vec<f32> {
        let mut p = preset("Init");
        p[REVERB] = 0.0;
        p[VOLUME] = 0.0;
        p
    }

    /// A loop of eight hits (short decaying tones), each `gap` apart.
    fn hits(gap: usize) -> Vec<f32> {
        let mut v = vec![0.0f32; gap * 8];
        for k in 0..8 {
            for i in 0..gap.min(6000) {
                let t = i as f32 / SR;
                v[k * gap + i] = 0.8 * (TAU * 200.0 * t).sin() * (-t / 0.03).exp();
            }
        }
        v
    }

    #[test]
    fn chops_go_at_the_hits_or_on_the_bars() {
        let gap = 12_000;
        let s = sample("break", hits(gap));
        let f = listen(&s, ChopBy::Hits, 8);
        assert_eq!(f.chops.len(), 8, "{:?}", f.chops);
        for (k, c) in f.chops.iter().enumerate() {
            assert!((c.start as i64 - (k * gap) as i64).abs() < 300, "chop {k} at {}", c.start);
            assert!(c.end > c.start);
        }
        // In bars: four bars at 120 (named), cut in half bars for eight chops.
        let s = sample("soul loop 120 BPM", sine(220.0, 8.0));
        let f = listen(&s, ChopBy::Bars, 8);
        assert_eq!(f.bpm, Some(120.0));
        assert_eq!(f.chops.len(), 8);
        for (k, c) in f.chops.iter().enumerate() {
            assert_eq!(c.start, k as u64 * 48_000, "chop {k}");
            assert_eq!(c.end - c.start, 48_000);
        }
        // Sixteen bars and sixteen chops: a bar each.
        let s = sample("long 120 BPM", sine(220.0, 32.5));
        let f = listen(&s, ChopBy::Bars, 16);
        assert_eq!(f.chops.len(), 16);
        assert_eq!(f.chops[1].start, 96_000);
        // Few hits (two in a long held tone): halved till there are eight.
        let mut two = sine(220.0, 4.0);
        two[96_000..].iter_mut().for_each(|v| *v *= 2.0);
        let c = chop(&sample("pad", two), ChopBy::Hits, 12, None, 0.0);
        assert!(c.len() >= 8, "{} chops", c.len());
        assert!(c.windows(2).all(|w| w[0].end == w[1].start && w[0].start < w[1].start));
        // No tempo: equal parts.
        let c = chop(&sample("x", sine(220.0, 1.0)), ChopBy::Bars, 10, None, 0.0);
        assert_eq!(c.len(), 10);
        assert_eq!(c[5].start, 24_000);
    }

    #[test]
    fn the_other_ways_of_chopping() {
        // On a grid of beats: a beat at 120 is half a second.
        let s = sample("loop 120 BPM", sine(220.0, 8.0));
        let f = listen_as(&s, Chopping { by: ChopBy::Bars, count: 32, grid: Grid::Beat, ..Chopping::default() });
        assert_eq!(f.chops.len(), 16);
        assert!(f.chops.iter().enumerate().all(|(k, c)| c.start == k as u64 * 24_000 && c.end - c.start == 24_000));
        let f = listen_as(&s, Chopping { by: ChopBy::Bars, count: 32, grid: Grid::Sixteenth, ..Chopping::default() });
        assert_eq!(f.chops.len(), 32, "as many as there may be");
        assert_eq!(f.chops[1].start, 6000);
        // Equal parts, any number up to 32.
        let f = listen_as(&s, Chopping { by: ChopBy::Equal, count: 20, ..Chopping::default() });
        assert_eq!(f.chops.len(), 20);
        assert_eq!(f.chops[10].start, 192_000);
        // By hand: one chop till lines are put.
        let f = listen_as(&s, Chopping { by: ChopBy::Manual, ..Chopping::default() });
        assert_eq!(f.chops, vec![FlipChop::new(0, s.frames() as u64)]);
        // Hits soft and loud: a lower sensitivity finds no more of them.
        let mut v = hits(12_000);
        for k in [1, 3, 5, 7] {
            v[k * 12_000..k * 12_000 + 6000].iter_mut().for_each(|x| *x *= 0.04);
        }
        let s = sample("soft and loud", v);
        let count = |sens: f32| chop_as(&s, &Chopping { by: ChopBy::Hits, count: 32, sensitivity: sens, ..Chopping::default() }, None, 0.0).len();
        assert!(count(0.0) <= count(1.0), "{} at 0 %, {} at 100 %", count(0.0), count(1.0));
        assert!(count(1.0) >= 8);
    }

    #[test]
    fn lines_move_but_never_past_their_neighbours() {
        let mut f = Flip { chops: vec![FlipChop::new(0, 1000), FlipChop::new(1000, 2000), FlipChop::new(2000, 3000)], ..Flip::default() };
        f.chops[1].pitch = 5.0;
        f.chops[1].reverse = true;
        assert_eq!(f.move_line(1, 1500, 3000), 1500);
        assert_eq!((f.chops[0].end, f.chops[1].start, f.chops[1].end), (1500, 1500, 2000), "the chop before ends at the line");
        assert!(f.chops[1].manual);
        assert_eq!((f.chops[1].pitch, f.chops[1].reverse), (5.0, true), "a chop moved keeps its sound");
        // Not past the next line, nor the one before.
        assert_eq!(f.move_line(1, 9000, 3000), 2000 - MIN_LEN);
        assert_eq!(f.move_line(1, 0, 3000), MIN_LEN);
        assert_eq!(f.chops[0].end, MIN_LEN);
        // The first line only to the start, the last chop's end stays.
        assert_eq!(f.move_line(0, 40, 3000), 0);
        assert_eq!(f.move_line(2, 2990, 3000), 3000 - MIN_LEN);
        assert_eq!(f.chops[2].end, 3000);
        assert_eq!(f.chops[1].end, 3000 - MIN_LEN);
        // Shuffled on the keys, they still keep to their places in the sample.
        let mut g = Flip { chops: vec![FlipChop::new(2000, 3000), FlipChop::new(0, 1000), FlipChop::new(1000, 2000)], ..Flip::default() };
        assert_eq!(g.move_line(0, 500, 3000), 1000 + MIN_LEN);
        assert_eq!(g.chops[2].end, 1000 + MIN_LEN);
        assert_eq!(g.lines().iter().map(|l| l.0).collect::<Vec<_>>(), vec![1, 2, 0]);
        // Saved with the set as they are.
        let back: Flip = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
        assert_eq!(back, f);
    }

    #[test]
    fn lines_are_added_and_taken_away() {
        let mut f = Flip { chops: vec![FlipChop::new(0, 1000), FlipChop::new(1000, 2000)], ..Flip::default() };
        f.chops[0].gain = -6.0;
        assert_eq!(f.add_line(400, 3000), Some(1));
        assert_eq!(f.chops.len(), 3);
        assert_eq!((f.chops[0].end, f.chops[1].start, f.chops[1].end), (400, 400, 1000));
        assert_eq!(f.chops[1].gain, -6.0, "cut from the first, it sounds as it did");
        assert!(f.chops[1].manual && !f.chops[0].manual);
        // Not too near a line, not past the end.
        assert_eq!(f.add_line(1010, 3000), None);
        assert_eq!(f.add_line(5000, 3000), None);
        // In the gap after the last chop: a chop of its own to the end.
        assert_eq!(f.add_line(2500, 3000), Some(3));
        assert_eq!((f.chops[3].start, f.chops[3].end), (2500, 3000));
        // Taken away: the chop before runs on over it.
        assert!(f.delete_line(1));
        assert_eq!(f.chops.len(), 3);
        assert_eq!((f.chops[0].start, f.chops[0].end), (0, 1000));
        assert!(f.delete_line(0));
        assert_eq!(f.chops[0].start, 1000, "the first just goes");
        assert!(!f.delete_line(9));
        // No more than 32.
        let mut f = Flip { chops: vec![FlipChop::new(0, 100_000)], ..Flip::default() };
        for k in 1..40 {
            f.add_line(k * 1000, 100_000);
        }
        assert_eq!(f.chops.len(), MAX_CHOPS);
    }

    #[test]
    fn a_line_snaps_to_a_hit_or_where_the_wave_crosses() {
        let gap = 12_000;
        let s = sample("break", hits(gap));
        let to = snap(&s, 3 * gap as u64 + 150, 480);
        assert!(to <= 3 * gap as u64 && to + 100 > 3 * gap as u64, "onto the hit: {to}");
        // Far from any hit: where it was, or the nearest zero crossing.
        let s = sample("tone", sine(100.0, 0.5));
        let to = snap(&s, 18_000 + 230, 0);
        assert!(to.abs_diff(18_240) <= 1, "{to}");
        let to = snap(&s, 18_000 + 120, 0);
        assert_eq!(to, 18_120, "no crossing within 2 ms: where it was");
    }

    #[test]
    fn chopping_again_keeps_what_the_hand_did() {
        let gap = 12_000;
        let s = sample("break", hits(gap));
        let mut f = listen(&s, ChopBy::Hits, 8);
        // A line by hand between two hits, with a sound of its own.
        let k = f.add_line(2 * gap as u64 + 5000, s.frames()).unwrap();
        f.chops[k].pitch = 7.0;
        // A hit's chop with a sound of its own too.
        f.chops[0].reverse = true;
        f.chopping = Chopping { by: ChopBy::Equal, count: 4, ..Chopping::default() };
        f.rechop(&s, false);
        let hand = f.chops.iter().find(|c| c.manual).expect("the hand's line stays");
        assert_eq!((hand.start, hand.pitch), (2 * gap as u64 + 5000, 7.0));
        assert_eq!(f.chops.len(), 5);
        assert!(f.chops[0].reverse, "a chop where one was keeps its sound");
        assert!(f.chops.windows(2).all(|w| w[0].end == w[1].start && w[0].start < w[1].start));
        // All of it again.
        f.rechop(&s, true);
        assert_eq!(f.chops.len(), 4);
        assert!(f.chops.iter().all(|c| !c.manual));
        // Shuffled and sorted again.
        let before = f.chops.clone();
        f.shuffle(7);
        assert_ne!(f.chops, before);
        f.sort();
        assert_eq!(f.chops, before);
        f.randomize(3);
        assert!(f.chops.iter().any(|c| c.has_sound()));
        assert!(f.chops.iter().zip(&before).all(|(a, b)| a.start == b.start && a.end == b.end));
    }

    #[test]
    fn pitch_half_time_and_reverse() {
        let s = sample("tone", sine(440.0, 0.5));
        let f = whole(&s);
        let mut p = clean();
        let mut st = FlipState::new(SR);
        let y = play(&mut st, &p, &s, &f, FIRST_NOTE, 0.4, 1.0);
        assert!((pitch(&y[2000..18_000]) - 440.0).abs() < 3.0, "{}", pitch(&y[2000..18_000]));
        // An octave up by its Pitch: twice as high, over in half the time.
        p[PITCH] = 12.0;
        let mut st = FlipState::new(SR);
        let y = play(&mut st, &p, &s, &f, FIRST_NOTE, 0.5, 1.0);
        assert!((pitch(&y[2000..10_000]) - 880.0).abs() < 6.0, "{}", pitch(&y[2000..10_000]));
        assert!(rms(&y[13_000..20_000]) < 1e-3, "over at 0.25 s");
        // Half-time: its own pitch, twice as long.
        p[PITCH] = 0.0;
        p[HALF] = 1.0;
        let mut st = FlipState::new(SR);
        let y = play(&mut st, &p, &s, &f, FIRST_NOTE, 1.2, 2.0);
        let w = &y[4000..40_000];
        assert!(heard(w, 440.0) > 4.0 * heard(w, 220.0) && heard(w, 440.0) > 0.1, "its own pitch: {} at 440, {} at 220", heard(w, 440.0), heard(w, 220.0));
        assert!(rms(&y[36_000..44_000]) > 0.05, "still playing at 0.8 s: {}", rms(&y[36_000..44_000]));
        assert!(rms(&y[50_000..]) < 0.01, "over by 1.05 s");
        // Reversed: a rising ramp plays falling.
        let ramp: Vec<f32> = (0..24_000).map(|i| i as f32 / 24_000.0).collect();
        let s = sample("ramp", ramp);
        let mut f = whole(&s);
        f.chops[0].reverse = true;
        let mut p = clean();
        p[CHOKE] = 0.0;
        let mut st = FlipState::new(SR);
        let y = play(&mut st, &p, &s, &f, FIRST_NOTE, 0.4, 1.0);
        assert!(y[1000] > y[10_000] && y[10_000] > y[18_000], "{} {} {}", y[1000], y[10_000], y[18_000]);
    }

    /// How much of `hz` there is in `x` (Goertzel).
    fn level(x: &[f32], hz: f32) -> f32 {
        let c = 2.0 * (TAU * hz / SR).cos();
        let (mut s1, mut s2) = (0.0f32, 0.0f32);
        for &v in x {
            let s = v + c * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        (s1 * s1 + s2 * s2 - c * s1 * s2).max(0.0).sqrt() * 2.0 / x.len() as f32
    }

    /// The same in 50 ms windows (grains' phases jump from one to the next).
    fn heard(x: &[f32], hz: f32) -> f32 {
        let w: Vec<f32> = x.chunks_exact(2400).map(|c| level(c, hz)).collect();
        w.iter().sum::<f32>() / w.len().max(1) as f32
    }

    #[test]
    fn each_chop_plays_with_its_own_sound() {
        let s = sample("tone", sine(440.0, 0.5));
        let p = clean();
        let with = |set: &dyn Fn(&mut FlipChop)| {
            let mut f = whole(&s);
            set(&mut f.chops[0]);
            f
        };
        let run = |f: &Flip, secs: f32, hold: f32| play2(&mut FlipState::new(SR), &p, &s, f, FIRST_NOTE, secs, hold);
        // Its pitch and cents.
        let (y, _) = run(&with(&|c| c.pitch = 12.0), 0.3, 1.0);
        assert!((pitch(&y[2000..10_000]) - 880.0).abs() < 6.0, "{}", pitch(&y[2000..10_000]));
        let (y, _) = run(&with(&|c| c.fine = 100.0), 0.3, 1.0);
        assert!((pitch(&y[2000..12_000]) - 466.16).abs() < 4.0, "{}", pitch(&y[2000..12_000]));
        // Its gain, its pan.
        let (plain, _) = run(&whole(&s), 0.3, 1.0);
        let (y, _) = run(&with(&|c| c.gain = -6.0), 0.3, 1.0);
        assert!((rms(&y[2000..12_000]) / rms(&plain[2000..12_000]) - 0.501).abs() < 0.01);
        let (l, r) = run(&with(&|c| c.pan = 1.0), 0.3, 1.0);
        assert!(rms(&l[2000..12_000]) < 1e-6 && rms(&r[2000..12_000]) > 0.2, "all right");
        // Its fades.
        let (y, _) = run(&with(&|c| c.fade_in = 100.0), 0.3, 1.0);
        assert!(rms(&y[..480]) < 0.15 * rms(&y[6000..9000]), "faded in");
        let (y, _) = run(&with(&|c| c.fade_out = 100.0), 0.6, 1.0);
        assert!(rms(&y[23_200..23_900]) < 0.15 * rms(&y[12_000..15_000]), "faded out");
        // Trimmed: from 10 ms in (a ramp tells where it reads).
        let ramp = sample("ramp", (0..24_000).map(|i| i as f32 / 24_000.0).collect());
        let mut f = whole(&ramp);
        f.chops[0].trim_in = 10.0;
        let (y, _) = play2(&mut FlipState::new(SR), &p, &ramp, &f, FIRST_NOTE, 0.2, 1.0);
        assert!((y[500] - 980.0 / 24_000.0).abs() < 0.002, "{}", y[500]);
        // Its tone: a low pass of its own.
        let hi = sample("high", sine(6000.0, 0.3));
        let mut f = whole(&hi);
        let (open, _) = play2(&mut FlipState::new(SR), &p, &hi, &f, FIRST_NOTE, 0.2, 1.0);
        f.chops[0].cutoff = 400.0;
        let (shut, _) = play2(&mut FlipState::new(SR), &p, &hi, &f, FIRST_NOTE, 0.2, 1.0);
        assert!(rms(&shut[2000..8000]) < 0.05 * rms(&open[2000..8000]));
        // Stretched: an octave up, as long as it was.
        let (y, _) = run(&with(&|c| {
            c.pitch = 12.0;
            c.stretch = true;
        }), 0.6, 1.0);
        assert!(heard(&y[4000..16_000], 880.0) > 4.0 * heard(&y[4000..16_000], 440.0));
        assert!(rms(&y[18_000..22_000]) > 0.05, "still playing at 0.4 s");
    }

    #[test]
    fn chops_play_once_while_held_round_and_back() {
        // A tenth of a second of tone in a chop.
        let s = sample("tone", sine(440.0, 0.5));
        let p = clean();
        let short = |play: Play| Flip { chops: vec![FlipChop { play, ..FlipChop::new(0, 4800) }], ..Flip::default() };
        let run = |f: &Flip| play(&mut FlipState::new(SR), &p, &s, f, FIRST_NOTE, 0.6, 0.4);
        let y = run(&short(Play::OneShot));
        assert!(rms(&y[2000..4000]) > 0.2 && rms(&y[6000..12_000]) < 1e-3, "once");
        let y = run(&short(Play::Loop));
        assert!(rms(&y[12_000..18_000]) > 0.2, "round again while held");
        assert!(rms(&y[22_000..]) < 1e-3, "out when let go");
        let y = run(&short(Play::PingPong));
        assert!(rms(&y[12_000..18_000]) > 0.2, "to its end and back while held");
        // A gated chop stops when let go, a one-shot plays on.
        let long = |play: Play| Flip { chops: vec![FlipChop { play, ..FlipChop::new(0, 24_000) }], ..Flip::default() };
        let run = |f: &Flip| play(&mut FlipState::new(SR), &p, &s, f, FIRST_NOTE, 0.5, 0.1);
        assert!(rms(&run(&long(Play::Gate))[9000..12_000]) < 1e-3);
        assert!(rms(&run(&long(Play::OneShot))[9000..12_000]) > 0.2);
        // Ping-pong on a ramp: up, then down again.
        let ramp = sample("ramp", (0..48_000).map(|i| i as f32 / 48_000.0).collect());
        let f = Flip { chops: vec![FlipChop { play: Play::PingPong, ..FlipChop::new(0, 4800) }], ..Flip::default() };
        let y = play(&mut FlipState::new(SR), &p, &ramp, &f, FIRST_NOTE, 0.3, 1.0);
        assert!(y[6000] < y[4500] && y[6000] > 0.01, "{} then {}", y[4500], y[6000]);
        // Choke groups: a chop cuts the others of its group only.
        let two = |g: u8| Flip { chops: vec![FlipChop { choke: g, ..FlipChop::new(0, 24_000) }, FlipChop { choke: g, ..FlipChop::new(0, 24_000) }], ..Flip::default() };
        let mut q = clean();
        q[CHOKE] = 0.0;
        let both = |f: &Flip| perform(&mut FlipState::new(SR), &q, &s, f, &[(0, FIRST_NOTE, 1.0), (40, FIRST_NOTE + 1, 1.0)], 120);
        let apart = both(&two(0));
        let cut = both(&two(1));
        assert!(rms(&apart[8000..12_000]) > 1.6 * rms(&cut[8000..12_000]), "{} and {}", rms(&apart[8000..12_000]), rms(&cut[8000..12_000]));
    }

    #[test]
    fn mono_legato_and_glide() {
        let s = sample("tone", sine(440.0, 2.0));
        let f = whole(&s);
        let mut p = clean();
        p[MODE] = 1.0;
        p[VOICING] = 1.0;
        // Mono: one voice; with a glide the second note slides there.
        p[GLIDE] = 100.0;
        let mut st = FlipState::new(SR);
        let notes = [(0, LOW_ROOT - 12, 1.0), (40, LOW_ROOT, 1.0)];
        let y = perform(&mut st, &p, &s, &f, &notes, 140);
        let mut vis = [0.0; crate::engine::INST_VALS];
        st.visual(&mut vis);
        assert_eq!(vis[VIS_VOICES], 1.0);
        let at = 40 * 128;
        let early = pitch(&y[at + 300..at + 2200]);
        assert!(early > 230.0 && early < 420.0, "on its way: {early}");
        assert!((pitch(&y[at + 6000..at + 12_000]) - 440.0).abs() < 4.0, "there: {}", pitch(&y[at + 6000..at + 12_000]));
        // Legato: the envelope goes on (a slow attack is not heard again).
        p[ATTACK] = 150.0;
        p[GLIDE] = 0.0;
        let quiet_start = |voicing: f32| {
            let mut q = p.clone();
            q[VOICING] = voicing;
            let y = perform(&mut FlipState::new(SR), &q, &s, &f, &[(0, LOW_ROOT, 1.0), (100, LOW_ROOT + 7, 1.0)], 140);
            rms(&y[100 * 128 + 200..100 * 128 + 1200])
        };
        assert!(quiet_start(2.0) > 0.3, "legato: at once");
        assert!(quiet_start(1.0) < 0.1, "mono: in again");
        // Poly with one voice: the new note takes the old one's place.
        let mut q = clean();
        q[CHOKE] = 0.0;
        q[VOICES] = 1.0;
        let mut st = FlipState::new(SR);
        perform(&mut st, &q, &s, &f, &[(0, LOW_ROOT, 1.0), (10, LOW_ROOT + 5, 1.0)], 20);
        st.visual(&mut vis);
        assert_eq!(vis[VIS_VOICES], 1.0);
    }

    #[test]
    fn velocity_and_the_envelope() {
        let s = sample("tone", sine(440.0, 1.0));
        let f = whole(&s);
        let mut p = clean();
        p[VEL_LEVEL] = 1.0;
        p[VEL_PITCH] = 12.0;
        let soft = perform(&mut FlipState::new(SR), &p, &s, &f, &[(0, FIRST_NOTE, 0.5)], 60);
        let hard = perform(&mut FlipState::new(SR), &p, &s, &f, &[(0, FIRST_NOTE, 1.0)], 60);
        assert!((rms(&soft[2000..6000]) / rms(&hard[2000..6000]) - 0.5).abs() < 0.03);
        assert!((pitch(&soft[2000..7000]) - 311.1).abs() < 4.0, "half way down an octave: {}", pitch(&soft[2000..7000]));
        // Decay to the sustain.
        let mut p = clean();
        p[DECAY] = 50.0;
        p[SUSTAIN] = 0.25;
        let y = perform(&mut FlipState::new(SR), &p, &s, &f, &[(0, FIRST_NOTE, 1.0)], 100);
        assert!((rms(&y[8000..12_000]) / rms(&y[100..600]) - 0.25).abs() < 0.05);
        // The filter's envelope and velocity open it.
        let bright = sample("bright", sine(4000.0, 1.0));
        let fb = whole(&bright);
        let mut p = clean();
        p[LOW_PASS] = 300.0;
        let shut = perform(&mut FlipState::new(SR), &p, &bright, &fb, &[(0, FIRST_NOTE, 1.0)], 30);
        p[FILTER_ENV] = 1.0;
        let opened = perform(&mut FlipState::new(SR), &p, &bright, &fb, &[(0, FIRST_NOTE, 1.0)], 30);
        assert!(rms(&opened[500..3000]) > 5.0 * rms(&shut[500..3000]));
        // A high pass keeps the highs.
        let low = sample("low", sine(100.0, 1.0));
        let fl = whole(&low);
        let mut p = clean();
        p[FILTER_TYPE] = 1.0;
        p[LOW_PASS] = 2000.0;
        let y = perform(&mut FlipState::new(SR), &p, &low, &fl, &[(0, FIRST_NOTE, 1.0)], 30);
        assert!(rms(&y[2000..3500]) < 0.05, "{}", rms(&y[2000..3500]));
    }

    #[test]
    fn the_lfo_keeps_the_songs_time() {
        let s = sample("tone", sine(440.0, 2.0));
        let f = whole(&s);
        let mut p = clean();
        p[LFO_SHAPE] = 2.0;
        p[LFO_RATE] = 4.0;
        p[LFO_PAN] = 1.0;
        // A square a beat long at 120, from the song's one: right, then left.
        let mut st = FlipState::new(SR);
        st.set_transport(120.0, 0.0, true);
        let (l, r) = play2(&mut st, &p, &s, &f, FIRST_NOTE, 0.5, 1.0);
        assert!(rms(&l[1000..11_000]) < 1e-4 && rms(&r[1000..11_000]) > 0.2, "right first");
        assert!(rms(&r[13_000..23_000]) < 1e-4 && rms(&l[13_000..23_000]) > 0.2, "then left");
        // Its pitch: an octave up for half a beat, an octave down for the other.
        let mut p = clean();
        p[LFO_SHAPE] = 2.0;
        p[LFO_RATE] = 4.0;
        p[LFO_PITCH] = 12.0;
        let mut st = FlipState::new(SR);
        st.set_transport(120.0, 0.0, true);
        let y = play(&mut st, &p, &s, &f, FIRST_NOTE, 0.5, 1.0);
        assert!((pitch(&y[1000..11_000]) - 880.0).abs() < 8.0, "{}", pitch(&y[1000..11_000]));
        assert!((pitch(&y[13_000..23_000]) - 220.0).abs() < 4.0, "{}", pitch(&y[13_000..23_000]));
    }

    #[test]
    fn the_crusher_the_wobbles_and_the_width() {
        let s = SampleRef::new(SampleData::from_buffers("stereo", None, 48_000, sine(440.0, 0.5), sine(660.0, 0.5)));
        let f = whole(&s);
        let mut p = clean();
        p[WIDTH] = 0.0;
        let (l, r) = play2(&mut FlipState::new(SR), &p, &s, &f, FIRST_NOTE, 0.3, 1.0);
        assert!(l.iter().zip(&r).all(|(a, b)| (a - b).abs() < 1e-6), "mono");
        let mut p = clean();
        p[BITS] = 2.0;
        let y = play(&mut FlipState::new(SR), &p, &s, &f, FIRST_NOTE, 0.3, 1.0);
        let mut levels: Vec<i32> = y[2000..10_000].iter().map(|v| (v * 1000.0).round() as i32).collect();
        levels.sort_unstable();
        levels.dedup();
        assert!(levels.len() <= 4, "{levels:?}");
        p[BITS] = 16.0;
        p[CRUSH_RATE] = 2000.0;
        let y = play(&mut FlipState::new(SR), &p, &s, &f, FIRST_NOTE, 0.3, 1.0);
        assert!(y[2000..2200].windows(2).filter(|w| w[0] != w[1]).count() < 20, "held for 24 samples at a time");
        for kind in 0..3 {
            let mut p = clean();
            p[WOBBLE] = 1.0;
            p[WOBBLE_TYPE] = kind as f32;
            let y = play(&mut FlipState::new(SR), &p, &s, &f, FIRST_NOTE, 0.3, 1.0);
            assert!(y.iter().all(|v| v.is_finite()) && rms(&y) > 0.1);
        }
    }

    #[test]
    fn chops_are_tuned_into_the_songs_key() {
        assert_eq!(key_shift((9, true), Scale::new(0, ScaleKind::Minor)), 3);
        assert_eq!(key_shift((0, false), Scale::new(7, ScaleKind::Major)), -5);
        assert_eq!(key_shift((9, true), Scale::new(0, ScaleKind::Major)), 0, "relative keys");
        assert_eq!(key_shift((2, true), Scale::new(1, ScaleKind::Minor)), -1);
        let s = sample("tone", sine(440.0, 0.5));
        let mut f = whole(&s);
        f.key = Some((9, true));
        let mut st = FlipState::new(SR);
        st.key = Scale::new(0, ScaleKind::Minor);
        let y = play(&mut st, &clean(), &s, &f, FIRST_NOTE, 0.3, 1.0);
        let want = 440.0 * 2f32.powf(3.0 / 12.0);
        assert!((pitch(&y[2000..12_000]) - want).abs() < 4.0, "{} for {want}", pitch(&y[2000..12_000]));
        // Song Key off: as it is.
        let mut p = clean();
        p[SONG_KEY] = 0.0;
        let mut st = FlipState::new(SR);
        st.key = Scale::new(0, ScaleKind::Minor);
        let y = play(&mut st, &p, &s, &f, FIRST_NOTE, 0.3, 1.0);
        assert!((pitch(&y[2000..12_000]) - 440.0).abs() < 3.0);
        // The keys below: the chosen chop at their pitch.
        let mut st = FlipState::new(SR);
        let y = play(&mut st, &p, &s, &f, LOW_ROOT + 7, 0.3, 1.0);
        assert!((pitch(&y[2000..10_000]) - 659.26).abs() < 5.0, "{}", pitch(&y[2000..10_000]));
    }

    #[test]
    fn a_held_chop_stutters_and_the_hiss_dies_away() {
        let s = sample("tone", sine(330.0, 1.0));
        let f = whole(&s);
        let mut p = clean();
        p[STUTTER] = 2.0;
        let mut st = FlipState::new(SR);
        st.bpm = 120.0;
        // A sixteenth at 120 is 6000 samples: the chop starts again on each.
        let y = play(&mut st, &p, &s, &f, FIRST_NOTE, 0.5, 0.4);
        let near = |a: usize, b: usize| (a..a + 3000).map(|i| (y[i] - y[i - a + b]).abs()).fold(0.0f32, f32::max);
        assert!(near(6200, 200) < 0.01, "again from the start: {}", near(6200, 200));
        assert!(near(12_200, 200) < 0.01);
        // Without the stutter it goes on.
        p[STUTTER] = 0.0;
        let mut st = FlipState::new(SR);
        let z = play(&mut st, &p, &s, &f, FIRST_NOTE, 0.5, 0.4);
        assert!((6200..9200).map(|i| (z[i] - z[i - 6000]).abs()).fold(0.0f32, f32::max) > 0.1);
        // With hiss: let go and nothing plays, and the hiss goes too.
        p[HISS] = 1.0;
        let mut st = FlipState::new(SR);
        let y = play(&mut st, &p, &s, &f, FIRST_NOTE, 0.5, 0.4);
        let mut tail = Vec::new();
        for _ in 0..(3.0 * SR / 128.0) as usize {
            let (mut l, mut r) = ([0.0f32; 128], [0.0f32; 128]);
            st.note_off(FIRST_NOTE);
            st.render(&p, Some(&s), &f, &mut l, &mut r);
            tail.extend_from_slice(&l);
        }
        let end = &tail[tail.len() - 4800..];
        assert!(end.iter().all(|v| v.abs() < 1e-3), "quiet at the end");
        assert!(y.iter().chain(&tail).all(|v| v.is_finite()));
    }

    #[test]
    fn an_old_set_loads_and_sounds_as_it_did() {
        use crate::dsp::princess_flip_legacy as old;
        // A flip as 0.90 saved it: chops of three fields, no chopping, sixteen parameters.
        let json = r#"{"chops":[{"start":0,"end":24000,"reverse":false},{"start":24000,"end":48000,"reverse":true},{"start":48000,"end":72000,"reverse":false},{"start":72000,"end":96000}],"key":[9,true],"bpm":120.0}"#;
        let new: Flip = serde_json::from_str(json).unwrap();
        let was: old::Flip = serde_json::from_str(json).unwrap();
        assert_eq!(new.chopping, Chopping::default());
        assert!(new.chops.iter().all(|c| !FlipChop { reverse: false, ..*c }.has_sound() && !c.manual));
        assert!(new.chops[1].reverse);
        // And written back the same way (nothing new where nothing changed).
        let back = serde_json::to_string(&new.chops[1]).unwrap();
        assert_eq!(back, r#"{"start":24000,"end":48000,"reverse":true}"#);
        let mut v = hits(12_000);
        v.extend(sine(330.0, 0.5));
        let s = sample("old", v);
        // A tune of chops and the keys below, held and let go, in every old preset.
        let notes: Vec<(usize, u8, f32)> = vec![(0, FIRST_NOTE, 0.9), (30, FIRST_NOTE, 0.0), (35, FIRST_NOTE + 1, 0.5), (50, FIRST_NOTE + 3, 1.0), (52, FIRST_NOTE + 2, 0.7), (90, FIRST_NOTE + 2, 0.0), (95, LOW_ROOT + 5, 0.8), (140, LOW_ROOT + 5, 0.0), (150, FIRST_NOTE + 1, 0.3), (151, FIRST_NOTE, 0.6), (152, FIRST_NOTE + 3, 1.0), (200, FIRST_NOTE + 3, 0.0)];
        for name in old::preset_names() {
            let mut params = old::preset(name);
            assert_eq!(params.len(), 16);
            let was_p = params.clone();
            crate::params::fix_len(&mut params, SPECS);
            let (mut a, mut b) = (FlipState::new(SR), old::FlipState::new(SR));
            a.key = Scale::new(0, ScaleKind::Minor);
            b.key = Scale::new(0, ScaleKind::Minor);
            a.set_transport(140.0, 0.0, false);
            b.set_transport(140.0);
            let (mut ya, mut yb) = (Vec::new(), Vec::new());
            for blk in 0..300 {
                for &(_, note, vel) in notes.iter().filter(|n| n.0 == blk) {
                    if vel > 0.0 {
                        a.note_on(note, vel);
                        b.note_on(note, vel);
                    } else {
                        a.note_off(note);
                        b.note_off(note);
                    }
                }
                // Blocks of all sizes, as the engine cuts them at its events.
                let len = 64 + (blk * 37) % 200;
                let (mut l, mut r) = (vec![0.0f32; len], vec![0.0f32; len]);
                let (mut l2, mut r2) = (vec![0.0f32; len], vec![0.0f32; len]);
                a.render(&params, Some(&s), &new, &mut l, &mut r);
                b.render(&was_p, Some(&s), &was, &mut l2, &mut r2);
                ya.extend(l.iter().chain(&r));
                yb.extend(l2.iter().chain(&r2));
            }
            assert!(rms(&yb) > 0.01, "{name} plays");
            let worst = ya.iter().zip(&yb).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
            assert_eq!(worst, 0.0, "{name} sounds as it did");
        }
    }

    #[test]
    fn auto_flips_keep_time() {
        // Eight chops of a beat each (at 120).
        let f = Flip { chops: (0..8).map(|k| FlipChop::new(k * 24_000, (k + 1) * 24_000)).collect(), ..Flip::default() };
        for style in AutoStyle::ALL {
            let notes = auto_flip(&f, 48_000, 120.0, 4.0, style, 4, 11);
            assert!(!notes.is_empty());
            let g = style.grid();
            for (i, n) in notes.iter().enumerate() {
                assert!((n.start / g - (n.start / g).round()).abs() < 1e-9, "{style:?}: on the grid at {}", n.start);
                assert!(n.start >= 0.0 && n.start + n.length <= 16.0 + 1e-9);
                assert!(n.pitch >= FIRST_NOTE && n.pitch < FIRST_NOTE + 8);
                if let Some(m) = notes.get(i + 1) {
                    assert!(n.start + n.length <= m.start + 1e-9, "{style:?}: no note over the next");
                }
            }
        }
        // Straight: every chop once before any comes again, the bars filled.
        let notes = auto_flip(&f, 48_000, 120.0, 4.0, AutoStyle::Straight, 4, 3);
        let mut first: Vec<u8> = notes.iter().take(8).map(|n| n.pitch).collect();
        first.sort_unstable();
        assert_eq!(first, (0..8).map(|k| FIRST_NOTE + k).collect::<Vec<_>>());
        assert!((notes.iter().map(|n| n.length).sum::<f64>() - 16.0).abs() < 1e-9);
        assert_ne!(notes.iter().take(8).map(|n| n.pitch).collect::<Vec<_>>(), (0..8).map(|k| FIRST_NOTE + k).collect::<Vec<_>>(), "in a new order");
        // The bounce: three, three and two sixteenths, the same chop on each one.
        let notes = auto_flip(&f, 48_000, 120.0, 4.0, AutoStyle::Bounce, 2, 5);
        let starts: Vec<f64> = notes.iter().map(|n| n.start).collect();
        assert_eq!(starts, vec![0.0, 0.75, 1.5, 2.0, 2.75, 3.5, 4.0, 4.75, 5.5, 6.0, 6.75, 7.5]);
        assert_eq!(notes[0].pitch, notes[6].pitch);
        assert!(auto_flip(&Flip::default(), 48_000, 120.0, 4.0, AutoStyle::Stutter, 4, 1).is_empty());
    }

    #[test]
    fn a_drum_rack_gets_a_pad_a_chop() {
        let s = sample("loop", sine(220.0, 2.0));
        let mut f = Flip { chops: (0..20).map(|k| FlipChop::new(k * 4000, (k + 1) * 4000)).collect(), ..Flip::default() };
        f.chops[2].pitch = 5.0;
        f.chops[2].pan = -0.5;
        f.chops[2].choke = 2;
        f.chops[2].gain = -3.0;
        f.chops[3].play = Play::Gate;
        let pads = drum_pads(&f, &s, 3, false);
        assert_eq!(pads.len(), crate::dsp::drums::MAX_PADS);
        assert_eq!(pads[0].note, crate::slicing::FIRST_NOTE);
        assert_eq!(pads[2].note, crate::slicing::FIRST_NOTE + 2);
        assert_eq!((pads[2].tune, pads[2].pan, pads[2].choke, pads[2].level_db), (8.0, -0.5, 2, -3.0));
        assert!(pads[3].gate && !pads[2].gate);
        let smp = pads[2].sample.as_ref().unwrap();
        assert_eq!(smp.frames(), 4000);
        assert_eq!(smp.region.map(|r| (r.start, r.end)), Some((8000, 12_000)));
    }

    #[test]
    fn the_presets_have_their_parameters() {
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            for (v, s) in p.iter().zip(SPECS) {
                assert!(*v >= s.min && *v <= s.max, "{name}: {} = {v}", s.name);
            }
        }
        let mut names: Vec<&str> = SPECS.iter().map(|s| s.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), SPECS.len());
        // The old ones as they were.
        for name in crate::dsp::princess_flip_legacy::preset_names() {
            assert_eq!(preset(name)[..16], crate::dsp::princess_flip_legacy::preset(name)[..], "{name}");
        }
    }
}
