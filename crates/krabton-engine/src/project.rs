//! The project data model. The UI owns a `Project` and edits it freely; the
//! audio engine receives immutable snapshots (`Arc<Project>`).

use crate::dsp::drums::{self, DrumVoice};
use crate::dsp::effects::EffectKind;
use crate::dsp::dx7::{self, Dx7Voice};
use crate::dsp::{analog, braids_synth, elements_synth, fm, macro_osc, sampler, synth};
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
}

impl Note {
    pub fn new(pitch: u8, start: f64, length: f64, velocity: f32) -> Self {
        Self { pitch, start, length, velocity }
    }

    pub fn end(&self) -> f64 {
        self.start + self.length
    }
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
}

impl Clip {
    pub fn new_midi(id: u64, name: impl Into<String>, color: Rgb, length: f64) -> Self {
        Self { id, name: name.into(), color, length, content: ClipContent::Midi { notes: Vec::new() } }
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
        Self { id, name: sample.name.clone(), color, length, content: ClipContent::Audio(audio) }
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
}

impl DrumPad {
    pub fn new(voice: DrumVoice) -> Self {
        Self {
            name: voice.name().to_string(),
            voice,
            note: voice.gm_note(),
            level_db: match voice {
                DrumVoice::Kick => 0.0,
                DrumVoice::ClosedHat | DrumVoice::OpenHat => -6.0,
                DrumVoice::Cowbell | DrumVoice::Rim => -8.0,
                _ => -3.0,
            },
            tune: 0.0,
            decay: 1.0,
            tone: 0.5,
            pan: 0.0,
            sample: None,
        }
    }
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
    Drums { kit: String, pads: Vec<DrumPad> },
    Sampler { params: Vec<f32>, sample: Option<SampleRef> },
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
}

impl Instrument {
    pub fn synth(preset: &str) -> Self {
        Instrument::Synth { preset: preset.to_string(), params: synth::preset(preset) }
    }

    pub fn drums(kit: &str) -> Self {
        Instrument::Drums { kit: kit.to_string(), pads: drums::kit(kit) }
    }

    pub fn sampler(sample: Option<SampleRef>) -> Self {
        Instrument::Sampler { params: params::defaults(sampler::SPECS), sample }
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

    pub fn kind_name(&self) -> &'static str {
        match self {
            Instrument::Synth { .. } => "Synth",
            Instrument::Drums { .. } => "Drums",
            Instrument::Sampler { .. } => "Sampler",
            Instrument::Plugin(_) => "CLAP Plugin",
            Instrument::Fm { .. } => "FM",
            Instrument::Macro { .. } => "Plaits",
            Instrument::Braids { .. } => "Braids",
            Instrument::Elements { .. } => "Elements",
            Instrument::Analog { .. } => "Analog",
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Instrument::Synth { preset, .. } => preset.clone(),
            Instrument::Drums { kit, .. } => kit.clone(),
            Instrument::Sampler { sample, .. } => sample.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "Empty".into()),
            Instrument::Plugin(p) => p.name.clone(),
            Instrument::Fm { voice, .. } => Dx7Voice::from_slice(voice).map(|v| v.name()).unwrap_or_else(|| "FM".into()),
            Instrument::Macro { preset, .. } | Instrument::Braids { preset, .. } | Instrument::Elements { preset, .. } | Instrument::Analog { preset, .. } => preset.clone(),
        }
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
    pub next_id: u64,
}

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
            next_id: 1,
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
    pub fn new_default() -> Self {
        let mut p = Self::blank();
        p.add_midi_track(Some(Instrument::drums("909 Krab")));
        p.add_midi_track(Some(Instrument::synth("Fat Bass")));
        p.add_audio_track();
        p
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
        Device { id: self.alloc_id(), kind, enabled: true, params: params::defaults(kind.specs()) }
    }

    pub fn make_midi_device(&mut self, kind: crate::midi_fx::MidiFxKind) -> MidiDevice {
        MidiDevice { id: self.alloc_id(), kind, enabled: true, params: params::defaults(kind.specs()), data: kind.default_data() }
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
        }
    }

    pub fn add_midi_track(&mut self, instrument: Option<Instrument>) -> usize {
        let n = self.tracks.len() + 1;
        let name = match &instrument {
            Some(Instrument::Drums { .. }) => "Drums".to_string(),
            Some(i) => i.display_name(),
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
            for d in devs.iter_mut() {
                params::fix_len(&mut d.params, d.kind.specs());
                *max_id = (*max_id).max(d.id + 1);
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
            }
            for a in &mut t.arrangement {
                a.length = a.length.max(0.0625);
                a.clip.length = a.clip.length.max(0.0625);
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
mod tests {
    use super::*;

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
}
