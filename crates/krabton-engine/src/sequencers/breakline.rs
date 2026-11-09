//! Breakline: a melody for breakbeat, in the song's key, from a seed, in
//! one of four manners:
//! - Lullaby: a slow, sad tune of a few notes wandering by steps and leaps
//!   over the breaks, held long, coming home at the end of its phrase (the
//!   music-box melodies laid over drill'n'bass).
//! - Runs: scales up and down at speed, a run of three to eight notes in one
//!   direction then another from somewhere else (the bass and lead runs of
//!   jazz-trained drill'n'bass).
//! - Arp Rush: a minor chord broken up fast, up, down or both ways, the
//!   chord changing every bar (breakcore's orchestral arpeggios).
//! - Acid: the root, mostly, an octave up or down, a few notes about it,
//!   hard accents.
//!
//! As Breaks has it: a *Phrase* of 1 to 8 bars that comes round again,
//! *Vary* for how much its later bars differ from its first, *Mutate* to
//! throw the dice again every phrase or every bar (or *Stay*), a *Seed*;
//! *Ratchet* rolls a step into 2 to 4 notes (runs climbing on), *Jumps*
//! takes notes an octave up or down.

use super::minimal::{choose, hash, DYNAMICS};
use super::{rate_beats, SeqState, RATES};
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

pub const RATE: usize = 0;
pub const STYLE: usize = 1;
pub const CENTER: usize = 2;
pub const RANGE: usize = 3;
pub const DENSITY: usize = 4;
pub const JUMPS: usize = 5;
pub const RATCHET: usize = 6;
pub const PHRASE: usize = 7;
pub const VARY: usize = 8;
pub const MUTATE: usize = 9;
pub const SEED: usize = 10;
pub const GATE: usize = 11;
pub const VELOCITY: usize = 12;
pub const ACCENT: usize = 13;

pub const STYLES: &[&str] = &["Lullaby", "Runs", "Arp Rush", "Acid"];
pub const PHRASES: &[&str] = &["1 Bar", "2 Bars", "4 Bars", "8 Bars"];
const PHRASE_BARS: [i64; 4] = [1, 2, 4, 8];
pub const MUTATES: &[&str] = &["Stay", "Every phrase", "Every bar"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Style", "Line", STYLES, 0),
    ParamSpec::int("Center", "Notes", 36.0, 96.0, 72.0, Unit::None),
    ParamSpec::int("Range", "Notes", 3.0, 21.0, 9.0, Unit::None),
    ParamSpec::lin("Density", "Line", 0.05, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Jumps", "Line", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::lin("Ratchet", "Line", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::choice("Phrase", "Dice", PHRASES, 2),
    ParamSpec::lin("Vary", "Dice", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::choice("Mutate", "Dice", MUTATES, 0),
    ParamSpec::int("Seed", "Dice", 0.0, 255.0, 0.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.5, 0.8, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Accent", "Notes", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
    ParamSpec::toggle("Band", "Band", false),
    ParamSpec::choice("Follow", "Band", crate::band::FOLLOWS, 1),
];

/// What a step plays: a degree of the key from Center, octaves on top, how
/// many notes in the step (each so many degrees on), and how hard.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Note {
    pub degree: i32,
    pub octave: i32,
    pub hits: u8,
    pub climb: i32,
    pub vel: f32,
}

/// Steps in a bar at this rate.
pub fn per_bar(p: &[f32]) -> i64 {
    ((4.0 / rate_beats(p[RATE])).round() as i64).max(1)
}

/// The dice of bar `bar`, step `i`: a step of a later bar of the phrase
/// takes its own (by Vary) or the phrase's first bar's.
struct Dice {
    seed: u32,
    epoch: u32,
    pos: u32,
    vary: f32,
}

impl Dice {
    fn new(p: &[f32], bar: i64) -> Self {
        let phrase = PHRASE_BARS[choose(p, PHRASE, PHRASE_BARS.len())];
        let epoch = match choose(p, MUTATE, MUTATES.len()) {
            0 => 0,
            1 => bar.div_euclid(phrase),
            _ => bar,
        };
        Self { seed: (p[SEED].round() as u32).wrapping_mul(0x9E37) ^ 0xB4EA, epoch: epoch as u32, pos: bar.rem_euclid(phrase) as u32, vary: p[VARY].clamp(0.0, 1.0) }
    }

    fn raw(&self, i: i64, salt: u32, bar: u32) -> f32 {
        hash(self.seed ^ self.epoch.wrapping_mul(0x85EB_CA6B) ^ salt.wrapping_mul(0x27D4_EB2F), (bar.wrapping_mul(977)).wrapping_add(i as u32))
    }

    /// 0..1 for step `i` and what it is for (`salt`).
    fn roll(&self, i: i64, salt: u32) -> f32 {
        let own = self.pos == 0 || self.raw(i, 0xBA5E, self.pos) < self.vary;
        self.raw(i, salt, if own { self.pos } else { 0 })
    }
}

/// Whether step `i` of a bar of `n` plays at all, in this manner.
fn plays(style: usize, d: &Dice, i: i64, n: i64, density: f32) -> bool {
    let beat = n / 4;
    let strong = beat > 0 && i % beat == 0;
    let r = d.roll(i, 1);
    match style {
        // Few notes, on the beats mostly.
        0 => {
            if strong {
                r < 0.35 + 0.65 * density
            } else {
                r < density * 0.35
            }
        }
        // Nearly every step.
        1 | 2 => r < 0.45 + 0.55 * density,
        _ => strong || r < density * 0.8,
    }
}

/// Step `k`'s note (None: a rest).
pub fn note_at(p: &[f32], k: i64) -> Option<Note> {
    let n = per_bar(p);
    let bar = k.div_euclid(n);
    let i = k.rem_euclid(n);
    let d = Dice::new(p, bar);
    let style = choose(p, STYLE, STYLES.len());
    let density = p[DENSITY].clamp(0.0, 1.0);
    if !plays(style, &d, i, n, density) {
        return None;
    }
    let range = p[RANGE].round().max(3.0) as i32;
    let (lo, hi) = (-(range / 2), range - range / 2);
    let phrase = PHRASE_BARS[choose(p, PHRASE, PHRASE_BARS.len())];
    let last_bar = d.pos as i64 == phrase - 1;
    let mut degree = match style {
        0 => {
            // A walk from the root by steps and the odd leap, kept in range;
            // home at the end of the phrase.
            let moves = [0, 1, -1, 2, -2, 1, -1, 3, -3, 4, -4, 2];
            let mut deg = 0;
            for j in 0..=i {
                if j == 0 || plays(style, &d, j, n, density) {
                    let m = moves[(d.roll(j, 2) * moves.len() as f32) as usize % moves.len()];
                    deg = (deg + m).clamp(lo, hi);
                }
            }
            if last_bar && i >= n - n / 4 {
                0
            } else {
                deg
            }
        }
        1 => {
            // Runs: from where each starts, one way, for three to eight steps.
            let mut start = 0;
            let mut j = 0;
            let mut deg = 0;
            while j <= i {
                let len = 3 + (d.roll(start, 3) * 6.0) as i64;
                let dir = if d.roll(start, 4) < 0.5 { 1 } else { -1 };
                let from = lo + (d.roll(start, 5) * (hi - lo + 1) as f32) as i32;
                if i < start + len {
                    let mut v = from + dir * (i - start) as i32;
                    // Turn back at the edges of the range.
                    let span = (hi - lo).max(1);
                    let x = (v - lo).rem_euclid(2 * span);
                    v = lo + if x > span { 2 * span - x } else { x };
                    deg = v;
                    break;
                }
                start += len;
                j = start;
            }
            deg
        }
        2 => {
            // A minor progression, a chord a bar, broken up.
            const PROGS: [[i32; 4]; 4] = [[0, 5, 3, 4], [0, 3, 5, 4], [0, 6, 5, 4], [0, 2, 5, 6]];
            let prog = PROGS[(d.seed >> 3) as usize % PROGS.len()];
            let root = prog[bar.rem_euclid(4) as usize];
            let tones = [0, 2, 4, 7, 9, 11];
            let span = ((range as usize * 6) / 14).clamp(3, 6);
            let way = (d.roll(0, 6) * 4.0) as i64;
            let at = match way {
                0 => i % span as i64,
                1 => span as i64 - 1 - i % span as i64,
                2 => {
                    let c = i % (2 * span as i64 - 2).max(1);
                    if c < span as i64 { c } else { 2 * span as i64 - 2 - c }
                }
                _ => (d.roll(i, 7) * span as f32) as i64,
            } as usize;
            (root + tones[at.min(tones.len() - 1)] + lo / 2).clamp(lo - 3, hi + 7)
        }
        _ => {
            // Acid: the root, mostly.
            let r = d.roll(i, 8);
            if r < 0.55 {
                0
            } else {
                let others = [2, 4, -1, 6, -3, 1];
                others[((r - 0.55) / 0.45 * others.len() as f32) as usize % others.len()]
            }
        }
    };
    let mut octave = 0;
    if d.roll(i, 9) < p[JUMPS].clamp(0.0, 1.0) {
        octave = if d.roll(i, 10) < 0.6 { 1 } else { -1 };
    }
    if style == 3 && octave == 0 && d.roll(i, 11) < 0.15 {
        octave = 1;
    }
    let (hits, climb) = if d.roll(i, 12) < p[RATCHET].clamp(0.0, 1.0) {
        let hits = [2u8, 3, 4][(d.roll(i, 13) * 3.0) as usize % 3];
        let climb = match style {
            1 => {
                if d.roll(i, 14) < 0.5 {
                    1
                } else {
                    -1
                }
            }
            2 => 2,
            _ => 0,
        };
        (hits, climb)
    } else {
        (1, 0)
    };
    // Accents: on the beat, and Acid's by chance.
    let beat = (n / 4).max(1);
    let accent = p[ACCENT].clamp(0.0, 1.0);
    let strong = i % beat == 0 || (style == 3 && d.roll(i, 15) < 0.3);
    let vel = p[VELOCITY] * if strong { 1.0 } else { 1.0 - 0.45 * accent };
    if style == 0 {
        degree = degree.clamp(lo - 2, hi + 2);
    }
    Some(Note { degree, octave, hits, climb, vel })
}

/// Steps to the next note (for how long this one holds).
fn gap(p: &[f32], k: i64) -> i64 {
    (1..16).find(|d| note_at(p, k + d).is_some()).unwrap_or(16)
}

impl SeqState {
    pub(crate) fn breakline(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        // Where it is, for the face (rests too).
        self.visual[0] = k.max(0) as u64;
        let Some(n) = note_at(p, k) else { return };
        let (home, _) = ctx.scale.locate(p[CENTER].round() as i32);
        let each = step_frames / n.hits as f64;
        let held = if n.hits > 1 { each * 0.8 } else { gap(p, k).min(8) as f64 * step_frames * p[GATE].clamp(0.1, 1.5) as f64 };
        for h in 0..n.hits as i32 {
            let pitch = ctx.scale.pitch_at(home + n.degree + n.climb * h) + 12 * n.octave;
            let fade = if n.hits > 1 { 1.0 - 0.12 * h as f32 } else { 1.0 };
            self.mlater(ctx, at + (h as f64 * each) as u64, pitch, n.vel * fade, held, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::MidiFxKind;
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};
    use crate::sequencers::tests::run;

    fn p() -> Vec<f32> {
        defaults(SPECS)
    }

    #[test]
    fn every_style_plays_in_the_key() {
        let c = Scale::new(0, ScaleKind::Major);
        for style in 0..STYLES.len() {
            let mut q = p();
            q[STYLE] = style as f32;
            let ons = run(MidiFxKind::Breakline, &q, &MidiFxKind::Breakline.default_data(), 16.0);
            assert!(ons.len() >= 8, "{}: {}", STYLES[style], ons.len());
            assert!(ons.iter().all(|o| c.contains(o.1)), "{}: {ons:?}", STYLES[style]);
        }
    }

    #[test]
    fn the_phrase_comes_round_and_varies_as_asked() {
        let mut q = p();
        q[STYLE] = 1.0;
        let n = per_bar(&q);
        let bar = |q: &[f32], b: i64| (0..n).map(|i| note_at(q, b * n + i)).collect::<Vec<_>>();
        assert_eq!(bar(&q, 0), bar(&q, 4), "a four-bar phrase, round again");
        q[VARY] = 0.0;
        assert_eq!(bar(&q, 0), bar(&q, 1), "no variation: the first bar again");
        q[VARY] = 1.0;
        assert_ne!(bar(&q, 0), bar(&q, 1));
        q[MUTATE] = 2.0;
        assert_ne!(bar(&q, 0), bar(&q, 4), "every bar new dice");
        q[MUTATE] = 0.0;
        q[SEED] = 9.0;
        let other = bar(&q, 0);
        q[SEED] = 0.0;
        assert_ne!(bar(&q, 0), other, "another seed, another line");
    }

    #[test]
    fn runs_run_and_ratchets_roll() {
        let mut q = p();
        q[STYLE] = 1.0;
        q[DENSITY] = 1.0;
        q[RATCHET] = 0.0;
        q[JUMPS] = 0.0;
        let n = per_bar(&q);
        let degs: Vec<i32> = (0..n).filter_map(|i| note_at(&q, i)).map(|x| x.degree).collect();
        let steps = degs.windows(2).filter(|w| (w[0] - w[1]).abs() == 1).count();
        assert!(steps * 2 >= degs.len(), "mostly by step: {degs:?}");
        q[RATCHET] = 1.0;
        assert!((0..n).filter_map(|i| note_at(&q, i)).all(|x| x.hits >= 2));
        // The lullaby comes home at the end of its phrase.
        let mut l = p();
        l[DENSITY] = 1.0;
        l[JUMPS] = 0.0;
        let last = 3 * n + n - 1;
        assert!(note_at(&l, last).is_none_or(|x| x.degree == 0));
    }
}
