//! Audio recording on the UI side: the audio interface input, the takes
//! that collect what the engine captures for armed audio tracks, and turning
//! them into audio clips (in a session slot or on the arrangement) backed by
//! WAV files.
//!
//! The engine streams the input of every armed audio track while capturing,
//! stamped with the session and arrangement positions. A take keeps the part
//! between its start and end beat, so session recordings start and stop on
//! the launch quantization grid like clips do.

use crate::app::{App, ClipSel, TrackSel, View};
use crate::audio;
use krabton_engine::{ArrangementClip, AudioClip, AudioInput, Clip, ClipContent, Command, Monitor, SampleData, SampleRef, Tap, TrackKind};
use std::path::PathBuf;
use std::sync::atomic::Ordering;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TakeTarget {
    /// A session slot (scene index), on the session clock.
    Slot(usize),
    /// The arrangement, on the song position.
    Arrangement,
}

/// One recording in progress.
pub struct Take {
    pub track: u64,
    pub target: TakeTarget,
    /// First beat kept; `None` until the first block arrives.
    pub start: Option<f64>,
    /// Beat to stop at, once known.
    pub end: Option<f64>,
    pub l: Vec<f32>,
    pub r: Vec<f32>,
    /// Beat just after the last frame received.
    pub last_beat: f64,
    /// The clip put in the slot when the take was told to stop, so it can
    /// start playing on time; its audio is completed when the take ends.
    pub clip_id: Option<u64>,
    /// Whether any block arrived yet (the transport may still be starting).
    pub got_data: bool,
    /// UI frame the take was made in.
    pub created: u64,
}

impl Take {
    fn new(track: u64, target: TakeTarget, start: Option<f64>, created: u64) -> Self {
        Self { track, target, start, end: None, l: Vec::new(), r: Vec::new(), last_beat: start.unwrap_or(0.0), clip_id: None, got_data: false, created }
    }

    /// Length recorded so far, in beats.
    pub fn beats(&self, samples_per_beat: f64) -> f64 {
        self.l.len() as f64 / samples_per_beat
    }
}

impl App {
    fn samples_per_beat(&self) -> f64 {
        self.engine.status.sample_rate.load() as f64 * 60.0 / self.project.bpm.max(1.0)
    }

    /// The next launch quantization point on the session clock.
    fn next_launch_beat(&self) -> f64 {
        let pos = self.engine.status.session_pos.load();
        match self.project.launch_quantize.beats(self.project.beats_per_bar) {
            None => pos,
            Some(g) => (pos / g - 1e-9).ceil() * g,
        }
    }

    pub fn take_on(&self, track: u64, target: TakeTarget) -> Option<&Take> {
        self.takes.iter().find(|t| t.track == track && t.target == target)
    }

    /// Starts recording track `ti` into session slot `scene`, at the next
    /// launch point (or right away when the transport is stopped).
    pub fn start_slot_take(&mut self, ti: usize, scene: usize) {
        let Some(t) = self.project.tracks.get(ti) else { return };
        if t.kind != TrackKind::Audio || self.takes.iter().any(|k| k.track == t.id) {
            return;
        }
        let id = t.id;
        if !t.arm {
            self.project.tracks[ti].arm = true;
        }
        let start = if self.is_playing() {
            Some(self.next_launch_beat())
        } else {
            self.cmd(Command::Play);
            None
        };
        self.takes.push(Take::new(id, TakeTarget::Slot(scene), start, self.frame));
        self.cmd(Command::SetCapture(true));
        self.recording = true;
        self.sel_track = TrackSel::Track(ti);
        self.sel_scene = scene;
    }

    /// Stops a session take at the next launch point. The clip goes into the
    /// slot right away and is launched there, so it plays on as the take ends.
    pub fn stop_slot_take(&mut self, ti: usize) {
        let Some(track) = self.project.tracks.get(ti).map(|t| t.id) else { return };
        let end = self.next_launch_beat();
        let Some(k) = self.takes.iter().position(|t| t.track == track && matches!(t.target, TakeTarget::Slot(_)) && t.end.is_none()) else { return };
        let take = &mut self.takes[k];
        let start = take.start.unwrap_or(end);
        // Shorter than a beat: keep going to the following launch point.
        let mut end = end;
        if end - start < 1.0 {
            end = start + self.project.launch_quantize.beats(self.project.beats_per_bar).unwrap_or(1.0).max(1.0);
        }
        take.end = Some(end);
        let TakeTarget::Slot(scene) = take.target else { return };
        let (l, r) = (take.l.clone(), take.r.clone());
        let clip_id = self.project.alloc_id();
        self.takes[k].clip_id = Some(clip_id);
        let name = self.take_name(ti);
        let clip = self.recorded_clip(ti, clip_id, end - start, l, r, None, name);
        if let Some(slot) = self.project.tracks[ti].slots.get_mut(scene) {
            *slot = Some(clip);
            self.cmd(Command::LaunchClip { track, slot: scene });
        }
    }

    /// Records every armed audio track on the arrangement from the play
    /// position, starting the transport if needed.
    pub fn start_arrangement_takes(&mut self) -> usize {
        let ids: Vec<u64> = self.project.tracks.iter().filter(|t| t.arm && t.kind == TrackKind::Audio).map(|t| t.id).collect();
        let mut n = 0;
        for id in ids {
            if !self.takes.iter().any(|k| k.track == id) {
                self.takes.push(Take::new(id, TakeTarget::Arrangement, None, self.frame));
                n += 1;
            }
        }
        if n > 0 {
            self.cmd(Command::SetCapture(true));
            if !self.is_playing() {
                self.cmd(Command::Play);
            }
        }
        n
    }

    /// Records every armed audio track into the first free slot from the
    /// selected scene down.
    pub fn start_session_takes(&mut self) -> usize {
        let mut n = 0;
        for ti in 0..self.project.tracks.len() {
            let t = &self.project.tracks[ti];
            if !t.arm || t.kind != TrackKind::Audio || self.takes.iter().any(|k| k.track == t.id) {
                continue;
            }
            let ns = self.project.scenes.len();
            let from = self.sel_scene.min(ns.saturating_sub(1));
            let scene = match (from..ns).find(|&s| self.project.tracks[ti].slots[s].is_none()) {
                Some(s) => s,
                None => {
                    self.project.add_scene(ns);
                    ns
                }
            };
            let keep = self.sel_scene;
            self.start_slot_take(ti, scene);
            self.sel_scene = keep;
            n += 1;
        }
        n
    }

    /// Stops all takes: session ones at the next launch point, arrangement
    /// ones right away.
    pub fn stop_takes(&mut self) {
        let slot_tracks: Vec<usize> = self
            .takes
            .iter()
            .filter(|t| matches!(t.target, TakeTarget::Slot(_)) && t.end.is_none())
            .filter_map(|t| self.project.track_index(t.track))
            .collect();
        for ti in slot_tracks {
            self.stop_slot_take(ti);
        }
        for t in self.takes.iter_mut().filter(|t| t.target == TakeTarget::Arrangement) {
            t.end = Some(t.last_beat);
        }
    }

    /// The name the next recording on track `ti` gets.
    fn take_name(&self, ti: usize) -> String {
        format!("{} {}", self.project.tracks[ti].name, self.rec_counter + 1)
    }

    /// A clip holding recorded audio (from the WAV file at `path`).
    #[allow(clippy::too_many_arguments)]
    fn recorded_clip(&self, ti: usize, id: u64, beats: f64, l: Vec<f32>, r: Vec<f32>, path: Option<PathBuf>, name: String) -> Clip {
        let t = &self.project.tracks[ti];
        let sr = self.engine.status.sample_rate.load() as u32;
        let sample = SampleRef::new(SampleData::from_buffers(name.clone(), path, sr, l, r));
        Clip { id, name, color: t.color, length: beats.max(0.0625), content: ClipContent::Audio(AudioClip { sample, bpm: self.project.bpm, gain_db: 0.0 }), envelopes: Vec::new(), sound: None }
    }

    /// Where recordings are written: next to the saved set, or in the
    /// user's Documents folder.
    fn recordings_dir(&self) -> PathBuf {
        let base = match &self.file_path {
            Some(p) => p.parent().map(|d| d.to_path_buf()).unwrap_or_default(),
            None => {
                let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default();
                home.join("Documents").join("Krabton")
            }
        };
        base.join("Recorded")
    }

    /// Opens the audio interface input once a track needs it.
    fn ensure_input(&mut self) {
        let needed = self.project.tracks.iter().any(|t| t.kind == TrackKind::Audio && matches!(t.input, AudioInput::Ext { .. }) && (t.arm || t.monitor == Monitor::In));
        if !needed || self.input_dev.is_some() || self.input_error.is_some() {
            return;
        }
        if std::env::var_os("KRABTON_NO_AUDIO").is_some() {
            self.input_error = Some("disabled by KRABTON_NO_AUDIO".into());
            return;
        }
        let sr = self.engine.status.sample_rate.load() as u32;
        match audio::start_input(sr, self.prefs.audio_input.as_deref()) {
            Ok((dev, ring)) => {
                self.toast(format!("Audio input: {} ({} channels)", dev.name, dev.channels));
                self.cmd(Command::SetInput(Some(ring)));
                self.input_dev = Some(dev);
            }
            Err(e) => {
                self.toast(format!("No audio input: {e}"));
                self.input_error = Some(e);
            }
        }
    }

    /// Per frame: opens the input when needed, collects recorded audio and
    /// finishes takes.
    pub fn poll_recording(&mut self) {
        self.ensure_input();
        self.poll_midi_recording();
        let spb = self.samples_per_beat();
        while let Ok(c) = self.engine.recorded.try_recv() {
            let Some(take) = self.takes.iter_mut().find(|t| t.track == c.track && t.end.is_none_or(|e| t.last_beat < e)) else { continue };
            let beat0 = match take.target {
                TakeTarget::Slot(_) => c.session_beat,
                TakeTarget::Arrangement => c.song_beat,
            };
            // The arrangement looped back: this take is done.
            if take.target == TakeTarget::Arrangement && take.start.is_some() && beat0 + 1e-6 < take.last_beat {
                take.end = Some(take.last_beat);
                continue;
            }
            take.got_data = true;
            let start = *take.start.get_or_insert(beat0);
            let (first, last) = keep_range(start, take.end, beat0, c.frames, spb);
            if last > first {
                take.l.extend_from_slice(&c.l[first..last]);
                take.r.extend_from_slice(&c.r[first..last]);
            }
            take.last_beat = beat0 + c.frames as f64 / spb;
        }
        // The transport stopped: whatever is recorded is the take. A take
        // that has not heard anything yet waits for the transport to start.
        if !self.is_playing() {
            let frame = self.frame;
            for t in self.takes.iter_mut().filter(|t| t.end.is_none() && (t.got_data || frame > t.created + 120)) {
                t.end = Some(t.last_beat);
            }
        }
        let mut k = 0;
        while k < self.takes.len() {
            let t = &self.takes[k];
            let done = t.end.is_some_and(|e| t.last_beat >= e - 1e-6) || (!self.is_playing() && t.end.is_some() && (t.got_data || self.frame > t.created + 120));
            if done {
                let take = self.takes.remove(k);
                self.finish_take(take, spb);
            } else {
                k += 1;
            }
        }
        if self.takes.is_empty() && self.engine.status.capturing.load(Ordering::Relaxed) {
            self.cmd(Command::SetCapture(false));
        }
    }

    /// Writes a finished take to disk and puts its clip in place.
    fn finish_take(&mut self, take: Take, spb: f64) {
        let Some(ti) = self.project.track_index(take.track) else { return };
        let (Some(start), Some(end)) = (take.start, take.end) else { return };
        if take.l.is_empty() {
            if let (TakeTarget::Slot(scene), Some(_)) = (take.target, take.clip_id) {
                self.project.tracks[ti].slots[scene] = None;
            }
            return;
        }
        let beats = (end - start).max(0.0625);
        let mut l = take.l;
        let mut r = take.r;
        // Pad to the exact length (blocks lost to a stall become silence).
        let frames = (beats * spb).round() as usize;
        if l.len() < frames {
            l.resize(frames, 0.0);
            r.resize(frames, 0.0);
        }
        let name = self.take_name(ti);
        self.rec_counter += 1;
        let dir = self.recordings_dir();
        let tname: String = self.project.tracks[ti].name.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' }).collect();
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let path = dir.join(format!("{} {} {stamp}.wav", tname.trim(), self.rec_counter));
        let sr = self.engine.status.sample_rate.load() as u32;
        let interleaved: Vec<f32> = l.iter().zip(&r).flat_map(|(a, b)| [*a, *b]).collect();
        let saved = std::fs::create_dir_all(&dir).map_err(|e| e.to_string()).and_then(|_| krabton_engine::render::write_wav(&path, sr, &interleaved));
        let path = match saved {
            Ok(()) => Some(path),
            Err(e) => {
                self.toast(format!("Recording kept in memory only: {e}"));
                None
            }
        };
        match take.target {
            TakeTarget::Slot(scene) => {
                let id = take.clip_id.unwrap_or_else(|| self.project.alloc_id());
                let clip = self.recorded_clip(ti, id, beats, l, r, path, name);
                if let Some(slot) = self.project.tracks[ti].slots.get_mut(scene) {
                    *slot = Some(clip);
                }
                if take.clip_id.is_none() {
                    self.select_clip(ClipSel::Slot { track: ti, scene });
                }
            }
            TakeTarget::Arrangement => {
                let id = self.project.alloc_id();
                let clip = self.recorded_clip(ti, id, beats, l, r, path, name);
                let aid = self.project.alloc_id();
                self.project.tracks[ti].arrangement.push(ArrangementClip { id: aid, start, length: clip.length, offset: 0.0, clip });
                self.select_clip(ClipSel::Arr { track: ti, id: aid });
            }
        }
        if self.takes.is_empty() {
            self.recording = false;
        }
    }

    /// Makes a new audio track that listens to track `src` and arms it, so
    /// recording a slot on it records `src` as audio.
    pub fn route_into_new_audio_track(&mut self, src: usize) -> Option<usize> {
        let (src_id, src_name) = self.project.tracks.get(src).map(|t| (t.id, t.name.clone()))?;
        let ai = self.project.add_audio_track();
        let t = &mut self.project.tracks[ai];
        t.name = format!("{src_name} Audio");
        t.input = AudioInput::Track { id: src_id, tap: Tap::PostFx };
        t.monitor = Monitor::Off;
        t.arm = true;
        self.sel_track = TrackSel::Track(ai);
        if self.view == View::Session {
            self.sel_scene = self.project.tracks[ai].slots.iter().position(|s| s.is_none()).unwrap_or(0);
        }
        self.toast(format!("{} records {src_name}: click a slot's record button, or press Record", self.project.tracks[ai].name));
        Some(ai)
    }
}

/// Which frames of a block starting at `beat0` belong to a take from
/// `start` to `end` (beats), as a range of frame indices.
pub fn keep_range(start: f64, end: Option<f64>, beat0: f64, frames: usize, samples_per_beat: f64) -> (usize, usize) {
    let at = |beat: f64| (((beat - beat0) * samples_per_beat).round().max(0.0) as usize).min(frames);
    let first = at(start);
    let last = end.map(at).unwrap_or(frames);
    (first, last.max(first))
}

#[cfg(test)]
mod tests {
    use super::keep_range;

    #[test]
    fn takes_keep_exactly_the_frames_between_start_and_end() {
        let spb = 24000.0; // 120 BPM at 48 kHz
        // A block before the start, one across it, one across the end.
        assert_eq!(keep_range(4.0, None, 3.99, 256, spb), (240, 256));
        assert_eq!(keep_range(4.0, None, 3.0, 256, spb), (256, 256));
        assert_eq!(keep_range(4.0, Some(8.0), 7.995, 256, spb), (0, 120));
        assert_eq!(keep_range(4.0, Some(8.0), 8.5, 256, spb), (0, 0));
        // Consecutive blocks add up to the exact length of the take.
        let mut kept = 0;
        let mut beat = 3.9;
        while beat < 8.1 {
            let (a, b) = keep_range(4.0, Some(8.0), beat, 256, spb);
            kept += b - a;
            beat += 256.0 / spb;
        }
        assert_eq!(kept, 4 * 24000);
    }
}
