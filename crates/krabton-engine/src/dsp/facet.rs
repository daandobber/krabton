//! Facet: one synth with six engines, each played with four knobs (blue,
//! green, white, orange) that mean something else per engine, sharing an
//! envelope, a play mode (poly, mono, legato), glide, octave and velocity.
//! The engines are Krabton's own:
//!
//! - **Crease**: phase distortion. A cosine read at a bent pace turns into a
//!   saw, a square, a pulse, a hollow double wave or a resonant sweep (Wave
//!   morphs through them); Bend sets how far, Bend Env lets each note start
//!   brighter, Detune adds a second line beside the first.
//! - **Swarm**: seven saws around the note. Spread fans them out, Drift lets
//!   each wander on its own, Shape turns them hollow (into squares), Tone is a
//!   filter that follows the keys. Spread across the stereo field.
//! - **Fold**: a sine, frequency modulated (Ratio, Index) and folded back on
//!   itself (Fold, Symmetry): west coast timbres.
//! - **Grit**: digital. A random stepped wave made per note (Steps), pitched
//!   noise mixed in (Noise), its sample rate and bits crushed (Crush), and
//!   steps that keep changing as it plays (Mutate).
//! - **Partials**: additive, twelve harmonics: Tilt from dark to bright,
//!   Odd/Even, Stretch pulls them inharmonic (bells), Shimmer lets each
//!   swell on its own.
//! - **Wide**: two pulse waves (Detune, Width) through a filter (Tone) and a
//!   wide stereo chorus (Chorus): string machines and pads.

use super::{decay_coef, midi_to_freq, pan_gains, Adsr, AdsrCoefs, DcBlock, Rng, Svf};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::{PI, TAU};

pub const ENGINES: &[&str] = &["Crease", "Swarm", "Fold", "Grit", "Partials", "Wide"];
pub const PLAY_MODES: &[&str] = &["Poly", "Mono", "Legato"];

pub const ENGINE: usize = 0;
pub const BLUE: usize = 1;
pub const GREEN: usize = 2;
pub const WHITE: usize = 3;
pub const ORANGE: usize = 4;
pub const ATTACK: usize = 5;
pub const DECAY: usize = 6;
pub const SUSTAIN: usize = 7;
pub const RELEASE: usize = 8;
pub const PLAY: usize = 9;
pub const GLIDE: usize = 10;
pub const OCTAVE: usize = 11;
pub const VOICES: usize = 12;
pub const VELOCITY: usize = 13;
pub const VOLUME: usize = 14;

/// The parameters, the four knobs named `knobs`.
const fn specs_named(knobs: [&'static str; 4]) -> [ParamSpec; 15] {
    [
        ParamSpec::choice("Engine", "Engine", ENGINES, 0),
        ParamSpec::lin(knobs[0], "Knobs", 0.0, 1.0, 0.5, Unit::Percent),
        ParamSpec::lin(knobs[1], "Knobs", 0.0, 1.0, 0.5, Unit::Percent),
        ParamSpec::lin(knobs[2], "Knobs", 0.0, 1.0, 0.3, Unit::Percent),
        ParamSpec::lin(knobs[3], "Knobs", 0.0, 1.0, 0.3, Unit::Percent),
        ParamSpec::log("Attack", "Envelope", 0.001, 8.0, 0.004, Unit::Sec),
        ParamSpec::log("Decay", "Envelope", 0.01, 8.0, 0.5, Unit::Sec),
        ParamSpec::lin("Sustain", "Envelope", 0.0, 1.0, 0.7, Unit::Percent),
        ParamSpec::log("Release", "Envelope", 0.005, 8.0, 0.35, Unit::Sec),
        ParamSpec::choice("Play", "Play", PLAY_MODES, 0),
        ParamSpec::log("Glide", "Play", 0.001, 2.0, 0.001, Unit::Sec),
        ParamSpec::int("Octave", "Play", -3.0, 3.0, 0.0, Unit::None),
        ParamSpec::int("Voices", "Play", 1.0, 8.0, 6.0, Unit::None),
        ParamSpec::lin("Velocity", "Play", 0.0, 1.0, 0.6, Unit::Percent),
        ParamSpec::lin("Volume", "Output", -36.0, 6.0, -6.0, Unit::Db),
    ]
}

/// What the four knobs do on each engine.
const KNOBS: [[&str; 4]; 6] = [
    ["Wave", "Bend", "Bend Env", "Detune"],
    ["Spread", "Drift", "Shape", "Tone"],
    ["Fold", "Symmetry", "Ratio", "Index"],
    ["Steps", "Noise", "Crush", "Mutate"],
    ["Tilt", "Odd/Even", "Stretch", "Shimmer"],
    ["Detune", "Width", "Chorus", "Tone"],
];

static GENERIC: [ParamSpec; 15] = specs_named(["Blue", "Green", "White", "Orange"]);
static NAMED: [[ParamSpec; 15]; 6] = [specs_named(KNOBS[0]), specs_named(KNOBS[1]), specs_named(KNOBS[2]), specs_named(KNOBS[3]), specs_named(KNOBS[4]), specs_named(KNOBS[5])];

/// The parameters, the knobs by their colours.
pub static SPECS: &[ParamSpec] = &GENERIC;

/// The parameters with the knobs named for what they do on `engine`.
pub fn specs_for(engine: usize) -> &'static [ParamSpec] {
    &NAMED[engine.min(ENGINES.len() - 1)]
}

pub fn knob_names(engine: usize) -> [&'static str; 4] {
    KNOBS[engine.min(ENGINES.len() - 1)]
}

pub fn engine_of(p: &[f32]) -> usize {
    (p.get(ENGINE).copied().unwrap_or(0.0).round().max(0.0) as usize).min(ENGINES.len() - 1)
}

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Crease Keys",
        "Crease Brass",
        "Crease Reso Bass",
        "Swarm Lead",
        "Swarm Pad",
        "Hollow Swarm",
        "Fold Bell",
        "Fold Growl",
        "Grit Chip",
        "Grit Machine",
        "Partials Organ",
        "Partials Bells",
        "Wide Strings",
        "Wide Pulse Bass",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Crease Keys" => &[(ENGINE, 0.0), (BLUE, 0.1), (GREEN, 0.35), (WHITE, 0.55), (ORANGE, 0.15), (DECAY, 0.9), (SUSTAIN, 0.35), (RELEASE, 0.4)],
        "Crease Brass" => &[(ENGINE, 0.0), (BLUE, 0.0), (GREEN, 0.25), (WHITE, 0.6), (ORANGE, 0.4), (ATTACK, 0.06), (DECAY, 0.6), (SUSTAIN, 0.75), (RELEASE, 0.25), (VOICES, 4.0)],
        "Crease Reso Bass" => &[(ENGINE, 0.0), (BLUE, 1.0), (GREEN, 0.2), (WHITE, 0.8), (ORANGE, 0.0), (DECAY, 0.35), (SUSTAIN, 0.2), (RELEASE, 0.1), (PLAY, 2.0), (GLIDE, 0.05), (OCTAVE, -1.0)],
        "Swarm Lead" => &[(ENGINE, 1.0), (BLUE, 0.45), (GREEN, 0.2), (WHITE, 0.0), (ORANGE, 0.75), (SUSTAIN, 0.85), (RELEASE, 0.25), (PLAY, 2.0), (GLIDE, 0.06)],
        "Swarm Pad" => &[(ENGINE, 1.0), (BLUE, 0.7), (GREEN, 0.5), (WHITE, 0.15), (ORANGE, 0.45), (ATTACK, 0.6), (DECAY, 1.5), (SUSTAIN, 0.85), (RELEASE, 2.0)],
        "Hollow Swarm" => &[(ENGINE, 1.0), (BLUE, 0.55), (GREEN, 0.35), (WHITE, 0.8), (ORANGE, 0.55), (ATTACK, 0.15), (SUSTAIN, 0.8), (RELEASE, 0.9)],
        "Fold Bell" => &[(ENGINE, 2.0), (BLUE, 0.25), (GREEN, 0.5), (WHITE, 0.6), (ORANGE, 0.35), (DECAY, 1.6), (SUSTAIN, 0.0), (RELEASE, 1.6)],
        "Fold Growl" => &[(ENGINE, 2.0), (BLUE, 0.7), (GREEN, 0.3), (WHITE, 0.15), (ORANGE, 0.2), (SUSTAIN, 0.9), (RELEASE, 0.2), (PLAY, 1.0), (OCTAVE, -1.0)],
        "Grit Chip" => &[(ENGINE, 3.0), (BLUE, 0.05), (GREEN, 0.0), (WHITE, 0.55), (ORANGE, 0.0), (SUSTAIN, 0.8), (RELEASE, 0.08), (PLAY, 1.0)],
        "Grit Machine" => &[(ENGINE, 3.0), (BLUE, 0.6), (GREEN, 0.35), (WHITE, 0.4), (ORANGE, 0.45), (DECAY, 0.4), (SUSTAIN, 0.3), (RELEASE, 0.3)],
        "Partials Organ" => &[(ENGINE, 4.0), (BLUE, 0.55), (GREEN, 0.35), (WHITE, 0.0), (ORANGE, 0.1), (ATTACK, 0.01), (SUSTAIN, 1.0), (RELEASE, 0.1)],
        "Partials Bells" => &[(ENGINE, 4.0), (BLUE, 0.7), (GREEN, 0.5), (WHITE, 0.65), (ORANGE, 0.4), (DECAY, 2.5), (SUSTAIN, 0.0), (RELEASE, 2.5)],
        "Wide Strings" => &[(ENGINE, 5.0), (BLUE, 0.5), (GREEN, 0.3), (WHITE, 0.8), (ORANGE, 0.45), (ATTACK, 0.4), (SUSTAIN, 0.9), (RELEASE, 1.2)],
        "Wide Pulse Bass" => &[(ENGINE, 5.0), (BLUE, 0.15), (GREEN, 0.6), (WHITE, 0.1), (ORANGE, 0.35), (DECAY, 0.3), (SUSTAIN, 0.5), (RELEASE, 0.12), (PLAY, 1.0), (OCTAVE, -1.0)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

const MAX_VOICES: usize = 8;
const SAWS: usize = 7;
/// Where Swarm's saws sit around the note (relative), and in the stereo field.
const SAW_DETUNE: [f32; SAWS] = [-1.0, -0.57, -0.18, 0.0, 0.18, 0.56, 0.98];
const SAW_PAN: [f32; SAWS] = [-0.85, 0.6, -0.35, 0.0, 0.35, -0.6, 0.85];
const PARTIALS: usize = 12;
const FOLD_RATIOS: [f32; 8] = [0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 7.0];
const STEPS: usize = 64;
const CHORUS_LEN: usize = 4096;

#[inline]
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

#[inline]
fn saw(p: f32, dt: f32) -> f32 {
    2.0 * p - 1.0 - poly_blep(p, dt)
}

#[inline]
fn pulse(p: f32, dt: f32, width: f32) -> f32 {
    let mut y = if p < width { 1.0 } else { -1.0 };
    y += poly_blep(p, dt);
    y -= poly_blep((p - width + 1.0).fract(), dt);
    y
}

/// One of Crease's phase-distorted shapes at phase `p`, bent by `d` (0..1).
fn crease_shape(shape: usize, p: f32, d: f32) -> f32 {
    let knee = |q: f32, m: f32| if q < m { 0.5 * q / m } else { 0.5 + 0.5 * (q - m) / (1.0 - m) };
    match shape {
        // Saw: the first half of the cosine sped up.
        0 => -(TAU * knee(p, 0.5 - 0.48 * d)).cos(),
        // Square: each half rises quickly and then holds.
        1 => {
            let m = 1.0 - 0.95 * d;
            let (q, base) = if p < 0.5 { (p * 2.0, 0.0) } else { (p * 2.0 - 1.0, 0.5) };
            let w = base + if q < m { 0.5 * q / m } else { 0.5 };
            -(TAU * w).cos()
        }
        // Pulse: one quick cycle, then rest.
        2 => {
            let m = 1.0 - 0.94 * d;
            let w = if p < m { p / m } else { 1.0 };
            -(TAU * w).cos()
        }
        // Hollow: two bent halves, the second softer and turned over.
        3 => {
            let m = 0.5 - 0.46 * d;
            if p < 0.5 {
                -(TAU * knee(p * 2.0, m)).cos()
            } else {
                0.6 * (TAU * knee(p * 2.0 - 1.0, m)).cos()
            }
        }
        // Resonance: a sine that speeds up with the bend, fading over the cycle.
        _ => (TAU * p * (1.0 + 14.0 * d)).sin() * (1.0 - p) * 1.4 - 0.2 * (1.0 - d),
    }
}

/// Crease's Wave knob: through the five shapes, blending neighbours.
fn crease(p: f32, wave: f32, d: f32) -> f32 {
    let x = wave.clamp(0.0, 1.0) * 4.0;
    let i = (x as usize).min(3);
    let t = x - i as f32;
    crease_shape(i, p, d) * (1.0 - t) + crease_shape(i + 1, p, d) * t
}

#[inline]
fn fold(x: f32, gain: f32, bias: f32) -> f32 {
    ((x * gain + bias) * PI * 0.5).sin()
}

/// One cycle of an engine at these knobs, for its screen (n points, -1..1).
pub fn preview(engine: usize, k: [f32; 4], n: usize) -> Vec<f32> {
    let mut rng = Rng::new(0x5EED_0000 ^ (k[0] * 1000.0) as u32);
    let steps = 4 + (k[0] * 60.0) as usize;
    let wave: Vec<f32> = (0..steps).map(|_| rng.bipolar()).collect();
    (0..n)
        .map(|i| {
            let p = i as f32 / n as f32;
            match engine {
                0 => crease(p, k[0], (k[1] + 0.5 * k[2]).min(1.0)),
                1 => {
                    let s = 2.0 * p - 1.0;
                    let q = if p < 0.5 { 1.0 } else { -1.0 };
                    s * (1.0 - k[2]) + q * k[2] * 0.8
                }
                2 => {
                    let ratio = FOLD_RATIOS[(k[2] * 7.0).round() as usize];
                    let x = (TAU * p + k[3] * 4.0 * (TAU * p * ratio).sin()).sin();
                    fold(x, 1.0 + k[0] * 7.0, (k[1] - 0.5) * 1.5)
                }
                3 => {
                    let v = wave[((p * steps as f32) as usize).min(steps - 1)];
                    let bits = 16.0 - k[2] * 13.0;
                    let q = bits.exp2() * 0.5;
                    (v * q).round() / q
                }
                4 => {
                    let amps = partial_amps(k[0], k[1]);
                    let sum: f32 = amps.iter().sum::<f32>().max(1e-6);
                    (0..PARTIALS).map(|h| amps[h] * (TAU * p * (h + 1) as f32).sin()).sum::<f32>() / sum * 1.6
                }
                _ => {
                    let w = 0.5 + 0.45 * k[1];
                    if p < w {
                        0.8
                    } else {
                        -0.8
                    }
                }
            }
        })
        .collect()
}

/// The loudness of each of the twelve partials: Tilt (dark to bright), Odd/Even.
fn partial_amps(tilt: f32, odd_even: f32) -> [f32; PARTIALS] {
    let slope = 0.4 + 2.6 * (1.0 - tilt.clamp(0.0, 1.0));
    std::array::from_fn(|h| {
        let n = (h + 1) as f32;
        let mut a = n.powf(-slope);
        if h > 0 {
            let even = (h + 1) % 2 == 0;
            let oe = odd_even.clamp(0.0, 1.0);
            a *= if even { (2.0 * oe).min(1.0) } else { (2.0 - 2.0 * oe).min(1.0) };
        }
        a
    })
}

/// A sine table for the partials.
fn sine_table() -> &'static [f32; 2049] {
    static T: std::sync::OnceLock<[f32; 2049]> = std::sync::OnceLock::new();
    T.get_or_init(|| std::array::from_fn(|i| (TAU * i as f32 / 2048.0).sin()))
}

#[inline]
fn tsin(t: &[f32; 2049], p: f32) -> f32 {
    let x = p.fract() * 2048.0;
    let i = x as usize;
    let f = x - i as f32;
    t[i] + (t[i + 1] - t[i]) * f
}

#[derive(Clone)]
struct Voice {
    note: u8,
    gate: bool,
    age: u64,
    vel: f32,
    /// Where the pitch is (semitones) and where it glides to.
    pitch: f32,
    target: f32,
    env: Adsr,
    /// A decaying envelope for Crease's Bend Env (1 at the note's start).
    bend_env: f32,
    phase: [f32; SAWS],
    drift: [f32; SAWS],
    drift_to: [f32; SAWS],
    svf: [Svf; 2],
    dc: [DcBlock; 2],
    rng: Rng,
    wave: [f32; STEPS],
    hold: f32,
    hold_left: u32,
    partial: [f32; PARTIALS],
    shimmer: [f32; PARTIALS],
}

impl Voice {
    fn new(i: usize) -> Self {
        Self {
            note: 0,
            gate: false,
            age: 0,
            vel: 0.0,
            pitch: 60.0,
            target: 60.0,
            env: Adsr::new(),
            bend_env: 0.0,
            phase: [0.0; SAWS],
            drift: [0.0; SAWS],
            drift_to: [0.0; SAWS],
            svf: [Svf::new(), Svf::new()],
            dc: [DcBlock::default(), DcBlock::default()],
            rng: Rng::new(0xFACE_7000 + i as u32 * 7919),
            wave: [0.0; STEPS],
            hold: 0.0,
            hold_left: 0,
            partial: [0.0; PARTIALS],
            shimmer: std::array::from_fn(|h| h as f32 * 0.37),
        }
    }

    fn active(&self) -> bool {
        self.env.is_active()
    }

    /// A new note: its stepped wave, its saws at random phases.
    fn start(&mut self, note: u8, pitch: f32, from: Option<f32>, vel: f32, age: u64) {
        self.note = note;
        self.gate = true;
        self.age = age;
        self.vel = vel;
        self.target = pitch;
        self.pitch = from.unwrap_or(pitch);
        self.bend_env = 1.0;
        if !self.active() {
            for k in 0..SAWS {
                self.phase[k] = (self.rng.next_u32() >> 8) as f32 / 16_777_216.0;
            }
            for s in self.svf.iter_mut() {
                s.reset();
            }
        }
        for w in self.wave.iter_mut() {
            *w = self.rng.bipolar();
        }
        self.env.trigger();
    }
}

pub struct FacetState {
    pub pitch_bend: f32,
    sr: f32,
    voices: Vec<Voice>,
    /// Keys held, oldest first (mono and legato go back to them).
    held: Vec<(u8, f32)>,
    counter: u64,
    last_pitch: Option<f32>,
    /// Wide's chorus: a stereo delay line and its two sweeps.
    chorus: Vec<(f32, f32)>,
    chorus_at: usize,
    chorus_lfo: [f32; 2],
}

impl FacetState {
    pub fn new(sr: f32) -> Self {
        Self {
            pitch_bend: 0.0,
            sr,
            voices: (0..MAX_VOICES).map(Voice::new).collect(),
            held: Vec::with_capacity(32),
            counter: 0,
            last_pitch: None,
            chorus: vec![(0.0, 0.0); CHORUS_LEN],
            chorus_at: 0,
            chorus_lfo: [0.0, 0.25],
        }
    }

    fn pitch_of(p: &[f32], note: u8) -> f32 {
        note as f32 + 12.0 * p[OCTAVE].round()
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        self.counter += 1;
        let pitch = Self::pitch_of(p, note);
        let glide = p[GLIDE] > 0.002;
        let mode = p[PLAY].round() as usize;
        self.held.retain(|h| h.0 != note);
        self.held.push((note, vel));
        if mode > 0 {
            // One voice: legato keeps the envelope going while a key is held.
            let v = &mut self.voices[0];
            let legato = mode == 2 && v.gate && v.active();
            if legato {
                v.note = note;
                v.target = pitch;
                if !glide {
                    v.pitch = pitch;
                }
                v.vel = vel;
            } else {
                let from = if glide && v.active() { Some(v.pitch) } else { None };
                v.start(note, pitch, from, vel, self.counter);
            }
            self.last_pitch = Some(pitch);
            return;
        }
        let n = (p[VOICES].round() as usize).clamp(1, MAX_VOICES);
        let pool = &mut self.voices[..n];
        let pick = pool
            .iter()
            .position(|v| v.gate && v.note == note)
            .or_else(|| pool.iter().position(|v| !v.active()))
            .or_else(|| pool.iter().enumerate().filter(|(_, v)| !v.gate).min_by_key(|(_, v)| v.age).map(|(i, _)| i))
            .unwrap_or_else(|| pool.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0));
        let from = if glide { self.last_pitch } else { None };
        pool[pick].start(note, pitch, from, vel, self.counter);
        self.last_pitch = Some(pitch);
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.held.retain(|h| h.0 != note);
        let mode = p.get(PLAY).map(|m| m.round() as usize).unwrap_or(0);
        if mode > 0 {
            let v = &mut self.voices[0];
            if v.note != note {
                return;
            }
            // Back to the key still held, if any.
            if let Some(&(prev, vel)) = self.held.last() {
                let pitch = Self::pitch_of(p, prev);
                if mode == 2 {
                    v.note = prev;
                    v.target = pitch;
                    if p[GLIDE] <= 0.002 {
                        v.pitch = pitch;
                    }
                } else {
                    let from = if p[GLIDE] > 0.002 { Some(v.pitch) } else { None };
                    self.counter += 1;
                    v.start(prev, pitch, from, vel, self.counter);
                }
                return;
            }
            v.gate = false;
            v.env.release();
            return;
        }
        for v in self.voices.iter_mut().filter(|v| v.gate && v.note == note) {
            v.gate = false;
            v.env.release();
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in self.voices.iter_mut() {
            v.gate = false;
            v.env.release();
        }
    }

    /// For the screen: how many voices sound, and the loudest one's envelope.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.voices.iter().filter(|v| v.active()).count() as f32;
        out[1] = self.voices.iter().map(|v| v.env.level).fold(0.0, f32::max);
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let sr = self.sr;
        let engine = engine_of(p);
        let k = [p[BLUE], p[GREEN], p[WHITE], p[ORANGE]].map(|v| v.clamp(0.0, 1.0));
        let coefs = AdsrCoefs::new(p[ATTACK], p[DECAY], p[SUSTAIN], p[RELEASE], sr);
        let glide = if p[GLIDE] > 0.002 { 1.0 - decay_coef(p[GLIDE], sr) } else { 1.0 };
        let bend_fall = decay_coef(p[DECAY].max(0.02) * 0.6, sr);
        let gain = 10f32.powf(p[VOLUME].clamp(-60.0, 12.0) / 20.0);
        let vel_amt = p[VELOCITY].clamp(0.0, 1.0);
        let bend = self.pitch_bend;
        let table = sine_table();
        let amps = partial_amps(k[0], k[1]);
        let amp_sum: f32 = amps.iter().sum::<f32>().max(1e-6);
        let stretch = 1.0 + 0.15 * k[2];
        let ratio = FOLD_RATIOS[(k[2] * 7.0).round() as usize];
        let steps = 4 + (k[0] * 60.0) as usize;
        let crush = 1 + (k[2] * k[2] * 40.0) as u32;
        let q = (16.0 - k[2] * 13.0).exp2() * 0.5;
        let width = 0.5 + 0.45 * k[1];
        let res = if engine == 1 { 0.1 } else { 0.25 };
        for v in self.voices.iter_mut() {
            if !v.active() {
                continue;
            }
            let level = 0.32 * (1.0 - vel_amt * (1.0 - v.vel));
            for i in 0..n {
                v.pitch += (v.target - v.pitch) * glide;
                let note = v.pitch + bend;
                let f = midi_to_freq(note);
                let dt = (f / sr).min(0.45);
                let e = v.env.next(&coefs);
                v.bend_env *= bend_fall;
                let (mut a, mut b) = match engine {
                    // Crease: two lines, the second beside the first.
                    0 => {
                        let d = (k[1] + k[2] * v.bend_env).clamp(0.0, 1.0) * (1.4 - f / 2500.0).clamp(0.2, 1.0);
                        v.phase[0] = (v.phase[0] + dt).fract();
                        let x = crease(v.phase[0], k[0], d);
                        if k[3] > 0.01 {
                            v.phase[1] = (v.phase[1] + dt * (k[3] * 30.0 / 1200.0).exp2()).fract();
                            let y = crease(v.phase[1], k[0], d);
                            (x * 0.8 + y * 0.35, x * 0.35 + y * 0.8)
                        } else {
                            (x, x)
                        }
                    }
                    // Swarm: seven saws, each its own pitch and place.
                    1 => {
                        let mut acc = (0.0, 0.0);
                        for s in 0..SAWS {
                            if v.rng.next_u32() % 24000 == 0 {
                                v.drift_to[s] = v.rng.bipolar();
                            }
                            v.drift[s] += (v.drift_to[s] - v.drift[s]) * 0.00005;
                            let semis = SAW_DETUNE[s] * k[0] * k[0] * 0.9 + v.drift[s] * k[1] * 0.25;
                            let ds = (dt * (semis / 12.0).exp2()).min(0.45);
                            v.phase[s] = (v.phase[s] + ds).fract();
                            let x = saw(v.phase[s], ds) * (1.0 - k[2]) + pulse(v.phase[s], ds, 0.5) * k[2] * 0.8;
                            let g = if s == 3 { 1.0 } else { 0.55 + 0.25 * k[0] };
                            let (gl, gr) = pan_gains(SAW_PAN[s] * k[0].min(0.8) * 1.2);
                            acc.0 += x * g * gl;
                            acc.1 += x * g * gr;
                        }
                        (acc.0 * 0.32, acc.1 * 0.32)
                    }
                    // Fold: a modulated sine, folded.
                    2 => {
                        v.phase[0] = (v.phase[0] + dt).fract();
                        v.phase[1] = (v.phase[1] + dt * ratio).fract();
                        let x = (TAU * v.phase[0] + k[3] * 4.0 * (TAU * v.phase[1]).sin()).sin();
                        let g = 1.0 + k[0] * 7.0 * (1.0 - f / 3000.0).clamp(0.2, 1.0);
                        let y = fold(x, g, (k[1] - 0.5) * 1.5);
                        (y, y)
                    }
                    // Grit: the stepped wave, noise in its steps, crushed.
                    3 => {
                        let before = v.phase[0];
                        v.phase[0] = (v.phase[0] + dt).fract();
                        if v.phase[0] < before && k[3] > 0.0 {
                            // A new cycle: some steps change.
                            for w in v.wave.iter_mut().take(steps) {
                                if (v.rng.next_u32() >> 8) as f32 / 16_777_216.0 < k[3] * 0.15 {
                                    *w = v.rng.bipolar();
                                }
                            }
                        }
                        if v.hold_left == 0 {
                            let j = ((v.phase[0] * steps as f32) as usize).min(steps - 1);
                            let x = v.wave[j] * (1.0 - k[1]) + v.rng.bipolar() * k[1];
                            v.hold = (x * q).round() / q;
                            v.hold_left = crush;
                        }
                        v.hold_left -= 1;
                        (v.hold * 0.7, v.hold * 0.7)
                    }
                    // Partials: twelve sines, odd ones a little left, even right.
                    4 => {
                        let mut acc = (0.0, 0.0);
                        for h in 0..PARTIALS {
                            let fh = f * ((h + 1) as f32).powf(stretch);
                            if fh > sr * 0.45 {
                                break;
                            }
                            v.partial[h] = (v.partial[h] + fh / sr).fract();
                            let mut amp = amps[h];
                            if k[3] > 0.0 {
                                v.shimmer[h] = (v.shimmer[h] + (0.2 + 0.23 * h as f32) / sr).fract();
                                amp *= 1.0 - k[3] * 0.5 * (1.0 + tsin(table, v.shimmer[h]));
                            }
                            let x = tsin(table, v.partial[h]) * amp;
                            if h % 2 == 0 {
                                acc.0 += x * 1.1;
                                acc.1 += x * 0.9;
                            } else {
                                acc.0 += x * 0.9;
                                acc.1 += x * 1.1;
                            }
                        }
                        (acc.0 / amp_sum * 1.4, acc.1 / amp_sum * 1.4)
                    }
                    // Wide: two pulses, a little apart.
                    _ => {
                        v.phase[0] = (v.phase[0] + dt).fract();
                        let d2 = (dt * (k[0] * 25.0 / 1200.0).exp2()).min(0.45);
                        v.phase[1] = (v.phase[1] + d2).fract();
                        let x = pulse(v.phase[0], dt, width);
                        let y = pulse(v.phase[1], d2, width);
                        (x * 0.5 + y * 0.3, x * 0.3 + y * 0.5)
                    }
                };
                // Swarm and Wide through their filter (following the keys).
                if engine == 1 || engine == 5 {
                    if i % 16 == 0 {
                        let fc = (f * (1.0 + k[3] * 6.0).exp2()).min(sr * 0.45);
                        v.svf[0].set(fc, res, sr);
                        v.svf[1].set(fc, res, sr);
                    }
                    a = v.svf[0].tick(a).low;
                    b = v.svf[1].tick(b).low;
                }
                if engine == 2 || engine == 0 {
                    a = v.dc[0].tick(a);
                    b = v.dc[1].tick(b);
                }
                let g = e * level;
                l[i] += a * g;
                r[i] += b * g;
            }
        }
        // Wide's chorus, after all the voices.
        if engine == 5 && k[2] > 0.01 {
            let base = 0.007 * sr;
            let depth = 0.0035 * sr;
            let rates = [0.53 / sr, 0.79 / sr];
            for i in 0..n {
                let (x, y) = (l[i], r[i]);
                self.chorus[self.chorus_at] = (x, y);
                let mut wet = [0.0f32; 2];
                for c in 0..2 {
                    self.chorus_lfo[c] = (self.chorus_lfo[c] + rates[c]).fract();
                    let d = base + depth * (0.5 + 0.5 * tsin(table, self.chorus_lfo[c]));
                    let pos = self.chorus_at as f32 + CHORUS_LEN as f32 - d;
                    let j = pos as usize;
                    let t = pos - j as f32;
                    let s0 = self.chorus[j % CHORUS_LEN];
                    let s1 = self.chorus[(j + 1) % CHORUS_LEN];
                    let (a0, a1) = if c == 0 { (s0.0, s1.0) } else { (s0.1, s1.1) };
                    wet[c] = a0 + (a1 - a0) * t;
                }
                self.chorus_at = (self.chorus_at + 1) % CHORUS_LEN;
                l[i] = x * (1.0 - 0.5 * k[2]) + (wet[0] - 0.3 * wet[1]) * k[2];
                r[i] = y * (1.0 - 0.5 * k[2]) + (wet[1] - 0.3 * wet[0]) * k[2];
            }
        }
        for i in 0..n {
            l[i] *= gain;
            r[i] *= gain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn play(p: &[f32], notes: &[u8], secs: f32, release: f32) -> (Vec<f32>, Vec<f32>) {
        let mut s = FacetState::new(SR);
        for &n in notes {
            s.note_on(p, n, 0.9);
        }
        let (mut l, mut r) = (Vec::new(), Vec::new());
        let total = (secs * SR) as usize;
        let mut done = 0;
        let mut released = false;
        while done < total {
            if !released && done as f32 >= release * SR {
                for &n in notes {
                    s.note_off(p, n);
                }
                released = true;
            }
            let len = 256.min(total - done);
            let (mut a, mut b) = (vec![0.0; len], vec![0.0; len]);
            s.render(p, &mut a, &mut b);
            assert!(a.iter().chain(&b).all(|x| x.is_finite() && x.abs() < 4.0), "blows up");
            l.extend_from_slice(&a);
            r.extend_from_slice(&b);
            done += len;
        }
        (l, r)
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    #[test]
    fn every_preset_plays_and_stops() {
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let (l, r) = play(&p, &[57, 64], 3.0, 0.5);
            let body = rms(&l[4800..19200]) + rms(&r[4800..19200]);
            assert!(body > 0.01, "{name} is silent: {body}");
            let tail = rms(&l[l.len() - 4800..]);
            assert!(tail < 0.003, "{name} rings on: {tail}");
        }
    }

    #[test]
    fn every_engine_at_every_corner_stays_sane() {
        for e in 0..ENGINES.len() {
            for corner in 0..16 {
                let mut p = crate::params::defaults(SPECS);
                p[ENGINE] = e as f32;
                for b in 0..4 {
                    p[BLUE + b] = if corner >> b & 1 == 1 { 1.0 } else { 0.0 };
                }
                let (l, _) = play(&p, &[36, 96], 0.3, 0.2);
                assert!(rms(&l) < 1.0, "{} corner {corner}: {}", ENGINES[e], rms(&l));
            }
        }
    }

    #[test]
    fn crease_unbent_is_almost_a_sine() {
        // Unbent, a sine: almost all of it at the fundamental.
        let w: Vec<f32> = (0..1000).map(|i| crease_shape(0, i as f32 / 1000.0, 0.0)).collect();
        let fund: f32 = w.iter().enumerate().map(|(i, x)| x * -(TAU * i as f32 / 1000.0).cos()).sum::<f32>() * 2.0 / 1000.0;
        assert!((fund - 1.0).abs() < 0.01, "{fund}");
        // Bent all the way, the saw has strong upper harmonics.
        let w: Vec<f32> = (0..1000).map(|i| crease_shape(0, i as f32 / 1000.0, 1.0)).collect();
        let second: f32 = w.iter().enumerate().map(|(i, x)| x * (TAU * 2.0 * i as f32 / 1000.0).sin()).sum::<f32>() * 2.0 / 1000.0;
        assert!(second.abs() > 0.2, "{second}");
    }

    #[test]
    fn swarm_spreads_across_the_stereo_field() {
        let mut p = preset("Swarm Pad");
        p[ATTACK] = 0.001;
        let (l, r) = play(&p, &[60], 1.0, 0.9);
        let diff: Vec<f32> = l.iter().zip(&r).map(|(a, b)| a - b).collect();
        assert!(rms(&diff[9600..]) > 0.2 * rms(&l[9600..]), "wide: {} of {}", rms(&diff[9600..]), rms(&l[9600..]));
        p[BLUE] = 0.0;
        p[GREEN] = 0.0;
        let (l, r) = play(&p, &[60], 1.0, 0.9);
        let diff: Vec<f32> = l.iter().zip(&r).map(|(a, b)| a - b).collect();
        assert!(rms(&diff[9600..]) < 0.01 * rms(&l[9600..]).max(1e-6) + 1e-4, "narrow when not spread");
    }

    #[test]
    fn legato_glides_without_a_new_attack() {
        let mut p = preset("Swarm Lead");
        p[ATTACK] = 0.2;
        let mut s = FacetState::new(SR);
        s.note_on(&p, 60, 0.9);
        let mut a = vec![0.0; 24000];
        let mut b = vec![0.0; 24000];
        s.render(&p, &mut a, &mut b);
        let level = s.voices[0].env.level;
        s.note_on(&p, 67, 0.9);
        let mut a = vec![0.0; 256];
        let mut b = vec![0.0; 256];
        s.render(&p, &mut a, &mut b);
        assert!(s.voices[0].env.level >= level * 0.95, "no new attack: {level} -> {}", s.voices[0].env.level);
        assert!(s.voices[0].pitch < 66.0, "it glides: {}", s.voices[0].pitch);
        // Letting go of the new key goes back to the first.
        s.note_off(&p, 67);
        assert_eq!(s.voices[0].note, 60);
        assert!(s.voices[0].gate);
    }

    #[test]
    fn previews_fit_their_screens() {
        for e in 0..ENGINES.len() {
            let w = preview(e, [0.3, 0.6, 0.5, 0.4], 128);
            assert_eq!(w.len(), 128);
            assert!(w.iter().all(|x| x.is_finite() && x.abs() <= 1.6), "{}", ENGINES[e]);
        }
    }
}
