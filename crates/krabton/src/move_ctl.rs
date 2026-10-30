//! The Ableton Move as a controller. In its Control Live mode the Move is a
//! plain USB-MIDI device (its "Live Port", channel 1): Krabton opens it as a
//! controller of its own (its pads are not a note keyboard), reads its pads,
//! steps, knobs and buttons, and lights them back. Its screen can't be
//! reached over USB, so the Move's screen is drawn on the computer instead
//! (the HUD when a knob is touched, and the Move tab). Not affiliated with
//! Ableton; the messages are as the Move's users found them (see the
//! extending-move wiki), not from a spec.
//!
//! - Pads (4x8, notes 68-99 from the bottom left): Notes (an in-key
//!   keyboard, rows a fourth apart), Drums (the kit on the left 4x4, sixteen
//!   velocities of the chosen drum on the right), Session (8 tracks by 4
//!   scenes of clips) and Touch (the Touch view's plates). Menu goes to the
//!   next; Shift + a row button picks one (the fourth: the colour test).
//! - Steps 1-16: the selected track's clip as a step sequencer (the chosen
//!   drum, or the notes held on the pads); Shift + a step: the clip that many
//!   bars long. In Session: the scenes (Shift: stop a track).
//! - The 8 knobs: the selected device's parameters, eight at a time (Left and
//!   Right: the device before or after (on a drum rack the chosen drum),
//!   the track's mixer last; Shift + Left and Right: the next eight). Shift
//!   turns finely; holding a row button, they turn that track's mixer.
//! - Volume: the master (Shift: the selected track; a row held: that track).
//!   The jog wheel picks the track (Shift: the scene).
//!
//! A real Move and the one on screen run the same actions; the lights are a
//! frame of the Move's notes and controls, sent where they changed.

use crate::app::{App, ClipSel, DetailTab, TrackSel, View};
use crossbeam_channel::{unbounded, Receiver, RecvTimeoutError, Sender};
use krabton_engine::params::{ParamKind, ParamSpec, Unit};
use krabton_engine::{Clip, Command, Instrument, Note, Project, Scale, TrackKind};
use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// The first pad's note (bottom left); the pads go up to 99, eight a row.
pub const PAD0: u8 = 68;
/// The first step button's note.
pub const STEP0: u8 = 16;
/// The knobs turn CCs 71-78 (and light them), the volume knob 79, the jog wheel 14.
pub const KNOB_CC0: u8 = 71;
pub const VOLUME_CC: u8 = 79;
pub const JOG_CC: u8 = 14;
/// Touching the volume knob and the jog wheel (the knobs touch notes 0-7).
pub const VOLUME_TOUCH: u8 = 8;
pub const JOG_TOUCH: u8 = 9;
/// A guess: the jog wheel pressed in (not known). Anything else unknown
/// shows in the Move tab's list of messages.
pub const JOG_PRESS_CC: u8 = 3;
/// The small lights under the steps (CCs 16-31; a guess that they light so).
pub const STEP_LED_CC0: u8 = 16;

/// The LED colour palette: the colour each index (the velocity of a pad's
/// Note On) lights. NOT VERIFIED: it is a guess after Push 2's (0 off, 120
/// and 122 white, 124 dark grey, 125 blue, 126 green, 127 red), with the
/// rest a wheel of 28 hues, each bright, pale, half and dim. The colour test
/// (Shift + Menu) lights the pads with the indices: correct this table to
/// what they show and everything else follows.
#[rustfmt::skip]
pub const PALETTE: [[u8; 3]; 128] = [
    [0x00, 0x00, 0x00], [0xff, 0x00, 0x00], [0xff, 0x73, 0x73], [0x73, 0x00, 0x00], [0x29, 0x00, 0x00], [0xff, 0x37, 0x00], [0xff, 0x91, 0x73], [0x73, 0x19, 0x00], // 0-7
    [0x29, 0x09, 0x00], [0xff, 0x6d, 0x00], [0xff, 0xaf, 0x73], [0x73, 0x31, 0x00], [0x29, 0x11, 0x00], [0xff, 0xa4, 0x00], [0xff, 0xcd, 0x73], [0x73, 0x4a, 0x00], // 8-15
    [0x29, 0x1a, 0x00], [0xff, 0xdb, 0x00], [0xff, 0xeb, 0x73], [0x73, 0x62, 0x00], [0x29, 0x23, 0x00], [0xed, 0xff, 0x00], [0xf5, 0xff, 0x73], [0x6b, 0x73, 0x00], // 16-23
    [0x26, 0x29, 0x00], [0xb6, 0xff, 0x00], [0xd7, 0xff, 0x73], [0x52, 0x73, 0x00], [0x1d, 0x29, 0x00], [0x80, 0xff, 0x00], [0xb9, 0xff, 0x73], [0x39, 0x73, 0x00], // 24-31
    [0x14, 0x29, 0x00], [0x49, 0xff, 0x00], [0x9b, 0xff, 0x73], [0x21, 0x73, 0x00], [0x0c, 0x29, 0x00], [0x12, 0xff, 0x00], [0x7d, 0xff, 0x73], [0x08, 0x73, 0x00], // 32-39
    [0x03, 0x29, 0x00], [0x00, 0xff, 0x24], [0x73, 0xff, 0x87], [0x00, 0x73, 0x10], [0x00, 0x29, 0x06], [0x00, 0xff, 0x5b], [0x73, 0xff, 0xa5], [0x00, 0x73, 0x29], // 40-47
    [0x00, 0x29, 0x0f], [0x00, 0xff, 0x92], [0x73, 0xff, 0xc3], [0x00, 0x73, 0x42], [0x00, 0x29, 0x17], [0x00, 0xff, 0xc8], [0x73, 0xff, 0xe1], [0x00, 0x73, 0x5a], // 48-55
    [0x00, 0x29, 0x20], [0x00, 0xff, 0xff], [0x73, 0xff, 0xff], [0x00, 0x73, 0x73], [0x00, 0x29, 0x29], [0x00, 0xc8, 0xff], [0x73, 0xe1, 0xff], [0x00, 0x5a, 0x73], // 56-63
    [0x00, 0x20, 0x29], [0x00, 0x92, 0xff], [0x73, 0xc3, 0xff], [0x00, 0x42, 0x73], [0x00, 0x17, 0x29], [0x00, 0x5b, 0xff], [0x73, 0xa5, 0xff], [0x00, 0x29, 0x73], // 64-71
    [0x00, 0x0f, 0x29], [0x00, 0x24, 0xff], [0x73, 0x87, 0xff], [0x00, 0x10, 0x73], [0x00, 0x06, 0x29], [0x12, 0x00, 0xff], [0x7d, 0x73, 0xff], [0x08, 0x00, 0x73], // 72-79
    [0x03, 0x00, 0x29], [0x49, 0x00, 0xff], [0x9b, 0x73, 0xff], [0x21, 0x00, 0x73], [0x0c, 0x00, 0x29], [0x80, 0x00, 0xff], [0xb9, 0x73, 0xff], [0x39, 0x00, 0x73], // 80-87
    [0x14, 0x00, 0x29], [0xb6, 0x00, 0xff], [0xd7, 0x73, 0xff], [0x52, 0x00, 0x73], [0x1d, 0x00, 0x29], [0xed, 0x00, 0xff], [0xf5, 0x73, 0xff], [0x6b, 0x00, 0x73], // 88-95
    [0x26, 0x00, 0x29], [0xff, 0x00, 0xdb], [0xff, 0x73, 0xeb], [0x73, 0x00, 0x62], [0x29, 0x00, 0x23], [0xff, 0x00, 0xa4], [0xff, 0x73, 0xcd], [0x73, 0x00, 0x4a], // 96-103
    [0x29, 0x00, 0x1a], [0xff, 0x00, 0x6d], [0xff, 0x73, 0xaf], [0x73, 0x00, 0x31], [0x29, 0x00, 0x11], [0xff, 0x00, 0x37], [0xff, 0x73, 0x91], [0x73, 0x00, 0x19], // 104-111
    [0x29, 0x00, 0x09], [0x20, 0x20, 0x20], [0x38, 0x38, 0x38], [0x50, 0x50, 0x50], [0x70, 0x70, 0x70], [0x90, 0x90, 0x90], [0xc0, 0xc0, 0xc0], [0x80, 0x80, 0x80], // 112-119
    [0xff, 0xff, 0xff], [0x30, 0x30, 0x30], [0xff, 0xff, 0xff], [0xb0, 0xb0, 0xb0], [0x40, 0x40, 0x40], [0x20, 0x40, 0xff], [0x20, 0xff, 0x40], [0xff, 0x10, 0x10], // 120-127
];

/// Palette indices used by name.
pub const WHITE: u8 = 120;
pub const GREEN: u8 = 126;
pub const RED: u8 = 127;
/// Brightness of the white button lights: off, dim (there to press), on.
pub const OFF: u8 = 0;
pub const DIM: u8 = 20;
pub const ON: u8 = 127;

/// The palette index nearest to a colour (0 for black).
pub fn nearest(c: [u8; 3]) -> u8 {
    if c.iter().all(|&v| v < 10) {
        return 0;
    }
    let d = |p: &[u8; 3]| {
        let e = |i: usize, w: i32| w * (p[i] as i32 - c[i] as i32).pow(2);
        e(0, 3) + e(1, 4) + e(2, 2)
    };
    (1..128u8).min_by_key(|&i| d(&PALETTE[i as usize])).unwrap_or(0)
}

/// A colour at `f` of its brightness.
pub fn dim(c: [u8; 3], f: f32) -> [u8; 3] {
    c.map(|v| (v as f32 * f).round().clamp(0.0, 255.0) as u8)
}

/// The Move's buttons, by the CC they send (127 pressed, 0 let go).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Button {
    /// The four buttons left of the pad rows, top to bottom (CCs 43 to 40).
    Row(u8),
    Menu,
    Play,
    Record,
    Up,
    Down,
    Left,
    Right,
    Capture,
    RecordAudio,
    Loop,
    Mute,
    Delete,
    Copy,
    Undo,
    Shift,
    /// The jog wheel pressed in (a guess, see `JOG_PRESS_CC`).
    Jog,
}

impl Button {
    pub const ALL: [Button; 20] = [
        Button::Row(0),
        Button::Row(1),
        Button::Row(2),
        Button::Row(3),
        Button::Menu,
        Button::Play,
        Button::Record,
        Button::Up,
        Button::Down,
        Button::Left,
        Button::Right,
        Button::Capture,
        Button::RecordAudio,
        Button::Loop,
        Button::Mute,
        Button::Delete,
        Button::Copy,
        Button::Undo,
        Button::Shift,
        Button::Jog,
    ];

    pub fn cc(self) -> u8 {
        match self {
            Button::Row(k) => 43 - k.min(3),
            Button::Menu => 50,
            Button::Play => 85,
            Button::Record => 86,
            Button::Up => 55,
            Button::Down => 54,
            Button::Left => 62,
            Button::Right => 63,
            Button::Capture => 52,
            Button::RecordAudio => 118,
            Button::Loop => 58,
            Button::Mute => 88,
            Button::Delete => 119,
            Button::Copy => 60,
            Button::Undo => 56,
            Button::Shift => 49,
            Button::Jog => JOG_PRESS_CC,
        }
    }

    pub fn from_cc(cc: u8) -> Option<Button> {
        Button::ALL.into_iter().find(|b| b.cc() == cc)
    }

    pub fn name(self) -> &'static str {
        match self {
            Button::Row(0) => "Row 1",
            Button::Row(1) => "Row 2",
            Button::Row(2) => "Row 3",
            Button::Row(_) => "Row 4",
            Button::Menu => "Menu",
            Button::Play => "Play",
            Button::Record => "Rec",
            Button::Up => "Up",
            Button::Down => "Down",
            Button::Left => "Left",
            Button::Right => "Right",
            Button::Capture => "Capture",
            Button::RecordAudio => "Sample",
            Button::Loop => "Loop",
            Button::Mute => "Mute",
            Button::Delete => "Delete",
            Button::Copy => "Copy",
            Button::Undo => "Undo",
            Button::Shift => "Shift",
            Button::Jog => "Jog",
        }
    }

    /// Whether its light takes a palette colour (else a brightness). A
    /// guess: the ones that are coloured on the Move.
    pub fn coloured(self) -> bool {
        matches!(self, Button::Row(_) | Button::Play | Button::Record | Button::Mute | Button::RecordAudio)
    }
}

/// The knobs: the eight above the pads, the volume knob and the jog wheel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Knob {
    Enc(u8),
    Volume,
    Jog,
}

impl Knob {
    /// Its place in `MoveState::touched`.
    pub fn index(self) -> usize {
        match self {
            Knob::Enc(k) => k.min(7) as usize,
            Knob::Volume => 8,
            Knob::Jog => 9,
        }
    }
}

/// What the Move sends, read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoveIn {
    /// Pad 0-31 (bottom left first, eight a row), how hard (0..1).
    Pad { pad: u8, down: bool, vel: f32 },
    /// A pad pressed harder or softer (polyphonic aftertouch, if it sends it).
    PadPressure { pad: u8, value: f32 },
    Step { step: u8, down: bool },
    Touch { knob: Knob, down: bool },
    /// Turned this many clicks (clockwise up).
    Turn { knob: Knob, ticks: i32 },
    Button { button: Button, down: bool },
    /// Something not placed (shown in the Move tab).
    Other([u8; 3]),
}

/// A relative knob value: 1..63 clockwise that many, 65..127 the other way
/// (128 minus it).
pub fn rel(v: u8) -> i32 {
    match v {
        0 => 0,
        1..=63 => v as i32,
        _ => v as i32 - 128,
    }
}

/// Reads a message from the Move (None: one to ignore, as the clock).
pub fn parse(msg: &[u8]) -> Option<MoveIn> {
    let status = *msg.first()?;
    if status >= 0xf0 {
        return None;
    }
    let (d1, d2) = (msg.get(1).copied().unwrap_or(0), msg.get(2).copied().unwrap_or(0));
    let raw = [status, d1, d2];
    let kind = status & 0xf0;
    Some(match kind {
        0x80 | 0x90 => {
            let down = kind == 0x90 && d2 > 0;
            match d1 {
                0..=7 => MoveIn::Touch { knob: Knob::Enc(d1), down },
                VOLUME_TOUCH => MoveIn::Touch { knob: Knob::Volume, down },
                JOG_TOUCH => MoveIn::Touch { knob: Knob::Jog, down },
                16..=31 => MoveIn::Step { step: d1 - STEP0, down },
                68..=99 => MoveIn::Pad { pad: d1 - PAD0, down, vel: d2 as f32 / 127.0 },
                _ => MoveIn::Other(raw),
            }
        }
        0xa0 if (68..=99).contains(&d1) => MoveIn::PadPressure { pad: d1 - PAD0, value: d2 as f32 / 127.0 },
        0xb0 => match d1 {
            71..=78 => MoveIn::Turn { knob: Knob::Enc(d1 - KNOB_CC0), ticks: rel(d2) },
            VOLUME_CC => MoveIn::Turn { knob: Knob::Volume, ticks: rel(d2) },
            JOG_CC => MoveIn::Turn { knob: Knob::Jog, ticks: rel(d2) },
            _ => match Button::from_cc(d1) {
                Some(button) => MoveIn::Button { button, down: d2 > 0 },
                None => MoveIn::Other(raw),
            },
        },
        _ => MoveIn::Other(raw),
    })
}

/// Whether a MIDI port is one of the Move's.
pub fn is_move_port(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("ableton move") || (n.contains("move") && n.contains("live port"))
}

/// The Move's port Krabton speaks to: its Live Port, else the first of its
/// ports that is not its User Port.
pub fn best_move_port(names: &[String]) -> Option<String> {
    let moves: Vec<&String> = names.iter().filter(|n| is_move_port(n)).collect();
    let low = |n: &&String| n.to_lowercase();
    moves.iter().find(|n| low(n).contains("live")).or_else(|| moves.iter().find(|n| !low(n).contains("user"))).map(|s| s.to_string())
}

/// The Move's lights: a value per note (pads and steps: a palette index)
/// and per CC (buttons: a brightness or an index; the knobs' lights).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub notes: [u8; 128],
    pub ccs: [u8; 128],
}

impl Frame {
    pub fn dark() -> Self {
        Self { notes: [0; 128], ccs: [0; 128] }
    }

    /// Nothing sent yet: every light goes out the first time.
    pub fn unknown() -> Self {
        Self { notes: [255; 128], ccs: [255; 128] }
    }

    pub fn pad(&self, i: u8) -> u8 {
        self.notes[(PAD0 + i.min(31)) as usize]
    }

    pub fn set_pad(&mut self, i: u8, c: u8) {
        if i < 32 {
            self.notes[(PAD0 + i) as usize] = c.min(127);
        }
    }

    pub fn step(&self, i: u8) -> u8 {
        self.notes[(STEP0 + i.min(15)) as usize]
    }

    pub fn set_step(&mut self, i: u8, c: u8) {
        if i < 16 {
            self.notes[(STEP0 + i) as usize] = c.min(127);
        }
    }

    pub fn button(&self, b: Button) -> u8 {
        self.ccs[b.cc() as usize]
    }

    pub fn set_button(&mut self, b: Button, v: u8) {
        self.ccs[b.cc() as usize] = v.min(127);
    }

    pub fn knob(&self, k: u8) -> u8 {
        self.ccs[(KNOB_CC0 + k.min(7)) as usize]
    }

    pub fn set_knob(&mut self, k: u8, v: u8) {
        if k < 8 {
            self.ccs[(KNOB_CC0 + k) as usize] = v.min(127);
        }
    }

    pub fn step_led(&self, i: u8) -> u8 {
        self.ccs[(STEP_LED_CC0 + i.min(15)) as usize]
    }

    pub fn set_step_led(&mut self, i: u8, v: u8) {
        if i < 16 {
            self.ccs[(STEP_LED_CC0 + i) as usize] = v.min(127);
        }
    }

    /// The messages that bring the Move from `sent` to this frame, at most
    /// `max` (the rest go next time); `sent` is what it shows after them.
    /// Only the Move's own lights are sent.
    pub fn diff(&self, sent: &mut Frame, max: usize) -> Vec<[u8; 3]> {
        let mut out = Vec::new();
        let notes = (STEP0..STEP0 + 16).chain(PAD0..PAD0 + 32);
        for n in notes {
            let i = n as usize;
            if out.len() < max && sent.notes[i] != self.notes[i] {
                out.push([0x90, n, self.notes[i]]);
                sent.notes[i] = self.notes[i];
            }
        }
        let ccs = Button::ALL.iter().filter(|b| **b != Button::Jog).map(|b| b.cc()).chain(KNOB_CC0..KNOB_CC0 + 8).chain(STEP_LED_CC0..STEP_LED_CC0 + 16);
        for c in ccs {
            let i = c as usize;
            if out.len() < max && sent.ccs[i] != self.ccs[i] {
                out.push([0xb0, c, self.ccs[i]]);
                sent.ccs[i] = self.ccs[i];
            }
        }
        out
    }
}

/// What the pads play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadMode {
    Notes,
    Drums,
    Session,
    Touch,
}

impl PadMode {
    pub fn name(self) -> &'static str {
        match self {
            PadMode::Notes => "Notes",
            PadMode::Drums => "Drums",
            PadMode::Session => "Session",
            PadMode::Touch => "Touch",
        }
    }
}

/// What the knobs turn: a device of the selected track, the chosen drum
/// of a drum rack, or the track's mixer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Instrument,
    Drum,
    Midi(usize),
    Fx(usize),
    Mixer,
}

/// The chosen drum's knobs, as the drum rack's pad view has them (A and B
/// take the names of the voice's own knobs).
const DRUM_SPECS: [ParamSpec; 8] = [
    ParamSpec::lin("Level", "Pad", -36.0, 6.0, 0.0, Unit::Db),
    ParamSpec::int("Tune", "Pad", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::log("Decay", "Pad", 0.1, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Tone", "Pad", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Pan", "Pad", -1.0, 1.0, 0.0, Unit::Pan),
    ParamSpec::lin("A", "Pad", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("B", "Pad", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Vel > Level", "Velocity", 0.0, 1.0, 1.0, Unit::Percent),
];

/// A drum's values in the order of `DRUM_SPECS`.
fn drum_values(p: &krabton_engine::DrumPad) -> [f32; 8] {
    [p.level_db, p.tune, p.decay, p.tone, p.pan, p.a, p.b, p.vel_level]
}

fn set_drum_value(p: &mut krabton_engine::DrumPad, k: usize, v: f32) {
    match k {
        0 => p.level_db = v,
        1 => p.tune = v,
        2 => p.decay = v,
        3 => p.tone = v,
        4 => p.pan = v,
        5 => p.a = v,
        6 => p.b = v,
        _ => p.vel_level = v,
    }
}

/// A step is a sixteenth.
pub const STEP: f64 = 0.25;

/// The note of a pad in Notes mode: every pad in the key, a step of the
/// scale to the right and three (a fourth) up a row; the bottom left the
/// root, `octave` as the computer keyboard's.
pub fn keys_pitch(scale: &Scale, octave: i32, pad: u8) -> u8 {
    let (col, row) = ((pad % 8) as i32, (pad / 8) as i32);
    let base = (octave + 1) * 12 + scale.root as i32;
    let (abs, _) = scale.locate(base);
    scale.pitch_at(abs + col + row * 3).clamp(0, 127) as u8
}

/// The drums of a drum track, the first sixteen: (note, name). Empty for a
/// melodic instrument.
pub fn drum_kit(inst: &Instrument) -> Vec<(u8, String)> {
    if let Instrument::Drums { pads, .. } = inst {
        return pads.iter().take(16).map(|p| (p.note, p.name.clone())).collect();
    }
    let bits = inst.drum_notes();
    (0..128u8).filter(|n| bits >> n & 1 == 1).take(16).map(|n| (n, krabton_engine::note_name(n))).collect()
}

/// A drum's colour, by what it is (kicks red, snares orange, hats yellow...).
pub fn drum_color(name: &str) -> [u8; 3] {
    let n = name.to_lowercase();
    let has = |w: &[&str]| w.iter().any(|x| n.contains(x));
    if has(&["kick", "bd", "bass drum"]) {
        [0xff, 0x30, 0x20]
    } else if has(&["snare", "sd", "rim"]) {
        [0xff, 0x90, 0x10]
    } else if has(&["clap", "snap"]) {
        [0xff, 0x50, 0xa0]
    } else if has(&["hat", "hh"]) {
        [0xff, 0xe0, 0x20]
    } else if has(&["cym", "crash", "ride", "china", "splash"]) {
        [0x40, 0xe0, 0xff]
    } else if has(&["tom", "conga", "bongo", "timb"]) {
        [0x40, 0xff, 0x60]
    } else {
        [0xa0, 0x60, 0xff]
    }
}

/// Whether a note starts on step `s`.
fn on_step(n: &Note, s: usize) -> bool {
    let a = s as f64 * STEP - 1e-6;
    n.start >= a && n.start < a + STEP
}

/// Whether step `s` has a note of `pitch` (None: any note).
pub fn step_has(notes: &[Note], s: usize, pitch: Option<u8>) -> bool {
    notes.iter().any(|n| on_step(n, s) && pitch.is_none_or(|p| n.pitch == p))
}

/// A step tapped: the pitches go onto it (a sixteenth long each), or off
/// when they are all there already. Returns whether they went on.
pub fn toggle_step(notes: &mut Vec<Note>, s: usize, pitches: &[u8], vel: f32) -> bool {
    if pitches.is_empty() {
        return false;
    }
    if pitches.iter().all(|&p| step_has(notes, s, Some(p))) {
        notes.retain(|n| !(on_step(n, s) && pitches.contains(&n.pitch)));
        return false;
    }
    for &p in pitches {
        if !step_has(notes, s, Some(p)) {
            notes.push(Note::new(p, s as f64 * STEP, STEP, vel));
        }
    }
    notes.sort_by(|a, b| a.start.total_cmp(&b.start).then(a.pitch.cmp(&b.pitch)));
    true
}

/// Takes every note off step `s` (of `pitch` only, when given).
pub fn clear_step(notes: &mut Vec<Note>, s: usize, pitch: Option<u8>) {
    notes.retain(|n| !(on_step(n, s) && pitch.is_none_or(|p| n.pitch == p)));
}

/// Step `to` gets the notes of step `from` (what was on it goes).
pub fn copy_step(notes: &mut Vec<Note>, from: usize, to: usize) {
    if from == to {
        return;
    }
    let moved: Vec<Note> = notes.iter().filter(|n| on_step(n, from)).map(|n| Note { start: n.start + (to as f64 - from as f64) * STEP, ..n.clone() }).collect();
    clear_step(notes, to, None);
    notes.extend(moved);
    notes.sort_by(|a, b| a.start.total_cmp(&b.start).then(a.pitch.cmp(&b.pitch)));
}

/// A parameter turned `ticks` clicks: a smooth one a 128th of its way each
/// (finely a 1024th), a stepped one a step every three clicks (finely
/// eight), kept in `acc` between turns. Stepped ones with many steps turn
/// as smooth ones do.
pub fn turn_param(spec: &ParamSpec, v: f32, ticks: i32, fine: bool, acc: &mut f32) -> f32 {
    let few = spec.is_discrete() && spec.max - spec.min <= 24.0;
    if !few {
        let per = if fine { 1024.0 } else { 128.0 };
        let n = spec.to_norm(v) + ticks as f32 / per;
        let out = spec.from_norm(n);
        // A stepped one with many steps still moves on every click.
        if spec.is_discrete() && out == v && ticks != 0 {
            return spec.clamp(v + ticks.signum() as f32);
        }
        return out;
    }
    *acc += ticks as f32 / if fine { 8.0 } else { 3.0 };
    let steps = (*acc + acc.signum() * 1e-4).trunc();
    *acc -= steps;
    if steps == 0.0 {
        return v;
    }
    match spec.kind {
        ParamKind::Toggle => {
            if steps > 0.0 {
                1.0
            } else {
                0.0
            }
        }
        _ => spec.clamp(v.round() + steps),
    }
}

/// A 0..1 value turned `ticks` clicks (a fader, a send).
pub fn turn_unit(v: f32, ticks: i32, fine: bool) -> f32 {
    (v + ticks as f32 / if fine { 1024.0 } else { 128.0 }).clamp(0.0, 1.0)
}

/// What the lights are drawn from: the project and what the Move is doing.
pub struct Look<'a> {
    pub project: &'a Project,
    pub mode: PadMode,
    pub track: Option<usize>,
    /// Per track: the slot playing and the one waiting (-1: none).
    pub slots: Vec<(i32, i32)>,
    /// The clip the steps edit (its scene on `track`) and where it plays.
    pub clip: Option<usize>,
    pub clip_pos: Option<f64>,
    /// The notes the pads play (Notes and Touch).
    pub pitches: [u8; 32],
    /// The notes held on the pads.
    pub held: Vec<u8>,
    /// The drum the steps edit and play (Drums).
    pub drum: usize,
    /// The velocity pad held (Drums, 0-15).
    pub vel_pad: Option<usize>,
    pub step_page: usize,
    pub track_offset: usize,
    pub scene_offset: usize,
    pub playing: bool,
    pub recording: bool,
    pub blink: bool,
    pub held_buttons: Vec<Button>,
    /// The colour test: its page of 32 indices.
    pub test_page: Option<usize>,
    /// How many knobs have something to turn, and the one touched.
    pub knobs: usize,
    pub touched: Option<usize>,
    /// Up and Down (and Left and Right) have somewhere to go.
    pub can_up: bool,
    pub can_down: bool,
    pub can_left: bool,
    pub can_right: bool,
}

impl Look<'_> {
    fn holds(&self, b: Button) -> bool {
        self.held_buttons.contains(&b)
    }

    fn track_color(&self) -> [u8; 3] {
        self.track.and_then(|t| self.project.tracks.get(t)).map(|t| t.color).unwrap_or([0xff, 0xa7, 0x33])
    }

    /// The notes of the clip the steps edit.
    fn clip_notes(&self) -> (&[Note], f64) {
        let c = self.track.zip(self.clip).and_then(|(t, s)| self.project.tracks.get(t)?.slots.get(s)?.as_ref());
        match c {
            Some(c) => (c.notes(), c.length),
            None => (&[], self.project.beats_per_bar_f()),
        }
    }
}

/// The step the playhead is on (of all of the clip's).
fn play_step(pos: Option<f64>) -> Option<usize> {
    pos.filter(|p| *p >= 0.0).map(|p| (p / STEP).floor() as usize)
}

/// The Move's lights for a state.
pub fn render(l: &Look) -> Frame {
    let mut f = Frame::dark();
    let tcol = l.track_color();
    let shift = l.holds(Button::Shift);
    let kit: Vec<(u8, String)> = l.track.and_then(|t| l.project.tracks.get(t)?.instrument.as_ref()).map(drum_kit).unwrap_or_default();

    // The pads.
    if let Some(page) = l.test_page {
        for i in 0..32u8 {
            f.set_pad(i, (page * 32) as u8 + i);
        }
    } else {
        match l.mode {
            PadMode::Notes | PadMode::Touch => {
                let scale = l.project.sounding_key();
                for i in 0..32u8 {
                    let p = l.pitches[i as usize];
                    let c = if l.held.contains(&p) {
                        WHITE
                    } else if scale.is_root(p) {
                        nearest(tcol)
                    } else {
                        nearest(dim([0xff, 0xff, 0xff], 0.16))
                    };
                    f.set_pad(i, c);
                }
            }
            PadMode::Drums => {
                for (k, (note, name)) in kit.iter().enumerate() {
                    let i = (k / 4 * 8 + k % 4) as u8;
                    let c = if l.held.contains(note) {
                        WHITE
                    } else if k == l.drum {
                        nearest(drum_color(name))
                    } else {
                        nearest(dim(drum_color(name), 0.4))
                    };
                    f.set_pad(i, c);
                }
                // Sixteen velocities of the chosen drum, soft bottom left.
                for v in 0..16usize {
                    let i = (v / 4 * 8 + 4 + v % 4) as u8;
                    let c = if l.vel_pad == Some(v) { WHITE } else { nearest(dim(tcol, 0.12 + 0.88 * v as f32 / 15.0)) };
                    f.set_pad(i, c);
                }
            }
            PadMode::Session => {
                for col in 0..8usize {
                    let t = l.track_offset + col;
                    let Some(track) = l.project.tracks.get(t) else { continue };
                    let (playing, queued) = l.slots.get(t).copied().unwrap_or((-1, -1));
                    for row in 0..4usize {
                        let s = l.scene_offset + (3 - row);
                        let i = (row * 8 + col) as u8;
                        let c = match track.slots.get(s) {
                            Some(Some(clip)) if playing == s as i32 => {
                                if l.blink {
                                    nearest(clip.color)
                                } else {
                                    WHITE
                                }
                            }
                            Some(Some(clip)) if queued == s as i32 => {
                                if l.blink {
                                    nearest(clip.color)
                                } else {
                                    0
                                }
                            }
                            Some(Some(clip)) => nearest(dim(clip.color, 0.5)),
                            Some(None) if track.arm && track.kind == TrackKind::Midi => nearest(dim([0xff, 0x20, 0x20], 0.16)),
                            _ => 0,
                        };
                        f.set_pad(i, c);
                    }
                }
            }
        }
    }

    // The steps.
    if let Some(page) = l.test_page {
        for i in 0..4u8 {
            f.set_step(i, if i as usize == page { WHITE } else { nearest(dim([0xff, 0xff, 0xff], 0.16)) });
        }
    } else if l.mode == PadMode::Session {
        let ns = l.project.scenes.len();
        for s in 0..16usize.min(ns) {
            let any = l.project.tracks.iter().any(|t| matches!(t.slots.get(s), Some(Some(_))));
            let playing = l.project.tracks.iter().enumerate().any(|(t, tr)| matches!(tr.slots.get(s), Some(Some(_))) && l.slots.get(t).is_some_and(|x| x.0 == s as i32));
            let c = if playing {
                if l.blink {
                    GREEN
                } else {
                    nearest(dim([0x20, 0xff, 0x40], 0.45))
                }
            } else if any {
                nearest(dim([0xff, 0xff, 0xff], 0.45))
            } else {
                nearest(dim([0xff, 0xff, 0xff], 0.12))
            };
            f.set_step(s as u8, c);
        }
    } else {
        let (notes, length) = l.clip_notes();
        let total = ((length / STEP).round() as usize).max(1);
        let bars = ((length / l.project.beats_per_bar_f()).ceil() as usize).clamp(1, 16);
        if shift {
            // The clip's length in bars.
            for i in 0..16usize {
                f.set_step(i as u8, if i < bars { nearest(tcol) } else { nearest(dim(tcol, 0.12)) });
            }
        } else {
            let pitch = match l.mode {
                PadMode::Drums => kit.get(l.drum).map(|d| d.0),
                _ => l.held.first().copied(),
            };
            let at = play_step(l.clip_pos);
            for i in 0..16usize {
                let s = l.step_page * 16 + i;
                let c = if s >= total {
                    0
                } else if at == Some(s) && l.playing {
                    WHITE
                } else if step_has(notes, s, pitch) {
                    nearest(tcol)
                } else if pitch.is_some() && step_has(notes, s, None) {
                    nearest(dim(tcol, 0.25))
                } else if i % 4 == 0 {
                    nearest(dim([0xff, 0xff, 0xff], 0.1))
                } else {
                    0
                };
                f.set_step(i as u8, c);
            }
        }
        // Under the steps: the clip's pages, the one shown brighter.
        let pages = total.div_ceil(16).clamp(1, 16);
        for i in 0..pages {
            f.set_step_led(i as u8, if i == l.step_page { ON } else { DIM });
        }
    }

    // The row buttons: scenes in Session, else the tracks of the bank of
    // four (with Shift: the modes).
    for k in 0..4u8 {
        let b = Button::Row(k);
        let c = if shift {
            let lit = match k {
                0 => matches!(l.mode, PadMode::Notes | PadMode::Drums) && l.test_page.is_none(),
                1 => l.mode == PadMode::Session && l.test_page.is_none(),
                2 => l.mode == PadMode::Touch && l.test_page.is_none(),
                _ => l.test_page.is_some(),
            };
            if lit {
                WHITE
            } else {
                nearest(dim([0xff, 0xff, 0xff], 0.16))
            }
        } else if l.mode == PadMode::Session {
            let s = l.scene_offset + k as usize;
            if s >= l.project.scenes.len() {
                0
            } else if l.project.tracks.iter().enumerate().any(|(t, tr)| matches!(tr.slots.get(s), Some(Some(_))) && l.slots.get(t).is_some_and(|x| x.0 == s as i32)) {
                GREEN
            } else {
                nearest(dim([0xff, 0xff, 0xff], 0.3))
            }
        } else {
            let bank = l.track.unwrap_or(0) / 4 * 4;
            match l.project.tracks.get(bank + k as usize) {
                Some(tr) if Some(bank + k as usize) == l.track => nearest(tr.color),
                Some(tr) if tr.mixer.mute => nearest(dim(tr.color, 0.08)),
                Some(tr) => nearest(dim(tr.color, 0.3)),
                None => 0,
            }
        };
        f.set_button(b, c);
    }

    // The other buttons.
    let sel = l.track.and_then(|t| l.project.tracks.get(t));
    let lit = |on: bool| if on { ON } else { DIM };
    f.set_button(Button::Play, if l.playing { GREEN } else { nearest(dim([0x20, 0xff, 0x40], 0.2)) });
    f.set_button(Button::Record, if l.recording { RED } else { nearest(dim([0xff, 0x10, 0x10], 0.2)) });
    let mute = match sel {
        Some(t) if t.mixer.solo => nearest([0x20, 0x40, 0xff]),
        Some(t) if t.mixer.mute => nearest([0xff, 0xa0, 0x10]),
        Some(_) => nearest(dim([0xff, 0xa0, 0x10], 0.2)),
        None => 0,
    };
    f.set_button(Button::Mute, mute);
    f.set_button(Button::RecordAudio, if sel.is_some_and(|t| t.arm) { RED } else { nearest(dim([0xff, 0x10, 0x10], 0.12)) });
    f.set_button(Button::Loop, lit(l.project.loop_enabled));
    f.set_button(Button::Shift, lit(shift));
    f.set_button(Button::Menu, ON);
    f.set_button(Button::Capture, DIM);
    f.set_button(Button::Undo, DIM);
    f.set_button(Button::Copy, lit(l.holds(Button::Copy)));
    f.set_button(Button::Delete, lit(l.holds(Button::Delete)));
    f.set_button(Button::Up, if l.can_up { DIM } else { OFF });
    f.set_button(Button::Down, if l.can_down { DIM } else { OFF });
    f.set_button(Button::Left, if l.can_left { DIM } else { OFF });
    f.set_button(Button::Right, if l.can_right { DIM } else { OFF });

    // The knobs' lights: the ones that turn something, the touched one white.
    for k in 0..8u8 {
        let c = if l.touched == Some(k as usize) {
            WHITE
        } else if (k as usize) < l.knobs {
            nearest(dim(tcol, 0.5))
        } else {
            0
        };
        f.set_knob(k, c);
    }
    f
}

/// To the Move's thread.
enum ToMove {
    Enable(bool),
    Ports(Option<String>, Option<String>),
    Leds(Vec<[u8; 3]>),
    Rescan,
    Quit(Sender<()>),
}

/// From the Move's thread.
pub enum MoveEvent {
    Connected { input: String, output: Option<String> },
    Gone(String),
    Error(String),
    Ports { inputs: Vec<String>, outputs: Vec<String> },
}

/// How many light messages go out at once (the Move takes about sixty), and how often.
const BURST: usize = 60;
const BURST_EVERY: Duration = Duration::from_millis(20);

/// A copy action: what was pressed first while Copy is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CopyFrom {
    Step(usize),
    Slot(usize, usize),
}

/// The Move, on the UI side.
pub struct MoveState {
    to: Option<Sender<ToMove>>,
    events: Option<Receiver<MoveEvent>>,
    pub rx: Receiver<MoveIn>,
    /// Its input and output, while it is plugged in.
    pub connected: Option<(String, Option<String>)>,
    /// The MIDI ports there are (for picking the Move's by hand).
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    /// What the pads play (Notes also means Drums on a drum track).
    pub mode: PadMode,
    /// The colour test's page of 32 indices.
    pub test_page: Option<usize>,
    pub leds: Frame,
    sent: Frame,
    last_send: Instant,
    pub held: Vec<Button>,
    /// A held button was used with something (so letting it go does nothing).
    used: bool,
    pub touched: [bool; 10],
    /// The knob turned or touched last, and when (for the screen).
    pub hud_knob: Option<Knob>,
    pub hud_at: Option<Instant>,
    /// The device the knobs turn (of the selected track's chain), and which eight.
    pub page: usize,
    pub bank: usize,
    pub step_page: usize,
    pub track_offset: usize,
    pub scene_offset: usize,
    pads_held: HashMap<u8, (u8, Vec<u64>)>,
    pub last_pitch: Option<u8>,
    pub last_vel: f32,
    /// The velocity pad held in Drums.
    pub vel_pad: Option<usize>,
    copy_from: Option<CopyFrom>,
    acc: [f32; 8],
    /// Messages not placed, newest last.
    pub log: VecDeque<String>,
    /// The on-screen Move: the pad, step or button the mouse holds, and
    /// buttons it latched (Shift, Mute, Delete, Copy stay down until clicked again).
    pub virtual_down: Option<MoveIn>,
    pub latched: Vec<Button>,
}

impl MoveState {
    /// Starts looking for a Move (not in test windows, unless KRABTON_MOVE=1).
    pub fn new(prefs: &crate::prefs::Prefs) -> Self {
        let (in_tx, rx) = unbounded();
        let test = std::env::var_os("KRABTON_TEST_WINDOW").is_some() || cfg!(test);
        let hardware = std::env::var("KRABTON_MOVE").map(|v| v == "1").unwrap_or(!test);
        let (to, events) = if hardware {
            let (to, to_rx) = unbounded();
            let (ev_tx, events) = unbounded();
            let (on, pick) = (!prefs.move_off, (prefs.move_in.clone(), prefs.move_out.clone()));
            let ok = std::thread::Builder::new().name("krabton-move".into()).spawn(move || run(to_rx, ev_tx, in_tx, on, pick)).is_ok();
            if ok {
                (Some(to), Some(events))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        Self {
            to,
            events,
            rx,
            connected: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            mode: PadMode::Notes,
            test_page: None,
            leds: Frame::dark(),
            sent: Frame::unknown(),
            last_send: Instant::now(),
            held: Vec::new(),
            used: false,
            touched: [false; 10],
            hud_knob: None,
            hud_at: None,
            page: 0,
            bank: 0,
            step_page: 0,
            track_offset: 0,
            scene_offset: 0,
            pads_held: HashMap::new(),
            last_pitch: None,
            last_vel: 0.8,
            vel_pad: None,
            copy_from: None,
            acc: [0.0; 8],
            log: VecDeque::new(),
            virtual_down: None,
            latched: Vec::new(),
        }
    }

    /// Whether a real Move can be used (the thread runs).
    pub fn hardware(&self) -> bool {
        self.to.is_some()
    }

    pub fn set_enabled(&self, on: bool) {
        if let Some(to) = &self.to {
            let _ = to.send(ToMove::Enable(on));
        }
    }

    pub fn set_ports(&self, input: Option<String>, output: Option<String>) {
        if let Some(to) = &self.to {
            let _ = to.send(ToMove::Ports(input, output));
        }
    }

    pub fn rescan(&self) {
        if let Some(to) = &self.to {
            let _ = to.send(ToMove::Rescan);
        }
    }

    pub fn holds(&self, b: Button) -> bool {
        self.held.contains(&b)
    }

    /// A knob is touched: what it turns is one edit until it is let go.
    pub fn holds_edit(&self) -> bool {
        self.touched[..9].iter().any(|t| *t)
    }

    /// Whether the screen shows (a knob touched, or turned a moment ago).
    pub fn hud_shown(&self) -> bool {
        self.holds_edit() || self.hud_at.is_some_and(|t| t.elapsed() < Duration::from_millis(1500))
    }

    fn note_unknown(&mut self, raw: [u8; 3]) {
        if self.log.len() >= 12 {
            self.log.pop_front();
        }
        self.log.push_back(format!("{:02X} {:02X} {:02X}", raw[0], raw[1], raw[2]));
    }

    /// Sends the lights that changed (a burst at a time).
    fn flush(&mut self) {
        let Some(to) = &self.to else { return };
        if self.connected.as_ref().and_then(|c| c.1.as_ref()).is_none() || self.last_send.elapsed() < BURST_EVERY {
            return;
        }
        let msgs = self.leds.diff(&mut self.sent, BURST);
        if !msgs.is_empty() {
            self.last_send = Instant::now();
            let _ = to.send(ToMove::Leds(msgs));
        }
    }
}

impl Drop for MoveState {
    /// The Move's lights go out when Krabton closes.
    fn drop(&mut self) {
        if let Some(to) = &self.to {
            let (done, wait) = unbounded();
            if to.send(ToMove::Quit(done)).is_ok() {
                let _ = wait.recv_timeout(Duration::from_millis(300));
            }
        }
    }
}

fn port_names_in() -> Vec<String> {
    MidiInput::new("krabton-move-scan").map(|i| i.ports().iter().filter_map(|p| i.port_name(p).ok()).collect()).unwrap_or_default()
}

fn port_names_out() -> Vec<String> {
    MidiOutput::new("krabton-move-scan-out").map(|o| o.ports().iter().filter_map(|p| o.port_name(p).ok()).collect()).unwrap_or_default()
}

fn open_input(name: &str, tx: Sender<MoveIn>) -> Result<MidiInputConnection<Sender<MoveIn>>, String> {
    let mut input = MidiInput::new("krabton-move").map_err(|e| e.to_string())?;
    input.ignore(Ignore::All);
    let port = input.ports().into_iter().find(|p| input.port_name(p).is_ok_and(|n| n == name)).ok_or("not found")?;
    input
        .connect(
            &port,
            "krabton-move-in",
            |_stamp, msg, tx: &mut Sender<MoveIn>| {
                if let Some(m) = parse(msg) {
                    let _ = tx.send(m);
                }
            },
            tx,
        )
        .map_err(|e| e.to_string())
}

fn open_output(name: &str) -> Result<MidiOutputConnection, String> {
    let out = MidiOutput::new("krabton-move").map_err(|e| e.to_string())?;
    let port = out.ports().into_iter().find(|p| out.port_name(p).is_ok_and(|n| n == name)).ok_or("not found")?;
    out.connect(&port, "krabton-move-out").map_err(|e| e.to_string())
}

/// Every light out.
fn blank(out: &mut Option<(String, MidiOutputConnection)>) {
    if let Some((_, c)) = out.as_mut() {
        for m in Frame::dark().diff(&mut Frame::unknown(), 512) {
            let _ = c.send(&m);
        }
    }
}

/// The Move's thread: looks for it now and then, opens it when it is
/// plugged in (and closes it when it goes), and sends its lights.
fn run(rx: Receiver<ToMove>, events: Sender<MoveEvent>, input_tx: Sender<MoveIn>, mut enabled: bool, mut pick: (Option<String>, Option<String>)) {
    let mut conn_in: Option<(String, MidiInputConnection<Sender<MoveIn>>)> = None;
    let mut conn_out: Option<(String, MidiOutputConnection)> = None;
    let mut last_scan: Option<Instant> = None;
    let mut reported: Option<String> = None;
    loop {
        let mut close = false;
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(ToMove::Leds(msgs)) => {
                if let Some((_, c)) = conn_out.as_mut() {
                    if msgs.iter().any(|m| c.send(m).is_err()) {
                        close = true;
                    }
                }
            }
            Ok(ToMove::Enable(on)) => {
                enabled = on;
                close = !on;
                last_scan = None;
            }
            Ok(ToMove::Ports(i, o)) => {
                pick = (i, o);
                close = true;
                last_scan = None;
                reported = None;
            }
            Ok(ToMove::Rescan) => {
                last_scan = None;
                reported = None;
            }
            Ok(ToMove::Quit(done)) => {
                blank(&mut conn_out);
                drop(conn_out.take());
                drop(conn_in.take());
                let _ = done.send(());
                return;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        if close && conn_in.is_some() {
            blank(&mut conn_out);
            conn_out = None;
            if let Some((name, c)) = conn_in.take() {
                c.close();
                let _ = events.send(MoveEvent::Gone(name));
            }
        }
        if last_scan.is_some_and(|t| t.elapsed() < Duration::from_millis(1500)) {
            continue;
        }
        last_scan = Some(Instant::now());
        let (inputs, outputs) = (port_names_in(), port_names_out());
        // Unplugged.
        if conn_in.as_ref().is_some_and(|(name, _)| !inputs.contains(name)) {
            conn_out = None;
            if let Some((name, c)) = conn_in.take() {
                c.close();
                let _ = events.send(MoveEvent::Gone(name));
            }
        }
        if enabled && conn_in.is_none() {
            let want_in = match &pick.0 {
                Some(p) => inputs.contains(p).then(|| p.clone()),
                None => best_move_port(&inputs),
            };
            if let Some(name) = want_in {
                match open_input(&name, input_tx.clone()) {
                    Ok(c) => {
                        let want_out = match &pick.1 {
                            Some(p) => outputs.contains(p).then(|| p.clone()),
                            None => outputs.iter().find(|o| **o == name).cloned().or_else(|| best_move_port(&outputs)),
                        };
                        conn_out = want_out.and_then(|o| match open_output(&o) {
                            Ok(c) => Some((o, c)),
                            Err(e) => {
                                let _ = events.send(MoveEvent::Error(format!("Ableton Move's lights ('{o}'): {e}")));
                                None
                            }
                        });
                        let _ = events.send(MoveEvent::Connected { input: name.clone(), output: conn_out.as_ref().map(|c| c.0.clone()) });
                        conn_in = Some((name, c));
                        reported = None;
                    }
                    Err(e) => {
                        let msg = format!("Ableton Move '{name}' would not open: {e} (is another program using it?)");
                        if reported.as_ref() != Some(&msg) {
                            let _ = events.send(MoveEvent::Error(msg.clone()));
                            reported = Some(msg);
                        }
                    }
                }
            }
        }
        let _ = events.send(MoveEvent::Ports { inputs, outputs });
    }
}

/// What the Move's screen shows: a title, a line under it, and up to eight
/// parameters (name, value, 0..1), the touched one.
pub struct Screen {
    pub title: String,
    pub sub: String,
    pub params: Vec<(String, String, f32)>,
    pub hot: Option<usize>,
}

impl App {
    /// The track the Move plays and edits: the selected one.
    fn move_track(&self) -> Option<usize> {
        match self.sel_track {
            TrackSel::Track(t) if t < self.project.tracks.len() => Some(t),
            _ => None,
        }
    }

    /// What the pads play now (Notes turns into Drums on a drum track).
    pub(crate) fn move_mode(&self) -> PadMode {
        let drums = self.move_track().and_then(|t| self.project.tracks[t].instrument.as_ref()).is_some_and(|i| !drum_kit(i).is_empty());
        match self.move_ctl.mode {
            PadMode::Notes | PadMode::Drums if drums => PadMode::Drums,
            PadMode::Notes | PadMode::Drums => PadMode::Notes,
            m => m,
        }
    }

    /// The drums of the selected track.
    fn move_kit(&self) -> Vec<(u8, String)> {
        self.move_track().and_then(|t| self.project.tracks[t].instrument.as_ref()).map(drum_kit).unwrap_or_default()
    }

    /// The notes the pads play (Notes, Touch).
    pub(crate) fn move_pitches(&self) -> [u8; 32] {
        let mode = self.move_mode();
        let t = self.move_track();
        std::array::from_fn(|i| match (mode, t) {
            (PadMode::Touch, Some(t)) => self.touch_pad_pitch(t, (i % 8) as i32, (i / 8) as i32),
            _ => keys_pitch(&self.project.sounding_key(), self.kb_octave, i as u8),
        })
    }

    /// The labels of the pads on screen: notes, drums, clips or palette indices.
    pub(crate) fn move_pad_labels(&self) -> Vec<String> {
        if let Some(page) = self.move_ctl.test_page {
            return (0..32).map(|i| (page * 32 + i).to_string()).collect();
        }
        match self.move_mode() {
            PadMode::Notes | PadMode::Touch => self.move_pitches().iter().map(|p| krabton_engine::note_name(*p)).collect(),
            PadMode::Drums => {
                let kit = self.move_kit();
                (0..32usize)
                    .map(|i| {
                        let (col, row) = (i % 8, i / 8);
                        if col < 4 {
                            kit.get(row * 4 + col).map(|d| d.1.clone()).unwrap_or_default()
                        } else {
                            format!("{}", ((row * 4 + col - 4) + 1) * 8 - 1)
                        }
                    })
                    .collect()
            }
            PadMode::Session => (0..32usize)
                .map(|i| {
                    let (col, row) = (i % 8, i / 8);
                    let s = self.move_ctl.scene_offset + (3 - row);
                    self.project.tracks.get(self.move_ctl.track_offset + col).and_then(|t| t.slots.get(s)?.as_ref()).map(|c| c.name.clone()).unwrap_or_default()
                })
                .collect(),
        }
    }

    /// The clip the steps edit: the selected one on the track, else the one
    /// playing, else the one in the selected scene. With `make`, a bar long
    /// one is made (and played) in the selected scene when there is none.
    fn move_clip(&mut self, make: bool) -> Option<ClipSel> {
        let t = self.move_track()?;
        let track = &self.project.tracks[t];
        if track.kind != TrackKind::Midi {
            return None;
        }
        let midi = |s: usize| matches!(track.slots.get(s), Some(Some(c)) if is_midi(c));
        if let Some(ClipSel::Slot { track: st, scene }) = self.sel_clip {
            if st == t && midi(scene) {
                return Some(ClipSel::Slot { track: t, scene });
            }
        }
        let playing = self.status().tracks.get(t).map(|ts| ts.playing_slot.load(Ordering::Relaxed)).unwrap_or(-1);
        if playing >= 0 && midi(playing as usize) {
            return Some(ClipSel::Slot { track: t, scene: playing as usize });
        }
        let s = self.sel_scene.min(self.project.scenes.len().saturating_sub(1));
        if midi(s) {
            return Some(ClipSel::Slot { track: t, scene: s });
        }
        if !make || track.slots.get(s).is_none_or(|c| c.is_some()) {
            return None;
        }
        let id = self.project.alloc_id();
        let bar = self.project.beats_per_bar_f();
        let tr = &mut self.project.tracks[t];
        tr.slots[s] = Some(Clip::new_midi(id, "Steps", tr.color, bar));
        let tid = tr.id;
        self.cmd(Command::LaunchClip { track: tid, slot: s });
        let sel = ClipSel::Slot { track: t, scene: s };
        self.sel_clip = Some(sel);
        Some(sel)
    }

    /// The scene of the clip the steps show (without making one).
    fn move_clip_scene(&self) -> Option<usize> {
        let t = self.move_track()?;
        let track = &self.project.tracks[t];
        let midi = |s: usize| matches!(track.slots.get(s), Some(Some(c)) if is_midi(c));
        if let Some(ClipSel::Slot { track: st, scene }) = self.sel_clip {
            if st == t && midi(scene) {
                return Some(scene);
            }
        }
        let playing = self.status().tracks.get(t).map(|ts| ts.playing_slot.load(Ordering::Relaxed)).unwrap_or(-1);
        if playing >= 0 && midi(playing as usize) {
            return Some(playing as usize);
        }
        let s = self.sel_scene.min(self.project.scenes.len().saturating_sub(1));
        midi(s).then_some(s)
    }

    /// The pages of the selected track's chain the knobs can turn.
    pub(crate) fn move_pages(&self) -> Vec<Page> {
        let Some(t) = self.move_track() else { return Vec::new() };
        let track = &self.project.tracks[t];
        let mut v = Vec::new();
        if track.instrument.as_ref().and_then(krabton_engine::modulation::instrument_specs).is_some() {
            v.push(Page::Instrument);
        }
        if track.instrument.as_ref().is_some_and(|i| i.is_drums()) {
            v.push(Page::Drum);
        }
        v.extend((0..track.midi_devices.len()).map(Page::Midi));
        v.extend((0..track.devices.len()).map(Page::Fx));
        v.push(Page::Mixer);
        v
    }

    fn move_page(&self) -> Option<Page> {
        let pages = self.move_pages();
        pages.get(self.move_ctl.page.min(pages.len().saturating_sub(1))).copied()
    }

    /// The chosen drum of a drum rack.
    fn move_drum(&self, t: usize) -> Option<&krabton_engine::DrumPad> {
        match &self.project.tracks.get(t)?.instrument {
            Some(Instrument::Drums { pads, .. }) => pads.get(self.sel_pad),
            _ => None,
        }
    }

    fn page_name(&self, t: usize, page: Page) -> String {
        let track = &self.project.tracks[t];
        match page {
            Page::Instrument => track.instrument.as_ref().map(|i| i.display_name()).unwrap_or_default(),
            Page::Drum => self.move_drum(t).map(|d| d.name.clone()).unwrap_or_default(),
            Page::Midi(k) => track.midi_devices.get(k).map(|d| d.kind.name().to_string()).unwrap_or_default(),
            Page::Fx(k) => track.devices.get(k).map(|d| d.kind.name().to_string()).unwrap_or_default(),
            Page::Mixer => "Mixer".into(),
        }
    }

    fn page_specs(&self, t: usize, page: Page) -> &'static [ParamSpec] {
        let track = &self.project.tracks[t];
        match page {
            Page::Instrument => track.instrument.as_ref().and_then(krabton_engine::modulation::instrument_specs).unwrap_or(&[]),
            Page::Midi(k) => track.midi_devices.get(k).map(|d| d.kind.specs()).unwrap_or(&[]),
            Page::Fx(k) => track.devices.get(k).map(|d| d.kind.specs()).unwrap_or(&[]),
            Page::Drum => &DRUM_SPECS,
            Page::Mixer => &[],
        }
    }

    fn page_values(&mut self, t: usize, page: Page) -> Option<&mut Vec<f32>> {
        let track = &mut self.project.tracks[t];
        match page {
            Page::Instrument => track.instrument.as_mut().and_then(krabton_engine::modulation::instrument_params_mut),
            Page::Midi(k) => track.midi_devices.get_mut(k).map(|d| &mut d.params),
            Page::Fx(k) => track.devices.get_mut(k).map(|d| &mut d.params),
            Page::Drum | Page::Mixer => None,
        }
    }

    fn page_value(&self, t: usize, page: Page, i: usize) -> Option<f32> {
        let track = &self.project.tracks[t];
        match page {
            Page::Instrument => track.instrument.as_ref().and_then(krabton_engine::modulation::instrument_params).and_then(|p| p.get(i).copied()),
            Page::Midi(k) => track.midi_devices.get(k).and_then(|d| d.params.get(i).copied()),
            Page::Fx(k) => track.devices.get(k).and_then(|d| d.params.get(i).copied()),
            Page::Drum => self.move_drum(t).map(|d| drum_values(d)[i.min(7)]),
            Page::Mixer => None,
        }
    }

    /// The parameters (by index) the eight knobs turn on a page.
    fn page_knobs(&self, t: usize, page: Page) -> Vec<usize> {
        if page == Page::Drum {
            // A and B only where the drum's voice has them.
            let macros = self.move_drum(t).map(|d| d.voice.macros()).unwrap_or([None, None]);
            return (0..8).filter(|&k| !(5..7).contains(&k) || macros[k - 5].is_some()).collect();
        }
        let shown: Vec<usize> = (0..self.page_specs(t, page).len()).filter(|&i| self.page_specs(t, page)[i].shown()).collect();
        let banks = shown.len().div_ceil(8).max(1);
        let b = self.move_ctl.bank.min(banks - 1);
        shown.into_iter().skip(b * 8).take(8).collect()
    }

    /// A track's mixer as eight knobs: volume, pan and the sends.
    fn mixer_knobs(&self, t: usize) -> Vec<(String, String, f32)> {
        let m = &self.project.tracks[t].mixer;
        let mut v = vec![("Volume".to_string(), format!("{:.1} dB", m.volume_db), crate::widgets::db_to_fader(m.volume_db))];
        let pan = if m.pan.abs() < 0.005 { "C".to_string() } else { format!("{}{}", if m.pan < 0.0 { "L" } else { "R" }, (m.pan.abs() * 50.0).round()) };
        v.push(("Pan".into(), pan, (m.pan + 1.0) * 0.5));
        for (k, r) in self.project.returns.iter().enumerate().take(6) {
            let s = m.sends.get(k).copied().unwrap_or(0.0);
            v.push((format!("Send {}", (b'A' + k as u8) as char), format!("{} · {:.0}%", r.name, s * 100.0), s));
        }
        v
    }

    fn mixer_turn(&mut self, t: usize, k: usize, ticks: i32, fine: bool) {
        let returns = self.project.returns.len();
        let m = &mut self.project.tracks[t].mixer;
        match k {
            0 => m.volume_db = crate::widgets::fader_to_db(turn_unit(crate::widgets::db_to_fader(m.volume_db), ticks, fine)),
            1 => m.pan = (m.pan + ticks as f32 * if fine { 2.0 / 1024.0 } else { 2.0 / 128.0 }).clamp(-1.0, 1.0),
            k if k - 2 < returns.min(6) => {
                if m.sends.len() < returns {
                    m.sends.resize(returns, 0.0);
                }
                m.sends[k - 2] = turn_unit(m.sends[k - 2], ticks, fine);
            }
            _ => {}
        }
    }

    /// What the knobs turn now: a held row button's track's mixer, else the
    /// selected track's page.
    fn move_knob_target(&self) -> Option<(usize, Page)> {
        let t = self.move_track()?;
        for k in 0..4u8 {
            if self.move_ctl.holds(Button::Row(k)) && self.move_mode() != PadMode::Session {
                let r = t / 4 * 4 + k as usize;
                if r < self.project.tracks.len() {
                    return Some((r, Page::Mixer));
                }
            }
        }
        Some((t, self.move_page()?))
    }

    /// What the Move's screen shows.
    pub(crate) fn move_screen(&self) -> Screen {
        let hot = match self.move_ctl.hud_knob {
            Some(Knob::Enc(k)) => Some(k as usize),
            _ => None,
        };
        if self.move_ctl.hud_knob == Some(Knob::Volume) {
            let (name, db) = match self.move_volume_target() {
                Some(t) => (self.project.tracks[t].name.clone(), self.project.tracks[t].mixer.volume_db),
                None => ("Master".to_string(), self.project.master.volume_db),
            };
            return Screen { title: name, sub: "Volume".into(), params: vec![("Volume".into(), format!("{db:.1} dB"), crate::widgets::db_to_fader(db))], hot: Some(0) };
        }
        let Some((t, page)) = self.move_knob_target() else {
            return Screen { title: "No track".into(), sub: "Select a track".into(), params: Vec::new(), hot: None };
        };
        let tname = self.project.tracks[t].name.clone();
        if page == Page::Mixer {
            return Screen { title: tname, sub: "Mixer".into(), params: self.mixer_knobs(t), hot };
        }
        let specs = self.page_specs(t, page);
        let knobs = self.page_knobs(t, page);
        let params = knobs
            .iter()
            .map(|&i| {
                let v = self.page_value(t, page, i).unwrap_or(specs[i].default);
                let macros = self.move_drum(t).map(|d| d.voice.macros()).unwrap_or([None, None]);
                let name = match (page, i) {
                    (Page::Drum, 5 | 6) => macros[i - 5].unwrap_or(specs[i].name),
                    _ => specs[i].name,
                };
                (name.to_string(), specs[i].format(v), specs[i].to_norm(v))
            })
            .collect();
        let shown = specs.iter().filter(|s| s.shown()).count();
        let banks = shown.div_ceil(8).max(1);
        let bank = self.move_ctl.bank.min(banks - 1);
        let sub = if banks > 1 { format!("{tname} · {}/{banks}", bank + 1) } else { tname };
        Screen { title: self.page_name(t, page), sub, params, hot }
    }

    /// The track the volume knob turns (None: the master).
    fn move_volume_target(&self) -> Option<usize> {
        let t = self.move_track();
        for k in 0..4u8 {
            if self.move_ctl.holds(Button::Row(k)) {
                let r = t.unwrap_or(0) / 4 * 4 + k as usize;
                return (r < self.project.tracks.len()).then_some(r);
            }
        }
        if self.move_ctl.holds(Button::Shift) {
            return t;
        }
        None
    }

    /// Polls the Move, handles what it sent, and lights it.
    pub(crate) fn move_frame(&mut self) {
        let events: Vec<MoveEvent> = self.move_ctl.events.as_ref().map(|e| e.try_iter().collect()).unwrap_or_default();
        for e in events {
            match e {
                MoveEvent::Connected { input, output } => {
                    self.toast(if output.is_some() { "Ableton Move connected".to_string() } else { "Ableton Move connected (its lights would not open)".to_string() });
                    self.move_ctl.connected = Some((input, output));
                    self.move_ctl.sent = Frame::unknown();
                }
                MoveEvent::Gone(name) => {
                    if self.move_ctl.connected.as_ref().is_some_and(|c| c.0 == name) {
                        self.move_ctl.connected = None;
                        self.toast("Ableton Move disconnected");
                    }
                    self.move_release_all();
                }
                MoveEvent::Error(e) => self.toast(e),
                MoveEvent::Ports { inputs, outputs } => {
                    self.move_ctl.inputs = inputs;
                    self.move_ctl.outputs = outputs;
                }
            }
        }
        let ins: Vec<MoveIn> = self.move_ctl.rx.try_iter().collect();
        for m in ins {
            self.move_input(m);
        }
        // The session's window follows the selected track.
        if let Some(t) = self.move_track() {
            let o = &mut self.move_ctl.track_offset;
            if t < *o {
                *o = t;
            } else if t >= *o + 8 {
                *o = t - 7;
            }
        }
        self.move_ctl.leds = self.move_render();
        self.move_ctl.flush();
    }

    /// The lights for the state now.
    pub(crate) fn move_render(&self) -> Frame {
        let status = self.status();
        let slots = (0..self.project.tracks.len())
            .map(|t| status.tracks.get(t).map(|ts| (ts.playing_slot.load(Ordering::Relaxed), ts.queued_slot.load(Ordering::Relaxed))).unwrap_or((-1, -1)))
            .collect();
        let track = self.move_track();
        let clip = self.move_clip_scene();
        let clip_pos = track.zip(clip).and_then(|(t, s)| self.clip_play_pos(ClipSel::Slot { track: t, scene: s }));
        let mode = self.move_mode();
        let knobs = match self.move_knob_target() {
            Some((t, Page::Mixer)) => self.mixer_knobs(t).len(),
            Some((t, page)) => self.page_knobs(t, page).len(),
            None => 0,
        };
        let pages = self.move_pages().len();
        let ns = self.project.scenes.len();
        let session = mode == PadMode::Session;
        let st = &self.move_ctl;
        let touched = st.touched[..8].iter().position(|t| *t);
        let look = Look {
            project: &self.project,
            mode,
            track,
            slots,
            clip,
            clip_pos,
            pitches: self.move_pitches(),
            held: st.pads_held.values().map(|v| v.0).collect(),
            drum: self.sel_pad,
            vel_pad: st.vel_pad,
            step_page: st.step_page,
            track_offset: st.track_offset,
            scene_offset: st.scene_offset,
            playing: status.playing.load(Ordering::Relaxed),
            recording: self.recording,
            blink: crate::ui::session::blink(),
            held_buttons: st.held.clone(),
            test_page: st.test_page,
            knobs,
            touched,
            can_up: if session { st.scene_offset > 0 } else { true },
            can_down: if session { st.scene_offset + 4 < ns } else { true },
            can_left: st.page > 0,
            can_right: st.page + 1 < pages,
        };
        render(&look)
    }

    /// Something the Move (or the one on screen) sent.
    pub(crate) fn move_input(&mut self, m: MoveIn) {
        match m {
            MoveIn::Other(raw) => self.move_ctl.note_unknown(raw),
            MoveIn::Button { button, down } => self.move_button(button, down),
            MoveIn::Pad { pad, down, vel } => self.move_pad(pad, down, vel),
            MoveIn::PadPressure { value, .. } => {
                if self.move_mode() == PadMode::Touch && !self.move_ctl.pads_held.is_empty() {
                    if let Some(t) = self.move_track() {
                        let track = self.project.tracks[t].id;
                        self.cmd(Command::Touch { track, pressure: value, position: 0.5 });
                    }
                }
            }
            MoveIn::Step { step, down } => {
                if down {
                    self.move_step(step as usize);
                }
            }
            MoveIn::Touch { knob, down } => {
                self.move_ctl.touched[knob.index()] = down;
                if down {
                    self.move_ctl.hud_knob = Some(knob);
                    if let Knob::Enc(k) = knob {
                        self.move_ctl.acc[k as usize] = 0.0;
                    }
                }
                self.move_ctl.hud_at = Some(Instant::now());
            }
            MoveIn::Turn { knob, ticks } => self.move_turn(knob, ticks),
        }
    }

    /// Lets go of the notes the pads hold.
    pub(crate) fn move_release_all(&mut self) {
        for (_, (pitch, ids)) in std::mem::take(&mut self.move_ctl.pads_held) {
            self.note_off(pitch, &ids);
        }
        self.move_ctl.held.clear();
        self.move_ctl.touched = [false; 10];
    }

    fn move_button(&mut self, b: Button, down: bool) {
        let st = &mut self.move_ctl;
        if down {
            if !st.held.contains(&b) {
                st.held.push(b);
            }
            if matches!(b, Button::Mute | Button::Delete | Button::Copy | Button::RecordAudio) {
                st.used = false;
                st.copy_from = None;
            }
        } else {
            st.held.retain(|x| *x != b);
        }
        let shift = st.holds(Button::Shift);
        let used = st.used;
        let mode = self.move_mode();
        if !down {
            // Buttons that also hold: letting go unused does their own thing.
            match b {
                Button::Mute if !used => {
                    if let Some(t) = self.move_track() {
                        let m = &mut self.project.tracks[t].mixer;
                        if shift {
                            m.solo = !m.solo;
                        } else {
                            m.mute = !m.mute;
                        }
                    }
                }
                Button::Copy if !used => self.edit_copy(),
                Button::RecordAudio if !used => {
                    if self.view == View::Tape {
                        let k = self.project.tape.armed.unwrap_or(0);
                        self.tape_arm(k);
                    } else if let Some(t) = self.move_track() {
                        self.project.tracks[t].arm = !self.project.tracks[t].arm;
                    }
                }
                _ => {}
            }
            return;
        }
        match b {
            Button::Row(k) => {
                let k = k as usize;
                if shift {
                    match k {
                        0 => self.move_set_mode(PadMode::Notes),
                        1 => self.move_set_mode(PadMode::Session),
                        2 => self.move_set_mode(PadMode::Touch),
                        _ => self.move_ctl.test_page = Some(0),
                    }
                    return;
                }
                let t = if mode == PadMode::Session { self.move_ctl.track_offset } else { self.move_track().unwrap_or(0) / 4 * 4 };
                let row_track = t + k;
                if self.move_ctl.holds(Button::Mute) {
                    self.move_ctl.used = true;
                    let track = if mode == PadMode::Session { self.move_track() } else { Some(row_track) };
                    if let Some(tr) = track.and_then(|t| self.project.tracks.get_mut(t)) {
                        tr.mixer.mute = !tr.mixer.mute;
                    }
                } else if self.move_ctl.holds(Button::RecordAudio) {
                    self.move_ctl.used = true;
                    if let Some(tr) = self.project.tracks.get_mut(row_track) {
                        tr.arm = !tr.arm;
                    }
                } else if mode == PadMode::Session {
                    let s = self.move_ctl.scene_offset + k;
                    if s < self.project.scenes.len() {
                        self.cmd(Command::LaunchScene(s));
                        self.sel_scene = s;
                    }
                } else if row_track < self.project.tracks.len() {
                    self.move_select_track(row_track);
                }
            }
            Button::Menu => {
                if shift {
                    self.move_ctl.test_page = if self.move_ctl.test_page.is_some() { None } else { Some(0) };
                } else if self.move_ctl.test_page.is_some() {
                    self.move_ctl.test_page = None;
                } else {
                    let next = match mode {
                        PadMode::Notes | PadMode::Drums => PadMode::Session,
                        PadMode::Session => PadMode::Touch,
                        PadMode::Touch => PadMode::Notes,
                    };
                    self.move_set_mode(next);
                }
            }
            Button::Play => {
                if shift {
                    self.cmd(Command::SetPosition(0.0));
                } else {
                    self.toggle_play();
                }
            }
            Button::Record => {
                if shift {
                    self.auto_arm = !self.auto_arm;
                    self.toast(if self.auto_arm { "Automation Arm on: knobs turned while clips play are recorded" } else { "Automation Arm off" });
                } else {
                    self.toggle_record();
                }
            }
            Button::Loop => {
                if shift {
                    self.project.metronome = !self.project.metronome;
                } else {
                    self.project.loop_enabled = !self.project.loop_enabled;
                }
            }
            Button::Capture => {
                if let Some(t) = self.move_track() {
                    self.touch_capture(t);
                }
            }
            Button::Undo => {
                if shift {
                    self.redo();
                } else {
                    self.undo();
                }
            }
            Button::Up | Button::Down => {
                let up = b == Button::Up;
                if let Some(p) = self.move_ctl.test_page.as_mut() {
                    *p = if up { (*p + 3) % 4 } else { (*p + 1) % 4 };
                    return;
                }
                if shift {
                    // The page of steps.
                    let pages = self.move_step_pages();
                    let sp = &mut self.move_ctl.step_page;
                    *sp = if up { sp.saturating_sub(1) } else { (*sp + 1).min(pages - 1) };
                    return;
                }
                match mode {
                    PadMode::Session => {
                        let ns = self.project.scenes.len();
                        let so = &mut self.move_ctl.scene_offset;
                        *so = if up { so.saturating_sub(4) } else { (*so + 4).min(ns.saturating_sub(4)) };
                    }
                    PadMode::Touch => {
                        if let Some(t) = self.move_track() {
                            let d = self.touch_device(t);
                            let base = krabton_engine::sequencers::touch::BASE;
                            if let Some(v) = self.project.tracks[t].midi_devices[d].params.get_mut(base) {
                                *v = (*v + if up { 12.0 } else { -12.0 }).clamp(12.0, 96.0);
                            }
                        }
                    }
                    _ => self.kb_octave = (self.kb_octave + if up { 1 } else { -1 }).clamp(-1, 8),
                }
            }
            Button::Left | Button::Right => {
                let right = b == Button::Right;
                let st = &mut self.move_ctl;
                if shift {
                    st.bank = if right { st.bank + 1 } else { st.bank.saturating_sub(1) };
                } else {
                    st.page = if right { st.page + 1 } else { st.page.saturating_sub(1) };
                    st.bank = 0;
                }
                let pages = self.move_pages().len();
                self.move_ctl.page = self.move_ctl.page.min(pages.saturating_sub(1));
                if let Some((t, page)) = self.move_knob_target() {
                    let banks = self.page_specs(t, page).iter().filter(|s| s.shown()).count().div_ceil(8).max(1);
                    self.move_ctl.bank = self.move_ctl.bank.min(banks - 1);
                }
                self.move_ctl.hud_knob = None;
                self.move_ctl.hud_at = Some(Instant::now());
            }
            Button::Jog => {
                if mode == PadMode::Session {
                    self.cmd(Command::LaunchScene(self.sel_scene));
                } else if let (Some(t), Some(s)) = (self.move_track(), self.move_clip_scene()) {
                    let id = self.project.tracks[t].id;
                    let playing = self.status().tracks.get(t).is_some_and(|ts| ts.playing_slot.load(Ordering::Relaxed) == s as i32);
                    self.cmd(if playing { Command::StopTrack { track: id } } else { Command::LaunchClip { track: id, slot: s } });
                }
            }
            Button::Mute | Button::Delete | Button::Copy | Button::RecordAudio | Button::Shift => {}
        }
    }

    pub(crate) fn move_set_mode(&mut self, m: PadMode) {
        self.move_release_pads();
        self.move_ctl.mode = m;
        self.move_ctl.test_page = None;
        if m == PadMode::Touch {
            // The Touch view plays what the Move's plates play.
            if let Some(t) = self.touch_track() {
                self.touch_device(t);
            }
        }
    }

    fn move_release_pads(&mut self) {
        for (_, (pitch, ids)) in std::mem::take(&mut self.move_ctl.pads_held) {
            self.note_off(pitch, &ids);
        }
        self.move_ctl.vel_pad = None;
    }

    fn move_select_track(&mut self, t: usize) {
        self.move_release_pads();
        self.sel_more.clear();
        self.sel_track = TrackSel::Track(t);
        self.move_ctl.page = 0;
        self.move_ctl.bank = 0;
        self.move_ctl.step_page = 0;
        self.sel_pad = self.sel_pad.min(self.move_kit().len().saturating_sub(1));
        if self.detail == DetailTab::Clip {
            self.sel_clip = None;
        }
    }

    /// How many pages of sixteen steps the clip has.
    fn move_step_pages(&self) -> usize {
        let len = self.move_track().zip(self.move_clip_scene()).and_then(|(t, s)| self.project.tracks[t].slots[s].as_ref().map(|c| c.length)).unwrap_or(4.0);
        ((len / STEP).round() as usize).div_ceil(16).max(1)
    }

    /// A note played on the pads: it sounds, Record takes it, Capture remembers it.
    fn move_play(&mut self, pad: u8, pitch: u8, vel: f32, drums: bool) {
        let played = if drums {
            let Some(t) = self.move_track() else { return };
            let ids = self.preview_note_on(t, pitch, vel);
            self.rec_note_on(t, pitch, vel);
            (pitch, ids)
        } else {
            self.note_on_from(None, pitch, vel)
        };
        for id in &played.1 {
            if let Some(ti) = self.project.track_index(*id) {
                self.capture_remember(ti, played.0, vel);
            }
        }
        self.move_ctl.last_pitch = Some(played.0);
        self.move_ctl.last_vel = vel;
        if let Some((p, ids)) = self.move_ctl.pads_held.insert(pad, played) {
            self.note_off(p, &ids);
        }
    }

    fn move_pad(&mut self, pad: u8, down: bool, vel: f32) {
        if pad >= 32 {
            return;
        }
        if !down {
            if let Some((p, ids)) = self.move_ctl.pads_held.remove(&pad) {
                self.note_off(p, &ids);
            }
            if self.move_mode() == PadMode::Drums && (pad % 8) >= 4 {
                self.move_ctl.vel_pad = None;
            }
            return;
        }
        if let Some(page) = self.move_ctl.test_page {
            let i = page * 32 + pad as usize;
            let c = PALETTE[i];
            self.toast(format!("Palette index {i}: Krabton guesses #{:02x}{:02x}{:02x}", c[0], c[1], c[2]));
            return;
        }
        let (col, row) = ((pad % 8) as usize, (pad / 8) as usize);
        let deleting = self.move_ctl.holds(Button::Delete);
        match self.move_mode() {
            PadMode::Notes => {
                let pitch = keys_pitch(&self.project.sounding_key(), self.kb_octave, pad);
                if deleting {
                    self.move_ctl.used = true;
                    self.move_clear_pitch(pitch);
                    return;
                }
                self.move_play(pad, pitch, vel.max(0.05), false);
            }
            PadMode::Drums => {
                let kit = self.move_kit();
                if col < 4 {
                    let k = row * 4 + col;
                    let Some(&(note, _)) = kit.get(k) else { return };
                    self.sel_pad = k;
                    if deleting {
                        self.move_ctl.used = true;
                        self.move_clear_pitch(note);
                        return;
                    }
                    self.move_play(pad, note, vel.max(0.05), true);
                } else {
                    let v = row * 4 + col - 4;
                    let Some(&(note, _)) = kit.get(self.sel_pad) else { return };
                    self.move_ctl.vel_pad = Some(v);
                    self.move_play(pad, note, (v + 1) as f32 / 16.0, true);
                }
            }
            PadMode::Session => {
                let t = self.move_ctl.track_offset + col;
                let s = self.move_ctl.scene_offset + (3 - row);
                let Some(track) = self.project.tracks.get(t) else { return };
                if s >= track.slots.len() {
                    return;
                }
                let id = track.id;
                let has = track.slots[s].is_some();
                if deleting {
                    self.move_ctl.used = true;
                    self.delete_clip(ClipSel::Slot { track: t, scene: s });
                } else if self.move_ctl.holds(Button::Copy) {
                    self.move_ctl.used = true;
                    match self.move_ctl.copy_from {
                        Some(CopyFrom::Slot(ft, fs)) => {
                            self.move_slot_clip(crate::app::SlotDrag { track: ft, scene: fs }, (t, s), true);
                            self.move_ctl.copy_from = None;
                        }
                        _ if has => self.move_ctl.copy_from = Some(CopyFrom::Slot(t, s)),
                        _ => {}
                    }
                } else if self.move_ctl.holds(Button::Mute) {
                    self.move_ctl.used = true;
                    self.project.tracks[t].mixer.mute = !self.project.tracks[t].mixer.mute;
                } else if self.move_ctl.holds(Button::Shift) {
                    // Picked, not played.
                    self.sel_track = TrackSel::Track(t);
                    self.sel_scene = s;
                    if has {
                        self.sel_clip = Some(ClipSel::Slot { track: t, scene: s });
                    }
                } else {
                    self.sel_track = TrackSel::Track(t);
                    self.sel_scene = s;
                    if has {
                        self.sel_clip = Some(ClipSel::Slot { track: t, scene: s });
                        self.cmd(Command::LaunchClip { track: id, slot: s });
                    } else {
                        self.cmd(Command::StopTrack { track: id });
                    }
                }
            }
            PadMode::Touch => {
                let Some(t) = self.move_track() else { return };
                if self.project.tracks[t].instrument.is_none() {
                    return;
                }
                if let Some(played) = self.touch_pad(t, col as i32, row as i32, vel.max(0.05)) {
                    self.move_ctl.last_pitch = Some(played.0);
                    self.move_ctl.last_vel = vel;
                    if let Some((p, ids)) = self.move_ctl.pads_held.insert(pad, played) {
                        self.note_off(p, &ids);
                    }
                }
            }
        }
    }

    /// Takes every note of `pitch` out of the clip the steps edit.
    fn move_clear_pitch(&mut self, pitch: u8) {
        let Some(sel) = self.move_clip(false) else { return };
        if let Some(notes) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
            notes.retain(|n| n.pitch != pitch);
        }
    }

    fn move_step(&mut self, i: usize) {
        if let Some(p) = self.move_ctl.test_page.as_mut() {
            if i < 4 {
                *p = i;
            }
            return;
        }
        let shift = self.move_ctl.holds(Button::Shift);
        let mode = self.move_mode();
        if mode == PadMode::Session {
            if shift {
                if let Some(id) = self.project.tracks.get(self.move_ctl.track_offset + i).map(|t| t.id) {
                    self.cmd(Command::StopTrack { track: id });
                }
            } else if i < self.project.scenes.len() {
                self.cmd(Command::LaunchScene(i));
                self.sel_scene = i;
            }
            return;
        }
        let Some(sel) = self.move_clip(true) else { return };
        if shift {
            // The clip that many bars long.
            let len = self.project.beats_per_bar_f() * (i + 1) as f64;
            if let Some(c) = self.clip_mut(sel) {
                c.length = len;
            }
            self.move_ctl.step_page = self.move_ctl.step_page.min(self.move_step_pages() - 1);
            return;
        }
        let s = self.move_ctl.step_page * 16 + i;
        if self.move_ctl.holds(Button::Copy) {
            self.move_ctl.used = true;
            match self.move_ctl.copy_from {
                Some(CopyFrom::Step(from)) => {
                    if let Some(notes) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
                        copy_step(notes, from, s);
                    }
                    self.move_ctl.copy_from = None;
                }
                _ => self.move_ctl.copy_from = Some(CopyFrom::Step(s)),
            }
            return;
        }
        let deleting = self.move_ctl.holds(Button::Delete);
        let (pitches, vel): (Vec<u8>, f32) = match mode {
            PadMode::Drums => {
                let kit = self.move_kit();
                let vel = self.move_ctl.vel_pad.map(|v| (v + 1) as f32 / 16.0).unwrap_or(self.move_ctl.last_vel);
                (kit.get(self.sel_pad).map(|d| vec![d.0]).unwrap_or_default(), vel)
            }
            _ => (self.move_ctl.pads_held.values().map(|v| v.0).collect(), self.move_ctl.last_vel),
        };
        if deleting {
            self.move_ctl.used = true;
        }
        let fallback = self.move_ctl.last_pitch.unwrap_or_else(|| keys_pitch(&self.project.sounding_key(), self.kb_octave, 0));
        let Some(notes) = self.clip_mut(sel).and_then(|c| c.notes_mut()) else { return };
        if deleting {
            clear_step(notes, s, None);
        } else if !pitches.is_empty() {
            toggle_step(notes, s, &pitches, vel);
        } else if step_has(notes, s, None) {
            // Nothing held: a lit step goes dark, a dark one gets the last note played.
            clear_step(notes, s, None);
        } else {
            toggle_step(notes, s, &[fallback], vel);
        }
    }

    fn move_turn(&mut self, knob: Knob, ticks: i32) {
        if ticks == 0 {
            return;
        }
        let shift = self.move_ctl.holds(Button::Shift);
        self.move_ctl.hud_at = Some(Instant::now());
        match knob {
            Knob::Enc(k) => {
                let k = k as usize;
                self.move_ctl.hud_knob = Some(knob);
                if self.move_ctl.holds(Button::Row(0)) || self.move_ctl.holds(Button::Row(1)) || self.move_ctl.holds(Button::Row(2)) || self.move_ctl.holds(Button::Row(3)) {
                    self.move_ctl.used = true;
                }
                let Some((t, page)) = self.move_knob_target() else { return };
                if page == Page::Mixer {
                    self.mixer_turn(t, k, ticks, shift);
                    return;
                }
                let knobs = self.page_knobs(t, page);
                let Some(&i) = knobs.get(k) else { return };
                let spec = &self.page_specs(t, page)[i];
                let mut acc = self.move_ctl.acc[k];
                if page == Page::Drum {
                    let sel = self.sel_pad;
                    if let Some(Instrument::Drums { pads, .. }) = self.project.tracks[t].instrument.as_mut() {
                        if let Some(d) = pads.get_mut(sel) {
                            let v = turn_param(spec, drum_values(d)[i], ticks, shift, &mut acc);
                            set_drum_value(d, i, v);
                        }
                    }
                    self.move_ctl.acc[k] = acc;
                    return;
                }
                if let Some(v) = self.page_values(t, page).and_then(|p| p.get_mut(i)) {
                    *v = turn_param(spec, *v, ticks, shift, &mut acc);
                }
                self.move_ctl.acc[k] = acc;
            }
            Knob::Volume => {
                self.move_ctl.hud_knob = Some(knob);
                match self.move_volume_target() {
                    Some(t) => {
                        let m = &mut self.project.tracks[t].mixer;
                        m.volume_db = crate::widgets::fader_to_db(turn_unit(crate::widgets::db_to_fader(m.volume_db), ticks, false));
                    }
                    None => {
                        let m = &mut self.project.master;
                        m.volume_db = crate::widgets::fader_to_db(turn_unit(crate::widgets::db_to_fader(m.volume_db), ticks, false));
                    }
                }
            }
            Knob::Jog => {
                let step = ticks.signum();
                if let Some(p) = self.move_ctl.test_page.as_mut() {
                    *p = (*p as i32 + step).rem_euclid(4) as usize;
                    return;
                }
                if shift {
                    let ns = self.project.scenes.len() as i32;
                    self.sel_scene = (self.sel_scene as i32 + step).clamp(0, ns - 1) as usize;
                    let so = &mut self.move_ctl.scene_offset;
                    if self.sel_scene < *so {
                        *so = self.sel_scene;
                    } else if self.sel_scene >= *so + 4 {
                        *so = self.sel_scene - 3;
                    }
                    self.sel_clip = None;
                    return;
                }
                let nt = self.project.tracks.len() as i32;
                if nt == 0 {
                    return;
                }
                let t = self.move_track().map(|t| t as i32).unwrap_or(-1);
                let next = (t + step).clamp(0, nt - 1) as usize;
                if next as i32 != t {
                    self.move_select_track(next);
                }
            }
        }
    }
}

/// Whether a clip has notes (not audio).
fn is_midi(c: &Clip) -> bool {
    matches!(c.content, krabton_engine::ClipContent::Midi { .. })
}

#[cfg(test)]
mod tests {
    use super::*;
    use krabton_engine::ScaleKind;

    #[test]
    fn the_moves_messages_are_read() {
        assert_eq!(parse(&[0x90, 68, 100]), Some(MoveIn::Pad { pad: 0, down: true, vel: 100.0 / 127.0 }));
        assert_eq!(parse(&[0x90, 99, 0]), Some(MoveIn::Pad { pad: 31, down: false, vel: 0.0 }));
        assert_eq!(parse(&[0x80, 75, 40]), Some(MoveIn::Pad { pad: 7, down: false, vel: 40.0 / 127.0 }));
        assert_eq!(parse(&[0x90, 16, 127]), Some(MoveIn::Step { step: 0, down: true }));
        assert_eq!(parse(&[0x90, 31, 0]), Some(MoveIn::Step { step: 15, down: false }));
        assert_eq!(parse(&[0x90, 3, 127]), Some(MoveIn::Touch { knob: Knob::Enc(3), down: true }));
        assert_eq!(parse(&[0x90, 3, 0]), Some(MoveIn::Touch { knob: Knob::Enc(3), down: false }));
        assert_eq!(parse(&[0x90, 8, 127]), Some(MoveIn::Touch { knob: Knob::Volume, down: true }));
        assert_eq!(parse(&[0x90, 9, 127]), Some(MoveIn::Touch { knob: Knob::Jog, down: true }));
        assert_eq!(parse(&[0xb0, 71, 1]), Some(MoveIn::Turn { knob: Knob::Enc(0), ticks: 1 }));
        assert_eq!(parse(&[0xb0, 78, 127]), Some(MoveIn::Turn { knob: Knob::Enc(7), ticks: -1 }));
        assert_eq!(parse(&[0xb0, 79, 3]), Some(MoveIn::Turn { knob: Knob::Volume, ticks: 3 }));
        assert_eq!(parse(&[0xb0, 14, 125]), Some(MoveIn::Turn { knob: Knob::Jog, ticks: -3 }));
        assert_eq!(parse(&[0xb0, 85, 127]), Some(MoveIn::Button { button: Button::Play, down: true }));
        assert_eq!(parse(&[0xb0, 49, 0]), Some(MoveIn::Button { button: Button::Shift, down: false }));
        assert_eq!(parse(&[0xb0, 43, 127]), Some(MoveIn::Button { button: Button::Row(0), down: true }));
        assert_eq!(parse(&[0xb0, 40, 127]), Some(MoveIn::Button { button: Button::Row(3), down: true }));
        assert_eq!(parse(&[0xa0, 70, 64]), Some(MoveIn::PadPressure { pad: 2, value: 64.0 / 127.0 }));
        assert_eq!(parse(&[0xb0, 20, 5]), Some(MoveIn::Other([0xb0, 20, 5])), "unknown: kept to show");
        assert_eq!(parse(&[0x90, 50, 5]), Some(MoveIn::Other([0x90, 50, 5])));
        assert_eq!(parse(&[0xf8]), None, "the clock");
        assert_eq!(parse(&[]), None);
        assert_eq!((rel(0), rel(1), rel(63), rel(65), rel(127), rel(64)), (0, 1, 63, -63, -1, -64));
        for b in Button::ALL {
            assert_eq!(Button::from_cc(b.cc()), Some(b), "{b:?}");
        }
    }

    #[test]
    fn the_moves_ports_are_found() {
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let ports = names(&["Keystation 49", "Ableton Move User Port", "Ableton Move Live Port"]);
        assert_eq!(best_move_port(&ports).as_deref(), Some("Ableton Move Live Port"));
        assert_eq!(best_move_port(&names(&["MIDIIN2 (Ableton Move)", "Ableton Move"])).as_deref(), Some("MIDIIN2 (Ableton Move)"));
        assert_eq!(best_move_port(&names(&["Ableton Move User Port"])), None, "the user port is not for Krabton");
        assert_eq!(best_move_port(&names(&["Keystation 49"])), None);
        assert!(is_move_port("Ableton Move Live Port") && !is_move_port("Moving Coil Synth"));
    }

    #[test]
    fn only_lights_that_changed_go_out_a_burst_at_a_time() {
        let mut sent = Frame::unknown();
        let mut f = Frame::dark();
        let first = f.diff(&mut sent, 1000);
        // Everything the Move has goes out once: 48 notes, the buttons, 8 knobs, 16 step lights.
        assert_eq!(first.len(), 48 + 19 + 8 + 16);
        assert!(f.diff(&mut sent, 1000).is_empty(), "nothing changed: nothing sent");
        f.set_pad(5, 33);
        f.set_button(Button::Play, GREEN);
        f.set_step(15, WHITE);
        let msgs = f.diff(&mut sent, 1000);
        assert_eq!(msgs, vec![[0x90, 31, WHITE], [0x90, 73, 33], [0xb0, 85, GREEN]]);
        // A burst at most: the rest next time.
        for i in 0..32 {
            f.set_pad(i, 7);
        }
        let a = f.diff(&mut sent, 20);
        let b = f.diff(&mut sent, 20);
        assert_eq!((a.len(), b.len()), (20, 12), "pad 5 was not 7 yet either");
        assert!(f.diff(&mut sent, 20).is_empty());
    }

    #[test]
    fn the_palette_is_found_nearest() {
        assert_eq!(nearest([0, 0, 0]), 0);
        assert_eq!(PALETTE[nearest([0xff, 0xff, 0xff]) as usize], [0xff, 0xff, 0xff]);
        assert_eq!(nearest([0xff, 0x00, 0x00]), 1);
        let blue = PALETTE[nearest([0x10, 0x30, 0xf0]) as usize];
        assert!(blue[2] > 0xc0 && blue[0] < 0x40, "{blue:?}");
        let dark = PALETTE[nearest(dim([0xff, 0x00, 0x00], 0.15)) as usize];
        assert!(dark[0] < 0x60 && dark[0] > 0, "a dim red: {dark:?}");
        assert_eq!(PALETTE[WHITE as usize], [0xff, 0xff, 0xff]);
    }

    #[test]
    fn notes_mode_plays_in_key_rows_a_fourth_apart() {
        let c_major = Scale::new(0, ScaleKind::Major);
        let row0: Vec<u8> = (0..8).map(|i| keys_pitch(&c_major, 3, i)).collect();
        assert_eq!(row0, vec![48, 50, 52, 53, 55, 57, 59, 60], "the bottom row: C major from C3");
        assert_eq!(keys_pitch(&c_major, 3, 8), 53, "a row up: three steps (a fourth) up");
        assert_eq!(keys_pitch(&c_major, 3, 24), 64, "the top row starts nine steps up");
        assert_eq!(keys_pitch(&c_major, 4, 0), 60, "an octave up");
        let a_minor = Scale::new(9, ScaleKind::Minor);
        let all: Vec<u8> = (0..32).map(|i| keys_pitch(&a_minor, 3, i)).collect();
        assert_eq!(all[0], 57, "the root bottom left");
        assert!(all.iter().all(|p| [9, 11, 0, 2, 4, 5, 7].contains(&(p % 12))), "all in A minor: {all:?}");
    }

    #[test]
    fn steps_write_notes_into_the_clip() {
        let mut notes = Vec::new();
        assert!(toggle_step(&mut notes, 4, &[36], 0.9));
        assert_eq!(notes, vec![Note::new(36, 1.0, STEP, 0.9)], "step 5 is beat two");
        assert!(step_has(&notes, 4, Some(36)) && !step_has(&notes, 4, Some(38)) && !step_has(&notes, 5, None));
        // A chord held: what is missing goes on.
        assert!(toggle_step(&mut notes, 4, &[36, 40], 0.5));
        assert_eq!(notes.len(), 2);
        // All there: they come off.
        assert!(!toggle_step(&mut notes, 4, &[36, 40], 0.5));
        assert!(notes.is_empty());
        toggle_step(&mut notes, 0, &[60, 64], 0.8);
        toggle_step(&mut notes, 17, &[62], 0.8);
        assert!((notes.last().unwrap().start - 4.25).abs() < 1e-9, "the second page");
        copy_step(&mut notes, 0, 8);
        assert!(step_has(&notes, 8, Some(60)) && step_has(&notes, 8, Some(64)) && step_has(&notes, 0, Some(60)));
        clear_step(&mut notes, 0, Some(64));
        assert!(step_has(&notes, 0, Some(60)) && !step_has(&notes, 0, Some(64)));
        clear_step(&mut notes, 8, None);
        assert!(!step_has(&notes, 8, None));
        // A note played in a little late still sits on its step.
        let late = vec![Note::new(50, 0.26, 0.2, 0.8)];
        assert!(step_has(&late, 1, Some(50)) && !step_has(&late, 0, None));
    }

    #[test]
    fn knobs_turn_parameters_in_steps_that_fit() {
        let lin = ParamSpec::lin("Cutoff", "", 0.0, 1.0, 0.5, krabton_engine::params::Unit::None);
        let mut acc = 0.0;
        assert!((turn_param(&lin, 0.5, 1, false, &mut acc) - (0.5 + 1.0 / 128.0)).abs() < 1e-6);
        assert!((turn_param(&lin, 0.5, -4, true, &mut acc) - (0.5 - 4.0 / 1024.0)).abs() < 1e-6, "finely with Shift");
        assert_eq!(turn_param(&lin, 0.99, 10, false, &mut acc), 1.0, "it stops at the end");
        let log = ParamSpec::log("Freq", "", 20.0, 20000.0, 1000.0, krabton_engine::params::Unit::None);
        let up = turn_param(&log, 1000.0, 1, false, &mut acc);
        assert!((log.to_norm(up) - log.to_norm(1000.0) - 1.0 / 128.0).abs() < 1e-4, "along the knob's own curve");
        let choice = ParamSpec::choice("Wave", "", &["Saw", "Square", "Sine", "Noise"], 0);
        let mut acc = 0.0;
        let mut v = 0.0;
        for _ in 0..2 {
            v = turn_param(&choice, v, 1, false, &mut acc);
        }
        assert_eq!(v, 0.0, "two clicks: not yet");
        v = turn_param(&choice, v, 1, false, &mut acc);
        assert_eq!(v, 1.0, "the third: the next");
        v = turn_param(&choice, v, 30, false, &mut acc);
        assert_eq!(v, 3.0, "no further than the last");
        let toggle = ParamSpec::toggle("On", "", false);
        let mut acc = 0.0;
        assert_eq!(turn_param(&toggle, 0.0, 3, false, &mut acc), 1.0);
        assert_eq!(turn_param(&toggle, 1.0, -3, false, &mut acc), 0.0);
        let int = ParamSpec::int("Note", "", 0.0, 127.0, 60.0, krabton_engine::params::Unit::None);
        let mut acc = 0.0;
        assert_eq!(turn_param(&int, 60.0, 1, false, &mut acc), 61.0, "many steps: one a click");
        assert!((turn_unit(0.5, 64, false) - 1.0).abs() < 1e-6 && turn_unit(0.0, -5, false) == 0.0);
    }

    fn look(p: &Project, mode: PadMode, track: Option<usize>) -> Look<'_> {
        Look {
            project: p,
            mode,
            track,
            slots: vec![(-1, -1); p.tracks.len()],
            clip: None,
            clip_pos: None,
            pitches: std::array::from_fn(|i| keys_pitch(&p.sounding_key(), 3, i as u8)),
            held: Vec::new(),
            drum: 0,
            vel_pad: None,
            step_page: 0,
            track_offset: 0,
            scene_offset: 0,
            playing: false,
            recording: false,
            blink: true,
            held_buttons: Vec::new(),
            test_page: None,
            knobs: 8,
            touched: None,
            can_up: true,
            can_down: true,
            can_left: false,
            can_right: true,
        }
    }

    #[test]
    fn the_lights_show_each_mode() {
        let mut p = Project::new_default();
        p.scale = Scale::new(0, ScaleKind::Major);
        p.tracks[1].color = [0xff, 0x00, 0x00];
        // Notes: roots in the track's colour, the rest dim, held notes white.
        let mut l = look(&p, PadMode::Notes, Some(1));
        l.held = vec![50];
        let f = render(&l);
        assert_eq!(f.pad(0), nearest([0xff, 0, 0]), "C: a root");
        assert_eq!(f.pad(1), WHITE, "D: held");
        assert_eq!(f.pad(2), nearest(dim([0xff, 0xff, 0xff], 0.16)), "E: in the key");
        assert_eq!(f.pad(7), nearest([0xff, 0, 0]), "the next C");
        assert_eq!(f.button(Button::Play), nearest(dim([0x20, 0xff, 0x40], 0.2)), "stopped: Play dim");
        assert_eq!(f.button(Button::Left), OFF);
        // Drums: the kit bottom left, velocities on the right.
        p.tracks[0].color = [0xff, 0x80, 0x20];
        let mut l = look(&p, PadMode::Drums, Some(0));
        l.drum = 1;
        l.playing = true;
        let f = render(&l);
        let kit = drum_kit(p.tracks[0].instrument.as_ref().unwrap());
        assert!(kit.len() >= 8, "{kit:?}");
        assert_eq!(f.pad(1), nearest(drum_color(&kit[1].1)), "the chosen drum bright");
        assert_eq!(f.pad(0), nearest(dim(drum_color(&kit[0].1), 0.4)));
        assert_ne!(f.pad(4), 0, "the softest velocity");
        assert_eq!(f.pad(31), nearest(p.tracks[0].color), "the loudest");
        assert_eq!(f.button(Button::Play), GREEN);
        // Session: clips in their colours, the one playing pulses.
        let id = p.alloc_id();
        p.tracks[1].slots[0] = Some(Clip::new_midi(id, "A", [0x00, 0x00, 0xff], 4.0));
        let id = p.alloc_id();
        p.tracks[0].slots[1] = Some(Clip::new_midi(id, "B", [0x00, 0xff, 0x00], 4.0));
        let mut l = look(&p, PadMode::Session, Some(1));
        l.slots[1] = (0, -1);
        let lit = render(&l);
        assert_eq!(lit.pad(24 + 1), nearest([0, 0, 0xff]), "track 2, scene 1 (top row): playing, lit");
        assert_eq!(lit.pad(16), nearest(dim([0, 0xff, 0], 0.5)), "track 1, scene 2: there, dim");
        assert_eq!(lit.pad(24), 0, "empty");
        assert_eq!(lit.step(0), GREEN, "scene 1 plays");
        l.blink = false;
        assert_eq!(render(&l).pad(25), WHITE, "and pulses");
        // The colour test: the pads show the indices of the page.
        let mut l = look(&p, PadMode::Notes, Some(1));
        l.test_page = Some(2);
        let f = render(&l);
        assert_eq!((f.pad(0), f.pad(31)), (64, 95));
    }

    #[test]
    fn the_steps_show_the_clip() {
        let mut p = Project::new_default();
        let id = p.alloc_id();
        let mut c = Clip::new_midi(id, "Beat", [0xff, 0, 0], 8.0);
        let kit = drum_kit(p.tracks[0].instrument.as_ref().unwrap());
        let (kick, snare) = (kit[0].0, kit[1].0);
        let notes = c.notes_mut().unwrap();
        toggle_step(notes, 0, &[kick], 1.0);
        toggle_step(notes, 4, &[snare], 1.0);
        toggle_step(notes, 20, &[kick], 1.0);
        p.tracks[0].slots[0] = Some(c);
        p.tracks[0].color = [0xff, 0, 0];
        let mut l = look(&p, PadMode::Drums, Some(0));
        l.clip = Some(0);
        let f = render(&l);
        assert_eq!(f.step(0), nearest([0xff, 0, 0]), "the kick's step");
        assert_eq!(f.step(4), nearest(dim([0xff, 0, 0], 0.25)), "another drum: dim");
        assert_eq!(f.step(1), 0);
        assert_eq!((f.step_led(0), f.step_led(1), f.step_led(2)), (ON, DIM, 0), "two pages, the first shown");
        l.step_page = 1;
        l.playing = true;
        l.clip_pos = Some(4.1);
        let f = render(&l);
        assert_eq!(f.step(4), nearest([0xff, 0, 0]), "step 21 on the second page");
        assert_eq!(f.step(0), WHITE, "the playhead (beat 4.1: step 17)");
        // Shift: the clip's length in bars.
        l.held_buttons = vec![Button::Shift];
        let f = render(&l);
        assert_eq!((f.step(1), f.step(2)), (nearest([0xff, 0, 0]), nearest(dim([0xff, 0, 0], 0.12))));
        assert_eq!(f.button(Button::Shift), ON);
    }
}
