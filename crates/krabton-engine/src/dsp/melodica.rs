//! Melodica: a keyboard you blow into. Every key opens a free reed (see
//! `free_reed`); the plastic body and the mouthpiece colour them (Body).
//! Played with the mouth (Playing: Mouth) all the reeds share one breath:
//! a chord divides the air between its notes, a key added dips it for a
//! moment, and a long phrase leaves the lungs a little empty (the tone
//! thins, the breath shows) until the player rests. Free gives every key
//! its own air, like a harmonium's bellows. Blown through the tube (Tube)
//! it starts softer and sounds warmer. A second reed a few cents off makes
//! it beat (an accordion's musette); the breath's vibrato is mostly in the
//! loudness. Original code.

use super::effects::FxCtx;
use super::free_reed::{free_reed, Blow, Reed};
use super::fx::hall::{self, Hall};
use super::{db_to_gain, midi_to_freq, pan_gains, Rng, Svf};
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

pub const PLAYINGS: &[&str] = &["Free", "Mouth"];

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
];

pub fn preset_names() -> &'static [&'static str] {
    &["Melodica", "Dub Melodica", "Bright Melodica", "Tube Melodica", "Lullaby", "Bass Melodica", "Accordion", "Harmonium"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Melodica" => &[(SPACE, 0.25)],
        "Dub Melodica" => &[(VIBRATO, 0.35), (VIB_RATE, 5.4), (BEAT, 0.12), (BODY, 0.8), (TONE, 0.45), (TUBE, 0.3), (RELEASE, 0.3), (SPACE, 0.6)],
        "Bright Melodica" => &[(TONE, 0.85), (BREATH, 0.15), (BODY, 0.5), (SPACE, 0.2)],
        "Tube Melodica" => &[(TUBE, 0.85), (TONE, 0.35), (BREATH, 0.35), (ATTACK, 0.06), (RELEASE, 0.2), (SPACE, 0.3)],
        "Lullaby" => &[(VIBRATO, 0.25), (TONE, 0.35), (BREATH, 0.3), (ATTACK, 0.08), (RELEASE, 0.4), (SPACE, 0.55)],
        "Bass Melodica" => &[(TRANSPOSE, -12.0), (TONE, 0.4), (BODY, 0.7), (VOICES, 4.0), (BREATH, 0.3), (SPACE, 0.2)],
        "Accordion" => &[(PLAYING, 0.0), (BEAT, 0.55), (BODY, 0.4), (VIBRATO, 0.0), (TONE, 0.65), (ATTACK, 0.02), (RELEASE, 0.08), (SPACE, 0.2)],
        "Harmonium" => &[(PLAYING, 0.0), (BEAT, 0.08), (BODY, 0.3), (TONE, 0.4), (BREATH, 0.15), (VIBRATO, 0.05), (ATTACK, 0.15), (RELEASE, 0.5), (SPACE, 0.5)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

const MAX_VOICES: usize = 8;
const CHUNK: usize = 64;

pub struct MelodicaState {
    /// Pitch bend in semitones, set by the engine.
    pub pitch_bend: f32,
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
    tmp: Vec<f32>,
    dl: Vec<f32>,
    dr: Vec<f32>,
    wl: Vec<f32>,
    wr: Vec<f32>,
    body1: [Svf; 2],
    body2: [Svf; 2],
    hall: Box<Hall>,
    hall_p: Vec<f32>,
}

impl MelodicaState {
    pub fn new(sr: f32) -> Self {
        Self {
            pitch_bend: 0.0,
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
            tmp: vec![0.0; CHUNK],
            dl: vec![0.0; CHUNK],
            dr: vec![0.0; CHUNK],
            wl: vec![0.0; CHUNK],
            wr: vec![0.0; CHUNK],
            body1: [Svf::new(), Svf::new()],
            body2: [Svf::new(), Svf::new()],
            hall: Box::new(Hall::new(sr)),
            hall_p: crate::params::defaults(hall::SPECS),
        }
    }

    fn freq(p: &[f32], note: u8) -> f32 {
        midi_to_freq(note as f32 + p[TRANSPOSE].round())
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
            }
            return;
        }
        let i = (0..poly).find(|&i| self.voices[i].owner == note && self.voices[i].gate).unwrap_or_else(|| free_reed(&self.voices[..poly]));
        self.voices[i].start(note, note, f, vel, 1.0, pan);
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.stack.retain(|&n| n != note);
        if p.len() < SPECS.len() {
            return;
        }
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
        let body = k(BODY);
        let space = k(SPACE);
        let gain = db_to_gain(p[VOLUME]) * 0.17;
        for c in 0..2 {
            self.body1[c].set_q(1150.0 * (1.0 - 0.2 * tube), 2.5, sr);
            self.body2[c].set_q(2700.0 * (1.0 - 0.15 * tube), 3.0, sr);
        }
        let n = l.len().min(r.len());
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            for j in 0..len {
                self.vib_ph = (self.vib_ph + rate / sr).fract();
                self.vib[j] = (TAU * self.vib_ph).sin() * depth;
            }
            // One breath for all the keys (Mouth), its own for each (Free).
            let dt = len as f32 / sr;
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
            let share = if mouth { (1.0 / open.max(1) as f32).sqrt() * (1.0 - self.dip) * (1.0 - 0.3 * tired) } else { 1.0 };
            self.dip *= (-dt / 0.06).exp();
            let blow = Blow {
                sr,
                att,
                rel,
                glide_k,
                bright: 0.72 + 0.24 * k(TONE) - 0.12 * tube,
                duty: 0.42 - 0.18 * k(TONE),
                breath: k(BREATH) * (1.0 + 0.6 * tube) * (1.0 + 0.6 * tired),
                beat: k(BEAT) * 0.3,
                vib: &self.vib[..len],
                vib_amp: 0.35,
                vib_pitch: 0.08,
                pitch_bend: self.pitch_bend,
                share,
                radiate: 0.45 - 0.25 * tube,
                ping: 0.04,
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
            // The body and the mouthpiece.
            for j in 0..len {
                for (c, d) in [&mut self.dl, &mut self.dr].into_iter().enumerate() {
                    let x = d[j];
                    let b1 = self.body1[c].tick(x).band;
                    let b2 = self.body2[c].tick(x).band;
                    d[j] = (x * (1.0 - 0.3 * body) + body * (b1 * 1.3 + b2 * 0.8)) * gain;
                }
            }
            self.wl[..len].copy_from_slice(&self.dl[..len]);
            self.wr[..len].copy_from_slice(&self.dr[..len]);
            if space > 0.001 {
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
                let ctx = FxCtx { sr, bpm: 120.0, beat: 0.0, playing: false };
                self.hall.process(&self.hall_p, &mut self.wl[..len], &mut self.wr[..len], &ctx, None);
                for j in 0..len {
                    self.dl[j] += self.wl[j] * space * 0.8;
                    self.dr[j] += self.wr[j] * space * 0.8;
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

    fn dry(name: &str) -> Vec<f32> {
        let mut p = preset(name);
        p[SPACE] = 0.0;
        p[VIBRATO] = 0.0;
        p[BEAT] = 0.0;
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
}
