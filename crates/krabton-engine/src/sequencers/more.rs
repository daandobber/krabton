//! More generative sequencers:
//!
//! - Orbits: planets circling at their own speeds; each pass of the top
//!   plays its note (polyrhythms that always meet again on the downbeat).
//! - Chaos: the logistic map, from steady cycles to chaos with one knob.
//! - Automaton: a one-dimensional cellular automaton (Wolfram's rules).
//! - Ant: Langton's ant walking an 8x8 board.
//! - Pong: balls bouncing in a box; every wall is a note.
//! - Markov: a melody from chances of repeating, stepping, leaping, resting.
//! - Rain: drops at random times and pitches, in storms and drizzles.
//! - Words: typed text played as a melody, or as Morse code.
//! - Lorenz: the Lorenz attractor's butterfly as a melody.

use super::{SeqState, RATES};
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

pub const SYNCS: &[&str] = &["Free", "1/32", "1/16", "1/8", "1/4"];
pub const SYNC_BEATS: [f64; 5] = [0.0, 0.125, 0.25, 0.5, 1.0];

/// Moves a beat onto the next line of a grid (`grid` 0: unchanged).
pub fn snap(beat: f64, grid: f64) -> f64 {
    if grid > 0.0 {
        (beat / grid - 1e-9).ceil() * grid
    } else {
        beat
    }
}

pub fn sync_beats(p: f32) -> f64 {
    SYNC_BEATS[(p.round().max(0.0) as usize).min(SYNC_BEATS.len() - 1)]
}

// ---------------------------------------------------------------- specs

pub const PERIODS: &[&str] = &["1 Bar", "2 Bars", "4 Bars", "8 Bars"];
const PERIOD_BEATS: [f64; 4] = [4.0, 8.0, 16.0, 32.0];

pub static ORBITS: &[ParamSpec] = &[
    ParamSpec::int("Planets", "Orbits", 1.0, 8.0, 4.0, Unit::None),
    ParamSpec::choice("Period", "Orbits", PERIODS, 0),
    ParamSpec::int("Base Note", "Orbits", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Orbits", 0.05, 2.0, 0.25, Unit::None),
    ParamSpec::lin("Velocity", "Orbits", 0.05, 1.0, 0.8, Unit::Percent),
    ParamSpec::int("Speed 1", "Speeds", 1.0, 16.0, 4.0, Unit::None),
    ParamSpec::int("Speed 2", "Speeds", 1.0, 16.0, 3.0, Unit::None),
    ParamSpec::int("Speed 3", "Speeds", 1.0, 16.0, 5.0, Unit::None),
    ParamSpec::int("Speed 4", "Speeds", 1.0, 16.0, 6.0, Unit::None),
    ParamSpec::int("Speed 5", "Speeds", 1.0, 16.0, 2.0, Unit::None),
    ParamSpec::int("Speed 6", "Speeds", 1.0, 16.0, 7.0, Unit::None),
    ParamSpec::int("Speed 7", "Speeds", 1.0, 16.0, 8.0, Unit::None),
    ParamSpec::int("Speed 8", "Speeds", 1.0, 16.0, 9.0, Unit::None),
    ParamSpec::int("Note 1", "Notes", -14.0, 14.0, 0.0, Unit::None),
    ParamSpec::int("Note 2", "Notes", -14.0, 14.0, 4.0, Unit::None),
    ParamSpec::int("Note 3", "Notes", -14.0, 14.0, 2.0, Unit::None),
    ParamSpec::int("Note 4", "Notes", -14.0, 14.0, 7.0, Unit::None),
    ParamSpec::int("Note 5", "Notes", -14.0, 14.0, -3.0, Unit::None),
    ParamSpec::int("Note 6", "Notes", -14.0, 14.0, 9.0, Unit::None),
    ParamSpec::int("Note 7", "Notes", -14.0, 14.0, 11.0, Unit::None),
    ParamSpec::int("Note 8", "Notes", -14.0, 14.0, 14.0, Unit::None),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];
pub static CHAOS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::lin("Chaos", "Map", 2.8, 4.0, 3.7, Unit::None),
    ParamSpec::int("Range", "Map", 2.0, 21.0, 10.0, Unit::None),
    ParamSpec::lin("Rest", "Map", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::int("Restart", "Map", 0.0, 64.0, 0.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.8, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];
pub static AUTOMATON: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::int("Rule", "Cells", 0.0, 255.0, 90.0, Unit::None),
    ParamSpec::int("Voices", "Cells", 1.0, 8.0, 5.0, Unit::None),
    ParamSpec::int("Spread", "Cells", 1.0, 4.0, 2.0, Unit::None),
    ParamSpec::int("Restart", "Cells", 0.0, 64.0, 32.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 48.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];
pub const ANT_PLAY: &[&str] = &["Always", "On Black", "On White"];
pub static ANT: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::int("Ants", "Board", 1.0, 3.0, 1.0, Unit::None),
    ParamSpec::choice("Play", "Board", ANT_PLAY, 1),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 48.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];
pub static PONG: &[ParamSpec] = &[
    ParamSpec::lin("Speed", "Box", 0.25, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Angle", "Box", 15.0, 75.0, 37.0, Unit::None),
    ParamSpec::int("Balls", "Box", 1.0, 2.0, 1.0, Unit::None),
    ParamSpec::choice("Sync", "Box", SYNCS, 2),
    ParamSpec::int("Top", "Walls", -14.0, 14.0, 7.0, Unit::None),
    ParamSpec::int("Right", "Walls", -14.0, 14.0, 4.0, Unit::None),
    ParamSpec::int("Bottom", "Walls", -14.0, 14.0, 0.0, Unit::None),
    ParamSpec::int("Left", "Walls", -14.0, 14.0, 2.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.05, 2.0, 0.25, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.8, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];
pub static MARKOV: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::lin("Repeat", "Chances", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::lin("Step", "Chances", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Leap", "Chances", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Rest", "Chances", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::lin("Up/Down", "Chances", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Accent", "Chances", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::int("Range", "Notes", 2.0, 21.0, 12.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];
pub static RAIN: &[ParamSpec] = &[
    ParamSpec::log("Density", "Rain", 0.1, 16.0, 2.0, Unit::None),
    ParamSpec::lin("Storm", "Rain", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::choice("Sync", "Rain", SYNCS, 0),
    ParamSpec::int("Range", "Notes", 2.0, 21.0, 14.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.05, 2.0, 0.5, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Spread", "Notes", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];
pub const WORD_MODES: &[&str] = &["Letters", "Morse"];
pub static WORDS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Mode", "Text", WORD_MODES, 0),
    ParamSpec::toggle("Accents", "Text", true),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];
pub static LORENZ: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::lin("Speed", "Attractor", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Rho", "Attractor", 20.0, 40.0, 28.0, Unit::None),
    ParamSpec::lin("Rest", "Attractor", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::int("Range", "Notes", 2.0, 21.0, 14.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 48.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

pub fn words_default() -> Vec<u8> {
    b"krabton makes music".to_vec()
}

/// Morse code of a letter or digit: dots and dashes.
fn morse(c: u8) -> &'static str {
    match c.to_ascii_lowercase() {
        b'a' => ".-",
        b'b' => "-...",
        b'c' => "-.-.",
        b'd' => "-..",
        b'e' => ".",
        b'f' => "..-.",
        b'g' => "--.",
        b'h' => "....",
        b'i' => "..",
        b'j' => ".---",
        b'k' => "-.-",
        b'l' => ".-..",
        b'm' => "--",
        b'n' => "-.",
        b'o' => "---",
        b'p' => ".--.",
        b'q' => "--.-",
        b'r' => ".-.",
        b's' => "...",
        b't' => "-",
        b'u' => "..-",
        b'v' => "...-",
        b'w' => ".--",
        b'x' => "-..-",
        b'y' => "-.--",
        b'z' => "--..",
        b'0' => "-----",
        b'1' => ".----",
        b'2' => "..---",
        b'3' => "...--",
        b'4' => "....-",
        b'5' => ".....",
        b'6' => "-....",
        b'7' => "--...",
        b'8' => "---..",
        b'9' => "----.",
        _ => "",
    }
}

/// The scale degree a character plays (None: a rest), and whether it is a vowel.
pub fn letter_degree(c: u8) -> Option<(i32, bool)> {
    let c = c.to_ascii_lowercase();
    match c {
        b'a'..=b'z' => {
            let i = (c - b'a') as i32;
            Some((i % 7 + 7 * ((i / 7) % 3), b"aeiouy".contains(&c)))
        }
        b'0'..=b'9' => Some(((c - b'0') as i32, false)),
        _ => None,
    }
}

/// What a step of Words does: a note (degree, accent) or a rest, for some steps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WordStep {
    pub note: Option<(i32, bool)>,
    pub steps: u8,
    /// The character it comes from (for the display).
    pub char_index: u16,
}

/// The steps of a text in Morse: dot 1, dash 3, gaps 1 / 3 / 7.
pub fn morse_steps(text: &[u8]) -> Vec<WordStep> {
    let mut v = Vec::new();
    for (ci, &c) in text.iter().enumerate().take(512) {
        let ci = ci as u16;
        if c == b' ' {
            v.push(WordStep { note: None, steps: 4, char_index: ci });
            continue;
        }
        let code = morse(c);
        if code.is_empty() {
            continue;
        }
        let degree = letter_degree(c);
        for (k, s) in code.bytes().enumerate() {
            if k > 0 {
                v.push(WordStep { note: None, steps: 1, char_index: ci });
            }
            v.push(WordStep { note: degree.map(|(d, a)| (d, a)), steps: if s == b'-' { 3 } else { 1 }, char_index: ci });
        }
        v.push(WordStep { note: None, steps: 3, char_index: ci });
    }
    v
}

// ---------------------------------------------------------------- state

pub struct Extra {
    pub chaos_x: f64,
    pub chaos_steps: u32,
    pub ca: u16,
    pub ca_hist: [u16; 16],
    pub ca_steps: u32,
    pub board: u64,
    pub ants: [(u8, u8, u8); 3],
    pub pong: [(f64, f64, f64, f64); 2],
    pub pong_ready: bool,
    pub melody: i32,
    pub rain_next: f64,
    pub rain_recent: u64,
    pub word_pos: usize,
    pub word_hold: u8,
    pub morse: Vec<WordStep>,
    pub morse_src: Vec<u8>,
    pub lorenz: (f64, f64, f64),
}

impl Extra {
    pub fn new() -> Self {
        Self {
            chaos_x: 0.5,
            chaos_steps: 0,
            ca: 1 << 8,
            ca_hist: [0; 16],
            ca_steps: 0,
            board: 0,
            ants: [(3, 3, 0), (5, 2, 1), (1, 6, 2)],
            pong: [(0.2, 0.3, 0.0, 0.0), (0.7, 0.6, 0.0, 0.0)],
            pong_ready: false,
            melody: 0,
            rain_next: -1.0,
            rain_recent: 0,
            word_pos: 0,
            word_hold: 0,
            morse: Vec::with_capacity(512),
            morse_src: Vec::with_capacity(512),
            lorenz: (0.1, 0.0, 20.0),
        }
    }

    pub fn reset(&mut self) {
        let morse = std::mem::take(&mut self.morse);
        let src = std::mem::take(&mut self.morse_src);
        *self = Self::new();
        self.morse = morse;
        self.morse_src = src;
    }
}

impl Default for Extra {
    fn default() -> Self {
        Self::new()
    }
}

/// The frame of song beat `beat` in this block (later blocks are fine: `play` waits).
fn frame_of(ctx: &MidiCtx, beat: f64) -> u64 {
    ctx.now + ((beat - ctx.beat).max(0.0) * ctx.samples_per_beat) as u64
}

impl SeqState {
    fn degree_pitch(ctx: &MidiCtx, base: f32, degree: i32) -> i32 {
        let (b, _) = ctx.scale.locate(base.round() as i32);
        ctx.scale.pitch_at(b + degree)
    }

    pub(crate) fn orbits(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let n = (p[0].round() as usize).clamp(1, 8);
        let period = PERIOD_BEATS[(p[1].round().max(0.0) as usize).min(3)];
        let (b0, b1) = (ctx.beat, ctx.beat + ctx.len as f64 / ctx.samples_per_beat);
        let mut vis = 0u64;
        for i in 0..n {
            let speed = p[5 + i].round().max(1.0) as f64;
            let every = period / speed;
            let mut k = (b0 / every - 1e-9).ceil();
            while k * every < b1 {
                // A pass right on a block edge is seen by both blocks: play it once.
                if k * every > self.ball_last[i] + 1e-6 {
                    self.ball_last[i] = k * every;
                    let pitch = Self::degree_pitch(ctx, p[2], p[13 + i].round() as i32);
                    self.play(ctx, frame_of(ctx, k * every), pitch, p[4], p[3] as f64 * ctx.samples_per_beat, out);
                }
                k += 1.0;
            }
            let phase = (b1 / every).fract();
            vis |= ((phase * 255.0) as u64 & 0xff) << (i * 8);
        }
        self.visual[0] = vis;
        self.visual[1] = n as u64;
    }

    pub(crate) fn chaos(&mut self, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let e = &mut self.extra;
        let restart = p[4].round() as u32;
        if restart > 0 && e.chaos_steps % restart == 0 {
            e.chaos_x = 0.5;
        }
        e.chaos_steps = e.chaos_steps.wrapping_add(1);
        let r = p[1].clamp(2.5, 4.0) as f64;
        let mut x = r * e.chaos_x * (1.0 - e.chaos_x);
        if !(1e-9..1.0 - 1e-9).contains(&x) {
            x = 0.31;
        }
        e.chaos_x = x;
        self.visual[0] = (x * 65535.0) as u64 | ((p[2].round() as u64) << 16);
        // A second look at the same value decides the rests.
        if (x * 997.0).fract() < p[3] as f64 {
            return;
        }
        let degree = (x * p[2].round() as f64).floor() as i32;
        let vel = p[7] * (0.6 + 0.4 * x as f32);
        let pitch = Self::degree_pitch(ctx, p[5], degree);
        self.note(ctx, at, pitch, vel, step_frames * p[6] as f64, out);
    }

    pub(crate) fn automaton(&mut self, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let rule = p[1].round().clamp(0.0, 255.0) as u32;
        let restart = p[4].round() as u32;
        let e = &mut self.extra;
        if (restart > 0 && e.ca_steps % restart == 0) || e.ca == 0 {
            e.ca = 1 << 8;
        } else {
            let row = e.ca as u32;
            let mut next = 0u16;
            for i in 0..16 {
                let l = (row >> ((i + 1) % 16)) & 1;
                let c = (row >> i) & 1;
                let r = (row >> ((i + 15) % 16)) & 1;
                if (rule >> (l << 2 | c << 1 | r)) & 1 == 1 {
                    next |= 1 << i;
                }
            }
            e.ca = next;
        }
        e.ca_steps = e.ca_steps.wrapping_add(1);
        e.ca_hist.rotate_left(1);
        e.ca_hist[15] = e.ca;
        for (k, v) in self.visual.iter_mut().enumerate() {
            *v = (0..4).fold(0u64, |acc, j| acc | (e.ca_hist[k * 4 + j] as u64) << (j * 16));
        }
        let voices = (p[2].round() as usize).clamp(1, 8);
        let spread = p[3].round().clamp(1.0, 4.0) as i32;
        let start = 8 - voices / 2;
        let row = e.ca;
        for j in 0..voices {
            if (row >> ((start + j) % 16)) & 1 == 1 {
                let pitch = Self::degree_pitch(ctx, p[5], j as i32 * spread);
                self.note(ctx, at, pitch, p[7], step_frames * p[6] as f64, out);
            }
        }
    }

    pub(crate) fn ant(&mut self, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let n = (p[1].round() as usize).clamp(1, 3);
        let play = p[2].round() as i32;
        let mut notes = [None; 3];
        let e = &mut self.extra;
        for (i, note) in notes.iter_mut().enumerate().take(n) {
            let (x, y, dir) = e.ants[i];
            let bit = 1u64 << (y as u64 * 8 + x as u64);
            let black = e.board & bit != 0;
            // White: turn right; black: turn left; flip the cell; step on.
            let dir = if black { (dir + 3) % 4 } else { (dir + 1) % 4 };
            e.board ^= bit;
            let (nx, ny) = match dir {
                0 => (x, (y + 7) % 8),
                1 => ((x + 1) % 8, y),
                2 => (x, (y + 1) % 8),
                _ => ((x + 7) % 8, y),
            };
            e.ants[i] = (nx, ny, dir);
            let now_black = e.board & bit != 0;
            let sound = match play {
                1 => now_black,
                2 => !now_black,
                _ => true,
            };
            if sound {
                *note = Some(x as i32 + (7 - y as i32));
            }
        }
        self.visual[0] = e.board;
        self.visual[1] = (0..n).fold(0u64, |acc, i| acc | ((e.ants[i].0 as u64) | (e.ants[i].1 as u64) << 3 | (e.ants[i].2 as u64) << 6) << (i * 8)) | (n as u64) << 32;
        for d in notes.iter().flatten() {
            let pitch = Self::degree_pitch(ctx, p[3], *d);
            self.note(ctx, at, pitch, p[5], step_frames * p[4] as f64, out);
        }
    }

    pub(crate) fn pong(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let n = (p[2].round() as usize).clamp(1, 2);
        let speed = p[0].clamp(0.05, 8.0) as f64;
        let grid = sync_beats(p[3]);
        let (b0, b1) = (ctx.beat, ctx.beat + ctx.len as f64 / ctx.samples_per_beat);
        if !self.extra.pong_ready {
            for i in 0..2 {
                let a = (p[1] as f64 + i as f64 * 23.0).to_radians();
                let (x, y, _, _) = self.extra.pong[i];
                self.extra.pong[i] = (x, y, speed * a.cos(), speed * a.sin());
            }
            self.extra.pong_ready = true;
        }
        let mut vis = 0u64;
        for i in 0..n {
            let (mut x, mut y, mut vx, mut vy) = self.extra.pong[i];
            // Keep the speed as set, and the direction.
            let norm = (vx * vx + vy * vy).sqrt().max(1e-9);
            vx *= speed / norm;
            vy *= speed / norm;
            let mut t = b0;
            for _ in 0..64 {
                let tx = if vx > 0.0 { (1.0 - x) / vx } else if vx < 0.0 { -x / vx } else { f64::MAX };
                let ty = if vy > 0.0 { (1.0 - y) / vy } else if vy < 0.0 { -y / vy } else { f64::MAX };
                let s = tx.min(ty).max(0.0);
                if t + s >= b1 {
                    x += vx * (b1 - t);
                    y += vy * (b1 - t);
                    break;
                }
                t += s;
                x = (x + vx * s).clamp(0.0, 1.0);
                y = (y + vy * s).clamp(0.0, 1.0);
                // Walls: top 4, right 5, bottom 6, left 7.
                let wall = if tx <= ty {
                    vx = -vx;
                    if x >= 0.5 {
                        5
                    } else {
                        7
                    }
                } else {
                    vy = -vy;
                    if y >= 0.5 {
                        4
                    } else {
                        6
                    }
                };
                let pitch = Self::degree_pitch(ctx, p[8], p[wall].round() as i32);
                self.play(ctx, frame_of(ctx, snap(t, grid)), pitch, p[10], p[9] as f64 * ctx.samples_per_beat, out);
            }
            self.extra.pong[i] = (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0), vx, vy);
            vis |= (((x * 255.0) as u64) | ((y * 255.0) as u64) << 8) << (i * 16);
        }
        self.visual[0] = vis;
        self.visual[1] = n as u64;
    }

    pub(crate) fn markov(&mut self, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let range = p[7].round().max(2.0) as i32;
        let weights = [p[1].max(0.0), p[2].max(0.0), p[3].max(0.0), p[4].max(0.0)];
        let total: f32 = weights.iter().sum::<f32>().max(1e-6);
        let mut roll = self.rng_unit() * total;
        let mut choice = 3;
        for (k, w) in weights.iter().enumerate() {
            if roll < *w {
                choice = k;
                break;
            }
            roll -= w;
        }
        let up = self.rng_unit() < 0.5 + 0.5 * p[5];
        let dir = if up { 1 } else { -1 };
        let mut pos = self.extra.melody;
        match choice {
            1 => pos += dir,
            2 => pos += dir * (2 + (self.rng_unit() * 4.0) as i32),
            _ => {}
        }
        if pos < 0 {
            pos = -pos;
        }
        if pos > range {
            pos = 2 * range - pos;
        }
        self.extra.melody = pos.clamp(0, range);
        self.visual[0] = self.extra.melody as u64 | (range as u64) << 8;
        if choice == 3 {
            return;
        }
        let accent = self.rng_unit() < p[6];
        let vel = if accent { (p[10] * 1.25).min(1.0) } else { p[10] * 0.8 };
        let pitch = Self::degree_pitch(ctx, p[8], self.extra.melody);
        self.note(ctx, at, pitch, vel, step_frames * p[9] as f64, out);
    }

    fn rng_unit(&mut self) -> f32 {
        (self.rng.bipolar() + 1.0) * 0.5
    }

    pub(crate) fn rain(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let (b0, b1) = (ctx.beat, ctx.beat + ctx.len as f64 / ctx.samples_per_beat);
        if self.extra.rain_next < b0 - 1.0 {
            self.extra.rain_next = b0;
        }
        let grid = sync_beats(p[2]);
        let range = p[3].round().max(2.0);
        for _ in 0..256 {
            if self.extra.rain_next >= b1 {
                break;
            }
            let t = self.extra.rain_next;
            // Storms swell and calm over four bars.
            let density = (p[0] as f64 * (1.0 + p[1] as f64 * (t * std::f64::consts::TAU / 16.0).sin())).max(0.02);
            let u = self.rng_unit().max(1e-4) as f64;
            self.extra.rain_next = t - u.ln() / density;
            let degree = (self.rng_unit() * range) as i32;
            let vel = p[6] * (1.0 - p[7] * self.rng_unit());
            let pitch = Self::degree_pitch(ctx, p[4], degree);
            self.play(ctx, frame_of(ctx, snap(t, grid)), pitch, vel, p[5] as f64 * ctx.samples_per_beat, out);
            self.extra.rain_recent = (self.extra.rain_recent << 8) | (degree as u64 & 0xff);
        }
        self.visual[0] = self.extra.rain_recent;
        self.visual[1] = range as u64;
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn words(&mut self, p: &[f32], data: &[u8], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        if data.is_empty() {
            return;
        }
        if self.extra.word_hold > 0 {
            self.extra.word_hold -= 1;
            return;
        }
        let accents = p[2] >= 0.5;
        let (note, steps, char_index) = if p[1] >= 0.5 {
            if self.extra.morse_src.as_slice() != data {
                self.extra.morse_src.clear();
                self.extra.morse_src.extend_from_slice(data);
                self.extra.morse = morse_steps(data);
                self.extra.word_pos = 0;
            }
            if self.extra.morse.is_empty() {
                return;
            }
            let s = self.extra.morse[self.extra.word_pos % self.extra.morse.len()];
            self.extra.word_pos = (self.extra.word_pos + 1) % self.extra.morse.len();
            (s.note, s.steps, s.char_index as usize)
        } else {
            let i = self.extra.word_pos % data.len();
            self.extra.word_pos = (i + 1) % data.len();
            let c = data[i];
            // Punctuation holds a little longer.
            let steps = if b".,;:!?".contains(&c) { 2 } else { 1 };
            (letter_degree(c), steps, i)
        };
        self.extra.word_hold = steps.saturating_sub(1);
        self.visual[0] = char_index as u64;
        self.visual[1] = data.len() as u64;
        if let Some((degree, vowel)) = note {
            let vel = if accents && vowel { (p[5] * 1.3).min(1.0) } else { p[5] };
            let pitch = Self::degree_pitch(ctx, p[3], degree);
            self.note(ctx, at, pitch, vel, step_frames * steps as f64 * p[4] as f64, out);
        }
    }

    pub(crate) fn lorenz(&mut self, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let (sigma, beta) = (10.0, 8.0 / 3.0);
        let rho = p[2].clamp(10.0, 60.0) as f64;
        let dt = 0.002 + p[1].clamp(0.0, 1.0) as f64 * 0.03;
        let (mut x, mut y, mut z) = self.extra.lorenz;
        for _ in 0..4 {
            let (dx, dy, dz) = (sigma * (y - x), x * (rho - z) - y, x * y - beta * z);
            x += dx * dt;
            y += dy * dt;
            z += dz * dt;
        }
        if !(x.is_finite() && y.is_finite() && z.is_finite()) {
            (x, y, z) = (0.1, 0.0, 20.0);
        }
        self.extra.lorenz = (x, y, z);
        let xi = ((x + 25.0) / 50.0 * 65535.0).clamp(0.0, 65535.0) as u64;
        let zi = (z / 60.0 * 65535.0).clamp(0.0, 65535.0) as u64;
        self.visual[0] = xi | zi << 16;
        // Rest while the butterfly is low in its wings.
        if z / 50.0 < p[3] as f64 * 0.6 {
            return;
        }
        let range = p[4].round().max(2.0) as f64;
        let degree = ((x + 20.0) / 40.0 * range).clamp(0.0, range) as i32;
        let vel = p[7] * (0.5 + 0.5 * (z / 50.0).clamp(0.0, 1.0) as f32);
        let pitch = Self::degree_pitch(ctx, p[5], degree);
        self.note(ctx, at, pitch, vel, step_frames * p[6] as f64, out);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::MidiFxKind;
    use crate::params::defaults;
    use crate::sequencers::tests::run;

    fn pitches(ons: &[(f64, u8)]) -> Vec<u8> {
        ons.iter().map(|o| o.1).collect()
    }

    #[test]
    fn every_new_sequencer_plays() {
        for (kind, specs) in [
            (MidiFxKind::Orbits, ORBITS),
            (MidiFxKind::Chaos, CHAOS),
            (MidiFxKind::Automaton, AUTOMATON),
            (MidiFxKind::Ant, ANT),
            (MidiFxKind::Pong, PONG),
            (MidiFxKind::Markov, MARKOV),
            (MidiFxKind::Rain, RAIN),
            (MidiFxKind::Words, WORDS),
            (MidiFxKind::Lorenz, LORENZ),
        ] {
            let ons = run(kind, &defaults(specs), &kind.default_data(), 16.0);
            assert!(ons.len() >= 8, "{kind:?}: {} notes", ons.len());
            assert!(ons.iter().all(|o| (24..=110).contains(&o.1)), "{kind:?}: {:?}", pitches(&ons));
            let distinct = { let mut v = pitches(&ons); v.sort(); v.dedup(); v.len() };
            assert!(distinct >= 2, "{kind:?} plays one note only");
        }
    }

    #[test]
    fn orbits_meet_on_the_bar() {
        let mut p = defaults(ORBITS);
        p[0] = 2.0;
        p[5] = 4.0;
        p[6] = 3.0;
        p[14] = 2.0;
        let ons = run(MidiFxKind::Orbits, &p, &[], 4.0);
        // Planet 1 four times a bar on C, planet 2 three times on E.
        let c: Vec<f64> = ons.iter().filter(|o| o.1 == 60).map(|o| o.0).collect();
        let e: Vec<f64> = ons.iter().filter(|o| o.1 == 64).map(|o| o.0).collect();
        assert_eq!(c.len(), 4, "{c:?}");
        assert_eq!(e.len(), 3, "{e:?}");
        assert!((e[1] - 4.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn time_based_sequencers_follow_a_loop() {
        use crate::midi_fx::MidiFxState;
        use crate::scale::{Scale, ScaleKind};
        // Two bars, then back to the start: the second pass plays the same.
        for (kind, specs) in [(MidiFxKind::Orbits, ORBITS), (MidiFxKind::Bounce, super::super::BOUNCE)] {
            let p = defaults(specs);
            let mut st = MidiFxState::new(kind);
            let spb = 24000.0;
            let mut passes = [Vec::new(), Vec::new()];
            for pass in passes.iter_mut() {
                for b in 0..(8.0 * spb / 256.0) as u64 {
                    let now = b * 256;
                    let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / spb, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 4] };
                    let mut out = Vec::new();
                    st.process(kind, &p, &[], &[], &mut out, &ctx);
                    pass.extend(out.iter().filter(|e| e.on).map(|e| (now + e.offset as u64, e.pitch)));
                }
            }
            assert!(!passes[0].is_empty());
            assert_eq!(passes[0], passes[1], "{kind:?}");
        }
    }

    #[test]
    fn chaos_settles_into_a_cycle_below_the_edge() {
        let mut p = defaults(CHAOS);
        p[1] = 3.2;
        p[3] = 0.0;
        let ons = run(MidiFxKind::Chaos, &p, &[], 16.0);
        let tail: Vec<u8> = pitches(&ons).into_iter().skip(20).collect();
        let mut d = tail.clone();
        d.sort();
        d.dedup();
        assert_eq!(d.len(), 2, "period two: {tail:?}");
        assert!(tail.windows(2).all(|w| w[0] != w[1]));
    }

    #[test]
    fn automaton_rule_90_grows_symmetric() {
        let mut p = defaults(AUTOMATON);
        p[2] = 8.0;
        p[4] = 0.0;
        let mut st = SeqState::new();
        st.extra.ca = 1 << 8;
        st.extra.ca_steps = 1;
        let ctx = MidiCtx { now: 0, len: 256, samples_per_beat: 24000.0, scale: Default::default(), scale_on: true, beat: 0.0, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 4] };
        let mut out = Vec::new();
        st.automaton(&p, 0, 6000.0, &mut out, &ctx);
        assert_eq!(st.extra.ca, (1 << 7) | (1 << 9));
        st.automaton(&p, 0, 6000.0, &mut out, &ctx);
        assert_eq!(st.extra.ca, (1 << 6) | (1 << 10));
    }

    #[test]
    fn ant_flips_cells() {
        let mut p = defaults(ANT);
        p[2] = 0.0;
        let ons = run(MidiFxKind::Ant, &p, &[], 4.0);
        assert_eq!(ons.len(), 16);
        let mut st = SeqState::new();
        let ctx = MidiCtx { now: 0, len: 256, samples_per_beat: 24000.0, scale: Default::default(), scale_on: true, beat: 0.0, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 4] };
        let mut out = Vec::new();
        for _ in 0..4 {
            st.ant(&p, 0, 6000.0, &mut out, &ctx);
        }
        // Four right turns on white: a little square, four cells black, home again.
        assert_eq!(st.extra.board.count_ones(), 4);
        assert_eq!((st.extra.ants[0].0, st.extra.ants[0].1), (3, 3));
    }

    #[test]
    fn pong_hits_walls_on_the_grid() {
        let mut p = defaults(PONG);
        p[3] = 4.0;
        let ons = run(MidiFxKind::Pong, &p, &[], 16.0);
        assert!(ons.len() >= 10, "{}", ons.len());
        assert!(ons.iter().all(|o| (o.0 - o.0.round()).abs() < 0.01), "{ons:?}");
        // All four walls get hit.
        let mut d = pitches(&ons);
        d.sort();
        d.dedup();
        assert_eq!(d.len(), 4, "{d:?}");
    }

    #[test]
    fn markov_stays_in_range() {
        let mut p = defaults(MARKOV);
        p[7] = 5.0;
        p[4] = 0.0;
        let ons = run(MidiFxKind::Markov, &p, &[], 16.0);
        // C major from C4, five steps up at most: C4..A4.
        assert!(ons.iter().all(|o| (60..=69).contains(&o.1)), "{:?}", pitches(&ons));
        assert_eq!(ons.len(), 64);
    }

    #[test]
    fn rain_density_is_drops_per_beat() {
        let mut p = defaults(RAIN);
        p[0] = 4.0;
        p[1] = 0.0;
        let ons = run(MidiFxKind::Rain, &p, &[], 32.0);
        assert!((90..=170).contains(&ons.len()), "{}", ons.len());
    }

    #[test]
    fn words_spell_text() {
        let p = defaults(WORDS);
        // Letters: a, b, rest, h -> C, D, -, C an octave up.
        let ons = run(MidiFxKind::Words, &p, b"ab h", 1.0);
        assert_eq!(pitches(&ons), vec![60, 62, 72]);
        assert!((ons[2].0 - 0.75).abs() < 0.01);
        // Morse: "e" is one dot and a gap of three: a note every fourth step.
        let mut p = p;
        p[1] = 1.0;
        let ons = run(MidiFxKind::Words, &p, b"e", 4.0);
        assert_eq!(ons.len(), 4);
        assert!((ons[1].0 - 1.0).abs() < 0.01);
        // "t" is a dash: three steps long.
        let steps = morse_steps(b"t");
        assert_eq!(steps[0].steps, 3);
        assert!(steps[0].note.is_some());
    }
}
