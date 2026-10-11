//! Playful generative sequencers. They live in a track's MIDI effect chain,
//! follow the transport (session clock) and play in the project scale.
//!
//! - Polymeter: Kria-style lanes (trigger, note, octave, duration,
//!   probability), each with its own loop length and clock division.
//! - Euclid: four euclidean rings (steps, pulses, rotation, note).
//! - Turing: a looping shift register that slowly mutates.
//! - Life: Conway's Game of Life on a 16x8 board, scanned column by column.
//! - Bounce: bouncing balls; every floor hit is a note. Each ball has its
//!   own note and drop height (in note values), hits can snap to a grid and
//!   balls that come to rest are dropped again on the beat or the bar.
//! - Drunk: a random walk through the scale.
//!
//! More in `more`: Orbits, Chaos, Automaton, Ant, Pong, Markov, Rain, Words
//! and Lorenz.

pub mod ambient;
pub mod strum;
pub mod more;

use crate::dsp::Rng;
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

pub const RATES: &[&str] = &["1/32", "1/16 T", "1/16", "1/8 T", "1/8", "1/4", "1/2"];
pub const RATE_BEATS: [f64; 7] = [0.125, 1.0 / 6.0, 0.25, 1.0 / 3.0, 0.5, 1.0, 2.0];

pub fn rate_beats(p: f32) -> f64 {
    RATE_BEATS[(p.round().max(0.0) as usize).min(RATE_BEATS.len() - 1)]
}

// ---------------------------------------------------------------- specs

pub static POLYMETER: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.8, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),];
pub static EUCLID: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::lin("Velocity", "Clock", 0.05, 1.0, 0.85, Unit::Percent),
    ParamSpec::int("Steps 1", "Ring 1", 1.0, 16.0, 16.0, Unit::None),
    ParamSpec::int("Pulses 1", "Ring 1", 0.0, 16.0, 4.0, Unit::None),
    ParamSpec::int("Rotate 1", "Ring 1", 0.0, 15.0, 0.0, Unit::None),
    ParamSpec::int("Note 1", "Ring 1", 0.0, 127.0, 36.0, Unit::None),
    ParamSpec::int("Steps 2", "Ring 2", 1.0, 16.0, 16.0, Unit::None),
    ParamSpec::int("Pulses 2", "Ring 2", 0.0, 16.0, 2.0, Unit::None),
    ParamSpec::int("Rotate 2", "Ring 2", 0.0, 15.0, 4.0, Unit::None),
    ParamSpec::int("Note 2", "Ring 2", 0.0, 127.0, 39.0, Unit::None),
    ParamSpec::int("Steps 3", "Ring 3", 1.0, 16.0, 16.0, Unit::None),
    ParamSpec::int("Pulses 3", "Ring 3", 0.0, 16.0, 7.0, Unit::None),
    ParamSpec::int("Rotate 3", "Ring 3", 0.0, 15.0, 0.0, Unit::None),
    ParamSpec::int("Note 3", "Ring 3", 0.0, 127.0, 42.0, Unit::None),
    ParamSpec::int("Steps 4", "Ring 4", 1.0, 16.0, 12.0, Unit::None),
    ParamSpec::int("Pulses 4", "Ring 4", 0.0, 16.0, 5.0, Unit::None),
    ParamSpec::int("Rotate 4", "Ring 4", 0.0, 15.0, 2.0, Unit::None),
    ParamSpec::int("Note 4", "Ring 4", 0.0, 127.0, 37.0, Unit::None),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),];
pub static TURING: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::int("Length", "Register", 2.0, 16.0, 8.0, Unit::None),
    ParamSpec::lin("Change", "Register", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::lin("Density", "Register", 0.0, 1.0, 0.75, Unit::Percent),
    ParamSpec::int("Range", "Notes", 1.0, 24.0, 7.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.8, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),];
pub static LIFE: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::int("Evolve", "Clock", 1.0, 32.0, 16.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 48.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),];
pub const DROPS: &[&str] = &["1/16", "1/8", "1/4", "1/2", "1 Bar", "2 Bars"];
pub const DROP_BEATS: [f64; 6] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0];
pub const REDROPS: &[&str] = &["At Once", "Next Beat", "Next Bar", "2 Bars"];
const REDROP_BEATS: [f64; 4] = [0.0, 1.0, 4.0, 8.0];

/// Bounce. The first five keep their meaning from earlier versions.
pub static BOUNCE: &[ParamSpec] = &[
    ParamSpec::int("Balls", "Physics", 1.0, 8.0, 4.0, Unit::None),
    ParamSpec::lin("Fall Time", "Physics", 0.25, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Energy", "Physics", 0.5, 0.97, 0.82, Unit::Percent),
    ParamSpec::int("Base Note", "Physics", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Velocity", "Physics", 0.05, 1.0, 0.9, Unit::Percent),
    ParamSpec::choice("Sync", "Physics", more::SYNCS, 2),
    ParamSpec::choice("Re-drop", "Physics", REDROPS, 2),
    ParamSpec::lin("Gate", "Physics", 0.05, 2.0, 0.25, Unit::None),
    ParamSpec::int("Note 1", "Notes", -14.0, 14.0, 0.0, Unit::None),
    ParamSpec::int("Note 2", "Notes", -14.0, 14.0, 2.0, Unit::None),
    ParamSpec::int("Note 3", "Notes", -14.0, 14.0, 4.0, Unit::None),
    ParamSpec::int("Note 4", "Notes", -14.0, 14.0, 7.0, Unit::None),
    ParamSpec::int("Note 5", "Notes", -14.0, 14.0, 9.0, Unit::None),
    ParamSpec::int("Note 6", "Notes", -14.0, 14.0, 11.0, Unit::None),
    ParamSpec::int("Note 7", "Notes", -14.0, 14.0, 14.0, Unit::None),
    ParamSpec::int("Note 8", "Notes", -14.0, 14.0, -3.0, Unit::None),
    ParamSpec::choice("Drop 1", "Drops", DROPS, 4),
    ParamSpec::choice("Drop 2", "Drops", DROPS, 3),
    ParamSpec::choice("Drop 3", "Drops", DROPS, 2),
    ParamSpec::choice("Drop 4", "Drops", DROPS, 5),
    ParamSpec::choice("Drop 5", "Drops", DROPS, 3),
    ParamSpec::choice("Drop 6", "Drops", DROPS, 4),
    ParamSpec::choice("Drop 7", "Drops", DROPS, 1),
    ParamSpec::choice("Drop 8", "Drops", DROPS, 5),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),];
pub static DRUNK: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::int("Step", "Walk", 1.0, 4.0, 1.0, Unit::None),
    ParamSpec::int("Range", "Walk", 2.0, 21.0, 10.0, Unit::None),
    ParamSpec::lin("Rest", "Walk", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),];

// ---------------------------------------------------------------- pattern data

/// Polymeter lanes: trigger, note, octave, duration, probability.
pub const LANES: usize = 5;
pub const LANE_NAMES: [&str; LANES] = ["Trigger", "Note", "Octave", "Length", "Chance"];
/// Highest value per lane (trigger 0/1, note degree 0..7, octave 0..4 with
/// 2 = no shift, duration 1..8 steps, probability 0..4 quarters).
pub const LANE_MAX: [u8; LANES] = [1, 7, 4, 8, 4];
pub const LANE_MIN: [u8; LANES] = [0, 0, 0, 1, 0];
/// Bytes per lane: 16 values, loop length, clock division.
pub const LANE_BYTES: usize = 18;

pub fn polymeter_default() -> Vec<u8> {
    let mut d = vec![0u8; LANES * LANE_BYTES];
    let lanes: [([u8; 16], u8, u8); LANES] = [
        ([1, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 0, 1, 0], 16, 1),
        ([0, 2, 4, 2, 5, 4, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0], 7, 1),
        ([2, 2, 3, 2, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 5, 1),
        ([1, 1, 2, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1], 9, 1),
        ([4, 4, 3, 4, 2, 4, 4, 3, 4, 4, 4, 4, 4, 4, 4, 4], 11, 1),
    ];
    for (l, (vals, len, div)) in lanes.iter().enumerate() {
        let o = l * LANE_BYTES;
        d[o..o + 16].copy_from_slice(vals);
        d[o + 16] = *len;
        d[o + 17] = *div;
    }
    d
}

/// Lane values, loop length and division from (possibly malformed) data.
pub fn lane(data: &[u8], l: usize) -> ([u8; 16], usize, usize) {
    let mut v = [LANE_MIN[l]; 16];
    let o = l * LANE_BYTES;
    if data.len() >= o + LANE_BYTES {
        for i in 0..16 {
            v[i] = data[o + i].clamp(LANE_MIN[l], LANE_MAX[l]);
        }
        (v, (data[o + 16] as usize).clamp(1, 16), (data[o + 17] as usize).clamp(1, 8))
    } else {
        (v, 16, 1)
    }
}

/// Life seed: 8 rows of 16 cells (row 0 at the top), 2 bytes per row.
pub fn life_default() -> Vec<u8> {
    let rows: [u16; 8] = [
        0b0000_0000_0000_0000,
        0b0000_0000_0100_0000,
        0b0000_0000_0010_0000,
        0b0011_1000_1110_0000,
        0b0000_0000_0000_0000,
        0b0000_0110_0000_0000,
        0b0000_0110_0000_0000,
        0b0000_0000_0000_0000,
    ];
    rows.iter().flat_map(|r| r.to_le_bytes()).collect()
}

pub fn life_rows(data: &[u8]) -> [u16; 8] {
    let mut rows = [0u16; 8];
    for (r, row) in rows.iter_mut().enumerate() {
        if data.len() >= r * 2 + 2 {
            *row = u16::from_le_bytes([data[r * 2], data[r * 2 + 1]]);
        }
    }
    rows
}

pub fn life_step(cells: &[u16; 8]) -> [u16; 8] {
    let alive = |x: i32, y: i32| -> u32 {
        let x = x.rem_euclid(16) as usize;
        let y = y.rem_euclid(8) as usize;
        ((cells[y] >> x) & 1) as u32
    };
    let mut next = [0u16; 8];
    for y in 0..8i32 {
        for x in 0..16i32 {
            let mut n = 0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx != 0 || dy != 0 {
                        n += alive(x + dx, y + dy);
                    }
                }
            }
            let me = alive(x, y) == 1;
            if (me && (n == 2 || n == 3)) || (!me && n == 3) {
                next[y as usize] |= 1 << x;
            }
        }
    }
    next
}

/// Whether step `i` of a euclidean rhythm is a hit.
pub fn euclid_hit(i: usize, steps: usize, pulses: usize, rotate: usize) -> bool {
    let steps = steps.max(1);
    if pulses == 0 {
        return false;
    }
    let j = (i % steps + steps - rotate % steps) % steps;
    (j * pulses.min(steps)) % steps < pulses.min(steps)
}

// ---------------------------------------------------------------- runtime

pub const MAX_BALLS: usize = 8;

/// Where a sequencer keeps its Transpose and Hold switches: the last two
/// parameters (Acid had Transpose before the others and keeps it at 7).
pub fn keys_params(kind: crate::midi_fx::MidiFxKind) -> (usize, usize) {
    if kind == crate::midi_fx::MidiFxKind::Acid {
        return (7, 8);
    }
    let n = kind.specs().len();
    (n.saturating_sub(2), n.saturating_sub(1))
}

pub struct SeqState {
    pub(crate) offs: Vec<(u64, u8)>,
    /// Notes that start in a later block: (frame, pitch, velocity, length).
    pub(crate) pending: Vec<(u64, u8, f32, u64)>,
    /// Acid: a note whose end waits for the next step (slide).
    pub(crate) acid_open: Option<u8>,
    /// Transpose: keys held on the track (newest last), the last one
    /// pressed, and the shift that applies now: (key, reference note).
    keys: Vec<u8>,
    latched: Option<u8>,
    pub(crate) shift: Option<(i32, i32)>,
    /// False when the sequencer applies the shift itself (Acid).
    shift_in_note: bool,
    pub(crate) rng: Rng,
    reg: u16,
    cells: [u16; 8],
    seed: [u16; 8],
    seeded: bool,
    generation: u32,
    /// Height, speed (in heights per beat) and when a resting ball drops again (song beat).
    balls: [(f64, f64, f64); MAX_BALLS],
    balls_ready: bool,
    /// The last (snapped) beat each ball played, so a grid never doubles a note.
    ball_last: [f64; MAX_BALLS],
    /// Where the next block should start; anything else is a jump (loop, locate).
    next_beat: f64,
    pub(crate) extra: Box<more::Extra>,
    pub(crate) ambient: Box<ambient::Ambient>,
    pos: i32,
    /// Packed state for the UI / grid (layout depends on the sequencer).
    pub visual: [u64; 4],
}

impl Default for SeqState {
    fn default() -> Self {
        Self::new()
    }
}

impl SeqState {
    pub fn new() -> Self {
        Self {
            offs: Vec::with_capacity(256),
            pending: Vec::with_capacity(256),
            acid_open: None,
            keys: Vec::with_capacity(16),
            latched: None,
            shift: None,
            shift_in_note: true,
            rng: Rng::new(0x7EA5_1234),
            reg: 0b1011_0010_0110_1001,
            cells: [0; 8],
            seed: [0; 8],
            seeded: false,
            generation: 0,
            balls: [(1.0, 0.0, 0.0); MAX_BALLS],
            balls_ready: false,
            ball_last: [f64::NEG_INFINITY; MAX_BALLS],
            next_beat: f64::NEG_INFINITY,
            extra: Box::default(),
            ambient: Box::default(),
            pos: 0,
            visual: [0; 4],
        }
    }

    pub fn reset(&mut self) {
        self.offs.clear();
        self.pending.clear();
        self.acid_open = None;
        self.seeded = false;
        self.balls_ready = false;
        self.ball_last = [f64::NEG_INFINITY; MAX_BALLS];
        self.extra.reset();
        self.ambient.restart();
    }

    fn unit(&mut self) -> f32 {
        (self.rng.bipolar() + 1.0) * 0.5
    }

    /// `pitch` moved by the held (or latched) key: in scale steps when the
    /// project's scale is on, so the pattern stays in key, else in semitones.
    pub(crate) fn shifted(&self, ctx: &MidiCtx, pitch: i32) -> i32 {
        match self.shift {
            None => pitch,
            Some((key, reference)) if ctx.scale_on => {
                let (d, rest) = ctx.scale.locate(pitch);
                let (dk, _) = ctx.scale.locate(key);
                let (dr, _) = ctx.scale.locate(reference);
                ctx.scale.pitch_at(d + dk - dr) + rest
            }
            Some((key, reference)) => pitch + key - reference,
        }
    }

    pub(crate) fn note(&mut self, ctx: &MidiCtx, at: u64, pitch: i32, vel: f32, len_frames: f64, out: &mut Vec<MidiEvent>) {
        let pitch = if self.shift_in_note { self.shifted(ctx, pitch) } else { pitch };
        let pitch = pitch.clamp(0, 127) as u8;
        let offset = at.saturating_sub(ctx.now).min(ctx.len.saturating_sub(1) as u64) as u32;
        // Retrigger: end a still-sounding copy first.
        if let Some(i) = self.offs.iter().position(|o| o.1 == pitch) {
            self.offs.swap_remove(i);
            out.push(MidiEvent::off(offset, pitch));
        }
        out.push(MidiEvent::on(offset, pitch, vel.clamp(0.01, 1.0)));
        if self.offs.len() < self.offs.capacity() {
            self.offs.push((at + len_frames.max(1.0) as u64, pitch));
        }
    }

    fn flush_offs(&mut self, ctx: &MidiCtx, out: &mut Vec<MidiEvent>) {
        let end = ctx.now + ctx.len as u64;
        let mut i = 0;
        while i < self.offs.len() {
            if self.offs[i].0 < end {
                let (at, pitch) = self.offs.swap_remove(i);
                out.push(MidiEvent::off(at.saturating_sub(ctx.now).min(ctx.len.saturating_sub(1) as u64) as u32, pitch));
            } else {
                i += 1;
            }
        }
    }

    /// Generates this block's notes. `kind` selects the sequencer.
    pub fn process(&mut self, kind: crate::midi_fx::MidiFxKind, p: &[f32], data: &[u8], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        use crate::midi_fx::MidiFxKind as K;
        // Transpose: notes coming in move the pattern instead of sounding;
        // with Hold the last one stays, otherwise only while it is held.
        let (ti, hi) = keys_params(kind);
        if p.get(ti).is_some_and(|v| *v >= 0.5) {
            for e in out.iter() {
                self.keys.retain(|k| *k != e.pitch);
                if e.on {
                    if self.keys.len() == self.keys.capacity() {
                        self.keys.remove(0);
                    }
                    self.keys.push(e.pitch);
                    self.latched = Some(e.pitch);
                }
            }
            out.clear();
            let hold = p.get(hi).is_some_and(|v| *v >= 0.5);
            let key = if hold { self.latched } else { self.keys.last().copied() };
            // The pattern is where it was written with its base note held (C3 without one).
            let reference = kind.specs().iter().position(|s| s.name == "Base Note").and_then(|i| p.get(i)).map(|v| v.round() as i32).unwrap_or(60);
            self.shift = key.map(|k| (k as i32, reference));
        } else {
            self.keys.clear();
            self.latched = None;
            self.shift = None;
        }
        // Acid and Guitar Chords move their own notes (a semitone line, chord shapes).
        self.shift_in_note = !matches!(kind, K::Acid | K::GuitarChords);
        if ctx.playing {
            self.flush_pending(ctx, out);
        }
        if !ctx.playing && kind == K::Bloom {
            // Your taps sound with the transport stopped too.
            let fresh = std::mem::replace(&mut self.next_beat, f64::NEG_INFINITY).is_finite();
            self.bloom_stopped(p, data, out, ctx, fresh);
            self.flush_pending(ctx, out);
        }
        if ctx.playing {
            if matches!(kind, K::Bounce | K::Orbits | K::Pong | K::Rain | K::Loops | K::Bloom | K::Chimes | K::DriftChords | K::Pedal) {
                // Time-based: they keep their own time against the song's beat.
                if (ctx.beat - self.next_beat).abs() > 1e-3 {
                    // The song jumped (a loop, a locate, a start): start over from here.
                    self.ball_last = [f64::NEG_INFINITY; MAX_BALLS];
                    self.balls_ready = false;
                    self.extra.rain_next = ctx.beat;
                    self.ambient.restart();
                }
                self.next_beat = ctx.beat + ctx.len as f64 / ctx.samples_per_beat;
                match kind {
                    K::Bounce => self.bounce(p, out, ctx),
                    K::Orbits => self.orbits(p, out, ctx),
                    K::Pong => self.pong(p, out, ctx),
                    K::Loops => self.loops(p, out, ctx),
                    K::Bloom => self.bloom(p, data, out, ctx, true),
                    K::Chimes => self.chimes(p, out, ctx),
                    K::DriftChords => self.drift_chords(p, out, ctx),
                    K::Pedal => self.pedal(p, out, ctx),
                    _ => self.rain(p, out, ctx),
                }
            } else {
                let rate = match kind {
                    K::Polymeter | K::Euclid | K::Turing | K::Life | K::Drunk | K::Step | K::Acid | K::DrumGrid => rate_beats(p[0]),
                    K::Chaos | K::Automaton | K::Ant | K::Markov | K::Words | K::Lorenz | K::Tintinnabuli => rate_beats(p[0]),
                    _ => 0.25,
                };
                let beat0 = ctx.beat;
                let beat1 = beat0 + ctx.len as f64 / ctx.samples_per_beat;
                let mut k = (beat0 / rate - 1e-9).ceil() as i64;
                while (k as f64) * rate < beat1 {
                    if k >= 0 {
                        let at = ctx.now + (((k as f64) * rate - beat0).max(0.0) * ctx.samples_per_beat) as u64;
                        let step_frames = rate * ctx.samples_per_beat;
                        match kind {
                            K::Polymeter => self.polymeter(k, p, data, at, step_frames, out, ctx),
                            K::Euclid => self.euclid(k, p, at, step_frames, out, ctx),
                            K::Turing => self.turing(p, at, step_frames, out, ctx),
                            K::Life => self.life(k, p, data, at, step_frames, out, ctx),
                            K::Drunk => self.drunk(p, at, step_frames, out, ctx),
                            K::Step => self.step_seq(k, p, data, at, step_frames, out, ctx),
                            K::Acid => self.acid(k, p, data, at, step_frames, out, ctx),
                            K::DrumGrid => self.drum_grid(k, p, data, at, step_frames, out, ctx),
                            K::Progression => self.progression(k, p, data, at, step_frames, out, ctx),
                            K::GuitarChords => self.strum(k, p, data, at, step_frames, out, ctx),
                            K::Chaos => self.chaos(p, at, step_frames, out, ctx),
                            K::Automaton => self.automaton(p, at, step_frames, out, ctx),
                            K::Ant => self.ant(p, at, step_frames, out, ctx),
                            K::Markov => self.markov(p, at, step_frames, out, ctx),
                            K::Words => self.words(p, data, at, step_frames, out, ctx),
                            K::Lorenz => self.lorenz(p, at, step_frames, out, ctx),
                            K::Tintinnabuli => self.tintinnabuli(k, p, at, step_frames, out, ctx),
                            _ => {}
                        }
                    }
                    k += 1;
                }
            }
        }
        self.flush_offs(ctx, out);
        if kind == K::Life && !self.seeded {
            // Show the seed while stopped.
            let seed = life_rows(data);
            self.visual[0] = pack_rows(&seed[0..4]);
            self.visual[1] = pack_rows(&seed[4..8]);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn polymeter(&mut self, k: i64, p: &[f32], data: &[u8], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let pos = |l: usize| -> usize {
            let (_, len, div) = lane(data, l);
            ((k as usize) / div) % len
        };
        let mut positions = 0u64;
        for l in 0..LANES {
            positions |= (pos(l) as u64) << (l * 8);
        }
        self.visual[0] = positions;
        let (trig, _, _) = lane(data, 0);
        if trig[pos(0)] == 0 {
            return;
        }
        let chance = lane(data, 4).0[pos(4)] as f32 / 4.0;
        if self.unit() >= chance && chance < 1.0 {
            return;
        }
        let degree = lane(data, 1).0[pos(1)] as i32;
        let octave = lane(data, 2).0[pos(2)] as i32 - 2;
        let dur = lane(data, 3).0[pos(3)] as f64;
        let (base, _) = ctx.scale.locate(p[1].round() as i32);
        let pitch = ctx.scale.pitch_at(base + degree) + 12 * octave;
        self.note(ctx, at, pitch, p[3], dur * step_frames * p[2] as f64, out);
    }

    fn euclid(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let mut vis = 0u64;
        for r in 0..4 {
            let b = 2 + r * 4;
            let steps = p[b].round().max(1.0) as usize;
            let i = (k as usize) % steps;
            vis |= (i as u64) << (r * 8);
            if euclid_hit(i, steps, p[b + 1].round() as usize, p[b + 2].round() as usize) {
                self.note(ctx, at, p[b + 3].round() as i32, p[1], step_frames * 0.5, out);
            }
        }
        self.visual[0] = vis;
    }

    fn turing(&mut self, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let len = p[1].round().clamp(2.0, 16.0) as u32;
        let mask: u32 = (1u32 << len) - 1;
        let mut bit = ((self.reg as u32) >> (len - 1)) & 1;
        if self.unit() < p[2] {
            bit ^= 1;
        }
        let reg = (((self.reg as u32) << 1) | bit) & mask;
        self.reg = reg as u16;
        self.visual[0] = reg as u64 | (len as u64) << 16;
        let value = (reg & 0xff) as f32 / (mask.min(0xff)) as f32;
        let gate_value = ((reg.wrapping_mul(0x9E37) >> 5) & 0xff) as f32 / 255.0;
        if gate_value >= p[3] {
            return;
        }
        let degree = (value * p[4]).round() as i32;
        let (base, _) = ctx.scale.locate(p[5].round() as i32);
        let pitch = ctx.scale.pitch_at(base + degree);
        self.note(ctx, at, pitch, p[7], step_frames * p[6] as f64, out);
    }

    #[allow(clippy::too_many_arguments)]
    fn life(&mut self, k: i64, p: &[f32], data: &[u8], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let seed = life_rows(data);
        if !self.seeded || seed != self.seed || self.cells.iter().all(|r| *r == 0) {
            self.seed = seed;
            self.cells = if seed.iter().all(|r| *r == 0) { self.random_cells() } else { seed };
            self.seeded = true;
            self.generation = 0;
        }
        let col = (k as usize) % 16;
        let (base, _) = ctx.scale.locate(p[2].round() as i32);
        for row in 0..8 {
            if (self.cells[row] >> col) & 1 == 1 {
                let pitch = ctx.scale.pitch_at(base + (7 - row as i32));
                self.note(ctx, at, pitch, p[4], step_frames * p[3] as f64, out);
            }
        }
        let evolve = p[1].round().max(1.0) as i64;
        if (k + 1) % evolve == 0 {
            self.cells = life_step(&self.cells);
            self.generation += 1;
        }
        self.visual[0] = pack_rows(&self.cells[0..4]);
        self.visual[1] = pack_rows(&self.cells[4..8]);
        self.visual[2] = col as u64 | (self.generation as u64) << 8;
    }

    fn random_cells(&mut self) -> [u16; 8] {
        let mut c = [0u16; 8];
        for row in c.iter_mut() {
            *row = (self.rng.next_u32() & self.rng.next_u32()) as u16;
        }
        c
    }

    fn drunk(&mut self, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let range = p[2].round().max(2.0) as i32;
        let step = p[1].round().max(1.0) as i32;
        let delta = (self.unit() * (2 * step + 1) as f32).floor() as i32 - step;
        let mut pos = self.pos + delta;
        if pos < 0 {
            pos = -pos;
        }
        if pos > range {
            pos = 2 * range - pos;
        }
        self.pos = pos.clamp(0, range);
        self.visual[0] = self.pos as u64 | (range as u64) << 8;
        if self.unit() < p[3] {
            return;
        }
        let (base, _) = ctx.scale.locate(p[4].round() as i32);
        let pitch = ctx.scale.pitch_at(base + self.pos);
        self.note(ctx, at, pitch, p[6], step_frames * p[5] as f64, out);
    }

    fn bounce(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let n = p[0].round().clamp(1.0, MAX_BALLS as f32) as usize;
        let energy = p[2].clamp(0.3, 0.99) as f64;
        let grid = more::sync_beats(p[5]);
        let redrop = REDROP_BEATS[(p[6].round().max(0.0) as usize).min(REDROP_BEATS.len() - 1)];
        let gate = p[7].max(0.01) as f64 * ctx.samples_per_beat;
        let (b0, b1) = (ctx.beat, ctx.beat + ctx.len as f64 / ctx.samples_per_beat);
        // When a ball that has come to rest goes up again.
        let next_drop = |t: f64| if redrop > 0.0 { (t / redrop - 1e-9).ceil() * redrop } else { t };
        if !self.balls_ready {
            let first = next_drop(b0);
            for b in self.balls.iter_mut() {
                *b = (1.0, 0.0, first);
            }
            self.balls_ready = true;
        }
        let (base, _) = ctx.scale.locate(p[3].round() as i32);
        let mut vis = 0u64;
        for i in 0..n {
            // A fall from the top takes the ball's drop length (in beats).
            let fall = DROP_BEATS[(p[16 + i].round().max(0.0) as usize).min(DROP_BEATS.len() - 1)] * p[1].clamp(0.1, 8.0) as f64;
            let g = 2.0 / (fall * fall);
            let full = (2.0 * g).sqrt();
            let (mut y, mut v, mut wait) = self.balls[i];
            let mut t = b0;
            for _ in 0..64 {
                if wait > t {
                    if wait >= b1 {
                        y = 1.0;
                        v = 0.0;
                        break;
                    }
                    t = wait;
                    y = 1.0;
                    v = 0.0;
                }
                // Time to the floor: y + v s - g s^2 / 2 = 0.
                let s = (v + (v * v + 2.0 * g * y.max(0.0)).sqrt()) / g;
                if t + s >= b1 {
                    let d = b1 - t;
                    y = (y + v * d - 0.5 * g * d * d).max(0.0);
                    v -= g * d;
                    break;
                }
                let hit = t + s;
                let impact = (v - g * s).abs();
                let vel = (impact / full).clamp(0.05, 1.0) as f32 * p[4];
                let pitch = ctx.scale.pitch_at(base + p[8 + i].round() as i32);
                let on = more::snap(hit, grid);
                if on > self.ball_last[i] + 1e-6 {
                    self.ball_last[i] = on;
                    let at = ctx.now + ((on - b0) * ctx.samples_per_beat).max(0.0) as u64;
                    self.play(ctx, at, pitch, vel, gate, out);
                }
                t = hit;
                y = 0.0;
                v = impact * energy;
                if v < full * 0.12 {
                    // Out of bounce: up again, on the beat or bar if asked.
                    wait = next_drop(hit).max(hit + 1e-6);
                    if redrop == 0.0 {
                        wait = hit;
                    }
                    y = 1.0;
                    v = 0.0;
                }
            }
            self.balls[i] = (y, v, wait);
            vis |= ((y.clamp(0.0, 1.0) * 255.0) as u64) << (i * 8);
        }
        self.visual[0] = vis;
        self.visual[1] = n as u64;
    }
}

fn pack_rows(rows: &[u16]) -> u64 {
    rows.iter().enumerate().fold(0u64, |acc, (i, r)| acc | (*r as u64) << (i * 16))
}

/// Unpacks the Life board from a visual state.
pub fn life_from_visual(v: &[u64; 4]) -> [u16; 8] {
    let mut rows = [0u16; 8];
    for (r, row) in rows.iter_mut().enumerate() {
        *row = ((v[r / 4] >> ((r % 4) * 16)) & 0xffff) as u16;
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::{MidiFxKind, MidiFxState};
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};

    /// Runs `beats` beats at 120 BPM / 48 kHz and returns note-ons as (beat, pitch).
    pub(crate) fn run(kind: MidiFxKind, p: &[f32], data: &[u8], beats: f64) -> Vec<(f64, u8)> {
        let spb = 24000.0;
        let mut st = MidiFxState::new(kind);
        let mut ons = Vec::new();
        let blocks = (beats * spb / 256.0) as u64;
        for b in 0..blocks {
            let now = b * 256;
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / spb, playing: true, sr: 48000.0 };
            let mut out = Vec::new();
            st.process(kind, p, data, &[], &mut out, &ctx);
            for e in out.iter().filter(|e| e.on) {
                ons.push(((now + e.offset as u64) as f64 / spb, e.pitch));
            }
        }
        ons
    }

    #[test]
    fn euclid_patterns() {
        let hits: Vec<bool> = (0..16).map(|i| euclid_hit(i, 16, 4, 0)).collect();
        assert_eq!(hits.iter().filter(|h| **h).count(), 4);
        assert!(hits[0] && hits[4] && hits[8] && hits[12]);
        assert_eq!((0..8).filter(|&i| euclid_hit(i, 8, 3, 0)).count(), 3);
        assert!(!euclid_hit(0, 16, 4, 1) && euclid_hit(1, 16, 4, 1));
        let p = defaults(EUCLID);
        let ons = run(MidiFxKind::Euclid, &p, &[], 4.0);
        // Kick on every beat.
        let kicks: Vec<f64> = ons.iter().filter(|o| o.1 == 36).map(|o| o.0).collect();
        assert_eq!(kicks.len(), 4);
        assert!((kicks[1] - 1.0).abs() < 1e-3);
    }

    #[test]
    fn polymeter_plays_in_key_with_polymetric_lanes() {
        let p = defaults(POLYMETER);
        let data = polymeter_default();
        let ons = run(MidiFxKind::Polymeter, &p, &data, 8.0);
        assert!(ons.len() > 8, "{}", ons.len());
        let c = Scale::new(0, ScaleKind::Major);
        assert!(ons.iter().all(|o| c.contains(o.1)));
        // Different lane lengths: the first two bars differ.
        let bar1: Vec<u8> = ons.iter().filter(|o| o.0 < 4.0).map(|o| o.1).collect();
        let bar2: Vec<u8> = ons.iter().filter(|o| o.0 >= 4.0 && o.0 < 8.0).map(|o| o.1).collect();
        assert_ne!(bar1, bar2);
    }

    #[test]
    fn turing_locks_into_a_loop() {
        let mut p = defaults(TURING);
        p[2] = 0.0; // never change: the register loops
        p[3] = 1.0; // every step plays
        let ons = run(MidiFxKind::Turing, &p, &[], 8.0);
        let len = p[1] as usize;
        assert!(ons.len() >= 2 * len);
        for i in 0..len {
            assert_eq!(ons[i].1, ons[i + len].1, "step {i}");
        }
    }

    #[test]
    fn life_board_evolves() {
        let glider_free = life_default();
        let cells = life_rows(&glider_free);
        let next = life_step(&cells);
        assert_ne!(cells, next);
        // A blinker oscillates with period 2.
        let mut b = [0u16; 8];
        b[3] = 0b111 << 5;
        assert_eq!(life_step(&life_step(&b)), b);
        let p = defaults(LIFE);
        let ons = run(MidiFxKind::Life, &p, &glider_free, 8.0);
        assert!(!ons.is_empty());
    }

    #[test]
    fn bounce_and_drunk_make_notes() {
        let p = defaults(BOUNCE);
        let ons = run(MidiFxKind::Bounce, &p, &[], 16.0);
        assert!(ons.len() >= 8, "bounces: {}", ons.len());
        // Two balls dropped from a quarter note, free running: ball 0 plays the
        // base note, ball 1 its own note (two scale steps up), and the gaps
        // between hits shrink.
        let mut p = defaults(BOUNCE);
        p[0] = 2.0;
        p[5] = 0.0;
        p[16] = 2.0;
        p[17] = 2.0;
        let ons = run(MidiFxKind::Bounce, &p, &[], 8.0);
        let b0: Vec<f64> = ons.iter().filter(|o| o.1 == 60).map(|o| o.0).collect();
        assert!(b0.len() >= 4, "{b0:?}");
        assert!((b0[0] - 1.0).abs() < 0.01, "first hit after one beat: {b0:?}");
        assert!(b0[2] - b0[1] < b0[1] - b0[0]);
        assert!(ons.iter().any(|o| o.1 == 64));
        assert!(ons.iter().all(|o| o.1 == 60 || o.1 == 64));
        // Synced to quarter notes: every hit lands on a beat, never twice.
        p[5] = 4.0;
        let ons = run(MidiFxKind::Bounce, &p, &[], 16.0);
        assert!(ons.len() >= 6);
        assert!(ons.iter().all(|o| (o.0 - o.0.round()).abs() < 0.01), "{ons:?}");
        for pitch in [60, 64] {
            let b: Vec<f64> = ons.iter().filter(|o| o.1 == pitch).map(|o| o.0).collect();
            assert!(b.windows(2).all(|w| w[1] > w[0] + 0.5), "{b:?}");
        }
        let p = defaults(DRUNK);
        let ons = run(MidiFxKind::Drunk, &p, &[], 8.0);
        assert!(ons.len() > 10);
        assert!(ons.windows(2).all(|w| (w[0].1 as i32 - w[1].1 as i32).abs() <= 4));
    }

    #[test]
    fn transpose_follows_keys_and_hold_keeps_the_last() {
        use crate::midi_fx::MidiEvent;
        for hold in [false, true] {
            // One planet on C3, every beat.
            let mut p = defaults(more::ORBITS);
            p[0] = 1.0;
            p[5] = 4.0;
            let (ti, hi) = keys_params(MidiFxKind::Orbits);
            p[ti] = 1.0;
            p[hi] = if hold { 1.0 } else { 0.0 };
            let spb = 24000.0;
            let mut st = MidiFxState::new(MidiFxKind::Orbits);
            let mut ons = Vec::new();
            let release = (3.5 * spb / 256.0) as u64;
            for b in 0..(8.0 * spb / 256.0) as u64 {
                let now = b * 256;
                let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / spb, playing: true, sr: 48000.0 };
                // D3 pressed just after the first beat, let go halfway through the fourth.
                let input = match b {
                    1 => vec![MidiEvent::on(0, 62, 0.8)],
                    b if b == release => vec![MidiEvent::off(0, 62)],
                    _ => Vec::new(),
                };
                let mut out = Vec::new();
                st.process(MidiFxKind::Orbits, &p, &[], &input, &mut out, &ctx);
                ons.extend(out.iter().filter(|e| e.on).map(|e| ((now + e.offset as u64) as f64 / spb, e.pitch)));
            }
            // The key itself never sounds; the planet moves up a scale step while it is held.
            let at = |beat: f64| ons.iter().find(|o| (o.0 - beat).abs() < 0.01).map(|o| o.1);
            assert_eq!(at(0.0), Some(60));
            assert_eq!(at(1.0), Some(62));
            assert_eq!(at(3.0), Some(62));
            assert_eq!(at(5.0), Some(if hold { 62 } else { 60 }), "hold {hold}");
            assert_eq!(ons.len(), 8, "{ons:?}");
        }
    }

    #[test]
    fn every_sequencer_has_transpose_and_hold() {
        for kind in MidiFxKind::SEQUENCERS {
            let (ti, hi) = keys_params(kind);
            assert_eq!(kind.specs()[ti].name, "Transpose", "{kind:?}");
            assert_eq!(kind.specs()[hi].name, "Hold", "{kind:?}");
        }
    }

    #[test]
    fn silent_when_stopped() {
        let mut st = MidiFxState::new(MidiFxKind::Euclid);
        let ctx = MidiCtx { now: 0, len: 256, samples_per_beat: 24000.0, scale: Scale::default(), scale_on: true, beat: 0.0, playing: false, sr: 48000.0 };
        let mut out = Vec::new();
        st.process(MidiFxKind::Euclid, &defaults(EUCLID), &[], &[], &mut out, &ctx);
        assert!(out.is_empty());
    }
}
