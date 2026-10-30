//! Home Rhythms: the rhythm section of the small home keyboards of the
//! eighties (and of the toy sampling ones). Forty preset rhythms, from Rock,
//! Disco and Swing through the ballroom and Latin dances to Reggae, Ska and
//! Dancehall Rock (the sparse rock preset with the syncopated bass that
//! dancehall riddims were built on), each with Variation A and B, a Fill In,
//! an Intro and an Ending, on the song's tempo.
//!
//! It plays the drums on the kit's notes (General MIDI; a drum the kit does
//! not have becomes the nearest it has), or, as the keyboards' auto
//! accompaniment, each rhythm's bass line and chords on a melodic track. The
//! chord comes from the keys held below the split (fingered, or single
//! finger: one key a major chord, two a minor, three a seventh, four a minor
//! seventh), and stays when they are let go; or from the song's key.
//!
//! The Fill In, Intro and Ending buttons are counters: each press moves one
//! on by one, and the rhythm hears it at its next step. A fill pressed with
//! more than a beat of the bar left plays from there to the bar's end, else
//! the next bar; Intro and Ending start at the next bar line. After the
//! Ending it is quiet until Intro or Fill In starts it again (or the song
//! starts over).

use super::SeqState;
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

pub const RHYTHM: usize = 0;
pub const VARIATION: usize = 1;
pub const PART: usize = 2;
pub const FILL_EVERY: usize = 3;
pub const START_INTRO: usize = 4;
pub const FILL_IN: usize = 5;
pub const INTRO: usize = 6;
pub const ENDING: usize = 7;
pub const CHORDS: usize = 8;
pub const SPLIT: usize = 9;
pub const BASS_NOTE: usize = 10;
pub const CHORD_NOTE: usize = 11;
pub const VELOCITY: usize = 12;

pub const VARIATIONS: &[&str] = &["A", "B"];
pub const PARTS: &[&str] = &["Drums", "Bass", "Chords", "Bass & Chords"];
pub const FILL_CHOICES: &[&str] = &["Off", "2 Bars", "4 Bars", "8 Bars"];
pub const CHORD_MODES: &[&str] = &["Fingered", "Single Finger", "Song Key"];

/// The rhythms' names, in the order of `RHYTHMS`.
pub const RHYTHM_NAMES: &[&str] = &[
    "Rock",
    "Slow Rock",
    "8 Beat",
    "16 Beat",
    "Pops",
    "Ballad",
    "Rock 'n' Roll",
    "Twist",
    "Country",
    "Latin Rock",
    "Disco",
    "Funk",
    "Hip Hop",
    "Electro",
    "Synth Pop",
    "Swing",
    "Shuffle",
    "Blues",
    "Boogie",
    "Jazz Waltz",
    "Big Band",
    "Fox Trot",
    "Waltz",
    "March",
    "Polka",
    "Tango",
    "Bolero",
    "Beguine",
    "Samba",
    "Bossa Nova",
    "Rhumba",
    "Cha-Cha",
    "Mambo",
    "Salsa",
    "Reggae",
    "Ska",
    "Steppers",
    "Dancehall Rock",
    "Calypso",
    "Soca",
];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Rhythm", "Rhythm", RHYTHM_NAMES, 0),
    ParamSpec::choice("Variation", "Rhythm", VARIATIONS, 0),
    ParamSpec::choice("Part", "Rhythm", PARTS, 0),
    ParamSpec::choice("Fill Every", "Sections", FILL_CHOICES, 3),
    ParamSpec::toggle("Start Intro", "Sections", false),
    ParamSpec::int("Fill In", "Sections", 0.0, 99.0, 0.0, Unit::None),
    ParamSpec::int("Intro", "Sections", 0.0, 99.0, 0.0, Unit::None),
    ParamSpec::int("Ending", "Sections", 0.0, 99.0, 0.0, Unit::None),
    ParamSpec::choice("Chords", "Accompaniment", CHORD_MODES, 0),
    ParamSpec::int("Split", "Accompaniment", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::int("Bass Note", "Accompaniment", 24.0, 52.0, 36.0, Unit::None),
    ParamSpec::int("Chord Note", "Accompaniment", 40.0, 72.0, 55.0, Unit::None),
    ParamSpec::lin("Velocity", "Rhythm", 0.05, 1.0, 0.85, Unit::Percent),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

// ---------------------------------------------------------------- lanes

pub const LANES: usize = 15;
pub const KICK: usize = 0;
pub const SNARE: usize = 1;
pub const RIM: usize = 2;
pub const CLAP: usize = 3;
pub const HAT: usize = 4;
pub const OPEN: usize = 5;
pub const CYMBAL: usize = 6;
pub const TOM_HI: usize = 7;
pub const TOM_LO: usize = 8;
pub const BELL: usize = 9;
pub const CLAVES: usize = 10;
pub const CONGA_HI: usize = 11;
pub const CONGA_LO: usize = 12;
pub const MARACAS: usize = 13;
pub const TAMB: usize = 14;

/// The letter of each lane in a rhythm's rows.
pub const LANE_CHARS: [char; LANES] = ['K', 'S', 'R', 'A', 'H', 'O', 'C', 'T', 'L', 'W', 'V', 'Q', 'U', 'M', 'B'];
pub const LANE_NAMES: [&str; LANES] = ["Bass Drum", "Snare", "Rim", "Clap", "Hi-Hat", "Open Hat", "Cymbal", "High Tom", "Low Tom", "Cowbell", "Claves", "High Conga", "Low Conga", "Maracas", "Tambourine"];

/// The drums each lane would like (General MIDI notes), best first.
pub const LANE_NOTES: [&[u8]; LANES] = [
    &[36, 35],
    &[38, 40, 39],
    &[37, 38],
    &[39, 38],
    &[42, 44, 46],
    &[46, 42],
    &[49, 57, 51, 46],
    &[48, 50, 47, 45],
    &[45, 43, 41, 47],
    &[56, 37],
    &[75, 37],
    &[62, 63, 60, 50],
    &[64, 61, 45],
    &[70, 69, 54, 42],
    &[54, 70, 42],
];

/// The note a lane plays in a kit (`kit`: a mask of the notes it has, 0
/// when unknown: then General MIDI).
pub fn lane_note(lane: usize, kit: u128) -> Option<u8> {
    let c = LANE_NOTES[lane.min(LANES - 1)];
    if kit == 0 {
        return Some(c[0]);
    }
    c.iter().copied().find(|n| kit >> n & 1 == 1)
}

// ---------------------------------------------------------------- rhythms

/// How a rhythm fills: down the toms, on the congas, or a snare that builds.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Fill {
    Rock,
    Latin,
    Swing,
}

pub struct Rhythm {
    pub name: &'static str,
    pub group: &'static str,
    /// Beats in a bar, and steps in a bar (sixteenths, or triplets).
    pub beats: usize,
    pub steps: usize,
    pub fill: Fill,
    /// Rows of steps, a bar or two each: a lane's letter (see `LANE_CHARS`),
    /// `.` off, `g` soft, `x` a hit, `X` an accent; `b` the bass (`1` the
    /// root, `3` the third, `5` the fifth, `6` the sixth, `7` the seventh,
    /// `8` the octave, `2` and `4`, `L` the fifth below; each lasts to the
    /// next), `c` the chords (`x` short, `X` to the next).
    pub a: &'static [(char, &'static str)],
    /// Variation B's rows; a lane it has not is A's, a little busier.
    pub b: &'static [(char, &'static str)],
}

pub const GROUPS: &[&str] = &["Rock & Pop", "Dance", "Jazz & Blues", "Ballroom & Folk", "Latin", "Caribbean"];

const fn r(name: &'static str, group: &'static str, beats: usize, steps: usize, fill: Fill, a: &'static [(char, &'static str)], b: &'static [(char, &'static str)]) -> Rhythm {
    Rhythm { name, group, beats, steps, fill, a, b }
}

use Fill::{Latin, Rock, Swing};

pub static RHYTHMS: &[Rhythm] = &[
    r("Rock", "Rock & Pop", 4, 16, Rock, &[('K', "x.......x.x....."), ('S', "....X.......X..."), ('H', "x.x.x.x.x.x.x.x."), ('b', "1.1.1.1.5.5.6.5."), ('c', "x...x.x.x...x.x.")], &[]),
    r("Slow Rock", "Rock & Pop", 4, 12, Swing, &[('K', "x.....x..x.."), ('S', "...X.....X.."), ('H', "xxxxxxxxxxxx"), ('b', "1.....5..8.."), ('c', "xxxxxxxxxxxx")], &[]),
    r("8 Beat", "Rock & Pop", 4, 16, Rock, &[('K', "x.....x.x......."), ('S', "....X.......X..."), ('H', "x.x.x.x.x.x.x.x."), ('b', "1...1..51...5..."), ('c', "X.......X.......")], &[]),
    r("16 Beat", "Rock & Pop", 4, 16, Rock, &[('K', "x.....x...x....."), ('S', "....X.......X..."), ('H', "XxxxXxxxXxxxXxxx"), ('b', "1..1..1.5..5..8."), ('c', "x..x..x...x.x...")], &[]),
    r("Pops", "Rock & Pop", 4, 16, Rock, &[('K', "x.......x...x..."), ('S', "....X.......X..."), ('H', "x.x.x.x.x.x.x.x."), ('B', "....x.......x..."), ('b', "1...5...1...5..."), ('c', "x.x.x.x.x.x.x.x.")], &[]),
    r("Ballad", "Rock & Pop", 4, 16, Rock, &[('K', "x.........x....."), ('R', "....x.......x..."), ('H', "x.x.x.x.x.x.x.x."), ('b', "1.......5......."), ('c', "X.......X.......")], &[('K', "x.........x....."), ('S', "....x.......x..."), ('H', "x.x.x.x.x.x.x.x."), ('O', "..............x.")]),
    r("Rock 'n' Roll", "Rock & Pop", 4, 12, Swing, &[('K', "x.....x....."), ('S', "...X.....X.."), ('H', "x.xx.xx.xx.x"), ('b', "1.15.56.65.3"), ('c', "...x.....x..")], &[]),
    r("Twist", "Rock & Pop", 4, 16, Rock, &[('K', "x...x...x...x..."), ('S', "....X.......X..."), ('H', "x.x.x.x.x.x.x.x."), ('b', "1.1.3.3.5.5.3.3."), ('c', "..x...x...x...x.")], &[]),
    r("Country", "Rock & Pop", 4, 16, Rock, &[('K', "x.......x......."), ('S', "....x.......x..."), ('H', "x.x.x.x.x.x.x.x."), ('b', "1.......5......."), ('c', "....x.x.....x.x.")], &[]),
    r("Latin Rock", "Rock & Pop", 4, 16, Latin, &[('K', "x.......x.x....."), ('S', "....X.......X..."), ('W', "x...x...x...x..."), ('Q', "..x...x...x...x."), ('H', "x.x.x.x.x.x.x.x."), ('b', "1..1..5.1..1..5."), ('c', "x..x..x...x..x..")], &[]),
    r("Disco", "Dance", 4, 16, Rock, &[('K', "x...x...x...x..."), ('S', "....X.......X..."), ('H', "x...x...x...x..."), ('O', "..x...x...x...x."), ('b', "1.8.1.8.1.8.1.8."), ('c', "..x...x...x...x.")], &[]),
    r("Funk", "Dance", 4, 16, Rock, &[('K', "x.x.......x..x.."), ('S', "....X..g.g..X..g"), ('H', "XxxxXxxxXxxxXxxx"), ('b', "1..18..7.1.5..3."), ('c', "..x..x....x..x..")], &[]),
    r("Hip Hop", "Dance", 4, 16, Rock, &[('K', "x......x..x....."), ('S', "....X.......X..."), ('H', "x.x.x.x.x.x.x.x."), ('b', "1......1..5....."), ('c', "X...............")], &[]),
    r("Electro", "Dance", 4, 16, Rock, &[('K', "x.....x...x....."), ('A', "....X.......X..."), ('H', "x.x.x.x.x.x.x.x."), ('W', "......x.......x."), ('b', "1.1...1.1...5.8."), ('c', "x..x..x.........")], &[]),
    r("Synth Pop", "Dance", 4, 16, Rock, &[('K', "x...x...x...x..."), ('A', "....X.......X..."), ('H', "xxxxxxxxxxxxxxxx"), ('b', "1.1.1.1.1.1.5.5."), ('c', "x..x..x...x.x...")], &[]),
    r("Swing", "Jazz & Blues", 4, 12, Swing, &[('K', "g..g..g..g.."), ('S', ".....g.....g"), ('H', "x..x.xx..x.x"), ('b', "1..3..5..6.."), ('c', "...x.....x..")], &[]),
    r("Shuffle", "Jazz & Blues", 4, 12, Swing, &[('K', "x.....x....."), ('S', "...X.....X.."), ('H', "x.xx.xx.xx.x"), ('b', "1.13.35.56.5"), ('c', "...x.x...x.x")], &[]),
    r("Blues", "Jazz & Blues", 4, 12, Swing, &[('K', "x.....x....."), ('S', "...X.....X.."), ('H', "xxxxxxxxxxxx"), ('b', "1..5..6..7.."), ('c', "x.xx.xx.xx.x")], &[]),
    r("Boogie", "Jazz & Blues", 4, 12, Swing, &[('K', "x..x..x..x.."), ('S', "...X.....X.."), ('H', "x.xx.xx.xx.x"), ('b', "1.35.67.65.3"), ('c', "...x.x...x.x")], &[]),
    r("Jazz Waltz", "Jazz & Blues", 3, 9, Swing, &[('K', "x........"), ('S', "....g...."), ('H', "x..x.xx.."), ('b', "1..5..3.."), ('c', "...x..x..")], &[]),
    r("Big Band", "Jazz & Blues", 4, 12, Swing, &[('K', "x..x..x..x.."), ('S', "...x.....x.."), ('H', "x..x.xx..x.x"), ('b', "1..3..5..8.."), ('c', "...x.....x.x")], &[]),
    r("Fox Trot", "Jazz & Blues", 4, 16, Rock, &[('K', "x.......x......."), ('S', "....x.......x..."), ('H', "x...x...x...x..."), ('b', "1.......5......."), ('c', "....x.......x...")], &[]),
    r("Waltz", "Ballroom & Folk", 3, 12, Rock, &[('K', "x...........x..........."), ('S', "....x...x.......x...x..."), ('H', "x...x...x..."), ('b', "1...........5..........."), ('c', "....x...x.......x...x...")], &[]),
    r("March", "Ballroom & Folk", 4, 16, Rock, &[('K', "x.......x......."), ('S', "....x.......x.x."), ('C', "x..............."), ('b', "1.......5......."), ('c', "....x.......x...")], &[]),
    r("Polka", "Ballroom & Folk", 2, 8, Rock, &[('K', "x...x..."), ('S', "..x...x."), ('H', "x.x.x.x."), ('b', "1...5..."), ('c', "..x...x.")], &[]),
    r("Tango", "Ballroom & Folk", 4, 16, Latin, &[('K', "X...x...x...x.x."), ('R', "x...x...x...x.x."), ('b', "1...1...5...5.5."), ('c', "x...x...x...x.x.")], &[]),
    r("Bolero", "Ballroom & Folk", 4, 16, Latin, &[('K', "x.......x......."), ('R', "....x.......x..."), ('M', "x.x.x.x.x.x.x.x."), ('Q', "......x.x.....x."), ('U', ".x.x.......x...."), ('b', "1.....5.1.....5."), ('c', "x..x..x...x..x..")], &[]),
    r("Beguine", "Ballroom & Folk", 4, 16, Latin, &[('K', "x.......x......."), ('L', "......x.x.....x."), ('T', "...x.......x...."), ('V', "x..x..x...x.x..."), ('H', "x.x.x.x.x.x.x.x."), ('b', "1.....5.1......."), ('c', "...x..x....x..x.")], &[]),
    r("Samba", "Latin", 4, 16, Latin, &[('K', "x..xx..xx..xx..x"), ('S', "g.g..x.g.g..x.g."), ('H', "XggxXggxXggxXggx"), ('B', "xgxgxgxgxgxgxgxg"), ('b', "1..1..5.1..1..5."), ('c', "x..x..x...x..x..")], &[]),
    r("Bossa Nova", "Latin", 4, 16, Latin, &[('K', "x..xx..xx..xx..x"), ('R', "x..x..x...x..x....x..x...x..x..."), ('H', "x.x.x.x.x.x.x.x."), ('b', "1.....5.1.....5."), ('c', "x..x..x...x..x...x..x..x..x.x...")], &[]),
    r("Rhumba", "Latin", 4, 16, Latin, &[('K', "x.......x......."), ('V', "x..x..x...x.x..."), ('Q', "....x.x.....x.x."), ('U', "......x.......xx"), ('M', "x.x.x.x.x.x.x.x."), ('b', "1..5..5.1..5..5."), ('c', "..x..x....x..x..")], &[]),
    r("Cha-Cha", "Latin", 4, 16, Latin, &[('K', "x.......x......."), ('W', "x...x...x...x..."), ('T', "............x.x."), ('M', "x.x.x.x.x.x.x.x."), ('U', "......x.......x."), ('b', "1.......5...5.5."), ('c', "x...x...x...x.x.")], &[]),
    r("Mambo", "Latin", 4, 16, Latin, &[('K', "x.......x......."), ('W', "x.x.x.xxx.xx.x.x"), ('V', "..x.x...x..x..x."), ('Q', "..x.x.x...x.x.x."), ('U', "......xx......xx"), ('b', "...1..5....1..5."), ('c', "x.x..x.x..x..x.x")], &[]),
    r("Salsa", "Latin", 4, 16, Latin, &[('K', "x.......x......."), ('V', "x.....x.....x.......x...x......."), ('W', "x.x.x.x.x.x.x.x."), ('Q', "....x.x.....x.x."), ('U', "......xx......xx"), ('b', "...5..1....5..1."), ('c', "x.x..x.x..x.x.x.x.x..x.x..x.x.x.")], &[]),
    r("Reggae", "Caribbean", 4, 12, Swing, &[('K', "......X....."), ('R', "......x....."), ('H', "x.xx.xx.xx.x"), ('b', "1.....5.3..."), ('c', "...x.....x..")], &[]),
    r("Ska", "Caribbean", 4, 16, Rock, &[('K', "x.......x......."), ('S', "....X.......X..."), ('H', "x.x.x.x.x.x.x.x."), ('b', "1.3.5.6.8.6.5.3."), ('c', "..x...x...x...x.")], &[]),
    r("Steppers", "Caribbean", 4, 16, Rock, &[('K', "x...x...x...x..."), ('R', "....x.......x..."), ('H', "..x...x...x...x."), ('b', "1...1..51...1..5"), ('c', "..x...x...x...x.")], &[]),
    r(
        "Dancehall Rock",
        "Caribbean",
        4,
        16,
        Rock,
        &[('K', "x.......x......."), ('S', "....x.......x..."), ('H', "x.x.x.x.x.x.x.x."), ('b', "1..1..8.1..7..5.1..1..8.1.5.3..."), ('c', "....x.......x...")],
        &[('K', "x.......x.x.....x.......x.x....."), ('O', "..............x...............x."), ('H', "x.x.x.x.x.x.x...x.x.x.x.x.x.x..."), ('V', "...x..x....x.......x..x....x....")],
    ),
    r("Calypso", "Caribbean", 4, 16, Latin, &[('K', "x.......x......."), ('R', "...x..x....x..x."), ('W', "x...x...x...x..."), ('Q', "..x...x...x...x."), ('b', "1...5..51...5..5"), ('c', "x..x..x...x.x...")], &[]),
    r("Soca", "Caribbean", 4, 16, Rock, &[('K', "x...x...x...x..."), ('S', "...x..x....x..x."), ('H', "x...x...x...x..."), ('O', "..x...x...x...x."), ('b', "1..1..1.5..5..5."), ('c', "..x...x...x...x.")], &[]),
];

fn pick(p: &[f32], i: usize, n: usize) -> usize {
    (p.get(i).copied().unwrap_or(0.0).round().max(0.0) as usize).min(n - 1)
}

/// The rhythm a device plays.
pub fn rhythm_of(p: &[f32]) -> &'static Rhythm {
    &RHYTHMS[pick(p, RHYTHM, RHYTHMS.len())]
}

/// Beats a step lasts.
pub fn step_beats(p: &[f32]) -> f64 {
    let r = rhythm_of(p);
    r.beats as f64 / r.steps as f64
}

/// Bars before the rhythm repeats (its longest row).
pub fn bars(r: &Rhythm) -> usize {
    r.a.iter().chain(r.b.iter()).map(|(_, s)| s.len()).max().unwrap_or(r.steps).div_ceil(r.steps).max(1)
}

fn row(rows: &[(char, &'static str)], c: char) -> Option<&'static [u8]> {
    rows.iter().find(|(k, _)| *k == c).map(|(_, s)| s.as_bytes()).filter(|s| !s.is_empty())
}

fn level(ch: u8) -> u8 {
    match ch {
        b'X' => 3,
        b'x' => 2,
        b'g' => 1,
        _ => 0,
    }
}

/// How hard lane `lane` plays at step `idx` of the pattern (0 off, 1 soft, 2 a hit, 3 an accent).
pub fn drum_level(r: &Rhythm, var: usize, lane: usize, idx: usize) -> u8 {
    let c = LANE_CHARS[lane.min(LANES - 1)];
    if var == 1 {
        if let Some(s) = row(r.b, c) {
            return level(s[idx % s.len()]);
        }
    }
    let a = row(r.a, c).map(|s| level(s[idx % s.len()])).unwrap_or(0);
    if var == 0 {
        return a;
    }
    // B: sixteenth hats where A has eighths, an open hat before the bar
    // line, a tambourine on the backbeat.
    let n = bars(r) * r.steps;
    let open_at = n - (r.steps / (r.beats * 2)).max(1);
    let j = idx % n;
    let open_here = row(r.a, 'O').is_none() && row(r.b, 'O').is_none() && j == open_at;
    match lane {
        HAT if open_here => 0,
        HAT if a == 0 && r.steps == 16 && idx % 2 == 1 => match row(r.a, 'H') {
            Some(h) if h.iter().enumerate().all(|(i, &ch)| i % 2 == 0 || ch == b'.') && h.iter().any(|&ch| ch != b'.') => 1,
            _ => 0,
        },
        OPEN if open_here && row(r.a, 'H').is_some_and(|h| h[j % h.len()] != b'.') => 2,
        TAMB if a == 0 && row(r.a, 'B').is_none() => row(r.a, 'S').map(|s| if level(s[idx % s.len()]) >= 2 { 2 } else { 0 }).unwrap_or(0),
        _ => a,
    }
}

/// The accompaniment's rows: `b` (bass) or `c` (chords), B's if it has them.
fn acc_row(r: &Rhythm, var: usize, c: char) -> Option<&'static [u8]> {
    if var == 1 {
        if let Some(s) = row(r.b, c) {
            return Some(s);
        }
    }
    row(r.a, c)
}

/// Steps from `idx` to the next note of a row (at most a bar).
fn to_next(s: &[u8], idx: usize, steps: usize) -> usize {
    (1..=steps).find(|&d| !matches!(s[(idx + d) % s.len()], b'.' | b'-')).unwrap_or(steps)
}

// ---------------------------------------------------------------- chords

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Quality {
    #[default]
    Major,
    Minor,
    Seventh,
    Minor7,
    Major7,
    Dim,
    Aug,
    Sus4,
}

impl Quality {
    pub const ALL: [Quality; 8] = [Quality::Major, Quality::Minor, Quality::Seventh, Quality::Minor7, Quality::Major7, Quality::Dim, Quality::Aug, Quality::Sus4];

    pub fn tones(self) -> &'static [i32] {
        match self {
            Quality::Major => &[0, 4, 7],
            Quality::Minor => &[0, 3, 7],
            Quality::Seventh => &[0, 4, 7, 10],
            Quality::Minor7 => &[0, 3, 7, 10],
            Quality::Major7 => &[0, 4, 7, 11],
            Quality::Dim => &[0, 3, 6],
            Quality::Aug => &[0, 4, 8],
            Quality::Sus4 => &[0, 5, 7],
        }
    }

    /// As written after the root (C, Cm, C7...).
    pub fn suffix(self) -> &'static str {
        match self {
            Quality::Major => "",
            Quality::Minor => "m",
            Quality::Seventh => "7",
            Quality::Minor7 => "m7",
            Quality::Major7 => "maj7",
            Quality::Dim => "dim",
            Quality::Aug => "aug",
            Quality::Sus4 => "sus4",
        }
    }

    /// Semitones above the root a bass step plays (`None`: a rest).
    fn bass(self, ch: u8) -> Option<i32> {
        let t = self.tones();
        Some(match ch {
            b'1' => 0,
            b'2' => 2,
            b'3' => t[1],
            b'4' => 5,
            b'5' => t[2],
            b'6' => 9,
            b'7' => t.get(3).copied().unwrap_or(10),
            b'8' => 12,
            b'L' => t[2] - 12,
            _ => return None,
        })
    }
}

/// The chord the keys held make: their root and kind. Fingered: the
/// shape that fits (any inversion), else the lowest key and its thirds;
/// single finger: the lowest key, and how many are held.
pub fn recognize(held: u128, single: bool) -> Option<(u8, Quality)> {
    if held == 0 {
        return None;
    }
    let low = held.trailing_zeros() as u8;
    if single {
        let q = match held.count_ones() {
            1 => Quality::Major,
            2 => Quality::Minor,
            3 => Quality::Seventh,
            _ => Quality::Minor7,
        };
        return Some((low % 12, q));
    }
    let mut pcs = 0u16;
    for n in 0..128u32 {
        if held >> n & 1 == 1 {
            pcs |= 1 << (n % 12);
        }
    }
    let rotated = |root: u8| (0..12).fold(0u16, |m, i| if pcs >> ((root as u16 + i) % 12) & 1 == 1 { m | 1 << i } else { m });
    let shape = |q: Quality| q.tones().iter().fold(0u16, |m, t| m | 1 << t);
    // The lowest key first, then the others, as the root.
    let mut roots = [low % 12; 13];
    let mut n = 1;
    for pc in 0..12u8 {
        if pcs >> pc & 1 == 1 && pc != low % 12 {
            roots[n] = pc;
            n += 1;
        }
    }
    for &root in &roots[..n] {
        let m = rotated(root);
        if let Some(q) = Quality::ALL.into_iter().find(|q| shape(*q) == m) {
            return Some((root, q));
        }
        // A seventh without its fifth, two notes of a triad.
        if m == 0b100_0001_0001 {
            return Some((root, Quality::Seventh));
        }
    }
    let m = rotated(low % 12);
    let has = |i: u16| m >> i & 1 == 1;
    let q = if has(3) && !has(4) { Quality::Minor } else { Quality::Major };
    Some((low % 12, q))
}

/// The song's key as a chord: its root, minor when its third is.
pub fn key_chord(ctx: &MidiCtx) -> (u8, Quality) {
    let root = ctx.scale.root % 12;
    let minor = ctx.scale.contains(60 + root + 3) && !ctx.scale.contains(60 + root + 4);
    (root, if minor { Quality::Minor } else { Quality::Major })
}

/// The chord's notes from `base` up (each tone in the octave above it).
pub fn voicing(root: u8, q: Quality, base: i32, out: &mut [i32; 4]) -> usize {
    let t = q.tones();
    for (i, tone) in t.iter().enumerate() {
        let pc = (root as i32 + tone).rem_euclid(12);
        out[i] = base + (pc - base).rem_euclid(12);
    }
    t.len()
}

// ---------------------------------------------------------------- playing

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Section {
    #[default]
    Main,
    Fill,
    Intro,
    Ending,
    Stopped,
}

impl Section {
    pub fn code(self) -> u64 {
        match self {
            Section::Main => 0,
            Section::Fill => 1,
            Section::Intro => 2,
            Section::Ending => 3,
            Section::Stopped => 4,
        }
    }
}

const WANT_INTRO: u8 = 1;
const WANT_FILL: u8 = 2;
const WANT_ENDING: u8 = 4;

/// What the rhythm section remembers between steps.
#[derive(Clone, Copy, Default)]
pub struct HomeState {
    /// Whether it has played since the start, the bar it is in and the step before.
    started: bool,
    bar: i64,
    last_k: i64,
    section: Section,
    /// The fill plays from this step of its bar; the intro's bar (0 or 1).
    fill_from: usize,
    intro_bar: u8,
    /// A cymbal on the next bar's one (after a fill or the intro).
    crash: bool,
    /// Presses waiting for the next bar line.
    want: u8,
    /// The Fill In, Intro and Ending counters as last seen.
    seen: Option<[u8; 3]>,
    /// The keys held below the split, and the chord they made (it stays).
    held: u128,
    pub chord: Option<(u8, Quality)>,
}

impl HomeState {
    /// The song starts over: the sections do too (the chord stays).
    pub fn restart(&mut self) {
        let (seen, chord) = (self.seen, self.chord);
        *self = Self { seen, chord, ..Self::default() };
    }
}

impl SeqState {
    /// Keys played on an accompaniment's track: those below the split make
    /// the chord (and do not sound), the rest pass. The drums let all pass.
    pub(crate) fn home_keys(&mut self, p: &[f32], out: &mut Vec<MidiEvent>) {
        if pick(p, PART, PARTS.len()) == 0 {
            return;
        }
        let split = p.get(SPLIT).map(|v| v.round() as i32).unwrap_or(60);
        let h = &mut self.home;
        let before = h.held;
        out.retain(|e| {
            let bit = 1u128 << (e.pitch & 127);
            if e.on && (e.pitch as i32) < split {
                h.held |= bit;
                false
            } else if !e.on && h.held & bit != 0 {
                h.held &= !bit;
                false
            } else {
                true
            }
        });
        if h.held & !before != 0 {
            h.chord = recognize(h.held, pick(p, CHORDS, CHORD_MODES.len()) == 1);
        }
    }

    /// One step of the rhythm (its own grid: sixteenths or triplets).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn home(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let ri = pick(p, RHYTHM, RHYTHMS.len());
        let r = &RHYTHMS[ri];
        let steps = r.steps;
        let spb = (steps / r.beats).max(1);
        let bar_no = k.div_euclid(steps as i64);
        let j = k.rem_euclid(steps as i64) as usize;
        let var = pick(p, VARIATION, 2);
        let h = &mut self.home;

        // A jump in the song (a loop, a locate, a start) starts over.
        if !h.started || k < h.last_k || k > h.last_k + steps as i64 {
            h.started = true;
            h.section = Section::Main;
            h.bar = bar_no - 1;
            h.crash = false;
            h.want = 0;
            if bar_no == 0 && j == 0 && p[START_INTRO] >= 0.5 {
                h.want |= WANT_INTRO;
            }
        }
        h.last_k = k;
        // The buttons: a counter moved on by one is a press.
        let counts = [FILL_IN, INTRO, ENDING].map(|i| p.get(i).map(|v| v.round().clamp(0.0, 255.0) as u8).unwrap_or(0));
        let seen = h.seen.unwrap_or(counts);
        h.seen = Some(counts);
        let pressed = |i: usize| (counts[i] as i32 - seen[i] as i32).rem_euclid(100) == 1;
        if pressed(1) {
            h.want |= WANT_INTRO;
        }
        if pressed(2) && h.section != Section::Stopped {
            h.want |= WANT_ENDING;
        }
        if pressed(0) {
            if h.section == Section::Main && bar_no == h.bar && j + spb < steps {
                h.section = Section::Fill;
                h.fill_from = j;
            } else {
                h.want |= WANT_FILL;
            }
        }
        // A new bar: what plays in it.
        if bar_no != h.bar {
            let prev = h.section;
            let prev_intro = h.intro_bar;
            h.bar = bar_no;
            h.crash = prev == Section::Fill || (prev == Section::Intro && prev_intro == 1);
            let every = [0i64, 2, 4, 8][pick(p, FILL_EVERY, FILL_CHOICES.len())];
            h.section = if h.want & WANT_INTRO != 0 {
                h.intro_bar = 0;
                Section::Intro
            } else if prev == Section::Intro && prev_intro == 0 {
                h.intro_bar = 1;
                Section::Intro
            } else if h.want & WANT_ENDING != 0 {
                Section::Ending
            } else if matches!(prev, Section::Ending | Section::Stopped) && h.want & WANT_FILL == 0 {
                Section::Stopped
            } else if h.want & WANT_FILL != 0 || (every > 0 && (bar_no + 1).rem_euclid(every) == 0) {
                h.fill_from = steps / 2;
                Section::Fill
            } else {
                Section::Main
            };
            if matches!(h.section, Section::Intro | Section::Stopped) {
                h.crash = false;
            }
            h.want = 0;
        }

        // The drums of this step.
        let pattern = bars(r) * steps;
        let idx = (bar_no.rem_euclid(bars(r) as i64) as usize * steps + j) % pattern;
        let section = h.section;
        let count_in = section == Section::Intro && h.intro_bar == 0;
        let mut hits: [u8; LANES] = std::array::from_fn(|l| drum_level(r, var, l, idx));
        // The ending: the groove, a fill over the beat before the last, one last hit.
        let last = steps - spb;
        let fill_from = match section {
            Section::Fill => Some(h.fill_from),
            Section::Intro if h.intro_bar == 1 => Some(steps / 2),
            Section::Ending => Some(last.saturating_sub(spb)),
            _ => None,
        };
        let mut final_hit = false;
        match section {
            Section::Stopped => hits = [0; LANES],
            Section::Intro if h.intro_bar == 0 => {
                // A count in: the hats on the beats.
                hits = [0; LANES];
                if j % spb == 0 {
                    hits[HAT] = 2;
                }
                if j == 0 {
                    hits[KICK] = 3;
                    final_hit = true;
                }
            }
            Section::Ending if j >= last => {
                hits = [0; LANES];
                if j == last {
                    hits[KICK] = 3;
                    hits[CYMBAL] = 3;
                    final_hit = true;
                }
            }
            _ => {}
        }
        if let Some(from) = fill_from.filter(|f| j >= *f && !final_hit && section != Section::Stopped && (section != Section::Ending || j < last)) {
            fill_hits(r.fill, var, &mut hits, j, from, steps, spb);
        }
        if h.crash && j == 0 && matches!(section, Section::Main | Section::Fill) {
            hits[CYMBAL] = 3;
            hits[KICK] = hits[KICK].max(2);
            hits[HAT] = 0;
        }
        let vel = p[VELOCITY].clamp(0.05, 1.0);
        let part = pick(p, PART, PARTS.len());
        let mut shown = 0u64;
        if part == 0 {
            for (lane, &lv) in hits.iter().enumerate() {
                if lv == 0 {
                    continue;
                }
                let Some(note) = lane_note(lane, ctx.drum_notes) else { continue };
                shown |= 1 << lane;
                let level = [0.0, 0.42, 0.75, 1.0][lv.min(3) as usize];
                self.play(ctx, at, note as i32, vel * level, (step_frames * 0.5).max(16.0), out);
            }
        } else if section != Section::Stopped {
            // The accompaniment: the chord held, or the song's key.
            let h = &self.home;
            let (root, q) = match (pick(p, CHORDS, CHORD_MODES.len()), h.chord) {
                (0 | 1, Some(c)) => c,
                _ => key_chord(ctx),
            };
            let bass_root = {
                let base = p[BASS_NOTE].round() as i32;
                base + (root as i32 - base).rem_euclid(12)
            };
            let chord_base = p[CHORD_NOTE].round() as i32;
            let (bass, chords) = (part != 2, part != 1);
            // The count in and the ending's last hit hold one chord.
            if final_hit || count_in || (section == Section::Ending && j > last) {
                if final_hit {
                    let len = (steps - j) as f64 * step_frames * 0.95;
                    if bass {
                        self.play(ctx, at, bass_root, vel * 0.85, len, out);
                    }
                    if chords {
                        let mut notes = [0i32; 4];
                        let n = voicing(root, q, chord_base, &mut notes);
                        for &nt in &notes[..n] {
                            self.play(ctx, at, nt, vel * 0.6, len, out);
                        }
                    }
                }
            } else {
                if bass {
                    if let Some(s) = acc_row(r, var, 'b') {
                        if let Some(off) = q.bass(s[idx % s.len()]) {
                            let len = to_next(s, idx % s.len(), steps) as f64 * step_frames * 0.92;
                            self.play(ctx, at, bass_root + off, vel * 0.85, len, out);
                        }
                    }
                }
                if chords {
                    if let Some(s) = acc_row(r, var, 'c') {
                        let ch = s[idx % s.len()];
                        if matches!(ch, b'x' | b'X') {
                            let len = (if ch == b'X' { to_next(s, idx % s.len(), steps) as f64 * 0.92 } else { 0.6 }) * step_frames;
                            let mut notes = [0i32; 4];
                            let n = voicing(root, q, chord_base, &mut notes);
                            for &nt in &notes[..n] {
                                self.play(ctx, at, nt, vel * if ch == b'X' { 0.65 } else { 0.55 }, len, out);
                            }
                        }
                    }
                }
            }
        }
        let h = &self.home;
        let (root, q) = h.chord.unwrap_or_else(|| key_chord(ctx));
        let q_i = Quality::ALL.iter().position(|x| *x == q).unwrap_or(0) as u64;
        self.visual[0] = j as u64 | (steps as u64) << 8 | section.code() << 16 | (h.intro_bar as u64) << 19 | (var as u64) << 20 | (ri as u64) << 24 | (bar_no.max(0) as u64 & 0xffff) << 32 | (h.want as u64) << 48 | (h.fill_from as u64 & 0x3f) << 52;
        self.visual[1] = shown | (root as u64) << 16 | q_i << 20 | (h.chord.is_some() as u64) << 24;
    }
}

/// A fill's hits at step `j` (it runs from `from` to the bar's end): the
/// hands leave the groove, the bass drum keeps the beats.
fn fill_hits(kind: Fill, var: usize, hits: &mut [u8; LANES], j: usize, from: usize, steps: usize, spb: usize) {
    let kick = hits[KICK];
    *hits = [0; LANES];
    if j % spb == 0 {
        hits[KICK] = kick;
    }
    let t = (j - from) as f32 / (steps - from).max(1) as f32;
    let build = if t < 0.34 { 1 } else if t < 0.67 { 2 } else { 3 };
    match (kind, var) {
        (Fill::Latin, 0) => {
            hits[if t < 0.5 { CONGA_HI } else { CONGA_LO }] = if (j - from) % 2 == 0 { 3 } else { 2 };
        }
        (Fill::Swing, 0) | (Fill::Rock, 1) => hits[SNARE] = build,
        _ => {
            let lane = if t < 0.5 { SNARE } else if t < 0.75 { TOM_HI } else { TOM_LO };
            hits[lane] = if j == from { 3 } else { 2 };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::MidiFxKind;
    use crate::scale::{Scale, ScaleKind};

    const SPB: u64 = 24000;

    fn ctx(now: u64, kit: u128, scale: Scale) -> MidiCtx {
        MidiCtx { now, len: 256, samples_per_beat: SPB as f64, scale, scale_on: true, beat: now as f64 / SPB as f64, playing: true, sr: 48000.0, drum_notes: kit, lead: 0.0, touch: [0.0; 5] }
    }

    fn plain(rhythm: &str) -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        p[RHYTHM] = RHYTHM_NAMES.iter().position(|n| *n == rhythm).unwrap() as f32;
        p[FILL_EVERY] = 0.0;
        p
    }

    /// Plays for `beats`, `change` changing the params (and the keys) at a frame: (frame, note, velocity).
    fn run(p: &[f32], beats: u64, change: &mut dyn FnMut(u64, &mut Vec<f32>, &mut Vec<MidiEvent>)) -> Vec<(u64, u8, f32)> {
        let mut s = SeqState::new();
        let mut p = p.to_vec();
        let mut notes = Vec::new();
        let mut now = 0;
        while now < beats * SPB {
            let mut out = Vec::new();
            change(now, &mut p, &mut out);
            s.process(MidiFxKind::HomeRhythms, &p, &[], &mut out, &ctx(now, 0, Scale::new(0, ScaleKind::Major)));
            notes.extend(out.iter().filter(|e| e.on).map(|e| (now + e.offset as u64, e.pitch, e.vel)));
            now += 256;
        }
        notes
    }

    /// Whether the block from `now` holds `frame`.
    fn during(now: u64, frame: u64) -> bool {
        now <= frame && frame < now + 256
    }

    fn bar_of(r: &Rhythm, frame: u64) -> u64 {
        frame / (r.beats as u64 * SPB)
    }

    #[test]
    fn every_rhythm_reads_and_has_its_name() {
        assert_eq!(RHYTHMS.len(), RHYTHM_NAMES.len());
        assert!(RHYTHMS.len() >= 30);
        for (i, r) in RHYTHMS.iter().enumerate() {
            assert_eq!(r.name, RHYTHM_NAMES[i]);
            assert!(GROUPS.contains(&r.group), "{}", r.name);
            assert_eq!(r.steps % r.beats, 0, "{}", r.name);
            for &(c, s) in r.a.iter().chain(r.b.iter()) {
                assert!(LANE_CHARS.contains(&c) || c == 'b' || c == 'c', "{}: row {c}", r.name);
                assert_eq!(s.len() % r.steps, 0, "{}: row {c} is {} long", r.name, s.len());
                let ok = match c {
                    'b' => "12345678L.-",
                    'c' => "xX.-",
                    _ => ".gxX",
                };
                assert!(s.chars().all(|ch| ok.contains(ch)), "{}: row {c}", r.name);
            }
            assert!(row(r.a, 'b').is_some() && row(r.a, 'c').is_some(), "{}: accompaniment", r.name);
        }
    }

    #[test]
    fn each_rhythm_plays_its_drums_on_their_notes() {
        for r in RHYTHMS {
            let notes = run(&plain(r.name), (bars(r) * r.beats) as u64 * 2, &mut |_, _, _| {});
            assert!(!notes.is_empty(), "{}", r.name);
            // Every lane it has sounds on its General MIDI note, and nothing else does.
            for (lane, &c) in LANE_CHARS.iter().enumerate() {
                let note = LANE_NOTES[lane][0];
                let has = row(r.a, c).is_some_and(|s| s.iter().any(|&ch| level(ch) > 0));
                assert_eq!(notes.iter().any(|n| n.1 == note), has, "{}: lane {c}", r.name);
            }
            for n in &notes {
                assert!(n.2 > 0.0 && n.2 <= 1.0 && n.2.is_finite());
            }
        }
        // Rock: the bass drum on one, the snare on two.
        let notes = run(&plain("Rock"), 4, &mut |_, _, _| {});
        assert!(notes.iter().any(|n| n.0 == 0 && n.1 == 36));
        assert!(notes.iter().any(|n| n.0.abs_diff(SPB) <= 1 && n.1 == 38));
    }

    #[test]
    fn fills_come_every_few_bars_and_a_crash_after() {
        let mut p = plain("Rock");
        p[FILL_EVERY] = 1.0; // 2 bars
        let notes = run(&p, 16, &mut |_, _, _| {});
        let toms = |bar: u64| notes.iter().any(|n| n.0 / (4 * SPB) == bar && (n.1 == 48 || n.1 == 45));
        assert!(!toms(0) && toms(1) && !toms(2) && toms(3));
        // The toms in the second half only.
        assert!(notes.iter().filter(|n| n.1 == 48 || n.1 == 45).all(|n| n.0 % (4 * SPB) >= 2 * SPB - 1));
        assert!(notes.iter().any(|n| n.0 == 8 * SPB && n.1 == 49), "a crash after the fill");
        // Variation B fills with the snare instead.
        p[VARIATION] = 1.0;
        let notes = run(&p, 8, &mut |_, _, _| {});
        assert!(!notes.iter().any(|n| n.1 == 48 || n.1 == 45));
    }

    #[test]
    fn the_fill_button_fills_the_rest_of_the_bar() {
        let p = plain("Rock");
        // Pressed a quarter into the second bar.
        let notes = run(&p, 12, &mut |now, p, _| {
            if during(now, 5 * SPB) {
                p[FILL_IN] += 1.0;
            }
        });
        let toms: Vec<u64> = notes.iter().filter(|n| n.1 == 48 || n.1 == 45).map(|n| n.0).collect();
        assert!(!toms.is_empty() && toms.iter().all(|&f| (5 * SPB..8 * SPB).contains(&f)), "{toms:?}");
        assert!(notes.iter().any(|n| n.0 == 8 * SPB && n.1 == 49));
        // A counter set back (a preset) is no press.
        let notes = run(&p, 8, &mut |now, p, _| {
            if now == 0 {
                p[FILL_IN] = 5.0;
            } else if during(now, 2 * SPB) {
                p[FILL_IN] = 0.0;
            }
        });
        assert!(!notes.iter().any(|n| n.1 == 48 || n.1 == 45));
    }

    #[test]
    fn an_intro_counts_in_and_an_ending_stops_it() {
        let mut p = plain("Rock");
        p[START_INTRO] = 1.0;
        let notes = run(&p, 16, &mut |_, _, _| {});
        let in_bar = |bar: u64, note: u8| notes.iter().any(|n| n.0 / (4 * SPB) == bar && n.1 == note);
        // The count in: hats and a bass drum, no snare; then a fill; then the groove after a crash.
        assert!(!in_bar(0, 38) && in_bar(0, 42) && in_bar(0, 36));
        assert!(in_bar(1, 48) && in_bar(1, 38));
        assert!(notes.iter().any(|n| n.0 == 8 * SPB && n.1 == 49));
        assert!(in_bar(3, 38) && !in_bar(3, 48));
        // The ending pressed in bar 1: bar 2 ends on a crash, then nothing.
        let p = plain("Rock");
        let notes = run(&p, 20, &mut |now, p, _| {
            if during(now, 5 * SPB) {
                p[ENDING] += 1.0;
            }
        });
        assert!(notes.iter().any(|n| n.0.abs_diff(11 * SPB) <= 1 && n.1 == 49), "the last hit on the last beat");
        assert!(notes.iter().all(|n| n.0 <= 11 * SPB + 1), "quiet after the ending");
        // Intro starts it again.
        let notes = run(&p, 24, &mut |now, p, _| {
            if during(now, 5 * SPB) {
                p[ENDING] += 1.0;
            }
            if during(now, 14 * SPB) {
                p[INTRO] += 1.0;
            }
        });
        assert!(!notes.iter().any(|n| (12 * SPB..16 * SPB).contains(&n.0)));
        assert!(notes.iter().any(|n| n.0 == 16 * SPB && n.1 == 36));
        assert!(notes.iter().any(|n| n.0 >= 20 * SPB && (n.1 == 49 || n.1 == 48)));
    }

    #[test]
    fn variation_b_is_busier() {
        let a = run(&plain("Pops"), 8, &mut |_, _, _| {});
        let mut p = plain("Pops");
        p[VARIATION] = 1.0;
        let b = run(&p, 8, &mut |_, _, _| {});
        assert!(b.len() > a.len());
        assert!(b.iter().any(|n| n.1 == 46) && !a.iter().any(|n| n.1 == 46));
    }

    #[test]
    fn the_dancehall_bass_follows_the_chord_held() {
        let r = &RHYTHMS[RHYTHM_NAMES.iter().position(|n| *n == "Dancehall Rock").unwrap()];
        let mut p = plain("Dancehall Rock");
        p[PART] = 1.0;
        // D held, then G (below the split: they do not sound).
        let notes = run(&p, 16, &mut |now, _, ins| {
            if now == 0 {
                ins.push(MidiEvent::on(0, 50, 0.8));
            }
            if now == 8 * SPB {
                ins.push(MidiEvent::off(0, 50));
                ins.push(MidiEvent::on(0, 55, 0.8));
            }
        });
        assert!(!notes.iter().any(|n| (n.1 == 50 || n.1 == 55) && n.2 == 0.8), "the chord keys are swallowed");
        let first: Vec<u8> = notes.iter().filter(|n| bar_of(r, n.0) < 2).map(|n| n.1).collect();
        let later: Vec<u8> = notes.iter().filter(|n| bar_of(r, n.0) >= 2).map(|n| n.1).collect();
        assert!(!first.is_empty() && !later.is_empty());
        // D's and G's notes (with the seventh below the octave), in the bass's octave.
        assert!(first.iter().all(|n| [2, 6, 9, 0].contains(&(n % 12)) && (36..=60).contains(n)), "{first:?}");
        assert!(later.iter().all(|n| [7, 11, 2, 5].contains(&(n % 12))), "{later:?}");
        assert!(notes.first().is_some_and(|n| n.0 == 0 && n.1 % 12 == 2), "the root on one");
        // Its syncopation: the second note of the riff comes on the "and" of two (step 3).
        assert!(notes.iter().any(|n| n.0.abs_diff(3 * SPB / 4) <= 1));
    }

    #[test]
    fn chords_from_one_finger_or_the_songs_key() {
        assert_eq!(recognize(1 << 48, false), Some((0, Quality::Major)));
        assert_eq!(recognize(1 << 52 | 1 << 55 | 1 << 60, false), Some((0, Quality::Major)), "an inversion");
        assert_eq!(recognize(1 << 57 | 1 << 60 | 1 << 64, false), Some((9, Quality::Minor)));
        assert_eq!(recognize(1 << 43 | 1 << 47 | 1 << 50 | 1 << 53, false), Some((7, Quality::Seventh)));
        assert_eq!(recognize(1 << 48 | 1 << 40, true), Some((4, Quality::Minor)));
        assert_eq!(recognize(1 << 48 | 1 << 49 | 1 << 50, true), Some((0, Quality::Seventh)));
        // Song Key in A minor: the chords are A, C and E.
        let mut p = plain("Ballad");
        p[PART] = 2.0;
        p[CHORDS] = 2.0;
        let mut s = SeqState::new();
        let mut notes = Vec::new();
        let mut now = 0;
        while now < 8 * SPB {
            let mut out = Vec::new();
            s.process(MidiFxKind::HomeRhythms, &p, &[], &mut out, &ctx(now, 0, Scale::new(9, ScaleKind::Minor)));
            notes.extend(out.iter().filter(|e| e.on).map(|e| e.pitch));
            now += 256;
        }
        assert!(!notes.is_empty() && notes.iter().all(|n| [9, 0, 4].contains(&(n % 12)) && (55..67).contains(n)), "{notes:?}");
    }

    #[test]
    fn it_plays_the_kit_it_has() {
        // A kit with a bass drum, a snare and hats only: congas become toms or nothing.
        let kit: u128 = 1 << 36 | 1 << 38 | 1 << 42 | 1 << 45;
        assert_eq!(lane_note(CONGA_LO, kit), Some(45));
        assert_eq!(lane_note(CONGA_HI, kit), None);
        assert_eq!(lane_note(RIM, kit), Some(38));
        assert_eq!(lane_note(MARACAS, kit), Some(42));
    }
}
