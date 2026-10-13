//! Beads as an instrument: the wavetable synth of Mutable Instruments
//! Beads (Emilie Gillet, MIT; see `krabton_beads`), played with keys. The
//! key sets the pitch (from C3, the wavetables' root) and holding it seeds
//! grains as DENSITY says (or the grains keep coming, or follow the song's
//! clock); the oscillator sounds while a key is held, shaped by Attack and
//! Release, and the grains and the reverb ring on. One voice: the last key
//! held plays.

use crate::params::{ParamSpec, Unit};
use krabton_beads::{Beads, Clock, GrainView, Media, Params, Seed, Source};

pub const BANK: usize = 0;
pub const WAVE: usize = 1;
pub const SIZE: usize = 2;
pub const SHAPE: usize = 3;
pub const DENSITY: usize = 4;
pub const TRANSPOSE: usize = 5;
pub const WAVE_RANDOM: usize = 6;
pub const SIZE_RANDOM: usize = 7;
pub const SHAPE_RANDOM: usize = 8;
pub const PITCH_RANDOM: usize = 9;
pub const GRAINS: usize = 10;
pub const REVERB: usize = 11;
pub const FREEZE: usize = 12;
pub const SEED: usize = 13;
pub const SYNC: usize = 14;
pub const QUALITY: usize = 15;
pub const ATTACK: usize = 16;
pub const RELEASE: usize = 17;
pub const VOLUME: usize = 18;

pub const BANKS: &[&str] = &["Sines", "Triangle Stacks", "Drawbars", "Tri to Saw", "Bursts & Formants", "Voices", "Digital & Metal", "Braids", "FM 2:1", "FM 1:2", "FM 1:3", "FM 1:8"];
pub const SEEDS: &[&str] = &["While Held", "Always", "Synced"];
pub const SYNCS: &[&str] = &["1/32", "1/16", "1/8", "1/4", "1/2", "1 Bar"];
const SYNC_BEATS: [f64; 6] = [0.125, 0.25, 0.5, 1.0, 2.0, 4.0];
pub const QUALITIES: &[&str] = &["Hi-Fi", "Clouds", "Sunny Tape", "Scorched Cassette"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Bank", "Wave", BANKS, 0),
    ParamSpec::lin("Wave", "Wave", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Size", "Grains", 0.0, 1.0, 0.45, Unit::Percent),
    ParamSpec::lin("Shape", "Grains", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Density", "Grains", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::int("Transpose", "Wave", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Wave Random", "Random", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Size Random", "Random", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Shape Random", "Random", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Pitch Random", "Random", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Grains", "Mix", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Reverb", "Mix", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::toggle("Freeze", "Mix", false),
    ParamSpec::choice("Seed", "Seed", SEEDS, 0),
    ParamSpec::choice("Sync", "Seed", SYNCS, 1),
    ParamSpec::choice("Quality", "Sound", QUALITIES, 0),
    ParamSpec::log("Attack", "Sound", 0.002, 2.0, 0.01, Unit::Sec),
    ParamSpec::log("Release", "Sound", 0.02, 5.0, 0.4, Unit::Sec),
    ParamSpec::lin("Volume", "Sound", -24.0, 12.0, 0.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Sine Sparkle", "Glass Pad", "Organ Grains", "Choir Dust", "FM Bells", "Metal Rain", "Pulse Swarm", "Tape Voices", "Bit Shards", "Frozen Formants"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let bank = |n: &str| BANKS.iter().position(|b| *b == n).unwrap_or(0) as f32;
    let set: Vec<(usize, f32)> = match name {
        "Glass Pad" => vec![(BANK, bank("Sines")), (WAVE, 0.3), (SIZE, 0.75), (DENSITY, 0.75), (WAVE_RANDOM, 0.3), (GRAINS, 0.7), (REVERB, 0.7), (ATTACK, 0.6), (RELEASE, 2.5)],
        "Organ Grains" => vec![(BANK, bank("Drawbars")), (WAVE, 0.4), (SIZE, 0.55), (DENSITY, 0.25), (GRAINS, 0.45), (REVERB, 0.35)],
        "Choir Dust" => vec![(BANK, bank("Voices")), (WAVE, 0.5), (SIZE, 0.65), (DENSITY, 0.72), (WAVE_RANDOM, 0.5), (PITCH_RANDOM, 0.15), (GRAINS, 0.65), (REVERB, 0.6), (ATTACK, 0.25), (RELEASE, 1.5)],
        "FM Bells" => vec![(BANK, bank("FM 1:3")), (WAVE, 0.45), (SIZE, 0.5), (SHAPE, 0.15), (DENSITY, 0.5), (GRAINS, 0.55), (REVERB, 0.45), (RELEASE, 1.2)],
        "Metal Rain" => vec![(BANK, bank("Digital & Metal")), (WAVE, 0.7), (SIZE, 0.25), (DENSITY, 0.85), (PITCH_RANDOM, 0.5), (SHAPE_RANDOM, 0.4), (GRAINS, 0.8), (REVERB, 0.4)],
        "Pulse Swarm" => vec![(BANK, bank("Bursts & Formants")), (WAVE, 0.2), (SIZE, 0.35), (DENSITY, 0.9), (WAVE_RANDOM, 0.7), (GRAINS, 0.85), (REVERB, 0.3)],
        "Tape Voices" => vec![(BANK, bank("Voices")), (WAVE, 0.3), (SIZE, 0.6), (DENSITY, 0.65), (QUALITY, 2.0), (GRAINS, 0.6), (REVERB, 0.5), (RELEASE, 1.0)],
        "Bit Shards" => vec![(BANK, bank("Braids")), (WAVE, 0.6), (SIZE, 0.2), (DENSITY, 0.12), (QUALITY, 3.0), (PITCH_RANDOM, -0.6), (GRAINS, 0.7), (REVERB, 0.2)],
        "Frozen Formants" => vec![(BANK, bank("Bursts & Formants")), (WAVE, 0.75), (SIZE, 0.8), (DENSITY, 0.7), (WAVE_RANDOM, -0.5), (SEED, 1.0), (GRAINS, 0.75), (REVERB, 0.75), (ATTACK, 1.0), (RELEASE, 3.0)],
        // Sine Sparkle.
        _ => vec![(BANK, bank("Sines")), (WAVE, 0.55), (SIZE, 0.4), (DENSITY, 0.3), (PITCH_RANDOM, 0.25), (GRAINS, 0.55), (REVERB, 0.45)],
    };
    for (i, v) in set {
        p[i] = v;
    }
    p
}

pub struct BeadsSynthState {
    /// Pitch bend in semitones (a wheel, a controller's triggers), set by the engine.
    pub pitch_bend: f32,
    /// The song plays and where it is (set by the engine every block).
    pub playing: bool,
    pub beat: f64,
    pub bpm: f32,
    sr: f32,
    beads: Box<Beads>,
    /// Keys held, in order; the last plays.
    stack: Vec<u8>,
    note: u8,
    velocity: f32,
    gain: f32,
    env: f32,
    views: [GrainView; 8],
}

impl BeadsSynthState {
    pub fn new(sr: f32) -> Self {
        let sr = sr.max(1000.0);
        Self {
            pitch_bend: 0.0, playing: false, beat: 0.0, bpm: 120.0, sr, beads: Box::new(Beads::new(sr)), stack: Vec::with_capacity(32), note: 48, velocity: 0.8, gain: 0.0, env: 0.0, views: [GrainView::default(); 8] }
    }

    pub fn note_on(&mut self, _p: &[f32], note: u8, vel: f32) {
        if self.stack.len() < 32 {
            self.stack.retain(|&n| n != note);
            self.stack.push(note);
        }
        self.note = note;
        self.velocity = vel.clamp(0.0, 1.0);
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        self.stack.retain(|&n| n != note);
        if let Some(&n) = self.stack.last() {
            self.note = n;
        }
    }

    pub fn all_off(&mut self) {
        self.stack.clear();
    }

    /// The screen: [0] on, [1] the bank, [2] a key held, [3] the wave, [4]
    /// the oscillator's level, then per grain (5 + 3i) its wave, level and
    /// pitch from C3.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = 1.0;
        out[1] = self.beads.status().bank as f32;
        out[2] = (!self.stack.is_empty()) as u8 as f32;
        out[4] = self.env;
        for (i, g) in self.views.iter().enumerate() {
            let o = 5 + 3 * i;
            out[o] = g.position;
            out[o + 1] = g.level;
            out[o + 2] = g.pitch;
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let held = !self.stack.is_empty();
        // The oscillator's envelope, per block.
        let dt = n as f32 / self.sr;
        let (target, time) = if held { (1.0, p[ATTACK].max(0.001)) } else { (0.0, p[RELEASE].max(0.005)) };
        self.env += (target - self.env) * (1.0 - (-dt * 4.6 / time).exp());
        let seed_mode = (p[SEED].round().max(0.0) as usize).min(SEEDS.len() - 1);
        let division = SYNC_BEATS[(p[SYNC].round().max(0.0) as usize).min(SYNC_BEATS.len() - 1)];
        let clock = (seed_mode == 2 && self.playing).then(|| Clock { beat: self.beat, beats_per_sample: self.bpm as f64 / 60.0 / self.sr as f64, division });
        let quality = match (p[QUALITY].round().max(0.0) as usize).min(3) {
            0 => Media::Digital,
            1 => Media::VintageDigital,
            2 => Media::CleanTape,
            _ => Media::DirtyTape,
        };
        let bank = (p[BANK].round().max(0.0) as usize).min(BANKS.len() - 1);
        let params = Params {
            time: p[WAVE],
            time_amount: p[WAVE_RANDOM],
            size: p[SIZE].min(0.98),
            size_amount: p[SIZE_RANDOM],
            shape: p[SHAPE],
            shape_amount: p[SHAPE_RANDOM],
            pitch: self.note as f32 - 48.0 + p[TRANSPOSE].round() + self.pitch_bend,
            pitch_amount: p[PITCH_RANDOM],
            density: p[DENSITY],
            // FEEDBACK picks the bank in the synth: the middle of its step.
            feedback: (bank as f32 + 0.5) / BANKS.len() as f32,
            dry_wet: p[GRAINS],
            reverb: p[REVERB],
            freeze: p[FREEZE] >= 0.5,
            seed: if seed_mode == 0 { Seed::Button(held) } else { Seed::Latched },
            clock,
            quality,
            mono: false,
            source: Source::Wavetables,
            input_gain: 0.0,
            dry_level: self.env,
        };
        l[..n].fill(0.0);
        r[..n].fill(0.0);
        self.beads.process(&params, &mut l[..n], &mut r[..n]);
        // Velocity sets the level, smoothly.
        let g = crate::dsp::db_to_gain(p[VOLUME]) * (0.35 + 0.65 * self.velocity) * 0.8;
        let start = self.gain;
        for i in 0..n {
            let k = start + (g - start) * (i as f32 / n.max(1) as f32);
            l[i] *= k;
            r[i] *= k;
        }
        self.gain = g;
        self.views = [GrainView::default(); 8];
        self.beads.grains(&mut self.views);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn run(st: &mut BeadsSynthState, p: &[f32], secs: f32) -> Vec<f32> {
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
        let w = std::f32::consts::TAU as f64 * hz as f64 / SR as f64;
        let n = x.len() as f64;
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, &s) in x.iter().enumerate() {
            let win = 0.5 - 0.5 * (std::f32::consts::TAU as f64 * i as f64 / n).cos();
            re += s as f64 * win * (w * i as f64).cos();
            im += s as f64 * win * (w * i as f64).sin();
        }
        ((re * re + im * im).sqrt() / n) as f32
    }

    #[test]
    fn every_preset_plays_a_key_and_stays_bounded() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = BeadsSynthState::new(SR);
            st.note_on(&p, 60, 0.9);
            let l = run(&mut st, &p, 2.0);
            assert!(l.iter().all(|v| v.is_finite() && v.abs() < 1.5), "{name}");
            assert!(rms(&l[48000..]) > 0.01, "{name}: {}", rms(&l[48000..]));
        }
    }

    #[test]
    fn silent_until_a_key_and_the_key_sets_the_pitch() {
        let mut p = preset("Organ Grains");
        p[GRAINS] = 0.0;
        p[REVERB] = 0.0;
        p[BANK] = 0.0;
        p[WAVE] = 0.0;
        let mut st = BeadsSynthState::new(SR);
        let l = run(&mut st, &p, 0.5);
        assert!(rms(&l) < 1e-3, "{}", rms(&l));
        // A3: 220 Hz.
        st.note_on(&p, 57, 0.9);
        let l = run(&mut st, &p, 1.5);
        let tail = &l[24000..];
        assert!(mag(tail, 220.0) > 3.0 * mag(tail, 250.0), "{} {}", mag(tail, 220.0), mag(tail, 250.0));
        // Released: it dies away.
        st.note_off(&p, 57);
        let l = run(&mut st, &p, 2.0);
        assert!(rms(&l[48000..]) < 0.2 * rms(tail), "{}", rms(&l[48000..]));
    }
}
