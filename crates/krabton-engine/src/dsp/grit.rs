//! Grit: a pad sampler of the late eighties and the nineties, sixteen pads
//! with a sample each, played through the signal path of the old machines.
//!
//! The machine runs at its own rate (26 or 40 kHz, 44.1 for the later
//! sixteen-bit ones, 11 kHz for the cheapest), on a clock shared by every
//! voice. On each of its ticks a voice reads its sample at the machine's
//! rate: the early ones by taking the nearest sample, no interpolation, so a
//! sample pitched up is read by skipping samples (drop-sample pitching, the
//! aliasing that came with it, folded under the machine's Nyquist), the
//! sixteen-bit ones between two samples. What is read is kept in the
//! machine's bits (12, 16 or 8), after an input drive into soft saturation;
//! the voices' sum goes out of a converter of as many bits (its steps, and
//! the noise of the old converters where they had it) and is held until the
//! next tick (zero-order hold: the images above the machine's rate stay
//! in), then through the output filter the unit had (none on the rawest).
//! *Clean* is the same pads at the host's rate, nothing lost.
//!
//! Each pad: its sample (or one of the sixteen built-in sounds), tune (the
//! old way: by the rate it is read at), level, pan, decay, a low-pass
//! filter, reverse, a choke group and one-shot or gate. A note on a pad
//! held with *Repeat* on plays it again on the beat's grid (1/4 to 1/32,
//! triplets too) with *Swing*; *16 Levels* plays one pad's sample on all
//! sixteen, its tune, level, decay or filter in sixteen steps. A loop can be
//! chopped to the pads at its hits or on a grid (see [`chop`]).

use super::{db_to_gain, FilterMode, Rng, Svf};
use crate::params::{ParamSpec, Unit};
use crate::sample::{SampleData, SampleRef, SampleRegion};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub const CHARACTER: usize = 0;
pub const DRIVE: usize = 1;
pub const REPEAT: usize = 2;
pub const SWING: usize = 3;
pub const LEVELS: usize = 4;
pub const LEVELS_PAD: usize = 5;
pub const VOLUME: usize = 6;

pub const CHARACTERS: &[&str] = &["12-bit 26k", "12-bit 40k", "16-bit Vintage", "8-bit Lo", "Clean"];
pub const REPEATS: &[&str] = &["Off", "1/4", "1/8", "1/8T", "1/16", "1/16T", "1/32"];
const REPEAT_BEATS: [f64; 7] = [0.0, 1.0, 0.5, 1.0 / 3.0, 0.25, 1.0 / 6.0, 0.125];
pub const LEVEL_MODES: &[&str] = &["Off", "Tune", "Level", "Decay", "Filter"];

pub const PADS: usize = 16;
/// The note of the first pad (C1); the others go up from it.
pub const FIRST_NOTE: u8 = 36;
const VOICES: usize = 16;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Character", "Machine", CHARACTERS, 0),
    ParamSpec::lin("Drive", "Machine", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Repeat", "Pads", REPEATS, 0),
    ParamSpec::lin("Swing", "Pads", 0.5, 0.75, 0.5, Unit::Percent),
    ParamSpec::choice("16 Levels", "Pads", LEVEL_MODES, 0),
    ParamSpec::int("Levels Pad", "Pads", 1.0, 16.0, 1.0, Unit::None),
    ParamSpec::lin("Volume", "Machine", -30.0, 6.0, -3.0, Unit::Db),
];

/// How a machine of the old kind is built: its rate, its bits, whether it
/// reads between samples, its converter's noise (in steps) and its output
/// filter (Hz, 0: none; and how steep: poles in twos).
struct Machine {
    rate: f32,
    bits: u32,
    interpolate: bool,
    noise: f32,
    filter: f32,
    poles: usize,
}

const MACHINES: [Machine; 5] = [
    // Twelve bits at 26.04 kHz, nearest-sample reads, its raw outputs unfiltered.
    Machine { rate: 26_040.0, bits: 12, interpolate: false, noise: 0.0, filter: 0.0, poles: 0 },
    // Twelve bits at 40 kHz, a gentle filter out, a little converter hiss.
    Machine { rate: 40_000.0, bits: 12, interpolate: false, noise: 0.35, filter: 14_000.0, poles: 1 },
    // Sixteen bits at 44.1 kHz, reads between samples, a steep filter out.
    Machine { rate: 44_100.0, bits: 16, interpolate: true, noise: 0.0, filter: 18_000.0, poles: 2 },
    // Eight bits at 11 kHz: the cheapest, a dull filter and hiss.
    Machine { rate: 11_025.0, bits: 8, interpolate: false, noise: 0.5, filter: 4_500.0, poles: 2 },
    // Clean: the host's rate (set below), no steps.
    Machine { rate: 0.0, bits: 0, interpolate: true, noise: 0.0, filter: 0.0, poles: 0 },
];

/// The built-in sounds, made by synthesis when they are first needed.
pub const SOUNDS: &[&str] = &[
    "Kick", "Snare", "Rim", "Clap", "Closed Hat", "Open Hat", "Low Tom", "High Tom", "Crash", "Ride", "Shaker", "Cowbell", "Sub Bass", "Stab", "Vox", "Noise Hit",
];
/// The rate the built-in sounds are made at.
const SOUND_RATE: f32 = 44_100.0;

static SOUND_TABLE: OnceLock<Vec<Vec<f32>>> = OnceLock::new();

/// A built-in sound's samples (at 44.1 kHz), once they are made.
pub fn sound(k: usize) -> Option<&'static [f32]> {
    SOUND_TABLE.get()?.get(k).map(|v| &v[..])
}

/// Makes the built-in sounds (once; never on the audio thread).
pub fn warm() {
    SOUND_TABLE.get_or_init(make_sounds);
}

fn make_sounds() -> Vec<Vec<f32>> {
    let sr = SOUND_RATE;
    let mut rng = Rng::new(0x6721_7001);
    let n = |secs: f32| (secs * sr) as usize;
    let env = |t: f32, d: f32| (-t / d).exp();
    let mut out = Vec::new();
    for (k, _) in SOUNDS.iter().enumerate() {
        let len = match k {
            5 | 8 | 9 => n(1.2),
            12 => n(1.0),
            13 | 14 => n(0.6),
            _ => n(0.45),
        };
        let mut v = vec![0.0f32; len];
        let mut ph = 0.0f32;
        let mut ph2 = 0.0f32;
        let mut lp = 0.0f32;
        let mut hp = 0.0f32;
        for (i, x) in v.iter_mut().enumerate() {
            let t = i as f32 / sr;
            let noise = rng.bipolar();
            *x = match k {
                0 => {
                    let f = 48.0 + 120.0 * env(t, 0.03);
                    ph += f / sr;
                    (ph * std::f32::consts::TAU).sin() * env(t, 0.22) * 1.1 + noise * env(t, 0.003) * 0.3
                }
                1 => {
                    ph += 190.0 / sr;
                    hp += (noise - hp) * 0.5;
                    (ph * std::f32::consts::TAU).sin() * env(t, 0.05) * 0.6 + (noise - hp * 0.6) * env(t, 0.12) * 0.6
                }
                2 => {
                    ph += 520.0 / sr;
                    (ph * std::f32::consts::TAU).sin() * env(t, 0.012) * 0.8 + noise * env(t, 0.004) * 0.3
                }
                3 => {
                    // Three claps close together, then the tail.
                    let burst = [0.0, 0.011, 0.023].iter().map(|&s: &f32| if t >= s { env(t - s, 0.008) } else { 0.0 }).sum::<f32>();
                    lp += (noise - lp) * 0.35;
                    (noise - lp) * (burst * 0.6 + env(t, 0.15) * 0.35)
                }
                4 | 5 | 10 => {
                    hp += (noise - hp) * 0.6;
                    let d = [0.0, 0.0, 0.0, 0.0, 0.035, 0.32, 0.0, 0.0, 0.0, 0.0, 0.06][k];
                    let a = if k == 10 { (t / 0.02).min(1.0) } else { 1.0 };
                    (noise - hp) * env(t, d) * a * 0.9
                }
                6 | 7 => {
                    let f0 = if k == 6 { 95.0 } else { 160.0 };
                    ph += (f0 + f0 * 0.6 * env(t, 0.05)) / sr;
                    (ph * std::f32::consts::TAU).sin() * env(t, 0.25) * 0.9
                }
                8 | 9 => {
                    // Metal: six square partials, hissed.
                    let fs = [263.0, 400.0, 421.0, 474.0, 587.0, 845.0];
                    let m: f32 = fs.iter().map(|&f| if ((t * f * if k == 8 { 1.6 } else { 2.3 }) % 1.0) < 0.5 { 1.0 } else { -1.0 }).sum::<f32>() / 6.0;
                    hp += (m + noise * 0.5 - hp) * 0.7;
                    (m + noise * 0.5 - hp) * env(t, if k == 8 { 0.5 } else { 0.35 }) * 0.6
                }
                11 => {
                    let sq = |f: f32| if (t * f) % 1.0 < 0.5 { 1.0 } else { -1.0 };
                    lp += ((sq(540.0) + sq(800.0)) * 0.5 - lp) * 0.3;
                    lp * env(t, 0.08) * 0.7
                }
                12 => {
                    ph += 55.0 / sr;
                    let s = (ph * std::f32::consts::TAU).sin();
                    (s * 1.6).tanh() * env(t, 0.5) * 0.9
                }
                13 => {
                    // A minor chord of saws through a closing filter.
                    let saw = |f: f32| 2.0 * ((t * f) % 1.0) - 1.0;
                    let c = (saw(220.0) + saw(261.6) + saw(329.6) + saw(440.0)) * 0.25;
                    let k2 = 0.05 + 0.5 * env(t, 0.08);
                    lp += (c - lp) * k2;
                    lp * env(t, 0.25) * 0.9
                }
                14 => {
                    // "Hey": a buzz through two formants that move.
                    ph += 140.0 / sr;
                    let buzz = 2.0 * (ph % 1.0) - 1.0;
                    let f1 = 600.0 + 200.0 * (t / 0.3).min(1.0);
                    ph2 += f1 / sr;
                    buzz * (ph2 * std::f32::consts::TAU).sin().abs() * env(t, 0.18) * 0.8 + noise * env(t, 0.03) * 0.2
                }
                _ => {
                    lp += (noise - lp) * 0.2;
                    lp * env(t, 0.2) * 1.2
                }
            };
        }
        let peak = v.iter().fold(0.0f32, |m, x| m.max(x.abs())).max(1e-6);
        v.iter_mut().for_each(|x| *x *= 0.9 / peak);
        out.push(v);
    }
    out
}

/// A pad: what it plays and how.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GritPad {
    #[serde(default)]
    pub name: String,
    /// Its own sample, or else the built-in sound `sound`.
    #[serde(default)]
    pub sample: Option<SampleRef>,
    #[serde(default)]
    pub sound: Option<u8>,
    /// Semitones (by the rate it is read at).
    #[serde(default)]
    pub tune: f32,
    #[serde(default)]
    pub level_db: f32,
    /// -1 (left) to 1 (right).
    #[serde(default)]
    pub pan: f32,
    /// 0..1: how soon it dies away (1: never, the whole sample).
    #[serde(default = "one")]
    pub decay: f32,
    /// 0..1: its low-pass filter (1: open).
    #[serde(default = "one")]
    pub filter: f32,
    #[serde(default)]
    pub reverse: bool,
    /// Pads of the same group (1 to 4; 0: none) cut each other off.
    #[serde(default)]
    pub choke: u8,
    /// Plays while held (else to its end: one-shot).
    #[serde(default)]
    pub gate: bool,
}

fn one() -> f32 {
    1.0
}

impl Default for GritPad {
    fn default() -> Self {
        Self { name: String::new(), sample: None, sound: None, tune: 0.0, level_db: 0.0, pan: 0.0, decay: 1.0, filter: 1.0, reverse: false, choke: 0, gate: false }
    }
}

impl GritPad {
    pub fn builtin(k: usize) -> Self {
        let k = k.min(SOUNDS.len() - 1);
        Self { name: SOUNDS[k].to_string(), sound: Some(k as u8), ..Self::default() }
    }

    /// What it shows as its name.
    pub fn label(&self) -> &str {
        if !self.name.is_empty() {
            &self.name
        } else if let Some(s) = &self.sample {
            &s.name
        } else {
            ""
        }
    }

    pub fn is_empty(&self) -> bool {
        self.sample.is_none() && self.sound.is_none()
    }
}

/// Sixteen pads with the built-in sounds, in order.
pub fn default_pads() -> Vec<GritPad> {
    (0..PADS).map(GritPad::builtin).collect()
}

pub fn preset_names() -> &'static [&'static str] {
    &["Boom Bap 26k", "Dusty 40k", "Vintage 16", "Lo-Fi 8-bit", "Clean Kit", "Swung Repeats", "16 Levels Tune", "Empty"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Boom Bap 26k" => &[(CHARACTER, 0.0), (DRIVE, 0.25)],
        "Dusty 40k" => &[(CHARACTER, 1.0), (DRIVE, 0.15)],
        "Vintage 16" => &[(CHARACTER, 2.0)],
        "Lo-Fi 8-bit" => &[(CHARACTER, 3.0), (DRIVE, 0.4)],
        "Clean Kit" => &[(CHARACTER, 4.0)],
        "Swung Repeats" => &[(CHARACTER, 1.0), (REPEAT, 4.0), (SWING, 0.62)],
        "16 Levels Tune" => &[(CHARACTER, 0.0), (LEVELS, 1.0), (LEVELS_PAD, 13.0)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

/// A preset's pads: the built-in kit, tuned and shaped to fit (Empty: none).
pub fn preset_pads(name: &str) -> Vec<GritPad> {
    let mut pads = default_pads();
    match name {
        "Empty" => pads = vec![GritPad::default(); PADS],
        "Boom Bap 26k" => {
            pads[0].tune = -2.0;
            pads[1].tune = -3.0;
            pads[4].choke = 1;
            pads[5].choke = 1;
            pads[12].tune = -5.0;
        }
        "Lo-Fi 8-bit" => {
            for p in pads.iter_mut() {
                p.tune -= 4.0;
                p.filter = 0.8;
            }
        }
        "Swung Repeats" => {
            pads[4].choke = 1;
            pads[5].choke = 1;
            pads[4].decay = 0.5;
        }
        _ => {
            pads[4].choke = 1;
            pads[5].choke = 1;
        }
    }
    pads
}

/// The pads to a whole number: the sixteen there are (empty ones added).
pub fn fix_pads(pads: &mut Vec<GritPad>) {
    pads.truncate(PADS);
    while pads.len() < PADS {
        pads.push(GritPad::default());
    }
}

/// How a loop is chopped to the pads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chop {
    /// At its hits (the strongest sixteen, in order).
    Transients,
    /// In sixteen equal parts.
    Grid,
}

/// A loop chopped to the pads from the first: at its hits or on a grid,
/// each a part of the file (so a set keeps it as the file and the part),
/// faded in over 1 ms and out over 4. Takes a moment for a long sample, so
/// never on the audio thread.
pub fn chop(s: &SampleRef, by: Chop) -> Vec<GritPad> {
    let n = s.frames();
    let sr = s.sample_rate as f64;
    let pre = (0.001 * sr) as usize;
    let mut starts: Vec<usize> = match by {
        Chop::Grid => (0..PADS).map(|k| k * n / PADS).collect(),
        Chop::Transients => {
            let an = s.analysis();
            let mut hits: Vec<(usize, f32)> = an.transients.iter().copied().zip(an.strengths.iter().copied().chain(std::iter::repeat(1.0))).collect();
            if hits.len() > PADS {
                hits.sort_by(|a, b| b.1.total_cmp(&a.1));
                hits.truncate(PADS);
            }
            let mut v: Vec<usize> = hits.into_iter().map(|(h, _)| h.saturating_sub(pre)).collect();
            v.sort_unstable();
            if v.first().is_none_or(|&f| f as f64 > 0.01 * sr) && v.len() < PADS {
                v.insert(0, 0);
            }
            v
        }
    };
    starts.dedup();
    let (fade_in, fade_out) = ((0.001 * sr) as u32, (0.004 * sr) as u32);
    let mut pads: Vec<GritPad> = (0..starts.len())
        .map(|k| {
            let end = starts.get(k + 1).copied().unwrap_or(n);
            let region = SampleRegion { start: starts[k] as u64, end: end as u64, fade_in, fade_out };
            let part = SampleRef::new(s.region(format!("{} {}", s.name, k + 1), region));
            GritPad { name: format!("Chop {}", k + 1), sample: Some(part), gate: false, ..GritPad::default() }
        })
        .collect();
    fix_pads(&mut pads);
    pads
}

/// Where a pad's sound is read from: its sample, or a built-in sound.
fn source<'a>(pad: &'a GritPad, table: Option<&'a Vec<Vec<f32>>>) -> Option<(&'a [f32], &'a [f32], f32)> {
    if let Some(s) = &pad.sample {
        let d: &SampleData = s;
        return (d.frames() > 0).then_some((&d.left[..], &d.right[..], d.sample_rate as f32));
    }
    let v = table?.get(pad.sound? as usize)?;
    Some((&v[..], &v[..], SOUND_RATE))
}

#[derive(Clone, Copy, Debug)]
struct Voice {
    active: bool,
    pad: usize,
    note: u8,
    /// Where it reads (frames of its source) and how far a tick of the machine moves it.
    pos: f64,
    step: f64,
    reverse: bool,
    env: f32,
    attack: f32,
    decay: f32,
    /// Letting go (gate off, choked): to silence by this much a tick.
    release: f32,
    gain_l: f32,
    gain_r: f32,
    choke: u8,
    gate: bool,
    filter: Option<f32>,
    age: u64,
}

impl Voice {
    const IDLE: Voice = Voice {
        active: false,
        pad: 0,
        note: 0,
        pos: 0.0,
        step: 1.0,
        reverse: false,
        env: 0.0,
        attack: 1.0,
        decay: 1.0,
        release: 0.0,
        gain_l: 0.0,
        gain_r: 0.0,
        choke: 0,
        gate: false,
        filter: None,
        age: 0,
    };
}

/// A pad held with Repeat on: the next step of the grid it plays on.
#[derive(Clone, Copy, Debug)]
struct Held {
    note: u8,
    vel: f32,
    next: u64,
}

pub struct GritState {
    sr: f32,
    voices: [Voice; VOICES],
    filters: [[Svf; 2]; VOICES],
    out_filters: [[Svf; 2]; 2],
    clock: f64,
    hold: (f32, f32),
    rng: Rng,
    age: u64,
    /// The song's tempo and beat (at the block's start, counted on), and whether it plays.
    pub bpm: f32,
    beat: f64,
    playing: bool,
    held: [Option<Held>; PADS],
    /// Notes waiting for the block: what the pads play is set at render
    /// (the pads are there then).
    pending: [Option<(u8, f32)>; 32],
    releases: [Option<u8>; 32],
    level: f32,
    lit: [f32; PADS],
    character: usize,
}

impl GritState {
    pub fn new(sr: f32) -> Self {
        Self {
            sr,
            voices: [Voice::IDLE; VOICES],
            filters: [[Svf::new(); 2]; VOICES],
            out_filters: [[Svf::new(), Svf::new()], [Svf::new(), Svf::new()]],
            clock: 0.0,
            hold: (0.0, 0.0),
            rng: Rng::new(0x6121_7A11),
            age: 0,
            bpm: 120.0,
            beat: 0.0,
            playing: false,
            held: [None; PADS],
            pending: [None; 32],
            releases: [None; 32],
            level: 0.0,
            lit: [0.0; PADS],
            character: 0,
        }
    }

    pub fn set_transport(&mut self, bpm: f32, beat: f64, playing: bool) {
        self.bpm = bpm.clamp(20.0, 999.0);
        if playing {
            self.beat = beat;
        }
        self.playing = playing;
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        if let Some(slot) = self.pending.iter_mut().find(|s| s.is_none()) {
            *slot = Some((note, vel));
        }
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        if let Some(slot) = self.releases.iter_mut().find(|s| s.is_none()) {
            *slot = Some(note);
        }
    }

    pub fn all_off(&mut self) {
        self.held = [None; PADS];
        self.pending = [None; 32];
        for v in self.voices.iter_mut().filter(|v| v.active) {
            v.release = v.release.max(0.02);
        }
    }

    /// For the face: the voices playing, the level, how lit each pad is,
    /// the pads held with Repeat (a bit each), the machine's rate.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.voices.iter().filter(|v| v.active).count() as f32;
        out[1] = self.level;
        out[2..2 + PADS].copy_from_slice(&self.lit);
        out[18] = self.held.iter().enumerate().filter(|(_, h)| h.is_some()).fold(0u32, |m, (k, _)| m | 1 << k) as f32;
        out[19] = self.character as f32;
    }

    /// The step of the repeat grid at or after `beat` (steps of `len`
    /// beats, every second one late by the swing).
    fn grid_index(beat: f64, len: f64, swing: f64) -> u64 {
        let pair = 2.0 * len;
        let base = (beat / pair).floor().max(0.0);
        let at = beat - base * pair;
        let n = base as u64 * 2;
        if at <= 1e-9 {
            n
        } else if at <= pair * swing + 1e-9 {
            n + 1
        } else {
            n + 2
        }
    }

    fn grid_beat(n: u64, len: f64, swing: f64) -> f64 {
        let pair = 2.0 * len;
        (n / 2) as f64 * pair + if n % 2 == 1 { pair * swing } else { 0.0 }
    }

    /// Starts pad `note` (with 16 Levels, the levels pad in step `note`'s place).
    fn trigger(&mut self, p: &[f32], pads: &[GritPad], note: u8, vel: f32, table: Option<&Vec<Vec<f32>>>, rate: f32) {
        let Some(k) = (note as usize).checked_sub(FIRST_NOTE as usize).filter(|&k| k < PADS) else { return };
        let levels = (p[LEVELS].round().max(0.0) as usize).min(LEVEL_MODES.len() - 1);
        let from = if levels > 0 { (p[LEVELS_PAD].round() as usize).clamp(1, PADS) - 1 } else { k };
        let Some(pad) = pads.get(from) else { return };
        let Some((src, _, src_rate)) = source(pad, table) else { return };
        let mut tune = pad.tune;
        let mut level = db_to_gain(pad.level_db);
        let mut decay = pad.decay;
        let mut filter = pad.filter;
        let stepv = (k + 1) as f32 / PADS as f32;
        match levels {
            1 => tune += k as f32 - 8.0,
            2 => level *= stepv,
            3 => decay = stepv,
            4 => filter = stepv,
            _ => {}
        }
        self.lit[k] = 1.0;
        // A choke group cuts the others of its group off (and a pad itself, retriggered).
        if pad.choke > 0 {
            for v in self.voices.iter_mut().filter(|v| v.active && v.choke == pad.choke) {
                v.release = v.release.max(1.0 / (0.002 * rate));
            }
        }
        let slot = self.voices.iter().position(|v| !v.active).unwrap_or_else(|| (0..VOICES).min_by_key(|&i| self.voices[i].age).unwrap_or(0));
        self.age += 1;
        let n = src.len() as f64;
        let step = src_rate as f64 / rate as f64 * 2f64.powf(tune as f64 / 12.0);
        let pan = pad.pan.clamp(-1.0, 1.0);
        let gain = level * vel.clamp(0.0, 1.0);
        let decay_secs = 0.02 * 250f32.powf(decay.clamp(0.0, 1.0));
        let filter = (filter < 0.999).then(|| 80.0 * 2f32.powf(filter.clamp(0.0, 1.0) * 8.0));
        if let Some(c) = filter {
            for f in self.filters[slot].iter_mut() {
                *f = Svf::new();
                f.set(c.min(rate * 0.45), 0.15, rate);
            }
        }
        self.voices[slot] = Voice {
            active: true,
            pad: from,
            note,
            pos: if pad.reverse { n - 1.0 } else { 0.0 },
            step,
            reverse: pad.reverse,
            env: 0.0,
            attack: 1.0 / (0.0005 * rate).max(1.0),
            decay: if decay >= 0.999 { 1.0 } else { (-1.0 / (decay_secs * rate)).exp() },
            release: 0.0,
            gain_l: gain * (1.0 - pan).min(1.0),
            gain_r: gain * (1.0 + pan).min(1.0),
            choke: pad.choke,
            gate: pad.gate,
            filter,
            age: self.age,
        };
    }

    pub fn render(&mut self, p: &[f32], pads: &[GritPad], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let table = SOUND_TABLE.get();
        let ch = (p[CHARACTER].round().max(0.0) as usize).min(MACHINES.len() - 1);
        let m = &MACHINES[ch];
        let rate = if m.rate > 0.0 { m.rate } else { self.sr };
        if ch != self.character {
            self.character = ch;
            self.out_filters = [[Svf::new(), Svf::new()], [Svf::new(), Svf::new()]];
        }
        if m.filter > 0.0 {
            for f in self.out_filters.iter_mut().flatten() {
                f.set(m.filter.min(self.sr * 0.45), 0.0, self.sr);
            }
        }
        let steps = if m.bits > 0 { (1u32 << (m.bits - 1)) as f32 } else { 0.0 };
        let drive = p[DRIVE].clamp(0.0, 1.0);
        let drive_gain = 1.0 + drive * 7.0;
        let drive_norm = 1.0 / drive_gain.tanh();
        let quant = |x: f32| if steps > 0.0 { (x.clamp(-1.0, 1.0 - 1.0 / steps) * steps).round() / steps } else { x };
        let repeat = REPEAT_BEATS[(p[REPEAT].round().max(0.0) as usize).min(REPEAT_BEATS.len() - 1)];
        let swing = p[SWING].clamp(0.5, 0.75) as f64;
        // Notes from the block's start: started (and a repeat's grid found), or let go.
        for i in 0..self.pending.len() {
            let Some((note, vel)) = self.pending[i].take() else { continue };
            self.trigger(p, pads, note, vel, table, rate);
            if let Some(k) = (note as usize).checked_sub(FIRST_NOTE as usize).filter(|&k| k < PADS) {
                if repeat > 0.0 {
                    let next = Self::grid_index(self.beat + 1e-6, repeat, swing);
                    self.held[k] = Some(Held { note, vel, next });
                }
            }
        }
        for i in 0..self.releases.len() {
            let Some(note) = self.releases[i].take() else { continue };
            if let Some(k) = (note as usize).checked_sub(FIRST_NOTE as usize).filter(|&k| k < PADS) {
                self.held[k] = None;
            }
            for v in self.voices.iter_mut().filter(|v| v.active && v.note == note && v.gate) {
                v.release = v.release.max(1.0 / (0.005 * rate));
            }
        }
        if repeat == 0.0 {
            self.held = [None; PADS];
        }
        let volume = db_to_gain(p[VOLUME]);
        let beat_step = self.bpm as f64 / 60.0 / self.sr as f64;
        let tick = rate as f64 / self.sr as f64;
        let mut peak = 0.0f32;
        for i in 0..n {
            // Repeats on their grid.
            if repeat > 0.0 {
                for k in 0..PADS {
                    let Some(h) = self.held[k] else { continue };
                    if self.beat + 1e-9 >= Self::grid_beat(h.next, repeat, swing) {
                        self.trigger(p, pads, h.note, h.vel, table, rate);
                        if let Some(hh) = self.held[k].as_mut() {
                            hh.next = Self::grid_index(self.beat + 1e-6, repeat, swing).max(h.next + 1);
                        }
                    }
                }
            }
            self.beat += beat_step;
            // The machine's ticks: every voice read once, summed, converted.
            self.clock += tick;
            while self.clock >= 1.0 {
                self.clock -= 1.0;
                let (mut sl, mut sr) = (0.0f32, 0.0f32);
                for vi in 0..VOICES {
                    let v = self.voices[vi];
                    if !v.active {
                        continue;
                    }
                    let Some((left, right, _)) = pads.get(v.pad).and_then(|pad| source(pad, table)) else {
                        self.voices[vi].active = false;
                        continue;
                    };
                    let len = left.len().min(right.len());
                    if v.pos < 0.0 || v.pos >= len as f64 {
                        self.voices[vi].active = false;
                        continue;
                    }
                    let i0 = v.pos as usize;
                    let (a, b) = if m.interpolate {
                        let f = (v.pos - i0 as f64) as f32;
                        let j = if v.reverse { i0.saturating_sub(1) } else { (i0 + 1).min(len - 1) };
                        let f = if v.reverse { 1.0 - f } else { f };
                        let (i0, j) = if v.reverse { (j, i0) } else { (i0, j) };
                        (left[i0] + (left[j] - left[i0]) * f, right[i0] + (right[j] - right[i0]) * f)
                    } else {
                        (left[i0], right[i0])
                    };
                    // As it was sampled: driven, in the machine's bits.
                    let (a, b) = if drive > 0.0 { ((a * drive_gain).tanh() * drive_norm, (b * drive_gain).tanh() * drive_norm) } else { (a, b) };
                    let (a, b) = (quant(a), quant(b));
                    let vv = &mut self.voices[vi];
                    vv.env = if vv.release > 0.0 { (vv.env - vv.release).max(0.0) } else { (vv.env + vv.attack).min(1.0) };
                    if vv.release == 0.0 && vv.env >= 1.0 {
                        vv.attack = 0.0;
                    }
                    if vv.attack == 0.0 && vv.release == 0.0 {
                        vv.env *= vv.decay;
                    }
                    let mut e = vv.env;
                    if vv.release > 0.0 && vv.env <= 0.0 || e < 1e-4 && vv.attack == 0.0 {
                        vv.active = false;
                        e = 0.0;
                    }
                    let (mut a, mut b) = (a * e, b * e);
                    if vv.filter.is_some() {
                        a = self.filters[vi][0].process(a, FilterMode::Low);
                        b = self.filters[vi][1].process(b, FilterMode::Low);
                    }
                    let vv = &mut self.voices[vi];
                    sl += a * vv.gain_l;
                    sr += b * vv.gain_r;
                    vv.pos += if vv.reverse { -vv.step } else { vv.step };
                }
                // Out of the converter, its noise in its last step.
                let noise = if m.noise > 0.0 && steps > 0.0 { m.noise / steps } else { 0.0 };
                let (nl, nr) = if noise > 0.0 { (self.rng.bipolar() * noise, self.rng.bipolar() * noise) } else { (0.0, 0.0) };
                self.hold = (quant(sl + nl), quant(sr + nr));
            }
            let (mut ol, mut or) = self.hold;
            for k in 0..m.poles {
                ol = self.out_filters[0][k].process(ol, FilterMode::Low);
                or = self.out_filters[1][k].process(or, FilterMode::Low);
            }
            ol *= volume;
            or *= volume;
            if !ol.is_finite() || !or.is_finite() {
                ol = 0.0;
                or = 0.0;
                self.out_filters = [[Svf::new(), Svf::new()], [Svf::new(), Svf::new()]];
                self.filters = [[Svf::new(); 2]; VOICES];
            }
            l[i] += ol;
            r[i] += or;
            peak = peak.max(ol.abs()).max(or.abs());
        }
        let fade = (-(n as f32) / (0.15 * self.sr)).exp();
        self.lit.iter_mut().for_each(|x| *x *= fade);
        self.level = peak.max(self.level * 0.9);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn sample(v: Vec<f32>, rate: u32) -> SampleRef {
        SampleRef::new(SampleData::from_buffers("test", None, rate, v.clone(), v))
    }

    fn sine(freq: f32, amp: f32, secs: f32) -> SampleRef {
        let n = (SR * secs) as usize;
        sample((0..n).map(|i| (std::f32::consts::TAU * freq * i as f32 / SR).sin() * amp).collect(), SR as u32)
    }

    fn pads_with(s: &SampleRef) -> Vec<GritPad> {
        let mut pads = vec![GritPad::default(); PADS];
        pads[0] = GritPad { sample: Some(s.clone()), ..GritPad::default() };
        pads
    }

    fn params(ch: usize) -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        p[CHARACTER] = ch as f32;
        p[VOLUME] = 0.0;
        p
    }

    fn play(st: &mut GritState, p: &[f32], pads: &[GritPad], secs: f32) -> Vec<f32> {
        let mut out = Vec::new();
        for _ in 0..(secs * SR / 128.0) as usize {
            let (mut l, mut r) = ([0.0f32; 128], [0.0f32; 128]);
            st.render(p, pads, &mut l, &mut r);
            out.extend_from_slice(&l);
        }
        out
    }

    /// The strength of `freq` in `x` (Goertzel).
    fn power(x: &[f32], freq: f32) -> f32 {
        let w = std::f32::consts::TAU * freq / SR;
        let c = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f32, 0.0f32);
        for &v in x {
            let s = v + c * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        (s1 * s1 + s2 * s2 - c * s1 * s2).max(0.0).sqrt() / x.len() as f32
    }

    #[test]
    fn the_old_rates_alias_and_clean_does_not() {
        // A 7 kHz sine read at 26.04 kHz with no filter in or out: its alias
        // at 26.04 - 7 = 19.04 kHz is there; at the host's rate it is not.
        let s = sine(7000.0, 0.5, 1.0);
        let pads = pads_with(&s);
        let mut alias = [0.0f32; 2];
        for (k, ch) in [0usize, 4].into_iter().enumerate() {
            let mut st = GritState::new(SR);
            st.note_on(&params(ch), FIRST_NOTE, 1.0);
            let x = play(&mut st, &params(ch), &pads, 0.5);
            let x = &x[2000..20000];
            let main = power(x, 7000.0);
            assert!(main > 0.1, "{ch}: the tone itself {main}");
            alias[k] = power(x, 19_040.0) / main;
        }
        assert!(alias[0] > 0.2, "aliasing at 26k: {}", alias[0]);
        assert!(alias[1] < 0.01, "none when clean: {}", alias[1]);
    }

    #[test]
    fn the_bits_are_steps() {
        // A quiet slow sine: twelve bits give it some forty steps, eight a few.
        let s = sine(50.0, 0.01, 1.0);
        let pads = pads_with(&s);
        let mut counts = Vec::new();
        for ch in [0usize, 3, 4] {
            let mut st = GritState::new(SR);
            st.note_on(&params(ch), FIRST_NOTE, 1.0);
            let x = play(&mut st, &params(ch), &pads, 0.5);
            let mut vals: Vec<i64> = x[4800..].iter().map(|v| (v * 1e7).round() as i64).collect();
            vals.sort_unstable();
            vals.dedup();
            counts.push(vals.len());
            if ch == 0 {
                // Each a whole number of steps of 1/2048.
                assert!(x[4800..].iter().all(|v| ((v * 2048.0) - (v * 2048.0).round()).abs() < 1e-3), "off the steps");
            }
        }
        assert!((35..=50).contains(&counts[0]), "12 bits: {}", counts[0]);
        // (Its own output filter smooths the steps; out of its converter they are a few.)
        assert!(counts[1] > 8, "8 bits, filtered: {}", counts[1]);
        let mut st = GritState::new(SR);
        let p = params(3);
        st.note_on(&p, FIRST_NOTE, 1.0);
        let mut held = Vec::new();
        for _ in 0..24_000 {
            let (mut l, mut r) = ([0.0f32; 1], [0.0f32; 1]);
            st.render(&p, &pads, &mut l, &mut r);
            held.push((st.hold.0 * 128.0).round() as i64);
            assert!(((st.hold.0 * 128.0) - (st.hold.0 * 128.0).round()).abs() < 1e-4, "off the steps of 8 bits");
        }
        held.sort_unstable();
        held.dedup();
        assert!(held.len() <= 5, "8 bits: {held:?}");
        assert!(counts[2] > 1000, "clean: {}", counts[2]);
    }

    #[test]
    fn pitching_up_drops_samples() {
        // Pitched up a fifth on the 26k machine: every value it plays is one
        // of the sample's own (in its steps), none in between.
        let s = sine(300.0, 0.6, 1.0);
        let mut pads = pads_with(&s);
        pads[0].tune = 7.0;
        let p = params(0);
        let mut own: Vec<i64> = s.left.iter().map(|v| ((v.clamp(-1.0, 1.0 - 1.0 / 2048.0) * 2048.0).round()) as i64).collect();
        own.sort_unstable();
        own.dedup();
        let mut st = GritState::new(SR);
        st.note_on(&p, FIRST_NOTE, 1.0);
        let x = play(&mut st, &p, &pads, 0.3);
        // Its step: 48/26.04 of a sample a tick, times a fifth.
        let step = st.voices.iter().find(|v| v.active).map(|v| v.step).unwrap();
        assert!((step - 48_000.0 / 26_040.0 * 2f64.powf(7.0 / 12.0)).abs() < 1e-9);
        for v in &x[2000..] {
            let q = (v * 2048.0).round() as i64;
            assert!(own.binary_search(&q).is_ok(), "{v} is not one of the sample's values");
        }
        // And the pitch is up a fifth: 449.5 Hz.
        let ups = (1..x.len()).filter(|&i| i > 2000 && x[i - 1] <= 0.0 && x[i] > 0.0).count() as f32;
        let f = ups / ((x.len() - 2000) as f32 / SR);
        assert!((f - 300.0 * 2f32.powf(7.0 / 12.0)).abs() < 6.0, "{f}");
    }

    #[test]
    fn note_repeat_keeps_the_grid_and_its_swing() {
        // A short click: each repeat is an onset.
        let mut v = vec![0.0f32; 4800];
        v[..120].iter_mut().for_each(|x| *x = 0.8);
        let s = sample(v, SR as u32);
        let pads = pads_with(&s);
        for (rate, swing, beats) in [(4.0f32, 0.5f32, [0.25, 0.25]), (2.0, 0.5, [0.5, 0.5]), (4.0, 0.66, [0.33, 0.17]), (5.0, 0.5, [1.0 / 6.0, 1.0 / 6.0])] {
            let mut p = params(4);
            p[REPEAT] = rate;
            p[SWING] = swing;
            let mut st = GritState::new(SR);
            st.set_transport(120.0, 0.0, true);
            st.note_on(&p, FIRST_NOTE, 1.0);
            let x = play(&mut st, &p, &pads, 2.0);
            let onsets: Vec<f32> = (1..x.len()).filter(|&i| x[i - 1].abs() < 0.05 && x[i].abs() >= 0.05).map(|i| i as f32 / SR).collect();
            assert!(onsets.len() > 6, "{onsets:?}");
            let beat = 0.5;
            for (k, w) in onsets.windows(2).take(6).enumerate() {
                let want = beats[k % 2] as f32 * beat;
                assert!((w[1] - w[0] - want).abs() < 0.002, "repeat {rate} swing {swing}: gap {} not {want}", w[1] - w[0]);
            }
            // Let go: no more.
            st.note_off(&p, FIRST_NOTE);
            play(&mut st, &p, &pads, 0.05);
            let after = play(&mut st, &p, &pads, 1.0);
            assert!(after.iter().all(|v| v.abs() < 1e-6));
        }
    }

    #[test]
    fn chop_puts_the_pads_at_the_hits() {
        // Six hits at known places in a two-second loop.
        let n = (SR * 2.0) as usize;
        let hits = [0usize, 13_000, 29_500, 41_000, 60_200, 77_000];
        let mut v = vec![0.0f32; n];
        for &h in &hits {
            for (k, x) in v[h..(h + 2000).min(n)].iter_mut().enumerate() {
                *x = (k as f32 * 0.4).sin() * (-(k as f32) / 300.0).exp() * 0.9;
            }
        }
        let s = sample(v, SR as u32);
        let pads = chop(&s, Chop::Transients);
        assert_eq!(pads.len(), PADS);
        let starts: Vec<u64> = pads.iter().filter_map(|p| p.sample.as_ref().and_then(|s| s.region).map(|r| r.start)).collect();
        assert_eq!(starts.len(), hits.len(), "{starts:?}");
        for (&s, &h) in starts.iter().zip(&hits) {
            assert!((s as i64 - h as i64).abs() < 200, "a pad at {s} for a hit at {h}");
        }
        // The grid: sixteen equal parts.
        let grid = chop(&s, Chop::Grid);
        assert!(grid.iter().all(|p| p.sample.is_some()));
        assert_eq!(grid[4].sample.as_ref().unwrap().region.unwrap().start, (4 * n / PADS) as u64);
    }

    #[test]
    fn sixteen_levels_tunes_one_sample_across_the_pads() {
        let s = sine(200.0, 0.5, 1.0);
        let pads = pads_with(&s);
        let mut p = params(4);
        p[LEVELS] = 1.0;
        p[LEVELS_PAD] = 1.0;
        let mut st = GritState::new(SR);
        st.note_on(&p, FIRST_NOTE + 12, 1.0);
        play(&mut st, &p, &pads, 0.01);
        let v = st.voices.iter().find(|v| v.active).unwrap();
        assert_eq!(v.pad, 0);
        assert!((v.step - 2f64.powf(4.0 / 12.0)).abs() < 1e-9);
    }

    #[test]
    fn choke_and_gate_cut_it_off() {
        let s = sine(200.0, 0.5, 2.0);
        let mut pads = pads_with(&s);
        pads[1] = pads[0].clone();
        pads[0].choke = 1;
        pads[1].choke = 1;
        pads[2] = GritPad { gate: true, ..pads[0].clone() };
        pads[2].choke = 0;
        let p = params(4);
        let mut st = GritState::new(SR);
        st.note_on(&p, FIRST_NOTE, 1.0);
        play(&mut st, &p, &pads, 0.1);
        st.note_on(&p, FIRST_NOTE + 1, 1.0);
        play(&mut st, &p, &pads, 0.1);
        assert_eq!(st.voices.iter().filter(|v| v.active).count(), 1);
        st.note_on(&p, FIRST_NOTE + 2, 1.0);
        play(&mut st, &p, &pads, 0.1);
        st.note_off(&p, FIRST_NOTE + 2);
        play(&mut st, &p, &pads, 0.1);
        assert_eq!(st.voices.iter().filter(|v| v.active).count(), 1);
    }

    #[test]
    fn every_preset_sounds_and_stays_in_bounds() {
        warm();
        for name in preset_names() {
            let p = preset(name);
            let mut pads = preset_pads(name);
            assert_eq!(p.len(), SPECS.len());
            assert_eq!(pads.len(), PADS);
            if *name == "Empty" {
                pads[0] = GritPad::builtin(0);
            }
            let mut st = GritState::new(SR);
            st.set_transport(96.0, 0.0, true);
            let mut out = Vec::new();
            for k in 0..PADS {
                st.note_on(&p, FIRST_NOTE + k as u8, 0.9);
                out.extend(play(&mut st, &p, &pads, 0.15));
                st.note_off(&p, FIRST_NOTE + k as u8);
            }
            let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(out.iter().all(|v| v.is_finite()), "{name}");
            assert!(peak > 0.05 && peak < 2.0, "{name}: {peak}");
        }
    }

    #[test]
    fn nothing_blows_up() {
        warm();
        let s = sine(900.0, 0.9, 0.5);
        let mut rng = Rng::new(3);
        for _ in 0..10 {
            let p: Vec<f32> = SPECS.iter().map(|s| s.from_norm(rng.unipolar())).collect();
            let mut pads = default_pads();
            for pad in pads.iter_mut() {
                pad.tune = rng.bipolar() * 24.0;
                pad.decay = rng.unipolar();
                pad.filter = rng.unipolar();
                pad.reverse = rng.unipolar() < 0.5;
                pad.gate = rng.unipolar() < 0.5;
                pad.choke = (rng.next_u32() % 5) as u8;
                pad.pan = rng.bipolar();
                if rng.unipolar() < 0.3 {
                    pad.sample = Some(s.clone());
                }
            }
            let mut st = GritState::new(44_100.0);
            st.set_transport(140.0, 0.0, true);
            for k in 0..200 {
                if k % 3 == 0 {
                    st.note_on(&p, 30 + (rng.next_u32() % 30) as u8, rng.unipolar());
                }
                if k % 5 == 0 {
                    st.note_off(&p, 30 + (rng.next_u32() % 30) as u8);
                }
                let (mut l, mut r) = ([0.0f32; 64], [0.0f32; 64]);
                st.render(&p, &pads, &mut l, &mut r);
                assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 8.0));
            }
        }
    }
}
