//! A third round of sequencers after composers and the ways they are known
//! by. Most play in the song's key; Bartók's mirrors and night noises,
//! Messiaen's modes and birds, Ligeti's and Cage's notes step out of it on
//! purpose, as theirs do.
//!
//! - Mozart: a minuet put together by dice, as in the musical dice game
//!   (two dice choose each of its sixteen bars from eleven), a Sonata (an
//!   Alberti bass under a classical period, a trill before the cadence) or
//!   Alla Turca (turns around each note, a semitone below it, over a
//!   Janissary oom-pah, and a loud refrain over rolled chords).
//! - Chopin: a Nocturne (a wide broken chord in the left hand, a singing
//!   line above it that takes its time while the left hand keeps it, grace
//!   notes and runs), a Waltz, or a Prelude (chords sinking a semitone at a
//!   time under a held melody).
//! - Ravel: Boléro. The side drum's rhythm and a plucked bass from the
//!   start, then a long tune and its answer, twice each, round and round,
//!   ever louder, doubled at the octave, then in parallel twelfths and
//!   seventeenths, then by everyone, up a third for the last bars, and the
//!   crash.
//! - Bartók: a Bulgarian dance in an uneven metre (2+2+3, 3+2+2, ...),
//!   Night Music (soft clusters, insects and frogs, a far tune) or a Mirror
//!   (two voices in exact contrary motion, semitone for semitone).
//! - Messiaen: chords in his modes of limited transposition with added
//!   values, birdsong (a blackbird, a thrush, a nightingale), or rhythmic
//!   characters (a palindromic rhythm three times: one growing, one
//!   shrinking, one staying the same).
//! - Ligeti: a hundred metronomes (as many as you like, each at its own
//!   tempo, running down one by one), Continuum (two hands as fast as they
//!   go, each round a few notes that change one at a time) or Musica
//!   ricercata (one pitch, then two, then three, ... until all twelve).
//! - Cage: chance (coins tossed for a hexagram of the I Ching, which picks
//!   a sound, a length and a loudness from a chart of sixty-four, some of
//!   them silence, the mobile ones changing as they are used), a prepared
//!   piano (a small gamut of odd sounds in quiet patterns, in a square-root
//!   form) or 4'33".
//! - Einaudi: a broken chord going round four chords with a few notes of
//!   melody over it, or a pattern that builds layer by layer and falls back.

use super::minimal::{choose, chord_root, hash, CHORD_AMOUNT, DYNAMICS};
use super::{SeqState, RATES};
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};
use std::f64::consts::{PI, TAU};

const KEYS: [ParamSpec; 2] = [ParamSpec::toggle("Transpose", "Keys", false), ParamSpec::toggle("Hold", "Keys", false)];

/// A bar's notes: (step, length in steps, degree over the chord's root).
type Fig = &'static [(u8, u8, i8)];

/// A chord's root brought near the tonic (-3 to 3), for the left hand.
fn low(root: i32) -> i32 {
    let r = root.rem_euclid(7);
    if r > 3 {
        r - 7
    } else {
        r
    }
}

/// The semitones of degree `d` above the tonic (0 to 11).
fn interval(ctx: &MidiCtx, base: f32, d: i32) -> i32 {
    (SeqState::mdeg(ctx, base, d, 0) - SeqState::mdeg(ctx, base, 0, 0)).rem_euclid(12)
}

fn minor(ctx: &MidiCtx, base: f32) -> bool {
    interval(ctx, base, 2) == 3
}

/// Eight notes so far, a byte each (64 the tonic), newest lowest.
fn push_hist(h: &mut u64, d: i32) {
    *h = (*h << 8) | ((d + 64).clamp(1, 255) as u64);
}

// ---------------------------------------------------------------- mozart

pub const MO_STYLE: usize = 1;
pub const MO_DICE: usize = 2;
pub const MO_REROLL: usize = 3;
pub const MO_BASE: usize = 4;
pub const MO_GATE: usize = 5;
pub const MO_VELOCITY: usize = 6;
/// (came later: after the keys and the dynamics)
pub const MO_CHORD_AMOUNT: usize = 13;
pub const MOZART_STYLES: &[&str] = &["Dice Minuet", "Sonata", "Alla Turca"];

pub static MOZART: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Style", "Mozart", MOZART_STYLES, 0),
    ParamSpec::int("Dice", "Mozart", 0.0, 99.0, 0.0, Unit::None),
    ParamSpec::toggle("Reroll", "Mozart", true),
    ParamSpec::int("Base Note", "Notes", 60.0, 84.0, 72.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
    CHORD_AMOUNT,
];

/// The minuet's sixteen bars: the chord of each (the first half ends on
/// the dominant, the second at home).
pub const MINUET_PLAN: [i32; 16] = [0, 4, 0, 3, 5, 1, 4, 4, 4, 0, 4, 0, 3, 0, 4, 0];

/// Eleven ways through a bar of the minuet, one for each throw of two dice
/// (2 to 12), in sixteenths of a bar of three.
pub const MINUET_BARS: [Fig; 11] = [
    &[(0, 4, 4), (4, 4, 2), (8, 4, 0)],
    &[(0, 2, 0), (2, 2, 2), (4, 2, 4), (6, 2, 7), (8, 4, 4)],
    &[(0, 6, 4), (6, 2, 3), (8, 4, 2)],
    &[(0, 1, 4), (1, 1, 5), (2, 1, 4), (3, 1, 3), (4, 4, 2), (8, 4, 0)],
    &[(0, 4, 7), (4, 2, 4), (6, 2, 2), (8, 4, 0)],
    &[(0, 2, 2), (2, 2, 4), (4, 2, 2), (6, 2, 0), (8, 2, 2), (10, 2, 4)],
    &[(0, 8, 4), (8, 4, 2)],
    &[(0, 2, 4), (2, 2, 3), (4, 2, 2), (6, 2, 1), (8, 4, 0)],
    &[(0, 4, 0), (4, 4, 4), (8, 4, 7)],
    &[(0, 1, 2), (1, 1, 3), (2, 1, 4), (3, 1, 5), (4, 2, 7), (6, 2, 4), (8, 4, 2)],
    &[(0, 4, 4), (4, 1, 5), (5, 1, 4), (6, 1, 3), (7, 1, 4), (8, 4, 7)],
];
const MINUET_END: Fig = &[(0, 8, 0)];

const SONATA_PLANS: [[i32; 8]; 3] = [[0, 4, 0, 4, 0, 3, 4, 0], [0, 3, 4, 0, 5, 1, 4, 0], [0, 0, 3, 4, 0, 3, 4, 0]];
/// Bars of four for the sonata's melody.
const SONATA_BARS: [Fig; 8] = [
    &[(0, 4, 4), (4, 4, 2), (8, 2, 0), (10, 2, 2), (12, 4, 4)],
    &[(0, 6, 0), (6, 2, 2), (8, 4, 4), (12, 4, 7)],
    &[(0, 8, 4), (8, 2, 5), (10, 2, 4), (12, 2, 3), (14, 2, 2)],
    &[(0, 4, 7), (4, 2, 6), (6, 2, 5), (8, 4, 4), (12, 4, 2)],
    &[(0, 2, 2), (2, 2, 4), (4, 2, 7), (6, 2, 4), (8, 4, 9), (12, 4, 7)],
    &[(0, 4, 4), (4, 1, 5), (5, 1, 4), (6, 1, 3), (7, 1, 4), (8, 4, 7), (12, 4, 4)],
    &[(0, 12, 4), (12, 2, 3), (14, 2, 2)],
    &[(0, 2, 0), (2, 2, 1), (4, 2, 2), (6, 2, 3), (8, 4, 4), (12, 4, 0)],
];
const SONATA_RUN: Fig = &[(0, 1, 7), (1, 1, 6), (2, 1, 5), (3, 1, 4), (4, 1, 3), (5, 1, 2), (6, 1, 1), (7, 1, 0), (8, 4, 2), (12, 4, 4)];
const HALF_CADENCE: Fig = &[(0, 6, 4), (6, 2, 2), (8, 8, 0)];
/// The trill before the cadence, from the note above, and its turn.
pub const TRILL: Fig = &[(0, 1, 5), (1, 1, 4), (2, 1, 5), (3, 1, 4), (4, 1, 5), (5, 1, 4), (6, 1, 5), (7, 1, 4), (8, 1, 5), (9, 1, 4), (10, 1, 5), (11, 1, 4), (12, 1, 5), (13, 1, 4), (14, 1, 3), (15, 1, 4)];
const CADENCE: Fig = &[(0, 4, 0)];

/// Alla Turca: (step, length, degree from the tonic, semitones off it).
type Turca = &'static [(u8, u8, i8, i8)];
const TURCA: [Turca; 8] = [
    &[(0, 1, 1, 0), (1, 1, 0, 0), (2, 1, 0, -1), (3, 1, 0, 0), (4, 2, 2, 0)],
    &[(0, 1, 3, 0), (1, 1, 2, 0), (2, 1, 2, -1), (3, 1, 2, 0), (4, 2, 4, 0)],
    &[(0, 1, 5, 0), (1, 1, 4, 0), (2, 1, 4, -1), (3, 1, 4, 0), (4, 1, 8, 0), (5, 1, 7, 0), (6, 1, 7, -1), (7, 1, 7, 0)],
    &[(0, 1, 8, 0), (1, 1, 7, 0), (2, 1, 7, -1), (3, 1, 7, 0), (4, 4, 9, 0)],
    &[(0, 1, 8, 0), (1, 1, 7, 0), (2, 1, 7, -1), (3, 1, 7, 0), (4, 2, 9, 0), (6, 2, 7, 0)],
    &[(0, 1, 6, 0), (1, 1, 5, 0), (2, 1, 5, -1), (3, 1, 5, 0), (4, 2, 7, 0), (6, 2, 5, 0)],
    &[(0, 1, 4, 0), (1, 1, 3, 0), (2, 1, 2, 0), (3, 1, 1, 0), (4, 2, 2, 0), (6, 2, -1, 0)],
    &[(0, 4, 0, 0)],
];
const TURCA_REFRAIN: [Turca; 8] = [
    &[(0, 2, 2, 0), (2, 2, 4, 0), (4, 1, 3, 0), (5, 1, 2, 0), (6, 1, 1, 0), (7, 1, 2, 0)],
    &[(0, 2, 4, 0), (2, 2, 2, 0), (4, 2, 0, 0), (6, 2, -3, 0)],
    &[(0, 2, 1, 0), (2, 2, 3, 0), (4, 1, 2, 0), (5, 1, 1, 0), (6, 1, 0, 0), (7, 1, 1, 0)],
    &[(0, 2, 2, 0), (2, 2, 0, 0), (4, 4, -3, 0)],
    &[(0, 2, 2, 0), (2, 2, 4, 0), (4, 1, 3, 0), (5, 1, 2, 0), (6, 1, 1, 0), (7, 1, 2, 0)],
    &[(0, 2, 4, 0), (2, 2, 2, 0), (4, 2, 0, 0), (6, 2, -3, 0)],
    &[(0, 2, 1, 0), (2, 2, 3, 0), (4, 2, 1, 0), (6, 2, -1, 0)],
    &[(0, 4, 0, 0)],
];
const TURCA_CHORDS: [i32; 8] = [0, 0, 0, 0, 0, 3, 4, 0];
const REFRAIN_CHORDS: [i32; 8] = [0, 0, 4, 4, 0, 0, 4, 0];

/// The throw of the two dice for bar `bar` of round `round`.
pub fn dice(seed: u32, round: u32, bar: usize) -> (u32, u32) {
    let s = seed.wrapping_mul(977).wrapping_add(round.wrapping_mul(31)) + 1;
    let a = 1 + (hash(s, bar as u32 * 2) * 6.0) as u32 % 6;
    let b = 1 + (hash(s, bar as u32 * 2 + 1) * 6.0) as u32 % 6;
    (a, b)
}

/// Where a bar's figure goes over its chord: the octave whose first note is
/// nearest the last one played, inside the melody's range.
fn voice(root: i32, fig: Fig, last: i32) -> i32 {
    let first = fig[0].2 as i32;
    let lo = fig.iter().map(|f| f.2 as i32).min().unwrap_or(0);
    let hi = fig.iter().map(|f| f.2 as i32).max().unwrap_or(0);
    let mut best = root;
    let mut dist = i32::MAX;
    for o in -2..=1 {
        let sh = root + 7 * o;
        if sh + lo < -1 || sh + hi > 12 {
            continue;
        }
        let d = (sh + first - last).abs();
        if d < dist {
            dist = d;
            best = sh;
        }
    }
    best
}

/// A chord's root brought just below the tonic (-6 to 0), for the
/// minuet's chords on two and three.
fn below(root: i32) -> i32 {
    let r = root.rem_euclid(7);
    if r > 0 {
        r - 7
    } else {
        0
    }
}

/// Bar `idx` of the minuet in round `round`: the dice's figure and where it
/// goes, each bar's octave led from the bar before (so a repeat is the same).
pub fn minuet_bar(seed: u32, round: u32, idx: usize, amount: f32) -> (usize, Fig, i32) {
    let mut last = 4;
    let mut out = (0, MINUET_END, 0);
    for i in 0..=idx.min(15) {
        let (a, b) = dice(seed, round, i);
        let f = (a + b - 2) as usize;
        let fig = if i == 15 { MINUET_END } else { MINUET_BARS[f] };
        let sh = voice(minuet_root(i, amount), fig, last);
        last = sh + fig[fig.len() - 1].2 as i32;
        out = (f, fig, sh);
    }
    out
}

/// The minuet's chord in bar `i`, with fewer Chords (`amount` below 1) home more often.
fn minuet_root(i: usize, amount: f32) -> i32 {
    chord_root(amount.min(1.0), i as i64, MINUET_PLAN[i % 16], MINUET_PLAN[(i + 1) % 16], false)
}

/// Bar `b` of the sonata's period `pi`: its chords (as Chords leaves
/// them), figure and where it goes.
fn sonata_bar(seed: u32, pi: u32, b: usize, amount: f32) -> ([i32; 8], Fig, i32) {
    let own = SONATA_PLANS[(hash(seed + 3, pi) * 3.0) as usize % 3];
    let plan: [i32; 8] = std::array::from_fn(|i| chord_root(amount.min(1.0), i as i64, own[i], own[(i + 1) % 8], false));
    let fa = (hash(seed * 7 + 1, pi) * 8.0) as usize % 8;
    let fb = (fa + 1 + (hash(seed * 7 + 2, pi) * 7.0) as usize % 7) % 8;
    let mut last = 4;
    let mut out = (CADENCE, 0);
    for (i, &root) in plan.iter().enumerate().take(b.min(7) + 1) {
        let fig: Fig = match i {
            0 | 1 | 4 => SONATA_BARS[fa],
            5 => SONATA_BARS[fb],
            2 => SONATA_RUN,
            3 => HALF_CADENCE,
            6 => TRILL,
            _ => CADENCE,
        };
        let sh = voice(root, fig, last);
        last = sh + fig[fig.len() - 1].2 as i32;
        out = (fig, sh);
    }
    (plan, out.0, out.1)
}

// ---------------------------------------------------------------- chopin

pub const CH_STYLE: usize = 1;
pub const CH_HARMONY: usize = 2;
pub const CH_RUBATO: usize = 3;
pub const CH_ORNAMENT: usize = 4;
pub const CH_BASE: usize = 5;
pub const CH_PEDAL: usize = 6;
pub const CH_VELOCITY: usize = 7;
pub const CH_CHORD_AMOUNT: usize = 14;
pub const CHOPIN_STYLES: &[&str] = &["Nocturne", "Waltz", "Prelude"];
pub const CHOPIN_HARMONIES: &[&str] = &["Romantic", "Simple", "Circle"];
pub const CHOPIN_ROOTS: [[i32; 8]; 3] = [[0, 5, 1, 4, 0, 3, 4, 0], [0, 3, 4, 0, 5, 3, 4, 0], [0, 3, 6, 2, 5, 1, 4, 0]];
/// Each step of Chopin is cut in twelve (for triplets and rubato).
pub const CHOPIN_TICKS: f64 = 12.0;

pub static CHOPIN: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 5),
    ParamSpec::choice("Style", "Chopin", CHOPIN_STYLES, 0),
    ParamSpec::choice("Harmony", "Chopin", CHOPIN_HARMONIES, 0),
    ParamSpec::lin("Rubato", "Chopin", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Ornaments", "Chopin", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Base Note", "Notes", 60.0, 84.0, 72.0, Unit::None),
    ParamSpec::lin("Pedal", "Notes", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.6, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
    CHORD_AMOUNT,
];

/// A nocturne's left hand: the bass, then up through the chord a tenth
/// and more wide, and down, in triplets.
pub const NOCTURNE_LH: [i32; 6] = [-14, -10, -5, -3, -5, -10];
/// Bars of four (in twelfths of a beat) for a nocturne's melody.
const NOCTURNE_RHYTHMS: [&[(u8, u8)]; 6] = [
    &[(0, 24), (24, 12), (36, 12)],
    &[(0, 36), (36, 12)],
    &[(0, 18), (18, 6), (24, 24)],
    &[(0, 12), (12, 12), (24, 16), (40, 8)],
    &[(0, 24), (24, 8), (32, 8), (40, 8)],
    &[(0, 30), (30, 6), (36, 6), (42, 6)],
];
/// Bars of three for a waltz.
const WALTZ_RHYTHMS: [&[(u8, u8)]; 5] = [
    &[(0, 12), (12, 6), (18, 6), (24, 12)],
    &[(0, 6), (6, 6), (12, 6), (18, 6), (24, 6), (30, 6)],
    &[(0, 18), (18, 6), (24, 12)],
    &[(0, 24), (24, 12)],
    &[(0, 4), (4, 4), (8, 4), (12, 12), (24, 12)],
];

// ---------------------------------------------------------------- ravel

pub const RA_LENGTH: usize = 1;
pub const RA_RHYTHM: usize = 2;
pub const RA_BASS: usize = 3;
pub const RA_THEME: usize = 4;
pub const RA_LAYERS: usize = 5;
pub const RA_BASE: usize = 6;
pub const RA_VELOCITY: usize = 7;
pub const RAVEL_RHYTHMS: &[&str] = &["Tonic", "Chord", "Off"];
pub const RAVEL_TICKS: f64 = 12.0;
/// The side drum's two bars, in twelfths of a beat: a quaver and a triplet
/// of semiquavers, twice, two quavers; then the same with two more triplets.
pub const BOLERO: [u8; 24] = [0, 6, 8, 10, 12, 18, 20, 22, 24, 30, 36, 42, 44, 46, 48, 54, 56, 58, 60, 62, 64, 66, 68, 70];
const BOLERO_BASS: [(u8, i32); 5] = [(0, -14), (24, -10), (36, -14), (48, -10), (60, -14)];
/// Bars of the tune, as note lengths (twelfths of a beat).
const BOLERO_CELLS: [&[u8]; 7] = [&[36], &[24, 12], &[12, 2, 2, 2, 6, 12], &[6, 6, 6, 6, 12], &[18, 6, 2, 2, 2, 6], &[12, 12, 4, 4, 4], &[24, 2, 2, 2, 6]];
/// Where each layer comes in (of the way through).
pub const RAVEL_LAYER_AT: [f32; 5] = [0.0, 0.2, 0.4, 0.6, 0.8];

pub static RAVEL: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 5),
    ParamSpec::int("Length", "Ravel", 16.0, 128.0, 64.0, Unit::None),
    ParamSpec::choice("Rhythm", "Ravel", RAVEL_RHYTHMS, 0),
    ParamSpec::toggle("Bass", "Ravel", true),
    ParamSpec::int("Theme", "Ravel", 0.0, 15.0, 0.0, Unit::None),
    ParamSpec::int("Layers", "Ravel", 1.0, 5.0, 5.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 60.0, 84.0, 72.0, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.95, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

/// One note of the tune: start and length (twelfths of a beat over its
/// sixteen bars), degree, and whether it is flattened (the second tune's
/// Moorish colour).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TuneNote {
    pub start: u16,
    pub len: u16,
    pub deg: i8,
    pub flat: bool,
}

/// The tune (or with `second` its answer) for `seed`: sixteen bars of
/// three, long held notes and turning triplets, hovering around the high
/// tonic and falling home in the second half.
pub fn bolero_tune(seed: u32, second: bool) -> Vec<TuneNote> {
    let s = seed.wrapping_mul(2).wrapping_add(second as u32).wrapping_add(17);
    let mut out = Vec::with_capacity(64);
    let mut d: i32 = if second { 9 } else { 7 };
    let mut tick = 0u16;
    for bar in 0..16u32 {
        let cell: &[u8] = match bar {
            15 => &[36],
            14 => &[24, 12],
            _ => BOLERO_CELLS[(hash(s, bar) * 7.0) as usize % 7],
        };
        let target = if bar < 8 { 7 } else { 7 - (bar as i32 - 8) };
        let mut trip = 0;
        for (j, &len) in cell.iter().enumerate() {
            let u = hash(s.wrapping_mul(13).wrapping_add(5), bar * 8 + j as u32);
            if len == 2 {
                // A turn: above, on, below.
                d += [1, -1, -1][trip % 3];
                trip += 1;
            } else {
                if trip > 0 {
                    // Back to the note the turn went round.
                    d += 1;
                    trip = 0;
                }
                let toward = (target - d).signum();
                d += if u < 0.45 { toward } else if u < 0.7 { 1 } else if u < 0.9 { -1 } else { 2 * toward };
                // A long note lands on the chord (the tonic's).
                if len >= 24 && ![0, 2, 4].contains(&d.rem_euclid(7)) {
                    d += if toward >= 0 { 1 } else { -1 };
                }
            }
            if bar == 15 {
                d = 0;
            }
            d = d.clamp(-1, 11);
            let flat = second && hash(s.wrapping_add(99), bar * 8 + j as u32) < 0.6;
            out.push(TuneNote { start: tick, len: len as u16, deg: d as i8, flat });
            tick += len as u16;
        }
    }
    out
}

// ---------------------------------------------------------------- bartók

pub const BT_STYLE: usize = 1;
pub const BT_METER: usize = 2;
pub const BT_MODE: usize = 3;
pub const BT_VARIATION: usize = 4;
pub const BT_BASE: usize = 5;
pub const BT_GATE: usize = 6;
pub const BT_VELOCITY: usize = 7;
pub const BARTOK_STYLES: &[&str] = &["Bulgarian", "Night Music", "Mirror"];
pub const BARTOK_METERS: &[&str] = &["2+2+3", "3+2+2", "2+2+2+3", "3+3+2", "2+3"];
pub const BARTOK_MODES: &[&str] = &["Acoustic", "In Key"];
pub const METERS: [&[u8]; 5] = [&[2, 2, 3], &[3, 2, 2], &[2, 2, 2, 3], &[3, 3, 2], &[2, 3]];
/// Each step (a quaver of the dance) is cut in two.
pub const BARTOK_TICKS: f64 = 2.0;

pub static BARTOK: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 4),
    ParamSpec::choice("Style", "Bartók", BARTOK_STYLES, 0),
    ParamSpec::choice("Metre", "Bartók", BARTOK_METERS, 0),
    ParamSpec::choice("Mode", "Bartók", BARTOK_MODES, 0),
    ParamSpec::int("Variation", "Bartók", 0.0, 15.0, 0.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 48.0, 84.0, 67.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

/// The acoustic scale's alterations: in major the fourth raised and the
/// seventh lowered (Lydian and Mixolydian at once), in minor the sixth and
/// seventh raised. Semitones to add to degree `d`.
fn acoustic(ctx: &MidiCtx, base: f32, d: i32) -> i32 {
    let iv = interval(ctx, base, d);
    if minor(ctx, base) {
        if iv == 8 || iv == 10 {
            1
        } else {
            0
        }
    } else {
        match iv {
            5 => 1,
            11 => -1,
            _ => 0,
        }
    }
}

/// A dance bar's tune for `seed` in the metre `groups`: for each quaver a
/// degree, or None where the note before is held.
pub fn dance_motif(seed: u32, groups: &[u8]) -> Vec<Option<i32>> {
    let mut out = Vec::new();
    let mut d = 2i32;
    let mut j = 0u32;
    for (g, &len) in groups.iter().enumerate() {
        for q in 0..len {
            let u = hash(seed.wrapping_mul(29).wrapping_add(3), j);
            let first = q == 0;
            if !first && hash(seed.wrapping_mul(7).wrapping_add(11), j) < 0.25 {
                out.push(None);
            } else {
                let step = if first && g > 0 && u < 0.3 {
                    if d > 3 {
                        -3
                    } else {
                        3
                    }
                } else if u < 0.4 {
                    1
                } else if u < 0.8 {
                    -1
                } else if u < 0.9 {
                    2
                } else {
                    -2
                };
                d = (d + step).clamp(-2, 7);
                out.push(Some(d));
            }
            j += 1;
        }
    }
    out
}

/// A mirror's line: semitones from the axis for sixteen quavers (None where
/// the note before is held).
pub fn mirror_line(seed: u32) -> [Option<i32>; 16] {
    const STEPS: [i32; 9] = [1, 2, 3, -1, -2, -3, 5, -5, 4];
    let mut out = [None; 16];
    let mut s = 0i32;
    for (j, slot) in out.iter_mut().enumerate() {
        if j == 0 {
            *slot = Some(0);
            continue;
        }
        if j % 4 != 0 && hash(seed.wrapping_add(41), j as u32) < 0.2 {
            continue;
        }
        let step = STEPS[(hash(seed.wrapping_mul(5).wrapping_add(1), j as u32) * 9.0) as usize % 9];
        s = if !(-4..=11).contains(&(s + step)) { s - step } else { s + step };
        *slot = Some(s);
    }
    out
}

/// A mirror's line as notes: (quaver, length in quavers, semitones).
pub fn mirror_notes(seed: u32) -> Vec<(usize, usize, i32)> {
    let line = mirror_line(seed);
    let mut out = Vec::with_capacity(16);
    for j in 0..16 {
        if let Some(v) = line[j] {
            let len = (j + 1..16).find(|&i| line[i].is_some()).unwrap_or(16) - j;
            out.push((j, len, v));
        }
    }
    out
}

// ---------------------------------------------------------------- messiaen

pub const ME_STYLE: usize = 1;
pub const ME_MODE: usize = 2;
pub const ME_BIRD: usize = 3;
pub const ME_DENSITY: usize = 4;
pub const ME_BASE: usize = 5;
pub const ME_VELOCITY: usize = 6;
pub const MESSIAEN_STYLES: &[&str] = &["Modes", "Birdsong", "Rhythms"];
pub const MESSIAEN_MODES: &[&str] = &["Mode 2", "Mode 3", "Mode 6"];
pub const MESSIAEN_BIRDS: &[&str] = &["Blackbird", "Thrush", "Nightingale"];
/// The modes of limited transposition, semitones from the tonic.
pub const MODES: [&[i32]; 3] = [&[0, 1, 3, 4, 6, 7, 9, 10], &[0, 2, 3, 4, 6, 7, 8, 10, 11], &[0, 2, 4, 5, 6, 8, 10, 11]];
/// Each step is cut in two (birds sing in demisemiquavers).
pub const MESSIAEN_TICKS: f64 = 2.0;
/// Durations in semiquavers, with the added values.
const ADDED: [u8; 8] = [4, 4, 5, 3, 6, 4, 7, 2];
pub const MAX_BIRD: usize = 48;

pub static MESSIAEN: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Style", "Messiaen", MESSIAEN_STYLES, 0),
    ParamSpec::choice("Mode", "Messiaen", MESSIAEN_MODES, 0),
    ParamSpec::choice("Bird", "Messiaen", MESSIAEN_BIRDS, 0),
    ParamSpec::lin("Density", "Messiaen", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Base Note", "Notes", 48.0, 84.0, 60.0, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.6, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

/// Note `i` of a mode over `tonic` (any number of octaves up or down).
pub fn mode_pitch(tonic: i32, mode: &[i32], i: i32) -> i32 {
    let n = mode.len() as i32;
    tonic + 12 * i.div_euclid(n) + mode[i.rem_euclid(n) as usize]
}

// ---------------------------------------------------------------- ligeti

pub const LI_STYLE: usize = 1;
pub const LI_VOICES: usize = 2;
pub const LI_SPREAD: usize = 3;
pub const LI_CHANGE: usize = 4;
pub const LI_BASE: usize = 5;
pub const LI_GATE: usize = 6;
pub const LI_VELOCITY: usize = 7;
pub const LIGETI_STYLES: &[&str] = &["Metronomes", "Continuum", "Ricercata"];
pub const MAX_METRONOMES: usize = 16;
const METRONOME_DEGREES: [i32; MAX_METRONOMES] = [0, 4, 2, 7, 5, 9, 3, 11, 1, 6, 8, 10, 12, -3, 14, -1];
pub const MAX_HAND: usize = 6;

pub static LIGETI: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Style", "Ligeti", LIGETI_STYLES, 0),
    ParamSpec::int("Voices", "Ligeti", 2.0, 16.0, 8.0, Unit::None),
    ParamSpec::lin("Spread", "Ligeti", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Change", "Ligeti", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Base Note", "Notes", 36.0, 84.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.6, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

/// Metronome `i` of round `c`: its tick (in beats) and how many ticks it
/// is wound for.
pub fn metronome(c: u32, i: usize, spread: f32, change: f32) -> (f64, u32) {
    let period = 0.3 + (0.15 + 0.9 * spread as f64) * hash(c.wrapping_mul(31).wrapping_add(7), i as u32) as f64;
    let winds = 24.0 + (1.25 - change.clamp(0.0, 1.0)) as f64 * 160.0 * hash(c.wrapping_mul(17).wrapping_add(3), i as u32) as f64;
    (period, winds as u32)
}

/// How long round `c` of the metronomes lasts (beats): the last one to run
/// down, and a bar of quiet.
pub fn metronome_round(c: u32, n: usize, spread: f32, change: f32) -> f64 {
    (0..n).map(|i| metronome(c, i, spread, change)).map(|(t, w)| t * w as f64).fold(0.0, f64::max) + 4.0
}

/// The order Musica ricercata takes its pitch classes in: the key's own
/// (tonic, fifth, second, sixth, third, seventh, fourth), then the rest.
pub fn ricercata_order(ctx: &MidiCtx, base: f32) -> [i32; 12] {
    let mut out = [0i32; 12];
    let mut n = 0;
    for d in [0, 4, 1, 5, 2, 6, 3] {
        let pc = interval(ctx, base, d);
        if !out[..n].contains(&pc) {
            out[n] = pc;
            n += 1;
        }
    }
    for pc in [1, 6, 8, 3, 10, 11, 4, 9, 5, 2, 7, 0] {
        if n < 12 && !out[..n].contains(&pc) {
            out[n] = pc;
            n += 1;
        }
    }
    out
}

// ---------------------------------------------------------------- cage

pub const CA_STYLE: usize = 1;
pub const CA_CHANGE: usize = 2;
pub const CA_SILENCE: usize = 3;
pub const CA_COINS: usize = 4;
pub const CA_BASE: usize = 5;
pub const CA_VELOCITY: usize = 6;
pub const CAGE_STYLES: &[&str] = &["Chance", "Prepared", "4'33\""];
/// The three movements of 4'33", in seconds (as at its first performance).
pub const MOVEMENTS: [f64; 3] = [30.0, 143.0, 100.0];
pub const PROPORTIONS: [&[u8]; 4] = [&[1, 1, 2, 1, 2], &[2, 3, 2], &[3, 3, 4, 2], &[1, 2, 1, 3]];
const LENGTHS: [u8; 9] = [1, 2, 3, 4, 5, 6, 8, 12, 16];
pub const GAMUT: usize = 10;

pub static CAGE: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Style", "Cage", CAGE_STYLES, 0),
    ParamSpec::lin("Change", "Cage", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Silence", "Cage", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::int("Coins", "Cage", 0.0, 63.0, 0.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 36.0, 84.0, 60.0, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.6, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

/// One sound of the chart: up to four notes (semitones from the base), how
/// long until the next, how loud (1 to 8). No notes: a silence.
#[derive(Clone, Copy, Default, Debug)]
pub struct Sound {
    pub notes: [i8; 4],
    pub n: u8,
    pub len: u8,
    pub dynamic: u8,
}

/// Six lines, each three coins (heads three, tails two): 6 and 9 are the
/// moving lines, odd ones whole (yang). The hexagram: a bit for each yang
/// line, the bottom one first.
pub fn hexagram(lines: &[u8; 6]) -> usize {
    lines.iter().enumerate().fold(0, |h, (i, &l)| h | ((l as usize & 1) << i))
}

// ---------------------------------------------------------------- einaudi

pub const EI_STYLE: usize = 1;
pub const EI_CHORDS: usize = 2;
pub const EI_MELODY: usize = 3;
pub const EI_BASE: usize = 4;
pub const EI_PEDAL: usize = 5;
pub const EI_VELOCITY: usize = 6;
pub const EI_CHORD_AMOUNT: usize = 13;
pub const EINAUDI_STYLES: &[&str] = &["Arpeggio", "Build"];
pub const EINAUDI_CHORDS: &[&str] = &["vi IV I V", "I V vi IV", "i VI III VII", "i iv VI V"];
pub const EINAUDI_ROOTS: [[i32; 4]; 4] = [[5, 3, 0, 4], [0, 4, 5, 3], [0, 5, 2, 6], [0, 3, 5, 4]];
/// Each step (a quaver) is cut in two.
pub const EINAUDI_TICKS: f64 = 2.0;
pub const EINAUDI_LH: [i32; 8] = [-14, -10, -7, -5, -7, -10, -7, -5];
/// Build: eight bars a layer, six of them, then round again.
pub const BUILD_STAGES: i64 = 6;

pub static EINAUDI: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 4),
    ParamSpec::choice("Style", "Einaudi", EINAUDI_STYLES, 0),
    ParamSpec::choice("Chords", "Einaudi", EINAUDI_CHORDS, 0),
    ParamSpec::lin("Melody", "Einaudi", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::int("Base Note", "Notes", 60.0, 84.0, 72.0, Unit::None),
    ParamSpec::lin("Pedal", "Notes", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.6, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
    CHORD_AMOUNT,
];

/// Einaudi's motif for phrase seed `seed`: two bars of quavers as
/// (quaver, length in quavers, degree from the tonic), a long note and a
/// sigh down.
pub fn einaudi_motif(seed: u32) -> Vec<(u8, u8, i32)> {
    const SHAPES: [&[(u8, u8)]; 4] = [&[(0, 4), (4, 2), (6, 2), (8, 6)], &[(0, 3), (3, 3), (6, 2), (8, 8)], &[(0, 6), (6, 2), (8, 4), (12, 4)], &[(0, 2), (2, 2), (4, 4), (8, 2), (10, 6)]];
    let shape = SHAPES[(hash(seed.wrapping_add(5), 1) * 4.0) as usize % 4];
    let mut d = [4, 2, 7, 5][(hash(seed.wrapping_add(5), 2) * 4.0) as usize % 4];
    shape
        .iter()
        .enumerate()
        .map(|(j, &(at, len))| {
            if j > 0 {
                let u = hash(seed.wrapping_add(9), j as u32);
                d += if u < 0.55 { -1 } else if u < 0.75 { 0 } else if u < 0.9 { 1 } else { -2 };
            }
            (at, len, d)
        })
        .collect()
}

// ---------------------------------------------------------------- state

pub struct Masters {
    rng: u32,
    /// The step before this one: anything else is a jump (a loop, a start).
    last_k: i64,
    // Mozart
    mo_last: i32,
    mo_hist: u64,
    // Chopin
    ch_line: i32,
    ch_rhythm: usize,
    ch_bar: i64,
    ch_chord: [i32; 3],
    ch_moves: u32,
    ch_voice: usize,
    ch_cadence: u8,
    ch_ready: bool,
    ch_hist: u64,
    // Ravel
    ra_key: Option<u32>,
    ra_tunes: [Vec<TuneNote>; 2],
    ra_hist: u64,
    // Bartók
    bt_cluster: i32,
    bt_tune: i32,
    bt_hist: u64,
    bt_low: u64,
    // Messiaen
    me_next: i64,
    me_top: i32,
    me_hist: u64,
    me_chord: u64,
    me_bird: [(u16, i8, u8); MAX_BIRD],
    me_n: usize,
    me_start: i64,
    me_rest: i64,
    me_cell: [u8; 5],
    me_vcell: [[u8; 5]; 3],
    me_vnext: [i64; 3],
    me_vpos: [u8; 3],
    me_vrep: [u32; 3],
    // Ligeti
    li_round: u32,
    li_start: f64,
    li_rh: [i8; MAX_HAND],
    li_rn: usize,
    li_lh: [i8; MAX_HAND],
    li_ln: usize,
    li_ready: bool,
    li_next: i64,
    li_left: bool,
    li_count: u32,
    li_hist: u64,
    // Cage
    ca_chart: [Sound; 64],
    ca_key: Option<(u32, u32)>,
    ca_next: i64,
    ca_lines: [u8; 6],
    ca_silent: bool,
    ca_gamut: [Sound; GAMUT],
    // Einaudi
    ei_top: i32,
    ei_hist: u64,
}

impl Default for Masters {
    fn default() -> Self {
        Self {
            rng: 0x3A57_E125,
            last_k: i64::MIN,
            mo_last: 4,
            mo_hist: 0,
            ch_line: 4,
            ch_rhythm: 0,
            ch_bar: i64::MIN,
            ch_chord: [0; 3],
            ch_moves: 0,
            ch_voice: 0,
            ch_cadence: 0,
            ch_ready: false,
            ch_hist: 0,
            ra_key: None,
            ra_tunes: [Vec::new(), Vec::new()],
            ra_hist: 0,
            bt_cluster: 0,
            bt_tune: 2,
            bt_hist: 0,
            bt_low: 0,
            me_next: 0,
            me_top: 12,
            me_hist: 0,
            me_chord: 0,
            me_bird: [(0, 0, 0); MAX_BIRD],
            me_n: 0,
            me_start: 0,
            me_rest: 0,
            me_cell: [2, 3, 5, 3, 2],
            me_vcell: [[2, 3, 5, 3, 2]; 3],
            me_vnext: [0; 3],
            me_vpos: [0; 3],
            me_vrep: [0; 3],
            li_round: 0,
            li_start: 0.0,
            li_rh: [0; MAX_HAND],
            li_rn: 0,
            li_lh: [0; MAX_HAND],
            li_ln: 0,
            li_ready: false,
            li_next: 0,
            li_left: false,
            li_count: 0,
            li_hist: 0,
            ca_chart: [Sound::default(); 64],
            ca_key: None,
            ca_next: 0,
            ca_lines: [7; 6],
            ca_silent: false,
            ca_gamut: [Sound::default(); GAMUT],
            ei_top: 7,
            ei_hist: 0,
        }
    }
}

impl Masters {
    fn unit(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / 16_777_216.0
    }

    fn bits(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    /// True when `k` does not follow the last step (and remembers it).
    fn jumped(&mut self, k: i64) -> bool {
        let j = self.last_k == i64::MIN || k != self.last_k + 1;
        self.last_k = k;
        j
    }

    /// A new random sound for Cage's chart: silent with `silence`.
    fn sound(&mut self, silence: f32) -> Sound {
        let len = LENGTHS[(self.unit() * LENGTHS.len() as f32) as usize % LENGTHS.len()];
        if self.unit() < silence {
            return Sound { notes: [0; 4], n: 0, len, dynamic: 0 };
        }
        let u = self.unit();
        let n = if u < 0.4 { 1 } else if u < 0.7 { 2 } else if u < 0.9 { 3 } else { 4 };
        let mut notes = [0i8; 4];
        for slot in notes.iter_mut().take(n) {
            *slot = (self.unit() * 60.0) as i8 - 24;
        }
        Sound { notes, n: n as u8, len, dynamic: 1 + (self.unit() * 8.0) as u8 % 8 }
    }
}

impl SeqState {
    /// Plays the notes of `fig` that start on step `t`, `shift` degrees up;
    /// returns the degree of the last one.
    #[allow(clippy::too_many_arguments)]
    fn play_fig(&mut self, ctx: &MidiCtx, base: f32, fig: Fig, t: u8, shift: i32, vel: f32, step: f64, gate: f64, at: u64, out: &mut Vec<MidiEvent>) -> Option<i32> {
        let mut last = None;
        for &(st, len, d) in fig {
            if st == t {
                let deg = shift + d as i32;
                let pitch = Self::mdeg(ctx, base, deg, 0);
                self.note(ctx, at, pitch, vel, len as f64 * step * gate, out);
                last = Some(deg);
            }
        }
        last
    }

    /// A chord whose notes come one after another, a few milliseconds apart.
    fn rolled(&mut self, ctx: &MidiCtx, at: u64, pitches: &[i32], vel: f32, len: f64, out: &mut Vec<MidiEvent>) {
        let gap = 0.012 * ctx.sr as f64;
        for (j, &p) in pitches.iter().enumerate() {
            self.mlater(ctx, at + (j as f64 * gap) as u64, p, vel * (0.85 + 0.05 * j as f32), len, out);
        }
    }

    // ------------------------------------------------------------ mozart

    /// Mozart, step `k` (a semiquaver).
    pub(crate) fn mozart(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let style = choose(p, MO_STYLE, MOZART_STYLES.len());
        let seed = p[MO_DICE].round().max(0.0) as u32;
        let reroll = p[MO_REROLL] >= 0.5;
        let base = p[MO_BASE];
        let gate = p[MO_GATE].clamp(0.1, 1.0) as f64;
        let vel = p[MO_VELOCITY];
        let chords = p.get(MO_CHORD_AMOUNT).copied().unwrap_or(1.0).clamp(0.0, 2.0);
        let s = step_frames;
        let bar_len: i64 = match style {
            0 => 12,
            1 => 16,
            _ => 8,
        };
        let n = k.div_euclid(bar_len);
        let t = k.rem_euclid(bar_len) as u8;
        self.masters.jumped(k);
        match style {
            // The dice minuet: A A B B, each of its sixteen bars thrown for.
            0 => {
                let round = if reroll { (n.div_euclid(32)) as u32 } else { 0 };
                let pos = n.rem_euclid(32) as usize;
                let idx = if pos < 16 { pos % 8 } else { 8 + (pos - 16) % 8 };
                let root = minuet_root(idx, chords);
                let (f, fig, shift) = minuet_bar(seed, round, idx, chords);
                // More Chords: the left hand's second half on a passing chord.
                let lh_root = if t >= 6 { chord_root(chords, idx as i64, root, minuet_root(idx + 1, chords), true) } else { root };
                if let Some(d) = self.play_fig(ctx, base, fig, t, shift, vel, s, gate, at, out) {
                    self.masters.mo_last = d;
                    push_hist(&mut self.masters.mo_hist, d);
                }
                // The left hand: the bass on one, the chord on two and three.
                let lr = low(root);
                if idx == 15 {
                    if t == 0 {
                        self.note(ctx, at, Self::mdeg(ctx, base, lr - 14, 0), vel * 0.7, 12.0 * s * 0.95, out);
                        for d in [-7, -5, -3] {
                            self.note(ctx, at, Self::mdeg(ctx, base, d, 0), vel * 0.5, 8.0 * s, out);
                        }
                    }
                } else if t == 0 {
                    self.note(ctx, at, Self::mdeg(ctx, base, lr - 14, 0), vel * 0.7, 4.0 * s * gate, out);
                } else if t == 4 || t == 8 {
                    for d in [2, 4] {
                        self.note(ctx, at, Self::mdeg(ctx, base, below(lh_root) - 7 + d, 0), vel * 0.48, 2.5 * s * gate, out);
                    }
                }
                let (a, b) = dice(seed, round, idx);
                let table = (0..16).fold(0u64, |acc, i| {
                    let (a, b) = dice(seed, round, i);
                    acc | ((a + b - 2) as u64) << (4 * i)
                });
                self.visual[0] = (style as u64) | (idx as u64) << 4 | (f.min(10) as u64) << 8 | (a as u64) << 12 | (b as u64) << 16 | (round as u64 & 0xff) << 20 | (t as u64) << 28;
                self.visual[1] = table;
                self.visual[2] = self.masters.mo_hist;
            }
            // A sonata: two four-bar phrases over an Alberti bass.
            1 => {
                let pi = if reroll { n.div_euclid(8) } else { n.div_euclid(8).rem_euclid(2) } as u32;
                let b = n.rem_euclid(8) as usize;
                let (plan, fig, shift) = sonata_bar(seed, pi, b, chords);
                let root = plan[b];
                let lh_root = if t >= 8 { chord_root(chords, b as i64, root, plan[(b + 1) % 8], true) } else { root };
                if let Some(d) = self.play_fig(ctx, base, fig, t, shift, vel, s, gate, at, out) {
                    self.masters.mo_last = d;
                    push_hist(&mut self.masters.mo_hist, d);
                }
                let lr = low(root);
                if b == 7 {
                    if t == 0 {
                        self.note(ctx, at, Self::mdeg(ctx, base, lr - 14, 0), vel * 0.6, 8.0 * s, out);
                        for d in [-7, -5, -3] {
                            self.note(ctx, at, Self::mdeg(ctx, base, lr + d, 0), vel * 0.5, 4.0 * s, out);
                        }
                    }
                } else {
                    // Low, high, middle, high.
                    const ALBERTI: [i32; 4] = [0, 4, 2, 4];
                    let d = low(lh_root) - 7 + ALBERTI[t as usize % 4];
                    self.note(ctx, at, Self::mdeg(ctx, base, d, 0), vel * if t % 4 == 0 { 0.5 } else { 0.4 }, s * gate * 0.9, out);
                }
                self.visual[0] = (style as u64) | (b as u64) << 4 | (t as u64) << 8 | ((root + 64) as u64 & 0xff) << 16;
                self.visual[1] = plan.iter().enumerate().fold(0u64, |acc, (i, &r)| acc | ((r.rem_euclid(7) as u64) << (4 * i)));
                self.visual[2] = self.masters.mo_hist;
            }
            // Alla Turca: turns over an oom-pah, A A; the refrain over rolled chords, twice.
            _ => {
                let pos = n.rem_euclid(32) as usize;
                let refrain = pos >= 16;
                let b = pos % 8;
                let notes = if refrain { TURCA_REFRAIN[b] } else { TURCA[b] };
                for &(st, len, d, c) in notes {
                    if st == t {
                        let pitch = Self::mdeg(ctx, base, d as i32, c as i32);
                        let len = len as f64 * s * gate;
                        if refrain {
                            self.note(ctx, at, pitch, vel, len, out);
                            self.note(ctx, at, Self::mdeg(ctx, base, d as i32 - 7, c as i32), vel * 0.85, len, out);
                        } else {
                            self.note(ctx, at, pitch, vel * 0.75, len, out);
                        }
                        push_hist(&mut self.masters.mo_hist, d as i32);
                    }
                }
                let chords_of = if refrain { REFRAIN_CHORDS } else { TURCA_CHORDS };
                let root = chord_root(chords, b as i64, chords_of[b], chords_of[(b + 1) % 8], t >= 4);
                let lr = low(root);
                if refrain {
                    if t % 2 == 0 && !(b == 7 && t > 0) {
                        let chord = [-14, -7, -5, -3].map(|d| Self::mdeg(ctx, base, lr + d, 0));
                        self.rolled(ctx, at, &chord, vel * 0.8, 2.0 * s * gate, out);
                    }
                } else if t == 0 || (t == 4 && b != 7) {
                    self.note(ctx, at, Self::mdeg(ctx, base, lr - 14, 0), vel * 0.6, 2.0 * s * gate, out);
                } else if (t == 2 || t == 6) && b != 7 {
                    for d in [2, 4] {
                        self.note(ctx, at, Self::mdeg(ctx, base, lr - 14 + d, 0), vel * 0.45, 1.6 * s * gate, out);
                    }
                }
                self.visual[0] = (style as u64) | (b as u64) << 4 | (t as u64) << 8 | ((root + 64) as u64 & 0xff) << 16 | (refrain as u64) << 24;
                let chords = if refrain { REFRAIN_CHORDS } else { TURCA_CHORDS };
                self.visual[1] = chords.iter().enumerate().fold(0u64, |acc, (i, &r)| acc | ((r.rem_euclid(7) as u64) << (4 * i)));
                self.visual[2] = self.masters.mo_hist;
            }
        }
    }

    // ------------------------------------------------------------ chopin

    /// The next note of Chopin's line: on a strong beat to the chord, else
    /// a step (down, mostly).
    fn chopin_next(&mut self, root: i32, strong: bool, phrase_start: bool) -> i32 {
        let m = &mut self.masters;
        let u = m.unit();
        let mut d = m.ch_line;
        if phrase_start {
            // A phrase begins with a leap up, as far as a sixth.
            d = (d + if u < 0.5 { 5 } else { 4 }).min(11);
            if d < 4 {
                d += 3;
            }
        } else {
            d += if u < 0.55 { -1 } else if u < 0.85 { 1 } else if u < 0.93 { 2 } else { -2 };
        }
        if strong {
            let in_chord = |x: i32| [0, 2, 4].contains(&(x - root).rem_euclid(7));
            if !in_chord(d) {
                d += if in_chord(d - 1) { -1 } else { 1 };
            }
        }
        d = d.clamp(0, 11);
        if d >= 10 && !phrase_start {
            d -= 2;
        }
        m.ch_line = d;
        push_hist(&mut m.ch_hist, d);
        d
    }

    /// Chopin, step `k` (a twelfth of a beat).
    pub(crate) fn chopin(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let style = choose(p, CH_STYLE, CHOPIN_STYLES.len());
        let roots = CHOPIN_ROOTS[choose(p, CH_HARMONY, CHOPIN_HARMONIES.len())];
        let rubato = p[CH_RUBATO].clamp(0.0, 1.0) as f64;
        let ornament = p[CH_ORNAMENT].clamp(0.0, 1.0);
        let base = p[CH_BASE];
        let pedal = p[CH_PEDAL].clamp(0.0, 1.0) as f64;
        let vel = p[CH_VELOCITY];
        let s = step_frames;
        let beat = 12.0 * s;
        let bar_len: i64 = if style == 1 { 36 } else { 48 };
        let bar = k.div_euclid(bar_len);
        let t = k.rem_euclid(bar_len) as u8;
        self.masters.jumped(k);
        let chords = p.get(CH_CHORD_AMOUNT).copied().unwrap_or(1.0).clamp(0.0, 2.0);
        let ci = bar.rem_euclid(8) as usize;
        let root = chord_root(chords, ci as i64, roots[ci], roots[(ci + 1) % 8], 2 * t as i64 >= bar_len);
        let lr = low(root);
        let in_phrase = bar.rem_euclid(4);
        // The right hand takes its time (late, then catching up) while the left keeps it.
        let sway = 0.5 + 0.5 * (TAU * k as f64 / (2.0 * bar_len as f64)).sin();
        let late = rubato * beat * (0.18 * sway + if in_phrase == 3 { 0.12 * t as f64 / bar_len as f64 } else { 0.0 });
        let arch = (0.8 + 0.2 * (PI * (in_phrase as f64 + t as f64 / bar_len as f64) / 4.0).sin()) as f32;
        let new_bar = self.masters.ch_bar != bar;
        if new_bar {
            self.masters.ch_bar = bar;
            let u = self.masters.unit();
            self.masters.ch_rhythm = (u * 64.0) as usize;
        }
        match style {
            // Prelude: chords sinking a semitone at a time, a long note above.
            2 => {
                let tonic_chord = |ctx: &MidiCtx| [Self::mdeg(ctx, base, -5, 0), Self::mdeg(ctx, base, -3, 0), Self::mdeg(ctx, base, 0, 0)];
                if !self.masters.ch_ready {
                    self.masters.ch_ready = true;
                    self.masters.ch_chord = tonic_chord(ctx);
                    self.masters.ch_line = 4;
                }
                if t == 0 || t == 24 {
                    let m = &mut self.masters;
                    match m.ch_cadence {
                        // The dominant (a seventh, its leading tone raised).
                        1 if t == 0 => {
                            let lead = if minor(ctx, base) { 1 } else { 0 };
                            m.ch_chord = [Self::mdeg(ctx, base, -8, lead), Self::mdeg(ctx, base, -6, 0), Self::mdeg(ctx, base, -4, 0)];
                            m.ch_cadence = 2;
                        }
                        2 if t == 0 => {
                            m.ch_chord = tonic_chord(ctx);
                            m.ch_cadence = 0;
                            m.ch_moves = 0;
                            m.ch_line = 4;
                        }
                        0 => {
                            // Chords: how often a voice sinks (none at 0, now and then twice as often).
                            let extra = m.unit() < 0.85 * (chords - 1.0).max(0.0);
                            if m.unit() < 0.85 * chords.min(1.0) || extra {
                                // One voice down a semitone, never onto another.
                                const ORDER: [usize; 3] = [2, 0, 1];
                                let v = ORDER[m.ch_voice % 3];
                                m.ch_voice += 1;
                                let mut x = m.ch_chord[v] - 1;
                                while m.ch_chord.iter().enumerate().any(|(i, &c)| i != v && c == x) {
                                    x -= 1;
                                }
                                m.ch_chord[v] = x;
                                m.ch_chord.sort();
                                m.ch_moves += 1;
                            }
                            if m.ch_moves >= 11 && t == 24 {
                                m.ch_cadence = 1;
                            }
                        }
                        _ => {}
                    }
                    if t == 0 && in_phrase == 0 && m.ch_cadence == 0 && m.ch_moves > 0 && m.ch_line > 1 && m.unit() < 0.5 {
                        m.ch_line -= 1;
                    }
                }
                if t % 6 == 0 {
                    let chord = self.masters.ch_chord;
                    for &c in &chord {
                        self.note(ctx, at, c, vel * 0.36, 5.5 * s, out);
                    }
                    if t == 0 {
                        self.note(ctx, at, chord[0] - 12, vel * 0.45, bar_len as f64 * s * (0.5 + 0.45 * pedal), out);
                    }
                }
                // The melody: one note a bar, or its neighbour on the last beat.
                let line = self.masters.ch_line;
                let cad = self.masters.ch_cadence;
                let notes: &[(u8, u8, i32)] = match cad {
                    2 => &[(0, 24, 0), (24, 12, 1), (36, 12, -1)],
                    _ if bar.rem_euclid(2) == 0 => &[(0, 48, 0)],
                    _ => &[(0, 36, 0), (36, 12, 1)],
                };
                for &(st, len, d) in notes {
                    if st == t {
                        let deg = if cad == 2 { d } else { line + d };
                        let pitch = Self::mdeg(ctx, base, deg, if cad == 2 && d == -1 && minor(ctx, base) { 1 } else { 0 });
                        self.mlater(ctx, at + late as u64, pitch, vel * 0.7 * arch, len as f64 * s * 0.98, out);
                        push_hist(&mut self.masters.ch_hist, deg);
                    }
                }
                if t == 36 && bar.rem_euclid(2) == 0 && self.masters.unit() < ornament * 0.3 {
                    // A turn.
                    for (j, d) in [1, 0, -1, 0].iter().enumerate() {
                        let pitch = Self::mdeg(ctx, base, line + d, 0);
                        self.mlater(ctx, at + late as u64 + (j as f64 * 3.0 * s) as u64, pitch, vel * 0.55, 2.8 * s, out);
                    }
                }
                let c = self.masters.ch_chord;
                self.visual[1] = self.masters.ch_hist;
                self.visual[2] = c.iter().enumerate().fold(0u64, |acc, (i, &x)| acc | ((x.clamp(0, 127) as u64) << (8 * i))) | (self.masters.ch_moves as u64 & 0xff) << 24 | (self.masters.ch_cadence as u64) << 32;
            }
            // Nocturne and Waltz.
            _ => {
                if style == 0 {
                    // The left hand: wide triplets, the bass held in the pedal.
                    if t % 4 == 0 {
                        let i = (t / 4) as usize % 6;
                        let pitch = Self::mdeg(ctx, base, lr + NOCTURNE_LH[i], 0);
                        let (v, len) = if t == 0 { (0.6, (4.0 + 40.0 * pedal) * s) } else if i == 0 { (0.5, (4.0 + 18.0 * pedal) * s) } else { (0.4, (4.0 + 8.0 * pedal) * s) };
                        self.note(ctx, at, pitch, vel * v, len, out);
                    }
                } else if t == 0 {
                    self.note(ctx, at, Self::mdeg(ctx, base, lr - 14, 0), vel * 0.6, (12.0 + 12.0 * pedal) * s, out);
                } else if t == 12 || t == 24 {
                    for d in [0, 2, 4] {
                        self.note(ctx, at, Self::mdeg(ctx, base, lr - 7 + d, 0), vel * 0.38, 5.0 * s, out);
                    }
                }
                // The right hand.
                let last_bar = in_phrase == 3;
                let rhythms: &[&[(u8, u8)]] = if style == 0 { &NOCTURNE_RHYTHMS } else { &WALTZ_RHYTHMS };
                let rhythm: &[(u8, u8)] = if last_bar {
                    if style == 0 {
                        &[(0, 36)]
                    } else {
                        &[(0, 24)]
                    }
                } else {
                    rhythms[self.masters.ch_rhythm % rhythms.len()]
                };
                let running = style == 1 && !last_bar && rhythm.len() == 6;
                for (j, &(st, len)) in rhythm.iter().enumerate() {
                    if st != t {
                        continue;
                    }
                    let deg = if running && j > 0 {
                        // Circling round the bar's first note.
                        let first = self.masters.ch_line - [0, 1, 0, -1, 0, 2][j - 1];
                        let d = first + [0, 1, 0, -1, 0, 2][j];
                        self.masters.ch_line = d;
                        push_hist(&mut self.masters.ch_hist, d);
                        d
                    } else {
                        self.chopin_next(root, st == 0 || st == 24, in_phrase == 0 && st == 0)
                    };
                    let pitch = Self::mdeg(ctx, base, deg, 0);
                    let mut when = at + late as u64;
                    let len_f = len as f64 * s * 0.97;
                    if len >= 12 && self.masters.unit() < ornament * 0.25 {
                        // A grace note from above.
                        let grace = (0.05 * ctx.sr as f64) as u64;
                        self.mlater(ctx, when, Self::mdeg(ctx, base, deg + 1, 0), vel * 0.5 * arch, grace as f64 * 0.9, out);
                        when += grace;
                    }
                    self.mlater(ctx, when, pitch, vel * arch * if st == 0 { 1.0 } else { 0.88 }, len_f, out);
                }
                // A run at a nocturne's phrase end: up the chord, then down, faster and softer.
                if style == 0 && last_bar && t == 36 && self.masters.unit() < ornament {
                    let line = self.masters.ch_line;
                    let n = 7 + (ornament * 8.0) as usize;
                    let mut degs: Vec<i32> = vec![line + 2, line + 4, line + 7];
                    for j in 0..n - 3 {
                        degs.push(line + 6 - j as i32);
                    }
                    let span = beat * 0.95;
                    for (j, &d) in degs.iter().enumerate() {
                        let x = (j as f64 / degs.len() as f64).powf(0.85);
                        let pitch = Self::mdeg(ctx, base, d, 0);
                        self.mlater(ctx, at + late as u64 + (x * span) as u64, pitch, vel * (0.7 - 0.25 * j as f32 / degs.len() as f32), span / degs.len() as f64 * 1.3, out);
                    }
                    let end = *degs.last().unwrap_or(&line);
                    self.masters.ch_line = end;
                    push_hist(&mut self.masters.ch_hist, end);
                }
                self.visual[1] = self.masters.ch_hist;
                self.visual[2] = (late / beat.max(1.0) * 255.0) as u64 & 0xff;
            }
        }
        self.visual[0] = (style as u64) | (bar.rem_euclid(8) as u64) << 4 | (t as u64) << 8 | ((root + 64) as u64 & 0xff) << 16;
        self.visual[3] = roots.iter().enumerate().fold(0u64, |acc, (i, &r)| acc | ((r.rem_euclid(7) as u64) << (4 * i)));
    }

    // ------------------------------------------------------------ ravel

    /// Ravel, step `k` (a twelfth of a beat).
    pub(crate) fn ravel(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let length = ((p[RA_LENGTH].round() as i64 / 16).max(1) * 16).clamp(16, 128);
        let rhythm = choose(p, RA_RHYTHM, RAVEL_RHYTHMS.len());
        let bass_on = p[RA_BASS] >= 0.5;
        let theme = p[RA_THEME].round().max(0.0) as u32;
        let max_layers = (p[RA_LAYERS].round() as usize).clamp(1, 5);
        let base = p[RA_BASE];
        let vel = p[RA_VELOCITY];
        let s = step_frames;
        let _ = self.masters.jumped(k);
        if self.masters.ra_key != Some(theme) {
            self.masters.ra_key = Some(theme);
            self.masters.ra_tunes = [bolero_tune(theme, false), bolero_tune(theme, true)];
        }
        // Two bars of drum and bass, the tunes, the ending, two bars of quiet.
        let cycle = length + 4;
        let bar = k.div_euclid(36);
        let cb = bar.rem_euclid(cycle);
        let t = k.rem_euclid(36) as u16;
        let x = (cb as f32 / (length + 2) as f32).min(1.0);
        let dynamic = 0.12 + 0.88 * x.powf(1.4);
        let layers = RAVEL_LAYER_AT.iter().filter(|&&a| x >= a).count().min(max_layers);
        let pos = (cb.rem_euclid(2) * 36) as u8 + t as u8;
        let silent = cb >= length + 2;
        // The last eight bars of the tunes a major third up.
        let lift = if length >= 32 && cb >= length - 6 && cb < length { 4 } else { 0 };
        let v = vel * dynamic;
        if !silent {
            // The side drum's rhythm (on the tonic), and the plucked bass.
            if rhythm < 2 && BOLERO.contains(&pos) {
                let accent = if t == 0 { 1.0 } else { 0.8 };
                let notes: &[i32] = match (rhythm, layers >= 4) {
                    (1, true) => &[-7, -3, -14],
                    (1, false) => &[-7, -3],
                    _ => &[-7],
                };
                for &d in notes {
                    self.note(ctx, at, Self::mdeg(ctx, base, d, 0) + lift, v * 0.5 * accent, 1.5 * s, out);
                }
            }
            if bass_on {
                for &(bt, d) in &BOLERO_BASS {
                    if bt == pos {
                        let pitch = Self::mdeg(ctx, base, d, 0) + lift;
                        self.note(ctx, at, pitch, v * 0.7, 6.0 * s, out);
                        if layers >= 4 {
                            self.note(ctx, at, pitch + 12, v * 0.5, 6.0 * s, out);
                        }
                    }
                }
            }
        }
        if cb >= 2 && cb < length {
            let tb = cb - 2;
            let second = (tb / 16).rem_euclid(4) >= 2;
            let tick = (tb.rem_euclid(16) * 36) as u16 + t;
            for i in 0..self.masters.ra_tunes[second as usize].len() {
                let nt = self.masters.ra_tunes[second as usize][i];
                if nt.start != tick {
                    continue;
                }
                let d = nt.deg as i32;
                let flat = if nt.flat && [2, 4, 9, 11].contains(&interval(ctx, base, d)) { -1 } else { 0 };
                let pitch = Self::mdeg(ctx, base, d, flat) + lift;
                let len = nt.len as f64 * s * 0.98;
                let vm = v * 0.9;
                self.note(ctx, at, pitch, vm, len, out);
                if layers >= 2 {
                    self.note(ctx, at, pitch + 12, vm * 0.7, len, out);
                }
                if layers >= 3 {
                    // Ravel's mixture: the tune again at the twelfth and the seventeenth.
                    self.note(ctx, at, pitch + 19, vm * 0.45, len, out);
                    self.note(ctx, at, pitch + 28, vm * 0.35, len, out);
                }
                if layers >= 4 {
                    self.note(ctx, at, pitch - 12, vm * 0.75, len, out);
                }
                if layers >= 5 {
                    self.note(ctx, at, Self::mdeg(ctx, base, d - 2, flat) + lift, vm * 0.7, len, out);
                }
                push_hist(&mut self.masters.ra_hist, d);
            }
        } else if cb == length {
            // The collapse: everyone down in triplets.
            if t % 2 == 0 {
                let top = Self::mdeg(ctx, base, 7, 0);
                let pitch = top - (t as i32 * 24) / 36;
                self.note(ctx, at, pitch, vel * (0.9 - 0.2 * t as f32 / 36.0), 2.0 * s, out);
                self.note(ctx, at, pitch - 12, vel * 0.8, 2.0 * s, out);
            }
            if t == 0 && bass_on {
                self.note(ctx, at, Self::mdeg(ctx, base, -14, 0), vel * 0.8, 36.0 * s, out);
            }
        } else if cb == length + 1 {
            // A crash a semitone up, then the tonic, and nothing.
            if t == 0 || t == 12 {
                let shift = if t == 0 { 1 } else { 0 };
                let len = if t == 0 { 6.0 } else { 20.0 } * s;
                let tonic = Self::mdeg(ctx, base, 0, 0);
                let third = if minor(ctx, base) { 3 } else { 4 };
                for o in [-24, -12, 0, 12] {
                    for iv in [0, third, 7] {
                        self.note(ctx, at, tonic + shift + o + iv, vel, len, out);
                    }
                }
            }
        }
        let tb = (cb - 2).max(0);
        let second = cb >= 2 && (tb / 16).rem_euclid(4) >= 2;
        self.visual[0] = ((x * 255.0) as u64) | (layers as u64) << 8 | (second as u64) << 12 | (silent as u64) << 13 | ((cb >= length) as u64) << 14 | (pos as u64) << 16 | (cb as u64 & 0xffff) << 24 | (length as u64) << 40;
        self.visual[1] = self.masters.ra_hist;
    }

    // ------------------------------------------------------------ bartók

    /// Bartók, step `k` (half a quaver).
    pub(crate) fn bartok(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let style = choose(p, BT_STYLE, BARTOK_STYLES.len());
        let meter = choose(p, BT_METER, BARTOK_METERS.len());
        let acoustic_on = choose(p, BT_MODE, BARTOK_MODES.len()) == 0;
        let var = p[BT_VARIATION].round().max(0.0) as u32;
        let base = p[BT_BASE];
        let gate = p[BT_GATE].clamp(0.1, 1.0) as f64;
        let vel = p[BT_VELOCITY];
        let s = step_frames;
        let q = 2.0 * s;
        let _ = self.masters.jumped(k);
        let alter = |ctx: &MidiCtx, d: i32| if acoustic_on { acoustic(ctx, base, d) } else { 0 };
        match style {
            // A Bulgarian dance: accents where the groups begin.
            0 => {
                let groups = METERS[meter];
                let eighths: i64 = groups.iter().map(|&g| g as i64).sum();
                let bar = k.div_euclid(2 * eighths);
                let tq = k.rem_euclid(2 * eighths);
                let j = (tq / 2) as usize;
                let half = tq % 2;
                let mut starts = Vec::with_capacity(4);
                let mut acc = 0usize;
                for &g in groups {
                    starts.push(acc);
                    acc += g as usize;
                }
                let gi = starts.iter().rposition(|&st| st <= j).unwrap_or(0);
                let group_start = starts[gi] == j;
                let phrase = bar.div_euclid(4);
                let pb = bar.rem_euclid(4);
                let motif = dance_motif(var.wrapping_mul(97).wrapping_add((phrase / 2) as u32), groups);
                // The phrase: the motif, a step higher, again, and home.
                let (shift, cadence) = match pb {
                    1 => (1, false),
                    3 => (0, true),
                    _ => (0, false),
                };
                if half == 0 {
                    let note = if cadence && gi > 0 {
                        if group_start {
                            Some(if gi + 1 == groups.len() { 0 } else { 1 })
                        } else {
                            None
                        }
                    } else {
                        motif[j].map(|d| d + shift)
                    };
                    if let Some(d) = note {
                        // How long: until the next note.
                        let mut len = 1;
                        while j + len < motif.len() && (motif[j + len].is_none() || (cadence && gi > 0 && !starts.contains(&(j + len)))) {
                            len += 1;
                        }
                        let accent = if group_start { 1.0 } else { 0.7 };
                        let pitch = Self::mdeg(ctx, base, d, alter(ctx, d));
                        let snap = group_start && hash(var + 5, (bar * 8 + j as i64) as u32) < 0.3;
                        if snap {
                            // A grace from above, snapped onto the beat.
                            self.note(ctx, at, Self::mdeg(ctx, base, d + 1, alter(ctx, d + 1)), vel * 0.6, s * 0.8, out);
                            self.mlater(ctx, at + s as u64, pitch, vel * accent, (len as f64 * q - s) * gate, out);
                        } else {
                            self.note(ctx, at, pitch, vel * accent, len as f64 * q * gate, out);
                        }
                        self.masters.bt_tune = d;
                        push_hist(&mut self.masters.bt_hist, d);
                    }
                    if group_start {
                        // A drone at the bar, and a stab on every group (the last one rubs).
                        if j == 0 {
                            for d in [-14, -10] {
                                self.note(ctx, at, Self::mdeg(ctx, base, d, 0), vel * 0.5, eighths as f64 * q * 0.95, out);
                            }
                        }
                        let stab: &[i32] = if gi + 1 == groups.len() && groups.len() > 2 { &[-8, -5, -4] } else { &[-7, -4, -3] };
                        for &d in stab {
                            self.note(ctx, at, Self::mdeg(ctx, base, d, alter(ctx, d)), vel * 0.5 * if j == 0 { 1.0 } else { 0.85 }, q * 0.8, out);
                        }
                    }
                }
                let packed = groups.iter().enumerate().fold(0u64, |a, (i, &g)| a | (g as u64) << (4 * i));
                self.visual[0] = (style as u64) | (j as u64) << 4 | (groups.len() as u64) << 12 | packed << 16 | (pb as u64) << 40;
                self.visual[1] = self.masters.bt_hist;
            }
            // Night music.
            1 => {
                let bar = k.div_euclid(16);
                let t = k.rem_euclid(16);
                let tonic = Self::mdeg(ctx, base, 0, 0);
                if t == 0 && bar.rem_euclid(2) == 0 {
                    let m = &mut self.masters;
                    let u = m.unit();
                    m.bt_cluster = (m.bt_cluster + if u < 0.3 { 1 } else if u < 0.6 { -1 } else { 0 }).clamp(-4, 4);
                    let n = 3 + (m.unit() * 3.0) as i32;
                    let c = m.bt_cluster;
                    for i in 0..n {
                        self.note(ctx, at, tonic - 12 + c + i, vel * 0.22, 32.0 * s * 0.95, out);
                    }
                }
                // Insects, now and then, and a frog.
                let m = &mut self.masters;
                if m.unit() < 0.12 + 0.08 * (var % 4) as f32 {
                    let kind = m.unit();
                    let pitch = tonic + 12 + (m.unit() * 26.0) as i32;
                    let v = vel * (0.25 + 0.3 * m.unit());
                    if kind < 0.5 {
                        self.note(ctx, at, pitch, v, s * 0.5, out);
                        push_hist(&mut self.masters.bt_low, pitch - tonic);
                    } else if kind < 0.85 {
                        let reps = 3 + (self.masters.unit() * 3.0) as usize;
                        for r in 0..reps {
                            self.mlater(ctx, at + (r as f64 * s * 0.5) as u64, pitch, v * (1.0 - 0.12 * r as f32), s * 0.3, out);
                        }
                        push_hist(&mut self.masters.bt_low, pitch - tonic);
                    } else {
                        let frog = tonic - 24 + (self.masters.unit() * 6.0) as i32;
                        self.note(ctx, at, frog, vel * 0.35, s * 0.4, out);
                        self.mlater(ctx, at + (s * 0.5) as u64, frog, vel * 0.3, s * 0.4, out);
                        push_hist(&mut self.masters.bt_low, frog - tonic);
                    }
                }
                // A far tune, in five notes, every eight bars.
                let pb = bar.rem_euclid(8);
                if (pb == 4 || pb == 5) && t % 4 == 0 {
                    const PENTA: [i32; 5] = [0, 1, 2, 4, 5];
                    let m = &mut self.masters;
                    let u = m.unit();
                    let last = pb == 5 && t == 12;
                    m.bt_tune = if last { 0 } else { (m.bt_tune + if u < 0.5 { 1 } else { -1 }).clamp(0, 6) };
                    let i = m.bt_tune as usize;
                    let d = PENTA[i % 5] + 7 * (i / 5) as i32;
                    push_hist(&mut m.bt_hist, d);
                    let pitch = Self::mdeg(ctx, base, d, 0);
                    self.note(ctx, at, pitch, vel * 0.45, if last { 16.0 } else { 4.0 } * s * 0.95, out);
                }
                self.visual[0] = (style as u64) | (t as u64) << 4 | (bar.rem_euclid(8) as u64) << 12 | ((self.masters.bt_cluster + 8) as u64) << 16;
                self.visual[1] = self.masters.bt_hist;
                self.visual[2] = self.masters.bt_low;
            }
            // A mirror: the lower voice the upper one turned upside down.
            _ => {
                let bar = k.div_euclid(16);
                let t = k.rem_euclid(16);
                if t % 2 == 0 {
                    let j = (bar.rem_euclid(2) * 8 + t / 2) as usize;
                    let phrase = bar.div_euclid(4);
                    let line = mirror_notes(var.wrapping_mul(13).wrapping_add((phrase / 2) as u32));
                    // The second two bars: the line backwards.
                    let backwards = bar.rem_euclid(4) >= 2;
                    let axis = Self::mdeg(ctx, base, 0, 0);
                    for &(st, len, sv) in &line {
                        let st = if backwards { 16 - st - len } else { st };
                        if st == j {
                            let lenf = len as f64 * q * gate;
                            self.note(ctx, at, axis + sv, vel, lenf, out);
                            self.note(ctx, at, axis - 12 - sv, vel * 0.9, lenf, out);
                            push_hist(&mut self.masters.bt_hist, sv);
                        }
                    }
                }
                self.visual[0] = (style as u64) | (t as u64) << 4 | (bar.rem_euclid(4) as u64) << 12;
                self.visual[1] = self.masters.bt_hist;
            }
        }
    }

    // ------------------------------------------------------------ messiaen

    /// A bird's phrase: (when, in demisemiquavers; semitones over the
    /// tonic; loudness 0-255).
    fn bird_phrase(&mut self, bird: usize) -> usize {
        let m = &mut self.masters;
        let mut n = 0;
        let mut push = |m: &mut [(u16, i8, u8); MAX_BIRD], t: u16, p: i32, v: f32| {
            if n < MAX_BIRD {
                m[n] = (t, p.clamp(-30, 60) as i8, (v.clamp(0.0, 1.0) * 255.0) as u8);
                n += 1;
            }
        };
        let mut song = [(0u16, 0i8, 0u8); MAX_BIRD];
        match bird {
            // Blackbird: fluting leaps, then chatter.
            0 => {
                let notes = 4 + (m.unit() * 7.0) as usize;
                let mut pitch = 12 + (m.unit() * 18.0) as i32;
                let mut t = 0u16;
                for _ in 0..notes {
                    push(&mut song, t, pitch, 0.5 + 0.4 * m.unit());
                    let leap = 2 + (m.unit() * 7.0) as i32;
                    pitch += if m.unit() < 0.5 { leap } else { -leap };
                    pitch = pitch.clamp(10, 36);
                    t += [1, 2, 2, 3][(m.unit() * 4.0) as usize % 4];
                }
                let high = pitch + 5 + (m.unit() * 7.0) as i32;
                for _ in 0..2 + (m.unit() * 3.0) as usize {
                    push(&mut song, t, high, 0.6);
                    t += 1;
                }
            }
            // Thrush: each motif two or three times.
            1 => {
                let len = 3 + (m.unit() * 3.0) as usize;
                let mut at = 0u16;
                let mut motif = [(0u16, 0i32); 5];
                for slot in motif.iter_mut().take(len) {
                    *slot = (at, 14 + (m.unit() * 20.0) as i32);
                    at += 1 + (m.unit() * 2.0) as u16;
                }
                let motif = &motif[..len];
                let span = motif.last().map(|x| x.0).unwrap_or(0) + 3;
                for r in 0..2 + (m.unit() * 2.0) as u16 {
                    for &(t, p) in motif {
                        push(&mut song, r * span + t, p, 0.75);
                    }
                }
            }
            // Nightingale: one note over and over, faster and louder, then a flourish.
            _ => {
                let pitch = 18 + (m.unit() * 10.0) as i32;
                let reps = 6 + (m.unit() * 9.0) as usize;
                let mut t = 0u16;
                for r in 0..reps {
                    let x = r as f32 / reps as f32;
                    push(&mut song, t, pitch, 0.3 + 0.6 * x);
                    t += (5.0 - 4.0 * x).round().max(1.0) as u16;
                }
                let mut f = pitch + 7;
                for _ in 0..3 + (m.unit() * 3.0) as usize {
                    push(&mut song, t, f, 0.8);
                    f -= 2 + (m.unit() * 3.0) as i32;
                    t += 1;
                }
            }
        }
        m.me_bird = song;
        m.me_n = n;
        n
    }

    /// Messiaen, step `k` (a demisemiquaver).
    pub(crate) fn messiaen(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let style = choose(p, ME_STYLE, MESSIAEN_STYLES.len());
        let mode_i = choose(p, ME_MODE, MESSIAEN_MODES.len());
        let mode = MODES[mode_i];
        let bird = choose(p, ME_BIRD, MESSIAEN_BIRDS.len());
        let density = p[ME_DENSITY].clamp(0.0, 1.0);
        let base = p[ME_BASE];
        let vel = p[ME_VELOCITY];
        let s = step_frames;
        let tonic = Self::mdeg(ctx, base, 0, 0);
        let nm = mode.len() as i32;
        if self.masters.jumped(k) {
            let m = &mut self.masters;
            m.me_next = k;
            m.me_rest = k;
            m.me_n = 0;
            m.me_start = i64::MIN;
            m.me_vnext = [k; 3];
            m.me_vpos = [0; 3];
            m.me_vrep = [0; 3];
            let a = 1 + (m.unit() * 4.0) as u8;
            let b = 1 + (m.unit() * 4.0) as u8;
            let c = 1 + (m.unit() * 6.0) as u8;
            m.me_cell = [a, b, c, b, a];
            m.me_vcell = [m.me_cell; 3];
        }
        match style {
            // Chords of the mode, the top a slow melody, lengths with an added value.
            0 => {
                if k >= self.masters.me_next {
                    let m = &mut self.masters;
                    let d = ADDED[(m.unit() * ADDED.len() as f32) as usize % ADDED.len()] as f32;
                    let dur = (d * 2.0 * (1.0 + 2.0 * (1.0 - density))).round().max(2.0) as i64;
                    let u = m.unit();
                    m.me_top = (m.me_top + if u < 0.2 { 0 } else if u < 0.5 { 1 } else if u < 0.8 { -1 } else if u < 0.9 { 2 } else { -2 }).clamp(nm, 3 * nm);
                    m.me_next = k + dur;
                    let top = m.me_top;
                    push_hist(&mut m.me_hist, top);
                    let len = dur as f64 * s * 0.98;
                    let mut mask = 0u64;
                    for (j, off) in [0, -2, -3, -5, -7].iter().enumerate() {
                        let pitch = mode_pitch(tonic - 12, mode, top + off);
                        mask |= 1u64 << pitch.rem_euclid(12);
                        self.note(ctx, at, pitch, vel * if j == 0 { 0.7 } else { 0.42 }, len, out);
                    }
                    let bass = mode_pitch(tonic - 12, mode, top - 2 * nm);
                    self.note(ctx, at, bass, vel * 0.55, len, out);
                    self.masters.me_chord = mask;
                }
                self.visual[1] = self.masters.me_hist;
                self.visual[2] = self.masters.me_chord;
            }
            // Birdsong: phrases, and silences between them.
            1 => {
                if self.masters.me_start == i64::MIN && k >= self.masters.me_rest {
                    self.bird_phrase(bird);
                    self.masters.me_start = k;
                }
                let m = &self.masters;
                if m.me_start != i64::MIN {
                    let off = (k - m.me_start) as u16;
                    let n = m.me_n;
                    let song = m.me_bird;
                    for (i, &(t, pitch, v)) in song[..n].iter().enumerate() {
                        if t == off {
                            let last = i + 1 == n;
                            self.note(ctx, at, tonic + 12 + pitch as i32, vel * v as f32 / 255.0, s * if last { 2.0 } else { 0.8 }, out);
                        }
                    }
                    let end = song[..n].iter().map(|x| x.0).max().unwrap_or(0);
                    if off >= end {
                        let m = &mut self.masters;
                        m.me_start = i64::MIN;
                        let rest = 4.0 + 60.0 * (1.0 - density) * m.unit();
                        m.me_rest = k + rest as i64;
                    }
                }
                let m = &self.masters;
                let mut a = 0u64;
                let mut b = 0u64;
                for (i, x) in m.me_bird[..m.me_n.min(16)].iter().enumerate() {
                    let v = ((x.1 as i32 + 64) as u64) & 0xff;
                    if i < 8 {
                        a |= v << (8 * i);
                    } else {
                        b |= v << (8 * (i - 8));
                    }
                }
                let played = if m.me_start == i64::MIN { 0 } else { m.me_bird[..m.me_n].iter().filter(|x| x.0 as i64 <= k - m.me_start).count() };
                self.visual[1] = a;
                self.visual[2] = b;
                self.visual[3] = (m.me_n as u64) | (played as u64) << 8 | ((m.me_start != i64::MIN) as u64) << 16;
            }
            // Rhythmic characters: one grows, one shrinks, one stays.
            _ => {
                const PITCHES: [i32; 5] = [0, 2, 4, 2, 0];
                for v in 0..3 {
                    if k < self.masters.me_vnext[v] {
                        continue;
                    }
                    let m = &mut self.masters;
                    let j = m.me_vpos[v] as usize;
                    let dur = m.me_vcell[v][j] as i64 * 2;
                    m.me_vnext[v] = k + dur;
                    m.me_vpos[v] += 1;
                    if m.me_vpos[v] as usize >= 5 {
                        m.me_vpos[v] = 0;
                        m.me_vrep[v] = (m.me_vrep[v] + 1) % 6;
                        let g = m.me_vrep[v] as u8;
                        let cell = m.me_cell;
                        m.me_vcell[v] = match v {
                            0 => cell.map(|c| c + g),
                            1 => cell.map(|c| c.saturating_sub(g).max(1)),
                            _ => cell,
                        };
                    }
                    let len = dur as f64 * s * 0.9;
                    let idx = PITCHES[j];
                    match v {
                        0 => {
                            for off in [0, -2, -3] {
                                self.note(ctx, at, mode_pitch(tonic, mode, nm + idx + off), vel * 0.5, len, out);
                            }
                        }
                        1 => self.note(ctx, at, mode_pitch(tonic, mode, idx + 1), vel * 0.65, len, out),
                        _ => self.note(ctx, at, mode_pitch(tonic, mode, idx - nm), vel * 0.6, len, out),
                    }
                }
                let m = &self.masters;
                for v in 0..3 {
                    self.visual[1 + v] = m.me_vcell[v].iter().enumerate().fold(0u64, |a, (i, &c)| a | (c as u64) << (8 * i)) | (m.me_vpos[v] as u64) << 48;
                }
            }
        }
        self.visual[0] = (style as u64) | (mode_i as u64) << 4 | (bird as u64) << 8;
    }

    // ------------------------------------------------------------ ligeti

    /// Ligeti, step `k` (as the rate goes).
    pub(crate) fn ligeti(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let style = choose(p, LI_STYLE, LIGETI_STYLES.len());
        let voices = (p[LI_VOICES].round() as usize).clamp(2, MAX_METRONOMES);
        let spread = p[LI_SPREAD].clamp(0.0, 1.0);
        let change = p[LI_CHANGE].clamp(0.0, 1.0);
        let base = p[LI_BASE];
        let gate = p[LI_GATE].clamp(0.1, 1.0) as f64;
        let vel = p[LI_VELOCITY];
        let s = step_frames;
        let jumped = self.masters.jumped(k);
        match style {
            // The metronomes, all set going at once, each running down.
            0 => {
                let sb = s / ctx.samples_per_beat;
                let b0 = k as f64 * sb;
                let m = &mut self.masters;
                if jumped || b0 < m.li_start {
                    m.li_round = 0;
                    m.li_start = 0.0;
                }
                loop {
                    let len = metronome_round(m.li_round, voices, spread, change);
                    if b0 < m.li_start + len {
                        break;
                    }
                    m.li_start += len;
                    m.li_round += 1;
                }
                let c = m.li_round;
                let local = b0 - m.li_start;
                let mut alive = 0u64;
                let mut phases = 0u64;
                for i in 0..voices {
                    let (period, winds) = metronome(c, i, spread, change);
                    let mut n = (local / period).ceil() as u32;
                    while (n as f64) * period < local + sb && n < winds {
                        let when = at + (((n as f64) * period - local) / sb * s).max(0.0) as u64;
                        let pitch = Self::mdeg(ctx, base, METRONOME_DEGREES[i], 0);
                        let v = vel * (0.5 + 0.35 * hash(c + 1, i as u32));
                        self.mlater(ctx, when, pitch, v, (period * ctx.samples_per_beat * gate * 0.5).min(0.2 * ctx.sr as f64), out);
                        n += 1;
                    }
                    if local < winds as f64 * period {
                        alive |= 1 << i;
                    }
                    let ph = ((local / period).rem_euclid(2.0) / 2.0 * 16.0) as u64 & 0xf;
                    phases |= ph << (4 * i);
                }
                self.visual[0] = (style as u64) | alive << 4 | (voices as u64) << 20 | (c as u64 & 0xff) << 28;
                self.visual[1] = phases;
            }
            // Continuum: both hands round their few notes, one note changing at a time.
            1 => {
                let size = (voices / 2).clamp(2, MAX_HAND);
                let tonic = Self::mdeg(ctx, base, 0, 0);
                let m = &mut self.masters;
                if !m.li_ready || jumped {
                    m.li_ready = true;
                    let third = interval(ctx, base, 2) as i8;
                    m.li_rh[..2].copy_from_slice(&[0, third]);
                    m.li_rn = 2;
                    m.li_lh[..2].copy_from_slice(&[third, 0]);
                    m.li_ln = 2;
                    m.li_next = k + 16;
                    m.li_count = 0;
                }
                if k >= m.li_next {
                    m.li_next = k + 8 + ((1.0 - change) * 120.0) as i64;
                    m.li_left = !m.li_left;
                    let u = m.bits();
                    let (notes, n) = if m.li_left { (&mut m.li_lh, &mut m.li_ln) } else { (&mut m.li_rh, &mut m.li_rn) };
                    let pick = (u >> 4) as usize % *n;
                    let step = 1 + ((u >> 9) % 3) as i8;
                    let sign = if u & 1 == 0 { 1 } else { -1 };
                    match (u >> 12) % 10 {
                        0..=3 if *n < size => {
                            notes[*n] = (notes[pick] + sign * (step + 1)).clamp(-7, 19);
                            *n += 1;
                        }
                        4..=5 if *n > 2 => {
                            for i in pick..*n - 1 {
                                notes[i] = notes[i + 1];
                            }
                            *n -= 1;
                        }
                        _ => notes[pick] = (notes[pick] + sign * step.min(2)).clamp(-7, 19),
                    }
                }
                let low = if spread > 0.5 { -12 } else { 0 };
                let (r, l) = (m.li_rh[(m.li_count as usize) % m.li_rn], m.li_lh[(m.li_count as usize) % m.li_ln]);
                m.li_count = m.li_count.wrapping_add(1);
                let v = vel * (0.85 + 0.15 * m.unit());
                self.note(ctx, at, tonic + r as i32, v, s * gate, out);
                self.note(ctx, at, tonic + low + l as i32, v * 0.92, s * gate, out);
                let m = &self.masters;
                let pack = |h: &[i8; MAX_HAND], n: usize| h[..n].iter().enumerate().fold(0u64, |a, (i, &x)| a | (((x as i32 + 64) as u64) & 0xff) << (8 * i));
                self.visual[0] = (style as u64) | (m.li_rn as u64) << 4 | (m.li_ln as u64) << 8 | ((m.li_count as u64) & 0xffff) << 16;
                self.visual[1] = pack(&m.li_rh, m.li_rn);
                self.visual[2] = pack(&m.li_lh, m.li_ln);
            }
            // Musica ricercata: one pitch class more each stage.
            _ => {
                let order = ricercata_order(ctx, base);
                let per = (4.0 + (1.0 - change) * 8.0).round() as i64;
                let bar = k.div_euclid(16);
                let t = k.rem_euclid(16);
                let stage = bar.div_euclid(per).rem_euclid(12) as usize;
                let round = bar.div_euclid(per * 12) as u32;
                let count = stage + 1;
                let tonic = Self::mdeg(ctx, base, 0, 0);
                let seed = (stage as u32).wrapping_mul(53).wrapping_add(round.wrapping_mul(7));
                let in_stage = bar.rem_euclid(per);
                if in_stage == 0 && t == 0 {
                    // The newcomer, announced.
                    let pc = order[stage];
                    for o in [0, -12] {
                        self.note(ctx, at, tonic + pc + o, vel, 8.0 * s, out);
                    }
                    push_hist(&mut self.masters.li_hist, pc);
                } else {
                    // The same obsessive rhythm all through a stage.
                    let hit = hash(seed, t as u32) < 0.42 + 0.1 * spread;
                    if hit || t == 0 {
                        let accent = t % 4 == 0 || hash(seed + 3, t as u32) < 0.2;
                        let pc = order[(hash(seed + 1, (in_stage * 16 + t) as u32) * count as f32) as usize % count];
                        let oct = [-12, 0, 0, 12, -24][(hash(seed + 2, (in_stage * 16 + t) as u32) * 5.0) as usize % 5];
                        self.note(ctx, at, tonic + pc + oct, vel * if accent { 1.0 } else { 0.6 }, s * gate * 1.5, out);
                        push_hist(&mut self.masters.li_hist, pc);
                    }
                }
                let packed = order.iter().enumerate().fold(0u64, |a, (i, &pc)| a | (pc as u64 & 0xf) << (4 * i));
                self.visual[0] = (style as u64) | (count as u64) << 4 | (in_stage as u64 & 0xff) << 12 | (per as u64) << 20;
                self.visual[1] = packed;
                self.visual[2] = self.masters.li_hist;
            }
        }
    }

    // ------------------------------------------------------------ cage

    /// Cage, step `k` (as the rate goes).
    pub(crate) fn cage(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let style = choose(p, CA_STYLE, CAGE_STYLES.len());
        let change = p[CA_CHANGE].clamp(0.0, 1.0);
        let silence = p[CA_SILENCE].clamp(0.0, 1.0);
        let coins = p[CA_COINS].round().max(0.0) as u32;
        let base = p[CA_BASE].round() as i32;
        let vel = p[CA_VELOCITY];
        let s = step_frames;
        let jumped = self.masters.jumped(k);
        let key = (coins, (silence * 100.0) as u32);
        if self.masters.ca_key != Some(key) {
            // A fresh chart and gamut for these coins.
            let m = &mut self.masters;
            m.ca_key = Some(key);
            m.rng = 0x51CE_0001 ^ coins.wrapping_mul(0x9E37_79B9);
            for i in 0..64 {
                m.ca_chart[i] = m.sound(silence);
            }
            for i in 0..GAMUT {
                let u = m.unit();
                let low = (m.unit() * 36.0) as i8 - 12;
                let (notes, n) = if u < 0.4 {
                    ([low, 0, 0, 0], 1)
                } else if u < 0.7 {
                    let iv = [1, 11, 13, 6, 10][(m.unit() * 5.0) as usize % 5];
                    ([low, low + iv, 0, 0], 2)
                } else {
                    let shape: [i8; 3] = [[0, 1, 6], [0, 6, 7], [0, 11, 13]][(m.unit() * 3.0) as usize % 3];
                    ([low + shape[0], low + shape[1], low + shape[2], 0], 3)
                };
                m.ca_gamut[i] = Sound { notes, n, len: 1, dynamic: 3 + (m.unit() * 3.0) as u8 };
            }
            m.ca_next = k;
        }
        if jumped {
            self.masters.ca_next = k;
        }
        match style {
            // Chance: the coins pick each sound from the chart.
            0 => {
                if k >= self.masters.ca_next {
                    let m = &mut self.masters;
                    let mut lines = [0u8; 6];
                    for l in lines.iter_mut() {
                        *l = (0..3).map(|_| if m.unit() < 0.5 { 3 } else { 2 }).sum();
                    }
                    let hex = hexagram(&lines);
                    let moving = lines.iter().any(|&l| l == 6 || l == 9);
                    let snd = m.ca_chart[hex];
                    m.ca_lines = lines;
                    m.ca_silent = snd.n == 0;
                    m.ca_next = k + snd.len.max(1) as i64;
                    if moving && m.unit() < change {
                        m.ca_chart[hex] = m.sound(silence);
                    }
                    for &o in &snd.notes[..snd.n as usize] {
                        let v = vel * snd.dynamic as f32 / 8.0;
                        self.note(ctx, at, base + o as i32, v, snd.len as f64 * s, out);
                    }
                }
                let m = &self.masters;
                let lines = m.ca_lines.iter().enumerate().fold(0u64, |a, (i, &l)| a | ((l as u64 - 6) & 3) << (2 * i));
                self.visual[0] = (style as u64) | lines << 4 | (hexagram(&m.ca_lines) as u64) << 16 | (m.ca_silent as u64) << 24;
            }
            // A prepared piano: quiet patterns from a small gamut, in a square-root form.
            1 => {
                let props = PROPORTIONS[coins as usize % PROPORTIONS.len()];
                let total: i64 = props.iter().map(|&x| x as i64).sum();
                let bar = k.div_euclid(16);
                let t = k.rem_euclid(16) as u32;
                let inside = bar.rem_euclid(total * total);
                let unit = inside / total;
                let bar_u = inside % total;
                let find = |x: i64| {
                    let mut acc = 0i64;
                    for (i, &pp) in props.iter().enumerate() {
                        acc += pp as i64;
                        if x < acc {
                            return i;
                        }
                    }
                    props.len() - 1
                };
                let section = find(unit);
                let phrase = find(bar_u);
                let seed = (section as u32).wrapping_mul(31).wrapping_add(phrase as u32 * 7).wrapping_add(coins * 101);
                let len = if phrase % 2 == 0 { 16 } else { 12 };
                let tt = t % len;
                let density = 0.15 + 0.5 * (1.0 - silence);
                if hash(seed, tt) < density {
                    let pick = (hash(seed + 1, tt) * 3.0) as usize % 3;
                    let which = (hash(section as u32 + coins * 3, pick as u32) * GAMUT as f32) as usize % GAMUT;
                    let snd = self.masters.ca_gamut[which];
                    let v = vel * snd.dynamic as f32 / 8.0 * if tt == 0 { 1.1 } else { 0.9 };
                    for &o in &snd.notes[..snd.n as usize] {
                        self.note(ctx, at, base + o as i32, v, s * 1.5, out);
                    }
                }
                let packed = props.iter().enumerate().fold(0u64, |a, (i, &x)| a | (x as u64) << (4 * i));
                self.visual[0] = (style as u64) | (section as u64) << 4 | (phrase as u64) << 8 | (props.len() as u64) << 12 | packed << 16 | (unit as u64 & 0xff) << 40 | (bar_u as u64 & 0xff) << 48;
            }
            // 4'33": nothing at all, for four minutes and thirty-three seconds.
            _ => {
                let secs = k as f64 * s / ctx.sr as f64;
                let total: f64 = MOVEMENTS.iter().sum();
                let local = secs.rem_euclid(total);
                self.visual[0] = (style as u64) | ((local * 10.0) as u64) << 4;
            }
        }
    }

    // ------------------------------------------------------------ einaudi

    /// Einaudi, step `k` (half a quaver).
    pub(crate) fn einaudi(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let style = choose(p, EI_STYLE, EINAUDI_STYLES.len());
        let roots = EINAUDI_ROOTS[choose(p, EI_CHORDS, EINAUDI_CHORDS.len())];
        let melody = p[EI_MELODY].clamp(0.0, 1.0);
        let base = p[EI_BASE];
        let pedal = p[EI_PEDAL].clamp(0.0, 1.0) as f64;
        let vel = p[EI_VELOCITY];
        let s = step_frames;
        let _ = self.masters.jumped(k);
        let bar = k.div_euclid(16);
        let t = k.rem_euclid(16);
        let chords = p.get(EI_CHORD_AMOUNT).copied().unwrap_or(1.0).clamp(0.0, 2.0);
        let ci = bar.rem_euclid(4) as usize;
        let root = chord_root(chords, ci as i64, roots[ci], roots[(ci + 1) % 4], t >= 8);
        let lr = low(root);
        let in_chord = |x: i32| [0, 2, 4].contains(&(x - root).rem_euclid(7));
        let nearest = |x: i32| if in_chord(x) { x } else if in_chord(x - 1) { x - 1 } else { x + 1 };
        let stage = if style == 1 { bar.rem_euclid(8 * BUILD_STAGES) / 8 } else { 0 };
        match style {
            0 => {
                // Phrases of eight bars, swelling and falling back.
                let phrase_x = (bar.rem_euclid(8) as f64 + t as f64 / 16.0) / 8.0;
                let arch = (0.75 + 0.25 * (PI * phrase_x).sin()) as f32;
                if t % 2 == 0 {
                    let j = (t / 2) as usize;
                    let pitch = Self::mdeg(ctx, base, lr + EINAUDI_LH[j], 0);
                    let (v, len) = if j == 0 { (0.55, (2.0 + 26.0 * pedal) * s) } else { (0.42, (2.0 + 8.0 * pedal) * s) };
                    self.note(ctx, at, pitch, vel * v * arch, len, out);
                }
                // The melody, after four bars of the left hand alone.
                if bar >= 4 && melody > 0.0 && t % 2 == 0 {
                    let mb = bar - 4;
                    let phrase = mb.div_euclid(8);
                    let motif = einaudi_motif((phrase / 2) as u32);
                    let q = ((mb.rem_euclid(2)) * 8 + t / 2) as u8;
                    let up = if melody > 0.7 && mb.rem_euclid(8) >= 6 { 7 } else { 0 };
                    for &(st, len, d) in &motif {
                        if st == q && hash(phrase as u32 + 3, q as u32 + 16 * (mb.rem_euclid(8) as u32)) < 0.4 + 0.6 * melody {
                            let strong = st % 4 == 0;
                            let deg = if strong { nearest(d) } else { d } + up;
                            let pitch = Self::mdeg(ctx, base, deg, 0);
                            self.note(ctx, at, pitch, vel * arch, len as f64 * 2.0 * s * 0.97, out);
                            push_hist(&mut self.masters.ei_hist, deg);
                        }
                    }
                }
            }
            _ => {
                let fade = if stage == 5 { 1.0 - 0.7 * (bar.rem_euclid(8) as f32 + t as f32 / 16.0) / 8.0 } else { 1.0 };
                let loud = [0.5, 0.6, 0.7, 0.85, 1.0, 0.7][stage as usize] * fade;
                // The ostinato: quavers, then semiquavers at the height of it.
                const FIG: [i32; 8] = [0, 2, 4, 7, 4, 2, 4, 2];
                let fast = stage == 3 || stage == 4;
                if fast || t % 2 == 0 {
                    let j = if fast { t as usize % 8 } else { (t / 2) as usize };
                    let pitch = Self::mdeg(ctx, base, lr - 7 + FIG[j], 0);
                    self.note(ctx, at, pitch, vel * 0.45 * loud, if fast { s } else { 2.0 * s } * (0.9 + 0.6 * pedal), out);
                }
                // Octaves in the bass.
                if (1..5).contains(&stage) && (t == 0 || t == 8 || (stage == 4 && t % 4 == 0)) {
                    for d in [-14, -7] {
                        self.note(ctx, at, Self::mdeg(ctx, base, lr + d, 0), vel * 0.55 * loud, if stage == 4 { 4.0 } else { 8.0 } * s * 0.95, out);
                    }
                }
                // A long line on top, from the third layer.
                if (2..5).contains(&stage) && t == 0 && melody > 0.0 {
                    let m = &mut self.masters;
                    let u = m.unit();
                    m.ei_top = nearest((m.ei_top + if u < 0.5 { -1 } else if u < 0.8 { 1 } else { 0 }).clamp(2, 9));
                    let top = m.ei_top;
                    push_hist(&mut m.ei_hist, top);
                    let pitch = Self::mdeg(ctx, base, top, 0);
                    self.note(ctx, at, pitch, vel * 0.8 * loud, 16.0 * s * 0.98, out);
                    if stage == 4 {
                        self.note(ctx, at, pitch + 12, vel * 0.6 * loud, 16.0 * s * 0.98, out);
                    }
                }
            }
        }
        self.visual[0] = (style as u64) | (bar.rem_euclid(4) as u64) << 4 | (t as u64) << 8 | (stage as u64) << 16 | ((bar.rem_euclid(8 * BUILD_STAGES)) as u64 & 0xff) << 20;
        self.visual[1] = self.masters.ei_hist;
        self.visual[2] = roots.iter().enumerate().fold(0u64, |a, (i, &r)| a | ((r.rem_euclid(7) as u64) << (4 * i)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::{MidiFxKind, MidiFxState};
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};

    /// Runs `beats` beats at 120 BPM in `scale`: (on beat, pitch, off beat, velocity).
    fn run(kind: MidiFxKind, p: &[f32], beats: f64, scale: Scale) -> Vec<(f64, u8, f64, f32)> {
        let spb = 24000.0;
        let mut st = MidiFxState::new(kind);
        let mut open: Vec<(f64, u8, f32)> = Vec::new();
        let mut done = Vec::new();
        for b in 0..(beats * spb / 256.0) as u64 {
            let now = b * 256;
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale, scale_on: true, beat: now as f64 / spb, playing: true, sr: 48000.0, drum_notes: 0 };
            let mut out = Vec::new();
            st.process(kind, p, &[], &[], &mut out, &ctx);
            for e in &out {
                let t = (now + e.offset as u64) as f64 / spb;
                if e.on {
                    open.push((t, e.pitch, e.vel));
                } else if let Some(i) = open.iter().position(|o| o.1 == e.pitch) {
                    let (on, pitch, vel) = open.remove(i);
                    done.push((on, pitch, t, vel));
                }
            }
        }
        done.extend(open.into_iter().map(|(on, pitch, vel)| (on, pitch, f64::INFINITY, vel)));
        done.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
        done
    }

    fn c_major() -> Scale {
        Scale::new(0, ScaleKind::Major)
    }

    fn notes(kind: MidiFxKind, p: &[f32], beats: f64) -> Vec<(f64, u8, f64, f32)> {
        run(kind, p, beats, c_major())
    }

    fn in_key(n: &[(f64, u8, f64, f32)]) -> bool {
        n.iter().all(|x| [0, 2, 4, 5, 7, 9, 11].contains(&(x.1 % 12)))
    }

    /// The pitches starting within [lo, hi) beats.
    fn between(n: &[(f64, u8, f64, f32)], lo: f64, hi: f64) -> Vec<u8> {
        n.iter().filter(|x| x.0 >= lo - 1e-3 && x.0 < hi - 1e-3).map(|x| x.1).collect()
    }

    #[test]
    fn the_dice_choose_each_bar_and_a_round_repeats_its_halves() {
        let p = defaults(MOZART);
        // A bar of three at 1/16: three beats. A A B B = 32 bars = 96 beats.
        let n = notes(MidiFxKind::Mozart, &p, 96.0);
        assert!(in_key(&n));
        // The repeat of A is A again, note for note.
        let a1 = between(&n, 0.0, 24.0);
        let a2 = between(&n, 24.0, 48.0);
        assert_eq!(a1, a2);
        assert_ne!(a1, between(&n, 48.0, 72.0));
        // Every bar of the melody is one of the eleven, thrown by the dice.
        for bar in 0..8 {
            let (a, b) = dice(0, 0, bar);
            assert!((2..=12).contains(&(a + b)));
            let fig = MINUET_BARS[(a + b - 2) as usize];
            let top: Vec<f64> = n.iter().filter(|x| x.0 >= bar as f64 * 3.0 - 1e-3 && x.0 < bar as f64 * 3.0 + 3.0 - 1e-3 && x.1 >= 69).map(|x| x.0).collect();
            assert_eq!(top.len(), fig.len(), "bar {bar}: {top:?}");
        }
        // The last bar ends at home.
        let last: Vec<u8> = between(&n, 93.0, 96.0);
        assert!(last.iter().all(|v| [0, 4, 7].contains(&(v % 12))), "{last:?}");
        // Another round: other dice.
        let next = notes(MidiFxKind::Mozart, &p, 192.0);
        assert_ne!(between(&next, 96.0, 120.0), a1);
    }

    #[test]
    fn a_sonata_has_an_alberti_bass_and_trills_before_the_cadence() {
        let mut p = defaults(MOZART);
        p[MO_STYLE] = 1.0;
        let n = notes(MidiFxKind::Mozart, &p, 32.0);
        assert!(in_key(&n));
        // Bar 1 (the tonic): the left hand low, high, middle, high: C G E G.
        let lh: Vec<u8> = n.iter().filter(|x| x.0 < 3.99 && x.1 < 72).map(|x| x.1 % 12).take(8).collect();
        assert_eq!(lh, vec![0, 7, 4, 7, 0, 7, 4, 7]);
        // Bar 7: a trill, two notes a step apart, back and forth.
        let trill: Vec<u8> = n.iter().filter(|x| x.0 >= 24.0 && x.0 < 27.0 && x.1 >= 72).map(|x| x.1).collect();
        assert!(trill.len() >= 12, "{trill:?}");
        assert!(trill.windows(3).all(|w| w[0] == w[2] && w[0] != w[1] && (w[0] as i32 - w[1] as i32).abs() <= 2), "{trill:?}");
        // Bar 8 on the tonic.
        assert!(between(&n, 28.0, 29.0).iter().all(|v| [0, 4, 7].contains(&(v % 12))));
    }

    #[test]
    fn alla_turca_turns_round_a_semitone_below() {
        let mut p = defaults(MOZART);
        p[MO_STYLE] = 2.0;
        let n = run(MidiFxKind::Mozart, &p, 4.0, Scale::new(9, ScaleKind::Minor));
        // In A minor: B A G# A, C; D C B C, E.
        let rh: Vec<u8> = n.iter().filter(|x| x.1 >= 60).map(|x| x.1).take(10).collect();
        assert_eq!(rh, vec![71, 69, 68, 69, 72, 74, 72, 71, 72, 76]);
    }

    #[test]
    fn a_nocturne_keeps_time_in_the_left_hand_and_takes_it_in_the_right() {
        let mut p = defaults(CHOPIN);
        p[CH_RUBATO] = 1.0;
        let n = notes(MidiFxKind::Chopin, &p, 32.0);
        assert!(in_key(&n));
        // The left hand on every triplet quaver, exactly.
        let lh: Vec<f64> = n.iter().filter(|x| x.1 < 70).map(|x| x.0).collect();
        assert!(lh.len() >= 90, "{}", lh.len());
        assert!(lh.iter().all(|t| ((t * 3.0) - (t * 3.0).round()).abs() < 1e-3), "{lh:?}");
        // The right hand comes late (off the grid) at times.
        let rh: Vec<f64> = n.iter().filter(|x| x.1 >= 72).map(|x| x.0).collect();
        assert!(rh.iter().any(|t| ((t * 12.0) - (t * 12.0).round()).abs() > 0.05), "{rh:?}");
        // Wide: the left hand spans more than an octave and a half in a bar.
        let bar: Vec<u8> = n.iter().filter(|x| x.0 < 3.99 && x.1 < 70).map(|x| x.1).collect();
        assert!(bar.iter().max().unwrap() - bar.iter().min().unwrap() >= 19, "{bar:?}");
    }

    #[test]
    fn a_waltz_has_its_bass_on_one_and_chords_on_two_and_three() {
        let mut p = defaults(CHOPIN);
        p[CH_STYLE] = 1.0;
        p[CH_RUBATO] = 0.0;
        let n = notes(MidiFxKind::Chopin, &p, 12.0);
        for bar in 0..4 {
            let b = bar as f64 * 3.0;
            let low: Vec<u8> = n.iter().filter(|x| (x.0 - b).abs() < 1e-3 && x.1 < 60).map(|x| x.1).collect();
            assert_eq!(low.len(), 1, "bar {bar}");
            for beat in [1.0, 2.0] {
                let chord = n.iter().filter(|x| (x.0 - b - beat).abs() < 1e-3 && x.2 - x.0 < 0.45 && x.1 < 73).count();
                assert_eq!(chord, 3, "bar {bar} beat {beat}");
            }
        }
    }

    #[test]
    fn a_prelude_sinks_by_semitones() {
        let mut p = defaults(CHOPIN);
        p[CH_STYLE] = 2.0;
        p[CH_RUBATO] = 0.0;
        let n = run(MidiFxKind::Chopin, &p, 24.0, Scale::new(4, ScaleKind::Minor));
        // The chords: on every quaver, three notes; from one to the next at most one moves, by a semitone (or two, round another).
        let chords: Vec<Vec<u8>> = (0..48)
            .map(|q| {
                // (the chords: short, on every quaver)
                let mut c: Vec<u8> = n.iter().filter(|x| (x.0 - q as f64 * 0.5).abs() < 1e-3 && x.2 - x.0 > 0.4 && x.2 - x.0 < 0.5).map(|x| x.1).collect();
                c.sort();
                c
            })
            .collect();
        let mut moves = 0;
        for w in chords.windows(2) {
            assert_eq!(w[0].len(), 3, "{w:?}");
            let gone: Vec<u8> = w[0].iter().filter(|x| !w[1].contains(x)).copied().collect();
            let came: Vec<u8> = w[1].iter().filter(|x| !w[0].contains(x)).copied().collect();
            if gone.len() == 1 && came.len() == 1 && came[0] < gone[0] && gone[0] - came[0] <= 2 {
                moves += 1;
            }
        }
        assert!(moves >= 6, "{moves}");
        // Off the key: the sinking takes them outside it.
        let e_minor = [4, 6, 7, 9, 11, 0, 2];
        assert!(chords.iter().flatten().any(|v| !e_minor.contains(&(v % 12))));
    }

    #[test]
    fn bolero_grows_from_soft_to_loud_with_its_rhythm_throughout() {
        let mut p = defaults(RAVEL);
        p[RA_LENGTH] = 16.0;
        // Two bars of drum, fourteen of tune, the ending's two, two quiet: twenty bars of three.
        let n = notes(MidiFxKind::Ravel, &p, 60.0);
        // The drum's two bars: 10 then 14 strokes on the tonic below.
        let drum = |lo: f64| n.iter().filter(|x| x.1 == 60 && x.0 >= lo - 1e-3 && x.0 < lo + 6.0 - 1e-3).count();
        assert_eq!(drum(0.0), 24);
        assert_eq!(drum(12.0), 24);
        // Soft at first, loud at the end.
        let v = |lo: f64, hi: f64| n.iter().filter(|x| x.0 >= lo && x.0 < hi).map(|x| x.3).fold(0.0f32, f32::max);
        assert!(v(48.0, 54.0) > 3.0 * v(0.0, 6.0), "{} {}", v(0.0, 6.0), v(48.0, 54.0));
        // The tune comes in after two bars.
        assert!(between(&n, 0.0, 6.0).iter().all(|&x| x < 72));
        assert!(between(&n, 6.0, 9.0).iter().any(|&x| x >= 72));
        // Then quiet.
        assert!(between(&n, 54.0, 60.0).is_empty());
    }

    #[test]
    fn bolero_doubles_in_parallel_twelfths_later_on() {
        let mut p = defaults(RAVEL);
        p[RA_LENGTH] = 32.0;
        let n = notes(MidiFxKind::Ravel, &p, 96.0);
        // Late on (more than two fifths through), a tune note comes with its twelfth and seventeenth.
        let late: Vec<&(f64, u8, f64, f32)> = n.iter().filter(|x| x.0 > 60.0 && x.0 < 80.0).collect();
        let found = late.iter().any(|a| late.iter().any(|b| (b.0 - a.0).abs() < 1e-6 && b.1 == a.1 + 19) && late.iter().any(|b| (b.0 - a.0).abs() < 1e-6 && b.1 == a.1 + 28));
        assert!(found);
        // Not at the start.
        let early: Vec<&(f64, u8, f64, f32)> = n.iter().filter(|x| x.0 < 15.0).collect();
        assert!(!early.iter().any(|a| early.iter().any(|b| (b.0 - a.0).abs() < 1e-6 && b.1 == a.1 + 19) && early.iter().any(|b| (b.0 - a.0).abs() < 1e-6 && b.1 == a.1 + 28)));
    }

    #[test]
    fn a_bolero_tune_lasts_sixteen_bars_and_ends_home() {
        for seed in 0..8 {
            for second in [false, true] {
                let t = bolero_tune(seed, second);
                let end = t.last().unwrap();
                assert_eq!(end.start + end.len, 16 * 36);
                assert_eq!(end.deg, 0);
                assert!(t.windows(2).all(|w| w[0].start + w[0].len == w[1].start));
            }
        }
    }

    #[test]
    fn a_bulgarian_dance_accents_its_uneven_groups() {
        let p = defaults(BARTOK);
        // 2+2+3 quavers at 1/8: a bar is three and a half beats.
        let n = notes(MidiFxKind::Bartok, &p, 14.0);
        // A drone on every bar's start, and stabs where each group begins: 0, 1, 2 beats in.
        for bar in 0..4 {
            let b = bar as f64 * 3.5;
            for g in [0.0, 1.0, 2.0] {
                let stabs = n.iter().filter(|x| (x.0 - b - g).abs() < 1e-3 && x.1 < 57 && x.1 >= 46).count();
                assert!(stabs >= 3, "bar {bar} group {g}: {stabs}");
            }
            let off = n.iter().filter(|x| (x.0 - b - 2.5).abs() < 1e-3 && x.1 < 57 && x.1 >= 46).count();
            assert_eq!(off, 0);
        }
        // The acoustic scale: F sharp and B flat, never F or B.
        let tune: Vec<u8> = n.iter().filter(|x| x.1 >= 60).map(|x| x.1 % 12).collect();
        assert!(!tune.contains(&5) && !tune.contains(&11), "{tune:?}");
    }

    #[test]
    fn a_mirror_moves_in_exact_contrary_motion() {
        let mut p = defaults(BARTOK);
        p[BT_STYLE] = 2.0;
        let n = notes(MidiFxKind::Bartok, &p, 16.0);
        let mut at: std::collections::BTreeMap<i64, Vec<u8>> = Default::default();
        for x in &n {
            at.entry((x.0 * 1000.0).round() as i64).or_default().push(x.1);
        }
        assert!(at.len() >= 16);
        for v in at.values() {
            assert_eq!(v.len(), 2, "{v:?}");
            // Upper and lower always add up to the same.
            // (round C4: 60 + s and 48 - s)
            assert_eq!(v[0] as i32 + v[1] as i32, 108, "{v:?}");
        }
    }

    #[test]
    fn night_music_is_clusters_and_insects() {
        let mut p = defaults(BARTOK);
        p[BT_STYLE] = 1.0;
        let n = notes(MidiFxKind::Bartok, &p, 32.0);
        // A cluster: at least three notes a semitone apart together.
        let first: Vec<u8> = n.iter().filter(|x| x.0 < 1e-3).map(|x| x.1).collect();
        let mut c = first.clone();
        c.sort();
        assert!(c.len() >= 3 && c.windows(2).all(|w| w[1] - w[0] == 1), "{c:?}");
        // Insects up high, short.
        assert!(n.iter().filter(|x| x.1 >= 72 && x.2 - x.0 < 0.2).count() > 10);
    }

    #[test]
    fn messiaen_stays_in_his_mode() {
        let p = defaults(MESSIAEN);
        let n = notes(MidiFxKind::Messiaen, &p, 32.0);
        assert!(n.len() > 20);
        // Mode 2 on C: C C# D# E F# G A Bb.
        assert!(n.iter().all(|x| [0, 1, 3, 4, 6, 7, 9, 10].contains(&(x.1 % 12))), "{n:?}");
        // The chords' lengths are not all the same (added values).
        let starts: std::collections::BTreeSet<i64> = n.iter().map(|x| (x.0 * 8.0).round() as i64).collect();
        let starts: Vec<i64> = starts.into_iter().collect();
        let gaps: std::collections::BTreeSet<i64> = starts.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(gaps.len() >= 3, "{gaps:?}");
    }

    #[test]
    fn birds_sing_in_phrases_with_silences() {
        let mut p = defaults(MESSIAEN);
        p[ME_STYLE] = 1.0;
        for bird in 0..3 {
            p[ME_BIRD] = bird as f32;
            let n = notes(MidiFxKind::Messiaen, &p, 32.0);
            assert!(n.len() > 20, "bird {bird}: {}", n.len());
            assert!(n.iter().all(|x| x.1 >= 72), "high up");
            // Gaps: somewhere a beat without a note.
            let starts: Vec<f64> = n.iter().map(|x| x.0).collect();
            assert!(starts.windows(2).any(|w| w[1] - w[0] > 0.5), "bird {bird}");
        }
        // A thrush says each thing more than once.
        p[ME_BIRD] = 1.0;
        let n = notes(MidiFxKind::Messiaen, &p, 8.0);
        let pitches: Vec<u8> = n.iter().map(|x| x.1).collect();
        let first3 = &pitches[..3];
        assert!(pitches[3..].windows(3).any(|w| w == first3), "{pitches:?}");
    }

    #[test]
    fn rhythmic_characters_grow_shrink_and_stay() {
        let mut p = defaults(MESSIAEN);
        p[ME_STYLE] = 2.0;
        let mut st = MidiFxState::new(MidiFxKind::Messiaen);
        let spb = 24000.0;
        let mut seen = Vec::new();
        for b in 0..(40.0 * spb / 256.0) as u64 {
            let now = b * 256;
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: c_major(), scale_on: true, beat: now as f64 / spb, playing: true, sr: 48000.0, drum_notes: 0 };
            let mut out = Vec::new();
            st.process(MidiFxKind::Messiaen, &p, &[], &[], &mut out, &ctx);
            seen.push(st.visual().unwrap());
        }
        let cell = |v: u64| (0..5).map(|i| ((v >> (8 * i)) & 0xff) as u8).collect::<Vec<u8>>();
        // Each is a palindrome, always.
        for v in &seen {
            for c in [cell(v[1]), cell(v[2]), cell(v[3])] {
                let mut r = c.clone();
                r.reverse();
                assert_eq!(c, r);
            }
        }
        // The first grows past the third, the second shrinks below it.
        assert!(seen.iter().any(|v| cell(v[1])[2] > cell(v[3])[2]));
        assert!(seen.iter().any(|v| cell(v[2]).iter().sum::<u8>() < cell(v[3]).iter().sum::<u8>()));
    }

    #[test]
    fn metronomes_run_down_one_by_one() {
        let mut p = defaults(LIGETI);
        p[LI_VOICES] = 6.0;
        let len = metronome_round(0, 6, 0.6, 0.5);
        let n = notes(MidiFxKind::Ligeti, &p, len - 3.0);
        // Each metronome its own note, at its own steady tick, until it stops.
        let mut ends = Vec::new();
        for i in 0..6 {
            let pitch = SeqState::mdeg(&MidiCtx { now: 0, len: 256, samples_per_beat: 24000.0, scale: c_major(), scale_on: true, beat: 0.0, playing: true, sr: 48000.0, drum_notes: 0 }, 60.0, METRONOME_DEGREES[i], 0) as u8;
            let ticks: Vec<f64> = n.iter().filter(|x| x.1 == pitch).map(|x| x.0).collect();
            let (period, winds) = metronome(0, i, 0.6, 0.5);
            assert_eq!(ticks.len(), winds as usize, "metronome {i}");
            assert!(ticks.windows(2).all(|w| (w[1] - w[0] - period).abs() < 0.002), "metronome {i}");
            ends.push(ticks.last().copied().unwrap());
        }
        ends.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(ends.windows(2).all(|w| w[1] > w[0]), "{ends:?}");
    }

    #[test]
    fn continuum_changes_one_note_at_a_time() {
        let mut p = defaults(LIGETI);
        p[LI_STYLE] = 1.0;
        p[LI_CHANGE] = 0.9;
        let n = notes(MidiFxKind::Ligeti, &p, 64.0);
        // Two notes every step.
        let steps: std::collections::BTreeSet<i64> = n.iter().map(|x| (x.0 * 4.0).round() as i64).collect();
        assert!(steps.len() >= 250);
        // The notes heard in each bar: from one bar to the next, few change.
        let sets: Vec<std::collections::BTreeSet<u8>> = (0..16).map(|b| between(&n, b as f64 * 4.0, b as f64 * 4.0 + 4.0).into_iter().collect()).collect();
        assert!(sets.windows(2).all(|w| w[0].symmetric_difference(&w[1]).count() <= 4), "{sets:?}");
        assert_ne!(sets[0], sets[15]);
    }

    #[test]
    fn ricercata_adds_one_pitch_class_at_a_time() {
        let mut p = defaults(LIGETI);
        p[LI_STYLE] = 2.0;
        p[LI_CHANGE] = 1.0; // a stage every four bars
        let n = notes(MidiFxKind::Ligeti, &p, 64.0);
        for stage in 0..4 {
            let pcs: std::collections::BTreeSet<u8> = between(&n, stage as f64 * 16.0, stage as f64 * 16.0 + 16.0).iter().map(|v| v % 12).collect();
            assert_eq!(pcs.len(), stage + 1, "stage {stage}: {pcs:?}");
        }
        // Only the tonic at first, then the fifth.
        assert!(between(&n, 0.0, 16.0).iter().all(|v| v % 12 == 0));
        assert!(between(&n, 16.0, 32.0).iter().any(|v| v % 12 == 7));
    }

    #[test]
    fn chance_tosses_coins_and_leaves_silences() {
        let p = defaults(CAGE);
        let n = notes(MidiFxKind::Cage, &p, 64.0);
        assert!(n.len() > 10);
        // Out of the key and over a wide range.
        assert!(!in_key(&n));
        let lo = n.iter().map(|x| x.1).min().unwrap();
        let hi = n.iter().map(|x| x.1).max().unwrap();
        assert!(hi - lo > 36);
        // Silences: somewhere two beats and more without a sound.
        let mut starts: Vec<f64> = n.iter().map(|x| x.0).collect();
        starts.dedup();
        assert!(starts.windows(2).any(|w| w[1] - w[0] >= 1.0));
        // The same coins, the same piece.
        assert_eq!(n, notes(MidiFxKind::Cage, &p, 64.0));
        let mut q = p.clone();
        q[CA_COINS] = 5.0;
        assert_ne!(n, notes(MidiFxKind::Cage, &q, 64.0));
    }

    #[test]
    fn a_hexagram_reads_its_lines() {
        assert_eq!(hexagram(&[7, 7, 7, 7, 7, 7]), 63);
        assert_eq!(hexagram(&[8, 8, 8, 8, 8, 8]), 0);
        assert_eq!(hexagram(&[9, 6, 8, 7, 8, 8]), 0b001001);
    }

    #[test]
    fn a_prepared_piano_keeps_to_its_gamut() {
        let mut p = defaults(CAGE);
        p[CA_STYLE] = 1.0;
        let n = notes(MidiFxKind::Cage, &p, 64.0);
        assert!(n.len() > 20);
        let pitches: std::collections::BTreeSet<u8> = n.iter().map(|x| x.1).collect();
        assert!(pitches.len() <= 30, "{}", pitches.len());
        assert!(n.iter().all(|x| x.3 < 0.6), "quiet");
    }

    #[test]
    fn four_thirty_three_is_silent() {
        let mut p = defaults(CAGE);
        p[CA_STYLE] = 2.0;
        assert!(notes(MidiFxKind::Cage, &p, 64.0).is_empty());
    }

    #[test]
    fn einaudi_waits_four_bars_for_the_melody() {
        let p = defaults(EINAUDI);
        let n = notes(MidiFxKind::Einaudi, &p, 48.0);
        assert!(in_key(&n));
        // Quavers in the left hand from the start; four bars of four beats alone.
        assert!(between(&n, 0.0, 16.0).iter().all(|&x| x < 72));
        assert_eq!(between(&n, 0.0, 4.0).len(), 8);
        assert!(between(&n, 16.0, 48.0).iter().any(|&x| x >= 72));
        // vi IV I V in C: A, F, C, G under each bar.
        for (bar, pc) in [(0, 9), (1, 5), (2, 0), (3, 7)] {
            let low = n.iter().find(|x| (x.0 - bar as f64 * 4.0).abs() < 1e-3).map(|x| x.1 % 12);
            assert_eq!(low, Some(pc), "bar {bar}");
        }
    }

    #[test]
    fn build_grows_layer_by_layer() {
        let mut p = defaults(EINAUDI);
        p[EI_STYLE] = 1.0;
        let n = notes(MidiFxKind::Einaudi, &p, 8.0 * 4.0 * 6.0);
        let count = |stage: usize| between(&n, stage as f64 * 32.0, stage as f64 * 32.0 + 32.0).len();
        assert!(count(1) > count(0));
        assert!(count(3) > count(2));
        assert!(count(4) > count(1));
        assert!(count(5) < count(4));
    }

    #[test]
    fn chords_off_keep_the_left_hand_home_and_more_moves_it_mid_bar() {
        // Einaudi: at 0 % every bar over the tonic.
        let mut p = defaults(EINAUDI);
        p[EI_CHORD_AMOUNT] = 0.0;
        let n = notes(MidiFxKind::Einaudi, &p, 16.0);
        for bar in 0..4 {
            let low = n.iter().find(|x| (x.0 - bar as f64 * 4.0).abs() < 1e-3).map(|x| x.1 % 12);
            assert_eq!(low, Some(0), "bar {bar}");
        }
        // At 200 %: somewhere a bar's second half on a passing chord.
        p[EI_CHORD_AMOUNT] = 2.0;
        let n = notes(MidiFxKind::Einaudi, &p, 64.0);
        const OWN: [[u8; 3]; 4] = [[9, 0, 4], [5, 9, 0], [0, 4, 7], [7, 11, 2]];
        let passing = (0..16).any(|bar| n.iter().filter(|x| x.0 >= bar as f64 * 4.0 + 2.0 - 1e-3 && x.0 < bar as f64 * 4.0 + 4.0 - 1e-3 && x.1 < 72).any(|x| !OWN[bar % 4].contains(&(x.1 % 12))));
        assert!(passing);
        // Chopin and Mozart's sonata at 0 %: the left hand on the tonic triad.
        let mut p = defaults(CHOPIN);
        p[CH_CHORD_AMOUNT] = 0.0;
        assert!(notes(MidiFxKind::Chopin, &p, 32.0).iter().filter(|x| x.1 < 70).all(|x| [0, 4, 7].contains(&(x.1 % 12))));
        let mut p = defaults(MOZART);
        p[MO_STYLE] = 1.0;
        p[MO_CHORD_AMOUNT] = 0.0;
        assert!(notes(MidiFxKind::Mozart, &p, 32.0).iter().filter(|x| x.1 < 72).all(|x| [0, 4, 7].contains(&(x.1 % 12))));
    }
}
