//! Voice: a singing voice. A band-limited glottal source (a discrete
//! summation of its harmonics, darker or brighter) with breath goes through
//! four formant resonators set from the 32 vowels and consonants of the
//! Synthesis ToolKit's phoneme table (`krabton_stk::phonemes`); the rest is
//! Krabton's own. Two vowels can be morphed, by hand or by a slow LFO, and
//! the formants shifted (a child or a giant). On top of that: glide (legato
//! with one voice), a choir of up to four detuned singers per note, vibrato
//! with a delay and drift, an undertone (every other glottal pulse louder,
//! so a voice an octave down growls along, as in Tibetan chant) and an
//! overtone (a narrow resonance on one harmonic, slowly swept, as in Tuvan
//! throat singing).

use super::biquad::{Coefs, State};
use super::{db_to_gain, midi_to_freq, pan_gains, Rng};
use crate::params::{ParamSpec, Unit};
use krabton_stk::phonemes;
use std::f32::consts::TAU;

pub const MODELS: &[&str] = &["Eee", "Ihh", "Ehh", "Aaa", "Ahh", "Aww", "Ohh", "Uhh", "Uuu", "Ooo", "Rrr", "Lll", "Mmm", "Nnn", "Nng", "Ngg", "Fff", "Sss", "Thh", "Shh", "Xxx", "Hee", "Hoo", "Hah", "Bbb", "Ddd", "Jjj", "Ggg", "Vvv", "Zzz", "Thz", "Zhh"];

pub const VOICES: usize = 0;
pub const VOLUME: usize = 1;
pub const BREATH: usize = 2;
pub const VIB_RATE: usize = 3;
pub const VIB_DEPTH: usize = 4;
pub const BRIGHT: usize = 5;
pub const ATTACK: usize = 6;
pub const RELEASE: usize = 7;
pub const GLIDE: usize = 8;
pub const VOWEL: usize = 9;
pub const VOWEL_B: usize = 10;
pub const MORPH: usize = 11;
pub const MORPH_RATE: usize = 12;
pub const MORPH_LFO: usize = 13;
pub const FORMANT: usize = 14;
pub const SINGERS: usize = 15;
pub const DETUNE: usize = 16;
pub const SPREAD: usize = 17;
pub const UNDERTONE: usize = 18;
pub const OVERTONE: usize = 19;
pub const HARMONIC: usize = 20;
pub const SWEEP: usize = 21;
pub const VIB_DELAY: usize = 22;
pub const DRIFT: usize = 23;
pub const OCTAVE: usize = 24;

/// The vowel (index 9) and breath (2) keep their places from the first
/// Voice, which played STK's VoicForm.
pub static SPECS: &[ParamSpec] = &[
    ParamSpec::int("Voices", "Voice", 1.0, 8.0, 4.0, Unit::None),
    ParamSpec::lin("Volume", "Voice", -24.0, 12.0, 0.0, Unit::Db),
    ParamSpec::lin("Breath", "Tone", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::log("Vib Rate", "Vibrato", 0.5, 12.0, 5.5, Unit::Hz),
    ParamSpec::lin("Vib Depth", "Vibrato", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Brightness", "Tone", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::log("Attack", "Voice", 0.005, 4.0, 0.06, Unit::Sec),
    ParamSpec::log("Release", "Voice", 0.02, 6.0, 0.3, Unit::Sec),
    ParamSpec::log("Glide", "Voice", 0.001, 2.0, 0.001, Unit::Sec),
    ParamSpec::choice("Vowel", "Vowel", MODELS, 4),
    ParamSpec::choice("Vowel B", "Vowel", MODELS, 9),
    ParamSpec::lin("Morph", "Vowel", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Morph Rate", "Vowel", 0.02, 8.0, 0.2, Unit::Hz),
    ParamSpec::lin("Morph LFO", "Vowel", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Formant", "Vowel", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::int("Singers", "Choir", 1.0, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Detune", "Choir", 0.0, 50.0, 12.0, Unit::Cents),
    ParamSpec::lin("Spread", "Choir", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Undertone", "Chant", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Overtone", "Chant", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Harmonic", "Chant", 3.0, 16.0, 8.0, Unit::None),
    ParamSpec::lin("Sweep", "Chant", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vib Delay", "Vibrato", 0.0, 2.0, 0.3, Unit::Sec),
    ParamSpec::lin("Drift", "Vibrato", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::int("Octave", "Voice", -2.0, 2.0, 0.0, Unit::None),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Ahh",
        "Ooo",
        "Eee",
        "Ohh",
        "Mmm",
        "Whisper",
        "Choir Aah",
        "Choir Ooh",
        "Gregorian",
        "Monk Chant",
        "Om Drone",
        "Throat Singer",
        "Overtone Choir",
        "Soprano",
        "Giant",
        "Legato Lead",
        "Robot Vowels",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let v = |n: &str| MODELS.iter().position(|m| *m == n).unwrap_or(4) as f32;
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, x) in pairs {
            p[i] = x;
        }
    };
    let choir = [(SINGERS, 4.0), (DETUNE, 14.0), (SPREAD, 0.8), (ATTACK, 0.4), (RELEASE, 1.2), (VIB_DEPTH, 0.15), (DRIFT, 0.5)];
    match name {
        "Ooo" => set(&[(VOWEL, v("Ooo"))]),
        "Eee" => set(&[(VOWEL, v("Eee")), (BRIGHT, 0.7)]),
        "Ohh" => set(&[(VOWEL, v("Ohh"))]),
        "Mmm" => set(&[(VOWEL, v("Mmm")), (BRIGHT, 0.4), (VIB_DEPTH, 0.1)]),
        "Whisper" => set(&[(VOWEL, v("Ahh")), (VOWEL_B, v("Ooo")), (BREATH, 1.0), (MORPH_LFO, 0.4), (MORPH_RATE, 0.3), (ATTACK, 0.15), (RELEASE, 0.6), (VIB_DEPTH, 0.0)]),
        "Choir Aah" => {
            set(&choir);
            set(&[(VOWEL, v("Ahh"))]);
        }
        "Choir Ooh" => {
            set(&choir);
            set(&[(VOWEL, v("Ooo")), (BRIGHT, 0.45)]);
        }
        "Gregorian" => {
            set(&choir);
            set(&[(VOWEL, v("Ahh")), (VOWEL_B, v("Ooo")), (MORPH_LFO, 0.25), (MORPH_RATE, 0.07), (DETUNE, 8.0), (VIB_DEPTH, 0.06), (FORMANT, -0.2), (RELEASE, 1.8)]);
        }
        "Monk Chant" => set(&[
            (OCTAVE, -1.0),
            (VOWEL, v("Ooo")),
            (VOWEL_B, v("Ohh")),
            (MORPH_LFO, 0.5),
            (MORPH_RATE, 0.08),
            (UNDERTONE, 0.75),
            (BRIGHT, 0.45),
            (SINGERS, 3.0),
            (DETUNE, 6.0),
            (SPREAD, 0.6),
            (ATTACK, 0.8),
            (RELEASE, 2.0),
            (VIB_DEPTH, 0.04),
            (DRIFT, 0.4),
            (FORMANT, -0.4),
        ]),
        "Om Drone" => set(&[(OCTAVE, -1.0), (VOWEL, v("Ohh")), (VOWEL_B, v("Mmm")), (MORPH_LFO, 1.0), (MORPH_RATE, 0.05), (UNDERTONE, 0.5), (SINGERS, 2.0), (DETUNE, 5.0), (ATTACK, 2.0), (RELEASE, 3.0), (VIB_DEPTH, 0.0), (DRIFT, 0.3), (BRIGHT, 0.4)]),
        "Throat Singer" => set(&[
            (OCTAVE, -1.0),
            (VOWEL, v("Uuu")),
            (VOWEL_B, v("Eee")),
            (MORPH_LFO, 0.6),
            (MORPH_RATE, 0.15),
            (OVERTONE, 0.9),
            (HARMONIC, 8.0),
            (SWEEP, 0.5),
            (BRIGHT, 0.85),
            (VIB_DEPTH, 0.0),
            (ATTACK, 0.3),
            (RELEASE, 0.8),
            (UNDERTONE, 0.2),
        ]),
        "Overtone Choir" => {
            set(&choir);
            set(&[(VOWEL, v("Ooo")), (VOWEL_B, v("Eee")), (MORPH_LFO, 0.5), (MORPH_RATE, 0.1), (OVERTONE, 0.7), (HARMONIC, 10.0), (SWEEP, 0.8), (BRIGHT, 0.8), (SINGERS, 3.0), (DETUNE, 6.0), (VOLUME, -4.0)]);
        }
        "Soprano" => set(&[(OCTAVE, 1.0), (FORMANT, 0.4), (VIB_DEPTH, 0.45), (VIB_DELAY, 0.4), (BRIGHT, 0.7), (ATTACK, 0.12)]),
        "Giant" => set(&[(OCTAVE, -2.0), (FORMANT, -1.0), (UNDERTONE, 0.3), (BRIGHT, 0.5), (VIB_DEPTH, 0.1), (ATTACK, 0.3), (RELEASE, 1.0)]),
        "Legato Lead" => set(&[(VOICES, 1.0), (GLIDE, 0.15), (VOWEL, v("Ohh")), (VIB_DEPTH, 0.35), (VIB_DELAY, 0.25), (BRIGHT, 0.65)]),
        "Robot Vowels" => set(&[(VOICES, 1.0), (GLIDE, 0.05), (VOWEL, v("Eee")), (VOWEL_B, v("Ooo")), (MORPH_LFO, 1.0), (MORPH_RATE, 3.0), (VIB_DEPTH, 0.0), (DRIFT, 0.0), (BRIGHT, 0.8)]),
        _ => {}
    }
    p
}

/// The four formants of phoneme `model` as (frequency in Hz, pole radius,
/// gain in dB), and its voiced and noise gains: for displays.
pub fn formants(model: usize) -> ([(f32, f32, f32); 4], (f32, f32)) {
    let k = model.min(phonemes::NAMES.len() - 1);
    let f = phonemes::PARAMETERS[k];
    let g = phonemes::GAINS[k];
    (std::array::from_fn(|i| (f[i][0] as f32, f[i][1] as f32, f[i][2] as f32)), (g[0] as f32, g[1] as f32))
}

/// Formants between vowels `a` and `b` (at `t`, 0..1), shifted by `shift`
/// octaves: (frequency, bandwidth in Hz, gain) and the voiced and noise gains.
pub fn blend(a: usize, b: usize, t: f32, shift: f32) -> ([(f32, f32, f32); 4], (f32, f32)) {
    let (fa, ga) = formants(a);
    let (fb, gb) = formants(b);
    let t = t.clamp(0.0, 1.0);
    let scale = 2f32.powf(shift);
    // The table's radii are for 44.1 kHz.
    let bw = |r: f32| -r.clamp(0.5, 0.9999).ln() * 44100.0 / std::f32::consts::PI;
    let f = std::array::from_fn(|k| {
        let freq = (fa[k].0.max(20.0).ln() * (1.0 - t) + fb[k].0.max(20.0).ln() * t).exp() * scale;
        let width = bw(fa[k].1) * (1.0 - t) + bw(fb[k].1) * t;
        let db = fa[k].2 * (1.0 - t) + fb[k].2 * t;
        (freq, width.max(30.0), db_to_gain(db))
    });
    (f, (ga.0 * (1.0 - t) + gb.0 * t, ga.1 * (1.0 - t) + gb.1 * t))
}

pub const MAX_VOICES: usize = 8;
const MAX_SINGERS: usize = 4;
/// Samples per control step (pitch, formants).
const CHUNK: usize = 32;

/// A band-pass with a peak gain of one (RBJ, constant 0 dB peak).
fn reson(f: f32, bw: f32, sr: f32) -> Coefs {
    let f = f.clamp(20.0, 0.45 * sr) as f64;
    let w0 = 2.0 * std::f64::consts::PI * f / sr as f64;
    let q = (f / bw.max(1.0) as f64).clamp(0.3, 200.0);
    let alpha = w0.sin() / (2.0 * q);
    let a0 = 1.0 + alpha;
    Coefs { b0: alpha / a0, b1: 0.0, b2: -alpha / a0, a1: -2.0 * w0.cos() / a0, a2: (1.0 - alpha) / a0 }
}

#[derive(Clone, Copy, Default)]
struct Singer {
    phase: f32,
    vib_phase: f32,
    drift: f32,
    drift_target: f32,
    /// Undertone: which glottal pulse this is, and the gain it gets.
    odd: bool,
    pulse_gain: f32,
}

#[derive(Clone)]
struct Voice {
    note: u8,
    gate: bool,
    active: bool,
    age: u64,
    vel: f32,
    /// Current and target pitch (MIDI note, fractional).
    pitch: f32,
    target: f32,
    env: f32,
    since_on: f32,
    singers: [Singer; MAX_SINGERS],
    form: [[State; 4]; 2],
    over: State,
    coefs: [Coefs; 4],
    gains: [f32; 4],
    over_coef: Coefs,
    rng: Rng,
    drift_timer: u32,
    /// Level compensation for the vowel and pitch (so every vowel is about as loud).
    comp: f32,
}

impl Voice {
    fn new(i: usize) -> Self {
        let mut rng = Rng::new(0x701C_E000 ^ (i as u32 + 1).wrapping_mul(0x9E37_79B9));
        let singers = std::array::from_fn(|_| Singer { vib_phase: (rng.bipolar() + 1.0) * 0.5, pulse_gain: 1.0, ..Default::default() });
        Self {
            note: 0,
            gate: false,
            active: false,
            age: 0,
            vel: 0.0,
            pitch: 60.0,
            target: 60.0,
            env: 0.0,
            since_on: 0.0,
            singers,
            form: Default::default(),
            over: State::default(),
            coefs: [Coefs::IDENTITY; 4],
            gains: [0.0; 4],
            over_coef: Coefs::IDENTITY,
            rng,
            drift_timer: 0,
            comp: 1.0,
        }
    }
}

pub struct VoiceState {
    /// Pitch bend in semitones (a wheel, a controller's triggers), set by the engine.
    pub pitch_bend: f32,
    voices: Vec<Voice>,
    /// Held keys, newest last (for legato with one voice).
    held: Vec<u8>,
    counter: u64,
    sr: f32,
    /// The pitch the last note ended on (poly glide starts there).
    last_pitch: f32,
    lfo: f32,
    /// Where the vowel morph and the overtone are now (for the display).
    morph_now: f32,
    harmonic_now: f32,
}

impl VoiceState {
    pub fn new(sr: f32) -> Self {
        Self {
            pitch_bend: 0.0, voices: (0..MAX_VOICES).map(Voice::new).collect(), held: Vec::with_capacity(32), counter: 0, sr, last_pitch: -1.0, lfo: 0.0, morph_now: 0.0, harmonic_now: 8.0 }
    }

    fn poly(p: &[f32]) -> usize {
        (p[VOICES].round() as usize).clamp(1, MAX_VOICES)
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let glide = p[GLIDE] > 0.0015;
        self.held.retain(|&n| n != note);
        if self.held.len() < self.held.capacity() {
            self.held.push(note);
        }
        self.counter += 1;
        let poly = Self::poly(p);
        if poly == 1 {
            // One voice: legato, gliding from note to note.
            let v = &mut self.voices[0];
            let legato = v.active && v.gate;
            v.note = note;
            v.target = note as f32;
            if !legato {
                v.pitch = if glide && v.active { v.pitch } else { note as f32 };
                v.env = if v.active { v.env } else { 0.0 };
                v.since_on = 0.0;
                v.vel = vel.clamp(0.05, 1.0);
            }
            v.gate = true;
            v.active = true;
            v.age = self.counter;
            return;
        }
        let pool = &mut self.voices[..poly];
        let pick = pool
            .iter()
            .position(|v| v.active && v.note == note)
            .or_else(|| pool.iter().position(|v| !v.active))
            .or_else(|| pool.iter().enumerate().filter(|(_, v)| !v.gate).min_by_key(|(_, v)| v.age).map(|(i, _)| i))
            .unwrap_or_else(|| pool.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0));
        let start = if glide && self.last_pitch >= 0.0 { self.last_pitch } else { note as f32 };
        let v = &mut pool[pick];
        if !v.active {
            v.env = 0.0;
            v.form = Default::default();
            v.over = State::default();
        }
        v.note = note;
        v.pitch = start;
        v.target = note as f32;
        v.vel = vel.clamp(0.05, 1.0);
        v.gate = true;
        v.active = true;
        v.since_on = 0.0;
        v.age = self.counter;
        self.last_pitch = note as f32;
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.held.retain(|&n| n != note);
        if p.len() >= SPECS.len() && Self::poly(p) == 1 {
            let v = &mut self.voices[0];
            if v.gate && v.note == note {
                match self.held.last() {
                    // Back to the key still held.
                    Some(&back) => {
                        v.note = back;
                        v.target = back as f32;
                    }
                    None => v.gate = false,
                }
            }
            return;
        }
        for v in self.voices.iter_mut().filter(|v| v.gate && v.note == note) {
            v.gate = false;
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in self.voices.iter_mut() {
            v.gate = false;
        }
    }

    /// For the display: [0] sounding voices, [1] where the vowel morph is
    /// (0: vowel, 1: vowel B), [2] the harmonic the overtone is on, [3] the
    /// newest voice's level and [4] its frequency (Hz).
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.voices.iter().filter(|v| v.active).count() as f32;
        out[1] = self.morph_now;
        out[2] = self.harmonic_now;
        if let Some(v) = self.voices.iter().filter(|v| v.active).max_by_key(|v| v.age) {
            out[3] = v.env;
            out[4] = midi_to_freq(v.pitch);
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            self.render_chunk(p, &mut l[done..done + len], &mut r[done..done + len]);
            done += len;
        }
    }

    fn render_chunk(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let pitch_bend = self.pitch_bend;
        let len = l.len();
        let sr = self.sr;
        let dt = len as f32 / sr;
        // The slow LFO moves the vowel and sweeps the overtone.
        self.lfo = (self.lfo + p[MORPH_RATE].max(0.0) * dt).fract();
        let swing = 0.5 - 0.5 * (self.lfo * TAU).cos();
        let morph = (p[MORPH] + p[MORPH_LFO] * swing * (1.0 - p[MORPH])).clamp(0.0, 1.0);
        self.morph_now = morph;
        let harmonic = (p[HARMONIC].round() + p[SWEEP] * 3.0 * (self.lfo * TAU * 0.5).sin()).max(1.5);
        self.harmonic_now = harmonic;
        if !self.voices.iter().any(|v| v.active) {
            return;
        }
        let va = (p[VOWEL].round().max(0.0) as usize).min(MODELS.len() - 1);
        let vb = (p[VOWEL_B].round().max(0.0) as usize).min(MODELS.len() - 1);
        let (forms, (voiced, noisy)) = blend(va, vb, morph, p[FORMANT].clamp(-1.0, 1.0) * 0.5);
        let breath = p[BREATH].clamp(0.0, 1.0);
        let voiced = voiced * (1.0 - 0.92 * breath * breath);
        let noise_amt = noisy * 0.25 + breath * 0.9;
        let a = 0.55 + 0.42 * p[BRIGHT].clamp(0.0, 1.0);
        let norm = (1.0 - a) * 1.6;
        let singers = (p[SINGERS].round() as usize).clamp(1, MAX_SINGERS);
        let detune = p[DETUNE] / 100.0;
        let spread = p[SPREAD].clamp(0.0, 1.0);
        let under = p[UNDERTONE].clamp(0.0, 1.0) * 0.9;
        let over = p[OVERTONE].clamp(0.0, 1.0);
        let vib_depth = p[VIB_DEPTH].clamp(0.0, 1.0) * 0.6;
        let vib_rate = p[VIB_RATE].max(0.0);
        let drift_amt = p[DRIFT].clamp(0.0, 1.0) * 0.25;
        let octave = p[OCTAVE].round() * 12.0;
        // Attack: nearly there after its time; release: 80 dB down after its time.
        let attack = 1.0 - (-4.6 / (p[ATTACK].max(0.002) * sr)).exp();
        let release = (-9.2 / (p[RELEASE].max(0.005) * sr)).exp();
        let glide = if p[GLIDE] > 0.0015 { 1.0 - (-dt / p[GLIDE]).exp() } else { 1.0 };
        let level = db_to_gain(p[VOLUME]) * 0.5;
        let offsets = [0.0, 1.0, -1.0, 0.5];
        let pans = [0.0, -1.0, 1.0, -0.45];
        let pan_g: [(f32, f32); MAX_SINGERS] = std::array::from_fn(|s| pan_gains(if singers > 1 { pans[s] * spread } else { 0.0 }));
        for v in self.voices.iter_mut().filter(|v| v.active) {
            v.pitch += (v.target - v.pitch) * glide;
            v.since_on += dt;
            let base = v.pitch + octave + pitch_bend;
            let f0 = midi_to_freq(base);
            for k in 0..4 {
                v.coefs[k] = reson(forms[k].0, forms[k].1, sr);
                v.gains[k] = forms[k].2;
            }
            v.over_coef = reson(f0 * harmonic, f0 * harmonic / (40.0 + 80.0 * over), sr);
            // How loud the formants let the harmonics through: even out vowels and pitches.
            let mut energy = 0.0f32;
            let mut ah = 1.0f32;
            for h in 1..=((0.45 * sr / f0) as usize).clamp(1, 48) {
                ah *= a;
                let fh = f0 * h as f32;
                let mut gain = 0.0f32;
                for &(fk, bw, g) in &forms {
                    let (x, q) = (fh / fk, fk / bw);
                    gain += g / (1.0 + q * q * (x - 1.0 / x) * (x - 1.0 / x)).sqrt();
                }
                energy += (ah * norm * gain).powi(2) * 0.5;
            }
            let want = (0.3 / energy.sqrt().max(0.004)).min(30.0);
            v.comp += (want - v.comp) * 0.3;
            // Drift wanders a little now and then.
            v.drift_timer = v.drift_timer.saturating_sub(len as u32);
            if v.drift_timer == 0 {
                v.drift_timer = (0.25 * sr) as u32;
                for s in v.singers.iter_mut() {
                    s.drift_target = v.rng.bipolar() * drift_amt;
                }
            }
            let fade = ((v.since_on - p[VIB_DELAY]) / 0.4).clamp(0.0, 1.0);
            // Each singer's pitch for this step.
            let mut inc = [0.0f32; MAX_SINGERS];
            let mut harmonics = [1.0f32; MAX_SINGERS];
            let mut a_n1 = [0.0f32; MAX_SINGERS];
            for (s, sg) in v.singers.iter_mut().enumerate().take(singers) {
                sg.drift += (sg.drift_target - sg.drift) * 0.05;
                sg.vib_phase = (sg.vib_phase + vib_rate * (1.0 + 0.06 * s as f32) * dt).fract();
                let vib = (sg.vib_phase * TAU).sin() * vib_depth * fade;
                let f = midi_to_freq(base + offsets[s] * detune + vib + sg.drift);
                inc[s] = f / sr;
                harmonics[s] = ((0.45 * sr / f) as usize).clamp(1, 160) as f32;
                a_n1[s] = a.powf(harmonics[s] + 1.0);
            }
            for i in 0..len {
                let (mut xl, mut xr) = (0.0f32, 0.0f32);
                for s in 0..singers {
                    let sg = &mut v.singers[s];
                    sg.phase += inc[s];
                    if sg.phase >= 1.0 {
                        sg.phase -= 1.0;
                        sg.odd = !sg.odd;
                    }
                    // Sum of harmonics 1..N with amplitudes a^k, in closed form.
                    let th = sg.phase * TAU;
                    let (nh, an1) = (harmonics[s], a_n1[s]);
                    let num = a * th.sin() - an1 * ((nh + 1.0) * th).sin() + an1 * a * (nh * th).sin();
                    let den = 1.0 - 2.0 * a * th.cos() + a * a;
                    let mut y = num / den.max(1e-6) * norm;
                    // Undertone: every other pulse louder.
                    let want = if sg.odd { 1.0 + under } else { 1.0 - under };
                    sg.pulse_gain += (want - sg.pulse_gain) * 0.02;
                    y *= sg.pulse_gain;
                    xl += y * pan_g[s].0;
                    xr += y * pan_g[s].1;
                }
                let g = 1.0 / (singers as f32).sqrt();
                let nz = v.rng.bipolar() * noise_amt;
                let (il, ir) = ((xl * g * voiced + nz) as f64, (xr * g * voiced + nz) as f64);
                let (mut yl, mut yr) = (0.0f32, 0.0f32);
                for k in 0..4 {
                    yl += v.form[0][k].run(&v.coefs[k], il) as f32 * v.gains[k];
                    yr += v.form[1][k].run(&v.coefs[k], ir) as f32 * v.gains[k];
                }
                if over > 0.0 {
                    let o = v.over.run(&v.over_coef, (il + ir) * 0.5) as f32 * over * 9.0;
                    yl = yl * (1.0 - 0.6 * over) + o;
                    yr = yr * (1.0 - 0.6 * over) + o;
                }
                if v.gate {
                    v.env += (1.0 - v.env) * attack;
                } else {
                    v.env *= release;
                }
                let amp = v.env * v.vel * level * v.comp;
                l[i] += yl * amp;
                r[i] += yr * amp;
            }
            if !v.gate && v.env < 1e-4 {
                v.active = false;
                self.last_pitch = v.pitch;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::fft::Fft;

    fn play(p: &[f32], notes: &[u8], secs: f32) -> (Vec<f32>, VoiceState) {
        let sr = 48000.0;
        let mut s = VoiceState::new(sr);
        for &n in notes {
            s.note_on(p, n, 0.9);
        }
        let mut l = vec![0.0; (secs * sr) as usize];
        let mut r = l.clone();
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            s.render(p, a, b);
        }
        (l, s)
    }

    fn peak(v: &[f32]) -> f32 {
        v.iter().fold(0.0f32, |m, x| m.max(x.abs()))
    }

    #[test]
    fn every_preset_sings_and_stops() {
        for name in preset_names() {
            let p = preset(name);
            let (l, mut s) = play(&p, &[57], 1.5);
            let pk = peak(&l);
            eprintln!("{name:<16} peak {pk:.3}");
            assert!(l.iter().all(|x| x.is_finite()), "{name}");
            assert!(pk > 0.05 && pk < 2.0, "{name}: peak {pk}");
            s.note_off(&p, 57);
            let mut l = vec![0.0; 48000 * 8];
            let mut r = l.clone();
            for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
                s.render(&p, a, b);
            }
            assert!(s.voices.iter().all(|v| !v.active), "{name}: still singing");
        }
    }

    #[test]
    fn one_voice_glides_legato() {
        let mut p = preset("Legato Lead");
        p[GLIDE] = 0.3;
        let (_, mut s) = play(&p, &[48], 0.2);
        s.note_on(&p, 60, 0.9);
        let mut l = vec![0.0; 4800];
        let mut r = l.clone();
        s.render(&p, &mut l, &mut r);
        let v = &s.voices[0];
        assert!(v.pitch > 49.0 && v.pitch < 58.0, "halfway: {}", v.pitch);
        assert!(v.env > 0.5, "no new attack");
        // Letting go of the new key goes back to the one still held.
        s.note_off(&p, 60);
        assert_eq!(s.voices[0].target, 48.0);
        assert!(s.voices[0].gate);
    }

    /// Level (dB) at frequency `f` of a mono render.
    fn level_at(x: &[f32], f: f32) -> f32 {
        let mut fft = Fft::new(8192);
        let mut out = Vec::new();
        fft.spectrum_db(x, &mut out);
        let bin = (f / 48000.0 * 8192.0).round() as usize;
        out[bin.saturating_sub(1)..=bin + 1].iter().copied().fold(-200.0, f32::max)
    }

    #[test]
    fn undertone_sings_an_octave_down() {
        let mut p = preset("Ahh");
        p[VIB_DEPTH] = 0.0;
        p[DRIFT] = 0.0;
        let f0 = midi_to_freq(48.0);
        let (plain, _) = play(&p, &[48], 1.0);
        p[UNDERTONE] = 0.8;
        let (chant, _) = play(&p, &[48], 1.0);
        let (a, b) = (level_at(&plain, f0 * 0.5), level_at(&chant, f0 * 0.5));
        assert!(b > a + 15.0, "half the frequency: {a:.1} dB plain, {b:.1} dB with undertone");
    }

    #[test]
    fn overtone_lifts_one_harmonic() {
        let mut p = preset("Ooo");
        p[VIB_DEPTH] = 0.0;
        p[DRIFT] = 0.0;
        p[HARMONIC] = 9.0;
        let f0 = midi_to_freq(45.0);
        let (plain, _) = play(&p, &[45], 1.0);
        p[OVERTONE] = 1.0;
        let (sung, _) = play(&p, &[45], 1.0);
        let lift = |x: &[f32]| level_at(x, f0 * 9.0) - level_at(x, f0 * 7.0).max(level_at(x, f0 * 11.0));
        assert!(lift(&sung) > lift(&plain) + 12.0, "9th harmonic stands out: {:.1} vs {:.1}", lift(&sung), lift(&plain));
    }

    #[test]
    fn the_vowel_morphs() {
        let (fa, _) = blend(MODELS.iter().position(|m| *m == "Ahh").unwrap(), MODELS.iter().position(|m| *m == "Eee").unwrap(), 0.0, 0.0);
        let (fm, _) = blend(MODELS.iter().position(|m| *m == "Ahh").unwrap(), MODELS.iter().position(|m| *m == "Eee").unwrap(), 0.5, 0.0);
        let (fb, _) = blend(MODELS.iter().position(|m| *m == "Ahh").unwrap(), MODELS.iter().position(|m| *m == "Eee").unwrap(), 1.0, 0.0);
        // The second formant moves up from Ahh to Eee, passing between.
        assert!(fa[1].0 < fm[1].0 && fm[1].0 < fb[1].0);
    }

    #[test]
    fn a_choir_spreads_across_the_stereo_field() {
        let p = preset("Choir Aah");
        let sr = 48000.0;
        let mut s = VoiceState::new(sr);
        s.note_on(&p, 57, 0.9);
        let (mut l, mut r) = (vec![0.0; 48000], vec![0.0; 48000]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            s.render(&p, a, b);
        }
        let diff = l.iter().zip(&r).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
        assert!(diff > 0.01, "left and right differ: {diff}");
    }
}
