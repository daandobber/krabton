//! Street Organ: a Dutch street organ (draaiorgel) and its fairground
//! cousins. Wind blows through ranks of pipes: stopped wooden ones (the
//! bourdon), open flutes, narrow violin pipes and a second violin rank tuned
//! a little sharp so it beats against the first (the celeste), and a piccolo
//! an octave up. Like the real organ its keys are divided: the low ones play
//! the bass pipes, the middle ones the accompaniment, the rest the melody;
//! and the lowest keys beat the drums the organ carries: the bass drum with
//! its cymbal, the snare (and a roll while held), the woodblock. The
//! tremulant shakes the wind, the wind itself can be heard, pipes speak with
//! a little chiff, and an old organ is a little out of tune.

use super::effects::FxCtx;
use super::fx::hall::{self, Hall};
use super::{db_to_gain, midi_to_freq, OnePole, Rng};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

pub const BOURDON: usize = 0;
pub const FLUTE: usize = 1;
pub const VIOLIN: usize = 2;
pub const CELESTE: usize = 3;
pub const PICCOLO: usize = 4;
pub const BASS: usize = 5;
pub const ACCOMP: usize = 6;
pub const DRUMS: usize = 7;
pub const DIVISIONS: usize = 8;
pub const PRESSURE: usize = 9;
pub const WIND: usize = 10;
pub const CHIFF: usize = 11;
pub const TREMULANT: usize = 12;
pub const TREM_RATE: usize = 13;
pub const BEAT: usize = 14;
pub const TUNING: usize = 15;
pub const ATTACK: usize = 16;
pub const RELEASE: usize = 17;
pub const SPACE: usize = 18;
pub const WIDTH: usize = 19;
pub const VOLUME: usize = 20;
pub const TRANSPOSE: usize = 21;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Bourdon", "Registers", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Flute", "Registers", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Violin", "Registers", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Celeste", "Registers", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Piccolo", "Registers", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Bass", "Divisions", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Accomp", "Divisions", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Drums", "Divisions", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::toggle("Divisions", "Divisions", true),
    ParamSpec::lin("Pressure", "Wind", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Wind", "Wind", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Chiff", "Wind", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Tremulant", "Wind", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Trem Rate", "Wind", 3.0, 9.0, 5.8, Unit::Hz),
    ParamSpec::lin("Beat", "Wind", 0.0, 20.0, 6.0, Unit::Cents),
    ParamSpec::lin("Out of Tune", "Pipes", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::log("Attack", "Pipes", 0.003, 0.2, 0.025, Unit::Sec),
    ParamSpec::log("Release", "Pipes", 0.01, 0.5, 0.06, Unit::Sec),
    ParamSpec::lin("Space", "Pipes", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Width", "Pipes", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Volume", "Pipes", -24.0, 6.0, -3.0, Unit::Db),
    ParamSpec::int("Transpose", "Divisions", -24.0, 24.0, 0.0, Unit::Semi),
];

/// The keys that beat the drums: 24 bass drum and cymbal together, 25 bass
/// drum, 26 snare, 27 snare roll (while held), 28 cymbal, 29 woodblock.
pub const DRUM_LO: u8 = 24;
pub const DRUM_HI: u8 = 29;
/// With the divisions on, keys below these play the bass and the accompaniment.
pub const ACCOMP_FROM: u8 = 48;
pub const MELODY_FROM: u8 = 60;

/// The drum keys, a bit each (the song's key leaves them where they are).
pub const DRUM_KEYS: u128 = {
    let mut m = 0u128;
    let mut n = DRUM_LO;
    while n <= DRUM_HI {
        m |= 1u128 << n;
        n += 1;
    }
    m
};

pub fn preset_names() -> &'static [&'static str] {
    &["Draaiorgel", "Celeste", "Fairground", "Flutes", "Violins", "Old Barrel Organ", "Night Street", "Dance Organ", "Bourdon Bass"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        // The street organ as it stands on the market: violins and the beating celeste over a bourdon.
        "Draaiorgel" => &[],
        // All shimmer: the two violin ranks beating wide, a slow tremulant.
        "Celeste" => &[(BOURDON, 0.4), (FLUTE, 0.2), (VIOLIN, 0.7), (CELESTE, 0.9), (BEAT, 9.0), (TREMULANT, 0.45), (TREM_RATE, 5.0), (SPACE, 0.4)],
        // Loud and bright, the piccolo on top, high pressure, a quick tremulant.
        "Fairground" => &[(BOURDON, 0.6), (FLUTE, 0.5), (VIOLIN, 0.8), (CELESTE, 0.4), (PICCOLO, 0.55), (PRESSURE, 0.75), (CHIFF, 0.5), (TREMULANT, 0.25), (TREM_RATE, 7.0), (SPACE, 0.2), (DRUMS, 1.0)],
        "Flutes" => &[(BOURDON, 0.8), (FLUTE, 0.8), (VIOLIN, 0.0), (CELESTE, 0.0), (PICCOLO, 0.3), (CHIFF, 0.55), (WIND, 0.35), (TREMULANT, 0.4)],
        "Violins" => &[(BOURDON, 0.2), (FLUTE, 0.0), (VIOLIN, 0.9), (CELESTE, 0.7), (PRESSURE, 0.55), (CHIFF, 0.2), (TREMULANT, 0.3)],
        // Leaky bellows, pipes gone out of tune, a wobbly tremulant.
        "Old Barrel Organ" => &[(TUNING, 0.8), (WIND, 0.6), (CHIFF, 0.6), (TREMULANT, 0.6), (TREM_RATE, 4.6), (BEAT, 11.0), (PRESSURE, 0.3), (SPACE, 0.25), (PICCOLO, 0.15)],
        // Far down an empty street at night.
        "Night Street" => &[(PRESSURE, 0.25), (WIND, 0.35), (TREMULANT, 0.5), (SPACE, 0.75), (WIDTH, 0.9), (DRUMS, 0.5), (VOLUME, -5.0)],
        // For dancing: punchy, no tremulant, the drums up.
        "Dance Organ" => &[(BOURDON, 0.7), (FLUTE, 0.4), (VIOLIN, 0.8), (CELESTE, 0.3), (PICCOLO, 0.25), (PRESSURE, 0.6), (TREMULANT, 0.0), (DRUMS, 1.0), (ATTACK, 0.012), (RELEASE, 0.04), (SPACE, 0.15)],
        // Every key on the bass pipes.
        "Bourdon Bass" => &[(DIVISIONS, 0.0), (BOURDON, 1.0), (FLUTE, 0.3), (VIOLIN, 0.0), (CELESTE, 0.0), (TRANSPOSE, -12.0), (TREMULANT, 0.1)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

// ---------------------------------------------------------------------------
// The pipes' waves: one cycle each, band-limited (a table per number of
// harmonics), made once.

const TABLE: usize = 2048;
const LEVELS: usize = 8;
/// Stopped wood, open flute, narrow violin.
const KINDS: usize = 3;
const K_BOURDON: usize = 0;
const K_FLUTE: usize = 1;
const K_VIOLIN: usize = 2;

struct Tables {
    t: Vec<Vec<f32>>,
}

/// How strong harmonic `n` of a kind of pipe is.
fn harmonic(kind: usize, n: usize) -> f32 {
    let nf = n as f32;
    match kind {
        // Stopped: odd harmonics, the fundamental strong.
        K_BOURDON => {
            if n % 2 == 1 {
                if n == 1 {
                    1.0
                } else {
                    0.55 / nf.powf(2.1)
                }
            } else {
                0.015 / nf.powi(2)
            }
        }
        // Open flute: a soft second and third, little above.
        K_FLUTE => match n {
            1 => 1.0,
            2 => 0.3,
            3 => 0.14,
            _ => 0.7 / nf.powf(2.6),
        },
        // Narrow and stringy: many harmonics, a bump around the fourth.
        _ => (1.0 / nf.powf(0.9)) * (0.7 + 0.6 * (-((nf - 4.0).powi(2)) / 10.0).exp()),
    }
}

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = Vec::with_capacity(KINDS * LEVELS);
        for kind in 0..KINDS {
            for level in 0..LEVELS {
                let maxh = (128usize >> level).max(1);
                let mut w = vec![0.0f32; TABLE + 1];
                for n in 1..=maxh {
                    let a = harmonic(kind, n);
                    if a < 1e-5 {
                        continue;
                    }
                    // Phases spread so the sum does not peak.
                    let ph = (n * n) as f32 * 0.37;
                    for (i, s) in w.iter_mut().enumerate().take(TABLE) {
                        *s += a * (TAU * n as f32 * i as f32 / TABLE as f32 + ph).sin();
                    }
                }
                let peak = w.iter().fold(0.0f32, |m, x| m.max(x.abs())).max(1e-6);
                for s in w.iter_mut() {
                    *s /= peak;
                }
                w[TABLE] = w[0];
                t.push(w);
            }
        }
        Tables { t }
    })
}

/// Makes the pipes' waves now (on the interface thread), so the audio
/// thread never has to.
pub fn warm() {
    let _ = tables();
}

impl Tables {
    /// The table of `kind` for a pipe at `freq` (as many harmonics as fit).
    #[inline]
    fn pick(&self, kind: usize, freq: f32, sr: f32) -> &[f32] {
        let fits = 0.45 * sr / freq.max(1.0);
        let mut level = 0;
        while level + 1 < LEVELS && (128usize >> level) as f32 > fits {
            level += 1;
        }
        &self.t[kind * LEVELS + level]
    }
}

#[inline]
fn read(t: &[f32], ph: f32) -> f32 {
    let x = ph * TABLE as f32;
    let i = x as usize;
    let f = x - i as f32;
    let i = i.min(TABLE - 1);
    t[i] + (t[i + 1] - t[i]) * f
}

/// How far a key's pipe is off (-1..1), the same every time.
fn key_error(note: u8) -> f32 {
    let h = (note as u32 ^ 0x5bd1).wrapping_mul(2_654_435_761) >> 16;
    (h & 0xffff) as f32 / 32_767.5 - 1.0
}

// ---------------------------------------------------------------------------

const MAX_PIPES: usize = 24;
const CHUNK: usize = 256;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Division {
    Melody,
    Accomp,
    Bass,
}

#[derive(Clone, Copy)]
struct Pipe {
    note: u8,
    active: bool,
    gate: bool,
    div: Division,
    freq: f32,
    /// Bourdon, flute, violin, celeste, piccolo, the bass's sixteen-foot.
    ph: [f32; 6],
    env: f32,
    vel: f32,
    chiff: f32,
    low: f32,
    band: f32,
    pan: f32,
    age: u32,
}

impl Default for Pipe {
    fn default() -> Self {
        Self { note: 0, active: false, gate: false, div: Division::Melody, freq: 440.0, ph: [0.0; 6], env: 0.0, vel: 0.0, chiff: 0.0, low: 0.0, band: 0.0, pan: 0.0, age: 0 }
    }
}

/// The organ's drums.
#[derive(Default)]
struct Drums {
    kick: f32,
    kick_ph: f32,
    kick_f: f32,
    cym: f32,
    cym_ph: [f32; 6],
    snare: f32,
    snare_tone: f32,
    snare_ph: f32,
    roll: bool,
    roll_t: f32,
    wood: f32,
    wood_ph: [f32; 2],
    hp: [OnePole; 3],
    sn_lp: OnePole,
    sn_hp: OnePole,
}

pub struct StreetOrganState {
    pub pitch_bend: f32,
    sr: f32,
    pipes: Vec<Pipe>,
    drums: Drums,
    rng: Rng,
    trem_ph: f32,
    /// The wind's slow unsteadiness.
    wander: f32,
    wander_lp: OnePole,
    wind_lp: OnePole,
    wind_hp: OnePole,
    clock: u32,
    dl: Vec<f32>,
    dr: Vec<f32>,
    wl: Vec<f32>,
    wr: Vec<f32>,
    hall: Box<Hall>,
    hall_p: Vec<f32>,
}

impl StreetOrganState {
    pub fn new(sr: f32) -> Self {
        warm();
        Self {
            pitch_bend: 0.0,
            sr: sr.max(1000.0),
            pipes: vec![Pipe::default(); MAX_PIPES],
            drums: Drums { kick_f: 50.0, ..Default::default() },
            rng: Rng::new(0x0D2A_A10E),
            trem_ph: 0.0,
            wander: 0.0,
            wander_lp: OnePole::default(),
            wind_lp: OnePole::default(),
            wind_hp: OnePole::default(),
            clock: 0,
            dl: vec![0.0; CHUNK],
            dr: vec![0.0; CHUNK],
            wl: vec![0.0; CHUNK],
            wr: vec![0.0; CHUNK],
            hall: Box::new(Hall::new(sr)),
            hall_p: crate::params::defaults(hall::SPECS),
        }
    }

    fn division(p: &[f32], note: u8) -> Division {
        if p[DIVISIONS] < 0.5 {
            Division::Melody
        } else if note < ACCOMP_FROM {
            Division::Bass
        } else if note < MELODY_FROM {
            Division::Accomp
        } else {
            Division::Melody
        }
    }

    fn drum_on(&mut self, note: u8, vel: f32) {
        let d = &mut self.drums;
        let v = 0.35 + 0.65 * vel.clamp(0.0, 1.0);
        match note {
            24 => {
                d.kick = v;
                d.kick_f = 120.0;
                d.cym = v * 0.8;
            }
            25 => {
                d.kick = v;
                d.kick_f = 120.0;
            }
            26 => {
                d.snare = v;
                d.snare_tone = v;
            }
            27 => {
                d.roll = true;
                d.roll_t = 0.0;
            }
            28 => d.cym = v,
            _ => {
                d.wood = v;
                d.wood_ph = [0.0; 2];
            }
        }
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        if (DRUM_LO..=DRUM_HI).contains(&note) {
            self.drum_on(note, vel);
            return;
        }
        let key = (note as i32 + p[TRANSPOSE].round() as i32).clamp(0, 127) as u8;
        let div = Self::division(p, key);
        // A pipe sounding already speaks again; else a free one, else the oldest.
        let i = self
            .pipes
            .iter()
            .position(|v| v.active && v.note == note)
            .or_else(|| self.pipes.iter().position(|v| !v.active))
            .unwrap_or_else(|| self.pipes.iter().enumerate().max_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0));
        let cents = p[TUNING].clamp(0.0, 1.0) * 14.0 * key_error(key);
        let width = p[WIDTH].clamp(0.0, 1.0);
        let side = if key % 2 == 0 { -1.0 } else { 1.0 };
        let pan = match div {
            Division::Bass => 0.0,
            Division::Accomp => side * 0.25 * width,
            Division::Melody => side * (0.2 + 0.4 * ((key as f32 - 60.0) / 36.0).clamp(0.0, 1.0)) * width,
        };
        let ph = [self.rng.unipolar(), self.rng.unipolar(), self.rng.unipolar(), self.rng.unipolar(), self.rng.unipolar(), self.rng.unipolar()];
        let v = &mut self.pipes[i];
        let again = v.active && v.note == note;
        *v = Pipe {
            note,
            active: true,
            gate: true,
            div,
            freq: midi_to_freq(key as f32 + cents / 100.0),
            ph: if again { v.ph } else { ph },
            env: if again { v.env } else { 0.0 },
            vel: 0.55 + 0.45 * vel.clamp(0.0, 1.0),
            chiff: 1.0,
            low: 0.0,
            band: 0.0,
            pan,
            age: 0,
        };
        for o in self.pipes.iter_mut() {
            o.age = o.age.saturating_add(1);
        }
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        if note == 27 {
            self.drums.roll = false;
            return;
        }
        for v in self.pipes.iter_mut().filter(|v| v.active && v.note == note) {
            v.gate = false;
        }
    }

    pub fn all_off(&mut self) {
        for v in self.pipes.iter_mut() {
            v.gate = false;
        }
        self.drums.roll = false;
    }

    /// The screen: [0] on, [1] the tremulant now (-1..1), [2..6] the bass
    /// drum, snare, cymbal and woodblock, then from [8] per pipe its note
    /// and how loud it is.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = 1.0;
        out[1] = (self.trem_ph * TAU).sin();
        out[2] = self.drums.kick;
        out[3] = self.drums.snare.max(if self.drums.roll { 0.6 } else { 0.0 });
        out[4] = self.drums.cym;
        out[5] = self.drums.wood;
        for (i, v) in self.pipes.iter().enumerate().take((crate::engine::INST_VALS - 8) / 2) {
            out[8 + 2 * i] = v.note as f32;
            out[9 + 2 * i] = if v.active { v.env * v.vel } else { 0.0 };
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let tables = tables();
        let sr = self.sr;
        let k = |i: usize| p[i].clamp(0.0, 1.0);
        let (bourdon, flute, violin, celeste, piccolo) = (k(BOURDON), k(FLUTE), k(VIOLIN), k(CELESTE), k(PICCOLO));
        let (bass, accomp, drums) = (k(BASS), k(ACCOMP), k(DRUMS));
        let pressure = k(PRESSURE);
        let wind = k(WIND);
        let chiff_amt = k(CHIFF);
        let trem = k(TREMULANT);
        let trem_rate = p[TREM_RATE].clamp(1.0, 12.0);
        let beat = 2f32.powf(p[BEAT].clamp(0.0, 40.0) / 1200.0);
        let attack = (-1.0 / (p[ATTACK].clamp(0.001, 1.0) * sr / 3.0)).exp();
        let release = (-1.0 / (p[RELEASE].clamp(0.005, 2.0) * sr / 4.6)).exp();
        let chiff_fall = (-1.0 / (0.03 * sr)).exp();
        let space = k(SPACE);
        let gain = db_to_gain(p[VOLUME]);
        let bend = 2f32.powf(self.pitch_bend / 12.0);
        let drive = 1.0 + 2.5 * pressure;
        let norm = 1.0 / drive.tanh();
        self.wander_lp.set(2.0, sr);
        self.wind_lp.set(2400.0, sr);
        self.wind_hp.set(300.0, sr);
        for h in self.drums.hp.iter_mut() {
            h.set(5000.0, sr);
        }
        self.drums.sn_lp.set(6000.0, sr);
        self.drums.sn_hp.set(800.0, sr);
        let mut done = 0;
        let n = l.len().min(r.len());
        while done < n {
            let len = (n - done).min(CHUNK);
            let active = self.pipes.iter().filter(|v| v.active).count() as f32;
            for j in 0..len {
                // The wind: the tremulant shaking it, and its own unsteadiness.
                self.trem_ph = (self.trem_ph + trem_rate / sr).fract();
                let tw = (self.trem_ph * TAU).sin();
                let w = self.wander_lp.lp(self.rng.bipolar());
                self.wander = w;
                let amp = (1.0 + trem * 0.22 * tw) * (1.0 + wind * 1.5 * w);
                let pitch = bend * 2f32.powf(trem * 7.0 * tw / 1200.0);
                let (mut sl, mut sr_) = (0.0f32, 0.0f32);
                for v in self.pipes.iter_mut().filter(|v| v.active) {
                    v.env = if v.gate { 1.0 - (1.0 - v.env) * attack } else { v.env * release };
                    if !v.gate && v.env < 1e-4 {
                        v.active = false;
                        continue;
                    }
                    let f = v.freq * pitch;
                    let mut s = 0.0;
                    let mut step = |idx: usize, kind: usize, fr: f32, lvl: f32| -> f32 {
                        let t = tables.pick(kind, fr, sr);
                        let o = read(t, v.ph[idx]) * lvl;
                        v.ph[idx] = (v.ph[idx] + fr / sr).fract();
                        o
                    };
                    match v.div {
                        Division::Melody => {
                            if bourdon > 0.0 {
                                s += step(0, K_BOURDON, f, bourdon);
                            }
                            if flute > 0.0 {
                                s += step(1, K_FLUTE, f, flute * 0.8);
                            }
                            if violin > 0.0 {
                                s += step(2, K_VIOLIN, f, violin * 0.6);
                            }
                            if celeste > 0.0 {
                                s += step(3, K_VIOLIN, f * beat, celeste * 0.6);
                            }
                            if piccolo > 0.0 {
                                s += step(4, K_FLUTE, f * 2.0, piccolo * 0.5);
                            }
                            s *= 0.4;
                        }
                        Division::Accomp => {
                            s = (step(0, K_BOURDON, f, 0.6) + step(2, K_VIOLIN, f, 0.4) + step(3, K_VIOLIN, f * beat, 0.25)) * accomp * 0.38;
                        }
                        Division::Bass => {
                            s = (step(0, K_BOURDON, f, 1.0) + step(5, K_BOURDON, f * 0.5, 0.7) + step(1, K_FLUTE, f, 0.3)) * bass * 0.42;
                        }
                    }
                    // The pipe's speech: a breathy chiff around its third harmonic.
                    if chiff_amt > 0.0 && v.chiff > 0.001 {
                        let fc = (3.0 * f).min(0.4 * sr);
                        let g = 2.0 * (PI * fc / sr).sin();
                        let x = self.rng.bipolar() * v.chiff;
                        let high = x - v.low - 0.3 * v.band;
                        v.band += g * high;
                        v.low += g * v.band;
                        s += v.band * chiff_amt * 0.25;
                        v.chiff *= chiff_fall;
                    }
                    s *= v.env * v.vel;
                    sl += s * (1.0 - v.pan).min(1.0);
                    sr_ += s * (1.0 + v.pan).min(1.0);
                }
                // The wind heard through the pipes.
                if wind > 0.0 && active > 0.0 {
                    let nz = self.wind_hp.hp(self.wind_lp.lp(self.rng.bipolar()));
                    let a = nz * wind * 0.03 * (0.3 + 0.1 * active).min(1.0);
                    sl += a;
                    sr_ += a;
                }
                let (dl, dr) = self.drum(drums);
                let ol = (sl * amp * drive).tanh() * norm + dl;
                let or = (sr_ * amp * drive).tanh() * norm + dr;
                self.dl[j] = ol * gain;
                self.dr[j] = or * gain;
            }
            // The street around it.
            if space > 0.001 {
                self.wl[..len].copy_from_slice(&self.dl[..len]);
                self.wr[..len].copy_from_slice(&self.dr[..len]);
                let hp = &mut self.hall_p;
                hp[hall::PREDELAY] = 15.0;
                hp[hall::SIZE] = 0.3 + 0.5 * space;
                hp[hall::DECAY] = 0.8 + 2.4 * space;
                hp[hall::LOW_DECAY] = 0.8;
                hp[hall::DAMPING] = 6500.0;
                hp[hall::DIFFUSION] = 0.75;
                hp[hall::MODULATION] = 0.15;
                hp[hall::FREEZE] = 0.0;
                hp[hall::WIDTH] = 1.0;
                hp[hall::MIX] = 1.0;
                let ctx = FxCtx { sr, bpm: 120.0, beat: 0.0, playing: false };
                self.hall.process(&self.hall_p, &mut self.wl[..len], &mut self.wr[..len], &ctx, None);
                for j in 0..len {
                    self.dl[j] += self.wl[j] * space * 0.6;
                    self.dr[j] += self.wr[j] * space * 0.6;
                }
            }
            for j in 0..len {
                l[done + j] += self.dl[j].clamp(-1.5, 1.5);
                r[done + j] += self.dr[j].clamp(-1.5, 1.5);
            }
            done += len;
            self.clock = self.clock.wrapping_add(len as u32);
        }
    }

    /// One sample of the drums (left, right).
    fn drum(&mut self, level: f32) -> (f32, f32) {
        let sr = self.sr;
        let d = &mut self.drums;
        if level <= 0.0 && !d.roll {
            return (0.0, 0.0);
        }
        // The snare roll: strokes sixteen to the second, never quite the same.
        if d.roll {
            d.roll_t -= 1.0 / sr;
            if d.roll_t <= 0.0 {
                d.roll_t = 1.0 / 17.0 * (0.9 + 0.2 * self.rng.unipolar());
                d.snare = d.snare.max(0.45 + 0.2 * self.rng.unipolar());
                d.snare_tone = d.snare_tone.max(0.3);
            }
        }
        let mut out_l = 0.0;
        let mut out_r = 0.0;
        if d.kick > 1e-4 {
            d.kick_f = 48.0 + (d.kick_f - 48.0) * (-1.0 / (0.04 * sr)).exp();
            d.kick_ph = (d.kick_ph + d.kick_f / sr).fract();
            let s = (d.kick_ph * TAU).sin() * d.kick * 0.9;
            d.kick *= (-4.6 / (0.38 * sr)).exp();
            out_l += s;
            out_r += s;
        }
        if d.cym > 1e-4 {
            // Noise and a few clanging squares, high-passed.
            let mut m = 0.0;
            for (i, ph) in d.cym_ph.iter_mut().enumerate() {
                let f = [3150.0, 4410.0, 5170.0, 6230.0, 7340.0, 8180.0][i];
                *ph = (*ph + f / sr).fract();
                m += if *ph < 0.5 { 1.0 } else { -1.0 };
            }
            let x = self.rng.bipolar() * 0.7 + m * 0.08;
            let mut y = x;
            for h in d.hp.iter_mut() {
                y = h.hp(y);
            }
            let s = y * d.cym * 0.55;
            d.cym *= (-4.6 / (0.9 * sr)).exp();
            out_l += s * 0.8;
            out_r += s;
        }
        if d.snare > 1e-4 || d.snare_tone > 1e-4 {
            let nz = d.sn_hp.hp(d.sn_lp.lp(self.rng.bipolar()));
            d.snare_ph = (d.snare_ph + 185.0 / sr).fract();
            let s = nz * d.snare * 0.6 + (d.snare_ph * TAU).sin() * d.snare_tone * 0.3;
            d.snare *= (-4.6 / (if d.roll { 0.09 } else { 0.16 } * sr)).exp();
            d.snare_tone *= (-4.6 / (0.07 * sr)).exp();
            out_l += s;
            out_r += s * 0.8;
        }
        if d.wood > 1e-4 {
            d.wood_ph[0] = (d.wood_ph[0] + 980.0 / sr).fract();
            d.wood_ph[1] = (d.wood_ph[1] + 1450.0 / sr).fract();
            let s = ((d.wood_ph[0] * TAU).sin() + 0.5 * (d.wood_ph[1] * TAU).sin()) * d.wood * 0.35;
            d.wood *= (-4.6 / (0.05 * sr)).exp();
            out_l += s * 0.7;
            out_r += s;
        }
        (out_l * level, out_r * level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn run(st: &mut StreetOrganState, p: &[f32], secs: f32) -> Vec<f32> {
        let n = (secs * SR) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            st.render(p, a, b);
        }
        l
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn mag(x: &[f32], hz: f32) -> f32 {
        let (mut re, mut im) = (0.0f32, 0.0f32);
        for (i, v) in x.iter().enumerate() {
            let a = TAU * hz * i as f32 / SR;
            re += v * a.cos();
            im += v * a.sin();
        }
        (re * re + im * im).sqrt() / x.len() as f32
    }

    #[test]
    fn every_preset_plays_its_pipes_and_stops() {
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let mut st = StreetOrganState::new(SR);
            st.note_on(&p, 69, 0.8);
            let on = run(&mut st, &p, 0.4);
            let level = rms(&on[9600..]);
            assert!(level > 0.01 && on.iter().all(|x| x.is_finite() && x.abs() <= 1.5), "{name}: {level}");
            st.note_off(&p, 69);
            run(&mut st, &p, 2.5);
            assert!(rms(&run(&mut st, &p, 0.2)) < level * 0.05, "{name} rings on");
        }
    }

    #[test]
    fn a_pipe_sounds_its_pitch() {
        let mut p = preset("Flutes");
        p[TUNING] = 0.0;
        p[TREMULANT] = 0.0;
        p[WIND] = 0.0;
        p[CHIFF] = 0.0;
        p[SPACE] = 0.0;
        let mut st = StreetOrganState::new(SR);
        st.note_on(&p, 69, 1.0);
        let x = run(&mut st, &p, 0.5);
        let tail = &x[12_000..];
        assert!(mag(tail, 440.0) > 4.0 * mag(tail, 466.16), "A4 is A4");
    }

    #[test]
    fn the_low_keys_beat_the_drums_and_divide_the_rest() {
        let p = preset("Draaiorgel");
        assert_eq!(StreetOrganState::division(&p, 40), Division::Bass);
        assert_eq!(StreetOrganState::division(&p, 52), Division::Accomp);
        assert_eq!(StreetOrganState::division(&p, 72), Division::Melody);
        for key in DRUM_LO..=DRUM_HI {
            let mut st = StreetOrganState::new(SR);
            st.note_on(&p, key, 1.0);
            assert!(st.pipes.iter().all(|v| !v.active), "key {key} is a drum, not a pipe");
            let x = run(&mut st, &p, 0.1);
            assert!(rms(&x) > 0.005, "drum key {key} is heard");
            st.note_off(&p, key);
        }
        assert_eq!(DRUM_KEYS.count_ones(), 6);
    }
}
