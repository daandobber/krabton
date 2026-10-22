//! Presets for the audio effects, the MIDI effects and the sequencers:
//! named settings, each a few parameters (by name) changed from their
//! defaults, choices by their label. Some sequencers' presets bring a
//! pattern too (beats, chord progressions, lines).

use crate::classic_seq::{DRUM_BYTES, DRUM_LANES, DRUM_NOTES, PROG_BYTES, PROG_SLOTS};
use crate::dsp::effects::EffectKind;
use crate::midi_fx::MidiFxKind;
use crate::params::{ParamKind, ParamSpec};

/// A value: a number, or a choice by its label.
#[derive(Clone, Copy, Debug)]
pub enum V {
    N(f32),
    C(&'static str),
}

use V::{C, N};

pub struct Preset {
    pub name: &'static str,
    pub set: &'static [(&'static str, V)],
    /// A pattern that comes with it (sequencers).
    pub data: Option<fn() -> Vec<u8>>,
}

pub(crate) const fn p(name: &'static str, set: &'static [(&'static str, V)]) -> Preset {
    Preset { name, set, data: None }
}

pub(crate) const fn pd(name: &'static str, set: &'static [(&'static str, V)], data: fn() -> Vec<u8>) -> Preset {
    Preset { name, set, data: Some(data) }
}

/// The parameters of a preset: the defaults, with its values set.
pub fn apply(specs: &[ParamSpec], preset: &Preset) -> Vec<f32> {
    let mut params = crate::params::defaults(specs);
    for (name, v) in preset.set {
        let Some(i) = specs.iter().position(|s| s.name == *name) else { continue };
        let s = &specs[i];
        params[i] = match (*v, s.kind) {
            (C(label), ParamKind::Choice(opts)) => opts.iter().position(|o| *o == label).map(|k| k as f32).unwrap_or(params[i]),
            (N(x), _) => x.clamp(s.min.min(s.max), s.max.max(s.min)),
            (C(_), _) => params[i],
        };
    }
    params
}

/// A preset over what a device has now: the Keys switches (Transpose,
/// Hold, Band) stay as they are unless the preset sets them.
pub fn apply_to(specs: &[ParamSpec], preset: &Preset, current: &[f32]) -> Vec<f32> {
    let mut params = apply(specs, preset);
    for (i, s) in specs.iter().enumerate() {
        if s.group == "Keys" && !preset.set.iter().any(|(n, _)| *n == s.name) {
            if let (Some(v), Some(c)) = (params.get_mut(i), current.get(i)) {
                *v = *c;
            }
        }
    }
    params
}

pub fn effect(kind: EffectKind) -> &'static [Preset] {
    use EffectKind as E;
    match kind {
        E::Eq3 => EQ3,
        E::AutoFilter => AUTO_FILTER,
        E::Compressor => COMPRESSOR,
        E::Saturator => SATURATOR,
        E::Delay => DELAY,
        E::Reverb => REVERB,
        E::Chorus => CHORUS,
        E::Redux => REDUX,
        E::Utility => UTILITY,
        E::LensComp => LENS_COMP,
        E::LensLimiter => LENS_LIMITER,
        E::Phaser => PHASER,
        E::Flanger => FLANGER,
        E::AutoPan => AUTO_PAN,
        E::FreqShifter => FREQ_SHIFTER,
        E::Rotary => ROTARY,
        E::Gate => GATE,
        E::Transient => TRANSIENT,
        E::Multiband => MULTIBAND,
        E::Echo => ECHO,
        E::BeatRepeat => BEAT_REPEAT,
        E::Shifter => SHIFTER,
        E::AutoWah => AUTO_WAH,
        E::Resonators => RESONATORS,
        E::Vinyl => VINYL,
        E::Rings => RINGS,
        E::Clouds => CLOUDS,
        E::Beads => BEADS,
        E::Plate => PLATE,
        E::Hall => HALL,
        E::Shimmer => SHIMMER,
        E::Spring => SPRING,
        E::OpenAir => OPEN_AIR,
        E::Stage => STAGE,
        E::PitchDelay => PITCH_DELAY,
        E::Grains => GRAINS,
        E::Overdrive => OVERDRIVE,
        E::DistortionPedal => DISTORTION,
        E::Fuzz => FUZZ,
        E::Octaver => OCTAVER,
        E::Squash => SQUASH,
        E::Wah => WAH,
        E::Tremolo => TREMOLO,
        E::Vibe => VIBE,
        E::Amp => AMP,
        E::Comb => COMB,
        E::Lattice => LATTICE,
        E::Telephone => TELEPHONE,
        E::Warps => WARPS_PRESETS,
        E::VaFilter => VAFILTER_PRESETS,
        E::Slam => SLAM,
        E::Moo => MOO,
        E::AwGalactic => AW_GALACTIC,
        E::AwVerbity2 => AW_VERBITY,
        E::AwChamber => AW_CHAMBER,
        E::AwToTape6 => AW_TO_TAPE,
        E::AwIronOxide5 => AW_IRON_OXIDE,
        E::AwPurestDrive => AW_PUREST_DRIVE,
        E::AwDensity2 => AW_DENSITY,
        E::AwTube2 => AW_TUBE,
        E::AwMackity => AW_MACKITY,
        E::AwPressure4 => AW_PRESSURE,
        E::AwButterComp2 => AW_BUTTER_COMP,
        E::AwAir3 => AW_AIR,
        E::AwCapacitor2 => AW_CAPACITOR,
        E::AwDeRez2 => AW_DEREZ,
        E::Sky => SKY,
        E::Massive => MASSIVE,
        E::Smear => SMEAR,
        E::Convolution => CONVOLUTION,
        E::Snowglobe => SNOWGLOBE,
        E::AwTapeDelay2 => AW_TAPE_DELAY,
        E::AwPear2 => AW_PEAR2_PRESETS,
        E::AwIsolator3 => AW_ISOLATOR3_PRESETS,
        E::AwHull2 => AW_HULL2_PRESETS,
        E::AwBaxandall2 => AW_BAXANDALL2_PRESETS,
        E::AwCabs => AW_CABS_PRESETS,
        E::AwDistance2 => AW_DISTANCE2_PRESETS,
        E::AwCoils2 => AW_COILS2_PRESETS,
        E::AwFracture2 => AW_FRACTURE2_PRESETS,
        E::AwEnsemble => AW_ENSEMBLE_PRESETS,
        E::AwInfinity2 => AW_INFINITY2_PRESETS,
        _ => &[],
    }
}

pub fn midi(kind: MidiFxKind) -> &'static [Preset] {
    use MidiFxKind as K;
    match kind {
        K::Arpeggiator => ARPEGGIATOR,
        K::Chord => CHORD,
        K::NoteLength => NOTE_LENGTH,
        K::Pitch => PITCH,
        K::Scale => SCALE,
        K::Random => RANDOM,
        K::Velocity => VELOCITY,
        K::NoteEcho => NOTE_ECHO,
        K::Strum => STRUM,
        K::NoteRepeat => NOTE_REPEAT,
        K::Humanize => HUMANIZE,
        K::NoteFilter => NOTE_FILTER,
        K::Mono => MONO,
        K::Quantize => QUANTIZE,
        K::Step => STEP,
        K::Acid => ACID,
        K::DrumGrid => DRUM_GRID,
        K::Progression => PROGRESSION,
        K::Polymeter => POLYMETER,
        K::Euclid => EUCLID,
        K::Turing => TURING,
        K::Life => LIFE,
        K::Bounce => BOUNCE,
        K::Drunk => DRUNK,
        K::Orbits => ORBITS,
        K::Chaos => CHAOS,
        K::Automaton => AUTOMATON,
        K::Ant => ANT,
        K::Pong => PONG,
        K::Markov => MARKOV,
        K::Rain => RAIN,
        K::Words => WORDS,
        K::Sing => SING,
        K::Touch => TOUCH,
        K::Lorenz => LORENZ,
        K::Loops => LOOPS,
        K::Bloom => BLOOM,
        K::Chimes => CHIMES,
        K::DriftChords => DRIFT_CHORDS,
        K::Tintinnabuli => TINTINNABULI,
        K::Pedal => PEDAL,
        K::Glass => GLASS,
        K::Satie => SATIE,
        K::Riley => RILEY,
        K::Reich => REICH,
        K::Bach => BACH,
        K::Nancarrow => NANCARROW,
        K::Debussy => DEBUSSY,
        K::Feldman => FELDMAN,
        K::Mozart => MOZART,
        K::Chopin => CHOPIN,
        K::Ravel => RAVEL,
        K::Bartok => BARTOK,
        K::Messiaen => MESSIAEN,
        K::Ligeti => LIGETI,
        K::Cage => CAGE,
        K::Einaudi => EINAUDI,
        K::OrganGrinder => ORGAN_GRINDER,
        K::MusicBox => MUSIC_BOX,
        K::GuitarChords => GUITAR_CHORDS,
        K::Ukulele => UKULELE,
        K::Drummer => DRUMMER,
        K::DrumMap => DRUM_MAP,
        K::Marbles => MARBLES,
        K::Chance => CHANCE,
        K::Bits => BITS,
        K::Percussion => PERCUSSION,
        K::Spinner => SPINNER,
        K::Endless => ENDLESS,
        K::Hands => HANDS,
        K::Draw => DRAW,
        _ => crate::sequencers::players::presets(kind),
    }
}

// ---------------------------------------------------------------- patterns

/// Chord slots (degree, quality, neck or inversion), the rest silent.
pub fn prog(chords: &[(u8, u8, u8)]) -> Vec<u8> {
    let mut d = vec![0u8; PROG_SLOTS * PROG_BYTES];
    for (i, c) in chords.iter().take(PROG_SLOTS).enumerate() {
        d[i * PROG_BYTES..i * PROG_BYTES + 3].copy_from_slice(&[c.0, c.1, c.2]);
    }
    d
}

/// Chords and a strumming pattern, by the pattern's name.
fn strum(chords: &[(u8, u8, u8)], pattern: &str) -> Vec<u8> {
    let mut d = prog(chords);
    let pat = crate::sequencers::strum::PATTERNS.iter().find(|p| p.0 == pattern).map(|p| p.1).unwrap_or(crate::sequencers::strum::PATTERNS[0].1);
    d.extend_from_slice(&pat);
    d
}

/// A drum grid from strings of 16 steps per lane (kick, snare, clap, closed
/// hat, open hat, low tom, rim, cowbell): x a hit, X an accent.
fn beat(lanes: [&str; DRUM_LANES]) -> Vec<u8> {
    let mut d = vec![0u8; DRUM_BYTES];
    for (l, row) in lanes.iter().enumerate() {
        for (s, c) in row.bytes().take(16).enumerate() {
            d[l * 16 + s] = match c {
                b'x' => 1,
                b'X' => 2,
                _ => 0,
            };
        }
    }
    d[DRUM_LANES * 16..].copy_from_slice(&DRUM_NOTES);
    d
}

fn named_beat(name: &str) -> Vec<u8> {
    let mut d = crate::classic_seq::drum_default();
    crate::classic_seq::drum_pattern(name, &mut d);
    d
}

/// An acid line: (gate 0 rest / 1 note / 2 tie, semitone, octave 0..2, accent, slide) per step.
fn acid(line: [(u8, u8, u8, u8, u8); 16]) -> Vec<u8> {
    line.iter().flat_map(|s| [s.0, s.1, s.2, s.3, s.4]).collect()
}

/// A step melody: scale degrees (None a rest), length 3, accents on the beats.
fn melody(degrees: [Option<u8>; 16]) -> Vec<u8> {
    degrees
        .iter()
        .enumerate()
        .flat_map(|(i, d)| [d.is_some() as u8, d.unwrap_or(0), if i % 4 == 0 { 112 } else { 90 }, 3, 1])
        .collect()
}

/// A Draw line from a function of 0..1.
fn line(f: fn(f32) -> f32) -> Vec<u8> {
    (0..crate::sequencers::played::DRAW_POINTS).map(|i| (f(i as f32 / (crate::sequencers::played::DRAW_POINTS - 1) as f32).clamp(0.0, 1.0) * 255.0) as u8).collect()
}

// ---------------------------------------------------------------- audio effects

static EQ3: &[Preset] = &[
    p("Warm", &[("Low", N(3.0)), ("High", N(-3.0))]),
    p("Bright", &[("Low", N(-2.0)), ("High", N(4.0)), ("Freq Hi", N(4000.0))]),
    p("Mid Scoop", &[("Low", N(2.0)), ("Mid", N(-6.0)), ("High", N(2.0))]),
    p("Kill the Lows", &[("Low", N(-40.0)), ("Freq Lo", N(300.0))]),
    p("Kill the Highs", &[("High", N(-40.0)), ("Freq Hi", N(2000.0))]),
    p("Mids Only", &[("Low", N(-40.0)), ("High", N(-40.0)), ("Mid", N(4.0))]),
];

static AUTO_FILTER: &[Preset] = &[
    p("Slow Sweep", &[("Type", C("Lowpass")), ("Cutoff", N(800.0)), ("Resonance", N(0.4)), ("LFO Amt", N(0.6)), ("LFO Rate", C("2 Bars"))]),
    p("Wobble", &[("Type", C("Lowpass")), ("Cutoff", N(400.0)), ("Resonance", N(0.65)), ("Drive", N(0.3)), ("LFO Amt", N(0.8)), ("LFO Rate", C("1/8"))]),
    p("Highpass Riser", &[("Type", C("Highpass")), ("Cutoff", N(300.0)), ("Resonance", N(0.3)), ("LFO Amt", N(0.5)), ("LFO Rate", C("8 Bars"))]),
    p("Talking Band", &[("Type", C("Bandpass")), ("Cutoff", N(1200.0)), ("Resonance", N(0.7)), ("LFO Amt", N(0.5)), ("LFO Rate", C("1/4"))]),
    p("Dark", &[("Type", C("Lowpass")), ("Cutoff", N(1500.0)), ("Resonance", N(0.1))]),
    p("Moving Notch", &[("Type", C("Notch")), ("Cutoff", N(1000.0)), ("Resonance", N(0.5)), ("LFO Amt", N(0.7)), ("LFO Rate", C("1 Bar"))]),
];

static COMPRESSOR: &[Preset] = &[
    p("Gentle Glue", &[("Threshold", N(-12.0)), ("Ratio", N(2.0)), ("Attack", N(30.0)), ("Release", N(200.0)), ("Makeup", N(2.0))]),
    p("Vocal", &[("Threshold", N(-20.0)), ("Ratio", N(4.0)), ("Attack", N(5.0)), ("Release", N(120.0)), ("Makeup", N(5.0))]),
    p("Drum Smash", &[("Threshold", N(-30.0)), ("Ratio", N(10.0)), ("Attack", N(1.0)), ("Release", N(60.0)), ("Makeup", N(10.0)), ("Mix", N(0.5))]),
    p("Even Bass", &[("Threshold", N(-24.0)), ("Ratio", N(6.0)), ("Attack", N(10.0)), ("Release", N(150.0)), ("Makeup", N(6.0))]),
    p("Brickwall", &[("Threshold", N(-6.0)), ("Ratio", N(20.0)), ("Attack", N(0.1)), ("Release", N(50.0)), ("Makeup", N(4.0))]),
];

static SATURATOR: &[Preset] = &[
    p("Warm Tube", &[("Drive", N(8.0)), ("Type", C("Tube")), ("Color", N(12000.0)), ("Output", N(-3.0))]),
    p("Tape Glow", &[("Drive", N(4.0)), ("Type", C("Soft")), ("Color", N(9000.0))]),
    p("Hard Clip", &[("Drive", N(18.0)), ("Type", C("Hard")), ("Output", N(-8.0))]),
    p("Wavefolder", &[("Drive", N(20.0)), ("Type", C("Fold")), ("Color", N(8000.0)), ("Output", N(-6.0)), ("Mix", N(0.7))]),
    p("Just a Touch", &[("Drive", N(3.0)), ("Type", C("Soft")), ("Mix", N(0.5))]),
];

static DELAY: &[Preset] = &[
    p("Slapback", &[("Time", C("1/16")), ("Feedback", N(0.1)), ("Ping Pong", N(0.0)), ("Mix", N(0.3))]),
    p("Dotted Eighth", &[("Time", C("1/8 D")), ("Feedback", N(0.45)), ("Ping Pong", N(1.0)), ("Mix", N(0.3))]),
    p("Quarter Echo", &[("Time", C("1/4")), ("Feedback", N(0.35)), ("Ping Pong", N(0.0)), ("High Cut", N(5000.0)), ("Mix", N(0.25))]),
    p("Ping Pong Eighths", &[("Time", C("1/8")), ("Feedback", N(0.5)), ("Ping Pong", N(1.0)), ("Mix", N(0.35))]),
    p("Dub Space", &[("Time", C("1/4 D")), ("Feedback", N(0.7)), ("Low Cut", N(400.0)), ("High Cut", N(2500.0)), ("Mix", N(0.4))]),
    p("Long Wash", &[("Time", C("1/2")), ("Feedback", N(0.8)), ("High Cut", N(3000.0)), ("Mix", N(0.3))]),
];

static REVERB: &[Preset] = &[
    p("Small Room", &[("Size", N(0.3)), ("Damping", N(0.6)), ("Predelay", N(5.0)), ("Mix", N(0.2))]),
    p("Big Hall", &[("Size", N(0.9)), ("Damping", N(0.4)), ("Predelay", N(30.0)), ("Mix", N(0.35))]),
    p("Dark Chamber", &[("Size", N(0.6)), ("Damping", N(0.85)), ("Predelay", N(15.0)), ("Mix", N(0.3))]),
    p("Bright Space", &[("Size", N(0.55)), ("Damping", N(0.15)), ("Predelay", N(10.0)), ("Low Cut", N(300.0)), ("Mix", N(0.25))]),
    p("Ambient Wash", &[("Size", N(1.0)), ("Damping", N(0.5)), ("Predelay", N(60.0)), ("Mix", N(0.6))]),
];

static CHORUS: &[Preset] = &[
    p("Classic Chorus", &[("Rate", N(0.8)), ("Depth", N(0.5)), ("Delay", N(8.0)), ("Mix", N(0.5))]),
    p("Slow Ensemble", &[("Rate", N(0.2)), ("Depth", N(0.7)), ("Delay", N(15.0)), ("Mix", N(0.5))]),
    p("Vibrato", &[("Rate", N(5.0)), ("Depth", N(0.4)), ("Delay", N(4.0)), ("Mix", N(1.0))]),
    p("Doubler", &[("Rate", N(0.1)), ("Depth", N(0.2)), ("Delay", N(25.0)), ("Mix", N(0.4))]),
    p("Almost a Flanger", &[("Rate", N(0.3)), ("Depth", N(0.8)), ("Delay", N(2.0)), ("Feedback", N(0.7)), ("Mix", N(0.5))]),
];

static REDUX: &[Preset] = &[
    p("8-Bit", &[("Bits", N(8.0)), ("Downsample", N(4.0))]),
    p("Old Sampler", &[("Bits", N(12.0)), ("Downsample", N(3.0))]),
    p("Crushed", &[("Bits", N(4.0)), ("Downsample", N(10.0))]),
    p("Radio Ghost", &[("Bits", N(6.0)), ("Downsample", N(20.0)), ("Mix", N(0.6))]),
];

static UTILITY: &[Preset] = &[
    p("Mono", &[("Width", N(0.0))]),
    p("Wide", &[("Width", N(1.6))]),
    p("Minus 6 dB", &[("Gain", N(-6.0))]),
    p("Phase Flip", &[("Phase L", N(1.0)), ("Phase R", N(1.0))]),
];

static LENS_COMP: &[Preset] = &[
    p("Mix Bus Glue", &[("Style", C("Bus")), ("Threshold", N(-18.0)), ("Ratio", N(2.0)), ("Attack", N(30.0)), ("Release", N(300.0)), ("Auto Release", N(1.0)), ("Auto Gain", N(1.0))]),
    p("Vocal Leveler", &[("Style", C("Vocal")), ("Threshold", N(-24.0)), ("Ratio", N(4.0)), ("Attack", N(5.0)), ("Release", N(150.0)), ("Auto Gain", N(1.0))]),
    p("Smooth Opto", &[("Style", C("Opto")), ("Threshold", N(-20.0)), ("Ratio", N(3.0)), ("Auto Gain", N(1.0))]),
    p("Drum Punch", &[("Style", C("Punch")), ("Threshold", N(-24.0)), ("Ratio", N(6.0)), ("Attack", N(15.0)), ("Release", N(80.0)), ("Mix", N(0.7))]),
    p("Pumping", &[("Style", C("Pumping")), ("Threshold", N(-30.0)), ("Ratio", N(10.0)), ("Attack", N(1.0)), ("Release", N(250.0))]),
    p("Mastering", &[("Style", C("Mastering")), ("Threshold", N(-10.0)), ("Ratio", N(1.5)), ("Knee", N(12.0)), ("Auto Gain", N(1.0))]),
];

static LENS_LIMITER: &[Preset] = &[
    p("Transparent Master", &[("Style", C("Transparent")), ("Gain", N(3.0)), ("Ceiling", N(-1.0))]),
    p("Loud and Modern", &[("Style", C("Modern")), ("Gain", N(8.0)), ("Ceiling", N(-1.0))]),
    p("Punchy Drums", &[("Style", C("Punchy")), ("Gain", N(6.0)), ("Ceiling", N(-0.5))]),
    p("Just Catch the Peaks", &[("Style", C("Safe")), ("Gain", N(0.0)), ("Ceiling", N(-1.0))]),
];

static PHASER: &[Preset] = &[
    p("Slow Swirl", &[("Rate", N(0.2)), ("Stages", C("6")), ("Feedback", N(0.5)), ("Mix", N(0.5))]),
    p("Funky Quarter", &[("Sync", C("1/4")), ("Stages", C("4")), ("Feedback", N(0.3)), ("Center", N(1200.0))]),
    p("Jet", &[("Rate", N(0.1)), ("Stages", C("12")), ("Feedback", N(0.85)), ("Depth", N(1.0)), ("Mix", N(0.6))]),
    p("Wide Stereo", &[("Rate", N(0.4)), ("Stereo", N(1.0)), ("Stages", C("8")), ("Mix", N(0.5))]),
];

static FLANGER: &[Preset] = &[
    p("Classic Jet", &[("Rate", N(0.15)), ("Feedback", N(0.7)), ("Delay", N(2.0)), ("Mix", N(0.5))]),
    p("Metallic", &[("Rate", N(0.05)), ("Feedback", N(0.9)), ("Delay", N(1.0)), ("Mix", N(0.6))]),
    p("Synced Sweep", &[("Sync", C("1 Bar")), ("Feedback", N(0.6)), ("Mix", N(0.5))]),
    p("Subtle Shimmer", &[("Rate", N(0.3)), ("Depth", N(0.3)), ("Feedback", N(0.2)), ("Mix", N(0.3))]),
];

static AUTO_PAN: &[Preset] = &[
    p("Gentle Pan", &[("Amount", N(0.6)), ("Sync", C("1 Bar")), ("Shape", C("Sine"))]),
    p("Eighth Tremolo", &[("Amount", N(0.8)), ("Sync", C("1/8")), ("Shape", C("Square")), ("Phase", N(0.0))]),
    p("Choppy Gate", &[("Amount", N(1.0)), ("Sync", C("1/16")), ("Shape", C("Square")), ("Phase", N(0.0))]),
    p("Random Pan", &[("Amount", N(0.7)), ("Sync", C("1/8")), ("Shape", C("Random"))]),
];

static FREQ_SHIFTER: &[Preset] = &[
    p("Wide Detune", &[("Mode", C("Shift")), ("Coarse", N(0.0)), ("Fine", N(3.0)), ("Wide", N(1.0)), ("Mix", N(0.5))]),
    p("Barber Pole", &[("Mode", C("Shift")), ("Coarse", N(30.0)), ("Feedback", N(0.7)), ("Mix", N(0.6))]),
    p("Ring Bell", &[("Mode", C("Ring")), ("Coarse", N(440.0)), ("Mix", N(0.7))]),
    p("Robot", &[("Mode", C("Ring")), ("Coarse", N(80.0)), ("Mix", N(1.0))]),
    p("Down Spiral", &[("Mode", C("Shift")), ("Coarse", N(-50.0)), ("Feedback", N(0.75)), ("Mix", N(0.5))]),
];

static ROTARY: &[Preset] = &[
    p("Chorale", &[("Speed", C("Slow")), ("Drive", N(0.2))]),
    p("Fast Spin", &[("Speed", C("Fast")), ("Drive", N(0.3))]),
    p("Driven Fast", &[("Speed", C("Fast")), ("Drive", N(0.7))]),
    p("Stopped", &[("Speed", C("Stop"))]),
];

static GATE: &[Preset] = &[
    p("Clean Up", &[("Threshold", N(-50.0)), ("Release", N(150.0))]),
    p("Tight Drums", &[("Threshold", N(-30.0)), ("Hold", N(10.0)), ("Release", N(50.0))]),
    p("Gated Reverb", &[("Threshold", N(-20.0)), ("Hold", N(200.0)), ("Release", N(20.0))]),
    p("Soft Floor", &[("Threshold", N(-40.0)), ("Floor", N(-12.0))]),
];

static TRANSIENT: &[Preset] = &[
    p("More Snap", &[("Attack", N(0.6))]),
    p("Tight", &[("Attack", N(0.3)), ("Sustain", N(-0.6))]),
    p("Roomy", &[("Sustain", N(0.6))]),
    p("Soften", &[("Attack", N(-0.6))]),
];

static MULTIBAND: &[Preset] = &[
    p("Gentle", &[("Depth", N(0.3))]),
    p("Smashed", &[("Depth", N(1.0)), ("Time", N(0.3)), ("Output", N(2.0))]),
    p("Upward Only", &[("Downward", N(0.0)), ("Upward", N(1.0)), ("Depth", N(0.6))]),
    p("Bass in Check", &[("Low", N(-2.0)), ("Depth", N(0.5))]),
];

static ECHO: &[Preset] = &[
    p("Worn Tape", &[("Sync", C("1/4 D")), ("Feedback", N(0.5)), ("Wow", N(0.5)), ("Flutter", N(0.4)), ("Saturation", N(0.5)), ("Tone", N(3000.0))]),
    p("Dub Delay", &[("Sync", C("1/4")), ("Feedback", N(0.75)), ("Tone", N(2500.0)), ("Low Cut", N(300.0)), ("Mix", N(0.4))]),
    p("Runaway", &[("Feedback", N(1.05)), ("Tone", N(2000.0)), ("Mix", N(0.4))]),
    p("Clean Repeats", &[("Wow", N(0.0)), ("Flutter", N(0.0)), ("Saturation", N(0.0)), ("Tone", N(12000.0))]),
    p("Wide Slap", &[("Sync", C("Free")), ("Time", N(90.0)), ("Feedback", N(0.1)), ("Spread", N(1.0)), ("Mix", N(0.3))]),
];

static BEAT_REPEAT: &[Preset] = &[
    p("End of Bar Stutter", &[("Interval", C("1 Bar")), ("Offset", N(12.0)), ("Gate", N(4.0)), ("Grid", C("1/16")), ("Mode", C("Insert"))]),
    p("Glitch Rolls", &[("Interval", C("1/2")), ("Offset", N(4.0)), ("Gate", N(4.0)), ("Grid", C("1/32")), ("Variation", N(0.5)), ("Chance", N(0.5))]),
    p("Pitch Dive", &[("Interval", C("2 Bars")), ("Offset", N(8.0)), ("Gate", N(8.0)), ("Grid", C("1/8")), ("Pitch Decay", N(0.6)), ("Decay", N(0.4))]),
    p("Triplet Chop", &[("Interval", C("1 Bar")), ("Offset", N(8.0)), ("Gate", N(8.0)), ("Grid", C("1/16 T")), ("Chance", N(0.7))]),
];

static SHIFTER: &[Preset] = &[
    p("Octave Up", &[("Pitch", N(12.0)), ("Mix", N(0.5))]),
    p("Octave Down", &[("Pitch", N(-12.0)), ("Mix", N(0.5))]),
    p("Fifth Harmony", &[("Pitch", N(7.0)), ("Mix", N(0.4))]),
    p("Wide Doubler", &[("Pitch", N(0.0)), ("Fine", N(8.0)), ("Wide", N(20.0)), ("Mix", N(0.4))]),
    p("Climbing Octaves", &[("Pitch", N(12.0)), ("Feedback", N(0.5)), ("Mix", N(0.4))]),
];

static AUTO_WAH: &[Preset] = &[
    p("Funk Wah", &[("Sensitivity", N(0.7)), ("Attack", N(4.0)), ("Release", N(120.0)), ("Mode", C("Bandpass")), ("Resonance", N(0.7))]),
    p("Quack", &[("Sensitivity", N(0.9)), ("Resonance", N(0.85)), ("Low", N(400.0)), ("High", N(2500.0))]),
    p("Slow Bloom", &[("Sensitivity", N(0.5)), ("Attack", N(60.0)), ("Release", N(600.0)), ("Mode", C("Lowpass"))]),
    p("Backwards", &[("Sensitivity", N(-0.6))]),
];

static RESONATORS: &[Preset] = &[
    p("Major Seventh Shimmer", &[("Chord", C("Major 7")), ("Note", N(60.0)), ("Decay", N(2.0))]),
    p("Minor Drone", &[("Chord", C("Minor")), ("Note", N(48.0)), ("Decay", N(4.0)), ("Color", N(3000.0))]),
    p("Metal Fifths", &[("Chord", C("Fifths")), ("Note", N(36.0)), ("Decay", N(1.0)), ("Color", N(12000.0))]),
    p("Cluster Cloud", &[("Chord", C("Cluster")), ("Decay", N(3.0)), ("Mix", N(0.4))]),
];

static VINYL: &[Preset] = &[
    p("Dusty", &[("Crackle", N(0.4)), ("Pops", N(0.2)), ("Hiss", N(0.15)), ("Wear", N(0.4))]),
    p("Old 78", &[("Crackle", N(0.8)), ("Pops", N(0.5)), ("Hiss", N(0.4)), ("Wear", N(0.8)), ("Width", N(0.0))]),
    p("Gentle Warmth", &[("Crackle", N(0.1)), ("Pops", N(0.05)), ("Hiss", N(0.05)), ("Wear", N(0.2))]),
    p("Warped Record", &[("Wow", N(0.8)), ("Crackle", N(0.3))]),
];

static RINGS: &[Preset] = &[
    p("Plucked Body", &[("Model", C("Modal")), ("Structure", N(0.25)), ("Brightness", N(0.6)), ("Damping", N(0.5))]),
    p("Sympathetic Strings", &[("Model", C("Sympathetic")), ("Structure", N(0.5)), ("Damping", N(0.7))]),
    p("Bell Ring", &[("Model", C("Inharmonic")), ("Brightness", N(0.7)), ("Damping", N(0.75))]),
    p("String Wash", &[("Model", C("String + Reverb")), ("Damping", N(0.3)), ("Mix", N(0.6))]),
    p("Chord Resonance", &[("Model", C("Chords")), ("Mix", N(0.5))]),
];

static CLOUDS: &[Preset] = &[
    p("Grain Cloud", &[("Mode", C("Granular")), ("Density", N(0.7)), ("Texture", N(0.6)), ("Reverb", N(0.4))]),
    p("Time Stretch", &[("Mode", C("Stretch")), ("Size", N(0.7)), ("Dry/Wet", N(0.7))]),
    p("Looping Delay", &[("Mode", C("Looping Delay")), ("Feedback", N(0.5)), ("Dry/Wet", N(0.5))]),
    p("Spectral Haze", &[("Mode", C("Spectral")), ("Texture", N(0.7)), ("Reverb", N(0.5)), ("Dry/Wet", N(0.7))]),
    p("Octave Shimmer", &[("Pitch", N(12.0)), ("Reverb", N(0.6)), ("Feedback", N(0.3))]),
];

static BEADS: &[Preset] = &[
    p("Gentle Grains", &[("Density", N(0.6)), ("Size", N(0.6)), ("Reverb", N(0.3))]),
    p("Frozen Pad", &[("Size", N(0.8)), ("Density", N(0.9)), ("Reverb", N(0.6)), ("Dry/Wet", N(0.8))]),
    p("Scattered", &[("Size", N(0.2)), ("Time Random", N(0.7)), ("Pitch Random", N(0.3)), ("Density", N(0.8))]),
    p("Cassette Cloud", &[("Quality", C("Scorched Cassette")), ("Density", N(0.6)), ("Reverb", N(0.4))]),
];

static PLATE: &[Preset] = &[
    p("Vocal Plate", &[("Pre-Delay", N(20.0)), ("Decay", N(0.5)), ("Mix", N(0.25))]),
    p("Drum Plate", &[("Size", N(0.35)), ("Decay", N(0.35)), ("Mix", N(0.2))]),
    p("Long Plate", &[("Decay", N(0.85)), ("Mix", N(0.35))]),
    p("Dark Plate", &[("Damping", N(3000.0)), ("Decay", N(0.6))]),
];

static HALL: &[Preset] = &[
    p("Concert Hall", &[("Decay", N(2.5)), ("Size", N(0.7))]),
    p("Cathedral", &[("Decay", N(8.0)), ("Size", N(1.0)), ("Pre-Delay", N(40.0)), ("Mix", N(0.35))]),
    p("Small Hall", &[("Decay", N(1.2)), ("Size", N(0.4)), ("Mix", N(0.2))]),
    p("Endless", &[("Decay", N(30.0)), ("Size", N(1.0)), ("Mix", N(0.45))]),
];

static SHIMMER: &[Preset] = &[
    p("Octave Heaven", &[("Pitch", N(12.0)), ("Shimmer", N(0.6))]),
    p("Fifth Glow", &[("Pitch", N(7.0)), ("Shimmer", N(0.5))]),
    p("Dark Shimmer", &[("Pitch", N(-12.0)), ("Shimmer", N(0.5))]),
    p("Infinite", &[("Decay", N(40.0)), ("Mix", N(0.5)), ("Shimmer", N(0.7))]),
];

static SPRING: &[Preset] = &[
    p("Surf", &[("Drip", N(0.7)), ("Decay", N(0.6)), ("Mix", N(0.4))]),
    p("Small Tank", &[("Decay", N(0.3)), ("Mix", N(0.25))]),
    p("Dub Splash", &[("Drip", N(0.9)), ("Drive", N(0.5)), ("Mix", N(0.5))]),
    p("Clean Spring", &[("Drip", N(0.1)), ("Drive", N(0.0)), ("Tone", N(6000.0))]),
];

static STAGE: &[Preset] = &[
    p("Studio", &[]),
    p("Small Room", &[("Room m", N(6.0)), ("Walls", N(0.6)), ("Reverb", N(0.4)), ("Listener", N(0.7))]),
    p("Club", &[("Room m", N(14.0)), ("Walls", N(0.35)), ("Reverb", N(0.55))]),
    p("Concert Hall", &[("Room m", N(32.0)), ("Walls", N(0.25)), ("Reverb", N(0.65)), ("Listener", N(0.75))]),
    p("Cathedral", &[("Room m", N(56.0)), ("Walls", N(0.05)), ("Reverb", N(0.8)), ("Air", N(0.7))]),
    p("Dry Booth", &[("Room m", N(4.0)), ("Walls", N(1.0)), ("Reverb", N(0.15))]),
    p("Headphones", &[("Ears", C("Headphones"))]),
];
static OPEN_AIR: &[Preset] = &[
    p("Forest Morning", &[("Place", C("Forest")), ("Ambience", C("Birds"))]),
    p("Canyon Echo", &[("Place", C("Canyon")), ("Distance", N(0.6))]),
    p("Cave", &[("Place", C("Cave")), ("Size", N(0.8))]),
    p("Lake at Night", &[("Place", C("Lake")), ("Ambience", C("Night"))]),
];

static PITCH_DELAY: &[Preset] = &[
    p("Octave Climb", &[("Pitch", N(12.0)), ("Mode", C("Climb")), ("Feedback", N(0.6))]),
    p("Falling Fifths", &[("Pitch", N(-7.0)), ("Mode", C("Climb")), ("Feedback", N(0.6))]),
    p("Harmony Echo", &[("Pitch", N(7.0)), ("Mode", C("Fixed")), ("Feedback", N(0.4))]),
    p("Ping Pong Thirds", &[("Pitch", N(4.0)), ("Ping Pong", N(1.0)), ("Feedback", N(0.5))]),
];

static GRAINS: &[Preset] = &[
    p("Soft Cloud", &[("Size", N(0.6)), ("Density", N(0.4)), ("Reverb", N(0.4))]),
    p("Stutter Grains", &[("Size", N(0.1)), ("Density", N(0.8)), ("Time Rand", N(0.3))]),
    p("Pitch Scatter", &[("Pitch Rand", N(0.6)), ("Density", N(0.5)), ("Spread", N(0.9))]),
    p("Cassette Grains", &[("Quality", C("Cassette")), ("Size", N(0.5)), ("Density", N(0.5))]),
];

static OVERDRIVE: &[Preset] = &[
    p("Light Crunch", &[("Drive", N(0.3)), ("Tone", N(0.5))]),
    p("Blues", &[("Drive", N(0.55)), ("Tone", N(0.6))]),
    p("Pushed", &[("Drive", N(0.85)), ("Tone", N(0.55)), ("Level", N(-3.0))]),
];

static DISTORTION: &[Preset] = &[
    p("Rhythm", &[("Gain", N(0.5)), ("Filter", N(0.45))]),
    p("Lead", &[("Gain", N(0.85)), ("Filter", N(0.55)), ("Level", N(-3.0))]),
    p("Lopsided", &[("Gain", N(0.7)), ("Clip", C("Lopsided"))]),
    p("Soft Clipper", &[("Gain", N(0.4)), ("Clip", C("Soft"))]),
];

static FUZZ: &[Preset] = &[
    p("Vintage", &[("Type", C("Vintage")), ("Fuzz", N(0.7))]),
    p("Wall of Fuzz", &[("Type", C("Wall")), ("Fuzz", N(0.9)), ("Level", N(-4.0))]),
    p("Octave Fuzz", &[("Type", C("Octave Up")), ("Fuzz", N(0.75))]),
    p("Gated", &[("Fuzz", N(0.8)), ("Gate", N(0.5))]),
];

static OCTAVER: &[Preset] = &[
    p("Sub Bass", &[("Octave Down", N(1.0)), ("Dry", N(0.6))]),
    p("Organ", &[("Octave Up", N(0.6)), ("Octave Down", N(0.6)), ("Dry", N(0.7))]),
    p("Deep", &[("Two Down", N(0.7)), ("Octave Down", N(0.4))]),
];

static SQUASH: &[Preset] = &[
    p("Country Squash", &[("Sustain", N(0.6)), ("Attack", N(2.0))]),
    p("Long Sustain", &[("Sustain", N(0.9)), ("Blend", N(0.7))]),
    p("Subtle", &[("Sustain", N(0.3)), ("Blend", N(0.5))]),
];

static WAH: &[Preset] = &[
    p("Half Open", &[("Mode", C("Pedal")), ("Pedal", N(0.6))]),
    p("Auto Funk", &[("Mode", C("Auto")), ("Sensitivity", N(0.7))]),
    p("Synced Wobble", &[("Mode", C("Wobble")), ("Sync", C("1/8"))]),
    p("Slow Sweep", &[("Mode", C("Wobble")), ("Rate", N(0.3)), ("Resonance", N(0.7))]),
];

static TREMOLO: &[Preset] = &[
    p("Vintage Amp", &[("Rate", N(5.0)), ("Depth", N(0.5))]),
    p("Choppy Eighths", &[("Sync", C("1/8")), ("Shape", C("Choppy")), ("Depth", N(1.0))]),
    p("Harmonic", &[("Shape", C("Harmonic")), ("Rate", N(4.0))]),
    p("Stereo Swirl", &[("Stereo", N(1.0)), ("Rate", N(3.0)), ("Depth", N(0.7))]),
];

static VIBE: &[Preset] = &[
    p("Slow Throb", &[("Speed", N(1.0)), ("Intensity", N(0.7))]),
    p("Fast Vibrato", &[("Mode", C("Vibrato")), ("Speed", N(6.0))]),
    p("Synced Quarter", &[("Sync", C("1/4")), ("Intensity", N(0.8))]),
];

static AMP: &[Preset] = &[
    p("Jazz Clean", &[("Type", C("Clean")), ("Gain", N(0.3)), ("Treble", N(-2.0)), ("Cabinet", C("1x12 Open"))]),
    p("Bright Clean", &[("Type", C("Clean")), ("Treble", N(4.0)), ("Presence", N(0.7)), ("Cabinet", C("2x12"))]),
    p("Crunch", &[("Type", C("Crunch")), ("Gain", N(0.6)), ("Cabinet", C("2x12"))]),
    p("Heavy Lead", &[("Type", C("Lead")), ("Gain", N(0.9)), ("Mid", N(-4.0)), ("Cabinet", C("4x12 Closed"))]),
    p("Bass Amp", &[("Type", C("Clean")), ("Bass", N(6.0)), ("Treble", N(-4.0)), ("Cabinet", C("4x12 Closed"))]),
];

static COMB: &[Preset] = &[
    p("Plucked Metal", &[("Pitch", N(60.0)), ("Feedback", N(0.9)), ("Damp", N(0.2)), ("Mix", N(0.5))]),
    p("Hollow Tube", &[("Pitch", N(48.0)), ("Feedback", N(-0.8)), ("Mix", N(0.6))]),
    p("Drone Drums", &[("Pitch", N(36.0)), ("Feedback", N(0.97)), ("Damp", N(0.4)), ("Mix", N(0.4))]),
    p("Wobbly", &[("Pitch", N(72.0)), ("Wobble", N(0.6)), ("Feedback", N(0.6))]),
];

static LATTICE: &[Preset] = &[
    p("Small Room", &[("Size", N(0.01)), ("Feedback", N(0.6)), ("Shape", N(0.0)), ("Mix", N(0.3))]),
    p("Metal Box", &[("Size", N(0.008)), ("Feedback", N(0.9)), ("Shape", N(1.0)), ("Mix", N(0.4))]),
    p("Big Space", &[("Size", N(0.08)), ("Feedback", N(0.9)), ("Tone", N(0.5)), ("Mix", N(0.45))]),
    p("Frozen", &[("Size", N(0.05)), ("Feedback", N(1.0)), ("Mix", N(0.5))]),
];

static VAFILTER_PRESETS: &[Preset] = &[
    p("Warm Ladder", &[("Model", C("Ladder")), ("Cutoff", N(800.0)), ("Resonance", N(0.4)), ("Drive", N(0.3))]),
    p("Acid Diode", &[("Model", C("Diode Ladder")), ("Cutoff", N(400.0)), ("Resonance", N(0.8)), ("Drive", N(0.5)), ("Env Amount", N(0.7)), ("Env Release", N(120.0))]),
    p("Scream", &[("Model", C("Scream LP")), ("Cutoff", N(900.0)), ("Resonance", N(0.85)), ("Drive", N(0.6))]),
    p("Thin Scream", &[("Model", C("Scream HP")), ("Cutoff", N(600.0)), ("Resonance", N(0.5))]),
    p("Wobble", &[("Model", C("Ladder")), ("Cutoff", N(300.0)), ("Resonance", N(0.6)), ("LFO Amount", N(0.6)), ("LFO Rate", N(2.0))]),
    p("Envelope Wah", &[("Model", C("Multimode BP")), ("Cutoff", N(500.0)), ("Resonance", N(0.6)), ("Env Amount", N(0.8)), ("Env Release", N(80.0))]),
    p("Moving Notch", &[("Model", C("Multimode Notch")), ("Cutoff", N(1000.0)), ("Resonance", N(0.3)), ("LFO Amount", N(0.5)), ("LFO Rate", N(0.3)), ("Stereo", N(0.5))]),
    p("Narrow Band", &[("Model", C("Clean BP")), ("Cutoff", N(1500.0)), ("Resonance", N(0.4))]),
    p("Random Steps", &[("Model", C("Half Ladder")), ("Cutoff", N(800.0)), ("Resonance", N(0.5)), ("LFO Amount", N(0.5)), ("LFO Rate", N(4.0)), ("LFO Shape", C("Random"))]),
    p("Slow Sweep", &[("Model", C("Multimode LP")), ("Cutoff", N(200.0)), ("Resonance", N(0.5)), ("LFO Amount", N(0.8)), ("LFO Rate", N(0.1)), ("LFO Shape", C("Saw Up"))]),
    p("Clean Low Cut", &[("Model", C("Clean HP")), ("Cutoff", N(120.0)), ("Resonance", N(0.1)), ("Drive", N(0.0))]),
];

static WARPS_PRESETS: &[Preset] = &[
    p("Fold", &[("Algorithm", N(0.15)), ("Timbre", N(0.6)), ("Drive", N(0.7))]),
    p("Diode Ring", &[("Algorithm", N(0.25)), ("Timbre", N(0.3)), ("Note", N(55.0))]),
    p("Digital Ring", &[("Algorithm", N(0.375)), ("Timbre", N(0.2)), ("Note", N(60.0)), ("Shape", C("Triangle"))]),
    p("Bit Crash", &[("Algorithm", N(0.5)), ("Timbre", N(0.8)), ("Drive", N(0.8))]),
    p("Comparator", &[("Algorithm", N(0.625)), ("Timbre", N(0.5))]),
    p("Robot Voice", &[("Algorithm", N(0.8)), ("Timbre", N(0.5)), ("Shape", C("Saw")), ("Note", N(43.0))]),
    p("Whisper", &[("Algorithm", N(0.8)), ("Timbre", N(0.5)), ("Shape", C("Saw")), ("Note", N(36.0)), ("Drive", N(0.7))]),
    p("Formant Up", &[("Algorithm", N(0.85)), ("Timbre", N(0.85)), ("Shape", C("Triangle")), ("Note", N(48.0))]),
    p("Wide Ring", &[("Algorithm", N(0.3)), ("Timbre", N(0.4)), ("Note", N(52.0)), ("Detune", N(20.0))]),
];

static TELEPHONE: &[Preset] = &[
    p("Old Phone", &[("Band", N(0.5)), ("Squash", N(0.4)), ("Line", N(0.2))]),
    p("Tin Can", &[("Band", N(1.0)), ("Squash", N(0.8)), ("Line", N(0.1))]),
    p("Bad Line", &[("Band", N(0.6)), ("Line", N(0.8))]),
    p("Radio", &[("Band", N(0.2)), ("Squash", N(0.3)), ("Line", N(0.05))]),
];

static SLAM: &[Preset] = &[
    p("Punchy Drums", &[("Punch", N(0.8)), ("Drive", N(0.3))]),
    p("Sidechain Pump", &[("Punch", N(0.0)), ("Drive", N(0.0)), ("Pump", N(0.8)), ("Shape", N(0.4))]),
    p("Smash", &[("Punch", N(0.6)), ("Drive", N(0.8))]),
    p("Gentle Pump", &[("Punch", N(0.2)), ("Drive", N(0.1)), ("Pump", N(0.4))]),
];

static MOO: &[Preset] = &[
    p("Low Moo", &[("Shift", N(15.0)), ("Sideband", C("Down")), ("Delay", N(0.3)), ("Feedback", N(0.7))]),
    p("Barber Pole", &[("Shift", N(60.0)), ("Sideband", C("Up")), ("Delay", N(0.08)), ("Feedback", N(0.85))]),
    p("Spiral Apart", &[("Shift", N(40.0)), ("Sideband", C("Both")), ("Feedback", N(0.7))]),
    p("Alien", &[("Shift", N(300.0)), ("Sideband", C("Up")), ("Delay", N(0.15)), ("Feedback", N(0.6))]),
];

static SKY: &[Preset] = &[
    p("Huge Cloud", &[("Type", C("Cloud")), ("Decay", N(12.0)), ("Size", N(0.9)), ("Character", N(0.7)), ("Modulation", N(0.6)), ("Mix", N(0.45))]),
    p("Bloom Pad", &[("Type", C("Bloom")), ("Decay", N(8.0)), ("Character", N(0.7)), ("Mix", N(0.4))]),
    p("Choir Tail", &[("Type", C("Chorale")), ("Decay", N(6.0)), ("Character", N(0.3)), ("Mix", N(0.4))]),
    p("Volume Swells", &[("Type", C("Swell")), ("Decay", N(7.0)), ("Character", N(0.5)), ("Mix", N(0.55))]),
    p("Magneto Echo", &[("Type", C("Magneto")), ("Decay", N(4.0)), ("Character", N(0.4)), ("Mix", N(0.4))]),
    p("Shimmer Sky", &[("Type", C("Hall")), ("Decay", N(10.0)), ("Shimmer", N(0.6)), ("Size", N(0.8)), ("Mix", N(0.45))]),
    p("Small Room", &[("Type", C("Room")), ("Decay", N(0.8)), ("Size", N(0.4)), ("Character", N(0.6)), ("Mix", N(0.25))]),
    p("Bright Plate", &[("Type", C("Plate")), ("Decay", N(2.5)), ("High", N(12000.0)), ("Character", N(0.6)), ("Mix", N(0.3))]),
    p("Dark Hall", &[("Type", C("Hall")), ("Decay", N(6.0)), ("High", N(2500.0)), ("Low", N(1.4)), ("Mix", N(0.35))]),
];

static MASSIVE: &[Preset] = &[
    p("Blooming Echoes", &[("Mode", C("Nebula")), ("Delay", N(350.0)), ("Warp", N(0.4)), ("Feedback", N(0.8)), ("Density", N(0.6))]),
    p("Comet Trails", &[("Mode", C("Comet")), ("Delay", N(400.0)), ("Feedback", N(0.7)), ("Density", N(0.3)), ("Warp", N(0.2))]),
    p("Rhythm Pulsar", &[("Mode", C("Pulsar")), ("Sync", C("1/8")), ("Feedback", N(0.6)), ("Density", N(0.2))]),
    p("Vast Space", &[("Mode", C("Quasar")), ("Delay", N(600.0)), ("Feedback", N(0.9)), ("Density", N(0.9))]),
    p("Endless Horizon", &[("Mode", C("Horizon")), ("Delay", N(800.0)), ("Feedback", N(0.95)), ("Density", N(0.8)), ("Mix", N(0.6))]),
    p("Bottomless", &[("Mode", C("Void")), ("Delay", N(900.0)), ("Feedback", N(0.93)), ("Density", N(1.0)), ("High Cut", N(4000.0))]),
    p("Wobbly Echoes", &[("Mode", C("Drift")), ("Delay", N(300.0)), ("Feedback", N(0.7)), ("Mod Depth", N(0.7)), ("Mod Rate", N(0.8))]),
    p("Halo Pad", &[("Mode", C("Halo")), ("Delay", N(500.0)), ("Feedback", N(0.85)), ("Density", N(0.8)), ("Mix", N(0.55))]),
    p("Infinite Hold", &[("Mode", C("Quasar")), ("Delay", N(700.0)), ("Feedback", N(1.0)), ("Density", N(0.9)), ("Mix", N(0.6))]),
];

static SMEAR: &[Preset] = &[
    p("Cavern", &[("Length", N(400.0)), ("Diffuse", N(0.8)), ("Reflect", N(0.7)), ("Dampen", N(0.4))]),
    p("Short Swarm", &[("Length", N(60.0)), ("Drag", N(0.8)), ("Reflect", N(0.5))]),
    p("Self-Oscillating", &[("Length", N(250.0)), ("Reflect", N(1.0)), ("Dampen", N(0.5)), ("Mix", N(0.5))]),
    p("Dark Smear", &[("Length", N(500.0)), ("Dampen", N(0.8)), ("Reflect", N(0.75))]),
    p("Metallic", &[("Length", N(30.0)), ("Diffuse", N(0.2)), ("Drag", N(0.2)), ("Reflect", N(0.85))]),
];

static SNOWGLOBE: &[Preset] = &[
    p("Layered Loops", &[("Mode", C("Tiles")), ("Variation", C("D")), ("Activity", N(0.7)), ("Space", N(0.35))]),
    p("Backwards Loops", &[("Mode", C("Tiles")), ("Variation", C("C")), ("Reverse", N(1.0)), ("Space", N(0.4))]),
    p("Pitch Steps", &[("Mode", C("Steps")), ("Variation", C("B")), ("Activity", N(0.6)), ("Repeats", N(0.2))]),
    p("Gliding Loop", &[("Mode", C("Slide")), ("Variation", C("B")), ("Space", N(0.5))]),
    p("Snow Cloud", &[("Mode", C("Mist")), ("Variation", C("A")), ("Activity", N(0.75)), ("Shape", N(0.7)), ("Space", N(0.6)), ("Mix", N(0.6))]),
    p("Shimmer Cloud", &[("Mode", C("Mist")), ("Variation", C("C")), ("Activity", N(0.6)), ("Shape", N(0.8)), ("Space", N(0.7)), ("Mix", N(0.55))]),
    p("Ringing Cave", &[("Mode", C("Cave")), ("Variation", C("B")), ("Repeats", N(0.6)), ("Space", N(0.4))]),
    p("Strummed Grains", &[("Mode", C("Harp")), ("Variation", C("A")), ("Activity", N(0.6)), ("Shape", N(0.2)), ("Space", N(0.4))]),
    p("Shuffled Blocks", &[("Mode", C("Shuffle")), ("Variation", C("C")), ("Activity", N(0.8)), ("Space", N(0.15))]),
    p("Stutter", &[("Mode", C("Chop")), ("Variation", C("D")), ("Activity", N(0.5)), ("Space", N(0.0)), ("Mix", N(1.0))]),
    p("Climbing Arp", &[("Mode", C("Climb")), ("Variation", C("A")), ("Repeats", N(0.7)), ("Space", N(0.35))]),
    p("Dotted Echoes", &[("Mode", C("Rhythm")), ("Variation", C("B")), ("Repeats", N(0.45)), ("Space", N(0.25))]),
    p("Bending Echoes", &[("Mode", C("Bend")), ("Variation", C("C")), ("Repeats", N(0.5)), ("Space", N(0.3))]),
    p("Frozen Glitter", &[("Mode", C("Mist")), ("Variation", C("C")), ("Activity", N(0.9)), ("Space", N(0.8)), ("Filter", N(0.2)), ("Mix", N(0.7))]),
];

static CONVOLUTION: &[Preset] = &[
    p("Studio", &[("Space", C("Studio")), ("Mix", N(0.25))]),
    p("Concert Hall", &[("Space", C("Concert Hall"))]),
    p("Cathedral", &[("Space", C("Cathedral")), ("Pre-Delay", N(20.0)), ("Mix", N(0.4))]),
    p("Huge Hall", &[("Space", C("Concert Hall")), ("Size", N(1.8)), ("Decay", N(1.5)), ("Mix", N(0.4))]),
    p("Reverse Cathedral", &[("Space", C("Cathedral")), ("Reverse", N(1.0)), ("Mix", N(0.45))]),
    p("Plate", &[("Space", C("Plate"))]),
    p("Spring Tank", &[("Space", C("Spring Tank")), ("Mix", N(0.3))]),
    p("Stairwell", &[("Space", C("Stairwell"))]),
    p("Parking Garage", &[("Space", C("Parking Garage"))]),
    p("Cave", &[("Space", C("Cave")), ("Mix", N(0.4))]),
    p("Tunnel", &[("Space", C("Tunnel"))]),
    p("Forest", &[("Space", C("Forest")), ("Mix", N(0.4))]),
    p("Speaker Cab", &[("Space", C("Speaker Cab")), ("Mix", N(1.0)), ("High Cut", N(9000.0))]),
    p("Small Cab", &[("Space", C("Speaker Cab")), ("Size", N(0.7)), ("Mix", N(1.0))]),
];

static AW_GALACTIC: &[Preset] = &[
    p("Huge Space", &[("Bigness", N(1.0)), ("Replace", N(0.7))]),
    p("Bright Galaxy", &[("Brightness", N(0.85)), ("Dry/Wet", N(0.5))]),
    p("Dark Nebula", &[("Brightness", N(0.2)), ("Detune", N(0.7)), ("Dry/Wet", N(0.6))]),
];

static AW_VERBITY: &[Preset] = &[p("Room", &[("RmSize", N(0.3)), ("Wetness", N(0.3))]), p("Big", &[("RmSize", N(0.9)), ("Sustain", N(0.8)), ("Wetness", N(0.4))])];
static AW_CHAMBER: &[Preset] = &[p("Small", &[("Bigness", N(0.2)), ("Longness", N(0.2))]), p("Large", &[("Bigness", N(0.8)), ("Longness", N(0.7))])];
static AW_TO_TAPE: &[Preset] = &[p("Gentle Tape", &[("Input", N(0.45))]), p("Hot Tape", &[("Input", N(0.8)), ("Flutter", N(0.6))])];
static AW_IRON_OXIDE: &[Preset] = &[p("Clean Deck", &[("Flutter", N(0.2)), ("Noise", N(0.1))]), p("Worn Deck", &[("Flutter", N(0.8)), ("Noise", N(0.6))])];
static AW_PUREST_DRIVE: &[Preset] = &[p("Touch", &[("Drive", N(0.3))]), p("Driven", &[("Drive", N(0.8))])];
static AW_DENSITY: &[Preset] = &[p("Thick", &[("Density", N(0.4))]), p("Heavy", &[("Density", N(0.8)), ("Output", N(0.8))])];
static AW_TUBE: &[Preset] = &[p("Warm", &[("Tube", N(0.4))]), p("Hot", &[("Input", N(0.7)), ("Tube", N(0.8))])];
static AW_MACKITY: &[Preset] = &[p("Console", &[("In Trim", N(0.15))]), p("Pushed", &[("In Trim", N(0.5)), ("Out Pad", N(0.6))])];
static AW_PRESSURE: &[Preset] = &[p("Bus Glue", &[("Pressure", N(0.3))]), p("Squash", &[("Pressure", N(0.7)), ("Speed", N(0.5))])];
static AW_BUTTER_COMP: &[Preset] = &[p("Soft", &[("Compress", N(0.3))]), p("Heavy", &[("Compress", N(0.7))])];
static AW_AIR: &[Preset] = &[p("Airy", &[("Air", N(0.7))]), p("Darker", &[("Air", N(0.3))])];
static AW_CAPACITOR: &[Preset] = &[p("Warm Filter", &[("Lowpass", N(0.6))]), p("Thin", &[("Highpass", N(0.4))])];
static AW_DEREZ: &[Preset] = &[p("Lo-Fi", &[("Rate", N(0.4)), ("Rez", N(0.5))]), p("Crushed", &[("Rate", N(0.2)), ("Rez", N(0.2))])];
static AW_TAPE_DELAY: &[Preset] = &[p("Short Tape", &[("Time", N(0.3)), ("Regen", N(0.3)), ("Dry/Wet", N(0.5))]), p("Dub Tape", &[("Time", N(0.7)), ("Regen", N(0.7)), ("Flutter", N(0.4)), ("Dry/Wet", N(0.5))])];
static AW_PEAR2_PRESETS: &[Preset] = &[p("Soft Low Pass", &[("Freq", N(0.5)), ("Poles", N(0.5))]), p("Steep High Pass", &[("Freq", N(0.4)), ("Poles", N(1.0)), ("Inv/Wet", N(0.0))])];
static AW_ISOLATOR3_PRESETS: &[Preset] = &[p("Lows Only", &[("Iso", N(0.3))]), p("Highs Only", &[("Iso", N(0.7))]), p("Narrow Band", &[("Iso", N(0.5)), ("Q", N(0.8))])];
static AW_HULL2_PRESETS: &[Preset] = &[p("Bright", &[("Treble", N(0.7))]), p("Warm", &[("Treble", N(0.4)), ("Bass", N(0.6))])];
static AW_BAXANDALL2_PRESETS: &[Preset] = &[p("Hi-Fi Smile", &[("Treble", N(0.6)), ("Bass", N(0.6))]), p("Darker", &[("Treble", N(0.35))])];
static AW_CABS_PRESETS: &[Preset] = &[p("Small Combo", &[("CabType", N(0.1)), ("CabSize", N(0.4))]), p("Big Stack", &[("CabType", N(0.5)), ("CabSize", N(0.9)), ("Intense", N(0.8))]), p("Off Axis", &[("OffAxis", N(0.8))])];
static AW_DISTANCE2_PRESETS: &[Preset] = &[p("Across The Room", &[("Atmosph", N(0.6)), ("Darken", N(0.4))]), p("Far Away", &[("Atmosph", N(0.95)), ("Darken", N(0.8)), ("Dry/Wet", N(0.9))])];
static AW_COILS2_PRESETS: &[Preset] = &[p("Iron", &[("Saturate", N(0.3))]), p("Cheap Transformer", &[("Saturate", N(0.5)), ("Cheapness", N(0.7))])];
static AW_FRACTURE2_PRESETS: &[Preset] = &[p("Folded", &[("Drive", N(0.4)), ("Fractre", N(0.3))]), p("Shattered", &[("Drive", N(0.7)), ("Fractre", N(0.8))])];
static AW_ENSEMBLE_PRESETS: &[Preset] = &[p("Lush", &[("Ensemble", N(0.6)), ("Fullness", N(0.4)), ("Dry/Wet", N(0.6))]), p("Subtle", &[("Ensemble", N(0.3)), ("Dry/Wet", N(0.4))])];
static AW_INFINITY2_PRESETS: &[Preset] = &[p("Big Space", &[("Feedback", N(0.7)), ("Dry/Wet", N(0.4))]), p("Endless", &[("Feedback", N(1.0)), ("Dry/Wet", N(0.5))])];

// ---------------------------------------------------------------- MIDI effects

static ARPEGGIATOR: &[Preset] = &[
    p("Sixteenths Up", &[("Style", C("Up")), ("Rate", C("1/16"))]),
    p("Trance Gate", &[("Style", C("Up")), ("Rate", C("1/16")), ("Gate", N(0.3)), ("Octaves", N(2.0))]),
    p("Up and Down Eighths", &[("Style", C("Up Down")), ("Rate", C("1/8"))]),
    p("Random Sparkle", &[("Style", C("Random")), ("Rate", C("1/16")), ("Octaves", N(3.0)), ("Gate", N(0.4))]),
    p("Triplet Roll", &[("Style", C("Up Down")), ("Rate", C("1/16 T"))]),
    p("Held Chords", &[("Style", C("Chord")), ("Rate", C("1/8")), ("Hold", N(1.0))]),
];

static CHORD: &[Preset] = &[
    p("Triads in Key", &[("Type", C("Key Triad"))]),
    p("Sevenths in Key", &[("Type", C("Key 7th"))]),
    p("Power Chord", &[("Type", C("Power"))]),
    p("Octaves", &[("Type", C("Octaves"))]),
    p("Sus2 Pads", &[("Type", C("Sus2"))]),
    p("Stacked Fifths", &[("Type", C("Custom")), ("Shift 1", N(7.0)), ("Shift 2", N(14.0)), ("Shift 3", N(21.0))]),
];

static NOTE_LENGTH: &[Preset] = &[
    p("Staccato", &[("Length", C("1/32"))]),
    p("Legato", &[("Length", C("1/4")), ("Gate", N(2.0))]),
    p("Pads", &[("Length", C("1 Bar"))]),
];

static PITCH: &[Preset] = &[p("Octave Up", &[("Pitch", N(12.0))]), p("Octave Down", &[("Pitch", N(-12.0))]), p("Up a Fifth", &[("Pitch", N(7.0))]), p("Third Up in Key", &[("Scale Steps", N(2.0))])];

static SCALE: &[Preset] = &[
    p("Project Key", &[("Source", C("Project"))]),
    p("C Minor Pentatonic", &[("Source", C("Custom")), ("Root", C("C")), ("Scale", C("Minor Pentatonic"))]),
    p("Blues", &[("Source", C("Custom")), ("Root", C("A")), ("Scale", C("Blues"))]),
    p("Japanese", &[("Source", C("Custom")), ("Root", C("D")), ("Scale", C("In Sen"))]),
];

static RANDOM: &[Preset] = &[p("A Little Variation", &[("Chance", N(0.2))]), p("Wild", &[("Chance", N(0.8)), ("Choices", N(8.0))]), p("Octave Jumps", &[("Chance", N(0.4)), ("Choices", N(2.0)), ("Spacing", N(12.0))])];

static VELOCITY: &[Preset] = &[
    p("Even", &[("Out Low", N(0.55)), ("Out High", N(0.9))]),
    p("All Loud", &[("Fixed", N(1.0)), ("Out High", N(1.0))]),
    p("Human", &[("Random", N(0.3))]),
    p("Softer", &[("Drive", N(-0.5))]),
];

static NOTE_ECHO: &[Preset] = &[
    p("Eighth Echoes", &[("Time", C("1/8")), ("Repeats", N(3.0))]),
    p("Dotted Repeats", &[("Time", C("1/8 D")), ("Repeats", N(4.0)), ("Feedback", N(0.7))]),
    p("Climbing", &[("Time", C("1/8")), ("Repeats", N(4.0)), ("Pitch", N(7.0))]),
    p("Falling Octaves", &[("Time", C("1/4")), ("Repeats", N(3.0)), ("Pitch", N(-12.0))]),
];

static STRUM: &[Preset] = &[p("Guitar Down", &[("Time", N(20.0)), ("Direction", C("Down"))]), p("Harp Up", &[("Time", N(60.0)), ("Direction", C("Up"))]), p("Back and Forth", &[("Direction", C("Alternate"))])];

static NOTE_REPEAT: &[Preset] = &[p("Sixteenth Roll", &[("Rate", C("1/16"))]), p("Triplet Roll", &[("Rate", C("1/16 T"))]), p("Fading Roll", &[("Rate", C("1/32")), ("Decay", N(0.5))])];

static HUMANIZE: &[Preset] = &[
    p("Subtle", &[("Timing", N(4.0)), ("Velocity", N(0.1))]),
    p("Loose", &[("Timing", N(15.0)), ("Velocity", N(0.25))]),
    p("Swing Eighths", &[("Swing", N(0.3)), ("Swing On", C("1/8"))]),
    p("Swing Sixteenths", &[("Swing", N(0.25)), ("Swing On", C("1/16"))]),
];

static NOTE_FILTER: &[Preset] = &[p("Every Other", &[("Every", N(2.0))]), p("Bass Only", &[("High Key", N(52.0))]), p("Treble Only", &[("Low Key", N(60.0))]), p("Thin Out", &[("Chance", N(0.5))])];

static MONO: &[Preset] = &[p("Last Note", &[("Priority", C("Last"))]), p("Lowest Note", &[("Priority", C("Low"))]), p("Legato Lead", &[("Priority", C("Last")), ("Legato", N(1.0))])];

static QUANTIZE: &[Preset] = &[p("Sixteenths", &[("Grid", C("1/16"))]), p("Loose Eighths", &[("Grid", C("1/8")), ("Amount", N(0.5))]), p("Triplets", &[("Grid", C("1/16 T"))])];

// ---------------------------------------------------------------- sequencers

static STEP: &[Preset] = &[
    pd("Rising Arp", &[("Rate", C("1/16"))], || melody([Some(0), Some(2), Some(4), Some(7), Some(9), Some(7), Some(4), Some(2), Some(0), Some(2), Some(4), Some(7), Some(11), Some(9), Some(7), Some(4)])),
    pd("Off-Beat Hook", &[("Rate", C("1/16")), ("Swing", N(0.2))], || melody([None, None, Some(7), None, None, Some(7), None, Some(9), None, None, Some(7), None, Some(4), None, Some(2), None])),
    pd("Bass Walk", &[("Rate", C("1/8")), ("Base Note", N(36.0))], || melody([Some(0), None, Some(0), Some(2), Some(4), None, Some(4), Some(3), Some(2), None, Some(2), Some(4), Some(5), Some(4), Some(2), Some(1)])),
    pd("Ping-Pong", &[("Direction", C("Ping-Pong"))], || melody([Some(0), Some(4), Some(7), Some(11), Some(14), Some(11), Some(7), Some(4), Some(0), Some(4), Some(7), Some(11), Some(14), Some(11), Some(7), Some(4)])),
];

static ACID: &[Preset] = &[
    pd("Classic Line", &[], crate::classic_seq::acid_default),
    pd(
        "Squelch Run",
        &[("Accent", N(0.8))],
        || acid([(1, 0, 1, 1, 0), (1, 0, 1, 0, 1), (1, 3, 1, 0, 1), (1, 5, 1, 1, 0), (1, 7, 1, 0, 1), (1, 10, 1, 0, 1), (1, 12, 1, 1, 0), (1, 10, 1, 0, 1), (1, 7, 1, 0, 0), (0, 0, 1, 0, 0), (1, 0, 2, 1, 0), (1, 0, 1, 0, 1), (1, 3, 1, 0, 0), (2, 3, 1, 0, 0), (1, 0, 1, 1, 0), (1, 7, 0, 0, 1)]),
    ),
    pd(
        "Octave Bounce",
        &[("Gate", N(0.4))],
        || acid([(1, 0, 0, 1, 0), (1, 0, 1, 0, 0), (1, 0, 0, 0, 0), (1, 0, 1, 1, 0), (1, 0, 0, 0, 0), (1, 0, 1, 0, 1), (1, 3, 1, 0, 0), (1, 0, 0, 1, 0), (1, 0, 1, 0, 0), (1, 0, 0, 0, 0), (1, 0, 1, 1, 0), (1, 5, 1, 0, 1), (1, 7, 1, 0, 0), (1, 0, 0, 1, 0), (1, 12, 1, 0, 1), (1, 0, 1, 0, 0)]),
    ),
    pd(
        "Minimal",
        &[("Gate", N(0.7))],
        || acid([(1, 0, 1, 1, 0), (0, 0, 1, 0, 0), (0, 0, 1, 0, 0), (1, 0, 1, 0, 1), (1, 12, 1, 0, 0), (0, 0, 1, 0, 0), (1, 0, 1, 1, 0), (0, 0, 1, 0, 0), (1, 0, 1, 0, 0), (0, 0, 1, 0, 0), (1, 3, 1, 1, 1), (1, 0, 1, 0, 0), (0, 0, 1, 0, 0), (1, 0, 1, 0, 0), (1, 10, 1, 0, 1), (1, 12, 1, 1, 0)]),
    ),
];

static DRUM_GRID: &[Preset] = &[
    pd("Four on the Floor", &[], || named_beat("Four on the Floor")),
    pd("Breakbeat", &[], || named_beat("Breakbeat")),
    pd("Electro", &[], || named_beat("Electro")),
    pd("Boom Bap", &[("Swing", N(0.15))], || named_beat("Boom Bap")),
    pd("Half Time", &[], || named_beat("Half Time")),
    pd("Trap", &[("Rate", C("1/16"))], || beat(["X......x..x.....", "........X.......", "................", "x.x.x.xxx.x.x.xx", "................", "..............x.", "................", "................"])),
    pd("Dembow", &[], || beat(["X...X...X...X...", "...x..x....x..x.", "................", "x.x.x.x.x.x.x.x.", "................", "................", "................", "................"])),
    pd("Disco", &[], || beat(["X...X...X...X...", "....X.......X...", "....x.......x...", "x.x.x.x.x.x.x.x.", "..x...x...x...x.", "................", "................", "................"])),
    pd("Funk", &[("Swing", N(0.1))], || beat(["X..x..X...x..x..", "....X..x.x..X..x", "................", "xxxxxxxxxxxxxxxx", "...........x....", "................", "................", "................"])),
    pd("Bossa", &[], || beat(["X..xX..xX..xX..x", "................", "................", "x.x.x.x.x.x.x.x.", "................", "................", "X..x..x...x..x..", "................"])),
    pd("Reggae One Drop", &[], || beat(["........X.......", "........X.......", "................", "..x...x...x...x.", "................", "................", "..x.......x.....", "................"])),
    pd("Rock", &[], || beat(["X.......X.x.....", "....X.......X...", "................", "x.x.x.x.x.x.x.x.", "..............x.", "................", "................", "................"])),
];

/// (degree, quality, neck) for the classic progressions.
const POP: &[(u8, u8, u8)] = &[(1, 0, 0), (5, 0, 0), (6, 0, 0), (4, 0, 0)];
const DOO_WOP: &[(u8, u8, u8)] = &[(1, 0, 0), (6, 0, 0), (4, 0, 0), (5, 0, 0)];
const SAD: &[(u8, u8, u8)] = &[(6, 0, 0), (4, 0, 0), (1, 0, 0), (5, 0, 0)];
const JAZZ: &[(u8, u8, u8)] = &[(2, 1, 1), (5, 1, 1), (1, 1, 1), (6, 1, 1)];
const CANON: &[(u8, u8, u8)] = &[(1, 0, 0), (5, 0, 0), (6, 0, 0), (3, 0, 0), (4, 0, 0), (1, 0, 0), (4, 0, 0), (5, 0, 0)];
const ANDALUSIAN: &[(u8, u8, u8)] = &[(6, 0, 0), (5, 0, 0), (4, 0, 0), (3, 0, 0)];
const ROYAL_ROAD: &[(u8, u8, u8)] = &[(4, 1, 1), (5, 0, 1), (3, 1, 1), (6, 0, 1)];
const BLUES: &[(u8, u8, u8)] = &[(1, 1, 0), (4, 1, 0), (1, 1, 0), (1, 1, 0), (4, 1, 0), (4, 1, 0), (1, 1, 0), (5, 1, 0)];
const DREAMY: &[(u8, u8, u8)] = &[(1, 1, 0), (4, 1, 0)];
const EPIC: &[(u8, u8, u8)] = &[(6, 5, 0), (4, 5, 0), (1, 5, 0), (5, 5, 0)];
const FOLK: &[(u8, u8, u8)] = &[(1, 0, 0), (4, 0, 0), (1, 0, 0), (5, 0, 0)];
const REGGAE: &[(u8, u8, u8)] = &[(1, 0, 1), (4, 0, 1), (5, 0, 1), (4, 0, 1)];
const FUNK: &[(u8, u8, u8)] = &[(2, 1, 2), (5, 1, 2)];
const PUNK: &[(u8, u8, u8)] = &[(1, 4, 1), (5, 4, 1), (6, 4, 1), (4, 4, 1)];

static PROGRESSION: &[Preset] = &[
    pd("Pop (I V vi IV)", &[("Chords", N(4.0)), ("Length", C("1 bar"))], || prog(POP)),
    pd("Doo-Wop (I vi IV V)", &[("Chords", N(4.0)), ("Rhythm", C("Charleston"))], || prog(DOO_WOP)),
    pd("Sad (vi IV I V)", &[("Chords", N(4.0)), ("Rhythm", C("Arp Up"))], || prog(SAD)),
    pd("Jazz (ii V I vi)", &[("Chords", N(4.0)), ("Rhythm", C("Stabs"))], || prog(JAZZ)),
    pd("Canon", &[("Chords", N(8.0)), ("Length", C("2 beats"))], || prog(CANON)),
    pd("Andalusian (vi V IV iii)", &[("Chords", N(4.0))], || prog(ANDALUSIAN)),
    pd("Royal Road", &[("Chords", N(4.0)), ("Rhythm", C("Eighths"))], || prog(ROYAL_ROAD)),
    pd("Eight-Bar Blues", &[("Chords", N(8.0)), ("Rhythm", C("Quarters"))], || prog(BLUES)),
    pd("Dreamy Sevenths", &[("Chords", N(2.0)), ("Length", C("2 bars"))], || prog(DREAMY)),
    pd("Epic Add9", &[("Chords", N(4.0))], || prog(EPIC)),
];

static GUITAR_CHORDS: &[Preset] = &[
    pd("Campfire", &[("Chords", N(4.0))], || strum(POP, "Old Faithful")),
    pd("Folk Picking", &[("Chords", N(4.0)), ("Let Ring", N(1.0))], || strum(FOLK, "Travis Picking")),
    pd("Reggae Skank", &[("Chords", N(4.0)), ("Gate", N(0.3))], || strum(REGGAE, "Reggae")),
    pd("Funk Vamp", &[("Chords", N(2.0)), ("Strum", N(8.0))], || strum(FUNK, "Funk")),
    pd("Ballad Arpeggio", &[("Chords", N(4.0)), ("Let Ring", N(1.0))], || strum(SAD, "Arpeggio")),
    pd("Punk Eighths", &[("Chords", N(4.0)), ("Strum", N(6.0))], || strum(PUNK, "Eighths")),
    pd("Waltz", &[("Chords", N(4.0))], || strum(FOLK, "Waltz Feel")),
    pd("Rock Rhythm", &[("Chords", N(4.0)), ("Strum", N(12.0))], || strum(DOO_WOP, "Rock")),
    pd("Gallop", &[("Chords", N(4.0)), ("Strum", N(10.0))], || strum(ANDALUSIAN, "Gallop")),
];

static UKULELE: &[Preset] = &[
    pd("Island Strum", &[("Chords", N(4.0))], || strum(POP, "Old Faithful")),
    pd("Calypso", &[("Chords", N(4.0)), ("Strum", N(10.0))], || strum(REGGAE, "Calypso")),
    pd("Fingerpicked", &[("Chords", N(4.0)), ("Let Ring", N(1.0))], || strum(SAD, "Fingerpicked Ballad")),
    pd("Low G Ballad", &[("Tuning", C("Low G")), ("Let Ring", N(1.0))], || strum(DREAMY, "Ballad")),
    pd("Baritone Folk", &[("Tuning", C("Baritone")), ("Let Ring", N(1.0))], || strum(FOLK, "Travis Picking")),
    pd("Ska Chop", &[("Gate", N(0.35))], || strum(POP, "Ska Upstrokes")),
    pd("Strummy Sixteenths", &[("Strum", N(8.0))], || strum(DOO_WOP, "Funk Sixteenths")),
    pd("Waltz", &[("Chords", N(4.0))], || strum(FOLK, "Waltz Feel")),
];

static POLYMETER: &[Preset] = &[p("Steady", &[("Rate", C("1/16"))]), p("Slow and Low", &[("Rate", C("1/8")), ("Base Note", N(48.0))]), p("Short Notes", &[("Gate", N(0.3))])];

static EUCLID: &[Preset] = &[
    p(
        "Four Rings",
        &[],
    ),
    p(
        "Afro Bell",
        &[("Steps 1", N(16.0)), ("Pulses 1", N(4.0)), ("Steps 2", N(12.0)), ("Pulses 2", N(7.0)), ("Note 2", N(56.0)), ("Steps 3", N(16.0)), ("Pulses 3", N(5.0)), ("Note 3", N(39.0)), ("Steps 4", N(16.0)), ("Pulses 4", N(9.0)), ("Rotate 4", N(1.0)), ("Note 4", N(42.0))],
    ),
    p(
        "Tresillo",
        &[("Pulses 1", N(3.0)), ("Steps 1", N(8.0)), ("Pulses 2", N(2.0)), ("Rotate 2", N(4.0)), ("Pulses 3", N(8.0)), ("Steps 4", N(16.0)), ("Pulses 4", N(3.0)), ("Rotate 4", N(2.0)), ("Note 4", N(37.0))],
    ),
    p("Sparse", &[("Pulses 1", N(2.0)), ("Pulses 2", N(1.0)), ("Pulses 3", N(3.0)), ("Pulses 4", N(2.0))]),
];

static TURING: &[Preset] = &[p("Slowly Changing", &[("Change", N(0.05))]), p("Locked Loop", &[("Change", N(0.0)), ("Length", N(16.0))]), p("Restless", &[("Change", N(0.5)), ("Density", N(0.9))]), p("Wide Range", &[("Range", N(14.0)), ("Base Note", N(48.0))])];
static LIFE: &[Preset] = &[p("Slow Evolution", &[("Evolve", N(32.0)), ("Rate", C("1/8"))]), p("Busy", &[("Evolve", N(4.0)), ("Rate", C("1/16"))]), p("Low", &[("Base Note", N(36.0))])];
static BOUNCE: &[Preset] = &[
    p("Gentle Bounce", &[("Balls", N(3.0)), ("Energy", N(0.85))]),
    p("Busy Balls", &[("Balls", N(8.0)), ("Fall Time", N(0.5))]),
    p("Slow Drops", &[("Balls", N(2.0)), ("Fall Time", N(2.5)), ("Gate", N(1.0))]),
    p("Pentatonic", &[("Note 1", N(0.0)), ("Note 2", N(1.0)), ("Note 3", N(2.0)), ("Note 4", N(4.0)), ("Note 5", N(5.0)), ("Note 6", N(7.0)), ("Note 7", N(9.0)), ("Note 8", N(-2.0))]),
];
static DRUNK: &[Preset] = &[p("Wanderer", &[("Step", N(1.0))]), p("Big Steps", &[("Step", N(3.0)), ("Range", N(14.0))]), p("Sparse", &[("Rest", N(0.5)), ("Rate", C("1/8"))])];
static ORBITS: &[Preset] = &[p("Solar System", &[("Planets", N(5.0)), ("Period", C("4 Bars"))]), p("Fast Moons", &[("Planets", N(8.0)), ("Period", C("1 Bar"))]), p("Two Planets", &[("Planets", N(2.0)), ("Period", C("2 Bars"))])];
static CHAOS: &[Preset] = &[p("Edge of Chaos", &[("Chaos", N(3.57))]), p("Full Chaos", &[("Chaos", N(4.0))]), p("Periodic", &[("Chaos", N(3.2))])];
static AUTOMATON: &[Preset] = &[p("Rule 30", &[("Rule", N(30.0))]), p("Rule 110", &[("Rule", N(110.0))]), p("Triangles", &[("Rule", N(90.0)), ("Voices", N(6.0))])];
static ANT: &[Preset] = &[p("One Ant", &[("Ants", N(1.0))]), p("Colony", &[("Ants", N(3.0)), ("Rate", C("1/16"))]), p("On Black Only", &[("Play", C("On Black"))])];
static PONG: &[Preset] = &[p("Rally", &[("Speed", N(1.0))]), p("Fast Game", &[("Speed", N(2.5)), ("Balls", N(2.0))]), p("Lob", &[("Speed", N(0.5)), ("Angle", N(60.0))])];
static MARKOV: &[Preset] = &[p("Singer", &[("Step", N(0.7)), ("Leap", N(0.1))]), p("Leaper", &[("Leap", N(0.6)), ("Step", N(0.3))]), p("Stammer", &[("Repeat", N(0.6))]), p("Falling", &[("Up/Down", N(-0.6))])];
static RAIN: &[Preset] = &[p("Drizzle", &[("Density", N(1.0)), ("Storm", N(0.1))]), p("Storm", &[("Density", N(8.0)), ("Storm", N(0.8))]), p("On the Grid", &[("Sync", C("1/16")), ("Density", N(4.0))])];
static TOUCH: &[Preset] = &[
    pd("Plates", &[], crate::sequencers::touch::default_data),
    pd("Pulse", &[("Pulse", N(1.0)), ("Rate", C("1/8")), ("Gate", N(0.5))], crate::sequencers::touch::default_data),
    pd("Walking Steps", &[("Pulse", N(1.0)), ("Rate", C("1/16")), ("Sequencer", N(1.0)), ("Gate", N(0.45))], || crate::sequencers::touch::notes(&[0, 2, 4, 7, 4, 2, 5, 4])),
    pd("Chance Garden", &[("Pulse", N(1.0)), ("Rate", C("1/16")), ("Latch", N(1.0)), ("Chance", N(0.55)), ("Wander", N(0.3)), ("Gate", N(0.4))], crate::sequencers::touch::default_data),
    pd("Pendulum", &[("Pulse", N(1.0)), ("Rate", C("1/8 T")), ("Sequencer", N(1.0)), ("Order", C("Pendulum")), ("Steps", N(6.0))], || crate::sequencers::touch::notes(&[0, 1, 2, 4, 5, 7])),
    pd("Random Walk", &[("Pulse", N(1.0)), ("Rate", C("1/8")), ("Sequencer", N(1.0)), ("Order", C("Drunk")), ("Latch", N(1.0))], || crate::sequencers::touch::notes(&[-2, 0, 3, 5, -4, 2, 7, 0])),
    pd("Ratchet Groove", &[("Pulse", N(1.0)), ("Rate", C("1/8")), ("Sequencer", N(1.0)), ("Pitch", N(0.0)), ("Swing", N(0.4)), ("Latch", N(1.0)), ("Gate", N(0.5))], || {
        use crate::sequencers::touch::{pattern, Step};
        let d = Step::default();
        pattern(&[Step { accent: true, ..d }, d, Step { ratchet: 2, ..d }, Step { rest: true, ..d }, Step { accent: true, ..d }, Step { ratchet: 3, chance: 0.7, ..d }, d, Step { ratchet: 4, chance: 0.5, ..d }])
    }),
    pd("Power Riff", &[("Pulse", N(1.0)), ("Rate", C("1/8")), ("Sequencer", N(1.0)), ("Voicing", C("Power")), ("Run", N(1.0)), ("Gate", N(0.35))], || {
        use crate::sequencers::touch::{pattern, Step};
        let s = |note: i32, accent: bool, rest: bool| Step { note, accent, rest, ..Step::default() };
        pattern(&[s(0, true, false), s(0, false, false), s(0, false, true), s(3, true, false), s(0, false, false), s(5, false, false), s(4, true, false), s(-2, false, false)])
    }),
    pd("Arp Up", &[("Play", C("Arp")), ("Rate", C("1/16")), ("Octaves", N(2.0)), ("Voicing", C("Triad")), ("Latch", N(1.0)), ("Gate", N(0.5))], crate::sequencers::touch::default_data),
    pd("Arp Converge", &[("Play", C("Arp")), ("Arp Order", C("Converge")), ("Rate", C("1/16 T")), ("Octaves", N(3.0)), ("Voicing", C("Seventh")), ("Latch", N(1.0))], crate::sequencers::touch::default_data),
    pd("Arp Steps", &[("Play", C("Arp")), ("Arp Order", C("Up-Down")), ("Rate", C("1/16")), ("Voicing", C("Triad")), ("Sequencer", N(1.0)), ("Steps", N(4.0)), ("Latch", N(1.0))], || crate::sequencers::touch::notes(&[0, 0, 3, -2])),
    pd("Strummer", &[("Play", C("Strum")), ("Voicing", C("Seventh")), ("Strum", N(0.06)), ("Pulse", N(1.0)), ("Rate", C("1/4")), ("Gate", N(0.9))], crate::sequencers::touch::default_data),
    pd("Echo Rain", &[("Play", C("Echo")), ("Echoes", N(5.0)), ("Feedback", N(0.55)), ("Rate", C("1/8 T")), ("Wander", N(0.2))], crate::sequencers::touch::default_data),
    pd("Slow Bloom", &[("Pulse", N(1.0)), ("Rate", C("1/2")), ("Decay", N(1.6)), ("Gate", N(1.0)), ("Latch", N(1.0)), ("Voicing", C("Triad"))], crate::sequencers::touch::default_data),
    pd("Open Pads", &[("Voicing", C("Open")), ("Latch", N(1.0)), ("Decay", N(1.2)), ("LFO Rate", C("2 Bars"))], crate::sequencers::touch::default_data),
    pd("Stutter", &[("Pulse", N(1.0)), ("Rate", C("1/32")), ("Gate", N(0.3)), ("Decay", N(0.05))], crate::sequencers::touch::default_data),
];
static SING: &[Preset] = &[
    pd("Sakura", &[], crate::sequencers::sing::default_text),
    pd("La La La", &[("Rhythm", C("Lilt")), ("Melody", C("Wave"))], || b"la la la li la lo".to_vec()),
    pd("Doo Wop", &[("Rhythm", C("Pop")), ("Melody", C("Walk")), ("Range", N(4.0))], || b"du du wa sha la la".to_vec()),
    pd("Hallelujah", &[("Rhythm", C("Chant")), ("Melody", C("Arch")), ("Range", N(4.0)), ("Word End", N(2.0))], || b"ha re ru ya".to_vec()),
    pd("Lullaby", &[("Rate", C("1/4")), ("Melody", C("Fall")), ("Breaths", N(0.7))], || b"nen nen ko ro ri yo".to_vec()),
    pd("Scat", &[("Rate", C("1/16")), ("Rhythm", C("Free")), ("Melody", C("Free")), ("Range", N(7.0)), ("Variation", N(0.6)), ("Word End", N(0.0))], || b"shu bi du ba da ba".to_vec()),
    pd("Chant", &[("Rhythm", C("Chant")), ("Melody", C("Repeat")), ("Range", N(2.0)), ("Breaths", N(0.0))], || b"o u a o u e".to_vec()),
    pd("Backwards", &[("Order", C("Backward"))], || b"konnichiwa sekai".to_vec()),
];
static WORDS: &[Preset] = &[
    pd("Hello", &[], || b"hello world".to_vec()),
    pd("Morse SOS", &[("Mode", C("Morse"))], || b"sos".to_vec()),
    pd("Poem", &[("Rate", C("1/8"))], || b"the sea is calm tonight".to_vec()),
];
static LORENZ: &[Preset] = &[p("Butterfly", &[("Rho", N(28.0))]), p("Calm", &[("Rho", N(22.0)), ("Speed", N(0.15))]), p("Wild", &[("Rho", N(38.0)), ("Speed", N(0.7))])];
static LOOPS: &[Preset] = &[p("Tape Loops", &[]), p("Slow", &[("Speed", N(0.5)), ("Length", N(10.0))]), p("Many Voices", &[("Voices", N(7.0))]), p("Low", &[("Base Note", N(48.0))])];
static BLOOM: &[Preset] = &[p("Pond", &[]), p("Long Echoes", &[("Echoes", N(10.0)), ("Decay", N(0.9))]), p("By Itself", &[("By Itself", N(2.0))]), p("Looping Taps", &[("Taps", C("Loop"))])];
static CHIMES: &[Preset] = &[p("Breeze", &[("Wind", N(0.3))]), p("Windy", &[("Wind", N(0.8)), ("Gusts", N(0.9))]), p("Low Chimes", &[("Base Note", N(60.0)), ("Octaves", N(1.0))])];
static DRIFT_CHORDS: &[Preset] = &[p("Slow Drift", &[("Change", C("4 Bars"))]), p("Restless", &[("Change", C("1 Bar")), ("Movement", N(0.8))]), p("Wide Voicing", &[("Spread", N(4.0)), ("Voices", N(6.0))])];
static TINTINNABULI: &[Preset] = &[p("Bells", &[]), p("Alternating", &[("Position", C("Alternating"))]), p("Slow Line", &[("Rate", C("1/4")), ("Length", N(1.2))])];
static GLASS: &[Preset] = &[
    p("Opening", &[]),
    p("Metamorphosis", &[("Figure", C("Rocking")), ("Chords", C("1 6")), ("Repeats", N(4.0)), ("Rate", C("1/8"))]),
    p("Additive", &[("Figure", C("Additive")), ("Chords", C("1 7 6 5"))]),
    p("Broken Organ", &[("Figure", C("Broken")), ("Chords", C("1 6 3 7")), ("Rate", C("1/16")), ("Repeats", N(8.0)), ("Sevenths", N(1.0))]),
    p("Pulsing Chords", &[("Figure", C("Pulse")), ("Chords", C("1 4 1 5")), ("Repeats", N(8.0)), ("Accent", N(0.6))]),
    p("Circle", &[("Figure", C("Broken")), ("Chords", C("Circle")), ("Repeats", N(2.0)), ("Octaves", N(3.0))]),
];
static SATIE: &[Preset] = &[
    p("Gymnopédie", &[]),
    p("Gnossienne", &[("Style", C("Gnossienne")), ("Harmony", C("Minor")), ("Rubato", N(0.45))]),
    p("Sarabande", &[("Style", C("Sarabande")), ("Pedal", N(0.8))]),
    p("Lydian Waltz", &[("Harmony", C("Lydian")), ("Wander", N(0.5))]),
    p("Quiet Waltz", &[("Melody", N(0.4)), ("Velocity", N(0.4)), ("Pedal", N(0.9))]),
    p("Gymnopédie AABA", &[("Form", C("AABA")), ("Melody", N(1.0)), ("Variation", N(3.0))]),
    p("Broken Lullaby", &[("Left Hand", C("Broken")), ("Color", C("Sixth")), ("Contour", C("Arch")), ("Pedal", N(0.8)), ("Form", C("AB"))]),
    p("Your Own Chords", &[("Harmony", C("Custom")), ("Chord 1", C("II")), ("Chord 2", C("V")), ("Chord 3", C("I")), ("Chord 4", C("VI")), ("Form", C("ABAC"))]),
    p("Ornate Gnossienne", &[("Style", C("Gnossienne")), ("Harmony", C("Minor")), ("Ornaments", N(0.8)), ("Motion", N(0.7)), ("Rubato", N(0.5))]),
    p("Rising Waltz", &[("Contour", C("Rising")), ("Range", N(9.0)), ("Phrase", N(2.0)), ("Intro", N(2.0))]),
    p("Bare Line", &[("Left Hand", C("Bass")), ("Melody", N(1.0)), ("Motion", N(0.3)), ("Intro", N(1.0))]),
    p("Drone Waltz", &[("Harmony", C("Drone")), ("Color", C("Ninth")), ("Contour", C("Wave")), ("Octave", N(1.0))]),
    p("Slow Chords", &[("Chord Change", C("4 Bars")), ("Left Hand", C("Block")), ("Harmony", C("Circle")), ("Motion", N(0.2))]),
];
static RILEY: &[Preset] = &[
    p("In C", &[]),
    p("Ensemble", &[("Players", N(5.0)), ("Spread", N(0.8)), ("Patience", N(0.7))]),
    p("Restless", &[("Players", N(3.0)), ("Patience", N(0.15))]),
    p("Rainbow", &[("Style", C("Rainbow")), ("Rate", C("1/16")), ("Pulse", N(0.0)), ("Players", N(3.0))]),
    p("Curved Air", &[("Style", C("Rainbow")), ("Rate", C("1/16")), ("Players", N(5.0)), ("Spread", N(0.8)), ("Patience", N(0.8))]),
];
static REICH: &[Preset] = &[
    p("Phase", &[]),
    p("Slow Phase", &[("Repeats", N(16.0)), ("Pattern", N(3.0))]),
    p("Shifting Rhythm", &[("Style", C("Shifting Rhythm")), ("Rate", C("1/8")), ("Repeats", N(4.0))]),
    p("Pulses", &[("Style", C("Pulses")), ("Rate", C("1/8")), ("Repeats", N(4.0))]),
];
static BACH: &[Preset] = &[
    p("Prelude", &[]),
    p("Circle Prelude", &[("Harmony", C("Circle"))]),
    p("Invention", &[("Style", C("Invention"))]),
    p("Lament Invention", &[("Style", C("Invention")), ("Harmony", C("Lament")), ("Variation", N(5.0))]),
    p("Chorale", &[("Style", C("Chorale")), ("Gate", N(0.95))]),
];
static NANCARROW: &[Preset] = &[
    p("Canon 3:4:5", &[]),
    p("Canon 3:4", &[("Ratios", C("3:4"))]),
    p("Four Voices", &[("Ratios", C("4:5:6:7")), ("Length", N(24.0)), ("Spread", N(0.8))]),
    p("Irrational", &[("Ratios", C("Irrational")), ("Length", N(20.0))]),
    p("Drifting Apart", &[("Converge", N(0.0)), ("Ratios", C("2:3"))]),
    p("Player Piano", &[("Rate", C("1/16")), ("Ratios", C("4:5:6:7")), ("Gate", N(0.5))]),
];
static DEBUSSY: &[Preset] = &[
    p("Planing Ninths", &[]),
    p("Planing Sevenths", &[("Shape", C("Major Seventh"))]),
    p("Fourths", &[("Shape", C("Fourths")), ("Motion", N(0.7))]),
    p("Whole Tone", &[("Style", C("Whole Tone"))]),
    p("Arabesque", &[("Style", C("Arabesque"))]),
    p("Pentatonic", &[("Style", C("Pentatonic")), ("Motion", N(0.7))]),
];
static FELDMAN: &[Preset] = &[
    p("Soft Patterns", &[]),
    p("Rothko Chapel", &[("Density", N(0.2)), ("Cluster", N(4.0)), ("Color", N(0.3)), ("Rate", C("1/2"))]),
    p("Single Notes", &[("Cluster", N(1.0)), ("Density", N(0.5)), ("Color", N(0.7))]),
    p("Restless Repeats", &[("Repeat", N(1.0)), ("Density", N(0.6))]),
];
static MOZART: &[Preset] = &[
    p("Dice Minuet", &[]),
    p("Same Throw", &[("Reroll", N(0.0)), ("Dice", N(7.0))]),
    p("Sonata", &[("Style", C("Sonata"))]),
    p("Sonata Facile", &[("Style", C("Sonata")), ("Reroll", N(0.0)), ("Dice", N(3.0)), ("Gate", N(0.7))]),
    p("Alla Turca", &[("Style", C("Alla Turca")), ("Gate", N(0.6))]),
];
static CHOPIN: &[Preset] = &[
    p("Nocturne", &[]),
    p("Nocturne, Ornate", &[("Ornaments", N(0.9)), ("Rubato", N(0.7))]),
    p("Waltz", &[("Style", C("Waltz")), ("Rate", C("1/8")), ("Harmony", C("Simple"))]),
    p("Prelude", &[("Style", C("Prelude")), ("Pedal", N(0.4)), ("Velocity", N(0.5))]),
    p("Circle Nocturne", &[("Harmony", C("Circle")), ("Rubato", N(0.3))]),
];
static RAVEL: &[Preset] = &[
    p("Boléro", &[]),
    p("Short Boléro", &[("Length", N(32.0))]),
    p("Long Boléro", &[("Length", N(128.0)), ("Theme", N(3.0))]),
    p("Chord Rhythm", &[("Rhythm", C("Chord")), ("Theme", N(5.0))]),
    p("Tune Only", &[("Rhythm", C("Off")), ("Bass", N(0.0)), ("Layers", N(3.0))]),
];
static BARTOK: &[Preset] = &[
    p("Bulgarian 7/8", &[]),
    p("Bulgarian 9/8", &[("Metre", C("2+2+2+3")), ("Variation", N(4.0))]),
    p("Dance 3+2+2", &[("Metre", C("3+2+2")), ("Variation", N(9.0))]),
    p("Five", &[("Metre", C("2+3")), ("Mode", C("In Key"))]),
    p("Night Music", &[("Style", C("Night Music")), ("Velocity", N(0.6))]),
    p("Mirror", &[("Style", C("Mirror")), ("Base Note", N(66.0))]),
];
static MESSIAEN: &[Preset] = &[
    p("Modes", &[]),
    p("Louange", &[("Density", N(0.1)), ("Mode", C("Mode 6"))]),
    p("Blackbird", &[("Style", C("Birdsong")), ("Bird", C("Blackbird"))]),
    p("Song Thrush", &[("Style", C("Birdsong")), ("Bird", C("Thrush")), ("Density", N(0.6))]),
    p("Nightingale", &[("Style", C("Birdsong")), ("Bird", C("Nightingale"))]),
    p("Rhythmic Characters", &[("Style", C("Rhythms")), ("Mode", C("Mode 3"))]),
];
static LIGETI: &[Preset] = &[
    p("Metronomes", &[]),
    p("Sixteen Metronomes", &[("Voices", N(16.0)), ("Spread", N(0.9)), ("Gate", N(0.2))]),
    p("Continuum", &[("Style", C("Continuum")), ("Rate", C("1/32")), ("Voices", N(8.0))]),
    p("Slow Continuum", &[("Style", C("Continuum")), ("Change", N(0.2))]),
    p("Musica Ricercata", &[("Style", C("Ricercata")), ("Gate", N(0.6))]),
];
static CAGE: &[Preset] = &[
    p("Music of Changes", &[]),
    p("More Silence", &[("Silence", N(0.7)), ("Coins", N(12.0))]),
    p("Restless Chart", &[("Change", N(1.0)), ("Silence", N(0.2)), ("Coins", N(31.0))]),
    p("Sonatas and Interludes", &[("Style", C("Prepared")), ("Velocity", N(0.8))]),
    p("4'33\"", &[("Style", C("4'33\""))]),
];
static EINAUDI: &[Preset] = &[
    p("Nuvole", &[]),
    p("Minor Arpeggio", &[("Chords", C("i VI III VII")), ("Melody", N(0.8))]),
    p("Pop Chords", &[("Chords", C("I V vi IV")), ("Pedal", N(0.6))]),
    p("Experience", &[("Style", C("Build")), ("Chords", C("i VI III VII"))]),
    p("Build in Major", &[("Style", C("Build")), ("Chords", C("I V vi IV"))]),
];
static ORGAN_GRINDER: &[Preset] = &[
    p("Waltz", &[]),
    p("March", &[("Style", C("March")), ("Drums", N(0.8))]),
    p("Polka", &[("Style", C("Polka")), ("Drums", N(0.8)), ("Tune", N(5.0))]),
    p("Foxtrot", &[("Style", C("Foxtrot")), ("Tune", N(3.0))]),
    p("Fancy Book", &[("Ornaments", N(0.9)), ("Counter", N(0.8)), ("Tune", N(12.0))]),
    p("Plain Tune", &[("Counter", N(0.0)), ("Ornaments", N(0.0))]),
    p("Fair March", &[("Style", C("March")), ("Drums", N(1.0)), ("Ornaments", N(0.7)), ("Tune", N(7.0))]),
    p("Quiet Waltz", &[("Drums", N(0.2)), ("Counter", N(0.7)), ("Velocity", N(0.6)), ("Tune", N(21.0))]),
];
static MUSIC_BOX: &[Preset] = &[
    p("Lullaby", &[]),
    p("Arpeggios", &[("Accomp", C("Arpeggio")), ("Tune", N(4.0))]),
    p("Alberti", &[("Accomp", C("Alberti")), ("Metre", C("4/4")), ("Tune", N(9.0))]),
    p("Small Box", &[("Comb", C("18 Notes")), ("Sparkle", N(0.2))]),
    p("Grand Box", &[("Comb", C("Full")), ("Sparkle", N(0.7)), ("Accomp", C("Arpeggio"))]),
    p("Running Down", &[("Wind", N(0.9))]),
    p("Wound Up", &[("Wind", N(0.0))]),
];
static PEDAL: &[Preset] = &[p("Low Drone", &[("Change", C("Never"))]), p("Organ Pedal", &[("Octave", N(1.0)), ("Fifth", N(1.0))]), p("Moving Bass", &[("Change", C("1 Bar")), ("Moves", N(0.7))])];
static DRUMMER: &[Preset] = &[
    p("Steady", &[("Complexity", N(0.2)), ("Energy", N(0.5))]),
    p("Busy", &[("Complexity", N(0.7)), ("Energy", N(0.8))]),
    p("Shuffle", &[("Swing", N(0.6)), ("Swing On", C("8ths"))]),
    p("Fills Every Bar", &[("Fill Every", C("2 Bars")), ("Fill Length", C("1 Bar"))]),
    p("Laid Back", &[("Energy", N(0.3)), ("Humanize", N(0.6))]),
];
static DRUM_MAP: &[Preset] = &[p("Straight", &[("Map X", N(0.0)), ("Map Y", N(0.5))]), p("Broken", &[("Map X", N(0.6)), ("Map Y", N(0.3)), ("Chaos", N(0.3))]), p("Dense", &[("Kick Fill", N(0.8)), ("Snare Fill", N(0.7)), ("Hat Fill", N(0.9))]), p("Swung", &[("Swing", N(0.4))])];
static MARBLES: &[Preset] = &[p("Coin Toss", &[("Model", C("Coin Toss"))]), p("Clusters", &[("Model", C("Clusters"))]), p("Drums", &[("Model", C("Drums"))]), p("Melody", &[("Play", C("Notes")), ("Deja Vu", N(0.6))])];
static CHANCE: &[Preset] = &[p("Probable", &[]), p("Sparse", &[("Chance", N(0.5))]), p("Swung", &[("Swing", N(0.4))]), p("Fills", &[("Fill Every", C("2 Bars"))])];
static BITS: &[Preset] = &[p("Classic", &[]), p("Busy", &[("Formula", C("t*(t>>a|t>>b)")), ("A", N(3.0))]), p("Sparse", &[("Formula", C("t&t>>a")), ("A", N(2.0))]), p("Looping", &[("Loop", C("2 Bars"))])];
static PERCUSSION: &[Preset] = &[
    p("Son Montuno", &[("Style", C("Son Montuno"))]),
    p("Bossa Nova", &[("Style", C("Bossa Nova"))]),
    p("Samba", &[("Style", C("Samba"))]),
    p("Cumbia", &[("Style", C("Cumbia"))]),
    p("Bembe", &[("Style", C("Bembe 6/8"))]),
];
static SPINNER: &[Preset] = &[p("Gentle Wheel", &[("Spin", N(0.5)), ("Gravity", N(0.3))]), p("Fast Hexagon", &[("Spin", N(2.0)), ("Sides", N(6.0))]), p("Triangle", &[("Sides", N(3.0)), ("Spin", N(1.0))]), p("Floating", &[("Gravity", N(0.0)), ("Spin", N(0.8)), ("Bounce", N(1.0))])];
static ENDLESS: &[Preset] = &[p("Sixteenths", &[("Rate", C("1/16"))]), p("Eighths", &[("Rate", C("1/8"))]), p("Staccato", &[("Gate", N(0.2))])];
static HANDS: &[Preset] = &[p("Four on the Floor", &[("Pattern", C("Four"))]), p("Groove per Key", &[("Per Key", N(1.0))]), p("Clave", &[("Pattern", C("Clave"))]), p("Gallop", &[("Pattern", C("Gallop"))])];
static DRAW: &[Preset] = &[
    pd("Rise", &[("Mode", C("Every Step"))], || line(|x| x)),
    pd("Fall", &[("Mode", C("Every Step"))], || line(|x| 1.0 - x)),
    pd("Wave", &[], || line(|x| 0.5 + 0.45 * (x * std::f32::consts::TAU).sin())),
    pd("Mountain", &[], || line(|x| 1.0 - (2.0 * x - 1.0).abs())),
    pd("Zigzag", &[("Mode", C("Every Step")), ("Rate", C("1/16"))], || line(|x| (x * 8.0).fract() * 0.8 + x * 0.2)),
    pd("Stairs", &[], || line(|x| (x * 4.0).floor() / 3.0)),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Every preset names parameters (and choices) its device has.
    #[test]
    fn a_preset_keeps_the_keys_switches() {
        let kind = MidiFxKind::Progression;
        let specs = kind.specs();
        let band = specs.iter().position(|s| s.name == "Band").unwrap();
        let mut current = crate::params::defaults(specs);
        current[band] = 1.0;
        let pr = &midi(kind)[0];
        assert_eq!(apply_to(specs, pr, &current)[band], 1.0);
        assert_eq!(apply(specs, pr)[band], 0.0);
    }

    #[test]
    fn every_preset_fits_its_device() {
        let check = |what: String, specs: &[ParamSpec], presets: &[Preset]| {
            for pr in presets {
                for (name, v) in pr.set {
                    let spec = specs.iter().find(|s| s.name == *name).unwrap_or_else(|| panic!("{what} / {}: no parameter {name}", pr.name));
                    if let C(label) = v {
                        let ParamKind::Choice(opts) = spec.kind else { panic!("{what} / {}: {name} is no choice", pr.name) };
                        assert!(opts.contains(label), "{what} / {}: {name} has no {label}", pr.name);
                    }
                }
                let params = apply(specs, pr);
                assert_eq!(params.len(), specs.len());
                if let Some(data) = pr.data {
                    assert!(!data().is_empty(), "{what} / {}", pr.name);
                }
            }
        };
        for k in EffectKind::ALL {
            check(format!("{k:?}"), k.specs(), effect(k));
        }
        for k in MidiFxKind::ALL.iter().chain(MidiFxKind::SEQUENCERS.iter()) {
            check(format!("{k:?}"), k.specs(), midi(*k));
        }
    }

    #[test]
    fn a_preset_sets_its_values() {
        let pr = &DELAY[1];
        let params = apply(EffectKind::Delay.specs(), pr);
        let time = EffectKind::Delay.specs().iter().position(|s| s.name == "Time").unwrap();
        assert_eq!(params[time], 4.0, "1/8 D is the fifth time");
        let fb = EffectKind::Delay.specs().iter().position(|s| s.name == "Feedback").unwrap();
        assert!((params[fb] - 0.45).abs() < 1e-6);
    }

    #[test]
    fn beats_and_progressions_have_their_shape() {
        let b = beat(["X...", "", "", "", "", "", "", ""]);
        assert_eq!(b.len(), DRUM_BYTES);
        assert_eq!(b[0], 2);
        assert_eq!(&b[DRUM_LANES * 16..], &DRUM_NOTES);
        let s = strum(POP, "Travis Picking");
        assert_eq!(s.len(), PROG_SLOTS * PROG_BYTES + 16);
        assert_eq!(&s[..3], &[1, 0, 0]);
    }
}
