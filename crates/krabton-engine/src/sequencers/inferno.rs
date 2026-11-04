//! Inferno: a big bass sequencer for synths, for more than acid.
//!
//! - Eight patterns (A to H) of up to 64 steps, played one at a time, in a
//!   chain, or one per song part when it is in the band. Each pattern has
//!   its length and four lanes that can run at lengths of their own against
//!   it (accents, slides, octaves, wob), so they drift as a polymeter does.
//! - Every step: on, a note (a degree of the song's key, or a semitone), an
//!   octave (-2 to +2), velocity, accent, slide, tie, gate, ratchets (one to
//!   eight, evenly, speeding up, slowing down, fading or swelling), a chance,
//!   a condition (as the drum machines have them: 1:2, fill, first...), a
//!   nudge earlier or later, a wob (a rate the mod wheel wobbles at while it
//!   sounds, and how far), a glide time, an octave jump and a fifth on top,
//!   a lock (Evolve leaves it be) and a mute.
//! - It plays one note at a time, as a bass does: a slide keeps the note
//!   down until the next one starts, a little after it (a mono synth with
//!   glide glides), or with Glide on Bend the next note starts with the
//!   pitch bent from the last and bends home. Nothing is left hanging: it
//!   keeps its own queue of notes, a count of what sounds, and lets go of
//!   all of it when the transport stops.
//! - Ignite writes a pattern in a genre (twenty of them, acid to amapiano):
//!   how dense, how busy, how melodic, on the kick or off it, on the band's
//!   chords. Mutate changes a few things; Evolve does it by itself every few
//!   bars (not the locked steps); Reverse, Rotate, Invert and Humanize.
//! - Burn (held) and Fill play hotter: more ratchets, accents and octaves,
//!   a fill into the next bar. In the band the part's energy, its build, its
//!   fill and its break do the same, the way a bass player would.

use super::chance::{condition_holds, CONDITIONS};
use super::SeqState;
use crate::band::MAX_PARTS;
use crate::dsp::Rng;
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit, SCREEN_ONLY};
use crate::presets::{pd, Preset, V};
use crate::scale::Scale;

// ---------------------------------------------------------------- specs

pub const DIVISION: usize = 0;
pub const DIRECTION: usize = 1;
pub const SWING: usize = 2;
pub const GROOVE: usize = 3;
pub const PATTERN: usize = 4;
pub const CHAIN: usize = 5;
pub const BASE: usize = 6;
pub const NOTES: usize = 7;
pub const GATE: usize = 8;
pub const ACCENT: usize = 9;
pub const GLIDE: usize = 10;
pub const GLIDE_TIME: usize = 11;
pub const WOB_OUT: usize = 12;
pub const WOB_SHAPE: usize = 13;
pub const WOB_DEPTH: usize = 14;
pub const GENRE: usize = 15;
pub const DENSITY: usize = 16;
pub const COMPLEXITY: usize = 17;
pub const MELODY: usize = 18;
pub const EVOLVE: usize = 19;
pub const EVOLVE_AMOUNT: usize = 20;
pub const SEED: usize = 21;
pub const KICK: usize = 22;
pub const BURN: usize = 23;
pub const FILL: usize = 24;
pub const FLAMES: usize = 25;
pub const BAND: usize = 32;

pub const DIVISIONS: &[&str] = &["1/8", "1/8 T", "1/16", "1/16 T", "1/32", "1/32 T"];
const DIVISION_BEATS: [f64; 6] = [0.5, 1.0 / 3.0, 0.25, 1.0 / 6.0, 0.125, 1.0 / 12.0];
pub const DIRECTIONS: &[&str] = &["Forward", "Backward", "Ping-Pong", "Random", "Brownian"];
pub const GROOVES: &[&str] = &["Swing", "MPC 54", "MPC 58", "MPC 62", "Triplet", "2-Step", "Dilla", "Push", "Drag", "Dembow"];
/// Each groove: how far each of eight steps comes later (a part of a step,
/// with Swing all the way up; half way it is the groove as it is named).
const GROOVE_SHAPES: [[f32; 8]; 10] = [
    [0.0, 0.5, 0.0, 0.5, 0.0, 0.5, 0.0, 0.5],
    [0.0, 0.16, 0.0, 0.16, 0.0, 0.16, 0.0, 0.16],
    [0.0, 0.32, 0.0, 0.32, 0.0, 0.32, 0.0, 0.32],
    [0.0, 0.48, 0.0, 0.48, 0.0, 0.48, 0.0, 0.48],
    [0.0, 0.667, 0.0, 0.667, 0.0, 0.667, 0.0, 0.667],
    [0.0, 0.6, 0.0, 0.45, 0.0, 0.6, 0.0, 0.3],
    [0.0, 0.35, -0.08, 0.45, 0.05, 0.3, -0.05, 0.5],
    [0.0, -0.3, -0.1, -0.25, 0.0, -0.3, -0.1, -0.2],
    [0.0, 0.1, 0.15, 0.2, 0.0, 0.1, 0.15, 0.25],
    [0.0, 0.0, 0.0, 0.3, 0.0, 0.0, 0.2, 0.0],
];
pub const LETTERS: [&str; PATTERNS] = ["A", "B", "C", "D", "E", "F", "G", "H"];
pub const NOTE_MODES: &[&str] = &["In Key", "Chromatic"];
pub const GLIDES: &[&str] = &["Legato", "Bend"];
pub const WOB_OUTS: &[&str] = &["Off", "Mod Wheel"];
pub const WOB_SHAPES: &[&str] = &["Sine", "Triangle", "Saw Down", "Saw Up", "Square"];
pub const EVOLVES: &[&str] = &["Off", "1 Bar", "2 Bars", "4 Bars", "8 Bars", "16 Bars"];
const EVOLVE_BARS: [i64; 6] = [0, 1, 2, 4, 8, 16];
pub const KICKS: &[&str] = &["Off", "Avoid", "Lock"];
pub const FLAME_MODES: &[&str] = &["Full", "Calm", "Off"];
/// A step's wob: the rate the mod wheel wobbles at while it sounds.
pub const WOBS: &[&str] = &["Off", "1 Bar", "1/2", "1/4", "1/4 T", "1/8", "1/8 T", "1/16", "1/16 T", "1/32"];
const WOB_BEATS: [f64; 10] = [0.0, 4.0, 2.0, 1.0, 2.0 / 3.0, 0.5, 1.0 / 3.0, 0.25, 1.0 / 6.0, 0.125];
pub const CURVES: &[&str] = &["Even", "Speed Up", "Slow Down", "Fade", "Swell"];
pub const GENRES: &[&str] = &[
    "Acid",
    "Techno",
    "House",
    "UK Garage",
    "Dubstep",
    "Riddim",
    "Drum & Bass",
    "Reese",
    "Trap",
    "Drill",
    "Pluggnb",
    "Synthwave",
    "Funk",
    "Disco",
    "Reggaeton",
    "Psytrance",
    "EBM",
    "Hip-Hop",
    "Afrobeats",
    "Amapiano",
];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Division", "Clock", DIVISIONS, 2),
    ParamSpec::choice("Direction", "Clock", DIRECTIONS, 0),
    ParamSpec::lin("Swing", "Clock", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Groove", "Clock", GROOVES, 0),
    ParamSpec::choice("Pattern", "Pattern", &LETTERS, 0),
    ParamSpec::toggle("Chain", "Pattern", false),
    ParamSpec::int("Base Note", "Notes", 24.0, 72.0, 36.0, Unit::None),
    ParamSpec::choice("Notes", "Notes", NOTE_MODES, 0),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.5, 1.0, Unit::Percent),
    ParamSpec::lin("Accent", "Notes", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::choice("Glide", "Glide", GLIDES, 0),
    ParamSpec::lin("Glide Time", "Glide", 0.05, 2.0, 0.5, Unit::None),
    ParamSpec::choice("Wob Out", "Wob", WOB_OUTS, 1),
    ParamSpec::choice("Wob Shape", "Wob", WOB_SHAPES, 0),
    ParamSpec::lin("Wob Depth", "Wob", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::choice("Genre", "Ignite", GENRES, 0),
    ParamSpec::lin("Density", "Ignite", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Complexity", "Ignite", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Melody", "Ignite", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::choice("Evolve", "Evolve", EVOLVES, 0),
    ParamSpec::lin("Amount", "Evolve", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::int("Seed", "Ignite", 0.0, 9999.0, 1.0, Unit::None),
    ParamSpec::choice("Kick", "Ignite", KICKS, 0),
    ParamSpec::toggle("Burn", SCREEN_ONLY, false),
    ParamSpec::toggle("Fill", SCREEN_ONLY, false),
    ParamSpec::choice("Flames", SCREEN_ONLY, FLAME_MODES, 0),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Band", "Band", false),
    ParamSpec::choice("Follow", "Band", crate::band::FOLLOWS, 0),
];

/// How long a step is (beats).
pub fn step_beats(p: &[f32]) -> f64 {
    DIVISION_BEATS[(p.get(DIVISION).copied().unwrap_or(2.0).round().max(0.0) as usize).min(DIVISION_BEATS.len() - 1)]
}

fn choice(p: &[f32], i: usize, n: usize) -> usize {
    (p.get(i).copied().unwrap_or(0.0).round().max(0.0) as usize).min(n.saturating_sub(1))
}

fn on(p: &[f32], i: usize) -> bool {
    p.get(i).is_some_and(|v| *v >= 0.5)
}

// ---------------------------------------------------------------- data

pub const PATTERNS: usize = 8;
pub const STEPS: usize = 64;
/// Steps on a page of the grid.
pub const PAGE: usize = 16;
pub const STEP_BYTES: usize = 12;
pub const NOTE_MAX: u8 = 24;
pub const CHAIN_MAX: usize = 8;
/// The lanes that can run at their own length (the first is the pattern's).
pub const LANES: [&str; 5] = ["Notes", "Accent", "Slide", "Octave", "Wob"];
const HEAD_BYTES: usize = 8;
const MAGIC: [u8; 2] = *b"F1";
const CHAIN_AT: usize = 2;
const PARTS_AT: usize = CHAIN_AT + 1 + CHAIN_MAX;
const HEADS_AT: usize = PARTS_AT + MAX_PARTS;
const STEPS_AT: usize = HEADS_AT + PATTERNS * HEAD_BYTES;
pub const DATA_BYTES: usize = STEPS_AT + PATTERNS * STEPS * STEP_BYTES;
/// No pattern for a part (it plays the one picked).
pub const NO_PATTERN: u8 = 0xff;

/// One step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub on: bool,
    pub accent: bool,
    /// Slides into the next step (its note stays down until that one starts).
    pub slide: bool,
    /// Holds the note before on through this step (no new attack).
    pub tie: bool,
    /// An octave up, and a fifth up, on top of its note.
    pub jump: bool,
    pub fifth: bool,
    /// Evolve and Mutate leave it as it is.
    pub lock: bool,
    /// Silent, but kept.
    pub mute: bool,
    /// A degree of the key above the base note (or a semitone: Chromatic).
    pub note: u8,
    pub octave: i8,
    pub velocity: u8,
    /// How long, in eighths of a step (1..16).
    pub gate: u8,
    /// 1..8 hits, and how they go (see CURVES).
    pub ratchet: u8,
    pub curve: u8,
    /// 0..100 %.
    pub chance: u8,
    /// See `chance::CONDITIONS`.
    pub condition: u8,
    /// Earlier or later, in percent of a step (-45..45).
    pub nudge: i8,
    /// See WOBS, and how far it wobbles (0: all the way).
    pub wob: u8,
    pub modv: u8,
    /// How long a Bend glide into the next note takes (percent of Glide Time).
    pub glide: u8,
}

impl Step {
    pub const REST: Step = Step {
        on: false,
        accent: false,
        slide: false,
        tie: false,
        jump: false,
        fifth: false,
        lock: false,
        mute: false,
        note: 0,
        octave: 0,
        velocity: 100,
        gate: 4,
        ratchet: 1,
        curve: 0,
        chance: 100,
        condition: 0,
        nudge: 0,
        wob: 0,
        modv: 0,
        glide: 100,
    };

    pub const fn note(note: u8) -> Step {
        Step { on: true, note, ..Step::REST }
    }

    fn to_bytes(self) -> [u8; STEP_BYTES] {
        let flags = self.on as u8 | (self.accent as u8) << 1 | (self.slide as u8) << 2 | (self.tie as u8) << 3 | (self.jump as u8) << 4 | (self.fifth as u8) << 5 | (self.lock as u8) << 6 | (self.mute as u8) << 7;
        [
            flags,
            self.note.min(NOTE_MAX),
            (self.octave.clamp(-2, 2) + 2) as u8,
            self.velocity.clamp(1, 127),
            self.gate.clamp(1, 16),
            self.ratchet.clamp(1, 8) | self.curve.min(CURVES.len() as u8 - 1) << 4,
            self.chance.min(100),
            self.condition.min(CONDITIONS.len() as u8 - 1),
            self.nudge.clamp(-45, 45) as u8,
            self.wob.min(WOBS.len() as u8 - 1),
            self.modv.min(127),
            self.glide.min(200),
        ]
    }

    fn from_bytes(b: &[u8]) -> Step {
        let f = b[0];
        Step {
            on: f & 1 != 0,
            accent: f & 2 != 0,
            slide: f & 4 != 0,
            tie: f & 8 != 0,
            jump: f & 16 != 0,
            fifth: f & 32 != 0,
            lock: f & 64 != 0,
            mute: f & 128 != 0,
            note: b[1].min(NOTE_MAX),
            octave: (b[2].min(4) as i8) - 2,
            velocity: b[3].clamp(1, 127),
            gate: b[4].clamp(1, 16),
            ratchet: (b[5] & 15).clamp(1, 8),
            curve: (b[5] >> 4).min(CURVES.len() as u8 - 1),
            chance: b[6].min(100),
            condition: b[7].min(CONDITIONS.len() as u8 - 1),
            nudge: (b[8] as i8).clamp(-45, 45),
            wob: b[9].min(WOBS.len() as u8 - 1),
            modv: b[10].min(127),
            glide: b[11].min(200),
        }
    }

    /// Sounds (on and not muted).
    pub fn sounds(&self) -> bool {
        self.on && !self.mute
    }
}

pub fn is_current(data: &[u8]) -> bool {
    data.len() >= DATA_BYTES && data[..2] == MAGIC
}

/// Eight empty patterns of sixteen steps.
pub fn empty() -> Vec<u8> {
    let mut d = vec![0u8; DATA_BYTES];
    d[..2].copy_from_slice(&MAGIC);
    for b in &mut d[PARTS_AT..PARTS_AT + MAX_PARTS] {
        *b = NO_PATTERN;
    }
    for pat in 0..PATTERNS {
        d[HEADS_AT + pat * HEAD_BYTES] = PAGE as u8;
        for i in 0..STEPS {
            let o = step_at(pat, i);
            d[o..o + STEP_BYTES].copy_from_slice(&Step::REST.to_bytes());
        }
    }
    d
}

fn step_at(pat: usize, i: usize) -> usize {
    STEPS_AT + (pat.min(PATTERNS - 1) * STEPS + i % STEPS) * STEP_BYTES
}

fn fix(data: &mut Vec<u8>) {
    if !is_current(data) {
        *data = empty();
    }
}

pub fn step(data: &[u8], pat: usize, i: usize) -> Step {
    if !is_current(data) {
        return Step::REST;
    }
    let o = step_at(pat, i);
    Step::from_bytes(&data[o..o + STEP_BYTES])
}

pub fn set_step(data: &mut Vec<u8>, pat: usize, i: usize, s: Step) {
    fix(data);
    let o = step_at(pat, i);
    data[o..o + STEP_BYTES].copy_from_slice(&s.to_bytes());
}

pub fn pattern(data: &[u8], pat: usize) -> [Step; STEPS] {
    let mut v = [Step::REST; STEPS];
    for (i, s) in v.iter_mut().enumerate() {
        *s = step(data, pat, i);
    }
    v
}

pub fn set_pattern(data: &mut Vec<u8>, pat: usize, steps: &[Step; STEPS]) {
    for (i, s) in steps.iter().enumerate() {
        set_step(data, pat, i, *s);
    }
}

/// A pattern's bytes (to copy it, or to see whether it changed).
fn pattern_bytes(data: &[u8], pat: usize) -> &[u8] {
    if !is_current(data) {
        return &[];
    }
    let o = step_at(pat, 0);
    &data[o..o + STEPS * STEP_BYTES]
}

/// How many steps a pattern plays (1..64).
pub fn length(data: &[u8], pat: usize) -> usize {
    if is_current(data) {
        (data[HEADS_AT + pat.min(PATTERNS - 1) * HEAD_BYTES] as usize).clamp(1, STEPS)
    } else {
        PAGE
    }
}

pub fn set_length(data: &mut Vec<u8>, pat: usize, n: usize) {
    fix(data);
    data[HEADS_AT + pat.min(PATTERNS - 1) * HEAD_BYTES] = n.clamp(1, STEPS) as u8;
}

/// How long one of a pattern's lanes runs (see LANES; 0: the pattern's length).
pub fn lane_length(data: &[u8], pat: usize, lane: usize) -> usize {
    let len = length(data, pat);
    if lane == 0 || lane >= LANES.len() || !is_current(data) {
        return len;
    }
    match data[HEADS_AT + pat.min(PATTERNS - 1) * HEAD_BYTES + lane] as usize {
        0 => len,
        n => n.clamp(1, STEPS),
    }
}

pub fn set_lane_length(data: &mut Vec<u8>, pat: usize, lane: usize, n: usize) {
    fix(data);
    if lane == 0 {
        set_length(data, pat, n);
    } else if lane < LANES.len() {
        data[HEADS_AT + pat.min(PATTERNS - 1) * HEAD_BYTES + lane] = n.min(STEPS) as u8;
    }
}

/// The chain: patterns played one after another, once each.
pub fn chain(data: &[u8]) -> ([u8; CHAIN_MAX], usize) {
    let mut c = [0u8; CHAIN_MAX];
    if !is_current(data) {
        return (c, 0);
    }
    let n = (data[CHAIN_AT] as usize).min(CHAIN_MAX);
    for (i, v) in c.iter_mut().enumerate() {
        *v = data[CHAIN_AT + 1 + i].min(PATTERNS as u8 - 1);
    }
    (c, n)
}

pub fn set_chain(data: &mut Vec<u8>, patterns: &[u8]) {
    fix(data);
    let n = patterns.len().min(CHAIN_MAX);
    data[CHAIN_AT] = n as u8;
    for i in 0..CHAIN_MAX {
        data[CHAIN_AT + 1 + i] = patterns.get(i).copied().unwrap_or(0).min(PATTERNS as u8 - 1);
    }
}

/// The pattern a song part plays (None: the one picked).
pub fn part_pattern(data: &[u8], part: usize) -> Option<usize> {
    if !is_current(data) || part >= MAX_PARTS {
        return None;
    }
    let v = data[PARTS_AT + part];
    (v != NO_PATTERN).then_some((v as usize).min(PATTERNS - 1))
}

pub fn set_part_pattern(data: &mut Vec<u8>, part: usize, pat: Option<usize>) {
    fix(data);
    if part < MAX_PARTS {
        data[PARTS_AT + part] = pat.map(|p| p.min(PATTERNS - 1) as u8).unwrap_or(NO_PATTERN);
    }
}

pub fn inferno_default() -> Vec<u8> {
    preset_data(0, 0x1F1A, 0.7, 0.45, 0.4)
}

/// The pitch a step plays: its note above `base` (in the key's degrees or in
/// semitones), its octave, the octave jump and the fifth.
pub fn pitch_of(s: &Step, base: i32, chromatic: bool, scale: &Scale) -> i32 {
    let oct = s.octave as i32 + s.jump as i32;
    if chromatic {
        base + s.note as i32 + if s.fifth { 7 } else { 0 } + 12 * oct
    } else {
        let n = scale.len().max(1) as i32;
        let (d, _) = scale.locate(base);
        let fifth = if s.fifth { (4 * n + 3) / 7 } else { 0 };
        scale.pitch_at(d + s.note as i32 + fifth) + 12 * oct
    }
}

// ---------------------------------------------------------------- generators

/// The notes a bass line is made of, as degrees of a seven-note key (they
/// are moved onto other keys by `tone_note`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tone {
    Root,
    Second,
    Flat2,
    Third,
    Fourth,
    Fifth,
    Sixth,
    Seventh,
    Octave,
    Down7,
    Down5,
}

/// A tone as (note, octave): a degree of a key of `n` notes, or a semitone.
fn tone_note(t: Tone, n: usize, chromatic: bool) -> (i32, i32) {
    let (d7, semis, oct) = match t {
        Tone::Root => (0, 0, 0),
        Tone::Second => (1, 2, 0),
        Tone::Flat2 => (1, 1, 0),
        Tone::Third => (2, 3, 0),
        Tone::Fourth => (3, 5, 0),
        Tone::Fifth => (4, 7, 0),
        Tone::Sixth => (5, 8, 0),
        Tone::Seventh => (6, 10, 0),
        Tone::Octave => (7, 12, 0),
        Tone::Down7 => (6, 10, -1),
        Tone::Down5 => (4, 7, -1),
    };
    if chromatic {
        (semis, oct)
    } else {
        let n = n.max(1) as i32;
        ((d7 * n + 3) / 7, oct)
    }
}

/// How a genre's bass goes.
struct Style {
    /// How likely a note is on each sixteenth of the bar (0..9).
    rhythm: [u8; 16],
    /// Bars it writes (two for the slow, long ones).
    bars: usize,
    /// Its notes and how often (the root and its octave always count;
    /// the others as far as Melody lets them).
    tones: &'static [(Tone, u8)],
    slide: f32,
    accent: f32,
    /// Notes held on into the steps after them.
    tie: f32,
    /// An octave up now and then, and on which sixteenths it always is.
    jump: f32,
    jump_on: u16,
    ratchet: f32,
    /// How long its notes are (eighths of a step).
    gate: u8,
    /// Wob rates it likes (indices into WOBS) and how often.
    wobs: &'static [u8],
    wob: f32,
    /// Softer notes off the strong places (ghost notes).
    ghost: f32,
    /// Where its kick drum usually is (a bit per sixteenth).
    kick: u16,
}

const FOUR: u16 = 0b0001_0001_0001_0001;

fn style(genre: usize) -> Style {
    use Tone::*;
    let base = Style { rhythm: [0; 16], bars: 1, tones: &[(Root, 10)], slide: 0.0, accent: 0.2, tie: 0.0, jump: 0.0, jump_on: 0, ratchet: 0.0, gate: 4, wobs: &[], wob: 0.0, ghost: 0.0, kick: FOUR };
    match genre {
        // Acid: sixteenths all through, slides and accents, minor thirds and sevenths.
        0 => Style { rhythm: [9, 6, 7, 6, 8, 6, 7, 6, 9, 6, 7, 6, 8, 6, 7, 6], tones: &[(Root, 10), (Octave, 5), (Third, 3), (Fifth, 3), (Seventh, 3), (Fourth, 2), (Second, 1)], slide: 0.25, accent: 0.3, tie: 0.08, jump: 0.12, ratchet: 0.05, gate: 4, ..base },
        // Techno: rolling sixteenths between the kicks.
        1 => Style { rhythm: [0, 7, 9, 7, 0, 7, 9, 7, 0, 7, 9, 7, 0, 7, 9, 7], tones: &[(Root, 12), (Octave, 3), (Fifth, 2), (Seventh, 1), (Third, 1)], slide: 0.05, accent: 0.25, jump: 0.15, ratchet: 0.05, gate: 3, ..base },
        // House: on the off-beat, the organ bass bouncing up an octave after it.
        2 => Style { rhythm: [0, 0, 9, 4, 0, 0, 9, 4, 0, 0, 9, 4, 0, 3, 9, 5], tones: &[(Root, 10), (Fifth, 3), (Seventh, 3), (Third, 2), (Octave, 2)], slide: 0.05, accent: 0.15, jump: 0.1, jump_on: 0b1000_1000_1000_1000, gate: 4, ..base },
        // UK garage: two-step, skippy, sliding.
        3 => Style { rhythm: [9, 0, 0, 5, 0, 0, 7, 0, 0, 4, 8, 0, 0, 0, 7, 3], tones: &[(Root, 8), (Fifth, 4), (Seventh, 4), (Octave, 3), (Third, 3), (Fourth, 2)], slide: 0.3, accent: 0.2, tie: 0.3, jump: 0.1, gate: 6, wobs: &[5, 7], wob: 0.15, kick: 0b0000_0100_0000_0001, ..base },
        // Dubstep: half time, long notes that wobble, the rate moving.
        4 => Style { rhythm: [9, 0, 0, 0, 0, 0, 6, 0, 7, 0, 0, 5, 0, 0, 6, 0], bars: 2, tones: &[(Root, 12), (Down7, 2), (Fifth, 2), (Flat2, 1), (Octave, 2)], slide: 0.15, accent: 0.3, tie: 0.7, jump: 0.1, gate: 8, wobs: &[3, 4, 5, 6, 7, 8], wob: 0.9, kick: 0b0000_0000_0000_0001, ..base },
        // Riddim: the root over and over in threes, wobbling in triplets.
        5 => Style { rhythm: [9, 0, 0, 7, 0, 0, 7, 0, 9, 0, 0, 7, 0, 0, 7, 0], tones: &[(Root, 14), (Fifth, 1), (Octave, 1)], accent: 0.3, tie: 0.2, gate: 7, wobs: &[6, 8], wob: 1.0, kick: 0b0000_0000_0000_0001, ..base },
        // Drum and bass: a roller, sliding, some long notes.
        6 => Style { rhythm: [9, 0, 0, 6, 0, 0, 5, 0, 0, 0, 8, 6, 0, 0, 5, 0], tones: &[(Root, 10), (Fifth, 3), (Seventh, 3), (Octave, 3), (Third, 2)], slide: 0.25, accent: 0.2, tie: 0.4, jump: 0.1, ratchet: 0.04, gate: 7, wobs: &[2, 3], wob: 0.2, kick: 0b0000_0100_0000_0001, ..base },
        // Reese: long, held, sliding notes, slowly wobbling.
        7 => Style { rhythm: [9, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 4, 0], bars: 2, tones: &[(Root, 10), (Seventh, 4), (Fifth, 4), (Third, 3), (Fourth, 2), (Second, 2)], slide: 0.5, accent: 0.1, tie: 0.95, gate: 8, wobs: &[1, 2], wob: 0.6, kick: 0b0000_0100_0000_0001, ..base },
        // Trap: 808s held long, gliding, jumping up an octave.
        8 => Style { rhythm: [9, 0, 0, 0, 0, 0, 0, 5, 0, 0, 7, 0, 0, 0, 6, 0], bars: 2, tones: &[(Root, 12), (Octave, 4), (Fifth, 3), (Seventh, 2), (Third, 1)], slide: 0.5, accent: 0.15, tie: 0.8, jump: 0.25, gate: 8, kick: 0b0100_0000_1000_0001, ..base },
        // Drill: sliding 808s, half steps.
        9 => Style { rhythm: [9, 0, 0, 6, 0, 0, 7, 0, 0, 0, 0, 6, 0, 0, 7, 0], tones: &[(Root, 10), (Flat2, 3), (Third, 3), (Fifth, 3), (Octave, 3), (Down5, 2)], slide: 0.55, accent: 0.2, tie: 0.5, jump: 0.1, gate: 7, kick: 0b0000_1000_0100_1001, ..base },
        // Pluggnb: smooth, melodic, sliding 808s.
        10 => Style { rhythm: [9, 0, 0, 4, 0, 0, 6, 0, 7, 0, 0, 4, 0, 0, 5, 0], tones: &[(Root, 6), (Third, 5), (Fifth, 5), (Seventh, 4), (Second, 3), (Octave, 3), (Sixth, 2)], slide: 0.6, accent: 0.1, tie: 0.5, jump: 0.1, gate: 8, kick: 0b0000_0100_0000_0001, ..base },
        // Synthwave: eighths, the root and its octave in turn.
        11 => Style { rhythm: [9, 0, 8, 0, 9, 0, 8, 0, 9, 0, 8, 0, 9, 0, 8, 0], tones: &[(Root, 14), (Fifth, 2), (Fourth, 1)], accent: 0.2, jump_on: 0b0100_0100_0100_0100, gate: 4, ..base },
        // Funk: syncopated sixteenths, ghost notes and slapped octaves.
        12 => Style { rhythm: [9, 0, 4, 6, 0, 5, 7, 4, 0, 3, 8, 5, 0, 6, 4, 6], tones: &[(Root, 9), (Octave, 6), (Fifth, 4), (Seventh, 4), (Third, 3), (Sixth, 2)], slide: 0.1, accent: 0.3, jump: 0.35, gate: 3, ghost: 0.5, kick: 0b0000_0100_0000_0001, ..base },
        // Disco: eighths, the off-beats up an octave.
        13 => Style { rhythm: [9, 0, 9, 0, 9, 0, 9, 0, 9, 0, 9, 0, 9, 0, 9, 0], tones: &[(Root, 12), (Fifth, 3), (Seventh, 2), (Third, 2)], accent: 0.15, jump_on: 0b0100_0100_0100_0100, gate: 4, ..base },
        // Reggaeton: on the dembow.
        14 => Style { rhythm: [9, 0, 0, 8, 0, 0, 7, 0, 9, 0, 0, 8, 0, 0, 7, 0], tones: &[(Root, 12), (Fifth, 3), (Octave, 2), (Seventh, 2)], accent: 0.25, tie: 0.3, gate: 6, ..base },
        // Psytrance: the three sixteenths after every kick, rolling.
        15 => Style { rhythm: [0, 8, 9, 9, 0, 8, 9, 9, 0, 8, 9, 9, 0, 8, 9, 9], tones: &[(Root, 16), (Octave, 1), (Fifth, 1)], accent: 0.1, gate: 2, ..base },
        // EBM: driving eighths.
        16 => Style { rhythm: [9, 0, 9, 0, 9, 0, 9, 0, 9, 0, 9, 0, 9, 0, 9, 0], tones: &[(Root, 14), (Fifth, 3), (Octave, 2), (Fourth, 1)], accent: 0.4, jump: 0.05, gate: 3, ..base },
        // Hip-hop: a few long notes, laid back.
        17 => Style { rhythm: [9, 0, 0, 0, 0, 0, 0, 6, 0, 0, 7, 0, 0, 0, 0, 4], tones: &[(Root, 8), (Fifth, 4), (Seventh, 4), (Third, 3), (Octave, 2), (Fourth, 2)], slide: 0.2, accent: 0.15, tie: 0.6, gate: 7, kick: 0b0000_0100_1000_0001, ..base },
        // Afrobeats: three-three-two and the off-beats.
        18 => Style { rhythm: [9, 0, 0, 7, 0, 0, 6, 0, 0, 0, 8, 0, 6, 0, 0, 4], tones: &[(Root, 8), (Fifth, 5), (Octave, 3), (Third, 3), (Sixth, 2)], slide: 0.15, accent: 0.2, tie: 0.3, gate: 5, ..base },
        // Amapiano: the log drum, bouncing, gliding, jumping.
        _ => Style { rhythm: [9, 0, 0, 6, 0, 0, 7, 0, 0, 6, 0, 0, 8, 0, 6, 0], tones: &[(Root, 8), (Octave, 5), (Fifth, 4), (Seventh, 3), (Fourth, 2)], slide: 0.45, accent: 0.3, jump: 0.3, ratchet: 0.15, gate: 3, ..base },
    }
}

/// What a genre sounds right with: its division, groove and swing, glide,
/// base note and an instrument to try it on (instrument, preset).
pub struct Feel {
    pub division: usize,
    pub groove: usize,
    pub swing: f32,
    pub glide: usize,
    pub base: f32,
    pub pair: (&'static str, &'static str),
}

pub fn feel(genre: usize) -> Feel {
    let f = |division, groove, swing, glide, base, pair| Feel { division, groove, swing, glide, base, pair };
    match genre {
        0 => f(2, 0, 0.1, 0, 36.0, ("Bassline", "Squelch")),
        1 => f(2, 1, 0.3, 0, 36.0, ("Analog", "Moog Bass")),
        2 => f(2, 2, 0.5, 0, 36.0, ("Analog", "Funk Clav")),
        3 => f(2, 5, 0.6, 0, 38.0, ("Crimson", "Reese Bass")),
        4 => f(2, 0, 0.0, 0, 33.0, ("Crimson", "Wobble Bass")),
        5 => f(2, 0, 0.0, 0, 33.0, ("Silicon", "Synced Wobble")),
        6 => f(2, 1, 0.2, 0, 36.0, ("Crimson", "Reese Bass")),
        7 => f(2, 0, 0.0, 0, 33.0, ("Crimson", "Reese Bass")),
        8 => f(2, 0, 0.2, 1, 36.0, ("Synth", "Sub Bass")),
        9 => f(3, 0, 0.0, 1, 36.0, ("Synth", "Sub Bass")),
        10 => f(2, 6, 0.4, 1, 36.0, ("Analog", "Sub Bass")),
        11 => f(2, 0, 0.0, 0, 36.0, ("Analog", "Moog Bass")),
        12 => f(2, 6, 0.5, 0, 36.0, ("Mono", "Classic Bass")),
        13 => f(2, 2, 0.3, 0, 36.0, ("Analog", "Moog Bass")),
        14 => f(2, 9, 0.5, 0, 36.0, ("Synth", "Fat Bass")),
        15 => f(2, 0, 0.0, 0, 36.0, ("Silicon", "Analog Bass")),
        16 => f(2, 0, 0.0, 0, 36.0, ("Mono", "Classic Bass")),
        17 => f(2, 6, 0.5, 0, 36.0, ("Synth", "Fat Bass")),
        18 => f(2, 2, 0.35, 0, 38.0, ("Analog", "Sub Bass")),
        _ => f(2, 1, 0.4, 1, 36.0, ("Silicon", "Chip Bass")),
    }
}

/// What Ignite writes from.
#[derive(Clone, Copy, Debug)]
pub struct Ignite {
    pub genre: usize,
    pub density: f32,
    pub complexity: f32,
    pub melody: f32,
    pub seed: u32,
    /// Notes in the key (its degrees an octave), or Chromatic.
    pub scale_len: usize,
    pub chromatic: bool,
    /// The band's chord (degree, quality) on each step, when it follows the
    /// band: its notes are the chord's, counted from its root (the band moves
    /// the root as it plays).
    pub chords: Option<[(u8, u8); STEPS]>,
    /// Where the kick is (a bit per sixteenth) and what to do there (see KICKS).
    pub kick: Option<u16>,
    pub kick_mode: usize,
}

impl Ignite {
    pub fn from_params(p: &[f32], scale: &Scale) -> Self {
        Ignite {
            genre: choice(p, GENRE, GENRES.len()),
            density: p.get(DENSITY).copied().unwrap_or(0.6),
            complexity: p.get(COMPLEXITY).copied().unwrap_or(0.4),
            melody: p.get(MELODY).copied().unwrap_or(0.35),
            seed: p.get(SEED).copied().unwrap_or(1.0).round().max(0.0) as u32,
            scale_len: scale.len(),
            chromatic: choice(p, NOTES, 2) == 1,
            chords: None,
            kick: None,
            kick_mode: choice(p, KICK, KICKS.len()),
        }
    }
}

fn mix_seed(a: u32, b: u32) -> u32 {
    (a ^ b.wrapping_mul(0x9E37_79B9)).wrapping_mul(0x85EB_CA6B).rotate_left(13) | 1
}

/// A new pattern in a genre, and how long it is.
pub fn ignite(g: &Ignite) -> ([Step; STEPS], usize) {
    let st = style(g.genre.min(GENRES.len() - 1));
    let mut r = Rng::new(mix_seed(g.seed, g.genre as u32 + 1));
    let density = g.density.clamp(0.0, 1.0);
    let cx = g.complexity.clamp(0.0, 1.0);
    let melody = g.melody.clamp(0.0, 1.0);
    // Busy ones write a second bar that answers the first.
    let bars = if st.bars == 1 && cx > 0.65 { 2 } else { st.bars };
    let len = (bars * 16).min(STEPS);
    let kick = g.kick.unwrap_or(st.kick);
    let mut steps = [Step::REST; STEPS];
    let mut last_on: Option<usize> = None;
    for i in 0..len {
        let pos = i % 16;
        let bar = i / 16;
        let mut w = st.rhythm[pos] as f32 / 9.0;
        let on_kick = kick >> pos & 1 == 1;
        match g.kick_mode {
            1 if on_kick && pos != 0 => w = 0.0,
            1 if on_kick => w *= 0.3,
            2 if on_kick => w = w.max(0.9),
            2 => w *= 0.5,
            _ => {}
        }
        let mut chance_on = (w * (0.35 + 1.1 * density)).clamp(0.0, 1.0);
        if w == 0.0 {
            // Busier: now and then a note where the style has none.
            chance_on = cx * density * 0.18;
        }
        // The answering bar moves a few of its notes.
        if bar % 2 == 1 && r.unipolar() < 0.25 * cx {
            chance_on = 1.0 - chance_on;
        }
        let first = i == 0 && st.rhythm[0] > 0 && g.kick_mode != 1;
        let plays = first || r.unipolar() < chance_on;
        if !plays {
            // A long note held on into this step.
            if let Some(l) = last_on {
                if steps[l].sounds() && r.unipolar() < st.tie && (i == l + 1 || steps[i - 1].tie) {
                    steps[i] = Step { on: true, tie: true, note: steps[l].note, octave: steps[l].octave, wob: steps[l].wob, modv: steps[l].modv, ..Step::REST };
                    if !st.wobs.is_empty() && r.unipolar() < st.wob * 0.5 {
                        steps[i].wob = st.wobs[(r.next_u32() as usize) % st.wobs.len()];
                    }
                }
            }
            continue;
        }
        // Its note: the root most of all, the others as far as Melody lets them.
        let tones: &[(Tone, u8)] = match g.chords.map(|c| c[i].1) {
            Some(q) => chord_tones(q),
            None => st.tones,
        };
        let weight = |t: Tone, w: u8| -> f32 {
            let w = w as f32;
            if matches!(t, Tone::Root | Tone::Octave) {
                w
            } else {
                w * (melody * 2.0).min(1.5)
            }
        };
        let total: f32 = tones.iter().map(|(t, w)| weight(*t, *w)).sum();
        let mut x = r.unipolar() * total.max(1e-6);
        let mut tone = Tone::Root;
        for (t, w) in tones {
            x -= weight(*t, *w);
            if x <= 0.0 {
                tone = *t;
                break;
            }
        }
        // The first of a bar comes back to the root.
        if pos == 0 && melody < 0.8 {
            tone = Tone::Root;
        }
        let (mut note, mut octave) = tone_note(tone, g.scale_len, g.chromatic);
        while note < 0 {
            note += if g.chromatic { 12 } else { g.scale_len.max(1) as i32 };
            octave -= 1;
        }
        let mut s = Step::note(note.min(NOTE_MAX as i32) as u8);
        s.octave = octave.clamp(-2, 2) as i8;
        s.jump = st.jump_on >> pos & 1 == 1 || r.unipolar() < st.jump * (0.5 + cx);
        s.accent = r.unipolar() < st.accent * if pos % 4 == 0 { 1.6 } else { 0.8 };
        s.slide = r.unipolar() < st.slide;
        s.gate = (st.gate as i32 + (r.bipolar() * 2.0 * cx).round() as i32).clamp(1, 16) as u8;
        s.velocity = if st.ghost > 0.0 && w < 0.6 && r.unipolar() < st.ghost { 58 } else { 96 + (r.unipolar() * 12.0) as u8 };
        if r.unipolar() < st.ratchet + cx * 0.08 {
            s.ratchet = 2 + (r.unipolar() * (1.0 + cx * 2.5)) as u8;
            s.curve = (r.next_u32() % 3) as u8;
        }
        if !st.wobs.is_empty() && r.unipolar() < st.wob {
            s.wob = st.wobs[(r.next_u32() as usize) % st.wobs.len()];
            s.modv = 80 + (r.unipolar() * 47.0) as u8;
        }
        // Busy ones: weak notes by chance, a few on conditions, a nudge.
        if cx > 0.3 && w < 0.7 && pos != 0 && r.unipolar() < cx * 0.3 {
            s.chance = 50 + (r.unipolar() * 40.0) as u8;
        }
        if cx > 0.5 && pos != 0 && r.unipolar() < (cx - 0.5) * 0.25 {
            s.condition = [1u8, 2, 10, 11, 6, 9][(r.next_u32() % 6) as usize];
        }
        if cx > 0.6 && r.unipolar() < 0.15 {
            s.nudge = (r.bipolar() * 10.0) as i8;
        }
        steps[i] = s;
        last_on = Some(i);
    }
    // A slide into a rest is no slide.
    for i in 0..len {
        if steps[i].slide && !steps[(i + 1) % len].on {
            steps[i].slide = false;
        }
    }
    (steps, len)
}

/// The band's chord on each step of a pattern (`rate` beats a step), from a
/// progression as the band shares it (see `players::band_part`): for Ignite.
pub fn chords_on_steps(part: &[u8], rate: f64) -> Option<[(u8, u8); STEPS]> {
    if part.len() < 2 + 16 {
        return None;
    }
    let beats = [1.0, 2.0, 4.0, 8.0][(part[0] as usize).min(3)];
    let count = (part[1] as usize).clamp(1, 8);
    let mut c = [(0u8, 0u8); STEPS];
    for (i, x) in c.iter_mut().enumerate() {
        let k = ((i as f64 * rate + 1e-9) / beats).floor() as usize % count;
        *x = (part[2 + k * 2].min(7), part[3 + k * 2]);
    }
    Some(c)
}

/// The notes of a chord of `quality` (see `classic_seq::QUALITIES`).
fn chord_tones(quality: u8) -> &'static [(Tone, u8)] {
    use Tone::*;
    match quality {
        1 | 6 => &[(Root, 10), (Third, 4), (Fifth, 5), (Seventh, 4), (Octave, 4)],
        2 => &[(Root, 10), (Second, 4), (Fifth, 5), (Octave, 4)],
        3 => &[(Root, 10), (Fourth, 4), (Fifth, 5), (Octave, 4)],
        4 => &[(Root, 12), (Fifth, 6), (Octave, 5)],
        _ => &[(Root, 10), (Third, 4), (Fifth, 5), (Octave, 4)],
    }
}

/// A few small musical changes (not to the locked steps): a note a step up
/// or down or onto its octave, an accent, a slide, an octave jump, a rest
/// on a weak step or a note in one, a ratchet, a tie.
pub fn mutate(steps: &mut [Step; STEPS], len: usize, amount: f32, seed: u32, scale_len: usize) {
    let len = len.clamp(1, STEPS);
    let free: usize = steps[..len].iter().filter(|s| !s.lock).count();
    if free == 0 {
        return;
    }
    let mut r = Rng::new(mix_seed(seed, 0x0BAD_F1E5));
    let changes = 1 + (amount.clamp(0.0, 1.0) * len as f32 * 0.3).round() as usize;
    let n = scale_len.max(1) as i32;
    for _ in 0..changes {
        // An unlocked step.
        let mut i = (r.next_u32() as usize) % len;
        for _ in 0..len {
            if !steps[i].lock {
                break;
            }
            i = (i + 1) % len;
        }
        let op = r.next_u32() % 8;
        if op == 7 {
            if i > 0 && steps[i - 1].on && steps[i].on {
                steps[i].tie = !steps[i].tie;
            }
            continue;
        }
        let s = &mut steps[i];
        match op {
            0 | 1 if s.on && !s.tie => {
                let d = s.note as i32 + if r.unipolar() < 0.5 { 1 } else { -1 };
                s.note = d.rem_euclid(n.max(2)).min(NOTE_MAX as i32) as u8;
            }
            2 => s.accent = !s.accent,
            3 if s.on => s.slide = !s.slide,
            4 if s.on => s.jump = !s.jump,
            5 if i % 4 != 0 => {
                if s.on {
                    s.on = false;
                } else {
                    *s = Step { lock: s.lock, ..Step::note(0) };
                }
            }
            6 if s.on && !s.tie => s.ratchet = if s.ratchet > 1 { 1 } else { 2 },
            _ if s.on && !s.tie => s.note = if s.note == 0 { n.min(NOTE_MAX as i32) as u8 } else { 0 },
            _ => {}
        }
    }
}

/// A pattern Evolve has had `generations` goes at (as it plays it).
pub fn evolved(data: &[u8], pat: usize, generations: u32, amount: f32, seed: u32, scale_len: usize) -> [Step; STEPS] {
    let mut steps = pattern(data, pat);
    let len = length(data, pat);
    for g in 1..=generations {
        mutate(&mut steps, len, amount, evolve_seed(seed, pat, g), scale_len);
    }
    steps
}

fn evolve_seed(seed: u32, pat: usize, g: u32) -> u32 {
    mix_seed(seed ^ (pat as u32) << 24, g)
}

/// Backwards.
pub fn reverse(steps: &mut [Step; STEPS], len: usize) {
    steps[..len.clamp(1, STEPS)].reverse();
}

/// A step round, later (`right`) or earlier.
pub fn rotate(steps: &mut [Step; STEPS], len: usize, right: bool) {
    let s = &mut steps[..len.clamp(1, STEPS)];
    if right {
        s.rotate_right(1);
    } else {
        s.rotate_left(1);
    }
}

/// Upside down: what went up goes down, round the root.
pub fn invert(steps: &mut [Step; STEPS], len: usize, scale_len: usize) {
    let n = scale_len.max(1) as i32;
    for s in steps[..len.clamp(1, STEPS)].iter_mut().filter(|s| s.on && !s.lock) {
        let d = -(s.note as i32);
        let oct = s.octave as i32 + d.div_euclid(n);
        s.note = d.rem_euclid(n).min(NOTE_MAX as i32) as u8;
        s.octave = oct.clamp(-2, 2) as i8;
    }
}

/// A little looser: velocities and timing moved a little.
pub fn humanize(steps: &mut [Step; STEPS], len: usize, amount: f32, seed: u32) {
    let mut r = Rng::new(mix_seed(seed, 0x4A11));
    let a = amount.clamp(0.0, 1.0);
    for s in steps[..len.clamp(1, STEPS)].iter_mut().filter(|s| !s.lock) {
        s.velocity = (s.velocity as f32 + r.bipolar() * 24.0 * a).clamp(20.0, 127.0) as u8;
        s.nudge = (s.nudge as f32 + r.bipolar() * 8.0 * a).clamp(-45.0, 45.0) as i8;
    }
}

/// The data a preset brings: pattern A in its genre, B a busier answer to
/// it, C a sparser one.
pub fn preset_data(genre: usize, seed: u32, density: f32, complexity: f32, melody: f32) -> Vec<u8> {
    let mut d = empty();
    let mut g = Ignite { genre, density, complexity, melody, seed, scale_len: 7, chromatic: false, chords: None, kick: None, kick_mode: 0 };
    let (a, len) = ignite(&g);
    set_pattern(&mut d, 0, &a);
    set_length(&mut d, 0, len);
    g.complexity = (complexity + 0.3).min(1.0);
    g.density = (density + 0.15).min(1.0);
    g.seed = seed.wrapping_add(17);
    let (b, len) = ignite(&g);
    set_pattern(&mut d, 1, &b);
    set_length(&mut d, 1, len);
    g.complexity = (complexity * 0.5).max(0.0);
    g.density = (density - 0.25).max(0.15);
    g.seed = seed.wrapping_add(41);
    let (c, len) = ignite(&g);
    set_pattern(&mut d, 2, &c);
    set_length(&mut d, 2, len);
    d
}

// ---------------------------------------------------------------- presets

use V::{C, N};

macro_rules! fire {
    ($name:expr, $genre:expr, $gi:expr, $seed:expr, $dens:expr, $cx:expr, $mel:expr, [$($set:expr),* $(,)?]) => {
        pd($name, &[("Genre", C($genre)), ("Density", N($dens)), ("Complexity", N($cx)), ("Melody", N($mel)), $($set),*], || preset_data($gi, $seed, $dens, $cx, $mel))
    };
}

pub(crate) static PRESETS: &[Preset] = &[
    fire!("Acid: Squelch", "Acid", 0, 0x1F1A, 0.7, 0.45, 0.4, [("Swing", N(0.1)), ("Accent", N(0.8))]),
    fire!("Acid: Rubber Run", "Acid", 0, 11, 0.85, 0.3, 0.55, [("Gate", N(0.8))]),
    fire!("Acid: Night Drive", "Acid", 0, 23, 0.65, 0.6, 0.7, [("Swing", N(0.2)), ("Groove", C("MPC 58"))]),
    fire!("Techno: Rolling", "Techno", 1, 5, 0.75, 0.3, 0.15, [("Groove", C("MPC 54")), ("Swing", N(0.3)), ("Gate", N(0.7))]),
    fire!("Techno: Warehouse", "Techno", 1, 9, 0.55, 0.2, 0.05, [("Gate", N(0.6)), ("Base Note", N(33.0))]),
    fire!("Techno: Peak Time", "Techno", 1, 31, 0.9, 0.65, 0.3, [("Groove", C("MPC 54")), ("Swing", N(0.3))]),
    fire!("House: Organ Bounce", "House", 2, 3, 0.7, 0.3, 0.3, [("Groove", C("MPC 58")), ("Swing", N(0.5))]),
    fire!("House: Deep Offbeat", "House", 2, 14, 0.5, 0.2, 0.5, [("Groove", C("MPC 58")), ("Swing", N(0.4)), ("Gate", N(1.2))]),
    fire!("UK Garage: 2-Step", "UK Garage", 3, 7, 0.6, 0.45, 0.5, [("Groove", C("2-Step")), ("Swing", N(0.6)), ("Base Note", N(38.0))]),
    fire!("UK Garage: Speed Bass", "UK Garage", 3, 19, 0.8, 0.6, 0.4, [("Groove", C("2-Step")), ("Swing", N(0.5)), ("Wob Shape", C("Saw Down"))]),
    fire!("Dubstep: Wobble", "Dubstep", 4, 2, 0.6, 0.4, 0.2, [("Base Note", N(33.0)), ("Wob Shape", C("Sine"))]),
    fire!("Dubstep: Halftime Growl", "Dubstep", 4, 27, 0.7, 0.65, 0.3, [("Base Note", N(33.0)), ("Wob Shape", C("Saw Down"))]),
    fire!("Riddim: Triplet Wob", "Riddim", 5, 4, 0.7, 0.3, 0.05, [("Base Note", N(33.0)), ("Wob Shape", C("Square"))]),
    fire!("Drum & Bass: Roller", "Drum & Bass", 6, 6, 0.65, 0.4, 0.35, [("Groove", C("MPC 54")), ("Swing", N(0.2))]),
    fire!("Drum & Bass: Neuro", "Drum & Bass", 6, 33, 0.8, 0.8, 0.3, [("Wob Shape", C("Saw Up")), ("Accent", N(0.9))]),
    fire!("Reese: Liquid", "Reese", 7, 8, 0.6, 0.3, 0.6, [("Base Note", N(33.0)), ("Wob Shape", C("Triangle")), ("Wob Depth", N(0.6))]),
    fire!("Reese: Dark Hold", "Reese", 7, 45, 0.5, 0.2, 0.3, [("Base Note", N(31.0)), ("Wob Depth", N(0.4))]),
    fire!("Trap: 808 Glide", "Trap", 8, 10, 0.6, 0.3, 0.3, [("Glide", C("Bend")), ("Glide Time", N(0.8)), ("Swing", N(0.2))]),
    fire!("Trap: Dark 808", "Trap", 8, 21, 0.5, 0.5, 0.2, [("Glide", C("Bend")), ("Glide Time", N(1.0)), ("Base Note", N(34.0))]),
    fire!("Drill: Sliding 808", "Drill", 9, 12, 0.65, 0.4, 0.5, [("Division", C("1/16 T")), ("Glide", C("Bend")), ("Glide Time", N(0.6))]),
    fire!("Pluggnb: Smooth 808", "Pluggnb", 10, 13, 0.6, 0.3, 0.8, [("Glide", C("Bend")), ("Glide Time", N(0.9)), ("Groove", C("Dilla")), ("Swing", N(0.4))]),
    fire!("Pluggnb: Dreamy", "Pluggnb", 10, 29, 0.5, 0.2, 1.0, [("Glide", C("Bend")), ("Glide Time", N(1.2)), ("Gate", N(1.3))]),
    fire!("Synthwave: Octaves", "Synthwave", 11, 15, 0.9, 0.1, 0.1, []),
    fire!("Outrun: Night Drive", "Synthwave", 11, 37, 1.0, 0.7, 0.2, [("Gate", N(0.7))]),
    fire!("Funk: Slap", "Funk", 12, 16, 0.75, 0.5, 0.5, [("Groove", C("Dilla")), ("Swing", N(0.5)), ("Accent", N(0.9))]),
    fire!("Funk: Ghost Notes", "Funk", 12, 39, 0.85, 0.6, 0.6, [("Groove", C("MPC 62")), ("Swing", N(0.5))]),
    fire!("Disco: Octave Run", "Disco", 13, 17, 1.0, 0.2, 0.3, [("Groove", C("MPC 58")), ("Swing", N(0.3))]),
    fire!("Reggaeton: Dembow", "Reggaeton", 14, 18, 0.75, 0.3, 0.3, [("Groove", C("Dembow")), ("Swing", N(0.5))]),
    fire!("Psytrance: Rolling", "Psytrance", 15, 20, 0.9, 0.1, 0.05, [("Gate", N(0.7)), ("Accent", N(0.4))]),
    fire!("Psytrance: Gallop", "Psytrance", 15, 43, 0.75, 0.5, 0.1, [("Gate", N(0.6))]),
    fire!("EBM: Driving Eighths", "EBM", 16, 22, 1.0, 0.1, 0.15, [("Gate", N(0.7)), ("Accent", N(0.8))]),
    fire!("Industrial: Grind", "EBM", 16, 47, 0.95, 0.7, 0.2, [("Notes", C("Chromatic")), ("Accent", N(1.0))]),
    fire!("Hip-Hop: Boom Bap", "Hip-Hop", 17, 24, 0.55, 0.3, 0.5, [("Groove", C("Dilla")), ("Swing", N(0.5))]),
    fire!("Hip-Hop: Dilla Bass", "Hip-Hop", 17, 51, 0.65, 0.55, 0.7, [("Groove", C("Dilla")), ("Swing", N(0.8))]),
    fire!("Afrobeats: Bounce", "Afrobeats", 18, 25, 0.65, 0.35, 0.5, [("Groove", C("MPC 58")), ("Swing", N(0.35)), ("Base Note", N(38.0))]),
    fire!("Amapiano: Log Drum", "Amapiano", 19, 26, 0.6, 0.45, 0.4, [("Glide", C("Bend")), ("Glide Time", N(0.5)), ("Groove", C("MPC 54")), ("Swing", N(0.4))]),
    fire!("Amapiano: Private School", "Amapiano", 19, 53, 0.5, 0.3, 0.7, [("Glide", C("Bend")), ("Glide Time", N(0.7)), ("Gate", N(1.2))]),
];

// ---------------------------------------------------------------- runtime

/// Most notes and changes it can have waiting.
const QCAP: usize = 512;
const Q_OFF: u8 = 0;
const Q_ON: u8 = 1;
const Q_WOB: u8 = 2;
const Q_BEND: u8 = 3;
const Q_TICK: u8 = 4;
const Q_HIT: u8 = 5;

/// Something it will do at a frame: a note on or off, a wob or a bend
/// starting, or a step coming round (for the face).
#[derive(Clone, Copy, Debug, Default)]
struct Ev {
    at: u64,
    kind: u8,
    pitch: u8,
    vel: f32,
    /// Wob: its rate (beats; 0 steady) and whether it starts its wave again.
    /// Bend: how far (semitones) and how long (frames).
    a: f32,
    b: f32,
    /// The step and its pattern (for the face), and what it does (see `TRIG_*`).
    step: u8,
    meta: u8,
}

const TRIG_FIRST: u8 = 1;
const TRIG_ACCENT: u8 = 2;
const TRIG_SLIDE: u8 = 4;
const TRIG_TIE: u8 = 8;
const TRIG_JUMP: u8 = 16;

/// Where a step plays from.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Plan {
    pattern: usize,
    idx: usize,
    /// Steps since the pattern began here, and its length.
    local: i64,
    len: usize,
}

/// What a step does, with the heat on it.
#[derive(Clone, Copy, Debug)]
struct Heat {
    heat: f32,
    energy: f32,
    fill: bool,
    build: Option<f32>,
    /// Beats left of the part (for a build's last beat).
    left: f64,
}

pub struct InfernoState {
    q: [Ev; QCAP],
    n: usize,
    /// How many of each note it has sounding.
    held: [u8; 128],
    /// The note whose end the next step decides (it slides or is tied), with
    /// its glide (percent of Glide Time).
    open: Option<(u8, u8)>,
    /// The latest note-on it has queued (notes never start out of order),
    /// and each note's (its note-off never comes before it).
    last_on: u64,
    on_at: [u64; 128],
    /// The next step to plan, and the one the play started from.
    next: i64,
    start: i64,
    plan: Option<(i64, Plan)>,
    /// Ping-pong and brownian walk.
    walk: i64,
    rng: Rng,
    pre: bool,
    played_last: bool,
    /// The wob: its rate (beats; 0 steady), how far, where in its wave.
    wob_rate: f64,
    wob_level: f32,
    wob_phase: f64,
    wob_out: f32,
    /// The bend: how far it starts, where and how long.
    bend_from: f32,
    bend_at: u64,
    bend_len: u64,
    bend_out: f32,
    /// Evolve: the pattern as it has become, and of what (pattern, goes, bytes).
    overlay: [Step; STEPS],
    overlay_key: (usize, u32, u64),
    generation: u32,
    /// For the face: the step playing, the last that struck and the one before.
    now: (u8, u8),
    hits: u16,
    last_hit: u64,
    prev_hit: u8,
    heat_now: f32,
    sounding: Option<u8>,
}

impl Default for InfernoState {
    fn default() -> Self {
        Self {
            q: [Ev::default(); QCAP],
            n: 0,
            held: [0; 128],
            open: None,
            last_on: 0,
            on_at: [0; 128],
            next: i64::MIN,
            start: 0,
            plan: None,
            walk: 0,
            rng: Rng::new(1),
            pre: false,
            played_last: false,
            wob_rate: 0.0,
            wob_level: 0.0,
            wob_phase: 0.0,
            wob_out: 0.0,
            bend_from: 0.0,
            bend_at: 0,
            bend_len: 0,
            bend_out: 0.0,
            overlay: [Step::REST; STEPS],
            overlay_key: (usize::MAX, 0, 0),
            generation: 0,
            now: (0, 0),
            hits: 0,
            last_hit: 0,
            prev_hit: 0xff,
            heat_now: 0.0,
            sounding: None,
        }
    }
}

impl InfernoState {
    pub fn restart(&mut self) {
        self.n = 0;
        self.held = [0; 128];
        self.open = None;
        self.next = i64::MIN;
        self.plan = None;
        self.bend_out = 0.0;
        self.bend_len = 0;
        self.generation = 0;
        self.sounding = None;
    }

    /// What it sends besides notes: the wob (0..1) and the bend (semitones).
    pub fn outputs(&self) -> (f32, f32) {
        (self.wob_out, self.bend_out)
    }

    fn push(&mut self, e: Ev) -> bool {
        if self.n < QCAP {
            self.q[self.n] = e;
            self.n += 1;
            true
        } else {
            false
        }
    }

    /// A note-on (never before one already queued); a copy of the note
    /// still to end later ends here instead.
    fn note_on(&mut self, at: u64, pitch: u8, vel: f32, step: u8, meta: u8) -> u64 {
        let at = at.max(self.last_on);
        // Room for its note-off too, or it does not start.
        if self.n + 2 > QCAP {
            return at;
        }
        for e in self.q[..self.n].iter_mut() {
            if e.kind == Q_OFF && e.pitch == pitch && e.at > at {
                e.at = at;
            }
        }
        self.last_on = at;
        self.on_at[pitch as usize & 127] = at;
        self.push(Ev { at, kind: Q_ON, pitch, vel: vel.clamp(0.01, 1.0), step, meta, ..Ev::default() });
        at
    }

    fn note_off(&mut self, at: u64, pitch: u8) {
        self.push(Ev { at: at.max(self.on_at[pitch as usize & 127] + 1), kind: Q_OFF, pitch, ..Ev::default() });
    }

    /// Forgets the notes planned from `at` on (the song jumped), each with
    /// its note-off; what already sounds ends as planned.
    fn drop_from(&mut self, at: u64) {
        let mut i = 0;
        while i < self.n {
            let e = self.q[i];
            if e.kind == Q_ON && e.at >= at {
                self.n -= 1;
                self.q[i] = self.q[self.n];
                // Its note-off: the first of that note after it.
                let off = (0..self.n).filter(|&k| self.q[k].kind == Q_OFF && self.q[k].pitch == e.pitch && self.q[k].at >= e.at).min_by_key(|&k| self.q[k].at);
                if let Some(k) = off {
                    self.n -= 1;
                    self.q[k] = self.q[self.n];
                }
                i = 0;
                continue;
            }
            if e.kind != Q_OFF && e.kind != Q_ON && e.at >= at {
                self.n -= 1;
                self.q[i] = self.q[self.n];
                continue;
            }
            i += 1;
        }
        self.last_on = self.last_on.min(at);
    }

    /// The open note ends at `at` (a rest, or a step that does not play).
    fn close(&mut self, at: u64) {
        if let Some((o, _)) = self.open.take() {
            self.note_off(at, o);
        }
    }
}

/// How strong a place in the bar is: its one, the beats, the eighths, the rest.
fn strength(beat: f64, bar: f64) -> f32 {
    let s = (beat * 4.0).round() as i64;
    let per_bar = ((bar * 4.0).round() as i64).max(1);
    if s.rem_euclid(per_bar) == 0 {
        1.0
    } else if s.rem_euclid(4) == 0 {
        0.75
    } else if s.rem_euclid(2) == 0 {
        0.5
    } else {
        0.25
    }
}

/// The same number for the same step and purpose (0..1).
fn hash01(j: i64, salt: u64) -> f32 {
    let h = (j as u64 ^ salt.wrapping_mul(0xD6E8_FEB8_6659_FD93)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    ((h >> 40) & 0xffff) as f32 / 65535.0
}

fn lfo(shape: usize, phase: f64) -> f32 {
    let x = phase.rem_euclid(1.0) as f32;
    match shape {
        1 => 1.0 - (2.0 * x - 1.0).abs(),
        2 => 1.0 - x,
        3 => x,
        4 => (x < 0.5) as u8 as f32,
        _ => 0.5 - 0.5 * (x * std::f32::consts::TAU).cos(),
    }
}

impl SeqState {
    /// Where step `j` plays from: the part's pattern in the band, the chain,
    /// or the one picked; and which of its steps by the direction.
    fn inferno_plan(&mut self, j: i64, p: &[f32], data: &[u8], beat: f64, rate: f64, ctx: &MidiCtx) -> Plan {
        if let Some((pj, plan)) = self.inferno.plan {
            if pj == j {
                return plan;
            }
        }
        let c = j - self.inferno.start;
        let band = &ctx.band;
        let part = if band.parts && self.role.is_some() {
            let part = if beat >= band.until { band.next_part } else { band.part };
            part_pattern(data, part as usize).map(|pat| (pat, (band.place(beat).since / rate).round() as i64))
        } else {
            None
        };
        let (pattern, local) = match part {
            Some(x) => x,
            None => {
                let (list, n) = chain(data);
                if on(p, CHAIN) && n > 0 {
                    let total: i64 = list[..n].iter().map(|&q| length(data, q as usize) as i64).sum();
                    let mut pos = c.rem_euclid(total.max(1));
                    let mut pat = list[0] as usize;
                    for &q in &list[..n] {
                        let l = length(data, q as usize) as i64;
                        if pos < l {
                            pat = q as usize;
                            break;
                        }
                        pos -= l;
                    }
                    // Its round counts the chain's rounds.
                    (pat, pos + c.div_euclid(total.max(1)) * length(data, pat) as i64)
                } else {
                    (choice(p, PATTERN, PATTERNS), c)
                }
            }
        };
        let len = length(data, pattern);
        let n = len as i64;
        let idx = match choice(p, DIRECTION, DIRECTIONS.len()) {
            1 => (n - 1 - local.rem_euclid(n)) as usize,
            2 => crate::classic_seq::step_index(local, len, 2, 0),
            3 => (self.inferno.rng.next_u32() % len as u32) as usize,
            4 => {
                // A drunk walk, leaning forward.
                let d = [-1i64, 0, 1, 1][(self.inferno.rng.next_u32() % 4) as usize];
                self.inferno.walk = (self.inferno.walk + d).rem_euclid(n);
                self.inferno.walk as usize
            }
            _ => local.rem_euclid(n) as usize,
        };
        let plan = Plan { pattern, idx, local, len };
        self.inferno.plan = Some((j, plan));
        plan
    }

    /// Step `idx` of a pattern, as Evolve has it now.
    fn inferno_step(&mut self, p: &[f32], data: &[u8], plan: &Plan, ctx: &MidiCtx) -> Step {
        let st = &mut self.inferno;
        let g = st.generation;
        let read = |data: &[u8], i: usize| step(data, plan.pattern, i);
        let main = if g == 0 || choice(p, EVOLVE, EVOLVES.len()) == 0 {
            read(data, plan.idx)
        } else {
            let bytes = pattern_bytes(data, plan.pattern);
            let sum = bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x100_0000_01b3)) ^ length(data, plan.pattern) as u64;
            let amount = p.get(EVOLVE_AMOUNT).copied().unwrap_or(0.25);
            let seed = p.get(SEED).copied().unwrap_or(1.0).round().max(0.0) as u32;
            let (kp, kg, ks) = st.overlay_key;
            if kp == plan.pattern && ks == sum && kg + 1 == g {
                // One more go.
                mutate(&mut st.overlay, plan.len, amount, evolve_seed(seed, plan.pattern, g), ctx.scale.len());
                st.overlay_key = (plan.pattern, g, sum);
            } else if st.overlay_key != (plan.pattern, g, sum) {
                st.overlay = evolved(data, plan.pattern, g, amount, seed, ctx.scale.len());
                st.overlay_key = (plan.pattern, g, sum);
            }
            st.overlay[plan.idx]
        };
        // The lanes that run at their own lengths.
        let lane = |l: usize| -> Option<usize> {
            let ll = lane_length(data, plan.pattern, l);
            (ll != plan.len).then(|| plan.local.rem_euclid(ll as i64) as usize)
        };
        let mut s = main;
        let over = |i: usize| if g == 0 || choice(p, EVOLVE, EVOLVES.len()) == 0 { step(data, plan.pattern, i) } else { self.inferno.overlay[i] };
        if let Some(i) = lane(1) {
            s.accent = over(i).accent;
        }
        if let Some(i) = lane(2) {
            let o = over(i);
            s.slide = o.slide;
            s.glide = o.glide;
        }
        if let Some(i) = lane(3) {
            let o = over(i);
            s.octave = o.octave;
            s.jump = o.jump;
            s.fifth = o.fifth;
        }
        if let Some(i) = lane(4) {
            let o = over(i);
            s.wob = o.wob;
            s.modv = o.modv;
        }
        s
    }

    /// How hot a step plays: Burn, the band's energy, its fill and build.
    fn inferno_heat(&self, p: &[f32], beat: f64, ctx: &MidiCtx) -> Heat {
        let b = &ctx.band;
        let bar = b.bar.max(1.0);
        let place = (b.parts && self.role.is_some()).then(|| b.place(beat));
        let energy = place.map(|pl| pl.energy(bar)).unwrap_or(0.5);
        let build = place.and_then(|pl| pl.build_at(bar));
        let fill = on(p, FILL) || place.is_some_and(|pl| pl.fill(bar));
        let burn = if on(p, BURN) { 1.0 } else { 0.0f32 };
        let heat = burn.max((energy - 0.5) * 2.0).max(build.unwrap_or(0.0) * 0.8).clamp(0.0, 1.0);
        Heat { heat, energy, fill, build, left: place.map(|pl| pl.left).unwrap_or(f64::INFINITY) }
    }

    /// Plans step `j` (on the grid at frame `t`): its notes, their ends, its
    /// wob and bend, into the queue. `early`: it may start before `t`.
    #[allow(clippy::too_many_arguments)]
    fn inferno_schedule(&mut self, j: i64, t: u64, step_frames: f64, early: bool, p: &[f32], data: &[u8], ctx: &MidiCtx) {
        let spb = ctx.samples_per_beat.max(1.0);
        let rate = step_frames / spb;
        let beat = crate::band::beat_at(ctx, t);
        let plan = self.inferno_plan(j, p, data, beat, rate, ctx);
        // What the step after it does (a tie keeps this one sounding).
        let next_plan = self.inferno_plan(j + 1, p, data, beat + rate, rate, ctx);
        self.inferno.plan = Some((j, plan));
        let next = self.inferno_step(p, data, &next_plan, ctx);
        let s = self.inferno_step(p, data, &plan, ctx);
        self.inferno.plan = Some((j + 1, next_plan));
        let tick = Ev { at: t, kind: Q_TICK, step: plan.idx as u8, meta: plan.pattern as u8, ..Ev::default() };
        self.inferno.push(tick);
        let next_ties = next.sounds() && next.tie;
        let h = self.inferno_heat(p, beat, ctx);
        self.inferno.heat_now = h.heat;
        let bar = ctx.band.bar.max(1.0);
        let gate_scale = p.get(GATE).copied().unwrap_or(1.0).clamp(0.05, 2.0) as f64;
        let gate_len = |s: &Step| (s.gate as f64 / 8.0 * step_frames * gate_scale).max(spb * 0.01);
        if !s.sounds() {
            self.inferno.close(t);
            self.inferno.played_last = false;
            return;
        }
        // A tie holds the note before on.
        if s.tie {
            if let Some((o, glide)) = self.inferno.open {
                self.inferno_wob(t, &s, false, p);
                if !(next_ties || s.slide) {
                    self.inferno.open = None;
                    self.inferno.note_off(t + gate_len(&s) as u64, o);
                } else {
                    self.inferno.open = Some((o, if s.slide { s.glide } else { glide }));
                }
                self.inferno.push(Ev { at: t, kind: Q_HIT, step: plan.idx as u8, meta: TRIG_TIE, ..Ev::default() });
                return;
            }
        }
        // Its condition and its chance.
        let round = plan.local.div_euclid(plan.len as i64);
        let first = round == 0;
        let holds = condition_holds(s.condition, round, first, h.fill, self.inferno.pre, self.inferno.played_last);
        if s.condition != 0 {
            self.inferno.pre = holds;
        }
        let lucky = s.chance >= 100 || self.inferno.rng.unipolar() * 100.0 < s.chance as f32;
        // A low energy plays the strong steps and the accents.
        let strong = strength(beat, bar);
        let thinned = h.energy < 0.5 && !s.accent && strong < (0.5 - h.energy) * 1.6;
        if !(holds && lucky) || thinned {
            self.inferno.close(t);
            self.inferno.played_last = false;
            return;
        }
        self.inferno.played_last = true;
        // When: the groove and its nudge.
        let groove = GROOVE_SHAPES[choice(p, GROOVE, GROOVES.len())][plan.local.rem_euclid(8) as usize] * p.get(SWING).copied().unwrap_or(0.0).clamp(0.0, 1.0);
        let shift = (groove + s.nudge as f32 / 100.0).clamp(if early { -0.45 } else { 0.0 }, 0.5);
        let at = (t as f64 + shift as f64 * step_frames).max(ctx.now as f64).round() as u64;
        // What it plays, with the heat on it.
        let chromatic = choice(p, NOTES, 2) == 1;
        let base = p.get(BASE).copied().unwrap_or(36.0).round() as i32;
        let mut s = s;
        let mut ratchet = s.ratchet as usize;
        if h.heat > 0.0 {
            s.accent |= hash01(j, 1) < h.heat * 0.5;
            if hash01(j, 2) < h.heat * 0.4 {
                ratchet = ratchet.max(2 + (h.heat > 0.8 && hash01(j, 3) < 0.3) as usize);
            }
            if !s.jump && strong < 0.75 && hash01(j, 4) < h.heat * 0.3 {
                s.jump = true;
            }
        }
        if h.fill {
            // A fill in the last beat of the bar: rolls, rising.
            let into = (beat / bar).fract() * bar;
            if into >= bar - 1.0 - 1e-6 {
                ratchet = ratchet.max(2 + (plan.local % 2) as usize);
                s.jump |= plan.local % 2 == 1;
            }
        }
        if let Some(f) = h.build {
            if hash01(j, 5) < f {
                ratchet = ratchet.max(2);
            }
            s.jump |= hash01(j, 6) < f * 0.5;
            if h.left <= 1.0 + 1e-6 {
                ratchet = ratchet.max(4);
            }
        }
        if h.energy < 0.3 {
            ratchet = 1;
        }
        let ratchet = ratchet.clamp(1, 8);
        let pitch = pitch_of(&s, base, chromatic, &ctx.scale);
        let pitch = self.followed(ctx, at, self.shifted(ctx, pitch)).clamp(0, 127) as u8;
        let accent = p.get(ACCENT).copied().unwrap_or(0.6).clamp(0.0, 1.0);
        let mut vel = s.velocity as f32 / 127.0 * (1.0 + 0.15 * h.heat);
        if s.accent {
            vel = (vel + 0.2 + 0.3 * accent).max(0.8);
        }
        let vel = vel.clamp(0.02, 1.0);
        let keep = s.slide || next_ties;
        let bend_mode = choice(p, GLIDE, GLIDES.len()) == 1;
        let glide_frames = |g: u8| (p.get(GLIDE_TIME).copied().unwrap_or(0.5).max(0.01) as f64 * g as f64 / 100.0 * step_frames).max(1.0) as f32;
        let mut meta = TRIG_FIRST | if s.accent { TRIG_ACCENT } else { 0 } | if s.slide { TRIG_SLIDE } else { 0 } | if s.jump { TRIG_JUMP } else { 0 };
        let idx = plan.idx as u8;
        if ratchet == 1 {
            let start = match self.inferno.open.take() {
                // Sliding into the same note: on, without a new attack.
                Some((o, _)) if o == pitch => {
                    self.inferno_wob(at, &s, false, p);
                    self.inferno.push(Ev { at, kind: Q_HIT, step: idx, meta: meta | TRIG_TIE, ..Ev::default() });
                    at
                }
                Some((o, glide)) => {
                    let at = self.inferno.note_on(at, pitch, vel, idx, meta);
                    if bend_mode {
                        // The new note starts where the old one was and bends home.
                        self.inferno.note_off(at, o);
                        let len = glide_frames(glide);
                        self.inferno.push(Ev { at, kind: Q_BEND, a: o as f32 - pitch as f32, b: len, ..Ev::default() });
                    } else {
                        // A little overlap, so a mono synth glides.
                        self.inferno.note_off(at + (step_frames * 0.06).max(8.0) as u64, o);
                    }
                    self.inferno_wob(at, &s, true, p);
                    at
                }
                None => {
                    let at = self.inferno.note_on(at, pitch, vel, idx, meta);
                    self.inferno_wob(at, &s, true, p);
                    at
                }
            };
            if keep {
                self.inferno.open = Some((pitch, s.glide));
            } else {
                self.inferno.note_off(start + gate_len(&s) as u64, pitch);
            }
        } else {
            // Ratchets: hits within the step, each its own note.
            if let Some((o, _)) = self.inferno.open.take() {
                self.inferno.note_off(at, o);
            }
            let curve = s.curve as usize;
            let span = step_frames * (1.0 - shift.max(0.0) as f64 * 0.5);
            let place = |i: usize| -> f64 {
                let x = i as f64 / ratchet as f64;
                match curve {
                    1 => x.powf(0.6),
                    2 => x.powf(1.6),
                    _ => x,
                }
            };
            for i in 0..ratchet {
                let t0 = at as f64 + place(i) * span;
                let t1 = at as f64 + if i + 1 < ratchet { place(i + 1) * span } else { span };
                let v = match curve {
                    3 => vel * (1.0 - 0.6 * i as f32 / ratchet as f32),
                    4 => vel * (0.4 + 0.6 * (i + 1) as f32 / ratchet as f32),
                    _ => vel * if i == 0 { 1.0 } else { 0.9 },
                };
                let on_at = self.inferno.note_on(t0.round() as u64, pitch, v, idx, meta);
                meta &= !TRIG_FIRST;
                if i == 0 {
                    self.inferno_wob(on_at, &s, true, p);
                }
                if i + 1 == ratchet && keep {
                    self.inferno.open = Some((pitch, s.glide));
                } else {
                    let len = ((t1 - t0) * 0.7).min(gate_len(&s)).max(1.0);
                    self.inferno.note_off(on_at + len as u64, pitch);
                }
            }
        }
    }

    /// The wob of a step that sounds, from `at`.
    fn inferno_wob(&mut self, at: u64, s: &Step, restart: bool, p: &[f32]) {
        if choice(p, WOB_OUT, WOB_OUTS.len()) == 0 {
            return;
        }
        let rate = WOB_BEATS[(s.wob as usize).min(WOB_BEATS.len() - 1)];
        let level = if s.modv == 0 { if s.wob > 0 { 1.0 } else { 0.0 } } else { s.modv as f32 / 127.0 };
        self.inferno.push(Ev { at, kind: Q_WOB, a: rate as f32, b: restart as u8 as f32, vel: level, ..Ev::default() });
    }

    /// Plays this block's step (from the step loop): plans the step after it
    /// too, so a step nudged early can start before its time.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn inferno(&mut self, k: i64, p: &[f32], data: &[u8], at: u64, step_frames: f64, _out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        // Where its steps are, as the step loop counts them (the part's feel too).
        let spb = ctx.samples_per_beat.max(1.0);
        let rate = step_frames / spb;
        let (origin, base) = match crate::band::feel(self.role, ctx) {
            Some((_, origin)) => (origin, (origin / rate).round() as i64),
            None => (0.0, 0),
        };
        let frame_of = |j: i64| (ctx.now as f64 + ((origin + (j - base) as f64 * rate - ctx.beat) * spb).max(0.0)).round() as u64;
        let st = &mut self.inferno;
        if st.next != k && st.next != k + 1 {
            // A start or a jump (a loop, a locate, a part in another feel).
            st.drop_from(at);
            st.close(at);
            st.next = k;
            st.start = k;
            st.plan = None;
            st.walk = -1;
            st.pre = false;
            st.played_last = false;
            st.generation = 0;
            st.overlay_key = (usize::MAX, 0, 0);
            let seed = p.get(SEED).copied().unwrap_or(1.0).round().max(0.0) as u32;
            st.rng = Rng::new(mix_seed(seed, 0xF1_4E));
        }
        // Evolve: a go every so many bars.
        let every = EVOLVE_BARS[choice(p, EVOLVE, EVOLVES.len())];
        while self.inferno.next <= k + 1 {
            let j = self.inferno.next;
            if every > 0 {
                let bars = (((j - self.inferno.start) as f64 * rate + 1e-6) / ctx.band.bar.max(1.0)).floor() as i64;
                self.inferno.generation = (bars / every).max(0) as u32;
            } else {
                self.inferno.generation = 0;
            }
            let t = frame_of(j);
            self.inferno_schedule(j, t, step_frames, j > k, p, data, ctx);
            self.inferno.next = j + 1;
        }
    }

    /// Sends what is due in this block (playing or not), works out the wob
    /// and the bend, and reports to the face. Stopped, it lets go of it all.
    pub(crate) fn inferno_flush(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let st = &mut self.inferno;
        if !ctx.playing {
            for (pitch, n) in st.held.iter_mut().enumerate() {
                for _ in 0..*n {
                    out.push(MidiEvent::off(0, pitch as u8));
                }
                *n = 0;
            }
            st.restart();
            st.last_on = 0;
            st.wob_out = 0.0;
            self.visual = st.visual(false);
            return;
        }
        let end = ctx.now + ctx.len as u64;
        let offset = |at: u64| at.saturating_sub(ctx.now).min(ctx.len.saturating_sub(1) as u64) as u32;
        loop {
            // The earliest that is due (offs before ons at the same frame).
            let mut best: Option<usize> = None;
            for i in 0..st.n {
                let e = &st.q[i];
                let earlier = match best {
                    Some(b) => (e.at, e.kind == Q_ON) < (st.q[b].at, st.q[b].kind == Q_ON),
                    None => true,
                };
                if e.at < end && earlier {
                    best = Some(i);
                }
            }
            let Some(i) = best else { break };
            let e = st.q[i];
            st.n -= 1;
            st.q[i] = st.q[st.n];
            match e.kind {
                Q_ON => {
                    st.held[e.pitch as usize] = st.held[e.pitch as usize].saturating_add(1);
                    out.push(MidiEvent::on(offset(e.at), e.pitch, e.vel));
                    st.sounding = Some(e.pitch);
                    if e.meta & TRIG_FIRST != 0 {
                        st.hit(e.step, e.meta);
                    }
                }
                Q_OFF => {
                    let h = &mut st.held[e.pitch as usize];
                    if *h > 0 {
                        *h -= 1;
                        out.push(MidiEvent::off(offset(e.at), e.pitch));
                    }
                    if st.sounding == Some(e.pitch) && *h == 0 {
                        st.sounding = None;
                    }
                }
                Q_WOB => {
                    st.wob_rate = e.a as f64;
                    st.wob_level = e.vel;
                    if e.b > 0.5 {
                        // Its wave starts again where the note does.
                        st.wob_phase = -(e.at.saturating_sub(ctx.now) as f64 / ctx.samples_per_beat.max(1.0)) / st.wob_rate.max(1e-6);
                    }
                }
                Q_BEND => {
                    st.bend_from = e.a;
                    st.bend_at = e.at;
                    st.bend_len = e.b.max(1.0) as u64;
                }
                Q_HIT => st.hit(e.step, e.meta),
                _ => st.now = (e.step, e.meta),
            }
        }
        // The wob at the block's end.
        let depth = p.get(WOB_DEPTH).copied().unwrap_or(1.0).clamp(0.0, 1.0);
        let beats = ctx.len as f64 / ctx.samples_per_beat.max(1.0);
        st.wob_out = if st.wob_rate > 0.0 {
            st.wob_phase += beats / st.wob_rate;
            st.wob_level * depth * lfo(choice(p, WOB_SHAPE, WOB_SHAPES.len()), st.wob_phase.max(0.0))
        } else {
            st.wob_level * depth
        };
        st.bend_out = if st.bend_len > 0 && end > st.bend_at {
            let f = ((end - st.bend_at) as f32 / st.bend_len as f32).clamp(0.0, 1.0);
            if f >= 1.0 {
                st.bend_len = 0;
            }
            st.bend_from * (1.0 - f)
        } else if st.bend_len > 0 {
            st.bend_from
        } else {
            0.0
        };
        self.visual = st.visual(true);
    }
}

impl InfernoState {
    /// A step struck (for the face).
    fn hit(&mut self, step: u8, meta: u8) {
        self.prev_hit = (self.last_hit & 0xff) as u8;
        self.hits = self.hits.wrapping_add(1);
        self.last_hit = step as u64 | (meta as u64) << 8 | (self.sounding.unwrap_or(0) as u64) << 16;
    }

    /// The face's word: (0) the step playing, its pattern, Evolve's goes,
    /// the heat, what sounds; (1) the last step struck, what it did, the one
    /// before, how many so far; (2) the wob and the bend.
    fn visual(&self, playing: bool) -> [u64; 4] {
        let heat = (self.heat_now.clamp(0.0, 1.0) * 255.0) as u64;
        let sounding = self.sounding.map(|p| p as u64 | 0x80 << 8).unwrap_or(0);
        let v0 = self.now.0 as u64 | (self.now.1 as u64) << 8 | ((self.generation & 0xff) as u64) << 16 | heat << 24 | sounding << 32 | (playing as u64) << 63;
        let v1 = (self.last_hit & 0xffff) | (self.prev_hit as u64) << 16 | (self.hits as u64) << 32;
        let v2 = (self.wob_out.clamp(0.0, 1.0) * 1000.0) as u64 | (((self.bend_out.clamp(-48.0, 48.0) + 48.0) * 100.0) as u64) << 16;
        [v0, v1, v2, 0]
    }
}

/// The face's view of the engine's word (see `InfernoState::visual`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Live {
    pub playing: bool,
    pub step: usize,
    pub pattern: usize,
    pub generation: u32,
    pub heat: f32,
    pub sounding: Option<u8>,
    pub hit_step: usize,
    pub hit_accent: bool,
    pub hit_slide: bool,
    pub hit_tie: bool,
    pub hit_jump: bool,
    pub prev_hit: Option<usize>,
    pub hits: u32,
    pub wob: f32,
    pub bend: f32,
}

impl Live {
    pub fn unpack(v: &[u64; 4]) -> Self {
        let b = |x: u64, s: u32| ((x >> s) & 0xff) as usize;
        let meta = b(v[1], 8) as u8;
        Live {
            playing: v[0] >> 63 == 1,
            step: b(v[0], 0),
            pattern: b(v[0], 8).min(PATTERNS - 1),
            generation: b(v[0], 16) as u32,
            heat: b(v[0], 24) as f32 / 255.0,
            sounding: (v[0] >> 40 & 0x80 != 0).then(|| b(v[0], 32) as u8),
            hit_step: b(v[1], 0),
            hit_accent: meta & TRIG_ACCENT != 0,
            hit_slide: meta & TRIG_SLIDE != 0,
            hit_tie: meta & TRIG_TIE != 0,
            hit_jump: meta & TRIG_JUMP != 0,
            prev_hit: (b(v[1], 16) != 0xff).then(|| b(v[1], 16)),
            hits: ((v[1] >> 32) & 0xffff) as u32,
            wob: (v[2] & 0xffff) as f32 / 1000.0,
            bend: ((v[2] >> 16) & 0xffff) as f32 / 100.0 - 48.0,
        }
    }
}

#[cfg(test)]
mod tests;
