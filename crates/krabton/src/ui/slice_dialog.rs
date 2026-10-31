//! Slice to New MIDI Track (as in Live): right-click an audio clip (or a
//! sample in the browser) and this dialog asks how to cut it: at its hits
//! (with a sensitivity) or on a grid of the beat, at most how many slices,
//! and whether the pads play one-shot or while their notes are held. A
//! picture of the clip shows where the cuts fall. Create makes the track
//! (see `krabton_engine::slicing`).
//!
//! Slice to Drum Rack is the same dialog for a film (right-click it in the
//! Video's bank, or slice the sound of one): *Picture too* puts the film in
//! the Video cut at the same places, so the new track's notes play its
//! picture with its sound. A strip of the film's frames at the cuts shows
//! under the sound.

use crate::app::{App, ClipSel, View};
use crate::theme::*;
use crate::video::media::Media;
use egui::{pos2, vec2, Color32, Context, Rect, RichText, Sense, Stroke, Ui};
use krabton_engine::analysis::{self, Flux};
use krabton_engine::dsp::drums::MAX_PADS;
use krabton_engine::slicing::{SliceBy, SliceClip, SliceOptions, SlicePlace, SlicePlan};
use krabton_engine::video::is_film;
use krabton_engine::{AudioClip, Clip, ClipContent, SampleRef};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::Arc;

/// What is sliced.
pub enum SliceSource {
    /// A clip of a track (the track by id, in case tracks move meanwhile).
    Clip { track_id: u64, clip: SliceClip },
    /// A sample from the browser: onto a new track.
    Sample(SampleRef),
}

pub struct SliceDialog {
    pub source: SliceSource,
    pub opts: SliceOptions,
    /// The clip's sound and length as it was when the dialog opened.
    audio: AudioClip,
    length: f64,
    name: String,
    /// The sample's flux, once its thread has heard it.
    flux: Option<Arc<Flux>>,
    pending: Option<Receiver<Flux>>,
    /// The cuts for the options last shown.
    plan: Option<(SliceOptions, SlicePlan)>,
    /// The film the sound is of, when it is one (its picture can come too).
    film: Option<PathBuf>,
    /// The film read for its frames at the cuts (the Video's own when it
    /// has it, else read here: `own`), and those frames, by frame.
    media: Option<Arc<Media>>,
    own: bool,
    thumbs: HashMap<usize, egui::TextureHandle>,
}

impl SliceDialog {
    fn new(source: SliceSource, audio: AudioClip, length: f64, name: String) -> Self {
        // Listening for the hits takes a moment on a long sample: on a thread.
        let (tx, rx) = std::sync::mpsc::channel();
        let sample = audio.sample.clone();
        let pending = std::thread::Builder::new()
            .name("slice analysis".into())
            .spawn(move || {
                let _ = tx.send(analysis::spectral_flux(&sample));
            })
            .ok()
            .map(|_| rx);
        // The sound of a film: its picture comes too, unless asked not to.
        let film = audio.sample.path.clone().filter(|p| is_film(p));
        let opts = SliceOptions { picture: film.is_some(), ..Default::default() };
        let mut d = Self { source, opts, audio, length, name, flux: None, pending, plan: None, film, media: None, own: false, thumbs: HashMap::new() };
        if d.pending.is_none() {
            d.flux = Some(Arc::new(analysis::spectral_flux(&d.audio.sample)));
        }
        d
    }

    /// The flux, waiting for its thread when `wait`.
    fn flux(&mut self, wait: bool) -> Option<Arc<Flux>> {
        if self.flux.is_none() {
            if let Some(rx) = &self.pending {
                let got = if wait { rx.recv().ok() } else { rx.try_recv().ok() };
                if let Some(f) = got {
                    self.flux = Some(Arc::new(f));
                    self.pending = None;
                } else if wait {
                    // The thread went away: hear it here.
                    self.flux = Some(Arc::new(analysis::spectral_flux(&self.audio.sample)));
                    self.pending = None;
                }
            }
        }
        self.flux.clone()
    }

    /// The cuts for the options now (made again when they change).
    fn plan(&mut self, beats_per_bar: f64) -> Option<&SlicePlan> {
        let flux = self.flux(false)?;
        if self.plan.as_ref().is_none_or(|(o, _)| *o != self.opts) {
            self.plan = Some((self.opts, SlicePlan::new(&self.audio, self.length, &self.opts, beats_per_bar, &flux)));
        }
        self.plan.as_ref().map(|p| &p.1)
    }

    /// The film's frame at each cut (None where it is not read yet), as
    /// small pictures made when first needed.
    fn film_frames(&mut self, ctx: &Context, plan: &SlicePlan) -> Vec<Option<egui::TextureHandle>> {
        let Some(m) = self.media.clone() else { return Vec::new() };
        if !m.is_done() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        let cues = plan.cues(&self.audio.sample, false, None);
        let last = m.frame_count();
        cues.times
            .iter()
            .map(|&t| {
                let i = m.frame_at(t)?;
                // Still being read up to there: wait for it.
                if i + 1 >= last && !m.is_done() {
                    return None;
                }
                if !self.thumbs.contains_key(&i) {
                    let img = m.thumbnail_of(i, 120)?;
                    self.thumbs.insert(i, ctx.load_texture(format!("slice_frame_{i}"), img, egui::TextureOptions::LINEAR));
                }
                self.thumbs.get(&i).cloned()
            })
            .collect()
    }

    /// Lets go of the film read only for this dialog.
    fn close(self) {
        if let (true, Some(m)) = (self.own, &self.media) {
            m.cancel();
        }
    }
}

impl App {
    /// Opens the dialog for an audio clip.
    pub(crate) fn open_slice_dialog(&mut self, sel: ClipSel) {
        let Some(track_id) = self.project.tracks.get(sel.track()).map(|t| t.id) else { return };
        let Some(c) = self.clip_ref(sel) else { return };
        let ClipContent::Audio(a) = &c.content else { return };
        let (audio, length, name) = (a.clone(), c.length, c.name.clone());
        let clip = match sel {
            ClipSel::Slot { scene, .. } => SliceClip::Slot(scene),
            ClipSel::Arr { id, .. } => SliceClip::Arrangement(id),
        };
        let d = SliceDialog::new(SliceSource::Clip { track_id, clip }, audio, length, name);
        self.slicing = Some(self.with_film(d));
    }

    /// Opens the dialog for a sample file (from the browser), or a film.
    pub(crate) fn open_slice_dialog_for(&mut self, path: &Path) {
        let sample = if is_film(path) { self.film_sound(path) } else { self.load_sample(path) };
        let Some(sample) = sample else { return };
        // As the clip it would make: its tempo guessed, as long as it is.
        let c = Clip::new_audio(0, sample.clone(), [255, 255, 255], self.project.bpm);
        let ClipContent::Audio(a) = &c.content else { return };
        let (audio, length) = (a.clone(), c.length);
        let d = SliceDialog::new(SliceSource::Sample(sample), audio, length, c.name.clone());
        self.slicing = Some(self.with_film(d));
    }

    /// Slice to Drum Rack for a film (from the Video's bank): its sound,
    /// read with ffmpeg, in the dialog with *Picture too* on. Without
    /// ffmpeg the Video says so and offers to fetch it.
    pub(crate) fn open_film_slice_dialog(&mut self, path: &Path) {
        if crate::video::ffmpeg::find().is_none() {
            self.video_window = true;
            if let Some(i) = self.project.video.clips.iter().position(|c| c.path == path) {
                self.video_pick(i);
            }
            self.toast("Films need ffmpeg: the Video's Get ffmpeg fetches it");
            return;
        }
        self.open_slice_dialog_for(path);
    }

    /// A film's sound (read once, then kept), through ffmpeg.
    fn film_sound(&mut self, path: &Path) -> Option<SampleRef> {
        if let Some(s) = self.cached_sample(path) {
            return Some(s);
        }
        match krabton_engine::sample::decode_with_external(path) {
            Ok(s) => {
                let s = SampleRef::new(s);
                self.remember_sample(path, &s);
                Some(s)
            }
            Err(e) => {
                self.toast(format!("Could not read the film's sound: {e}"));
                None
            }
        }
    }

    /// A film's frames for the dialog: the Video's when it reads it already.
    fn with_film(&self, mut d: SliceDialog) -> SliceDialog {
        if let Some(path) = &d.film {
            match self.video.media.get(path) {
                Some(m) => d.media = Some(m.clone()),
                None => {
                    d.media = Some(Media::open(path));
                    d.own = true;
                }
            }
        }
        d
    }

    pub(crate) fn ui_slice_dialog(&mut self, ctx: &Context) {
        let Some(mut d) = self.slicing.take() else { return };
        let bpb = self.project.beats_per_bar_f();
        let (mut open, mut create, mut cancel) = (true, false, false);
        let title = if d.film.is_some() { "Slice to Drum Rack" } else { "Slice to New MIDI Track" };
        egui::Window::new(title).id(egui::Id::new("slice_dialog")).collapsible(false).resizable(false).open(&mut open).show(ctx, |ui| {
            ui.label(RichText::new(&d.name).strong());
            let plan = d.plan(bpb).cloned();
            slice_picture(ui, &d.audio, d.length, plan.as_ref());
            if let (Some(plan), true) = (&plan, d.film.is_some()) {
                let frames = d.film_frames(ctx, plan);
                film_strip(ui, plan, &frames);
            }
            if plan.is_none() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new("Listening for the hits...").small().color(TEXT_DIM));
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            }
            egui::Grid::new("slice_opts").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Slicing");
                egui::ComboBox::from_id_salt("slice_by").width(120.0).selected_text(d.opts.by.name()).show_ui(ui, |ui| {
                    for by in SliceBy::ALL {
                        ui.selectable_value(&mut d.opts.by, by, by.name());
                    }
                })
                .response
                .on_hover_text("At the hits of the sound, or on a grid of the beat (by the clip's own tempo)");
                ui.end_row();
                ui.label("Sensitivity");
                let mut pct = d.opts.sensitivity * 100.0;
                let resp = ui.add_enabled(d.opts.by == SliceBy::Transients, egui::Slider::new(&mut pct, 0.0..=100.0).suffix(" %").fixed_decimals(0));
                if resp.on_hover_text("How soft a hit still gets a slice of its own").changed() {
                    d.opts.sensitivity = pct / 100.0;
                }
                ui.end_row();
                ui.label("Max slices");
                ui.add(egui::DragValue::new(&mut d.opts.max_slices).range(1..=MAX_PADS)).on_hover_text(format!("A Drum Rack has {MAX_PADS} pads: past that, the weakest cuts are left out (their slices join the ones before them)"));
                ui.end_row();
                ui.label("Preset");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut d.opts.gate, false, "One-Shot").on_hover_text("Every slice plays to its end, however short its note");
                    ui.radio_value(&mut d.opts.gate, true, "Gate").on_hover_text("A slice plays while its note is held: shorter notes cut it short");
                });
                ui.end_row();
                if d.film.is_some() {
                    ui.label("Film");
                    ui.checkbox(&mut d.opts.picture, "Picture too").on_hover_text("The film into the Video, cut at the same places: the new track's notes show each slice's piece of film while they play its sound (as the Preset says: to its end, or while the note is held)");
                    ui.end_row();
                }
            });
            if let Some(plan) = &plan {
                let merged = if plan.merged > 0 { format!(" ({} more left out: at most {})", plan.merged, d.opts.max_slices.min(MAX_PADS)) } else { String::new() };
                let picture = if d.film.is_some() && d.opts.picture { ", the film cut alike in the Video" } else { "" };
                ui.label(RichText::new(format!("{} slices, on the pads from C1{merged}{picture}", plan.len())).small().color(TEXT_DIM));
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(plan.is_some(), egui::Button::new(RichText::new("Create").strong())).on_hover_text("A new MIDI track with a Drum Rack of the slices and a clip that plays them").clicked() {
                    create = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
        if create {
            self.slice_create(d);
        } else if open && !cancel {
            self.slicing = Some(d);
        } else {
            d.close();
        }
    }

    /// Makes the track the dialog describes, and picks its clip and devices.
    pub(crate) fn slice_create(&mut self, mut d: SliceDialog) {
        let flux = d.flux(true);
        let flux = flux.as_deref();
        let result = match &d.source {
            SliceSource::Clip { track_id, clip } => match self.project.tracks.iter().position(|t| t.id == *track_id) {
                Some(t) => self.project.slice_to_midi_track(t, *clip, &d.opts, flux),
                None => Err("That clip's track is gone".into()),
            },
            SliceSource::Sample(s) => {
                let place = if self.view == View::Arrangement {
                    let bpb = self.project.beats_per_bar_f();
                    let bar = (self.engine.status.song_pos.load().max(0.0) / bpb).floor() * bpb;
                    SlicePlace::Arrangement { start: bar, length: 0.0, offset: 0.0 }
                } else {
                    SlicePlace::Slot(self.sel_scene)
                };
                self.project.slice_sample_to_midi_track(s.clone(), place, &d.opts, flux)
            }
        };
        match result {
            Ok(done) => {
                let film = d.film.clone().filter(|_| d.opts.picture);
                self.slice_picture_set(&mut d, film.as_deref(), done.picture);
                let t = &self.project.tracks[done.track];
                let sel = match t.slots.iter().position(|s| s.is_some()) {
                    Some(scene) => Some(ClipSel::Slot { track: done.track, scene }),
                    None => t.arrangement.first().map(|a| ClipSel::Arr { track: done.track, id: a.id }),
                };
                if let Some(sel) = sel {
                    self.select_clip(sel);
                }
                self.show_devices();
                let more = if done.merged > 0 { format!(" ({} more left out)", done.merged) } else { String::new() };
                let picture = match (&film, done.picture) {
                    (Some(_), Some(_)) => ", the film cut alike in the Video",
                    (Some(_), None) => " (the Video's bank is full: no picture)",
                    _ => "",
                };
                self.toast(format!("{} slices on {}, from C1{more}{picture}", done.slices, self.project.tracks[done.track].name));
            }
            Err(e) => {
                self.toast(e);
                d.close();
            }
        }
    }

    /// After Create with *Picture too*: the film (cued in the bank by the
    /// slicing) plays in the Video, read once, and the Video opens.
    fn slice_picture_set(&mut self, d: &mut SliceDialog, film: Option<&Path>, at: Option<usize>) {
        let (Some(path), Some(i)) = (film, at) else {
            if let (true, Some(m)) = (d.own, d.media.take()) {
                m.cancel();
            }
            return;
        };
        if let Some(m) = d.media.take() {
            self.video.media.entry(path.to_path_buf()).or_insert(m);
        }
        // The first clip in the bank: the links listen to the drums for the kicks (as `video_add`).
        if self.project.video.clips.len() == 1 {
            let mut links = std::mem::take(&mut self.project.video.links);
            self.video_fit_links(&mut links);
            self.project.video.links = links;
        }
        self.video_pick(i);
        self.video.playhead.reset();
        self.video_window = true;
    }
}

/// The film's frame at every cut, each over its slice (as wide as it is
/// under the sound), a line between them.
fn film_strip(ui: &mut Ui, plan: &SlicePlan, frames: &[Option<egui::TextureHandle>]) {
    let (r, _) = ui.allocate_exact_size(vec2(420.0, 46.0), Sense::hover());
    let p = ui.painter().with_clip_rect(r);
    p.rect_filled(r, 2.0, Color32::BLACK);
    if plan.to <= plan.from {
        return;
    }
    let x_of = |f: usize| r.left() + ((f - plan.from) as f64 / (plan.to - plan.from) as f64) as f32 * r.width();
    for (k, &f) in plan.starts.iter().enumerate() {
        let cell = Rect::from_min_max(pos2(x_of(f), r.top()), pos2(x_of(plan.end_of(k)), r.bottom()));
        match frames.get(k).and_then(|t| t.as_ref()) {
            Some(t) => {
                // Its middle, as much as fits the slice's width.
                let [w, h] = t.size();
                let (pic, cell_aspect) = (w as f32 / h.max(1) as f32, cell.width() / cell.height().max(1.0));
                let u = (cell_aspect / pic).min(1.0);
                let uv = Rect::from_min_max(pos2(0.5 - u / 2.0, 0.0), pos2(0.5 + u / 2.0, 1.0));
                let shown = if u < 1.0 { cell } else { Rect::from_min_size(cell.min, vec2(cell.height() * pic, cell.height())) };
                p.image(t.id(), shown, uv, Color32::WHITE);
            }
            None => {
                p.text(cell.center(), egui::Align2::CENTER_CENTER, "...", small(), TEXT_DIM);
            }
        }
        p.line_segment([pos2(cell.left(), r.top()), pos2(cell.left(), r.bottom())], Stroke::new(1.0_f32, accent_now()));
    }
}

/// The clip's waveform over what is sliced, a line at every cut.
fn slice_picture(ui: &mut Ui, a: &AudioClip, length: f64, plan: Option<&SlicePlan>) {
    let (r, _) = ui.allocate_exact_size(vec2(420.0, 80.0), Sense::hover());
    let p = ui.painter().with_clip_rect(r);
    p.rect_filled(r, 2.0, BG);
    let s = &a.sample;
    let (from, to) = plan.map_or_else(|| krabton_engine::slicing::clip_region(a, length), |pl| (pl.from, pl.to));
    if to <= from {
        return;
    }
    let per_px = (to - from) as f64 / r.width() as f64;
    let (mid, half) = (r.center().y, r.height() * 0.45);
    let block = krabton_engine::sample::PEAK_BLOCK;
    for x in 0..r.width() as usize {
        let (f0, f1) = (from + (x as f64 * per_px) as usize, from + ((x + 1) as f64 * per_px) as usize);
        let (i0, i1) = (f0 / block, (f1 / block).max(f0 / block + 1).min(s.peaks.len()));
        let (mut mn, mut mx) = (0.0f32, 0.0f32);
        for &(lo, hi) in s.peaks.get(i0..i1).unwrap_or(&[]) {
            mn = mn.min(lo);
            mx = mx.max(hi);
        }
        let xx = r.left() + x as f32 + 0.5;
        p.line_segment([pos2(xx, mid - mx * half), pos2(xx, mid - mn * half + 0.5)], Stroke::new(1.0_f32, TEXT_DIM));
    }
    if let Some(plan) = plan {
        for (k, &f) in plan.starts.iter().enumerate() {
            let x = r.left() + ((f - from) as f64 / per_px) as f32;
            p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], Stroke::new(1.0_f32, accent_now()));
            p.text(pos2(x + 2.0, r.top() + 1.0), egui::Align2::LEFT_TOP, (k + 1).to_string(), small(), accent_now());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use krabton_engine::sample::SampleData;
    use krabton_engine::Project;

    /// Four thumps a second for two seconds, as the sound of `path`.
    fn sound(path: &str) -> SampleRef {
        let sr = 48_000usize;
        let x: Vec<f32> = (0..2 * sr).map(|i| if i % (sr / 4) < 2000 { (i as f32 * 0.02).sin() * (1.0 - (i % (sr / 4)) as f32 / 2000.0) } else { 0.0 }).collect();
        SampleRef::new(SampleData::from_buffers("Zoo", Some(path.into()), sr as u32, x.clone(), x))
    }

    fn dialog(s: &SampleRef) -> SliceDialog {
        let c = Clip::new_audio(0, s.clone(), [255, 255, 255], 120.0);
        let ClipContent::Audio(a) = &c.content else { panic!("an audio clip") };
        SliceDialog::new(SliceSource::Sample(s.clone()), a.clone(), c.length, c.name.clone())
    }

    #[test]
    fn a_films_sound_slices_with_its_picture_as_create_does() {
        // A sound file: no Picture too.
        let d = dialog(&sound("loops/zoo.wav"));
        assert!(d.film.is_none() && !d.opts.picture);
        // A film's: on, and Create's way makes the track and cues the film alike.
        let film = sound("films/zoo.mp4");
        let mut d = dialog(&film);
        assert_eq!(d.film.as_deref(), Some(Path::new("films/zoo.mp4")));
        assert!(d.opts.picture);
        d.opts.by = SliceBy::Quarter;
        let flux = d.flux(true);
        let plan = d.plan(4.0).cloned().expect("a plan");
        let mut p = Project::blank();
        p.bpm = 120.0;
        let done = p.slice_sample_to_midi_track(film, SlicePlace::Slot(0), &d.opts, flux.as_deref()).unwrap();
        assert_eq!(done.slices, plan.len());
        let clip = &p.video.clips[done.picture.expect("cued")];
        let cues = clip.cues.as_ref().unwrap();
        assert_eq!(cues.times, (0..4).map(|k| k as f64 * 0.5).collect::<Vec<_>>());
        assert_eq!(cues.track, Some(p.tracks[done.track].id));
        assert_eq!(p.tracks[done.track].name, "Zoo Slices");
        // Picture too off: the sound alone.
        d.opts.picture = false;
        let mut q = Project::blank();
        let done = q.slice_sample_to_midi_track(sound("films/zoo.mp4"), SlicePlace::Slot(0), &d.opts, None).unwrap();
        assert_eq!(done.picture, None);
        assert!(q.video.clips.is_empty());
    }
}
