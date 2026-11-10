//! Shatter: a melody maker for breakcore, built on a motif and what each
//! bar does with it.
//!
//! The motif is a few notes (2 to 16 steps) drawn in the song's key: a
//! degree each, an accent, held over the rests after it. Every bar of the
//! phrase (1 to 8 bars) has a move, as a breakcore track tears one riff up
//! into a whole section: *Play* it, *Answer* it a fifth up, *Climb* (every
//! time round two steps higher), *Invert* it, play it *Backwards*, *Run* it
//! at double speed, *Stutter* its end, *Leap* (every other note an octave
//! up), *Shatter* it (its notes in another order, new every time round) or
//! *Rest* (a hole for the break). *Vary* changes a few notes every time the
//! phrase comes round; *Stack* puts a fifth, a triad, an octave or a power
//! chord on every note (rave stabs).
//!
//! *Write* makes a motif and its moves in a mood: *Sad* (a slow tune
//! falling over the breaks), *Rave* (stabs in a broken rhythm), *J-Core*
//! (arpeggios running up and leaping), *Orchestral* (an odd-length riff
//! against the bar, in octaves) or *Glitch* (wide jumps at the 32nds).
//! Held, *Run* doubles it, *Stutter* repeats the step, *Up* plays it an
//! octave higher and *Cut* leaves a hole. In the band it follows the chords.

use super::SeqState;
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

pub const RATE: usize = 0;
pub const MOOD: usize = 1;
pub const CENTER: usize = 2;
pub const LENGTH: usize = 3;
pub const BARS: usize = 4;
/// The bars' moves: eight of them, from here.
pub const MOVE0: usize = 5;
pub const STACK: usize = 13;
pub const GATE: usize = 14;
pub const VELOCITY: usize = 15;
pub const VARY: usize = 16;
pub const SEED: usize = 17;
pub const RUN: usize = 18;
pub const STUTTER: usize = 19;
pub const UP: usize = 20;
pub const CUT: usize = 21;
pub const BAND: usize = 28;

/// The most steps a motif has.
pub const CELLS: usize = 16;
/// A rest in the motif.
pub const REST: i8 = 127;
pub const ACCENT: u8 = 1;
pub const HOLD: u8 = 2;

pub const MOODS: &[&str] = &["Sad", "Rave", "J-Core", "Orchestral", "Glitch"];
pub const BAR_COUNTS: &[&str] = &["1", "2", "4", "8"];
const BAR_N: [i64; 4] = [1, 2, 4, 8];
pub const MOVES: &[&str] = &["Play", "Answer", "Climb", "Invert", "Backwards", "Run", "Stutter", "Leap", "Shatter", "Rest"];
pub const PLAY: u8 = 0;
pub const ANSWER: u8 = 1;
pub const CLIMB: u8 = 2;
pub const INVERT: u8 = 3;
pub const BACKWARDS: u8 = 4;
pub const RUN_MOVE: u8 = 5;
pub const STUTTER_MOVE: u8 = 6;
pub const LEAP: u8 = 7;
pub const SHATTER: u8 = 8;
pub const REST_MOVE: u8 = 9;
pub const STACKS: &[&str] = &["Off", "Fifth", "Triad", "Octave", "Power"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", super::RATES, 2),
    ParamSpec::choice("Mood", "Write", MOODS, 0),
    ParamSpec::int("Center", "Notes", 36.0, 96.0, 72.0, Unit::None),
    ParamSpec::int("Length", "Motif", 2.0, 16.0, 8.0, Unit::None),
    ParamSpec::choice("Bars", "Phrase", BAR_COUNTS, 2),
    ParamSpec::choice("Move 1", "Phrase", MOVES, 0),
    ParamSpec::choice("Move 2", "Phrase", MOVES, 1),
    ParamSpec::choice("Move 3", "Phrase", MOVES, 0),
    ParamSpec::choice("Move 4", "Phrase", MOVES, 4),
    ParamSpec::choice("Move 5", "Phrase", MOVES, 0),
    ParamSpec::choice("Move 6", "Phrase", MOVES, 1),
    ParamSpec::choice("Move 7", "Phrase", MOVES, 2),
    ParamSpec::choice("Move 8", "Phrase", MOVES, 6),
    ParamSpec::choice("Stack", "Notes", STACKS, 0),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.5, 0.9, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.1, 1.0, 0.85, Unit::Percent),
    ParamSpec::lin("Vary", "Phrase", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::int("Seed", "Write", 0.0, 999.0, 0.0, Unit::None),
    ParamSpec::toggle("Run", "Hold", false),
    ParamSpec::toggle("Stutter", "Hold", false),
    ParamSpec::toggle("Up", "Hold", false),
    ParamSpec::toggle("Cut", "Hold", false),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    super::minimal::DYNAMICS[0],
    super::minimal::DYNAMICS[1],
    super::minimal::DYNAMICS[2],
    super::minimal::DYNAMICS[3],
    ParamSpec::toggle("Band", "Band", false),
    ParamSpec::choice("Follow", "Band", crate::band::FOLLOWS, 1),
];

/// A cell of the motif: its degree (from the centre) and flags, or None for a rest.
pub fn cell(data: &[u8], i: usize) -> Option<(i32, u8)> {
    let d = *data.get(i % CELLS)? as i8;
    (d != REST).then(|| (d as i32, data.get(CELLS + i % CELLS).copied().unwrap_or(0)))
}

pub fn set_cell(data: &mut Vec<u8>, i: usize, v: Option<(i32, u8)>) {
    if data.len() < 2 * CELLS {
        data.resize(2 * CELLS, 0);
        for b in data.iter_mut().take(CELLS) {
            if *b == 0 {
                *b = REST as u8;
            }
        }
    }
    match v {
        Some((d, f)) => {
            data[i % CELLS] = d.clamp(-24, 24) as i8 as u8;
            data[CELLS + i % CELLS] = f;
        }
        None => {
            data[i % CELLS] = REST as u8;
            data[CELLS + i % CELLS] = 0;
        }
    }
}

fn hash(a: u64) -> u32 {
    let mut x = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
    x ^= x >> 31;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 29;
    (x >> 16) as u32
}

fn roll(seed: u64, salt: u64) -> f32 {
    (hash(seed.wrapping_mul(0x1000_0001).wrapping_add(salt)) & 0xFFFF) as f32 / 65536.0
}

/// Steps in a bar of four beats at its rate.
pub fn per_bar(p: &[f32]) -> i64 {
    ((4.0 / super::rate_beats(p[RATE])).round() as i64).max(1)
}

/// What a motif and its phrase are, as Write makes them.
pub struct Written {
    pub data: Vec<u8>,
    pub rate: usize,
    pub length: usize,
    pub bars: usize,
    pub moves: [u8; 8],
    pub stack: usize,
    pub center: i32,
    pub gate: f32,
}

/// A motif and its moves in `mood`, from `seed`.
pub fn write(mood: usize, seed: u32) -> Written {
    let s = seed as u64 * 7919 + mood as u64 * 104_729;
    let r = |k: u64| roll(s, k);
    let pick = |k: u64, from: &[i32]| from[((r(k) * from.len() as f32) as usize).min(from.len() - 1)];
    let mut data = vec![REST as u8; CELLS];
    data.extend(std::iter::repeat_n(0u8, CELLS));
    let put = |data: &mut Vec<u8>, i: usize, d: i32, f: u8| set_cell(data, i, Some((d, f)));
    match mood {
        // Rave: stabs in a broken rhythm (3 3 2, 3 3 2), chords on top.
        1 => {
            let hits = [0usize, 3, 6, 8, 11, 14];
            let roots = [0, 0, 0, 5, 3, 4, -1, 0];
            for (j, &i) in hits.iter().enumerate() {
                let d = if j == 0 { 0 } else { pick(10 + j as u64, &roots) };
                put(&mut data, i, d, if j % 3 == 0 { ACCENT } else { 0 });
            }
            Written { data, rate: 2, length: 16, bars: 2, moves: [PLAY, PLAY, ANSWER, STUTTER_MOVE, PLAY, CLIMB, SHATTER, RUN_MOVE], stack: 2, center: 60, gate: 0.45 }
        }
        // J-Core: an arpeggio running up and back, chord tones and the ninth.
        2 => {
            let tones = [0, 2, 4, 7, 9, 11, 14];
            let up = 3 + (r(1) * 3.0) as usize;
            let mut seq = Vec::new();
            for k in 0..=up.min(6) {
                seq.push(tones[k]);
            }
            for k in (1..up.min(6)).rev() {
                seq.push(tones[k]);
            }
            seq.truncate(8);
            while seq.len() < 8 {
                seq.push(pick(20 + seq.len() as u64, &tones));
            }
            for (i, d) in seq.iter().enumerate() {
                put(&mut data, i, *d, if i == 0 || i == 4 { ACCENT } else { 0 });
            }
            Written { data, rate: 2, length: 8, bars: 2, moves: [PLAY, CLIMB, LEAP, RUN_MOVE, PLAY, ANSWER, CLIMB, STUTTER_MOVE], stack: 0, center: 72, gate: 0.7 }
        }
        // Orchestral: an odd riff (five or seven) against the bar, in octaves, hard accents.
        3 => {
            let len = if r(1) < 0.5 { 5 } else { 7 };
            let mut d = 0;
            for i in 0..len {
                if i > 0 {
                    d += pick(30 + i as u64, &[-1, -1, 1, 2, -2, 3, -4]);
                    d = d.clamp(-5, 7);
                }
                let rest = i > 0 && r(40 + i as u64) < 0.15;
                if !rest {
                    put(&mut data, i, d, if i == 0 || r(50 + i as u64) < 0.3 { ACCENT } else { 0 });
                }
            }
            Written { data, rate: 2, length: len, bars: 2, moves: [PLAY, INVERT, ANSWER, SHATTER, PLAY, BACKWARDS, LEAP, REST_MOVE], stack: 3, center: 55, gate: 0.85 }
        }
        // Glitch: wide jumps at the 32nds, accents by chance.
        4 => {
            let len = 6;
            for i in 0..len {
                if i > 0 && r(60 + i as u64) < 0.2 {
                    continue;
                }
                let d = pick(70 + i as u64, &[0, 7, -3, 9, 4, -5, 11, 2]);
                put(&mut data, i, d, if r(80 + i as u64) < 0.4 { ACCENT } else { 0 });
            }
            Written { data, rate: 0, length: len, bars: 2, moves: [SHATTER, STUTTER_MOVE, RUN_MOVE, REST_MOVE, SHATTER, LEAP, STUTTER_MOVE, BACKWARDS], stack: 0, center: 72, gate: 0.4 }
        }
        // Sad: a slow tune, mostly falling a step at a time from a chord tone, held notes.
        _ => {
            let mut d = pick(1, &[4, 2, 7, 4]);
            let mut i = 0;
            while i < 8 {
                let hold = r(100 + i as u64) < 0.3 && i < 7;
                put(&mut data, i, d, if i == 0 { ACCENT } else if hold { HOLD } else { 0 });
                i += if hold { 2 } else { 1 };
                let step = r(200 + i as u64);
                d += if step < 0.55 { -1 } else if step < 0.75 { 1 } else if step < 0.9 { -2 } else { 3 };
                d = d.clamp(-3, 9);
            }
            Written { data, rate: 4, length: 8, bars: 2, moves: [PLAY, ANSWER, PLAY, BACKWARDS, PLAY, ANSWER, CLIMB, REST_MOVE], stack: 0, center: 72, gate: 1.2 }
        }
    }
}

/// The motif with a few of its notes changed (from `seed`): up or down a step, or another rhythm.
pub fn mutate(data: &mut Vec<u8>, length: usize, seed: u32) {
    let len = length.clamp(2, CELLS);
    for k in 0..3u64 {
        let i = (roll(seed as u64, k) * len as f32) as usize % len;
        match cell(data, i) {
            Some((d, f)) => {
                let by = [-2, -1, 1, 2][(roll(seed as u64, 10 + k) * 4.0) as usize % 4];
                set_cell(data, i, Some((d + by, f)));
            }
            None => {
                let near = (0..len).filter_map(|j| cell(data, j)).map(|c| c.0).next().unwrap_or(0);
                set_cell(data, i, Some((near + [0, 2, 4][(roll(seed as u64, 20 + k) * 3.0) as usize % 3], 0)));
            }
        }
    }
}

/// The notes of step `k`: (degree, accent, hits). None: a rest.
pub fn note_at(p: &[f32], data: &[u8], k: i64, scale_len: i32) -> Option<(i32, bool, u32)> {
    if p.len() < SPECS.len() || p[CUT] >= 0.5 {
        return None;
    }
    let spb = per_bar(p);
    let bar = k.div_euclid(spb);
    let s = k.rem_euclid(spb);
    let n = BAR_N[(p[BARS].round().max(0.0) as usize).min(3)];
    let slot = bar.rem_euclid(n) as usize;
    let pass = bar.div_euclid(n);
    let mut mv = (p[MOVE0 + slot.min(7)].round().max(0.0) as u8).min(REST_MOVE);
    if p[RUN] >= 0.5 {
        mv = RUN_MOVE;
    }
    let len = (p[LENGTH].round() as usize).clamp(2, CELLS);
    let idx = (s as usize) % len;
    let rep = (s as usize / len) as i32;
    let seed = p[SEED].round().max(0.0) as u64;
    let oct = scale_len.max(1);
    let first = (0..len).find_map(|i| cell(data, i)).map_or(0, |c| c.0);
    let mut hits = 1u32;
    let got = match mv {
        REST_MOVE => None,
        ANSWER => cell(data, idx).map(|(d, f)| (d + 4, f)),
        CLIMB => cell(data, idx).map(|(d, f)| (d + 2 * rep, f)),
        INVERT => cell(data, idx).map(|(d, f)| (2 * first - d, f)),
        BACKWARDS => cell(data, len - 1 - idx),
        RUN_MOVE => {
            // Twice as fast: two of the motif's notes in every step.
            hits = 2;
            cell(data, (2 * idx) % len).or_else(|| cell(data, (2 * idx + 1) % len))
        }
        STUTTER_MOVE => {
            let from = spb * 3 / 4;
            if s >= from {
                hits = if s >= spb - spb / 8 { 4 } else { 2 };
                cell(data, (from as usize) % len).or_else(|| (0..len).find_map(|i| cell(data, i)))
            } else {
                cell(data, idx)
            }
        }
        LEAP => cell(data, idx).map(|(d, f)| (d + if (idx + rep as usize) % 2 == 1 { oct } else { 0 }, f)),
        SHATTER => {
            // Another order every time round.
            let mut order: [usize; CELLS] = std::array::from_fn(|i| i);
            for i in (1..len).rev() {
                let j = (hash(seed ^ (pass as u64) << 20 ^ (bar as u64) << 8 ^ i as u64) as usize) % (i + 1);
                order.swap(i, j);
            }
            cell(data, order[idx])
        }
        _ => cell(data, idx),
    };
    let (mut d, f) = got?;
    // Varied a little every time the phrase comes round.
    let vary = p[VARY].clamp(0.0, 1.0);
    if vary > 0.0 {
        let r = roll(seed ^ 0xABCD, (pass as u64) << 24 ^ (k.rem_euclid(spb * n) as u64));
        if r < vary * 0.12 && f & ACCENT == 0 {
            return None;
        } else if r < vary * 0.3 {
            d += if r < vary * 0.21 { 1 } else { -1 };
        } else if r < vary * 0.38 {
            hits = hits.max(2);
        }
    }
    if p[STUTTER] >= 0.5 {
        hits = 4;
    }
    if p[UP] >= 0.5 {
        d += oct;
    }
    Some((d, f & ACCENT != 0, hits))
}

/// Steps until the next note (for how long a held one lasts).
fn gap(p: &[f32], data: &[u8], k: i64, scale_len: i32) -> i64 {
    (1..16).find(|g| note_at(p, data, k + g, scale_len).is_some()).unwrap_or(16)
}

impl SeqState {
    pub(crate) fn shatter(&mut self, k: i64, p: &[f32], data: &[u8], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        if p.len() < SPECS.len() {
            return;
        }
        let spb = per_bar(p);
        let n = BAR_N[(p[BARS].round().max(0.0) as usize).min(3)];
        let len = (p[LENGTH].round() as usize).clamp(2, CELLS);
        // Where it is, for the face: the step, the bar of the phrase, the motif's step.
        self.visual[0] = k.max(0) as u64;
        self.visual[1] = k.div_euclid(spb).rem_euclid(n) as u64;
        self.visual[2] = (k.rem_euclid(spb) as usize % len) as u64;
        let sl = ctx.scale.len() as i32;
        let Some((d, accent, hits)) = note_at(p, data, k, sl) else {
            self.visual[3] = 0;
            return;
        };
        self.visual[3] = 1;
        let held_flag = cell(data, k.rem_euclid(spb) as usize % len).is_some_and(|c| c.1 & HOLD != 0);
        let (home, _) = ctx.scale.locate(p[CENTER].round() as i32);
        let each = step_frames / hits as f64;
        let gate = p[GATE].clamp(0.1, 1.5) as f64;
        let held = if hits > 1 {
            each * 0.75
        } else if held_flag {
            gap(p, data, k, sl).min(8) as f64 * step_frames * gate.max(0.95)
        } else {
            step_frames * gate
        };
        let vel = p[VELOCITY].clamp(0.1, 1.0) * if accent { 1.0 } else { 0.78 };
        let stack: &[(i32, i32)] = match p[STACK].round() as usize {
            1 => &[(4, 0)],
            2 => &[(2, 0), (4, 0)],
            3 => &[(0, 12)],
            4 => &[(4, 0), (0, 12)],
            _ => &[],
        };
        // A run's second note: the motif's next.
        let run_next = |h: u32| if hits == 2 && p[STUTTER] < 0.5 && h == 1 { cell(data, ((2 * (k.rem_euclid(spb) as usize % len)) + 1) % len).map(|c| c.0 - cell(data, (2 * (k.rem_euclid(spb) as usize % len)) % len).map_or(0, |c| c.0)).unwrap_or(0) } else { 0 };
        for h in 0..hits {
            let dd = d + run_next(h);
            let pitch = ctx.scale.pitch_at(home + dd);
            let fade = if hits > 2 { 1.0 - 0.1 * h as f32 } else { 1.0 };
            let t = at + (h as f64 * each) as u64;
            self.mlater(ctx, t, pitch, vel * fade, held, out);
            for (sd, semis) in stack {
                let sp = ctx.scale.pitch_at(home + dd + sd) + semis;
                self.mlater(ctx, t, sp, vel * fade * 0.85, held, out);
            }
        }
    }
}

fn sad() -> Vec<u8> {
    write(0, 0).data
}
fn rave() -> Vec<u8> {
    write(1, 0).data
}
fn jcore() -> Vec<u8> {
    write(2, 0).data
}
fn orchestral() -> Vec<u8> {
    (0..64).map(|s| write(3, s)).find(|w| w.length == 7).map_or_else(|| write(3, 0).data, |w| w.data)
}
fn glitch() -> Vec<u8> {
    write(4, 0).data
}

use crate::presets::{pd, Preset, V::C, V::N};

pub(crate) static PRESETS: &[Preset] = &[
    pd("Sad Lullaby", &[("Rate", C("1/8")), ("Mood", C("Sad")), ("Length", N(8.0)), ("Bars", C("4")), ("Move 1", C("Play")), ("Move 2", C("Answer")), ("Move 3", C("Play")), ("Move 4", C("Backwards")), ("Move 5", C("Play")), ("Move 6", C("Answer")), ("Move 7", C("Climb")), ("Move 8", C("Rest")), ("Center", N(72.0)), ("Gate", N(1.2))], sad),
    pd("Rave Stabs", &[("Rate", C("1/16")), ("Mood", C("Rave")), ("Length", N(16.0)), ("Bars", C("4")), ("Move 1", C("Play")), ("Move 2", C("Play")), ("Move 3", C("Answer")), ("Move 4", C("Stutter")), ("Move 5", C("Play")), ("Move 6", C("Climb")), ("Move 7", C("Shatter")), ("Move 8", C("Run")), ("Stack", C("Triad")), ("Center", N(60.0)), ("Gate", N(0.45))], rave),
    pd("J-Core Runs", &[("Rate", C("1/16")), ("Mood", C("J-Core")), ("Length", N(8.0)), ("Bars", C("4")), ("Move 1", C("Play")), ("Move 2", C("Climb")), ("Move 3", C("Leap")), ("Move 4", C("Run")), ("Move 5", C("Play")), ("Move 6", C("Answer")), ("Move 7", C("Climb")), ("Move 8", C("Stutter")), ("Center", N(72.0)), ("Gate", N(0.7))], jcore),
    pd("Orchestral Stabs", &[("Rate", C("1/16")), ("Mood", C("Orchestral")), ("Length", N(7.0)), ("Bars", C("4")), ("Move 1", C("Play")), ("Move 2", C("Invert")), ("Move 3", C("Answer")), ("Move 4", C("Shatter")), ("Move 5", C("Play")), ("Move 6", C("Backwards")), ("Move 7", C("Leap")), ("Move 8", C("Rest")), ("Stack", C("Octave")), ("Center", N(55.0)), ("Gate", N(0.85))], orchestral),
    pd("Glitch Shards", &[("Rate", C("1/32")), ("Mood", C("Glitch")), ("Length", N(6.0)), ("Bars", C("4")), ("Move 1", C("Shatter")), ("Move 2", C("Stutter")), ("Move 3", C("Run")), ("Move 4", C("Rest")), ("Move 5", C("Shatter")), ("Move 6", C("Leap")), ("Move 7", C("Stutter")), ("Move 8", C("Backwards")), ("Center", N(72.0)), ("Gate", N(0.4))], glitch),
    pd("Amen Lead", &[("Rate", C("1/16")), ("Mood", C("Sad")), ("Length", N(8.0)), ("Bars", C("4")), ("Move 1", C("Play")), ("Move 2", C("Run")), ("Move 3", C("Answer")), ("Move 4", C("Stutter")), ("Move 5", C("Play")), ("Move 6", C("Leap")), ("Move 7", C("Shatter")), ("Move 8", C("Rest")), ("Stack", C("Fifth")), ("Vary", N(0.4)), ("Center", N(67.0)), ("Gate", N(0.6))], sad),
    pd("Hoover Power", &[("Rate", C("1/8")), ("Mood", C("Rave")), ("Length", N(16.0)), ("Bars", C("2")), ("Move 1", C("Play")), ("Move 2", C("Answer")), ("Stack", C("Power")), ("Center", N(48.0)), ("Gate", N(0.9))], rave),
];

/// The motif a new Shatter starts with: Sad, seed 0.
pub fn shatter_default() -> Vec<u8> {
    write(0, 0).data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::MidiFxKind;
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};
    use crate::sequencers::tests::run;

    fn setup(mood: usize, seed: u32) -> (Vec<f32>, Vec<u8>) {
        let w = write(mood, seed);
        let mut p = defaults(SPECS);
        p[RATE] = w.rate as f32;
        p[LENGTH] = w.length as f32;
        p[BARS] = w.bars as f32;
        for (k, m) in w.moves.iter().enumerate() {
            p[MOVE0 + k] = *m as f32;
        }
        p[STACK] = w.stack as f32;
        p[CENTER] = w.center as f32;
        p[GATE] = w.gate;
        (p, w.data)
    }

    #[test]
    fn every_mood_writes_a_motif_that_plays_in_the_key() {
        let c = Scale::new(0, ScaleKind::Major);
        for mood in 0..MOODS.len() {
            for seed in 0..6 {
                let (p, data) = setup(mood, seed);
                let notes = (0..CELLS).filter(|i| cell(&data, *i).is_some()).count();
                assert!(notes >= 3, "{} {seed}: {notes} notes", MOODS[mood]);
                let ons = run(MidiFxKind::Shatter, &p, &data, 16.0);
                assert!(ons.len() >= 8, "{} {seed}: {}", MOODS[mood], ons.len());
                assert!(ons.iter().all(|o| c.contains(o.1)), "{}: {ons:?}", MOODS[mood]);
            }
        }
    }

    #[test]
    fn the_moves_do_what_they_say() {
        let (mut p, data) = setup(2, 1);
        p[VARY] = 0.0;
        p[BARS] = 0.0;
        let sl = 7;
        let line = |p: &[f32]| (0..8).map(|k| note_at(p, &data, k, sl).map(|n| n.0)).collect::<Vec<_>>();
        p[MOVE0] = PLAY as f32;
        let play = line(&p);
        p[MOVE0] = BACKWARDS as f32;
        let back = line(&p);
        let mut rev = play.clone();
        rev.reverse();
        assert_eq!(back, rev, "backwards");
        p[MOVE0] = ANSWER as f32;
        assert_eq!(line(&p), play.iter().map(|n| n.map(|d| d + 4)).collect::<Vec<_>>(), "a fifth up");
        p[MOVE0] = REST_MOVE as f32;
        assert!(line(&p).iter().all(|n| n.is_none()), "a hole");
        p[MOVE0] = RUN_MOVE as f32;
        assert!((0..8).all(|k| note_at(&p, &data, k, sl).is_none_or(|n| n.2 == 2)), "two a step");
        p[MOVE0] = SHATTER as f32;
        let mut a: Vec<i32> = line(&p).into_iter().flatten().collect();
        let mut b: Vec<i32> = play.into_iter().flatten().collect();
        a.sort();
        b.sort();
        assert_eq!(a, b, "the same notes in another order");
    }

    #[test]
    fn held_buttons_play_hotter_and_cut() {
        let (mut p, data) = setup(0, 3);
        p[VARY] = 0.0;
        let plain = run(MidiFxKind::Shatter, &p, &data, 8.0).len();
        p[STUTTER] = 1.0;
        let stut = run(MidiFxKind::Shatter, &p, &data, 8.0).len();
        assert!(stut > plain * 2, "{stut} {plain}");
        p[STUTTER] = 0.0;
        p[CUT] = 1.0;
        assert!(run(MidiFxKind::Shatter, &p, &data, 8.0).is_empty());
    }
}
