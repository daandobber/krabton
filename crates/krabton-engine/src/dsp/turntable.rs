//! Turntable: a DJ deck with a record of any sample on it (a song, a loop,
//! a break). The platter is turned by a motor that takes its time to get up
//! to speed and to stop (*Start*, *Stop*, the long *Brake*), set by the pitch
//! fader (±8, ±16 or ±50 %) or, with *Sync*, by the song's tempo over the
//! record's (found when it is put on). The record sits on a slipmat: held by
//! the hand it stops or goes where the hand takes it, forwards and back,
//! while the platter turns on under it; let go, the mat pulls it back to the
//! platter's speed. The hand is a stiff spring on the record, so the
//! record's speed follows the hand smoothly however unevenly the hand's
//! places come in, and the sound is read at exactly that speed through a
//! band-limited (windowed sinc) interpolator: pitch with the speed, no
//! clicks, no aliasing when it goes fast. *Key Lock* keeps the pitch while
//! the motor turns it at another speed (a simple two-grain stretch, as the
//! Tones mode of warping), and lets go of it while the hand is on.
//!
//! Notes play it: C1 to G1 the eight hot cues (a jump, exactly to the
//! frame, softened over 2 ms), G#1 start and stop, A1 the brake, A#1 a
//! backspin, B1 a loop roll of *Loop* beats (the record goes on underneath
//! and comes back where it would have been), and C2 to F2 six scratches
//! performed in the song's time while held (the platter and the cut fader
//! moved together): baby, forward, chirp, transformer, tear and flare;
//! F#2 cuts the sound while held. After the record a cut fader (with its
//! curve), a DJ filter (low-pass to the left, high-pass to the right), the
//! record's crackle and hiss, and the wow and flutter of the platter.

use super::{db_to_gain, DcBlock, FilterMode, Rng, Svf};
use crate::params::{ParamSpec, Unit};
use crate::sample::{SampleData, SampleRef};
use serde::{Deserialize, Serialize};

pub const PITCH: usize = 0;
pub const RANGE: usize = 1;
pub const SYNC: usize = 2;
pub const KEY_LOCK: usize = 3;
pub const RPM: usize = 4;
pub const START: usize = 5;
pub const STOP: usize = 6;
pub const BRAKE: usize = 7;
pub const SPIN: usize = 8;
pub const SLIPMAT: usize = 9;
pub const SONG: usize = 10;
pub const QUANTIZE: usize = 11;
pub const LOOP: usize = 12;
pub const MOVE_RATE: usize = 13;
pub const MOVE_SIZE: usize = 14;
pub const FADER: usize = 15;
pub const CURVE: usize = 16;
pub const FILTER: usize = 17;
pub const RESONANCE: usize = 18;
pub const CRACKLE: usize = 19;
pub const NOISE: usize = 20;
pub const WOW: usize = 21;
pub const VOLUME: usize = 22;

pub const RANGES: &[&str] = &["±8 %", "±16 %", "±50 %"];
const RANGE_OF: [f64; 3] = [0.08, 0.16, 0.5];
pub const RPMS: &[&str] = &["33", "45"];
pub const SONGS: &[&str] = &["Free", "Start", "Follow"];
pub const QUANTIZES: &[&str] = &["Off", "Beat", "Bar"];
pub const LOOPS: &[&str] = &["1/4", "1/2", "1", "2", "4", "8"];
const LOOP_BEATS: [f64; 6] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0];
pub const MOVE_RATES: &[&str] = &["1/4", "1/8", "1/16"];
/// Beats a scratch takes, out and back.
const MOVE_BEATS: [f64; 3] = [1.0, 0.5, 0.25];
pub const MOVES: &[&str] = &["Baby", "Forward", "Chirp", "Transformer", "Tear", "Flare"];

/// The hot cues.
pub const CUES: usize = 8;
/// The notes that play it: the hot cues from C1, then start and stop, the
/// brake, a backspin, a loop roll, the scratches and the cut.
pub const CUE_NOTE: u8 = 36;
pub const PLAY_NOTE: u8 = 44;
pub const BRAKE_NOTE: u8 = 45;
pub const BACKSPIN_NOTE: u8 = 46;
pub const ROLL_NOTE: u8 = 47;
pub const MOVE_NOTE: u8 = 48;
pub const CUT_NOTE: u8 = 54;

/// What each of its notes does, for the piano roll (from C1 up).
pub fn note_names() -> Vec<(u8, String)> {
    let mut v: Vec<(u8, String)> = (0..CUES).map(|k| (CUE_NOTE + k as u8, format!("Cue {}", k + 1))).collect();
    v.push((PLAY_NOTE, "Start/Stop".into()));
    v.push((BRAKE_NOTE, "Brake".into()));
    v.push((BACKSPIN_NOTE, "Backspin".into()));
    v.push((ROLL_NOTE, "Loop Roll".into()));
    for (k, m) in MOVES.iter().enumerate() {
        v.push((MOVE_NOTE + k as u8, format!("{m} Scratch")));
    }
    v.push((CUT_NOTE, "Cut".into()));
    v
}

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Pitch", "Deck", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::choice("Range", "Deck", RANGES, 0),
    ParamSpec::toggle("Sync", "Deck", false),
    ParamSpec::toggle("Key Lock", "Deck", false),
    ParamSpec::choice("RPM", "Deck", RPMS, 0),
    ParamSpec::log("Start", "Motor", 0.01, 4.0, 0.3, Unit::Sec),
    ParamSpec::log("Stop", "Motor", 0.01, 4.0, 0.4, Unit::Sec),
    ParamSpec::log("Brake", "Motor", 0.3, 10.0, 2.5, Unit::Sec),
    ParamSpec::log("Backspin", "Motor", 0.2, 4.0, 1.2, Unit::Sec),
    ParamSpec::lin("Slipmat", "Motor", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::choice("Song", "Time", SONGS, 0),
    ParamSpec::choice("Quantize", "Time", QUANTIZES, 1),
    ParamSpec::choice("Loop", "Time", LOOPS, 2),
    ParamSpec::choice("Move Rate", "Scratch", MOVE_RATES, 1),
    ParamSpec::lin("Move Size", "Scratch", 0.05, 0.5, 0.2, Unit::None),
    ParamSpec::lin("Fader", "Mixer", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Curve", "Mixer", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Filter", "Mixer", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Resonance", "Mixer", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Crackle", "Vinyl", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Noise", "Vinyl", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Wow", "Vinyl", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Volume", "Mixer", -30.0, 6.0, -3.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Club Deck", "Battle Deck", "Old Vinyl", "Slow Motor", "Synced Loop", "Follow Song", "Chopped Cuts", "Tape Brake"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Club Deck" => &[(START, 0.15), (STOP, 0.3), (QUANTIZE, 1.0)],
        "Battle Deck" => &[(START, 0.05), (STOP, 0.08), (SLIPMAT, 0.1), (CURVE, 1.0), (MOVE_RATE, 2.0), (MOVE_SIZE, 0.15), (RANGE, 1.0)],
        "Old Vinyl" => &[(CRACKLE, 0.45), (NOISE, 0.35), (WOW, 0.4), (FILTER, -0.18), (START, 0.6), (STOP, 0.8)],
        "Slow Motor" => &[(START, 2.5), (STOP, 3.0), (BRAKE, 6.0), (SLIPMAT, 0.7), (WOW, 0.2)],
        "Synced Loop" => &[(SYNC, 1.0), (KEY_LOCK, 1.0), (SONG, 1.0), (START, 0.01), (QUANTIZE, 2.0), (RANGE, 2.0)],
        "Follow Song" => &[(SYNC, 1.0), (SONG, 2.0), (START, 0.01), (RANGE, 2.0)],
        "Chopped Cuts" => &[(START, 0.02), (STOP, 0.05), (CURVE, 1.0), (MOVE_RATE, 2.0), (QUANTIZE, 0.0), (FILTER, 0.12)],
        "Tape Brake" => &[(BRAKE, 4.0), (SPIN, 2.0), (STOP, 1.5), (WOW, 0.15), (NOISE, 0.1)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

/// What is known of the record on the deck: its tempo and the hot cues.
/// Found when it is put on (see [`listen`]), kept with the set.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Record {
    /// Its tempo (0: none found).
    #[serde(default)]
    pub bpm: f64,
    /// Its first downbeat, in seconds: where its beats are counted from.
    #[serde(default)]
    pub beat: f64,
    /// Whether it has been listened to (its tempo and cues looked for).
    #[serde(default)]
    pub heard: bool,
    /// The hot cues, in seconds into the record.
    #[serde(default)]
    pub cues: [Option<f64>; CUES],
}

/// Listens to a record put on the deck: its tempo and first downbeat, and
/// the eight hot cues spread over it on its bars (or, with no steady beat,
/// on the hits nearest to eighths of it). Takes a moment for a whole song,
/// so never on the audio thread.
pub fn listen(s: &SampleData) -> Record {
    let dur = s.duration();
    let mut rec = Record { heard: true, ..Record::default() };
    let tempo = crate::analysis::detect_tempo(s);
    if let Some(t) = tempo.filter(|t| t.bpm > 0.0) {
        rec.bpm = t.bpm;
        rec.beat = t.downbeat.max(0.0);
        let bar = 4.0 * 60.0 / t.bpm;
        let bars = ((dur - rec.beat) / bar).floor().max(1.0);
        let step = (bars / CUES as f64).floor().max(1.0);
        for (k, c) in rec.cues.iter_mut().enumerate() {
            let at = rec.beat + k as f64 * step * bar;
            if at < dur {
                *c = Some(at);
            }
        }
    } else {
        let hits = &s.analysis().transients;
        let rate = s.sample_rate as f64;
        for (k, c) in rec.cues.iter_mut().enumerate() {
            let want = dur * k as f64 / CUES as f64;
            let near = hits.iter().map(|&h| h as f64 / rate).min_by(|a, b| (a - want).abs().total_cmp(&(b - want).abs()));
            *c = Some(near.filter(|h| (h - want).abs() < dur / CUES as f64 * 0.5).unwrap_or(want));
        }
    }
    rec
}

/// Seconds of the record a turn of the platter holds (60 over the RPM).
pub fn turn_secs(p: &[f32]) -> f64 {
    if p.get(RPM).copied().unwrap_or(0.0) >= 0.5 {
        60.0 / 45.0
    } else {
        60.0 / (100.0 / 3.0)
    }
}

/// The pitch fader's range (as a part: 0.08 for ±8 %).
pub fn range_of(p: &[f32]) -> f64 {
    RANGE_OF[(p[RANGE].round().max(0.0) as usize).min(RANGE_OF.len() - 1)]
}

/// The speed that plays a record of `rec_bpm` in a song of `song_bpm`:
/// their ratio, halved or doubled until it is within ±50 % of the record's
/// own (a record at 70 plays along at 140 beat for beat on the half time).
pub fn sync_speed(rec_bpm: f64, song_bpm: f64) -> Option<f64> {
    if !(rec_bpm > 0.0 && song_bpm > 0.0) {
        return None;
    }
    let mut ratio = song_bpm / rec_bpm;
    while ratio >= 1.5 {
        ratio *= 0.5;
    }
    while ratio < 0.75 {
        ratio *= 2.0;
    }
    Some(ratio)
}

/// Where Sync sets the pitch fader: the narrowest range that holds the
/// speed, and the fader's place in it (-1..1).
pub fn sync_pitch(rec_bpm: f64, song_bpm: f64) -> Option<(usize, f32)> {
    let d = sync_speed(rec_bpm, song_bpm)? - 1.0;
    let range = RANGE_OF.iter().position(|r| d.abs() <= r + 1e-9).unwrap_or(RANGE_OF.len() - 1);
    Some((range, (d / RANGE_OF[range]).clamp(-1.0, 1.0) as f32))
}

/// The speed the motor turns the platter at: the pitch fader's, or with
/// Sync (and a tempo known) the song's tempo over the record's.
pub fn motor_speed(p: &[f32], rec: &Record, song_bpm: f64) -> f64 {
    if p[SYNC] >= 0.5 {
        if let Some(s) = sync_speed(rec.bpm, song_bpm) {
            return s;
        }
    }
    1.0 + p[PITCH].clamp(-1.0, 1.0) as f64 * range_of(p)
}

/// A scratch at `ph` (0..1 of it, out and back): how far the hand has
/// taken the record (0..1 of the move's size) and whether the cut fader is
/// open.
pub fn move_shape(which: usize, ph: f64) -> (f64, bool) {
    let ph = ph.rem_euclid(1.0);
    let baby = (1.0 - (std::f64::consts::TAU * ph).cos()) * 0.5;
    let half = ph % 0.5;
    match which {
        // Forward with the fader open, back with it closed.
        1 => (baby, ph < 0.5),
        // Open as each stroke starts, cut just before it turns.
        2 => (baby, half < 0.38),
        // Out and back, the fader clicked open and closed four times a stroke.
        3 => (baby, ((ph * 8.0) as i64) % 2 == 0),
        // Out in one stroke, back in two with a stop between.
        4 => {
            let ease = |x: f64| (1.0 - (std::f64::consts::PI * x.clamp(0.0, 1.0)).cos()) * 0.5;
            let d = if ph < 0.4 {
                ease(ph / 0.4)
            } else if ph < 0.6 {
                1.0 - 0.5 * ease((ph - 0.4) / 0.2)
            } else if ph < 0.7 {
                0.5
            } else {
                0.5 - 0.5 * ease((ph - 0.7) / 0.3)
            };
            (d, true)
        }
        // The fader open, clicked shut once in the middle of each stroke.
        5 => (baby, !(0.22..0.28).contains(&half)),
        // Baby: out and back, the fader open.
        _ => (baby, true),
    }
}

/// What the face (or a script) does to the deck.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DeckAct {
    /// The hand on the platter, turned so many turns from where it took
    /// hold of it; None: let go. Sent again while it is on.
    Hand(Option<f64>),
    StartStop,
    Brake,
    Backspin,
    /// To this place of the record (seconds): a hot cue (and it plays), or
    /// the needle dropped there (it goes on as it was).
    Jump { secs: f64, play: bool },
    /// The loop on or off (the record stays where the loop took it).
    Loop(bool),
    /// The sound cut while held.
    Cut(bool),
    /// A scratch played while held.
    Move(usize, bool),
}

/// Zero crossings either side in the interpolator, points between two.
const HALF: usize = 8;
const RES: usize = 128;
/// The fastest it band-limits for (faster aliases a little).
const MAX_WIDE: f32 = 4.0;
/// How firmly the record follows the hand (a spring, critically damped).
const HAND_HZ: f64 = 18.0;
/// How fast a backspin throws the record back.
const SPIN_SPEED: f64 = 3.0;
/// The length of a key-locked grain (seconds).
const GRAIN: f64 = 0.05;
/// How long a jump is softened over (seconds).
const XF: f64 = 0.002;
/// Commands waiting for the next block.
const INBOX: usize = 16;

/// A Kaiser-windowed sinc, `RES` points per zero crossing (one more at the end).
fn kernel() -> Box<[f32]> {
    let beta = 7.5f64;
    let i0 = |x: f64| {
        let (mut sum, mut term) = (1.0f64, 1.0f64);
        for k in 1..32 {
            term *= (x / 2.0 / k as f64).powi(2);
            sum += term;
        }
        sum
    };
    (0..HALF * RES + 2)
        .map(|j| {
            let x = j as f64 / RES as f64;
            if x >= HALF as f64 {
                return 0.0;
            }
            let w = i0(beta * (1.0 - (x / HALF as f64).powi(2)).sqrt()) / i0(beta);
            let s = if j == 0 { 1.0 } else { (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x) };
            (s * w) as f32
        })
        .collect()
}

#[inline]
fn tap(kernel: &[f32], x: f32) -> f32 {
    let a = x.abs() * RES as f32;
    let i = a as usize;
    if i >= HALF * RES {
        return 0.0;
    }
    kernel[i] + (kernel[i + 1] - kernel[i]) * (a - i as f32)
}

/// The record at `pos` (frames), read `step` frames a sample: a windowed
/// sinc, widened when it reads faster than the record's rate so what would
/// fold back is filtered away. Silence off the ends.
fn read(kernel: &[f32], s: &SampleData, pos: f64, step: f64) -> (f32, f32) {
    let n = s.left.len().min(s.right.len()) as i64;
    let c = 1.0 / (step.abs() as f32).clamp(1.0, MAX_WIDE);
    let reach = (HALF as f32 / c).ceil() as i64;
    if !(pos > -(reach as f64) && pos < (n + reach) as f64) {
        return (0.0, 0.0);
    }
    let i0 = pos.floor() as i64;
    let frac = (pos - i0 as f64) as f32;
    let (lo, hi) = ((i0 - reach + 1).max(0), (i0 + reach).min(n - 1));
    let (mut l, mut r) = (0.0f32, 0.0f32);
    let mut i = lo;
    while i <= hi {
        let w = tap(kernel, ((i - i0) as f32 - frac) * c);
        l += s.left[i as usize] * w;
        r += s.right[i as usize] * w;
        i += 1;
    }
    (l * c, r * c)
}

/// How the platter slows when the motor lets go of it.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Slow {
    Stop,
    Brake,
    Spin,
}

/// The hand on the record: where it has it (frames), how fast it moves it
/// (frames a sample), and the samples since it was last heard of.
#[derive(Clone, Copy, Debug)]
struct Hand {
    anchor: f64,
    at: f64,
    speed: f64,
    since: u32,
    gap: u32,
}

#[derive(Clone, Copy, Debug)]
struct Move {
    which: usize,
    anchor: f64,
    /// Beats since it started.
    t: f64,
}

#[derive(Clone, Copy, Debug)]
struct Looping {
    start: f64,
    len: f64,
    /// Where the record would be without the loop (a roll comes back there).
    shadow: f64,
    roll: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct Grain {
    start: f64,
    age: u32,
}

pub struct TurntableState {
    sr: f32,
    /// The song: its tempo, the beat (at the block's start, then counted on) and whether it plays.
    pub bpm: f32,
    beat: f64,
    playing: bool,
    was_playing: bool,
    kernel: Box<[f32]>,
    /// Where the needle is (frames of the record) and how fast the record turns (1: as recorded).
    pos: f64,
    vel: f64,
    /// The platter, under the record.
    platter: f64,
    motor: bool,
    slow: Slow,
    hand: Option<Hand>,
    moving: Option<Move>,
    looping: Option<Looping>,
    cut_face: bool,
    cut_note: bool,
    /// A command waiting for the beat (Quantize).
    waiting: Option<(DeckAct, f64)>,
    inbox: [Option<DeckAct>; INBOX],
    /// The fader's gain now, and the old place still sounding while a jump fades in.
    gain: f32,
    xf: Option<(f64, u32)>,
    grains: [Grain; 2],
    lock: f32,
    filter: [Svf; 2],
    dc: [DcBlock; 2],
    rng: Rng,
    crackle: f32,
    hiss: [f32; 2],
    rumble: f32,
    wow: f64,
    flutter: f32,
    level: f32,
    frames: usize,
    rate: f64,
    /// The speed the motor turns it at now.
    target: f64,
}

impl TurntableState {
    pub fn new(sr: f32) -> Self {
        Self {
            sr,
            bpm: 120.0,
            beat: 0.0,
            playing: false,
            was_playing: false,
            kernel: kernel(),
            pos: 0.0,
            vel: 0.0,
            platter: 0.0,
            motor: false,
            slow: Slow::Stop,
            hand: None,
            moving: None,
            looping: None,
            cut_face: false,
            cut_note: false,
            waiting: None,
            inbox: [None; INBOX],
            gain: 1.0,
            xf: None,
            grains: [Grain::default(); 2],
            lock: 0.0,
            filter: [Svf::new(), Svf::new()],
            dc: [DcBlock::default(); 2],
            rng: Rng::new(0x7A8B_1E00),
            crackle: 0.0,
            hiss: [0.0; 2],
            rumble: 0.0,
            wow: 0.0,
            flutter: 0.0,
            level: 0.0,
            frames: 0,
            rate: sr as f64,
            target: 1.0,
        }
    }

    pub fn set_transport(&mut self, bpm: f32, beat: f64, playing: bool) {
        self.bpm = bpm.clamp(20.0, 999.0);
        self.beat = beat;
        self.playing = playing;
    }

    /// Where the needle is, in seconds of the record.
    pub fn position(&self) -> f64 {
        self.pos / self.rate.max(1.0)
    }

    /// How fast the record turns (1: as recorded; backwards below 0).
    pub fn speed(&self) -> f64 {
        self.vel
    }

    /// How fast the platter turns under it.
    pub fn platter(&self) -> f64 {
        self.platter
    }

    pub fn running(&self) -> bool {
        self.motor
    }

    /// A command from the face: done at the start of the next block.
    pub fn act(&mut self, a: DeckAct) {
        // A hand's new place replaces one not yet taken.
        if let DeckAct::Hand(Some(_)) = a {
            if let Some(slot) = self.inbox.iter_mut().find(|s| matches!(s, Some(DeckAct::Hand(Some(_))))) {
                *slot = Some(a);
                return;
            }
        }
        if let Some(slot) = self.inbox.iter_mut().find(|s| s.is_none()) {
            *slot = Some(a);
        }
    }

    pub fn note_on(&mut self, p: &[f32], rec: &Record, note: u8, _vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        match note {
            n if (CUE_NOTE..CUE_NOTE + CUES as u8).contains(&n) => {
                if let Some(secs) = rec.cues[(n - CUE_NOTE) as usize] {
                    self.jump(secs * self.rate, true);
                }
            }
            PLAY_NOTE => self.start_stop(),
            BRAKE_NOTE => self.brake(),
            BACKSPIN_NOTE => self.backspin(),
            ROLL_NOTE => self.loop_on(p, rec, true),
            n if (MOVE_NOTE..MOVE_NOTE + MOVES.len() as u8).contains(&n) => self.move_on((n - MOVE_NOTE) as usize),
            CUT_NOTE => self.cut_note = true,
            _ => {}
        }
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        match note {
            ROLL_NOTE => self.loop_off(),
            n if (MOVE_NOTE..MOVE_NOTE + MOVES.len() as u8).contains(&n) => {
                if self.moving.is_some_and(|m| m.which == (n - MOVE_NOTE) as usize) {
                    self.moving = None;
                }
            }
            CUT_NOTE => self.cut_note = false,
            _ => {}
        }
    }

    pub fn all_off(&mut self) {
        self.moving = None;
        self.cut_note = false;
        if self.looping.is_some_and(|l| l.roll) {
            self.loop_off();
        }
    }

    fn start_stop(&mut self) {
        if self.motor {
            self.motor = false;
            self.slow = Slow::Stop;
        } else {
            self.motor = true;
        }
    }

    fn brake(&mut self) {
        self.motor = false;
        self.slow = Slow::Brake;
    }

    fn backspin(&mut self) {
        self.motor = false;
        self.slow = Slow::Spin;
        self.platter = -SPIN_SPEED;
        self.vel = -SPIN_SPEED;
        self.hand = None;
        self.moving = None;
    }

    /// To `to` (frames), softened: the old place fades out as the new one comes in.
    fn jump(&mut self, to: f64, play: bool) {
        if (to - self.pos).abs() > 0.5 {
            self.xf = Some((self.pos, (XF * self.sr as f64) as u32));
        }
        self.pos = to;
        if let Some(l) = self.looping.as_mut() {
            // A jump out of a loop ends it.
            if to < l.start || to >= l.start + l.len {
                self.looping = None;
            }
        }
        if let Some(m) = self.moving.as_mut() {
            m.anchor = to;
        }
        if let Some(h) = self.hand.as_mut() {
            let d = to - h.anchor;
            h.anchor += d;
            h.at += d;
        }
        if play && !self.motor {
            self.motor = true;
        }
    }

    fn move_on(&mut self, which: usize) {
        self.moving = Some(Move { which, anchor: self.pos, t: 0.0 });
    }

    /// The length of a beat of the record (frames): its own tempo's, or the song's at the motor's speed.
    fn beat_frames(&self, p: &[f32], rec: &Record) -> f64 {
        let secs = if rec.bpm > 0.0 { 60.0 / rec.bpm } else { 60.0 / self.bpm as f64 * motor_speed(p, rec, self.bpm as f64).abs().max(0.05) };
        secs * self.rate
    }

    fn loop_on(&mut self, p: &[f32], rec: &Record, roll: bool) {
        let beat = self.beat_frames(p, rec);
        let len = LOOP_BEATS[(p[LOOP].round().max(0.0) as usize).min(LOOP_BEATS.len() - 1)] * beat;
        let origin = rec.beat * self.rate;
        let q = len.min(beat).max(1.0);
        let start = origin + ((self.pos - origin) / q).floor() * q;
        self.looping = Some(Looping { start, len: len.max(1.0), shadow: self.pos, roll });
    }

    fn loop_off(&mut self) {
        if let Some(l) = self.looping.take() {
            if l.roll {
                self.jump(l.shadow, false);
            }
        }
    }

    fn take(&mut self, a: DeckAct, p: &[f32], rec: &Record) {
        match a {
            DeckAct::Hand(Some(turns)) => {
                let frames = turns * turn_secs(p) * self.rate;
                match self.hand.as_mut() {
                    Some(h) => {
                        let to = h.anchor + frames;
                        let gap = h.since.max(1);
                        h.speed = ((to - h.at) / gap as f64).clamp(-40.0, 40.0);
                        h.at = to;
                        h.gap = gap;
                        h.since = 0;
                    }
                    None => self.hand = Some(Hand { anchor: self.pos - frames, at: self.pos, speed: 0.0, since: 0, gap: 1024 }),
                }
            }
            DeckAct::Hand(None) => self.hand = None,
            DeckAct::StartStop => self.start_stop(),
            DeckAct::Brake => self.brake(),
            DeckAct::Backspin => self.backspin(),
            DeckAct::Jump { secs, play } => self.jump(secs * self.rate, play),
            DeckAct::Loop(true) => self.loop_on(p, rec, false),
            DeckAct::Loop(false) => self.loop_off(),
            DeckAct::Cut(on) => self.cut_face = on,
            DeckAct::Move(k, true) => self.move_on(k.min(MOVES.len() - 1)),
            DeckAct::Move(k, false) => {
                if self.moving.is_some_and(|m| m.which == k) {
                    self.moving = None;
                }
            }
        }
    }

    /// For the face: where the needle is and how long the record is
    /// (seconds), the record's and the platter's speed, the motor, the
    /// hand, the scratch playing (-1: none), the loop (on, its start and
    /// length in seconds), the level, the fader's gain, the motor's speed,
    /// a start waiting for the beat, and how much Key Lock holds the pitch.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        let rate = self.rate.max(1.0);
        out[0] = (self.pos / rate) as f32;
        out[1] = (self.frames as f64 / rate) as f32;
        out[2] = self.vel as f32;
        out[3] = self.platter as f32;
        out[4] = if self.motor { 1.0 } else { 0.0 };
        out[5] = if self.hand.is_some() { 1.0 } else { 0.0 };
        out[6] = self.moving.map_or(-1.0, |m| m.which as f32);
        out[7] = if self.looping.is_some() { 1.0 } else { 0.0 };
        out[8] = self.looping.map_or(0.0, |l| (l.start / rate) as f32);
        out[9] = self.looping.map_or(0.0, |l| (l.len / rate) as f32);
        out[10] = self.level;
        out[11] = self.gain;
        out[12] = self.target as f32;
        out[13] = if self.waiting.is_some() { 1.0 } else { 0.0 };
        out[14] = self.lock;
    }

    pub fn render(&mut self, p: &[f32], sample: Option<&SampleRef>, rec: &Record, l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let src: Option<&SampleData> = sample.map(|s| &**s).filter(|s| s.frames() > 1);
        if let Some(s) = src {
            self.rate = s.sample_rate as f64;
            if s.frames() != self.frames {
                // Another record: from its start.
                self.frames = s.frames();
                self.pos = 0.0;
                self.xf = None;
                self.looping = None;
                self.moving = None;
            }
        }
        let song_bpm = self.bpm as f64;
        let target = motor_speed(p, rec, song_bpm);
        self.target = target;
        let mode = p[SONG].round() as i32;
        // The song starting or stopping.
        if self.playing != self.was_playing {
            self.was_playing = self.playing;
            if self.playing && mode >= 1 {
                self.motor = true;
                if mode == 2 {
                    self.platter = target;
                    self.vel = target;
                    let want = self.follow_place(rec, target);
                    self.jump(want, true);
                }
            } else if !self.playing && mode >= 1 {
                self.motor = false;
                self.slow = Slow::Stop;
                self.waiting = None;
            }
        }
        // The face's commands; a start or a cue waits for the beat (or bar) when the song plays.
        let quant = [0.0, 1.0, 4.0][(p[QUANTIZE].round().max(0.0) as usize).min(2)];
        for k in 0..INBOX {
            let Some(a) = self.inbox[k].take() else { continue };
            let waits = matches!(a, DeckAct::StartStop | DeckAct::Jump { play: true, .. }) && quant > 0.0 && self.playing;
            if waits {
                let at = ((self.beat + 1e-6) / quant).ceil() * quant;
                self.waiting = Some((a, at));
            } else {
                self.take(a, p, rec);
            }
        }
        // Following the song: its place pulled to where the song is.
        if mode == 2 && self.playing && self.hand.is_none() && self.moving.is_none() && self.looping.is_none() && self.motor {
            let want = self.follow_place(rec, target);
            let off = want - self.pos;
            if off.abs() > 0.02 * self.rate {
                self.jump(want, false);
                self.platter = target;
                self.vel = target;
            } else {
                self.pos += off * 0.05;
            }
        }
        let mut kernel = std::mem::take(&mut self.kernel);
        let sr = self.sr as f64;
        let ratio = self.rate / sr;
        let dt = 1.0 / sr;
        let beat_step = if self.playing { song_bpm / 60.0 * dt } else { 0.0 };
        let move_beat = song_bpm / 60.0 * dt;
        let start_rate = 1.0 / p[START].max(0.005) as f64;
        let slow_rate = match self.slow {
            Slow::Stop => 1.0 / p[STOP].max(0.005) as f64,
            Slow::Brake => 1.0 / p[BRAKE].max(0.05) as f64,
            Slow::Spin => SPIN_SPEED / p[SPIN].max(0.05) as f64,
        };
        // The mat pulls the record to the platter's speed (a time constant of 20 to 400 ms).
        let slip = 1.0 - (-1.0 / ((0.02 + 0.38 * p[SLIPMAT].clamp(0.0, 1.0) as f64) * sr)).exp();
        let w = std::f64::consts::TAU * HAND_HZ / sr;
        let move_size = p[MOVE_SIZE].clamp(0.01, 1.0) as f64 * turn_secs(p) * self.rate;
        let move_beats = MOVE_BEATS[(p[MOVE_RATE].round().max(0.0) as usize).min(MOVE_BEATS.len() - 1)];
        let key_lock = p[KEY_LOCK] >= 0.5;
        let grain_len = (GRAIN * sr) as u32;
        let lock_k = 1.0 - (-1.0 / (0.012 * self.sr)).exp();
        let gain_k = 1.0 - (-1.0 / (0.0006 * self.sr)).exp();
        let fader = {
            let (f, c) = (p[FADER].clamp(0.0, 1.0), p[CURVE].clamp(0.0, 1.0));
            // Smooth: a slow rise; sharp: open in the first few per cent.
            (f * (1.0 + 29.0 * c * c)).min(1.0).powf(2.0 - c)
        };
        let volume = db_to_gain(p[VOLUME]);
        // The DJ filter: low-pass to the left, high-pass to the right.
        let fv = p[FILTER].clamp(-1.0, 1.0);
        let res = p[RESONANCE].clamp(0.0, 1.0) * 0.75;
        let fmode = if fv < -0.02 {
            let c = 20_000.0 * 2f32.powf(fv * 9.5);
            self.filter.iter_mut().for_each(|f| f.set(c, res, self.sr));
            Some(FilterMode::Low)
        } else if fv > 0.02 {
            let c = 20.0 * 2f32.powf(fv * 9.5);
            self.filter.iter_mut().for_each(|f| f.set(c, res, self.sr));
            Some(FilterMode::High)
        } else {
            None
        };
        let crackle = p[CRACKLE].clamp(0.0, 1.0);
        let noise = p[NOISE].clamp(0.0, 1.0);
        let wow = p[WOW].clamp(0.0, 1.0) as f64;
        let rot = 1.0 / turn_secs(p);
        let hiss_k = 1.0 - (-std::f32::consts::TAU * 3000.0 / self.sr).exp();
        let rumble_k = 1.0 - (-std::f32::consts::TAU * 25.0 / self.sr).exp();
        let flutter_k = 1.0 - (-std::f32::consts::TAU * 9.0 / self.sr).exp();
        let mut peak = 0.0f32;
        for i in 0..n {
            // A command waiting for its beat.
            if let Some((a, at)) = self.waiting {
                if self.beat >= at - 1e-9 || !self.playing {
                    self.waiting = None;
                    self.take(a, p, rec);
                }
            }
            self.beat += beat_step;
            // The platter: the motor brings it up to speed, or lets it run down.
            let goal = if self.motor { target } else { 0.0 };
            let step = if self.motor { start_rate * dt } else { slow_rate * dt };
            self.platter += (goal - self.platter).clamp(-step, step);
            // The record: on the hand, on a scratch (which moves it exactly
            // as written), or on the mat.
            let mut gate = true;
            let mut exact = false;
            let held = if let Some(m) = self.moving.as_mut() {
                m.t += move_beat;
                let ph = m.t / move_beats;
                let (d, g) = move_shape(m.which, ph);
                let d2 = move_shape(m.which, ph + 1e-3).0;
                gate = g;
                exact = true;
                let samples_per_cycle = move_beats * 60.0 / song_bpm * sr;
                Some((m.anchor + d * move_size, (d2 - d) / 1e-3 * move_size / samples_per_cycle))
            } else if let Some(h) = self.hand.as_mut() {
                h.since = h.since.saturating_add(1);
                // Between its places the hand goes on as it went (for a while).
                if h.since <= h.gap + h.gap / 2 {
                    h.at += h.speed;
                } else {
                    h.speed = 0.0;
                }
                Some((h.at, h.speed))
            } else {
                None
            };
            let wobble = if let Some((at, speed)) = held {
                if exact {
                    self.vel = speed / ratio;
                    // Lands where the scratch has it (the step below takes it there).
                    self.pos = at - speed;
                } else {
                    let e = at - self.pos;
                    let ev = speed - self.vel * ratio;
                    self.vel += (w * w * e + 2.0 * w * ev) / ratio;
                }
                1.0
            } else {
                self.vel += (self.platter - self.vel) * slip;
                // Wow at the platter's turn, flutter faster and less even.
                self.wow = (self.wow + self.platter.abs() * rot * dt).fract();
                self.flutter += (self.rng.bipolar() - self.flutter) * flutter_k;
                1.0 + wow * (0.004 * (std::f64::consts::TAU * self.wow).sin() + 0.02 * self.flutter as f64)
            };
            if !self.vel.is_finite() {
                self.vel = 0.0;
            }
            let step = self.vel * wobble * ratio;
            self.pos += step;
            if let Some((old, left)) = self.xf.as_mut() {
                *old += step;
                *left = left.saturating_sub(1);
            }
            // The loop: back to its start (or end, backwards), the roll's shadow going on.
            if let Some(lp) = self.looping.as_mut() {
                lp.shadow += step;
                let end = lp.start + lp.len;
                let wrap = if self.pos >= end {
                    Some(self.pos - lp.len)
                } else if self.pos < lp.start && step < 0.0 {
                    Some(self.pos + lp.len)
                } else {
                    None
                };
                if let Some(to) = wrap {
                    self.xf = Some((self.pos, (XF * sr) as u32));
                    self.pos = to;
                }
            }
            if !self.pos.is_finite() {
                self.pos = 0.0;
            }
            // Key Lock: on while the motor alone turns it at its speed.
            let locked = key_lock && held.is_none() && self.motor && (self.vel - target).abs() < 0.01 && (target - 1.0).abs() > 1e-4;
            let was = self.lock;
            self.lock += ((if locked { 1.0 } else { 0.0 }) - self.lock) * lock_k;
            if self.lock < 1e-4 {
                self.lock = 0.0;
            }
            let (mut sl, mut sr_) = (0.0f32, 0.0f32);
            if let Some(s) = src {
                let (vl, vr) = if self.lock < 0.9999 { read(&kernel, s, self.pos, step) } else { (0.0, 0.0) };
                let (mut kl, mut kr) = (0.0f32, 0.0f32);
                if self.lock > 0.0 {
                    if was == 0.0 {
                        self.grains = [Grain { start: self.pos, age: 0 }, Grain { start: self.pos - grain_len as f64 * 0.5 * ratio, age: grain_len / 2 }];
                    }
                    for g in self.grains.iter_mut() {
                        if g.age >= grain_len {
                            *g = Grain { start: self.pos, age: 0 };
                        }
                        let win = 0.5 - 0.5 * (std::f32::consts::TAU * g.age as f32 / grain_len as f32).cos();
                        let (a, b) = read(&kernel, s, g.start + g.age as f64 * ratio, ratio);
                        kl += a * win;
                        kr += b * win;
                        g.age += 1;
                    }
                }
                sl = vl * (1.0 - self.lock) + kl * self.lock;
                sr_ = vr * (1.0 - self.lock) + kr * self.lock;
                if let Some((old, left)) = self.xf {
                    let len = (XF * sr).max(1.0) as f32;
                    let f = left as f32 / len;
                    let (ol, or) = read(&kernel, s, old, step);
                    sl = sl * (1.0 - f) + ol * f;
                    sr_ = sr_ * (1.0 - f) + or * f;
                    if left == 0 {
                        self.xf = None;
                    }
                }
            } else {
                self.xf = None;
            }
            // The record's own noise, as fast as it turns.
            let turning = (self.vel.abs() as f32).min(2.0);
            if crackle > 0.0 && turning > 0.01 {
                if self.rng.unipolar() < crackle * crackle * 60.0 * turning / self.sr {
                    let pop = self.rng.unipolar();
                    self.crackle += (0.02 + 0.3 * pop * pop * pop) * if self.rng.unipolar() < 0.5 { -1.0 } else { 1.0 };
                }
            }
            let pop = self.crackle;
            self.crackle *= 0.55;
            if noise > 0.0 && turning > 0.01 {
                for h in self.hiss.iter_mut() {
                    let x = self.rng.bipolar();
                    *h += (x - *h) * hiss_k;
                }
                self.rumble += (self.rng.bipolar() - self.rumble) * rumble_k;
            }
            let surface = noise * turning * 0.5;
            let hl = (self.hiss[0] * 0.05 + self.rumble * 0.4) * surface;
            let hr = (self.hiss[1] * 0.05 + self.rumble * 0.4) * surface;
            let (mut ol, mut or) = (sl + pop + hl, sr_ + pop + hr);
            if let Some(m) = fmode {
                ol = self.filter[0].process(ol, m);
                or = self.filter[1].process(or, m);
            }
            // A cartridge passes no DC (a record stopped on a slope is silent).
            ol = self.dc[0].tick(ol);
            or = self.dc[1].tick(or);
            let open = if gate && !self.cut_face && !self.cut_note { fader } else { 0.0 };
            self.gain += (open - self.gain) * gain_k;
            ol *= self.gain * volume;
            or *= self.gain * volume;
            if !ol.is_finite() || !or.is_finite() {
                ol = 0.0;
                or = 0.0;
                self.filter = [Svf::new(), Svf::new()];
                self.dc = [DcBlock::default(); 2];
            }
            l[i] += ol;
            r[i] += or;
            peak = peak.max(ol.abs()).max(or.abs());
        }
        self.kernel = std::mem::take(&mut kernel);
        self.level = peak.max(self.level * 0.9);
    }

    /// Where the song's beat puts the record (frames): its downbeat on the
    /// song's first, on at the motor's speed.
    fn follow_place(&self, rec: &Record, speed: f64) -> f64 {
        let song_secs = self.beat * 60.0 / self.bpm.max(1.0) as f64;
        (rec.beat + song_secs * speed) * self.rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn sine(freq: f32, secs: f32) -> SampleRef {
        let n = (SR * secs) as usize;
        let v: Vec<f32> = (0..n).map(|i| (std::f32::consts::TAU * freq * i as f32 / SR).sin() * 0.5).collect();
        SampleRef::new(SampleData::from_buffers("sine", None, SR as u32, v.clone(), v))
    }

    fn plain() -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        p[VOLUME] = 0.0;
        p[START] = 0.01;
        p[STOP] = 0.01;
        p
    }

    fn run(st: &mut TurntableState, p: &[f32], s: Option<&SampleRef>, rec: &Record, secs: f32) -> Vec<f32> {
        let mut out = Vec::new();
        for _ in 0..(secs * SR / 256.0) as usize {
            let (mut l, mut r) = ([0.0f32; 256], [0.0f32; 256]);
            st.render(p, s, rec, &mut l, &mut r);
            out.extend_from_slice(&l);
        }
        out
    }

    /// The frequency of `x` from its upward zero crossings.
    fn freq(x: &[f32]) -> f32 {
        let ups: Vec<usize> = (1..x.len()).filter(|&i| x[i - 1] <= 0.0 && x[i] > 0.0).collect();
        if ups.len() < 3 {
            return 0.0;
        }
        (ups.len() - 1) as f32 * SR / (ups[ups.len() - 1] - ups[0]) as f32
    }

    /// The strength of `freq` in `x` (Goertzel), and of the 20 Hz about it.
    fn power(x: &[f32], freq: f32) -> f32 {
        let w = std::f32::consts::TAU * freq / SR;
        let c = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f32, 0.0f32);
        for &v in x {
            let s = v + c * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        (s1 * s1 + s2 * s2 - c * s1 * s2) / (x.len() as f32).powi(2)
    }

    fn band(x: &[f32], freq: f32) -> f32 {
        (-5..=5).map(|k| power(x, freq + 2.0 * k as f32)).sum()
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0f32, |m, v| m.max(v.abs()))
    }

    #[test]
    fn the_pitch_follows_the_platter_forwards_and_backwards() {
        let s = sine(440.0, 20.0);
        let rec = Record::default();
        let mut p = plain();
        // The motor at +8 %: 8 % higher.
        p[PITCH] = 1.0;
        let mut st = TurntableState::new(SR);
        st.act(DeckAct::StartStop);
        let x = run(&mut st, &p, Some(&s), &rec, 1.0);
        let f = freq(&x[24_000..]);
        assert!((f - 475.2).abs() < 0.5, "at +8 %: {f}");
        // The hand moving the record twice as fast, then backwards at 1.5.
        for (speed, want) in [(2.0f64, 880.0f32), (-1.5, 660.0)] {
            let mut st = TurntableState::new(SR);
            st.pos = 5.0 * SR as f64;
            let turn = turn_secs(&p) * SR as f64;
            let mut turns = 0.0;
            let mut out = Vec::new();
            for _ in 0..375 {
                // A hand's place every 256 samples, as the face sends them about.
                st.act(DeckAct::Hand(Some(turns)));
                let (mut l, mut r) = ([0.0f32; 256], [0.0f32; 256]);
                st.render(&p, Some(&s), &rec, &mut l, &mut r);
                out.extend_from_slice(&l);
                turns += speed * 256.0 / turn;
            }
            let f = freq(&out[out.len() / 2..]);
            assert!((f - want).abs() < 2.0, "the hand at {speed}: {f} Hz");
            assert!((st.speed() - speed).abs() < 0.02, "the record at {}", st.speed());
            // Let go: the mat brings it back to the platter (stopped here).
            st.act(DeckAct::Hand(None));
            run(&mut st, &p, Some(&s), &rec, 1.5);
            assert!(st.speed().abs() < 0.01, "back on the mat: {}", st.speed());
        }
    }

    #[test]
    fn key_lock_holds_the_pitch_at_another_speed() {
        let s = sine(440.0, 20.0);
        let rec = Record::default();
        let mut p = plain();
        p[RANGE] = 1.0;
        p[PITCH] = 1.0;
        p[KEY_LOCK] = 1.0;
        let mut st = TurntableState::new(SR);
        st.act(DeckAct::StartStop);
        let x = run(&mut st, &p, Some(&s), &rec, 2.0);
        let x = &x[48_000..];
        let (kept, moved) = (band(x, 440.0), band(x, 440.0 * 1.16));
        assert!(kept > moved * 20.0, "key locked at +16 %: {kept} at 440 Hz, {moved} at 510 Hz");
        // Without it, the vinyl's way: 16 % higher.
        p[KEY_LOCK] = 0.0;
        let x = run(&mut st, &p, Some(&s), &rec, 1.0);
        let x = &x[24_000..];
        assert!(band(x, 440.0 * 1.16) > band(x, 440.0) * 20.0);
        p[KEY_LOCK] = 1.0;
        run(&mut st, &p, Some(&s), &rec, 1.0);
        // And the record still goes 16 % faster.
        let before = st.position();
        run(&mut st, &p, Some(&s), &rec, 1.0);
        assert!((st.position() - before - 1.16).abs() < 0.01, "{}", st.position() - before);
    }

    #[test]
    fn the_motor_takes_its_start_and_stop_times() {
        let s = sine(440.0, 30.0);
        let rec = Record::default();
        let mut p = plain();
        p[START] = 1.0;
        p[STOP] = 0.5;
        p[BRAKE] = 3.0;
        let mut st = TurntableState::new(SR);
        st.act(DeckAct::StartStop);
        run(&mut st, &p, Some(&s), &rec, 0.5);
        assert!((st.platter() - 0.5).abs() < 0.02, "half way up in half the start time: {}", st.platter());
        run(&mut st, &p, Some(&s), &rec, 0.52);
        assert!((st.platter() - 1.0).abs() < 1e-6, "up to speed: {}", st.platter());
        st.act(DeckAct::StartStop);
        run(&mut st, &p, Some(&s), &rec, 0.25);
        assert!((st.platter() - 0.5).abs() < 0.02, "half way down: {}", st.platter());
        run(&mut st, &p, Some(&s), &rec, 0.27);
        assert_eq!(st.platter(), 0.0);
        // The brake: the long way down.
        st.act(DeckAct::StartStop);
        run(&mut st, &p, Some(&s), &rec, 1.1);
        st.act(DeckAct::Brake);
        run(&mut st, &p, Some(&s), &rec, 1.5);
        assert!((st.platter() - 0.5).abs() < 0.02, "braking: {}", st.platter());
        run(&mut st, &p, Some(&s), &rec, 2.5);
        assert_eq!(st.platter(), 0.0);
        assert!(st.speed().abs() < 0.01);
        // A backspin throws it back, and it runs down in its time.
        st.act(DeckAct::Backspin);
        run(&mut st, &p, Some(&s), &rec, 0.1);
        assert!(st.speed() < -2.0, "{}", st.speed());
        run(&mut st, &p, Some(&s), &rec, p[SPIN] + 0.5);
        assert!(st.speed().abs() < 0.01);
    }

    #[test]
    fn sync_sets_the_pitch_of_a_loop_of_known_tempo() {
        // A loop of clicks at 100 BPM.
        let bpm = 100.0;
        let n = (SR * 16.0) as usize;
        let beat = (60.0 / bpm * SR as f64) as usize;
        let mut v = vec![0.0f32; n];
        for b in (0..n).step_by(beat) {
            for (k, x) in v[b..(b + 600).min(n)].iter_mut().enumerate() {
                *x = (k as f32 * 0.3).sin() * (-(k as f32) / 120.0).exp() * if (b / beat) % 4 == 0 { 0.9 } else { 0.5 };
            }
        }
        let s = SampleRef::new(SampleData::from_buffers("loop", None, SR as u32, v.clone(), v));
        let rec = listen(&s);
        assert!((rec.bpm - 100.0).abs() < 0.5, "heard at {}", rec.bpm);
        assert!(rec.cues.iter().all(|c| c.is_some()));
        // In a song at 120: 20 % faster, on the ±50 % range.
        let (range, pitch) = sync_pitch(100.0, 120.0).unwrap();
        assert_eq!(range, 2);
        assert!((pitch - 0.4).abs() < 1e-6);
        // At 104 it fits the ±8 % range; at 200 it plays beat for beat on the half time.
        assert_eq!(sync_pitch(100.0, 104.0).unwrap().0, 0);
        assert!((sync_speed(100.0, 200.0).unwrap() - 1.0).abs() < 1e-9);
        // The motor turns at the speed the song asks.
        let mut p = plain();
        p[SYNC] = 1.0;
        let mut st = TurntableState::new(SR);
        st.set_transport(120.0, 0.0, false);
        st.act(DeckAct::StartStop);
        run(&mut st, &p, Some(&s), &rec, 0.5);
        assert!((st.platter() - 120.0 / rec.bpm).abs() < 1e-9, "{}", st.platter());
    }

    #[test]
    fn a_hot_cue_jumps_exactly() {
        let s = sine(440.0, 20.0);
        let mut rec = Record::default();
        rec.cues[2] = Some(3.25);
        let mut p = plain();
        p[SLIPMAT] = 0.0;
        let mut st = TurntableState::new(SR);
        st.act(DeckAct::StartStop);
        run(&mut st, &p, Some(&s), &rec, 1.0);
        assert!((st.speed() - 1.0).abs() < 1e-12);
        st.note_on(&p, &rec, CUE_NOTE + 2, 1.0);
        assert_eq!(st.position(), 3.25);
        let (mut l, mut r) = ([0.0f32; 100], [0.0f32; 100]);
        st.render(&p, Some(&s), &rec, &mut l, &mut r);
        assert!((st.pos - (3.25 * SR as f64 + 100.0)).abs() < 1e-6, "{}", st.pos);
        // An empty cue does nothing; a cue started stopped plays.
        let at = st.pos;
        st.note_on(&p, &rec, CUE_NOTE + 5, 1.0);
        assert_eq!(st.pos, at);
        st.act(DeckAct::StartStop);
        run(&mut st, &p, Some(&s), &rec, 0.2);
        assert!(!st.running());
        st.note_on(&p, &rec, CUE_NOTE + 2, 1.0);
        assert!(st.running());
        // No click: the jump fades over 2 ms.
        let x = run(&mut st, &p, Some(&s), &rec, 0.1);
        let worst = x.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
        assert!(worst < 0.1, "a step of {worst}");
    }

    #[test]
    fn scratches_keep_the_songs_time() {
        let s = sine(440.0, 30.0);
        let rec = Record::default();
        for (bpm, rate, beats) in [(120.0f32, 1.0, 0.5), (90.0, 1.0, 0.5), (120.0, 2.0, 0.25)] {
            let mut p = plain();
            p[MOVE_RATE] = rate;
            let mut st = TurntableState::new(SR);
            st.pos = 10.0 * SR as f64;
            st.set_transport(bpm, 0.0, true);
            st.note_on(&p, &rec, MOVE_NOTE, 1.0);
            // The record's way: where it turns from forward to back.
            let mut turns = Vec::new();
            let mut last = 0.0;
            let mut t = 0usize;
            for _ in 0..(3.0 * SR / 64.0) as usize {
                let (mut l, mut r) = ([0.0f32; 64], [0.0f32; 64]);
                st.render(&p, Some(&s), &rec, &mut l, &mut r);
                t += 64;
                let v = st.speed();
                if last > 0.0 && v <= 0.0 {
                    turns.push(t as f64 / SR as f64);
                }
                last = v;
            }
            let period = beats * 60.0 / bpm as f64;
            let gaps: Vec<f64> = turns.windows(2).map(|w| w[1] - w[0]).collect();
            assert!(gaps.len() >= 3, "{bpm}: {turns:?}");
            for g in &gaps {
                assert!((g - period).abs() < 0.004, "at {bpm} BPM a baby scratch turns every {period} s, not {g}");
            }
        }
        // The forward scratch: the fader open forward only.
        let p = plain();
        let mut st = TurntableState::new(SR);
        st.pos = 10.0 * SR as f64;
        st.set_transport(120.0, 0.0, true);
        st.note_on(&p, &rec, MOVE_NOTE + 1, 1.0);
        let mut open_back = 0;
        let mut open_fwd = 0;
        for _ in 0..(2.0 * SR / 32.0) as usize {
            let (mut l, mut r) = ([0.0f32; 32], [0.0f32; 32]);
            st.render(&p, Some(&s), &rec, &mut l, &mut r);
            if st.gain > 0.5 {
                if st.speed() < -0.3 {
                    open_back += 1;
                } else if st.speed() > 0.3 {
                    open_fwd += 1;
                }
            }
        }
        assert!(open_fwd > 100 && open_back < open_fwd / 10, "{open_fwd} {open_back}");
        st.note_off(&p, MOVE_NOTE + 1);
        assert!(st.moving.is_none());
    }

    #[test]
    fn a_loop_roll_comes_back_where_the_record_would_be() {
        let s = sine(440.0, 30.0);
        let rec = Record { bpm: 120.0, beat: 0.0, heard: true, cues: [None; CUES] };
        let mut p = plain();
        p[LOOP] = 2.0;
        let mut st = TurntableState::new(SR);
        st.act(DeckAct::StartStop);
        run(&mut st, &p, Some(&s), &rec, 1.2);
        st.note_on(&p, &rec, ROLL_NOTE, 1.0);
        let lp = st.looping.unwrap();
        assert!((lp.start - 1.0 * SR as f64).abs() < 1.0 && (lp.len - 0.5 * SR as f64).abs() < 1.0, "{lp:?}");
        let before = st.position();
        run(&mut st, &p, Some(&s), &rec, 2.0);
        assert!(st.position() >= 1.0 && st.position() < 1.5 + 1e-3, "looping: {}", st.position());
        st.note_off(&p, ROLL_NOTE);
        assert!(st.looping.is_none());
        assert!((st.position() - (before + 2.0)).abs() < 0.005, "back at {} for {}", st.position(), before + 2.0);
    }

    #[test]
    fn nothing_blows_up() {
        let s = sine(300.0, 4.0);
        let rec = Record { bpm: 97.0, beat: 0.1, heard: true, cues: [Some(0.5); CUES] };
        let mut rng = Rng::new(9);
        for round in 0..12 {
            let mut p: Vec<f32> = SPECS.iter().map(|s| s.from_norm(rng.unipolar())).collect();
            p[VOLUME] = 0.0;
            let mut st = TurntableState::new(SR);
            st.set_transport(60.0 + rng.unipolar() * 140.0, 0.0, round % 2 == 0);
            let mut turns = 0.0;
            for k in 0..300 {
                match k % 37 {
                    0 => st.act(DeckAct::StartStop),
                    5 => st.note_on(&p, &rec, 36 + (rng.next_u32() % 19) as u8, 1.0),
                    9 => st.act(DeckAct::Backspin),
                    13 => st.act(DeckAct::Jump { secs: rng.unipolar() as f64 * 6.0 - 1.0, play: true }),
                    17 => st.act(DeckAct::Loop(true)),
                    23 => st.act(DeckAct::Loop(false)),
                    _ => {}
                }
                if k % 3 == 0 {
                    turns += rng.bipolar() as f64 * 0.3;
                    st.act(DeckAct::Hand(if k % 50 < 25 { Some(turns) } else { None }));
                }
                let (mut l, mut r) = ([0.0f32; 128], [0.0f32; 128]);
                st.render(&p, Some(&s), &rec, &mut l, &mut r);
                assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 8.0), "round {round}");
            }
        }
    }

    #[test]
    fn every_preset_plays_a_record_in_bounds() {
        let s = sine(220.0, 6.0);
        let rec = Record::default();
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let mut st = TurntableState::new(SR);
            st.set_transport(120.0, 0.0, true);
            // Those that start with the song are started by it.
            if p[SONG] < 0.5 {
                st.act(DeckAct::StartStop);
            }
            let x = run(&mut st, &p, Some(&s), &rec, 3.0);
            assert!(x.iter().all(|v| v.is_finite()), "{name}");
            assert!(peak(&x[x.len() / 2..]) > 0.05, "{name} plays: {}", peak(&x));
            assert!(peak(&x) < 2.0, "{name}");
        }
    }
}
