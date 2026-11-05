//! DJ: two decks and your own set on one mixer. The decks are tracks with a
//! Turntable on them; everything else in the project is the set, a channel of
//! its own, so a record can be mixed into the music you are making. Sync plays
//! a deck at the tempo of the other (or of the song) and lines up its beats;
//! a deck that is Master gives the whole song its tempo. See `dsp::dj`.

use crate::app::{App, View};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, RichText, Sense, Stroke, Ui};
use krabton_engine::dsp::dj::{self, DjChannel, EQ_MAX_DB, KILL_DB, SET};
use krabton_engine::dsp::turntable::{self as tt, DeckAct, Record};
use krabton_engine::sample::PEAK_BLOCK;
use krabton_engine::{Command, Instrument, SampleRef};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;

const DECK_COLOR: [Color32; 2] = [Color32::from_rgb(0x4f, 0x9d, 0xf7), Color32::from_rgb(0xf2, 0x8c, 0x38)];
const NAMES: [&str; 3] = ["A", "B", "Set"];
const AUDIO: [&str; 6] = ["wav", "flac", "mp3", "aif", "aiff", "ogg"];
/// Seconds shown either side of the needle.
const WAVE_SECS: f32 = 5.0;
/// A nudge of the record, in seconds.
const NUDGE: f64 = 0.015;

/// A record heard, ready to go on its deck.
struct Heard {
    sample: SampleRef,
    record: Record,
}

#[derive(Default)]
pub struct DjState {
    loading: [Option<Receiver<Result<Heard, String>>>; 2],
    /// The deck that gives the song its tempo.
    master: Option<usize>,
    /// Where Cue goes back to, on each deck.
    cue: [f64; 2],
    /// Where the hand took each record's waveform.
    drag_from: [f32; 2],
    files: Vec<PathBuf>,
    listed: Option<PathBuf>,
}

/// What can be seen of a deck.
#[derive(Clone)]
struct DeckView {
    track: usize,
    id: u64,
    sample: Option<SampleRef>,
    record: Record,
    params: Vec<f32>,
    pos: f64,
    running: bool,
    looping: Option<(f64, f64)>,
}

impl DeckView {
    fn speed(&self, song_bpm: f64) -> f64 {
        tt::motor_speed(&self.params, &self.record, song_bpm)
    }

    fn bpm(&self, song_bpm: f64) -> f64 {
        self.record.bpm * self.speed(song_bpm)
    }
}

/// The audio files of a folder, by name.
pub(crate) fn audio_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|d| d.flatten().map(|e| e.path()).filter(|p| p.extension().and_then(|e| e.to_str()).is_some_and(|e| AUDIO.contains(&e.to_lowercase().as_str()))).collect())
        .unwrap_or_default();
    v.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    v
}

fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

impl App {
    /// The mixer on, and both decks there.
    pub(crate) fn dj_enter(&mut self) {
        self.project.dj.on = true;
        for d in 0..2 {
            let there = self.project.dj.decks[d].is_some_and(|id| self.project.tracks.iter().any(|t| t.id == id));
            if !there {
                let at = self.project.tracks.len();
                let mut inst = Instrument::turntable("Club Deck");
                if let Instrument::Turntable { params, .. } = &mut inst {
                    // A DJ starts a record when they say, not on the next bar.
                    params[tt::QUANTIZE] = 0.0;
                    params[tt::START] = 0.05;
                    params[tt::STOP] = 0.1;
                }
                let t = self.project.insert_midi_track(at, format!("Deck {}", NAMES[d]), Some(inst));
                self.project.dj.decks[d] = Some(self.project.tracks[t].id);
            }
        }
    }

    fn dj_deck(&self, d: usize) -> Option<DeckView> {
        let id = self.project.dj.decks[d]?;
        let track = self.project.tracks.iter().position(|t| t.id == id)?;
        let Some(Instrument::Turntable { sample, record, params, .. }) = &self.project.tracks[track].instrument else { return None };
        let status = self.status();
        let v = status.tracks.get(track)?.inst_values();
        let looping = (v[7] > 0.5).then_some((v[8] as f64, v[9] as f64));
        Some(DeckView { track, id, sample: sample.clone(), record: record.clone(), params: params.clone(), pos: v[0] as f64, running: v[4] > 0.5, looping })
    }

    /// Puts a file on a deck, heard on a thread of its own: a song takes a moment.
    pub(crate) fn dj_load(&mut self, d: usize, path: &Path) {
        let path = path.to_path_buf();
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new().name("dj load".into()).spawn(move || {
            let result = SampleRef::load(&path).map(|sample| {
                let mut record = tt::listen(&sample);
                record.key = krabton_engine::analysis::detect_key(&sample).map(|k| (k.root, k.minor));
                Heard { sample, record }
            });
            let _ = tx.send(result);
        });
        match spawned {
            Ok(_) => self.dj.loading[d] = Some(rx),
            Err(_) => self.toast("Could not start loading the record"),
        }
    }

    fn dj_poll(&mut self) {
        for d in 0..2 {
            let Some(rx) = &self.dj.loading[d] else { continue };
            let result = match rx.try_recv() {
                Ok(r) => r,
                Err(std::sync::mpsc::TryRecvError::Empty) => continue,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => Err("Loading the record stopped".to_string()),
            };
            self.dj.loading[d] = None;
            match result {
                Ok(heard) => self.dj_put_on(d, heard),
                Err(e) => self.toast(format!("Could not load the record: {e}")),
            }
        }
    }

    fn dj_put_on(&mut self, d: usize, heard: Heard) {
        let Some(view) = self.dj_deck(d) else { return };
        if view.running {
            self.cmd(Command::Deck { track: view.id, act: DeckAct::StartStop });
        }
        let start = heard.record.beat;
        if let Some(Instrument::Turntable { sample, record, .. }) = &mut self.project.tracks[view.track].instrument {
            *sample = Some(heard.sample);
            *record = heard.record;
        }
        self.dj.cue[d] = start;
        self.cmd(Command::Deck { track: view.id, act: DeckAct::Jump { secs: start, play: false } });
    }

    fn dj_set_param(&mut self, view: &DeckView, k: usize, v: f32) {
        if let Some(Instrument::Turntable { params, .. }) = &mut self.project.tracks[view.track].instrument {
            params[k] = v;
        }
    }

    /// Sync on: the deck plays at the tempo of the other deck if that is running
    /// (which becomes Master), else of the song; its beats are lined up with theirs.
    fn dj_sync(&mut self, d: usize, view: &DeckView) {
        if view.params[tt::SYNC] >= 0.5 {
            self.dj_set_param(view, tt::SYNC, 0.0);
            return;
        }
        if view.record.bpm <= 0.0 {
            self.toast("No steady beat found on this record, so it cannot sync");
            return;
        }
        let other = self.dj_deck(1 - d).filter(|o| o.running && o.record.bpm > 0.0);
        let status = self.status();
        let phase = if let Some(o) = &other {
            if o.params[tt::SYNC] < 0.5 {
                self.project.bpm = (o.bpm(self.project.bpm) * 100.0).round() / 100.0;
                self.dj.master = Some(1 - d);
            }
            Some(dj::phase_of(o.pos, o.record.bpm, o.record.beat))
        } else if status.playing.load(Ordering::Relaxed) {
            Some(status.session_pos.load().rem_euclid(1.0))
        } else {
            None
        };
        self.dj_set_param(view, tt::SYNC, 1.0);
        if let Some(phase) = phase {
            let jump = dj::phase_jump(view.pos, view.record.bpm, view.record.beat, phase);
            self.cmd(Command::Deck { track: view.id, act: DeckAct::Jump { secs: (view.pos + jump).max(0.0), play: false } });
        }
    }

    /// A deck that is Master gives the song its tempo, and the other deck and your set follow.
    fn dj_follow_master(&mut self) {
        let Some(view) = self.dj.master.and_then(|d| self.dj_deck(d)) else { return };
        if view.record.bpm <= 0.0 || view.params[tt::SYNC] >= 0.5 {
            return;
        }
        let bpm = (view.bpm(self.project.bpm) * 100.0).round() / 100.0;
        if (bpm - self.project.bpm).abs() >= 0.01 {
            self.project.bpm = bpm;
        }
    }

    /// The DJ view.
    pub(crate) fn ui_dj(&mut self, ui: &mut Ui) {
        self.dj_enter();
        self.dj_poll();
        self.dj_follow_master();
        ui.ctx().request_repaint();
        let decks = [self.dj_deck(0), self.dj_deck(1)];
        let full = ui.available_rect_before_wrap();
        ui.allocate_rect(full, Sense::hover());
        let wave_h = (full.height() * 0.15).clamp(56.0, 110.0);
        let over_h = 16.0;
        let lib_h = (full.height() * 0.2).clamp(80.0, 150.0);
        let mixer_w = 330.0f32.min(full.width() * 0.4);
        let mut y = full.top();
        for d in 0..2 {
            let over = Rect::from_min_size(pos2(full.left(), y), vec2(full.width(), over_h));
            let wave = Rect::from_min_size(pos2(full.left(), y + over_h + 2.0), vec2(full.width(), wave_h));
            if let Some(view) = &decks[d] {
                self.dj_overview(ui, over, d, view);
                self.dj_wave(ui, wave, d, view);
            }
            y = wave.bottom() + 4.0;
        }
        let row = Rect::from_min_max(pos2(full.left(), y), pos2(full.right(), full.bottom() - lib_h - 4.0));
        let side_w = (row.width() - mixer_w - 12.0) * 0.5;
        let left = Rect::from_min_size(row.min, vec2(side_w, row.height()));
        let mixer = Rect::from_min_size(pos2(left.right() + 6.0, row.top()), vec2(mixer_w, row.height()));
        let right = Rect::from_min_size(pos2(mixer.right() + 6.0, row.top()), vec2(side_w, row.height()));
        for (d, rect) in [(0, left), (1, right)] {
            if let Some(view) = &decks[d] {
                let other_key = decks[1 - d].as_ref().and_then(|o| o.record.key);
                let mut sub = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                sub.set_clip_rect(rect);
                self.dj_controls(&mut sub, d, view, other_key);
            }
        }
        let mut sub = ui.new_child(egui::UiBuilder::new().max_rect(mixer));
        sub.set_clip_rect(mixer);
        self.dj_mixer(&mut sub);
        let lib = Rect::from_min_max(pos2(full.left(), full.bottom() - lib_h), full.max);
        let mut sub = ui.new_child(egui::UiBuilder::new().max_rect(lib));
        sub.set_clip_rect(lib);
        self.dj_library(&mut sub);
    }

    /// The whole record, small: click to drop the needle there.
    fn dj_overview(&mut self, ui: &mut Ui, rect: Rect, d: usize, view: &DeckView) {
        let resp = ui.interact(rect, Id::new(("dj_over", d)), Sense::click_and_drag());
        let p = ui.painter_at(rect);
        rect_fill(&p, rect, 2.0, BG);
        let Some(sample) = &view.sample else { return };
        let dur = sample.duration().max(0.001);
        let cols = rect.width() as usize;
        if !sample.peaks.is_empty() && cols > 0 {
            for x in 0..cols {
                let a = x * sample.peaks.len() / cols;
                let b = ((x + 1) * sample.peaks.len() / cols).max(a + 1).min(sample.peaks.len());
                let step = ((b - a) / 16).max(1);
                let top = sample.peaks[a..b].iter().step_by(step).fold(0.0f32, |m, q| m.max(q.1.abs()).max(q.0.abs()));
                let h = (top.min(1.0) * rect.height() * 0.5).max(0.5);
                let fx = rect.left() + x as f32;
                p.line_segment([pos2(fx, rect.center().y - h), pos2(fx, rect.center().y + h)], Stroke::new(1.0_f32, DECK_COLOR[d].gamma_multiply(0.55)));
            }
        }
        let x = rect.left() + (view.pos / dur).clamp(0.0, 1.0) as f32 * rect.width();
        p.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(2.0_f32, Color32::WHITE));
        if resp.clicked() || resp.dragged() {
            if let Some(at) = resp.interact_pointer_pos() {
                let secs = ((at.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64 * dur;
                self.cmd(Command::Deck { track: view.id, act: DeckAct::Jump { secs, play: false } });
            }
        }
    }

    /// Around the needle, with its beats. Dragging it moves the record by hand.
    fn dj_wave(&mut self, ui: &mut Ui, rect: Rect, d: usize, view: &DeckView) {
        let resp = ui.interact(rect, Id::new(("dj_wave", d)), Sense::click_and_drag());
        let p = ui.painter_at(rect);
        rect_fill(&p, rect, 2.0, Color32::from_rgb(0x12, 0x12, 0x15));
        let px_per_sec = rect.width() / (2.0 * WAVE_SECS);
        let mid = rect.center();
        let Some(sample) = &view.sample else {
            p.text(mid, Align2::CENTER_CENTER, match self.dj.loading[d] {
                Some(_) => "Listening to the record...".to_string(),
                None => format!("Deck {}: choose a record below", NAMES[d]),
            }, FontId::proportional(13.0), TEXT_DIM);
            return;
        };
        let sr = sample.sample_rate.max(1) as f64;
        let x_of = |secs: f64| mid.x + ((secs - view.pos) as f32) * px_per_sec;
        if let Some((start, len)) = view.looping {
            let shade = Rect::from_min_max(pos2(x_of(start), rect.top()), pos2(x_of(start + len), rect.bottom()));
            p.rect_filled(shade.intersect(rect), 0.0, DECK_COLOR[d].gamma_multiply(0.2));
        }
        // The beats, bars brighter.
        if view.record.bpm > 0.0 {
            let beat = 60.0 / view.record.bpm;
            let from = ((view.pos - WAVE_SECS as f64 - view.record.beat) / beat).floor() as i64;
            let to = ((view.pos + WAVE_SECS as f64 - view.record.beat) / beat).ceil() as i64;
            for k in from..=to {
                let x = x_of(view.record.beat + k as f64 * beat);
                let bar = k.rem_euclid(4) == 0;
                let col = if bar { Color32::from_gray(150) } else { Color32::from_gray(58) };
                p.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(if bar { 1.5_f32 } else { 1.0 }, col));
            }
        }
        let peaks_per_sec = sr / PEAK_BLOCK as f64;
        let halfh = rect.height() * 0.5 - 2.0;
        for x in 0..rect.width() as usize {
            let fx = rect.left() + x as f32;
            let t0 = view.pos + ((fx - mid.x) / px_per_sec) as f64;
            let t1 = t0 + 1.0 / px_per_sec as f64;
            if t1 < 0.0 {
                continue;
            }
            let a = (t0.max(0.0) * peaks_per_sec) as usize;
            let b = ((t1 * peaks_per_sec) as usize).max(a + 1).min(sample.peaks.len());
            if a >= sample.peaks.len() {
                break;
            }
            let (lo, hi) = sample.peaks[a..b].iter().fold((0.0f32, 0.0f32), |m, q| (m.0.min(q.0), m.1.max(q.1)));
            p.line_segment([pos2(fx, mid.y - hi.min(1.0) * halfh), pos2(fx, mid.y - lo.max(-1.0) * halfh)], Stroke::new(1.0_f32, DECK_COLOR[d]));
        }
        for (k, c) in view.record.cues.iter().enumerate() {
            if let Some(c) = c {
                let x = x_of(*c);
                if rect.x_range().contains(x) {
                    p.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(1.0_f32, Color32::from_rgb(0x7f, 0xe0, 0x9a)));
                    p.text(pos2(x + 2.0, rect.top()), Align2::LEFT_TOP, format!("{}", k + 1), FontId::proportional(10.0), Color32::from_rgb(0x7f, 0xe0, 0x9a));
                }
            }
        }
        p.line_segment([pos2(mid.x, rect.top()), pos2(mid.x, rect.bottom())], Stroke::new(2.0_f32, Color32::WHITE));
        if resp.drag_started() {
            self.dj.drag_from[d] = resp.interact_pointer_pos().map_or(mid.x, |q| q.x);
        }
        if resp.dragged() {
            if let Some(at) = resp.interact_pointer_pos() {
                let secs = ((self.dj.drag_from[d] - at.x) / px_per_sec) as f64;
                self.cmd(Command::Deck { track: view.id, act: DeckAct::Hand(Some(secs / tt::turn_secs(&view.params))) });
            }
        }
        if resp.drag_stopped() {
            self.cmd(Command::Deck { track: view.id, act: DeckAct::Hand(None) });
        }
    }

    fn dj_controls(&mut self, ui: &mut Ui, d: usize, view: &DeckView, other_key: Option<(u8, bool)>) {
        let act = |app: &mut App, act: DeckAct| app.cmd(Command::Deck { track: view.id, act });
        let song_bpm = self.project.bpm;
        egui::Frame::new().fill(PANEL2).corner_radius(4.0).inner_margin(6).show(ui, |ui| {
            ui.set_min_height(ui.available_height() - 12.0);
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Deck {}", NAMES[d])).strong().color(DECK_COLOR[d]));
                let name = view.sample.as_ref().map_or("No record".to_string(), |s| s.name.clone());
                ui.add(egui::Label::new(RichText::new(name).color(TEXT)).truncate());
            });
            ui.horizontal(|ui| {
                if view.record.bpm > 0.0 {
                    ui.label(RichText::new(format!("{:.2}", view.bpm(song_bpm))).size(20.0).color(TEXT));
                    ui.label(RichText::new("BPM").small().color(TEXT_DIM));
                    let pct = (view.speed(song_bpm) - 1.0) * 100.0;
                    ui.label(RichText::new(format!("{pct:+.1} %")).small().color(TEXT_DIM));
                } else {
                    ui.label(RichText::new("-- BPM").size(20.0).color(TEXT_DIM));
                }
                if let Some(key) = view.record.key {
                    let fits = other_key.is_some_and(|o| dj::keys_fit(key, o));
                    let col = if other_key.is_none() { TEXT } else if fits { PLAY } else { REC };
                    ui.label(RichText::new(dj::wheel_name(key.0, key.1)).size(16.0).color(col)).on_hover_text("Its key on the wheel: green mixes with the other deck, red clashes");
                }
                if let Some(s) = &view.sample {
                    ui.label(RichText::new(format!("-{}", clock(s.duration() - view.pos))).small().color(TEXT_DIM));
                }
            });
            ui.horizontal(|ui| {
                let play = RichText::new(if view.running { "Pause" } else { "Play" }).color(if view.running { TEXT_DARK } else { TEXT });
                if ui.add(egui::Button::new(play).fill(if view.running { PLAY } else { PANEL3 })).clicked() {
                    act(self, DeckAct::StartStop);
                }
                if ui.button("Cue").on_hover_text("Stopped: this is where it goes back to. Playing: back there, and it stops").clicked() {
                    if view.running {
                        let secs = self.dj.cue[d];
                        act(self, DeckAct::StartStop);
                        act(self, DeckAct::Jump { secs, play: false });
                    } else {
                        self.dj.cue[d] = view.pos;
                    }
                }
                let synced = view.params[tt::SYNC] >= 0.5;
                if ui.selectable_label(synced, "Sync").on_hover_text("Plays at the tempo of the other deck (or your song) and lines up the beats").clicked() {
                    self.dj_sync(d, view);
                }
                let master = self.dj.master == Some(d);
                if ui.selectable_label(master, "Master").on_hover_text("This deck gives the song its tempo: your set and the other deck follow").clicked() {
                    self.dj.master = if master { None } else { Some(d) };
                    if !master && synced {
                        self.dj_set_param(view, tt::SYNC, 0.0);
                    }
                }
                let lock = view.params[tt::KEY_LOCK] >= 0.5;
                if ui.selectable_label(lock, "Key Lock").on_hover_text("Keeps the pitch while the tempo changes (a simple stretch: you can hear it on big changes)").clicked() {
                    self.dj_set_param(view, tt::KEY_LOCK, if lock { 0.0 } else { 1.0 });
                }
                if ui.small_button("<").on_hover_text("Nudge back").clicked() {
                    act(self, DeckAct::Jump { secs: (view.pos - NUDGE).max(0.0), play: false });
                }
                if ui.small_button(">").on_hover_text("Nudge on").clicked() {
                    act(self, DeckAct::Jump { secs: view.pos + NUDGE, play: false });
                }
            });
            ui.horizontal(|ui| {
                let mut pitch = view.params[tt::PITCH];
                if ui.add_enabled(view.params[tt::SYNC] < 0.5, egui::Slider::new(&mut pitch, -1.0..=1.0).show_value(false)).on_hover_text("The pitch fader").changed() {
                    self.dj_set_param(view, tt::PITCH, pitch);
                }
                let mut range = view.params[tt::RANGE].round() as usize;
                let before = range;
                egui::ComboBox::from_id_salt(("dj_range", d)).width(62.0).selected_text(tt::RANGES[range.min(2)]).show_ui(ui, |ui| {
                    for (k, r) in tt::RANGES.iter().enumerate() {
                        ui.selectable_value(&mut range, k, *r);
                    }
                });
                if range != before {
                    self.dj_set_param(view, tt::RANGE, range as f32);
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Cues").small().color(TEXT_DIM));
                for k in 0..tt::CUES {
                    let set = view.record.cues[k].is_some();
                    let label = RichText::new(format!("{}", k + 1)).color(if set { TEXT } else { TEXT_DIM });
                    let resp = ui.small_button(label).on_hover_text("Click: jump there and play. Right-click: this is cue now");
                    if resp.clicked() {
                        if let Some(secs) = view.record.cues[k] {
                            act(self, DeckAct::Jump { secs, play: true });
                        }
                    }
                    if resp.secondary_clicked() {
                        if let Some(Instrument::Turntable { record, .. }) = &mut self.project.tracks[view.track].instrument {
                            record.cues[k] = Some(view.pos);
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Loop").small().color(TEXT_DIM));
                let mut size = view.params[tt::LOOP].round() as usize;
                let before = size;
                egui::ComboBox::from_id_salt(("dj_loop", d)).width(44.0).selected_text(tt::LOOPS[size.min(tt::LOOPS.len() - 1)]).show_ui(ui, |ui| {
                    for (k, l) in tt::LOOPS.iter().enumerate() {
                        ui.selectable_value(&mut size, k, *l);
                    }
                });
                if size != before {
                    self.dj_set_param(view, tt::LOOP, size as f32);
                }
                let on = view.looping.is_some();
                if ui.selectable_label(on, if on { "Exit" } else { "Loop" }).clicked() {
                    act(self, DeckAct::Loop(!on));
                }
                if ui.small_button("Brake").clicked() {
                    act(self, DeckAct::Brake);
                }
                if ui.small_button("Spin").on_hover_text("A backspin").clicked() {
                    act(self, DeckAct::Backspin);
                }
                if ui.small_button("Load...").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("Audio", &AUDIO).pick_file() {
                        self.dj_load(d, &path);
                    }
                }
                if self.dj.loading[d].is_some() {
                    ui.spinner();
                }
            });
        });
    }

    fn dj_mixer(&mut self, ui: &mut Ui) {
        egui::Frame::new().fill(PANEL2).corner_radius(4.0).inner_margin(6).show(ui, |ui| {
            ui.set_min_height(ui.available_height() - 12.0);
            ui.set_width(ui.available_width());
            let width = ui.available_width() / 3.0 - 4.0;
            ui.horizontal_top(|ui| {
                for c in [0usize, SET, 1] {
                    ui.vertical(|ui| {
                        ui.set_width(width);
                        self.dj_channel(ui, c);
                    });
                }
            });
            let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::click_and_drag());
            let id = Id::new("dj_crossfader");
            let resp = ui.interact(bar, id, Sense::click_and_drag());
            if resp.double_clicked() {
                self.project.dj.crossfader = 0.0;
            } else if resp.dragged() || resp.clicked() {
                if let Some(at) = resp.interact_pointer_pos() {
                    self.project.dj.crossfader = (((at.x - bar.left()) / bar.width()) * 2.0 - 1.0).clamp(-1.0, 1.0);
                }
            }
            rect_fill(ui.painter(), bar.shrink2(vec2(0.0, 7.0)), 2.0, BG);
            let x = bar.left() + (self.project.dj.crossfader + 1.0) * 0.5 * bar.width();
            rect_fill(ui.painter(), Rect::from_center_size(pos2(x, bar.center().y), vec2(16.0, 20.0)), 2.0, Color32::from_gray(210));
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.project.dj.split_cue, "Split").on_hover_text("One stereo output: the music in your left ear, what you cue in the right");
                // How far apart the beats of the two decks are.
                let running = [self.dj_deck(0), self.dj_deck(1)];
                if let [Some(x), Some(y)] = running {
                    if x.running && y.running && x.record.bpm > 0.0 && y.record.bpm > 0.0 {
                        let off = (dj::phase_of(x.pos, x.record.bpm, x.record.beat) - dj::phase_of(y.pos, y.record.bpm, y.record.beat) + 0.5).rem_euclid(1.0) - 0.5;
                        let ms = off * 60_000.0 / x.bpm(self.project.bpm).max(1.0);
                        let col = if ms.abs() < 12.0 { PLAY } else { REC };
                        ui.label(RichText::new(format!("Beats {ms:+.0} ms apart")).small().color(col)).on_hover_text("How far deck A's beats are ahead of deck B's: under about 12 ms nobody hears it. The < and > on a deck nudge it");
                    }
                }
            });
        });
    }

    fn dj_channel(&mut self, ui: &mut Ui, c: usize) {
        let mut ch: DjChannel = self.project.dj.channels[c];
        let before = ch;
        let color = if c == SET { accent_now() } else { DECK_COLOR[c] };
        ui.horizontal(|ui| {
            ui.label(RichText::new(NAMES[c]).strong().color(color));
            if c == SET {
                let mut bpm = self.project.bpm;
                if ui.add(egui::DragValue::new(&mut bpm).range(40.0..=300.0).speed(0.1).max_decimals(2)).on_hover_text("Your song's tempo").changed() {
                    self.project.bpm = bpm;
                }
            }
        });
        let knob_at = |ui: &mut Ui, name: &str, id: (usize, usize), value: &mut f32, min: f32, max: f32, rest: f32, bipolar: bool| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover());
                let norm = (*value - min) / (max - min);
                let (resp, new) = knob(ui, r, Id::new(("dj_knob", id)), norm, (rest - min) / (max - min), bipolar, color);
                if let Some(n) = new {
                    *value = min + n * (max - min);
                }
                resp.on_hover_text(format!("{name}: {:.1}", *value));
                ui.label(RichText::new(name).small().color(TEXT_DIM));
            });
        };
        knob_at(ui, "Trim", (c, 0), &mut ch.trim_db, -24.0, 12.0, 0.0, true);
        knob_at(ui, "High", (c, 1), &mut ch.high_db, KILL_DB, EQ_MAX_DB, 0.0, true);
        knob_at(ui, "Mid", (c, 2), &mut ch.mid_db, KILL_DB, EQ_MAX_DB, 0.0, true);
        knob_at(ui, "Low", (c, 3), &mut ch.low_db, KILL_DB, EQ_MAX_DB, 0.0, true);
        knob_at(ui, "Filter", (c, 4), &mut ch.filter, -1.0, 1.0, 0.0, true);
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(26.0, 74.0), Sense::hover());
            fader(ui, r, Id::new(("dj_fader", c)), &mut ch.fader_db, color);
            let (m, _) = ui.allocate_exact_size(vec2(6.0, 74.0), Sense::hover());
            rect_fill(ui.painter(), m, 2.0, BG);
            let level = self.status().dj_levels[c].load();
            let f = ((20.0 * level.max(1e-5).log10() + 60.0) / 66.0).clamp(0.0, 1.0);
            rect_fill(ui.painter(), Rect::from_min_max(pos2(m.left(), m.bottom() - f * m.height()), m.max), 1.0, if f > 0.95 { REC } else { Color32::from_gray(190) });
            ui.vertical(|ui| {
                if ui.selectable_label(ch.cue, "Cue").on_hover_text("Hear it in the headphones (with Split: in your right ear), before the fader").clicked() {
                    ch.cue = !ch.cue;
                }
                let mut side = ch.side as usize;
                egui::ComboBox::from_id_salt(("dj_side", c)).width(38.0).selected_text(dj::SIDES[side.min(2)]).show_ui(ui, |ui| {
                    for (k, s) in dj::SIDES.iter().enumerate() {
                        ui.selectable_value(&mut side, k, *s);
                    }
                });
                ch.side = side as u8;
                if c == SET {
                    let playing = self.status().playing.load(Ordering::Relaxed);
                    if ui.small_button(if playing { "Stop" } else { "Play" }).on_hover_text("Your song's transport").clicked() {
                        self.cmd(if playing { Command::Stop } else { Command::Play });
                    }
                }
            });
        });
        if ch != before {
            self.project.dj.channels[c] = ch;
        }
    }

    fn dj_library(&mut self, ui: &mut Ui) {
        let folder = self.prefs.dj_folder.clone();
        if self.dj.listed != folder {
            self.dj.files = folder.as_deref().map(audio_files).unwrap_or_default();
            self.dj.listed = folder.clone();
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new("Records").strong());
            if ui.small_button("Folder...").on_hover_text("The folder your records are in").clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    self.prefs.dj_folder = Some(dir);
                    self.prefs.save();
                }
            }
            if let Some(f) = &folder {
                ui.label(RichText::new(f.display().to_string()).small().color(TEXT_DIM));
            }
            if ui.small_button("Back").on_hover_text("Leaves the DJ view; the mixer goes on until you turn it off").clicked() {
                self.view = View::Session;
            }
            if ui.small_button("Mixer off").on_hover_text("The decks go through the song's mixer as ordinary tracks again").clicked() {
                self.project.dj.on = false;
                self.view = View::Session;
            }
        });
        let files = self.dj.files.clone();
        if files.is_empty() {
            ui.label(RichText::new("Choose a folder with your music, or Load... on a deck").small().color(TEXT_DIM));
            return;
        }
        egui::ScrollArea::vertical().id_salt("dj_records").auto_shrink([false, false]).show_rows(ui, 20.0, files.len(), |ui, rows| {
            for k in rows {
                ui.horizontal(|ui| {
                    if ui.small_button(RichText::new("A").color(DECK_COLOR[0])).clicked() {
                        self.dj_load(0, &files[k]);
                    }
                    if ui.small_button(RichText::new("B").color(DECK_COLOR[1])).clicked() {
                        self.dj_load(1, &files[k]);
                    }
                    ui.label(RichText::new(files[k].file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()).small().color(TEXT));
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_lists_its_audio_by_name() {
        let dir = std::env::temp_dir().join(format!("krabton_dj_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for f in ["b.MP3", "a.wav", "notes.txt", "c.flac"] {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        let names: Vec<String> = audio_files(&dir).iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(names, ["a.wav", "b.MP3", "c.flac"]);
    }

    #[test]
    fn the_clock_reads_minutes_and_seconds() {
        assert_eq!(clock(65.4), "1:05");
        assert_eq!(clock(-3.0), "0:00");
    }
}
