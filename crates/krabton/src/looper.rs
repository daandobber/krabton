//! The Looper on the UI side: every Looper given its memory (with its loop,
//! when a set brings one), the song's tempo set by a first loop, the pedal
//! on F8, the loop made into a clip (To Clip, or dragged out of the device
//! as a file), and the loops saved as WAV files next to the set.

use crate::app::{App, BrowserItem, ClipSel, TrackSel, View};
use krabton_engine::dsp::fx::looper::{LoopAction, LoopHandle, LoopMemory, LoopState, MAX_SECONDS};
use krabton_engine::project::Device;
use krabton_engine::{ArrangementClip, AudioClip, Clip, ClipContent, SampleData, SampleRef};
use std::path::PathBuf;
use std::sync::Arc;

/// The tempo a Looper's loop was recorded at, as its device keeps it (0: free).
fn kept_bpm(d: &Device) -> f32 {
    d.data.get(..4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).filter(|v| v.is_finite()).unwrap_or(0.0)
}

impl App {
    /// Per frame, before the project goes to the engine: a memory for every
    /// Looper without one (its saved loop in it), the bar's length for all,
    /// and the song's tempo from a first loop that sets it. Returns whether
    /// a memory was given (the engine must hear of it).
    pub(crate) fn looper_frame(&mut self) -> bool {
        let sr = self.engine.status.sample_rate.load().max(8000.0);
        let bar = self.project.beats_per_bar_f() as f32;
        let mut made = false;
        let mut tempo = None;
        for d in self.project.loopers_mut() {
            if d.looper.0.is_none() {
                let m = LoopMemory::new(sr, MAX_SECONDS);
                if let Some(s) = &d.sample {
                    let (l, r) = at_rate(s, sr);
                    m.load(&l, &r, kept_bpm(d));
                }
                d.looper = LoopHandle(Some(Arc::new(m)));
                made = true;
            }
            if let Some(m) = &d.looper.0 {
                m.bar_beats.store(bar);
                let t = m.set_tempo.swap(0.0);
                if t > 0.0 {
                    tempo = Some(t);
                }
            }
        }
        if let Some(t) = tempo {
            self.project.bpm = (t as f64 * 100.0).round() / 100.0;
            self.toast(format!("The loop set the tempo: {:.2} BPM", self.project.bpm));
        }
        made
    }

    /// The memory of the Looper with this device id.
    pub(crate) fn looper_memory(&self, dev: u64) -> Option<Arc<LoopMemory>> {
        let on_tracks = self.project.tracks.iter().flat_map(|t| &t.devices);
        let rest = self.project.returns.iter().flat_map(|r| &r.devices).chain(&self.project.master.devices);
        on_tracks.chain(rest).find(|d| d.id == dev).and_then(|d| d.looper.0.clone())
    }

    /// The Looper the keys play: the first on the selected track (or return,
    /// or the master), else the first in the set.
    fn played_looper(&self) -> Option<Arc<LoopMemory>> {
        let chain = match self.sel_track {
            TrackSel::Track(i) => self.project.tracks.get(i).map(|t| &t.devices),
            TrackSel::Return(j) => self.project.returns.get(j).map(|r| &r.devices),
            TrackSel::Master => Some(&self.project.master.devices),
        };
        let first = |ds: &Vec<Device>| ds.iter().find(|d| d.looper.0.is_some()).and_then(|d| d.looper.0.clone());
        chain.and_then(first).or_else(|| self.project.all_devices().find(|d| d.looper.0.is_some()).and_then(|d| d.looper.0.clone()))
    }

    /// F8 (Shift+F8: stop): the Looper's pedal.
    pub(crate) fn looper_key(&mut self, a: LoopAction) {
        match self.played_looper() {
            Some(m) => {
                m.press(a);
            }
            None => self.toast("No Looper here: put one on a track (Audio Effects > Delay & Loop)"),
        }
    }

    /// The loop as a WAV file in the recordings folder (written once for
    /// each version of the loop), with how many beats it lasts.
    pub(crate) fn looper_file(&mut self, dev: u64) -> Option<(PathBuf, f64, f32)> {
        let m = self.looper_memory(dev)?;
        let gen = m.generation.load(std::sync::atomic::Ordering::Relaxed);
        let (l, r) = m.frames();
        if l.is_empty() {
            self.toast("The Looper has no loop yet");
            return None;
        }
        let sr = m.rate();
        let bpm = match m.loop_bpm.load() {
            b if b > 0.0 => b,
            _ => self.project.bpm as f32,
        };
        let beats = l.len() as f64 / (sr as f64 * 60.0 / bpm as f64);
        if let Some((g, path)) = self.looper_files.get(&dev) {
            if *g == gen && path.exists() {
                return Some((path.clone(), beats, bpm));
            }
        }
        let dir = self.recordings_dir();
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        let path = dir.join(format!("Loop {stamp}.wav"));
        let audio: Vec<f32> = l.iter().zip(&r).flat_map(|(a, b)| [*a, *b]).collect();
        match std::fs::create_dir_all(&dir).map_err(|e| e.to_string()).and_then(|_| krabton_engine::render::write_wav(&path, sr as u32, &audio)) {
            Ok(()) => {
                self.looper_files.insert(dev, (gen, path.clone()));
                Some((path, beats, bpm))
            }
            Err(e) => {
                self.toast(format!("The loop could not be written: {e}"));
                None
            }
        }
    }

    /// To Clip: the loop as an audio clip on a new audio track, in the
    /// selected scene (or at the playhead's bar in the arrangement).
    pub(crate) fn looper_to_clip(&mut self, dev: u64) {
        let Some((path, beats, bpm)) = self.looper_file(dev) else { return };
        let sample = match SampleRef::load(&path) {
            Ok(s) => s,
            Err(e) => {
                self.toast(format!("The loop could not be read back: {e}"));
                return;
            }
        };
        let ti = self.add_audio_track();
        let name = format!("Loop {}", self.project.tracks.len());
        self.project.tracks[ti].name = name.clone();
        let id = self.project.alloc_id();
        let color = self.project.tracks[ti].color;
        let clip = Clip { id, name, color, length: beats.max(0.0625), content: ClipContent::Audio(AudioClip::new(sample, bpm as f64)), envelopes: Vec::new(), sound: None };
        if self.view == View::Arrangement {
            let bar = self.project.beats_per_bar_f();
            let at = (self.status().song_pos.load() / bar).floor() * bar;
            let aid = self.project.alloc_id();
            let len = clip.length;
            self.project.tracks[ti].arrangement.push(ArrangementClip::new(aid, at, len, clip));
            self.select_clip(ClipSel::Arr { track: ti, id: aid });
        } else {
            let scene = self.sel_scene.min(self.project.scenes.len().saturating_sub(1));
            if let Some(slot) = self.project.tracks[ti].slots.get_mut(scene) {
                *slot = Some(clip);
            }
            self.select_clip(ClipSel::Slot { track: ti, scene });
        }
        self.toast(format!("The loop is a clip on {}", self.project.tracks[ti].name));
    }

    /// Dragging the loop out of the device: it goes as a sample, and lands
    /// where a sample from the browser would (a slot, a lane, a track).
    pub(crate) fn looper_drag(&mut self, ctx: &egui::Context, dev: u64) {
        if let Some((path, _, _)) = self.looper_file(dev) {
            egui::DragAndDrop::set_payload(ctx, BrowserItem::Sample(path));
        }
    }

    /// Writes every Looper's loop next to the set (`dir`, the set named
    /// `name`) as `<set> Looper 1.wav`, ... (only what changed), and keeps
    /// it in the device, so the set brings it back.
    pub(crate) fn looper_save(&mut self, dir: &std::path::Path, name: &str) {
        let mut errors = Vec::new();
        for (k, d) in self.project.loopers_mut().into_iter().enumerate() {
            let Some(m) = d.looper.0.clone() else { continue };
            let (l, r) = m.frames();
            if l.is_empty() {
                d.sample = None;
                d.data.clear();
                continue;
            }
            let path = dir.join(format!("{name} Looper {}.wav", k + 1));
            let gen = m.generation.load(std::sync::atomic::Ordering::Relaxed);
            let fresh = d.sample.as_ref().and_then(|s| s.path.as_deref()) == Some(path.as_path()) && path.exists();
            if fresh && m.saved.load(std::sync::atomic::Ordering::Relaxed) == gen {
                continue;
            }
            let sr = m.rate() as u32;
            let audio: Vec<f32> = l.iter().zip(&r).flat_map(|(a, b)| [*a, *b]).collect();
            match krabton_engine::render::write_wav(&path, sr, &audio) {
                Ok(()) => {
                    m.saved.store(gen, std::sync::atomic::Ordering::Relaxed);
                    let label = format!("{name} Looper {}", k + 1);
                    d.sample = Some(SampleRef::new(SampleData::from_buffers(label, Some(path), sr, l, r)));
                    d.data = m.loop_bpm.load().to_le_bytes().to_vec();
                }
                Err(e) => errors.push(format!("Looper {} not saved: {e}", k + 1)),
            }
        }
        if let Some(e) = errors.into_iter().next() {
            self.toast(e);
        }
    }

    /// What the Looper with this id is doing (for scripts and the status bar).
    pub(crate) fn looper_state(&self, dev: u64) -> Option<LoopState> {
        self.looper_memory(dev).map(|m| m.state())
    }
}

/// A sample's frames at the rate `sr` (read between its frames when its
/// own rate differs).
fn at_rate(s: &SampleData, sr: f32) -> (Vec<f32>, Vec<f32>) {
    let ratio = s.sample_rate as f64 / sr as f64;
    if (ratio - 1.0).abs() < 1e-9 {
        return (s.left.clone(), s.right.clone());
    }
    let frames = (s.frames() as f64 / ratio) as usize;
    (0..frames).map(|i| s.read(i as f64 * ratio)).unzip()
}
