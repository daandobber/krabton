//! Touch: a track played with plates, made to play with. The plates (on the
//! Touch view, the computer keys or fingers on a touch screen) give notes and
//! how hard they are pressed. How they sound:
//!
//! - *Play*: as they are (Notes), arpeggiated (Arp: six orders over one to
//!   four octaves), strummed (Strum: the chord's notes one after the other,
//!   down and up in turn), with echoes that fade (Echo), or bouncing like a
//!   ball dropped, each bounce sooner and softer (Bounce).
//! - *Voicing*: a plate plays one note or a chord built in the key (a third,
//!   a fifth, an octave, a triad, a seventh, a ninth, sus4, power, a cluster,
//!   an open voicing).
//! - The pulser plays the held plate again in time (with swing), also with
//!   the song stopped; Latch keeps the last plate (or chord) going, Run plays
//!   with no plate at all (from the base note); Chance leaves pulses out and
//!   Wander moves a note a step or two.
//! - The sequencer (switched on and off): sixteen steps, each a note in scale
//!   steps, a value, a chance, a ratchet (one to four notes in the step), and
//!   rest, tie and accent. *Trigger* lets the steps decide which notes play
//!   (rests, chances, ratchets, accents, ties), *Pitch* lets them move the
//!   notes; each pulse (or each touch, without the pulser) a step further, in
//!   one of seven orders.
//!
//! What it makes is patched to the instrument's knobs through Touch
//! modulators (`ModKind::Touch`): the pressure, the pulse, the steps' values,
//! a stepped random (quantized to a few levels, or not) and a drifting one,
//! where along the plates the hand is, an LFO, a drawn shape, the XY pad's X
//! and Y, an envelope (once, held, or cycling) and each plate's own value.
//!
//! The plates can also be the Tonnetz (the *Tonnetz* layout): the lattice of
//! notes a fifth apart along it and a major third apart up it, where every
//! triangle is a major or a minor triad and the triangles beside it share two
//! of its notes (P, L and R: the parallel, the leading-tone exchange, the
//! relative). A triangle pressed is its three notes pressed together (its
//! root last): one touch, a chord the pulser pulses whole. The sequence's
//! steps are then triangles too: each plays its triad around the root of the
//! triangle held (or the base note with Run), each note in the octave
//! nearest it, so a walk across the lattice moves as little as it can.
//!
//! Each step lasts one to eight pulses and its gate decides what they play:
//! the first pulse (Single), every one (Every), one note held over them all
//! (Hold), none (Rest), the last, every other one, or more notes on each
//! pulse (Roll); a ratchet splits each note into one to eight. Steps can be
//! skipped.
//!
//! Its data (version 4): a version byte, sixteen steps of six bytes (note i8,
//! value, chance, ratchet - 1, pulses - 1, flags: the gate in the low three
//! bits, then tie, accent, skip), eight patch points (the instrument's
//! parameters the Touch view shows, u16 little-endian, 0xFFFF: none), the
//! shape's 32 points, each plate's value (24) and each step's triangle (16:
//! a + 8 in the low four bits, b + 4 in the next three, minor in the top
//! one). The first version (five steps of two bytes and six patch points),
//! the second (steps of four bytes: ratchets up to four and a rest, with or
//! without the plates' values) and the third (no triangles) are read too.

use super::SeqState;
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

pub const PULSE: usize = 0;
pub const RATE: usize = 1;
pub const DECAY: usize = 2;
pub const LATCH: usize = 3;
pub const STEPS: usize = 4;
pub const ORDER: usize = 5;
pub const PITCH: usize = 6;
pub const CHANCE: usize = 7;
pub const WANDER: usize = 8;
pub const GATE: usize = 9;
pub const BASE: usize = 10;
pub const PRESS_VEL: usize = 11;
pub const PLAY: usize = 16;
pub const VOICING: usize = 17;
pub const SWING: usize = 18;
pub const ARP_ORDER: usize = 19;
pub const OCTAVES: usize = 20;
pub const STRUM: usize = 21;
pub const ECHOES: usize = 22;
pub const FEEDBACK: usize = 23;
pub const RUN: usize = 24;
pub const LFO_SHAPE: usize = 25;
pub const LFO_RATE: usize = 26;
pub const SHAPE_RATE: usize = 27;
pub const LAYOUT: usize = 28;
pub const NOTES: usize = 29;
pub const SPRING: usize = 30;
pub const SEQ: usize = 31;
pub const TRIGGER: usize = 32;
pub const ENV_MODE: usize = 33;
pub const ENV_ATTACK: usize = 34;
pub const ENV_DECAY: usize = 35;
pub const RANDOM_STEPS: usize = 36;
/// The plates slide: a plate pressed and the finger moved bends its note
/// smoothly to the plate under it (the Touch view plays it so).
pub const SLIDE: usize = 37;

pub const RATES: &[&str] = &["1/32", "1/16 T", "1/16", "1/8 T", "1/8", "1/4 T", "1/4", "1/2", "1 Bar"];
const RATE_BEATS: [f64; 9] = [0.125, 1.0 / 6.0, 0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 1.0, 2.0, 4.0];
pub const ORDERS: &[&str] = &["Forward", "Pendulum", "Random", "Backward", "Drunk", "Every 3rd", "Every 5th"];
pub const PLAYS: &[&str] = &["Notes", "Arp", "Strum", "Echo", "Bounce"];
pub const ENV_MODES: &[&str] = &["Transient", "Sustained", "Cycle"];
pub const VOICINGS: &[&str] = &["Single", "+3rd", "+5th", "+Octave", "Triad", "Seventh", "Ninth", "Sus4", "Power", "Cluster", "Open"];
/// Each voicing's notes: (scale steps up, octaves up).
const VOICING_NOTES: [&[(i32, i32)]; 11] = [
    &[(0, 0)],
    &[(0, 0), (2, 0)],
    &[(0, 0), (4, 0)],
    &[(0, 0), (0, 1)],
    &[(0, 0), (2, 0), (4, 0)],
    &[(0, 0), (2, 0), (4, 0), (6, 0)],
    &[(0, 0), (2, 0), (4, 0), (6, 0), (8, 0)],
    &[(0, 0), (3, 0), (4, 0)],
    &[(0, 0), (4, 0), (0, 1)],
    &[(0, 0), (1, 0), (2, 0)],
    &[(0, -1), (4, 0), (2, 1)],
];
/// What a step plays on its pulses.
pub const GATES: &[&str] = &["Single", "Every", "Hold", "Rest", "Last", "Alternate", "Roll"];
pub const ARP_ORDERS: &[&str] = &["Up", "Down", "Up-Down", "Random", "As Played", "Converge"];
pub const LFO_SHAPES: &[&str] = &["Sine", "Triangle", "Saw", "Square", "S&H"];
pub const SLOW_RATES: &[&str] = &["1/16", "1/8", "1/4", "1/2", "1 Bar", "2 Bars", "4 Bars", "8 Bars"];
const SLOW_BEATS: [f64; 8] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
pub const LAYOUTS: &[&str] = &["Row", "Wide", "Grid", "Tonnetz"];
/// The layout that makes the plates the Tonnetz.
pub const TONNETZ_LAYOUT: usize = 3;
pub const NOTE_SETS: &[&str] = &["Key", "Pentatonic", "Chromatic"];
/// What the Touch modulators can take from it (the first six kept their places).
pub const OUTPUTS: &[&str] = &["Pressure", "Pulse", "Steps", "Random", "Drift", "Position", "LFO", "Shape", "X", "Y", "Env", "Plate"];
pub const N_OUT: usize = 12;
/// Plates with a value of their own.
pub const N_PLATES: usize = 24;
pub const N_STEPS: usize = 16;
pub const N_PATCH: usize = 8;
pub const N_SHAPE: usize = 32;
/// Plates held at once, notes sounding at once.
const MAX_HELD: usize = 16;
const MAX_SOUNDING: usize = 48;

/// Ratchets and pulses: at most this many.
pub const MAX_RATCHET: u8 = 8;
pub const MAX_PULSES: u8 = 8;

const VERSION: u8 = 4;
const STEP_BYTES: usize = 6;
const STEP_AT: usize = 1;
const PATCH_AT: usize = STEP_AT + N_STEPS * STEP_BYTES;
const SHAPE_AT: usize = PATCH_AT + N_PATCH * 2;
const PLATES_AT: usize = SHAPE_AT + N_SHAPE;
const TRI_AT: usize = PLATES_AT + N_PLATES;
const LEN: usize = TRI_AT + N_STEPS;
/// The third version: the same without the triangles.
const V3_LEN: usize = TRI_AT;
/// The second version: steps of four bytes, with the plates' values or (an
/// older one) without.
const V2_PATCH_AT: usize = STEP_AT + N_STEPS * 4;
const V2_PLATES_AT: usize = V2_PATCH_AT + N_PATCH * 2 + N_SHAPE;
const V2_LEN: usize = V2_PLATES_AT + N_PLATES;
/// The first version's length (five steps of two bytes, six patch points).
const V1_LEN: usize = 22;

pub static TOUCH: &[ParamSpec] = &[
    ParamSpec::toggle("Pulse", "Rhythm", false),
    ParamSpec::choice("Rate", "Rhythm", RATES, 4),
    ParamSpec::log("Decay", "Motion", 0.02, 2.0, 0.25, Unit::Sec),
    ParamSpec::toggle("Latch", "Play", false),
    ParamSpec::int("Steps", "Sequence", 1.0, 16.0, 8.0, Unit::None),
    ParamSpec::choice("Order", "Sequence", ORDERS, 0),
    ParamSpec::toggle("Pitch", "Sequence", true),
    ParamSpec::lin("Chance", "Rhythm", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Wander", "Rhythm", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Gate", "Rhythm", 0.05, 1.0, 0.6, Unit::Percent),
    ParamSpec::int("Base Note", "Plates", 24.0, 84.0, 48.0, Unit::None),
    ParamSpec::lin("Press Velocity", "Plates", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Play", "Play", PLAYS, 0),
    ParamSpec::choice("Voicing", "Play", VOICINGS, 0),
    ParamSpec::lin("Swing", "Rhythm", 0.0, 0.75, 0.0, Unit::Percent),
    ParamSpec::choice("Arp Order", "Play", ARP_ORDERS, 0),
    ParamSpec::int("Octaves", "Play", 1.0, 4.0, 1.0, Unit::None),
    ParamSpec::log("Strum", "Play", 0.005, 0.25, 0.04, Unit::Sec),
    ParamSpec::int("Repeats", "Play", 1.0, 8.0, 3.0, Unit::None),
    ParamSpec::lin("Fade", "Play", 0.1, 0.95, 0.6, Unit::Percent),
    ParamSpec::toggle("Run", "Play", false),
    ParamSpec::choice("LFO Shape", "Motion", LFO_SHAPES, 0),
    ParamSpec::choice("LFO Rate", "Motion", SLOW_RATES, 4),
    ParamSpec::choice("Shape Rate", "Motion", SLOW_RATES, 5),
    ParamSpec::choice("Layout", "Plates", LAYOUTS, 0),
    ParamSpec::choice("Notes", "Plates", NOTE_SETS, 0),
    ParamSpec::toggle("Spring", "Motion", true),
    ParamSpec::toggle("Sequencer", "Sequence", false),
    ParamSpec::toggle("Trigger", "Sequence", true),
    ParamSpec::choice("Env Mode", "Motion", ENV_MODES, 0),
    ParamSpec::log("Env Attack", "Motion", 0.001, 4.0, 0.01, Unit::Sec),
    ParamSpec::log("Env Decay", "Motion", 0.005, 8.0, 0.5, Unit::Sec),
    ParamSpec::int("Random Steps", "Motion", 0.0, 12.0, 0.0, Unit::None),
    ParamSpec::toggle("Slide", "Plates", true),
];

/// A triangle of the Tonnetz. Its corners are points (a, b) of the lattice,
/// `a` fifths to the right and `b` major thirds up and to the right from the
/// middle (the plate's note): the upward triangle (a, b), (a + 1, b),
/// (a, b + 1) is a major triad, the downward one (a + 1, b), (a, b + 1),
/// (a + 1, b + 1) a minor triad. The lattice repeats every three thirds up
/// and every four fifths less a third along.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Tri {
    pub a: i8,
    pub b: i8,
    pub minor: bool,
}

/// The neighbours across a triangle's sides: the parallel (C to Cm), the
/// leading-tone exchange (C to Em) and the relative (C to Am).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turn {
    P,
    L,
    R,
}

impl Tri {
    pub const fn new(a: i8, b: i8, minor: bool) -> Self {
        Self { a, b, minor }
    }

    /// Semitones from the middle to its corner (a, b).
    fn corner(self) -> i32 {
        7 * self.a as i32 + 4 * self.b as i32
    }

    /// Its root, in semitones above the middle (0..12).
    pub fn root(self) -> i32 {
        (self.corner() + if self.minor { 4 } else { 0 }).rem_euclid(12)
    }

    /// Its three notes, in semitones above the middle (0..12), root first.
    pub fn classes(self) -> [i32; 3] {
        let r = self.root();
        let third = if self.minor { 3 } else { 4 };
        [r, (r + third) % 12, (r + 7) % 12]
    }

    /// The triangle across one of its sides, the same chord moved back near
    /// the middle when it walks off the lattice shown.
    pub fn turn(self, t: Turn) -> Tri {
        let (a, b) = (self.a as i32, self.b as i32);
        let (a, b) = match (self.minor, t) {
            (false, Turn::P) => (a, b - 1),
            (false, Turn::L) => (a, b),
            (false, Turn::R) => (a - 1, b),
            (true, Turn::P) => (a, b + 1),
            (true, Turn::L) => (a, b),
            (true, Turn::R) => (a + 1, b),
        };
        let (a, b) = if (-4..=3).contains(&a) && (-2..=1).contains(&b) { (a, b) } else { fold(a, b) };
        Tri::new(a as i8, b as i8, !self.minor)
    }

    fn byte(self) -> u8 {
        ((self.a.clamp(-8, 7) + 8) as u8) | ((self.b.clamp(-4, 3) + 4) as u8) << 4 | (self.minor as u8) << 7
    }

    fn from_byte(v: u8) -> Tri {
        Tri::new((v & 15) as i8 - 8, ((v >> 4) & 7) as i8 - 4, v & 128 != 0)
    }
}

/// A point of the lattice moved to the same note near the middle.
fn fold(mut a: i32, mut b: i32) -> (i32, i32) {
    while a > 1 {
        a -= 4;
        b += 1;
    }
    while a < -2 {
        a += 4;
        b -= 1;
    }
    (a, (b + 1).rem_euclid(3) - 1)
}

/// The notes of triangle `t` around `center` (the plate's note), each in the
/// octave nearest it, low to high: triangles side by side share two notes
/// and keep them where they are.
pub fn triad(center: u8, t: Tri) -> [u8; 3] {
    let mut n = t.classes().map(|c| (center as i32 + (c + 6).rem_euclid(12) - 6).clamp(0, 127) as u8);
    n.sort_unstable();
    n
}

/// What a step plays on its pulses (in the order of `GATES`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Gate {
    /// A note on the first pulse.
    #[default]
    Single,
    /// A note on every pulse.
    Every,
    /// One note held over all its pulses.
    Hold,
    /// Nothing.
    Rest,
    /// A note on the last pulse.
    Last,
    /// A note on every other pulse, the first first.
    Alternate,
    /// A note on every pulse, each with one more ratchet than the last.
    Roll,
}

impl Gate {
    pub const ALL: [Gate; 7] = [Gate::Single, Gate::Every, Gate::Hold, Gate::Rest, Gate::Last, Gate::Alternate, Gate::Roll];

    pub fn from_index(i: usize) -> Gate {
        Gate::ALL.get(i).copied().unwrap_or_default()
    }

    pub fn index(self) -> usize {
        Gate::ALL.iter().position(|&g| g == self).unwrap_or(0)
    }

    pub fn name(self) -> &'static str {
        GATES[self.index()]
    }
}

/// One step of the sequence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    /// Scale steps up or down (-14..14).
    pub note: i32,
    /// The value it gives the Steps output (0..1).
    pub value: f32,
    /// How likely each of its notes plays (0..1).
    pub chance: f32,
    /// Notes each of its notes is split into (1..8).
    pub ratchet: u8,
    /// Pulses it lasts (1..8).
    pub pulses: u8,
    /// What it plays on them.
    pub gate: Gate,
    /// Held into the next step (legato).
    pub tie: bool,
    pub accent: bool,
    /// Passed over.
    pub skip: bool,
    /// Its triangle of the Tonnetz (with Tonnetz on, the triad it plays).
    pub tri: Tri,
}

impl Default for Step {
    fn default() -> Self {
        Self { note: 0, value: 0.0, chance: 1.0, ratchet: 1, pulses: 1, gate: Gate::Single, tie: false, accent: false, skip: false, tri: Tri::default() }
    }
}

impl Step {
    pub fn rests(&self) -> bool {
        self.gate == Gate::Rest
    }
}

fn step_bytes(s: &Step) -> [u8; STEP_BYTES] {
    let flags = s.gate.index() as u8 | (s.tie as u8) << 3 | (s.accent as u8) << 4 | (s.skip as u8) << 5;
    [
        s.note.clamp(-14, 14) as i8 as u8,
        (s.value.clamp(0.0, 1.0) * 255.0).round() as u8,
        (s.chance.clamp(0.0, 1.0) * 255.0).round() as u8,
        s.ratchet.clamp(1, MAX_RATCHET) - 1,
        s.pulses.clamp(1, MAX_PULSES) - 1,
        flags,
    ]
}

fn write_step(data: &mut [u8], i: usize, s: &Step) {
    let at = STEP_AT + i * STEP_BYTES;
    data[at..at + STEP_BYTES].copy_from_slice(&step_bytes(s));
    data[TRI_AT + i] = s.tri.byte();
}

fn write_patch(data: &mut [u8], k: usize, param: Option<u16>) {
    data[PATCH_AT + k * 2..PATCH_AT + k * 2 + 2].copy_from_slice(&param.unwrap_or(0xFFFF).to_le_bytes());
}

fn is_current(data: &[u8]) -> bool {
    data.len() == LEN && data[0] == VERSION
}

/// Where the patch points, the shape and the plates' values are in data of
/// the second or third version: (version, patch points, shape, plates).
fn layout(data: &[u8]) -> Option<(u8, usize, usize, Option<usize>)> {
    match (data.first().copied(), data.len()) {
        (Some(VERSION), LEN) => Some((4, PATCH_AT, SHAPE_AT, Some(PLATES_AT))),
        (Some(3), V3_LEN) => Some((3, PATCH_AT, SHAPE_AT, Some(PLATES_AT))),
        (Some(2), V2_LEN) => Some((2, V2_PATCH_AT, V2_PATCH_AT + N_PATCH * 2, Some(V2_PLATES_AT))),
        (Some(2), V2_PLATES_AT) => Some((2, V2_PATCH_AT, V2_PATCH_AT + N_PATCH * 2, None)),
        _ => None,
    }
}

/// A pattern's data from its steps (the rest of the sixteen plain), no patch
/// points, the shape a gentle rise and fall.
pub fn pattern(steps: &[Step]) -> Vec<u8> {
    let mut d = vec![0u8; LEN];
    d[0] = VERSION;
    for i in 0..N_STEPS {
        write_step(&mut d, i, &steps.get(i).copied().unwrap_or_default());
    }
    for k in 0..N_PATCH {
        write_patch(&mut d, k, None);
    }
    for j in 0..N_SHAPE {
        d[SHAPE_AT + j] = default_shape(j);
    }
    for k in 0..N_PLATES {
        d[PLATES_AT + k] = default_plate(k);
    }
    d
}

/// The plates' values at first: rising along them, an octave's worth at a time.
fn default_plate(k: usize) -> u8 {
    ((k % 8) as f32 / 7.0 * 255.0).round() as u8
}

/// Plate `k`'s value (0..1).
pub fn plate_value(data: &[u8], k: usize) -> f32 {
    let k = k % N_PLATES;
    match layout(data).and_then(|l| l.3) {
        Some(at) => data[at + k] as f32 / 255.0,
        None => default_plate(k) as f32 / 255.0,
    }
}

pub fn set_plate_value(data: &mut Vec<u8>, k: usize, v: f32) {
    whole(data);
    data[PLATES_AT + k % N_PLATES] = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
}

fn default_shape(j: usize) -> u8 {
    let t = j as f32 / N_SHAPE as f32;
    ((0.5 - 0.5 * (std::f32::consts::TAU * t).cos()) * 255.0).round() as u8
}

/// A pattern of notes (scale steps), with values rising along it.
pub fn notes(ns: &[i32]) -> Vec<u8> {
    let steps: Vec<Step> = ns.iter().enumerate().map(|(i, &n)| Step { note: n, value: i as f32 / ns.len().max(1) as f32, ..Step::default() }).collect();
    pattern(&steps)
}

/// What it starts with: a little tune, accents on the ones; on the Tonnetz
/// I, vi, IV and V over and over.
pub fn default_data() -> Vec<u8> {
    let ns = [0, 2, 4, 2, 7, 4, 2, -1, 0, 4, 2, 5, 4, 2, 0, -3];
    let tris = [Tri::new(0, 0, false), Tri::new(-1, 0, true), Tri::new(-1, 0, false), Tri::new(1, 0, false)];
    let steps: Vec<Step> = ns.iter().enumerate().map(|(i, &n)| Step { note: n, value: ((i * 5) % 16) as f32 / 15.0, accent: i % 8 == 0, tri: tris[i % 4], ..Step::default() }).collect();
    pattern(&steps)
}

/// Data of any version made the current one, keeping what it had.
pub fn whole(data: &mut Vec<u8>) {
    if is_current(data) {
        return;
    }
    let old = std::mem::take(data);
    *data = default_data();
    if layout(&old).is_none() && old.len() != V1_LEN {
        return;
    }
    for i in 0..N_STEPS {
        write_step(data, i, &step(&old, i));
    }
    for k in 0..N_PATCH {
        write_patch(data, k, patch(&old, k));
    }
    for j in 0..N_SHAPE {
        data[SHAPE_AT + j] = (shape(&old, j) * 255.0).round() as u8;
    }
    for k in 0..N_PLATES {
        data[PLATES_AT + k] = (plate_value(&old, k) * 255.0).round() as u8;
    }
}

/// Step `i` (of any version's data).
pub fn step(data: &[u8], i: usize) -> Step {
    match layout(data) {
        Some((v @ (3 | 4), ..)) if i < N_STEPS => {
            let b = &data[STEP_AT + i * STEP_BYTES..STEP_AT + (i + 1) * STEP_BYTES];
            Step {
                tri: if v == 4 { Tri::from_byte(data[TRI_AT + i]) } else { Tri::default() },
                note: ((b[0] as i8) as i32).clamp(-14, 14),
                value: b[1] as f32 / 255.0,
                chance: b[2] as f32 / 255.0,
                ratchet: (b[3] + 1).min(MAX_RATCHET),
                pulses: (b[4] + 1).min(MAX_PULSES),
                gate: Gate::from_index((b[5] & 7) as usize),
                tie: b[5] & 8 != 0,
                accent: b[5] & 16 != 0,
                skip: b[5] & 32 != 0,
            }
        }
        Some((2, ..)) if i < N_STEPS => {
            // Ratchets up to four and a rest.
            let b = &data[STEP_AT + i * 4..STEP_AT + i * 4 + 4];
            Step {
                note: ((b[0] as i8) as i32).clamp(-14, 14),
                value: b[1] as f32 / 255.0,
                chance: b[2] as f32 / 255.0,
                ratchet: (b[3] & 3) + 1,
                gate: if b[3] & 4 != 0 { Gate::Rest } else { Gate::Single },
                tie: b[3] & 8 != 0,
                accent: b[3] & 16 != 0,
                ..Step::default()
            }
        }
        None if data.len() == V1_LEN && i < 5 => Step { note: ((data[i * 2] as i8) as i32).clamp(-7, 7), value: data[i * 2 + 1] as f32 / 255.0, ..Step::default() },
        _ => Step::default(),
    }
}

pub fn set_step(data: &mut Vec<u8>, i: usize, s: Step) {
    whole(data);
    if i < N_STEPS {
        write_step(data, i, &s);
    }
}

/// The instrument parameter on patch point `k`, if one is set.
pub fn patch(data: &[u8], k: usize) -> Option<u16> {
    let at = match layout(data) {
        Some((_, at, ..)) if k < N_PATCH => at + k * 2,
        None if data.len() == V1_LEN && k < 6 => 10 + k * 2,
        _ => return None,
    };
    let v = u16::from_le_bytes([data[at], data[at + 1]]);
    (v != 0xFFFF).then_some(v)
}

pub fn set_patch(data: &mut Vec<u8>, k: usize, param: Option<u16>) {
    whole(data);
    if k < N_PATCH {
        write_patch(data, k, param);
    }
}

/// The drawn shape's point `j` (0..1).
pub fn shape(data: &[u8], j: usize) -> f32 {
    let j = j % N_SHAPE;
    match layout(data) {
        Some((_, _, at, _)) => data[at + j] as f32 / 255.0,
        None => default_shape(j) as f32 / 255.0,
    }
}

pub fn set_shape(data: &mut Vec<u8>, j: usize, v: f32) {
    whole(data);
    data[SHAPE_AT + j % N_SHAPE] = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
}

/// The shape at `phase` (0..1), smoothly between its points.
pub fn shape_at(data: &[u8], phase: f32) -> f32 {
    let x = phase.rem_euclid(1.0) * N_SHAPE as f32;
    let j = x.floor() as usize;
    let f = x - j as f32;
    shape(data, j) * (1.0 - f) + shape(data, j + 1) * f
}

/// What the screen is told: the step, how many steps, the pulses so far,
/// the note played last, whether the sequencer runs, which of its step's
/// pulses it is at, and the outputs.
pub struct Seen {
    pub step: usize,
    pub steps: usize,
    pub pulses: u32,
    pub last_note: u8,
    pub seq: bool,
    pub pulse: usize,
    pub outs: [f32; N_OUT],
}

pub fn read_visual(v: [u64; 4]) -> Seen {
    let outs = std::array::from_fn(|k| {
        let (word, at) = if k < 8 { (v[1], k) } else { (v[2], k - 8) };
        ((word >> (at * 8)) & 0xFF) as f32 / 255.0
    });
    Seen { step: (v[0] & 0xFF) as usize, steps: ((v[0] >> 8) & 0xFF) as usize, pulses: ((v[0] >> 16) & 0xFFFF) as u32, last_note: ((v[0] >> 32) & 0x7F) as u8, seq: (v[0] >> 40) & 1 == 1, pulse: ((v[0] >> 41) & 7) as usize, outs }
}

/// A small fixed set of notes (the audio thread never allocates).
#[derive(Clone, Copy)]
struct Notes<const N: usize> {
    n: usize,
    v: [u8; N],
}

impl<const N: usize> Default for Notes<N> {
    fn default() -> Self {
        Self { n: 0, v: [0; N] }
    }
}

impl<const N: usize> Notes<N> {
    fn remove(&mut self, x: u8) {
        if let Some(i) = self.v[..self.n].iter().position(|&y| y == x) {
            self.v.copy_within(i + 1..self.n, i);
            self.n -= 1;
        }
    }

    fn push(&mut self, x: u8) {
        self.remove(x);
        if self.n == N {
            self.v.copy_within(1..N, 0);
            self.n -= 1;
        }
        self.v[self.n] = x;
        self.n += 1;
    }

    fn last(&self) -> Option<u8> {
        self.n.checked_sub(1).map(|i| self.v[i])
    }

    fn contains(&self, x: u8) -> bool {
        self.v[..self.n].contains(&x)
    }

    fn as_slice(&self) -> &[u8] {
        &self.v[..self.n]
    }
}

pub struct TouchState {
    /// The plates held, the last pressed last.
    held: Notes<MAX_HELD>,
    /// The plates of the last touch (a chord pressed together), for Latch.
    latched: Notes<MAX_HELD>,
    /// Without the pulser: what each plate sounds, (plate, note).
    sounding: [(u8, u8); MAX_SOUNDING],
    n_sounding: usize,
    /// The beat it is at when the song does not run.
    clock: f64,
    last_pulse: Option<i64>,
    step: usize,
    /// Pulses of the step so far, and how many it lasts.
    pulse: u8,
    of: u8,
    backwards: bool,
    arp: usize,
    strum_up: bool,
    /// The outputs: the pulse's envelope, the stepped and drifting random, the LFO's held value.
    env: f32,
    random: f32,
    drift: f32,
    drift_to: f32,
    drift_wait: f32,
    hold: f32,
    hold_cycle: i64,
    /// The envelope: its level, and rising (or not).
    env2: f32,
    env2_up: bool,
    /// The pressure the last note was played with (a latched plate keeps it).
    pressure: f32,
    pulses: u32,
    last_note: u8,
}

impl Default for TouchState {
    fn default() -> Self {
        Self {
            held: Notes::default(),
            latched: Notes::default(),
            sounding: [(0, 0); MAX_SOUNDING],
            n_sounding: 0,
            clock: 0.0,
            last_pulse: None,
            // The first step taken is the first.
            step: N_STEPS - 1,
            pulse: 0,
            of: 0,
            backwards: false,
            arp: 0,
            strum_up: false,
            env: 0.0,
            random: 0.0,
            drift: 0.0,
            drift_to: 0.0,
            drift_wait: 0.0,
            hold: 0.0,
            hold_cycle: i64::MIN,
            env2: 0.0,
            env2_up: false,
            pressure: 0.0,
            pulses: 0,
            last_note: 0,
        }
    }
}

impl TouchState {
    fn remember(&mut self, plate: u8, note: u8) {
        if self.n_sounding == MAX_SOUNDING {
            self.sounding.copy_within(1..MAX_SOUNDING, 0);
            self.n_sounding -= 1;
        }
        self.sounding[self.n_sounding] = (plate, note);
        self.n_sounding += 1;
    }

    /// Takes out what a plate sounds; its notes into `notes`.
    fn forget(&mut self, plate: u8, notes: &mut Notes<MAX_SOUNDING>) {
        let mut i = 0;
        while i < self.n_sounding {
            if self.sounding[i].0 == plate {
                notes.push(self.sounding[i].1);
                self.sounding.copy_within(i + 1..self.n_sounding, i);
                self.n_sounding -= 1;
            } else {
                i += 1;
            }
        }
    }
}

fn unit(rng: &mut crate::dsp::Rng) -> f32 {
    (rng.bipolar() + 1.0) * 0.5
}

/// A note played: when (frame), which, how hard, how long (frames).
#[derive(Clone, Copy)]
struct Hit {
    at: u64,
    note: u8,
    vel: f32,
    len: f64,
}

/// What a trigger plays, at most this many notes.
const MAX_HITS: usize = 64;

struct Hits {
    n: usize,
    v: [Hit; MAX_HITS],
}

impl Hits {
    fn new() -> Self {
        Self { n: 0, v: [Hit { at: 0, note: 0, vel: 0.0, len: 0.0 }; MAX_HITS] }
    }

    fn push(&mut self, h: Hit) {
        if self.n < MAX_HITS {
            self.v[self.n] = h;
            self.n += 1;
        }
    }
}

/// What a step lets through on a pulse: notes split in `ratchet`, louder,
/// held into the next step, held over its pulses, and how many pulses its
/// note covers.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Gates {
    ratchet: u8,
    accent: bool,
    tie: bool,
    hold: bool,
    span: u8,
}

impl Gates {
    const PLAIN: Gates = Gates { ratchet: 1, accent: false, tie: false, hold: false, span: 1 };

    /// How long its notes are (`len` as the gate knob sets it): a tie into
    /// the next step, a held note over its pulses.
    fn length(&self, interval: f64, len: f64) -> f64 {
        if self.tie {
            interval * (self.span as f64 + 0.05)
        } else if self.hold {
            interval * self.span as f64 * 0.97
        } else {
            len
        }
    }
}

/// A step's pulse `k` (of `of`): what its gate plays there.
fn gates_of(st: &Step, k: u8, of: u8) -> Option<Gates> {
    let last = k + 1 >= of;
    let left = of.saturating_sub(k).max(1);
    let r = st.ratchet.clamp(1, MAX_RATCHET);
    let (ratchet, hold, span) = match st.gate {
        Gate::Rest => return None,
        Gate::Single if k == 0 => (r, false, left),
        Gate::Every => (r, false, 1),
        Gate::Hold if k == 0 => (1, true, left),
        Gate::Last if last => (r, false, 1),
        Gate::Alternate if k % 2 == 0 => (r, false, left.min(2)),
        Gate::Roll => ((r + k).min(MAX_RATCHET), false, 1),
        _ => return None,
    };
    Some(Gates { ratchet, accent: st.accent, tie: st.tie, hold, span })
}

/// What a step plays on its pulse `k`: how many notes (0: none), and
/// whether that note is held over the rest of its pulses.
pub fn plays(st: &Step, k: u8) -> (u8, bool) {
    gates_of(st, k, st.pulses.clamp(1, MAX_PULSES)).map(|g| (g.ratchet, g.hold)).unwrap_or((0, false))
}

/// The touches among the plates pressed in a block: each plate its own, or
/// (on the Tonnetz, a triangle's notes come together) all of them one, as
/// loud as the loudest.
fn touches(presses: &[(u8, f32)], together: bool) -> ([(Notes<MAX_HELD>, f32); MAX_HELD], usize) {
    let one = |ps: &[(u8, f32)]| {
        let mut n: Notes<MAX_HELD> = Notes::default();
        ps.iter().for_each(|&(pl, _)| n.push(pl));
        (n, ps.iter().map(|x| x.1).fold(0.0f32, f32::max))
    };
    let mut out = [(Notes::default(), 0.0); MAX_HELD];
    if together && !presses.is_empty() {
        out[0] = one(presses);
        return (out, 1);
    }
    let n = presses.len().min(MAX_HELD);
    for (k, x) in presses[..n].iter().enumerate() {
        out[k] = one(std::slice::from_ref(x));
    }
    (out, n)
}

fn param(p: &[f32], i: usize, or: f32) -> f32 {
    p.get(i).copied().unwrap_or(or)
}

fn choice(p: &[f32], i: usize, n: usize) -> usize {
    (param(p, i, 0.0).round().max(0.0) as usize).min(n - 1)
}

/// Whether the plates are the Tonnetz.
fn tonnetz(p: &[f32]) -> bool {
    choice(p, LAYOUT, LAYOUTS.len()) == TONNETZ_LAYOUT
}

fn rate_beats(p: &[f32]) -> f64 {
    RATE_BEATS[choice(p, RATE, RATE_BEATS.len())]
}

fn gate(p: &[f32]) -> f64 {
    param(p, GATE, 0.6).clamp(0.05, 1.0) as f64
}

impl SeqState {
    /// One block of Touch: the plates that came in, the clock, the outputs.
    pub(crate) fn touch(&mut self, p: &[f32], data: &[u8], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        self.shift_in_note = false;
        let arp = choice(p, PLAY, PLAYS.len()) == 1;
        let pulse_on = param(p, PULSE, 0.0) >= 0.5;
        let clocked = pulse_on || arp;
        let latch = param(p, LATCH, 0.0) >= 0.5;
        let seq = param(p, SEQ, 0.0) >= 0.5;
        let rate = rate_beats(p);
        let spb = ctx.samples_per_beat.max(1.0);
        let interval = rate * spb;
        let pressure = ctx.touch[0].clamp(0.0, 1.0);
        let press_vel = param(p, PRESS_VEL, 1.0).clamp(0.0, 1.0);
        let vel_of = |pressed: f32| (0.8 * (1.0 - press_vel) + pressed.clamp(0.05, 1.0) * press_vel).clamp(0.05, 1.0);

        // The plates that came in.
        let mut presses = [(0u8, 0f32); MAX_HELD];
        let mut n_press = 0;
        let mut releases = [0u8; MAX_HELD];
        let mut n_release = 0;
        for e in out.iter() {
            if e.on {
                if self.touch.held.n == 0 {
                    // A new touch: what is latched now is this one.
                    self.touch.latched = Notes::default();
                }
                self.touch.held.push(e.pitch);
                self.touch.latched.push(e.pitch);
                if n_press < MAX_HELD {
                    presses[n_press] = (e.pitch, e.vel);
                    n_press += 1;
                }
            } else {
                self.touch.held.remove(e.pitch);
                if n_release < MAX_HELD {
                    releases[n_release] = e.pitch;
                    n_release += 1;
                }
            }
        }
        out.clear();
        if self.touch.held.n > 0 {
            self.touch.pressure = pressure.max(0.05);
        }
        let b0 = if ctx.playing { ctx.beat } else { self.touch.clock };
        let b1 = b0 + ctx.len as f64 / spb;
        self.touch.clock = b1;

        if !clocked {
            // Let go: the plate stops (Latch: when the next touch comes).
            let mut ending: Notes<MAX_SOUNDING> = Notes::default();
            if !latch {
                for &plate in &releases[..n_release] {
                    self.touch.forget(plate, &mut ending);
                }
                // Latch turned off: the plates no longer held stop.
                let mut i = 0;
                while i < self.touch.n_sounding {
                    let plate = self.touch.sounding[i].0;
                    if self.touch.held.contains(plate) {
                        i += 1;
                    } else {
                        self.touch.forget(plate, &mut ending);
                    }
                }
            } else if n_press > 0 {
                while self.touch.n_sounding > 0 {
                    let plate = self.touch.sounding[0].0;
                    self.touch.forget(plate, &mut ending);
                }
            }
            self.touch_end_all(&ending, out);
            // Each touch plays (and moves the sequence a step).
            let (groups, n_groups) = touches(&presses[..n_press], tonnetz(p));
            for &(plates, vel) in &groups[..n_groups] {
                let mut hits = Hits::new();
                let last = plates.last().unwrap_or(0);
                self.touch_trigger(p, data, ctx, ctx.now, plates.as_slice(), vel_of(vel), 1e12, false, &mut hits);
                for h in &hits.v[..hits.n] {
                    if h.len >= 1e11 {
                        // Its own plate's note, or (a chord made from it) the touch's last plate's.
                        self.touch.remember(if plates.contains(h.note) { h.note } else { last }, h.note);
                    }
                    self.touch_play(ctx, *h, out);
                }
            }
        } else {
            // Notes held from before the pulser came on end now.
            let mut ending: Notes<MAX_SOUNDING> = Notes::default();
            while self.touch.n_sounding > 0 {
                let plate = self.touch.sounding[0].0;
                self.touch.forget(plate, &mut ending);
            }
            self.touch_end_all(&ending, out);
            // A touch plays at once (an arpeggio waits for its clock).
            if !arp {
                let (groups, n_groups) = touches(&presses[..n_press], tonnetz(p));
                for &(plates, vel) in &groups[..n_groups] {
                    let mut hits = Hits::new();
                    self.touch_trigger(p, data, ctx, ctx.now, plates.as_slice(), vel_of(vel), interval * gate(p), true, &mut hits);
                    for h in &hits.v[..hits.n] {
                        self.touch_play(ctx, *h, out);
                    }
                }
            }
            // The pulses, the odd ones later by the swing.
            let sw = param(p, SWING, 0.0).clamp(0.0, 0.75) as f64 * rate * 0.5;
            let mut k = ((b0 - sw) / rate).floor() as i64 - 1;
            while (k as f64) * rate < b1 {
                let t = k as f64 * rate + if k.rem_euclid(2) == 1 { sw } else { 0.0 };
                if t >= b0 - 1e-9 && t < b1 - 1e-9 && Some(k) != self.touch.last_pulse {
                    self.touch.last_pulse = Some(k);
                    let at = ctx.now + ((t - b0).max(0.0) * spb).round() as u64;
                    let vel = vel_of(if self.touch.held.n > 0 { pressure } else { self.touch.pressure });
                    self.touch_pulse(p, data, ctx, at, latch, vel, out);
                }
                k += 1;
            }
        }
        // Echoes, strums and ratchets that are due.
        self.flush_pending(ctx, out);

        // The outputs, at the end of the block.
        let secs = ctx.len as f32 / ctx.sr.max(1.0);
        self.touch.env *= (-secs / param(p, DECAY, 0.25).max(0.005)).exp();
        if !clocked {
            self.touch.drift_wait -= secs;
            if self.touch.drift_wait <= 0.0 {
                self.touch.drift_wait = 1.5;
                self.touch.drift_to = unit(&mut self.rng);
            }
        }
        let tau = if clocked { (interval / ctx.sr.max(1.0) as f64) as f32 * 2.0 } else { 1.2 };
        self.touch.drift += (self.touch.drift_to - self.touch.drift) * (1.0 - (-secs / tau.max(0.05)).exp());
        let lfo_beats = SLOW_BEATS[choice(p, LFO_RATE, SLOW_BEATS.len())];
        let ph = (b1 / lfo_beats).rem_euclid(1.0) as f32;
        let lfo = match choice(p, LFO_SHAPE, LFO_SHAPES.len()) {
            0 => 0.5 - 0.5 * (std::f32::consts::TAU * ph).cos(),
            1 => 1.0 - (2.0 * ph - 1.0).abs(),
            2 => ph,
            3 => (ph < 0.5) as u8 as f32,
            _ => {
                let cycle = (b1 / lfo_beats).floor() as i64;
                if cycle != self.touch.hold_cycle {
                    self.touch.hold_cycle = cycle;
                    self.touch.hold = unit(&mut self.rng);
                }
                self.touch.hold
            }
        };
        let shape_beats = SLOW_BEATS[choice(p, SHAPE_RATE, SLOW_BEATS.len())];
        let drawn = shape_at(data, (b1 / shape_beats).rem_euclid(1.0) as f32);
        let steps = (param(p, STEPS, 8.0).round() as usize).clamp(1, N_STEPS);
        let shown = self.touch.step.min(steps - 1);
        let value = if seq { step(data, shown).value } else { 0.0 };
        // The envelope: once (Transient), held while a plate is (Sustained), or round and round (Cycle).
        let mode = choice(p, ENV_MODE, ENV_MODES.len());
        let rise = secs / param(p, ENV_ATTACK, 0.01).max(0.001);
        let fall = (-4.6 * secs / param(p, ENV_DECAY, 0.5).max(0.005)).exp();
        let t = &mut self.touch;
        if t.env2_up {
            t.env2 = (t.env2 + rise).min(1.0);
            if t.env2 >= 1.0 && !(mode == 1 && t.held.n > 0) {
                t.env2_up = false;
            }
        } else {
            t.env2 *= fall;
            if mode == 2 && t.env2 < 0.01 {
                t.env2_up = true;
            }
        }
        // Random Steps: the random on that many levels (0: as it comes).
        let levels = param(p, RANDOM_STEPS, 0.0).round().clamp(0.0, 12.0);
        let random = if levels >= 2.0 { (t.random * levels).floor().min(levels - 1.0) / (levels - 1.0) } else { t.random };
        let outs = [pressure, t.env, value, random, t.drift, ctx.touch[1].clamp(0.0, 1.0), lfo, drawn, ctx.touch[2].clamp(0.0, 1.0), ctx.touch[3].clamp(0.0, 1.0), t.env2, ctx.touch[4].clamp(0.0, 1.0)];
        self.touch_out = outs;
        let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u64;
        self.visual[0] = shown as u64 | (steps as u64) << 8 | ((self.touch.pulses & 0xFFFF) as u64) << 16 | ((self.touch.last_note & 0x7F) as u64) << 32 | (seq as u64) << 40 | ((self.touch.pulse.saturating_sub(1) & 7) as u64) << 41;
        self.visual[1] = (0..8).fold(0u64, |a, k| a | byte(outs[k]) << (k * 8));
        self.visual[2] = (8..N_OUT).fold(0u64, |a, k| a | byte(outs[k]) << ((k - 8) * 8));
    }

    /// A pulse: what plays (the plate held last, the latched one, or with Run
    /// the base note), chance, the sequence a step further, the outputs.
    #[allow(clippy::too_many_arguments)]
    fn touch_pulse(&mut self, p: &[f32], data: &[u8], ctx: &MidiCtx, at: u64, latch: bool, vel: f32, out: &mut Vec<MidiEvent>) {
        let run = param(p, RUN, 0.0) >= 0.5;
        let arp = choice(p, PLAY, PLAYS.len()) == 1;
        let source = self.touch.held.last().or(if latch { self.touch.latched.last() } else { None }).or(if run { Some(param(p, BASE, 48.0).round().clamp(0.0, 127.0) as u8) } else { None });
        let chance = param(p, CHANCE, 1.0);
        let rests = chance < 1.0 && unit(&mut self.rng) >= chance;
        let Some(plate) = source.filter(|_| !rests) else {
            // Nothing to play (or chance rests): the sequence and the outputs still move.
            self.touch_step(p, data);
            self.touch_outputs();
            return;
        };
        let mut hits = Hits::new();
        let len = rate_beats(p) * ctx.samples_per_beat * gate(p);
        if arp {
            self.touch_arp(p, data, ctx, at, plate, vel, len, latch, run, &mut hits);
        } else {
            // On the Tonnetz the chord held (or latched) pulses whole.
            let mut plates: Notes<MAX_HELD> = Notes::default();
            if tonnetz(p) && self.touch.held.n > 0 {
                plates = self.touch.held;
            } else if tonnetz(p) && latch && self.touch.latched.n > 0 {
                plates = self.touch.latched;
            } else {
                plates.push(plate);
            }
            self.touch_trigger(p, data, ctx, at, plates.as_slice(), vel, len, true, &mut hits);
        }
        for h in &hits.v[..hits.n] {
            self.touch_play(ctx, *h, out);
        }
    }

    /// What a touch or a pulse plays from its plates (one, or a triangle's
    /// three on the Tonnetz, the last the one that counts): the step decides
    /// (rest, chance, ratchets, accent, tie), the voicing (or the step's
    /// triangle) makes it a chord, the play mode strums it or adds echoes.
    #[allow(clippy::too_many_arguments)]
    fn touch_trigger(&mut self, p: &[f32], data: &[u8], ctx: &MidiCtx, at: u64, plates: &[u8], vel: f32, len: f64, clocked: bool, hits: &mut Hits) {
        let Some(&plate) = plates.last() else { return };
        let interval = rate_beats(p) * ctx.samples_per_beat;
        let st = self.touch_step(p, data);
        self.touch_outputs();
        let Some(g) = self.touch_gates(p, st) else { return };
        let vel = if g.accent { (vel * 1.3).min(1.0) } else { vel };
        // A tie holds into the next step, Hold over the step's pulses; ratchets split the pulse.
        let len = if clocked { g.length(interval, len) } else { len };
        let ratchet = if clocked { g.ratchet } else { 1 };
        let sub = interval / ratchet as f64;
        let rlen = if ratchet > 1 { (sub * gate(p)).min(len) } else { len };
        let shift = self.touch_shift(p, st.map(|s| s.0));
        let chord = self.touch_chord(ctx, p, plates, shift, st.map(|s| s.0));
        let play = choice(p, PLAY, PLAYS.len());
        let strum = (param(p, STRUM, 0.04).clamp(0.005, 0.25) * ctx.sr) as u64;
        self.touch.strum_up = !self.touch.strum_up;
        let up = self.touch.strum_up;
        let echoes = param(p, ECHOES, 3.0).round().clamp(1.0, 8.0) as u32;
        let feedback = param(p, FEEDBACK, 0.6).clamp(0.1, 0.95);
        // Bounce: the first bounce a pulse later, each one after three quarters of the last.
        let bounce_gaps = |e: u32| (0..e).fold(0.0f64, |a, j| a + interval * 0.75f64.powi(j as i32));
        let n = chord.n;
        for r in 0..ratchet {
            let rat = at + (r as f64 * sub) as u64;
            for (j, &note) in chord.as_slice().iter().enumerate() {
                // Strum: one after the other, up and down in turn.
                let k = if up { j } else { n - 1 - j } as u64;
                let start = if play == 2 { rat + k * strum } else { rat };
                hits.push(Hit { at: start, note, vel, len: rlen });
                if play == 3 {
                    // Echo: again on the following pulses, fainter each time.
                    for e in 1..=echoes {
                        hits.push(Hit { at: start + (e as f64 * interval) as u64, note, vel: vel * feedback.powi(e as i32), len: (interval * gate(p)).min(len) });
                    }
                } else if play == 4 {
                    // Bounce: sooner and softer each time, like a ball dropped.
                    for e in 1..=echoes + 3 {
                        let gap = interval * 0.75f64.powi(e as i32 - 1);
                        hits.push(Hit { at: start + bounce_gaps(e) as u64, note, vel: vel * feedback.powi(e as i32), len: (gap * gate(p)).min(len) });
                    }
                }
            }
        }
        self.touch.last_note = chord.as_slice().first().copied().unwrap_or(plate);
    }

    /// The notes a touch's plates play on a step: the last plate's voicing in
    /// the key, moved by the step. On the Tonnetz the plates themselves (a
    /// triangle), or with the sequence moving the notes its step's triad
    /// around the last plate (the triangle's root).
    fn touch_chord(&self, ctx: &MidiCtx, p: &[f32], plates: &[u8], shift: i32, st: Option<Step>) -> Notes<MAX_SOUNDING> {
        let mut chord: Notes<MAX_SOUNDING> = Notes::default();
        let Some(&plate) = plates.last() else { return chord };
        if tonnetz(p) {
            match st.filter(|_| param(p, PITCH, 1.0) >= 0.5) {
                Some(s) => triad(plate, s.tri).into_iter().for_each(|n| chord.push(n)),
                None => {
                    let mut own = [0u8; MAX_HELD];
                    let k = plates.len().min(MAX_HELD);
                    own[..k].copy_from_slice(&plates[..k]);
                    own[..k].sort_unstable();
                    own[..k].iter().for_each(|&n| chord.push(n));
                }
            }
            return chord;
        }
        for &(steps, octs) in VOICING_NOTES[choice(p, VOICING, VOICING_NOTES.len())] {
            chord.push(self.touch_pitch(ctx, plate, shift + steps, octs));
        }
        chord
    }

    /// An arpeggio's next note from the plates held (or latched): their
    /// voicings over the octaves.
    #[allow(clippy::too_many_arguments)]
    fn touch_arp(&mut self, p: &[f32], data: &[u8], ctx: &MidiCtx, at: u64, plate: u8, vel: f32, len: f64, latch: bool, run: bool, hits: &mut Hits) {
        let st = self.touch_step(p, data);
        self.touch_outputs();
        let Some(g) = self.touch_gates(p, st) else { return };
        let shift = self.touch_shift(p, st.map(|s| s.0));
        let octaves = param(p, OCTAVES, 1.0).round().clamp(1.0, 4.0) as i32;
        let plates = if self.touch.held.n > 0 {
            self.touch.held
        } else if latch && self.touch.latched.n > 0 {
            self.touch.latched
        } else {
            let mut one: Notes<MAX_HELD> = Notes::default();
            if run {
                one.push(plate);
            }
            one
        };
        // The pool: every note of every plate (on the Tonnetz: of the triangle), in the octaves above.
        let order = choice(p, ARP_ORDER, ARP_ORDERS.len());
        let mut pool: Notes<MAX_SOUNDING> = Notes::default();
        let whole = tonnetz(p);
        for o in 0..octaves {
            for c in 0..if whole { 1 } else { plates.n } {
                let these = if whole { plates.as_slice() } else { &plates.as_slice()[c..c + 1] };
                for &note in self.touch_chord(ctx, p, these, shift, st.map(|s| s.0)).as_slice() {
                    pool.push((note as i32 + 12 * o).clamp(0, 127) as u8);
                }
            }
        }
        let n = pool.n;
        if n == 0 {
            return;
        }
        if order != 4 {
            pool.v[..n].sort_unstable();
        }
        let i = self.touch.arp;
        self.touch.arp = self.touch.arp.wrapping_add(1);
        let idx = match order {
            1 => n - 1 - i % n,
            2 if n > 1 => {
                let period = 2 * n - 2;
                let k = i % period;
                if k < n {
                    k
                } else {
                    period - k
                }
            }
            3 => ((unit(&mut self.rng) * n as f32) as usize).min(n - 1),
            5 => {
                // Converge: the lowest, the highest, the next lowest...
                let k = i % n;
                if k % 2 == 0 {
                    k / 2
                } else {
                    n - 1 - k / 2
                }
            }
            _ => i % n,
        };
        let note = pool.v[idx];
        let vel = if g.accent { (vel * 1.3).min(1.0) } else { vel };
        let interval = rate_beats(p) * ctx.samples_per_beat;
        let len = g.length(interval, len);
        let ratchet = g.ratchet;
        let sub = interval / ratchet as f64;
        for r in 0..ratchet {
            let rlen = if ratchet > 1 { (sub * gate(p)).min(len) } else { len };
            hits.push(Hit { at: at + (r as f64 * sub) as u64, note, vel, len: rlen });
        }
        self.touch.last_note = note;
    }

    /// A pulse of the sequence (when it runs): a step further once the
    /// step's pulses are done (past the steps skipped), and that step with
    /// which of its pulses this is and how many it lasts.
    fn touch_step(&mut self, p: &[f32], data: &[u8]) -> Option<(Step, u8, u8)> {
        if param(p, SEQ, 0.0) < 0.5 {
            return None;
        }
        let n = (param(p, STEPS, 8.0).round() as usize).clamp(1, N_STEPS);
        if self.touch.pulse >= self.touch.of || self.touch.step >= n {
            // The next step not skipped (all skipped: the next one anyway).
            for _ in 0..n {
                self.touch_advance(p);
                if !step(data, self.touch.step.min(n - 1)).skip {
                    break;
                }
            }
            self.touch.pulse = 0;
            self.touch.of = step(data, self.touch.step.min(n - 1)).pulses.clamp(1, MAX_PULSES);
        }
        let k = self.touch.pulse;
        self.touch.pulse += 1;
        Some((step(data, self.touch.step.min(n - 1)), k, self.touch.of))
    }

    /// What a step lets through on a pulse when it decides (Trigger), or
    /// None: its gate plays nothing there (or its chance leaves the note out).
    fn touch_gates(&mut self, p: &[f32], st: Option<(Step, u8, u8)>) -> Option<Gates> {
        let Some((st, k, of)) = st.filter(|_| param(p, TRIGGER, 1.0) >= 0.5) else { return Some(Gates::PLAIN) };
        let g = gates_of(&st, k, of)?;
        if st.chance < 1.0 && unit(&mut self.rng) >= st.chance {
            return None;
        }
        Some(g)
    }

    /// How many scale steps a note moves: the step (Pitch), and Wander.
    fn touch_shift(&mut self, p: &[f32], st: Option<Step>) -> i32 {
        let mut shift = 0;
        if let Some(st) = st.filter(|_| param(p, PITCH, 1.0) >= 0.5) {
            shift += st.note;
        }
        let wander = param(p, WANDER, 0.0);
        if wander > 0.0 && unit(&mut self.rng) < wander {
            let by = if unit(&mut self.rng) < 0.7 { 1 } else { 2 };
            shift += if unit(&mut self.rng) < 0.5 { by } else { -by };
        }
        shift
    }

    fn touch_pitch(&self, ctx: &MidiCtx, plate: u8, steps: i32, octaves: i32) -> u8 {
        let base = if steps == 0 {
            plate as i32
        } else {
            let (d, rest) = ctx.scale.locate(plate as i32);
            ctx.scale.pitch_at(d + steps) + rest
        };
        (base + 12 * octaves).clamp(0, 127) as u8
    }

    /// A pulse (or a touch): the envelope again, a new random.
    fn touch_outputs(&mut self) {
        self.touch.env = 1.0;
        self.touch.env2_up = true;
        self.touch.random = unit(&mut self.rng);
        self.touch.drift_to = unit(&mut self.rng);
        self.touch.pulses = self.touch.pulses.wrapping_add(1);
    }

    /// The next step, in the order set.
    fn touch_advance(&mut self, p: &[f32]) {
        let n = (param(p, STEPS, 8.0).round() as usize).clamp(1, N_STEPS);
        let order = choice(p, ORDER, ORDERS.len());
        let t = &mut self.touch;
        if n == 1 {
            t.step = 0;
            return;
        }
        // From the start (or a step past the end): the first step.
        if t.step >= n {
            t.step = match order {
                3 => n - 1,
                _ => 0,
            };
            t.backwards = false;
            return;
        }
        let cur = t.step;
        t.step = match order {
            1 => {
                // Pendulum: there and back, the ends once.
                if cur + 1 >= n {
                    t.backwards = true;
                } else if cur == 0 {
                    t.backwards = false;
                }
                if t.backwards {
                    cur.saturating_sub(1)
                } else {
                    cur + 1
                }
            }
            2 => ((unit(&mut self.rng) * n as f32) as usize).min(n - 1),
            3 => (cur + n - 1) % n,
            4 => {
                // Drunk: a step this way or that.
                if unit(&mut self.rng) < 0.5 {
                    (cur + 1) % n
                } else {
                    (cur + n - 1) % n
                }
            }
            5 => (cur + 3) % n,
            6 => (cur + 5) % n,
            _ => (cur + 1) % n,
        };
    }

    /// A note now, or later (strums, echoes, ratchets): kept until its block.
    fn touch_play(&mut self, ctx: &MidiCtx, h: Hit, out: &mut Vec<MidiEvent>) {
        if h.at < ctx.now + ctx.len as u64 {
            self.note(ctx, h.at.max(ctx.now), h.note as i32, h.vel, h.len, out);
        } else if self.pending.len() < self.pending.capacity() {
            self.pending.push((h.at, h.note, h.vel, h.len as u64));
        }
    }

    /// Ends notes sounding until their plate was let go (and those not yet begun).
    fn touch_end_all(&mut self, notes: &Notes<MAX_SOUNDING>, out: &mut Vec<MidiEvent>) {
        for &note in notes.as_slice() {
            self.pending.retain(|e| e.1 != note);
            if let Some(i) = self.offs.iter().position(|o| o.1 == note) {
                self.offs.swap_remove(i);
                out.push(MidiEvent::off(0, note));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::{MidiFxKind, MidiFxState};
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};

    const SPB: f64 = 24_000.0;

    fn ctx(now: u64, playing: bool, pressure: f32) -> MidiCtx {
        MidiCtx { now, len: 256, samples_per_beat: SPB, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / SPB, playing, sr: 48_000.0, drum_notes: 0, lead: 0.0, touch: [pressure, 0.5, 0.25, 0.75, 0.4] }
    }

    /// Runs `blocks` blocks with plate events at given blocks: (block, pitch, on).
    fn run(p: &[f32], data: &[u8], blocks: u64, plates: &[(u64, u8, bool)], playing: bool) -> (Vec<(u64, u8, bool, f32)>, MidiFxState) {
        let mut st = MidiFxState::new(MidiFxKind::Touch);
        let mut evs = Vec::new();
        for b in 0..blocks {
            let now = b * 256;
            let input: Vec<MidiEvent> = plates.iter().filter(|e| e.0 == b).map(|e| if e.2 { MidiEvent::on(0, e.1, 0.9) } else { MidiEvent::off(0, e.1) }).collect();
            let mut out = Vec::with_capacity(64);
            st.process(MidiFxKind::Touch, p, data, &input, &mut out, &ctx(now, playing, 0.7));
            for e in &out {
                evs.push((now + e.offset as u64, e.pitch, e.on, e.vel));
            }
        }
        (evs, st)
    }

    fn ons(evs: &[(u64, u8, bool, f32)]) -> Vec<(u64, u8)> {
        evs.iter().filter(|e| e.2).map(|e| (e.0, e.1)).collect()
    }

    fn beats(b: f64) -> u64 {
        (b * SPB / 256.0) as u64
    }

    #[test]
    fn the_tonnetz_triangles_are_triads_and_their_neighbours_share_two_notes() {
        let c = Tri::default();
        assert_eq!((c.root(), c.minor, c.classes()), (0, false, [0, 4, 7]), "C");
        assert_eq!(Tri::new(0, 0, true).classes(), [4, 7, 11], "Em: the triangle beside C");
        assert_eq!(Tri::new(-1, 0, true).classes(), [9, 0, 4], "Am");
        assert_eq!(Tri::new(-1, 0, false).root(), 5, "F");
        // P, L and R: each shares two notes, and going back undoes it.
        for t in [c, Tri::new(1, -1, true), Tri::new(-3, 1, false)] {
            for turn in [Turn::P, Turn::L, Turn::R] {
                let u = t.turn(turn);
                assert_ne!(u.minor, t.minor);
                let shared = u.classes().iter().filter(|x| t.classes().contains(x)).count();
                assert_eq!(shared, 2, "{t:?} {turn:?} {u:?}");
                assert_eq!(u.turn(turn).classes(), t.classes(), "{turn:?} twice");
            }
        }
        assert_eq!((c.turn(Turn::P).root(), c.turn(Turn::L).root(), c.turn(Turn::R).root()), (0, 4, 9), "Cm, Em, Am");
        // A long walk stays near the middle.
        let mut t = c;
        for k in 0..200 {
            t = t.turn([Turn::P, Turn::L, Turn::R, Turn::L][k % 4]);
            assert!((-4..=3).contains(&t.a) && (-2..=1).contains(&t.b), "{t:?}");
        }
        // Around a plate: the nearest octave, so C to Am keeps C and E.
        assert_eq!(triad(60, c), [55, 60, 64]);
        assert_eq!(triad(60, Tri::new(-1, 0, true)), [57, 60, 64]);
        for v in 0..=255u8 {
            let t = Tri::from_byte(v);
            assert_eq!(Tri::from_byte(t.byte()), t);
        }
    }

    #[test]
    fn with_tonnetz_on_the_steps_play_their_triads() {
        let mut p = defaults(TOUCH);
        (p[SEQ], p[LAYOUT], p[STEPS]) = (1.0, TONNETZ_LAYOUT as f32, 4.0);
        let data = default_data();
        // Four touches: I, vi, IV, V around C4.
        let plates: Vec<(u64, u8, bool)> = (0..4).flat_map(|k| [(2 + k * 10, 60, true), (7 + k * 10, 60, false)]).collect();
        let (evs, _) = run(&p, &data, 50, &plates, false);
        let on = ons(&evs);
        let chords: Vec<Vec<u8>> = (0..4)
            .map(|k| {
                let at = (2 + k * 10) * 256;
                let mut c: Vec<u8> = on.iter().filter(|e| e.0 == at).map(|e| e.1).collect();
                c.sort_unstable();
                c
            })
            .collect();
        assert_eq!(chords, vec![vec![55, 60, 64], vec![57, 60, 64], vec![57, 60, 65], vec![55, 59, 62]]);
        // A triangle pressed (its three notes together, the root last) is one touch: one step further, around its root.
        let plates = [(2, 64, true), (2, 67, true), (2, 60, true), (8, 64, false), (8, 67, false), (8, 60, false), (12, 69, true), (12, 72, true), (12, 65, true), (18, 69, false), (18, 72, false), (18, 65, false)];
        let (evs, _) = run(&p, &data, 30, &plates, false);
        let on = ons(&evs);
        let at = |b: u64| {
            let mut c: Vec<u8> = on.iter().filter(|e| e.0 == b * 256).map(|e| e.1).collect();
            c.sort_unstable();
            c
        };
        assert_eq!((at(2), at(12)), (vec![55, 60, 64], vec![62, 65, 69]), "I around C, then vi around F: Dm");
        // Without the sequence a triangle plays itself, and the pulser pulses it whole.
        let mut q = defaults(TOUCH);
        (q[LAYOUT], q[PULSE], q[RATE]) = (TONNETZ_LAYOUT as f32, 1.0, 0.0);
        let held = [(2, 64, true), (2, 67, true), (2, 60, true), (60, 64, false), (60, 67, false), (60, 60, false)];
        let (evs, _) = run(&q, &data, 70, &held, false);
        let on = ons(&evs);
        let pulses: Vec<u64> = {
            let mut t: Vec<u64> = on.iter().map(|e| e.0).collect();
            t.dedup();
            t
        };
        assert!(pulses.len() >= 4, "{pulses:?}");
        for t in pulses {
            let mut c: Vec<u8> = on.iter().filter(|e| e.0 == t).map(|e| e.1).collect();
            c.sort_unstable();
            assert_eq!(c, vec![60, 64, 67], "the whole chord at {t}");
        }
        // Older data (the third version) reads with every triangle on I.
        let mut old = data.clone();
        old.truncate(V3_LEN);
        old[0] = 3;
        assert_eq!(step(&old, 1).tri, Tri::default());
        assert_eq!(step(&old, 1).note, step(&data, 1).note);
        let mut grown = old.clone();
        whole(&mut grown);
        assert_eq!(grown.len(), LEN);
    }

    #[test]
    fn a_plate_plays_while_it_is_held() {
        let p = defaults(TOUCH);
        let (evs, _) = run(&p, &default_data(), 40, &[(2, 60, true), (30, 60, false)], false);
        let simple: Vec<(u64, u8, bool)> = evs.iter().map(|e| (e.0, e.1, e.2)).collect();
        assert_eq!(simple, vec![(512, 60, true), (30 * 256, 60, false)]);
    }

    #[test]
    fn the_sequencer_off_leaves_the_notes_alone() {
        let mut p = defaults(TOUCH);
        assert!(p[SEQ] < 0.5, "off at first");
        p[PULSE] = 1.0;
        p[RATE] = 6.0;
        let (evs, _) = run(&p, &default_data(), beats(4.0), &[(1, 60, true)], false);
        assert!(ons(&evs).iter().all(|e| e.1 == 60), "{evs:?}");
    }

    #[test]
    fn the_pulser_plays_the_held_plate_again_with_the_transport_stopped() {
        let mut p = defaults(TOUCH);
        p[PULSE] = 1.0;
        p[RATE] = 6.0;
        let (evs, _) = run(&p, &default_data(), beats(4.0), &[(1, 62, true)], false);
        let at: Vec<u64> = ons(&evs).iter().map(|e| e.0).collect();
        assert_eq!(at, vec![256, 24_000, 48_000, 72_000], "at once, then every beat");
    }

    #[test]
    fn swing_delays_the_odd_pulses() {
        let mut p = defaults(TOUCH);
        p[PULSE] = 1.0;
        p[RATE] = 6.0;
        p[SWING] = 0.5;
        let (evs, _) = run(&p, &default_data(), beats(4.0), &[(1, 62, true)], false);
        let at: Vec<u64> = ons(&evs).iter().map(|e| e.0).collect();
        assert_eq!(at, vec![256, 24_000 + 6_000, 48_000, 72_000 + 6_000]);
    }

    #[test]
    fn steps_move_the_notes_through_the_key_when_pitch_is_on() {
        let mut p = defaults(TOUCH);
        p[SEQ] = 1.0;
        let data = notes(&[0, 2, 4, 0]);
        let presses: Vec<(u64, u8, bool)> = (0..3).flat_map(|i| [(i * 4 + 1, 60, true), (i * 4 + 3, 60, false)]).collect();
        let (evs, _) = run(&p, &data, 16, &presses, false);
        assert_eq!(ons(&evs).iter().map(|e| e.1).collect::<Vec<_>>(), vec![60, 64, 67], "each touch a step further (C, E, G)");
        // Pitch off: the steps still run, the notes stay.
        p[PITCH] = 0.0;
        let (evs, _) = run(&p, &data, 16, &presses, false);
        assert_eq!(ons(&evs).iter().map(|e| e.1).collect::<Vec<_>>(), vec![60, 60, 60]);
    }

    #[test]
    fn trigger_lets_the_steps_rest_ratchet_and_accent() {
        let mut p = defaults(TOUCH);
        p[SEQ] = 1.0;
        p[PITCH] = 0.0;
        p[PULSE] = 1.0;
        p[RATE] = 6.0;
        p[RUN] = 1.0;
        p[BASE] = 60.0;
        p[STEPS] = 4.0;
        let steps = [Step { gate: Gate::Rest, ..Step::default() }, Step { ratchet: 3, ..Step::default() }, Step { accent: true, ..Step::default() }, Step::default()];
        let data = pattern(&steps);
        // Run: no plate needed. Pulses at beats 0..3 play steps 1 to 4.
        let (evs, _) = run(&p, &data, beats(4.0), &[], false);
        let at: Vec<u64> = ons(&evs).iter().map(|e| e.0).collect();
        assert_eq!(at, vec![24_000, 32_000, 40_000, 48_000, 72_000], "a rest, a ratchet of three, the accent, one: {at:?}");
        let vels: Vec<f32> = evs.iter().filter(|e| e.2).map(|e| e.3).collect();
        assert!(vels[3] > vels[4], "the accent is louder: {vels:?}");
        // Trigger off: every pulse plays one note.
        p[TRIGGER] = 0.0;
        let (evs, _) = run(&p, &data, beats(4.0), &[], false);
        assert_eq!(ons(&evs).len(), 4);
    }

    #[test]
    fn a_voicing_plays_a_chord_in_the_key() {
        let mut p = defaults(TOUCH);
        p[VOICING] = 4.0;
        let (evs, _) = run(&p, &default_data(), 10, &[(1, 62, true)], false);
        let mut chord: Vec<u8> = ons(&evs).iter().map(|e| e.1).collect();
        chord.sort();
        assert_eq!(chord, vec![62, 65, 69], "D minor in C major");
    }

    #[test]
    fn the_arp_goes_up_over_the_plates_and_octaves() {
        let mut p = defaults(TOUCH);
        p[PLAY] = 1.0;
        p[RATE] = 6.0;
        p[OCTAVES] = 2.0;
        let (evs, _) = run(&p, &default_data(), beats(5.0), &[(1, 60, true), (1, 64, true)], false);
        let notes: Vec<u8> = ons(&evs).iter().map(|e| e.1).collect();
        assert_eq!(notes, vec![60, 64, 72, 76], "on the beats after the touch: {notes:?}");
    }

    #[test]
    fn strum_spreads_the_chord_and_echo_repeats_it_fainter() {
        let mut p = defaults(TOUCH);
        p[VOICING] = 4.0;
        p[PLAY] = 2.0;
        p[STRUM] = 0.05;
        let (evs, _) = run(&p, &default_data(), 40, &[(1, 60, true)], false);
        let at: Vec<u64> = ons(&evs).iter().map(|e| e.0).collect();
        assert_eq!(at, vec![256, 256 + 2_400, 256 + 4_800], "50 ms apart");
        let mut p = defaults(TOUCH);
        p[PLAY] = 3.0;
        p[ECHOES] = 2.0;
        p[RATE] = 6.0;
        let (evs, _) = run(&p, &default_data(), beats(3.0), &[(1, 60, true), (beats(2.5), 60, false)], false);
        let hits: Vec<(u64, f32)> = evs.iter().filter(|e| e.2).map(|e| (e.0, e.3)).collect();
        assert_eq!(hits.len(), 3, "{hits:?}");
        assert!(hits[1].1 < hits[0].1 && hits[2].1 < hits[1].1);
        assert_eq!(hits[1].0, 256 + 24_000);
    }

    #[test]
    fn bounce_comes_back_sooner_and_softer() {
        let mut p = defaults(TOUCH);
        p[PLAY] = 4.0;
        p[ECHOES] = 2.0;
        p[RATE] = 6.0;
        let (evs, _) = run(&p, &default_data(), beats(4.0), &[(1, 60, true)], false);
        let hits: Vec<(u64, f32)> = evs.iter().filter(|e| e.2).map(|e| (e.0, e.3)).collect();
        assert_eq!(hits.len(), 6, "{hits:?}");
        let gaps: Vec<u64> = hits.windows(2).map(|w| w[1].0 - w[0].0).collect();
        assert!(gaps.windows(2).all(|g| g[1] < g[0]), "sooner each time: {gaps:?}");
        assert!(hits.windows(2).all(|w| w[1].1 < w[0].1), "softer each time");
    }

    #[test]
    fn the_envelope_rises_falls_holds_and_cycles() {
        let mut p = defaults(TOUCH);
        p[ENV_ATTACK] = 0.01;
        p[ENV_DECAY] = 0.1;
        let env_after = |p: &[f32], blocks: u64, release: Option<u64>| {
            let mut plates = vec![(1, 60, true)];
            if let Some(b) = release {
                plates.push((b, 60, false));
            }
            let (_, st) = run(p, &default_data(), blocks, &plates, false);
            st.touch_out().unwrap()[10]
        };
        // Transient: up and down, held or not.
        assert!(env_after(&p, 4, None) > 0.5);
        assert!(env_after(&p, 120, None) < 0.05);
        // Sustained: up while held, down after.
        p[ENV_MODE] = 1.0;
        assert!(env_after(&p, 120, None) > 0.99);
        assert!(env_after(&p, 240, Some(100)) < 0.05);
        // Cycle: round and round.
        p[ENV_MODE] = 2.0;
        let (_, st) = run(&p, &default_data(), 600, &[(1, 60, true), (2, 60, false)], false);
        let mut highs = 0;
        let mut st = st;
        for b in 600..1200u64 {
            let mut out = Vec::with_capacity(8);
            st.process(MidiFxKind::Touch, &p, &default_data(), &[], &mut out, &ctx(b * 256, false, 0.0));
            if st.touch_out().unwrap()[10] > 0.95 {
                highs += 1;
            }
        }
        assert!(highs > 0, "it rises again by itself");
    }

    #[test]
    fn random_steps_quantize_and_the_plate_value_comes_through() {
        let mut p = defaults(TOUCH);
        p[PULSE] = 1.0;
        p[RANDOM_STEPS] = 3.0;
        let (_, st) = run(&p, &default_data(), 200, &[(1, 60, true)], false);
        let o = st.touch_out().unwrap();
        assert!([0.0, 0.5, 1.0].iter().any(|l| (o[3] - l).abs() < 1e-4), "three levels: {}", o[3]);
        assert!((o[11] - 0.4).abs() < 1e-4, "the plate's value");
        let mut d = default_data();
        assert!((plate_value(&d, 7) - 1.0).abs() < 1e-3 && plate_value(&d, 0) == 0.0);
        set_plate_value(&mut d, 30, 0.5);
        assert!((plate_value(&d, 6) - 0.5).abs() < 0.01, "plates wrap");
        // The second version without plate values grows them.
        let mut v2 = d[..super::V2_LEN].to_vec();
        assert!((step(&v2, 0).note, patch(&v2, 0)) == (step(&d, 0).note, patch(&d, 0)));
        whole(&mut v2);
        assert_eq!(v2.len(), d.len());
    }

    #[test]
    fn latch_keeps_the_last_plate_and_a_new_touch_ends_it() {
        let mut p = defaults(TOUCH);
        p[LATCH] = 1.0;
        let (evs, _) = run(&p, &default_data(), 20, &[(1, 60, true), (3, 60, false), (10, 64, true), (12, 64, false)], false);
        let simple: Vec<(u64, u8, bool)> = evs.iter().map(|e| (e.0, e.1, e.2)).collect();
        assert_eq!(simple, vec![(256, 60, true), (2560, 60, false), (2560, 64, true)]);
    }

    #[test]
    fn its_outputs_reach_the_modulators() {
        let mut p = defaults(TOUCH);
        p[PULSE] = 1.0;
        p[SEQ] = 1.0;
        let (_, st) = run(&p, &default_data(), 30, &[(1, 60, true)], true);
        let o = st.touch_out().expect("touch");
        assert!((o[0] - 0.7).abs() < 1e-3, "pressure {o:?}");
        assert!(o[1] > 0.0 && o[1] <= 1.0, "the pulse decays {o:?}");
        assert!((o[5] - 0.5).abs() < 1e-3 && (o[8] - 0.25).abs() < 1e-3 && (o[9] - 0.75).abs() < 1e-3, "position, X, Y {o:?}");
        assert!((0.0..=1.0).contains(&o[6]) && (0.0..=1.0).contains(&o[7]), "LFO and shape {o:?}");
        let seen = read_visual(st.visual().unwrap());
        assert!(seen.step < seen.steps && seen.steps == 8 && seen.pulses > 0 && seen.seq && (seen.outs[0] - 0.7).abs() < 0.01 && (seen.outs[9] - 0.75).abs() < 0.01);
    }

    #[test]
    fn data_keeps_steps_patch_points_and_shape_and_reads_the_first_version() {
        let mut d = default_data();
        let s = Step { note: -5, value: 0.5, chance: 0.25, ratchet: 7, pulses: 5, gate: Gate::Roll, tie: true, accent: true, skip: true, tri: Tri::new(-3, 1, true) };
        set_step(&mut d, 11, s);
        set_patch(&mut d, 7, Some(17));
        set_shape(&mut d, 3, 1.0);
        let back = step(&d, 11);
        assert_eq!((back.note, back.ratchet, back.pulses, back.gate, back.tie, back.accent, back.skip, back.tri), (-5, 7, 5, Gate::Roll, true, true, true, Tri::new(-3, 1, true)));
        assert_eq!(step(&d, 10).gate, Gate::Single);
        assert!((back.value - 0.5).abs() < 0.01 && (back.chance - 0.25).abs() < 0.01);
        assert_eq!((patch(&d, 7), patch(&d, 0)), (Some(17), None));
        assert_eq!(shape(&d, 3), 1.0);
        // The first version: five steps (note, value) and six patch points.
        let mut v1 = vec![2u8, 200, 254, 10, 0, 0, 0, 0, 0, 0];
        for k in 0..6u16 {
            v1.extend_from_slice(&(if k == 1 { 5u16 } else { 0xFFFF }).to_le_bytes());
        }
        assert_eq!(v1.len(), 22);
        assert_eq!((step(&v1, 0).note, step(&v1, 1).note, patch(&v1, 1)), (2, -2, Some(5)));
        set_shape(&mut v1, 0, 0.5);
        assert!(is_current(&v1));
        assert_eq!((step(&v1, 0).note, step(&v1, 1).note, patch(&v1, 1), patch(&v1, 6)), (2, -2, Some(5), None));
    }

    /// Run on, the base note C (60), C major: a step's note n is the key's n-th note up.
    fn stages(steps: &[Step], beats_long: f64) -> Vec<(u64, u8, bool, f32)> {
        let mut p = defaults(TOUCH);
        p[SEQ] = 1.0;
        p[PULSE] = 1.0;
        p[RATE] = 6.0;
        p[RUN] = 1.0;
        p[BASE] = 60.0;
        p[STEPS] = steps.len() as f32;
        run(&p, &pattern(steps), beats(beats_long), &[], false).0
    }

    #[test]
    fn a_step_lasts_its_pulses_and_its_gate_decides_what_they_play() {
        let st = |note: i32, pulses: u8, gate: Gate| Step { note, pulses, gate, ..Step::default() };
        let steps = [st(0, 3, Gate::Every), st(1, 2, Gate::Single), st(2, 3, Gate::Hold), st(3, 2, Gate::Rest), st(4, 3, Gate::Last)];
        let evs = stages(&steps, 13.0);
        let b = 24_000;
        assert_eq!(ons(&evs), vec![(0, 60), (b, 60), (2 * b, 60), (3 * b, 62), (5 * b, 64), (12 * b, 67)], "every pulse, the first, held, none, the last");
        let off = |pitch: u8, after: u64| evs.iter().find(|e| !e.2 && e.1 == pitch && e.0 > after).map(|e| e.0).unwrap();
        assert!(off(62, 3 * b) < 4 * b, "Single: as long as the gate");
        assert!(off(64, 5 * b) > 7 * b + b / 2, "Hold: over its three pulses");
    }

    #[test]
    fn alternate_roll_and_ratchets_up_to_eight() {
        let steps = [Step { note: 0, pulses: 4, gate: Gate::Alternate, ..Step::default() }, Step { note: 1, pulses: 3, gate: Gate::Roll, ..Step::default() }, Step { note: 2, ratchet: 8, ..Step::default() }];
        let evs = stages(&steps, 8.0);
        let count = |pitch: u8| ons(&evs).iter().filter(|e| e.1 == pitch).count();
        assert_eq!((count(60), count(62), count(64)), (2, 1 + 2 + 3, 8), "every other pulse, more each pulse, eight in one");
        let eight: Vec<u64> = ons(&evs).iter().filter(|e| e.1 == 64).map(|e| e.0).collect();
        assert_eq!(eight[1] - eight[0], 3_000, "a beat split in eight");
    }

    #[test]
    fn skipped_steps_are_passed_over() {
        let st = |note: i32, skip: bool| Step { note, skip, ..Step::default() };
        let evs = stages(&[st(0, false), st(1, true), st(2, false), st(3, true)], 4.0);
        let notes: Vec<u8> = ons(&evs).iter().map(|e| e.1).collect();
        assert_eq!(notes, vec![60, 64, 60, 64]);
        // All of them skipped: it still goes on.
        let evs = stages(&[st(0, true), st(1, true)], 4.0);
        assert_eq!(ons(&evs).len(), 4);
    }

    #[test]
    fn the_second_version_is_read_and_made_the_third() {
        let mut v2 = vec![0u8; 137];
        v2[0] = 2;
        for i in 0..N_STEPS {
            v2[1 + i * 4 + 2] = 255;
        }
        // Step 4: note -3, ratchet 3, a rest, an accent.
        v2[1 + 3 * 4..1 + 3 * 4 + 4].copy_from_slice(&[(-3i8) as u8, 128, 255, 2 | 4 | 16]);
        for k in 0..N_PATCH {
            v2[65 + k * 2..67 + k * 2].copy_from_slice(&(if k == 2 { 9u16 } else { 0xFFFF }).to_le_bytes());
        }
        v2[81 + 5] = 255;
        v2[113 + 1] = 77;
        let s = step(&v2, 3);
        assert_eq!((s.note, s.ratchet, s.gate, s.accent, s.pulses), (-3, 3, Gate::Rest, true, 1));
        assert_eq!((patch(&v2, 2), patch(&v2, 0), shape(&v2, 5)), (Some(9), None, 1.0));
        set_step(&mut v2, 0, Step { note: 1, ..Step::default() });
        assert!(is_current(&v2));
        let s = step(&v2, 3);
        assert_eq!((s.note, s.ratchet, s.gate, s.accent, step(&v2, 0).note), (-3, 3, Gate::Rest, true, 1));
        assert_eq!((patch(&v2, 2), shape(&v2, 5), (plate_value(&v2, 1) * 255.0).round()), (Some(9), 1.0, 77.0));
        // Without the plates' values: theirs at first.
        let mut short = vec![0u8; 113];
        short[0] = 2;
        assert_eq!(plate_value(&short, 7), 1.0);
        whole(&mut short);
        assert_eq!((short.len(), plate_value(&short, 7)), (LEN, 1.0));
    }
}
