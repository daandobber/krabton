//! Sequencers after composers who made music out of repetition. All of
//! them play in the song's key.
//!
//! - Glass, after Philip Glass: a broken chord repeated and repeated over a
//!   slow round of chords. Three against two (a triplet figure above a
//!   rocking one), rocking figures, a chord broken up and down, a figure
//!   that grows note by note and shrinks again (an additive process), or
//!   pulsing block chords; the bass held under each chord.
//! - Satie, after Erik Satie: a Gymnopédie (a slow waltz: the bass on one, a
//!   seventh chord on two, swaying between two chords, a sparse melody
//!   falling above it in long notes), a Gnossienne (oom-pah in a slow four,
//!   a melody turning around its notes with grace notes, the seventh raised
//!   in minor) or a Sarabande (ninth chords sliding in parallel).
//! - Riley, after Terry Riley's In C: players each repeat a short figure as
//!   long as they like before going on to the next, never more than three
//!   apart, sometimes resting, over a steady pulse on the tonic, through a
//!   series of figures whose harmony slowly turns (made here, not his).
//!   Or a Rainbow: modal cells of different lengths looping against each
//!   other and slowly changing, as with his organ and tape delays.

use super::{SeqState, RATES};
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

pub(crate) const DYNAMICS: [ParamSpec; 4] = [
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

// ---------------------------------------------------------------- glass

pub const G_FIGURE: usize = 1;
pub const G_CHORDS: usize = 2;
pub const G_REPEATS: usize = 3;
pub const G_BASS: usize = 4;
pub const G_SEVENTHS: usize = 5;
pub const G_BASE: usize = 6;
pub const G_OCTAVES: usize = 7;
pub const G_GATE: usize = 8;
pub const G_VELOCITY: usize = 9;
pub const G_ACCENT: usize = 10;

pub const FIGURES: &[&str] = &["Three Over Two", "Rocking", "Broken", "Additive", "Pulse"];
pub const GLASS_CHORDS: &[&str] = &["1 6 3 7", "1 7 6 5", "1 6", "1 4 1 5", "1 5 6 4", "Circle"];
/// The chords (scale degrees, 0 the tonic) of each.
pub const GLASS_ROOTS: [&[i32]; 6] = [&[0, 5, 2, 6], &[0, 6, 5, 4], &[0, 5], &[0, 3, 0, 4], &[0, 4, 5, 3], &[0, 3, 6, 2, 5, 1, 4, 0]];
/// The additive process: the figure's length, each played twice.
const ADDITIVE: [u32; 8] = [2, 3, 4, 5, 6, 5, 4, 3];

pub static GLASS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 4),
    ParamSpec::choice("Figure", "Glass", FIGURES, 0),
    ParamSpec::choice("Chords", "Glass", GLASS_CHORDS, 0),
    ParamSpec::int("Repeats", "Glass", 1.0, 16.0, 4.0, Unit::None),
    ParamSpec::toggle("Bass", "Glass", true),
    ParamSpec::toggle("Sevenths", "Glass", false),
    ParamSpec::int("Base Note", "Notes", 36.0, 84.0, 60.0, Unit::None),
    ParamSpec::int("Octaves", "Notes", 1.0, 3.0, 2.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.85, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Accent", "Notes", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

// ---------------------------------------------------------------- satie

pub const S_STYLE: usize = 1;
pub const S_HARMONY: usize = 2;
pub const S_MELODY: usize = 3;
pub const S_WANDER: usize = 4;
pub const S_BASE: usize = 5;
pub const S_RUBATO: usize = 6;
pub const S_PEDAL: usize = 7;
pub const S_VELOCITY: usize = 8;

pub const SATIE_STYLES: &[&str] = &["Gymnopédie", "Gnossienne", "Sarabande"];
pub const SATIE_HARMONIES: &[&str] = &["Sway", "Circle", "Minor", "Lydian"];
/// The chords (scale degrees) of each harmony, a bar each (two in a Gnossienne).
pub const SATIE_ROOTS: [&[i32]; 4] = [&[3, 0], &[0, 5, 1, 4], &[5, 1], &[0, 1]];

pub static SATIE: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 5),
    ParamSpec::choice("Style", "Satie", SATIE_STYLES, 0),
    ParamSpec::choice("Harmony", "Satie", SATIE_HARMONIES, 0),
    ParamSpec::lin("Melody", "Satie", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Wander", "Satie", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::int("Base Note", "Notes", 48.0, 84.0, 69.0, Unit::None),
    ParamSpec::lin("Rubato", "Notes", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Pedal", "Notes", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.55, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

// ---------------------------------------------------------------- riley

pub const R_STYLE: usize = 1;
pub const R_PLAYERS: usize = 2;
pub const R_PULSE: usize = 3;
pub const R_PATIENCE: usize = 4;
pub const R_FIGURES: usize = 5;
pub const R_BASE: usize = 6;
pub const R_SPREAD: usize = 7;
pub const R_GATE: usize = 8;
pub const R_VELOCITY: usize = 9;

pub const RILEY_STYLES: &[&str] = &["In C", "Rainbow"];
pub const MAX_PLAYERS: usize = 5;
/// Rainbow's loops: their lengths in steps.
pub const RAINBOW_LENGTHS: [usize; MAX_PLAYERS] = [5, 7, 9, 11, 13];

pub static RILEY: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 4),
    ParamSpec::choice("Style", "Riley", RILEY_STYLES, 0),
    ParamSpec::int("Players", "Riley", 1.0, 5.0, 3.0, Unit::None),
    ParamSpec::toggle("Pulse", "Riley", true),
    ParamSpec::lin("Patience", "Riley", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Figures", "Riley", 8.0, 48.0, 24.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 36.0, 84.0, 60.0, Unit::None),
    ParamSpec::lin("Spread", "Notes", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.65, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
];

/// How finely each counts its steps: Glass in sixths of its pulse (three
/// against two), Satie in twelfths of its beat (ornaments), Riley in halves
/// of its pulse (grace notes and runs).
pub const GLASS_TICKS: f64 = 6.0;
pub const SATIE_TICKS: f64 = 12.0;
pub const RILEY_TICKS: f64 = 2.0;

// ---------------------------------------------------------------- state

/// One of Riley's figures: notes (start, length in steps, scale degree,
/// semitones off the scale) and how long it is.
#[derive(Clone, Copy)]
struct Figure {
    notes: [(u8, u8, i8, i8); 8],
    n: usize,
    len: u8,
}

pub struct Minimal {
    rng: u32,
    // Glass
    g_chord: usize,
    g_rep: u32,
    g_note: u32,
    g_add: usize,
    g_started: bool,
    // Satie
    s_m: i32,
    s_phrase: i64,
    s_root: i32,
    s_bar: i64,
    s_pattern: u8,
    s_recent: u64,
    // Riley
    r_ready: bool,
    r_fig: [usize; MAX_PLAYERS],
    r_pos: [u8; MAX_PLAYERS],
    r_left: [u32; MAX_PLAYERS],
    r_rest: [bool; MAX_PLAYERS],
    r_cells: [[i8; 13]; MAX_PLAYERS],
    r_cycles: [u32; MAX_PLAYERS],
}

impl Default for Minimal {
    fn default() -> Self {
        Self {
            rng: 0x6_1A55,
            g_chord: 0,
            g_rep: 0,
            g_note: 0,
            g_add: 0,
            g_started: false,
            s_m: 4,
            s_phrase: -1,
            s_root: 0,
            s_bar: i64::MIN,
            s_pattern: 0,
            s_recent: 0,
            r_ready: false,
            r_fig: [0; MAX_PLAYERS],
            r_pos: [0; MAX_PLAYERS],
            r_left: [0; MAX_PLAYERS],
            r_rest: [false; MAX_PLAYERS],
            r_cells: [[0; 13]; MAX_PLAYERS],
            r_cycles: [0; MAX_PLAYERS],
        }
    }
}

impl Minimal {
    fn unit(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / 16_777_216.0
    }
}

/// A steady pseudo-random number for `a` and `b` (0..1).
pub(crate) fn hash(a: u32, b: u32) -> f32 {
    let mut h = a.wrapping_mul(0x9E37_79B9) ^ b.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h >> 8) as f32 / 16_777_216.0
}

pub(crate) fn choose(p: &[f32], i: usize, n: usize) -> usize {
    (p[i].round().max(0.0) as usize).min(n - 1)
}

/// Riley's figure `i` of `total`: its harmony turns through the piece, from
/// the tonic triad, through a sharpened fourth and a flattened seventh,
/// back home.
fn figure(i: usize, total: usize) -> Figure {
    let f = i as f32 / total.max(1) as f32;
    // The notes of this part of the piece (degree, semitones off the scale).
    let field: &[(i8, i8)] = if f < 0.3 {
        &[(0, 0), (2, 0), (4, 0), (1, 0), (2, 0), (4, 0)]
    } else if f < 0.55 {
        &[(3, 1), (4, 0), (2, 0), (0, 0), (3, 1), (5, 0)]
    } else if f < 0.8 {
        &[(6, -1), (4, 0), (2, 0), (1, 0), (6, -1), (5, 0)]
    } else {
        &[(0, 0), (4, 0), (2, 0), (7, 0), (0, 0), (4, 0)]
    };
    let pick = |j: u32| field[(hash(i as u32 * 7 + 1, j) * field.len() as f32) as usize % field.len()];
    let (a, b, c) = (pick(1), pick(2), pick(3));
    let mut fig = Figure { notes: [(0, 0, 0, 0); 8], n: 0, len: 8 };
    let mut push = |start: u8, len: u8, (d, ch): (i8, i8)| {
        if fig.n < 8 {
            fig.notes[fig.n] = (start, len, d, ch);
            fig.n += 1;
        }
    };
    // The last figure: the tonic's octaves.
    if i + 1 == total {
        push(0, 2, (0, 0));
        push(2, 2, (7, 0));
        push(4, 2, (0, 0));
        push(6, 2, (7, 0));
        return fig;
    }
    match (hash(i as u32, 99) * 8.0) as u32 {
        // A grace note into a long one.
        0 => {
            push(0, 1, a);
            push(1, 7, b);
        }
        // Rocking between two.
        1 => {
            push(0, 2, a);
            push(2, 2, b);
            push(4, 2, a);
            push(6, 2, b);
        }
        // A long tone.
        2 => {
            push(0, 16, a);
            fig.len = 16;
        }
        // A little run up and back.
        3 => {
            let s = a.0;
            for (j, d) in [0i8, 1, 2, 1, 0].iter().enumerate() {
                push(j as u8, 1, (s + d, a.1));
            }
        }
        // Off the beat.
        4 => {
            push(2, 2, a);
            push(6, 2, b);
        }
        // A call: two short, one long.
        5 => {
            push(0, 2, a);
            push(2, 2, b);
            push(4, 4, c);
        }
        // A rest, then a grace note into a note.
        6 => {
            push(4, 1, a);
            push(5, 3, b);
        }
        // An octave leap.
        _ => {
            push(0, 4, a);
            push(4, 4, (a.0 + 7, a.1));
        }
    }
    fig
}

impl SeqState {
    /// Degree `d` of the song's key (0 its tonic) in the octave of `base`
    /// (the tonic at or below it), plus `chroma` semitones.
    pub(crate) fn mdeg(ctx: &MidiCtx, base: f32, d: i32, chroma: i32) -> i32 {
        let n = (ctx.scale.len() as i32).max(1);
        let (b, _) = ctx.scale.locate(base.round() as i32);
        ctx.scale.pitch_at(b - b.rem_euclid(n) + d) + chroma
    }

    /// Plays a note now, or later in a block to come.
    pub(crate) fn mlater(&mut self, ctx: &MidiCtx, at: u64, pitch: i32, vel: f32, len: f64, out: &mut Vec<MidiEvent>) {
        if at < ctx.now + ctx.len as u64 {
            self.note(ctx, at.max(ctx.now), pitch, vel, len, out);
        } else if self.pending.len() < self.pending.capacity() {
            self.pending.push((at, pitch.clamp(0, 127) as u8, vel, len.max(1.0) as u64));
        }
    }

    // ------------------------------------------------------------ glass

    /// Glass, step `k` (a sixth of its pulse).
    pub(crate) fn glass(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let figure = choose(p, G_FIGURE, FIGURES.len());
        let roots = GLASS_ROOTS[choose(p, G_CHORDS, GLASS_CHORDS.len())];
        let repeats = (p[G_REPEATS].round() as u32).clamp(1, 16);
        let base = p[G_BASE];
        let octaves = (p[G_OCTAVES].round() as i32).clamp(1, 3);
        let gate = p[G_GATE].clamp(0.1, 1.0) as f64;
        let vel = p[G_VELOCITY];
        let accent = p[G_ACCENT].clamp(0.0, 1.0);
        let pulse = 6.0 * step_frames;
        let m = &mut self.minimal;
        if !m.g_started {
            m.g_started = true;
            m.g_chord = 0;
            m.g_rep = 0;
            m.g_note = 0;
            m.g_add = 0;
        }
        m.g_chord %= roots.len();
        let root = roots[m.g_chord];
        let tones: &[i32] = if p[G_SEVENTHS] >= 0.5 { &[0, 2, 4, 6] } else { &[0, 2, 4] };
        // The chord's notes up the octaves, and the top root.
        let mut arp = [0i32; 16];
        let mut n = 0;
        for o in 0..octaves {
            for &t in tones {
                arp[n] = root + t + 7 * o;
                n += 1;
            }
        }
        arp[n] = root + 7 * octaves;
        n += 1;
        let fig_len = match figure {
            2 => 6,
            3 => ADDITIVE[m.g_add % ADDITIVE.len()],
            _ => 4,
        };
        let step = k.rem_euclid(6);
        // Three over two: the triplets above, every fourth step.
        if figure == 0 && k.rem_euclid(4) == 0 {
            let i = (k / 4).rem_euclid(3) as usize;
            let d = arp[(1 + i).min(n - 1)] + 7;
            let first = i == 0;
            let v = vel * if first { 1.0 + 0.2 * accent } else { 0.9 };
            let pitch = Self::mdeg(ctx, base, d, 0);
            self.note(ctx, at, pitch, v, 4.0 * step_frames * gate, out);
        }
        if step != 0 {
            return;
        }
        let m = &mut self.minimal;
        let i = m.g_note as usize;
        let first = i == 0;
        let chord_start = first && m.g_rep == 0 && (figure != 3 || m.g_add == 0);
        let v = vel * if first { 1.0 + 0.3 * accent } else { 1.0 - 0.1 * accent };
        // The bass, held under the whole chord.
        if chord_start && p[G_BASS] >= 0.5 {
            let span = if figure == 3 { ADDITIVE.iter().sum::<u32>() * 2 } else { fig_len * repeats };
            let pitch = Self::mdeg(ctx, base, root - 14, 0);
            self.note(ctx, at, pitch, vel * 0.9, span as f64 * pulse * 0.98, out);
        }
        match figure {
            // Rocking under the triplets: low, middle, high, middle.
            0 => {
                let d = arp[[0usize, 1, 2, 1][i % 4].min(n - 1)] - 7;
                let pitch = Self::mdeg(ctx, base, d, 0);
                self.note(ctx, at, pitch, v * 0.85, pulse * gate, out);
            }
            // Rocking: low, high, middle, high.
            1 => {
                let d = arp[[0usize, 2, 1, 2][i % 4].min(n - 1)];
                let pitch = Self::mdeg(ctx, base, d, 0);
                self.note(ctx, at, pitch, v, pulse * gate, out);
            }
            // Broken: up and down again.
            2 => {
                let d = arp[[0usize, 1, 2, 3, 2, 1][i % 6].min(n - 1)];
                let pitch = Self::mdeg(ctx, base, d, 0);
                self.note(ctx, at, pitch, v, pulse * gate, out);
            }
            // Additive: up through as many notes as the figure has now.
            3 => {
                let d = arp[i.min(n - 1)];
                let pitch = Self::mdeg(ctx, base, d, 0);
                self.note(ctx, at, pitch, v, pulse * gate, out);
            }
            // Pulse: the chord, struck on every pulse.
            _ => {
                for &t in tones {
                    let pitch = Self::mdeg(ctx, base, root + t, 0);
                    self.note(ctx, at, pitch, v * 0.8, pulse * gate * 0.9, out);
                }
            }
        }
        // On through the figure, the repeats and the chords.
        let m = &mut self.minimal;
        m.g_note += 1;
        if m.g_note >= fig_len {
            m.g_note = 0;
            m.g_rep += 1;
            if figure == 3 {
                if m.g_rep >= 2 {
                    m.g_rep = 0;
                    m.g_add += 1;
                    if m.g_add >= ADDITIVE.len() {
                        m.g_add = 0;
                        m.g_chord = (m.g_chord + 1) % roots.len();
                    }
                }
            } else if m.g_rep >= repeats {
                m.g_rep = 0;
                m.g_chord = (m.g_chord + 1) % roots.len();
            }
        }
        let shown_len = if figure == 3 { ADDITIVE[m.g_add % ADDITIVE.len()] } else { fig_len };
        self.visual[0] = (m.g_chord as u64 & 0xf)
            | (roots.len() as u64 & 0xf) << 4
            | (figure as u64 & 0xf) << 8
            | (m.g_rep as u64 & 0x1f) << 12
            | (repeats as u64 & 0x1f) << 17
            | (i as u64 & 0x1f) << 22
            | (shown_len as u64 & 0x1f) << 27
            | ((root + 64) as u64 & 0xff) << 32;
        self.visual[1] = k as u64;
    }

    // ------------------------------------------------------------ satie

    /// Satie, step `k` (a twelfth of its beat).
    pub(crate) fn satie(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let style = choose(p, S_STYLE, SATIE_STYLES.len());
        let roots = SATIE_ROOTS[choose(p, S_HARMONY, SATIE_HARMONIES.len())];
        let base = p[S_BASE];
        let melody = p[S_MELODY].clamp(0.0, 1.0);
        let wander = p[S_WANDER].clamp(0.0, 1.0);
        let rubato = p[S_RUBATO].clamp(0.0, 1.0);
        let pedal = p[S_PEDAL].clamp(0.0, 1.0) as f64;
        let vel = p[S_VELOCITY];
        let beat = 12.0 * step_frames;
        let beats_in_bar = if style == 1 { 4 } else { 3 };
        let bar_len = 12 * beats_in_bar as i64;
        let bar = k.div_euclid(bar_len);
        let t = k.rem_euclid(bar_len);
        // A Gnossienne's chords last two bars.
        let chord_i = if style == 1 { bar.div_euclid(2) } else { bar }.rem_euclid(roots.len() as i64) as usize;
        let minor = ctx.scale.intervals().get(2) == Some(&3);
        // The leading tone in minor (a Gnossienne's raised seventh).
        let lead = |d: i32| if style == 1 && minor && d.rem_euclid(7) == 6 { 1 } else { 0 };
        if t == 0 {
            let m = &mut self.minimal;
            if m.s_bar != bar {
                m.s_bar = bar;
                if style == 2 {
                    // A Sarabande's chords slide: a step up or down, home at times.
                    let u = m.unit();
                    m.s_root = if m.s_root != 0 && u < 0.3 { 0 } else if u < 0.65 { m.s_root + 1 } else { m.s_root - 1 }.clamp(-3, 4);
                } else {
                    m.s_root = roots[chord_i];
                }
            }
        }
        let root = self.minimal.s_root;
        let accomp = vel * 0.75;
        match style {
            // A Sarabande: ninth chords sliding, on one and on three.
            2 => {
                if t == 0 || t == 24 {
                    let r = if t == 24 { root + if self.minimal.unit() < 0.5 { 1 } else { -1 } } else { root };
                    let len = if t == 0 { 2.0 } else { 1.0 } * beat * (0.85 + 0.25 * pedal);
                    for d in [0, 2, 4, 6, 8] {
                        let pitch = Self::mdeg(ctx, base, r + d - 7, 0);
                        self.note(ctx, at, pitch, accomp * 0.85, len, out);
                    }
                    if t == 0 {
                        let pitch = Self::mdeg(ctx, base, r - 14, 0);
                        self.note(ctx, at, pitch, accomp, 3.0 * beat * (0.7 + 0.3 * pedal), out);
                    }
                    // The top line now and then an octave up, a melody of its own.
                    if self.minimal.unit() < melody * 0.5 {
                        let pitch = Self::mdeg(ctx, base, r + 8, 0);
                        self.note(ctx, at, pitch, vel, len, out);
                    }
                }
                self.satie_visual(style, t, chord_i, roots.len(), root, k);
                return;
            }
            // A Gnossienne: oom-pah in four.
            1 => {
                if t % 24 == 0 {
                    let pitch = Self::mdeg(ctx, base, root - 14, 0);
                    self.note(ctx, at, pitch, accomp, beat * (0.8 + 0.4 * pedal), out);
                } else if t % 24 == 12 {
                    for d in [2, 4, 7] {
                        let pitch = Self::mdeg(ctx, base, root + d - 7, lead(root + d));
                        self.note(ctx, at, pitch, accomp * 0.8, beat * (0.7 + 0.4 * pedal), out);
                    }
                }
            }
            // A Gymnopédie: the bass on one, the chord on two, held through three.
            _ => {
                if t == 0 {
                    let pitch = Self::mdeg(ctx, base, root - 14, 0);
                    self.note(ctx, at, pitch, accomp, beat * (1.8 + 1.1 * pedal), out);
                } else if t == 12 {
                    for d in [2, 4, 6] {
                        let pitch = Self::mdeg(ctx, base, root + d - 7, 0);
                        self.note(ctx, at, pitch, accomp * 0.75, beat * (1.6 + 0.35 * pedal), out);
                    }
                }
            }
        }
        // The melody: phrases of four bars after four bars of the waltz alone.
        if bar >= 4 && melody > 0.0 {
            let phrase = (bar - 4).div_euclid(4);
            let in_phrase = (bar - 4).rem_euclid(4);
            let m = &mut self.minimal;
            if m.s_phrase != phrase && t == 0 && in_phrase == 0 {
                m.s_phrase = phrase;
                // Some phrases rest; a new one begins up high and falls.
                if m.unit() > 0.35 + 0.65 * melody {
                    m.s_pattern = 0xff;
                } else {
                    m.s_pattern = 0;
                    m.s_m = 3 + (m.unit() * 4.0) as i32;
                }
            }
            if m.s_pattern != 0xff {
                // The rhythm of this bar, chosen on its first step.
                let last = in_phrase == 3;
                if t == 0 {
                    let u = m.unit();
                    m.s_pattern = if last {
                        0
                    } else if style == 1 {
                        5 + (u * 2.0) as u8
                    } else if u < 0.3 {
                        0
                    } else if u < 0.55 {
                        1
                    } else if u < 0.75 {
                        2
                    } else if u < 0.9 {
                        3
                    } else {
                        4
                    };
                }
                // (start, length) in steps.
                let notes: &[(i64, i64)] = match m.s_pattern {
                    0 => &[(0, 36)],
                    1 => &[(0, 12), (12, 24)],
                    2 => &[(0, 24), (24, 12)],
                    3 => &[(0, 12), (12, 12), (24, 12)],
                    4 => &[(12, 24)],
                    5 => &[(0, 12), (12, 12), (24, 24)],
                    _ => &[(0, 24), (24, 6), (30, 6), (36, 12)],
                };
                if let Some(&(_, len)) = notes.iter().find(|n| n.0 == t) {
                    let m = &mut self.minimal;
                    let u = m.unit();
                    // Mostly a step, falling more than rising; a leap now and then.
                    let step = if u < 0.12 * wander {
                        if m.unit() < 0.5 { 3 } else { -4 }
                    } else if u < 0.55 {
                        -1
                    } else if u < 0.8 {
                        1
                    } else if u < 0.9 {
                        -2
                    } else {
                        2
                    };
                    m.s_m = (m.s_m + step).clamp(-2, 9);
                    // A phrase ends on the chord.
                    if last {
                        let rel = (m.s_m - root).rem_euclid(7);
                        m.s_m += match rel {
                            1 => -1,
                            3 => 1,
                            5 => -1,
                            6 => 1,
                            _ => 0,
                        };
                    }
                    let d = m.s_m;
                    m.s_recent = (m.s_recent << 8) | ((d + 64) as u64 & 0xff);
                    let late = (m.unit() as f64 * rubato as f64 * 0.15 * beat) as u64;
                    let ornament = style == 1 && m.unit() < 0.3 && len >= 12;
                    let pitch = Self::mdeg(ctx, base, d, lead(d));
                    let length = len as f64 * step_frames * (0.9 + 0.3 * pedal);
                    if ornament {
                        // A turn: the note above, the note, the note below, then the note.
                        let up = Self::mdeg(ctx, base, d + 1, lead(d + 1));
                        let down = Self::mdeg(ctx, base, d - 1, lead(d - 1));
                        let s = 2.0 * step_frames;
                        self.mlater(ctx, at + late, up, vel * 0.8, s * 0.9, out);
                        self.mlater(ctx, at + late + s as u64, pitch, vel * 0.8, s * 0.9, out);
                        self.mlater(ctx, at + late + 2 * s as u64, down, vel * 0.8, s * 0.9, out);
                        self.mlater(ctx, at + late + 3 * s as u64, pitch, vel, length - 3.0 * s, out);
                    } else {
                        self.mlater(ctx, at + late, pitch, vel, length, out);
                    }
                }
            }
        }
        self.satie_visual(style, t, chord_i, roots.len(), root, k);
    }

    fn satie_visual(&mut self, style: usize, t: i64, chord_i: usize, chords: usize, root: i32, k: i64) {
        self.visual[0] = (style as u64 & 0xf) | (t as u64 & 0xff) << 4 | (chord_i as u64 & 0xf) << 12 | (chords as u64 & 0xf) << 16 | ((root + 64) as u64 & 0xff) << 20;
        self.visual[1] = self.minimal.s_recent;
        self.visual[2] = k as u64;
    }

    // ------------------------------------------------------------ riley

    /// Riley, step `k` (half its pulse).
    pub(crate) fn riley(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let style = choose(p, R_STYLE, RILEY_STYLES.len());
        let players = (p[R_PLAYERS].round() as usize).clamp(1, MAX_PLAYERS);
        let patience = p[R_PATIENCE].clamp(0.0, 1.0);
        let total = (p[R_FIGURES].round() as usize).clamp(8, 48);
        let base = p[R_BASE];
        let spread = p[R_SPREAD].clamp(0.0, 1.0);
        let gate = p[R_GATE].clamp(0.1, 1.0) as f64;
        let vel = p[R_VELOCITY];
        // The pulse: the tonic in two octaves above, every pulse.
        if p[R_PULSE] >= 0.5 && k.rem_euclid(2) == 0 {
            for d in [7, 14] {
                let pitch = Self::mdeg(ctx, base, d, 0);
                self.note(ctx, at, pitch, vel * 0.55, 2.0 * step_frames * 0.45, out);
            }
        }
        // Where each player sits: some an octave up or down.
        const OCT: [f32; MAX_PLAYERS] = [0.0, 1.0, -1.0, 2.0, 0.5];
        let octave = |i: usize| (spread * OCT[i]).round() as i32 * 7;
        if !self.minimal.r_ready {
            let m = &mut self.minimal;
            m.r_ready = true;
            for i in 0..MAX_PLAYERS {
                m.r_fig[i] = 0;
                m.r_pos[i] = 0;
                m.r_left[i] = 1 + i as u32 * 2;
                m.r_rest[i] = i > 0;
                m.r_cycles[i] = 0;
                // Rainbow's cells: a walk in the mode.
                let mut d = (i as i8 * 2) % 5;
                for j in 0..13 {
                    m.r_cells[i][j] = d;
                    let u = m.unit();
                    d = (d + if u < 0.4 { 1 } else if u < 0.75 { -1 } else if u < 0.9 { 2 } else { -2 }).clamp(-2, 9);
                }
            }
        }
        if style == 1 {
            // Rainbow: cells of 5, 7, 9, 11 and 13 steps against each other.
            for i in 0..players {
                let len = RAINBOW_LENGTHS[i];
                let pos = k.rem_euclid(len as i64) as usize;
                let m = &mut self.minimal;
                if pos == 0 {
                    m.r_cycles[i] += 1;
                    // Now and then a note of the cell moves a step.
                    if m.unit() < 0.15 + 0.5 * (1.0 - patience) {
                        let j = (m.unit() * len as f32) as usize % len;
                        let s = if m.unit() < 0.5 { 1 } else { -1 };
                        m.r_cells[i][j] = (m.r_cells[i][j] + s).clamp(-3, 10);
                    }
                }
                let d = m.r_cells[i][pos] as i32;
                // Some steps rest, so the cells breathe.
                if hash(i as u32 * 31 + pos as u32, m.r_cycles[i] / 4) < 0.12 {
                    continue;
                }
                let v = vel * if pos == 0 { 1.0 } else { 0.8 };
                let pitch = Self::mdeg(ctx, base, d + octave(i), 0);
                self.note(ctx, at, pitch, v, step_frames * gate, out);
            }
            let m = &self.minimal;
            let mut v1 = 0u64;
            for i in 0..MAX_PLAYERS {
                v1 |= (k.rem_euclid(RAINBOW_LENGTHS[i] as i64) as u64 & 0xf) << (4 * i);
            }
            self.visual[0] = (players as u64) | 1 << 8 | ((p[R_PULSE] >= 0.5) as u64) << 9 | (total as u64 & 0x3f) << 16;
            self.visual[1] = v1;
            self.visual[2] = k as u64;
            let _ = m;
            return;
        }
        // In C: each player on through the figures at their own pace.
        let slowest = (0..players).map(|i| self.minimal.r_fig[i]).min().unwrap_or(0);
        for i in 0..players {
            let m = &mut self.minimal;
            if m.r_fig[i] >= total {
                continue;
            }
            let fig = figure(m.r_fig[i], total);
            let pos = m.r_pos[i];
            if !m.r_rest[i] {
                for &(s, len, d, ch) in &fig.notes[..fig.n] {
                    if s == pos {
                        let pitch = Self::mdeg(ctx, base, d as i32 + octave(i), ch as i32);
                        let v = vel * if s == 0 { 1.0 } else { 0.85 };
                        self.note(ctx, at, pitch, v, len as f64 * step_frames * gate, out);
                    }
                }
            }
            let m = &mut self.minimal;
            m.r_pos[i] += 1;
            if m.r_pos[i] >= fig.len {
                m.r_pos[i] = 0;
                m.r_left[i] = m.r_left[i].saturating_sub(1);
                m.r_rest[i] = false;
                // Done with it: on to the next, but never more than three ahead.
                if m.r_left[i] == 0 && m.r_fig[i] < slowest + 3 {
                    m.r_fig[i] += 1;
                    m.r_left[i] = 2 + (m.unit() * (2.0 + 16.0 * patience)) as u32;
                    // Sometimes a player sits out a round.
                    m.r_rest[i] = m.unit() < 0.12;
                } else if m.r_left[i] == 0 {
                    m.r_left[i] = 1;
                }
            }
        }
        // All through: from the top again.
        let m = &mut self.minimal;
        if (0..players).all(|i| m.r_fig[i] >= total) {
            m.r_ready = false;
        }
        let mut v1 = 0u64;
        for i in 0..MAX_PLAYERS {
            let f = if i < players { m.r_fig[i].min(63) as u64 } else { 0xff };
            v1 |= (f & 0xff) << (8 * i);
        }
        let mut rest = 0u64;
        for i in 0..MAX_PLAYERS {
            rest |= (m.r_rest[i] as u64) << i;
        }
        self.visual[0] = (players as u64) | ((p[R_PULSE] >= 0.5) as u64) << 9 | rest << 10 | (total as u64 & 0x3f) << 16;
        self.visual[1] = v1;
        self.visual[2] = k as u64;
    }
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
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / spb, playing: true, sr: 48000.0, drum_notes: 0 };
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
    fn glass_breaks_each_chord_over_its_bass_then_moves_on() {
        // Rocking eighths, 4 repeats of 4: each chord lasts 8 beats.
        let mut p = defaults(GLASS);
        p[G_FIGURE] = 1.0;
        p[G_CHORDS] = 2.0; // 1 6: C, then A minor
        let n = notes(MidiFxKind::Glass, &p, 16.0);
        assert!(in_key(&n));
        let first: Vec<u8> = n.iter().filter(|x| x.0 < 7.9).map(|x| x.1 % 12).collect();
        let second: Vec<u8> = n.iter().filter(|x| x.0 >= 8.0 && x.0 < 15.9).map(|x| x.1 % 12).collect();
        assert!(first.iter().all(|c| [0, 4, 7].contains(c)), "{first:?}");
        assert!(second.iter().all(|c| [9, 0, 4].contains(c)), "{second:?}");
        // The bass: two octaves down, held for the chord.
        assert!(n.iter().any(|x| x.1 == 36 && x.2 - x.0 > 7.0), "bass");
        assert!(n.iter().any(|x| x.1 == 45 && x.0 >= 8.0), "A in the bass");
    }

    #[test]
    fn glass_plays_three_against_two() {
        let p = defaults(GLASS);
        let n = notes(MidiFxKind::Glass, &p, 8.0);
        // Above C5 the triplets: three to every two eighths (a beat).
        let high = n.iter().filter(|x| x.1 >= 72 && x.0 < 3.99).count();
        let low = n.iter().filter(|x| x.1 < 72 && x.1 > 40 && x.0 < 3.99).count();
        let times: Vec<(f64, u8)> = n.iter().filter(|x| x.1 >= 72 && x.0 < 3.99).map(|x| (x.0, x.1)).collect();
        assert_eq!(high, 12, "triplets {times:?}");
        assert_eq!(low, 8, "eighths");
    }

    #[test]
    fn glass_adds_and_takes_away() {
        // Additive: 2, 2, 3, 3, 4, 4... eighths before the figure grows.
        let mut p = defaults(GLASS);
        p[G_FIGURE] = 3.0;
        p[G_BASS] = 0.0;
        let n = notes(MidiFxKind::Glass, &p, 8.0);
        let pitches: Vec<u8> = n.iter().map(|x| x.1).collect();
        assert_eq!(&pitches[..10], &[60, 64, 60, 64, 60, 64, 67, 60, 64, 67]);
    }

    #[test]
    fn a_gymnopedie_waltzes_and_its_melody_comes_in_after_four_bars() {
        let mut p = defaults(SATIE);
        p[S_MELODY] = 1.0;
        let n = notes(MidiFxKind::Satie, &p, 48.0);
        assert!(in_key(&n));
        // In 3/4: the bass on every bar's first beat, low.
        for bar in 0..16 {
            let b = bar as f64 * 3.0;
            assert!(n.iter().any(|x| (x.0 - b).abs() < 0.01 && x.1 < 50), "bass in bar {bar}");
            // Nothing struck by the accompaniment on beat three.
            assert!(!n.iter().any(|x| (x.0 - b - 2.0).abs() < 0.01 && x.1 < 60), "beat three in bar {bar}");
        }
        // The chord on two: three notes.
        assert_eq!(n.iter().filter(|x| (x.0 - 1.0).abs() < 0.01).count(), 3);
        // The melody: none in the first four bars, some later.
        // (the chords reach up to E4; the melody sings above them)
        assert!(!n.iter().any(|x| x.1 > 64 && x.0 < 12.0), "melody waits");
        assert!(n.iter().any(|x| x.1 > 64 && x.0 >= 12.0), "melody comes");
    }

    #[test]
    fn a_gnossienne_oom_pahs_in_four_with_turns() {
        let mut p = defaults(SATIE);
        p[S_STYLE] = 1.0;
        p[S_MELODY] = 1.0;
        let n = notes(MidiFxKind::Satie, &p, 64.0);
        for bar in 0..4 {
            let b = bar as f64 * 4.0;
            assert!(n.iter().any(|x| (x.0 - b).abs() < 0.01 && x.1 < 50), "oom");
            assert!(n.iter().filter(|x| (x.0 - b - 1.0).abs() < 0.01).count() >= 3, "pah");
        }
        // Turns: notes a sixth of a beat apart.
        let quick = n.windows(2).filter(|w| (w[1].0 - w[0].0 - 1.0 / 6.0).abs() < 0.01 && w[0].1 >= 64 && w[1].1 >= 64).count();
        assert!(quick > 0, "no turns");
    }

    #[test]
    fn a_sarabande_slides_ninth_chords() {
        let mut p = defaults(SATIE);
        p[S_STYLE] = 2.0;
        let n = notes(MidiFxKind::Satie, &p, 12.0);
        let first = n.iter().filter(|x| x.0.abs() < 0.01 && x.1 >= 45).count();
        assert!(first >= 5, "five notes: {first}");
        assert!(in_key(&n));
    }

    #[test]
    fn riley_players_go_through_the_figures_over_the_pulse() {
        let p = defaults(RILEY);
        let n = notes(MidiFxKind::Riley, &p, 64.0);
        assert!(!n.is_empty());
        // The pulse: the tonic two octaves up on every eighth.
        let pulse = n.iter().filter(|x| x.1 == 84).count();
        assert!(pulse >= 120, "{pulse}");
        // The figures turn through a sharpened fourth and a flattened seventh.
        let total = 24;
        let some_sharp4 = (0..total).any(|i| figure(i, total).notes.iter().any(|n| n.3 == 1));
        let some_flat7 = (0..total).any(|i| figure(i, total).notes.iter().any(|n| n.3 == -1));
        assert!(some_sharp4 && some_flat7);
        // Three players: several notes at once now and then.
        let mut at: std::collections::BTreeMap<i64, usize> = Default::default();
        for x in n.iter().filter(|x| x.1 < 84) {
            *at.entry((x.0 * 1000.0).round() as i64).or_default() += 1;
        }
        assert!(at.values().any(|&c| c >= 2), "players together");
    }

    #[test]
    fn riley_players_move_on() {
        let mut p = defaults(RILEY);
        p[R_PATIENCE] = 0.0;
        let kind = MidiFxKind::Riley;
        let mut st = MidiFxState::new(kind);
        let spb = 24000.0;
        for b in 0..(96.0 * spb / 256.0) as u64 {
            let now = b * 256;
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / spb, playing: true, sr: 48000.0, drum_notes: 0 };
            let mut out = Vec::new();
            st.process(kind, &p, &[], &[], &mut out, &ctx);
        }
        let v = st.visual().expect("a sequencer shows its state");
        let figs: Vec<u64> = (0..3).map(|i| (v[1] >> (8 * i)) & 0xff).collect();
        assert!(figs.iter().all(|&f| f > 2), "{figs:?}");
        let (lo, hi) = (*figs.iter().min().unwrap(), *figs.iter().max().unwrap());
        assert!(hi - lo <= 3, "{figs:?}");
    }

    #[test]
    fn a_rainbow_loops_cells_of_different_lengths() {
        let mut p = defaults(RILEY);
        p[R_STYLE] = 1.0;
        p[R_PULSE] = 0.0;
        let n = notes(MidiFxKind::Riley, &p, 16.0);
        assert!(in_key(&n));
        // Sixteenths, often several at once.
        assert!(n.len() > 120, "{}", n.len());
    }
}
