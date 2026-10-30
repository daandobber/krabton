//! MIDI effects that sit in front of an instrument (like Live's MIDI
//! Effects): Arpeggiator, Chord, Note Length, Pitch, Scale, Random,
//! Velocity and Note Echo. They transform note events per audio block and
//! can schedule notes in the future (sample-accurate, tempo-synced).

use crate::dsp::Rng;
use crate::params::{ParamSpec, Unit};
use crate::scale::{Scale, ScaleKind, NOTE_NAMES};
use crate::sequencers::{self, SeqState};
use serde::{Deserialize, Serialize};

mod more;

/// A note event inside one processing block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MidiEvent {
    /// Frame offset inside the current block.
    pub offset: u32,
    pub on: bool,
    pub pitch: u8,
    pub vel: f32,
    /// What it sings (a clip note's lyric; empty for the rest).
    pub lyric: Lyric,
}

impl MidiEvent {
    pub fn on(offset: u32, pitch: u8, vel: f32) -> Self {
        Self { offset, on: true, pitch, vel, lyric: Lyric::default() }
    }
    pub fn off(offset: u32, pitch: u8) -> Self {
        Self { offset, on: false, pitch, vel: 0.0, lyric: Lyric::default() }
    }
}

/// A note's lyric carried with it, short and inline so events stay small
/// and copyable; `early` when the note comes ahead of its time (a clip note
/// sent to the Singer before the beat, for its consonant).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Lyric {
    len: u8,
    bytes: [u8; 22],
    pub early: bool,
}

impl Lyric {
    /// As many whole characters as fit, without the invisible ones.
    pub fn new(s: &str, early: bool) -> Self {
        let mut bytes = [0u8; 22];
        let mut len = 0;
        for c in s.trim().chars().filter(|c| !crate::singer::kana::invisible(*c)) {
            let n = c.len_utf8();
            if len + n > bytes.len() {
                break;
            }
            c.encode_utf8(&mut bytes[len..len + n]);
            len += n;
        }
        Self { len: len as u8, bytes, early }
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or("")
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl std::fmt::Debug for Lyric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}{}", self.as_str(), if self.early { " (early)" } else { "" })
    }
}

/// Timing and musical context for one block.
#[derive(Clone, Copy, Debug)]
pub struct MidiCtx {
    /// Absolute frame of the block start (monotonic).
    pub now: u64,
    pub len: usize,
    pub samples_per_beat: f64,
    pub scale: Scale,
    pub scale_on: bool,
    /// Session-clock beat at the block start (sequencers follow it).
    pub beat: f64,
    pub playing: bool,
    pub sr: f32,
    /// The notes the track's drum kit plays (bit per note; 0: not a kit).
    pub drum_notes: u128,
    /// How far ahead (seconds) the track's instrument wants the notes a
    /// sequencer sings (a Singer: its consonants start before the beat); 0
    /// for every other instrument.
    pub lead: f64,
    /// The Touch view: how hard its plates are pressed, where along them the
    /// hand is, its XY pad, and the value of the plate pressed (0..1 each).
    pub touch: [f32; 5],
    /// Where the band is: its chords, its parts (see `band`).
    pub band: crate::band::BandNow,
}

impl MidiCtx {
    pub fn frames_per_second(&self) -> f32 {
        self.sr
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MidiFxKind {
    Arpeggiator,
    Chord,
    NoteLength,
    Pitch,
    Scale,
    Random,
    Velocity,
    NoteEcho,
    Polymeter,
    Euclid,
    Turing,
    Life,
    Bounce,
    Drunk,
    Step,
    Acid,
    DrumGrid,
    Progression,
    Strum,
    NoteRepeat,
    Humanize,
    NoteFilter,
    Mono,
    Quantize,
    Orbits,
    Chaos,
    Automaton,
    Ant,
    Pong,
    Markov,
    Rain,
    Words,
    Lorenz,
    Loops,
    Bloom,
    Chimes,
    DriftChords,
    Tintinnabuli,
    Pedal,
    Glass,
    Satie,
    Riley,
    Reich,
    Bach,
    Nancarrow,
    Debussy,
    Feldman,
    Mozart,
    Chopin,
    Ravel,
    Bartok,
    Messiaen,
    Ligeti,
    Cage,
    Einaudi,
    GuitarChords,
    Drummer,
    DrumMap,
    Marbles,
    Chance,
    Bits,
    Percussion,
    Spinner,
    Endless,
    Hands,
    Draw,
    PianoPlayer,
    BassPlayer,
    Banjo,
    Section,
    Ukulele,
    OrganGrinder,
    MusicBox,
    Sing,
    Touch,
    Fireflies,
    Sandpile,
    Pendulums,
    Flock,
    Plants,
    Bells,
    Canon,
    Row,
    Gears,
    Formula,
    Staircase,
    Gamelan,
    Raga,
    Call,
}

const ARP_STYLES: &[&str] = &["Up", "Down", "Up Down", "Down Up", "Converge", "Diverge", "Random", "Played", "Chord"];
const ARP_RATES: &[&str] = &["1/32", "1/16 T", "1/16", "1/8 T", "1/8", "1/4", "1/2", "1 Bar"];
const ARP_RATE_BEATS: [f64; 8] = [0.125, 1.0 / 6.0, 0.25, 1.0 / 3.0, 0.5, 1.0, 2.0, 4.0];
const CHORD_TYPES: &[&str] = &["Key Triad", "Key 7th", "Major", "Minor", "Maj7", "Min7", "Dom7", "Sus2", "Sus4", "Power", "Octaves", "Add9", "Custom"];
const LENGTHS: &[&str] = &["1/64", "1/32", "1/16", "1/8", "1/4", "1/2", "1 Bar", "2 Bars"];
const LENGTH_BEATS: [f64; 8] = [1.0 / 16.0, 0.125, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0];
const ECHO_TIMES: &[&str] = &["1/16", "1/8 T", "1/8", "1/8 D", "1/4", "1/4 D", "1/2"];
const ECHO_BEATS: [f64; 7] = [0.25, 1.0 / 3.0, 0.5, 0.75, 1.0, 1.5, 2.0];
const SOURCES: &[&str] = &["Project", "Custom"];
const SCALE_MODES: &[&str] = &["Snap", "Filter"];
const DIRECTIONS: &[&str] = &["Up", "Down", "Both"];
const SCALE_NAMES: &[&str] = &[
    "Major",
    "Minor",
    "Dorian",
    "Phrygian",
    "Lydian",
    "Mixolydian",
    "Locrian",
    "Harmonic Minor",
    "Melodic Minor",
    "Phrygian Dominant",
    "Hungarian Minor",
    "Major Pentatonic",
    "Minor Pentatonic",
    "Blues",
    "Whole Tone",
    "In Sen",
    "Hirajoshi",
    "Chromatic",
];

static ARP: &[ParamSpec] = &[
    ParamSpec::choice("Style", "Arp", ARP_STYLES, 0),
    ParamSpec::choice("Rate", "Arp", ARP_RATES, 2),
    ParamSpec::lin("Gate", "Arp", 0.05, 2.0, 0.75, Unit::Percent),
    ParamSpec::int("Octaves", "Range", 1.0, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Swing", "Range", 0.0, 0.75, 0.0, Unit::Percent),
    ParamSpec::toggle("Hold", "Range", false),
    ParamSpec::toggle("Band", "Band", false),
    ParamSpec::choice("Follow", "Band", crate::band::FOLLOWS, 0),
];
static CHORD: &[ParamSpec] = &[
    ParamSpec::choice("Type", "Chord", CHORD_TYPES, 0),
    ParamSpec::int("Inversion", "Chord", -2.0, 2.0, 0.0, Unit::None),
    ParamSpec::lin("Velocity", "Chord", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::int("Shift 1", "Custom", -24.0, 24.0, 4.0, Unit::Semi),
    ParamSpec::int("Shift 2", "Custom", -24.0, 24.0, 7.0, Unit::Semi),
    ParamSpec::int("Shift 3", "Custom", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::int("Shift 4", "Custom", -24.0, 24.0, 0.0, Unit::Semi),
];
static NOTE_LENGTH: &[ParamSpec] = &[ParamSpec::choice("Length", "Length", LENGTHS, 2), ParamSpec::lin("Gate", "Length", 0.1, 2.0, 1.0, Unit::Percent)];
static PITCH: &[ParamSpec] = &[
    ParamSpec::int("Pitch", "Pitch", -48.0, 48.0, 0.0, Unit::Semi),
    ParamSpec::int("Scale Steps", "Pitch", -14.0, 14.0, 0.0, Unit::None),
];
static SCALE: &[ParamSpec] = &[
    ParamSpec::choice("Source", "Scale", SOURCES, 0),
    ParamSpec::choice("Root", "Scale", &NOTE_NAMES, 0),
    ParamSpec::choice("Scale", "Scale", SCALE_NAMES, 0),
    ParamSpec::choice("Mode", "Scale", SCALE_MODES, 0),
];
static RANDOM: &[ParamSpec] = &[
    ParamSpec::lin("Chance", "Random", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Choices", "Random", 1.0, 12.0, 4.0, Unit::None),
    ParamSpec::int("Spacing", "Random", 1.0, 12.0, 1.0, Unit::Semi),
    ParamSpec::choice("Direction", "Random", DIRECTIONS, 2),
    ParamSpec::toggle("In Key", "Random", true),
];
static VELOCITY: &[ParamSpec] = &[
    ParamSpec::lin("Drive", "Velocity", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Out Low", "Velocity", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Out High", "Velocity", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Random", "Velocity", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Fixed", "Velocity", false),
];
static NOTE_ECHO: &[ParamSpec] = &[
    ParamSpec::choice("Time", "Echo", ECHO_TIMES, 2),
    ParamSpec::int("Repeats", "Echo", 1.0, 8.0, 3.0, Unit::None),
    ParamSpec::lin("Feedback", "Echo", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::int("Pitch", "Echo", -12.0, 12.0, 0.0, Unit::Semi),
];

impl MidiFxKind {
    pub const ALL: [MidiFxKind; 14] = [
        MidiFxKind::Arpeggiator,
        MidiFxKind::Chord,
        MidiFxKind::NoteLength,
        MidiFxKind::Pitch,
        MidiFxKind::Scale,
        MidiFxKind::Random,
        MidiFxKind::Velocity,
        MidiFxKind::NoteEcho,
        MidiFxKind::Strum,
        MidiFxKind::NoteRepeat,
        MidiFxKind::Humanize,
        MidiFxKind::NoteFilter,
        MidiFxKind::Mono,
        MidiFxKind::Quantize,
    ];

    /// The sequencers' groups, in menu order.
    pub const SEQUENCER_GROUPS: [&'static str; 11] = ["Classic", "Players", "Rhythm", "Played", "Generative", "Physics & Chaos", "Nature", "Systems", "Ambient", "Composers", "World"];

    /// Whether it is in a key by its nature (chords, the composers, the
    /// ambient ones): with the project's notes free (scale mode off) it
    /// stays in the song's key, where the others take all twelve notes.
    pub fn keeps_key(self) -> bool {
        matches!(self.group(), "Composers" | "Players" | "Ambient" | "World") || matches!(self, MidiFxKind::Progression | MidiFxKind::Chord | MidiFxKind::Sing | MidiFxKind::Touch)
    }

    /// Which group a sequencer belongs to (see `SEQUENCER_GROUPS`).
    pub fn group(self) -> &'static str {
        use MidiFxKind as K;
        match self {
            K::Step | K::Acid | K::DrumGrid | K::Progression | K::Drummer => "Classic",
            K::GuitarChords | K::Ukulele | K::PianoPlayer | K::BassPlayer | K::Banjo | K::Section | K::OrganGrinder | K::MusicBox => "Players",
            K::Euclid | K::DrumMap | K::Marbles | K::Chance | K::Bits | K::Percussion => "Rhythm",
            K::Spinner | K::Endless | K::Hands | K::Draw | K::Touch => "Played",
            K::Polymeter | K::Turing | K::Life | K::Drunk | K::Markov | K::Words | K::Sing => "Generative",
            K::Bounce | K::Orbits | K::Pong | K::Rain | K::Chaos | K::Automaton | K::Ant | K::Lorenz => "Physics & Chaos",
            K::Fireflies | K::Sandpile | K::Pendulums | K::Flock | K::Plants => "Nature",
            K::Bells | K::Canon | K::Row | K::Gears | K::Formula | K::Staircase => "Systems",
            K::Gamelan | K::Raga | K::Call => "World",
            K::Loops | K::Bloom | K::Chimes | K::DriftChords | K::Tintinnabuli | K::Pedal => "Ambient",
            K::Glass | K::Satie | K::Riley | K::Reich | K::Bach | K::Nancarrow | K::Debussy | K::Feldman | K::Mozart | K::Chopin | K::Ravel | K::Bartok | K::Messiaen | K::Ligeti | K::Cage | K::Einaudi => "Composers",
            _ => "MIDI Effects",
        }
    }

    pub const SEQUENCERS: [MidiFxKind; 75] = [
        MidiFxKind::Step,
        MidiFxKind::Acid,
        MidiFxKind::DrumGrid,
        MidiFxKind::Progression,
        MidiFxKind::Polymeter,
        MidiFxKind::Euclid,
        MidiFxKind::Turing,
        MidiFxKind::Life,
        MidiFxKind::Bounce,
        MidiFxKind::Drunk,
        MidiFxKind::Orbits,
        MidiFxKind::Chaos,
        MidiFxKind::Automaton,
        MidiFxKind::Ant,
        MidiFxKind::Pong,
        MidiFxKind::Markov,
        MidiFxKind::Rain,
        MidiFxKind::Words,
        MidiFxKind::Lorenz,
        MidiFxKind::Loops,
        MidiFxKind::Bloom,
        MidiFxKind::Chimes,
        MidiFxKind::DriftChords,
        MidiFxKind::Tintinnabuli,
        MidiFxKind::Pedal,
        MidiFxKind::Glass,
        MidiFxKind::Satie,
        MidiFxKind::Riley,
        MidiFxKind::Reich,
        MidiFxKind::Bach,
        MidiFxKind::Nancarrow,
        MidiFxKind::Debussy,
        MidiFxKind::Feldman,
        MidiFxKind::Mozart,
        MidiFxKind::Chopin,
        MidiFxKind::Ravel,
        MidiFxKind::Bartok,
        MidiFxKind::Messiaen,
        MidiFxKind::Ligeti,
        MidiFxKind::Cage,
        MidiFxKind::Einaudi,
        MidiFxKind::GuitarChords,
        MidiFxKind::Drummer,
        MidiFxKind::DrumMap,
        MidiFxKind::Marbles,
        MidiFxKind::Chance,
        MidiFxKind::Bits,
        MidiFxKind::Percussion,
        MidiFxKind::Spinner,
        MidiFxKind::Endless,
        MidiFxKind::Hands,
        MidiFxKind::Draw,
        MidiFxKind::PianoPlayer,
        MidiFxKind::BassPlayer,
        MidiFxKind::Banjo,
        MidiFxKind::Section,
        MidiFxKind::Ukulele,
        MidiFxKind::OrganGrinder,
        MidiFxKind::MusicBox,
        MidiFxKind::Sing,
        MidiFxKind::Touch,
        MidiFxKind::Fireflies,
        MidiFxKind::Sandpile,
        MidiFxKind::Pendulums,
        MidiFxKind::Flock,
        MidiFxKind::Plants,
        MidiFxKind::Bells,
        MidiFxKind::Canon,
        MidiFxKind::Row,
        MidiFxKind::Gears,
        MidiFxKind::Formula,
        MidiFxKind::Staircase,
        MidiFxKind::Gamelan,
        MidiFxKind::Raga,
        MidiFxKind::Call,
    ];

    pub fn is_sequencer(self) -> bool {
        Self::SEQUENCERS.contains(&self)
    }

    /// Initial pattern data for sequencers that have editable patterns.
    pub fn default_data(self) -> Vec<u8> {
        match self {
            MidiFxKind::Polymeter => sequencers::polymeter_default(),
            MidiFxKind::Step => crate::classic_seq::step_default(),
            MidiFxKind::Acid => crate::classic_seq::acid_default(),
            MidiFxKind::DrumGrid => crate::classic_seq::drum_default(),
            MidiFxKind::Progression => crate::classic_seq::progression_default(),
            MidiFxKind::Life => sequencers::life_default(),
            MidiFxKind::Words => sequencers::more::words_default(),
            MidiFxKind::Sing => sequencers::sing::default_text(),
            MidiFxKind::Touch => sequencers::touch::default_data(),
            MidiFxKind::GuitarChords | MidiFxKind::Ukulele => sequencers::strum::strum_default(),
            MidiFxKind::Drummer => sequencers::drummer::drummer_default(),
            MidiFxKind::Chance => sequencers::chance::chance_default(),
            MidiFxKind::Endless => sequencers::played::endless_default(),
            MidiFxKind::Draw => sequencers::played::draw_default(),
            MidiFxKind::PianoPlayer | MidiFxKind::BassPlayer | MidiFxKind::Banjo | MidiFxKind::Section => sequencers::players::player_default(),
            _ => Vec::new(),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            MidiFxKind::Polymeter => "Polymeter",
            MidiFxKind::Step => "Step",
            MidiFxKind::Acid => "Acid",
            MidiFxKind::DrumGrid => "Drum Grid",
            MidiFxKind::Progression => "Progression",
            MidiFxKind::Euclid => "Euclid",
            MidiFxKind::Turing => "Turing Machine",
            MidiFxKind::Life => "Life",
            MidiFxKind::Bounce => "Bounce",
            MidiFxKind::Drunk => "Drunk",
            MidiFxKind::Orbits => "Orbits",
            MidiFxKind::Chaos => "Chaos",
            MidiFxKind::Automaton => "Automaton",
            MidiFxKind::Ant => "Ant",
            MidiFxKind::Pong => "Pong",
            MidiFxKind::Markov => "Markov",
            MidiFxKind::Rain => "Rain",
            MidiFxKind::Words => "Words",
            MidiFxKind::Sing => "Sing",
            MidiFxKind::Touch => "Touch",
            MidiFxKind::Lorenz => "Lorenz",
            MidiFxKind::Loops => "Loops",
            MidiFxKind::Bloom => "Bloom",
            MidiFxKind::Chimes => "Chimes",
            MidiFxKind::DriftChords => "Drift Chords",
            MidiFxKind::Tintinnabuli => "Tintinnabuli",
            MidiFxKind::Pedal => "Pedal",
            MidiFxKind::Glass => "Glass",
            MidiFxKind::Satie => "Satie",
            MidiFxKind::Riley => "Riley",
            MidiFxKind::Reich => "Reich",
            MidiFxKind::Bach => "Bach",
            MidiFxKind::Nancarrow => "Nancarrow",
            MidiFxKind::Debussy => "Debussy",
            MidiFxKind::Feldman => "Feldman",
            MidiFxKind::Mozart => "Mozart",
            MidiFxKind::Chopin => "Chopin",
            MidiFxKind::Ravel => "Ravel",
            MidiFxKind::Bartok => "Bartók",
            MidiFxKind::Messiaen => "Messiaen",
            MidiFxKind::Ligeti => "Ligeti",
            MidiFxKind::Cage => "Cage",
            MidiFxKind::Einaudi => "Einaudi",
            MidiFxKind::OrganGrinder => "Organ Grinder",
            MidiFxKind::MusicBox => "Music Box",
            MidiFxKind::GuitarChords => "Guitar Chords",
            MidiFxKind::Drummer => "Drummer",
            MidiFxKind::DrumMap => "Drum Map",
            MidiFxKind::Marbles => "Marbles",
            MidiFxKind::Chance => "Chance",
            MidiFxKind::Bits => "Bits",
            MidiFxKind::Percussion => "Percussion",
            MidiFxKind::Spinner => "Spinner",
            MidiFxKind::Endless => "Endless",
            MidiFxKind::Hands => "Hands",
            MidiFxKind::Draw => "Draw",
            MidiFxKind::PianoPlayer => "Piano Player",
            MidiFxKind::BassPlayer => "Bass Player",
            MidiFxKind::Banjo => "Banjo Roll",
            MidiFxKind::Section => "String Section",
            MidiFxKind::Ukulele => "Ukulele",
            MidiFxKind::Fireflies => "Fireflies",
            MidiFxKind::Sandpile => "Sandpile",
            MidiFxKind::Pendulums => "Pendulums",
            MidiFxKind::Flock => "Flock",
            MidiFxKind::Plants => "Plants",
            MidiFxKind::Bells => "Bells",
            MidiFxKind::Canon => "Canon",
            MidiFxKind::Row => "Row",
            MidiFxKind::Gears => "Gears",
            MidiFxKind::Formula => "Formula",
            MidiFxKind::Staircase => "Staircase",
            MidiFxKind::Gamelan => "Gamelan",
            MidiFxKind::Raga => "Raga",
            MidiFxKind::Call => "Call & Response",

            MidiFxKind::Arpeggiator => "Arpeggiator",
            MidiFxKind::Chord => "Chord",
            MidiFxKind::NoteLength => "Note Length",
            MidiFxKind::Pitch => "Pitch",
            MidiFxKind::Scale => "Scale",
            MidiFxKind::Random => "Random",
            MidiFxKind::Velocity => "Velocity",
            MidiFxKind::NoteEcho => "Note Echo",
            MidiFxKind::Strum => "Strum",
            MidiFxKind::NoteRepeat => "Note Repeat",
            MidiFxKind::Humanize => "Humanize",
            MidiFxKind::NoteFilter => "Note Filter",
            MidiFxKind::Mono => "Mono",
            MidiFxKind::Quantize => "Quantize",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            MidiFxKind::Polymeter => "Kria-style step sequencer: every lane has its own loop length",
            MidiFxKind::Step => "Classic 16-step sequencer: note, velocity, length and ratchets per step",
            MidiFxKind::Acid => "TB-303 style bass line with accent, slide and tie",
            MidiFxKind::DrumGrid => "x0x drum machine grid: 8 lanes, 16 steps, accents and swing",
            MidiFxKind::Progression => "A chord progression in the project key, with rhythms",
            MidiFxKind::Euclid => "Four euclidean rhythm rings, great for drums",
            MidiFxKind::Turing => "A looping shift register that slowly mutates into new melodies",
            MidiFxKind::Life => "Conway's Game of Life plays the board column by column",
            MidiFxKind::Bounce => "Bouncing balls: every floor hit plays a note",
            MidiFxKind::Drunk => "A random walk through the scale",
            MidiFxKind::Orbits => "Planets circling at their own speeds; each pass plays its note: polyrhythms that always meet again",
            MidiFxKind::Chaos => "The logistic map: one knob from steady cycles to chaos, as a melody",
            MidiFxKind::Automaton => "A one-dimensional cellular automaton (rules 30, 90, 110...): patterns that grow and break",
            MidiFxKind::Ant => "Langton's ant walks an 8x8 board, flipping cells: order out of chaos",
            MidiFxKind::Pong => "Balls bouncing in a box: every wall plays its note",
            MidiFxKind::Markov => "A melody from chances: repeat, step, leap or rest, with accents",
            MidiFxKind::Rain => "Raindrops at random times and pitches, in drizzles and storms",
            MidiFxKind::Words => "Type text and it plays as a melody, or as Morse code",
            MidiFxKind::Touch => "Play the track with plates (the Touch view): as notes, an arpeggio, strummed or with echoes, each plate a note or a chord in the key; a pulser with swing, Latch and Run, chance and wander; a sixteen-step sequencer (per step a note, one to eight pulses, a gate for what they play: single, every, hold, rest, last, alternate or roll, value, chance, ratchets up to eight, tie, accent and skip) that can trigger and move the notes; its pressure, pulse, steps, randoms, position, LFO, drawn shape and XY pad patched to the instrument's knobs",
            MidiFxKind::Sing => "Type words or sounds and it sings them: each syllable a note with it as its lyric (for the Singer), in a rhythm and a melody of its own in the song's key, a little different each time round",
            MidiFxKind::Lorenz => "The Lorenz attractor's butterfly as a melody",
            MidiFxKind::Loops => "Notes on their own long loops that drift in and out of phase, like tape loops: ambient chords that never repeat",
            MidiFxKind::Bloom => "Click the pond to drop notes (higher is higher) that echo away, softer each time; left alone it blooms by itself",
            MidiFxKind::Chimes => "Wind chimes, simulated: the wind pushes the sail, the clapper swings, and a note sounds whenever it meets a tube",
            MidiFxKind::DriftChords => "Slowly changing chords: one voice moves a step, the others are held",
            MidiFxKind::Tintinnabuli => "A stepwise melody with a voice on the nearest note of the tonic triad, over a held tonic",
            MidiFxKind::Pedal => "A held drone note that now and then moves to the fifth, fourth or octave",
            MidiFxKind::Glass => "After Philip Glass: a broken chord repeated and repeated over a slow round of chords: three against two, rocking, broken, an additive figure that grows and shrinks, or pulsing chords, the bass held under each",
            MidiFxKind::Satie => "After Erik Satie: a Gymnopédie (a slow waltz, bass on one, a seventh chord on two, a sparse falling melody), a Gnossienne (oom-pah in four, a melody turning with grace notes) or a Sarabande (ninth chords sliding in parallel)",
            MidiFxKind::Reich => "After Steve Reich: two players on one figure, the second slowly pulling a note ahead and holding there (Phase), a clapped rhythm against itself shifting a step (Shifting Rhythm), or chords pulsing and swelling like breaths (Pulses)",
            MidiFxKind::Bach => "After J. S. Bach: a Prelude (a broken chord each half bar, voices led smoothly), an Invention (a motif answered an octave down half a bar later, falling in sequence) or a Chorale (four voices, cadences held)",
            MidiFxKind::Nancarrow => "After Conlon Nancarrow's player-piano studies: one melody in two to four voices at once at tempos in a ratio (3:4, 3:4:5, 4:5:6:7...), timed to arrive at its end together",
            MidiFxKind::Debussy => "After Claude Debussy: a line with the same chord shape sliding under every note (Planing), whole-tone washes, triplet arabesques, or a pentatonic line over parallel fourths",
            MidiFxKind::Mozart => "After W. A. Mozart: a minuet whose sixteen bars are thrown for with two dice (his musical dice game), a Sonata (an Alberti bass, a classical period, a trill before the cadence) or Alla Turca (turns round each note over an oom-pah, a loud refrain over rolled chords)",
            MidiFxKind::Chopin => "After Frédéric Chopin: a Nocturne (wide broken chords below, a singing line above that takes its time while the left hand keeps it, grace notes and runs), a Waltz, or a Prelude (chords sinking a semitone at a time under a held melody)",
            MidiFxKind::Ravel => "After Maurice Ravel's Boléro: the side drum's rhythm and a plucked bass, a long tune and its answer round and round, ever louder, doubled at the octave, then in parallel twelfths and seventeenths, then by everyone, and the crash",
            MidiFxKind::Bartok => "After Béla Bartók: a Bulgarian dance in an uneven metre (2+2+3, 3+2+2...) in the acoustic scale, Night Music (soft clusters, insects and frogs, a far tune) or a Mirror (two voices in exact contrary motion)",
            MidiFxKind::Messiaen => "After Olivier Messiaen: chords in his modes of limited transposition with added values, birdsong (a blackbird, a thrush, a nightingale), or rhythmic characters (a palindrome three times: one growing, one shrinking, one staying)",
            MidiFxKind::Ligeti => "After György Ligeti: a hundred metronomes (each at its own tempo, running down one by one), Continuum (two hands as fast as they go round a few notes that change one at a time) or Musica ricercata (one pitch, then two, then three... until all twelve)",
            MidiFxKind::Cage => "After John Cage: chance (coins tossed for a hexagram of the I Ching pick each sound, length and loudness from a chart, some of it silence), a prepared piano (odd sounds in quiet patterns, a square-root form), or 4'33\"",
            MidiFxKind::OrganGrinder => "A street organ's book: a tune in the song's key over its own chords, as the old dance tunes go (a phrase, again, another, the first once more), with the bass, the oom-pah, a counter melody, grace notes, trills and runs, and the drums on the Street Organ's drum keys; as a waltz, a march, a polka or a foxtrot",
            MidiFxKind::MusicBox => "A music box: a tune pinned on a cylinder, plucking only the notes of its comb, the melody high over a broken chord, long notes plucked again, and a spring that runs down until it stops and is wound again",
            MidiFxKind::Einaudi => "After Ludovico Einaudi: a broken chord round four chords with a few notes of melody over it, or a pattern that builds layer by layer and falls back",
            MidiFxKind::Feldman => "After Morton Feldman: soft chords, few and far apart, in patterns that come back a few times, each time a little changed",
            MidiFxKind::Riley => "After Terry Riley's In C: players each repeat a short figure as long as they like before moving on, never far apart, over a steady pulse; or a Rainbow of modal loops of different lengths turning against each other",
            MidiFxKind::GuitarChords => "Guitar chords the way a guitarist plays them: real chord shapes in the key, strummed down and up in patterns, or picked",
            MidiFxKind::Drummer => "A drummer who plays styles (rock, funk, house, boom bap, bossa nova, jazz...): ghost notes, accents, fills with a crash after, humanized; edit the groove in its grid",
            MidiFxKind::DrumMap => "A topographic drum sequencer: a map of 25 grooves (straight to broken, laid back to driving) that blend as you move; a fill per drum from backbone to every ghost note, and chaos that shakes it every two bars",
            MidiFxKind::Marbles => "A random sampler after Mutable Instruments Marbles: a clock (T2) and two triggers that toss coins, cluster, divide or follow a Markov chain, three random notes in key, and a déjà vu loop to make chance repeat",
            MidiFxKind::Chance => "A drum grid of chances and conditions: every step a chance to play, a condition (1:2, 3:4, only in a fill, only the first time, if the last one played...) and a ratchet; lanes of their own lengths, fills every few bars",
            MidiFxKind::Bits => "Rhythms from bytebeat formulas: a counter, a formula like t*(t>>5|t>>8), and four drums each listening to one bit of it",
            MidiFxKind::Percussion => "A percussion ensemble: clave or bell, shakers, congas, bongos, low drum and güiro in son, guaguancó, bossa nova, samba, baião, cumbia or bembé, a lead drum that improvises, players brought in by Density",
            MidiFxKind::Spinner => "Every key you play drops a ball into a turning polygon; each wall it hits plays its note, as hard as it hit",
            MidiFxKind::Endless => "Record: the notes you play are added one by one (Rest for a rest); then the row plays round and round and a held key moves it",
            MidiFxKind::Hands => "Hold keys and each plays a rhythm: one pattern for all, or a pattern per key (a groove under your fingers on a drum kit)",
            MidiFxKind::Draw => "A line drawn across 32 steps, played in the key over a range of octaves, every step or where it moves",
            MidiFxKind::PianoPlayer => "A pianist on the chord progression: the left hand on the root, the right voicing each chord close to the last; ballad, stride, Alberti, arpeggio, gospel, bossa, block chords, pop eighths",
            MidiFxKind::BassPlayer => "A bass player on the chord progression: roots, root and fifth, octaves, a walking line that leans into the next chord, reggae, funk, Motown, eighths",
            MidiFxKind::Banjo => "A banjo on the chord progression: five strings in open tuning moved to the key, picked in rolls with the high drone string",
            MidiFxKind::Section => "A string section on the chord progression: three to five voices that move as little as they can, held, pulsing, tremolo, pizzicato, stabs or arpeggio",
            MidiFxKind::Fireflies => "Fireflies flashing at their own pace, each pulled towards the others' rhythm: a little Coupling and they drift, more and they flash as one",
            MidiFxKind::Sandpile => "Grains dropped on a board: a cell with four topples onto its neighbours, and every wave of the avalanche plays the cells that fell",
            MidiFxKind::Pendulums => "A pendulum wave: a row of pendulums, each a little shorter, let go together; they drift through waves and chaos and meet again after the cycle",
            MidiFxKind::Flock => "A flock of birds (separation, alignment, cohesion) flying over the notes: a bird crossing a line plays the note of its height",
            MidiFxKind::Plants => "A plant grown by its rules (a Lindenmayer system) and walked as a melody: F a note, + and - a step, [ ] a branch that returns",
            MidiFxKind::Bells => "Change ringing: four to eight bells in rounds, then changing order by Plain Hunt, Plain Bob, Grandsire or Stedman until they come round, with Bobs and Singles called",
            MidiFxKind::Canon => "A phrase (its own, or what you play) and up to four voices following it: later, higher or lower, upside down, backwards, at half or double speed",
            MidiFxKind::Row => "A twelve-tone row (or a row of the key's notes): prime, retrograde, inversion and both, moved round, as a line or in chords, in serial rhythms",
            MidiFxKind::Gears => "Meshed gears turning a tooth a step; pegs on their teeth play as they pass the top: gears of different sizes make polyrhythms",
            MidiFxKind::Formula => "A melody from a bytebeat formula like t*(t>>5|t>>8): each step's byte a note of the key, a rest or a long note, looped",
            MidiFxKind::Staircase => "A stair that rises (or falls) for ever: one note in several octaves, the top ones fading out as new ones fade in below",
            MidiFxKind::Gamelan => "A gamelan: a slow core melody, two parts that lock into each other above it in one fast line, and the gongs marking the cycle; irama sets how dense",
            MidiFxKind::Raga => "A melody by a raga's rules: other notes going up than coming down, notes it rests on, its signature phrase; Alap, Jor and Jhala, grace notes, slides and a drone",
            MidiFxKind::Call => "Call and response: a caller's phrase that ends open, answered by a choir: the same, higher, upside down or coming home to the key note",
            MidiFxKind::Ukulele => "Ukulele chords the way a ukulele player plays them: four strings (high G, low G, baritone or D tuning), real shapes in the key, strummed or picked in the same patterns as Guitar Chords",

            MidiFxKind::Arpeggiator => "Plays held notes one after another, tempo-synced",
            MidiFxKind::Chord => "Turns single notes into chords, also in the project key",
            MidiFxKind::NoteLength => "Gives every note a fixed, tempo-synced length",
            MidiFxKind::Pitch => "Transposes by semitones or by scale steps",
            MidiFxKind::Scale => "Snaps or filters notes to a scale",
            MidiFxKind::Random => "Randomly changes note pitches (optionally in key)",
            MidiFxKind::Velocity => "Reshapes, limits or randomizes velocities",
            MidiFxKind::NoteEcho => "Repeats notes like a delay, with feedback and pitch",
            MidiFxKind::Strum => "Plays chords as strums: note by note, up, down or alternating",
            MidiFxKind::NoteRepeat => "Held notes repeat in time: rolls, ratchets and drum fills",
            MidiFxKind::Humanize => "Loosens timing and velocity, and swings the off-beats",
            MidiFxKind::NoteFilter => "Lets notes through by chance, every Nth, or by key and velocity range",
            MidiFxKind::Mono => "One note at a time: last, lowest or highest held, with legato",
            MidiFxKind::Quantize => "Moves notes that come in late onto the beat grid",
        }
    }

    pub fn specs(self) -> &'static [ParamSpec] {
        match self {
            MidiFxKind::Polymeter => sequencers::POLYMETER,
            MidiFxKind::Step => crate::classic_seq::STEP,
            MidiFxKind::Acid => crate::classic_seq::ACID,
            MidiFxKind::DrumGrid => crate::classic_seq::DRUM_GRID,
            MidiFxKind::Progression => crate::classic_seq::PROGRESSION,
            MidiFxKind::Euclid => sequencers::EUCLID,
            MidiFxKind::Turing => sequencers::TURING,
            MidiFxKind::Life => sequencers::LIFE,
            MidiFxKind::Bounce => sequencers::BOUNCE,
            MidiFxKind::Drunk => sequencers::DRUNK,
            MidiFxKind::Orbits => sequencers::more::ORBITS,
            MidiFxKind::Chaos => sequencers::more::CHAOS,
            MidiFxKind::Automaton => sequencers::more::AUTOMATON,
            MidiFxKind::Ant => sequencers::more::ANT,
            MidiFxKind::Pong => sequencers::more::PONG,
            MidiFxKind::Markov => sequencers::more::MARKOV,
            MidiFxKind::Rain => sequencers::more::RAIN,
            MidiFxKind::Words => sequencers::more::WORDS,
            MidiFxKind::Sing => sequencers::sing::SING,
            MidiFxKind::Touch => sequencers::touch::TOUCH,
            MidiFxKind::Lorenz => sequencers::more::LORENZ,
            MidiFxKind::Loops => sequencers::ambient::LOOPS,
            MidiFxKind::Bloom => sequencers::ambient::BLOOM,
            MidiFxKind::Chimes => sequencers::ambient::CHIMES,
            MidiFxKind::DriftChords => sequencers::ambient::DRIFT_CHORDS,
            MidiFxKind::Tintinnabuli => sequencers::ambient::TINTINNABULI,
            MidiFxKind::Pedal => sequencers::ambient::PEDAL,
            MidiFxKind::Glass => sequencers::minimal::GLASS,
            MidiFxKind::Satie => sequencers::minimal::SATIE,
            MidiFxKind::Riley => sequencers::minimal::RILEY,
            MidiFxKind::Reich => sequencers::composers::REICH,
            MidiFxKind::Bach => sequencers::composers::BACH,
            MidiFxKind::Nancarrow => sequencers::composers::NANCARROW,
            MidiFxKind::Debussy => sequencers::composers::DEBUSSY,
            MidiFxKind::Feldman => sequencers::composers::FELDMAN,
            MidiFxKind::Mozart => sequencers::masters::MOZART,
            MidiFxKind::Chopin => sequencers::masters::CHOPIN,
            MidiFxKind::Ravel => sequencers::masters::RAVEL,
            MidiFxKind::Bartok => sequencers::masters::BARTOK,
            MidiFxKind::Messiaen => sequencers::masters::MESSIAEN,
            MidiFxKind::Ligeti => sequencers::masters::LIGETI,
            MidiFxKind::Cage => sequencers::masters::CAGE,
            MidiFxKind::Einaudi => sequencers::masters::EINAUDI,
            MidiFxKind::OrganGrinder => sequencers::mechanical::ORGAN,
            MidiFxKind::MusicBox => sequencers::mechanical::MUSIC_BOX,
            MidiFxKind::GuitarChords => sequencers::strum::SPECS,
            MidiFxKind::Ukulele => sequencers::strum::UKE_SPECS,
            MidiFxKind::Drummer => sequencers::drummer::SPECS,
            MidiFxKind::DrumMap => sequencers::drum_map::SPECS,
            MidiFxKind::Marbles => sequencers::marbles::SPECS,
            MidiFxKind::Chance => sequencers::chance::SPECS,
            MidiFxKind::Bits => sequencers::bits::SPECS,
            MidiFxKind::Percussion => sequencers::percussion::SPECS,
            MidiFxKind::Spinner => sequencers::played::SPINNER,
            MidiFxKind::Endless => sequencers::played::ENDLESS,
            MidiFxKind::Hands => sequencers::played::HANDS,
            MidiFxKind::Draw => sequencers::played::DRAW,
            MidiFxKind::PianoPlayer => sequencers::players::PIANO,
            MidiFxKind::BassPlayer => sequencers::players::BASS,
            MidiFxKind::Banjo => sequencers::players::BANJO,
            MidiFxKind::Section => sequencers::players::SECTION,
            MidiFxKind::Fireflies => sequencers::nature::FIREFLIES,
            MidiFxKind::Sandpile => sequencers::nature::SANDPILE,
            MidiFxKind::Pendulums => sequencers::nature::PENDULUMS,
            MidiFxKind::Flock => sequencers::nature::FLOCK,
            MidiFxKind::Plants => sequencers::nature::PLANT,
            MidiFxKind::Bells => sequencers::systems::BELLS,
            MidiFxKind::Canon => sequencers::systems::CANON,
            MidiFxKind::Row => sequencers::systems::ROW,
            MidiFxKind::Gears => sequencers::systems::GEARS,
            MidiFxKind::Formula => sequencers::systems::FORMULA,
            MidiFxKind::Staircase => sequencers::systems::STAIRCASE,
            MidiFxKind::Gamelan => sequencers::world::GAMELAN,
            MidiFxKind::Raga => sequencers::world::RAGA,
            MidiFxKind::Call => sequencers::world::CALL,

            MidiFxKind::Arpeggiator => ARP,
            MidiFxKind::Chord => CHORD,
            MidiFxKind::NoteLength => NOTE_LENGTH,
            MidiFxKind::Pitch => PITCH,
            MidiFxKind::Scale => SCALE,
            MidiFxKind::Random => RANDOM,
            MidiFxKind::Velocity => VELOCITY,
            MidiFxKind::NoteEcho => NOTE_ECHO,
            MidiFxKind::Strum => more::STRUM,
            MidiFxKind::NoteRepeat => more::NOTE_REPEAT,
            MidiFxKind::Humanize => more::HUMANIZE,
            MidiFxKind::NoteFilter => more::NOTE_FILTER,
            MidiFxKind::Mono => more::MONO,
            MidiFxKind::Quantize => more::QUANTIZE,
        }
    }
}

const MAX_MAP: usize = 8;

/// Remembers which output notes each input note produced, so that the
/// matching note-offs are always sent even if parameters change meanwhile.
pub struct NoteMap {
    outs: [[u8; MAX_MAP]; 128],
    counts: [u8; 128],
}

impl NoteMap {
    fn new() -> Self {
        Self { outs: [[0; MAX_MAP]; 128], counts: [0; 128] }
    }

    fn release(&mut self, pitch: u8, offset: u32, out: &mut Vec<MidiEvent>) {
        let p = pitch as usize & 127;
        for i in 0..self.counts[p] as usize {
            out.push(MidiEvent::off(offset, self.outs[p][i]));
        }
        self.counts[p] = 0;
    }

    fn start(&mut self, pitch: u8, notes: &[u8], vel: &[f32], offset: u32, out: &mut Vec<MidiEvent>) {
        self.release(pitch, offset, out);
        let p = pitch as usize & 127;
        let mut n = 0;
        for (i, &q) in notes.iter().enumerate() {
            if n >= MAX_MAP || self.outs[p][..n].contains(&q) {
                continue;
            }
            self.outs[p][n] = q;
            n += 1;
            out.push(MidiEvent::on(offset, q, vel.get(i).copied().unwrap_or(vel[0])));
        }
        self.counts[p] = n as u8;
    }

    fn clear(&mut self) {
        self.counts = [0; 128];
    }
}

/// Future events at absolute frame times.
pub struct Scheduler {
    events: Vec<(u64, MidiEvent)>,
}

impl Scheduler {
    fn new() -> Self {
        Self { events: Vec::with_capacity(512) }
    }

    fn push(&mut self, at: u64, ev: MidiEvent) {
        if self.events.len() < self.events.capacity() {
            self.events.push((at, ev));
        }
    }

    /// Moves every event due before the end of this block to `out`.
    fn emit_due(&mut self, ctx: &MidiCtx, out: &mut Vec<MidiEvent>) {
        let end = ctx.now + ctx.len as u64;
        let mut i = 0;
        while i < self.events.len() {
            let (at, mut ev) = self.events[i];
            if at < end {
                ev.offset = at.saturating_sub(ctx.now).min(ctx.len.saturating_sub(1) as u64) as u32;
                out.push(ev);
                self.events.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }
}

pub struct ArpState {
    notes: Vec<(u8, f32)>,
    physical: u32,
    next_step: Option<u64>,
    next_grid: u64,
    step: u64,
    sounding: Vec<(u8, u64)>,
    rng: Rng,
    seq: Vec<(u8, f32)>,
}

impl ArpState {
    fn new() -> Self {
        Self {
            notes: Vec::with_capacity(32),
            physical: 0,
            next_step: None,
            next_grid: 0,
            step: 0,
            sounding: Vec::with_capacity(64),
            rng: Rng::new(0x5EED_A2B1),
            seq: Vec::with_capacity(128),
        }
    }
}

pub enum MidiFxState {
    Seq(Box<SeqState>),
    Arp(Box<ArpState>),
    Mapped(Box<NoteMap>, Rng),
    Scheduled(Box<Scheduler>),
    Velocity(Rng),
    More(Box<more::MoreState>),
}

fn rand01(rng: &mut Rng) -> f32 {
    (rng.bipolar() + 1.0) * 0.5
}

fn chord_intervals(kind: usize) -> &'static [i32] {
    match kind {
        2 => &[0, 4, 7],
        3 => &[0, 3, 7],
        4 => &[0, 4, 7, 11],
        5 => &[0, 3, 7, 10],
        6 => &[0, 4, 7, 10],
        7 => &[0, 2, 7],
        8 => &[0, 5, 7],
        9 => &[0, 7],
        10 => &[0, 12],
        11 => &[0, 4, 7, 14],
        _ => &[0],
    }
}

impl MidiFxState {
    pub fn new(kind: MidiFxKind) -> Self {
        if kind.is_sequencer() {
            return MidiFxState::Seq(Box::default());
        }
        match kind {
            MidiFxKind::Arpeggiator => MidiFxState::Arp(Box::new(ArpState::new())),
            MidiFxKind::NoteLength | MidiFxKind::NoteEcho => MidiFxState::Scheduled(Box::new(Scheduler::new())),
            MidiFxKind::Velocity => MidiFxState::Velocity(Rng::new(0x0BAD_5EED)),
            MidiFxKind::Strum | MidiFxKind::NoteRepeat | MidiFxKind::Humanize | MidiFxKind::NoteFilter | MidiFxKind::Mono | MidiFxKind::Quantize => MidiFxState::More(Box::new(more::MoreState::new())),
            _ => MidiFxState::Mapped(Box::new(NoteMap::new()), Rng::new(0xFEED_1234)),
        }
    }

    pub fn matches(&self, kind: MidiFxKind) -> bool {
        if let MidiFxState::Seq(_) = self {
            return kind.is_sequencer();
        }
        matches!(
            (self, kind),
            (MidiFxState::Arp(_), MidiFxKind::Arpeggiator)
                | (MidiFxState::Scheduled(_), MidiFxKind::NoteLength | MidiFxKind::NoteEcho)
                | (MidiFxState::Velocity(_), MidiFxKind::Velocity)
                | (MidiFxState::More(_), MidiFxKind::Strum | MidiFxKind::NoteRepeat | MidiFxKind::Humanize | MidiFxKind::NoteFilter | MidiFxKind::Mono | MidiFxKind::Quantize)
                | (MidiFxState::Mapped(..), MidiFxKind::Chord | MidiFxKind::Pitch | MidiFxKind::Scale | MidiFxKind::Random)
        )
    }

    /// Forgets everything (transport stop / all notes off).
    pub fn reset(&mut self) {
        match self {
            MidiFxState::Seq(s) => s.reset(),
            MidiFxState::Arp(a) => {
                a.notes.clear();
                a.sounding.clear();
                a.physical = 0;
                a.next_step = None;
            }
            MidiFxState::Mapped(m, _) => m.clear(),
            MidiFxState::Scheduled(s) => s.events.clear(),
            MidiFxState::Velocity(_) => {}
            MidiFxState::More(m) => m.reset(),
        }
    }

    /// Packed visual state of a sequencer (for the UI and grid controllers).
    pub fn visual(&self) -> Option<[u64; 4]> {
        match self {
            MidiFxState::Seq(s) => Some(s.visual),
            _ => None,
        }
    }

    /// A Touch device's outputs (see `sequencers::touch::OUTPUTS`).
    pub fn touch_out(&self) -> Option<[f32; sequencers::touch::N_OUT]> {
        match self {
            MidiFxState::Seq(s) => Some(s.touch_out),
            _ => None,
        }
    }

    /// Transforms `input` (sorted by offset) into `out`. `data` is the
    /// device's pattern data (sequencers).
    pub fn process(&mut self, kind: MidiFxKind, p: &[f32], data: &[u8], input: &[MidiEvent], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        match (self, kind) {
            (MidiFxState::Seq(s), kind) => {
                // Sequencers add notes; what comes in is passed through.
                out.extend_from_slice(input);
                s.process(kind, p, data, out, ctx);
            }
            (MidiFxState::Arp(a), MidiFxKind::Arpeggiator) => arp(a, p, input, out, ctx),
            (MidiFxState::Mapped(map, rng), kind) => {
                let scale = scale_for(kind, p, ctx);
                for e in input {
                    if !e.on {
                        map.release(e.pitch, e.offset, out);
                        continue;
                    }
                    let mut notes = [0u8; MAX_MAP];
                    let mut vels = [e.vel; MAX_MAP];
                    let n = map_note(kind, p, e.pitch, e.vel, &scale, ctx.scale_on, rng, &mut notes, &mut vels);
                    if n > 0 {
                        map.start(e.pitch, &notes[..n], &vels[..n], e.offset, out);
                    }
                }
            }
            (MidiFxState::Scheduled(s), MidiFxKind::NoteLength) => {
                let len = LENGTH_BEATS[(p[0].round() as usize).min(7)] * p[1] as f64;
                let frames = (len * ctx.samples_per_beat).max(1.0) as u64;
                for e in input.iter().filter(|e| e.on) {
                    let at = ctx.now + e.offset as u64;
                    // Retrigger: end an earlier copy of this note now.
                    if let Some(pos) = s.events.iter().position(|(_, ev)| ev.pitch == e.pitch) {
                        s.events.swap_remove(pos);
                        out.push(MidiEvent::off(e.offset, e.pitch));
                    }
                    out.push(*e);
                    s.push(at + frames, MidiEvent::off(0, e.pitch));
                }
                s.emit_due(ctx, out);
            }
            (MidiFxState::Scheduled(s), MidiFxKind::NoteEcho) => {
                let time = ECHO_BEATS[(p[0].round() as usize).min(6)];
                let step = (time * ctx.samples_per_beat).max(1.0) as u64;
                let repeats = p[1].round().clamp(1.0, 8.0) as u64;
                let fb = p[2];
                let shift = p[3].round() as i32;
                for e in input {
                    out.push(*e);
                    let at = ctx.now + e.offset as u64;
                    let mut vel = e.vel;
                    for k in 1..=repeats {
                        let pitch = (e.pitch as i32 + shift * k as i32).clamp(0, 127) as u8;
                        if e.on {
                            vel *= fb;
                            if vel < 0.02 {
                                break;
                            }
                            s.push(at + k * step, MidiEvent::on(0, pitch, vel));
                        } else {
                            s.push(at + k * step, MidiEvent::off(0, pitch));
                        }
                    }
                }
                s.emit_due(ctx, out);
            }
            (MidiFxState::Velocity(rng), MidiFxKind::Velocity) => {
                for e in input {
                    let mut e = *e;
                    if e.on {
                        let (lo, hi) = (p[1].min(p[2]), p[1].max(p[2]));
                        let v = if p[4] >= 0.5 {
                            hi
                        } else {
                            let curved = e.vel.clamp(0.0, 1.0).powf(2f32.powf(-p[0] * 2.0));
                            lo + (hi - lo) * curved
                        };
                        let jitter = rng.bipolar() * p[3] * 0.5;
                        e.vel = (v + jitter).clamp(0.01, 1.0);
                    }
                    out.push(e);
                }
            }
            (MidiFxState::More(m), kind) => m.process(kind, p, input, out, ctx),
            _ => out.extend_from_slice(input),
        }
    }
}

fn scale_for(kind: MidiFxKind, p: &[f32], ctx: &MidiCtx) -> Scale {
    if kind == MidiFxKind::Scale && p[0] >= 0.5 {
        let kind = ScaleKind::ALL[(p[2].round() as usize).min(ScaleKind::ALL.len() - 1)];
        Scale::new(p[1].round() as u8, kind)
    } else {
        ctx.scale
    }
}

/// Output notes for one input note of a stateless-per-note device.
#[allow(clippy::too_many_arguments)]
fn map_note(kind: MidiFxKind, p: &[f32], pitch: u8, vel: f32, scale: &Scale, scale_on: bool, rng: &mut Rng, notes: &mut [u8; MAX_MAP], vels: &mut [f32; MAX_MAP]) -> usize {
    let clamp = |x: i32| x.clamp(0, 127) as u8;
    match kind {
        MidiFxKind::Chord => {
            let ty = p[0].round() as usize;
            let mut list = [0i32; MAX_MAP];
            let mut n = 0;
            match ty {
                0 | 1 => {
                    let chord = scale.chord(pitch, if ty == 0 { 3 } else { 4 });
                    // Keep the played note as the root even if it is outside the scale.
                    let shift = pitch as i32 - chord.first().copied().unwrap_or(pitch) as i32;
                    for c in chord {
                        list[n] = c as i32 + shift;
                        n += 1;
                    }
                }
                12 => {
                    list[0] = pitch as i32;
                    n = 1;
                    for k in 3..7 {
                        let s = p[k].round() as i32;
                        if s != 0 {
                            list[n] = pitch as i32 + s;
                            n += 1;
                        }
                    }
                }
                _ => {
                    for &iv in chord_intervals(ty) {
                        list[n] = pitch as i32 + iv;
                        n += 1;
                    }
                }
            }
            // Inversions: move the lowest notes up (or the highest down) an octave.
            let inv = p[1].round() as i32;
            let notes_slice = &mut list[..n];
            notes_slice.sort_unstable();
            for _ in 0..inv.max(0) {
                notes_slice[0] += 12;
                notes_slice.sort_unstable();
            }
            for _ in 0..(-inv).max(0) {
                let last = notes_slice.len() - 1;
                notes_slice[last] -= 12;
                notes_slice.sort_unstable();
            }
            for i in 0..n {
                notes[i] = clamp(list[i]);
                vels[i] = if list[i] == pitch as i32 { vel } else { vel * p[2] };
            }
            n
        }
        MidiFxKind::Pitch => {
            let mut q = pitch as i32 + p[0].round() as i32;
            let steps = p[1].round() as i32;
            if steps != 0 {
                q = scale.step(clamp(q), steps) as i32;
            }
            notes[0] = clamp(q);
            1
        }
        MidiFxKind::Scale => {
            let filter = p[3] >= 0.5;
            if filter && !scale.contains(pitch) {
                return 0;
            }
            notes[0] = scale.snap(pitch);
            1
        }
        MidiFxKind::Random => {
            notes[0] = pitch;
            if rand01(rng) < p[0] {
                let choices = p[1].round().max(1.0) as i32;
                let k = 1 + (rand01(rng) * choices as f32).floor().min(choices as f32 - 1.0) as i32;
                let dir = match p[3].round() as i32 {
                    0 => 1,
                    1 => -1,
                    _ => {
                        if rng.bipolar() >= 0.0 {
                            1
                        } else {
                            -1
                        }
                    }
                };
                let in_key = p[4] >= 0.5 && scale_on;
                notes[0] = if in_key { scale.step(pitch, dir * k * p[2].round().max(1.0) as i32) } else { clamp(pitch as i32 + dir * k * p[2].round() as i32) };
            }
            1
        }
        _ => {
            notes[0] = pitch;
            1
        }
    }
}

fn arp(a: &mut ArpState, p: &[f32], input: &[MidiEvent], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
    let hold = p[5] >= 0.5;
    for e in input {
        if e.on {
            if hold && a.physical == 0 {
                a.notes.clear();
            }
            a.physical += 1;
            if !a.notes.iter().any(|n| n.0 == e.pitch) && a.notes.len() < a.notes.capacity() {
                a.notes.push((e.pitch, e.vel));
            }
            if a.next_step.is_none() {
                let at = ctx.now + e.offset as u64;
                a.next_step = Some(at);
                a.next_grid = at;
                a.step = 0;
            }
        } else {
            a.physical = a.physical.saturating_sub(1);
            if !hold {
                a.notes.retain(|n| n.0 != e.pitch);
            }
        }
    }
    if !hold && a.physical == 0 {
        a.notes.clear();
    }
    if a.notes.is_empty() {
        a.next_step = None;
    }

    let rate = ARP_RATE_BEATS[(p[1].round() as usize).min(7)];
    let step_len = (rate * ctx.samples_per_beat).max(1.0) as u64;
    let gate = ((p[2] as f64 * step_len as f64) as u64).max(1);
    let octaves = p[3].round().clamp(1.0, 4.0) as u8;
    let swing = (p[4] as f64 * step_len as f64 * 0.5) as u64;
    let style = p[0].round() as usize;
    let follow = crate::band::follow_mode(MidiFxKind::Arpeggiator, p);
    let end = ctx.now + ctx.len as u64;
    let offset_of = |t: u64| t.saturating_sub(ctx.now).min(ctx.len.saturating_sub(1) as u64) as u32;

    // Build the note order for this block.
    a.seq.clear();
    let mut base: Vec<(u8, f32)> = Vec::new();
    std::mem::swap(&mut base, &mut a.seq);
    base.extend_from_slice(&a.notes);
    if style != 7 {
        base.sort_unstable_by_key(|n| n.0);
    }
    let n0 = base.len();
    for o in 1..octaves {
        for i in 0..n0 {
            let (pch, v) = base[i];
            if (pch as u32 + 12 * o as u32) <= 127 {
                base.push((pch + 12 * o, v));
            }
        }
    }
    let n = base.len();

    loop {
        let next_off = a.sounding.iter().map(|s| s.1).min();
        let step_at = if n > 0 { a.next_step } else { None };
        let t = match (next_off, step_at) {
            (Some(o), Some(s)) => o.min(s),
            (Some(o), None) => o,
            (None, Some(s)) => s,
            (None, None) => break,
        };
        if t >= end {
            break;
        }
        if next_off == Some(t) {
            if let Some(pos) = a.sounding.iter().position(|s| s.1 == t) {
                let (pitch, _) = a.sounding.swap_remove(pos);
                out.push(MidiEvent::off(offset_of(t), pitch));
            }
            continue;
        }
        // A step: pick the note(s).
        let s = a.step as usize;
        let mut play = |pitch: u8, vel: f32, a: &mut ArpState| {
            // With the band: moved with its chord here.
            let pitch = match follow {
                Some(mode) => crate::band::follow(ctx, mode, crate::band::beat_at(ctx, t), pitch as i32).clamp(0, 127) as u8,
                None => pitch,
            };
            if let Some(pos) = a.sounding.iter().position(|x| x.0 == pitch) {
                a.sounding.swap_remove(pos);
                out.push(MidiEvent::off(offset_of(t), pitch));
            }
            out.push(MidiEvent::on(offset_of(t), pitch, vel));
            if a.sounding.len() < a.sounding.capacity() {
                a.sounding.push((pitch, t + gate));
            }
        };
        match style {
            8 => {
                for i in 0..n {
                    let (pch, v) = base[i];
                    play(pch, v, a);
                }
            }
            _ => {
                let idx = match style {
                    1 => n - 1 - s % n,
                    2 | 3 => {
                        let i = if n == 1 {
                            0
                        } else {
                            let period = 2 * n - 2;
                            let k = s % period;
                            if k < n {
                                k
                            } else {
                                period - k
                            }
                        };
                        if style == 3 {
                            n - 1 - i
                        } else {
                            i
                        }
                    }
                    4 | 5 => {
                        let i = s % n;
                        let conv = |i: usize| if i % 2 == 0 { i / 2 } else { n - 1 - i / 2 };
                        if style == 4 {
                            conv(i)
                        } else {
                            conv(n - 1 - i)
                        }
                    }
                    6 => (a.rng.next_u32() as usize) % n,
                    _ => s % n,
                };
                let (pch, v) = base[idx];
                play(pch, v, a);
            }
        }
        a.step += 1;
        a.next_grid += step_len;
        a.next_step = Some(a.next_grid + if a.step % 2 == 1 { swing } else { 0 });
    }
    base.clear();
    a.seq = base;
}

/// What a sequencer plays over its first `beats` beats, as a clip's notes
/// ("To Clip"): played offline, block by block from the song's start, at
/// `bpm`, in the project's key (`scale`, `scale_on` as the engine has them)
/// and for the track's kit (`drum_notes`). Notes still sounding at the end
/// end there.
#[allow(clippy::too_many_arguments)]
pub fn render_notes(kind: MidiFxKind, params: &[f32], data: &[u8], beats: f64, bpm: f64, scale: Scale, scale_on: bool, drum_notes: u128) -> Vec<crate::project::Note> {
    use crate::project::Note;
    const SR: f32 = 48_000.0;
    const BLOCK: usize = 256;
    let spb = SR as f64 * 60.0 / bpm.clamp(20.0, 999.0);
    // As in the engine: with the key off, all twelve notes, unless the device keeps a key itself.
    let scale = if scale_on || kind.keeps_key() { scale } else { Scale::new(scale.root, ScaleKind::Chromatic) };
    let base = MidiCtx { now: 0, len: BLOCK, samples_per_beat: spb, scale, scale_on, beat: 0.0, playing: true, sr: SR, drum_notes, lead: 0.0, touch: [0.0; 5], band: Default::default() };
    let mut st = MidiFxState::new(kind);
    let total = (beats.max(0.0) * spb).ceil() as u64;
    // Starts and ends land on a 1/960 beat grid, so a step is where it belongs.
    let snap = |frame: u64| (frame as f64 / spb * 960.0).round() / 960.0;
    let mut held: Vec<(u8, f64, f32, Lyric)> = Vec::new();
    let mut notes = Vec::new();
    let mut out = Vec::new();
    let mut frame = 0u64;
    while frame < total {
        let len = BLOCK.min((total - frame) as usize);
        let ctx = MidiCtx { now: frame, len, beat: frame as f64 / spb, ..base };
        out.clear();
        st.process(kind, params, data, &[], &mut out, &ctx);
        out.sort_by_key(|e| (e.offset, e.on));
        for e in &out {
            let at = snap(frame + e.offset as u64);
            // An off ends the note; an on of a note still sounding ends it too.
            if let Some(i) = held.iter().position(|h| h.0 == e.pitch) {
                let (pitch, start, vel, lyric) = held.swap_remove(i);
                notes.push(Note { lyric: lyric.as_str().to_string(), ..Note::new(pitch, start, (at - start).max(1.0 / 64.0), vel) });
            }
            if e.on && at < beats {
                held.push((e.pitch, at, e.vel.clamp(0.0, 1.0), e.lyric));
            }
        }
        frame += len as u64;
    }
    for (pitch, start, vel, lyric) in held {
        notes.push(Note { lyric: lyric.as_str().to_string(), ..Note::new(pitch, start, (beats - start).max(1.0 / 64.0), vel) });
    }
    notes.sort_by(|a, b| a.start.total_cmp(&b.start).then(a.pitch.cmp(&b.pitch)));
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::defaults;

    #[test]
    fn a_sequencer_renders_into_notes_inside_the_clip() {
        let k = MidiFxKind::Step;
        let p = defaults(k.specs());
        let data = k.default_data();
        let render = |beats: f64| render_notes(k, &p, &data, beats, 120.0, Scale::new(0, ScaleKind::Minor), true, 0);
        let bar = render(4.0);
        assert!(!bar.is_empty(), "the default pattern plays something");
        assert!(bar.iter().all(|n| n.start >= 0.0 && n.start < 4.0 && n.length > 0.0 && n.start + n.length <= 4.0 + 1e-9));
        // The first bar of two is that bar again (tempo does not move a note).
        let two = render_notes(k, &p, &data, 8.0, 90.0, Scale::new(0, ScaleKind::Minor), true, 0);
        let first: Vec<(u8, f64)> = two.iter().filter(|n| n.start < 4.0).map(|n| (n.pitch, n.start)).collect();
        assert_eq!(first, bar.iter().map(|n| (n.pitch, n.start)).collect::<Vec<_>>());
        // Steps (and the default's one ratchet, two to a step) sit on the grid.
        assert!(bar.iter().all(|n| (n.start * 8.0 - (n.start * 8.0).round()).abs() < 1e-6), "{:?}", bar.iter().map(|n| n.start).collect::<Vec<_>>());
    }

    fn ctx(now: u64, len: usize) -> MidiCtx {
        MidiCtx { now, len, samples_per_beat: 24000.0, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: 0.0, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 5], band: Default::default() }
    }

    #[test]
    fn every_sequencer_has_a_group() {
        for k in MidiFxKind::SEQUENCERS {
            assert!(MidiFxKind::SEQUENCER_GROUPS.contains(&k.group()), "{k:?}");
        }
        for k in MidiFxKind::ALL {
            assert_eq!(k.group(), "MIDI Effects", "{k:?}");
        }
    }

    /// Runs a device over `blocks` blocks of 256 frames; `input` holds (absolute frame, event).
    fn run(kind: MidiFxKind, p: &[f32], input: &[(u64, MidiEvent)], blocks: u64) -> Vec<(u64, MidiEvent)> {
        let mut st = MidiFxState::new(kind);
        let mut out_all = Vec::new();
        for b in 0..blocks {
            let now = b * 256;
            let evs: Vec<MidiEvent> = input
                .iter()
                .filter(|(t, _)| *t >= now && *t < now + 256)
                .map(|(t, e)| MidiEvent { offset: (t - now) as u32, ..*e })
                .collect();
            let mut out = Vec::new();
            st.process(kind, p, &[], &evs, &mut out, &ctx(now, 256));
            out_all.extend(out.into_iter().map(|e| (now + e.offset as u64, e)));
        }
        out_all
    }

    #[test]
    fn chord_in_key_and_types() {
        let mut p = defaults(MidiFxKind::Chord.specs());
        let out = run(MidiFxKind::Chord, &p, &[(0, MidiEvent::on(0, 62, 1.0)), (1000, MidiEvent::off(0, 62))], 8);
        let ons: Vec<u8> = out.iter().filter(|e| e.1.on).map(|e| e.1.pitch).collect();
        assert_eq!(ons, vec![62, 65, 69], "D minor triad in C major");
        assert_eq!(out.iter().filter(|e| !e.1.on).count(), 3);
        p[0] = 2.0; // Major
        p[1] = 1.0; // first inversion
        let out = run(MidiFxKind::Chord, &p, &[(0, MidiEvent::on(0, 60, 1.0))], 1);
        let mut ons: Vec<u8> = out.iter().map(|e| e.1.pitch).collect();
        ons.sort();
        assert_eq!(ons, vec![64, 67, 72]);
    }

    #[test]
    fn arpeggiator_steps_in_time() {
        let p = defaults(MidiFxKind::Arpeggiator.specs()); // Up, 1/16, gate 75 %
        let input = [(0, MidiEvent::on(0, 60, 0.9)), (0, MidiEvent::on(0, 64, 0.9)), (0, MidiEvent::on(0, 67, 0.9)), (48000, MidiEvent::off(0, 60)), (48000, MidiEvent::off(0, 64)), (48000, MidiEvent::off(0, 67))];
        let out = run(MidiFxKind::Arpeggiator, &p, &input, 300);
        let ons: Vec<(u64, u8)> = out.iter().filter(|e| e.1.on).map(|e| (e.0, e.1.pitch)).collect();
        // Two beats held at 1/16 = 8 steps.
        assert_eq!(ons.len(), 8, "{ons:?}");
        assert_eq!(ons[0], (0, 60));
        assert_eq!(ons[1], (6000, 64));
        assert_eq!(ons[2], (12000, 67));
        assert_eq!(ons[3], (18000, 60));
        // Every note ends.
        assert_eq!(out.iter().filter(|e| !e.1.on).count(), 8);
    }

    #[test]
    fn arpeggiator_hold_keeps_playing() {
        let mut p = defaults(MidiFxKind::Arpeggiator.specs());
        p[5] = 1.0;
        let input = [(0, MidiEvent::on(0, 60, 0.9)), (100, MidiEvent::off(0, 60))];
        let out = run(MidiFxKind::Arpeggiator, &p, &input, 200);
        assert!(out.iter().filter(|e| e.1.on).count() >= 8);
    }

    #[test]
    fn note_length_and_echo() {
        let p = defaults(MidiFxKind::NoteLength.specs()); // 1/16 = 6000 frames
        let out = run(MidiFxKind::NoteLength, &p, &[(10, MidiEvent::on(0, 60, 1.0)), (100, MidiEvent::off(0, 60))], 40);
        assert_eq!(out.len(), 2);
        assert_eq!(out[1], (6010, MidiEvent::off(out[1].1.offset, 60)));

        let p = defaults(MidiFxKind::NoteEcho.specs()); // 1/8 = 12000 frames, 3 repeats
        let out = run(MidiFxKind::NoteEcho, &p, &[(0, MidiEvent::on(0, 60, 1.0)), (1000, MidiEvent::off(0, 60))], 200);
        let ons: Vec<u64> = out.iter().filter(|e| e.1.on).map(|e| e.0).collect();
        assert_eq!(ons, vec![0, 12000, 24000, 36000]);
        assert_eq!(out.iter().filter(|e| !e.1.on).count(), 4);
        assert!(out.iter().filter(|e| e.1.on).last().unwrap().1.vel < 0.3);
    }

    #[test]
    fn pitch_scale_random_velocity() {
        let mut p = defaults(MidiFxKind::Pitch.specs());
        p[1] = 2.0; // two scale steps
        let out = run(MidiFxKind::Pitch, &p, &[(0, MidiEvent::on(0, 60, 1.0)), (10, MidiEvent::off(0, 60))], 1);
        assert_eq!(out[0].1.pitch, 64);
        assert_eq!(out[1].1.pitch, 64, "note-off follows the transposed note");

        let mut p = defaults(MidiFxKind::Scale.specs());
        let out = run(MidiFxKind::Scale, &p, &[(0, MidiEvent::on(0, 61, 1.0))], 1);
        assert_eq!(out[0].1.pitch, 60);
        p[3] = 1.0; // filter
        let out = run(MidiFxKind::Scale, &p, &[(0, MidiEvent::on(0, 61, 1.0))], 1);
        assert!(out.is_empty());

        let mut p = defaults(MidiFxKind::Random.specs());
        p[0] = 1.0;
        let input: Vec<(u64, MidiEvent)> = (0..50).map(|i| (i * 10, MidiEvent::on(0, 60, 1.0))).collect();
        let out = run(MidiFxKind::Random, &p, &input, 4);
        let s = Scale::new(0, ScaleKind::Major);
        assert!(out.iter().filter(|e| e.1.on).all(|e| s.contains(e.1.pitch)));
        assert!(out.iter().any(|e| e.1.on && e.1.pitch != 60));

        let mut p = defaults(MidiFxKind::Velocity.specs());
        p[1] = 0.5;
        p[2] = 0.6;
        let out = run(MidiFxKind::Velocity, &p, &[(0, MidiEvent::on(0, 60, 1.0)), (1, MidiEvent::on(0, 62, 0.1))], 1);
        assert!(out.iter().all(|e| (0.49..=0.61).contains(&e.1.vel)));
    }
}
