//! Hum a Melody: sing or hum into the microphone and it becomes notes in a
//! clip on the selected MIDI track, played by whatever is on that track. It
//! hears the voice the way Convert Melody does (see `krabton_engine::to_midi`).
//! With the song playing the notes stay where they were sung in the bar;
//! stopped, the clip starts at the first note. The last take can still be put
//! on another grid, in the key or an octave away.

use super::windows::tool_window;
use crate::app::{App, ClipSel, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Color32, Context, Rect, RichText, Sense, Ui};
use krabton_engine::audio_in::InputRing;
use krabton_engine::scale::Scale;
use krabton_engine::to_midi::{self, Heard};
use krabton_engine::{Clip, ClipContent, Note, TrackKind};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;

/// The grids the notes can go on (beats; None: where they were sung).
const GRIDS: [(Option<f64>, &str); 4] = [(None, "Off"), (Some(0.25), "1/16"), (Some(0.5), "1/8"), (Some(1.0), "1/4")];
const OCTAVES: [i32; 5] = [-2, -1, 0, 1, 2];

/// Being recorded.
struct Take {
    _device: crate::audio::AudioInputDevice,
    ring: Arc<InputRing>,
    samples: Vec<f32>,
    sr: u32,
    /// Where in the bar it started (beats), with the song playing.
    offset: Option<f64>,
    level: f32,
}

/// Being heard, on a thread of its own.
struct Listening {
    progress: Arc<AtomicU32>,
    done: Receiver<Vec<Heard>>,
    offset: Option<f64>,
    track: u64,
}

/// The last take, to go on changing.
struct Last {
    heard: Vec<Heard>,
    offset: Option<f64>,
    track: u64,
    clip: u64,
}

pub struct HumState {
    pub open: bool,
    take: Option<Take>,
    listening: Option<Listening>,
    last: Option<Last>,
    grid: usize,
    octave: i32,
    in_key: bool,
}

impl Default for HumState {
    fn default() -> Self {
        Self { open: false, take: None, listening: None, last: None, grid: 1, octave: 0, in_key: true }
    }
}

/// The notes of what was heard, and how long their clip is (whole bars).
pub(crate) fn hum_notes(heard: &[Heard], bpm: f64, bar: f64, offset: Option<f64>, grid: Option<f64>, scale: Option<Scale>, octave: i32) -> (Vec<Note>, f64) {
    let beat_of = |t: f64| t * bpm / 60.0;
    let first = heard.iter().map(|h| h.start).fold(f64::INFINITY, f64::min);
    let shift = offset.unwrap_or(if first.is_finite() { -beat_of(first) } else { 0.0 });
    let mut notes = Vec::with_capacity(heard.len());
    for h in heard {
        let (mut s, mut e) = (beat_of(h.start) + shift, beat_of(h.end) + shift);
        if let Some(q) = grid.filter(|q| *q > 0.0) {
            s = (s / q).round() * q;
            e = ((e / q).round() * q).max(s + q);
        }
        let pitch = scale.map_or(h.pitch, |sc| sc.snap(h.pitch)) as i32 + 12 * octave;
        notes.push(Note::new(pitch.clamp(0, 127) as u8, s.max(0.0), (e - s.max(0.0)).max(0.01), h.velocity));
    }
    if grid.is_some() {
        krabton_engine::project::tidy_notes(&mut notes);
    }
    notes.sort_by(|a, b| a.start.total_cmp(&b.start).then(a.pitch.cmp(&b.pitch)));
    let end = notes.iter().map(|n| n.start + n.length).fold(0.0, f64::max);
    (notes, ((end / bar - 1e-6).ceil().max(1.0)) * bar)
}

impl App {
    /// The MIDI track the notes go to.
    fn hum_track(&self) -> Option<usize> {
        match self.sel_track {
            TrackSel::Track(t) => self.project.tracks.get(t).filter(|tr| tr.kind == TrackKind::Midi).map(|_| t),
            _ => None,
        }
    }

    fn hum_start(&mut self) {
        let status = self.status();
        let sr = status.sample_rate.load();
        let sr = if sr >= 8000.0 { sr as u32 } else { 48000 };
        match crate::audio::start_input(sr, self.prefs.audio_input.as_deref()) {
            Ok((device, ring)) => {
                let bar = self.project.beats_per_bar.max(1) as f64;
                let playing = status.playing.load(Ordering::Relaxed);
                let offset = playing.then(|| status.session_pos.load().rem_euclid(bar));
                self.hum.take = Some(Take { _device: device, ring, samples: Vec::new(), sr, offset, level: 0.0 });
            }
            Err(e) => self.toast(format!("No microphone to hear: {e}")),
        }
    }

    fn hum_stop(&mut self) {
        let Some(take) = self.hum.take.take() else { return };
        let Some(t) = self.hum_track() else {
            self.toast("Select a MIDI track for the notes");
            return;
        };
        let track = self.project.tracks[t].id;
        let progress = Arc::new(AtomicU32::new(0));
        let (tx, rx) = std::sync::mpsc::channel();
        let pr = progress.clone();
        let (samples, sr, offset) = (take.samples, take.sr, take.offset);
        let spawned = std::thread::Builder::new().name("hum".into()).spawn(move || {
            let mut tell = |f: f32| pr.store((f.clamp(0.0, 1.0) * 1000.0) as u32, Ordering::Relaxed);
            let _ = tx.send(to_midi::melody(&samples, sr, 0.0, &mut tell));
        });
        match spawned {
            Ok(_) => self.hum.listening = Some(Listening { progress, done: rx, offset, track }),
            Err(_) => self.toast("Could not start listening to the take"),
        }
    }

    /// The last take as notes in its clip, as the settings have it now.
    fn hum_write(&mut self) {
        let Some(last) = &self.hum.last else { return };
        let scale = self.hum.in_key.then_some(self.project.scale);
        let bar = self.project.beats_per_bar.max(1) as f64;
        let (notes, length) = hum_notes(&last.heard, self.project.bpm, bar, last.offset, GRIDS[self.hum.grid].0, scale, self.hum.octave);
        let (track, clip) = (last.track, last.clip);
        let Some(tr) = self.project.tracks.iter_mut().find(|t| t.id == track) else { return };
        if let Some(c) = tr.slots.iter_mut().flatten().find(|c| c.id == clip) {
            c.length = length;
            c.content = ClipContent::Midi { notes };
        }
    }

    /// A take heard: a new clip with its notes.
    fn hum_heard(&mut self, heard: Vec<Heard>, offset: Option<f64>, track: u64) {
        if heard.is_empty() {
            self.toast("No melody heard: hum a little louder, or closer to the microphone");
            return;
        }
        let Some(t) = self.project.tracks.iter().position(|tr| tr.id == track) else { return };
        let from = self.sel_scene.min(self.project.tracks[t].slots.len());
        let free = (from..self.project.tracks[t].slots.len()).find(|&s| self.project.tracks[t].slots[s].is_none());
        let scene = free.unwrap_or_else(|| {
            let at = self.project.scenes.len();
            self.project.add_scene(at);
            at
        });
        let id = self.project.alloc_id();
        let color = self.project.tracks[t].color;
        self.project.tracks[t].slots[scene] = Some(Clip::new_midi(id, "Hum", color, 4.0));
        self.hum.last = Some(Last { heard, offset, track, clip: id });
        self.hum_write();
        self.select_clip(ClipSel::Slot { track: t, scene });
        self.detail = crate::app::DetailTab::Clip;
        let name = self.project.tracks[t].name.clone();
        self.toast(format!("Your melody is a clip on {name}"));
    }

    /// What goes on every frame, the window open or not.
    fn hum_poll(&mut self, ctx: &Context) {
        if let Some(take) = &mut self.hum.take {
            let before = take.samples.len();
            take.ring.drain_mono(&mut take.samples);
            if take.samples.len() > before {
                take.level = take.samples[before..].iter().fold(0.0f32, |m, v| m.max(v.abs()));
            }
            ctx.request_repaint();
        }
        if let Some(l) = &self.hum.listening {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
            match l.done.try_recv() {
                Ok(heard) => {
                    let (offset, track) = (l.offset, l.track);
                    self.hum.listening = None;
                    self.hum_heard(heard, offset, track);
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.hum.listening = None;
                    self.toast("Listening to the take stopped");
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
    }

    pub(crate) fn ui_hum(&mut self, ctx: &Context) {
        self.hum_poll(ctx);
        if !self.hum.open {
            return;
        }
        let (keep, _) = tool_window(ctx, "hum", "Hum a Melody", vec2(340.0, 230.0), |ui| self.hum_body(ui));
        if !keep {
            self.hum.open = false;
            self.hum.take = None;
        }
    }

    fn hum_body(&mut self, ui: &mut Ui) {
        let target = self.hum_track().map(|t| self.project.tracks[t].name.clone());
        ui.label(RichText::new(match &target {
            Some(name) => format!("The notes go to {name}"),
            None => "Select a MIDI track for the notes".to_string(),
        }).small().color(TEXT_DIM));
        ui.horizontal(|ui| {
            let recording = self.hum.take.is_some();
            let busy = self.hum.listening.is_some();
            let label = RichText::new(if recording { "Stop" } else { "Record" }).color(if recording { REC } else { TEXT });
            if ui.add_enabled(!busy && (recording || target.is_some()), egui::Button::new(label)).clicked() {
                if recording {
                    self.hum_stop();
                } else {
                    self.hum_start();
                }
            }
            if let Some(take) = &self.hum.take {
                let (bar, _) = ui.allocate_exact_size(vec2(120.0, 12.0), Sense::hover());
                rect_fill(ui.painter(), bar, 2.0, BG);
                let f = (take.level.max(1e-5).log10() * 20.0 + 60.0) / 60.0;
                let lit = Rect::from_min_max(bar.min, pos2(bar.left() + f.clamp(0.0, 1.0) * bar.width(), bar.bottom()));
                rect_fill(ui.painter(), lit, 2.0, if take.level > 0.9 { REC } else { Color32::from_gray(190) });
                let secs = take.samples.len() as f32 / take.sr.max(1) as f32;
                ui.label(RichText::new(format!("{secs:.1} s")).small().color(TEXT_DIM));
            }
            if let Some(l) = &self.hum.listening {
                ui.spinner();
                ui.add(egui::ProgressBar::new(l.progress.load(Ordering::Relaxed) as f32 / 1000.0).desired_width(120.0));
            }
        });
        let (grid, octave, in_key) = (self.hum.grid, self.hum.octave, self.hum.in_key);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Grid").small().color(TEXT_DIM));
            egui::ComboBox::from_id_salt("hum_grid").width(60.0).selected_text(GRIDS[self.hum.grid].1).show_ui(ui, |ui| {
                for (k, g) in GRIDS.iter().enumerate() {
                    ui.selectable_value(&mut self.hum.grid, k, g.1);
                }
            });
            ui.label(RichText::new("Octave").small().color(TEXT_DIM));
            egui::ComboBox::from_id_salt("hum_octave").width(50.0).selected_text(format!("{:+}", self.hum.octave)).show_ui(ui, |ui| {
                for o in OCTAVES {
                    ui.selectable_value(&mut self.hum.octave, o, format!("{o:+}"));
                }
            });
        });
        let key = self.project.scale.name();
        ui.checkbox(&mut self.hum.in_key, format!("In the song's key ({key})")).on_hover_text("A note sung between the notes of the key goes to the nearest one");
        if (grid, octave, in_key) != (self.hum.grid, self.hum.octave, self.hum.in_key) {
            self.hum_write();
        }
        ui.label(RichText::new("With the song playing the notes stay where you sang them in the bar. Use headphones, or the microphone hears the song as well").small().color(TEXT_DIM));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use krabton_engine::scale::ScaleKind;

    fn heard(pitch: u8, start: f64, end: f64) -> Heard {
        Heard { pitch, start, end, velocity: 0.8 }
    }

    #[test]
    fn stopped_the_clip_starts_at_the_first_note() {
        // At 120 a beat is half a second.
        let (notes, length) = hum_notes(&[heard(60, 1.0, 1.5), heard(62, 1.5, 2.0)], 120.0, 4.0, None, None, None, 0);
        assert_eq!((notes[0].start, notes[0].length), (0.0, 1.0));
        assert_eq!(notes[1].start, 1.0);
        assert_eq!(length, 4.0, "a whole bar");
    }

    #[test]
    fn playing_the_notes_stay_where_they_were_sung_in_the_bar() {
        let (notes, _) = hum_notes(&[heard(60, 0.5, 1.0)], 120.0, 4.0, Some(2.0), None, None, 0);
        assert_eq!(notes[0].start, 3.0);
    }

    #[test]
    fn the_grid_the_key_and_the_octave_do_their_part() {
        let c = Scale::new(0, ScaleKind::Major);
        let (notes, length) = hum_notes(&[heard(61, 0.0, 0.3), heard(64, 2.1, 2.6)], 120.0, 4.0, None, Some(0.5), Some(c), -1);
        assert_eq!(notes[0].length, 0.5, "on the grid, at least one step");
        assert!(c.contains(notes[0].pitch), "{}", notes[0].pitch);
        assert_eq!(notes[1].pitch, 52, "an octave down");
        assert_eq!(notes[1].start, 4.0);
        assert_eq!(length, 8.0, "it goes into the second bar");
    }

    #[test]
    fn a_hummed_melody_comes_out_as_its_notes() {
        // A voice a little flat of C, E and G, with a vibrato and overtones.
        let sr = 48000u32;
        let mut x = Vec::new();
        let mut phase = 0.0f64;
        for (k, midi) in [59.7f64, 63.7, 66.7].iter().enumerate() {
            for i in 0..sr as usize / 2 {
                let t = (k * sr as usize / 2 + i) as f64 / sr as f64;
                let vib = 0.25 * (std::f64::consts::TAU * 5.5 * t).sin();
                let f = 440.0 * 2f64.powf((midi + vib - 69.0) / 12.0);
                phase += std::f64::consts::TAU * f / sr as f64;
                let env = ((i as f64 / 1500.0).min(1.0)) * (((sr as usize / 2 - i) as f64 / 1500.0).min(1.0));
                x.push((env * 0.4 * (phase.sin() + 0.4 * (2.0 * phase).sin() + 0.2 * (3.0 * phase).sin())) as f32);
            }
        }
        let got = to_midi::melody(&x, sr, 0.0, &mut |_| {});
        let c = Scale::new(0, ScaleKind::Major);
        let (notes, _) = hum_notes(&got, 120.0, 4.0, None, Some(0.25), Some(c), 0);
        let pitches: Vec<u8> = notes.iter().map(|n| n.pitch).collect();
        assert_eq!(pitches, [60, 64, 67], "{got:?}");
    }
}
