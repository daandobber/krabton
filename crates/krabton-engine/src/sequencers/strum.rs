//! Strum: guitar chords the way a guitarist plays them. Each chord of the
//! progression (in the project key, as in Progression) gets a real chord
//! shape on the six strings: open chords in the first position, barre
//! chords up the neck, or high up. A pattern of strokes on a sixteenth grid
//! plays it: down strokes sweep from the low strings up, up strokes catch
//! the top four from the high string down, mutes chuck, and the picking
//! strokes pluck the bass, an alternating bass or single high strings
//! (Travis picking, arpeggios). Ukulele does the same on four strings, in
//! the shapes a ukulele player uses.

use super::SeqState;
use crate::classic_seq::{chord_pitches, prog_slot, CHORD_LENGTHS, PROG_BYTES, PROG_SLOTS};
use crate::midi_fx::{MidiCtx, MidiEvent, MidiFxKind};
use crate::params::{ParamSpec, Unit};

/// Open strings in standard tuning, low E to high E.
pub const OPEN: [i32; 6] = [40, 45, 50, 55, 59, 64];

pub const LENGTH: usize = 0;
pub const CHORDS: usize = 1;
pub const STRUM: usize = 2;
pub const SWING: usize = 3;
pub const GATE: usize = 4;
pub const LET_RING: usize = 5;
pub const VELOCITY: usize = 6;
pub const HUMANIZE: usize = 7;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Length", "Chords", CHORD_LENGTHS, 2),
    ParamSpec::int("Chords", "Chords", 1.0, 8.0, 4.0, Unit::None),
    ParamSpec::lin("Strum", "Strokes", 0.0, 80.0, 22.0, Unit::Ms),
    ParamSpec::lin("Swing", "Strokes", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Gate", "Strokes", 0.1, 1.0, 1.0, Unit::Percent),
    ParamSpec::toggle("Let Ring", "Strokes", false),
    ParamSpec::lin("Velocity", "Strokes", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Humanize", "Strokes", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::toggle("Band", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

pub const TUNING: usize = 8;
pub const UKE_TUNINGS: &[&str] = &["GCEA", "Low G", "Baritone", "D Tuning"];

/// The open strings of each ukulele tuning, from the string nearest the
/// chin (where a down stroke starts): the high G of the usual re-entrant
/// tuning, a low G, the baritone's DGBE, the old ADF#B.
const UKE_OPEN: [[i32; 4]; 4] = [[67, 60, 64, 69], [55, 60, 64, 69], [50, 55, 59, 64], [69, 62, 66, 71]];

pub static UKE_SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Length", "Chords", CHORD_LENGTHS, 2),
    ParamSpec::int("Chords", "Chords", 1.0, 8.0, 4.0, Unit::None),
    ParamSpec::lin("Strum", "Strokes", 0.0, 80.0, 16.0, Unit::Ms),
    ParamSpec::lin("Swing", "Strokes", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Gate", "Strokes", 0.1, 1.0, 1.0, Unit::Percent),
    ParamSpec::toggle("Let Ring", "Strokes", false),
    ParamSpec::lin("Velocity", "Strokes", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Humanize", "Strokes", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::choice("Tuning", "Strings", UKE_TUNINGS, 0),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::toggle("Band", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

/// The open strings a device plays, in the order a down stroke hits them:
/// six for Guitar Chords, four for Ukulele.
pub fn strings(kind: MidiFxKind, p: &[f32]) -> &'static [i32] {
    if kind == MidiFxKind::Ukulele {
        &UKE_OPEN[(p.get(TUNING).copied().unwrap_or(0.0).round().max(0.0) as usize).min(UKE_OPEN.len() - 1)]
    } else {
        &OPEN
    }
}

/// Where each chord sits on the neck (the third byte of a chord slot).
pub const NECKS: &[&str] = &["Open", "Barre", "High"];

// ---------------------------------------------------------------- strokes

/// The strokes of the pattern, one per sixteenth.
pub const REST: u8 = 0;
pub const DOWN: u8 = 1;
pub const UP: u8 = 2;
pub const SOFT_DOWN: u8 = 3;
pub const SOFT_UP: u8 = 4;
pub const MUTE: u8 = 5;
pub const BASS: u8 = 6;
pub const ALT_BASS: u8 = 7;
pub const PINCH: u8 = 8;
/// 9..=12: one string, counted from the top (9 is the highest played).
pub const TOP1: u8 = 9;
pub const CHOP: u8 = 13;
pub const STROKES: u8 = 14;

/// A short label for a stroke (for the editor).
pub fn stroke_label(s: u8) -> &'static str {
    match s {
        DOWN => "↓",
        UP => "↑",
        SOFT_DOWN => "d",
        SOFT_UP => "u",
        MUTE => "×",
        BASS => "B",
        ALT_BASS => "b",
        PINCH => "P",
        9 => "1",
        10 => "2",
        11 => "3",
        12 => "4",
        CHOP => "c",
        _ => "·",
    }
}

pub fn stroke_name(s: u8) -> &'static str {
    match s {
        DOWN => "down stroke",
        UP => "up stroke (top strings)",
        SOFT_DOWN => "soft down stroke",
        SOFT_UP => "soft up stroke",
        MUTE => "muted chuck",
        BASS => "bass note",
        ALT_BASS => "alternating bass",
        PINCH => "pinch: the two top strings",
        9 => "the top string",
        10 => "the second string from the top",
        11 => "the third string from the top",
        12 => "the fourth string from the top",
        CHOP => "chop: short, on the top strings",
        _ => "rest",
    }
}

pub const PATTERN_STEPS: usize = 16;
pub const PATTERNS: &[(&str, [u8; 16])] = &[
    ("Old Faithful", [1, 0, 0, 0, 1, 0, 2, 0, 0, 0, 2, 0, 1, 0, 2, 0]),
    ("Quarters", [1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0]),
    ("Eighths", [1, 0, 2, 0, 1, 0, 2, 0, 1, 0, 2, 0, 1, 0, 2, 0]),
    ("Rock", [1, 0, 3, 0, 1, 0, 3, 0, 1, 0, 3, 0, 1, 0, 3, 2]),
    ("Ballad", [1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 1, 0]),
    ("Reggae", [0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0]),
    ("Funk", [1, 0, 5, 2, 1, 0, 5, 2, 0, 2, 5, 2, 1, 0, 5, 2]),
    ("Gallop", [1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1]),
    ("Travis Picking", [6, 0, 9, 0, 7, 0, 10, 0, 6, 0, 9, 0, 7, 0, 10, 0]),
    ("Arpeggio", [6, 0, 7, 0, 11, 0, 10, 0, 9, 0, 10, 0, 11, 0, 7, 0]),
    ("Waltz Feel", [6, 0, 0, 0, 8, 0, 0, 0, 8, 0, 0, 0, 6, 0, 8, 0]),
    ("Boom-Chick", [6, 0, 0, 0, 3, 0, 0, 0, 7, 0, 0, 0, 3, 0, 0, 0]),
    ("Bossa", [6, 0, 0, 8, 0, 0, 8, 0, 7, 0, 0, 8, 0, 0, 8, 0]),
    ("Calypso", [1, 0, 0, 2, 0, 2, 1, 0, 1, 0, 0, 2, 0, 2, 1, 0]),
    ("Rasgueado", [1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 0, 2, 0]),
    ("Downstrokes", [1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0]),
    ("Funk Sixteenths", [1, 5, 2, 5, 5, 2, 1, 5, 2, 5, 1, 2, 5, 2, 1, 2]),
    ("Fingerpicked Ballad", [6, 0, 11, 0, 10, 0, 9, 0, 7, 0, 11, 0, 10, 0, 9, 0]),
    ("Ska Upstrokes", [0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 2, 0]),
    ("Shuffle", [1, 0, 0, 2, 1, 0, 0, 2, 1, 0, 0, 2, 1, 0, 0, 2]),
];

/// Chords, then the pattern.
pub const DATA_BYTES: usize = PROG_SLOTS * PROG_BYTES + PATTERN_STEPS;

pub fn strum_default() -> Vec<u8> {
    // I - V - vi - IV, all open; then the same with the IV and V as 7ths.
    let slots: [(u8, u8, u8); PROG_SLOTS] = [(1, 0, 0), (5, 0, 0), (6, 0, 0), (4, 0, 0), (1, 0, 0), (4, 0, 0), (5, 1, 0), (1, 0, 0)];
    let mut d: Vec<u8> = slots.iter().flat_map(|s| [s.0, s.1, s.2]).collect();
    d.extend_from_slice(&PATTERNS[0].1);
    d
}

pub fn pattern_at(data: &[u8], i: usize) -> u8 {
    data.get(PROG_SLOTS * PROG_BYTES + i % PATTERN_STEPS).copied().filter(|s| *s < STROKES).unwrap_or(REST)
}

// ---------------------------------------------------------------- shapes

/// A chord shape on the six strings: the fret of each string (None:
/// muted), low E first, in standard tuning.
pub type Shape = [Option<u8>; 6];

/// The MIDI notes of a shape, low string first.
pub fn shape_notes(shape: &Shape) -> Vec<i32> {
    frets_notes(&OPEN, shape)
}

/// The notes of a shape on these open strings, in string order.
pub fn frets_notes(open: &[i32], shape: &Shape) -> Vec<i32> {
    open.iter().zip(shape).filter_map(|(o, f)| f.map(|f| o + f as i32)).collect()
}

/// A ukulele chord on four strings (the rest of the shape stays empty):
/// every string rings a chord tone, every tone is there (a four-note chord
/// may leave out the fifth), with as few fingers as it takes, open strings
/// and low frets first: C is 0003, Am 2000, G 0232. `neck` 0 looks in the
/// first position, 1 from the second fret up, 2 higher.
pub fn uke_shape(open: &[i32], tones: &[i32], neck: u8) -> Shape {
    let mut pcs: Vec<i32> = Vec::new();
    for t in tones {
        let pc = t.rem_euclid(12);
        if !pcs.contains(&pc) {
            pcs.push(pc);
        }
    }
    let Some(&root) = pcs.first() else { return [None; 6] };
    let fifth = (root + 7).rem_euclid(12);
    let n = open.len().min(6);
    let positions: std::ops::RangeInclusive<i32> = match neck {
        0 => 0..=0,
        1 => 2..=6,
        _ => 5..=10,
    };
    let mut best: (f32, Shape) = (f32::MIN, [None; 6]);
    for pos in positions {
        let window: Vec<i32> = if neck == 0 { (0..=4).collect() } else { (pos..=pos + 3).collect() };
        let cands: Vec<Vec<i32>> = open[..n].iter().map(|o| window.iter().copied().filter(|f| pcs.contains(&(o + f).rem_euclid(12))).collect()).collect();
        if cands.iter().any(|c| c.is_empty()) {
            continue;
        }
        // Every way to finger it.
        let mut idx = vec![0usize; n];
        loop {
            let f: Vec<i32> = (0..n).map(|s| cands[s][idx[s]]).collect();
            let rung: Vec<i32> = (0..n).map(|s| (open[s] + f[s]).rem_euclid(12)).collect();
            let missing = pcs.iter().filter(|t| !rung.contains(t) && !(**t == fifth && pcs.len() > 3)).count();
            let fretted: Vec<i32> = f.iter().copied().filter(|&x| x > 0).collect();
            let lo = fretted.iter().copied().min().unwrap_or(0);
            let hi = fretted.iter().copied().max().unwrap_or(0);
            let at_lo = fretted.iter().filter(|&&x| x == lo).count();
            // A barre takes the lowest fret with one finger.
            let fingers = if at_lo >= 2 { fretted.len() - at_lo + 1 } else { fretted.len() };
            if hi - lo <= 3 && fingers <= 4 {
                let opens = f.iter().filter(|&&x| x == 0).count();
                let score = -4.0 * missing as f32 - 0.5 * fingers as f32 - 0.15 * f.iter().sum::<i32>() as f32 + 0.3 * opens as f32;
                if score > best.0 {
                    let mut shape: Shape = [None; 6];
                    for s in 0..n {
                        shape[s] = Some(f[s] as u8);
                    }
                    best = (score, shape);
                }
            }
            // The next combination.
            let mut s = 0;
            while s < n {
                idx[s] += 1;
                if idx[s] < cands[s].len() {
                    break;
                }
                idx[s] = 0;
                s += 1;
            }
            if s == n {
                break;
            }
        }
    }
    best.1
}

/// Finds the shape a guitarist would play for a chord with root `root` (a
/// pitch class) and pitch classes `tones`: the root in the bass, as many
/// strings as can ring without gaps, every tone if it can be had (the
/// third and seventh before a doubled root or the fifth), four fingers (a
/// barre counts as one). `neck` 0 looks in the first position with open
/// strings, 1 for barre chords from the first fret up, 2 higher up.
pub fn guitar_shape(root: i32, tones: &[i32], power: bool, neck: u8) -> Shape {
    let root = root.rem_euclid(12);
    let has = |n: i32| tones.contains(&n.rem_euclid(12));
    let positions: std::ops::RangeInclusive<i32> = match neck {
        0 => 0..=4,
        1 => 1..=8,
        _ => 5..=12,
    };
    let mut best: (f32, Shape) = (f32::MIN, [None; 6]);
    for pos in positions {
        let open_ok = neck == 0 && pos == 0;
        let window: Vec<i32> = if open_ok { (0..=3).collect() } else { (pos..=pos + 3).collect() };
        // The bass: the lowest of E, A, D strings with the root in reach.
        let Some((bass_s, bass_f)) = (0..3).find_map(|s| window.iter().find(|&&f| (OPEN[s] + f).rem_euclid(12) == root).map(|&f| (s, f))) else {
            continue;
        };
        let mut shape: Shape = [None; 6];
        shape[bass_s] = Some(bass_f as u8);
        if power {
            // Root, fifth, octave on the next two strings.
            for (k, iv) in [(1usize, 7), (2, 12)] {
                let s = bass_s + k;
                let f = OPEN[bass_s] + bass_f + iv - OPEN[s];
                if s < 6 && (0..=15).contains(&f) {
                    shape[s] = Some(f as u8);
                }
            }
            let score = 10.0 - pos as f32 * 0.2 + if bass_s == 0 { 0.5 } else { 0.0 };
            if score > best.0 {
                best = (score, shape);
            }
            continue;
        }
        for s in bass_s + 1..6 {
            shape[s] = window.iter().find(|&&f| has(OPEN[s] + f)).map(|&f| f as u8);
        }
        // Tones still missing: trade a doubled tone (or the fifth) for them,
        // the lowest fret first.
        let pc = |s: usize, f: u8| (OPEN[s] + f as i32).rem_euclid(12);
        let count = |sh: &Shape, n: i32| (0..6).filter(|&s| sh[s].is_some_and(|f| pc(s, f) == n)).count();
        let fifth = (root + 7).rem_euclid(12);
        let mut wanted: Vec<i32> = tones.iter().map(|t| t.rem_euclid(12)).filter(|t| *t != root && *t != fifth).collect();
        wanted.sort_by_key(|t| {
            let iv = (t - root).rem_euclid(12);
            // Thirds first, then sevenths, then the rest.
            if iv == 3 || iv == 4 {
                0
            } else if iv == 10 || iv == 11 {
                1
            } else {
                2
            }
        });
        for m in wanted {
            if count(&shape, m) > 0 {
                continue;
            }
            let mut swap: Option<(usize, u8)> = None;
            for s in bass_s + 1..6 {
                let Some(cur) = shape[s] else { continue };
                let c = pc(s, cur);
                if !(count(&shape, c) > 1 || c == fifth) {
                    continue;
                }
                if let Some(&f) = window.iter().find(|&&f| pc(s, f as u8) == m) {
                    let better = match swap {
                        None => true,
                        Some((ss, ff)) => (f as u8) < ff || (f as u8 == ff && s > ss),
                    };
                    if better {
                        swap = Some((s, f as u8));
                    }
                }
            }
            if let Some((s, f)) = swap {
                shape[s] = Some(f);
            }
        }
        // Fingers: open strings are free, a barre at the position is one.
        let fretted: Vec<u8> = shape.iter().flatten().copied().filter(|&f| f > 0).collect();
        let fingers = if open_ok {
            fretted.len()
        } else {
            let barre = fretted.iter().filter(|&&f| f as i32 == pos).count() >= 2;
            fretted.iter().filter(|&&f| !barre || f as i32 > pos).count() + barre as usize
        };
        if fingers > 4 {
            continue;
        }
        let played = shape.iter().filter(|f| f.is_some()).count();
        let top = (0..6).rev().find(|&s| shape[s].is_some()).unwrap_or(0);
        let gaps = (bass_s..top).filter(|&s| shape[s].is_none()).count();
        // The fifth may go when there is more to the chord (as in x32310).
        let missing = tones.iter().filter(|&&t| count(&shape, t.rem_euclid(12)) == 0 && !(t.rem_euclid(12) == fifth && tones.len() > 3)).count();
        // Up the neck the root sits under the barre (the first finger).
        let off_barre = if !open_ok && bass_f != pos { 1.5 } else { 0.0 };
        let score = played as f32 - 3.0 * gaps as f32 - 3.0 * missing as f32 - 0.25 * pos as f32 - off_barre + if open_ok { 0.6 } else { 0.0 } + if bass_s == 0 { 0.2 } else { 0.0 };
        if score > best.0 {
            best = (score, shape);
        }
    }
    best.1
}

/// The shape of chord slot `slot` of `data` in `scale`, moved by `shift`
/// scale steps.
pub fn slot_shape(scale: &crate::scale::Scale, data: &[u8], slot: usize, shift: i32) -> Option<(Shape, u8, u8)> {
    let (degree, quality, neck) = prog_slot(data, slot);
    if degree == 0 {
        return None;
    }
    let degree = ((degree as i32 - 1 + shift).rem_euclid(7) + 1) as u8;
    let notes = chord_pitches(scale, 60 + scale.root as i32, degree, quality, 0);
    let root = *notes.first()?;
    let tones: Vec<i32> = notes.iter().map(|n| n.rem_euclid(12)).collect();
    Some((guitar_shape(root, &tones, quality == 4, neck.min(2)), degree, quality))
}

/// The shape of chord slot `slot` on the device's strings (a guitar's or a
/// ukulele's).
pub fn slot_shape_for(kind: MidiFxKind, p: &[f32], scale: &crate::scale::Scale, data: &[u8], slot: usize, shift: i32) -> Option<(Shape, u8, u8)> {
    if kind != MidiFxKind::Ukulele {
        return slot_shape(scale, data, slot, shift);
    }
    let (degree, quality, neck) = prog_slot(data, slot);
    if degree == 0 {
        return None;
    }
    let degree = ((degree as i32 - 1 + shift).rem_euclid(7) + 1) as u8;
    let notes = chord_pitches(scale, 60 + scale.root as i32, degree, quality, 0);
    let tones: Vec<i32> = notes.iter().map(|n| n.rem_euclid(12)).collect();
    Some((uke_shape(strings(kind, p), &tones, neck.min(2)), degree, quality))
}

/// The name of a chord from its root and tones, like "Am7" or "G".
pub fn chord_letter_name(scale: &crate::scale::Scale, degree: u8, quality: u8) -> String {
    let notes = chord_pitches(scale, 60 + scale.root as i32, degree, quality, 0);
    let Some(&root) = notes.first() else { return "-".into() };
    let name = crate::note_name(root.clamp(0, 127) as u8);
    let letter = name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '-');
    let iv = |n: usize| notes.get(n).map(|x| (x - root).rem_euclid(12));
    let minor = iv(1) == Some(3);
    let dim = minor && iv(2) == Some(6);
    let suffix = match quality {
        1 => match (minor, iv(3)) {
            (true, Some(10)) if dim => "m7b5",
            (true, Some(10)) => "m7",
            (false, Some(11)) => "maj7",
            (false, Some(10)) => "7",
            (true, Some(11)) => "m(maj7)",
            _ => "7",
        },
        2 => "sus2",
        3 => "sus4",
        4 => "5",
        5 => {
            if minor {
                "m(add9)"
            } else {
                "add9"
            }
        }
        6 => {
            if minor {
                "m9"
            } else {
                "9"
            }
        }
        _ => {
            if dim {
                "dim"
            } else if minor {
                "m"
            } else {
                ""
            }
        }
    };
    format!("{letter}{suffix}")
}

/// Packs a shape for the editor and the grid: 5 bits a string (31: muted).
pub fn pack_shape(shape: &Shape) -> u64 {
    (0..6).fold(0u64, |acc, s| acc | ((shape[s].map(|f| f.min(30) as u64).unwrap_or(31)) << (s * 5)))
}

pub fn unpack_shape(v: u64) -> Shape {
    std::array::from_fn(|s| {
        let f = (v >> (s * 5)) & 31;
        (f != 31).then_some(f as u8)
    })
}

// ---------------------------------------------------------------- playing

impl SeqState {
    /// Runs on a 1/16 grid.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn strum(&mut self, kind: MidiFxKind, k: i64, p: &[f32], data: &[u8], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let beats = [1.0, 2.0, 4.0, 8.0][(p[LENGTH].round().max(0.0) as usize).min(3)];
        let per_chord = (beats * 4.0f64).round() as i64;
        let chords = p[CHORDS].round().clamp(1.0, PROG_SLOTS as f32) as i64;
        let slot = k.div_euclid(per_chord).rem_euclid(chords) as usize;
        let step = k.rem_euclid(PATTERN_STEPS as i64) as usize;
        // Transpose: a key moves the progression through the scale (or by
        // semitones with the scale off), from C.
        let (shift_steps, shift_semis) = match self.shift {
            Some((key, reference)) if ctx.scale_on => {
                let (dk, _) = ctx.scale.locate(key);
                let (dr, _) = ctx.scale.locate(reference);
                (dk - dr, 0)
            }
            Some((key, reference)) => (0, key - reference),
            None => (0, 0),
        };
        let shape = slot_shape_for(kind, p, &ctx.scale, data, slot, shift_steps);
        let stroke = pattern_at(data, step);
        self.visual[0] = slot as u64 | (step as u64) << 8 | (stroke as u64) << 16 | (k.rem_euclid(per_chord) as u64) << 24 | (per_chord as u64) << 40;
        self.visual[1] = shape.map(|s| pack_shape(&s.0)).unwrap_or(u64::MAX);
        let Some((shape, _, _)) = shape else { return };
        if stroke == REST {
            return;
        }
        // The strings this stroke plays, in the order they are hit.
        let played: Vec<i32> = frets_notes(strings(kind, p), &shape).into_iter().map(|n| n + shift_semis).collect();
        if played.is_empty() {
            return;
        }
        let n = played.len();
        let top = |i: usize| played[n - 1 - i.min(n - 1)];
        let hits: Vec<i32> = match stroke {
            DOWN | SOFT_DOWN | MUTE => played.clone(),
            UP | SOFT_UP => played[n.saturating_sub(4)..].iter().rev().copied().collect(),
            CHOP => played[n.saturating_sub(4)..].to_vec(),
            BASS => vec![played[0]],
            ALT_BASS => vec![played[1.min(n - 1)]],
            PINCH => vec![top(1), top(0)],
            s => vec![top((s - TOP1) as usize)],
        };
        // How long it rings: to the next stroke (or the end of the chord
        // with Let Ring).
        let next = (1..=PATTERN_STEPS).find(|&j| pattern_at(data, step + j) != REST).unwrap_or(PATTERN_STEPS) as f64;
        let to_chord_end = (per_chord - k.rem_euclid(per_chord)) as f64;
        let ring = if p[LET_RING] >= 0.5 { to_chord_end.max(next) } else { next.min(to_chord_end) };
        let len = match stroke {
            MUTE => 0.12 * step_frames,
            CHOP => 0.45 * step_frames,
            _ => (ring * step_frames * p[GATE].clamp(0.1, 1.0) as f64 - 8.0).max(16.0),
        };
        let human = p[HUMANIZE].clamp(0.0, 1.0);
        let accent = if step % 4 == 0 { 1.0 } else if step % 2 == 0 { 0.88 } else { 0.8 };
        let kind_vel = match stroke {
            UP => 0.8,
            SOFT_DOWN | SOFT_UP => 0.55,
            MUTE => 0.45,
            BASS | ALT_BASS => 0.95,
            _ => 1.0,
        };
        let at = Self::swing_at(at, k, p[SWING], step_frames) as f64 + human as f64 * 0.006 * ctx.sr as f64 * self.unit_bipolar();
        // The sweep across the strings: Strum for a full six-string stroke.
        let sweep = p[STRUM].clamp(0.0, 80.0) as f64 * 0.001 * ctx.sr as f64 * (1.0 + 0.25 * human as f64 * self.unit_bipolar());
        let gap = if hits.len() > 1 { sweep / 5.0 } else { 0.0 };
        for (i, &pitch) in hits.iter().enumerate() {
            let when = (at + i as f64 * gap * (1.0 + 0.3 * human as f64 * self.unit_bipolar())).max(at) as u64;
            // A sweep loses a little on the way.
            let vel = p[VELOCITY] * accent * kind_vel * (1.0 - 0.04 * i as f32) * (1.0 + 0.1 * human * self.unit_bipolar() as f32);
            self.play(ctx, when, pitch, vel.clamp(0.02, 1.0), len, out);
        }
    }

    fn unit_bipolar(&mut self) -> f64 {
        self.unit() as f64 * 2.0 - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::{MidiFxKind, MidiFxState};
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};

    fn shape_of(root: i32, ivs: &[i32], neck: u8) -> String {
        let tones: Vec<i32> = ivs.iter().map(|i| (root + i).rem_euclid(12)).collect();
        guitar_shape(root, &tones, false, neck).iter().map(|f| f.map(|f| char::from_digit(f as u32, 16).unwrap()).unwrap_or('x')).collect()
    }

    #[test]
    fn open_chords_look_like_open_chords() {
        let maj = [0, 4, 7];
        let min = [0, 3, 7];
        assert_eq!(shape_of(0, &maj, 0), "x32010", "C");
        assert_eq!(shape_of(7, &maj, 0), "320003", "G");
        assert_eq!(shape_of(2, &maj, 0), "xx0232", "D");
        assert_eq!(shape_of(4, &maj, 0), "022100", "E");
        assert_eq!(shape_of(9, &maj, 0), "x02220", "A");
        assert_eq!(shape_of(9, &min, 0), "x02210", "Am");
        assert_eq!(shape_of(4, &min, 0), "022000", "Em");
        assert_eq!(shape_of(2, &min, 0), "xx0231", "Dm");
        assert_eq!(shape_of(0, &[0, 4, 7, 10], 0), "x32310", "C7");
        assert_eq!(shape_of(7, &[0, 4, 7, 10], 0), "320001", "G7");
        assert_eq!(shape_of(9, &[0, 3, 7, 10], 0), "x02010", "Am7");
        assert_eq!(shape_of(0, &[0, 4, 7, 11], 0), "x32000", "Cmaj7");
    }

    #[test]
    fn f_and_b_minor_are_barre_chords() {
        assert_eq!(shape_of(5, &[0, 4, 7], 0), "133211", "F");
        assert_eq!(shape_of(11, &[0, 3, 7], 0), "x24432", "Bm");
        // Barre asked for: A major up at the fifth fret.
        assert_eq!(shape_of(9, &[0, 4, 7], 1), "577655", "A barre");
    }

    #[test]
    fn ukulele_chords_are_the_ones_players_use() {
        let uke = |root: i32, ivs: &[i32], tuning: usize, neck: u8| -> String {
            let tones: Vec<i32> = ivs.iter().map(|i| (root + i).rem_euclid(12)).collect();
            uke_shape(&UKE_OPEN[tuning], &tones, neck).iter().take(4).map(|f| f.map(|f| char::from_digit(f as u32, 16).unwrap()).unwrap_or('x')).collect()
        };
        let maj = [0, 4, 7];
        assert_eq!(uke(0, &maj, 0, 0), "0003", "C");
        assert_eq!(uke(9, &[0, 3, 7], 0, 0), "2000", "Am");
        assert_eq!(uke(5, &maj, 0, 0), "2010", "F");
        assert_eq!(uke(7, &maj, 0, 0), "0232", "G");
        assert_eq!(uke(7, &[0, 4, 7, 10], 0, 0), "0212", "G7");
        assert_eq!(uke(2, &maj, 0, 0), "2220", "D");
        assert_eq!(uke(10, &maj, 0, 0), "3211", "Bb");
        // Up the neck every string is fretted.
        let high = uke(0, &maj, 0, 2);
        assert!(!high.contains('0'), "{high}");
        // A baritone is the top four strings of a guitar: G is 0003 there too.
        assert_eq!(uke(7, &maj, 2, 0), "0003", "G on a baritone");
    }

    #[test]
    fn a_ukulele_strums_four_strings_from_the_high_g() {
        let mut p = defaults(UKE_SPECS);
        p[HUMANIZE] = 0.0;
        let data = strum_default();
        let spb = 24000.0;
        let mut st = MidiFxState::new(MidiFxKind::Ukulele);
        let mut ons = Vec::new();
        for b in 0..(spb / 256.0) as u64 {
            let now = b * 256;
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / spb, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 4] };
            let mut out = Vec::new();
            st.process(MidiFxKind::Ukulele, &p, &data, &[], &mut out, &ctx);
            ons.extend(out.iter().filter(|e| e.on).map(|e| ((now + e.offset as u64) as f64 / spb, e.pitch)));
        }
        ons.sort_by(|a, b| a.0.total_cmp(&b.0));
        // C (0003): G, C, E, then the C on the A string.
        let first: Vec<u8> = ons.iter().filter(|o| o.0 < 0.2).map(|o| o.1).collect();
        assert_eq!(first, vec![67, 60, 64, 72]);
    }

    #[test]
    fn power_chords_are_three_strings() {
        let s = guitar_shape(4, &[4, 11], true, 0);
        assert_eq!(s, [Some(0), Some(2), Some(2), None, None, None]);
    }

    #[test]
    fn strokes_go_down_and_up() {
        let mut p = defaults(SPECS);
        p[HUMANIZE] = 0.0;
        let data = strum_default();
        let spb = 24000.0;
        let mut st = MidiFxState::new(MidiFxKind::GuitarChords);
        let mut ons = Vec::new();
        for b in 0..(4.0 * spb / 256.0) as u64 {
            let now = b * 256;
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / spb, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 4] };
            let mut out = Vec::new();
            st.process(MidiFxKind::GuitarChords, &p, &data, &[], &mut out, &ctx);
            ons.extend(out.iter().filter(|e| e.on).map(|e| ((now + e.offset as u64) as f64 / spb, e.pitch)));
        }
        ons.sort_by(|a, b| a.0.total_cmp(&b.0));
        // Beat 1: a down stroke of C (x32010), low to high, spread over the strum.
        let first: Vec<u8> = ons.iter().filter(|o| o.0 < 0.2).map(|o| o.1).collect();
        assert_eq!(first, vec![48, 52, 55, 60, 64]);
        let spread = ons.iter().filter(|o| o.0 < 0.2).map(|o| o.0).fold(0.0f64, f64::max);
        assert!(spread > 0.02 && spread < 0.06, "{spread}");
        // The "and" of 2: an up stroke of the top four, high to low.
        let up: Vec<u8> = ons.iter().filter(|o| o.0 >= 1.45 && o.0 < 1.7).map(|o| o.1).collect();
        assert_eq!(up, vec![64, 60, 55, 52]);
    }
}
