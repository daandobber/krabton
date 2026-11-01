//! The Video: pictures (a film, a photo, an animated GIF) that play along
//! with the music, as VJ software would, inside the set.
//!
//! This is its part of the project (the clips in its bank, how they play,
//! which sound moves which effect) and the reckoning that needs no screen:
//! listening to the sound (how loud, the bass, the mids, the highs, the
//! kicks and other hits), following the notes and the beat, and where in
//! the film it is now. The app decodes the pictures and draws them.
//!
//! A *link* joins a source to an effect with an amount: Kick > Zoom 40 %
//! makes the picture jump in on every kick. A few presets fill the links so
//! a clip moves the moment it is loaded.

use serde::{Deserialize, Serialize};

/// Clips in the bank.
pub const MAX_CLIPS: usize = 8;
/// Links at the same time.
pub const MAX_LINKS: usize = 8;
/// Parts kept of one film.
pub const MAX_PARTS: usize = 64;

/// The Video's part of the project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "SetFile")]
pub struct VideoSet {
    /// The bank: films, pictures and GIFs, by path, each with how it plays.
    pub clips: Vec<VideoClip>,
    /// The one that plays (in `clips`).
    pub current: usize,
    /// The scene that plays picks the clip (scene 1 the first, ...).
    pub follow_scenes: bool,
    /// The notes of this track pick the clip (C1 the first, ...; None: they do not).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip_notes: Option<u64>,
    /// When a clip clicked in the bank starts (while the song plays).
    pub launch: Launch,
    /// How one clip makes way for the next, and in how many beats.
    pub transition: Transition,
    pub transition_beats: f32,
    pub fit: Fit,
    /// How much the music moves the picture, everything at once (0..1).
    pub amount: f32,
    /// The Looks (effects and links), as pads to change between.
    pub looks: Vec<VideoLook>,
    /// The Look edited, and the set's own: it plays unless the part, the
    /// clip or the Look sequence has another.
    pub look: usize,
    /// When a Look pad clicked starts, and how many beats a change of Look
    /// takes to morph (0: at once).
    pub look_launch: Launch,
    pub morph_beats: f32,
    /// Looks in turn, a step every so often.
    #[serde(default, skip_serializing_if = "is_default_seq")]
    pub look_seq: LookSeq,
    /// The scene that plays picks the set's Look (scene 1 the first).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub look_scenes: bool,
    /// The notes of this track pick the set's Look (C1 the first).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look_notes: Option<u64>,
    /// Hits heard change the set's Look.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look_hits: Option<HitTrigger>,
    /// Hits heard change the clip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip_hits: Option<HitTrigger>,
}

fn is_default_seq(s: &LookSeq) -> bool {
    *s == LookSeq::default()
}

impl Default for VideoSet {
    fn default() -> Self {
        Self {
            clips: Vec::new(),
            current: 0,
            follow_scenes: false,
            clip_notes: None,
            launch: Launch::Now,
            transition: Transition::Cut,
            transition_beats: 1.0,
            fit: Fit::Fill,
            amount: 1.0,
            looks: vec![VideoLook::new("Pulse", preset("Pulse").map(|p| p.links).unwrap_or_default())],
            look: 0,
            look_launch: Launch::Now,
            morph_beats: 0.0,
            look_seq: LookSeq::default(),
            look_scenes: false,
            look_notes: None,
            look_hits: None,
            clip_hits: None,
        }
    }
}

/// The Video as a set's file holds it: also what older sets wrote, when
/// how a clip plays was the same for every clip (`play`, `speed`, ...):
/// each clip of such a set gets those.
#[derive(Deserialize, Default)]
#[serde(default)]
struct SetFile {
    clips: Vec<ClipFile>,
    current: usize,
    follow_scenes: bool,
    clip_notes: Option<u64>,
    launch: Option<Launch>,
    transition: Option<Transition>,
    transition_beats: Option<f32>,
    fit: Option<Fit>,
    amount: Option<f32>,
    looks: Option<Vec<VideoLook>>,
    look: usize,
    look_launch: Option<Launch>,
    morph_beats: f32,
    look_seq: LookSeq,
    look_scenes: bool,
    look_notes: Option<u64>,
    look_hits: Option<HitTrigger>,
    clip_hits: Option<HitTrigger>,
    // Older sets: one Look of links.
    links: Option<Vec<VideoLink>>,
    // Older sets: how every clip played.
    play: Option<PlayMode>,
    speed: Option<f32>,
    bars: Option<u32>,
    slices: Option<u32>,
    slice_track: Option<u64>,
    reverse: Option<bool>,
}

impl From<SetFile> for VideoSet {
    fn from(f: SetFile) -> Self {
        let d = VideoSet::default();
        let p = ClipPlay::default();
        let old = ClipPlay {
            mode: f.play.unwrap_or(p.mode),
            speed: f.speed.unwrap_or(p.speed),
            bars: f.bars.unwrap_or(p.bars),
            slices: f.slices.unwrap_or(p.slices),
            slice_track: f.slice_track,
            reverse: f.reverse.unwrap_or(p.reverse),
            ..ClipPlay::default()
        };
        // An older set's links are its one Look (named as the preset they are, if they are one).
        let looks = match (f.looks, f.links) {
            (Some(looks), _) => looks,
            (None, Some(links)) => {
                let name = PRESET_NAMES.iter().copied().find(|n| preset(n).is_some_and(|p| same_links(&p.links, &links))).unwrap_or("Look 1");
                vec![VideoLook::new(name, links)]
            }
            (None, None) => d.looks.clone(),
        };
        Self {
            clips: f.clips.into_iter().map(|c| c.into_clip(&old)).collect(),
            current: f.current,
            follow_scenes: f.follow_scenes,
            clip_notes: f.clip_notes,
            launch: f.launch.unwrap_or(d.launch),
            transition: f.transition.unwrap_or(d.transition),
            transition_beats: f.transition_beats.unwrap_or(d.transition_beats),
            fit: f.fit.unwrap_or(d.fit),
            amount: f.amount.unwrap_or(d.amount),
            looks,
            look: f.look,
            look_launch: f.look_launch.unwrap_or(d.look_launch),
            morph_beats: f.morph_beats,
            look_seq: f.look_seq,
            look_scenes: f.look_scenes,
            look_notes: f.look_notes,
            look_hits: f.look_hits,
            clip_hits: f.clip_hits,
        }
    }
}

/// Whether two lists of links are the same, whichever tracks they hear.
pub fn same_links(a: &[VideoLink], b: &[VideoLink]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x.source, x.effect, x.amount) == (y.source, y.effect, y.amount) && x.low == y.low && x.response == y.response)
}

impl VideoSet {
    /// Nothing loaded and nothing changed (not written to the set's file).
    pub fn is_empty(&self) -> bool {
        self.clips.is_empty() && *self == Self::default()
    }

    /// The clip that plays, if there is one.
    pub fn current_clip(&self) -> Option<&VideoClip> {
        self.clips.get(self.current.min(self.clips.len().saturating_sub(1)))
    }

    /// The Look edited (the set's own).
    pub fn edited_look(&self) -> &VideoLook {
        static NONE: std::sync::OnceLock<VideoLook> = std::sync::OnceLock::new();
        self.looks.get(self.look).or(self.looks.first()).unwrap_or_else(|| NONE.get_or_init(VideoLook::default))
    }

    /// The Look edited, to change (one made when there is none).
    pub fn edited_look_mut(&mut self) -> &mut VideoLook {
        if self.looks.is_empty() {
            self.looks.push(VideoLook::new("Look 1", Vec::new()));
        }
        self.look = self.look.min(self.looks.len() - 1);
        &mut self.looks[self.look]
    }

    /// The links of the Look edited.
    pub fn links(&self) -> &[VideoLink] {
        &self.edited_look().links
    }

    /// Puts a preset's links in the Look edited, its effects as they start
    /// (and its way of playing, when it has one, on the clip that plays).
    pub fn apply_preset(&mut self, p: &Preset) {
        let look = self.edited_look_mut();
        *look = VideoLook::new(p.name, p.links.clone());
        if let Some(play) = p.play {
            let i = self.current.min(self.clips.len().saturating_sub(1));
            if let Some(c) = self.clips.get_mut(i) {
                c.play.mode = play;
            }
        }
    }

    /// The transition's length in seconds at `bpm` (0 for a cut).
    pub fn transition_secs(&self, bpm: f64) -> f64 {
        if self.transition == Transition::Cut {
            return 0.0;
        }
        self.transition_beats.clamp(0.0, 64.0) as f64 * 60.0 / bpm.max(1.0)
    }

    /// Fixes what a hand-edited or older file might hold.
    pub fn sanitize(&mut self) {
        self.clips.truncate(MAX_CLIPS);
        self.looks.truncate(MAX_LOOKS);
        for l in &mut self.looks {
            l.sanitize();
        }
        self.look = self.look.min(self.looks.len().saturating_sub(1));
        self.current = self.current.min(self.clips.len().saturating_sub(1));
        self.look_seq.steps.truncate(MAX_STEPS);
        for t in [&mut self.look_hits, &mut self.clip_hits].into_iter().flatten() {
            t.sanitize();
        }
        let ok = |v: f32, lo: f32, hi: f32, d: f32| if v.is_finite() { v.clamp(lo, hi) } else { d };
        self.amount = ok(self.amount, 0.0, 1.0, 1.0);
        self.transition_beats = ok(self.transition_beats, 0.0625, 16.0, 1.0);
        self.morph_beats = ok(self.morph_beats, 0.0, 16.0, 0.0);
        for c in &mut self.clips {
            if let Some(cues) = &mut c.cues {
                cues.sanitize();
            }
            if c.cues.as_ref().is_some_and(|q| q.times.is_empty()) {
                c.cues = None;
            }
            let before = c.parts.clone();
            normalize_parts(&mut c.parts);
            c.retag_parts(&before);
            c.frame.sanitize();
            let p = &mut c.play;
            p.speed = ok(p.speed, 0.0, 4.0, 1.0);
            p.bars = p.bars.clamp(1, 64);
            p.slices = p.slices.clamp(2, 64);
            p.offset = ok(p.offset, 0.0, 1.0e6, 0.0);
            p.seq.steps.truncate(MAX_STEPS);
            p.seq.hits.sanitize();
            if let Some(t) = &mut p.slice_hits {
                t.sanitize();
            }
        }
    }

    /// Slice to Drum Rack: the film at `path` (into the bank when it is not
    /// there yet) cut where its sound was, `cues` saying where and whose
    /// notes play them; it plays now, in Slices. None when the bank is full.
    pub fn cue_film(&mut self, path: &std::path::Path, cues: Cues) -> Option<usize> {
        let i = match self.clips.iter().position(|c| c.path == path) {
            Some(i) => i,
            None if self.clips.len() < MAX_CLIPS => {
                self.clips.push(VideoClip::new(path.to_path_buf()));
                self.clips.len() - 1
            }
            None => return None,
        };
        self.clips[i].cues = Some(cues);
        self.clips[i].play.mode = PlayMode::Slices;
        self.current = i;
        Some(i)
    }
}

/// How a clip plays. Each clip has its own: a short loop can keep time with
/// the song while a film next to it runs on its own.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClipPlay {
    pub mode: PlayMode,
    /// Loop and Slices: how fast it plays (1: as filmed).
    pub speed: f32,
    /// Tempo: the whole clip (its parts back to back) lasts this many bars.
    pub bars: u32,
    /// Where it can be cut: the clip in this many equal slices (Slices, Jump).
    pub slices: u32,
    /// Slices: the track whose notes pick them (None: any track).
    pub slice_track: Option<u64>,
    pub reverse: bool,
    /// Slices: one slice for each part (when it has several), so the notes
    /// play the parts: C1 the first part, C#1 the second...
    pub part_slices: bool,
    /// Forwards and back again (Loop and Tempo; with Reverse it starts backwards).
    pub bounce: bool,
    /// Where it starts (seconds into its parts, Loop and Tempo).
    pub offset: f32,
    /// It stands still: one frame (where `offset` puts it, or the slice a note picks).
    pub freeze: bool,
    /// Slices: hits heard in a sound pick them instead of notes (for
    /// sequencers that make sound without notes).
    pub slice_hits: Option<HitTrigger>,
    /// Its parts one at a time, changing on the beat (Slices then cut the part that plays).
    pub seq: PartSeq,
}

impl Default for ClipPlay {
    fn default() -> Self {
        Self { mode: PlayMode::Loop, speed: 1.0, bars: 4, slices: 16, slice_track: None, reverse: false, part_slices: false, bounce: false, offset: 0.0, freeze: false, slice_hits: None, seq: PartSeq::default() }
    }
}

/// How often something changes in time with the song.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Every {
    Beat,
    Bar,
    TwoBars,
    FourBars,
}

impl Every {
    pub const ALL: [Every; 4] = [Every::Beat, Every::Bar, Every::TwoBars, Every::FourBars];

    pub fn name(self) -> &'static str {
        match self {
            Every::Beat => "Beat",
            Every::Bar => "Bar",
            Every::TwoBars => "2 bars",
            Every::FourBars => "4 bars",
        }
    }

    /// How many beats that is.
    pub fn beats(self, beats_per_bar: u32) -> f64 {
        let bar = beats_per_bar.max(1) as f64;
        match self {
            Every::Beat => 1.0,
            Every::Bar => bar,
            Every::TwoBars => 2.0 * bar,
            Every::FourBars => 4.0 * bar,
        }
    }

    /// The step of the song's position `beat` (the first bar's first: 0).
    pub fn step(self, beat: f64, beats_per_bar: u32) -> i64 {
        ((beat + 1e-9) / self.beats(beats_per_bar)).floor() as i64
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

/// Which part a clip's part sequence plays when.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartOrder {
    /// One after the other, round again.
    InOrder,
    /// Up to the last and back down.
    PingPong,
    /// Another one by chance, never the same twice.
    Random,
    /// The notes of a track: C3 the first part, C#3 the second...
    Notes,
    /// As its steps say, one a step.
    Steps,
    /// Hits heard in a sound.
    Hits,
}

impl PartOrder {
    pub const ALL: [PartOrder; 6] = [PartOrder::InOrder, PartOrder::PingPong, PartOrder::Random, PartOrder::Steps, PartOrder::Notes, PartOrder::Hits];

    pub fn name(self) -> &'static str {
        match self {
            PartOrder::InOrder => "In order",
            PartOrder::PingPong => "Ping-pong",
            PartOrder::Random => "Random",
            PartOrder::Notes => "Notes",
            PartOrder::Steps => "Steps",
            PartOrder::Hits => "Hits heard",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            PartOrder::InOrder => "One part after the other, a step each, round again",
            PartOrder::PingPong => "Up to the last part and back down",
            PartOrder::Random => "Another part by chance every step, never the same twice",
            PartOrder::Notes => "The notes of a track pick the part, as they come: C3 the first part, C#3 the second (below C3 they still play slices)",
            PartOrder::Steps => "As the steps say: a part for every step, or the one before held",
            PartOrder::Hits => "Hits heard in a sound change the part (for sequencers that make sound without notes)",
        }
    }

    /// Whether the song's steps change it (else notes or hits do).
    pub fn on_steps(self) -> bool {
        matches!(self, PartOrder::InOrder | PartOrder::PingPong | PartOrder::Random | PartOrder::Steps)
    }
}

/// The note that picks the first part (C3): the parts sit above the slices.
pub const PART_NOTE: u8 = 60;

/// Steps in a sequence, at most.
pub const MAX_STEPS: usize = 16;

/// A clip's parts one at a time: which part plays changes every `every`,
/// as `order` says; Loop runs round in it, Tempo fits it into its bars and
/// Slices cut it alone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PartSeq {
    pub on: bool,
    pub every: Every,
    pub order: PartOrder,
    /// Notes: the track whose notes pick the part (None: every track).
    pub track: Option<u64>,
    /// Steps: a part (from 0) for every step; None holds the one before.
    pub steps: Vec<Option<u8>>,
    /// Hits: what hears them and what a hit does.
    pub hits: HitTrigger,
}

impl Default for PartSeq {
    fn default() -> Self {
        Self { on: false, every: Every::Bar, order: PartOrder::InOrder, track: None, steps: vec![Some(0), Some(1), Some(2), Some(3)], hits: HitTrigger::default() }
    }
}

impl PartSeq {
    /// The part step `step` of the song plays, of `n` (None: it holds the
    /// one before; Random, Notes and Hits are not a matter of the step).
    pub fn part_at(&self, step: i64, n: usize) -> Option<usize> {
        if n == 0 {
            return None;
        }
        match self.order {
            PartOrder::InOrder => Some(step.rem_euclid(n as i64) as usize),
            PartOrder::PingPong if n > 1 => {
                let lap = 2 * (n as i64 - 1);
                let i = step.rem_euclid(lap);
                Some(if i < n as i64 { i } else { lap - i } as usize)
            }
            PartOrder::PingPong => Some(0),
            PartOrder::Steps if !self.steps.is_empty() => self.steps[step.rem_euclid(self.steps.len() as i64) as usize].map(|k| k as usize % n),
            _ => None,
        }
    }
}

/// The part a note picks: C3 the first, round again after the last; None below C3.
pub fn part_for_note(pitch: u8, parts: usize) -> Option<usize> {
    (pitch >= PART_NOTE && parts > 0).then(|| (pitch - PART_NOTE) as usize % parts)
}

/// Which band a hit is heard in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HitBand {
    /// The low end: kicks.
    Kick,
    /// The middle: snares, claps.
    Snare,
    /// The top: hats, shakers.
    Hats,
    /// Any of them.
    Any,
}

impl HitBand {
    pub const ALL: [HitBand; 4] = [HitBand::Kick, HitBand::Snare, HitBand::Hats, HitBand::Any];

    pub fn name(self) -> &'static str {
        match self {
            HitBand::Kick => "Kicks",
            HitBand::Snare => "Snares",
            HitBand::Hats => "Hats",
            HitBand::Any => "Any hit",
        }
    }
}

/// Where a hit moves something along a row: the next, the one before,
/// another by chance, or one by the hit's band.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepTo {
    Next,
    Previous,
    Random,
    /// The hit's band: a kick the first, a snare the second, a hat the third.
    ByBand,
}

impl StepTo {
    pub const ALL: [StepTo; 4] = [StepTo::Next, StepTo::Previous, StepTo::Random, StepTo::ByBand];

    pub fn name(self) -> &'static str {
        match self {
            StepTo::Next => "Next",
            StepTo::Previous => "Previous",
            StepTo::Random => "Random",
            StepTo::ByBand => "By band",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            StepTo::Next => "Every hit the next one, round again",
            StepTo::Previous => "Every hit the one before",
            StepTo::Random => "Every hit another one by chance, never the same twice",
            StepTo::ByBand => "A kick the first, a snare the second, a hat the third",
        }
    }
}

/// A move along a row, resolved by whoever keeps the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Next,
    Previous,
    Random,
    To(usize),
}

impl Step {
    /// From `now`, of `n`.
    pub fn from(self, now: usize, n: usize, dice: &mut Dice) -> usize {
        let n = n.max(1);
        match self {
            Step::Next => (now + 1) % n,
            Step::Previous => (now + n - 1) % n,
            Step::Random => other_slice(dice, now, n as u32),
            Step::To(k) => k % n,
        }
    }

    /// What a hit in `band` (0 kick, 1 snare, 2 hat) does, as `to` says.
    pub fn of_hit(to: StepTo, band: usize) -> Step {
        match to {
            StepTo::Next => Step::Next,
            StepTo::Previous => Step::Previous,
            StepTo::Random => Step::Random,
            StepTo::ByBand => Step::To(band),
        }
    }
}

/// Hits heard in a sound as a trigger: in which band, how keenly, no
/// sooner than `gap` beats after the last, and (with `quantize`) put on
/// the song's grid.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HitTrigger {
    /// The track heard (None: the master).
    pub track: Option<u64>,
    pub band: HitBand,
    /// 0..1: how small a hit still counts.
    pub sensitivity: f32,
    /// The least time between two hits (beats).
    pub gap: f32,
    /// Put on the next step of a grid this long (beats; 0: as heard).
    pub quantize: f32,
    pub to: StepTo,
}

impl Default for HitTrigger {
    fn default() -> Self {
        Self { track: None, band: HitBand::Kick, sensitivity: 0.5, gap: 0.25, quantize: 0.0, to: StepTo::Next }
    }
}

/// The lengths a hit's gap and grid offer (beats; 0: none).
pub const HIT_LENGTHS: [f32; 6] = [0.0, 0.125, 0.25, 0.5, 1.0, 2.0];

impl HitTrigger {
    pub fn sanitize(&mut self) {
        let ok = |v: f32, lo: f32, hi: f32, d: f32| if v.is_finite() { v.clamp(lo, hi) } else { d };
        self.sensitivity = ok(self.sensitivity, 0.0, 1.0, 0.5);
        self.gap = ok(self.gap, 0.0, 16.0, 0.25);
        self.quantize = ok(self.quantize, 0.0, 16.0, 0.0);
    }
}

/// How sharply each band rose (dB over what it just was) since it was last
/// asked, counted only when loud enough to matter, and how loud it was
/// then next to the whole sound (dB, 0 or less): the low end, the middle
/// and the top.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rises {
    pub rise: [f32; 3],
    pub share: [f32; 3],
}

/// Hits heard, as a trigger fires them: the gap kept, the grid waited for.
#[derive(Clone, Copy, Debug)]
pub struct HitDetector {
    since: f64,
    /// A hit waiting for the grid: where (beats) and in which band.
    pending: Option<(f64, usize)>,
}

impl Default for HitDetector {
    fn default() -> Self {
        Self { since: f64::INFINITY, pending: None }
    }
}

impl HitDetector {
    /// One frame: what the sound did (`rises`), `dt` seconds on, the song at
    /// `beat`. The band (0 kick, 1 snare, 2 hat) of a hit that fires now.
    pub fn frame(&mut self, t: &HitTrigger, rises: Rises, dt: f32, beat: f64, playing: bool, bpm: f64) -> Option<usize> {
        self.since += dt.max(0.0) as f64;
        let mut fired = None;
        if let Some((at, band)) = self.pending {
            let q = t.quantize.max(1e-3) as f64;
            if !playing || beat >= at - 1e-9 || beat < at - 2.0 * q - 1e-6 {
                self.pending = None;
                fired = Some(band);
            }
        }
        // How far a band has to rise: 12 dB with no keenness, 3 dB at most.
        let need = 12.0 - 9.0 * t.sensitivity.clamp(0.0, 1.0);
        let heard = match t.band {
            HitBand::Kick => (rises.rise[0] > need).then_some(0),
            HitBand::Snare => (rises.rise[1] > need).then_some(1),
            HitBand::Hats => (rises.rise[2] > need).then_some(2),
            // Any: the band that carries most of the sound, of those that rose.
            HitBand::Any => (0..3).filter(|&b| rises.rise[b] > need).max_by(|&a, &b| rises.share[a].total_cmp(&rises.share[b])),
        };
        let gap = (t.gap.max(0.0) as f64 * 60.0 / bpm.max(1.0)).max(0.06);
        if let (Some(band), true) = (heard, self.since >= gap) {
            self.since = 0.0;
            let q = t.quantize as f64;
            if q > 0.0 && playing && fired.is_none() {
                let start = (beat / q).floor() * q;
                // Just after a step of the grid (a little late): on it, at once.
                if beat - start < q * 0.15 {
                    fired = Some(band);
                } else {
                    self.pending = Some((start + q, band));
                }
            } else if fired.is_none() {
                fired = Some(band);
            }
        }
        fired
    }
}

/// Looks in turn: a step every `every`, each step a Look (None: the set's own).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LookSeq {
    pub on: bool,
    pub every: Every,
    pub steps: Vec<Option<u8>>,
}

impl Default for LookSeq {
    fn default() -> Self {
        Self { on: false, every: Every::Bar, steps: vec![Some(0), Some(1), Some(0), Some(1)] }
    }
}

impl LookSeq {
    /// The Look the song's position `beat` asks for (None: the set's own).
    pub fn look_at(&self, beat: f64, beats_per_bar: u32, looks: usize) -> Option<usize> {
        if !self.on || self.steps.is_empty() || looks == 0 {
            return None;
        }
        let step = self.every.step(beat, beats_per_bar).rem_euclid(self.steps.len() as i64) as usize;
        self.steps[step].map(|k| k as usize).filter(|k| *k < looks)
    }
}

/// Which Look plays, of `n`: the part's, else the clip's, else the Look
/// sequence's step, else the set's own (the pad clicked, or picked by the
/// notes, the hits or the scenes).
pub fn look_for(part: Option<usize>, clip: Option<usize>, seq: Option<usize>, set: usize, n: usize) -> usize {
    [part, clip, seq].into_iter().flatten().find(|k| *k < n).unwrap_or(set).min(n.saturating_sub(1))
}

/// Looks in the set, at most (the pads).
pub const MAX_LOOKS: usize = 8;

/// A Look: the effects (how much of each there always is, their own
/// settings) and the links of the music to them, under a name. The set
/// keeps a few, as pads to change between while it plays.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoLook {
    pub name: String,
    /// The effects set differently from how they start.
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub fx: std::collections::BTreeMap<Effect, FxSetting>,
    pub links: Vec<VideoLink>,
}

impl VideoLook {
    pub fn new(name: impl Into<String>, links: Vec<VideoLink>) -> Self {
        Self { name: name.into(), fx: Default::default(), links }
    }

    /// How effect `e` is set (as it starts when it is not).
    pub fn setting(&self, e: Effect) -> FxSetting {
        self.fx.get(&e).cloned().unwrap_or_default()
    }

    /// Effect `e`'s setting to change (made when it is not there yet).
    pub fn setting_mut(&mut self, e: Effect) -> &mut FxSetting {
        self.fx.entry(e).or_default()
    }

    /// The effects it uses: set, or moved by a link (in the lists' order).
    pub fn effects(&self) -> Vec<Effect> {
        Effect::ALL.iter().copied().filter(|e| self.fx.contains_key(e) || self.links.iter().any(|l| l.effect == *e)).collect()
    }

    pub fn sanitize(&mut self) {
        self.links.truncate(MAX_LINKS);
        for l in &mut self.links {
            l.sanitize();
        }
        for (e, s) in self.fx.iter_mut() {
            s.sanitize(*e);
        }
    }
}

/// How a clip sits on the screen: its own fit, how see-through, where,
/// how big and turned, and what is cut off its sides.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClipFrame {
    /// None: the set's Fill or Fit.
    pub fit: Option<Fit>,
    pub opacity: f32,
    pub zoom: f32,
    /// Moved (fractions of the screen; right and down).
    pub x: f32,
    pub y: f32,
    /// Turned (a turn: 1).
    pub turn: f32,
    /// Cut off the left, top, right and bottom (fractions of the picture).
    pub crop: [f32; 4],
}

impl Default for ClipFrame {
    fn default() -> Self {
        Self { fit: None, opacity: 1.0, zoom: 1.0, x: 0.0, y: 0.0, turn: 0.0, crop: [0.0; 4] }
    }
}

impl ClipFrame {
    pub fn is_plain(&self) -> bool {
        *self == Self::default()
    }

    pub fn sanitize(&mut self) {
        let ok = |v: f32, lo: f32, hi: f32, d: f32| if v.is_finite() { v.clamp(lo, hi) } else { d };
        self.opacity = ok(self.opacity, 0.0, 1.0, 1.0);
        self.zoom = ok(self.zoom, 0.1, 8.0, 1.0);
        self.x = ok(self.x, -1.0, 1.0, 0.0);
        self.y = ok(self.y, -1.0, 1.0, 0.0);
        self.turn = ok(self.turn, -1.0, 1.0, 0.0);
        for c in &mut self.crop {
            *c = ok(*c, 0.0, 0.45, 0.0);
        }
    }
}

/// A clip in the bank.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "ClipFile")]
pub struct VideoClip {
    pub path: std::path::PathBuf,
    pub name: String,
    /// Where Slice to Drum Rack cut it (None: in equal slices).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cues: Option<Cues>,
    /// The parts of the film that play (seconds, from and to), in order and
    /// apart; they play back to back, the rest is skipped. None: all of it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<(f32, f32)>,
    pub play: ClipPlay,
    /// The Look it plays with (None: the set's), and each part's (by place
    /// in `parts`; None: the clip's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub part_looks: Vec<Option<usize>>,
    /// How it sits on the screen.
    #[serde(default, skip_serializing_if = "ClipFrame::is_plain")]
    pub frame: ClipFrame,
}

/// A clip as a set's file holds it: older sets kept one part, as `start`
/// and `end`, and had no `play` of their own.
#[derive(Deserialize)]
struct ClipFile {
    path: std::path::PathBuf,
    #[serde(default)]
    name: String,
    #[serde(default)]
    cues: Option<Cues>,
    #[serde(default)]
    parts: Vec<(f32, f32)>,
    #[serde(default)]
    start: f32,
    #[serde(default)]
    end: Option<f32>,
    #[serde(default)]
    play: Option<ClipPlay>,
    #[serde(default)]
    look: Option<usize>,
    #[serde(default)]
    part_looks: Vec<Option<usize>>,
    #[serde(default)]
    frame: ClipFrame,
}

impl ClipFile {
    fn into_clip(self, old: &ClipPlay) -> VideoClip {
        let mut parts = self.parts;
        if parts.is_empty() && (self.start > 0.0 || self.end.is_some()) {
            parts.push((self.start, self.end.unwrap_or(TO_THE_END)));
        }
        VideoClip { path: self.path, name: self.name, cues: self.cues, parts, play: self.play.unwrap_or_else(|| old.clone()), look: self.look, part_looks: self.part_looks, frame: self.frame }
    }
}

impl From<ClipFile> for VideoClip {
    fn from(f: ClipFile) -> Self {
        f.into_clip(&ClipPlay::default())
    }
}

/// The shortest part of a film that can be kept (seconds).
pub const MIN_PART: f32 = 0.1;
/// A part's end that is the film's end, however long the film is.
pub const TO_THE_END: f32 = 1.0e9;

/// Puts parts in order: each the right way round and not before the
/// film's start, the ones shorter than [`MIN_PART`] gone, overlapping ones
/// made one (touching ones stay two, as a split leaves them), no more than
/// [`MAX_PARTS`]. (A part may end at
/// [`TO_THE_END`]: the film's end, whatever its length.)
pub fn normalize_parts(parts: &mut Vec<(f32, f32)>) {
    parts.retain(|(a, b)| !a.is_nan() && !b.is_nan());
    for p in parts.iter_mut() {
        if p.1 < p.0 {
            *p = (p.1, p.0);
        }
        p.0 = p.0.max(0.0);
        p.1 = p.1.min(TO_THE_END);
    }
    parts.retain(|(a, b)| a.is_finite() && b - a >= MIN_PART - 1e-6);
    parts.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut out: Vec<(f32, f32)> = Vec::with_capacity(parts.len());
    for &(a, b) in parts.iter() {
        match out.last_mut() {
            Some(last) if a < last.1 - 1e-4 => last.1 = last.1.max(b),
            // Apart (or touching: then from where the last ends).
            last => {
                let a = last.map_or(a, |l| a.max(l.1));
                out.push((a, b));
            }
        }
    }
    out.truncate(MAX_PARTS);
    *parts = out;
}

impl VideoClip {
    pub fn new(path: std::path::PathBuf) -> Self {
        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        Self { path, name, cues: None, parts: Vec::new(), play: ClipPlay::default(), look: None, part_looks: Vec::new(), frame: ClipFrame::default() }
    }

    /// The Look part `k` has (None: the clip's).
    pub fn part_look(&self, k: usize) -> Option<usize> {
        self.part_looks.get(k).copied().flatten()
    }

    /// Gives part `k` a Look (None: the clip's).
    pub fn set_part_look(&mut self, k: usize, look: Option<usize>) {
        if k >= self.parts.len() {
            return;
        }
        self.part_looks.resize(self.parts.len(), None);
        self.part_looks[k] = look;
        if self.part_looks.iter().all(Option::is_none) {
            self.part_looks.clear();
        }
    }

    /// After the parts changed from `before`: each part keeps the Look of
    /// the part it overlaps most of those before.
    pub fn retag_parts(&mut self, before: &[(f32, f32)]) {
        if self.part_looks.iter().all(Option::is_none) {
            self.part_looks.clear();
            return;
        }
        let old = std::mem::take(&mut self.part_looks);
        let looks: Vec<Option<usize>> = self
            .parts
            .iter()
            .map(|&(a, b)| {
                let best = before.iter().enumerate().map(|(j, &(c, d))| (j, (b.min(d) - a.max(c)).max(0.0))).filter(|x| x.1 > 0.0).max_by(|x, y| x.1.total_cmp(&y.1));
                best.and_then(|(j, _)| old.get(j).copied().flatten())
            })
            .collect();
        if looks.iter().any(Option::is_some) {
            self.part_looks = looks;
        }
    }

    /// Takes part `k` away (and its Look).
    pub fn remove_part(&mut self, k: usize) {
        if k < self.parts.len() {
            self.parts.remove(k);
            if k < self.part_looks.len() {
                self.part_looks.remove(k);
            }
        }
    }

    /// Cut by Slice to Drum Rack: it plays whole, by its cues.
    pub fn is_cued(&self) -> bool {
        self.cues.as_ref().is_some_and(|c| !c.is_empty())
    }

    /// Whether only parts of it play.
    pub fn is_cut(&self) -> bool {
        !self.parts.is_empty() && !self.is_cued()
    }

    /// The parts that play in a film `duration` long (seconds, in order):
    /// those inside the film, or all of it when none are (or it is cued).
    pub fn spans(&self, duration: f32) -> Vec<(f32, f32)> {
        let d = duration.max(0.0);
        let mut out: Vec<(f32, f32)> = if self.is_cued() { Vec::new() } else { self.parts.iter().map(|&(a, b)| (a.clamp(0.0, d), b.clamp(0.0, d))).filter(|(a, b)| b - a >= 1e-3).collect() };
        if out.is_empty() {
            out.push((0.0, d));
        }
        out
    }

    /// How long its parts last back to back (seconds).
    pub fn joined_len(&self, duration: f32) -> f32 {
        joined_len(&self.spans(duration))
    }

    /// Where in the film it is `t` seconds into its parts played back to
    /// back (round again after the last).
    pub fn film_time(&self, t: f64, duration: f32) -> f64 {
        film_time(&self.spans(duration), t)
    }

    /// The other way: how far into the parts the film's `t` is (None in a gap).
    pub fn joined_time(&self, t: f64, duration: f32) -> Option<f64> {
        joined_time(&self.spans(duration), t)
    }

    /// The part that holds the film's `t`, if one does.
    pub fn part_at(&self, t: f32) -> Option<usize> {
        self.parts.iter().position(|&(a, b)| t >= a && t < b)
    }

    /// Keeps the part from `a` to `b` too (put in order, joined to any it touches).
    pub fn add_part(&mut self, a: f32, b: f32) {
        let before = self.parts.clone();
        self.parts.push((a.min(b), a.max(b)));
        normalize_parts(&mut self.parts);
        self.retag_parts(&before);
    }

    /// Cuts the part that holds `t` in two there. Whether it did.
    pub fn split_part(&mut self, t: f32) -> bool {
        let Some(i) = self.part_at(t) else { return false };
        let (a, b) = self.parts[i];
        if t - a < MIN_PART || b - t < MIN_PART || self.parts.len() >= MAX_PARTS {
            return false;
        }
        self.parts[i] = (a, t);
        self.parts.insert(i + 1, (t, b));
        if i < self.part_looks.len() {
            self.part_looks.insert(i + 1, self.part_looks[i]);
        }
        true
    }

    /// How many slices it is cut in (by its parts, or equally), with the
    /// parts as they play now.
    pub fn slice_count(&self, spans: &[(f32, f32)]) -> u32 {
        if self.play.part_slices && spans.len() > 1 {
            spans.len() as u32
        } else {
            self.play.slices.max(1)
        }
    }

    /// Where slice `k` starts and ends in the parts played back to back.
    pub fn slice_bounds(&self, spans: &[(f32, f32)], k: usize) -> (f64, f64) {
        let total = joined_len(spans) as f64;
        if self.play.part_slices && spans.len() > 1 {
            let k = k.min(spans.len() - 1);
            let from: f64 = spans[..k].iter().map(|(a, b)| (b - a) as f64).sum();
            return (from, from + (spans[k].1 - spans[k].0) as f64);
        }
        let n = self.play.slices.max(1) as f64;
        let len = total / n;
        (k as f64 * len, (k as f64 + 1.0) * len)
    }
}

/// How long `spans` last back to back.
pub fn joined_len(spans: &[(f32, f32)]) -> f32 {
    spans.iter().map(|(a, b)| (b - a).max(0.0)).sum()
}

/// Where in the film it is `t` seconds into `spans` played back to back
/// (round again after the last).
pub fn film_time(spans: &[(f32, f32)], t: f64) -> f64 {
    let total = joined_len(spans) as f64;
    if total <= 0.0 {
        return spans.first().map_or(0.0, |s| s.0 as f64);
    }
    let mut left = t.rem_euclid(total);
    for &(a, b) in spans {
        let len = (b - a) as f64;
        if left < len {
            return a as f64 + left;
        }
        left -= len;
    }
    // Only by rounding: the very end of the last.
    spans.last().map_or(0.0, |s| s.1 as f64)
}

/// How far into `spans` played back to back the film's `t` is (None in a gap).
pub fn joined_time(spans: &[(f32, f32)], t: f64) -> Option<f64> {
    let mut acc = 0.0;
    for &(a, b) in spans {
        if t >= a as f64 && t < b as f64 {
            return Some(acc + t - a as f64);
        }
        acc += (b - a) as f64;
    }
    None
}

/// When a clip clicked in the bank starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Launch {
    /// At once.
    Now,
    /// On the next beat.
    Beat,
    /// On the next bar.
    Bar,
}

impl Launch {
    pub const ALL: [Launch; 3] = [Launch::Now, Launch::Beat, Launch::Bar];

    pub fn name(self) -> &'static str {
        match self {
            Launch::Now => "Now",
            Launch::Beat => "Beat",
            Launch::Bar => "Bar",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            Launch::Now => "A clip clicked in the bank plays at once",
            Launch::Beat => "A clip clicked in the bank waits for the next beat (while the song plays)",
            Launch::Bar => "A clip clicked in the bank waits for the next bar (while the song plays)",
        }
    }
}

/// Where in the song (beats) a clip asked for at `beat` starts, by
/// `launch`; None: at once (Now, the song stopped, or just on the beat).
pub fn launch_beat(launch: Launch, beat: f64, playing: bool, beats_per_bar: u32) -> Option<f64> {
    let q = match launch {
        Launch::Now => return None,
        Launch::Beat => 1.0,
        Launch::Bar => beats_per_bar.max(1) as f64,
    };
    if !playing || !beat.is_finite() {
        return None;
    }
    let start = (beat / q).floor() * q;
    // A click a little late for the beat still makes it.
    if beat - start < 0.05 {
        return None;
    }
    Some(start + q)
}

/// How one clip makes way for the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transition {
    /// At once.
    Cut,
    /// The one fading into the other.
    Fade,
    /// Through white.
    Flash,
    /// Strips of the next tearing through.
    Glitch,
}

impl Transition {
    pub const ALL: [Transition; 4] = [Transition::Cut, Transition::Fade, Transition::Flash, Transition::Glitch];

    pub fn name(self) -> &'static str {
        match self {
            Transition::Cut => "Cut",
            Transition::Fade => "Fade",
            Transition::Flash => "Flash",
            Transition::Glitch => "Glitch",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            Transition::Cut => "The next clip at once",
            Transition::Fade => "The clip fades into the next",
            Transition::Flash => "To white and out of it on the next clip",
            Transition::Glitch => "Strips of the next clip tear through, more and more",
        }
    }
}

/// Which clip plays, a clip waiting for its beat, and the one it is
/// changing from. Clips change at once ([`Switch::go`]) or on the beat
/// ([`Switch::ask`]); a change with a transition keeps the last clip for
/// its length.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Switch {
    pub clip: usize,
    /// A clip waiting for a song position (beats), and where the song was
    /// when it was asked for.
    pub pending: Option<(usize, f64, f64)>,
    pub fade: Option<Fade>,
}

/// A change going on: from clip `from`, `elapsed` of `len` seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fade {
    pub from: usize,
    pub elapsed: f64,
    pub len: f64,
}

impl Fade {
    /// How far it is, 0 (all the last clip) to 1 (all the new one).
    pub fn mix(&self) -> f32 {
        if self.len <= 0.0 {
            1.0
        } else {
            (self.elapsed / self.len).clamp(0.0, 1.0) as f32
        }
    }
}

impl Switch {
    /// Clip `clip` now, changing in `secs` seconds (0: a cut). Whether it changed.
    pub fn go(&mut self, clip: usize, secs: f64) -> bool {
        self.pending = None;
        if clip == self.clip {
            return false;
        }
        self.fade = (secs > 0.0).then_some(Fade { from: self.clip, elapsed: 0.0, len: secs });
        self.clip = clip;
        true
    }

    /// Clip `clip` at song position `at`, asked for at `now` (None: at
    /// once, see `go`). Whether it changed now.
    pub fn ask(&mut self, clip: usize, now: f64, at: Option<f64>, secs: f64) -> bool {
        match at {
            None => self.go(clip, secs),
            Some(b) => {
                self.pending = Some((clip, b, now));
                false
            }
        }
    }

    /// One frame at song position `beat`: a waiting clip starts when its
    /// beat comes (or at once when the song stops or jumps back), a change
    /// moves on. Whether a clip started.
    pub fn frame(&mut self, beat: f64, playing: bool, dt: f64, secs: f64) -> bool {
        if let Some(f) = &mut self.fade {
            f.elapsed += dt.max(0.0);
            if f.elapsed >= f.len {
                self.fade = None;
            }
        }
        match self.pending {
            Some((clip, at, asked)) if !playing || beat >= at - 1e-9 || beat < asked - 1e-6 => self.go(clip, secs),
            _ => false,
        }
    }
}

/// The films the Video reads (with ffmpeg), by extension (lower case).
pub const FILM_EXTENSIONS: [&str; 12] = ["mp4", "mov", "m4v", "webm", "mkv", "avi", "wmv", "mpg", "mpeg", "ogv", "flv", "3gp"];

/// Whether a file is a film (by its name): its sound can be sliced with
/// its picture.
pub fn is_film(path: &std::path::Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| FILM_EXTENSIONS.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// Cues are kept to this many (a Drum Rack has 16 pads; room to spare).
pub const MAX_CUES: usize = 128;

/// Where Slice to Drum Rack cut a film: the same cuts as its sound's
/// slices, so a note shows the picture from where its pad's sound starts,
/// C1 the first cue, C#1 the second...
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Cues {
    /// Where each piece starts (seconds of the film), in order.
    pub times: Vec<f64>,
    /// Where the last one ends.
    pub end: f64,
    /// The track whose notes play them (by id; None: every track).
    pub track: Option<u64>,
    /// As the pads: Gate holds the picture where the note ends; One-Shot
    /// plays the piece on to its end.
    pub gate: bool,
}

impl Default for Cues {
    fn default() -> Self {
        Self { times: Vec::new(), end: 0.0, track: None, gate: false }
    }
}

impl Cues {
    pub fn len(&self) -> usize {
        self.times.len()
    }

    pub fn is_empty(&self) -> bool {
        self.times.is_empty()
    }

    /// The cue a note plays: C1 (36) the first, round again after the last.
    pub fn for_note(&self, pitch: u8) -> usize {
        slice_for_note(pitch, self.len() as u32)
    }

    /// Where piece `k` starts and ends (seconds), the last no further than
    /// `duration` (the film's length, when known).
    pub fn piece(&self, k: usize, duration: f64) -> (f64, f64) {
        let a = self.times.get(k).copied().unwrap_or(0.0);
        let mut b = self.times.get(k + 1).copied().unwrap_or(self.end);
        if duration > 0.0 {
            b = b.min(duration);
        }
        (a, b.max(a))
    }

    /// The piece that `t` seconds is in.
    pub fn at(&self, t: f64) -> usize {
        self.times.partition_point(|&c| c <= t).saturating_sub(1)
    }

    /// Fixes what a hand-edited file might hold: in order, none below 0.
    pub fn sanitize(&mut self) {
        self.times.retain(|t| t.is_finite());
        for t in &mut self.times {
            *t = t.max(0.0);
        }
        self.times.sort_by(f64::total_cmp);
        self.times.truncate(MAX_CUES);
        let last = self.times.last().copied().unwrap_or(0.0);
        self.end = if self.end.is_finite() { self.end.max(last) } else { last };
    }
}

/// How the clip runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayMode {
    /// On its own, round and round, at its speed.
    Loop,
    /// In time with the song: the whole clip in so many bars.
    Tempo,
    /// Cut in slices that notes play, as pads of a drum rack: C1 the
    /// first slice, C#1 the second... A clip with cues (see `Cues`) is cut
    /// at them instead.
    Slices,
}

impl PlayMode {
    pub const ALL: [PlayMode; 3] = [PlayMode::Loop, PlayMode::Tempo, PlayMode::Slices];

    pub fn name(self) -> &'static str {
        match self {
            PlayMode::Loop => "Loop",
            PlayMode::Tempo => "Tempo",
            PlayMode::Slices => "Slices",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            PlayMode::Loop => "Plays round and round on its own, at its speed",
            PlayMode::Tempo => "In time with the song: the whole clip lasts so many bars, and follows the song's position",
            PlayMode::Slices => "Cut in slices that notes play, like pads: C1 the first slice, C#1 the second, and on. Equal slices, or a film's cues where Slice to Drum Rack cut its sound",
        }
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

/// How the picture fills the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fit {
    /// All of the screen, cutting off what sticks out.
    Fill,
    /// All of the picture, with black bars.
    Fit,
}

impl Fit {
    pub fn name(self) -> &'static str {
        match self {
            Fit::Fill => "Fill",
            Fit::Fit => "Fit",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Fit::Fill => Fit::Fit,
            Fit::Fit => Fit::Fill,
        }
    }
}

/// What the music does that can move the picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Source {
    /// How loud.
    Level,
    /// The low end (below about 150 Hz).
    Bass,
    Mids,
    /// The top (above about 3 kHz): hats, cymbals, air.
    Highs,
    /// A hit in the low end, falling away.
    Kick,
    /// Any hit (a sudden rise anywhere), falling away.
    Hits,
    /// A note starting (as hard as it is played), falling away.
    Notes,
    /// The note played last, round the octave (C to B).
    Pitch,
    /// Every beat of the song, falling away.
    Beat,
    /// Every bar.
    Bar,
    /// Through the bar from 0 to 1, again every bar.
    Ramp,
    /// Always full: the effect as much as the link's amount, whatever the music.
    Fixed,
    /// An LFO in time with the song (its shape and length on the link).
    Lfo,
    /// How loud a band of its own is (from and to, on the link).
    Band,
}

impl Source {
    pub const ALL: [Source; 14] = [Source::Kick, Source::Hits, Source::Bass, Source::Mids, Source::Highs, Source::Band, Source::Level, Source::Notes, Source::Pitch, Source::Beat, Source::Bar, Source::Ramp, Source::Lfo, Source::Fixed];

    pub fn name(self) -> &'static str {
        match self {
            Source::Level => "Level",
            Source::Bass => "Bass",
            Source::Mids => "Mids",
            Source::Highs => "Highs",
            Source::Kick => "Kick",
            Source::Hits => "Hits",
            Source::Notes => "Notes",
            Source::Pitch => "Pitch",
            Source::Beat => "Beat",
            Source::Bar => "Bar",
            Source::Ramp => "Ramp",
            Source::Fixed => "Fixed",
            Source::Lfo => "LFO",
            Source::Band => "Band",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            Source::Level => "How loud it is (next to how loud it was lately)",
            Source::Bass => "How much low end there is, below about 150 Hz",
            Source::Mids => "How much there is in the middle: voices, chords, snares",
            Source::Highs => "How much top there is, above about 3 kHz: hats, cymbals, air",
            Source::Kick => "A hit in the low end (a kick drum), falling away after it",
            Source::Hits => "Any hit: something starting suddenly anywhere, falling away after it",
            Source::Notes => "A note starting (as hard as it is played), falling away after it. Of one track or of all",
            Source::Pitch => "The note played last, round the octave: C at the start, B near the end (with Hue: a colour for every note)",
            Source::Beat => "Every beat of the song (while it plays), falling away after it",
            Source::Bar => "Every bar of the song (while it plays), falling away after it",
            Source::Ramp => "Through the bar from 0 to 1, every bar again (with Spin or Hue: round once a bar)",
            Source::Fixed => "Always full: the effect stays at the link's amount, whatever the music (a look of its own)",
            Source::Lfo => "An LFO in time with the song: its shape, and a cycle of so many beats",
            Source::Band => "How loud a band of its own is (from and to, in Hz), next to how loud it was lately",
        }
    }

    /// Whether it hears a sound (the master's, or a track's).
    pub fn listens(self) -> bool {
        matches!(self, Source::Level | Source::Bass | Source::Mids | Source::Highs | Source::Kick | Source::Hits | Source::Band)
    }

    /// Whether it follows a track's notes.
    pub fn follows_notes(self) -> bool {
        matches!(self, Source::Notes | Source::Pitch)
    }

    /// Whether it can come from one track (else it is the song's).
    pub fn has_track(self) -> bool {
        self.listens() || self.follows_notes()
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

/// What a source can do to the picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Effect {
    /// Brighter, to white.
    Flash,
    /// Closer in.
    Zoom,
    /// Shaken about.
    Shake,
    /// Turned (all the way round at full).
    Spin,
    /// Red, green and blue pulled apart.
    Split,
    /// The colours turned round the colour circle.
    Hue,
    /// Coloured in the track's colour.
    Tint,
    /// In big pixels.
    Pixels,
    /// In few colours, as a poster.
    Poster,
    /// The left half mirrored onto the right.
    Mirror,
    /// A kaleidoscope.
    Kaleido,
    /// What was there stays a while, fading.
    Trails,
    /// Negative.
    Invert,
    /// Strips of the picture pushed sideways.
    Glitch,
    /// Flashing white, sixteen times a bar.
    Strobe,
    /// Dark, unless the source is up: the picture comes with the sound.
    Gate,
    /// Plays faster.
    Speed,
    /// Stands still.
    Freeze,
    /// Jumps to another slice on a hit.
    Jump,
    /// Repeats a sixteenth note of the film on a hit, for a beat.
    Stutter,
    /// Pulled back and let go, as a record under a hand.
    Scratch,
    /// The next clip of the bank, on a hit.
    NextClip,
    /// Soft, out of focus.
    Blur,
    /// Darker towards the corners.
    Vignette,
    /// The lines of an old screen, curved like its glass.
    Scanlines,
    /// Film grain.
    Grain,
    /// Brightness, contrast and saturation.
    Grade,
    /// Only the outlines.
    Edges,
    /// Streaks out from a point, as when zooming in during the exposure.
    ZoomBlur,
    /// The last frame again under this one, moved a little: an echo.
    Echo,
}

impl Effect {
    /// In the order the lists show them: where the picture is, how it looks, its colours, what it leaves behind, the film.
    pub const ALL: [Effect; 30] = [
        Effect::Zoom,
        Effect::Flash,
        Effect::Shake,
        Effect::Spin,
        Effect::Split,
        Effect::Glitch,
        Effect::Hue,
        Effect::Tint,
        Effect::Grade,
        Effect::Pixels,
        Effect::Poster,
        Effect::Blur,
        Effect::ZoomBlur,
        Effect::Edges,
        Effect::Mirror,
        Effect::Kaleido,
        Effect::Vignette,
        Effect::Scanlines,
        Effect::Grain,
        Effect::Trails,
        Effect::Echo,
        Effect::Invert,
        Effect::Strobe,
        Effect::Gate,
        Effect::Speed,
        Effect::Freeze,
        Effect::Jump,
        Effect::Stutter,
        Effect::Scratch,
        Effect::NextClip,
    ];
    pub const COUNT: usize = 30;

    /// Its place in [`Look::fx`].
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Effect::Flash => "Flash",
            Effect::Zoom => "Zoom",
            Effect::Shake => "Shake",
            Effect::Spin => "Spin",
            Effect::Split => "RGB Split",
            Effect::Hue => "Hue",
            Effect::Tint => "Tint",
            Effect::Pixels => "Pixels",
            Effect::Poster => "Poster",
            Effect::Mirror => "Mirror",
            Effect::Kaleido => "Kaleido",
            Effect::Trails => "Trails",
            Effect::Invert => "Invert",
            Effect::Glitch => "Glitch",
            Effect::Strobe => "Strobe",
            Effect::Gate => "Gate",
            Effect::Speed => "Speed",
            Effect::Freeze => "Freeze",
            Effect::Jump => "Jump",
            Effect::Stutter => "Stutter",
            Effect::Scratch => "Scratch",
            Effect::NextClip => "Next Clip",
            Effect::Blur => "Blur",
            Effect::Vignette => "Vignette",
            Effect::Scanlines => "Scanlines",
            Effect::Grain => "Grain",
            Effect::Grade => "Grade",
            Effect::Edges => "Edges",
            Effect::ZoomBlur => "Zoom Blur",
            Effect::Echo => "Echo",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            Effect::Flash => "Brighter, towards white",
            Effect::Zoom => "Closer in",
            Effect::Shake => "Shaken about",
            Effect::Spin => "Turned, all the way round at full",
            Effect::Split => "Red, green and blue pulled apart",
            Effect::Hue => "The colours turned round the colour circle",
            Effect::Tint => "Coloured in the colour of the source's track",
            Effect::Pixels => "In big pixels",
            Effect::Poster => "In few colours, as a poster",
            Effect::Mirror => "The left half mirrored onto the right",
            Effect::Kaleido => "A kaleidoscope of six",
            Effect::Trails => "What was there stays a while, fading",
            Effect::Invert => "Negative",
            Effect::Glitch => "Strips of the picture pushed sideways",
            Effect::Strobe => "Flashing white, sixteen times a bar",
            Effect::Gate => "Dark unless the source is up: the picture comes with the sound",
            Effect::Speed => "Plays faster (up to four times)",
            Effect::Freeze => "Stands still",
            Effect::Jump => "Cuts to another slice of the clip on a hit (the amount: how often)",
            Effect::Stutter => "Repeats a sixteenth note of the film on a hit, for a beat (the amount: how often)",
            Effect::Scratch => "Pulled back up to a beat and let go, as a record under a hand",
            Effect::NextClip => "The next clip of the bank on a hit (the amount: how often)",
            Effect::Blur => "Soft, out of focus",
            Effect::Vignette => "Darker towards the corners",
            Effect::Scanlines => "The lines of an old screen, its glass curved",
            Effect::Grain => "Film grain, moving every frame",
            Effect::Grade => "Brightness, contrast and saturation, as its settings say",
            Effect::Edges => "Only the outlines, lit",
            Effect::ZoomBlur => "Streaks out from a point, as when zooming during the exposure",
            Effect::Echo => "The last frame again under this one, moved a little, fading",
        }
    }

    /// Its own settings, besides how much of it there is (see [`FxSetting`]).
    pub fn params(self) -> &'static [FxParam] {
        const CENTER_X: FxParam = FxParam::range("Center X", "Where it is centred, across (0: the middle)", -0.5, 0.5, 0.0);
        const CENTER_Y: FxParam = FxParam::range("Center Y", "Where it is centred, up and down (0: the middle)", -0.5, 0.5, 0.0);
        const RATE: FxParam = FxParam::range("Rate", "How many times a second it changes (60: every frame)", 1.0, 60.0, 60.0);
        match self {
            Effect::Zoom => {
                const P: &[FxParam] = &[FxParam::range("Depth", "How far in at full (1.5: two and a half times as close)", 0.1, 4.0, 1.5), CENTER_X, CENTER_Y, FxParam::range("Ease", "How it comes in: above 1 late and sudden, below 1 early and soft", 0.25, 4.0, 1.0)];
                P
            }
            Effect::Flash => {
                const P: &[FxParam] = &[FxParam::range("Strength", "How near its colour it goes at full", 0.0, 1.0, 0.85)];
                P
            }
            Effect::Shake => {
                const P: &[FxParam] = &[FxParam::range("Size", "How far it is shaken at full (of the screen)", 0.0, 0.2, 0.05), RATE, FxParam::range("Across", "How much it goes sideways", 0.0, 1.0, 1.0), FxParam::range("Up and down", "How much it goes up and down", 0.0, 1.0, 1.0)];
                P
            }
            Effect::Spin => {
                const P: &[FxParam] = &[FxParam::range("Turns", "How far round at full (1: all the way, below 0 the other way)", -2.0, 2.0, 1.0), FxParam::range("Angle", "Turned this far always (a turn: 1)", -0.5, 0.5, 0.0), FxParam::range("Speed", "Turning on by itself while it is up: turns a bar", -4.0, 4.0, 0.0)];
                P
            }
            Effect::Split => {
                const P: &[FxParam] = &[FxParam::range("Size", "How far red and blue are pulled apart at full", 0.0, 0.1, 0.03), FxParam::range("Angle", "Which way they are pulled (0: sideways, 0.25: up and down)", 0.0, 0.5, 0.0)];
                P
            }
            Effect::Glitch => {
                const P: &[FxParam] = &[FxParam::range("Strips", "How many strips the picture is cut in", 2.0, 64.0, 18.0), FxParam::range("Shift", "How far a strip is pushed at full", 0.0, 1.0, 0.4), RATE];
                P
            }
            Effect::Hue => {
                const P: &[FxParam] = &[FxParam::range("Turn", "How far round the colour circle at full (1: all the way)", 0.0, 1.0, 1.0), FxParam::range("Saturation", "How strong the colours are (1: as they are)", 0.0, 2.0, 1.0)];
                P
            }
            Effect::Tint => {
                const P: &[FxParam] = &[FxParam::pick("Blend", "How its colour goes on the picture", &["Colour", "Multiply", "Screen", "Add"], 0.0)];
                P
            }
            Effect::Pixels => {
                const P: &[FxParam] = &[FxParam::range("Size", "How big the pixels get at full (pixels of the screen)", 2.0, 120.0, 40.0)];
                P
            }
            Effect::Poster => {
                const P: &[FxParam] = &[FxParam::range("Levels", "How few colours at full (of each of red, green and blue)", 2.0, 16.0, 2.0)];
                P
            }
            Effect::Mirror => {
                const P: &[FxParam] = &[FxParam::pick("Axis", "Which half is mirrored onto the other", &["Left onto right", "Right onto left", "Top onto bottom", "Bottom onto top", "Four ways"], 0.0)];
                P
            }
            Effect::Kaleido => {
                const P: &[FxParam] = &[FxParam::range("Segments", "How many mirrored wedges", 2.0, 16.0, 6.0), FxParam::range("Turn", "The wedges turned (a turn: 1)", 0.0, 1.0, 0.0)];
                P
            }
            Effect::Trails => {
                const P: &[FxParam] = &[FxParam::range("Drift", "The last frame drawn this much closer (below 1) or further (above 1)", 0.9, 1.1, 0.985), FxParam::range("Keep", "How much of the last frame stays", 0.0, 1.0, 0.9)];
                P
            }
            Effect::Strobe => {
                const P: &[FxParam] = &[FxParam::lengths("Rate", "How often it flashes", &["1/4", "1/8", "1/16", "1/32"], 2.0), FxParam::range("Duty", "How long each flash is lit (of its time)", 0.1, 0.9, 0.5)];
                P
            }
            Effect::Gate => {
                const P: &[FxParam] = &[FxParam::range("Threshold", "The source lifts it only above this", 0.0, 1.0, 0.0), FxParam::range("Softness", "How gradually it opens above the threshold (1: all the way up)", 0.0, 1.0, 1.0)];
                P
            }
            Effect::Speed => {
                const P: &[FxParam] = &[FxParam::range("Top", "How fast at full (times as filmed)", 1.0, 8.0, 4.0)];
                P
            }
            Effect::Stutter => {
                const P: &[FxParam] = &[FxParam::lengths("Length", "How much of the film is repeated", &["1/4", "1/8", "1/16", "1/32"], 2.0)];
                P
            }
            Effect::Scratch => {
                const P: &[FxParam] = &[FxParam::range("Reach", "How far back it is pulled at full (beats)", 0.0, 4.0, 1.0)];
                P
            }
            Effect::Blur => {
                const P: &[FxParam] = &[FxParam::range("Size", "How soft at full", 0.0, 1.0, 0.5)];
                P
            }
            Effect::Vignette => {
                const P: &[FxParam] = &[FxParam::range("Size", "How far in the dark comes at full", 0.0, 1.0, 0.5), FxParam::range("Softness", "How gradually it darkens", 0.05, 1.0, 0.5)];
                P
            }
            Effect::Scanlines => {
                const P: &[FxParam] = &[FxParam::range("Lines", "How many lines down the screen", 60.0, 720.0, 240.0), FxParam::range("Curve", "How much the glass bulges", 0.0, 1.0, 0.3)];
                P
            }
            Effect::Grain => {
                const P: &[FxParam] = &[FxParam::range("Size", "How coarse the grain is (pixels)", 1.0, 8.0, 1.0)];
                P
            }
            Effect::Grade => {
                const P: &[FxParam] = &[FxParam::range("Brightness", "Lighter (above 0) or darker", -1.0, 1.0, 0.0), FxParam::range("Contrast", "1: as it is", 0.0, 3.0, 1.25), FxParam::range("Saturation", "1: as it is, 0: grey", 0.0, 3.0, 1.25)];
                P
            }
            Effect::Edges => {
                const P: &[FxParam] = &[FxParam::range("Thickness", "How wide the outlines are (pixels)", 0.5, 4.0, 1.0), FxParam::pick("Over", "What the outlines are drawn over", &["Black", "The picture"], 0.0)];
                P
            }
            Effect::ZoomBlur => {
                const P: &[FxParam] = &[FxParam::range("Length", "How long the streaks are at full", 0.0, 1.0, 0.5), CENTER_X, CENTER_Y];
                P
            }
            Effect::Echo => {
                const P: &[FxParam] = &[FxParam::range("Across", "How far the echo moves each frame, across", -0.05, 0.05, 0.01), FxParam::range("Up and down", "How far it moves, up and down", -0.05, 0.05, 0.0), FxParam::range("Keep", "How much of the echo stays", 0.0, 1.0, 0.8)];
                P
            }
            Effect::Invert | Effect::Freeze | Effect::Jump | Effect::NextClip => &[],
        }
    }

    /// The colour it has unless one is picked (None: none, or Tint's track colour).
    pub fn color(self) -> Option<[u8; 3]> {
        match self {
            Effect::Flash | Effect::Strobe | Effect::Edges => Some([0xff, 0xff, 0xff]),
            _ => None,
        }
    }

    /// Whether a colour can be picked for it.
    pub fn has_color(self) -> bool {
        self.color().is_some() || self == Effect::Tint
    }

    /// Whether it happens on a hit (the amount is how often), rather than
    /// following the source.
    pub fn on_hits(self) -> bool {
        matches!(self, Effect::Jump | Effect::Stutter | Effect::NextClip)
    }

    /// Whether it changes where the film is, not how it looks.
    pub fn moves_film(self) -> bool {
        matches!(self, Effect::Speed | Effect::Freeze | Effect::Jump | Effect::Stutter | Effect::Scratch | Effect::NextClip)
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

/// One of an effect's own settings: a range, or a choice among a few.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FxParam {
    pub name: &'static str,
    pub about: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    /// A choice among these (the value is its place), instead of a range.
    pub choices: &'static [&'static str],
    /// The choices are lengths (clicked through, rather than a list).
    pub length: bool,
}

impl FxParam {
    const fn range(name: &'static str, about: &'static str, min: f32, max: f32, default: f32) -> Self {
        Self { name, about, min, max, default, choices: &[], length: false }
    }

    const fn pick(name: &'static str, about: &'static str, choices: &'static [&'static str], default: f32) -> Self {
        Self { name, about, min: 0.0, max: (choices.len() - 1) as f32, default, choices, length: false }
    }

    const fn lengths(name: &'static str, about: &'static str, choices: &'static [&'static str], default: f32) -> Self {
        Self { name, about, min: 0.0, max: (choices.len() - 1) as f32, default, choices, length: true }
    }

    /// A value put in its range (a choice: a whole place), NaN its default.
    pub fn fit(&self, v: f32) -> f32 {
        if !v.is_finite() {
            return self.default;
        }
        let v = v.clamp(self.min, self.max);
        if self.choices.is_empty() {
            v
        } else {
            v.round()
        }
    }
}

/// How one effect looks in a Look: how much of it there always is, its
/// own settings and its colour.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FxSetting {
    /// How much of it there always is, whatever the music (0..1); the links add to it.
    #[serde(skip_serializing_if = "is_zero")]
    pub base: f32,
    /// Its own settings (see [`Effect::params`]) by place; missing ones are their defaults.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<f32>,
    /// Its colour (None: the effect's own, for Tint the track's).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
}

fn is_zero(v: &f32) -> bool {
    *v == 0.0
}

impl FxSetting {
    /// Setting `k` of effect `e` (its default when not set).
    pub fn param(&self, e: Effect, k: usize) -> f32 {
        let Some(p) = e.params().get(k) else { return 0.0 };
        self.params.get(k).map_or(p.default, |v| p.fit(*v))
    }

    /// Sets setting `k` of effect `e` (the ones before it filled with their defaults).
    pub fn set_param(&mut self, e: Effect, k: usize, v: f32) {
        let defs = e.params();
        if k >= defs.len() {
            return;
        }
        while self.params.len() <= k {
            self.params.push(defs[self.params.len()].default);
        }
        self.params[k] = defs[k].fit(v);
    }

    /// All four settings (unused ones 0), for the shader.
    pub fn params4(&self, e: Effect) -> [f32; 4] {
        [0, 1, 2, 3].map(|k| self.param(e, k))
    }

    /// Fixes what a hand-edited file might hold.
    pub fn sanitize(&mut self, e: Effect) {
        self.base = if self.base.is_finite() { self.base.clamp(0.0, 1.0) } else { 0.0 };
        let defs = e.params();
        self.params.truncate(defs.len());
        for (v, d) in self.params.iter_mut().zip(defs) {
            *v = d.fit(*v);
        }
    }

    /// Nothing set differently from the start.
    pub fn is_plain(&self, e: Effect) -> bool {
        self.base == 0.0 && self.color.is_none() && (0..e.params().len()).all(|k| self.param(e, k) == e.params()[k].default)
    }
}

/// How a link answers its source: smoothed as it rises (attack) and
/// falls (release, seconds), turned upside down, only above a threshold,
/// and bent by a curve. As it starts it answers as the source is.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Response {
    pub attack: f32,
    pub release: f32,
    /// Above 1 it holds back and comes late; below 1 it comes early.
    pub curve: f32,
    pub invert: bool,
    pub threshold: f32,
}

impl Default for Response {
    fn default() -> Self {
        Self { attack: 0.0, release: 0.0, curve: 1.0, invert: false, threshold: 0.0 }
    }
}

impl Response {
    pub fn is_plain(&self) -> bool {
        *self == Self::default()
    }

    /// The source's value `v` (0..1) as the link takes it: inverted, over
    /// the threshold, bent.
    pub fn shape(&self, v: f32) -> f32 {
        let mut v = if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
        if self.invert {
            v = 1.0 - v;
        }
        let th = self.threshold.clamp(0.0, 0.99);
        if th > 0.0 {
            v = ((v - th) / (1.0 - th)).max(0.0);
        }
        let c = if self.curve.is_finite() { self.curve.clamp(0.1, 10.0) } else { 1.0 };
        if c != 1.0 {
            v = v.powf(c);
        }
        v
    }

    /// From `was` towards `v` over `dt` seconds, as fast as its attack or release says.
    pub fn smooth(&self, was: f32, v: f32, dt: f32) -> f32 {
        let t = if v > was { self.attack } else { self.release };
        if !(t > 0.0) || !was.is_finite() {
            return v;
        }
        was + (v - was) * (1.0 - (-dt.max(0.0) / t).exp())
    }

    pub fn sanitize(&mut self) {
        let ok = |v: f32, lo: f32, hi: f32, d: f32| if v.is_finite() { v.clamp(lo, hi) } else { d };
        self.attack = ok(self.attack, 0.0, 4.0, 0.0);
        self.release = ok(self.release, 0.0, 4.0, 0.0);
        self.curve = ok(self.curve, 0.1, 10.0, 1.0);
        self.threshold = ok(self.threshold, 0.0, 0.99, 0.0);
    }
}

/// The shape of an LFO.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LfoShape {
    Sine,
    Triangle,
    /// Up, then down at once.
    Saw,
    /// Down, then up at once.
    Ramp,
    Square,
    /// A new value by chance every cycle, held.
    Random,
}

impl LfoShape {
    pub const ALL: [LfoShape; 6] = [LfoShape::Sine, LfoShape::Triangle, LfoShape::Saw, LfoShape::Ramp, LfoShape::Square, LfoShape::Random];

    pub fn name(self) -> &'static str {
        match self {
            LfoShape::Sine => "Sine",
            LfoShape::Triangle => "Triangle",
            LfoShape::Saw => "Saw up",
            LfoShape::Ramp => "Saw down",
            LfoShape::Square => "Square",
            LfoShape::Random => "Random",
        }
    }
}

/// An LFO in time with the song: a cycle every `beats`, started `phase` in.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Lfo {
    pub shape: LfoShape,
    pub beats: f32,
    pub phase: f32,
}

impl Default for Lfo {
    fn default() -> Self {
        Self { shape: LfoShape::Sine, beats: 4.0, phase: 0.0 }
    }
}

/// The cycle lengths an LFO offers (beats).
pub const LFO_BEATS: [f32; 9] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0];

impl Lfo {
    /// Its value (0..1) at song position `beat`, and the cycle it is in.
    pub fn at(&self, beat: f64) -> (f32, i64) {
        let len = if self.beats.is_finite() { self.beats.clamp(1.0 / 16.0, 256.0) } else { 4.0 } as f64;
        let x = beat / len + self.phase as f64;
        let cycle = x.floor() as i64;
        let f = (x - x.floor()) as f32;
        let v = match self.shape {
            LfoShape::Sine => 0.5 - 0.5 * (f * std::f32::consts::TAU).cos(),
            LfoShape::Triangle => 1.0 - (f * 2.0 - 1.0).abs(),
            LfoShape::Saw => f,
            LfoShape::Ramp => 1.0 - f,
            LfoShape::Square => {
                if f < 0.5 {
                    1.0
                } else {
                    0.0
                }
            }
            LfoShape::Random => {
                let mut d = Dice::new((cycle as u32).wrapping_mul(0x9E37_79B9) ^ 0x51ED);
                d.next();
                d.unit()
            }
        };
        (v, cycle)
    }
}

/// A source moving an effect: from `low` (the source at 0) to `amount`
/// (at full), as its response says.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VideoLink {
    pub source: Source,
    /// The track it listens to or follows (None: the master, or every track's notes).
    #[serde(default)]
    pub from: Option<u64>,
    pub effect: Effect,
    pub amount: f32,
    /// Where the effect is with the source at 0 (the range's other end).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub low: f32,
    #[serde(default, skip_serializing_if = "Response::is_plain")]
    pub response: Response,
    /// The LFO's shape and length (Source::Lfo).
    #[serde(default, skip_serializing_if = "is_default_lfo")]
    pub lfo: Lfo,
    /// The band heard (Hz, from and to; Source::Band).
    #[serde(default = "default_band", skip_serializing_if = "is_default_band")]
    pub band: (f32, f32),
}

fn is_default_lfo(l: &Lfo) -> bool {
    *l == Lfo::default()
}

fn default_band() -> (f32, f32) {
    (200.0, 2000.0)
}

fn is_default_band(b: &(f32, f32)) -> bool {
    *b == default_band()
}

impl VideoLink {
    pub fn new(source: Source, effect: Effect, amount: f32) -> Self {
        Self { source, from: None, effect, amount, low: 0.0, response: Response::default(), lfo: Lfo::default(), band: default_band() }
    }

    pub fn sanitize(&mut self) {
        let ok = |v: f32, d: f32| if v.is_finite() { v.clamp(0.0, 1.0) } else { d };
        self.amount = ok(self.amount, 0.5);
        self.low = ok(self.low, 0.0);
        self.response.sanitize();
        if !self.lfo.beats.is_finite() || self.lfo.beats <= 0.0 {
            self.lfo.beats = 4.0;
        }
        self.lfo.phase = if self.lfo.phase.is_finite() { self.lfo.phase.rem_euclid(1.0) } else { 0.0 };
        let (a, b) = self.band;
        let hz = |v: f32, d: f32| if v.is_finite() { v.clamp(20.0, 20000.0) } else { d };
        let (a, b) = (hz(a, 200.0), hz(b, 2000.0));
        self.band = (a.min(b), a.max(b).max(a.min(b) * 1.05));
    }
}

/// A ready set of links.
pub struct Preset {
    pub name: &'static str,
    pub about: &'static str,
    pub play: Option<PlayMode>,
    pub links: Vec<VideoLink>,
}

pub const PRESET_NAMES: [&str; 7] = ["Pulse", "Glitch", "Dream", "Strobe", "Kaleido", "Cutup", "Still"];

/// A preset by name.
pub fn preset(name: &str) -> Option<Preset> {
    use Effect as E;
    use Source as S;
    let l = VideoLink::new;
    let (about, play, links) = match name {
        "Pulse" => ("Jumps in on the kicks, shakes a little on the beat, splits on the highs", None, vec![l(S::Kick, E::Zoom, 0.35), l(S::Kick, E::Flash, 0.3), l(S::Highs, E::Split, 0.3), l(S::Beat, E::Shake, 0.15)]),
        "Glitch" => ("Cuts to another place on the kicks, tears on every hit, pixels with the level", None, vec![l(S::Kick, E::Jump, 0.7), l(S::Hits, E::Glitch, 0.6), l(S::Highs, E::Split, 0.5), l(S::Level, E::Pixels, 0.25), l(S::Bar, E::Invert, 0.5)]),
        "Dream" => ("Slow trails, colours with the notes, breathing with the bass", None, vec![l(S::Level, E::Trails, 0.8), l(S::Pitch, E::Hue, 0.6), l(S::Bass, E::Zoom, 0.15), l(S::Mids, E::Mirror, 0.5)]),
        "Strobe" => ("White on the kicks, negative on the bar, the picture with the sound", None, vec![l(S::Kick, E::Strobe, 0.8), l(S::Bar, E::Invert, 0.8), l(S::Level, E::Gate, 0.6), l(S::Kick, E::Zoom, 0.25)]),
        "Kaleido" => ("A kaleidoscope turning once a bar, colours with the notes", None, vec![l(S::Bass, E::Kaleido, 1.0), l(S::Ramp, E::Spin, 1.0), l(S::Pitch, E::Hue, 0.5), l(S::Kick, E::Zoom, 0.3), l(S::Mids, E::Trails, 0.4)]),
        "Cutup" => ("Played by the notes, slice by slice, as a drum rack of film", Some(PlayMode::Slices), vec![l(S::Notes, E::Flash, 0.25), l(S::Hits, E::Stutter, 0.4), l(S::Kick, E::Zoom, 0.2)]),
        "Still" => ("Nothing moves it: the clip as it is", None, Vec::new()),
        _ => return None,
    };
    Some(Preset { name: PRESET_NAMES.iter().copied().find(|n| *n == name)?, about, play, links })
}

// ----------------------------------------------------------------------
// Listening
// ----------------------------------------------------------------------

/// How loud each band is: index into [`Listener::bands`].
const FULL: usize = 0;
const BASS: usize = 1;
const MIDS: usize = 2;
const HIGHS: usize = 3;

/// How far below the loudest lately a band reads as nothing (dB).
const RANGE_DB: f32 = 24.0;

/// Listens to one sound: how loud it is overall and in three bands, and
/// the hits in it (an onset: a band suddenly louder than it just was).
/// Fed with the samples as they come, a few hundred at a time.
pub struct Listener {
    /// Samples in a hop (about 5 ms) and those gathered so far.
    hop: usize,
    filled: usize,
    hop_secs: f32,
    /// One-pole low-passes: two at 150 Hz for the bass, one at 3 kHz.
    a_low: f32,
    a_high: f32,
    low1: f32,
    low2: f32,
    high: f32,
    /// The power of each band, smoothed (over 20 ms for the full sound and
    /// the bass, 10 ms for the rest), and how fast it follows.
    pow: [f32; 4],
    a_pow: [f32; 4],
    /// The same, quicker (8 ms for the full sound and the bass, 4 ms for
    /// the rest), for the hits.
    fast: [f32; 4],
    a_fast: [f32; 4],
    /// The band levels now (dB, falling slowly after a peak), the loudest
    /// lately (falling slower still), and what they were just before.
    env: [f32; 4],
    loudest: [f32; 4],
    slow: [f32; 4],
    /// The hits: falling from 1, and whether one came since `take_hits`.
    kick: f32,
    hits: f32,
    kick_new: bool,
    hit_new: bool,
    since_kick: f32,
    since_hit: f32,
    /// How sharply the low end, the middle and the top rose since `take_rises`.
    rises: Rises,
}

impl Listener {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(8000.0);
        let hop = ((sr / 187.5).round() as usize).max(32);
        let coef = |hz: f32| 1.0 - (-2.0 * std::f32::consts::PI * hz / sr).exp();
        Self {
            hop,
            filled: 0,
            hop_secs: hop as f32 / sr,
            a_low: coef(150.0),
            a_high: coef(3000.0),
            low1: 0.0,
            low2: 0.0,
            high: 0.0,
            pow: [0.0; 4],
            a_pow: [0.02, 0.02, 0.01, 0.01].map(|t: f32| 1.0 - (-1.0 / (t * sr)).exp()),
            fast: [0.0; 4],
            a_fast: [0.008, 0.008, 0.004, 0.004].map(|t: f32| 1.0 - (-1.0 / (t * sr)).exp()),
            env: [-120.0; 4],
            loudest: [-60.0; 4],
            slow: [-120.0; 4],
            kick: 0.0,
            hits: 0.0,
            kick_new: false,
            hit_new: false,
            since_kick: 1.0,
            since_hit: 1.0,
            rises: Rises::default(),
        }
    }

    /// Takes in a run of (mono) samples.
    pub fn feed(&mut self, samples: &[f32]) {
        for &x in samples {
            let x = if x.is_finite() { x } else { 0.0 };
            self.low1 += self.a_low * (x - self.low1);
            self.low2 += self.a_low * (self.low1 - self.low2);
            self.high += self.a_high * (x - self.high);
            let bass = self.low2;
            let highs = x - self.high;
            let mids = self.high - self.low2;
            for (b, v) in [x, bass, mids, highs].into_iter().enumerate() {
                self.pow[b] += self.a_pow[b] * (v * v - self.pow[b]);
                self.fast[b] += self.a_fast[b] * (v * v - self.fast[b]);
            }
            self.filled += 1;
            if self.filled >= self.hop {
                self.end_hop();
            }
        }
    }

    fn end_hop(&mut self) {
        let dt = self.hop_secs;
        let mut flux = 0.0;
        let mut db = [0.0f32; 4];
        // The quick level, for the hits.
        let quick = self.fast.map(|p| 10.0 * (p + 1e-12).log10());
        for b in 0..4 {
            db[b] = 10.0 * (self.pow[b] + 1e-12).log10();
            // Up at once, down 48 dB a second.
            self.env[b] = db[b].max(self.env[b] - 48.0 * dt);
            // The loudest lately: down 3 dB a second, never under -60.
            self.loudest[b] = db[b].max(self.loudest[b] - 3.0 * dt).max(-60.0);
            if b != FULL {
                flux += (quick[b] - self.slow[b]).max(0.0);
            }
        }
        self.filled = 0;
        self.since_kick += dt;
        self.since_hit += dt;
        // How sharply each band rises, when it is loud enough to matter (for the triggers).
        for (k, b) in [BASS, MIDS, HIGHS].into_iter().enumerate() {
            if quick[b] > self.loudest[b] - 12.0 && quick[b] > -50.0 {
                self.rises.rise[k] = self.rises.rise[k].max(quick[b] - self.slow[b]);
            }
        }
        // A kick: the bass 6 dB over what it just was, loud enough, not just
        // after the last. (A bass note after a rest is one too: listening to
        // the drum track alone hears only the kicks.)
        let rise = quick[BASS] - self.slow[BASS];
        if rise > 6.0 && quick[BASS] > self.loudest[BASS] - 12.0 && quick[BASS] > -50.0 && self.since_kick > 0.09 {
            self.kick = 1.0;
            self.kick_new = true;
            self.since_kick = 0.0;
        }
        // Any hit: the bands together 9 dB over what they just were.
        if flux > 9.0 && quick[FULL] > self.loudest[FULL] - 24.0 && quick[FULL] > -50.0 && self.since_hit > 0.06 {
            self.hits = 1.0;
            self.hit_new = true;
            self.since_hit = 0.0;
        }
        // What it just was: following a rise within about 25 ms and a fall
        // within 20 ms (so a hit after a short rest stands out), and never
        // far under the loudest lately (so the long climb out of silence is
        // one hit, not several).
        let (up, down) = (1.0 - (-dt / 0.025).exp(), 1.0 - (-dt / 0.02).exp());
        for b in 0..4 {
            let a = if quick[b] > self.slow[b] { up } else { down };
            self.slow[b] += a * (quick[b] - self.slow[b]);
            self.slow[b] = self.slow[b].max(self.loudest[b] - 45.0);
        }
        self.kick *= half_life(dt, 0.12);
        self.hits *= half_life(dt, 0.09);
    }

    /// A band from 0 (more than [`RANGE_DB`] under the loudest lately, or
    /// silence) to 1 (as loud as lately).
    fn band(&self, b: usize) -> f32 {
        let v = (self.env[b] - (self.loudest[b] - RANGE_DB)) / RANGE_DB;
        // Near silence it fades out, however loud it was lately.
        let floor = ((self.env[b] + 70.0) / 20.0).clamp(0.0, 1.0);
        v.clamp(0.0, 1.0) * floor
    }

    /// The value of a source it hears (0 for the others).
    pub fn value(&self, s: Source) -> f32 {
        match s {
            Source::Level => self.band(FULL),
            Source::Bass => self.band(BASS),
            Source::Mids => self.band(MIDS),
            Source::Highs => self.band(HIGHS),
            Source::Kick => self.kick,
            Source::Hits => self.hits,
            _ => 0.0,
        }
    }

    /// Whether a kick and whether any hit came since the last call.
    pub fn take_hits(&mut self) -> (bool, bool) {
        let h = (self.kick_new, self.hit_new);
        self.kick_new = false;
        self.hit_new = false;
        h
    }

    /// How sharply the bands rose since the last call (for the hit triggers).
    pub fn take_rises(&mut self) -> Rises {
        // How loud each band is now next to the whole: a kick is mostly low end, a hat mostly top.
        for (k, b) in [BASS, MIDS, HIGHS].into_iter().enumerate() {
            self.rises.share[k] = (self.env[b] - self.env[FULL]).min(0.0);
        }
        std::mem::take(&mut self.rises)
    }
}

/// Listens to a band of its own (a band-pass from `lo` to `hi` Hz): how
/// loud it is next to how loud it was lately, as the Listener's bands are.
pub struct BandListener {
    pub band: (f32, f32),
    coef: [f32; 5],
    z: [f32; 2],
    pow: f32,
    a_pow: f32,
    env: f32,
    loudest: f32,
    hop: usize,
    filled: usize,
    hop_secs: f32,
}

impl BandListener {
    pub fn new(sample_rate: f32, band: (f32, f32)) -> Self {
        let sr = sample_rate.max(8000.0);
        let lo = band.0.clamp(20.0, sr * 0.45);
        let hi = band.1.clamp(lo * 1.05, sr * 0.49);
        // A band-pass round the middle (geometric) of the band, as wide as it is.
        let fc = (lo * hi).sqrt();
        let q = (fc / (hi - lo)).max(0.1);
        let w0 = 2.0 * std::f32::consts::PI * fc / sr;
        let alpha = w0.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        let coef = [alpha / a0, -alpha / a0, -2.0 * w0.cos() / a0, (1.0 - alpha) / a0, 0.0];
        let hop = ((sr / 187.5).round() as usize).max(32);
        Self { band, coef, z: [0.0; 2], pow: 0.0, a_pow: 1.0 - (-1.0 / (0.02 * sr)).exp(), env: -120.0, loudest: -60.0, hop, filled: 0, hop_secs: hop as f32 / sr }
    }

    pub fn feed(&mut self, samples: &[f32]) {
        let [b0, b2, a1, a2, _] = self.coef;
        for &x in samples {
            let x = if x.is_finite() { x } else { 0.0 };
            // Transposed direct form II (b1 is 0 for a band-pass).
            let y = b0 * x + self.z[0];
            self.z[0] = -a1 * y + self.z[1];
            self.z[1] = b2 * x - a2 * y;
            let y = if y.is_finite() { y } else { 0.0 };
            if !self.z[0].is_finite() || !self.z[1].is_finite() {
                self.z = [0.0; 2];
            }
            self.pow += self.a_pow * (y * y - self.pow);
            self.filled += 1;
            if self.filled >= self.hop {
                self.filled = 0;
                let db = 10.0 * (self.pow + 1e-12).log10();
                self.env = db.max(self.env - 48.0 * self.hop_secs);
                self.loudest = db.max(self.loudest - 3.0 * self.hop_secs).max(-60.0);
            }
        }
    }

    /// 0 (far under the loudest lately, or silence) to 1 (as loud as lately).
    pub fn value(&self) -> f32 {
        let v = (self.env - (self.loudest - RANGE_DB)) / RANGE_DB;
        let floor = ((self.env + 70.0) / 20.0).clamp(0.0, 1.0);
        v.clamp(0.0, 1.0) * floor
    }
}

/// What a value keeps of itself over `dt` seconds, halving every `half` seconds.
pub fn half_life(dt: f32, half: f32) -> f32 {
    0.5f32.powf(dt.max(0.0) / half.max(1e-4))
}

/// Follows the notes of a track (or of all): a pulse on every note that
/// starts, as hard as it is played, and the last one's pitch.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoteFollower {
    pub pulse: f32,
    pub pitch: u8,
    /// A note started this frame.
    pub hit: bool,
}

impl NoteFollower {
    /// One frame: the notes that started since the last (a 128-bit mask)
    /// and the loudest of them (0..1).
    pub fn frame(&mut self, struck: u128, velocity: f32, dt: f32) {
        self.pulse *= half_life(dt, 0.12);
        self.hit = struck != 0;
        if struck != 0 {
            // The highest: in a chord, the tune is usually on top.
            self.pitch = 127 - struck.leading_zeros() as u8;
            self.pulse = self.pulse.max(velocity.clamp(0.05, 1.0));
        }
    }

    pub fn value(&self, s: Source) -> f32 {
        match s {
            Source::Notes => self.pulse,
            Source::Pitch => (self.pitch % 12) as f32 / 12.0,
            _ => 0.0,
        }
    }
}

/// The song's beat: a pulse on every beat and bar, and the ramp through the bar.
#[derive(Clone, Copy, Debug, Default)]
pub struct BeatClock {
    last: Option<f64>,
    pub beat: f32,
    pub bar: f32,
    pub ramp: f32,
    pub beat_hit: bool,
    pub bar_hit: bool,
}

impl BeatClock {
    /// One frame at song position `pos` (in beats).
    pub fn frame(&mut self, pos: f64, playing: bool, beats_per_bar: u32, dt: f32) {
        self.beat *= half_life(dt, 0.1);
        self.bar *= half_life(dt, 0.15);
        self.beat_hit = false;
        self.bar_hit = false;
        let bpb = beats_per_bar.max(1) as f64;
        if !playing {
            self.last = None;
            return;
        }
        self.ramp = ((pos / bpb).rem_euclid(1.0)) as f32;
        // A beat crossed since the last frame (or the song started on one).
        let crossed = match self.last {
            Some(prev) if pos >= prev => pos.floor() > prev.floor(),
            // Started, or jumped back (a loop): on a beat when right at one.
            _ => pos - pos.floor() < 0.05,
        };
        if crossed && self.last != Some(pos) {
            self.beat = 1.0;
            self.beat_hit = true;
            if pos.floor().rem_euclid(bpb) == 0.0 {
                self.bar = 1.0;
                self.bar_hit = true;
            }
        }
        self.last = Some(pos);
    }

    pub fn value(&self, s: Source) -> f32 {
        match s {
            Source::Beat => self.beat,
            Source::Bar => self.bar,
            Source::Ramp => self.ramp,
            _ => 0.0,
        }
    }

    /// Whether the source had a hit this frame.
    pub fn hit(&self, s: Source) -> bool {
        match s {
            Source::Beat => self.beat_hit,
            Source::Bar => self.bar_hit,
            _ => false,
        }
    }
}

// ----------------------------------------------------------------------
// The links together
// ----------------------------------------------------------------------

/// A source read for one frame: its value (0..1) and whether it hit.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Reading {
    pub value: f32,
    pub hit: bool,
}

/// What the links make of the picture this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Look {
    /// How much of each effect (0..1), by [`Effect::index`].
    pub fx: [f32; Effect::COUNT],
    /// The effects on hits that happen this frame.
    pub jump: bool,
    pub stutter: bool,
    pub next_clip: bool,
    /// The Tint link that is strongest now (its index in the links).
    pub tint_link: Option<usize>,
    /// How far Spin's Speed has turned it (turns, kept by whoever plays it).
    pub turned: f32,
}

impl Look {
    pub fn get(&self, e: Effect) -> f32 {
        self.fx[e.index()]
    }

    /// Between `a` (at 0) and `b` (at 1): every amount on the way; the hits and the Tint link are `b`'s.
    pub fn morph(a: &Look, b: &Look, t: f32) -> Look {
        let t = t.clamp(0.0, 1.0);
        let mut out = *b;
        for (o, (x, y)) in out.fx.iter_mut().zip(a.fx.iter().zip(b.fx.iter())) {
            *o = x + (y - x) * t;
        }
        out.turned = a.turned + (b.turned - a.turned) * t;
        out
    }
}

/// A small random number generator (xorshift), for the dice of the hits
/// and where a Jump lands.
#[derive(Clone, Copy, Debug)]
pub struct Dice(u32);

impl Default for Dice {
    fn default() -> Self {
        Self(0x9E37_79B9)
    }
}

impl Dice {
    pub fn new(seed: u32) -> Self {
        Self(seed.max(1))
    }

    pub fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// 0..1.
    pub fn unit(&mut self) -> f32 {
        (self.next() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// True with chance `p` (always at 1, never at 0).
    pub fn chance(&mut self, p: f32) -> bool {
        p >= 1.0 || (p > 0.0 && self.unit() < p)
    }
}

/// Every link read and put together, on what the effects always have
/// (their base in `fx`): an effect moved by two links takes the stronger,
/// added to its base. `amount` scales the links (not the bases); at 0 the
/// music moves nothing. A link goes from its `low` (the source at 0) to its
/// `amount` (at full); Gate the other way round, from dark to open.
pub fn look(links: &[VideoLink], fx: &std::collections::BTreeMap<Effect, FxSetting>, amount: f32, dice: &mut Dice, mut read: impl FnMut(&VideoLink) -> Reading) -> Look {
    let mut out = Look::default();
    let amount = amount.clamp(0.0, 1.0);
    let mut tint_best = 0.0;
    let gate = fx.get(&Effect::Gate).cloned().unwrap_or_default();
    let (th, soft) = (gate.param(Effect::Gate, 0), gate.param(Effect::Gate, 1));
    for (i, l) in links.iter().enumerate() {
        let r = read(l);
        let value = r.value.clamp(0.0, 1.0);
        let range = |v: f32| l.low + (l.amount - l.low) * v;
        let v = if l.effect.on_hits() {
            // How often: the chance a hit does it.
            if r.hit && amount > 0.0 && dice.chance(l.amount) {
                match l.effect {
                    Effect::Jump => out.jump = true,
                    Effect::Stutter => out.stutter = true,
                    _ => out.next_clip = true,
                }
                1.0
            } else {
                0.0
            }
        } else if l.effect == Effect::Gate {
            // Dark by the amount, lifted by the source (above its threshold, as softly as it says).
            let open = ((value - th) / (soft * (1.0 - th)).max(1e-4)).clamp(0.0, 1.0);
            range(1.0 - open) * amount
        } else {
            range(value) * amount
        };
        let slot = &mut out.fx[l.effect.index()];
        *slot = slot.max(v.clamp(0.0, 1.0));
        if l.effect == Effect::Tint && v >= tint_best && v > 0.0 {
            tint_best = v;
            out.tint_link = Some(i);
        }
    }
    // What every effect always has, under the links.
    for (e, s) in fx {
        let slot = &mut out.fx[e.index()];
        *slot = (*slot + s.base).clamp(0.0, 1.0);
    }
    out
}

/// The strobe's light now: on and off `per_beat` times a beat (four:
/// sixteen times a bar), lit for `duty` of each, as bright as `amount`.
pub fn strobe_light(amount: f32, beat: f64, per_beat: f32, duty: f32) -> f32 {
    if amount <= 0.0 {
        return 0.0;
    }
    if (beat * per_beat.max(0.01) as f64).rem_euclid(1.0) < duty.clamp(0.0, 1.0) as f64 {
        amount.min(1.0)
    } else {
        0.0
    }
}

// ----------------------------------------------------------------------
// Where in the film
// ----------------------------------------------------------------------

/// The slice a note plays: C1 (36) the first, C#1 the second, round again
/// after the last.
pub fn slice_for_note(pitch: u8, slices: u32) -> usize {
    (pitch as i32 - 36).rem_euclid(slices.max(1) as i32) as usize
}

/// Another slice than `now`, by chance.
pub fn other_slice(dice: &mut Dice, now: usize, slices: u32) -> usize {
    let n = slices.max(1) as usize;
    if n < 2 {
        return 0;
    }
    // One of the others: never the same twice in a row.
    let step = 1 + dice.next() as usize % (n - 1);
    (now % n + step) % n
}

/// The clip of the scene most tracks play now (scene 1 picks the first
/// clip, round again after the last); None when nothing plays.
pub fn scene_clip(playing_slots: impl Iterator<Item = i32>, clips: usize) -> Option<usize> {
    if clips == 0 {
        return None;
    }
    let mut counts: Vec<(i32, usize)> = Vec::new();
    for s in playing_slots.filter(|s| *s >= 0) {
        match counts.iter_mut().find(|c| c.0 == s) {
            Some(c) => c.1 += 1,
            None => counts.push((s, 1)),
        }
    }
    // The most, and of those the first scene.
    let best = counts.iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))?;
    Some(best.0 as usize % clips)
}

/// What moves the film this frame, besides the settings.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlayInput {
    /// Seconds since the last frame.
    pub dt: f32,
    /// The film's length (seconds, all of it); 0 for a still picture.
    pub duration: f32,
    /// The song: where it is (beats), whether it plays, its tempo.
    pub beat: f64,
    pub playing: bool,
    pub bpm: f64,
    pub beats_per_bar: u32,
    /// The links: how much faster (0: as filmed, 3: four times), how much
    /// it stands still (0..1), how far it is pulled back (beats), and the hits.
    pub speed: f32,
    pub freeze: f32,
    pub scratch: f32,
    pub jump: bool,
    pub stutter: bool,
    /// How much of the film a stutter repeats (beats; 0: a sixteenth).
    pub stutter_beats: f32,
    /// Slices: the note that just picked one, or a hit heard that moves to another.
    pub note: Option<u8>,
    pub slice_step: Option<Step>,
    /// The part sequence: a note (from C3) or a hit that picks the part.
    pub part_note: Option<u8>,
    pub part_step: Option<Step>,
    /// Cues: the notes held now on the track that plays them (a 128-bit
    /// mask), so Gate holds the picture where its note ends.
    pub held: u128,
    /// Put the film here (seconds into its parts back to back; a cued
    /// film's own time), as the timeline's playhead is dragged.
    pub seek: Option<f64>,
    /// Paused: the film stands where it is (only `seek` moves it).
    pub hold: bool,
}

/// A stutter going on: the film from `anchor` repeated, `left` seconds more.
#[derive(Clone, Copy, Debug)]
struct Stutter {
    anchor: f64,
    len: f64,
    elapsed: f64,
    left: f64,
}

/// Where the film is, frame by frame. It runs through the clip's parts
/// played back to back (its `pos` counts in them), and says where that is
/// in the film. With a part sequence it runs in one part at a time.
#[derive(Clone, Debug, Default)]
pub struct Playhead {
    /// Where it runs (seconds into the parts), before a stutter or scratch.
    pub pos: f64,
    /// Tempo: how far it is from where the song puts it (after a jump).
    offset: f64,
    /// Slices: the slice the last note picked.
    slice: Option<usize>,
    /// Cues: the note that picked it (None: a jump).
    pitch: Option<u8>,
    stutter: Option<Stutter>,
    /// The part sequence: the part that plays, and the song's step it is on.
    pub part: Option<usize>,
    step: Option<i64>,
    pub dice: Dice,
}

/// Which of `spans` (back to back) holds `t` seconds into them, and how far into it.
fn part_of_joined(spans: &[(f32, f32)], t: f64) -> (usize, f64) {
    let mut acc = 0.0;
    for (k, &(a, b)) in spans.iter().enumerate() {
        let len = (b - a) as f64;
        if t < acc + len {
            return (k, t - acc);
        }
        acc += len;
    }
    let last = spans.len().saturating_sub(1);
    (last, spans.last().map_or(0.0, |s| (s.1 - s.0) as f64))
}

impl Playhead {
    /// Back to the start (a new clip).
    pub fn reset(&mut self) {
        self.pos = 0.0;
        self.offset = 0.0;
        self.slice = None;
        self.pitch = None;
        self.stutter = None;
        self.part = None;
        self.step = None;
    }

    /// The slice that `pos` (into the parts) is in.
    fn slice_at(clip: &VideoClip, spans: &[(f32, f32)], pos: f64) -> usize {
        let n = clip.slice_count(spans) as usize;
        (0..n).find(|&k| pos < clip.slice_bounds(spans, k).1).unwrap_or(n.saturating_sub(1))
    }

    /// The part sequence's part this frame, of `n`: on a new step of the
    /// song as its order says, or as a note or a hit picks. Whether it changed.
    fn pick_part(&mut self, seq: &PartSeq, n: usize, inp: &PlayInput) -> bool {
        let before = self.part;
        if seq.order.on_steps() {
            if inp.playing {
                let step = seq.every.step(inp.beat, inp.beats_per_bar);
                if self.step != Some(step) {
                    self.step = Some(step);
                    let want = match seq.order {
                        PartOrder::Random => Some(match self.part {
                            Some(p) => other_slice(&mut self.dice, p, n as u32),
                            None => self.dice.next() as usize % n,
                        }),
                        _ => seq.part_at(step, n),
                    };
                    if want.is_some() {
                        self.part = want;
                    }
                }
            } else {
                // Stopped: it stays on its part, and is reckoned again from where the song starts.
                self.step = None;
            }
        }
        if seq.order == PartOrder::Notes {
            if let Some(k) = inp.part_note.and_then(|p| part_for_note(p, n)) {
                self.part = Some(k);
            }
        }
        if let (PartOrder::Hits, Some(s)) = (seq.order, inp.part_step) {
            self.part = Some(s.from(self.part.unwrap_or(0), n, &mut self.dice));
        }
        self.part = Some(self.part.unwrap_or(0).min(n - 1));
        before != self.part
    }

    /// One frame of `clip`: moves on and returns where to show the film
    /// (seconds, 0 up to its length).
    pub fn advance(&mut self, clip: &VideoClip, inp: &PlayInput) -> f64 {
        if inp.duration <= 0.0 {
            self.reset();
            return 0.0;
        }
        let play = &clip.play;
        let cues = clip.cues.as_ref().filter(|c| !c.is_empty() && play.mode == PlayMode::Slices);
        // Cues are times in the whole film; the rest plays the parts back to
        // back, or with a part sequence one part at a time.
        let all = clip.spans(inp.duration);
        let mut seek = inp.seek;
        let mut entered = false;
        let spans = if cues.is_none() && play.seq.on && all.len() > 1 {
            if let Some(t) = seek {
                // Dragged somewhere: the part there, from where it was put.
                let (k, local) = part_of_joined(&all, t.rem_euclid(joined_len(&all).max(1e-6) as f64));
                self.part = Some(k);
                seek = Some(local);
            }
            entered = self.pick_part(&play.seq, all.len(), inp);
            vec![all[self.part.unwrap_or(0).min(all.len() - 1)]]
        } else {
            self.part = None;
            self.step = None;
            all
        };
        let dur = if cues.is_some() { inp.duration as f64 } else { joined_len(&spans) as f64 };
        if dur <= 0.0 {
            self.reset();
            return 0.0;
        }
        let to_film = |t: f64| if cues.is_some() { t.clamp(0.0, dur) } else { film_time(&spans, t) };
        let bounds = |k: usize| clip.slice_bounds(&spans, k);
        if entered {
            // Into another part: from its start (or the same slice of it).
            self.offset = 0.0;
            self.stutter = None;
            self.pos = match self.slice.filter(|_| play.mode == PlayMode::Slices) {
                Some(k) => bounds(k).0,
                None if play.reverse => dur - 1e-6,
                None => 0.0,
            };
        }
        // Loop and Tempo show the film from the clip's offset, there and back with Bounce.
        let free = cues.is_none() && play.mode != PlayMode::Slices;
        let bounce = free && play.bounce;
        let lap = if bounce && play.mode == PlayMode::Loop { 2.0 * dur } else { dur };
        let show_of = |pos: f64| {
            let mut s = pos.rem_euclid(lap);
            if bounce && s >= dur {
                s = 2.0 * dur - s;
            }
            if free {
                s += play.offset.max(0.0) as f64;
            }
            s
        };
        if inp.hold {
            if let Some(t) = seek {
                self.seek(clip, &spans, cues, t.rem_euclid(dur), inp);
            }
            self.stutter = None;
            return to_film(show_of(self.pos).rem_euclid(dur));
        }
        let dt = inp.dt.clamp(0.0, 0.25) as f64;
        let dir = if play.reverse { -1.0 } else { 1.0 };
        let beat_secs = 60.0 / inp.bpm.max(1.0);
        let slices = clip.slice_count(&spans).max(1);
        // How much faster the links make it, and how much it stands still.
        let still = if play.freeze { 1.0 } else { inp.freeze.clamp(0.0, 1.0) as f64 };
        let push = (1.0 + inp.speed.clamp(0.0, 16.0) as f64) * (1.0 - still);
        match (play.mode, cues) {
            (PlayMode::Loop, _) => self.pos += dt * play.speed as f64 * push * dir,
            (PlayMode::Tempo, _) => {
                // The whole clip (its parts back to back, or the part that plays) in `bars` bars.
                let len = (play.bars.max(1) * inp.beats_per_bar.max(1)) as f64;
                let rate = dur / (len * beat_secs);
                if inp.playing {
                    // Pinned to the song; the links push it off from there.
                    self.offset += dt * rate * (push - 1.0) * dir;
                    let mut u = (inp.beat / len).rem_euclid(if bounce { 2.0 } else { 1.0 });
                    if u > 1.0 {
                        u = 2.0 - u;
                    }
                    let target = u * dur;
                    self.pos = if play.reverse { dur - target } else { target } + self.offset;
                } else {
                    self.pos += dt * rate * push * dir;
                }
            }
            (PlayMode::Slices, Some(c)) => {
                let k = match (inp.note, inp.slice_step) {
                    (Some(n), _) => Some((c.for_note(n), Some(n))),
                    (None, Some(s)) => Some((s.from(self.slice.unwrap_or_else(|| c.at(self.pos)), c.len(), &mut self.dice), None)),
                    _ => None,
                };
                if let Some((k, pitch)) = k {
                    // The piece of film under the pad's slice of sound, from its start.
                    self.slice = Some(k);
                    self.pitch = pitch;
                    self.pos = c.piece(k, dur).0;
                    self.stutter = None;
                } else if self.slice.is_some() {
                    // As filmed, as the sound plays; Gate stands still once its note ends.
                    let held = self.pitch.is_none_or(|p| inp.held >> (p & 127) & 1 == 1);
                    if !c.gate || held {
                        self.pos += dt * push;
                    }
                } else {
                    // Waiting for a note: on the first cue.
                    self.pos = c.piece(0, dur).0;
                }
            }
            (PlayMode::Slices, None) => {
                let k = match (inp.note, inp.slice_step) {
                    (Some(n), _) => Some(slice_for_note(n, slices)),
                    (None, Some(s)) => Some(s.from(self.slice.unwrap_or_else(|| Self::slice_at(clip, &spans, self.pos.rem_euclid(dur))), slices as usize, &mut self.dice)),
                    _ => None,
                };
                if let Some(k) = k {
                    self.slice = Some(k);
                    let (a, b) = bounds(k);
                    self.pos = if play.reverse { b - 1e-6 } else { a };
                    self.stutter = None;
                } else {
                    self.pos += dt * play.speed as f64 * push * dir;
                }
            }
        }
        if let Some(t) = seek {
            self.seek(clip, &spans, cues, t.rem_euclid(dur), inp);
        }
        if let (true, Some(c)) = (inp.jump, cues) {
            // To the start of another cue.
            let now = self.slice.unwrap_or_else(|| c.at(self.pos));
            let k = other_slice(&mut self.dice, now, c.len() as u32);
            self.slice = Some(k);
            self.pitch = None;
            self.pos = c.piece(k, dur).0;
            self.stutter = None;
        } else if inp.jump {
            let now = self.slice.unwrap_or_else(|| Self::slice_at(clip, &spans, self.pos.rem_euclid(dur)));
            let k = other_slice(&mut self.dice, now, slices);
            let to = bounds(k).0;
            if play.mode == PlayMode::Slices {
                self.slice = Some(k);
            }
            if play.mode == PlayMode::Tempo && inp.playing {
                self.offset += to - self.pos;
            }
            self.pos = to;
            self.stutter = None;
        }
        match (self.slice, cues) {
            // No further than its piece: the sound has stopped there too.
            (Some(k), Some(c)) => {
                let (a, b) = c.piece(k, dur);
                self.pos = self.pos.clamp(a, (b - 1e-3).max(a));
            }
            // Round and round in its slice.
            (Some(k), None) if play.mode == PlayMode::Slices => {
                let (a, b) = bounds(k);
                if b > a {
                    self.pos = a + (self.pos - a).rem_euclid(b - a);
                }
            }
            _ => {}
        }
        self.pos = self.pos.rem_euclid(lap);
        self.offset = self.offset.rem_euclid(dur);
        // A stutter: a sixteenth note of film (or as long as it says), over and over for a beat.
        if inp.stutter {
            let beats = if inp.stutter_beats > 0.0 { inp.stutter_beats as f64 } else { 0.25 };
            let len = (beat_secs * beats).min(dur);
            self.stutter = Some(Stutter { anchor: show_of(self.pos), len, elapsed: 0.0, left: beat_secs });
        }
        let mut show = show_of(self.pos);
        if let Some(s) = &mut self.stutter {
            s.elapsed += dt;
            s.left -= dt;
            if s.left <= 0.0 {
                self.stutter = None;
            } else {
                show = s.anchor + s.elapsed.rem_euclid(s.len);
            }
        }
        // Scratch: pulled back by as many beats of film as it says.
        show -= inp.scratch.clamp(0.0, 16.0) as f64 * beat_secs * dir;
        to_film(show.rem_euclid(dur))
    }

    /// Puts it at `t` (into the parts, or a cued film's own time), in the
    /// slice or cue there.
    fn seek(&mut self, clip: &VideoClip, spans: &[(f32, f32)], cues: Option<&Cues>, t: f64, inp: &PlayInput) {
        // Shown from the clip's offset: put where it shows `t`.
        let t = if cues.is_none() && clip.play.mode != PlayMode::Slices { (t - clip.play.offset.max(0.0) as f64).rem_euclid(joined_len(spans).max(1e-6) as f64) } else { t };
        if clip.play.mode == PlayMode::Tempo && inp.playing {
            self.offset += t - self.pos;
        }
        self.pos = t;
        self.stutter = None;
        match cues {
            Some(c) => {
                self.slice = Some(c.at(t));
                self.pitch = None;
            }
            None if clip.play.mode == PlayMode::Slices => self.slice = Some(Self::slice_at(clip, spans, t)),
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    #[test]
    fn a_clip_plays_its_parts_back_to_back_and_skips_the_gaps() {
        let mut c = VideoClip::new("a.mp4".into());
        // None: the whole film.
        assert_eq!(c.spans(10.0), vec![(0.0, 10.0)]);
        assert!(!c.is_cut());
        c.add_part(6.0, 8.0);
        c.add_part(1.0, 2.0);
        // In the film's order whatever the order they came in.
        assert_eq!(c.parts, vec![(1.0, 2.0), (6.0, 8.0)]);
        assert!(c.is_cut());
        assert!((c.joined_len(10.0) - 3.0).abs() < 1e-6);
        // 0.5 s in: in the first part; 1.5 s in: half a second into the second (the gap skipped).
        assert!((c.film_time(0.5, 10.0) - 1.5).abs() < 1e-9);
        assert!((c.film_time(1.0, 10.0) - 6.0).abs() < 1e-9, "the second part starts where the first ends");
        assert!((c.film_time(1.5, 10.0) - 6.5).abs() < 1e-9);
        // Round again after the last.
        assert!((c.film_time(3.25, 10.0) - 1.25).abs() < 1e-9);
        // And back: a gap is in no part.
        assert_eq!(c.joined_time(6.5, 10.0), Some(1.5));
        assert_eq!(c.joined_time(4.0, 10.0), None);
        assert_eq!((c.part_at(1.5), c.part_at(4.0), c.part_at(7.9)), (Some(0), None, Some(1)));
        // A part past the film's end is cut there; one wholly past it plays nothing.
        c.add_part(9.0, 30.0);
        assert_eq!(c.spans(10.0), vec![(1.0, 2.0), (6.0, 8.0), (9.0, 10.0)]);
        assert_eq!(c.spans(5.0), vec![(1.0, 2.0)]);
        // A cued film plays whole, by its cues.
        let mut cued = c.clone();
        cued.cues = Some(Cues { times: vec![0.0, 1.0], end: 2.0, track: None, gate: false });
        assert_eq!(cued.spans(10.0), vec![(0.0, 10.0)]);
        assert!(!cued.is_cut());
    }

    #[test]
    fn parts_are_put_in_order_joined_where_they_overlap_and_tiny_ones_dropped() {
        let mut parts = vec![(5.0, 7.0), (6.5, 9.0), (2.0, 1.0), (3.0, 3.05), (-1.0, 0.5), (f32::NAN, 2.0), (9.0, 9.5)];
        normalize_parts(&mut parts);
        // Turned the right way, joined where they overlap (touching ones stay two), a twentieth of a second gone.
        assert_eq!(parts, vec![(0.0, 0.5), (1.0, 2.0), (5.0, 9.0), (9.0, 9.5)]);
        // A part as short as can be stays; split in two only where both halves can be kept.
        let mut c = VideoClip::new("a.mp4".into());
        c.add_part(2.0, 2.0 + MIN_PART);
        assert_eq!(c.parts.len(), 1);
        c.parts = vec![(0.0, 4.0)];
        assert!(!c.split_part(0.05), "too near the start");
        assert!(!c.split_part(5.0), "in no part");
        assert!(c.split_part(1.5));
        assert_eq!(c.parts, vec![(0.0, 1.5), (1.5, 4.0)]);
        // Another part kept later does not join them again.
        c.add_part(6.0, 7.0);
        assert_eq!(c.parts, vec![(0.0, 1.5), (1.5, 4.0), (6.0, 7.0)]);
        // Tiny parts of a film keep playing (back to back, no time lost).
        c.parts = vec![(1.0, 1.1), (3.0, 3.1), (5.0, 5.1)];
        assert!((c.joined_len(10.0) - 0.3).abs() < 1e-5);
        assert!((c.film_time(0.15, 10.0) - 3.05).abs() < 1e-5);
        assert!((c.film_time(0.299, 10.0) - 5.099).abs() < 1e-4);
        // Too many: no more than MAX_PARTS.
        let mut many: Vec<(f32, f32)> = (0..100).map(|k| (k as f32, k as f32 + 0.5)).collect();
        normalize_parts(&mut many);
        assert_eq!(many.len(), MAX_PARTS);
    }

    #[test]
    fn older_sets_keep_their_part_and_how_they_played() {
        // From before parts: start and end make one part; start alone runs to the end.
        let old: VideoClip = serde_json::from_str(r#"{"path":"c.mp4","name":"c","start":2.0,"end":6.5}"#).unwrap();
        assert_eq!(old.parts, vec![(2.0, 6.5)]);
        let open: VideoClip = serde_json::from_str(r#"{"path":"c.mp4","name":"c","start":3.0}"#).unwrap();
        assert_eq!(open.spans(8.0), vec![(3.0, 8.0)]);
        let whole: VideoClip = serde_json::from_str(r#"{"path":"c.mp4","name":"c"}"#).unwrap();
        assert!(whole.parts.is_empty());
        // From before every clip played its own way: each gets the set's.
        let set: VideoSet = serde_json::from_str(r#"{"clips":[{"path":"a.mp4","name":"a"},{"path":"b.gif","name":"b","start":1.0,"end":2.0}],"play":"Tempo","bars":8,"speed":2.0,"slices":4,"slice_track":7,"reverse":true}"#).unwrap();
        for c in &set.clips {
            assert_eq!(c.play, ClipPlay { mode: PlayMode::Tempo, speed: 2.0, bars: 8, slices: 4, slice_track: Some(7), reverse: true, part_slices: false, ..Default::default() });
        }
        assert_eq!(set.clips[1].parts, vec![(1.0, 2.0)]);
        // Switching as before: at once, a cut.
        assert_eq!((set.launch, set.transition), (Launch::Now, Transition::Cut));
        // Written again, it reads the same, and nothing of the old kind is written.
        let json = serde_json::to_string(&set).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v.get("play").is_none() && v.get("bars").is_none() && v["clips"][1].get("start").is_none(), "{json}");
        let back: VideoSet = serde_json::from_str(&json).unwrap();
        assert_eq!(back, set);
        // A clip of its own keeps its own, whatever the set said.
        let mixed: VideoSet = serde_json::from_str(r#"{"clips":[{"path":"a.mp4","name":"a","play":{"mode":"Slices"}}],"play":"Tempo"}"#).unwrap();
        assert_eq!((mixed.clips[0].play.mode, mixed.clips[0].play.speed), (PlayMode::Slices, 1.0));
    }

    #[test]
    fn a_clicked_clip_waits_for_its_beat_or_bar() {
        // Now, or the song stopped: at once.
        assert_eq!(launch_beat(Launch::Now, 5.3, true, 4), None);
        assert_eq!(launch_beat(Launch::Bar, 5.3, false, 4), None);
        // The next beat, the next bar (of three beats in 3/4).
        assert_eq!(launch_beat(Launch::Beat, 5.3, true, 4), Some(6.0));
        assert_eq!(launch_beat(Launch::Bar, 5.3, true, 4), Some(8.0));
        assert_eq!(launch_beat(Launch::Bar, 5.3, true, 3), Some(6.0));
        // Just after the beat (a click a hair late) still makes it.
        assert_eq!(launch_beat(Launch::Bar, 8.02, true, 4), None);
        assert_eq!(launch_beat(Launch::Bar, 8.2, true, 4), Some(12.0));
        // The switch waits, then changes on its beat.
        let mut sw = Switch::default();
        assert!(!sw.ask(2, 5.3, launch_beat(Launch::Bar, 5.3, true, 4), 0.0));
        assert_eq!(sw.clip, 0);
        assert!(!sw.frame(7.9, true, 0.016, 0.0));
        assert!(sw.frame(8.01, true, 0.016, 0.0));
        assert_eq!((sw.clip, sw.pending), (2, None));
        // Stopping (or jumping back past where it was asked) starts it at once.
        sw.ask(1, 9.5, Some(12.0), 0.0);
        assert!(sw.frame(0.0, true, 0.016, 0.0), "the song went back to the start");
        assert_eq!(sw.clip, 1);
        sw.ask(3, 1.0, Some(4.0), 0.0);
        assert!(sw.frame(1.1, false, 0.016, 0.0));
        // The same clip again is no change.
        assert!(!sw.go(3, 0.0));
    }

    #[test]
    fn a_transition_lasts_its_beats_and_then_is_over() {
        let set = VideoSet { transition: Transition::Fade, transition_beats: 1.0, ..Default::default() };
        // A beat at 120 BPM: half a second.
        let secs = set.transition_secs(120.0);
        assert!((secs - 0.5).abs() < 1e-9);
        assert_eq!(VideoSet { transition: Transition::Cut, ..set.clone() }.transition_secs(120.0), 0.0);
        let mut sw = Switch::default();
        assert!(sw.go(1, secs));
        let f = sw.fade.unwrap();
        assert_eq!((f.from, f.mix()), (0, 0.0));
        sw.frame(0.0, true, 0.25, secs);
        assert!((sw.fade.unwrap().mix() - 0.5).abs() < 1e-6);
        sw.frame(0.0, true, 0.2, secs);
        assert!((sw.fade.unwrap().mix() - 0.9).abs() < 1e-6);
        sw.frame(0.0, true, 0.1, secs);
        assert_eq!(sw.fade, None, "over after its half second");
        // A cut has none.
        assert!(sw.go(0, 0.0));
        assert_eq!(sw.fade, None);
        // Clicked on the bar: the fade starts on the bar.
        sw.ask(1, 0.5, Some(4.0), secs);
        sw.frame(3.0, true, 0.1, secs);
        assert_eq!(sw.fade, None);
        sw.frame(4.0, true, 0.1, secs);
        assert_eq!(sw.fade.map(|f| (f.from, f.mix())), Some((0, 0.0)));
    }

    /// Four bars of a four-on-the-floor kick (a falling 55 Hz thump) with a
    /// short burst of hiss (a hat) between the kicks, at 120 BPM.
    fn kicks_and_hats(beats: usize) -> Vec<f32> {
        let beat = (SR * 0.5) as usize;
        let mut out = vec![0.0f32; beat * beats];
        let mut noise = Dice::new(7);
        for b in 0..beats {
            let at = b * beat;
            for i in 0..(SR * 0.25) as usize {
                let t = i as f32 / SR;
                out[at + i] += 0.8 * (2.0 * std::f32::consts::PI * 55.0 * t).sin() * (-t * 14.0).exp();
            }
            // The hat on the off-beat: highs only (hiss through a crude high-pass).
            let mut prev = 0.0;
            for i in 0..(SR * 0.04) as usize {
                let n = noise.unit() * 2.0 - 1.0;
                let hp = n - prev;
                prev = n;
                out[at + beat / 2 + i] += 0.25 * hp * (-(i as f32) / (SR * 0.01)).exp();
            }
        }
        out
    }

    #[test]
    fn kicks_are_heard_and_the_hats_between_them_are_not_kicks() {
        let sound = kicks_and_hats(16);
        let mut ears = Listener::new(SR);
        let (mut kicks, mut hits) = (0, 0);
        let mut highs_peak = 0.0f32;
        // In UI-frame-sized runs, as the app reads them.
        for run in sound.chunks(800) {
            ears.feed(run);
            let (k, h) = ears.take_hits();
            kicks += k as usize;
            hits += h as usize;
            highs_peak = highs_peak.max(ears.value(Source::Highs));
        }
        // Every kick (the very first one too), and nothing else in the low end.
        assert!((15..=16).contains(&kicks), "{kicks} kicks");
        // The hats are hits, so there are about twice as many hits as kicks.
        assert!(hits >= 24, "{hits} hits");
        assert!(highs_peak > 0.5, "highs {highs_peak}");
    }

    #[test]
    fn a_kick_pulse_falls_away_and_silence_reads_nothing() {
        let mut ears = Listener::new(SR);
        ears.feed(&kicks_and_hats(1)[..(SR * 0.05) as usize]);
        assert!(ears.value(Source::Kick) > 0.6);
        assert!(ears.value(Source::Bass) > 0.6);
        ears.feed(&vec![0.0; SR as usize]);
        assert!(ears.value(Source::Kick) < 0.01);
        assert_eq!(ears.value(Source::Bass), 0.0);
        assert_eq!(ears.value(Source::Level), 0.0);
        // Garbage in does not poison it.
        ears.feed(&[f32::NAN, f32::INFINITY, 0.5]);
        assert!(ears.value(Source::Level).is_finite());
    }

    #[test]
    fn a_steady_tone_reads_loud_in_its_own_band_only() {
        let mut ears = Listener::new(SR);
        let tone: Vec<f32> = (0..SR as usize).map(|i| 0.5 * (2.0 * std::f32::consts::PI * 6000.0 * i as f32 / SR).sin()).collect();
        ears.feed(&tone);
        assert!(ears.value(Source::Highs) > 0.9);
        assert!(ears.value(Source::Level) > 0.9);
        // The bass hears it far under the top: the loudest lately is the tone's
        // leak, so compare levels instead of the 0..1 reading.
        assert!(ears.env[BASS] < ears.env[HIGHS] - 20.0);
    }

    #[test]
    fn notes_pulse_as_hard_as_played_and_keep_the_top_pitch() {
        let mut n = NoteFollower::default();
        n.frame((1u128 << 60) | (1u128 << 64) | (1u128 << 67), 0.8, 0.016);
        assert!(n.hit);
        assert_eq!(n.pitch, 67);
        assert!((n.value(Source::Notes) - 0.8).abs() < 1e-6);
        assert!((n.value(Source::Pitch) - 7.0 / 12.0).abs() < 1e-6);
        for _ in 0..60 {
            n.frame(0, 0.0, 0.016);
        }
        assert!(!n.hit);
        assert!(n.value(Source::Notes) < 0.01);
        // The pitch stays the last one played.
        assert_eq!(n.pitch, 67);
    }

    #[test]
    fn the_beat_clock_pulses_on_beats_and_bars_once_each() {
        let mut c = BeatClock::default();
        let (mut beats, mut bars) = (0, 0);
        let mut pos = 0.0;
        // Eight beats at 60 frames a second, 120 BPM.
        while pos < 8.0 {
            c.frame(pos, true, 4, 1.0 / 60.0);
            beats += c.beat_hit as usize;
            bars += c.bar_hit as usize;
            pos += 2.0 / 60.0;
        }
        assert_eq!(beats, 8);
        assert_eq!(bars, 2);
        assert!(c.value(Source::Ramp) > 0.9);
        // Stopped: nothing comes.
        c.frame(8.5, false, 4, 0.1);
        assert!(!c.beat_hit);
    }

    #[test]
    fn links_take_the_stronger_and_the_master_amount_scales_them() {
        let links = vec![VideoLink::new(Source::Kick, Effect::Zoom, 0.5), VideoLink::new(Source::Bass, Effect::Zoom, 1.0), VideoLink::new(Source::Level, Effect::Gate, 0.8)];
        let mut dice = Dice::default();
        let read = |l: &VideoLink| Reading { value: if l.source == Source::Kick { 1.0 } else { 0.25 }, hit: false };
        let lk = look(&links, &Default::default(), 1.0, &mut dice, read);
        assert!((lk.get(Effect::Zoom) - 0.5).abs() < 1e-6);
        // Gate: dark by the amount, lifted by the level.
        assert!((lk.get(Effect::Gate) - 0.8 * 0.75).abs() < 1e-6);
        let half = look(&links, &Default::default(), 0.5, &mut dice, read);
        assert!((half.get(Effect::Zoom) - 0.25).abs() < 1e-6);
        let none = look(&links, &Default::default(), 0.0, &mut dice, read);
        assert!(none.fx.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn hits_trigger_jumps_as_often_as_the_amount_says() {
        let mut dice = Dice::new(42);
        let always = vec![VideoLink::new(Source::Kick, Effect::Jump, 1.0)];
        let never = vec![VideoLink::new(Source::Kick, Effect::Jump, 0.0)];
        let half = vec![VideoLink::new(Source::Kick, Effect::Stutter, 0.5)];
        let hit = |_: &VideoLink| Reading { value: 1.0, hit: true };
        let calm = |_: &VideoLink| Reading { value: 1.0, hit: false };
        assert!(look(&always, &Default::default(), 1.0, &mut dice, hit).jump);
        assert!(!look(&always, &Default::default(), 1.0, &mut dice, calm).jump);
        assert!(!look(&never, &Default::default(), 1.0, &mut dice, hit).jump);
        let n = (0..1000).filter(|_| look(&half, &Default::default(), 1.0, &mut dice, hit).stutter).count();
        assert!((400..600).contains(&n), "{n}");
    }

    #[test]
    fn notes_pick_slices_like_pads_from_c1() {
        assert_eq!(slice_for_note(36, 16), 0);
        assert_eq!(slice_for_note(37, 16), 1);
        assert_eq!(slice_for_note(51, 16), 15);
        assert_eq!(slice_for_note(52, 16), 0);
        // Below C1 it wraps round from the top.
        assert_eq!(slice_for_note(35, 16), 15);
        assert_eq!(slice_for_note(0, 8), 4);
    }

    #[test]
    fn a_jump_never_lands_on_the_slice_it_left() {
        let mut dice = Dice::new(3);
        let mut now = 5;
        let mut seen = std::collections::HashSet::new();
        for _ in 0..500 {
            let k = other_slice(&mut dice, now, 8);
            assert_ne!(k, now);
            assert!(k < 8);
            seen.insert(k);
            now = k;
        }
        assert_eq!(seen.len(), 8);
        assert_eq!(other_slice(&mut dice, 0, 1), 0);
    }

    #[test]
    fn the_scene_most_tracks_play_picks_the_clip() {
        assert_eq!(scene_clip([2, 2, -1, 0].into_iter(), 4), Some(2));
        assert_eq!(scene_clip([5, 5, 1].into_iter(), 4), Some(1));
        // A tie: the first scene.
        assert_eq!(scene_clip([3, 1].into_iter(), 8), Some(1));
        assert_eq!(scene_clip([-1, -1].into_iter(), 4), None);
        assert_eq!(scene_clip([1].into_iter(), 0), None);
    }

    fn input(dt: f32, beat: f64) -> PlayInput {
        PlayInput { dt, duration: 8.0, beat, playing: true, bpm: 120.0, beats_per_bar: 4, ..Default::default() }
    }

    /// A clip of an 8 s film playing `mode`, changed by `f`.
    fn clip(mode: PlayMode, f: impl FnOnce(&mut ClipPlay)) -> VideoClip {
        let mut c = VideoClip::new("sea.mp4".into());
        c.play.mode = mode;
        f(&mut c.play);
        c
    }

    #[test]
    fn tempo_mode_pins_the_clip_to_the_song() {
        let c = clip(PlayMode::Tempo, |p| p.bars = 2);
        let mut ph = Playhead::default();
        // Two bars of 4/4 = 8 beats for 8 seconds of film: a second a beat.
        assert!((ph.advance(&c, &input(0.016, 3.0)) - 3.0).abs() < 1e-9);
        assert!((ph.advance(&c, &input(0.016, 9.5)) - 1.5).abs() < 1e-9);
        // Backwards: from the end.
        let rev = clip(PlayMode::Tempo, |p| (p.bars, p.reverse) = (2, true));
        let mut ph = Playhead::default();
        assert!((ph.advance(&rev, &input(0.016, 2.0)) - 6.0).abs() < 1e-9);
        // Stopped, it runs on at the song's tempo: 8 s of film in 4 s of song.
        let mut ph = Playhead::default();
        let mut i = input(0.1, 0.0);
        i.playing = false;
        for _ in 0..10 {
            ph.advance(&c, &i);
        }
        assert!((ph.pos - 2.0).abs() < 1e-6);
    }

    #[test]
    fn tempo_fits_the_parts_together_into_its_bars() {
        // Two parts of 1 s and 3 s: 4 s back to back, in one bar (4 beats at 120: 2 s).
        let mut c = clip(PlayMode::Tempo, |p| p.bars = 1);
        c.parts = vec![(1.0, 2.0), (4.0, 7.0)];
        let mut ph = Playhead::default();
        // Half a beat in: a quarter of a second of song, half a second of film, in the first part.
        assert!((ph.advance(&c, &input(0.016, 0.5)) - 1.5).abs() < 1e-9);
        // A beat in: the first part over, the second starting (the gap skipped).
        assert!((ph.advance(&c, &input(0.016, 1.0)) - 4.0).abs() < 1e-9);
        assert!((ph.advance(&c, &input(0.016, 3.0)) - 6.0).abs() < 1e-9);
        // The next bar: from the first part again.
        assert!((ph.advance(&c, &input(0.016, 4.5)) - 1.5).abs() < 1e-9);
    }

    #[test]
    fn loop_mode_wraps_and_reverse_runs_backwards() {
        let mut c = clip(PlayMode::Loop, |p| p.speed = 2.0);
        let mut ph = Playhead::default();
        for _ in 0..100 {
            ph.advance(&c, &input(0.1, 0.0));
        }
        // 10 s at double speed = 20 s of film, round an 8 s clip: 4 s.
        assert!((ph.pos - 4.0).abs() < 1e-6);
        c.play.reverse = true;
        let shown = ph.advance(&c, &input(0.25, 0.0));
        assert!((shown - 3.5).abs() < 1e-6);
        // Freeze holds it still.
        let mut frozen = input(0.25, 0.0);
        frozen.freeze = 1.0;
        assert!((ph.advance(&c, &frozen) - 3.5).abs() < 1e-6);
        // A still picture stays at 0.
        let mut still = input(0.25, 0.0);
        still.duration = 0.0;
        assert_eq!(ph.advance(&c, &still), 0.0);
    }

    #[test]
    fn a_loop_runs_through_its_parts_and_skips_what_is_between() {
        let mut c = clip(PlayMode::Loop, |_| {});
        c.parts = vec![(0.5, 1.0), (6.0, 7.0)];
        let mut ph = Playhead::default();
        let mut seen = Vec::new();
        for _ in 0..30 {
            seen.push(ph.advance(&c, &input(0.1, 0.0)));
        }
        // Never in the gap, and both parts shown.
        assert!(seen.iter().all(|t| (0.5..1.0).contains(t) || (6.0..7.0).contains(t)), "{seen:?}");
        assert!(seen.iter().any(|t| *t < 1.0) && seen.iter().any(|t| *t > 6.0));
        // Paused, it stands; dragged, it goes where it is put (into the parts).
        let mut hold = input(0.1, 0.0);
        hold.hold = true;
        let at = ph.advance(&c, &hold);
        assert_eq!(ph.advance(&c, &hold), at);
        hold.seek = Some(1.25);
        assert!((ph.advance(&c, &hold) - 6.75).abs() < 1e-9);
    }

    #[test]
    fn slices_mode_plays_the_slice_a_note_picks_and_keeps_to_it() {
        let c = clip(PlayMode::Slices, |p| p.slices = 8);
        let mut ph = Playhead::default();
        let mut i = input(0.016, 0.0);
        i.note = Some(38);
        // Slice 2 of 8 in 8 s: from 2 s.
        assert!((ph.advance(&c, &i) - 2.0).abs() < 1e-9);
        i.note = None;
        i.dt = 0.25;
        for _ in 0..6 {
            ph.advance(&c, &i);
        }
        // 1.5 s on, round the one-second slice: 2.5 s.
        assert!((ph.pos - 2.5).abs() < 1e-6);
    }

    #[test]
    fn slices_cut_the_joined_parts_or_one_slice_a_part() {
        // Parts of 2 s and 2 s (4 s back to back) in four slices: a second each.
        let mut c = clip(PlayMode::Slices, |p| p.slices = 4);
        c.parts = vec![(1.0, 3.0), (5.0, 7.0)];
        let mut ph = Playhead::default();
        let mut i = input(0.016, 0.0);
        i.note = Some(38);
        // The third slice: the start of the second part.
        assert!((ph.advance(&c, &i) - 5.0).abs() < 1e-9);
        i.note = Some(37);
        assert!((ph.advance(&c, &i) - 2.0).abs() < 1e-9);
        // Parts as slices: C1 the first part, C#1 the second, round again after.
        let mut by_part = c.clone();
        by_part.play.part_slices = true;
        by_part.parts = vec![(1.0, 1.5), (4.0, 7.0)];
        let spans = by_part.spans(8.0);
        assert_eq!(by_part.slice_count(&spans), 2);
        assert_eq!(by_part.slice_bounds(&spans, 1), (0.5, 3.5));
        let mut ph = Playhead::default();
        i.note = Some(37);
        assert!((ph.advance(&by_part, &i) - 4.0).abs() < 1e-9);
        // It keeps to its part: round and round in the 3 s of it.
        i.note = None;
        i.dt = 0.25;
        for _ in 0..14 {
            ph.advance(&by_part, &i);
        }
        assert!((ph.pos - (0.5 + 3.5 % 3.0)).abs() < 1e-6, "{}", ph.pos);
        i.note = Some(38);
        assert!((ph.advance(&by_part, &i) - 1.0).abs() < 1e-9, "E1 is the first part again");
        // One part only: the slices count again.
        by_part.parts = vec![(1.0, 5.0)];
        assert_eq!(by_part.slice_count(&by_part.spans(8.0)), 4);
    }

    /// A film of 8 s cut at uneven places (as its sound's hits fell),
    /// played by track 7, in Slices.
    fn cued(gate: bool) -> VideoSet {
        let mut set = VideoSet::default();
        let cues = Cues { times: vec![0.0, 0.7, 1.9, 4.25], end: 6.0, track: Some(7), gate };
        assert_eq!(set.cue_film(std::path::Path::new("zoo.mp4"), cues), Some(0));
        set
    }

    #[test]
    fn a_note_shows_its_cue_and_one_shot_plays_the_piece_to_its_end() {
        let set = cued(false);
        let c = &set.clips[0];
        assert_eq!(c.play.mode, PlayMode::Slices);
        let mut ph = Playhead::default();
        // Before any note: on the first cue, standing.
        assert_eq!(ph.advance(c, &input(0.1, 0.0)), 0.0);
        assert_eq!(ph.advance(c, &input(0.1, 0.0)), 0.0);
        // D1, the third pad: the third cue, exactly (not a third of the film).
        let mut i = input(0.016, 0.0);
        i.note = Some(38);
        assert_eq!(ph.advance(c, &i), 1.9);
        // On as filmed, whatever the Speed: the sound plays as recorded.
        let mut fast = c.clone();
        fast.play.speed = 3.0;
        let at = ph.advance(&fast, &input(0.25, 0.0));
        assert!((at - 2.15).abs() < 1e-9, "{at}");
        // The note ended long ago (One-Shot does not care): to the piece's end, and it stays there.
        for _ in 0..20 {
            ph.advance(c, &input(0.25, 0.0));
        }
        assert!((ph.pos - (4.25 - 1e-3)).abs() < 1e-9, "{}", ph.pos);
        // The last piece ends where the cut sound ends; past the pads it goes round (E1 is the first again).
        i.note = Some(39);
        assert_eq!(ph.advance(c, &i), 4.25);
        for _ in 0..20 {
            ph.advance(c, &input(0.25, 0.0));
        }
        assert!((ph.pos - (6.0 - 1e-3)).abs() < 1e-9);
        i.note = Some(40);
        assert_eq!(ph.advance(c, &i), 0.0);
        // A jump goes to the start of another cue.
        let mut j = input(0.016, 0.0);
        j.jump = true;
        let at = ph.advance(c, &j);
        assert!(c.cues.as_ref().unwrap().times[1..].contains(&at), "{at}");
        // Parts do not change a cued film: it plays whole, by its cues.
        let mut parted = c.clone();
        parted.parts = vec![(5.0, 6.0)];
        i.note = Some(37);
        assert_eq!(Playhead::default().advance(&parted, &i), 0.7);
    }

    #[test]
    fn gate_holds_the_picture_where_its_note_ends() {
        let set = cued(true);
        let c = &set.clips[0];
        let mut ph = Playhead::default();
        let mut i = input(0.016, 0.0);
        i.note = Some(37);
        i.held = 1 << 37;
        assert_eq!(ph.advance(c, &i), 0.7);
        // Held: on it runs.
        let mut held = input(0.1, 0.0);
        held.held = 1 << 37;
        ph.advance(c, &held);
        ph.advance(c, &held);
        assert!((ph.pos - 0.9).abs() < 1e-6);
        // Let go (another note held does not count): it stands where it was.
        let mut gone = input(0.1, 0.0);
        gone.held = 1 << 40;
        ph.advance(c, &gone);
        ph.advance(c, &gone);
        assert!((ph.pos - 0.9).abs() < 1e-6);
        // Held past its piece, it stops at the end like the slice.
        ph.advance(c, &i);
        for _ in 0..30 {
            ph.advance(c, &held);
        }
        assert!((ph.pos - (1.9 - 1e-3)).abs() < 1e-9);
        // A clip without cues still cuts in equal slices.
        let mut even = c.clone();
        even.cues = None;
        even.play.slices = 8;
        let mut ph = Playhead::default();
        i.note = Some(38);
        assert!((ph.advance(&even, &i) - 2.0).abs() < 1e-9);
        // And another clip of the bank does not use the cues: each plays its own way.
        let other = clip(PlayMode::Slices, |p| p.slices = 8);
        let mut ph = Playhead::default();
        assert!((ph.advance(&other, &i) - 2.0).abs() < 1e-9);
        // Not in Slices, a cued clip ignores its cues and loops its film.
        let mut looping = c.clone();
        looping.play.mode = PlayMode::Loop;
        let mut ph = Playhead::default();
        assert!((ph.advance(&looping, &input(0.25, 0.0)) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn cues_find_their_pieces_and_a_film_is_cued_once_in_the_bank() {
        let c = Cues { times: vec![0.0, 0.5, 2.0], end: 3.0, track: None, gate: false };
        assert_eq!((c.at(0.0), c.at(0.49), c.at(0.5), c.at(9.0)), (0, 0, 1, 2));
        assert_eq!(c.piece(2, 0.0), (2.0, 3.0));
        assert_eq!(c.piece(2, 2.5), (2.0, 2.5), "no further than the film");
        assert_eq!((c.for_note(36), c.for_note(38), c.for_note(39)), (0, 2, 0));
        let mut set = cued(false);
        set.clips[0].play.mode = PlayMode::Loop;
        set.clips.insert(0, VideoClip::new("sea.mp4".into()));
        // Cued again (sliced again): the same clip, new cues, playing in Slices.
        assert_eq!(set.cue_film(std::path::Path::new("zoo.mp4"), c.clone()), Some(1));
        assert_eq!((set.clips.len(), set.current, set.clips[1].play.mode), (2, 1, PlayMode::Slices));
        assert_eq!(set.clips[1].cues.as_ref(), Some(&c));
        // The other clip plays as it did.
        assert_eq!(set.clips[0].play.mode, PlayMode::Loop);
        // A full bank has no room for another.
        while set.clips.len() < MAX_CLIPS {
            let n = set.clips.len();
            set.clips.push(VideoClip::new(format!("{n}.gif").into()));
        }
        assert_eq!(set.cue_film(std::path::Path::new("new.mp4"), c), None);
        assert!(is_film(std::path::Path::new("a/b.MP4")) && is_film(std::path::Path::new("x.webm")));
        assert!(!is_film(std::path::Path::new("a.wav")) && !is_film(std::path::Path::new("a.gif")));
    }

    #[test]
    fn a_jump_cuts_to_the_start_of_another_slice_and_tempo_keeps_it() {
        let c = clip(PlayMode::Tempo, |p| (p.bars, p.slices) = (2, 8));
        let mut ph = Playhead::default();
        ph.advance(&c, &input(0.016, 0.5));
        let mut j = input(0.016, 0.5);
        j.jump = true;
        let at = ph.advance(&c, &j);
        assert!((at - at.round()).abs() < 1e-9, "on a slice start: {at}");
        assert!((at - 0.5).abs() > 0.4);
        // Then it runs on from there with the song.
        let later = ph.advance(&c, &input(0.016, 1.5));
        assert!(((later - at).rem_euclid(8.0) - 1.0).abs() < 1e-9);
        // Dragged somewhere while the song plays: from there on, with the song.
        let mut s = input(0.016, 2.0);
        s.seek = Some(5.0);
        assert!((ph.advance(&c, &s) - 5.0).abs() < 1e-9);
        assert!((ph.advance(&c, &input(0.016, 3.0)) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn a_stutter_repeats_a_sixteenth_for_a_beat_and_scratch_pulls_back() {
        let c = clip(PlayMode::Loop, |_| {});
        let mut ph = Playhead::default();
        ph.pos = 3.0;
        let mut s = input(0.0, 0.0);
        s.stutter = true;
        assert!((ph.advance(&c, &s) - 3.0).abs() < 1e-9);
        // A sixteenth at 120 BPM is 0.125 s: 0.15 s later it is 0.025 s in again.
        let shown = ph.advance(&c, &input(0.15, 0.0));
        assert!((shown - 3.025).abs() < 1e-6, "{shown}");
        // After the beat it is back where the film ran to.
        ph.advance(&c, &input(0.2, 0.0));
        let shown = ph.advance(&c, &input(0.2, 0.0));
        assert!((shown - ph.pos).abs() < 1e-9);
        // Scratch: a beat (0.5 s) back at full.
        let mut sc = input(0.0, 0.0);
        sc.scratch = 1.0;
        assert!((ph.advance(&c, &sc) - (ph.pos - 0.5)).abs() < 1e-9);
    }

    #[test]
    fn the_strobe_is_on_half_of_every_sixteenth() {
        assert_eq!(strobe_light(0.8, 0.1, 4.0, 0.5), 0.8);
        assert_eq!(strobe_light(0.8, 0.2, 4.0, 0.5), 0.0);
        assert_eq!(strobe_light(0.0, 0.1, 4.0, 0.5), 0.0);
        // Eighths, lit a quarter of each.
        assert_eq!(strobe_light(1.0, 0.1, 2.0, 0.25), 1.0);
        assert_eq!(strobe_light(1.0, 0.2, 2.0, 0.25), 0.0);
    }

    #[test]
    fn presets_exist_and_fit_in_the_links() {
        for name in PRESET_NAMES {
            let p = preset(name).expect(name);
            assert!(p.links.len() <= MAX_LINKS);
            assert!(!p.about.is_empty());
        }
        assert!(preset("Nope").is_none());
        // Cutup plays the clip that plays in Slices (the others as they were).
        let mut set = VideoSet { clips: vec![VideoClip::new("a.mp4".into()), VideoClip::new("b.mp4".into())], current: 1, ..Default::default() };
        set.apply_preset(&preset("Cutup").unwrap());
        assert_eq!((set.clips[0].play.mode, set.clips[1].play.mode), (PlayMode::Loop, PlayMode::Slices));
        // Every effect and source has a name and a place.
        assert_eq!(Effect::ALL.len(), Effect::COUNT);
        for (i, e) in Effect::ALL.iter().enumerate() {
            assert!(Effect::ALL.iter().skip(i + 1).all(|o| o != e));
            assert!(e.index() < Effect::COUNT);
        }
        assert_eq!(Source::Fixed.next(), Source::Kick);
        assert_eq!(PlayMode::Slices.next(), PlayMode::Loop);
    }

    #[test]
    fn the_video_round_trips_and_old_or_odd_files_load() {
        let mut set = VideoSet { clips: vec![VideoClip::new("C:/films/sea.mp4".into()), VideoClip::new("/home/me/cat.gif".into())], current: 1, transition: Transition::Glitch, launch: Launch::Bar, clip_notes: Some(3), ..Default::default() };
        set.clips[0].play = ClipPlay { mode: PlayMode::Tempo, bars: 8, ..Default::default() };
        set.clips[0].parts = vec![(1.0, 2.0), (4.0, 5.5)];
        set.looks[0].links[0].from = Some(12);
        let json = serde_json::to_string(&set).unwrap();
        let back: VideoSet = serde_json::from_str(&json).unwrap();
        assert_eq!(back, set);
        assert_eq!(back.current_clip().map(|c| c.name.as_str()), Some("cat"));
        // Fields it does not know yet come out as they start.
        let bare: VideoSet = serde_json::from_str(r#"{"clips":[{"path":"a.png","name":"a"}]}"#).unwrap();
        assert_eq!(bare.clips[0].play, ClipPlay::default());
        assert_eq!(bare.clips[0].cues, None);
        assert!(!json.contains("cues"), "no cues, nothing written");
        // Cues go along, and odd ones are put right.
        let cued = cued(true);
        let back: VideoSet = serde_json::from_str(&serde_json::to_string(&cued).unwrap()).unwrap();
        assert_eq!(back, cued);
        let mut odd_cues: VideoSet = serde_json::from_str(r#"{"clips":[{"path":"a.mp4","name":"a","cues":{"times":[2.0,-1.0,1.0]}},{"path":"b.mp4","name":"b","cues":{}}]}"#).unwrap();
        odd_cues.sanitize();
        let c = odd_cues.clips[0].cues.as_ref().unwrap();
        assert_eq!((c.times.as_slice(), c.end, c.track, c.gate), (&[0.0, 1.0, 2.0][..], 2.0, None, false));
        assert_eq!(odd_cues.clips[1].cues, None, "no cues at all is no cues");
        assert_eq!(bare.looks, VideoSet::default().looks);
        // Out of range is put right (an old set's way of playing too, on its clip).
        let mut odd: VideoSet = serde_json::from_str(r#"{"clips":[{"path":"a.png","name":"a","parts":[[3.0,1.0],[2.5,4.0]]}],"current":9,"speed":99.0,"amount":-1.0,"slices":0,"transition_beats":-3.0}"#).unwrap();
        odd.sanitize();
        let p = &odd.clips[0].play;
        assert_eq!((odd.current, p.speed, odd.amount, p.slices, odd.transition_beats), (0, 4.0, 0.0, 2, 0.0625));
        assert_eq!(odd.clips[0].parts, vec![(1.0, 4.0)]);
        assert!(VideoSet::default().is_empty());
        assert!(!set.is_empty());
    }

    /// A clip of an 8 s film with parts of 2 s (from 0) and 4 s (from 4), playing them one a bar.
    fn sequenced(mode: PlayMode, f: impl FnOnce(&mut ClipPlay)) -> VideoClip {
        let mut c = clip(mode, |p| {
            p.seq.on = true;
            p.seq.every = Every::Bar;
            f(p);
        });
        c.parts = vec![(0.0, 2.0), (4.0, 8.0)];
        c
    }

    #[test]
    fn a_part_sequence_changes_part_exactly_on_the_bar_and_slices_cut_only_that_part() {
        let c = sequenced(PlayMode::Slices, |p| p.slices = 4);
        let mut ph = Playhead::default();
        // The first bar: the first part. D1 (the third of four slices of its 2 s): from 1 s.
        let mut i = input(0.0, 3.999);
        i.note = Some(38);
        assert!((ph.advance(&c, &i) - 1.0).abs() < 1e-9);
        assert_eq!(ph.part, Some(0));
        // Exactly on the second bar: the second part, the same slice of it (4 s in four: from 4 + 2).
        assert!((ph.advance(&c, &input(0.0, 4.0)) - 6.0).abs() < 1e-9);
        assert_eq!(ph.part, Some(1));
        // A note now cuts this part only: C1 its start, E1 (round again) too.
        let mut i = input(0.0, 5.0);
        i.note = Some(36);
        assert!((ph.advance(&c, &i) - 4.0).abs() < 1e-9);
        i.note = Some(39);
        assert!((ph.advance(&c, &i) - 7.0).abs() < 1e-9);
        // It plays round in its slice, never into the other part.
        for _ in 0..10 {
            let at = ph.advance(&c, &input(0.25, 5.5));
            assert!((7.0..8.0).contains(&at), "{at}");
        }
        // The third bar: the first part again.
        ph.advance(&c, &input(0.0, 8.0));
        assert_eq!(ph.part, Some(0));
    }

    #[test]
    fn tempo_fits_each_part_of_a_sequence_into_its_bars_and_loop_stays_in_it() {
        let c = sequenced(PlayMode::Tempo, |p| p.bars = 1);
        let mut ph = Playhead::default();
        // Half a bar: half of each part, whatever its length.
        assert!((ph.advance(&c, &input(0.016, 2.0)) - 1.0).abs() < 1e-9);
        assert!((ph.advance(&c, &input(0.016, 6.0)) - 6.0).abs() < 1e-9);
        assert!((ph.advance(&c, &input(0.016, 10.0)) - 1.0).abs() < 1e-9);
        // Loop: round in the part that plays, from its start when it comes.
        let c = sequenced(PlayMode::Loop, |_| {});
        let mut ph = Playhead::default();
        for k in 0..20 {
            let at = ph.advance(&c, &input(0.25, 0.1 + k as f64 * 0.1));
            assert!((0.0..2.0).contains(&at), "{at}");
        }
        assert!((ph.advance(&c, &input(0.0, 4.0)) - 4.0).abs() < 1e-9, "the next part from its start");
        // Stopped: it stays on its part.
        let mut stopped = input(0.25, 0.0);
        stopped.playing = false;
        ph.advance(&c, &stopped);
        assert_eq!(ph.part, Some(1));
        // Without the sequence the parts play back to back as before.
        let mut plain = c.clone();
        plain.play.seq.on = false;
        let mut ph = Playhead::default();
        ph.advance(&plain, &input(0.0, 0.0));
        assert_eq!(ph.part, None);
    }

    #[test]
    fn parts_come_in_order_ping_pong_by_steps_by_chance_by_notes_or_by_hits() {
        let mut seq = PartSeq::default();
        assert_eq!((0..7).map(|s| seq.part_at(s, 3).unwrap()).collect::<Vec<_>>(), [0, 1, 2, 0, 1, 2, 0]);
        seq.order = PartOrder::PingPong;
        assert_eq!((0..7).map(|s| seq.part_at(s, 3).unwrap()).collect::<Vec<_>>(), [0, 1, 2, 1, 0, 1, 2]);
        seq.order = PartOrder::Steps;
        seq.steps = vec![Some(2), None, Some(4)];
        assert_eq!((0..4).map(|s| seq.part_at(s, 3)).collect::<Vec<_>>(), [Some(2), None, Some(1), Some(2)]);
        // A hold keeps the part before.
        let c = sequenced(PlayMode::Loop, |p| (p.seq.order, p.seq.steps) = (PartOrder::Steps, vec![Some(1), None]));
        let mut ph = Playhead::default();
        ph.advance(&c, &input(0.0, 0.0));
        ph.advance(&c, &input(0.0, 4.0));
        assert_eq!(ph.part, Some(1));
        // By chance: never the same twice running.
        let mut c = sequenced(PlayMode::Loop, |p| (p.seq.order, p.seq.every) = (PartOrder::Random, Every::Beat));
        c.parts = vec![(0.0, 1.0), (2.0, 3.0), (4.0, 5.0), (6.0, 7.0)];
        let mut ph = Playhead::default();
        let mut seen = Vec::new();
        for b in 0..40 {
            ph.advance(&c, &input(0.0, b as f64));
            seen.push(ph.part.unwrap());
        }
        assert!(seen.windows(2).all(|w| w[0] != w[1]), "{seen:?}");
        assert!((0..4).all(|k| seen.contains(&k)));
        // Notes from C3: C#3 the second part; lower notes leave it.
        let c = sequenced(PlayMode::Loop, |p| p.seq.order = PartOrder::Notes);
        let mut ph = Playhead::default();
        let mut i = input(0.0, 0.0);
        i.part_note = Some(61);
        assert!((ph.advance(&c, &i) - 4.0).abs() < 1e-9);
        i.part_note = Some(40);
        ph.advance(&c, &i);
        assert_eq!(ph.part, Some(1));
        assert_eq!((part_for_note(62, 2), part_for_note(59, 2)), (Some(0), None));
        // Hits: the next one each time.
        let c = sequenced(PlayMode::Loop, |p| p.seq.order = PartOrder::Hits);
        let mut ph = Playhead::default();
        let mut i = input(0.0, 0.0);
        i.part_step = Some(Step::Next);
        ph.advance(&c, &i);
        assert_eq!(ph.part, Some(1));
        ph.advance(&c, &i);
        assert_eq!(ph.part, Some(0));
    }

    #[test]
    fn bounce_runs_there_and_back_offset_starts_later_and_freeze_stands() {
        // On to the end (8 s) and back: 10.25 s in, it is 2.25 s back from the end.
        let c = clip(PlayMode::Loop, |p| p.bounce = true);
        let mut ph = Playhead::default();
        for _ in 0..40 {
            ph.advance(&c, &input(0.25, 0.0));
        }
        assert!((ph.advance(&c, &input(0.25, 0.0)) - (16.0 - 10.25)).abs() < 1e-6);
        // Offset: from 3 s on.
        let c = clip(PlayMode::Loop, |p| p.offset = 3.0);
        let mut ph = Playhead::default();
        assert!((ph.advance(&c, &input(0.0, 0.0)) - 3.0).abs() < 1e-9);
        // Freeze: one frame, whatever the time.
        let c = clip(PlayMode::Loop, |p| (p.freeze, p.offset) = (true, 2.0));
        let mut ph = Playhead::default();
        for _ in 0..10 {
            assert!((ph.advance(&c, &input(0.25, 0.0)) - 2.0).abs() < 1e-9);
        }
        // Tempo there and back: the second lap backwards.
        let c = clip(PlayMode::Tempo, |p| (p.bars, p.bounce) = (2, true));
        let mut ph = Playhead::default();
        assert!((ph.advance(&c, &input(0.016, 10.0)) - 6.0).abs() < 1e-9);
    }

    /// Hits in eight beats of `kicks_and_hats` heard by `t`, a frame every 800 samples (60 a second at 48 kHz): when (s) and in which band.
    fn hear_hits(t: &HitTrigger, beats: usize) -> Vec<(f64, usize)> {
        let sound = kicks_and_hats(beats);
        let mut ears = Listener::new(SR);
        let mut det = HitDetector::default();
        let mut out = Vec::new();
        for (k, run) in sound.chunks(800).enumerate() {
            ears.feed(run);
            let now = ((k + 1) * 800) as f64 / SR as f64;
            if let Some(b) = det.frame(t, ears.take_rises(), 800.0 / SR, now * 2.0, true, 120.0) {
                out.push((now, b));
            }
        }
        out
    }

    #[test]
    fn a_drum_track_without_notes_drives_the_slices_at_its_kicks() {
        let t = HitTrigger::default();
        let hits = hear_hits(&t, 8);
        // Every kick, within a frame of it (the hats between are not kicks).
        assert_eq!(hits.len(), 8, "{hits:?}");
        for (n, (at, band)) in hits.iter().enumerate() {
            let late = at - n as f64 * 0.5;
            assert!((0.0..=1.0 / 30.0).contains(&late), "kick {n} heard {late} s late");
            assert_eq!(*band, 0);
        }
        // Each one the next slice: an 8 s film in eight slices shows 1 s, 2 s, 3 s...
        let c = clip(PlayMode::Slices, |p| p.slices = 8);
        let mut ph = Playhead::default();
        let mut shown = Vec::new();
        for (_, b) in &hits {
            let mut i = input(0.0, 0.0);
            i.slice_step = Some(Step::of_hit(t.to, *b));
            shown.push(ph.advance(&c, &i));
        }
        assert_eq!(shown, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 0.0]);
        // A gap of two beats: every other kick.
        assert_eq!(hear_hits(&HitTrigger { gap: 2.0, ..t }, 8).len(), 4);
        // The hats on the off-beats, put on the next beat.
        let hats = hear_hits(&HitTrigger { band: HitBand::Hats, gap: 0.25, quantize: 1.0, ..t }, 8);
        assert!(hats.len() >= 6, "{hats:?}");
        for (at, band) in &hats {
            let beat = at * 2.0;
            assert!(beat - beat.floor() < 0.07, "on a beat: {beat}");
            assert_eq!(*band, 2);
        }
        // Any hit, by its band: kicks pick the first, hats the third.
        let any = hear_hits(&HitTrigger { band: HitBand::Any, to: StepTo::ByBand, gap: 0.125, ..t }, 8);
        let kicks = any.iter().filter(|h| h.1 == 0).count();
        let hat = any.iter().filter(|h| h.1 == 2).count();
        assert!(kicks >= 7 && hat >= 7, "{any:?}");
        assert_eq!(Step::of_hit(StepTo::ByBand, 2).from(0, 8, &mut Dice::default()), 2);
        // A cued film is moved along its cues the same way.
        let set = cued(false);
        let mut ph = Playhead::default();
        let mut i = input(0.0, 0.0);
        i.slice_step = Some(Step::Next);
        assert_eq!(ph.advance(&set.clips[0], &i), 0.7);
        assert_eq!(ph.advance(&set.clips[0], &i), 1.9);
    }

    #[test]
    fn a_look_has_effects_of_its_own_under_the_links_and_old_links_look_the_same() {
        let links = vec![VideoLink::new(Source::Kick, Effect::Zoom, 0.5), VideoLink::new(Source::Level, Effect::Gate, 0.8)];
        let read = |l: &VideoLink| Reading { value: if l.source == Source::Kick { 1.0 } else { 0.25 }, hit: false };
        let plain = look(&links, &Default::default(), 1.0, &mut Dice::default(), read);
        // Settings that are as they start change nothing.
        let mut fx = std::collections::BTreeMap::new();
        fx.insert(Effect::Gate, FxSetting { params: vec![0.0, 1.0], ..Default::default() });
        fx.insert(Effect::Zoom, FxSetting::default());
        assert_eq!(look(&links, &fx, 1.0, &mut Dice::default(), read), plain);
        // A base: always there, the links on top, not scaled by the music.
        fx.insert(Effect::Hue, FxSetting { base: 0.3, ..Default::default() });
        fx.get_mut(&Effect::Zoom).unwrap().base = 0.25;
        let lk = look(&links, &fx, 0.0, &mut Dice::default(), read);
        assert!((lk.get(Effect::Hue) - 0.3).abs() < 1e-6 && (lk.get(Effect::Zoom) - 0.25).abs() < 1e-6);
        let lk = look(&links, &fx, 1.0, &mut Dice::default(), read);
        assert!((lk.get(Effect::Zoom) - 0.75).abs() < 1e-6);
        // A range: from low at 0 to the amount at full.
        let ranged = vec![VideoLink { low: 0.2, ..VideoLink::new(Source::Level, Effect::Pixels, 0.6) }];
        let lk = look(&ranged, &Default::default(), 1.0, &mut Dice::default(), |_| Reading { value: 0.5, hit: false });
        assert!((lk.get(Effect::Pixels) - 0.4).abs() < 1e-6);
        // Gate's threshold: shut until the level is past it.
        fx.insert(Effect::Gate, FxSetting { params: vec![0.5, 1.0], ..Default::default() });
        assert!((look(&links, &fx, 1.0, &mut Dice::default(), read).get(Effect::Gate) - 0.8).abs() < 1e-6);
        // Settings stay in their range; a choice is a whole place.
        let mut s = FxSetting::default();
        s.set_param(Effect::Mirror, 0, 2.6);
        assert_eq!(s.param(Effect::Mirror, 0), 3.0);
        s.set_param(Effect::Pixels, 0, 1000.0);
        assert_eq!(s.param(Effect::Pixels, 0), 120.0);
        assert_eq!(FxSetting::default().params4(Effect::Zoom), [1.5, 0.0, 0.0, 1.0]);
        for e in Effect::ALL {
            assert!(e.params().len() <= 4, "{e:?} has room for four settings");
            assert!(e.params().iter().all(|p| (p.min..=p.max).contains(&p.default)), "{e:?}");
        }
    }

    #[test]
    fn a_links_response_smooths_bends_inverts_and_waits_for_its_threshold() {
        let plain = Response::default();
        assert_eq!((plain.shape(0.3), plain.smooth(0.0, 1.0, 0.016)), (0.3, 1.0));
        let r = Response { curve: 2.0, threshold: 0.5, invert: true, ..plain };
        // Inverted 0.1 is 0.9; over the half: 0.8, squared.
        assert!((r.shape(0.1) - 0.64).abs() < 1e-6);
        assert_eq!(r.shape(0.8), 0.0);
        let slow = Response { attack: 0.1, release: 1.0, ..plain };
        let up = slow.smooth(0.0, 1.0, 0.1);
        assert!((up - (1.0 - (-1.0f32).exp())).abs() < 1e-6);
        assert!(slow.smooth(1.0, 0.0, 0.1) > 0.9, "it lets go slowly");
        // An LFO in time with the song.
        let lfo = Lfo { shape: LfoShape::Triangle, beats: 4.0, phase: 0.0 };
        assert_eq!(lfo.at(2.0), (1.0, 0));
        assert_eq!(lfo.at(5.0).1, 1);
        let saw = Lfo { shape: LfoShape::Saw, beats: 1.0, phase: 0.25 };
        assert!((saw.at(0.5).0 - 0.75).abs() < 1e-6);
        let random = Lfo { shape: LfoShape::Random, ..lfo };
        assert_eq!(random.at(1.0), random.at(3.9), "held through its cycle");
        // A band of its own hears a tone in it, not one outside it.
        let tone = |hz: f32| (0..SR as usize).map(|i| 0.5 * (2.0 * std::f32::consts::PI * hz * i as f32 / SR).sin()).collect::<Vec<f32>>();
        let mut inside = BandListener::new(SR, (800.0, 1200.0));
        inside.feed(&tone(1000.0));
        assert!(inside.value() > 0.9);
        let mut both = BandListener::new(SR, (800.0, 1200.0));
        both.feed(&tone(1000.0));
        both.feed(&tone(80.0));
        assert!(both.value() < 0.2, "{}", both.value());
    }

    #[test]
    fn looks_change_by_part_clip_sequence_and_pad_and_old_sets_get_one_look() {
        // The part's Look wins, then the clip's, then the sequence's, then the set's.
        assert_eq!(look_for(Some(2), Some(1), Some(3), 0, 4), 2);
        assert_eq!(look_for(None, Some(1), Some(3), 0, 4), 1);
        assert_eq!(look_for(None, None, Some(3), 0, 4), 3);
        assert_eq!(look_for(None, None, None, 0, 4), 0);
        assert_eq!(look_for(Some(9), None, None, 1, 4), 1, "a Look that is gone");
        // The sequence: a step a bar, exactly on the bar; a rest is the set's own.
        let seq = LookSeq { on: true, every: Every::Bar, steps: vec![Some(1), None, Some(2)] };
        assert_eq!(seq.look_at(3.999, 4, 3), Some(1));
        assert_eq!(seq.look_at(4.0, 4, 3), None);
        assert_eq!(seq.look_at(8.0, 4, 3), Some(2));
        assert_eq!(seq.look_at(12.0, 4, 3), Some(1));
        assert_eq!(seq.look_at(8.0, 4, 2), None, "no third Look");
        assert_eq!(LookSeq { on: false, ..seq }.look_at(0.0, 4, 3), None);
        // An older set: its links are its one Look (named as their preset), nothing in sequence.
        let pulse = serde_json::to_string(&preset("Pulse").unwrap().links).unwrap();
        let old: VideoSet = serde_json::from_str(&format!(r#"{{"clips":[],"links":{pulse}}}"#)).unwrap();
        assert_eq!(old.looks.len(), 1);
        assert_eq!(old.looks[0].name, "Pulse");
        assert!(old.looks[0].fx.is_empty() && !old.look_seq.on);
        let mine: VideoSet = serde_json::from_str(r#"{"clips":[],"links":[{"source":"Bass","effect":"Hue","amount":0.7}]}"#).unwrap();
        assert_eq!((mine.looks[0].name.as_str(), mine.links()[0].amount), ("Look 1", 0.7));
        assert_eq!(mine.links()[0].response, Response::default());
        // Written again: Looks, no links of the old kind; it reads back the same.
        let mut set = mine.clone();
        set.looks.push(VideoLook::new("Two", vec![]));
        set.looks[1].setting_mut(Effect::Grade).base = 1.0;
        set.clips.push(VideoClip::new("a.mp4".into()));
        set.clips[0].parts = vec![(0.0, 1.0), (2.0, 3.0)];
        set.clips[0].set_part_look(1, Some(1));
        set.look_seq.on = true;
        let json = serde_json::to_string(&set).unwrap();
        assert!(!json.contains("\"links\":[{\"source\":\"Bass\",\"effect\":\"Hue\",\"amount\":0.7}],\"looks\""));
        let back: VideoSet = serde_json::from_str(&json).unwrap();
        assert_eq!(back, set);
        // A part's Look stays with it as parts are split, kept and taken away.
        let c = &mut set.clips[0];
        assert!(c.split_part(2.5));
        assert_eq!(c.part_looks, vec![None, Some(1), Some(1)]);
        c.add_part(5.0, 6.0);
        assert_eq!(c.part_look(1), Some(1));
        assert_eq!(c.part_look(3), None);
        c.remove_part(0);
        assert_eq!(c.part_look(0), Some(1));
    }
}
