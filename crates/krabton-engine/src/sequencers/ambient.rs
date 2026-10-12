//! Sequencers for drones and ambient music: slow, overlapping, never quite
//! repeating.
//!
//! - Loops: up to seven notes, each on its own long loop, as in Brian Eno's
//!   tape loops for Music for Airports: they drift in and out of phase and
//!   meet in ever new chords.
//! - Bloom: after Brian Eno and Peter Chilvers' Bloom: notes you drop on a
//!   pond echo in time, looping or fading away, sometimes wandering a step;
//!   left alone it plays by itself around your notes.
//! - Chimes: wind chimes, simulated: the wind pushes a sail, the clapper
//!   swings and a note sounds whenever it meets a tube (see `chime_sim`).
//! - Drift Chords: chords that change slowly, one voice moving a step at a
//!   time, the others held.
//! - Tintinnabuli: a melody moving by steps and a second voice on the
//!   nearest note of the tonic triad, after Arvo Pärt, over a held tonic.
//! - Pedal: a held drone note that now and then moves to the fifth, the
//!   fourth or the octave, struck again for instruments that fade.

use super::{SeqState, RATES};
use crate::dsp::chime_sim::{lengths_for, ChimeSim, Shape, MAX_TUBES};
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

const KEYS: [ParamSpec; 2] = [ParamSpec::toggle("Transpose", "Keys", false), ParamSpec::toggle("Hold", "Keys", false)];

pub static LOOPS: &[ParamSpec] = &[
    ParamSpec::int("Voices", "Loops", 1.0, 7.0, 5.0, Unit::None),
    ParamSpec::lin("Speed", "Loops", 0.25, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Length", "Loops", 0.5, 16.0, 6.0, Unit::None),
    ParamSpec::int("Base Note", "Loops", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Velocity", "Loops", 0.05, 1.0, 0.55, Unit::Percent),
    ParamSpec::lin("Humanize", "Loops", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::int("Loop 1", "Loop Lengths", 3.0, 64.0, 17.0, Unit::None),
    ParamSpec::int("Loop 2", "Loop Lengths", 3.0, 64.0, 21.0, Unit::None),
    ParamSpec::int("Loop 3", "Loop Lengths", 3.0, 64.0, 25.0, Unit::None),
    ParamSpec::int("Loop 4", "Loop Lengths", 3.0, 64.0, 29.0, Unit::None),
    ParamSpec::int("Loop 5", "Loop Lengths", 3.0, 64.0, 34.0, Unit::None),
    ParamSpec::int("Loop 6", "Loop Lengths", 3.0, 64.0, 39.0, Unit::None),
    ParamSpec::int("Loop 7", "Loop Lengths", 3.0, 64.0, 43.0, Unit::None),
    ParamSpec::int("Note 1", "Notes", -14.0, 14.0, 0.0, Unit::None),
    ParamSpec::int("Note 2", "Notes", -14.0, 14.0, 4.0, Unit::None),
    ParamSpec::int("Note 3", "Notes", -14.0, 14.0, 2.0, Unit::None),
    ParamSpec::int("Note 4", "Notes", -14.0, 14.0, 7.0, Unit::None),
    ParamSpec::int("Note 5", "Notes", -14.0, 14.0, 9.0, Unit::None),
    ParamSpec::int("Note 6", "Notes", -14.0, 14.0, -3.0, Unit::None),
    ParamSpec::int("Note 7", "Notes", -14.0, 14.0, 11.0, Unit::None),
    KEYS[0],
    KEYS[1],
];

pub const ECHOES: &[&str] = &["1/2 Bar", "1 Bar", "1.5 Bars", "2 Bars", "3 Bars"];
const ECHO_BEATS: [f64; 5] = [2.0, 4.0, 6.0, 8.0, 12.0];

/// Bloom's taps in the device data: 6 bytes each, a sequence number (u16,
/// little endian), the height (0 = bottom, 255 = top: the pitch), the place
/// across (for the picture) and where in the echo loop it fell (u16, a
/// fraction of the loop). At most `MAX_TAPS` are kept.
pub const TAP_BYTES: usize = 6;
pub const MAX_TAPS: usize = 16;
/// Bloom's Taps param: fade away, or loop for as long as they are there.
pub const TAPS: usize = 9;
pub const TAP_MODES: &[&str] = &["Fade Away", "Loop"];

/// Bloom leaves the playing to you for this long after a tap (beats).
const BLOOM_IDLE: f64 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tap {
    pub seq: u16,
    pub height: u8,
    pub across: u8,
    pub phase: u16,
}

/// The taps in `data`, oldest first.
pub fn bloom_taps(data: &[u8]) -> impl Iterator<Item = Tap> + '_ {
    data.chunks_exact(TAP_BYTES).map(|c| Tap { seq: u16::from_le_bytes([c[0], c[1]]), height: c[2], across: c[3], phase: u16::from_le_bytes([c[4], c[5]]) })
}

/// Adds a tap (keeping the newest `MAX_TAPS`).
pub fn bloom_add_tap(data: &mut Vec<u8>, height: u8, across: u8, phase: u16) {
    data.truncate(data.len() / TAP_BYTES * TAP_BYTES);
    let seq = bloom_taps(data).last().map(|t| t.seq).unwrap_or(0).wrapping_add(1).to_le_bytes();
    let ph = phase.to_le_bytes();
    data.extend_from_slice(&[seq[0], seq[1], height, across, ph[0], ph[1]]);
    let keep = MAX_TAPS * TAP_BYTES;
    if data.len() > keep {
        data.drain(..data.len() - keep);
    }
}

/// Removes the `i`th tap.
pub fn bloom_remove_tap(data: &mut Vec<u8>, i: usize) {
    if (i + 1) * TAP_BYTES <= data.len() {
        data.drain(i * TAP_BYTES..(i + 1) * TAP_BYTES);
    }
}

/// The scale degree of a tap at `height` (0..255) in a range of `range` steps.
pub fn bloom_degree(height: u8, range: i32) -> i32 {
    ((height as f32 / 255.0) * range as f32).round() as i32
}

pub static BLOOM: &[ParamSpec] = &[
    ParamSpec::lin("By Itself", "Bloom", 0.0, 4.0, 0.75, Unit::None),
    ParamSpec::choice("Echo", "Bloom", ECHOES, 1),
    ParamSpec::int("Echoes", "Bloom", 1.0, 12.0, 6.0, Unit::None),
    ParamSpec::lin("Decay", "Bloom", 0.3, 0.95, 0.75, Unit::Percent),
    ParamSpec::lin("Drift", "Bloom", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::int("Range", "Notes", 2.0, 21.0, 14.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Length", "Notes", 0.25, 8.0, 1.5, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
    ParamSpec::choice("Taps", "Bloom", TAP_MODES, 1),
    KEYS[0],
    KEYS[1],
];

pub static CHIMES: &[ParamSpec] = &[
    ParamSpec::lin("Wind", "Wind", 0.0, 1.0, 0.45, Unit::Percent),
    ParamSpec::lin("Gusts", "Wind", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::int("Chimes", "Chimes", 3.0, 8.0, 6.0, Unit::None),
    ParamSpec::int("Octaves", "Chimes", 1.0, 3.0, 2.0, Unit::None),
    ParamSpec::int("Base Note", "Chimes", 24.0, 96.0, 72.0, Unit::None),
    ParamSpec::lin("Length", "Chimes", 0.5, 8.0, 3.0, Unit::None),
    ParamSpec::lin("Velocity", "Chimes", 0.05, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Spacing", "Build", 0.0, 1.0, 0.45, Unit::Percent),
    ParamSpec::lin("Clapper", "Build", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Sail", "Build", 0.0, 1.0, 0.5, Unit::Percent),
    KEYS[0],
    KEYS[1],
];

/// The Chimes sequencer's tubes: how many, and the note of each, from a
/// pentatonic set of the scale's steps (1, 2, 3, 5, 6) over the octaves.
pub fn chime_notes(scale: &crate::scale::Scale, p: &[f32]) -> (usize, [i32; MAX_TUBES]) {
    let count = (p[2].round() as usize).clamp(3, MAX_TUBES);
    let octaves = p[3].round().clamp(1.0, 3.0) as i32;
    let set: Vec<i32> = (0..octaves).flat_map(|o| [0, 1, 2, 4, 5].map(|d| o * 7 + d)).collect();
    let (b, _) = scale.locate(p[4].round() as i32);
    let notes = std::array::from_fn(|j| {
        let j = j.min(count - 1);
        scale.pitch_at(b + set[(j * (set.len() - 1)) / (count - 1).max(1)])
    });
    (count, notes)
}

/// The build the simulation runs for these params.
pub fn chime_shape(scale: &crate::scale::Scale, p: &[f32]) -> Shape {
    let (count, notes) = chime_notes(scale, p);
    let f: Vec<f32> = notes[..count].iter().map(|&n| crate::dsp::midi_to_freq(n as f32)).collect();
    let g = |i: usize, d: f32| p.get(i).copied().unwrap_or(d);
    Shape { tubes: count, spacing: g(7, 0.45), clapper: g(8, 0.5), sail: g(9, 0.5), wind: p[0].clamp(0.0, 1.5), gusts: p[1].clamp(0.0, 1.0), lengths: lengths_for(&f) }
}

pub const CHANGES: &[&str] = &["1 Bar", "2 Bars", "4 Bars", "8 Bars"];
const CHANGE_BARS: [i64; 4] = [1, 2, 4, 8];

pub static DRIFT_CHORDS: &[ParamSpec] = &[
    ParamSpec::int("Voices", "Chords", 3.0, 6.0, 4.0, Unit::None),
    ParamSpec::choice("Change", "Chords", CHANGES, 1),
    ParamSpec::lin("Movement", "Chords", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Spread", "Chords", 1.0, 4.0, 2.0, Unit::None),
    ParamSpec::toggle("Legato", "Chords", true),
    ParamSpec::int("Base Note", "Notes", 24.0, 84.0, 48.0, Unit::None),
    ParamSpec::int("Range", "Notes", 7.0, 28.0, 14.0, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.5, Unit::Percent),
    KEYS[0],
    KEYS[1],
];

pub const POSITIONS: &[&str] = &["1st Above", "1st Below", "Alternating", "2nd Above", "2nd Below"];

pub static TINTINNABULI: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Melody", RATES, 5),
    ParamSpec::int("Range", "Melody", 3.0, 14.0, 7.0, Unit::None),
    ParamSpec::lin("Steps", "Melody", 0.0, 1.0, 0.85, Unit::Percent),
    ParamSpec::choice("Position", "Triad", POSITIONS, 0),
    ParamSpec::toggle("Drone", "Triad", true),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Length", "Notes", 0.1, 1.5, 0.95, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.6, Unit::Percent),
    KEYS[0],
    KEYS[1],
];

pub const PEDAL_MOVES: &[&str] = &["Fifth", "Fourth", "Octave", "Anywhere"];
pub const RETRIGGERS: &[&str] = &["Never", "1 Bar", "2 Bars", "4 Bars"];

pub static PEDAL: &[ParamSpec] = &[
    ParamSpec::choice("Change", "Pedal", &["1 Bar", "2 Bars", "4 Bars", "8 Bars", "Never"], 2),
    ParamSpec::lin("Moves", "Pedal", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::choice("Moves To", "Pedal", PEDAL_MOVES, 0),
    ParamSpec::toggle("Octave", "Pedal", true),
    ParamSpec::toggle("Fifth", "Pedal", false),
    ParamSpec::choice("Strike Again", "Pedal", RETRIGGERS, 0),
    ParamSpec::int("Base Note", "Notes", 12.0, 72.0, 36.0, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
    KEYS[0],
    KEYS[1],
];

const SEEDS: usize = 32;

/// Bloom's echoing notes: degree, next echo (song beat), echoes left,
/// velocity, and where across the pond it was dropped.
#[derive(Clone, Copy, Default)]
struct Seed {
    active: bool,
    degree: i32,
    next: f64,
    left: u8,
    vel: f32,
    across: u8,
    /// The tap it came from (None when Bloom played it by itself).
    tap: Option<u16>,
}

pub struct Ambient {
    loop_last: [f64; 7],
    seeds: [Seed; SEEDS],
    seed_next: f64,
    pub(crate) recent: u64,
    recent_across: u64,
    chime_sim: Box<ChimeSim>,
    /// How strongly each tube rings now, for the screen.
    chime_ring: [f32; MAX_TUBES],
    pub(crate) chord: [i32; 6],
    chord_n: usize,
    chord_bar: i64,
    tin_m: i32,
    tin_dir: i32,
    tin_alt: bool,
    tin_drone: i64,
    pedal_root: i32,
    pedal_bar: i64,
    /// The newest Bloom tap already played (None until the data is seen).
    tap_seen: Option<u16>,
    last_tap: f64,
    /// Loop mode as last seen, and whether the loop must be set up again
    /// from the taps (after a jump, a stop or a mode change).
    looping: bool,
    rebuild: bool,
    /// Stopped and not tapped since: quiet.
    asleep: bool,
    /// Bloom's own clock while the transport stands still (beats).
    free_beat: f64,
    rng: u32,
}

impl Default for Ambient {
    fn default() -> Self {
        Self {
            loop_last: [f64::NEG_INFINITY; 7],
            seeds: [Seed::default(); SEEDS],
            seed_next: -1.0,
            recent: 0,
            recent_across: 0,
            chime_sim: Box::default(),
            chime_ring: [0.0; MAX_TUBES],
            chord: [0; 6],
            chord_n: 0,
            chord_bar: i64::MIN,
            tin_m: 0,
            tin_dir: 1,
            tin_alt: false,
            tin_drone: i64::MIN,
            pedal_root: 0,
            pedal_bar: i64::MIN,
            tap_seen: None,
            last_tap: f64::NEG_INFINITY,
            looping: false,
            rebuild: true,
            asleep: true,
            free_beat: 0.0,
            rng: 0xA4B1_E417,
        }
    }
}

impl Ambient {
    /// The song jumped: start the clocks over (the music itself carries on).
    pub fn restart(&mut self) {
        self.loop_last = [f64::NEG_INFINITY; 7];
        for s in self.seeds.iter_mut() {
            s.active = false;
        }
        self.seed_next = -1.0;
        self.chord_bar = i64::MIN;
        self.chord_n = 0;
        self.tin_drone = i64::MIN;
        self.pedal_bar = i64::MIN;
        self.last_tap = f64::NEG_INFINITY;
        self.rebuild = true;
    }

    fn unit(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / 16_777_216.0
    }
}

/// The frame of song beat `t` (later blocks are fine: `play` waits).
fn frame(ctx: &MidiCtx, t: f64) -> u64 {
    ctx.now + ((t - ctx.beat).max(0.0) * ctx.samples_per_beat) as u64
}

/// The bar boundary in this block (song beats), if a new bar starts in it;
/// right away when nothing has played yet (`last` is i64::MIN).
fn bar_in_block(ctx: &MidiCtx, last: &mut i64) -> Option<(i64, f64)> {
    let b1 = ctx.beat + ctx.len as f64 / ctx.samples_per_beat;
    if *last == i64::MIN {
        let k = (ctx.beat / 4.0 + 1e-9).floor() as i64;
        *last = k;
        return Some((k, ctx.beat));
    }
    let k = (b1 / 4.0 - 1e-9).floor() as i64;
    if k > *last && k as f64 * 4.0 >= ctx.beat - 1e-9 {
        *last = k;
        return Some((k, k as f64 * 4.0));
    }
    None
}

impl SeqState {
    fn degree(ctx: &MidiCtx, base: f32, degree: i32) -> i32 {
        let (b, _) = ctx.scale.locate(base.round() as i32);
        ctx.scale.pitch_at(b + degree)
    }

    /// Plays `pitch` from `at` for `len` frames, or, if it is already
    /// sounding, keeps it sounding that long (a held note, no new attack).
    fn hold(&mut self, ctx: &MidiCtx, at: u64, pitch: i32, vel: f32, len: f64, out: &mut Vec<MidiEvent>) {
        let sounding = if self.shift_in_note { self.shifted(ctx, pitch) } else { pitch }.clamp(0, 127) as u8;
        let end = at + len.max(1.0) as u64;
        if let Some(o) = self.offs.iter_mut().find(|o| o.1 == sounding && o.0 + 2 >= at) {
            o.0 = o.0.max(end);
            return;
        }
        self.play(ctx, at, pitch, vel, len, out);
    }

    pub(crate) fn loops(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let n = (p[0].round() as usize).clamp(1, 7);
        let speed = p[1].clamp(0.1, 8.0) as f64;
        let (b0, b1) = (ctx.beat, ctx.beat + ctx.len as f64 / ctx.samples_per_beat);
        let len = p[2].max(0.1) as f64 * ctx.samples_per_beat;
        let mut vis = 0u64;
        for i in 0..n {
            let period = p[6 + i].round().max(1.0) as f64 / speed;
            // Every loop starts somewhere else on its tape.
            let offset = ((i as f64 + 1.0) * 0.618_034).fract() * period;
            let mut k = ((b0 - offset) / period - 1e-9).ceil();
            while offset + k * period < b1 {
                let t = offset + k * period;
                if t > self.ambient.loop_last[i] + 1e-6 {
                    self.ambient.loop_last[i] = t;
                    let human = p[5].clamp(0.0, 1.0);
                    let vel = p[4] * (1.0 - human * 0.5 * self.ambient.unit());
                    let late = (human * 0.04 * self.ambient.unit()) as f64;
                    let pitch = Self::degree(ctx, p[3], p[13 + i].round() as i32);
                    self.play(ctx, frame(ctx, t + late), pitch, vel, len, out);
                }
                k += 1.0;
            }
            let phase = ((b1 - offset) / period).rem_euclid(1.0);
            vis |= ((phase * 255.0) as u64 & 0xff) << (i * 8);
        }
        self.visual[0] = vis;
        self.visual[1] = n as u64;
        self.visual[2] = (0..n).fold(0u64, |acc, i| acc | ((p[6 + i].round().clamp(0.0, 255.0) as u64) << (i * 8)));
    }

    /// Bloom while the transport stands still, on its own clock. Stopping
    /// silences it; a tap wakes it (and in Loop, the whole loop with it).
    /// `fresh` is true on the first stopped block.
    pub(crate) fn bloom_stopped(&mut self, p: &[f32], data: &[u8], out: &mut Vec<MidiEvent>, ctx: &MidiCtx, fresh: bool) {
        if fresh {
            self.ambient.restart();
            self.ambient.asleep = true;
        }
        let free = MidiCtx { beat: self.ambient.free_beat, ..*ctx };
        self.ambient.free_beat += ctx.len as f64 / ctx.samples_per_beat;
        self.bloom(p, data, out, &free, false);
    }

    /// Puts `seed` in a free place, or in place of the faintest one.
    fn sow(&mut self, seed: Seed) {
        let a = &mut self.ambient;
        let i = a.seeds.iter().position(|s| !s.active).unwrap_or_else(|| (0..SEEDS).min_by(|&x, &y| a.seeds[x].vel.total_cmp(&a.seeds[y].vel)).unwrap_or(0));
        a.seeds[i] = seed;
    }

    pub(crate) fn bloom(&mut self, p: &[f32], data: &[u8], out: &mut Vec<MidiEvent>, ctx: &MidiCtx, playing: bool) {
        let (b0, b1) = (ctx.beat, ctx.beat + ctx.len as f64 / ctx.samples_per_beat);
        let echo = ECHO_BEATS[(p[1].round().max(0.0) as usize).min(ECHO_BEATS.len() - 1)];
        let range = p[5].round().max(2.0) as i32;
        let len = p[7].max(0.1) as f64 * ctx.samples_per_beat;
        let looping = p.get(TAPS).is_some_and(|v| *v >= 0.5);
        let echoes = p[2].round().clamp(1.0, 12.0) as u8;
        if playing {
            self.ambient.asleep = false;
        }
        if looping != self.ambient.looping {
            // Into Loop: every tap comes back; out of it: they fade from here.
            self.ambient.looping = looping;
            self.ambient.rebuild = looping;
            for s in self.ambient.seeds.iter_mut().filter(|s| s.tap.is_some()) {
                s.left = echoes;
            }
        }
        // Your taps: each sounds now and echoes on. Taps already in the data
        // when Bloom first looks (a loaded set) are not replayed, except as
        // the loop.
        let newest = bloom_taps(data).map(|t| t.seq).reduce(|a, b| if (b.wrapping_sub(a) as i16) > 0 { b } else { a });
        match (self.ambient.tap_seen, newest) {
            (None, n) => self.ambient.tap_seen = Some(n.unwrap_or(0)),
            (Some(seen), Some(_)) => {
                for tap in bloom_taps(data) {
                    if (tap.seq.wrapping_sub(seen) as i16) <= 0 {
                        continue;
                    }
                    self.sow(Seed { active: true, degree: bloom_degree(tap.height, range), next: b0, left: echoes, vel: p[8], across: tap.across, tap: Some(tap.seq) });
                    self.ambient.last_tap = b0;
                    if self.ambient.asleep {
                        self.ambient.asleep = false;
                        self.ambient.rebuild = true;
                    }
                }
                self.ambient.tap_seen = newest;
            }
            _ => {}
        }
        if looping {
            // Taps clicked away stop; the others keep their place in the loop.
            for s in self.ambient.seeds.iter_mut() {
                if let Some(seq) = s.tap {
                    if s.active && !bloom_taps(data).any(|t| t.seq == seq) {
                        s.active = false;
                    }
                }
            }
            if self.ambient.rebuild && !self.ambient.asleep {
                self.ambient.rebuild = false;
                for tap in bloom_taps(data) {
                    if self.ambient.seeds.iter().any(|s| s.active && s.tap == Some(tap.seq)) {
                        continue;
                    }
                    let mut next = ((b0 / echo).floor() + tap.phase as f64 / 65536.0) * echo;
                    if next < b0 - 1e-9 {
                        next += echo;
                    }
                    self.sow(Seed { active: true, degree: bloom_degree(tap.height, range), next, left: echoes, vel: p[8], across: tap.across, tap: Some(tap.seq) });
                }
            }
        }
        // Left alone, it blooms by itself: By Itself notes a bar on average,
        // on and around the notes you dropped (anywhere, before you have).
        let per_beat = p[0].max(0.0) as f64 / 4.0;
        if per_beat <= 1e-4 || !playing || b0 - self.ambient.last_tap < BLOOM_IDLE {
            self.ambient.seed_next = -1.0;
        } else {
            if self.ambient.seed_next < 0.0 {
                self.ambient.seed_next = b0 + (-(self.ambient.unit().max(1e-4) as f64).ln()) / per_beat;
            }
            let yours: Vec<i32> = bloom_taps(data).map(|t| bloom_degree(t.height, range)).collect();
            while self.ambient.seed_next < b1 {
                let t = self.ambient.seed_next;
                self.ambient.seed_next += (-(self.ambient.unit().max(1e-4) as f64).ln()) / per_beat;
                let last = (self.ambient.recent & 0xff) as i32 - 64;
                let mut d = if yours.is_empty() {
                    (self.ambient.unit() * range as f32) as i32
                } else {
                    let pick = yours[(self.ambient.unit() * yours.len() as f32) as usize % yours.len()];
                    let r = self.ambient.unit();
                    pick + if r < 0.5 { 0 } else if r < 0.65 { 2 } else if r < 0.8 { -2 } else if r < 0.9 { 4 } else { 7 }
                };
                // Not the note just played.
                if d == last {
                    d = (d + 2) % range.max(1);
                }
                let across = (self.ambient.unit() * 255.0) as u8;
                if let Some(s) = self.ambient.seeds.iter_mut().find(|s| !s.active) {
                    *s = Seed { active: true, degree: d, next: t, left: echoes, vel: p[8], across, tap: None };
                }
            }
        }
        for i in 0..SEEDS {
            while self.ambient.seeds[i].active && self.ambient.seeds[i].next < b1 {
                let s = self.ambient.seeds[i];
                // Now and then an echo wanders a step, or an octave up.
                let mut d = s.degree;
                if self.ambient.unit() < p[4] * 0.5 {
                    d += if self.ambient.unit() < 0.3 { 7 } else if self.ambient.unit() < 0.5 { 1 } else { -1 };
                }
                let loop_tap = looping && s.tap.is_some();
                // A looping tap keeps its level, with a little life in it.
                let vel = if loop_tap { p[8] * (0.85 + 0.15 * self.ambient.unit()) } else { s.vel };
                let pitch = Self::degree(ctx, p[6], d);
                self.play(ctx, frame(ctx, s.next), pitch, vel, len, out);
                self.ambient.recent = (self.ambient.recent << 8) | ((d + 64) as u64 & 0xff);
                self.ambient.recent_across = (self.ambient.recent_across << 8) | s.across as u64;
                let st = &mut self.ambient.seeds[i];
                st.next += echo;
                if !loop_tap {
                    st.vel *= p[3].clamp(0.1, 0.99);
                    st.left = st.left.saturating_sub(1);
                    if st.left == 0 || st.vel < 0.02 {
                        st.active = false;
                    }
                }
            }
        }
        self.visual[0] = self.ambient.recent;
        self.visual[1] = self.ambient.seeds.iter().filter(|s| s.active).count() as u64 | (range as u64) << 8;
        self.visual[2] = self.ambient.recent_across;
        // Where the loop is now, for the editor to stamp taps with.
        self.visual[3] = ((b1 / echo).rem_euclid(1.0) * 65536.0) as u64 & 0xffff;
    }

    pub(crate) fn chimes(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let shape = chime_shape(&ctx.scale, p);
        let (_, notes) = chime_notes(&ctx.scale, p);
        let len = p[5].max(0.1) as f64 * ctx.samples_per_beat;
        // The wind blows for this block's time; every meeting of clapper and
        // tube is a note, as loud as they met.
        let secs = ctx.len as f32 / ctx.sr.max(1.0);
        let mut hits: Vec<(usize, f32, f32)> = Vec::new();
        self.ambient.chime_sim.run(&shape, secs, true, &mut |i, speed, at| hits.push((i, speed, at)));
        for (i, speed, at) in hits {
            let vel = (speed / 2.0).sqrt().min(1.0);
            let when = ctx.now + (at * ctx.sr) as u64;
            self.play(ctx, when, notes[i], (p[6] * (0.25 + 0.75 * vel)).clamp(0.02, 1.0), len, out);
            self.ambient.chime_ring[i] = self.ambient.chime_ring[i].max(vel);
        }
        // The rings fade over the notes' length.
        let fade = (-(ctx.len as f64) / (len / 3.0).max(1.0)).exp() as f32;
        for r in self.ambient.chime_ring.iter_mut() {
            *r *= fade;
        }
        let sim = &self.ambient.chime_sim;
        let i16b = |v: f32| ((v * 10000.0).clamp(-32767.0, 32767.0) as i16) as u16 as u64;
        let i8b = |v: f32| ((v * 200.0).clamp(-127.0, 127.0) as i8) as u8 as u64;
        let dir = (sim.dir.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * 255.0) as u64;
        self.visual[0] = i16b(sim.clapper[0]) | i16b(sim.clapper[1]) << 16 | ((sim.wind / 1.5 * 255.0).clamp(0.0, 255.0) as u64) << 32 | dir << 40 | (shape.tubes as u64) << 48;
        self.visual[1] = (0..MAX_TUBES).fold(0, |acc, i| acc | i8b(sim.tube[i][0]) << (8 * i));
        self.visual[2] = (0..MAX_TUBES).fold(0, |acc, i| acc | i8b(sim.tube[i][1]) << (8 * i));
        self.visual[3] = (0..MAX_TUBES).fold(0, |acc, i| acc | ((self.ambient.chime_ring[i] * 255.0).clamp(0.0, 255.0) as u64) << (8 * i));
    }

    pub(crate) fn drift_chords(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let n = (p[0].round() as usize).clamp(3, 6);
        let every = CHANGE_BARS[(p[1].round().max(0.0) as usize).min(CHANGE_BARS.len() - 1)];
        let range = p[6].round().max(7.0) as i32;
        let mut bar = self.ambient.chord_bar;
        if let Some((k, t)) = bar_in_block(ctx, &mut bar) {
            self.ambient.chord_bar = bar;
            let at = frame(ctx, t);
            let bar_len = 4.0 * ctx.samples_per_beat;
            let change = k.rem_euclid(every) == 0 || self.ambient.chord_n != n;
            let mut moved = [false; 6];
            if self.ambient.chord_n != n {
                // A new chord, spread in steps of Spread.
                let spread = p[3].round().clamp(1.0, 4.0) as i32;
                for i in 0..n {
                    self.ambient.chord[i] = (i as i32 * spread).min(range);
                    moved[i] = true;
                }
                self.ambient.chord_n = n;
            } else if change {
                // Each voice may move a step, never onto another voice.
                for i in 0..n {
                    if self.ambient.unit() < p[2] {
                        let dir = if self.ambient.unit() < 0.5 { 1 } else { -1 };
                        let to = self.ambient.chord[i] + dir;
                        if (0..=range).contains(&to) && !self.ambient.chord[..n].contains(&to) {
                            self.ambient.chord[i] = to;
                            moved[i] = true;
                        }
                    }
                }
            }
            for i in 0..n {
                let pitch = Self::degree(ctx, p[5], self.ambient.chord[i]);
                let vel = p[7] * (0.85 + 0.15 * self.ambient.unit());
                if moved[i] || (change && p[4] < 0.5) {
                    self.play(ctx, at, pitch, vel, bar_len + 8.0, out);
                } else {
                    self.hold(ctx, at, pitch, vel, bar_len + 8.0, out);
                }
            }
        }
        self.visual[0] = (0..6).fold(0u64, |acc, i| acc | (((self.ambient.chord[i] + 64) as u64 & 0xff) << (i * 8)));
        self.visual[1] = self.ambient.chord_n as u64 | (range as u64) << 8;
    }

    /// The tonic triad's note nearest to degree `m`, `nth` (1 or 2) above or below.
    fn triad_note(m: i32, above: bool, nth: usize) -> i32 {
        let is_triad = |d: i32| matches!(d.rem_euclid(7), 0 | 2 | 4);
        let mut d = m;
        let mut found = 0;
        loop {
            d += if above { 1 } else { -1 };
            if is_triad(d) {
                found += 1;
                if found == nth {
                    return d;
                }
            }
        }
    }

    pub(crate) fn tintinnabuli(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let range = p[1].round().clamp(3.0, 14.0) as i32;
        // The melody: steps in one direction for a while, turning at the edges.
        let a = &mut self.ambient;
        // Mostly a step on, now and then the same note again.
        if a.unit() < p[2].clamp(0.0, 1.0) {
            if a.tin_m + a.tin_dir > range || a.tin_m + a.tin_dir < -range / 2 {
                a.tin_dir = -a.tin_dir;
            }
            a.tin_m += a.tin_dir;
        }
        if a.unit() < 0.2 {
            a.tin_dir = -a.tin_dir;
        }
        let m = a.tin_m;
        let pos = (p[3].round().max(0.0) as usize).min(POSITIONS.len() - 1);
        let t = match pos {
            0 => Self::triad_note(m, true, 1),
            1 => Self::triad_note(m, false, 1),
            2 => {
                a.tin_alt = !a.tin_alt;
                Self::triad_note(m, a.tin_alt, 1)
            }
            3 => Self::triad_note(m, true, 2),
            _ => Self::triad_note(m, false, 2),
        };
        let len = step_frames * p[6].clamp(0.05, 2.0) as f64;
        let vel = p[7];
        let (mp, tp) = (Self::degree(ctx, p[5], m), Self::degree(ctx, p[5], t));
        self.note(ctx, at, mp, vel, len, out);
        self.note(ctx, at, tp, vel * 0.85, len, out);
        // The tonic below, held (struck again every four bars).
        if p[4] >= 0.5 {
            let beat = k as f64 * super::rate_beats(p[0]);
            let bar = (beat / 16.0).floor() as i64;
            let pitch = Self::degree(ctx, p[5], -7);
            if bar != self.ambient.tin_drone {
                self.ambient.tin_drone = bar;
                self.note(ctx, at, pitch, vel * 0.8, 16.0 * ctx.samples_per_beat + 8.0, out);
            }
        }
        self.visual[0] = ((m + 64) as u64 & 0xff) | (((t + 64) as u64 & 0xff) << 8) | ((p[4] >= 0.5) as u64) << 16;
        self.visual[1] = k as u64;
    }

    pub(crate) fn pedal(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let change = [1i64, 2, 4, 8, 0][(p[0].round().max(0.0) as usize).min(4)];
        let again = [0i64, 1, 2, 4][(p[5].round().max(0.0) as usize).min(3)];
        let mut bar = self.ambient.pedal_bar;
        if let Some((k, t)) = bar_in_block(ctx, &mut bar) {
            let first = self.ambient.pedal_bar == i64::MIN;
            self.ambient.pedal_bar = bar;
            let mut strike = first || (again > 0 && k.rem_euclid(again) == 0);
            if !first && change > 0 && k.rem_euclid(change) == 0 && self.ambient.unit() < p[1] {
                let to = match (p[2].round().max(0.0) as usize).min(3) {
                    0 => 4,
                    1 => 3,
                    2 => 7,
                    _ => [4, 3, 7, -3, 5, -7][(self.ambient.unit() * 6.0) as usize % 6],
                };
                // Away from the tonic, or back to it.
                self.ambient.pedal_root = if self.ambient.pedal_root == 0 { to } else { 0 };
                strike = true;
            }
            let at = frame(ctx, t);
            let len = 4.0 * ctx.samples_per_beat + 8.0;
            let root = self.ambient.pedal_root;
            let notes = [Some(root), (p[3] >= 0.5).then_some(root + 7), (p[4] >= 0.5).then_some(root + 4)];
            for d in notes.into_iter().flatten() {
                let pitch = Self::degree(ctx, p[6], d);
                if strike {
                    self.play(ctx, at, pitch, p[7], len, out);
                } else {
                    self.hold(ctx, at, pitch, p[7], len, out);
                }
            }
        }
        self.visual[0] = ((self.ambient.pedal_root + 64) as u64 & 0xff) | ((p[3] >= 0.5) as u64) << 8 | ((p[4] >= 0.5) as u64) << 9;
        self.visual[1] = self.ambient.pedal_bar.max(0) as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::{MidiFxKind, MidiFxState};
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};

    /// Runs `beats` beats at 120 BPM and returns (on beat, pitch, off beat).
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

    #[test]
    fn loops_drift_apart() {
        let p = defaults(LOOPS);
        let n = notes(MidiFxKind::Loops, &p, 128.0);
        // Each loop repeats at its own length: the root (C) every 17 beats.
        let c: Vec<f64> = n.iter().filter(|x| x.1 == 60).map(|x| x.0).collect();
        assert!(c.len() >= 6, "{c:?}");
        assert!(c.windows(2).all(|w| (w[1] - w[0] - 17.0).abs() < 0.1), "{c:?}");
        // Long notes: six beats.
        assert!(n.iter().filter(|x| x.2.is_finite()).all(|x| (x.2 - x.0 - 6.0).abs() < 0.1));
    }

    #[test]
    fn bloom_plays_taps_and_waits_before_playing_by_itself() {
        let spb = 24000.0;
        let mut p = defaults(BLOOM);
        p[4] = 0.0;
        let mut st = MidiFxState::new(MidiFxKind::Bloom);
        let mut data: Vec<u8> = Vec::new();
        let mut ons: Vec<(f64, u8)> = Vec::new();
        for b in 0..(32.0 * spb / 256.0) as u64 {
            let now = b * 256;
            let beat = now as f64 / spb;
            // A tap at the top after one beat.
            if b == (spb / 256.0) as u64 {
                data.extend_from_slice(&[1, 0, 255, 128, 0, 0]);
            }
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat, playing: true, sr: 48000.0, drum_notes: 0 };
            let mut out = Vec::new();
            st.process(MidiFxKind::Bloom, &p, &data, &[], &mut out, &ctx);
            ons.extend(out.iter().filter(|e| e.on).map(|e| ((now + e.offset as u64) as f64 / spb, e.pitch)));
        }
        // C4 plus a range of 14 steps: the top is C6, played at once and a bar later.
        let top: Vec<f64> = ons.iter().filter(|o| o.1 == 84).map(|o| o.0).collect();
        assert!(top.len() >= 3 && (top[0] - 1.0).abs() < 0.02 && (top[1] - top[0] - 4.0).abs() < 0.02, "{top:?}");
        // Nothing of its own for two bars after the tap.
        assert!(ons.iter().all(|o| o.1 == 84 || o.0 > 9.0 || o.0 < 1.0), "{ons:?}");

        // Stopped: a tap still sounds, on Bloom's own clock.
        let mut st = MidiFxState::new(MidiFxKind::Bloom);
        let mut data: Vec<u8> = Vec::new();
        let mut heard = 0;
        for b in 0..200u64 {
            if b == 10 {
                data.extend_from_slice(&[1, 0, 0, 0, 0, 0]);
            }
            let ctx = MidiCtx { now: b * 256, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: 0.0, playing: false, sr: 48000.0, drum_notes: 0 };
            let mut out = Vec::new();
            st.process(MidiFxKind::Bloom, &p, &data, &[], &mut out, &ctx);
            heard += out.iter().filter(|e| e.on && e.pitch == 60).count();
        }
        assert_eq!(heard, 1);
    }

    /// Runs Bloom from `from` to `to` (beats, playing), letting `edit` change
    /// the data at given blocks; returns the note-ons (beat, pitch).
    fn bloom_run(st: &mut MidiFxState, p: &[f32], data: &mut Vec<u8>, from: f64, to: f64, edit: &mut dyn FnMut(f64, &mut Vec<u8>)) -> Vec<(f64, u8)> {
        let spb = 24000.0;
        let mut ons = Vec::new();
        let mut beat = from;
        while beat < to {
            edit(beat, data);
            let now = (beat * spb) as u64;
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat, playing: true, sr: 48000.0, drum_notes: 0 };
            let mut out = Vec::new();
            st.process(MidiFxKind::Bloom, p, data, &[], &mut out, &ctx);
            ons.extend(out.iter().filter(|e| e.on).map(|e| (beat + e.offset as f64 / spb, e.pitch)));
            beat += 256.0 / spb;
        }
        ons
    }

    #[test]
    fn bloom_loops_taps_until_they_are_clicked_away() {
        let mut p = defaults(BLOOM);
        p[0] = 0.0;
        p[4] = 0.0;
        let mut st = MidiFxState::new(MidiFxKind::Bloom);
        let mut data = Vec::new();
        // A tap at the top, a quarter into the bar.
        let ons = bloom_run(&mut st, &p, &mut data, 0.0, 60.0, &mut |b, d| {
            if (b - 1.0).abs() < 0.005 {
                bloom_add_tap(d, 255, 10, 16384);
            }
        });
        let top: Vec<f64> = ons.iter().filter(|o| o.1 == 84).map(|o| o.0).collect();
        // Far more than the six echoes, a bar apart.
        assert!(top.len() >= 14, "{top:?}");
        assert!(top.windows(2).all(|w| (w[1] - w[0] - 4.0).abs() < 0.02), "{top:?}");
        // The song jumps: the loop carries on in its place in the bar.
        let ons = bloom_run(&mut st, &p, &mut data, 100.0, 110.0, &mut |_, _| {});
        let top: Vec<f64> = ons.iter().filter(|o| o.1 == 84).map(|o| o.0).collect();
        assert!(!top.is_empty() && top.iter().all(|t| (t.rem_euclid(4.0) - 1.0).abs() < 0.02), "{top:?}");
        // Clicked away: silence.
        bloom_remove_tap(&mut data, 0);
        let ons = bloom_run(&mut st, &p, &mut data, 110.0, 130.0, &mut |_, _| {});
        assert!(ons.iter().all(|o| o.1 != 84), "{ons:?}");
        // Fade Away: the six echoes and then no more.
        p[TAPS] = 0.0;
        let mut st = MidiFxState::new(MidiFxKind::Bloom);
        let mut data = Vec::new();
        let ons = bloom_run(&mut st, &p, &mut data, 0.0, 60.0, &mut |b, d| {
            if (b - 1.0).abs() < 0.005 {
                bloom_add_tap(d, 255, 10, 16384);
            }
        });
        assert_eq!(ons.iter().filter(|o| o.1 == 84).count(), 6);
    }

    #[test]
    fn bloom_echoes_fade() {
        let mut p = defaults(BLOOM);
        p[4] = 0.0;
        let n = notes(MidiFxKind::Bloom, &p, 96.0);
        assert!(n.len() > 8, "{}", n.len());
        // Echoes of one note come a bar apart.
        let first = n[0];
        let echoes: Vec<f64> = n.iter().filter(|x| x.1 == first.1).map(|x| x.0 - first.0).collect();
        assert!(echoes.iter().any(|d| (d - 4.0).abs() < 0.01), "{echoes:?}");
    }

    #[test]
    fn chimes_follow_the_wind() {
        let mut p = defaults(CHIMES);
        p[0] = 0.0;
        assert!(notes(MidiFxKind::Chimes, &p, 32.0).is_empty(), "no wind, no chimes");
        p[0] = 0.8;
        let n = notes(MidiFxKind::Chimes, &p, 32.0);
        assert!(n.len() > 5, "{}", n.len());
        // In key: C major pentatonic-ish steps from C5.
        assert!(n.iter().all(|x| [0, 2, 4, 7, 9].contains(&((x.1 as i32 - 72).rem_euclid(12)))));
    }

    #[test]
    fn drift_chords_move_one_step_at_a_time() {
        let mut p = defaults(DRIFT_CHORDS);
        p[1] = 0.0; // a change every bar
        p[2] = 1.0;
        let n = notes(MidiFxKind::DriftChords, &p, 32.0);
        // Four voices to begin with, together.
        assert_eq!(n.iter().filter(|x| x.0 < 0.01).count(), 4);
        // Held voices are not struck again every bar.
        let per_bar = |bar: f64| n.iter().filter(|x| (x.0 - bar * 4.0).abs() < 0.01).count();
        assert!((1..8).map(|b| per_bar(b as f64)).all(|c| c <= 4));
        // Notes hang on for at least a bar.
        assert!(n.iter().filter(|x| x.2.is_finite()).all(|x| x.2 - x.0 > 3.9));
    }

    #[test]
    fn tintinnabuli_pairs_melody_and_triad() {
        let mut p = defaults(TINTINNABULI);
        p[4] = 0.0;
        let n = notes(MidiFxKind::Tintinnabuli, &p, 16.0);
        // Two notes per step: one of them always in the C major triad (C, E, G).
        let mut by_time: std::collections::BTreeMap<i64, Vec<u8>> = Default::default();
        for x in &n {
            by_time.entry((x.0 * 100.0).round() as i64).or_default().push(x.1);
        }
        assert!(by_time.len() >= 12);
        for v in by_time.values() {
            assert_eq!(v.len(), 2, "{v:?}");
            assert!(v.iter().any(|&q| [0, 4, 7].contains(&(q % 12))), "{v:?}");
        }
        // With the drone: a low C held.
        p[4] = 1.0;
        let n = notes(MidiFxKind::Tintinnabuli, &p, 4.0);
        assert!(n.iter().any(|x| x.1 == 48));
    }

    #[test]
    fn pedal_holds_on() {
        let mut p = defaults(PEDAL);
        p[1] = 0.0;
        let n = notes(MidiFxKind::Pedal, &p, 64.0);
        // C2 and C3 struck once at the start and held throughout.
        assert_eq!(n.len(), 2, "{n:?}");
        assert!(n.iter().all(|x| x.0 < 0.01 && x.2 > 60.0), "{n:?}");
        assert!(n.iter().any(|x| x.1 == 36) && n.iter().any(|x| x.1 == 48));
        // Struck again every bar.
        p[5] = 1.0;
        let n = notes(MidiFxKind::Pedal, &p, 16.0);
        assert!(n.iter().filter(|x| x.1 == 36).count() >= 4);
    }
}
