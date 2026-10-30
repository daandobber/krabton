//! The band: the chord progression the chord players share (Progression,
//! Guitar Chords, Ukulele and the players; see `players::band_part`), and
//! what every other sequencer in the band hears of it.
//!
//! - Parts: the band can play a song of parts (Verse, Chorus, Bridge...,
//!   up to six, A to F), each with a progression of its own, in a form (a
//!   part so many times, then the next), so all of it moves from verse to
//!   chorus together. A cue (the next part, or one picked) waits for the end
//!   of the part playing, or the next bar. Without parts it is today's one
//!   progression, as the chord players have it.
//! - Follow: a sequencer that is not a chord player follows the band's chord
//!   where it plays a note: *Root* moves the note by the chord's root (in
//!   steps of the key, the nearer way round), *Chord* also pulls it onto the
//!   nearest note of the chord, *Bass* onto the chord's root or fifth.
//!
//! The engine works out where the band is once a block (`BandRt`) and hands
//! it to every MIDI effect in `MidiCtx::band`, without allocating.

use crate::classic_seq::{prog_slot, quality_steps, PROG_SLOTS};
use crate::midi_fx::{MidiCtx, MidiFxKind};
use crate::project::Project;
use crate::scale::Scale;
use crate::sequencers::players::{band_param, CHORDS, LENGTH};
use serde::{Deserialize, Serialize};

/// How many parts a song can have, and their letters.
pub const MAX_PARTS: usize = 6;
pub const LETTERS: [&str; MAX_PARTS] = ["A", "B", "C", "D", "E", "F"];
/// Names new parts get, in turn.
pub const PART_NAMES: [&str; MAX_PARTS] = ["Verse", "Chorus", "Bridge", "Intro", "Outro", "Solo"];
/// How many steps a form can have.
pub const MAX_FORM: usize = 16;
/// How a sequencer follows the band (its Follow parameter).
pub const FOLLOWS: &[&str] = &["Root", "Chord", "Bass"];
/// No part (no parts, or no cue).
pub const NONE: u8 = 0xff;

/// The sequencers that can follow the band (they have Band and Follow, last).
pub const FOLLOWERS: [MidiFxKind; 40] = {
    use MidiFxKind as K;
    [
        K::Step,
        K::Acid,
        K::Polymeter,
        K::Euclid,
        K::Turing,
        K::Life,
        K::Bounce,
        K::Drunk,
        K::Orbits,
        K::Chaos,
        K::Automaton,
        K::Ant,
        K::Pong,
        K::Markov,
        K::Rain,
        K::Words,
        K::Lorenz,
        K::Loops,
        K::Bloom,
        K::Chimes,
        K::DriftChords,
        K::Tintinnabuli,
        K::Pedal,
        K::Marbles,
        K::Endless,
        K::Draw,
        K::Touch,
        K::Sing,
        K::Fireflies,
        K::Sandpile,
        K::Pendulums,
        K::Flock,
        K::Plants,
        K::Bells,
        K::Canon,
        K::Gears,
        K::Formula,
        K::Gamelan,
        K::Raga,
        K::Call,
    ]
};

/// A part of the song: its name and its progression.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BandPart {
    pub name: String,
    /// How long each chord lasts (as a chord player's Length: 1 beat, 2
    /// beats, 1 bar, 2 bars).
    pub length: u8,
    /// How many of the chords play (1 to 8).
    pub count: u8,
    /// Each chord: its degree (1 to 7, 0 a rest) and quality.
    pub chords: [(u8, u8); PROG_SLOTS],
}

impl BandPart {
    /// A part from what the band shares (see `players::band_part`).
    pub fn from_part(name: &str, part: &[u8]) -> Self {
        let mut p = Self { name: name.to_string(), length: 2, count: 4, chords: [(0, 0); PROG_SLOTS] };
        p.set_part(part);
        p
    }

    /// What the band shares (see `players::band_part`).
    pub fn part(&self) -> Vec<u8> {
        let mut v = vec![self.length, self.count];
        for c in self.chords {
            v.extend([c.0, c.1]);
        }
        v
    }

    pub fn set_part(&mut self, part: &[u8]) {
        if part.len() < 2 + PROG_SLOTS * 2 {
            return;
        }
        self.length = part[0].min(3);
        self.count = part[1].clamp(1, PROG_SLOTS as u8);
        for (i, c) in self.chords.iter_mut().enumerate() {
            *c = (part[2 + i * 2].min(7), part[3 + i * 2]);
        }
    }

    fn prog(&self) -> Prog {
        Prog { length: self.length.min(3), count: self.count.clamp(1, PROG_SLOTS as u8), chords: self.chords }
    }
}

/// A step of the form: a part, so many times.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormStep {
    pub part: u8,
    pub times: u8,
}

/// The band's song: its parts and its form (none: one progression, as the
/// chord players have it).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BandSong {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<BandPart>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub form: Vec<FormStep>,
    /// A cue changes the part on the next bar (else at the end of the part).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cue_on_bar: bool,
}

impl BandSong {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// How long the whole form takes, in beats.
    pub fn form_beats(&self) -> f64 {
        self.form.iter().map(|f| self.prog(f.part as usize).beats() * f.times.max(1) as f64).sum()
    }

    fn prog(&self, part: usize) -> Prog {
        match self.parts.get(part.min(self.parts.len().saturating_sub(1))) {
            Some(p) => p.prog(),
            None => Prog::default(),
        }
    }

    /// What moves the parts in time (not the chords themselves).
    fn signature(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |x: u64| h = (h ^ x).wrapping_mul(0x0100_0000_01b3);
        for p in &self.parts {
            mix(p.length as u64 | (p.count as u64) << 8);
        }
        mix(0xffff);
        for f in &self.form {
            mix(f.part as u64 | (f.times as u64) << 8);
        }
        h
    }
}

/// A progression as the engine plays it (copied, so it never allocates).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prog {
    pub length: u8,
    pub count: u8,
    pub chords: [(u8, u8); PROG_SLOTS],
}

impl Default for Prog {
    fn default() -> Self {
        Self { length: 2, count: 1, chords: [(0, 0); PROG_SLOTS] }
    }
}

impl Prog {
    /// A chord player's progression (its Length, Chords and chords).
    pub fn of(params: &[f32], data: &[u8]) -> Self {
        let mut chords = [(0, 0); PROG_SLOTS];
        for (i, c) in chords.iter_mut().enumerate() {
            let (d, q, _) = prog_slot(data, i);
            *c = (d, q);
        }
        let length = params.get(LENGTH).map(|v| v.round().clamp(0.0, 3.0) as u8).unwrap_or(2);
        let count = params.get(CHORDS).map(|v| v.round().clamp(1.0, PROG_SLOTS as f32) as u8).unwrap_or(4);
        Self { length, count, chords }
    }

    pub fn chord_beats(&self) -> f64 {
        [1.0, 2.0, 4.0, 8.0][(self.length as usize).min(3)]
    }

    /// Once through all its chords.
    pub fn beats(&self) -> f64 {
        self.chord_beats() * self.count.clamp(1, PROG_SLOTS as u8) as f64
    }
}

/// A chord of the band: which of the part's chords, its degree and
/// quality, and when it started and how long it lasts (beats).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chord {
    pub slot: usize,
    pub degree: u8,
    pub quality: u8,
    pub start: f64,
    pub beats: f64,
}

/// Where the band is this block (see `MidiCtx::band`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandNow {
    /// There is a band (a chord player with Band on, or parts).
    pub on: bool,
    /// It plays parts (the chord players in it take their chords from here).
    pub parts: bool,
    /// The song's key (the chords are its degrees).
    pub key: Scale,
    /// The song's beat (the session clock) at the start of the block.
    pub beat: f64,
    /// The progression playing, from which beat, until which; then the next.
    pub prog: Prog,
    pub from: f64,
    pub until: f64,
    pub next: Prog,
    /// The part playing, the form's step and which time round (NONE without
    /// parts; the step NONE off the form), and the part a cue goes to.
    pub part: u8,
    pub step: u8,
    pub pass: u8,
    pub cued: u8,
}

impl Default for BandNow {
    fn default() -> Self {
        Self { on: false, parts: false, key: Scale::default(), beat: 0.0, prog: Prog::default(), from: 0.0, until: f64::INFINITY, next: Prog::default(), part: NONE, step: NONE, pass: 0, cued: NONE }
    }
}

impl BandNow {
    /// The band's chord at `beat` (None without a band).
    pub fn at(&self, beat: f64) -> Option<Chord> {
        if !self.on {
            return None;
        }
        let (prog, origin) = if beat >= self.until { (&self.next, self.until) } else { (&self.prog, self.from) };
        let len = prog.chord_beats();
        let i = ((beat - origin) / len).floor().max(0.0) as i64;
        let slot = i.rem_euclid(prog.count.clamp(1, PROG_SLOTS as u8) as i64) as usize;
        let (degree, quality) = prog.chords[slot];
        Some(Chord { slot, degree, quality, start: origin + i as f64 * len, beats: len })
    }

    /// For the UI: the part, step, time round, cue and the chord now, in one word.
    pub fn pack(&self) -> u64 {
        let c = self.at(self.beat + 1e-6);
        let (slot, degree, quality) = c.map(|c| (c.slot as u64, c.degree as u64, c.quality as u64)).unwrap_or((NONE as u64, 0, 0));
        self.part as u64 | (self.step as u64) << 8 | (self.pass as u64) << 16 | (self.cued as u64) << 24 | slot << 32 | (degree & 15) << 40 | (quality & 15) << 44 | (self.on as u64) << 48 | (self.parts as u64) << 49
    }
}

/// What the UI hears of the band (see `BandNow::pack`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct BandStatus {
    pub on: bool,
    pub parts: bool,
    pub part: u8,
    pub step: u8,
    pub pass: u8,
    pub cued: u8,
    pub slot: u8,
    pub degree: u8,
    pub quality: u8,
}

impl BandStatus {
    pub fn unpack(v: u64) -> Self {
        let b = |s: u32| ((v >> s) & 0xff) as u8;
        Self { on: v >> 48 & 1 == 1, parts: v >> 49 & 1 == 1, part: b(0), step: b(8), pass: b(16), cued: b(24), slot: b(32), degree: ((v >> 40) & 15) as u8, quality: ((v >> 44) & 15) as u8 }
    }
}

/// Where a sequencer that can follow the band keeps its Band switch (its
/// Follow is the one after it).
pub fn follow_param(kind: MidiFxKind) -> Option<usize> {
    (FOLLOWERS.contains(&kind) || kind == MidiFxKind::Arpeggiator).then(|| kind.specs().iter().position(|s| s.name == "Band")).flatten()
}

/// How a device follows the band now (an index into FOLLOWS), or None when
/// it is not in it.
pub fn follow_mode(kind: MidiFxKind, p: &[f32]) -> Option<u8> {
    let i = follow_param(kind)?;
    (p.get(i)? >= &0.5).then(|| p.get(i + 1).map(|v| v.round().clamp(0.0, FOLLOWS.len() as f32 - 1.0) as u8).unwrap_or(0))
}

/// The band's beat at frame `at` of the block.
pub fn beat_at(ctx: &MidiCtx, at: u64) -> f64 {
    // A note's frame is rounded down: a hair later, so one on a chord's first beat is in it.
    ctx.band.beat + (at as f64 - ctx.now as f64) / ctx.samples_per_beat.max(1.0) + 2e-4
}

/// `pitch` moved with the band's chord at `beat` (see FOLLOWS). A rest, or
/// no band, leaves it as it is.
pub fn follow(ctx: &MidiCtx, mode: u8, beat: f64, pitch: i32) -> i32 {
    let Some(c) = ctx.band.at(beat) else { return pitch };
    if c.degree == 0 {
        return pitch;
    }
    let key = &ctx.band.key;
    let n = key.len().max(1) as i32;
    let root = (c.degree as i32 - 1) % n;
    // The nearer way round: up to a fourth up, or down.
    let s = if root > n / 2 { root - n } else { root };
    let moved = if ctx.scale_on {
        let (d, rest) = key.locate(pitch);
        key.pitch_at(d + s) + rest
    } else {
        pitch + key.pitch_at(s) - key.pitch_at(0)
    };
    let steps: &[i32] = match mode {
        0 => return moved,
        2 => &[0, 4],
        _ => quality_steps(c.quality),
    };
    let mut tones = [0i32; 5];
    for (t, st) in tones.iter_mut().zip(steps) {
        *t = key.pitch_at(root + st).rem_euclid(12);
    }
    let tones = &tones[..steps.len().min(5)];
    // The nearest note of the chord (ties go down).
    for d in 0..12 {
        for q in [moved - d, moved + d] {
            if tones.contains(&q.rem_euclid(12)) {
                return q;
            }
        }
    }
    moved
}

/// The chord a chord player plays on its sixteenth `k`: its own, or with
/// Band on while the band plays parts, the band's (with its own inversion,
/// or neck, for that chord). (slot, degree, quality, inversion, sixteenths
/// the chord lasts, sixteenths into it.)
pub fn chord_for(kind: MidiFxKind, p: &[f32], data: &[u8], k: i64, ctx: &MidiCtx) -> (usize, u8, u8, u8, i64, i64) {
    let in_band = ctx.band.parts && band_param(kind).and_then(|i| p.get(i)).is_some_and(|v| *v >= 0.5);
    if in_band {
        let beat = ctx.band.beat + (k as f64 * 0.25 - ctx.beat) + 1e-6;
        if let Some(c) = ctx.band.at(beat) {
            let steps = ((c.beats * 4.0).round() as i64).max(1);
            let into = (((beat - c.start) * 4.0).floor() as i64).clamp(0, steps - 1);
            return (c.slot, c.degree, c.quality, prog_slot(data, c.slot).2, steps, into);
        }
    }
    let beats = [1.0, 2.0, 4.0, 8.0][(p.get(LENGTH).copied().unwrap_or(2.0).round().max(0.0) as usize).min(3)];
    let steps = (beats * 4.0) as i64;
    let n = p.get(CHORDS).copied().unwrap_or(4.0).round().clamp(1.0, PROG_SLOTS as f32) as i64;
    let slot = k.div_euclid(steps).rem_euclid(n) as usize;
    let (d, q, inv) = prog_slot(data, slot);
    (slot, d, q, inv, steps, k.rem_euclid(steps))
}

// ---------------------------------------------------------------- the engine's

/// A cue: the next part, or a part picked (by its index).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    Next,
    Part(u8),
}

/// Where the song is in its parts.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Cursor {
    /// The form's step, which time round, and the beat this time round began.
    step: usize,
    pass: u32,
    start: f64,
    /// Off the form: a part played round and round until the next cue.
    held: Option<u8>,
}

/// The part a cursor plays.
fn part_of(song: &BandSong, c: &Cursor) -> usize {
    let s = match c.held {
        Some(s) => s as usize,
        None => song.form.get(c.step).map(|f| f.part as usize).unwrap_or(0),
    };
    s.min(song.parts.len().saturating_sub(1))
}

/// Where the cursor goes after this time round (without a cue).
fn after(song: &BandSong, mut c: Cursor) -> Cursor {
    c.start += song.prog(part_of(song, &c)).beats();
    c.pass += 1;
    if c.held.is_none() && !song.form.is_empty() {
        let times = song.form.get(c.step).map(|f| f.times.max(1) as u32).unwrap_or(1);
        if c.pass >= times {
            c.step = (c.step + 1) % song.form.len();
            c.pass = 0;
        }
    }
    c
}

/// Where a cue takes the cursor (from the same beat).
fn take_cue(song: &BandSong, mut c: Cursor, cue: Cue) -> Cursor {
    let n = song.form.len();
    c.pass = 0;
    match cue {
        Cue::Next if n == 0 => c.held = Some(((part_of(song, &c) + 1) % song.parts.len().max(1)) as u8),
        Cue::Next => {
            // Off the form, back to it after where it left it.
            c.held = None;
            c.step = (c.step + 1) % n;
        }
        Cue::Part(s) => match (1..=n).map(|i| (c.step + i) % n).find(|&i| song.form[i].part == s) {
            // The next time the form plays it, or off the form until the next cue.
            Some(i) => {
                c.held = None;
                c.step = i;
            }
            None => c.held = Some(s),
        },
    }
    c
}

/// The cursor moved on to `beat`, a cue taken at the end of its part (or at
/// `on_bar`, if sooner).
fn advance(song: &BandSong, mut c: Cursor, beat: f64, mut cue: Option<Cue>, on_bar: f64) -> (Cursor, Option<Cue>) {
    if c.held.is_none() && c.step >= song.form.len() {
        c.step = 0;
        c.pass = 0;
    }
    for _ in 0..4096 {
        let len = song.prog(part_of(song, &c)).beats();
        let end = if cue.is_some() { (c.start + len).min(on_bar) } else { c.start + len };
        if beat < end - 1e-9 {
            break;
        }
        c = match cue.take() {
            Some(q) => Cursor { start: end, ..take_cue(song, c, q) },
            None => after(song, c),
        };
    }
    (c, cue)
}

/// The band in the engine: where the song is in its parts, and a cue waiting.
pub struct BandRt {
    at: Cursor,
    cue: Option<Cue>,
    /// The bar a cue goes on (NaN: not known yet).
    cue_at: f64,
    /// The beat and the parts' layout last seen (stopped, a new one starts over).
    last: f64,
    sig: u64,
}

impl Default for BandRt {
    fn default() -> Self {
        Self { at: Cursor::default(), cue: None, cue_at: f64::NAN, last: f64::NAN, sig: 0 }
    }
}

impl BandRt {
    /// A cue (None: forget the one waiting).
    pub fn cue(&mut self, cue: Option<Cue>) {
        self.cue = cue;
        self.cue_at = f64::NAN;
    }

    /// Where the band is at `beat` (the session clock playing; stopped, the
    /// song position it will start from).
    pub fn resolve(&mut self, project: &Project, beat: f64, playing: bool) -> BandNow {
        let song = &project.band;
        let key = project.scale;
        if song.parts.is_empty() {
            self.cue = None;
            return one_progression(project, beat);
        }
        let sig = song.signature();
        let on_bar = if playing {
            if self.cue.is_some() && self.cue_at.is_nan() {
                let bar = project.beats_per_bar_f().max(1.0);
                self.cue_at = ((beat / bar + 1e-9).floor() + 1.0) * bar;
            }
            if song.cue_on_bar && self.cue.is_some() {
                self.cue_at
            } else {
                f64::INFINITY
            }
        } else {
            f64::INFINITY
        };
        if !playing {
            // Stopped: from the start, as far as the song position (a cue goes at once).
            if sig != self.sig || beat != self.last {
                let whole = song.form_beats();
                let start = if whole > 0.0 { (beat / whole).floor().max(0.0) * whole } else { 0.0 };
                self.at = advance(song, Cursor { start, ..Cursor::default() }, beat, None, f64::INFINITY).0;
            }
            if let Some(q) = self.cue.take() {
                self.at = Cursor { start: beat, ..take_cue(song, self.at, q) };
            }
        } else {
            (self.at, self.cue) = advance(song, self.at, beat, self.cue, on_bar);
        }
        self.sig = sig;
        self.last = beat;
        // A cue still waiting for its bar ends this part there.
        let on_bar = if self.cue.is_some() { on_bar } else { f64::INFINITY };
        let part = part_of(song, &self.at);
        let next = match self.cue {
            Some(q) => take_cue(song, self.at, q),
            None => after(song, self.at),
        };
        let prog = song.prog(part);
        BandNow {
            on: true,
            parts: true,
            key,
            beat,
            prog,
            from: self.at.start,
            until: (self.at.start + prog.beats()).min(on_bar),
            next: song.prog(part_of(song, &next)),
            part: part as u8,
            step: if self.at.held.is_some() || song.form.is_empty() { NONE } else { self.at.step as u8 },
            pass: self.at.pass.min(255) as u8,
            cued: if self.cue.is_some() { part_of(song, &next) as u8 } else { NONE },
        }
    }
}

/// Without parts: the progression of the first chord player with Band on
/// (the others play the same), from the song's start.
fn one_progression(project: &Project, beat: f64) -> BandNow {
    let none = BandNow { key: project.scale, beat, ..BandNow::default() };
    for t in &project.tracks {
        for d in &t.midi_devices {
            if band_param(d.kind).and_then(|b| d.params.get(b)).is_some_and(|v| *v >= 0.5) {
                let prog = Prog::of(&d.params, &d.data);
                return BandNow { on: true, prog, next: prog, ..none };
            }
        }
    }
    none
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::MidiFxState;
    use crate::params::defaults;
    use crate::scale::ScaleKind;
    use crate::sequencers::players::{band_part, player_default, set_band_part};

    const SPB: f64 = 24000.0;

    fn part(name: &str, length: u8, chords: &[(u8, u8)]) -> BandPart {
        let mut c = [(0, 0); PROG_SLOTS];
        c[..chords.len()].copy_from_slice(chords);
        BandPart { name: name.into(), length, count: chords.len() as u8, chords: c }
    }

    /// Verse (I V, a bar each) and Chorus (IV, a bar), Verse twice, Chorus once.
    fn song() -> Project {
        let mut p = Project::blank();
        p.band.parts = vec![part("Verse", 2, &[(1, 0), (5, 0)]), part("Chorus", 2, &[(4, 0)])];
        p.band.form = vec![FormStep { part: 0, times: 2 }, FormStep { part: 1, times: 1 }];
        p
    }

    /// Runs `kind` (with the band resolved per block from `project`) for
    /// `beats`; returns (beat, pitch) of every note-on.
    fn run(project: &Project, kind: MidiFxKind, p: &[f32], data: &[u8], beats: f64) -> Vec<(f64, u8)> {
        let mut band = BandRt::default();
        let mut st = MidiFxState::new(kind);
        let mut ons = Vec::new();
        let mut now = 0u64;
        while (now as f64) < beats * SPB {
            let beat = now as f64 / SPB;
            let b = band.resolve(project, beat, true);
            let ctx = MidiCtx { now, len: 256, samples_per_beat: SPB, scale: project.scale, scale_on: true, beat, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 5], band: b };
            let mut out = Vec::new();
            st.process(kind, p, data, &[], &mut out, &ctx);
            for e in out.iter().filter(|e| e.on) {
                ons.push(((now + e.offset as u64) as f64 / SPB, e.pitch));
            }
            now += 256;
        }
        ons
    }

    /// A Step sequencer playing C4 on every quarter, following the band.
    fn steady_step(follow: u8) -> (Vec<f32>, Vec<u8>) {
        let kind = MidiFxKind::Step;
        let mut p = defaults(kind.specs());
        p[0] = 5.0; // 1/4
        p[3] = 0.0; // no swing
        p[4] = 60.0; // from C4
        let b = follow_param(kind).unwrap();
        p[b] = 1.0;
        p[b + 1] = follow as f32;
        let mut data = Vec::new();
        for _ in 0..16 {
            data.extend_from_slice(&[1, 0, 100, 3, 1]);
        }
        (p, data)
    }

    fn first_in_bar(ons: &[(f64, u8)], bar: usize) -> Option<u8> {
        ons.iter().find(|o| (o.0 - bar as f64 * 4.0).abs() < 0.01).map(|o| o.1)
    }

    #[test]
    fn root_moves_a_line_with_the_band() {
        // The band without parts: a Piano Player with Band on, I V vi IV a bar each.
        let mut project = Project::blank();
        let t = project.add_midi_track(None);
        let mut piano = defaults(MidiFxKind::PianoPlayer.specs());
        piano[band_param(MidiFxKind::PianoPlayer).unwrap()] = 1.0;
        project.tracks[t].midi_devices.push(crate::project::MidiDevice { id: 9001, kind: MidiFxKind::PianoPlayer, params: piano, data: player_default(), enabled: true, preset: String::new() });
        let (p, data) = steady_step(0);
        let ons = run(&project, MidiFxKind::Step, &p, &data, 16.0);
        // C, then down to G, down to A, up to F: each the nearer way.
        let firsts: Vec<Option<u8>> = (0..4).map(|b| first_in_bar(&ons, b)).collect();
        assert_eq!(firsts, vec![Some(60), Some(55), Some(57), Some(65)]);
        // Band off: it stays where it was written.
        let mut off = p.clone();
        off[follow_param(MidiFxKind::Step).unwrap()] = 0.0;
        let ons = run(&project, MidiFxKind::Step, &off, &data, 16.0);
        assert!(ons.iter().all(|o| o.1 == 60));
    }

    #[test]
    fn chord_lands_on_the_chords_notes() {
        let mut project = Project::blank();
        project.scale = Scale::new(0, ScaleKind::Major);
        project.band.parts = vec![part("Verse", 2, &[(2, 1), (5, 0), (6, 0), (4, 1)])];
        // A line through every note of the key, each a quarter.
        let kind = MidiFxKind::Step;
        let (mut p, _) = steady_step(1);
        p[1] = 7.0;
        let mut data = Vec::new();
        for d in 0..16u8 {
            data.extend_from_slice(&[1, d % 7, 100, 3, 1]);
        }
        let ons = run(&project, kind, &p, &data, 16.0);
        assert!(ons.len() >= 16);
        let tones = |deg: i32, q: u8| -> Vec<i32> { quality_steps(q).iter().map(|s| project.scale.pitch_at(deg - 1 + s).rem_euclid(12)).collect() };
        for (beat, pitch) in ons {
            let (d, q) = [(2, 1), (5, 0), (6, 0), (4, 1)][((beat / 4.0).floor() as usize) % 4];
            assert!(tones(d, q).contains(&(pitch as i32 % 12)), "beat {beat}: {pitch} not in {d}/{q}");
        }
    }

    #[test]
    fn bass_plays_roots_and_fifths() {
        let mut project = Project::blank();
        project.band.parts = vec![part("Verse", 2, &[(1, 0), (4, 0), (5, 0)])];
        let (p, data) = steady_step(2);
        let ons = run(&project, MidiFxKind::Step, &p, &data, 12.0);
        for (beat, pitch) in ons {
            let root = [0, 5, 7][(beat / 4.0).floor() as usize % 3];
            let pc = pitch as i32 % 12;
            assert!(pc == root || pc == (root + 7) % 12, "beat {beat}: {pitch}");
        }
    }

    #[test]
    fn parts_follow_the_form_with_repeats() {
        let project = song();
        let mut band = BandRt::default();
        // Verse is 2 bars: twice is 16 beats, then the Chorus a bar, then the Verse again.
        let at = |band: &mut BandRt, beat: f64| {
            let b = band.resolve(&project, beat, true);
            (b.part, b.pass, b.at(beat).map(|c| c.degree))
        };
        let mut seen = Vec::new();
        let mut beat = 0.0;
        while beat < 44.0 {
            seen.push((beat, at(&mut band, beat)));
            beat += 0.5;
        }
        let when = |b: f64| seen.iter().find(|s| s.0 == b).map(|s| s.1).unwrap();
        assert_eq!(when(0.0), (0, 0, Some(1)));
        assert_eq!(when(4.0), (0, 0, Some(5)));
        assert_eq!(when(8.0), (0, 1, Some(1)));
        assert_eq!(when(15.5), (0, 1, Some(5)));
        assert_eq!(when(16.0), (1, 0, Some(4)));
        assert_eq!(when(19.5), (1, 0, Some(4)));
        assert_eq!(when(20.0), (0, 0, Some(1)));
        assert_eq!(when(36.0), (1, 0, Some(4)));
        // Stopped at a song position, it shows what will play there.
        let mut stopped = BandRt::default();
        assert_eq!(stopped.resolve(&project, 17.0, false).part, 1);
        assert_eq!(stopped.resolve(&project, 41.0, false).part, 0, "round again");
    }

    #[test]
    fn a_cue_waits_for_the_end_of_the_part() {
        let project = song();
        let mut band = BandRt::default();
        band.resolve(&project, 0.0, true);
        band.resolve(&project, 2.0, true);
        band.cue(Some(Cue::Next));
        // Still the verse until its end (beat 8), the chorus from there.
        let b = band.resolve(&project, 3.0, true);
        assert_eq!((b.part, b.cued), (0, 1));
        assert_eq!(band.resolve(&project, 7.9, true).part, 0);
        let b = band.resolve(&project, 8.0, true);
        assert_eq!((b.part, b.cued, b.at(8.0).map(|c| c.degree)), (1, NONE, Some(4)));
        // Then the form goes on: the verse after the chorus.
        assert_eq!(band.resolve(&project, 12.0, true).part, 0);
        // On the bar: a part picked starts on the next bar.
        let mut project = song();
        project.band.cue_on_bar = true;
        let mut band = BandRt::default();
        band.resolve(&project, 0.0, true);
        band.resolve(&project, 1.0, true);
        band.cue(Some(Cue::Part(1)));
        assert_eq!(band.resolve(&project, 1.5, true).part, 0);
        assert_eq!(band.resolve(&project, 3.99, true).part, 0);
        let b = band.resolve(&project, 4.0, true);
        assert_eq!((b.part, b.at(4.0).map(|c| c.degree)), (1, Some(4)));
        // A part that is not in the form plays until the next cue.
        let mut project = song();
        project.band.parts.push(part("Bridge", 1, &[(6, 0)]));
        let mut band = BandRt::default();
        band.resolve(&project, 0.0, true);
        band.cue(Some(Cue::Part(2)));
        assert_eq!(band.resolve(&project, 8.0, true).part, 2);
        assert_eq!(band.resolve(&project, 30.0, true).part, 2);
        band.cue(Some(Cue::Next));
        assert_eq!(band.resolve(&project, 30.5, true).part, 2);
        assert_eq!(band.resolve(&project, 32.0, true).part, 1, "back to the form after the verse");
    }

    #[test]
    fn without_parts_the_band_is_as_before() {
        // An old song: no parts in its file, the chord players as they were.
        let p = Project::blank();
        assert!(p.band.is_empty());
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("\"band\""), "nothing saved for it");
        let back: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(back.band, BandSong::default());
        let b = BandRt::default().resolve(&p, 3.0, true);
        assert!(!b.on && !b.parts);
        // A Progression with Band on plays its own chords, as it did.
        let kind = MidiFxKind::Progression;
        let mut params = defaults(kind.specs());
        params[band_param(kind).unwrap()] = 1.0;
        let data = crate::classic_seq::progression_default();
        let mut solo = params.clone();
        solo[band_param(kind).unwrap()] = 0.0;
        let mut with = Project::blank();
        let t = with.add_midi_track(None);
        with.tracks[t].midi_devices.push(crate::project::MidiDevice { id: 9001, kind, params: params.clone(), data: data.clone(), enabled: true, preset: String::new() });
        assert_eq!(run(&with, kind, &params, &data, 16.0), run(&Project::blank(), kind, &solo, &data, 16.0));
    }

    #[test]
    fn chord_players_share_the_parts() {
        let project = song();
        // A Bass Player whose own chords are something else plays the band's.
        let kind = MidiFxKind::BassPlayer;
        let mut p = defaults(kind.specs());
        p[crate::sequencers::players::HUMANIZE] = 0.0;
        p[band_param(kind).unwrap()] = 1.0;
        let ons = run(&project, kind, &p, &player_default(), 24.0);
        // C (I), G (V), C, G, F (chorus), C: from C2.
        for (bar, want) in [36u8, 43, 36, 43, 41, 36].iter().enumerate() {
            assert_eq!(first_in_bar(&ons, bar), Some(*want), "bar {bar}");
        }
        // And what they share is still the same for all (the UI keeps them so).
        let mut a = defaults(MidiFxKind::PianoPlayer.specs());
        let mut a_data = player_default();
        set_band_part(&mut a, &mut a_data, &project.band.parts[1].part());
        assert_eq!(band_part(&a, &a_data), project.band.parts[1].part());
    }
}
