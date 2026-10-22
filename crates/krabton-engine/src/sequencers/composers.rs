//! More sequencers after composers and the ways they are known by. All of
//! them play in the song's key (Debussy's whole tones and parallel chords
//! step out of it on purpose, as his do).
//!
//! - Reich, after Steve Reich: Phase (two players on the same twelve-note
//!   figure, the second slowly pulling ahead until it is a note further on,
//!   then holding there, round all twelve), Shifting Rhythm (a clapped
//!   rhythm against itself, the second jumping a step on now and then), or
//!   Pulses (chords pulsing on every step and swelling like breaths).
//! - Bach, after J. S. Bach: a Prelude (a broken chord, the same figure each
//!   half bar, the voices led smoothly from chord to chord), an Invention
//!   (a motif answered a half bar later an octave down, the two of them
//!   falling in sequence) or a Chorale (four voices, a chord a beat,
//!   cadences held).
//! - Nancarrow, after Conlon Nancarrow's studies for player piano: one
//!   melody in two to four voices at once at tempos in a ratio (3:4, 3:4:5,
//!   4:5:6:7, 2:3, or one an irrational step faster), started so that they
//!   all arrive at its end together (or all start together and drift apart).
//! - Debussy, after Claude Debussy: Planing (a line harmonised by the same
//!   chord shape all the way, sliding in parallel), Whole Tone (washes up
//!   and down the whole-tone scale over an augmented chord), Arabesque
//!   (waves of triplets over the chords of the key) or Pentatonic (a line in
//!   five notes over parallel fourths and an open fifth).
//! - Feldman, after Morton Feldman: soft chords, few and far apart, in
//!   patterns that come back a few times, each time a little changed, before
//!   another takes their place.

use super::minimal::{choose, chord_root, hash, CHORD_AMOUNT, DYNAMICS};
use super::{SeqState, RATES};
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

const KEYS: [ParamSpec; 2] = [ParamSpec::toggle("Transpose", "Keys", false), ParamSpec::toggle("Hold", "Keys", false)];

// ---------------------------------------------------------------- reich

pub const RE_STYLE: usize = 1;
pub const RE_SHIFT: usize = 2;
pub const RE_PATTERN: usize = 3;
pub const RE_BASE: usize = 4;
pub const RE_GATE: usize = 5;
pub const RE_VELOCITY: usize = 6;
/// (came later: after the keys and the dynamics)
pub const RE_CHORD_AMOUNT: usize = 13;
pub const REICH_STYLES: &[&str] = &["Phase", "Shifting Rhythm", "Pulses"];
/// Each step of Reich is cut in eighths (for the phasing).
pub const REICH_TICKS: f64 = 8.0;

pub static REICH: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Style", "Reich", REICH_STYLES, 0),
    ParamSpec::int("Repeats", "Reich", 2.0, 32.0, 8.0, Unit::None),
    ParamSpec::int("Pattern", "Reich", 0.0, 7.0, 0.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 36.0, 84.0, 64.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
    CHORD_AMOUNT,
];

/// Rhythms of twelve, as groups of claps with a rest after each.
const GROUPS: [&[u8]; 8] = [&[3, 2, 1, 2], &[2, 2, 1, 3], &[1, 3, 2, 2], &[4, 1, 2, 1], &[2, 1, 2, 3], &[1, 2, 3, 2], &[2, 3, 2, 1], &[1, 1, 2, 4]];

/// The rhythm `i` as twelve steps (a bit each, step 0 the lowest).
pub fn reich_rhythm(i: usize) -> u16 {
    let mut bits = 0u16;
    let mut s = 0;
    for &g in GROUPS[i % GROUPS.len()] {
        for _ in 0..g {
            if s < 12 {
                bits |= 1 << s;
            }
            s += 1;
        }
        s += 1;
    }
    bits
}

/// The twelve-note figure `i`: five notes of the key (within its octave)
/// in a restless order.
pub fn reich_figure(i: usize) -> [i32; 12] {
    let mut pool = [0i32, 1, 2, 3, 4, 5, 6];
    for j in (1..pool.len()).rev() {
        let k = (hash(i as u32 + 11, j as u32) * (j + 1) as f32) as usize % (j + 1);
        pool.swap(j, k);
    }
    let pool = &pool[..5];
    let mut out = [0i32; 12];
    for j in 0..12 {
        let mut k = (hash(i as u32 * 7 + 3, j as u32) * 5.0) as usize % 5;
        if j > 0 && pool[k] == out[j - 1] {
            k = (k + 1) % 5;
        }
        out[j] = pool[k];
    }
    out
}

// ---------------------------------------------------------------- bach

pub const BA_STYLE: usize = 1;
pub const BA_HARMONY: usize = 2;
pub const BA_VARIATION: usize = 3;
pub const BA_BASE: usize = 4;
pub const BA_GATE: usize = 5;
pub const BA_VELOCITY: usize = 6;
pub const BA_CHORD_AMOUNT: usize = 13;
pub const BACH_STYLES: &[&str] = &["Prelude", "Invention", "Chorale"];
pub const BACH_HARMONIES: &[&str] = &["Wandering", "Circle", "Lament"];

pub static BACH: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Style", "Bach", BACH_STYLES, 0),
    ParamSpec::choice("Harmony", "Bach", BACH_HARMONIES, 0),
    ParamSpec::int("Variation", "Bach", 0.0, 15.0, 0.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 48.0, 84.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.9, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.65, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
    CHORD_AMOUNT,
];

const CIRCLE: [i32; 8] = [0, 3, 6, 2, 5, 1, 4, 0];
const LAMENT_ROOTS: [i32; 4] = [0, 4, 3, 4];
const LAMENT_BASS: [i32; 4] = [0, -1, -2, -3];

// ---------------------------------------------------------------- nancarrow

pub const NA_RATIOS: usize = 1;
pub const NA_LENGTH: usize = 2;
pub const NA_VARIATION: usize = 3;
pub const NA_CONVERGE: usize = 4;
pub const NA_BASE: usize = 5;
pub const NA_SPREAD: usize = 6;
pub const NA_GATE: usize = 7;
pub const NA_VELOCITY: usize = 8;
pub const NANCARROW_RATIOS: &[&str] = &["3:4", "3:4:5", "4:5:6:7", "2:3", "Irrational"];
pub const NANCARROW_TICKS: f64 = 12.0;
const SPEEDS: [&[f64]; 5] = [&[3.0, 4.0], &[3.0, 4.0, 5.0], &[4.0, 5.0, 6.0, 7.0], &[2.0, 3.0], &[1.0, std::f64::consts::SQRT_2]];

pub static NANCARROW: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 4),
    ParamSpec::choice("Ratios", "Nancarrow", NANCARROW_RATIOS, 1),
    ParamSpec::int("Length", "Nancarrow", 8.0, 32.0, 16.0, Unit::None),
    ParamSpec::int("Variation", "Nancarrow", 0.0, 15.0, 0.0, Unit::None),
    ParamSpec::toggle("Converge", "Nancarrow", true),
    ParamSpec::int("Base Note", "Notes", 36.0, 84.0, 60.0, Unit::None),
    ParamSpec::lin("Spread", "Notes", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

/// Nancarrow's melody `var` of `n` notes: (degree, length in units).
pub fn nancarrow_melody(var: usize, n: usize) -> Vec<(i32, u32)> {
    let mut d = 2i32;
    (0..n)
        .map(|j| {
            let u = hash(var as u32 * 31 + 7, j as u32);
            let step = if u < 0.3 { 1 } else if u < 0.55 { -1 } else if u < 0.7 { 2 } else if u < 0.85 { -2 } else if u < 0.93 { 4 } else { -4 };
            d = (d + step).clamp(-3, 10);
            let len = [1u32, 1, 1, 2, 2, 3][(hash(var as u32 * 17 + 1, j as u32) * 6.0) as usize % 6];
            (d, len)
        })
        .collect()
}

// ---------------------------------------------------------------- debussy

pub const DE_STYLE: usize = 1;
pub const DE_SHAPE: usize = 2;
pub const DE_MOTION: usize = 3;
pub const DE_BASE: usize = 4;
pub const DE_PEDAL: usize = 5;
pub const DE_VELOCITY: usize = 6;
pub const DE_CHORD_AMOUNT: usize = 13;
pub const DEBUSSY_STYLES: &[&str] = &["Planing", "Whole Tone", "Arabesque", "Pentatonic"];
pub const DEBUSSY_SHAPES: &[&str] = &["Ninth", "Major Seventh", "Fourths", "Triad"];
/// The chord shapes, semitones up from the lowest note.
pub const SHAPES: [&[i32]; 4] = [&[0, 4, 7, 10, 14], &[0, 4, 7, 11], &[0, 5, 10, 15], &[0, 4, 7]];
pub const DEBUSSY_TICKS: f64 = 12.0;

pub static DEBUSSY: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 5),
    ParamSpec::choice("Style", "Debussy", DEBUSSY_STYLES, 0),
    ParamSpec::choice("Shape", "Debussy", DEBUSSY_SHAPES, 0),
    ParamSpec::lin("Motion", "Debussy", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Base Note", "Notes", 48.0, 84.0, 67.0, Unit::None),
    ParamSpec::lin("Pedal", "Notes", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.55, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
    CHORD_AMOUNT,
];

// ---------------------------------------------------------------- feldman

pub const FE_DENSITY: usize = 1;
pub const FE_CLUSTER: usize = 2;
pub const FE_COLOR: usize = 3;
pub const FE_REPEAT: usize = 4;
pub const FE_BASE: usize = 5;
pub const FE_SPREAD: usize = 6;
pub const FE_VELOCITY: usize = 7;
pub const FELDMAN_TICKS: f64 = 2.0;
pub const MAX_EVENTS: usize = 4;

pub static FELDMAN: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 5),
    ParamSpec::lin("Density", "Feldman", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::int("Cluster", "Feldman", 1.0, 5.0, 3.0, Unit::None),
    ParamSpec::lin("Color", "Feldman", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Repeat", "Feldman", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::int("Base Note", "Notes", 36.0, 84.0, 60.0, Unit::None),
    ParamSpec::int("Octaves", "Notes", 1.0, 3.0, 2.0, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.28, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

// ---------------------------------------------------------------- state

/// One of Feldman's events: when in the pattern, how long, which notes.
#[derive(Clone, Copy, Default)]
struct Event {
    start: u8,
    len: u8,
    notes: [i8; 5],
    n: u8,
}

pub struct Composers {
    rng: u32,
    // Reich: the second player's last step.
    re_last: i64,
    // Bach
    ba_bar: i64,
    ba_count: i64,
    ba_root: i32,
    ba_bass: i32,
    /// The chord after this one, chosen ahead (for a passing chord into it).
    ba_next: Option<(i32, i32)>,
    /// The chord sounding (its own, home, or passing).
    ba_shown: i32,
    ba_voices: [i32; 5],
    ba_voiced: bool,
    ba_hist: u64,
    ba_top: u64,
    // Debussy
    de_top: i32,
    de_last: i64,
    de_hist: u64,
    // Feldman
    fe_events: [Event; MAX_EVENTS],
    fe_n: usize,
    fe_len: u8,
    fe_pos: u8,
    fe_left: u32,
    fe_ready: bool,
}

impl Default for Composers {
    fn default() -> Self {
        Self {
            rng: 0xB4C_4,
            re_last: i64::MIN,
            ba_bar: i64::MIN,
            ba_count: 0,
            ba_root: 0,
            ba_bass: 0,
            ba_next: None,
            ba_shown: 0,
            ba_voices: [-14, -5, 0, 2, 4],
            ba_voiced: false,
            ba_hist: 0,
            ba_top: 0,
            de_top: 4,
            de_last: i64::MIN,
            de_hist: 0,
            fe_events: [Event::default(); MAX_EVENTS],
            fe_n: 0,
            fe_len: 16,
            fe_pos: 0,
            fe_left: 0,
            fe_ready: false,
        }
    }
}

impl Composers {
    fn unit(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / 16_777_216.0
    }

    /// Bach's next chord (root, bass) at place `i` of an eight-chord phrase.
    fn next_chord(&mut self, harmony: usize, i: i64) -> (i32, i32) {
        match harmony {
            1 => {
                let r = CIRCLE[i.rem_euclid(8) as usize];
                (r, r)
            }
            2 => {
                let k = i.rem_euclid(4) as usize;
                (LAMENT_ROOTS[k], LAMENT_BASS[k])
            }
            _ => {
                // Where a chord likes to go; the phrase ends ii (or IV), V, I.
                let r = match i.rem_euclid(8) {
                    0 => 0,
                    5 => {
                        if self.unit() < 0.6 {
                            1
                        } else {
                            3
                        }
                    }
                    6 => 4,
                    7 => 0,
                    _ => {
                        let next: &[i32] = match self.ba_root.rem_euclid(7) {
                            0 => &[3, 1, 5, 4, 3],
                            1 => &[4, 6, 4],
                            2 => &[5, 3],
                            3 => &[4, 1, 0, 4],
                            4 => &[0, 5, 0],
                            5 => &[1, 3, 1],
                            _ => &[0],
                        };
                        next[(self.unit() * next.len() as f32) as usize % next.len()]
                    }
                };
                (r, r)
            }
        }
    }
}

/// Each voice to the nearest note of the chord (degrees `tones` above `root`),
/// from the lowest up, none crossing the one below.
fn lead(prev: &[i32], root: i32, tones: &[i32]) -> Vec<i32> {
    let in_chord = |d: i32| tones.iter().any(|&t| (d - root - t).rem_euclid(7) == 0);
    let mut out: Vec<i32> = Vec::with_capacity(prev.len());
    for &p in prev {
        let floor = out.last().map(|&l| l + 1).unwrap_or(i32::MIN / 2);
        let mut best = p;
        let mut found = false;
        for dist in 0..8 {
            for cand in [p - dist, p + dist] {
                if cand >= floor && in_chord(cand) {
                    best = cand;
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        if !found {
            best = floor.max(p);
        }
        out.push(best);
    }
    out
}

impl SeqState {
    // ------------------------------------------------------------ reich

    /// Reich, step `k` (an eighth of its step).
    pub(crate) fn reich(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let style = choose(p, RE_STYLE, REICH_STYLES.len());
        let repeats = (p[RE_SHIFT].round() as i64).clamp(2, 32);
        let pat = (p[RE_PATTERN].round().max(0.0) as usize) % 8;
        let base = p[RE_BASE];
        let gate = p[RE_GATE].clamp(0.1, 1.0) as f64;
        let vel = p[RE_VELOCITY];
        let step = 8.0 * step_frames;
        let on_step = k.rem_euclid(8) == 0;
        let s = k.div_euclid(8);
        match style {
            // Phase: B holds for `repeats` rounds, then pulls a note ahead over two.
            0 => {
                let fig = reich_figure(pat);
                let hold = 12 * repeats;
                let drift = 24;
                let period = hold + drift;
                let sf = k as f64 / 8.0;
                let c = (sf / period as f64).floor();
                let r = sf - c * period as f64;
                let frac = if r < hold as f64 { 0.0 } else { (r - hold as f64) / drift as f64 };
                let offset = c as i64 % 12;
                let pb = sf + offset as f64 + frac;
                let nb = pb.floor() as i64;
                if on_step {
                    let pitch = Self::mdeg(ctx, base, fig[s.rem_euclid(12) as usize], 0);
                    self.note(ctx, at, pitch, vel, step * gate, out);
                }
                if nb != self.composers.re_last {
                    self.composers.re_last = nb;
                    let pitch = Self::mdeg(ctx, base, fig[nb.rem_euclid(12) as usize] + 7, 0);
                    self.note(ctx, at, pitch, vel * 0.85, step * gate, out);
                }
                self.visual[0] = (style as u64) | (offset as u64 & 0xf) << 4 | ((frac * 15.0) as u64 & 0xf) << 8 | (pat as u64) << 12;
                self.visual[1] = s as u64;
                self.visual[2] = nb as u64;
            }
            // Shifting Rhythm: B the same rhythm, a step further on every `repeats` rounds.
            1 => {
                let bits = reich_rhythm(pat);
                let offset = (s / (12 * repeats)).rem_euclid(12);
                if on_step {
                    let a = s.rem_euclid(12) as u32;
                    let b = (s + offset).rem_euclid(12) as u32;
                    if bits >> a & 1 == 1 {
                        let pitch = Self::mdeg(ctx, base, 0, 0);
                        self.note(ctx, at, pitch, vel, step * gate * 0.8, out);
                    }
                    if bits >> b & 1 == 1 {
                        let pitch = Self::mdeg(ctx, base, 4, 0);
                        self.note(ctx, at, pitch, vel * 0.9, step * gate * 0.8, out);
                    }
                }
                self.visual[0] = (style as u64) | (offset as u64 & 0xf) << 4 | (pat as u64) << 12 | (bits as u64) << 16;
                self.visual[1] = s as u64;
            }
            // Pulses: the chord on every step, swelling like a breath.
            _ => {
                const CHORDS: [i32; 6] = [0, 4, 5, 3, 1, 4];
                let breath = 16i64;
                let bi = s.div_euclid(breath);
                let bp = s.rem_euclid(breath);
                let per = (repeats / 2).max(1);
                let ci = (bi / per).rem_euclid(CHORDS.len() as i64) as usize;
                let amount = p.get(RE_CHORD_AMOUNT).copied().unwrap_or(1.0);
                let late = if per >= 2 { bi.rem_euclid(per) >= per / 2 } else { bp >= breath / 2 };
                let root = chord_root(amount, ci as i64, CHORDS[ci], CHORDS[(ci + 1) % CHORDS.len()], late);
                if on_step {
                    let swell = (std::f64::consts::PI * (bp as f64 + 0.5) / breath as f64).sin() as f32;
                    let v = vel * (0.25 + 0.75 * swell);
                    for d in [-7, -3] {
                        let pitch = Self::mdeg(ctx, base, root + d, 0);
                        self.note(ctx, at, pitch, v * 0.8, step * gate, out);
                    }
                    let top = [2, 4, 6, 8][s.rem_euclid(4) as usize];
                    let pitch = Self::mdeg(ctx, base, root + top, 0);
                    self.note(ctx, at, pitch, v, step * gate, out);
                    // The held voices, one breath long.
                    if bp == 0 {
                        for d in [2, 4] {
                            let pitch = Self::mdeg(ctx, base, root + d + 7, 0);
                            self.note(ctx, at, pitch, vel * 0.45, step * breath as f64 * 0.95, out);
                        }
                    }
                }
                self.visual[0] = (style as u64) | (ci as u64 & 0xf) << 4 | (bp as u64 & 0xf) << 8 | ((root + 64) as u64 & 0xff) << 16;
                self.visual[1] = s as u64;
            }
        }
    }

    // ------------------------------------------------------------ bach

    /// Bach, step `k` (a sixteenth, by default).
    pub(crate) fn bach(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let style = choose(p, BA_STYLE, BACH_STYLES.len());
        let harmony = choose(p, BA_HARMONY, BACH_HARMONIES.len());
        let var = (p[BA_VARIATION].round().max(0.0) as u32) % 16;
        let base = p[BA_BASE];
        let gate = p[BA_GATE].clamp(0.1, 1.0) as f64;
        let vel = p[BA_VELOCITY];
        let amount = p.get(BA_CHORD_AMOUNT).copied().unwrap_or(1.0);
        // A chord a bar (sixteen steps), a beat (four) in a chorale.
        let span = if style == 2 { 4 } else { 16 };
        let unit = k.div_euclid(span);
        let t = k.rem_euclid(span);
        if t == 0 && self.composers.ba_bar != unit {
            let c = &mut self.composers;
            c.ba_bar = unit;
            let i = c.ba_count;
            c.ba_count += 1;
            let (own, own_bass) = match c.ba_next.take() {
                Some(n) => n,
                None => c.next_chord(harmony, i),
            };
            c.ba_root = own;
            c.ba_bass = own_bass;
            c.ba_next = Some(c.next_chord(harmony, i + 1));
            // Chords: fewer of its own leave home.
            let root = chord_root(amount.min(1.0), i, own, own, false);
            let bass = if root == own { own_bass } else { root };
            c.ba_shown = root;
            // A seventh on the dominant and the supertonic, now and then.
            let seventh = matches!(root.rem_euclid(7), 1 | 4) && hash(var, i as u32) < 0.6;
            let tones: &[i32] = if seventh { &[0, 2, 4, 6] } else { &[0, 2, 4] };
            if !c.ba_voiced {
                c.ba_voiced = true;
                c.ba_voices = if style == 2 { [-12, -5, -1, 2, 0] } else { [-14, -5, 0, 2, 4] };
            }
            let upper = if style == 2 { 3 } else { 4 };
            let led = lead(&c.ba_voices[1..=upper], root, tones);
            c.ba_voices[1..=upper].copy_from_slice(&led);
            // The bass: the chord's bass note, near where it was.
            let mut b = bass;
            while b > c.ba_voices[0] + 3 {
                b -= 7;
            }
            while b < c.ba_voices[0] - 3 {
                b += 7;
            }
            c.ba_voices[0] = b.clamp(-17, -7);
            c.ba_hist = (c.ba_hist << 4) | (root.rem_euclid(7) as u64 & 0xf);
            c.ba_top = (c.ba_top << 8) | ((c.ba_voices[upper] + 64) as u64 & 0xff);
        }
        // More Chords: at the half bar, now and then a passing chord into the next.
        if t == 8 && style != 2 && amount > 1.0 && self.composers.ba_bar == unit {
            let c = &mut self.composers;
            let i = c.ba_count - 1;
            let next = c.ba_next.map(|n| n.0).unwrap_or(0);
            let pass = chord_root(amount, i, c.ba_shown, next, true);
            if pass != c.ba_shown {
                c.ba_shown = pass;
                let led = lead(&c.ba_voices[1..=4], pass, &[0, 2, 4]);
                c.ba_voices[1..=4].copy_from_slice(&led);
                let mut b = pass;
                while b > c.ba_voices[0] + 3 {
                    b -= 7;
                }
                while b < c.ba_voices[0] - 3 {
                    b += 7;
                }
                c.ba_voices[0] = b.clamp(-17, -7);
                c.ba_hist = (c.ba_hist << 4) | (pass.rem_euclid(7) as u64 & 0xf);
            }
        }
        let v = self.composers.ba_voices;
        let note_len = step_frames * gate;
        match style {
            // A Prelude: bass and tenor held, the top three broken, twice a bar.
            0 => {
                const FIG: [usize; 8] = [0, 1, 2, 3, 4, 2, 3, 4];
                let h = t.rem_euclid(8) as usize;
                let voice = FIG[h];
                let len = match h {
                    0 => 8.0 * step_frames * 0.98,
                    1 => 7.0 * step_frames * 0.98,
                    _ => note_len,
                };
                let pitch = Self::mdeg(ctx, base, v[voice], 0);
                let accent = if h == 0 { 1.0 } else { 0.85 };
                self.note(ctx, at, pitch, vel * accent, len, out);
            }
            // An Invention: the motif above, answered a half bar later an octave down.
            1 => {
                let motif = invention_motif(var);
                let half = k.div_euclid(8);
                let pos = k.rem_euclid(8) as usize;
                // The motif falls a step each half bar through the phrase, then home.
                let phrase = half.div_euclid(8);
                let fall = |h: i64| -(h.rem_euclid(8) as i32).min(5) + if h.rem_euclid(8) >= 6 { 5 } else { 0 };
                let inverted = phrase.rem_euclid(2) == 1;
                let m = |h: i64, j: usize| {
                    let d = motif[j];
                    fall(h) + if inverted { -d + 4 } else { d }
                };
                let up = Self::mdeg(ctx, base, m(half, pos), 0);
                self.note(ctx, at, up, vel, note_len, out);
                if half.rem_euclid(8) > 0 {
                    let low = Self::mdeg(ctx, base, m(half - 1, pos) - 7, 0);
                    self.note(ctx, at, low, vel * 0.85, note_len, out);
                } else if pos == 0 {
                    // The phrase begins over the tonic, held.
                    let low = Self::mdeg(ctx, base, -7, 0);
                    self.note(ctx, at, low, vel * 0.8, 8.0 * step_frames * 0.95, out);
                }
                self.composers.ba_top = (self.composers.ba_top & !0xff) | ((m(half, pos) + 64) as u64 & 0xff);
            }
            // A Chorale: four voices a beat; the phrase's last chord held.
            _ => {
                if t == 0 {
                    let i = self.composers.ba_count - 1;
                    let cadence = i.rem_euclid(8) == 7;
                    let len = 4.0 * step_frames * if cadence { 1.9 } else { 0.95 };
                    // (a held chord takes the next beat too: the next is silent)
                    if i.rem_euclid(8) != 0 || i == 0 {
                        for &d in &v[..4] {
                            let pitch = Self::mdeg(ctx, base, d, 0);
                            self.note(ctx, at, pitch, vel * if cadence { 0.9 } else { 0.8 }, len, out);
                        }
                    }
                }
            }
        }
        let c = &self.composers;
        self.visual[0] = (style as u64) | (harmony as u64) << 4 | ((c.ba_shown + 64) as u64 & 0xff) << 8 | (t as u64 & 0xff) << 16;
        self.visual[1] = c.ba_hist;
        self.visual[2] = c.ba_top;
        self.visual[3] = k as u64;
    }

    // ------------------------------------------------------------ nancarrow

    /// Nancarrow, step `k` (a twelfth of its unit).
    pub(crate) fn nancarrow(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let speeds = SPEEDS[choose(p, NA_RATIOS, NANCARROW_RATIOS.len())];
        let n = (p[NA_LENGTH].round() as usize).clamp(8, 32);
        let var = (p[NA_VARIATION].round().max(0.0) as usize) % 16;
        let converge = p[NA_CONVERGE] >= 0.5;
        let base = p[NA_BASE];
        let spread = p[NA_SPREAD].clamp(0.0, 1.0);
        let gate = p[NA_GATE].clamp(0.1, 1.0) as f64;
        let vel = p[NA_VELOCITY];
        // In steps: the slowest voice's note lasts twelve.
        let melody = nancarrow_melody(var, n);
        let total: u32 = melody.iter().map(|m| m.1).sum();
        let slowest = speeds.iter().copied().fold(f64::MAX, f64::min);
        let unit = |i: usize| 12.0 * slowest / speeds[i];
        let dur = |i: usize| total as f64 * unit(i);
        let longest = (0..speeds.len()).map(dur).fold(0.0, f64::max);
        let cycle = longest + 24.0;
        let t0 = k as f64;
        let local = t0 - (t0 / cycle).floor() * cycle;
        let nv = speeds.len();
        let mut progress = 0u64;
        for i in 0..nv {
            let off = if converge { longest - dur(i) } else { 0.0 };
            let oct = (spread * (i as f32 - (nv - 1) as f32 / 2.0) * 1.2).round() as i32 * 7;
            let mut cum = 0u32;
            for &(d, len) in &melody {
                let s = off + cum as f64 * unit(i);
                if s >= local && s < local + 1.0 {
                    let when = at + ((s - local) * step_frames) as u64;
                    let pitch = Self::mdeg(ctx, base, d + oct, 0);
                    self.mlater(ctx, when, pitch, vel * if cum == 0 { 1.0 } else { 0.85 }, len as f64 * unit(i) * step_frames * gate, out);
                }
                cum += len;
            }
            let pr = ((local - off) / dur(i)).clamp(0.0, 1.0);
            progress |= ((pr * 255.0) as u64 & 0xff) << (8 * i);
        }
        self.visual[0] = (nv as u64) | ((local / cycle * 255.0) as u64 & 0xff) << 8 | (converge as u64) << 16 | ((longest / cycle * 255.0) as u64 & 0xff) << 24;
        self.visual[1] = progress;
        self.visual[2] = k as u64;
    }

    // ------------------------------------------------------------ debussy

    /// Debussy, step `k` (a twelfth of its beat).
    pub(crate) fn debussy(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let style = choose(p, DE_STYLE, DEBUSSY_STYLES.len());
        let shape = SHAPES[choose(p, DE_SHAPE, DEBUSSY_SHAPES.len())];
        let motion = p[DE_MOTION].clamp(0.0, 1.0);
        let base = p[DE_BASE];
        let pedal = p[DE_PEDAL].clamp(0.0, 1.0) as f64;
        let vel = p[DE_VELOCITY];
        let beat = 12.0 * step_frames;
        let bar = k.div_euclid(48);
        let t = k.rem_euclid(48);
        // The tonic in the octave of the base note, and a low one.
        let tonic = Self::mdeg(ctx, base, 0, 0);
        let low = Self::mdeg(ctx, base, -14, 0);
        match style {
            // Planing: the line's every note with the same chord under it.
            0 => {
                if t.rem_euclid(24) == 0 {
                    let c = &mut self.composers;
                    let u = c.unit();
                    let step = if u < 0.25 * (1.0 - motion) { 0 } else if u < 0.55 { 1 } else if u < 0.85 { -1 } else if u < 0.93 { 2 } else { -2 };
                    c.de_top = (c.de_top + step).clamp(1, 9);
                    c.de_hist = (c.de_hist << 8) | ((c.de_top + 64) as u64 & 0xff);
                    let top = Self::mdeg(ctx, base, c.de_top, 0);
                    let lowest = top - shape[shape.len() - 1];
                    let len = 2.0 * beat * (0.9 + 0.4 * pedal);
                    for &s in shape {
                        self.note(ctx, at, lowest + s, vel * if s == shape[shape.len() - 1] { 1.0 } else { 0.7 }, len, out);
                    }
                }
                if t == 0 && bar.rem_euclid(2) == 0 && pedal > 0.2 {
                    self.note(ctx, at, low, vel * 0.6, 8.0 * beat * pedal, out);
                }
            }
            // Whole tone: up and down in sixths of a beat, an augmented chord below.
            1 => {
                if k.rem_euclid(2) == 0 {
                    let i = (k / 2).rem_euclid(22);
                    // Up eleven whole tones and down again.
                    let w = if i <= 11 { i } else { 22 - i };
                    let pitch = tonic + 2 * w as i32 - 6;
                    self.note(ctx, at, pitch, vel * (0.5 + 0.03 * w as f32), 2.0 * step_frames * (1.0 + 4.0 * pedal), out);
                }
                if t == 0 {
                    for s in [0, 4, 8] {
                        self.note(ctx, at, low + 12 + s, vel * 0.5, 4.0 * beat * (0.6 + 0.4 * pedal), out);
                    }
                }
            }
            // Arabesque: waves of triplets up the chord and back.
            2 => {
                const CHORDS: [i32; 4] = [0, 3, 1, 4];
                let ci = bar.rem_euclid(4) as usize;
                let amount = p.get(DE_CHORD_AMOUNT).copied().unwrap_or(1.0);
                let root = chord_root(amount, ci as i64, CHORDS[ci], CHORDS[(ci + 1) % 4], t >= 24);
                if t.rem_euclid(4) == 0 {
                    let i = t / 4;
                    const WAVE: [i32; 12] = [0, 1, 2, 3, 4, 5, 6, 5, 4, 3, 2, 1];
                    let tones = [0, 2, 4, 8];
                    let j = WAVE[i as usize % 12];
                    let d = root + tones[(j % 4) as usize] + 7 * (j / 4) - 7;
                    let pitch = Self::mdeg(ctx, base, d, 0);
                    self.note(ctx, at, pitch, vel * (0.6 + 0.06 * j as f32), 4.0 * step_frames * (1.0 + 2.0 * pedal), out);
                    self.composers.de_hist = (self.composers.de_hist << 8) | ((d + 64) as u64 & 0xff);
                }
                if t == 0 {
                    let pitch = Self::mdeg(ctx, base, root - 14, 0);
                    self.note(ctx, at, pitch, vel * 0.7, 4.0 * beat * (0.7 + 0.3 * pedal), out);
                }
            }
            // Pentatonic: a line in five notes, a fourth under it, an open fifth below.
            _ => {
                const PENTA: [i32; 5] = [0, 1, 2, 4, 5];
                let every = if motion > 0.5 { 6 } else { 12 };
                if t.rem_euclid(every) == 0 && self.composers.de_last != k {
                    let c = &mut self.composers;
                    c.de_last = k;
                    if c.unit() < 0.75 {
                        let u = c.unit();
                        c.de_top = (c.de_top + if u < 0.45 { 1 } else if u < 0.9 { -1 } else { 2 }).clamp(0, 9);
                        let i = c.de_top as usize;
                        let d = PENTA[i % 5] + 7 * (i / 5) as i32;
                        c.de_hist = (c.de_hist << 8) | ((d + 64) as u64 & 0xff);
                        let pitch = Self::mdeg(ctx, base, d, 0);
                        let len = every as f64 * step_frames * (0.9 + 0.5 * pedal);
                        self.note(ctx, at, pitch, vel, len, out);
                        self.note(ctx, at, pitch - 5, vel * 0.6, len, out);
                    }
                }
                if t == 0 {
                    for s in [0, 7] {
                        self.note(ctx, at, low + s, vel * 0.55, 4.0 * beat * (0.7 + 0.3 * pedal), out);
                    }
                }
            }
        }
        let c = &self.composers;
        self.visual[0] = (style as u64) | (choose(p, DE_SHAPE, DEBUSSY_SHAPES.len()) as u64) << 4 | (t as u64 & 0xff) << 8 | ((c.de_top + 64) as u64 & 0xff) << 16;
        self.visual[1] = c.de_hist;
        self.visual[2] = k as u64;
    }

    // ------------------------------------------------------------ feldman

    /// Feldman, step `k` (half its unit).
    pub(crate) fn feldman(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let density = p[FE_DENSITY].clamp(0.0, 1.0);
        let cluster = (p[FE_CLUSTER].round() as usize).clamp(1, 5);
        let color = p[FE_COLOR].clamp(0.0, 1.0);
        let repeat = p[FE_REPEAT].clamp(0.0, 1.0);
        let base = p[FE_BASE];
        let octaves = (p[FE_SPREAD].round() as i32).clamp(1, 3);
        let vel = p[FE_VELOCITY];
        let _ = k;
        let c = &mut self.composers;
        if c.fe_pos == 0 {
            if c.fe_left == 0 || !c.fe_ready {
                // A new pattern, sometimes keeping a chord of the last.
                let keep = c.fe_ready && c.unit() < 0.5 * repeat && c.fe_n > 0;
                let kept = c.fe_events[0];
                c.fe_ready = true;
                c.fe_len = 12 + (c.unit() * 24.0) as u8;
                c.fe_n = (1 + (density * 3.0 * c.unit() + density) as usize).clamp(1, MAX_EVENTS);
                for e in 0..c.fe_n {
                    let start = if e == 0 { 0 } else { (c.unit() * c.fe_len as f32 * 0.7) as u8 };
                    let len = 3 + (c.unit() * 9.0) as u8;
                    let n = (1 + (c.unit() * cluster as f32) as usize).min(cluster).max(1);
                    let mut notes = [0i8; 5];
                    for slot in notes.iter_mut().take(n) {
                        // Out of the key now and then (a semitone off a note of it).
                        let d = (c.unit() * (7 * octaves) as f32) as i32 - 3;
                        let chroma = if c.unit() < color * 0.6 { 1 } else { 0 };
                        *slot = (d * 2 + chroma).clamp(-120, 120) as i8;
                    }
                    c.fe_events[e] = Event { start, len, notes, n: n as u8 };
                }
                if keep {
                    c.fe_events[0] = kept;
                }
                c.fe_events[..c.fe_n].sort_by_key(|e| e.start);
                c.fe_left = 2 + (c.unit() * (1.0 + 4.0 * repeat)) as u32;
            } else {
                // Again, a little changed.
                c.fe_left -= 1;
                if c.unit() < 0.4 {
                    let e = (c.unit() * c.fe_n as f32) as usize % c.fe_n.max(1);
                    let ev = &mut c.fe_events[e];
                    match (c.rng >> 3) % 3 {
                        0 if ev.n > 0 => {
                            let j = (c.rng >> 7) as usize % ev.n as usize;
                            ev.notes[j] += if c.rng & 1 == 0 { 1 } else { -1 };
                        }
                        1 if ev.start > 0 => ev.start = (ev.start as i32 + if c.rng & 2 == 0 { 1 } else { -1 }).max(1) as u8,
                        _ => ev.len = (ev.len as i32 + if c.rng & 4 == 0 { 1 } else { -1 }).clamp(2, 14) as u8,
                    }
                }
            }
        }
        let pos = self.composers.fe_pos;
        let events = self.composers.fe_events;
        for ev in events[..self.composers.fe_n].iter().filter(|e| e.start == pos) {
            for &code in &ev.notes[..ev.n as usize] {
                // (each note: twice its degree, and one if it is a semitone up)
                let (d, chroma) = ((code as i32).div_euclid(2), (code as i32).rem_euclid(2));
                let pitch = Self::mdeg(ctx, base, d - 7 * (octaves - 1) / 2, chroma);
                let v = vel * (0.85 + 0.15 * self.composers.unit());
                self.note(ctx, at, pitch, v, ev.len as f64 * step_frames * 1.1, out);
            }
        }
        let c = &mut self.composers;
        c.fe_pos += 1;
        if c.fe_pos >= c.fe_len {
            c.fe_pos = 0;
        }
        let mut evs = 0u64;
        for (i, e) in c.fe_events[..c.fe_n].iter().enumerate() {
            evs |= ((e.start as u64) | (e.len as u64) << 8) << (16 * i);
        }
        self.visual[0] = (c.fe_len as u64) | (c.fe_pos as u64) << 8 | (c.fe_left as u64 & 0xff) << 16 | (c.fe_n as u64) << 24;
        self.visual[1] = evs;
    }
}

/// The motif of an invention: eight sixteenths, mostly steps, one leap.
pub fn invention_motif(var: u32) -> [i32; 8] {
    let mut m = [0i32; 8];
    for j in 1..8 {
        let u = hash(var * 13 + 5, j as u32);
        let step = if u < 0.35 { 1 } else if u < 0.6 { -1 } else if u < 0.75 { 2 } else if u < 0.88 { -2 } else { 3 };
        m[j] = (m[j - 1] + step).clamp(-3, 7);
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::{MidiFxKind, MidiFxState};
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};

    /// Runs `beats` beats at 120 BPM in C major: (on beat, pitch, off beat).
    fn notes(kind: MidiFxKind, p: &[f32], beats: f64) -> Vec<(f64, u8, f64)> {
        let spb = 24000.0;
        let mut st = MidiFxState::new(kind);
        let mut open: Vec<(f64, u8)> = Vec::new();
        let mut done = Vec::new();
        for b in 0..(beats * spb / 256.0) as u64 {
            let now = b * 256;
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / spb, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 2] };
            let mut out = Vec::new();
            st.process(kind, p, &[], &[], &mut out, &ctx);
            for e in &out {
                let t = (now + e.offset as u64) as f64 / spb;
                if e.on {
                    open.push((t, e.pitch));
                } else if let Some(i) = open.iter().position(|o| o.1 == e.pitch) {
                    let (on, pitch) = open.remove(i);
                    done.push((on, pitch, t));
                }
            }
        }
        done.extend(open.into_iter().map(|(on, pitch)| (on, pitch, f64::INFINITY)));
        done.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        done
    }

    fn in_key(n: &[(f64, u8, f64)]) -> bool {
        n.iter().all(|x| [0, 2, 4, 5, 7, 9, 11].contains(&(x.1 % 12)))
    }

    #[test]
    fn reich_phase_pulls_a_note_ahead_after_holding() {
        // 1/16 steps (a quarter beat), 2 repeats: held for 24 steps (6 beats),
        // drifting over the next 24 (6 beats).
        let mut p = defaults(REICH);
        p[RE_SHIFT] = 2.0;
        let n = notes(MidiFxKind::Reich, &p, 24.0);
        assert!(in_key(&n));
        let fig = reich_figure(0);
        // While held, the second player (an octave up) plays the same note on the same step.
        let a: Vec<&(f64, u8, f64)> = n.iter().filter(|x| x.0 < 5.9 && (x.1 as i32) < 72).collect();
        let b: Vec<&(f64, u8, f64)> = n.iter().filter(|x| x.0 < 5.9 && (x.1 as i32) >= 72).collect();
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(&b) {
            assert!((x.0 - y.0).abs() < 1e-3 && x.1 + 12 == y.1, "{x:?} {y:?}");
        }
        // After the drift: the second a note ahead, in step again.
        let later_a: Vec<u8> = n.iter().filter(|x| x.0 >= 12.0 && x.0 < 14.9 && (x.1 as i32) < 72).map(|x| x.1).collect();
        let later_b: Vec<u8> = n.iter().filter(|x| x.0 >= 12.0 && x.0 < 14.9 && (x.1 as i32) >= 72).map(|x| x.1).collect();
        assert!(later_a.len() >= 10 && later_b.len() >= 10);
        let _ = fig;
        assert_ne!(later_a.iter().map(|v| v % 12).collect::<Vec<_>>(), later_b.iter().map(|v| v % 12).collect::<Vec<_>>(), "B moved on");
    }

    #[test]
    fn reich_rhythm_shifts_a_step_against_itself() {
        let mut p = defaults(REICH);
        p[RE_STYLE] = 1.0;
        p[RE_SHIFT] = 2.0;
        let n = notes(MidiFxKind::Reich, &p, 12.0);
        let bits = reich_rhythm(0);
        assert_eq!(bits.count_ones(), 8);
        // First 24 steps (6 beats): both on the same steps.
        let at = |pitch: u8, lo: f64, hi: f64| n.iter().filter(|x| x.1 == pitch && x.0 >= lo && x.0 < hi).map(|x| (x.0 * 4.0).round() as i64).collect::<Vec<_>>();
        assert_eq!(at(60, 0.0, 5.99), at(67, 0.0, 5.99));
        // Then the fifth's rhythm is turned a step.
        assert_ne!(at(60, 6.0, 11.99), at(67, 6.0, 11.99));
    }

    #[test]
    fn reich_pulses_swell() {
        let mut p = defaults(REICH);
        p[RE_STYLE] = 2.0;
        let mut st = MidiFxState::new(MidiFxKind::Reich);
        let spb = 24000.0;
        let mut vels = Vec::new();
        for b in 0..(4.0 * spb / 256.0) as u64 {
            let now = b * 256;
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / spb, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 2] };
            let mut out = Vec::new();
            st.process(MidiFxKind::Reich, &p, &[], &[], &mut out, &ctx);
            vels.extend(out.iter().filter(|e| e.on && e.pitch >= 64 && e.pitch < 84).map(|e| e.vel));
        }
        let (first, mid) = (vels[0], vels[vels.len() / 2]);
        assert!(mid > 2.0 * first, "{first} {mid}");
    }

    #[test]
    fn bach_prelude_breaks_each_chord_twice_a_bar_its_voices_led() {
        let p = defaults(BACH);
        let n = notes(MidiFxKind::Bach, &p, 32.0);
        assert!(in_key(&n));
        // 16 sixteenths a bar, the bar's figure the same in both halves.
        let bar0: Vec<u8> = n.iter().filter(|x| x.0 < 3.99).map(|x| x.1).collect();
        assert_eq!(bar0.len(), 16);
        assert_eq!(&bar0[..8], &bar0[8..]);
        // The first chord is the tonic: C, E, G only.
        assert!(bar0.iter().all(|v| [0, 4, 7].contains(&(v % 12))), "{bar0:?}");
        // Voice leading: the top voice never leaps more than a fourth between bars.
        let tops: Vec<u8> = (0..8).map(|b| n.iter().filter(|x| x.0 >= b as f64 * 4.0 && x.0 < b as f64 * 4.0 + 1.99).map(|x| x.1).max().unwrap()).collect();
        assert!(tops.windows(2).all(|w| (w[0] as i32 - w[1] as i32).abs() <= 5), "{tops:?}");
        // The phrase cadences home: bar 8 on the tonic.
        let bar7: Vec<u8> = n.iter().filter(|x| x.0 >= 28.0 && x.0 < 31.99).map(|x| x.1 % 12).collect();
        assert!(bar7.iter().all(|v| [0, 4, 7].contains(v)), "{bar7:?}");
    }

    #[test]
    fn bach_invention_answers_the_motif_an_octave_down() {
        let mut p = defaults(BACH);
        p[BA_STYLE] = 1.0;
        let n = notes(MidiFxKind::Bach, &p, 8.0);
        // Two voices: at each step the lower one plays what the upper one
        // played half a bar (2 beats) before, an octave down.
        let mut at: std::collections::BTreeMap<i64, Vec<u8>> = Default::default();
        for x in &n {
            at.entry((x.0 * 4.0).round() as i64).or_default().push(x.1);
        }
        let mut checked = 0;
        for (&s, v) in at.range(8..32) {
            if let (2, Some(before)) = (v.len(), at.get(&(s - 8))) {
                let low = *v.iter().min().unwrap();
                let up = *before.iter().max().unwrap();
                assert_eq!(up, low + 12, "step {s}");
                checked += 1;
            }
        }
        assert!(checked >= 16, "{checked}");
    }

    #[test]
    fn bach_chorale_sings_four_voices_a_beat() {
        let mut p = defaults(BACH);
        p[BA_STYLE] = 2.0;
        let n = notes(MidiFxKind::Bach, &p, 8.0);
        for beat in 0..7 {
            let c = n.iter().filter(|x| (x.0 - beat as f64).abs() < 1e-3).count();
            assert_eq!(c, 4, "beat {beat}");
        }
        assert!(in_key(&n));
    }

    #[test]
    fn nancarrow_voices_at_their_tempos_end_together() {
        let mut p = defaults(NANCARROW);
        p[NA_RATIOS] = 0.0; // 3:4
        p[NA_SPREAD] = 1.0;
        let n = notes(MidiFxKind::Nancarrow, &p, 64.0);
        assert!(in_key(&n));
        let melody = nancarrow_melody(0, 16);
        let total: u32 = melody.iter().map(|m| m.1).sum();
        // The slow voice (half a beat a unit) and the fast one (3/4 of that):
        // both last notes start at the same time less their own last length.
        let slow_dur = total as f64 * 0.5;
        let fast_dur = total as f64 * 0.375;
        let last = melody.last().unwrap().1 as f64;
        let slow_last = slow_dur - last * 0.5;
        let fast_last = slow_dur - last * 0.375;
        let near = |t: f64| n.iter().any(|x| (x.0 - t).abs() < 0.02);
        assert!(near(slow_last) && near(fast_last), "{slow_last} {fast_last}");
        // The fast one starts later, by the difference.
        assert!(near(slow_dur - fast_dur));
        assert!(n.iter().any(|x| x.0 < 0.01));
    }

    #[test]
    fn nancarrow_shows_each_voice_on_its_way() {
        let p = defaults(NANCARROW);
        let mut st = MidiFxState::new(MidiFxKind::Nancarrow);
        let spb = 24000.0;
        for b in 0..(12.0 * spb / 256.0) as u64 {
            let now = b * 256;
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / spb, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 2] };
            let mut out = Vec::new();
            st.process(MidiFxKind::Nancarrow, &p, &[], &[], &mut out, &ctx);
        }
        let v = st.visual().unwrap();
        let slow = (v[1] & 0xff) as f32 / 255.0;
        assert!(slow > 0.3 && slow < 1.0, "{v:x?}");
    }

    #[test]
    fn debussy_planes_the_same_chord_shape() {
        let p = defaults(DEBUSSY);
        let n = notes(MidiFxKind::Debussy, &p, 16.0);
        // Each chord: the ninth shape, semitone for semitone.
        for beat in [0.0, 2.0, 4.0, 6.0] {
            let mut c: Vec<i32> = n.iter().filter(|x| (x.0 - beat).abs() < 1e-3 && x.1 >= 48).map(|x| x.1 as i32).collect();
            c.sort();
            c.dedup();
            let c: Vec<i32> = c.iter().map(|v| v - c[0]).collect();
            assert_eq!(c, vec![0, 4, 7, 10, 14], "beat {beat}");
        }
    }

    #[test]
    fn debussy_whole_tone_washes_step_by_whole_tones() {
        let mut p = defaults(DEBUSSY);
        p[DE_STYLE] = 1.0;
        let n = notes(MidiFxKind::Debussy, &p, 4.0);
        let run: Vec<i32> = n.iter().filter(|x| x.1 >= 58).map(|x| x.1 as i32).take(10).collect();
        assert!(run.windows(2).all(|w| w[1] - w[0] == 2), "{run:?}");
    }

    #[test]
    fn feldman_is_sparse_soft_and_comes_back() {
        let p = defaults(FELDMAN);
        let n = notes(MidiFxKind::Feldman, &p, 64.0);
        assert!(!n.is_empty());
        // Few notes for so long, with silences between.
        assert!(n.len() < 120, "{}", n.len());
        let starts: std::collections::BTreeSet<i64> = n.iter().map(|x| (x.0 * 100.0).round() as i64).collect();
        assert!(starts.len() < 64);
        // Something comes back: a chord's notes heard again later.
        let first: Vec<u8> = n.iter().filter(|x| x.0 < 0.01).map(|x| x.1).collect();
        assert!(n.iter().filter(|x| x.0 > 4.0).any(|x| first.contains(&x.1)), "{first:?}");
    }

    #[test]
    fn bach_at_no_chords_stays_on_the_tonic_and_with_more_passes() {
        let mut p = defaults(BACH);
        p[BA_CHORD_AMOUNT] = 0.0;
        let n = notes(MidiFxKind::Bach, &p, 32.0);
        assert!(n.iter().all(|x| [0, 4, 7].contains(&(x.1 % 12))), "home all along");
        // More: some half bars break another chord than their bar's first half.
        p[BA_CHORD_AMOUNT] = 2.0;
        let n = notes(MidiFxKind::Bach, &p, 64.0);
        let differs = (0..16).filter(|&b| {
            let set = |lo: f64, hi: f64| n.iter().filter(|x| x.0 >= lo - 1e-3 && x.0 < hi - 1e-3).map(|x| x.1 % 12).collect::<std::collections::BTreeSet<u8>>();
            set(b as f64 * 4.0, b as f64 * 4.0 + 2.0) != set(b as f64 * 4.0 + 2.0, b as f64 * 4.0 + 4.0)
        });
        assert!(differs.count() >= 3);
    }
}
