//! Elements: a polyphonic instrument around the Elements modal voice
//! (Mutable Instruments, Emilie Gillet, MIT; ported to Rust in the
//! `krabton-elements` crate).
//!
//! Elements renders at 32 kHz in blocks of 16 samples: bow, blow and strike
//! exciters drive a modal resonator or a set of strings, all voices share
//! the reverb. The output is resampled to the host rate with a windowed-sinc
//! interpolator. Notes open and close the gate of a voice; velocity sets
//! its strength (accent).

use crate::params::{ParamSpec, Unit};
use krabton_elements::{Part, Patch, PerformanceState, ResonatorModel, BLOCK_SIZE, SAMPLE_RATE};

pub const MAX_VOICES: usize = 8;

pub const CONTOUR: usize = 0;
pub const VELOCITY: usize = 1;
pub const SIGNATURE: usize = 2;
pub const BOW: usize = 3;
pub const BOW_TIMBRE: usize = 4;
pub const BLOW: usize = 5;
pub const FLOW: usize = 6;
pub const BLOW_TIMBRE: usize = 7;
pub const STRIKE: usize = 8;
pub const MALLET: usize = 9;
pub const STRIKE_TIMBRE: usize = 10;
pub const GEOMETRY: usize = 11;
pub const BRIGHTNESS: usize = 12;
pub const DAMPING: usize = 13;
pub const POSITION: usize = 14;
pub const SPACE: usize = 15;
pub const FREEZE: usize = 16;
pub const SHIMMER: usize = 17;
pub const VOICES: usize = 18;
pub const TRANSPOSE: usize = 19;
pub const VOLUME: usize = 20;
/// Last, so the panel can show it as buttons instead of a knob.
pub const MODEL: usize = 21;

pub const MODEL_NAMES: &[&str] = &["Modal", "String", "Strings"];

/// What the MALLET knob selects along its range: (from, name).
pub const MALLET_ZONES: [(f32, &str); 4] = [(0.0, "Samples"), (0.4, "Mallet"), (0.6, "Plectrum"), (0.8, "Particles")];

/// The chords of the Strings model, picked by GEOMETRY.
pub const CHORD_NAMES: [&str; 11] = ["Octaves", "m7", "Minor", "m9", "m11", "Fifths", "add11", "add9", "Major", "maj7", "sus4"];

pub fn mallet_zone(mallet: f32) -> &'static str {
    MALLET_ZONES.iter().rev().find(|(from, _)| mallet >= *from).map(|z| z.1).unwrap_or("Samples")
}

/// The chord the Strings model plays (without the module's hysteresis).
pub fn chord_name(geometry: f32) -> &'static str {
    CHORD_NAMES[((geometry.clamp(0.0, 1.0) * 10.0 + 0.5) as usize).min(10)]
}

/// What GEOMETRY does in each resonator model.
pub fn geometry_name(model: f32) -> &'static str {
    match model.round() as i32 {
        1 => "Dispersion",
        2 => "Chord",
        _ => "Geometry",
    }
}

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Contour", "Exciter", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Velocity", "Exciter", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Signature", "Exciter", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Bow", "Bow", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Bow Timbre", "Bow", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Blow", "Blow", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Flow", "Blow", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Blow Timbre", "Blow", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Strike", "Strike", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Mallet", "Strike", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Strike Timbre", "Strike", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Geometry", "Resonator", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Brightness", "Resonator", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Damping", "Resonator", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Position", "Resonator", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Space", "Space", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::toggle("Freeze", "Space", false),
    ParamSpec::log("Shimmer", "Space", 0.05, 8.0, 0.5, Unit::Hz),
    ParamSpec::int("Voices", "Voice", 1.0, 8.0, 4.0, Unit::None),
    ParamSpec::int("Transpose", "Voice", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Volume", "Voice", -24.0, 6.0, 0.0, Unit::Db),
    ParamSpec::choice("Model", "Resonator", MODEL_NAMES, 0),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Init Mallet",
        "Marimba",
        "Vibraphone",
        "Glockenspiel",
        "Kalimba",
        "Steel Drum",
        "Tubular Bell",
        "Gong",
        "Wind Chimes",
        "Rain Stick",
        "Wood Block",
        "Membrane",
        "Deep Drum",
        "Metal Hit",
        "Plucked String",
        "Harp",
        "Sitar",
        "Piano-ish",
        "Strummed Chords",
        "Cello",
        "Bowed Glass",
        "Bowed Metal",
        "Bowed Chords",
        "Blown Pipe",
        "Breath Pad",
        "Ambient Drone",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    let (modal, string, strings) = (0.0, 1.0, 2.0);
    let no_strike = (STRIKE, 0.0);
    match name {
        "Marimba" => set(&[(MALLET, 0.45), (STRIKE_TIMBRE, 0.4), (GEOMETRY, 0.4), (BRIGHTNESS, 0.45), (DAMPING, 0.5), (POSITION, 0.25), (SPACE, 0.3)]),
        "Vibraphone" => set(&[(MALLET, 0.5), (STRIKE_TIMBRE, 0.55), (GEOMETRY, 0.42), (BRIGHTNESS, 0.6), (DAMPING, 0.22), (POSITION, 0.3), (SPACE, 0.6), (SHIMMER, 3.0)]),
        "Glockenspiel" => set(&[(MALLET, 0.5), (STRIKE_TIMBRE, 0.8), (GEOMETRY, 0.55), (BRIGHTNESS, 0.8), (DAMPING, 0.35), (POSITION, 0.2), (SPACE, 0.5), (TRANSPOSE, 12.0)]),
        "Kalimba" => set(&[(MALLET, 0.55), (STRIKE_TIMBRE, 0.55), (GEOMETRY, 0.31), (BRIGHTNESS, 0.65), (DAMPING, 0.55), (POSITION, 0.15), (SPACE, 0.4)]),
        "Steel Drum" => set(&[(MALLET, 0.45), (STRIKE_TIMBRE, 0.6), (GEOMETRY, 0.33), (BRIGHTNESS, 0.7), (DAMPING, 0.45), (POSITION, 0.6), (SPACE, 0.45)]),
        "Tubular Bell" => set(&[(MALLET, 0.5), (STRIKE_TIMBRE, 0.65), (GEOMETRY, 0.7), (BRIGHTNESS, 0.6), (DAMPING, 0.15), (POSITION, 0.2), (SPACE, 0.65), (VOLUME, 2.0)]),
        "Gong" => set(&[(MALLET, 0.45), (STRIKE_TIMBRE, 0.3), (GEOMETRY, 0.9), (BRIGHTNESS, 0.5), (DAMPING, 0.1), (POSITION, 0.4), (SPACE, 0.75), (TRANSPOSE, -12.0), (VOLUME, 3.0)]),
        "Wind Chimes" => set(&[(MALLET, 0.92), (STRIKE_TIMBRE, 0.7), (GEOMETRY, 0.75), (BRIGHTNESS, 0.75), (DAMPING, 0.2), (POSITION, 0.3), (SPACE, 0.75), (CONTOUR, 0.6)]),
        "Rain Stick" => set(&[(MALLET, 0.85), (STRIKE_TIMBRE, 0.55), (GEOMETRY, 0.3), (BRIGHTNESS, 0.5), (DAMPING, 0.6), (POSITION, 0.5), (SPACE, 0.55)]),
        "Wood Block" => set(&[(MALLET, 0.42), (STRIKE_TIMBRE, 0.6), (GEOMETRY, 0.45), (BRIGHTNESS, 0.6), (DAMPING, 0.85), (POSITION, 0.2), (SPACE, 0.2)]),
        "Membrane" => set(&[(MALLET, 0.45), (STRIKE_TIMBRE, 0.5), (GEOMETRY, 0.12), (BRIGHTNESS, 0.45), (DAMPING, 0.7), (POSITION, 0.15), (SPACE, 0.25), (TRANSPOSE, -12.0)]),
        "Deep Drum" => set(&[(MALLET, 0.45), (STRIKE_TIMBRE, 0.3), (GEOMETRY, 0.1), (BRIGHTNESS, 0.25), (DAMPING, 0.72), (POSITION, 0.1), (SPACE, 0.15), (TRANSPOSE, -24.0)]),
        "Metal Hit" => set(&[(MALLET, 0.15), (STRIKE_TIMBRE, 0.75), (GEOMETRY, 0.95), (BRIGHTNESS, 0.85), (DAMPING, 0.45), (POSITION, 0.35), (SPACE, 0.4)]),
        "Plucked String" => set(&[(MODEL, string), (MALLET, 0.7), (STRIKE_TIMBRE, 0.6), (GEOMETRY, 0.25), (BRIGHTNESS, 0.55), (DAMPING, 0.45), (POSITION, 0.3), (SPACE, 0.35)]),
        "Harp" => set(&[(MODEL, string), (MALLET, 0.65), (STRIKE_TIMBRE, 0.45), (GEOMETRY, 0.25), (BRIGHTNESS, 0.45), (DAMPING, 0.35), (POSITION, 0.25), (SPACE, 0.6), (VOICES, 8.0)]),
        "Sitar" => set(&[(MODEL, string), (MALLET, 0.72), (STRIKE_TIMBRE, 0.7), (GEOMETRY, 0.05), (BRIGHTNESS, 0.7), (DAMPING, 0.4), (POSITION, 0.15), (SPACE, 0.4)]),
        "Piano-ish" => set(&[(MODEL, string), (MALLET, 0.5), (STRIKE_TIMBRE, 0.5), (GEOMETRY, 0.4), (BRIGHTNESS, 0.6), (DAMPING, 0.38), (POSITION, 0.12), (SPACE, 0.4), (VOICES, 8.0), (VELOCITY, 0.8)]),
        "Strummed Chords" => set(&[(MODEL, strings), (MALLET, 0.68), (STRIKE_TIMBRE, 0.55), (GEOMETRY, 0.3), (BRIGHTNESS, 0.55), (DAMPING, 0.4), (POSITION, 0.3), (SPACE, 0.5), (VOICES, 2.0)]),
        "Cello" => set(&[no_strike, (MODEL, modal), (BOW, 1.0), (BOW_TIMBRE, 0.4), (CONTOUR, 0.7), (GEOMETRY, 0.26), (BRIGHTNESS, 0.45), (DAMPING, 0.4), (POSITION, 0.2), (SPACE, 0.45), (TRANSPOSE, -12.0), (VOICES, 2.0), (VOLUME, 5.0)]),
        "Bowed Glass" => set(&[no_strike, (BOW, 0.9), (BOW_TIMBRE, 0.5), (CONTOUR, 0.75), (GEOMETRY, 0.65), (BRIGHTNESS, 0.7), (DAMPING, 0.3), (POSITION, 0.3), (SPACE, 0.65)]),
        "Bowed Metal" => set(&[(STRIKE, 0.3), (BOW, 0.8), (BOW_TIMBRE, 0.6), (CONTOUR, 0.8), (GEOMETRY, 0.85), (BRIGHTNESS, 0.6), (DAMPING, 0.3), (POSITION, 0.4), (SPACE, 0.7), (VOLUME, 2.0)]),
        "Bowed Chords" => set(&[no_strike, (MODEL, strings), (BOW, 1.0), (BOW_TIMBRE, 0.45), (CONTOUR, 0.7), (GEOMETRY, 0.8), (BRIGHTNESS, 0.5), (DAMPING, 0.5), (POSITION, 0.3), (SPACE, 0.6), (VOICES, 2.0), (VOLUME, 6.0)]),
        "Blown Pipe" => set(&[no_strike, (BLOW, 0.8), (FLOW, 0.5), (BLOW_TIMBRE, 0.4), (CONTOUR, 0.7), (GEOMETRY, 0.25), (BRIGHTNESS, 0.4), (DAMPING, 0.5), (POSITION, 0.3), (SPACE, 0.45)]),
        "Breath Pad" => set(&[no_strike, (BLOW, 0.6), (FLOW, 0.7), (BLOW_TIMBRE, 0.6), (CONTOUR, 0.85), (GEOMETRY, 0.4), (BRIGHTNESS, 0.55), (DAMPING, 0.3), (POSITION, 0.35), (SPACE, 0.75), (VOICES, 6.0)]),
        "Ambient Drone" => set(&[(STRIKE, 0.2), (BOW, 0.5), (BLOW, 0.5), (FLOW, 0.8), (CONTOUR, 0.9), (GEOMETRY, 0.6), (BRIGHTNESS, 0.5), (DAMPING, 0.15), (POSITION, 0.4), (SPACE, 1.0), (SHIMMER, 0.2), (VOLUME, 5.0)]),
        _ => {}
    }
    p
}

/// Interpolator taps and phases of the 32 kHz to host-rate resampler.
const TAPS: usize = 32;
const HALF: usize = TAPS / 2;
const PHASES: usize = 64;

fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    for k in 1..40 {
        term *= x / (2.0 * k as f64);
        sum += term * term;
    }
    sum
}

/// Kaiser-windowed sinc kernels, one row of `TAPS` per phase (plus a last
/// row for interpolation), each summing to 1. `cutoff` is relative to the
/// input Nyquist frequency.
fn kernel(cutoff: f64) -> Vec<f32> {
    const BETA: f64 = 8.0;
    let mut k = vec![0.0f32; (PHASES + 1) * TAPS];
    for p in 0..=PHASES {
        let frac = p as f64 / PHASES as f64;
        let row = &mut k[p * TAPS..(p + 1) * TAPS];
        let mut sum = 0.0;
        let mut vals = [0.0f64; TAPS];
        for (i, v) in vals.iter_mut().enumerate() {
            let d = (HALF as f64 - 1.0 - i as f64) + frac;
            let x = d * cutoff;
            let sinc = if x.abs() < 1e-9 { 1.0 } else { (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x) };
            let t = (d / HALF as f64).clamp(-1.0, 1.0);
            *v = sinc * bessel_i0(BETA * (1.0 - t * t).sqrt()) / bessel_i0(BETA);
            sum += *v;
        }
        for (r, v) in row.iter_mut().zip(vals) {
            *r = (v / sum) as f32;
        }
    }
    k
}

#[derive(Clone, Copy, Default)]
struct Slot {
    pitch: u8,
    note: f32,
    strength: f32,
    /// The key is down.
    gate: bool,
    /// Started but not yet sent high for a block.
    fresh: bool,
    /// Needs a low block first, to strike again.
    retrigger: bool,
    /// The gate Elements saw in the last block.
    sent: bool,
    age: u64,
}

pub struct ElementsSynthState {
    part: Part,
    slots: [Slot; MAX_VOICES],
    held: Vec<u8>,
    counter: u64,
    block_l: [f32; BLOCK_SIZE],
    block_r: [f32; BLOCK_SIZE],
    block_pos: usize,
    hist_l: [f32; 2 * TAPS],
    hist_r: [f32; 2 * TAPS],
    hist_pos: usize,
    frac: f32,
    step: f32,
    kernel: Vec<f32>,
    silent_blocks: u32,
}

/// After this many silent blocks with no voice playing, rendering stops (a
/// few seconds: the reverb tail).
const SILENT_BLOCKS: u32 = (SAMPLE_RATE as u32 / BLOCK_SIZE as u32) * 3;

impl ElementsSynthState {
    pub fn new(sr: f32) -> Self {
        let cutoff = 0.9 * (sr as f64 / SAMPLE_RATE as f64).min(1.0);
        Self {
            part: Part::new(MAX_VOICES),
            slots: [Slot::default(); MAX_VOICES],
            held: Vec::with_capacity(32),
            counter: 0,
            block_l: [0.0; BLOCK_SIZE],
            block_r: [0.0; BLOCK_SIZE],
            block_pos: BLOCK_SIZE,
            hist_l: [0.0; 2 * TAPS],
            hist_r: [0.0; 2 * TAPS],
            hist_pos: 0,
            frac: 0.0,
            step: SAMPLE_RATE / sr,
            kernel: kernel(cutoff),
            silent_blocks: SILENT_BLOCKS + 1,
        }
    }

    fn voices_used(p: &[f32]) -> usize {
        (p.get(VOICES).copied().unwrap_or(4.0).round() as usize).clamp(1, MAX_VOICES)
    }

    fn strength(p: &[f32], vel: f32) -> f32 {
        (0.5 + p[VELOCITY] * (vel - 0.6) * 0.8).clamp(0.0, 1.0)
    }

    fn start(slot: &mut Slot, pitch: u8, note: f32, strength: f32, age: u64) {
        if slot.sent {
            slot.retrigger = true;
        }
        *slot = Slot { pitch, note, strength, gate: true, fresh: true, age, ..*slot };
    }

    pub fn note_on(&mut self, p: &[f32], pitch: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = Self::voices_used(p);
        let note = pitch as f32 + p[TRANSPOSE].round();
        let strength = Self::strength(p, vel);
        self.counter += 1;
        let age = self.counter;
        self.silent_blocks = 0;
        if n == 1 {
            // Mono: legato while a key is held, like a held gate on the module.
            let legato = !self.held.is_empty() && self.slots[0].gate;
            self.held.retain(|&h| h != pitch);
            if self.held.len() < self.held.capacity() {
                self.held.push(pitch);
            }
            let s = &mut self.slots[0];
            if legato {
                s.pitch = pitch;
                s.note = note;
            } else {
                Self::start(s, pitch, note, strength, age);
            }
            return;
        }
        let part = &self.part;
        let pool = &mut self.slots[..n];
        let pick = pool
            .iter()
            .position(|s| s.gate && s.pitch == pitch)
            .or_else(|| pool.iter().enumerate().position(|(i, s)| !s.gate && !s.sent && !part.voice_active(i)))
            .or_else(|| pool.iter().enumerate().filter(|(_, s)| !s.gate).min_by_key(|(_, s)| s.age).map(|(i, _)| i))
            .unwrap_or_else(|| pool.iter().enumerate().min_by_key(|(_, s)| s.age).map(|(i, _)| i).unwrap_or(0));
        Self::start(&mut pool[pick], pitch, note, strength, age);
    }

    pub fn note_off(&mut self, p: &[f32], pitch: u8) {
        if p.len() < SPECS.len() {
            return;
        }
        if Self::voices_used(p) == 1 {
            self.held.retain(|&h| h != pitch);
            let s = &mut self.slots[0];
            match self.held.last() {
                Some(&back) if s.gate => {
                    s.pitch = back;
                    s.note = back as f32 + p[TRANSPOSE].round();
                }
                _ => {
                    if s.pitch == pitch || self.held.is_empty() {
                        s.gate = false;
                    }
                }
            }
            return;
        }
        for s in self.slots.iter_mut() {
            if s.gate && s.pitch == pitch {
                s.gate = false;
            }
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for s in self.slots.iter_mut() {
            s.gate = false;
        }
    }

    /// Voices that are held or still ringing.
    pub fn active_voices(&self) -> usize {
        (0..MAX_VOICES).filter(|&i| self.slots[i].gate || self.part.voice_active(i)).count()
    }

    fn patch(p: &[f32]) -> (Patch, ResonatorModel) {
        let k = |i: usize| p[i].clamp(0.0, 0.9995);
        let patch = Patch {
            exciter_envelope_shape: p[CONTOUR].clamp(0.0, 1.0),
            exciter_bow_level: p[BOW].clamp(0.0, 1.0),
            exciter_bow_timbre: k(BOW_TIMBRE),
            exciter_blow_level: p[BLOW].clamp(0.0, 1.0),
            exciter_blow_meta: k(FLOW),
            exciter_blow_timbre: k(BLOW_TIMBRE),
            exciter_strike_level: p[STRIKE].clamp(0.0, 1.0),
            exciter_strike_meta: k(MALLET),
            exciter_strike_timbre: p[STRIKE_TIMBRE].clamp(0.0, 0.995),
            exciter_signature: k(SIGNATURE),
            resonator_geometry: k(GEOMETRY),
            resonator_brightness: k(BRIGHTNESS),
            resonator_damping: k(DAMPING),
            resonator_position: k(POSITION),
            resonator_modulation_frequency: p[SHIMMER].clamp(0.01, 20.0) / SAMPLE_RATE,
            space: if p[FREEZE] >= 0.5 { 2.0 } else { p[SPACE].clamp(0.0, 1.0) },
            ..Patch::default()
        };
        let model = match p[MODEL].round() as i32 {
            1 => ResonatorModel::String,
            2 => ResonatorModel::Strings,
            _ => ResonatorModel::Modal,
        };
        (patch, model)
    }

    /// Renders one 32 kHz block into `block_l`/`block_r`.
    fn render_block(&mut self, p: &[f32]) {
        self.block_pos = 0;
        let playing = self.slots.iter().any(|s| s.gate || s.sent || s.fresh);
        if !playing && self.silent_blocks > SILENT_BLOCKS {
            self.block_l = [0.0; BLOCK_SIZE];
            self.block_r = [0.0; BLOCK_SIZE];
            return;
        }
        let (patch, model) = Self::patch(p);
        self.part.patch = patch;
        self.part.set_resonator_model(model);
        let mut states = [PerformanceState::default(); MAX_VOICES];
        for (s, st) in self.slots.iter_mut().zip(states.iter_mut()) {
            let gate = if s.retrigger {
                s.retrigger = false;
                false
            } else if s.fresh {
                s.fresh = false;
                true
            } else {
                s.gate
            };
            s.sent = gate;
            *st = PerformanceState { gate, note: s.note, modulation: 0.0, strength: s.strength };
        }
        // Elements calls the right output "out" and the left one "aux".
        self.part.process(&states, &mut self.block_r, &mut self.block_l);
        let peak = self.block_l.iter().chain(self.block_r.iter()).fold(0.0f32, |m, x| m.max(x.abs()));
        if peak > 1e-5 || playing {
            self.silent_blocks = 0;
        } else {
            self.silent_blocks += 1;
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let gain = crate::dsp::db_to_gain(p[VOLUME]);
        for i in 0..n {
            // Output at `frac` between the two middle taps.
            let pos = self.frac * PHASES as f32;
            let row = (pos as usize).min(PHASES - 1);
            let t = pos - row as f32;
            let k0 = &self.kernel[row * TAPS..(row + 1) * TAPS];
            let k1 = &self.kernel[(row + 1) * TAPS..(row + 2) * TAPS];
            let hl = &self.hist_l[self.hist_pos..self.hist_pos + TAPS];
            let hr = &self.hist_r[self.hist_pos..self.hist_pos + TAPS];
            let (mut yl, mut yr) = (0.0, 0.0);
            for j in 0..TAPS {
                let c = k0[j] + (k1[j] - k0[j]) * t;
                yl += c * hl[j];
                yr += c * hr[j];
            }
            // The module's output stage soft-clips at twice full scale; the
            // level sits 3 dB below it so chords keep some headroom.
            l[i] += krabton_elements::dsp::soft_limit(yl * 0.5) * 1.4 * gain;
            r[i] += krabton_elements::dsp::soft_limit(yr * 0.5) * 1.4 * gain;

            self.frac += self.step;
            while self.frac >= 1.0 {
                self.frac -= 1.0;
                if self.block_pos >= BLOCK_SIZE {
                    self.render_block(p);
                }
                let (sl, sr) = (self.block_l[self.block_pos], self.block_r[self.block_pos]);
                self.block_pos += 1;
                self.hist_l[self.hist_pos] = sl;
                self.hist_l[self.hist_pos + TAPS] = sl;
                self.hist_r[self.hist_pos] = sr;
                self.hist_r[self.hist_pos + TAPS] = sr;
                self.hist_pos = (self.hist_pos + 1) % TAPS;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(st: &mut ElementsSynthState, p: &[f32], secs: f32, sr: f32) -> (Vec<f32>, Vec<f32>) {
        let n = (secs * sr) as usize;
        let mut l = vec![0.0; n];
        let mut r = vec![0.0; n];
        for (cl, cr) in l.chunks_mut(64).zip(r.chunks_mut(64)) {
            st.render(p, cl, cr);
        }
        (l, r)
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// Pitch by normalized autocorrelation with parabolic refinement.
    fn pitch_hz(x: &[f32], sr: f32, lo: f32, hi: f32) -> f32 {
        let (min_lag, max_lag) = ((sr / hi) as usize, (sr / lo) as usize);
        let n = x.len() - max_lag - 2;
        let corr = |lag: usize| {
            let (mut xy, mut xx, mut yy) = (0.0f64, 0.0f64, 0.0f64);
            for i in 0..n {
                let (a, b) = (x[i] as f64, x[i + lag] as f64);
                xy += a * b;
                xx += a * a;
                yy += b * b;
            }
            xy / (xx * yy).sqrt().max(1e-20)
        };
        let s: Vec<f64> = (min_lag..=max_lag + 1).map(corr).collect();
        let b = (1..s.len() - 1).max_by(|&a, &b| s[a].partial_cmp(&s[b]).unwrap()).unwrap();
        let shift = 0.5 * (s[b - 1] - s[b + 1]) / (s[b - 1] - 2.0 * s[b] + s[b + 1]);
        sr / ((min_lag + b) as f64 + shift) as f32
    }

    #[test]
    fn every_preset_sounds_and_stops() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = ElementsSynthState::new(48000.0);
            st.note_on(&p, 60, 0.9);
            let (l, r) = render(&mut st, &p, 0.6, 48000.0);
            let level = rms(&l).max(rms(&r));
            assert!(level > 2e-3, "{name} is silent ({level})");
            let peak = l.iter().chain(&r).fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(l.iter().chain(&r).all(|v| v.is_finite()) && peak < 1.5, "{name} peaks at {peak}");
            st.note_off(&p, 60);
            let _ = render(&mut st, &p, 12.0, 48000.0);
            assert_eq!(st.active_voices(), 0, "{name} keeps a voice");
            let (l, _) = render(&mut st, &p, 0.1, 48000.0);
            assert!(rms(&l) < 1e-4, "{name} does not fade out");
        }
    }

    #[test]
    fn in_tune_at_48k_and_44k1() {
        for sr in [48000.0f32, 44100.0] {
            let mut p = preset("Init Mallet");
            p[GEOMETRY] = 0.27; // harmonic partials
            p[SPACE] = 0.1; // dry, centered
            let mut st = ElementsSynthState::new(sr);
            st.note_on(&p, 57, 0.8);
            let (l, _) = render(&mut st, &p, 0.5, sr);
            let f = pitch_hz(&l[(0.1 * sr) as usize..], sr, 100.0, 800.0);
            assert!((f - 220.0).abs() < 2.0, "{sr}: {f} Hz");
        }
    }

    #[test]
    fn polyphony_and_retrigger() {
        let p = preset("Marimba");
        let mut st = ElementsSynthState::new(48000.0);
        for k in [60, 64, 67] {
            st.note_on(&p, k, 0.8);
        }
        render(&mut st, &p, 0.05, 48000.0);
        assert_eq!(st.active_voices(), 3);
        // Striking the same key again re-strikes the same voice.
        st.note_on(&p, 60, 0.8);
        render(&mut st, &p, 0.05, 48000.0);
        assert_eq!(st.active_voices(), 3);
        // A note shorter than a block still strikes.
        let mut st = ElementsSynthState::new(48000.0);
        st.note_on(&p, 72, 1.0);
        st.note_off(&p, 72);
        let (l, _) = render(&mut st, &p, 0.2, 48000.0);
        assert!(rms(&l) > 1e-3);
    }

    #[test]
    fn velocity_sets_the_strength() {
        let p = preset("Init Mallet");
        let level = |vel: f32| {
            let mut st = ElementsSynthState::new(48000.0);
            st.note_on(&p, 60, vel);
            let (l, r) = render(&mut st, &p, 0.3, 48000.0);
            rms(&l).max(rms(&r))
        };
        assert!(level(1.0) > level(0.3) * 1.5);
    }
}
