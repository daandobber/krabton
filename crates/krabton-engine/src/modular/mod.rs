//! Modular: a eurorack-style modular synthesizer. Modules sit in a rack
//! (rows of 104 HP), patched with cables from outputs to inputs. Signals are
//! voltages as in eurorack: 1 V per octave (0 V is C4), gates and triggers
//! of 10 V, audio of about ±5 V, envelopes 0 to 10 V, LFOs ±5 V.
//!
//! The patch is played with notes through the MIDI module; with more than
//! one voice, each voice runs its own copy of the patch.

pub mod dsp;
pub mod presets;
pub mod runtime;

use crate::params::{ParamSpec, Unit};
use serde::{Deserialize, Serialize};

/// Width of a rack row in HP.
pub const ROW_HP: u16 = 104;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleKind {
    Midi,
    Out,
    Macros,
    Clock,
    Vco,
    Lfo,
    Noise,
    Vcf,
    Ladder,
    Vca,
    Adsr,
    Ad,
    Mixer,
    Atten,
    SampleHold,
    Slew,
    Quantizer,
    Sequencer,
    Folder,
    RingMod,
    Delay,
    Reverb,
    Divider,
    Chance,
    MacroVco,
    ShapeVco,
    Resonator,
    Texture,
    BassDrum,
    SnareDrum,
    HiHat,
    Function,
    RandomGen,
    QuadVca,
    Logic,
    Switch,
    MetaMod,
    Follower,
    Octave,
    Burst,
    Euclid,
    ShiftReg,
    StereoMixer,
    Pulse,
    Pluck,
    Formant,
    TrigSeq,
    Crusher,
}

/// What a module is: its name, width, knobs and jacks.
pub struct ModuleInfo {
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub hp: u16,
    pub params: &'static [ParamSpec],
    pub inputs: &'static [&'static str],
    pub outputs: &'static [&'static str],
}

pub const CATEGORIES: [&str; 7] = ["Sources", "Drums", "Filters & Amps", "Envelopes & LFOs", "Sequencing", "Utilities", "Effects & I/O"];

impl ModuleKind {
    pub const ALL: [ModuleKind; 48] = [
        ModuleKind::Vco,
        ModuleKind::Noise,
        ModuleKind::Vcf,
        ModuleKind::Ladder,
        ModuleKind::Vca,
        ModuleKind::Folder,
        ModuleKind::RingMod,
        ModuleKind::Adsr,
        ModuleKind::Ad,
        ModuleKind::Lfo,
        ModuleKind::Clock,
        ModuleKind::Sequencer,
        ModuleKind::Quantizer,
        ModuleKind::Divider,
        ModuleKind::Chance,
        ModuleKind::SampleHold,
        ModuleKind::Mixer,
        ModuleKind::Atten,
        ModuleKind::Slew,
        ModuleKind::Delay,
        ModuleKind::Reverb,
        ModuleKind::Midi,
        ModuleKind::Macros,
        ModuleKind::Out,
        ModuleKind::MacroVco,
        ModuleKind::ShapeVco,
        ModuleKind::Resonator,
        ModuleKind::Texture,
        ModuleKind::BassDrum,
        ModuleKind::SnareDrum,
        ModuleKind::HiHat,
        ModuleKind::Function,
        ModuleKind::RandomGen,
        ModuleKind::QuadVca,
        ModuleKind::Logic,
        ModuleKind::Switch,
        ModuleKind::MetaMod,
        ModuleKind::Follower,
        ModuleKind::Octave,
        ModuleKind::Burst,
        ModuleKind::Euclid,
        ModuleKind::ShiftReg,
        ModuleKind::StereoMixer,
        ModuleKind::Pulse,
        ModuleKind::Pluck,
        ModuleKind::Formant,
        ModuleKind::TrigSeq,
        ModuleKind::Crusher,
    ];

    pub fn info(self) -> &'static ModuleInfo {
        use ModuleKind::*;
        match self {
            Midi => &MIDI,
            Out => &OUT,
            Macros => &MACROS,
            Clock => &CLOCK,
            Vco => &VCO,
            Lfo => &LFO,
            Noise => &NOISE,
            Vcf => &VCF,
            Ladder => &LADDER,
            Vca => &VCA,
            Adsr => &ADSR,
            Ad => &AD,
            Mixer => &MIXER,
            Atten => &ATTEN,
            SampleHold => &SAMPLE_HOLD,
            Slew => &SLEW,
            Quantizer => &QUANTIZER,
            Sequencer => &SEQUENCER,
            Folder => &FOLDER,
            RingMod => &RING_MOD,
            Delay => &DELAY,
            Reverb => &REVERB,
            Divider => &DIVIDER,
            Chance => &CHANCE,
            MacroVco => &MACRO_VCO,
            ShapeVco => &SHAPE_VCO,
            Resonator => &RESONATOR,
            Texture => &TEXTURE,
            BassDrum => &BASS_DRUM,
            SnareDrum => &SNARE_DRUM,
            HiHat => &HI_HAT,
            Function => &FUNCTION,
            RandomGen => &RANDOM_GEN,
            QuadVca => &QUAD_VCA,
            Logic => &LOGIC,
            Switch => &SWITCH,
            MetaMod => &META_MOD,
            Follower => &FOLLOWER,
            Octave => &OCTAVE,
            Burst => &BURST,
            Euclid => &EUCLID,
            ShiftReg => &SHIFT_REG,
            StereoMixer => &STEREO_MIXER,
            Pulse => &PULSE,
            Pluck => &PLUCK,
            Formant => &FORMANT,
            TrigSeq => &TRIG_SEQ,
            Crusher => &CRUSHER,
        }
    }
}

const fn lin(name: &'static str, min: f32, max: f32, default: f32, unit: Unit) -> ParamSpec {
    ParamSpec::lin(name, "", min, max, default, unit)
}
const fn log(name: &'static str, min: f32, max: f32, default: f32, unit: Unit) -> ParamSpec {
    ParamSpec::log(name, "", min, max, default, unit)
}
const fn int(name: &'static str, min: f32, max: f32, default: f32, unit: Unit) -> ParamSpec {
    ParamSpec::int(name, "", min, max, default, unit)
}

pub const SCALES: &[&str] = &["Chromatic", "Major", "Minor", "Dorian", "Pent. Major", "Pent. Minor", "Blues", "Whole Tone"];
pub const CLOCK_RATES: &[&str] = &["1/4", "1/8", "1/8 T", "1/16", "1/16 T", "1/32"];

static MIDI: ModuleInfo = ModuleInfo {
    name: "MIDI-CV",
    category: "Effects & I/O",
    description: "Your notes as voltages: pitch (1 V/oct), gate, velocity and a trigger on every note",
    hp: 6,
    params: &[log("Glide", 0.001, 2.0, 0.001, Unit::Sec), int("Octave", -3.0, 3.0, 0.0, Unit::None)],
    inputs: &[],
    outputs: &["V/Oct", "Gate", "Vel", "Trig"],
};
static OUT: ModuleInfo = ModuleInfo {
    name: "Audio Out",
    category: "Effects & I/O",
    description: "To the track: left and right (right follows left when unpatched), ±5 V is full scale",
    hp: 6,
    params: &[lin("Level", 0.0, 1.0, 0.8, Unit::Percent)],
    inputs: &["L", "R"],
    outputs: &[],
};
static MACROS: ModuleInfo = ModuleInfo {
    name: "Macros",
    category: "Effects & I/O",
    description: "The instrument's eight macro knobs as 0..10 V: modulate and automate them from Krabton",
    hp: 12,
    params: &[],
    inputs: &[],
    outputs: &["M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8"],
};
static CLOCK: ModuleInfo = ModuleInfo {
    name: "Clock",
    category: "Sequencing",
    description: "Gates in time with the song (free-running at the tempo when stopped), with swing, a bar reset and divisions",
    hp: 6,
    params: &[ParamSpec::choice("Rate", "", CLOCK_RATES, 3), lin("Gate", 0.05, 0.95, 0.5, Unit::Percent), lin("Swing", 0.0, 0.75, 0.0, Unit::Percent)],
    inputs: &[],
    outputs: &["Clock", "/2", "/4", "Bar"],
};
static VCO: ModuleInfo = ModuleInfo {
    name: "VCO",
    category: "Sources",
    description: "Analog-style oscillator: sine, triangle, saw and square at once, with pulse width, linear FM and hard sync",
    hp: 12,
    params: &[
        int("Octave", -4.0, 4.0, 0.0, Unit::None),
        int("Tune", -12.0, 12.0, 0.0, Unit::Semi),
        lin("Fine", -50.0, 50.0, 0.0, Unit::Cents),
        lin("PW", 0.05, 0.95, 0.5, Unit::Percent),
        lin("FM", 0.0, 1.0, 0.0, Unit::Percent),
        lin("PWM", 0.0, 1.0, 0.0, Unit::Percent),
    ],
    inputs: &["V/Oct", "FM", "PWM", "Sync"],
    outputs: &["Sine", "Tri", "Saw", "Square"],
};
static LFO: ModuleInfo = ModuleInfo {
    name: "LFO",
    category: "Envelopes & LFOs",
    description: "Low-frequency oscillator, ±5 V: sine, triangle, saw, square and a random step every cycle",
    hp: 8,
    params: &[log("Rate", 0.01, 50.0, 1.0, Unit::Hz), lin("Offset", -5.0, 5.0, 0.0, Unit::None)],
    inputs: &["Rate", "Reset"],
    outputs: &["Sine", "Tri", "Saw", "Square", "Random"],
};
static NOISE: ModuleInfo = ModuleInfo {
    name: "Noise",
    category: "Sources",
    description: "White and pink noise, and a slowly drifting random voltage",
    hp: 4,
    params: &[log("Drift", 0.05, 20.0, 1.0, Unit::Hz)],
    inputs: &[],
    outputs: &["White", "Pink", "Drift"],
};
static VCF: ModuleInfo = ModuleInfo {
    name: "VCF",
    category: "Filters & Amps",
    description: "Multimode state-variable filter: low-, band- and highpass at once, cutoff at 1 V/oct",
    hp: 8,
    params: &[log("Cutoff", 20.0, 20000.0, 1200.0, Unit::Hz), lin("Res", 0.0, 1.0, 0.3, Unit::Percent), lin("CV Amt", -1.0, 1.0, 1.0, Unit::Percent), lin("Drive", 0.0, 1.0, 0.0, Unit::Percent)],
    inputs: &["In", "Cutoff", "Res"],
    outputs: &["LP", "BP", "HP"],
};
static LADDER: ModuleInfo = ModuleInfo {
    name: "Ladder",
    category: "Filters & Amps",
    description: "24 dB transistor ladder lowpass that screams when pushed, cutoff at 1 V/oct",
    hp: 8,
    params: &[log("Cutoff", 20.0, 20000.0, 800.0, Unit::Hz), lin("Res", 0.0, 1.0, 0.4, Unit::Percent), lin("CV Amt", -1.0, 1.0, 1.0, Unit::Percent), lin("Drive", 0.0, 1.0, 0.2, Unit::Percent)],
    inputs: &["In", "Cutoff", "Res"],
    outputs: &["LP"],
};
static VCA: ModuleInfo = ModuleInfo {
    name: "VCA",
    category: "Filters & Amps",
    description: "Amplifier: the CV (0..10 V) opens it, on top of the Level knob; linear or exponential",
    hp: 4,
    params: &[lin("Level", 0.0, 1.0, 0.0, Unit::Percent), ParamSpec::choice("Resp", "", &["Lin", "Exp"], 1)],
    inputs: &["In", "CV"],
    outputs: &["Out"],
};
static ADSR: ModuleInfo = ModuleInfo {
    name: "ADSR",
    category: "Envelopes & LFOs",
    description: "Attack, decay, sustain, release envelope, 0..10 V, with an inverted output and end-of-cycle",
    hp: 8,
    params: &[log("Attack", 0.001, 10.0, 0.005, Unit::Sec), log("Decay", 0.005, 10.0, 0.3, Unit::Sec), lin("Sustain", 0.0, 1.0, 0.6, Unit::Percent), log("Release", 0.005, 10.0, 0.4, Unit::Sec)],
    inputs: &["Gate", "Retrig"],
    outputs: &["Env", "Inv", "EOC"],
};
static AD: ModuleInfo = ModuleInfo {
    name: "AD",
    category: "Envelopes & LFOs",
    description: "Attack-decay envelope on a trigger, with a curve from punchy to linear",
    hp: 4,
    params: &[log("Attack", 0.0005, 5.0, 0.001, Unit::Sec), log("Decay", 0.005, 10.0, 0.25, Unit::Sec), lin("Curve", 0.0, 1.0, 0.7, Unit::Percent)],
    inputs: &["Trig"],
    outputs: &["Env"],
};
static MIXER: ModuleInfo = ModuleInfo {
    name: "Mixer",
    category: "Utilities",
    description: "Four-channel mixer for audio or CV",
    hp: 8,
    params: &[lin("1", 0.0, 1.0, 0.8, Unit::Percent), lin("2", 0.0, 1.0, 0.8, Unit::Percent), lin("3", 0.0, 1.0, 0.8, Unit::Percent), lin("4", 0.0, 1.0, 0.8, Unit::Percent), lin("Master", 0.0, 2.0, 1.0, Unit::Percent)],
    inputs: &["In 1", "In 2", "In 3", "In 4"],
    outputs: &["Out"],
};
static ATTEN: ModuleInfo = ModuleInfo {
    name: "Atten",
    category: "Utilities",
    description: "Two attenuverters with offset: scale, flip and shift voltages (In 2 follows In 1 when unpatched)",
    hp: 6,
    params: &[lin("Amt 1", -1.0, 1.0, 1.0, Unit::Percent), lin("Off 1", -10.0, 10.0, 0.0, Unit::None), lin("Amt 2", -1.0, 1.0, 1.0, Unit::Percent), lin("Off 2", -10.0, 10.0, 0.0, Unit::None)],
    inputs: &["In 1", "In 2"],
    outputs: &["Out 1", "Out 2"],
};
static SAMPLE_HOLD: ModuleInfo = ModuleInfo {
    name: "S&H",
    category: "Utilities",
    description: "Sample and hold: on each trigger the output takes the input (noise when unpatched)",
    hp: 4,
    params: &[log("Slew", 0.0001, 1.0, 0.0001, Unit::Sec)],
    inputs: &["In", "Trig"],
    outputs: &["Out"],
};
static SLEW: ModuleInfo = ModuleInfo {
    name: "Slew",
    category: "Utilities",
    description: "Slew limiter: glides between voltages, rising and falling at their own rates (portamento)",
    hp: 4,
    params: &[log("Rise", 0.0001, 5.0, 0.05, Unit::Sec), log("Fall", 0.0001, 5.0, 0.05, Unit::Sec)],
    inputs: &["In"],
    outputs: &["Out"],
};
static QUANTIZER: ModuleInfo = ModuleInfo {
    name: "Quantizer",
    category: "Sequencing",
    description: "Snaps a voltage to the notes of a scale, with root and transpose, and a trigger when the note changes",
    hp: 6,
    params: &[ParamSpec::choice("Scale", "", SCALES, 1), int("Root", 0.0, 11.0, 0.0, Unit::Semi), int("Transp", -24.0, 24.0, 0.0, Unit::Semi)],
    inputs: &["In"],
    outputs: &["Out", "Trig"],
};
static SEQUENCER: ModuleInfo = ModuleInfo {
    name: "Sequencer",
    category: "Sequencing",
    description: "Eight-step voltage sequencer: a step per clock, a gate and trigger on every step, back to 1 on reset",
    hp: 16,
    params: &[
        lin("1", -3.0, 3.0, 0.0, Unit::None),
        lin("2", -3.0, 3.0, 0.25, Unit::None),
        lin("3", -3.0, 3.0, 0.5833, Unit::None),
        lin("4", -3.0, 3.0, 0.0, Unit::None),
        lin("5", -3.0, 3.0, 1.0, Unit::None),
        lin("6", -3.0, 3.0, 0.5833, Unit::None),
        lin("7", -3.0, 3.0, 0.25, Unit::None),
        lin("8", -3.0, 3.0, -0.4167, Unit::None),
        int("Steps", 1.0, 8.0, 8.0, Unit::None),
        lin("Gate", 0.05, 1.0, 0.5, Unit::Percent),
    ],
    inputs: &["Clock", "Reset"],
    outputs: &["CV", "Gate", "Trig"],
};
static FOLDER: ModuleInfo = ModuleInfo {
    name: "Folder",
    category: "Filters & Amps",
    description: "Wavefolder: folds the wave back on itself for bright, buzzy west-coast harmonics",
    hp: 6,
    params: &[lin("Fold", 0.0, 1.0, 0.3, Unit::Percent), lin("Bias", -1.0, 1.0, 0.0, Unit::Percent), lin("CV Amt", 0.0, 1.0, 0.5, Unit::Percent)],
    inputs: &["In", "Fold"],
    outputs: &["Out"],
};
static RING_MOD: ModuleInfo = ModuleInfo {
    name: "Ring Mod",
    category: "Filters & Amps",
    description: "Multiplies two signals: bells, clangs and amplitude modulation",
    hp: 4,
    params: &[lin("Mix", 0.0, 1.0, 1.0, Unit::Percent)],
    inputs: &["A", "B"],
    outputs: &["Out"],
};
static DELAY: ModuleInfo = ModuleInfo {
    name: "Delay",
    category: "Effects & I/O",
    description: "Echo up to two seconds, with feedback, darkening repeats and time CV",
    hp: 8,
    params: &[log("Time", 1.0, 2000.0, 300.0, Unit::Ms), lin("Feedback", 0.0, 1.0, 0.4, Unit::Percent), log("Tone", 500.0, 16000.0, 5000.0, Unit::Hz), lin("Mix", 0.0, 1.0, 0.35, Unit::Percent)],
    inputs: &["In", "Time"],
    outputs: &["Out"],
};
static REVERB: ModuleInfo = ModuleInfo {
    name: "Reverb",
    category: "Effects & I/O",
    description: "Plate reverb, mono in, stereo out",
    hp: 8,
    params: &[lin("Size", 0.0, 1.0, 0.5, Unit::Percent), lin("Decay", 0.0, 1.0, 0.6, Unit::Percent), lin("Mix", 0.0, 1.0, 0.35, Unit::Percent)],
    inputs: &["In"],
    outputs: &["L", "R"],
};
static DIVIDER: ModuleInfo = ModuleInfo {
    name: "Divider",
    category: "Sequencing",
    description: "Clock divider: one gate out for every N clocks in",
    hp: 4,
    params: &[int("Div", 1.0, 16.0, 2.0, Unit::None)],
    inputs: &["Clock", "Reset"],
    outputs: &["Out"],
};
static CHANCE: ModuleInfo = ModuleInfo {
    name: "Chance",
    category: "Sequencing",
    description: "Bernoulli gate: each trigger goes to A (with this chance) or else to B",
    hp: 4,
    params: &[lin("Prob", 0.0, 1.0, 0.5, Unit::Percent)],
    inputs: &["Trig"],
    outputs: &["A", "B"],
};

static MACRO_VCO: ModuleInfo = ModuleInfo {
    name: "Macro VCO",
    category: "Sources",
    description: "Macro oscillator with 24 synthesis models (virtual analog, FM, formants, chords, speech, strings, modal, drums...)",
    hp: 12,
    params: &[
        ParamSpec::choice("Model", "", crate::dsp::macro_osc::MODELS, 8),
        int("Octave", -3.0, 3.0, 0.0, Unit::None),
        lin("Harmonics", 0.0, 1.0, 0.5, Unit::Percent),
        lin("Timbre", 0.0, 1.0, 0.5, Unit::Percent),
        lin("Morph", 0.0, 1.0, 0.5, Unit::Percent),
        lin("Decay", 0.0, 1.0, 0.5, Unit::Percent),
        lin("LPG", 0.0, 1.0, 0.5, Unit::Percent),
    ],
    inputs: &["V/Oct", "Trig", "Harm", "Timbre", "Morph", "Level"],
    outputs: &["Out", "Aux"],
};
static SHAPE_VCO: ModuleInfo = ModuleInfo {
    name: "Shape VCO",
    category: "Sources",
    description: "Digital oscillator with 47 shapes (sync, triple saws, vowels, FM, plucked and bowed models, wavetables, noises)",
    hp: 10,
    params: &[ParamSpec::choice("Shape", "", &crate::dsp::braids_synth::SHAPE_LIST, 0), int("Octave", -3.0, 3.0, 0.0, Unit::None), lin("Timbre", 0.0, 1.0, 0.5, Unit::Percent), lin("Color", 0.0, 1.0, 0.5, Unit::Percent)],
    inputs: &["V/Oct", "Timbre", "Color", "Strike"],
    outputs: &["Out"],
};
static RESONATOR: ModuleInfo = ModuleInfo {
    name: "Resonator",
    category: "Sources",
    description: "Modal and string resonator struck by triggers or played by audio: modal, sympathetic strings, inharmonic string, FM voice, chords, string and reverb",
    hp: 12,
    params: &[
        ParamSpec::choice("Model", "", crate::dsp::rings_synth::MODELS, 0),
        int("Note", 24.0, 96.0, 48.0, Unit::None),
        lin("Structure", 0.0, 1.0, 0.3, Unit::Percent),
        lin("Brightness", 0.0, 1.0, 0.5, Unit::Percent),
        lin("Damping", 0.0, 1.0, 0.5, Unit::Percent),
        lin("Position", 0.0, 1.0, 0.5, Unit::Percent),
    ],
    inputs: &["In", "V/Oct", "Strum", "Struct", "Bright", "Damp", "Pos"],
    outputs: &["Odd", "Even"],
};
static TEXTURE: ModuleInfo = ModuleInfo {
    name: "Texture",
    category: "Effects & I/O",
    description: "Granular texture processor: grains, time stretch, looping delay and spectral freezes of what comes in",
    hp: 16,
    params: &[
        ParamSpec::choice("Mode", "", crate::dsp::fx::clouds::MODES, 0),
        lin("Position", 0.0, 1.0, 0.5, Unit::Percent),
        lin("Size", 0.0, 1.0, 0.5, Unit::Percent),
        int("Pitch", -24.0, 24.0, 0.0, Unit::Semi),
        lin("Density", 0.0, 1.0, 0.7, Unit::Percent),
        lin("Texture", 0.0, 1.0, 0.5, Unit::Percent),
        lin("Blend", 0.0, 1.0, 0.5, Unit::Percent),
        lin("Spread", 0.0, 1.0, 0.5, Unit::Percent),
        lin("Feedback", 0.0, 1.0, 0.0, Unit::Percent),
        lin("Reverb", 0.0, 1.0, 0.2, Unit::Percent),
    ],
    inputs: &["L", "R", "Freeze", "Trig", "Pos", "Size", "V/Oct", "Dens"],
    outputs: &["L", "R"],
};
static BASS_DRUM: ModuleInfo = ModuleInfo {
    name: "Bass Drum",
    category: "Drums",
    description: "Analog-style bass drum (bridged-T resonator with attack and self FM)",
    hp: 8,
    params: &[log("Tune", 25.0, 150.0, 52.0, Unit::Hz), lin("Tone", 0.0, 1.0, 0.5, Unit::Percent), lin("Decay", 0.0, 1.0, 0.5, Unit::Percent), lin("Punch", 0.0, 1.0, 0.4, Unit::Percent)],
    inputs: &["Trig", "Accent", "V/Oct"],
    outputs: &["Out"],
};
static SNARE_DRUM: ModuleInfo = ModuleInfo {
    name: "Snare Drum",
    category: "Drums",
    description: "Analog-style snare: tuned body modes and noise",
    hp: 8,
    params: &[log("Tune", 100.0, 400.0, 190.0, Unit::Hz), lin("Tone", 0.0, 1.0, 0.5, Unit::Percent), lin("Decay", 0.0, 1.0, 0.5, Unit::Percent), lin("Snappy", 0.0, 1.0, 0.5, Unit::Percent)],
    inputs: &["Trig", "Accent", "V/Oct"],
    outputs: &["Out"],
};
static HI_HAT: ModuleInfo = ModuleInfo {
    name: "Hi-Hat",
    category: "Drums",
    description: "Metallic hi-hat from six square oscillators and noise",
    hp: 8,
    params: &[log("Tune", 100.0, 800.0, 207.0, Unit::Hz), lin("Tone", 0.0, 1.0, 0.5, Unit::Percent), lin("Decay", 0.0, 1.0, 0.3, Unit::Percent), lin("Noise", 0.0, 1.0, 0.5, Unit::Percent)],
    inputs: &["Trig", "Accent", "V/Oct"],
    outputs: &["Out"],
};
static FUNCTION: ModuleInfo = ModuleInfo {
    name: "Function",
    category: "Envelopes & LFOs",
    description: "Function generator: a trigger starts a rise and a fall, with a curve from logarithmic to exponential; in cycle it loops as an LFO",
    hp: 8,
    params: &[log("Rise", 0.001, 10.0, 0.05, Unit::Sec), log("Fall", 0.001, 10.0, 0.3, Unit::Sec), lin("Shape", -1.0, 1.0, 0.0, Unit::Percent), ParamSpec::toggle("Cycle", "", false)],
    inputs: &["Trig", "Rise", "Fall", "Cycle"],
    outputs: &["Out", "EOR", "EOC"],
};
static RANDOM_GEN: ModuleInfo = ModuleInfo {
    name: "Random",
    category: "Sequencing",
    description: "Random gates and voltages on a clock (its own when unpatched): bias for the gates, spread and steps for the voltages, and deja vu to loop what came before",
    hp: 12,
    params: &[
        log("Rate", 0.1, 30.0, 4.0, Unit::Hz),
        lin("Bias", 0.0, 1.0, 0.5, Unit::Percent),
        lin("Jitter", 0.0, 1.0, 0.0, Unit::Percent),
        lin("Spread", 0.0, 5.0, 2.0, Unit::None),
        lin("Steps", 0.0, 1.0, 0.5, Unit::Percent),
        lin("Deja Vu", 0.0, 1.0, 0.0, Unit::Percent),
        int("Length", 1.0, 16.0, 8.0, Unit::None),
    ],
    inputs: &["Clock"],
    outputs: &["T1", "T2", "X1", "X2"],
};
static QUAD_VCA: ModuleInfo = ModuleInfo {
    name: "Quad VCA",
    category: "Filters & Amps",
    description: "Four VCAs, each with a gain knob and CV, linear to exponential; the Mix output sums them",
    hp: 12,
    params: &[lin("Gain 1", 0.0, 1.0, 0.0, Unit::Percent), lin("Gain 2", 0.0, 1.0, 0.0, Unit::Percent), lin("Gain 3", 0.0, 1.0, 0.0, Unit::Percent), lin("Gain 4", 0.0, 1.0, 0.0, Unit::Percent), lin("Response", 0.0, 1.0, 0.5, Unit::Percent)],
    inputs: &["In 1", "In 2", "In 3", "In 4", "CV 1", "CV 2", "CV 3", "CV 4"],
    outputs: &["Out 1", "Out 2", "Out 3", "Out 4", "Mix"],
};
static LOGIC: ModuleInfo = ModuleInfo {
    name: "Logic",
    category: "Utilities",
    description: "Analog logic: the larger and the smaller of two voltages, the rectified A, and a gate when A is above B",
    hp: 6,
    params: &[],
    inputs: &["A", "B"],
    outputs: &["Max", "Min", "|A|", "A>B"],
};
static SWITCH: ModuleInfo = ModuleInfo {
    name: "Switch",
    category: "Sequencing",
    description: "Sequential switch: each clock steps to the next input",
    hp: 8,
    params: &[int("Steps", 1.0, 4.0, 4.0, Unit::None)],
    inputs: &["Clock", "Reset", "In 1", "In 2", "In 3", "In 4"],
    outputs: &["Out"],
};
static META_MOD: ModuleInfo = ModuleInfo {
    name: "Meta Mod",
    category: "Filters & Amps",
    description: "Meta modulator: carrier and modulator through a morphing chain of algorithms: crossfade, fold, analog ring, digital ring, bit XOR, compare",
    hp: 8,
    params: &[lin("Algorithm", 0.0, 1.0, 0.0, Unit::Percent), lin("Timbre", 0.0, 1.0, 0.5, Unit::Percent), lin("Carrier", 0.0, 1.0, 1.0, Unit::Percent), lin("Mod", 0.0, 1.0, 1.0, Unit::Percent)],
    inputs: &["Carrier", "Mod", "Algo", "Timbre"],
    outputs: &["Out", "Aux"],
};
static FOLLOWER: ModuleInfo = ModuleInfo {
    name: "Follower",
    category: "Utilities",
    description: "Envelope follower: the level of what comes in as 0..10 V, and a gate above the threshold",
    hp: 6,
    params: &[lin("Gain", 0.0, 4.0, 1.0, Unit::None), log("Attack", 0.0005, 1.0, 0.005, Unit::Sec), log("Release", 0.005, 4.0, 0.2, Unit::Sec), lin("Threshold", 0.0, 10.0, 2.0, Unit::None)],
    inputs: &["In"],
    outputs: &["Env", "Gate"],
};
static OCTAVE: ModuleInfo = ModuleInfo {
    name: "Transpose",
    category: "Utilities",
    description: "Adds octaves and semitones to a 1 V/oct voltage (0 V when unpatched: a fixed pitch)",
    hp: 4,
    params: &[int("Octave", -4.0, 4.0, 0.0, Unit::None), int("Semi", -12.0, 12.0, 0.0, Unit::Semi)],
    inputs: &["In"],
    outputs: &["Out"],
};
static BURST: ModuleInfo = ModuleInfo {
    name: "Burst",
    category: "Sequencing",
    description: "A trigger fires a burst of triggers: ratchets, rolls and flams",
    hp: 6,
    params: &[int("Count", 1.0, 16.0, 4.0, Unit::None), log("Rate", 1.0, 60.0, 12.0, Unit::Hz)],
    inputs: &["Trig", "Rate"],
    outputs: &["Out", "EOC"],
};
static EUCLID: ModuleInfo = ModuleInfo {
    name: "Euclid",
    category: "Sequencing",
    description: "Euclidean rhythm: spreads the pulses as evenly as it can over the steps, one step per clock",
    hp: 6,
    params: &[int("Steps", 1.0, 32.0, 16.0, Unit::None), int("Pulses", 0.0, 32.0, 5.0, Unit::None), int("Rotate", 0.0, 31.0, 0.0, Unit::None)],
    inputs: &["Clock", "Reset"],
    outputs: &["Out", "Inv"],
};
static SHIFT_REG: ModuleInfo = ModuleInfo {
    name: "Shift Register",
    category: "Sequencing",
    description: "A looping random shift register: locked it repeats, opened it slowly changes; a voltage from its bits and a gate from its first",
    hp: 8,
    params: &[lin("Change", 0.0, 1.0, 0.1, Unit::Percent), int("Length", 2.0, 16.0, 8.0, Unit::None), lin("Range", 0.0, 5.0, 2.0, Unit::None)],
    inputs: &["Clock", "Change"],
    outputs: &["CV", "Gate", "Pulse"],
};
static STEREO_MIXER: ModuleInfo = ModuleInfo {
    name: "Stereo Mixer",
    category: "Utilities",
    description: "Four channels with level and pan to a stereo pair",
    hp: 12,
    params: &[
        lin("Level 1", 0.0, 1.0, 0.8, Unit::Percent),
        lin("Level 2", 0.0, 1.0, 0.8, Unit::Percent),
        lin("Level 3", 0.0, 1.0, 0.8, Unit::Percent),
        lin("Level 4", 0.0, 1.0, 0.8, Unit::Percent),
        lin("Pan 1", -1.0, 1.0, -0.5, Unit::Pan),
        lin("Pan 2", -1.0, 1.0, 0.5, Unit::Pan),
        lin("Pan 3", -1.0, 1.0, -0.2, Unit::Pan),
        lin("Pan 4", -1.0, 1.0, 0.2, Unit::Pan),
        lin("Master", 0.0, 2.0, 1.0, Unit::Percent),
    ],
    inputs: &["In 1", "In 2", "In 3", "In 4"],
    outputs: &["L", "R"],
};
static PULSE: ModuleInfo = ModuleInfo {
    name: "Pulse",
    category: "Utilities",
    description: "Turns a trigger into a gate of a set length, after a delay",
    hp: 4,
    params: &[log("Length", 0.001, 4.0, 0.1, Unit::Sec), log("Delay", 0.0001, 4.0, 0.0001, Unit::Sec)],
    inputs: &["Trig"],
    outputs: &["Out"],
};
static PLUCK: ModuleInfo = ModuleInfo {
    name: "Pluck",
    category: "Sources",
    description: "Plucked string (Karplus-Strong): a trigger plucks it, or feed it your own excitation",
    hp: 8,
    params: &[lin("Decay", 0.0, 1.0, 0.7, Unit::Percent), lin("Damping", 0.0, 1.0, 0.4, Unit::Percent), lin("Bright", 0.0, 1.0, 0.7, Unit::Percent), int("Octave", -3.0, 3.0, 0.0, Unit::None)],
    inputs: &["V/Oct", "Trig", "In"],
    outputs: &["Out"],
};
static FORMANT: ModuleInfo = ModuleInfo {
    name: "Formant",
    category: "Filters & Amps",
    description: "Vowel filter: three resonant formants morphing from A through E, I and O to U",
    hp: 8,
    params: &[lin("Vowel", 0.0, 1.0, 0.0, Unit::Percent), int("Shift", -12.0, 12.0, 0.0, Unit::Semi), lin("Res", 0.0, 1.0, 0.6, Unit::Percent), lin("Mix", 0.0, 1.0, 1.0, Unit::Percent)],
    inputs: &["In", "Vowel", "Shift"],
    outputs: &["Out"],
};
static TRIG_SEQ: ModuleInfo = ModuleInfo {
    name: "Trigger Seq",
    category: "Sequencing",
    description: "Sixteen-step trigger sequencer for drums: a step per clock, a gate and a trigger on every lit step",
    hp: 16,
    params: &[
        ParamSpec::toggle("1", "", true), ParamSpec::toggle("2", "", false), ParamSpec::toggle("3", "", false), ParamSpec::toggle("4", "", false),
        ParamSpec::toggle("5", "", true), ParamSpec::toggle("6", "", false), ParamSpec::toggle("7", "", false), ParamSpec::toggle("8", "", false),
        ParamSpec::toggle("9", "", true), ParamSpec::toggle("10", "", false), ParamSpec::toggle("11", "", false), ParamSpec::toggle("12", "", false),
        ParamSpec::toggle("13", "", true), ParamSpec::toggle("14", "", false), ParamSpec::toggle("15", "", false), ParamSpec::toggle("16", "", false),
    ],
    inputs: &["Clock", "Reset"],
    outputs: &["Gate", "Trig"],
};
static CRUSHER: ModuleInfo = ModuleInfo {
    name: "Crusher",
    category: "Effects & I/O",
    description: "Bit and sample rate reduction",
    hp: 6,
    params: &[int("Bits", 1.0, 16.0, 8.0, Unit::None), log("Rate", 200.0, 48000.0, 8000.0, Unit::Hz), lin("Mix", 0.0, 1.0, 1.0, Unit::Percent)],
    inputs: &["In", "Rate"],
    outputs: &["Out"],
};

/// A module in the rack: where it sits (HP from the left, row) and its knobs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub id: u32,
    pub kind: ModuleKind,
    pub x: u16,
    pub row: u16,
    pub params: Vec<f32>,
}

/// A patch cable from an output to an input.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cable {
    pub from: u32,
    pub out: u8,
    pub to: u32,
    pub input: u8,
    pub color: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    pub modules: Vec<Module>,
    pub cables: Vec<Cable>,
    #[serde(default)]
    pub next_id: u32,
}

impl Patch {
    pub fn module(&self, id: u32) -> Option<&Module> {
        self.modules.iter().find(|m| m.id == id)
    }

    /// Adds a module at the first free place wide enough, from row `row`.
    pub fn add(&mut self, kind: ModuleKind, row: u16) -> u32 {
        let hp = kind.info().hp;
        let (x, row) = self.free_spot(hp, row, 0, None);
        self.add_at(kind, x, row)
    }

    pub fn add_at(&mut self, kind: ModuleKind, x: u16, row: u16) -> u32 {
        let id = self.next_id.max(self.modules.iter().map(|m| m.id + 1).max().unwrap_or(0));
        self.next_id = id + 1;
        self.modules.push(Module { id, kind, x, row, params: crate::params::defaults(kind.info().params) });
        id
    }

    /// The first place from (`x`, `row`) where `hp` fits, ignoring module `skip`.
    pub fn free_spot(&self, hp: u16, row: u16, x: u16, skip: Option<u32>) -> (u16, u16) {
        let mut row = row;
        let mut start = x.min(ROW_HP.saturating_sub(hp));
        loop {
            let taken: Vec<(u16, u16)> = self.modules.iter().filter(|m| m.row == row && Some(m.id) != skip).map(|m| (m.x, m.x + m.kind.info().hp)).collect();
            for at in start..=ROW_HP.saturating_sub(hp) {
                if taken.iter().all(|(a, b)| at + hp <= *a || at >= *b) {
                    return (at, row);
                }
            }
            row += 1;
            start = 0;
        }
    }

    /// Connects `from`'s output to `to`'s input; an input takes one cable.
    pub fn connect(&mut self, from: u32, out: u8, to: u32, input: u8) {
        self.cables.retain(|c| !(c.to == to && c.input == input));
        let color = (self.cables.len() % 6) as u8;
        self.cables.push(Cable { from, out, to, input, color });
    }

    pub fn remove(&mut self, id: u32) {
        self.modules.retain(|m| m.id != id);
        self.cables.retain(|c| c.from != id && c.to != id);
    }

    /// Fixes parameter lists and drops cables to missing modules or jacks.
    pub fn sanitize(&mut self) {
        for m in &mut self.modules {
            crate::params::fix_len(&mut m.params, m.kind.info().params);
        }
        let ok = |id: u32, port: u8, out: bool, mods: &[Module]| mods.iter().any(|m| m.id == id && (port as usize) < if out { m.kind.info().outputs.len() } else { m.kind.info().inputs.len() });
        let mods = self.modules.clone();
        self.cables.retain(|c| ok(c.from, c.out, true, &mods) && ok(c.to, c.input, false, &mods));
        self.next_id = self.next_id.max(self.modules.iter().map(|m| m.id + 1).max().unwrap_or(0));
    }

    /// Changes when modules or cables do (not knobs): the engine rebuilds then.
    pub fn structure(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for m in &self.modules {
            (m.id, m.kind).hash(&mut h);
        }
        for c in &self.cables {
            (c.from, c.out, c.to, c.input).hash(&mut h);
        }
        h.finish()
    }
}

/// The instrument's own parameters: eight macros, voices and volume.
pub const VOICES: usize = 8;
pub const VOLUME: usize = 9;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Macro 1", "Macros", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Macro 2", "Macros", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Macro 3", "Macros", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Macro 4", "Macros", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Macro 5", "Macros", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Macro 6", "Macros", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Macro 7", "Macros", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Macro 8", "Macros", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Voices", "Voice", 1.0, 8.0, 1.0, Unit::None),
    ParamSpec::lin("Volume", "Voice", -24.0, 12.0, 0.0, Unit::Db),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modules_find_a_free_place_and_inputs_take_one_cable() {
        let mut p = Patch::default();
        let a = p.add(ModuleKind::Vco, 0);
        let b = p.add(ModuleKind::Vcf, 0);
        let (ma, mb) = (p.module(a).unwrap().clone(), p.module(b).unwrap().clone());
        assert_eq!((ma.x, mb.x), (0, ModuleKind::Vco.info().hp));
        // A full row sends a module to the next one.
        while p.modules.iter().all(|m| m.row == 0) {
            p.add(ModuleKind::Sequencer, 0);
        }
        assert_eq!(p.modules.last().unwrap().row, 1);
        p.connect(a, 2, b, 0);
        p.connect(a, 3, b, 0);
        assert_eq!(p.cables.len(), 1);
        assert_eq!(p.cables[0].out, 3);
        p.remove(a);
        assert!(p.cables.is_empty());
    }

    #[test]
    fn sanitize_drops_cables_to_nowhere() {
        let mut p = Patch::default();
        let a = p.add(ModuleKind::Lfo, 0);
        let b = p.add(ModuleKind::Out, 0);
        p.connect(a, 0, b, 0);
        p.cables.push(Cable { from: a, out: 9, to: b, input: 0, color: 0 });
        p.cables.push(Cable { from: 77, out: 0, to: b, input: 1, color: 0 });
        p.modules[0].params.truncate(1);
        p.sanitize();
        assert_eq!(p.cables.len(), 1);
        assert_eq!(p.modules[0].params.len(), ModuleKind::Lfo.info().params.len());
    }

    #[test]
    fn every_module_fits_its_row() {
        // Knobs (52 x 58) and jacks (38 x 36) must fit a 3U row of 352 px.
        for kind in ModuleKind::ALL {
            let info = kind.info();
            let w = info.hp as f32 * 15.0 - 2.0;
            let cols = (((w - 6.0) / 52.0).floor() as usize).max(1);
            let n = if kind == ModuleKind::Macros { 8 } else { info.params.len() };
            let jcols = (((w - 6.0) / 38.0).floor() as usize).max(1);
            let h = 27.0 + n.div_ceil(cols) as f32 * 58.0 + 8.0 + (info.inputs.len().div_ceil(jcols) + info.outputs.len().div_ceil(jcols)) as f32 * 36.0;
            assert!(h <= 352.0, "{}: {h}", info.name);
        }
    }
}
