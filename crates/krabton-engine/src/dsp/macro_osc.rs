//! Plaits: a polyphonic instrument around the Plaits macro-oscillator.
//!
//! The synthesis is the DSP code of Mutable Instruments Plaits by Emilie
//! Gillet (MIT), through the Rust port `mi-plaits-dsp` by Oliver Rockstedt
//! (MIT). It has 24 models, from virtual analog with a resonant filter to
//! phase distortion, DX7-style FM, wave terrains, speech, physical models
//! and drums. Krabton adds voice allocation (up to 8 voices), an ADSR or
//! the original low-pass gate per voice, glide, stereo spread and presets.

use crate::params::{ParamSpec, Unit};
use mi_plaits_dsp::voice::{Modulations, Patch, Voice};

/// Plaits renders in small fixed blocks.
const BLOCK: usize = 12;
pub const MAX_VOICES: usize = 8;

pub const MODELS: &[&str] = &[
    "VA + Filter",
    "Phase Dist",
    "6-Op FM A",
    "6-Op FM B",
    "6-Op FM C",
    "Wave Terrain",
    "String Machine",
    "Chiptune",
    "Virtual Analog",
    "Waveshaper",
    "FM",
    "Formant",
    "Additive",
    "Wavetable",
    "Chords",
    "Speech",
    "Swarm",
    "Noise",
    "Particles",
    "String",
    "Modal",
    "Bass Drum",
    "Snare",
    "Hi-Hat",
];

/// What HARMONICS, TIMBRE and MORPH do in each model.
pub const MODEL_KNOBS: [[&str; 3]; 24] = [
    ["Resonance", "Cutoff", "Wave+Sub"],
    ["PD Freq", "PD Amount", "Pulse Width"],
    ["Patch", "Brightness", "Env Stretch"],
    ["Patch", "Brightness", "Env Stretch"],
    ["Patch", "Brightness", "Env Stretch"],
    ["Terrain", "Radius", "Offset"],
    ["Chord", "Tone", "Registration"],
    ["Chord", "Arpeggio", "Pulse Shape"],
    ["Detune", "Square", "Saw"],
    ["Waveform", "Fold", "Asymmetry"],
    ["Ratio", "Index", "Feedback"],
    ["Ratio", "Formant", "Width"],
    ["Bumps", "Peak", "Shape"],
    ["Bank", "Row", "Column"],
    ["Chord", "Inversion", "Waveform"],
    ["Mode", "Species", "Phoneme"],
    ["Spread", "Density", "Grain"],
    ["Response", "Clock", "Resonance"],
    ["Spread", "Density", "Filter"],
    ["Inharmonic", "Brightness", "Decay"],
    ["Material", "Brightness", "Decay"],
    ["Punch", "Tone", "Decay"],
    ["Noise", "Modes", "Decay"],
    ["Metal", "HP Cutoff", "Decay"],
];

/// Models that shape their own amplitude (the low-pass gate is bypassed).
fn self_enveloped(model: usize) -> bool {
    matches!(model, 2..=4 | 19..=23)
}

pub const MODEL: usize = 0;
pub const HARMONICS: usize = 1;
pub const TIMBRE: usize = 2;
pub const MORPH: usize = 3;
pub const AUX_MIX: usize = 4;
pub const DECAY: usize = 5;
pub const TIMBRE_ENV: usize = 6;
pub const MORPH_ENV: usize = 7;
pub const PITCH_ENV: usize = 8;
pub const LPG_COLOUR: usize = 9;
pub const AMP_MODE: usize = 10;
pub const ATTACK: usize = 11;
pub const AMP_DECAY: usize = 12;
pub const SUSTAIN: usize = 13;
pub const RELEASE: usize = 14;
pub const VELOCITY: usize = 15;
pub const VOICES: usize = 16;
pub const GLIDE: usize = 17;
pub const TRANSPOSE: usize = 18;
pub const SPREAD: usize = 19;
pub const VOLUME: usize = 20;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Model", "Model", MODELS, 0),
    ParamSpec::lin("Harmonics", "Model", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Timbre", "Model", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Morph", "Model", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Out / Aux", "Model", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Decay", "Envelope", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Timbre Env", "Envelope", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Morph Env", "Envelope", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Pitch Env", "Envelope", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("LPG Colour", "Envelope", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::choice("Amp", "Amp", &["Pluck (LPG)", "ADSR"], 1),
    ParamSpec::log("Attack", "Amp", 0.001, 4.0, 0.003, Unit::Sec),
    ParamSpec::log("Amp Decay", "Amp", 0.005, 8.0, 0.4, Unit::Sec),
    ParamSpec::lin("Sustain", "Amp", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::log("Release", "Amp", 0.005, 8.0, 0.25, Unit::Sec),
    ParamSpec::lin("Velocity", "Amp", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Voices", "Voice", 1.0, 8.0, 8.0, Unit::None),
    ParamSpec::log("Glide", "Voice", 0.001, 2.0, 0.001, Unit::Sec),
    ParamSpec::int("Transpose", "Voice", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Spread", "Voice", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Volume", "Voice", -24.0, 6.0, -4.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Analog Poly",
        "Analog Brass",
        "Analog Bass",
        "Acid Line",
        "Sync Lead",
        "Square Pluck",
        "String Machine",
        "PD Keys",
        "DX Piano",
        "DX Bass",
        "Terrain Pad",
        "Chiptune Arp",
        "Chord Stab",
        "Swarm Pad",
        "Formant Voice",
        "Talking Synth",
        "Glass Modal",
        "Plucked String",
        "Particles",
        "Kick",
        "Snare",
        "Hat",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    // Shared starting points.
    let pluck = [(AMP_MODE, 0.0), (VOICES, 8.0)];
    match name {
        "Analog Poly" => set(&[(MODEL, 0.0), (HARMONICS, 0.42), (TIMBRE, 0.5), (MORPH, 0.3), (DECAY, 0.55), (TIMBRE_ENV, 0.25), (ATTACK, 0.01), (AMP_DECAY, 0.6), (SUSTAIN, 0.7), (RELEASE, 0.5), (SPREAD, 0.6)]),
        "Analog Brass" => set(&[(MODEL, 0.0), (HARMONICS, 0.3), (TIMBRE, 0.35), (MORPH, 0.2), (DECAY, 0.6), (TIMBRE_ENV, 0.45), (ATTACK, 0.04), (SUSTAIN, 0.85), (RELEASE, 0.3), (SPREAD, 0.4)]),
        "Analog Bass" => set(&[(MODEL, 0.0), (HARMONICS, 0.62), (TIMBRE, 0.28), (MORPH, 0.0), (DECAY, 0.4), (TIMBRE_ENV, 0.5), (VOICES, 1.0), (GLIDE, 0.04), (TRANSPOSE, -12.0), (SUSTAIN, 0.9), (RELEASE, 0.08), (SPREAD, 0.0), (VOLUME, 1.0)]),
        "Acid Line" => set(&[(MODEL, 0.0), (HARMONICS, 0.9), (TIMBRE, 0.25), (MORPH, 0.25), (DECAY, 0.3), (TIMBRE_ENV, 0.6), (VOICES, 1.0), (GLIDE, 0.06), (TRANSPOSE, -12.0), (SUSTAIN, 1.0), (RELEASE, 0.05), (SPREAD, 0.0)]),
        "Sync Lead" => set(&[(MODEL, 8.0), (HARMONICS, 0.55), (TIMBRE, 0.75), (MORPH, 0.6), (AUX_MIX, 0.5), (VOICES, 1.0), (GLIDE, 0.08), (SUSTAIN, 1.0), (RELEASE, 0.2), (SPREAD, 0.0)]),
        "Square Pluck" => {
            set(&pluck);
            set(&[(MODEL, 0.0), (HARMONICS, 0.45), (TIMBRE, 0.6), (MORPH, 0.62), (DECAY, 0.35), (LPG_COLOUR, 0.4), (SPREAD, 0.5)]);
        }
        "String Machine" => set(&[(MODEL, 6.0), (HARMONICS, 0.0), (TIMBRE, 0.45), (MORPH, 0.4), (ATTACK, 0.25), (SUSTAIN, 1.0), (RELEASE, 1.2), (SPREAD, 0.7)]),
        "PD Keys" => set(&[(MODEL, 1.0), (HARMONICS, 0.3), (TIMBRE, 0.5), (MORPH, 0.3), (DECAY, 0.45), (TIMBRE_ENV, 0.4), (AMP_DECAY, 1.2), (SUSTAIN, 0.3), (RELEASE, 0.4), (VOLUME, -8.0)]),
        "DX Piano" => set(&[(MODEL, 2.0), (HARMONICS, 0.03), (TIMBRE, 0.5), (MORPH, 0.5), (SUSTAIN, 1.0), (RELEASE, 0.6), (SPREAD, 0.5)]),
        "DX Bass" => set(&[(MODEL, 3.0), (HARMONICS, 0.13), (TIMBRE, 0.55), (MORPH, 0.5), (VOICES, 1.0), (TRANSPOSE, -12.0), (SUSTAIN, 1.0), (RELEASE, 0.1), (SPREAD, 0.0), (VOLUME, 2.0)]),
        "Terrain Pad" => set(&[(MODEL, 5.0), (HARMONICS, 0.4), (TIMBRE, 0.6), (MORPH, 0.5), (MORPH_ENV, 0.3), (DECAY, 0.8), (ATTACK, 0.6), (SUSTAIN, 0.9), (RELEASE, 1.8), (SPREAD, 0.8), (VOLUME, -7.0)]),
        "Chiptune Arp" => set(&[(MODEL, 7.0), (HARMONICS, 0.3), (TIMBRE, 0.7), (MORPH, 0.4), (VOICES, 4.0), (SUSTAIN, 1.0), (RELEASE, 0.1)]),
        "Chord Stab" => {
            set(&pluck);
            set(&[(MODEL, 14.0), (HARMONICS, 0.4), (TIMBRE, 0.5), (MORPH, 0.35), (DECAY, 0.4), (VOICES, 4.0)]);
        }
        "Swarm Pad" => set(&[(MODEL, 16.0), (HARMONICS, 0.35), (TIMBRE, 0.6), (MORPH, 0.7), (ATTACK, 0.8), (SUSTAIN, 1.0), (RELEASE, 2.0), (VOICES, 4.0), (SPREAD, 0.8)]),
        "Formant Voice" => set(&[(MODEL, 11.0), (HARMONICS, 0.35), (TIMBRE, 0.45), (MORPH, 0.4), (TIMBRE_ENV, 0.2), (ATTACK, 0.05), (SUSTAIN, 0.9), (VOLUME, -11.0)]),
        "Talking Synth" => {
            set(&pluck);
            set(&[(MODEL, 15.0), (HARMONICS, 0.8), (TIMBRE, 0.5), (MORPH, 0.2), (DECAY, 0.7), (VOICES, 2.0)]);
        }
        "Glass Modal" => {
            set(&pluck);
            set(&[(MODEL, 20.0), (HARMONICS, 0.7), (TIMBRE, 0.6), (MORPH, 0.7), (SPREAD, 0.6), (VOLUME, -9.0)]);
        }
        "Plucked String" => {
            set(&pluck);
            set(&[(MODEL, 19.0), (HARMONICS, 0.2), (TIMBRE, 0.55), (MORPH, 0.6), (SPREAD, 0.5)]);
        }
        "Particles" => set(&[(MODEL, 18.0), (HARMONICS, 0.4), (TIMBRE, 0.5), (MORPH, 0.8), (ATTACK, 0.2), (SUSTAIN, 1.0), (RELEASE, 1.5), (VOICES, 4.0), (SPREAD, 0.8)]),
        "Kick" => {
            set(&pluck);
            set(&[(MODEL, 21.0), (HARMONICS, 0.6), (TIMBRE, 0.4), (MORPH, 0.55), (TRANSPOSE, -24.0), (SPREAD, 0.0), (VELOCITY, 0.8)]);
        }
        "Snare" => {
            set(&pluck);
            set(&[(MODEL, 22.0), (HARMONICS, 0.5), (TIMBRE, 0.5), (MORPH, 0.4), (TRANSPOSE, -12.0), (SPREAD, 0.0), (VELOCITY, 0.8)]);
        }
        "Hat" => {
            set(&pluck);
            set(&[(MODEL, 23.0), (HARMONICS, 0.5), (TIMBRE, 0.65), (MORPH, 0.3), (SPREAD, 0.1), (VELOCITY, 0.8)]);
        }
        _ => {}
    }
    p
}

struct MacroVoice {
    plaits: Box<Voice<'static>>,
    out: [f32; BLOCK],
    aux: [f32; BLOCK],
    pos: usize,
    pitch: u8,
    note: f32,
    target: f32,
    gate: bool,
    /// Blocks to hold the trigger low before raising it (retrigger).
    low_blocks: u8,
    /// Blocks to keep the trigger high even after a short note.
    min_high: u8,
    velocity: f32,
    env: f32,
    stage: u8,
    active: bool,
    quiet_blocks: u32,
    age: u64,
    pan: f32,
}

impl MacroVoice {
    fn new(sr: f32, index: usize) -> Self {
        let mut plaits = Box::new(Voice::new(BLOCK, sr));
        plaits.init();
        const PANS: [f32; MAX_VOICES] = [-1.0, 1.0, -0.5, 0.5, -0.75, 0.75, -0.25, 0.25];
        Self {
            plaits,
            out: [0.0; BLOCK],
            aux: [0.0; BLOCK],
            pos: BLOCK,
            pitch: 0,
            note: 48.0,
            target: 48.0,
            gate: false,
            low_blocks: 0,
            min_high: 0,
            velocity: 0.0,
            env: 0.0,
            stage: 0,
            active: false,
            quiet_blocks: 0,
            age: 0,
            pan: PANS[index % MAX_VOICES],
        }
    }

    fn start(&mut self, pitch: u8, note: f32, vel: f32, glide: bool, age: u64) {
        if !glide || !self.active {
            self.note = note;
        }
        self.target = note;
        self.pitch = pitch;
        // A voice whose trigger is still high needs a low block first.
        self.low_blocks = if self.gate || self.min_high > 0 { 2 } else { 0 };
        self.min_high = 3;
        self.gate = true;
        self.velocity = vel;
        self.stage = 1;
        self.active = true;
        self.quiet_blocks = 0;
        self.age = age;
    }

    fn release(&mut self) {
        self.gate = false;
        if self.stage != 0 {
            self.stage = 4;
        }
    }
}

pub struct MacroState {
    /// Pitch bend in semitones (a wheel, a controller's triggers), set by the engine.
    pub pitch_bend: f32,
    voices: Vec<MacroVoice>,
    /// Held notes, newest last (mono mode).
    held: Vec<u8>,
    counter: u64,
    sr: f32,
}

impl MacroState {
    pub fn new(sr: f32) -> Self {
        Self {
            pitch_bend: 0.0, voices: (0..MAX_VOICES).map(|i| MacroVoice::new(sr, i)).collect(), held: Vec::with_capacity(32), counter: 0, sr }
    }

    fn voices_used(p: &[f32]) -> usize {
        (p.get(VOICES).copied().unwrap_or(8.0).round() as usize).clamp(1, MAX_VOICES)
    }

    pub fn note_on(&mut self, p: &[f32], pitch: u8, vel: f32) {
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
                // Slide to the new note without a new attack.
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
        let model = (p[MODEL].round().max(0.0) as usize).min(MODELS.len() - 1);
        let adsr = p[AMP_MODE] >= 0.5;
        let enveloped = self_enveloped(model);
        let patch = Patch {
            note: 48.0,
            harmonics: p[HARMONICS],
            timbre: p[TIMBRE],
            morph: p[MORPH],
            frequency_modulation_amount: p[PITCH_ENV],
            timbre_modulation_amount: p[TIMBRE_ENV],
            morph_modulation_amount: p[MORPH_ENV],
            engine: model,
            decay: p[DECAY],
            lpg_colour: p[LPG_COLOUR],
        };
        let block_dt = BLOCK as f32 / self.sr;
        let glide_k = 1.0 - (-block_dt / p[GLIDE].max(1e-4)).exp();
        let attack_step = block_dt / p[ATTACK].max(1e-4);
        let decay_k = 1.0 - (-block_dt * 4.6 / p[AMP_DECAY].max(1e-4)).exp();
        let release_k = 1.0 - (-block_dt * 4.6 / p[RELEASE].max(1e-4)).exp();
        let sustain = p[SUSTAIN];
        let aux_mix = p[AUX_MIX];
        let master = crate::dsp::db_to_gain(p[VOLUME]) * 0.4;
        let spread = p[SPREAD];
        let vel_amt = p[VELOCITY];

        // Voices beyond the current count still finish their tails.
        for v in self.voices.iter_mut() {
            if !v.active {
                continue;
            }
            let (pl, pr) = crate::dsp::pan_gains(v.pan * spread);
            let vel_gain = 1.0 - vel_amt + vel_amt * v.velocity;
            for i in 0..n {
                if v.pos >= BLOCK {
                    // Control rate: glide, envelope, trigger, then one Plaits block.
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
                            }
                        }
                        _ => v.env = 0.0,
                    }
                    let trigger = if v.low_blocks > 0 {
                        v.low_blocks -= 1;
                        0.0
                    } else if v.gate || v.min_high > 0 {
                        v.min_high = v.min_high.saturating_sub(1);
                        1.0
                    } else {
                        0.0
                    };
                    let m = Modulations {
                        note: v.note - 48.0 + self.pitch_bend,
                        trigger,
                        trigger_patched: true,
                        level: v.env,
                        level_patched: adsr && !enveloped,
                        ..Default::default()
                    };
                    v.plaits.render(&patch, &m, &mut v.out, &mut v.aux);
                    // In ADSR mode the envelope also drives a VCA, so the
                    // release ends when it should (the gate adds its own tail).
                    let vca = if adsr { v.env } else { 1.0 };
                    let mut peak = 0.0f32;
                    for k in 0..BLOCK {
                        let s = (v.out[k] * (1.0 - aux_mix) + v.aux[k] * aux_mix) * vca;
                        v.out[k] = if s.is_finite() { s } else { 0.0 };
                        peak = peak.max(v.out[k].abs());
                    }
                    v.pos = 0;
                    // A released voice that has gone quiet stops rendering.
                    let done = if adsr { v.stage == 0 } else { !v.gate && v.min_high == 0 && peak < 2e-5 };
                    if done && peak < 1e-3 {
                        v.quiet_blocks += 1;
                        if v.quiet_blocks > 40 {
                            v.active = false;
                        }
                    } else {
                        v.quiet_blocks = 0;
                    }
                }
                let s = v.out[v.pos] * master * vel_gain;
                v.pos += 1;
                l[i] += s * pl;
                r[i] += s * pr;
            }
        }
    }

    /// Voices currently sounding (for tests and meters).
    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(st: &mut MacroState, p: &[f32], secs: f32) -> (Vec<f32>, Vec<f32>) {
        let n = (secs * 48000.0) as usize;
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
    fn every_preset_makes_sound_and_releases() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = MacroState::new(48000.0);
            st.note_on(&p, 60, 0.9);
            let (l, r) = render(&mut st, &p, 0.4);
            let level = rms(&l).max(rms(&r));
            assert!(level > 1e-3, "{name} is silent ({level})");
            assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 4.0), "{name} blew up");
            st.note_off(&p, 60);
            let _ = render(&mut st, &p, 4.0);
            let (l, _) = render(&mut st, &p, 0.2);
            assert!(rms(&l) < 2e-3, "{name} keeps sounding after release ({})", rms(&l));
        }
    }

    #[test]
    fn plays_chords_and_tracks_pitch() {
        let p = preset("Analog Poly");
        let mut st = MacroState::new(48000.0);
        for n in [60, 64, 67] {
            st.note_on(&p, n, 0.8);
        }
        let _ = render(&mut st, &p, 0.1);
        assert_eq!(st.active_voices(), 3);
        // Pitch: count zero crossings of a single low note on the plain VA model.
        let mut p = preset("Sync Lead");
        p[TIMBRE] = 0.0;
        p[MORPH] = 0.0;
        p[HARMONICS] = 0.5;
        p[AUX_MIX] = 0.0;
        let mut st = MacroState::new(48000.0);
        st.note_on(&p, 57, 1.0); // A3 = 220 Hz
        let (l, _) = render(&mut st, &p, 1.0);
        let tail = &l[24000..];
        let ups = tail.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f32 * 2.0;
        assert!((ups - 220.0).abs() < 12.0, "A3 measured at {ups} Hz");
    }

    #[test]
    fn mono_legato_glides_without_retrigger() {
        let p = preset("Analog Bass");
        let mut st = MacroState::new(48000.0);
        st.note_on(&p, 48, 1.0);
        let _ = render(&mut st, &p, 0.2);
        st.note_on(&p, 55, 1.0);
        assert!(st.voices[0].stage >= 2, "legato does not restart the attack");
        let _ = render(&mut st, &p, 0.5);
        assert!((st.voices[0].note - (55.0 - 12.0)).abs() < 0.05);
        st.note_off(&p, 55);
        assert!(st.voices[0].gate, "falls back to the held note");
        assert!((st.voices[0].target - 36.0).abs() < 1e-3);
        st.note_off(&p, 48);
        assert!(!st.voices[0].gate);
    }
}
