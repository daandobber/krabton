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
//! shakuhachi.
//!
//! One voice plays legato with glide; more play chords. Original code.

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

pub const MODELS: &[&str] = &["Pan Flute", "Shakuhachi", "Ocarina", "Bamboo Flute", "Wooden Flute"];

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
];

pub fn preset_names() -> &'static [&'static str] {
    &["Andes Pan Flute", "Romanian Pan Flute", "Zen Shakuhachi", "Shakuhachi Breath", "Ocarina", "Bansuri Evening", "Canyon Flute", "Bamboo Grove", "Breathy Pad"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Andes Pan Flute" => &[(BREATH, 0.35), (CHIFF, 0.7), (TONE, 0.45), (VIBRATO, 0.2), (VIB_DELAY, 0.3), (BEND, 0.2), (ATTACK, 0.04), (RELEASE, 0.2), (GLIDE, 0.03), (SPACE, 0.3)],
        "Romanian Pan Flute" => &[(BREATH, 0.35), (CHIFF, 0.55), (TONE, 0.55), (VIBRATO, 0.6), (VIB_RATE, 5.8), (VIB_DELAY, 0.25), (BEND, 0.35), (ATTACK, 0.05), (RELEASE, 0.4), (GLIDE, 0.08), (SPACE, 0.55)],
        "Zen Shakuhachi" => &[(MODEL, 1.0), (BREATH, 0.45), (CHIFF, 0.5), (TONE, 0.5), (VIBRATO, 0.3), (VIB_RATE, 4.5), (VIB_DELAY, 0.5), (BEND, 0.6), (ATTACK, 0.12), (RELEASE, 0.5), (GLIDE, 0.12), (SPACE, 0.6)],
        "Shakuhachi Breath" => &[(MODEL, 1.0), (BREATH, 0.7), (CHIFF, 0.9), (TONE, 0.75), (VIBRATO, 0.15), (BEND, 0.8), (ATTACK, 0.03), (RELEASE, 0.35), (SPACE, 0.45)],
        "Ocarina" => &[(MODEL, 2.0), (BREATH, 0.2), (CHIFF, 0.3), (TONE, 0.4), (VIBRATO, 0.25), (VIB_RATE, 5.0), (VIB_DELAY, 0.4), (BEND, 0.1), (ATTACK, 0.03), (RELEASE, 0.12), (SPACE, 0.3)],
        "Bansuri Evening" => &[(MODEL, 3.0), (BREATH, 0.4), (CHIFF, 0.45), (TONE, 0.5), (VIBRATO, 0.35), (VIB_RATE, 5.3), (BEND, 0.4), (ATTACK, 0.08), (RELEASE, 0.35), (GLIDE, 0.1), (SPACE, 0.5)],
        "Canyon Flute" => &[(MODEL, 4.0), (BREATH, 0.35), (CHIFF, 0.6), (TONE, 0.55), (VIBRATO, 0.45), (VIB_RATE, 5.5), (VIB_DELAY, 0.3), (BEND, 0.3), (RELEASE, 0.6), (SPACE, 0.85)],
        "Bamboo Grove" => &[(MODEL, 3.0), (VOICES, 4.0), (BREATH, 0.45), (CHIFF, 0.35), (TONE, 0.4), (VIBRATO, 0.25), (ATTACK, 0.15), (RELEASE, 0.9), (SPREAD, 0.7), (SPACE, 0.6)],
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
}

const MODEL_DATA: [Model; 5] = [
    Model { stopped: true, harm: [1.0, 0.0, 0.18, 0.0, 0.06, 0.0], breath: 0.11, ring: 0.9, chiff: 0.8, flash: 3, vib_amp: 0.7, scoop: 0.6, scoop_t: 0.06, buzz: 0.0 },
    Model { stopped: false, harm: [1.0, 0.35, 0.18, 0.1, 0.05, 0.03], breath: 0.18, ring: 0.85, chiff: 0.6, flash: 2, vib_amp: 0.3, scoop: 1.2, scoop_t: 0.2, buzz: 0.0 },
    Model { stopped: true, harm: [1.0, 0.05, 0.03, 0.01, 0.0, 0.0], breath: 0.04, ring: 0.6, chiff: 0.25, flash: 2, vib_amp: 0.5, scoop: 0.3, scoop_t: 0.05, buzz: 0.0 },
    Model { stopped: false, harm: [1.0, 0.25, 0.12, 0.05, 0.02, 0.01], breath: 0.13, ring: 0.9, chiff: 0.4, flash: 2, vib_amp: 0.5, scoop: 0.8, scoop_t: 0.12, buzz: 0.0 },
    Model { stopped: false, harm: [1.0, 0.3, 0.22, 0.08, 0.05, 0.02], breath: 0.09, ring: 0.92, chiff: 0.5, flash: 2, vib_amp: 0.6, scoop: 0.5, scoop_t: 0.08, buzz: 0.25 },
];

const MAX_VOICES: usize = 8;
const LINE: usize = 4096;

struct Voice {
    note: u8,
    gate: bool,
    f: f32,
    target: f32,
    phase: f32,
    vib_ph: f32,
    env: f32,
    t: f32,
    vel: f32,
    line: Vec<f32>,
    w: usize,
    loop_lp: f32,
    hp: OnePole,
    lp: OnePole,
    lp2: OnePole,
    drift: f32,
    pan: f32,
    level: f32,
    /// Slurred in (no fresh chiff).
    slur: bool,
}

impl Voice {
    fn new() -> Self {
        Self { note: 60, gate: false, f: 261.6, target: 261.6, phase: 0.0, vib_ph: 0.0, env: 0.0, t: 0.0, vel: 0.0, line: vec![0.0; LINE], w: 0, loop_lp: 0.0, hp: OnePole::default(), lp: OnePole::default(), lp2: OnePole::default(), drift: 0.0, pan: 0.0, level: 0.0, slur: false }
    }

    fn quiet(&self) -> bool {
        !self.gate && self.env < 1e-4
    }
}

pub struct PanFluteState {
    /// Pitch bend in semitones (a wheel, a controller's triggers), set by the engine.
    pub pitch_bend: f32,
    sr: f32,
    voices: Vec<Voice>,
    /// Keys held, in order (one voice plays the last).
    stack: Vec<u8>,
    rng: Rng,
    hall: Box<Hall>,
    hall_p: Vec<f32>,
    dl: Vec<f32>,
    dr: Vec<f32>,
    wl: Vec<f32>,
    wr: Vec<f32>,
}

const CHUNK: usize = 64;

impl PanFluteState {
    pub fn new(sr: f32) -> Self {
        Self {
            pitch_bend: 0.0,
            sr: sr.max(1000.0),
            voices: (0..MAX_VOICES).map(|_| Voice::new()).collect(),
            stack: Vec::with_capacity(32),
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

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let poly = (p[VOICES].round() as usize).clamp(1, MAX_VOICES);
        let f = Self::freq(p, note);
        if self.stack.len() < 32 {
            self.stack.retain(|&n| n != note);
            self.stack.push(note);
        }
        if poly == 1 {
            let v = &mut self.voices[0];
            let legato = v.gate;
            if !legato || p[GLIDE] <= 0.001 {
                v.f = f;
            }
            if !legato {
                v.t = 0.0;
                v.vel = vel;
            }
            v.slur = legato;
            v.target = f;
            v.note = note;
            v.gate = true;
            v.pan = Self::pan_of(p, note);
            return;
        }
        let i = (0..poly)
            .find(|&i| self.voices[i].note == note && !self.voices[i].quiet())
            .or_else(|| (0..poly).find(|&i| self.voices[i].quiet()))
            .unwrap_or_else(|| (0..poly).min_by(|&a, &b| self.voices[a].env.total_cmp(&self.voices[b].env)).unwrap_or(0));
        let v = &mut self.voices[i];
        v.f = f;
        v.target = f;
        v.t = 0.0;
        v.vel = vel;
        v.note = note;
        v.gate = true;
        v.slur = false;
        v.pan = Self::pan_of(p, note);
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.stack.retain(|&n| n != note);
        if p.len() < SPECS.len() {
            return;
        }
        let poly = (p[VOICES].round() as usize).clamp(1, MAX_VOICES);
        if poly == 1 {
            let v = &mut self.voices[0];
            if v.note == note {
                // Back to the key still held, slurred.
                if let Some(&prev) = self.stack.last() {
                    v.note = prev;
                    v.target = Self::freq(p, prev);
                    v.slur = true;
                    if p[GLIDE] <= 0.001 {
                        v.f = v.target;
                    }
                } else {
                    v.gate = false;
                }
            }
            return;
        }
        for v in self.voices.iter_mut().filter(|v| v.note == note) {
            v.gate = false;
        }
    }

    pub fn all_off(&mut self) {
        self.stack.clear();
        for v in self.voices.iter_mut() {
            v.gate = false;
        }
    }

    /// The screen: [0] the flute, [1] the breath, then per voice (2 + 2i)
    /// its note and how loud it is.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = 1.0;
        let mut breath = 0.0f32;
        for (i, v) in self.voices.iter().enumerate() {
            out[2 + 2 * i] = v.note as f32;
            out[3 + 2 * i] = v.level;
            breath = breath.max(v.env);
        }
        out[1] = breath;
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let pitch_bend = self.pitch_bend;
        if p.len() < SPECS.len() {
            return;
        }
        let sr = self.sr;
        let m = &MODEL_DATA[(p[MODEL].round().max(0.0) as usize).min(MODELS.len() - 1)];
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
        let n = l.len().min(r.len());
        let lim = 0.45 * sr;
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            self.dl[..len].fill(0.0);
            self.dr[..len].fill(0.0);
            for v in self.voices.iter_mut() {
                if v.quiet() {
                    v.level *= 0.9;
                    continue;
                }
                v.hp.set(250.0, sr);
                v.lp.set(2800.0, sr);
                v.lp2.set(4000.0, sr);
                let (pl, pr) = pan_gains(v.pan);
                // Brighter when blown harder, darker for a soft tone.
                let bright = 0.3 + 1.4 * tone;
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
                    let semis = scoop + vib * (1.0 - m.vib_amp) * 0.4 + v.drift * 0.04 + pitch_bend;
                    let f = v.f * (semis * (std::f32::consts::LN_2 / 12.0)).exp();
                    let press = v.env * (0.55 + 0.45 * v.vel) * (1.0 + vib * m.vib_amp * 0.35);
                    v.phase = (v.phase + f / sr).fract();
                    // The harmonics, by recurrence from one sine and cosine.
                    let (s1, c1) = (TAU * v.phase).sin_cos();
                    let (mut sk, mut ck) = (s1, c1);
                    let b = bright * (0.6 + 0.4 * press);
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
                    // The chiff: a puff and an overblown flash at the start.
                    let fresh = if v.slur { 0.25 } else { 1.0 };
                    let ch = chiff * m.chiff * fresh * (0.4 + 0.6 * v.vel) * (-v.t / 0.035).exp();
                    y += ch * 0.6 * flash_s;
                    if m.buzz > 0.0 {
                        y += m.buzz * 0.4 * (y * y.abs() - 0.15);
                    }
                    // Breath: noise from the jet, rippling with the tone, then
                    // ringing in the pipe.
                    let nz = v.lp2.lp(v.lp.lp(v.hp.hp(self.rng.bipolar()))) * (1.0 + 0.6 * s1);
                    let amount = breath * 2.0 * m.breath * (0.4 + 0.6 * press) + ch * 0.8;
                    let delay = if m.stopped { sr / (2.0 * f) } else { sr / f };
                    let d = delay.clamp(2.0, (LINE - 2) as f32);
                    let rp = v.w as f32 + LINE as f32 - d;
                    let i0 = rp as usize;
                    let fr = rp - i0 as f32;
                    let a0 = v.line[i0 % LINE];
                    let a1 = v.line[(i0 + 1) % LINE];
                    let back = a0 + (a1 - a0) * fr;
                    v.loop_lp += 0.35 * (back - v.loop_lp);
                    let fb = if m.stopped { -m.ring } else { m.ring } * v.loop_lp;
                    let x = nz * amount + fb;
                    v.line[v.w] = if x.is_finite() { x } else { 0.0 };
                    v.w = (v.w + 1) % LINE;
                    // Mostly what rings in the pipe (on its harmonics), only a
                    // little of the plain hiss.
                    let breath_out = (fb * (1.0 - m.ring) * 2.6 + nz * amount * 0.12) * 1.4;
                    let out = (press * y * 0.5 + breath_out * v.env.max(ch)) * gain;
                    peak = peak.max(out.abs());
                    self.dl[j] += out * pl;
                    self.dr[j] += out * pr;
                }
                v.level = (v.level * 0.85).max(peak * 6.0).min(1.0);
                if !v.gate && v.env < 1e-4 {
                    v.line.fill(0.0);
                    v.loop_lp = 0.0;
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

    fn steady(name: &str, model: f32) -> Vec<f32> {
        let mut p = preset(name);
        p[MODEL] = model;
        p[VIBRATO] = 0.0;
        p[SPACE] = 0.0;
        p[BEND] = 0.0;
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
        let mut p = preset("Andes Pan Flute");
        p[GLIDE] = 0.0;
        p[VIBRATO] = 0.0;
        p[SPACE] = 0.0;
        p[BEND] = 0.0;
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
        let mut p = preset("Breathy Pad");
        p[SPACE] = 0.0;
        p[VIBRATO] = 0.0;
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
}
