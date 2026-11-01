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
    /// What a Singer sings on it (a syllable, in romaji or kana, or a voicebank's alias).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub lyric: String,
}

fn one() -> f32 {
    1.0
}
fn is_one(v: &f32) -> bool {
    *v == 1.0
}
fn is_zero64(v: &f64) -> bool {
    *v == 0.0
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

/// Presets that were renamed, and what they are called now (Analog's had
/// other names in 0.11.1 only).
fn renamed_preset(name: &str) -> Option<&'static str> {
    Some(match name {
        "Ladder Bass" => "Moog Bass",
        "Chorus Pad" => "Juno Pad",
        "Chorus Strings" => "Juno Strings",
        "Poly Brass" => "Prophet Brass",
        "Da Da Da" => "Toy March",
        "8580 Pad" => "Clean Pad",
        "Hubbard Bass" => "Driving Bass",
        _ => return None,
    })
}

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
        Self { pitch, start, length, velocity, chance: 1.0, vel_spread: 0.0, repeats: 1, recur_len: 0, recur_mask: 0xFF, lyric: String::new() }
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

/// An audio clip, as in Live: a sample with a Start and End marker, a loop,
/// and warping that keeps it on the beat at any tempo.
///
/// Its own time runs in beats ("clip beats"): the warp markers say which
/// second of the sample falls on which beat (straight lines between them,
/// and the tempo `bpm` before the first and after the last; no markers is
/// one straight line from the sample's start). Warped, a clip beat lasts a
/// beat of the song; the warp mode says how the audio is stretched to fit.
/// Unwarped, the sample plays at its own speed from the Start marker.
///
/// The loop runs `Clip::length` beats from `loop_start`. Older sets know
/// only `bpm` and the gain: warped, Re-Pitch, from the start, looping.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioClip {
    pub sample: SampleRef,
    /// The tempo of the material (Live's Seg. BPM): where there are no
    /// warp markers, or before the first and after the last.
    pub bpm: f64,
    pub gain_db: f32,
    /// Locked to the beat grid (Live's Warp switch).
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub warp: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub mode: crate::warp::WarpMode,
    /// (clip beat, second in the sample), in order of both.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<(f64, f64)>,
    /// The Start marker, in clip beats: where playing begins.
    #[serde(default, skip_serializing_if = "is_zero64")]
    pub start: f64,
    /// The End marker, in clip beats, where it stops when it does not loop
    /// (None: the sample's end).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<f64>,
    /// It loops (from `loop_start`, `Clip::length` beats); otherwise it
    /// plays once from the Start to the End marker.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub looping: bool,
    #[serde(default, skip_serializing_if = "is_zero64")]
    pub loop_start: f64,
    /// Beats: where it is cut, at the transients or on a grid.
    #[serde(default, skip_serializing_if = "is_default")]
    pub preserve: crate::warp::Preserve,
    /// Beats: what a piece does when it ends before the next begins.
    #[serde(default, skip_serializing_if = "is_default")]
    pub transient_loop: crate::warp::TransientLoop,
    /// Beats: 1 lets each piece ring, lower fades it out sooner.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub transient_env: f32,
    /// Tones and Texture: the size of the grains (0..1, 5 to 80 ms).
    #[serde(default = "default_grain", skip_serializing_if = "is_default_grain")]
    pub grain: f32,
    /// Texture: how far the grains wander (0..1).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub flux: f32,
    /// Pitch, in semitones and cents: warped it keeps the tempo; Re-Pitch
    /// and unwarped it plays faster or slower, as a record would.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub transpose: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub detune: f32,
    /// Plays the sample backwards (its markers mirrored).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reverse: bool,
    /// A session clip's fade in from where it starts and fade out before its
    /// End marker, in beats (arrangement clips have their own).
    #[serde(default, skip_serializing_if = "is_zero64")]
    pub fade_in: f64,
    #[serde(default, skip_serializing_if = "is_zero64")]
    pub fade_out: f64,
    /// The shape of the fades: 0 equal power, up to 1 quicker, down to -1
    /// slower.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub fade_curve: f32,
    /// A millisecond fade wherever the audio is cut (Live's Clip Fade).
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub declick: bool,
    /// -1 left .. 1 right.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub pan: f32,
    /// Where the loop goes round: a crossfade this long, in milliseconds.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub loop_xfade: f32,
}

fn is_true(v: &bool) -> bool {
    *v
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

fn default_grain() -> f32 {
    0.4
}

fn is_default_grain(v: &f32) -> bool {
    *v == default_grain()
}

impl AudioClip {
    /// A clip of `sample` at `bpm`, warped in Re-Pitch from its start.
    pub fn new(sample: SampleRef, bpm: f64) -> Self {
        Self {
            sample,
            bpm,
            gain_db: 0.0,
            warp: true,
            mode: Default::default(),
            markers: Vec::new(),
            start: 0.0,
            end: None,
            looping: true,
            loop_start: 0.0,
            preserve: Default::default(),
            transient_loop: Default::default(),
            transient_env: 1.0,
            grain: default_grain(),
            flux: 0.0,
            transpose: 0.0,
            detune: 0.0,
            reverse: false,
            fade_in: 0.0,
            fade_out: 0.0,
            fade_curve: 0.0,
            declick: true,
            pan: 0.0,
            loop_xfade: 0.0,
        }
    }

    /// Length of the whole sample in beats.
    pub fn sample_beats(&self) -> f64 {
        self.sample_end_beat() - self.sample_start_beat()
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
    /// The track's sound kept in the clip: when the clip starts, the track
    /// plays with it (and while it plays, what changes on the track goes
    /// into it). None: the track sounds as it does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound: Option<Box<Sound>>,
    /// A MIDI clip's groove (from the Groove Pool): its notes play with that
    /// feel, where they are written (see `groove`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groove: Option<crate::groove::ClipGroove>,
}

/// A track's sound: its MIDI effects, its instrument and its audio effects,
/// with all their settings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sound {
    #[serde(default)]
    pub midi_devices: Vec<MidiDevice>,
    #[serde(default)]
    pub instrument: Option<Instrument>,
    #[serde(default)]
    pub devices: Vec<Device>,
}

/// A track's sound as the engine reads it: its own or a clip's.
#[derive(Clone, Copy, Debug)]
pub struct SoundRef<'a> {
    pub midi_devices: &'a [MidiDevice],
    pub instrument: Option<&'a Instrument>,
    pub devices: &'a [Device],
}

impl SoundRef<'_> {
    /// Whether it is `s`, setting for setting.
    pub fn is(&self, s: &Sound) -> bool {
        self.instrument == s.instrument.as_ref() && self.midi_devices == s.midi_devices.as_slice() && self.devices == s.devices.as_slice()
    }
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
        Self { id, name: name.into(), color, length, content: ClipContent::Midi { notes: Vec::new() }, envelopes: Vec::new(), sound: None, groove: None }
    }

    /// Creates an audio clip warped the way Live's auto-warp does it. A loop
    /// (a power-of-two number of beats at about the tempo heard in it, or
    /// else the project's) gets exactly that tempo and plays in Beats mode.
    /// A longer recording with a steady beat (a song) gets the tempo found in
    /// it, a warp marker on its first downbeat and Complex mode, and plays
    /// once. A short one-shot follows the tempo in Re-Pitch, as before; a
    /// long one without a beat plays unwarped.
    pub fn new_audio(id: u64, sample: SampleRef, color: Rgb, project_bpm: f64) -> Self {
        use crate::warp::WarpMode;
        let dur = sample.duration().max(0.001);
        let mut audio = AudioClip::new(sample.clone(), project_bpm);
        let tempo = if dur > 1.5 { sample.analysis().tempo.filter(|t| t.confidence > 0.2) } else { None };
        let mut as_loop = None;
        // A tempo in its name ("Melody A minor 150 BPM") is the loop's own.
        let named = crate::analysis::bpm_in_name(&sample.name);
        if dur > 1.5 && dur <= 32.0 {
            let mut best: Option<(f64, f64)> = None;
            let mut beats = 1.0f64;
            while beats <= 256.0 {
                let bpm = beats * 60.0 / dur;
                // How far from the tempo named or heard (an octave off costs a little), or the project's.
                let err = match (named, tempo) {
                    (Some(n), _) => (bpm / n - 1.0).abs(),
                    (None, Some(t)) => [(1.0, 0.0), (2.0, 0.01), (0.5, 0.01)].iter().map(|&(o, pen)| (bpm / (t.bpm * o)).ln().abs() + pen).fold(f64::MAX, f64::min),
                    (None, None) => (bpm / project_bpm - 1.0).abs(),
                };
                if (40.0..=250.0).contains(&bpm) && best.is_none_or(|b| err < b.1) {
                    best = Some((bpm, err));
                }
                beats *= 2.0;
            }
            let tol = if tempo.is_some() || named.is_some() { 0.03 } else { 0.2 };
            as_loop = best.filter(|b| b.1 < tol).map(|b| b.0);
        }
        let length = match (as_loop, tempo) {
            (Some(bpm), _) => {
                audio.bpm = bpm;
                audio.mode = WarpMode::Beats;
                audio.sample_beats()
            }
            (None, Some(t)) if dur > 8.0 => {
                audio.bpm = t.bpm;
                audio.mode = WarpMode::Complex;
                audio.markers = vec![(0.0, t.downbeat)];
                // From the bar before the sound starts, so the downbeat stays on the grid.
                audio.start = (audio.sample_start_beat() / 4.0).floor() * 4.0;
                audio.loop_start = audio.start;
                audio.looping = false;
                audio.sample_end_beat() - audio.start
            }
            _ if dur > 8.0 => {
                audio.warp = false;
                audio.looping = false;
                audio.sample_beats()
            }
            _ => audio.sample_beats(),
        };
        Self { id, name: sample.name.clone(), color, length: length.max(0.25), content: ClipContent::Audio(audio), envelopes: Vec::new(), sound: None, groove: None }
    }

    /// How long it plays before it repeats or ends, in beats: the loop, or
    /// (an audio clip that does not loop) from its Start to its End marker.
    pub fn play_length(&self) -> f64 {
        match &self.content {
            ClipContent::Audio(a) if !a.looping => (a.end_beat() - a.start).max(1.0 / 64.0),
            _ => self.length.max(1.0 / 64.0),
        }
    }

    /// An arrangement clip's `offset` (how far into its playing it begins)
    /// brought back into its first time round, for a clip that loops.
    pub fn fold_offset(&self, offset: f64) -> f64 {
        let len = self.length.max(1e-6);
        match &self.content {
            ClipContent::Audio(a) if !a.looping => offset,
            // Up to the loop's end first, then round and round.
            ClipContent::Audio(a) => {
                let first = a.loop_start + len - a.start;
                if (a.start - a.loop_start).abs() < 1e-9 || first <= 0.0 {
                    offset.rem_euclid(len)
                } else if offset < first {
                    offset
                } else {
                    first + (offset - first).rem_euclid(len)
                }
            }
            ClipContent::Midi { .. } => offset.rem_euclid(len),
        }
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
    /// An audio clip's fades at its start and end, in beats.
    #[serde(default, skip_serializing_if = "is_zero64")]
    pub fade_in: f64,
    #[serde(default, skip_serializing_if = "is_zero64")]
    pub fade_out: f64,
    /// Switched off: it stays where it is and does not play.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub muted: bool,
}

impl ArrangementClip {
    /// A clip at `start`, `length` beats long, from the start of its content.
    pub fn new(id: u64, start: f64, length: f64, clip: Clip) -> Self {
        Self { id, start, length, offset: 0.0, clip, fade_in: 0.0, fade_out: 0.0, muted: false }
    }

    pub fn end(&self) -> f64 {
        self.start + self.length
    }

    /// Its gain at song beat `g` from its fades (0..1, an equal-power curve).
    pub fn fade_gain(&self, g: f64) -> f32 {
        let mut x = 1.0f64;
        if self.fade_in > 1e-9 {
            x = x.min((g - self.start) / self.fade_in);
        }
        if self.fade_out > 1e-9 {
            x = x.min((self.end() - g) / self.fade_out);
        }
        (x.clamp(0.0, 1.0) * std::f64::consts::FRAC_PI_2).sin() as f32
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
    /// Brings the recorded layers (or the sample) to a common level (dB),
    /// set when a kit is loaded (and on the slices of a loop, so they play
    /// as loud as the loop); the pad's own level comes on top.
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
    /// Plays only while its note is held (a slice stops where its note
    /// ends); off: every hit plays to its end, as drums do.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub gate: bool,
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
        if voice.family() == drums::Family::Keytone {
            // The toy keyboard's samples made now, not on the audio thread.
            crate::dsp::keytone::warm();
        }
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
                | DrumVoice::M909Ride
                | DrumVoice::M909Crash
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
            gate: false,
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
    /// Calculator: the calculator with a keyboard of 1981, its ten sounds, stepped envelope and rhythm box.
    #[serde(alias = "VlTone")]
    Calculator { preset: String, params: Vec<f32> },
    /// Console: an 8-bit console's two pulses, triangle and noise, layered, split or arpeggiated.
    #[serde(alias = "Nes")]
    Console { preset: String, params: Vec<f32> },
    /// Handheld: two pulses, the wave channel and noise, each left, right or both.
    #[serde(alias = "GameBoy")]
    Handheld { preset: String, params: Vec<f32> },
    /// Micro: a home computer's three voices, ring, sync and the multimode filter.
    #[serde(alias = "Sid")]
    Micro { preset: String, params: Vec<f32> },
    /// Warp: phase distortion, two lines, eight-step envelopes (see `dsp::warp`).
    Warp { preset: String, params: Vec<f32> },
    /// Hybrid: two oscillators of nine kinds through two filters of eight (see `dsp::hybrid`).
    Hybrid { preset: String, params: Vec<f32> },
    /// Chip FM: the four-operator FM chips of 16-bit consoles and arcade boards, emulated (see `dsp::chipfm`).
    ChipFm { preset: String, params: Vec<f32> },
    /// V2: farbrausch's demoscene synth, three oscillators, two filters, a matrix, effects (see `dsp::v2`).
    V2 { preset: String, params: Vec<f32> },
    /// Oidos: additive synthesis, hundreds of partials in random modes (see `dsp::oidos`).
    Oidos { preset: String, params: Vec<f32> },
    /// Quartet: four oscillators of eleven kinds, two filters, five envelopes, three LFOs, a matrix (see `dsp::quartet`).
    Quartet { preset: String, params: Vec<f32> },
    /// Orchestra: sampled orchestral instruments (see `orchestra`).
    Orchestra { preset: String, params: Vec<f32> },
    /// One synth, six engines, four knobs (see `dsp::facet`).
    Facet { preset: String, params: Vec<f32> },
    /// Coast: a west-coast voice, folded and frequency modulated, through a low pass gate (see `dsp::coast`).
    Coast { preset: String, params: Vec<f32> },
    /// Crimson: a virtual analog with a hypersaw, wavetables, two filters, unison and effects (see `dsp::crimson`).
    Crimson { preset: String, params: Vec<f32> },
    /// Silicon: a synth that sounds on the emulated DSP56300 with Krabton's own firmware (see `dsp::silicon`).
    Silicon { preset: String, params: Vec<f32> },
    /// Beads: the wavetable synth of Mutable Instruments Beads, played with keys, with its grains.
    BeadsSynth { preset: String, params: Vec<f32> },
    /// Singer: sings its notes' lyrics with a UTAU voicebank (see `singer`).
    Singer {
        params: Vec<f32>,
        bank: Option<crate::singer::VoicebankRef>,
    },
    /// Hyper: a Virus emulated by its own firmware, which the player brings
    /// (see `hyper`): a factory preset (bank, program) and the knobs.
    Hyper {
        params: Vec<f32>,
        firmware: Option<crate::hyper::FirmwareRef>,
        #[serde(default)]
        bank: u16,
        #[serde(default)]
        program: u16,
        #[serde(default)]
        preset: String,
        /// The device booted from the firmware (not saved: it boots again).
        #[serde(skip)]
        engine: crate::hyper::HyperRef,
    },
    /// Street Organ: a Dutch street organ's pipes, divisions and drums (see `dsp::street_organ`).
    StreetOrgan { preset: String, params: Vec<f32> },
    /// SF2 Player: a preset (bank, program) of a SoundFont 2 bank (see `dsp::sf2`).
    Sf2 {
        params: Vec<f32>,
        font: Option<crate::dsp::sf2::Sf2Ref>,
        #[serde(default)]
        bank: u16,
        #[serde(default)]
        patch: u8,
        /// The preset's name, as the bank calls it.
        #[serde(default)]
        preset: String,
    },
    /// Theremin: two radio oscillators beating, played gliding, with vibrato and a hand (see `dsp::theremin`).
    Theremin { preset: String, params: Vec<f32> },
    /// Granulator: a sample played as clouds of grains, one per key (see
    /// `dsp::granulator`); without a sample, a sound of its own (Source).
    Granulator {
        preset: String,
        params: Vec<f32>,
        #[serde(default)]
        sample: Option<SampleRef>,
    },
    /// Keytone: a toy sampling keyboard, its 8-bit sounds and a sample of
    /// your own in the user slot (see `dsp::keytone`).
    Keytone {
        preset: String,
        params: Vec<f32>,
        #[serde(default)]
        sample: Option<SampleRef>,
    },
    /// Analog Princess, the machine: deep-fried pluggnb drums with a step
    /// sequencer and a FRY section (see `dsp::princess_machine`).
    Princess {
        preset: String,
        params: Vec<f32>,
        #[serde(default)]
        machine: crate::dsp::princess_machine::Machine,
    },
    /// Stretch: a sample stretched 2 to 1000 times, every key a voice of
    /// it at the key's pitch (see `dsp::stretch`); without a sample, a sound
    /// of its own.
    Stretch {
        preset: String,
        params: Vec<f32>,
        #[serde(default)]
        sample: Option<SampleRef>,
    },
    /// Turntable: a DJ deck with a record of any sample on it, scratched
    /// by hand, synced to the song, with hot cues (see `dsp::turntable`).
    Turntable {
        preset: String,
        params: Vec<f32>,
        #[serde(default)]
        sample: Option<SampleRef>,
        #[serde(default)]
        record: crate::dsp::turntable::Record,
    },
    /// Grit: a pad sampler of the old kind, sixteen pads through the
    /// signal path of the late-eighties machines (see `dsp::grit`).
    Grit {
        preset: String,
        params: Vec<f32>,
        #[serde(default)]
        pads: Vec<crate::dsp::grit::GritPad>,
    },
    /// Flipside (Princess Flip before): a sample flipped for pluggnb,
    /// chopped over the keys, in the song's key (see `dsp::princess_flip`).
    /// Its tag stays `Flip`, so the sets made with it open as they were.
    Flip {
        preset: String,
        params: Vec<f32>,
        #[serde(default)]
        sample: Option<SampleRef>,
        #[serde(default)]
        flip: crate::dsp::princess_flip::Flip,
    },
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

    pub fn calculator(preset: &str) -> Self {
        Instrument::Calculator { preset: preset.to_string(), params: crate::dsp::calculator::preset(preset) }
    }

    pub fn console(preset: &str) -> Self {
        Instrument::Console { preset: preset.to_string(), params: crate::dsp::console::preset(preset) }
    }

    pub fn handheld(preset: &str) -> Self {
        Instrument::Handheld { preset: preset.to_string(), params: crate::dsp::handheld::preset(preset) }
    }

    pub fn micro(preset: &str) -> Self {
        Instrument::Micro { preset: preset.to_string(), params: crate::dsp::micro::preset(preset) }
    }

    pub fn warp(preset: &str) -> Self {
        Instrument::Warp { preset: preset.to_string(), params: crate::dsp::warp::preset(preset) }
    }

    pub fn hybrid(preset: &str) -> Self {
        Instrument::Hybrid { preset: preset.to_string(), params: crate::dsp::hybrid::preset(preset) }
    }

    pub fn quartet(preset: &str) -> Self {
        Instrument::Quartet { preset: preset.to_string(), params: crate::dsp::quartet::preset(preset) }
    }

    pub fn orchestra(preset: &str) -> Self {
        Instrument::Orchestra { preset: preset.to_string(), params: crate::orchestra::preset(preset) }
    }

    pub fn oidos(preset: &str) -> Self {
        Instrument::Oidos { preset: preset.to_string(), params: crate::dsp::oidos::preset(preset) }
    }

    pub fn v2(preset: &str) -> Self {
        Instrument::V2 { preset: preset.to_string(), params: crate::dsp::v2::preset(preset) }
    }

    pub fn chipfm(preset: &str) -> Self {
        Instrument::ChipFm { preset: preset.to_string(), params: crate::dsp::chipfm::preset(preset) }
    }

    pub fn facet(preset: &str) -> Self {
        Instrument::Facet { preset: preset.to_string(), params: crate::dsp::facet::preset(preset) }
    }

    pub fn coast(preset: &str) -> Self {
        Instrument::Coast { preset: preset.to_string(), params: crate::dsp::coast::preset(preset) }
    }

    pub fn crimson(preset: &str) -> Self {
        crate::dsp::waves::warm();
        Instrument::Crimson { preset: preset.to_string(), params: crate::dsp::crimson::preset(preset) }
    }

    pub fn silicon(preset: &str) -> Self {
        // Its firmware assembled now, not on the audio thread.
        let _ = crate::dsp::silicon::firmware();
        Instrument::Silicon { preset: preset.to_string(), params: crate::dsp::silicon::preset(preset) }
    }

    pub fn theremin(preset: &str) -> Self {
        Instrument::Theremin { preset: preset.to_string(), params: crate::dsp::theremin::preset(preset) }
    }

    pub fn singer(bank: Option<crate::singer::VoicebankRef>) -> Self {
        Instrument::Singer { params: params::defaults(crate::singer::SPECS), bank }
    }

    /// Hyper on a firmware's first preset, its knobs as the preset has them.
    pub fn hyper(firmware: Option<crate::hyper::FirmwareRef>) -> Self {
        let first = firmware.as_ref().and_then(|f| f.preset(0, 0)).cloned();
        Instrument::Hyper {
            params: first.as_ref().map(|p| p.knobs()).unwrap_or_else(|| params::defaults(crate::hyper::SPECS)),
            firmware,
            bank: 0,
            program: 0,
            preset: first.map(|p| p.name).unwrap_or_default(),
            engine: crate::hyper::HyperRef::fresh(),
        }
    }

    pub fn street_organ(preset: &str) -> Self {
        crate::dsp::street_organ::warm();
        Instrument::StreetOrgan { preset: preset.to_string(), params: crate::dsp::street_organ::preset(preset) }
    }

    /// The SF2 Player on a preset of `font` (its name looked up).
    pub fn sf2(font: Option<crate::dsp::sf2::Sf2Ref>, bank: u16, patch: u8) -> Self {
        let preset = font.as_ref().and_then(|f| f.preset(bank, patch)).map(|p| p.name.clone()).unwrap_or_default();
        Instrument::Sf2 { params: params::defaults(crate::dsp::sf2::SPECS), font, bank, patch, preset }
    }

    pub fn beads_synth(preset: &str) -> Self {
        Instrument::BeadsSynth { preset: preset.to_string(), params: crate::dsp::beads_synth::preset(preset) }
    }

    pub fn granulator(preset: &str) -> Self {
        // Its own sounds made now, not on the audio thread.
        crate::dsp::granulator::warm();
        Instrument::Granulator { preset: preset.to_string(), params: crate::dsp::granulator::preset(preset), sample: None }
    }

    pub fn keytone(preset: &str) -> Self {
        // Its sounds made now, not on the audio thread.
        crate::dsp::keytone::warm();
        Instrument::Keytone { preset: preset.to_string(), params: crate::dsp::keytone::preset(preset), sample: None }
    }

    pub fn turntable(preset: &str) -> Self {
        Instrument::Turntable { preset: preset.to_string(), params: crate::dsp::turntable::preset(preset), sample: None, record: Default::default() }
    }

    pub fn flip(preset: &str) -> Self {
        Instrument::Flip { preset: preset.to_string(), params: crate::dsp::princess_flip::preset(preset), sample: None, flip: Default::default() }
    }

    pub fn grit(preset: &str) -> Self {
        crate::dsp::grit::warm();
        Instrument::Grit { preset: preset.to_string(), params: crate::dsp::grit::preset(preset), pads: crate::dsp::grit::preset_pads(preset) }
    }

    pub fn princess(preset: &str) -> Self {
        use crate::dsp::princess_machine as pm;
        Instrument::Princess { preset: preset.to_string(), params: pm::preset(preset), machine: pm::preset_machine(preset) }
    }

    pub fn stretch(preset: &str) -> Self {
        // Its transforms and own sound made now, not on the audio thread.
        crate::dsp::stretch::warm();
        Instrument::Stretch { preset: preset.to_string(), params: crate::dsp::stretch::preset(preset), sample: None }
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
            Instrument::Calculator { .. } => "Calculator",
            Instrument::Console { .. } => "Console",
            Instrument::Handheld { .. } => "Handheld",
            Instrument::Micro { .. } => "Micro",
            Instrument::Warp { .. } => "Warp",
            Instrument::Hybrid { .. } => "Hybrid",
            Instrument::ChipFm { .. } => "Chip FM",
            Instrument::V2 { .. } => "V2",
            Instrument::Oidos { .. } => "Oidos",
            Instrument::Quartet { .. } => "Quartet",
            Instrument::Orchestra { params, .. } => crate::orchestra::device_name(params),
            Instrument::Facet { .. } => "Facet",
            Instrument::Coast { .. } => "Coast",
            Instrument::Crimson { .. } => "Crimson",
            Instrument::Silicon { .. } => "Silicon",
            Instrument::Sf2 { .. } => "SF2 Player",
            Instrument::StreetOrgan { .. } => "Street Organ",
            Instrument::Singer { .. } => "Singer",
            Instrument::Hyper { .. } => "Hyper",
            Instrument::Theremin { .. } => "Theremin",
            Instrument::Granulator { .. } => "Granulator",
            Instrument::Keytone { .. } => "Keytone",
            Instrument::Princess { .. } => "Analog Princess",
            Instrument::Stretch { .. } => "Stretch",
            Instrument::Turntable { .. } => "Turntable",
            Instrument::Grit { .. } => "Grit",
            Instrument::Flip { .. } => "Flipside",
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
            Instrument::Sf2 { preset, .. } if !preset.is_empty() => preset.clone(),
            Instrument::Sf2 { font, .. } => font.as_ref().map(|f| f.name.clone()).unwrap_or_else(|| "Empty".into()),
            Instrument::Singer { bank, .. } => bank.as_ref().map(|b| b.name.clone()).unwrap_or_else(|| "Own voice".into()),
            Instrument::Hyper { preset, .. } if !preset.is_empty() => preset.clone(),
            Instrument::Hyper { .. } => "No firmware".into(),
            Instrument::Fm { voice, .. } => Dx7Voice::from_slice(voice).map(|v| v.name()).unwrap_or_else(|| "FM".into()),
            Instrument::Coast { preset, .. } => preset.clone(),
            Instrument::Crimson { preset, .. } => preset.clone(),
            Instrument::Silicon { preset, .. } => preset.clone(),
            Instrument::Theremin { preset, .. } => preset.clone(),
            Instrument::Granulator { preset, .. } => preset.clone(),
            Instrument::Keytone { preset, .. } => preset.clone(),
            Instrument::Princess { preset, .. } => preset.clone(),
            Instrument::Stretch { preset, .. } => preset.clone(),
            Instrument::Turntable { sample: Some(s), .. } => s.name.clone(),
            Instrument::Turntable { preset, .. } => preset.clone(),
            Instrument::Grit { preset, .. } => preset.clone(),
            Instrument::Flip { sample: Some(s), .. } => s.name.clone(),
            Instrument::Flip { preset, .. } => preset.clone(),
            Instrument::Macro { preset, .. } | Instrument::Braids { preset, .. } | Instrument::Elements { preset, .. } | Instrument::Analog { preset, .. } | Instrument::Shakers { preset, .. } | Instrument::Modular { preset, .. } | Instrument::Voice { preset, .. } | Instrument::Mallets { preset, .. } | Instrument::Strings { preset, .. } | Instrument::Winds { preset, .. } | Instrument::Organ { preset, .. } | Instrument::Rings { preset, .. } | Instrument::Monk { preset, .. } | Instrument::Piano { preset, .. } | Instrument::Bassline { preset, .. } | Instrument::Mono101 { preset, .. } | Instrument::Drone { preset, .. } | Instrument::Waves { preset, .. } | Instrument::Guitar { preset, .. } | Instrument::WindChimes { preset, .. } | Instrument::Birds { preset, .. } | Instrument::Nature { preset, .. } | Instrument::PanFlute { preset, .. } | Instrument::Melodica { preset, .. } | Instrument::Harmonica { preset, .. } | Instrument::Calculator { preset, .. } | Instrument::Console { preset, .. } | Instrument::Handheld { preset, .. } | Instrument::Micro { preset, .. } | Instrument::Warp { preset, .. } | Instrument::Hybrid { preset, .. } | Instrument::Quartet { preset, .. } | Instrument::ChipFm { preset, .. } | Instrument::V2 { preset, .. } | Instrument::Oidos { preset, .. } | Instrument::Orchestra { preset, .. } | Instrument::Facet { preset, .. } | Instrument::BeadsSynth { preset, .. } | Instrument::StreetOrgan { preset, .. } => preset.clone(),
        }
    }

    /// The name a track gets for this instrument: the device ("Analog",
    /// "Drums"), or the plugin's name.
    pub fn track_name(&self) -> String {
        match self {
            Instrument::Plugin(p) => p.name.clone(),
            Instrument::Guitar { params, .. } => crate::dsp::guitar::family_name(params).to_string(),
            // A bank's preset is the instrument (a piano, a kit, a choir).
            Instrument::Sf2 { preset, .. } if !preset.is_empty() => preset.clone(),
            // A voicebank is the singer.
            Instrument::Singer { bank: Some(b), .. } => b.name.clone(),
            i => i.kind_name().to_string(),
        }
    }

    /// The notes a drum kit plays, a bit per note (0 for other instruments).
    pub fn drum_notes(&self) -> u128 {
        match self {
            Instrument::Drums { pads, .. } => pads.iter().fold(0, |m, p| m | 1u128 << (p.note & 127)),
            Instrument::Sf2 { bank, .. } if *bank >= crate::dsp::sf2::DRUM_BANK => crate::dsp::sf2::GM_DRUM_NOTES,
            Instrument::Princess { .. } => crate::dsp::princess_machine::NOTES.iter().fold(0, |m, &n| m | 1u128 << n),
            Instrument::Turntable { .. } => crate::dsp::turntable::note_names().iter().fold(0, |m, &(n, _)| m | 1u128 << n),
            Instrument::Grit { .. } => (0..crate::dsp::grit::PADS).fold(0, |m, k| m | 1u128 << (crate::dsp::grit::FIRST_NOTE as usize + k)),
            Instrument::Flip { flip, .. } => (0..flip.chops.len().min(crate::dsp::princess_flip::MAX_CHOPS)).fold(0, |m, k| m | 1u128 << (crate::dsp::princess_flip::FIRST_NOTE as usize + k)),
            _ => 0,
        }
    }

    /// The keys of a melodic instrument that play drums (the song's key
    /// leaves them where they are), a bit each.
    pub fn drum_keys(&self) -> u128 {
        match self {
            Instrument::StreetOrgan { .. } => crate::dsp::street_organ::DRUM_KEYS,
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
    /// A Looper's memory, shared with the engine (made by the UI, not saved:
    /// the loop is saved as `sample`).
    #[serde(skip)]
    pub looper: crate::dsp::fx::looper::LoopHandle,
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
    /// Where it stands on the Map (None: not on it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<crate::dsp::fx::stage::Place>,
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
    /// Its height in the arrangement (0: the one all lanes have).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub lane_h: f32,
    /// A clock of its own (Sync off): its session clips, MIDI effects and
    /// effects that keep time run at this tempo instead of the project's,
    /// drifting against the other tracks. None: in sync with the project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tempo: Option<f64>,
    /// Where its notes also go out (MIDI To): a MIDI port, with its clock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub midi_to: Option<MidiTo>,
}

/// A MIDI port a track plays (a synth, a drum machine): its notes after its
/// MIDI effects, on a channel, and (Clock on) the clock, Start and Stop at
/// the track's tempo, its own when it has one. Of the tracks on one port with
/// Clock on, the first gives the port its clock.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MidiTo {
    /// The port's name.
    pub port: String,
    /// 0..15.
    #[serde(default)]
    pub channel: u8,
    #[serde(default = "default_true")]
    pub clock: bool,
}

impl Track {
    /// Its sound now, to keep in a clip; None while a CLAP plug-in is part of
    /// it (a plug-in keeps its own state, which a clip can't switch).
    pub fn sound_now(&self) -> Option<Sound> {
        let plugin = matches!(self.instrument, Some(Instrument::Plugin(_))) || self.devices.iter().any(|d| d.plugin.is_some());
        (!plugin).then(|| Sound { midi_devices: self.midi_devices.clone(), instrument: self.instrument.clone(), devices: self.devices.clone() })
    }

    /// The sound kept in its clip `clip` (a session or an arrangement clip).
    pub fn clip_sound(&self, clip: u64) -> Option<&Sound> {
        let c = self.slots.iter().flatten().find(|c| c.id == clip).or_else(|| self.arrangement.iter().map(|a| &a.clip).find(|c| c.id == clip));
        c.and_then(|c| c.sound.as_deref())
    }

    /// The same, to change.
    pub fn clip_sound_mut(&mut self, clip: u64) -> Option<&mut Sound> {
        let c = match self.slots.iter_mut().flatten().find(|c| c.id == clip) {
            Some(c) => Some(c),
            None => self.arrangement.iter_mut().map(|a| &mut a.clip).find(|c| c.id == clip),
        };
        c.and_then(|c| c.sound.as_deref_mut())
    }

    /// The sound it plays with: the one in clip `from` if that has one, else its own.
    pub fn sound(&self, from: Option<u64>) -> SoundRef<'_> {
        match from.and_then(|id| self.clip_sound(id)) {
            Some(s) => SoundRef { midi_devices: &s.midi_devices, instrument: s.instrument.as_ref(), devices: &s.devices },
            None => SoundRef { midi_devices: &self.midi_devices, instrument: self.instrument.as_ref(), devices: &self.devices },
        }
    }

    /// Takes the sound kept in clip `clip`. Returns whether anything changed.
    pub fn take_clip_sound(&mut self, clip: u64) -> bool {
        let Some(s) = self.clip_sound(clip) else { return false };
        if self.sound(None).is(s) {
            return false;
        }
        let s = s.clone();
        self.midi_devices = s.midi_devices;
        self.instrument = s.instrument;
        self.devices = s.devices;
        true
    }

    /// Keeps its sound in clip `clip` (which has one already). Returns whether it changed.
    pub fn update_clip_sound(&mut self, clip: u64) -> bool {
        match self.clip_sound(clip) {
            Some(s) if !self.sound(None).is(s) => {}
            _ => return false,
        }
        let Some(now) = self.sound_now() else { return false };
        if let Some(s) = self.clip_sound_mut(clip) {
            *s = now;
        }
        true
    }

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
    /// The Map's room (the tracks on it stand there).
    #[serde(default)]
    pub map: crate::dsp::fx::stage::MapRoom,
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
    /// Devices folded to a strip in the device chain, by device id (an
    /// instrument by its track's id). The engine ignores them.
    #[serde(default, skip_serializing_if = "std::collections::BTreeSet::is_empty")]
    pub folded: std::collections::BTreeSet<u64>,
    /// The band's parts (Verse, Chorus...) and its form (none: the chord
    /// players' one progression).
    #[serde(default, skip_serializing_if = "crate::band::BandSong::is_empty")]
    pub band: crate::band::BandSong,
    /// The Video: the films and pictures that play along, and what in the
    /// music moves them. The engine ignores it.
    #[serde(default, skip_serializing_if = "crate::video::VideoSet::is_empty")]
    pub video: crate::video::VideoSet,
    /// The Groove Pool: grooves taken from clips (Extract Groove), beside the
    /// ones Krabton comes with, for MIDI clips to play with.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grooves: Vec<crate::groove::Groove>,
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
            map: Default::default(),
            play_key: None,
            play_key_on_bar: false,
            key_seq: Vec::new(),
            key_seq_bars: 1,
            key_seq_on: false,
            rack_heights: Default::default(),
            folded: Default::default(),
            band: Default::default(),
            video: Default::default(),
            grooves: Vec::new(),
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
        p.add_midi_track(Some(Instrument::drums("Classic Punch")));
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
                    right.fade_in = 0.0;
                    right.fade_out = right.fade_out.min(right.length);
                    ac.length = t - ac.start;
                    ac.fade_out = 0.0;
                    ac.fade_in = ac.fade_in.min(ac.length);
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

    /// Cuts the clip `id` on track `track` in two at `at` (inside it).
    /// Returns the new right part's id.
    pub fn split_clip(&mut self, track: usize, id: u64, at: f64) -> Option<u64> {
        let t = self.tracks.get(track)?;
        let ac = t.arrangement.iter().find(|a| a.id == id)?;
        if at <= ac.start + 1e-6 || at >= ac.end() - 1e-6 {
            return None;
        }
        let mut right = ac.clone();
        right.start = at;
        right.length = ac.end() - at;
        right.offset = right.clip.fold_offset(ac.offset + (at - ac.start));
        right.fade_in = 0.0;
        right.fade_out = right.fade_out.min(right.length);
        right.id = self.alloc_id();
        right.clip.id = self.alloc_id();
        let new = right.id;
        let ac = self.tracks[track].arrangement.iter_mut().find(|a| a.id == id)?;
        ac.length = at - ac.start;
        ac.fade_out = 0.0;
        ac.fade_in = ac.fade_in.min(ac.length);
        self.tracks[track].arrangement.push(right);
        Some(new)
    }

    /// Puts a point at `t` on every automation line that has points on
    /// both sides (its value there), so a cut keeps the line's shape.
    fn split_automation_at(&mut self, t: f64) {
        for tr in &mut self.tracks {
            for e in &mut tr.automation {
                let before = e.points.iter().any(|p| p.0 < t - 1e-9);
                let after = e.points.iter().any(|p| p.0 > t + 1e-9);
                let there = e.points.iter().any(|p| (p.0 - t).abs() < 1e-9);
                if before && after && !there {
                    if let Some(v) = e.value_at(t) {
                        let at = e.points.iter().position(|p| p.0 > t).unwrap_or(e.points.len());
                        e.points.insert(at, (t, v));
                    }
                }
            }
        }
    }

    /// Takes the automation points in `[a, b)` out: (track, line, points).
    fn lift_automation(&mut self, a: f64, b: f64) -> Vec<(usize, usize, Vec<(f64, f32)>)> {
        let mut out = Vec::new();
        for (ti, tr) in self.tracks.iter_mut().enumerate() {
            for (ei, e) in tr.automation.iter_mut().enumerate() {
                let (inside, rest): (Vec<_>, Vec<_>) = e.points.drain(..).partition(|p| p.0 >= a - 1e-9 && p.0 < b - 1e-9);
                e.points = rest;
                if !inside.is_empty() {
                    out.push((ti, ei, inside));
                }
            }
        }
        out
    }

    /// Puts lifted automation points back, moved by `by` beats.
    fn drop_automation(&mut self, lifted: Vec<(usize, usize, Vec<(f64, f32)>)>, by: f64) {
        for (ti, ei, pts) in lifted {
            if let Some(e) = self.tracks.get_mut(ti).and_then(|t| t.automation.get_mut(ei)) {
                e.points.extend(pts.into_iter().map(|(b, v)| (b + by, v)));
                e.points.sort_by(|x, y| x.0.total_cmp(&y.0));
            }
        }
    }

    /// Moves everything at or after `at` by `by` beats: arrangement clips,
    /// automation, sections (but `keep`) and a loop that lies after it.
    fn shift_from(&mut self, at: f64, by: f64, keep: Option<u64>) {
        for t in &mut self.tracks {
            for ac in &mut t.arrangement {
                if ac.start >= at - 1e-9 {
                    ac.start += by;
                }
            }
            for e in &mut t.automation {
                for p in &mut e.points {
                    if p.0 >= at - 1e-9 {
                        p.0 += by;
                    }
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
            self.split_automation_at(t);
        }
        let lifted = self.lift(a, b);
        let lines = self.lift_automation(a, b);
        self.shift_from(b, -len, Some(id));
        let to = if to > b { to - len } else { to };
        self.shift_from(to, len, Some(id));
        for (ti, mut ac) in lifted {
            ac.start = ac.start - a + to;
            self.tracks[ti].arrangement.push(ac);
        }
        self.drop_automation(lines, to - a);
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
            self.split_automation_at(t);
        }
        let lines: Vec<(usize, usize, Vec<(f64, f32)>)> = self
            .tracks
            .iter()
            .enumerate()
            .flat_map(|(ti, t)| t.automation.iter().enumerate().map(move |(ei, e)| (ti, ei, e.points.iter().copied().filter(|p| p.0 >= a - 1e-9 && p.0 < b - 1e-9).collect::<Vec<_>>())))
            .filter(|(_, _, pts)| !pts.is_empty())
            .collect();
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
        self.drop_automation(lines, to - a);
        let new = self.alloc_id();
        self.sections.push(SongSection { id: new, name: s.name.clone(), start: to, length: len, color: s.color });
        self.sections.sort_by(|a, b| a.start.total_cmp(&b.start));
        Some(new)
    }

    /// Removes section `id`; with its content, the time it took closes too.
    pub fn delete_section(&mut self, id: u64, with_content: bool) {
        let Some(s) = self.sections.iter().find(|s| s.id == id).cloned() else { return };
        if with_content {
            for t in [s.start, s.end()] {
                self.split_arrangement_at(t);
                self.split_automation_at(t);
            }
            self.lift(s.start, s.end());
            self.lift_automation(s.start, s.end());
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
        Device { id: self.alloc_id(), kind, enabled: true, params: params::defaults(kind.specs()), plugin: None, data, children: Vec::new(), preset: String::new(), sample: None, looper: Default::default() }
    }

    /// A device for an effect plug-in; `plugin.instance` is set here.
    pub fn make_plugin_device(&mut self, mut plugin: PluginInstrument) -> Device {
        plugin.instance = self.alloc_id();
        Device { id: self.alloc_id(), kind: EffectKind::Plugin, enabled: true, params: Vec::new(), plugin: Some(plugin), data: Vec::new(), children: Vec::new(), preset: String::new(), sample: None, looper: Default::default() }
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
            tempo: None,
            midi_to: None,
            place: None,
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
            lane_h: 0.0,
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

    /// A new MIDI track called `name` at `at` (the tracks from there move
    /// one on). Returns where it is.
    pub fn insert_midi_track(&mut self, at: usize, name: impl Into<String>, instrument: Option<Instrument>) -> usize {
        let t = self.new_track(TrackKind::Midi, name.into(), instrument);
        let at = at.min(self.tracks.len());
        self.tracks.insert(at, t);
        at
    }

    pub fn add_audio_track(&mut self) -> usize {
        let n = self.tracks.len() + 1;
        let t = self.new_track(TrackKind::Audio, format!("{n} Audio"), None);
        self.tracks.push(t);
        self.tracks.len() - 1
    }

    /// A device in a clip's sound has an id of its own track: one that is
    /// another track's (a clip moved or copied to another track) is renewed,
    /// the same way in all that track's clips. Returns whether any changed.
    pub fn fix_clip_sound_ids(&mut self) -> bool {
        use std::collections::HashMap;
        // Whose each id is: the tracks' own devices first, then their clips', in order.
        let mut owner: HashMap<u64, usize> = HashMap::new();
        for (ti, t) in self.tracks.iter().enumerate() {
            for d in &t.devices {
                owner.insert(d.id, ti);
                for c in &d.children {
                    owner.insert(c.id, ti);
                }
            }
            for d in &t.midi_devices {
                owner.insert(d.id, ti);
            }
        }
        for devs in self.returns.iter().map(|r| &r.devices).chain(std::iter::once(&self.master.devices)) {
            for d in devs {
                owner.insert(d.id, usize::MAX);
            }
        }
        let mut next = self.next_id;
        let mut changed = false;
        for (ti, t) in self.tracks.iter_mut().enumerate() {
            let mut renewed: HashMap<u64, u64> = HashMap::new();
            let sounds = t.slots.iter_mut().flatten().chain(t.arrangement.iter_mut().map(|a| &mut a.clip)).filter_map(|c| c.sound.as_deref_mut());
            for s in sounds {
                let ids = s.devices.iter_mut().flat_map(|d| std::iter::once(&mut d.id).chain(d.children.iter_mut().map(|c| &mut c.id))).chain(s.midi_devices.iter_mut().map(|d| &mut d.id));
                for id in ids {
                    match owner.get(id) {
                        Some(&o) if o != ti => {
                            *id = *renewed.entry(*id).or_insert_with(|| {
                                next += 1;
                                next - 1
                            });
                            owner.insert(*id, ti);
                            changed = true;
                        }
                        Some(_) => {}
                        None => {
                            owner.insert(*id, ti);
                        }
                    }
                }
            }
        }
        self.next_id = next;
        changed
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
        // A device gets one new id, in the track and in the sounds of its clips.
        let mut ids = std::collections::HashMap::new();
        let mut renew = |id: &mut u64, p: &mut Project| *id = *ids.entry(*id).or_insert_with(|| p.alloc_id());
        for d in &mut t.devices {
            renew(&mut d.id, self);
            for c in &mut d.children {
                renew(&mut c.id, self);
            }
        }
        for d in &mut t.midi_devices {
            renew(&mut d.id, self);
        }
        let sounds = t.slots.iter_mut().flatten().chain(t.arrangement.iter_mut().map(|a| &mut a.clip)).filter_map(|c| c.sound.as_deref_mut());
        for s in sounds {
            for d in &mut s.devices {
                renew(&mut d.id, self);
                for c in &mut d.children {
                    renew(&mut c.id, self);
                }
            }
            for d in &mut s.midi_devices {
                renew(&mut d.id, self);
            }
        }
        if let Some(Instrument::Plugin(p)) = &mut t.instrument {
            p.instance = self.alloc_id();
        }
        // A device of its own (it boots again).
        if let Some(Instrument::Hyper { engine, .. }) = &mut t.instrument {
            *engine = crate::hyper::HyperRef::fresh();
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
                self.tracks[ti].arrangement.push(ArrangementClip::new(id, cursor, len, clip));
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
        let fix_instrument = |inst: &mut Option<Instrument>, max_id: &mut u64| {
                // Names that changed.
                match &mut *inst {
                    Some(Instrument::Drums { kit, .. }) => *kit = drums::current_name(kit).to_string(),
                    Some(Instrument::Analog { preset, .. } | Instrument::Calculator { preset, .. } | Instrument::Micro { preset, .. }) => {
                        if let Some(n) = renamed_preset(preset) {
                            *preset = n.to_string();
                        }
                    }
                    _ => {}
                }
                match &mut *inst {
                    Some(Instrument::Synth { params, .. }) => params::fix_len(params, synth::SPECS),
                    Some(Instrument::Sampler { params, .. }) => params::fix_len(params, sampler::SPECS),
                    Some(Instrument::Sf2 { params, .. }) => params::fix_len(params, crate::dsp::sf2::SPECS),
                    Some(Instrument::Singer { params, .. }) => params::fix_len(params, crate::singer::SPECS),
                    Some(Instrument::Hyper { params, .. }) => params::fix_len(params, crate::hyper::SPECS),
                    Some(Instrument::StreetOrgan { params, .. }) => {
                        params::fix_len(params, crate::dsp::street_organ::SPECS);
                        crate::dsp::street_organ::warm();
                    }
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
                    Some(Instrument::Calculator { params, .. }) => params::fix_len(params, crate::dsp::calculator::SPECS),
                    Some(Instrument::Console { params, .. }) => params::fix_len(params, crate::dsp::console::SPECS),
                    Some(Instrument::Handheld { params, .. }) => params::fix_len(params, crate::dsp::handheld::SPECS),
                    Some(Instrument::Micro { params, .. }) => params::fix_len(params, crate::dsp::micro::SPECS),
                    Some(Instrument::Warp { params, .. }) => params::fix_len(params, crate::dsp::warp::SPECS),
                    Some(Instrument::Hybrid { params, .. }) => params::fix_len(params, crate::dsp::hybrid::SPECS),
                    Some(Instrument::ChipFm { params, .. }) => params::fix_len(params, crate::dsp::chipfm::SPECS),
                    Some(Instrument::V2 { params, .. }) => params::fix_len(params, crate::dsp::v2::SPECS),
                    Some(Instrument::Oidos { params, .. }) => params::fix_len(params, crate::dsp::oidos::SPECS),
                    Some(Instrument::Quartet { params, .. }) => params::fix_len(params, crate::dsp::quartet::SPECS),
                    Some(Instrument::Orchestra { params, .. }) => params::fix_len(params, crate::orchestra::SPECS),
                    Some(Instrument::Facet { params, .. }) => params::fix_len(params, crate::dsp::facet::SPECS),
                    Some(Instrument::Coast { params, .. }) => params::fix_len(params, crate::dsp::coast::SPECS),
                    Some(Instrument::Theremin { params, .. }) => params::fix_len(params, crate::dsp::theremin::SPECS),
                    Some(Instrument::Crimson { params, .. }) => {
                        params::fix_len(params, crate::dsp::crimson::SPECS);
                        crate::dsp::waves::warm();
                    }
                    Some(Instrument::Silicon { params, .. }) => {
                        params::fix_len(params, crate::dsp::silicon::SPECS);
                        let _ = crate::dsp::silicon::firmware();
                    }
                    Some(Instrument::Granulator { params, .. }) => {
                        params::fix_len(params, crate::dsp::granulator::SPECS);
                        crate::dsp::granulator::warm();
                    }
                    Some(Instrument::Keytone { params, .. }) => {
                        params::fix_len(params, crate::dsp::keytone::SPECS);
                        crate::dsp::keytone::warm();
                    }
                    Some(Instrument::Stretch { params, .. }) => {
                        params::fix_len(params, crate::dsp::stretch::SPECS);
                        crate::dsp::stretch::warm();
                    }
                    Some(Instrument::Princess { params, machine, .. }) => {
                        params::fix_len(params, crate::dsp::princess_machine::SPECS);
                        machine.fix();
                    }
                    Some(Instrument::Turntable { params, .. }) => params::fix_len(params, crate::dsp::turntable::SPECS),
                    Some(Instrument::Grit { params, pads, .. }) => {
                        params::fix_len(params, crate::dsp::grit::SPECS);
                        crate::dsp::grit::fix_pads(pads);
                        crate::dsp::grit::warm();
                    }
                    Some(Instrument::Flip { params, .. }) => params::fix_len(params, crate::dsp::princess_flip::SPECS),
                    // The toy keyboard's drums made now, not on the audio thread.
                    Some(Instrument::Drums { pads, .. }) if pads.iter().any(|p| p.voice.family() == drums::Family::Keytone) => crate::dsp::keytone::warm(),
                    // Shakers became an instrument; their kits are the Drum Rack's.
                    Some(Instrument::Drums { engine, .. }) if *engine == drums::DrumEngine::Shakers => *engine = drums::DrumEngine::Rack,
                    Some(Instrument::Plugin(p)) => *max_id = (*max_id).max(p.instance + 1),
                    Some(Instrument::Fm { voice, params, .. }) => {
                        params::fix_len(params, fm::SPECS);
                        let fixed = Dx7Voice::from_slice(voice).unwrap_or_else(dx7::init_voice);
                        *voice = fixed.0.to_vec();
                    }
                    _ => {}
                }
        };
        // The sounds kept in clips: the same, and without plug-ins (a clip can't switch them).
        let fix_sound = |snd: &mut Sound, max_id: &mut u64| {
            snd.devices.retain(|d| d.plugin.is_none());
            fix_devices(&mut snd.devices, max_id);
            for d in snd.midi_devices.iter_mut() {
                params::fix_len(&mut d.params, d.kind.specs());
                *max_id = (*max_id).max(d.id + 1);
            }
            if matches!(snd.instrument, Some(Instrument::Plugin(_))) {
                snd.instrument = None;
            }
            fix_instrument(&mut snd.instrument, max_id);
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
            fix_instrument(&mut t.instrument, &mut max_id);
            for c in t.slots.iter_mut().flatten() {
                c.length = c.length.max(0.0625);
                max_id = max_id.max(c.id + 1);
                if let Some(snd) = c.sound.as_deref_mut() {
                    fix_sound(snd, &mut max_id);
                }
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
                if let Some(snd) = a.clip.sound.as_deref_mut() {
                    fix_sound(snd, &mut max_id);
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
        self.video.sanitize();
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
        p.prepare_audio();
        Ok((p, missing))
    }

    /// Finds what the audio clips' warping needs before the audio thread
    /// plays them: their samples' transients, and the reversed audio of
    /// clips that play backwards. Done once per sample; cheap after.
    /// Every Looper in the set: on the tracks, the returns and the master,
    /// and in the sounds clips keep.
    pub fn loopers_mut(&mut self) -> Vec<&mut Device> {
        let mut out = Vec::new();
        for t in &mut self.tracks {
            out.extend(t.devices.iter_mut().filter(|d| d.kind == EffectKind::Looper));
            for c in t.slots.iter_mut().flatten().chain(t.arrangement.iter_mut().map(|a| &mut a.clip)) {
                if let Some(s) = &mut c.sound {
                    out.extend(s.devices.iter_mut().filter(|d| d.kind == EffectKind::Looper));
                }
            }
        }
        for r in &mut self.returns {
            out.extend(r.devices.iter_mut().filter(|d| d.kind == EffectKind::Looper));
        }
        out.extend(self.master.devices.iter_mut().filter(|d| d.kind == EffectKind::Looper));
        out
    }

    pub fn prepare_audio(&self) {
        for t in &self.tracks {
            for c in t.slots.iter().flatten().chain(t.arrangement.iter().map(|a| &a.clip)) {
                if let ClipContent::Audio(a) = &c.content {
                    if a.warp && a.mode != crate::warp::WarpMode::RePitch {
                        a.sample.analysis();
                    }
                    if a.reverse {
                        a.sample.reversed();
                    }
                }
            }
        }
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

    fn with_clip_sound() -> (Project, usize, u64) {
        let mut p = crate::demo::demo_project();
        let ti = 1;
        let mut snd = p.tracks[ti].sound_now().expect("no plug-ins");
        let rev = p.make_device(EffectKind::Reverb);
        snd.devices.push(rev);
        let clip = p.tracks[ti].slots[1].as_mut().expect("a second clip");
        clip.sound = Some(Box::new(snd));
        let id = clip.id;
        (p, ti, id)
    }

    #[test]
    fn a_clip_splits_in_two_and_keeps_its_outer_fades() {
        let mut p = Project::new_default();
        let id = p.alloc_id();
        let cid = p.alloc_id();
        let mut ac = ArrangementClip::new(id, 4.0, 8.0, Clip::new_midi(cid, "c", [1, 2, 3], 4.0));
        ac.offset = 1.0;
        ac.fade_in = 1.0;
        ac.fade_out = 2.0;
        p.tracks[0].arrangement.push(ac);
        assert!(p.split_clip(0, id, 4.0).is_none(), "not at its edge");
        let right = p.split_clip(0, id, 7.0).expect("splits");
        let a = p.tracks[0].arrangement.iter().find(|a| a.id == id).unwrap().clone();
        let b = p.tracks[0].arrangement.iter().find(|a| a.id == right).unwrap().clone();
        assert_eq!((a.start, a.length, a.fade_in, a.fade_out), (4.0, 3.0, 1.0, 0.0));
        assert_eq!((b.start, b.length, b.fade_in, b.fade_out), (7.0, 5.0, 0.0, 2.0));
        // The right part goes on where the left one stopped, in the looping content.
        assert!((b.offset - 0.0).abs() < 1e-9, "1 + 3 beats into a 4-beat clip is its start again");
        assert_ne!(a.clip.id, b.clip.id);
    }

    #[test]
    fn fades_rise_from_silence_and_fall_back_to_it() {
        let mut ac = ArrangementClip::new(1, 0.0, 8.0, Clip::new_midi(2, "c", [1, 2, 3], 8.0));
        ac.fade_in = 2.0;
        ac.fade_out = 2.0;
        assert!(ac.fade_gain(0.0) < 1e-6);
        assert!((ac.fade_gain(4.0) - 1.0).abs() < 1e-6);
        assert!(ac.fade_gain(1.0) > 0.5 && ac.fade_gain(1.0) < 0.9, "equal power: about 0.71 halfway");
        assert!(ac.fade_gain(8.0) < 1e-6);
    }

    #[test]
    fn moving_a_section_takes_its_automation_along() {
        let mut p = crate::demo::demo_project();
        p.scenes_to_arrangement();
        let bpb = p.beats_per_bar_f();
        let a = p.add_section(0.0, 4.0 * bpb);
        let b = p.add_section(4.0 * bpb, 4.0 * bpb);
        let target = crate::modulation::ModTarget::Volume(p.tracks[0].id);
        p.tracks[0].automation_mut(target).points = vec![(1.0, 0.2), (2.0, 0.9), (4.0 * bpb + 1.0, 0.5)];
        // The first section goes after the second: its points come along, the second's move up.
        p.move_section(a, 8.0 * bpb);
        let pts = &p.tracks[0].automation_of(target).unwrap().points;
        assert!(pts.iter().any(|&(x, v)| (x - (4.0 * bpb + 1.0)).abs() < 1e-6 && (v - 0.2).abs() < 1e-6), "{pts:?}");
        assert!(pts.iter().any(|&(x, v)| (x - 1.0).abs() < 1e-6 && (v - 0.5).abs() < 1e-6), "{pts:?}");
        let _ = b;
    }

    #[test]
    fn a_clip_sound_is_saved_and_loaded() {
        let (p, ti, clip) = with_clip_sound();
        let json = serde_json::to_string(&p).expect("saves");
        let mut back: Project = serde_json::from_str(&json).expect("loads");
        back.sanitize();
        assert_eq!(back.tracks[ti].clip_sound(clip), p.tracks[ti].clip_sound(clip));
        // Clips without one save as they did.
        let plain = serde_json::to_string(&p.tracks[0].slots[0]).expect("saves");
        assert!(!plain.contains("sound"));
        // A track takes it, and keeps what changes on it in the clip.
        let mut t = back.tracks[ti].clone();
        assert!(t.take_clip_sound(clip));
        assert!(!t.take_clip_sound(clip), "already its own");
        if let Some(d) = t.devices.last_mut() {
            d.params[0] = 0.123;
        }
        assert!(t.update_clip_sound(clip));
        assert_eq!(t.clip_sound(clip).and_then(|s| s.devices.last()).map(|d| d.params[0]), Some(0.123));
    }

    #[test]
    fn a_duplicated_track_and_a_moved_clip_get_their_own_ids() {
        let (mut p, ti, clip) = with_clip_sound();
        let at = p.duplicate_track(ti).expect("duplicates");
        let own: Vec<u64> = p.tracks[at].devices.iter().map(|d| d.id).collect();
        let copy = p.tracks[at].slots[1].as_ref().and_then(|c| c.sound.as_deref()).expect("its clip keeps the sound").clone();
        // The devices the track has are the same ones in its clip's sound (new ids, the same in both).
        for id in &own {
            assert!(copy.devices.iter().any(|d| d.id == *id));
            assert!(!p.tracks[ti].devices.iter().any(|d| d.id == *id));
        }
        assert!(!p.fix_clip_sound_ids(), "nothing to fix");
        // A clip moved to another track brings its sound with ids of that track.
        let moved = p.tracks[ti].slots[1].clone();
        p.tracks[0].slots[3] = moved;
        assert!(p.fix_clip_sound_ids());
        let theirs: Vec<u64> = p.tracks[0].slots[3].as_ref().and_then(|c| c.sound.as_deref()).map(|s| s.devices.iter().map(|d| d.id).collect()).unwrap_or_default();
        let mine: Vec<u64> = p.tracks[ti].clip_sound(clip).map(|s| s.devices.iter().map(|d| d.id).collect()).unwrap_or_default();
        assert!(!theirs.is_empty() && theirs.iter().all(|id| !mine.contains(id)));
        assert!(!p.fix_clip_sound_ids(), "fixed once");
    }

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
    fn the_video_is_saved_with_the_set_and_older_sets_load_without_it() {
        let mut p = crate::demo::demo_project();
        // Nothing loaded: nothing written.
        assert!(!serde_json::to_string(&p).unwrap().contains("\"video\""));
        let dir = std::env::temp_dir().join(format!("krabton-test-video-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("video.krab");
        p.video.clips.push(crate::video::VideoClip::new(dir.join("sea.mp4")));
        p.video.clips[0].play.mode = crate::video::PlayMode::Slices;
        p.video.clips[0].play.slice_track = Some(p.tracks[0].id);
        p.video.links[1].from = Some(p.tracks[1].id);
        p.save(&path).unwrap();
        let (q, _) = Project::load(&path).unwrap();
        assert_eq!(q.video, p.video);
        // A set from before the Video: the JSON without it.
        let mut old: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        old.as_object_mut().unwrap().remove("video");
        std::fs::write(&path, old.to_string()).unwrap();
        let (q, _) = Project::load(&path).unwrap();
        assert!(q.video.is_empty());
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
            p.tracks[t].arrangement.push(ArrangementClip::new(id, start, length, clip));
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
        p.master.devices.push(Device { id: 999, kind: EffectKind::Plugin, enabled: true, params: Vec::new(), plugin: None, data: Vec::new(), children: Vec::new() , preset: String::new(), sample: None, looper: Default::default() });
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

    #[test]
    fn renamed_instruments_and_presets_still_load() {
        let mut p = Project::blank();
        let a = p.add_midi_track(Some(Instrument::calculator("Piano")));
        let b = p.add_midi_track(Some(Instrument::analog("Moog Bass")));
        let c = p.add_midi_track(Some(Instrument::drums("808 Classic")));
        let d = p.add_midi_track(Some(Instrument::micro("Clean Pad")));
        // As 0.11.0 (the synths) and 0.11.1 (Analog's presets, the kits) wrote them.
        let json = serde_json::to_string(&p)
            .unwrap()
            .replace("\"Calculator\"", "\"VlTone\"")
            .replace("\"Micro\"", "\"Sid\"")
            .replace("\"Moog Bass\"", "\"Ladder Bass\"")
            .replace("\"808 Classic\"", "\"Boom Classic\"")
            .replace("\"Clean Pad\"", "\"8580 Pad\"");
        assert!(json.contains("VlTone") && json.contains("Ladder Bass") && json.contains("Boom Classic"));
        let mut back: Project = serde_json::from_str(&json).unwrap();
        back.sanitize();
        assert!(matches!(&back.tracks[a].instrument, Some(Instrument::Calculator { .. })));
        assert!(matches!(&back.tracks[b].instrument, Some(Instrument::Analog { preset, .. }) if preset == "Moog Bass"));
        assert!(matches!(&back.tracks[c].instrument, Some(Instrument::Drums { kit, engine, .. }) if kit == "808 Classic" && *engine == drums::DrumEngine::M808));
        assert!(matches!(&back.tracks[d].instrument, Some(Instrument::Micro { preset, .. }) if preset == "Clean Pad"));
    }

}
