//! Slice to New MIDI Track (as in Live): right-click an audio clip (or a
//! sample in the browser) and this dialog asks how to cut it: at its hits
//! (with a sensitivity) or on a grid of the beat, at most how many slices,
//! and whether the pads play one-shot or while their notes are held. A
//! picture of the clip shows where the cuts fall. Create makes the track
//! (see `krabton_engine::slicing`).

use crate::app::{App, ClipSel, View};
use crate::theme::*;
use egui::{pos2, vec2, Context, RichText, Sense, Stroke, Ui};
use krabton_engine::analysis::{self, Flux};
use krabton_engine::dsp::drums::MAX_PADS;
use krabton_engine::slicing::{SliceBy, SliceClip, SliceOptions, SlicePlace, SlicePlan};
use krabton_engine::{AudioClip, Clip, ClipContent, SampleRef};
use std::path::Path;
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
        let mut d = Self { source, opts: SliceOptions::default(), audio, length, name, flux: None, pending, plan: None };
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
        self.slicing = Some(SliceDialog::new(SliceSource::Clip { track_id, clip }, audio, length, name));
    }

    /// Opens the dialog for a sample file (from the browser).
    pub(crate) fn open_slice_dialog_for(&mut self, path: &Path) {
        let Some(sample) = self.load_sample(path) else { return };
        // As the clip it would make: its tempo guessed, as long as it is.
        let c = Clip::new_audio(0, sample.clone(), [255, 255, 255], self.project.bpm);
        let ClipContent::Audio(a) = &c.content else { return };
        let (audio, length) = (a.clone(), c.length);
        self.slicing = Some(SliceDialog::new(SliceSource::Sample(sample), audio, length, c.name.clone()));
    }

    pub(crate) fn ui_slice_dialog(&mut self, ctx: &Context) {
        let Some(mut d) = self.slicing.take() else { return };
        let bpb = self.project.beats_per_bar_f();
        let (mut open, mut create, mut cancel) = (true, false, false);
        egui::Window::new("Slice to New MIDI Track").collapsible(false).resizable(false).open(&mut open).show(ctx, |ui| {
            ui.label(RichText::new(&d.name).strong());
            let plan = d.plan(bpb).cloned();
            slice_picture(ui, &d.audio, d.length, plan.as_ref());
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
            });
            if let Some(plan) = &plan {
                let merged = if plan.merged > 0 { format!(" ({} more left out: at most {})", plan.merged, d.opts.max_slices.min(MAX_PADS)) } else { String::new() };
                ui.label(RichText::new(format!("{} slices, on the pads from C1{merged}", plan.len())).small().color(TEXT_DIM));
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
                self.toast(format!("{} slices on {}, from C1{more}", done.slices, self.project.tracks[done.track].name));
            }
            Err(e) => self.toast(e),
        }
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
