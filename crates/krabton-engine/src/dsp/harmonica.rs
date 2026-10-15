//! Harmonica: a diatonic harp in Richter tuning, ten holes with a blow and
//! a draw reed each (in C: blow C E G C E G C E G C, draw D G B D F A B D
//! F A). Free reeds (see `free_reed`) in a small comb, played in cupped
//! hands.
//!
//! With Holes every note goes where a player finds it: in a hole, blown or
//! drawn. A note the harp lacks is bent (a draw bend on the low holes, a
//! blow bend on the high ones): it starts on the hole's own note and wails
//! down into place, darker and breathier (Bends: how slowly), and lets go
//! upwards. Past what bends reach it is overblown: thin, edgy and a little
//! unsteady. Blowing and drawing never sound together: a note the other way
//! takes over. The lips cover one hole or more (Mouth), and the holes
//! beside it sound along in the same breath: chords, a train's chug.
//!
//! Its small reeds are bright and buzzy (what is heard is the flow's change,
//! and the tongue pings as it starts), and the player's mouth and throat
//! are part of the instrument: two vowel formants, an "aw" with open hands
//! that closes to an "oo" as the hands cup around it (Hands), and opening
//! them on every note (Wah) says wah. Free plays every note plainly, as a
//! chromatic harmonica.
//!
//! The harp is in a key: its own, or the song's (Song) played in a
//! position: 1st straight, 2nd cross harp (the blues: a harp a fourth up),
//! 3rd slant (minor: a whole tone down). Original code.

use super::effects::FxCtx;
use super::free_reed::{free_reed, Blow, Reed};
use super::fx::hall::{self, Hall};
use super::{db_to_gain, midi_to_freq, Rng, Svf};
use crate::params::{ParamSpec, Unit};
use crate::scale::{Scale, ScaleKind};
use std::f32::consts::TAU;

pub const KEY: usize = 0;
pub const POSITION: usize = 1;
pub const PLAYING: usize = 2;
pub const MOUTH: usize = 3;
pub const BENDS: usize = 4;
pub const HANDS: usize = 5;
pub const WAH: usize = 6;
pub const BREATH: usize = 7;
pub const TONE: usize = 8;
pub const THROAT: usize = 9;
pub const VIB_RATE: usize = 10;
pub const BEAT: usize = 11;
pub const ATTACK: usize = 12;
pub const RELEASE: usize = 13;
pub const TRANSPOSE: usize = 14;
pub const SPACE: usize = 15;
pub const VOLUME: usize = 16;

pub const KEYS: &[&str] = &["Song", "C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];
pub const POSITIONS: &[&str] = &["1st Straight", "2nd Cross", "3rd Slant"];
pub const PLAYINGS: &[&str] = &["Free", "Holes"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Key", "Harp", KEYS, 0),
    ParamSpec::choice("Position", "Harp", POSITIONS, 1),
    ParamSpec::choice("Playing", "Play", PLAYINGS, 1),
    ParamSpec::int("Mouth", "Play", 1.0, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Bends", "Play", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Hands", "Hands", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Wah", "Hands", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Breath", "Reed", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Tone", "Reed", 0.0, 1.0, 0.55, Unit::Percent),
    ParamSpec::lin("Throat", "Expression", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Vibrato Rate", "Expression", 3.0, 8.0, 5.5, Unit::Hz),
    ParamSpec::lin("Beat", "Reed", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Attack", "Expression", 0.005, 0.5, 0.02, Unit::Sec),
    ParamSpec::log("Release", "Expression", 0.02, 1.5, 0.1, Unit::Sec),
    ParamSpec::int("Transpose", "Play", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Space", "Sound", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Volume", "Sound", -24.0, 12.0, 0.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Blues Harp", "Country Bend", "Cupped Wah", "Lonesome Train", "Train Chug", "Folk Harp", "Tremolo Harmonica", "Chromatic Harmonica"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Blues Harp" => &[(BENDS, 0.65), (HANDS, 0.5), (WAH, 0.2), (BREATH, 0.35), (THROAT, 0.35), (SPACE, 0.2)],
        "Country Bend" => &[(BENDS, 0.95), (HANDS, 0.25), (TONE, 0.6), (THROAT, 0.2), (SPACE, 0.2)],
        "Cupped Wah" => &[(HANDS, 0.75), (WAH, 0.8), (THROAT, 0.45), (BENDS, 0.6), (SPACE, 0.25)],
        "Lonesome Train" => &[(MOUTH, 2.0), (HANDS, 0.6), (WAH, 0.3), (THROAT, 0.5), (BENDS, 0.8), (RELEASE, 0.25), (SPACE, 0.55)],
        "Train Chug" => &[(MOUTH, 3.0), (BENDS, 0.3), (HANDS, 0.35), (WAH, 0.45), (BREATH, 0.55), (TONE, 0.5), (THROAT, 0.0), (ATTACK, 0.01), (RELEASE, 0.06), (SPACE, 0.15)],
        "Folk Harp" => &[(POSITION, 0.0), (MOUTH, 2.0), (BENDS, 0.25), (HANDS, 0.1), (BREATH, 0.25), (TONE, 0.62), (THROAT, 0.15), (SPACE, 0.3)],
        "Tremolo Harmonica" => &[(POSITION, 0.0), (MOUTH, 2.0), (BEAT, 0.55), (HANDS, 0.1), (BENDS, 0.2), (THROAT, 0.0), (SPACE, 0.4)],
        "Chromatic Harmonica" => &[(PLAYING, 0.0), (TONE, 0.5), (HANDS, 0.15), (THROAT, 0.45), (VIB_RATE, 5.2), (BREATH, 0.2), (ATTACK, 0.03), (RELEASE, 0.15), (SPACE, 0.4)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

/// The blow and draw notes of each hole, in semitones above the harp's
/// lowest note.
pub const BLOW: [i32; 10] = [0, 4, 7, 12, 16, 19, 24, 28, 31, 36];
pub const DRAW: [i32; 10] = [2, 7, 11, 14, 17, 21, 23, 26, 29, 33];

/// How a note is played on the harp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tech {
    Plain,
    /// Bent down this many semitones from the hole's own note.
    Bend(u8),
    Overblow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hole {
    /// 1..10.
    pub hole: u8,
    pub draw: bool,
    pub tech: Tech,
}

/// Where a note `rel` semitones above the harp's lowest (0..=37) is played.
pub fn hole_of(rel: i32) -> Hole {
    use Tech::*;
    let (hole, draw, tech) = match rel {
        0 => (1, false, Plain),
        1 => (1, true, Bend(1)),
        2 => (1, true, Plain),
        3 => (1, false, Overblow),
        4 => (2, false, Plain),
        5 => (2, true, Bend(2)),
        6 => (2, true, Bend(1)),
        7 => (2, true, Plain),
        8 => (3, true, Bend(3)),
        9 => (3, true, Bend(2)),
        10 => (3, true, Bend(1)),
        11 => (3, true, Plain),
        12 => (4, false, Plain),
        13 => (4, true, Bend(1)),
        14 => (4, true, Plain),
        15 => (4, false, Overblow),
        16 => (5, false, Plain),
        17 => (5, true, Plain),
        18 => (5, false, Overblow),
        19 => (6, false, Plain),
        20 => (6, true, Bend(1)),
        21 => (6, true, Plain),
        22 => (6, false, Overblow),
        23 => (7, true, Plain),
        24 => (7, false, Plain),
        25 => (7, true, Overblow),
        26 => (8, true, Plain),
        27 => (8, false, Bend(1)),
        28 => (8, false, Plain),
        29 => (9, true, Plain),
        30 => (9, false, Bend(1)),
        31 => (9, false, Plain),
        32 => (9, true, Overblow),
        33 => (10, true, Plain),
        34 => (10, false, Bend(2)),
        35 => (10, false, Bend(1)),
        36 => (10, false, Plain),
        _ => (10, true, Overblow),
    };
    Hole { hole, draw, tech }
}

/// The note of a hole, blown or drawn, above the harp's lowest.
pub fn hole_note(hole: u8, draw: bool) -> i32 {
    let i = (hole.clamp(1, 10) - 1) as usize;
    if draw {
        DRAW[i]
    } else {
        BLOW[i]
    }
}

/// The harp's key (a pitch class), from its own or the song's.
pub fn harp_key(p: &[f32], song: &Scale) -> u8 {
    let k = p[KEY].round().clamp(0.0, 12.0) as u8;
    if k > 0 {
        return k - 1;
    }
    let pos = p[POSITION].round().clamp(0.0, 2.0) as usize;
    ((song.root as u32 + [0, 5, 10][pos]) % 12) as u8
}

/// The harp's lowest note: harps from G (G3) up to F# (F#4).
pub fn harp_low(p: &[f32], song: &Scale) -> i32 {
    55 + (harp_key(p, song) as i32 - 7).rem_euclid(12)
}

const MAX_VOICES: usize = 8;
const CHUNK: usize = 64;

pub struct HarmonicaState {
    /// Pitch bend in semitones, set by the engine.
    pub pitch_bend: f32,
    /// The key the song plays in, set by the engine.
    pub song_key: Scale,
    sr: f32,
    voices: Vec<Reed>,
    rng: Rng,
    vib_ph: f32,
    vib: Vec<f32>,
    /// The hands open after each note (Wah), 0 closed to 1.
    wah: f32,
    hands_now: f32,
    air: f32,
    /// For the screen: the harp's lowest note, its key, and Holes.
    low: i32,
    key_pc: u8,
    holes: bool,
    tmp: Vec<f32>,
    dl: Vec<f32>,
    dr: Vec<f32>,
    hands: [Svf; 2],
    comb: [Svf; 2],
    tract1: Svf,
    tract2: Svf,
    hall: Box<Hall>,
    hall_p: Vec<f32>,
}

impl HarmonicaState {
    pub fn new(sr: f32) -> Self {
        Self {
            pitch_bend: 0.0,
            song_key: Scale::new(0, ScaleKind::Major),
            sr: sr.max(1000.0),
            voices: (0..MAX_VOICES).map(|_| Reed::default()).collect(),
            rng: Rng::new(0x4A2_B105),
            vib_ph: 0.0,
            vib: vec![0.0; CHUNK],
            wah: 1.0,
            hands_now: 0.0,
            air: 0.0,
            low: 60,
            key_pc: 0,
            holes: true,
            tmp: vec![0.0; CHUNK],
            dl: vec![0.0; CHUNK],
            dr: vec![0.0; CHUNK],
            hands: [Svf::new(), Svf::new()],
            comb: [Svf::new(), Svf::new()],
            tract1: Svf::new(),
            tract2: Svf::new(),
            hall: Box::new(Hall::new(sr)),
            hall_p: crate::params::defaults(hall::SPECS),
        }
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = note as i32 + p[TRANSPOSE].round() as i32;
        if p[PLAYING] < 0.5 {
            // Free: every note plainly.
            let i = free_reed(&self.voices);
            let sounding = n.clamp(0, 127) as u8;
            self.voices[i].start(sounding, note, midi_to_freq(sounding as f32), vel, 1.0, 0.0);
            self.wah = 0.0;
            return;
        }
        let low = harp_low(p, &self.song_key);
        let mut rel = n - low;
        while rel < 0 {
            rel += 12;
        }
        while rel > 37 {
            rel -= 12;
        }
        let blowing = self.voices.iter().any(|v| v.gate && !v.draw && v.hole > 0);
        let mut h = hole_of(rel);
        // G (in a C harp) is in two holes: blown in 3 while the breath goes out.
        if rel == 7 && blowing {
            h = Hole { hole: 3, draw: false, tech: Tech::Plain };
        }
        // The breath goes one way, and the lips span no more than four holes.
        for v in self.voices.iter_mut().filter(|v| v.gate) {
            if v.draw != h.draw || (v.hole as i32 - h.hole as i32).abs() >= 4 || v.hole == 0 {
                v.gate = false;
            }
        }
        let bends = p[BENDS].clamp(0.0, 1.0);
        let sounding = (low + rel).clamp(0, 127) as u8;
        let i = free_reed(&self.voices);
        let v = &mut self.voices[i];
        v.start(sounding, note, midi_to_freq(sounding as f32), vel, 1.0, 0.0);
        v.hole = h.hole;
        v.draw = h.draw;
        match h.tech {
            Tech::Bend(k) => {
                v.tech = 1;
                v.bend_from = k as f32;
                v.bend_t = 0.015 + 0.12 * bends;
                v.rise = k as f32 * 0.5 * bends;
                v.dark = 0.72;
                v.amp = 0.85;
                v.airy = 1.5;
            }
            Tech::Overblow => {
                v.tech = 2;
                v.dark = 1.3;
                v.amp = 0.7;
                v.rough = 1.0;
                v.bend_from = 0.4;
                v.bend_t = 0.012;
                v.airy = 1.8;
            }
            Tech::Plain => {}
        }
        // The other holes under the lips sound along, the same way.
        let mouth = p[MOUTH].round().clamp(1.0, 4.0) as usize;
        const SPANS: [&[i32]; 4] = [&[], &[1], &[-1, 1], &[-1, 1, 2]];
        for &d in SPANS[mouth - 1] {
            let mut hh = h.hole as i32 + d;
            if mouth == 2 && hh > 10 {
                hh = h.hole as i32 - 1;
            }
            if !(1..=10).contains(&hh) {
                continue;
            }
            let hh = hh as u8;
            if self.voices.iter().any(|v| v.gate && v.hole == hh && v.draw == h.draw) {
                continue;
            }
            let s = (low + hole_note(hh, h.draw)).clamp(0, 127) as u8;
            let j = free_reed(&self.voices);
            let w = &mut self.voices[j];
            w.start(s, note, midi_to_freq(s as f32), vel * 0.9, 0.62, 0.0);
            w.hole = hh;
            w.draw = h.draw;
        }
        self.wah = 0.0;
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        let _ = p;
        for v in self.voices.iter_mut().filter(|v| v.owner == note) {
            v.gate = false;
        }
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.gate = false;
        }
    }

    /// The screen: [0] on, [1] the harp's lowest note, [2] the air, [3] how
    /// closed the hands are now, then per hole (4 + h) how loud it sounds
    /// (blown above 0, drawn below), and (14 + h) its bend in semitones (-1:
    /// overblown); [24] Holes, [25] the key the harp is in.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = 1.0;
        out[1] = self.low as f32;
        out[2] = self.air;
        out[3] = self.hands_now;
        out[24] = if self.holes { 1.0 } else { 0.0 };
        out[25] = self.key_pc as f32;
        for i in 0..10 {
            out[4 + i] = 0.0;
            out[14 + i] = 0.0;
        }
        for v in self.voices.iter().filter(|v| v.hole > 0 && v.level > 0.02) {
            let i = (v.hole - 1) as usize;
            let lv = v.level.min(1.0);
            out[4 + i] = if v.draw { -lv } else { lv };
            out[14 + i] = match v.tech {
                1 => v.bend_from,
                2 => -1.0,
                _ => 0.0,
            };
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let sr = self.sr;
        self.low = harp_low(p, &self.song_key);
        self.key_pc = harp_key(p, &self.song_key);
        self.holes = p[PLAYING] >= 0.5;
        let k = |i: usize| p[i].clamp(0.0, 1.0);
        let att = 1.0 - (-1.0 / (p[ATTACK].clamp(0.002, 2.0) * 0.4 * sr)).exp();
        let rel = 1.0 - (-1.0 / (p[RELEASE].clamp(0.01, 4.0) * 0.4 * sr)).exp();
        let (depth, rate) = (k(THROAT), p[VIB_RATE].clamp(1.0, 12.0));
        let (closed, wah_amt) = (k(HANDS), k(WAH));
        let space = k(SPACE);
        let gain = db_to_gain(p[VOLUME]) * 0.12;
        for c in 0..2 {
            self.comb[c].set_q(2300.0, 2.0, sr);
        }
        let n = l.len().min(r.len());
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            let dt = len as f32 / sr;
            for j in 0..len {
                self.vib_ph = (self.vib_ph + rate / sr).fract();
                self.vib[j] = (TAU * self.vib_ph).sin() * depth;
            }
            // The hands: cupped around it, opening after each note.
            self.wah += (1.0 - self.wah) * (1.0 - (-dt / 0.18).exp());
            let shut = (closed + wah_amt * 0.75 * (1.0 - self.wah) * (1.0 - closed)).clamp(0.0, 1.0);
            self.hands_now = shut;
            // The mouth and throat: "aw" open, "oo" as the hands close.
            let o = 1.0 - 0.8 * shut;
            self.tract1.set_q(320.0 + 380.0 * o, 2.5, sr);
            self.tract2.set_q(760.0 + 640.0 * o, 3.5, sr);
            // The hands: a softer lowpass round it.
            let cutoff = 2200.0 * 2f32.powf(2.2 * (1.0 - shut));
            for c in 0..2 {
                self.hands[c].set_q(cutoff, 0.707 + 0.6 * closed, sr);
            }
            let blow = Blow {
                sr,
                att,
                rel,
                glide_k: 1.0,
                bright: 0.94 + 0.05 * k(TONE),
                duty: 0.34 - 0.1 * k(TONE),
                breath: k(BREATH),
                beat: k(BEAT) * 0.3,
                vib: &self.vib[..len],
                vib_amp: 0.5,
                vib_pitch: 0.12,
                pitch_bend: self.pitch_bend,
                share: 1.0,
                radiate: 0.9,
                ping: 0.09,
                tongue: 0.2,
                puff_off: 0.0,
            };
            self.dl[..len].fill(0.0);
            let mut air = 0.0f32;
            for v in self.voices.iter_mut() {
                if v.quiet() {
                    v.level *= 0.9;
                    continue;
                }
                self.tmp[..len].fill(0.0);
                v.block(&blow, &mut self.rng, &mut self.tmp[..len]);
                air = air.max(v.env);
                for j in 0..len {
                    self.dl[j] += self.tmp[j];
                }
            }
            self.air = self.air * 0.8 + air * 0.2;
            // The comb's small chambers, the mouth and throat, then the hands.
            for j in 0..len {
                let x = self.dl[j];
                let c = self.comb[0].tick(x).band;
                let v1 = self.tract1.tick(x).band;
                let v2 = self.tract2.tick(x).band;
                let y = x * 0.55 + 0.2 * c + 1.1 * v1 + 0.8 * v2;
                let y = self.hands[0].tick(y).low * gain;
                self.dl[j] = y;
                self.dr[j] = y;
            }
            if space > 0.001 {
                let mut wl = [0.0f32; CHUNK];
                let mut wr = [0.0f32; CHUNK];
                wl[..len].copy_from_slice(&self.dl[..len]);
                wr[..len].copy_from_slice(&self.dr[..len]);
                let hp = &mut self.hall_p;
                hp[hall::PREDELAY] = 15.0;
                hp[hall::SIZE] = 0.4 + 0.5 * space;
                hp[hall::DECAY] = 1.2 + 4.0 * space;
                hp[hall::LOW_DECAY] = 0.9;
                hp[hall::DAMPING] = 5000.0;
                hp[hall::DIFFUSION] = 0.8;
                hp[hall::MODULATION] = 0.2;
                hp[hall::FREEZE] = 0.0;
                hp[hall::WIDTH] = 1.0;
                hp[hall::MIX] = 1.0;
                let ctx = FxCtx { sr, bpm: 120.0, beat: 0.0, playing: false };
                self.hall.process(&self.hall_p, &mut wl[..len], &mut wr[..len], &ctx, None);
                for j in 0..len {
                    self.dl[j] += wl[j] * space * 0.8;
                    self.dr[j] += wr[j] * space * 0.8;
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

    fn run(st: &mut HarmonicaState, p: &[f32], secs: f32) -> Vec<f32> {
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

    /// A C harp, plain and dry.
    fn c_harp(name: &str) -> Vec<f32> {
        let mut p = preset(name);
        p[KEY] = 1.0;
        p[SPACE] = 0.0;
        p[THROAT] = 0.0;
        p[BEAT] = 0.0;
        p[WAH] = 0.0;
        p[HANDS] = 0.0;
        p
    }

    #[test]
    fn the_holes_of_a_richter_harp() {
        // Every note from the lowest to three octaves up has a place.
        for rel in 0..=37 {
            let h = hole_of(rel);
            let natural = hole_note(h.hole, h.draw);
            match h.tech {
                Tech::Plain => assert_eq!(natural, rel),
                Tech::Bend(k) => assert_eq!(natural - k as i32, rel),
                Tech::Overblow => assert_eq!((rel - hole_note(h.hole, !h.draw)).abs(), 1),
            }
        }
        // A C harp starts at C4, a G harp at G3; the blues on a C harp is in G.
        let mut p = preset("Blues Harp");
        p[KEY] = 1.0;
        assert_eq!(harp_low(&p, &Scale::new(0, ScaleKind::Major)), 60);
        p[KEY] = 8.0;
        assert_eq!(harp_low(&p, &Scale::new(0, ScaleKind::Major)), 55);
        p[KEY] = 0.0;
        assert_eq!(harp_key(&p, &Scale::new(7, ScaleKind::Major)), 0, "G in 2nd position: a C harp");
    }

    #[test]
    fn every_preset_plays_and_stays_bounded() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = HarmonicaState::new(SR);
            // The second hole blown: on every harp a plain note.
            let note = (harp_low(&p, &st.song_key) + 4) as u8;
            st.note_on(&p, note, 0.8);
            let l = run(&mut st, &p, 1.2);
            st.note_off(&p, note);
            let l2 = run(&mut st, &p, 1.5);
            let peak = l.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            if std::env::var("REED_REPORT").is_ok() {
                eprintln!("{name}: peak {peak:.3}");
            }
            assert!(l.iter().chain(&l2).all(|v| v.is_finite()), "{name}");
            assert!(peak > 0.02 && peak < 1.2, "{name}: {peak}");
            let f = midi_to_freq(note as f32);
            let tail = &l[(0.5 * SR) as usize..];
            assert!(mag(tail, f) > 3.0 * mag(tail, f * 1.06), "{name}: pitch");
        }
    }

    #[test]
    fn blowing_and_drawing_never_sound_together() {
        let p = c_harp("Blues Harp");
        // C4 blown, then D4 drawn in the same hole: only D.
        let mut st = HarmonicaState::new(SR);
        st.note_on(&p, 60, 0.8);
        st.note_on(&p, 62, 0.8);
        let l = run(&mut st, &p, 1.0);
        let t = &l[(0.4 * SR) as usize..];
        assert!(mag(t, midi_to_freq(62.0)) > 6.0 * mag(t, midi_to_freq(60.0)));
        // C4 and E4, both blown: both.
        let mut st = HarmonicaState::new(SR);
        st.note_on(&p, 60, 0.8);
        st.note_on(&p, 64, 0.8);
        let l = run(&mut st, &p, 1.0);
        let t = &l[(0.4 * SR) as usize..];
        let (c, e) = (mag(t, midi_to_freq(60.0)), mag(t, midi_to_freq(64.0)));
        assert!(c > 0.3 * e && e > 0.3 * c, "{c} {e}");
    }

    #[test]
    fn three_holes_under_the_lips_play_a_chord() {
        let mut p = c_harp("Train Chug");
        p[MOUTH] = 3.0;
        let mut st = HarmonicaState::new(SR);
        // E4 is the second hole blown: with it the first and third, C and G.
        st.note_on(&p, 64, 0.8);
        let l = run(&mut st, &p, 1.0);
        let t = &l[(0.4 * SR) as usize..];
        let e = mag(t, midi_to_freq(64.0));
        for n in [60.0, 67.0] {
            assert!(mag(t, midi_to_freq(n)) > 0.2 * e, "{n}");
        }
        assert!(mag(t, midi_to_freq(62.0)) < 0.1 * e);
    }

    #[test]
    fn a_bent_note_wails_down_into_place() {
        // A4 on a C harp: the third hole drawn (B4), bent down a whole tone.
        let mut p = c_harp("Country Bend");
        p[BENDS] = 1.0;
        let mut st = HarmonicaState::new(SR);
        st.note_on(&p, 69, 0.8);
        let l = run(&mut st, &p, 1.0);
        let (a, b) = (midi_to_freq(69.0), midi_to_freq(71.0));
        let early = &l[(0.005 * SR) as usize..(0.045 * SR) as usize];
        let late = &l[(0.5 * SR) as usize..];
        assert!(mag(late, a) > 4.0 * mag(late, b), "lands on A");
        assert!(mag(early, b) / mag(early, a) > 2.0 * mag(late, b) / mag(late, a), "starts higher");
    }

    #[test]
    fn free_plays_any_chord() {
        let p = c_harp("Chromatic Harmonica");
        let mut st = HarmonicaState::new(SR);
        for n in [61, 66, 70] {
            st.note_on(&p, n, 0.8);
        }
        let l = run(&mut st, &p, 1.0);
        let t = &l[(0.4 * SR) as usize..];
        for n in [61.0, 66.0, 70.0] {
            assert!(mag(t, midi_to_freq(n)) > 2.0 * mag(t, midi_to_freq(n + 1.0)), "{n}");
        }
    }
}
