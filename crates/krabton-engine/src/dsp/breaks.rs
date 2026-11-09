//! Breaks: a break (a drum loop: your own, or one of its own) cut on its
//! sixteenths (or thirty-seconds), each cut moved onto the hit nearest it,
//! and played again on the song's grid as it plays, the way breakcore and
//! drill'n'bass cut breaks up: cuts swapped for others (*Shuffle*), played
//! again (*Repeat*), rolled into 2 to 8 hits in the time of one (*Ratchet*)
//! rising or falling as they go (*Ramp*), backwards (*Reverse*), pitched up
//! or down (*Pitch Jumps*: the snare runs), at half speed (*Half*), a roll
//! that speeds up to close every 2, 4 or 8 bars (*Fill*); the dice thrown
//! again every bar, 2 or 4 (*Mutate*) or never, from a *Seed*. Each cut
//! plays at its own speed, whatever the song's tempo, cut off at the next
//! (or sooner, *Gate*); *Swing*, the break's *Pitch*, *Tone* and *Level*.
//! It plays by itself while the song plays; a held key moves it up or down
//! from C3.
//!
//! `plan` says what a bar plays (the face shows it); `BreaksState` plays it.

use crate::params::{ParamSpec, Unit};
use crate::sample::{SampleData, SampleRef};

pub const BARS: usize = 0;
pub const GRID: usize = 1;
pub const SHUFFLE: usize = 2;
pub const REPEAT: usize = 3;
pub const RATCHET: usize = 4;
pub const RATCHET_MAX: usize = 5;
pub const RAMP: usize = 6;
pub const REVERSE: usize = 7;
pub const JUMPS: usize = 8;
pub const JUMP_RANGE: usize = 9;
pub const HALF: usize = 10;
pub const FILL: usize = 11;
pub const MUTATE: usize = 12;
pub const SWING: usize = 13;
pub const GATE: usize = 14;
pub const PITCH: usize = 15;
pub const TONE: usize = 16;
pub const LEVEL: usize = 17;
pub const SEED: usize = 18;

pub const BAR_NAMES: &[&str] = &["Auto", "1", "2", "4", "8"];
pub const GRIDS: &[&str] = &["1/16", "1/32"];
pub const RATCHETS: [u8; 5] = [2, 3, 4, 6, 8];
pub const RATCHET_NAMES: &[&str] = &["2", "3", "4", "6", "8"];
pub const FILLS: &[&str] = &["Off", "2 bars", "4 bars", "8 bars"];
pub const MUTATES: &[&str] = &["Never", "Every bar", "2 bars", "4 bars"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Bars", "Break", BAR_NAMES, 0),
    ParamSpec::choice("Grid", "Break", GRIDS, 0),
    ParamSpec::lin("Shuffle", "Cut", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Repeat", "Cut", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Ratchet", "Roll", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Most Hits", "Roll", RATCHET_NAMES, 2),
    ParamSpec::lin("Ramp", "Roll", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Reverse", "Cut", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Pitch Jumps", "Pitch", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Jump Range", "Pitch", 0.0, 24.0, 7.0, Unit::Semi),
    ParamSpec::lin("Half", "Cut", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Fill", "Roll", FILLS, 0),
    ParamSpec::choice("Mutate", "Dice", MUTATES, 0),
    ParamSpec::lin("Swing", "Groove", 0.0, 0.5, 0.0, Unit::Percent),
    ParamSpec::lin("Gate", "Groove", 0.1, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Pitch", "Sound", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::log("Tone", "Sound", 300.0, 20000.0, 20000.0, Unit::Hz),
    ParamSpec::lin("Level", "Sound", -24.0, 12.0, 0.0, Unit::Db),
    ParamSpec::lin("Seed", "Dice", 0.0, 999.0, 1.0, Unit::None),
];

/// Its presets: (name, settings changed from where they start).
const PRESETS: &[(&str, &[(usize, f32)])] = &[
    ("Clean", &[]),
    ("Amen Mash", &[(SHUFFLE, 0.35), (REPEAT, 0.15), (RATCHET, 0.12), (REVERSE, 0.05), (FILL, 2.0), (MUTATE, 2.0)]),
    ("Drill", &[(SHUFFLE, 0.25), (RATCHET, 0.35), (RATCHET_MAX, 4.0), (RAMP, 12.0), (GRID, 1.0), (FILL, 1.0), (MUTATE, 1.0), (JUMPS, 0.1)]),
    ("Breakcore", &[(SHUFFLE, 0.6), (REPEAT, 0.25), (RATCHET, 0.3), (RATCHET_MAX, 4.0), (RAMP, 7.0), (REVERSE, 0.15), (JUMPS, 0.3), (JUMP_RANGE, 12.0), (FILL, 1.0), (MUTATE, 1.0)]),
    ("Snare Runs", &[(JUMPS, 0.55), (JUMP_RANGE, 12.0), (RATCHET, 0.2), (RAMP, 5.0), (MUTATE, 2.0)]),
    ("Half-Time", &[(HALF, 0.6), (SHUFFLE, 0.15), (PITCH, -3.0), (TONE, 6000.0)]),
    ("Stutter", &[(REPEAT, 0.45), (RATCHET, 0.25), (RATCHET_MAX, 2.0), (GATE, 0.55), (MUTATE, 1.0)]),
];

pub fn preset_names() -> &'static [&'static str] {
    const N: [&str; 7] = ["Clean", "Amen Mash", "Drill", "Breakcore", "Snare Runs", "Half-Time", "Stutter"];
    &N
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    if let Some((_, set)) = PRESETS.iter().find(|(n, _)| *n == name) {
        for &(k, v) in *set {
            p[k] = v;
        }
    }
    p
}

/// What one step of a bar plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    /// Which cut (of the whole break), and whether it plays at all (the second half of a Half).
    pub cut: usize,
    pub play: bool,
    /// Hits in the time of the step (1: one), rising so many semitones over them.
    pub ratchet: u8,
    pub ramp: f32,
    pub reverse: bool,
    pub semis: f32,
    /// At half speed, over this step and the next.
    pub half: bool,
}

fn hash(mut x: u32) -> f32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    (x >> 8) as f32 / 16_777_216.0
}

/// Steps in a bar.
pub fn steps_per_bar(p: &[f32]) -> usize {
    if p[GRID] >= 0.5 {
        32
    } else {
        16
    }
}

/// How many bars the break is: as set, or (Auto) 1, 2, 4 or 8, whichever
/// makes its tempo nearest the one heard in it (folded into 100..200), or
/// 140 when none is: a break's own length, whatever the song's tempo.
pub fn bars_of(p: &[f32], sample: &SampleData) -> usize {
    match p[BARS].round() as usize {
        0 => {
            let secs = (sample.frames() as f64 / sample.sample_rate.max(1) as f64).max(1e-3);
            let mut target = sample.analysis_ready().and_then(|a| a.tempo).filter(|t| t.confidence > 0.3 && t.bpm > 20.0).map_or(140.0, |t| t.bpm);
            while target < 100.0 {
                target *= 2.0;
            }
            while target >= 200.0 {
                target /= 2.0;
            }
            let off = |b: usize| (b as f64 * 240.0 / secs / target).log2().abs();
            [1usize, 2, 4, 8].into_iter().min_by(|a, b| off(*a).total_cmp(&off(*b))).unwrap_or(1)
        }
        k => [1, 1, 2, 4, 8][k.min(4)],
    }
}

/// What bar `bar` of the song plays, of a break `bars` bars long.
pub fn plan(p: &[f32], bars: usize, bar: i64) -> Vec<Step> {
    let n = steps_per_bar(p);
    let cuts = n * bars.max(1);
    let seed = p[SEED].round() as u32;
    let every = [0i64, 1, 2, 4][(p[MUTATE].round() as usize).min(3)];
    // The dice: the same until it mutates (never: the same every time round the break).
    let epoch = if every == 0 { 0 } else { bar.div_euclid(every) };
    let within = bar.rem_euclid(bars.max(1) as i64) as usize;
    let fill_every = [0i64, 2, 4, 8][(p[FILL].round() as usize).min(3)];
    let fill = fill_every > 0 && (bar + 1).rem_euclid(fill_every) == 0;
    let roll = |k: usize, salt: u32| hash(seed.wrapping_mul(0x9E37_79B9) ^ (epoch as u32).wrapping_mul(0x85EB_CA6B) ^ (within as u32).wrapping_mul(0xC2B2_AE35) ^ (k as u32).wrapping_mul(0x27D4_EB2F) ^ salt.wrapping_mul(0x1656_67B1));
    let most = RATCHETS.iter().position(|r| *r == RATCHETS[(p[RATCHET_MAX].round() as usize).min(4)]).unwrap_or(2);
    // The snare: the cut on the second beat of the break's first bar.
    let snare = n / 4;
    let mut out: Vec<Step> = Vec::with_capacity(n);
    for k in 0..n {
        let natural = (within * n + k) % cuts;
        let mut s = Step { cut: natural, play: true, ratchet: 1, ramp: 0.0, reverse: false, semis: 0.0, half: false };
        if roll(k, 1) < p[SHUFFLE] {
            s.cut = (roll(k, 2) * cuts as f32) as usize % cuts;
        }
        if k > 0 && roll(k, 3) < p[REPEAT] {
            s.cut = out[k - 1].cut;
        }
        if roll(k, 4) < p[RATCHET] {
            s.ratchet = RATCHETS[(roll(k, 5) * (most + 1) as f32) as usize % (most + 1)];
            s.ramp = p[RAMP];
        }
        s.reverse = roll(k, 6) < p[REVERSE];
        if roll(k, 7) < p[JUMPS] {
            s.semis = ((roll(k, 8) * 2.0 - 1.0) * p[JUMP_RANGE]).round();
        }
        if k % 2 == 0 && k + 1 < n && roll(k, 9) < p[HALF] {
            s.half = true;
        }
        if k > 0 && out[k - 1].half {
            s.play = false;
        }
        // The fill: the last beat rolls faster and faster on the snare, rising.
        if fill && k >= n - n / 4 {
            let into = k - (n - n / 4);
            s.cut = snare % cuts;
            s.ratchet = RATCHETS[(into * 5 / (n / 4).max(1)).min(4)].max(2);
            s.ramp = if p[RAMP] != 0.0 { p[RAMP] } else { 5.0 };
            s.semis = into as f32;
            s.play = true;
            s.half = false;
            s.reverse = false;
        }
        out.push(s);
    }
    out
}

/// Where cut `k` of `cuts` starts and ends in a break of `frames` frames:
/// on the grid, moved onto the hit nearest it (within a quarter of a cut).
pub fn cut_bounds(sample: &SampleData, k: usize, cuts: usize, frames: usize) -> (usize, usize) {
    let len = frames as f64 / cuts.max(1) as f64;
    let snap = |at: f64| -> usize {
        let at = at.round() as usize;
        match sample.analysis_ready() {
            Some(a) if !a.transients.is_empty() => {
                let i = a.transients.partition_point(|&t| t < at);
                let near = [i.checked_sub(1), Some(i)].into_iter().flatten().filter_map(|j| a.transients.get(j)).min_by_key(|t| t.abs_diff(at)).copied();
                match near {
                    Some(t) if (t.abs_diff(at) as f64) < len / 4.0 => t,
                    _ => at,
                }
            }
            _ => at,
        }
    };
    let a = snap(k as f64 * len).min(frames);
    let b = if k + 1 >= cuts { frames } else { snap((k + 1) as f64 * len).min(frames) };
    (a, b.max(a + 1).min(frames.max(a + 1)))
}

/// Its own break, made once: two bars at 170, a busy break in the
/// manner of the classic funk breaks (a kick, a snare with ghost notes, a
/// ride), so it plays the moment it is put on a track.
pub fn builtin() -> &'static SampleRef {
    static B: std::sync::OnceLock<SampleRef> = std::sync::OnceLock::new();
    B.get_or_init(|| {
        let sr = 44100.0f32;
        let bpm = 170.0f32;
        let step = 60.0 / bpm / 4.0;
        let frames = (sr * step * 32.0) as usize;
        let mut l = vec![0.0f32; frames];
        // (step, what: 0 kick, 1 snare, 2 ghost, 3 ride)
        let hits: [(usize, u8); 30] = [
            (0, 0), (2, 3), (2, 0), (4, 1), (6, 3), (7, 2), (9, 2), (10, 0), (10, 3), (12, 1), (14, 3), (15, 2), (0, 3), (8, 3),
            (16, 0), (18, 3), (18, 0), (20, 1), (22, 3), (23, 2), (25, 2), (26, 0), (26, 3), (27, 2), (28, 1), (30, 3), (31, 1), (16, 3), (24, 3), (29, 2),
        ];
        let mut rng = 0x1234_567u32;
        let mut noise = move || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            (rng >> 8) as f32 / 8_388_608.0 - 1.0
        };
        for (at, what) in hits {
            let start = (at as f32 * step * sr) as usize;
            let len = (sr * 0.35) as usize;
            let mut lp = 0.0f32;
            for i in 0..len.min(frames - start) {
                let t = i as f32 / sr;
                let v = match what {
                    0 => (std::f32::consts::TAU * (48.0 + 110.0 * (-t * 40.0).exp()) * t).sin() * (-t * 9.0).exp() * 0.9,
                    1 | 2 => {
                        let n = noise();
                        lp += (n - lp) * 0.45;
                        let body = (std::f32::consts::TAU * 190.0 * t).sin() * (-t * 30.0).exp();
                        let g = if what == 1 { 0.75 } else { 0.25 };
                        (lp * (-t * 16.0).exp() + body * 0.5) * g
                    }
                    _ => {
                        let n = noise();
                        let ring = (std::f32::consts::TAU * 5200.0 * t).sin() * 0.3 + (std::f32::consts::TAU * 7400.0 * t).sin() * 0.2;
                        (n * 0.6 + ring) * (-t * 12.0).exp() * 0.12
                    }
                };
                l[start + i] += v;
            }
        }
        // A little glue, as the records had.
        for v in l.iter_mut() {
            *v = (*v * 1.3).tanh() * 0.8;
        }
        SampleRef::new(SampleData::from_buffers("Breaks' own break", None, sr as u32, l.clone(), l))
    })
}

/// A cut sounding (or fading out, cut off by the next).
#[derive(Clone, Copy, Debug, Default)]
struct Voice {
    on: bool,
    /// Where it reads (frames of the break), how fast (frames a sample, below 0 backwards), where it ends.
    pos: f64,
    rate: f64,
    from: f64,
    to: f64,
    /// Samples it has played, it plays before its gate shuts, and the fade.
    age: usize,
    gate: usize,
    fade: f32,
    fading: bool,
}

/// Breaks playing.
pub struct BreaksState {
    sr: f32,
    bpm: f64,
    beat: f64,
    playing: bool,
    /// The step it is on (song steps), the sub-hit of a ratchet, the bar planned and its plan.
    step: Option<i64>,
    hit: Option<(i64, u8)>,
    planned: Option<i64>,
    plan: Vec<Step>,
    voices: [Voice; 2],
    lp: [f32; 2],
    /// The key held (semitones from C3).
    held: Vec<u8>,
    /// For the face: the step playing, the cut, the bar; how many bars the break is.
    pub shown: (i64, usize, i64),
    bars: usize,
}

impl BreaksState {
    pub fn new(sr: f32) -> Self {
        Self { sr, bpm: 120.0, beat: 0.0, playing: false, step: None, hit: None, planned: None, plan: Vec::with_capacity(32), voices: [Voice::default(); 2], lp: [0.0; 2], held: Vec::with_capacity(16), shown: (-1, 0, 0), bars: 0 }
    }

    /// The song's tempo and place (each block).
    pub fn set_transport(&mut self, bpm: f64, beat: f64, playing: bool) {
        if !playing && self.playing {
            for v in &mut self.voices {
                v.fading = v.on;
            }
            self.step = None;
            self.hit = None;
        }
        self.bpm = bpm.max(1.0);
        self.beat = beat;
        self.playing = playing;
    }

    pub fn note_on(&mut self, pitch: u8) {
        if self.held.len() < self.held.capacity() && !self.held.contains(&pitch) {
            self.held.push(pitch);
        }
    }

    pub fn note_off(&mut self, pitch: u8) {
        self.held.retain(|p| *p != pitch);
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in &mut self.voices {
            v.fading = v.on;
        }
    }

    /// What the face is told: the step playing (-1: none), the cut, the bar, the break's bars, playing.
    pub fn visual(&self, out: &mut [f32]) {
        if out.len() >= 5 {
            out[0] = if self.playing { self.shown.0 as f32 } else { -1.0 };
            out[1] = self.shown.1 as f32;
            out[2] = self.shown.2 as f32;
            out[3] = self.bars as f32;
            out[4] = if self.playing { 1.0 } else { 0.0 };
        }
    }

    fn start(&mut self, s: &Step, sample: &SampleData, cuts: usize, frames: usize, p: &[f32], sub: u8, sub_len: f64) {
        let (a, b) = cut_bounds(sample, s.cut % cuts.max(1), cuts, frames);
        let transpose = self.held.last().map_or(0.0, |k| *k as f32 - 60.0);
        let ramp = if s.ratchet > 1 { s.ramp * sub as f32 / (s.ratchet - 1) as f32 } else { 0.0 };
        let semis = p[PITCH] + s.semis + ramp + transpose;
        let mut rate = 2f64.powf(semis as f64 / 12.0) * sample.sample_rate as f64 / self.sr as f64;
        if s.half {
            rate *= 0.5;
        }
        if s.reverse {
            rate = -rate;
        }
        // The one sounding fades out; this one in.
        let free = if self.voices[0].on && !self.voices[0].fading { 1 } else { 0 };
        for (k, v) in self.voices.iter_mut().enumerate() {
            if k != free && v.on {
                v.fading = true;
            }
        }
        let gate = (sub_len * p[GATE].clamp(0.1, 1.0) as f64 * if s.half { 2.0 } else { 1.0 }).max(64.0) as usize;
        self.voices[free] = Voice { on: true, pos: if s.reverse { b as f64 - 1.0 } else { a as f64 }, rate, from: a as f64, to: b as f64, age: 0, gate, fade: 0.0, fading: false };
    }

    pub fn render(&mut self, p: &[f32], sample: Option<&SampleRef>, l: &mut [f32], r: &mut [f32]) {
        let sample: &SampleData = sample.unwrap_or_else(|| builtin());
        let frames = sample.frames();
        if frames < 2 {
            return;
        }
        let bars = bars_of(p, sample);
        self.bars = bars;
        let n = steps_per_bar(p);
        let cuts = n * bars;
        let beats_per_step = 4.0 / n as f64;
        let step_secs = beats_per_step * 60.0 / self.bpm;
        let step_len = step_secs * self.sr as f64;
        let swing = p[SWING].clamp(0.0, 0.5) as f64;
        let level = 10f32.powf(p[LEVEL] / 20.0);
        let k_lp = 1.0 - (-std::f32::consts::TAU * p[TONE].clamp(20.0, 20000.0) / self.sr).exp();
        let beat_per_sample = self.bpm / 60.0 / self.sr as f64;
        let attack = 1.0 / (0.0015 * self.sr);
        let release = 1.0 / (0.004 * self.sr);
        let right = if sample.right.is_empty() { &sample.left } else { &sample.right };
        for i in 0..l.len() {
            if self.playing {
                let beat = self.beat + i as f64 * beat_per_sample;
                // Swing: every second step late.
                let raw = beat / beats_per_step;
                let step = raw.floor() as i64;
                let into = raw - step as f64;
                let late = if step.rem_euclid(2) == 1 { swing } else { 0.0 };
                if into >= late {
                    let bar = (beat / 4.0).floor() as i64;
                    if self.planned != Some(bar) {
                        self.plan = plan(p, bars, bar);
                        self.planned = Some(bar);
                    }
                    let k = step.rem_euclid(n as i64) as usize;
                    if let Some(s) = self.plan.get(k).copied() {
                        let span = 1.0 - late;
                        let sub = (((into - late) / span) * s.ratchet as f64).floor().clamp(0.0, (s.ratchet - 1) as f64) as u8;
                        if s.play && self.hit != Some((step, sub)) {
                            self.hit = Some((step, sub));
                            self.step = Some(step);
                            self.start(&s, sample, cuts, frames, p, sub, step_len * span / s.ratchet as f64);
                            self.shown = (k as i64, s.cut, bar);
                        }
                    }
                }
            }
            let (mut ol, mut or) = (0.0f32, 0.0f32);
            for v in self.voices.iter_mut().filter(|v| v.on) {
                let idx = v.pos as usize;
                let f = (v.pos - idx as f64) as f32;
                let at = |ch: &[f32]| {
                    let a = ch.get(idx).copied().unwrap_or(0.0);
                    a + (ch.get(idx + 1).copied().unwrap_or(0.0) - a) * f
                };
                if v.fading || v.age >= v.gate {
                    v.fade -= release;
                } else {
                    v.fade = (v.fade + attack).min(1.0);
                }
                if v.fade <= 0.0 && (v.fading || v.age >= v.gate) {
                    v.on = false;
                    continue;
                }
                ol += at(&sample.left) * v.fade;
                or += at(right) * v.fade;
                v.pos += v.rate;
                v.age += 1;
                // Past its end: silence (its cut is over).
                if v.pos >= v.to || v.pos < v.from {
                    v.fading = true;
                    v.pos = v.pos.clamp(v.from, (v.to - 1.0).max(v.from));
                }
            }
            self.lp[0] += (ol - self.lp[0]) * k_lp;
            self.lp[1] += (or - self.lp[1]) * k_lp;
            l[i] += self.lp[0] * level;
            r[i] += self.lp[1] * level;
        }
        self.beat += l.len() as f64 * beat_per_sample;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p() -> Vec<f32> {
        crate::params::defaults(SPECS)
    }

    #[test]
    fn clean_plays_the_break_as_it_is() {
        let p = p();
        let a = plan(&p, 2, 0);
        let b = plan(&p, 2, 1);
        assert_eq!(a.iter().map(|s| s.cut).collect::<Vec<_>>(), (0..16).collect::<Vec<_>>());
        assert_eq!(b.iter().map(|s| s.cut).collect::<Vec<_>>(), (16..32).collect::<Vec<_>>(), "the second bar of the break");
        assert!(a.iter().all(|s| s.play && s.ratchet == 1 && !s.reverse && s.semis == 0.0));
        assert_eq!(plan(&p, 2, 2), a, "round again");
    }

    #[test]
    fn the_dice_change_it_and_keep_it_until_it_mutates() {
        let mut p = p();
        p[SHUFFLE] = 0.8;
        p[RATCHET] = 0.5;
        let a = plan(&p, 1, 0);
        assert!(a.iter().filter(|s| s.cut != 0 && s.ratchet == 1).count() > 3 || a.iter().any(|s| s.ratchet > 1));
        assert_eq!(plan(&p, 1, 0), a, "the same dice, the same bar");
        assert_eq!(plan(&p, 1, 5), a, "Never: every bar of a one-bar break the same");
        p[MUTATE] = 1.0;
        assert_ne!(plan(&p, 1, 0), plan(&p, 1, 1), "every bar: another");
        p[SEED] = 7.0;
        assert_ne!(plan(&p, 1, 0), a, "another seed");
        assert!(a.iter().filter(|s| s.ratchet > 1).all(|s| RATCHETS[..3].contains(&s.ratchet)), "no more hits than Most Hits (4)");
    }

    #[test]
    fn a_fill_rolls_faster_on_the_snare_at_the_end() {
        let mut p = p();
        p[FILL] = 2.0; // every 4 bars
        assert!(plan(&p, 1, 0).iter().all(|s| s.ratchet == 1), "not yet");
        let fill = plan(&p, 1, 3);
        let last: Vec<u8> = fill[12..].iter().map(|s| s.ratchet).collect();
        assert!(last.windows(2).all(|w| w[1] >= w[0]) && last[0] >= 2, "faster and faster: {last:?}");
        assert!(fill[12..].iter().all(|s| s.cut == 4), "on the snare (beat two)");
        // Half: a step at half speed over two.
        let mut p = super::tests::p();
        p[HALF] = 1.0;
        let h = plan(&p, 1, 0);
        assert!(h[0].half && !h[1].play);
    }

    #[test]
    fn it_plays_the_break_while_the_song_plays_and_rolls_with_ratchets() {
        let b = builtin();
        assert!(b.frames() > 40000);
        assert_eq!(bars_of(&p(), b), 2, "two bars at 170");
        let mut st = BreaksState::new(48000.0);
        let p = p();
        let (mut l, mut r) = (vec![0.0f32; 48000], vec![0.0f32; 48000]);
        st.set_transport(170.0, 0.0, false);
        st.render(&p, None, &mut l, &mut r);
        assert!(l.iter().all(|v| *v == 0.0), "stopped: nothing");
        st.set_transport(170.0, 0.0, true);
        let onsets = |p: &[f32]| {
            let mut st = BreaksState::new(48000.0);
            st.set_transport(170.0, 0.0, true);
            let (mut l, mut r) = (vec![0.0f32; 48000], vec![0.0f32; 48000]);
            for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
                st.render(p, None, a, b);
            }
            let peak = l.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            (peak, st.shown)
        };
        let (peak, shown) = onsets(&p);
        assert!(peak > 0.2, "it plays: {peak}");
        assert!(shown.0 >= 0, "the face is told");
        // Starting a cut fades in, so nothing clicks above the break.
        let mut q = p.clone();
        q[RATCHET] = 1.0;
        q[RATCHET_MAX] = 4.0;
        let (peak, _) = onsets(&q);
        assert!(peak > 0.1 && peak < 2.0, "{peak}");
        for name in preset_names() {
            assert_eq!(preset(name).len(), SPECS.len());
        }
    }

    #[test]
    fn a_cut_lands_on_the_hit_nearest_it() {
        let b = builtin();
        let _ = b.analysis();
        let frames = b.frames();
        let (a, e) = cut_bounds(b, 4, 32, frames);
        let grid = 4 * frames / 32;
        assert!(a.abs_diff(grid) < frames / 32 / 4 + 1, "near its place");
        assert!(e > a);
        assert_eq!(cut_bounds(b, 31, 32, frames).1, frames);
    }
}

/// Makes its own break (and finds its hits) now, off the audio thread.
pub fn warm() {
    let _ = builtin().analysis();
}
