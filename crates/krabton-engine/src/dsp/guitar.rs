//! Guitar: a six-string guitar, played the way a guitarist would play it.
//!
//! Every string is a digital waveguide in the single-delay-loop form of the
//! extended Karplus-Strong algorithm (Jaffe & Smith): a delay line read
//! through a third-order Lagrange interpolator for exact tuning, a one-pole
//! loss filter fitted to a decay time for the low and for the high
//! frequencies (Sustain, Brightness), and a chain of identical first-order
//! all-passes for the dispersion of a stiff string (four sections on the
//! wound strings, two on the plain ones), so the upper partials run a little
//! sharp. Each string has two loops for the two polarisations of its motion,
//! slightly detuned and with different decay: the slow beating and the
//! two-stage decay of a real string. Hard plucks raise the tension, so a
//! note starts a little sharp and settles.
//!
//! The pluck is one period of the force a released string puts on the
//! bridge: a rectangle whose duty cycle is the pick position (the comb
//! filtering along the string), its corners rounded by the pick or the
//! finger (harder and faster is brighter), plus a little pick noise. Notes
//! are placed on strings and frets as a player would (open strings or low
//! frets close to where the hand is), a new note on a sounding string stops
//! the old one, and chords are strummed: spread over the Strum time from low
//! to high, high to low or alternating, with a little human timing and
//! velocity. Releasing a key lifts the finger: the string is damped within
//! a moment, with a touch of finger noise.
//!
//! The acoustic models send the bridge through a bank of body modes (the
//! air resonance, top and back plate modes and a brighter, denser top) and
//! a few early reflections. The electric models hear the string through a
//! magnetic pickup: a comb for its distance from the bridge (two for the
//! coils of a humbucker) and the resonance of the coil's inductance against
//! the cable, swept by the tone control.
//!
//! Its relatives use the same strings with their own bodies and tunings: a
//! resonator (a metal cone: nasal and ringing), a carved archtop, a
//! twelve-string (the lower four courses with an octave string, the upper
//! two doubled), a baritone (a fourth lower), an electric bass (four wound
//! strings into a pickup), a ukulele (four nylon strings, the high G), a
//! banjo (a drum head for a body, five strings with the short drone string)
//! and a mandolin (four doubled courses in fifths).

use super::biquad::{self, Coefs, State};
use super::{db_to_gain, midi_to_freq, pan_gains, DcBlock, OnePole, Rng, Svf, TAU};
use crate::params::{ParamSpec, Unit};

pub const MODEL: usize = 0;
pub const TUNING: usize = 1;
pub const CAPO: usize = 2;
pub const POSITION: usize = 3;
pub const PLUCK: usize = 4;
pub const PICK_POSITION: usize = 5;
pub const HARDNESS: usize = 6;
pub const VELOCITY: usize = 7;
pub const BRIGHTNESS: usize = 8;
pub const SUSTAIN: usize = 9;
pub const STIFFNESS: usize = 10;
pub const BEATING: usize = 11;
pub const PALM_MUTE: usize = 12;
pub const LET_RING: usize = 13;
pub const RELEASE_NOISE: usize = 14;
pub const SLIDE: usize = 15;
pub const STRUM: usize = 16;
pub const STRUM_DIRECTION: usize = 17;
pub const HUMANIZE: usize = 18;
pub const PICKUP: usize = 19;
pub const TONE: usize = 20;
pub const BODY: usize = 21;
pub const WIDTH: usize = 22;
pub const VOLUME: usize = 23;
pub const STRING_MODE: usize = 24;
pub const ONE_STRING: usize = 25;

/// How notes find their strings. Like a Player: open strings and the
/// position of the hand, the way a guitarist thinks. Lowest Fret: always the
/// string where the note sits lowest on the neck. In Position: the hand
/// stays at Position. One String: everything on one string (guitarists'
/// numbering: 1 is the high E), a slide guitar.
pub const STRING_MODES: &[&str] = &["Like a Player", "Lowest Fret", "In Position", "One String"];

pub const MODELS: &[&str] = &["Acoustic Steel", "Acoustic Nylon", "Electric Clean", "Electric Humbucker", "Resonator", "Archtop", "Twelve-String", "Baritone", "Bass", "Ukulele", "Banjo", "Mandolin"];
pub const TUNINGS: &[&str] = &["Standard", "Drop D", "DADGAD", "Open G", "Open D"];
pub const PLUCKS: &[&str] = &["Pick", "Finger"];
pub const DIRECTIONS: &[&str] = &["Down", "Up", "Alternate"];
pub const PICKUPS: &[&str] = &["Neck", "Middle", "Bridge"];

/// Open strings (MIDI notes, low to high) for each tuning.
pub const TUNING_NOTES: [[u8; 6]; 5] = [
    [40, 45, 50, 55, 59, 64], // E A D G B E
    [38, 45, 50, 55, 59, 64], // D A D G B E
    [38, 45, 50, 55, 57, 62], // D A D G A D
    [38, 43, 50, 55, 59, 62], // D G D G B D
    [38, 45, 50, 54, 57, 62], // D A D F# A D
];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Model", "Guitar", MODELS, 0),
    ParamSpec::choice("Tuning", "Guitar", TUNINGS, 0),
    ParamSpec::int("Capo", "Guitar", 0.0, 12.0, 0.0, Unit::None),
    ParamSpec::int("Position", "Guitar", 0.0, 12.0, 0.0, Unit::None),
    ParamSpec::choice("Pluck", "Pick", PLUCKS, 0),
    ParamSpec::lin("Pick Position", "Pick", 0.03, 0.5, 0.13, Unit::Percent),
    ParamSpec::lin("Hardness", "Pick", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Velocity", "Pick", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Brightness", "String", 0.0, 1.0, 0.55, Unit::Percent),
    ParamSpec::log("Sustain", "String", 0.3, 20.0, 6.0, Unit::Sec),
    ParamSpec::lin("Stiffness", "String", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Beating", "String", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Palm Mute", "Hand", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Let Ring", "Hand", false),
    ParamSpec::lin("Release Noise", "Hand", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Slide", "Hand", 0.0, 400.0, 0.0, Unit::Ms),
    ParamSpec::lin("Strum", "Strum", 0.0, 60.0, 15.0, Unit::Ms),
    ParamSpec::choice("Direction", "Strum", DIRECTIONS, 0),
    ParamSpec::lin("Humanize", "Strum", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::choice("Pickup", "Body", PICKUPS, 1),
    ParamSpec::lin("Tone", "Body", 0.0, 1.0, 0.65, Unit::Percent),
    ParamSpec::lin("Body", "Body", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Width", "Output", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Volume", "Output", -24.0, 12.0, 0.0, Unit::Db),
    ParamSpec::choice("Strings", "Hand", STRING_MODES, 0),
    ParamSpec::int("One String", "Hand", 1.0, 6.0, 3.0, Unit::None),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Steel Acoustic",
        "Nylon Classical",
        "Fingerpicked",
        "Strummed Chords",
        "DADGAD Drone",
        "Clean Electric",
        "Funky Neck",
        "Jazz Box",
        "Bridge Bite",
        "Palm Muted",
        "Drop D Chug",
        "Open G Slide",
        "Resonator Slide",
        "Jazz Archtop",
        "Twelve-String Jangle",
        "Baritone Twang",
        "Finger Bass",
        "Picked Bass",
        "Muted Bass",
        "Bright Bass",
        "Ukulele",
        "Strummed Ukulele",
        "Soft Ukulele",
        "Banjo",
        "Clawhammer Banjo",
        "Bright Banjo",
        "Mandolin",
        "Soft Mandolin",
        "Bright Mandolin",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    let (nylon, clean, humbucker) = (NYLON as f32, CLEAN as f32, HUMBUCKER as f32);
    let m = |k: usize| k as f32;
    let (neck, bridge) = (0.0, 2.0);
    let (finger, on, alternate) = (1.0, 1.0, 2.0);
    match name {
        // The defaults are a steel-string flat top.
        "Nylon Classical" => set(&[(MODEL, nylon), (PLUCK, finger), (PICK_POSITION, 0.2), (HARDNESS, 0.4), (BRIGHTNESS, 0.4), (SUSTAIN, 4.5), (STIFFNESS, 0.3), (STRUM, 22.0), (BODY, 0.85), (TONE, 0.55), (RELEASE_NOISE, 0.15)]),
        "Fingerpicked" => set(&[(PLUCK, finger), (PICK_POSITION, 0.18), (HARDNESS, 0.65), (LET_RING, on), (STRUM, 0.0), (BRIGHTNESS, 0.5), (SUSTAIN, 7.0), (BODY, 0.75), (HUMANIZE, 0.35)]),
        "Strummed Chords" => set(&[(PICK_POSITION, 0.11), (HARDNESS, 0.6), (STRUM, 32.0), (STRUM_DIRECTION, alternate), (HUMANIZE, 0.5), (BRIGHTNESS, 0.6), (BODY, 0.75), (WIDTH, 0.8)]),
        "DADGAD Drone" => set(&[(TUNING, 2.0), (PLUCK, finger), (LET_RING, on), (HARDNESS, 0.55), (SUSTAIN, 9.0), (STRUM, 45.0), (BODY, 0.8), (WIDTH, 0.8)]),
        "Clean Electric" => set(&[(MODEL, clean), (TONE, 0.75), (SUSTAIN, 8.0), (BRIGHTNESS, 0.62), (STRUM, 12.0), (BODY, 0.2), (WIDTH, 0.4)]),
        "Funky Neck" => set(&[(MODEL, clean), (PICKUP, neck), (POSITION, 7.0), (HARDNESS, 0.7), (SUSTAIN, 2.5), (BRIGHTNESS, 0.66), (STRUM, 8.0), (STRUM_DIRECTION, alternate), (RELEASE_NOISE, 0.55), (PALM_MUTE, 0.1), (TONE, 0.8), (BODY, 0.15), (WIDTH, 0.4)]),
        "Jazz Box" => set(&[(MODEL, humbucker), (PICKUP, neck), (PLUCK, finger), (HARDNESS, 0.45), (TONE, 0.3), (BRIGHTNESS, 0.35), (SUSTAIN, 5.0), (STIFFNESS, 0.35), (STRUM, 14.0), (BODY, 0.25), (WIDTH, 0.4)]),
        "Bridge Bite" => set(&[(MODEL, humbucker), (PICKUP, bridge), (TONE, 0.9), (HARDNESS, 0.75), (BRIGHTNESS, 0.7), (SUSTAIN, 10.0), (STRUM, 10.0), (BODY, 0.1), (WIDTH, 0.3)]),
        "Palm Muted" => set(&[(MODEL, humbucker), (PICKUP, bridge), (PALM_MUTE, 0.75), (HARDNESS, 0.7), (STRUM, 6.0), (TONE, 0.8), (BODY, 0.1), (WIDTH, 0.3)]),
        // Clean, meant for the drive pedals and the amp.
        "Drop D Chug" => set(&[(MODEL, humbucker), (TUNING, 1.0), (PICKUP, bridge), (PALM_MUTE, 0.55), (HARDNESS, 0.8), (STRUM, 5.0), (HUMANIZE, 0.25), (TONE, 0.85), (BODY, 0.05), (WIDTH, 0.25), (VOLUME, 2.0)]),
        "Open G Slide" => set(&[(MODEL, clean), (TUNING, 3.0), (PICKUP, bridge), (SLIDE, 140.0), (SUSTAIN, 10.0), (HARDNESS, 0.55), (STRUM, 20.0), (BODY, 0.25), (WIDTH, 0.4)]),
        "Resonator Slide" => set(&[(MODEL, m(RESONATOR)), (TUNING, 4.0), (SLIDE, 160.0), (PLUCK, finger), (HARDNESS, 0.65), (SUSTAIN, 7.0), (BRIGHTNESS, 0.62), (STRUM, 18.0), (BODY, 0.85)]),
        "Jazz Archtop" => set(&[(MODEL, m(ARCHTOP)), (HARDNESS, 0.4), (BRIGHTNESS, 0.42), (SUSTAIN, 4.0), (STRUM, 12.0), (BODY, 0.8), (TONE, 0.5), (POSITION, 5.0)]),
        "Twelve-String Jangle" => set(&[(MODEL, m(TWELVE)), (BEATING, 0.5), (HARDNESS, 0.6), (STRUM, 28.0), (STRUM_DIRECTION, alternate), (BRIGHTNESS, 0.62), (SUSTAIN, 7.0), (BODY, 0.75), (WIDTH, 0.85)]),
        "Baritone Twang" => set(&[(MODEL, m(BARITONE)), (HARDNESS, 0.6), (BRIGHTNESS, 0.55), (SUSTAIN, 8.0), (STRUM, 20.0), (BODY, 0.7)]),
        "Finger Bass" => set(&[(MODEL, m(BASS)), (PLUCK, finger), (PICKUP, neck), (HARDNESS, 0.45), (BRIGHTNESS, 0.4), (SUSTAIN, 6.0), (STRUM, 0.0), (TONE, 0.5), (BODY, 0.05), (WIDTH, 0.1), (RELEASE_NOISE, 0.4)]),
        "Picked Bass" => set(&[(MODEL, m(BASS)), (PICKUP, bridge), (HARDNESS, 0.75), (BRIGHTNESS, 0.55), (SUSTAIN, 5.0), (STRUM, 0.0), (TONE, 0.75), (PALM_MUTE, 0.15), (BODY, 0.05), (WIDTH, 0.1)]),
        "Muted Bass" => set(&[(MODEL, m(BASS)), (PLUCK, finger), (PICKUP, neck), (PALM_MUTE, 0.6), (HARDNESS, 0.35), (BRIGHTNESS, 0.3), (SUSTAIN, 4.0), (STRUM, 0.0), (TONE, 0.35), (BODY, 0.05), (WIDTH, 0.1), (VOLUME, 3.0)]),
        "Bright Bass" => set(&[(MODEL, m(BASS)), (PLUCK, finger), (PICKUP, bridge), (HARDNESS, 0.7), (BRIGHTNESS, 0.62), (SUSTAIN, 7.0), (STIFFNESS, 0.6), (STRUM, 0.0), (TONE, 0.85), (BODY, 0.05), (WIDTH, 0.1)]),
        "Ukulele" => set(&[(MODEL, m(UKULELE)), (PLUCK, finger), (PICK_POSITION, 0.2), (HARDNESS, 0.5), (BRIGHTNESS, 0.5), (SUSTAIN, 2.5), (STIFFNESS, 0.3), (STRUM, 18.0), (STRUM_DIRECTION, alternate), (BODY, 0.85), (RELEASE_NOISE, 0.15)]),
        "Strummed Ukulele" => set(&[(MODEL, m(UKULELE)), (PICK_POSITION, 0.16), (HARDNESS, 0.6), (BRIGHTNESS, 0.55), (SUSTAIN, 2.0), (STIFFNESS, 0.3), (STRUM, 26.0), (STRUM_DIRECTION, alternate), (HUMANIZE, 0.45), (BODY, 0.85), (WIDTH, 0.75)]),
        "Soft Ukulele" => set(&[(MODEL, m(UKULELE)), (PLUCK, finger), (PICK_POSITION, 0.25), (HARDNESS, 0.3), (BRIGHTNESS, 0.38), (SUSTAIN, 3.0), (STIFFNESS, 0.25), (STRUM, 30.0), (BODY, 0.9), (RELEASE_NOISE, 0.1)]),
        "Clawhammer Banjo" => set(&[(MODEL, m(BANJO)), (PLUCK, finger), (PICK_POSITION, 0.14), (HARDNESS, 0.55), (BRIGHTNESS, 0.6), (SUSTAIN, 2.5), (STRUM, 14.0), (STRUM_DIRECTION, 0.0), (BODY, 0.85), (TONE, 0.55)]),
        "Bright Banjo" => set(&[(MODEL, m(BANJO)), (PICK_POSITION, 0.08), (HARDNESS, 0.9), (BRIGHTNESS, 0.8), (SUSTAIN, 1.6), (STRUM, 8.0), (BODY, 0.95), (TONE, 0.85)]),
        "Soft Mandolin" => set(&[(MODEL, m(MANDOLIN)), (PLUCK, finger), (PICK_POSITION, 0.18), (HARDNESS, 0.4), (BRIGHTNESS, 0.45), (SUSTAIN, 3.0), (BEATING, 0.4), (STRUM, 16.0), (BODY, 0.85), (WIDTH, 0.7)]),
        "Bright Mandolin" => set(&[(MODEL, m(MANDOLIN)), (PICK_POSITION, 0.09), (HARDNESS, 0.85), (BRIGHTNESS, 0.78), (SUSTAIN, 2.2), (BEATING, 0.55), (STRUM, 9.0), (BODY, 0.8), (TONE, 0.8), (WIDTH, 0.75)]),
        "Banjo" => set(&[(MODEL, m(BANJO)), (PLUCK, finger), (PICK_POSITION, 0.1), (HARDNESS, 0.8), (BRIGHTNESS, 0.7), (SUSTAIN, 2.0), (STRUM, 10.0), (BODY, 0.9), (TONE, 0.7)]),
        "Mandolin" => set(&[(MODEL, m(MANDOLIN)), (PICK_POSITION, 0.12), (HARDNESS, 0.7), (BRIGHTNESS, 0.65), (SUSTAIN, 2.5), (BEATING, 0.45), (STRUM, 12.0), (BODY, 0.8), (WIDTH, 0.7)]),
        _ => {}
    }
    p
}

const STRINGS: usize = 6;
const MAX_FRET: i32 = 20;
/// Samples per control step (pitch glides, loss and tuning updates).
const CTRL: usize = 16;
const MAX_AP: usize = 4;
/// The lowest frequency a string loop has room for.
const LOWEST_HZ: f32 = 30.0;
const QUEUE: usize = 16;
const N_MODES: usize = 14;

const STEEL: usize = 0;
const NYLON: usize = 1;
const CLEAN: usize = 2;
const HUMBUCKER: usize = 3;
const RESONATOR: usize = 4;
const ARCHTOP: usize = 5;
const TWELVE: usize = 6;
const BARITONE: usize = 7;
const BASS: usize = 8;
const UKULELE: usize = 9;
const BANJO: usize = 10;
const MANDOLIN: usize = 11;

/// A string the instrument does not have.
const NO_STRING: i32 = -1000;

/// Decay scaling (low frequencies, high frequencies) per model: nylon dies
/// sooner and much darker, a solid body sustains longer, a banjo's head
/// takes the energy out quickly and brightly.
const MODEL_DECAY: [(f32, f32); 12] = [(1.0, 1.0), (0.75, 0.45), (1.4, 1.1), (1.5, 1.0), (0.85, 1.3), (0.8, 0.75), (1.0, 1.05), (1.1, 0.9), (1.7, 0.55), (0.5, 0.55), (0.4, 1.5), (0.6, 1.2)];

/// Output level per model, so that a strummed chord is about as loud on
/// each (the pickups hear the strings much louder than the bodies do).
const MODEL_GAIN: [f32; 12] = [2.0, 2.6, 0.6, 0.6, 1.8, 2.8, 1.95, 2.0, 0.8, 4.6, 2.9, 3.5];

/// Whether a model is heard through a pickup.
pub fn is_electric(model: usize) -> bool {
    matches!(model, CLEAN | HUMBUCKER | BASS)
}

/// The open strings of a model (MIDI notes in the order a down stroke hits
/// them, None where it has no string), without a capo. The guitars take
/// the Tuning; the others have their own.
pub fn open_strings(model: usize, tuning: usize) -> [Option<u8>; 6] {
    let t = TUNING_NOTES[tuning.min(TUNING_NOTES.len() - 1)];
    match model {
        // A fourth below the guitar.
        BARITONE => t.map(|n| Some(n - 5)),
        // E A D G; Drop D lowers the E.
        BASS => [Some(if tuning == 1 { 26 } else { 28 }), Some(33), Some(38), Some(43), None, None],
        // The re-entrant G C E A.
        UKULELE => [Some(67), Some(60), Some(64), Some(69), None, None],
        // Open G with the short drone string first: g D G B D.
        BANJO => [Some(67), Some(50), Some(55), Some(59), Some(62), None],
        // G D A E in fifths.
        MANDOLIN => [Some(55), Some(62), Some(69), Some(76), None, None],
        _ => t.map(Some),
    }
}

/// What the instrument is called (for a new track): a Bass, a Ukulele...
pub fn family_name(p: &[f32]) -> &'static str {
    match p.get(MODEL).map(|v| v.round().max(0.0) as usize) {
        Some(BASS) => "Bass",
        Some(UKULELE) => "Ukulele",
        Some(BANJO) => "Banjo",
        Some(MANDOLIN) => "Mandolin",
        _ => "Guitar",
    }
}

/// The presets of the bass, the ukulele, the banjo and the mandolin, for the browser.
pub const BASS_PRESETS: &[&str] = &["Finger Bass", "Picked Bass", "Muted Bass", "Bright Bass"];
pub const UKULELE_PRESETS: &[&str] = &["Ukulele", "Strummed Ukulele", "Soft Ukulele"];
pub const BANJO_PRESETS: &[&str] = &["Banjo", "Clawhammer Banjo", "Bright Banjo"];
pub const MANDOLIN_PRESETS: &[&str] = &["Mandolin", "Soft Mandolin", "Bright Mandolin"];
pub const GUITAR_PRESETS: &[&str] = &[
    "Steel Acoustic",
    "Nylon Classical",
    "Fingerpicked",
    "Strummed Chords",
    "DADGAD Drone",
    "Clean Electric",
    "Funky Neck",
    "Jazz Box",
    "Bridge Bite",
    "Palm Muted",
    "Drop D Chug",
    "Open G Slide",
    "Resonator Slide",
    "Jazz Archtop",
    "Twelve-String Jangle",
    "Baritone Twang",
];

/// The Guitar's presets by instrument (the browser shows them in folders).
pub const FAMILY: &[(&str, &[&str])] = &[("Guitars", GUITAR_PRESETS), ("Bass", BASS_PRESETS), ("Ukulele", UKULELE_PRESETS), ("Banjo", BANJO_PRESETS), ("Mandolin", MANDOLIN_PRESETS)];

/// The name of what a model is tuned to: the Tuning, or its own.
pub fn tuning_name(model: usize, tuning: usize) -> &'static str {
    match model {
        BASS if tuning == 1 => "Drop D",
        BASS => "EADG",
        UKULELE => "GCEA",
        BANJO => "Open G",
        MANDOLIN => "GDAE",
        BARITONE => ["B Standard", "Drop A", "ADGCDG", "Open D", "Open A"][tuning.min(4)],
        _ => TUNINGS[tuning.min(TUNINGS.len() - 1)],
    }
}

/// Inharmonicity coefficients of the open strings at Stiffness 50 %.
const B_STEEL: [f32; 6] = [9e-5, 7e-5, 5e-5, 4e-5, 8e-5, 5e-5];
const B_ELECTRIC: [f32; 6] = [8e-5, 6e-5, 5e-5, 1.2e-4, 8e-5, 5e-5];
const B_NYLON: [f32; 6] = [3e-5, 2.5e-5, 2e-5, 1e-5, 1e-5, 1e-5];
const B_BASS: [f32; 6] = [2.4e-4, 2e-4, 1.6e-4, 1.3e-4, 1e-4, 1e-4];
const B_LIGHT: [f32; 6] = [4e-5, 6e-5, 4e-5, 3e-5, 3e-5, 3e-5];

/// Body modes (Hz, Q, gain) of a steel-string flat top: the air (Helmholtz)
/// resonance, the top and back plate modes, then a brighter, denser top.
const STEEL_MODES: [(f32, f32, f32); N_MODES] = [
    (102.0, 14.0, 1.0),
    (196.0, 18.0, -0.85),
    (226.0, 20.0, 0.5),
    (385.0, 16.0, -0.45),
    (450.0, 14.0, 0.35),
    (550.0, 14.0, 0.35),
    (700.0, 12.0, -0.3),
    (880.0, 11.0, 0.28),
    (1080.0, 10.0, -0.25),
    (1350.0, 9.0, 0.24),
    (1800.0, 8.0, 0.26),
    (2400.0, 7.0, -0.22),
    (3100.0, 6.0, 0.22),
    (4200.0, 5.0, 0.14),
];

/// A classical body: lower and softer, with less sparkle on top.
const NYLON_MODES: [(f32, f32, f32); N_MODES] = [
    (96.0, 12.0, 1.0),
    (185.0, 16.0, -0.9),
    (215.0, 18.0, 0.45),
    (290.0, 15.0, 0.38),
    (400.0, 14.0, -0.4),
    (520.0, 12.0, 0.3),
    (680.0, 11.0, -0.26),
    (850.0, 10.0, 0.22),
    (1100.0, 9.0, -0.17),
    (1450.0, 8.0, 0.15),
    (1900.0, 7.0, 0.12),
    (2600.0, 6.0, -0.08),
    (3300.0, 5.0, 0.05),
    (4200.0, 4.0, 0.03),
];

/// A resonator: the metal cone rings nasal and bright, its modes sharp.
const RESONATOR_MODES: [(f32, f32, f32); N_MODES] = [
    (180.0, 10.0, 0.5),
    (420.0, 24.0, 0.9),
    (640.0, 30.0, -0.7),
    (900.0, 28.0, 0.85),
    (1250.0, 30.0, -0.6),
    (1600.0, 26.0, 0.7),
    (2100.0, 22.0, 0.5),
    (2700.0, 20.0, -0.4),
    (3300.0, 18.0, 0.42),
    (4100.0, 15.0, 0.3),
    (5200.0, 12.0, 0.25),
    (6500.0, 10.0, -0.15),
    (7800.0, 9.0, 0.12),
    (9500.0, 8.0, 0.08),
];

/// A carved archtop: little air below, woody and focused in the mids.
const ARCHTOP_MODES: [(f32, f32, f32); N_MODES] = [
    (110.0, 12.0, 0.45),
    (210.0, 14.0, -0.7),
    (280.0, 16.0, 0.65),
    (380.0, 15.0, 0.6),
    (470.0, 14.0, -0.5),
    (600.0, 12.0, 0.5),
    (760.0, 11.0, -0.35),
    (950.0, 10.0, 0.38),
    (1200.0, 9.0, -0.3),
    (1500.0, 8.0, 0.3),
    (1950.0, 7.0, 0.2),
    (2500.0, 6.0, -0.15),
    (3200.0, 5.0, 0.1),
    (4000.0, 4.0, 0.06),
];

/// A ukulele: a small body, its air resonance high.
const UKULELE_MODES: [(f32, f32, f32); N_MODES] = [
    (240.0, 12.0, 0.8),
    (420.0, 16.0, -0.7),
    (520.0, 18.0, 0.65),
    (700.0, 14.0, 0.5),
    (880.0, 13.0, -0.4),
    (1050.0, 12.0, 0.4),
    (1300.0, 11.0, -0.3),
    (1600.0, 10.0, 0.28),
    (2000.0, 9.0, 0.22),
    (2500.0, 8.0, -0.2),
    (3100.0, 7.0, 0.15),
    (3800.0, 6.0, 0.1),
    (4600.0, 5.0, 0.06),
    (5500.0, 4.0, 0.04),
];

/// A banjo: a drum head, its modes at the ratios of a stretched membrane
/// (the zeros of the Bessel functions) and ringing sharply.
const BANJO_MODES: [(f32, f32, f32); N_MODES] = [
    (330.0, 30.0, 1.0),
    (526.0, 34.0, -0.85),
    (705.0, 36.0, 0.8),
    (758.0, 36.0, -0.6),
    (875.0, 38.0, 0.7),
    (963.0, 38.0, -0.55),
    (1041.0, 40.0, 0.6),
    (1155.0, 40.0, -0.5),
    (1340.0, 36.0, 0.5),
    (1520.0, 32.0, -0.45),
    (1900.0, 26.0, 0.45),
    (2600.0, 20.0, 0.4),
    (3400.0, 16.0, -0.3),
    (4500.0, 12.0, 0.3),
];

/// A mandolin: a small carved body, bright and quick.
const MANDOLIN_MODES: [(f32, f32, f32); N_MODES] = [
    (190.0, 10.0, 0.55),
    (380.0, 14.0, 0.8),
    (560.0, 16.0, -0.6),
    (700.0, 15.0, 0.6),
    (900.0, 14.0, -0.45),
    (1150.0, 12.0, 0.45),
    (1450.0, 11.0, -0.35),
    (1800.0, 10.0, 0.35),
    (2200.0, 9.0, 0.3),
    (2800.0, 8.0, -0.25),
    (3500.0, 7.0, 0.2),
    (4300.0, 6.0, 0.15),
    (5300.0, 5.0, 0.1),
    (6500.0, 4.0, 0.06),
];

/// The body of a model, and how much its modes are moved (a baritone is a
/// bigger steel-string body).
fn body_modes(model: usize) -> (&'static [(f32, f32, f32); N_MODES], f32) {
    match model {
        NYLON => (&NYLON_MODES, 1.0),
        RESONATOR => (&RESONATOR_MODES, 1.0),
        ARCHTOP => (&ARCHTOP_MODES, 1.0),
        BARITONE => (&STEEL_MODES, 0.85),
        UKULELE => (&UKULELE_MODES, 1.0),
        BANJO => (&BANJO_MODES, 1.0),
        MANDOLIN => (&MANDOLIN_MODES, 1.0),
        _ => (&STEEL_MODES, 1.0),
    }
}

/// Early reflections (seconds, gain) for each side: a small room.
const ROOM: [[(f32, f32); 6]; 2] = [
    [(0.0071, 0.5), (0.0113, -0.4), (0.0167, 0.33), (0.0219, -0.27), (0.0283, 0.22), (0.0347, -0.16)],
    [(0.0083, 0.5), (0.0127, -0.4), (0.0181, 0.33), (0.0241, -0.27), (0.0301, 0.2), (0.0361, -0.15)],
];

#[inline]
fn cents(c: f32) -> f32 {
    (c * (std::f32::consts::LN_2 / 1200.0)).exp()
}

/// Phase delay (samples) at `w` of the first-order all-pass (a + z^-1) / (1 + a z^-1).
fn ap_delay(a: f32, w: f32) -> f32 {
    let (s, c) = w.sin_cos();
    1.0 - 2.0 * (a * s).atan2(1.0 + a * c) / w
}

/// Phase delay (samples) at `w` of the loss filter g (1 - a) / (1 - a z^-1).
fn loss_delay(a: f32, w: f32) -> f32 {
    let (s, c) = w.sin_cos();
    (a * s).atan2(1.0 - a * c) / w
}

/// The pole of a one-pole loss filter that is `r` times weaker at `w` than
/// at DC.
fn loss_pole(r: f32, w: f32) -> f32 {
    let r2 = (r as f64).powi(2);
    let aa = 1.0 - r2;
    if aa < 1e-9 {
        return 0.0;
    }
    let bb = 1.0 - r2 * (w as f64).cos();
    let disc = (bb * bb - aa * aa).max(0.0).sqrt();
    ((bb - disc) / aa).clamp(0.0, 0.97) as f32
}

/// The coefficient for `stages` identical first-order all-passes that
/// stretch the partials of a string of inharmonicity `b` (partial n at
/// n f0 sqrt(1 + b n^2), relative to the first), using at most `room`
/// samples of delay at the fundamental.
fn dispersion_coef(b: f32, f0: f32, sr: f32, stages: usize, room: f32) -> f32 {
    let m = stages as f32;
    // The DC delay of a section is (1 - a) / (1 + a).
    let floor = ((m - room) / (m + room)).max(-0.95);
    if floor >= 0.0 {
        return 0.0;
    }
    let n = ((3000.0f32).min(0.3 * sr) / f0).floor().clamp(2.0, 40.0);
    let fnn = n * f0 * ((1.0 + b * n * n) / (1.0 + b)).sqrt();
    let want = sr / f0 * (1.0 - ((1.0 + b) / (1.0 + b * n * n)).sqrt());
    let (w1, wn) = (TAU * f0 / sr, (TAU * fnn / sr).min(3.0));
    let spread = |a: f32| m * (ap_delay(a, w1) - ap_delay(a, wn));
    if spread(floor) <= want {
        return floor;
    }
    let (mut lo, mut hi) = (floor, 0.0f32);
    for _ in 0..22 {
        let mid = 0.5 * (lo + hi);
        if spread(mid) > want {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// One polarisation of a string: delay line, dispersion and loss.
struct Loop {
    buf: Vec<f32>,
    mask: usize,
    w: usize,
    /// Read delay (samples) of the Lagrange interpolator.
    delay: f32,
    g: f32,
    a: f32,
    z: f32,
    ap: f32,
    n_ap: usize,
    ap_x: [f32; MAX_AP],
    ap_y: [f32; MAX_AP],
    /// The frequency and inharmonicity the all-pass was designed for.
    disp_f: f32,
    disp_b: f32,
}

impl Loop {
    fn new(len: usize) -> Self {
        let size = len.next_power_of_two();
        Self { buf: vec![0.0; size], mask: size - 1, w: 0, delay: 100.0, g: 0.99, a: 0.1, z: 0.0, ap: 0.0, n_ap: 0, ap_x: [0.0; MAX_AP], ap_y: [0.0; MAX_AP], disp_f: 0.0, disp_b: 0.0 }
    }

    fn clear(&mut self) {
        self.buf.fill(0.0);
        self.z = 0.0;
        self.ap_x = [0.0; MAX_AP];
        self.ap_y = [0.0; MAX_AP];
        self.disp_f = 0.0;
    }

    #[inline]
    fn tick(&mut self, input: f32) -> f32 {
        // Third-order Lagrange read with the fraction kept in [1, 2).
        let d = self.delay;
        let i = d as usize - 1;
        let f = d - i as f32;
        let m = self.mask;
        let base = self.w.wrapping_sub(i);
        let x0 = self.buf[base & m];
        let x1 = self.buf[base.wrapping_sub(1) & m];
        let x2 = self.buf[base.wrapping_sub(2) & m];
        let x3 = self.buf[base.wrapping_sub(3) & m];
        let (d1, d2, d3) = (f - 1.0, f - 2.0, f - 3.0);
        let mut v = -d1 * d2 * d3 * (1.0 / 6.0) * x0 + f * d2 * d3 * 0.5 * x1 - f * d1 * d3 * 0.5 * x2 + f * d1 * d2 * (1.0 / 6.0) * x3;
        // Stiffness: high frequencies travel faster.
        let c = self.ap;
        for k in 0..self.n_ap {
            let y = c * (v - self.ap_y[k]) + self.ap_x[k];
            self.ap_x[k] = v;
            self.ap_y[k] = y;
            v = y;
        }
        self.z = self.g * (1.0 - self.a) * v + self.a * self.z;
        let out = self.z + input;
        self.buf[self.w & m] = out;
        self.w = self.w.wrapping_add(1);
        out
    }
}

/// One note on a string. A string has two, so a new pluck can start fresh
/// while the old note is being stopped.
struct Slot {
    loops: [Loop; 2],
    active: bool,
    /// Being stopped by the pick or finger of the next note.
    fading: bool,
    fade: f32,
    /// The finger is lifted (or the palm is on it): damped.
    released: bool,
    freq: f32,
    target: f32,
    slide_ratio: f32,
    slide_steps: u32,
    /// Tension raised by the pluck, in cents; decays.
    glide: f32,
    vib_phase: f32,
    level: f32,
    /// Samples since the pluck (or the end of a slide).
    age: u32,
    b: f32,
    stages: usize,
    /// The second loop is the octave string of a twelve-string's course.
    octave: bool,
}

impl Slot {
    fn new(len: usize) -> Self {
        Self {
            loops: [Loop::new(len), Loop::new(len)],
            active: false,
            fading: false,
            fade: 1.0,
            released: false,
            freq: 110.0,
            target: 110.0,
            slide_ratio: 1.0,
            slide_steps: 0,
            glide: 0.0,
            vib_phase: 0.0,
            level: 0.0,
            age: 0,
            b: 0.0,
            stages: 0,
            octave: false,
        }
    }

    fn sounding(&self) -> bool {
        self.active && !self.fading
    }
}

struct GString {
    idx: usize,
    slots: [Slot; 2],
    cur: usize,
    /// The pitch it sounds (after moving out-of-range keys by octaves).
    note: Option<u8>,
    /// The key that played it, for its note-off.
    key: u8,
    /// Fret from the nut (a capo counts).
    fret: u8,
    held: bool,
    exc: Vec<f32>,
    exc_len: usize,
    exc_pos: usize,
    /// How the pluck divides between the two polarisations.
    exc_w: [f32; 2],
    /// Recent output, for the pickup comb.
    hist: Vec<f32>,
    hist_mask: usize,
    hist_w: usize,
    pick_d: [f32; 4],
    n_pick: usize,
    /// Finger noise at note-off.
    noise: f32,
    noise_k: f32,
    noise_lp: OnePole,
    noise_hp: OnePole,
    rng: Rng,
    gl: f32,
    gr: f32,
}

impl GString {
    fn new(idx: usize, sr: f32) -> Self {
        let len = (sr / LOWEST_HZ) as usize + 8;
        let hist_len = len.next_power_of_two();
        let pan = (idx as f32 - 2.5) / 2.5 * 0.5;
        let (gl, gr) = pan_gains(pan);
        let mut noise_lp = OnePole::default();
        noise_lp.set(3200.0, sr);
        let mut noise_hp = OnePole::default();
        noise_hp.set(500.0, sr);
        Self {
            idx,
            slots: [Slot::new(len), Slot::new(len)],
            cur: 0,
            note: None,
            key: 0,
            fret: 0,
            held: false,
            exc: vec![0.0; len],
            exc_len: 0,
            exc_pos: 0,
            exc_w: [0.6, 0.4],
            hist: vec![0.0; hist_len],
            hist_mask: hist_len - 1,
            hist_w: 0,
            pick_d: [0.0; 4],
            n_pick: 0,
            noise: 0.0,
            noise_k: (-1.0 / (0.02 * sr)).exp(),
            noise_lp,
            noise_hp,
            rng: Rng::new(0x6A17_0001 + 7919 * idx as u32),
            gl,
            gr,
        }
    }

    fn active(&self) -> bool {
        self.slots[0].active || self.slots[1].active || self.noise > 1e-6
    }

    /// Ringing (or held) loud enough to count as a sounding note.
    fn sounding(&self) -> bool {
        let s = &self.slots[self.cur];
        s.sounding() && (self.held || s.level > 1e-3)
    }

    /// The pickup's comb: its distance from the bridge, as a delay. It does
    /// not depend on the fret: a fretted string is shorter, but its waves
    /// travel just as fast.
    fn set_pickup(&mut self, c: &Ctl, f_nut: f32) {
        let s = self.idx as f32;
        let (pos, n): ([f32; 4], usize) = match (c.model, c.pickup) {
            (CLEAN, 0) => ([0.245, 0.0, 0.0, 0.0], 1),
            (CLEAN, 1) => ([0.155, 0.0, 0.0, 0.0], 1),
            // A slanted bridge pickup: closer to the bridge under the treble strings.
            (CLEAN, _) => ([0.078 - 0.004 * s, 0.0, 0.0, 0.0], 1),
            (HUMBUCKER, 0) => ([0.205, 0.235, 0.0, 0.0], 2),
            (HUMBUCKER, 2) => ([0.052, 0.082, 0.0, 0.0], 2),
            // Both pickups.
            (HUMBUCKER, _) => ([0.205, 0.235, 0.052, 0.082], 4),
            (BASS, 0) => ([0.22, 0.0, 0.0, 0.0], 1),
            // A split coil in the middle.
            (BASS, 1) => ([0.15, 0.17, 0.0, 0.0], 2),
            (BASS, _) => ([0.085, 0.0, 0.0, 0.0], 1),
            _ => ([0.0; 4], 0),
        };
        self.n_pick = n;
        for j in 0..n {
            self.pick_d[j] = (2.0 * pos[j] * c.sr / f_nut).clamp(1.0, (self.hist_mask - 2) as f32);
        }
    }

    /// One period of the force of a released string on the bridge: a
    /// rectangle with the pick position as its duty cycle, rounded off by
    /// the pick or finger (two one-poles at `fc`), plus a scrape of noise.
    fn excite(&mut self, period: f32, pulse: f32, fc: f32, amp: f32, noise: f32, sr: f32) {
        let len = (period.round() as usize).clamp(8, self.exc.len());
        let k = (pulse.round() as usize).clamp(1, len / 2);
        let b = k as f32 / len as f32;
        let c = 1.0 - (-TAU * fc.min(0.45 * sr) / sr).exp();
        let (mut z1, mut z2) = (0.0f32, 0.0f32);
        // Two passes: the first settles the filters on the periodic shape.
        for pass in 0..2 {
            for i in 0..len {
                let x = if i < k { 1.0 - b } else { -b };
                z1 += c * (x - z1);
                z2 += c * (z1 - z2);
                if pass == 1 {
                    self.exc[i] = z2 * amp;
                }
            }
        }
        let n = ((0.0012 * sr) as usize).min(len);
        let mut prev = 0.0;
        for i in 0..n {
            let r = self.rng.bipolar();
            let env = 1.0 - i as f32 / n as f32;
            self.exc[i] += amp * noise * (r - prev) * env * env;
            prev = r;
        }
        self.exc_len = len;
        self.exc_pos = 0;
    }

    #[inline]
    fn tick(&mut self, fade_k: f32, level_k: f32) -> f32 {
        let e = if self.exc_pos < self.exc_len {
            let v = self.exc[self.exc_pos];
            self.exc_pos += 1;
            v
        } else {
            0.0
        };
        let mut nz = 0.0;
        if self.noise > 1e-6 {
            let raw = self.rng.bipolar() * self.noise;
            self.noise *= self.noise_k;
            nz = self.noise_hp.hp(self.noise_lp.lp(raw));
        }
        let mut y = 0.0;
        let cur = self.cur;
        let w = self.exc_w;
        for (k, s) in self.slots.iter_mut().enumerate() {
            if !s.active {
                continue;
            }
            let input = if k == cur { e + 0.3 * nz } else { 0.0 };
            let mut v = s.loops[0].tick(input * w[0]) + s.loops[1].tick(input * w[1]);
            if s.fading {
                s.fade *= fade_k;
                v *= s.fade;
                if s.fade < 1e-4 {
                    s.active = false;
                }
            }
            s.level = v.abs().max(s.level * level_k);
            s.age = s.age.saturating_add(1);
            y += v;
        }
        y += 0.7 * nz;
        let m = self.hist_mask;
        self.hist[self.hist_w & m] = y;
        if self.n_pick > 0 {
            let mut acc = 0.0;
            for &d in &self.pick_d[..self.n_pick] {
                let i = d as usize;
                let fr = d - i as f32;
                let a = self.hist[self.hist_w.wrapping_sub(i) & m];
                let b = self.hist[self.hist_w.wrapping_sub(i + 1) & m];
                acc += a + (b - a) * fr;
            }
            y -= acc / self.n_pick as f32;
        }
        self.hist_w = self.hist_w.wrapping_add(1);
        y
    }
}

/// Parameters for one block.
struct Ctl {
    sr: f32,
    /// Pitch bend in cents.
    bend_cents: f32,
    /// Index into STRING_MODES, and One String's string (0 = low E).
    mode: usize,
    one: usize,
    model: usize,
    tuning: usize,
    capo: i32,
    home: f32,
    finger: bool,
    pick_pos: f32,
    hardness: f32,
    vel_sens: f32,
    sustain: f32,
    t60_hi: f32,
    stiffness: f32,
    beat_ratio: f32,
    palm: f32,
    let_ring: bool,
    rel_noise: f32,
    slide_s: f32,
    strum_s: f32,
    direction: usize,
    humanize: f32,
    pickup: usize,
    tone: f32,
    body: f32,
    width: f32,
    gain: f32,
    glide_k: f32,
}

impl Ctl {
    fn new(p: &[f32], sr: f32) -> Self {
        let choice = |i: usize, n: usize| (p[i].round().max(0.0) as usize).min(n - 1);
        let model = choice(MODEL, MODELS.len());
        // Doubled courses beat more against each other.
        let course = if matches!(model, TWELVE | MANDOLIN) { 2.5 } else { 1.0 };
        let mut c = Self {
            sr,
            bend_cents: 0.0,
            mode: if p.len() > STRING_MODE { choice(STRING_MODE, STRING_MODES.len()) } else { 0 },
            one: 6 - p.get(ONE_STRING).map(|v| v.round().clamp(1.0, 6.0) as usize).unwrap_or(3),
            model: choice(MODEL, MODELS.len()),
            tuning: choice(TUNING, TUNINGS.len()),
            capo: p[CAPO].round().clamp(0.0, 12.0) as i32,
            home: p[POSITION].round().clamp(0.0, 12.0),
            finger: p[PLUCK] >= 0.5,
            pick_pos: p[PICK_POSITION].clamp(0.03, 0.5),
            hardness: p[HARDNESS].clamp(0.0, 1.0),
            vel_sens: p[VELOCITY].clamp(0.0, 1.0),
            sustain: p[SUSTAIN].clamp(0.3, 20.0),
            // Decay time around 2.5 kHz.
            t60_hi: 0.04 * 60f32.powf(p[BRIGHTNESS].clamp(0.0, 1.0)),
            stiffness: p[STIFFNESS].clamp(0.0, 1.0),
            beat_ratio: cents(p[BEATING].clamp(0.0, 1.0) * 2.5 * course),
            palm: p[PALM_MUTE].clamp(0.0, 1.0),
            let_ring: p[LET_RING] >= 0.5,
            rel_noise: p[RELEASE_NOISE].clamp(0.0, 1.0),
            slide_s: p[SLIDE].clamp(0.0, 400.0) * 0.001,
            strum_s: p[STRUM].clamp(0.0, 60.0) * 0.001,
            direction: choice(STRUM_DIRECTION, DIRECTIONS.len()),
            humanize: p[HUMANIZE].clamp(0.0, 1.0),
            pickup: choice(PICKUP, PICKUPS.len()),
            tone: p[TONE].clamp(0.0, 1.0),
            body: p[BODY].clamp(0.0, 1.0),
            width: p[WIDTH].clamp(0.0, 1.0),
            gain: db_to_gain(p[VOLUME].clamp(-24.0, 12.0)) * MODEL_GAIN[model],
            glide_k: (-(CTRL as f32) / (0.09 * sr)).exp(),
        };
        // One String on a string the instrument lacks takes the next one down.
        while c.one > 0 && c.open()[c.one] == NO_STRING {
            c.one -= 1;
        }
        c
    }

    fn electric(&self) -> bool {
        is_electric(self.model)
    }

    /// Open strings with the capo on (NO_STRING where there is none).
    fn open(&self) -> [i32; 6] {
        open_strings(self.model, self.tuning).map(|n| n.map(|n| n as i32 + self.capo).unwrap_or(NO_STRING))
    }

    /// The open string's frequency (without the capo), for the pick and pickup.
    fn nut_hz(&self, s: usize) -> f32 {
        midi_to_freq(open_strings(self.model, self.tuning)[s].unwrap_or(60) as f32)
    }

    fn inharmonicity(&self, s: usize, fret: i32) -> f32 {
        let base = match self.model {
            NYLON | UKULELE => B_NYLON[s],
            STEEL | RESONATOR | ARCHTOP | TWELVE | BARITONE => B_STEEL[s],
            BASS => B_BASS[s],
            BANJO | MANDOLIN => B_LIGHT[s],
            _ => B_ELECTRIC[s],
        };
        // B grows with the inverse square of the vibrating length.
        2.0 * self.stiffness * base * 2f32.powf(fret as f32 / 6.0)
    }

    fn wound(&self, s: usize) -> bool {
        match self.model {
            UKULELE => false,
            // The banjo's fourth string (the low D) is the wound one.
            BANJO => s == 1,
            MANDOLIN => s < 2,
            BASS => true,
            // Acoustic steel sets have a wound G too.
            STEEL | RESONATOR | ARCHTOP | TWELVE | BARITONE => s < 4,
            _ => s < 3,
        }
    }
}

/// Decay times (low, around 2.5 kHz) for one polarisation.
fn decay_times(c: &Ctl, f: f32, polar: usize, released: bool) -> (f32, f32) {
    let (sus_k, bright_k) = MODEL_DECAY[c.model];
    let mut lo = c.sustain * sus_k * (110.0 / f).sqrt();
    let mut hi = c.t60_hi * bright_k;
    if polar == 1 && !matches!(c.model, TWELVE | MANDOLIN) {
        // The polarisation across the top drives the bridge harder and
        // loses its energy sooner (on a doubled course it is the other
        // string, which rings as long).
        lo *= 0.5;
        hi *= 0.8;
    }
    if c.palm > 0.0 {
        lo = (lo.ln() * (1.0 - c.palm) + 0.18f32.ln() * c.palm).exp();
        hi = (hi.ln() * (1.0 - c.palm) + 0.012f32.ln() * c.palm).exp();
    }
    if released {
        lo = lo.min(0.11);
        hi = hi.min(0.03);
    }
    (lo, hi.min(0.9 * lo).max(0.003))
}

/// Loss, dispersion and tuning of one loop at `f`.
fn tune_loop(lp: &mut Loop, f: f32, lo: f32, hi: f32, b: f32, stages: usize, sr: f32) {
    let s0 = 6.91 / lo;
    let s_hi = 6.91 / hi;
    // Decay rate s0 + b3 f^2, as measured on real strings.
    let b3 = ((s_hi - s0) / (2500.0 * 2500.0)).max(0.0);
    let fref = (2.0 * f).max(2500.0).min(0.4 * sr);
    lp.g = (-s0 / f).exp();
    lp.a = loss_pole((-b3 * fref * fref / f).exp(), TAU * fref / sr);
    let period = sr / f;
    if stages > 0 && b > 0.0 {
        if (f / lp.disp_f.max(1.0) - 1.0).abs() > 0.002 || lp.disp_b != b {
            lp.ap = dispersion_coef(b, f, sr, stages, 0.4 * period);
            lp.disp_f = f;
            lp.disp_b = b;
        }
        lp.n_ap = stages;
    } else {
        lp.n_ap = 0;
    }
    let w = TAU * f / sr;
    let d = period - loss_delay(lp.a, w) - lp.n_ap as f32 * ap_delay(lp.ap, w);
    lp.delay = d.clamp(2.0, (lp.mask - 4) as f32);
}

fn update_slot(s: &mut Slot, c: &Ctl) {
    s.glide *= c.glide_k;
    if s.slide_steps > 0 {
        s.freq *= s.slide_ratio;
        s.slide_steps -= 1;
        if s.slide_steps == 0 {
            s.freq = s.target;
            s.age = 0;
        }
    }
    // A slide player's vibrato, once the note has settled.
    let mut vib = 0.0;
    if c.slide_s > 0.0 && !s.released && s.slide_steps == 0 {
        let depth = (s.age as f32 / c.sr - 0.3).clamp(0.0, 0.5) * 2.0;
        s.vib_phase = (s.vib_phase + 5.2 * CTRL as f32 / c.sr).fract();
        vib = 9.0 * depth * (TAU * s.vib_phase).sin();
    }
    let f = s.freq * cents(s.glide + vib + c.bend_cents);
    for (k, lp) in s.loops.iter_mut().enumerate() {
        let fk = if k == 1 { f * c.beat_ratio * if s.octave { 2.0 } else { 1.0 } } else { f };
        let (lo, hi) = decay_times(c, fk, k, s.released);
        tune_loop(lp, fk, lo, hi, s.b, s.stages, c.sr);
    }
}

#[derive(Clone, Copy, Default)]
struct Incoming {
    key: u8,
    vel: f32,
    released: bool,
}

#[derive(Clone, Copy, Default)]
struct Pending {
    string: usize,
    note: u8,
    key: u8,
    fret: i32,
    vel: f32,
    wait: u32,
    slide: bool,
    up: bool,
    release: bool,
}

#[derive(Clone, Copy, Default)]
struct Mode {
    c: [Coefs; 2],
    st: [State; 2],
    gain: [f32; 2],
}

/// Body, pickup and room: everything after the strings.
struct Output {
    model: usize,
    modes: [Mode; N_MODES],
    direct: [OnePole; 2],
    tone_c: Coefs,
    tone_st: [State; 2],
    tone: f32,
    pickup: [Svf; 2],
    pickup_hp: [OnePole; 2],
    room: Vec<f32>,
    room_mask: usize,
    room_w: usize,
    room_taps: [[(usize, f32); 6]; 2],
    room_lp: [OnePole; 2],
    dc: [DcBlock; 2],
}

impl Output {
    fn new(sr: f32) -> Self {
        let len = ((0.05 * sr) as usize).next_power_of_two();
        let mut direct = [OnePole::default(); 2];
        let mut pickup_hp = [OnePole::default(); 2];
        let mut room_lp = [OnePole::default(); 2];
        for k in 0..2 {
            direct[k].set(280.0, sr);
            pickup_hp[k].set(70.0, sr);
            room_lp[k].set(4500.0, sr);
        }
        let room_taps = ROOM.map(|side| side.map(|(t, g)| (((t * sr) as usize).max(1), g)));
        Self {
            model: usize::MAX,
            modes: [Mode::default(); N_MODES],
            direct,
            tone_c: Coefs::IDENTITY,
            tone_st: [State::default(); 2],
            tone: -1.0,
            pickup: [Svf::new(), Svf::new()],
            pickup_hp,
            room: vec![0.0; len],
            room_mask: len - 1,
            room_w: 0,
            room_taps,
            room_lp,
            dc: [DcBlock::default(); 2],
        }
    }

    fn prepare(&mut self, c: &Ctl) {
        let sr = c.sr;
        if self.model != c.model {
            self.model = c.model;
            // A bass pickup hears the low E's fundamental.
            for hp in self.pickup_hp.iter_mut() {
                hp.set(if c.model == BASS { 25.0 } else { 70.0 }, sr);
            }
            if !c.electric() {
                let (table, scale) = body_modes(c.model);
                // The two sides hear slightly different bodies (two mics).
                for (k, &(f, q, g)) in table.iter().enumerate() {
                    let spread = if k % 2 == 0 { 0.025 } else { -0.02 };
                    for ch in 0..2 {
                        let fc = scale * f * (1.0 + if ch == 0 { spread } else { -spread });
                        self.modes[k].c[ch] = biquad::bandpass(fc as f64, sr as f64, q as f64);
                        self.modes[k].gain[ch] = g * if (k + ch) % 3 == 0 { 0.8 } else { 1.0 };
                    }
                }
            }
        }
        if (self.tone - c.tone).abs() > 1e-4 {
            self.tone = c.tone;
            self.tone_c = biquad::high_shelf(3000.0, sr as f64, 0.7, (-10.0 + 16.0 * c.tone) as f64);
        }
        // The pickup coil's resonance with the cable, lowered by the tone control.
        let (fc, q) = match c.model {
            HUMBUCKER => (1000.0 * 2.8f32.powf(c.tone), 0.7 + 1.1 * c.tone),
            BASS => (700.0 * 3.2f32.powf(c.tone), 0.7 + 0.9 * c.tone),
            _ => (1300.0 * 3.85f32.powf(c.tone), 0.8 + 1.6 * c.tone),
        };
        for f in self.pickup.iter_mut() {
            f.set_q(fc, q, sr);
        }
    }

    #[inline]
    fn tick(&mut self, bl: f32, br: f32, c: &Ctl) -> (f32, f32) {
        let x = [bl, br];
        let mut y = [0.0f32; 2];
        if c.electric() {
            for ch in 0..2 {
                y[ch] = 1.6 * self.pickup[ch].tick(self.pickup_hp[ch].hp(x[ch])).low;
            }
        } else {
            let mut body = [0.0f32; 2];
            for m in self.modes.iter_mut() {
                for ch in 0..2 {
                    body[ch] += m.gain[ch] * m.st[ch].run(&m.c[ch], x[ch] as f64) as f32;
                }
            }
            for ch in 0..2 {
                let acoustic = 1.1 * body[ch] + 0.45 * self.direct[ch].hp(x[ch]);
                let v = x[ch] * (1.0 - c.body) + acoustic * c.body;
                y[ch] = self.tone_st[ch].run(&self.tone_c, v as f64) as f32;
            }
        }
        // A few early reflections.
        let m = self.room_mask;
        self.room[self.room_w & m] = 0.5 * (y[0] + y[1]);
        let amount = c.body * if c.electric() { 0.5 } else { 0.3 };
        for ch in 0..2 {
            let mut acc = 0.0;
            for &(d, g) in &self.room_taps[ch] {
                acc += g * self.room[self.room_w.wrapping_sub(d) & m];
            }
            y[ch] += amount * self.room_lp[ch].lp(acc);
        }
        self.room_w = self.room_w.wrapping_add(1);
        let mid = 0.5 * (y[0] + y[1]);
        let side = 0.5 * (y[0] - y[1]) * c.width;
        let l = self.dc[0].tick(mid + side) * c.gain;
        let r = self.dc[1].tick(mid - side) * c.gain;
        (l, r)
    }
}

pub struct GuitarState {
    /// Pitch bend in semitones (a wheel, a controller's triggers), set by the engine.
    pub pitch_bend: f32,
    sr: f32,
    strings: [GString; STRINGS],
    incoming: [Incoming; QUEUE],
    n_incoming: usize,
    pending: [Pending; QUEUE],
    n_pending: usize,
    /// Where the fretting hand is (its first finger's fret).
    hand: f32,
    next_up: bool,
    /// Strings taken by the last chord, and when it came.
    chord_mask: u8,
    chord_at: u64,
    last_string: usize,
    clock: u64,
    ctrl_left: usize,
    rng: Rng,
    out: Output,
    quiet: usize,
    fade_k: f32,
    level_k: f32,
}

impl GuitarState {
    pub fn new(sr: f32) -> Self {
        Self {
            pitch_bend: 0.0,
            sr,
            strings: std::array::from_fn(|i| GString::new(i, sr)),
            incoming: [Incoming::default(); QUEUE],
            n_incoming: 0,
            pending: [Pending::default(); QUEUE],
            n_pending: 0,
            hand: 0.0,
            next_up: false,
            chord_mask: 0,
            chord_at: 0,
            last_string: 0,
            clock: 0,
            ctrl_left: 0,
            rng: Rng::new(0x6017_A12),
            out: Output::new(sr),
            quiet: usize::MAX / 2,
            // A pick or finger stops a string within a few milliseconds.
            fade_k: (-1.0 / (0.0025 * sr)).exp(),
            level_k: (-1.0 / (0.05 * sr)).exp(),
        }
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        if vel <= 0.0 {
            self.note_off(p, note);
            return;
        }
        if self.n_incoming < QUEUE {
            self.incoming[self.n_incoming] = Incoming { key: note, vel: vel.min(1.0), released: false };
            self.n_incoming += 1;
        }
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        if p.len() < SPECS.len() {
            return;
        }
        for inc in self.incoming[..self.n_incoming].iter_mut() {
            if inc.key == note {
                inc.released = true;
            }
        }
        for pd in self.pending[..self.n_pending].iter_mut() {
            if pd.key == note {
                pd.release = true;
            }
        }
        let mut c = Ctl::new(p, self.sr);
        c.bend_cents = self.pitch_bend * 100.0;
        for s in 0..STRINGS {
            let st = &self.strings[s];
            if st.held && st.key == note && st.note.is_some() {
                self.strings[s].held = false;
                if !c.let_ring {
                    self.lift(s, &c);
                }
            }
        }
    }

    pub fn all_off(&mut self) {
        self.n_incoming = 0;
        self.n_pending = 0;
        for st in self.strings.iter_mut() {
            st.held = false;
            for s in st.slots.iter_mut() {
                s.released = true;
            }
        }
        // Make the damping take effect at the next control step.
        self.ctrl_left = 0;
    }

    /// For a display: which string each note is on and the fret, per string
    /// (None: silent).
    pub fn frets(&self) -> [Option<u8>; 6] {
        std::array::from_fn(|s| {
            let st = &self.strings[s];
            if st.note.is_some() && st.sounding() {
                Some(st.fret)
            } else {
                None
            }
        })
    }

    /// The finger lets go of a string: it is damped, with a little noise.
    /// What the display shows: [0..6] the fret each string is held at (-1
    /// when it is silent), [6..12] how strongly each rings, [12] where the
    /// hand is on the neck.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        for (s, f) in self.frets().iter().enumerate() {
            out[s] = f.map(|f| f as f32).unwrap_or(-1.0);
            let st = &self.strings[s];
            out[6 + s] = st.slots[st.cur].level;
        }
        out[12] = self.hand;
    }

    fn lift(&mut self, s: usize, c: &Ctl) {
        let st = &mut self.strings[s];
        let slot = &mut st.slots[st.cur];
        if !slot.active || slot.released {
            return;
        }
        slot.released = true;
        update_slot(slot, c);
        let fretted = st.fret as i32 > c.capo;
        let level = (slot.level * 4.0).min(1.0);
        st.noise = c.rel_noise * 0.05 * (0.3 + level) * if fretted { 1.0 } else { 0.5 };
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        if n == 0 {
            return;
        }
        let mut c = Ctl::new(p, self.sr);
        c.bend_cents = self.pitch_bend * 100.0;
        if self.n_incoming > 0 {
            self.place(&c);
        }
        let busy = self.n_pending > 0 || self.strings.iter().any(|s| s.active());
        if busy {
            self.quiet = 0;
        } else if self.quiet > self.sr as usize {
            // Silent for a second: body and room have died away too.
            self.clock += n as u64;
            return;
        }
        self.out.prepare(&c);
        let electric = c.electric();
        for i in 0..n {
            if self.n_pending > 0 {
                self.fire_due(&c);
            }
            if self.ctrl_left == 0 {
                self.control(&c);
                self.ctrl_left = CTRL;
            }
            self.ctrl_left -= 1;
            let (mut bl, mut br) = (0.0, 0.0);
            for st in self.strings.iter_mut() {
                if st.active() {
                    let y = st.tick(self.fade_k, self.level_k);
                    bl += y * st.gl;
                    br += y * st.gr;
                }
            }
            if electric {
                // A pickup hears the string's motion, not the body.
                bl *= 0.8;
                br *= 0.8;
            }
            let (ol, or) = self.out.tick(bl, br, &c);
            l[i] += ol;
            r[i] += or;
            self.clock += 1;
        }
        if !busy {
            self.quiet += n;
        }
    }

    fn control(&mut self, c: &Ctl) {
        for (s, st) in self.strings.iter_mut().enumerate() {
            st.set_pickup(c, c.nut_hz(s));
            for slot in st.slots.iter_mut() {
                if !slot.active {
                    continue;
                }
                update_slot(slot, c);
                if !slot.fading && slot.level < 2e-6 && slot.age as f32 > 0.05 * c.sr {
                    slot.active = false;
                }
            }
        }
    }

    fn fire_due(&mut self, c: &Ctl) {
        let mut j = 0;
        while j < self.n_pending {
            if self.pending[j].wait == 0 {
                let pd = self.pending[j];
                self.n_pending -= 1;
                self.pending[j] = self.pending[self.n_pending];
                self.start(pd, c);
            } else {
                self.pending[j].wait -= 1;
                j += 1;
            }
        }
    }

    /// Plays one string: a fresh pluck, or a slide on a ringing one.
    fn start(&mut self, pd: Pending, c: &Ctl) {
        let sr = c.sr;
        let s = pd.string;
        let f_nut = c.nut_hz(s);
        let freq = midi_to_freq(pd.note as f32);
        let fret = pd.fret + c.capo;
        let b = c.inharmonicity(s, fret);
        let stages = if c.wound(s) { 4 } else { 2 };
        let st = &mut self.strings[s];
        st.note = Some(pd.note);
        st.key = pd.key;
        st.fret = fret as u8;
        st.held = true;
        let cur = st.cur;
        if pd.slide && st.slots[cur].sounding() && !st.slots[cur].released {
            let slot = &mut st.slots[cur];
            let steps = ((c.slide_s * sr / CTRL as f32) as u32).max(1);
            slot.target = freq;
            slot.slide_ratio = (freq / slot.freq).powf(1.0 / steps as f32);
            slot.slide_steps = steps;
            slot.b = b;
        } else {
            // The pick (or finger) touches the string: the old note stops.
            if st.slots[cur].active {
                st.slots[cur].fading = true;
            }
            let new = 1 - cur;
            st.cur = new;
            let v = pd.vel.clamp(0.0, 1.0);
            let x = (0.4 * c.hardness + 0.6 * v).clamp(0.0, 1.0);
            let palm = c.palm;
            let slot = &mut st.slots[new];
            for lp in slot.loops.iter_mut() {
                lp.clear();
            }
            slot.active = true;
            slot.fading = false;
            slot.fade = 1.0;
            slot.released = false;
            slot.freq = freq;
            slot.target = freq;
            slot.slide_steps = 0;
            slot.level = 0.0;
            slot.age = 0;
            slot.vib_phase = 0.0;
            slot.b = b;
            slot.stages = stages;
            slot.octave = c.model == TWELVE && s < 4;
            // Hard plucks stretch the string: a few cents sharp at first.
            let low = 1.0 - s as f32 / 10.0;
            slot.glide = 16.0 * x * x * if c.finger { 0.6 } else { 1.0 } * low * (1.0 - 0.7 * palm);
            update_slot(slot, c);
            // Pick position relative to the vibrating length: the pick stays
            // where it is, the fretted string gets shorter.
            let period = sr / freq;
            let pulse = c.pick_pos * (1.0 + 0.15 * c.humanize * st.rng.bipolar()) * sr / f_nut;
            let mut fc = if c.finger { 450.0 * 5f32.powf(x) } else { 1400.0 * 8f32.powf(x) };
            fc *= 1.0 - 0.6 * palm;
            match c.model {
                NYLON | UKULELE => fc *= 0.8,
                BANJO => fc *= 1.4,
                BASS => fc *= 0.6,
                _ => {}
            }
            let level = (1.0 - c.vel_sens) + c.vel_sens * v.powf(1.4);
            let amp = 0.3 * level * (1.0 - 0.35 * palm) * if pd.up { -1.0 } else { 1.0 };
            let noise = if c.finger { 0.04 } else { 0.12 } * x;
            st.excite(period, pulse.min(0.5 * period), fc, amp, noise, sr);
            st.exc_w = if matches!(c.model, TWELVE | MANDOLIN) {
                // Both strings of the course are struck.
                [0.5, 0.5]
            } else {
                let theta = 0.4 + 0.4 * st.rng.unipolar();
                let (sn, cs) = theta.sin_cos();
                [cs / (sn + cs), sn / (sn + cs)]
            };
        }
        if pd.release {
            st.held = false;
            if !c.let_ring {
                self.lift(s, c);
            }
        }
        self.last_string = s;
    }

    fn fret_cost(f: i32, h: i32) -> f32 {
        // Open strings ring the longest: a player takes them in low positions.
        if f == 0 {
            return if h <= 4 { 0.2 } else { 0.9 };
        }
        let lo = h.max(1);
        let hi = lo + 3;
        if f < lo {
            1.5 + (lo - f) as f32
        } else if f > hi {
            1.5 + (f - hi) as f32
        } else {
            0.1 * (f - lo) as f32
        }
    }

    /// What it costs to take string `s` from what it is playing. With
    /// `legato` (one new note while keys are held) a step from a held note
    /// is played on its string, like a hammer-on or pull-off.
    fn busy_cost(&self, s: usize, note: i32, c: &Ctl, pending_mask: u8, legato: bool) -> f32 {
        let st = &self.strings[s];
        if pending_mask & (1 << s) != 0 {
            return 6.0;
        }
        if c.mode != 0 {
            return if st.sounding() && st.held && !legato { 6.0 } else { 0.0 };
        }
        if !st.sounding() {
            0.0
        } else if st.note.map(|n| n as i32) == Some(note) {
            -0.4
        } else if st.held && legato && st.note.is_some_and(|n| (n as i32 - note).abs() <= 2) {
            0.0
        } else if st.held {
            6.0
        } else if c.let_ring {
            1.2
        } else {
            0.25
        }
    }

    /// Notes (ascending) to strings: for every hand position, each note
    /// takes its cheapest free string; the cheapest position wins. With
    /// `slide_from` (a ringing string and its note, when Slide is on) Like a
    /// Player likes to slide along that string for steps up to a fifth.
    #[allow(clippy::too_many_arguments)]
    fn assign(&self, notes: &[i32], open: &[i32; 6], max_fret: i32, avoid: u8, legato: bool, slide_from: Option<(usize, i32)>, c: &Ctl, out: &mut [Option<(usize, i32)>; QUEUE]) -> f32 {
        let pending_mask = self.pending[..self.n_pending].iter().fold(0u8, |m, pd| m | (1 << pd.string));
        let mut best = f32::MAX;
        let mut best_hand = self.hand;
        let mut trial = [None; QUEUE];
        let home = (c.home as i32).clamp(0, (max_fret - 3).max(0));
        let hands = match c.mode {
            0 => 0..=(max_fret - 3).max(0),
            2 => home..=home,
            _ => 0..=0,
        };
        for h in hands {
            let hf = h as f32;
            let mut cost = if c.mode == 0 { 0.3 * (hf - self.hand).abs() + 0.1 * (hf - c.home).abs() + 0.02 * hf } else { 0.0 };
            let mut used = avoid;
            for (k, &nt) in notes.iter().enumerate() {
                let mut pick = None;
                let mut pc = f32::MAX;
                for (s, &o) in open.iter().enumerate() {
                    let f = nt - o;
                    if used & (1 << s) != 0 || f < 0 || f > max_fret {
                        continue;
                    }
                    // A banjo's short fifth string is played open.
                    if c.model == BANJO && s == 0 && f > 0 {
                        continue;
                    }
                    let place = match c.mode {
                        1 => 0.5 * f as f32,
                        3 => {
                            if s == c.one {
                                0.0
                            } else {
                                50.0 + 0.5 * f as f32
                            }
                        }
                        _ => Self::fret_cost(f, h),
                    };
                    let cost_s = match slide_from {
                        Some((ss, old)) if c.mode == 0 && ss == s && (old - nt).abs() <= 7 => place - 0.6,
                        _ => place + self.busy_cost(s, nt, c, pending_mask, legato),
                    };
                    if cost_s < pc {
                        pc = cost_s;
                        pick = Some((s, f));
                    }
                }
                trial[k] = pick;
                match pick {
                    Some((s, _)) => {
                        used |= 1 << s;
                        cost += pc;
                    }
                    None => cost += 20.0,
                }
            }
            if cost < best - 1e-6 {
                best = cost;
                best_hand = hf;
                out[..notes.len()].copy_from_slice(&trial[..notes.len()]);
            }
        }
        best_hand
    }

    /// Takes the notes that came in since the last sample: places them on
    /// strings and schedules the plucks (strummed if it is a chord).
    fn place(&mut self, c: &Ctl) {
        let n_in = self.n_incoming;
        self.n_incoming = 0;
        let open = c.open();
        let max_fret = MAX_FRET - c.capo;
        let lowest = open.iter().copied().filter(|&o| o != NO_STRING).min().unwrap_or(40);
        let highest = open.iter().copied().filter(|&o| o != NO_STRING).max().unwrap_or(64) + max_fret;
        // (pitch, key, velocity, released), ascending, one per pitch.
        let mut notes = [(0i32, 0u8, 0.0f32, false); QUEUE];
        let mut k = 0;
        for inc in &self.incoming[..n_in] {
            let mut nt = inc.key as i32;
            while nt < lowest {
                nt += 12;
            }
            while nt > highest {
                nt -= 12;
            }
            if let Some(j) = notes[..k].iter().position(|x| x.0 == nt) {
                notes[j].2 = notes[j].2.max(inc.vel);
                continue;
            }
            notes[k] = (nt, inc.key, inc.vel, inc.released);
            k += 1;
        }
        let notes = &mut notes[..k];
        notes.sort_by_key(|x| x.0);
        if k == 0 {
            return;
        }
        // Notes that follow each other within 25 ms (a chord, or a strum
        // from a sequencer) belong together: they go on strings of their own.
        let join_window = (0.025 * c.sr) as u64;
        let join = self.chord_mask != 0 && self.clock < self.chord_at + join_window;
        let avoid = if join { self.chord_mask } else { 0 };

        // Slide: a single note goes to its string as the mode says; when
        // that is the string still ringing, it slides there.
        let ringing = {
            let st = &self.strings[self.last_string];
            let slot = &st.slots[st.cur];
            (slot.sounding() && !slot.released).then_some(()).and(st.note).map(|n| (self.last_string, n as i32))
        };
        let slide_from = if k == 1 && c.slide_s > 0.0 && !join { ringing } else { None };

        let pitches: [i32; QUEUE] = std::array::from_fn(|j| if j < k { notes[j].0 } else { 0 });
        let mut placed = [None; QUEUE];
        let hand = self.assign(&pitches[..k], &open, max_fret, avoid, k == 1 && !join, slide_from, c, &mut placed);
        if let (Some((s, old)), Some((ps, f))) = (slide_from, placed[0]) {
            if ps == s && old != notes[0].0 {
                self.push(Pending { string: s, note: notes[0].0 as u8, key: notes[0].1, fret: f, vel: notes[0].2, wait: 0, slide: true, up: false, release: notes[0].3 });
                self.hand = if c.mode == 2 { c.home } else { (f as f32 - 1.0).max(0.0) };
                self.chord_mask = 1 << s;
                self.chord_at = self.clock;
                return;
            }
        }
        // (string, fret, index into notes), in string order.
        let mut plucks = [(0usize, 0i32, 0usize); STRINGS];
        let mut m = 0;
        for (j, pl) in placed[..k].iter().enumerate() {
            if let Some((s, f)) = *pl {
                if m < STRINGS {
                    plucks[m] = (s, f, j);
                    m += 1;
                }
            }
        }
        if m == 0 {
            return;
        }
        if c.mode == 2 {
            self.hand = c.home;
        } else if c.mode != 0 {
            // The hand goes where the fretted notes are.
            if let Some(f) = plucks[..m].iter().map(|p| p.1).filter(|&f| f > 0).min() {
                self.hand = (f as f32 - 1.0).max(0.0);
            }
        } else if plucks[..m].iter().any(|p| p.1 > 0) {
            self.hand = hand;
        }
        let plucks = &mut plucks[..m];
        plucks.sort_by_key(|p| p.0);
        let up = match c.direction {
            0 => false,
            1 => true,
            _ => {
                let u = self.next_up;
                self.next_up = !u;
                u
            }
        };
        if up {
            plucks.reverse();
        }
        let strum = m >= 2 && c.strum_s > 0.0 && !join;
        let first = plucks[0].0 as f32;
        let span = (plucks[m - 1].0 as f32 - first).abs().max(1.0);
        let total = c.strum_s * c.sr * (1.0 + 0.3 * c.humanize * self.rng.bipolar()).max(0.3);
        for (j, &(s, f, ni)) in plucks.iter().enumerate() {
            let mut wait = 0.0;
            if strum {
                wait = (s as f32 - first).abs() / span * total;
                if j > 0 {
                    wait += c.humanize * self.rng.bipolar() * (0.15 * total / span + 0.0015 * c.sr);
                }
            }
            let mut vel = notes[ni].2 * (1.0 + 0.12 * c.humanize * self.rng.bipolar());
            if up && strum {
                vel *= 0.88;
            }
            self.push(Pending {
                string: s,
                note: notes[ni].0 as u8,
                key: notes[ni].1,
                fret: f,
                vel: vel.clamp(0.02, 1.0),
                wait: wait.max(0.0) as u32,
                slide: false,
                up: up && strum,
                release: notes[ni].3,
            });
        }
        let mask = plucks.iter().fold(0u8, |acc, p| acc | (1 << p.0));
        if join {
            self.chord_mask |= mask;
        } else {
            self.chord_mask = mask;
        }
        self.chord_at = self.clock;
    }

    fn push(&mut self, pd: Pending) {
        // A string still waiting for an older pluck gets the new one instead.
        if let Some(j) = self.pending[..self.n_pending].iter().position(|x| x.string == pd.string) {
            self.pending[j] = pd;
            return;
        }
        if self.n_pending < QUEUE {
            self.pending[self.n_pending] = pd;
            self.n_pending += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn render(st: &mut GuitarState, p: &[f32], secs: f32) -> (Vec<f32>, Vec<f32>) {
        let n = (secs * SR) as usize;
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

    /// Magnitude of one frequency (Hann-windowed correlation).
    fn mag(x: &[f32], hz: f32, sr: f32) -> f32 {
        let w = std::f64::consts::TAU * hz as f64 / sr as f64;
        let n = x.len() as f64;
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, &s) in x.iter().enumerate() {
            let win = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n).cos();
            re += s as f64 * win * (w * i as f64).cos();
            im += s as f64 * win * (w * i as f64).sin();
        }
        ((re * re + im * im).sqrt() / n) as f32
    }

    /// Pitch by normalized autocorrelation with parabolic refinement.
    fn pitch_hz(x: &[f32], sr: f32, lo: f32, hi: f32) -> f32 {
        let (min_lag, max_lag) = ((sr / hi) as usize, (sr / lo) as usize);
        let n = x.len() - max_lag - 2;
        let corr = |lag: usize| {
            let (mut xy, mut xx, mut yy) = (0.0f64, 0.0f64, 0.0f64);
            for i in 0..n {
                let (a, b) = (x[i] as f64, x[i + lag] as f64);
                xy += a * b;
                xx += a * a;
                yy += b * b;
            }
            xy / (xx * yy).sqrt().max(1e-20)
        };
        let s: Vec<f64> = (min_lag..=max_lag + 1).map(corr).collect();
        let best = s.iter().cloned().fold(f64::MIN, f64::max);
        let b = (1..s.len() - 1).find(|&i| s[i] >= 0.97 * best && s[i] >= s[i - 1] && s[i] >= s[i + 1]).unwrap();
        let shift = 0.5 * (s[b - 1] - s[b + 1]) / (s[b - 1] - 2.0 * s[b] + s[b + 1]);
        sr / ((min_lag + b) as f64 + shift) as f32
    }

    /// The fundamental's own frequency, near `guess`: how fast its phase
    /// turns between two windows a tenth of a second apart. Unlike the
    /// period of the whole wave, the stretched upper partials of a stiff
    /// string do not pull it sharp.
    fn fundamental_hz(x: &[f32], sr: f32, guess: f32) -> f32 {
        let n = (0.25 * sr) as usize;
        let hop = (0.1 * sr) as usize;
        let w = std::f64::consts::TAU * guess as f64 / sr as f64;
        let dft = |from: usize| {
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for i in 0..n {
                let win = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos();
                let ph = w * (from + i) as f64;
                re += x[from + i] as f64 * win * ph.cos();
                im -= x[from + i] as f64 * win * ph.sin();
            }
            im.atan2(re)
        };
        let mut d = dft(hop) - dft(0);
        while d > std::f64::consts::PI {
            d -= std::f64::consts::TAU;
        }
        while d < -std::f64::consts::PI {
            d += std::f64::consts::TAU;
        }
        guess + (d / (std::f64::consts::TAU * hop as f64 / sr as f64)) as f32
    }

    fn still(name: &str) -> Vec<f32> {
        let mut p = preset(name);
        p[HUMANIZE] = 0.0;
        p
    }

    #[test]
    fn every_preset_sounds() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = GuitarState::new(SR);
            for k in [40, 47, 52, 56, 59, 64] {
                st.note_on(&p, k, 0.8);
            }
            let (l, r) = render(&mut st, &p, 1.0);
            let level = rms(&l).max(rms(&r));
            let peak = l.iter().chain(&r).fold(0.0f32, |m, v| m.max(v.abs()));
            if std::env::var("GUITAR_REPORT").is_ok() {
                eprintln!("{name}: rms {level:.3} peak {peak:.3}");
                continue;
            }
            assert!(l.iter().chain(&r).all(|v| v.is_finite()), "{name} is not finite");
            assert!(level > 0.01, "{name} is too quiet ({level})");
            assert!(peak < 1.2, "{name} peaks at {peak}");
        }
    }

    #[test]
    fn open_strings_are_in_tune() {
        for model in [STEEL, CLEAN] {
            for sr in [48000.0f32, 44100.0] {
                for (s, &k) in TUNING_NOTES[0].iter().enumerate() {
                    let mut p = still("Steel Acoustic");
                    p[MODEL] = model as f32;
                    let mut st = GuitarState::new(sr);
                    st.note_on(&p, k, 0.7);
                    let n = (0.8 * sr) as usize;
                    let mut l = vec![0.0; n];
                    let mut r = vec![0.0; n];
                    for (cl, cr) in l.chunks_mut(64).zip(r.chunks_mut(64)) {
                        st.render(&p, cl, cr);
                    }
                    assert_eq!(st.frets()[s], Some(0), "note {k} is not on open string {s}: {:?}", st.frets());
                    let want = midi_to_freq(k as f32);
                    let got = fundamental_hz(&l[(0.3 * sr) as usize..], sr, want);
                    let cents = 1200.0 * (got / want).log2();
                    assert!(cents.abs() < 3.0, "model {model} at {sr}: string {s} plays {got} Hz, wants {want} Hz ({cents:.1} ct)");
                }
            }
        }
    }

    #[test]
    fn every_relative_is_in_tune_on_its_own_strings() {
        let sr = 48000.0f32;
        for model in RESONATOR..MODELS.len() {
            let mut p = still("Steel Acoustic");
            p[MODEL] = model as f32;
            p[BEATING] = 0.0;
            for (s, k) in open_strings(model, 0).iter().enumerate() {
                let Some(k) = *k else { continue };
                let mut st = GuitarState::new(sr);
                st.note_on(&p, k, 0.7);
                let n = (0.8 * sr) as usize;
                let mut l = vec![0.0; n];
                let mut r = vec![0.0; n];
                for (cl, cr) in l.chunks_mut(64).zip(r.chunks_mut(64)) {
                    st.render(&p, cl, cr);
                }
                // On a string it has (a re-entrant ukulele may take the A on the G string).
                let open = open_strings(model, 0);
                let on: Vec<usize> = (0..6).filter(|&j| st.frets()[j].is_some()).collect();
                assert!(on.len() == 1 && open[on[0]].is_some_and(|o| o as i32 + st.frets()[on[0]].unwrap() as i32 == k as i32), "{}: note {k} (string {s}) lands on {:?}", MODELS[model], st.frets());
                assert!(l.iter().all(|v| v.is_finite()), "{}", MODELS[model]);
                let want = midi_to_freq(k as f32);
                let got = fundamental_hz(&l[(0.3 * sr) as usize..], sr, want);
                let cents = 1200.0 * (got / want).log2();
                assert!(cents.abs() < 3.0, "{}: string {s} plays {got} Hz, wants {want} Hz ({cents:.1} ct)", MODELS[model]);
            }
        }
    }

    #[test]
    fn a_ukulele_has_four_strings_and_a_bass_goes_low() {
        // A chord on a ukulele: four notes at most, on its strings.
        let p = still("Ukulele");
        let mut st = GuitarState::new(SR);
        for k in [60, 64, 67, 72, 76] {
            st.note_on(&p, k, 0.8);
        }
        let _ = render(&mut st, &p, 0.3);
        let f = st.frets();
        assert!(f[4].is_none() && f[5].is_none(), "{f:?}");
        assert!(f.iter().flatten().count() == 4, "{f:?}");
        // The bass's low E is E1, an octave below the guitar's.
        let p = still("Finger Bass");
        let mut st = GuitarState::new(SR);
        st.note_on(&p, 28, 0.8);
        let (l, _) = render(&mut st, &p, 1.0);
        assert_eq!(st.frets()[0], Some(0));
        let got = fundamental_hz(&l[(0.3 * SR) as usize..], SR, 41.2);
        assert!((got - 41.2).abs() < 0.3, "{got}");
    }

    #[test]
    fn a2_is_110_hz() {
        let p = still("Clean Electric");
        let mut st = GuitarState::new(SR);
        st.note_on(&p, 45, 0.8);
        let (l, _) = render(&mut st, &p, 1.0);
        let f = pitch_hz(&l[24000..], SR, 80.0, 160.0);
        assert!((f - 110.0).abs() < 0.2, "{f} Hz");
    }

    #[test]
    fn a_new_note_on_the_same_string_stops_the_old_one() {
        let p = still("Steel Acoustic");
        // A2 on the open A string, then C3 on its third fret.
        let run = |second: bool| {
            let mut st = GuitarState::new(SR);
            st.note_on(&p, 45, 0.8);
            let (mut l, _) = render(&mut st, &p, 0.5);
            if second {
                st.note_off(&p, 45);
                st.note_on(&p, 48, 0.8);
            }
            let (l2, _) = render(&mut st, &p, 0.5);
            if second {
                assert_eq!(st.frets()[1], Some(3), "{:?}", st.frets());
                assert_eq!(st.frets().iter().filter(|f| f.is_some()).count(), 1);
            }
            l.extend_from_slice(&l2);
            l
        };
        let alone = run(false);
        let cut = run(true);
        let at = |x: &[f32]| mag(&x[(0.7 * SR) as usize..(0.95 * SR) as usize], 110.0, SR);
        let ratio = at(&cut) / at(&alone);
        assert!(ratio < 0.03, "A2 is still ringing: {ratio}");
        // And C3 sounds.
        assert!(mag(&cut[(0.7 * SR) as usize..], 130.81, SR) > 10.0 * at(&cut));
    }

    #[test]
    fn same_string_damping_holds_while_the_key_is_held() {
        // Without a note-off (legato), the second note still takes the string.
        let p = still("Clean Electric");
        let mut st = GuitarState::new(SR);
        st.note_on(&p, 45, 0.8);
        render(&mut st, &p, 0.3);
        st.note_on(&p, 47, 0.8);
        let (l, _) = render(&mut st, &p, 0.6);
        let f = st.frets();
        assert_eq!(f[1], Some(2), "{f:?}");
        let tail = &l[(0.3 * SR) as usize..];
        assert!(mag(tail, 110.0, SR) < 0.05 * mag(tail, 123.47, SR));
    }

    /// When each string starts, by watching the fret display.
    fn onsets(p: &[f32], chord: &[u8]) -> [Option<f32>; 6] {
        let mut st = GuitarState::new(SR);
        for &k in chord {
            st.note_on(p, k, 0.8);
        }
        let mut on = [None; 6];
        let mut l = [0.0f32; 16];
        let mut r = [0.0f32; 16];
        for b in 0..(0.1 * SR) as usize / 16 {
            st.render(p, &mut l, &mut r);
            for (s, f) in st.frets().iter().enumerate() {
                if f.is_some() && on[s].is_none() {
                    on[s] = Some(b as f32 * 16.0 / SR * 1000.0);
                }
            }
        }
        on
    }

    /// The strings a run of single notes lands on, one at a time.
    fn strings_for(p: &[f32], keys: &[u8]) -> Vec<usize> {
        let mut st = GuitarState::new(SR);
        keys.iter()
            .map(|&k| {
                st.note_on(p, k, 0.8);
                render(&mut st, p, 0.15);
                let s = st.frets().iter().enumerate().filter(|(_, f)| f.is_some()).map(|(s, _)| s).find(|&s| st.strings[s].note == Some(k)).unwrap_or(99);
                st.note_off(p, k);
                render(&mut st, p, 0.05);
                s
            })
            .collect()
    }

    #[test]
    fn string_modes_place_notes_their_own_way() {
        let mut p = still("Clean Electric");
        // C4, E4, G4, C5.
        let keys = [60, 64, 67, 72];
        p[STRING_MODE] = 1.0;
        // Lowest fret: C4 on the B string (1st fret), E4 open high E, G4 high E 3rd, C5 high E 8th.
        assert_eq!(strings_for(&p, &keys), vec![4, 5, 5, 5]);
        p[STRING_MODE] = 2.0;
        p[POSITION] = 7.0;
        // At the 7th fret: C4 on the G string (5th..), E4 on the B (5th), G4 on the B (8th), C5 on the high E (8th).
        let at7 = strings_for(&p, &keys);
        let open = TUNING_NOTES[0];
        for (s, k) in at7.iter().zip(keys) {
            let f = k as i32 - open[*s] as i32;
            assert!((5..=11).contains(&f), "{at7:?}: fret {f}");
        }
        p[STRING_MODE] = 3.0;
        p[ONE_STRING] = 3.0;
        assert_eq!(strings_for(&p, &keys), vec![3, 3, 3, 3]);
    }

    #[test]
    fn slide_follows_the_string_mode() {
        let mut p = still("Open G Slide");
        p[TUNING] = 0.0;
        p[STRING_MODE] = 3.0;
        p[ONE_STRING] = 2.0;
        // Legato on one string: the second note slides, no new pluck.
        let mut st = GuitarState::new(SR);
        st.note_on(&p, 60, 0.8);
        render(&mut st, &p, 0.3);
        st.note_on(&p, 64, 0.8);
        render(&mut st, &p, 0.02);
        assert_eq!(st.frets()[4], Some(5));
        assert!(st.strings[4].slots[st.strings[4].cur].slide_steps > 0, "it slides");
        // Like a Player: a leap of an octave is plucked on another string.
        p[STRING_MODE] = 0.0;
        let mut st = GuitarState::new(SR);
        st.note_on(&p, 55, 0.8);
        render(&mut st, &p, 0.3);
        st.note_on(&p, 67, 0.8);
        render(&mut st, &p, 0.02);
        let s = (0..6).find(|&s| st.strings[s].note == Some(67)).unwrap();
        assert!(st.strings[s].slots[st.strings[s].cur].slide_steps == 0, "a leap is not slid");
    }

    #[test]
    fn a_chord_is_strummed() {
        let mut p = still("Steel Acoustic");
        p[STRUM] = 40.0;
        let chord = [40, 47, 52, 56, 59, 64];
        p[STRUM_DIRECTION] = 0.0;
        let down = onsets(&p, &chord).map(|t| t.expect("every string plays"));
        assert!(down[0] < 1.0, "{down:?}");
        assert!((down[5] - 40.0).abs() < 2.0, "{down:?}");
        assert!(down.windows(2).all(|w| w[1] > w[0] + 4.0), "{down:?}");
        p[STRUM_DIRECTION] = 1.0;
        let up = onsets(&p, &chord).map(|t| t.expect("every string plays"));
        assert!(up[5] < 1.0 && (up[0] - 40.0).abs() < 2.0, "{up:?}");
        // Without strum, the chord is one block.
        p[STRUM] = 0.0;
        let block = onsets(&p, &chord).map(|t| t.expect("every string plays"));
        assert!(block.iter().all(|&t| t < 1.0), "{block:?}");
        // With humanize, onsets wander but stay ordered roughly over the strum.
        p[STRUM] = 40.0;
        p[STRUM_DIRECTION] = 0.0;
        p[HUMANIZE] = 1.0;
        let human = onsets(&p, &chord).map(|t| t.expect("every string plays"));
        assert!(human[0] < 1.0 && human[5] > 20.0 && human[5] < 60.0, "{human:?}");
    }

    #[test]
    fn chords_go_on_the_strings_a_player_would_use() {
        let p = still("Steel Acoustic");
        let mut st = GuitarState::new(SR);
        // C major in first position: x32010.
        for k in [48, 52, 55, 60, 64] {
            st.note_on(&p, k, 0.8);
        }
        render(&mut st, &p, 0.2);
        assert_eq!(st.frets(), [None, Some(3), Some(2), Some(0), Some(1), Some(0)]);
    }

    #[test]
    fn palm_mute_decays_much_faster() {
        let decay = |palm: f32| {
            let mut p = still("Clean Electric");
            p[PALM_MUTE] = palm;
            let mut st = GuitarState::new(SR);
            st.note_on(&p, 45, 0.8);
            let (l, _) = render(&mut st, &p, 0.8);
            rms(&l[(0.5 * SR) as usize..(0.7 * SR) as usize]) / rms(&l[..(0.1 * SR) as usize])
        };
        let open = decay(0.0);
        let muted = decay(1.0);
        assert!(muted < 0.02 * open, "open {open}, muted {muted}");
    }

    #[test]
    fn lifting_the_finger_damps_and_let_ring_does_not() {
        let tail = |let_ring: bool| {
            let mut p = still("Steel Acoustic");
            p[LET_RING] = if let_ring { 1.0 } else { 0.0 };
            let mut st = GuitarState::new(SR);
            st.note_on(&p, 50, 0.8);
            render(&mut st, &p, 0.3);
            st.note_off(&p, 50);
            let (l, _) = render(&mut st, &p, 0.6);
            rms(&l[(0.4 * SR) as usize..])
        };
        let damped = tail(false);
        let ringing = tail(true);
        assert!(damped < 0.01 * ringing, "damped {damped}, ringing {ringing}");
    }

    #[test]
    fn low_notes_move_up_an_octave_and_slides_glide() {
        let mut p = still("Open G Slide");
        let mut st = GuitarState::new(SR);
        // Below the lowest string (D2): played an octave up.
        st.note_on(&p, 30, 0.8);
        render(&mut st, &p, 0.1);
        assert!(st.frets().iter().any(|f| f.is_some()));
        st.all_off();
        render(&mut st, &p, 1.0);
        // A slide from G3 to A3 on the same string, no new pluck.
        p[HUMANIZE] = 0.0;
        let mut st = GuitarState::new(SR);
        st.note_on(&p, 55, 0.8);
        render(&mut st, &p, 0.3);
        let s = st.frets().iter().position(|f| f.is_some()).unwrap();
        st.note_on(&p, 57, 0.8);
        st.note_off(&p, 55);
        let (l, _) = render(&mut st, &p, 0.7);
        assert!(st.frets()[s].is_some(), "the slide left string {s}: {:?}", st.frets());
        let f = pitch_hz(&l[(0.3 * SR) as usize..(0.45 * SR) as usize], SR, 150.0, 300.0);
        assert!((f / midi_to_freq(57.0) - 1.0).abs() < 0.01, "{f} Hz");
    }

    #[test]
    fn stays_silent_and_cheap_when_idle() {
        let p = preset("Steel Acoustic");
        let mut st = GuitarState::new(SR);
        let (l, r) = render(&mut st, &p, 0.2);
        assert!(l.iter().chain(&r).all(|&v| v == 0.0));
        st.note_on(&p, 52, 0.5);
        let (l, _) = render(&mut st, &p, 0.2);
        assert!(rms(&l) > 1e-3);
        // Short buffers and odd parameter vectors are ignored.
        st.note_on(&[0.0; 3], 60, 1.0);
        st.render(&[0.0; 3], &mut [0.0; 8], &mut [0.0; 8]);
        st.render(&p, &mut [], &mut []);
    }

    #[test]
    #[ignore]
    fn report_cpu() {
        let p = preset("Strummed Chords");
        let mut st = GuitarState::new(SR);
        let t = std::time::Instant::now();
        for k in 0..4 {
            for n in [40, 47, 52, 56, 59, 64] {
                st.note_on(&p, n + k, 0.8);
            }
            render(&mut st, &p, 1.0);
        }
        let el = t.elapsed().as_secs_f32();
        eprintln!("4 s of strummed chords in {:.1} ms ({:.1}% of a core)", el * 1000.0, el * 25.0);
    }
}
