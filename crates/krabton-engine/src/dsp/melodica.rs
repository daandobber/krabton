//! Melodica: a keyboard you blow into, the honky little reed organ of
//! cartoon bands and dub records. Every key opens a free reed (see
//! `free_reed`), and its tone follows a real melodica's, measured. The reed
//! lets the air through in a pulse whose width is never still: narrow as a
//! note starts (bright, its gaps far up), widening to nearly half the cycle
//! as the reed swings out (the even harmonics weak: the honk), and
//! breathing with the player's pressure and vibrato, so the gaps in the
//! spectrum sweep through the harmonics and each rises and falls by many
//! dB, as they do in the recordings. The case keeps it level up to about
//! 3 kHz, then it falls away steeply, some 20 dB by 5.6 kHz (Tone moves
//! where; Body how strongly the case colours it); each reed has a few dB
//! of its own. Blown softly it is darker above 500 Hz, up to 14 dB; at the
//! start the high harmonics come in last. Fitted to recordings of a
//! melodica (Sampleson's Melox samples, 2017): only measured numbers are
//! here, no recording.
//!
//! It is a real one, not a perfect one: every key is a few cents off in its
//! own way (Tuning), so chords beat; the breath is never still (Life): the
//! pressure wanders, and with it the loudness, the brightness and a little
//! of the pitch, and the vibrato from the diaphragm wavers in speed and
//! depth; as measured, a player's breath moves the loudness by a few dB two
//! or three times a second while the pitch holds within a cent or two. A
//! note starts some 40 cents flat and rises into place in 35 ms. A hard
//! note is tongued ("pah"), a soft one swells in; letting go shuts the valve
//! with a puff and the pitch lifts as the pressure drops. There is hardly
//! any breath in the held tone, only as a note starts. The plastic keys
//! click (Keys): a short tick between 0.5 and 2 kHz.
//!
//! Played with the mouth (Playing: Mouth) all the reeds share one breath:
//! a chord divides the air between its notes, a key added dips it for a
//! moment, and a long phrase leaves the lungs a little empty (the tone
//! thins, the breath shows) until the player rests. Free gives every key
//! its own air, like a harmonium's bellows. Blown through the tube (Tube)
//! it starts softer and sounds warmer. A second reed a few cents off makes
//! it beat (an accordion's musette).
//!
//! And the dub desk after it: Drive (a warm preamp and tape), Echo (a tape
//! echo on the song's beat, the repeats darkening) and Spring (a spring
//! reverb that the echoes fall into too), as well as a room (Space).
//! Original code.

use super::effects::FxCtx;
use super::free_reed::{free_reed, Blow, Reed};
use super::fx::echo::{self, Echo};
use super::fx::hall::{self, Hall};
use super::fx::spring::{self, SpringReverb};
use super::{db_to_gain, midi_to_freq, pan_gains, OnePole, Rng};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

pub const BREATH: usize = 0;
pub const TONE: usize = 1;
pub const BODY: usize = 2;
pub const BEAT: usize = 3;
pub const VIBRATO: usize = 4;
pub const VIB_RATE: usize = 5;
pub const ATTACK: usize = 6;
pub const RELEASE: usize = 7;
pub const VOICES: usize = 8;
pub const GLIDE: usize = 9;
pub const TRANSPOSE: usize = 10;
pub const PLAYING: usize = 11;
pub const TUBE: usize = 12;
pub const SPREAD: usize = 13;
pub const SPACE: usize = 14;
pub const VOLUME: usize = 15;
pub const TUNING: usize = 16;
pub const LIFE: usize = 17;
pub const KEYS: usize = 18;
pub const DRIVE: usize = 19;
pub const ECHO: usize = 20;
pub const ECHO_TIME: usize = 21;
pub const SPRING: usize = 22;

pub const PLAYINGS: &[&str] = &["Free", "Mouth"];
pub const ECHO_TIMES: &[&str] = &["1/16 D", "1/8", "1/8 D", "1/4", "1/4 D"];
/// The echo's Sync choice for each of them.
const ECHO_SYNC: [f32; 5] = [2.0, 3.0, 4.0, 5.0, 6.0];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Breath", "Reed", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Tone", "Reed", 0.0, 1.0, 0.55, Unit::Percent),
    ParamSpec::lin("Body", "Reed", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Beat", "Reed", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vibrato", "Expression", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::lin("Vibrato Rate", "Expression", 3.0, 8.0, 5.0, Unit::Hz),
    ParamSpec::log("Attack", "Expression", 0.005, 0.8, 0.03, Unit::Sec),
    ParamSpec::log("Release", "Expression", 0.02, 2.0, 0.12, Unit::Sec),
    ParamSpec::int("Voices", "Play", 1.0, 8.0, 8.0, Unit::None),
    ParamSpec::lin("Glide", "Play", 0.0, 0.5, 0.0, Unit::Sec),
    ParamSpec::int("Transpose", "Play", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::choice("Playing", "Play", PLAYINGS, 1),
    ParamSpec::lin("Tube", "Play", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Spread", "Sound", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Space", "Sound", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Volume", "Sound", -24.0, 12.0, 0.0, Unit::Db),
    ParamSpec::lin("Tuning", "Reed", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Life", "Expression", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Keys", "Reed", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Drive", "Dub", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Echo", "Dub", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Echo Time", "Dub", ECHO_TIMES, 2),
    ParamSpec::lin("Spring", "Dub", 0.0, 1.0, 0.0, Unit::Percent),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Melodica",
        "Dusty Melodica",
        "Dub Melodica",
        "Rockers Melodica",
        "Echo Chamber",
        "Bright Melodica",
        "Tube Melodica",
        "Lullaby",
        "Bass Melodica",
        "Accordion",
        "Harmonium",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Melodica" => &[(SPACE, 0.2)],
        // Close up, a little worn and out of tune, a small room.
        "Dusty Melodica" => &[(TUNING, 0.55), (BODY, 0.8), (TONE, 0.5), (DRIVE, 0.35), (VIBRATO, 0.12), (LIFE, 0.55), (KEYS, 0.4), (BREATH, 0.3), (RELEASE, 0.1), (SPACE, 0.15)],
        // Breathing, swelling, into the echo and the spring.
        "Dub Melodica" => &[(VIBRATO, 0.4), (VIB_RATE, 5.2), (LIFE, 0.65), (TUNING, 0.35), (BODY, 0.75), (TONE, 0.5), (ATTACK, 0.05), (RELEASE, 0.25), (DRIVE, 0.25), (ECHO, 0.5), (ECHO_TIME, 2.0), (SPRING, 0.5), (SPACE, 0.1)],
        "Rockers Melodica" => &[(TONE, 0.7), (VIBRATO, 0.25), (LIFE, 0.5), (TUNING, 0.3), (BODY, 0.65), (DRIVE, 0.3), (ECHO, 0.35), (ECHO_TIME, 3.0), (SPRING, 0.35), (RELEASE, 0.15), (SPACE, 0.1)],
        "Echo Chamber" => &[(VIBRATO, 0.45), (LIFE, 0.7), (TUNING, 0.4), (TONE, 0.45), (BODY, 0.8), (ATTACK, 0.07), (RELEASE, 0.35), (DRIVE, 0.4), (ECHO, 0.8), (ECHO_TIME, 2.0), (SPRING, 0.75), (SPACE, 0.2)],
        "Bright Melodica" => &[(TONE, 0.85), (BREATH, 0.15), (BODY, 0.5), (SPACE, 0.2)],
        "Tube Melodica" => &[(TUBE, 0.85), (TONE, 0.35), (BREATH, 0.35), (ATTACK, 0.06), (RELEASE, 0.2), (SPACE, 0.3)],
        "Lullaby" => &[(VIBRATO, 0.25), (TONE, 0.35), (BREATH, 0.3), (ATTACK, 0.08), (RELEASE, 0.4), (SPACE, 0.55)],
        "Bass Melodica" => &[(TRANSPOSE, -12.0), (TONE, 0.4), (BODY, 0.7), (VOICES, 4.0), (BREATH, 0.3), (SPACE, 0.2), (VOLUME, -3.0)],
        "Accordion" => &[(PLAYING, 0.0), (BEAT, 0.55), (BODY, 0.4), (VIBRATO, 0.0), (TONE, 0.65), (ATTACK, 0.02), (RELEASE, 0.08), (TUNING, 0.1), (LIFE, 0.2), (KEYS, 0.35), (SPACE, 0.2)],
        "Harmonium" => &[(PLAYING, 0.0), (BEAT, 0.08), (BODY, 0.3), (TONE, 0.4), (BREATH, 0.15), (VIBRATO, 0.05), (ATTACK, 0.15), (RELEASE, 0.5), (TUNING, 0.1), (LIFE, 0.25), (KEYS, 0.15), (SPACE, 0.5)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

const MAX_VOICES: usize = 8;
const CHUNK: usize = 64;

/// A melodica's case over frequency (dB), fitted to recordings of a real
/// one with the reed's own fall (some 1.3 dB an octave) taken out: level to
/// about 3 kHz with small bumps, then a steep fall. Below 266 Hz it holds.
const BODY_HZ: [f32; 21] = [265.7, 321.4, 388.9, 470.5, 569.2, 688.7, 833.3, 1008.1, 1219.7, 1475.7, 1785.5, 2160.2, 2613.6, 3162.1, 3825.8, 4628.8, 5600.3, 6775.7, 8197.8, 9918.3, 12000.0];
const BODY_DB: [f32; 21] = [0.0, -1.2, -2.1, -2.2, -1.3, 0.1, -0.5, -2.8, -2.0, -3.5, -4.0, -2.9, -0.1, -1.0, -4.7, -13.2, -21.0, -28.3, -21.6, -29.2, -43.5];

/// The body's level at `f` Hz, between the measured points.
fn body_db(f: f32) -> f32 {
    if f <= BODY_HZ[0] {
        return BODY_DB[0];
    }
    let n = BODY_HZ.len();
    if f >= BODY_HZ[n - 1] {
        return BODY_DB[n - 1] - 24.0 * (f / BODY_HZ[n - 1]).log2();
    }
    let i = BODY_HZ.partition_point(|&h| h <= f) - 1;
    let t = (f / BODY_HZ[i]).ln() / (BODY_HZ[i + 1] / BODY_HZ[i]).ln();
    BODY_DB[i] + (BODY_DB[i + 1] - BODY_DB[i]) * t
}

/// Each reed's own few dB, harmonic by harmonic (-1..1), always the same.
fn reed_print(note: u8, k: usize) -> f32 {
    let h = (note as u32).wrapping_mul(2_654_435_761) ^ (k as u32).wrapping_mul(2_246_822_519);
    let h = h ^ (h >> 15);
    (h.wrapping_mul(0x2c1b_3c6d) >> 16 & 0xffff) as f32 / 32_767.5 - 1.0
}

/// The level (dB) of harmonic `k` at `f` Hz of key `note`, blown `blown`
/// hard, `env` far into its start, `t` seconds after it; `swing` is how the
/// breath moves the reed now (1: still); `tone` moves where it falls away,
/// `body` sets how strongly the case colours it.
#[allow(clippy::too_many_arguments)]
fn melodica_db(f: f32, k: usize, note: u8, blown: f32, env: f32, t: f32, swing: f32, tone: f32, body: f32) -> f32 {
    // The air pulse: narrow as it starts, nearly half the cycle once the
    // reed swings out, breathing with the pressure. Its gaps are never
    // quite empty.
    // (Measured: once it sounds the pulse is wider the higher the key, about
    // 0.38 of the cycle at C4 to nearly half at C5, where the odd harmonics
    // ring and the even ones all but vanish.)
    let grow = 1.0 - (-t / 0.12).exp();
    let held = 0.38 + 0.12 * ((note as f32 - 60.0) / 12.0).clamp(-0.5, 1.0);
    let duty = (0.2 + (held - 0.2) * grow * swing).clamp(0.12, 0.5);
    let x = std::f32::consts::PI * k as f32 * duty;
    let pulse = (x.sin() / x).abs().max(0.03) * (k as f32).powf(0.85);
    let fe = f / (0.75 + 0.5 * tone);
    let mut db = 20.0 * pulse.log10() + body_db(fe) * (0.5 + 0.8 * body);
    // Every reed a few dB of its own.
    db += 2.0 * reed_print(note, k) * if k <= 2 { 0.5 } else { 1.0 };
    // Softly blown, and at the very start, the high harmonics are weaker.
    let over = ((fe / 500.0).log2() / 3.0).clamp(0.0, 1.0);
    let started = env.clamp(0.0, 1.0);
    db - over * (14.0 * (1.0 - blown).clamp(0.0, 1.0) + 24.0 * (1.0 - started * started))
}

/// How far a key is off, -1..1, always the same for the same key.
fn key_error(note: u8) -> f32 {
    let h = (note as u32).wrapping_mul(2_654_435_761) >> 16;
    (h & 0xffff) as f32 / 32_767.5 - 1.0
}

pub struct MelodicaState {
    /// Pitch bend in semitones, set by the engine.
    pub pitch_bend: f32,
    /// The song's tempo, where it is and whether it plays, set by the engine
    /// (for the echo).
    pub bpm: f32,
    pub beat: f64,
    pub playing: bool,
    sr: f32,
    voices: Vec<Reed>,
    stack: Vec<u8>,
    rng: Rng,
    vib_ph: f32,
    vib: Vec<f32>,
    /// The air dips when a key opens; the lungs empty over a long phrase.
    dip: f32,
    lungs: f32,
    rest: f32,
    air: f32,
    /// The breath wandering: pressure, pitch, the vibrato's speed and depth.
    wander: [f32; 4],
    /// A key's click and thump.
    click: f32,
    thump: f32,
    thump_ph: f32,
    click_hp: OnePole,
    click_lp: OnePole,
    tape_lp: [OnePole; 2],
    tmp: Vec<f32>,
    dl: Vec<f32>,
    dr: Vec<f32>,
    wl: Vec<f32>,
    wr: Vec<f32>,
    el: Vec<f32>,
    er: Vec<f32>,
    hall: Box<Hall>,
    hall_p: Vec<f32>,
    echo: Box<Echo>,
    echo_p: Vec<f32>,
    spring: Box<SpringReverb>,
    spring_p: Vec<f32>,
}

impl MelodicaState {
    pub fn new(sr: f32) -> Self {
        Self {
            pitch_bend: 0.0,
            bpm: 120.0,
            beat: 0.0,
            playing: false,
            sr: sr.max(1000.0),
            voices: (0..MAX_VOICES).map(|_| Reed::default()).collect(),
            stack: Vec::with_capacity(32),
            rng: Rng::new(0x3E10_D1CA),
            vib_ph: 0.0,
            vib: vec![0.0; CHUNK],
            dip: 0.0,
            lungs: 0.0,
            rest: 1.0,
            air: 0.0,
            wander: [0.0; 4],
            click: 0.0,
            thump: 0.0,
            thump_ph: 0.0,
            click_hp: OnePole::default(),
            click_lp: OnePole::default(),
            tape_lp: [OnePole::default(), OnePole::default()],
            tmp: vec![0.0; CHUNK],
            dl: vec![0.0; CHUNK],
            dr: vec![0.0; CHUNK],
            wl: vec![0.0; CHUNK],
            wr: vec![0.0; CHUNK],
            el: vec![0.0; CHUNK],
            er: vec![0.0; CHUNK],
            hall: Box::new(Hall::new(sr)),
            hall_p: crate::params::defaults(hall::SPECS),
            echo: Box::new(Echo::new(sr)),
            echo_p: crate::params::defaults(echo::SPECS),
            spring: Box::new(SpringReverb::new(sr)),
            spring_p: crate::params::defaults(spring::SPECS),
        }
    }

    /// The key's pitch: its note, and how far this melodica has it off.
    fn freq(p: &[f32], note: u8) -> f32 {
        let cents = p[TUNING].clamp(0.0, 1.0) * 12.0 * key_error(note);
        midi_to_freq(note as f32 + p[TRANSPOSE].round() + cents / 100.0)
    }

    fn pan_of(p: &[f32], note: u8) -> f32 {
        p[SPREAD].clamp(0.0, 1.0) * ((note as f32 - 64.0) / 24.0).clamp(-1.0, 1.0)
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        if self.stack.len() < 32 {
            self.stack.retain(|&n| n != note);
            self.stack.push(note);
        }
        // One breath: a key added takes some of the air for a moment.
        if p[PLAYING] >= 0.5 && self.voices.iter().any(|v| v.gate) {
            self.dip = 0.22;
        }
        // The key goes down.
        let keys = p[KEYS].clamp(0.0, 1.0);
        self.click += keys * (0.4 + 0.6 * vel);
        self.thump += keys * (0.3 + 0.7 * vel);
        let (f, pan) = (Self::freq(p, note), Self::pan_of(p, note));
        let poly = (p[VOICES].round() as usize).clamp(1, MAX_VOICES);
        if poly == 1 {
            let v = &mut self.voices[0];
            if v.gate {
                // Legato: the next key while the breath goes on.
                v.target = f;
                if p[GLIDE] <= 0.001 {
                    v.f = f;
                }
                v.note = note;
                v.owner = note;
                v.pan = pan;
            } else {
                v.start(note, note, f, vel, 1.0, pan);
                v.rise = 0.08;
                v.bend_from = -0.6;
                v.bend_t = 0.035;
            }
            return;
        }
        let i = (0..poly).find(|&i| self.voices[i].owner == note && self.voices[i].gate).unwrap_or_else(|| free_reed(&self.voices[..poly]));
        let v = &mut self.voices[i];
        v.start(note, note, f, vel, 1.0, pan);
        // As the valve shuts and the pressure drops, it lifts a little; it
        // starts some 30 cents flat and rises into place.
        v.rise = 0.08;
        v.bend_from = -0.6;
        v.bend_t = 0.035;
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.stack.retain(|&n| n != note);
        if p.len() < SPECS.len() {
            return;
        }
        self.click += p[KEYS].clamp(0.0, 1.0) * 0.25;
        let poly = (p[VOICES].round() as usize).clamp(1, MAX_VOICES);
        if poly == 1 {
            let v = &mut self.voices[0];
            if v.owner == note {
                if let Some(&prev) = self.stack.last() {
                    v.target = Self::freq(p, prev);
                    if p[GLIDE] <= 0.001 {
                        v.f = v.target;
                    }
                    v.note = prev;
                    v.owner = prev;
                } else {
                    v.gate = false;
                }
            }
            return;
        }
        for v in self.voices.iter_mut().filter(|v| v.owner == note) {
            v.gate = false;
        }
    }

    pub fn all_off(&mut self) {
        self.stack.clear();
        for v in self.voices.iter_mut() {
            v.gate = false;
        }
    }

    /// The screen: [0] on, [1] the air, then per voice (2 + 2i) its note and
    /// how loud it is.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = 1.0;
        out[1] = self.air;
        for (i, v) in self.voices.iter().enumerate() {
            out[2 + 2 * i] = v.note as f32;
            out[3 + 2 * i] = v.level;
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let sr = self.sr;
        let k = |i: usize| p[i].clamp(0.0, 1.0);
        let mouth = p[PLAYING] >= 0.5;
        let tube = k(TUBE);
        let att = 1.0 - (-1.0 / (p[ATTACK].clamp(0.002, 2.0) * (1.0 + 2.0 * tube) * 0.4 * sr)).exp();
        let rel = 1.0 - (-1.0 / (p[RELEASE].clamp(0.01, 4.0) * 0.4 * sr)).exp();
        let glide = p[GLIDE].clamp(0.0, 1.0);
        let glide_k = if glide > 0.001 { 1.0 - (-1.0 / (glide * 0.35 * sr)).exp() } else { 1.0 };
        let (depth, rate) = (k(VIBRATO), p[VIB_RATE].clamp(1.0, 12.0));
        let life = k(LIFE);
        let body = k(BODY);
        let tone = (k(TONE) - 0.2 * tube).max(0.0);
        let (space, drive, echo_amt, spring_amt) = (k(SPACE), k(DRIVE), k(ECHO), k(SPRING));
        let gain = db_to_gain(p[VOLUME]) * 0.2;
        for c in 0..2 {
            self.tape_lp[c].set(9000.0 - 3500.0 * drive, sr);
        }
        self.click_hp.set(500.0, sr);
        self.click_lp.set(2000.0, sr);
        let n = l.len().min(r.len());
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            let dt = len as f32 / sr;
            // The breath wanders: slow, unsteady, never a machine's.
            for (w, fc) in self.wander.iter_mut().zip([2.5f32, 0.7, 0.35, 0.25]) {
                let a = (TAU * fc * dt).min(0.5);
                *w += a * (self.rng.bipolar() / (a / 6.0).sqrt() - *w);
            }
            let vib_rate = rate * (1.0 + 0.12 * life * self.wander[2]).clamp(0.6, 1.5);
            let vib_depth = depth * (1.0 + 0.35 * life * self.wander[3]).clamp(0.2, 1.8);
            for j in 0..len {
                self.vib_ph = (self.vib_ph + vib_rate / sr).fract();
                self.vib[j] = (TAU * self.vib_ph).sin() * vib_depth;
            }
            // One breath for all the keys (Mouth), its own for each (Free).
            let open = self.voices.iter().filter(|v| v.gate).count();
            let tired = if mouth {
                if open > 0 {
                    self.lungs += dt;
                    self.rest = 0.0;
                } else {
                    self.rest += dt;
                    if self.rest > 0.3 {
                        self.lungs = 0.0;
                    }
                }
                ((self.lungs - 8.0) / 6.0).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let wander = db_to_gain(7.5 * life * self.wander[0].clamp(-2.5, 2.5));
            // The breath swings the reed: the pressure's wander and the vibrato.
            let swing = (1.0 + 0.12 * life * self.wander[0].clamp(-2.5, 2.5) + 0.15 * self.vib[0]).clamp(0.6, 1.3);
            let shape = move |f: f32, k: usize, note: u8, blown: f32, env: f32, t: f32| melodica_db(f, k, note, blown, env, t, swing, tone, body);
            let share = if mouth { (1.0 / open.max(1) as f32).sqrt() * (1.0 - self.dip) * (1.0 - 0.3 * tired) } else { 1.0 } * wander;
            self.dip *= (-dt / 0.06).exp();
            let blow = Blow {
                sr,
                att,
                rel,
                glide_k,
                bright: 0.93 + 0.06 * k(TONE) - 0.06 * tube,
                duty: 0.36 - 0.14 * k(TONE),
                breath: k(BREATH) * 0.12 * (1.0 + 0.6 * tube) * (1.0 + 0.6 * tired),
                beat: k(BEAT) * 0.3,
                vib: &self.vib[..len],
                vib_amp: 0.5,
                vib_pitch: 0.02,
                pitch_bend: self.pitch_bend + 0.015 * life * self.wander[1],
                share,
                radiate: 0.75 - 0.3 * tube,
                ping: 0.04,
                tongue: 0.35 * (1.0 - 0.6 * tube),
                puff_off: 0.5,
                shape: Some(&shape),
            };
            self.dl[..len].fill(0.0);
            self.dr[..len].fill(0.0);
            let mut air = 0.0f32;
            for v in self.voices.iter_mut() {
                if v.quiet() {
                    v.level *= 0.9;
                    continue;
                }
                self.tmp[..len].fill(0.0);
                v.block(&blow, &mut self.rng, &mut self.tmp[..len]);
                air = air.max(v.env * share);
                let (pl, pr) = pan_gains(v.pan);
                for j in 0..len {
                    self.dl[j] += self.tmp[j] * pl;
                    self.dr[j] += self.tmp[j] * pr;
                }
            }
            self.air = self.air * 0.8 + air * 0.2;
            // The keys; the preamp and tape.
            let click_k = (-1.0 / (0.012 * sr)).exp();
            let thump_k = (-1.0 / (0.008 * sr)).exp();
            let drive_g = 1.0 + 5.0 * drive;
            for j in 0..len {
                let nz = self.click_lp.lp(self.click_hp.hp(self.rng.bipolar()));
                self.thump_ph = (self.thump_ph + 240.0 / sr).fract();
                let key = (nz * self.click * 0.6 + (TAU * self.thump_ph).sin() * self.thump * 0.15) * 0.06;
                self.click *= click_k;
                self.thump *= thump_k;
                for (c, d) in [&mut self.dl, &mut self.dr].into_iter().enumerate() {
                    let mut y = d[j] * gain + key;
                    if drive > 0.001 {
                        y = self.tape_lp[c].lp((y * drive_g).tanh() / drive_g.sqrt());
                    }
                    d[j] = y;
                }
            }
            let ctx = FxCtx { sr, bpm: if self.bpm > 1.0 { self.bpm } else { 120.0 }, beat: self.beat, playing: self.playing };
            // The dub desk: the echo, then the spring that the echoes fall into too.
            self.el[..len].fill(0.0);
            self.er[..len].fill(0.0);
            if echo_amt > 0.001 {
                self.el[..len].copy_from_slice(&self.dl[..len]);
                self.er[..len].copy_from_slice(&self.dr[..len]);
                let ep = &mut self.echo_p;
                ep[echo::SYNC] = ECHO_SYNC[(p[ECHO_TIME].round().max(0.0) as usize).min(ECHO_SYNC.len() - 1)];
                ep[echo::FEEDBACK] = 0.3 + 0.45 * echo_amt;
                ep[echo::WOW] = 0.35;
                ep[echo::FLUTTER] = 0.25;
                ep[echo::SATURATION] = 0.4;
                ep[echo::TONE] = 2800.0;
                ep[echo::LOW_CUT] = 250.0;
                ep[echo::SPREAD] = 0.5;
                ep[echo::MIX] = 1.0;
                self.echo.process(&self.echo_p, &mut self.el[..len], &mut self.er[..len], &ctx);
            }
            if spring_amt > 0.001 {
                for j in 0..len {
                    self.wl[j] = self.dl[j] + self.el[j] * 0.6;
                    self.wr[j] = self.dr[j] + self.er[j] * 0.6;
                }
                let sp = &mut self.spring_p;
                sp[spring::DECAY] = 0.5 + 0.4 * spring_amt;
                sp[spring::TENSION] = 0.5;
                sp[spring::LENGTH] = 0.55;
                sp[spring::TONE] = 3500.0;
                sp[spring::DRIP] = 0.45;
                sp[spring::DRIVE] = 0.15;
                sp[spring::MIX] = 1.0;
                self.spring.process(&self.spring_p, &mut self.wl[..len], &mut self.wr[..len], &ctx);
                for j in 0..len {
                    self.el[j] = self.el[j] * echo_amt * 0.7 + self.wl[j] * spring_amt * 0.6;
                    self.er[j] = self.er[j] * echo_amt * 0.7 + self.wr[j] * spring_amt * 0.6;
                }
            } else {
                for j in 0..len {
                    self.el[j] *= echo_amt * 0.7;
                    self.er[j] *= echo_amt * 0.7;
                }
            }
            // And the room.
            if space > 0.001 {
                self.wl[..len].copy_from_slice(&self.dl[..len]);
                self.wr[..len].copy_from_slice(&self.dr[..len]);
                let hp = &mut self.hall_p;
                hp[hall::PREDELAY] = 20.0;
                hp[hall::SIZE] = 0.45 + 0.45 * space;
                hp[hall::DECAY] = 1.4 + 4.0 * space;
                hp[hall::LOW_DECAY] = 0.9;
                hp[hall::DAMPING] = 5500.0;
                hp[hall::DIFFUSION] = 0.8;
                hp[hall::MODULATION] = 0.2;
                hp[hall::FREEZE] = 0.0;
                hp[hall::WIDTH] = 1.0;
                hp[hall::MIX] = 1.0;
                self.hall.process(&self.hall_p, &mut self.wl[..len], &mut self.wr[..len], &ctx, None);
                for j in 0..len {
                    self.el[j] += self.wl[j] * space * 0.8;
                    self.er[j] += self.wr[j] * space * 0.8;
                }
            }
            for j in 0..len {
                l[done + j] += (self.dl[j] + self.el[j]).clamp(-1.5, 1.5);
                r[done + j] += (self.dr[j] + self.er[j]).clamp(-1.5, 1.5);
            }
            done += len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn run(st: &mut MelodicaState, p: &[f32], secs: f32) -> Vec<f32> {
        let n = (secs * SR) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            st.render(p, a, b);
        }
        l
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

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// How much of it is above 1.5 kHz.
    fn high_share(x: &[f32]) -> f32 {
        let a = (-TAU * 1500.0 / SR).exp();
        let mut lp = 0.0;
        let hp: Vec<f32> = x
            .iter()
            .map(|&s| {
                lp = lp * a + s * (1.0 - a);
                s - lp
            })
            .collect();
        rms(&hp) / rms(x)
    }

    /// A preset still, dry and in tune, to measure.
    fn dry(name: &str) -> Vec<f32> {
        let mut p = preset(name);
        p[SPACE] = 0.0;
        p[VIBRATO] = 0.0;
        p[BEAT] = 0.0;
        p[LIFE] = 0.0;
        p[TUNING] = 0.0;
        p[ECHO] = 0.0;
        p[SPRING] = 0.0;
        p[KEYS] = 0.0;
        p
    }

    #[test]
    fn every_preset_plays_its_note_and_stays_bounded() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = MelodicaState::new(SR);
            st.note_on(&p, 64, 0.8);
            let l = run(&mut st, &p, 1.2);
            st.note_off(&p, 64);
            let l2 = run(&mut st, &p, 2.0);
            let peak = l.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            if std::env::var("REED_REPORT").is_ok() {
                eprintln!("{name}: peak {peak:.3}");
            }
            assert!(l.iter().chain(&l2).all(|v| v.is_finite()), "{name}");
            assert!(peak > 0.02 && peak < 1.2, "{name}: {peak}");
            // Its own pitch, a few cents off at most.
            let f = midi_to_freq(64.0 + p[TRANSPOSE]);
            let tail = &l[(0.5 * SR) as usize..];
            assert!(mag(tail, f) > 3.0 * mag(tail, f * 1.06), "{name}: pitch");
        }
    }

    #[test]
    fn one_breath_shares_its_air_between_the_keys_of_a_chord() {
        // Three keys sound louder than one, but with one breath much less so.
        let level = |mouth: bool, chord: bool| {
            let mut p = dry("Melodica");
            p[PLAYING] = if mouth { 1.0 } else { 0.0 };
            let mut st = MelodicaState::new(SR);
            st.note_on(&p, 60, 0.8);
            if chord {
                st.note_on(&p, 64, 0.8);
                st.note_on(&p, 67, 0.8);
            }
            let l = run(&mut st, &p, 1.0);
            rms(&l[(0.4 * SR) as usize..])
        };
        let mouth = level(true, true) / level(true, false);
        let free = level(false, true) / level(false, false);
        assert!(mouth < 0.75 * free, "mouth {mouth} free {free}");
    }

    #[test]
    fn blown_harder_it_is_brighter() {
        let bright = |vel: f32| {
            let p = dry("Melodica");
            let mut st = MelodicaState::new(SR);
            st.note_on(&p, 57, vel);
            let l = run(&mut st, &p, 1.0);
            high_share(&l[(0.4 * SR) as usize..])
        };
        assert!(bright(1.0) > 1.2 * bright(0.25), "{} {}", bright(1.0), bright(0.25));
    }

    /// Harmonics 1 to 16 of a note, each against the first.
    fn profile(p: &[f32], note: u8) -> Vec<f32> {
        let mut st = MelodicaState::new(SR);
        st.note_on(p, note, 0.8);
        let l = run(&mut st, p, 1.0);
        let t = &l[(0.4 * SR) as usize..];
        // The real pitch: the strongest near the note.
        let f0 = midi_to_freq(note as f32);
        let f = (-30..=30).map(|c| f0 * 2f32.powf(c as f32 / 1200.0)).max_by(|a, b| mag(t, *a).total_cmp(&mag(t, *b))).unwrap();
        let h1 = mag(t, f);
        (1..=16).map(|k| mag(t, f * k as f32) / h1).collect()
    }

    #[test]
    fn a_melodica_is_reedy_not_a_flute() {
        // Its upper harmonics stay strong: the honk of a free reed.
        for name in ["Melodica", "Dusty Melodica", "Dub Melodica"] {
            let pr = profile(&dry(name), 57);
            if std::env::var("REED_REPORT").is_ok() {
                eprintln!("{name}: {}", pr.iter().map(|v| format!("{:.0}", 20.0 * v.max(1e-6).log10())).collect::<Vec<_>>().join(" "));
            }
            let mid = pr[3..8].iter().sum::<f32>() / 5.0;
            let high = pr[9..14].iter().sum::<f32>() / 5.0;
            assert!(mid > 0.25, "{name}: harmonics 4-8 at {:.1} dB", 20.0 * mid.log10());
            assert!(high > 0.1, "{name}: harmonics 10-14 at {:.1} dB", 20.0 * high.log10());
        }
    }

    #[test]
    fn its_harmonics_move_as_a_real_ones_do() {
        // The second harmonic at the start and later: the pulse widens, it
        // falls away (as in the recordings, by many dB).
        let p = dry("Melodica");
        let mut st = MelodicaState::new(SR);
        st.note_on(&p, 67, 0.8);
        let l = run(&mut st, &p, 1.0);
        let f = midi_to_freq(67.0);
        let h2 = |a: f32, b: f32| {
            let w = &l[(a * SR) as usize..(b * SR) as usize];
            mag(w, 2.0 * f) / mag(w, f)
        };
        let (early, late) = (h2(0.02, 0.07), h2(0.5, 0.9));
        assert!(early > 2.5 * late, "h2/h1 early {early:.3} late {late:.3}");
    }

    #[test]
    fn it_falls_away_above_four_kilohertz_like_a_real_one() {
        // A3: the harmonics from 5.5 to 8 kHz far below those from 1 to 3 kHz.
        let p = dry("Melodica");
        let mut st = MelodicaState::new(SR);
        st.note_on(&p, 57, 0.8);
        let l = run(&mut st, &p, 1.0);
        let t = &l[(0.4 * SR) as usize..];
        let f0 = midi_to_freq(57.0) * 2f32.powf(-0.04 / 12.0);
        let band = |lo: f32, hi: f32| {
            let hs: Vec<f32> = (1..60).map(|k| k as f32 * f0).filter(|f| *f >= lo && *f < hi).map(|f| mag(t, f)).collect();
            20.0 * (hs.iter().sum::<f32>() / hs.len() as f32).log10()
        };
        let (mid, top) = (band(1000.0, 3000.0), band(5500.0, 8000.0));
        assert!(top < mid - 15.0, "1-3 kHz {mid:.1} dB, 5.5-8 kHz {top:.1} dB");
    }

    /// MELODICA_WAV=folder writes a few notes of it to listen to and to measure.
    #[test]
    fn notes_to_listen_to() {
        let Ok(dir) = std::env::var("MELODICA_WAV") else { return };
        for (name, note) in [("Melodica", 60u8), ("Melodica", 67), ("Melodica", 72), ("Dub Melodica", 67)] {
            let p = preset(name);
            let mut st = MelodicaState::new(SR);
            st.bpm = 90.0;
            st.note_on(&p, note, 0.8);
            let n = (2.5 * SR) as usize;
            let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
            for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
                st.render(&p, a, b);
            }
            st.note_off(&p, note);
            let (mut l2, mut r2) = (vec![0.0; n], vec![0.0; n]);
            for (a, b) in l2.chunks_mut(256).zip(r2.chunks_mut(256)) {
                st.render(&p, a, b);
            }
            let inter: Vec<f32> = l.iter().chain(&l2).zip(r.iter().chain(&r2)).flat_map(|(a, b)| [*a, *b]).collect();
            let path = std::path::Path::new(&dir).join(format!("{}-{note}.wav", name.replace(' ', "_")));
            crate::render::write_wav(&path, SR as u32, &inter).unwrap();
        }
    }

    #[test]
    fn every_key_is_a_little_off_in_its_own_way() {
        // With Tuning up, a key is off by a few cents; the same key each time.
        let mut p = dry("Melodica");
        p[TUNING] = 1.0;
        let f = |n: u8| MelodicaState::freq(&p, n) / midi_to_freq(n as f32);
        let cents = |r: f32| 1200.0 * r.log2();
        let offs: Vec<f32> = (60..72).map(|n| cents(f(n))).collect();
        assert!(offs.iter().all(|c| c.abs() <= 12.01));
        assert!(offs.iter().any(|c| c.abs() > 2.0), "{offs:?}");
        assert!(offs.windows(2).any(|w| (w[0] - w[1]).abs() > 1.0), "not all the same");
        assert_eq!(f(64), f(64));
    }

    #[test]
    fn a_hard_note_is_tongued_a_soft_one_swells() {
        // The first 15 ms against the steady tone: hard much more of it.
        let start = |vel: f32| {
            let p = dry("Melodica");
            let mut st = MelodicaState::new(SR);
            st.note_on(&p, 64, vel);
            let l = run(&mut st, &p, 0.6);
            rms(&l[..(0.015 * SR) as usize]) / rms(&l[(0.4 * SR) as usize..])
        };
        assert!(start(1.0) > 1.3 * start(0.3), "{} {}", start(1.0), start(0.3));
    }

    #[test]
    fn the_dub_echo_comes_back_on_the_beat() {
        // A short note at 120 BPM; the dotted eighth echo 375 ms later.
        let mut p = dry("Melodica");
        p[ECHO] = 0.6;
        p[ECHO_TIME] = 2.0;
        let mut st = MelodicaState::new(SR);
        st.bpm = 120.0;
        st.note_on(&p, 64, 0.9);
        let a = run(&mut st, &p, 0.06);
        st.note_off(&p, 64);
        let b = run(&mut st, &p, 0.6);
        let l: Vec<f32> = a.into_iter().chain(b).collect();
        let at = |s: f32, e: f32| rms(&l[(s * SR) as usize..(e * SR) as usize]);
        assert!(at(0.37, 0.43) > 4.0 * at(0.22, 0.30), "echo {} gap {}", at(0.37, 0.43), at(0.22, 0.30));
    }
}
