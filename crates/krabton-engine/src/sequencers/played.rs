//! Sequencers you play into.
//!
//! - Spinner: every key you play drops a ball into a turning polygon; each
//!   time a ball hits a wall it plays its note, as hard as it hit. Spin,
//!   Sides, Gravity, Bounce and Size shape the machine; without Keep a ball
//!   leaves when its key is let go.
//! - Endless: with Record on, the notes you play (or Rest) are added one by
//!   one to a row of up to 64 steps; with it off the row plays round and
//!   round, and a key you hold moves it there (from its first note).
//! - Hands: hold keys and each plays a rhythm: one Pattern for all, or (Per
//!   Key) a pattern of its own, so a drum kit plays a groove under your
//!   fingers. Accent marks the strong steps.
//! - Draw: a line drawn across 32 steps, played in the key over a Range of
//!   octaves from the Base Note, every step or only where it moves.

use super::{SeqState, RATES};
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};
use std::f64::consts::TAU;

// ---------------------------------------------------------------- specs

pub mod spinner {
    pub const SPIN: usize = 0;
    pub const SIDES: usize = 1;
    pub const GRAVITY: usize = 2;
    pub const BOUNCE: usize = 3;
    pub const SIZE: usize = 4;
    pub const GATE: usize = 5;
    pub const VELOCITY: usize = 6;
    pub const KEEP: usize = 7;
}

pub static SPINNER: &[ParamSpec] = &[
    ParamSpec::lin("Spin", "Machine", -2.0, 2.0, 1.0, Unit::None),
    ParamSpec::int("Sides", "Machine", 3.0, 8.0, 6.0, Unit::None),
    ParamSpec::lin("Gravity", "Machine", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Bounce", "Machine", 0.5, 1.0, 0.96, Unit::Percent),
    ParamSpec::lin("Size", "Machine", 0.03, 0.2, 0.08, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.05, 2.0, 0.2, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.9, Unit::Percent),
    ParamSpec::toggle("Keep", "Notes", true),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

pub mod endless {
    pub const RATE: usize = 0;
    pub const GATE: usize = 1;
    pub const VELOCITY: usize = 2;
    pub const RECORD: usize = 3;
}

pub static ENDLESS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.8, Unit::Percent),
    ParamSpec::toggle("Record", "Record", true),
    ParamSpec::toggle("Transpose", "Keys", true),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

pub mod hands {
    pub const RATE: usize = 0;
    pub const PATTERN: usize = 1;
    pub const PER_KEY: usize = 2;
    pub const GATE: usize = 3;
    pub const VELOCITY: usize = 4;
    pub const ACCENT: usize = 5;
}

pub const PATTERNS: &[&str] = &["Four", "Offbeat", "Eighths", "Sixteenths", "Clave", "Tresillo", "Gallop", "Sparse"];
/// The patterns: 0 rest, 1 a hit, 2 an accented hit.
pub const PATTERN_STEPS: [[u8; 16]; 8] = [
    [2, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0],
    [0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, 0],
    [2, 0, 1, 0, 1, 0, 1, 0, 2, 0, 1, 0, 1, 0, 1, 0],
    [2, 1, 1, 1, 2, 1, 1, 1, 2, 1, 1, 1, 2, 1, 1, 1],
    [2, 0, 0, 1, 0, 0, 1, 0, 0, 0, 2, 0, 1, 0, 0, 0],
    [2, 0, 0, 1, 0, 0, 1, 0, 2, 0, 0, 1, 0, 0, 1, 0],
    [2, 0, 1, 1, 2, 0, 1, 1, 2, 0, 1, 1, 2, 0, 1, 1],
    [2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0],
];

pub static HANDS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Pattern", "Rhythm", PATTERNS, 0),
    ParamSpec::toggle("Per Key", "Rhythm", false),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.85, Unit::Percent),
    ParamSpec::lin("Accent", "Notes", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

pub mod draw {
    pub const RATE: usize = 0;
    pub const RANGE: usize = 1;
    pub const BASE: usize = 2;
    pub const MODE: usize = 3;
    pub const GATE: usize = 4;
    pub const VELOCITY: usize = 5;
}

pub const DRAW_MODES: &[&str] = &["Every Step", "On Change"];
pub const DRAW_POINTS: usize = 32;

pub static DRAW: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 4),
    ParamSpec::int("Range", "Line", 1.0, 4.0, 2.0, Unit::Octaves),
    ParamSpec::int("Base Note", "Line", 24.0, 84.0, 48.0, Unit::None),
    ParamSpec::choice("Mode", "Line", DRAW_MODES, 1),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.9, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.8, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

// ---------------------------------------------------------------- data

/// Endless's row: its length, then a note per step (0: a rest).
pub const ENDLESS_STEPS: usize = 64;

pub fn endless_default() -> Vec<u8> {
    vec![0; ENDLESS_STEPS + 1]
}

/// The row: (its steps, how many).
pub fn endless_row(data: &[u8]) -> (&[u8], usize) {
    let len = data.first().map(|&n| n as usize).unwrap_or(0).min(ENDLESS_STEPS).min(data.len().saturating_sub(1));
    (&data[1.min(data.len())..], len)
}

/// Adds a step (a note, or 0 for a rest); false when the row is full.
pub fn endless_add(data: &mut Vec<u8>, note: u8) -> bool {
    if data.len() < ENDLESS_STEPS + 1 {
        data.resize(ENDLESS_STEPS + 1, 0);
    }
    let len = data[0] as usize;
    if len >= ENDLESS_STEPS {
        return false;
    }
    data[1 + len] = note;
    data[0] = len as u8 + 1;
    true
}

/// Draw's line: a gentle rise and fall.
pub fn draw_default() -> Vec<u8> {
    (0..DRAW_POINTS).map(|i| (128.0 + 90.0 * (i as f64 / DRAW_POINTS as f64 * TAU).sin() + 30.0 * (i as f64 / DRAW_POINTS as f64 * TAU * 3.0).sin()) as u8).collect()
}

// ---------------------------------------------------------------- state

pub const MAX_SPIN_BALLS: usize = 16;

#[derive(Clone, Copy, Debug)]
struct Ball {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    pitch: u8,
    vel: f32,
}

pub struct PlayedState {
    balls: Vec<Ball>,
    angle: f64,
    /// Where the spinner's time is (song beat).
    spun_to: f64,
    /// Keys held (Hands), oldest first.
    held: Vec<(u8, f32)>,
    /// Draw's last degree (for On Change).
    last_degree: Option<i32>,
    seed: u32,
}

impl Default for PlayedState {
    fn default() -> Self {
        Self { balls: Vec::with_capacity(MAX_SPIN_BALLS), angle: 0.0, spun_to: f64::NEG_INFINITY, held: Vec::with_capacity(16), last_degree: None, seed: 0x5917_A11 }
    }
}

impl PlayedState {
    pub fn restart(&mut self) {
        self.spun_to = f64::NEG_INFINITY;
        self.last_degree = None;
    }

    fn rand(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed >> 8) as f64 / 16_777_216.0
    }
}

impl SeqState {
    /// Spinner and Hands take the keys played into them (they do not sound
    /// themselves).
    pub(crate) fn played_keys(&mut self, kind: crate::midi_fx::MidiFxKind, p: &[f32], out: &mut Vec<MidiEvent>) {
        use crate::midi_fx::MidiFxKind as K;
        let events = std::mem::take(out);
        for e in events.iter() {
            let pl = &mut self.played;
            match kind {
                K::Spinner if e.on => {
                    if pl.balls.len() == MAX_SPIN_BALLS {
                        pl.balls.remove(0);
                    }
                    let (vx, x) = (pl.rand() - 0.5, (pl.rand() - 0.5) * 0.3);
                    pl.balls.push(Ball { x, y: 0.3, vx, vy: 0.0, pitch: e.pitch, vel: e.vel });
                }
                K::Spinner => {
                    if p.get(spinner::KEEP).is_some_and(|v| *v < 0.5) {
                        pl.balls.retain(|b| b.pitch != e.pitch);
                    }
                }
                K::Hands => {
                    pl.held.retain(|h| h.0 != e.pitch);
                    if e.on {
                        if pl.held.len() == pl.held.capacity() {
                            pl.held.remove(0);
                        }
                        pl.held.push((e.pitch, e.vel));
                    }
                }
                _ => {}
            }
        }
        // Give the (now empty) buffer back, with its room.
        *out = events;
        out.clear();
    }

    /// Spinner: the balls fly until the end of this block; every wall they
    /// hit plays their note.
    pub(crate) fn spinner(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        use spinner::*;
        let b0 = ctx.beat;
        let b1 = b0 + ctx.len as f64 / ctx.samples_per_beat;
        if !(self.played.spun_to.is_finite() && (self.played.spun_to - b0).abs() < 0.05) {
            self.played.spun_to = b0;
        }
        let sides = (p[SIDES].round() as usize).clamp(3, 8);
        let omega = p[SPIN] as f64 * TAU / 4.0;
        let gravity = p[GRAVITY].clamp(0.0, 1.0) as f64 * 10.0;
        let bounce = p[BOUNCE].clamp(0.5, 1.0) as f64;
        let size = p[SIZE].clamp(0.03, 0.2) as f64;
        let gate = p[GATE].max(0.05) as f64 * ctx.samples_per_beat;
        let base_vel = p[VELOCITY];
        const DT: f64 = 1.0 / 96.0;
        let mut hits = [(0.0f64, 0u8, 0.0f32); 64];
        let mut n_hits = 0;
        let mut t = self.played.spun_to;
        while t + DT <= b1 {
            t += DT;
            self.played.angle = (self.played.angle + omega * DT).rem_euclid(TAU);
            let a0 = self.played.angle;
            for ball in self.played.balls.iter_mut() {
                ball.vy -= gravity * DT;
                ball.x += ball.vx * DT;
                ball.y += ball.vy * DT;
                for s in 0..sides {
                    let a = a0 + TAU * (s as f64 + 0.5) / sides as f64;
                    // The wall's inward normal, and how far inside the ball is.
                    let (nx, ny) = (-a.cos(), -a.sin());
                    let apothem = (TAU / (2.0 * sides as f64)).cos();
                    let d = apothem + (ball.x * nx + ball.y * ny);
                    if d < size {
                        let (cx, cy) = (ball.x - nx * d, ball.y - ny * d);
                        let (wx, wy) = (-omega * cy, omega * cx);
                        let (rx, ry) = (ball.vx - wx, ball.vy - wy);
                        let vn = rx * nx + ry * ny;
                        if vn < 0.0 {
                            let (tx, ty) = (rx - vn * nx, ry - vn * ny);
                            ball.vx = tx * 0.98 - vn * nx * bounce + wx;
                            ball.vy = ty * 0.98 - vn * ny * bounce + wy;
                            if -vn > 0.4 && n_hits < hits.len() {
                                hits[n_hits] = (t, ball.pitch, (base_vel * ball.vel * ((-vn / 4.0) as f32).clamp(0.35, 1.0)).clamp(0.05, 1.0));
                                n_hits += 1;
                            }
                        }
                        ball.x += nx * (size - d);
                        ball.y += ny * (size - d);
                    }
                }
                // Never out of the machine.
                let r = (ball.x * ball.x + ball.y * ball.y).sqrt();
                if r > 0.95 {
                    ball.x *= 0.9 / r;
                    ball.y *= 0.9 / r;
                }
            }
        }
        self.played.spun_to = t;
        for &(beat, pitch, vel) in &hits[..n_hits] {
            if beat >= b0 {
                let at = ctx.now + ((beat - b0) * ctx.samples_per_beat) as u64;
                self.note(ctx, at, pitch as i32, vel, gate, out);
            }
        }
        // For the screen: the angle, the sides, the balls.
        let mut bytes = [0u8; 32];
        let ang = (self.played.angle / TAU * 65535.0) as u16;
        bytes[0..2].copy_from_slice(&ang.to_le_bytes());
        bytes[2] = sides as u8;
        let shown = self.played.balls.len().min(14);
        bytes[3] = shown as u8;
        for (i, b) in self.played.balls.iter().take(14).enumerate() {
            bytes[4 + i * 2] = ((b.x.clamp(-1.0, 1.0) + 1.0) * 127.5) as u8;
            bytes[5 + i * 2] = ((b.y.clamp(-1.0, 1.0) + 1.0) * 127.5) as u8;
        }
        for (k, v) in self.visual.iter_mut().enumerate() {
            *v = u64::from_le_bytes(bytes[k * 8..k * 8 + 8].try_into().unwrap());
        }
    }

    /// Endless: step `k` of the row (nothing while it records).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn endless(&mut self, k: i64, p: &[f32], data: &[u8], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        use endless::*;
        let (row, len) = endless_row(data);
        let recording = p[RECORD] >= 0.5;
        let i = if len > 0 { (k as usize) % len } else { 0 };
        self.visual[0] = i as u64 | (len as u64) << 8 | (recording as u64) << 16;
        if recording || len == 0 {
            return;
        }
        // A held key moves the row from its first note.
        if let (Some((key, _)), Some(&first)) = (self.shift, row.iter().take(len).find(|&&n| n > 0)) {
            self.shift = Some((key, first as i32));
        }
        let note = row[i];
        if note > 0 {
            self.note(ctx, at, note as i32, p[VELOCITY], step_frames * p[GATE].clamp(0.1, 1.0) as f64, out);
        }
    }

    /// Hands: step `k` of the rhythm of every key held.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn hands(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        use hands::*;
        let s = (k as usize) % 16;
        let per_key = p[PER_KEY] >= 0.5;
        let shared = (p[PATTERN].round().max(0.0) as usize).min(PATTERN_STEPS.len() - 1);
        let accent = p[ACCENT].clamp(0.0, 1.0);
        let n_held = self.played.held.len();
        for h in 0..n_held {
            let (pitch, vel) = self.played.held[h];
            let pat = if per_key { pitch as usize % PATTERN_STEPS.len() } else { shared };
            let hit = PATTERN_STEPS[pat][s];
            if hit == 0 {
                continue;
            }
            let v = p[VELOCITY] * vel.max(0.3) * if hit == 2 { 1.0 } else { 1.0 - accent * 0.45 };
            self.note(ctx, at, pitch as i32, v, step_frames * p[GATE].clamp(0.1, 1.0) as f64, out);
        }
        self.visual[0] = s as u64 | (n_held as u64) << 8 | (if per_key { 255 } else { shared } as u64) << 16;
    }

    /// Draw: step `k` of the line, in the key.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_seq(&mut self, k: i64, p: &[f32], data: &[u8], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        use draw::*;
        let i = (k as usize) % DRAW_POINTS;
        self.visual[0] = i as u64;
        let point = |j: usize| data.get(j % DRAW_POINTS).copied().unwrap_or(128) as f32 / 255.0;
        let base = p[BASE].round() as i32;
        let range = p[RANGE].round().clamp(1.0, 4.0) as i32;
        let per_octave = if ctx.scale_on { ctx.scale.intervals().len() as i32 } else { 12 };
        let degree_of = |v: f32| (v * (range * per_octave) as f32).round() as i32;
        let pitch_of = |d: i32| if ctx.scale_on { ctx.scale.pitch_at(ctx.scale.locate(base).0 + d) } else { base + d };
        let d = degree_of(point(i));
        let on_change = p[MODE] >= 0.5;
        if on_change && self.played.last_degree == Some(d) {
            return;
        }
        self.played.last_degree = Some(d);
        // On Change: the note lasts until the line moves on.
        let steps = if on_change { (1..DRAW_POINTS).find(|&s| degree_of(point(i + s)) != d).unwrap_or(DRAW_POINTS) } else { 1 };
        self.note(ctx, at, pitch_of(d), p[VELOCITY], step_frames * (steps as f64 - 1.0 + p[GATE].clamp(0.1, 1.0) as f64), out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::MidiFxKind;
    use crate::scale::Scale;

    fn ctx(now: u64, beat: f64) -> MidiCtx {
        MidiCtx { now, len: 512, samples_per_beat: 24000.0, scale: Scale::default(), scale_on: false, beat, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 5] }
    }

    /// Runs `kind` for `beats`, playing `keys` into it at the start; returns the note-ons.
    fn run(kind: MidiFxKind, p: &[f32], data: &[u8], keys: &[u8], beats: f64) -> Vec<(f64, u8)> {
        let mut s = SeqState::new();
        let mut ons = Vec::new();
        let mut now = 0u64;
        let mut first = true;
        while (now as f64) < beats * 24000.0 {
            let c = ctx(now, now as f64 / 24000.0);
            let mut out: Vec<MidiEvent> = if first { keys.iter().map(|&k| MidiEvent::on(0, k, 0.9)).collect() } else { Vec::new() };
            first = false;
            s.process(kind, p, data, &mut out, &c);
            for e in out.iter().filter(|e| e.on) {
                ons.push(((now + e.offset as u64) as f64 / 24000.0, e.pitch));
            }
            now += 512;
        }
        ons
    }

    #[test]
    fn spinner_balls_play_their_notes_on_the_walls() {
        let p = crate::params::defaults(SPINNER);
        let ons = run(MidiFxKind::Spinner, &p, &[], &[60, 64, 67], 16.0);
        assert!(ons.len() >= 6, "the balls hit the walls: {}", ons.len());
        for n in [60, 64, 67] {
            assert!(ons.iter().any(|o| o.1 == n), "{n} plays");
        }
        assert!(ons.iter().all(|o| [60, 64, 67].contains(&o.1)), "only the notes played in");
        // No keys, no balls, nothing.
        assert!(run(MidiFxKind::Spinner, &p, &[], &[], 8.0).is_empty());
    }

    #[test]
    fn endless_plays_its_row_and_a_key_moves_it() {
        let mut p = crate::params::defaults(ENDLESS);
        let mut data = endless_default();
        for n in [60, 0, 64, 67] {
            assert!(endless_add(&mut data, n));
        }
        p[endless::RECORD] = 1.0;
        assert!(run(MidiFxKind::Endless, &p, &data, &[], 4.0).is_empty(), "silent while it records");
        p[endless::RECORD] = 0.0;
        let ons = run(MidiFxKind::Endless, &p, &data, &[], 2.0);
        let notes: Vec<u8> = ons.iter().map(|o| o.1).collect();
        assert_eq!(&notes[..6], &[60, 64, 67, 60, 64, 67], "a rest in between: {notes:?}");
        // Holding D moves the row up a whole tone (from its first note, C).
        let ons = run(MidiFxKind::Endless, &p, &data, &[62], 1.0);
        assert_eq!(ons.first().map(|o| o.1), Some(62));
    }

    #[test]
    fn hands_play_a_rhythm_per_key() {
        let mut p = crate::params::defaults(HANDS);
        let ons = run(MidiFxKind::Hands, &p, &[], &[36], 4.0);
        // Four on the floor, a sixteenth grid: a hit every beat.
        assert_eq!(ons.iter().filter(|o| o.0 < 3.999).count(), 4, "{ons:?}");
        p[hands::PER_KEY] = 1.0;
        let ons = run(MidiFxKind::Hands, &p, &[], &[36, 42], 4.0);
        let kicks = ons.iter().filter(|o| o.1 == 36).count();
        let hats = ons.iter().filter(|o| o.1 == 42).count();
        assert!(kicks > 0 && hats > 0 && kicks != hats, "each its own: {kicks} and {hats}");
    }

    #[test]
    fn draw_follows_its_line() {
        let mut p = crate::params::defaults(DRAW);
        p[draw::MODE] = 0.0;
        p[draw::RANGE] = 1.0;
        let up: Vec<u8> = (0..DRAW_POINTS).map(|i| (i * 255 / (DRAW_POINTS - 1)) as u8).collect();
        let ons = run(MidiFxKind::Draw, &p, &up, &[], 16.0);
        assert!(ons.len() >= 30, "{}", ons.len());
        let pitches: Vec<u8> = ons.iter().take(DRAW_POINTS).map(|o| o.1).collect();
        assert!(pitches.windows(2).all(|w| w[1] >= w[0]), "rising: {pitches:?}");
        assert_eq!(pitches[0], 48);
        assert_eq!(*pitches.last().unwrap(), 60, "an octave up at the top");
        // On Change: a flat line plays once and holds.
        p[draw::MODE] = 1.0;
        let flat = vec![100u8; DRAW_POINTS];
        assert_eq!(run(MidiFxKind::Draw, &p, &flat, &[], 16.0).len(), 1);
    }
}
