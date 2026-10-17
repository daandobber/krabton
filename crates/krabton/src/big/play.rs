//! Big Picture's Play page, in three ways to play (the sticks):
//!
//! *Pads*: a melodic track gets eight by four pads in the key (each row a
//! fourth up, the roots lit), a drum track a pad per drum with its name.
//! Touch them (with as many fingers as you like), or move over them with the
//! D-pad and hold A; X and Y move an octave. The right stick plays the row
//! the cursor is on, the pad it points at (left the first, up the middle,
//! right the last): in Trigger every new pad plays at once and letting go
//! ends the note, in Hold the note rings on until another pad (or B); L3
//! switches.
//!
//! *Glide*: each stick sweeps a ring of notes in the key, all the way round
//! from the bottom (low) over the left, the top and the right (high), one to
//! four octaves, the left stick an octave below the right. Every note a
//! stick passes plays, the new one before the old one ends (so a mono synth
//! glides), and with Velocity how far the stick is pushed sets how hard each
//! note is struck: a new note waits until the stick stops going out (or
//! reaches the rim), so a quick flick plays hard and a gentle push soft. The
//! rings can be touched too (further out is harder).
//!
//! *XY*: the left stick plays the ring, the right stick is a joystick on two
//! knobs (Joystick modulators, around where the knobs are set): the most
//! telling two of the instrument, or any of its knobs, its effects' knobs,
//! volume or pan.
//!
//! The triggers change the track (Track), bend the notes (Bend: LT down, RT
//! up, one to 24 semitones) or move the track's mod wheel (Mod: a Mod Wheel
//! modulator, made and connected when the track has none). Menu puts the
//! focus on the row of modes (left and right choose, up and down change, A
//! sets, B goes back); Menu again opens the menu. The pads and rings light up
//! with every note the track plays, also from its clips and sequencers.

use super::input::{Frame, Nav};
use super::{fit_text, scrolled_rows, tile, value_bar, Glyph, Sz, TILE, TILE_HI};
use crate::app::{App, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Painter, Pos2, Rect, Sense, Stroke, Ui};
use krabton_engine::engine::Command;
use krabton_engine::modulation::{instrument_specs, ModKind, ModOwner, ModTarget};
use krabton_engine::params::{ParamKind, ParamSpec};
use krabton_engine::Instrument;
use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

/// Who holds a note: the pad cursor, the mouse, a stick, or a finger (by touch id).
pub const HOLD_CURSOR: u64 = u64::MAX;
pub const HOLD_MOUSE: u64 = u64::MAX - 1;
pub const HOLD_STICK: u64 = u64::MAX - 2;
pub const HOLD_LSTICK: u64 = u64::MAX - 3;

/// How the sticks play.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StickMode {
    #[default]
    Pads,
    Glide,
    Xy,
}

impl StickMode {
    pub const ALL: [StickMode; 3] = [StickMode::Pads, StickMode::Glide, StickMode::Xy];

    pub fn name(self) -> &'static str {
        match self {
            StickMode::Pads => "Pads",
            StickMode::Glide => "Glide",
            StickMode::Xy => "XY",
        }
    }

    fn help(self) -> &'static str {
        match self {
            StickMode::Pads => "Pads, and the right stick over the cursor's row",
            StickMode::Glide => "Both sticks play rings of notes in the key: the left an octave below the right",
            StickMode::Xy => "The left stick plays a ring of notes, the right stick moves two knobs (X and Y)",
        }
    }
}

/// What the triggers do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrigMode {
    /// Change the track.
    #[default]
    Track,
    /// Bend the notes: LT down, RT up.
    Bend,
    /// The mod wheel.
    Mod,
}

impl TrigMode {
    pub const ALL: [TrigMode; 3] = [TrigMode::Track, TrigMode::Bend, TrigMode::Mod];

    pub fn name(self) -> &'static str {
        match self {
            TrigMode::Track => "Track",
            TrigMode::Bend => "Bend",
            TrigMode::Mod => "Mod",
        }
    }

    fn help(self) -> &'static str {
        match self {
            TrigMode::Track => "The triggers change the track",
            TrigMode::Bend => "The triggers bend the notes: LT down, RT up (how far: the next button)",
            TrigMode::Mod => "The triggers move the track's mod wheel (a Mod Wheel modulator, connected to the filter when it is new)",
        }
    }
}

/// How far the triggers can bend, in semitones.
pub const BEND_RANGES: [f32; 8] = [1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 12.0, 24.0];

/// What the triggers and sticks send besides notes.
#[derive(Clone, Copy, PartialEq)]
enum Wheel {
    Bend,
    Mod,
    Joy,
}

impl Wheel {
    const ALL: [Wheel; 3] = [Wheel::Bend, Wheel::Mod, Wheel::Joy];

    fn rest(self) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn command(self, track: u64, v: (f32, f32)) -> Command {
        match self {
            Wheel::Bend => Command::PitchBend { track, semis: v.0 },
            Wheel::Mod => Command::ModWheel { track, value: v.0 },
            Wheel::Joy => Command::Joystick { track, x: v.0, y: v.1 },
        }
    }
}

pub struct PlayState {
    pub cursor: (i32, i32),
    pub octave: i32,
    /// Notes held, by who holds them: (the pitch played, the tracks).
    pub held: HashMap<u64, (u8, Vec<u64>)>,
    /// Which pad each holder is on (on a ring: the ring and the note on it).
    pub on_pad: HashMap<u64, (i32, i32)>,
    /// Sticks on their way out to strike a note (Velocity): how far out they
    /// got, where they were last frame, and for how long.
    arming: HashMap<u64, Arm>,
    /// Sticks that struck their note and stay out: they glide.
    gliding: Vec<u64>,
    /// The triggers' last value (they change the track when pulled).
    pub last_turn: f32,
    /// A touch screen was used: its fingers, not the mouse it imitates.
    pub touch: bool,
    /// The sticks hold their note after letting go.
    pub stick_hold: bool,
    pub sticks: StickMode,
    /// How many octaves a ring spans (1..4).
    pub range: i32,
    pub trig_mode: TrigMode,
    /// How far the triggers bend: an index into BEND_RANGES.
    pub bend_range: usize,
    /// How far a stick (or finger) is out sets how hard it strikes.
    pub velocity: bool,
    /// What each wheel sends now: the tracks, and the value.
    wheels: [(Vec<u64>, (f32, f32)); 3],
    /// The finger (or mouse) on the XY pad, and where it holds it.
    xy_finger: Option<(u64, (f32, f32))>,
    /// The row of modes has the focus (Menu on the Play page), on this item.
    pub bar_focus: Option<usize>,
    /// Choosing what a stick axis moves: (the axis, the row).
    pub picker: Option<(usize, usize)>,
}

impl Default for PlayState {
    fn default() -> Self {
        Self {
            cursor: (0, 0),
            octave: 0,
            held: HashMap::new(),
            on_pad: HashMap::new(),
            arming: HashMap::new(),
            gliding: Vec::new(),
            last_turn: 0.0,
            touch: false,
            stick_hold: false,
            sticks: StickMode::Pads,
            range: 2,
            trig_mode: TrigMode::Track,
            bend_range: 1,
            velocity: true,
            wheels: Wheel::ALL.map(|w| (Vec::new(), w.rest())),
            xy_finger: None,
            bar_focus: None,
            picker: None,
        }
    }
}

/// A stick on its way out to strike a note.
#[derive(Clone, Copy, Debug)]
struct Arm {
    peak: f32,
    last: f32,
    secs: f32,
}

impl PlayState {
    /// Lets go of what the sticks were doing (after their notes were released).
    pub fn forget_sticks(&mut self) {
        self.on_pad.clear();
        self.arming.clear();
        self.gliding.clear();
    }

    /// The bend the triggers send now, in semitones.
    pub fn bend(&self) -> f32 {
        self.wheels[Wheel::Bend as usize].1 .0
    }
}

/// The pads of a track: (columns, rows, pitch of each pad, name of each).
fn pads(app: &App, t: usize, octave: i32) -> (i32, i32, Vec<u8>, Vec<String>) {
    let track = &app.project.tracks[t];
    if let Some(Instrument::Drums { pads, .. }) = &track.instrument {
        let n = pads.len().clamp(1, 16);
        let cols = 4;
        let rows = (n as i32 + cols - 1) / cols;
        // The first pad bottom left, as on a drum machine.
        let mut pitches = vec![0u8; (cols * rows) as usize];
        let mut names = vec![String::new(); (cols * rows) as usize];
        for (i, pad) in pads.iter().take(16).enumerate() {
            let (c, r) = (i as i32 % cols, rows - 1 - i as i32 / cols);
            pitches[(r * cols + c) as usize] = pad.note;
            names[(r * cols + c) as usize] = pad.name.clone();
        }
        return (cols, rows, pitches, names);
    }
    let (cols, rows) = (8, 4);
    let scale = app.project.scale;
    let base = (4 + octave) * 12 + scale.root as i32;
    let (abs, _) = scale.locate(base);
    let mut pitches = Vec::new();
    let mut names = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let degree = c + (rows - 1 - r) * 3;
            let p = scale.pitch_at(abs + degree).clamp(0, 127) as u8;
            pitches.push(p);
            names.push(krabton_engine::note_name(p));
        }
    }
    (cols, rows, pitches, names)
}

/// The notes of a ring, low to high, and their names: `range` octaves of
/// the key from the root (a drum track: its drums).
fn ring_notes(app: &App, t: usize, octave: i32, range: i32) -> (Vec<u8>, Vec<String>) {
    if let Some(Instrument::Drums { pads, .. }) = &app.project.tracks[t].instrument {
        return pads.iter().take(16).map(|p| (p.note, p.name.clone())).unzip();
    }
    let scale = app.project.scale;
    let base = (4 + octave) * 12 + scale.root as i32;
    let (abs, _) = scale.locate(base);
    let top = scale.pitch_at(abs) + 12 * range.clamp(1, 4);
    (0..)
        .map(|k| scale.pitch_at(abs + k))
        .take_while(|&p| p <= top)
        .filter(|p| (0..=127).contains(p))
        .map(|p| (p as u8, krabton_engine::note_name(p as u8)))
        .unzip()
}

/// Where on a ring a direction (x right, y up) points: 0 at the bottom,
/// going round over the left, the top and the right to 1 at the bottom again.
pub fn ring_at(x: f32, y: f32) -> f32 {
    ((x.atan2(y) + PI) / TAU).clamp(0.0, 1.0)
}

/// The note at `u` on a ring of `n`, staying on `cur` until well past its
/// edges (also across the bottom, where the ring closes).
pub fn ring_slot(u: f32, n: usize, cur: Option<usize>) -> usize {
    let x = u * n as f32;
    if let Some(c) = cur {
        let d = (x - c as f32).rem_euclid(n as f32);
        if c < n && (d < 1.2 || d > n as f32 - 0.2) {
            return c;
        }
    }
    (x.max(0.0) as usize).min(n.saturating_sub(1))
}

fn ring_point(c: Pos2, rad: f32, u: f32) -> Pos2 {
    let a = -PI + u * TAU;
    c + vec2(a.sin(), -a.cos()) * rad
}

/// Where along a row of pads the right stick points: 0 straight left, 0.5
/// up, 1 straight right; below that the nearer end, and None straight down.
pub fn row_at(x: f32, y: f32) -> Option<f32> {
    let a = x.atan2(y);
    (a.abs() < PI - 0.35).then(|| (a / PI + 0.5).clamp(0.0, 1.0))
}

/// The pad at `u` along a row of `n`, staying on `cur` until well past its edges.
pub fn row_slot(u: f32, n: usize, cur: Option<usize>) -> usize {
    let x = u * n as f32;
    if let Some(c) = cur {
        if c < n && x > c as f32 - 0.2 && x < c as f32 + 1.2 {
            return c;
        }
    }
    (x.max(0.0) as usize).min(n.saturating_sub(1))
}

/// How hard a stick that far out (its magnitude) strikes: softly just past
/// where it starts playing, fully at the rim.
pub fn stick_velocity(mag: f32) -> f32 {
    0.25 + 0.75 * ((mag - 0.5) / 0.43).clamp(0.0, 1.0)
}

fn smooth(spec: &ParamSpec) -> bool {
    matches!(spec.kind, ParamKind::Linear | ParamKind::Log) && spec.shown()
}

/// The two knobs of an instrument a stick moves best (X, Y): its filter and
/// resonance, timbre and colour, ... by name, and otherwise what the names say.
pub fn telling_params(inst: &Instrument) -> [Option<usize>; 2] {
    if let Instrument::Facet { .. } = inst {
        return [Some(krabton_engine::dsp::facet::BLUE), Some(krabton_engine::dsp::facet::GREEN)];
    }
    let Some(specs) = instrument_specs(inst) else { return [None, None] };
    let pair = match inst {
        Instrument::Synth { .. } | Instrument::Sampler { .. } | Instrument::Analog { .. } | Instrument::Bassline { .. } => Some(("Cutoff", "Resonance")),
        Instrument::Waves { .. } => Some(("Position 1", "Cutoff")),
        Instrument::Fm { .. } => Some(("Brightness", "Cutoff")),
        Instrument::Macro { .. } => Some(("Timbre", "Morph")),
        Instrument::Braids { .. } => Some(("Timbre", "Color")),
        Instrument::Elements { .. } => Some(("Brightness", "Position")),
        Instrument::Rings { .. } => Some(("Brightness", "Structure")),
        Instrument::Mono101 { .. } => Some(("Freq", "Res")),
        Instrument::Shakers { .. } => Some(("Brightness", "Ring")),
        Instrument::Voice { .. } => Some(("Morph", "Formant")),
        Instrument::Mallets { .. } | Instrument::Strings { .. } | Instrument::Winds { .. } => Some(("Control 1", "Control 2")),
        Instrument::Organ { .. } => Some(("Drive", "Key Click")),
        Instrument::Monk { .. } => Some(("Vowel", "Head")),
        Instrument::Drone { .. } => Some(("Tone", "Harmonics")),
        Instrument::Guitar { .. } => Some(("Brightness", "Palm Mute")),
        Instrument::WindChimes { .. } => Some(("Wind", "Gusts")),
        Instrument::BeadsSynth { .. } => Some(("Density", "Size")),
        Instrument::Birds { .. } => Some(("Busy", "Pitch")),
        Instrument::Nature { .. } => Some(("Wind", "Rain")),
        Instrument::PanFlute { .. } => Some(("Breath", "Tone")),
        Instrument::Melodica { .. } => Some(("Breath", "Tone")),
        Instrument::Harmonica { .. } => Some(("Hands", "Bends")),
        Instrument::Calculator { .. } => Some(("Tune", "Rhythm Level")),
        Instrument::Console { .. } => Some(("Vibrato", "Vib Rate")),
        Instrument::Handheld { .. } => Some(("Treble", "Vibrato")),
        Instrument::Micro { .. } => Some(("Cutoff", "PWM")),
        Instrument::Warp { .. } => Some(("Warp 1", "Warp 2")),
        Instrument::Hybrid { .. } => Some(("Cutoff 1", "Reso 1")),
        Instrument::Quartet { .. } => Some(("Cutoff A", "Reso A")),
        Instrument::Orchestra { .. } => Some(("Dynamics", "Release")),
        _ => None,
    };
    let named = |n: &str| specs.iter().position(|s| s.name == n && smooth(s));
    let said = |words: &[&str], not: Option<usize>| {
        words.iter().find_map(|w| specs.iter().enumerate().position(|(i, s)| Some(i) != not && smooth(s) && s.name.to_lowercase().contains(w)))
    };
    let x = pair.and_then(|(a, _)| named(a)).or_else(|| said(&["cutoff", "bright", "timbre", "filter", "tone", "colo", "morph", "harmonic"], None));
    let y = pair.and_then(|(_, b)| named(b)).or_else(|| said(&["reso", "morph", "colo", "harmonic", "structure", "position", "drive", "space"], x));
    [x, y]
}

/// What a row of modes can do.
#[derive(Clone, Copy, PartialEq)]
enum ModeAct {
    Sticks(StickMode),
    Trig(TrigMode),
    BendRange,
    Range,
    Hold,
    Velocity,
    Axis(usize),
}

/// A button in the row of modes (or, group 3, under the XY pad).
struct ModeItem {
    label: String,
    lit: bool,
    act: ModeAct,
    help: &'static str,
    group: u8,
    /// Up and down (or the halves of the button) step it.
    stepper: bool,
}

impl App {
    pub(super) fn play_track(&mut self) -> usize {
        let n = self.project.tracks.len();
        let t = match self.sel_track {
            TrackSel::Track(t) if t < n => t,
            _ => 0,
        };
        // The armed track is the one that hears the pads: when another is
        // armed, the arm moves here (in Big Picture; on the desktop the arms
        // stay as they are).
        if self.big.on && self.project.tracks.iter().enumerate().any(|(i, tr)| tr.arm && i != t) {
            for (i, tr) in self.project.tracks.iter_mut().enumerate() {
                tr.arm = i == t;
            }
        }
        t
    }

    /// Plays the track `by` tracks on from the one played now.
    pub(super) fn play_next_track(&mut self, by: i32) {
        let n = self.project.tracks.len() as i32;
        if n == 0 {
            return;
        }
        let t = self.play_track() as i32;
        self.play_switch_to(((t + by).rem_euclid(n)) as usize);
    }

    fn play_switch_to(&mut self, t: usize) {
        self.big_release_notes();
        self.sel_track = TrackSel::Track(t);
        let t = self.play_track();
        if self.big.play.sticks == StickMode::Xy {
            self.ensure_joystick(t);
        }
    }

    pub(super) fn set_sticks(&mut self, mode: StickMode) {
        if self.big.play.sticks == mode {
            return;
        }
        self.big_release_notes();
        self.big.play.sticks = mode;
        if mode == StickMode::Xy && !self.project.tracks.is_empty() {
            let t = self.play_track();
            self.ensure_joystick(t);
        }
    }

    pub(super) fn set_trig_mode(&mut self, mode: TrigMode) {
        if self.big.play.trig_mode == mode {
            return;
        }
        self.big_unbend();
        self.big.play.trig_mode = mode;
        if mode == TrigMode::Mod && !self.project.tracks.is_empty() {
            let t = self.play_track();
            self.ensure_mod_wheel(t);
        }
    }

    /// Adds a modulator listening to track `t` without selecting it.
    fn add_track_modulator(&mut self, t: usize, kind: ModKind, name: String) -> Option<u64> {
        let id = self.project.tracks.get(t)?.id;
        let prev = self.sel_mod;
        let mid = self.add_modulator(kind)?;
        self.sel_mod = prev;
        if let Some(m) = self.project.modulators.iter_mut().find(|m| m.id == mid) {
            m.track = Some(id);
            m.name = name;
        }
        Some(mid)
    }

    /// Gives a track a Mod Wheel modulator if it has none, connected to the
    /// instrument's most telling knob when it has one.
    fn ensure_mod_wheel(&mut self, t: usize) {
        let Some(track) = self.project.tracks.get(t) else { return };
        let id = track.id;
        if self.project.modulators.iter().any(|m| m.kind == ModKind::ModWheel && m.track == Some(id)) {
            return;
        }
        let target = track.instrument.as_ref().and_then(|i| telling_params(i)[0]);
        let name = format!("Mod Wheel {}", track.name);
        let Some(mid) = self.add_track_modulator(t, ModKind::ModWheel, name) else { return };
        match target {
            Some(param) => {
                let target = ModTarget::Param { owner: ModOwner::Instrument(id), param: param as u16 };
                self.project.add_route(mid, target, 0.5);
                self.toast(format!("Mod Wheel -> {}", self.project.target_label(target).1));
            }
            None => self.toast("A Mod Wheel is on the track: connect it to a knob in the desktop's Mod view"),
        }
    }

    /// The Joystick modulator of a track for one axis (0: X, 1: Y).
    fn joystick_mod(&self, t: usize, axis: usize) -> Option<u64> {
        let id = self.project.tracks.get(t)?.id;
        self.project.modulators.iter().find(|m| m.kind == ModKind::Joystick && m.track == Some(id) && (m.params.first().copied().unwrap_or(0.0).round().max(0.0) as usize).min(1) == axis).map(|m| m.id)
    }

    fn make_joystick(&mut self, t: usize, axis: usize) -> Option<u64> {
        let name = format!("Stick {} {}", ["X", "Y"][axis], self.project.tracks.get(t)?.name);
        let mid = self.add_track_modulator(t, ModKind::Joystick, name)?;
        if let Some(m) = self.project.modulators.iter_mut().find(|m| m.id == mid) {
            if let Some(a) = m.params.first_mut() {
                *a = axis as f32;
            }
        }
        Some(mid)
    }

    /// Gives a track the two Joystick modulators XY plays with, on the
    /// instrument's most telling knobs, when it has none yet.
    fn ensure_joystick(&mut self, t: usize) {
        let Some(track) = self.project.tracks.get(t) else { return };
        let id = track.id;
        let telling = track.instrument.as_ref().map(telling_params).unwrap_or([None, None]);
        let mut made = Vec::new();
        for axis in 0..2 {
            if self.joystick_mod(t, axis).is_some() {
                continue;
            }
            let Some(param) = telling[axis] else { continue };
            if let Some(mid) = self.make_joystick(t, axis) {
                let target = ModTarget::Param { owner: ModOwner::Instrument(id), param: param as u16 };
                self.project.add_route(mid, target, 0.5);
                made.push(format!("{} {}", ["X", "Y"][axis], self.project.target_label(target).1));
            }
        }
        if !made.is_empty() {
            self.toast(format!("The right stick moves {}", made.join(", ")));
        } else if self.joystick_mod(t, 0).is_none() && self.joystick_mod(t, 1).is_none() {
            self.toast("Choose what the right stick moves: X and Y under the pad");
        }
    }

    /// What an axis of the stick moves on a track now.
    fn axis_target(&self, t: usize, axis: usize) -> Option<ModTarget> {
        let mid = self.joystick_mod(t, axis)?;
        self.project.mod_routes.iter().find(|r| r.source == mid).map(|r| r.target)
    }

    fn target_name(&self, t: usize, target: ModTarget) -> String {
        let (where_, name) = self.project.target_label(target);
        match target {
            ModTarget::Param { owner: ModOwner::Device(d), .. } => {
                let kind = self.project.tracks.get(t).and_then(|tr| tr.devices.iter().find(|x| x.id == d)).map(|x| x.kind.name()).unwrap_or("");
                if kind.is_empty() {
                    format!("{where_} {name}")
                } else {
                    format!("{kind} {name}")
                }
            }
            ModTarget::Volume(_) | ModTarget::Pan(_) => format!("Track {}", name.to_lowercase()),
            _ => name,
        }
    }

    fn axis_label(&self, t: usize, axis: usize) -> String {
        self.axis_target(t, axis).map(|tg| self.target_name(t, tg)).unwrap_or_else(|| "Nothing".into())
    }

    /// Everything a stick axis can move on a track: the instrument's knobs,
    /// its effects' knobs, volume, pan, or nothing.
    fn axis_choices(&self, t: usize) -> Vec<(Option<ModTarget>, String)> {
        let Some(track) = self.project.tracks.get(t) else { return vec![(None, "Nothing".into())] };
        let id = track.id;
        let mut out = Vec::new();
        if let Some(specs) = track.instrument.as_ref().and_then(instrument_specs) {
            for (i, sp) in specs.iter().enumerate().filter(|(_, sp)| smooth(sp)) {
                // Two knobs of the same name (two envelopes): their group tells them apart.
                let twin = specs.iter().filter(|o| o.name == sp.name).count() > 1;
                let label = if twin { format!("{} {}", sp.group, sp.name) } else { sp.name.to_string() };
                out.push((Some(ModTarget::Param { owner: ModOwner::Instrument(id), param: i as u16 }), label));
            }
        }
        for d in &track.devices {
            let owner = ModOwner::Device(d.id);
            if let Some(specs) = self.project.owner_specs(owner) {
                for (i, sp) in specs.iter().enumerate().filter(|(_, sp)| smooth(sp)) {
                    out.push((Some(ModTarget::Param { owner, param: i as u16 }), format!("{} {}", d.kind.name(), sp.name)));
                }
            }
        }
        out.push((Some(ModTarget::Volume(id)), "Track volume".into()));
        out.push((Some(ModTarget::Pan(id)), "Track pan".into()));
        out.push((None, "Nothing".into()));
        out
    }

    /// Points an axis of the stick at something else (or at nothing).
    fn set_axis(&mut self, t: usize, axis: usize, target: Option<ModTarget>) {
        let mid = match self.joystick_mod(t, axis) {
            Some(m) => m,
            None if target.is_some() => match self.make_joystick(t, axis) {
                Some(m) => m,
                None => return,
            },
            None => return,
        };
        self.project.mod_routes.retain(|r| r.source != mid);
        if let Some(tg) = target {
            self.project.add_route(mid, tg, 0.5);
        }
    }

    /// Puts what the triggers and sticks moved (bend, mod wheel, joystick)
    /// back to rest.
    pub fn big_unbend(&mut self) {
        for w in Wheel::ALL {
            let (list, last) = &mut self.big.play.wheels[w as usize];
            let list = std::mem::take(list);
            *last = w.rest();
            for track in list {
                self.cmd(w.command(track, w.rest()));
            }
        }
    }

    /// Sends a wheel's value to `targets` when it changed, and puts tracks
    /// it no longer goes to back to rest.
    fn wheel(&mut self, w: Wheel, targets: Vec<u64>, v: (f32, f32)) {
        let (list, last) = &mut self.big.play.wheels[w as usize];
        let gone: Vec<u64> = list.iter().filter(|id| !targets.contains(id)).copied().collect();
        let changed = (v.0 - last.0).abs() > 0.002 || (v.1 - last.1).abs() > 0.002 || targets.iter().any(|id| !list.contains(id));
        *last = v;
        *list = targets.clone();
        for id in gone {
            self.cmd(w.command(id, w.rest()));
        }
        if changed {
            for id in targets {
                self.cmd(w.command(id, v));
            }
        }
    }

    /// The track played, and every track a held note sounds on.
    fn play_targets(&self, t: usize) -> Vec<u64> {
        let mut targets = vec![self.project.tracks[t].id];
        for (_, ids) in self.big.play.held.values() {
            for id in ids {
                if !targets.contains(id) {
                    targets.push(*id);
                }
            }
        }
        targets
    }

    fn pad_on(&mut self, who: u64, pad: (i32, i32), pitch: u8) {
        self.pad_off(who);
        let (played, ids) = self.note_on(pitch, 0.85);
        self.big.play.held.insert(who, (played, ids));
        self.big.play.on_pad.insert(who, pad);
    }

    /// Moves a held note to another pitch: the new note starts before the
    /// old one ends, so a mono synth glides.
    fn pad_slide(&mut self, who: u64, pad: (i32, i32), pitch: u8, vel: f32) {
        let old = self.big.play.held.remove(&who);
        match old {
            Some((p, ids)) if p == pitch => {
                self.big.play.held.insert(who, (p, ids));
            }
            _ => {
                let (played, ids) = self.note_on(pitch, vel);
                self.big.play.held.insert(who, (played, ids));
                if let Some((p, ids)) = old {
                    self.note_off(p, &ids);
                }
            }
        }
        self.big.play.on_pad.insert(who, pad);
    }

    fn pad_off(&mut self, who: u64) {
        if let Some((p, ids)) = self.big.play.held.remove(&who) {
            self.note_off(p, &ids);
        }
        self.big.play.on_pad.remove(&who);
    }

    /// On the desktop a controller plays, always: the Play page runs
    /// unseen on the selected track, its sticks gliding over the rings (XY
    /// stays XY), the triggers and buttons as set in Big Picture (Start
    /// opens Big Picture, the right stick pressed records).
    pub(crate) fn desktop_pad_play(&mut self, ctx: &egui::Context, mut inp: Frame) {
        if self.big.pads.name.is_none() && inp.pressed.is_empty() && inp.stick == (0.0, 0.0) && inp.lstick == (0.0, 0.0) && inp.lt == 0.0 && inp.rt == 0.0 {
            return;
        }
        if self.project.tracks.is_empty() || !matches!(self.sel_track, TrackSel::Track(_)) {
            return;
        }
        if inp.take(Nav::Record) {
            self.toggle_record();
        }
        let screen = ctx.screen_rect();
        // Without a screen the pads have no use: the sticks glide.
        let keep = self.big.play.sticks;
        if keep == StickMode::Pads {
            self.big.play.sticks = StickMode::Glide;
        }
        egui::Area::new(Id::new("desktop_pad_play")).order(egui::Order::Background).fixed_pos(screen.min).interactable(false).show(ctx, |ui| {
            let mut unseen = ui.new_child(egui::UiBuilder::new().max_rect(screen).invisible());
            let s = Sz::of(screen);
            let _ = self.big_play(&mut unseen, screen, &s, inp);
        });
        self.big.play.sticks = keep;
    }

    /// What the controller plays on the desktop, for the status bar.
    pub(crate) fn desktop_pad_label(&self) -> Option<String> {
        self.big.pads.name.as_ref()?;
        let TrackSel::Track(t) = self.sel_track else { return None };
        let track = self.project.tracks.get(t)?;
        let mode = if self.big.play.sticks == StickMode::Pads { StickMode::Glide } else { self.big.play.sticks };
        Some(format!("Controller plays {} ({})", track.name, mode.name()))
    }

    pub(crate) fn big_play(&mut self, ui: &mut Ui, r: Rect, s: &Sz, mut inp: Frame) -> Vec<(Glyph, &'static str)> {
        if self.project.tracks.is_empty() {
            return Vec::new();
        }
        let mut t = self.play_track();
        let n_tracks = self.project.tracks.len();
        let mut new_t = t;
        let items = self.mode_items(t);
        // A list to choose from, or the row of modes, takes the D-pad, A and B.
        let mut act = None;
        let mut picker_navs = Vec::new();
        let menu_nav = |n: &Nav| n.is_direction() || matches!(n, Nav::A | Nav::B);
        if self.big.play.picker.is_some() {
            picker_navs = inp.pressed.iter().copied().filter(menu_nav).collect();
            inp.pressed.retain(|n| !menu_nav(n));
        } else if let Some(mut f) = self.big.play.bar_focus {
            f = f.min(items.len() - 1);
            for nav in inp.pressed.iter().copied().filter(menu_nav) {
                let it = &items[f];
                match nav {
                    Nav::Left => f = (f + items.len() - 1) % items.len(),
                    Nav::Right => f = (f + 1) % items.len(),
                    Nav::Up if it.stepper => act = Some((it.act, 1)),
                    Nav::Down if it.stepper => act = Some((it.act, -1)),
                    Nav::A => act = Some((it.act, 0)),
                    Nav::B => {
                        self.big.play.bar_focus = None;
                        break;
                    }
                    _ => {}
                }
            }
            inp.pressed.retain(|n| !menu_nav(n));
            if self.big.play.bar_focus.is_some() {
                self.big.play.bar_focus = Some(f);
            }
        }
        // The triggers change the track (when they do not bend or modulate).
        let turn = inp.trig;
        let last = self.big.play.last_turn;
        self.big.play.last_turn = turn;
        if self.big.play.trig_mode == TrigMode::Track {
            if turn > 0.6 && last <= 0.6 {
                new_t = (t + 1) % n_tracks;
            } else if turn < -0.6 && last >= -0.6 {
                new_t = (t + n_tracks - 1) % n_tracks;
            }
        }
        // With rings the D-pad changes the track and the range.
        let rings = self.big.play.sticks != StickMode::Pads;
        if rings {
            for nav in std::mem::take(&mut inp.pressed) {
                match nav {
                    Nav::Left => new_t = (new_t + n_tracks - 1) % n_tracks,
                    Nav::Right => new_t = (new_t + 1) % n_tracks,
                    Nav::Up => self.big.play.range = (self.big.play.range + 1).min(4),
                    Nav::Down => self.big.play.range = (self.big.play.range - 1).max(1),
                    Nav::X => self.big.play.octave = (self.big.play.octave - 1).max(-3),
                    Nav::Y => self.big.play.octave = (self.big.play.octave + 1).min(4),
                    Nav::LeftStick => self.big.play.stick_hold = !self.big.play.stick_hold,
                    Nav::B => {
                        self.pad_off(HOLD_STICK);
                        self.pad_off(HOLD_LSTICK);
                    }
                    _ => {}
                }
            }
        }
        let p = ui.painter().clone();
        let accent = rgb(self.project.tracks[t].color);
        // The head: the track (arrows to change it), the octave, recording.
        let head_h = s.u * 1.4;
        let head = Rect::from_min_size(r.min, vec2(r.width(), head_h));
        let arrow_w = s.u * 1.5;
        let left = Rect::from_min_size(head.min, vec2(arrow_w, head_h - s.u * 0.2));
        let right = Rect::from_min_size(pos2(left.right() + s.u * 7.6, head.top()), vec2(arrow_w, head_h - s.u * 0.2));
        for (rr, by, label) in [(left, -1i32, "<"), (right, 1, ">")] {
            let resp = ui.interact(rr, Id::new(("big_play_track", by)), Sense::click());
            rect_fill(&p, rr, 8.0, if resp.hovered() { TILE_HI } else { TILE });
            text(&p, rr.center(), Align2::CENTER_CENTER, label, s.big(), TEXT);
            if resp.clicked() {
                new_t = ((t as i32 + by).rem_euclid(n_tracks as i32)) as usize;
            }
        }
        if new_t != t {
            self.play_switch_to(new_t);
            t = self.play_track();
        }
        let track = &self.project.tracks[t];
        let name_r = Rect::from_min_max(pos2(left.right() + s.u * 0.3, head.top()), pos2(right.left() - s.u * 0.3, head.bottom() - s.u * 0.2));
        rect_fill(&p, name_r, 8.0, shade(accent, 0.35));
        let inst = track.instrument.as_ref().map(|i| format!("{} · {}", i.kind_name(), i.display_name())).unwrap_or_default();
        fit_text(&p, pos2(name_r.left() + s.u * 0.3, name_r.top() + name_r.height() * 0.34), Align2::LEFT_CENTER, &track.name, s.font, name_r.width() - s.u * 0.6, TEXT);
        fit_text(&p, pos2(name_r.left() + s.u * 0.3, name_r.top() + name_r.height() * 0.74), Align2::LEFT_CENTER, &inst, s.small, name_r.width() - s.u * 0.6, TEXT_DIM);
        let drums = matches!(track.instrument, Some(Instrument::Drums { .. }));
        let mut x = right.right() + s.u * 0.5;
        if !drums {
            for (by, label) in [(-1i32, "Oct -"), (1, "Oct +")] {
                let br = Rect::from_min_size(pos2(x, head.top()), vec2(s.u * 2.6, head_h - s.u * 0.2));
                let resp = ui.interact(br, Id::new(("big_oct", by)), Sense::click());
                rect_fill(&p, br, 8.0, if resp.hovered() { TILE_HI } else { TILE });
                text(&p, br.center(), Align2::CENTER_CENTER, label, s.f(), TEXT);
                if resp.clicked() {
                    self.big.play.octave = (self.big.play.octave + by).clamp(-3, 4);
                }
                x += s.u * 2.8;
            }
            text(&p, pos2(x + s.u * 0.2, head.top() + head_h * 0.45), Align2::LEFT_CENTER, format!("C{}", 4 + self.big.play.octave - 1), s.f(), TEXT_DIM);
        }
        let rec_r = Rect::from_min_size(pos2(head.right() - s.u * 3.2, head.top()), vec2(s.u * 3.2, head_h - s.u * 0.2));
        let rresp = ui.interact(rec_r, Id::new("big_play_rec"), Sense::click());
        rect_fill(&p, rec_r, 8.0, if self.recording { REC } else { TILE });
        rec_icon(&p, pos2(rec_r.left() + s.u * 0.6, rec_r.center().y), s.u * 0.45, if self.recording { TEXT } else { REC });
        text(&p, pos2(rec_r.left() + s.u * 1.1, rec_r.center().y), Align2::LEFT_CENTER, "Rec", s.f(), TEXT);
        if rresp.on_hover_text("Record what you play into a new clip").clicked() {
            self.toggle_record();
        }
        // The row of modes.
        let items = self.mode_items(t);
        let bar = Rect::from_min_size(pos2(r.left(), head.bottom()), vec2(r.width(), s.u * 1.15));
        if let Some(a) = self.play_modes(ui, bar, s, accent, &items) {
            act = Some(a);
        }
        let area = Rect::from_min_max(pos2(r.left(), bar.bottom() + s.u * 0.25), r.max);

        let hold_hint = if self.big.play.stick_hold { "Trigger" } else { "Hold" };
        let trig_hint = self.big.play.trig_mode.name();
        let mut hints = match self.big.play.sticks {
            StickMode::Pads => {
                self.play_pads(ui, area, s, t, drums, &mut inp, accent);
                let mut h = vec![(Glyph::Dpad, "Move"), (Glyph::Btn(Nav::A), "Play"), (Glyph::RightStick, "Play row"), (Glyph::Btn(Nav::LeftStick), hold_hint)];
                if !drums {
                    h.extend([(Glyph::Btn(Nav::X), "Oct -"), (Glyph::Btn(Nav::Y), "Oct +")]);
                }
                h.push((Glyph::Triggers, trig_hint));
                h
            }
            mode => {
                if let Some(a) = self.play_rings(ui, area, s, t, &inp, accent, &items) {
                    act = Some(a);
                }
                let sticks = if mode == StickMode::Xy { "Notes, XY" } else { "Play" };
                vec![(Glyph::Sticks, sticks), (Glyph::Dpad, "Track, range"), (Glyph::Btn(Nav::X), "Oct -"), (Glyph::Btn(Nav::Y), "Oct +"), (Glyph::Btn(Nav::LeftStick), hold_hint), (Glyph::Triggers, trig_hint)]
            }
        };
        if self.big.play.picker.is_some() {
            self.play_picker(ui, area, s, t, &picker_navs);
            hints = vec![(Glyph::Dpad, "Move"), (Glyph::Btn(Nav::A), "Choose"), (Glyph::Btn(Nav::B), "Close")];
        } else if self.big.play.bar_focus.is_some() {
            hints = vec![(Glyph::Dpad, "Choose, change"), (Glyph::Btn(Nav::A), "Set"), (Glyph::Btn(Nav::B), "Back to playing")];
        }
        if let Some((a, step)) = act {
            self.mode_act(t, a, step);
        }
        self.play_wheels(t, &inp);
        hints
    }

    /// Bend, mod wheel and joystick, from the triggers and sticks.
    fn play_wheels(&mut self, t: usize, inp: &Frame) {
        let targets = self.play_targets(t);
        let pl = &self.big.play;
        let (bend, modwheel) = (pl.trig_mode == TrigMode::Bend, pl.trig_mode == TrigMode::Mod);
        let range = BEND_RANGES[pl.bend_range.min(BEND_RANGES.len() - 1)];
        let xy = pl.sticks == StickMode::Xy;
        let joy = pl.xy_finger.map(|f| f.1).unwrap_or(inp.stick);
        let on = |b: bool| if b { targets.clone() } else { Vec::new() };
        self.wheel(Wheel::Bend, on(bend), ((inp.rt - inp.lt) * range, 0.0));
        self.wheel(Wheel::Mod, on(modwheel), (inp.rt.max(inp.lt), 0.0));
        let id = self.project.tracks[t].id;
        self.wheel(Wheel::Joy, if xy { vec![id] } else { Vec::new() }, joy);
    }

    /// The buttons of the row of modes (and, group 3, under the XY pad).
    fn mode_items(&self, t: usize) -> Vec<ModeItem> {
        let pl = &self.big.play;
        let item = |label: String, lit: bool, act: ModeAct, help: &'static str, group: u8, stepper: bool| ModeItem { label, lit, act, help, group, stepper };
        let mut v: Vec<ModeItem> = StickMode::ALL.iter().map(|&m| item(m.name().into(), pl.sticks == m, ModeAct::Sticks(m), m.help(), 0, false)).collect();
        for m in TrigMode::ALL {
            v.push(item(m.name().into(), pl.trig_mode == m, ModeAct::Trig(m), m.help(), 1, false));
            // How far, right beside Bend.
            if m == TrigMode::Bend && pl.trig_mode == TrigMode::Bend {
                let semis = BEND_RANGES[pl.bend_range.min(BEND_RANGES.len() - 1)];
                v.push(item(format!("±{semis} st"), false, ModeAct::BendRange, "How far the triggers bend: 1 to 24 semitones (up and down, or the halves of the button)", 1, true));
            }
        }
        if pl.sticks != StickMode::Pads {
            v.push(item(format!("{} oct", pl.range), false, ModeAct::Range, "How many octaves a ring spans (up and down, or the halves of the button)", 2, true));
        }
        v.push(item("Hold".into(), pl.stick_hold, ModeAct::Hold, "The sticks' notes ring on after letting go (L3); B ends them", 2, false));
        if pl.sticks != StickMode::Pads {
            v.push(item("Velocity".into(), pl.velocity, ModeAct::Velocity, "How far a stick is pushed when it strikes a note (or how far out a finger is on a ring) sets how hard it plays", 2, false));
        }
        if pl.sticks == StickMode::Xy {
            for axis in 0..2 {
                let help = if axis == 0 { "What the right stick moves left and right: choose" } else { "What the right stick moves up and down: choose" };
                v.push(item(format!("{}: {}", ["X", "Y"][axis], self.axis_label(t, axis)), false, ModeAct::Axis(axis), help, 3, false));
            }
        }
        v
    }

    /// Does what a mode button says: `step` 0 sets it (A, a tap), 1 and -1
    /// step it (up and down).
    fn mode_act(&mut self, t: usize, act: ModeAct, step: i32) {
        match act {
            ModeAct::Sticks(m) if step == 0 => self.set_sticks(m),
            ModeAct::Trig(m) if step == 0 => self.set_trig_mode(m),
            ModeAct::BendRange => {
                let (n, i) = (BEND_RANGES.len() as i32, self.big.play.bend_range as i32);
                self.big.play.bend_range = if step == 0 { (i + 1) % n } else { (i + step).clamp(0, n - 1) } as usize;
            }
            ModeAct::Range => {
                let r = self.big.play.range;
                self.big.play.range = if step == 0 { r % 4 + 1 } else { (r + step).clamp(1, 4) };
            }
            ModeAct::Hold if step == 0 => {
                self.big.play.stick_hold = !self.big.play.stick_hold;
                if !self.big.play.stick_hold {
                    self.pad_off(HOLD_STICK);
                    self.pad_off(HOLD_LSTICK);
                }
            }
            ModeAct::Velocity if step == 0 => self.big.play.velocity = !self.big.play.velocity,
            ModeAct::Axis(axis) if step == 0 => {
                let cur = self.axis_target(t, axis);
                let row = self.axis_choices(t).iter().position(|c| c.0 == cur).unwrap_or(0);
                self.big.play.picker = Some((axis, row));
            }
            _ => {}
        }
    }

    /// The row of modes under the head; returns a button pressed.
    fn play_modes(&mut self, ui: &mut Ui, bar: Rect, s: &Sz, accent: Color32, items: &[ModeItem]) -> Option<(ModeAct, i32)> {
        let p = ui.painter().clone();
        let pl = &self.big.play;
        let meter = pl.trig_mode != TrigMode::Track;
        let shown: Vec<(usize, &ModeItem)> = items.iter().enumerate().filter(|(_, it)| it.group < 3).collect();
        // Widths from the labels, shrunk together when the row is short.
        let font = s.f();
        let pad_x = s.u * 0.7;
        let (in_gap, group_gap) = (s.u * 0.08, s.u * 0.35);
        let widths: Vec<f32> = shown.iter().map(|(_, it)| p.layout_no_wrap(it.label.clone(), font.clone(), TEXT).size().x + pad_x + if it.stepper { s.u * 0.6 } else { 0.0 }).collect();
        let meter_w = if meter { s.u * 4.0 } else { 0.0 };
        let groups = shown.windows(2).filter(|w| w[0].1.group != w[1].1.group).count();
        let gaps = (shown.len() - 1 - groups) as f32 * in_gap + groups as f32 * group_gap + if meter { group_gap } else { 0.0 };
        let k = ((bar.width() - gaps) / (widths.iter().sum::<f32>() + meter_w)).min(1.0);
        let h = bar.height() - s.u * 0.2;
        let mut x = bar.left();
        let mut act = None;
        let pointer = ui.input(|i| i.pointer.interact_pos());
        for (j, &(i, it)) in shown.iter().enumerate() {
            if j > 0 {
                x += if shown[j - 1].1.group != it.group { group_gap } else { in_gap };
            }
            let w = widths[j] * k;
            let br = Rect::from_min_size(pos2(x, bar.top()), vec2(w, h));
            let resp = ui.interact(br, Id::new(("big_play_mode", i)), Sense::click());
            let fill = if it.lit { shade(accent, 0.8) } else if resp.hovered() { TILE_HI } else { TILE };
            tile(&p, br, fill, pl.bar_focus == Some(i));
            let col = if it.lit { TEXT_DARK } else { TEXT };
            fit_text(&p, br.center(), Align2::CENTER_CENTER, &it.label, s.font, w - if it.stepper { s.u * 1.0 } else { 6.0 }, col);
            if it.stepper {
                text(&p, pos2(br.left() + s.u * 0.25, br.center().y), Align2::CENTER_CENTER, "‹", s.f(), TEXT_DIM);
                text(&p, pos2(br.right() - s.u * 0.25, br.center().y), Align2::CENTER_CENTER, "›", s.f(), TEXT_DIM);
            }
            if resp.on_hover_text(it.help).clicked() {
                let step = if !it.stepper {
                    0
                } else if pointer.is_some_and(|pt| pt.x < br.center().x) {
                    -1
                } else {
                    1
                };
                act = Some((it.act, step));
            }
            x += w;
        }
        if meter {
            x += group_gap;
            let mr = Rect::from_min_size(pos2(x, bar.top() + h * 0.15), vec2((bar.right() - x).min(meter_w * k).max(0.0), h * 0.7));
            if mr.width() > s.u {
                rect_fill(&p, mr.expand(2.0), 8.0, TILE);
                if pl.trig_mode == TrigMode::Mod {
                    let v = pl.wheels[Wheel::Mod as usize].1 .0;
                    value_bar(&p, mr, v, false, &format!("Mod {:.0}%", v * 100.0), s, accent);
                } else {
                    let range = BEND_RANGES[pl.bend_range.min(BEND_RANGES.len() - 1)];
                    let bend = pl.bend();
                    value_bar(&p, mr, 0.5 + bend / (2.0 * range), true, &format!("{bend:+.1} st"), s, accent);
                }
            }
        }
        act
    }

    /// The pads, the cursor, the right stick over a row, fingers and the mouse.
    #[allow(clippy::too_many_arguments)]
    fn play_pads(&mut self, ui: &mut Ui, area: Rect, s: &Sz, t: usize, drums: bool, inp: &mut Frame, accent: Color32) {
        let p = ui.painter().clone();
        let (cols, rows, pitches, names) = pads(self, t, self.big.play.octave);
        let gap = s.u * 0.18;
        let pw = (area.width() - gap * (cols - 1) as f32) / cols as f32;
        let ph = (area.height() - gap * (rows - 1) as f32) / rows as f32;
        let side = if drums { pw.min(ph) } else { pw };
        let ph = if drums { side } else { ph };
        let grid_w = side * cols as f32 + gap * (cols - 1) as f32;
        let x0 = area.left() + (area.width() - grid_w) * 0.5;
        let pad_rect = |c: i32, rr: i32| Rect::from_min_size(pos2(x0 + c as f32 * (side + gap), area.top() + rr as f32 * (ph + gap)), vec2(side, ph));
        let pad_at = |pt: Pos2| -> Option<(i32, i32)> { (0..rows).flat_map(|rr| (0..cols).map(move |c| (c, rr))).find(|&(c, rr)| pad_rect(c, rr).contains(pt)) };
        // The cursor and A.
        let (mut cx, mut cy) = self.big.play.cursor;
        cx = cx.clamp(0, cols - 1);
        cy = cy.clamp(0, rows - 1);
        let mut a_down = None;
        for nav in inp.pressed.drain(..) {
            match nav {
                Nav::Left => cx = (cx - 1).max(0),
                Nav::Right => cx = (cx + 1).min(cols - 1),
                Nav::Up => cy = (cy - 1).max(0),
                Nav::Down => cy = (cy + 1).min(rows - 1),
                Nav::A => a_down = Some(true),
                Nav::X if !drums => self.big.play.octave = (self.big.play.octave - 1).max(-3),
                Nav::LeftStick => self.big.play.stick_hold = !self.big.play.stick_hold,
                Nav::B => self.pad_off(HOLD_STICK),
                Nav::Y if !drums => self.big.play.octave = (self.big.play.octave + 1).min(4),
                _ => {}
            }
        }
        if inp.released.contains(&Nav::A) {
            a_down = Some(false);
        }
        let pitch_of = |c: i32, rr: i32| pitches.get((rr * cols + c) as usize).copied().unwrap_or(60);
        match a_down {
            Some(true) => self.pad_on(HOLD_CURSOR, (cx, cy), pitch_of(cx, cy)),
            Some(false) => self.pad_off(HOLD_CURSOR),
            None => {
                // Held A slides along with the cursor.
                if self.big.play.on_pad.get(&HOLD_CURSOR).is_some_and(|&pad| pad != (cx, cy)) {
                    self.pad_on(HOLD_CURSOR, (cx, cy), pitch_of(cx, cy));
                }
            }
        }
        self.big.play.cursor = (cx, cy);
        // The right stick plays the cursor's row where it points: left the
        // first pad, up the middle, right the last.
        let (sx, sy) = inp.stick;
        let mag = (sx * sx + sy * sy).sqrt();
        let cur = self.big.play.on_pad.get(&HOLD_STICK).filter(|pad| pad.1 == cy).map(|pad| pad.0 as usize);
        let pointing = if mag > 0.55 || (cur.is_some() && mag > 0.35) { row_at(sx, sy) } else { None };
        if let Some(u) = pointing {
            let pad = (row_slot(u, cols as usize, cur) as i32, cy);
            if self.big.play.on_pad.get(&HOLD_STICK) != Some(&pad) {
                self.pad_on(HOLD_STICK, pad, pitch_of(pad.0, pad.1));
            }
        } else if mag < 0.35 && !self.big.play.stick_hold {
            self.pad_off(HOLD_STICK);
        }
        // Fingers, and the mouse when there is no touch screen.
        for (id, pos) in self.pointers(ui) {
            match pos.and_then(pad_at) {
                Some(pad) if self.big.play.on_pad.get(&id) != Some(&pad) => self.pad_on(id, pad, pitch_of(pad.0, pad.1)),
                Some(_) => {}
                None => self.pad_off(id),
            }
        }
        // What sounds now (the track's notes, from anywhere).
        let sounds = self.sounding(t);
        let held: Vec<(i32, i32)> = self.big.play.on_pad.values().copied().collect();
        // The row the right stick plays.
        let band = pad_rect(0, cy).union(pad_rect(cols - 1, cy));
        rect_line(&p, band.expand(s.u * 0.09), 10.0, 1.5, shade(accent, 0.7));
        let scale = self.project.scale;
        for rr in 0..rows {
            for c in 0..cols {
                let pr = pad_rect(c, rr);
                let pitch = pitch_of(c, rr);
                let name = &names[(rr * cols + c) as usize];
                let pressed = held.contains(&(c, rr));
                let lit = pressed || sounds(pitch);
                let root = !drums && scale.is_root(pitch);
                let base = if drums {
                    if name.is_empty() {
                        Color32::from_rgb(0x1c, 0x1c, 0x20)
                    } else {
                        shade(accent, 0.45)
                    }
                } else if root {
                    shade(accent, 0.7)
                } else {
                    TILE
                };
                let fill = if lit { mix(accent, Color32::WHITE, 0.35) } else { base };
                tile(&p, pr, fill, (c, rr) == (cx, cy));
                fit_text(&p, pr.center(), Align2::CENTER_CENTER, name, s.font, pr.width() - 8.0, if lit { TEXT_DARK } else { TEXT });
            }
        }
        // Where along the row the stick points.
        if let Some(u) = (mag > 0.3).then(|| row_at(sx, sy)).flatten() {
            let inset = s.u * 0.35;
            let at = pos2(band.left() + inset + u * (band.width() - 2.0 * inset), band.bottom() - s.u * 0.28);
            p.circle_filled(at, s.u * 0.17, TEXT);
            p.circle_stroke(at, s.u * 0.17, Stroke::new(2.0_f32, shade(accent, 0.5)));
        }
    }

    /// Glide (a ring for each stick) and XY (a ring for the left stick, a
    /// joystick pad for the right); returns a button under the pad pressed.
    #[allow(clippy::too_many_arguments)]
    fn play_rings(&mut self, ui: &mut Ui, area: Rect, s: &Sz, t: usize, inp: &Frame, accent: Color32, items: &[ModeItem]) -> Option<(ModeAct, i32)> {
        let p = ui.painter().clone();
        let xy = self.big.play.sticks == StickMode::Xy;
        let (octave, range) = (self.big.play.octave, self.big.play.range);
        let drums = matches!(self.project.tracks[t].instrument, Some(Instrument::Drums { .. }));
        let gap = s.u * 0.6;
        let half_w = (area.width() - gap) * 0.5;
        let rad = ((half_w * 0.5).min(area.height() * 0.5) / 1.17).max(s.u);
        let band = rad * 0.3;
        let left_c = pos2(area.left() + half_w * 0.5, area.center().y);
        let right_c = pos2(area.right() - half_w * 0.5, area.center().y);
        // (centre, who plays it, its stick, notes, names, what it is)
        let mut rings: Vec<(Pos2, u64, (f32, f32), Vec<u8>, Vec<String>, &str)> = Vec::new();
        if xy {
            let (n, m) = ring_notes(self, t, octave, range);
            rings.push((left_c, HOLD_LSTICK, inp.lstick, n, m, "Left stick"));
        } else {
            let (n0, m0) = ring_notes(self, t, octave - 1, range);
            let (n1, m1) = ring_notes(self, t, octave, range);
            rings.push((left_c, HOLD_LSTICK, inp.lstick, n0, m0, "Left stick"));
            rings.push((right_c, HOLD_STICK, inp.stick, n1, m1, "Right stick"));
        }
        if rings.iter().any(|ring| ring.3.is_empty()) {
            return None;
        }
        let dynamic = self.big.play.velocity;
        let vel = |v: f32| if dynamic { v } else { 0.85 };
        // The sticks: the further out, the harder.
        for (side, (_, who, (x, y), notes, _, _)) in rings.iter().enumerate() {
            let mag = (x * x + y * y).sqrt();
            let cur = self.big.play.on_pad.get(who).filter(|pad| pad.0 == side as i32).map(|pad| pad.1 as usize);
            let gliding = cur.is_some() && self.big.play.gliding.contains(who);
            if gliding && mag > 0.3 {
                // Out on the ring: every note passed plays as hard as the stick is out now.
                let slot = ring_slot(ring_at(*x, *y), notes.len(), cur);
                if cur != Some(slot) {
                    self.pad_slide(*who, (side as i32, slot as i32), notes[slot], vel(stick_velocity(mag)));
                }
            } else if mag > 0.5 {
                // On its way out: strike once it stops going out, reaches the
                // rim, or after 50 ms, as hard as it got.
                let a = self.big.play.arming.entry(*who).or_insert(Arm { peak: 0.0, last: 0.0, secs: 0.0 });
                let rising = a.last == 0.0 || mag > a.last + 0.01;
                a.peak = a.peak.max(mag);
                a.last = mag;
                a.secs += inp.dt;
                if !dynamic || mag >= 0.93 || !rising || a.secs >= 0.05 {
                    let peak = a.peak;
                    self.big.play.arming.remove(who);
                    let slot = ring_slot(ring_at(*x, *y), notes.len(), None);
                    // A note still held (Hold) is struck again, not just kept.
                    if self.big.play.held.get(who).is_some_and(|h| h.0 == notes[slot]) {
                        self.pad_off(*who);
                    }
                    self.pad_slide(*who, (side as i32, slot as i32), notes[slot], vel(stick_velocity(peak)));
                    self.big.play.gliding.push(*who);
                }
            } else if mag < 0.3 {
                self.big.play.arming.remove(who);
                self.big.play.gliding.retain(|g| g != who);
                if !self.big.play.stick_hold {
                    self.pad_off(*who);
                }
            }
        }
        // The XY pad, and the two buttons under it.
        let tiles_h = s.u * 1.2;
        let xy_side = (half_w - s.u * 0.6).min(area.height() - tiles_h - s.u * 0.4).max(s.u * 2.0);
        let xy_top = area.top() + (area.height() - xy_side - tiles_h - s.u * 0.3).max(0.0) * 0.5;
        let xy_rect = Rect::from_min_size(pos2(right_c.x - xy_side * 0.5, xy_top), vec2(xy_side, xy_side));
        // Fingers and the mouse: on a ring (further out is harder), or on the pad.
        if self.big.play.picker.is_none() {
            for (id, pos) in self.pointers(ui) {
                let on_ring = pos.and_then(|pt| {
                    rings.iter().enumerate().find_map(|(side, ring)| {
                        let d = pt - ring.0;
                        let dist = d.length();
                        let inner = rad - band * 1.6;
                        (dist > inner && dist < rad + band * 1.4).then(|| (side, ring_at(d.x, -d.y), ((dist - inner) / (band * 3.0)).clamp(0.05, 1.0)))
                    })
                });
                match on_ring {
                    Some((side, u, level)) => {
                        let notes = &rings[side].3;
                        let cur = self.big.play.on_pad.get(&id).filter(|pad| pad.0 == side as i32).map(|pad| pad.1 as usize);
                        let slot = ring_slot(u, notes.len(), cur);
                        if cur != Some(slot) {
                            self.pad_slide(id, (side as i32, slot as i32), notes[slot], vel(0.25 + 0.75 * level));
                        }
                    }
                    None => self.pad_off(id),
                }
                match pos.filter(|pt| xy && xy_rect.contains(*pt)) {
                    Some(pt) => {
                        let v = ((pt.x - xy_rect.center().x) / (xy_side * 0.5), (xy_rect.center().y - pt.y) / (xy_side * 0.5));
                        self.big.play.xy_finger = Some((id, (v.0.clamp(-1.0, 1.0), v.1.clamp(-1.0, 1.0))));
                    }
                    None => {
                        if self.big.play.xy_finger.is_some_and(|f| f.0 == id) {
                            self.big.play.xy_finger = None;
                        }
                    }
                }
            }
        }
        // The rings.
        let sounds = self.sounding(t);
        let scale = self.project.scale;
        for (side, (c, who, (x, y), notes, names, label)) in rings.iter().enumerate() {
            let held: Vec<usize> = self.big.play.on_pad.values().filter(|pad| pad.0 == side as i32).map(|pad| pad.1 as usize).collect();
            draw_ring(&p, s, *c, rad, band, notes, names, &held, &|pitch| sounds(pitch), |pitch| !drums && scale.is_root(pitch), accent);
            // The stick, and the note it plays.
            let dot = *c + vec2(*x, -*y) * (rad - band * 0.5 - s.u * 0.3);
            p.line_segment([*c, dot], Stroke::new(2.0_f32, shade(accent, 0.6)));
            p.circle_filled(*c, s.u * 0.12, shade(accent, 0.6));
            p.circle_filled(dot, s.u * 0.3, if self.big.play.held.contains_key(who) { accent } else { TEXT_DIM });
            let playing = self.big.play.on_pad.get(who).filter(|pad| pad.0 == side as i32).and_then(|pad| names.get(pad.1 as usize)).cloned().unwrap_or_default();
            if !playing.is_empty() {
                fit_text(&p, *c + vec2(0.0, rad * 0.3), Align2::CENTER_CENTER, &playing, s.font * 1.6, rad * 0.9, TEXT);
            }
            let span = if drums || notes.is_empty() { String::new() } else { format!("   {} – {}", names[0], names[names.len() - 1]) };
            fit_text(&p, *c + vec2(0.0, rad * 0.58), Align2::CENTER_CENTER, &format!("{label}{span}"), s.small, rad * 1.3, TEXT_DIM);
        }
        if !xy {
            return None;
        }
        // The XY pad.
        let joy = self.big.play.xy_finger.map(|f| f.1).unwrap_or(inp.stick);
        let c = xy_rect.center();
        rect_fill(&p, xy_rect, 14.0, TILE);
        let line = Stroke::new(1.5_f32, TILE_HI);
        p.line_segment([pos2(xy_rect.left() + 8.0, c.y), pos2(xy_rect.right() - 8.0, c.y)], line);
        p.line_segment([pos2(c.x, xy_rect.top() + 8.0), pos2(c.x, xy_rect.bottom() - 8.0)], line);
        p.circle_stroke(c, xy_side * 0.5 - 6.0, line);
        let dot = c + vec2(joy.0, -joy.1) * (xy_side * 0.5 - s.u * 0.45);
        let moved = joy.0 != 0.0 || joy.1 != 0.0;
        p.line_segment([c, dot], Stroke::new(2.0_f32, shade(accent, 0.6)));
        p.circle_filled(dot, s.u * 0.38, if moved { accent } else { TEXT_DIM });
        fit_text(&p, pos2(xy_rect.right() - s.u * 0.3, c.y - s.u * 0.35), Align2::RIGHT_CENTER, &self.axis_label(t, 0), s.small, xy_side * 0.45, TEXT_DIM);
        fit_text(&p, pos2(c.x + s.u * 0.3, xy_rect.top() + s.u * 0.4), Align2::LEFT_CENTER, &self.axis_label(t, 1), s.small, xy_side * 0.45, TEXT_DIM);
        let row = Rect::from_min_size(pos2(right_c.x - half_w * 0.5 + s.u * 0.3, xy_rect.bottom() + s.u * 0.3), vec2(half_w - s.u * 0.6, tiles_h));
        let axis: Vec<(usize, &ModeItem)> = items.iter().enumerate().filter(|(_, it)| it.group == 3).collect();
        let w = (row.width() - s.u * 0.2) / axis.len().max(1) as f32;
        let mut act = None;
        for (k, (i, it)) in axis.iter().enumerate() {
            let tr = Rect::from_min_size(pos2(row.left() + k as f32 * (w + s.u * 0.2), row.top()), vec2(w, tiles_h));
            let resp = ui.interact(tr, Id::new(("big_axis", k)), Sense::click());
            tile(&p, tr, if resp.hovered() { TILE_HI } else { TILE }, self.big.play.bar_focus == Some(*i));
            fit_text(&p, tr.center(), Align2::CENTER_CENTER, &it.label, s.font, tr.width() - 8.0, TEXT);
            if resp.on_hover_text(it.help).clicked() {
                act = Some((it.act, 0));
            }
        }
        act
    }

    /// The list of what a stick axis can move.
    fn play_picker(&mut self, ui: &mut Ui, area: Rect, s: &Sz, t: usize, navs: &[Nav]) {
        let Some((axis, mut row)) = self.big.play.picker else { return };
        let choices = self.axis_choices(t);
        let n = choices.len();
        let cur = self.axis_target(t, axis);
        row = row.min(n - 1);
        let mut choose = None;
        for nav in navs {
            match nav {
                Nav::Up => row = (row + n - 1) % n,
                Nav::Down => row = (row + 1) % n,
                Nav::A => choose = Some(row),
                Nav::B => {
                    self.big.play.picker = None;
                    return;
                }
                _ => {}
            }
        }
        let p = ui.painter().clone();
        let row_h = s.u * 1.0;
        let shown = (((area.height() - s.u * 1.8) / row_h).floor() as usize).clamp(1, n);
        let panel = Rect::from_center_size(area.center(), vec2((s.u * 12.0).min(area.width()), shown as f32 * row_h + s.u * 1.5));
        let behind = ui.interact(area, Id::new("big_picker_behind"), Sense::click());
        p.rect_filled(area, 8.0, Color32::from_black_alpha(160));
        rect_fill(&p, panel, 12.0, PANEL2);
        let title = if axis == 0 { "The right stick, left and right, moves" } else { "The right stick, up and down, moves" };
        fit_text(&p, pos2(panel.left() + s.u * 0.5, panel.top() + s.u * 0.6), Align2::LEFT_CENTER, title, s.font, panel.width() - s.u, accent_now());
        let list = Rect::from_min_max(pos2(panel.left(), panel.top() + s.u * 1.2), panel.max);
        let lresp = ui.interact(list, Id::new("big_picker_list"), Sense::click_and_drag());
        let scrolled = scrolled_rows(ui, &lresp, list, row_h, Id::new("big_picker_scroll"));
        if scrolled != 0 {
            row = (row as i64 + scrolled).clamp(0, n as i64 - 1) as usize;
        }
        let first = row.saturating_sub(shown / 2).min(n - shown);
        for (k, i) in (first..first + shown).enumerate() {
            let rr = Rect::from_min_size(pos2(list.left() + s.u * 0.3, list.top() + k as f32 * row_h), vec2(list.width() - s.u * 0.6, row_h - s.u * 0.12));
            tile(&p, rr, if i == row { TILE_HI } else { TILE }, i == row);
            let (target, label) = &choices[i];
            let on = *target == cur;
            fit_text(&p, pos2(rr.left() + s.u * 0.4, rr.center().y), Align2::LEFT_CENTER, label, s.font, rr.width() - s.u * 1.4, if on { accent_now() } else { TEXT });
            if on {
                p.circle_filled(pos2(rr.right() - s.u * 0.5, rr.center().y), s.u * 0.15, accent_now());
            }
            if lresp.clicked() && lresp.interact_pointer_pos().is_some_and(|pt| rr.contains(pt)) {
                choose = Some(i);
            }
        }
        if behind.clicked() && !behind.interact_pointer_pos().is_some_and(|pt| panel.contains(pt)) {
            self.big.play.picker = None;
            return;
        }
        match choose {
            Some(i) => {
                self.big.play.picker = None;
                let target = choices[i].0;
                self.set_axis(t, axis, target);
            }
            None => self.big.play.picker = Some((axis, row)),
        }
    }

    /// Where each finger is (by touch id; None once lifted), or the mouse
    /// (HOLD_MOUSE) when there is no touch screen.
    fn pointers(&mut self, ui: &Ui) -> Vec<(u64, Option<Pos2>)> {
        // On the desktop the page runs unseen for the controller only: the
        // mouse and fingers belong to the desktop.
        if !self.big.on {
            return Vec::new();
        }
        let touches: Vec<(u64, egui::TouchPhase, Pos2)> = ui.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Touch { id, phase, pos, .. } => Some((id.0, *phase, *pos)),
                    _ => None,
                })
                .collect()
        });
        if !touches.is_empty() {
            self.big.play.touch = true;
        }
        let mut out: Vec<(u64, Option<Pos2>)> = touches.into_iter().map(|(id, phase, pos)| (id, matches!(phase, egui::TouchPhase::Start | egui::TouchPhase::Move).then_some(pos))).collect();
        if !self.big.play.touch {
            let (down, pos) = ui.input(|i| (i.pointer.primary_down(), i.pointer.interact_pos()));
            out.push((HOLD_MOUSE, if down { pos } else { None }));
        }
        out
    }

    /// Whether a pitch sounds on a track now (from anywhere).
    fn sounding(&self, t: usize) -> impl Fn(u8) -> bool {
        use std::sync::atomic::Ordering::Relaxed;
        let st = self.status();
        let bits = st.tracks.get(t).map(|ts| [ts.inst_held[0].load(Relaxed), ts.inst_held[1].load(Relaxed)]).unwrap_or([0, 0]);
        move |pitch: u8| (bits[(pitch / 64) as usize] >> (pitch % 64)) & 1 == 1
    }
}

/// A ring of notes: a slot per note all the way round, the roots in the
/// track's colour, what is held or sounds lit.
#[allow(clippy::too_many_arguments)]
fn draw_ring(p: &Painter, s: &Sz, c: Pos2, rad: f32, band: f32, notes: &[u8], names: &[String], held: &[usize], sounds: &dyn Fn(u8) -> bool, root: impl Fn(u8) -> bool, accent: Color32) {
    let n = notes.len();
    let slot_len = rad * TAU / n as f32;
    let gap_u = (1.2 / (rad * TAU)).min(0.3 / n as f32);
    for i in 0..n {
        let (u0, u1) = (i as f32 / n as f32 + gap_u, (i + 1) as f32 / n as f32 - gap_u);
        let steps = (((u1 - u0) * TAU) / 0.06).ceil().max(2.0) as usize;
        let pts: Vec<Pos2> = (0..=steps).map(|k| ring_point(c, rad, u0 + (u1 - u0) * k as f32 / steps as f32)).collect();
        let pressed = held.contains(&i);
        let lit = pressed || sounds(notes[i]);
        let is_root = root(notes[i]);
        let fill = if pressed {
            mix(accent, Color32::WHITE, 0.45)
        } else if lit {
            mix(accent, Color32::WHITE, 0.2)
        } else if is_root {
            shade(accent, 0.7)
        } else {
            TILE_HI
        };
        p.add(egui::Shape::line(pts, Stroke::new(band, fill)));
        if slot_len > s.u * 1.15 || is_root || pressed {
            let at = ring_point(c, rad, (i as f32 + 0.5) / n as f32);
            fit_text(p, at, Align2::CENTER_CENTER, &names[i], s.small, slot_len.max(band) - 4.0, if lit || is_root { TEXT_DARK } else { TEXT });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(deg: f32) -> (f32, f32) {
        let a = deg.to_radians();
        (a.sin(), a.cos())
    }

    #[test]
    fn the_ring_goes_all_the_way_round() {
        let at = |deg: f32| {
            let (x, y) = dir(deg);
            ring_at(x, y)
        };
        // From the bottom over the left (low), the top, the right (high).
        assert!(at(-179.9) < 0.01);
        assert!((at(-90.0) - 0.25).abs() < 1e-4);
        assert!((at(0.0) - 0.5).abs() < 1e-4);
        assert!((at(90.0) - 0.75).abs() < 1e-4);
        assert!(at(179.9) > 0.99);
    }

    #[test]
    fn a_ring_slot_holds_across_the_bottom() {
        assert_eq!(ring_slot(0.5, 15, None), 7);
        assert_eq!(ring_slot(8.1 / 15.0, 15, Some(7)), 7);
        assert_eq!(ring_slot(8.3 / 15.0, 15, Some(7)), 8);
        // The top note stays when the stick crosses the bottom a little.
        assert_eq!(ring_slot(0.1 / 15.0, 15, Some(14)), 14);
        assert_eq!(ring_slot(0.3 / 15.0, 15, Some(14)), 0);
        assert_eq!(ring_slot(14.9 / 15.0, 15, Some(0)), 0);
    }

    #[test]
    fn the_row_follows_the_stick() {
        assert_eq!(row_at(-1.0, 0.0), Some(0.0));
        assert!((row_at(0.0, 1.0).unwrap() - 0.5).abs() < 1e-6);
        assert_eq!(row_at(1.0, 0.0), Some(1.0));
        // Below the middle: the nearer end; straight down: no pad.
        assert_eq!(row_at(-0.7, -0.7), Some(0.0));
        assert_eq!(row_at(0.7, -0.7), Some(1.0));
        assert_eq!(row_at(0.0, -1.0), None);
        // Eight pads: left plays the first, right the last, each a step to the right of the one before.
        let pads: Vec<usize> = (0..=8)
            .map(|k| {
                let (x, y) = dir(k as f32 * 22.5 - 90.0);
                row_slot(row_at(x, y).unwrap(), 8, None)
            })
            .collect();
        assert_eq!(pads.first(), Some(&0));
        assert_eq!(pads.last(), Some(&7));
        assert!(pads.windows(2).all(|w| w[1] >= w[0]));
    }

    #[test]
    fn a_row_slot_holds_until_well_past_its_edge() {
        assert_eq!(row_slot(0.0, 15, None), 0);
        assert_eq!(row_slot(1.0, 15, None), 14);
        assert_eq!(row_slot(8.1 / 15.0, 15, Some(7)), 7);
        assert_eq!(row_slot(8.3 / 15.0, 15, Some(7)), 8);
        assert_eq!(row_slot(6.9 / 15.0, 15, Some(7)), 7);
    }

    #[test]
    fn further_out_strikes_harder() {
        assert!((stick_velocity(1.0) - 1.0).abs() < 1e-6);
        assert!((stick_velocity(0.95) - 1.0).abs() < 1e-6);
        assert!((stick_velocity(0.5) - 0.25).abs() < 1e-6);
        assert!(stick_velocity(0.8) > stick_velocity(0.6));
    }

    #[test]
    fn every_instrument_gets_two_telling_knobs() {
        for name in ["Fat Bass", "Soft Pad"] {
            let inst = Instrument::synth(name);
            let [x, y] = telling_params(&inst);
            let specs = instrument_specs(&inst).unwrap();
            assert_eq!(specs[x.unwrap()].name, "Cutoff");
            assert_eq!(specs[y.unwrap()].name, "Resonance");
        }
    }
}
