//! Physical models from the Synthesis ToolKit (Perry R. Cook and Gary P.
//! Scavone, MIT-style license; see `krabton_stk`), as four polyphonic
//! instruments: Winds, Strings, Mallets and Voice. Each has a model choice
//! and up to seven controls, which are that model's own STK controls.

use super::{db_to_gain, midi_to_freq};
use crate::params::ParamSpec;
use krabton_stk::{ControlInfo, Instrument};

pub const VOICES: usize = 0;
pub const VOLUME: usize = 1;
pub const CTL0: usize = 2;
pub const N_CTL: usize = 7;
pub const MODEL: usize = CTL0 + N_CTL;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Winds,
    Strings,
    Mallets,
    Voice,
}

/// A model of a family: the STK instrument, its preset (ModalBar,
/// BandedWG) or phoneme (VoicForm), and a level trim.
#[derive(Clone, Copy)]
pub struct Model {
    pub id: &'static str,
    pub name: &'static str,
    pub preset: Option<usize>,
    pub phoneme: Option<usize>,
    pub trim: f32,
}

const fn m(id: &'static str, name: &'static str, trim: f32) -> Model {
    Model { id, name, preset: None, phoneme: None, trim }
}

const fn pre(id: &'static str, name: &'static str, preset: usize, trim: f32) -> Model {
    Model { id, name, preset: Some(preset), phoneme: None, trim }
}

const fn ph(name: &'static str, phoneme: usize) -> Model {
    Model { id: "voicform", name, preset: None, phoneme: Some(phoneme), trim: 0.75 }
}

pub static WINDS: &[Model] = &[
    m("flute", "Flute", 1.2),
    m("clarinet", "Clarinet", 1.0),
    m("saxofony", "Saxophone", 0.5),
    m("brass", "Brass", 1.7),
    m("blowbotl", "Blown Bottle", 0.5),
    m("blowhole", "Blow Hole", 1.0),
    m("whistle", "Whistle", 0.8),
];
pub static STRINGS: &[Model] = &[m("bowed", "Bowed", 1.5), m("plucked", "Plucked", 1.1), m("mandolin", "Mandolin", 2.4), m("sitar", "Sitar", 2.2), m("stifkarp", "Stiff String", 0.85)];
pub static MALLETS: &[Model] = &[
    pre("modalbar", "Marimba", 0, 0.31),
    pre("modalbar", "Vibraphone", 1, 0.34),
    pre("modalbar", "Agogo", 2, 1.7),
    pre("modalbar", "Wood 1", 3, 1.5),
    pre("modalbar", "Reso", 4, 1.0),
    pre("modalbar", "Wood 2", 5, 1.7),
    pre("modalbar", "Beats", 6, 2.0),
    pre("modalbar", "Two Fixed", 7, 1.1),
    pre("modalbar", "Clump", 8, 0.43),
    pre("bandedwg", "Uniform Bar", 0, 16.0),
    pre("bandedwg", "Tuned Bar", 1, 6.7),
    pre("bandedwg", "Glass Harmonica", 2, 14.0),
    pre("bandedwg", "Tibetan Bowl", 3, 4.0),
];
pub static VOICE: &[Model] = &[
    ph("Eee", 0),
    ph("Ihh", 1),
    ph("Ehh", 2),
    ph("Aaa", 3),
    ph("Ahh", 4),
    ph("Aww", 5),
    ph("Ohh", 6),
    ph("Uhh", 7),
    ph("Uuu", 8),
    ph("Ooo", 9),
    ph("Rrr", 10),
    ph("Lll", 11),
    ph("Mmm", 12),
    ph("Nnn", 13),
    ph("Nng", 14),
    ph("Ngg", 15),
    ph("Fff", 16),
    ph("Sss", 17),
    ph("Thh", 18),
    ph("Shh", 19),
    ph("Xxx", 20),
    ph("Hee", 21),
    ph("Hoo", 22),
    ph("Hah", 23),
    ph("Bbb", 24),
    ph("Ddd", 25),
    ph("Jjj", 26),
    ph("Ggg", 27),
    ph("Vvv", 28),
    ph("Zzz", 29),
    ph("Thz", 30),
    ph("Zhh", 31),
];

pub fn models(f: Family) -> &'static [Model] {
    match f {
        Family::Winds => WINDS,
        Family::Strings => STRINGS,
        Family::Mallets => MALLETS,
        Family::Voice => VOICE,
    }
}

pub fn specs(f: Family) -> &'static [ParamSpec] {
    match f {
        Family::Winds => super::stk_winds::SPECS,
        Family::Strings => super::stk_strings::SPECS,
        Family::Mallets => super::stk_mallets::SPECS,
        Family::Voice => super::stk_voice::SPECS,
    }
}

/// The model of `p` (its index, clamped).
pub fn model_of(f: Family, p: &[f32]) -> usize {
    (p.get(MODEL).copied().unwrap_or(0.0).round().max(0.0) as usize).min(models(f).len() - 1)
}

/// The controls a model shows, in order: its STK controls without the ones
/// the model choice already sets (presets, phonemes).
pub fn controls(model: &Model) -> [Option<ControlInfo>; N_CTL] {
    let mut out = [None; N_CTL];
    let Some(info) = krabton_stk::MODELS.iter().find(|m| m.id == model.id) else { return out };
    let skip = |n: u8| (model.preset.is_some() && n == 16) || (model.phoneme.is_some() && n == 4);
    for (slot, c) in out.iter_mut().zip(info.controls.iter().filter(|c| !skip(c.number))) {
        *slot = Some(*c);
    }
    out
}

/// A short label for an STK control name, to fit under a knob.
pub fn short_name(name: &'static str) -> &'static str {
    match name {
        "Blow Position" => "Blow Pos",
        "Blowing Frequency Modulation" => "Blow Mod",
        "Body Size" => "Body",
        "Bow Position" => "Bow Pos",
        "Bow Pressure" => "Bow Press",
        "Bow Velocity" => "Bow Speed",
        "Breath Pressure" => "Breath",
        "Direct Stick Mix" => "Stick Mix",
        "Fipple Modulation Frequency" => "Fipple Rate",
        "Fipple Modulation Gain" => "Fipple Amt",
        "Lip Tension" => "Lips",
        "Loudness (Spectral Tilt)" => "Loudness",
        "Microphone Position" => "Mic Pos",
        "Noise Cutoff" => "Noise Tone",
        "Pickup Position" => "Pickup",
        "Pluck Position" => "Pluck Pos",
        "Reed Aperture" => "Aperture",
        "Reed Stiffness" => "Reed",
        "Register State" => "Register",
        "Set Striking" => "Strike",
        "Slide Length" => "Slide",
        "Stick Hardness" => "Hardness",
        "Stick Position" => "Stick Pos",
        "String Detuning" => "Detune",
        "String Stretch" => "Stretch",
        "String Sustain" => "Sustain",
        "Tonehole State" => "Tonehole",
        "Vibrato Frequency" => "Vib Rate",
        "Vibrato Gain" => "Vib Depth",
        "Voiced/Unvoiced Mix" => "Breathy",
        "Volume" => "Level",
        n => n,
    }
}

/// Parameters for `model` with its controls at STK's defaults.
pub fn defaults_for(f: Family, model: usize) -> Vec<f32> {
    let mut p = crate::params::defaults(specs(f));
    let model = model.min(models(f).len() - 1);
    p[MODEL] = model as f32;
    for (k, c) in controls(&models(f)[model]).iter().enumerate() {
        p[CTL0 + k] = c.map(|c| c.default / 128.0).unwrap_or(0.5);
    }
    p
}

/// The presets of a family: every model at its defaults, plus a few more.
pub fn preset_names(f: Family) -> &'static [&'static str] {
    match f {
        Family::Winds => &["Flute", "Breathy Flute", "Clarinet", "Soft Clarinet", "Saxophone", "Growl Sax", "Brass", "Bright Brass", "Blown Bottle", "Blow Hole", "Whistle"],
        Family::Strings => &["Bowed", "Pressed Bow", "Plucked", "Mandolin", "Sitar", "Stiff String"],
        Family::Mallets => &["Marimba", "Vibraphone", "Agogo", "Wood 1", "Reso", "Wood 2", "Beats", "Two Fixed", "Clump", "Uniform Bar", "Tuned Bar", "Glass Harmonica", "Tibetan Bowl"],
        Family::Voice => &["Eee", "Ahh", "Ooo", "Ohh", "Mmm", "Hum", "Whisper", "Choir Aah"],
    }
}

pub fn preset(f: Family, name: &str) -> Vec<f32> {
    let by_name = |n: &str| models(f).iter().position(|m| m.name == n).unwrap_or(0);
    // (model, [(STK control number, value 0..128)])
    let (model, tweaks): (usize, &[(u8, f32)]) = match (f, name) {
        (Family::Winds, "Breathy Flute") => (by_name("Flute"), &[(4, 100.0), (1, 30.0)]),
        (Family::Winds, "Soft Clarinet") => (by_name("Clarinet"), &[(2, 40.0), (128, 80.0)]),
        (Family::Winds, "Growl Sax") => (by_name("Saxophone"), &[(4, 90.0), (1, 45.0)]),
        (Family::Winds, "Bright Brass") => (by_name("Brass"), &[(2, 66.0), (1, 15.0)]),
        (Family::Strings, "Pressed Bow") => (by_name("Bowed"), &[(2, 100.0), (1, 20.0)]),
        (Family::Voice, "Hum") => (by_name("Nnn"), &[]),
        (Family::Voice, "Whisper") => (by_name("Hah"), &[(2, 64.0)]),
        (Family::Voice, "Choir Aah") => (by_name("Ahh"), &[(1, 50.0), (11, 70.0)]),
        _ => (by_name(name), &[]),
    };
    let mut p = defaults_for(f, model);
    let ctl = controls(&models(f)[model]);
    for &(num, v) in tweaks {
        if let Some(k) = ctl.iter().position(|c| c.map(|c| c.number) == Some(num)) {
            p[CTL0 + k] = v / 128.0;
        }
    }
    // Struck and plucked notes ring on: more voices.
    p[VOICES] = match f {
        Family::Winds | Family::Voice => 4.0,
        _ => 6.0,
    };
    p
}

const MAX_VOICES: usize = 8;
const PANS: [f32; MAX_VOICES] = [-0.2, 0.2, -0.1, 0.1, -0.3, 0.3, 0.0, 0.0];

struct Voice {
    inst: Option<Box<dyn Instrument>>,
    note: u8,
    gate: bool,
    active: bool,
    age: u64,
    /// Samples spent quiet after the note was let go.
    quiet: u32,
}

pub struct StkState {
    family: Family,
    sr: f32,
    voices: Vec<Voice>,
    model: Option<usize>,
    ctl: [Option<ControlInfo>; N_CTL],
    /// Control values (0..1) the voices have.
    sent: [f32; N_CTL],
    counter: u64,
}

impl StkState {
    pub fn new(family: Family, sr: f32) -> Self {
        Self {
            family,
            sr,
            voices: (0..MAX_VOICES).map(|_| Voice { inst: None, note: 0, gate: false, active: false, age: 0, quiet: 0 }).collect(),
            model: None,
            ctl: [None; N_CTL],
            sent: [0.0; N_CTL],
            counter: 0,
        }
    }

    /// Sets up the voices for the model in `p`, and sends changed controls.
    fn prepare(&mut self, p: &[f32]) {
        let k = model_of(self.family, p);
        let model = models(self.family)[k];
        if self.model != Some(k) {
            let same_inst = self.model.map(|old| models(self.family)[old].id == model.id).unwrap_or(false);
            self.model = Some(k);
            self.ctl = controls(&model);
            for v in self.voices.iter_mut() {
                if !same_inst || v.inst.is_none() {
                    v.inst = krabton_stk::create(model.id, self.sr);
                    v.active = false;
                    v.gate = false;
                }
                if let Some(inst) = v.inst.as_mut() {
                    if let Some(i) = model.preset {
                        inst.set_preset(i);
                    }
                    if let Some(i) = model.phoneme {
                        inst.control_change(4, 32.0 + i as f32);
                    }
                }
            }
            // New voices start at STK's defaults.
            for (s, c) in self.sent.iter_mut().zip(self.ctl.iter()) {
                *s = c.map(|c| c.default / 128.0).unwrap_or(0.0);
            }
        }
        for slot in 0..N_CTL {
            let Some(c) = self.ctl[slot] else { continue };
            let v = p[CTL0 + slot].clamp(0.0, 1.0);
            if (v - self.sent[slot]).abs() > 1e-6 {
                self.sent[slot] = v;
                for voice in self.voices.iter_mut() {
                    if let Some(inst) = voice.inst.as_mut() {
                        inst.control_change(c.number, v * 128.0);
                    }
                }
            }
        }
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() <= MODEL {
            return;
        }
        self.prepare(p);
        let poly = (p[VOICES].round() as usize).clamp(1, MAX_VOICES);
        self.counter += 1;
        let pool = &mut self.voices[..poly];
        let pick = pool
            .iter()
            .position(|v| v.active && v.note == note)
            .or_else(|| pool.iter().position(|v| !v.active))
            .or_else(|| pool.iter().enumerate().filter(|(_, v)| !v.gate).min_by_key(|(_, v)| v.age).map(|(i, _)| i))
            .unwrap_or_else(|| pool.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0));
        let v = &mut pool[pick];
        if let Some(inst) = v.inst.as_mut() {
            inst.note_on(midi_to_freq(note as f32), vel.clamp(0.05, 1.0));
        }
        v.note = note;
        v.gate = true;
        v.active = true;
        v.quiet = 0;
        v.age = self.counter;
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        for v in self.voices.iter_mut().filter(|v| v.gate && v.note == note) {
            if let Some(inst) = v.inst.as_mut() {
                inst.note_off(0.5);
            }
            v.gate = false;
        }
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut().filter(|v| v.gate) {
            if let Some(inst) = v.inst.as_mut() {
                inst.note_off(0.5);
            }
            v.gate = false;
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() <= MODEL {
            return;
        }
        self.prepare(p);
        let Some(k) = self.model else { return };
        let gain = models(self.family)[k].trim * db_to_gain(p[VOLUME]);
        let n = l.len().min(r.len());
        let quiet_limit = (0.3 * self.sr) as u32;
        for (vi, v) in self.voices.iter_mut().enumerate() {
            if !v.active {
                continue;
            }
            let Some(inst) = v.inst.as_mut() else { continue };
            let (gl, gr) = super::pan_gains(PANS[vi]);
            for i in 0..n {
                let y = inst.tick() * gain;
                l[i] += y * gl;
                r[i] += y * gr;
                if !v.gate && y.abs() < 1e-4 {
                    v.quiet += 1;
                } else {
                    v.quiet = 0;
                }
            }
            if v.quiet > quiet_limit {
                v.active = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_sounds_and_stops() {
        for f in [Family::Winds, Family::Strings, Family::Mallets, Family::Voice] {
            for name in preset_names(f) {
                let p = preset(f, name);
                let mut s = StkState::new(f, 48000.0);
                s.note_on(&p, 57, 0.8);
                let (mut l, mut r) = (vec![0.0; 48000], vec![0.0; 48000]);
                for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
                    s.render(&p, cl, cr);
                }
                let peak = l.iter().fold(0.0f32, |m, x| m.max(x.abs()));
                eprintln!("{f:?} {name:<16} peak {peak:.3}");
                assert!(l.iter().all(|x| x.is_finite()), "{name}");
                if std::env::var_os("STK_CAL").is_some() {
                    continue;
                }
                assert!(peak > 0.03 && peak < 3.0, "{f:?} {name}: {peak}");
                s.note_off(&p, 57);
                let (mut l, mut r) = (vec![0.0; 48000 * 5], vec![0.0; 48000 * 5]);
                for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
                    s.render(&p, cl, cr);
                }
                assert!(s.voices.iter().all(|v| !v.active), "{f:?} {name}: still playing");
            }
        }
    }

    #[test]
    fn controls_show_the_models_own_names() {
        let c = controls(&WINDS[1]);
        assert_eq!(c[0].map(|c| c.name), Some("Reed Stiffness"));
        // Mallets choose their preset with the model; Voice its phoneme.
        assert!(controls(&MALLETS[0]).iter().flatten().all(|c| c.number != 16));
        assert!(controls(&VOICE[0]).iter().flatten().all(|c| c.number != 4));
    }
}
