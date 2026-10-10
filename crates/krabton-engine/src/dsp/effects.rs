//! Built-in audio effects.

use super::reverb::Reverb;
use super::*;
use crate::params::{ParamSpec, Unit};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectKind {
    Eq3,
    AutoFilter,
    Compressor,
    Saturator,
    Delay,
    Reverb,
    Chorus,
    Redux,
    Utility,
    Scope,
    LensEq,
    LensComp,
    LensLimiter,
    Phaser,
    Flanger,
    AutoPan,
    FreqShifter,
    Rotary,
    Gate,
    Transient,
    Multiband,
    Echo,
    BeatRepeat,
    Shifter,
    AutoWah,
    Resonators,
    Vinyl,
    AwGalactic,
    AwVerbity2,
    AwChamber,
    AwToTape6,
    AwIronOxide5,
    AwPurestDrive,
    AwDensity2,
    AwTube2,
    AwMackity,
    AwPressure4,
    AwButterComp2,
    AwAir3,
    AwCapacitor2,
    AwDeRez2,
    AwTapeDelay2,
    Rings,
    Clouds,
    /// A hosted effect plug-in (`Device::plugin`), not in `ALL`.
    Plugin,
}

const FMODES: &[&str] = &["Lowpass", "Highpass", "Bandpass", "Notch"];
const SYNC: &[&str] = &["1/32", "1/16", "1/16 D", "1/8", "1/8 D", "1/4", "1/4 D", "1/2", "1 Bar"];
const SYNC_BEATS: [f32; 9] = [0.125, 0.25, 0.375, 0.5, 0.75, 1.0, 1.5, 2.0, 4.0];
const LFO_SYNC: &[&str] = &["1/16", "1/8", "1/4", "1/2", "1 Bar", "2 Bars", "4 Bars", "8 Bars"];
const LFO_SYNC_BEATS: [f32; 8] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
const SAT_TYPES: &[&str] = &["Soft", "Hard", "Fold", "Tube"];

static EQ3: &[ParamSpec] = &[
    ParamSpec::lin("Low", "EQ", -40.0, 12.0, 0.0, Unit::Db),
    ParamSpec::lin("Mid", "EQ", -40.0, 12.0, 0.0, Unit::Db),
    ParamSpec::lin("High", "EQ", -40.0, 12.0, 0.0, Unit::Db),
    ParamSpec::log("Freq Lo", "EQ", 40.0, 1000.0, 250.0, Unit::Hz),
    ParamSpec::log("Freq Hi", "EQ", 800.0, 12000.0, 2500.0, Unit::Hz),
];
static AUTOFILTER: &[ParamSpec] = &[
    ParamSpec::choice("Type", "Filter", FMODES, 0),
    ParamSpec::log("Cutoff", "Filter", 20.0, 20000.0, 1200.0, Unit::Hz),
    ParamSpec::lin("Resonance", "Filter", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Drive", "Filter", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("LFO Amt", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("LFO Rate", "LFO", LFO_SYNC, 2),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];
static COMPRESSOR: &[ParamSpec] = &[
    ParamSpec::lin("Threshold", "Dynamics", -60.0, 0.0, -18.0, Unit::Db),
    ParamSpec::log("Ratio", "Dynamics", 1.0, 20.0, 4.0, Unit::Ratio),
    ParamSpec::log("Attack", "Dynamics", 0.1, 100.0, 10.0, Unit::Ms),
    ParamSpec::log("Release", "Dynamics", 5.0, 1000.0, 120.0, Unit::Ms),
    ParamSpec::lin("Makeup", "Output", 0.0, 24.0, 0.0, Unit::Db),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];
static SATURATOR: &[ParamSpec] = &[
    ParamSpec::lin("Drive", "Shape", 0.0, 36.0, 6.0, Unit::Db),
    ParamSpec::choice("Type", "Shape", SAT_TYPES, 0),
    ParamSpec::log("Color", "Shape", 1000.0, 20000.0, 20000.0, Unit::Hz),
    ParamSpec::lin("Output", "Output", -24.0, 6.0, 0.0, Unit::Db),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];
static DELAY: &[ParamSpec] = &[
    ParamSpec::choice("Time", "Delay", SYNC, 4),
    ParamSpec::lin("Feedback", "Delay", 0.0, 0.95, 0.4, Unit::Percent),
    ParamSpec::toggle("Ping Pong", "Delay", true),
    ParamSpec::log("Low Cut", "Filter", 20.0, 2000.0, 200.0, Unit::Hz),
    ParamSpec::log("High Cut", "Filter", 1000.0, 20000.0, 6000.0, Unit::Hz),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.3, Unit::Percent),
];
static REVERB: &[ParamSpec] = &[
    ParamSpec::lin("Size", "Room", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Damping", "Room", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Width", "Room", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Predelay", "Input", 0.0, 200.0, 15.0, Unit::Ms),
    ParamSpec::log("Low Cut", "Input", 20.0, 1000.0, 150.0, Unit::Hz),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.3, Unit::Percent),
];
static CHORUS: &[ParamSpec] = &[
    ParamSpec::log("Rate", "Mod", 0.05, 5.0, 0.8, Unit::Hz),
    ParamSpec::lin("Depth", "Mod", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Delay", "Mod", 2.0, 30.0, 8.0, Unit::Ms),
    ParamSpec::lin("Feedback", "Mod", -0.9, 0.9, 0.0, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.5, Unit::Percent),
];
static REDUX: &[ParamSpec] = &[
    ParamSpec::int("Bits", "Crush", 1.0, 16.0, 8.0, Unit::None),
    ParamSpec::int("Downsample", "Crush", 1.0, 50.0, 4.0, Unit::None),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];
pub const SCOPE_MODES: &[&str] = &["Wave", "Time", "XY", "Vector", "Spectrum"];
pub const SCOPE_COLORS: &[&str] = &["Green", "Amber", "Ice", "Pink", "White"];
pub const SCOPE_TRIGGERS: &[&str] = &["Rising", "Falling", "Free"];
pub const SCOPE_CHANNELS: &[&str] = &["Mid", "Stereo", "Left", "Right"];
pub const SCOPE_MODE: usize = 0;
pub const SCOPE_CYCLES: usize = 1;
pub const SCOPE_TIME: usize = 2;
pub const SCOPE_TRIGGER: usize = 3;
pub const SCOPE_CHANNEL: usize = 4;
pub const SCOPE_AUTO: usize = 5;
pub const SCOPE_GAIN: usize = 6;
pub const SCOPE_GLOW: usize = 7;
pub const SCOPE_FREEZE: usize = 8;
pub const SCOPE_COLOR: usize = 9;
pub const SCOPE_LINE: usize = 10;
pub const SCOPE_FILL: usize = 11;
pub const SCOPE_GRID: usize = 12;
pub const SCOPE_PHASE: usize = 13;
static SCOPE: &[ParamSpec] = &[
    ParamSpec::choice("Mode", "Scope", SCOPE_MODES, 0),
    ParamSpec::int("Cycles", "Scope", 1.0, 8.0, 2.0, Unit::None),
    ParamSpec::log("Time", "Scope", 1.0, 1000.0, 20.0, Unit::Ms),
    ParamSpec::choice("Trigger", "Scope", SCOPE_TRIGGERS, 0),
    ParamSpec::choice("Channel", "Display", SCOPE_CHANNELS, 0),
    ParamSpec::toggle("Auto Gain", "Display", true),
    ParamSpec::lin("Gain", "Display", -24.0, 36.0, 0.0, Unit::Db),
    ParamSpec::lin("Glow", "Display", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::toggle("Freeze", "Display", false),
    ParamSpec::choice("Color", "Look", SCOPE_COLORS, 0),
    ParamSpec::lin("Line", "Look", 0.5, 4.0, 1.4, Unit::None),
    ParamSpec::toggle("Fill", "Look", false),
    ParamSpec::toggle("Grid", "Look", true),
    ParamSpec::lin("Phase", "Scope", 0.0, 1.0, 0.0, Unit::Percent),
];
static UTILITY: &[ParamSpec] = &[
    ParamSpec::lin("Gain", "Utility", -60.0, 24.0, 0.0, Unit::Db),
    ParamSpec::lin("Width", "Utility", 0.0, 2.0, 1.0, Unit::Percent),
    ParamSpec::lin("Pan", "Utility", -1.0, 1.0, 0.0, Unit::Pan),
    ParamSpec::toggle("Phase L", "Utility", false),
    ParamSpec::toggle("Phase R", "Utility", false),
];

impl EffectKind {
    pub const ALL: [EffectKind; 44] = [
        EffectKind::Eq3,
        EffectKind::AutoFilter,
        EffectKind::Compressor,
        EffectKind::Saturator,
        EffectKind::Delay,
        EffectKind::Reverb,
        EffectKind::Chorus,
        EffectKind::Redux,
        EffectKind::Utility,
        EffectKind::Scope,
        EffectKind::LensEq,
        EffectKind::LensComp,
        EffectKind::LensLimiter,
        EffectKind::Phaser,
        EffectKind::Flanger,
        EffectKind::AutoPan,
        EffectKind::FreqShifter,
        EffectKind::Rotary,
        EffectKind::Gate,
        EffectKind::Transient,
        EffectKind::Multiband,
        EffectKind::Echo,
        EffectKind::BeatRepeat,
        EffectKind::Shifter,
        EffectKind::AutoWah,
        EffectKind::Resonators,
        EffectKind::Vinyl,
        EffectKind::Rings,
        EffectKind::Clouds,
        EffectKind::AwGalactic,
        EffectKind::AwVerbity2,
        EffectKind::AwChamber,
        EffectKind::AwToTape6,
        EffectKind::AwIronOxide5,
        EffectKind::AwPurestDrive,
        EffectKind::AwDensity2,
        EffectKind::AwTube2,
        EffectKind::AwMackity,
        EffectKind::AwPressure4,
        EffectKind::AwButterComp2,
        EffectKind::AwAir3,
        EffectKind::AwCapacitor2,
        EffectKind::AwDeRez2,
        EffectKind::AwTapeDelay2,
    ];

    /// Where it goes in the browser and the effect menu.
    pub fn category(self) -> &'static str {
        match self {
            EffectKind::Eq3 => "EQ & Filters",
            EffectKind::AutoFilter => "EQ & Filters",
            EffectKind::Compressor => "Dynamics",
            EffectKind::Saturator => "Distortion",
            EffectKind::Delay => "Delay & Loop",
            EffectKind::Reverb => "Reverb & Resonance",
            EffectKind::Chorus => "Modulation",
            EffectKind::Redux => "Distortion",
            EffectKind::Utility => "Utilities",
            EffectKind::Scope => "Utilities",
            EffectKind::LensEq => "EQ & Filters",
            EffectKind::LensComp => "Dynamics",
            EffectKind::LensLimiter => "Dynamics",
            EffectKind::Plugin => "Plug-ins",
            EffectKind::Rings => "Reverb & Resonance",
            EffectKind::Clouds => "Delay & Loop",
            EffectKind::AwGalactic => "Airwindows",
            EffectKind::AwVerbity2 => "Airwindows",
            EffectKind::AwChamber => "Airwindows",
            EffectKind::AwToTape6 => "Airwindows",
            EffectKind::AwIronOxide5 => "Airwindows",
            EffectKind::AwPurestDrive => "Airwindows",
            EffectKind::AwDensity2 => "Airwindows",
            EffectKind::AwTube2 => "Airwindows",
            EffectKind::AwMackity => "Airwindows",
            EffectKind::AwPressure4 => "Airwindows",
            EffectKind::AwButterComp2 => "Airwindows",
            EffectKind::AwAir3 => "Airwindows",
            EffectKind::AwCapacitor2 => "Airwindows",
            EffectKind::AwDeRez2 => "Airwindows",
            EffectKind::AwTapeDelay2 => "Airwindows",

            EffectKind::Phaser => "Modulation",
            EffectKind::Flanger => "Modulation",
            EffectKind::AutoPan => "Modulation",
            EffectKind::FreqShifter => "Modulation",
            EffectKind::Rotary => "Modulation",
            EffectKind::Gate => "Dynamics",
            EffectKind::Transient => "Dynamics",
            EffectKind::Multiband => "Dynamics",
            EffectKind::Echo => "Delay & Loop",
            EffectKind::BeatRepeat => "Delay & Loop",
            EffectKind::Shifter => "Pitch",
            EffectKind::AutoWah => "EQ & Filters",
            EffectKind::Resonators => "Reverb & Resonance",
            EffectKind::Vinyl => "Distortion",
        }
    }

    /// The categories in menu order.
    pub const CATEGORIES: [&'static str; 9] = ["Delay & Loop", "Distortion", "Dynamics", "EQ & Filters", "Modulation", "Pitch", "Reverb & Resonance", "Utilities", "Airwindows"];

    /// Devices with their own display (they send audio and meters to the UI).
    pub fn has_display(self) -> bool {
        matches!(self, EffectKind::Scope | EffectKind::LensEq | EffectKind::LensComp | EffectKind::LensLimiter)
    }

    pub fn name(self) -> &'static str {
        match self {
            EffectKind::Eq3 => "EQ Three",
            EffectKind::AutoFilter => "Auto Filter",
            EffectKind::Compressor => "Compressor",
            EffectKind::Saturator => "Saturator",
            EffectKind::Delay => "Delay",
            EffectKind::Reverb => "Reverb",
            EffectKind::Chorus => "Chorus",
            EffectKind::Redux => "Redux",
            EffectKind::Utility => "Utility",
            EffectKind::Scope => "Oscilloscope",
            EffectKind::LensEq => "Lens EQ",
            EffectKind::LensComp => "Lens Comp",
            EffectKind::LensLimiter => "Lens Limiter",
            EffectKind::Phaser => "Phaser",
            EffectKind::Flanger => "Flanger",
            EffectKind::AutoPan => "Auto Pan",
            EffectKind::FreqShifter => "Frequency Shifter",
            EffectKind::Rotary => "Rotary",
            EffectKind::Gate => "Gate",
            EffectKind::Transient => "Transient",
            EffectKind::Multiband => "Multiband",
            EffectKind::Echo => "Echo",
            EffectKind::BeatRepeat => "Beat Repeat",
            EffectKind::Shifter => "Shifter",
            EffectKind::AutoWah => "Auto Wah",
            EffectKind::Resonators => "Resonators",
            EffectKind::Vinyl => "Vinyl",
            EffectKind::AwGalactic => "Galactic",
            EffectKind::AwVerbity2 => "Verbity2",
            EffectKind::AwChamber => "Chamber",
            EffectKind::AwToTape6 => "ToTape6",
            EffectKind::AwIronOxide5 => "IronOxide5",
            EffectKind::AwPurestDrive => "PurestDrive",
            EffectKind::AwDensity2 => "Density2",
            EffectKind::AwTube2 => "Tube2",
            EffectKind::AwMackity => "Mackity",
            EffectKind::AwPressure4 => "Pressure4",
            EffectKind::AwButterComp2 => "ButterComp2",
            EffectKind::AwAir3 => "Air3",
            EffectKind::AwCapacitor2 => "Capacitor2",
            EffectKind::AwDeRez2 => "DeRez2",
            EffectKind::AwTapeDelay2 => "TapeDelay2",
            EffectKind::Rings => "Rings",
            EffectKind::Clouds => "Clouds",
            EffectKind::Plugin => "Plug-in",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            EffectKind::Eq3 => "DJ-style three band EQ with kill",
            EffectKind::AutoFilter => "Resonant filter with tempo-synced LFO",
            EffectKind::Compressor => "Feed-forward compressor with parallel mix",
            EffectKind::Saturator => "Waveshaping distortion",
            EffectKind::Delay => "Tempo-synced stereo / ping-pong delay",
            EffectKind::Reverb => "Algorithmic room reverb",
            EffectKind::Chorus => "Stereo chorus / flanger",
            EffectKind::Redux => "Bit and sample rate reduction",
            EffectKind::Utility => "Gain, width, pan and phase",
            EffectKind::Scope => "See the sound: the wave locked to its pitch, a triggered time base, XY, vector or a spectrum",
            EffectKind::LensEq => "Twelve-band EQ with a big curve, spectrum analyzer, mid/side and dynamic bands",
            EffectKind::LensComp => "Compressor with styles, lookahead and a scrolling level display",
            EffectKind::LensLimiter => "True-peak limiter with styles and a loudness meter (LUFS)",
            EffectKind::Phaser => "Sweeping notches from a chain of all-pass filters, with feedback and a stereo offset",
            EffectKind::Flanger => "Jet-plane sweeps from a short delay moved by an LFO, with feedback",
            EffectKind::AutoPan => "Tremolo, or panning from side to side, with tempo sync and several LFO shapes",
            EffectKind::FreqShifter => "Moves every frequency by the same number of Hz (metallic, inharmonic), or ring modulates",
            EffectKind::Rotary => "A rotating speaker cabinet: horn and drum spinning slow or fast, with inertia and drive",
            EffectKind::Gate => "Shuts out sound below a threshold, with return, attack, hold, release and a floor",
            EffectKind::Transient => "Harder or softer attacks and longer or shorter tails, whatever the level",
            EffectKind::Multiband => "Three-band upward and downward compression, over the top: quiet comes up, loud goes down",
            EffectKind::Echo => "Tape echo with wow, flutter, saturation and feedback that can run away",
            EffectKind::BeatRepeat => "Grabs slices in time with the song and repeats them: stutters, rolls and glitches",
            EffectKind::Shifter => "Pitch shifter, up to two octaves either way, with feedback cascades and stereo detune",
            EffectKind::AutoWah => "Envelope filter: playing louder opens (or closes) a resonant filter",
            EffectKind::Resonators => "Five tuned resonators on a note and chord: drums and noise ring as chords",
            EffectKind::Vinyl => "An old record: crackle, pops, hiss, rumble, wow and worn-out bandwidth",
            EffectKind::AwGalactic => "A huge, lush, slowly drifting stereo reverb for pads and ambient spaces (Airwindows)",
            EffectKind::AwVerbity2 => "A smooth, dense room-to-hall reverb with size, sustain and darkness (Airwindows)",
            EffectKind::AwChamber => "An echo-chamber reverb with size, decay, highpass and lowpass (Airwindows)",
            EffectKind::AwToTape6 => "Tape machine: saturation, head bump, softened highs and flutter (Airwindows)",
            EffectKind::AwIronOxide5 => "Punchy tape saturation with high and low tape speeds, flutter and tape noise (Airwindows)",
            EffectKind::AwPurestDrive => "Subtle, musical saturation that thickens the sound and keeps the highs (Airwindows)",
            EffectKind::AwDensity2 => "Density and drive when turned up, thinner sound when turned down (Airwindows)",
            EffectKind::AwTube2 => "Warm, asymmetric tube saturation (Airwindows)",
            EffectKind::AwMackity => "The gritty overdriven input stage of a classic compact mixer (Airwindows)",
            EffectKind::AwPressure4 => "Program-dependent stereo compressor and limiter with a soft-clipped output (Airwindows)",
            EffectKind::AwButterComp2 => "Soft, glue-like compression without obvious pumping (Airwindows)",
            EffectKind::AwAir3 => "Boosts or cuts the airy top and the grounded lows, transparently (Airwindows)",
            EffectKind::AwCapacitor2 => "Soft analog-style lowpass and highpass with capacitor non-linearity (Airwindows)",
            EffectKind::AwDeRez2 => "Lower sample rate and bit depth: crunchy retro digital lo-fi (Airwindows)",
            EffectKind::AwTapeDelay2 => "Warm tape echo with a resonant filter on the repeats and flutter (Airwindows)",
            EffectKind::Clouds => "Granular texture machine after Mutable Instruments Clouds: grains, time stretch, looping delay and spectral freezes",
            EffectKind::Rings => "The Rings resonator rung by the track's audio: drums and noise become tuned strings, bells and chords",
            EffectKind::Plugin => "A CLAP effect plug-in",
        }
    }

    pub fn specs(self) -> &'static [ParamSpec] {
        match self {
            EffectKind::Eq3 => EQ3,
            EffectKind::AutoFilter => AUTOFILTER,
            EffectKind::Compressor => COMPRESSOR,
            EffectKind::Saturator => SATURATOR,
            EffectKind::Delay => DELAY,
            EffectKind::Reverb => REVERB,
            EffectKind::Chorus => CHORUS,
            EffectKind::Redux => REDUX,
            EffectKind::Utility => UTILITY,
            EffectKind::Scope => SCOPE,
            EffectKind::LensEq => super::lens_eq::SPECS,
            EffectKind::LensComp => super::lens_comp::SPECS,
            EffectKind::LensLimiter => super::lens_limiter::SPECS,
            EffectKind::Phaser => super::fx::phaser::SPECS,
            EffectKind::Flanger => super::fx::flanger::SPECS,
            EffectKind::AutoPan => super::fx::auto_pan::SPECS,
            EffectKind::FreqShifter => super::fx::freq_shifter::SPECS,
            EffectKind::Rotary => super::fx::rotary::SPECS,
            EffectKind::Gate => super::fx::gate::SPECS,
            EffectKind::Transient => super::fx::transient::SPECS,
            EffectKind::Multiband => super::fx::multiband::SPECS,
            EffectKind::Echo => super::fx::echo::SPECS,
            EffectKind::BeatRepeat => super::fx::beat_repeat::SPECS,
            EffectKind::Shifter => super::fx::shifter::SPECS,
            EffectKind::AutoWah => super::fx::auto_wah::SPECS,
            EffectKind::Resonators => super::fx::resonators::SPECS,
            EffectKind::Vinyl => super::fx::vinyl::SPECS,
            EffectKind::AwGalactic => super::fx::airwindows::GALACTIC,
            EffectKind::AwVerbity2 => super::fx::airwindows::VERBITY2,
            EffectKind::AwChamber => super::fx::airwindows::CHAMBER,
            EffectKind::AwToTape6 => super::fx::airwindows::TOTAPE6,
            EffectKind::AwIronOxide5 => super::fx::airwindows::IRONOXIDE5,
            EffectKind::AwPurestDrive => super::fx::airwindows::PURESTDRIVE,
            EffectKind::AwDensity2 => super::fx::airwindows::DENSITY2,
            EffectKind::AwTube2 => super::fx::airwindows::TUBE2,
            EffectKind::AwMackity => super::fx::airwindows::MACKITY,
            EffectKind::AwPressure4 => super::fx::airwindows::PRESSURE4,
            EffectKind::AwButterComp2 => super::fx::airwindows::BUTTERCOMP2,
            EffectKind::AwAir3 => super::fx::airwindows::AIR3,
            EffectKind::AwCapacitor2 => super::fx::airwindows::CAPACITOR2,
            EffectKind::AwDeRez2 => super::fx::airwindows::DEREZ2,
            EffectKind::AwTapeDelay2 => super::fx::airwindows::TAPEDELAY2,
            EffectKind::Rings => super::rings_synth::FX_SPECS,
            EffectKind::Clouds => super::fx::clouds::SPECS,
            EffectKind::Plugin => &[],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FxCtx {
    pub sr: f32,
    pub bpm: f32,
    /// Song position at the start of the block, in beats.
    pub beat: f64,
    pub playing: bool,
}

pub enum EffectState {
    Eq3 { lo: [Svf; 2], hi: [Svf; 2] },
    AutoFilter { f: [Svf; 2], phase: f32 },
    Compressor { env_db: f32 },
    Saturator { color: [OnePole; 2], dc: [DcBlock; 2] },
    Delay(Box<DelayState>),
    Reverb(Box<Reverb>),
    Chorus(Box<ChorusState>),
    Redux { hold: [f32; 2], counter: u32 },
    Utility,
    /// Passes audio through; the engine copies it to a scope buffer.
    Scope(crate::scope::DeviceTaps),
    LensEq(Box<super::lens_eq::LensEqState>),
    LensComp(Box<super::lens_comp::LensCompState>),
    LensLimiter(Box<super::lens_limiter::LensLimiterState>),
    Phaser(Box<super::fx::phaser::Phaser>),
    Flanger(Box<super::fx::flanger::Flanger>),
    AutoPan(Box<super::fx::auto_pan::AutoPan>),
    FreqShifter(Box<super::fx::freq_shifter::FreqShifter>),
    Rotary(Box<super::fx::rotary::Rotary>),
    Gate(Box<super::fx::gate::Gate>),
    Transient(Box<super::fx::transient::Transient>),
    Multiband(Box<super::fx::multiband::Multiband>),
    Echo(Box<super::fx::echo::Echo>),
    BeatRepeat(Box<super::fx::beat_repeat::BeatRepeat>),
    Shifter(Box<super::fx::shifter::Shifter>),
    AutoWah(Box<super::fx::auto_wah::AutoWah>),
    Resonators(Box<super::fx::resonators::Resonators>),
    Vinyl(Box<super::fx::vinyl::Vinyl>),
    Rings(Box<super::rings_synth::RingsFx>),
    Clouds(Box<super::fx::clouds::CloudsFx>),
    /// An Airwindows effect.
    Airwindows(EffectKind, Box<dyn krabton_airwindows::Effect>),
    /// A hosted plug-in; passes audio through until its processor arrives.
    Plugin { instance: u64, proc: Option<Box<dyn crate::plugin::AudioProcessor>> },
}

pub struct DelayState {
    buf: [Vec<f32>; 2],
    idx: usize,
    cur: f32,
    lc: [OnePole; 2],
    hc: [OnePole; 2],
}

pub struct ChorusState {
    buf: [Vec<f32>; 2],
    idx: usize,
    phase: f32,
}

impl EffectState {
    pub fn new(kind: EffectKind, sr: f32) -> Self {
        match kind {
            EffectKind::Eq3 => EffectState::Eq3 { lo: [Svf::new(); 2], hi: [Svf::new(); 2] },
            EffectKind::AutoFilter => EffectState::AutoFilter { f: [Svf::new(); 2], phase: 0.0 },
            EffectKind::Compressor => EffectState::Compressor { env_db: 0.0 },
            EffectKind::Saturator => EffectState::Saturator { color: [OnePole::default(); 2], dc: [DcBlock::default(); 2] },
            EffectKind::Delay => {
                let len = (sr * 12.0) as usize;
                EffectState::Delay(Box::new(DelayState {
                    buf: [vec![0.0; len], vec![0.0; len]],
                    idx: 0,
                    cur: -1.0,
                    lc: [OnePole::default(); 2],
                    hc: [OnePole::default(); 2],
                }))
            }
            EffectKind::Reverb => EffectState::Reverb(Box::new(Reverb::new(sr))),
            EffectKind::Chorus => {
                let len = (sr * 0.08) as usize;
                EffectState::Chorus(Box::new(ChorusState { buf: [vec![0.0; len], vec![0.0; len]], idx: 0, phase: 0.0 }))
            }
            EffectKind::Redux => EffectState::Redux { hold: [0.0; 2], counter: 0 },
            EffectKind::Utility => EffectState::Utility,
            EffectKind::Scope => EffectState::Scope(Default::default()),
            EffectKind::LensEq => EffectState::LensEq(Box::default()),
            EffectKind::LensComp => EffectState::LensComp(Box::default()),
            EffectKind::LensLimiter => EffectState::LensLimiter(Box::new(super::lens_limiter::LensLimiterState::new(sr))),
            EffectKind::Phaser => EffectState::Phaser(Box::new(super::fx::phaser::Phaser::new())),
            EffectKind::Flanger => EffectState::Flanger(Box::new(super::fx::flanger::Flanger::new(sr))),
            EffectKind::AutoPan => EffectState::AutoPan(Box::new(super::fx::auto_pan::AutoPan::new())),
            EffectKind::FreqShifter => EffectState::FreqShifter(Box::new(super::fx::freq_shifter::FreqShifter::new())),
            EffectKind::Rotary => EffectState::Rotary(Box::new(super::fx::rotary::Rotary::new(sr))),
            EffectKind::Gate => EffectState::Gate(Box::new(super::fx::gate::Gate::new())),
            EffectKind::Transient => EffectState::Transient(Box::new(super::fx::transient::Transient::new())),
            EffectKind::Multiband => EffectState::Multiband(Box::new(super::fx::multiband::Multiband::new())),
            EffectKind::Echo => EffectState::Echo(Box::new(super::fx::echo::Echo::new(sr))),
            EffectKind::BeatRepeat => EffectState::BeatRepeat(Box::new(super::fx::beat_repeat::BeatRepeat::new(sr))),
            EffectKind::Shifter => EffectState::Shifter(Box::new(super::fx::shifter::Shifter::new(sr))),
            EffectKind::AutoWah => EffectState::AutoWah(Box::new(super::fx::auto_wah::AutoWah::new())),
            EffectKind::Resonators => EffectState::Resonators(Box::new(super::fx::resonators::Resonators::new(sr))),
            EffectKind::Vinyl => EffectState::Vinyl(Box::new(super::fx::vinyl::Vinyl::new(sr))),
            EffectKind::AwGalactic => Self::airwindows(kind, "Galactic", sr),
            EffectKind::AwVerbity2 => Self::airwindows(kind, "Verbity2", sr),
            EffectKind::AwChamber => Self::airwindows(kind, "Chamber", sr),
            EffectKind::AwToTape6 => Self::airwindows(kind, "ToTape6", sr),
            EffectKind::AwIronOxide5 => Self::airwindows(kind, "IronOxide5", sr),
            EffectKind::AwPurestDrive => Self::airwindows(kind, "PurestDrive", sr),
            EffectKind::AwDensity2 => Self::airwindows(kind, "Density2", sr),
            EffectKind::AwTube2 => Self::airwindows(kind, "Tube2", sr),
            EffectKind::AwMackity => Self::airwindows(kind, "Mackity", sr),
            EffectKind::AwPressure4 => Self::airwindows(kind, "Pressure4", sr),
            EffectKind::AwButterComp2 => Self::airwindows(kind, "ButterComp2", sr),
            EffectKind::AwAir3 => Self::airwindows(kind, "Air3", sr),
            EffectKind::AwCapacitor2 => Self::airwindows(kind, "Capacitor2", sr),
            EffectKind::AwDeRez2 => Self::airwindows(kind, "DeRez2", sr),
            EffectKind::AwTapeDelay2 => Self::airwindows(kind, "TapeDelay2", sr),
            EffectKind::Rings => EffectState::Rings(Box::new(super::rings_synth::RingsFx::new(sr))),
            EffectKind::Clouds => EffectState::Clouds(Box::new(super::fx::clouds::CloudsFx::new(sr))),
            EffectKind::Plugin => EffectState::Plugin { instance: 0, proc: None },
        }
    }

    fn airwindows(kind: EffectKind, id: &str, sr: f32) -> Self {
        match krabton_airwindows::create(id, sr) {
            Some(e) => EffectState::Airwindows(kind, e),
            None => EffectState::Utility,
        }
    }

    /// Stops a plug-in's processing (on the audio thread) before the state
    /// is handed away to be dropped.
    pub fn suspend(&mut self) {
        if let EffectState::Plugin { proc: Some(p), .. } = self {
            p.suspend();
        }
    }

    /// What the device sends to its display, if it has one.
    pub fn taps(&mut self) -> Option<&mut crate::scope::DeviceTaps> {
        match self {
            EffectState::Scope(t) => Some(t),
            EffectState::LensEq(s) => Some(&mut s.taps),
            EffectState::LensComp(s) => Some(&mut s.taps),
            EffectState::LensLimiter(s) => Some(&mut s.taps),
            _ => None,
        }
    }

    pub fn kind(&self) -> EffectKind {
        match self {
            EffectState::Eq3 { .. } => EffectKind::Eq3,
            EffectState::AutoFilter { .. } => EffectKind::AutoFilter,
            EffectState::Compressor { .. } => EffectKind::Compressor,
            EffectState::Saturator { .. } => EffectKind::Saturator,
            EffectState::Delay(_) => EffectKind::Delay,
            EffectState::Reverb(_) => EffectKind::Reverb,
            EffectState::Chorus(_) => EffectKind::Chorus,
            EffectState::Redux { .. } => EffectKind::Redux,
            EffectState::Utility => EffectKind::Utility,
            EffectState::Scope(_) => EffectKind::Scope,
            EffectState::LensEq(_) => EffectKind::LensEq,
            EffectState::LensComp(_) => EffectKind::LensComp,
            EffectState::LensLimiter(_) => EffectKind::LensLimiter,
            EffectState::Phaser(_) => EffectKind::Phaser,
            EffectState::Flanger(_) => EffectKind::Flanger,
            EffectState::AutoPan(_) => EffectKind::AutoPan,
            EffectState::FreqShifter(_) => EffectKind::FreqShifter,
            EffectState::Rotary(_) => EffectKind::Rotary,
            EffectState::Gate(_) => EffectKind::Gate,
            EffectState::Transient(_) => EffectKind::Transient,
            EffectState::Multiband(_) => EffectKind::Multiband,
            EffectState::Echo(_) => EffectKind::Echo,
            EffectState::BeatRepeat(_) => EffectKind::BeatRepeat,
            EffectState::Shifter(_) => EffectKind::Shifter,
            EffectState::AutoWah(_) => EffectKind::AutoWah,
            EffectState::Resonators(_) => EffectKind::Resonators,
            EffectState::Vinyl(_) => EffectKind::Vinyl,
            EffectState::Airwindows(kind, _) => *kind,
            EffectState::Rings(_) => EffectKind::Rings,
            EffectState::Clouds(_) => EffectKind::Clouds,
            EffectState::Plugin { .. } => EffectKind::Plugin,
        }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = ctx.sr;
        match self {
            EffectState::Eq3 { lo, hi } => {
                let g = |db: f32| if db <= -39.9 { 0.0 } else { db_to_gain(db) };
                let (gl, gm, gh) = (g(p[0]), g(p[1]), g(p[2]));
                for ch in 0..2 {
                    lo[ch].set_q(p[3], 0.707, sr);
                    hi[ch].set_q(p[4], 0.707, sr);
                }
                for i in 0..n {
                    for (ch, x) in [&mut l[i], &mut r[i]].into_iter().enumerate() {
                        let low = lo[ch].tick(*x).low;
                        let rest = *x - low;
                        let high = hi[ch].tick(rest).high;
                        let mid = rest - high;
                        *x = low * gl + mid * gm + high * gh;
                    }
                }
            }
            EffectState::AutoFilter { f, phase } => {
                let mode = FilterMode::from_index(p[0]);
                let cutoff = p[1];
                let res = p[2];
                let drive = p[3];
                let lfo_amt = p[4] * 4.0;
                let period = LFO_SYNC_BEATS[(p[5].round() as usize).min(LFO_SYNC_BEATS.len() - 1)];
                let lfo_hz = ctx.bpm / 60.0 / period;
                let mix = p[6];
                let drive_in = 1.0 + drive * 6.0;
                let mut i = 0;
                while i < n {
                    let end = (i + 16).min(n);
                    let lfo = (*phase * TAU).sin();
                    *phase = wrap_phase(*phase + lfo_hz * (end - i) as f32 / sr);
                    let fc = cutoff * 2f32.powf(lfo * lfo_amt);
                    f[0].set(fc, res, sr);
                    f[1].set(fc, res, sr);
                    for j in i..end {
                        let (dl, dr) = (l[j], r[j]);
                        let (xl, xr) = if drive > 0.001 { (fast_tanh(dl * drive_in), fast_tanh(dr * drive_in)) } else { (dl, dr) };
                        let yl = f[0].process(xl, mode);
                        let yr = f[1].process(xr, mode);
                        l[j] = dl * (1.0 - mix) + yl * mix;
                        r[j] = dr * (1.0 - mix) + yr * mix;
                    }
                    i = end;
                }
            }
            EffectState::Compressor { env_db } => {
                let thr = p[0];
                let ratio = p[1].max(1.0);
                let att = (-1.0 / (p[2].max(0.05) * 0.001 * sr)).exp();
                let rel = (-1.0 / (p[3].max(1.0) * 0.001 * sr)).exp();
                let makeup = p[4];
                let mix = p[5];
                let slope = 1.0 - 1.0 / ratio;
                for i in 0..n {
                    let peak = l[i].abs().max(r[i].abs());
                    let over = gain_to_db(peak) - thr;
                    let target = if over > 0.0 { over * slope } else { 0.0 };
                    let c = if target > *env_db { att } else { rel };
                    *env_db = target + (*env_db - target) * c;
                    let g = db_to_gain(makeup - *env_db);
                    l[i] = l[i] * (1.0 - mix) + l[i] * g * mix;
                    r[i] = r[i] * (1.0 - mix) + r[i] * g * mix;
                }
            }
            EffectState::Saturator { color, dc } => {
                let drive = db_to_gain(p[0]);
                let kind = p[1].round() as i32;
                let out = db_to_gain(p[3]);
                let mix = p[4];
                let tone_on = p[2] < 19_000.0;
                for c in color.iter_mut() {
                    c.set(p[2], sr);
                }
                for i in 0..n {
                    for (ch, x) in [&mut l[i], &mut r[i]].into_iter().enumerate() {
                        let dry = *x;
                        let v = dry * drive;
                        let mut y = match kind {
                            1 => v.clamp(-1.0, 1.0),
                            2 => {
                                // Foldback distortion.
                                let mut f = v;
                                for _ in 0..4 {
                                    if f > 1.0 {
                                        f = 2.0 - f;
                                    } else if f < -1.0 {
                                        f = -2.0 - f;
                                    } else {
                                        break;
                                    }
                                }
                                f.clamp(-1.0, 1.0)
                            }
                            3 => dc[ch].tick(fast_tanh(v + 0.3) - 0.291),
                            _ => fast_tanh(v),
                        };
                        if tone_on {
                            y = color[ch].lp(y);
                        }
                        *x = dry * (1.0 - mix) + y * out * mix;
                    }
                }
            }
            EffectState::Delay(d) => {
                let beats = SYNC_BEATS[(p[0].round() as usize).min(SYNC_BEATS.len() - 1)];
                let len = d.buf[0].len();
                let target = (beats * 60.0 / ctx.bpm.max(1.0) * sr).clamp(1.0, (len - 2) as f32);
                if d.cur < 0.0 {
                    d.cur = target;
                }
                let fb = p[1];
                let ping = p[2] >= 0.5;
                for c in d.lc.iter_mut() {
                    c.set(p[3], sr);
                }
                for c in d.hc.iter_mut() {
                    c.set(p[4], sr);
                }
                let mix = p[5];
                for i in 0..n {
                    d.cur += (target - d.cur) * 0.0004;
                    let pos = d.idx as f32 + len as f32 - d.cur;
                    let i0 = pos as usize % len;
                    let i1 = (i0 + 1) % len;
                    let frac = pos - pos.floor();
                    let mut rl = d.buf[0][i0] + (d.buf[0][i1] - d.buf[0][i0]) * frac;
                    let mut rr = d.buf[1][i0] + (d.buf[1][i1] - d.buf[1][i0]) * frac;
                    rl = d.hc[0].lp(d.lc[0].hp(rl));
                    rr = d.hc[1].lp(d.lc[1].hp(rr));
                    let (xl, xr) = (l[i], r[i]);
                    let (wl, wr) = if ping { ((xl + xr) * 0.5 + rr * fb, rl * fb) } else { (xl + rl * fb, xr + rr * fb) };
                    d.buf[0][d.idx] = fast_tanh(wl);
                    d.buf[1][d.idx] = fast_tanh(wr);
                    d.idx = (d.idx + 1) % len;
                    l[i] = xl * (1.0 - mix) + rl * mix;
                    r[i] = xr * (1.0 - mix) + rr * mix;
                }
            }
            EffectState::Reverb(rv) => rv.process(p[0], p[1], p[2], p[3], p[4], p[5], &mut l[..n], &mut r[..n]),
            EffectState::Chorus(c) => {
                let rate = p[0];
                let depth = p[1] * 0.006 * sr;
                let base = p[2] * 0.001 * sr;
                let fb = p[3];
                let mix = p[4];
                let len = c.buf[0].len();
                for i in 0..n {
                    c.phase = wrap_phase(c.phase + rate / sr);
                    let mods = [(c.phase * TAU).sin(), ((c.phase + 0.25) * TAU).sin()];
                    let xs = [l[i], r[i]];
                    let mut ys = [0.0f32; 2];
                    for ch in 0..2 {
                        let d = (base + depth * (0.5 + 0.5 * mods[ch])).clamp(1.0, (len - 2) as f32);
                        let pos = c.idx as f32 + len as f32 - d;
                        let i0 = pos as usize % len;
                        let i1 = (i0 + 1) % len;
                        let frac = pos - pos.floor();
                        ys[ch] = c.buf[ch][i0] + (c.buf[ch][i1] - c.buf[ch][i0]) * frac;
                        c.buf[ch][c.idx] = xs[ch] + ys[ch] * fb;
                    }
                    c.idx = (c.idx + 1) % len;
                    l[i] = xs[0] * (1.0 - mix * 0.5) + ys[0] * mix;
                    r[i] = xs[1] * (1.0 - mix * 0.5) + ys[1] * mix;
                }
            }
            EffectState::Redux { hold, counter } => {
                let steps = 2f32.powf(p[0] - 1.0);
                let factor = p[1].max(1.0) as u32;
                let mix = p[2];
                for i in 0..n {
                    if *counter == 0 {
                        hold[0] = (l[i] * steps).round() / steps;
                        hold[1] = (r[i] * steps).round() / steps;
                    }
                    *counter = (*counter + 1) % factor;
                    l[i] = l[i] * (1.0 - mix) + hold[0] * mix;
                    r[i] = r[i] * (1.0 - mix) + hold[1] * mix;
                }
            }
            EffectState::Utility => {
                let gain = if p[0] <= -59.9 { 0.0 } else { db_to_gain(p[0]) };
                let width = p[1];
                let (pl, pr) = pan_gains(p[2]);
                let il = if p[3] >= 0.5 { -1.0 } else { 1.0 };
                let ir = if p[4] >= 0.5 { -1.0 } else { 1.0 };
                for i in 0..n {
                    let (a, b) = (l[i] * il, r[i] * ir);
                    let mid = (a + b) * 0.5;
                    let side = (a - b) * 0.5 * width;
                    l[i] = (mid + side) * gain * pl;
                    r[i] = (mid - side) * gain * pr;
                }
            }
            EffectState::Scope(_) => {}
            EffectState::LensEq(s) => s.process(p, l, r, sr),
            EffectState::LensComp(s) => s.process(p, l, r, sr),
            EffectState::LensLimiter(s) => s.process(p, l, r, sr),
            EffectState::Phaser(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::Flanger(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::AutoPan(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::FreqShifter(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::Rotary(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::Gate(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::Transient(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::Multiband(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::Echo(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::BeatRepeat(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::Shifter(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::AutoWah(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::Resonators(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::Vinyl(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::Airwindows(_, e) => e.process(p, &mut l[..n], &mut r[..n]),
            EffectState::Rings(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::Clouds(s) => s.process(p, &mut l[..n], &mut r[..n], ctx),
            EffectState::Plugin { proc, .. } => {
                if let Some(pr) = proc {
                    pr.process_effect(&mut l[..n], &mut r[..n]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_effects_stay_finite() {
        let ctx = FxCtx { sr: 48000.0, bpm: 120.0, beat: 0.0, playing: false };
        for kind in EffectKind::ALL {
            let p = crate::params::defaults(kind.specs());
            let mut st = EffectState::new(kind, 48000.0);
            assert_eq!(st.kind(), kind);
            let mut rng = Rng::new(3);
            for _ in 0..50 {
                let mut l: Vec<f32> = (0..256).map(|_| rng.bipolar() * 0.5).collect();
                let mut r = l.clone();
                st.process(&p, &mut l, &mut r, &ctx);
                assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 50.0), "{kind:?}");
            }
        }
    }

    #[test]
    fn eq3_flat_is_transparent() {
        let ctx = FxCtx { sr: 48000.0, bpm: 120.0, beat: 0.0, playing: false };
        let p = crate::params::defaults(EffectKind::Eq3.specs());
        let mut st = EffectState::new(EffectKind::Eq3, 48000.0);
        let mut rng = Rng::new(9);
        let src: Vec<f32> = (0..512).map(|_| rng.bipolar()).collect();
        let mut l = src.clone();
        let mut r = src.clone();
        st.process(&p, &mut l, &mut r, &ctx);
        for (a, b) in l.iter().zip(&src) {
            assert!((a - b).abs() < 1e-4);
        }
    }
}
