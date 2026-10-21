//! The modulation matrix: project-wide modulators (LFOs, envelopes,
//! followers, step sequences, macros, note sources and modifiers) routed to
//! any parameter: instrument, audio effect, MIDI effect, mixer, or another
//! modulator. Every connection is a [`ModRoute`] with a bipolar amount.
//!
//! The engine runs the matrix at control rate (every block of at most
//! [`MOD_BLOCK`] frames): modulators advance, then each modulated parameter
//! set is recomputed from the project value plus the routed offsets.

use crate::dsp::{analog, braids_synth, elements_synth, fm, macro_osc, sampler, shaker_synth, synth, stk_voice, stk_mallets, stk_strings, stk_winds, organ, rings_synth, Rng};
use crate::params::{ParamKind, ParamSpec, Unit};
use crate::project::{Instrument, Project};
use serde::{Deserialize, Serialize};

/// Control-rate block size while any modulator exists.
pub const MOD_BLOCK: usize = 64;
/// Most modulators a project can have (their outputs are reported to the UI).
pub const MAX_MODS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModKind {
    Lfo,
    Envelope,
    Follower,
    Steps,
    Macro,
    Notes,
    Math,
    Shaper,
    Tides,
    ModWheel,
    Joystick,
}

pub const LFO_SHAPES: &[&str] = &["Sine", "Triangle", "Saw Up", "Saw Down", "Square", "S&H", "Smooth Random"];
pub const BEAT_RATES: &[&str] = &["1/32", "1/16", "1/8 T", "1/8", "1/4 T", "1/4", "1/2", "1 bar", "2 bars", "4 bars", "8 bars"];
const BEAT_RATE_BEATS: [f64; 11] = [0.125, 0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
pub const NOTE_OUTPUTS: &[&str] = &["Velocity", "Pitch", "Gate", "Random"];
pub const MATH_OPS: &[&str] = &["A + B", "A x B", "Min", "Max", "A - B", "Average", "A > B"];
pub const SHAPER_MODES: &[&str] = &["Normal", "Invert", "Rectify", "Unipolar"];

pub static LFO: &[ParamSpec] = &[
    ParamSpec::choice("Shape", "LFO", LFO_SHAPES, 0),
    ParamSpec::toggle("Sync", "LFO", true),
    ParamSpec::choice("Beat Rate", "LFO", BEAT_RATES, 5),
    ParamSpec::log("Rate", "LFO", 0.02, 20.0, 1.0, Unit::Hz),
    ParamSpec::lin("Phase", "Output", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Depth", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];
pub static ENVELOPE: &[ParamSpec] = &[
    ParamSpec::log("Attack", "Envelope", 0.001, 4.0, 0.005, Unit::Sec),
    ParamSpec::log("Decay", "Envelope", 0.005, 8.0, 0.3, Unit::Sec),
    ParamSpec::lin("Sustain", "Envelope", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Release", "Envelope", 0.005, 8.0, 0.2, Unit::Sec),
    ParamSpec::toggle("Legato", "Trigger", false),
    ParamSpec::lin("Depth", "Trigger", 0.0, 1.0, 1.0, Unit::Percent),
];
pub static FOLLOWER: &[ParamSpec] = &[
    ParamSpec::lin("Gain", "Follower", -12.0, 24.0, 6.0, Unit::Db),
    ParamSpec::log("Attack", "Follower", 0.001, 0.5, 0.005, Unit::Sec),
    ParamSpec::log("Release", "Follower", 0.01, 2.0, 0.15, Unit::Sec),
    ParamSpec::lin("Depth", "Follower", 0.0, 1.0, 1.0, Unit::Percent),
];
pub static STEPS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Steps", BEAT_RATES, 1),
    ParamSpec::int("Length", "Steps", 1.0, 16.0, 16.0, Unit::None),
    ParamSpec::lin("Smooth", "Steps", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Bipolar", "Output", false),
    ParamSpec::lin("Depth", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];
pub static MACRO: &[ParamSpec] = &[ParamSpec::lin("Value", "Macro", 0.0, 1.0, 0.0, Unit::Percent)];
pub static NOTES: &[ParamSpec] = &[
    ParamSpec::choice("Output", "Notes", NOTE_OUTPUTS, 0),
    ParamSpec::log("Glide", "Notes", 0.001, 2.0, 0.01, Unit::Sec),
];
pub static MATH: &[ParamSpec] = &[
    ParamSpec::lin("A", "Inputs", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("B", "Inputs", -1.0, 1.0, 1.0, Unit::None),
    ParamSpec::choice("Op", "Math", MATH_OPS, 1),
    ParamSpec::lin("Gain", "Math", 0.0, 2.0, 1.0, Unit::None),
];
pub static SHAPER: &[ParamSpec] = &[
    ParamSpec::lin("Input", "Input", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Gain", "Shape", 0.0, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Offset", "Shape", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Curve", "Shape", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::int("Steps", "Shape", 0.0, 16.0, 0.0, Unit::None),
    ParamSpec::log("Slew", "Shape", 0.001, 2.0, 0.001, Unit::Sec),
    ParamSpec::choice("Mode", "Shape", SHAPER_MODES, 0),
];

pub static TIDES: &[ParamSpec] = &[
    ParamSpec::choice("Mode", "Tides", crate::dsp::tides::RAMP_MODES, 1),
    ParamSpec::choice("Outputs", "Tides", crate::dsp::tides::OUTPUT_MODES, 2),
    ParamSpec::toggle("Sync", "Rate", true),
    ParamSpec::choice("Beat Rate", "Rate", BEAT_RATES, 7),
    ParamSpec::log("Rate", "Rate", 0.02, 20.0, 0.5, Unit::Hz),
    ParamSpec::lin("Shape", "Slope", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Slope", "Slope", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Smoothness", "Slope", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Shift", "Outputs", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Out", "Outputs", 1.0, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Depth", "Outputs", 0.0, 1.0, 1.0, Unit::Percent),
];

pub static MOD_WHEEL: &[ParamSpec] = &[
    ParamSpec::log("Smooth", "Wheel", 0.001, 1.0, 0.02, Unit::Sec),
    ParamSpec::lin("Depth", "Wheel", 0.0, 1.0, 1.0, Unit::Percent),
];

pub static JOYSTICK: &[ParamSpec] = &[
    ParamSpec::choice("Axis", "Stick", &["X", "Y"], 0),
    ParamSpec::log("Smooth", "Stick", 0.001, 1.0, 0.02, Unit::Sec),
    ParamSpec::lin("Depth", "Stick", 0.0, 1.0, 1.0, Unit::Percent),
];

impl ModKind {
    pub const ALL: [ModKind; 11] =
        [ModKind::Lfo, ModKind::Envelope, ModKind::Follower, ModKind::Steps, ModKind::Macro, ModKind::Notes, ModKind::Math, ModKind::Shaper, ModKind::Tides, ModKind::ModWheel, ModKind::Joystick];

    pub fn name(self) -> &'static str {
        match self {
            ModKind::Lfo => "LFO",
            ModKind::Envelope => "Envelope",
            ModKind::Follower => "Follower",
            ModKind::Steps => "Steps",
            ModKind::Macro => "Macro",
            ModKind::Notes => "Notes",
            ModKind::Math => "Math",
            ModKind::Shaper => "Shaper",
            ModKind::Tides => "Tides",
            ModKind::ModWheel => "Mod Wheel",
            ModKind::Joystick => "Joystick",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            ModKind::Lfo => "Low-frequency oscillator, free or synced to the tempo",
            ModKind::Envelope => "ADSR triggered by the notes of a track (after its MIDI effects)",
            ModKind::Follower => "Follows the loudness of a track: sidechain ducking, auto-wah, ...",
            ModKind::Steps => "A step sequence of values, synced to the tempo",
            ModKind::Macro => "One knob that moves many parameters",
            ModKind::Notes => "Velocity, pitch, gate or a random value per note of a track",
            ModKind::Math => "Combines two inputs: route modulators into A and B",
            ModKind::Shaper => "Reshapes its input: gain, offset, curve, quantize, slew, rectify",
            ModKind::Tides => "A tidal modulator after Mutable Instruments Tides: a slope looping with the tempo, or rising and falling on the notes of a track (AD, AR), its curve, peak and smoothness set by Shape, Slope and Smoothness; four outputs in phase, amplitude or frequency relations (Shift)",
            ModKind::ModWheel => "The mod wheel of a track: a MIDI keyboard's (CC 1), or a controller's triggers in Big Picture",
            ModKind::Joystick => "One way (X or Y) of a track's joystick: the right stick in Big Picture's XY play mode, -1 to 1 around the knob",
        }
    }

    pub fn specs(self) -> &'static [ParamSpec] {
        match self {
            ModKind::Lfo => LFO,
            ModKind::Envelope => ENVELOPE,
            ModKind::Follower => FOLLOWER,
            ModKind::Steps => STEPS,
            ModKind::Macro => MACRO,
            ModKind::Notes => NOTES,
            ModKind::Math => MATH,
            ModKind::Shaper => SHAPER,
            ModKind::Tides => TIDES,
            ModKind::ModWheel => MOD_WHEEL,
            ModKind::Joystick => JOYSTICK,
        }
    }

    /// Modulators that listen to a track (notes or audio).
    pub fn uses_track(self) -> bool {
        matches!(self, ModKind::Envelope | ModKind::Follower | ModKind::Notes | ModKind::Tides | ModKind::ModWheel | ModKind::Joystick)
    }

    /// True when the output goes from -1 to 1 (otherwise 0 to 1).
    pub fn bipolar(self) -> bool {
        matches!(self, ModKind::Lfo | ModKind::Math | ModKind::Shaper | ModKind::Tides | ModKind::Joystick)
    }

    pub fn default_data(self) -> Vec<u8> {
        match self {
            ModKind::Steps => vec![255, 0, 160, 60, 220, 40, 120, 0, 255, 90, 180, 30, 200, 70, 140, 10],
            _ => Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Modulator {
    pub id: u64,
    pub name: String,
    pub kind: ModKind,
    pub params: Vec<f32>,
    /// Step values (Steps).
    #[serde(default)]
    pub data: Vec<u8>,
    /// Track whose notes or audio drive it (Envelope, Follower, Notes).
    #[serde(default)]
    pub track: Option<u64>,
}

/// A parameter set that can be modulated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModOwner {
    /// The instrument of a track (Synth, Sampler, FM, ...).
    Instrument(u64),
    /// An audio effect (on a track, return or the master).
    Device(u64),
    MidiDevice(u64),
    Modulator(u64),
    /// Where a track (by id) stands on the Map.
    Map(u64),
}

/// Where a route goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModTarget {
    Param { owner: ModOwner, param: u16 },
    /// Track or return volume: the amount scales the gain (-100% = silent).
    Volume(u64),
    Pan(u64),
    Send { track: u64, send: u16 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModRoute {
    pub id: u64,
    pub source: u64,
    pub target: ModTarget,
    /// -1..1: the share of the parameter's range the source sweeps.
    pub amount: f32,
}

/// Instrument parameter specs, for instruments that can be modulated.
pub fn instrument_specs(i: &Instrument) -> Option<&'static [ParamSpec]> {
    match i {
        Instrument::Synth { .. } => Some(synth::SPECS),
        Instrument::Sampler { .. } => Some(sampler::SPECS),
        Instrument::Sf2 { .. } => Some(crate::dsp::sf2::SPECS),
        Instrument::StreetOrgan { .. } => Some(crate::dsp::street_organ::SPECS),
        Instrument::Singer { .. } => Some(crate::singer::SPECS),
        Instrument::Hyper { .. } => Some(crate::hyper::SPECS),
        Instrument::Fm { .. } => Some(fm::SPECS),
        Instrument::Macro { .. } => Some(macro_osc::SPECS),
        Instrument::Braids { .. } => Some(braids_synth::SPECS),
        Instrument::Elements { .. } => Some(elements_synth::SPECS),
        Instrument::Analog { .. } => Some(analog::SPECS),
        Instrument::Shakers { .. } => Some(shaker_synth::SPECS),
        Instrument::Modular { .. } => Some(crate::modular::SPECS),
        Instrument::Voice { .. } => Some(stk_voice::SPECS),
        Instrument::Mallets { .. } => Some(stk_mallets::SPECS),
        Instrument::Strings { .. } => Some(stk_strings::SPECS),
        Instrument::Winds { .. } => Some(stk_winds::SPECS),
        Instrument::Organ { .. } => Some(organ::SPECS),
        Instrument::Rings { .. } => Some(rings_synth::SPECS),
        Instrument::Monk { .. } => Some(crate::dsp::monk::SPECS),
        Instrument::Piano { .. } => Some(crate::piano::SPECS),
        Instrument::Bassline { .. } => Some(crate::dsp::bassline::SPECS),
        Instrument::Mono101 { .. } => Some(crate::dsp::mono101::SPECS),
        Instrument::Drone { .. } => Some(crate::dsp::drone::SPECS),
        Instrument::Waves { .. } => Some(crate::dsp::waves::SPECS),
        Instrument::Guitar { .. } => Some(crate::dsp::guitar::SPECS),
        Instrument::WindChimes { .. } => Some(crate::dsp::wind_chimes::SPECS),
        Instrument::BeadsSynth { .. } => Some(crate::dsp::beads_synth::SPECS),
        Instrument::Birds { .. } => Some(crate::dsp::birds::SPECS),
        Instrument::Nature { .. } => Some(crate::dsp::nature::SPECS),
        Instrument::PanFlute { .. } => Some(crate::dsp::pan_flute::SPECS),
        Instrument::Melodica { .. } => Some(crate::dsp::melodica::SPECS),
        Instrument::Harmonica { .. } => Some(crate::dsp::harmonica::SPECS),
        Instrument::Calculator { .. } => Some(crate::dsp::calculator::SPECS),
        Instrument::Console { .. } => Some(crate::dsp::console::SPECS),
        Instrument::Handheld { .. } => Some(crate::dsp::handheld::SPECS),
        Instrument::Micro { .. } => Some(crate::dsp::micro::SPECS),
        Instrument::Warp { .. } => Some(crate::dsp::warp::SPECS),
        Instrument::Hybrid { .. } => Some(crate::dsp::hybrid::SPECS),
        Instrument::ChipFm { .. } => Some(crate::dsp::chipfm::SPECS),
        Instrument::V2 { .. } => Some(crate::dsp::v2::SPECS),
        Instrument::Oidos { .. } => Some(crate::dsp::oidos::SPECS),
        Instrument::Quartet { .. } => Some(crate::dsp::quartet::SPECS),
        Instrument::Orchestra { .. } => Some(crate::orchestra::SPECS),
        Instrument::Facet { params, .. } => Some(crate::dsp::facet::specs_for(crate::dsp::facet::engine_of(params))),
        _ => None,
    }
}

pub fn instrument_params(i: &Instrument) -> Option<&[f32]> {
    match i {
        Instrument::Synth { params, .. } | Instrument::Sampler { params, .. } | Instrument::Fm { params, .. } | Instrument::Macro { params, .. } | Instrument::Braids { params, .. } | Instrument::Elements { params, .. } | Instrument::Analog { params, .. } | Instrument::Shakers { params, .. } | Instrument::Modular { params, .. } | Instrument::Voice { params, .. } | Instrument::Mallets { params, .. } | Instrument::Strings { params, .. } | Instrument::Winds { params, .. } | Instrument::Organ { params, .. } | Instrument::Rings { params, .. } | Instrument::Monk { params, .. } | Instrument::Piano { params, .. } | Instrument::Bassline { params, .. } | Instrument::Mono101 { params, .. } | Instrument::Drone { params, .. } | Instrument::Waves { params, .. } | Instrument::Guitar { params, .. } | Instrument::WindChimes { params, .. } | Instrument::Birds { params, .. } | Instrument::Nature { params, .. } | Instrument::PanFlute { params, .. } | Instrument::Melodica { params, .. } | Instrument::Harmonica { params, .. } | Instrument::Calculator { params, .. } | Instrument::Console { params, .. } | Instrument::Handheld { params, .. } | Instrument::Micro { params, .. } | Instrument::Warp { params, .. } | Instrument::Hybrid { params, .. } | Instrument::Quartet { params, .. } | Instrument::ChipFm { params, .. } | Instrument::V2 { params, .. } | Instrument::Oidos { params, .. } | Instrument::Orchestra { params, .. } | Instrument::Facet { params, .. } | Instrument::BeadsSynth { params, .. } | Instrument::Sf2 { params, .. } | Instrument::StreetOrgan { params, .. } | Instrument::Singer { params, .. } | Instrument::Hyper { params, .. } => Some(params),
        _ => None,
    }
}

/// [`instrument_params`], to change them.
pub fn instrument_params_mut(i: &mut Instrument) -> Option<&mut Vec<f32>> {
    match i {
        Instrument::Synth { params, .. } | Instrument::Sampler { params, .. } | Instrument::Fm { params, .. } | Instrument::Macro { params, .. } | Instrument::Braids { params, .. } | Instrument::Elements { params, .. } | Instrument::Analog { params, .. } | Instrument::Shakers { params, .. } | Instrument::Modular { params, .. } | Instrument::Voice { params, .. } | Instrument::Mallets { params, .. } | Instrument::Strings { params, .. } | Instrument::Winds { params, .. } | Instrument::Organ { params, .. } | Instrument::Rings { params, .. } | Instrument::Monk { params, .. } | Instrument::Piano { params, .. } | Instrument::Bassline { params, .. } | Instrument::Mono101 { params, .. } | Instrument::Drone { params, .. } | Instrument::Waves { params, .. } | Instrument::Guitar { params, .. } | Instrument::WindChimes { params, .. } | Instrument::Birds { params, .. } | Instrument::Nature { params, .. } | Instrument::PanFlute { params, .. } | Instrument::Melodica { params, .. } | Instrument::Harmonica { params, .. } | Instrument::Calculator { params, .. } | Instrument::Console { params, .. } | Instrument::Handheld { params, .. } | Instrument::Micro { params, .. } | Instrument::Warp { params, .. } | Instrument::Hybrid { params, .. } | Instrument::Quartet { params, .. } | Instrument::ChipFm { params, .. } | Instrument::V2 { params, .. } | Instrument::Oidos { params, .. } | Instrument::Orchestra { params, .. } | Instrument::Facet { params, .. } | Instrument::BeadsSynth { params, .. } | Instrument::Sf2 { params, .. } | Instrument::StreetOrgan { params, .. } | Instrument::Singer { params, .. } | Instrument::Hyper { params, .. } => Some(params),
        _ => None,
    }
}

/// Applies a normalized offset to a value: bipolar parameters (like pan) are
/// swept from their center, so 100% covers the whole range either way.
pub fn modulate(spec: &ParamSpec, base: f32, offset: f32) -> f32 {
    if offset == 0.0 {
        return base;
    }
    let bipolar = spec.min < 0.0 && (spec.max + spec.min).abs() < 1e-6;
    let span = if bipolar { 0.5 } else { 1.0 };
    let n = spec.to_norm(base) + offset * span;
    match spec.kind {
        // Discrete values move in steps; continuous ones stay unrounded.
        ParamKind::Int | ParamKind::Choice(_) | ParamKind::Toggle => spec.from_norm(n),
        _ => {
            let n = n.clamp(0.0, 1.0);
            match spec.kind {
                ParamKind::Log => spec.min * (spec.max / spec.min).powf(n),
                _ => spec.min + (spec.max - spec.min) * n,
            }
        }
    }
}

impl Project {
    pub fn make_modulator(&mut self, kind: ModKind) -> Modulator {
        let n = self.modulators.iter().filter(|m| m.kind == kind).count() + 1;
        Modulator {
            id: self.alloc_id(),
            name: format!("{} {n}", kind.name()),
            kind,
            params: crate::params::defaults(kind.specs()),
            data: kind.default_data(),
            track: None,
        }
    }

    /// Connects (or re-weights) a source to a target.
    pub fn add_route(&mut self, source: u64, target: ModTarget, amount: f32) -> u64 {
        if let Some(r) = self.mod_routes.iter_mut().find(|r| r.source == source && r.target == target) {
            r.amount = amount;
            return r.id;
        }
        let id = self.alloc_id();
        self.mod_routes.push(ModRoute { id, source, target, amount });
        id
    }

    /// The specs of a modulatable parameter set, if it exists.
    pub fn owner_specs(&self, owner: ModOwner) -> Option<&'static [ParamSpec]> {
        match owner {
            ModOwner::Instrument(t) => self.tracks.iter().find(|tr| tr.id == t)?.instrument.as_ref().and_then(instrument_specs),
            ModOwner::Device(d) => self.all_devices().find(|x| x.id == d).map(|x| x.kind.specs()),
            ModOwner::MidiDevice(d) => self.tracks.iter().flat_map(|t| &t.midi_devices).find(|x| x.id == d).map(|x| x.kind.specs()),
            ModOwner::Modulator(m) => self.modulators.iter().find(|x| x.id == m).map(|x| x.kind.specs()),
            // (kept while the track is off the map: back on, it moves again)
            ModOwner::Map(t) => self.tracks.iter().any(|tr| tr.id == t).then_some(crate::dsp::fx::stage::PLACE_SPECS),
        }
    }

    pub fn all_devices(&self) -> impl Iterator<Item = &crate::project::Device> {
        self.tracks.iter().flat_map(|t| &t.devices).chain(self.returns.iter().flat_map(|r| &r.devices)).chain(&self.master.devices)
    }

    /// A short name for a route target, like "Bass · Cutoff".
    pub fn target_label(&self, target: ModTarget) -> (String, String) {
        let track_name = |id: u64| {
            self.tracks.iter().find(|t| t.id == id).map(|t| t.name.clone()).or_else(|| self.returns.iter().find(|r| r.id == id).map(|r| r.name.clone()))
        };
        match target {
            ModTarget::Volume(t) => (track_name(t).unwrap_or_default(), "Volume".into()),
            ModTarget::Pan(t) => (track_name(t).unwrap_or_default(), "Pan".into()),
            ModTarget::Send { track, send } => {
                let name = self.returns.get(send as usize).map(|r| r.name.clone()).unwrap_or_else(|| format!("Send {}", send + 1));
                (track_name(track).unwrap_or_default(), format!("Send {}", name.chars().next().unwrap_or('?')))
            }
            ModTarget::Param { owner, param } => {
                let pname = self.owner_specs(owner).and_then(|s| s.get(param as usize)).map(|s| s.name).unwrap_or("?").to_string();
                let where_ = match owner {
                    ModOwner::Instrument(t) => track_name(t).unwrap_or_default(),
                    ModOwner::Device(d) => {
                        let on_track = self.tracks.iter().find(|t| t.devices.iter().any(|x| x.id == d)).map(|t| t.name.clone());
                        let on_ret = self.returns.iter().find(|r| r.devices.iter().any(|x| x.id == d)).map(|r| r.name.clone());
                        let kind = self.all_devices().find(|x| x.id == d).map(|x| x.kind.name()).unwrap_or("?");
                        format!("{} {kind}", on_track.or(on_ret).unwrap_or_else(|| "Master".into()))
                    }
                    ModOwner::MidiDevice(d) => {
                        let t = self.tracks.iter().find(|t| t.midi_devices.iter().any(|x| x.id == d));
                        let kind = t.and_then(|t| t.midi_devices.iter().find(|x| x.id == d)).map(|x| x.kind.name()).unwrap_or("?");
                        format!("{} {kind}", t.map(|t| t.name.as_str()).unwrap_or(""))
                    }
                    ModOwner::Modulator(m) => self.modulators.iter().find(|x| x.id == m).map(|x| x.name.clone()).unwrap_or_default(),
                    ModOwner::Map(t) => format!("{} on the Map", track_name(t).unwrap_or_default()),
                };
                (where_, pname)
            }
        }
    }

    pub fn target_exists(&self, target: ModTarget) -> bool {
        let has_track = |id: u64| self.tracks.iter().any(|t| t.id == id) || self.returns.iter().any(|r| r.id == id);
        match target {
            ModTarget::Volume(t) | ModTarget::Pan(t) => has_track(t),
            ModTarget::Send { track, send } => self.tracks.iter().any(|t| t.id == track) && (send as usize) < self.returns.len(),
            ModTarget::Param { owner, param } => self.owner_specs(owner).map(|s| (param as usize) < s.len()).unwrap_or(false) || self.in_clip_sounds(owner, param as usize),
        }
    }

    /// Whether `owner`'s parameter `param` is in a sound kept in a clip (a
    /// connection to it stays while another sound plays).
    fn in_clip_sounds(&self, owner: ModOwner, param: usize) -> bool {
        self.tracks.iter().any(|t| {
            let clips = t.slots.iter().flatten().chain(t.arrangement.iter().map(|a| &a.clip));
            clips.filter_map(|c| c.sound.as_deref()).any(|s| {
                let specs = match owner {
                    ModOwner::Instrument(id) if id == t.id => s.instrument.as_ref().and_then(instrument_specs),
                    ModOwner::Device(id) => s.devices.iter().find(|d| d.id == id).map(|d| d.kind.specs()),
                    ModOwner::MidiDevice(id) => s.midi_devices.iter().find(|d| d.id == id).map(|d| d.kind.specs()),
                    _ => None,
                };
                specs.is_some_and(|sp| param < sp.len())
            })
        })
    }

    /// Drops routes whose source or target no longer exists. Returns true
    /// when something was removed.
    pub fn prune_mod_routes(&mut self) -> bool {
        if self.mod_routes.is_empty() {
            return false;
        }
        let before = self.mod_routes.len();
        let routes = std::mem::take(&mut self.mod_routes);
        self.mod_routes = routes.into_iter().filter(|r| self.modulators.iter().any(|m| m.id == r.source) && self.target_exists(r.target)).collect();
        self.mod_routes.len() != before
    }
}

// ---------------------------------------------------------------- runtime

/// What the engine measured on a track during the last block.
#[derive(Clone, Copy, Default, Debug)]
pub struct TrackSignal {
    /// Note-ons since the last control tick.
    pub ons: u32,
    pub pitch: u8,
    pub velocity: f32,
    /// Notes currently held (after the MIDI effects).
    pub held: u16,
    /// Peak level of the track before its fader.
    pub level: f32,
    /// The mod wheel (0..1): a MIDI keyboard's, or a controller's triggers.
    pub modwheel: f32,
    /// A joystick (x, y; -1..1): a controller's stick in Big Picture.
    pub joy: [f32; 2],
}

/// Musical time for a control tick.
#[derive(Clone, Copy, Debug)]
pub struct ModClock {
    pub frames: usize,
    pub sr: f32,
    pub samples_per_beat: f64,
    /// Beat position: the session clock while playing, a free-running one otherwise.
    pub beat: f64,
}

#[derive(Clone, Copy, Default)]
struct ModState {
    phase: f64,
    value: f32,
    target: f32,
    last_cycle: i64,
    env: f32,
    stage: u8,
    was_held: bool,
    rng: u32,
    tides: crate::dsp::tides::Tides,
}

#[derive(Clone, Copy)]
enum Path {
    Instrument(usize),
    TrackDevice(usize, usize),
    ReturnDevice(usize, usize),
    MasterDevice(usize),
    MidiDevice(usize, usize),
    Modulator(usize),
    Map(usize),
}

struct Slot {
    owner: ModOwner,
    path: Path,
    specs: &'static [ParamSpec],
    params: Vec<f32>,
}

struct Resolved {
    source: usize,
    amount: f32,
    dest: Dest,
}

#[derive(Clone, Copy)]
enum Dest {
    Param { slot: usize, param: usize },
    Volume(MixIndex),
    Pan(MixIndex),
    Send(usize, usize),
}

#[derive(Clone, Copy)]
enum MixIndex {
    Track(usize),
    Return(usize),
}

/// Mixer offsets for one track or return.
#[derive(Clone, Copy, Default, Debug)]
pub struct MixMod {
    /// Gain factor (1 = unchanged).
    pub gain: f32,
    pub pan: f32,
    pub sends: [f32; 8],
    /// A clip envelope's volume (dB), pan and sends, in place of the
    /// mixer's (NaN: none).
    pub vol_db: f32,
    pub pan_abs: f32,
    pub sends_abs: [f32; 8],
}

impl MixMod {
    const NONE: MixMod = MixMod { gain: 1.0, pan: 0.0, sends: [0.0; 8], vol_db: f32::NAN, pan_abs: f32::NAN, sends_abs: [f32::NAN; 8] };

    /// The volume to use: the envelope's, else the mixer's.
    pub fn volume(&self, mixer_db: f32) -> f32 {
        if self.vol_db.is_nan() {
            mixer_db
        } else {
            self.vol_db
        }
    }

    pub fn pan_base(&self, mixer_pan: f32) -> f32 {
        if self.pan_abs.is_nan() {
            mixer_pan
        } else {
            self.pan_abs
        }
    }

    pub fn send_base(&self, j: usize, mixer_send: f32) -> f32 {
        match self.sends_abs.get(j) {
            Some(v) if !v.is_nan() => *v,
            _ => mixer_send,
        }
    }
}

/// The audio-thread side of the matrix. Per-modulator state lives in fixed
/// arrays so snapshot swaps don't allocate for it.
pub struct ModRuntime {
    n: usize,
    ids: [u64; MAX_MODS],
    states: [ModState; MAX_MODS],
    /// Current output of each modulator (index = position in the project).
    pub outputs: [f32; MAX_MODS],
    /// Source track index per modulator.
    sources: [Option<usize>; MAX_MODS],
    slots: Vec<Slot>,
    routes: Vec<Resolved>,
    track_mix: Vec<MixMod>,
    return_mix: Vec<MixMod>,
    free_beat: f64,
    /// Some clip has envelopes (then this runs even without modulators).
    has_env: bool,
}

impl Default for ModRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ModRuntime {
    pub fn new() -> Self {
        Self {
            n: 0,
            ids: [0; MAX_MODS],
            states: [ModState::default(); MAX_MODS],
            outputs: [0.0; MAX_MODS],
            sources: [None; MAX_MODS],
            slots: Vec::with_capacity(16),
            routes: Vec::with_capacity(64),
            track_mix: Vec::with_capacity(128),
            return_mix: Vec::with_capacity(8),
            free_beat: 0.0,
            has_env: false,
        }
    }

    /// True when the project has modulators or clip envelopes (the engine
    /// then runs at control rate).
    pub fn active(&self) -> bool {
        self.n > 0 || self.has_env
    }

    /// Rebuilds the routing for a new project snapshot, keeping modulator
    /// state (LFO phases, envelopes) by id.
    pub fn rebuild(&mut self, p: &Project) {
        let mods = &p.modulators[..p.modulators.len().min(MAX_MODS)];
        let mut states = [ModState::default(); MAX_MODS];
        let mut outputs = [0.0f32; MAX_MODS];
        let mut ids = [0u64; MAX_MODS];
        for (i, m) in mods.iter().enumerate() {
            ids[i] = m.id;
            match self.ids[..self.n].iter().position(|&id| id == m.id) {
                Some(k) => {
                    states[i] = self.states[k];
                    outputs[i] = self.outputs[k];
                }
                None => {
                    states[i] = ModState { rng: 0x9E37_79B9 ^ (m.id as u32).wrapping_mul(0x85EB_CA6B), last_cycle: i64::MIN, ..Default::default() };
                }
            }
            self.sources[i] = m.track.and_then(|t| p.track_index(t));
        }
        self.n = mods.len();
        self.ids = ids;
        self.states = states;
        self.outputs = outputs;

        self.slots.clear();
        self.routes.clear();
        self.track_mix.clear();
        self.track_mix.resize(p.tracks.len(), MixMod::NONE);
        self.return_mix.clear();
        self.return_mix.resize(p.returns.len(), MixMod::NONE);
        // Clip envelopes and the arrangement's automation need a parameter
        // set for whatever they move.
        self.has_env = false;
        for t in &p.tracks {
            let clips = t.slots.iter().flatten().chain(t.arrangement.iter().map(|a| &a.clip));
            for e in clips.flat_map(|c| c.envelopes.iter()).chain(t.automation.iter()) {
                {
                    self.has_env = true;
                    if let ModTarget::Param { owner, .. } = e.target {
                        if !self.slots.iter().any(|s| s.owner == owner) {
                            if let Some((path, specs)) = resolve(p, owner) {
                                self.slots.push(Slot { owner, path, specs, params: Vec::with_capacity(specs.len()) });
                            }
                        }
                    }
                }
            }
        }
        for r in &p.mod_routes {
            let Some(source) = self.ids[..self.n].iter().position(|&id| id == r.source) else { continue };
            let dest = match r.target {
                ModTarget::Volume(id) | ModTarget::Pan(id) => {
                    let mix = match (p.track_index(id), p.returns.iter().position(|x| x.id == id)) {
                        (Some(t), _) => MixIndex::Track(t),
                        (None, Some(j)) => MixIndex::Return(j),
                        _ => continue,
                    };
                    if matches!(r.target, ModTarget::Volume(_)) {
                        Dest::Volume(mix)
                    } else {
                        Dest::Pan(mix)
                    }
                }
                ModTarget::Send { track, send } => match p.track_index(track) {
                    Some(t) if (send as usize) < p.returns.len().min(8) => Dest::Send(t, send as usize),
                    _ => continue,
                },
                ModTarget::Param { owner, param } => {
                    let slot = match self.slots.iter().position(|s| s.owner == owner) {
                        Some(s) => s,
                        None => {
                            let Some((path, specs)) = resolve(p, owner) else { continue };
                            self.slots.push(Slot { owner, path, specs, params: Vec::with_capacity(specs.len()) });
                            self.slots.len() - 1
                        }
                    };
                    if param as usize >= self.slots[slot].specs.len() {
                        continue;
                    }
                    Dest::Param { slot, param: param as usize }
                }
            };
            self.routes.push(Resolved { source, amount: r.amount, dest });
        }
    }

    /// The modulated parameters of `owner`, if anything modulates it.
    pub fn params(&self, owner: ModOwner) -> Option<&[f32]> {
        self.slots.iter().find(|s| s.owner == owner && !matches!(s.path, Path::Modulator(_)) && !s.params.is_empty()).map(|s| &s.params[..])
    }

    pub fn track_mix(&self, ti: usize) -> MixMod {
        self.track_mix.get(ti).copied().unwrap_or(MixMod::NONE)
    }

    pub fn return_mix(&self, j: usize) -> MixMod {
        self.return_mix.get(j).copied().unwrap_or(MixMod::NONE)
    }

    /// Advances every modulator by one control block and recomputes the
    /// modulated parameters.
    /// `envs` are the clip envelopes' values now (0..1 of each range).
    pub fn tick(&mut self, p: &Project, signals: &[TrackSignal], clock: ModClock, playing: bool, envs: &[(ModTarget, f32)]) {
        if self.n == 0 && !self.has_env {
            return;
        }
        self.free_beat += clock.frames as f64 / clock.samples_per_beat;
        let beat = if playing { clock.beat } else { self.free_beat };
        for i in 0..self.n {
            let Some(m) = p.modulators.get(i) else { break };
            let specs = m.kind.specs();
            // The modulator's own parameters, modulated by the others.
            let n = specs.len().min(16);
            let mut params = [0.0f32; 16];
            let mut offsets = [0.0f32; 16];
            for k in 0..n {
                params[k] = m.params.get(k).copied().unwrap_or(specs[k].default);
            }
            for r in &self.routes {
                if let Dest::Param { slot, param } = r.dest {
                    if matches!(self.slots[slot].path, Path::Modulator(mi) if mi == i) && param < n {
                        offsets[param] += r.amount * self.outputs[r.source];
                    }
                }
            }
            for k in 0..n {
                params[k] = modulate(&specs[k], params[k], offsets[k]);
            }
            let sig = self.sources[i].and_then(|t| signals.get(t)).copied().unwrap_or_default();
            let out = step(m, &params[..n], &mut self.states[i], sig, clock, beat);
            self.outputs[i] = if out.is_finite() { out.clamp(-1.0, 1.0) } else { 0.0 };
        }

        // Parameter sets start from the project values...
        for slot in self.slots.iter_mut() {
            let place;
            let base: &[f32] = match slot.path {
                Path::Map(t) => {
                    place = p.tracks.get(t).and_then(|t| t.place).map(|pl| pl.to_params()).unwrap_or_default();
                    &place
                }
                Path::Instrument(t) => p.tracks.get(t).and_then(|t| t.instrument.as_ref()).and_then(instrument_params).unwrap_or(&[]),
                Path::TrackDevice(t, d) => p.tracks.get(t).and_then(|t| t.devices.get(d)).map(|d| &d.params[..]).unwrap_or(&[]),
                Path::ReturnDevice(r, d) => p.returns.get(r).and_then(|r| r.devices.get(d)).map(|d| &d.params[..]).unwrap_or(&[]),
                Path::MasterDevice(d) => p.master.devices.get(d).map(|d| &d.params[..]).unwrap_or(&[]),
                Path::MidiDevice(t, d) => p.tracks.get(t).and_then(|t| t.midi_devices.get(d)).map(|d| &d.params[..]).unwrap_or(&[]),
                Path::Modulator(_) => continue,
            };
            slot.params.clear();
            slot.params.extend_from_slice(base);
        }
        for m in self.track_mix.iter_mut().chain(self.return_mix.iter_mut()) {
            *m = MixMod { gain: 0.0, ..MixMod::NONE };
        }
        // Clip envelopes set the values the modulators then move.
        for &(target, v) in envs {
            match target {
                ModTarget::Param { owner, param } => {
                    if let Some(s) = self.slots.iter_mut().find(|s| s.owner == owner && !matches!(s.path, Path::Modulator(_))) {
                        if let (Some(spec), Some(x)) = (s.specs.get(param as usize), s.params.get_mut(param as usize)) {
                            *x = spec.from_norm(v);
                        }
                    }
                }
                ModTarget::Volume(id) | ModTarget::Pan(id) => {
                    let m = match (p.track_index(id), p.returns.iter().position(|x| x.id == id)) {
                        (Some(t), _) => self.track_mix.get_mut(t),
                        (None, Some(j)) => self.return_mix.get_mut(j),
                        _ => None,
                    };
                    if let Some(m) = m {
                        if matches!(target, ModTarget::Volume(_)) {
                            m.vol_db = crate::project::fader_pos_to_db(v);
                        } else {
                            m.pan_abs = v * 2.0 - 1.0;
                        }
                    }
                }
                ModTarget::Send { track, send } => {
                    if let Some(m) = p.track_index(track).and_then(|t| self.track_mix.get_mut(t)) {
                        if let Some(s) = m.sends_abs.get_mut(send as usize) {
                            *s = v;
                        }
                    }
                }
            }
        }
        // ...then the routed offsets are summed per parameter and applied once.
        let mut param_offsets = [(usize::MAX, 0usize, 0.0f32); 64];
        let mut n_off = 0;
        for r in &self.routes {
            let v = r.amount * self.outputs[r.source];
            match r.dest {
                Dest::Param { slot, param } => {
                    if matches!(self.slots[slot].path, Path::Modulator(_)) {
                        continue;
                    }
                    match param_offsets[..n_off].iter().position(|o| o.0 == slot && o.1 == param) {
                        Some(k) => param_offsets[k].2 += v,
                        None if n_off < param_offsets.len() => {
                            param_offsets[n_off] = (slot, param, v);
                            n_off += 1;
                        }
                        None => {}
                    }
                }
                Dest::Volume(ix) => mix_of(&mut self.track_mix, &mut self.return_mix, ix).gain += v,
                Dest::Pan(ix) => mix_of(&mut self.track_mix, &mut self.return_mix, ix).pan += v,
                Dest::Send(t, s) => {
                    if let Some(m) = self.track_mix.get_mut(t) {
                        m.sends[s] += v;
                    }
                }
            }
        }
        for &(slot, param, off) in &param_offsets[..n_off] {
            let s = &mut self.slots[slot];
            if let (Some(spec), Some(v)) = (s.specs.get(param), s.params.get_mut(param)) {
                *v = modulate(spec, *v, off);
            }
        }
        for m in self.track_mix.iter_mut().chain(self.return_mix.iter_mut()) {
            m.gain = (1.0 + m.gain).clamp(0.0, 2.0);
        }
    }
}

fn mix_of<'a>(tracks: &'a mut [MixMod], returns: &'a mut [MixMod], ix: MixIndex) -> &'a mut MixMod {
    match ix {
        MixIndex::Track(t) => &mut tracks[t],
        MixIndex::Return(j) => &mut returns[j],
    }
}

fn resolve(p: &Project, owner: ModOwner) -> Option<(Path, &'static [ParamSpec])> {
    match owner {
        ModOwner::Instrument(id) => {
            let t = p.track_index(id)?;
            let specs = p.tracks[t].instrument.as_ref().and_then(instrument_specs)?;
            Some((Path::Instrument(t), specs))
        }
        ModOwner::Device(id) => {
            for (t, tr) in p.tracks.iter().enumerate() {
                if let Some(d) = tr.devices.iter().position(|x| x.id == id) {
                    return Some((Path::TrackDevice(t, d), tr.devices[d].kind.specs()));
                }
            }
            for (r, ret) in p.returns.iter().enumerate() {
                if let Some(d) = ret.devices.iter().position(|x| x.id == id) {
                    return Some((Path::ReturnDevice(r, d), ret.devices[d].kind.specs()));
                }
            }
            let d = p.master.devices.iter().position(|x| x.id == id)?;
            Some((Path::MasterDevice(d), p.master.devices[d].kind.specs()))
        }
        ModOwner::MidiDevice(id) => {
            for (t, tr) in p.tracks.iter().enumerate() {
                if let Some(d) = tr.midi_devices.iter().position(|x| x.id == id) {
                    return Some((Path::MidiDevice(t, d), tr.midi_devices[d].kind.specs()));
                }
            }
            None
        }
        ModOwner::Modulator(id) => {
            let i = p.modulators.iter().take(MAX_MODS).position(|m| m.id == id)?;
            Some((Path::Modulator(i), p.modulators[i].kind.specs()))
        }
        ModOwner::Map(id) => {
            let t = p.track_index(id)?;
            p.tracks[t].place?;
            Some((Path::Map(t), crate::dsp::fx::stage::PLACE_SPECS))
        }
    }
}

fn rand_unit(st: &mut ModState) -> f32 {
    let mut r = Rng::new(st.rng);
    let v = r.next_u32();
    st.rng = v | 1;
    (v >> 8) as f32 / (1u32 << 24) as f32
}

/// One-pole smoothing toward `target` with time constant `secs`.
fn slew(cur: f32, target: f32, secs: f32, dt: f32) -> f32 {
    if secs <= 0.0011 {
        return target;
    }
    cur + (target - cur) * (1.0 - (-dt / secs).exp())
}

fn step(m: &Modulator, p: &[f32], st: &mut ModState, sig: TrackSignal, clock: ModClock, beat: f64) -> f32 {
    let dt = clock.frames as f32 / clock.sr;
    let get = |i: usize| p.get(i).copied().unwrap_or(0.0);
    match m.kind {
        ModKind::Lfo => {
            let shape = get(0).round() as usize;
            let synced = get(1) >= 0.5;
            let phase = if synced {
                let period = BEAT_RATE_BEATS[(get(2).round().max(0.0) as usize).min(BEAT_RATE_BEATS.len() - 1)];
                let t = beat / period;
                let cycle = t.floor() as i64;
                if cycle != st.last_cycle {
                    st.last_cycle = cycle;
                    st.target = rand_unit(st) * 2.0 - 1.0;
                }
                t.fract()
            } else {
                st.phase += get(3) as f64 * dt as f64;
                if st.phase >= 1.0 {
                    st.phase -= st.phase.floor();
                    st.target = rand_unit(st) * 2.0 - 1.0;
                }
                st.phase
            };
            let ph = ((phase + get(4) as f64).fract()) as f32;
            let tau = std::f32::consts::TAU;
            let v = match shape {
                0 => (ph * tau).sin(),
                1 => 1.0 - 4.0 * (ph - 0.5).abs(),
                2 => 2.0 * ph - 1.0,
                3 => 1.0 - 2.0 * ph,
                4 => {
                    if ph < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                5 => st.target,
                _ => {
                    // Smooth random: glide to the new value over a cycle.
                    st.value += (st.target - st.value) * (ph * 0.25).min(1.0);
                    st.value
                }
            };
            v * get(5)
        }
        ModKind::Envelope => {
            let (a, d, s, r) = (get(0).max(1e-4), get(1).max(1e-4), get(2), get(3).max(1e-4));
            let legato = get(4) >= 0.5;
            let held = sig.held > 0;
            let trigger = if legato { held && !st.was_held } else { sig.ons > 0 };
            st.was_held = held;
            if trigger {
                st.stage = 1;
            } else if !held && st.stage != 0 && st.stage != 4 {
                st.stage = 4;
            }
            match st.stage {
                1 => {
                    st.env += dt / a;
                    if st.env >= 1.0 {
                        st.env = 1.0;
                        st.stage = 2;
                    }
                }
                2 => {
                    st.env = s + (st.env - s) * (-dt * 4.6 / d).exp();
                    if (st.env - s).abs() < 1e-3 {
                        st.stage = 3;
                    }
                }
                3 => st.env = s,
                4 => {
                    st.env *= (-dt * 4.6 / r).exp();
                    if st.env < 1e-4 {
                        st.env = 0.0;
                        st.stage = 0;
                    }
                }
                _ => st.env = 0.0,
            }
            st.env * get(5)
        }
        ModKind::Follower => {
            let target = (sig.level * 10f32.powf(get(0) / 20.0)).min(1.0);
            let t = if target > st.value { get(1) } else { get(2) };
            st.value = slew(st.value, target, t, dt);
            st.value * get(3)
        }
        ModKind::Steps => {
            let rate = BEAT_RATE_BEATS[(get(0).round().max(0.0) as usize).min(BEAT_RATE_BEATS.len() - 1)];
            let len = get(1).round().clamp(1.0, 16.0) as i64;
            let i = ((beat / rate).floor() as i64).rem_euclid(len) as usize;
            let raw = m.data.get(i).copied().unwrap_or(0) as f32 / 255.0;
            let target = if get(3) >= 0.5 { raw * 2.0 - 1.0 } else { raw };
            let step_secs = (rate * clock.samples_per_beat) as f32 / clock.sr;
            st.value = slew(st.value, target, get(2) * step_secs * 0.5, dt);
            st.value * get(4)
        }
        ModKind::Macro => get(0),
        ModKind::Notes => {
            if sig.ons > 0 {
                st.target = match get(0).round() as usize {
                    0 => sig.velocity,
                    1 => ((sig.pitch as f32 - 60.0) / 48.0).clamp(-1.0, 1.0),
                    3 => rand_unit(st) * 2.0 - 1.0,
                    _ => st.target,
                };
            }
            if get(0).round() as usize == 2 {
                st.target = if sig.held > 0 { 1.0 } else { 0.0 };
            }
            st.value = slew(st.value, st.target, get(1), dt);
            st.value
        }
        ModKind::Math => {
            let (a, b) = (get(0), get(1));
            let v = match get(2).round() as usize {
                0 => a + b,
                1 => a * b,
                2 => a.min(b),
                3 => a.max(b),
                4 => a - b,
                5 => (a + b) * 0.5,
                _ => {
                    if a > b {
                        1.0
                    } else {
                        0.0
                    }
                }
            };
            v * get(3)
        }
        ModKind::Shaper => {
            let mut x = get(0) * get(1) + get(2);
            x = x.clamp(-1.0, 1.0);
            x = match get(6).round() as usize {
                1 => -x,
                2 => x.abs(),
                3 => x * 0.5 + 0.5,
                _ => x,
            };
            let curve = get(3);
            if curve.abs() > 1e-3 {
                x = x.signum() * x.abs().powf(2f32.powf(curve * 2.0));
            }
            let steps = get(4).round();
            if steps >= 2.0 {
                x = (x * steps).round() / steps;
            }
            st.value = slew(st.value, x, get(5), dt);
            st.value
        }
        ModKind::Tides => {
            use crate::dsp::tides as td;
            let s = td::Settings {
                ramp_mode: get(0).round().max(0.0) as usize,
                output_mode: get(1).round().max(0.0) as usize,
                shape: get(5),
                slope: get(6),
                smoothness: get(7),
                shift: get(8),
            };
            let (f0, ramp) = if get(2) >= 0.5 {
                // Synced: a cycle (or an envelope) lasts the beat rate; a loop keeps the song's phase.
                let period = BEAT_RATE_BEATS[(get(3).round().max(0.0) as usize).min(BEAT_RATE_BEATS.len() - 1)];
                let f0 = (1.0 / (period * clock.samples_per_beat)) as f32;
                (f0, (s.ramp_mode == td::LOOP).then(|| (beat / period).rem_euclid(1.0) as f32))
            } else {
                (get(4) / clock.sr, None)
            };
            let out = st.tides.process(&s, f0, clock.frames, ramp, sig.ons > 0, sig.held > 0);
            let ch = (get(9).round().clamp(1.0, 4.0) as usize) - 1;
            out[ch] * get(10)
        }
        ModKind::ModWheel => {
            st.value = slew(st.value, sig.modwheel, get(0), dt);
            st.value * get(1)
        }
        ModKind::Joystick => {
            let axis = (get(0).round() as usize).min(1);
            st.value = slew(st.value, sig.joy[axis], get(1), dt);
            st.value * get(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::effects::EffectKind;

    fn clock(beat: f64) -> ModClock {
        ModClock { frames: 64, sr: 48000.0, samples_per_beat: 24000.0, beat }
    }

    #[test]
    fn modulate_sweeps_the_range() {
        let cutoff = ParamSpec::log("Cutoff", "", 20.0, 20000.0, 1000.0, Unit::Hz);
        assert!((modulate(&cutoff, 1000.0, 0.0) - 1000.0).abs() < 1e-3);
        assert!(modulate(&cutoff, 1000.0, 1.0) >= 19999.0);
        assert!(modulate(&cutoff, 1000.0, -1.0) <= 20.01);
        let pan = ParamSpec::lin("Pan", "", -1.0, 1.0, 0.0, Unit::Pan);
        assert!((modulate(&pan, 0.0, 1.0) - 1.0).abs() < 1e-6, "100% on a bipolar knob reaches the end from the center");
        assert!((modulate(&pan, 0.0, -0.5) + 0.5).abs() < 1e-6);
        let choice = ParamSpec::choice("Shape", "", &["a", "b", "c"], 0);
        assert_eq!(modulate(&choice, 0.0, 0.6), 1.0);
    }

    #[test]
    fn lfo_reaches_an_effect_parameter() {
        let mut p = Project::new_default();
        let dev = p.make_device(EffectKind::AutoFilter);
        let dev_id = dev.id;
        p.tracks[1].devices.push(dev);
        let lfo = p.make_modulator(ModKind::Lfo);
        let lfo_id = lfo.id;
        p.modulators.push(lfo);
        p.add_route(lfo_id, ModTarget::Param { owner: ModOwner::Device(dev_id), param: 1 }, 0.5);
        let mut rt = ModRuntime::new();
        rt.rebuild(&p);
        let base = p.tracks[1].devices[0].params[1];
        let mut seen = Vec::new();
        for k in 0..400 {
            rt.tick(&p, &[], clock(k as f64 * 64.0 / 24000.0), true, &[]);
            seen.push(rt.params(ModOwner::Device(dev_id)).unwrap()[1]);
        }
        let (lo, hi) = seen.iter().fold((f32::MAX, f32::MIN), |(a, b), &v| (a.min(v), b.max(v)));
        assert!(lo < base && hi > base, "{lo} {base} {hi}");
        // Untouched sets are not overridden.
        assert!(rt.params(ModOwner::Instrument(p.tracks[1].id)).is_none());
    }

    #[test]
    fn envelope_follows_notes_and_modulators_chain() {
        let mut p = Project::new_default();
        let mut env = p.make_modulator(ModKind::Envelope);
        env.track = Some(p.tracks[0].id);
        env.params[2] = 0.5; // sustain
        let env_id = env.id;
        p.modulators.push(env);
        // A Shaper inverts the envelope; route the envelope into its input.
        let mut sh = p.make_modulator(ModKind::Shaper);
        sh.params[6] = 1.0; // invert
        let sh_id = sh.id;
        p.modulators.push(sh);
        p.add_route(env_id, ModTarget::Param { owner: ModOwner::Modulator(sh_id), param: 0 }, 1.0);
        let mut rt = ModRuntime::new();
        rt.rebuild(&p);
        let mut sig = [TrackSignal { ons: 1, pitch: 60, velocity: 1.0, held: 1, ..Default::default() }];
        rt.tick(&p, &sig, clock(0.0), true, &[]);
        sig[0].ons = 0;
        for _ in 0..200 {
            rt.tick(&p, &sig, clock(0.0), true, &[]);
        }
        assert!((rt.outputs[0] - 0.5).abs() < 0.02, "sustain {}", rt.outputs[0]);
        // Input = 0.5 (half the bipolar range from center) -> inverted.
        assert!(rt.outputs[1] < -0.2, "shaper {}", rt.outputs[1]);
        sig[0].held = 0;
        for _ in 0..800 {
            rt.tick(&p, &sig, clock(0.0), true, &[]);
        }
        assert!(rt.outputs[0] < 0.01, "released {}", rt.outputs[0]);
    }

    #[test]
    fn follower_ducks_volume() {
        let mut p = Project::new_default();
        let mut f = p.make_modulator(ModKind::Follower);
        f.track = Some(p.tracks[0].id);
        let fid = f.id;
        p.modulators.push(f);
        let bass = p.tracks[1].id;
        p.add_route(fid, ModTarget::Volume(bass), -1.0);
        let mut rt = ModRuntime::new();
        rt.rebuild(&p);
        let loud = [TrackSignal { level: 0.9, ..Default::default() }];
        for _ in 0..100 {
            rt.tick(&p, &loud, clock(0.0), true, &[]);
        }
        assert!(rt.track_mix(1).gain < 0.05, "ducked gain {}", rt.track_mix(1).gain);
        assert!((rt.track_mix(0).gain - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_mod_wheel_follows_the_wheel() {
        let mut p = Project::new_default();
        let mut m = p.make_modulator(ModKind::ModWheel);
        m.track = Some(p.tracks[0].id);
        p.modulators.push(m);
        let mut rt = ModRuntime::new();
        rt.rebuild(&p);
        let up = [TrackSignal { modwheel: 0.8, ..Default::default() }];
        for _ in 0..200 {
            rt.tick(&p, &up, clock(0.0), true, &[]);
        }
        assert!((rt.outputs[0] - 0.8).abs() < 0.01, "up {}", rt.outputs[0]);
        let down = [TrackSignal::default()];
        for _ in 0..200 {
            rt.tick(&p, &down, clock(0.0), true, &[]);
        }
        assert!(rt.outputs[0] < 0.01, "down {}", rt.outputs[0]);
    }

    #[test]
    fn a_joystick_follows_its_axis() {
        let mut p = Project::new_default();
        let mut x = p.make_modulator(ModKind::Joystick);
        x.track = Some(p.tracks[0].id);
        p.modulators.push(x);
        let mut y = p.make_modulator(ModKind::Joystick);
        y.track = Some(p.tracks[0].id);
        y.params[0] = 1.0;
        p.modulators.push(y);
        let mut rt = ModRuntime::new();
        rt.rebuild(&p);
        let sig = [TrackSignal { joy: [-0.6, 0.9], ..Default::default() }];
        for _ in 0..200 {
            rt.tick(&p, &sig, clock(0.0), true, &[]);
        }
        assert!((rt.outputs[0] + 0.6).abs() < 0.01, "x {}", rt.outputs[0]);
        assert!((rt.outputs[1] - 0.9).abs() < 0.01, "y {}", rt.outputs[1]);
    }

    #[test]
    fn routes_to_deleted_things_are_pruned() {
        let mut p = Project::new_default();
        let m = p.make_modulator(ModKind::Macro);
        let mid = m.id;
        p.modulators.push(m);
        let t = p.tracks[1].id;
        p.add_route(mid, ModTarget::Param { owner: ModOwner::Instrument(t), param: 3 }, 0.3);
        p.add_route(mid, ModTarget::Pan(t), 0.3);
        assert!(!p.prune_mod_routes());
        p.tracks.remove(1);
        assert!(p.prune_mod_routes());
        assert!(p.mod_routes.is_empty());
    }
}
