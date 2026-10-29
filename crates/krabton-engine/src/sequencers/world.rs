//! Sequencers after ways of making music from around the world:
//!
//! - Gamelan: a slow core melody (the balungan) and two parts that lock
//!   into each other above it (polos and sangsih: each plays where the
//!   other rests, together one fast line), in Norot, Nyog Cag, Telu or
//!   Empat; the gongs mark the cycle (the big gong at its end, kenong at its
//!   quarters, kempul between). Irama sets how many notes the parts play to
//!   each note of the core, and can change from cycle to cycle. In five
//!   notes of the key (or all seven).
//! - Raga: a melody by a raga's rules: the notes it may take going up and
//!   the ones going down (not always the same), the notes it rests on, and
//!   its signature phrase it keeps coming back to. A slow, free Alap; a Jor
//!   with a pulse; a fast Jhala with the drone string struck between; or
//!   the Journey through all three. Grace notes and slides, a drone of Sa
//!   and Pa. Sa is the song's key note.
//! - Call: call and response. A caller sings a phrase that ends open, a
//!   choir answers it: the same, higher, upside down, or coming home to the
//!   key note; now and then a new call.

use super::minimal::{choose, hash, DYNAMICS};
use super::nature::pitch;
use super::systems::{phrase, Phrase, PHRASES, PHRASE_LEN};
use super::{rate_beats, SeqState, RATES};
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

const KEYS: [ParamSpec; 2] = [ParamSpec::toggle("Transpose", "Keys", false), ParamSpec::toggle("Hold", "Keys", false)];

// ---------------------------------------------------------------- gamelan

pub const GA_RATE: usize = 0;
pub const GA_IRAMA: usize = 1;
pub const GA_STYLE: usize = 2;
pub const GA_CYCLE: usize = 3;
pub const GA_MELODY: usize = 4;
pub const GA_SCALE: usize = 5;
pub const GA_CHANGE: usize = 6;
pub const GA_VARIATION: usize = 7;
pub const GA_GONGS: usize = 8;
pub const GA_CORE: usize = 9;
pub const GA_PARTS: usize = 10;
pub const GA_BASE: usize = 11;
pub const GA_GATE: usize = 12;
pub const GA_VELOCITY: usize = 13;
pub const GA_ACCENT: usize = 14;
pub const IRAMAS: &[&str] = &["1", "2", "3"];
pub const KOTEKAN: &[&str] = &["Norot", "Nyog Cag", "Telu", "Empat"];
pub const GONG_CYCLES: &[&str] = &["8", "16", "32"];
pub const GAMELAN_SCALES: &[&str] = &["Five", "Seven"];
pub const IRAMA_CHANGES: &[&str] = &["Never", "2 Cycles", "4 Cycles"];
pub const MAX_CORE: usize = 32;

pub static GAMELAN: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Irama", "Gamelan", IRAMAS, 1),
    ParamSpec::choice("Style", "Gamelan", KOTEKAN, 0),
    ParamSpec::choice("Cycle", "Gamelan", GONG_CYCLES, 0),
    ParamSpec::int("Melody", "Gamelan", 0.0, 99.0, 0.0, Unit::None),
    ParamSpec::choice("Scale", "Gamelan", GAMELAN_SCALES, 0),
    ParamSpec::choice("Change", "Gamelan", IRAMA_CHANGES, 0),
    ParamSpec::lin("Variation", "Gamelan", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::toggle("Gongs", "Parts", true),
    ParamSpec::toggle("Core", "Parts", true),
    ParamSpec::toggle("Kotekan", "Parts", true),
    ParamSpec::int("Base Note", "Notes", 48.0, 96.0, 72.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 2.0, 0.9, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Accent", "Notes", 0.0, 1.0, 0.4, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

/// The core melody of cycle `cycle`: `n` notes (in the five or seven notes,
/// 0 the gong tone), the last always the gong tone; Variation changes a few
/// from cycle to cycle.
pub fn balungan(seed: u32, cycle: u32, variation: f32, n: usize) -> [i8; MAX_CORE] {
    let mut out = [0i8; MAX_CORE];
    let mut d = 0i32;
    let n = n.min(MAX_CORE);
    for (j, o) in out.iter_mut().enumerate().take(n) {
        let h = hash(seed * 29 + 7, j as u32);
        d = (d + [-2, -1, -1, 1, 1, 2, 0, 3][(h * 8.0) as usize % 8]).clamp(-2, 6);
        let mut v = d;
        if j + 1 < n && hash(seed * 31 + 3 + cycle * 977, j as u32) < variation {
            v += if hash(seed + cycle, j as u32 + 99) < 0.5 { 1 } else { -1 };
        }
        if j + 1 == n {
            v = 0;
            d = 0;
        }
        *o = v.clamp(-3, 7) as i8;
    }
    out
}

/// The interlocking parts at step `s` of `m` leading from core note `a` to
/// `b`: the polos's note and the sangsih's (in the five or seven notes).
pub fn kotekan(style: usize, a: i32, b: i32, s: usize, m: usize) -> (Option<i32>, Option<i32>) {
    let from_end = m.saturating_sub(1 + s);
    let rel = (s as i32 - (m as i32 - 1)).rem_euclid(4) as usize;
    match style {
        // Each on the note and the one above it, by turns.
        0 => {
            let main = if s < m / 2 { a } else { b };
            if from_end % 2 == 0 {
                (Some(main), None)
            } else {
                (None, Some(main + 1))
            }
        }
        // One line from the last note to the next, its notes shared out.
        1 => {
            let v = if a == b { b + [0, 1, 0, -1][rel] } else { a + ((b - a) as f32 * (s + 1) as f32 / m as f32).round() as i32 };
            if from_end % 2 == 0 {
                (Some(v), None)
            } else {
                (None, Some(v))
            }
        }
        // Three notes round the next: the polos the lower two, the sangsih the upper two.
        2 => {
            let o = [0, 1, 0, -1][rel];
            (if o <= 0 { Some(b + o) } else { None }, if o >= 0 { Some(b + o) } else { None })
        }
        // Four notes: the sangsih a fourth above where the polos is lowest.
        _ => match rel {
            0 => (Some(b), None),
            1 => (None, Some(b + 2)),
            2 => (Some(b - 1), Some(b + 2)),
            _ => (None, Some(b + 1)),
        },
    }
}

/// The irama level of cycle `c`: Change moves it up and back down round the one set.
pub fn irama_at(p: &[f32], c: i64) -> usize {
    let base = choose(p, GA_IRAMA, IRAMAS.len()) as i32;
    let every = [0i64, 2, 4][choose(p, GA_CHANGE, IRAMA_CHANGES.len())];
    if every == 0 {
        return base as usize;
    }
    (base + [0, 1, 0, -1][(c / every).rem_euclid(4) as usize]).clamp(0, 2) as usize
}

// ---------------------------------------------------------------- raga

pub const RA_RATE: usize = 0;
pub const RA_RAGA: usize = 1;
pub const RA_STAGE: usize = 2;
pub const RA_JOURNEY: usize = 3;
pub const RA_PAKAD: usize = 4;
pub const RA_ORNAMENTS: usize = 5;
pub const RA_LINGER: usize = 6;
pub const RA_RANGE: usize = 7;
pub const RA_SEED: usize = 8;
pub const RA_BASE: usize = 9;
pub const RA_GATE: usize = 10;
pub const RA_VELOCITY: usize = 11;
pub const RA_DRONE: usize = 12;
pub const RA_DRONE_NOTE: usize = 13;
pub const RAGAS: &[&str] = &["Yaman", "Bhairav", "Bhupali", "Malkauns", "Kafi", "Bhimpalasi", "Durga", "Marwa", "Key"];
pub const STAGES: &[&str] = &["Alap", "Jor", "Jhala", "Journey"];
pub const DRONE_NOTES: &[&str] = &["Pa", "Ma", "Ni"];

pub static RAGA: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 4),
    ParamSpec::choice("Raga", "Raga", RAGAS, 0),
    ParamSpec::choice("Stage", "Raga", STAGES, 3),
    ParamSpec::int("Journey", "Raga", 4.0, 64.0, 16.0, Unit::None),
    ParamSpec::lin("Pakad", "Raga", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Ornaments", "Raga", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Linger", "Raga", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Range", "Raga", 1.0, 3.0, 2.0, Unit::Octaves),
    ParamSpec::int("Seed", "Raga", 0.0, 99.0, 0.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 36.0, 72.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.9, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::toggle("Drone", "Drone", true),
    ParamSpec::choice("Drone Note", "Drone", DRONE_NOTES, 0),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

/// A raga: the notes going up and going down (semitones above Sa), the ones
/// it rests on and its signature phrase (pakad).
#[derive(Clone, Copy)]
pub struct Raga {
    pub up: [i8; 12],
    pub n_up: usize,
    pub down: [i8; 12],
    pub n_down: usize,
    pub rest: [i8; 3],
    pub pakad: [i8; 12],
    pub n_pakad: usize,
}

const fn raga(up: &[i8], down: &[i8], rest: [i8; 3], pakad: &[i8]) -> Raga {
    let mut r = Raga { up: [0; 12], n_up: up.len(), down: [0; 12], n_down: down.len(), rest, pakad: [0; 12], n_pakad: pakad.len() };
    let mut i = 0;
    while i < up.len() {
        r.up[i] = up[i];
        i += 1;
    }
    i = 0;
    while i < down.len() {
        r.down[i] = down[i];
        i += 1;
    }
    i = 0;
    while i < pakad.len() {
        r.pakad[i] = pakad[i];
        i += 1;
    }
    r
}

pub static RAGA_NOTES: [Raga; 8] = [
    raga(&[0, 2, 4, 6, 9, 11], &[0, 2, 4, 6, 7, 9, 11], [0, 4, 11], &[-1, 2, 4, 2, 0, 7, 6, 4, 2, 0]),
    raga(&[0, 1, 4, 5, 7, 8, 11], &[0, 1, 4, 5, 7, 8, 11], [0, 8, 1], &[4, 5, 8, 8, 7, 4, 5, 1, 1, 0]),
    raga(&[0, 2, 4, 7, 9], &[0, 2, 4, 7, 9], [0, 4, 9], &[4, 2, 0, -3, 0, 2, 4, 7, 4, 2, 0]),
    raga(&[0, 3, 5, 8, 10], &[0, 3, 5, 8, 10], [0, 5, 3], &[5, 3, 5, 8, 10, 8, 5, 3, 0]),
    raga(&[0, 2, 3, 5, 7, 9, 10], &[0, 2, 3, 5, 7, 9, 10], [0, 7, 3], &[0, 2, 3, 5, 7, 5, 3, 2, 0]),
    raga(&[0, 3, 5, 7, 10], &[0, 2, 3, 5, 7, 9, 10], [0, 5, 3], &[-2, 0, 3, 5, 7, 5, 3, 2, 0]),
    raga(&[0, 2, 5, 7, 9], &[0, 2, 5, 7, 9], [0, 5, 2], &[5, 2, 5, 7, 9, 7, 5, 2, 0]),
    raga(&[0, 1, 4, 6, 9, 11], &[0, 1, 4, 6, 9, 11], [1, 9, 0], &[-3, -1, 1, 4, 6, 4, 1, 0]),
];

/// The raga `i` (the last: one made from the song's scale, going up
/// without its fourth and seventh).
pub fn raga_notes(i: usize, scale: &crate::scale::Scale) -> Raga {
    if i < RAGA_NOTES.len() {
        return RAGA_NOTES[i];
    }
    let iv = scale.intervals();
    let mut r = Raga { up: [0; 12], n_up: 0, down: [0; 12], n_down: 0, rest: [0; 3], pakad: [0; 12], n_pakad: 0 };
    for (k, &x) in iv.iter().enumerate().take(12) {
        r.down[r.n_down] = x as i8;
        r.n_down += 1;
        if !(iv.len() == 7 && (k == 3 || k == 6)) {
            r.up[r.n_up] = x as i8;
            r.n_up += 1;
        }
    }
    let at = |d: usize| iv.get(d.min(iv.len() - 1)).copied().unwrap_or(0) as i8;
    r.rest = [0, at(2), at(4)];
    for (j, d) in [2usize, 1, 0, 4, 3, 2, 1, 0].iter().enumerate() {
        r.pakad[j] = at(*d);
    }
    r.n_pakad = 8;
    r
}

impl Raga {
    /// The next note above `x` going up (None above `hi`).
    pub fn next_up(&self, x: i32, hi: i32) -> Option<i32> {
        (-1..=4).flat_map(|o| self.up[..self.n_up].iter().map(move |&u| u as i32 + 12 * o)).filter(|&v| v > x && v <= hi).min()
    }

    /// The next note below `x` coming down (None below `lo`).
    pub fn next_down(&self, x: i32, lo: i32) -> Option<i32> {
        (-1..=4).flat_map(|o| self.down[..self.n_down].iter().map(move |&u| u as i32 + 12 * o)).filter(|&v| v < x && v >= lo).max()
    }

    pub fn rests_on(&self, x: i32) -> bool {
        self.rest.contains(&(x.rem_euclid(12) as i8))
    }
}

/// The stage at step `k` (0 Alap, 1 Jor, 2 Jhala) and how far through the Journey (0..1).
pub fn raga_stage(p: &[f32], k: i64) -> (usize, f64) {
    let stage = choose(p, RA_STAGE, STAGES.len());
    let bars = p[RA_JOURNEY].round().clamp(1.0, 64.0) as f64;
    let at = (k.max(0) as f64 * rate_beats(p[RA_RATE]) / 4.0 / bars).fract();
    if stage < 3 {
        return (stage, at);
    }
    (if at < 0.4 { 0 } else if at < 0.75 { 1 } else { 2 }, at)
}

// ---------------------------------------------------------------- call

pub const CR_RATE: usize = 0;
pub const CR_LENGTH: usize = 1;
pub const CR_ANSWER: usize = 2;
pub const CR_CHOIR: usize = 3;
pub const CR_GAP: usize = 4;
pub const CR_PHRASE: usize = 5;
pub const CR_SEED: usize = 6;
pub const CR_CHANGE: usize = 7;
pub const CR_BASE: usize = 8;
pub const CR_OCTAVE: usize = 9;
pub const CR_GATE: usize = 10;
pub const CR_VELOCITY: usize = 11;
pub const CR_ANSWER_VELOCITY: usize = 12;
pub const CR_SWING: usize = 13;
pub const ANSWERS: &[&str] = &["Echo", "Higher", "Upside Down", "Home", "Turns"];

pub static CALL: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 4),
    ParamSpec::int("Length", "Call", 2.0, 16.0, 6.0, Unit::None),
    ParamSpec::choice("Answer", "Call", ANSWERS, 3),
    ParamSpec::int("Choir", "Call", 1.0, 3.0, 2.0, Unit::None),
    ParamSpec::int("Gap", "Call", 0.0, 8.0, 2.0, Unit::None),
    ParamSpec::choice("Phrase", "Call", PHRASES, 0),
    ParamSpec::int("Seed", "Call", 0.0, 99.0, 0.0, Unit::None),
    ParamSpec::lin("Change", "Call", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::int("Base Note", "Notes", 36.0, 84.0, 64.0, Unit::None),
    ParamSpec::int("Octave", "Notes", -2.0, 2.0, 0.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Answer Level", "Notes", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Swing", "Notes", 0.0, 0.75, 0.0, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

/// The call of round `round` and its answer (`answer` one of `ANSWERS`, but Turns).
pub fn call_and_answer(p: &[f32], round: i64) -> (Phrase, Phrase, usize) {
    let len = (p[CR_LENGTH].round() as usize).clamp(2, PHRASE_LEN);
    let seed = p[CR_SEED].round() as u32;
    // A new call now and then (Change), else the last one again.
    let fresh = (0..=round.max(0)).rev().take(64).find(|&r| r == 0 || hash(seed + 71, r as u32) < p[CR_CHANGE]).unwrap_or(0);
    let mut call = phrase(choose(p, CR_PHRASE, PHRASES.len()), len, seed + fresh as u32 * 13);
    // The call ends open: not on the key note.
    let last = (0..len).rev().find(|&i| call[i].is_some()).unwrap_or(0);
    if call[last].map(|d| (d as i32).rem_euclid(7) == 0).unwrap_or(true) {
        call[last] = Some(if hash(seed + 5, round as u32) < 0.5 { 1 } else { 4 });
    }
    let mut kind = choose(p, CR_ANSWER, ANSWERS.len());
    if kind == 4 {
        kind = round.rem_euclid(4) as usize;
    }
    let mut answer = call;
    match kind {
        1 => answer.iter_mut().for_each(|d| *d = d.map(|d| d + 2)),
        2 => answer.iter_mut().for_each(|d| *d = d.map(|d| -d)),
        3 => {
            // Home: the same, but the last note down (or up) to the key note.
            answer[last] = Some(if call[last].unwrap_or(0) > 3 { 7 } else { 0 });
        }
        _ => {}
    }
    (call, answer, kind)
}

// ---------------------------------------------------------------- state

pub struct WorldState {
    // Raga: the note (semitones above Sa), which way, how long to stay, where in the pakad.
    pub(crate) raga_at: i32,
    raga_dir: i32,
    raga_hold: u32,
    raga_pakad: Option<usize>,
    raga_k: i64,
    raga_shown: u64,
    raga_last: u8,
}

impl Default for WorldState {
    fn default() -> Self {
        Self { raga_at: 0, raga_dir: 1, raga_hold: 0, raga_pakad: None, raga_k: i64::MIN, raga_shown: 0, raga_last: 0 }
    }
}

impl SeqState {
    // ------------------------------------------------------------ gamelan

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn gamelan(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let n = [8usize, 16, 32][choose(p, GA_CYCLE, GONG_CYCLES.len())];
        let every = [0i64, 2, 4][choose(p, GA_CHANGE, IRAMA_CHANGES.len())];
        // Find the cycle this step is in: their lengths repeat every four changes.
        let steps_of = |c: i64| (n * [2usize, 4, 8][irama_at(p, c)]) as i64;
        let period_cycles = if every == 0 { 1 } else { 4 * every };
        let period: i64 = (0..period_cycles).map(steps_of).sum();
        let k = k.max(0);
        let mut c = (k / period) * period_cycles;
        let mut rest = k % period;
        while rest >= steps_of(c) {
            rest -= steps_of(c);
            c += 1;
        }
        let level = irama_at(p, c);
        let m = [2usize, 4, 8][level];
        let j = rest as usize / m;
        let s = rest as usize % m;
        let core = balungan(p[GA_MELODY].round() as u32, c as u32, p[GA_VARIATION], n);
        let a = if j == 0 { 0 } else { core[j - 1] as i32 };
        let b = core[j] as i32;
        // The five notes (or all of the key), counted from the base note.
        let len = ctx.scale.len().max(1) as i32;
        let five = choose(p, GA_SCALE, GAMELAN_SCALES.len()) == 0 && len == 7;
        let degree = |x: i32| -> i32 {
            if five {
                [0, 1, 2, 4, 5][x.rem_euclid(5) as usize] + 7 * x.div_euclid(5)
            } else {
                x
            }
        };
        let vel = p[GA_VELOCITY];
        let accent = p[GA_ACCENT].clamp(0.0, 1.0);
        let gate = p[GA_GATE].max(0.05) as f64 * step_frames;
        let mut played = 0u64;
        let (polos, sangsih) = kotekan(choose(p, GA_STYLE, KOTEKAN.len()), a, b, s, m);
        if p[GA_PARTS] >= 0.5 {
            let on_beat = s == m - 1;
            let v = vel * if on_beat { 1.0 } else { 1.0 - 0.25 * accent };
            if let Some(x) = polos {
                self.note(ctx, at, pitch(ctx, p[GA_BASE], degree(x)), v, gate, out);
                played |= 1;
            }
            if let Some(x) = sangsih {
                if polos != sangsih {
                    self.note(ctx, at, pitch(ctx, p[GA_BASE], degree(x)), v * 0.92, gate, out);
                }
                played |= 2;
            }
        }
        let mut hits = 0u64;
        if s == m - 1 {
            // The core note lands at the end of its group, and the gongs with it.
            if p[GA_CORE] >= 0.5 {
                self.note(ctx, at, pitch(ctx, p[GA_BASE], degree(b) - len), vel * 0.9, step_frames * m as f64 * 0.9, out);
            }
            if p[GA_GONGS] >= 0.5 {
                let quarter = n / 4;
                let pos = j + 1;
                if pos == n {
                    self.note(ctx, at, pitch(ctx, p[GA_BASE], -3 * len), (vel * (1.0 + 0.4 * accent)).min(1.0), step_frames * (m * quarter) as f64, out);
                    hits |= 1;
                }
                if pos % quarter == 0 {
                    self.note(ctx, at, pitch(ctx, p[GA_BASE], degree(b) - 2 * len), (vel * (0.85 + 0.2 * accent)).min(1.0), step_frames * (m * 2) as f64, out);
                    hits |= 2;
                } else if pos % quarter == quarter / 2 && pos != quarter / 2 {
                    self.note(ctx, at, pitch(ctx, p[GA_BASE], degree(b) - 2 * len + 2), vel * 0.75, step_frames * (m * 2) as f64, out);
                    hits |= 4;
                }
            }
        }
        // For the screen: where in the cycle, the core notes of its half, the parts.
        let half = (j / 16) * 16;
        let mut notes = 0u64;
        for i in 0..16.min(n) {
            notes |= ((core[(half + i).min(MAX_CORE - 1)] as i64 + 4) as u64 & 0xf) << (i * 4);
        }
        self.visual[0] = s as u64 | (j as u64) << 4 | (n as u64) << 10 | (level as u64) << 16 | hits << 18 | (c as u64 & 0xffff) << 24 | (m as u64) << 40;
        self.visual[1] = notes;
        self.visual[2] = (polos.map(|x| x + 8).unwrap_or(0) as u64 & 0xff) | (sangsih.map(|x| x + 8).unwrap_or(0) as u64 & 0xff) << 8 | played << 16 | (half as u64) << 24;
    }

    // ------------------------------------------------------------ raga

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn raga(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let r = raga_notes(choose(p, RA_RAGA, RAGAS.len()), &ctx.scale);
        let (stage, journey) = raga_stage(p, k);
        let base = p[RA_BASE].round() as i32;
        let sa = base - (base - ctx.scale.root as i32).rem_euclid(12);
        let hi = 12 * p[RA_RANGE].round().clamp(1.0, 3.0) as i32;
        let lo = -5;
        let seed = p[RA_SEED].round() as u32;
        let rnd = |salt: u32| hash(seed * 997 + salt, k as u32);
        let vel = p[RA_VELOCITY] * [0.75, 0.9, 1.0][stage];
        let gate = p[RA_GATE] as f64;
        // The drone: Pa (or Ma, Ni) and Sa below, a pluck a beat.
        let rate = rate_beats(p[RA_RATE]);
        let beat = k as f64 * rate;
        if p[RA_DRONE] >= 0.5 && (beat - beat.round()).abs() < 1e-6 {
            let drone = [7, 5, 11][choose(p, RA_DRONE_NOTE, DRONE_NOTES.len())];
            let pluck = [drone - 12, 0, 0, -12][(beat.round() as i64).rem_euclid(4) as usize];
            self.note(ctx, at, sa - 12 + pluck, p[RA_VELOCITY] * 0.35, ctx.samples_per_beat * 1.5, out);
        }
        let w = &mut self.world;
        if k != w.raga_k + 1 {
            // A jump: from Sa again.
            w.raga_at = 0;
            w.raga_dir = 1;
            w.raga_hold = 0;
            w.raga_pakad = None;
        }
        w.raga_k = k;
        if w.raga_hold > 0 {
            w.raga_hold -= 1;
            return;
        }
        // Jhala: the drone string struck between the notes.
        if stage == 2 && k % 2 == 1 {
            self.note(ctx, at, sa + 12, vel * 0.55, step_frames * 0.5, out);
            return;
        }
        let from = w.raga_at;
        let mut next = None;
        let mut ornament = 0u8;
        if let Some(i) = w.raga_pakad {
            next = Some(r.pakad[i] as i32);
            w.raga_pakad = if i + 1 < r.n_pakad { Some(i + 1) } else { None };
        }
        if next.is_none() {
            let keep = [0.65, 0.75, 0.8][stage];
            if from >= hi {
                w.raga_dir = -1;
            } else if from <= lo {
                w.raga_dir = 1;
            } else if rnd(1) > keep {
                w.raga_dir = -w.raga_dir;
            }
            let leap = stage > 0 && rnd(2) < 0.15;
            let mut x = from;
            for _ in 0..if leap { 2 } else { 1 } {
                let step = if w.raga_dir > 0 { r.next_up(x, hi) } else { r.next_down(x, lo) };
                match step {
                    Some(v) => x = v,
                    None => {
                        w.raga_dir = -w.raga_dir;
                        x = if w.raga_dir > 0 { r.next_up(x, hi) } else { r.next_down(x, lo) }.unwrap_or(0);
                        break;
                    }
                }
            }
            next = Some(x);
        }
        let x = next.unwrap_or(0);
        w.raga_at = x;
        // Lingering: long on the resting notes in the Alap, a little in the Jor.
        let linger = p[RA_LINGER].clamp(0.0, 1.0);
        let resting = r.rests_on(x) && w.raga_pakad.is_none();
        w.raga_hold = match stage {
            0 => {
                if resting {
                    2 + (rnd(3) * 4.0 * linger) as u32
                } else {
                    (rnd(4) * 2.0 * linger) as u32
                }
            }
            1 if resting => (rnd(5) * 2.0 * linger) as u32,
            _ => 0,
        };
        // A phrase that comes to rest may lead into the pakad.
        if resting && w.raga_hold >= 1 && w.raga_pakad.is_none() && rnd(6) < p[RA_PAKAD] {
            w.raga_pakad = Some(0);
        }
        let hold = w.raga_hold;
        let len = step_frames * (hold as f64 + gate);
        // The Alap is free: a little early or late.
        let at = if stage == 0 { at + (rnd(7) as f64 * 0.2 * step_frames) as u64 } else { at };
        let orn = p[RA_ORNAMENTS].clamp(0.0, 1.0);
        let mut main_at = at;
        if orn > 0.0 && stage < 2 && rnd(8) < orn {
            if (x - from).abs() >= 3 {
                // A slide: through the notes between, quickly.
                ornament = 2;
                let mut y = from;
                let mut t = at;
                for _ in 0..3 {
                    let nxt = if x > y { r.next_up(y, hi) } else { r.next_down(y, lo) };
                    match nxt {
                        Some(v) if v != x && (v - x).signum() == (y - x).signum() => {
                            self.play(ctx, t, sa + v, vel * 0.5, step_frames / 8.0, out);
                            t += (step_frames / 6.0) as u64;
                            y = v;
                        }
                        _ => break,
                    }
                }
                main_at = t;
            } else if let Some(g) = r.next_up(x, hi + 2) {
                // A grace note: a touch of the note above first.
                ornament = 1;
                self.play(ctx, at, sa + g, vel * 0.55, step_frames / 6.0, out);
                main_at = at + (step_frames / 5.0) as u64;
            }
        }
        self.play(ctx, main_at, sa + x, vel, len, out);
        let w = &mut self.world;
        w.raga_shown = (w.raga_shown << 8) | ((x + 64) as u64 & 0xff);
        w.raga_last = ornament;
        self.visual[0] = ((x + 64) as u64 & 0xff) | (stage as u64) << 8 | (w.raga_pakad.is_some() as u64) << 10 | (ornament as u64) << 11 | ((journey * 255.0) as u64 & 0xff) << 16 | (choose(p, RA_RAGA, RAGAS.len()) as u64) << 24 | (hold as u64 & 0xff) << 32;
        self.visual[1] = w.raga_shown;
    }

    // ------------------------------------------------------------ call

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn call(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let len = (p[CR_LENGTH].round() as usize).clamp(2, PHRASE_LEN) as i64;
        let gap = p[CR_GAP].round().clamp(0.0, 8.0) as i64;
        let period = 2 * (len + gap);
        let k = k.max(0);
        let round = k / period;
        let pos = k % period;
        let (call, answer, kind) = call_and_answer(p, round);
        let at = SeqState::swing_at(at, k, p[CR_SWING], step_frames);
        let gate = step_frames * p[CR_GATE] as f64;
        let phase = if pos < len {
            if let Some(d) = call[pos as usize] {
                self.note(ctx, at, pitch(ctx, p[CR_BASE], d as i32), p[CR_VELOCITY], gate, out);
            }
            0
        } else if pos >= len + gap && pos < 2 * len + gap {
            let i = (pos - len - gap) as usize;
            if let Some(d) = answer[i] {
                let octave = 7 * p[CR_OCTAVE].round() as i32;
                let v = p[CR_VELOCITY] * p[CR_ANSWER_VELOCITY].clamp(0.0, 1.0);
                // The choir: the tune, a third below, a third above.
                let voices = (p[CR_CHOIR].round() as usize).clamp(1, 3);
                for (c, shift) in [0, -2, 2].iter().enumerate().take(voices) {
                    let note = pitch(ctx, p[CR_BASE], d as i32 + octave + shift);
                    self.note(ctx, at, note, v * if c == 0 { 1.0 } else { 0.8 }, gate, out);
                }
            }
            1
        } else {
            2
        };
        let pack = |ph: &Phrase| -> u64 { (0..PHRASE_LEN).fold(0u64, |acc, i| acc | (ph[i].map(|d| (d as i64 + 5).clamp(0, 14) as u64).unwrap_or(15)) << (i * 4)) };
        self.visual[0] = pos as u64 | (period as u64) << 8 | (len as u64) << 16 | (round as u64 & 0xffff) << 24 | (phase as u64) << 40 | (kind as u64) << 42;
        self.visual[1] = pack(&call);
        self.visual[2] = pack(&answer);
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
    fn every_world_sequencer_plays() {
        for kind in [MidiFxKind::Gamelan, MidiFxKind::Raga, MidiFxKind::Call] {
            let ons = run(kind, &defaults(kind.specs()), &kind.default_data(), 16.0);
            assert!(ons.len() >= 8, "{kind:?}: {} notes", ons.len());
            assert!(ons.iter().all(|o| (24..=110).contains(&o.1)), "{kind:?}");
            let mut d: Vec<u8> = ons.iter().map(|o| o.1).collect();
            d.sort();
            d.dedup();
            assert!(d.len() >= 3, "{kind:?}: {d:?}");
        }
        // Gamelan and Call stay in the key.
        let c = Scale::new(0, ScaleKind::Major);
        for kind in [MidiFxKind::Gamelan, MidiFxKind::Call] {
            let ons = run(kind, &defaults(kind.specs()), &[], 16.0);
            assert!(ons.iter().all(|o| c.contains(o.1)), "{kind:?}");
        }
    }

    #[test]
    fn gamelan_parts_interlock_and_the_gong_ends_the_cycle() {
        for style in 0..KOTEKAN.len() {
            for m in [2usize, 4, 8] {
                for (a, b) in [(0, 2), (3, 1), (2, 2)] {
                    let mut together = 0;
                    for s in 0..m {
                        let (po, sa) = kotekan(style, a, b, s, m);
                        assert!(po.is_some() || sa.is_some(), "a note every step: {style} {m} {s}");
                        if po.is_some() && sa.is_some() {
                            together += 1;
                        }
                    }
                    // Norot and Nyog Cag: never both at once, one line between them.
                    if style < 2 {
                        assert_eq!(together, 0);
                    }
                    // Each ends on the next core note.
                    let (po, _) = kotekan(style, a, b, m - 1, m);
                    assert_eq!(po, Some(b), "{style} {m}");
                }
            }
        }
        // Eight core notes of four sixteenths: the gong (two octaves and more below) at the last one.
        let p = defaults(GAMELAN);
        let ons = run(MidiFxKind::Gamelan, &p, &[], 16.0);
        let gongs: Vec<f64> = ons.iter().filter(|o| o.1 == 36).map(|o| o.0).collect();
        assert_eq!(gongs, vec![7.75, 15.75], "{gongs:?}");
        // The core's last note is the gong tone.
        for c in 0..4 {
            assert_eq!(balungan(3, c, 0.5, 16)[15], 0);
        }
    }

    #[test]
    fn raga_goes_up_and_down_by_its_rules() {
        let mut p = defaults(RAGA);
        p[RA_STAGE] = 1.0; // Jor
        p[RA_PAKAD] = 0.0;
        p[RA_ORNAMENTS] = 0.0;
        p[RA_DRONE] = 0.0;
        let ons = run(MidiFxKind::Raga, &p, &[], 64.0);
        assert!(ons.len() > 40);
        let yaman = RAGA_NOTES[0];
        let ups: Vec<i8> = yaman.up[..yaman.n_up].to_vec();
        let downs: Vec<i8> = yaman.down[..yaman.n_down].to_vec();
        let mut went_down_to_pa = false;
        for w in ons.windows(2) {
            let (a, b) = (w[0].1 as i32, w[1].1 as i32);
            let pc = (b - 60).rem_euclid(12) as i8;
            if b > a {
                assert!(ups.contains(&pc), "{a} up to {b}");
            } else if b < a {
                assert!(downs.contains(&pc), "{a} down to {b}");
                went_down_to_pa |= pc == 7;
            }
        }
        assert!(went_down_to_pa, "Pa only coming down");
        // With the pakad always: its phrase comes back.
        p[RA_PAKAD] = 1.0;
        p[RA_LINGER] = 1.0;
        let ons = run(MidiFxKind::Raga, &p, &[], 64.0);
        let notes: Vec<i32> = ons.iter().map(|o| o.1 as i32 - 60).collect();
        let pakad: Vec<i32> = yaman.pakad[..yaman.n_pakad].iter().map(|&x| x as i32).collect();
        assert!(notes.windows(pakad.len()).any(|w| w == pakad.as_slice()), "{notes:?}");
    }

    #[test]
    fn call_is_answered() {
        let mut p = defaults(CALL);
        p[CR_ANSWER] = 0.0; // Echo
        p[CR_CHOIR] = 1.0;
        p[CR_GAP] = 2.0;
        p[CR_LENGTH] = 6.0;
        let ons = run(MidiFxKind::Call, &p, &[], 8.0);
        // Six eighths of call, a gap of two, the answer the same.
        let at = |t: f64| (t * 100.0).round() as i64;
        let call: Vec<(i64, u8)> = ons.iter().filter(|o| o.0 < 3.0).map(|o| (at(o.0), o.1)).collect();
        let answer: Vec<(i64, u8)> = ons.iter().filter(|o| o.0 >= 4.0 - 1e-3 && o.0 < 7.0).map(|o| (at(o.0 - 4.0), o.1)).collect();
        assert!(!call.is_empty());
        assert_eq!(call, answer);
        // Home: the answer ends on the key note, the call never does.
        for round in 0..8 {
            p[CR_ANSWER] = 3.0;
            let (c, a, _) = call_and_answer(&p, round);
            let last = (0..6).rev().find(|&i| c[i].is_some()).unwrap();
            assert_ne!((c[last].unwrap() as i32).rem_euclid(7), 0);
            assert_eq!((a[last].unwrap() as i32).rem_euclid(7), 0);
        }
    }
}
