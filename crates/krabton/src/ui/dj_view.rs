//! DJ: two decks and your own set on one mixer, laid out as a DJ program is:
//! the two waveforms along the top, each deck with its jog wheel, loop, effect
//! and pads on either side of the mixer, the records along the bottom. The decks
//! are tracks with a Turntable on them; everything else in the project is the
//! set, a channel of its own (and a tab with all its tracks and scenes), so a
//! record can be mixed into the music you are making. See `dsp::dj`.

use super::dj_library::{artist_title, clock, Info, Library};
use crate::app::{App, View};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder};
use krabton_engine::dsp::dj::{self, DjChannel, EQ_MAX_DB, KILL_DB, SET};
use krabton_engine::dsp::effects::EffectKind;
use krabton_engine::dsp::turntable::{self as tt, DeckAct, Record};
use krabton_engine::params::ParamKind;
use krabton_engine::sample::PEAK_BLOCK;
use krabton_engine::{Command, Instrument, SampleRef};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;

const DECK_COLOR: [Color32; 2] = [Color32::from_rgb(0x3f, 0xa7, 0xf0), Color32::from_rgb(0xe8, 0x3a, 0x3a)];
const NAMES: [&str; 3] = ["A", "B", "Set"];
/// A nudge of the record, in seconds.
const NUDGE: f64 = 0.015;
/// The effects a deck can have, in its one slot.
const FX: [EffectKind; 10] = [EffectKind::Echo, EffectKind::Flanger, EffectKind::Phaser, EffectKind::Chorus, EffectKind::AutoFilter, EffectKind::BeatRepeat, EffectKind::Reverb, EffectKind::Rackverb, EffectKind::Tremolo, EffectKind::Vinyl];
const PAD_PAGES: [&str; 4] = ["Hotcues", "Loop Roll", "Scratch", "Beat Jump"];
const JUMPS: [f64; 8] = [-16.0, -8.0, -4.0, -1.0, 1.0, 4.0, 8.0, 16.0];

/// A record heard, ready to go on its deck.
struct Heard {
    path: PathBuf,
    sample: SampleRef,
    record: Record,
}

#[derive(Clone, Copy, PartialEq, Default)]
enum Tab {
    #[default]
    Mixer,
    Tracks,
    Master,
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
    /// The hand on a jog wheel: its last angle and the turns since it took hold.
    jog: [Option<(f32, f64)>; 2],
    tab: Tab,
    pad_page: [usize; 2],
    /// A pad held down: which, for what it has to let go of.
    pad_held: [Option<usize>; 2],
    /// Tap tempo: when the last taps came (seconds).
    taps: [Vec<f64>; 2],
    /// Seconds of waveform shown either side of the needle.
    zoom: f32,
    pub library: Library,
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

/// The tempo of some taps (seconds): their average beat, the last eight.
pub(crate) fn tapped_bpm(taps: &[f64]) -> Option<f64> {
    if taps.len() < 2 {
        return None;
    }
    let from = taps.len().saturating_sub(8);
    let t = &taps[from..];
    let beat = (t[t.len() - 1] - t[0]) / (t.len() - 1) as f64;
    (beat > 0.0).then(|| 60.0 / beat)
}

/// A point on a circle: 0 at the top, going clockwise.
fn on_circle(c: Pos2, r: f32, turns: f32) -> Pos2 {
    let a = turns * std::f32::consts::TAU;
    c + vec2(a.sin(), -a.cos()) * r
}

/// How far the hand went round between two angles (turns), the short way.
pub(crate) fn turned(from: f32, to: f32) -> f32 {
    (to - from + 0.5).rem_euclid(1.0) - 0.5
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

    pub(crate) fn dj_deck_running(&self, d: usize) -> bool {
        self.dj_deck(d).is_some_and(|v| v.running)
    }

    /// Puts a file on a deck, heard on a thread of its own: a song takes a moment.
    pub(crate) fn dj_load(&mut self, d: usize, path: &Path) {
        let path = path.to_path_buf();
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new().name("dj load".into()).spawn(move || {
            let result = SampleRef::load(&path).map(|sample| {
                let mut record = tt::listen(&sample);
                record.key = krabton_engine::analysis::detect_key(&sample).map(|k| (k.root, k.minor));
                Heard { path, sample, record }
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
        self.dj.library.learn(&heard.path, Info { bpm: heard.record.bpm, key: heard.record.key, secs: heard.sample.duration() });
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

    fn dj_record_mut(&mut self, view: &DeckView) -> Option<&mut Record> {
        match &mut self.project.tracks[view.track].instrument {
            Some(Instrument::Turntable { record, .. }) => Some(record),
            _ => None,
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
        if self.dj.zoom <= 0.0 {
            self.dj.zoom = 5.0;
        }
        let decks = [self.dj_deck(0), self.dj_deck(1)];
        let full = ui.available_rect_before_wrap();
        ui.allocate_rect(full, Sense::hover());
        let waves_h = (full.height() * 0.17).clamp(70.0, 150.0);
        let head_h = 62.0;
        let lib_h = (full.height() * 0.24).clamp(100.0, 240.0);
        let mixer_w = 430.0f32.min(full.width() * 0.38);
        let waves = Rect::from_min_size(full.min, vec2(full.width(), waves_h));
        self.dj_waves(ui, waves, &decks);
        let heads_top = waves.bottom() + 3.0;
        let side_w = (full.width() - mixer_w - 8.0) * 0.5;
        let mid_x = full.left() + side_w + 4.0;
        // The tabs of the mixer sit between the two heads.
        let tabs = Rect::from_min_size(pos2(mid_x, heads_top), vec2(mixer_w, head_h));
        for (d, x) in [(0usize, full.left()), (1, mid_x + mixer_w + 4.0)] {
            let head = Rect::from_min_size(pos2(x, heads_top), vec2(side_w, head_h));
            if let Some(view) = &decks[d] {
                self.dj_head(ui, head, d, view, decks[1 - d].as_ref().and_then(|o| o.record.key));
                self.dj_accept_drop(ui, head, d);
            }
        }
        self.dj_tabs(ui, tabs, &decks);
        let row_top = heads_top + head_h + 3.0;
        let row_bottom = full.bottom() - lib_h - 3.0;
        for (d, x) in [(0usize, full.left()), (1, mid_x + mixer_w + 4.0)] {
            let rect = Rect::from_min_max(pos2(x, row_top), pos2(x + side_w, row_bottom));
            if let Some(view) = &decks[d] {
                self.dj_deck_panel(ui, rect, d, view);
                self.dj_accept_drop(ui, rect, d);
            }
        }
        let centre = Rect::from_min_max(pos2(mid_x, row_top), pos2(mid_x + mixer_w, row_bottom));
        let mut sub = ui.new_child(UiBuilder::new().max_rect(centre));
        sub.set_clip_rect(centre);
        match self.dj.tab {
            Tab::Mixer => self.dj_mixer(&mut sub),
            Tab::Tracks => self.dj_tracks(&mut sub, centre.height()),
            Tab::Master => self.map_master(&mut sub, centre.height()),
        }
        let lib = Rect::from_min_max(pos2(full.left(), full.bottom() - lib_h), full.max);
        self.dj_library(ui, lib);
        // What is being dragged, along with the mouse.
        if let (Some(path), Some(at)) = (egui::DragAndDrop::payload::<PathBuf>(ui.ctx()), ui.ctx().pointer_latest_pos()) {
            let name = path.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let p = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, Id::new("dj_drag")));
            p.text(at + vec2(12.0, 4.0), Align2::LEFT_TOP, name, small(), TEXT);
        }
    }

    /// A record dragged onto a deck, from the list or from outside.
    fn dj_accept_drop(&mut self, ui: &mut Ui, rect: Rect, d: usize) {
        let resp = ui.interact(rect, Id::new(("dj_drop", d, rect.top() as i32)), Sense::hover());
        if resp.dnd_hover_payload::<PathBuf>().is_some() {
            ui.painter().rect_stroke(rect, 3.0, Stroke::new(2.0_f32, DECK_COLOR[d]), StrokeKind::Inside);
        }
        if let Some(path) = resp.dnd_release_payload::<PathBuf>() {
            self.dj_load(d, &path);
        }
        if crate::import::file_drop_pos(ui.ctx()).is_some_and(|p| rect.contains(p)) {
            ui.painter().rect_stroke(rect, 3.0, Stroke::new(2.0_f32, DECK_COLOR[d]), StrokeKind::Inside);
            let dropped: Vec<PathBuf> = ui.ctx().input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
            if let Some(path) = dropped.iter().find(|p| krabton_engine::sample::is_audio_file(p)) {
                self.dj_load(d, path);
                crate::import::take_file_drop(ui.ctx());
            }
        }
    }

    /// The two waveforms along the top, beats under beats when they are in sync.
    fn dj_waves(&mut self, ui: &mut Ui, rect: Rect, decks: &[Option<DeckView>; 2]) {
        rect_fill(ui.painter(), rect, 2.0, Color32::from_rgb(0x0e, 0x0e, 0x11));
        let badge = 24.0;
        let area = Rect::from_min_max(pos2(rect.left() + badge, rect.top()), pos2(rect.right() - badge, rect.bottom()));
        let h = area.height() * 0.5;
        // The mouse wheel zooms.
        if ui.ctx().pointer_hover_pos().is_some_and(|p| area.contains(p)) {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.dj.zoom = (self.dj.zoom * (1.0 - scroll * 0.002)).clamp(1.5, 30.0);
            }
        }
        for d in 0..2 {
            let row = Rect::from_min_size(pos2(area.left(), area.top() + d as f32 * h), vec2(area.width(), h));
            if let Some(view) = &decks[d] {
                self.dj_wave(ui, row, d, view);
                self.dj_accept_drop(ui, row, d);
            }
            let side = if d == 0 { rect.left() } else { rect.right() - badge };
            let b = Rect::from_min_size(pos2(side, area.top() + d as f32 * h), vec2(badge, h));
            rect_fill(ui.painter(), b.shrink(1.0), 2.0, if d == 0 { Color32::from_gray(0xdd) } else { DECK_COLOR[1] });
            ui.painter().text(b.center(), Align2::CENTER_CENTER, NAMES[d], FontId::proportional(16.0), if d == 0 { Color32::from_gray(0x18) } else { Color32::WHITE });
        }
        let x = area.center().x;
        ui.painter().line_segment([pos2(x, area.top()), pos2(x, area.bottom())], Stroke::new(2.0_f32, Color32::WHITE));
    }

    /// One deck around the needle, with its beats. Dragging it moves the record by hand.
    fn dj_wave(&mut self, ui: &mut Ui, rect: Rect, d: usize, view: &DeckView) {
        let resp = ui.interact(rect, Id::new(("dj_wave", d)), Sense::click_and_drag());
        let p = ui.painter_at(rect);
        let mid = rect.center();
        let Some(sample) = &view.sample else {
            p.text(mid, Align2::CENTER_CENTER, match self.dj.loading[d] {
                Some(_) => "Listening to the record...".to_string(),
                None => format!("Deck {}: drag a record here", NAMES[d]),
            }, FontId::proportional(12.0), TEXT_DIM);
            return;
        };
        let speed = view.speed(self.project.bpm).abs().max(0.25);
        // At its playing speed, so that two records' beats line up on the screen.
        let px_per_sec = rect.width() / (2.0 * self.dj.zoom) * speed as f32;
        let reach = (rect.width() * 0.5 / px_per_sec) as f64;
        let sr = sample.sample_rate.max(1) as f64;
        let x_of = |secs: f64| mid.x + ((secs - view.pos) as f32) * px_per_sec;
        if let Some((start, len)) = view.looping {
            let shade = Rect::from_min_max(pos2(x_of(start), rect.top()), pos2(x_of(start + len), rect.bottom()));
            p.rect_filled(shade.intersect(rect), 0.0, DECK_COLOR[d].gamma_multiply(0.22));
        }
        if view.record.bpm > 0.0 {
            let beat = 60.0 / view.record.bpm;
            let from = ((view.pos - reach - view.record.beat) / beat).floor() as i64;
            let to = ((view.pos + reach - view.record.beat) / beat).ceil() as i64;
            for k in from..=to {
                let x = x_of(view.record.beat + k as f64 * beat);
                let bar = k.rem_euclid(4) == 0;
                p.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(if bar { 1.5_f32 } else { 1.0 }, Color32::from_gray(if bar { 150 } else { 52 })));
            }
        }
        let peaks_per_sec = sr / PEAK_BLOCK as f64;
        let halfh = rect.height() * 0.5 - 1.0;
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

    /// A deck's head: what is on it, the whole record small, its tempo and key.
    fn dj_head(&mut self, ui: &mut Ui, rect: Rect, d: usize, view: &DeckView, other_key: Option<(u8, bool)>) {
        rect_fill(ui.painter(), rect, 2.0, PANEL2);
        let mirrored = d == 1;
        let disc = rect.height() - 8.0;
        let disc_rect = if mirrored { Rect::from_min_size(pos2(rect.right() - disc - 4.0, rect.top() + 4.0), vec2(disc, disc)) } else { Rect::from_min_size(rect.min + vec2(4.0, 4.0), vec2(disc, disc)) };
        rect_fill(ui.painter(), disc_rect, 2.0, Color32::from_gray(0x16));
        ui.painter().circle_filled(disc_rect.center(), disc * 0.42, Color32::from_gray(0x08));
        ui.painter().circle_filled(disc_rect.center(), disc * 0.16, DECK_COLOR[d]);
        let numbers_w = 150.0f32.min(rect.width() * 0.4);
        let (info, numbers) = if mirrored {
            (Rect::from_min_max(pos2(rect.left() + numbers_w + 4.0, rect.top() + 3.0), pos2(disc_rect.left() - 6.0, rect.bottom() - 3.0)), Rect::from_min_size(rect.min + vec2(4.0, 3.0), vec2(numbers_w, rect.height() - 6.0)))
        } else {
            (Rect::from_min_max(pos2(disc_rect.right() + 6.0, rect.top() + 3.0), pos2(rect.right() - numbers_w - 4.0, rect.bottom() - 3.0)), Rect::from_min_size(pos2(rect.right() - numbers_w - 4.0, rect.top() + 3.0), vec2(numbers_w, rect.height() - 6.0)))
        };
        let align = if mirrored { Align2::RIGHT_TOP } else { Align2::LEFT_TOP };
        let tx = if mirrored { info.right() } else { info.left() };
        let (artist, title) = match &view.sample {
            Some(s) => s.path.as_deref().map_or((String::new(), s.name.clone()), artist_title),
            None => (String::new(), "No record".to_string()),
        };
        let p = ui.painter_at(info);
        p.text(pos2(tx, info.top()), align, title, FontId::proportional(14.0), TEXT);
        p.text(pos2(tx, info.top() + 17.0), align, artist, FontId::proportional(11.0), DECK_COLOR[d]);
        // The whole record, small: click to drop the needle there.
        let over = Rect::from_min_max(pos2(info.left(), info.bottom() - 22.0), info.max);
        self.dj_overview(ui, over, d, view);
        // Its tempo and key.
        let song_bpm = self.project.bpm;
        let mut sub = ui.new_child(UiBuilder::new().max_rect(numbers));
        sub.set_clip_rect(numbers);
        sub.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.horizontal(|ui| {
                if view.record.bpm > 0.0 {
                    ui.label(RichText::new(format!("{:.2}", view.bpm(song_bpm))).size(19.0).color(DECK_COLOR[d]));
                } else {
                    ui.label(RichText::new("--").size(19.0).color(TEXT_DIM));
                }
                if let Some(key) = view.record.key {
                    let fits = other_key.is_some_and(|o| dj::keys_fit(key, o));
                    let col = if other_key.is_none() { TEXT } else if fits { PLAY } else { REC };
                    ui.label(RichText::new(dj::wheel_name(key.0, key.1)).size(14.0).color(col)).on_hover_text("Its key on the wheel: green mixes with the other deck, red clashes");
                }
            });
            ui.horizontal(|ui| {
                if ui.small_button("TAP").on_hover_text("Tap along if the tempo was heard wrong").clicked() {
                    self.dj_tap(d, view);
                }
                if ui.small_button("/2").on_hover_text("It is half this tempo").clicked() {
                    if let Some(r) = self.dj_record_mut(view) {
                        r.bpm *= 0.5;
                    }
                }
                if ui.small_button("x2").on_hover_text("It is twice this tempo").clicked() {
                    if let Some(r) = self.dj_record_mut(view) {
                        r.bpm *= 2.0;
                    }
                }
                let pos = view.pos;
                if ui.small_button("Grid").on_hover_text("The first beat is here: the beats on the waveform count from this").clicked() {
                    if let Some(r) = self.dj_record_mut(view) {
                        r.beat = pos;
                    }
                }
            });
            if let Some(s) = &view.sample {
                let pct = (view.speed(song_bpm) - 1.0) * 100.0;
                ui.label(RichText::new(format!("-{}  of {}   {pct:+.1} %", clock(s.duration() - view.pos), clock(s.duration()))).small().color(TEXT_DIM));
            }
        });
    }

    fn dj_tap(&mut self, d: usize, view: &DeckView) {
        let now = ui_now();
        let taps = &mut self.dj.taps[d];
        if taps.last().is_some_and(|t| now - t > 2.5) {
            taps.clear();
        }
        taps.push(now);
        if let Some(bpm) = tapped_bpm(&self.dj.taps[d]) {
            if let Some(r) = self.dj_record_mut(view) {
                r.bpm = (bpm * 100.0).round() / 100.0;
            }
        }
    }

    /// The whole record, small.
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
                let played = (x as f64 / cols as f64) * dur < view.pos;
                p.line_segment([pos2(fx, rect.center().y - h), pos2(fx, rect.center().y + h)], Stroke::new(1.0_f32, if played { DECK_COLOR[d].gamma_multiply(0.35) } else { DECK_COLOR[d] }));
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

    /// A deck: loop, effect and pads on the outside, the jog wheel and its
    /// buttons in the middle, the pitch fader next to the mixer.
    fn dj_deck_panel(&mut self, ui: &mut Ui, rect: Rect, d: usize, view: &DeckView) {
        rect_fill(ui.painter(), rect, 2.0, PANEL2);
        let mirrored = d == 1;
        let pitch_w = 44.0;
        let outer_w = (rect.width() * 0.44).clamp(150.0, 250.0);
        let (outer, jog, pitch) = if mirrored {
            (
                Rect::from_min_max(pos2(rect.right() - outer_w, rect.top()), rect.max),
                Rect::from_min_max(pos2(rect.left() + pitch_w, rect.top()), pos2(rect.right() - outer_w, rect.bottom())),
                Rect::from_min_size(rect.min, vec2(pitch_w, rect.height())),
            )
        } else {
            (
                Rect::from_min_size(rect.min, vec2(outer_w, rect.height())),
                Rect::from_min_max(pos2(rect.left() + outer_w, rect.top()), pos2(rect.right() - pitch_w, rect.bottom())),
                Rect::from_min_max(pos2(rect.right() - pitch_w, rect.top()), rect.max),
            )
        };
        let mut sub = ui.new_child(UiBuilder::new().max_rect(outer.shrink(4.0)));
        sub.set_clip_rect(outer);
        self.dj_outer(&mut sub, d, view);
        self.dj_jog_column(ui, jog, d, view);
        self.dj_pitch(ui, pitch, view);
    }

    fn dj_outer(&mut self, ui: &mut Ui, d: usize, view: &DeckView) {
        ui.columns(2, |cols| {
            // The loop.
            let ui = &mut cols[0];
            ui.label(RichText::new("LOOP").small().color(TEXT_DIM));
            ui.horizontal(|ui| {
                let size = (view.params[tt::LOOP].round() as usize).min(tt::LOOPS.len() - 1);
                if ui.small_button("-").clicked() && size > 0 {
                    self.dj_set_param(view, tt::LOOP, (size - 1) as f32);
                }
                ui.label(RichText::new(tt::LOOPS[size]).color(TEXT));
                if ui.small_button("+").clicked() && size + 1 < tt::LOOPS.len() {
                    self.dj_set_param(view, tt::LOOP, (size + 1) as f32);
                }
            });
            let on = view.looping.is_some();
            if ui.selectable_label(on, if on { "Exit loop" } else { "Loop" }).on_hover_text("So many beats, round and round from here").clicked() {
                self.cmd(Command::Deck { track: view.id, act: DeckAct::Loop(!on) });
            }
            // The effect.
            let ui = &mut cols[1];
            ui.label(RichText::new("EFFECT").small().color(TEXT_DIM));
            self.dj_effect(ui, d, view);
        });
        ui.add_space(2.0);
        // The pads.
        ui.horizontal(|ui| {
            ui.label(RichText::new("PADS").small().color(TEXT_DIM));
            let mut page = self.dj.pad_page[d];
            egui::ComboBox::from_id_salt(("dj_pad_page", d)).width(82.0).selected_text(PAD_PAGES[page]).show_ui(ui, |ui| {
                for (k, name) in PAD_PAGES.iter().enumerate() {
                    ui.selectable_value(&mut page, k, *name);
                }
            });
            self.dj.pad_page[d] = page;
        });
        let avail = ui.available_size();
        let (grid, _) = ui.allocate_exact_size(vec2(avail.x, avail.y.max(40.0)), Sense::hover());
        self.dj_pads(ui, grid, d, view);
    }

    /// The deck's one effect: the first device of its track.
    fn dj_effect(&mut self, ui: &mut Ui, d: usize, view: &DeckView) {
        let (now_kind, enabled) = self.project.tracks[view.track].devices.first().map_or((None, false), |dev| (Some(dev.kind), dev.enabled));
        let mut pick = now_kind;
        egui::ComboBox::from_id_salt(("dj_fx", d)).width(80.0).selected_text(now_kind.map_or("None", |k| k.name())).show_ui(ui, |ui| {
            ui.selectable_value(&mut pick, None, "None");
            for k in FX {
                ui.selectable_value(&mut pick, Some(k), k.name());
            }
        });
        if pick != now_kind {
            let devices_empty = self.project.tracks[view.track].devices.is_empty();
            match pick {
                Some(k) => {
                    let mut dev = self.project.make_device(k);
                    dev.enabled = enabled || now_kind.is_none();
                    let devices = &mut self.project.tracks[view.track].devices;
                    if devices_empty {
                        devices.push(dev);
                    } else {
                        devices[0] = dev;
                    }
                }
                None => {
                    if !devices_empty {
                        self.project.tracks[view.track].devices.remove(0);
                    }
                }
            }
            return;
        }
        let Some(kind) = now_kind else { return };
        let specs = kind.specs();
        let first = crate::move_knobs::banks(specs, &[]);
        let knobs: Vec<usize> = first.first().map(|b| b.knobs.iter().copied().filter(|&i| matches!(specs[i].kind, ParamKind::Linear | ParamKind::Log)).take(2).collect()).unwrap_or_default();
        ui.horizontal(|ui| {
            if ui.selectable_label(enabled, "On").clicked() {
                if let Some(dev) = self.project.tracks[view.track].devices.first_mut() {
                    dev.enabled = !enabled;
                }
            }
            for k in knobs {
                let v = self.project.tracks[view.track].devices[0].params.get(k).copied().unwrap_or(specs[k].default);
                let (r, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
                let (resp, new) = knob(ui, r, Id::new(("dj_fx_knob", d, k)), specs[k].to_norm(v), specs[k].to_norm(specs[k].default), false, DECK_COLOR[d]);
                resp.on_hover_text(format!("{}: {}", specs[k].name, specs[k].format(v)));
                if let Some(n) = new {
                    if let Some(p) = self.project.tracks[view.track].devices[0].params.get_mut(k) {
                        *p = specs[k].from_norm(n);
                    }
                }
            }
        });
    }

    /// Eight pads, doing what their page says.
    fn dj_pads(&mut self, ui: &mut Ui, rect: Rect, d: usize, view: &DeckView) {
        let page = self.dj.pad_page[d];
        let (w, h) = (rect.width() / 4.0, rect.height() / 2.0);
        let mut held = None;
        let before = self.dj.pad_held[d];
        for k in 0..8 {
            let r = Rect::from_min_size(rect.min + vec2((k % 4) as f32 * w, (k / 4) as f32 * h), vec2(w, h)).shrink(2.0);
            let resp = ui.interact(r, Id::new(("dj_pad", d, k)), Sense::click_and_drag());
            let down = resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down());
            if down {
                held = Some(k);
            }
            let (label, lit): (String, bool) = match page {
                0 => (format!("{}", k + 1), view.record.cues[k].is_some()),
                1 => (tt::LOOPS.get(k).map_or(String::new(), |s| (*s).to_string()), k < tt::LOOPS.len()),
                2 => (tt::MOVES.get(k).map_or(if k == 6 { "Cut".to_string() } else { "Spin".to_string() }, |s| (*s).to_string()), true),
                _ => (format!("{:+}", JUMPS[k]), view.record.bpm > 0.0),
            };
            let fill = if down { DECK_COLOR[d] } else if lit { Color32::from_gray(0x3c) } else { Color32::from_gray(0x24) };
            rect_fill(ui.painter(), r, 2.0, fill);
            ui.painter().with_clip_rect(r).text(r.center(), Align2::CENTER_CENTER, label, small(), if down { Color32::WHITE } else if lit { TEXT } else { TEXT_DIM });
            // What a click does: jumps, and a hot cue set by its other button.
            if resp.clicked() {
                match page {
                    0 => {
                        if let Some(secs) = view.record.cues[k] {
                            self.cmd(Command::Deck { track: view.id, act: DeckAct::Jump { secs, play: true } });
                        }
                    }
                    3 if view.record.bpm > 0.0 => {
                        let secs = (view.pos + JUMPS[k] * 60.0 / view.record.bpm).max(0.0);
                        self.cmd(Command::Deck { track: view.id, act: DeckAct::Jump { secs, play: false } });
                    }
                    _ => {}
                }
            }
            if page == 0 && resp.secondary_clicked() {
                let pos = view.pos;
                if let Some(rec) = self.dj_record_mut(view) {
                    rec.cues[k] = Some(pos);
                }
            }
            if page == 0 {
                resp.on_hover_text("Click: jump there and play. Right-click: this is the cue now");
            }
        }
        // What is played while a pad is held.
        if held != before {
            if let Some(k) = before {
                self.dj_pad_up(page, k, view);
            }
            if let Some(k) = held {
                self.dj_pad_down(page, k, view);
            }
            self.dj.pad_held[d] = held;
        }
    }

    fn dj_pad_down(&mut self, page: usize, k: usize, view: &DeckView) {
        match page {
            1 if k < tt::LOOPS.len() => {
                self.dj_set_param(view, tt::LOOP, k as f32);
                self.cmd(Command::NoteOn { track: view.id, pitch: tt::ROLL_NOTE, velocity: 1.0 });
            }
            2 if k < tt::MOVES.len() => self.cmd(Command::Deck { track: view.id, act: DeckAct::Move(k, true) }),
            2 if k == 6 => self.cmd(Command::Deck { track: view.id, act: DeckAct::Cut(true) }),
            2 => self.cmd(Command::Deck { track: view.id, act: DeckAct::Backspin }),
            _ => {}
        }
    }

    fn dj_pad_up(&mut self, page: usize, k: usize, view: &DeckView) {
        match page {
            1 if k < tt::LOOPS.len() => self.cmd(Command::NoteOff { track: view.id, pitch: tt::ROLL_NOTE }),
            2 if k < tt::MOVES.len() => self.cmd(Command::Deck { track: view.id, act: DeckAct::Move(k, false) }),
            2 if k == 6 => self.cmd(Command::Deck { track: view.id, act: DeckAct::Cut(false) }),
            _ => {}
        }
    }

    /// The jog wheel, and under it Cue, pause, play and Sync.
    fn dj_jog_column(&mut self, ui: &mut Ui, rect: Rect, d: usize, view: &DeckView) {
        let buttons_h = 28.0;
        let wheel = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - buttons_h - 2.0));
        self.dj_jog(ui, wheel, d, view);
        let row = Rect::from_min_max(pos2(rect.left() + 3.0, rect.bottom() - buttons_h), pos2(rect.right() - 3.0, rect.bottom() - 2.0));
        let w = row.width() / 4.0;
        let synced = view.params[tt::SYNC] >= 0.5;
        let master = self.dj.master == Some(d);
        for k in 0..4 {
            let r = Rect::from_min_size(row.min + vec2(k as f32 * w, 0.0), vec2(w, row.height())).shrink(1.5);
            let resp = ui.interact(r, Id::new(("dj_transport", d, k)), Sense::click());
            let (label, on, tip) = match k {
                0 => ("CUE", false, "Stopped: this is where it goes back to. Playing: back there, and it stops"),
                1 => ("||", !view.running, "Pause"),
                2 => (">", view.running, "Play"),
                _ => ("SYNC", synced, "Plays at the tempo of the other deck (or your song) and lines up the beats. Right-click: Master, this deck gives the song its tempo"),
            };
            let fill = if on { if k == 2 { DECK_COLOR[d] } else { Color32::from_gray(0x5a) } } else if resp.hovered() { Color32::from_gray(0x44) } else { Color32::from_gray(0x34) };
            rect_fill(ui.painter(), r, 2.0, fill);
            ui.painter().text(r.center(), Align2::CENTER_CENTER, label, FontId::proportional(11.0), Color32::WHITE);
            if k == 3 && master {
                ui.painter().rect_stroke(r, 2.0, Stroke::new(2.0_f32, DECK_COLOR[d]), StrokeKind::Inside);
            }
            let secondary = resp.secondary_clicked();
            if resp.on_hover_text(tip).clicked() {
                match k {
                    0 => {
                        if view.running {
                            let secs = self.dj.cue[d];
                            self.cmd(Command::Deck { track: view.id, act: DeckAct::StartStop });
                            self.cmd(Command::Deck { track: view.id, act: DeckAct::Jump { secs, play: false } });
                        } else {
                            self.dj.cue[d] = view.pos;
                        }
                    }
                    1 => {
                        if view.running {
                            self.cmd(Command::Deck { track: view.id, act: DeckAct::StartStop });
                        }
                    }
                    2 => {
                        if !view.running {
                            self.cmd(Command::Deck { track: view.id, act: DeckAct::StartStop });
                        }
                    }
                    _ => self.dj_sync(d, view),
                }
            }
            if k == 3 && secondary {
                self.dj.master = if master { None } else { Some(d) };
                if !master && synced {
                    self.dj_set_param(view, tt::SYNC, 0.0);
                }
            }
        }
    }

    /// The wheel: it turns as the record does, and the hand on it moves the record.
    fn dj_jog(&mut self, ui: &mut Ui, rect: Rect, d: usize, view: &DeckView) {
        let r = (rect.width().min(rect.height()) * 0.5 - 5.0).max(16.0);
        let c = rect.center();
        let p = ui.painter_at(rect);
        let turn = (tt::turn_secs(&view.params)) as f32;
        let angle = (view.pos as f32 / turn.max(0.1)).rem_euclid(1.0);
        p.circle_filled(c, r, Color32::from_gray(0x24));
        p.circle_stroke(c, r, Stroke::new(1.0_f32, Color32::from_gray(0x4a)));
        // How far the record has played, round the edge.
        let dur = view.sample.as_ref().map_or(0.0, |s| s.duration());
        if dur > 0.0 {
            let part = (view.pos / dur).clamp(0.0, 1.0) as f32;
            let pts: Vec<Pos2> = (0..=64).map(|k| on_circle(c, r - 3.0, part * k as f32 / 64.0)).collect();
            p.add(Shape::line(pts, Stroke::new(4.0_f32, DECK_COLOR[d])));
        }
        p.circle_filled(c, r * 0.84, Color32::from_gray(0x12));
        for k in 0..3 {
            p.circle_stroke(c, r * (0.45 + 0.13 * k as f32), Stroke::new(1.0_f32, Color32::from_gray(0x1c)));
        }
        p.line_segment([on_circle(c, r * 0.22, angle), on_circle(c, r * 0.8, angle)], Stroke::new(3.0_f32, DECK_COLOR[d]));
        p.circle_filled(c, r * 0.2, DECK_COLOR[d]);
        p.circle_filled(c, r * 0.04, Color32::WHITE);
        // The hand.
        let resp = ui.interact(Rect::from_center_size(c, vec2(r * 2.0, r * 2.0)), Id::new(("dj_jog", d)), Sense::click_and_drag());
        let angle_at = |at: Pos2| -> f32 {
            let v = at - c;
            v.x.atan2(-v.y) / std::f32::consts::TAU
        };
        if resp.drag_started() {
            if let Some(at) = resp.interact_pointer_pos() {
                self.dj.jog[d] = Some((angle_at(at), 0.0));
            }
        }
        if resp.dragged() {
            if let (Some((last, total)), Some(at)) = (self.dj.jog[d], resp.interact_pointer_pos()) {
                let a = angle_at(at);
                let total = total + turned(last, a) as f64;
                self.dj.jog[d] = Some((a, total));
                self.cmd(Command::Deck { track: view.id, act: DeckAct::Hand(Some(total)) });
            }
        }
        if resp.drag_stopped() {
            self.dj.jog[d] = None;
            self.cmd(Command::Deck { track: view.id, act: DeckAct::Hand(None) });
        }
        resp.on_hover_text("Turn it to move the record by hand");
    }

    /// The pitch fader, with its range and nudges.
    fn dj_pitch(&mut self, ui: &mut Ui, rect: Rect, view: &DeckView) {
        let pct = (view.speed(self.project.bpm) - 1.0) * 100.0;
        ui.painter().text(pos2(rect.center().x, rect.top() + 3.0), Align2::CENTER_TOP, format!("{pct:+.1}"), FontId::proportional(11.0), TEXT);
        let mut sub = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(pos2(rect.left() + 8.0, rect.top() + 18.0), pos2(rect.right() - 6.0, rect.bottom() - 54.0))));
        let mut pitch = view.params[tt::PITCH];
        let free = view.params[tt::SYNC] < 0.5;
        let h = (rect.height() - 76.0).max(30.0);
        sub.spacing_mut().slider_width = h;
        if sub.add_enabled(free, egui::Slider::new(&mut pitch, -1.0..=1.0).vertical().show_value(false)).on_hover_text("The pitch fader: near the middle it snaps to it").changed() {
            self.dj_set_param(view, tt::PITCH, if pitch.abs() < 0.02 { 0.0 } else { pitch });
        }
        let row = Rect::from_min_max(pos2(rect.left() + 2.0, rect.bottom() - 52.0), pos2(rect.right() - 2.0, rect.bottom() - 2.0));
        let mut sub = ui.new_child(UiBuilder::new().max_rect(row));
        sub.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let range = (view.params[tt::RANGE].round() as usize).min(tt::RANGES.len() - 1);
            if ui.small_button(RichText::new(tt::RANGES[range]).small()).on_hover_text("How far the pitch fader goes").clicked() {
                self.dj_set_param(view, tt::RANGE, ((range + 1) % tt::RANGES.len()) as f32);
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if ui.small_button("<").on_hover_text("Nudge back").clicked() {
                    self.cmd(Command::Deck { track: view.id, act: DeckAct::Jump { secs: (view.pos - NUDGE).max(0.0), play: false } });
                }
                if ui.small_button(">").on_hover_text("Nudge on").clicked() {
                    self.cmd(Command::Deck { track: view.id, act: DeckAct::Jump { secs: view.pos + NUDGE, play: false } });
                }
            });
            let lock = view.params[tt::KEY_LOCK] >= 0.5;
            if ui.selectable_label(lock, RichText::new("Key").small()).on_hover_text("Key Lock: keeps the pitch while the tempo changes (a simple stretch: you can hear it on big changes)").clicked() {
                self.dj_set_param(view, tt::KEY_LOCK, if lock { 0.0 } else { 1.0 });
            }
        });
    }

    /// Mixer, tracks or master, between the two decks.
    fn dj_tabs(&mut self, ui: &mut Ui, rect: Rect, decks: &[Option<DeckView>; 2]) {
        rect_fill(ui.painter(), rect, 2.0, PANEL2);
        let mut sub = ui.new_child(UiBuilder::new().max_rect(rect.shrink(4.0)));
        sub.horizontal(|ui| {
            for (tab, name, tip) in [(Tab::Mixer, "MIXER", "A, your set and B"), (Tab::Tracks, "TRACKS", "The volume and sends of all your tracks, and your scenes"), (Tab::Master, "MASTER", "The master and how loud it comes out")] {
                if ui.selectable_label(self.dj.tab == tab, name).on_hover_text(tip).clicked() {
                    self.dj.tab = tab;
                }
            }
            if ui.small_button("Back").on_hover_text("Leaves the DJ view; the mixer goes on until you turn it off").clicked() {
                self.view = View::Session;
            }
            if ui.small_button("Off").on_hover_text("Turns the DJ mixer off: the decks go through the song's mixer as ordinary tracks again").clicked() {
                self.project.dj.on = false;
                self.view = View::Session;
            }
        });
        // How far apart the beats are, and the tempo of the song.
        sub.horizontal(|ui| {
            let mut bpm = self.project.bpm;
            ui.label(RichText::new("Song").small().color(TEXT_DIM));
            if ui.add(egui::DragValue::new(&mut bpm).range(40.0..=300.0).speed(0.1).max_decimals(2)).on_hover_text("Your song's tempo").changed() {
                self.project.bpm = bpm;
            }
            let playing = self.status().playing.load(Ordering::Relaxed);
            if ui.small_button(if playing { "Stop" } else { "Play" }).on_hover_text("Your song's transport").clicked() {
                self.cmd(if playing { Command::Stop } else { Command::Play });
            }
            if let [Some(x), Some(y)] = decks {
                if x.running && y.running && x.record.bpm > 0.0 && y.record.bpm > 0.0 {
                    let off = (dj::phase_of(x.pos, x.record.bpm, x.record.beat) - dj::phase_of(y.pos, y.record.bpm, y.record.beat) + 0.5).rem_euclid(1.0) - 0.5;
                    let ms = off * 60_000.0 / x.bpm(self.project.bpm).max(1.0);
                    let col = if ms.abs() < 12.0 { PLAY } else { REC };
                    ui.label(RichText::new(format!("Beats {ms:+.0} ms apart")).small().color(col)).on_hover_text("How far deck A's beats are ahead of deck B's: under about 12 ms nobody hears it. The < and > on a deck nudge it");
                }
            }
        });
    }

    fn dj_mixer(&mut self, ui: &mut Ui) {
        egui::Frame::new().fill(PANEL2).corner_radius(2.0).inner_margin(6).show(ui, |ui| {
            ui.set_min_height(ui.available_height() - 12.0);
            ui.set_width(ui.available_width());
            let width = ui.available_width() / 3.0 - 4.0;
            // What the knobs and the crossfader leave goes to the faders.
            let fader_h = (ui.available_height() - 215.0).clamp(50.0, 320.0);
            ui.horizontal_top(|ui| {
                for c in [0usize, SET, 1] {
                    ui.vertical(|ui| {
                        ui.set_width(width);
                        self.dj_channel(ui, c, fader_h);
                    });
                }
            });
            let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
            let resp = ui.interact(bar, Id::new("dj_crossfader"), Sense::click_and_drag());
            if resp.double_clicked() {
                self.project.dj.crossfader = 0.0;
            } else if resp.dragged() || resp.clicked() {
                if let Some(at) = resp.interact_pointer_pos() {
                    self.project.dj.crossfader = (((at.x - bar.left()) / bar.width()) * 2.0 - 1.0).clamp(-1.0, 1.0);
                }
            }
            let track = bar.shrink2(vec2(0.0, 8.0));
            let x = bar.left() + (self.project.dj.crossfader + 1.0) * 0.5 * bar.width();
            rect_fill(ui.painter(), Rect::from_min_max(track.min, pos2(x, track.bottom())), 2.0, DECK_COLOR[0]);
            rect_fill(ui.painter(), Rect::from_min_max(pos2(x, track.top()), track.max), 2.0, DECK_COLOR[1]);
            rect_fill(ui.painter(), Rect::from_center_size(pos2(x, bar.center().y), vec2(16.0, 22.0)), 2.0, Color32::from_gray(0xd8));
            ui.checkbox(&mut self.project.dj.split_cue, "Split").on_hover_text("One stereo output: the music in your left ear, what you cue in the right. Double-click the crossfader for its middle");
        });
    }

    fn dj_channel(&mut self, ui: &mut Ui, c: usize, fader_h: f32) {
        let mut ch: DjChannel = self.project.dj.channels[c];
        let before = ch;
        let color = if c == SET { accent_now() } else { DECK_COLOR[c] };
        ui.label(RichText::new(NAMES[c]).strong().color(color));
        let knob_at = |ui: &mut Ui, name: &str, id: (usize, usize), value: &mut f32, min: f32, max: f32, rest: f32| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover());
                let norm = (*value - min) / (max - min);
                let (resp, new) = knob(ui, r, Id::new(("dj_knob", id)), norm, (rest - min) / (max - min), true, color);
                if let Some(n) = new {
                    *value = min + n * (max - min);
                }
                resp.on_hover_text(format!("{name}: {:.1}", *value));
                ui.label(RichText::new(name).small().color(TEXT_DIM));
            });
        };
        ui.spacing_mut().item_spacing.y = 1.0;
        knob_at(ui, "Gain", (c, 0), &mut ch.trim_db, -24.0, 12.0, 0.0);
        knob_at(ui, "High", (c, 1), &mut ch.high_db, KILL_DB, EQ_MAX_DB, 0.0);
        knob_at(ui, "Mid", (c, 2), &mut ch.mid_db, KILL_DB, EQ_MAX_DB, 0.0);
        knob_at(ui, "Low", (c, 3), &mut ch.low_db, KILL_DB, EQ_MAX_DB, 0.0);
        knob_at(ui, "Filter", (c, 4), &mut ch.filter, -1.0, 1.0, 0.0);
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(26.0, fader_h), Sense::hover());
            fader(ui, r, Id::new(("dj_fader", c)), &mut ch.fader_db, color);
            let (m, _) = ui.allocate_exact_size(vec2(8.0, fader_h), Sense::hover());
            rect_fill(ui.painter(), m, 2.0, BG);
            let level = self.status().dj_levels[c].load();
            let f = ((20.0 * level.max(1e-5).log10() + 60.0) / 66.0).clamp(0.0, 1.0);
            let bar = Rect::from_min_max(pos2(m.left(), m.bottom() - f * m.height()), m.max);
            rect_fill(ui.painter(), bar, 1.0, if f > 0.92 { REC } else if f > 0.75 { Color32::from_rgb(0xe8, 0xc2, 0x3a) } else { Color32::from_rgb(0x4c, 0xc8, 0x6a) });
            ui.vertical(|ui| {
                if ui.selectable_label(ch.cue, "Cue").on_hover_text("Hear it in the headphones (with Split: in your right ear), before the fader").clicked() {
                    ch.cue = !ch.cue;
                }
                let mut side = ch.side as usize;
                egui::ComboBox::from_id_salt(("dj_side", c)).width(40.0).selected_text(dj::SIDES[side.min(2)]).show_ui(ui, |ui| {
                    for (k, s) in dj::SIDES.iter().enumerate() {
                        ui.selectable_value(&mut side, k, *s);
                    }
                });
                ch.side = side as u8;
            });
        });
        if ch != before {
            self.project.dj.channels[c] = ch;
        }
    }

    /// All the tracks of your set: volume, mute, solo, sends; your scenes beside them.
    fn dj_tracks(&mut self, ui: &mut Ui, h: f32) {
        let decks = self.project.dj.decks;
        let strip_h = (h - 8.0).max(120.0);
        ui.horizontal_top(|ui| {
            // The scenes.
            ui.vertical(|ui| {
                ui.set_width(70.0);
                ui.label(RichText::new("Scenes").small().color(TEXT_DIM));
                egui::ScrollArea::vertical().id_salt("dj_scenes").max_height(strip_h - 20.0).show(ui, |ui| {
                    for k in 0..self.project.scenes.len() {
                        let name = self.project.scenes[k].name.clone();
                        let label = if name.is_empty() { format!("{}", k + 1) } else { name };
                        if ui.add_sized(vec2(64.0, 20.0), egui::Button::new(RichText::new(label).small())).on_hover_text("Starts this scene of your set").clicked() {
                            self.cmd(Command::LaunchScene(k));
                        }
                    }
                });
            });
            egui::ScrollArea::horizontal().id_salt("dj_track_strips").auto_shrink([false, false]).show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    let names: Vec<String> = self.project.returns.iter().map(|r| r.name.clone()).collect();
                    for i in 0..self.project.tracks.len() {
                        if decks.contains(&Some(self.project.tracks[i].id)) {
                            continue;
                        }
                        self.dj_track_strip(ui, i, &names, strip_h);
                    }
                });
            });
        });
    }

    fn dj_track_strip(&mut self, ui: &mut Ui, i: usize, returns: &[String], h: f32) {
        let level = self.meters.get(i).copied().unwrap_or((0.0, 0.0));
        let t = &mut self.project.tracks[i];
        let color = Color32::from_rgb(t.color[0], t.color[1], t.color[2]);
        egui::Frame::new().fill(PANEL3).corner_radius(3.0).inner_margin(3).show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(58.0);
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.add(egui::Label::new(RichText::new(t.name.as_str()).small().color(color)).truncate());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if ui.selectable_label(t.mixer.mute, RichText::new("M").small()).on_hover_text("Mute").clicked() {
                        t.mixer.mute = !t.mixer.mute;
                    }
                    if ui.selectable_label(t.mixer.solo, RichText::new("S").small()).on_hover_text("Solo").clicked() {
                        t.mixer.solo = !t.mixer.solo;
                    }
                });
                // The sends.
                let sends = returns.len().min(4);
                for k in 0..sends {
                    if t.mixer.sends.len() <= k {
                        t.mixer.sends.resize(k + 1, 0.0);
                    }
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                        let (resp, new) = knob(ui, r, Id::new(("dj_send", i, k)), t.mixer.sends[k], 0.0, false, color);
                        resp.on_hover_text(format!("Send to {}: {:.0} %", returns[k], t.mixer.sends[k] * 100.0));
                        if let Some(n) = new {
                            t.mixer.sends[k] = n;
                        }
                        ui.label(RichText::new(format!("{}", (b'A' + k as u8) as char)).small().color(TEXT_DIM));
                    });
                }
                // The fader and its meter: what is left.
                let fader_h = (h - 74.0 - sends as f32 * 21.0).max(30.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let (r, _) = ui.allocate_exact_size(vec2(24.0, fader_h), Sense::hover());
                    fader(ui, r, Id::new(("dj_track_fader", i)), &mut t.mixer.volume_db, color);
                    let (m, _) = ui.allocate_exact_size(vec2(8.0, fader_h), Sense::hover());
                    rect_fill(ui.painter(), m, 2.0, BG);
                    for (k, v) in [level.0, level.1].iter().enumerate() {
                        let f = ((20.0 * v.max(1e-5).log10() + 60.0) / 66.0).clamp(0.0, 1.0);
                        let x = m.left() + k as f32 * 4.0;
                        rect_fill(ui.painter(), Rect::from_min_max(pos2(x, m.bottom() - f * m.height()), pos2(x + 3.5, m.bottom())), 1.0, if f > 0.92 { REC } else { Color32::from_gray(190) });
                    }
                });
                ui.label(RichText::new(format!("{:+.1}", t.mixer.volume_db)).small().color(TEXT_DIM));
            });
        });
    }
}

/// Seconds since the program started, for tap tempo.
fn ui_now() -> f64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tapping_tells_the_tempo_of_the_last_taps() {
        assert_eq!(tapped_bpm(&[1.0]), None);
        let taps: Vec<f64> = (0..5).map(|k| k as f64 * 0.5).collect();
        assert!((tapped_bpm(&taps).unwrap() - 120.0).abs() < 1e-6);
        // Only the last eight count.
        let mut long: Vec<f64> = (0..20).map(|k| k as f64 * 1.0).collect();
        let end = *long.last().unwrap();
        long.extend((1..=8).map(|k| end + k as f64 * 0.4));
        assert!((tapped_bpm(&long).unwrap() - 150.0).abs() < 1e-6);
    }

    #[test]
    fn the_hand_on_the_wheel_goes_the_short_way_round() {
        assert!((turned(0.1, 0.2) - 0.1).abs() < 1e-5);
        assert!((turned(0.45, -0.45) - 0.1).abs() < 1e-5, "across the bottom");
        assert!((turned(-0.45, 0.45) + 0.1).abs() < 1e-5);
    }

    #[test]
    fn a_circle_starts_at_the_top_and_goes_clockwise() {
        let c = pos2(0.0, 0.0);
        assert!((on_circle(c, 1.0, 0.0) - pos2(0.0, -1.0)).length() < 1e-5);
        assert!((on_circle(c, 1.0, 0.25) - pos2(1.0, 0.0)).length() < 1e-5);
    }
}
