//! Convert to MIDI and grooves (as in Live): right-click an audio clip for
//! Convert Melody, Harmony or Drums to New MIDI Track (heard on a thread
//! of its own, a window showing how far it is; see
//! `krabton_engine::to_midi`), or any clip for Extract Groove, which puts
//! its feel in the Groove Pool. A MIDI clip's view picks a groove and how
//! much of it (timing, velocity, random), and Commit writes it into the
//! notes (see `krabton_engine::groove`).

use crate::app::{App, ClipSel};
use crate::theme::*;
use egui::{Context, RichText, Ui};
use krabton_engine::groove::{self, ClipGroove};
use krabton_engine::slicing::{SliceClip, SlicePlace};
use krabton_engine::to_midi::{self, Convert, ConvertOptions};
use krabton_engine::{Clip, ClipContent, Note};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;

/// The grids Convert can put the notes on (beats; None: where they are heard).
const QUANTIZE: [(Option<f64>, &str); 3] = [(None, "Off"), (Some(0.25), "1/16"), (Some(0.5), "1/8")];

/// What converting is doing and how it was asked.
pub struct ConvertState {
    /// The grid the converted notes go on.
    pub quantize: Option<f64>,
    jobs: Vec<Job>,
}

impl Default for ConvertState {
    fn default() -> Self {
        Self { quantize: Some(0.25), jobs: Vec::new() }
    }
}

/// A conversion being heard on its thread.
struct Job {
    kind: Convert,
    /// The audio clip's track (by id, in case tracks move meanwhile).
    track_id: u64,
    source: Clip,
    place: SlicePlace,
    /// How far it is, in thousandths.
    progress: Arc<AtomicU32>,
    done: Receiver<Result<(Vec<Note>, f64), String>>,
}

impl App {
    /// The Convert and Extract Groove entries of a clip's right-click menu.
    pub(crate) fn convert_menu(&mut self, ui: &mut Ui, sel: ClipSel) {
        let Some(clip) = self.clip_ref(sel) else { return };
        if !clip.is_midi() {
            for kind in Convert::ALL {
                let tip = match kind {
                    Convert::Melody => "Hear the tune of one voice (a singer, a bass, a lead) and play it as notes on a new MIDI track",
                    Convert::Harmony => "Hear the chords (of a piano, a guitar, a pad) and play them as notes on a new MIDI track",
                    Convert::Drums => "Hear the kicks, snares and hats of a beat and play them on a new track with a drum machine",
                };
                if ui.button(format!("Convert {} to New MIDI Track", kind.name())).on_hover_text(tip).clicked() {
                    self.start_convert(sel, kind);
                    ui.close();
                }
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new("Quantize").small().color(TEXT_DIM));
                for (q, name) in QUANTIZE {
                    ui.selectable_value(&mut self.convert.quantize, q, name).on_hover_text("The converted notes start and end on this grid (Off: where they are heard)");
                }
            });
        }
        if ui.button("Extract Groove").on_hover_text("Its feel (how late or early its sixteenths fall, how loud) into the Groove Pool, for MIDI clips to play with").clicked() {
            self.extract_groove(sel);
            ui.close();
        }
    }

    /// Starts hearing the audio clip at `sel` as `kind`, on a thread.
    pub(crate) fn start_convert(&mut self, sel: ClipSel, kind: Convert) {
        let track = sel.track();
        let Some(track_id) = self.project.tracks.get(track).map(|t| t.id) else { return };
        let clip = match sel {
            ClipSel::Slot { scene, .. } => SliceClip::Slot(scene),
            ClipSel::Arr { id, .. } => SliceClip::Arrangement(id),
        };
        let (source, place) = match self.project.convert_source(track, clip) {
            Ok(s) => s,
            Err(e) => {
                self.toast(e);
                return;
            }
        };
        let ClipContent::Audio(a) = &source.content else { return };
        let (a, len, bpm) = (a.clone(), source.length, self.project.bpm);
        let opts = ConvertOptions { kind, quantize: self.convert.quantize };
        let progress = Arc::new(AtomicU32::new(0));
        let (tx, rx) = std::sync::mpsc::channel();
        let pr = progress.clone();
        let spawned = std::thread::Builder::new().name("convert to midi".into()).spawn(move || {
            let mut tell = |f: f32| pr.store((f.clamp(0.0, 1.0) * 1000.0) as u32, Ordering::Relaxed);
            let _ = tx.send(to_midi::convert(&a, len, &opts, bpm, &mut tell));
        });
        if spawned.is_err() {
            self.toast("Could not start listening to the clip");
            return;
        }
        self.toast(format!("Listening to {} for its {}...", source.name, kind.name().to_lowercase()));
        self.convert.jobs.push(Job { kind, track_id, source, place, progress, done: rx });
    }

    /// The conversions under way: a window with how far each is, and the
    /// new tracks of the ones done.
    pub(crate) fn ui_convert(&mut self, ctx: &Context) {
        if self.convert.jobs.is_empty() {
            return;
        }
        let mut finished = Vec::new();
        for (i, job) in self.convert.jobs.iter().enumerate() {
            match job.done.try_recv() {
                Ok(result) => finished.push((i, Some(result))),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => finished.push((i, None)),
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        for (i, result) in finished.into_iter().rev() {
            let job = self.convert.jobs.remove(i);
            match result {
                Some(Ok((notes, length))) => self.convert_done(job, notes, length),
                Some(Err(e)) => self.toast(e),
                None => self.toast("Listening to the clip stopped"),
            }
        }
        if self.convert.jobs.is_empty() {
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
        egui::Area::new(egui::Id::new("convert_progress")).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -48.0)).order(egui::Order::Foreground).show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                for job in &self.convert.jobs {
                    let f = job.progress.load(Ordering::Relaxed) as f32 / 1000.0;
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new(format!("Converting {} to MIDI: its {}", job.source.name, job.kind.name().to_lowercase())).small());
                        ui.add(egui::ProgressBar::new(f).desired_width(140.0).show_percentage());
                    });
                }
            });
        });
    }

    /// A conversion done: its track right after the audio's, its clip picked.
    fn convert_done(&mut self, job: Job, notes: Vec<Note>, length: f64) {
        let Some(t) = self.project.tracks.iter().position(|t| t.id == job.track_id) else {
            self.toast("That clip's track is gone");
            return;
        };
        let done = self.project.add_converted_track(t + 1, &job.source, job.place, job.kind, notes, length);
        let track = &self.project.tracks[done.track];
        let sel = match track.slots.iter().position(|s| s.is_some()) {
            Some(scene) => Some(ClipSel::Slot { track: done.track, scene }),
            None => track.arrangement.first().map(|a| ClipSel::Arr { track: done.track, id: a.id }),
        };
        let name = track.name.clone();
        if let Some(sel) = sel {
            self.select_clip(sel);
        }
        self.detail = crate::app::DetailTab::Clip;
        self.toast(format!("{} notes on {name}", done.notes));
    }

    /// Extract Groove: the clip's feel into the Groove Pool (an audio
    /// clip's hits are found once, then kept with its sample).
    pub(crate) fn extract_groove(&mut self, sel: ClipSel) {
        let Some(clip) = self.clip_ref(sel) else { return };
        let name = groove::free_name(&self.project.grooves, &format!("{} Groove", clip.name));
        match groove::from_clip(clip, name, self.project.beats_per_bar_f()) {
            Some(g) => {
                let name = g.name.clone();
                self.project.grooves.push(g);
                self.toast(format!("{name} is in the Groove Pool: pick it in a MIDI clip's view"));
            }
            None => self.toast("Too few hits in the clip for a groove"),
        }
    }

    /// A MIDI clip's groove in its view: which, how much, and Commit.
    pub(crate) fn groove_controls(&mut self, ui: &mut Ui, sel: ClipSel, clip: &Clip) {
        let current = clip.groove.clone();
        let mut picked = current.as_ref().map(|g| g.name.clone());
        let shown = picked.clone().unwrap_or_else(|| "None".into());
        let missing = current.as_ref().is_some_and(|g| groove::find(&self.project.grooves, &g.name).is_none());
        ui.horizontal(|ui| {
            ui.label(RichText::new("Groove").small().color(TEXT_DIM));
            let text = if missing { RichText::new(format!("{shown} (gone)")).color(TEXT_DIM) } else { RichText::new(shown) };
            egui::ComboBox::from_id_salt("clip_groove").width(ui.available_width() - 4.0).selected_text(text).show_ui(ui, |ui| {
                ui.selectable_value(&mut picked, None, "None");
                if !self.project.grooves.is_empty() {
                    ui.label(RichText::new("Groove Pool").small().color(TEXT_DIM));
                    for g in &self.project.grooves {
                        ui.selectable_value(&mut picked, Some(g.name.clone()), &g.name);
                    }
                }
                ui.label(RichText::new("Built in").small().color(TEXT_DIM));
                for g in groove::built_in() {
                    ui.selectable_value(&mut picked, Some(g.name.clone()), &g.name);
                }
            })
            .response
            .on_hover_text("The notes play with this feel (where they are written stays): extract one from a clip with Extract Groove in its right-click menu");
        });
        if picked != current.as_ref().map(|g| g.name.clone()) {
            let amounts = current.clone();
            if let Some(c) = self.clip_mut(sel) {
                // A new groove keeps the amounts the clip had.
                c.groove = picked.map(|name| match amounts {
                    Some(a) => ClipGroove { name, ..a },
                    None => ClipGroove::new(name),
                });
            }
            return;
        }
        let Some(mut g) = current else { return };
        let mut changed = false;
        ui.horizontal(|ui| {
            for (label, v, tip) in [
                ("Timing", &mut g.timing, "How far the notes move to the groove's timing"),
                ("Velocity", &mut g.velocity, "How much of the groove's accents they take"),
                ("Random", &mut g.random, "How far they wander about, a little differently each time round"),
            ] {
                let mut pct = *v * 100.0;
                ui.label(RichText::new(label).small().color(TEXT_DIM));
                if ui.add(egui::DragValue::new(&mut pct).range(0.0..=100.0).speed(0.5).suffix("%").fixed_decimals(0)).on_hover_text(tip).changed() {
                    *v = pct / 100.0;
                    changed = true;
                }
            }
        });
        if changed {
            if let Some(c) = self.clip_mut(sel) {
                c.groove = Some(g.clone());
            }
        }
        ui.horizontal(|ui| {
            if ui.add_enabled(!missing, egui::Button::new("Commit")).on_hover_text("Write the groove into the notes (where they play now, as loud) and take it off the clip").clicked() {
                self.commit_groove(sel);
            }
            let in_pool = self.project.grooves.iter().position(|p| p.name == g.name);
            if let Some(i) = in_pool {
                if ui.small_button("Remove from Pool").on_hover_text("This groove leaves the Groove Pool (the clips with it play straight)").clicked() {
                    self.project.grooves.remove(i);
                }
            }
        });
    }

    /// Commit: the clip's groove written into its notes, and taken off.
    pub(crate) fn commit_groove(&mut self, sel: ClipSel) {
        let Some(clip) = self.clip_ref(sel) else { return };
        let len = clip.length;
        let Some(feel) = self.project.feel(clip) else { return };
        let mut notes = clip.notes().to_vec();
        groove::commit(&mut notes, &feel, len);
        if let Some(c) = self.clip_mut(sel) {
            if let Some(v) = c.notes_mut() {
                *v = notes;
            }
            c.groove = None;
        }
        self.toast("The groove is in the notes");
    }

    /// Script actions for these (see `KRABTON_SCRIPT`). Returns whether
    /// `name` was one of them.
    pub(crate) fn convert_script(&mut self, name: &str, arg: &str) -> bool {
        let nums: Vec<usize> = arg.split(',').filter_map(|v| v.trim().parse().ok()).collect();
        let slot = |n: &[usize]| (n.len() >= 2).then(|| ClipSel::Slot { track: n[0], scene: n[1] });
        match name {
            // convert=KIND,TRACK,SCENE: Convert (0 Melody, 1 Harmony, 2 Drums) that clip.
            "convert" if nums.len() == 3 => {
                if let (Some(kind), Some(sel)) = (Convert::ALL.get(nums[0]), slot(&nums[1..])) {
                    self.start_convert(sel, *kind);
                }
            }
            // convertq=16|8|0: Convert's grid (sixteenths, eighths, off).
            "convertq" => {
                self.convert.quantize = match nums.first() {
                    Some(16) => Some(0.25),
                    Some(8) => Some(0.5),
                    _ => None,
                }
            }
            // extractgroove=TRACK,SCENE: that clip's groove into the pool.
            "extractgroove" => {
                if let Some(sel) = slot(&nums) {
                    self.extract_groove(sel);
                }
            }
            // groove=NAME: the selected clip plays with that groove (nothing: none);
            // grooveamt=TIMING,VELOCITY,RANDOM in percent; groovecommit: Commit.
            "groove" => {
                if let Some(sel) = self.sel_clip {
                    let name = arg.trim().to_string();
                    if let Some(c) = self.clip_mut(sel) {
                        c.groove = (!name.is_empty()).then(|| ClipGroove::new(name));
                    }
                }
            }
            "grooveamt" if nums.len() == 3 => {
                if let Some(c) = self.sel_clip.and_then(|s| self.clip_mut(s)) {
                    if let Some(g) = &mut c.groove {
                        g.timing = nums[0].min(100) as f32 / 100.0;
                        g.velocity = nums[1].min(100) as f32 / 100.0;
                        g.random = nums[2].min(100) as f32 / 100.0;
                    }
                }
            }
            "groovecommit" => {
                if let Some(sel) = self.sel_clip {
                    self.commit_groove(sel);
                }
            }
            _ => return false,
        }
        true
    }
}
