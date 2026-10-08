//! Playful generative sequencers. They live in a track's MIDI effect chain,
//! follow the transport (session clock) and play in the project scale.
//!
//! - Polymeter: Kria-style lanes (trigger, note, octave, duration,
//!   probability), each with its own loop length and clock division.
//! - Euclid: four euclidean rings (steps, pulses, rotation, note).
//! - Turing: a looping shift register that slowly mutates.
//! - Life: Conway's Game of Life on a 16x8 board, scanned column by column.
//! - Bounce: bouncing balls; every floor hit is a note.
//! - Drunk: a random walk through the scale.

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
];
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
];
pub static TURING: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::int("Length", "Register", 2.0, 16.0, 8.0, Unit::None),
    ParamSpec::lin("Change", "Register", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::lin("Density", "Register", 0.0, 1.0, 0.75, Unit::Percent),
    ParamSpec::int("Range", "Notes", 1.0, 24.0, 7.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.8, Unit::Percent),
];
pub static LIFE: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::int("Evolve", "Clock", 1.0, 32.0, 16.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 48.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
];
pub static BOUNCE: &[ParamSpec] = &[
    ParamSpec::int("Balls", "Physics", 1.0, 8.0, 4.0, Unit::None),
    ParamSpec::lin("Fall Time", "Physics", 0.25, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Energy", "Physics", 0.5, 0.97, 0.82, Unit::Percent),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.9, Unit::Percent),
];
pub static DRUNK: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::int("Step", "Walk", 1.0, 4.0, 1.0, Unit::None),
    ParamSpec::int("Range", "Walk", 2.0, 21.0, 10.0, Unit::None),
    ParamSpec::lin("Rest", "Walk", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
];

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

pub struct SeqState {
    pub(crate) offs: Vec<(u64, u8)>,
    /// Notes that start in a later block: (frame, pitch, velocity, length).
    pub(crate) pending: Vec<(u64, u8, f32, u64)>,
    /// Acid: a note whose end waits for the next step (slide).
    pub(crate) acid_open: Option<u8>,
    /// Acid: the key held on the track, for pattern transposition.
    pub(crate) held_key: Option<u8>,
    pub(crate) rng: Rng,
    reg: u16,
    cells: [u16; 8],
    seed: [u16; 8],
    seeded: bool,
    generation: u32,
    balls: [(f32, f32); MAX_BALLS],
    balls_ready: bool,
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
            held_key: None,
            rng: Rng::new(0x7EA5_1234),
            reg: 0b1011_0010_0110_1001,
            cells: [0; 8],
            seed: [0; 8],
            seeded: false,
            generation: 0,
            balls: [(1.0, 0.0); MAX_BALLS],
            balls_ready: false,
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
    }

    fn unit(&mut self) -> f32 {
        (self.rng.bipolar() + 1.0) * 0.5
    }

    pub(crate) fn note(&mut self, ctx: &MidiCtx, at: u64, pitch: i32, vel: f32, len_frames: f64, out: &mut Vec<MidiEvent>) {
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
        if kind == K::Acid && p.len() > 7 && p[7] >= 0.5 {
            // Key Transpose: held keys move the pattern instead of sounding.
            for e in out.iter() {
                if e.on {
                    self.held_key = Some(e.pitch);
                } else if self.held_key == Some(e.pitch) {
                    self.held_key = None;
                }
            }
            out.clear();
        }
        if ctx.playing {
            self.flush_pending(ctx, out);
        }
        if ctx.playing {
            if kind == K::Bounce {
                self.bounce(p, out, ctx);
            } else {
                let rate = match kind {
                    K::Polymeter | K::Euclid | K::Turing | K::Life | K::Drunk | K::Step | K::Acid | K::DrumGrid => rate_beats(p[0]),
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
        let fall = p[1].max(0.05);
        let g = 2.0 / (fall * fall);
        let energy = p[2];
        let height = |i: usize| 1.0 - i as f32 * 0.09;
        if !self.balls_ready {
            for i in 0..MAX_BALLS {
                self.balls[i] = (height(i), 0.0);
            }
            self.balls_ready = true;
        }
        let dt = (ctx.len as f64 / ctx.samples_per_beat) as f32;
        let (base, _) = ctx.scale.locate(p[3].round() as i32);
        let mut vis = 0u64;
        for i in 0..n {
            let (mut y, mut v) = self.balls[i];
            // Time until the floor within this block (solve y + v t - g t^2 / 2 = 0).
            let t_hit = (v + (v * v + 2.0 * g * y).sqrt()) / g;
            if t_hit <= dt {
                let impact = (v - g * t_hit).abs();
                let full = (2.0 * g * height(i)).sqrt();
                let vel = (impact / full).clamp(0.05, 1.0) * p[4];
                let at = ctx.now + (t_hit as f64 * ctx.samples_per_beat) as u64;
                let pitch = ctx.scale.pitch_at(base + i as i32 * 2);
                self.note(ctx, at, pitch, vel, ctx.samples_per_beat * 0.25, out);
                v = impact * energy;
                y = 0.0;
                let rest = dt - t_hit;
                if v < full * 0.12 {
                    // Out of energy: drop it again from the top.
                    y = height(i);
                    v = 0.0;
                } else {
                    y += v * rest - 0.5 * g * rest * rest;
                    v -= g * rest;
                    y = y.max(0.0);
                }
            } else {
                y += v * dt - 0.5 * g * dt * dt;
                v -= g * dt;
            }
            self.balls[i] = (y, v);
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
    fn run(kind: MidiFxKind, p: &[f32], data: &[u8], beats: f64) -> Vec<(f64, u8)> {
        let spb = 24000.0;
        let mut st = MidiFxState::new(kind);
        let mut ons = Vec::new();
        let blocks = (beats * spb / 256.0) as u64;
        for b in 0..blocks {
            let now = b * 256;
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / spb, playing: true };
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
        let ons = run(MidiFxKind::Bounce, &p, &[], 8.0);
        assert!(ons.len() >= 8, "bounces: {}", ons.len());
        // Bounces speed up: gaps between hits of ball 0 shrink.
        let b0: Vec<f64> = ons.iter().filter(|o| o.1 == 60).map(|o| o.0).collect();
        assert!(b0.len() >= 3);
        assert!(b0[2] - b0[1] < b0[1] - b0[0] + 1e-6);
        let p = defaults(DRUNK);
        let ons = run(MidiFxKind::Drunk, &p, &[], 8.0);
        assert!(ons.len() > 10);
        assert!(ons.windows(2).all(|w| (w[0].1 as i32 - w[1].1 as i32).abs() <= 4));
    }

    #[test]
    fn silent_when_stopped() {
        let mut st = MidiFxState::new(MidiFxKind::Euclid);
        let ctx = MidiCtx { now: 0, len: 256, samples_per_beat: 24000.0, scale: Scale::default(), scale_on: true, beat: 0.0, playing: false };
        let mut out = Vec::new();
        st.process(MidiFxKind::Euclid, &defaults(EUCLID), &[], &[], &mut out, &ctx);
        assert!(out.is_empty());
    }
}
