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
//! - Drums: the drum and rhythm sequencers can be in the band too; their
//!   notes stay drums, they play the parts the way the parts say.
//! - Arranging: each part says how the band plays it (`PartPlay`): how full
//!   (Energy), how hard (Dynamics), in which key, with which swing and feel,
//!   and how it ends (a fill, a build, a break); and who sits it out. The
//!   `Shaper` does it to each member's notes on their way out.
//!
//! The engine works out where the band is once a block (`BandRt`) and hands
//! it to every MIDI effect in `MidiCtx::band`, without allocating.

use crate::classic_seq::{prog_slot, quality_steps, PROG_SLOTS};
use crate::midi_fx::{MidiCtx, MidiEvent, MidiFxKind};
use crate::project::Project;
use crate::scale::{Scale, ScaleKind};
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

/// The drum and rhythm sequencers that can be in the band (a Band switch,
/// last; no Follow: their notes are drums).
pub const DRUMS: [MidiFxKind; 7] = {
    use MidiFxKind as K;
    [K::Drummer, K::HomeRhythms, K::DrumMap, K::Chance, K::Percussion, K::Bits, K::DrumGrid]
};

/// A part's feel: its steps as written, twice as long, or half as long.
pub const FEELS: &[&str] = &["Normal", "Half Time", "Double Time"];
/// A part's key: the song's, its parallel (C major to C minor) or its
/// relative (C major to A minor).
pub const MODES: &[&str] = &["Song's key", "Parallel", "Relative"];
/// A build over the part's last bars, and a break at its end.
pub const BUILDS: &[&str] = &["Off", "1 Bar", "2 Bars"];
pub const BREAKS: &[&str] = &["Off", "Last Beat", "Last Bar"];

/// How the band plays a part. At the defaults it plays as written.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PartPlay {
    /// How full (0..1): 0.5 as written, lower fewer notes, higher more
    /// (octaves on top, busier drums).
    pub energy: f32,
    /// Softer or harder, in dB (-12 to +6).
    pub dynamics: f32,
    /// Semitones up or down, and the key (see MODES).
    pub transpose: i8,
    pub mode: u8,
    /// Swing for everyone in it (0..1; 0: each their own).
    pub swing: f32,
    /// See FEELS.
    pub feel: u8,
    /// The drums fill the last bar.
    pub fill: bool,
    /// Louder and fuller over the last bars (see BUILDS), and silent at the
    /// end (see BREAKS).
    pub build: u8,
    #[serde(rename = "break")]
    pub brk: u8,
}

impl Default for PartPlay {
    fn default() -> Self {
        Self { energy: 0.5, dynamics: 0.0, transpose: 0, mode: 0, swing: 0.0, feel: 0, fill: false, build: 0, brk: 0 }
    }
}

impl PartPlay {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// The key it plays in before the transposition (the song's, or its
    /// parallel or relative major or minor).
    pub fn mode_key(&self, song: Scale) -> Scale {
        let major = song.intervals().contains(&4);
        let pent = matches!(song.kind, ScaleKind::MajorPentatonic | ScaleKind::MinorPentatonic);
        let other = match (major, pent) {
            (true, true) => ScaleKind::MinorPentatonic,
            (false, true) => ScaleKind::MajorPentatonic,
            (true, false) => ScaleKind::Minor,
            (false, false) => ScaleKind::Major,
        };
        match self.mode {
            1 if song.kind != ScaleKind::Chromatic => Scale::new(song.root, other),
            2 if song.kind != ScaleKind::Chromatic => Scale::new(song.root + if major { 9 } else { 3 }, other),
            _ => song,
        }
    }

    /// The key it sounds in (for the chords' names).
    pub fn key(&self, song: Scale) -> Scale {
        let k = self.mode_key(song);
        Scale::new((k.root as i32 + self.transpose as i32).rem_euclid(12) as u8, k.kind)
    }

    /// Its dynamics as a factor on each note's velocity (-12 dB: half as hard).
    pub fn gain(&self) -> f32 {
        10f32.powf(self.dynamics.clamp(-24.0, 12.0) / 40.0)
    }

    /// How many beats its build lasts and its break (0: none).
    pub fn build_beats(&self, bar: f64) -> f64 {
        bar * self.build.min(2) as f64
    }

    pub fn break_beats(&self, bar: f64) -> f64 {
        [0.0, 1.0, bar][self.brk.min(2) as usize]
    }

    /// Its steps' length against the written one (Half Time: twice as long).
    pub fn feel_factor(&self) -> f64 {
        [1.0, 2.0, 0.5][self.feel.min(2) as usize]
    }
}

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
    /// How the band plays it.
    #[serde(default, skip_serializing_if = "PartPlay::is_default")]
    pub play: PartPlay,
    /// The members who sit it out (their devices' ids).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub out: Vec<u64>,
}

impl BandPart {
    /// A part from what the band shares (see `players::band_part`).
    pub fn from_part(name: &str, part: &[u8]) -> Self {
        let mut p = Self { name: name.to_string(), length: 2, count: 4, chords: [(0, 0); PROG_SLOTS], play: PartPlay::default(), out: Vec::new() };
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

    fn play(&self, part: usize) -> PartPlay {
        self.parts.get(part).map(|p| p.play).unwrap_or_default()
    }

    /// Whether member `id` (a device) sits part `part` out.
    pub fn sits_out(&self, part: u8, id: u64) -> bool {
        self.parts.get(part as usize).is_some_and(|p| p.out.contains(&id))
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
    /// How the band plays the part now and the next one, and which the next is.
    pub play: PartPlay,
    pub next_play: PartPlay,
    pub next_part: u8,
    /// This time round is the part's last before another (or a cue): its
    /// end fills, builds or breaks.
    pub ends: bool,
    /// Beats in a bar.
    pub bar: f64,
    /// For one member (the engine sets it per device): it sits out the part
    /// now, the next one.
    pub sits_out: bool,
    pub sits_out_next: bool,
}

impl Default for BandNow {
    fn default() -> Self {
        Self {
            on: false,
            parts: false,
            key: Scale::default(),
            beat: 0.0,
            prog: Prog::default(),
            from: 0.0,
            until: f64::INFINITY,
            next: Prog::default(),
            part: NONE,
            step: NONE,
            pass: 0,
            cued: NONE,
            play: PartPlay::default(),
            next_play: PartPlay::default(),
            next_part: NONE,
            ends: false,
            bar: 4.0,
            sits_out: false,
            sits_out_next: false,
        }
    }
}

/// Where a beat is in its part (see `BandNow::place`).
#[derive(Clone, Copy, Debug)]
pub struct Place {
    pub play: PartPlay,
    /// Beats into the part, and beats left of it.
    pub since: f64,
    pub left: f64,
    /// The member sits this part out.
    pub out: bool,
    /// Its end is the part's end (see `BandNow::ends`).
    pub ends: bool,
}

impl Place {
    /// How far into its build (0..1), in a build.
    pub fn build_at(&self, bar: f64) -> Option<f32> {
        let b = self.play.build_beats(bar);
        (self.ends && b > 0.0 && self.left > 1e-9 && self.left <= b + 1e-9).then(|| (1.0 - self.left / b).clamp(0.0, 1.0) as f32)
    }

    /// How full the band plays here: the part's energy, from low to high through a build.
    pub fn energy(&self, bar: f64) -> f32 {
        let e = self.play.energy.clamp(0.0, 1.0);
        match self.build_at(bar) {
            Some(f) => e.min(0.3) + (e.max(0.9) - e.min(0.3)) * f,
            None => e,
        }
    }

    /// The factor on a note's velocity here: the part's dynamics, rising through a build.
    pub fn gain(&self, bar: f64) -> f32 {
        self.play.gain() * self.build_at(bar).map(|f| 0.6 + 0.5 * f).unwrap_or(1.0)
    }

    /// Silent here: the member sits the part out, or it is the part's break.
    pub fn silent(&self, bar: f64) -> bool {
        self.out || (self.ends && self.left > 1e-9 && self.left <= self.play.break_beats(bar) + 1e-9)
    }

    /// In the bar the drums fill (the part's last).
    pub fn fill(&self, bar: f64) -> bool {
        self.ends && self.play.fill && self.left > 1e-9 && self.left <= bar + 1e-9
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

    /// Where `beat` is in its part: the part now, or from its end the next.
    pub fn place(&self, beat: f64) -> Place {
        if beat >= self.until {
            let since = beat - self.until;
            Place { play: self.next_play, since, left: self.next.beats() - since, out: self.sits_out_next, ends: false }
        } else {
            Place { play: self.play, since: beat - self.from, left: self.until - beat, out: self.sits_out, ends: self.ends }
        }
    }

    /// For the UI: the part, step, time round, cue, the chord now and how
    /// far through the part, in one word.
    pub fn pack(&self) -> u64 {
        let c = self.at(self.beat + 1e-6);
        let (slot, degree, quality) = c.map(|c| (c.slot as u64, c.degree as u64, c.quality as u64)).unwrap_or((NONE as u64, 0, 0));
        let len = self.until - self.from;
        let through = if self.parts && len.is_finite() && len > 0.0 { ((self.beat - self.from) / len).clamp(0.0, 1.0) } else { 0.0 };
        self.part as u64 | (self.step as u64) << 8 | (self.pass as u64) << 16 | (self.cued as u64) << 24 | slot << 32 | (degree & 15) << 40 | (quality & 15) << 44 | (self.on as u64) << 48 | (self.parts as u64) << 49 | ((through * 255.0) as u64) << 50
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
    /// How far through the part playing (0..255).
    pub through: u8,
}

impl BandStatus {
    pub fn unpack(v: u64) -> Self {
        let b = |s: u32| ((v >> s) & 0xff) as u8;
        Self { on: v >> 48 & 1 == 1, parts: v >> 49 & 1 == 1, part: b(0), step: b(8), pass: b(16), cued: b(24), slot: b(32), degree: ((v >> 40) & 15) as u8, quality: ((v >> 44) & 15) as u8, through: b(50) }
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

/// Where a drum or rhythm sequencer keeps its Band switch (last).
pub fn drum_param(kind: MidiFxKind) -> Option<usize> {
    DRUMS.contains(&kind).then(|| kind.specs().iter().position(|s| s.name == "Band")).flatten()
}

/// Where any device that can be in the band keeps its Band switch.
pub fn switch_param(kind: MidiFxKind) -> Option<usize> {
    band_param(kind).or(follow_param(kind)).or(drum_param(kind))
}

/// What a member plays in the band as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// A chord player: the band's chords.
    Chords,
    /// A follower, and how it follows (see FOLLOWS).
    Follows(u8),
    /// Drums: its notes stay where they are.
    Drums,
}

/// What a device plays in the band as (None: it is not in it). A follower
/// on a drum kit plays drums.
pub fn role(kind: MidiFxKind, p: &[f32], kit: bool) -> Option<Role> {
    let on = |i: Option<usize>| i.and_then(|i| p.get(i)).is_some_and(|v| *v >= 0.5);
    if on(band_param(kind)) {
        Some(Role::Chords)
    } else if let Some(m) = follow_mode(kind, p) {
        Some(if kit { Role::Drums } else { Role::Follows(m) })
    } else {
        on(drum_param(kind)).then_some(Role::Drums)
    }
}

/// Most parameters a device has (see `part_params`).
pub const MAX_PARAMS: usize = 160;

/// A member's parameters as the part playing has them, into `buf`: the
/// knobs a drum sequencer has for how much it plays (Drummer's Energy and
/// Complexity, Drum Map's fills, Chance's chance, Percussion's Density, Home
/// Rhythms' variation) moved by the part's energy, its fill, and the part's
/// swing. False: as they are.
pub fn part_params(kind: MidiFxKind, role: Option<Role>, p: &[f32], ctx: &MidiCtx, buf: &mut [f32; MAX_PARAMS]) -> bool {
    use crate::sequencers::{chance, drum_map, drummer, home, percussion};
    use MidiFxKind as K;
    let b = &ctx.band;
    if role.is_none() || !b.parts || p.len() > MAX_PARAMS {
        return false;
    }
    let bar = b.bar.max(1.0);
    let place = b.place(b.beat + 1e-6);
    let e = place.energy(bar);
    let fill = place.fill(bar);
    let specs = kind.specs();
    let swing = place.play.swing.clamp(0.0, 1.0);
    let swing_at = (swing > 0.0).then(|| specs.iter().position(|s| s.name == "Swing")).flatten();
    if (e - 0.5).abs() < 1e-6 && !fill && swing_at.is_none() {
        return false;
    }
    let q = &mut buf[..p.len()];
    q.copy_from_slice(p);
    let lift = e - 0.5;
    let nudge = |q: &mut [f32], i: usize, by: f32| {
        if let Some(v) = q.get_mut(i) {
            *v = (*v + by).clamp(0.0, 1.0);
        }
    };
    match kind {
        K::Drummer => {
            nudge(q, drummer::COMPLEXITY, lift * 1.2);
            nudge(q, drummer::ENERGY, lift * 1.2);
        }
        K::DrumMap => {
            for i in drum_map::FILL..drum_map::FILL + 3 {
                nudge(q, i, lift * 0.8);
                if fill {
                    q[i] = q[i].max(0.9);
                }
            }
        }
        K::Chance => {
            q[chance::CHANCE] = (q[chance::CHANCE] * (0.4 + 1.2 * e)).clamp(0.0, 1.0);
            if fill {
                q[chance::FILL] = 1.0;
            }
        }
        K::Percussion => nudge(q, percussion::DENSITY, lift),
        K::HomeRhythms if e > 0.6 => q[home::VARIATION] = 1.0,
        K::HomeRhythms if e < 0.4 => q[home::VARIATION] = 0.0,
        _ => {}
    }
    if let Some(i) = swing_at {
        q[i] = specs[i].min + swing * (specs[i].max - specs[i].min);
    }
    true
}

/// A member's block in a part in another key (its parallel or relative):
/// the key it plays in and the band's chords count in. None: the song's.
pub fn part_ctx(role: Option<Role>, ctx: &MidiCtx) -> Option<MidiCtx> {
    let b = &ctx.band;
    if role.is_none() || !b.parts {
        return None;
    }
    let play = b.place(b.beat + 1e-6).play;
    if play.mode == 0 {
        return None;
    }
    let key = play.mode_key(b.key);
    // All twelve notes free stays free.
    let scale = if ctx.scale.kind == ScaleKind::Chromatic { ctx.scale } else { key };
    Some(MidiCtx { scale, band: BandNow { key, ..*b }, ..*ctx })
}

/// Whether the shaper thins a member's notes for a low energy itself (the
/// drums with a knob for how much they play do it with that).
fn thins(kind: MidiFxKind) -> bool {
    !matches!(kind, MidiFxKind::Drummer | MidiFxKind::DrumMap | MidiFxKind::Chance | MidiFxKind::Percussion)
}

/// A member's steps in the part's feel: how long a step is against the
/// written one, and the beat (the session clock's) from which it counts.
/// None: as written (a chord player keeps the band's chords in time).
pub fn feel(role: Option<Role>, ctx: &MidiCtx) -> Option<(f64, f64)> {
    let b = &ctx.band;
    if !b.parts || !matches!(role, Some(Role::Follows(_) | Role::Drums)) {
        return None;
    }
    let beat = b.beat + 1e-6;
    let (play, from) = if beat >= b.until { (b.next_play, b.until) } else { (b.play, b.from) };
    let f = play.feel_factor();
    (f != 1.0).then(|| (f, from - b.beat + ctx.beat))
}

/// How strong a place in the bar is (a note's beat): the bar's one, the
/// beats, the eighths, the rest, with a little of each note's own (so a low
/// energy leaves the strong ones and a few others, always the same).
fn strength(beat: f64, bar: f64, pitch: u8) -> f32 {
    let s = (beat * 4.0).round();
    let off_grid = (beat * 4.0 - s).abs() > 0.15;
    let s = s as i64;
    let per_bar = ((bar * 4.0).round() as i64).max(1);
    let place = if off_grid {
        0.25
    } else if s.rem_euclid(per_bar) == 0 {
        1.0
    } else if s.rem_euclid(4) == 0 {
        0.75
    } else if s.rem_euclid(2) == 0 {
        0.5
    } else {
        0.25
    };
    let h = (s as u64 ^ (pitch as u64) << 40).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    place * 0.75 + ((h >> 40) & 0xffff) as f32 / 65535.0 * 0.25
}

/// What a part does to a member's notes on their way out: silence where it
/// sits the part out or in the break, fewer notes for a low energy and
/// octaves on top for a high one, the dynamics and a build, and the
/// transposition. It remembers where each note went, so its note-off
/// follows it and nothing hangs.
#[derive(Clone, Debug)]
pub struct Shaper {
    /// Each note as it went out (+1; 0: it did not), and the ones doubled.
    sent: [u8; 128],
    doubled: u128,
    /// The notes it has sounding.
    sounding: u128,
}

impl Default for Shaper {
    fn default() -> Self {
        Self { sent: [0; 128], doubled: 0, sounding: 0 }
    }
}

impl Shaper {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Shapes the notes from `from` on (what the member plays; what came in
    /// before it passes as it is).
    pub fn shape(&mut self, kind: MidiFxKind, role: Option<Role>, ctx: &MidiCtx, out: &mut Vec<MidiEvent>, from: usize) {
        let b = &ctx.band;
        let shaping = role.is_some() && b.parts;
        if !shaping && self.sounding == 0 {
            return;
        }
        let drums = role == Some(Role::Drums);
        let bar = b.bar.max(1.0);
        let from = from.min(out.len());
        let n = out.len();
        // Silence comes (the part sat out, the break, the next part sat out): let go of what sounds.
        if shaping && self.sounding != 0 {
            if let Some(at) = self.quiet_at(ctx) {
                self.let_go(at, out);
            }
        }
        let mut w = from;
        for r in from..n {
            let e = out[r];
            let i = e.pitch as usize;
            if !e.on {
                match self.sent[i] {
                    // Its note-on did not go out, and another note sounds there.
                    0 if self.sounding >> i & 1 == 1 => {}
                    0 => {
                        out[w] = e;
                        w += 1;
                    }
                    s => {
                        out[w] = MidiEvent { pitch: s - 1, ..e };
                        w += 1;
                        self.release(i, e.offset, out);
                    }
                }
                continue;
            }
            if !shaping {
                out[w] = e;
                w += 1;
                continue;
            }
            let beat = beat_at(ctx, ctx.now + e.offset as u64);
            let place = b.place(beat);
            if place.silent(bar) {
                continue;
            }
            let energy = place.energy(bar);
            let strength = strength(beat, bar, e.pitch);
            if energy < 0.5 && thins(kind) && strength < (0.5 - energy) * 1.6 {
                continue;
            }
            let to = if drums { e.pitch as i32 } else { e.pitch as i32 + place.play.transpose as i32 }.clamp(0, 127) as u8;
            if self.sent[i] != 0 {
                // Played again before its note-off: the one sounding ends first.
                out.push(MidiEvent::off(e.offset, self.sent[i] - 1));
                self.release(i, e.offset, out);
            }
            let vel = (e.vel * place.gain(bar)).clamp(0.01, 1.0);
            out[w] = MidiEvent { pitch: to, vel, ..e };
            w += 1;
            self.sent[i] = to + 1;
            self.sounding |= 1u128 << to;
            // A high energy: the strong notes an octave up as well.
            if !drums && energy > 0.5 && to <= 115 && strength >= 1.0 - (energy - 0.5) * 1.2 {
                out.push(MidiEvent::on(e.offset, to + 12, vel * 0.8));
                self.doubled |= 1u128 << i;
                self.sounding |= 1u128 << (to + 12);
            }
        }
        out.drain(w..n);
    }

    /// Note `i` (as it came) has ended: its octave too.
    fn release(&mut self, i: usize, offset: u32, out: &mut Vec<MidiEvent>) {
        let to = self.sent[i].saturating_sub(1);
        self.sent[i] = 0;
        self.sounding &= !(1u128 << to);
        if self.doubled >> i & 1 == 1 {
            self.doubled &= !(1u128 << i);
            out.push(MidiEvent::off(offset, to + 12));
            self.sounding &= !(1u128 << (to + 12));
        }
    }

    /// Every note it has sounding ends at `offset`.
    fn let_go(&mut self, offset: u32, out: &mut Vec<MidiEvent>) {
        for i in 0..128 {
            if self.sent[i] != 0 {
                out.push(MidiEvent::off(offset, self.sent[i] - 1));
                self.release(i, offset, out);
            }
        }
        self.sounding = 0;
    }

    /// Where in this block the member goes silent (its frame offset): at once
    /// when it is silent now, where the break starts, or where a part it sits
    /// out begins.
    fn quiet_at(&self, ctx: &MidiCtx) -> Option<u32> {
        let b = &ctx.band;
        let bar = b.bar.max(1.0);
        let spb = ctx.samples_per_beat.max(1.0);
        let beat0 = beat_at(ctx, ctx.now);
        let beat1 = b.beat + ctx.len as f64 / spb;
        let offset = |beat: f64| ((beat - b.beat) * spb).clamp(0.0, ctx.len.saturating_sub(1) as f64) as u32;
        let now = b.place(beat0);
        if now.silent(bar) {
            return Some(0);
        }
        let brk = now.play.break_beats(bar);
        let starts = beat0 + now.left - brk;
        if brk > 0.0 && starts > beat0 && starts < beat1 {
            return Some(offset(starts));
        }
        let next = beat0 + now.left;
        (next < beat1 && b.place(next + 1e-6).silent(bar)).then(|| offset(next))
    }
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
            play: song.play(part),
            next_play: song.play(part_of(song, &next)),
            next_part: part_of(song, &next) as u8,
            ends: next.pass == 0,
            bar: project.beats_per_bar_f(),
            sits_out: false,
            sits_out_next: false,
        }
    }
}

/// Without parts: the progression of the first chord player with Band on
/// (the others play the same), from the song's start.
fn one_progression(project: &Project, beat: f64) -> BandNow {
    let none = BandNow { key: project.scale, beat, bar: project.beats_per_bar_f(), ..BandNow::default() };
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
        BandPart { name: name.into(), length, count: chords.len() as u8, chords: c, play: PartPlay::default(), out: Vec::new() }
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
        run_events(project, kind, p, data, beats, 0).into_iter().filter(|e| e.1).map(|e| (e.0, e.2)).collect()
    }

    /// The same as device `id` (who sits out which part, as the engine
    /// tells it): (beat, on, pitch, velocity) of every note.
    fn run_events(project: &Project, kind: MidiFxKind, p: &[f32], data: &[u8], beats: f64, id: u64) -> Vec<(f64, bool, u8, f32)> {
        let mut band = BandRt::default();
        let mut st = MidiFxState::new(kind);
        let mut evs = Vec::new();
        let mut now = 0u64;
        while (now as f64) < beats * SPB {
            let beat = now as f64 / SPB;
            let mut b = band.resolve(project, beat, true);
            b.sits_out = project.band.sits_out(b.part, id);
            b.sits_out_next = project.band.sits_out(b.next_part, id);
            let ctx = MidiCtx { now, len: 256, samples_per_beat: SPB, scale: project.scale, scale_on: true, beat, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 5], band: b };
            let mut out = Vec::new();
            st.process(kind, p, data, &[], &mut out, &ctx);
            out.sort_by_key(|e| (e.offset, e.on));
            for e in &out {
                evs.push(((now + e.offset as u64) as f64 / SPB, e.on, e.pitch, e.vel));
            }
            now += 256;
        }
        evs
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

    /// A Step sequencer in the band playing C4 on every sixteenth.
    fn busy_step() -> (Vec<f32>, Vec<u8>) {
        let (mut p, data) = steady_step(0);
        p[0] = 2.0; // 1/16
        (p, data)
    }

    /// One part, Verse (I V vi IV, a bar each), played as `play` says.
    fn one_part(play: PartPlay) -> Project {
        let mut p = Project::blank();
        p.band.parts = vec![part("Verse", 2, &[(1, 0), (5, 0), (6, 0), (4, 0)])];
        p.band.parts[0].play = play;
        p
    }

    fn ons_in(evs: &[(f64, bool, u8, f32)], from: f64, to: f64) -> Vec<(f64, bool, u8, f32)> {
        evs.iter().filter(|e| e.1 && e.0 >= from - 0.01 && e.0 < to - 0.01).copied().collect()
    }

    /// A Drummer in the band, without fills of its own.
    fn drummer() -> (MidiFxKind, Vec<f32>, Vec<u8>) {
        let kind = MidiFxKind::Drummer;
        let mut p = defaults(kind.specs());
        p[crate::sequencers::drummer::FILLS] = 0.0;
        p[drum_param(kind).unwrap()] = 1.0;
        (kind, p, crate::sequencers::drummer::drummer_default())
    }

    #[test]
    fn a_part_at_its_defaults_plays_as_written() {
        // The same chords as one progression (a Piano Player's) and as a part: the same notes.
        let mut piano = Project::blank();
        let t = piano.add_midi_track(None);
        let mut pp = defaults(MidiFxKind::PianoPlayer.specs());
        pp[band_param(MidiFxKind::PianoPlayer).unwrap()] = 1.0;
        piano.tracks[t].midi_devices.push(crate::project::MidiDevice { id: 9001, kind: MidiFxKind::PianoPlayer, params: pp, data: player_default(), enabled: true, preset: String::new() });
        let (p, data) = busy_step();
        let one = run_events(&piano, MidiFxKind::Step, &p, &data, 16.0, 0);
        assert_eq!(one.len(), 128);
        assert_eq!(one, run_events(&one_part(PartPlay::default()), MidiFxKind::Step, &p, &data, 16.0, 0));
    }

    #[test]
    fn energy_thins_and_fills_up() {
        let (p, data) = busy_step();
        let at = |energy: f32| run(&one_part(PartPlay { energy, ..PartPlay::default() }), MidiFxKind::Step, &p, &data, 16.0);
        let (low, mid, high) = (at(0.1), at(0.5), at(0.95));
        assert_eq!(mid.len(), 64);
        assert!(!low.is_empty() && low.len() < 32, "{}", low.len());
        // What stays is the strong places: every bar's one.
        for bar in 0..4 {
            assert!(low.iter().any(|o| (o.0 - bar as f64 * 4.0).abs() < 0.01), "bar {bar}");
        }
        assert!(low.iter().all(|o| (o.0 * 2.0 - (o.0 * 2.0).round()).abs() < 0.01), "no sixteenths left");
        // Full: every note, and the strong ones an octave up too.
        assert!(high.len() > 64);
        assert!(high.iter().any(|o| o.0 < 0.01 && o.1 == 72));
        // Always the same notes.
        assert_eq!(low, at(0.1));
    }

    #[test]
    fn dynamics_scale_the_velocity() {
        let (p, data) = steady_step(0);
        let vel = |dynamics: f32| run_events(&one_part(PartPlay { dynamics, ..PartPlay::default() }), MidiFxKind::Step, &p, &data, 8.0, 0).iter().find(|e| e.1).map(|e| e.3).unwrap();
        let (soft, flat, hard) = (vel(-12.0), vel(0.0), vel(6.0));
        assert!((soft / flat - 0.5).abs() < 0.01, "{soft} {flat}");
        assert!(hard > flat && hard <= 1.0);
    }

    #[test]
    fn a_part_in_another_key_moves_chords_and_followers() {
        // Up a half step: the follower and the bass player both.
        let (p, data) = steady_step(0);
        let up = one_part(PartPlay { transpose: 1, ..PartPlay::default() });
        let plain = run(&one_part(PartPlay::default()), MidiFxKind::Step, &p, &data, 16.0);
        let moved = run(&up, MidiFxKind::Step, &p, &data, 16.0);
        assert_eq!(plain.len(), moved.len());
        assert!(plain.iter().zip(&moved).all(|(a, b)| b.1 == a.1 + 1));
        let kind = MidiFxKind::BassPlayer;
        let mut bass = defaults(kind.specs());
        bass[crate::sequencers::players::HUMANIZE] = 0.0;
        bass[band_param(kind).unwrap()] = 1.0;
        let a = run(&one_part(PartPlay::default()), kind, &bass, &player_default(), 16.0);
        let b = run(&up, kind, &bass, &player_default(), 16.0);
        assert!(!a.is_empty() && a.len() == b.len() && a.iter().zip(&b).all(|(x, y)| y.1 == x.1 + 1));
        // Parallel: C major to C minor, the line's E an E flat.
        let (p, mut data) = steady_step(0);
        for s in data.chunks_mut(5) {
            s[1] = 2;
        }
        let first = |mode: u8| run(&one_part(PartPlay { mode, ..PartPlay::default() }), MidiFxKind::Step, &p, &data, 4.0)[0].1;
        assert_eq!((first(0), first(1)), (64, 63));
        // Relative: A minor, the same notes, but the chords count from A (I is A minor).
        assert_eq!(first(2), 64);
        let rel = run(&one_part(PartPlay { mode: 2, ..PartPlay::default() }), kind, &bass, &player_default(), 4.0);
        assert_eq!(first_in_bar(&rel, 0).map(|p| p % 12), Some(9));
        assert_eq!(PartPlay { mode: 2, transpose: 2, ..PartPlay::default() }.key(Scale::new(0, ScaleKind::Major)), Scale::new(11, ScaleKind::Minor));
    }

    #[test]
    fn a_member_that_sits_a_part_out_is_silent_there_and_comes_back() {
        // The Step (device 7) sits out the chorus (beats 16 to 20); its
        // notes last a beat and a half, so one still sounds as it comes.
        let mut project = song();
        project.band.parts[1].out = vec![7];
        let (p, mut data) = steady_step(0);
        for s in data.chunks_mut(5) {
            s[3] = 6;
        }
        let evs = run_events(&project, MidiFxKind::Step, &p, &data, 28.0, 7);
        assert!(!ons_in(&evs, 0.0, 16.0).is_empty());
        assert!(ons_in(&evs, 16.0, 20.0).is_empty());
        assert!(!ons_in(&evs, 20.0, 28.0).is_empty(), "back for the verse");
        // Nothing sounds once the chorus starts.
        let mut sounding = std::collections::BTreeSet::new();
        for e in evs.iter().filter(|e| e.0 < 16.0 + 1e-6) {
            if e.1 {
                sounding.insert(e.2);
            } else {
                sounding.remove(&e.2);
            }
        }
        assert!(sounding.is_empty(), "{sounding:?}");
        // Another member plays it.
        assert!(!ons_in(&run_events(&project, MidiFxKind::Step, &p, &data, 28.0, 8), 16.0, 20.0).is_empty());
    }

    #[test]
    fn breaks_and_builds_come_at_the_end_of_the_part() {
        // Verse twice (8 beats each), then the chorus: the verse breaks on
        // its last beat, the second time round only; the chorus builds.
        let mut project = song();
        project.band.parts[0].play.brk = 1;
        project.band.parts[1].play.build = 1;
        let (p, data) = steady_step(0);
        let evs = run_events(&project, MidiFxKind::Step, &p, &data, 24.0, 0);
        let on_at = |b: f64| evs.iter().find(|e| e.1 && (e.0 - b).abs() < 0.01).map(|e| e.3);
        assert!(on_at(7.0).is_some(), "the first time round goes on");
        assert!(on_at(15.0).is_none(), "the break");
        assert!(on_at(14.0).is_some() && on_at(16.0).is_some());
        // Softer where the build starts, louder towards its end.
        let (a, z) = (on_at(16.0).unwrap(), on_at(19.0).unwrap());
        assert!(a < z && a < on_at(14.0).unwrap(), "{a} {z}");
        // A break of the last bar.
        project.band.parts[0].play.brk = 2;
        let evs = run_events(&project, MidiFxKind::Step, &p, &data, 24.0, 0);
        assert!(ons_in(&evs, 12.0, 16.0).is_empty() && !ons_in(&evs, 4.0, 8.0).is_empty());
    }

    #[test]
    fn the_drummer_fills_the_parts_last_bar() {
        let (kind, p, data) = drummer();
        let mut project = song();
        let plain = run_events(&project, kind, &p, &data, 24.0, 0);
        project.band.parts[0].play.fill = true;
        let filled = run_events(&project, kind, &p, &data, 24.0, 0);
        // The same up to the verse's last bar the second time round (beats
        // 12 to 16), a fill there, and a crash on the chorus's one.
        assert_eq!(ons_in(&plain, 0.0, 12.0), ons_in(&filled, 0.0, 12.0));
        assert_ne!(ons_in(&plain, 12.0, 16.0), ons_in(&filled, 12.0, 16.0));
        assert!(ons_in(&filled, 16.0, 16.5).iter().any(|e| e.2 == 49), "a crash after it");
        assert!(!ons_in(&plain, 16.0, 16.5).iter().any(|e| e.2 == 49));
        // Home Rhythms fills it too.
        let kind = MidiFxKind::HomeRhythms;
        let mut hp = defaults(kind.specs());
        hp[crate::sequencers::home::FILL_EVERY] = 0.0;
        hp[drum_param(kind).unwrap()] = 1.0;
        let mut project = song();
        let plain = run_events(&project, kind, &hp, &[], 24.0, 0);
        project.band.parts[0].play.fill = true;
        let filled = run_events(&project, kind, &hp, &[], 24.0, 0);
        assert_eq!(ons_in(&plain, 0.0, 12.0), ons_in(&filled, 0.0, 12.0));
        assert_ne!(ons_in(&plain, 12.0, 16.0), ons_in(&filled, 12.0, 16.0));
    }

    #[test]
    fn drums_in_the_band_follow_energy_and_who_plays() {
        let (kind, p, data) = drummer();
        let hits = |energy: f32| {
            let evs = run_events(&one_part(PartPlay { energy, ..PartPlay::default() }), kind, &p, &data, 16.0, 0);
            let ons = ons_in(&evs, 0.0, 16.0);
            (ons.len(), ons.iter().map(|e| e.3).sum::<f32>())
        };
        let (lo, mid, hi) = (hits(0.0), hits(0.5), hits(1.0));
        assert!(lo.1 < mid.1 && mid.1 < hi.1, "{lo:?} {mid:?} {hi:?}");
        assert!(lo.0 <= mid.0 && mid.0 < hi.0, "{lo:?} {mid:?} {hi:?}");
        // A drum grid has no knob for it: fewer hits when low, none added when high.
        let grid = MidiFxKind::DrumGrid;
        let mut gp = defaults(grid.specs());
        gp[drum_param(grid).unwrap()] = 1.0;
        let gd = crate::classic_seq::drum_default();
        let n = |energy: f32| run(&one_part(PartPlay { energy, ..PartPlay::default() }), grid, &gp, &gd, 16.0).len();
        assert!(n(0.1) < n(0.5) && n(0.9) == n(0.5));
        // Sitting the part out: nothing.
        let mut project = one_part(PartPlay::default());
        project.band.parts[0].out = vec![3];
        assert!(run_events(&project, kind, &p, &data, 16.0, 3).iter().all(|e| !e.1));
        // With Band off it plays as it always did, parts or not.
        let mut off = p.clone();
        off[drum_param(kind).unwrap()] = 0.0;
        assert_eq!(run(&one_part(PartPlay { energy: 0.0, fill: true, ..PartPlay::default() }), kind, &off, &data, 16.0), run(&Project::blank(), kind, &off, &data, 16.0));
    }

    #[test]
    fn half_time_and_double_time() {
        // Quarters: half as many steps in a part in half time, twice as many in double.
        let (p, data) = steady_step(0);
        let n = |feel: u8| run(&one_part(PartPlay { feel, ..PartPlay::default() }), MidiFxKind::Step, &p, &data, 16.0).len();
        assert_eq!((n(0), n(1), n(2)), (16, 8, 32));
        // Swing for everyone: the Step's off-beat sixteenths come later.
        let (p, data) = busy_step();
        let straight = run(&one_part(PartPlay::default()), MidiFxKind::Step, &p, &data, 4.0);
        let swung = run(&one_part(PartPlay { swing: 0.5, ..PartPlay::default() }), MidiFxKind::Step, &p, &data, 4.0);
        assert_eq!(straight[0], swung[0]);
        assert!(swung[1].0 > straight[1].0 + 0.01);
    }

    #[test]
    fn parts_saved_before_load_as_they_were() {
        // A part saved before it had settings: as written, everyone in it.
        let json = r#"{"name":"Verse","length":2,"count":4,"chords":[[1,0],[5,0],[6,0],[4,0],[0,0],[0,0],[0,0],[0,0]]}"#;
        let p: BandPart = serde_json::from_str(json).unwrap();
        assert!(p.play.is_default() && p.out.is_empty());
        assert_eq!(serde_json::to_string(&p).unwrap(), json);
        // A drum sequencer of before: its new Band switch off.
        for kind in DRUMS {
            let mut params = defaults(kind.specs());
            params.pop();
            crate::params::fix_len(&mut params, kind.specs());
            assert_eq!(params[drum_param(kind).unwrap()], 0.0, "{}", kind.name());
            assert_eq!(drum_param(kind), Some(kind.specs().len() - 1));
        }
    }
}
