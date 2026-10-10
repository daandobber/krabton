//! Braids: a polyphonic instrument around the Braids macro-oscillator
//! (Mutable Instruments, Emilie Gillet, MIT; ported to Rust in the
//! `krabton-braids` crate).
//!
//! Braids renders at 96 kHz. Each voice runs at twice the host rate and is
//! decimated with a half-band filter; pitch is corrected for host rates
//! other than 48 kHz. Like the module, an AD envelope can move TIMBRE,
//! COLOR and the pitch, and the output can be bit- and rate-reduced.

use crate::params::{ParamSpec, Unit};
use krabton_braids::{MacroOscillator, BLOCK_SIZE, SHAPES};

pub const MAX_VOICES: usize = 8;
/// Output samples per Braids block (24 at the doubled rate).
const OUT_BLOCK: usize = BLOCK_SIZE / 2;
pub(crate) const FIR_TAPS: usize = 31;

/// The shapes offered in the instrument (the "?" easter egg is left out).
pub const SHAPE_COUNT: usize = 47;

pub fn shape_names() -> Vec<&'static str> {
    SHAPES[..SHAPE_COUNT].iter().map(|s| s.1).collect()
}

/// What TIMBRE and COLOR do for each shape.
pub const SHAPE_KNOBS: [[&str; 2]; SHAPE_COUNT] = [
    ["Notch Width", "Notch Depth"],
    ["Waveform", "Filter / Fuzz"],
    ["Pulse Width", "Saw / Square"],
    ["Fold", "Sine / Tri"],
    ["Harmonics", "Detune"],
    ["Pulse Width", "Sub Octave"],
    ["Saw Shape", "Sub Octave"],
    ["Sync Ratio", "Mix"],
    ["Sync Ratio", "Mix"],
    ["Interval 1", "Interval 2"],
    ["Interval 1", "Interval 2"],
    ["Interval 1", "Interval 2"],
    ["Interval 1", "Interval 2"],
    ["Ratio 1", "Ratio 2"],
    ["Detune", "High-pass"],
    ["Delay", "Feedback"],
    ["Decimation", "Bit Mangle"],
    ["Cutoff", "Shape"],
    ["Cutoff", "Shape"],
    ["Cutoff", "Shape"],
    ["Cutoff", "Shape"],
    ["Formant 1", "Formant 2"],
    ["Vowel", "Formant Shift"],
    ["Vowel", "Voice Type"],
    ["Peak", "Spread"],
    ["Index", "Ratio"],
    ["Index", "Ratio"],
    ["Index", "Ratio"],
    ["Damping", "Position"],
    ["Friction", "Bow Position"],
    ["Breath", "Bore Tone"],
    ["Breath", "Jet Ratio"],
    ["Decay", "Inharmonic"],
    ["Decay", "Noise / Tone"],
    ["Decay", "Tone"],
    ["Cutoff", "Noise"],
    ["Tone", "Snappy"],
    ["Position", "Table"],
    ["X", "Y"],
    ["Scan", "Smoothness"],
    ["Wave", "Chord"],
    ["Resonance", "LP / BP / HP"],
    ["Resonance", "Peak 2"],
    ["Period", "Steps"],
    ["Density", "Pitch Spread"],
    ["Density", "Spread"],
    ["Symbol Rate", "Data"],
];

/// Families for the shape picker: (name, first shape, count).
pub const FAMILIES: [(&str, usize, usize); 5] = [("Analog", 0, 13), ("Digital", 13, 15), ("Physical", 28, 9), ("Wavetable", 37, 4), ("Noise", 41, 6)];

pub const SHAPE: usize = 0;
pub const TIMBRE: usize = 1;
pub const COLOR: usize = 2;
pub const ENV_ATTACK: usize = 3;
pub const ENV_DECAY: usize = 4;
pub const TIMBRE_ENV: usize = 5;
pub const COLOR_ENV: usize = 6;
pub const PITCH_ENV: usize = 7;
pub const ATTACK: usize = 8;
pub const DECAY: usize = 9;
pub const SUSTAIN: usize = 10;
pub const RELEASE: usize = 11;
pub const VELOCITY: usize = 12;
pub const VOICES: usize = 13;
pub const GLIDE: usize = 14;
pub const TRANSPOSE: usize = 15;
pub const SPREAD: usize = 16;
pub const DRIFT: usize = 17;
pub const BITS: usize = 18;
pub const RATE: usize = 19;
pub const VOLUME: usize = 20;

pub const BITS_NAMES: &[&str] = &["16 bit", "12 bit", "8 bit", "6 bit", "4 bit", "3 bit", "2 bit"];
const BIT_MASKS: [u16; 7] = [0xffff, 0xfff0, 0xff00, 0xf800, 0xf000, 0xe000, 0xc000];
pub const RATE_NAMES: &[&str] = &["96 kHz", "48 kHz", "32 kHz", "24 kHz", "16 kHz", "8 kHz", "4 kHz"];
const DECIMATION: [usize; 7] = [1, 2, 3, 4, 6, 12, 24];

pub static SHAPE_LIST: [&str; SHAPE_COUNT] = {
    let mut a = [""; SHAPE_COUNT];
    let mut i = 0;
    while i < SHAPE_COUNT {
        a[i] = SHAPES[i].1;
        i += 1;
    }
    a
};

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Shape", "Oscillator", &SHAPE_LIST, 0),
    ParamSpec::lin("Timbre", "Oscillator", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Color", "Oscillator", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::log("Env Attack", "AD Env", 0.001, 4.0, 0.003, Unit::Sec),
    ParamSpec::log("Env Decay", "AD Env", 0.01, 8.0, 0.4, Unit::Sec),
    ParamSpec::lin("Timbre Env", "AD Env", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Color Env", "AD Env", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Pitch Env", "AD Env", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::log("Attack", "Amp", 0.001, 4.0, 0.003, Unit::Sec),
    ParamSpec::log("Decay", "Amp", 0.005, 8.0, 0.5, Unit::Sec),
    ParamSpec::lin("Sustain", "Amp", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::log("Release", "Amp", 0.005, 8.0, 0.25, Unit::Sec),
    ParamSpec::lin("Velocity", "Amp", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Voices", "Voice", 1.0, 8.0, 8.0, Unit::None),
    ParamSpec::log("Glide", "Voice", 0.001, 2.0, 0.001, Unit::Sec),
    ParamSpec::int("Transpose", "Voice", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Spread", "Voice", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Drift", "Voice", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Bits", "Lo-Fi", BITS_NAMES, 0),
    ParamSpec::choice("Rate", "Lo-Fi", RATE_NAMES, 0),
    ParamSpec::lin("Volume", "Lo-Fi", -24.0, 6.0, -6.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "CSaw Lead",
        "Morph Bass",
        "Sync Lead",
        "Fold Bass",
        "Buzz Pad",
        "Triple Saw",
        "Saw Swarm",
        "Ring Bells",
        "Toy Box",
        "Resonant Filter",
        "Vowel Choir",
        "Harmonic Organ",
        "FM Keys",
        "Feedback FM",
        "Plucked",
        "Bowed",
        "Blown",
        "Flute",
        "Bell",
        "Struck Drum",
        "Kick",
        "Snare",
        "Cymbal",
        "Wavetable Pad",
        "Wave Map",
        "Paraphonic",
        "Twin Peaks",
        "Clocked Noise",
        "Granular",
        "Particles",
        "QPSK Data",
        "Lo-Fi Saw",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    let perc = [(SUSTAIN, 0.0), (DECAY, 1.5), (RELEASE, 1.2)];
    match name {
        "CSaw Lead" => set(&[(VOLUME, 0.0), (SHAPE, 0.0), (TIMBRE, 0.3), (COLOR, 0.6), (VOICES, 1.0), (GLIDE, 0.05), (TIMBRE_ENV, 0.3), (SUSTAIN, 1.0)]),
        "Morph Bass" => set(&[(VOLUME, 0.0), (SHAPE, 1.0), (TIMBRE, 0.55), (COLOR, 0.3), (COLOR_ENV, 0.5), (ENV_DECAY, 0.25), (VOICES, 1.0), (TRANSPOSE, -12.0), (SUSTAIN, 0.9), (RELEASE, 0.08), (SPREAD, 0.0)]),
        "Sync Lead" => set(&[(VOLUME, 0.0), (SHAPE, 8.0), (TIMBRE, 0.4), (COLOR, 0.5), (TIMBRE_ENV, 0.4), (ENV_DECAY, 0.6), (VOICES, 1.0), (GLIDE, 0.06), (SUSTAIN, 1.0)]),
        "Fold Bass" => set(&[(VOLUME, 0.0), (SHAPE, 3.0), (TIMBRE, 0.25), (COLOR, 0.2), (TIMBRE_ENV, 0.5), (ENV_DECAY, 0.3), (VOICES, 1.0), (TRANSPOSE, -12.0), (SUSTAIN, 0.9), (SPREAD, 0.0)]),
        "Buzz Pad" => set(&[(SHAPE, 4.0), (TIMBRE, 0.4), (COLOR, 0.55), (ATTACK, 0.4), (SUSTAIN, 1.0), (RELEASE, 1.2), (SPREAD, 0.7), (DRIFT, 0.4)]),
        "Triple Saw" => set(&[(SHAPE, 9.0), (TIMBRE, 0.52), (COLOR, 0.48), (ATTACK, 0.02), (SUSTAIN, 0.9), (RELEASE, 0.5), (SPREAD, 0.6), (DRIFT, 0.3)]),
        "Saw Swarm" => set(&[(SHAPE, 14.0), (TIMBRE, 0.4), (COLOR, 0.3), (ATTACK, 0.3), (SUSTAIN, 1.0), (RELEASE, 1.5), (SPREAD, 0.8)]),
        "Ring Bells" => {
            set(&perc);
            set(&[(SHAPE, 13.0), (TIMBRE, 0.7), (COLOR, 0.35), (SPREAD, 0.6)]);
        }
        "Toy Box" => set(&[(SHAPE, 16.0), (TIMBRE, 0.3), (COLOR, 0.4), (SUSTAIN, 0.6), (RELEASE, 0.2)]),
        "Resonant Filter" => set(&[(SHAPE, 17.0), (TIMBRE, 0.35), (COLOR, 0.3), (TIMBRE_ENV, 0.5), (ENV_DECAY, 0.3), (SUSTAIN, 0.8), (VOICES, 4.0)]),
        "Vowel Choir" => set(&[(SHAPE, 23.0), (TIMBRE, 0.3), (COLOR, 0.6), (ATTACK, 0.25), (SUSTAIN, 1.0), (RELEASE, 1.0), (SPREAD, 0.7), (DRIFT, 0.3)]),
        "Harmonic Organ" => set(&[(SHAPE, 24.0), (TIMBRE, 0.35), (COLOR, 0.2), (ATTACK, 0.01), (SUSTAIN, 1.0), (RELEASE, 0.15)]),
        "FM Keys" => set(&[(SHAPE, 25.0), (TIMBRE, 0.35), (COLOR, 0.5), (TIMBRE_ENV, 0.4), (ENV_DECAY, 0.6), (DECAY, 1.4), (SUSTAIN, 0.4), (RELEASE, 0.4)]),
        "Feedback FM" => set(&[(SHAPE, 26.0), (TIMBRE, 0.4), (COLOR, 0.45), (TIMBRE_ENV, 0.3), (SUSTAIN, 0.8)]),
        "Plucked" => {
            set(&perc);
            set(&[(SHAPE, 28.0), (TIMBRE, 0.3), (COLOR, 0.5), (SPREAD, 0.5)]);
        }
        "Bowed" => set(&[(SHAPE, 29.0), (TIMBRE, 0.5), (COLOR, 0.4), (ATTACK, 0.15), (SUSTAIN, 1.0), (RELEASE, 0.6)]),
        "Blown" => set(&[(SHAPE, 30.0), (TIMBRE, 0.5), (COLOR, 0.5), (ATTACK, 0.08), (SUSTAIN, 1.0), (RELEASE, 0.3)]),
        "Flute" => set(&[(SHAPE, 31.0), (TIMBRE, 0.4), (COLOR, 0.5), (ATTACK, 0.06), (SUSTAIN, 1.0), (RELEASE, 0.3)]),
        "Bell" => {
            set(&perc);
            set(&[(SHAPE, 32.0), (TIMBRE, 0.6), (COLOR, 0.3), (DECAY, 3.0), (RELEASE, 2.5), (SPREAD, 0.6)]);
        }
        "Struck Drum" => {
            set(&perc);
            set(&[(SHAPE, 33.0), (TIMBRE, 0.5), (COLOR, 0.4), (SPREAD, 0.3)]);
        }
        "Kick" => {
            set(&perc);
            set(&[(SHAPE, 34.0), (TIMBRE, 0.5), (COLOR, 0.3), (TRANSPOSE, -24.0), (SPREAD, 0.0)]);
        }
        "Snare" => {
            set(&perc);
            set(&[(SHAPE, 36.0), (TIMBRE, 0.5), (COLOR, 0.5), (TRANSPOSE, -12.0), (SPREAD, 0.0)]);
        }
        "Cymbal" => {
            set(&perc);
            set(&[(SHAPE, 35.0), (TIMBRE, 0.7), (COLOR, 0.5), (DECAY, 0.8), (RELEASE, 0.6), (SPREAD, 0.2)]);
        }
        "Wavetable Pad" => set(&[(SHAPE, 37.0), (TIMBRE, 0.3), (COLOR, 0.45), (TIMBRE_ENV, 0.4), (ENV_ATTACK, 1.0), (ENV_DECAY, 3.0), (ATTACK, 0.5), (SUSTAIN, 1.0), (RELEASE, 1.5), (SPREAD, 0.7)]),
        "Wave Map" => set(&[(SHAPE, 38.0), (TIMBRE, 0.4), (COLOR, 0.6), (SUSTAIN, 0.9), (RELEASE, 0.6)]),
        "Paraphonic" => set(&[(VOLUME, 0.0), (SHAPE, 40.0), (TIMBRE, 0.3), (COLOR, 0.6), (VOICES, 2.0), (ATTACK, 0.2), (SUSTAIN, 1.0), (RELEASE, 1.0)]),
        "Twin Peaks" => set(&[(SHAPE, 42.0), (TIMBRE, 0.8), (COLOR, 0.6), (SUSTAIN, 1.0), (RELEASE, 0.5)]),
        "Clocked Noise" => set(&[(SHAPE, 43.0), (TIMBRE, 0.5), (COLOR, 0.3), (SUSTAIN, 0.8), (RELEASE, 0.2), (VOICES, 2.0)]),
        "Granular" => set(&[(SHAPE, 44.0), (TIMBRE, 0.6), (COLOR, 0.3), (ATTACK, 0.3), (SUSTAIN, 1.0), (RELEASE, 1.5), (SPREAD, 0.8)]),
        "Particles" => set(&[(SHAPE, 45.0), (TIMBRE, 0.4), (COLOR, 0.5), (SUSTAIN, 1.0), (RELEASE, 1.5), (SPREAD, 0.8)]),
        "QPSK Data" => set(&[(SHAPE, 46.0), (TIMBRE, 0.5), (COLOR, 0.5), (SUSTAIN, 1.0), (RELEASE, 0.2)]),
        "Lo-Fi Saw" => set(&[(SHAPE, 0.0), (TIMBRE, 0.5), (COLOR, 0.5), (BITS, 4.0), (RATE, 4.0), (SUSTAIN, 0.9), (VOLUME, -10.0)]),
        _ => {}
    }
    p
}

/// A windowed-sinc half-band low-pass for 2:1 decimation.
pub(crate) fn halfband() -> [f32; FIR_TAPS] {
    let mut h = [0.0f32; FIR_TAPS];
    let m = (FIR_TAPS - 1) as f32 / 2.0;
    let mut sum = 0.0;
    for (i, c) in h.iter_mut().enumerate() {
        let x = i as f32 - m;
        let sinc = if x == 0.0 { 1.0 } else { (std::f32::consts::PI * x * 0.5).sin() / (std::f32::consts::PI * x * 0.5) };
        let w = 0.42 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (FIR_TAPS - 1) as f32).cos() + 0.08 * (4.0 * std::f32::consts::PI * i as f32 / (FIR_TAPS - 1) as f32).cos();
        *c = sinc * w;
        sum += *c;
    }
    for c in h.iter_mut() {
        *c /= sum;
    }
    h
}

struct BraidsVoice {
    osc: MacroOscillator,
    jitter: krabton_braids::jitter::VcoJitterSource,
    history: [f32; FIR_TAPS],
    hist_pos: usize,
    out: [f32; OUT_BLOCK],
    pos: usize,
    pitch: u8,
    note: f32,
    target: f32,
    gate: bool,
    strike: bool,
    velocity: f32,
    env: f32,
    stage: u8,
    ad: f32,
    ad_stage: u8,
    active: bool,
    age: u64,
    pan: f32,
    held: i16,
    hold_count: usize,
}

impl BraidsVoice {
    fn new(index: usize) -> Self {
        const PANS: [f32; MAX_VOICES] = [-1.0, 1.0, -0.5, 0.5, -0.75, 0.75, -0.25, 0.25];
        Self {
            osc: MacroOscillator::new(),
            jitter: Default::default(),
            history: [0.0; FIR_TAPS],
            hist_pos: 0,
            out: [0.0; OUT_BLOCK],
            pos: OUT_BLOCK,
            pitch: 0,
            note: 60.0,
            target: 60.0,
            gate: false,
            strike: false,
            velocity: 0.0,
            env: 0.0,
            stage: 0,
            ad: 0.0,
            ad_stage: 0,
            active: false,
            age: 0,
            pan: PANS[index % MAX_VOICES],
            held: 0,
            hold_count: 0,
        }
    }

    fn start(&mut self, pitch: u8, note: f32, vel: f32, glide: bool, age: u64) {
        if !glide || !self.active {
            self.note = note;
        }
        self.target = note;
        self.pitch = pitch;
        self.gate = true;
        self.strike = true;
        self.velocity = vel;
        self.stage = 1;
        self.ad_stage = 1;
        self.active = true;
        self.age = age;
    }

    fn release(&mut self) {
        self.gate = false;
        if self.stage != 0 {
            self.stage = 4;
        }
    }
}

pub struct BraidsSynthState {
    voices: Vec<BraidsVoice>,
    held: Vec<u8>,
    counter: u64,
    sr: f32,
    fir: [f32; FIR_TAPS],
    /// Pitch correction (1/128 semitones) when running at 2x a rate other than 48 kHz.
    rate_offset: i32,
}

impl BraidsSynthState {
    pub fn new(sr: f32) -> Self {
        let rate_offset = (12.0 * (krabton_braids::SAMPLE_RATE / (2.0 * sr)).log2() * 128.0).round() as i32;
        Self { voices: (0..MAX_VOICES).map(BraidsVoice::new).collect(), held: Vec::with_capacity(32), counter: 0, sr, fir: halfband(), rate_offset }
    }

    fn voices_used(p: &[f32]) -> usize {
        (p.get(VOICES).copied().unwrap_or(8.0).round() as usize).clamp(1, MAX_VOICES)
    }

    pub fn note_on(&mut self, p: &[f32], pitch: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = Self::voices_used(p);
        let note = pitch as f32 + p[TRANSPOSE].round();
        let glide = p[GLIDE] > 0.0015;
        self.counter += 1;
        let age = self.counter;
        if n == 1 {
            let legato = !self.held.is_empty() && self.voices[0].gate;
            self.held.retain(|&h| h != pitch);
            if self.held.len() < self.held.capacity() {
                self.held.push(pitch);
            }
            let v = &mut self.voices[0];
            if legato {
                v.target = note;
                v.pitch = pitch;
                if !glide {
                    v.note = note;
                }
            } else {
                v.start(pitch, note, vel, glide, age);
            }
            return;
        }
        let pool = &mut self.voices[..n];
        let pick = pool
            .iter()
            .position(|v| v.gate && v.pitch == pitch)
            .or_else(|| pool.iter().position(|v| !v.active))
            .or_else(|| pool.iter().enumerate().filter(|(_, v)| !v.gate).min_by_key(|(_, v)| v.age).map(|(i, _)| i))
            .unwrap_or_else(|| pool.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0));
        pool[pick].start(pitch, note, vel, glide, age);
    }

    pub fn note_off(&mut self, p: &[f32], pitch: u8) {
        if p.len() < SPECS.len() {
            return;
        }
        if Self::voices_used(p) == 1 {
            self.held.retain(|&h| h != pitch);
            let v = &mut self.voices[0];
            match self.held.last() {
                Some(&back) if v.gate => {
                    v.target = back as f32 + p[TRANSPOSE].round();
                    v.pitch = back;
                    if p[GLIDE] <= 0.0015 {
                        v.note = v.target;
                    }
                }
                _ => {
                    if v.pitch == pitch || self.held.is_empty() {
                        v.release();
                    }
                }
            }
            return;
        }
        for v in self.voices.iter_mut() {
            if v.gate && v.pitch == pitch {
                v.release();
            }
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in self.voices.iter_mut() {
            v.release();
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let shape = (p[SHAPE].round().max(0.0) as usize).min(SHAPE_COUNT - 1);
        let block_dt = OUT_BLOCK as f32 / self.sr;
        let glide_k = 1.0 - (-block_dt / p[GLIDE].max(1e-4)).exp();
        let attack_step = block_dt / p[ATTACK].max(1e-4);
        let decay_k = 1.0 - (-block_dt * 4.6 / p[DECAY].max(1e-4)).exp();
        let release_k = 1.0 - (-block_dt * 4.6 / p[RELEASE].max(1e-4)).exp();
        let ad_attack = block_dt / p[ENV_ATTACK].max(1e-4);
        let ad_decay_k = 1.0 - (-block_dt * 4.6 / p[ENV_DECAY].max(1e-4)).exp();
        let sustain = p[SUSTAIN];
        let master = crate::dsp::db_to_gain(p[VOLUME]) * 0.33 / 32768.0;
        let spread = p[SPREAD];
        let vel_amt = p[VELOCITY];
        let drift = (p[DRIFT].clamp(0.0, 1.0) * 32767.0) as i32;
        let mask = BIT_MASKS[(p[BITS].round().max(0.0) as usize).min(BIT_MASKS.len() - 1)];
        let decimation = DECIMATION[(p[RATE].round().max(0.0) as usize).min(DECIMATION.len() - 1)];
        let rate_offset = self.rate_offset;
        let fir = self.fir;

        for v in self.voices.iter_mut() {
            if !v.active {
                continue;
            }
            let (pl, pr) = crate::dsp::pan_gains(v.pan * spread);
            let vel_gain = 1.0 - vel_amt + vel_amt * v.velocity;
            for i in 0..n {
                if v.pos >= OUT_BLOCK {
                    // Control rate: glide, envelopes, parameters, one Braids block.
                    v.note += (v.target - v.note) * glide_k;
                    match v.stage {
                        1 => {
                            v.env += attack_step;
                            if v.env >= 1.0 {
                                v.env = 1.0;
                                v.stage = 2;
                            }
                        }
                        2 => {
                            v.env += (sustain - v.env) * decay_k;
                            if (v.env - sustain).abs() < 1e-3 {
                                v.stage = 3;
                            }
                        }
                        3 => v.env = sustain,
                        4 => {
                            v.env -= v.env * release_k;
                            if v.env < 1e-4 {
                                v.env = 0.0;
                                v.stage = 0;
                                v.active = false;
                            }
                        }
                        _ => {
                            v.env = 0.0;
                            v.active = false;
                        }
                    }
                    match v.ad_stage {
                        1 => {
                            v.ad += ad_attack;
                            if v.ad >= 1.0 {
                                v.ad = 1.0;
                                v.ad_stage = 2;
                            }
                        }
                        2 => v.ad -= v.ad * ad_decay_k,
                        _ => {}
                    }
                    let timbre = ((p[TIMBRE] + p[TIMBRE_ENV] * v.ad).clamp(0.0, 1.0) * 32767.0) as i16;
                    let color = ((p[COLOR] + p[COLOR_ENV] * v.ad).clamp(0.0, 1.0) * 32767.0) as i16;
                    let semis = v.note + p[PITCH_ENV] * v.ad;
                    let mut pitch = (semis * 128.0) as i32 + rate_offset;
                    if drift > 0 {
                        pitch += v.jitter.render(drift) as i32 * 4;
                    }
                    let pitch = pitch.clamp(0, 16383) as i16;
                    if v.strike {
                        v.osc.strike();
                        v.strike = false;
                    }
                    v.osc.set_shape(shape);
                    v.osc.set_pitch(pitch);
                    v.osc.set_parameters(timbre, color);
                    let mut raw = [0i16; BLOCK_SIZE];
                    v.osc.render(&[0u8; BLOCK_SIZE], &mut raw);
                    // Sample-rate and bit reduction, like the module's RATE/BITS.
                    for s in raw.iter_mut() {
                        if decimation > 1 {
                            if v.hold_count % decimation == 0 {
                                v.held = *s;
                            }
                            v.hold_count = v.hold_count.wrapping_add(1);
                            *s = v.held;
                        }
                        *s = (*s as u16 & mask) as i16;
                    }
                    // 2:1 decimation through the half-band filter.
                    for j in 0..OUT_BLOCK {
                        for s in &raw[j * 2..j * 2 + 2] {
                            v.history[v.hist_pos] = *s as f32;
                            v.hist_pos = (v.hist_pos + 1) % FIR_TAPS;
                        }
                        let mut acc = 0.0;
                        for (t, c) in fir.iter().enumerate() {
                            acc += c * v.history[(v.hist_pos + t) % FIR_TAPS];
                        }
                        v.out[j] = acc;
                    }
                    v.pos = 0;
                }
                let s = v.out[v.pos] * v.env * master * vel_gain;
                v.pos += 1;
                l[i] += s * pl;
                r[i] += s * pr;
            }
        }
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(st: &mut BraidsSynthState, p: &[f32], secs: f32, sr: f32) -> (Vec<f32>, Vec<f32>) {
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

    #[test]
    fn every_preset_sounds_and_stops() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = BraidsSynthState::new(48000.0);
            st.note_on(&p, 60, 0.9);
            let (l, r) = render(&mut st, &p, 0.4, 48000.0);
            let level = rms(&l).max(rms(&r));
            assert!(level > 1e-3, "{name} is silent ({level})");
            assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 4.0), "{name} blew up");
            st.note_off(&p, 60);
            let _ = render(&mut st, &p, 6.0, 48000.0);
            assert_eq!(st.active_voices(), 0, "{name} keeps a voice");
        }
    }

    #[test]
    fn in_tune_at_48k_and_44k1() {
        for sr in [48000.0f32, 44100.0] {
            let mut p = preset("CSaw Lead");
            p[SHAPE] = 3.0; // sine/triangle fold with no fold: a sine
            p[TIMBRE] = 0.0;
            p[COLOR] = 0.0;
            p[TIMBRE_ENV] = 0.0;
            let mut st = BraidsSynthState::new(sr);
            st.note_on(&p, 69, 1.0);
            let (l, _) = render(&mut st, &p, 1.5, sr);
            let tail = &l[(0.5 * sr) as usize..];
            let ups = tail.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f32;
            assert!((ups - 440.0).abs() < 8.0, "A4 at {ups} Hz ({sr} Hz)");
        }
    }

    #[test]
    fn lofi_reduces_bits() {
        let mut p = preset("CSaw Lead");
        p[BITS] = 6.0; // 2 bits
        let mut st = BraidsSynthState::new(48000.0);
        st.note_on(&p, 60, 1.0);
        let (l, _) = render(&mut st, &p, 0.3, 48000.0);
        assert!(rms(&l) > 1e-3);
    }
}
