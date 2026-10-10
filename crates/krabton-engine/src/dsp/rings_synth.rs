//! Rings: the Mutable Instruments Rings resonator (Emilie Gillet, MIT; see
//! `krabton_rings`) as an instrument played with notes, and as an effect
//! that rings with whatever audio comes in.

use super::db_to_gain;
use super::effects::FxCtx;
use crate::params::{ParamSpec, Unit};
use krabton_rings::{EggFx, Model, Params, Rings};

pub const MODEL: usize = 0;
pub const POLYPHONY: usize = 1;
pub const STRUCTURE: usize = 2;
pub const BRIGHTNESS: usize = 3;
pub const DAMPING: usize = 4;
pub const POSITION: usize = 5;
pub const EGG: usize = 6;
pub const EGG_FX: usize = 7;
pub const WIDTH: usize = 8;
pub const VOLUME: usize = 9;

pub const MODELS: &[&str] = &["Modal", "Sympathetic", "Inharmonic", "FM Voice", "Chords", "String + Reverb"];
pub const EGG_FXS: &[&str] = &["Formant", "Chorus", "Reverb", "Formant 2", "Ensemble", "Reverb 2"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Model", "Resonator", MODELS, 0),
    ParamSpec::choice("Voices", "Resonator", &["1", "2", "4"], 2),
    ParamSpec::lin("Structure", "Resonator", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Brightness", "Resonator", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Damping", "Resonator", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Position", "Resonator", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::toggle("Peace Mode", "String Synth", false),
    ParamSpec::choice("Peace FX", "String Synth", EGG_FXS, 0),
    ParamSpec::lin("Width", "Output", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Volume", "Output", -24.0, 12.0, 0.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Init",
        "Marimba",
        "Kalimba",
        "Modal Bell",
        "Glass Bowl",
        "Sympathetic Strings",
        "Plucked String",
        "Sitar",
        "Stiff String",
        "FM Bells",
        "FM Bass",
        "Chord Strings",
        "Ambient String",
        "Disastrous Peace",
        "Peace Pad",
        "Peace Choir",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    let egg = (EGG, 1.0);
    match name {
        "Marimba" => set(&[(STRUCTURE, 0.27), (BRIGHTNESS, 0.3), (DAMPING, 0.35), (POSITION, 0.3), (VOLUME, 4.0)]),
        "Kalimba" => set(&[(STRUCTURE, 0.32), (BRIGHTNESS, 0.45), (DAMPING, 0.45), (POSITION, 0.15)]),
        "Modal Bell" => set(&[(STRUCTURE, 0.7), (BRIGHTNESS, 0.65), (DAMPING, 0.65), (POSITION, 0.4)]),
        "Glass Bowl" => set(&[(STRUCTURE, 0.85), (BRIGHTNESS, 0.5), (DAMPING, 0.8), (POSITION, 0.6), (VOLUME, 5.0)]),
        "Sympathetic Strings" => set(&[(MODEL, 1.0), (STRUCTURE, 0.5), (BRIGHTNESS, 0.55), (DAMPING, 0.7), (VOLUME, 3.0)]),
        "Plucked String" => set(&[(MODEL, 2.0), (STRUCTURE, 0.25), (BRIGHTNESS, 0.5), (DAMPING, 0.45), (POSITION, 0.25), (VOLUME, 3.0)]),
        "Sitar" => set(&[(MODEL, 2.0), (STRUCTURE, 0.1), (BRIGHTNESS, 0.6), (DAMPING, 0.6), (POSITION, 0.15)]),
        "Stiff String" => set(&[(MODEL, 2.0), (STRUCTURE, 0.6), (BRIGHTNESS, 0.55), (DAMPING, 0.55)]),
        "FM Bells" => set(&[(MODEL, 3.0), (STRUCTURE, 0.55), (BRIGHTNESS, 0.4), (DAMPING, 0.65), (POSITION, 0.4), (VOLUME, 0.0)]),
        "FM Bass" => set(&[(MODEL, 3.0), (STRUCTURE, 0.15), (BRIGHTNESS, 0.35), (DAMPING, 0.4), (POSITION, 0.6), (POLYPHONY, 0.0), (VOLUME, -2.0)]),
        "Chord Strings" => set(&[(MODEL, 4.0), (STRUCTURE, 0.3), (BRIGHTNESS, 0.5), (DAMPING, 0.75), (VOLUME, 5.0)]),
        "Ambient String" => set(&[(MODEL, 5.0), (STRUCTURE, 0.25), (BRIGHTNESS, 0.6), (DAMPING, 0.8), (POSITION, 0.7), (VOLUME, 3.0)]),
        "Disastrous Peace" => set(&[egg, (EGG_FX, 1.0), (STRUCTURE, 0.3), (BRIGHTNESS, 0.5), (DAMPING, 0.6), (POSITION, 0.5), (VOLUME, 6.0)]),
        "Peace Pad" => set(&[egg, (EGG_FX, 4.0), (STRUCTURE, 0.4), (BRIGHTNESS, 0.6), (DAMPING, 0.85), (POSITION, 0.7), (VOLUME, 5.0)]),
        "Peace Choir" => set(&[egg, (EGG_FX, 0.0), (STRUCTURE, 0.2), (BRIGHTNESS, 0.4), (DAMPING, 0.75), (POSITION, 0.4)]),
        _ => {}
    }
    p
}

fn model(v: f32) -> Model {
    match v.round() as i32 {
        1 => Model::SympatheticString,
        2 => Model::InharmonicString,
        3 => Model::FmVoice,
        4 => Model::SympatheticStringQuantized,
        5 => Model::StringAndReverb,
        _ => Model::Modal,
    }
}

fn params(model_v: f32, poly: u8, structure: f32, brightness: f32, damping: f32, position: f32, egg: bool, egg_fx: f32) -> Params {
    let egg_fx = match egg_fx.round() as i32 {
        1 => EggFx::Chorus,
        2 => EggFx::Reverb,
        3 => EggFx::Formant2,
        4 => EggFx::Ensemble,
        5 => EggFx::Reverb2,
        _ => EggFx::Formant,
    };
    Params { model: model(model_v), polyphony: poly, easter_egg: egg, egg_fx, structure: structure.clamp(0.0, 1.0), brightness: brightness.clamp(0.0, 1.0), damping: damping.clamp(0.0, 1.0), position: position.clamp(0.0, 1.0), chord: None }
}

const SCRATCH: usize = 4096;

pub struct RingsState {
    rings: Box<Rings>,
    l: Vec<f32>,
    r: Vec<f32>,
    /// Sounding (or a note came since it went quiet).
    active: bool,
    /// Notes wait here until the settings they are played with are in.
    pending: Vec<(f32, f32)>,
    /// Model, voices and string synth last used: changing them resets Rings.
    setup: Option<(i32, u8, bool)>,
    /// Samples since the last note (it takes Rings a moment to sound).
    since_note: usize,
    sr: f32,
}

impl RingsState {
    pub fn new(sr: f32) -> Self {
        Self { rings: Box::new(Rings::new(sr)), l: vec![0.0; SCRATCH], r: vec![0.0; SCRATCH], active: false, pending: Vec::with_capacity(32), setup: None, since_note: 0, sr }
    }

    pub fn note_on(&mut self, _p: &[f32], note: u8, vel: f32) {
        if self.pending.len() < self.pending.capacity() {
            self.pending.push((note as f32, vel));
        }
        self.active = true;
    }

    /// Rings is struck: a note rings out by itself.
    pub fn note_off(&mut self, _p: &[f32], _note: u8) {}

    pub fn all_off(&mut self) {}

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let egg = p[EGG] >= 0.5;
        // A drone keeps going without notes; otherwise quiet means idle.
        if !self.active && !egg {
            return;
        }
        let poly = [1u8, 2, 4][(p[POLYPHONY].round().max(0.0) as usize).min(2)];
        let prm = params(p[MODEL], poly, p[STRUCTURE], p[BRIGHTNESS], p[DAMPING], p[POSITION], egg, p[EGG_FX]);
        let gain = db_to_gain(p[VOLUME]);
        // Rings sends odd and even voices to opposite sides; Width blends them.
        let w = p[WIDTH].clamp(0.0, 1.0);
        let (near, far) = (0.5 + 0.5 * w, 0.5 - 0.5 * w);
        // New settings first (a short block that is not heard), then the notes.
        let setup = (p[MODEL].round() as i32, poly, egg);
        if self.setup != Some(setup) {
            self.setup = Some(setup);
            self.rings.process(&prm, None, &mut self.l[..48], &mut self.r[..48]);
        }
        for (note, vel) in self.pending.drain(..) {
            self.rings.note_on(note, vel);
            self.since_note = 0;
        }
        let n = l.len().min(r.len());
        let mut done = 0;
        while done < n {
            let len = (n - done).min(SCRATCH);
            self.rings.process(&prm, None, &mut self.l[..len], &mut self.r[..len]);
            for i in 0..len {
                let (a, b) = (self.l[i], self.r[i]);
                l[done + i] += (a * near + b * far) * gain;
                r[done + i] += (b * near + a * far) * gain;
            }
            done += len;
        }
        self.since_note = self.since_note.saturating_add(n);
        self.active = (self.since_note as f32) < 0.25 * self.sr || !self.rings.is_silent();
    }
}

pub const FX_MODEL: usize = 0;
pub const FX_NOTE: usize = 1;
pub const FX_STRUCTURE: usize = 2;
pub const FX_BRIGHTNESS: usize = 3;
pub const FX_DAMPING: usize = 4;
pub const FX_POSITION: usize = 5;
pub const FX_MIX: usize = 6;

pub static FX_SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Model", "Resonator", MODELS, 0),
    ParamSpec::int("Note", "Resonator", 24.0, 96.0, 48.0, Unit::None),
    ParamSpec::lin("Structure", "Resonator", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Brightness", "Resonator", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Damping", "Resonator", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Position", "Resonator", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.5, Unit::Percent),
];

/// Rings as an effect: the track's audio excites the resonator, tuned to a note.
pub struct RingsFx {
    rings: Box<Rings>,
    note: f32,
    input: Vec<f32>,
    l: Vec<f32>,
    r: Vec<f32>,
}

impl RingsFx {
    pub fn new(sr: f32) -> Self {
        Self { rings: Box::new(Rings::new(sr)), note: -1.0, input: vec![0.0; SCRATCH], l: vec![0.0; SCRATCH], r: vec![0.0; SCRATCH] }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let note = p[FX_NOTE].round().clamp(12.0, 108.0);
        if note != self.note {
            self.note = note;
            self.rings.note_on(note, 0.0);
        }
        let prm = params(p[FX_MODEL], 1, p[FX_STRUCTURE], p[FX_BRIGHTNESS], p[FX_DAMPING], p[FX_POSITION], false, 0.0);
        let mix = p[FX_MIX].clamp(0.0, 1.0);
        let n = l.len().min(r.len());
        let mut done = 0;
        while done < n {
            let len = (n - done).min(SCRATCH);
            for i in 0..len {
                self.input[i] = 0.5 * (l[done + i] + r[done + i]);
            }
            self.rings.process(&prm, Some(&self.input[..len]), &mut self.l[..len], &mut self.r[..len]);
            for i in 0..len {
                let k = done + i;
                l[k] = l[k] * (1.0 - mix) + self.l[i] * mix;
                r[k] = r[k] * (1.0 - mix) + self.r[i] * mix;
            }
            done += len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(name: &str, secs: f32) -> (Vec<f32>, RingsState) {
        let (l, r, s) = play2(name, secs);
        (l.iter().zip(&r).map(|(a, b)| a.abs().max(b.abs())).collect(), s)
    }

    fn play2(name: &str, secs: f32) -> (Vec<f32>, Vec<f32>, RingsState) {
        let sr = 48000.0;
        let p = preset(name);
        let mut s = RingsState::new(sr);
        s.note_on(&p, 57, 0.8);
        let n = (secs * sr) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (cl, cr) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
            s.render(&p, cl, cr);
        }
        (l, r, s)
    }

    #[test]
    fn every_preset_sounds() {
        for name in preset_names() {
            let (l, _) = play(name, 1.0);
            let peak = l.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            eprintln!("{name:<20} peak {peak:.3}");
            assert!(l.iter().all(|x| x.is_finite()), "{name}");
            assert!(peak > 0.02 && peak < 3.0, "{name}: {peak}");
        }
    }

    #[test]
    fn a_struck_note_goes_quiet_and_idle() {
        let (l, s) = play("Marimba", 6.0);
        assert!(l[l.len() - 4800..].iter().all(|x| x.abs() < 1e-3));
        assert!(!s.active);
    }

    #[test]
    fn the_effect_rings_with_its_input() {
        let mut fx = RingsFx::new(48000.0);
        let p = crate::params::defaults(FX_SPECS);
        let ctx = FxCtx { sr: 48000.0, bpm: 120.0, beat: 0.0, playing: false };
        let mut rng = crate::dsp::Rng::new(1);
        let mut l: Vec<f32> = (0..48000).map(|i| if i < 2000 { rng.bipolar() * 0.5 } else { 0.0 }).collect();
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            fx.process(&p, cl, cr, &ctx);
        }
        // Still ringing well after the burst.
        let tail = l[12000..24000].iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(tail > 0.005 && l.iter().all(|x| x.is_finite()), "{tail}");
    }
}
