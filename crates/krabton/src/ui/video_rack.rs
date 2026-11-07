//! The Video Rack: a drum rack whose pads are films. A pad plays the film's
//! sound (read by ffmpeg, cut from where the pad starts for as long as it
//! lasts, a sample as any other, with all of the Drum Rack's settings); the
//! screen in the device, and the Video, show the picture of the pad struck
//! last, from that same place, for as long as it sounds. Films are dropped on
//! the screen from outside, or loaded one by one or a folder at a time; each
//! pad shows its film's picture.

use crate::app::App;
use crate::theme::*;
use crate::video::media::{Kind, Media};
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, ColorImage, Rect, Sense, TextureHandle, TextureOptions, Ui};
use krabton_engine::dsp::drums::DrumEngine;
use krabton_engine::sample::{SampleRegion, SampleRef};
use krabton_engine::{DrumPad, Instrument};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Instant;

/// How long a pad is at first (seconds): a hit, not a whole film.
const FIRST_LENGTH: f64 = 4.0;
const THUMB_W: usize = 120;

/// A film's sound being read for a pad.
struct Loading {
    track: u64,
    pad: usize,
    path: PathBuf,
    /// Where the pad starts and how long it is (seconds); None: from the start.
    cut: Option<(f64, f64)>,
    rx: Receiver<Result<SampleRef, String>>,
}

#[derive(Default)]
pub(crate) struct VideoRackState {
    media: HashMap<PathBuf, Arc<Media>>,
    /// The whole sound of each film, to cut pads from.
    whole: HashMap<PathBuf, SampleRef>,
    loading: Vec<Loading>,
    /// Each Video Rack's last strike: the pad, the count of strikes, since when.
    struck: HashMap<u64, (usize, u32, Instant)>,
    screen: HashMap<u64, (TextureHandle, PathBuf, usize)>,
    thumbs: HashMap<(PathBuf, u64), TextureHandle>,
}

/// Whether a file can go on a pad: a film, or plain sound.
pub(crate) fn takes(path: &Path) -> bool {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    (crate::video::media::takes(path) && !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif")) || krabton_engine::sample::is_audio_file(path)
}

/// Where a pad starts in its film, and how long it is (seconds).
pub(crate) fn pad_cut(s: &SampleRef) -> (f64, f64) {
    let rate = s.sample_rate.max(1) as f64;
    (s.region.map_or(0.0, |r| r.start as f64 / rate), s.frames() as f64 / rate)
}

/// The part of the film a pad plays, as frames of its sound.
pub(crate) fn region_of(whole: &SampleRef, start: f64, length: f64) -> SampleRegion {
    let rate = whole.sample_rate.max(1) as f64;
    let n = whole.frames() as u64;
    let a = ((start.max(0.0) * rate) as u64).min(n.saturating_sub(1));
    let b = (a + (length.max(0.01) * rate) as u64).min(n).max(a + 1);
    SampleRegion { start: a, end: b, fade_in: 0, fade_out: (rate * 0.005) as u32 }
}

/// How much of its film a pad has played, a while after the strike: at its
/// tune's speed (an octave up, twice as fast), as its sound does.
pub(crate) fn played(pad: &DrumPad, since: f64) -> f64 {
    since.max(0.0) * 2f64.powf(pad.tune as f64 / 12.0)
}

/// Where in the film the screen is: as far along as the sound, backwards when
/// the pad is reversed; it holds the pad's last picture once its sound has ended.
pub(crate) fn film_time(start: f64, length: f64, played: f64, reverse: bool) -> f64 {
    let p = played.clamp(0.0, length.max(0.0));
    if reverse {
        start + length - p
    } else {
        start + p
    }
}

/// The pads in the order of their notes (C1 first), as the grid shows them.
pub(crate) fn pads_in_order(pads: &[DrumPad]) -> Vec<usize> {
    let mut v: Vec<usize> = (0..pads.len().min(16)).collect();
    v.sort_by_key(|&k| pads[k].note);
    v
}

/// The film a pad shows, if it has one.
pub(crate) fn film_of(pad: &DrumPad) -> Option<&Path> {
    let path = pad.sample.as_ref()?.path.as_deref()?;
    crate::video::media::takes(path).then_some(path)
}

/// Of the pads struck (track, pad, seconds since, length), the latest that
/// still sounds: (track, pad, seconds since).
pub(crate) fn latest_strike(struck: &[(u64, usize, f64, f64)]) -> Option<(u64, usize, f64)> {
    struck.iter().filter(|s| s.2 <= s.3).min_by(|a, b| a.2.total_cmp(&b.2)).map(|s| (s.0, s.1, s.2))
}

impl VideoRackState {
    /// How long ago pad `k` of a drum track was struck.
    pub(crate) fn pad_since(&self, track: u64, k: usize) -> Option<f64> {
        self.struck.get(&track).filter(|s| s.0 == k && s.1 > 0).map(|s| s.2.elapsed().as_secs_f64())
    }
}

impl App {
    /// Starts reading a film's sound for a pad (the frames come in by themselves).
    pub(crate) fn load_video_pad(&mut self, track: u64, pad: usize, path: PathBuf, cut: Option<(f64, f64)>) {
        if let Some(whole) = self.video_rack.whole.get(&path).cloned() {
            self.put_on_pad(track, pad, &path, &whole, cut);
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let p = path.clone();
        std::thread::spawn(move || {
            let _ = tx.send(SampleRef::load(&p));
        });
        self.video_rack.loading.push(Loading { track, pad, path: path.clone(), cut, rx });
        self.rack_media(&path);
    }

    pub(crate) fn rack_media(&mut self, path: &Path) -> Option<Arc<Media>> {
        if !crate::video::media::takes(path) {
            return None;
        }
        Some(self.video_rack.media.entry(path.to_path_buf()).or_insert_with(|| Media::open(path)).clone())
    }

    fn put_on_pad(&mut self, track: u64, pad: usize, path: &Path, whole: &SampleRef, cut: Option<(f64, f64)>) {
        let (start, length) = cut.unwrap_or((0.0, FIRST_LENGTH.min(whole.duration())));
        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Film".into());
        let sample = SampleRef::new(whole.region(name.clone(), region_of(whole, start, length)));
        let Some(ti) = self.project.track_index(track) else { return };
        if let Some(Instrument::Drums { pads, .. }) = self.project.tracks[ti].instrument.as_mut() {
            if let Some(p) = pads.get_mut(pad) {
                p.sample = Some(sample);
                p.layers.clear();
                if cut.is_none() {
                    p.name = name;
                }
            }
        }
    }

    /// Every frame: films read for pads go onto them, and each rack's strikes are noted.
    pub(crate) fn poll_video_rack(&mut self) {
        let mut done = Vec::new();
        let mut i = 0;
        while i < self.video_rack.loading.len() {
            match self.video_rack.loading[i].rx.try_recv() {
                Ok(result) => done.push((self.video_rack.loading.remove(i), result)),
                Err(std::sync::mpsc::TryRecvError::Empty) => i += 1,
                Err(_) => {
                    self.video_rack.loading.remove(i);
                }
            }
        }
        for (l, result) in done {
            match result {
                Ok(whole) => {
                    self.video_rack.whole.insert(l.path.clone(), whole.clone());
                    self.waves.keep_whole(&l.path, &whole);
                    self.put_on_pad(l.track, l.pad, &l.path, &whole, l.cut);
                }
                Err(e) => self.toast(format!("Could not read its sound: {e}. Krabton reads films with ffmpeg: the Video can fetch it, {}", crate::video::ffmpeg::install_hint())),
            }
        }
        // What each Video Rack struck (a new strike starts its film again).
        let status = self.status();
        for (ti, t) in self.project.tracks.iter().enumerate() {
            // Every drum track: its waveform's line runs while a pad sounds.
            if !matches!(t.instrument, Some(Instrument::Drums { .. })) {
                continue;
            }
            let Some(ts) = status.tracks.get(ti) else { continue };
            let vals = ts.inst_values();
            let (pad, strikes) = (vals[0].round().max(0.0) as usize, vals[1].round() as u32);
            let e = self.video_rack.struck.entry(t.id).or_insert((pad, strikes, Instant::now() - std::time::Duration::from_secs(3600)));
            if e.1 != strikes {
                *e = (pad, strikes, Instant::now());
            }
        }
    }

    /// What the Video shows of the racks now: the film of the pad struck last,
    /// while it sounds, and where in it.
    pub(crate) fn rack_frame(&mut self) -> Option<(Arc<Media>, f64)> {
        let mut struck = Vec::new();
        for t in &self.project.tracks {
            let Some(Instrument::Drums { engine: DrumEngine::Video, pads, .. }) = &t.instrument else { continue };
            let Some(&(pad, strikes, at)) = self.video_rack.struck.get(&t.id) else { continue };
            let Some(p) = pads.get(pad).filter(|_| strikes > 0) else { continue };
            if let (Some(_), Some(s)) = (film_of(p), &p.sample) {
                struck.push((t.id, pad, played(p, at.elapsed().as_secs_f64()), pad_cut(s).1));
            }
        }
        let (track, pad, since) = latest_strike(&struck)?;
        let ti = self.project.track_index(track)?;
        let Some(Instrument::Drums { pads, .. }) = &self.project.tracks[ti].instrument else { return None };
        let p = pads.get(pad)?;
        let path = film_of(p)?.to_path_buf();
        let (start, length) = pad_cut(p.sample.as_ref()?);
        let reverse = p.reverse;
        let media = self.rack_media(&path)?;
        Some((media, film_time(start, length, since, reverse)))
    }

    /// The rack's screen in the device, its buttons over it, and the pad
    /// chosen: where it starts in its film and how long it is.
    pub(crate) fn video_rack_screen(&mut self, ui: &mut Ui, track: u64, pads: &mut [DrumPad], height: f32) {
        let selected = self.sel_pad.min(pads.len().saturating_sub(1));
        let (shown_pad, strikes, since) = self.video_rack.struck.get(&track).map_or((selected, 0, 1e9), |s| (s.0, s.1, s.2.elapsed().as_secs_f64()));
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui.small_button("Load folder...").on_hover_text("Films from a folder, one on each pad from the one chosen").clicked() {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        let mut files: Vec<PathBuf> = std::fs::read_dir(&dir).map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect()).unwrap_or_default();
                        files.sort();
                        self.fill_video_pads(track, pads, selected, files);
                    }
                }
                if ui.small_button("Load film...").on_hover_text("A film on the pad chosen (or drop films on the screen)").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("Films", &crate::video::media::EXTENSIONS).pick_file() {
                        self.load_video_pad(track, selected, path, None);
                    }
                }
                if !self.video_rack.loading.is_empty() {
                    ui.spinner();
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                }
            });
            // The screen.
            let h = (height - 26.0).max(80.0);
            let (screen, _) = ui.allocate_exact_size(vec2(h * 16.0 / 9.0, h), Sense::hover());
            ui.painter().rect_filled(screen, 3.0, Color32::BLACK);
            let showing = pads.get(shown_pad).filter(|p| film_of(p).is_some() && strikes > 0).map(|_| shown_pad).unwrap_or(selected);
            match pads.get(showing).and_then(|p| Some((film_of(p)?.to_path_buf(), p.sample.clone()?))) {
                Some((path, s)) => {
                    let (start, length) = pad_cut(&s);
                    let along = pads.get(showing).map_or(since, |p| played(p, since));
                    let reverse = pads.get(showing).is_some_and(|p| p.reverse);
                    let t = if showing == shown_pad && strikes > 0 { film_time(start, length, along, reverse) } else if reverse { start + length } else { start };
                    if let Some(media) = self.rack_media(&path) {
                        if let Some(index) = media.frame_at(t) {
                            let stale = self.video_rack.screen.get(&track).is_none_or(|(_, p, k)| *p != path || *k != index);
                            if stale {
                                if let Some(img) = media.image(index) {
                                    self.show_on_screen(ui, track, &path, index, img);
                                }
                            }
                        }
                        if let Some((tex, _, _)) = self.video_rack.screen.get(&track) {
                            let size = tex.size_vec2();
                            let scale = (screen.width() / size.x).min(screen.height() / size.y);
                            let r = Rect::from_center_size(screen.center(), size * scale);
                            ui.painter().image(tex.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                        }
                        if !media.is_done() || (showing == shown_pad && along < length + 0.2) {
                            ui.ctx().request_repaint();
                        }
                    }
                }
                None => {
                    text(ui.painter(), screen.center(), Align2::CENTER_CENTER, "Drop films here", small(), TEXT_DIM);
                }
            }
            // Films from outside, let go over the screen: onto the pads from the one chosen.
            let drag = &self.imports.drag;
            let here = drag.pos.is_some_and(|q| screen.contains(q));
            if here && !drag.files.is_empty() {
                ui.painter().rect_stroke(screen, 3.0, egui::Stroke::new(2.0_f32, accent_now()), egui::StrokeKind::Inside);
            }
            if here && drag.dropped {
                let files = drag.files.clone();
                self.fill_video_pads(track, pads, selected, files);
                crate::import::take_file_drop(ui.ctx());
            }
        });
    }

    /// Films from pad `first` on, one per pad, in the order of the notes.
    fn fill_video_pads(&mut self, track: u64, pads: &[DrumPad], first: usize, files: Vec<PathBuf>) {
        let order = pads_in_order(pads);
        let from = order.iter().position(|&k| k == first).unwrap_or(0);
        for (path, &pad) in files.into_iter().filter(|p| takes(p)).zip(order[from..].iter()) {
            self.load_video_pad(track, pad, path, None);
        }
    }

    /// A pad of the grid: its film's picture where it starts, under its name.
    pub(crate) fn paint_video_thumb(&mut self, ui: &Ui, r: Rect, pad: &DrumPad) {
        let Some(path) = film_of(pad).map(Path::to_path_buf) else { return };
        let start = pad.sample.as_ref().map_or(0.0, |s| pad_cut(s).0);
        let Some(tex) = self.thumb(ui, &path, start) else { return };
        let inner = r.shrink(1.0);
        let size = tex.size_vec2();
        let scale = (inner.width() / size.x).max(inner.height() / size.y);
        let uv = Rect::from_center_size(pos2(0.5, 0.5), vec2(inner.width() / (size.x * scale), inner.height() / (size.y * scale)));
        ui.painter().image(tex.id(), inner, uv, Color32::from_white_alpha(150));
    }

    fn show_on_screen(&mut self, ui: &Ui, track: u64, path: &Path, index: usize, img: ColorImage) {
        match self.video_rack.screen.get_mut(&track) {
            Some((tex, p, k)) => {
                tex.set(img, TextureOptions::LINEAR);
                *p = path.to_path_buf();
                *k = index;
            }
            None => {
                let tex = ui.ctx().load_texture(format!("video_rack_{track}"), img, TextureOptions::LINEAR);
                self.video_rack.screen.insert(track, (tex, path.to_path_buf(), index));
            }
        }
    }

    /// A pad's little picture: its film where the pad starts.
    fn thumb(&mut self, ui: &Ui, path: &Path, start: f64) -> Option<TextureHandle> {
        let key = (path.to_path_buf(), (start * 100.0) as u64);
        if let Some(t) = self.video_rack.thumbs.get(&key) {
            return Some(t.clone());
        }
        let media = self.rack_media(path)?;
        if media.kind != Kind::Film {
            return None;
        }
        let index = media.frame_at(start)?;
        // Not yet while its film is still coming in at that point.
        if !media.is_done() && media.frame_count() <= index + 1 {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
            return None;
        }
        let img = media.thumbnail_of(index, THUMB_W)?;
        let tex = ui.ctx().load_texture(format!("video_thumb_{}_{}", path.display(), key.1), img, TextureOptions::LINEAR);
        self.video_rack.thumbs.insert(key, tex.clone());
        Some(tex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use krabton_engine::sample::SampleData;

    fn sound(seconds: f64) -> SampleRef {
        let n = (seconds * 1000.0) as usize;
        SampleRef::new(SampleData::from_buffers("film", Some(PathBuf::from("film.mp4")), 1000, vec![0.1; n], vec![0.1; n]))
    }

    #[test]
    fn a_pad_is_a_part_of_its_film_and_says_where() {
        let whole = sound(10.0);
        let part = SampleRef::new(whole.region("hit", region_of(&whole, 2.5, 1.5)));
        let (start, length) = pad_cut(&part);
        assert!((start - 2.5).abs() < 1e-9 && (length - 1.5).abs() < 1e-9, "{start} {length}");
        // Past the end: what there is of it.
        let r = region_of(&whole, 9.5, 4.0);
        assert_eq!((r.start, r.end), (9500, 10000));
    }

    #[test]
    fn the_screen_runs_with_the_sound_and_holds_its_last_picture() {
        assert_eq!(film_time(2.0, 1.0, 0.25, false), 2.25);
        assert_eq!(film_time(2.0, 1.0, 5.0, false), 3.0);
        assert_eq!(film_time(2.0, 1.0, -1.0, false), 2.0);
        assert_eq!(film_time(2.0, 1.0, 0.25, true), 2.75, "reversed: from its end");
        // Tuned an octave up it runs twice as fast, an octave down half.
        let mut pad = krabton_engine::dsp::drums::video_kit().remove(0);
        pad.tune = 12.0;
        assert!((played(&pad, 0.5) - 1.0).abs() < 1e-9);
        pad.tune = -12.0;
        assert!((played(&pad, 0.5) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn the_video_shows_the_latest_strike_that_still_sounds() {
        // (track, pad, since, length)
        let struck = [(1, 0, 0.5, 2.0), (2, 3, 0.1, 1.0), (3, 1, 0.05, 0.02)];
        assert_eq!(latest_strike(&struck), Some((2, 3, 0.1)), "the third has ended");
        assert_eq!(latest_strike(&[(1, 0, 3.0, 2.0)]), None, "all ended: the Video's own clip again");
    }

    #[test]
    fn films_and_sounds_go_on_pads_and_pictures_do_not() {
        assert!(takes(Path::new("a.mp4")) && takes(Path::new("b.MOV")) && takes(Path::new("c.wav")));
        assert!(!takes(Path::new("d.png")) && !takes(Path::new("e.txt")));
    }

    #[test]
    fn the_pads_are_in_the_order_of_their_notes() {
        let mut pads = krabton_engine::dsp::drums::video_kit();
        assert_eq!(pads.len(), 16);
        assert!(pads.iter().all(|p| p.sample.is_some()), "silent, not a drum");
        pads.swap(0, 5);
        let order = pads_in_order(&pads);
        assert_eq!(pads[order[0]].note, 36);
        assert_eq!(pads[order[15]].note, 51);
    }
}
