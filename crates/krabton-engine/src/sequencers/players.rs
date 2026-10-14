//! Players: musicians who play a chord progression, the same chords as
//! Progression and Guitar Chords (with Band on, all of them play the same
//! chords: change them on one and the others follow).
//!
//! - Piano Player: a left hand on the root (and fifth, octave) and a right
//!   hand that voices each chord close to the last one, as a pianist moves
//!   least: Ballad, Stride, Alberti, Arpeggio, Gospel, Bossa, Block chords,
//!   Pop Eighths.
//! - Bass Player: roots, root and fifth, octaves, a walking line that steps
//!   through the chord and leans into the next one by a semitone, reggae,
//!   funk, Motown, driving eighths.
//! - Banjo Roll: the five strings of a banjo in open tuning (moved to the
//!   key), each fretted to a tone of the chord, picked in rolls (forward,
//!   backward, alternating thumb, mixed, foggy) with the high drone string.
//! - String Section: three to five voices that move as little as they can
//!   from chord to chord, held, pulsing, in tremolo, pizzicato, stabs or an
//!   arpeggio.

use super::SeqState;
use crate::classic_seq::{chord_pitches, prog_slot, CHORD_LENGTHS, PROG_BYTES, PROG_SLOTS};
use crate::midi_fx::{MidiCtx, MidiEvent, MidiFxKind};
use crate::params::{ParamSpec, Unit};
use crate::presets::{pd, prog, Preset, V::C, V::N};

pub const LENGTH: usize = 0;
pub const CHORDS: usize = 1;
pub const STYLE: usize = 2;
pub const BASE: usize = 3;
pub const SWING: usize = 4;
pub const GATE: usize = 5;
pub const VELOCITY: usize = 6;
pub const HUMANIZE: usize = 7;
/// String Section's voices (after Humanize).
pub const VOICES: usize = 8;

pub const PIANO_STYLES: &[&str] = &["Ballad", "Stride", "Alberti", "Arpeggio", "Gospel", "Bossa", "Block", "Pop Eighths"];
pub const BASS_STYLES: &[&str] = &["Roots", "Root and Fifth", "Octaves", "Walking", "Reggae", "Funk", "Motown", "Eighths"];
pub const ROLLS: &[&str] = &["Forward", "Backward", "Alternating Thumb", "Mixed", "Foggy"];
pub const SECTION_STYLES: &[&str] = &["Sustain", "Pulse", "Tremolo", "Pizzicato", "Stabs", "Arpeggio"];

macro_rules! player_specs {
    ($styles:expr, $style:expr, $base_lo:expr, $base_hi:expr, $base:expr, $gate:expr) => {
        &[
            ParamSpec::choice("Length", "Chords", CHORD_LENGTHS, 2),
            ParamSpec::int("Chords", "Chords", 1.0, 8.0, 4.0, Unit::None),
            ParamSpec::choice("Style", "Player", $styles, $style),
            ParamSpec::int("Base Note", "Player", $base_lo, $base_hi, $base, Unit::None),
            ParamSpec::lin("Swing", "Player", 0.0, 1.0, 0.0, Unit::Percent),
            ParamSpec::lin("Gate", "Notes", 0.1, 1.0, $gate, Unit::Percent),
            ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
            ParamSpec::lin("Humanize", "Notes", 0.0, 1.0, 0.3, Unit::Percent),
            ParamSpec::toggle("Transpose", "Keys", false),
            ParamSpec::toggle("Hold", "Keys", false),
            ParamSpec::toggle("Band", "Keys", false),
        ]
    };
}

pub static PIANO: &[ParamSpec] = player_specs!(PIANO_STYLES, 0, 36.0, 72.0, 48.0, 0.9);
pub static BASS: &[ParamSpec] = player_specs!(BASS_STYLES, 0, 24.0, 52.0, 36.0, 0.8);
pub static BANJO: &[ParamSpec] = player_specs!(ROLLS, 0, 48.0, 72.0, 60.0, 1.0);
pub static SECTION: &[ParamSpec] = &[
    ParamSpec::choice("Length", "Chords", CHORD_LENGTHS, 2),
    ParamSpec::int("Chords", "Chords", 1.0, 8.0, 4.0, Unit::None),
    ParamSpec::choice("Style", "Player", SECTION_STYLES, 0),
    ParamSpec::int("Base Note", "Player", 36.0, 72.0, 53.0, Unit::None),
    ParamSpec::lin("Swing", "Player", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.95, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Humanize", "Notes", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::int("Voices", "Player", 3.0, 5.0, 4.0, Unit::None),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::toggle("Band", "Keys", false),
];

/// Whether a kind plays a chord progression (and can be in the band).
pub fn plays_chords(kind: MidiFxKind) -> bool {
    matches!(kind, MidiFxKind::Progression | MidiFxKind::GuitarChords | MidiFxKind::Ukulele | MidiFxKind::PianoPlayer | MidiFxKind::BassPlayer | MidiFxKind::Banjo | MidiFxKind::Section)
}

/// Where a progression-playing device keeps its Band switch.
pub fn band_param(kind: MidiFxKind) -> Option<usize> {
    plays_chords(kind).then(|| kind.specs().iter().position(|s| s.name == "Band")).flatten()
}

/// What the band shares: how long each chord lasts, how many there are,
/// and each chord (degree and quality; inversions and necks stay each
/// player's own).
pub fn band_part(params: &[f32], data: &[u8]) -> Vec<u8> {
    let mut part = vec![params.get(LENGTH).map(|v| v.round().max(0.0) as u8).unwrap_or(2), params.get(CHORDS).map(|v| v.round().max(1.0) as u8).unwrap_or(4)];
    for slot in 0..PROG_SLOTS {
        let (d, q, _) = prog_slot(data, slot);
        part.extend([d, q]);
    }
    part
}

/// Plays the band's part.
pub fn set_band_part(params: &mut [f32], data: &mut Vec<u8>, part: &[u8]) {
    if part.len() < 2 + PROG_SLOTS * 2 {
        return;
    }
    if let Some(v) = params.get_mut(LENGTH) {
        *v = part[0] as f32;
    }
    if let Some(v) = params.get_mut(CHORDS) {
        *v = part[1] as f32;
    }
    if data.len() < PROG_SLOTS * PROG_BYTES {
        data.resize(PROG_SLOTS * PROG_BYTES, 0);
    }
    for slot in 0..PROG_SLOTS {
        data[slot * PROG_BYTES] = part[2 + slot * 2];
        data[slot * PROG_BYTES + 1] = part[3 + slot * 2];
    }
}

pub fn player_default() -> Vec<u8> {
    prog(&[(1, 0, 0), (5, 0, 0), (6, 0, 0), (4, 0, 0)])
}

// ---------------------------------------------------------------- patterns

/// Who plays a step of the piano: the left hand (root, fifth, octave) or
/// the right (the chord, or one voice of it).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Hand {
    Root,
    Fifth,
    Octave,
    Chord,
    Voice(u8),
}

type Hit<T> = (u8, T, u8, f32);

fn piano_pattern(style: usize) -> &'static [Hit<Hand>] {
    use Hand::*;
    match style {
        // Ballad: the left hand holds, the chord on one and three.
        0 => &[(0, Root, 8, 1.0), (0, Chord, 8, 0.85), (8, Fifth, 8, 0.8), (8, Chord, 8, 0.75)],
        // Stride: bass on the beat, the chord after it.
        1 => &[(0, Root, 3, 1.0), (4, Chord, 2, 0.8), (8, Fifth, 3, 0.9), (12, Chord, 2, 0.8)],
        // Alberti: low, high, middle, high.
        2 => &[(0, Root, 16, 0.8), (0, Voice(0), 2, 0.85), (2, Voice(2), 2, 0.7), (4, Voice(1), 2, 0.75), (6, Voice(2), 2, 0.7), (8, Voice(0), 2, 0.85), (10, Voice(2), 2, 0.7), (12, Voice(1), 2, 0.75), (14, Voice(2), 2, 0.7)],
        // Arpeggio: up and down in eighths.
        3 => &[(0, Root, 16, 0.8), (0, Voice(0), 4, 0.8), (2, Voice(1), 4, 0.7), (4, Voice(2), 4, 0.75), (6, Voice(3), 4, 0.7), (8, Voice(4), 4, 0.8), (10, Voice(3), 4, 0.7), (12, Voice(2), 4, 0.75), (14, Voice(1), 4, 0.7)],
        // Gospel: octaves in the left hand, pushed chords in the right.
        4 => &[(0, Root, 4, 1.0), (0, Octave, 4, 0.9), (4, Chord, 2, 0.85), (7, Chord, 3, 0.95), (10, Root, 2, 0.8), (12, Chord, 2, 0.85), (14, Chord, 2, 0.9)],
        // Bossa: root and fifth in the bass, the chord off the beat.
        5 => &[(0, Root, 5, 0.9), (3, Chord, 2, 0.75), (6, Fifth, 2, 0.8), (6, Chord, 2, 0.7), (8, Root, 5, 0.85), (10, Chord, 2, 0.75), (13, Chord, 2, 0.7), (14, Fifth, 2, 0.8)],
        // Block chords on every beat.
        6 => &[(0, Root, 8, 0.9), (0, Chord, 4, 0.9), (4, Chord, 4, 0.75), (8, Root, 8, 0.85), (8, Chord, 4, 0.85), (12, Chord, 4, 0.75)],
        // Pop eighths: the chord on every eighth.
        _ => &[(0, Root, 8, 0.9), (0, Chord, 2, 0.9), (2, Chord, 2, 0.65), (4, Chord, 2, 0.8), (6, Chord, 2, 0.65), (8, Root, 8, 0.85), (8, Chord, 2, 0.85), (10, Chord, 2, 0.65), (12, Chord, 2, 0.8), (14, Chord, 2, 0.7)],
    }
}

/// What the bass plays on a step: root, the octave above, the fifth, the
/// third, or the approach (a semitone off the next chord's root).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Line {
    Root,
    Up,
    Fifth,
    Third,
    Approach,
}

fn bass_pattern(style: usize) -> &'static [Hit<Line>] {
    use Line::*;
    match style {
        0 => &[(0, Root, 4, 1.0), (4, Root, 4, 0.8), (8, Root, 4, 0.9), (12, Root, 4, 0.8)],
        1 => &[(0, Root, 7, 1.0), (8, Fifth, 7, 0.85)],
        2 => &[(0, Root, 2, 1.0), (2, Up, 2, 0.8), (4, Root, 2, 0.9), (6, Up, 2, 0.8), (8, Root, 2, 0.95), (10, Up, 2, 0.8), (12, Root, 2, 0.9), (14, Up, 2, 0.8)],
        3 => &[(0, Root, 4, 0.95), (4, Third, 4, 0.8), (8, Fifth, 4, 0.85), (12, Approach, 4, 0.8)],
        4 => &[(0, Root, 3, 1.0), (6, Fifth, 2, 0.8), (8, Root, 2, 0.9), (10, Up, 2, 0.75), (14, Fifth, 2, 0.8)],
        5 => &[(0, Root, 1, 1.0), (3, Root, 1, 0.7), (6, Up, 1, 0.9), (8, Root, 2, 0.95), (11, Fifth, 1, 0.75), (13, Up, 1, 0.85), (14, Root, 1, 0.7)],
        6 => &[(0, Root, 2, 1.0), (3, Third, 1, 0.75), (4, Fifth, 2, 0.85), (6, Up, 2, 0.8), (8, Fifth, 2, 0.85), (10, Third, 2, 0.75), (12, Root, 2, 0.8), (14, Approach, 2, 0.8)],
        _ => &[(0, Root, 2, 1.0), (2, Root, 2, 0.75), (4, Root, 2, 0.85), (6, Root, 2, 0.75), (8, Root, 2, 0.95), (10, Root, 2, 0.75), (12, Root, 2, 0.85), (14, Root, 2, 0.75)],
    }
}

/// Banjo rolls: the string picked on each sixteenth (1 the highest, 5 the drone).
fn roll(style: usize) -> [u8; 16] {
    match style {
        0 => [3, 2, 1, 5, 3, 2, 1, 5, 3, 2, 1, 5, 3, 2, 1, 5],
        1 => [1, 2, 3, 1, 2, 3, 1, 2, 1, 2, 3, 1, 2, 3, 1, 2],
        2 => [3, 1, 4, 1, 3, 1, 4, 1, 3, 1, 4, 1, 3, 1, 4, 1],
        3 => [3, 2, 1, 5, 1, 2, 3, 1, 3, 2, 1, 5, 1, 2, 3, 1],
        _ => [2, 1, 5, 1, 3, 1, 5, 1, 2, 1, 5, 1, 3, 1, 5, 1],
    }
}

/// When the strings play: (step, voice or -1 for all, length in steps, velocity).
fn section_pattern(style: usize) -> &'static [(u8, i8, u8, f32)] {
    match style {
        0 => &[(0, -1, 16, 0.8)],
        1 => &[(0, -1, 2, 0.9), (2, -1, 2, 0.7), (4, -1, 2, 0.8), (6, -1, 2, 0.7), (8, -1, 2, 0.85), (10, -1, 2, 0.7), (12, -1, 2, 0.8), (14, -1, 2, 0.7)],
        2 => &[
            (0, -1, 1, 0.8),
            (1, -1, 1, 0.6),
            (2, -1, 1, 0.75),
            (3, -1, 1, 0.6),
            (4, -1, 1, 0.8),
            (5, -1, 1, 0.6),
            (6, -1, 1, 0.75),
            (7, -1, 1, 0.6),
            (8, -1, 1, 0.8),
            (9, -1, 1, 0.6),
            (10, -1, 1, 0.75),
            (11, -1, 1, 0.6),
            (12, -1, 1, 0.8),
            (13, -1, 1, 0.6),
            (14, -1, 1, 0.75),
            (15, -1, 1, 0.6),
        ],
        3 => &[(0, -1, 1, 0.9), (4, -1, 1, 0.75), (8, -1, 1, 0.85), (12, -1, 1, 0.75)],
        4 => &[(0, -1, 2, 1.0), (3, -1, 1, 0.8), (6, -1, 2, 0.9), (10, -1, 1, 0.8), (12, -1, 3, 0.95)],
        _ => &[(0, 0, 4, 0.8), (2, 1, 4, 0.7), (4, 2, 4, 0.75), (6, 3, 4, 0.7), (8, 4, 4, 0.8), (10, 3, 4, 0.7), (12, 2, 4, 0.75), (14, 1, 4, 0.7)],
    }
}

/// The steps a style plays in a bar (for the editor): 0 none, 1 a note, 2 more.
pub fn style_steps(kind: MidiFxKind, style: usize) -> [u8; 16] {
    let mut s = [0u8; 16];
    let mut mark = |step: u8| {
        let c = &mut s[(step as usize) % 16];
        *c = (*c + 1).min(2);
    };
    match kind {
        MidiFxKind::PianoPlayer => piano_pattern(style).iter().for_each(|h| mark(h.0)),
        MidiFxKind::BassPlayer => bass_pattern(style).iter().for_each(|h| mark(h.0)),
        MidiFxKind::Banjo => (0..16u8).for_each(&mut mark),
        _ => section_pattern(style).iter().for_each(|h| mark(h.0)),
    }
    s
}

// ---------------------------------------------------------------- state

#[derive(Default)]
pub struct PlayersState {
    /// The voicing played last (the right hand, the section), low to high.
    voicing: Vec<i32>,
    /// The chord it was for.
    voiced: Option<(u8, u8, i32)>,
    /// The notes struck last (for the editor's keys), and on which step.
    struck: u128,
    struck_at: i64,
}

impl PlayersState {
    /// Tones (pitch classes) voiced near the last voicing: every inversion
    /// above `low`, the one that moves least (or sits nearest `center`).
    fn voice(&mut self, key: (u8, u8, i32), tones: &[i32], voices: usize, low: i32, center: i32) -> Vec<i32> {
        if self.voiced == Some(key) && self.voicing.len() == voices {
            return self.voicing.clone();
        }
        let n = tones.len().max(1);
        let mut best: Option<(i32, Vec<i32>)> = None;
        for rot in 0..n {
            for lift in [-12, 0, 12] {
                let mut v = Vec::with_capacity(voices);
                let mut at = low + lift;
                for i in 0..voices {
                    let pc = tones[(rot + i) % n].rem_euclid(12);
                    let mut pitch = at - at.rem_euclid(12) + pc;
                    if pitch < at {
                        pitch += 12;
                    }
                    v.push(pitch);
                    at = pitch + 1;
                }
                let cost = if self.voicing.len() == voices {
                    v.iter().zip(&self.voicing).map(|(a, b)| (a - b).abs()).sum::<i32>()
                } else {
                    (v.iter().sum::<i32>() / voices as i32 - center).abs() * voices as i32
                };
                // Keep it in its range.
                let range = (v[0] - low).abs() + (v[voices - 1] - (center + 12)).max(0) * 2;
                let cost = cost + range;
                if best.as_ref().is_none_or(|b| cost < b.0) {
                    best = Some((cost, v));
                }
            }
        }
        let v = best.map(|b| b.1).unwrap_or_default();
        self.voicing = v.clone();
        self.voiced = Some(key);
        v
    }
}

/// The chord on step `k` (sixteenths): (slot, degree, quality, inversion),
/// and whether the next step starts another chord.
fn chord_at(p: &[f32], data: &[u8], k: i64) -> (usize, u8, u8, u8, bool) {
    let beats = [1.0, 2.0, 4.0, 8.0][(p[LENGTH].round().max(0.0) as usize).min(3)];
    let steps = (beats * 4.0) as i64;
    let n = (p[CHORDS].round().clamp(1.0, PROG_SLOTS as f32)) as i64;
    let slot = (k.div_euclid(steps)).rem_euclid(n) as usize;
    let (d, q, inv) = prog_slot(data, slot);
    (slot, d, q, inv, (k + 1).rem_euclid(steps) == 0)
}

impl SeqState {
    #[allow(clippy::too_many_arguments)]
    fn player_note(&mut self, ctx: &MidiCtx, p: &[f32], at: u64, step_frames: f64, k: i64, pitch: i32, len_steps: f64, vel: f32, out: &mut Vec<MidiEvent>) {
        // Swing and a human hand can land the note in a later block.
        let hum = p[HUMANIZE].clamp(0.0, 1.0);
        let jitter = (self.rng.bipolar().abs() * hum * 0.006 * ctx.sr) as u64;
        let at = Self::swing_at(at, k, p[SWING], step_frames) + jitter;
        let v = (vel * p[VELOCITY] * (1.0 + self.rng.bipolar() * hum * 0.15)).clamp(0.05, 1.0);
        self.play(ctx, at, pitch, v, step_frames * len_steps * p[GATE].clamp(0.1, 1.0) as f64, out);
        let pl = &mut self.players;
        if pl.struck_at != k {
            pl.struck = 0;
            pl.struck_at = k;
        }
        if (0..128).contains(&pitch) {
            pl.struck |= 1u128 << pitch;
        }
        self.visual[1] = pl.struck as u64;
        self.visual[2] = (pl.struck >> 64) as u64;
    }

    /// Piano Player, Bass Player, Banjo Roll and String Section: step `k` (a
/// sixteenth). The visual: the slot, the step and the style, then the notes
/// struck last (128 bits).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn player(&mut self, kind: MidiFxKind, k: i64, p: &[f32], data: &[u8], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let (slot, degree, quality, inversion, _) = chord_at(p, data, k);
        let style = p[STYLE].round().max(0.0) as usize;
        let step = k.rem_euclid(16) as u8;
        self.visual[0] = slot as u64 | (step as u64) << 8 | (style as u64) << 16;
        if degree == 0 {
            return;
        }
        let base = p[BASE].round() as i32;
        let tones = chord_pitches(&ctx.scale, base, degree, quality, 0);
        let Some(&root) = tones.first() else { return };
        let pcs: Vec<i32> = tones.iter().map(|t| t.rem_euclid(12)).collect();
        // The root in the octave at the base (bass and left hand).
        let low_root = base + (root - base).rem_euclid(12) - if kind == MidiFxKind::PianoPlayer { 12 } else { 0 };
        match kind {
            MidiFxKind::PianoPlayer => {
                let voices = pcs.len().clamp(3, 4);
                let right = self.players.voice((degree, quality, inversion as i32), &pcs, voices, base + 4 + inversion as i32 * 2, base + 10);
                for &(s, hand, len, vel) in piano_pattern(style.min(PIANO_STYLES.len() - 1)) {
                    if s != step {
                        continue;
                    }
                    let len = len as f64;
                    match hand {
                        Hand::Root => self.player_note(ctx, p, at, step_frames, k, low_root, len, vel, out),
                        Hand::Fifth => self.player_note(ctx, p, at, step_frames, k, low_root + 7, len, vel, out),
                        Hand::Octave => self.player_note(ctx, p, at, step_frames, k, low_root + 12, len, vel, out),
                        Hand::Chord => {
                            for &n in &right {
                                self.player_note(ctx, p, at, step_frames, k, n, len, vel * 0.9, out);
                            }
                        }
                        Hand::Voice(v) => {
                            // Up the voicing and on into the next octave.
                            let i = v as usize;
                            let n = right[i % right.len()] + 12 * (i / right.len()) as i32;
                            self.player_note(ctx, p, at, step_frames, k, n, len, vel, out);
                        }
                    }
                }
            }
            MidiFxKind::BassPlayer => {
                // The root the line walks to: of the chord when this note is over.
                let target_root = |len: u8| -> Option<i32> {
                    let (target, ..) = chord_at(p, data, k + len as i64);
                    let (d, q, _) = prog_slot(data, target);
                    chord_pitches(&ctx.scale, base, d, q, 0).first().map(|r| base + (r - base).rem_euclid(12))
                };
                for &(s, line, len, vel) in bass_pattern(style.min(BASS_STYLES.len() - 1)) {
                    if s != step {
                        continue;
                    }
                    let third = pcs.get(1).map(|t| low_root + (t - low_root).rem_euclid(12)).unwrap_or(low_root + 4);
                    let pitch = match line {
                        Line::Root => low_root,
                        Line::Up => low_root + 12,
                        Line::Fifth => low_root + 7,
                        Line::Third => third,
                        // A semitone under (or over) where the line lands next.
                        Line::Approach => match target_root(len) {
                            Some(r) if r > low_root => r - 1,
                            Some(r) => r + 1,
                            None => low_root + 7,
                        },
                    };
                    self.player_note(ctx, p, at, step_frames, k, pitch, len as f64, vel, out);
                }
            }
            MidiFxKind::Banjo => {
                // Open G tuning moved to the key: drone, D, G, B, D.
                let shift = (ctx.scale.root as i32 - 7 + 6).rem_euclid(12) - 6;
                let open = [62, 59, 55, 50, 67].map(|n| n + shift + (base - 60));
                let string = roll(style.min(ROLLS.len() - 1))[step as usize % 16];
                let pitch = if string == 5 {
                    open[4]
                } else {
                    let o = open[string as usize - 1];
                    (0..5).map(|f| o + f).find(|n| pcs.contains(&n.rem_euclid(12))).unwrap_or(o)
                };
                let vel = if step % 4 == 0 { 0.95 } else { 0.75 };
                self.player_note(ctx, p, at, step_frames, k, pitch, 3.0, vel, out);
            }
            _ => {
                let voices = (p.get(VOICES).copied().unwrap_or(4.0).round() as usize).clamp(3, 5);
                let voicing = self.players.voice((degree, quality, inversion as i32), &pcs, voices, base, base + 7);
                let pattern = section_pattern(style.min(SECTION_STYLES.len() - 1));
                // Held chords last until the chord changes.
                let chord_steps = ([1.0, 2.0, 4.0, 8.0][(p[LENGTH].round().max(0.0) as usize).min(3)] * 4.0) as i64;
                let into = k.rem_euclid(chord_steps);
                for &(s, voice, len, vel) in pattern {
                    let held = style == 0;
                    let fires = if held { into == 0 } else { s == step };
                    if !fires {
                        continue;
                    }
                    let len = if held { chord_steps as f64 } else { len as f64 };
                    if voice < 0 {
                        for &n in &voicing {
                            self.player_note(ctx, p, at, step_frames, k, n, len, vel, out);
                        }
                    } else {
                        let n = voicing[voice as usize % voicing.len()];
                        self.player_note(ctx, p, at, step_frames, k, n, len, vel, out);
                    }
                    if held {
                        break;
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------- presets

const POP: &[(u8, u8, u8)] = &[(1, 0, 0), (5, 0, 0), (6, 0, 0), (4, 0, 0)];
const JAZZ: &[(u8, u8, u8)] = &[(2, 1, 0), (5, 1, 0), (1, 1, 0), (6, 1, 0)];
const SAD: &[(u8, u8, u8)] = &[(6, 0, 0), (4, 0, 0), (1, 0, 0), (5, 0, 0)];
const GOSPEL: &[(u8, u8, u8)] = &[(1, 1, 0), (4, 1, 0), (1, 1, 0), (5, 1, 0)];
const BOSSA: &[(u8, u8, u8)] = &[(2, 1, 0), (5, 1, 0), (1, 1, 0), (1, 1, 0)];
const FOLK: &[(u8, u8, u8)] = &[(1, 0, 0), (4, 0, 0), (1, 0, 0), (5, 0, 0)];
const EPIC: &[(u8, u8, u8)] = &[(6, 5, 0), (4, 5, 0), (1, 5, 0), (5, 5, 0)];

static PIANO_PRESETS: &[Preset] = &[
    pd("Ballad", &[("Style", C("Ballad"))], || prog(SAD)),
    pd("Ragtime Stride", &[("Style", C("Stride")), ("Swing", N(0.3))], || prog(POP)),
    pd("Classical Alberti", &[("Style", C("Alberti")), ("Humanize", N(0.15))], || prog(FOLK)),
    pd("Gospel Church", &[("Style", C("Gospel")), ("Swing", N(0.4))], || prog(GOSPEL)),
    pd("Bossa Keys", &[("Style", C("Bossa"))], || prog(BOSSA)),
    pd("Jazz Block Chords", &[("Style", C("Block")), ("Swing", N(0.5))], || prog(JAZZ)),
    pd("Pop Piano", &[("Style", C("Pop Eighths"))], || prog(POP)),
    pd("Dreamy Arpeggios", &[("Style", C("Arpeggio")), ("Length", C("2 bars"))], || prog(EPIC)),
];

static BASS_PRESETS: &[Preset] = &[
    pd("Simple Roots", &[("Style", C("Roots"))], || prog(POP)),
    pd("Country Two-Beat", &[("Style", C("Root and Fifth"))], || prog(FOLK)),
    pd("Disco Octaves", &[("Style", C("Octaves"))], || prog(SAD)),
    pd("Jazz Walk", &[("Style", C("Walking")), ("Swing", N(0.5)), ("Length", C("1 bar"))], || prog(JAZZ)),
    pd("Reggae", &[("Style", C("Reggae"))], || prog(POP)),
    pd("Funk", &[("Style", C("Funk")), ("Gate", N(0.5))], || prog(&[(2, 1, 0), (5, 1, 0)])),
    pd("Motown", &[("Style", C("Motown"))], || prog(POP)),
    pd("Driving Eighths", &[("Style", C("Eighths")), ("Gate", N(0.6))], || prog(SAD)),
];

static BANJO_PRESETS: &[Preset] = &[
    pd("Forward Roll", &[("Style", C("Forward"))], || prog(FOLK)),
    pd("Backward Roll", &[("Style", C("Backward"))], || prog(POP)),
    pd("Thumb and Finger", &[("Style", C("Alternating Thumb"))], || prog(FOLK)),
    pd("Bluegrass Breakdown", &[("Style", C("Foggy")), ("Length", C("2 beats"))], || prog(&[(1, 0, 0), (1, 0, 0), (4, 0, 0), (1, 0, 0), (5, 0, 0), (1, 0, 0), (5, 0, 0), (1, 0, 0)])),
    pd("Mixed Roll", &[("Style", C("Mixed"))], || prog(SAD)),
];

static SECTION_PRESETS: &[Preset] = &[
    pd("Warm Pad", &[("Style", C("Sustain"))], || prog(EPIC)),
    pd("Driving Strings", &[("Style", C("Pulse")), ("Gate", N(0.6))], || prog(SAD)),
    pd("Tremolo Tension", &[("Style", C("Tremolo")), ("Voices", N(5.0))], || prog(&[(6, 0, 0), (6, 0, 0), (5, 0, 0), (5, 0, 0)])),
    pd("Pizzicato", &[("Style", C("Pizzicato")), ("Gate", N(0.2))], || prog(POP)),
    pd("Stabs", &[("Style", C("Stabs")), ("Gate", N(0.5))], || prog(&[(1, 0, 0), (4, 0, 0)])),
    pd("Harp-like Arpeggio", &[("Style", C("Arpeggio")), ("Base Note", N(60.0))], || prog(GOSPEL)),
];

pub fn presets(kind: MidiFxKind) -> &'static [Preset] {
    match kind {
        MidiFxKind::PianoPlayer => PIANO_PRESETS,
        MidiFxKind::BassPlayer => BASS_PRESETS,
        MidiFxKind::Banjo => BANJO_PRESETS,
        MidiFxKind::Section => SECTION_PRESETS,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_band_shares_chords_but_not_inversions() {
        use super::*;
        let mut a = crate::params::defaults(PIANO);
        let a_data = prog(&[(2, 1, 0), (5, 1, 0), (1, 1, 0)]);
        a[CHORDS] = 3.0;
        a[LENGTH] = 1.0;
        let mut b = crate::params::defaults(crate::classic_seq::PROGRESSION);
        let mut b_data = prog(&[(1, 0, 2), (4, 0, 1)]);
        set_band_part(&mut b, &mut b_data, &band_part(&a, &a_data));
        assert_eq!(band_part(&b, &b_data), band_part(&a, &a_data));
        assert_eq!((b[0], b[1]), (1.0, 3.0));
        // The second chord keeps its inversion.
        assert_eq!(prog_slot(&b_data, 1), (5, 1, 1));
        // A device without chords yet gets them.
        let mut c = crate::params::defaults(crate::sequencers::strum::SPECS);
        let mut c_data = Vec::new();
        set_band_part(&mut c, &mut c_data, &band_part(&a, &a_data));
        assert_eq!(band_part(&c, &c_data), band_part(&a, &a_data));
    }

    use super::*;
    use crate::scale::Scale;

    /// Runs a player for `bars` bars; returns (beat, pitch) of every note-on.
    fn run(kind: MidiFxKind, p: &[f32], data: &[u8], bars: f64) -> Vec<(f64, u8)> {
        let mut s = SeqState::new();
        let mut ons = Vec::new();
        let mut now = 0u64;
        while (now as f64) < bars * 4.0 * 24000.0 {
            let c = MidiCtx { now, len: 512, samples_per_beat: 24000.0, scale: Scale::default(), scale_on: true, beat: now as f64 / 24000.0, playing: true, sr: 48000.0, drum_notes: 0 };
            let mut out = Vec::new();
            s.process(kind, p, data, &mut out, &c);
            for e in out.iter().filter(|e| e.on) {
                ons.push(((now + e.offset as u64) as f64 / 24000.0, e.pitch));
            }
            now += 512;
        }
        ons
    }

    fn quiet(p: &mut [f32]) {
        p[HUMANIZE] = 0.0;
    }

    #[test]
    fn every_player_and_style_plays_in_key() {
        let scale = Scale::default();
        for (kind, styles) in [(MidiFxKind::PianoPlayer, PIANO_STYLES), (MidiFxKind::BassPlayer, BASS_STYLES), (MidiFxKind::Banjo, ROLLS), (MidiFxKind::Section, SECTION_STYLES)] {
            for s in 0..styles.len() {
                let mut p = crate::params::defaults(kind.specs());
                quiet(&mut p);
                p[STYLE] = s as f32;
                let ons = run(kind, &p, &player_default(), 4.0);
                assert!(ons.len() >= 4, "{kind:?} {} plays: {}", styles[s], ons.len());
                // Walking bass leans in by a semitone; the rest stays in the key.
                if !(kind == MidiFxKind::BassPlayer && (styles[s] == "Walking" || styles[s] == "Motown")) && kind != MidiFxKind::Banjo {
                    assert!(ons.iter().all(|o| scale.contains(o.1)), "{kind:?} {}: {:?}", styles[s], ons.iter().filter(|o| !scale.contains(o.1)).collect::<Vec<_>>());
                }
            }
        }
    }

    #[test]
    fn the_bass_plays_each_chords_root_on_the_one() {
        let mut p = crate::params::defaults(BASS);
        quiet(&mut p);
        let ons = run(MidiFxKind::BassPlayer, &p, &player_default(), 4.0);
        // I V vi IV in C, a bar each: C, G, A, F from C2 (36) up.
        for (bar, want) in [36u8, 43, 45, 41].iter().enumerate() {
            let first = ons.iter().find(|o| (o.0 - bar as f64 * 4.0).abs() < 0.02).map(|o| o.1);
            assert_eq!(first, Some(*want), "bar {bar}");
        }
    }

    #[test]
    fn walking_leans_into_the_next_chord() {
        let mut p = crate::params::defaults(BASS);
        quiet(&mut p);
        p[STYLE] = 3.0;
        let ons = run(MidiFxKind::BassPlayer, &p, &player_default(), 2.0);
        // The last beat of the first bar (C) is a semitone from G (43): F# (42).
        let last = ons.iter().find(|o| (o.0 - 3.0).abs() < 0.02).map(|o| o.1);
        assert_eq!(last, Some(42));
    }

    #[test]
    fn the_right_hand_moves_little() {
        let mut p = crate::params::defaults(PIANO);
        quiet(&mut p);
        p[STYLE] = 6.0;
        let ons = run(MidiFxKind::PianoPlayer, &p, &player_default(), 4.0);
        // The chords on each downbeat, without the left hand.
        let chord = |bar: usize| -> Vec<u8> {
            let mut v: Vec<u8> = ons.iter().filter(|o| (o.0 - bar as f64 * 4.0).abs() < 0.02 && o.1 >= 48).map(|o| o.1).collect();
            v.sort();
            v
        };
        for bar in 0..3 {
            let (a, b) = (chord(bar), chord(bar + 1));
            assert_eq!(a.len(), b.len());
            let moved: i32 = a.iter().zip(&b).map(|(x, y)| (*x as i32 - *y as i32).abs()).sum();
            assert!(moved <= 8, "bar {bar} to {}: {a:?} -> {b:?}", bar + 1);
        }
    }
}
