//! Touch: a track played with plates, made to play with. The plates (on the
//! Touch view, the computer keys or fingers on a touch screen) give notes and
//! how hard they are pressed. How they sound:
//!
//! - *Play*: as they are (Notes), arpeggiated (Arp: six orders over one to
//!   four octaves), strummed (Strum: the chord's notes one after the other,
//!   down and up in turn), or with echoes that fade (Echo).
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
//! a stepped and a drifting random, where along the plates the hand is, an
//! LFO, a drawn shape, and the XY pad's X and Y.
//!
//! Its data (version 2): a version byte, sixteen steps of four bytes (note
//! i8, value, chance, flags: ratchet - 1 in the low two bits, then rest, tie,
//! accent), eight patch points (the instrument's parameters the Touch view
//! shows, u16 little-endian, 0xFFFF: none) and the shape's 32 points. The
//! first version (five steps of two bytes and six patch points) is read too.

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

pub const RATES: &[&str] = &["1/32", "1/16 T", "1/16", "1/8 T", "1/8", "1/4 T", "1/4", "1/2", "1 Bar"];
const RATE_BEATS: [f64; 9] = [0.125, 1.0 / 6.0, 0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 1.0, 2.0, 4.0];
pub const ORDERS: &[&str] = &["Forward", "Pendulum", "Random", "Backward", "Drunk", "Every 3rd", "Every 5th"];
pub const PLAYS: &[&str] = &["Notes", "Arp", "Strum", "Echo"];
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
pub const ARP_ORDERS: &[&str] = &["Up", "Down", "Up-Down", "Random", "As Played", "Converge"];
pub const LFO_SHAPES: &[&str] = &["Sine", "Triangle", "Saw", "Square", "S&H"];
pub const SLOW_RATES: &[&str] = &["1/16", "1/8", "1/4", "1/2", "1 Bar", "2 Bars", "4 Bars", "8 Bars"];
const SLOW_BEATS: [f64; 8] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
pub const LAYOUTS: &[&str] = &["Row", "Wide", "Grid"];
pub const NOTE_SETS: &[&str] = &["Key", "Pentatonic", "Chromatic"];
/// What the Touch modulators can take from it (the first six kept their places).
pub const OUTPUTS: &[&str] = &["Pressure", "Pulse", "Steps", "Random", "Drift", "Position", "LFO", "Shape", "X", "Y"];
pub const N_OUT: usize = 10;
pub const N_STEPS: usize = 16;
pub const N_PATCH: usize = 8;
pub const N_SHAPE: usize = 32;
/// Plates held at once, notes sounding at once.
const MAX_HELD: usize = 16;
const MAX_SOUNDING: usize = 48;

const VERSION: u8 = 2;
const STEP_AT: usize = 1;
const PATCH_AT: usize = STEP_AT + N_STEPS * 4;
const SHAPE_AT: usize = PATCH_AT + N_PATCH * 2;
const LEN: usize = SHAPE_AT + N_SHAPE;
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
    ParamSpec::int("Echoes", "Play", 1.0, 8.0, 3.0, Unit::None),
    ParamSpec::lin("Feedback", "Play", 0.1, 0.95, 0.6, Unit::Percent),
    ParamSpec::toggle("Run", "Play", false),
    ParamSpec::choice("LFO Shape", "Motion", LFO_SHAPES, 0),
    ParamSpec::choice("LFO Rate", "Motion", SLOW_RATES, 4),
    ParamSpec::choice("Shape Rate", "Motion", SLOW_RATES, 5),
    ParamSpec::choice("Layout", "Plates", LAYOUTS, 0),
    ParamSpec::choice("Notes", "Plates", NOTE_SETS, 0),
    ParamSpec::toggle("Spring", "Motion", true),
    ParamSpec::toggle("Sequencer", "Sequence", false),
    ParamSpec::toggle("Trigger", "Sequence", true),
];

/// One step of the sequence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    /// Scale steps up or down (-14..14).
    pub note: i32,
    /// The value it gives the Steps output (0..1).
    pub value: f32,
    /// How likely it plays (0..1).
    pub chance: f32,
    /// Notes in the step (1..4).
    pub ratchet: u8,
    pub rest: bool,
    /// Held into the next step (legato).
    pub tie: bool,
    pub accent: bool,
}

impl Default for Step {
    fn default() -> Self {
        Self { note: 0, value: 0.0, chance: 1.0, ratchet: 1, rest: false, tie: false, accent: false }
    }
}

fn step_bytes(s: &Step) -> [u8; 4] {
    let flags = (s.ratchet.clamp(1, 4) - 1) | (s.rest as u8) << 2 | (s.tie as u8) << 3 | (s.accent as u8) << 4;
    [s.note.clamp(-14, 14) as i8 as u8, (s.value.clamp(0.0, 1.0) * 255.0).round() as u8, (s.chance.clamp(0.0, 1.0) * 255.0).round() as u8, flags]
}

fn is_v2(data: &[u8]) -> bool {
    data.len() == LEN && data[0] == VERSION
}

/// A pattern's data from its steps (the rest of the sixteen plain), no patch
/// points, the shape a gentle rise and fall.
pub fn pattern(steps: &[Step]) -> Vec<u8> {
    let mut d = vec![0u8; LEN];
    d[0] = VERSION;
    for i in 0..N_STEPS {
        let s = steps.get(i).copied().unwrap_or_default();
        d[STEP_AT + i * 4..STEP_AT + i * 4 + 4].copy_from_slice(&step_bytes(&s));
    }
    for k in 0..N_PATCH {
        d[PATCH_AT + k * 2..PATCH_AT + k * 2 + 2].copy_from_slice(&0xFFFFu16.to_le_bytes());
    }
    for j in 0..N_SHAPE {
        d[SHAPE_AT + j] = default_shape(j);
    }
    d
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

/// What it starts with: a little tune, accents on the ones.
pub fn default_data() -> Vec<u8> {
    let ns = [0, 2, 4, 2, 7, 4, 2, -1, 0, 4, 2, 5, 4, 2, 0, -3];
    let steps: Vec<Step> = ns.iter().enumerate().map(|(i, &n)| Step { note: n, value: ((i * 5) % 16) as f32 / 15.0, accent: i % 8 == 0, ..Step::default() }).collect();
    pattern(&steps)
}

/// Data of any version made the current one, keeping what it had.
pub fn whole(data: &mut Vec<u8>) {
    if is_v2(data) {
        return;
    }
    let old = std::mem::take(data);
    *data = default_data();
    if old.len() == V1_LEN {
        for i in 0..N_STEPS {
            let s = if i < 5 { Step { note: (old[i * 2] as i8) as i32, value: old[i * 2 + 1] as f32 / 255.0, ..Step::default() } } else { Step::default() };
            data[STEP_AT + i * 4..STEP_AT + i * 4 + 4].copy_from_slice(&step_bytes(&s));
        }
        for k in 0..N_PATCH {
            let v = if k < 6 { [old[10 + k * 2], old[11 + k * 2]] } else { [0xFF, 0xFF] };
            data[PATCH_AT + k * 2..PATCH_AT + k * 2 + 2].copy_from_slice(&v);
        }
    }
}

/// Step `i` (of any version's data).
pub fn step(data: &[u8], i: usize) -> Step {
    if is_v2(data) && i < N_STEPS {
        let b = &data[STEP_AT + i * 4..STEP_AT + i * 4 + 4];
        return Step {
            note: ((b[0] as i8) as i32).clamp(-14, 14),
            value: b[1] as f32 / 255.0,
            chance: b[2] as f32 / 255.0,
            ratchet: (b[3] & 3) + 1,
            rest: b[3] & 4 != 0,
            tie: b[3] & 8 != 0,
            accent: b[3] & 16 != 0,
        };
    }
    if data.len() == V1_LEN && i < 5 {
        return Step { note: ((data[i * 2] as i8) as i32).clamp(-7, 7), value: data[i * 2 + 1] as f32 / 255.0, ..Step::default() };
    }
    Step::default()
}

pub fn set_step(data: &mut Vec<u8>, i: usize, s: Step) {
    whole(data);
    if i < N_STEPS {
        data[STEP_AT + i * 4..STEP_AT + i * 4 + 4].copy_from_slice(&step_bytes(&s));
    }
}

/// The instrument parameter on patch point `k`, if one is set.
pub fn patch(data: &[u8], k: usize) -> Option<u16> {
    let at = if is_v2(data) && k < N_PATCH {
        PATCH_AT + k * 2
    } else if data.len() == V1_LEN && k < 6 {
        10 + k * 2
    } else {
        return None;
    };
    let v = u16::from_le_bytes([data[at], data[at + 1]]);
    (v != 0xFFFF).then_some(v)
}

pub fn set_patch(data: &mut Vec<u8>, k: usize, param: Option<u16>) {
    whole(data);
    if k < N_PATCH {
        data[PATCH_AT + k * 2..PATCH_AT + k * 2 + 2].copy_from_slice(&param.unwrap_or(0xFFFF).to_le_bytes());
    }
}

/// The drawn shape's point `j` (0..1).
pub fn shape(data: &[u8], j: usize) -> f32 {
    let j = j % N_SHAPE;
    if is_v2(data) {
        data[SHAPE_AT + j] as f32 / 255.0
    } else {
        default_shape(j) as f32 / 255.0
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
/// the note played last, whether the sequencer runs, and the outputs.
pub struct Seen {
    pub step: usize,
    pub steps: usize,
    pub pulses: u32,
    pub last_note: u8,
    pub seq: bool,
    pub outs: [f32; N_OUT],
}

pub fn read_visual(v: [u64; 4]) -> Seen {
    let outs = std::array::from_fn(|k| {
        let (word, at) = if k < 8 { (v[1], k) } else { (v[2], k - 8) };
        ((word >> (at * 8)) & 0xFF) as f32 / 255.0
    });
    Seen { step: (v[0] & 0xFF) as usize, steps: ((v[0] >> 8) & 0xFF) as usize, pulses: ((v[0] >> 16) & 0xFFFF) as u32, last_note: ((v[0] >> 32) & 0x7F) as u8, seq: (v[0] >> 40) & 1 == 1, outs }
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

fn param(p: &[f32], i: usize, or: f32) -> f32 {
    p.get(i).copied().unwrap_or(or)
}

fn choice(p: &[f32], i: usize, n: usize) -> usize {
    (param(p, i, 0.0).round().max(0.0) as usize).min(n - 1)
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
            for &(plate, vel) in &presses[..n_press] {
                let mut hits = Hits::new();
                self.touch_trigger(p, data, ctx, ctx.now, plate, vel_of(vel), 1e12, false, &mut hits);
                for h in &hits.v[..hits.n] {
                    if h.len >= 1e11 {
                        self.touch.remember(plate, h.note);
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
                for &(plate, vel) in &presses[..n_press] {
                    let mut hits = Hits::new();
                    self.touch_trigger(p, data, ctx, ctx.now, plate, vel_of(vel), interval * gate(p), true, &mut hits);
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
        let outs = [pressure, self.touch.env, value, self.touch.random, self.touch.drift, ctx.touch[1].clamp(0.0, 1.0), lfo, drawn, ctx.touch[2].clamp(0.0, 1.0), ctx.touch[3].clamp(0.0, 1.0)];
        self.touch_out = outs;
        let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u64;
        self.visual[0] = shown as u64 | (steps as u64) << 8 | ((self.touch.pulses & 0xFFFF) as u64) << 16 | ((self.touch.last_note & 0x7F) as u64) << 32 | (seq as u64) << 40;
        self.visual[1] = (0..8).fold(0u64, |a, k| a | byte(outs[k]) << (k * 8));
        self.visual[2] = byte(outs[8]) | byte(outs[9]) << 8;
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
            if param(p, SEQ, 0.0) >= 0.5 {
                self.touch_advance(p);
            }
            self.touch_outputs();
            return;
        };
        let mut hits = Hits::new();
        let len = rate_beats(p) * ctx.samples_per_beat * gate(p);
        if arp {
            self.touch_arp(p, data, ctx, at, plate, vel, len, latch, run, &mut hits);
        } else {
            self.touch_trigger(p, data, ctx, at, plate, vel, len, true, &mut hits);
        }
        for h in &hits.v[..hits.n] {
            self.touch_play(ctx, *h, out);
        }
    }

    /// What a touch or a pulse plays from one plate: the step decides (rest,
    /// chance, ratchets, accent, tie), the voicing makes it a chord, the play
    /// mode strums it or adds echoes.
    #[allow(clippy::too_many_arguments)]
    fn touch_trigger(&mut self, p: &[f32], data: &[u8], ctx: &MidiCtx, at: u64, plate: u8, vel: f32, len: f64, clocked: bool, hits: &mut Hits) {
        let interval = rate_beats(p) * ctx.samples_per_beat;
        let st = self.touch_step(p, data);
        self.touch_outputs();
        let Some((ratchet, accent, tie)) = self.touch_gates(p, st) else { return };
        let vel = if accent { (vel * 1.3).min(1.0) } else { vel };
        // A tie holds into the next pulse; ratchets split the pulse.
        let len = if clocked && tie { interval * 1.05 } else { len };
        let ratchet = if clocked { ratchet } else { 1 };
        let sub = interval / ratchet as f64;
        let rlen = if ratchet > 1 { (sub * gate(p)).min(len) } else { len };
        let shift = self.touch_shift(p, st);
        let voicing = VOICING_NOTES[choice(p, VOICING, VOICING_NOTES.len())];
        let play = choice(p, PLAY, PLAYS.len());
        let strum = (param(p, STRUM, 0.04).clamp(0.005, 0.25) * ctx.sr) as u64;
        self.touch.strum_up = !self.touch.strum_up;
        let up = self.touch.strum_up;
        let echoes = param(p, ECHOES, 3.0).round().clamp(1.0, 8.0) as u32;
        let feedback = param(p, FEEDBACK, 0.6).clamp(0.1, 0.95);
        let n = voicing.len();
        for r in 0..ratchet {
            let rat = at + (r as f64 * sub) as u64;
            for (j, &(steps, octs)) in voicing.iter().enumerate() {
                let note = self.touch_pitch(ctx, plate, shift + steps, octs);
                // Strum: one after the other, up and down in turn.
                let k = if up { j } else { n - 1 - j } as u64;
                let start = if play == 2 { rat + k * strum } else { rat };
                hits.push(Hit { at: start, note, vel, len: rlen });
                if play == 3 {
                    // Echo: again on the following pulses, fainter each time.
                    for e in 1..=echoes {
                        hits.push(Hit { at: start + (e as f64 * interval) as u64, note, vel: vel * feedback.powi(e as i32), len: (interval * gate(p)).min(len) });
                    }
                }
            }
        }
        self.touch.last_note = self.touch_pitch(ctx, plate, shift, 0);
    }

    /// An arpeggio's next note from the plates held (or latched): their
    /// voicings over the octaves.
    #[allow(clippy::too_many_arguments)]
    fn touch_arp(&mut self, p: &[f32], data: &[u8], ctx: &MidiCtx, at: u64, plate: u8, vel: f32, len: f64, latch: bool, run: bool, hits: &mut Hits) {
        let st = self.touch_step(p, data);
        self.touch_outputs();
        let Some((ratchet, accent, tie)) = self.touch_gates(p, st) else { return };
        let shift = self.touch_shift(p, st);
        let voicing = VOICING_NOTES[choice(p, VOICING, VOICING_NOTES.len())];
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
        // The pool: every note of every plate, in the octaves above.
        let order = choice(p, ARP_ORDER, ARP_ORDERS.len());
        let mut pool: Notes<MAX_SOUNDING> = Notes::default();
        for o in 0..octaves {
            for &pl in plates.as_slice() {
                for &(steps, octs) in voicing {
                    pool.push(self.touch_pitch(ctx, pl, shift + steps, octs + o));
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
        let vel = if accent { (vel * 1.3).min(1.0) } else { vel };
        let interval = rate_beats(p) * ctx.samples_per_beat;
        let len = if tie { interval * 1.05 } else { len };
        let sub = interval / ratchet as f64;
        for r in 0..ratchet {
            let rlen = if ratchet > 1 { (sub * gate(p)).min(len) } else { len };
            hits.push(Hit { at: at + (r as f64 * sub) as u64, note, vel, len: rlen });
        }
        self.touch.last_note = note;
    }

    /// The sequence a step further (when it runs), and that step.
    fn touch_step(&mut self, p: &[f32], data: &[u8]) -> Option<Step> {
        if param(p, SEQ, 0.0) < 0.5 {
            return None;
        }
        self.touch_advance(p);
        let n = (param(p, STEPS, 8.0).round() as usize).clamp(1, N_STEPS);
        Some(step(data, self.touch.step.min(n - 1)))
    }

    /// What a step lets through when it decides (Trigger): (ratchet, accent,
    /// tie), or None for a rest (or a step its chance leaves out).
    fn touch_gates(&mut self, p: &[f32], st: Option<Step>) -> Option<(u8, bool, bool)> {
        let Some(st) = st.filter(|_| param(p, TRIGGER, 1.0) >= 0.5) else { return Some((1, false, false)) };
        if st.rest || (st.chance < 1.0 && unit(&mut self.rng) >= st.chance) {
            return None;
        }
        Some((st.ratchet.clamp(1, 4), st.accent, st.tie))
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
        MidiCtx { now, len: 256, samples_per_beat: SPB, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / SPB, playing, sr: 48_000.0, drum_notes: 0, lead: 0.0, touch: [pressure, 0.5, 0.25, 0.75] }
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
        let steps = [Step { rest: true, ..Step::default() }, Step { ratchet: 3, ..Step::default() }, Step { accent: true, ..Step::default() }, Step::default()];
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
        let s = Step { note: -5, value: 0.5, chance: 0.25, ratchet: 3, rest: false, tie: true, accent: true };
        set_step(&mut d, 11, s);
        set_patch(&mut d, 7, Some(17));
        set_shape(&mut d, 3, 1.0);
        let back = step(&d, 11);
        assert_eq!((back.note, back.ratchet, back.tie, back.accent, back.rest), (-5, 3, true, true, false));
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
        assert!(is_v2(&v1));
        assert_eq!((step(&v1, 0).note, step(&v1, 1).note, patch(&v1, 1), patch(&v1, 6)), (2, -2, Some(5), None));
    }
}
