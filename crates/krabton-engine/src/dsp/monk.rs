//! Monk: a chanting voice made of FOF grains (formant wave functions, after
//! Xavier Rodet at IRCAM). A Rust port of the DSP of MonkSynth by Jonathan
//! Taylor (MIT License, Copyright (c) 2026 Jonathan Taylor,
//! <https://github.com/JonET/monksynth>), which recreates the freeware monk
//! plug-in Delay Lama (AudioNerdz, 2002).
//!
//! Every pitch period a grain starts: three damped sines at the formants of
//! the vowel (a spline through five vowel positions) plus breath (two
//! inharmonic damped sines), in a short window, overlap-added. The pitch is
//! quantized to 1/32 of a semitone, part of the character. Around the voice:
//! glide, vibrato whose rate wanders, an ADSR, up to ten detuned singers and
//! a stereo echo.
//!
//! Krabton changes: polyphony (Voices above 1 gives every note its own choir;
//! with 1 it is the original's legato monk with its note stack), Octave (the
//! original sings an octave below the key: Octave -1), Wobble (a vowel LFO),
//! Velocity, and lookup tables shared by all singers.
//!
//! MonkSynth's license:
//!
//! MIT License
//!
//! Copyright (c) 2026 Jonathan Taylor
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.

use super::db_to_gain;
use crate::params::{ParamSpec, Unit};
use std::f32::consts::{PI, TAU};

pub const VOWEL: usize = 0;
pub const HEAD: usize = 1;
pub const GLIDE: usize = 2;
pub const OCTAVE: usize = 3;
pub const BREATH: usize = 4;
pub const VIBRATO: usize = 5;
pub const VIB_RATE: usize = 6;
pub const ATTACK: usize = 7;
pub const DECAY: usize = 8;
pub const SUSTAIN: usize = 9;
pub const RELEASE: usize = 10;
pub const UNISON: usize = 11;
pub const DETUNE: usize = 12;
pub const HEAD_SPREAD: usize = 13;
pub const VOICES: usize = 14;
pub const ECHO: usize = 15;
pub const ECHO_TIME: usize = 16;
pub const WOBBLE: usize = 17;
pub const WOBBLE_RATE: usize = 18;
pub const VELOCITY: usize = 19;
pub const VOLUME: usize = 20;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Vowel", "Monk", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Head", "Monk", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Glide", "Monk", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Octave", "Monk", -2.0, 1.0, -1.0, Unit::None),
    ParamSpec::lin("Breath", "Monk", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Vibrato", "Vibrato", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vib Rate", "Vibrato", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Attack", "Envelope", 0.0, 5.0, 0.0, Unit::Sec),
    ParamSpec::lin("Decay", "Envelope", 0.0, 5.0, 0.0, Unit::Sec),
    ParamSpec::lin("Sustain", "Envelope", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Release", "Envelope", 0.0, 5.0, 0.0, Unit::Sec),
    ParamSpec::int("Unison", "Choir", 1.0, 10.0, 1.0, Unit::None),
    ParamSpec::lin("Detune", "Choir", 0.0, 50.0, 0.0, Unit::Cents),
    ParamSpec::lin("Head Spread", "Choir", 0.0, 0.5, 0.0, Unit::Percent),
    ParamSpec::int("Voices", "Choir", 1.0, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Echo", "Echo", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Echo Time", "Echo", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Wobble", "Motion", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Wobble Rate", "Motion", 0.05, 8.0, 0.5, Unit::Hz),
    ParamSpec::lin("Velocity", "Output", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Volume", "Output", -24.0, 12.0, 0.0, Unit::Db),
];

/// The first six are MonkSynth's factory presets (its monks' names).
pub fn preset_names() -> &'static [&'static str] {
    &["Rabten", "Dorje", "Jamyang", "Monastery", "Ngawang", "Tinley", "Choir of Monks", "Om", "Wah Monk", "Bass Monk", "High Monk"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    match name {
        "Dorje" => set(&[(GLIDE, 0.4), (ECHO, 0.3), (HEAD, 0.0)]),
        "Jamyang" => set(&[(ECHO, 0.0), (HEAD, 0.75)]),
        "Monastery" => set(&[
            (GLIDE, 0.4),
            (VOWEL, 0.445),
            (ECHO, 0.3),
            (HEAD, 0.0),
            (VIBRATO, 0.094),
            (BREATH, 0.034),
            (ATTACK, 0.55),
            (RELEASE, 0.205),
            (UNISON, 3.0),
            (DETUNE, 23.3),
            (ECHO_TIME, 0.336),
            (HEAD_SPREAD, 0.089),
        ]),
        "Ngawang" => set(&[(GLIDE, 0.8), (ECHO, 0.6), (HEAD, 0.25)]),
        "Tinley" => set(&[(GLIDE, 1.0), (ECHO, 0.9), (HEAD, 1.0)]),
        "Choir of Monks" => set(&[(VOICES, 4.0), (UNISON, 4.0), (DETUNE, 18.0), (HEAD_SPREAD, 0.2), (ECHO, 0.5), (ATTACK, 0.3), (RELEASE, 0.8), (GLIDE, 0.2), (VOLUME, -4.0)]),
        "Om" => set(&[(VOWEL, 0.85), (WOBBLE, 0.4), (WOBBLE_RATE, 0.1), (UNISON, 2.0), (DETUNE, 8.0), (ATTACK, 0.6), (RELEASE, 1.2), (ECHO, 0.4)]),
        "Wah Monk" => set(&[(WOBBLE, 0.9), (WOBBLE_RATE, 2.5), (GLIDE, 0.3), (ECHO, 0.5)]),
        "Bass Monk" => set(&[(OCTAVE, -2.0), (HEAD, 0.0), (ECHO, 0.3), (UNISON, 2.0), (DETUNE, 6.0)]),
        "High Monk" => set(&[(OCTAVE, 0.0), (HEAD, 1.0), (VIBRATO, 0.3), (ECHO, 0.6)]),
        _ => {}
    }
    p
}

/// The three formants (Hz) at a vowel position (0..1) for a head (0..1),
/// as the grains use them: for displays.
pub fn formants(vowel: f32, head: f32) -> [f32; 3] {
    let pos = vowel.clamp(0.0, 1.0) * 4.0;
    let seg = (pos as usize).min(3);
    let t = pos - seg as f32;
    let (t2, t3) = (t * t, t * t * t);
    std::array::from_fn(|f| {
        let pts = FORMANT_FREQS[f];
        let padded = [pts[0], pts[0], pts[1], pts[2], pts[3], pts[4], pts[4]];
        let (p0, p1, p2, p3) = (padded[seg], padded[seg + 1], padded[seg + 2], padded[seg + 3]);
        0.5 * ((2.0 * p1) + (-p0 + p2) * t + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3) * (head.clamp(0.0, 1.0) * 0.5 + 0.75)
    })
}

const OVERLAP: usize = 10240;
const NUM_FORMANTS: usize = 3;
const SPLINE_SEG: usize = 320;
const SPLINE_TBL: usize = SPLINE_SEG * 4;
const SINE_TBL: usize = 1024;
const MAX_GRAIN: usize = 3840;
const PITCH_TABLE_BASE: f32 = 8.175_799;
const GRAIN_SECONDS: f32 = 0.02;
const ASPIRATION_PERIOD1: f32 = 0.000202;
const ASPIRATION_PERIOD2: f32 = 0.000263;
/// Shortest envelope ramp (no clicks).
const ANTICLICK: f32 = 0.003;
pub const MAX_UNISON: usize = 10;
pub const MAX_CHOIRS: usize = 4;

/// Formant frequencies (Hz) at the five vowel positions.
const FORMANT_FREQS: [[f32; 5]; NUM_FORMANTS] = [[280.0, 450.0, 800.0, 350.0, 270.0], [600.0, 800.0, 1150.0, 2000.0, 2140.0], [2240.0, 2830.0, 2900.0, 2800.0, 2950.0]];
/// Formant bandwidths: how fast each formant's sine decays.
const FORMANT_BW: [f32; NUM_FORMANTS] = [32.5, 47.5, 62.5];

/// Lookup tables, the same for every singer at a sample rate.
struct Tables {
    sr: f32,
    grain_len: usize,
    sine: Vec<f32>,
    formant: [Vec<f32>; NUM_FORMANTS],
    window: Vec<f32>,
    decay: Vec<f32>,
    aspiration: Vec<f32>,
}

impl Tables {
    fn new(sr: f32) -> Self {
        let grain_len = ((sr * GRAIN_SECONDS) as usize).min(MAX_GRAIN);
        let sine = (0..SINE_TBL).map(|i| (TAU * i as f32 / SINE_TBL as f32).sin()).collect();
        // Catmull-Rom splines through the five vowels (ends doubled).
        let formant = std::array::from_fn(|f| {
            let pts = FORMANT_FREQS[f];
            let padded = [pts[0], pts[0], pts[1], pts[2], pts[3], pts[4], pts[4]];
            let mut tbl = vec![0.0; SPLINE_TBL];
            for seg in 0..4 {
                let (p0, p1, p2, p3) = (padded[seg], padded[seg + 1], padded[seg + 2], padded[seg + 3]);
                for i in 0..SPLINE_SEG {
                    let t = i as f32 / SPLINE_SEG as f32;
                    let (t2, t3) = (t * t, t * t * t);
                    tbl[seg * SPLINE_SEG + i] = 0.5 * ((2.0 * p1) + (-p0 + p2) * t + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3);
                }
            }
            tbl
        });
        // Short cosine attack, flat, and a release that stays a little open.
        let attack_len = (sr * 0.0018) as usize;
        let release_start = (sr * 0.013) as usize;
        let release_len = (sr * 0.007) as usize;
        let mut window = vec![1.0f32; grain_len];
        for (i, w) in window.iter_mut().enumerate().take(attack_len.min(grain_len)) {
            *w = 0.5 * (1.0 - (PI * i as f32 / attack_len as f32).cos());
        }
        for (i, w) in window.iter_mut().enumerate().skip(release_start) {
            *w = 0.5 * (1.0 - (PI * (release_len + i) as f32 / release_len.max(1) as f32).cos());
        }
        let bw = 50.0 * PI;
        let decay: Vec<f32> = (0..grain_len * 4).map(|i| (-bw * i as f32 / sr).exp()).collect();
        // Breath: two inharmonic damped sines.
        let (p1, p2) = (sr * ASPIRATION_PERIOD1, sr * ASPIRATION_PERIOD2);
        let aspiration = (0..grain_len)
            .map(|i| {
                let d1 = decay.get((i as f32 * 3.0) as usize).copied().unwrap_or(0.0);
                let d2 = decay.get((i as f32 * 3.5) as usize).copied().unwrap_or(0.0);
                (TAU * i as f32 / p1).sin() * d1 + (TAU * i as f32 / p2).sin() * d2
            })
            .collect();
        Self { sr, grain_len, sine, formant, window, decay, aspiration }
    }

    fn formant(&self, f: usize, vowel: f32) -> f32 {
        let pos = vowel.clamp(0.0, 1.0) * (SPLINE_TBL - 1) as f32;
        let idx = (pos as usize).min(SPLINE_TBL - 2);
        let frac = pos - idx as f32;
        self.formant[f][idx] * (1.0 - frac) + self.formant[f][idx + 1] * frac
    }

    /// Samples between grains for a pitch (MIDI note space, an octave above
    /// what sounds), quantized to 1/32 semitone.
    fn period(&self, note: f32) -> f32 {
        let internal = note.clamp(0.0, 150.0) - 12.0;
        let idx = (internal * 32.0) as i32;
        let freq = 2f32.powf(idx as f32 / 384.0) * PITCH_TABLE_BASE;
        (self.sr / freq).max(1.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// One singer.
struct Singer {
    active: bool,
    current_pitch: f32,
    target_pitch: f32,
    glide: f32,
    vib_phase: f32,
    vib_depth: f32,
    vib_rate: f32,
    jitter: f32,
    jitter_counter: u32,
    jitter_period: u32,
    rng: u32,
    overlap: Vec<f32>,
    write: usize,
    read: usize,
    offset: usize,
    grain: Vec<f32>,
    vowel: f32,
    head: f32,
    breath: f32,
    dirty: bool,
    stage: Stage,
    env: f32,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    /// Samples of grains still to be read out after it stopped.
    tail: usize,
}

impl Singer {
    fn new(sr: f32, index: usize) -> Self {
        Self {
            active: false,
            current_pitch: 57.0,
            target_pitch: 57.0,
            glide: 0.5,
            // Staggered so the singers of a choir do not move in lockstep.
            vib_phase: (SINE_TBL * index) as f32 / MAX_UNISON as f32,
            vib_depth: 0.0,
            vib_rate: 0.5,
            jitter: 4.0,
            jitter_counter: 0,
            jitter_period: ((sr / 44100.0 * 4586.0) as u32).max(1),
            rng: 12345 + index as u32 * 7919,
            overlap: vec![0.0; OVERLAP],
            write: 0,
            read: 0,
            offset: 0,
            grain: vec![0.0; MAX_GRAIN],
            vowel: 0.5,
            head: 0.5,
            breath: 0.5,
            dirty: true,
            stage: Stage::Idle,
            env: 0.0,
            attack: 0.0,
            decay: 0.0,
            sustain: 1.0,
            release: 0.0,
            tail: 0,
        }
    }

    /// Still making sound (singing, or its last grains playing out).
    fn busy(&self) -> bool {
        self.is_active() || self.tail > 0
    }

    fn has_env(&self) -> bool {
        self.attack > 0.0 || self.decay > 0.0 || self.sustain < 1.0 || self.release > 0.0
    }

    fn is_active(&self) -> bool {
        self.active || self.stage == Stage::Release
    }

    fn note_on(&mut self, note: f32) {
        let was = self.active;
        self.active = true;
        self.target_pitch = note;
        if !was {
            self.current_pitch = note;
            self.dirty = true;
            // The overlap buffer drains on its own; new grains go where it is read.
            self.write = self.read;
            self.offset = 0;
        }
        if self.has_env() {
            self.stage = Stage::Attack;
        } else {
            self.env = 1.0;
            self.stage = Stage::Sustain;
        }
    }

    fn note_off(&mut self) {
        if self.has_env() {
            self.stage = Stage::Release;
        } else {
            self.active = false;
            self.stage = Stage::Idle;
            self.env = 0.0;
        }
    }

    fn rand(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_103_515_245).wrapping_add(12345);
        ((self.rng >> 16) & 0x7FFF) as f32 / 32768.0
    }

    fn env_tick(&mut self, sr: f32) -> f32 {
        match self.stage {
            Stage::Attack => {
                self.env += 1.0 / (self.attack.max(ANTICLICK) * sr);
                if self.env >= 1.0 {
                    self.env = 1.0;
                    self.stage = if self.decay > 0.0 { Stage::Decay } else { Stage::Sustain };
                }
            }
            Stage::Decay => {
                let rate = if self.decay > 0.0 { 1.0 / (self.decay * sr) } else { 1.0 };
                self.env -= rate * (1.0 - self.sustain);
                if self.env <= self.sustain {
                    self.env = self.sustain;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Release => {
                self.env -= 1.0 / (self.release.max(ANTICLICK) * sr);
                if self.env <= 0.0 {
                    self.env = 0.0;
                    self.stage = Stage::Idle;
                    self.active = false;
                }
            }
            Stage::Sustain | Stage::Idle => {}
        }
        self.env
    }

    /// Glide at a constant rate (12 semitones a second at Glide 0.99),
    /// snapping within 0.2 semitones.
    fn portamento(&mut self, sr: f32) {
        if self.glide > 0.001 {
            let diff = self.target_pitch - self.current_pitch;
            let rate = 12.0 / ((self.glide + 0.01) * sr);
            if diff > 0.2 {
                self.current_pitch += rate;
            } else if diff < -0.2 {
                self.current_pitch -= rate;
            } else {
                self.current_pitch = self.target_pitch;
            }
        } else {
            self.current_pitch = self.target_pitch;
        }
    }

    /// A sine vibrato whose rate wanders now and then. Even at depth 0 a
    /// 0.2 semitone wobble keeps the voice alive.
    fn vibrato(&mut self, t: &Tables) -> f32 {
        let rate_scale = 4f32.powf(self.vib_rate * 2.0 - 1.0);
        let rate = (self.vib_depth * 0.2 + 1.0) * self.jitter * rate_scale;
        self.vib_phase += rate / (t.sr / SINE_TBL as f32);
        if self.vib_phase >= SINE_TBL as f32 || self.vib_phase.is_nan() {
            self.vib_phase = self.vib_phase.rem_euclid(SINE_TBL as f32);
            if self.vib_phase.is_nan() {
                self.vib_phase = 0.0;
            }
        }
        self.jitter_counter += 1;
        if self.jitter_counter >= self.jitter_period {
            self.jitter_counter = 0;
            self.jitter = self.rand() * 2.0 + 5.0;
        }
        (self.vib_depth + 0.2) * t.sine[self.vib_phase as usize % SINE_TBL]
    }

    /// One grain: three damped sines at the vowel's formants (scaled by the
    /// head: 0.75 a baritone, 1.25 a soprano) plus breath, windowed.
    fn compute_grain(&mut self, t: &Tables) {
        let n = t.grain_len;
        let scale = self.head * 0.5 + 0.75;
        let formants: [f32; NUM_FORMANTS] = std::array::from_fn(|f| t.formant(f, self.vowel) * scale);
        let decay_len = n * 4;
        for i in 0..n {
            let time = i as f32 / t.sr;
            let mut sample = 0.0;
            for f in 0..NUM_FORMANTS {
                let sine = t.sine[(formants[f] * time * SINE_TBL as f32) as usize % SINE_TBL];
                let di = (FORMANT_BW[f] * i as f32 / 50.0) as usize;
                let decay = if di < decay_len { t.decay[di] } else { 0.0 };
                sample += sine * decay;
            }
            sample += t.aspiration[i] * self.breath;
            self.grain[i] = sample * t.window[i];
        }
        self.dirty = false;
    }

    fn overlap_add(&mut self, t: &Tables) {
        self.write = (self.write + self.offset) % OVERLAP;
        for i in 0..t.grain_len {
            self.overlap[(self.write + i) % OVERLAP] += self.grain[i];
        }
    }

    /// Adds this singer's next `out.len()` samples to `out`.
    fn process(&mut self, t: &Tables, out: &mut [f32]) {
        let has_env = self.has_env() || self.stage == Stage::Release;
        if self.is_active() {
            self.tail = t.grain_len + 2 * CHUNK;
        } else {
            self.tail = self.tail.saturating_sub(out.len());
        }
        for _ in 0..out.len() {
            if self.is_active() {
                self.portamento(t.sr);
                let vib = self.vibrato(t);
                let period = t.period(self.current_pitch + vib) as usize;
                if self.offset >= period || self.dirty {
                    self.compute_grain(t);
                    self.overlap_add(t);
                    self.offset = 0;
                }
            }
            self.offset += 1;
        }
        for o in out.iter_mut() {
            let mut s = self.overlap[self.read];
            self.overlap[self.read] = 0.0;
            self.read = (self.read + 1) % OVERLAP;
            if has_env {
                s *= self.env_tick(t.sr);
            }
            *o += s;
        }
    }
}

/// Louder at low pitches, where fewer grains overlap per second.
fn pitch_gain(note: f32) -> f32 {
    ((note - 12.0) * (-1.0 / 72.0) + 2.0).clamp(0.1, 3.0)
}

/// A note's choir: up to ten detuned singers.
struct Choir {
    singers: Vec<Singer>,
    count: usize,
    /// Key, its pitch for the singers (MIDI note space), velocity gain, age.
    note: u8,
    base: f32,
    vel: f32,
    gate: bool,
    age: u64,
    gain: f32,
}

impl Choir {
    fn new(sr: f32) -> Self {
        Self { singers: (0..MAX_UNISON).map(|i| Singer::new(sr, i)).collect(), count: 1, note: 0, base: 57.0, vel: 1.0, gate: false, age: 0, gain: 1.0 }
    }

    fn active(&self) -> bool {
        self.singers.iter().any(|s| s.is_active())
    }

    fn busy(&self) -> bool {
        self.singers.iter().any(|s| s.busy())
    }

    fn offset(i: usize, n: usize) -> f32 {
        if n <= 1 {
            0.0
        } else {
            i as f32 / (n - 1) as f32 * 2.0 - 1.0
        }
    }

    fn start(&mut self, base: f32, detune: f32) {
        self.base = base;
        let n = self.count;
        for (i, s) in self.singers.iter_mut().enumerate().take(n) {
            s.note_on(base + detune * Self::offset(i, n) / 100.0);
        }
    }
}

/// The stereo echo: two lines (about 310 and 400 ms), each feeding back into itself.
struct Echo {
    l: Vec<f32>,
    r: Vec<f32>,
    pos: usize,
    target: (f32, f32),
    current: (f32, f32),
    smooth: f32,
    sr: f32,
}

const ECHO_LINE: usize = 96000;

impl Echo {
    fn new(sr: f32) -> Self {
        let mut e = Self { l: vec![0.0; ECHO_LINE], r: vec![0.0; ECHO_LINE], pos: 0, target: (0.0, 0.0), current: (0.0, 0.0), smooth: 1.0 / (0.05 * sr), sr };
        e.set_rate(0.5);
        e.current = e.target;
        e
    }

    fn set_rate(&mut self, rate: f32) {
        let scale = self.sr / 44100.0 * (0.1 + rate.clamp(0.0, 1.0) * 1.9);
        self.target = ((13653.0 * scale).min(ECHO_LINE as f32 - 1.0), (17570.0 * scale).min(ECHO_LINE as f32 - 1.0));
    }

    fn tap(line: &[f32], pos: usize, delay: f32) -> f32 {
        let rd = pos as f32 - delay;
        let idx = rd.floor();
        let frac = rd - idx;
        let i = (idx as i64).rem_euclid(ECHO_LINE as i64) as usize;
        line[i] * (1.0 - frac) + line[(i + 1) % ECHO_LINE] * frac
    }

    /// Mono in, dry plus echo out.
    fn process(&mut self, mix: f32, input: &[f32], l: &mut [f32], r: &mut [f32]) {
        const FEEDBACK: f32 = 0.5;
        for i in 0..input.len() {
            let x = input[i];
            self.current.0 += self.smooth * (self.target.0 - self.current.0);
            self.current.1 += self.smooth * (self.target.1 - self.current.1);
            let tl = Self::tap(&self.l, self.pos, self.current.0);
            let tr = Self::tap(&self.r, self.pos, self.current.1);
            self.l[self.pos] = (tl * FEEDBACK + x) * mix;
            self.r[self.pos] = (tr * FEEDBACK + x) * mix;
            l[i] += x + tl;
            r[i] += x + tr;
            self.pos = (self.pos + 1) % ECHO_LINE;
        }
    }
}

const CHUNK: usize = 256;

pub struct MonkState {
    tables: Tables,
    choirs: Vec<Choir>,
    /// Held keys, newest last (one voice: the newest sounds).
    held: Vec<u8>,
    counter: u64,
    echo: Echo,
    mono: Vec<f32>,
    scratch: Vec<f32>,
    out_gain: f32,
    wobble: f32,
    sr: f32,
}

impl MonkState {
    pub fn new(sr: f32) -> Self {
        let sr = if (1000.0..=768000.0).contains(&sr) { sr } else { 44100.0 };
        Self {
            tables: Tables::new(sr),
            choirs: (0..MAX_CHOIRS).map(|_| Choir::new(sr)).collect(),
            held: Vec::with_capacity(32),
            counter: 0,
            echo: Echo::new(sr),
            mono: vec![0.0; CHUNK],
            scratch: vec![0.0; CHUNK],
            out_gain: -1.0,
            wobble: 0.0,
            sr,
        }
    }

    /// The pitch the singers get for a key (an octave above what sounds).
    fn base(p: &[f32], note: u8) -> f32 {
        note as f32 + 12.0 + 12.0 * p[OCTAVE].round()
    }

    fn vel_gain(p: &[f32], vel: f32) -> f32 {
        1.0 - p[VELOCITY].clamp(0.0, 1.0) * (1.0 - vel.clamp(0.0, 1.0))
    }

    fn poly(p: &[f32]) -> usize {
        (p[VOICES].round() as usize).clamp(1, MAX_CHOIRS)
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        self.apply(p);
        self.counter += 1;
        self.held.retain(|&n| n != note);
        if self.held.len() < self.held.capacity() {
            self.held.push(note);
        }
        let detune = p[DETUNE];
        let poly = Self::poly(p);
        let k = if poly == 1 {
            0
        } else {
            let pool = &self.choirs[..poly];
            pool.iter()
                .position(|c| c.gate && c.note == note)
                .or_else(|| pool.iter().position(|c| !c.active()))
                .or_else(|| pool.iter().enumerate().filter(|(_, c)| !c.gate).min_by_key(|(_, c)| c.age).map(|(i, _)| i))
                .unwrap_or_else(|| pool.iter().enumerate().min_by_key(|(_, c)| c.age).map(|(i, _)| i).unwrap_or(0))
        };
        let c = &mut self.choirs[k];
        c.note = note;
        c.vel = Self::vel_gain(p, vel);
        c.gate = true;
        c.age = self.counter;
        c.start(Self::base(p, note), detune);
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.held.retain(|&n| n != note);
        if p.len() < SPECS.len() {
            return;
        }
        if Self::poly(p) == 1 {
            let c = &mut self.choirs[0];
            if !c.gate {
                return;
            }
            match self.held.last() {
                // Back to the key still held, gliding.
                Some(&back) => {
                    c.note = back;
                    c.start(Self::base(p, back), p[DETUNE]);
                }
                None => {
                    c.gate = false;
                    for s in c.singers.iter_mut() {
                        s.note_off();
                    }
                }
            }
            return;
        }
        for c in self.choirs.iter_mut().filter(|c| c.gate && c.note == note) {
            c.gate = false;
            for s in c.singers.iter_mut() {
                s.note_off();
            }
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for c in self.choirs.iter_mut() {
            c.gate = false;
            for s in c.singers.iter_mut() {
                s.note_off();
            }
        }
    }

    /// Sends the knobs to the singers.
    fn apply(&mut self, p: &[f32]) {
        let n = (p[UNISON].round() as usize).clamp(1, MAX_UNISON);
        let wob = (self.wobble * TAU).sin() * p[WOBBLE].clamp(0.0, 1.0) * 0.5;
        let vowel = (p[VOWEL] + wob).clamp(0.0, 1.0);
        let detune = p[DETUNE];
        for c in self.choirs.iter_mut() {
            let old = c.count;
            c.count = n;
            if n != old && c.gate {
                // Singers that are no longer in the choir fade out; new ones join.
                for s in c.singers.iter_mut().take(old).skip(n) {
                    if s.is_active() {
                        s.stage = Stage::Release;
                    }
                }
                let base = c.base;
                for (i, s) in c.singers.iter_mut().enumerate().take(n) {
                    let pitch = base + detune * Choir::offset(i, n) / 100.0;
                    if i >= old {
                        s.note_on(pitch);
                    }
                }
            }
            for (i, s) in c.singers.iter_mut().enumerate() {
                s.glide = p[GLIDE].clamp(0.0, 1.0);
                s.vowel = vowel;
                s.head = (p[HEAD] + p[HEAD_SPREAD] * Choir::offset(i, n)).clamp(0.0, 1.0);
                s.vib_depth = p[VIBRATO].clamp(0.0, 1.0);
                s.vib_rate = p[VIB_RATE].clamp(0.0, 1.0);
                s.breath = p[BREATH].clamp(0.0, 1.0);
                s.attack = p[ATTACK].max(0.0);
                s.decay = p[DECAY].max(0.0);
                s.sustain = p[SUSTAIN].clamp(0.0, 1.0);
                s.release = p[RELEASE].max(0.0);
                if i < n && c.gate {
                    s.target_pitch = c.base + detune * Choir::offset(i, n) / 100.0;
                }
            }
        }
        self.echo.set_rate(p[ECHO_TIME]);
    }

    /// For the display: [0] choirs singing, [1] the vowel now, [2] the
    /// newest choir's pitch (sounding, MIDI note), [3] its level, [4] singers.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.choirs.iter().filter(|c| c.active()).count() as f32;
        if let Some(c) = self.choirs.iter().filter(|c| c.active()).max_by_key(|c| c.age) {
            let s = &c.singers[0];
            out[1] = s.vowel;
            out[2] = s.current_pitch - 12.0;
            out[3] = if s.has_env() { s.env } else { 1.0 };
            out[4] = c.count as f32;
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
        let len = l.len();
        self.wobble = (self.wobble + p[WOBBLE_RATE].max(0.0) * len as f32 / self.sr).fract();
        self.apply(p);
        let any = self.choirs.iter().any(|c| c.busy());
        // The echo rings on after the singing stops.
        let mono = &mut self.mono[..len];
        mono.fill(0.0);
        let k = 1.0 - (-1.0 / (0.005 * self.sr)).exp();
        if any {
            for c in self.choirs.iter_mut() {
                let target = 1.0 / (c.count as f32).sqrt() * c.vel;
                if !c.busy() {
                    continue;
                }
                let scratch = &mut self.scratch[..len];
                scratch.fill(0.0);
                for s in c.singers.iter_mut().filter(|s| s.busy()) {
                    s.process(&self.tables, scratch);
                }
                for (m, x) in mono.iter_mut().zip(scratch.iter()) {
                    c.gain += k * (target - c.gain);
                    *m += x * c.gain;
                }
            }
        }
        // Gain: the pitch compensation of the newest choir, and the volume.
        let pitch = self.choirs.iter().max_by_key(|c| c.age).map(|c| c.singers[0].current_pitch).unwrap_or(57.0);
        let target = pitch_gain(pitch) * 0.15 * db_to_gain(p[VOLUME]);
        if self.out_gain < 0.0 {
            self.out_gain = target;
        }
        let (mut el, mut er) = ([0.0f32; CHUNK], [0.0f32; CHUNK]);
        self.echo.process(p[ECHO].clamp(0.0, 1.0), mono, &mut el[..len], &mut er[..len]);
        for i in 0..len {
            self.out_gain += k * (target - self.out_gain);
            l[i] += el[i] * self.out_gain;
            r[i] += er[i] * self.out_gain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(p: &[f32], note: u8, secs: f32) -> (Vec<f32>, MonkState) {
        let mut s = MonkState::new(48000.0);
        s.note_on(p, note, 0.9);
        let n = (secs * 48000.0) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            s.render(p, a, b);
        }
        (l, s)
    }

    fn peak(v: &[f32]) -> f32 {
        v.iter().fold(0.0f32, |m, x| m.max(x.abs()))
    }

    /// The strongest period (in samples) between `lo` and `hi`, by autocorrelation.
    fn period(x: &[f32], lo: usize, hi: usize) -> usize {
        let x = &x[x.len() / 2..];
        let n = x.len() - hi;
        (lo..hi).max_by(|&a, &b| {
            let ca: f32 = (0..n).map(|i| x[i] * x[i + a]).sum();
            let cb: f32 = (0..n).map(|i| x[i] * x[i + b]).sum();
            ca.partial_cmp(&cb).unwrap()
        }).unwrap()
    }

    #[test]
    fn every_preset_chants_and_stops() {
        for name in preset_names() {
            let mut p = preset(name);
            p[ECHO] = 0.0;
            let (l, mut s) = play(&p, 57, 1.5);
            let pk = peak(&l);
            eprintln!("{name:<16} peak {pk:.3}");
            assert!(l.iter().all(|x| x.is_finite()), "{name}");
            assert!(pk > 0.05 && pk < 2.0, "{name}: peak {pk}");
            s.note_off(&p, 57);
            let (mut l, mut r) = (vec![0.0; 48000 * 6], vec![0.0; 48000 * 6]);
            for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
                s.render(&p, a, b);
            }
            assert!(s.choirs.iter().all(|c| !c.active()), "{name}: still singing");
            assert!(peak(&l[l.len() - 4800..]) < 1e-3, "{name}: still sounding");
        }
    }

    #[test]
    fn sings_the_key_or_an_octave_below() {
        let mut p = preset("Rabten");
        p[ECHO] = 0.0;
        p[OCTAVE] = 0.0;
        // A3 = 220 Hz: about 218 samples a period at 48 kHz.
        let (l, _) = play(&p, 57, 1.0);
        let t = period(&l, 150, 300);
        assert!((214..=222).contains(&t), "period {t}");
        p[OCTAVE] = -1.0;
        let (l, _) = play(&p, 57, 1.0);
        let t = period(&l, 300, 600);
        assert!((428..=444).contains(&t), "period {t}");
    }

    #[test]
    fn one_voice_glides_to_the_new_key() {
        let p = preset("Rabten");
        let (_, mut s) = play(&p, 48, 0.3);
        s.note_on(&p, 60, 0.9);
        let (mut l, mut r) = (vec![0.0; 4800], vec![0.0; 4800]);
        s.render(&p, &mut l, &mut r);
        let now = s.choirs[0].singers[0].current_pitch - 12.0 - 12.0 * p[OCTAVE];
        assert!(now > 48.5 && now < 59.5, "gliding: {now}");
        // Letting go goes back to the key still held.
        s.note_off(&p, 60);
        assert_eq!(s.choirs[0].note, 48);
        assert!(s.choirs[0].gate);
    }

    #[test]
    fn several_voices_sing_a_chord() {
        let mut p = preset("Choir of Monks");
        p[ECHO] = 0.0;
        let mut s = MonkState::new(48000.0);
        for n in [48, 52, 55] {
            s.note_on(&p, n, 0.9);
        }
        let (mut l, mut r) = (vec![0.0; 48000], vec![0.0; 48000]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            s.render(&p, a, b);
        }
        assert_eq!(s.choirs.iter().filter(|c| c.active()).count(), 3);
        assert!(peak(&l) > 0.05 && peak(&l) < 2.0);
    }

    #[test]
    fn the_vowel_moves_the_formants() {
        let t = Tables::new(48000.0);
        // The second formant climbs from the first vowel to the last.
        assert!(t.formant(1, 0.0) < t.formant(1, 1.0));
        assert!((t.formant(1, 0.0) - 600.0).abs() < 1.0);
        assert!((t.formant(1, 1.0) - 2140.0).abs() < 5.0);
    }
}
