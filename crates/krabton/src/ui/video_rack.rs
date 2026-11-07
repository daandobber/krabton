//! The Video Rack: a drum rack whose pads are films. A pad plays the film's
//! sound (read by ffmpeg, cut from where the pad starts for as long as it
//! lasts, a sample as any other); the screen in the device shows the picture
//! of the pad struck last, from that same place, for as long as it sounds.
//! Films are dropped on the pads from outside, or loaded one by one or a
//! folder at a time; each pad shows its film's first picture.

use super::inst_screen::InstLive;
use crate::app::App;
use crate::theme::*;
use crate::video::media::{Kind, Media};
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, ColorImage, Id, Rect, RichText, Sense, Stroke, TextureHandle, TextureOptions, Ui};
use krabton_engine::sample::{SampleRegion, SampleRef};
use krabton_engine::{Command, DrumPad, Instrument};
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
    /// Each track's screen: the pad shown, the strike that started it, since when.
    playing: HashMap<u64, (usize, u32, Instant)>,
    screen: HashMap<u64, (TextureHandle, PathBuf, usize)>,
    thumbs: HashMap<(PathBuf, u64), TextureHandle>,
    selected: HashMap<u64, usize>,
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

/// Where in the film the screen is, a while after the strike: it holds the
/// pad's last picture once its sound has ended.
pub(crate) fn film_time(start: f64, length: f64, since: f64) -> f64 {
    start + since.clamp(0.0, length.max(0.0))
}

/// The pads in the order of their notes (C1 first), as the grid shows them.
pub(crate) fn pads_in_order(pads: &[DrumPad]) -> Vec<usize> {
    let mut v: Vec<usize> = (0..pads.len().min(16)).collect();
    v.sort_by_key(|&k| pads[k].note);
    v
}

/// The film a pad shows, if it has one.
fn film_of(pad: &DrumPad) -> Option<&Path> {
    let path = pad.sample.as_ref()?.path.as_deref()?;
    crate::video::media::takes(path).then_some(path)
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
        self.video_media(&path);
    }

    fn video_media(&mut self, path: &Path) -> Option<Arc<Media>> {
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

    /// Films read for pads: onto them as they come.
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
                    self.put_on_pad(l.track, l.pad, &l.path, &whole, l.cut);
                }
                Err(e) => self.toast(format!("Could not read its sound: {e}. Krabton reads films with ffmpeg: the Video can fetch it, {}", crate::video::ffmpeg::install_hint())),
            }
        }
    }

    /// Films from `first` on, one per pad, in the order of the notes.
    fn fill_video_pads(&mut self, track: u64, pads: &[DrumPad], first: usize, files: Vec<PathBuf>) {
        let order = pads_in_order(pads);
        let from = order.iter().position(|&k| k == first).unwrap_or(0);
        for (path, &pad) in files.into_iter().filter(|p| takes(p)).zip(order[from..].iter()) {
            self.load_video_pad(track, pad, path, None);
        }
    }

    /// The Video Rack in the device chain: its screen, its pads, the pad chosen.
    pub(crate) fn video_rack_panel(&mut self, ui: &mut Ui, track: u64, pads: &mut Vec<DrumPad>, live: &InstLive, h: f32) {
        let selected = self.video_rack.selected.get(&track).copied().unwrap_or(0).min(pads.len().saturating_sub(1));
        // What was struck last starts its film again.
        let (struck, strikes) = (live.vals[0].round().max(0.0) as usize, live.vals[1].round() as u32);
        let entry = self.video_rack.playing.entry(track).or_insert((struck, strikes, Instant::now() - std::time::Duration::from_secs(3600)));
        if entry.1 != strikes {
            *entry = (struck, strikes, Instant::now());
        }
        let (shown_pad, _, since) = *entry;
        let since = since.elapsed().as_secs_f64();

        ui.horizontal(|ui| {
            if ui.button("Load folder...").on_hover_text("Films from a folder, one on each pad").clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir).map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect()).unwrap_or_default();
                    files.sort();
                    self.fill_video_pads(track, pads, pads_in_order(pads).first().copied().unwrap_or(0), files);
                }
            }
            if ui.button("Load film...").on_hover_text("A film on the pad chosen (or drop films on the pads)").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("Films", &crate::video::media::EXTENSIONS).pick_file() {
                    self.load_video_pad(track, selected, path, None);
                }
            }
            // The pad chosen: where it starts in its film and how long it is.
            if let Some(pad) = pads.get_mut(selected) {
                ui.label(RichText::new(&pad.name).small().color(crate::skin::ink()));
                if let (Some(s), Some(path)) = (pad.sample.clone(), film_of(pad).map(Path::to_path_buf)) {
                    let (mut start, mut length) = pad_cut(&s);
                    let total = self.video_rack.whole.get(&path).map_or(start + length, |w| w.duration());
                    ui.label(RichText::new("Start").small().color(crate::skin::ink_dim()));
                    let a = ui.add(egui::DragValue::new(&mut start).range(0.0..=total).speed(0.01).suffix(" s").max_decimals(2));
                    ui.label(RichText::new("Length").small().color(crate::skin::ink_dim()));
                    let b = ui.add(egui::DragValue::new(&mut length).range(0.02..=total.max(0.02)).speed(0.01).suffix(" s").max_decimals(2));
                    if a.changed() || b.changed() {
                        self.load_video_pad(track, selected, path, Some((start, length)));
                    }
                    ui.checkbox(&mut pad.gate, "Held").on_hover_text("Plays only while its key is held");
                }
            }
            if !self.video_rack.loading.is_empty() {
                ui.spinner();
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
            }
        });

        let height = (h - 34.0).clamp(110.0, 260.0);
        ui.horizontal_top(|ui| {
            // The screen.
            let (screen, _) = ui.allocate_exact_size(vec2(height * 16.0 / 9.0, height), Sense::hover());
            ui.painter().rect_filled(screen, 3.0, Color32::BLACK);
            let showing = pads.get(shown_pad).filter(|p| film_of(p).is_some()).map(|_| shown_pad).unwrap_or(selected);
            if let Some(pad) = pads.get(showing) {
                if let (Some(path), Some(s)) = (film_of(pad).map(Path::to_path_buf), pad.sample.clone()) {
                    let (start, length) = pad_cut(&s);
                    let t = if showing == shown_pad && strikes > 0 { film_time(start, length, since) } else { start };
                    if let Some(media) = self.video_media(&path) {
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
                        if !media.is_done() || (showing == shown_pad && since < length + 0.2) {
                            ui.ctx().request_repaint();
                        }
                    }
                } else {
                    text(ui.painter(), screen.center(), Align2::CENTER_CENTER, "Drop films on the pads", small(), TEXT_DIM);
                }
            }
            ui.add_space(8.0);
            // The pads, four by four, the lowest note at the bottom left.
            let cell = ((height - 6.0) / 4.0).floor();
            let (grid, _) = ui.allocate_exact_size(vec2(cell * 4.0 * 1.4, cell * 4.0), Sense::hover());
            let order = pads_in_order(pads);
            // Films from outside, hovering or let go over a pad.
            let drag = &self.imports.drag;
            let drop_at = drag.pos;
            let dropped: Vec<PathBuf> = if drag.dropped { drag.files.clone() } else { Vec::new() };
            let hovering = !drag.files.is_empty() && !drag.dropped;
            for (n, &k) in order.iter().enumerate() {
                let (col, row) = (n % 4, 3 - n / 4);
                let r = Rect::from_min_size(grid.min + vec2(col as f32 * cell * 1.4, row as f32 * cell), vec2(cell * 1.4, cell)).shrink(2.0);
                let pad = &pads[k];
                let id = Id::new(("video_pad", track, k));
                let resp = ui.interact(r, id, Sense::click());
                let lit = k == shown_pad && strikes > 0 && since < 0.15;
                let p = ui.painter();
                p.rect_filled(r, 3.0, if lit { Color32::WHITE } else { Color32::from_gray(0x22) });
                if let Some(path) = film_of(pad).map(Path::to_path_buf) {
                    let start = pad.sample.as_ref().map_or(0.0, |s| pad_cut(s).0);
                    if let Some(tex) = self.thumb(ui, &path, start) {
                        let inner = r.shrink(if lit { 3.0 } else { 1.0 });
                        let size = tex.size_vec2();
                        let scale = (inner.width() / size.x).max(inner.height() / size.y);
                        let uv = Rect::from_center_size(pos2(0.5, 0.5), vec2(inner.width() / (size.x * scale), inner.height() / (size.y * scale)));
                        ui.painter().image(tex.id(), inner, uv, Color32::WHITE);
                    }
                }
                let p = ui.painter();
                let label = pad.name.clone();
                p.rect_filled(Rect::from_min_max(pos2(r.left(), r.bottom() - 14.0), r.max), 0.0, Color32::from_black_alpha(160));
                p.with_clip_rect(r).text(pos2(r.left() + 3.0, r.bottom() - 7.0), Align2::LEFT_CENTER, label, small(), Color32::WHITE);
                if k == selected {
                    p.rect_stroke(r, 3.0, Stroke::new(2.0_f32, accent_now()), egui::StrokeKind::Inside);
                }
                let here = drop_at.is_some_and(|q| r.contains(q));
                if hovering && here {
                    p.rect_stroke(r, 3.0, Stroke::new(2.0_f32, Color32::WHITE), egui::StrokeKind::Inside);
                }
                if here && !dropped.is_empty() {
                    let files = dropped.clone();
                    self.fill_video_pads(track, pads, k, files);
                    crate::import::take_file_drop(ui.ctx());
                }
                if resp.clicked() {
                    self.video_rack.selected.insert(track, k);
                    let note = pads[k].note;
                    self.cmd(Command::NoteOn { track, pitch: note, velocity: 0.9 });
                    self.cmd(Command::NoteOff { track, pitch: note });
                }
                if resp.double_clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("Films", &crate::video::media::EXTENSIONS).pick_file() {
                        self.load_video_pad(track, k, path, None);
                    }
                }
                resp.context_menu(|ui| {
                    if ui.button("Load film...").clicked() {
                        ui.close();
                        if let Some(path) = rfd::FileDialog::new().add_filter("Films", &crate::video::media::EXTENSIONS).pick_file() {
                            self.load_video_pad(track, k, path, None);
                        }
                    }
                    if ui.button("Empty").clicked() {
                        ui.close();
                        let mut empty = krabton_engine::dsp::drums::video_kit().swap_remove(0);
                        empty.note = pads[k].note;
                        empty.name = format!("Pad {}", n + 1);
                        pads[k] = empty;
                    }
                });
            }
        });
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
        let media = self.video_media(path)?;
        if media.kind != Kind::Film {
            return None;
        }
        let index = media.frame_at(start)?;
        // Not the first picture yet while its film is still coming in at that point.
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
        assert_eq!(film_time(2.0, 1.0, 0.25), 2.25);
        assert_eq!(film_time(2.0, 1.0, 5.0), 3.0);
        assert_eq!(film_time(2.0, 1.0, -1.0), 2.0);
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
