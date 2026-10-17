//! VL-Tone: the little Casio VL-1 of 1981, the calculator with a keyboard.
//! Ported from the VL1 emulator (<https://github.com/linuxmao-org/VL1-emulator>),
//! Copyright (C) 2006 PolyValens, Copyright (C) 2019 Jean Pierre Cimalando,
//! SPDX-License-Identifier: CC0-1.0 or MIT. Its ten sounds (narrow pulse tables
//! read sixteen times oversampled; the Electro ones jump an octave on the
//! rhythm's clock), its stepped envelope (fifteen levels, from Casio's own
//! tables), the RC network that shapes each pulse, vibrato and tremolo,
//! and its rhythm box: a blip, the same blip twice as fast, and stepped
//! noise, in March, Waltz, 4 Beat, Swing, Rock 1 and 2, Bossa Nova, Samba,
//! Rhumba and Beguine, on the song's tempo or its own.

use crate::params::{ParamSpec, Unit};

pub const SOUND: usize = 0;
pub const ATTACK: usize = 1;
pub const DECAY: usize = 2;
pub const SUSTAIN_LEVEL: usize = 3;
pub const SUSTAIN_TIME: usize = 4;
pub const RELEASE: usize = 5;
pub const VIBRATO: usize = 6;
pub const TREMOLO: usize = 7;
pub const OCTAVE: usize = 8;
pub const TUNE: usize = 9;
pub const VOICES: usize = 10;
pub const RHYTHM: usize = 11;
pub const RHYTHM_LEVEL: usize = 12;
pub const TEMPO: usize = 13;
pub const SYNC: usize = 14;
pub const VOLUME: usize = 15;

pub const SOUNDS: &[&str] = &["Piano", "Fantasy", "Violin", "Flute", "Guitar 1", "Guitar 2", "English Horn", "Electro 1", "Electro 2", "Electro 3"];
pub const RHYTHMS: &[&str] = &["Off", "March", "Waltz", "4 Beat", "Swing", "Rock 1", "Rock 2", "Bossa Nova", "Samba", "Rhumba", "Beguine"];
pub const OCTAVES: &[&str] = &["Low", "Middle", "High"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Sound", "Sound", SOUNDS, 0),
    ParamSpec::int("Attack", "ADSR", 0.0, 9.0, 0.0, Unit::None),
    ParamSpec::int("Decay", "ADSR", 0.0, 9.0, 4.0, Unit::None),
    ParamSpec::int("Sustain Level", "ADSR", 0.0, 9.0, 5.0, Unit::None),
    ParamSpec::int("Sustain Time", "ADSR", 0.0, 9.0, 3.0, Unit::None),
    ParamSpec::int("Release", "ADSR", 0.0, 9.0, 2.0, Unit::None),
    ParamSpec::int("Vibrato", "Sound", 0.0, 9.0, 0.0, Unit::None),
    ParamSpec::int("Tremolo", "Sound", 0.0, 9.0, 0.0, Unit::None),
    ParamSpec::choice("Octave", "Sound", OCTAVES, 1),
    ParamSpec::lin("Tune", "Sound", -100.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::int("Voices", "Sound", 1.0, 8.0, 1.0, Unit::None),
    ParamSpec::choice("Rhythm", "Rhythm", RHYTHMS, 0),
    ParamSpec::lin("Rhythm Level", "Rhythm", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::int("Tempo", "Rhythm", -9.0, 9.0, 0.0, Unit::None),
    ParamSpec::toggle("Song Tempo", "Rhythm", true),
    ParamSpec::lin("Volume", "Sound", 0.0, 1.0, 0.7, Unit::Percent),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Piano", "Fantasy", "Violin", "Flute", "Guitar", "English Horn", "Electro 1", "Electro 2", "Electro 3", "Da Da Da", "Bossa Calculator", "Rock Box", "Vibrato Organ", "Toy Strings"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    // The VL-1's own programs: sound, A, D, Sl, St, R, V, T (from its tables).
    let set: &[(usize, f32)] = match name {
        "Fantasy" => &[(SOUND, 1.0), (ATTACK, 0.0), (DECAY, 0.0), (SUSTAIN_LEVEL, 9.0), (SUSTAIN_TIME, 9.0), (RELEASE, 6.0), (VIBRATO, 3.0)],
        "Violin" => &[(SOUND, 2.0), (ATTACK, 3.0), (DECAY, 0.0), (SUSTAIN_LEVEL, 9.0), (SUSTAIN_TIME, 9.0), (RELEASE, 1.0), (VIBRATO, 3.0)],
        "Flute" => &[(SOUND, 3.0), (ATTACK, 3.0), (DECAY, 0.0), (SUSTAIN_LEVEL, 9.0), (SUSTAIN_TIME, 9.0), (RELEASE, 1.0), (VIBRATO, 3.0)],
        "Guitar" => &[(SOUND, 4.0), (ATTACK, 1.0), (DECAY, 0.0), (SUSTAIN_LEVEL, 7.0), (SUSTAIN_TIME, 1.0), (RELEASE, 2.0)],
        "English Horn" => &[(SOUND, 6.0), (ATTACK, 2.0), (SUSTAIN_LEVEL, 9.0), (SUSTAIN_TIME, 9.0), (RELEASE, 2.0), (VIBRATO, 2.0)],
        "Electro 1" => &[(SOUND, 7.0), (SUSTAIN_LEVEL, 9.0), (SUSTAIN_TIME, 9.0), (RELEASE, 2.0)],
        "Electro 2" => &[(SOUND, 8.0), (SUSTAIN_LEVEL, 9.0), (SUSTAIN_TIME, 9.0), (RELEASE, 2.0)],
        "Electro 3" => &[(SOUND, 9.0), (SUSTAIN_LEVEL, 9.0), (SUSTAIN_TIME, 9.0), (RELEASE, 2.0)],
        // Trio's: a short piano over the March.
        "Da Da Da" => &[(SOUND, 0.0), (DECAY, 3.0), (SUSTAIN_LEVEL, 2.0), (SUSTAIN_TIME, 2.0), (RELEASE, 1.0), (RHYTHM, 1.0), (RHYTHM_LEVEL, 0.8)],
        "Bossa Calculator" => &[(SOUND, 3.0), (ATTACK, 1.0), (SUSTAIN_LEVEL, 7.0), (SUSTAIN_TIME, 6.0), (RELEASE, 3.0), (VIBRATO, 2.0), (RHYTHM, 7.0), (RHYTHM_LEVEL, 0.6)],
        "Rock Box" => &[(SOUND, 7.0), (SUSTAIN_LEVEL, 8.0), (SUSTAIN_TIME, 9.0), (RELEASE, 1.0), (RHYTHM, 5.0), (RHYTHM_LEVEL, 0.7)],
        "Vibrato Organ" => &[(SOUND, 1.0), (ATTACK, 1.0), (SUSTAIN_LEVEL, 9.0), (SUSTAIN_TIME, 9.0), (RELEASE, 3.0), (VIBRATO, 5.0), (TREMOLO, 4.0), (VOICES, 4.0)],
        "Toy Strings" => &[(SOUND, 2.0), (ATTACK, 5.0), (SUSTAIN_LEVEL, 9.0), (SUSTAIN_TIME, 9.0), (RELEASE, 5.0), (VIBRATO, 4.0), (VOICES, 6.0), (OCTAVE, 0.0)],
        // "Piano": the VL-1's first program.
        _ => &[(SOUND, 0.0), (ATTACK, 0.0), (DECAY, 4.0), (SUSTAIN_LEVEL, 5.0), (SUSTAIN_TIME, 3.0), (RELEASE, 2.0)],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

// ------------------------------------------------------------ the sounds

const TABLE: usize = 32;
const OVERSAMPLE: usize = 16;
const WAVE_LEN: usize = TABLE * OVERSAMPLE;

/// The tables, as the emulator makes them (pulses, 0 and 1), and how much
/// higher or lower than the key each one plays.
fn make_wave(sound: usize) -> ([f32; WAVE_LEN], f32) {
    let mut w = [0.0f32; WAVE_LEN];
    let n = WAVE_LEN;
    let fill = |w: &mut [f32; WAVE_LEN], highs: &[(usize, usize)]| {
        for &(a, b) in highs {
            for x in w.iter_mut().take(b * n / 16).skip(a * n / 16) {
                *x = 1.0;
            }
        }
    };
    // Electro 1-3 are Piano, Fantasy and Violin, jumping an octave.
    let base = match sound {
        7 => 0,
        8 => 1,
        9 => 2,
        s => s,
    };
    let scale = match base {
        0 => {
            fill(&mut w, &[(0, 11)]);
            1.0
        }
        1 => {
            fill(&mut w, &[(0, 8)]);
            2.0
        }
        2 => {
            fill(&mut w, &[(0, 4), (5, 8), (9, 11), (12, 13), (14, 15)]);
            1.0
        }
        3 => {
            fill(&mut w, &[(0, 8)]);
            1.0
        }
        4 => {
            fill(&mut w, &[(0, 1), (7, 8)]);
            0.5
        }
        5 => {
            fill(&mut w, &[(0, 1), (2, 6), (8, 10)]);
            0.5
        }
        _ => {
            // English horn: a pulse of a seventh.
            for x in w.iter_mut().take(n / 7) {
                *x = 1.0;
            }
            0.5
        }
    };
    (w, scale)
}

// ------------------------------------------------------------ the envelope

/// Casio's tables: level from, level to, time units (12 ms each).
const ATTACK_T: [(f32, f32, f32); 10] = [(15.0, 15.0, 0.0), (11.0, 15.0, 1.0), (7.0, 15.0, 4.0), (5.0, 15.0, 7.0), (4.0, 15.0, 10.0), (4.0, 15.0, 13.0), (3.0, 15.0, 16.0), (3.0, 15.0, 19.0), (3.0, 15.0, 22.0), (3.0, 15.0, 25.0)];
const DECAY_T: [f32; 10] = [0.0, 9.0, 18.0, 27.0, 39.0, 48.0, 58.0, 67.0, 78.0, 87.0];
const SUSTAIN_LEVEL_T: [f32; 10] = [1.0, 2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 13.0, 14.0, 15.0];
/// How long the sustain takes to die away (-1: never).
const SUSTAIN_SLOPE_T: [f32; 10] = [0.0, 79.0, 154.0, 231.0, 312.0, 389.0, 467.0, 543.0, 615.0, -1.0];
const RELEASE_T: [f32; 10] = [0.0, 26.0, 48.0, 73.0, 96.0, 116.0, 142.0, 167.0, 193.0, 217.0];
const UNIT: f32 = 0.012;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Phase {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// The stepped envelope: a line that moves smoothly, and the level that
/// follows it only every 12 ms, a whole step (of fifteen) at a time.
#[derive(Clone, Copy)]
struct Adsr {
    phase: Phase,
    y: f32,
    acc: f32,
    target: f32,
    rate: f32,
    step_left: f32,
    release_after: bool,
}

impl Adsr {
    fn new() -> Self {
        Self { phase: Phase::Idle, y: 0.0, acc: 0.0, target: 0.0, rate: 0.0, step_left: 0.0, release_after: false }
    }

    /// A line from where it is (or `from`) to `to` over `units` (12 ms each).
    fn line(&mut self, from: Option<f32>, to: f32, span: f32, units: f32, clock: f32) {
        if let Some(f) = from {
            self.acc = f;
        }
        self.y = self.acc;
        self.target = to;
        let secs = if units < 0.0 { 1.0e7 } else { (units * UNIT).max(0.0005) };
        self.rate = span.abs() / (secs * clock);
        self.step_left = 0.0;
    }

    fn gate(&mut self, on: bool, p: &[usize; 5], clock: f32) {
        if on {
            self.phase = Phase::Attack;
            self.release_after = false;
            let (a, b, t) = ATTACK_T[p[0].min(9)];
            let t = if t <= 0.0 { 1.5 } else { t };
            self.line(Some(a), b, b - a, t, clock);
        } else if self.phase == Phase::Attack {
            // The attack always finishes.
            self.release_after = true;
        } else if self.phase != Phase::Idle {
            self.release(p, clock);
        }
    }

    fn release(&mut self, p: &[usize; 5], clock: f32) {
        self.phase = Phase::Release;
        self.line(None, 0.0, 14.0, RELEASE_T[p[4].min(9)], clock);
    }

    /// One tick of `clock` Hz; the level (0 to 1).
    fn tick(&mut self, p: &[usize; 5], clock: f32) -> f32 {
        if self.phase == Phase::Idle {
            return 0.0;
        }
        let up = self.target > self.y;
        if up {
            self.y = (self.y + self.rate).min(self.target);
        } else {
            self.y = (self.y - self.rate).max(self.target);
        }
        // The level catches up a whole step every 12 ms.
        self.step_left -= 1.0;
        if self.step_left <= 0.0 {
            self.step_left = UNIT * clock;
            if up {
                while self.acc < self.y {
                    self.acc += 1.0;
                }
            } else {
                while self.acc > self.y {
                    self.acc -= 1.0;
                }
            }
        }
        let done = (self.y - self.target).abs() < 1e-6;
        if done {
            self.acc = self.target;
        }
        match self.phase {
            Phase::Attack if done => {
                if self.release_after {
                    self.release(p, clock);
                } else if p[1] > 0 {
                    self.phase = Phase::Decay;
                    self.line(None, 0.0, 15.0, DECAY_T[p[1].min(9)], clock);
                } else {
                    self.phase = Phase::Sustain;
                    self.acc = SUSTAIN_LEVEL_T[p[2].min(9)];
                    self.line(None, 0.0, 15.0, SUSTAIN_SLOPE_T[p[3].min(9)], clock);
                }
            }
            Phase::Decay if self.acc <= SUSTAIN_LEVEL_T[p[2].min(9)] => {
                self.phase = Phase::Sustain;
                self.line(None, 0.0, 15.0, SUSTAIN_SLOPE_T[p[3].min(9)], clock);
            }
            _ => {}
        }
        let out = (self.acc / 15.0).clamp(0.0, 1.0);
        if out <= 1e-6 && matches!(self.phase, Phase::Release | Phase::Sustain | Phase::Decay) {
            self.phase = Phase::Idle;
            return 0.0;
        }
        out
    }
}

// ------------------------------------------------------------ the voice

#[derive(Clone, Copy)]
struct Voice {
    note: u8,
    freq: f32,
    vel: f32,
    phase: f32,
    adsr: Adsr,
    /// The RC network's capacitor (volts).
    cap: f32,
    /// The anti-alias filter's memory.
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    vib: f32,
    trem: f32,
    age: u64,
}

impl Voice {
    fn new() -> Self {
        Self { note: 0, freq: 0.0, vel: 0.0, phase: 0.0, adsr: Adsr::new(), cap: 0.0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0, vib: 0.0, trem: 0.0, age: 0 }
    }
}

// ------------------------------------------------------------ the rhythm

#[derive(Clone, Copy, PartialEq)]
enum Hit {
    Pock,
    Peek,
    Noise,
}

/// The rhythms: (sound, length in 32nds of a beat).
const PATTERNS: [&[(Hit, u8)]; 10] = [
    &[(Hit::Pock, 32), (Hit::Noise, 32)],
    &[(Hit::Pock, 32), (Hit::Noise, 32), (Hit::Noise, 32)],
    &[(Hit::Pock, 32), (Hit::Noise, 32), (Hit::Noise, 32), (Hit::Noise, 32)],
    &[(Hit::Peek, 32), (Hit::Noise, 24), (Hit::Peek, 8)],
    &[(Hit::Pock, 16), (Hit::Peek, 16), (Hit::Noise, 16), (Hit::Noise, 16), (Hit::Pock, 16), (Hit::Peek, 16), (Hit::Noise, 16), (Hit::Peek, 16)],
    &[(Hit::Pock, 16), (Hit::Peek, 16), (Hit::Noise, 16), (Hit::Peek, 8), (Hit::Pock, 8), (Hit::Pock, 16), (Hit::Pock, 16), (Hit::Noise, 16), (Hit::Peek, 16)],
    &[(Hit::Peek, 16), (Hit::Noise, 16), (Hit::Noise, 16), (Hit::Peek, 16), (Hit::Pock, 16), (Hit::Noise, 16), (Hit::Peek, 16), (Hit::Noise, 16), (Hit::Pock, 16), (Hit::Noise, 16), (Hit::Peek, 16), (Hit::Pock, 16), (Hit::Pock, 16), (Hit::Peek, 16), (Hit::Noise, 16), (Hit::Pock, 16)],
    &[(Hit::Peek, 16), (Hit::Noise, 16), (Hit::Peek, 16), (Hit::Noise, 16), (Hit::Pock, 16), (Hit::Pock, 16), (Hit::Noise, 16), (Hit::Peek, 16), (Hit::Noise, 16), (Hit::Peek, 16), (Hit::Noise, 16), (Hit::Peek, 16), (Hit::Pock, 16), (Hit::Pock, 16), (Hit::Pock, 16), (Hit::Noise, 16)],
    &[(Hit::Peek, 16), (Hit::Noise, 8), (Hit::Noise, 8), (Hit::Noise, 16), (Hit::Peek, 16), (Hit::Noise, 16), (Hit::Noise, 16), (Hit::Peek, 16), (Hit::Noise, 16), (Hit::Noise, 16), (Hit::Noise, 8), (Hit::Noise, 8), (Hit::Peek, 16), (Hit::Noise, 16), (Hit::Peek, 16), (Hit::Noise, 16), (Hit::Pock, 16), (Hit::Pock, 16)],
    &[(Hit::Pock, 16), (Hit::Noise, 8), (Hit::Noise, 8), (Hit::Noise, 16), (Hit::Noise, 16), (Hit::Peek, 16), (Hit::Noise, 16), (Hit::Pock, 16), (Hit::Noise, 16)],
];

/// The 16-bit noise register (taps 16, 15, 13, 4).
fn lfsr(r: &mut u16) -> f32 {
    let bit = ((*r >> 15) ^ (*r >> 14) ^ (*r >> 12) ^ (*r >> 3)) & 1;
    *r = (*r << 1) | (bit ^ 1);
    if *r & 0x8000 != 0 {
        1.0
    } else {
        0.0
    }
}

pub struct Vl1State {
    sr: f32,
    voices: [Voice; 8],
    waves: Vec<([f32; WAVE_LEN], f32)>,
    clock: u64,
    held: Vec<u8>,
    // The ASIC's clock: a tick a millisecond or so (with the tempo).
    tick_acc: f32,
    ticks: u64,
    /// Electro: the octave it is on (1 or 0.5).
    jump: f32,
    // The rhythm.
    blip: Vec<f32>,
    noise: Vec<f32>,
    hit: Option<Hit>,
    hit_t: usize,
    step: usize,
    left: u32,
    beat_seen: i64,
    dc: f32,
    pub playing: bool,
    pub beat: f64,
    pub bpm: f32,
    pub pitch_bend: f32,
}

impl Vl1State {
    pub fn new(sr: f32) -> Self {
        let waves = (0..SOUNDS.len()).map(make_wave).collect();
        // The blip: a 780 Hz square, its level a step down every two periods.
        let half = ((sr / 780.0) as usize).div_ceil(2).max(1);
        let mut blip = Vec::with_capacity(30 * 2 * half);
        let mut level = 1.0f32;
        for _ in 0..15 {
            for _ in 0..2 {
                blip.extend(std::iter::repeat_n(level, half));
                blip.extend(std::iter::repeat_n(0.0, half));
            }
            level -= 1.0 / 15.0;
        }
        // The noise: eight steps of 5 ms, then six of 20, clocked at 50 kHz.
        let mut noise = Vec::new();
        let mut r: u16 = 0;
        let mut level = 1.0f32;
        let per = |ms: f32| (sr * ms / 1000.0) as usize;
        let ratio = 50000.0 / sr;
        let mut acc = 0.0;
        let mut cur = 0.0;
        for (n, ms) in [(8, 5.0), (6, 20.0)] {
            for _ in 0..n {
                for _ in 0..per(ms) {
                    acc += ratio;
                    while acc >= 1.0 {
                        acc -= 1.0;
                        cur = lfsr(&mut r);
                    }
                    noise.push(level * cur);
                }
                level -= 1.0 / 15.0;
            }
        }
        Self {
            sr,
            voices: [Voice::new(); 8],
            waves,
            clock: 0,
            held: Vec::with_capacity(16),
            tick_acc: 0.0,
            ticks: 0,
            jump: 1.0,
            blip,
            noise,
            hit: None,
            hit_t: 0,
            step: 0,
            left: 0,
            beat_seen: i64::MIN,
            dc: 0.0,
            playing: false,
            beat: 0.0,
            bpm: 120.0,
            pitch_bend: 0.0,
        }
    }

    fn adsr_params(p: &[f32]) -> [usize; 5] {
        let g = |i: usize| p.get(i).map(|v| v.round().clamp(0.0, 9.0) as usize).unwrap_or(0);
        [g(ATTACK), g(DECAY), g(SUSTAIN_LEVEL), g(SUSTAIN_TIME), g(RELEASE)]
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        self.held.retain(|&n| n != note);
        self.held.push(note);
        let voices = p.get(VOICES).map(|v| v.round().clamp(1.0, 8.0) as usize).unwrap_or(1);
        self.clock += 1;
        let clock = self.sr * OVERSAMPLE as f32;
        let ap = Self::adsr_params(p);
        // Mono: the one voice takes the newest key (its phase goes on).
        let v = if voices == 1 {
            0
        } else {
            (0..voices).find(|&i| self.voices[i].adsr.phase == Phase::Idle).unwrap_or_else(|| (0..voices).min_by_key(|&i| self.voices[i].age).unwrap_or(0))
        };
        let voice = &mut self.voices[v];
        if voice.adsr.phase == Phase::Idle {
            voice.phase = 0.0;
            voice.vib = 0.0;
            voice.trem = 0.0;
        }
        voice.note = note;
        voice.freq = 440.0 * 2f32.powf((note as f32 - 69.0) / 12.0);
        voice.vel = vel;
        voice.age = self.clock;
        voice.adsr.gate(true, &ap, clock);
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.held.retain(|&n| n != note);
        let voices = p.get(VOICES).map(|v| v.round().clamp(1.0, 8.0) as usize).unwrap_or(1);
        let clock = self.sr * OVERSAMPLE as f32;
        let ap = Self::adsr_params(p);
        if voices == 1 {
            // Back to a key still held, else let go.
            if self.voices[0].note == note {
                match self.held.last() {
                    Some(&n) => {
                        self.voices[0].note = n;
                        self.voices[0].freq = 440.0 * 2f32.powf((n as f32 - 69.0) / 12.0);
                    }
                    None => self.voices[0].adsr.gate(false, &ap, clock),
                }
            }
        } else {
            for v in self.voices.iter_mut().take(voices) {
                if v.note == note && v.adsr.phase != Phase::Idle && v.adsr.phase != Phase::Release {
                    v.adsr.gate(false, &ap, clock);
                }
            }
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in self.voices.iter_mut() {
            v.adsr.phase = Phase::Idle;
        }
        self.hit = None;
    }

    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        let v = &self.voices[0];
        out[0] = v.note as f32;
        out[1] = self.voices.iter().map(|v| v.adsr.acc / 15.0).fold(0.0, f32::max);
        out[2] = self.step as f32;
        out[3] = match self.hit {
            Some(Hit::Pock) => 1.0,
            Some(Hit::Peek) => 2.0,
            Some(Hit::Noise) => 3.0,
            None => 0.0,
        } * if self.hit_t < 2400 { 1.0 } else { 0.0 };
        out[4] = if self.voices.iter().any(|v| v.adsr.phase != Phase::Idle) { 1.0 } else { 0.0 };
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        let get = |i: usize, or: f32| p.get(i).copied().unwrap_or(or);
        let sound = (get(SOUND, 0.0).round().max(0.0) as usize).min(SOUNDS.len() - 1);
        let electro = sound >= 7;
        let ap = Self::adsr_params(p);
        let vib = get(VIBRATO, 0.0).round().clamp(0.0, 9.0);
        let trem = get(TREMOLO, 0.0).round().clamp(0.0, 9.0);
        let octave = [0.5f32, 1.0, 2.0][(get(OCTAVE, 1.0).round().clamp(0.0, 2.0)) as usize];
        let tune = 2f32.powf(get(TUNE, 0.0) / 1200.0 + self.pitch_bend / 12.0);
        let voices = get(VOICES, 1.0).round().clamp(1.0, 8.0) as usize;
        let rhythm = (get(RHYTHM, 0.0).round().max(0.0) as usize).min(RHYTHMS.len() - 1);
        let rhythm_level = get(RHYTHM_LEVEL, 0.7).clamp(0.0, 1.0);
        let tempo = get(TEMPO, 0.0).round().clamp(-9.0, 9.0);
        let sync = get(SYNC, 1.0) >= 0.5;
        let volume = get(VOLUME, 0.7).clamp(0.0, 1.0);
        let sr = self.sr;
        let clock = sr * OVERSAMPLE as f32;
        let (wave, scale) = self.waves[sound];
        // The ASIC's tick: so many ms with the tempo, or a 32nd of the song's beat.
        let tick_secs = if sync { 60.0 / self.bpm.max(20.0) / 32.0 } else { (5.5 + (9.0 - tempo) * 0.98) * 0.001 };
        // The anti-alias filter (as the emulator sets it, at the oversampled rate).
        let w0 = std::f32::consts::TAU * 0.95 / OVERSAMPLE as f32;
        let (cw, sw) = (w0.cos(), w0.sin());
        let alpha = sw / 2.0 * 4.0;
        let a0 = 1.0 + alpha;
        let (b1, a1, a2) = ((1.0 - cw) / a0, -2.0 * cw / a0, (1.0 - alpha) / a0);
        let b0 = 0.5 * b1;
        let vib_hz = if vib > 0.0 { 15.0 / vib } else { 0.0 };
        let vib_amt = if vib > 0.0 { 0.1123 / vib_hz } else { 0.0 };
        let trem_hz = if trem > 0.0 { 35.0 / trem } else { 0.0 };
        let dc_k = 1.0 - (-std::f32::consts::TAU * 12.0 / sr).exp();
        let beats_per_sample = self.bpm as f64 / 60.0 / sr as f64;
        let pattern = if rhythm > 0 { Some(PATTERNS[rhythm - 1]) } else { None };
        for i in 0..n {
            // The tick: Electro jumps an octave every eight.
            self.tick_acc += 1.0 / sr;
            while self.tick_acc >= tick_secs {
                self.tick_acc -= tick_secs;
                self.ticks += 1;
                if self.ticks % 8 == 0 {
                    self.jump = if electro && self.jump > 0.75 { 0.5 } else { 1.0 };
                }
                // The free-running rhythm counts its own ticks.
                if !sync && pattern.is_some() && self.playing {
                    if self.left == 0 {
                        let pat = pattern.unwrap_or(&[]);
                        self.step %= pat.len().max(1);
                        let (hit, len) = pat[self.step];
                        self.hit = Some(hit);
                        self.hit_t = 0;
                        self.left = len as u32;
                        self.step += 1;
                    }
                    self.left = self.left.saturating_sub(1);
                }
            }
            if !electro {
                self.jump = 1.0;
            }
            // The rhythm on the song's beat.
            if sync && self.playing {
                if let Some(pat) = pattern {
                    let total: u32 = pat.iter().map(|x| x.1 as u32).sum();
                    let pos32 = (((self.beat + i as f64 * beats_per_sample) * 32.0).floor() as i64).rem_euclid(total as i64 * 1_000_000);
                    if pos32 != self.beat_seen {
                        self.beat_seen = pos32;
                        let at = (pos32 % total as i64) as u32;
                        let mut acc = 0u32;
                        for (k, &(hit, len)) in pat.iter().enumerate() {
                            if acc == at {
                                self.hit = Some(hit);
                                self.hit_t = 0;
                                self.step = k + 1;
                            }
                            acc += len as u32;
                        }
                    }
                }
            }
            let rh = match self.hit {
                Some(Hit::Pock) => self.blip.get(self.hit_t).copied().unwrap_or(0.0),
                Some(Hit::Peek) => self.blip.get(2 * self.hit_t).copied().unwrap_or(0.0),
                Some(Hit::Noise) => self.noise.get(self.hit_t).copied().unwrap_or(0.0),
                None => 0.0,
            };
            if self.hit.is_some() {
                self.hit_t += 1;
            }
            // The voices, sixteen times over.
            let mut sum = 0.0f32;
            for v in self.voices.iter_mut().take(voices) {
                if v.adsr.phase == Phase::Idle {
                    continue;
                }
                let inc_base = scale * tune * octave * v.freq * WAVE_LEN as f32 / clock;
                let mut acc = 0.0f32;
                for _ in 0..OVERSAMPLE {
                    let i0 = v.phase as usize % WAVE_LEN;
                    let i1 = (i0 + 1) % WAVE_LEN;
                    let f = v.phase - v.phase.floor();
                    let mut s = (1.0 - f) * wave[i0] + f * wave[i1];
                    if trem_hz > 0.0 {
                        v.trem += trem_hz / clock;
                        s *= 0.5 + 0.5 * (std::f32::consts::TAU * v.trem).cos();
                    }
                    s *= v.adsr.tick(&ap, clock);
                    // The RC network: charged through 600 ohm while the pulse is high, let go through 33k.
                    let vin = s * 5.0 - 0.6;
                    let i_c = if vin > v.cap { (vin - v.cap) / 600.0 } else { -v.cap / 33000.0 };
                    v.cap += i_c / (clock * 3.3e-6);
                    let shaped = (0.131_033_18 * v.cap * v.cap + 0.246_632_17 * v.cap - 0.219_399_43).max(0.0);
                    let s = if s > 1e-15 { shaped } else { 0.0 };
                    let y = b0 * s + b1 * v.x1 + b0 * v.x2 - a1 * v.y1 - a2 * v.y2;
                    v.x2 = v.x1;
                    v.x1 = s;
                    v.y2 = v.y1;
                    v.y1 = if y.abs() < 1e-20 { 0.0 } else { y };
                    acc += y;
                    let mut inc = inc_base;
                    if vib_hz > 0.0 {
                        v.vib += vib_hz / clock;
                        inc *= 1.0 + vib_amt * (std::f32::consts::TAU * v.vib).sin();
                    }
                    v.phase += self.jump * inc;
                    if v.phase >= WAVE_LEN as f32 {
                        v.phase -= WAVE_LEN as f32;
                    }
                }
                sum += acc * v.vel / OVERSAMPLE as f32;
            }
            // (the shaper peaks near 3.4 volts)
            let out = sum * 0.08 + rh * rhythm_level * 0.12;
            // Its pulses stand on zero: the middle taken out.
            self.dc += dc_k * (out - self.dc);
            let y = (out - self.dc) * volume;
            l[i] = y;
            r[i] = y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(p: &[f32], note: u8, secs: f32) -> Vec<f32> {
        let mut s = Vl1State::new(48000.0);
        s.playing = true;
        s.note_on(p, note, 0.9);
        let n = (secs * 48000.0) as usize;
        let mut out = Vec::with_capacity(n);
        for k in 0..n / 256 {
            let mut l = vec![0.0; 256];
            let mut r = vec![0.0; 256];
            s.beat = k as f64 * 256.0 / 48000.0 * 2.0;
            s.render(p, &mut l, &mut r);
            out.extend(l);
        }
        out
    }

    fn rms(v: &[f32]) -> f32 {
        (v.iter().map(|x| x * x).sum::<f32>() / v.len().max(1) as f32).sqrt()
    }

    /// The main period (in samples) by autocorrelation.
    fn period(v: &[f32]) -> f32 {
        let (mut best, mut at) = (f32::MIN, 0);
        for lag in 20..1200 {
            let c: f32 = v.iter().zip(&v[lag..]).map(|(a, b)| a * b).sum();
            if c > best {
                best = c;
                at = lag;
            }
        }
        at as f32
    }

    #[test]
    fn every_sound_plays_at_its_pitch() {
        for (k, name) in SOUNDS.iter().enumerate().take(7) {
            let mut p = preset("Flute");
            p[SOUND] = k as f32;
            let out = play(&p, 69, 0.4);
            assert!(rms(&out[4800..]) > 0.01, "{name}");
            let (_, scale) = make_wave(k);
            let want = 48000.0 / (440.0 * scale);
            let got = period(&out[9600..19200]);
            // (the autocorrelation may find a multiple of a period)
            let ratio = got / want;
            assert!((ratio - ratio.round()).abs() < 0.03 && ratio.round() >= 1.0, "{name}: {got} {want}");
        }
    }

    #[test]
    fn the_envelope_steps_and_lets_go() {
        let p = preset("Piano");
        let mut s = Vl1State::new(48000.0);
        s.note_on(&p, 60, 1.0);
        let mut levels = Vec::new();
        for _ in 0..200 {
            let mut l = vec![0.0; 256];
            let mut r = vec![0.0; 256];
            s.render(&p, &mut l, &mut r);
            levels.push(s.voices[0].adsr.acc);
        }
        // Whole steps of fifteen only.
        assert!(levels.iter().all(|v| (v - v.round()).abs() < 1e-3), "{levels:?}");
        // The piano falls away to its sustain and on down.
        assert!(levels[0] >= 14.0 && levels[199] < levels[20]);
        s.note_off(&p, 60);
        for _ in 0..400 {
            let mut l = vec![0.0; 256];
            let mut r = vec![0.0; 256];
            s.render(&p, &mut l, &mut r);
        }
        assert_eq!(s.voices[0].adsr.phase, Phase::Idle);
    }

    #[test]
    fn electro_jumps_an_octave() {
        let mut p = preset("Electro 1");
        p[SYNC] = 0.0;
        let out = play(&p, 57, 1.0);
        // The period changes back and forth (twice as long at times).
        let chunks: Vec<f32> = out.chunks(4800).skip(1).map(period).collect();
        let lo = chunks.iter().copied().fold(f32::MAX, f32::min);
        let hi = chunks.iter().copied().fold(0.0, f32::max);
        assert!(hi > 1.6 * lo, "{chunks:?}");
    }

    #[test]
    fn the_rhythm_plays_on_the_song() {
        let mut p = preset("Da Da Da");
        p[VOLUME] = 1.0;
        let mut s = Vl1State::new(48000.0);
        s.playing = true;
        s.bpm = 120.0;
        let mut hits = 0;
        let mut was = None;
        for k in 0..(4.0 * 48000.0 / 256.0) as usize {
            let mut l = vec![0.0; 256];
            let mut r = vec![0.0; 256];
            s.beat = k as f64 * 256.0 / 48000.0 * 2.0;
            s.render(&p, &mut l, &mut r);
            if s.hit_t < 256 && s.hit != was {
                hits += 1;
            }
            was = s.hit;
            let _ = l;
        }
        // March: a hit a beat, eight beats in four seconds at 120.
        assert!(hits >= 2, "{hits}");
        assert!(s.step > 0);
    }

    #[test]
    fn stays_finite_and_quiet_enough() {
        for name in preset_names() {
            let p = preset(name);
            let out = play(&p, 48, 1.0);
            assert!(out.iter().all(|x| x.is_finite() && x.abs() < 1.5), "{name}");
        }
    }
}
