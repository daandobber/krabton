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
//! Every bar is made from the parameters alone (`organ_bar`, `box_bar`), so
//! what the screens draw is what plays.

use super::minimal::{choose, chord_root, hash, CHORD_AMOUNT, DYNAMICS};
use super::{SeqState, RATES};
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

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
}

impl Default for Mechanical {
    fn default() -> Self {
        Self { bar: i64::MIN, key: 0, root: 0, holes: Vec::with_capacity(160), hist: 0, chords: 0 }
    }
}

/// What the bar depends on, so a turned knob makes it again.
fn fingerprint(which: u64, p: &[f32]) -> u64 {
    p.iter().take(8).chain(p.last()).fold(which, |h, v| h.wrapping_mul(31).wrapping_add((v * 1000.0) as i64 as u64))
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
    /// One tick of a mechanical sequencer (`which`: 1 the organ, 2 the box).
    #[allow(clippy::too_many_arguments)]
    fn mechanical(&mut self, which: u64, k: i64, p: &[f32], at: u64, tick_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let organ = which == 1;
        let tpb = if organ { organ_ticks_per_bar(p) } else { box_ticks_per_bar(p) } as i64;
        let bar = k.div_euclid(tpb);
        let t = k.rem_euclid(tpb) as u16;
        let key = fingerprint(which, p);
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
        let wind = if organ { 0.0 } else { p.get(MB_WIND).copied().unwrap_or(0.0).clamp(0.0, 1.0) };
        // The spring: through the cylinder's turn the notes come later and softer.
        let x = if organ { 0.0 } else { (bar.rem_euclid(16) as f32 + t as f32 / tpb as f32) / 16.0 };
        let last_bar = !organ && bar.rem_euclid(16) == 15;
        let n = self.mechanical.holes.len();
        for i in 0..n {
            let h = self.mechanical.holes[i];
            if h.at != t {
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
            let mut v = vel * h.vel;
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
        self.visual[0] = which | style << 4 | (bar.rem_euclid(64) as u64) << 8 | (t as u64) << 16 | (tpb as u64) << 24;
        self.visual[1] = self.mechanical.hist;
        self.visual[2] = self.mechanical.chords;
        self.visual[3] = k as u64;
    }

    pub(crate) fn organ_grinder(&mut self, k: i64, p: &[f32], at: u64, tick_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        self.mechanical(1, k, p, at, tick_frames, out, ctx);
    }

    pub(crate) fn music_box(&mut self, k: i64, p: &[f32], at: u64, tick_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        self.mechanical(2, k, p, at, tick_frames, out, ctx);
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
