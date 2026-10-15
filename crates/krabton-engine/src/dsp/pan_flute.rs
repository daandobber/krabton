//! Pan Flute: blown pipes. The tone is the pipe's own harmonics: a pan
//! flute's pipes are closed at the bottom, so they sound odd harmonics
//! only (as a clarinet does); a shakuhachi or a bamboo flute is open and
//! has them all; an ocarina is a vessel, nearly a pure tone. Blowing harder
//! brightens it. The breath is noise from the jet, rippling with the tone,
//! that goes through the pipe and so rings at its resonances (every
//! harmonic for an open pipe, the odd ones for a closed one). A note starts
//! with a chiff, a puff of breath and a flash of the overblown harmonic,
//! and may scoop up into its pitch (a shakuhachi's meri); vibrato comes in
//! after a while, more in loudness for a pan flute, more in pitch for a
//! shakuhachi. A flute blown across an edge has to speak: first the breath
//! rings in the pipe, then the tone grows out of it, unsteady for a moment
//! and never quite still (a pan pipe speaks at once).
//!
//! What it is made of colours it (Material): bamboo, wood and clay soak up
//! the high harmonics and breathe more, metal and glass ring along with a
//! shimmer of their own walls, bone is hollow and rough, plastic plain.
//!
//! How it is played (Playing). Free plays anything: one voice legato with
//! glide, more play chords, far apart too. Mouth plays as a mouth does. On
//! a pan flute the lips are on one pipe, or on two neighbours at once (both
//! sound and share the breath); a note further away moves the lips there
//! along the row, and the pipes they pass sound briefly on the way (Slide).
//! The pipes are the notes of the song's key. Neighbours lets the lips catch
//! the next pipe as well. A flute with finger holes plays one note: notes
//! held together slur under the fingers, the same note again is tongued, a
//! big leap cracks as it overblows, and Neighbours flicks a finger to the
//! note above just before each note.
//!
//! Original code.

use super::effects::FxCtx;
use super::fx::hall::{self, Hall};
use super::{db_to_gain, midi_to_freq, pan_gains, OnePole, Rng};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

pub const MODEL: usize = 0;
pub const BREATH: usize = 1;
pub const CHIFF: usize = 2;
pub const TONE: usize = 3;
pub const VIBRATO: usize = 4;
pub const VIB_RATE: usize = 5;
pub const VIB_DELAY: usize = 6;
pub const BEND: usize = 7;
pub const ATTACK: usize = 8;
pub const RELEASE: usize = 9;
pub const VOICES: usize = 10;
pub const GLIDE: usize = 11;
pub const TRANSPOSE: usize = 12;
pub const SPREAD: usize = 13;
pub const SPACE: usize = 14;
pub const VOLUME: usize = 15;
pub const MATERIAL: usize = 16;
pub const PLAYING: usize = 17;
pub const NEIGHBOURS: usize = 18;
pub const SLIDE: usize = 19;

pub const MODELS: &[&str] = &["Pan Flute", "Shakuhachi", "Ocarina", "Bamboo Flute", "Wooden Flute"];
pub const MATERIALS: &[&str] = &["Natural", "Bamboo", "Wood", "Clay", "Metal", "Glass", "Bone", "Plastic"];
pub const PLAYINGS: &[&str] = &["Free", "Mouth"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Flute", "Flute", MODELS, 0),
    ParamSpec::lin("Breath", "Flute", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Chiff", "Flute", 0.0, 1.0, 0.55, Unit::Percent),
    ParamSpec::lin("Tone", "Flute", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Vibrato", "Expression", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Vibrato Rate", "Expression", 3.0, 8.0, 5.2, Unit::Hz),
    ParamSpec::lin("Vibrato Delay", "Expression", 0.0, 1.5, 0.35, Unit::Sec),
    ParamSpec::lin("Bend", "Expression", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::log("Attack", "Expression", 0.01, 0.8, 0.05, Unit::Sec),
    ParamSpec::log("Release", "Expression", 0.03, 2.5, 0.25, Unit::Sec),
    ParamSpec::int("Voices", "Play", 1.0, 8.0, 1.0, Unit::None),
    ParamSpec::lin("Glide", "Play", 0.0, 0.5, 0.05, Unit::Sec),
    ParamSpec::int("Transpose", "Play", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Spread", "Sound", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Space", "Sound", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Volume", "Sound", -24.0, 12.0, 0.0, Unit::Db),
    ParamSpec::choice("Material", "Flute", MATERIALS, 0),
    ParamSpec::choice("Playing", "Play", PLAYINGS, 0),
    ParamSpec::lin("Neighbours", "Play", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Slide", "Play", 0.0, 1.0, 0.5, Unit::Percent),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Andes Pan Flute",
        "Romanian Pan Flute",
        "Double Pipes",
        "Clay Pipes",
        "Zen Shakuhachi",
        "Shakuhachi Breath",
        "Ocarina",
        "Bansuri Evening",
        "Canyon Flute",
        "Silver Flute",
        "Glass Flute",
        "Bone Flute",
        "School Recorder",
        "Bamboo Grove",
        "Breathy Pad",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Andes Pan Flute" => &[(PLAYING, 1.0), (SLIDE, 0.45), (NEIGHBOURS, 0.1), (BREATH, 0.35), (CHIFF, 0.7), (TONE, 0.45), (VIBRATO, 0.2), (VIB_DELAY, 0.3), (BEND, 0.2), (ATTACK, 0.04), (RELEASE, 0.2), (GLIDE, 0.03), (SPACE, 0.3)],
        "Romanian Pan Flute" => &[(PLAYING, 1.0), (SLIDE, 0.8), (NEIGHBOURS, 0.05), (BREATH, 0.35), (CHIFF, 0.55), (TONE, 0.55), (VIBRATO, 0.6), (VIB_RATE, 5.8), (VIB_DELAY, 0.25), (BEND, 0.35), (ATTACK, 0.05), (RELEASE, 0.4), (GLIDE, 0.08), (SPACE, 0.55)],
        "Double Pipes" => &[(PLAYING, 1.0), (NEIGHBOURS, 0.7), (SLIDE, 0.5), (BREATH, 0.4), (CHIFF, 0.6), (TONE, 0.5), (VIBRATO, 0.3), (VIB_DELAY, 0.3), (BEND, 0.2), (RELEASE, 0.3), (SPACE, 0.4)],
        "Clay Pipes" => &[(PLAYING, 1.0), (MATERIAL, 3.0), (SLIDE, 0.4), (BREATH, 0.5), (CHIFF, 0.4), (TONE, 0.4), (VIBRATO, 0.2), (BEND, 0.15), (RELEASE, 0.3), (SPACE, 0.45)],
        "Zen Shakuhachi" => &[(MODEL, 1.0), (PLAYING, 1.0), (NEIGHBOURS, 0.25), (BREATH, 0.45), (CHIFF, 0.5), (TONE, 0.5), (VIBRATO, 0.3), (VIB_RATE, 4.5), (VIB_DELAY, 0.5), (BEND, 0.6), (ATTACK, 0.12), (RELEASE, 0.5), (GLIDE, 0.12), (SPACE, 0.6)],
        "Shakuhachi Breath" => &[(MODEL, 1.0), (PLAYING, 1.0), (BREATH, 0.7), (CHIFF, 0.9), (TONE, 0.75), (VIBRATO, 0.15), (BEND, 0.8), (ATTACK, 0.03), (RELEASE, 0.35), (SPACE, 0.45)],
        "Ocarina" => &[(MODEL, 2.0), (PLAYING, 1.0), (MATERIAL, 3.0), (BREATH, 0.2), (CHIFF, 0.3), (TONE, 0.4), (VIBRATO, 0.25), (VIB_RATE, 5.0), (VIB_DELAY, 0.4), (BEND, 0.1), (ATTACK, 0.03), (RELEASE, 0.12), (SPACE, 0.3)],
        "Bansuri Evening" => &[(MODEL, 3.0), (PLAYING, 1.0), (NEIGHBOURS, 0.35), (BREATH, 0.4), (CHIFF, 0.45), (TONE, 0.5), (VIBRATO, 0.35), (VIB_RATE, 5.3), (BEND, 0.4), (ATTACK, 0.08), (RELEASE, 0.35), (GLIDE, 0.1), (SPACE, 0.5)],
        "Canyon Flute" => &[(MODEL, 4.0), (PLAYING, 1.0), (MATERIAL, 2.0), (BREATH, 0.35), (CHIFF, 0.6), (TONE, 0.55), (VIBRATO, 0.45), (VIB_RATE, 5.5), (VIB_DELAY, 0.3), (BEND, 0.3), (RELEASE, 0.6), (SPACE, 0.85)],
        "Silver Flute" => &[(MODEL, 3.0), (PLAYING, 1.0), (MATERIAL, 4.0), (NEIGHBOURS, 0.1), (BREATH, 0.25), (CHIFF, 0.35), (TONE, 0.6), (VIBRATO, 0.45), (VIB_RATE, 5.4), (VIB_DELAY, 0.2), (BEND, 0.1), (ATTACK, 0.04), (RELEASE, 0.3), (SPACE, 0.45)],
        "Glass Flute" => &[(MODEL, 3.0), (PLAYING, 1.0), (MATERIAL, 5.0), (BREATH, 0.2), (CHIFF, 0.45), (TONE, 0.55), (VIBRATO, 0.3), (VIB_DELAY, 0.4), (BEND, 0.15), (RELEASE, 0.45), (SPACE, 0.6)],
        "Bone Flute" => &[(MODEL, 4.0), (PLAYING, 1.0), (MATERIAL, 6.0), (NEIGHBOURS, 0.2), (BREATH, 0.55), (CHIFF, 0.6), (TONE, 0.45), (VIBRATO, 0.15), (BEND, 0.4), (RELEASE, 0.4), (SPACE, 0.7)],
        "School Recorder" => &[(MODEL, 4.0), (PLAYING, 1.0), (MATERIAL, 7.0), (BREATH, 0.15), (CHIFF, 0.45), (TONE, 0.5), (VIBRATO, 0.05), (BEND, 0.05), (ATTACK, 0.02), (RELEASE, 0.08), (GLIDE, 0.0), (SPACE, 0.15)],
        "Bamboo Grove" => &[(MODEL, 3.0), (MATERIAL, 1.0), (VOICES, 4.0), (BREATH, 0.45), (CHIFF, 0.35), (TONE, 0.4), (VIBRATO, 0.25), (ATTACK, 0.15), (RELEASE, 0.9), (SPREAD, 0.7), (SPACE, 0.6)],
        "Breathy Pad" => &[(VOICES, 6.0), (BREATH, 0.45), (CHIFF, 0.1), (TONE, 0.35), (VIBRATO, 0.2), (ATTACK, 0.5), (RELEASE, 1.5), (SPREAD, 0.8), (SPACE, 0.7)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

/// How each flute sounds.
struct Model {
    /// Closed at one end: odd harmonics, in tone and in breath.
    stopped: bool,
    harm: [f32; 6],
    breath: f32,
    /// How strongly the breath rings in the pipe (comb feedback).
    ring: f32,
    chiff: f32,
    /// The harmonic that flashes in the chiff (overblowing).
    flash: usize,
    /// How much of the vibrato is loudness rather than pitch.
    vib_amp: f32,
    /// How far a note scoops up into its pitch (semitones) and how fast.
    scoop: f32,
    scoop_t: f32,
    /// A little buzz (the wooden flute's two chambers).
    buzz: f32,
    /// How much the tone has to speak: grow out of the breath, unsteady at
    /// first (0: at once, as a pan pipe does).
    speak: f32,
}

const MODEL_DATA: [Model; 5] = [
    Model { stopped: true, harm: [1.0, 0.0, 0.18, 0.0, 0.06, 0.0], breath: 0.11, ring: 0.9, chiff: 0.8, flash: 3, vib_amp: 0.7, scoop: 0.6, scoop_t: 0.06, buzz: 0.0, speak: 0.0 },
    Model { stopped: false, harm: [1.0, 0.35, 0.18, 0.1, 0.05, 0.03], breath: 0.18, ring: 0.85, chiff: 0.6, flash: 2, vib_amp: 0.3, scoop: 1.2, scoop_t: 0.2, buzz: 0.0, speak: 1.0 },
    Model { stopped: true, harm: [1.0, 0.05, 0.03, 0.01, 0.0, 0.0], breath: 0.04, ring: 0.6, chiff: 0.25, flash: 2, vib_amp: 0.5, scoop: 0.3, scoop_t: 0.05, buzz: 0.0, speak: 0.5 },
    Model { stopped: false, harm: [1.0, 0.25, 0.12, 0.05, 0.02, 0.01], breath: 0.13, ring: 0.9, chiff: 0.4, flash: 2, vib_amp: 0.5, scoop: 0.8, scoop_t: 0.12, buzz: 0.0, speak: 1.0 },
    Model { stopped: false, harm: [1.0, 0.3, 0.22, 0.08, 0.05, 0.02], breath: 0.09, ring: 0.92, chiff: 0.5, flash: 2, vib_amp: 0.6, scoop: 0.5, scoop_t: 0.08, buzz: 0.25, speak: 0.9 },
];

/// What the walls do to the sound.
struct Material {
    /// The harmonics' brightness, and how much the pipe's walls damp the
    /// breath ringing in it (the loop's lowpass: higher rings brighter).
    bright: f32,
    damp: f32,
    ring: f32,
    breath: f32,
    /// The wall's own ring: how much, at what ratio of the note, how long.
    wall: f32,
    wall_ratio: f32,
    wall_t: f32,
    /// The colour of the breath noise.
    hp: f32,
    lp: f32,
    lp2: f32,
}

const MATERIAL_DATA: [Material; 8] = [
    // Natural: as the flute is.
    Material { bright: 1.0, damp: 0.35, ring: 1.0, breath: 1.0, wall: 0.0, wall_ratio: 4.0, wall_t: 0.01, hp: 250.0, lp: 2800.0, lp2: 4000.0 },
    // Bamboo: light, a dry fibre in the breath.
    Material { bright: 1.0, damp: 0.33, ring: 1.0, breath: 1.1, wall: 0.04, wall_ratio: 3.9, wall_t: 0.02, hp: 260.0, lp: 2900.0, lp2: 4200.0 },
    // Wood: warm, the high harmonics soaked up.
    Material { bright: 0.85, damp: 0.24, ring: 0.98, breath: 0.95, wall: 0.03, wall_ratio: 3.3, wall_t: 0.015, hp: 220.0, lp: 2300.0, lp2: 3300.0 },
    // Clay: dull and airy, an ocarina's walls.
    Material { bright: 0.7, damp: 0.18, ring: 0.94, breath: 1.35, wall: 0.0, wall_ratio: 3.0, wall_t: 0.01, hp: 180.0, lp: 1800.0, lp2: 2600.0 },
    // Metal: bright, the walls ring along.
    Material { bright: 1.25, damp: 0.6, ring: 1.03, breath: 0.8, wall: 0.18, wall_ratio: 5.4, wall_t: 0.25, hp: 400.0, lp: 4800.0, lp2: 6500.0 },
    // Glass: pure and cool, a long glassy ring.
    Material { bright: 1.15, damp: 0.7, ring: 1.04, breath: 0.65, wall: 0.25, wall_ratio: 6.7, wall_t: 0.6, hp: 500.0, lp: 5500.0, lp2: 7500.0 },
    // Bone: old and hollow, a little rough.
    Material { bright: 1.05, damp: 0.4, ring: 0.96, breath: 1.25, wall: 0.06, wall_ratio: 4.3, wall_t: 0.05, hp: 320.0, lp: 3400.0, lp2: 4600.0 },
    // Plastic: plain, a school recorder.
    Material { bright: 0.95, damp: 0.3, ring: 0.95, breath: 0.85, wall: 0.0, wall_ratio: 3.0, wall_t: 0.01, hp: 260.0, lp: 3000.0, lp2: 4200.0 },
];

const MAX_VOICES: usize = 8;
const LINE: usize = 4096;
/// A voice that belongs to no key (a pipe passed on the way).
const NONE: u8 = 255;
/// The notes of C major, a bit each: the pipes when no key is known.
const C_MAJOR: u16 = 0b1010_1011_0101;

struct Voice {
    note: u8,
    /// The key that made it (its note-off ends it).
    owner: u8,
    gate: bool,
    f: f32,
    target: f32,
    phase: f32,
    vib_ph: f32,
    env: f32,
    t: f32,
    vel: f32,
    /// Its share of the sound (a neighbour caught by the lips is quieter).
    amp: f32,
    line: Vec<f32>,
    w: usize,
    loop_lp: f32,
    hp: OnePole,
    lp: OnePole,
    lp2: OnePole,
    drift: f32,
    pan: f32,
    level: f32,
    /// Slurred in (a smaller scoop), and how much chiff it starts with.
    slur: bool,
    fresh: f32,
    /// Samples before it starts, and after which it stops by itself.
    delay: u32,
    auto_off: u32,
    /// A finger moves: the pitch it goes to, and in how many samples.
    then: Option<(f32, u32)>,
    /// The wall's ring.
    wall1: f32,
    wall2: f32,
    /// How far the tone has spoken (0..1), when it starts to, and the
    /// turbulence that keeps it from standing still.
    speak: f32,
    speak_at: f32,
    flut: f32,
}

impl Voice {
    fn new() -> Self {
        Self {
            note: 60,
            owner: NONE,
            gate: false,
            f: 261.6,
            target: 261.6,
            phase: 0.0,
            vib_ph: 0.0,
            env: 0.0,
            t: 0.0,
            vel: 0.0,
            amp: 1.0,
            line: vec![0.0; LINE],
            w: 0,
            loop_lp: 0.0,
            hp: OnePole::default(),
            lp: OnePole::default(),
            lp2: OnePole::default(),
            drift: 0.0,
            pan: 0.0,
            level: 0.0,
            slur: false,
            fresh: 1.0,
            delay: 0,
            auto_off: 0,
            then: None,
            wall1: 0.0,
            wall2: 0.0,
            speak: 1.0,
            speak_at: 0.0,
            flut: 0.0,
        }
    }

    fn quiet(&self) -> bool {
        !self.gate && self.env < 1e-4 && self.delay == 0
    }

    /// Starts it on `note` (for the key `owner`).
    #[allow(clippy::too_many_arguments)]
    fn start(&mut self, note: u8, owner: u8, f: f32, vel: f32, amp: f32, fresh: f32, delay: u32, auto_off: u32, pan: f32) {
        self.note = note;
        self.owner = owner;
        self.f = f;
        self.target = f;
        self.t = 0.0;
        self.vel = vel;
        self.amp = amp;
        self.fresh = fresh;
        self.slur = fresh < 0.99;
        self.delay = delay;
        self.auto_off = auto_off;
        self.then = None;
        self.gate = true;
        self.pan = pan;
        // It speaks anew: soon after the breath, sooner when slid into.
        self.speak = 0.0;
        self.speak_at = if fresh < 0.5 { 0.002 } else { 0.006 + 0.003 * (note % 5) as f32 };
    }

    fn stop(&mut self) {
        self.gate = false;
        self.delay = 0;
        self.auto_off = 0;
        self.then = None;
    }
}

pub struct PanFluteState {
    /// Pitch bend in semitones (a wheel, a controller's triggers), set by the engine.
    pub pitch_bend: f32,
    /// The notes of the song's key, a bit each (the pipes), set by the engine.
    pub key: u16,
    sr: f32,
    voices: Vec<Voice>,
    /// Keys held, in order (one voice plays the last).
    stack: Vec<u8>,
    /// The pipes under the lips (Mouth), and where the lips are.
    lips: [u8; 2],
    n_lips: usize,
    mouth: u8,
    last_vel: f32,
    rng: Rng,
    hall: Box<Hall>,
    hall_p: Vec<f32>,
    dl: Vec<f32>,
    dr: Vec<f32>,
    wl: Vec<f32>,
    wr: Vec<f32>,
}

const CHUNK: usize = 64;

fn model_of(p: &[f32]) -> usize {
    (p[MODEL].round().max(0.0) as usize).min(MODELS.len() - 1)
}

fn mouth_of(p: &[f32]) -> bool {
    p[PLAYING] >= 0.5
}

impl PanFluteState {
    pub fn new(sr: f32) -> Self {
        Self {
            pitch_bend: 0.0,
            key: C_MAJOR,
            sr: sr.max(1000.0),
            voices: (0..MAX_VOICES).map(|_| Voice::new()).collect(),
            stack: Vec::with_capacity(32),
            lips: [60; 2],
            n_lips: 0,
            mouth: 60,
            last_vel: 0.8,
            rng: Rng::new(0xF1_07E5),
            hall: Box::new(Hall::new(sr)),
            hall_p: crate::params::defaults(hall::SPECS),
            dl: vec![0.0; CHUNK],
            dr: vec![0.0; CHUNK],
            wl: vec![0.0; CHUNK],
            wr: vec![0.0; CHUNK],
        }
    }

    fn freq(p: &[f32], note: u8) -> f32 {
        midi_to_freq(note as f32 + p[TRANSPOSE].round())
    }

    fn pan_of(p: &[f32], note: u8) -> f32 {
        p[SPREAD].clamp(0.0, 1.0) * ((note as f32 - 66.0) / 24.0).clamp(-1.0, 1.0)
    }

    /// A pipe of the row: a note of the song's key.
    fn in_key(&self, n: i32) -> bool {
        let k = if self.key == 0 { C_MAJOR } else { self.key };
        (k >> n.rem_euclid(12)) & 1 == 1
    }

    /// The next pipe up (`dir` 1) or down (-1).
    fn next_pipe(&self, n: u8, dir: i32) -> u8 {
        for s in 1..=4 {
            let m = n as i32 + dir * s;
            if self.in_key(m) {
                return m.clamp(0, 127) as u8;
            }
        }
        (n as i32 + 2 * dir).clamp(0, 127) as u8
    }

    /// Neighbouring pipes: no pipe between them.
    fn adjacent(&self, a: u8, b: u8) -> bool {
        let (lo, hi) = (a.min(b) as i32, a.max(b) as i32);
        hi != lo && hi - lo <= 4 && !(lo + 1..hi).any(|n| self.in_key(n))
    }

    /// A voice to use: a quiet one, else the softest that is letting go,
    /// else the softest of the pipes passed on the way.
    fn free_voice(&self) -> usize {
        let vs = &self.voices;
        (0..MAX_VOICES)
            .find(|&i| vs[i].quiet())
            .or_else(|| (0..MAX_VOICES).filter(|&i| !vs[i].gate).min_by(|&a, &b| vs[a].env.total_cmp(&vs[b].env)))
            .or_else(|| (0..MAX_VOICES).filter(|&i| vs[i].owner == NONE).min_by(|&a, &b| vs[a].env.total_cmp(&vs[b].env)))
            .unwrap_or(MAX_VOICES - 1)
    }

    fn release_owner(&mut self, owner: u8) {
        for v in self.voices.iter_mut().filter(|v| v.owner == owner && v.gate) {
            v.stop();
        }
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        if self.stack.len() < 32 {
            self.stack.retain(|&n| n != note);
            self.stack.push(note);
        }
        if mouth_of(p) {
            if model_of(p) == 0 {
                self.pipes_on(p, note, vel);
            } else {
                self.finger_on(p, note, vel);
            }
            return;
        }
        let poly = (p[VOICES].round() as usize).clamp(1, MAX_VOICES);
        let f = Self::freq(p, note);
        let pan = Self::pan_of(p, note);
        if poly == 1 {
            let v = &mut self.voices[0];
            let legato = v.gate;
            if !legato || p[GLIDE] <= 0.001 {
                v.f = f;
            }
            if !legato {
                v.t = 0.0;
                v.vel = vel;
                v.speak = 0.0;
                v.speak_at = 0.008;
            }
            v.slur = legato;
            v.fresh = if legato { 0.25 } else { 1.0 };
            v.target = f;
            v.note = note;
            v.owner = note;
            v.amp = 1.0;
            v.delay = 0;
            v.auto_off = 0;
            v.then = None;
            v.gate = true;
            v.pan = pan;
            return;
        }
        let i = (0..poly)
            .find(|&i| self.voices[i].note == note && !self.voices[i].quiet())
            .or_else(|| (0..poly).find(|&i| self.voices[i].quiet()))
            .unwrap_or_else(|| (0..poly).min_by(|&a, &b| self.voices[a].env.total_cmp(&self.voices[b].env)).unwrap_or(0));
        self.voices[i].start(note, note, f, vel, 1.0, 1.0, 0, 0, pan);
    }

    // ------------------------------------------------------------ the lips

    /// Mouth on a pan flute: the lips on one pipe or two neighbours; further
    /// away they move along the row.
    fn pipes_on(&mut self, p: &[f32], note: u8, vel: f32) {
        self.last_vel = vel;
        let lips = self.lips;
        let n = self.n_lips;
        // Already under the lips: tongued again.
        if lips[..n].contains(&note) {
            for v in self.voices.iter_mut().filter(|v| v.owner == note) {
                v.t = 0.0;
                v.fresh = 0.8;
                v.slur = false;
                v.vel = vel;
                v.gate = true;
            }
            return;
        }
        // Next to a pipe under the lips: they cover both (and leave a third).
        if let Some(keep) = lips[..n].iter().copied().find(|&l| self.adjacent(l, note)) {
            for &l in &lips[..n] {
                if l != keep {
                    self.release_owner(l);
                }
            }
            self.lips = [keep, note];
            self.n_lips = 2;
            self.mouth = note;
            self.pipe(p, note, vel, 0.5, 0);
            return;
        }
        let from = (n > 0).then_some(self.mouth);
        self.move_lips(p, note, vel, from);
    }

    /// The lips leave what sounds and go to `note`, from `from` along the
    /// row (the pipes passed sound briefly), or straight onto it.
    fn move_lips(&mut self, p: &[f32], note: u8, vel: f32, from: Option<u8>) {
        for v in self.voices.iter_mut().filter(|v| v.gate) {
            v.stop();
        }
        self.lips = [note, note];
        self.n_lips = 1;
        self.mouth = note;
        let Some(from) = from else {
            self.pipe(p, note, vel, 1.0, 0);
            return;
        };
        let slide = p[SLIDE].clamp(0.0, 1.0);
        let step = if slide > 0.02 { ((0.01 + 0.035 * slide) * self.sr) as u32 } else { 0 };
        let mut k = 0u32;
        if step > 0 && from != note {
            let dir = if note > from { 1 } else { -1 };
            let mut n = from as i32 + dir;
            while n != note as i32 && k < 12 {
                if self.in_key(n) {
                    let i = self.free_voice();
                    let pn = n.clamp(0, 127) as u8;
                    let (f, pan) = (Self::freq(p, pn), Self::pan_of(p, pn));
                    self.voices[i].start(pn, NONE, f, vel * 0.7, 0.3 + 0.45 * slide, 0.12, k * step + step / 2, step * 3 / 2, pan);
                    k += 1;
                }
                n += dir;
            }
        }
        let delay = if k > 0 { k * step + step / 2 } else { 0 };
        self.pipe(p, note, vel, if step > 0 { 0.3 } else { 0.55 }, delay);
    }

    /// A pipe under the lips (and with Neighbours the next one caught too).
    fn pipe(&mut self, p: &[f32], note: u8, vel: f32, fresh: f32, delay: u32) {
        let i = self.free_voice();
        let (f, pan) = (Self::freq(p, note), Self::pan_of(p, note));
        self.voices[i].start(note, note, f, vel, 1.0, fresh, delay, 0, pan);
        let nb = p[NEIGHBOURS].clamp(0.0, 1.0);
        if nb > 0.02 {
            let g = self.next_pipe(note, 1);
            let j = self.free_voice();
            let (gf, gpan) = (Self::freq(p, g), Self::pan_of(p, g));
            self.voices[j].start(g, note, gf, vel * 0.8, 0.5 * nb, fresh * 0.5, delay, 0, gpan);
        }
    }

    /// Mouth on a flute with finger holes: one note, slurred under the
    /// fingers.
    fn finger_on(&mut self, p: &[f32], note: u8, vel: f32) {
        let f = Self::freq(p, note);
        let pan = Self::pan_of(p, note);
        let nb = p[NEIGHBOURS].clamp(0.0, 1.0);
        let grace = (nb > 0.02).then(|| Self::freq(p, self.next_pipe(note, 1)));
        let hold = ((0.02 + 0.06 * nb) * self.sr) as u32;
        for v in self.voices[1..].iter_mut() {
            v.stop();
        }
        let v = &mut self.voices[0];
        let legato = v.gate;
        if legato {
            let leap = (note as i32 - v.note as i32).abs();
            v.slur = true;
            // Tongued again, a crack as it overblows, or a slur.
            v.fresh = if leap == 0 { 0.8 } else if leap > 9 { 0.75 } else { 0.15 };
            if leap == 0 || leap > 9 {
                v.t = 0.0;
            }
            if p[GLIDE] <= 0.001 {
                v.f = f;
            }
        } else {
            v.t = 0.0;
            v.vel = vel;
            v.slur = false;
            v.fresh = 1.0;
            v.f = f;
            v.speak = 0.0;
            v.speak_at = 0.008;
        }
        v.target = f;
        v.note = note;
        v.owner = note;
        v.amp = 1.0;
        v.delay = 0;
        v.auto_off = 0;
        v.then = None;
        v.gate = true;
        v.pan = pan;
        // A finger flicked to the note above, just before.
        if let Some(gf) = grace {
            v.target = gf;
            if !legato {
                v.f = gf;
            }
            v.then = Some((f, hold));
        }
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.stack.retain(|&n| n != note);
        if p.len() < SPECS.len() {
            return;
        }
        if mouth_of(p) && model_of(p) == 0 {
            self.release_owner(note);
            let n = self.n_lips;
            if let Some(k) = self.lips[..n].iter().position(|&l| l == note) {
                self.lips[k] = self.lips[n - 1];
                self.n_lips -= 1;
            }
            // Back to a pipe still held: the lips go there.
            if self.n_lips == 0 {
                if let Some(&prev) = self.stack.last() {
                    let (from, vel) = (self.mouth, self.last_vel);
                    self.move_lips(p, prev, vel, Some(from));
                }
            } else {
                self.mouth = self.lips[0];
            }
            return;
        }
        let poly = if mouth_of(p) { 1 } else { (p[VOICES].round() as usize).clamp(1, MAX_VOICES) };
        if poly == 1 {
            let v = &mut self.voices[0];
            if v.note == note || v.owner == note {
                // Back to the key still held, slurred.
                if let Some(&prev) = self.stack.last() {
                    v.note = prev;
                    v.owner = prev;
                    v.target = Self::freq(p, prev);
                    v.then = None;
                    v.slur = true;
                    v.fresh = 0.15;
                    if p[GLIDE] <= 0.001 {
                        v.f = v.target;
                    }
                } else {
                    v.stop();
                }
            }
            return;
        }
        for v in self.voices.iter_mut().filter(|v| v.note == note) {
            v.stop();
        }
    }

    pub fn all_off(&mut self) {
        self.stack.clear();
        self.n_lips = 0;
        for v in self.voices.iter_mut() {
            v.stop();
        }
    }

    /// The screen: [0] the flute, [1] the breath, then per voice (2 + 2i)
    /// its note and how loud it is; [18] the key's pipes (a bit each), [19]
    /// where the lips are, [20] the material, [21] Mouth.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = 1.0;
        let mut breath = 0.0f32;
        for (i, v) in self.voices.iter().enumerate() {
            out[2 + 2 * i] = v.note as f32;
            out[3 + 2 * i] = v.level;
            breath = breath.max(v.env);
        }
        out[1] = breath;
        out[18] = if self.key == 0 { C_MAJOR } else { self.key } as f32;
        out[19] = self.mouth as f32;
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let pitch_bend = self.pitch_bend;
        if p.len() < SPECS.len() {
            return;
        }
        let sr = self.sr;
        let m = &MODEL_DATA[model_of(p)];
        let mat = &MATERIAL_DATA[(p[MATERIAL].round().max(0.0) as usize).min(MATERIALS.len() - 1)];
        let mouth = mouth_of(p);
        let k = |i: usize| p[i].clamp(0.0, 1.0);
        let (breath, chiff, tone, vibrato, bend) = (k(BREATH), k(CHIFF), k(TONE), k(VIBRATO), k(BEND));
        let vib_rate = p[VIB_RATE].clamp(1.0, 12.0);
        let vib_delay = p[VIB_DELAY].clamp(0.0, 3.0);
        let att = 1.0 - (-1.0 / (p[ATTACK].clamp(0.005, 2.0) * 0.4 * sr)).exp();
        let rel = 1.0 - (-1.0 / (p[RELEASE].clamp(0.01, 4.0) * 0.4 * sr)).exp();
        let glide = p[GLIDE].clamp(0.0, 1.0);
        let glide_k = if glide > 0.001 { 1.0 - (-1.0 / (glide * 0.35 * sr)).exp() } else { 1.0 };
        let space = k(SPACE);
        let gain = db_to_gain(p[VOLUME]) * 0.42;
        let ring = (m.ring * mat.ring).min(0.96);
        let wall_r = (-1.0 / (mat.wall_t.max(0.001) * sr)).exp();
        // How a flute blown across an edge speaks.
        let sp = m.speak;
        let speak_fresh = 1.0 - (-1.0 / ((0.012 + 0.5 * p[ATTACK].clamp(0.005, 1.0)) * sr)).exp();
        let speak_slur = 1.0 - (-1.0 / (0.008 * sr)).exp();
        let flut_a = TAU * 10.0 / sr;
        let n = l.len().min(r.len());
        let lim = 0.45 * sr;
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            self.dl[..len].fill(0.0);
            self.dr[..len].fill(0.0);
            // One breath: more pipes open, less air for each.
            let share = if mouth {
                let open: f32 = self.voices.iter().filter(|v| v.gate && v.delay == 0).map(|v| v.amp).sum();
                1.0 / (0.7 + 0.3 * open.max(1.0))
            } else {
                1.0
            };
            for v in self.voices.iter_mut() {
                if v.quiet() {
                    v.level *= 0.9;
                    continue;
                }
                // Waiting for the lips to get there.
                if v.delay > 0 {
                    if v.delay as usize >= len {
                        v.delay -= len as u32;
                        continue;
                    }
                    v.delay = 0;
                }
                // A finger moves.
                if let Some((tf, wait)) = v.then {
                    if wait as usize <= len {
                        v.target = tf;
                        v.f = tf;
                        v.then = None;
                    } else {
                        v.then = Some((tf, wait - len as u32));
                    }
                }
                v.hp.set(mat.hp, sr);
                v.lp.set(mat.lp, sr);
                v.lp2.set(mat.lp2, sr);
                let (pl, pr) = pan_gains(v.pan);
                // Brighter when blown harder, darker for a soft tone.
                let bright = (0.3 + 1.4 * tone) * mat.bright;
                let wf = (v.f * mat.wall_ratio).min(0.42 * sr);
                let wall_c = 2.0 * wall_r * (TAU * wf / sr).cos();
                let fresh = v.fresh;
                let speak_k = if v.slur { speak_slur } else { speak_fresh };
                let mut peak = 0.0f32;
                for j in 0..len {
                    v.t += 1.0 / sr;
                    v.f += (v.target - v.f) * glide_k;
                    v.env += ((if v.gate { 1.0 } else { 0.0 }) - v.env) * if v.gate { att } else { rel };
                    // A slow wander of the pitch, as a player's breath does.
                    v.drift += 0.0005 * (self.rng.bipolar() - v.drift * 0.05);
                    v.drift = v.drift.clamp(-1.0, 1.0);
                    let scoop = if v.slur { -0.3 * bend * m.scoop * (-v.t / m.scoop_t).exp() } else { -bend * m.scoop * (-v.t / m.scoop_t).exp() };
                    let vin = ((v.t - vib_delay) / 0.5).clamp(0.0, 1.0);
                    let depth = vibrato * vin * vin * (3.0 - 2.0 * vin);
                    v.vib_ph = (v.vib_ph + vib_rate / sr).fract();
                    let vib = (TAU * v.vib_ph).sin() * depth;
                    // The tone speaks after the breath, unsteady at first.
                    if sp > 0.0 {
                        if v.gate && v.t > v.speak_at {
                            v.speak += (1.0 - v.speak) * speak_k;
                        }
                        v.flut += flut_a * (68.0 * self.rng.bipolar() - v.flut);
                    } else {
                        v.speak = 1.0;
                    }
                    let unspoken = 1.0 - v.speak;
                    let rough = sp * (0.02 + 0.15 * unspoken);
                    let semis = scoop + vib * (1.0 - m.vib_amp) * 0.4 + v.drift * 0.04 + pitch_bend + sp * 0.03 * v.flut * (1.0 - 0.7 * v.speak);
                    let f = v.f * (semis * (std::f32::consts::LN_2 / 12.0)).exp();
                    let press = v.env * (0.55 + 0.45 * v.vel) * (1.0 + vib * m.vib_amp * 0.35) * share * v.amp;
                    v.phase = (v.phase + f / sr).fract();
                    // The harmonics, by recurrence from one sine and cosine.
                    let (s1, c1) = (TAU * v.phase).sin_cos();
                    let (mut sk, mut ck) = (s1, c1);
                    let b = (bright * (0.6 + 0.4 * press / v.amp.max(0.1))).min(1.8);
                    let mut y = 0.0;
                    let mut bk = 1.0;
                    let mut flash_s = 0.0;
                    for (h, &a) in m.harm.iter().enumerate() {
                        if (h + 1) as f32 * f >= lim {
                            break;
                        }
                        y += a * bk * sk;
                        if h + 1 == m.flash {
                            flash_s = sk;
                        }
                        bk *= b;
                        let s_next = sk * c1 + ck * s1;
                        ck = ck * c1 - sk * s1;
                        sk = s_next;
                    }
                    // The chiff: a puff and an overblown flash at the start (rough
                    // with the turbulence where the tone has to speak).
                    let ch = chiff * m.chiff * fresh * (0.4 + 0.6 * v.vel) * (-v.t / 0.035).exp();
                    y *= v.speak * v.speak * (1.0 + rough * v.flut);
                    y += if sp > 0.0 { ch * 0.25 * flash_s * (0.5 + 0.5 * v.flut.abs().min(2.0)) } else { ch * 0.6 * flash_s };
                    if m.buzz > 0.0 {
                        y += m.buzz * 0.4 * (y * y.abs() - 0.15);
                    }
                    // Breath: noise from the jet, rippling with the tone, then
                    // ringing in the pipe.
                    let nz = v.lp2.lp(v.lp.lp(v.hp.hp(self.rng.bipolar()))) * (1.0 + 0.6 * s1);
                    let onset = sp * unspoken * v.env * 0.25 * (0.4 + chiff) * if v.slur { 0.3 } else { 1.0 };
                    let amount = (breath * 2.0 * m.breath * (0.4 + 0.6 * press) + ch * 0.8 + onset) * mat.breath;
                    let delay = if m.stopped { sr / (2.0 * f) } else { sr / f };
                    let d = delay.clamp(2.0, (LINE - 2) as f32);
                    let rp = v.w as f32 + LINE as f32 - d;
                    let i0 = rp as usize;
                    let fr = rp - i0 as f32;
                    let a0 = v.line[i0 % LINE];
                    let a1 = v.line[(i0 + 1) % LINE];
                    let back = a0 + (a1 - a0) * fr;
                    v.loop_lp += mat.damp * (back - v.loop_lp);
                    let fb = if m.stopped { -ring } else { ring } * v.loop_lp;
                    let x = nz * amount + fb;
                    v.line[v.w] = if x.is_finite() { x } else { 0.0 };
                    v.w = (v.w + 1) % LINE;
                    // The walls ring a little of their own.
                    let mut wall = 0.0;
                    if mat.wall > 0.0 {
                        let wx = (ch * flash_s + nz * amount * 0.5 + y * 0.1 * press) * (1.0 - wall_r);
                        let w0 = wall_c * v.wall1 - wall_r * wall_r * v.wall2 + wx;
                        v.wall2 = v.wall1;
                        v.wall1 = if w0.is_finite() { w0 } else { 0.0 };
                        wall = v.wall1 * mat.wall * 2.0;
                    }
                    // Mostly what rings in the pipe (on its harmonics), only a
                    // little of the plain hiss.
                    let breath_out = (fb * (1.0 - ring) * 2.6 + nz * amount * 0.12) * 1.4 * v.amp.sqrt();
                    let out = (press * y * 0.5 + breath_out * v.env.max(ch).max(onset * 2.0) + wall) * gain;
                    peak = peak.max(out.abs());
                    self.dl[j] += out * pl;
                    self.dr[j] += out * pr;
                }
                v.level = (v.level * 0.85).max(peak * 6.0).min(1.0);
                // A pipe passed on the way stops by itself.
                if v.auto_off > 0 {
                    if v.auto_off as usize <= len {
                        v.auto_off = 0;
                        v.gate = false;
                    } else {
                        v.auto_off -= len as u32;
                    }
                }
                if !v.gate && v.env < 1e-4 {
                    v.line.fill(0.0);
                    v.loop_lp = 0.0;
                    v.wall1 = 0.0;
                    v.wall2 = 0.0;
                }
            }
            self.wl[..len].copy_from_slice(&self.dl[..len]);
            self.wr[..len].copy_from_slice(&self.dr[..len]);
            if space > 0.001 {
                let hp = &mut self.hall_p;
                hp[hall::PREDELAY] = 30.0;
                hp[hall::SIZE] = 0.55 + 0.45 * space;
                hp[hall::DECAY] = 1.8 + 5.0 * space;
                hp[hall::LOW_DECAY] = 0.9;
                hp[hall::DAMPING] = 6000.0;
                hp[hall::DIFFUSION] = 0.8;
                hp[hall::MODULATION] = 0.25;
                hp[hall::FREEZE] = 0.0;
                hp[hall::WIDTH] = 1.0;
                hp[hall::MIX] = 1.0;
                let ctx = FxCtx { sr, bpm: 120.0, beat: 0.0, playing: false };
                self.hall.process(&self.hall_p, &mut self.wl[..len], &mut self.wr[..len], &ctx, None);
                for j in 0..len {
                    self.dl[j] += self.wl[j] * space * 0.9;
                    self.dr[j] += self.wr[j] * space * 0.9;
                }
            }
            for j in 0..len {
                l[done + j] += self.dl[j].clamp(-1.5, 1.5);
                r[done + j] += self.dr[j].clamp(-1.5, 1.5);
            }
            done += len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn run(st: &mut PanFluteState, p: &[f32], secs: f32) -> (Vec<f32>, Vec<f32>) {
        let n = (secs * SR) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            st.render(p, a, b);
        }
        (l, r)
    }

    fn mag(x: &[f32], hz: f32) -> f32 {
        let w = TAU as f64 * hz as f64 / SR as f64;
        let n = x.len() as f64;
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, &s) in x.iter().enumerate() {
            let win = 0.5 - 0.5 * (TAU as f64 * i as f64 / n).cos();
            re += s as f64 * win * (w * i as f64).cos();
            im += s as f64 * win * (w * i as f64).sin();
        }
        ((re * re + im * im).sqrt() / n) as f32
    }

    /// A preset still and dry, to measure.
    fn plain(name: &str) -> Vec<f32> {
        let mut p = preset(name);
        p[VIBRATO] = 0.0;
        p[SPACE] = 0.0;
        p[BEND] = 0.0;
        p[NEIGHBOURS] = 0.0;
        p
    }

    fn steady(name: &str, model: f32) -> Vec<f32> {
        let mut p = plain(name);
        p[MODEL] = model;
        let mut st = PanFluteState::new(SR);
        st.note_on(&p, 69, 0.9);
        let (l, _) = run(&mut st, &p, 1.5);
        l[(0.5 * SR) as usize..].to_vec()
    }

    #[test]
    fn every_preset_plays_its_note_and_stays_bounded() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = PanFluteState::new(SR);
            st.note_on(&p, 67, 0.8);
            let (l, r) = run(&mut st, &p, 1.5);
            st.note_off(&p, 67);
            let (l2, _) = run(&mut st, &p, 3.0);
            let peak = l.iter().chain(&r).fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(l.iter().chain(&r).chain(&l2).all(|v| v.is_finite()), "{name}");
            assert!(peak > 0.02 && peak < 1.0, "{name}: {peak}");
            let f = midi_to_freq(67.0 + p[TRANSPOSE]);
            let tail = &l[(0.8 * SR) as usize..];
            assert!(mag(tail, f) > 4.0 * mag(tail, f * 1.26), "{name}: pitch");
            if std::env::var("FLUTE_REPORT").is_ok() {
                eprintln!("{name}: peak {peak:.3}");
            }
        }
    }

    #[test]
    fn a_pan_pipe_sounds_odd_harmonics_a_shakuhachi_all_of_them() {
        let pan = steady("Andes Pan Flute", 0.0);
        let shaku = steady("Andes Pan Flute", 1.0);
        let (f, h2, h3) = (440.0, 880.0, 1320.0);
        // The closed pipe: the third far above the second.
        assert!(mag(&pan, h3) > 5.0 * mag(&pan, h2), "pan {} {}", mag(&pan, h3), mag(&pan, h2));
        // The open one: a strong second.
        assert!(mag(&shaku, h2) > 0.15 * mag(&shaku, f), "shakuhachi {} {}", mag(&shaku, h2), mag(&shaku, f));
    }

    #[test]
    fn one_voice_glides_and_goes_back_to_the_key_still_held() {
        let mut p = plain("Andes Pan Flute");
        p[PLAYING] = 0.0;
        p[GLIDE] = 0.0;
        let mut st = PanFluteState::new(SR);
        st.note_on(&p, 60, 0.8);
        let _ = run(&mut st, &p, 0.3);
        st.note_on(&p, 64, 0.8);
        let (l, _) = run(&mut st, &p, 0.6);
        let tail = &l[(0.2 * SR) as usize..];
        assert!(mag(tail, midi_to_freq(64.0)) > 4.0 * mag(tail, midi_to_freq(60.0)));
        st.note_off(&p, 64);
        let (l, _) = run(&mut st, &p, 0.6);
        let tail = &l[(0.2 * SR) as usize..];
        assert!(mag(tail, midi_to_freq(60.0)) > 4.0 * mag(tail, midi_to_freq(64.0)));
    }

    #[test]
    fn more_voices_play_a_chord() {
        let mut p = plain("Breathy Pad");
        p[SPACE] = 0.0;
        let mut st = PanFluteState::new(SR);
        for n in [60, 64, 67] {
            st.note_on(&p, n, 0.8);
        }
        let (l, _) = run(&mut st, &p, 2.0);
        let tail = &l[SR as usize..];
        for n in [60.0, 64.0, 67.0] {
            assert!(mag(tail, midi_to_freq(n)) > 3.0 * mag(tail, midi_to_freq(n + 1.0)), "{n}");
        }
    }

    #[test]
    fn the_lips_cover_two_neighbouring_pipes_but_not_pipes_far_apart() {
        let mut p = plain("Andes Pan Flute");
        p[SLIDE] = 0.0;
        // C and D: neighbours in C major, both sound.
        let mut st = PanFluteState::new(SR);
        st.note_on(&p, 60, 0.8);
        st.note_on(&p, 62, 0.8);
        let (l, _) = run(&mut st, &p, 1.0);
        let tail = &l[(0.4 * SR) as usize..];
        let (c, d) = (mag(tail, midi_to_freq(60.0)), mag(tail, midi_to_freq(62.0)));
        assert!(c > 0.3 * d && d > 0.3 * c, "both: {c} {d}");
        // C and G: the lips go to G.
        let mut st = PanFluteState::new(SR);
        st.note_on(&p, 60, 0.8);
        let _ = run(&mut st, &p, 0.2);
        st.note_on(&p, 67, 0.8);
        let (l, _) = run(&mut st, &p, 1.0);
        let tail = &l[(0.5 * SR) as usize..];
        let (c, g) = (mag(tail, midi_to_freq(60.0)), mag(tail, midi_to_freq(67.0)));
        assert!(g > 8.0 * c, "only G: {c} {g}");
    }

    #[test]
    fn the_lips_pass_the_pipes_between_on_the_way() {
        let at = |slide: f32| {
            let mut p = plain("Andes Pan Flute");
            p[SLIDE] = slide;
            let mut st = PanFluteState::new(SR);
            st.note_on(&p, 60, 0.8);
            let _ = run(&mut st, &p, 0.3);
            st.note_on(&p, 72, 0.8);
            let (l, _) = run(&mut st, &p, 0.35);
            // G4 is passed a little after the lips leave C4.
            let w = &l[(0.12 * SR) as usize..(0.26 * SR) as usize];
            mag(w, midi_to_freq(67.0))
        };
        let (with, without) = (at(1.0), at(0.0));
        assert!(with > 3.0 * without, "passed: {with} {without}");
    }

    #[test]
    fn a_flute_with_finger_holes_plays_one_note() {
        let mut p = plain("Bansuri Evening");
        p[VOICES] = 4.0;
        let mut st = PanFluteState::new(SR);
        for n in [60, 64, 67] {
            st.note_on(&p, n, 0.8);
        }
        let (l, _) = run(&mut st, &p, 1.0);
        let tail = &l[(0.4 * SR) as usize..];
        assert!(mag(tail, midi_to_freq(67.0)) > 6.0 * mag(tail, midi_to_freq(60.0)));
        // Free, it plays the chord.
        p[PLAYING] = 0.0;
        let mut st = PanFluteState::new(SR);
        for n in [60, 64, 67] {
            st.note_on(&p, n, 0.8);
        }
        let (l, _) = run(&mut st, &p, 1.0);
        let tail = &l[(0.4 * SR) as usize..];
        assert!(mag(tail, midi_to_freq(60.0)) > 0.2 * mag(tail, midi_to_freq(67.0)));
    }

    #[test]
    fn a_bamboo_flute_speaks_out_of_its_breath_a_pan_pipe_at_once() {
        // How much of the first 20 ms is the tone, against a moment later.
        let early_share = |name: &str| {
            let mut p = plain(name);
            p[BREATH] = 0.3;
            let mut st = PanFluteState::new(SR);
            st.note_on(&p, 69, 0.9);
            let (l, _) = run(&mut st, &p, 0.3);
            let e = &l[(0.004 * SR) as usize..(0.024 * SR) as usize];
            let s = &l[(0.2 * SR) as usize..(0.22 * SR) as usize];
            let rms = |x: &[f32]| (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt();
            (mag(e, 440.0) / rms(e).max(1e-9)) / (mag(s, 440.0) / rms(s).max(1e-9))
        };
        let (bamboo, pan) = (early_share("Bansuri Evening"), early_share("Andes Pan Flute"));
        assert!(bamboo < 0.7 * pan, "bamboo {bamboo} pan {pan}");
    }

    #[test]
    fn metal_rings_brighter_than_clay() {
        let bright = |material: f32| {
            let mut p = plain("Zen Shakuhachi");
            p[MATERIAL] = material;
            p[BREATH] = 0.0;
            p[TONE] = 0.7;
            let mut st = PanFluteState::new(SR);
            st.note_on(&p, 69, 0.9);
            let (l, _) = run(&mut st, &p, 1.2);
            let t = &l[(0.4 * SR) as usize..];
            (mag(t, 1320.0) + mag(t, 1760.0)) / mag(t, 440.0)
        };
        let (metal, clay) = (bright(4.0), bright(3.0));
        assert!(metal > 1.5 * clay, "metal {metal} clay {clay}");
    }
}
