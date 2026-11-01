//! Slice to New MIDI Track, as in Live: an audio clip cut into slices (at
//! its hits, or on a grid of the beat), each slice on a pad of a new Drum
//! Rack from C1 up, and a MIDI clip that plays them where they were, so the
//! new track sounds like the loop and its slices can be played, tuned and
//! moved about for a new beat.
//!
//! The slices are samples in memory, cut from the clip's sample with short
//! fades at both ends. A set keeps each one as its file and the part of it
//! (see `SampleRegion`), and cuts it again when it opens; no files are
//! written.
//!
//! The sound of a film can bring its picture along (Slice to Drum Rack):
//! the film goes into the Video's bank with a cue at every cut, the same
//! cuts as the pads', and the new track's notes play the picture with the
//! sound (see `video::Cues`).

use crate::analysis::{self, Flux};
use crate::dsp::drums::{DrumEngine, DrumVoice, MAX_PADS, PAD_GAIN};
use crate::project::{ArrangementClip, AudioClip, Clip, ClipContent, DrumPad, Instrument, Note, Project};
use crate::sample::{SampleData, SampleRef, SampleRegion};
use crate::video::{self, Cues};

/// The note of the first slice (C1, as in Live); the next ones go up.
pub const FIRST_NOTE: u8 = 36;
/// A slice fades in over this long (seconds); a slice at a hit starts this
/// much before it, so the hit itself is whole.
const FADE_IN: f64 = 0.001;
/// And fades out over this long at its end.
const FADE_OUT: f64 = 0.004;
/// Hits nearer than this to the start or end of what is sliced make no
/// slice of their own (seconds).
const EDGE: f64 = 0.01;

/// Where the slices go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliceBy {
    /// At the hits.
    Transients,
    Bar,
    Half,
    Quarter,
    Eighth,
    Sixteenth,
    ThirtySecond,
}

impl SliceBy {
    pub const ALL: [SliceBy; 7] = [SliceBy::Transients, SliceBy::Bar, SliceBy::Half, SliceBy::Quarter, SliceBy::Eighth, SliceBy::Sixteenth, SliceBy::ThirtySecond];

    pub fn name(self) -> &'static str {
        match self {
            SliceBy::Transients => "Transients",
            SliceBy::Bar => "1 Bar",
            SliceBy::Half => "1/2",
            SliceBy::Quarter => "1/4",
            SliceBy::Eighth => "1/8",
            SliceBy::Sixteenth => "1/16",
            SliceBy::ThirtySecond => "1/32",
        }
    }

    /// Its grid in beats (None: at the hits).
    pub fn beats(self, beats_per_bar: f64) -> Option<f64> {
        match self {
            SliceBy::Transients => None,
            SliceBy::Bar => Some(beats_per_bar.max(1.0)),
            SliceBy::Half => Some(2.0),
            SliceBy::Quarter => Some(1.0),
            SliceBy::Eighth => Some(0.5),
            SliceBy::Sixteenth => Some(0.25),
            SliceBy::ThirtySecond => Some(0.125),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliceOptions {
    pub by: SliceBy,
    /// How soft a hit still counts (0..1), slicing at the hits.
    pub sensitivity: f32,
    /// At most this many slices (a Drum Rack has `MAX_PADS` pads): the
    /// weakest cuts go first, their slices joining the ones before them.
    pub max_slices: usize,
    /// The pads play while their notes are held, and stop where the notes
    /// end; off, they are one-shots: each slice plays to its end.
    pub gate: bool,
    /// A film's sound: its picture too, cut at the same places in the
    /// Video, played by the new track's notes.
    pub picture: bool,
}

impl Default for SliceOptions {
    fn default() -> Self {
        Self { by: SliceBy::Transients, sensitivity: 0.5, max_slices: MAX_PADS, gate: false, picture: false }
    }
}

/// The part of a clip's sample that is sliced (frames): its loop when it
/// loops (`loop_len` beats from the loop's start), else from its Start to
/// its End marker, through its warp markers. A reversed clip's part is the
/// same stretch of the sample, which the slices play forwards.
pub fn clip_region(a: &AudioClip, loop_len: f64) -> (usize, usize) {
    let frames = a.sample.frames();
    let (b0, b1) = clip_beats(a, loop_len);
    let sr = a.sample.sample_rate as f64;
    let at = |b: f64| ((a.secs_at(b) * sr).round().max(0.0) as usize).min(frames);
    let (f0, f1) = (at(b0), at(b1).max(at(b0)));
    if a.reverse {
        (frames - f1, frames - f0)
    } else {
        (f0, f1)
    }
}

/// The clip beats that are sliced (see [`clip_region`]).
fn clip_beats(a: &AudioClip, loop_len: f64) -> (f64, f64) {
    if a.looping {
        (a.loop_start, a.loop_start + loop_len.max(0.0))
    } else {
        (a.start, a.end_beat().max(a.start))
    }
}

/// Frames of a clip's sample per beat of the clip, over the part sliced
/// (between warp markers the tempo may change; this is its average).
fn frames_per_beat(a: &AudioClip, loop_len: f64) -> f64 {
    let (b0, b1) = clip_beats(a, loop_len);
    if b1 - b0 > 1e-6 {
        (a.secs_at(b1) - a.secs_at(b0)) * a.sample.sample_rate as f64 / (b1 - b0)
    } else {
        a.sample.sample_rate as f64 * 60.0 / a.bpm.max(1.0)
    }
}

/// Where the slices of a clip are.
#[derive(Clone, Debug, PartialEq)]
pub struct SlicePlan {
    /// The part of the sample that is sliced (frames).
    pub from: usize,
    pub to: usize,
    /// Where each slice starts (frames of the sample), in order; the first
    /// is `from`, each ends where the next starts.
    pub starts: Vec<usize>,
    /// How many more slices there would have been without `max_slices`.
    pub merged: usize,
    /// Frames of the sample per beat of the clip.
    pub frames_per_beat: f64,
}

impl SlicePlan {
    /// The slices of `a`, a clip `loop_len` beats long, in a song of
    /// `beats_per_bar`; `flux` is the sample's (see `analysis`).
    pub fn new(a: &AudioClip, loop_len: f64, opts: &SliceOptions, beats_per_bar: f64, flux: &Flux) -> Self {
        let (from, to) = clip_region(a, loop_len);
        let fpb = frames_per_beat(a, loop_len);
        let sr = a.sample.sample_rate as f64;
        let edge = (EDGE * sr) as usize;
        let pre = (FADE_IN * sr) as usize;
        // The cuts there could be, each with how strong a hit it is on.
        let mut cuts: Vec<(usize, f32)> = match opts.by.beats(beats_per_bar) {
            None => flux.hits(&a.sample, opts.sensitivity).into_iter().map(|h| (h.frame.saturating_sub(pre), h.strength)).collect(),
            Some(step) => {
                let step = step * fpb;
                let line = |f: usize| 0.5 * flux.strength_near(f) + 0.5 * analysis::level_jump(&a.sample, f);
                (1..).map(|k| from + (k as f64 * step).round() as usize).take_while(|&f| f < to).map(|f| (f, line(f))).collect()
            }
        };
        cuts.retain(|&(f, _)| f > from + edge && f + edge < to);
        let room = opts.max_slices.clamp(1, MAX_PADS) - 1;
        let mut merged = 0;
        if cuts.len() > room {
            merged = cuts.len() - room;
            // The strongest stay (the earlier of two as strong).
            cuts.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            cuts.truncate(room);
            cuts.sort_by_key(|c| c.0);
        }
        let mut starts = vec![from];
        starts.extend(cuts.iter().map(|c| c.0));
        Self { from, to, starts, merged, frames_per_beat: fpb }
    }

    pub fn len(&self) -> usize {
        self.starts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.starts.is_empty()
    }

    /// Where slice `i` ends.
    pub fn end_of(&self, i: usize) -> usize {
        self.starts.get(i + 1).copied().unwrap_or(self.to)
    }

    /// The pads, one per slice from C1 up, each playing its part of
    /// `sample` as loud as the loop.
    pub fn pads(&self, sample: &SampleRef, opts: &SliceOptions) -> Vec<DrumPad> {
        let sr = sample.sample_rate as f64;
        let (fade_in, fade_out) = ((FADE_IN * sr).round() as u32, (FADE_OUT * sr).round() as u32);
        (0..self.len())
            .map(|i| {
                let region = SampleRegion { start: self.starts[i] as u64, end: self.end_of(i) as u64, fade_in, fade_out };
                let mut pad = DrumPad::new(DrumVoice::Kick);
                pad.name = format!("Slice {}", i + 1);
                pad.note = FIRST_NOTE + i as u8;
                pad.level_db = 0.0;
                pad.trim_db = -crate::dsp::gain_to_db(PAD_GAIN);
                pad.gate = opts.gate;
                pad.sample = Some(SampleRef::new(sample.region(format!("{} {}", sample.name, i + 1), region)));
                pad
            })
            .collect()
    }

    /// The cuts as cues of the film the sample's sound came from (seconds of
    /// the file, as its pads' regions count them), played by `track`'s notes.
    pub fn cues(&self, sample: &SampleData, gate: bool, track: Option<u64>) -> Cues {
        let base = sample.region.map_or(0, |r| r.start) as f64;
        let sr = sample.sample_rate.max(1) as f64;
        let secs = |f: usize| (base + f as f64) / sr;
        Cues { times: self.starts.iter().map(|&f| secs(f)).collect(), end: secs(self.to), track, gate }
    }

    /// The notes that play the slices where they were (beats of the clip),
    /// each as long as its slice.
    pub fn notes(&self) -> Vec<Note> {
        let fpb = self.frames_per_beat.max(1e-9);
        (0..self.len())
            .map(|i| {
                let start = (self.starts[i] - self.from) as f64 / fpb;
                let length = (self.end_of(i) - self.starts[i]) as f64 / fpb;
                Note::new(FIRST_NOTE + i as u8, start, length, 1.0)
            })
            .collect()
    }
}

/// Which clip of a track is sliced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliceClip {
    /// The clip in this scene's slot.
    Slot(usize),
    /// The arrangement clip with this id.
    Arrangement(u64),
}

/// Where the new MIDI clip goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SlicePlace {
    Slot(usize),
    /// On the arrangement: from `start`, `length` beats, from `offset` in.
    Arrangement { start: f64, length: f64, offset: f64 },
}

/// What slicing made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sliced {
    /// The new track.
    pub track: usize,
    pub slices: usize,
    /// Slices left out over the most (see `SlicePlan::merged`).
    pub merged: usize,
    /// The film's place in the Video's bank, cued at the cuts (with
    /// `SliceOptions::picture`, when the sound is a film's and the bank had room).
    pub picture: Option<usize>,
}

impl Project {
    /// Slice to New MIDI Track: the audio clip of `track` at `clip` cut as
    /// `opts` says onto a new MIDI track right after it, with a Drum Rack of
    /// the slices and a MIDI clip in the same scene (or the same place on
    /// the arrangement) that plays them. `flux` is the sample's, when it is
    /// known already. The audio track stays as it is.
    pub fn slice_to_midi_track(&mut self, track: usize, clip: SliceClip, opts: &SliceOptions, flux: Option<&Flux>) -> Result<Sliced, String> {
        let t = self.tracks.get(track).ok_or("There is no such track")?;
        let (source, place) = match clip {
            SliceClip::Slot(s) => (t.slots.get(s).and_then(|c| c.clone()), SlicePlace::Slot(s)),
            SliceClip::Arrangement(id) => match t.arrangement.iter().find(|a| a.id == id) {
                Some(ac) => (Some(ac.clip.clone()), SlicePlace::Arrangement { start: ac.start, length: ac.length, offset: ac.offset }),
                None => (None, SlicePlace::Slot(0)),
            },
        };
        let source = source.ok_or("There is no clip there")?;
        self.add_sliced_track(track + 1, &source, place, opts, flux)
    }

    /// A sample (from the browser, say) sliced onto a new MIDI track after
    /// the others, as a clip of it would be, its MIDI clip at `place`.
    pub fn slice_sample_to_midi_track(&mut self, sample: SampleRef, place: SlicePlace, opts: &SliceOptions, flux: Option<&Flux>) -> Result<Sliced, String> {
        let id = self.alloc_id();
        let color = crate::project::PALETTE[(id as usize * 7) % crate::project::PALETTE.len()];
        let clip = Clip::new_audio(id, sample, color, self.bpm);
        let place = match place {
            SlicePlace::Arrangement { start, .. } => SlicePlace::Arrangement { start, length: clip.length, offset: 0.0 },
            p => p,
        };
        self.add_sliced_track(self.tracks.len(), &clip, place, opts, flux)
    }

    fn add_sliced_track(&mut self, at: usize, source: &Clip, place: SlicePlace, opts: &SliceOptions, flux: Option<&Flux>) -> Result<Sliced, String> {
        let ClipContent::Audio(a) = &source.content else {
            return Err("Only audio clips can be sliced".into());
        };
        let own;
        let flux = match flux {
            Some(f) => f,
            None => {
                own = analysis::spectral_flux(&a.sample);
                &own
            }
        };
        let plan = SlicePlan::new(a, source.length, opts, self.beats_per_bar_f(), flux);
        if plan.to <= plan.from {
            return Err("The clip has no sound to slice".into());
        }
        let name = format!("{} Slices", source.name);
        let inst = Instrument::Drums { kit: source.name.clone(), pads: plan.pads(&a.sample, opts), engine: DrumEngine::Rack };
        let t = self.insert_midi_track(at, name, Some(inst));
        self.tracks[t].color = source.color;
        let mut clip = Clip::new_midi(self.alloc_id(), source.name.clone(), source.color, source.length);
        clip.content = ClipContent::Midi { notes: plan.notes() };
        match place {
            SlicePlace::Slot(s) => {
                while s >= self.scenes.len() {
                    let n = self.scenes.len();
                    self.add_scene(n);
                }
                self.tracks[t].slots[s] = Some(clip);
            }
            SlicePlace::Arrangement { start, length, offset } => {
                let id = self.alloc_id();
                self.tracks[t].arrangement.push(ArrangementClip { offset, ..ArrangementClip::new(id, start, length, clip) });
            }
        }
        // The picture: the film cut where its sound was, the same cuts.
        let film = a.sample.path.as_deref().filter(|p| opts.picture && video::is_film(p));
        let picture = film.and_then(|path| self.video.cue_film(path, plan.cues(&a.sample, opts.gate, Some(self.tracks[t].id))));
        Ok(Sliced { track: t, slices: plan.len(), merged: plan.merged, picture })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::tests::{drum_loop, BEAT};
    use crate::project::TrackKind;
    use crate::render::{render, RenderSource};
    use crate::sample::SampleData;

    /// A two-bar drum loop as a WAV file, and its hits.
    fn loop_file(name: &str, sr: u32, bass: bool) -> (std::path::PathBuf, Vec<usize>) {
        let (s, hits) = drum_loop(sr, BEAT, bass);
        let path = std::env::temp_dir().join(format!("krabton_slice_{}_{name}.wav", std::process::id()));
        let inter: Vec<f32> = s.left.iter().zip(&s.right).flat_map(|(l, r)| [*l * 0.7, *r * 0.7]).collect();
        let spec = hound::WavSpec { channels: 2, sample_rate: sr, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for v in inter {
            w.write_sample(v).unwrap();
        }
        w.finalize().unwrap();
        (path, hits)
    }

    /// A project at `bpm` with an audio track holding the loop in scene 1.
    fn project_with(sample: SampleRef, bpm: f64) -> Project {
        let mut p = Project::blank();
        p.bpm = bpm;
        let t = p.add_audio_track();
        let id = p.alloc_id();
        let mut c = Clip::new_audio(id, sample, [200, 100, 50], 120.0);
        c.name = "Break".into();
        if let ClipContent::Audio(a) = &mut c.content {
            a.bpm = 120.0;
        }
        c.length = 8.0;
        p.tracks[t].slots[0] = Some(c);
        p
    }

    fn opts(by: SliceBy, max: usize) -> SliceOptions {
        SliceOptions { by, max_slices: max, ..Default::default() }
    }

    #[test]
    fn a_loop_sliced_at_its_hits_gets_a_pad_and_a_note_per_hit() {
        let sr = 48_000;
        let (s, hits) = drum_loop(sr, BEAT, true);
        let mut p = project_with(SampleRef::new(s), 120.0);
        // A MIDI track after it stays after the new one.
        p.add_midi_track(None);
        let done = p.slice_to_midi_track(0, SliceClip::Slot(0), &opts(SliceBy::Transients, 16), None).unwrap();
        assert_eq!(done.track, 1);
        assert_eq!(p.tracks.len(), 3);
        let t = &p.tracks[1];
        assert_eq!((t.kind, t.name.as_str()), (TrackKind::Midi, "Break Slices"));
        let Some(Instrument::Drums { pads, engine, .. }) = &t.instrument else { panic!("a Drum Rack") };
        assert_eq!(*engine, DrumEngine::Rack);
        // Eighteen hits, sixteen pads: the two weakest cuts (the soft hats) go.
        assert_eq!((done.slices, done.merged), (16, hits.len() - 16));
        assert_eq!(pads.len(), 16);
        assert_eq!((pads[0].name.as_str(), pads[0].note, pads[15].note), ("Slice 1", 36, 51));
        let clip = t.slots[0].as_ref().expect("the MIDI clip in the same scene");
        assert_eq!((clip.length, clip.name.as_str()), (8.0, "Break"));
        let notes = clip.notes();
        assert_eq!(notes.len(), 16);
        // Each note on its slice's hit (a millisecond early), as long as the slice.
        let step = sr as usize / 8;
        let strong: Vec<usize> = hits.iter().copied().filter(|h| ![11 * step, 27 * step].contains(h)).collect();
        for (k, n) in notes.iter().enumerate() {
            assert_eq!(n.pitch, 36 + k as u8);
            let want = strong[k] as f64 / sr as f64 * 2.0;
            assert!((n.start - want).abs() < 0.004 * 2.0 + 1e-9, "note {k} at {} for a hit at {want}", n.start);
            let next = notes.get(k + 1).map_or(8.0, |m| m.start);
            assert!((n.end() - next).abs() < 1e-9);
        }
        // The pads hold their slices, faded at both ends.
        let s0 = pads[1].sample.as_ref().unwrap();
        let r = s0.region.expect("a region of the file");
        assert_eq!(r.end - r.start, s0.frames() as u64);
        assert_eq!((r.fade_in, r.fade_out), (48, 192));
        assert!(s0.left[0].abs() < 1e-3 && s0.left[s0.frames() - 1].abs() < 1e-3, "faded: {} {}", s0.left[0], s0.left[s0.frames() - 1]);
    }

    #[test]
    fn the_region_follows_the_clips_markers() {
        let s = SampleRef::new(SampleData::from_buffers("x", None, 1000, vec![0.0; 8000], vec![0.0; 8000]));
        let mut a = AudioClip::new(s, 120.0);
        // Looping: the loop, two beats from beat 3 (a beat is 500 frames).
        a.loop_start = 3.0;
        assert_eq!(clip_region(&a, 2.0), (1500, 2500));
        // Once from the Start to the End marker, through a warp marker.
        a.looping = false;
        a.start = 1.0;
        a.end = Some(5.0);
        a.markers = vec![(1.0, 0.5), (5.0, 4.5)];
        assert_eq!(clip_region(&a, 2.0), (500, 4500));
        assert!((frames_per_beat(&a, 2.0) - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn a_grid_slices_on_the_beat_and_merges_beyond_the_pads() {
        let sr = 48_000;
        let (s, _) = drum_loop(sr, BEAT, false);
        let a = AudioClip::new(SampleRef::new(s), 120.0);
        let flux = analysis::spectral_flux(&a.sample);
        let quarter = SlicePlan::new(&a, 8.0, &opts(SliceBy::Quarter, 16), 4.0, &flux);
        assert_eq!(quarter.starts, (0..8).map(|k| k * sr as usize / 2).collect::<Vec<_>>());
        assert_eq!(quarter.merged, 0);
        let bar = SlicePlan::new(&a, 8.0, &opts(SliceBy::Bar, 16), 4.0, &flux);
        assert_eq!(bar.starts, vec![0, 2 * sr as usize]);
        // A 1/32 grid has 64 lines: the sixteen on the strongest hits stay.
        let fine = SlicePlan::new(&a, 8.0, &opts(SliceBy::ThirtySecond, 16), 4.0, &flux);
        assert_eq!((fine.len(), fine.merged), (16, 48));
        let step = sr as usize / 8;
        let kicks_and_snares = [0, 4, 6, 10, 12, 16, 20, 22, 25, 28];
        for k in kicks_and_snares {
            assert!(fine.starts.contains(&(k * step)), "the hit of step {k} keeps its cut: {:?}", fine.starts);
        }
        // Notes in beats of the clip: an eighth is half a beat.
        let eighth = SlicePlan::new(&a, 8.0, &opts(SliceBy::Eighth, 16), 4.0, &flux);
        let notes = eighth.notes();
        assert_eq!(notes.len(), 16);
        assert!(notes.iter().enumerate().all(|(k, n)| (n.start - k as f64 * 0.5).abs() < 1e-9 && (n.length - 0.5).abs() < 1e-9));
        // Fewer at most when asked.
        assert_eq!(SlicePlan::new(&a, 8.0, &opts(SliceBy::Eighth, 4), 4.0, &flux).len(), 4);
        // A clip cut shorter is sliced as far as it plays.
        let half = SlicePlan::new(&a, 4.0, &opts(SliceBy::Quarter, 16), 4.0, &flux);
        assert_eq!((half.len(), half.to), (4, 2 * sr as usize));
    }

    #[test]
    fn a_clip_of_another_tempo_is_sliced_on_its_own_beats() {
        // A loop at 100 BPM: its beats are 0.6 s; the notes are on them.
        let sr = 48_000;
        let mut x = vec![0.0f32; (sr as f64 * 2.4) as usize];
        for k in 0..4 {
            for (i, v) in crate::analysis::tests::drum('k', sr, 3).iter().enumerate() {
                let at = (k as f64 * 0.6 * sr as f64) as usize + i;
                if at < x.len() {
                    x[at] += v;
                }
            }
        }
        let s = SampleRef::new(SampleData::from_buffers("100", None, sr, x.clone(), x));
        let a = AudioClip::new(s, 100.0);
        let plan = SlicePlan::new(&a, 4.0, &opts(SliceBy::Transients, 16), 4.0, &analysis::spectral_flux(&a.sample));
        let notes = plan.notes();
        assert_eq!(notes.len(), 4);
        for (k, n) in notes.iter().enumerate() {
            assert!((n.start - k as f64).abs() < 0.01, "{n:?}");
        }
    }

    /// The mono mix of interleaved stereo.
    fn mono(x: &[f32]) -> Vec<f32> {
        x.chunks(2).map(|c| 0.5 * (c[0] + c[1])).collect()
    }

    fn correlation(a: &[f32], b: &[f32]) -> f64 {
        let n = a.len().min(b.len());
        let (mut ab, mut aa, mut bb) = (0.0f64, 0.0f64, 0.0f64);
        for i in 0..n {
            ab += a[i] as f64 * b[i] as f64;
            aa += a[i] as f64 * a[i] as f64;
            bb += b[i] as f64 * b[i] as f64;
        }
        ab / (aa * bb).sqrt().max(1e-12)
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt()
    }

    #[test]
    fn the_new_track_sounds_like_the_loop() {
        let sr = 48_000;
        let (path, _) = loop_file("render", sr, true);
        let sample = SampleRef::load(&path).unwrap();
        for (by, gate) in [(SliceBy::Transients, false), (SliceBy::Sixteenth, false), (SliceBy::Eighth, true)] {
            let mut p = project_with(sample.clone(), 120.0);
            let o = SliceOptions { gate, ..opts(by, 16) };
            p.slice_to_midi_track(0, SliceClip::Slot(0), &o, None).unwrap();
            let original = {
                let mut q = p.clone();
                q.tracks.remove(1);
                mono(&render(&q, RenderSource::Scene { scene: 0, beats: 8.0 }, sr, 0.0, None).unwrap())
            };
            let sliced = {
                let mut q = p.clone();
                q.tracks.remove(0);
                mono(&render(&q, RenderSource::Scene { scene: 0, beats: 8.0 }, sr, 0.0, None).unwrap())
            };
            let c = correlation(&original, &sliced);
            let level = rms(&sliced) / rms(&original);
            assert!(c > 0.97, "{by:?}: correlation {c}");
            assert!((level - 1.0).abs() < 0.1, "{by:?}: as loud as the loop ({level})");
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_set_keeps_the_slices_as_parts_of_the_file() {
        let sr = 44_100;
        let (path, _) = loop_file("save", sr, false);
        let sample = SampleRef::load(&path).unwrap();
        let mut p = project_with(sample, 120.0);
        p.slice_to_midi_track(0, SliceClip::Slot(0), &SliceOptions { gate: true, ..opts(SliceBy::Quarter, 16) }, None).unwrap();
        let set = std::env::temp_dir().join(format!("krabton_slice_{}.krab", std::process::id()));
        p.save(&set).unwrap();
        let json = std::fs::read_to_string(&set).unwrap();
        assert!(json.contains("\"region\"") && json.contains("\"gate\": true"));
        let (back, missing) = Project::load(&set).unwrap();
        assert!(missing.is_empty(), "{missing:?}");
        let pads = |p: &Project| match &p.tracks[1].instrument {
            Some(Instrument::Drums { pads, .. }) => pads.clone(),
            _ => panic!("a Drum Rack"),
        };
        let (a, b) = (pads(&p), pads(&back));
        assert_eq!(a.len(), 8);
        for (x, y) in a.iter().zip(&b) {
            let (sx, sy) = (x.sample.as_ref().unwrap(), y.sample.as_ref().unwrap());
            assert_eq!((sx.region, sx.name.as_str(), sx.path.as_deref()), (sy.region, sy.name.as_str(), sy.path.as_deref()));
            assert_eq!(sx.left, sy.left, "cut again the same");
            assert!(y.gate && y.trim_db > 9.0);
        }
        // An old set (no regions) still opens: a plain sample stays whole.
        assert!(matches!(&back.tracks[0].slots[0], Some(c) if matches!(&c.content, ClipContent::Audio(a) if a.sample.region.is_none() && a.sample.frames() == 32 * (sr as usize / 8))));
        // A missing file keeps its slices' places for when it is back.
        let _ = std::fs::remove_file(&path);
        let (gone, missing) = Project::load(&set).unwrap();
        assert!(!missing.is_empty());
        assert_eq!(pads(&gone)[3].sample.as_ref().unwrap().region, a[3].sample.as_ref().unwrap().region);
        let _ = std::fs::remove_file(&set);
    }

    #[test]
    fn a_sample_from_the_browser_and_an_arrangement_clip_slice_too() {
        let sr = 48_000;
        let (s, _) = drum_loop(sr, BEAT, false);
        let sample = SampleRef::new(s);
        let mut p = Project::blank();
        p.bpm = 120.0;
        let done = p.slice_sample_to_midi_track(sample.clone(), SlicePlace::Arrangement { start: 16.0, length: 0.0, offset: 0.0 }, &opts(SliceBy::Quarter, 16), None).unwrap();
        let ac = &p.tracks[done.track].arrangement[0];
        assert_eq!((ac.start, ac.length, done.slices), (16.0, 8.0, 8));
        // An arrangement clip: the MIDI clip lies where it lies.
        let t = p.add_audio_track();
        let mut clip = Clip::new_audio(p.alloc_id(), sample, [9, 9, 9], 120.0);
        clip.name = "Loop".into();
        let id = p.alloc_id();
        p.tracks[t].arrangement.push(ArrangementClip { offset: 2.0, ..ArrangementClip::new(id, 4.0, 6.0, clip) });
        let done = p.slice_to_midi_track(t, SliceClip::Arrangement(id), &opts(SliceBy::Half, 16), None).unwrap();
        let ac = &p.tracks[done.track].arrangement[0];
        assert_eq!((ac.start, ac.length, ac.offset, ac.clip.length), (4.0, 6.0, 2.0, 8.0));
        assert!(p.slice_to_midi_track(done.track, SliceClip::Slot(0), &opts(SliceBy::Half, 16), None).is_err(), "a MIDI track has no audio to slice");
    }

    /// The drum loop as the sound of a film (its path a film's).
    fn film_sound(sr: u32, path: &str) -> SampleRef {
        let (s, _) = drum_loop(sr, BEAT, true);
        SampleRef::new(SampleData::from_buffers("Zoo", Some(path.into()), sr, s.left.clone(), s.right.clone()))
    }

    fn pads_of(p: &Project, t: usize) -> Vec<DrumPad> {
        match &p.tracks[t].instrument {
            Some(Instrument::Drums { pads, .. }) => pads.clone(),
            _ => panic!("a Drum Rack"),
        }
    }

    #[test]
    fn a_films_sound_brings_its_picture_cut_at_the_same_places() {
        let sr = 48_000;
        let film = film_sound(sr, "C:/films/Zoo.mp4");
        let mut p = project_with(film.clone(), 120.0);
        // Without Picture too, the Video is left alone.
        let mut q = p.clone();
        let done = q.slice_to_midi_track(0, SliceClip::Slot(0), &opts(SliceBy::Transients, 16), None).unwrap();
        assert_eq!(done.picture, None);
        assert!(q.video.clips.is_empty());
        // With it: the film in the bank, cued where the pads' slices start.
        let o = SliceOptions { gate: true, picture: true, ..opts(SliceBy::Transients, 16) };
        let done = p.slice_to_midi_track(0, SliceClip::Slot(0), &o, None).unwrap();
        assert_eq!(done.picture, Some(0));
        let v = &p.video;
        assert_eq!((v.clips.len(), v.current, v.clips[0].play.mode), (1, 0, crate::video::PlayMode::Slices));
        assert_eq!(v.clips[0].path, std::path::PathBuf::from("C:/films/Zoo.mp4"));
        let cues = v.clips[0].cues.clone().expect("cues");
        let pads = pads_of(&p, done.track);
        assert_eq!(cues.len(), pads.len());
        for (k, pad) in pads.iter().enumerate() {
            let r = pad.sample.as_ref().unwrap().region.unwrap();
            assert_eq!(cues.times[k], r.start as f64 / sr as f64, "cue {k} on its pad's slice");
        }
        assert_eq!(cues.end, 4.0, "two bars at 120 BPM");
        assert_eq!((cues.track, cues.gate), (Some(p.tracks[done.track].id), true));
        // The clip's notes come when their cues are in the film (the song at the clip's tempo).
        let notes = p.tracks[done.track].slots[0].as_ref().unwrap().notes();
        for (k, n) in notes.iter().enumerate() {
            assert_eq!(cues.for_note(n.pitch), k);
            assert!((n.start * 0.5 - cues.times[k]).abs() < 1e-9);
        }
        // Sliced again (on the beat): a new track, and the same film's cues follow it.
        let again = p.slice_to_midi_track(0, SliceClip::Slot(0), &SliceOptions { picture: true, ..opts(SliceBy::Quarter, 16) }, None).unwrap();
        assert_eq!(again.picture, Some(0));
        assert_eq!(p.video.clips.len(), 1);
        let cues = p.video.clips[0].cues.clone().unwrap();
        assert_eq!(cues.times, (0..8).map(|k| k as f64 * 0.5).collect::<Vec<_>>());
        assert_eq!((cues.track, cues.gate), (Some(p.tracks[again.track].id), false));
        // A part of the film (a region of it) is cued at its place in the film.
        let part = SampleRef::new(film.region("Zoo part", SampleRegion { start: sr as u64, end: 3 * sr as u64, fade_in: 0, fade_out: 0 }));
        let mut p2 = Project::blank();
        p2.bpm = 120.0;
        let o = SliceOptions { picture: true, ..opts(SliceBy::Half, 16) };
        let done = p2.slice_sample_to_midi_track(part, SlicePlace::Slot(0), &o, None).unwrap();
        let cues = p2.video.clips[done.picture.unwrap()].cues.clone().unwrap();
        assert_eq!((cues.times[0], cues.end), (1.0, 3.0));
        // A sound file is no film: no picture, whatever is asked.
        let mut p3 = project_with(film_sound(sr, "C:/loops/zoo.wav"), 120.0);
        let done = p3.slice_to_midi_track(0, SliceClip::Slot(0), &SliceOptions { picture: true, ..opts(SliceBy::Quarter, 16) }, None).unwrap();
        assert_eq!(done.picture, None);
        assert!(p3.video.clips.is_empty());
    }

    #[test]
    fn a_real_films_sound_and_picture_are_cut_alike() {
        // A small film with ffmpeg (skipped without it): a test picture and
        // a low thump every half second, from 0.
        let Some(ffmpeg) = crate::ffmpeg::find() else {
            eprintln!("no ffmpeg: skipped");
            return;
        };
        let path = std::env::temp_dir().join(format!("krabton_slice_film_{}.mp4", std::process::id()));
        let out = crate::ffmpeg::command(&ffmpeg)
            .args(["-y", "-v", "error", "-f", "lavfi", "-i", "testsrc2=size=160x90:rate=25:duration=4", "-f", "lavfi", "-i", r"aevalsrc=0.8*sin(2*PI*90*t)*exp(-25*mod(t\,0.5)):s=48000:d=4", "-shortest", "-c:v", "mpeg4", "-c:a", "aac"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let sound = SampleRef::new(crate::sample::decode_with_external(&path).unwrap());
        assert_eq!(sound.path.as_deref(), Some(path.as_path()));
        let mut p = Project::blank();
        p.bpm = 120.0;
        let o = SliceOptions { picture: true, ..Default::default() };
        let done = p.slice_sample_to_midi_track(sound.clone(), SlicePlace::Slot(0), &o, None).unwrap();
        let cues = p.video.clips[done.picture.expect("the film in the Video")].cues.clone().unwrap();
        // A cue on every thump (a millisecond early), the pads cut there too.
        assert_eq!(cues.len(), 8, "{:?}", cues.times);
        for (k, t) in cues.times.iter().enumerate() {
            assert!((t - k as f64 * 0.5).abs() < 0.02, "cue {k} at {t}");
        }
        for (pad, t) in pads_of(&p, done.track).iter().zip(&cues.times) {
            assert_eq!(pad.sample.as_ref().unwrap().region.unwrap().start as f64 / sound.sample_rate as f64, *t);
        }
        let _ = std::fs::remove_file(&path);
    }
}
