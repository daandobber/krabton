//! Silicon: a synth that sounds on the emulated DSP56300 (the chip Hyper's
//! Virus runs on) with Krabton's own firmware, `silicon.asm`: no Virus
//! firmware, nothing of Access's. Every sample is worked out on the chip:
//! per voice a saw turning into a square, a second saw, a sub, a state
//! variable filter swept by its envelope, and an amplifier envelope.
//!
//! This side is the chip's host, as a microcontroller would be: it takes the
//! notes and picks the voices, glides their pitch, moves the envelopes from
//! attack to decay to release, works out the knobs (the LFO too, once a
//! block), writes it all into the chip's X memory, runs the firmware for a
//! block and reads the samples back from Y memory.

use super::{db_to_gain, midi_to_freq};
use crate::params::{ParamSpec, Unit};
use krabton_virus::chip::{self, frac, to_frac, Area, Chip, Program};
use std::sync::OnceLock;

pub const SHAPE: usize = 0;
pub const OSC2: usize = 1;
pub const DETUNE: usize = 2;
pub const SEMITONE: usize = 3;
pub const SUB: usize = 4;
pub const CUTOFF: usize = 5;
pub const RESONANCE: usize = 6;
pub const FILTER: usize = 7;
pub const ENV_AMT: usize = 8;
pub const F_DECAY: usize = 9;
pub const KEY_TRACK: usize = 10;
pub const ATTACK: usize = 11;
pub const DECAY: usize = 12;
pub const SUSTAIN: usize = 13;
pub const RELEASE: usize = 14;
pub const LFO_RATE: usize = 15;
pub const LFO_PITCH: usize = 16;
pub const LFO_CUTOFF: usize = 17;
pub const GLIDE: usize = 18;
pub const VOICES: usize = 19;
pub const VELOCITY: usize = 20;
pub const VOLUME: usize = 21;

pub const FILTERS: &[&str] = &["Low Pass", "Band Pass", "High Pass"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Shape", "Oscillators", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Osc 2", "Oscillators", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Detune", "Oscillators", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::int("Semitone", "Oscillators", -12.0, 12.0, 0.0, Unit::Semi),
    ParamSpec::lin("Sub", "Oscillators", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Cutoff", "Filter", 40.0, 15_000.0, 1800.0, Unit::Hz),
    ParamSpec::lin("Resonance", "Filter", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::choice("Filter", "Filter", FILTERS, 0),
    ParamSpec::lin("Env Amount", "Filter", -1.0, 1.0, 0.5, Unit::None),
    ParamSpec::log("Env Decay", "Filter", 0.01, 5.0, 0.3, Unit::Sec),
    ParamSpec::lin("Key Track", "Filter", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::log("Attack", "Amp", 0.001, 5.0, 0.005, Unit::Sec),
    ParamSpec::log("Decay", "Amp", 0.01, 5.0, 0.4, Unit::Sec),
    ParamSpec::lin("Sustain", "Amp", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::log("Release", "Amp", 0.01, 5.0, 0.3, Unit::Sec),
    ParamSpec::log("LFO Rate", "LFO", 0.05, 20.0, 4.0, Unit::Hz),
    ParamSpec::lin("LFO Pitch", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("LFO Cutoff", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Glide", "Play", 0.001, 1.0, 0.001, Unit::Sec),
    ParamSpec::int("Voices", "Play", 1.0, 8.0, 8.0, Unit::None),
    ParamSpec::lin("Velocity", "Play", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Volume", "Play", -30.0, 6.0, -4.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Init", "Chip Bass", "Square Lead", "Silicon Pad", "Resonant Pluck", "Hollow", "Sub Drone", "Sync Sweep", "Bright Brass", "Wobble"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Chip Bass" => &[(SHAPE, 0.0), (OSC2, 0.4), (DETUNE, 0.1), (SUB, 0.5), (CUTOFF, 300.0), (RESONANCE, 0.55), (ENV_AMT, 0.6), (F_DECAY, 0.2), (DECAY, 0.3), (SUSTAIN, 0.6), (RELEASE, 0.08), (VOICES, 1.0), (GLIDE, 0.05)],
        "Square Lead" => &[(SHAPE, 1.0), (OSC2, 0.3), (DETUNE, 0.2), (SEMITONE, 12.0), (CUTOFF, 3500.0), (RESONANCE, 0.25), (ENV_AMT, 0.25), (F_DECAY, 0.5), (SUSTAIN, 0.9), (RELEASE, 0.2), (LFO_RATE, 5.5), (LFO_PITCH, 0.12), (VOICES, 1.0), (GLIDE, 0.04)],
        "Silicon Pad" => &[(SHAPE, 0.3), (OSC2, 0.8), (DETUNE, 0.35), (CUTOFF, 1400.0), (RESONANCE, 0.2), (ENV_AMT, 0.15), (F_DECAY, 1.5), (ATTACK, 0.6), (DECAY, 1.0), (SUSTAIN, 0.85), (RELEASE, 1.5), (LFO_RATE, 0.3), (LFO_CUTOFF, 0.25)],
        "Resonant Pluck" => &[(SHAPE, 0.0), (OSC2, 0.3), (CUTOFF, 250.0), (RESONANCE, 0.8), (ENV_AMT, 0.8), (F_DECAY, 0.18), (DECAY, 0.5), (SUSTAIN, 0.0), (RELEASE, 0.3)],
        "Hollow" => &[(SHAPE, 0.8), (OSC2, 0.0), (FILTER, 1.0), (CUTOFF, 900.0), (RESONANCE, 0.5), (ENV_AMT, 0.3), (F_DECAY, 0.6), (SUSTAIN, 0.8), (LFO_RATE, 0.5), (LFO_CUTOFF, 0.3)],
        "Sub Drone" => &[(SHAPE, 0.5), (OSC2, 0.6), (DETUNE, 0.5), (SEMITONE, -12.0), (SUB, 0.8), (CUTOFF, 500.0), (RESONANCE, 0.4), (ENV_AMT, 0.0), (ATTACK, 1.5), (SUSTAIN, 1.0), (RELEASE, 2.5), (LFO_RATE, 0.15), (LFO_CUTOFF, 0.4)],
        "Sync Sweep" => &[(SHAPE, 0.0), (OSC2, 0.7), (SEMITONE, 7.0), (DETUNE, 0.05), (CUTOFF, 200.0), (RESONANCE, 0.7), (ENV_AMT, 0.9), (F_DECAY, 1.2), (SUSTAIN, 0.9), (VOICES, 1.0)],
        "Bright Brass" => &[(SHAPE, 0.0), (OSC2, 0.7), (DETUNE, 0.25), (CUTOFF, 700.0), (RESONANCE, 0.15), (ENV_AMT, 0.55), (F_DECAY, 0.5), (ATTACK, 0.04), (SUSTAIN, 0.85), (RELEASE, 0.25)],
        "Wobble" => &[(SHAPE, 0.6), (OSC2, 0.5), (SEMITONE, -12.0), (SUB, 0.4), (CUTOFF, 350.0), (RESONANCE, 0.6), (ENV_AMT, 0.0), (SUSTAIN, 1.0), (RELEASE, 0.1), (LFO_RATE, 3.0), (LFO_CUTOFF, 0.8), (VOICES, 1.0)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

/// The firmware's source.
pub static FIRMWARE: &str = include_str!("silicon.asm");
/// Where it is put in P memory.
const ORIGIN: u32 = 0x100;
/// The chip's memories, each this many words.
const MEMORY: u32 = 0x1000;
pub const MAX_VOICES: usize = 8;
/// The most samples the firmware makes in one go.
const MAX_BLOCK: usize = 256;

// Where the firmware reads the knobs (X memory).
const G_N: u32 = 0x00;
const G_VOICES: u32 = 0x0b;
// A voice's sixteen words, in X from V_BASE on.
const V_BASE: u32 = 0x100;
const V_WORDS: u32 = 16;
const W_ACTIVE: u32 = 0;
const W_PH1: u32 = 1;
const W_INC1: u32 = 2;
const W_PH2: u32 = 3;
const W_INC2: u32 = 4;
const W_PHS: u32 = 5;
const W_INCS: u32 = 6;
const W_ENV: u32 = 7;
/// Its envelope's target, and (next) its rate.
const W_TARGET: u32 = 9;
const W_FENV: u32 = 11;
const W_LOW: u32 = 12;
const W_VEL: u32 = 14;
const W_CUT: u32 = 15;

/// The firmware assembled (once): its words, and where it starts and waits.
pub fn firmware() -> Result<&'static Program, &'static str> {
    static PROGRAM: OnceLock<Result<Program, String>> = OnceLock::new();
    PROGRAM.get_or_init(|| chip::assemble(FIRMWARE, ORIGIN)).as_ref().map_err(|e| e.as_str())
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Stage {
    Off,
    Attack,
    Decay,
    Release,
}

#[derive(Clone, Copy)]
struct Voice {
    note: u8,
    gate: bool,
    age: u64,
    pitch: f32,
    target: f32,
    stage: Stage,
}

impl Voice {
    const OFF: Voice = Voice { note: 0, gate: false, age: 0, pitch: 60.0, target: 60.0, stage: Stage::Off };
}

/// One-pole coefficient for a time constant of `tau` seconds.
fn pole(tau: f32, sr: f32) -> f32 {
    1.0 - (-1.0 / (tau.max(0.0002) * sr)).exp()
}

pub struct SiliconState {
    pub pitch_bend: f32,
    sr: f32,
    chip: Option<Chip>,
    start: u32,
    done: u32,
    voices: [Voice; MAX_VOICES],
    held: Vec<(u8, f32)>,
    counter: u64,
    lfo: f32,
    last_pitch: Option<f32>,
    seed: u32,
    out: Vec<u32>,
    /// What the face shows: the chip's millions of instructions cycles a second of sound.
    mips: f32,
}

impl SiliconState {
    pub fn new(sr: f32) -> Self {
        let mut st = Self { pitch_bend: 0.0, sr, chip: None, start: 0, done: 0, voices: [Voice::OFF; MAX_VOICES], held: Vec::with_capacity(32), counter: 0, lfo: 0.0, last_pitch: None, seed: 0x5111_C0DE, out: vec![0; MAX_BLOCK], mips: 0.0 };
        if let (Ok(program), Ok(mut chip)) = (firmware(), Chip::new(MEMORY)) {
            chip.load(program);
            st.start = program.at("render_block");
            st.done = program.at("render_done");
            // Every voice sounding once, so the chip compiles its code now and not at the first note.
            for v in 0..MAX_VOICES as u32 {
                chip.write(Area::X, V_BASE + v * V_WORDS, &[1, 0, frac(0.01), 0, frac(0.01), 0, frac(0.005), 0, 0, frac(0.5), frac(0.01), 0, 0, 0, frac(0.1), frac(0.1)]);
            }
            chip.write(Area::X, G_N, &[4]);
            chip.write(Area::X, G_VOICES, &[MAX_VOICES as u32]);
            chip.run(st.start, st.done, 1_000_000);
            for v in 0..MAX_VOICES as u32 {
                chip.write(Area::X, V_BASE + v * V_WORDS, &[0; V_WORDS as usize]);
            }
            st.chip = Some(chip);
        }
        st
    }

    /// Whether the chip is there (with its firmware).
    pub fn ready(&self) -> bool {
        self.chip.is_some()
    }

    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed >> 8) as f32 / 8_388_608.0 - 1.0
    }

    fn write(&mut self, v: usize, word: u32, values: &[u32]) {
        if let Some(chip) = self.chip.as_mut() {
            chip.write(Area::X, V_BASE + v as u32 * V_WORDS + word, values);
        }
    }

    fn pitch_of(note: u8) -> f32 {
        note as f32
    }

    /// Voice `v` starts a note (from where it is, if it sounds).
    fn start(&mut self, p: &[f32], v: usize, note: u8, vel: f32, from: Option<f32>, retrigger: bool) {
        self.counter += 1;
        let fresh = self.voices[v].stage == Stage::Off;
        let pitch = Self::pitch_of(note);
        self.voices[v] = Voice { note, gate: true, age: self.counter, pitch: from.unwrap_or(pitch), target: pitch, stage: if retrigger || fresh { Stage::Attack } else { self.voices[v].stage } };
        if fresh {
            // Free running: each from a phase of its own; the envelope and the filter from rest.
            let (a, b, c) = (self.random(), self.random(), self.random());
            self.write(v, W_PH1, &[frac(a)]);
            self.write(v, W_PH2, &[frac(b)]);
            self.write(v, W_PHS, &[frac(c)]);
            self.write(v, W_ENV, &[0, 0]);
            self.write(v, W_LOW, &[0, 0]);
        }
        if retrigger || fresh {
            let rate = pole(p[ATTACK] / 3.0, self.sr);
            self.write(v, W_TARGET, &[frac(0.999), frac(rate)]);
            self.write(v, W_FENV, &[frac(0.999)]);
        }
        let amount = 1.0 - p[VELOCITY].clamp(0.0, 1.0) * (1.0 - vel.clamp(0.0, 1.0));
        self.write(v, W_VEL, &[frac(amount * 0.5)]);
        self.write(v, W_ACTIVE, &[1]);
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() || self.chip.is_none() {
            return;
        }
        let poly = (p[VOICES].round() as usize).clamp(1, MAX_VOICES);
        let glide = p[GLIDE] > 0.002;
        let legato = poly == 1 && !self.held.is_empty();
        self.held.retain(|h| h.0 != note);
        self.held.push((note, vel));
        if poly == 1 {
            let from = (glide && self.voices[0].stage != Stage::Off).then_some(self.voices[0].pitch);
            self.start(p, 0, note, vel, from, !legato);
            return;
        }
        // Its own voice again, a free one, the oldest let go, the oldest.
        let pick = (0..poly)
            .find(|&v| self.voices[v].gate && self.voices[v].note == note)
            .or_else(|| (0..poly).find(|&v| self.voices[v].stage == Stage::Off))
            .or_else(|| (0..poly).filter(|&v| !self.voices[v].gate).min_by_key(|&v| self.voices[v].age))
            .unwrap_or_else(|| (0..poly).min_by_key(|&v| self.voices[v].age).unwrap_or(0));
        let from = if glide { self.last_pitch } else { None };
        self.start(p, pick, note, vel, from, true);
        self.last_pitch = Some(Self::pitch_of(note));
    }

    fn release(&mut self, p: &[f32], v: usize) {
        self.voices[v].gate = false;
        self.voices[v].stage = Stage::Release;
        let rate = pole(p[RELEASE] / 4.6, self.sr);
        self.write(v, W_TARGET, &[0, frac(rate)]);
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        if p.len() < SPECS.len() {
            return;
        }
        self.held.retain(|h| h.0 != note);
        let poly = (p[VOICES].round() as usize).clamp(1, MAX_VOICES);
        if poly == 1 && self.voices[0].gate && self.voices[0].note == note {
            // Back to a key still held, gliding there.
            if let Some(&(prev, _)) = self.held.last() {
                self.voices[0].note = prev;
                self.voices[0].target = Self::pitch_of(prev);
                return;
            }
        }
        for v in 0..MAX_VOICES {
            if self.voices[v].gate && self.voices[v].note == note {
                self.release(p, v);
            }
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in 0..MAX_VOICES {
            if self.voices[v].gate {
                self.voices[v].gate = false;
                self.voices[v].stage = Stage::Release;
                let rate = pole(0.05, self.sr);
                self.write(v, W_TARGET, &[0, frac(rate)]);
            }
        }
    }

    /// For the face: voices sounding, the chip's MIPS.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.voices.iter().filter(|v| v.stage != Stage::Off).count() as f32;
        out[1] = self.mips;
        out[2] = self.ready() as u8 as f32;
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() || self.chip.is_none() {
            return;
        }
        let n = l.len().min(r.len());
        let mut at = 0;
        while at < n {
            let len = MAX_BLOCK.min(n - at);
            self.block(p, len);
            let gain = db_to_gain(p[VOLUME]);
            for i in 0..len {
                let s = to_frac(self.out[i]) * gain;
                l[at + i] += s;
                r[at + i] += s;
            }
            at += len;
        }
    }

    /// One block on the chip: the knobs and the voices into its memory,
    /// the firmware run, the samples out.
    fn block(&mut self, p: &[f32], len: usize) {
        let sr = self.sr;
        let dt = len as f32 / sr;
        self.lfo = (self.lfo + p[LFO_RATE] * dt).fract();
        let lfo = (std::f32::consts::TAU * self.lfo).sin();
        let shape = p[SHAPE].clamp(0.0, 1.0);
        let filter = (p[FILTER].round().max(0.0) as usize).min(2);
        let res = p[RESONANCE].clamp(0.0, 1.0);
        let env_amt = p[ENV_AMT].clamp(-1.0, 1.0) * 0.45;
        let mut mix = [0.0f32; 3];
        mix[filter] = 0.9;
        let globals = [
            len as u32,
            frac((1.0 - shape) * 0.5),
            frac(shape * 0.4),
            frac(p[OSC2].clamp(0.0, 1.0) * 0.4),
            frac(p[SUB].clamp(0.0, 1.0) * 0.4),
            frac(env_amt),
            frac(1.0 - 0.985 * res),
            frac(mix[0]),
            frac(mix[1]),
            frac(mix[2]),
            frac((-4.6 / (p[F_DECAY].max(0.005) * sr)).exp()),
            MAX_VOICES as u32,
        ];
        let glide = if p[GLIDE] > 0.002 { 1.0 - (-dt / (p[GLIDE] * 0.3)).exp() } else { 1.0 };
        let vib = lfo * p[LFO_PITCH] * p[LFO_PITCH] * 2.0;
        let detune = p[DETUNE].clamp(0.0, 1.0) * 0.5 + p[SEMITONE].round();
        let decay = pole(p[DECAY] / 4.6, sr);
        let sustain = p[SUSTAIN].clamp(0.0, 0.999);
        let Some(chip) = self.chip.as_mut() else { return };
        chip.write(Area::X, G_N, &globals);
        for v in 0..MAX_VOICES {
            let base = V_BASE + v as u32 * V_WORDS;
            let voice = &mut self.voices[v];
            if voice.stage == Stage::Off {
                continue;
            }
            // The envelope: from its attack to its decay, and silent after its release.
            let env = to_frac(chip.read_one(Area::X, base + W_ENV));
            match voice.stage {
                Stage::Attack if env >= 0.95 => {
                    voice.stage = Stage::Decay;
                    chip.write(Area::X, base + W_TARGET, &[frac(sustain), frac(decay)]);
                }
                Stage::Release if env < 0.0003 => {
                    voice.stage = Stage::Off;
                    chip.write(Area::X, base + W_ACTIVE, &[0]);
                    continue;
                }
                _ => {}
            }
            voice.pitch += (voice.target - voice.pitch) * glide;
            let pitch = voice.pitch + self.pitch_bend + vib;
            let f1 = midi_to_freq(pitch);
            let f2 = midi_to_freq(pitch + detune);
            let inc = |f: f32| frac((2.0 * f / sr).min(0.95));
            // The cutoff, followed by the key and the LFO; as f/2 for a filter run twice a sample.
            let hz = p[CUTOFF] * 2f32.powf(p[KEY_TRACK] * (voice.pitch - 60.0) / 12.0 + lfo * p[LFO_CUTOFF] * 2.0);
            let mut f = (std::f32::consts::PI * hz.clamp(20.0, sr * 0.45) / (2.0 * sr)).sin();
            // Room for the envelope to open (or close) it, the filter kept stable.
            f = if env_amt >= 0.0 { f.min(0.48 - env_amt) } else { f.max(-env_amt + 0.002) };
            chip.write(Area::X, base + W_INC1, &[inc(f1)]);
            chip.write(Area::X, base + W_INC2, &[inc(f2)]);
            chip.write(Area::X, base + W_INCS, &[inc(f1 * 0.5)]);
            chip.write(Area::X, base + W_CUT, &[frac(f.clamp(0.0005, 0.48))]);
        }
        let before = chip.cycles();
        chip.run(self.start, self.done, 4_000_000);
        let cycles = chip.cycles().saturating_sub(before) as f32;
        self.mips = self.mips * 0.9 + cycles / dt / 1e6 * 0.1;
        chip.read(Area::Y, 0, &mut self.out[..len]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn play(p: &[f32], notes: &[u8], secs: f32, hold: f32) -> (Vec<f32>, SiliconState) {
        let mut st = SiliconState::new(SR);
        assert!(st.ready(), "the chip and its firmware");
        for &k in notes {
            st.note_on(p, k, 0.9);
        }
        let mut out = Vec::new();
        let blocks = (secs * SR / 256.0) as usize;
        let mut released = false;
        for b in 0..blocks {
            if !released && b as f32 * 256.0 / SR >= hold {
                for &k in notes {
                    st.note_off(p, k);
                }
                released = true;
            }
            let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
            st.render(p, &mut l, &mut r);
            out.extend(l);
        }
        (out, st)
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    #[test]
    fn the_firmware_assembles() {
        let p = firmware().unwrap_or_else(|e| panic!("{e}"));
        assert!(p.words.len() > 80, "{} words", p.words.len());
        assert!(p.at("render_done") > p.at("render_block"));
    }

    #[test]
    fn a_note_sounds_at_its_pitch_from_the_chip() {
        let mut p = preset("Init");
        (p[OSC2], p[CUTOFF], p[ENV_AMT], p[SUSTAIN], p[RESONANCE]) = (0.0, 12_000.0, 0.0, 0.9, 0.0);
        let (out, st) = play(&p, &[57], 1.0, 1.0);
        let level = rms(&out[4800..]);
        assert!(level > 0.02, "it sounds: {level}");
        // A saw at 220 Hz: about 220 upward crossings a second (the first part left out).
        let crossings = out[4800..].windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count() as f32 / (out.len() - 4800) as f32 * SR;
        assert!((crossings - 220.0).abs() < 12.0, "{crossings}");
        assert!(st.mips > 1.0, "the chip did the work: {} MIPS", st.mips);
    }

    #[test]
    fn its_release_ends_it_and_the_chip_lets_the_voice_go() {
        let mut p = preset("Init");
        p[RELEASE] = 0.05;
        let (out, mut st) = play(&p, &[60, 64, 67], 1.0, 0.3);
        assert!(rms(&out[4800..12_000]) > 0.02);
        assert!(rms(&out[out.len() - 4800..]) < 1e-4, "silent after the release");
        assert!(st.voices.iter().all(|v| v.stage == Stage::Off));
        let chip = st.chip.as_mut().unwrap();
        assert!((0..MAX_VOICES as u32).all(|v| chip.read_one(Area::X, V_BASE + v * V_WORDS + W_ACTIVE) == 0));
    }

    #[test]
    fn three_notes_take_three_voices() {
        let (_, st) = play(&preset("Silicon Pad"), &[48, 55, 60], 0.3, 1.0);
        assert_eq!(st.voices.iter().filter(|v| v.gate).count(), 3);
    }

    #[test]
    fn the_filter_envelope_closes_the_filter() {
        let mut p = preset("Resonant Pluck");
        (p[SUSTAIN], p[DECAY], p[RESONANCE]) = (0.9, 1.0, 0.2);
        let (out, _) = play(&p, &[48], 1.2, 1.2);
        let sharp = |x: &[f32]| rms(&x.windows(2).map(|w| w[1] - w[0]).collect::<Vec<f32>>()) / rms(x).max(1e-9);
        let (early, late) = (sharp(&out[480..4800]), sharp(&out[48_000..52_800]));
        assert!(early > late * 2.0, "{early} {late}");
    }

    #[test]
    fn every_preset_sounds_and_stays_in_bounds() {
        for name in preset_names() {
            let (out, _) = play(&preset(name), &[45, 57], 1.0, 0.6);
            assert!(rms(&out) > 1e-3, "{name} sounds");
            assert!(out.iter().all(|v| v.is_finite() && v.abs() <= 2.0), "{name} stays in bounds");
        }
    }

    #[test]
    #[ignore]
    fn speed() {
        let p = preset("Silicon Pad");
        let mut st = SiliconState::new(SR);
        for k in [48, 52, 55, 59, 60, 64, 67, 71] {
            st.note_on(&p, k, 0.9);
        }
        let t = std::time::Instant::now();
        let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
        for _ in 0..(SR as usize / 256) {
            st.render(&p, &mut l, &mut r);
        }
        eprintln!("one second of eight voices on the chip: {:?}, {:.1} MIPS", t.elapsed(), st.mips);
    }
}
