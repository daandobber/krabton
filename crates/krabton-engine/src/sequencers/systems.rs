//! Sequencers that follow a system, a set of rules that makes the music:
//!
//! - Bells: English change ringing. Four to eight bells ring in rounds (the
//!   scale down from the treble to the tenor), then change their order row
//!   after row by a method (Plain Hunt, Plain Bob, Grandsire, Stedman) until
//!   they come back to rounds; now and then a Bob or a Single is called at a
//!   lead end, which takes them another way round. A gap before every
//!   handstroke row, as in a tower; the tenor covers behind the odd methods.
//! - Canon: a phrase of its own (or what you play) and up to four voices
//!   that follow it, each so many steps later, so many steps higher or
//!   lower, the right way up or upside down, forwards or backwards, at half,
//!   the same or double speed.
//! - Row: a twelve-tone row (or a row of the key's notes), played as it is
//!   (P), backwards (R), upside down (I) or both (RI), moved round the
//!   twelve, as a line or in chords of two, three, four or six, in even or
//!   serial rhythms, spread over octaves.
//! - Gears: up to six gears meshed in a train, a tooth passing the top each
//!   step; each has pegs on some of its teeth that play its note when they
//!   pass the top. Gears of different sizes turn at different speeds: three
//!   against four, five against eight, all meeting again when every gear is
//!   back where it began.
//! - Formula: a melody from a bytebeat formula of a counter (t*(t>>a|t>>b)
//!   and others): each step the byte it gives is a note of the key, a rest
//!   or a long note; looped to a few bars.
//! - Staircase: a stair that rises (or falls) for ever: the same note in
//!   several octaves at once, loud in the middle and fading at the top and
//!   the bottom, so as they climb the top ones fade out and new ones fade in
//!   below (a Shepard scale).

use super::minimal::{choose, hash, DYNAMICS};
use super::nature::{pitch, voice_degree};
use super::{euclid_hit, rate_beats, SeqState, RATES};
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

const KEYS: [ParamSpec; 2] = [ParamSpec::toggle("Transpose", "Keys", false), ParamSpec::toggle("Hold", "Keys", false)];

// ---------------------------------------------------------------- bells

pub const BE_RATE: usize = 0;
pub const BE_BELLS: usize = 1;
pub const BE_METHOD: usize = 2;
pub const BE_CALLS: usize = 3;
pub const BE_SINGLES: usize = 4;
pub const BE_ROUNDS: usize = 5;
pub const BE_GAP: usize = 6;
pub const BE_NOTES: usize = 7;
pub const BE_BASE: usize = 8;
pub const BE_GATE: usize = 9;
pub const BE_VELOCITY: usize = 10;
pub const BE_STRIKE: usize = 11;
pub const BE_LOOSE: usize = 12;
pub const BE_SEED: usize = 13;
pub const METHODS: &[&str] = &["Rounds", "Plain Hunt", "Plain Bob", "Grandsire", "Stedman"];
pub const BELL_NOTES: &[&str] = &["Scale", "Pentatonic", "Chord"];
pub const MAX_BELLS: usize = 8;

pub static BELLS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 4),
    ParamSpec::int("Bells", "Tower", 4.0, MAX_BELLS as f32, 6.0, Unit::None),
    ParamSpec::choice("Method", "Tower", METHODS, 2),
    ParamSpec::lin("Calls", "Tower", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Singles", "Tower", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::int("Rounds", "Tower", 0.0, 8.0, 2.0, Unit::None),
    ParamSpec::toggle("Gap", "Tower", true),
    ParamSpec::choice("Notes", "Notes", BELL_NOTES, 0),
    ParamSpec::int("Base Note", "Notes", 36.0, 84.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 2.0, 1.0, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Strike", "Notes", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Loose", "Notes", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::int("Seed", "Notes", 0.0, 99.0, 0.0, Unit::None),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

/// The method rung on `n` bells (Grandsire and Stedman need five; on fewer
/// they plain hunt).
pub fn bell_method(method: usize, n: usize) -> usize {
    if method >= 3 && n < 5 {
        1
    } else {
        method
    }
}

/// How many bells change places: all of them, but Grandsire and Stedman
/// on an even number ring the odd ones with the tenor covering behind.
pub fn working_bells(method: usize, n: usize) -> usize {
    if method >= 3 && n % 2 == 0 {
        n - 1
    } else {
        n
    }
}

/// Changes in a lead.
pub fn lead_len(method: usize, w: usize) -> usize {
    if method == 4 {
        12
    } else {
        2 * w
    }
}

/// The places that stay in change `c` of a lead (a bit each, place 1 the
/// lowest), on `w` working bells; `call` 1 a Bob, 2 a Single at the lead end.
pub fn places(method: usize, w: usize, c: usize, call: u8) -> u16 {
    let last = lead_len(method, w) - 1;
    let pl = |ps: &[usize]| ps.iter().fold(0u16, |m, &q| if q < w { m | 1 << q } else { m });
    let hunt = |c: usize| {
        if w % 2 == 0 {
            if c % 2 == 0 {
                0
            } else {
                pl(&[0, w - 1])
            }
        } else if c % 2 == 0 {
            pl(&[w - 1])
        } else {
            pl(&[0])
        }
    };
    match method {
        2 if c == last => match (w % 2 == 0, call) {
            (true, 1) => pl(&[0, 3]),
            (true, 2) => pl(&[0, 1, 2, 3]),
            (true, _) => pl(&[0, 1]),
            (false, 1) => pl(&[0, 3, w - 1]),
            (false, 2) => pl(&[0, 1, 2]),
            (false, _) => pl(&[0, 1, w - 1]),
        },
        3 => {
            if c == 0 {
                pl(&[2])
            } else if c + 1 == last && call > 0 {
                pl(&[2])
            } else if c == last && call == 2 {
                pl(&[0, 1, 2])
            } else if c % 2 == 1 {
                pl(&[0])
            } else {
                pl(&[w - 1])
            }
        }
        4 => match [3usize, 1, 0, 3, 1, 3, 1, 3, 0, 1, 3, 1][c % 12] {
            0 if c % 12 == 8 && call == 1 => pl(&[w - 3]),
            0 if c % 12 == 8 && call == 2 => pl(&[w - 3, w - 2, w - 1]),
            0 => pl(&[w - 1]),
            q => pl(&[q - 1]),
        },
        _ => hunt(c),
    }
}

/// Rings one change: the bells not in a place swap in pairs.
pub fn apply_change(row: &mut [u8], places: u16, w: usize) {
    let mut i = 0;
    while i < w {
        if places >> i & 1 == 1 || i + 1 >= w || places >> (i + 1) & 1 == 1 {
            i += 1;
        } else {
            row.swap(i, i + 1);
            i += 2;
        }
    }
}

// ---------------------------------------------------------------- canon

pub const CA_RATE: usize = 0;
pub const CA_SOURCE: usize = 1;
pub const CA_PHRASE: usize = 2;
pub const CA_LENGTH: usize = 3;
pub const CA_SEED: usize = 4;
pub const CA_VOICES: usize = 5;
pub const CA_DELAY: usize = 6; // Four voices' delays: 6..=9.
pub const CA_INTERVAL: usize = 10; // 10..=13.
pub const CA_FORM: usize = 14; // 14..=17.
pub const CA_SPEED: usize = 18; // 18..=21.
pub const CA_BASE: usize = 22;
pub const CA_GATE: usize = 23;
pub const CA_VELOCITY: usize = 24;
pub const CA_FADE: usize = 25;
pub const MAX_FOLLOWERS: usize = 4;
pub const CANON_SOURCES: &[&str] = &["Phrase", "Played"];
pub const PHRASES: &[&str] = &["Walk", "Arch", "Leaps", "Turn", "Random"];
pub const FORMS: &[&str] = &["Same", "Upside Down", "Backward", "Both"];
pub const SPEEDS: &[&str] = &["Half", "Same", "Double"];

pub static CANON: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 4),
    ParamSpec::choice("Source", "Leader", CANON_SOURCES, 0),
    ParamSpec::choice("Phrase", "Leader", PHRASES, 0),
    ParamSpec::int("Length", "Leader", 4.0, 16.0, 8.0, Unit::None),
    ParamSpec::int("Seed", "Leader", 0.0, 99.0, 0.0, Unit::None),
    ParamSpec::int("Voices", "Followers", 1.0, MAX_FOLLOWERS as f32, 2.0, Unit::None),
    ParamSpec::int("Delay 1", "Followers", 1.0, 32.0, 4.0, Unit::None),
    ParamSpec::int("Delay 2", "Followers", 1.0, 32.0, 8.0, Unit::None),
    ParamSpec::int("Delay 3", "Followers", 1.0, 32.0, 12.0, Unit::None),
    ParamSpec::int("Delay 4", "Followers", 1.0, 32.0, 16.0, Unit::None),
    ParamSpec::int("Interval 1", "Followers", -14.0, 14.0, -7.0, Unit::None),
    ParamSpec::int("Interval 2", "Followers", -14.0, 14.0, 4.0, Unit::None),
    ParamSpec::int("Interval 3", "Followers", -14.0, 14.0, -3.0, Unit::None),
    ParamSpec::int("Interval 4", "Followers", -14.0, 14.0, 7.0, Unit::None),
    ParamSpec::choice("Form 1", "Followers", FORMS, 0),
    ParamSpec::choice("Form 2", "Followers", FORMS, 0),
    ParamSpec::choice("Form 3", "Followers", FORMS, 1),
    ParamSpec::choice("Form 4", "Followers", FORMS, 0),
    ParamSpec::choice("Speed 1", "Followers", SPEEDS, 1),
    ParamSpec::choice("Speed 2", "Followers", SPEEDS, 1),
    ParamSpec::choice("Speed 3", "Followers", SPEEDS, 0),
    ParamSpec::choice("Speed 4", "Followers", SPEEDS, 2),
    ParamSpec::int("Base Note", "Notes", 36.0, 84.0, 64.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Fade", "Notes", 0.0, 1.0, 0.25, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

pub const PHRASE_LEN: usize = 16;
/// A phrase: a scale degree a step (None: a rest).
pub type Phrase = [Option<i8>; PHRASE_LEN];

/// A phrase of `len` steps in `shape`, made from `seed`: it starts on the
/// base note and moves within a little over an octave.
pub fn phrase(shape: usize, len: usize, seed: u32) -> Phrase {
    let len = len.clamp(1, PHRASE_LEN);
    let mut out = [None; PHRASE_LEN];
    let mut d = 0i32;
    for (i, slot) in out.iter_mut().enumerate().take(len) {
        let h = hash(seed.wrapping_mul(7) + 3 + shape as u32 * 1000, i as u32);
        if i > 0 {
            d = match shape {
                1 => {
                    let x = i as f32 / (len - 1).max(1) as f32;
                    let top = 4.0 + (seed % 3) as f32;
                    (top * (1.0 - (2.0 * x - 1.0).powi(2))).round() as i32 + if h < 0.25 { 1 } else { 0 }
                }
                2 => d + [2, -2, 3, -3, 4, -1, 2, -4][(h * 8.0) as usize % 8],
                3 => [0, 1, 0, -1, 0, 2, 1, 0][i % 8] + 2 * ((i / 8) as i32 + (seed % 2) as i32),
                4 => (h * 9.0) as i32 - 1,
                _ => d + [-1, 1, 1, -1, 0, 2, -2, 1][(h * 8.0) as usize % 8],
            }
            .clamp(-4, 9);
        }
        let rest = i > 0 && hash(seed.wrapping_mul(5) + 1, i as u32) < 0.12;
        *slot = if rest { None } else { Some(d as i8) };
    }
    out
}

/// The degree a follower plays at its step `src` of the phrase (`form` one
/// of `FORMS`), before its interval.
pub fn canon_degree(ph: &Phrase, len: usize, src: i64, form: usize) -> Option<i32> {
    let len = len.clamp(1, PHRASE_LEN);
    let mut i = src.rem_euclid(len as i64) as usize;
    if form >= 2 {
        i = len - 1 - i;
    }
    ph[i].map(|d| if form % 2 == 1 { -(d as i32) } else { d as i32 })
}

// ---------------------------------------------------------------- row

pub const RO_RATE: usize = 0;
pub const RO_ROW: usize = 1;
pub const RO_SEED: usize = 2;
pub const RO_FORM: usize = 3;
pub const RO_SHIFT: usize = 4;
pub const RO_MOVE: usize = 5;
pub const RO_TEXTURE: usize = 6;
pub const RO_RHYTHM: usize = 7;
pub const RO_NOTES: usize = 8;
pub const RO_OCTAVES: usize = 9;
pub const RO_BASE: usize = 10;
pub const RO_GATE: usize = 11;
pub const RO_VELOCITY: usize = 12;
pub const RO_DYNAMICS: usize = 13;
pub const RO_SWING: usize = 14;
pub const ROWS: &[&str] = &["Shuffle", "All Intervals", "Fourths", "Chromatic"];
pub const ROW_FORMS: &[&str] = &["P", "R", "I", "RI", "Turn", "Chance"];
pub const MOVES: &[&str] = &["Stay", "By Row", "Fifths", "Chance"];
pub const TEXTURES: &[&str] = &["Line", "Twos", "Threes", "Fours", "Sixes", "Split"];
pub const ROW_RHYTHMS: &[&str] = &["Even", "Long Short", "Serial", "Scattered"];
pub const ROW_NOTES: &[&str] = &["Twelve Tones", "Key"];

pub static ROW: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 4),
    ParamSpec::choice("Row", "Row", ROWS, 0),
    ParamSpec::int("Seed", "Row", 0.0, 99.0, 0.0, Unit::None),
    ParamSpec::choice("Form", "Row", ROW_FORMS, 4),
    ParamSpec::int("Shift", "Row", 0.0, 11.0, 0.0, Unit::None),
    ParamSpec::choice("Move", "Row", MOVES, 1),
    ParamSpec::choice("Texture", "Play", TEXTURES, 0),
    ParamSpec::choice("Rhythm", "Play", ROW_RHYTHMS, 2),
    ParamSpec::choice("Notes", "Play", ROW_NOTES, 0),
    ParamSpec::int("Octaves", "Play", 1.0, 4.0, 2.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 36.0, 84.0, 55.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Dynamics", "Notes", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Swing", "Notes", 0.0, 0.75, 0.0, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// The prime row of `m` notes (12, or the key's): `kind` one of `ROWS`.
pub fn prime_row(kind: usize, seed: u32, m: usize) -> [u8; 12] {
    let m = m.clamp(1, 12);
    let mut r = [0u8; 12];
    for (i, x) in r.iter_mut().enumerate().take(m) {
        *x = i as u8;
    }
    match kind {
        1 => {
            // The wedge: 0, 11, 1, 10, 2, 9 ... every interval once.
            for (i, x) in r.iter_mut().enumerate().take(m) {
                *x = (if i % 2 == 0 { i / 2 } else { m - 1 - i / 2 }) as u8;
            }
        }
        2 => {
            let step = if m == 12 { 5 } else { [3, 2, 1].into_iter().find(|&s| gcd(s, m) == 1).unwrap_or(1) };
            for (i, x) in r.iter_mut().enumerate().take(m) {
                *x = (i * step % m) as u8;
            }
        }
        3 => {}
        _ => {
            for i in (1..m).rev() {
                let j = (hash(seed.wrapping_mul(131) + 17, i as u32) * (i + 1) as f32) as usize % (i + 1);
                r.swap(i, j);
            }
        }
    }
    r
}

/// A form of the row (`form` 0 P, 1 R, 2 I, 3 RI), moved up `t`.
pub fn row_form(prime: &[u8; 12], m: usize, form: usize, t: usize) -> [u8; 12] {
    let m = m.clamp(1, 12);
    let mut out = [0u8; 12];
    let first = prime[0] as usize;
    for (i, x) in out.iter_mut().enumerate().take(m) {
        let j = if form % 2 == 1 { m - 1 - i } else { i };
        let pc = prime[j] as usize;
        let v = if form >= 2 { (2 * first + m * 2 - pc) % m } else { pc };
        *x = ((v + t) % m) as u8;
    }
    out
}

// ---------------------------------------------------------------- gears

pub const GE_RATE: usize = 0;
pub const GE_GEARS: usize = 1;
pub const GE_TEETH: usize = 2; // Six gears' teeth: 2..=7.
pub const GE_PEGS: usize = 8; // 8..=13.
pub const GE_NOTE: usize = 14; // 14..=19.
pub const GE_BASE: usize = 20;
pub const GE_GATE: usize = 21;
pub const GE_VELOCITY: usize = 22;
pub const GE_ACCENT: usize = 23;
pub const GE_SLIP: usize = 24;
pub const GE_SWING: usize = 25;
pub const MAX_GEARS: usize = 6;

pub static GEARS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::int("Gears", "Train", 2.0, MAX_GEARS as f32, 3.0, Unit::None),
    ParamSpec::int("Teeth 1", "Teeth", 3.0, 32.0, 16.0, Unit::None),
    ParamSpec::int("Teeth 2", "Teeth", 3.0, 32.0, 12.0, Unit::None),
    ParamSpec::int("Teeth 3", "Teeth", 3.0, 32.0, 9.0, Unit::None),
    ParamSpec::int("Teeth 4", "Teeth", 3.0, 32.0, 10.0, Unit::None),
    ParamSpec::int("Teeth 5", "Teeth", 3.0, 32.0, 6.0, Unit::None),
    ParamSpec::int("Teeth 6", "Teeth", 3.0, 32.0, 20.0, Unit::None),
    ParamSpec::int("Pegs 1", "Pegs", 1.0, 8.0, 4.0, Unit::None),
    ParamSpec::int("Pegs 2", "Pegs", 1.0, 8.0, 2.0, Unit::None),
    ParamSpec::int("Pegs 3", "Pegs", 1.0, 8.0, 3.0, Unit::None),
    ParamSpec::int("Pegs 4", "Pegs", 1.0, 8.0, 2.0, Unit::None),
    ParamSpec::int("Pegs 5", "Pegs", 1.0, 8.0, 1.0, Unit::None),
    ParamSpec::int("Pegs 6", "Pegs", 1.0, 8.0, 5.0, Unit::None),
    ParamSpec::int("Note 1", "Notes", -14.0, 14.0, 0.0, Unit::None),
    ParamSpec::int("Note 2", "Notes", -14.0, 14.0, 4.0, Unit::None),
    ParamSpec::int("Note 3", "Notes", -14.0, 14.0, 7.0, Unit::None),
    ParamSpec::int("Note 4", "Notes", -14.0, 14.0, 2.0, Unit::None),
    ParamSpec::int("Note 5", "Notes", -14.0, 14.0, 9.0, Unit::None),
    ParamSpec::int("Note 6", "Notes", -14.0, 14.0, 11.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 55.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 2.0, 0.5, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Accent", "Notes", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Slip", "Notes", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Swing", "Notes", 0.0, 0.75, 0.0, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

/// Whether gear `i`'s tooth at the top at step `k` carries a peg.
pub fn gear_peg(p: &[f32], i: usize, k: i64) -> bool {
    let teeth = p[GE_TEETH + i].round().clamp(3.0, 32.0) as usize;
    let pegs = (p[GE_PEGS + i].round().max(1.0) as usize).min(teeth);
    euclid_hit(k.rem_euclid(teeth as i64) as usize, teeth, pegs, 0)
}

// ---------------------------------------------------------------- formula

pub const FO_RATE: usize = 0;
pub const FO_FORMULA: usize = 1;
pub const FO_A: usize = 2;
pub const FO_B: usize = 3;
pub const FO_STEP: usize = 4;
pub const FO_SEED: usize = 5;
pub const FO_LOOP: usize = 6;
pub const FO_MAP: usize = 7;
pub const FO_RANGE: usize = 8;
pub const FO_RESTS: usize = 9;
pub const FO_LEGATO: usize = 10;
pub const FO_BASE: usize = 11;
pub const FO_GATE: usize = 12;
pub const FO_VELOCITY: usize = 13;
pub const FO_DYNAMICS: usize = 14;
pub const FO_SWING: usize = 15;
pub const FORMULAS: &[&str] = &["t*(t>>a|t>>b)", "t*(t>>a)", "t*b^t>>a", "t^t>>a^t>>b", "t*a%(b+7)", "t*a+(t>>b)", "t*a|t>>b", "t*(a+(t>>b&3))", "(t*a^t>>b)&63"];
pub const MAPS: &[&str] = &["Spread", "Wrap", "Steps"];
/// How far the counter goes each step (the formulas were made for a fast one).
pub const FORMULA_STEPS: &[&str] = &["1", "4", "16", "64", "256"];

pub static FORMULA: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Formula", "Formula", FORMULAS, 0),
    ParamSpec::int("A", "Formula", 1.0, 16.0, 5.0, Unit::None),
    ParamSpec::int("B", "Formula", 1.0, 16.0, 8.0, Unit::None),
    ParamSpec::choice("Step", "Formula", FORMULA_STEPS, 2),
    ParamSpec::int("Seed", "Formula", 0.0, 255.0, 0.0, Unit::None),
    ParamSpec::choice("Loop", "Formula", super::bits::LOOPS, 2),
    ParamSpec::choice("Map", "Notes", MAPS, 0),
    ParamSpec::int("Range", "Notes", 2.0, 21.0, 9.0, Unit::None),
    ParamSpec::lin("Rests", "Notes", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::toggle("Legato", "Notes", false),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 55.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Dynamics", "Notes", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Swing", "Notes", 0.0, 0.75, 0.0, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

/// The byte formula `f` gives at `t`.
pub fn formula_byte(f: usize, t: u32, a: u32, b: u32) -> u8 {
    let (a, b) = (a.clamp(1, 16), b.clamp(1, 16));
    let v = match f {
        1 => t.wrapping_mul(t >> a),
        2 => t.wrapping_mul(b) ^ (t >> a),
        3 => t ^ (t >> a) ^ (t >> b),
        4 => t.wrapping_mul(a) % (b + 7) * 256 / (b + 7),
        5 => t.wrapping_mul(a).wrapping_add(t >> b),
        6 => t.wrapping_mul(a) | (t >> b),
        7 => t.wrapping_mul(a + ((t >> b) & 3)),
        8 => ((t.wrapping_mul(a) ^ (t >> b)) & 63) << 2,
        _ => t.wrapping_mul((t >> a) | (t >> b)),
    };
    v as u8
}

/// What Formula plays at counter `t`: the degree, or None for a rest.
pub fn formula_note(p: &[f32], t: u32) -> Option<i32> {
    let f = choose(p, FO_FORMULA, FORMULAS.len());
    let v = formula_byte(f, t, p[FO_A].round() as u32, p[FO_B].round() as u32) as u32;
    let rest = ((v ^ t).wrapping_mul(0x9E37) >> 6 & 255) as f32 / 256.0 < p[FO_RESTS];
    if rest {
        return None;
    }
    let range = p[FO_RANGE].round().max(1.0) as u32;
    Some(match choose(p, FO_MAP, MAPS.len()) {
        1 => v % (range + 1),
        2 => (v >> 5) * range / 7,
        _ => v * (range + 1) / 256,
    } as i32)
}

/// How far the counter goes each step.
pub fn formula_stride(p: &[f32]) -> u32 {
    [1, 4, 16, 64, 256][choose(p, FO_STEP, FORMULA_STEPS.len())]
}

/// The counter at song step `k` (looped to a few bars, moved by the seed).
pub fn formula_t(k: i64, p: &[f32], steps_per_bar: i64) -> u32 {
    let bars = [0i64, 1, 2, 4, 8][choose(p, FO_LOOP, 5)];
    let k = if bars > 0 { k.rem_euclid(bars * steps_per_bar.max(1)) } else { k.max(0) };
    (k as u32).wrapping_add(p[FO_SEED].round().clamp(0.0, 255.0) as u32).wrapping_mul(formula_stride(p))
}

// ---------------------------------------------------------------- staircase

pub const SC_RATE: usize = 0;
pub const SC_DIRECTION: usize = 1;
pub const SC_STEP: usize = 2;
pub const SC_EVERY: usize = 3;
pub const SC_VOICES: usize = 4;
pub const SC_CENTER: usize = 5;
pub const SC_WIDTH: usize = 6;
pub const SC_PLAY: usize = 7;
pub const SC_GATE: usize = 8;
pub const SC_VELOCITY: usize = 9;
pub const SC_SWING: usize = 10;
pub const DIRECTIONS: &[&str] = &["Up", "Down", "Up & Down", "Wander"];
pub const STAIR_PLAYS: &[&str] = &["Chord", "Rolled", "Climb"];

pub static STAIRCASE: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 4),
    ParamSpec::choice("Direction", "Stair", DIRECTIONS, 0),
    ParamSpec::int("Step", "Stair", 1.0, 4.0, 1.0, Unit::None),
    ParamSpec::int("Every", "Stair", 1.0, 8.0, 2.0, Unit::None),
    ParamSpec::int("Voices", "Stair", 2.0, 6.0, 4.0, Unit::None),
    ParamSpec::int("Center", "Stair", 36.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Width", "Stair", 0.5, 3.0, 1.2, Unit::Octaves),
    ParamSpec::choice("Play", "Notes", STAIR_PLAYS, 1),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.9, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Swing", "Notes", 0.0, 0.75, 0.0, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

/// The stair's voices at position `x` (scale steps climbed): each voice's
/// absolute degree and how loud it is (0..1), for a scale of `n` notes.
pub fn stair_voices(p: &[f32], x: i64, n: usize, center_degree: i32) -> [(i32, f32); 6] {
    let voices = (p[SC_VOICES].round() as usize).clamp(2, 6);
    let n = n.max(1) as i64;
    let span = voices as i64 * n;
    let bottom = center_degree as i64 - span / 2;
    let width = p[SC_WIDTH].clamp(0.25, 4.0) * n as f32;
    let mut out = [(0, 0.0); 6];
    for (j, o) in out.iter_mut().enumerate().take(voices) {
        let d = bottom + (x + j as i64 * n).rem_euclid(span);
        let rel = (d as f32 - center_degree as f32) / width;
        *o = (d as i32, (-2.0 * rel * rel).exp());
    }
    out
}

// ---------------------------------------------------------------- state

#[derive(Clone, Copy)]
pub(crate) struct BellState {
    pub(crate) row: [u8; MAX_BELLS],
    place: usize,
    stroke: u8,
    gap_done: bool,
    /// 0 rounds before going into changes, 1 ringing changes, 2 rounds after.
    pub(crate) mode: u8,
    mode_rows: i32,
    change: usize,
    lead: u32,
    call: u8,
    shown_call: u8,
    touch: u32,
    pub(crate) rows: u32,
    key: (usize, usize),
}

impl Default for BellState {
    fn default() -> Self {
        Self { row: [0, 1, 2, 3, 4, 5, 6, 7], place: 0, stroke: 0, gap_done: false, mode: 0, mode_rows: 0, change: 0, lead: 0, call: 0, shown_call: 0, touch: 0, rows: 0, key: (0, usize::MAX) }
    }
}

pub struct SystemsState {
    pub(crate) bells: BellState,
    // Canon (played): where the phrase began (frame) and on which degree, and the last note.
    canon_anchor: u64,
    canon_anchor_degree: i32,
    canon_last: u64,
    canon_shown: [u8; 5],
    // Row: where in the row, which time round, steps left of a long note.
    row_pos: usize,
    row_round: u32,
    row_wait: u32,
    row_key: (usize, u32, usize),
    // Staircase (Wander): where it is and the last step it was at.
    stair_x: i64,
    stair_k: i64,
}

impl Default for SystemsState {
    fn default() -> Self {
        Self { bells: BellState::default(), canon_anchor: 0, canon_anchor_degree: 0, canon_last: 0, canon_shown: [0; 5], row_pos: 0, row_round: 0, row_wait: 0, row_key: (usize::MAX, 0, 0), stair_x: 0, stair_k: i64::MIN }
    }
}

impl SeqState {
    // ------------------------------------------------------------ bells

    /// Bells: one bell a step, row after row; a gap before each handstroke row.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn bells(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let n = (p[BE_BELLS].round() as usize).clamp(4, MAX_BELLS);
        let method = bell_method(choose(p, BE_METHOD, METHODS.len()), n);
        let rounds = p[BE_ROUNDS].round().clamp(0.0, 8.0) as i32;
        let b = &mut self.systems.bells;
        if b.key != (n, method) {
            *b = BellState { key: (n, method), mode_rows: rounds.max(1), ..BellState::default() };
        }
        let gap = p[BE_GAP] >= 0.5;
        if b.place == 0 && b.stroke == 0 && gap && !b.gap_done {
            // The open handstroke lead: a beat of silence.
            b.gap_done = true;
            self.bells_visual(n, None);
            return;
        }
        let bell = b.row[b.place.min(n - 1)] as usize;
        let stroke = b.stroke;
        b.place += 1;
        if b.place >= n {
            Self::next_row(b, p, n, method, rounds);
        }
        let notes = [0, 2, 1][choose(p, BE_NOTES, BELL_NOTES.len())];
        // The tenor (the last bell) the lowest, the treble the highest.
        let degree = voice_degree(notes, n - 1 - bell, 1);
        let strike = p[BE_STRIKE].clamp(0.0, 1.0);
        let mut vel = p[BE_VELOCITY] * if stroke == 0 { 1.0 } else { 1.0 - 0.25 * strike };
        if bell == 0 {
            vel *= 1.0 + 0.2 * strike;
        }
        let loose = p[BE_LOOSE].clamp(0.0, 1.0) as f64;
        let at = at + (hash(0xBE11, k as u32) as f64 * loose * 0.3 * step_frames) as u64;
        let note = pitch(ctx, p[BE_BASE], degree);
        self.play(ctx, at, note, vel.min(1.0), step_frames * p[BE_GATE].max(0.05) as f64, out);
        self.bells_visual(n, Some(bell));
    }

    /// The row is rung: on to the next (a change, or rounds again).
    fn next_row(b: &mut BellState, p: &[f32], n: usize, method: usize, rounds: i32) {
        b.place = 0;
        b.gap_done = false;
        b.stroke ^= 1;
        b.rows = b.rows.wrapping_add(1);
        let w = working_bells(method, n);
        let seed = p[BE_SEED].round() as u32;
        let decide = |b: &BellState| -> u8 {
            let h = hash(seed * 13 + 5 + b.touch * 101, b.lead);
            if h < p[BE_CALLS] {
                if hash(seed * 7 + 9 + b.touch * 31, b.lead) < p[BE_SINGLES] {
                    2
                } else {
                    1
                }
            } else {
                0
            }
        };
        match b.mode {
            1 => {
                let len = lead_len(method, w);
                if b.change + 1 >= len.saturating_sub(2) {
                    b.shown_call = b.call;
                }
                apply_change(&mut b.row[..n], places(method, w, b.change, b.call), w);
                b.change += 1;
                if b.change >= len {
                    b.change = 0;
                    b.lead += 1;
                    b.call = decide(b);
                } else if b.change == 2 {
                    b.shown_call = 0;
                }
                let home = b.row[..n].iter().enumerate().all(|(i, &x)| x as usize == i);
                if home || b.lead >= 48 {
                    // Back in rounds (or "that's all" after a long touch).
                    for (i, x) in b.row.iter_mut().enumerate() {
                        *x = i as u8;
                    }
                    b.mode = 2;
                    b.mode_rows = rounds;
                    b.shown_call = 0;
                    if rounds == 0 {
                        Self::go(b);
                        b.call = decide(b);
                    }
                }
            }
            _ => {
                b.mode_rows -= 1;
                if b.mode_rows <= 0 && method != 0 {
                    Self::go(b);
                    b.call = decide(b);
                }
            }
        }
    }

    /// "Go": into changes from rounds.
    fn go(b: &mut BellState) {
        b.mode = 1;
        b.change = 0;
        b.lead = 0;
        b.touch = b.touch.wrapping_add(1);
    }

    fn bells_visual(&mut self, n: usize, struck: Option<usize>) {
        let b = &self.systems.bells;
        let mut row = 0u64;
        for i in 0..n {
            row |= (b.row[i] as u64 & 0xf) << (i * 4);
        }
        self.visual[0] = row | (b.place as u64) << 32 | (n as u64) << 36 | (b.stroke as u64) << 40 | (b.shown_call as u64 & 3) << 41 | (b.mode as u64 & 3) << 43 | (b.change as u64 & 0xff) << 48;
        self.visual[1] = b.rows as u64 | (b.lead as u64) << 32;
        self.visual[2] = struck.map(|s| s as u64 + 1).unwrap_or(0);
    }

    // ------------------------------------------------------------ canon

    /// Canon on its own phrase: the leader and each voice following it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn canon(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let len = (p[CA_LENGTH].round() as usize).clamp(4, PHRASE_LEN);
        let ph = phrase(choose(p, CA_PHRASE, PHRASES.len()), len, p[CA_SEED].round() as u32);
        let voices = (p[CA_VOICES].round() as usize).clamp(1, MAX_FOLLOWERS);
        let gate = step_frames * p[CA_GATE] as f64;
        let fade = p[CA_FADE].clamp(0.0, 1.0);
        let mut bytes = [0u8; 32];
        let lead = k.rem_euclid(len as i64) as usize;
        bytes[0] = lead as u8;
        bytes[5] = len as u8;
        bytes[6] = voices as u8;
        if let Some(d) = ph[lead] {
            let note = pitch(ctx, p[CA_BASE], d as i32);
            self.note(ctx, at, note, p[CA_VELOCITY], gate, out);
            bytes[8] = (d as i32 + 64) as u8;
            bytes[13] |= 1;
        }
        for v in 0..voices {
            let pos = k - p[CA_DELAY + v].round().max(1.0) as i64;
            if pos < 0 {
                // Not in yet.
                bytes[1 + v] = 255;
                continue;
            }
            let form = choose(p, CA_FORM + v, FORMS.len());
            let interval = p[CA_INTERVAL + v].round() as i32;
            let vel = p[CA_VELOCITY] * (1.0 - fade * (v + 1) as f32 / 5.0);
            // (source step, when, how long)
            let mut plays = [(0i64, 0u64, 0.0f64); 2];
            let n = match choose(p, CA_SPEED + v, SPEEDS.len()) {
                0 => {
                    plays[0] = (pos / 2, at, gate * 2.0);
                    (pos % 2 == 0) as usize
                }
                2 => {
                    plays[0] = (pos * 2, at, gate * 0.5);
                    plays[1] = (pos * 2 + 1, at + (step_frames * 0.5) as u64, gate * 0.5);
                    2
                }
                _ => {
                    plays[0] = (pos, at, gate);
                    1
                }
            };
            bytes[1 + v] = (plays[0].0.rem_euclid(len as i64)) as u8;
            for &(src, when, l) in &plays[..n] {
                if let Some(d) = canon_degree(&ph, len, src, form) {
                    let note = pitch(ctx, p[CA_BASE], d + interval);
                    self.play(ctx, when, note, vel, l, out);
                    bytes[9 + v] = (d + interval + 64).clamp(0, 255) as u8;
                    bytes[13] |= 2 << v;
                }
            }
        }
        self.visual = super::nature::pack(&bytes);
    }

    /// Canon on what is played: each note you play is answered by the
    /// voices (Backward needs the future, so it plays as Same here).
    pub(crate) fn canon_keys(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let step = rate_beats(p[CA_RATE]) * ctx.samples_per_beat;
        let voices = (p[CA_VOICES].round() as usize).clamp(1, MAX_FOLLOWERS);
        let fade = p[CA_FADE].clamp(0.0, 1.0);
        let incoming = out.len();
        for i in 0..incoming {
            let e = out[i];
            if !e.on {
                continue;
            }
            let f = ctx.now + e.offset as u64;
            let (d, rest) = ctx.scale.locate(e.pitch as i32);
            let c = &mut self.systems;
            // A new phrase after two bars of quiet: it starts where this note is.
            if c.canon_last == 0 || f.saturating_sub(c.canon_last) as f64 > 8.0 * ctx.samples_per_beat || f < c.canon_anchor {
                c.canon_anchor = f;
                c.canon_anchor_degree = d;
            }
            c.canon_last = f;
            c.canon_shown[0] = (d - c.canon_anchor_degree + 64).clamp(0, 255) as u8;
            let (anchor, anchor_d) = (c.canon_anchor, c.canon_anchor_degree);
            for v in 0..voices {
                let stretch = [2.0, 1.0, 0.5][choose(p, CA_SPEED + v, SPEEDS.len())];
                let form = choose(p, CA_FORM + v, FORMS.len());
                let delay = p[CA_DELAY + v].round().max(1.0) as f64 * step;
                let when = (anchor as f64 + (f - anchor) as f64 * stretch + delay) as u64;
                let dv = if form % 2 == 1 { 2 * anchor_d - d } else { d } + p[CA_INTERVAL + v].round() as i32;
                let note = ctx.scale.pitch_at(dv) + if form % 2 == 1 { 0 } else { rest };
                let vel = e.vel * (1.0 - fade * (v + 1) as f32 / 5.0);
                self.play(ctx, when.max(f + 1), note, vel, step * p[CA_GATE] as f64 * stretch, out);
                self.systems.canon_shown[1 + v] = (dv - anchor_d + 64).clamp(0, 255) as u8;
            }
        }
        let mut bytes = [0u8; 32];
        bytes[6] = voices as u8;
        bytes[7] = 1;
        bytes[8..13].copy_from_slice(&self.systems.canon_shown);
        self.visual = super::nature::pack(&bytes);
    }

    // ------------------------------------------------------------ row

    /// Row: the next note (or chord) of the row in the form of this time round.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn row(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let twelve = choose(p, RO_NOTES, ROW_NOTES.len()) == 0;
        let m = if twelve { 12 } else { ctx.scale.len().clamp(1, 12) };
        let kind = choose(p, RO_ROW, ROWS.len());
        let seed = p[RO_SEED].round() as u32;
        let st = &mut self.systems;
        if st.row_key != (kind, seed, m) {
            st.row_key = (kind, seed, m);
            st.row_pos = 0;
            st.row_round = 0;
            st.row_wait = 0;
        }
        if st.row_wait > 0 {
            st.row_wait -= 1;
            return;
        }
        let prime = prime_row(kind, seed, m);
        let round = st.row_round;
        let form = match choose(p, RO_FORM, ROW_FORMS.len()) {
            f @ 0..=3 => f,
            4 => round as usize % 4,
            _ => (hash(seed + 41, round) * 4.0) as usize % 4,
        };
        let base_t = p[RO_SHIFT].round().clamp(0.0, 11.0) as usize;
        let t = match choose(p, RO_MOVE, MOVES.len()) {
            1 => base_t + prime[round as usize % m] as usize,
            2 => base_t + round as usize * if twelve { 7 } else { 4 },
            3 => base_t + (hash(seed + 43, round) * m as f32) as usize,
            _ => base_t,
        } % m;
        let r = row_form(&prime, m, form, t);
        let texture = choose(p, RO_TEXTURE, TEXTURES.len());
        let g = [1, 2, 3, 4, 6, 1][texture].min(m);
        let pos = st.row_pos;
        let end = (pos + g).min(m);
        let steps = match choose(p, RO_RHYTHM, ROW_RHYTHMS.len()) {
            1 => {
                if (pos / g) % 2 == 0 {
                    2
                } else {
                    1
                }
            }
            2 => r[pos] as u32 % 4 + 1,
            3 => 1 + (hash(seed + 47, round * 12 + pos as u32) * 3.0) as u32,
            _ => 1,
        };
        st.row_wait = steps - 1;
        st.row_pos = end;
        if st.row_pos >= m {
            st.row_pos = 0;
            st.row_round = st.row_round.wrapping_add(1);
        }
        let octaves = p[RO_OCTAVES].round().clamp(1.0, 4.0) as i32;
        let dynamics = p[RO_DYNAMICS].clamp(0.0, 1.0);
        let at = SeqState::swing_at(at, k, p[RO_SWING], step_frames);
        let len = step_frames * steps as f64 * p[RO_GATE] as f64;
        let mut lit = 0u64;
        for (i, &pc) in r.iter().enumerate().take(end).skip(pos) {
            let pc = pc as i32;
            let oct = if texture == 5 { (i % 2) as i32 * (octaves - 1).max(1) } else { (hash(seed + 3, pc as u32) * octaves as f32) as i32 % octaves };
            let note = if twelve {
                let base = p[RO_BASE].round() as i32;
                let low = base - (base - ctx.scale.root as i32).rem_euclid(12);
                low + pc + 12 * oct
            } else {
                pitch(ctx, p[RO_BASE], pc + m as i32 * oct)
            };
            // Serial dynamics: each note of the row its own loudness.
            let level = ((pc as usize * 5) % m) as f32 / (m - 1).max(1) as f32;
            let vel = p[RO_VELOCITY] * (1.0 - dynamics + dynamics * (0.35 + 0.65 * level));
            self.note(ctx, at, note, vel, len, out);
            lit |= 1 << i;
        }
        let mut row_bits = 0u64;
        for (i, &pc) in r.iter().enumerate().take(m) {
            row_bits |= (pc as u64 & 0xf) << (i * 4);
        }
        self.visual[0] = row_bits | (form as u64) << 48 | (pos as u64) << 52 | (m as u64 & 0xf) << 56;
        self.visual[1] = t as u64 | (round as u64 & 0xff) << 8 | lit << 16;
    }

    // ------------------------------------------------------------ gears

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn gears(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let n = (p[GE_GEARS].round() as usize).clamp(2, MAX_GEARS);
        let accent = p[GE_ACCENT].clamp(0.0, 1.0);
        let slip = p[GE_SLIP].clamp(0.0, 1.0);
        let at = SeqState::swing_at(at, k, p[GE_SWING], step_frames);
        let mut bytes = [0u8; 32];
        bytes[6] = n as u8;
        let tooth = |i: usize| k.rem_euclid(p[GE_TEETH + i].round().clamp(3.0, 32.0) as i64) as usize;
        // Every gear back where it began: the whole train's downbeat.
        let home = (0..n).all(|i| tooth(i) == 0);
        for i in 0..n {
            bytes[i] = tooth(i) as u8;
            if !gear_peg(p, i, k) || hash(0x6EA2 + i as u32, k as u32) < slip {
                continue;
            }
            bytes[7] |= 1 << i;
            let first = tooth(i) == 0;
            let vel = p[GE_VELOCITY] * (1.0 - 0.3 * accent + if first { 0.3 * accent } else { 0.0 } + if home { 0.2 * accent } else { 0.0 });
            let note = pitch(ctx, p[GE_BASE], p[GE_NOTE + i].round() as i32);
            self.note(ctx, at, note, vel.min(1.0), step_frames * p[GE_GATE].max(0.05) as f64, out);
        }
        bytes[8] = home as u8;
        self.visual = super::nature::pack(&bytes);
    }

    // ------------------------------------------------------------ formula

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn formula(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let per_bar = (4.0 / rate_beats(p[FO_RATE])).round() as i64;
        let t = formula_t(k, p, per_bar);
        let note = formula_note(p, t);
        let legato = p[FO_LEGATO] >= 0.5;
        let f = choose(p, FO_FORMULA, FORMULAS.len());
        let v = formula_byte(f, t, p[FO_A].round() as u32, p[FO_B].round() as u32);
        self.visual[0] = t as u64 | (note.map(|d| d + 1).unwrap_or(0) as u64 & 0xff) << 32;
        let Some(d) = note else { return };
        let mut steps = 1;
        if legato {
            // The same note on the step before: it is still sounding.
            if k > 0 && formula_note(p, formula_t(k - 1, p, per_bar)) == Some(d) {
                return;
            }
            while steps < 8 && formula_note(p, formula_t(k + steps, p, per_bar)) == Some(d) {
                steps += 1;
            }
        }
        let dynamics = p[FO_DYNAMICS].clamp(0.0, 1.0);
        let vel = p[FO_VELOCITY] * (1.0 - dynamics + dynamics * (0.4 + 0.6 * (v >> 4) as f32 / 15.0));
        let at = SeqState::swing_at(at, k, p[FO_SWING], step_frames);
        let pch = pitch(ctx, p[FO_BASE], d);
        let len = if legato { step_frames * (steps as f64 - 1.0 + p[FO_GATE] as f64) } else { step_frames * p[FO_GATE] as f64 };
        self.note(ctx, at, pch, vel, len, out);
    }

    // ------------------------------------------------------------ staircase

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn staircase(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let n = ctx.scale.len().max(1) as i64;
        let voices = (p[SC_VOICES].round() as usize).clamp(2, 6);
        let step = p[SC_STEP].round().clamp(1.0, 4.0) as i64;
        let every = p[SC_EVERY].round().clamp(1.0, 8.0) as i64;
        let moves = k.max(0) / every;
        let span = voices as i64 * n;
        let x = match choose(p, SC_DIRECTION, DIRECTIONS.len()) {
            1 => -moves * step,
            2 => {
                // Up a whole turn of the stair, then down it again.
                let m = (moves * step).rem_euclid(2 * span);
                if m < span {
                    m
                } else {
                    2 * span - m
                }
            }
            3 => {
                let st = &mut self.systems;
                if k != st.stair_k {
                    if k != st.stair_k + 1 && k % every != 0 {
                        st.stair_x = 0;
                    }
                    if k % every == 0 {
                        st.stair_x += if hash(0x57A1, k as u32) < 0.6 { step } else { -step };
                    }
                    st.stair_k = k;
                }
                st.stair_x
            }
            _ => moves * step,
        };
        let (center_degree, _) = ctx.scale.locate(p[SC_CENTER].round() as i32);
        let vs = stair_voices(p, x, n as usize, center_degree);
        let play = choose(p, SC_PLAY, STAIR_PLAYS.len());
        let at = SeqState::swing_at(at, k, p[SC_SWING], step_frames);
        let len = step_frames * p[SC_GATE] as f64;
        let mut order: [usize; 6] = [0, 1, 2, 3, 4, 5];
        order[..voices].sort_by_key(|&j| vs[j].0);
        for (r, &j) in order.iter().enumerate().take(voices) {
            let (d, loud) = vs[j];
            if loud < 0.06 {
                continue;
            }
            let when = match play {
                1 => at + (r as f64 * step_frames / (2.0 * voices as f64)) as u64,
                2 if (k.rem_euclid(voices as i64)) as usize != r => continue,
                _ => at,
            };
            let note = ctx.scale.pitch_at(d);
            self.play(ctx, when, note, p[SC_VELOCITY] * loud, len, out);
        }
        self.visual[0] = (x as i32 as u32) as u64 | (voices as u64) << 32 | (n as u64 & 0xff) << 40;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::MidiFxKind;
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};
    use crate::sequencers::tests::run;

    #[test]
    fn every_systems_sequencer_plays() {
        let c = Scale::new(0, ScaleKind::Major);
        for kind in [MidiFxKind::Bells, MidiFxKind::Canon, MidiFxKind::Row, MidiFxKind::Gears, MidiFxKind::Formula, MidiFxKind::Staircase] {
            let ons = run(kind, &defaults(kind.specs()), &kind.default_data(), 16.0);
            assert!(ons.len() >= 8, "{kind:?}: {} notes", ons.len());
            assert!(ons.iter().all(|o| (24..=110).contains(&o.1)), "{kind:?}");
            // In the key, but the twelve-tone row.
            if kind != MidiFxKind::Row {
                assert!(ons.iter().all(|o| c.contains(o.1)), "{kind:?}: {ons:?}");
            }
            let mut d: Vec<u8> = ons.iter().map(|o| o.1).collect();
            d.sort();
            d.dedup();
            assert!(d.len() >= 3, "{kind:?}: {d:?}");
        }
    }

    /// Rows rung from rounds until rounds come back (None: not within `max`).
    fn course(method: usize, n: usize, call: u8, max: usize) -> Option<usize> {
        let w = working_bells(method, n);
        let mut row: Vec<u8> = (0..n as u8).collect();
        let mut c = 0;
        for rows in 1..=max {
            apply_change(&mut row, places(method, w, c, call), w);
            assert!({
                let mut s = row.clone();
                s.sort();
                s == (0..n as u8).collect::<Vec<_>>()
            });
            c = (c + 1) % lead_len(method, w);
            if row.iter().enumerate().all(|(i, &x)| x as usize == i) {
                return Some(rows);
            }
        }
        None
    }

    #[test]
    fn bells_methods_come_back_to_rounds() {
        // The plain courses, as rung in a tower.
        assert_eq!(course(1, 6, 0, 1000), Some(12), "Plain Hunt Minor");
        assert_eq!(course(2, 5, 0, 1000), Some(40), "Plain Bob Doubles");
        assert_eq!(course(2, 6, 0, 1000), Some(60), "Plain Bob Minor");
        assert_eq!(course(2, 8, 0, 1000), Some(112), "Plain Bob Major");
        assert_eq!(course(3, 5, 0, 1000), Some(30), "Grandsire Doubles");
        assert_eq!(course(3, 8, 0, 1000), Some(70), "Grandsire Triples, the tenor covering");
        assert_eq!(course(4, 5, 0, 1000), Some(60), "Stedman Doubles");
        assert_eq!(course(4, 7, 0, 1000), Some(84), "Stedman Triples");
        // With a call at every lead end they come round too.
        for method in 1..5 {
            for n in 4..=8 {
                for call in [1, 2] {
                    assert!(course(bell_method(method, n), n, call, 5040).is_some(), "{method} on {n}, call {call}");
                }
            }
        }
        // The sequencer: rounds first (the scale down from the treble), every bell once a row.
        let mut p = defaults(BELLS);
        p[BE_GAP] = 0.0;
        p[BE_LOOSE] = 0.0;
        let ons = run(MidiFxKind::Bells, &p, &[], 32.0);
        let first: Vec<u8> = ons.iter().take(6).map(|o| o.1).collect();
        assert_eq!(first, vec![69, 67, 65, 64, 62, 60]);
        for row in ons.chunks_exact(6).take(10) {
            let mut r: Vec<u8> = row.iter().map(|o| o.1).collect();
            r.sort();
            assert_eq!(r, vec![60, 62, 64, 65, 67, 69]);
        }
        assert!(ons.chunks(6).skip(2).take(4).any(|row| row[0].1 != 69), "it goes into changes");
    }

    #[test]
    fn canon_voices_follow_the_leader() {
        let mut p = defaults(CANON);
        p[CA_VOICES] = 1.0;
        p[CA_DELAY] = 4.0;
        p[CA_INTERVAL] = -14.0;
        p[CA_FORM] = 0.0;
        p[CA_SPEED] = 1.0;
        let ons = run(MidiFxKind::Canon, &p, &[], 8.0);
        // The follower: four eighths (two beats) later, two octaves lower.
        let leader: Vec<(f64, u8)> = ons.iter().filter(|o| o.1 >= 56).copied().collect();
        let follower: Vec<(f64, u8)> = ons.iter().filter(|o| o.1 < 56).copied().collect();
        assert!(follower.len() >= 4, "{ons:?}");
        assert!(follower.iter().all(|f| f.0 >= 2.0 - 1e-3), "it comes in after its delay");
        for f in &follower {
            assert!(leader.iter().any(|l| (l.0 + 2.0 - f.0).abs() < 1e-3 && l.1 == f.1 + 24), "{f:?}");
        }
        // Upside down mirrors the phrase round its first note; backward plays it from its end.
        let ph = phrase(0, 8, 3);
        for i in 0..8 {
            assert_eq!(canon_degree(&ph, 8, i, 1), ph[i as usize].map(|d| -(d as i32)));
            assert_eq!(canon_degree(&ph, 8, i, 2), ph[7 - i as usize].map(|d| d as i32));
        }
        // Played: each key answered later by the voices.
        use crate::midi_fx::{MidiEvent, MidiFxState};
        let mut p = defaults(CANON);
        p[CA_SOURCE] = 1.0;
        p[CA_VOICES] = 2.0;
        let mut st = MidiFxState::new(MidiFxKind::Canon);
        let spb = 24000.0;
        let mut ons = Vec::new();
        for b in 0..(8.0 * spb / 256.0) as u64 {
            let now = b * 256;
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / spb, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 5] };
            let input = if b == 0 { vec![MidiEvent::on(0, 64, 0.8)] } else { Vec::new() };
            let mut out = Vec::new();
            st.process(MidiFxKind::Canon, &p, &[], &input, &mut out, &ctx);
            ons.extend(out.iter().filter(|e| e.on).map(|e| ((now + e.offset as u64) as f64 / spb, e.pitch)));
        }
        // The key itself, an octave below two beats later, a fifth above four beats later.
        assert_eq!(ons.len(), 3, "{ons:?}");
        assert_eq!(ons[0], (0.0, 64));
        assert!((ons[1].0 - 2.0).abs() < 1e-3 && ons[1].1 == 52, "{ons:?}");
        assert!((ons[2].0 - 4.0).abs() < 1e-3 && ons[2].1 == 71, "{ons:?}");
    }

    #[test]
    fn row_forms_are_permutations() {
        for kind in 0..ROWS.len() {
            for m in [5, 7, 12] {
                let prime = prime_row(kind, 9, m);
                for form in 0..4 {
                    for t in 0..m {
                        let mut r = row_form(&prime, m, form, t)[..m].to_vec();
                        r.sort();
                        assert_eq!(r, (0..m as u8).collect::<Vec<_>>(), "{kind} {m} {form} {t}");
                    }
                }
                let p0 = row_form(&prime, m, 0, 0);
                let r0 = row_form(&prime, m, 1, 0);
                let i0 = row_form(&prime, m, 2, 0);
                for i in 0..m {
                    assert_eq!(r0[i], p0[m - 1 - i], "R is P backwards");
                    // I: the intervals turned over, from the same first note.
                    assert_eq!((p0[i] as usize + i0[i] as usize) % m, (2 * p0[0] as usize) % m);
                }
            }
        }
        // The wedge has every interval once.
        let w = prime_row(1, 0, 12);
        let mut iv: Vec<i32> = w.windows(2).map(|x| (x[1] as i32 - x[0] as i32).rem_euclid(12)).collect();
        iv.sort();
        assert_eq!(iv, (1..12).collect::<Vec<_>>());
        // A line of twelve even notes: every pitch class once in each round.
        let mut p = defaults(ROW);
        p[RO_RHYTHM] = 0.0;
        let ons = run(MidiFxKind::Row, &p, &[], 12.0);
        for round in ons.chunks(12).take(2) {
            let mut pcs: Vec<u8> = round.iter().map(|o| o.1 % 12).collect();
            pcs.sort();
            assert_eq!(pcs, (0..12).collect::<Vec<_>>());
        }
    }

    #[test]
    fn gears_make_polyrhythms_that_meet_again() {
        let mut p = defaults(GEARS);
        p[GE_GEARS] = 2.0;
        p[GE_TEETH] = 4.0;
        p[GE_TEETH + 1] = 3.0;
        p[GE_PEGS] = 1.0;
        p[GE_PEGS + 1] = 1.0;
        p[GE_NOTE] = 0.0;
        p[GE_NOTE + 1] = 4.0;
        // Twelve sixteenths: four turns of the small gear, three of the big one.
        let ons = run(MidiFxKind::Gears, &p, &[], 3.0);
        assert_eq!(ons.iter().filter(|o| o.1 == 55).count(), 3);
        assert_eq!(ons.iter().filter(|o| o.1 == 62).count(), 4);
        let together: Vec<f64> = ons.iter().filter(|o| o.1 == 55).map(|o| o.0).filter(|t| ons.iter().any(|o| o.1 == 62 && (o.0 - t).abs() < 1e-6)).collect();
        assert_eq!(together, vec![0.0], "they meet at the start (and again after twelve)");
        assert!(gear_peg(&p, 0, 12) && gear_peg(&p, 1, 12));
    }

    #[test]
    fn formula_loops_in_key_and_range() {
        let mut p = defaults(FORMULA);
        p[FO_LOOP] = 1.0; // a bar
        let ons = run(MidiFxKind::Formula, &p, &[], 8.0);
        let bar = |b: f64| ons.iter().filter(|o| o.0 >= b - 1e-3 && o.0 < b + 4.0 - 1e-3).map(|o| (((o.0 - b) * 100.0).round() as i64, o.1)).collect::<Vec<_>>();
        assert!(!bar(0.0).is_empty());
        assert_eq!(bar(0.0), bar(4.0));
        let top = Scale::new(0, ScaleKind::Major).pitch_at(Scale::new(0, ScaleKind::Major).locate(55).0 + 9) as u8;
        assert!(ons.iter().all(|o| (55..=top).contains(&o.1)), "{ons:?}");
        // Every formula makes a melody at one Step or another.
        for f in 0..FORMULAS.len() {
            p[FO_FORMULA] = f as f32;
            let melodic = (0..FORMULA_STEPS.len()).any(|step| {
                p[FO_STEP] = step as f32;
                let notes: Vec<i32> = (0..32).filter_map(|k| formula_note(&p, formula_t(k, &p, 16))).collect();
                let mut d = notes.clone();
                d.sort();
                d.dedup();
                notes.len() > 8 && d.len() >= 3
            });
            assert!(melodic, "{}", FORMULAS[f]);
        }
    }

    #[test]
    fn staircase_climbs_for_ever() {
        let p = defaults(STAIRCASE);
        let c = Scale::new(0, ScaleKind::Major);
        let (cd, _) = c.locate(60);
        // A whole turn of the stair (four octaves of seven steps) and it is where it began.
        assert_eq!(stair_voices(&p, 3, 7, cd), stair_voices(&p, 3 + 28, 7, cd));
        // The loudest voice rises with every step but where the stair turns over.
        let mut rises = 0;
        for x in 0..28 {
            let a = stair_voices(&p, x, 7, cd);
            let b = stair_voices(&p, x + 1, 7, cd);
            let mut higher = 0;
            for j in 0..4 {
                if b[j].0 == a[j].0 + 1 {
                    higher += 1;
                }
            }
            if higher >= 3 {
                rises += 1;
            }
        }
        assert_eq!(rises, 28);
        // The edges are quiet: the voices that turn over are hardly heard.
        let v = stair_voices(&p, 0, 7, cd);
        assert!(v.iter().take(4).any(|x| x.1 < 0.1));
        assert!(v.iter().take(4).any(|x| x.1 > 0.8));
    }
}
