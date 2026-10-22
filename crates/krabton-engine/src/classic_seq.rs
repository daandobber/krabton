//! Deterministic sequencers: what you program is what plays.
//!
//! - Step: a classic 16-step sequencer with note, velocity, length and
//!   ratchets per step and several play directions.
//! - Acid: a TB-303 style bass line with accent, slide and tie per step.
//! - Drum Grid: an x0x drum machine grid, 8 lanes by 16 steps.
//! - Progression: a chord progression in the project key, with rhythms.

use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};
use crate::scale::Scale;
use crate::sequencers::{SeqState, RATES};

// ---------------------------------------------------------------- specs

pub const DIRECTIONS: &[&str] = &["Forward", "Backward", "Ping-Pong", "Random"];

pub static STEP: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::int("Steps", "Clock", 1.0, 16.0, 16.0, Unit::None),
    ParamSpec::choice("Direction", "Clock", DIRECTIONS, 0),
    ParamSpec::lin("Swing", "Clock", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 48.0, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.1, 1.5, 1.0, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];
pub static ACID: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::int("Steps", "Clock", 1.0, 16.0, 16.0, Unit::None),
    ParamSpec::lin("Swing", "Clock", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Base Note", "Notes", 24.0, 72.0, 36.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.55, Unit::Percent),
    ParamSpec::lin("Accent", "Notes", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::toggle("In Key", "Notes", true),
    // Index 7 was "Key Transpose" before every sequencer had Transpose and Hold.
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];
pub static DRUM_GRID: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::int("Steps", "Clock", 1.0, 16.0, 16.0, Unit::None),
    ParamSpec::lin("Swing", "Clock", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Velocity", "Hits", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Accent", "Hits", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];
pub const CHORD_LENGTHS: &[&str] = &["1 beat", "2 beats", "1 bar", "2 bars"];
const CHORD_BEATS: [f64; 4] = [1.0, 2.0, 4.0, 8.0];
pub const RHYTHMS: &[&str] = &["Hold", "Quarters", "Eighths", "Offbeats", "Stabs", "Charleston", "Arp Up", "Arp Down"];
pub static PROGRESSION: &[ParamSpec] = &[
    ParamSpec::choice("Length", "Chords", CHORD_LENGTHS, 2),
    ParamSpec::int("Chords", "Chords", 1.0, 8.0, 4.0, Unit::None),
    ParamSpec::choice("Rhythm", "Chords", RHYTHMS, 0),
    ParamSpec::int("Base Note", "Notes", 36.0, 72.0, 48.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.9, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::toggle("Band", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

// ---------------------------------------------------------------- Step data

/// Per step: on, scale degree above the base note, velocity, length in
/// quarter steps, ratchet count.
pub const STEP_BYTES: usize = 5;
pub const STEP_MAX_DEGREE: u8 = 21;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub on: bool,
    pub degree: u8,
    pub velocity: u8,
    pub length: u8,
    pub ratchet: u8,
}

pub fn step_default() -> Vec<u8> {
    let degrees = [0, 4, 2, 4, 7, 4, 2, 4, 0, 4, 2, 4, 9, 7, 4, 2];
    let mut d = Vec::with_capacity(16 * STEP_BYTES);
    for (i, deg) in degrees.iter().enumerate() {
        let on = !matches!(i, 7 | 11);
        let vel = if i % 4 == 0 { 112 } else { 88 };
        let ratchet = if i == 15 { 2 } else { 1 };
        d.extend_from_slice(&[on as u8, *deg, vel, 3, ratchet]);
    }
    d
}

pub fn step_at(data: &[u8], i: usize) -> Step {
    let o = i * STEP_BYTES;
    match data.get(o..o + STEP_BYTES) {
        Some(s) => Step {
            on: s[0] != 0,
            degree: s[1].min(STEP_MAX_DEGREE),
            velocity: s[2].clamp(1, 127),
            length: s[3].clamp(1, 8),
            ratchet: s[4].clamp(1, 4),
        },
        None => Step { on: false, degree: 0, velocity: 100, length: 3, ratchet: 1 },
    }
}

/// Which step plays at tick `k` for a direction.
pub fn step_index(k: i64, steps: usize, direction: usize, random: u32) -> usize {
    let n = steps.max(1) as i64;
    match direction {
        1 => (n - 1 - k.rem_euclid(n)) as usize,
        2 => {
            if n == 1 {
                0
            } else {
                let p = k.rem_euclid(2 * n - 2);
                (if p < n { p } else { 2 * n - 2 - p }) as usize
            }
        }
        3 => (random % n as u32) as usize,
        _ => k.rem_euclid(n) as usize,
    }
}

// ---------------------------------------------------------------- Acid data

/// Per step: gate (0 rest, 1 note, 2 tie), semitone (0..12), octave (0 down,
/// 1 middle, 2 up), accent, slide.
pub const ACID_BYTES: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AcidStep {
    pub gate: u8,
    pub semitone: u8,
    pub octave: u8,
    pub accent: bool,
    pub slide: bool,
}

pub fn acid_at(data: &[u8], i: usize) -> AcidStep {
    let o = i * ACID_BYTES;
    match data.get(o..o + ACID_BYTES) {
        Some(s) => AcidStep { gate: s[0].min(2), semitone: s[1].min(12), octave: s[2].min(2), accent: s[3] != 0, slide: s[4] != 0 },
        None => AcidStep { gate: 0, semitone: 0, octave: 1, accent: false, slide: false },
    }
}

pub fn acid_default() -> Vec<u8> {
    // A classic minor line: (gate, semitone, octave, accent, slide).
    let line: [(u8, u8, u8, u8, u8); 16] = [
        (1, 0, 1, 1, 0),
        (1, 0, 1, 0, 0),
        (1, 12, 1, 0, 1),
        (1, 0, 1, 0, 0),
        (0, 0, 1, 0, 0),
        (1, 3, 1, 1, 0),
        (1, 0, 1, 0, 1),
        (1, 7, 1, 0, 0),
        (1, 0, 1, 1, 0),
        (2, 0, 1, 0, 0),
        (1, 10, 1, 0, 1),
        (1, 12, 1, 0, 0),
        (1, 0, 0, 1, 0),
        (1, 3, 1, 0, 1),
        (1, 5, 1, 0, 0),
        (1, 7, 1, 1, 0),
    ];
    line.iter().flat_map(|s| [s.0, s.1, s.2, s.3, s.4]).collect()
}

/// A new 303-style line from a seed: mostly root notes, some octaves,
/// fifths and minor thirds, with accents, slides and the odd rest or tie.
pub fn acid_generate(seed: u32) -> Vec<u8> {
    let mut r = crate::dsp::Rng::new(seed | 1);
    let mut unit = move || (r.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
    let notes = [0u8, 0, 0, 0, 12, 12, 3, 7, 10, 5, 0, 7];
    let mut d = Vec::with_capacity(16 * ACID_BYTES);
    for i in 0..16 {
        let x = unit();
        let gate = if i == 0 {
            1
        } else if x < 0.12 {
            0
        } else if x < 0.2 {
            2
        } else {
            1
        };
        let semitone = notes[(unit() * notes.len() as f32) as usize % notes.len()];
        let y = unit();
        let octave = if y < 0.12 {
            0
        } else if y > 0.9 {
            2
        } else {
            1
        };
        let accent = (unit() < if i % 4 == 0 { 0.5 } else { 0.22 }) as u8;
        let slide = (unit() < 0.25) as u8;
        d.extend_from_slice(&[gate, semitone, octave, accent, slide]);
    }
    d
}

// ---------------------------------------------------------------- Drum Grid data

pub const DRUM_LANES: usize = 8;
/// 8 lanes of 16 cells (0 off, 1 hit, 2 accent), then one note per lane.
pub const DRUM_BYTES: usize = DRUM_LANES * 16 + DRUM_LANES;
pub const DRUM_NOTES: [u8; DRUM_LANES] = [36, 38, 39, 42, 46, 45, 37, 56];
pub const DRUM_PATTERNS: &[&str] = &["Four on the Floor", "Breakbeat", "Electro", "Boom Bap", "Half Time", "Clear"];

pub fn drum_cell(data: &[u8], lane: usize, step: usize) -> u8 {
    data.get(lane * 16 + step).copied().unwrap_or(0).min(2)
}

pub fn drum_note(data: &[u8], lane: usize) -> u8 {
    data.get(DRUM_LANES * 16 + lane).copied().unwrap_or(DRUM_NOTES[lane.min(DRUM_LANES - 1)]).min(127)
}

/// A General MIDI drum name for a note.
pub fn gm_drum_name(note: u8) -> &'static str {
    match note {
        35 | 36 => "Kick",
        37 => "Rim",
        38 | 40 => "Snare",
        39 => "Clap",
        41 | 43 | 45 => "Low Tom",
        47 | 48 | 50 => "High Tom",
        42 | 44 => "Closed Hat",
        46 => "Open Hat",
        49 | 57 => "Crash",
        51 | 59 => "Ride",
        54 => "Tambourine",
        56 => "Cowbell",
        _ => "Note",
    }
}

/// The grid of a named pattern (lanes: kick, snare, clap, closed hat, open
/// hat, tom, rim, cowbell). Notes are kept.
pub fn drum_pattern(name: &str, data: &mut Vec<u8>) {
    if data.len() < DRUM_BYTES {
        *data = drum_default();
    }
    for c in data[..DRUM_LANES * 16].iter_mut() {
        *c = 0;
    }
    let mut set = |lane: usize, steps: &[usize], v: u8| {
        for &s in steps {
            data[lane * 16 + s] = v;
        }
    };
    match name {
        "Four on the Floor" => {
            set(0, &[0, 4, 8, 12], 2);
            set(2, &[4, 12], 1);
            set(3, &[0, 4, 8, 12], 1);
            set(3, &[2, 6, 10, 14], 2);
            set(4, &[14], 1);
        }
        "Breakbeat" => {
            set(0, &[0, 10], 2);
            set(0, &[3], 1);
            set(1, &[4, 12], 2);
            set(1, &[7, 15], 1);
            set(3, &[0, 2, 4, 6, 8, 10, 12, 14], 1);
            set(4, &[11], 1);
        }
        "Electro" => {
            set(0, &[0, 6, 10], 2);
            set(2, &[4, 12], 2);
            set(3, &[0, 2, 4, 6, 8, 10, 12, 14], 1);
            set(7, &[3, 11], 1);
            set(5, &[14], 1);
        }
        "Boom Bap" => {
            set(0, &[0, 7, 10], 2);
            set(1, &[4, 12], 2);
            set(3, &[0, 2, 4, 6, 8, 10, 12, 14], 1);
            set(6, &[15], 1);
        }
        "Half Time" => {
            set(0, &[0, 3], 2);
            set(1, &[8], 2);
            set(3, &[0, 2, 4, 6, 8, 10, 12, 14], 1);
            set(4, &[14], 1);
        }
        _ => {}
    }
}

pub fn drum_default() -> Vec<u8> {
    let mut d = vec![0u8; DRUM_LANES * 16];
    d.extend_from_slice(&DRUM_NOTES);
    drum_pattern("Four on the Floor", &mut d);
    d
}

// ---------------------------------------------------------------- Progression data

/// Per chord: degree (0 = rest, 1..7), quality, inversion.
pub const PROG_BYTES: usize = 3;
pub const PROG_SLOTS: usize = 8;
pub const QUALITIES: &[&str] = &["Triad", "7th", "Sus2", "Sus4", "Power", "Add9", "9th"];

pub fn progression_default() -> Vec<u8> {
    // I - V - vi - IV, then IV - V - iii - vi for the second half.
    let slots: [(u8, u8, u8); PROG_SLOTS] = [(1, 0, 0), (5, 0, 1), (6, 0, 1), (4, 0, 1), (4, 1, 1), (5, 1, 1), (3, 0, 1), (6, 1, 0)];
    slots.iter().flat_map(|s| [s.0, s.1, s.2]).collect()
}

pub fn prog_slot(data: &[u8], i: usize) -> (u8, u8, u8) {
    let o = i * PROG_BYTES;
    match data.get(o..o + PROG_BYTES) {
        Some(s) => (s[0].min(7), s[1].min(QUALITIES.len() as u8 - 1), s[2].min(3)),
        None => (0, 0, 0),
    }
}

/// The pitches of a chord on `degree` (1-based) of the scale, built from
/// the scale degree at or below `base`.
pub fn chord_pitches(scale: &Scale, base: i32, degree: u8, quality: u8, inversion: u8) -> Vec<i32> {
    if degree == 0 {
        return Vec::new();
    }
    // Degree 1 is the tonic at or below the base note.
    let n = scale.len().max(1) as i32;
    let (b, _) = scale.locate(base);
    let root = b.div_euclid(n) * n + degree as i32 - 1;
    let steps: &[i32] = match quality {
        1 => &[0, 2, 4, 6],
        2 => &[0, 1, 4],
        3 => &[0, 3, 4],
        4 => &[0, 4, 7],
        5 => &[0, 2, 4, 8],
        6 => &[0, 2, 4, 6, 8],
        _ => &[0, 2, 4],
    };
    let mut notes: Vec<i32> = steps.iter().map(|s| scale.pitch_at(root + s)).collect();
    for _ in 0..inversion.min(notes.len() as u8 - 1) {
        let low = notes.remove(0);
        notes.push(low + 12);
    }
    notes
}

/// A roman-numeral name like "vi" or "V7" for a chord in a scale.
pub fn chord_name(scale: &Scale, degree: u8, quality: u8) -> String {
    if degree == 0 {
        return "-".into();
    }
    const ROMAN: [&str; 7] = ["I", "II", "III", "IV", "V", "VI", "VII"];
    let notes = chord_pitches(scale, 60 + scale.root as i32, degree, 0, 0);
    let third = notes.get(1).map(|n| n - notes[0]).unwrap_or(4);
    let fifth = notes.get(2).map(|n| n - notes[0]).unwrap_or(7);
    let mut name = ROMAN[(degree as usize - 1) % 7].to_string();
    if third == 3 {
        name = name.to_lowercase();
    }
    if fifth == 6 {
        name.push('°');
    }
    name.push_str(match quality {
        1 => "7",
        2 => "sus2",
        3 => "sus4",
        4 => "5",
        5 => "add9",
        6 => "9",
        _ => "",
    });
    name
}

// ---------------------------------------------------------------- runtime

impl SeqState {
    /// Plays a note now or in a later block (swing and ratchets can land
    /// after the current block).
    pub(crate) fn play(&mut self, ctx: &MidiCtx, at: u64, pitch: i32, vel: f32, len_frames: f64, out: &mut Vec<MidiEvent>) {
        if at < ctx.now + ctx.len as u64 {
            self.note(ctx, at, pitch, vel, len_frames, out);
        } else if self.pending.len() < self.pending.capacity() {
            self.pending.push((at, pitch.clamp(0, 127) as u8, vel, len_frames.max(1.0) as u64));
        }
    }

    /// Emits the queued notes that fall into this block.
    pub(crate) fn flush_pending(&mut self, ctx: &MidiCtx, out: &mut Vec<MidiEvent>) {
        let end = ctx.now + ctx.len as u64;
        let mut i = 0;
        while i < self.pending.len() {
            if self.pending[i].0 < end {
                let (at, pitch, vel, len) = self.pending.swap_remove(i);
                self.note(ctx, at.max(ctx.now), pitch as i32, vel, len as f64, out);
            } else {
                i += 1;
            }
        }
    }

    pub(crate) fn swing_at(at: u64, k: i64, swing: f32, step_frames: f64) -> u64 {
        if k % 2 == 1 {
            at + (swing.clamp(0.0, 1.0) as f64 * 0.5 * step_frames) as u64
        } else {
            at
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn step_seq(&mut self, k: i64, p: &[f32], data: &[u8], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let steps = p[1].round().clamp(1.0, 16.0) as usize;
        let random = self.rng.next_u32();
        let i = step_index(k, steps, p[2].round() as usize, random);
        self.visual[0] = i as u64;
        let s = step_at(data, i);
        if !s.on {
            return;
        }
        let at = Self::swing_at(at, k, p[3], step_frames);
        let (base, _) = ctx.scale.locate(p[4].round() as i32);
        let pitch = ctx.scale.pitch_at(base + s.degree as i32);
        let vel = (s.velocity as f32 / 127.0 * p[5]).clamp(0.02, 1.0);
        let r = s.ratchet.max(1) as u64;
        let sub = step_frames / r as f64;
        let len = (s.length as f64 * 0.25 * step_frames).min(if r > 1 { sub * 0.8 } else { f64::MAX });
        for j in 0..r {
            self.play(ctx, at + (j as f64 * sub) as u64, pitch, vel, len, out);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn acid(&mut self, k: i64, p: &[f32], data: &[u8], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let steps = p[1].round().clamp(1.0, 16.0) as usize;
        let i = k.rem_euclid(steps as i64) as usize;
        self.visual[0] = i as u64;
        let s = acid_at(data, i);
        let at = Self::swing_at(at, k, p[2], step_frames);
        let gate_len = (p[4] as f64 * step_frames).max(1.0);
        let in_key = p[6] >= 0.5;
        // Acid moves by semitones before keeping the line in key.
        let transpose = self.shift.map(|(key, reference)| key - reference).unwrap_or(0);
        let mut pitch = p[3].round() as i32 + s.semitone as i32 + (s.octave as i32 - 1) * 12 + transpose;
        if in_key {
            pitch = ctx.scale.snap(pitch.clamp(0, 127) as u8) as i32;
        }
        let vel = if s.accent { (0.62 + 0.38 * p[5]).min(1.0) } else { 0.62 };
        // A slide or tie into this step keeps the previous note sounding.
        let previous = self.acid_open.take();
        match (s.gate, previous) {
            (2, Some(prev)) => {
                // Tie: extend the previous note through this step.
                self.close_or_keep(prev, at, gate_len, s.slide);
            }
            (1, Some(prev)) if prev == pitch as u8 => {
                // Slide into the same note: legato, no new attack.
                self.close_or_keep(prev, at, gate_len, s.slide);
            }
            (1, prev) => {
                self.retrigger(ctx, pitch as u8, at, out);
                self.play_open(ctx, at, pitch as u8, vel, out);
                if let Some(prev) = prev {
                    // Overlap a little so a mono synth glides.
                    self.schedule_off(prev, at + (step_frames * 0.06).max(8.0) as u64);
                }
                self.close_or_keep(pitch as u8, at, gate_len, s.slide);
            }
            (_, Some(prev)) => self.schedule_off(prev, at),
            _ => {}
        }
    }

    /// Ends `pitch` after this step's gate, or keeps it open for a slide.
    fn close_or_keep(&mut self, pitch: u8, at: u64, gate_len: f64, slide: bool) {
        if slide {
            self.acid_open = Some(pitch);
        } else {
            self.schedule_off(pitch, at + gate_len as u64);
        }
    }

    /// A copy of `pitch` that is still sounding ends where the new one starts.
    fn retrigger(&mut self, ctx: &MidiCtx, pitch: u8, at: u64, out: &mut Vec<MidiEvent>) {
        if at >= ctx.now + ctx.len as u64 {
            return; // the pending note will retrigger when it is played
        }
        if let Some(i) = self.offs.iter().position(|o| o.1 == pitch) {
            self.offs.swap_remove(i);
            out.push(MidiEvent::off(at.saturating_sub(ctx.now) as u32, pitch));
        }
    }

    fn schedule_off(&mut self, pitch: u8, at: u64) {
        self.offs.retain(|o| o.1 != pitch);
        if self.offs.len() < self.offs.capacity() {
            self.offs.push((at, pitch));
        }
    }

    /// A note-on without a scheduled note-off (slides decide it later).
    fn play_open(&mut self, ctx: &MidiCtx, at: u64, pitch: u8, vel: f32, out: &mut Vec<MidiEvent>) {
        if at < ctx.now + ctx.len as u64 {
            let offset = at.saturating_sub(ctx.now) as u32;
            out.push(MidiEvent::on(offset, pitch, vel.clamp(0.01, 1.0)));
        } else if self.pending.len() < self.pending.capacity() {
            // Swung into the next block: a pending note with an "open" length.
            self.pending.push((at, pitch, vel, u64::MAX / 4));
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn drum_grid(&mut self, k: i64, p: &[f32], data: &[u8], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let steps = p[1].round().clamp(1.0, 16.0) as usize;
        let i = k.rem_euclid(steps as i64) as usize;
        self.visual[0] = i as u64;
        let at = Self::swing_at(at, k, p[2], step_frames);
        for lane in 0..DRUM_LANES {
            let c = drum_cell(data, lane, i);
            if c == 0 {
                continue;
            }
            let vel = if c == 2 { (p[3] + p[4]).min(1.0) } else { p[3] };
            self.play(ctx, at, drum_note(data, lane) as i32, vel, step_frames * 0.5, out);
        }
    }

    /// Runs on a 1/16 grid.
    pub(crate) fn progression(&mut self, k: i64, p: &[f32], data: &[u8], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let beats = CHORD_BEATS[(p[0].round().max(0.0) as usize).min(CHORD_BEATS.len() - 1)];
        let sixteenths = (beats * 4.0).round() as i64;
        let chords = p[1].round().clamp(1.0, PROG_SLOTS as f32) as i64;
        let slot = (k.div_euclid(sixteenths)).rem_euclid(chords) as usize;
        let pos = k.rem_euclid(sixteenths);
        self.visual[0] = slot as u64 | (pos as u64) << 8;
        let (degree, quality, inversion) = prog_slot(data, slot);
        let notes = chord_pitches(&ctx.scale, p[3].round() as i32, degree, quality, inversion);
        if notes.is_empty() {
            return;
        }
        let gate = p[4] as f64;
        let vel = p[5];
        let sixteenth = step_frames;
        let hit = |len16: f64| (len16 * sixteenth * gate).max(1.0);
        let all = |this: &mut Self, len: f64, out: &mut Vec<MidiEvent>| {
            for &n in &notes {
                this.play(ctx, at, n, vel, len, out);
            }
        };
        match p[2].round() as usize {
            0 => {
                if pos == 0 {
                    all(self, hit(sixteenths as f64), out);
                }
            }
            1 => {
                if pos % 4 == 0 {
                    all(self, hit(4.0), out);
                }
            }
            2 => {
                if pos % 2 == 0 {
                    all(self, hit(2.0), out);
                }
            }
            3 => {
                if pos % 4 == 2 {
                    all(self, hit(2.0), out);
                }
            }
            4 => {
                // Syncopated stabs: 1, a-of-1, and-of-2, 3 ... (3-3-2 feel).
                if matches!(pos % 16, 0 | 3 | 6 | 10 | 12) {
                    all(self, hit(1.5), out);
                }
            }
            5 => {
                if matches!(pos % 8, 0 | 6) {
                    all(self, hit(2.0), out);
                }
            }
            r => {
                let n = notes.len() as i64;
                let idx = pos.rem_euclid(n) as usize;
                let note = if r == 6 { notes[idx] } else { notes[notes.len() - 1 - idx] };
                self.play(ctx, at, note, vel, hit(1.0), out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::{MidiFxKind, MidiFxState};
    use crate::params::defaults;
    use crate::scale::ScaleKind;

    /// Runs `beats` at 120 BPM / 48 kHz; returns (beat, on, pitch, velocity).
    fn run(kind: MidiFxKind, p: &[f32], data: &[u8], beats: f64) -> Vec<(f64, bool, u8, f32)> {
        let spb = 24000.0;
        let mut st = MidiFxState::new(kind);
        let mut evs = Vec::new();
        let blocks = (beats * spb / 256.0) as u64;
        for b in 0..blocks {
            let now = b * 256;
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(9, ScaleKind::Minor), scale_on: true, beat: now as f64 / spb, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 2] };
            let mut out = Vec::new();
            st.process(kind, p, data, &[], &mut out, &ctx);
            for e in &out {
                evs.push(((now + e.offset as u64) as f64 / spb, e.on, e.pitch, e.vel));
            }
        }
        evs
    }

    #[test]
    fn step_plays_its_pattern_exactly() {
        let p = defaults(STEP);
        let data = step_default();
        let ons: Vec<_> = run(MidiFxKind::Step, &p, &data, 4.0).into_iter().filter(|e| e.1).collect();
        // 14 steps on, the last one ratcheted twice.
        assert_eq!(ons.len(), 15, "{ons:?}");
        let a_minor = Scale::new(9, ScaleKind::Minor);
        assert!(ons.iter().all(|e| a_minor.contains(e.2)));
        // Deterministic: the second bar repeats the first.
        let two: Vec<_> = run(MidiFxKind::Step, &p, &data, 8.0).into_iter().filter(|e| e.1).map(|e| e.2).collect();
        assert_eq!(two[..15], two[15..30]);
        // Backward plays the same notes in reverse order.
        let mut back = p.clone();
        back[2] = 1.0;
        let rev: Vec<u8> = run(MidiFxKind::Step, &back, &data, 4.0).into_iter().filter(|e| e.1).map(|e| e.2).collect();
        assert_eq!(rev[0], ons[ons.len() - 1].2);
    }

    #[test]
    fn acid_slides_overlap_and_accents_are_louder() {
        let p = defaults(ACID);
        let data = acid_default();
        let evs = run(MidiFxKind::Acid, &p, &data, 4.0);
        // Step 2 slides into step 3: the next note-on comes before the old note-off.
        let on3 = evs.iter().find(|e| e.1 && (e.0 - 0.75).abs() < 1e-3).expect("step 3 note");
        let off2 = evs.iter().find(|e| !e.1 && e.2 != on3.2 && e.0 > 0.5 && e.0 < 0.9).expect("step 2 off");
        assert!(off2.0 > on3.0, "slide overlaps: off {} vs on {}", off2.0, on3.0);
        let accented = evs.iter().find(|e| e.1 && e.0 < 0.01).unwrap().3;
        let plain = evs.iter().find(|e| e.1 && (e.0 - 0.25).abs() < 1e-3).unwrap().3;
        assert!(accented > plain);
        // Every note-on gets its note-off.
        let ons = evs.iter().filter(|e| e.1).count();
        let offs = evs.iter().filter(|e| !e.1).count();
        assert!((ons as i32 - offs as i32).abs() <= 1, "{ons} ons, {offs} offs");
        // A generated line is reproducible.
        assert_eq!(acid_generate(7), acid_generate(7));
        assert_ne!(acid_generate(7), acid_generate(8));
    }

    #[test]
    fn drum_grid_and_swing() {
        let p = defaults(DRUM_GRID);
        let mut data = drum_default();
        data[3 * 16 + 1] = 1; // a closed hat on the second sixteenth
        let evs: Vec<_> = run(MidiFxKind::DrumGrid, &p, &data, 4.0).into_iter().filter(|e| e.1).collect();
        let kicks: Vec<f64> = evs.iter().filter(|e| e.2 == 36).map(|e| e.0).collect();
        assert_eq!(kicks.len(), 4);
        assert!((kicks[2] - 2.0).abs() < 1e-3);
        let mut swung = p.clone();
        swung[2] = 0.5;
        let hats: Vec<f64> = run(MidiFxKind::DrumGrid, &swung, &data, 4.0).into_iter().filter(|e| e.1 && e.2 == 42).map(|e| e.0).collect();
        // The odd sixteenth moves later by a quarter of a sixteenth (50% swing).
        assert!(hats.iter().any(|b| (b - 0.3125).abs() < 2e-3), "{hats:?}");
        assert!(hats.iter().any(|b| b.abs() < 1e-3), "even steps stay put");
    }

    #[test]
    fn progression_plays_chords_in_key() {
        let p = defaults(PROGRESSION);
        let data = progression_default();
        let evs: Vec<_> = run(MidiFxKind::Progression, &p, &data, 16.0).into_iter().filter(|e| e.1).collect();
        // Four bars, one triad per bar.
        assert_eq!(evs.len(), 12, "{evs:?}");
        let bar = |b: f64| -> Vec<u8> {
            let mut v: Vec<u8> = evs.iter().filter(|e| (e.0 - b).abs() < 1e-3).map(|e| e.2 % 12).collect();
            v.sort();
            v
        };
        // A minor: i = A C E, v = E G B, VI = F A C, iv = D F A.
        assert_eq!(bar(0.0), vec![0, 4, 9]);
        assert_eq!(bar(4.0), vec![4, 7, 11]);
        assert_eq!(bar(8.0), vec![0, 5, 9]);
        assert_eq!(bar(12.0), vec![2, 5, 9]);
        let s = Scale::new(9, ScaleKind::Minor);
        assert_eq!(chord_name(&s, 1, 0), "i");
        assert_eq!(chord_name(&s, 3, 1), "III7");
        assert_eq!(chord_name(&s, 2, 0), "ii°");
    }
}
