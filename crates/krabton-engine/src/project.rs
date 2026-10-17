//! The project data model. The UI owns a `Project` and edits it freely; the
//! audio engine receives immutable snapshots (`Arc<Project>`).

use crate::dsp::drums::{self, DrumVoice};
use crate::dsp::effects::EffectKind;
use crate::dsp::dx7::{self, Dx7Voice};
use crate::dsp::{analog, braids_synth, elements_synth, fm, macro_osc, sampler, shaker_synth, synth, stk_voice, stk_mallets, stk_strings, stk_winds, organ, rings_synth};
use crate::params;
use crate::routing::{AudioInput, AudioOutput, MidiInput, Monitor};
use crate::sample::{LoadCacheGuard, SampleRef};
use crate::scale::{convert_pitch, Scale};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub type Rgb = [u8; 3];

/// Clip/track colors, roughly following Live's palette.
pub const PALETTE: [Rgb; 16] = [
    [255, 148, 166],
    [255, 165, 41],
    [204, 153, 39],
    [247, 244, 124],
    [191, 251, 0],
    [26, 255, 47],
    [37, 255, 168],
    [92, 255, 232],
    [139, 197, 255],
    [84, 128, 228],
    [146, 167, 255],
    [216, 108, 228],
    [229, 83, 160],
    [255, 255, 255],
    [255, 87, 6],
    [133, 218, 194],
];

/// Lowest volume fader value; treated as silence.
pub const FADER_MIN_DB: f32 = -70.0;
pub const FADER_MAX_DB: f32 = 6.0;

pub fn fader_gain(db: f32) -> f32 {
    if db <= FADER_MIN_DB + 0.05 {
        0.0
    } else {
        crate::dsp::db_to_gain(db)
    }
}

/// Note name using Live's convention (middle C = C3 = 60).
pub fn note_name(pitch: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    format!("{}{}", NAMES[pitch as usize % 12], pitch as i32 / 12 - 2)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quantize {
    None,
    Sixteenth,
    Eighth,
    Quarter,
    Half,
    Bar,
    TwoBars,
    FourBars,
}

impl Quantize {
    pub const ALL: [Quantize; 8] = [
        Quantize::None,
        Quantize::Sixteenth,
        Quantize::Eighth,
        Quantize::Quarter,
        Quantize::Half,
        Quantize::Bar,
        Quantize::TwoBars,
        Quantize::FourBars,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Quantize::None => "None",
            Quantize::Sixteenth => "1/16",
            Quantize::Eighth => "1/8",
            Quantize::Quarter => "1/4",
            Quantize::Half => "1/2",
            Quantize::Bar => "1 Bar",
            Quantize::TwoBars => "2 Bars",
            Quantize::FourBars => "4 Bars",
        }
    }

    pub fn beats(self, beats_per_bar: u32) -> Option<f64> {
        let bar = beats_per_bar.max(1) as f64;
        match self {
            Quantize::None => None,
            Quantize::Sixteenth => Some(0.25),
            Quantize::Eighth => Some(0.5),
            Quantize::Quarter => Some(1.0),
            Quantize::Half => Some(2.0),
            Quantize::Bar => Some(bar),
            Quantize::TwoBars => Some(bar * 2.0),
            Quantize::FourBars => Some(bar * 4.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub pitch: u8,
    /// Start in beats from the clip start.
    pub start: f64,
    pub length: f64,
    pub velocity: f32,
    /// Note operators (as in Bitwig): the chance it plays each time (0..1).
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub chance: f32,
    /// How far the velocity may wander each time it plays (0..1).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub vel_spread: f32,
    /// Plays as this many quick repeats over its length (ratchets).
    #[serde(default = "one_u8", skip_serializing_if = "is_one_u8")]
    pub repeats: u8,
    /// Recurrence: over a cycle of this many loops (2..8; 0 or 1: off) the
    /// note plays on the loops whose bit is set in `recur_mask`.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub recur_len: u8,
    #[serde(default = "all_bits")]
    pub recur_mask: u8,
}

fn one() -> f32 {
    1.0
}
fn is_one(v: &f32) -> bool {
    *v == 1.0
}
fn is_zero(v: &f32) -> bool {
    *v == 0.0
}
fn one_u8() -> u8 {
    1
}
fn is_one_u8(v: &u8) -> bool {
    *v <= 1
}
fn is_zero_u8(v: &u8) -> bool {
    *v == 0
}
fn all_bits() -> u8 {
    0xFF
}

/// The shortest note `tidy_notes` leaves when it shortens one.
pub const MIN_TIDY_LENGTH: f64 = 1.0 / 32.0;

/// Notes of the same pitch never overlap, as in Live: a note that lies
/// wholly under another of its pitch goes (also one that starts with a
/// longer one), and one that runs on into the next of its pitch ends where
/// that one starts (or goes, when that would leave less than a 1/32).
/// Returns, when anything changed, where each old note went (None: gone).
pub fn tidy_notes(notes: &mut Vec<Note>) -> Option<Vec<Option<usize>>> {
    if notes.len() < 2 {
        return None;
    }
    const EPS: f64 = 1e-6;
    let mut order: Vec<usize> = (0..notes.len()).collect();
    order.sort_by(|&a, &b| notes[a].pitch.cmp(&notes[b].pitch).then(notes[a].start.total_cmp(&notes[b].start)).then(notes[b].length.total_cmp(&notes[a].length)));
    let mut gone = vec![false; notes.len()];
    let mut changed = false;
    let mut prev: Option<usize> = None;
    for &k in &order {
        let Some(p) = prev.filter(|&p| notes[p].pitch == notes[k].pitch) else {
            prev = Some(k);
            continue;
        };
        let p_end = notes[p].start + notes[p].length;
        let k_end = notes[k].start + notes[k].length;
        if notes[k].start < p_end - EPS {
            changed = true;
            if k_end <= p_end + EPS {
                // Wholly under the one before: it goes.
                gone[k] = true;
                continue;
            }
            let shortened = notes[k].start - notes[p].start;
            if shortened < MIN_TIDY_LENGTH {
                gone[p] = true;
            } else {
                notes[p].length = shortened;
            }
        }
        prev = Some(k);
    }
    if !changed {
        return None;
    }
    let mut map = Vec::with_capacity(notes.len());
    let mut next = 0;
    for g in &gone {
        if *g {
            map.push(None);
        } else {
            map.push(Some(next));
            next += 1;
        }
    }
    let mut i = 0;
    notes.retain(|_| {
        let keep = !gone[i];
        i += 1;
        keep
    });
    Some(map)
}

impl Note {
    pub fn new(pitch: u8, start: f64, length: f64, velocity: f32) -> Self {
        Self { pitch, start, length, velocity, chance: 1.0, vel_spread: 0.0, repeats: 1, recur_len: 0, recur_mask: 0xFF }
    }

    pub fn end(&self) -> f64 {
        self.start + self.length
    }

    /// Whether the note plays on loop `k` of its clip (recurrence).
    pub fn plays_on_loop(&self, k: i64) -> bool {
        let n = self.recur_len as i64;
        n < 2 || (self.recur_mask >> k.rem_euclid(n)) & 1 == 1
    }

    /// Whether any operator is set.
    pub fn has_operators(&self) -> bool {
        self.chance < 1.0 || self.vel_spread > 0.0 || self.repeats > 1 || self.recur_len >= 2
    }
}

/// A repeatable random number in 0..1 for note `idx` on loop `k` (so a
/// render sounds like playback); `salt` picks an independent stream.
pub fn note_random(k: i64, idx: usize, n: &Note, salt: u64) -> f32 {
    let mut x = (k as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (idx as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ ((n.pitch as u64) << 32 | (n.start * 960.0) as u64).wrapping_mul(0x1656_67B1_9E37_79F9)
        ^ salt.wrapping_mul(0x27D4_EB2F_1656_67C5);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    x = x.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    x ^= x >> 33;
    (x >> 40) as f32 / (1u64 << 24) as f32
}

/// An audio clip is always "warped" in Re-Pitch mode: it is locked to the
/// beat grid and speeds up / slows down with the project tempo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioClip {
    pub sample: SampleRef,
    /// Original tempo of the material.
    pub bpm: f64,
    pub gain_db: f32,
}

impl AudioClip {
    /// Length of the whole sample in beats.
    pub fn sample_beats(&self) -> f64 {
        self.sample.duration() * self.bpm / 60.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClipContent {
    Midi { notes: Vec<Note> },
    Audio(AudioClip),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub id: u64,
    pub name: String,
    pub color: Rgb,
    /// Loop length in beats.
    pub length: f64,
    pub content: ClipContent,
    /// Parameters that follow a line while the clip plays (Live's clip
    /// envelopes): drawn, or recorded from the knobs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub envelopes: Vec<Envelope>,
}

/// A clip envelope: what it moves, and the line it follows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub target: crate::modulation::ModTarget,
    /// (beat in the clip, 0..1 of the parameter's range), in order of beat.
    pub points: Vec<(f64, f32)>,
}

impl Envelope {
    /// The value at `beat`: straight lines between the points, flat before
    /// the first and after the last.
    pub fn value_at(&self, beat: f64) -> Option<f32> {
        let pts = &self.points;
        let first = pts.first()?;
        if beat <= first.0 {
            return Some(first.1);
        }
        let i = pts.partition_point(|p| p.0 <= beat);
        if i >= pts.len() {
            return pts.last().map(|p| p.1);
        }
        let (a, b) = (pts[i - 1], pts[i]);
        let t = if b.0 > a.0 { ((beat - a.0) / (b.0 - a.0)) as f32 } else { 1.0 };
        Some(a.1 + (b.1 - a.1) * t)
    }

    /// Puts a point at `beat` (replacing one within `snap` beats of it) and
    /// keeps the points in order.
    pub fn set(&mut self, beat: f64, value: f32, snap: f64) {
        let value = value.clamp(0.0, 1.0);
        self.points.retain(|p| (p.0 - beat).abs() > snap);
        let i = self.points.partition_point(|p| p.0 < beat);
        self.points.insert(i, (beat.max(0.0), value));
    }

    /// Leaves out the points between `from` and `to` that the line would
    /// pass within `tol` of anyway (Douglas-Peucker), so a recorded knob
    /// move keeps its shape with a handful of points.
    pub fn simplify(&mut self, from: f64, to: f64, tol: f32) {
        let lo = self.points.partition_point(|p| p.0 < from);
        let hi = self.points.partition_point(|p| p.0 <= to);
        if hi < lo + 3 {
            return;
        }
        let seg = self.points[lo..hi].to_vec();
        let mut keep = vec![false; seg.len()];
        keep[0] = true;
        keep[seg.len() - 1] = true;
        let mut todo = vec![(0, seg.len() - 1)];
        while let Some((a, b)) = todo.pop() {
            let (pa, pb) = (seg[a], seg[b]);
            let mut worst = (tol, None);
            for (i, p) in seg.iter().enumerate().take(b).skip(a + 1) {
                let f = if pb.0 > pa.0 { ((p.0 - pa.0) / (pb.0 - pa.0)) as f32 } else { 0.0 };
                let d = (p.1 - (pa.1 + (pb.1 - pa.1) * f)).abs();
                if d > worst.0 {
                    worst = (d, Some(i));
                }
            }
            if let (_, Some(i)) = worst {
                keep[i] = true;
                todo.push((a, i));
                todo.push((i, b));
            }
        }
        let mut i = 0;
        self.points.retain(|_| {
            let k = i < lo || i >= hi || keep[i - lo];
            i += 1;
            k
        });
    }
}

/// Volume envelopes run over the fader's travel (0 dB near 84%), as the
/// fader does.
pub fn fader_pos_to_db(pos: f32) -> f32 {
    let gmax = 10f32.powf(FADER_MAX_DB / 20.0);
    let g = (pos.clamp(0.0, 1.0) * gmax.powf(0.25)).powi(4);
    if g <= 1e-6 {
        return FADER_MIN_DB;
    }
    (20.0 * g.log10()).clamp(FADER_MIN_DB, FADER_MAX_DB)
}

pub fn db_to_fader_pos(db: f32) -> f32 {
    if db <= FADER_MIN_DB + 0.05 {
        return 0.0;
    }
    let g = 10f32.powf(db / 20.0);
    let gmax = 10f32.powf(FADER_MAX_DB / 20.0);
    (g.powf(0.25) / gmax.powf(0.25)).clamp(0.0, 1.0)
}

impl Clip {
    pub fn new_midi(id: u64, name: impl Into<String>, color: Rgb, length: f64) -> Self {
        Self { id, name: name.into(), color, length, content: ClipContent::Midi { notes: Vec::new() }, envelopes: Vec::new() }
    }

    /// Creates an audio clip, guessing the source tempo the way Live's
    /// auto-warp does for loops: snap to a power-of-two number of beats.
    pub fn new_audio(id: u64, sample: SampleRef, color: Rgb, project_bpm: f64) -> Self {
        let dur = sample.duration().max(0.001);
        let beats_at_project = dur * project_bpm / 60.0;
        let mut bpm = project_bpm;
        if dur > 1.5 {
            let mut best = 1.0f64;
            let mut b = 1.0f64;
            while b <= 256.0 {
                if (b - beats_at_project).abs() < (best - beats_at_project).abs() {
                    best = b;
                }
                b *= 2.0;
            }
            let candidate = best * 60.0 / dur;
            if (candidate / project_bpm - 1.0).abs() < 0.2 {
                bpm = candidate;
            }
        }
        let audio = AudioClip { sample: sample.clone(), bpm, gain_db: 0.0 };
        let length = audio.sample_beats().max(0.25);
        Self { id, name: sample.name.clone(), color, length, content: ClipContent::Audio(audio), envelopes: Vec::new() }
    }

    pub fn is_midi(&self) -> bool {
        matches!(self.content, ClipContent::Midi { .. })
    }

    pub fn notes(&self) -> &[Note] {
        match &self.content {
            ClipContent::Midi { notes } => notes,
            ClipContent::Audio(_) => &[],
        }
    }

    pub fn notes_mut(&mut self) -> Option<&mut Vec<Note>> {
        match &mut self.content {
            ClipContent::Midi { notes } => Some(notes),
            ClipContent::Audio(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArrangementClip {
    pub id: u64,
    /// Position on the timeline, in beats.
    pub start: f64,
    pub length: f64,
    /// Where inside the (looping) clip content playback begins.
    pub offset: f64,
    pub clip: Clip,
}

impl ArrangementClip {
    pub fn end(&self) -> f64 {
        self.start + self.length
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrumPad {
    pub name: String,
    pub voice: DrumVoice,
    pub note: u8,
    pub level_db: f32,
    pub tune: f32,
    pub decay: f32,
    pub tone: f32,
    pub pan: f32,
    pub sample: Option<SampleRef>,
    /// The voice's extra knobs (see `DrumVoice::macros`).
    #[serde(default = "half")]
    pub a: f32,
    #[serde(default = "half")]
    pub b: f32,
    /// Pads in the same group (1-4) cut each other off; 0: none.
    #[serde(default)]
    pub choke: u8,
    /// A recorded drum: velocity layers (soft to loud), each with its round
    /// robins. Empty: `sample` or the voice plays.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<Vec<SampleRef>>,
    /// Brings the recorded layers to a common level (dB), set when a kit
    /// is loaded; the pad's own level comes on top.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub trim_db: f32,
    /// What velocity does to the pad: how much it sets the loudness (0:
    /// always full, 1: all of it), and how much a hard hit brightens it and
    /// lengthens it (-1..1; negative turns it around).
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub vel_level: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub vel_tone: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub vel_decay: f32,
}

impl DrumPad {
    /// The gain velocity gives a hit (recorded layers carry most of the
    /// dynamics themselves).
    pub fn vel_gain(&self, vel: f32) -> f32 {
        let curve = if self.layers.is_empty() { vel.powf(1.3) } else { vel.powf(0.5) }.max(0.02);
        1.0 + (curve - 1.0) * self.vel_level.clamp(0.0, 1.0)
    }

    /// The tone and decay of a hit at `vel`, moved by the velocity knobs
    /// (around a usual velocity of 0.7).
    pub fn vel_tone_decay(&self, vel: f32) -> (f32, f32) {
        let dv = vel.clamp(0.0, 1.0) - 0.7;
        let tone = (self.tone + self.vel_tone.clamp(-1.0, 1.0) * dv * 1.2).clamp(0.0, 1.0);
        let decay = self.decay * 2f32.powf(self.vel_decay.clamp(-1.0, 1.0) * dv * 2.5);
        (tone, decay)
    }
}

fn half() -> f32 {
    0.5
}

impl DrumPad {
    pub fn new(voice: DrumVoice) -> Self {
        Self {
            name: voice.name().to_string(),
            voice,
            note: voice.gm_note(),
            level_db: match voice {
                DrumVoice::Kick | DrumVoice::AnalogKick | DrumVoice::SynthKick | DrumVoice::M808Kick | DrumVoice::M909Kick | DrumVoice::Cr78Kick => 0.0,
                DrumVoice::ClosedHat
                | DrumVoice::OpenHat
                | DrumVoice::MetalHat
                | DrumVoice::RingHat
                | DrumVoice::M808ClosedHat
                | DrumVoice::M808OpenHat
                | DrumVoice::M909ClosedHat
                | DrumVoice::M909OpenHat
                | DrumVoice::Cr78Hat
                | DrumVoice::M808Cymbal
                | DrumVoice::Cr78Cymbal => -6.0,
                DrumVoice::Cowbell | DrumVoice::Rim | DrumVoice::M808Cowbell | DrumVoice::M808Rim | DrumVoice::M909Rim | DrumVoice::M808Claves | DrumVoice::Cr78Metal => -8.0,
                v if v.family() == drums::Family::Shaker => -2.0,
                _ => -3.0,
            },
            tune: 0.0,
            decay: 1.0,
            tone: 0.5,
            pan: 0.0,
            sample: None,
            a: 0.5,
            b: 0.5,
            choke: 0,
            layers: Vec::new(),
            trim_db: 0.0,
            vel_level: 1.0,
            vel_tone: 0.0,
            vel_decay: 0.0,
        }
    }
}

/// One sample of a Sampler, played over a range of keys and velocities.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SampleZone {
    pub sample: SampleRef,
    pub lo_key: u8,
    pub hi_key: u8,
    /// The key that plays the sample at its own pitch.
    pub root: u8,
    #[serde(default)]
    pub lo_vel: u8,
    #[serde(default = "max_vel")]
    pub hi_vel: u8,
    /// Cents.
    #[serde(default)]
    pub tune: f32,
    #[serde(default)]
    pub gain_db: f32,
    #[serde(default)]
    pub pan: f32,
    /// Where it starts and ends, in frames (end 0: the end of the sample).
    #[serde(default)]
    pub start: u32,
    #[serde(default)]
    pub end: u32,
    /// A loop it always plays (from an SFZ), in frames.
    #[serde(default)]
    pub loop_range: Option<(u32, u32)>,
}

fn max_vel() -> u8 {
    127
}

/// A hosted plugin instrument (currently CLAP).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginInstrument {
    /// Unique per placed instance; links the project to the live plugin.
    pub instance: u64,
    pub format: String,
    pub plugin_id: String,
    pub name: String,
    pub vendor: String,
    pub path: PathBuf,
    /// The plugin's own state (patch, settings), base64, refreshed on save.
    #[serde(default)]
    pub state: Option<Arc<str>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Instrument {
    Synth { preset: String, params: Vec<f32> },
    /// A drum instrument: an engine's pads (808, 909, ... or a rack).
    Drums {
        kit: String,
        pads: Vec<DrumPad>,
        #[serde(default)]
        engine: drums::DrumEngine,
    },
    /// One sample on every key, or `zones` (when there are any).
    Sampler {
        params: Vec<f32>,
        sample: Option<SampleRef>,
        #[serde(default)]
        zones: Vec<SampleZone>,
    },
    Plugin(PluginInstrument),
    /// FM: a DX7-compatible voice (155 bytes, VCED) plus engine settings.
    Fm { voice: Vec<u8>, op_mask: u8, bank: String, params: Vec<f32> },
    /// Plaits: the Plaits macro-oscillator models, polyphonic.
    Macro { preset: String, params: Vec<f32> },
    /// Braids: the Braids macro-oscillator shapes, polyphonic.
    Braids { preset: String, params: Vec<f32> },
    /// Elements: the Elements modal voice (bow, blow, strike into a
    /// resonator), polyphonic.
    Elements { preset: String, params: Vec<f32> },
    /// Analog: a virtual analog polysynth modelled on classic analog synths.
    Analog { preset: String, params: Vec<f32> },
    /// Shakers: PhISEM shakers played with notes.
    Shakers { preset: String, params: Vec<f32> },
    /// Modular: a eurorack-style patch of modules and cables; `params` are
    /// the macros, voices and volume.
    Modular { preset: String, params: Vec<f32>, patch: crate::modular::Patch },
    /// Voice: A singing voice from formant filters, 32 vowels and consonants, from the Synthesis ToolKit.
    Voice { preset: String, params: Vec<f32> },
    /// Mallets: Marimba, vibraphone, agogo, woods, glass harmonica and Tibetan bowl: struck physical models from the Synthesis ToolKit.
    Mallets { preset: String, params: Vec<f32> },
    /// Strings: Bowed, plucked, mandolin, sitar and stiff strings: string physical models from the Synthesis ToolKit.
    Strings { preset: String, params: Vec<f32> },
    /// Winds: Flute, clarinet, saxophone, brass, blown bottle, blow hole and whistle: blown physical models from the Synthesis ToolKit.
    Winds { preset: String, params: Vec<f32> },
    /// Organ: Tonewheel organ: nine drawbars, percussion, key click, scanner vibrato and chorus, drive and a rotating speaker.
    Organ { preset: String, params: Vec<f32> },
    /// Rings: Resonator after Mutable Instruments Rings: modal, sympathetic and inharmonic strings, FM voice, chords, and the Disastrous Peace string synth.
    Rings { preset: String, params: Vec<f32> },
    /// Monk: a chanting voice of FOF grains, ported from MonkSynth.
    Monk { preset: String, params: Vec<f32> },
    /// Grand Piano: the Salamander Grand Piano samples (downloaded once).
    Piano { preset: String, params: Vec<f32> },
    /// Bassline: an acid bass synth, a port of Open303.
    Bassline { preset: String, params: Vec<f32> },
    /// Mono: the small mono synth of the early eighties.
    Mono101 { preset: String, params: Vec<f32> },
    /// Drone: tanpura strings, PADsynth pad, swelling FM pairs and air, slowly evolving.
    Drone { preset: String, params: Vec<f32> },
    /// Waves: a wavetable synth on the Adventure Kid Waveforms.
    Waves { preset: String, params: Vec<f32> },
    /// Guitar: six plucked waveguide strings, played like a guitarist would, acoustic or electric.
    Guitar { preset: String, params: Vec<f32> },
    /// Wind Chimes: a simulated wind chime that plays by itself, in aluminium, brass, steel, bamboo, glass or wood.
    WindChimes { preset: String, params: Vec<f32> },
    /// Birds: songbirds of fifteen species singing by themselves, near and far.
    Birds { preset: String, params: Vec<f32> },
    /// Nature: wind, rain, thunder, a brook, the sea, crickets, frogs and a fire, made from scratch.
    Nature { preset: String, params: Vec<f32> },
    /// Pan Flute: blown pipes: pan flute, shakuhachi, ocarina, bamboo and wooden flutes.
    PanFlute { preset: String, params: Vec<f32> },
    /// Melodica: a keyboard you blow into, free reeds sharing one breath.
    Melodica { preset: String, params: Vec<f32> },
    /// Harmonica: a diatonic harp, blown and drawn, bent and overblown.
    Harmonica { preset: String, params: Vec<f32> },
    /// One synth, six engines, four knobs (see `dsp::facet`).
    Facet { preset: String, params: Vec<f32> },
    /// Beads: the wavetable synth of Mutable Instruments Beads, played with keys, with its grains.
    BeadsSynth { preset: String, params: Vec<f32> },
}

impl Instrument {
    pub fn synth(preset: &str) -> Self {
        Instrument::Synth { preset: preset.to_string(), params: synth::preset(preset) }
    }

    pub fn drums(kit: &str) -> Self {
        Instrument::Drums { kit: kit.to_string(), pads: drums::kit(kit), engine: drums::engine_of(kit) }
    }

    pub fn sampler(sample: Option<SampleRef>) -> Self {
        Instrument::Sampler { params: params::defaults(sampler::SPECS), sample, zones: Vec::new() }
    }

    pub fn fm(voice: &Dx7Voice, bank: &str) -> Self {
        Instrument::Fm { voice: voice.0.to_vec(), op_mask: 0x3f, bank: bank.to_string(), params: params::defaults(fm::SPECS) }
    }

    pub fn macro_osc(preset: &str) -> Self {
        Instrument::Macro { preset: preset.to_string(), params: macro_osc::preset(preset) }
    }

    pub fn braids(preset: &str) -> Self {
        Instrument::Braids { preset: preset.to_string(), params: braids_synth::preset(preset) }
    }

    pub fn elements(preset: &str) -> Self {
        Instrument::Elements { preset: preset.to_string(), params: elements_synth::preset(preset) }
    }

    pub fn analog(preset: &str) -> Self {
        Instrument::Analog { preset: preset.to_string(), params: analog::preset(preset) }
    }

    pub fn modular(preset: &str) -> Self {
        let (params, patch) = crate::modular::presets::preset(preset);
        Instrument::Modular { preset: preset.to_string(), params, patch }
    }

    pub fn shakers(preset: &str) -> Self {
        Instrument::Shakers { preset: preset.to_string(), params: shaker_synth::preset(preset) }
    }

    pub fn voice(preset: &str) -> Self {
        Instrument::Voice { preset: preset.to_string(), params: stk_voice::preset(preset) }
    }

    pub fn mallets(preset: &str) -> Self {
        Instrument::Mallets { preset: preset.to_string(), params: stk_mallets::preset(preset) }
    }

    pub fn strings(preset: &str) -> Self {
        Instrument::Strings { preset: preset.to_string(), params: stk_strings::preset(preset) }
    }

    pub fn winds(preset: &str) -> Self {
        Instrument::Winds { preset: preset.to_string(), params: stk_winds::preset(preset) }
    }

    pub fn organ(preset: &str) -> Self {
        Instrument::Organ { preset: preset.to_string(), params: organ::preset(preset) }
    }

    pub fn rings(preset: &str) -> Self {
        Instrument::Rings { preset: preset.to_string(), params: rings_synth::preset(preset) }
    }

    pub fn piano(preset: &str) -> Self {
        Instrument::Piano { preset: preset.to_string(), params: crate::piano::preset(preset) }
    }

    pub fn monk(preset: &str) -> Self {
        Instrument::Monk { preset: preset.to_string(), params: crate::dsp::monk::preset(preset) }
    }

    pub fn bassline(preset: &str) -> Self {
        Instrument::Bassline { preset: preset.to_string(), params: crate::dsp::bassline::preset(preset) }
    }

    pub fn mono101(preset: &str) -> Self {
        Instrument::Mono101 { preset: preset.to_string(), params: crate::dsp::mono101::preset(preset) }
    }

    pub fn drone(preset: &str) -> Self {
        Instrument::Drone { preset: preset.to_string(), params: crate::dsp::drone::preset(preset) }
    }

    pub fn waves(preset: &str) -> Self {
        Instrument::Waves { preset: preset.to_string(), params: crate::dsp::waves::preset(preset) }
    }

    pub fn guitar(preset: &str) -> Self {
        Instrument::Guitar { preset: preset.to_string(), params: crate::dsp::guitar::preset(preset) }
    }

    pub fn wind_chimes(preset: &str) -> Self {
        Instrument::WindChimes { preset: preset.to_string(), params: crate::dsp::wind_chimes::preset(preset) }
    }

    pub fn birds(preset: &str) -> Self {
        Instrument::Birds { preset: preset.to_string(), params: crate::dsp::birds::preset(preset) }
    }

    pub fn nature(preset: &str) -> Self {
        Instrument::Nature { preset: preset.to_string(), params: crate::dsp::nature::preset(preset) }
    }

    pub fn pan_flute(preset: &str) -> Self {
        Instrument::PanFlute { preset: preset.to_string(), params: crate::dsp::pan_flute::preset(preset) }
    }

    pub fn melodica(preset: &str) -> Self {
        Instrument::Melodica { preset: preset.to_string(), params: crate::dsp::melodica::preset(preset) }
    }

    pub fn harmonica(preset: &str) -> Self {
        Instrument::Harmonica { preset: preset.to_string(), params: crate::dsp::harmonica::preset(preset) }
    }

    pub fn facet(preset: &str) -> Self {
        Instrument::Facet { preset: preset.to_string(), params: crate::dsp::facet::preset(preset) }
    }

    pub fn beads_synth(preset: &str) -> Self {
        Instrument::BeadsSynth { preset: preset.to_string(), params: crate::dsp::beads_synth::preset(preset) }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            Instrument::Synth { .. } => "Synth",
            Instrument::Drums { engine, .. } => engine.name(),
            Instrument::Sampler { .. } => "Sampler",
            Instrument::Plugin(_) => "CLAP Plugin",
            Instrument::Fm { .. } => "FM",
            Instrument::Macro { .. } => "Plaits",
            Instrument::Braids { .. } => "Braids",
            Instrument::Elements { .. } => "Elements",
            Instrument::Analog { .. } => "Analog",
            Instrument::Shakers { .. } => "Particles",
            Instrument::Modular { .. } => "Modular",
            Instrument::Voice { .. } => "Voice",
            Instrument::Mallets { .. } => "Mallets",
            Instrument::Strings { .. } => "Strings",
            Instrument::Winds { .. } => "Winds",
            Instrument::Organ { .. } => "Organ",
            Instrument::Rings { .. } => "Rings",
            Instrument::Monk { .. } => "Monk",
            Instrument::Piano { .. } => "Grand Piano",
            Instrument::Bassline { .. } => "Bassline",
            Instrument::Mono101 { .. } => "Mono",
            Instrument::Drone { .. } => "Drone",
            Instrument::Waves { .. } => "Waves",
            Instrument::Guitar { .. } => "Guitar",
            Instrument::WindChimes { .. } => "Wind Chimes",
            Instrument::BeadsSynth { .. } => "Beads",
            Instrument::Birds { .. } => "Birds",
            Instrument::Nature { .. } => "Nature",
            Instrument::PanFlute { .. } => "Pan Flute",
            Instrument::Melodica { .. } => "Melodica",
            Instrument::Harmonica { .. } => "Harmonica",
            Instrument::Facet { .. } => "Facet",
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Instrument::Synth { preset, .. } => preset.clone(),
            Instrument::Drums { kit, .. } => kit.clone(),
            Instrument::Sampler { zones, .. } if zones.len() > 1 => format!("{} zones", zones.len()),
            Instrument::Sampler { zones, .. } if zones.len() == 1 => zones[0].sample.name.clone(),
            Instrument::Sampler { sample, .. } => sample.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "Empty".into()),
            Instrument::Plugin(p) => p.name.clone(),
            Instrument::Fm { voice, .. } => Dx7Voice::from_slice(voice).map(|v| v.name()).unwrap_or_else(|| "FM".into()),
            Instrument::Macro { preset, .. } | Instrument::Braids { preset, .. } | Instrument::Elements { preset, .. } | Instrument::Analog { preset, .. } | Instrument::Shakers { preset, .. } | Instrument::Modular { preset, .. } | Instrument::Voice { preset, .. } | Instrument::Mallets { preset, .. } | Instrument::Strings { preset, .. } | Instrument::Winds { preset, .. } | Instrument::Organ { preset, .. } | Instrument::Rings { preset, .. } | Instrument::Monk { preset, .. } | Instrument::Piano { preset, .. } | Instrument::Bassline { preset, .. } | Instrument::Mono101 { preset, .. } | Instrument::Drone { preset, .. } | Instrument::Waves { preset, .. } | Instrument::Guitar { preset, .. } | Instrument::WindChimes { preset, .. } | Instrument::Birds { preset, .. } | Instrument::Nature { preset, .. } | Instrument::PanFlute { preset, .. } | Instrument::Melodica { preset, .. } | Instrument::Harmonica { preset, .. } | Instrument::Facet { preset, .. } | Instrument::BeadsSynth { preset, .. } => preset.clone(),
        }
    }

    /// The name a track gets for this instrument: the device ("Analog",
    /// "Drums"), or the plugin's name.
    pub fn track_name(&self) -> String {
        match self {
            Instrument::Plugin(p) => p.name.clone(),
            Instrument::Guitar { params, .. } => crate::dsp::guitar::family_name(params).to_string(),
            i => i.kind_name().to_string(),
        }
    }

    /// The notes a drum kit plays, a bit per note (0 for other instruments).
    pub fn drum_notes(&self) -> u128 {
        match self {
            Instrument::Drums { pads, .. } => pads.iter().fold(0, |m, p| m | 1u128 << (p.note & 127)),
            _ => 0,
        }
    }

    /// Whether the project's Transpose moves it (drums stay where they are).
    pub fn transposes(&self) -> bool {
        !self.is_drums() && self.drum_notes() == 0
    }

    pub fn is_drums(&self) -> bool {
        matches!(self, Instrument::Drums { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub id: u64,
    pub kind: EffectKind,
    pub enabled: bool,
    pub params: Vec<f32>,
    /// The plug-in of an `EffectKind::Plugin`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<PluginInstrument>,
    /// A sequencing effect's pattern (Glitch Sequencer).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data: Vec<u8>,
    /// Effects held inside this one (the Glitch Sequencer's slots).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Device>,
    /// The preset it was set to last (for the menu).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub preset: String,
    /// A sound file it plays with (Convolution's own answer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<SampleRef>,
}

impl Device {
    /// The name to show: the plug-in's, or the kind of effect.
    pub fn name(&self) -> &str {
        match &self.plugin {
            Some(p) => &p.name,
            None => self.kind.name(),
        }
    }
}

/// A MIDI effect in front of a track's instrument.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MidiDevice {
    pub id: u64,
    pub kind: crate::midi_fx::MidiFxKind,
    pub enabled: bool,
    pub params: Vec<f32>,
    /// Pattern data for sequencers (steps, Life board, ...).
    #[serde(default)]
    pub data: Vec<u8>,
    /// The preset it was set to last (for the menu).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub preset: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mixer {
    pub volume_db: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    /// One send level (0..1) per return track.
    pub sends: Vec<f32>,
}

impl Default for Mixer {
    fn default() -> Self {
        Self { volume_db: 0.0, pan: 0.0, mute: false, solo: false, sends: Vec::new() }
    }
}

/// Send knob (0..1) to linear gain.
pub fn send_gain(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    v * v
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackKind {
    Midi,
    Audio,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: u64,
    pub name: String,
    pub color: Rgb,
    pub kind: TrackKind,
    /// The Key drawer leaves this track where it is (it plays as written).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub key_fixed: bool,
    /// MIDI effects, processed in order before the instrument.
    #[serde(default)]
    pub midi_devices: Vec<MidiDevice>,
    pub instrument: Option<Instrument>,
    pub devices: Vec<Device>,
    pub mixer: Mixer,
    pub arm: bool,
    pub slots: Vec<Option<Clip>>,
    pub arrangement: Vec<ArrangementClip>,
    /// Where an audio track's input comes from ("Audio From").
    #[serde(default)]
    pub input: AudioInput,
    #[serde(default)]
    pub monitor: Monitor,
    /// Where the track's audio goes ("Audio To").
    #[serde(default)]
    pub output: AudioOutput,
    /// Which live notes a MIDI track takes ("MIDI From").
    #[serde(default)]
    pub midi_in: MidiInput,
    /// MIDI channel (0-15) to listen to; `None` is all channels.
    #[serde(default)]
    pub midi_channel: Option<u8>,
    /// Automation on the arrangement (as in Live): lines in song beats that
    /// move the mixer, the instrument and the effects while the track
    /// follows the arrangement.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub automation: Vec<Envelope>,
    /// The parameter whose line is drawn over the clips (Automation mode).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_shown: Option<crate::modulation::ModTarget>,
    /// More parameters, each in a lane of its own under the track.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub auto_lanes: Vec<crate::modulation::ModTarget>,
}

impl Track {
    /// The automation line of `target`, if it has one.
    pub fn automation_of(&self, target: crate::modulation::ModTarget) -> Option<&Envelope> {
        self.automation.iter().find(|e| e.target == target)
    }

    /// The automation line of `target`, made when it has none.
    pub fn automation_mut(&mut self, target: crate::modulation::ModTarget) -> &mut Envelope {
        if let Some(i) = self.automation.iter().position(|e| e.target == target) {
            return &mut self.automation[i];
        }
        self.automation.push(Envelope { target, points: Vec::new() });
        self.automation.last_mut().expect("just pushed")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReturnTrack {
    pub id: u64,
    pub name: String,
    pub color: Rgb,
    pub devices: Vec<Device>,
    pub mixer: Mixer,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MasterTrack {
    pub devices: Vec<Device>,
    pub volume_db: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
}

fn default_bpb() -> u32 {
    4
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub bpm: f64,
    #[serde(default = "default_bpb")]
    pub beats_per_bar: u32,
    pub tracks: Vec<Track>,
    pub returns: Vec<ReturnTrack>,
    pub master: MasterTrack,
    pub scenes: Vec<Scene>,
    pub launch_quantize: Quantize,
    #[serde(default)]
    pub metronome: bool,
    #[serde(default)]
    pub loop_enabled: bool,
    #[serde(default)]
    pub loop_start: f64,
    #[serde(default)]
    pub loop_end: f64,
    /// Project key and scale (Live 12 style).
    #[serde(default)]
    pub scale: Scale,
    /// Highlight the scale in the piano roll and enable scale-aware tools.
    #[serde(default = "default_true")]
    pub scale_mode: bool,
    /// The modulation matrix: modulators and their connections.
    #[serde(default)]
    pub modulators: Vec<crate::modulation::Modulator>,
    #[serde(default)]
    pub mod_routes: Vec<crate::modulation::ModRoute>,
    /// The arranger track: song sections over the arrangement.
    #[serde(default)]
    pub sections: Vec<SongSection>,
    pub next_id: u64,
    /// The four-track tape.
    #[serde(default)]
    pub tape: crate::tape::TapeSettings,
    /// The key everything melodic plays in right now (the Key drawer), when
    /// it is not the project's own: every note is moved degree by degree
    /// from `scale` into it, live. Drums stay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub play_key: Option<Scale>,
    /// A new play key waits for the next bar (while the song plays).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub play_key_on_bar: bool,
    /// A sequence of keys the song moves through, a step every
    /// `key_seq_bars` bars (None: its own key), while `key_seq_on` and the
    /// song plays.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_seq: Vec<Option<Scale>>,
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub key_seq_bars: u8,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub key_seq_on: bool,
    /// Heights the user gave devices in the Rack view, by device id (an
    /// instrument by its track's id). The engine ignores them.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub rack_heights: std::collections::BTreeMap<u64, f32>,
}

/// A part of the song on the arranger track (Intro, Verse, Chorus...), as in
/// Studio One: moving or copying it takes everything in it along.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SongSection {
    pub id: u64,
    pub name: String,
    pub start: f64,
    pub length: f64,
    pub color: Rgb,
}

impl SongSection {
    pub fn end(&self) -> f64 {
        self.start + self.length
    }
}

/// Names new sections get, in turn.
const SECTION_NAMES: [&str; 8] = ["Intro", "Verse", "Chorus", "Verse", "Chorus", "Bridge", "Chorus", "Outro"];
const SECTION_COLORS: [Rgb; 6] = [[0x5b, 0x8d, 0xd9], [0x4c, 0xb8, 0x7a], [0xe0, 0x8a, 0x3c], [0xb0, 0x6c, 0xd8], [0xd8, 0x5a, 0x7a], [0x3c, 0xb4, 0xc0]];

pub const FILE_EXTENSION: &str = "krab";

impl Project {
    /// A bare project: no tracks, two return tracks, eight scenes.
    pub fn blank() -> Self {
        let mut p = Project {
            name: "Untitled".into(),
            bpm: 120.0,
            beats_per_bar: 4,
            tracks: Vec::new(),
            returns: Vec::new(),
            master: MasterTrack { devices: Vec::new(), volume_db: 0.0 },
            scenes: (1..=8).map(|i| Scene { name: i.to_string() }).collect(),
            launch_quantize: Quantize::Bar,
            metronome: false,
            loop_enabled: false,
            loop_start: 0.0,
            loop_end: 16.0,
            scale: Scale::default(),
            scale_mode: true,
            modulators: Vec::new(),
            mod_routes: Vec::new(),
            sections: Vec::new(),
            next_id: 1,
            tape: Default::default(),
            play_key: None,
            play_key_on_bar: false,
            key_seq: Vec::new(),
            key_seq_bars: 1,
            key_seq_on: false,
            rack_heights: Default::default(),
        };
        let mut reverb = p.make_device(EffectKind::Reverb);
        reverb.params[5] = 1.0;
        let mut delay = p.make_device(EffectKind::Delay);
        delay.params[5] = 1.0;
        p.add_return("A Reverb", vec![reverb]);
        p.add_return("B Delay", vec![delay]);
        p
    }

    /// The default "new set": a drum track, a synth track and an audio track.
    /// The key the music plays in now: the Key drawer's, or the project's.
    pub fn sounding_key(&self) -> Scale {
        self.play_key.unwrap_or(self.scale)
    }

    /// How many beats a step of the key sequence lasts.
    pub fn key_seq_step_beats(&self) -> f64 {
        self.beats_per_bar_f().max(1.0) * self.key_seq_bars.max(1) as f64
    }

    /// The step of the key sequence at `beat` (the session clock).
    pub fn key_seq_step(&self, beat: f64) -> Option<usize> {
        let n = self.key_seq.len();
        (n > 0).then(|| ((beat / self.key_seq_step_beats()).floor().max(0.0) as usize) % n)
    }

    /// Makes the key the drawer plays in the project's own: every note of
    /// the melodic clips moves into it (see `convert_scale`). It sounds the
    /// same as before.
    pub fn bake_play_key(&mut self) {
        if let Some(k) = self.play_key.take() {
            self.convert_scale(k);
        }
    }

    pub fn new_default() -> Self {
        let mut p = Self::blank();
        p.add_midi_track(Some(Instrument::drums("Classic 909")));
        p.add_midi_track(Some(Instrument::synth("Fat Bass")));
        p.add_audio_track();
        p
    }

    /// Adds a section from `start` for `length` beats, named in turn.
    pub fn add_section(&mut self, start: f64, length: f64) -> u64 {
        let id = self.alloc_id();
        let k = self.sections.len();
        self.sections.push(SongSection { id, name: SECTION_NAMES[k % SECTION_NAMES.len()].into(), start, length: length.max(1.0), color: SECTION_COLORS[k % SECTION_COLORS.len()] });
        self.sections.sort_by(|a, b| a.start.total_cmp(&b.start));
        id
    }

    /// Cuts every arrangement clip that crosses `t` in two.
    pub fn split_arrangement_at(&mut self, t: f64) {
        for ti in 0..self.tracks.len() {
            let mut right_parts = Vec::new();
            for ac in self.tracks[ti].arrangement.iter_mut() {
                if ac.start < t - 1e-9 && ac.end() > t + 1e-9 {
                    let mut right = ac.clone();
                    right.start = t;
                    right.length = ac.end() - t;
                    right.offset = ac.offset + (t - ac.start);
                    ac.length = t - ac.start;
                    right_parts.push(right);
                }
            }
            for mut r in right_parts {
                r.id = self.alloc_id();
                r.clip.id = self.alloc_id();
                self.tracks[ti].arrangement.push(r);
            }
        }
    }

    /// Moves everything at or after `at` by `by` beats: arrangement clips,
    /// sections (but `keep`) and a loop that lies after it.
    fn shift_from(&mut self, at: f64, by: f64, keep: Option<u64>) {
        for t in &mut self.tracks {
            for ac in &mut t.arrangement {
                if ac.start >= at - 1e-9 {
                    ac.start += by;
                }
            }
        }
        for s in &mut self.sections {
            if Some(s.id) != keep && s.start >= at - 1e-9 {
                s.start += by;
            }
        }
        if self.loop_start >= at - 1e-9 {
            self.loop_start += by;
            self.loop_end += by;
        }
    }

    /// Takes the clips starting in `[a, b)` out of the arrangement.
    fn lift(&mut self, a: f64, b: f64) -> Vec<(usize, ArrangementClip)> {
        let mut out = Vec::new();
        for (ti, t) in self.tracks.iter_mut().enumerate() {
            let (inside, rest): (Vec<_>, Vec<_>) = t.arrangement.drain(..).partition(|ac| ac.start >= a - 1e-9 && ac.start < b - 1e-9);
            t.arrangement = rest;
            out.extend(inside.into_iter().map(|ac| (ti, ac)));
        }
        out
    }

    /// Moves section `id`, and everything in it, to start at `to`: the gap
    /// it leaves closes and room opens where it lands.
    pub fn move_section(&mut self, id: u64, to: f64) {
        let Some(s) = self.sections.iter().find(|s| s.id == id).cloned() else { return };
        let (a, b, len) = (s.start, s.end(), s.length);
        if to > a - 1e-9 && to < b + 1e-9 {
            return;
        }
        for t in [a, b, to] {
            self.split_arrangement_at(t);
        }
        let lifted = self.lift(a, b);
        self.shift_from(b, -len, Some(id));
        let to = if to > b { to - len } else { to };
        self.shift_from(to, len, Some(id));
        for (ti, mut ac) in lifted {
            ac.start = ac.start - a + to;
            self.tracks[ti].arrangement.push(ac);
        }
        if let Some(s) = self.sections.iter_mut().find(|s| s.id == id) {
            s.start = to;
        }
        self.sections.sort_by(|a, b| a.start.total_cmp(&b.start));
    }

    /// Copies section `id` and everything in it to start at `to` (right
    /// after it when `None`), making room there. Returns the new section.
    pub fn copy_section(&mut self, id: u64, to: Option<f64>) -> Option<u64> {
        let s = self.sections.iter().find(|s| s.id == id).cloned()?;
        let (a, b, len) = (s.start, s.end(), s.length);
        let to = to.unwrap_or(b);
        if to > a + 1e-9 && to < b - 1e-9 {
            return None;
        }
        for t in [a, b, to] {
            self.split_arrangement_at(t);
        }
        let copies: Vec<(usize, ArrangementClip)> = self
            .tracks
            .iter()
            .enumerate()
            .flat_map(|(ti, t)| t.arrangement.iter().filter(|ac| ac.start >= a - 1e-9 && ac.start < b - 1e-9).map(move |ac| (ti, ac.clone())))
            .collect();
        self.shift_from(to, len, None);
        for (ti, mut ac) in copies {
            ac.start = ac.start - a + to;
            ac.id = self.alloc_id();
            ac.clip.id = self.alloc_id();
            self.tracks[ti].arrangement.push(ac);
        }
        let new = self.alloc_id();
        self.sections.push(SongSection { id: new, name: s.name.clone(), start: to, length: len, color: s.color });
        self.sections.sort_by(|a, b| a.start.total_cmp(&b.start));
        Some(new)
    }

    /// Removes section `id`; with its content, the time it took closes too.
    pub fn delete_section(&mut self, id: u64, with_content: bool) {
        let Some(s) = self.sections.iter().find(|s| s.id == id).cloned() else { return };
        if with_content {
            self.split_arrangement_at(s.start);
            self.split_arrangement_at(s.end());
            self.lift(s.start, s.end());
            self.shift_from(s.end(), -s.length, Some(id));
        }
        self.sections.retain(|x| x.id != id);
    }

    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn beats_per_bar_f(&self) -> f64 {
        self.beats_per_bar.max(1) as f64
    }

    pub fn track_index(&self, id: u64) -> Option<usize> {
        self.tracks.iter().position(|t| t.id == id)
    }

    pub fn make_device(&mut self, kind: EffectKind) -> Device {
        let data = if kind == EffectKind::Glitch { crate::dsp::fx::glitch::default_data() } else { Vec::new() };
        Device { id: self.alloc_id(), kind, enabled: true, params: params::defaults(kind.specs()), plugin: None, data, children: Vec::new(), preset: String::new(), sample: None }
    }

    /// A device for an effect plug-in; `plugin.instance` is set here.
    pub fn make_plugin_device(&mut self, mut plugin: PluginInstrument) -> Device {
        plugin.instance = self.alloc_id();
        Device { id: self.alloc_id(), kind: EffectKind::Plugin, enabled: true, params: Vec::new(), plugin: Some(plugin), data: Vec::new(), children: Vec::new(), preset: String::new(), sample: None }
    }

    /// Every hosted plug-in (instruments and effects) by instance.
    pub fn plugins(&self) -> Vec<&PluginInstrument> {
        let mut v = Vec::new();
        for t in &self.tracks {
            if let Some(Instrument::Plugin(p)) = &t.instrument {
                v.push(p);
            }
        }
        let chains = self.tracks.iter().map(|t| &t.devices).chain(self.returns.iter().map(|r| &r.devices)).chain(std::iter::once(&self.master.devices));
        for devs in chains {
            v.extend(devs.iter().filter_map(|d| d.plugin.as_ref()));
        }
        v
    }

    /// The same, to change (e.g. to store their state).
    pub fn plugins_mut(&mut self) -> Vec<&mut PluginInstrument> {
        let mut v = Vec::new();
        for t in &mut self.tracks {
            if let Some(Instrument::Plugin(p)) = &mut t.instrument {
                v.push(p);
            }
            v.extend(t.devices.iter_mut().filter_map(|d| d.plugin.as_mut()));
        }
        for devs in self.returns.iter_mut().map(|r| &mut r.devices).chain(std::iter::once(&mut self.master.devices)) {
            v.extend(devs.iter_mut().filter_map(|d| d.plugin.as_mut()));
        }
        v
    }

    pub fn make_midi_device(&mut self, kind: crate::midi_fx::MidiFxKind) -> MidiDevice {
        let mut params = params::defaults(kind.specs());
        // Whatever plays the chords joins the band (older sets keep theirs apart).
        if let Some(b) = crate::sequencers::players::band_param(kind) {
            params[b] = 1.0;
        }
        MidiDevice { id: self.alloc_id(), kind, enabled: true, params, data: kind.default_data(), preset: String::new() }
    }

    pub fn add_return(&mut self, name: &str, devices: Vec<Device>) {
        let id = self.alloc_id();
        let color = PALETTE[(self.returns.len() * 5 + 8) % PALETTE.len()];
        self.returns.push(ReturnTrack { id, name: name.into(), color, devices, mixer: Mixer::default() });
        for t in &mut self.tracks {
            t.mixer.sends.push(0.0);
        }
    }

    fn new_track(&mut self, kind: TrackKind, name: String, instrument: Option<Instrument>) -> Track {
        let id = self.alloc_id();
        let color = PALETTE[(id as usize * 7) % PALETTE.len()];
        Track {
            id,
            name,
            color,
            kind,
            key_fixed: false,
            midi_devices: Vec::new(),
            instrument,
            devices: Vec::new(),
            mixer: Mixer { sends: vec![0.0; self.returns.len()], ..Mixer::default() },
            arm: false,
            slots: vec![None; self.scenes.len()],
            arrangement: Vec::new(),
            // Like Live, a new audio track listens to the first two inputs.
            input: if kind == TrackKind::Audio { AudioInput::Ext { channel: 0, stereo: true } } else { AudioInput::None },
            monitor: Monitor::Auto,
            output: AudioOutput::Master,
            midi_in: MidiInput::AllIns,
            midi_channel: None,
            automation: Vec::new(),
            auto_shown: None,
            auto_lanes: Vec::new(),
        }
    }

    pub fn add_midi_track(&mut self, instrument: Option<Instrument>) -> usize {
        let n = self.tracks.len() + 1;
        // Like Live, a track is named after its instrument, not the preset.
        let name = match &instrument {
            Some(i) => i.track_name(),
            None => format!("{n} MIDI"),
        };
        let t = self.new_track(TrackKind::Midi, name, instrument);
        self.tracks.push(t);
        self.tracks.len() - 1
    }

    pub fn add_audio_track(&mut self) -> usize {
        let n = self.tracks.len() + 1;
        let t = self.new_track(TrackKind::Audio, format!("{n} Audio"), None);
        self.tracks.push(t);
        self.tracks.len() - 1
    }

    /// Copy of a clip with fresh ids, for duplicating.
    pub fn fresh_clip(&mut self, clip: &Clip) -> Clip {
        let mut c = clip.clone();
        c.id = self.alloc_id();
        c
    }

    pub fn duplicate_track(&mut self, idx: usize) -> Option<usize> {
        let mut t = self.tracks.get(idx)?.clone();
        t.id = self.alloc_id();
        t.name = format!("{} copy", t.name);
        t.arm = false;
        for d in &mut t.devices {
            d.id = self.alloc_id();
            for c in &mut d.children {
                c.id = self.alloc_id();
            }
        }
        for d in &mut t.midi_devices {
            d.id = self.alloc_id();
        }
        if let Some(Instrument::Plugin(p)) = &mut t.instrument {
            p.instance = self.alloc_id();
        }
        for c in t.slots.iter_mut().flatten() {
            c.id = self.alloc_id();
        }
        for a in &mut t.arrangement {
            a.id = self.alloc_id();
            a.clip.id = self.alloc_id();
        }
        self.tracks.insert(idx + 1, t);
        Some(idx + 1)
    }

    pub fn add_scene(&mut self, at: usize) {
        let at = at.min(self.scenes.len());
        self.scenes.insert(at, Scene { name: (self.scenes.len() + 1).to_string() });
        for t in &mut self.tracks {
            t.slots.insert(at, None);
        }
    }

    pub fn duplicate_scene(&mut self, idx: usize) {
        if idx >= self.scenes.len() {
            return;
        }
        let name = format!("{} copy", self.scenes[idx].name);
        self.scenes.insert(idx + 1, Scene { name });
        for ti in 0..self.tracks.len() {
            let copy = self.tracks[ti].slots[idx].clone();
            let copy = copy.map(|c| self.fresh_clip(&c));
            self.tracks[ti].slots.insert(idx + 1, copy);
        }
    }

    pub fn remove_scene(&mut self, idx: usize) {
        if self.scenes.len() <= 1 || idx >= self.scenes.len() {
            return;
        }
        self.scenes.remove(idx);
        for t in &mut self.tracks {
            if idx < t.slots.len() {
                t.slots.remove(idx);
            }
        }
    }

    /// End of the last arrangement clip, in beats.
    pub fn arrangement_end(&self) -> f64 {
        self.tracks.iter().flat_map(|t| t.arrangement.iter().map(|a| a.end())).fold(0.0, f64::max)
    }

    /// Lays out every non-empty scene one after another on the timeline,
    /// appended after the current arrangement content.
    pub fn scenes_to_arrangement(&mut self) -> usize {
        let bar = self.beats_per_bar_f();
        let mut cursor = (self.arrangement_end() / bar).ceil() * bar;
        let mut placed = 0;
        for si in 0..self.scenes.len() {
            let len = self
                .tracks
                .iter()
                .filter_map(|t| t.slots.get(si).and_then(|s| s.as_ref()))
                .map(|c| c.length)
                .fold(0.0, f64::max);
            if len <= 0.0 {
                continue;
            }
            let len = (len / bar).ceil() * bar;
            for ti in 0..self.tracks.len() {
                let Some(Some(clip)) = self.tracks[ti].slots.get(si).cloned() else { continue };
                let id = self.alloc_id();
                let clip = self.fresh_clip(&clip);
                self.tracks[ti].arrangement.push(ArrangementClip { id, start: cursor, length: len, offset: 0.0, clip });
            }
            cursor += len;
            placed += 1;
        }
        placed
    }

    /// Changes the project key and moves every note of every melodic MIDI
    /// clip (session and arrangement) into the new scale. Drum tracks are
    /// left alone. Returns the number of notes that changed pitch.
    pub fn convert_scale(&mut self, to: Scale) -> usize {
        let from = self.scale;
        let mut changed = 0;
        for t in &mut self.tracks {
            if t.kind != TrackKind::Midi || t.instrument.as_ref().map(|i| i.is_drums()).unwrap_or(false) {
                continue;
            }
            let clips = t.slots.iter_mut().flatten().chain(t.arrangement.iter_mut().map(|a| &mut a.clip));
            for clip in clips {
                if let Some(notes) = clip.notes_mut() {
                    for n in notes.iter_mut() {
                        let p = convert_pitch(n.pitch, &from, &to);
                        if p != n.pitch {
                            n.pitch = p;
                            changed += 1;
                        }
                    }
                }
            }
        }
        self.scale = to;
        changed
    }

    /// Repairs invariants after loading or bulk edits.
    pub fn sanitize(&mut self) {
        if self.scenes.is_empty() {
            self.scenes.push(Scene { name: "1".into() });
        }
        if !(20.0..=999.0).contains(&self.bpm) {
            self.bpm = self.bpm.clamp(20.0, 999.0);
            if !self.bpm.is_finite() {
                self.bpm = 120.0;
            }
        }
        self.beats_per_bar = self.beats_per_bar.clamp(1, 16);
        let n_scenes = self.scenes.len();
        let n_returns = self.returns.len();
        let mut max_id = self.next_id;
        let fix_devices = |devs: &mut Vec<Device>, max_id: &mut u64| {
            // A plug-in device without its plug-in has nothing to play.
            devs.retain(|d| (d.kind == EffectKind::Plugin) == d.plugin.is_some());
            for d in devs.iter_mut() {
                // The first Glitch Sequencer's patterns and knobs were laid out otherwise.
                if d.kind == EffectKind::Glitch && crate::dsp::fx::glitch::migrate(&mut d.data) {
                    d.params = params::defaults(d.kind.specs());
                }
                params::fix_len(&mut d.params, d.kind.specs());
                *max_id = (*max_id).max(d.id + 1);
                if let Some(p) = &d.plugin {
                    *max_id = (*max_id).max(p.instance + 1);
                }
                // Only the Glitch Sequencer holds effects (and no plug-ins or
                // sequencers within), and only as many as it has slots.
                if d.kind != EffectKind::Glitch {
                    d.children.clear();
                }
                d.children.retain(|c| c.kind != EffectKind::Plugin && c.kind != EffectKind::Glitch);
                d.children.truncate(crate::dsp::fx::glitch::SLOTS);
                for c in d.children.iter_mut() {
                    params::fix_len(&mut c.params, c.kind.specs());
                    *max_id = (*max_id).max(c.id + 1);
                }
            }
        };
        for t in &mut self.tracks {
            t.slots.resize(n_scenes, None);
            t.mixer.sends.resize(n_returns, 0.0);
            max_id = max_id.max(t.id + 1);
            fix_devices(&mut t.devices, &mut max_id);
            for d in t.midi_devices.iter_mut() {
                params::fix_len(&mut d.params, d.kind.specs());
                max_id = max_id.max(d.id + 1);
            }
            match &mut t.instrument {
                Some(Instrument::Synth { params, .. }) => params::fix_len(params, synth::SPECS),
                Some(Instrument::Sampler { params, .. }) => params::fix_len(params, sampler::SPECS),
                Some(Instrument::Macro { params, .. }) => params::fix_len(params, macro_osc::SPECS),
                Some(Instrument::Braids { params, .. }) => params::fix_len(params, braids_synth::SPECS),
                Some(Instrument::Elements { params, .. }) => params::fix_len(params, elements_synth::SPECS),
                Some(Instrument::Analog { params, .. }) => params::fix_len(params, analog::SPECS),
                Some(Instrument::Shakers { params, .. }) => params::fix_len(params, shaker_synth::SPECS),
                Some(Instrument::Modular { params, patch, .. }) => {
                    params::fix_len(params, crate::modular::SPECS);
                    patch.sanitize();
                }
                Some(Instrument::Voice { params, .. }) => params::fix_len(params, stk_voice::SPECS),
                Some(Instrument::Mallets { params, .. }) => params::fix_len(params, stk_mallets::SPECS),
                Some(Instrument::Strings { params, .. }) => params::fix_len(params, stk_strings::SPECS),
                Some(Instrument::Winds { params, .. }) => params::fix_len(params, stk_winds::SPECS),
                Some(Instrument::Organ { params, .. }) => params::fix_len(params, organ::SPECS),
                Some(Instrument::Rings { params, .. }) => params::fix_len(params, rings_synth::SPECS),
                Some(Instrument::Monk { params, .. }) => params::fix_len(params, crate::dsp::monk::SPECS),
                Some(Instrument::Piano { params, .. }) => params::fix_len(params, crate::piano::SPECS),
                Some(Instrument::Bassline { params, .. }) => params::fix_len(params, crate::dsp::bassline::SPECS),
                Some(Instrument::Mono101 { params, .. }) => params::fix_len(params, crate::dsp::mono101::SPECS),
                Some(Instrument::Drone { params, .. }) => params::fix_len(params, crate::dsp::drone::SPECS),
                Some(Instrument::Waves { params, .. }) => params::fix_len(params, crate::dsp::waves::SPECS),
                Some(Instrument::Guitar { params, .. }) => params::fix_len(params, crate::dsp::guitar::SPECS),
                Some(Instrument::WindChimes { params, .. }) => params::fix_len(params, crate::dsp::wind_chimes::SPECS),
                Some(Instrument::BeadsSynth { params, .. }) => params::fix_len(params, crate::dsp::beads_synth::SPECS),
                Some(Instrument::Birds { params, .. }) => params::fix_len(params, crate::dsp::birds::SPECS),
                Some(Instrument::Nature { params, .. }) => params::fix_len(params, crate::dsp::nature::SPECS),
                Some(Instrument::PanFlute { params, .. }) => params::fix_len(params, crate::dsp::pan_flute::SPECS),
                Some(Instrument::Melodica { params, .. }) => params::fix_len(params, crate::dsp::melodica::SPECS),
                Some(Instrument::Harmonica { params, .. }) => params::fix_len(params, crate::dsp::harmonica::SPECS),
                Some(Instrument::Facet { params, .. }) => params::fix_len(params, crate::dsp::facet::SPECS),
                // Shakers became an instrument; their kits are the Drum Rack's.
                Some(Instrument::Drums { engine, .. }) if *engine == drums::DrumEngine::Shakers => *engine = drums::DrumEngine::Rack,
                Some(Instrument::Plugin(p)) => max_id = max_id.max(p.instance + 1),
                Some(Instrument::Fm { voice, params, .. }) => {
                    params::fix_len(params, fm::SPECS);
                    let fixed = Dx7Voice::from_slice(voice).unwrap_or_else(dx7::init_voice);
                    *voice = fixed.0.to_vec();
                }
                _ => {}
            }
            for c in t.slots.iter_mut().flatten() {
                c.length = c.length.max(0.0625);
                max_id = max_id.max(c.id + 1);
                // Notes hidden under others of their pitch (older recordings) go.
                if let Some(n) = c.notes_mut() {
                    tidy_notes(n);
                }
            }
            for a in &mut t.arrangement {
                a.length = a.length.max(0.0625);
                a.clip.length = a.clip.length.max(0.0625);
                if let Some(n) = a.clip.notes_mut() {
                    tidy_notes(n);
                }
                max_id = max_id.max(a.id + 1).max(a.clip.id + 1);
            }
        }
        self.fix_routing();
        for r in &mut self.returns {
            max_id = max_id.max(r.id + 1);
            fix_devices(&mut r.devices, &mut max_id);
        }
        fix_devices(&mut self.master.devices, &mut max_id);
        self.modulators.truncate(crate::modulation::MAX_MODS);
        for m in &mut self.modulators {
            params::fix_len(&mut m.params, m.kind.specs());
            max_id = max_id.max(m.id + 1);
        }
        for r in &mut self.mod_routes {
            r.amount = if r.amount.is_finite() { r.amount.clamp(-1.0, 1.0) } else { 0.0 };
            max_id = max_id.max(r.id + 1);
        }
        self.next_id = max_id;
        self.prune_mod_routes();
        if self.loop_end <= self.loop_start {
            self.loop_end = self.loop_start + 16.0;
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Loads a project. Missing samples are replaced by silence and reported
    /// in the second tuple element.
    pub fn load(path: &Path) -> Result<(Project, Vec<String>), String> {
        let json = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let guard = LoadCacheGuard::new();
        let mut p: Project = serde_json::from_str(&json).map_err(|e| format!("{}: {e}", path.display()))?;
        let missing = guard.missing();
        p.sanitize();
        Ok((p, missing))
    }
}

#[cfg(test)]
mod tidy_tests {
    use super::*;

    #[test]
    fn a_note_under_another_goes_and_one_running_on_is_shortened() {
        let mut notes = vec![
            Note::new(60, 12.0, 4.0, 0.8),
            // A tiny one under it (an overdub), and one at the same start.
            Note::new(60, 12.1, 0.03, 0.8),
            Note::new(60, 12.0, 0.5, 0.8),
            // Another pitch is left alone.
            Note::new(64, 12.1, 0.03, 0.8),
            // Running on into the next: shortened.
            Note::new(67, 0.0, 3.0, 0.8),
            Note::new(67, 2.0, 2.0, 0.8),
            // Starting a hair before the next: it goes.
            Note::new(72, 3.99, 0.5, 0.8),
            Note::new(72, 4.0, 2.0, 0.8),
        ];
        let map = tidy_notes(&mut notes).expect("changed");
        assert_eq!(map, vec![Some(0), None, None, Some(1), Some(2), Some(3), None, Some(4)]);
        assert_eq!(notes.len(), 5);
        assert_eq!((notes[0].pitch, notes[0].length), (60, 4.0));
        assert_eq!((notes[2].pitch, notes[2].length), (67, 2.0));
        assert_eq!((notes[4].pitch, notes[4].start), (72, 4.0));
        // Tidy notes stay as they are.
        assert!(tidy_notes(&mut notes).is_none());
        let mut chord = vec![Note::new(60, 0.0, 4.0, 0.8), Note::new(64, 0.0, 4.0, 0.8), Note::new(60, 4.0, 4.0, 0.8)];
        assert!(tidy_notes(&mut chord).is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_old_glitch_sequencer_is_brought_up_to_date() {
        let mut p = Project::new_default();
        let mut d = p.make_device(EffectKind::Glitch);
        // As the first version saved it: 25 knobs, 4 patterns of 32 steps.
        d.params = vec![0.5; 25];
        d.data = vec![0u8; 256];
        d.data[0] = 9; // Delay on the first step
        d.data[1] = 100;
        p.tracks[0].devices.push(d);
        p.sanitize();
        let d = p.tracks[0].devices.last().unwrap();
        assert_eq!(d.params, params::defaults(crate::dsp::fx::glitch::SPECS));
        assert!(crate::dsp::fx::glitch::is_current(&d.data));
        assert_eq!(crate::dsp::fx::glitch::blocks(&d.data, 0, 32)[0].module, 9);
        // A new one is left as it is.
        let fresh = p.make_device(EffectKind::Glitch);
        p.tracks[0].devices.push(fresh.clone());
        p.sanitize();
        assert_eq!(p.tracks[0].devices.last().unwrap().data, fresh.data);
    }

    #[test]
    fn simplify_keeps_the_shape_of_a_recorded_move_with_few_points() {
        let mut e = Envelope { target: crate::modulation::ModTarget::Volume(1), points: vec![(0.0, 0.5)] };
        // A ramp up, then a ramp down, sampled every 1/64 beat.
        for k in 1..=128 {
            let b = k as f64 / 64.0;
            let v = if b <= 1.0 { 0.5 + 0.4 * b as f32 } else { 0.9 - 0.4 * (b - 1.0) as f32 };
            e.set(b, v, 1e-6);
        }
        e.set(3.0, 0.2, 1e-6);
        let before = e.clone();
        e.simplify(0.0, 2.0, 0.004);
        assert!(e.points.len() <= 5, "{:?}", e.points);
        assert_eq!(e.points.first(), before.points.first());
        assert_eq!(e.points.last(), Some(&(3.0, 0.2)), "points outside the range stay");
        for k in 0..=300 {
            let b = k as f64 / 100.0;
            let (x, y) = (before.value_at(b).unwrap(), e.value_at(b).unwrap());
            assert!((x - y).abs() < 0.005, "beat {b}: {x} vs {y}");
        }
    }

    #[test]
    fn note_names_follow_live() {
        assert_eq!(note_name(60), "C3");
        assert_eq!(note_name(36), "C1");
        assert_eq!(note_name(69), "A3");
    }

    #[test]
    fn scenes_and_tracks_stay_in_sync() {
        let mut p = Project::new_default();
        p.add_scene(2);
        assert!(p.tracks.iter().all(|t| t.slots.len() == p.scenes.len()));
        p.remove_scene(0);
        assert!(p.tracks.iter().all(|t| t.slots.len() == p.scenes.len()));
        p.add_return("C", vec![]);
        assert!(p.tracks.iter().all(|t| t.mixer.sends.len() == 3));
    }

    #[test]
    fn save_load_roundtrip() {
        let p = crate::demo::demo_project();
        let dir = std::env::temp_dir().join(format!("krabton-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("roundtrip.krab");
        p.save(&path).unwrap();
        let (q, missing) = Project::load(&path).unwrap();
        assert!(missing.is_empty());
        assert_eq!(p, q);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn convert_scale_moves_melodic_clips_but_not_drums() {
        use crate::scale::ScaleKind;
        let mut p = crate::demo::demo_project();
        let drums_before = p.tracks[0].clone();
        let bass_first = p.tracks[1].slots[1].as_ref().unwrap().notes()[0].pitch;
        let n = p.convert_scale(Scale::new(0, ScaleKind::Major));
        assert!(n > 0);
        assert_eq!(p.tracks[0], drums_before);
        let bass_after = p.tracks[1].slots[1].as_ref().unwrap().notes()[0].pitch;
        assert_eq!(bass_after, bass_first + 3, "A becomes C");
        let c = Scale::new(0, ScaleKind::Major);
        for t in &p.tracks[1..] {
            for clip in t.slots.iter().flatten() {
                assert!(clip.notes().iter().all(|n| c.contains(n.pitch)));
            }
        }
    }

    #[test]
    fn scenes_to_arrangement_places_clips() {
        let mut p = crate::demo::demo_project();
        let placed = p.scenes_to_arrangement();
        assert!(placed > 0);
        assert!(p.arrangement_end() > 0.0);
    }

    /// A project with one MIDI track and arrangement clips at the given
    /// (start, length) in beats, each holding one note at its start.
    fn arranged(clips: &[(f64, f64)]) -> Project {
        let mut p = Project::blank();
        let t = p.add_midi_track(None);
        for (k, &(start, length)) in clips.iter().enumerate() {
            let id = p.alloc_id();
            let cid = p.alloc_id();
            let mut clip = Clip::new_midi(cid, format!("c{k}"), [1, 2, 3], length);
            if let Some(v) = clip.notes_mut() {
                v.push(Note::new(60 + k as u8, 0.0, 1.0, 0.8));
            }
            p.tracks[t].arrangement.push(ArrangementClip { id, start, length, offset: 0.0, clip });
        }
        p
    }

    fn starts(p: &Project) -> Vec<(String, f64)> {
        let mut v: Vec<(String, f64)> = p.tracks[0].arrangement.iter().map(|a| (a.clip.name.clone(), a.start)).collect();
        v.sort_by(|a, b| a.1.total_cmp(&b.1));
        v
    }

    #[test]
    fn sections_move_copy_and_delete_with_their_music() {
        // Intro 0..16 (clip c0), Verse 16..32 (c1), Chorus 32..48 (c2).
        let mut p = arranged(&[(0.0, 16.0), (16.0, 16.0), (32.0, 16.0)]);
        let intro = p.add_section(0.0, 16.0);
        let verse = p.add_section(16.0, 16.0);
        let chorus = p.add_section(32.0, 16.0);
        assert_eq!(p.sections.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["Intro", "Verse", "Chorus"]);
        // The chorus to the front.
        p.move_section(chorus, 0.0);
        assert_eq!(starts(&p), vec![("c2".into(), 0.0), ("c0".into(), 16.0), ("c1".into(), 32.0)]);
        assert_eq!(p.sections.iter().map(|s| (s.id, s.start)).collect::<Vec<_>>(), vec![(chorus, 0.0), (intro, 16.0), (verse, 32.0)]);
        // A copy of the verse right after it.
        let copy = p.copy_section(verse, None).unwrap();
        assert_eq!(starts(&p).last().unwrap(), &("c1".to_string(), 48.0));
        assert_eq!(p.sections.iter().find(|s| s.id == copy).unwrap().start, 48.0);
        // Deleting the intro with its music closes the gap.
        p.delete_section(intro, true);
        assert_eq!(starts(&p), vec![("c2".into(), 0.0), ("c1".into(), 16.0), ("c1".into(), 32.0)]);
    }

    #[test]
    fn a_clip_across_a_section_edge_is_cut_in_two() {
        let mut p = arranged(&[(8.0, 16.0)]);
        let s = p.add_section(16.0, 8.0);
        p.add_section(0.0, 16.0);
        p.move_section(s, 0.0);
        let mut v = p.tracks[0].arrangement.clone();
        v.sort_by(|a, b| a.start.total_cmp(&b.start));
        // The second half (from beat 16, 8 beats into the clip) moved to the front.
        assert_eq!((v[0].start, v[0].length, v[0].offset), (0.0, 8.0, 8.0));
        assert_eq!((v[1].start, v[1].length, v[1].offset), (16.0, 8.0, 0.0));
    }


    #[test]
    fn old_shaker_drum_tracks_open_in_the_drum_rack() {
        let mut p = Project::blank();
        let t = p.add_midi_track(None);
        p.tracks[t].instrument = Some(Instrument::Drums { kit: "Shakers".into(), pads: drums::kit("Shakers"), engine: drums::DrumEngine::Shakers });
        p.sanitize();
        assert!(matches!(&p.tracks[t].instrument, Some(Instrument::Drums { engine: drums::DrumEngine::Rack, pads, .. }) if pads.len() == 8));
        // And the Shakers instrument keeps its parameters in shape.
        p.tracks[t].instrument = Some(Instrument::Shakers { preset: "Maraca".into(), params: vec![0.0; 3] });
        p.sanitize();
        assert!(matches!(&p.tracks[t].instrument, Some(Instrument::Shakers { params, .. }) if params.len() == crate::dsp::shaker_synth::SPECS.len()));
    }


    #[test]
    fn a_plugin_device_saves_and_loads() {
        let mut p = Project::blank();
        let t = p.add_midi_track(None);
        let fx = PluginInstrument { instance: 0, format: "clap".into(), plugin_id: "org.example.verb".into(), name: "Verb".into(), vendor: "Example".into(), path: "/usr/lib/clap/verb.clap".into(), state: Some("c3RhdGU=".into()) };
        let dev = p.make_plugin_device(fx);
        assert_eq!(dev.name(), "Verb");
        p.tracks[t].devices.push(dev);
        p.master.devices.push(Device { id: 999, kind: EffectKind::Plugin, enabled: true, params: Vec::new(), plugin: None, data: Vec::new(), children: Vec::new() , preset: String::new(), sample: None });
        let json = serde_json::to_string(&p).unwrap();
        let mut back: Project = serde_json::from_str(&json).unwrap();
        back.sanitize();
        assert_eq!(back.tracks[t].devices, p.tracks[t].devices);
        // A plug-in device that lost its plug-in is dropped.
        assert!(back.master.devices.is_empty());
        assert_eq!(back.plugins().len(), 1);
        // Built-in effects do not write a plugin field.
        let eq = p.make_device(EffectKind::Eq3);
        assert!(!serde_json::to_string(&eq).unwrap().contains("plugin"));
    }

}
