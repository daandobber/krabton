//! Analog Princess, the machine: a drum machine for deep-fried pluggnb,
//! made of Analog Princess's voices (see `princess`), with a step sequencer
//! of its own and a master FRY section that the whole kit goes through.
//!
//! - Twelve tracks: 808, Kick, Snare, Clap, Hat, Open Hat, Rim, Perc, Bell,
//!   Laser, Surf and Subliminal. Each plays one of the voices (Voice) and
//!   goes through its own chain: the drop in pitch at the hit (Pitch Env,
//!   Pitch Time), Drive, Clip, a ring modulator (Ring, Ring Hz), a crusher
//!   (Bits, Downsample), a filter (Filter: low pass to the left, high pass to
//!   the right), then a send to the room (Room) and to the FRY section (Fry;
//!   the rest goes past it), Pan, Level, a choke group and Mute. A track
//!   playing the 808 takes the 808's knobs too (Glide, Distortion, Sub,
//!   Click, the ringing 808's tune and decay) and its notes: in the song's
//!   key or chromatic, from its Root.
//! - Patterns A to H of 16, 32 or 64 steps, each track with a length of its
//!   own if it likes (polymeter). A step has its velocity, an accent, a roll
//!   (2 or 4 hits in the step, 3 or 6 over two steps for triplets, 8 over two,
//!   or seven hits over a beat ramping up or down), a flam, a chance, a note
//!   and a slide (into the next step's note, without a new hit). Swing moves
//!   every second sixteenth later. A pattern picked while it plays starts
//!   when the one playing ends; Chain plays the patterns in a row instead (A
//!   A B A, say). It plays on the song's transport, and from notes: each
//!   track on a note of its own (General MIDI where there is one), the 808
//!   also on the keys below them (MIDI 18 to 34) at their pitch.
//! - FRY: the tracks' sends through a ring modulator, Drive and a crusher
//!   (Bits, Downsample), then with the rest of the kit Pump (all but the key
//!   ducking under the kick, or the 808), Stutter (a beat repeated in eighths
//!   to thirty-seconds, now and then or while held), Tape Stop (slowing to a
//!   stop into the bar line, or while held), Channel Surf (the whole kit
//!   flipping between radio, muffled, tinny and pitched channels and static
//!   for a moment), Clip into the ceiling (soft or hard) behind a limiter that
//!   keeps the peaks under it. Deep Fry turns most of them up at once.

use super::drums::DrumVoice;
use super::princess::{Dist, Knobs808, Princess808, PrincessVoice, BASE_808};
use super::reverb::Reverb;
use super::*;
use crate::params::{ParamSpec, Unit};
use crate::scale::{Scale, ScaleKind};
use serde::{Deserialize, Serialize};

pub const TRACKS: usize = 12;
pub const PATTERNS: usize = 8;
/// The most steps a pattern has.
pub const MAX_STEPS: usize = 64;
pub const TRACK_NAMES: [&str; TRACKS] = ["808", "Kick", "Snare", "Clap", "Hat", "Open Hat", "Rim", "Perc", "Bell", "Laser", "Surf", "Subliminal"];
/// The notes the tracks play on.
pub const NOTES: [u8; TRACKS] = [35, 36, 38, 39, 42, 46, 37, 41, 53, 48, 49, 55];
/// The keys below the drums that play the 808 at their own pitch.
pub const KEYS_808: std::ops::RangeInclusive<u8> = 18..=34;
pub const PATTERN_NAMES: [&str; PATTERNS] = ["A", "B", "C", "D", "E", "F", "G", "H"];
/// The tracks by their place.
pub const BASS: usize = 0;
pub const KICK: usize = 1;
pub const SNARE: usize = 2;
pub const CLAP: usize = 3;
pub const HAT: usize = 4;
pub const OPEN: usize = 5;
pub const RIM: usize = 6;
pub const PERC: usize = 7;
pub const BELL: usize = 8;
pub const LASER: usize = 9;
pub const SURF: usize = 10;
pub const SUB: usize = 11;

/// The voices a track can play.
pub const VOICES: &[&str] = &["808", "Kick", "Snare", "Clap", "Hat", "Open Hat", "Hat Roll", "Rim", "808 Tom", "Bell", "Laser", "Surf", "Subliminal"];
/// What a roll does: off, so many hits, or a ramp.
pub const ROLLS: &[&str] = &["Off", "2", "3", "4", "6", "8", "Up", "Down"];
/// Chances a step can have (%).
pub const CHANCES: [u8; 6] = [100, 75, 50, 33, 25, 12];

/// The voice behind a choice of `VOICES` (None: the machine's 808).
pub fn drum_voice(i: usize) -> Option<DrumVoice> {
    use DrumVoice::*;
    Some(match i {
        1 => ApKick,
        2 => ApSnare,
        3 => ApClap,
        4 => ApHat,
        5 => ApOpenHat,
        6 => ApHatRoll,
        7 => ApBitPerc,
        8 => ApFried808,
        9 => ApBell,
        10 => ApLaser,
        11 => ApChannelSurf,
        12 => ApSubliminal,
        _ => return None,
    })
}

/// The names of a voice's two knobs of its own.
pub fn voice_knobs(i: usize) -> [&'static str; 2] {
    match drum_voice(i) {
        None => ["Ring", "Feed"],
        Some(DrumVoice::ApFried808) => ["Glide", "Ring"],
        Some(v) => {
            let [a, b] = v.macros();
            [a.unwrap_or("Color"), b.unwrap_or("Shape")]
        }
    }
}

// ------------------------------------------------------------ parameters

/// A track's parameters, in this order.
pub const T_VOICE: usize = 0;
pub const T_TUNE: usize = 1;
pub const T_DECAY: usize = 2;
pub const T_TONE: usize = 3;
pub const T_A: usize = 4;
pub const T_B: usize = 5;
pub const T_PITCH: usize = 6;
pub const T_PITCH_TIME: usize = 7;
pub const T_DRIVE: usize = 8;
pub const T_CLIP: usize = 9;
pub const T_RING: usize = 10;
pub const T_RING_HZ: usize = 11;
pub const T_BITS: usize = 12;
pub const T_RATE: usize = 13;
pub const T_FILTER: usize = 14;
pub const T_ROOM: usize = 15;
pub const T_FRY: usize = 16;
pub const T_PAN: usize = 17;
pub const T_LEVEL: usize = 18;
pub const T_CHOKE: usize = 19;
pub const T_MUTE: usize = 20;
/// Parameters a track has.
pub const TP: usize = 21;

/// Track `t`'s parameter `k`.
pub const fn tp(t: usize, k: usize) -> usize {
    t * TP + k
}

macro_rules! track {
    ($t:literal, $voice:expr, $tone:expr, $a:expr, $b:expr, $level:expr, $choke:expr) => {
        [
            ParamSpec::choice(concat!($t, " Voice"), $t, VOICES, $voice),
            ParamSpec::lin(concat!($t, " Tune"), $t, -24.0, 24.0, 0.0, Unit::Semi),
            ParamSpec::log(concat!($t, " Decay"), $t, 0.1, 4.0, 1.0, Unit::None),
            ParamSpec::lin(concat!($t, " Tone"), $t, 0.0, 1.0, $tone, Unit::Percent),
            ParamSpec::lin(concat!($t, " Color"), $t, 0.0, 1.0, $a, Unit::Percent),
            ParamSpec::lin(concat!($t, " Shape"), $t, 0.0, 1.0, $b, Unit::Percent),
            ParamSpec::lin(concat!($t, " Pitch Env"), $t, 0.0, 4.0, 1.0, Unit::None),
            ParamSpec::log(concat!($t, " Pitch Time"), $t, 0.25, 4.0, 1.0, Unit::None),
            ParamSpec::lin(concat!($t, " Drive"), $t, 0.0, 1.0, 0.0, Unit::Percent),
            ParamSpec::lin(concat!($t, " Clip"), $t, 0.0, 1.0, 0.0, Unit::Percent),
            ParamSpec::lin(concat!($t, " Ring"), $t, 0.0, 1.0, 0.0, Unit::Percent),
            ParamSpec::log(concat!($t, " Ring Hz"), $t, 30.0, 5000.0, 440.0, Unit::Hz),
            ParamSpec::int(concat!($t, " Bits"), $t, 2.0, 16.0, 16.0, Unit::None),
            ParamSpec::log(concat!($t, " Downsample"), $t, 1.0, 32.0, 1.0, Unit::None),
            ParamSpec::lin(concat!($t, " Filter"), $t, -1.0, 1.0, 0.0, Unit::None),
            ParamSpec::lin(concat!($t, " Room"), $t, 0.0, 1.0, 0.0, Unit::Percent),
            ParamSpec::lin(concat!($t, " Fry"), $t, 0.0, 1.0, 1.0, Unit::Percent),
            ParamSpec::lin(concat!($t, " Pan"), $t, -1.0, 1.0, 0.0, Unit::Pan),
            ParamSpec::lin(concat!($t, " Level"), $t, -40.0, 6.0, $level, Unit::Db),
            ParamSpec::int(concat!($t, " Choke"), $t, 0.0, 4.0, $choke, Unit::None),
            ParamSpec::toggle(concat!($t, " Mute"), $t, false),
        ]
    };
}

const TRACK_SPECS: [[ParamSpec; TP]; TRACKS] = [
    track!("808", 0, 0.5, 0.3, 0.0, -2.0, 0.0),
    track!("Kick", 1, 0.5, 0.5, 0.35, 0.0, 0.0),
    track!("Snare", 2, 0.5, 0.35, 0.4, -2.0, 0.0),
    track!("Clap", 3, 0.5, 0.3, 0.3, -3.0, 0.0),
    track!("Hat", 4, 0.5, 0.5, 0.4, -7.0, 1.0),
    track!("Open Hat", 5, 0.5, 0.5, 0.5, -9.0, 1.0),
    track!("Rim", 7, 0.5, 0.3, 0.3, -8.0, 0.0),
    track!("Perc", 8, 0.6, 0.0, 0.2, -6.0, 0.0),
    track!("Bell", 9, 0.5, 0.35, 0.6, -9.0, 0.0),
    track!("Laser", 10, 0.5, 0.5, 0.5, -10.0, 0.0),
    track!("Surf", 11, 0.5, 0.5, 0.5, -12.0, 0.0),
    track!("Subliminal", 12, 0.5, 0.5, 0.0, -10.0, 0.0),
];

const G: usize = TRACKS * TP;
// The machine.
pub const PATTERN: usize = G;
pub const CHAIN: usize = G + 1;
pub const SWING: usize = G + 2;
pub const SEQUENCER: usize = G + 3;
pub const FLAM: usize = G + 4;
pub const ROOM_SIZE: usize = G + 5;
// The 808.
pub const GLIDE: usize = G + 6;
pub const DIST: usize = G + 7;
pub const SUB_808: usize = G + 8;
pub const CLICK: usize = G + 9;
pub const RING_TUNE: usize = G + 10;
pub const RING_DECAY: usize = G + 11;
pub const ROOT: usize = G + 12;
pub const SCALE: usize = G + 13;
// FRY.
pub const DEEP_FRY: usize = G + 14;
pub const F_DRIVE: usize = G + 15;
pub const F_CLIP: usize = G + 16;
pub const F_CLIP_MODE: usize = G + 17;
pub const F_BITS: usize = G + 18;
pub const F_DOWN: usize = G + 19;
pub const F_RING: usize = G + 20;
pub const F_RING_HZ: usize = G + 21;
pub const PUMP: usize = G + 22;
pub const PUMP_TIME: usize = G + 23;
pub const PUMP_KEY: usize = G + 24;
pub const STUTTER: usize = G + 25;
pub const STUTTER_RATE: usize = G + 26;
pub const STUTTER_HOLD: usize = G + 27;
pub const TAPE: usize = G + 28;
pub const TAPE_TIME: usize = G + 29;
pub const TAPE_HOLD: usize = G + 30;
pub const CHANNEL_SURF: usize = G + 31;
pub const SURF_HOLD: usize = G + 32;
pub const CEILING: usize = G + 33;
pub const VOLUME: usize = G + 34;
const NG: usize = 35;

pub const DISTS: &[&str] = &["Soft", "Hard", "Fold", "Tube"];
pub const STUTTER_RATES: &[&str] = &["1/8", "1/16", "1/32", "Mixed"];
pub const TAPE_TIMES: &[&str] = &["1/4 Beat", "1/2 Beat", "1 Beat", "2 Beats"];

const GLOBAL_SPECS: [ParamSpec; NG] = [
    ParamSpec::choice("Pattern", "Machine", &PATTERN_NAMES, 0),
    ParamSpec::toggle("Chain", "Machine", false),
    ParamSpec::lin("Swing", "Machine", 0.5, 0.75, 0.5, Unit::Percent),
    ParamSpec::toggle("Sequencer", "Machine", true),
    ParamSpec::lin("Flam", "Machine", 5.0, 60.0, 22.0, Unit::Ms),
    ParamSpec::lin("Room Size", "Machine", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("808 Glide", "808", 0.01, 1.0, 0.12, Unit::Sec),
    ParamSpec::choice("808 Distortion", "808", DISTS, 1),
    ParamSpec::lin("808 Sub", "808", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("808 Click", "808", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("808 Ring Tune", "808", 0.0, 24.0, 7.0, Unit::Semi),
    ParamSpec::log("808 Ring Decay", "808", 0.1, 4.0, 1.0, Unit::None),
    ParamSpec::choice("808 Root", "808", &crate::scale::NOTE_NAMES, 0),
    ParamSpec::choice("808 Scale", "808", &["Chromatic", "Song Key"], 1),
    ParamSpec::lin("Deep Fry", "Fry", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Fry Drive", "Fry", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Fry Clip", "Fry", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::choice("Clip Mode", "Fry", &["Soft", "Hard"], 1),
    ParamSpec::int("Fry Bits", "Fry", 2.0, 16.0, 16.0, Unit::None),
    ParamSpec::log("Fry Downsample", "Fry", 1.0, 32.0, 1.0, Unit::None),
    ParamSpec::lin("Fry Ring", "Fry", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Fry Ring Hz", "Fry", 30.0, 3000.0, 180.0, Unit::Hz),
    ParamSpec::lin("Pump", "Fry", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Pump Time", "Fry", 0.03, 0.6, 0.18, Unit::Sec),
    ParamSpec::choice("Pump Key", "Fry", &["Kick", "808", "Kick & 808"], 0),
    ParamSpec::lin("Stutter", "Fry", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Stutter Rate", "Fry", STUTTER_RATES, 1),
    ParamSpec::toggle("Stutter Hold", "Fry", false),
    ParamSpec::lin("Tape Stop", "Fry", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Tape Time", "Fry", TAPE_TIMES, 2),
    ParamSpec::toggle("Tape Hold", "Fry", false),
    ParamSpec::lin("Channel Surf", "Fry", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Surf Hold", "Fry", false),
    ParamSpec::lin("Ceiling", "Fry", -12.0, 0.0, -0.3, Unit::Db),
    ParamSpec::lin("Volume", "Fry", -40.0, 0.0, -3.0, Unit::Db),
];

const N_PARAMS: usize = TRACKS * TP + NG;

const fn all_specs() -> [ParamSpec; N_PARAMS] {
    let mut out = [GLOBAL_SPECS[0]; N_PARAMS];
    let mut t = 0;
    while t < TRACKS {
        let mut k = 0;
        while k < TP {
            out[t * TP + k] = TRACK_SPECS[t][k];
            k += 1;
        }
        t += 1;
    }
    let mut g = 0;
    while g < NG {
        out[G + g] = GLOBAL_SPECS[g];
        g += 1;
    }
    out
}

static ALL_SPECS: [ParamSpec; N_PARAMS] = all_specs();
pub static SPECS: &[ParamSpec] = &ALL_SPECS;

// ------------------------------------------------------------ the patterns

/// One step of a track.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Step {
    pub on: bool,
    pub accent: bool,
    pub flam: bool,
    /// Glide into the next step's note without a new hit.
    pub slide: bool,
    /// An index into `ROLLS`.
    pub roll: u8,
    /// 1..127.
    pub vel: u8,
    /// The chance it plays (%).
    pub prob: u8,
    /// Semitones from the track's pitch (the 808's from its Root).
    pub note: i8,
}

impl Step {
    pub const fn hit(vel: u8) -> Self {
        Step { on: true, accent: false, flam: false, slide: false, roll: 0, vel, prob: 100, note: 0 }
    }

    /// The step in one number (as saved).
    pub fn pack(self) -> u32 {
        (self.on as u32) | (self.accent as u32) << 1 | (self.flam as u32) << 2 | (self.slide as u32) << 3 | (self.roll as u32 & 15) << 4 | (self.vel as u32 & 127) << 8 | (self.prob.min(100) as u32) << 15 | ((self.note.clamp(-64, 63) as i32 + 64) as u32 & 127) << 22
    }

    pub fn unpack(x: u32) -> Self {
        Step {
            on: x & 1 != 0,
            accent: x & 2 != 0,
            flam: x & 4 != 0,
            slide: x & 8 != 0,
            roll: ((x >> 4) & 15).min(ROLLS.len() as u32 - 1) as u8,
            vel: ((x >> 8) & 127) as u8,
            prob: (((x >> 15) & 127) as u8).min(100),
            note: (((x >> 22) & 127) as i32 - 64) as i8,
        }
    }

    pub fn velocity(&self) -> f32 {
        self.vel.clamp(1, 127) as f32 / 127.0
    }
}

/// Steps saved as a line of text: a dot for an empty step, else the step
/// as a number in hex.
mod steps_text {
    use super::{Step, MAX_STEPS};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(steps: &[Step], s: S) -> Result<S::Ok, S::Error> {
        let n = steps.iter().rposition(|x| *x != Step::default()).map_or(0, |i| i + 1);
        let text: Vec<String> = steps[..n].iter().map(|x| if *x == Step::default() { ".".to_string() } else { format!("{:x}", x.pack()) }).collect();
        s.serialize_str(&text.join(" "))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Step>, D::Error> {
        let text = String::deserialize(d)?;
        let mut v: Vec<Step> = text.split_whitespace().take(MAX_STEPS).map(|t| if t == "." { Step::default() } else { u32::from_str_radix(t, 16).map(Step::unpack).unwrap_or_default() }).collect();
        v.resize(MAX_STEPS, Step::default());
        Ok(v)
    }
}

/// A track's steps in a pattern.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lane {
    #[serde(with = "steps_text")]
    pub steps: Vec<Step>,
    /// Its own length in steps (0: the pattern's).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub length: u8,
}

fn is_zero(x: &u8) -> bool {
    *x == 0
}

impl Default for Lane {
    fn default() -> Self {
        Self { steps: vec![Step::default(); MAX_STEPS], length: 0 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pattern {
    /// 16, 32 or 64 steps.
    pub length: u8,
    pub lanes: Vec<Lane>,
}

impl Default for Pattern {
    fn default() -> Self {
        Self { length: 16, lanes: vec![Lane::default(); TRACKS] }
    }
}

impl Pattern {
    /// How many steps track `t` goes round in.
    pub fn track_len(&self, t: usize) -> usize {
        let own = self.lanes.get(t).map_or(0, |l| l.length as usize);
        if own > 0 {
            own.min(MAX_STEPS)
        } else {
            self.len()
        }
    }

    pub fn len(&self) -> usize {
        (self.length as usize).clamp(1, MAX_STEPS)
    }

    pub fn is_empty(&self) -> bool {
        self.lanes.iter().all(|l| l.steps.iter().all(|s| !s.on))
    }

    pub fn step(&self, t: usize, i: usize) -> Step {
        self.lanes.get(t).and_then(|l| l.steps.get(i)).copied().unwrap_or_default()
    }

    /// Empties track `t` (or every track).
    pub fn clear(&mut self, track: Option<usize>) {
        for (t, lane) in self.lanes.iter_mut().enumerate() {
            if track.is_none_or(|k| k == t) {
                *lane = Lane::default();
            }
        }
    }

    /// Makes up steps for track `t` (or every track) as that kind of track
    /// plays in this music: hats in eighths and sixteenths with rolls, the
    /// kick and the 808 on the strong places with slides, the snare on the
    /// third beat, the rest here and there.
    pub fn randomise(&mut self, track: Option<usize>, voices: &[usize; TRACKS], seed: u32) {
        let mut rng = Rng::new(seed.wrapping_mul(2_654_435_761).wrapping_add(17));
        let len = self.len();
        for t in 0..TRACKS {
            if track.is_some_and(|k| k != t) {
                continue;
            }
            let lane = &mut self.lanes[t];
            *lane = Lane::default();
            let chance = |rng: &mut Rng, p: f32| rng.unipolar() < p;
            for i in 0..len {
                let beat = i % 4 == 0;
                let bar = i % 16;
                let r = &mut rng;
                let on = match voices[t] {
                    0 | 1 => bar == 0 || (beat && chance(r, 0.3)) || (i % 2 == 0 && chance(r, 0.18)) || chance(r, 0.06),
                    2 | 3 => bar == 8 || (bar == 15 && chance(r, 0.25)) || chance(r, 0.03),
                    4 => i % 2 == 0 || chance(r, 0.35),
                    5 => i % 4 == 2 && chance(r, 0.4),
                    6 => i % 8 == 6 && chance(r, 0.5),
                    _ => chance(r, 0.08),
                };
                if !on {
                    continue;
                }
                let mut s = Step::hit(if beat { 110 } else { 70 + (rng.unipolar() * 40.0) as u8 });
                match voices[t] {
                    0 | 8 => {
                        const LINE: [i8; 8] = [0, 0, 0, 3, 5, 7, 10, 12];
                        s.note = LINE[(rng.unipolar() * LINE.len() as f32) as usize % LINE.len()];
                        s.slide = rng.unipolar() < 0.3;
                    }
                    4 => {
                        if rng.unipolar() < 0.22 {
                            s.roll = [1, 1, 2, 3, 4, 6][(rng.unipolar() * 6.0) as usize % 6];
                        }
                    }
                    _ => {}
                }
                if voices[t] == 9 {
                    s.note = [0, 3, 5, 7, 10, 12][(rng.unipolar() * 6.0) as usize % 6];
                }
                lane.steps[i] = s;
            }
        }
    }

    /// Velocities moved a little this way and that, as a hand plays.
    pub fn humanise(&mut self, track: Option<usize>, seed: u32) {
        let mut rng = Rng::new(seed ^ 0x51ED_2701);
        for (t, lane) in self.lanes.iter_mut().enumerate() {
            if track.is_some_and(|k| k != t) {
                continue;
            }
            for s in lane.steps.iter_mut().filter(|s| s.on) {
                s.vel = (s.vel as f32 + rng.bipolar() * 16.0).round().clamp(20.0, 127.0) as u8;
            }
        }
    }
}

/// The machine's patterns and its chain (as saved with the set).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Machine {
    pub patterns: Vec<Pattern>,
    /// The patterns Chain plays in a row.
    #[serde(default)]
    pub chain: Vec<u8>,
}

impl Default for Machine {
    fn default() -> Self {
        Self { patterns: vec![Pattern::default(); PATTERNS], chain: vec![0, 0, 1, 0] }
    }
}

/// The most patterns a chain holds.
pub const MAX_CHAIN: usize = 32;

impl Machine {
    /// Brings it to its shape: eight patterns of twelve tracks of 64 steps,
    /// lengths and chain within bounds.
    pub fn fix(&mut self) {
        self.patterns.resize(PATTERNS, Pattern::default());
        for p in &mut self.patterns {
            p.length = match p.length {
                0..=16 => 16,
                17..=32 => 32,
                _ => 64,
            };
            p.lanes.resize(TRACKS, Lane::default());
            for l in &mut p.lanes {
                l.steps.resize(MAX_STEPS, Step::default());
                l.length = l.length.min(MAX_STEPS as u8);
            }
        }
        self.chain.retain(|&c| (c as usize) < PATTERNS);
        self.chain.truncate(MAX_CHAIN);
    }

    pub fn pattern(&self, i: usize) -> Option<&Pattern> {
        self.patterns.get(i)
    }
}

// ------------------------------------------------------------ the sound

const BLOCK: usize = 128;
const MAX_PENDING: usize = 32;
const MAX_EVENTS: usize = 16;
/// The tracks' and the kit's headroom.
const TRACK_GAIN: f32 = 0.55;

/// A hit waiting for its time (a roll's, a flam's).
#[derive(Clone, Copy, Debug, Default)]
struct Pending {
    at: f64,
    vel: f32,
    note: i8,
    slide: bool,
    accent: bool,
}

struct Track {
    voice: Box<PrincessVoice>,
    bass: Princess808,
    /// The voice it was hit with (an index into `VOICES`).
    kind: usize,
    active: bool,
    vel: f32,
    accent: bool,
    choke: f32,
    choking: bool,
    filt: Svf,
    filt_mode: u8,
    ring_ph: f32,
    held: f32,
    hold_ph: f32,
    pending: [Pending; MAX_PENDING],
    n_pending: usize,
    /// The last step asked to glide into the next.
    slide: bool,
    flash: f32,
    step: i32,
}

impl Track {
    fn new() -> Self {
        Self {
            voice: Box::default(),
            bass: Princess808::default(),
            kind: 0,
            active: false,
            vel: 1.0,
            accent: false,
            choke: 1.0,
            choking: false,
            filt: Svf::new(),
            filt_mode: 0,
            ring_ph: 0.0,
            held: 0.0,
            hold_ph: 0.0,
            pending: [Pending::default(); MAX_PENDING],
            n_pending: 0,
            slide: false,
            flash: 0.0,
            step: -1,
        }
    }

    /// Puts a hit in its place in time.
    fn queue(&mut self, h: Pending) {
        if self.n_pending >= MAX_PENDING {
            return;
        }
        let mut i = self.n_pending;
        while i > 0 && self.pending[i - 1].at > h.at {
            self.pending[i] = self.pending[i - 1];
            i -= 1;
        }
        self.pending[i] = h;
        self.n_pending += 1;
    }

    fn pop(&mut self) {
        for i in 1..self.n_pending {
            self.pending[i - 1] = self.pending[i];
        }
        self.n_pending = self.n_pending.saturating_sub(1);
    }
}

/// What a track does to its voice, worked out once a block.
#[derive(Clone, Copy, Debug, Default)]
struct TrackKnobs {
    drive: f32,
    clip: f32,
    ring: f32,
    ring_inc: f32,
    crush: bool,
    steps: f32,
    rate: f32,
    gain: f32,
    pl: f32,
    pr: f32,
    room: f32,
    fry: f32,
    mute: bool,
    ducks: bool,
}

/// A time effect starting at a sample of the block.
#[derive(Clone, Copy, Debug)]
enum Fx {
    /// A slice this long repeated for so long (samples).
    Stutter(u32, u32),
    /// Slowing to a stop over so long.
    Tape(u32),
    /// Flipping channels for so long.
    Surf(u32),
}

/// The FRY section's knobs for a block.
#[derive(Clone, Copy, Debug, Default)]
struct BusKnobs {
    drive: f32,
    steps: f32,
    rate: f32,
    crush: bool,
    ring: f32,
    ring_inc: f32,
    push: f32,
    hard: bool,
    ceiling: f32,
    volume: f32,
    lim_rel: f32,
    stutter_hold: bool,
    stutter_slice: u32,
    tape_hold: bool,
    tape_len: u32,
    surf_hold: bool,
}

/// The FRY section and the kit's time effects.
struct Bus {
    sr: f32,
    rng: Rng,
    ring_ph: f32,
    crushed: [f32; 2],
    crush_ph: f32,
    /// What came in, for the stutter.
    cap: Vec<[f32; 2]>,
    cap_w: usize,
    stut_len: u32,
    stut_t: u32,
    stut_left: u32,
    stut_start: usize,
    stut_mix: f32,
    stut_held: bool,
    /// What the stutter let through, for the tape and the channels.
    vari: Vec<[f32; 2]>,
    vari_w: usize,
    tape_len: u32,
    tape_t: u32,
    tape_left: u32,
    tape_read: f64,
    tape_mix: f32,
    tape_held: bool,
    surf_left: u32,
    surf_chan: u32,
    surf_t: u32,
    surf_kind: u8,
    surf_rate: f64,
    surf_read: f64,
    surf_mix: f32,
    surf_held: bool,
    surf_on: bool,
    surf_filt: [Svf; 2],
    surf_ph: f32,
    lim: f32,
}

impl Bus {
    fn new(sr: f32) -> Self {
        Self {
            sr,
            rng: Rng::new(0xF2_1ED5),
            ring_ph: 0.0,
            crushed: [0.0; 2],
            crush_ph: 0.0,
            cap: vec![[0.0; 2]; (2.0 * sr) as usize + 2],
            cap_w: 0,
            stut_len: 0,
            stut_t: 0,
            stut_left: 0,
            stut_start: 0,
            stut_mix: 0.0,
            stut_held: false,
            vari: vec![[0.0; 2]; (3.0 * sr) as usize + 2],
            vari_w: 0,
            tape_len: 0,
            tape_t: 0,
            tape_left: 0,
            tape_read: 0.0,
            tape_mix: 0.0,
            tape_held: false,
            surf_left: 0,
            surf_chan: 0,
            surf_t: 0,
            surf_kind: 0,
            surf_rate: 1.0,
            surf_read: 0.0,
            surf_mix: 0.0,
            surf_held: false,
            surf_on: false,
            surf_filt: [Svf::new(); 2],
            surf_ph: 0.0,
            lim: 1.0,
        }
    }

    fn reset(&mut self) {
        self.stut_len = 0;
        self.stut_left = 0;
        self.stut_mix = 0.0;
        self.tape_len = 0;
        self.tape_left = 0;
        self.tape_mix = 0.0;
        self.surf_left = 0;
        self.surf_on = false;
        self.surf_mix = 0.0;
    }

    fn start(&mut self, fx: Fx) {
        match fx {
            Fx::Stutter(slice, len) => {
                if self.stut_left == 0 && !self.stut_held {
                    self.stut_len = slice.clamp(16, self.cap.len() as u32 - 2);
                    self.stut_t = 0;
                    self.stut_start = self.cap_w;
                }
                self.stut_left = self.stut_left.max(len);
            }
            Fx::Tape(len) => {
                if self.tape_left == 0 && !self.tape_held {
                    self.start_tape(len);
                }
            }
            Fx::Surf(len) => {
                if !self.surf_on {
                    self.surf_on = true;
                    self.new_channel();
                }
                self.surf_left = self.surf_left.max(len);
            }
        }
    }

    fn start_tape(&mut self, len: u32) {
        // At most so long that the reader stays in the buffer.
        self.tape_len = len.clamp(64, (self.vari.len() as u32 - 4) * 2 - 8);
        self.tape_left = self.tape_len;
        self.tape_t = 0;
        self.tape_read = self.vari_w as f64;
    }

    /// Flips to another channel.
    fn new_channel(&mut self) {
        let sr = self.sr;
        self.surf_t = 0;
        self.surf_chan = ((0.04 + 0.13 * self.rng.unipolar()) * sr) as u32;
        let kind = (self.rng.unipolar() * 6.0) as u8 % 6;
        // Not the same channel twice.
        self.surf_kind = if kind == self.surf_kind { (kind + 1) % 6 } else { kind };
        let vlen = self.vari.len() as f64;
        match self.surf_kind {
            0 => {
                let f = 900.0 + 1800.0 * self.rng.unipolar();
                self.surf_filt[0].set_q(f, 2.5, sr);
                self.surf_filt[1].set_q(f, 2.5, sr);
            }
            1 => {
                let f = 250.0 + 450.0 * self.rng.unipolar();
                self.surf_filt[0].set_q(f, 0.9, sr);
                self.surf_filt[1].set_q(f, 0.9, sr);
            }
            2 => {
                self.surf_filt[0].set_q(1800.0, 0.8, sr);
                self.surf_filt[1].set_q(1800.0, 0.8, sr);
            }
            3 => {
                // An octave or a fifth down: read more slowly than it is written.
                self.surf_rate = if self.rng.unipolar() < 0.5 { 0.5 } else { 0.667 };
                self.surf_read = self.vari_w as f64;
            }
            4 => {
                // Up: read faster, from far enough back to never catch up.
                self.surf_rate = if self.rng.unipolar() < 0.5 { 2.0 } else { 1.5 };
                let back = (self.surf_chan as f64 * (self.surf_rate - 1.0)).min(vlen - 4.0);
                self.surf_read = (self.vari_w as f64 - back).rem_euclid(vlen);
            }
            _ => {
                self.surf_filt[0].set_q(3000.0 + 3000.0 * self.rng.unipolar(), 1.2, sr);
                self.surf_filt[1].set_q(3000.0, 1.2, sr);
            }
        }
    }

    #[inline]
    fn read(buf: &[[f32; 2]], at: f64) -> [f32; 2] {
        let n = buf.len();
        let i = at.floor();
        let f = (at - i) as f32;
        let i0 = (i as i64).rem_euclid(n as i64) as usize;
        let i1 = (i0 + 1) % n;
        [buf[i0][0] + (buf[i1][0] - buf[i0][0]) * f, buf[i0][1] + (buf[i1][1] - buf[i0][1]) * f]
    }

    /// The fry chain on what is sent to it.
    #[inline]
    fn fry(&mut self, x: [f32; 2], k: &BusKnobs) -> [f32; 2] {
        let mut y = x;
        if k.ring > 0.0 {
            let m = 1.0 - k.ring + k.ring * (self.ring_ph * TAU).sin();
            self.ring_ph = (self.ring_ph + k.ring_inc).fract();
            y = [y[0] * m, y[1] * m];
        }
        if k.drive > 0.0 {
            let g = 1.0 + 14.0 * k.drive * k.drive;
            let comp = 1.0 / (1.0 + 0.6 * k.drive);
            y = [fast_tanh(y[0] * g) * comp, fast_tanh(y[1] * g) * comp];
        }
        if k.crush {
            self.crush_ph += k.rate;
            if self.crush_ph >= 1.0 {
                self.crush_ph -= 1.0;
                self.crushed = [(y[0] * k.steps).round() / k.steps, (y[1] * k.steps).round() / k.steps];
            }
            y = self.crushed;
        }
        y
    }

    /// The time effects, then the clip into the ceiling.
    #[inline]
    fn tick(&mut self, x: [f32; 2], k: &BusKnobs) -> [f32; 2] {
        let sr = self.sr;
        let smooth = 1.0 - (-1.0 / (0.002 * sr)).exp();
        // The stutter: the slice as it came in, then again and again.
        let clen = self.cap.len();
        self.cap[self.cap_w] = x;
        let mut y = x;
        if self.stut_len > 0 {
            let on = self.stut_left > 0 || self.stut_held;
            let len = self.stut_len;
            let ph = self.stut_t % len;
            let s = self.cap[(self.stut_start + ph as usize) % clen];
            // Short fades where it jumps back.
            let fade = (0.002 * sr).max(1.0);
            let mut g = ((len - ph) as f32 / fade).min(1.0);
            if self.stut_t >= len {
                g = g.min((ph as f32 + 1.0) / fade);
            }
            self.stut_mix += ((on as u8 as f32) - self.stut_mix) * smooth;
            y = [x[0] + (s[0] * g - x[0]) * self.stut_mix, x[1] + (s[1] * g - x[1]) * self.stut_mix];
            self.stut_t = self.stut_t.wrapping_add(1);
            self.stut_left = self.stut_left.saturating_sub(1);
            if !on && self.stut_mix < 1e-3 {
                self.stut_len = 0;
            }
        }
        self.cap_w = (self.cap_w + 1) % clen;
        // Tape and channels read what the stutter let through.
        let vlen = self.vari.len();
        self.vari[self.vari_w] = y;
        if self.tape_len > 0 {
            let on = self.tape_left > 0 || self.tape_held;
            let rate = (1.0 - self.tape_t as f64 / self.tape_len as f64).max(0.0);
            let s = Self::read(&self.vari, self.tape_read);
            let g = (rate as f32).sqrt();
            self.tape_read = (self.tape_read + rate).rem_euclid(vlen as f64);
            self.tape_mix += ((on as u8 as f32) - self.tape_mix) * smooth;
            y = [y[0] + (s[0] * g - y[0]) * self.tape_mix, y[1] + (s[1] * g - y[1]) * self.tape_mix];
            self.tape_t = (self.tape_t + 1).min(self.tape_len);
            self.tape_left = self.tape_left.saturating_sub(1);
            if !on && self.tape_mix < 1e-3 {
                self.tape_len = 0;
            }
        } else if self.surf_on || self.surf_mix > 1e-3 {
            let n = self.rng.bipolar();
            let s = match self.surf_kind {
                // A radio: a narrow band and a little hiss.
                0 => [self.surf_filt[0].process(y[0], FilterMode::Band) * 2.2 + n * 0.05, self.surf_filt[1].process(y[1], FilterMode::Band) * 2.2 + n * 0.05],
                // Through the wall.
                1 => [self.surf_filt[0].process(y[0], FilterMode::Low) * 1.3, self.surf_filt[1].process(y[1], FilterMode::Low) * 1.3],
                // A tinny speaker, ringing.
                2 => {
                    self.surf_ph = (self.surf_ph + 900.0 / sr).fract();
                    let m = 0.5 + 0.5 * (self.surf_ph * TAU).sin();
                    let mono = (y[0] + y[1]) * 0.5;
                    let t = self.surf_filt[0].process(mono, FilterMode::High) * (0.6 + 0.8 * m) * 1.8;
                    [t, t]
                }
                // Pitched down or up.
                3 | 4 => {
                    let s = Self::read(&self.vari, self.surf_read);
                    self.surf_read = (self.surf_read + self.surf_rate).rem_euclid(vlen as f64);
                    s
                }
                // Mostly static.
                _ => {
                    let st = self.surf_filt[0].process(n, FilterMode::Band) * 0.5;
                    [st + y[0] * 0.25, st + y[1] * 0.25]
                }
            };
            // The flip itself: a burst of static.
            let burst_len = 0.012 * sr;
            let burst = if (self.surf_t as f32) < burst_len { n * 0.3 * (1.0 - self.surf_t as f32 / burst_len) } else { 0.0 };
            let target = if self.surf_on { 1.0 } else { 0.0 };
            self.surf_mix += (target - self.surf_mix) * smooth * 2.0;
            let m = self.surf_mix;
            y = [y[0] + (s[0] + burst - y[0]) * m, y[1] + (s[1] + burst - y[1]) * m];
            self.surf_t = self.surf_t.saturating_add(1);
            if self.surf_on {
                self.surf_left = self.surf_left.saturating_sub(1);
                if self.surf_left == 0 && !self.surf_held {
                    self.surf_on = false;
                } else if self.surf_t >= self.surf_chan {
                    self.new_channel();
                }
            }
        }
        self.vari_w = (self.vari_w + 1) % vlen;
        // Pushed into a limiter a little over the ceiling, then clipped to it.
        let ceil = k.ceiling;
        let pre = [y[0] * k.push, y[1] * k.push];
        let peak = pre[0].abs().max(pre[1].abs());
        let thr = ceil * 1.4;
        if peak * self.lim > thr {
            self.lim = thr / peak.max(1e-9);
        } else {
            self.lim += (1.0 - self.lim) * k.lim_rel;
        }
        let z = [pre[0] * self.lim, pre[1] * self.lim];
        let out = if k.hard { [z[0].clamp(-ceil, ceil), z[1].clamp(-ceil, ceil)] } else { [ceil * fast_tanh(z[0] / ceil), ceil * fast_tanh(z[1] / ceil)] };
        let out = [out[0].clamp(-ceil, ceil) * k.volume, out[1].clamp(-ceil, ceil) * k.volume];
        if out[0].is_finite() && out[1].is_finite() {
            out
        } else {
            self.lim = 1.0;
            [0.0, 0.0]
        }
    }
}

/// Analog Princess, the machine, as it plays.
pub struct PrincessState {
    sr: f32,
    pub bpm: f32,
    pub beat: f64,
    pub playing: bool,
    /// The song's key (an 808 in Song Key keeps to it).
    pub key: Scale,
    tracks: [Track; TRACKS],
    rng: Rng,
    held: u128,
    /// The last step it played (counted from the song's start), the
    /// pattern playing, the step it began on, where the tracks' own lengths
    /// count from, and where in the chain it is.
    last_k: i64,
    pat: usize,
    pat_start: i64,
    poly_start: i64,
    chain_at: usize,
    step_now: i32,
    pump_env: f32,
    pump_level: f32,
    pump_gain: f32,
    room: Reverb,
    scratch: Box<[[f32; BLOCK]; 6]>,
    events: [(usize, Fx); MAX_EVENTS],
    n_events: usize,
    bus: Bus,
    #[cfg(test)]
    log: Vec<Hit>,
    #[cfg(test)]
    clock: u64,
}

/// A hit as it happened (for the tests).
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
struct Hit {
    track: usize,
    at: u64,
    vel: f32,
    hz: f32,
}

/// The knobs of the 808 on track `t`.
fn knobs_808(p: &[f32], t: usize) -> Knobs808 {
    let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
    Knobs808 {
        drive: get(tp(t, T_TONE)).clamp(0.0, 1.0),
        dist: Dist::from_index(get(DIST)),
        ring: get(tp(t, T_A)).clamp(0.0, 1.0),
        feed: get(tp(t, T_B)).clamp(0.0, 1.0),
        ring_semis: get(RING_TUNE).clamp(0.0, 24.0),
        ring_decay: get(RING_DECAY).clamp(0.1, 4.0),
        decay: get(tp(t, T_DECAY)).clamp(0.1, 4.0),
        glide: get(GLIDE).clamp(0.01, 1.0),
        sub: get(SUB_808).clamp(0.0, 1.0),
        click: get(CLICK).clamp(0.0, 1.0),
        pitch_amt: get(tp(t, T_PITCH)).clamp(0.0, 4.0),
        pitch_time: get(tp(t, T_PITCH_TIME)).clamp(0.25, 4.0),
    }
}

/// When step `k` (from the song's start) plays, in beats: every second
/// sixteenth `swing` (0..0.5 of a step) later.
fn step_time(k: i64, swing: f64) -> f64 {
    (k as f64 + if k.rem_euclid(2) == 1 { swing } else { 0.0 }) * 0.25
}

/// The last step that has played by beat `b`.
fn step_at(b: f64, swing: f64) -> i64 {
    let k = (b * 4.0).floor() as i64;
    if k.rem_euclid(2) == 1 && b < step_time(k, swing) {
        k - 1
    } else {
        k
    }
}

/// A roll's hits, the steps it spans and whether it ramps (1 up, -1 down).
pub fn roll_shape(roll: u8) -> (usize, f64, i8) {
    match roll {
        1 => (2, 1.0, 0),
        2 => (3, 2.0, 0),
        3 => (4, 1.0, 0),
        4 => (6, 2.0, 0),
        5 => (8, 2.0, 0),
        6 => (7, 4.0, 1),
        7 => (7, 4.0, -1),
        _ => (1, 1.0, 0),
    }
}

impl PrincessState {
    pub fn new(sr: f32) -> Self {
        Self {
            sr,
            bpm: 120.0,
            beat: 0.0,
            playing: false,
            key: Scale::new(0, ScaleKind::Minor),
            tracks: std::array::from_fn(|_| Track::new()),
            rng: Rng::new(0xA9A1_0915),
            held: 0,
            last_k: i64::MIN,
            pat: 0,
            pat_start: 0,
            poly_start: 0,
            chain_at: 0,
            step_now: -1,
            pump_env: 0.0,
            pump_level: 0.0,
            pump_gain: 1.0,
            room: Reverb::new(sr),
            scratch: Box::new([[0.0; BLOCK]; 6]),
            events: [(0, Fx::Tape(0)); MAX_EVENTS],
            n_events: 0,
            bus: Bus::new(sr),
            #[cfg(test)]
            log: Vec::new(),
            #[cfg(test)]
            clock: 0,
        }
    }

    pub fn set_transport(&mut self, bpm: f32, beat: f64, playing: bool) {
        self.bpm = bpm;
        self.beat = beat;
        self.playing = playing;
    }

    /// Plays track `t`: `note` semitones from its pitch (or, with `key`, the
    /// 808 at that MIDI note), `slide` into it if it still rings.
    #[allow(clippy::too_many_arguments)]
    fn trigger(&mut self, p: &[f32], t: usize, vel: f32, note: i8, key: Option<u8>, slide: bool, accent: bool) {
        let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
        let kind = (get(tp(t, T_VOICE)).round().max(0.0) as usize).min(VOICES.len() - 1);
        let sr = self.sr;
        let tune = get(tp(t, T_TUNE));
        let vel = vel.clamp(0.0, 1.0);
        // A hit cuts the other tracks of its choke group.
        let group = get(tp(t, T_CHOKE)).round() as i32;
        if group > 0 && !slide {
            for (o, tr) in self.tracks.iter_mut().enumerate() {
                if o != t && tr.active && get(tp(o, T_CHOKE)).round() as i32 == group {
                    tr.choking = true;
                }
            }
        }
        let tr = &mut self.tracks[t];
        let hz;
        if kind == 0 {
            let midi = match key {
                Some(k) => k as f32 + tune,
                None => {
                    let in_key = get(SCALE) >= 0.5;
                    let root = if in_key { self.key.root as i32 } else { get(ROOT).round() as i32 };
                    let mut m = (24 + root.rem_euclid(12) + note as i32).clamp(0, 127);
                    if in_key && self.key.kind != ScaleKind::Chromatic {
                        m = self.key.snap(m as u8) as i32;
                    }
                    m as f32 + tune
                }
            };
            hz = midi_to_freq(midi);
            let ringing = tr.active && tr.kind == 0 && !tr.choking;
            tr.bass.trigger(hz, vel, slide && ringing, knobs_808(p, t), sr);
            if slide && ringing {
                return;
            }
        } else {
            let Some(dv) = drum_voice(kind) else { return };
            let semis = tune + note as f32 + key.map_or(0.0, |k| k as f32 - NOTES[BASS] as f32);
            let ratio = 2f32.powf(semis / 12.0);
            // The 808 Tom: an 808 an octave up, short.
            let (key_hz, decay) = if kind == 8 { (Some(BASE_808 * 2.0), 0.35) } else { (None, 1.0) };
            let v = &mut tr.voice;
            v.bpm = self.bpm;
            v.pitch_amt = get(tp(t, T_PITCH)).clamp(0.0, 4.0);
            v.pitch_time = get(tp(t, T_PITCH_TIME)).clamp(0.25, 4.0);
            let d = get(tp(t, T_DECAY)).clamp(0.1, 4.0) * decay;
            v.trigger(dv, vel, ratio, key_hz, false, d, get(tp(t, T_TONE)), get(tp(t, T_A)), get(tp(t, T_B)), sr);
            hz = key_hz.unwrap_or(BASE_808) * ratio;
            tr.bass.stop();
        }
        tr.kind = kind;
        tr.active = true;
        tr.vel = vel;
        tr.accent = accent;
        tr.choke = 1.0;
        tr.choking = false;
        tr.flash = vel.max(0.3);
        // The key ducks the rest.
        let pump_key = get(PUMP_KEY).round() as i32;
        if (t == KICK && pump_key != 1) || (t == BASS && pump_key >= 1) {
            self.pump_env = 1.0;
        }
        #[cfg(test)]
        self.log.push(Hit { track: t, at: self.clock, vel, hz });
        let _ = hz;
    }

    pub fn note_on(&mut self, p: &[f32], pitch: u8, vel: f32) {
        let pitch = pitch & 127;
        let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
        let is_808 = get(tp(BASS, T_VOICE)).round() as i32 == 0;
        // The 808 slides while another of its keys is held.
        let range = KEYS_808.into_iter().fold(0u128, |m, n| m | 1 << n) | 1u128 << NOTES[BASS];
        let legato = is_808 && self.held & range & !(1u128 << pitch) != 0;
        self.held |= 1u128 << pitch;
        let mut played = false;
        for t in 0..TRACKS {
            if NOTES[t] == pitch {
                self.trigger(p, t, vel, 0, None, legato && t == BASS, false);
                played = true;
            }
        }
        if !played && is_808 && KEYS_808.contains(&pitch) {
            self.trigger(p, BASS, vel, 0, Some(pitch), legato, false);
        }
    }

    pub fn note_off(&mut self, pitch: u8) {
        self.held &= !(1u128 << (pitch & 127));
    }

    pub fn all_off(&mut self) {
        self.held = 0;
        for tr in self.tracks.iter_mut() {
            if tr.active {
                tr.choking = true;
            }
            tr.n_pending = 0;
            tr.slide = false;
        }
        self.bus.reset();
    }

    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.pat as f32;
        out[1] = self.step_now as f32;
        out[2] = self.chain_at as f32;
        out[3] = 1.0 - self.pump_gain;
        out[4] = 1.0 - self.bus.lim;
        out[5] = if self.bus.stut_len > 0 && (self.bus.stut_left > 0 || self.bus.stut_held) { 1.0 } else { 0.0 };
        out[6] = if self.bus.tape_len > 0 { 1.0 } else { 0.0 };
        out[7] = if self.bus.surf_on { 1.0 } else { 0.0 };
        for (t, tr) in self.tracks.iter().enumerate() {
            out[8 + t] = tr.flash;
            out[20 + t] = tr.step as f32;
        }
        out[32] = if self.playing { 1.0 } else { 0.0 };
    }

    /// Where the song's step `k` lands after a jump (or the start): the
    /// pattern then, and the step it began on.
    fn resync(&mut self, k: i64, p: &[f32], m: &Machine) {
        let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
        let len_of = |i: usize| m.pattern(i).map_or(16, |x| x.len()) as i64;
        if get(CHAIN) >= 0.5 && !m.chain.is_empty() {
            let total: i64 = m.chain.iter().map(|&c| len_of(c as usize)).sum();
            let mut at = k.rem_euclid(total.max(1));
            for (i, &c) in m.chain.iter().enumerate() {
                let l = len_of(c as usize);
                if at < l {
                    self.chain_at = i;
                    self.pat = c as usize;
                    self.pat_start = k - at;
                    break;
                }
                at -= l;
            }
        } else {
            self.pat = (get(PATTERN).round().max(0.0) as usize).min(PATTERNS - 1);
            let l = len_of(self.pat);
            self.pat_start = k - k.rem_euclid(l);
        }
        self.poly_start = self.pat_start;
        for tr in self.tracks.iter_mut() {
            tr.n_pending = 0;
            tr.slide = false;
        }
    }

    /// Plays the song's step `k` (at beat `at`).
    fn fire(&mut self, k: i64, at: f64, i: usize, p: &[f32], m: &Machine) {
        let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
        // A pattern that has played to its end: the next one starts.
        let len = m.pattern(self.pat).map_or(16, |x| x.len()) as i64;
        if k - self.pat_start >= len {
            let before = self.pat;
            if get(CHAIN) >= 0.5 && !m.chain.is_empty() {
                self.chain_at = (self.chain_at + 1) % m.chain.len();
                self.pat = (m.chain[self.chain_at] as usize).min(PATTERNS - 1);
            } else {
                self.pat = (get(PATTERN).round().max(0.0) as usize).min(PATTERNS - 1);
            }
            self.pat_start = k;
            if self.pat != before {
                self.poly_start = k;
            }
        }
        let Some(pat) = m.pattern(self.pat) else { return };
        let local = k - self.pat_start;
        self.step_now = local as i32;
        let bps = self.bpm.max(1.0) as f64 / 60.0;
        let flam = get(FLAM) as f64 / 1000.0 * bps;
        for t in 0..TRACKS {
            let tl = pat.track_len(t) as i64;
            let at_step = ((k - self.poly_start).rem_euclid(tl)) as usize;
            self.tracks[t].step = at_step as i32;
            let s = pat.step(t, at_step);
            if !s.on || get(tp(t, T_MUTE)) >= 0.5 {
                continue;
            }
            if s.prob < 100 && self.rng.unipolar() * 100.0 >= s.prob as f32 {
                continue;
            }
            let tr = &mut self.tracks[t];
            let slide = tr.slide;
            tr.slide = s.slide;
            let vel = s.velocity();
            let (hits, span, ramp) = roll_shape(s.roll);
            let mut t0 = at;
            if s.flam {
                // A grace note first, softer.
                tr.queue(Pending { at, vel: vel * 0.45, note: s.note, slide: false, accent: false });
                t0 += flam;
            }
            for j in 0..hits {
                let x = j as f64 / hits as f64;
                let pos = match ramp {
                    1 => x.powf(0.6),
                    -1 => x.powf(1.6),
                    _ => x,
                };
                let v = match ramp {
                    1 => vel * (0.4 + 0.6 * j as f32 / (hits - 1) as f32),
                    -1 => vel * (1.0 - 0.6 * j as f32 / (hits - 1) as f32),
                    _ if j > 0 => vel * 0.85,
                    _ => vel,
                };
                tr.queue(Pending { at: t0 + pos * span * 0.25, vel: v, note: s.note, slide: slide && j == 0 && !s.flam, accent: s.accent && j == 0 });
            }
        }
        // The kit's time effects, now and then.
        let bar = local.rem_euclid(16);
        let spb = 60.0 / self.bpm.max(1.0) * self.sr;
        let df = get(DEEP_FRY).clamp(0.0, 1.0);
        let stut = (get(STUTTER) + df * 0.15).clamp(0.0, 1.0);
        let tape = (get(TAPE) + df * 0.05).clamp(0.0, 1.0);
        let surf = (get(CHANNEL_SURF) + df * 0.12).clamp(0.0, 1.0);
        let tape_steps = [1, 2, 4, 8][(get(TAPE_TIME).round().max(0.0) as usize).min(3)];
        if tape > 0.0 && bar == 16 - tape_steps && self.rng.unipolar() < tape {
            self.event(i, Fx::Tape((tape_steps as f32 * 0.25 * spb) as u32));
        } else if stut > 0.0 && bar % 4 == 0 && self.rng.unipolar() < stut {
            let slice = stutter_slice(get(STUTTER_RATE), &mut self.rng) * spb;
            let steps = if self.rng.unipolar() < 0.5 { 2.0 } else { 4.0 };
            let len = (steps * 0.25 * spb).max(2.0 * slice);
            self.event(i, Fx::Stutter(slice as u32, len as u32));
        } else if surf > 0.0 && self.rng.unipolar() < surf * 0.3 {
            let steps = 1.0 + (self.rng.unipolar() * 3.0).floor();
            self.event(i, Fx::Surf((steps * 0.25 * spb) as u32));
        }
    }

    fn event(&mut self, i: usize, fx: Fx) {
        if self.n_events < MAX_EVENTS {
            self.events[self.n_events] = (i, fx);
            self.n_events += 1;
        }
    }

    pub fn render(&mut self, p: &[f32], m: &Machine, l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        let mut pos = 0;
        while pos < n {
            let len = (n - pos).min(BLOCK);
            self.render_block(p, m, &mut l[pos..pos + len], &mut r[pos..pos + len]);
            pos += len;
        }
    }

    fn render_block(&mut self, p: &[f32], m: &Machine, l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len()).min(BLOCK);
        let sr = self.sr;
        let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
        let df = get(DEEP_FRY).clamp(0.0, 1.0);
        // The tracks' knobs.
        let pump_depth = (get(PUMP) + df * 0.35 * (1.0 - get(PUMP))).clamp(0.0, 1.0);
        let pump_key = get(PUMP_KEY).round() as i32;
        let mut tk = [TrackKnobs::default(); TRACKS];
        for (t, k) in tk.iter_mut().enumerate() {
            let g = |i: usize| get(tp(t, i));
            let bits = g(T_BITS).round().clamp(2.0, 16.0);
            let rate = 1.0 / g(T_RATE).clamp(1.0, 32.0);
            let (pl, pr) = pan_gains(g(T_PAN));
            let is_key = (t == KICK && pump_key != 1) || (t == BASS && pump_key >= 1);
            *k = TrackKnobs {
                drive: g(T_DRIVE).clamp(0.0, 1.0),
                clip: g(T_CLIP).clamp(0.0, 1.0),
                ring: g(T_RING).clamp(0.0, 1.0),
                ring_inc: g(T_RING_HZ).clamp(1.0, sr * 0.45) / sr,
                crush: bits < 16.0 || rate < 0.999,
                steps: 2f32.powf(bits - 1.0),
                rate,
                gain: db_to_gain(g(T_LEVEL)) * TRACK_GAIN,
                pl,
                pr,
                room: g(T_ROOM).clamp(0.0, 1.0),
                fry: g(T_FRY).clamp(0.0, 1.0),
                mute: g(T_MUTE) >= 0.5,
                ducks: pump_depth > 0.0 && !is_key,
            };
            let f = g(T_FILTER).clamp(-1.0, 1.0);
            let tr = &mut self.tracks[t];
            let mode = if f < -0.02 {
                tr.filt.set(20000.0 * 0.0075f32.powf(-f), 0.15, sr);
                1
            } else if f > 0.02 {
                tr.filt.set(20.0 * 300f32.powf(f), 0.15, sr);
                2
            } else {
                0
            };
            if mode != tr.filt_mode {
                tr.filt.reset();
                tr.filt_mode = mode;
            }
        }
        let pump_fall = 1.0 / (get(PUMP_TIME).clamp(0.03, 0.6) * sr);
        let pump_att = 1.0 - (-1.0 / (0.001 * sr)).exp();
        let choke_coef = decay_coef(0.015, sr);
        let seq = get(SEQUENCER) >= 0.5;
        let swing = ((get(SWING) - 0.5).clamp(0.0, 0.25) * 2.0) as f64;
        let bps = self.bpm.max(1.0) as f64 / 60.0 / sr as f64;
        for buf in self.scratch.iter_mut() {
            buf[..n].fill(0.0);
        }
        self.n_events = 0;
        for i in 0..n {
            // The sequencer, on the song's beat.
            if self.playing && seq {
                let b = self.beat + i as f64 * bps;
                let k = step_at(b, swing);
                if k != self.last_k {
                    let jumped = self.last_k == i64::MIN || k != self.last_k + 1;
                    if jumped {
                        self.resync(k, p, m);
                    }
                    let at = step_time(k, swing);
                    if !jumped || b - at < 0.02 {
                        self.fire(k, at, i, p, m);
                    }
                    self.last_k = k;
                }
                for t in 0..TRACKS {
                    while self.tracks[t].n_pending > 0 && self.tracks[t].pending[0].at <= b {
                        let h = self.tracks[t].pending[0];
                        self.tracks[t].pop();
                        self.trigger(p, t, h.vel, h.note, None, h.slide, h.accent);
                    }
                }
            } else if self.last_k != i64::MIN {
                self.last_k = i64::MIN;
                self.step_now = -1;
                for tr in self.tracks.iter_mut() {
                    tr.n_pending = 0;
                    tr.slide = false;
                    tr.step = -1;
                }
            }
            // The pump.
            self.pump_level += (self.pump_env - self.pump_level) * pump_att;
            self.pump_env = (self.pump_env - pump_fall).max(0.0);
            self.pump_gain = 1.0 - pump_depth * self.pump_level;
            // The tracks.
            let rng = &mut self.rng;
            let sc = &mut *self.scratch;
            let pump_gain = self.pump_gain;
            for (t, tr) in self.tracks.iter_mut().enumerate() {
                if !tr.active {
                    continue;
                }
                let k = &tk[t];
                let mut y = if tr.kind == 0 {
                    let y = tr.bass.tick(rng);
                    if !tr.bass.is_active() {
                        tr.active = false;
                    }
                    y
                } else {
                    let dv = drum_voice(tr.kind).unwrap_or(DrumVoice::ApKick);
                    let y = tr.voice.tick(dv, rng);
                    if !tr.voice.is_active() {
                        tr.active = false;
                    }
                    y
                };
                let drive = if tr.accent { (k.drive + 0.35).min(1.0) } else { k.drive };
                if drive > 0.0 {
                    y = fast_tanh(y * (1.0 + 12.0 * drive * drive));
                }
                if k.clip > 0.0 {
                    y = (y * (1.0 + 6.0 * k.clip)).clamp(-0.9, 0.9);
                }
                if k.ring > 0.0 {
                    tr.ring_ph = (tr.ring_ph + k.ring_inc).fract();
                    y *= 1.0 - k.ring + k.ring * (tr.ring_ph * TAU).sin();
                }
                if k.crush {
                    tr.hold_ph += k.rate;
                    if tr.hold_ph >= 1.0 {
                        tr.hold_ph -= 1.0;
                        tr.held = (y * k.steps).round() / k.steps;
                    }
                    y = tr.held;
                }
                match tr.filt_mode {
                    1 => y = tr.filt.process(y, FilterMode::Low),
                    2 => y = tr.filt.process(y, FilterMode::High),
                    _ => {}
                }
                let vel_gain = (0.3 + 0.7 * tr.vel) * if tr.accent { 1.3 } else { 1.0 };
                y *= k.gain * vel_gain * tr.choke;
                if tr.choking {
                    tr.choke *= choke_coef;
                    if tr.choke < 1e-3 {
                        tr.active = false;
                        tr.bass.stop();
                    }
                }
                if k.mute || !y.is_finite() {
                    continue;
                }
                if k.ducks {
                    y *= pump_gain;
                }
                let (a, b) = (y * k.pl, y * k.pr);
                sc[4][i] += y * k.room;
                sc[2][i] += a * k.fry;
                sc[3][i] += b * k.fry;
                sc[0][i] += a * (1.0 - k.fry);
                sc[1][i] += b * (1.0 - k.fry);
            }
            #[cfg(test)]
            {
                self.clock += 1;
            }
        }
        for tr in self.tracks.iter_mut() {
            tr.flash *= (-(n as f32) / (0.12 * sr)).exp();
        }
        // The room, back into the dry side.
        let [dl, dr, fl, fr, wl, wr] = &mut *self.scratch;
        wr[..n].copy_from_slice(&wl[..n]);
        self.room.process(0.2 + 0.75 * get(ROOM_SIZE), 0.45, 0.8, 4.0, 180.0, 1.0, &mut wl[..n], &mut wr[..n]);
        // The FRY section.
        let fdrive = get(F_DRIVE).clamp(0.0, 1.0);
        let bits = (get(F_BITS).round() - df * 9.0).clamp(2.0, 16.0);
        let down = get(F_DOWN).clamp(1.0, 32.0) * (1.0 + df * 7.0);
        let ring = get(F_RING).clamp(0.0, 1.0);
        let clip = get(F_CLIP).clamp(0.0, 1.0);
        let clip = clip + df * 0.6 * (1.0 - clip);
        let spb = 60.0 / self.bpm.max(1.0) * sr;
        let bk = BusKnobs {
            drive: fdrive + df * 0.7 * (1.0 - fdrive),
            steps: 2f32.powf(bits - 1.0),
            rate: 1.0 / down,
            crush: bits < 16.0 || down > 1.001,
            ring: ring + df * 0.3 * (1.0 - ring),
            ring_inc: get(F_RING_HZ).clamp(1.0, sr * 0.45) / sr,
            push: 1.0 + 5.0 * clip,
            hard: get(F_CLIP_MODE) >= 0.5,
            ceiling: db_to_gain(get(CEILING).clamp(-12.0, 0.0)),
            volume: db_to_gain(get(VOLUME).clamp(-40.0, 0.0)),
            lim_rel: 1.0 - (-1.0 / (0.08 * sr)).exp(),
            stutter_hold: get(STUTTER_HOLD) >= 0.5,
            stutter_slice: (stutter_slice(get(STUTTER_RATE), &mut self.rng) * spb) as u32,
            tape_hold: get(TAPE_HOLD) >= 0.5,
            tape_len: ([0.25, 0.5, 1.0, 2.0][(get(TAPE_TIME).round().max(0.0) as usize).min(3)] * spb) as u32,
            surf_hold: get(SURF_HOLD) >= 0.5,
        };
        // Held buttons: on now, off when let go.
        let bus = &mut self.bus;
        if bk.stutter_hold && !bus.stut_held {
            bus.start(Fx::Stutter(bk.stutter_slice, 0));
        }
        bus.stut_held = bk.stutter_hold;
        if bk.tape_hold && !bus.tape_held {
            bus.start_tape(bk.tape_len);
        }
        bus.tape_held = bk.tape_hold;
        if bk.surf_hold && !bus.surf_held {
            bus.start(Fx::Surf(0));
        }
        bus.surf_held = bk.surf_hold;
        if !bk.surf_hold && bus.surf_left == 0 {
            bus.surf_on = false;
        }
        let mut ev = 0;
        for i in 0..n {
            while ev < self.n_events && self.events[ev].0 <= i {
                bus.start(self.events[ev].1);
                ev += 1;
            }
            let f = bus.fry([fl[i], fr[i]], &bk);
            let x = [dl[i] + wl[i] + f[0], dr[i] + wr[i] + f[1]];
            let y = bus.tick(x, &bk);
            l[i] += y[0];
            r[i] += y[1];
        }
        self.beat += n as f64 * bps;
    }
}

/// A stutter's slice (beats) at Stutter Rate `rate` (Mixed: any of them).
fn stutter_slice(rate: f32, rng: &mut Rng) -> f32 {
    let i = rate.round().max(0.0) as usize;
    let i = if i >= 3 { (rng.unipolar() * 3.0) as usize % 3 } else { i };
    [0.5, 0.25, 0.125][i]
}

// ------------------------------------------------------------ presets

pub fn preset_names() -> &'static [&'static str] {
    &["Pluggnb Bounce", "Fried 2015", "Deep Fried Meme", "Rage Synth Drums", "Jerk", "Drill Slide", "Lo-Fi Plugg", "Channel Surfing", "Subliminal Drift", "Init"]
}

/// A drum lane from text, a character a step: `.` nothing, `x` a hit, `X`
/// an accent, `o` a soft hit, `?` a hit half the time, `f` a flam, `2` `3`
/// `4` `6` `8` a roll of that many, `u` and `d` a roll ramping up or down.
/// Spaces and bars are left out.
pub fn lane(text: &str) -> Lane {
    let mut l = Lane::default();
    for (i, c) in text.chars().filter(|c| !matches!(*c, ' ' | '|' | '_')).take(MAX_STEPS).enumerate() {
        let mut s = Step::hit(100);
        match c {
            'x' => {}
            'X' => {
                s.vel = 124;
                s.accent = true;
            }
            'o' => s.vel = 58,
            '?' => s.prob = 50,
            'f' => s.flam = true,
            '2' => s.roll = 1,
            '3' => s.roll = 2,
            '4' => s.roll = 3,
            '6' => s.roll = 4,
            '8' => s.roll = 5,
            'u' => s.roll = 6,
            'd' => s.roll = 7,
            _ => continue,
        }
        if s.roll > 0 {
            s.vel = 90;
        }
        l.steps[i] = s;
    }
    l
}

/// A melodic lane from text, a word a step: `.` nothing, a number the
/// semitones of a hit, then `~` to slide into the next, `!` before it for an
/// accent, `o` after it for a soft one.
pub fn notes(text: &str) -> Lane {
    let mut l = Lane::default();
    for (i, w) in text.split_whitespace().take(MAX_STEPS).enumerate() {
        if w == "." {
            continue;
        }
        let mut s = Step::hit(104);
        let mut w = w;
        if let Some(rest) = w.strip_prefix('!') {
            s.accent = true;
            s.vel = 124;
            w = rest;
        }
        if let Some(rest) = w.strip_suffix('~') {
            s.slide = true;
            w = rest;
        }
        if let Some(rest) = w.strip_suffix('o') {
            s.vel = 64;
            w = rest;
        }
        s.note = w.parse::<i8>().unwrap_or(0);
        l.steps[i] = s;
    }
    l
}

fn pattern(length: u8, lanes: Vec<(usize, Lane)>) -> Pattern {
    let mut p = Pattern { length, ..Pattern::default() };
    for (t, l) in lanes {
        p.lanes[t] = l;
    }
    p
}

/// The same lane, every step's note moved.
fn moved(l: &Lane, semis: i8) -> Lane {
    let mut l = l.clone();
    for s in l.steps.iter_mut().filter(|s| s.on) {
        s.note = s.note.saturating_add(semis);
    }
    l
}

/// The patterns a preset makes from its A and B: C busier hats over A, D
/// B with a fill at its end, E A lifted a fourth, F a breakdown, G a build,
/// H A in double time.
fn derive(a: &Pattern, b: &Pattern) -> Vec<Pattern> {
    let mut c = a.clone();
    let hats = lane("x2x.x.3.x2x.x.6.");
    c.lanes[HAT] = hats;
    if let Some(s) = c.lanes[BASS].steps.iter_mut().filter(|s| s.on).nth(2) {
        s.note = s.note.saturating_add(12);
    }
    let mut d = b.clone();
    for i in 12..16 {
        d.lanes[SNARE].steps[i] = Step { roll: [1, 1, 2, 6][i - 12], ..Step::hit(70 + 15 * (i as u8 - 12)) };
        d.lanes[HAT].steps[i] = Step::default();
    }
    d.lanes[HAT].steps[12] = Step { roll: 6, ..Step::hit(100) };
    d.lanes[LASER].steps[15] = Step::hit(110);
    let mut e = a.clone();
    e.lanes[BASS] = moved(&a.lanes[BASS], 5);
    e.lanes[BELL] = moved(&a.lanes[BELL], 5);
    let mut f = Pattern { length: a.length, ..Pattern::default() };
    f.lanes[BASS] = notes("0~ . . . . . . . 3 . . . . . . .");
    f.lanes[HAT] = lane("o...o...o...o...");
    f.lanes[SUB] = lane("x...............");
    f.lanes[SURF] = lane("............x...");
    f.lanes[BELL] = a.lanes[BELL].clone();
    let mut g = Pattern { length: a.length, ..Pattern::default() };
    g.lanes[KICK] = lane("x...x...x...x...");
    g.lanes[BASS] = notes("0 . . . 0 . . . 0 . . . 0 . . .");
    g.lanes[SNARE] = lane("........xoxoxxXX");
    g.lanes[HAT] = lane("x.x.x.x.u...8...");
    g.lanes[LASER] = lane("...............X");
    let mut h = a.clone();
    for t in [KICK, SNARE, CLAP] {
        let src = a.lanes[t].steps.clone();
        for i in 0..16 {
            h.lanes[t].steps[i] = src[(i * 2) % 16];
        }
    }
    h.lanes[OPEN] = lane("..x...x...x...x.");
    vec![a.clone(), b.clone(), c, d, e, f, g, h]
}

/// A preset's patterns and its sound.
pub fn preset_machine(name: &str) -> Machine {
    let (a, b) = match name {
        "Pluggnb Bounce" => (
            pattern(16, vec![
                (BASS, notes("0 . . . . . 0~ . . . 7 . . 5o . .")),
                (KICK, lane("x.....x...x.....")),
                (SNARE, lane("........x.......")),
                (CLAP, lane("........o.......")),
                (HAT, lane("x.x.x.x.x.x.3.x.")),
                (RIM, lane("...o......o.....")),
                (BELL, notes("12 . . . 10 . . . 7 . . . 10 . . 3o")),
                (SUB, lane("x...............")),
            ]),
            pattern(16, vec![
                (BASS, notes("0 . . . . . 0~ . . 7~ . . 5 . . .")),
                (KICK, lane("x.....x..x..x...")),
                (SNARE, lane("........x.......")),
                (CLAP, lane("........x......o")),
                (HAT, lane("x.x.x2x.x.x.x.u.")),
                (OPEN, lane("..............x.")),
                (RIM, lane("...o.......o..o.")),
                (BELL, notes("12 . . . 10 . . . 7 . . . 3 . . .")),
                (SURF, lane("............x...")),
            ]),
        ),
        "Fried 2015" => (
            pattern(16, vec![
                (BASS, notes("!0 . . . . . . 0 . . 0~ . 3 . . .")),
                (KICK, lane("X......x..x.....")),
                (SNARE, lane("........X.......")),
                (CLAP, lane("........x.......")),
                (HAT, lane("xxxxxxxxxx2xxxxx")),
                (OPEN, lane("......x.........")),
                (PERC, lane("...........x..x.")),
            ]),
            pattern(16, vec![
                (BASS, notes("!0 . . . . . . 0~ . . 5 . . 3~ . 0")),
                (KICK, lane("X......x..x...x.")),
                (SNARE, lane("........X.......")),
                (CLAP, lane("........x.....x.")),
                (HAT, lane("xxxx2xxxxxxx4xuo")),
                (OPEN, lane("......x.......x.")),
                (LASER, lane("...............x")),
            ]),
        ),
        "Deep Fried Meme" => (
            pattern(16, vec![
                (BASS, notes("!0 . . 0 . . 0~ . 12~ . 0 . . 7~ . 5")),
                (KICK, lane("X..x..x...X..x..")),
                (SNARE, lane("....X.......X...")),
                (CLAP, lane("....x.......x..x")),
                (HAT, lane("x.x.x.x.x.x.x.x.")),
                (RIM, lane("..x....x..x....x")),
                (LASER, lane("..........X.....")),
                (SURF, lane(".......x........")),
            ]),
            pattern(16, vec![
                (BASS, notes("!0 . . 0 . . 0~ . 12~ . 10~ . 7 . . .")),
                (KICK, lane("X..x..x...X..xxx")),
                (SNARE, lane("....X.......X.44")),
                (CLAP, lane("....x.......x...")),
                (HAT, lane("8.x.x.x.6.x.x.u.")),
                (LASER, lane("X.......X.......")),
                (PERC, lane("...x...x...x...x")),
                (SUB, lane("x...............")),
            ]),
        ),
        "Rage Synth Drums" => (
            pattern(16, vec![
                (BASS, notes("!0 . . 0 . . 0 . !0 . . 0 . . 3~ 5")),
                (KICK, lane("X..x..x.X..x..x.")),
                (SNARE, lane("....X.......X...")),
                (CLAP, lane("....X.......X...")),
                (HAT, lane("xxxxxxxxxxxxxx8.")),
                (OPEN, lane("..x...x...x...x.")),
                (LASER, lane("...............X")),
            ]),
            pattern(16, vec![
                (BASS, notes("!0 . . 0 . . 0 . !0 . 3~ . 5 . 7~ 0")),
                (KICK, lane("X..x..x.X..x.xxx")),
                (SNARE, lane("....X.......X..x")),
                (CLAP, lane("....X.......X...")),
                (HAT, lane("x8xxxxx8xxxxu...")),
                (OPEN, lane("..x...x...x.....")),
                (PERC, lane("x...x...x...x...")),
            ]),
        ),
        "Jerk" => (
            pattern(16, vec![
                (BASS, notes("0 . . 0 . . 7 . . . 0 . . 5 . .")),
                (KICK, lane("x..x..x...x..x..")),
                (CLAP, lane("....X.......X...")),
                (HAT, lane("..x...x...x...x.")),
                (RIM, lane(".x...x.x.x...x..")),
                (LASER, lane("..............x.")),
            ]),
            pattern(16, vec![
                (BASS, notes("0 . . 0 . . 7 . . . 0~ . 3 . 5 .")),
                (KICK, lane("x..x..x...x..x.x")),
                (CLAP, lane("....X.......X...")),
                (SNARE, lane("...............x")),
                (HAT, lane("..x...x.2.x...x.")),
                (RIM, lane(".x...x.x.x.x.x..")),
                (BELL, notes(". . 12 . . . . . . . 7 . . . . .")),
            ]),
        ),
        "Drill Slide" => (
            pattern(16, vec![
                (BASS, notes("0~ . . -2~ . . 3 . . . . . 5~ . . 3")),
                (KICK, lane("x.........x.....")),
                (SNARE, lane("....X.......X...")),
                (HAT, lane("x..x3.x..x..6.x.")),
                (RIM, lane("..........x..x..")),
                (PERC, lane(".......x........")),
            ]),
            pattern(16, vec![
                (BASS, notes("0~ . . -2~ . . 3~ . . 7~ . . 5~ . . 3")),
                (KICK, lane("x......x..x.....")),
                (SNARE, lane("....X.......X..x")),
                (HAT, lane("x..x3.x..6..x.d.")),
                (OPEN, lane("..............x.")),
                (RIM, lane(".......x..x..x..")),
            ]),
        ),
        "Lo-Fi Plugg" => (
            pattern(16, vec![
                (BASS, notes("0 . . . . . . . 0~ . . 5 . . . .")),
                (KICK, lane("x.......x.x.....")),
                (SNARE, lane("........x.......")),
                (HAT, lane("x.o.x.o.x.o.x.3.")),
                (RIM, lane("....o.......o...")),
                (BELL, notes("7 . . 3 . . 5 . . . 3 . 0 . . .")),
            ]),
            pattern(16, vec![
                (BASS, notes("0 . . . . . . . -2~ . . 3 . . . .")),
                (KICK, lane("x.......x.x...o.")),
                (SNARE, lane("........x.......")),
                (CLAP, lane("........o......o")),
                (HAT, lane("x.o.x.o.x.o.x2x.")),
                (BELL, notes("7 . . 3 . . 5 . . . 10 . 7 . . .")),
                (SUB, lane("x...............")),
            ]),
        ),
        "Channel Surfing" => (
            pattern(16, vec![
                (BASS, notes("0 . . . . . 0 . . . 3 . . . . .")),
                (KICK, lane("x.....x...x.....")),
                (SNARE, lane("........x.......")),
                (HAT, lane("x.x.x.x.x.x.x.x.")),
                (SURF, lane("..x.....x..x...x")),
                (LASER, lane(".........x......")),
                (SUB, lane("x...............")),
            ]),
            pattern(16, vec![
                (BASS, notes("0 . . . . . 0~ . . 5 . . 3 . . .")),
                (KICK, lane("x.....x...x..x..")),
                (SNARE, lane("........x.......")),
                (HAT, lane("x.x.x3x.x.x.x.u.")),
                (SURF, lane("x...x.x.....x.x.")),
                (RIM, lane("...x.......x....")),
            ]),
        ),
        "Subliminal Drift" => (
            pattern(32, vec![
                (BASS, notes("0 . . . . . . . . . . . . . . . -2~ . . . . . . . 3 . . . . . . .")),
                (KICK, lane("x...............x.........x.....")),
                (SNARE, lane("................x...............")),
                (HAT, lane("o...o...o...o...o...o...o...o.3.")),
                (SUB, lane("x...............x...............")),
                (SURF, lane("...........................x....")),
                (BELL, notes("12 . . . . . . . 7 . . . . . . . 10 . . . . . . . 3 . . . . . . .")),
            ]),
            pattern(32, vec![
                (BASS, notes("0 . . . . . . . 0~ . . . 3 . . . -2 . . . . . . . 0 . . . . . . .")),
                (KICK, lane("x.......x.......x.........x.....")),
                (SNARE, lane("................x...............")),
                (HAT, lane("o...o...o...o...o...o...o...u...")),
                (SUB, lane("x...............x...............")),
                (RIM, lane("........o...............o.......")),
                (BELL, notes("12 . . . . . . . 7 . . . . . . . 10 . . . . . . . 15 . . . . . . .")),
            ]),
        ),
        _ => return Machine { patterns: vec![Pattern::default(); PATTERNS], chain: vec![0, 0, 1, 0] },
    };
    let chain = match name {
        "Subliminal Drift" => vec![0, 1, 5, 0],
        "Deep Fried Meme" => vec![0, 7, 1, 3],
        _ => vec![0, 0, 1, 0, 2, 2, 1, 3],
    };
    Machine { patterns: derive(&a, &b), chain }
}

/// A preset's knobs.
pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut set = |t: usize, k: usize, v: f32| p[tp(t, k)] = v;
    match name {
        "Pluggnb Bounce" => {
            set(BASS, T_A, 0.35);
            set(BASS, T_DECAY, 1.2);
            set(BASS, T_TONE, 0.45);
            set(KICK, T_B, 0.35);
            set(HAT, T_B, 0.45);
            set(BELL, T_ROOM, 0.35);
            set(SNARE, T_ROOM, 0.2);
        }
        "Fried 2015" => {
            set(BASS, T_TONE, 1.0);
            set(BASS, T_A, 0.1);
            set(KICK, T_B, 1.0);
            set(KICK, T_CLIP, 0.6);
            set(SNARE, T_CLIP, 0.5);
            set(CLAP, T_B, 1.0);
            set(HAT, T_B, 1.0);
            set(PERC, T_TUNE, 5.0);
        }
        "Deep Fried Meme" => {
            set(BASS, T_TONE, 0.9);
            set(BASS, T_A, 0.6);
            set(BASS, T_B, 0.7);
            set(KICK, T_B, 0.9);
            set(KICK, T_DRIVE, 0.6);
            set(SNARE, T_DRIVE, 0.5);
            set(HAT, T_A, 0.9);
            set(HAT, T_B, 0.9);
            set(RIM, T_A, 0.8);
            set(LASER, T_LEVEL, -6.0);
            set(SURF, T_LEVEL, -8.0);
        }
        "Rage Synth Drums" => {
            set(BASS, T_TONE, 0.85);
            set(BASS, T_B, 0.3);
            set(BASS, T_DECAY, 0.7);
            set(KICK, T_B, 0.7);
            set(KICK, T_PITCH, 2.0);
            set(SNARE, T_VOICE, 3.0);
            set(SNARE, T_DRIVE, 0.4);
            set(HAT, T_RING, 0.3);
            set(HAT, T_RING_HZ, 2400.0);
            set(PERC, T_VOICE, 10.0);
            set(PERC, T_TUNE, 12.0);
        }
        "Jerk" => {
            set(BASS, T_TONE, 0.4);
            set(BASS, T_DECAY, 0.45);
            set(KICK, T_B, 0.4);
            set(CLAP, T_A, 0.6);
            set(CLAP, T_ROOM, 0.25);
            set(RIM, T_B, 0.2);
            set(HAT, T_FILTER, 0.25);
        }
        "Drill Slide" => {
            set(BASS, T_TONE, 0.6);
            set(BASS, T_DECAY, 1.8);
            set(BASS, T_A, 0.15);
            set(KICK, T_B, 0.3);
            set(SNARE, T_A, 0.2);
            set(SNARE, T_ROOM, 0.25);
            set(HAT, T_B, 0.3);
            set(PERC, T_VOICE, 9.0);
        }
        "Lo-Fi Plugg" => {
            set(BASS, T_TONE, 0.25);
            set(BASS, T_A, 0.15);
            set(BASS, T_DECAY, 1.4);
            set(HAT, T_FILTER, -0.35);
            set(HAT, T_BITS, 9.0);
            set(SNARE, T_FILTER, -0.25);
            set(SNARE, T_ROOM, 0.35);
            set(BELL, T_ROOM, 0.5);
            set(BELL, T_RATE, 3.0);
        }
        "Channel Surfing" => {
            set(BASS, T_TONE, 0.55);
            set(SURF, T_LEVEL, -7.0);
            set(SURF, T_A, 0.85);
            set(LASER, T_B, 0.8);
            set(HAT, T_DRIVE, 0.3);
        }
        "Subliminal Drift" => {
            set(BASS, T_TONE, 0.3);
            set(BASS, T_A, 0.6);
            set(BASS, T_DECAY, 2.6);
            set(SUB, T_LEVEL, -5.0);
            set(SUB, T_ROOM, 0.5);
            set(BELL, T_ROOM, 0.6);
            set(BELL, T_B, 0.85);
            set(SNARE, T_ROOM, 0.5);
            set(HAT, T_FILTER, -0.3);
        }
        _ => {}
    }
    let mut g = |i: usize, v: f32| p[i] = v;
    match name {
        "Pluggnb Bounce" => {
            g(SWING, 0.54);
            g(DEEP_FRY, 0.15);
            g(F_CLIP, 0.35);
            g(PUMP, 0.2);
        }
        "Fried 2015" => {
            g(DIST, 1.0);
            g(F_CLIP, 0.85);
            g(F_DRIVE, 0.35);
            g(F_BITS, 12.0);
            g(DEEP_FRY, 0.3);
        }
        "Deep Fried Meme" => {
            g(DIST, 2.0);
            g(DEEP_FRY, 0.85);
            g(F_RING, 0.35);
            g(F_BITS, 7.0);
            g(F_DOWN, 4.0);
            g(PUMP, 0.5);
            g(STUTTER, 0.3);
            g(CHANNEL_SURF, 0.15);
            g(CEILING, -1.0);
        }
        "Rage Synth Drums" => {
            g(DIST, 0.0);
            g(F_DRIVE, 0.5);
            g(F_CLIP, 0.6);
            g(STUTTER, 0.2);
            g(STUTTER_RATE, 2.0);
            g(PUMP, 0.3);
        }
        "Jerk" => {
            g(SWING, 0.6);
            g(F_CLIP, 0.4);
            g(GLIDE, 0.08);
        }
        "Drill Slide" => {
            g(GLIDE, 0.22);
            g(DIST, 3.0);
            g(SWING, 0.52);
            g(F_CLIP, 0.3);
        }
        "Lo-Fi Plugg" => {
            g(SWING, 0.6);
            g(DIST, 0.0);
            g(F_BITS, 10.0);
            g(F_DOWN, 2.5);
            g(ROOM_SIZE, 0.6);
            g(TAPE, 0.12);
            g(SUB_808, 0.3);
        }
        "Channel Surfing" => {
            g(CHANNEL_SURF, 0.45);
            g(TAPE, 0.2);
            g(STUTTER, 0.12);
            g(STUTTER_RATE, 3.0);
            g(F_CLIP, 0.4);
        }
        "Subliminal Drift" => {
            g(PUMP, 0.45);
            g(PUMP_KEY, 1.0);
            g(PUMP_TIME, 0.45);
            g(ROOM_SIZE, 0.85);
            g(TAPE, 0.15);
            g(TAPE_TIME, 3.0);
            g(DIST, 0.0);
            g(SUB_808, 0.4);
        }
        _ => {}
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn machine_with(lanes: Vec<(usize, Lane)>) -> Machine {
        let mut m = Machine::default();
        m.patterns[0] = pattern(16, lanes);
        m
    }

    /// Plays `secs` from beat 0 at 120 BPM.
    fn run(s: &mut PrincessState, p: &[f32], m: &Machine, secs: f32) -> (Vec<f32>, Vec<f32>) {
        s.set_transport(120.0, 0.0, true);
        let n = (secs * SR) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        // In engine-sized pieces, the beat set at each as the engine does.
        let mut pos = 0;
        while pos < n {
            let len = (n - pos).min(256);
            s.beat = pos as f64 / SR as f64 * 2.0;
            s.render(p, m, &mut l[pos..pos + len], &mut r[pos..pos + len]);
            pos += len;
        }
        (l, r)
    }

    fn hits(s: &PrincessState, t: usize) -> Vec<u64> {
        s.log.iter().filter(|h| h.track == t).map(|h| h.at).collect()
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0f32, |m, v| m.max(v.abs()))
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    #[test]
    fn every_parameter_has_its_place() {
        assert_eq!(SPECS.len(), N_PARAMS);
        assert_eq!(SPECS[tp(HAT, T_CHOKE)].name, "Hat Choke");
        assert_eq!(SPECS[tp(SUB, T_MUTE)].name, "Subliminal Mute");
        assert_eq!(SPECS[PATTERN].name, "Pattern");
        assert_eq!(SPECS[VOLUME].name, "Volume");
        assert_eq!(SPECS[CHANNEL_SURF].name, "Channel Surf");
        let mut names: Vec<&str> = SPECS.iter().map(|s| s.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), SPECS.len(), "every name once");
        for (t, name) in TRACK_NAMES.iter().enumerate() {
            assert_eq!(SPECS[tp(t, T_VOICE)].group, *name);
        }
    }

    #[test]
    fn steps_play_on_the_beat_and_swing_moves_every_second_one() {
        let m = machine_with(vec![(KICK, lane("xxxxxxxxxxxxxxxx"))]);
        for swing in [0.5f32, 0.75, 0.6] {
            let mut p = preset("Init");
            p[SWING] = swing;
            let mut s = PrincessState::new(SR);
            run(&mut s, &p, &m, 2.1);
            let at = hits(&s, KICK);
            assert_eq!(at.len(), 17, "a bar and the next downbeat at swing {swing}");
            let off = (swing as f64 - 0.5) * 2.0;
            for (k, &a) in at.iter().enumerate() {
                // A sixteenth at 120 BPM is 125 ms: 6000 samples.
                let want = (k as f64 + if k % 2 == 1 { off } else { 0.0 }) * 6000.0;
                assert!((a as f64 - want).abs() <= 1.0, "step {k} at {a}, not {want} (swing {swing})");
            }
        }
    }

    #[test]
    fn rolls_play_their_hits() {
        for (roll, want) in [(1u8, 2usize), (2, 3), (3, 4), (4, 6), (5, 8), (6, 7), (7, 7)] {
            let mut l = Lane::default();
            l.steps[0] = Step { roll, ..Step::hit(100) };
            let m = machine_with(vec![(HAT, l)]);
            let mut s = PrincessState::new(SR);
            run(&mut s, &preset("Init"), &m, 1.9);
            let at = hits(&s, HAT);
            assert_eq!(at.len(), want, "roll {}", ROLLS[roll as usize]);
            let (_, span, ramp) = roll_shape(roll);
            assert!((*at.last().unwrap() as f64) < span * 6000.0, "inside its {span} steps");
            let gaps: Vec<i64> = at.windows(2).map(|w| w[1] as i64 - w[0] as i64).collect();
            match ramp {
                1 => assert!(gaps.windows(2).all(|g| g[1] < g[0]), "ramping up: {gaps:?}"),
                -1 => assert!(gaps.windows(2).all(|g| g[1] > g[0]), "ramping down: {gaps:?}"),
                _ => assert!(gaps.iter().all(|g| (g - gaps[0]).abs() <= 1), "even: {gaps:?}"),
            }
        }
        // A flam: a soft hit, the hit a little after.
        let mut l = Lane::default();
        l.steps[0] = Step { flam: true, ..Step::hit(100) };
        let mut s = PrincessState::new(SR);
        run(&mut s, &preset("Init"), &machine_with(vec![(SNARE, l)]), 0.5);
        let log: Vec<Hit> = s.log.iter().copied().filter(|h| h.track == SNARE).collect();
        assert_eq!(log.len(), 2);
        assert!(log[0].vel < log[1].vel * 0.6);
        assert!(((log[1].at - log[0].at) as f32 - 0.022 * SR).abs() < 2.0);
    }

    #[test]
    fn an_808_slide_glides_to_the_next_steps_note() {
        let m = machine_with(vec![(BASS, notes("0~ . . . 7 . . . 3 . . . . . . ."))]);
        let mut p = preset("Init");
        p[SCALE] = 0.0;
        p[GLIDE] = 0.1;
        p[tp(BASS, T_DECAY)] = 3.0;
        let mut s = PrincessState::new(SR);
        // To step 4 (0.5 s), and the glide halfway.
        run(&mut s, &p, &m, 0.55);
        let (from, to) = (midi_to_freq(24.0), midi_to_freq(31.0));
        let mid = s.tracks[BASS].bass.hz();
        assert!(mid > from * 1.1 && mid < to * 0.95, "on its way: {mid} Hz from {from} to {to}");
        // One hit only: step 4 slid in.
        assert_eq!(hits(&s, BASS).len(), 1);
        let mut s = PrincessState::new(SR);
        run(&mut s, &p, &m, 0.7);
        assert!((s.tracks[BASS].bass.hz() / to - 1.0).abs() < 1e-3, "there: {}", s.tracks[BASS].bass.hz());
        // Step 8 has no slide before it: a new hit, at once on its note.
        let mut s = PrincessState::new(SR);
        run(&mut s, &p, &m, 1.01);
        assert_eq!(hits(&s, BASS).len(), 2);
        assert!((s.tracks[BASS].bass.hz() / midi_to_freq(27.0) - 1.0).abs() < 1e-3);
        // In the song's key, a note out of it is moved into it.
        let mut p = p.clone();
        p[SCALE] = 1.0;
        let mut s = PrincessState::new(SR);
        s.key = Scale::new(2, ScaleKind::Minor);
        run(&mut s, &p, &machine_with(vec![(BASS, notes("4 . . ."))]), 0.1);
        let hz = s.log[0].hz;
        let midi = 69.0 + 12.0 * (hz / 440.0).log2();
        assert!((midi - 31.0).abs() < 0.01 || (midi - 29.0).abs() < 0.01, "D minor has no F#: {midi}");
    }

    #[test]
    fn a_chain_switches_patterns_on_the_bar() {
        let mut m = Machine::default();
        m.patterns[0] = pattern(16, vec![(KICK, lane("x..............."))]);
        m.patterns[1] = pattern(16, vec![(SNARE, lane("x..............."))]);
        m.chain = vec![0, 1, 1];
        let mut p = preset("Init");
        p[CHAIN] = 1.0;
        let mut s = PrincessState::new(SR);
        run(&mut s, &p, &m, 8.1);
        // A bar is two seconds: A, B, B, then A again.
        assert_eq!(hits(&s, KICK), vec![0, 288_000]);
        assert_eq!(hits(&s, SNARE), vec![96_000, 192_000, 384_000]);
        // Without Chain, a pattern picked plays from the next bar.
        let mut p = preset("Init");
        let mut s = PrincessState::new(SR);
        run(&mut s, &p, &m, 1.0);
        p[PATTERN] = 1.0;
        let mut l = vec![0.0; 96_000];
        let mut r = l.clone();
        s.render(&p, &m, &mut l, &mut r);
        assert_eq!(hits(&s, KICK), vec![0]);
        let at = hits(&s, SNARE);
        assert!(at.len() == 1 && at[0].abs_diff(96_000) <= 1, "{at:?}");
        // A track of its own length goes round on its own.
        let mut m = Machine::default();
        m.patterns[0] = pattern(16, vec![(HAT, lane("x.."))]);
        m.patterns[0].lanes[HAT].length = 3;
        let mut s = PrincessState::new(SR);
        run(&mut s, &preset("Init"), &m, 3.99);
        let at: Vec<u64> = hits(&s, HAT).iter().map(|a| a / 6000).collect();
        assert_eq!(at, (0..11).map(|k| k * 3).collect::<Vec<u64>>());
    }

    #[test]
    fn the_fry_section_clips_and_crushes_and_the_ceiling_holds() {
        let sr = SR;
        let mut bus = Bus::new(sr);
        let mut k = BusKnobs { push: 1.0, hard: true, ceiling: 0.5, volume: 1.0, lim_rel: 0.001, steps: 2f32.powf(3.0), rate: 0.25, crush: true, drive: 0.8, ..BusKnobs::default() };
        let sine: Vec<f32> = (0..4800).map(|i| 0.6 * (TAU * 110.0 * i as f32 / sr).sin()).collect();
        let out: Vec<f32> = sine.iter().map(|&x| bus.fry([x, x], &k)[0]).collect();
        let mut levels: Vec<i32> = out.iter().map(|v| (v * 1000.0).round() as i32).collect();
        levels.sort();
        levels.dedup();
        assert!(levels.len() <= 17, "four bits: {} levels", levels.len());
        // Held for four samples at a quarter of the rate.
        assert!(out.windows(4).filter(|w| w.iter().all(|v| *v == w[0])).count() > 1000);
        // Driven: squarer than the sine.
        assert!(peak(&out) / rms(&out) < 1.2, "crest {}", peak(&out) / rms(&out));
        // However hard it is pushed, never over the ceiling.
        for hard in [true, false] {
            k.hard = hard;
            k.push = 6.0;
            let y: Vec<f32> = (0..9600).map(|i| bus.tick([4.0 * (i as f32 * 0.03).sin(), -3.0], &k)[0]).collect();
            assert!(peak(&y) <= 0.5 + 1e-6, "{}", peak(&y));
            assert!(rms(&y) > 0.3, "loud: {}", rms(&y));
        }
    }

    #[test]
    fn a_whole_kit_at_full_fry_stays_under_the_ceiling() {
        for name in preset_names() {
            let mut p = preset(name);
            p[DEEP_FRY] = 1.0;
            p[CEILING] = -6.0;
            p[VOLUME] = 0.0;
            for t in 0..TRACKS {
                p[tp(t, T_LEVEL)] = 6.0;
            }
            let m = preset_machine(name);
            let mut s = PrincessState::new(SR);
            let (l, r) = run(&mut s, &p, &m, 4.0);
            assert!(l.iter().chain(&r).all(|v| v.is_finite()), "{name}");
            assert!(peak(&l).max(peak(&r)) <= db_to_gain(-6.0) + 1e-5, "{name}: {}", peak(&l));
        }
    }

    #[test]
    fn the_pump_ducks_after_the_kick() {
        let m = machine_with(vec![(BASS, notes("0 . . . . . . . . . . . . . . .")), (KICK, lane("....x..........."))]);
        let measure = |pump: f32| {
            let mut p = preset("Init");
            p[PUMP] = pump;
            p[PUMP_TIME] = 0.2;
            p[tp(BASS, T_DECAY)] = 4.0;
            p[tp(BASS, T_A)] = 0.0;
            p[tp(KICK, T_LEVEL)] = -40.0;
            p[F_CLIP] = 0.0;
            let mut s = PrincessState::new(SR);
            let (l, _) = run(&mut s, &p, &m, 0.8);
            // Before the kick (at 0.5 s) and just after it.
            (rms(&l[(0.4 * SR) as usize..(0.49 * SR) as usize]), rms(&l[(0.52 * SR) as usize..(0.58 * SR) as usize]))
        };
        let (before, after) = measure(1.0);
        assert!(after < 0.35 * before, "ducked: {after} after, {before} before");
        let (before, after) = measure(0.0);
        assert!(after > 0.6 * before, "without the pump it rings on: {after}, {before}");
    }

    #[test]
    fn the_stutter_repeats_a_slice() {
        // Hats in sixteenths, the stutter held: an eighth (0.25 s) again and again.
        let m = machine_with(vec![(HAT, lane("xoxoxoxoxoxoxoxo")), (KICK, lane("x..x..x...x..x.."))]);
        let mut p = preset("Init");
        p[STUTTER_HOLD] = 1.0;
        p[STUTTER_RATE] = 0.0;
        p[F_CLIP] = 0.0;
        let mut s = PrincessState::new(SR);
        let (l, _) = run(&mut s, &p, &m, 1.5);
        let slice = (0.25 * SR) as usize;
        let a = &l[slice + 500..2 * slice - 500];
        let b = &l[2 * slice + 500..3 * slice - 500];
        let c = &l[3 * slice + 500..4 * slice - 500];
        let diff = |x: &[f32], y: &[f32]| rms(&x.iter().zip(y).map(|(p, q)| p - q).collect::<Vec<f32>>());
        assert!(rms(a) > 0.01);
        assert!(diff(a, b) < 0.02 * rms(a) && diff(b, c) < 0.02 * rms(a), "repeats: {} {}", diff(a, b), rms(a));
        // Without it the bar goes on (the kicks fall elsewhere).
        let mut p = p.clone();
        p[STUTTER_HOLD] = 0.0;
        let mut s = PrincessState::new(SR);
        let (l, _) = run(&mut s, &p, &m, 1.5);
        let (a, b) = (&l[slice + 500..2 * slice - 500], &l[2 * slice + 500..3 * slice - 500]);
        assert!(diff(a, b) > 0.2 * rms(a));
    }

    #[test]
    fn tape_stop_and_channel_surf_change_the_kit_and_let_go() {
        let m = machine_with(vec![(HAT, lane("xxxxxxxxxxxxxxxx")), (BASS, notes("0 . . . 0 . . . 0 . . . 0 . . ."))]);
        let mut p = preset("Init");
        p[F_CLIP] = 0.0;
        let mut s = PrincessState::new(SR);
        let (plain, _) = run(&mut s, &p, &m, 2.0);
        for hold in [TAPE_HOLD, SURF_HOLD] {
            let mut q = p.clone();
            q[hold] = 1.0;
            let mut s = PrincessState::new(SR);
            let (y, _) = run(&mut s, &q, &m, 2.0);
            assert!(y.iter().all(|v| v.is_finite()));
            let w = (1.0 * SR) as usize..(1.9 * SR) as usize;
            let d = rms(&y[w.clone()].iter().zip(&plain[w.clone()]).map(|(a, b)| a - b).collect::<Vec<f32>>());
            assert!(d > 0.3 * rms(&plain[w]), "{}: changes it", SPECS[hold].name);
        }
        // A tape stopped all the way is silent.
        let mut q = p.clone();
        q[TAPE_HOLD] = 1.0;
        q[TAPE_TIME] = 0.0;
        let mut s = PrincessState::new(SR);
        let (y, _) = run(&mut s, &q, &m, 1.0);
        assert!(rms(&y[(0.5 * SR) as usize..]) < 1e-3);
    }

    #[test]
    fn notes_play_the_tracks_and_the_808_takes_the_keys_below() {
        let p = preset("Init");
        let m = Machine::default();
        let mut s = PrincessState::new(SR);
        for (t, &n) in NOTES.iter().enumerate() {
            s.note_on(&p, n, 0.9);
            assert_eq!(s.log.last().map(|h| h.track), Some(t), "note {n}");
            s.note_off(n);
        }
        s.note_on(&p, 20, 1.0);
        let h = *s.log.last().unwrap();
        assert_eq!(h.track, BASS);
        assert!((h.hz / midi_to_freq(20.0) - 1.0).abs() < 1e-4);
        // Played while another is held: a slide, no new hit.
        let before = s.log.len();
        s.note_on(&p, 27, 1.0);
        assert_eq!(s.log.len(), before);
        let (mut l, mut r) = (vec![0.0; 4800], vec![0.0; 4800]);
        s.render(&p, &m, &mut l, &mut r);
        assert!(peak(&l) > 0.01 && l.iter().all(|v| v.is_finite()));
        // Stopped, the sequencer is quiet.
        let mut s = PrincessState::new(SR);
        let (mut l, mut r) = (vec![0.0; 48000], vec![0.0; 48000]);
        s.render(&p, &preset_machine("Pluggnb Bounce"), &mut l, &mut r);
        assert!(s.log.is_empty() && peak(&l) == 0.0);
    }

    #[test]
    fn the_presets_load_and_play() {
        assert_eq!(preset_names().len(), 10);
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            for (v, s) in p.iter().zip(SPECS) {
                assert!(*v >= s.min && *v <= s.max, "{name}: {} = {v}", s.name);
            }
            let mut m = preset_machine(name);
            let before = m.clone();
            m.fix();
            assert_eq!(m, before, "{name} is in shape");
            if *name == "Init" {
                assert!(m.patterns.iter().all(|p| p.is_empty()));
                continue;
            }
            assert!(m.patterns.iter().all(|p| !p.is_empty()), "{name}: every pattern plays");
            // Hat rolls and 808 slides in it.
            assert!(m.patterns.iter().any(|p| p.lanes[HAT].steps.iter().any(|s| s.on && s.roll > 0)), "{name} rolls");
            assert!(m.patterns.iter().any(|p| p.lanes[BASS].steps.iter().any(|s| s.on && s.slide)), "{name} slides");
            let mut s = PrincessState::new(SR);
            let (l, r) = run(&mut s, &p, &m, 4.0);
            assert!(l.iter().chain(&r).all(|v| v.is_finite()), "{name}");
            assert!(rms(&l) > 0.02, "{name} plays: {}", rms(&l));
            assert!(peak(&l).max(peak(&r)) <= db_to_gain(p[CEILING]) + 1e-5);
        }
    }

    #[test]
    fn patterns_are_saved_as_text_and_come_back() {
        let m = preset_machine("Drill Slide");
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.len() < 20_000, "{} bytes", json.len());
        let back: Machine = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
        for x in [Step::hit(100), Step { roll: 7, flam: true, slide: true, accent: true, prob: 33, note: -12, vel: 1, on: true }, Step { note: 36, ..Step::default() }] {
            assert_eq!(Step::unpack(x.pack()), x);
        }
        // An old or broken one is brought into shape.
        let mut m: Machine = serde_json::from_str(r#"{"patterns":[{"length":20,"lanes":[{"steps":"1 . zz"}]}]}"#).unwrap();
        m.fix();
        assert_eq!(m.patterns.len(), PATTERNS);
        assert_eq!(m.patterns[0].length, 32);
        assert!(m.patterns[0].lanes[0].steps[0].on);
        assert_eq!(m.patterns[0].lanes.len(), TRACKS);
    }

    #[test]
    fn randomise_and_humanise_keep_to_the_tracks() {
        let mut pat = Pattern::default();
        let voices: [usize; TRACKS] = std::array::from_fn(|t| SPECS[tp(t, T_VOICE)].default as usize);
        pat.randomise(None, &voices, 3);
        assert!(pat.lanes[KICK].steps[0].on && pat.lanes[SNARE].steps[8].on);
        assert!(pat.lanes[HAT].steps.iter().take(16).filter(|s| s.on).count() >= 8);
        assert!(pat.lanes.iter().all(|l| l.steps[16..].iter().all(|s| !s.on)), "only within its length");
        let before = pat.clone();
        pat.humanise(Some(HAT), 1);
        assert_ne!(pat.lanes[HAT], before.lanes[HAT]);
        assert_eq!(pat.lanes[KICK], before.lanes[KICK]);
        pat.clear(Some(HAT));
        assert!(pat.lanes[HAT].steps.iter().all(|s| !s.on));
    }
}
