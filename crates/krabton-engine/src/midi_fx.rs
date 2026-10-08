//! MIDI effects that sit in front of an instrument (like Live's MIDI
//! Effects): Arpeggiator, Chord, Note Length, Pitch, Scale, Random,
//! Velocity and Note Echo. They transform note events per audio block and
//! can schedule notes in the future (sample-accurate, tempo-synced).

use crate::dsp::Rng;
use crate::params::{ParamSpec, Unit};
use crate::scale::{Scale, ScaleKind, NOTE_NAMES};
use crate::sequencers::{self, SeqState};
use serde::{Deserialize, Serialize};

/// A note event inside one processing block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MidiEvent {
    /// Frame offset inside the current block.
    pub offset: u32,
    pub on: bool,
    pub pitch: u8,
    pub vel: f32,
}

impl MidiEvent {
    pub fn on(offset: u32, pitch: u8, vel: f32) -> Self {
        Self { offset, on: true, pitch, vel }
    }
    pub fn off(offset: u32, pitch: u8) -> Self {
        Self { offset, on: false, pitch, vel: 0.0 }
    }
}

/// Timing and musical context for one block.
#[derive(Clone, Copy, Debug)]
pub struct MidiCtx {
    /// Absolute frame of the block start (monotonic).
    pub now: u64,
    pub len: usize,
    pub samples_per_beat: f64,
    pub scale: Scale,
    pub scale_on: bool,
    /// Session-clock beat at the block start (sequencers follow it).
    pub beat: f64,
    pub playing: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MidiFxKind {
    Arpeggiator,
    Chord,
    NoteLength,
    Pitch,
    Scale,
    Random,
    Velocity,
    NoteEcho,
    Polymeter,
    Euclid,
    Turing,
    Life,
    Bounce,
    Drunk,
    Step,
    Acid,
    DrumGrid,
    Progression,
}

const ARP_STYLES: &[&str] = &["Up", "Down", "Up Down", "Down Up", "Converge", "Diverge", "Random", "Played", "Chord"];
const ARP_RATES: &[&str] = &["1/32", "1/16 T", "1/16", "1/8 T", "1/8", "1/4", "1/2", "1 Bar"];
const ARP_RATE_BEATS: [f64; 8] = [0.125, 1.0 / 6.0, 0.25, 1.0 / 3.0, 0.5, 1.0, 2.0, 4.0];
const CHORD_TYPES: &[&str] = &["Key Triad", "Key 7th", "Major", "Minor", "Maj7", "Min7", "Dom7", "Sus2", "Sus4", "Power", "Octaves", "Add9", "Custom"];
const LENGTHS: &[&str] = &["1/64", "1/32", "1/16", "1/8", "1/4", "1/2", "1 Bar", "2 Bars"];
const LENGTH_BEATS: [f64; 8] = [1.0 / 16.0, 0.125, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0];
const ECHO_TIMES: &[&str] = &["1/16", "1/8 T", "1/8", "1/8 D", "1/4", "1/4 D", "1/2"];
const ECHO_BEATS: [f64; 7] = [0.25, 1.0 / 3.0, 0.5, 0.75, 1.0, 1.5, 2.0];
const SOURCES: &[&str] = &["Project", "Custom"];
const SCALE_MODES: &[&str] = &["Snap", "Filter"];
const DIRECTIONS: &[&str] = &["Up", "Down", "Both"];
const SCALE_NAMES: &[&str] = &[
    "Major",
    "Minor",
    "Dorian",
    "Phrygian",
    "Lydian",
    "Mixolydian",
    "Locrian",
    "Harmonic Minor",
    "Melodic Minor",
    "Phrygian Dominant",
    "Hungarian Minor",
    "Major Pentatonic",
    "Minor Pentatonic",
    "Blues",
    "Whole Tone",
    "In Sen",
    "Hirajoshi",
    "Chromatic",
];

static ARP: &[ParamSpec] = &[
    ParamSpec::choice("Style", "Arp", ARP_STYLES, 0),
    ParamSpec::choice("Rate", "Arp", ARP_RATES, 2),
    ParamSpec::lin("Gate", "Arp", 0.05, 2.0, 0.75, Unit::Percent),
    ParamSpec::int("Octaves", "Range", 1.0, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Swing", "Range", 0.0, 0.75, 0.0, Unit::Percent),
    ParamSpec::toggle("Hold", "Range", false),
];
static CHORD: &[ParamSpec] = &[
    ParamSpec::choice("Type", "Chord", CHORD_TYPES, 0),
    ParamSpec::int("Inversion", "Chord", -2.0, 2.0, 0.0, Unit::None),
    ParamSpec::lin("Velocity", "Chord", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::int("Shift 1", "Custom", -24.0, 24.0, 4.0, Unit::Semi),
    ParamSpec::int("Shift 2", "Custom", -24.0, 24.0, 7.0, Unit::Semi),
    ParamSpec::int("Shift 3", "Custom", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::int("Shift 4", "Custom", -24.0, 24.0, 0.0, Unit::Semi),
];
static NOTE_LENGTH: &[ParamSpec] = &[ParamSpec::choice("Length", "Length", LENGTHS, 2), ParamSpec::lin("Gate", "Length", 0.1, 2.0, 1.0, Unit::Percent)];
static PITCH: &[ParamSpec] = &[
    ParamSpec::int("Pitch", "Pitch", -48.0, 48.0, 0.0, Unit::Semi),
    ParamSpec::int("Scale Steps", "Pitch", -14.0, 14.0, 0.0, Unit::None),
];
static SCALE: &[ParamSpec] = &[
    ParamSpec::choice("Source", "Scale", SOURCES, 0),
    ParamSpec::choice("Root", "Scale", &NOTE_NAMES, 0),
    ParamSpec::choice("Scale", "Scale", SCALE_NAMES, 0),
    ParamSpec::choice("Mode", "Scale", SCALE_MODES, 0),
];
static RANDOM: &[ParamSpec] = &[
    ParamSpec::lin("Chance", "Random", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Choices", "Random", 1.0, 12.0, 4.0, Unit::None),
    ParamSpec::int("Spacing", "Random", 1.0, 12.0, 1.0, Unit::Semi),
    ParamSpec::choice("Direction", "Random", DIRECTIONS, 2),
    ParamSpec::toggle("In Key", "Random", true),
];
static VELOCITY: &[ParamSpec] = &[
    ParamSpec::lin("Drive", "Velocity", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Out Low", "Velocity", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Out High", "Velocity", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Random", "Velocity", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Fixed", "Velocity", false),
];
static NOTE_ECHO: &[ParamSpec] = &[
    ParamSpec::choice("Time", "Echo", ECHO_TIMES, 2),
    ParamSpec::int("Repeats", "Echo", 1.0, 8.0, 3.0, Unit::None),
    ParamSpec::lin("Feedback", "Echo", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::int("Pitch", "Echo", -12.0, 12.0, 0.0, Unit::Semi),
];

impl MidiFxKind {
    pub const ALL: [MidiFxKind; 8] = [
        MidiFxKind::Arpeggiator,
        MidiFxKind::Chord,
        MidiFxKind::NoteLength,
        MidiFxKind::Pitch,
        MidiFxKind::Scale,
        MidiFxKind::Random,
        MidiFxKind::Velocity,
        MidiFxKind::NoteEcho,
    ];

    pub const SEQUENCERS: [MidiFxKind; 10] = [
        MidiFxKind::Step,
        MidiFxKind::Acid,
        MidiFxKind::DrumGrid,
        MidiFxKind::Progression,
        MidiFxKind::Polymeter,
        MidiFxKind::Euclid,
        MidiFxKind::Turing,
        MidiFxKind::Life,
        MidiFxKind::Bounce,
        MidiFxKind::Drunk,
    ];

    pub fn is_sequencer(self) -> bool {
        Self::SEQUENCERS.contains(&self)
    }

    /// Initial pattern data for sequencers that have editable patterns.
    pub fn default_data(self) -> Vec<u8> {
        match self {
            MidiFxKind::Polymeter => sequencers::polymeter_default(),
            MidiFxKind::Step => crate::classic_seq::step_default(),
            MidiFxKind::Acid => crate::classic_seq::acid_default(),
            MidiFxKind::DrumGrid => crate::classic_seq::drum_default(),
            MidiFxKind::Progression => crate::classic_seq::progression_default(),
            MidiFxKind::Life => sequencers::life_default(),
            _ => Vec::new(),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            MidiFxKind::Polymeter => "Polymeter",
            MidiFxKind::Step => "Step",
            MidiFxKind::Acid => "Acid",
            MidiFxKind::DrumGrid => "Drum Grid",
            MidiFxKind::Progression => "Progression",
            MidiFxKind::Euclid => "Euclid",
            MidiFxKind::Turing => "Turing Machine",
            MidiFxKind::Life => "Life",
            MidiFxKind::Bounce => "Bounce",
            MidiFxKind::Drunk => "Drunk",
            MidiFxKind::Arpeggiator => "Arpeggiator",
            MidiFxKind::Chord => "Chord",
            MidiFxKind::NoteLength => "Note Length",
            MidiFxKind::Pitch => "Pitch",
            MidiFxKind::Scale => "Scale",
            MidiFxKind::Random => "Random",
            MidiFxKind::Velocity => "Velocity",
            MidiFxKind::NoteEcho => "Note Echo",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            MidiFxKind::Polymeter => "Kria-style step sequencer: every lane has its own loop length",
            MidiFxKind::Step => "Classic 16-step sequencer: note, velocity, length and ratchets per step",
            MidiFxKind::Acid => "TB-303 style bass line with accent, slide and tie",
            MidiFxKind::DrumGrid => "x0x drum machine grid: 8 lanes, 16 steps, accents and swing",
            MidiFxKind::Progression => "A chord progression in the project key, with rhythms",
            MidiFxKind::Euclid => "Four euclidean rhythm rings, great for drums",
            MidiFxKind::Turing => "A looping shift register that slowly mutates into new melodies",
            MidiFxKind::Life => "Conway's Game of Life plays the board column by column",
            MidiFxKind::Bounce => "Bouncing balls: every floor hit plays a note",
            MidiFxKind::Drunk => "A random walk through the scale",
            MidiFxKind::Arpeggiator => "Plays held notes one after another, tempo-synced",
            MidiFxKind::Chord => "Turns single notes into chords, also in the project key",
            MidiFxKind::NoteLength => "Gives every note a fixed, tempo-synced length",
            MidiFxKind::Pitch => "Transposes by semitones or by scale steps",
            MidiFxKind::Scale => "Snaps or filters notes to a scale",
            MidiFxKind::Random => "Randomly changes note pitches (optionally in key)",
            MidiFxKind::Velocity => "Reshapes, limits or randomizes velocities",
            MidiFxKind::NoteEcho => "Repeats notes like a delay, with feedback and pitch",
        }
    }

    pub fn specs(self) -> &'static [ParamSpec] {
        match self {
            MidiFxKind::Polymeter => sequencers::POLYMETER,
            MidiFxKind::Step => crate::classic_seq::STEP,
            MidiFxKind::Acid => crate::classic_seq::ACID,
            MidiFxKind::DrumGrid => crate::classic_seq::DRUM_GRID,
            MidiFxKind::Progression => crate::classic_seq::PROGRESSION,
            MidiFxKind::Euclid => sequencers::EUCLID,
            MidiFxKind::Turing => sequencers::TURING,
            MidiFxKind::Life => sequencers::LIFE,
            MidiFxKind::Bounce => sequencers::BOUNCE,
            MidiFxKind::Drunk => sequencers::DRUNK,
            MidiFxKind::Arpeggiator => ARP,
            MidiFxKind::Chord => CHORD,
            MidiFxKind::NoteLength => NOTE_LENGTH,
            MidiFxKind::Pitch => PITCH,
            MidiFxKind::Scale => SCALE,
            MidiFxKind::Random => RANDOM,
            MidiFxKind::Velocity => VELOCITY,
            MidiFxKind::NoteEcho => NOTE_ECHO,
        }
    }
}

const MAX_MAP: usize = 8;

/// Remembers which output notes each input note produced, so that the
/// matching note-offs are always sent even if parameters change meanwhile.
pub struct NoteMap {
    outs: [[u8; MAX_MAP]; 128],
    counts: [u8; 128],
}

impl NoteMap {
    fn new() -> Self {
        Self { outs: [[0; MAX_MAP]; 128], counts: [0; 128] }
    }

    fn release(&mut self, pitch: u8, offset: u32, out: &mut Vec<MidiEvent>) {
        let p = pitch as usize & 127;
        for i in 0..self.counts[p] as usize {
            out.push(MidiEvent::off(offset, self.outs[p][i]));
        }
        self.counts[p] = 0;
    }

    fn start(&mut self, pitch: u8, notes: &[u8], vel: &[f32], offset: u32, out: &mut Vec<MidiEvent>) {
        self.release(pitch, offset, out);
        let p = pitch as usize & 127;
        let mut n = 0;
        for (i, &q) in notes.iter().enumerate() {
            if n >= MAX_MAP || self.outs[p][..n].contains(&q) {
                continue;
            }
            self.outs[p][n] = q;
            n += 1;
            out.push(MidiEvent::on(offset, q, vel.get(i).copied().unwrap_or(vel[0])));
        }
        self.counts[p] = n as u8;
    }

    fn clear(&mut self) {
        self.counts = [0; 128];
    }
}

/// Future events at absolute frame times.
pub struct Scheduler {
    events: Vec<(u64, MidiEvent)>,
}

impl Scheduler {
    fn new() -> Self {
        Self { events: Vec::with_capacity(512) }
    }

    fn push(&mut self, at: u64, ev: MidiEvent) {
        if self.events.len() < self.events.capacity() {
            self.events.push((at, ev));
        }
    }

    /// Moves every event due before the end of this block to `out`.
    fn emit_due(&mut self, ctx: &MidiCtx, out: &mut Vec<MidiEvent>) {
        let end = ctx.now + ctx.len as u64;
        let mut i = 0;
        while i < self.events.len() {
            let (at, mut ev) = self.events[i];
            if at < end {
                ev.offset = at.saturating_sub(ctx.now).min(ctx.len.saturating_sub(1) as u64) as u32;
                out.push(ev);
                self.events.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }
}

pub struct ArpState {
    notes: Vec<(u8, f32)>,
    physical: u32,
    next_step: Option<u64>,
    next_grid: u64,
    step: u64,
    sounding: Vec<(u8, u64)>,
    rng: Rng,
    seq: Vec<(u8, f32)>,
}

impl ArpState {
    fn new() -> Self {
        Self {
            notes: Vec::with_capacity(32),
            physical: 0,
            next_step: None,
            next_grid: 0,
            step: 0,
            sounding: Vec::with_capacity(64),
            rng: Rng::new(0x5EED_A2B1),
            seq: Vec::with_capacity(128),
        }
    }
}

pub enum MidiFxState {
    Seq(Box<SeqState>),
    Arp(Box<ArpState>),
    Mapped(Box<NoteMap>, Rng),
    Scheduled(Box<Scheduler>),
    Velocity(Rng),
}

fn rand01(rng: &mut Rng) -> f32 {
    (rng.bipolar() + 1.0) * 0.5
}

fn chord_intervals(kind: usize) -> &'static [i32] {
    match kind {
        2 => &[0, 4, 7],
        3 => &[0, 3, 7],
        4 => &[0, 4, 7, 11],
        5 => &[0, 3, 7, 10],
        6 => &[0, 4, 7, 10],
        7 => &[0, 2, 7],
        8 => &[0, 5, 7],
        9 => &[0, 7],
        10 => &[0, 12],
        11 => &[0, 4, 7, 14],
        _ => &[0],
    }
}

impl MidiFxState {
    pub fn new(kind: MidiFxKind) -> Self {
        if kind.is_sequencer() {
            return MidiFxState::Seq(Box::default());
        }
        match kind {
            MidiFxKind::Arpeggiator => MidiFxState::Arp(Box::new(ArpState::new())),
            MidiFxKind::NoteLength | MidiFxKind::NoteEcho => MidiFxState::Scheduled(Box::new(Scheduler::new())),
            MidiFxKind::Velocity => MidiFxState::Velocity(Rng::new(0x0BAD_5EED)),
            _ => MidiFxState::Mapped(Box::new(NoteMap::new()), Rng::new(0xFEED_1234)),
        }
    }

    pub fn matches(&self, kind: MidiFxKind) -> bool {
        if let MidiFxState::Seq(_) = self {
            return kind.is_sequencer();
        }
        matches!(
            (self, kind),
            (MidiFxState::Arp(_), MidiFxKind::Arpeggiator)
                | (MidiFxState::Scheduled(_), MidiFxKind::NoteLength | MidiFxKind::NoteEcho)
                | (MidiFxState::Velocity(_), MidiFxKind::Velocity)
                | (MidiFxState::Mapped(..), MidiFxKind::Chord | MidiFxKind::Pitch | MidiFxKind::Scale | MidiFxKind::Random)
        )
    }

    /// Forgets everything (transport stop / all notes off).
    pub fn reset(&mut self) {
        match self {
            MidiFxState::Seq(s) => s.reset(),
            MidiFxState::Arp(a) => {
                a.notes.clear();
                a.sounding.clear();
                a.physical = 0;
                a.next_step = None;
            }
            MidiFxState::Mapped(m, _) => m.clear(),
            MidiFxState::Scheduled(s) => s.events.clear(),
            MidiFxState::Velocity(_) => {}
        }
    }

    /// Packed visual state of a sequencer (for the UI and grid controllers).
    pub fn visual(&self) -> Option<[u64; 4]> {
        match self {
            MidiFxState::Seq(s) => Some(s.visual),
            _ => None,
        }
    }

    /// Transforms `input` (sorted by offset) into `out`. `data` is the
    /// device's pattern data (sequencers).
    pub fn process(&mut self, kind: MidiFxKind, p: &[f32], data: &[u8], input: &[MidiEvent], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        match (self, kind) {
            (MidiFxState::Seq(s), kind) => {
                // Sequencers add notes; what comes in is passed through.
                out.extend_from_slice(input);
                s.process(kind, p, data, out, ctx);
            }
            (MidiFxState::Arp(a), MidiFxKind::Arpeggiator) => arp(a, p, input, out, ctx),
            (MidiFxState::Mapped(map, rng), kind) => {
                let scale = scale_for(kind, p, ctx);
                for e in input {
                    if !e.on {
                        map.release(e.pitch, e.offset, out);
                        continue;
                    }
                    let mut notes = [0u8; MAX_MAP];
                    let mut vels = [e.vel; MAX_MAP];
                    let n = map_note(kind, p, e.pitch, e.vel, &scale, ctx.scale_on, rng, &mut notes, &mut vels);
                    if n > 0 {
                        map.start(e.pitch, &notes[..n], &vels[..n], e.offset, out);
                    }
                }
            }
            (MidiFxState::Scheduled(s), MidiFxKind::NoteLength) => {
                let len = LENGTH_BEATS[(p[0].round() as usize).min(7)] * p[1] as f64;
                let frames = (len * ctx.samples_per_beat).max(1.0) as u64;
                for e in input.iter().filter(|e| e.on) {
                    let at = ctx.now + e.offset as u64;
                    // Retrigger: end an earlier copy of this note now.
                    if let Some(pos) = s.events.iter().position(|(_, ev)| ev.pitch == e.pitch) {
                        s.events.swap_remove(pos);
                        out.push(MidiEvent::off(e.offset, e.pitch));
                    }
                    out.push(*e);
                    s.push(at + frames, MidiEvent::off(0, e.pitch));
                }
                s.emit_due(ctx, out);
            }
            (MidiFxState::Scheduled(s), MidiFxKind::NoteEcho) => {
                let time = ECHO_BEATS[(p[0].round() as usize).min(6)];
                let step = (time * ctx.samples_per_beat).max(1.0) as u64;
                let repeats = p[1].round().clamp(1.0, 8.0) as u64;
                let fb = p[2];
                let shift = p[3].round() as i32;
                for e in input {
                    out.push(*e);
                    let at = ctx.now + e.offset as u64;
                    let mut vel = e.vel;
                    for k in 1..=repeats {
                        let pitch = (e.pitch as i32 + shift * k as i32).clamp(0, 127) as u8;
                        if e.on {
                            vel *= fb;
                            if vel < 0.02 {
                                break;
                            }
                            s.push(at + k * step, MidiEvent::on(0, pitch, vel));
                        } else {
                            s.push(at + k * step, MidiEvent::off(0, pitch));
                        }
                    }
                }
                s.emit_due(ctx, out);
            }
            (MidiFxState::Velocity(rng), MidiFxKind::Velocity) => {
                for e in input {
                    let mut e = *e;
                    if e.on {
                        let (lo, hi) = (p[1].min(p[2]), p[1].max(p[2]));
                        let v = if p[4] >= 0.5 {
                            hi
                        } else {
                            let curved = e.vel.clamp(0.0, 1.0).powf(2f32.powf(-p[0] * 2.0));
                            lo + (hi - lo) * curved
                        };
                        let jitter = rng.bipolar() * p[3] * 0.5;
                        e.vel = (v + jitter).clamp(0.01, 1.0);
                    }
                    out.push(e);
                }
            }
            _ => out.extend_from_slice(input),
        }
    }
}

fn scale_for(kind: MidiFxKind, p: &[f32], ctx: &MidiCtx) -> Scale {
    if kind == MidiFxKind::Scale && p[0] >= 0.5 {
        let kind = ScaleKind::ALL[(p[2].round() as usize).min(ScaleKind::ALL.len() - 1)];
        Scale::new(p[1].round() as u8, kind)
    } else {
        ctx.scale
    }
}

/// Output notes for one input note of a stateless-per-note device.
#[allow(clippy::too_many_arguments)]
fn map_note(kind: MidiFxKind, p: &[f32], pitch: u8, vel: f32, scale: &Scale, scale_on: bool, rng: &mut Rng, notes: &mut [u8; MAX_MAP], vels: &mut [f32; MAX_MAP]) -> usize {
    let clamp = |x: i32| x.clamp(0, 127) as u8;
    match kind {
        MidiFxKind::Chord => {
            let ty = p[0].round() as usize;
            let mut list = [0i32; MAX_MAP];
            let mut n = 0;
            match ty {
                0 | 1 => {
                    let chord = scale.chord(pitch, if ty == 0 { 3 } else { 4 });
                    // Keep the played note as the root even if it is outside the scale.
                    let shift = pitch as i32 - chord.first().copied().unwrap_or(pitch) as i32;
                    for c in chord {
                        list[n] = c as i32 + shift;
                        n += 1;
                    }
                }
                12 => {
                    list[0] = pitch as i32;
                    n = 1;
                    for k in 3..7 {
                        let s = p[k].round() as i32;
                        if s != 0 {
                            list[n] = pitch as i32 + s;
                            n += 1;
                        }
                    }
                }
                _ => {
                    for &iv in chord_intervals(ty) {
                        list[n] = pitch as i32 + iv;
                        n += 1;
                    }
                }
            }
            // Inversions: move the lowest notes up (or the highest down) an octave.
            let inv = p[1].round() as i32;
            let notes_slice = &mut list[..n];
            notes_slice.sort_unstable();
            for _ in 0..inv.max(0) {
                notes_slice[0] += 12;
                notes_slice.sort_unstable();
            }
            for _ in 0..(-inv).max(0) {
                let last = notes_slice.len() - 1;
                notes_slice[last] -= 12;
                notes_slice.sort_unstable();
            }
            for i in 0..n {
                notes[i] = clamp(list[i]);
                vels[i] = if list[i] == pitch as i32 { vel } else { vel * p[2] };
            }
            n
        }
        MidiFxKind::Pitch => {
            let mut q = pitch as i32 + p[0].round() as i32;
            let steps = p[1].round() as i32;
            if steps != 0 {
                q = scale.step(clamp(q), steps) as i32;
            }
            notes[0] = clamp(q);
            1
        }
        MidiFxKind::Scale => {
            let filter = p[3] >= 0.5;
            if filter && !scale.contains(pitch) {
                return 0;
            }
            notes[0] = scale.snap(pitch);
            1
        }
        MidiFxKind::Random => {
            notes[0] = pitch;
            if rand01(rng) < p[0] {
                let choices = p[1].round().max(1.0) as i32;
                let k = 1 + (rand01(rng) * choices as f32).floor().min(choices as f32 - 1.0) as i32;
                let dir = match p[3].round() as i32 {
                    0 => 1,
                    1 => -1,
                    _ => {
                        if rng.bipolar() >= 0.0 {
                            1
                        } else {
                            -1
                        }
                    }
                };
                let in_key = p[4] >= 0.5 && scale_on;
                notes[0] = if in_key { scale.step(pitch, dir * k * p[2].round().max(1.0) as i32) } else { clamp(pitch as i32 + dir * k * p[2].round() as i32) };
            }
            1
        }
        _ => {
            notes[0] = pitch;
            1
        }
    }
}

fn arp(a: &mut ArpState, p: &[f32], input: &[MidiEvent], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
    let hold = p[5] >= 0.5;
    for e in input {
        if e.on {
            if hold && a.physical == 0 {
                a.notes.clear();
            }
            a.physical += 1;
            if !a.notes.iter().any(|n| n.0 == e.pitch) && a.notes.len() < a.notes.capacity() {
                a.notes.push((e.pitch, e.vel));
            }
            if a.next_step.is_none() {
                let at = ctx.now + e.offset as u64;
                a.next_step = Some(at);
                a.next_grid = at;
                a.step = 0;
            }
        } else {
            a.physical = a.physical.saturating_sub(1);
            if !hold {
                a.notes.retain(|n| n.0 != e.pitch);
            }
        }
    }
    if !hold && a.physical == 0 {
        a.notes.clear();
    }
    if a.notes.is_empty() {
        a.next_step = None;
    }

    let rate = ARP_RATE_BEATS[(p[1].round() as usize).min(7)];
    let step_len = (rate * ctx.samples_per_beat).max(1.0) as u64;
    let gate = ((p[2] as f64 * step_len as f64) as u64).max(1);
    let octaves = p[3].round().clamp(1.0, 4.0) as u8;
    let swing = (p[4] as f64 * step_len as f64 * 0.5) as u64;
    let style = p[0].round() as usize;
    let end = ctx.now + ctx.len as u64;
    let offset_of = |t: u64| t.saturating_sub(ctx.now).min(ctx.len.saturating_sub(1) as u64) as u32;

    // Build the note order for this block.
    a.seq.clear();
    let mut base: Vec<(u8, f32)> = Vec::new();
    std::mem::swap(&mut base, &mut a.seq);
    base.extend_from_slice(&a.notes);
    if style != 7 {
        base.sort_unstable_by_key(|n| n.0);
    }
    let n0 = base.len();
    for o in 1..octaves {
        for i in 0..n0 {
            let (pch, v) = base[i];
            if (pch as u32 + 12 * o as u32) <= 127 {
                base.push((pch + 12 * o, v));
            }
        }
    }
    let n = base.len();

    loop {
        let next_off = a.sounding.iter().map(|s| s.1).min();
        let step_at = if n > 0 { a.next_step } else { None };
        let t = match (next_off, step_at) {
            (Some(o), Some(s)) => o.min(s),
            (Some(o), None) => o,
            (None, Some(s)) => s,
            (None, None) => break,
        };
        if t >= end {
            break;
        }
        if next_off == Some(t) {
            if let Some(pos) = a.sounding.iter().position(|s| s.1 == t) {
                let (pitch, _) = a.sounding.swap_remove(pos);
                out.push(MidiEvent::off(offset_of(t), pitch));
            }
            continue;
        }
        // A step: pick the note(s).
        let s = a.step as usize;
        let mut play = |pitch: u8, vel: f32, a: &mut ArpState| {
            if let Some(pos) = a.sounding.iter().position(|x| x.0 == pitch) {
                a.sounding.swap_remove(pos);
                out.push(MidiEvent::off(offset_of(t), pitch));
            }
            out.push(MidiEvent::on(offset_of(t), pitch, vel));
            if a.sounding.len() < a.sounding.capacity() {
                a.sounding.push((pitch, t + gate));
            }
        };
        match style {
            8 => {
                for i in 0..n {
                    let (pch, v) = base[i];
                    play(pch, v, a);
                }
            }
            _ => {
                let idx = match style {
                    1 => n - 1 - s % n,
                    2 | 3 => {
                        let i = if n == 1 {
                            0
                        } else {
                            let period = 2 * n - 2;
                            let k = s % period;
                            if k < n {
                                k
                            } else {
                                period - k
                            }
                        };
                        if style == 3 {
                            n - 1 - i
                        } else {
                            i
                        }
                    }
                    4 | 5 => {
                        let i = s % n;
                        let conv = |i: usize| if i % 2 == 0 { i / 2 } else { n - 1 - i / 2 };
                        if style == 4 {
                            conv(i)
                        } else {
                            conv(n - 1 - i)
                        }
                    }
                    6 => (a.rng.next_u32() as usize) % n,
                    _ => s % n,
                };
                let (pch, v) = base[idx];
                play(pch, v, a);
            }
        }
        a.step += 1;
        a.next_grid += step_len;
        a.next_step = Some(a.next_grid + if a.step % 2 == 1 { swing } else { 0 });
    }
    base.clear();
    a.seq = base;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::defaults;

    fn ctx(now: u64, len: usize) -> MidiCtx {
        MidiCtx { now, len, samples_per_beat: 24000.0, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: 0.0, playing: true }
    }

    /// Runs a device over `blocks` blocks of 256 frames; `input` holds (absolute frame, event).
    fn run(kind: MidiFxKind, p: &[f32], input: &[(u64, MidiEvent)], blocks: u64) -> Vec<(u64, MidiEvent)> {
        let mut st = MidiFxState::new(kind);
        let mut out_all = Vec::new();
        for b in 0..blocks {
            let now = b * 256;
            let evs: Vec<MidiEvent> = input
                .iter()
                .filter(|(t, _)| *t >= now && *t < now + 256)
                .map(|(t, e)| MidiEvent { offset: (t - now) as u32, ..*e })
                .collect();
            let mut out = Vec::new();
            st.process(kind, p, &[], &evs, &mut out, &ctx(now, 256));
            out_all.extend(out.into_iter().map(|e| (now + e.offset as u64, e)));
        }
        out_all
    }

    #[test]
    fn chord_in_key_and_types() {
        let mut p = defaults(MidiFxKind::Chord.specs());
        let out = run(MidiFxKind::Chord, &p, &[(0, MidiEvent::on(0, 62, 1.0)), (1000, MidiEvent::off(0, 62))], 8);
        let ons: Vec<u8> = out.iter().filter(|e| e.1.on).map(|e| e.1.pitch).collect();
        assert_eq!(ons, vec![62, 65, 69], "D minor triad in C major");
        assert_eq!(out.iter().filter(|e| !e.1.on).count(), 3);
        p[0] = 2.0; // Major
        p[1] = 1.0; // first inversion
        let out = run(MidiFxKind::Chord, &p, &[(0, MidiEvent::on(0, 60, 1.0))], 1);
        let mut ons: Vec<u8> = out.iter().map(|e| e.1.pitch).collect();
        ons.sort();
        assert_eq!(ons, vec![64, 67, 72]);
    }

    #[test]
    fn arpeggiator_steps_in_time() {
        let p = defaults(MidiFxKind::Arpeggiator.specs()); // Up, 1/16, gate 75 %
        let input = [(0, MidiEvent::on(0, 60, 0.9)), (0, MidiEvent::on(0, 64, 0.9)), (0, MidiEvent::on(0, 67, 0.9)), (48000, MidiEvent::off(0, 60)), (48000, MidiEvent::off(0, 64)), (48000, MidiEvent::off(0, 67))];
        let out = run(MidiFxKind::Arpeggiator, &p, &input, 300);
        let ons: Vec<(u64, u8)> = out.iter().filter(|e| e.1.on).map(|e| (e.0, e.1.pitch)).collect();
        // Two beats held at 1/16 = 8 steps.
        assert_eq!(ons.len(), 8, "{ons:?}");
        assert_eq!(ons[0], (0, 60));
        assert_eq!(ons[1], (6000, 64));
        assert_eq!(ons[2], (12000, 67));
        assert_eq!(ons[3], (18000, 60));
        // Every note ends.
        assert_eq!(out.iter().filter(|e| !e.1.on).count(), 8);
    }

    #[test]
    fn arpeggiator_hold_keeps_playing() {
        let mut p = defaults(MidiFxKind::Arpeggiator.specs());
        p[5] = 1.0;
        let input = [(0, MidiEvent::on(0, 60, 0.9)), (100, MidiEvent::off(0, 60))];
        let out = run(MidiFxKind::Arpeggiator, &p, &input, 200);
        assert!(out.iter().filter(|e| e.1.on).count() >= 8);
    }

    #[test]
    fn note_length_and_echo() {
        let p = defaults(MidiFxKind::NoteLength.specs()); // 1/16 = 6000 frames
        let out = run(MidiFxKind::NoteLength, &p, &[(10, MidiEvent::on(0, 60, 1.0)), (100, MidiEvent::off(0, 60))], 40);
        assert_eq!(out.len(), 2);
        assert_eq!(out[1], (6010, MidiEvent::off(out[1].1.offset, 60)));

        let p = defaults(MidiFxKind::NoteEcho.specs()); // 1/8 = 12000 frames, 3 repeats
        let out = run(MidiFxKind::NoteEcho, &p, &[(0, MidiEvent::on(0, 60, 1.0)), (1000, MidiEvent::off(0, 60))], 200);
        let ons: Vec<u64> = out.iter().filter(|e| e.1.on).map(|e| e.0).collect();
        assert_eq!(ons, vec![0, 12000, 24000, 36000]);
        assert_eq!(out.iter().filter(|e| !e.1.on).count(), 4);
        assert!(out.iter().filter(|e| e.1.on).last().unwrap().1.vel < 0.3);
    }

    #[test]
    fn pitch_scale_random_velocity() {
        let mut p = defaults(MidiFxKind::Pitch.specs());
        p[1] = 2.0; // two scale steps
        let out = run(MidiFxKind::Pitch, &p, &[(0, MidiEvent::on(0, 60, 1.0)), (10, MidiEvent::off(0, 60))], 1);
        assert_eq!(out[0].1.pitch, 64);
        assert_eq!(out[1].1.pitch, 64, "note-off follows the transposed note");

        let mut p = defaults(MidiFxKind::Scale.specs());
        let out = run(MidiFxKind::Scale, &p, &[(0, MidiEvent::on(0, 61, 1.0))], 1);
        assert_eq!(out[0].1.pitch, 60);
        p[3] = 1.0; // filter
        let out = run(MidiFxKind::Scale, &p, &[(0, MidiEvent::on(0, 61, 1.0))], 1);
        assert!(out.is_empty());

        let mut p = defaults(MidiFxKind::Random.specs());
        p[0] = 1.0;
        let input: Vec<(u64, MidiEvent)> = (0..50).map(|i| (i * 10, MidiEvent::on(0, 60, 1.0))).collect();
        let out = run(MidiFxKind::Random, &p, &input, 4);
        let s = Scale::new(0, ScaleKind::Major);
        assert!(out.iter().filter(|e| e.1.on).all(|e| s.contains(e.1.pitch)));
        assert!(out.iter().any(|e| e.1.on && e.1.pitch != 60));

        let mut p = defaults(MidiFxKind::Velocity.specs());
        p[1] = 0.5;
        p[2] = 0.6;
        let out = run(MidiFxKind::Velocity, &p, &[(0, MidiEvent::on(0, 60, 1.0)), (1, MidiEvent::on(0, 62, 0.1))], 1);
        assert!(out.iter().all(|e| (0.49..=0.61).contains(&e.1.vel)));
    }
}
