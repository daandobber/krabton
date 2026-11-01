//! The flip as Krabton 0.90 played it (when it was called Princess Flip),
//! kept as it was for the tests: a set made then must sound the same now
//! (see `princess_flip`'s tests).

#![allow(dead_code)]

use super::fry::{Bus, BusKnobs};
use super::reverb::Reverb;
use super::{db_to_gain, decay_coef, FilterMode, OnePole, Rng, Svf};
use crate::params::{ParamSpec, Unit};
use crate::sample::{SampleData, SampleRef};
use crate::scale::{Scale, ScaleKind};
use serde::{Deserialize, Serialize};

pub const PITCH: usize = 0;
pub const SLOWED: usize = 1;
pub const SONG_KEY: usize = 2;
pub const HALF: usize = 3;
pub const MODE: usize = 4;
pub const CHOKE: usize = 5;
pub const STUTTER: usize = 6;
pub const ATTACK: usize = 7;
pub const RELEASE: usize = 8;
pub const WOBBLE: usize = 9;
pub const LOW_PASS: usize = 10;
pub const HISS: usize = 11;
pub const FRY: usize = 12;
pub const REVERB: usize = 13;
pub const VOLUME: usize = 14;
pub const CHOP: usize = 15;

pub const MODES: &[&str] = &["One-Shot", "Gate"];
pub const STUTTERS: &[&str] = &["Off", "1/8", "1/16", "1/32", "1/64"];
const STUTTER_BEATS: [f64; 5] = [0.0, 0.5, 0.25, 0.125, 0.0625];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Pitch", "Flip", -12.0, 12.0, 0.0, Unit::Semi),
    ParamSpec::toggle("Slowed + Reverb", "Flip", false),
    ParamSpec::toggle("Song Key", "Flip", true),
    ParamSpec::toggle("Half-Time", "Flip", false),
    ParamSpec::choice("Mode", "Chops", MODES, 0),
    ParamSpec::toggle("Choke", "Chops", true),
    ParamSpec::choice("Stutter", "Chops", STUTTERS, 0),
    ParamSpec::log("Attack", "Chops", 0.5, 200.0, 1.0, Unit::Ms),
    ParamSpec::log("Release", "Chops", 5.0, 2000.0, 40.0, Unit::Ms),
    ParamSpec::lin("Wobble", "Tape", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Low Pass", "Tape", 200.0, 20_000.0, 20_000.0, Unit::Hz),
    ParamSpec::lin("Hiss", "Tape", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Fry", "Fry", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Reverb", "Fry", 0.0, 1.0, 0.12, Unit::Percent),
    ParamSpec::lin("Volume", "Fry", -30.0, 6.0, -3.0, Unit::Db),
    ParamSpec::int("Chop", "Chops", 1.0, 16.0, 1.0, Unit::None),
];

/// The first chop's key (C3); the others go up from it.
pub const FIRST_NOTE: u8 = 60;
/// The most chops a flip has, and the fewest it is cut in.
pub const MAX_CHOPS: usize = 16;
pub const MIN_CHOPS: usize = 8;
/// The keys that play the chosen chop at their pitch (C1 to B2), and the
/// one that plays it as it is (C2).
pub const LOW_KEYS: std::ops::RangeInclusive<u8> = 36..=59;
pub const LOW_ROOT: u8 = 48;
/// How much slower Slowed + Reverb plays (about three semitones down).
const SLOWED_SPEED: f64 = 0.84;
const VOICES: usize = 8;
const BLOCK: usize = 128;

/// A part of the sample a key plays: frames `start..end`, maybe backwards.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlipChop {
    pub start: u64,
    pub end: u64,
    #[serde(default)]
    pub reverse: bool,
}

/// What a flip knows of its sample: its chops, its key (root, minor) and
/// tempo as they were found (saved with the set; the sample is the
/// instrument's own).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Flip {
    #[serde(default)]
    pub chops: Vec<FlipChop>,
    #[serde(default)]
    pub key: Option<(u8, bool)>,
    #[serde(default)]
    pub bpm: Option<f64>,
}

/// How a sample is chopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChopBy {
    /// At its hits: the strongest, in order.
    Hits,
    /// In bars (of its tempo, from its first downbeat), or halves and
    /// beats of them for a short one; equal parts without a tempo.
    Bars,
}

impl Flip {
    pub fn key_name(&self) -> Option<String> {
        self.key.map(|(root, minor)| crate::analysis::Key { root, minor, confidence: 1.0 }.name())
    }
}

/// Listens to a sample and chops it into `count` (8 to 16) chops. Takes a
/// moment for a whole song, so never on the audio thread.
pub fn listen(s: &SampleData, by: ChopBy, count: usize) -> Flip {
    let key = crate::analysis::key_in_name(&s.name).or_else(|| crate::analysis::detect_key(s)).map(|k| (k.root, k.minor));
    let tempo = s.analysis().tempo.filter(|t| t.confidence > 0.2);
    let bpm = crate::analysis::bpm_in_name(&s.name).or(tempo.map(|t| t.bpm));
    Flip { chops: chop(s, by, count, bpm, tempo.map_or(0.0, |t| t.downbeat)), key, bpm }
}

/// The chops of a sample (see [`listen`]): `downbeat` (s) is where its bars
/// start.
pub fn chop(s: &SampleData, by: ChopBy, count: usize, bpm: Option<f64>, downbeat: f64) -> Vec<FlipChop> {
    let n = s.frames();
    let count = count.clamp(1, MAX_CHOPS);
    let sr = s.sample_rate as f64;
    if n < 64 {
        return Vec::new();
    }
    let mut starts: Vec<usize> = match (by, bpm) {
        (ChopBy::Hits, _) => {
            let an = s.analysis();
            let mut hits: Vec<(usize, f32)> = an.transients.iter().copied().zip(an.strengths.iter().copied().chain(std::iter::repeat(1.0))).collect();
            if hits.len() > count {
                hits.sort_by(|a, b| b.1.total_cmp(&a.1));
                hits.truncate(count);
            }
            // A millisecond before each, so the hit is whole.
            let pre = (0.001 * sr) as usize;
            let mut v: Vec<usize> = hits.into_iter().map(|(h, _)| h.saturating_sub(pre)).collect();
            v.sort_unstable();
            // What comes before the first hit is a chop too.
            if v.first().is_some_and(|&f| f as f64 > 0.01 * sr) && v.len() < count {
                v.insert(0, 0);
            }
            if v.is_empty() {
                (0..count).map(|k| k * n / count).collect()
            } else {
                // Few hits (a pad, a held chord): the longest chops halved
                // till there are at least eight.
                while v.len() < MIN_CHOPS.min(count) {
                    let ends: Vec<usize> = v.iter().skip(1).copied().chain(std::iter::once(n)).collect();
                    let Some((k, len)) = v.iter().zip(&ends).map(|(a, b)| b - a).enumerate().max_by_key(|(_, len)| *len) else { break };
                    if len < (0.05 * sr) as usize {
                        break;
                    }
                    v.insert(k + 1, v[k] + len / 2);
                }
                v
            }
        }
        (ChopBy::Bars, Some(bpm)) if bpm > 0.0 => {
            // Bars, or halves, beats, half beats of them till there are enough.
            let beat = 60.0 / bpm * sr;
            let first = ((downbeat.max(0.0) * sr) as usize).min(n - 1);
            let mut unit = 4.0 * beat;
            while ((n - first) as f64 / unit).floor() < count as f64 && unit > beat * 0.5 + 1.0 {
                unit *= 0.5;
            }
            let fit = (((n - first) as f64 / unit).floor() as usize).clamp(1, count);
            (0..fit).map(|k| first + (k as f64 * unit) as usize).collect()
        }
        _ => (0..count).map(|k| k * n / count).collect(),
    };
    starts.dedup();
    let unit_end = |k: usize, starts: &[usize]| starts.get(k + 1).copied().unwrap_or(n);
    let bars = by == ChopBy::Bars;
    (0..starts.len())
        .map(|k| {
            let end = if bars && bpm.is_some() && k + 1 == starts.len() {
                // The last bar as long as the others.
                let len = if k > 0 { starts[k] - starts[k - 1] } else { n - starts[k] };
                (starts[k] + len).min(n)
            } else {
                unit_end(k, &starts)
            };
            FlipChop { start: starts[k] as u64, end: end as u64, reverse: false }
        })
        .filter(|c| c.end > c.start + 32)
        .collect()
}

/// How far a sample in `from` (root, minor) moves to play in `to`: the
/// difference of their roots (of the relative keys when one is minor and
/// the other not), the nearer way round (-6 to +5).
pub fn key_shift(from: (u8, bool), to: Scale) -> i32 {
    let minor = matches!(to.kind, ScaleKind::Minor | ScaleKind::Dorian | ScaleKind::Phrygian | ScaleKind::Locrian | ScaleKind::HarmonicMinor | ScaleKind::MelodicMinor | ScaleKind::MinorPentatonic | ScaleKind::HungarianMinor | ScaleKind::Blues | ScaleKind::Hirajoshi | ScaleKind::InSen);
    // Both as their major (a minor key's relative major is three up).
    let a = from.0 as i32 + if from.1 { 3 } else { 0 };
    let b = to.root as i32 + if minor { 3 } else { 0 };
    let d = (b - a).rem_euclid(12);
    if d > 5 {
        d - 12
    } else {
        d
    }
}

pub fn preset_names() -> &'static [&'static str] {
    &["Init", "Plugg Flip", "Slowed Soul", "Half-Time Dream", "Deep Fried Flip", "Tape Warble", "Stutter Chops", "Anime Lo-Fi", "Gated Chops"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Plugg Flip" => &[(PITCH, -2.0), (REVERB, 0.22), (LOW_PASS, 9000.0), (FRY, 0.12)],
        "Slowed Soul" => &[(SLOWED, 1.0), (WOBBLE, 0.25), (LOW_PASS, 7000.0), (HISS, 0.25)],
        "Half-Time Dream" => &[(HALF, 1.0), (REVERB, 0.4), (RELEASE, 400.0), (ATTACK, 20.0), (LOW_PASS, 6000.0)],
        "Deep Fried Flip" => &[(FRY, 0.8), (PITCH, -1.0), (REVERB, 0.05)],
        "Tape Warble" => &[(WOBBLE, 0.7), (HISS, 0.4), (LOW_PASS, 5000.0), (PITCH, -1.0)],
        "Stutter Chops" => &[(STUTTER, 2.0), (MODE, 1.0), (FRY, 0.2)],
        "Anime Lo-Fi" => &[(PITCH, -3.0), (LOW_PASS, 3500.0), (HISS, 0.5), (WOBBLE, 0.35), (REVERB, 0.25)],
        "Gated Chops" => &[(MODE, 1.0), (RELEASE, 15.0)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

/// A chop as it sounds.
#[derive(Clone, Copy, Debug, Default)]
struct Voice {
    active: bool,
    note: u8,
    chop: usize,
    /// Its part of the sample (frames) and whether backwards.
    a: f64,
    b: f64,
    reverse: bool,
    /// Where it is (frames), its speed (the key's, the note's).
    pos: f64,
    ratio: f64,
    env: f32,
    attack: f32,
    release: f32,
    held: bool,
    gain: f32,
    /// Output samples since it (re)started, for the stutter.
    since: u64,
    /// The half-time grains: how far the first is (0..1), where each began.
    gph: f32,
    gfrom: [f64; 2],
    last: [f32; 2],
    tail: [f32; 2],
    tail_left: u32,
    age: u64,
}

pub struct FlipState {
    sr: f32,
    pub bpm: f32,
    /// The song's key (Song Key moves the chops into it).
    pub key: Scale,
    voices: [Voice; VOICES],
    pending: [Option<(u8, f32)>; 32],
    releases: [Option<u8>; 32],
    age: u64,
    rng: Rng,
    wow: f32,
    flutter: f32,
    drift: f32,
    drift_to: f32,
    lp: [Svf; 2],
    lp_on: bool,
    hiss_lp: OnePole,
    hiss_hp: OnePole,
    crackle: f32,
    /// How much it is playing (for the hiss: on with a chop, fading after).
    act: f32,
    bus: Box<Bus>,
    room: Box<Reverb>,
    buf: Box<[[f32; BLOCK]; 2]>,
    lit: [f32; MAX_CHOPS],
    /// Where the last chop played is in the sample (0..1).
    at: f32,
    shift: i32,
}

impl FlipState {
    pub fn new(sr: f32) -> Self {
        let mut hiss_lp = OnePole::default();
        hiss_lp.set(6000.0, sr);
        let mut hiss_hp = OnePole::default();
        hiss_hp.set(400.0, sr);
        Self {
            sr,
            bpm: 120.0,
            key: Scale::new(0, ScaleKind::Minor),
            voices: [Voice::default(); VOICES],
            pending: [None; 32],
            releases: [None; 32],
            age: 0,
            rng: Rng::new(0xF11F_0042),
            wow: 0.0,
            flutter: 0.0,
            drift: 0.0,
            drift_to: 0.0,
            lp: [Svf::new(); 2],
            lp_on: false,
            hiss_lp,
            hiss_hp,
            crackle: 0.0,
            act: 0.0,
            bus: Box::new(Bus::new(sr)),
            room: Box::new(Reverb::new(sr)),
            buf: Box::new([[0.0; BLOCK]; 2]),
            lit: [0.0; MAX_CHOPS],
            at: -1.0,
            shift: 0,
        }
    }

    pub fn set_transport(&mut self, bpm: f32) {
        self.bpm = bpm.clamp(20.0, 999.0);
    }

    pub fn note_on(&mut self, note: u8, vel: f32) {
        if let Some(slot) = self.pending.iter_mut().find(|s| s.is_none()) {
            *slot = Some((note, vel));
        }
    }

    pub fn note_off(&mut self, note: u8) {
        if let Some(slot) = self.releases.iter_mut().find(|s| s.is_none()) {
            *slot = Some(note);
        }
    }

    pub fn all_off(&mut self) {
        self.pending = [None; 32];
        for v in self.voices.iter_mut().filter(|v| v.active) {
            v.held = false;
            v.release = v.release.max(1.0 / (0.01 * self.sr));
        }
    }

    /// For the face: how lit each chop is, where the last one played is in
    /// the sample (-1: none), the key's move, the voices playing.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[..MAX_CHOPS].copy_from_slice(&self.lit);
        out[16] = self.at;
        out[17] = self.shift as f32;
        out[18] = self.voices.iter().filter(|v| v.active).count() as f32;
    }

    /// The semitones the key moves everything by now.
    pub fn shift(&self, p: &[f32], flip: &Flip) -> i32 {
        match flip.key {
            Some(k) if p.get(SONG_KEY).is_some_and(|v| *v >= 0.5) && self.key.kind != ScaleKind::Chromatic => key_shift(k, self.key),
            _ => 0,
        }
    }

    fn start(&mut self, p: &[f32], flip: &Flip, n: usize, note: u8, vel: f32) {
        let (chop, semis) = if note >= FIRST_NOTE && ((note - FIRST_NOTE) as usize) < flip.chops.len() {
            ((note - FIRST_NOTE) as usize, 0.0)
        } else if LOW_KEYS.contains(&note) && !flip.chops.is_empty() {
            let c = (p[CHOP].round().max(1.0) as usize - 1).min(flip.chops.len() - 1);
            (c, note as f32 - LOW_ROOT as f32)
        } else {
            return;
        };
        let c = flip.chops[chop];
        let (a, b) = ((c.start as usize).min(n) as f64, (c.end as usize).min(n) as f64);
        if b - a < 4.0 {
            return;
        }
        let sr = self.sr;
        if p[CHOKE] >= 0.5 {
            for v in self.voices.iter_mut().filter(|v| v.active) {
                v.held = false;
                v.release = v.release.max(1.0 / (0.005 * sr));
            }
        }
        let slot = self.voices.iter().position(|v| !v.active).unwrap_or_else(|| (0..VOICES).min_by_key(|&i| self.voices[i].age).unwrap_or(0));
        self.age += 1;
        let shift = self.shift as f32;
        let pos = if c.reverse { b - 1.0 } else { a };
        let attack = 1.0 / (p[ATTACK].clamp(0.5, 200.0) * 0.001 * sr).max(1.0);
        let old = self.voices[slot];
        self.voices[slot] = Voice {
            active: true,
            note,
            chop,
            a,
            b,
            reverse: c.reverse,
            pos,
            ratio: 2f64.powf((semis + shift) as f64 / 12.0),
            env: 0.0,
            attack,
            release: 0.0,
            held: true,
            gain: 0.35 + 0.65 * vel.clamp(0.0, 1.0),
            since: 0,
            gph: 0.0,
            gfrom: [pos, pos],
            last: [0.0; 2],
            // A voice taken while it sounded fades out under the new one.
            tail: if old.active { old.last } else { [0.0; 2] },
            tail_left: if old.active { (0.003 * sr) as u32 } else { 0 },
            age: self.age,
        };
        self.lit[chop.min(MAX_CHOPS - 1)] = 1.0;
    }

    pub fn render(&mut self, p: &[f32], sample: Option<&SampleRef>, flip: &Flip, l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let mut pos = 0;
        while pos < n {
            let len = (n - pos).min(BLOCK);
            self.render_block(p, sample, flip, &mut l[pos..pos + len], &mut r[pos..pos + len]);
            pos += len;
        }
    }

    fn render_block(&mut self, p: &[f32], sample: Option<&SampleRef>, flip: &Flip, l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len()).min(BLOCK);
        let sr = self.sr;
        self.shift = self.shift(p, flip);
        let frames = sample.map_or(0, |s| s.frames());
        // Notes from the block's start.
        for i in 0..self.pending.len() {
            let Some((note, vel)) = self.pending[i].take() else { continue };
            if frames > 0 {
                self.start(p, flip, frames, note, vel);
            }
        }
        let gate = p[MODE] >= 0.5;
        let release = 1.0 / (p[RELEASE].clamp(5.0, 2000.0) * 0.001 * sr).max(1.0);
        for i in 0..self.releases.len() {
            let Some(note) = self.releases[i].take() else { continue };
            for v in self.voices.iter_mut().filter(|v| v.active && v.note == note) {
                v.held = false;
                if gate {
                    v.release = v.release.max(release);
                }
            }
        }
        let [bl, br] = &mut *self.buf;
        bl[..n].fill(0.0);
        br[..n].fill(0.0);
        // The tape: how fast it runs now.
        let wobble = p[WOBBLE].clamp(0.0, 1.0);
        let slowed = p[SLOWED] >= 0.5;
        let half = p[HALF] >= 0.5;
        let base = 2f64.powf(p[PITCH].clamp(-12.0, 12.0) as f64 / 12.0) * if slowed { SLOWED_SPEED } else { 1.0 };
        let stutter = STUTTER_BEATS[(p[STUTTER].round().max(0.0) as usize).min(STUTTER_BEATS.len() - 1)];
        let stutter_len = (stutter * 60.0 / self.bpm.max(1.0) as f64 * sr as f64) as u64;
        let lp_hz = p[LOW_PASS].clamp(200.0, 20_000.0);
        let lp_on = lp_hz < 19_000.0;
        if lp_on {
            if !self.lp_on {
                self.lp = [Svf::new(); 2];
            }
            for f in self.lp.iter_mut() {
                f.set(lp_hz, 0.1, sr);
            }
        }
        self.lp_on = lp_on;
        let hiss = p[HISS].clamp(0.0, 1.0);
        let act_fall = decay_coef(1.5, sr);
        let grain = (0.07 * sr).max(16.0);
        let mut last_at = None;
        if let Some(s) = sample.filter(|_| frames > 1) {
            let sd: &SampleData = s;
            let rate = sd.sample_rate as f64 / sr as f64;
            for i in 0..n {
                // Wow (a slow drift and a sway) and flutter, in cents.
                self.wow = (self.wow + 0.55 / sr).fract();
                self.flutter = (self.flutter + 6.3 / sr).fract();
                if self.rng.next_u32() % 4096 == 0 {
                    self.drift_to = self.rng.bipolar();
                }
                self.drift += (self.drift_to - self.drift) * 0.0002;
                let cents = wobble * (16.0 * (self.wow * std::f32::consts::TAU).sin() + 5.0 * (self.flutter * std::f32::consts::TAU).sin() + 10.0 * self.drift);
                let speed = base * 2f64.powf(cents as f64 / 1200.0);
                let (mut sl, mut sr_) = (0.0f32, 0.0f32);
                for v in self.voices.iter_mut().filter(|v| v.active) {
                    let step = v.ratio * speed * rate;
                    let rev = v.reverse;
                    let dir = if rev { -1.0 } else { 1.0 };
                    let (a, b) = (v.a, v.b);
                    if v.since == 0 {
                        // The grains from the chop's start: the second one, half
                        // way through its own, reads from there now.
                        v.gph = 0.0;
                        v.gfrom = [v.pos, v.pos - dir * 0.5 * grain as f64 * step];
                    }
                    // In over 2 ms at the chop's start, out over 4 at its end.
                    let (fin, fout) = (0.002 * sd.sample_rate as f64, 0.004 * sd.sample_rate as f64);
                    let read = |at: f64| -> [f32; 2] {
                        if at < a || at > b - 1.0 {
                            return [0.0; 2];
                        }
                        let (from_start, to_end) = if rev { (b - 1.0 - at, at - a) } else { (at - a, b - 1.0 - at) };
                        let e = ((from_start / fin).min(to_end / fout)).min(1.0) as f32;
                        let (x, y) = sd.read(at);
                        [x * e, y * e]
                    };
                    let y = if half {
                        // Half-time: grains read at the chop's pitch, from a
                        // place moving at half its speed.
                        let before = v.gph;
                        v.gph = (v.gph + 1.0 / grain).fract();
                        let mut y = [0.0f32; 2];
                        for k in 0..2 {
                            let (age, was) = ((v.gph + 0.5 * k as f32).fract(), (before + 0.5 * k as f32).fract());
                            if age < was {
                                v.gfrom[k] = v.pos;
                            }
                            let at = v.gfrom[k] + dir * age as f64 * grain as f64 * step;
                            let w = (std::f32::consts::PI * age).sin();
                            let s = read(at);
                            y[0] += s[0] * w * w;
                            y[1] += s[1] * w * w;
                        }
                        v.pos += dir * step * 0.5;
                        y
                    } else {
                        let y = read(v.pos);
                        v.pos += dir * step;
                        y
                    };
                    // The envelope: in, held, out.
                    if v.release > 0.0 {
                        v.env -= v.release;
                    } else if v.env < 1.0 {
                        v.env = (v.env + v.attack).min(1.0);
                    }
                    let g = v.env.max(0.0) * v.gain;
                    let mut out = [y[0] * g, y[1] * g];
                    if v.tail_left > 0 {
                        let k = v.tail_left as f32 / (0.003 * sr);
                        out[0] += v.tail[0] * k;
                        out[1] += v.tail[1] * k;
                        v.tail_left -= 1;
                    }
                    v.last = out;
                    sl += out[0];
                    sr_ += out[1];
                    v.since += 1;
                    // A held chop again from its start, every so often.
                    if stutter_len > 0 && v.held && v.since >= stutter_len {
                        v.since = 0;
                        v.tail = v.last;
                        v.tail_left = (0.003 * sr) as u32;
                        v.pos = if v.reverse { b - 1.0 } else { a };
                    }
                    let ended = if v.reverse { v.pos < a } else { v.pos > b - 1.0 };
                    if ended && v.tail_left == 0 || v.env <= 0.0 && v.release > 0.0 {
                        v.active = false;
                    }
                    last_at = Some((v.age, v.pos / frames as f64));
                }
                bl[i] = sl;
                br[i] = sr_;
            }
        }
        if let Some((_, at)) = last_at {
            self.at = at as f32;
        } else if self.voices.iter().all(|v| !v.active) {
            self.at = -1.0;
        }
        // The tape's tone and its hiss, while it plays and a moment after.
        let playing = self.voices.iter().any(|v| v.active);
        for i in 0..n {
            if lp_on {
                bl[i] = self.lp[0].process(bl[i], FilterMode::Low);
                br[i] = self.lp[1].process(br[i], FilterMode::Low);
            }
            self.act = if playing { 1.0 } else { self.act * act_fall };
            if hiss > 0.0 && self.act > 1e-3 {
                let w = self.rng.bipolar();
                let h = self.hiss_hp.hp(self.hiss_lp.lp(w));
                if self.rng.next_u32() % 9000 == 0 {
                    self.crackle = self.rng.bipolar() * 0.6;
                }
                self.crackle *= 0.7;
                let x = (h * 0.05 + self.crackle * 0.25) * hiss * self.act;
                bl[i] += x;
                br[i] += x * 0.9 + self.crackle * 0.02;
            }
        }
        // The FRY section, turned up together.
        let fry = p[FRY].clamp(0.0, 1.0);
        if fry > 0.001 {
            let bits = (16.0 - fry * 9.0).clamp(2.0, 16.0);
            let down = 1.0 + fry * 4.0;
            let k = BusKnobs {
                drive: fry * 0.6,
                steps: 2f32.powf(bits - 1.0),
                rate: 1.0 / down,
                crush: true,
                ring: fry * 0.15,
                ring_inc: 180.0 / sr,
                push: 1.0 + 4.0 * fry,
                hard: fry > 0.5,
                ceiling: db_to_gain(-0.3),
                volume: 1.0,
                lim_rel: 1.0 - (-1.0 / (0.08 * sr)).exp(),
                ..BusKnobs::default()
            };
            for i in 0..n {
                let y = self.bus.fry([bl[i], br[i]], &k);
                let y = self.bus.tick(y, &k);
                bl[i] = y[0];
                br[i] = y[1];
            }
        }
        // The room: more of it slowed.
        let verb = (p[REVERB].clamp(0.0, 1.0) + if slowed { 0.3 } else { 0.0 }).min(1.0);
        if verb > 0.001 {
            let size = if slowed { 0.92 } else { 0.7 };
            self.room.process(size, 0.55, 1.0, 12.0, 150.0, verb * 0.6, &mut bl[..n], &mut br[..n]);
        }
        let vol = db_to_gain(p[VOLUME].clamp(-30.0, 6.0));
        for i in 0..n {
            let (a, b) = (bl[i] * vol, br[i] * vol);
            if a.is_finite() && b.is_finite() {
                l[i] += a;
                r[i] += b;
            }
        }
        let fade = (-(n as f32) / (0.15 * sr)).exp();
        self.lit.iter_mut().for_each(|x| *x *= fade);
        // A chop still sounding stays lit.
        for v in self.voices.iter().filter(|v| v.active) {
            let x = &mut self.lit[v.chop.min(MAX_CHOPS - 1)];
            *x = x.max(0.6 * v.env.clamp(0.0, 1.0));
        }
    }
}

