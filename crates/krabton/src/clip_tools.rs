//! Sound design on audio clips, from their menus: *Stretch* x2, x4, x8 or as
//! much as you like (Paulstretch: a new clip as many times as long, smeared
//! and still), *Reverse to New Clip*, *Freeze to Drone* (thirty seconds held
//! at the moment under the playhead) and *Resample in Place* (the clip as it
//! plays, warp, transpose, gain and fades in it, made its new sample). The
//! work is done on a thread of its own (a stretch of a long clip takes a
//! while) and the new audio is written next to the sample, as Crop does.

use crate::app::{App, ClipSel};
use egui::Context;
use krabton_engine::{AudioClip, ClipContent, SampleData, SampleRef};
use std::path::PathBuf;
use std::thread::JoinHandle;

/// What is made of a clip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ClipTool {
    Stretch(f64),
    Reverse,
    /// A drone held at this many beats into the clip.
    Drone(f64),
    Resample,
}

impl ClipTool {
    fn suffix(self) -> String {
        match self {
            ClipTool::Stretch(f) => format!("Stretch x{}", trim(f)),
            ClipTool::Reverse => "Reverse".into(),
            ClipTool::Drone(_) => "Drone".into(),
            ClipTool::Resample => "Resampled".into(),
        }
    }
}

fn trim(f: f64) -> String {
    if (f - f.round()).abs() < 1e-6 {
        format!("{}", f.round())
    } else {
        format!("{f:.1}")
    }
}

/// How long a drone is.
pub(crate) const DRONE_SECONDS: f64 = 30.0;

type Made = Result<(PathBuf, u32, Vec<f32>, Vec<f32>), String>;

/// A clip being worked on.
pub(crate) struct ClipJob {
    sel: ClipSel,
    tool: ClipTool,
    name: String,
    handle: JoinHandle<Made>,
}

/// The audio a tool makes from a clip's audio (as it plays), at `sr`.
pub(crate) fn make(tool: ClipTool, sr: u32, l: &[f32], r: &[f32], bpm: f64) -> (Vec<f32>, Vec<f32>) {
    use krabton_engine::dsp::stretch;
    let src = SampleData::from_buffers("clip", None, sr, l.to_vec(), r.to_vec());
    match tool {
        ClipTool::Stretch(f) => {
            let frames = (l.len() as f64 * f).round() as usize;
            // Long windows for a big stretch, shorter ones to keep a little more of its movement.
            let window = if f >= 8.0 { 3 } else { 2 };
            stretch::render(&src, 0.0, f, frames, window, 0.0, 0x57E7)
        }
        ClipTool::Reverse => (l.iter().rev().copied().collect(), r.iter().rev().copied().collect()),
        ClipTool::Drone(at) => {
            let from = at * sr as f64 * 60.0 / bpm.max(1.0);
            let frames = (DRONE_SECONDS * sr as f64) as usize;
            let (mut l, mut r) = stretch::render(&src, from, 0.0, frames, 3, 0.0, 0xD20E);
            // A second in and out.
            let fade = (sr as usize).min(frames / 2);
            for i in 0..fade {
                let g = i as f32 / fade as f32;
                l[i] *= g;
                r[i] *= g;
                l[frames - 1 - i] *= g;
                r[frames - 1 - i] *= g;
            }
            (l, r)
        }
        ClipTool::Resample => (l.to_vec(), r.to_vec()),
    }
}

impl App {
    /// The clip tools in a clip's menu (the session, the arrangement, the
    /// clip view).
    pub(crate) fn clip_tools_menu(&mut self, ui: &mut egui::Ui, sel: ClipSel) {
        ui.menu_button("Stretch", |ui| {
            for f in [2.0, 4.0, 8.0] {
                if ui.button(format!("x{}", trim(f))).on_hover_text("A new clip this many times as long, smeared and still (Paulstretch), beside this one").clicked() {
                    self.start_clip_job(sel, ClipTool::Stretch(f));
                    ui.close();
                }
            }
            if ui.button("Custom...").on_hover_text("As many times as long as you like, 1.5 to 1000").clicked() {
                self.stretch_custom = Some((sel, 20.0));
                ui.close();
            }
        });
        if ui.button("Reverse to New Clip").on_hover_text("The clip as it plays, backwards, as a new clip beside it").clicked() {
            self.start_clip_job(sel, ClipTool::Reverse);
            ui.close();
        }
        if ui.button("Freeze to Drone").on_hover_text("Thirty seconds held at the moment under the playhead (or the clip's middle), as a new clip").clicked() {
            let at = self.clip_moment(sel);
            self.start_clip_job(sel, ClipTool::Drone(at));
            ui.close();
        }
        if ui.button("Resample in Place").on_hover_text("The clip as it plays (warp, transpose, gain, fades) made its new sample").clicked() {
            self.start_clip_job(sel, ClipTool::Resample);
            ui.close();
        }
    }

    /// Where the playhead is in a clip (beats from where it starts playing),
    /// or its middle.
    fn clip_moment(&self, sel: ClipSel) -> f64 {
        match sel {
            ClipSel::Slot { .. } => {
                let len = self.clip_ref(sel).map(|c| c.length).unwrap_or(4.0);
                self.clip_play_pos(sel).filter(|p| p.is_finite()).unwrap_or(len * 0.5).clamp(0.0, len)
            }
            ClipSel::Arr { track, id } => {
                let Some(ac) = self.project.tracks.get(track).and_then(|t| t.arrangement.iter().find(|a| a.id == id)) else { return 0.0 };
                let at = self.status().song_pos.load() - ac.start;
                if (0.0..ac.length).contains(&at) {
                    at
                } else {
                    ac.length * 0.5
                }
            }
        }
    }

    /// Starts making something of an audio clip, on a thread of its own.
    pub(crate) fn start_clip_job(&mut self, sel: ClipSel, tool: ClipTool) {
        let (loop_len, offset, length) = match sel {
            ClipSel::Slot { .. } => {
                let Some(c) = self.clip_ref(sel) else { return };
                (c.length, 0.0, c.length)
            }
            ClipSel::Arr { track, id } => {
                let Some(ac) = self.project.tracks.get(track).and_then(|t| t.arrangement.iter().find(|a| a.id == id)) else { return };
                (ac.clip.length, ac.offset, ac.length)
            }
        };
        let Some(clip) = self.clip_ref(sel) else { return };
        let ClipContent::Audio(a) = &clip.content else { return };
        let a = a.clone();
        let bpm = self.project.bpm;
        let s = &a.sample;
        let dir = s.path.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf())).or_else(|| self.file_path.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf()))).unwrap_or_else(|| self.recordings_dir());
        let stem = s.path.as_ref().and_then(|p| p.file_stem()).map(|x| x.to_string_lossy().to_string()).unwrap_or_else(|| clip.name.clone());
        let suffix = tool.suffix();
        let mut path = dir.join(format!("{stem} {suffix}.wav"));
        let mut k = 2;
        while path.exists() {
            path = dir.join(format!("{stem} {suffix} {k}.wav"));
            k += 1;
        }
        let name = format!("{} {suffix}", clip.name);
        let handle = std::thread::spawn(move || -> Made {
            let (sr, audio) = krabton_engine::render::clip_audio(&a, loop_len, offset, length, bpm);
            if audio.len() < 32 {
                return Err("The clip has no audio to work on".into());
            }
            let (l, r): (Vec<f32>, Vec<f32>) = audio.chunks_exact(2).map(|f| (f[0], f[1])).unzip();
            let (l, r) = make(tool, sr, &l, &r, bpm);
            let inter: Vec<f32> = l.iter().zip(&r).flat_map(|(a, b)| [*a, *b]).collect();
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            krabton_engine::render::write_wav(&path, sr, &inter)?;
            Ok((path, sr, l, r))
        });
        let what = match tool {
            ClipTool::Stretch(f) => format!("Stretching {} x{} ...", clip.name, trim(f)),
            ClipTool::Reverse => format!("Reversing {} ...", clip.name),
            ClipTool::Drone(_) => format!("Freezing {} to a drone ...", clip.name),
            ClipTool::Resample => format!("Resampling {} ...", clip.name),
        };
        self.toast(what);
        self.clip_jobs.push(ClipJob { sel, tool, name, handle });
    }

    /// Per frame: clips finished are put in place.
    pub(crate) fn poll_clip_jobs(&mut self) {
        let mut k = 0;
        while k < self.clip_jobs.len() {
            if !self.clip_jobs[k].handle.is_finished() {
                k += 1;
                continue;
            }
            let job = self.clip_jobs.remove(k);
            match job.handle.join() {
                Ok(Ok((path, sr, l, r))) => self.place_made_clip(job.sel, job.tool, &job.name, path, sr, l, r),
                Ok(Err(e)) => self.toast(format!("{}: {e}", job.name)),
                Err(_) => self.toast(format!("{} failed", job.name)),
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn place_made_clip(&mut self, sel: ClipSel, tool: ClipTool, name: &str, path: PathBuf, sr: u32, l: Vec<f32>, r: Vec<f32>) {
        let bpm = self.project.bpm;
        let beats = (l.len() as f64 / (sr as f64 * 60.0 / bpm)).max(0.0625);
        let sample = SampleRef::new(SampleData::from_buffers(name.to_string(), Some(path.clone()), sr, l, r));
        let audio = AudioClip::new(sample, bpm);
        let Some(old) = self.clip_ref(sel).cloned() else {
            self.toast(format!("The clip is gone; {} is in {}", name, path.display()));
            return;
        };
        if tool == ClipTool::Resample {
            // The clip plays its new sample, as it played the old one.
            if let Some(c) = self.clip_mut(sel) {
                c.content = ClipContent::Audio(audio);
                c.length = beats;
            }
            if let ClipSel::Arr { track, id } = sel {
                if let Some(ac) = self.project.tracks.get_mut(track).and_then(|t| t.arrangement.iter_mut().find(|a| a.id == id)) {
                    ac.offset = 0.0;
                    ac.length = beats;
                }
            }
            self.toast(format!("Resampled into {}", path.display()));
            return;
        }
        let id = self.project.alloc_id();
        let clip = krabton_engine::Clip { id, name: name.to_string(), color: old.color, length: beats, content: ClipContent::Audio(audio), envelopes: Vec::new(), sound: None, groove: None };
        match sel {
            ClipSel::Slot { track, scene } => {
                // The next free slot below it (a new scene when there is none).
                let free = self.project.tracks[track].slots.iter().enumerate().skip(scene + 1).find(|(_, s)| s.is_none()).map(|(i, _)| i);
                let at = match free {
                    Some(i) => i,
                    None => {
                        let at = self.project.scenes.len();
                        self.project.add_scene(at);
                        at
                    }
                };
                if let Some(slot) = self.project.tracks[track].slots.get_mut(at) {
                    *slot = Some(clip);
                }
                self.select_clip(ClipSel::Slot { track, scene: at });
            }
            ClipSel::Arr { track, id: aid } => {
                // On a new audio track, where the clip starts.
                let start = self.project.tracks[track].arrangement.iter().find(|a| a.id == aid).map(|a| a.start).unwrap_or(0.0);
                let ti = self.add_audio_track();
                self.project.tracks[ti].name = name.to_string();
                let new_id = self.project.alloc_id();
                self.project.tracks[ti].arrangement.push(krabton_engine::ArrangementClip::new(new_id, start, beats, clip));
                self.select_clip(ClipSel::Arr { track: ti, id: new_id });
            }
        }
        self.toast(format!("{name}: {}", path.display()));
    }

    /// The box for a stretch of any size.
    pub(crate) fn ui_stretch_dialog(&mut self, ctx: &Context) {
        let Some((sel, mut factor)) = self.stretch_custom else { return };
        let mut open = true;
        let mut go = false;
        egui::Window::new("Stretch Clip").collapsible(false).resizable(false).open(&mut open).show(ctx, |ui| {
            ui.label("How many times as long the new clip is:");
            ui.add(egui::Slider::new(&mut factor, 1.5..=1000.0).logarithmic(true).suffix(" x"));
            let secs = self.clip_ref(sel).map(|c| c.length * 60.0 / self.project.bpm).unwrap_or(0.0) * factor;
            ui.label(egui::RichText::new(format!("about {:.0} s", secs)).small());
            ui.horizontal(|ui| {
                if ui.button("Stretch").clicked() {
                    go = true;
                }
                if ui.button("Cancel").clicked() {
                    self.stretch_custom = None;
                }
            });
        });
        if self.stretch_custom.is_some() {
            self.stretch_custom = if go || !open { None } else { Some((sel, factor)) };
        }
        if go {
            self.start_clip_job(sel, ClipTool::Stretch(factor));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_offline_stretch_writes_a_wav_of_the_right_length() {
        let sr = 48000u32;
        let l: Vec<f32> = (0..sr as usize / 2).map(|i| (i as f32 * 0.05).sin() * 0.4).collect();
        let (sl, sr2) = make(ClipTool::Stretch(4.0), sr, &l, &l, 120.0);
        assert_eq!(sl.len(), l.len() * 4);
        assert_eq!(sr2.len(), sl.len());
        let dir = std::env::temp_dir().join(format!("krabton-stretch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("x4.wav");
        let inter: Vec<f32> = sl.iter().zip(&sr2).flat_map(|(a, b)| [*a, *b]).collect();
        krabton_engine::render::write_wav(&path, sr, &inter).unwrap();
        let back = SampleData::load(&path).unwrap();
        assert_eq!(back.frames(), l.len() * 4);
        let _ = std::fs::remove_dir_all(&dir);
        // Reverse is the clip backwards; a drone is thirty seconds.
        let (rl, _) = make(ClipTool::Reverse, sr, &l, &l, 120.0);
        assert_eq!(rl.first(), l.last());
        let (dl, _) = make(ClipTool::Drone(0.5), sr, &l, &l, 120.0);
        assert_eq!(dl.len(), (DRONE_SECONDS * sr as f64) as usize);
        assert!(dl.iter().all(|v| v.is_finite()));
    }
}
