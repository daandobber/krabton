//! Mechanical music: what a street organ plays from its book, and what a
//! music box plays from its pinned cylinder.
//!
//! - Organ Grinder: a tune in the song's key over its own chords, in the
//!   shape of the old dance tunes (an eight-bar phrase, the same again,
//!   another, the first once more), arranged for the organ's divisions as
//!   the arrangers punch it into the book: the bass, the accompaniment's
//!   oom-pah, a counter melody under the tune, grace notes, trills and runs,
//!   and the drums on the Street Organ's drum keys. As a waltz, a march, a
//!   polka or a foxtrot.
//! - Music Box: a tune pinned on a cylinder plucking a steel comb: the
//!   melody high, a broken chord under it, only the notes its comb has,
//!   long notes repeated so they keep sounding, and a spring that runs down
//!   (the notes come later and softer and some go missing, until it stops
//!   and is wound again).
//!
//! Both turn: the organ's book is pulled through by a crank, the box's
//! cylinder by a crank or a spring. *Drive* says what turns it: the song
//! (in its tempo, with a hand's unevenness), a crank (its speed, or yours on
//! the screen: the book goes as fast as you turn, and the organ, short of
//! wind, plays softer when you turn slowly) or, for the box, its spring
//! (wound on the screen, or by Play, it runs at its own pace, slows as it
//! runs down and stops).
//!
//! Every bar is made from the parameters alone (`organ_bar`, `box_bar`), so
//! what the screens draw is what plays.

use super::minimal::{choose, chord_root, hash, CHORD_AMOUNT, DYNAMICS};
use super::{SeqState, RATES};
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit, SCREEN_ONLY};

const KEYS: [ParamSpec; 2] = [ParamSpec::toggle("Transpose", "Keys", false), ParamSpec::toggle("Hold", "Keys", false)];

/// Each step (a sixteenth at the default rate) is cut in two, for grace
/// notes and trills.
pub const TICKS: f64 = 2.0;

pub const OG_STYLE: usize = 1;
pub const OG_TUNE: usize = 2;
pub const OG_COUNTER: usize = 3;
pub const OG_ORNAMENTS: usize = 4;
pub const OG_DRUMS: usize = 5;
pub const OG_BASE: usize = 6;
pub const OG_VELOCITY: usize = 7;
pub const OG_CHORD_AMOUNT: usize = 14;
pub const ORGAN_STYLES: &[&str] = &["Waltz", "March", "Polka", "Foxtrot"];
pub const OG_DRIVE: usize = 15;
pub const OG_CRANK: usize = 16;
/// How a hand turns the crank: how uneven within a turn, where it pushes,
/// how much the turns differ, how long a turn is, and whose feel.
pub const OG_SWING: usize = 17;
/// Your hand on the crank (the screen sets it while you turn): its speed,
/// negative turned backwards, 0 when let go.
pub const OG_TURNING: usize = 18;
pub const OG_PUSH: usize = 19;
pub const OG_WANDER: usize = 20;
pub const OG_TURN: usize = 21;
pub const OG_FEEL: usize = 22;
pub const ORGAN_DRIVES: &[&str] = &["Song", "Crank"];
/// A turn of the crank: a bar, half a bar or a beat (a beat a turn swings the quavers).
pub const TURNS: &[&str] = &["Bar", "Half Bar", "Beat"];
/// The feel of the turning: set by Swing and Push, or learned from yours on the screen.
pub const FEELS: &[&str] = &["Swing", "Yours"];

pub static ORGAN: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Style", "Organ", ORGAN_STYLES, 0),
    ParamSpec::int("Tune", "Organ", 1.0, 64.0, 1.0, Unit::None),
    ParamSpec::lin("Counter", "Organ", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Ornaments", "Organ", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Drums", "Organ", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::int("Base Note", "Notes", 60.0, 84.0, 72.0, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.8, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
    CHORD_AMOUNT,
    ParamSpec::choice("Drive", "Crank", ORGAN_DRIVES, 0),
    ParamSpec::log("Crank", "Crank", 0.25, 2.0, 1.0, Unit::Ratio),
    ParamSpec::lin("Swing", "Crank", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Turning", SCREEN_ONLY, -4.0, 4.0, 0.0, Unit::Ratio),
    ParamSpec::int("Push", "Crank", 0.0, 360.0, 90.0, Unit::None),
    ParamSpec::lin("Wander", "Crank", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::choice("Turn", "Crank", TURNS, 0),
    ParamSpec::choice("Feel", "Crank", FEELS, 0),
];

pub const MB_METRE: usize = 1;
pub const MB_TUNE: usize = 2;
pub const MB_ACCOMP: usize = 3;
pub const MB_COMB: usize = 4;
pub const MB_SPARKLE: usize = 5;
pub const MB_WIND: usize = 6;
pub const MB_BASE: usize = 7;
pub const MB_VELOCITY: usize = 8;
pub const MB_CHORD_AMOUNT: usize = 15;
pub const BOX_METRES: &[&str] = &["3/4", "4/4"];
pub const BOX_ACCOMPS: &[&str] = &["Waltz", "Arpeggio", "Alberti"];
pub const BOX_COMBS: &[&str] = &["18 Notes", "30 Notes", "Full"];
pub const MB_DRIVE: usize = 16;
pub const MB_CRANK: usize = 17;
pub const MB_SWING: usize = 18;
pub const MB_SPRING: usize = 19;
pub const MB_TURNING: usize = 20;
/// How many turns the key has been wound (the screen counts them up).
pub const MB_WOUND: usize = 21;
pub const MB_PUSH: usize = 22;
pub const MB_WANDER: usize = 23;
pub const MB_TURN: usize = 24;
pub const MB_FEEL: usize = 25;
pub const BOX_DRIVES: &[&str] = &["Song", "Spring", "Crank"];

pub static MUSIC_BOX: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Metre", "Box", BOX_METRES, 0),
    ParamSpec::int("Tune", "Box", 1.0, 64.0, 1.0, Unit::None),
    ParamSpec::choice("Accomp", "Box", BOX_ACCOMPS, 0),
    ParamSpec::choice("Comb", "Box", BOX_COMBS, 1),
    ParamSpec::lin("Sparkle", "Box", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Wind", "Box", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::int("Base Note", "Notes", 60.0, 84.0, 72.0, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
    CHORD_AMOUNT,
    ParamSpec::choice("Drive", "Turn", BOX_DRIVES, 0),
    ParamSpec::log("Crank", "Turn", 0.25, 2.0, 1.0, Unit::Ratio),
    ParamSpec::lin("Swing", "Turn", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::log("Spring", "Turn", 5.0, 180.0, 40.0, Unit::Sec),
    ParamSpec::lin("Turning", SCREEN_ONLY, -4.0, 4.0, 0.0, Unit::Ratio),
    ParamSpec::lin("Wound", SCREEN_ONLY, 0.0, 1_000_000.0, 0.0, Unit::None),
    ParamSpec::int("Push", "Turn", 0.0, 360.0, 90.0, Unit::None),
    ParamSpec::lin("Wander", "Turn", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::choice("Turn", "Turn", TURNS, 0),
    ParamSpec::choice("Feel", "Turn", FEELS, 0),
];

/// What a hole in the book (or a pin on the cylinder) plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lane {
    Melody,
    Counter,
    Accomp,
    Bass,
    /// `deg` is the Street Organ's drum key.
    Drum,
}

/// A note of a bar: where it starts and how long it is (ticks), what plays
/// it, its degree of the key from the tonic (or its drum key) and how hard.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hole {
    pub at: u16,
    pub len: u16,
    pub lane: Lane,
    pub deg: i32,
    pub vel: f32,
}

/// Chord roots of the phrases (degrees from the tonic): the A phrases come
/// home, the B phrases end on the dominant to lead back.
const A_ROOTS: [[i32; 8]; 4] = [[0, 0, 3, 0, 4, 4, 0, 0], [0, 0, 4, 4, 4, 4, 0, 0], [0, 3, 0, 4, 0, 3, 4, 0], [0, 5, 3, 4, 0, 5, 4, 0]];
const B_ROOTS: [[i32; 8]; 3] = [[3, 3, 0, 0, 1, 4, 4, 4], [5, 5, 1, 4, 0, 0, 4, 4], [3, 0, 3, 0, 1, 4, 1, 4]];
/// Where the tune heads at each bar's start (degrees from the tonic).
const A_ARCS: [[i32; 8]; 4] = [[2, 4, 5, 4, 2, 4, 1, 0], [4, 2, 4, 7, 5, 4, 1, 0], [0, 2, 4, 5, 7, 5, 2, 0], [7, 5, 4, 2, 4, 5, 1, 0]];
const B_ARCS: [[i32; 8]; 3] = [[5, 7, 4, 5, 3, 4, 6, 4], [3, 5, 7, 5, 8, 7, 5, 4], [5, 3, 4, 2, 3, 1, 2, 1]];
/// The melody's rhythms (in sixteenths: start, length): a bar of 3/4, a bar of 2/4.
const WALTZ_FIGS: [&[(u8, u8)]; 6] = [&[(0, 8), (8, 4)], &[(0, 4), (4, 4), (8, 4)], &[(0, 6), (6, 2), (8, 4)], &[(0, 12)], &[(0, 4), (4, 2), (6, 2), (8, 4)], &[(0, 2), (2, 2), (4, 4), (8, 4)]];
const DUPLE_FIGS: [&[(u8, u8)]; 6] = [&[(0, 4), (4, 4)], &[(0, 3), (3, 1), (4, 4)], &[(0, 2), (2, 2), (4, 2), (6, 2)], &[(0, 8)], &[(0, 2), (2, 2), (4, 4)], &[(0, 3), (3, 1), (4, 2), (6, 2)]];
/// Which rhythm each bar of a phrase takes: the first returns, the seventh
/// runs on, the last is the long note home.
const ROLES: [usize; 8] = [0, 1, 0, 2, 0, 1, 3, 4];

/// The nearest degree to `x` that is in the chord on `root`.
fn chord_tone(root: i32, x: i32) -> i32 {
    for d in 0..4 {
        for s in [x - d, x + d] {
            if [0, 2, 4].contains(&(s - root).rem_euclid(7)) {
                return s;
            }
        }
    }
    x
}

/// A phrase's chords and contour: (roots, arcs) for tune `tune`.
fn phrase(tune: u32, b_section: bool) -> ([i32; 8], [i32; 8]) {
    if b_section {
        (B_ROOTS[(hash(tune, 11) * 3.0) as usize % 3], B_ARCS[(hash(tune, 12) * 3.0) as usize % 3])
    } else {
        (A_ROOTS[(hash(tune, 13) * 4.0) as usize % 4], A_ARCS[(hash(tune, 14) * 4.0) as usize % 4])
    }
}

/// The rhythm of bar role `role` (in sixteenths), `steps` long.
fn figure(tune: u32, sec: u32, role: usize, steps: u16) -> Vec<(u8, u8)> {
    let pick = |salt: u32| -> usize { (hash(tune.wrapping_add(31), sec * 16 + role as u32 * 2 + salt) * 6.0) as usize % 6 };
    let long = role == 4;
    match steps {
        12 => (if long { WALTZ_FIGS[3] } else { WALTZ_FIGS[pick(0)] }).to_vec(),
        8 => (if long { DUPLE_FIGS[3] } else { DUPLE_FIGS[pick(0)] }).to_vec(),
        _ => {
            let a = DUPLE_FIGS[pick(0)];
            let b = if long { &[(0u8, 8u8)][..] } else { DUPLE_FIGS[pick(1)] };
            a.iter().copied().chain(b.iter().map(|&(s, l)| (s + 8, l))).collect()
        }
    }
}

/// The melody of bar `b` (0..8) of a phrase, into `out`; its notes kept
/// between `lo` and `hi` degrees, shifted by `lift`. Ornaments add grace
/// notes, trills and a run into the cadence.
#[allow(clippy::too_many_arguments)]
fn melody(tune: u32, sec: u32, b: usize, steps: u16, roots: &[i32; 8], arcs: &[i32; 8], ornaments: f32, lo: i32, hi: i32, lift: i32, vel: f32, out: &mut Vec<Hole>) {
    let root = roots[b];
    let target = arcs[(b + 1) % 8];
    let mut cur = chord_tone(root, arcs[b]).clamp(lo, hi);
    let fig = figure(tune, sec, ROLES[b], steps);
    // The run into the cadence: sixteenths stepping towards home.
    if ROLES[b] == 3 && ornaments > 0.55 && hash(tune.wrapping_add(41), sec * 8 + b as u32) < ornaments {
        let n = steps as i32;
        let dir = if target >= cur { 1 } else { -1 };
        for s in 0..n {
            out.push(Hole { at: (s * 2) as u16, len: 2, lane: Lane::Melody, deg: (cur + lift).clamp(lo + lift, hi + lift), vel: vel * if s % 4 == 0 { 0.95 } else { 0.8 } });
            if s % 4 == 3 && (cur - target).abs() <= 1 {
                cur = chord_tone(root, cur);
            } else {
                cur = (cur + dir).clamp(lo, hi);
            }
        }
        return;
    }
    for (j, &(st, len)) in fig.iter().enumerate() {
        if j > 0 {
            let strong = st % 4 == 0;
            let dir = (target - cur).signum();
            let u = hash(tune.wrapping_add(7), sec * 1024 + b as u32 * 64 + j as u32);
            let step = if dir == 0 { if u < 0.5 { 1 } else { -1 } } else if u < 0.75 { dir } else if u < 0.88 { 0 } else { -dir };
            cur = if strong { chord_tone(root, cur + step) } else { cur + step }.clamp(lo, hi);
        }
        let at = st as u16 * 2;
        let ln = len as u16 * 2;
        let accent = if st == 0 { 1.0 } else if st % 4 == 0 { 0.92 } else { 0.85 };
        let d = cur + lift;
        let o = hash(tune.wrapping_add(53), sec * 1024 + b as u32 * 64 + j as u32);
        if len >= 4 && o < ornaments * 0.45 {
            if o < ornaments * 0.2 {
                // A grace note from above.
                out.push(Hole { at, len: 1, lane: Lane::Melody, deg: d + 1, vel: vel * accent * 0.8 });
                out.push(Hole { at: at + 1, len: ln - 1, lane: Lane::Melody, deg: d, vel: vel * accent });
            } else {
                // A trill on the upper note over the first half, then the note.
                let half = ln / 2;
                for i in 0..half {
                    out.push(Hole { at: at + i, len: 1, lane: Lane::Melody, deg: if i % 2 == 0 { d + 1 } else { d }, vel: vel * accent * 0.85 });
                }
                out.push(Hole { at: at + half, len: ln - half, lane: Lane::Melody, deg: d, vel: vel * accent });
            }
        } else {
            out.push(Hole { at, len: ln, lane: Lane::Melody, deg: d, vel: vel * accent });
        }
    }
}

/// How long a bar of the Organ Grinder is (ticks).
pub fn organ_ticks_per_bar(p: &[f32]) -> u16 {
    [24, 16, 16, 32][choose(p, OG_STYLE, ORGAN_STYLES.len())]
}

/// How long a bar of the Music Box is (ticks).
pub fn box_ticks_per_bar(p: &[f32]) -> u16 {
    [24, 32][choose(p, MB_METRE, BOX_METRES.len())]
}

/// The tune's number (a seed).
fn tune_of(p: &[f32], i: usize) -> u32 {
    p.get(i).copied().unwrap_or(1.0).round().clamp(0.0, 1000.0) as u32
}

/// Bar `bar` of the Organ Grinder's book, into `out` (cleared first); its chord root.
pub fn organ_bar(p: &[f32], bar: i64, out: &mut Vec<Hole>) -> i32 {
    out.clear();
    let style = choose(p, OG_STYLE, ORGAN_STYLES.len());
    let tune = tune_of(p, OG_TUNE);
    let tpb = organ_ticks_per_bar(p);
    let steps = tpb / 2;
    let counter = p.get(OG_COUNTER).copied().unwrap_or(0.5).clamp(0.0, 1.0);
    let ornaments = p.get(OG_ORNAMENTS).copied().unwrap_or(0.4).clamp(0.0, 1.0);
    let drums = p.get(OG_DRUMS).copied().unwrap_or(0.7).clamp(0.0, 1.0);
    let chords = p.get(OG_CHORD_AMOUNT).copied().unwrap_or(1.0).clamp(0.0, 2.0);
    // A A B A: thirty-two bars round.
    let cycle = bar.rem_euclid(32);
    let ph = (cycle / 8) as u32;
    let b = (cycle % 8) as usize;
    let b_section = ph == 2;
    let (roots, arcs) = phrase(tune, b_section);
    let root = chord_root(chords, bar, roots[b], roots[(b + 1) % 8], false);
    let roots_now = {
        let mut r = roots;
        r[b] = root;
        r
    };
    let sec = if b_section { 1 } else { 0 };
    let start = out.len();
    melody(tune, sec, b, steps, &roots_now, &arcs, ornaments, -3, 11, 0, if b_section { 0.85 } else { 0.95 }, out);
    // The counter melody, a third (or the nearest chord note) under the long notes.
    let c_amount = (counter * if b_section { 1.3 } else { 1.0 }).min(1.0);
    let tune_notes: Vec<Hole> = out[start..].to_vec();
    for (j, h) in tune_notes.iter().enumerate() {
        // Keyed to the place in the phrase, so a phrase played again is the same.
        if h.len >= 8 && hash(tune.wrapping_add(61), (sec * 8 + b as u32) * 64 + j as u32) < c_amount {
            let mut c = chord_tone(root, h.deg - 2);
            if c >= h.deg {
                c -= 2;
            }
            out.push(Hole { at: h.at, len: h.len, lane: Lane::Counter, deg: c, vel: 0.62 });
        }
    }
    // The bass, alternating with the fifth below, and the accompaniment's chords.
    let fifth = root - 3;
    let alt = |odd: bool| if odd { fifth } else { root };
    let chord = |out: &mut Vec<Hole>, at: u16, len: u16| {
        for d in [root, root + 2, root + 4] {
            out.push(Hole { at, len, lane: Lane::Accomp, deg: d, vel: 0.55 });
        }
    };
    match style {
        0 => {
            out.push(Hole { at: 0, len: 14, lane: Lane::Bass, deg: alt(bar % 2 == 1), vel: 0.8 });
            chord(out, 8, 6);
            chord(out, 16, 6);
        }
        1 => {
            out.push(Hole { at: 0, len: 6, lane: Lane::Bass, deg: alt(bar % 2 == 1), vel: 0.85 });
            chord(out, 8, 6);
        }
        2 => {
            out.push(Hole { at: 0, len: 4, lane: Lane::Bass, deg: root, vel: 0.85 });
            chord(out, 4, 3);
            out.push(Hole { at: 8, len: 4, lane: Lane::Bass, deg: fifth, vel: 0.75 });
            chord(out, 12, 3);
        }
        _ => {
            out.push(Hole { at: 0, len: 8, lane: Lane::Bass, deg: root, vel: 0.85 });
            chord(out, 8, 6);
            out.push(Hole { at: 16, len: 8, lane: Lane::Bass, deg: fifth, vel: 0.75 });
            chord(out, 24, 6);
        }
    }
    // The drums: the bass drum (with its cymbal at the big moments), the snare, the woodblock.
    if drums > 0.0 {
        let hit = |out: &mut Vec<Hole>, at: u16, key: i32, v: f32| out.push(Hole { at, len: 2, lane: Lane::Drum, deg: key, vel: v * (0.45 + 0.55 * drums) });
        let big = bar % 4 == 0;
        match style {
            0 => {
                hit(out, 0, if bar % 2 == 0 { 24 } else { 25 }, 0.9);
                if drums > 0.5 {
                    hit(out, 8, 29, 0.45);
                    hit(out, 16, 29, 0.45);
                }
            }
            1 => {
                hit(out, 0, if big || drums > 0.8 { 24 } else { 25 }, 0.9);
                hit(out, 8, 26, 0.7);
            }
            2 => {
                hit(out, 0, if big { 24 } else { 25 }, 0.85);
                hit(out, 4, 26, 0.6);
                hit(out, 12, 26, 0.6);
            }
            _ => {
                hit(out, 0, if big { 24 } else { 25 }, 0.9);
                hit(out, 8, 26, 0.7);
                hit(out, 16, 25, 0.8);
                hit(out, 24, 26, 0.7);
            }
        }
        // A roll on the snare into the next phrase.
        if b == 7 && drums > 0.3 {
            out.retain(|h| !(h.lane == Lane::Drum && h.deg == 26 && h.at >= tpb / 2));
            out.push(Hole { at: tpb / 2, len: tpb / 2 - 1, lane: Lane::Drum, deg: 27, vel: 0.6 * drums });
        }
    }
    root
}

/// Bar `bar` of the Music Box's cylinder, into `out` (cleared first); its chord root.
pub fn box_bar(p: &[f32], bar: i64, out: &mut Vec<Hole>) -> i32 {
    out.clear();
    let tune = tune_of(p, MB_TUNE).wrapping_add(1000);
    let tpb = box_ticks_per_bar(p);
    let steps = tpb / 2;
    let accomp = choose(p, MB_ACCOMP, BOX_ACCOMPS.len());
    let comb = choose(p, MB_COMB, BOX_COMBS.len());
    let sparkle = p.get(MB_SPARKLE).copied().unwrap_or(0.35).clamp(0.0, 1.0);
    let chords = p.get(MB_CHORD_AMOUNT).copied().unwrap_or(1.0).clamp(0.0, 2.0);
    // The comb's teeth: its lowest and highest degree around the base note's tonic.
    let (lo, hi) = [(-7, 10), (-11, 17), (-14, 21)][comb];
    // A then B: sixteen bars round, and the cylinder starts again.
    let cycle = bar.rem_euclid(16);
    let b_section = cycle >= 8;
    let b = (cycle % 8) as usize;
    let (roots, arcs) = phrase(tune, b_section);
    let root = chord_root(chords, bar, roots[b], roots[(b + 1) % 8], false);
    let mut roots_now = roots;
    roots_now[b] = root;
    let sec = if b_section { 1 } else { 0 };
    let start = out.len();
    // The tune high on the comb.
    let top = hi - 1;
    melody(tune, sec, b, steps, &roots_now, &arcs, sparkle * 0.5, (top - 10).max(lo + 4), top, 4, 0.9, out);
    // Long notes keep sounding by being plucked again; some doubled an octave up.
    let tune_notes: Vec<Hole> = out[start..].to_vec();
    for (j, h) in tune_notes.iter().enumerate() {
        let u = hash(tune.wrapping_add(71), (sec * 8 + b as u32) * 64 + j as u32);
        if h.len >= 12 && u < sparkle {
            let mut at = h.at + 4;
            while at < h.at + h.len {
                out.push(Hole { at, len: 4, lane: Lane::Melody, deg: h.deg, vel: h.vel * 0.55 });
                at += 4;
            }
        }
        if h.at % 8 == 0 && h.deg + 7 <= hi && u > 1.0 - sparkle * 0.3 {
            out.push(Hole { at: h.at, len: h.len, lane: Lane::Counter, deg: h.deg + 7, vel: h.vel * 0.5 });
        }
    }
    // The broken chord low on the comb.
    let fold = |d: i32| {
        let mut d = d;
        while d < lo {
            d += 7;
        }
        while d > lo + 9 {
            d -= 7;
        }
        d
    };
    let r = fold(root);
    let tone = |k: i32| fold(root + k);
    match accomp {
        0 => {
            out.push(Hole { at: 0, len: 8, lane: Lane::Bass, deg: r, vel: 0.62 });
            let mut beat = 8;
            while beat < tpb {
                out.push(Hole { at: beat, len: 6, lane: Lane::Accomp, deg: tone(2), vel: 0.48 });
                out.push(Hole { at: beat, len: 6, lane: Lane::Accomp, deg: tone(4), vel: 0.45 });
                beat += 8;
            }
        }
        1 => {
            const UP_DOWN: [i32; 6] = [0, 2, 4, 7, 4, 2];
            for (i, at) in (0..tpb).step_by(4).enumerate() {
                let k = UP_DOWN[i % 6];
                let d = if k == 7 { r + 7 } else { tone(k) };
                out.push(Hole { at, len: 4, lane: if i == 0 { Lane::Bass } else { Lane::Accomp }, deg: d, vel: if i == 0 { 0.6 } else { 0.45 } });
            }
        }
        _ => {
            const ALBERTI: [i32; 4] = [0, 4, 2, 4];
            for (i, at) in (0..tpb).step_by(4).enumerate() {
                out.push(Hole { at, len: 4, lane: if i == 0 { Lane::Bass } else { Lane::Accomp }, deg: tone(ALBERTI[i % 4]), vel: if i % 4 == 0 { 0.55 } else { 0.42 } });
            }
        }
    }
    // Nothing the comb lacks.
    for h in out.iter_mut() {
        while h.deg > hi {
            h.deg -= 7;
        }
        while h.deg < lo {
            h.deg += 7;
        }
    }
    root
}

/// The bar the mechanical sequencers last made, kept until the next one.
pub struct Mechanical {
    bar: i64,
    key: u64,
    root: i32,
    holes: Vec<Hole>,
    hist: u64,
    chords: u64,
    /// Turned by a crank or a spring: where the book or cylinder is (ticks),
    /// how fast it goes (1: the song's tempo), how its drive was.
    pos: f64,
    speed: f32,
    drive: usize,
    /// The spring: how wound (0..1), the winding counted last, whether the song played.
    tension: f32,
    wound: f32,
    was_playing: bool,
}

impl Default for Mechanical {
    fn default() -> Self {
        Self { bar: i64::MIN, key: 0, root: 0, holes: Vec::with_capacity(160), hist: 0, chords: 0, pos: 0.0, speed: 0.0, drive: usize::MAX, tension: 0.0, wound: f32::NAN, was_playing: false }
    }
}

/// What the bar depends on, so a turned knob makes it again.
fn fingerprint(which: u64, p: &[f32], chords: usize) -> u64 {
    p.iter().take(8).chain(p.get(chords)).fold(which, |h, v| h.wrapping_mul(31).wrapping_add((v * 1000.0) as i64 as u64))
}

/// How many parts of a turn a learned feel has.
pub const FEEL_BINS: usize = 16;

/// A learned feel as a device keeps it: a tag, then a byte for each part of the turn.
pub fn encode_feel(bins: &[f32; FEEL_BINS]) -> Vec<u8> {
    let mean = bins.iter().sum::<f32>() / FEEL_BINS as f32;
    let mut v = vec![b'F'];
    v.extend(bins.iter().map(|b| (((b / mean.max(1e-3)) - 0.25) / 1.75 * 255.0).round().clamp(0.0, 255.0) as u8));
    v
}

/// A learned feel back from a device's data (its speed through a turn, 1 on average).
pub fn decode_feel(data: &[u8]) -> Option<[f32; FEEL_BINS]> {
    if data.len() < FEEL_BINS + 1 || data[0] != b'F' {
        return None;
    }
    let mut b = [0.0f32; FEEL_BINS];
    for (i, x) in b.iter_mut().enumerate() {
        *x = 0.25 + data[1 + i] as f32 / 255.0 * 1.75;
    }
    let mean = b.iter().sum::<f32>() / FEEL_BINS as f32;
    for x in b.iter_mut() {
        *x /= mean.max(1e-3);
    }
    Some(b)
}

/// How long a turn of the crank is (ticks).
pub fn ticks_per_turn(p: &[f32], organ: bool) -> f64 {
    let tpb = if organ { organ_ticks_per_bar(p) } else { box_ticks_per_bar(p) } as f64;
    let tick_beats = super::rate_beats(p[0]) / TICKS;
    let turn = p.get(if organ { OG_TURN } else { MB_TURN }).copied().unwrap_or(0.0).round().max(0.0) as usize;
    match turn {
        0 => tpb,
        1 => tpb / 2.0,
        _ => 1.0 / tick_beats,
    }
}

/// How a hand turns the crank: its speed through a turn (1 on average),
/// set by Swing (how uneven) and Push (where in the turn it pushes), or
/// learned from yours; Wander makes every turn a little different.
pub struct Feel {
    bins: Option<[f32; FEEL_BINS]>,
    swing: f64,
    push: f64,
    wander: f64,
}

impl Feel {
    pub fn of(p: &[f32], data: &[u8], organ: bool) -> Self {
        let (si, pi, wi, fi) = if organ { (OG_SWING, OG_PUSH, OG_WANDER, OG_FEEL) } else { (MB_SWING, MB_PUSH, MB_WANDER, MB_FEEL) };
        let get = |i: usize, or: f32| p.get(i).copied().unwrap_or(or);
        let bins = if get(fi, 0.0) >= 0.5 { decode_feel(data) } else { None };
        Feel { bins, swing: get(si, 0.0).clamp(0.0, 1.0) as f64, push: (get(pi, 90.0) as f64 / 360.0).rem_euclid(1.0), wander: get(wi, 0.0).clamp(0.0, 1.0) as f64 }
    }

    /// Whether it is your learned feel.
    pub fn learned(&self) -> bool {
        self.bins.is_some()
    }

    /// How uneven turn `turn` is, against the others.
    fn depth(&self, turn: i64) -> f64 {
        1.0 + self.wander * 1.2 * (hash(0xFE11, turn as u32) as f64 - 0.5)
    }

    /// How fast a whole turn goes against the others (with Wander).
    pub fn tempo(&self, turn: i64) -> f64 {
        1.0 + self.wander * 0.08 * (hash(0xFE12, turn as u32) as f64 - 0.5)
    }

    /// The speed at `phase` (0..1) of turn `turn` (1 an even hand).
    pub fn speed(&self, phase: f64, turn: i64) -> f64 {
        let d = self.depth(turn);
        match &self.bins {
            Some(b) => (1.0 + d * (b[((phase.rem_euclid(1.0) * FEEL_BINS as f64) as usize).min(FEEL_BINS - 1)] as f64 - 1.0)).max(0.2),
            None => {
                let a = self.swing * 0.45 * d;
                let x = std::f64::consts::TAU * (phase - self.push);
                (1.0 + a * (x.sin() + 0.3 * (2.0 * x).sin())).max(0.1)
            }
        }
    }

    /// How far ahead of an even hand the book is at `phase` of turn `turn`
    /// (in turns): none at a turn's start or end, so every turn comes round on time.
    pub fn offset(&self, phase: f64, turn: i64) -> f64 {
        let d = self.depth(turn);
        let ph = phase.clamp(0.0, 1.0);
        match &self.bins {
            Some(b) => {
                let x = ph * FEEL_BINS as f64;
                let i = (x as usize).min(FEEL_BINS - 1);
                let mut sum: f64 = b[..i].iter().map(|v| *v as f64 - 1.0).sum();
                sum += (b[i] as f64 - 1.0) * (x - i as f64);
                (d * sum / FEEL_BINS as f64).max(-ph + 1e-6)
            }
            None => {
                let a = self.swing * 0.45 * d;
                let w = std::f64::consts::TAU;
                let p0 = self.push;
                a * (((-w * p0).cos() - (w * (ph - p0)).cos()) / w + 0.3 * ((-2.0 * w * p0).cos() - (2.0 * w * (ph - p0)).cos()) / (2.0 * w))
            }
        }
    }

    /// Where the book is when an even clock says `pos` (ticks), turned by this hand.
    pub fn book(&self, pos: f64, ticks_per_turn: f64) -> f64 {
        let t = pos / ticks_per_turn;
        let n = t.floor();
        pos + self.offset(t - n, n as i64) * ticks_per_turn
    }
}

/// How fast a spring this wound turns the cylinder: steady, then slowing
/// as it runs down, stopped when it is.
pub fn spring_speed(tension: f32) -> f32 {
    if tension <= 0.0 {
        0.0
    } else if tension > 0.25 {
        1.0 + 0.04 * (tension - 0.25)
    } else {
        (tension / 0.25).powf(0.7)
    }
}

/// A pitch in `lo..=hi`, moved by octaves.
fn fit(pitch: i32, lo: i32, hi: i32) -> i32 {
    let mut p = pitch;
    while p < lo {
        p += 12;
    }
    while p > hi {
        p -= 12;
    }
    p
}

impl SeqState {
    /// A mechanical sequencer's block: where its own clock (the song's, a
    /// crank's or a spring's) takes the book or the cylinder, and the ticks
    /// it passes on the way. A hand on the crank turns it unevenly, as its
    /// Feel says (`data` holds a feel learned from yours).
    pub(crate) fn mechanical_time(&mut self, organ: bool, p: &[f32], data: &[u8], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let which = if organ { 1 } else { 2 };
        let tick_beats = super::rate_beats(p[0]) / TICKS;
        let tick_frames = tick_beats * ctx.samples_per_beat;
        let (di, ci, ti) = if organ { (OG_DRIVE, OG_CRANK, OG_TURNING) } else { (MB_DRIVE, MB_CRANK, MB_TURNING) };
        let get = |i: usize, or: f32| p.get(i).copied().unwrap_or(or);
        let drive = (get(di, 0.0).round().max(0.0) as usize).min(if organ { 1 } else { 2 });
        // Your hand: its speed, backwards below 0.
        let turning = get(ti, 0.0).clamp(-4.0, 4.0);
        let held = turning.abs() > 0.01;
        let feel = Feel::of(p, data, organ);
        let tpt = ticks_per_turn(p, organ).max(1.0);
        let dt = ctx.len as f32 / ctx.sr.max(1.0);
        let song_pos = ctx.beat / tick_beats;
        let started = ctx.playing && !self.mechanical.was_playing;
        self.mechanical.was_playing = ctx.playing;
        let m = &mut self.mechanical;
        if m.drive != drive {
            // Taken over by another drive: it carries on from where the song is.
            m.pos = song_pos;
            m.speed = if drive == 0 { 1.0 } else { 0.0 };
            m.drive = drive;
        }
        let (pos0, pos1, breath) = if drive == 0 {
            if !ctx.playing {
                return;
            }
            let end = song_pos + ctx.len as f64 / tick_frames;
            m.speed = 1.0;
            // The organ's book under a hand in the song's time: uneven within
            // every turn, on time at the end of each (the box has a governor).
            if organ {
                (feel.book(song_pos, tpt), feel.book(end, tpt), 1.0)
            } else {
                (song_pos, end, 1.0)
            }
        } else {
            let spring = !organ && drive == 1;
            let target = if spring {
                // Wound by Play, and by your turns of the key.
                let wound = get(MB_WOUND, 0.0);
                if m.wound.is_nan() {
                    m.wound = wound;
                } else if wound > m.wound {
                    m.tension = (m.tension + 0.25 * (wound - m.wound)).min(1.0);
                    m.wound = wound;
                } else {
                    m.wound = wound;
                }
                if started {
                    m.tension = 1.0;
                }
                spring_speed(m.tension)
            } else if held {
                turning
            } else if ctx.playing {
                get(ci, 1.0).clamp(0.05, 4.0)
            } else {
                0.0
            };
            // A flywheel under the crank, a governor in the box: the speed follows, not jumps.
            let tau = if spring { 0.12 } else if held { 0.15 } else { 0.4 };
            m.speed += (target - m.speed) * (1.0 - (-dt / tau).exp());
            if m.speed.abs() < 0.003 && target == 0.0 {
                m.speed = 0.0;
            }
            if spring {
                let secs = get(MB_SPRING, 40.0).clamp(1.0, 600.0);
                m.tension = (m.tension - dt * m.speed / secs).max(0.0);
            }
            // Turned by a hand (not while yours is on it: then it is your speed): uneven within the turn.
            let turn = (m.pos / tpt).floor();
            let uneven = if spring || held { 1.0 } else { feel.speed(m.pos / tpt - turn, turn as i64) * feel.tempo(turn as i64) };
            let adv = m.speed as f64 * uneven * ctx.len as f64 / tick_frames;
            let from = m.pos;
            m.pos += adv;
            // Short of wind when the organ is turned slowly: softer.
            let breath = if organ && !spring { 0.45 + 0.55 * m.speed.abs().min(1.0) } else { 1.0 };
            (from, m.pos, breath)
        };
        let speed = self.mechanical.speed.abs().max(0.08) as f64;
        if pos1 > pos0 {
            let mut k = pos0.ceil() as i64;
            while (k as f64) < pos1 {
                let frac = (k as f64 - pos0) / (pos1 - pos0);
                let at = ctx.now + (frac * ctx.len as f64) as u64;
                self.mechanical_tick(which, k, p, at, tick_frames / speed, breath, drive == 0, false, out, ctx);
                k += 1;
            }
        } else if pos1 < pos0 {
            // Backwards: the reading bar meets each hole at its end first.
            let mut k = pos0.floor() as i64;
            while (k as f64) > pos1 {
                let frac = (pos0 - k as f64) / (pos0 - pos1);
                let at = ctx.now + (frac * ctx.len as f64) as u64;
                self.mechanical_tick(which, k, p, at, tick_frames / speed, breath, false, true, out, ctx);
                k -= 1;
            }
        }
        if !ctx.playing {
            self.flush_pending(ctx, out);
        }
        // Where it is, finely, and how fast, how wound (for the screens).
        let m = &self.mechanical;
        self.visual[0] = (self.visual[0] & 0xFFFF_FFFF) | ((m.tension.clamp(0.0, 1.0) * 255.0) as u64) << 32 | ((m.speed.abs().min(2.55) * 100.0) as u64) << 40 | (drive as u64) << 48 | ((m.speed < 0.0) as u64) << 56;
        // Within sixty-four bars (backwards it can go below the start).
        let tpb = if organ { organ_ticks_per_bar(p) } else { box_ticks_per_bar(p) } as f64;
        self.visual[3] = (pos1.rem_euclid(tpb * 64.0) * 16.0) as u64;
    }

    /// One tick of a mechanical sequencer (`which`: 1 the organ, 2 the box).
    #[allow(clippy::too_many_arguments)]
    /// Backwards (`back`) it plays the holes that end at `k`, each for as long as it is.
    fn mechanical_tick(&mut self, which: u64, k: i64, p: &[f32], at: u64, tick_frames: f64, breath: f32, song: bool, back: bool, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let organ = which == 1;
        let tpb = if organ { organ_ticks_per_bar(p) } else { box_ticks_per_bar(p) } as i64;
        // A hole ending on a bar line belongs to the bar before it.
        let bar = if back { (k - 1).div_euclid(tpb) } else { k.div_euclid(tpb) };
        let t = (k - bar * tpb) as u16;
        let key = fingerprint(which, p, if organ { OG_CHORD_AMOUNT } else { MB_CHORD_AMOUNT });
        let m = &mut self.mechanical;
        if m.bar != bar || m.key != key {
            m.root = if organ { organ_bar(p, bar, &mut m.holes) } else { box_bar(p, bar, &mut m.holes) };
            if m.bar != bar {
                m.chords = (m.chords << 4) | (m.root.rem_euclid(7) as u64);
            }
            m.bar = bar;
            m.key = key;
        }
        let base = p[if organ { OG_BASE } else { MB_BASE }];
        let vel = p[if organ { OG_VELOCITY } else { MB_VELOCITY }];
        // In the song's time the box's turn slows by Wind; a spring or a crank slows it for real.
        let wind = if organ || !song { 0.0 } else { p.get(MB_WIND).copied().unwrap_or(0.0).clamp(0.0, 1.0) };
        // The spring: through the cylinder's turn the notes come later and softer.
        let x = if organ { 0.0 } else { (bar.rem_euclid(16) as f32 + t as f32 / tpb as f32) / 16.0 };
        let last_bar = !organ && bar.rem_euclid(16) == 15;
        let n = self.mechanical.holes.len();
        for i in 0..n {
            let h = self.mechanical.holes[i];
            if (back && h.at + h.len != t) || (!back && h.at != t) {
                continue;
            }
            let pitch = match h.lane {
                Lane::Drum => h.deg,
                Lane::Bass if organ => fit(Self::mdeg(ctx, 43.0, h.deg, 0), 31, 47),
                Lane::Accomp if organ => fit(Self::mdeg(ctx, 52.0, h.deg, 0), 48, 59),
                _ => Self::mdeg(ctx, base, h.deg, 0),
            };
            if h.lane == Lane::Melody {
                self.mechanical.hist = (self.mechanical.hist << 8) | ((h.deg + 64).clamp(1, 255) as u64);
            }
            let mut v = vel * h.vel * breath;
            let mut when = at;
            if wind > 0.0 {
                let salt = (bar.rem_euclid(16) as u32) * 1024 + i as u32;
                if (last_bar && wind > 0.5 && t >= tpb as u16 / 3) || hash(0x5B12, salt) < wind * ((x - 0.75).max(0.0) * 3.0) {
                    continue;
                }
                let lag = wind * 6.0 * x.powi(3) + 0.15 * (hash(0x5B13, salt) - 0.5);
                when = at + (lag.max(0.0) as f64 * tick_frames) as u64;
                v *= 1.0 - 0.4 * wind * x * x;
            }
            let len = h.len as f64 * tick_frames * if organ { 0.94 } else { 1.0 };
            self.mlater(ctx, when, pitch, v, len, out);
        }
        let style = choose(p, 1, if organ { ORGAN_STYLES.len() } else { BOX_METRES.len() }) as u64;
        self.visual[0] = (self.visual[0] & !0xFFFF_FFFF) | which | style << 4 | (bar.rem_euclid(64) as u64) << 8 | (t as u64) << 16 | (tpb as u64) << 24;
        self.visual[1] = self.mechanical.hist;
        self.visual[2] = self.mechanical.chords;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::{render_notes, MidiFxKind};
    use crate::params::defaults;
    use crate::project::Note;
    use crate::scale::{Scale, ScaleKind};

    fn play(kind: MidiFxKind, p: &[f32], beats: f64) -> Vec<Note> {
        render_notes(kind, p, &[], beats, 120.0, Scale::new(0, ScaleKind::Major), true, 0)
    }

    fn in_c_major(pitch: u8) -> bool {
        [0, 2, 4, 5, 7, 9, 11].contains(&(pitch % 12))
    }

    #[test]
    fn the_organ_plays_its_divisions_in_key_and_repeats_its_first_phrase() {
        for style in 0..4 {
            let mut p = defaults(ORGAN);
            p[OG_STYLE] = style as f32;
            p[OG_ORNAMENTS] = 0.0;
            // Every turn the same (with Wander each is a little different, as a hand's are).
            p[OG_WANDER] = 0.0;
            let bar = organ_ticks_per_bar(&p) as f64 / 8.0;
            let notes = play(MidiFxKind::OrganGrinder, &p, bar * 32.0);
            assert!(!notes.is_empty(), "style {style}");
            let drums: Vec<&Note> = notes.iter().filter(|n| n.pitch <= 29).collect();
            assert!(!drums.is_empty() && drums.iter().all(|n| n.pitch >= 24), "drums on the drum keys");
            let pipes: Vec<&Note> = notes.iter().filter(|n| n.pitch > 29).collect();
            assert!(pipes.iter().all(|n| in_c_major(n.pitch)), "style {style}: in key");
            assert!(pipes.iter().any(|n| n.pitch < 48) && pipes.iter().any(|n| (48..60).contains(&n.pitch)) && pipes.iter().any(|n| n.pitch >= 60), "bass, accompaniment and melody");
            assert!(pipes.iter().all(|n| n.pitch >= 31), "no bass note on a drum key");
            // A A: the second phrase plays the first again.
            let phrase = bar * 8.0;
            let tune = |from: f64| -> Vec<(i64, u8)> { notes.iter().filter(|n| n.pitch >= 60 && n.start >= from && n.start < from + phrase).map(|n| (((n.start - from) * 8.0).round() as i64, n.pitch)).collect() };
            assert_eq!(tune(0.0), tune(phrase), "style {style}");
        }
    }

    #[test]
    fn tunes_differ_and_a_waltz_bar_is_three_beats() {
        let mut p = defaults(ORGAN);
        assert_eq!(organ_ticks_per_bar(&p), 24);
        let a = play(MidiFxKind::OrganGrinder, &p, 24.0);
        p[OG_TUNE] = 9.0;
        let b = play(MidiFxKind::OrganGrinder, &p, 24.0);
        let mel = |v: &[Note]| v.iter().filter(|n| n.pitch >= 60).map(|n| n.pitch).collect::<Vec<_>>();
        assert_ne!(mel(&a), mel(&b));
        // The bass on every bar's first beat, three beats apart.
        let bass: Vec<f64> = a.iter().filter(|n| (31..48).contains(&n.pitch)).map(|n| n.start).collect();
        assert!(bass.windows(2).all(|w| (w[1] - w[0] - 3.0).abs() < 1e-6), "{bass:?}");
    }

    /// Runs a sequencer block by block for `secs` seconds at 120 BPM, with
    /// `turn` setting its parameters before each block; the notes played
    /// (start in seconds, pitch).
    fn turn(kind: MidiFxKind, p: &mut Vec<f32>, secs: f64, playing: bool, mut each: impl FnMut(f64, &mut Vec<f32>)) -> Vec<(f64, u8)> {
        use crate::midi_fx::{MidiCtx, MidiFxState};
        let sr = 48_000.0f32;
        let spb = sr as f64 * 0.5;
        let mut st = MidiFxState::new(kind);
        let mut out = Vec::new();
        let mut notes = Vec::new();
        let block = 256usize;
        let mut frame = 0u64;
        while (frame as f64) < secs * sr as f64 {
            each(frame as f64 / sr as f64, p);
            let ctx = MidiCtx { now: frame, len: block, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: frame as f64 / spb, playing, sr, drum_notes: 0, lead: 0.0, touch: [0.0; 5] };
            out.clear();
            st.process(kind, p, &[], &[], &mut out, &ctx);
            notes.extend(out.iter().filter(|e| e.on).map(|e| ((frame + e.offset as u64) as f64 / sr as f64, e.pitch)));
            frame += block as u64;
        }
        notes
    }

    #[test]
    fn the_crank_drives_the_book_as_fast_as_it_is_turned() {
        let mut p = defaults(ORGAN);
        p[OG_DRIVE] = 1.0;
        p[OG_SWING] = 0.0;
        p[OG_WANDER] = 0.0;
        // Turned at the song's speed, then twice as fast: twice the notes a second.
        let slow = turn(MidiFxKind::OrganGrinder, &mut p.clone(), 12.0, true, |_, _| {}).iter().filter(|n| n.0 > 3.0).count();
        p[OG_CRANK] = 2.0;
        let fast = turn(MidiFxKind::OrganGrinder, &mut p.clone(), 12.0, true, |_, _| {}).iter().filter(|n| n.0 > 3.0).count();
        assert!(fast as f32 > slow as f32 * 1.6, "{slow} then {fast}");
        // Stopped, it plays only while a hand turns it, and stops when let go.
        let mut q = defaults(ORGAN);
        q[OG_DRIVE] = 1.0;
        let notes = turn(MidiFxKind::OrganGrinder, &mut q, 8.0, false, |t, p| p[OG_TURNING] = if t < 4.0 { 1.0 } else { 0.0 });
        assert!(notes.iter().any(|n| n.0 < 4.0), "turning plays it");
        assert!(notes.iter().all(|n| n.0 < 5.5), "let go, it runs out and stops: {:?}", notes.last());
    }

    #[test]
    fn turned_backwards_it_plays_the_book_backwards() {
        let mut p = defaults(ORGAN);
        p[OG_DRIVE] = 1.0;
        p[OG_DRUMS] = 0.0;
        // Forward for six seconds, then back as fast for six: the melody comes in the opposite order.
        let notes = turn(MidiFxKind::OrganGrinder, &mut p, 12.0, false, |t, p| p[OG_TURNING] = if t < 6.0 { 1.0 } else { -1.0 });
        // The tune: the highest note at each moment (the counter melody sounds under it).
        let tune = |from: f64, to: f64| -> Vec<u8> {
            let mut v: Vec<(i64, u8)> = Vec::new();
            for n in notes.iter().filter(|n| n.0 > from && n.0 < to && n.1 >= 60) {
                let t = (n.0 * 1000.0).round() as i64;
                match v.last_mut() {
                    Some(last) if last.0 == t => last.1 = last.1.max(n.1),
                    _ => v.push((t, n.1)),
                }
            }
            v.into_iter().map(|x| x.1).collect()
        };
        let fwd = tune(1.0, 5.0);
        let back = tune(6.0, 12.0);
        assert!(fwd.len() > 6 && back.len() > 6, "{} {}", fwd.len(), back.len());
        // What it played going forward comes back reversed: the last notes forward are the first going back.
        let tail: Vec<u8> = fwd.iter().rev().take(4).copied().collect();
        assert!(back.windows(4).any(|w| w == tail.as_slice()), "forward ended {tail:?}, back went {back:?}");
    }

    #[test]
    fn a_turn_swings_and_comes_round_on_time() {
        let mut p = defaults(ORGAN);
        p[OG_SWING] = 1.0;
        p[OG_WANDER] = 0.0;
        p[OG_PUSH] = 0.0;
        let f = Feel::of(&p, &[], true);
        assert!(f.offset(0.0, 0).abs() < 1e-9 && f.offset(1.0, 0).abs() < 1e-9, "on time at every turn's end");
        assert!(f.offset(0.25, 0) > 0.02, "ahead where it pushes");
        let mean: f64 = (0..1000).map(|i| f.speed(i as f64 / 1000.0, 0)).sum::<f64>() / 1000.0;
        assert!((mean - 1.0).abs() < 0.01, "the same speed on average ({mean})");
        // Without swing, even.
        p[OG_SWING] = 0.0;
        assert!(Feel::of(&p, &[], true).offset(0.3, 0).abs() < 1e-12);
        // A learned turn: slow first half, quick second half.
        let mut bins = [0.6f32; FEEL_BINS];
        for b in bins.iter_mut().skip(8) {
            *b = 1.4;
        }
        let data = encode_feel(&bins);
        assert!(decode_feel(&data).is_some_and(|b| (b[0] - 0.6).abs() < 0.02 && (b[15] - 1.4).abs() < 0.02));
        p[OG_FEEL] = 1.0;
        let y = Feel::of(&p, &data, true);
        assert!(y.learned() && y.offset(0.5, 0) < -0.1 && y.offset(1.0, 0).abs() < 1e-6, "{}", y.offset(0.5, 0));
        // With a turn a beat and swing, the quavers in the song's time come long, short.
        let mut q = defaults(ORGAN);
        q[OG_TURN] = 2.0;
        q[OG_SWING] = 1.0;
        q[OG_PUSH] = 180.0;
        q[OG_WANDER] = 0.0;
        let tpt = ticks_per_turn(&q, true);
        let fq = Feel::of(&q, &[], true);
        let half = fq.book(tpt * 0.5, tpt) / tpt;
        assert!(half < 0.45 && (fq.book(tpt, tpt) / tpt - 1.0).abs() < 1e-9, "the second quaver comes late: {half}");
    }

    #[test]
    fn a_wound_spring_plays_the_box_until_it_runs_down() {
        let mut p = defaults(MUSIC_BOX);
        p[MB_DRIVE] = 1.0;
        p[MB_SPRING] = 8.0;
        // Unwound and stopped: silent. Wound (two turns of the key): it plays, slows and stops.
        let notes = turn(MidiFxKind::MusicBox, &mut p, 14.0, false, |t, p| p[MB_WOUND] = if t >= 1.0 { 2.0 } else { 0.0 });
        assert!(notes.iter().all(|n| n.0 >= 1.0), "nothing before it is wound");
        assert!(notes.iter().any(|n| n.0 < 3.0), "it plays once wound");
        let last = notes.last().map(|n| n.0).unwrap_or(0.0);
        assert!(last < 12.0, "and stops when it has run down ({last})");
        assert!((0.25..0.9).contains(&spring_speed(0.1)) && spring_speed(0.0) == 0.0 && spring_speed(0.8) > 1.0);
    }

    #[test]
    fn the_music_box_keeps_to_its_comb_and_its_spring_runs_down() {
        let mut p = defaults(MUSIC_BOX);
        p[MB_WIND] = 0.0;
        let steady = play(MidiFxKind::MusicBox, &p, 48.0);
        assert!(!steady.is_empty() && steady.iter().all(|n| in_c_major(n.pitch)));
        let (lo, hi) = (steady.iter().map(|n| n.pitch).min().unwrap(), steady.iter().map(|n| n.pitch).max().unwrap());
        // Thirty notes on the comb: about four octaves at most.
        assert!(hi - lo <= 4 * 12, "{lo}..{hi}");
        // On the grid without wind; late towards the end of the turn with it.
        assert!(steady.iter().all(|n| (n.start * 8.0 - (n.start * 8.0).round()).abs() < 1e-6));
        p[MB_WIND] = 1.0;
        let wound = play(MidiFxKind::MusicBox, &p, 48.0);
        let late = wound.iter().filter(|n| n.start > 36.0 && n.start < 45.0).filter(|n| (n.start * 8.0 - (n.start * 8.0).round()).abs() > 0.05).count();
        assert!(late > 0, "the spring makes notes late");
        assert!(wound.len() < steady.len(), "and some go missing");
    }
}
