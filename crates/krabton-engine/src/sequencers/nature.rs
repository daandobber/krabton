//! Sequencers after things that happen by themselves outside:
//!
//! - Fireflies: a meadow of fireflies, each flashing at its own pace, each
//!   pulled towards the others' rhythm (Kuramoto's coupled oscillators).
//!   With little Coupling they flash as they like; turn it up and they
//!   find each other until they all flash as one. Nudge kicks one out of
//!   step now and then; Seed scatters them again.
//! - Sandpile: grains dropped one by one on a board (the Abelian sandpile):
//!   a cell with four grains topples, one to each neighbour, and those can
//!   topple in turn. Every wave of an avalanche plays the cells that
//!   toppled, by their row, column, diagonal or distance from the middle;
//!   grains that fall off the edge are gone.
//! - Pendulums: a pendulum wave. A row of pendulums, each a little shorter
//!   than the last, let go together: each swings Spread times more per
//!   cycle than the one before, so they drift through waves, snakes and
//!   chaos and meet again at the end of the cycle (up to 64 bars, slowed
//!   down up to eight times more). A note whenever one passes the middle
//!   (or turns at an end), another on the far side if Other Side says so;
//!   Wobble puts each a hair out so they never quite meet again, Together
//!   makes them louder where many pass at once, Hold Swing holds each note
//!   until its pendulum comes by again.
//! - Flock: birds flying over the notes (boids: Separation keeps them
//!   apart, Alignment turns them the way their neighbours fly, Cohesion
//!   pulls them together). A bird crossing one of the lines plays the note
//!   of the height it flies at.
//! - Plants: a Lindenmayer system grows a plant from a seed (its axiom) by
//!   its rules, generation after generation; the plant is then walked as a
//!   melody: F a note, + and - a step up or down, [ a branch that comes back
//!   to where it began with ], X a rest.

use super::minimal::{choose, hash, DYNAMICS};
use super::{SeqState, RATES};
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};
use std::f64::consts::TAU;

const KEYS: [ParamSpec; 2] = [ParamSpec::toggle("Transpose", "Keys", false), ParamSpec::toggle("Hold", "Keys", false)];

/// How the voices of a sequencer (fireflies, pendulums) are spread over the key.
pub const LAYOUTS: &[&str] = &["Scale", "Chord", "Pentatonic", "One Note"];

/// The scale degree of voice `i` in `layout`, `spread` steps apart (Scale).
pub fn voice_degree(layout: usize, i: usize, spread: i32) -> i32 {
    let i = i as i32;
    match layout {
        1 => [0, 2, 4][(i % 3) as usize] + 7 * (i / 3),
        2 => [0, 1, 2, 4, 5][(i % 5) as usize] + 7 * (i / 5),
        3 => 0,
        _ => i * spread.max(1),
    }
}

/// The pitch of `degree` scale steps above the base note.
pub(crate) fn pitch(ctx: &MidiCtx, base: f32, degree: i32) -> i32 {
    let (b, _) = ctx.scale.locate(base.round() as i32);
    ctx.scale.pitch_at(b + degree)
}

/// The frame of song beat `beat` (later blocks are fine: `play` waits).
pub(crate) fn frame_of(ctx: &MidiCtx, beat: f64) -> u64 {
    ctx.now + ((beat - ctx.beat).max(0.0) * ctx.samples_per_beat) as u64
}

/// A degree folded back into the `range` steps around 0 (as a ball off two walls).
pub fn fold(d: i32, range: i32) -> i32 {
    let r = range.max(1);
    let h = r / 2;
    let m = (d + h).rem_euclid(2 * r);
    if m > r {
        2 * r - m - h
    } else {
        m - h
    }
}

// ---------------------------------------------------------------- fireflies

pub const FF_COUNT: usize = 0;
pub const FF_PULSE: usize = 1;
pub const FF_SPREAD: usize = 2;
pub const FF_COUPLING: usize = 3;
pub const FF_NUDGE: usize = 4;
pub const FF_SYNC: usize = 5;
pub const FF_SEED: usize = 6;
pub const FF_LAYOUT: usize = 7;
pub const FF_STEPS: usize = 8;
pub const FF_BASE: usize = 9;
pub const FF_GATE: usize = 10;
pub const FF_VELOCITY: usize = 11;
pub const FF_SWELL: usize = 12;
pub const MAX_FIREFLIES: usize = 12;
pub const PULSES: &[&str] = &["1/8", "1/4", "1/2", "1 Bar", "2 Bars"];
pub const PULSE_BEATS: [f64; 5] = [0.5, 1.0, 2.0, 4.0, 8.0];

pub static FIREFLIES: &[ParamSpec] = &[
    ParamSpec::int("Fireflies", "Meadow", 2.0, MAX_FIREFLIES as f32, 6.0, Unit::None),
    ParamSpec::choice("Pulse", "Meadow", PULSES, 1),
    ParamSpec::lin("Spread", "Meadow", 0.0, 0.5, 0.12, Unit::Percent),
    ParamSpec::lin("Coupling", "Meadow", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Nudge", "Meadow", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::choice("Sync", "Meadow", super::more::SYNCS, 0),
    ParamSpec::int("Seed", "Meadow", 0.0, 99.0, 0.0, Unit::None),
    ParamSpec::choice("Notes", "Notes", LAYOUTS, 1),
    ParamSpec::int("Steps", "Notes", 1.0, 4.0, 1.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.05, 2.0, 0.25, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Swell", "Notes", 0.0, 1.0, 0.5, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

// ---------------------------------------------------------------- sandpile

pub const SP_RATE: usize = 0;
pub const SP_SIZE: usize = 1;
pub const SP_DROP: usize = 2;
pub const SP_SPOT: usize = 3;
pub const SP_START: usize = 4;
pub const SP_SEED: usize = 5;
pub const SP_PLAY: usize = 6;
pub const SP_VOICES: usize = 7;
pub const SP_STEPS: usize = 8;
pub const SP_BASE: usize = 9;
pub const SP_GATE: usize = 10;
pub const SP_VELOCITY: usize = 11;
pub const SP_GROW: usize = 12;
pub const SP_GRAINS: usize = 13;
pub const SPOTS: &[&str] = &["Middle", "Random", "Wander", "Corner", "Two"];
pub const STARTS: &[&str] = &["Empty", "Random", "Full"];
pub const SAND_PLAYS: &[&str] = &["Row", "Column", "Diagonal", "Distance"];
pub const MAX_SIDE: usize = 8;

pub static SANDPILE: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::int("Size", "Pile", 4.0, MAX_SIDE as f32, 7.0, Unit::None),
    ParamSpec::lin("Drop", "Pile", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::choice("Spot", "Pile", SPOTS, 0),
    ParamSpec::choice("Start", "Pile", STARTS, 1),
    ParamSpec::int("Seed", "Pile", 0.0, 99.0, 0.0, Unit::None),
    ParamSpec::choice("Play", "Notes", SAND_PLAYS, 0),
    ParamSpec::int("Voices", "Notes", 1.0, 6.0, 3.0, Unit::None),
    ParamSpec::int("Steps", "Notes", 1.0, 3.0, 1.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 55.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 2.0, 0.6, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Grow", "Notes", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::toggle("Grains", "Notes", true),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

// ---------------------------------------------------------------- pendulums

pub const PE_COUNT: usize = 0;
pub const PE_CYCLE: usize = 1;
pub const PE_SWINGS: usize = 2;
pub const PE_PLAY: usize = 3;
pub const PE_SYNC: usize = 4;
pub const PE_FADE: usize = 5;
pub const PE_LAYOUT: usize = 6;
pub const PE_STEPS: usize = 7;
pub const PE_REVERSE: usize = 8;
pub const PE_BASE: usize = 9;
pub const PE_GATE: usize = 10;
pub const PE_VELOCITY: usize = 11;
pub const PE_SLOW: usize = 18;
pub const PE_SPREAD: usize = 19;
pub const PE_WOBBLE: usize = 20;
pub const PE_OTHER: usize = 21;
pub const PE_TOGETHER: usize = 22;
pub const PE_CHANCE: usize = 23;
pub const PE_HOLD: usize = 24;
pub const PE_TILT: usize = 25;
pub const MAX_PENDULUMS: usize = 16;
pub const CYCLES: &[&str] = &["1 Bar", "2 Bars", "4 Bars", "8 Bars", "16 Bars", "32 Bars", "64 Bars"];
pub const CYCLE_BEATS: [f64; 7] = [4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0];
pub const PENDULUM_PLAYS: &[&str] = &["Middle", "Ends", "One Side"];

pub static PENDULUMS: &[ParamSpec] = &[
    ParamSpec::int("Pendulums", "Wave", 2.0, MAX_PENDULUMS as f32, 8.0, Unit::None),
    ParamSpec::choice("Cycle", "Wave", CYCLES, 2),
    ParamSpec::int("Swings", "Wave", 1.0, 24.0, 8.0, Unit::None),
    ParamSpec::choice("Play", "Wave", PENDULUM_PLAYS, 1),
    ParamSpec::choice("Sync", "Wave", super::more::SYNCS, 2),
    ParamSpec::lin("Fade", "Wave", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::choice("Notes", "Notes", LAYOUTS, 2),
    ParamSpec::int("Steps", "Notes", 1.0, 4.0, 1.0, Unit::None),
    ParamSpec::toggle("Reverse", "Notes", false),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 55.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.05, 2.0, 0.25, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
    ParamSpec::int("Slower", "Wave", 1.0, 8.0, 1.0, Unit::None),
    ParamSpec::lin("Spread", "Wave", 0.25, 3.0, 1.0, Unit::None),
    ParamSpec::lin("Wobble", "Wave", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Other Side", "Notes", -7.0, 7.0, 0.0, Unit::None),
    ParamSpec::lin("Together", "Notes", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Chance", "Notes", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::toggle("Hold Swing", "Notes", false),
    ParamSpec::lin("Tilt", "Notes", -1.0, 1.0, 0.0, Unit::None),
];

/// How many times pendulum `i` swings to and fro in a cycle: Spread more
/// than the one before, and (Wobble) a hair off.
pub fn pendulum_swings(p: &[f32], i: usize) -> f64 {
    let spread = p.get(PE_SPREAD).copied().unwrap_or(1.0).max(0.05) as f64;
    let wobble = p.get(PE_WOBBLE).copied().unwrap_or(0.0).clamp(0.0, 1.0) as f64;
    let off = 1.0 + wobble * 0.03 * (hash(41, i as u32) as f64 * 2.0 - 1.0);
    (p[PE_SWINGS].round().max(1.0) as f64 + i as f64 * spread) * off
}

/// The beats a cycle of the pendulums lasts (its bars, slowed down).
pub fn pendulum_cycle(p: &[f32]) -> f64 {
    CYCLE_BEATS[choose(p, PE_CYCLE, CYCLE_BEATS.len())] * p.get(PE_SLOW).copied().unwrap_or(1.0).round().clamp(1.0, 8.0) as f64
}

// ---------------------------------------------------------------- flock

pub const FL_BIRDS: usize = 0;
pub const FL_SPEED: usize = 1;
pub const FL_SEPARATION: usize = 2;
pub const FL_ALIGNMENT: usize = 3;
pub const FL_COHESION: usize = 4;
pub const FL_WANDER: usize = 5;
pub const FL_LINES: usize = 6;
pub const FL_SYNC: usize = 7;
pub const FL_SEED: usize = 8;
pub const FL_RANGE: usize = 9;
pub const FL_BASE: usize = 10;
pub const FL_GATE: usize = 11;
pub const FL_VELOCITY: usize = 12;
pub const MAX_BIRDS: usize = 12;

pub static FLOCK: &[ParamSpec] = &[
    ParamSpec::int("Birds", "Flock", 2.0, MAX_BIRDS as f32, 7.0, Unit::None),
    ParamSpec::lin("Speed", "Flock", 0.05, 1.0, 0.3, Unit::None),
    ParamSpec::lin("Separation", "Flock", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Alignment", "Flock", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Cohesion", "Flock", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Wander", "Flock", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::int("Lines", "Flock", 1.0, 8.0, 2.0, Unit::None),
    ParamSpec::choice("Sync", "Flock", super::more::SYNCS, 2),
    ParamSpec::int("Seed", "Flock", 0.0, 99.0, 0.0, Unit::None),
    ParamSpec::int("Range", "Notes", 3.0, 21.0, 10.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 55.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.05, 2.0, 0.3, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

// ---------------------------------------------------------------- plants

pub const PL_RATE: usize = 0;
pub const PL_PLANT: usize = 1;
pub const PL_GENERATIONS: usize = 2;
pub const PL_GROW: usize = 3;
pub const PL_TURN: usize = 4;
pub const PL_X: usize = 5;
pub const PL_BRANCH: usize = 6;
pub const PL_RANGE: usize = 7;
pub const PL_BASE: usize = 8;
pub const PL_GATE: usize = 9;
pub const PL_VELOCITY: usize = 10;
pub const PL_SWING: usize = 11;
pub const PLANTS: &[&str] = &["Algae", "Koch", "Weed", "Bush", "Dragon", "Triangle"];
pub const X_MODES: &[&str] = &["Rest", "Skip"];

pub static PLANT: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Plant", "Growth", PLANTS, 2),
    ParamSpec::int("Generations", "Growth", 1.0, 7.0, 3.0, Unit::None),
    ParamSpec::toggle("Grow", "Growth", true),
    ParamSpec::int("Turn", "Growth", 1.0, 4.0, 1.0, Unit::None),
    ParamSpec::choice("X", "Growth", X_MODES, 0),
    ParamSpec::lin("Branch", "Growth", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::int("Range", "Notes", 3.0, 21.0, 10.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Swing", "Notes", 0.0, 0.75, 0.0, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

/// A plant: its axiom, its rules (symbol, what it becomes) and the angle it
/// is drawn with (degrees).
pub struct Lsystem {
    pub axiom: &'static [u8],
    pub rules: &'static [(u8, &'static [u8])],
    pub angle: f32,
}

pub static LSYSTEMS: [Lsystem; 6] = [
    Lsystem { axiom: b"A", rules: &[(b'A', b"AB"), (b'B', b"A")], angle: 0.0 },
    Lsystem { axiom: b"F", rules: &[(b'F', b"F+F-F-F+F")], angle: 90.0 },
    Lsystem { axiom: b"X", rules: &[(b'X', b"F+[[X]-X]-F[-X]+X"), (b'F', b"FF")], angle: 25.0 },
    Lsystem { axiom: b"F", rules: &[(b'F', b"FF+[+F-F-F]-[-F+F+F]")], angle: 22.5 },
    Lsystem { axiom: b"FX", rules: &[(b'X', b"X+YF+"), (b'Y', b"-FX-Y")], angle: 90.0 },
    Lsystem { axiom: b"A", rules: &[(b'A', b"B-A-B"), (b'B', b"A+B+A")], angle: 60.0 },
];

/// The symbols a plant is written in (the index of each in the length table).
const ALPHABET: &[u8] = b"FGABXY+-[]";

fn letter(c: u8) -> usize {
    ALPHABET.iter().position(|&a| a == c).unwrap_or(0)
}

impl Lsystem {
    fn rule(&self, c: u8) -> Option<&'static [u8]> {
        self.rules.iter().find(|r| r.0 == c).map(|r| r.1)
    }

    /// The plant after `gens` generations, written out (for its picture;
    /// the sequencer walks it without writing it out).
    pub fn grow(&self, gens: usize, max: usize) -> Vec<u8> {
        let mut s = self.axiom.to_vec();
        for _ in 0..gens {
            let mut next = Vec::with_capacity(s.len() * 3);
            for &c in &s {
                match self.rule(c) {
                    Some(r) => next.extend_from_slice(r),
                    None => next.push(c),
                }
                if next.len() >= max {
                    break;
                }
            }
            next.truncate(max);
            s = next;
        }
        s
    }
}

/// How long each symbol grows in each generation (none longer than the cap).
pub(crate) const MAX_GEN: usize = 7;
const LEN_CAP: u64 = 1 << 40;

fn lengths(sys: &Lsystem) -> [[u64; MAX_GEN + 1]; 10] {
    let mut t = [[1u64; MAX_GEN + 1]; 10];
    for g in 1..=MAX_GEN {
        for (k, &c) in ALPHABET.iter().enumerate() {
            if let Some(r) = sys.rule(c) {
                t[k][g] = r.iter().map(|&s| t[letter(s)][g - 1]).sum::<u64>().min(LEN_CAP);
            }
        }
    }
    t
}

/// The `i`th symbol of the plant after `gens` generations, found by going
/// down the rules (nothing written out).
fn symbol_at(sys: &Lsystem, t: &[[u64; MAX_GEN + 1]; 10], gens: usize, mut i: u64) -> u8 {
    let mut s: &[u8] = sys.axiom;
    let mut g = gens;
    loop {
        let mut next = None;
        for &c in s {
            let l = t[letter(c)][g];
            if i < l {
                next = Some(c);
                break;
            }
            i -= l;
        }
        let Some(c) = next else { return b'X' };
        match sys.rule(c) {
            Some(r) if g > 0 => {
                s = r;
                g -= 1;
            }
            _ => return c,
        }
    }
}

fn plant_len(sys: &Lsystem, t: &[[u64; MAX_GEN + 1]; 10], gens: usize) -> u64 {
    sys.axiom.iter().map(|&c| t[letter(c)][gens]).sum::<u64>().clamp(1, LEN_CAP)
}

/// How many symbols plant `which` has after `gens` generations.
pub fn plant_length(which: usize, gens: usize) -> u64 {
    let sys = &LSYSTEMS[which.min(LSYSTEMS.len() - 1)];
    plant_len(sys, &lengths(sys), gens.min(MAX_GEN))
}

// ---------------------------------------------------------------- state

#[derive(Clone, Copy, Default)]
struct Bird {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

pub struct NatureState {
    rng: u32,
    // Fireflies: phase (cycles), natural pace (cycles per beat), last flash (beat).
    ff_phase: [f64; MAX_FIREFLIES],
    ff_pace: [f64; MAX_FIREFLIES],
    ff_last: [f64; MAX_FIREFLIES],
    ff_t: f64,
    ff_seed: i32,
    ff_ready: bool,
    pub(crate) ff_order: f64,
    // Sandpile: grains per cell (row by row), where grains drop.
    pub(crate) sand: [u8; MAX_SIDE * MAX_SIDE],
    sand_spot: (usize, usize),
    sand_ready: bool,
    sand_seed: i32,
    sand_start: usize,
    sand_drops: u32,
    pub(crate) sand_wave: u32,
    pub(crate) sand_size: u32,
    pub(crate) sand_dropped: u64,
    pub(crate) sand_lost: u64,
    // Pendulums: the last crossing played (beat) per pendulum.
    pe_last: [f64; MAX_PENDULUMS],
    // Flock.
    birds: [Bird; MAX_BIRDS],
    fl_last: [f64; MAX_BIRDS],
    fl_t: f64,
    fl_seed: i32,
    fl_ready: bool,
    // Plants: where in the plant, the pitch and the branches it came from.
    pl_pos: u64,
    pl_gen: usize,
    pl_degree: i32,
    pl_stack: [i32; 16],
    pl_depth: usize,
    pl_key: (usize, usize),
    pl_lens: [[u64; MAX_GEN + 1]; 10],
}

impl Default for NatureState {
    fn default() -> Self {
        Self {
            rng: 0x1F1E_F17E,
            ff_phase: [0.0; MAX_FIREFLIES],
            ff_pace: [1.0; MAX_FIREFLIES],
            ff_last: [f64::NEG_INFINITY; MAX_FIREFLIES],
            ff_t: f64::NEG_INFINITY,
            ff_seed: -1,
            ff_ready: false,
            ff_order: 0.0,
            sand: [0; MAX_SIDE * MAX_SIDE],
            sand_spot: (3, 3),
            sand_ready: false,
            sand_seed: -1,
            sand_start: usize::MAX,
            sand_drops: 0,
            sand_wave: 0,
            sand_size: 0,
            sand_dropped: 0,
            sand_lost: 0,
            pe_last: [f64::NEG_INFINITY; MAX_PENDULUMS],
            birds: [Bird::default(); MAX_BIRDS],
            fl_last: [f64::NEG_INFINITY; MAX_BIRDS],
            fl_t: f64::NEG_INFINITY,
            fl_seed: -1,
            fl_ready: false,
            pl_pos: 0,
            pl_gen: 0,
            pl_degree: 0,
            pl_stack: [0; 16],
            pl_depth: 0,
            pl_key: (usize::MAX, 0),
            pl_lens: [[1; MAX_GEN + 1]; 10],
        }
    }
}

impl NatureState {
    /// The song jumped: the moving ones start again from where they begin.
    pub fn restart(&mut self) {
        self.ff_ready = false;
        self.fl_ready = false;
        self.pe_last = [f64::NEG_INFINITY; MAX_PENDULUMS];
    }

    fn rand(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f64 / 16_777_216.0
    }
}

impl SeqState {
    // ------------------------------------------------------------ fireflies

    /// Fireflies: every firefly's phase runs at its own pace and is pulled
    /// towards the others'; it flashes (plays) when its phase comes round.
    pub(crate) fn fireflies(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let (b0, b1) = (ctx.beat, ctx.beat + ctx.len as f64 / ctx.samples_per_beat);
        let n = (p[FF_COUNT].round() as usize).clamp(2, MAX_FIREFLIES);
        let pace = 1.0 / PULSE_BEATS[choose(p, FF_PULSE, PULSE_BEATS.len())];
        let spread = p[FF_SPREAD].clamp(0.0, 0.5) as f64;
        let seed = p[FF_SEED].round() as i32;
        let st = &mut self.nature;
        if !st.ff_ready || st.ff_seed != seed || (st.ff_t - b0).abs() > 0.05 {
            // Scattered over the meadow: their phases and paces from the seed.
            for i in 0..MAX_FIREFLIES {
                st.ff_phase[i] = hash(seed as u32 * 97 + 1, i as u32) as f64;
                st.ff_last[i] = f64::NEG_INFINITY;
            }
            st.ff_t = b0;
            st.ff_seed = seed;
            st.ff_ready = true;
            st.rng = 0x1F1E_F17E ^ (seed as u32).wrapping_mul(0x9E37_79B9);
        }
        for i in 0..n {
            st.ff_pace[i] = pace * (1.0 + spread * (2.0 * hash(seed as u32 * 31 + 7, i as u32) as f64 - 1.0));
        }
        let k = p[FF_COUPLING].clamp(0.0, 1.0) as f64 * pace;
        let nudge = p[FF_NUDGE].clamp(0.0, 1.0) as f64;
        const DT: f64 = 1.0 / 192.0;
        let mut flashes = [(0.0f64, 0usize); 64];
        let mut nf = 0;
        let mut t = st.ff_t;
        while t + DT <= b1 {
            // Each pulled by the sine of how far the others are ahead.
            let mut d = [0.0f64; MAX_FIREFLIES];
            for i in 0..n {
                let mut pull = 0.0;
                for j in 0..n {
                    pull += (TAU * (st.ff_phase[j] - st.ff_phase[i])).sin();
                }
                d[i] = (st.ff_pace[i] + k * pull / n as f64).max(0.0);
            }
            for i in 0..n {
                let before = st.ff_phase[i];
                let after = before + d[i] * DT;
                if after >= 1.0 {
                    if nf < flashes.len() {
                        let frac = (1.0 - before) / (after - before).max(1e-12);
                        flashes[nf] = (t + frac * DT, i);
                        nf += 1;
                    }
                    st.ff_phase[i] = after - 1.0;
                } else {
                    st.ff_phase[i] = after;
                }
            }
            // Now and then, on a beat, one is kicked out of step.
            if (t + DT).floor() > t.floor() && nudge > 0.0 && st.rand() < nudge * 0.5 {
                let i = (st.rand() * n as f64) as usize % n;
                st.ff_phase[i] = (st.ff_phase[i] + 0.2 + 0.6 * st.rand()).fract();
            }
            t += DT;
        }
        st.ff_t = t;
        // How together they are (1: as one).
        let (mut sx, mut sy) = (0.0, 0.0);
        for i in 0..n {
            sx += (TAU * st.ff_phase[i]).cos();
            sy += (TAU * st.ff_phase[i]).sin();
        }
        let order = (sx * sx + sy * sy).sqrt() / n as f64;
        st.ff_order = order;
        let grid = super::more::sync_beats(p[FF_SYNC]);
        let layout = choose(p, FF_LAYOUT, LAYOUTS.len());
        let steps = p[FF_STEPS].round() as i32;
        let swell = p[FF_SWELL].clamp(0.0, 1.0) as f64;
        let vel = (p[FF_VELOCITY] as f64 * (1.0 - swell + swell * order)) as f32;
        let gate = p[FF_GATE].max(0.02) as f64 * ctx.samples_per_beat;
        for &(beat, i) in &flashes[..nf] {
            let on = super::more::snap(beat, grid);
            if on > self.nature.ff_last[i] + 1e-6 && beat >= b0 - 1e-9 {
                self.nature.ff_last[i] = on;
                let note = pitch(ctx, p[FF_BASE], voice_degree(layout, i, steps));
                self.play(ctx, frame_of(ctx, on), note, vel, gate, out);
            }
        }
        let st = &self.nature;
        let mut bytes = [0u8; 32];
        for i in 0..n {
            bytes[i] = (st.ff_phase[i].clamp(0.0, 0.999) * 256.0) as u8;
        }
        bytes[12] = n as u8;
        bytes[13] = (order.clamp(0.0, 1.0) * 255.0) as u8;
        self.visual = pack(&bytes);
    }

    // ------------------------------------------------------------ sandpile

    /// Sandpile: a step drops a grain (by chance) when the pile is still,
    /// then one wave of toppling; the cells that toppled play.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn sandpile(&mut self, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let side = (p[SP_SIZE].round() as usize).clamp(4, MAX_SIDE);
        let seed = p[SP_SEED].round() as i32;
        let start = choose(p, SP_START, STARTS.len());
        let st = &mut self.nature;
        if !st.sand_ready || st.sand_seed != seed || st.sand_start != start {
            st.sand = [0; MAX_SIDE * MAX_SIDE];
            for (i, c) in st.sand.iter_mut().enumerate() {
                *c = match start {
                    1 => (hash(seed as u32 + 501, i as u32) * 4.0) as u8 % 4,
                    2 => 3,
                    _ => 0,
                };
            }
            st.sand_seed = seed;
            st.sand_start = start;
            st.sand_ready = true;
            st.sand_wave = 0;
            st.sand_drops = 0;
            st.sand_spot = (side / 2, side / 2);
            st.rng = 0x5A4D_0001 ^ (seed as u32).wrapping_mul(0x85EB_CA6B);
        }
        // Cells outside a smaller board are empty.
        for y in 0..MAX_SIDE {
            for x in 0..MAX_SIDE {
                if x >= side || y >= side {
                    st.sand[y * MAX_SIDE + x] = 0;
                }
            }
        }
        let still = !st.sand.iter().any(|&h| h >= 4);
        let mut dropped = None;
        if still {
            st.sand_wave = 0;
            if st.rand() < p[SP_DROP].clamp(0.0, 1.0) as f64 {
                let (x, y) = Self::sand_spot(st, choose(p, SP_SPOT, SPOTS.len()), side);
                st.sand[y * MAX_SIDE + x] += 1;
                st.sand_dropped += 1;
                st.sand_drops = st.sand_drops.wrapping_add(1);
                dropped = Some((x, y));
            }
        }
        // One wave: every cell with four or more topples at once.
        let mut toppled = 0u64;
        let before = st.sand;
        for y in 0..side {
            for x in 0..side {
                if before[y * MAX_SIDE + x] >= 4 {
                    toppled |= 1 << (y * MAX_SIDE + x);
                    st.sand[y * MAX_SIDE + x] -= 4;
                    for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                        if nx < 0 || ny < 0 || nx >= side as i32 || ny >= side as i32 {
                            st.sand_lost += 1;
                        } else {
                            st.sand[ny as usize * MAX_SIDE + nx as usize] += 1;
                        }
                    }
                }
            }
        }
        if toppled != 0 {
            if st.sand_wave == 0 {
                st.sand_size = 0;
            }
            st.sand_wave += 1;
            st.sand_size += toppled.count_ones();
        }
        let wave = st.sand_wave;
        // The notes: the toppled cells' degrees, each once, as many as Voices.
        let play = choose(p, SP_PLAY, SAND_PLAYS.len());
        let steps = p[SP_STEPS].round().max(1.0) as i32;
        let degree = |x: usize, y: usize| -> i32 {
            let d = match play {
                1 => x as i32,
                2 => (side - 1 - y) as i32 + x as i32,
                3 => {
                    let c = (side as f32 - 1.0) / 2.0;
                    ((x as f32 - c).abs() + (y as f32 - c).abs()).round() as i32
                }
                _ => (side - 1 - y) as i32,
            };
            d * steps
        };
        let mut seen = 0u64;
        let mut degs = [0i32; 64];
        let mut nd = 0;
        for i in 0..MAX_SIDE * MAX_SIDE {
            if toppled >> i & 1 == 1 {
                let d = degree(i % MAX_SIDE, i / MAX_SIDE);
                if d < 64 && seen >> d & 1 == 0 {
                    seen |= 1 << d;
                    degs[nd] = d;
                    nd += 1;
                }
            }
        }
        let voices = (p[SP_VOICES].round() as usize).clamp(1, 6);
        let grow = p[SP_GROW].clamp(0.0, 1.0);
        let vel = p[SP_VELOCITY] * (1.0 - grow + grow * (0.4 + 0.6 * (wave as f32 / 6.0).min(1.0)));
        let len = step_frames * p[SP_GATE].max(0.05) as f64;
        // Spread over what toppled: the lowest, the highest and between.
        for v in 0..voices.min(nd) {
            let k = if voices >= nd { v } else { v * (nd - 1) / (voices - 1).max(1) };
            let note = pitch(ctx, p[SP_BASE], degs[k]);
            self.note(ctx, at, note, vel, len, out);
        }
        if nd == 0 && p[SP_GRAINS] >= 0.5 {
            if let Some((x, y)) = dropped {
                // A grain landing: a soft tick of its cell.
                let note = pitch(ctx, p[SP_BASE], degree(x, y));
                self.note(ctx, at, note, p[SP_VELOCITY] * 0.35, len * 0.5, out);
            }
        }
        let st = &self.nature;
        let mut heights = [0u64; 2];
        for i in 0..MAX_SIDE * MAX_SIDE {
            heights[i / 32] |= (st.sand[i].min(3) as u64) << ((i % 32) * 2);
        }
        let (sx, sy) = st.sand_spot;
        self.visual = [heights[0], heights[1], toppled, sx as u64 | (sy as u64) << 4 | (side as u64) << 8 | (wave.min(255) as u64) << 12 | (st.sand_size.min(4095) as u64) << 20 | ((dropped.is_some() as u64) << 32)];
    }

    /// Where the next grain falls.
    fn sand_spot(st: &mut NatureState, spot: usize, side: usize) -> (usize, usize) {
        let mid = side / 2;
        let s = match spot {
            1 => ((st.rand() * side as f64) as usize, (st.rand() * side as f64) as usize),
            2 => {
                let (x, y) = st.sand_spot;
                let r = (st.rand() * 4.0) as usize;
                let (dx, dy) = [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)][r % 4];
                ((x as i32 + dx).clamp(0, side as i32 - 1) as usize, (y as i32 + dy).clamp(0, side as i32 - 1) as usize)
            }
            3 => (0, side - 1),
            4 => {
                if st.sand_drops % 2 == 0 {
                    (side / 4, mid)
                } else {
                    (side - 1 - side / 4, mid)
                }
            }
            _ => (mid, mid),
        };
        let s = (s.0.min(side - 1), s.1.min(side - 1));
        st.sand_spot = s;
        s
    }

    // ------------------------------------------------------------ pendulums

    /// Pendulums: pendulum `i` swings `swings + i` times a cycle, all let go
    /// together at the start of each cycle (at an end).
    pub(crate) fn pendulums(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let (b0, b1) = (ctx.beat, ctx.beat + ctx.len as f64 / ctx.samples_per_beat);
        let n = (p[PE_COUNT].round() as usize).clamp(2, MAX_PENDULUMS);
        let cycle = pendulum_cycle(p);
        let play = choose(p, PE_PLAY, PENDULUM_PLAYS.len());
        let other = p[PE_OTHER].round() as i32;
        let together = p[PE_TOGETHER].clamp(0.0, 1.0) as f64;
        let chance = p[PE_CHANCE].clamp(0.0, 1.0);
        let hold = p[PE_HOLD] >= 0.5;
        let tilt = p[PE_TILT].clamp(-1.0, 1.0) as f64;
        // When each plays: the beats between, and where the first is.
        let when = |i: usize| {
            let swings = pendulum_swings(p, i);
            match play {
                0 => (cycle / (2.0 * swings), cycle / (4.0 * swings)),
                1 => (cycle / (2.0 * swings), 0.0),
                _ => (cycle / swings, 0.0),
            }
        };
        let grid = super::more::sync_beats(p[PE_SYNC]);
        let fade = p[PE_FADE].clamp(0.0, 1.0) as f64;
        let layout = choose(p, PE_LAYOUT, LAYOUTS.len());
        let steps = p[PE_STEPS].round() as i32;
        let reverse = p[PE_REVERSE] >= 0.5;
        let gate = p[PE_GATE].max(0.02) as f64 * ctx.samples_per_beat;
        let mut lit = 0u64;
        for i in 0..n {
            let (every, first) = when(i);
            let mut k = ((b0 - first) / every - 1e-9).ceil();
            while first + k * every < b1 {
                let t = first + k * every;
                let on = super::more::snap(t, grid);
                if on > self.nature.pe_last[i] + 1e-6 {
                    self.nature.pe_last[i] = on;
                    k += 1.0;
                    // Chance leaves some out.
                    if chance < 1.0 && hash(i as u32 + 7, (k as i64) as u32) >= chance {
                        continue;
                    }
                    // Through the middle fastest; slower as the swing dies away in the cycle.
                    let age = t.rem_euclid(cycle) / cycle;
                    let mut vel = p[PE_VELOCITY] as f64 * (1.0 - fade * age * 0.85);
                    // Together: louder where many pass at once, softer alone.
                    if together > 0.0 && n > 1 {
                        let tol = 0.03;
                        let with = (0..n)
                            .filter(|&j| j != i)
                            .filter(|&j| {
                                let (e, f) = when(j);
                                let x = (t - f) / e;
                                (x - x.round()).abs() * e < tol
                            })
                            .count();
                        vel *= 1.0 + together * (with as f64 / (n - 1) as f64 * 1.2 - 0.3);
                    }
                    // Tilt: the short ones louder (or the long ones).
                    vel *= 1.0 + tilt * 0.6 * (i as f64 / (n - 1) as f64 - 0.5) * 2.0;
                    let voice = if reverse { n - 1 - i } else { i };
                    // The far side (every other pass) its own note.
                    let far = play != 2 && (k as i64 - 1).rem_euclid(2) == 1;
                    let degree = voice_degree(layout, voice, steps) + if far { other } else { 0 };
                    let note = pitch(ctx, p[PE_BASE], degree);
                    let len = if hold { (if play == 2 { every } else { every * 2.0 }) * 0.97 * ctx.samples_per_beat } else { gate };
                    self.play(ctx, frame_of(ctx, on), note, vel.clamp(0.02, 1.0) as f32, len, out);
                    lit |= 1 << i;
                    continue;
                }
                k += 1.0;
            }
        }
        // For the screen: where in the cycle (it works out each swing from there) and who just played.
        let pos = (b1.rem_euclid(cycle) / cycle * 65536.0) as u64 & 0xffff;
        self.visual[0] = pos | (n as u64) << 16;
        self.visual[1] = lit;
    }

    // ------------------------------------------------------------ flock

    /// Flock: the birds fly on (boids) until the end of this block; one that
    /// crosses a line plays the note of its height.
    pub(crate) fn flock(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let (b0, b1) = (ctx.beat, ctx.beat + ctx.len as f64 / ctx.samples_per_beat);
        let n = (p[FL_BIRDS].round() as usize).clamp(2, MAX_BIRDS);
        let seed = p[FL_SEED].round() as i32;
        let st = &mut self.nature;
        if !st.fl_ready || st.fl_seed != seed || (st.fl_t - b0).abs() > 0.05 {
            st.rng = 0xB1AD_5EED ^ (seed as u32).wrapping_mul(0x9E37_79B9);
            for i in 0..MAX_BIRDS {
                let a = hash(seed as u32 + 77, i as u32) as f64 * 0.8;
                st.birds[i] = Bird {
                    x: hash(seed as u32 + 11, i as u32) as f64,
                    y: 0.2 + 0.6 * hash(seed as u32 + 13, i as u32) as f64,
                    vx: (a * 1.2 - 0.48).cos(),
                    vy: (a * 1.2 - 0.48).sin(),
                };
                st.fl_last[i] = f64::NEG_INFINITY;
            }
            st.fl_t = b0;
            st.fl_seed = seed;
            st.fl_ready = true;
        }
        let speed = p[FL_SPEED].clamp(0.02, 2.0) as f64;
        let (sep, ali, coh, wander) = (p[FL_SEPARATION] as f64, p[FL_ALIGNMENT] as f64, p[FL_COHESION] as f64, p[FL_WANDER] as f64);
        let lines = (p[FL_LINES].round() as usize).clamp(1, 8);
        const DT: f64 = 1.0 / 64.0;
        let mut crossings = [(0.0f64, 0usize, 0.0f64); 64];
        let mut nc = 0;
        let mut t = st.fl_t;
        while t + DT <= b1 {
            Self::flock_step(st, n, speed, sep, ali, coh, wander, DT);
            t += DT;
            // Lines are crossed going either way (the field wraps round).
            for i in 0..n {
                let b = st.birds[i];
                let (x0, x1) = (b.x - b.vx * DT, b.x);
                for l in 0..lines {
                    let lx = (l as f64 + 0.5) / lines as f64;
                    let crossed = (x0 < lx && x1 >= lx) || (x0 > lx && x1 <= lx) || (x0 < lx + 1.0 && x1 >= lx + 1.0) || (x0 > lx - 1.0 && x1 <= lx - 1.0);
                    if crossed && nc < crossings.len() {
                        crossings[nc] = (t, i, b.y);
                        nc += 1;
                    }
                }
                st.birds[i].x = b.x.rem_euclid(1.0);
            }
        }
        st.fl_t = t;
        let grid = super::more::sync_beats(p[FL_SYNC]);
        let range = p[FL_RANGE].round().max(1.0) as f64;
        let gate = p[FL_GATE].max(0.02) as f64 * ctx.samples_per_beat;
        for &(beat, i, y) in &crossings[..nc] {
            let on = super::more::snap(beat, grid);
            if on > self.nature.fl_last[i] + 1e-6 && beat >= b0 - 1e-9 {
                self.nature.fl_last[i] = on;
                let degree = (y.clamp(0.0, 0.999) * (range + 1.0)).floor() as i32;
                let note = pitch(ctx, p[FL_BASE], degree);
                let vel = p[FL_VELOCITY] * (0.75 + 0.25 * (i % 3) as f32 / 2.0);
                self.play(ctx, frame_of(ctx, on), note, vel, gate, out);
            }
        }
        // For the screen: each bird's place and heading (12 bits, 12 bits... in 20 bits).
        let st = &self.nature;
        let mut v = [0u64; 4];
        let mut bit = 16usize;
        v[0] = n as u64 | (lines as u64) << 8;
        for b in st.birds.iter().take(n) {
            let x = (b.x.clamp(0.0, 0.999) * 256.0) as u64;
            let y = (b.y.clamp(0.0, 0.999) * 256.0) as u64;
            let h = ((b.vy.atan2(b.vx) / TAU).rem_euclid(1.0) * 16.0) as u64 & 0xf;
            let word = x | y << 8 | h << 16;
            for k in 0..20 {
                let w = bit + k;
                if w < 256 {
                    v[w / 64] |= (word >> k & 1) << (w % 64);
                }
            }
            bit += 20;
        }
        self.visual = v;
    }

    #[allow(clippy::too_many_arguments)]
    fn flock_step(st: &mut NatureState, n: usize, speed: f64, sep: f64, ali: f64, coh: f64, wander: f64, dt: f64) {
        let birds = st.birds;
        for i in 0..n {
            let me = birds[i];
            let (mut ax, mut ay) = (0.0, 0.0);
            let (mut cx, mut cy, mut avx, mut avy, mut count) = (0.0, 0.0, 0.0, 0.0, 0.0);
            for (j, o) in birds.iter().enumerate().take(n) {
                if j == i {
                    continue;
                }
                // The shortest way round the field (it wraps left to right).
                let mut dx = o.x - me.x;
                dx -= dx.round();
                let dy = o.y - me.y;
                let d = (dx * dx + dy * dy).sqrt();
                if d < 0.35 {
                    cx += dx;
                    cy += dy;
                    avx += o.vx;
                    avy += o.vy;
                    count += 1.0;
                }
                if d < 0.1 && d > 1e-6 {
                    ax -= dx / (d * d) * 0.01;
                    ay -= dy / (d * d) * 0.01;
                }
            }
            ax *= sep * 4.0;
            ay *= sep * 4.0;
            if count > 0.0 {
                ax += (avx / count - me.vx) * ali * 3.0 + cx / count * coh * 6.0;
                ay += (avy / count - me.vy) * ali * 3.0 + cy / count * coh * 6.0;
            }
            ax += (st.rand() - 0.5) * wander * 8.0;
            ay += (st.rand() - 0.5) * wander * 8.0;
            // Away from the top and the bottom.
            if me.y < 0.12 {
                ay += (0.12 - me.y) * 40.0;
            }
            if me.y > 0.88 {
                ay -= (me.y - 0.88) * 40.0;
            }
            let b = &mut st.birds[i];
            b.vx += ax * dt;
            b.vy += ay * dt;
            // Each flies at the flock's speed.
            let v = (b.vx * b.vx + b.vy * b.vy).sqrt().max(1e-9);
            b.vx *= speed / v;
            b.vy *= speed / v;
            b.x += b.vx * dt;
            b.y = (b.y + b.vy * dt).clamp(0.0, 1.0);
        }
    }

    // ------------------------------------------------------------ plants

    /// Plants: one symbol of the grown plant a step (turns, branches and,
    /// with X on Skip, X and Y take no time).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn plant(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let which = choose(p, PL_PLANT, PLANTS.len());
        let sys = &LSYSTEMS[which];
        let gens = (p[PL_GENERATIONS].round() as usize).clamp(1, MAX_GEN);
        let grow = p[PL_GROW] >= 0.5;
        let st = &mut self.nature;
        if st.pl_key != (which, gens) {
            st.pl_lens = lengths(sys);
            st.pl_key = (which, gens);
            st.pl_pos = 0;
            st.pl_gen = if grow { 1 } else { gens };
            st.pl_degree = 0;
            st.pl_depth = 0;
        }
        let range = p[PL_RANGE].round().max(1.0) as i32;
        let turn = p[PL_TURN].round().clamp(1.0, 4.0) as i32;
        let skip_x = p[PL_X] >= 0.5;
        let mut note = None;
        let mut rest = false;
        for _ in 0..64 {
            let g = if grow { st.pl_gen.clamp(1, gens) } else { gens };
            let total = plant_len(sys, &st.pl_lens, g);
            if st.pl_pos >= total {
                // The end of the plant: from its root again (the next generation when growing).
                st.pl_pos = 0;
                st.pl_degree = 0;
                st.pl_depth = 0;
                st.pl_gen = if grow { if st.pl_gen >= gens { 1 } else { st.pl_gen + 1 } } else { gens };
                continue;
            }
            let c = symbol_at(sys, &st.pl_lens, g, st.pl_pos);
            st.pl_pos += 1;
            match c {
                b'F' | b'G' | b'A' => {
                    note = Some(st.pl_degree);
                    break;
                }
                b'B' => {
                    // Algae's other cell: the note two steps up.
                    note = Some(st.pl_degree + if which == 0 { 2 } else { 0 });
                    break;
                }
                b'+' => st.pl_degree += turn,
                b'-' => st.pl_degree -= turn,
                b'[' => {
                    if st.pl_depth < st.pl_stack.len() {
                        st.pl_stack[st.pl_depth] = st.pl_degree;
                    }
                    st.pl_depth += 1;
                }
                b']' => {
                    if st.pl_depth > 0 {
                        st.pl_depth -= 1;
                        if st.pl_depth < st.pl_stack.len() {
                            st.pl_degree = st.pl_stack[st.pl_depth];
                        }
                    }
                }
                _ => {
                    if !skip_x {
                        rest = true;
                        break;
                    }
                }
            }
        }
        let depth = st.pl_depth;
        let pos = st.pl_pos;
        let gen = st.pl_gen;
        let degree = st.pl_degree;
        self.visual[0] = pos;
        self.visual[1] = gen as u64 | (depth.min(255) as u64) << 8 | ((fold(degree, range) + 128).clamp(0, 255) as u64) << 16 | (rest as u64) << 24 | (note.is_some() as u64) << 25;
        if let Some(d) = note {
            let branch = p[PL_BRANCH].clamp(0.0, 1.0);
            let vel = p[PL_VELOCITY] * (1.0 - branch * (depth as f32 / 5.0).min(0.8));
            let at = SeqState::swing_at(at, k, p[PL_SWING], step_frames);
            let pch = pitch(ctx, p[PL_BASE], fold(d, range));
            self.note(ctx, at, pch, vel, step_frames * p[PL_GATE] as f64, out);
        }
    }
}

/// 32 bytes into the four words of a visual.
pub(crate) fn pack(bytes: &[u8; 32]) -> [u64; 4] {
    std::array::from_fn(|k| u64::from_le_bytes(bytes[k * 8..k * 8 + 8].try_into().unwrap()))
}

/// The 32 bytes of a visual.
pub fn unpack(v: &[u64; 4]) -> [u8; 32] {
    let mut b = [0u8; 32];
    for k in 0..4 {
        b[k * 8..k * 8 + 8].copy_from_slice(&v[k].to_le_bytes());
    }
    b
}

/// The birds on a Flock's screen: (x, y, heading in sixteenths of a turn).
pub fn birds_from_visual(v: &[u64; 4]) -> (usize, usize, Vec<(f32, f32, f32)>) {
    let n = (v[0] & 0xff) as usize;
    let lines = (v[0] >> 8 & 0xff) as usize;
    let mut birds = Vec::new();
    for i in 0..n.min(MAX_BIRDS) {
        let mut word = 0u64;
        for k in 0..20 {
            let w = 16 + i * 20 + k;
            if w < 256 {
                word |= (v[w / 64] >> (w % 64) & 1) << k;
            }
        }
        birds.push(((word & 0xff) as f32 / 256.0, (word >> 8 & 0xff) as f32 / 256.0, (word >> 16 & 0xf) as f32 / 16.0));
    }
    (n, lines, birds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::MidiFxKind;
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};
    use crate::sequencers::tests::run;

    fn in_key(ons: &[(f64, u8)]) -> bool {
        let c = Scale::new(0, ScaleKind::Major);
        ons.iter().all(|o| c.contains(o.1))
    }

    #[test]
    fn every_nature_sequencer_plays_in_key() {
        for kind in [MidiFxKind::Fireflies, MidiFxKind::Sandpile, MidiFxKind::Pendulums, MidiFxKind::Flock, MidiFxKind::Plants] {
            let ons = run(kind, &defaults(kind.specs()), &kind.default_data(), 16.0);
            assert!(ons.len() >= 8, "{kind:?}: {} notes", ons.len());
            assert!(in_key(&ons), "{kind:?}: {ons:?}");
            assert!(ons.iter().all(|o| (24..=110).contains(&o.1)), "{kind:?}");
            let mut d: Vec<u8> = ons.iter().map(|o| o.1).collect();
            d.sort();
            d.dedup();
            assert!(d.len() >= 2, "{kind:?} plays one note only");
        }
    }

    #[test]
    fn fireflies_flash_as_one_when_coupled() {
        // Coupled: in the last bars every flash has another right with it.
        let mut p = defaults(FIREFLIES);
        p[FF_COUPLING] = 1.0;
        p[FF_NUDGE] = 0.0;
        p[FF_LAYOUT] = 0.0;
        let ons = run(MidiFxKind::Fireflies, &p, &[], 64.0);
        let tail: Vec<f64> = ons.iter().filter(|o| o.0 >= 48.0).map(|o| o.0).collect();
        assert!(tail.len() >= 30, "{}", tail.len());
        for (i, t) in tail.iter().enumerate() {
            assert!(tail.iter().enumerate().any(|(j, u)| j != i && (t - u).abs() < 0.05), "alone at {t}");
        }
        // Uncoupled they keep their own paces: at times far apart.
        let mut st = SeqState::new();
        let mut p = defaults(FIREFLIES);
        p[FF_COUPLING] = 0.0;
        p[FF_NUDGE] = 0.0;
        let ctx = MidiCtx { now: 0, len: 256, samples_per_beat: 24000.0, scale: Scale::default(), scale_on: true, beat: 0.0, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 5] };
        let mut out = Vec::new();
        let mut orders = Vec::new();
        for b in 0..(32.0 * 24000.0 / 256.0) as u64 {
            let c = MidiCtx { now: b * 256, beat: b as f64 * 256.0 / 24000.0, ..ctx };
            out.clear();
            st.fireflies(&p, &mut out, &c);
            orders.push(st.nature.ff_order);
        }
        assert!(orders.iter().any(|o| *o < 0.6), "uncoupled never drifts apart");
        p[FF_COUPLING] = 1.0;
        let mut st = SeqState::new();
        for b in 0..(32.0 * 24000.0 / 256.0) as u64 {
            let c = MidiCtx { now: b * 256, beat: b as f64 * 256.0 / 24000.0, ..ctx };
            out.clear();
            st.fireflies(&p, &mut out, &c);
        }
        assert!(st.nature.ff_order > 0.98, "{}", st.nature.ff_order);
    }

    #[test]
    fn sandpile_conserves_grains_and_topples_at_four() {
        let mut p = defaults(SANDPILE);
        p[SP_START] = 0.0; // empty
        p[SP_DROP] = 1.0;
        let mut st = SeqState::new();
        let ctx = MidiCtx { now: 0, len: 256, samples_per_beat: 24000.0, scale: Scale::default(), scale_on: true, beat: 0.0, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 5] };
        let mut out = Vec::new();
        let mut toppled_any = false;
        for _ in 0..600 {
            st.sandpile(&p, 0, 6000.0, &mut out, &ctx);
            let on_board: u64 = st.nature.sand.iter().map(|&h| h as u64).sum();
            assert_eq!(on_board + st.nature.sand_lost, st.nature.sand_dropped, "grains are kept or fall off");
            toppled_any |= st.visual[2] != 0;
            // Between avalanches every cell holds three at most.
            if st.nature.sand_wave == 0 {
                assert!(st.nature.sand.iter().all(|&h| h < 4));
            }
        }
        assert!(toppled_any);
        assert!(st.nature.sand_lost > 0, "the biggest avalanches reach the edge");
        // The first four grains in the middle: the fourth topples it, one to each side.
        let mut st = SeqState::new();
        for _ in 0..4 {
            st.sandpile(&p, 0, 6000.0, &mut out, &ctx);
        }
        let m = 3 * MAX_SIDE + 3;
        assert_eq!(st.nature.sand[m], 0);
        assert_eq!((st.nature.sand[m - 1], st.nature.sand[m + 1], st.nature.sand[m - MAX_SIDE], st.nature.sand[m + MAX_SIDE]), (1, 1, 1, 1));
    }

    #[test]
    fn pendulums_meet_again_after_a_cycle() {
        let mut p = defaults(PENDULUMS);
        p[PE_COUNT] = 4.0;
        p[PE_CYCLE] = 0.0; // a bar
        p[PE_SWINGS] = 4.0;
        p[PE_SYNC] = 0.0;
        p[PE_LAYOUT] = 0.0;
        let ons = run(MidiFxKind::Pendulums, &p, &[], 8.0);
        // At the ends: pendulum i twice per swing, 4 + i swings a bar.
        let at_start: Vec<u8> = ons.iter().filter(|o| o.0.abs() < 1e-3).map(|o| o.1).collect();
        let at_bar: Vec<u8> = ons.iter().filter(|o| (o.0 - 4.0).abs() < 1e-3).map(|o| o.1).collect();
        assert_eq!(at_start.len(), 4, "{at_start:?}");
        assert_eq!(at_bar.len(), 4, "all together again: {at_bar:?}");
        let first_bar = ons.iter().filter(|o| o.0 < 4.0 - 1e-3).count();
        assert_eq!(first_bar, (0..4).map(|i| 2 * (4 + i)).sum::<usize>());
        // In between they are apart.
        assert!(ons.iter().filter(|o| (o.0 - 2.0).abs() > 1e-3 && o.0 > 0.1 && o.0 < 3.9).count() > 10);
    }

    #[test]
    fn pendulums_slow_down_spread_and_play_their_far_side() {
        let mut p = defaults(PENDULUMS);
        (p[PE_COUNT], p[PE_CYCLE], p[PE_SWINGS], p[PE_SYNC], p[PE_LAYOUT]) = (2.0, 0.0, 2.0, 0.0, 3.0);
        let count = |p: &[f32]| run(MidiFxKind::Pendulums, p, &[], 4.0).iter().filter(|o| o.0 < 4.0 - 1e-3).count();
        assert_eq!(count(&p), 2 * 2 + 2 * 3, "a bar: 2 and 3 swings, at both ends");
        p[PE_SLOW] = 2.0;
        assert_eq!(count(&p), 2 + 3, "twice as slow: half as many");
        p[PE_SLOW] = 1.0;
        p[PE_SPREAD] = 2.0;
        assert_eq!(count(&p), 2 * 2 + 2 * 4, "Spread 2: the second swings two more");
        // Other Side: the far end plays its own note (One Note: the base, and four steps up there).
        p[PE_SPREAD] = 1.0;
        p[PE_OTHER] = 4.0;
        let ons = run(MidiFxKind::Pendulums, &p, &[], 4.0);
        let notes: std::collections::BTreeSet<u8> = ons.iter().map(|o| o.1).collect();
        assert_eq!(notes.len(), 2, "{notes:?}");
        let first: Vec<u8> = ons.iter().filter(|o| o.0 < 1e-3).map(|o| o.1).collect();
        assert!(first.iter().all(|&n| n == *notes.iter().next().unwrap()), "let go from the near side");
        // Wobble: they no longer meet at the end of the cycle.
        p[PE_OTHER] = 0.0;
        p[PE_COUNT] = 6.0;
        p[PE_WOBBLE] = 1.0;
        let ons = run(MidiFxKind::Pendulums, &p, &[], 4.5);
        let at_bar = ons.iter().filter(|o| (o.0 - 4.0).abs() < 1e-3).count();
        assert!(at_bar < 6, "{at_bar} together");
        // A long cycle: 64 bars, slowed down 8 times, 2048 beats.
        p[PE_CYCLE] = 6.0;
        p[PE_SLOW] = 8.0;
        assert_eq!(pendulum_cycle(&p), 2048.0);
    }

    #[test]
    fn flock_aligns_with_alignment() {
        let order = |ali: f32, coh: f32, wander: f32| {
            let mut st = NatureState::default();
            st.rng = 7;
            for i in 0..MAX_BIRDS {
                let a = i as f64 * 2.4;
                st.birds[i] = Bird { x: hash(3, i as u32) as f64, y: 0.2 + 0.6 * hash(4, i as u32) as f64, vx: a.cos(), vy: a.sin() };
            }
            for _ in 0..2000 {
                SeqState::flock_step(&mut st, 10, 0.3, 0.3, ali as f64, coh as f64, wander as f64, 1.0 / 64.0);
                for b in st.birds.iter_mut() {
                    b.x = b.x.rem_euclid(1.0);
                }
            }
            let (sx, sy) = st.birds[..10].iter().fold((0.0, 0.0), |a, b| (a.0 + b.vx, a.1 + b.vy));
            (sx * sx + sy * sy).sqrt() / (10.0 * 0.3)
        };
        let flock = order(1.0, 0.6, 0.05);
        let loose = order(0.0, 0.0, 1.0);
        assert!(flock > 0.8, "{flock}");
        assert!(loose < flock, "{loose} vs {flock}");
    }

    #[test]
    fn plants_grow_by_their_rules() {
        let algae = &LSYSTEMS[0];
        assert_eq!(algae.grow(4, 1000), b"ABAABABA".to_vec());
        let t = lengths(algae);
        // Fibonacci lengths, and the walk finds the same symbols.
        assert_eq!(plant_len(algae, &t, 5), 13);
        let grown = algae.grow(5, 1000);
        for (i, &c) in grown.iter().enumerate() {
            assert_eq!(symbol_at(algae, &t, 5, i as u64), c);
        }
        let weed = &LSYSTEMS[2];
        let t = lengths(weed);
        let grown = weed.grow(3, 100_000);
        assert_eq!(plant_len(weed, &t, 3), grown.len() as u64);
        for i in (0..grown.len()).step_by(7) {
            assert_eq!(symbol_at(weed, &t, 3, i as u64), grown[i]);
        }
        // Koch's curve: every + has its -; the melody comes back to where it began.
        let mut p = defaults(PLANT);
        p[PL_PLANT] = 1.0;
        p[PL_GROW] = 0.0;
        p[PL_GENERATIONS] = 1.0;
        p[PL_RANGE] = 21.0;
        p[PL_BASE] = 60.0;
        let ons = run(MidiFxKind::Plants, &p, &[], 4.0);
        // F+F-F-F+F: C D C B C (one step a turn), five notes then again.
        let first: Vec<u8> = ons.iter().take(10).map(|o| o.1).collect();
        assert_eq!(first, vec![60, 62, 60, 59, 60, 60, 62, 60, 59, 60]);
        assert_eq!(fold(12, 10), -2);
        assert_eq!(fold(-7, 10), -3);
    }
}
