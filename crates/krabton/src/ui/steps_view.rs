//! Steps: a clip's notes as a step sequencer (as in Logic's). A row per drum or
//! note, each with its own length and rate; click a step to put a note on it or
//! take it off, paint along a row, and drag up and down on a step to set what
//! the *Edit* list says: how hard, how long, the chance, the ratchets, every how
//! many times it plays, how far off the beat. A row's triangle opens all of those
//! as rows of their own, and *Add lane* gives a row of steps to anything the clip
//! can move: an effect's knob, a synth's envelope, the mod wheel. See
//! `krabton_engine::steps`.

use crate::app::{App, ClipSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, RichText, Sense, Stroke, Ui};
use krabton_engine::note_name;
use krabton_engine::project::Envelope;
use krabton_engine::steps::{self, StepLane, StepRow, RATES};
use krabton_engine::{ClipContent, Command, Note};
use std::collections::HashSet;
use std::time::Instant;

const HEAD_W: f32 = 196.0;
const ROW_H: f32 = 23.0;
const SUB_H: f32 = 28.0;
const LANE_H: f32 = 32.0;
const LENGTHS: [u16; 17] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 14, 16, 24, 32, 48, 64];

/// What dragging on a step sets.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Edit {
    Velocity,
    Gate,
    Chance,
    Ratchet,
    Every,
    Offset,
    /// A melody row's: the note within its octave, and which octave.
    Note,
    Octave,
}

const EDITS: [(Edit, &str, &str); 6] = [
    (Edit::Velocity, "Velocity", "How hard it plays"),
    (Edit::Gate, "Gate", "How long it sounds: past the end of the step it ties into the next"),
    (Edit::Chance, "Chance", "How likely it is to play each time"),
    (Edit::Ratchet, "Ratchet", "So many quick hits on the step"),
    (Edit::Every, "Every", "It plays on the first of so many times round"),
    (Edit::Offset, "Offset", "How far before or after the beat"),
];

/// A melody row's rows: its notes first.
const MELODY_EDITS: [(Edit, &str, &str); 8] = [
    (Edit::Note, "Note", "Which note of the octave, in the song's key"),
    (Edit::Octave, "Octave", "Two octaves down to two up"),
    EDITS[0],
    EDITS[1],
    EDITS[2],
    EDITS[3],
    EDITS[4],
    EDITS[5],
];

/// The octaves a melody row reaches either way from its own.
const OCTAVES: i32 = 2;

impl Edit {
    /// What a note's value is on a scale of 0 to 1.
    pub(crate) fn value(self, n: &Note, row: &StepRow, at: f64) -> f32 {
        match self {
            Edit::Velocity => n.velocity,
            Edit::Gate => ((n.length / row.step_len()) * 0.5) as f32,
            Edit::Chance => n.chance,
            Edit::Ratchet => (n.repeats.max(1) - 1) as f32 / 7.0,
            Edit::Every => (n.recur_len.max(1) - 1) as f32 / 7.0,
            Edit::Offset => (((n.start - at) / row.step_len()) as f32 + 0.5).clamp(0.0, 1.0),
            Edit::Note => (n.pitch as i32 - row.pitch as i32).rem_euclid(12) as f32 / 11.0,
            Edit::Octave => (((n.pitch as i32 - row.pitch as i32).div_euclid(12) + OCTAVES) as f32 / (2 * OCTAVES) as f32).clamp(0.0, 1.0),
        }
    }

    /// Sets a note from a value of 0 to 1.
    pub(crate) fn apply(self, n: &mut Note, row: &StepRow, at: f64, v: f32) {
        let v = v.clamp(0.0, 1.0);
        match self {
            Edit::Velocity => n.velocity = v.max(0.01),
            Edit::Gate => n.length = (v as f64 * 2.0 * row.step_len()).max(row.step_len() * 0.05),
            Edit::Chance => n.chance = v,
            Edit::Ratchet => n.repeats = 1 + (v * 7.0).round() as u8,
            Edit::Every => {
                n.recur_len = 1 + (v * 7.0).round() as u8;
                n.recur_mask = 1;
            }
            Edit::Offset => n.start = (at + (v as f64 - 0.5) * row.step_len() * 0.98).max(0.0),
            Edit::Note => {
                let octave = (n.pitch as i32 - row.pitch as i32).div_euclid(12);
                n.pitch = (row.pitch as i32 + 12 * octave + (v * 11.0).round() as i32).clamp(0, 127) as u8;
            }
            Edit::Octave => {
                let within = (n.pitch as i32 - row.pitch as i32).rem_euclid(12);
                let octave = (v * (2 * OCTAVES) as f32).round() as i32 - OCTAVES;
                n.pitch = (row.pitch as i32 + 12 * octave + within).clamp(0, 127) as u8;
            }
        }
    }

    /// What a step says when the hand is over it.
    fn tell(self, n: &Note, row: &StepRow, at: f64) -> String {
        match self {
            Edit::Velocity => format!("Velocity {:.0}", n.velocity * 127.0),
            Edit::Gate => format!("Gate {:.2} steps", n.length / row.step_len()),
            Edit::Chance => format!("Chance {:.0} %", n.chance * 100.0),
            Edit::Ratchet => format!("{} hits", n.repeats.max(1)),
            Edit::Every if n.recur_len > 1 => format!("Every {}", n.recur_len),
            Edit::Every => "Every time".into(),
            Edit::Offset => format!("Offset {:+.0} %", (n.start - at) / row.step_len() * 100.0),
            Edit::Note => note_name(n.pitch),
            Edit::Octave => format!("Octave {:+}", (n.pitch as i32 - row.pitch as i32).div_euclid(12)),
        }
    }

    /// What is written on a step.
    fn label(self, n: &Note) -> Option<String> {
        match self {
            Edit::Note => Some(note_name(n.pitch)),
            Edit::Ratchet if n.repeats > 1 => Some(format!("{}", n.repeats)),
            Edit::Every if n.recur_len > 1 => Some(format!("1/{}", n.recur_len)),
            Edit::Chance if n.chance < 0.995 => Some(format!("{:.0}", n.chance * 100.0)),
            _ => None,
        }
    }
}

/// A drag on the grid.
#[derive(Clone, Copy)]
struct Stroke_ {
    /// Painting steps on or off, or setting one step's value.
    paint: Option<bool>,
    row: usize,
    step: usize,
    start_y: f32,
    start_value: f32,
}

pub struct StepsState {
    pub(crate) open: bool,
    edit: Edit,
    /// The rate new rows start at.
    rate: u8,
    audition: bool,
    drag: Option<Stroke_>,
    /// The note an audition is holding, and since when.
    sounding: Option<(u64, u8, Instant)>,
    /// Hits for the Fill menu.
    hits: usize,
    /// The rows opened up.
    open_rows: HashSet<u8>,
    /// The last place a value was drawn: (which row, step, value), so that a
    /// fast hand leaves no steps out.
    last_draw: Option<(u64, usize, f32)>,
}

impl Default for StepsState {
    fn default() -> Self {
        Self { open: false, edit: Edit::Velocity, rate: steps::DEFAULT_RATE, audition: true, drag: None, sounding: None, hits: 4, open_rows: HashSet::new(), last_draw: None }
    }
}

impl App {
    /// The rows: every pad of a drum track; else the notes in use and those
    /// added, and an octave of the key to start from.
    fn step_rows(&self, sel: ClipSel) -> Vec<(StepRow, String)> {
        let Some(clip) = self.clip_ref(sel) else { return Vec::new() };
        let (pitches, names, drums) = self.piano_rows(sel.track());
        if let Some(m) = clip.step_melody.filter(|_| !drums) {
            return vec![(m.fit(clip.length), "Melody".into())];
        }
        let used: Vec<u8> = match &clip.content {
            ClipContent::Midi { notes } => notes.iter().map(|n| n.pitch).collect(),
            _ => Vec::new(),
        };
        let mut list: Vec<(u8, String)> = if drums {
            pitches.into_iter().zip(names).collect()
        } else {
            let mut v: Vec<u8> = used.clone();
            v.extend(clip.step_rows.iter().map(|r| r.pitch));
            if v.is_empty() {
                let scale = self.project.scale;
                v.extend((60..72u8).filter(|p| scale.contains(*p)));
            }
            v.sort_unstable_by(|a, b| b.cmp(a));
            v.dedup();
            v.into_iter().map(|p| (p, note_name(p))).collect()
        };
        // A note outside a kit's pads still gets its row.
        for p in used {
            if !list.iter().any(|x| x.0 == p) {
                list.push((p, note_name(p)));
            }
        }
        let rate = self.steps.rate;
        let length = clip.length;
        list.into_iter()
            .map(|(p, name)| {
                let row = clip.step_rows.iter().find(|r| r.pitch == p).copied().unwrap_or_else(|| StepRow::new(p, rate, length));
                (row.fit(length), name)
            })
            .collect()
    }

    fn step_notes_mut(&mut self, sel: ClipSel) -> Option<(&mut Vec<Note>, f64)> {
        let clip = self.clip_mut(sel)?;
        let length = clip.length;
        match &mut clip.content {
            ClipContent::Midi { notes } => Some((notes, length)),
            _ => None,
        }
    }

    /// A row's settings changed: remembered in the clip, and the notes follow.
    fn set_step_row(&mut self, sel: ClipSel, old: StepRow, new: StepRow) {
        if let Some((notes, length)) = self.step_notes_mut(sel) {
            steps::change_row(notes, length, &old, &new);
        }
        if let Some(clip) = self.clip_mut(sel) {
            if new.any {
                clip.step_melody = Some(new);
            } else {
                clip.step_rows.retain(|r| r.pitch != new.pitch);
                clip.step_rows.push(new);
            }
        }
        self.piano.selected.clear();
    }

    /// The clip as one melody row, or back to a row per note.
    fn set_melody(&mut self, sel: ClipSel, on: bool) {
        let rate = self.steps.rate;
        let Some(clip) = self.clip_mut(sel) else { return };
        if !on {
            clip.step_melody = None;
            return;
        }
        // From the C below its lowest note, else C3.
        let low = match &clip.content {
            ClipContent::Midi { notes } => notes.iter().map(|n| n.pitch).min(),
            _ => None,
        };
        let home = low.map_or(48, |p| p - p % 12);
        let row = StepRow::melody(home, rate, clip.length);
        clip.step_melody = Some(row);
        self.steps.open_rows.insert(home);
    }

    fn step_audition(&mut self, sel: ClipSel, pitch: u8, velocity: f32) {
        let Some(id) = self.project.tracks.get(sel.track()).map(|t| t.id) else { return };
        self.step_audition_end();
        self.cmd(Command::NoteOn { track: id, pitch, velocity });
        self.steps.sounding = Some((id, pitch, Instant::now()));
    }

    fn step_audition_end(&mut self) {
        if let Some((track, pitch, _)) = self.steps.sounding.take() {
            self.cmd(Command::NoteOff { track, pitch });
        }
    }

    /// The Steps tab of a MIDI clip.
    pub(crate) fn ui_steps(&mut self, ui: &mut Ui, area: Rect, sel: ClipSel) {
        if self.steps.sounding.is_some_and(|s| s.2.elapsed().as_millis() > 200) {
            self.step_audition_end();
        }
        if !ui.input(|i| i.pointer.any_down()) {
            self.steps.last_draw = None;
        }
        let bar = Rect::from_min_size(area.min, vec2(area.width(), 28.0));
        let below = Rect::from_min_max(pos2(area.left(), bar.bottom()), area.max);
        rect_fill(ui.painter(), bar, 0.0, PANEL2);
        let mut sub = ui.new_child(egui::UiBuilder::new().max_rect(bar.shrink2(vec2(6.0, 3.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
        sub.set_clip_rect(bar);
        self.steps_bar(&mut sub, sel);
        let rows = self.step_rows(sel);
        let playhead = self.clip_play_pos(sel);
        // The lanes of what is still there (a device can have gone).
        let targets = self.env_targets(sel.track());
        let lanes: Vec<StepLane> = self.clip_ref(sel).map_or(Vec::new(), |c| c.step_lanes.iter().filter(|l| targets.iter().any(|x| x.target == l.target)).copied().collect());
        let longest = rows.iter().map(|r| r.0.steps as usize).chain(lanes.iter().map(|l| l.steps as usize)).max().unwrap_or(16).max(1);
        let step_w = ((below.width() - HEAD_W - 14.0) / longest as f32).clamp(14.0, 44.0);
        let mut grid = ui.new_child(egui::UiBuilder::new().max_rect(below));
        grid.set_clip_rect(below);
        egui::ScrollArea::both().id_salt("steps_rows").auto_shrink([false, false]).show(&mut grid, |ui| {
            for (index, lane) in lanes.iter().enumerate() {
                let Some(info) = targets.iter().find(|x| x.target == lane.target) else { continue };
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    let (head, _) = ui.allocate_exact_size(vec2(HEAD_W, LANE_H), Sense::hover());
                    self.lane_head(ui, head, sel, *lane, info, index);
                    let (cells, _) = ui.allocate_exact_size(vec2(step_w * lane.steps as f32, LANE_H), Sense::hover());
                    self.lane_cells(ui, cells, sel, *lane, info, index, step_w, playhead);
                });
            }
            for (index, (row, name)) in rows.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    let (head, _) = ui.allocate_exact_size(vec2(HEAD_W, ROW_H), Sense::hover());
                    self.step_head(ui, head, sel, *row, name, index);
                    let (cells, _) = ui.allocate_exact_size(vec2(step_w * row.steps as f32, ROW_H), Sense::hover());
                    self.step_cells(ui, cells, sel, *row, index, step_w, playhead);
                });
                if self.steps.open_rows.contains(&row.pitch) {
                    let subs: &[(Edit, &str, &str)] = if row.any { &MELODY_EDITS } else { &EDITS };
                    for (which, &(edit, name, tip)) in subs.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                            let (head, resp) = ui.allocate_exact_size(vec2(HEAD_W, SUB_H), Sense::hover());
                            rect_fill(ui.painter(), head.shrink2(vec2(0.0, 1.0)), 0.0, PANEL);
                            ui.painter().text(head.right_center() - vec2(8.0, 0.0), Align2::RIGHT_CENTER, name, small(), TEXT_DIM);
                            resp.on_hover_text(tip);
                            let (cells, _) = ui.allocate_exact_size(vec2(step_w * row.steps as f32, SUB_H), Sense::hover());
                            self.step_sub_cells(ui, cells, sel, *row, edit, which, step_w, playhead);
                        });
                    }
                }
            }
        });
    }

    fn steps_bar(&mut self, ui: &mut Ui, sel: ClipSel) {
        ui.spacing_mut().item_spacing = vec2(6.0, 0.0);
        ui.label(RichText::new("Edit").small().color(TEXT_DIM));
        let now = EDITS.iter().find(|e| e.0 == self.steps.edit).map_or("", |e| e.1);
        egui::ComboBox::from_id_salt("steps_edit").width(76.0).selected_text(now).show_ui(ui, |ui| {
            for (e, name, tip) in EDITS {
                ui.selectable_value(&mut self.steps.edit, e, name).on_hover_text(tip);
            }
        });
        ui.label(RichText::new("New rows").small().color(TEXT_DIM));
        egui::ComboBox::from_id_salt("steps_rate").width(56.0).selected_text(RATES[self.steps.rate as usize].0).show_ui(ui, |ui| {
            for (k, r) in RATES.iter().enumerate() {
                ui.selectable_value(&mut self.steps.rate, k as u8, r.0);
            }
        });
        ui.checkbox(&mut self.steps.audition, "Hear").on_hover_text("You hear a step when you put it on");
        // A row for another note.
        let (_, _, drums) = self.piano_rows(sel.track());
        let melody = self.clip_ref(sel).is_some_and(|c| c.step_melody.is_some());
        if !drums {
            // A row per note, or one melody row whose steps each have their own note.
            if ui.selectable_label(!melody, "Rows").on_hover_text("A row per note").clicked() && melody {
                self.set_melody(sel, false);
            }
            if ui.selectable_label(melody, "Melody").on_hover_text("One row, each step its own note and octave, as in Logic").clicked() && !melody {
                self.set_melody(sel, true);
            }
        }
        if !drums && !melody {
            let mut add = None;
            ui.menu_button("Add row", |ui| {
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    let scale = self.project.scale;
                    let fold = self.project.scale_mode;
                    for p in (24..=96u8).rev().filter(|p| !fold || scale.contains(*p)) {
                        if ui.button(note_name(p)).clicked() {
                            add = Some(p);
                            ui.close();
                        }
                    }
                });
            });
            if let Some(p) = add {
                let rate = self.steps.rate;
                if let Some(clip) = self.clip_mut(sel) {
                    if !clip.step_rows.iter().any(|r| r.pitch == p) {
                        let row = StepRow::new(p, rate, clip.length);
                        clip.step_rows.push(row);
                    }
                }
            }
        }
        // A row of steps for anything the clip can move.
        let targets = self.env_targets(sel.track());
        let mut groups: Vec<String> = Vec::new();
        for x in &targets {
            if !groups.contains(&x.group) {
                groups.push(x.group.clone());
            }
        }
        let mut lane = None;
        ui.menu_button("Add lane", |ui| {
            for g in &groups {
                ui.menu_button(g, |ui| {
                    egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                        for x in targets.iter().filter(|x| &x.group == g) {
                            if ui.button(&x.name).clicked() {
                                lane = Some(x.target);
                                ui.close();
                            }
                        }
                    });
                });
            }
        })
        .response
        .on_hover_text("Steps for an effect's knob, a synth's envelope, the mod wheel: anything the clip can move");
        if let Some(target) = lane {
            let rate = self.steps.rate;
            if let Some(clip) = self.clip_mut(sel) {
                if !clip.step_lanes.iter().any(|l| l.target == target) {
                    let new = StepLane::new(target, rate, clip.length);
                    clip.step_lanes.push(new);
                }
            }
        }
        ui.label(RichText::new("Click: on or off. Drag along: paint. Drag up or down: its value").small().color(TEXT_DIM));
    }

    /// A row's head: its name to hear it, its length and rate, what can be done to it.
    fn step_head(&mut self, ui: &mut Ui, r: Rect, sel: ClipSel, row: StepRow, name: &str, index: usize) {
        rect_fill(ui.painter(), r.shrink2(vec2(0.0, 1.0)), 0.0, PANEL2);
        // Its triangle: everything about its steps, row by row.
        let open = self.steps.open_rows.contains(&row.pitch);
        let tri_r = Rect::from_min_size(r.min, vec2(16.0, r.height()));
        let tri = ui.interact(tri_r, Id::new(("steps_open", index)), Sense::click());
        if tri.clicked() {
            if open {
                self.steps.open_rows.remove(&row.pitch);
            } else {
                self.steps.open_rows.insert(row.pitch);
            }
        }
        let c = tri_r.center();
        let shape = if open { vec![c + vec2(-4.0, -2.5), c + vec2(4.0, -2.5), c + vec2(0.0, 3.5)] } else { vec![c + vec2(-2.5, -4.0), c + vec2(3.5, 0.0), c + vec2(-2.5, 4.0)] };
        ui.painter().add(egui::Shape::convex_polygon(shape, if tri.hovered() { accent_now() } else { TEXT_DIM }, Stroke::NONE));
        tri.on_hover_text("Open it up: velocity, gate, chance, ratchet, every and offset, step by step");
        let name_r = Rect::from_min_size(r.min + vec2(16.0, 0.0), vec2(56.0, r.height()));
        let resp = ui.interact(name_r, Id::new(("steps_name", index)), Sense::click());
        if resp.clicked() {
            self.step_audition(sel, row.pitch, 0.8);
        }
        ui.painter().with_clip_rect(name_r).text(name_r.left_center(), Align2::LEFT_CENTER, name, small(), if resp.hovered() { accent_now() } else { TEXT });
        resp.on_hover_text("Hear it");
        let tools = Rect::from_min_max(pos2(name_r.right(), r.top() + 2.0), pos2(r.right() - 2.0, r.bottom() - 2.0));
        let mut sub = ui.new_child(egui::UiBuilder::new().max_rect(tools).layout(egui::Layout::left_to_right(egui::Align::Center)));
        sub.spacing_mut().item_spacing = vec2(2.0, 0.0);
        let length = self.clip_ref(sel).map_or(4.0, |c| c.length);
        let most = steps::most_steps(row.rate, length);
        let mut new = row;
        egui::ComboBox::from_id_salt(("steps_len", index)).width(32.0).selected_text(format!("{}", row.steps)).show_ui(&mut sub, |ui| {
            for n in LENGTHS.into_iter().filter(|n| *n < most).chain(std::iter::once(most)) {
                ui.selectable_value(&mut new.steps, n, format!("{n}"));
            }
        }).response.on_hover_text("How many steps before this row goes round");
        egui::ComboBox::from_id_salt(("steps_rate_row", index)).width(36.0).selected_text(RATES[(row.rate as usize).min(RATES.len() - 1)].0).show_ui(&mut sub, |ui| {
            for (k, x) in RATES.iter().enumerate() {
                ui.selectable_value(&mut new.rate, k as u8, x.0);
            }
        }).response.on_hover_text("How fast this row steps");
        if new != row {
            // A row that ran the whole clip still does at its new rate.
            let new = if new.rate != row.rate && row.fills(length) { StepRow::new(row.pitch, new.rate, length) } else { new.fit(length) };
            self.set_step_row(sel, row, new);
        }
        let hits = self.steps.hits;
        let mut act: Option<u8> = None;
        sub.menu_button("...", |ui| {
            if ui.button("Shift left").clicked() {
                act = Some(0);
            }
            if ui.button("Shift right").clicked() {
                act = Some(1);
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(RichText::new("Hits").small());
                ui.add(egui::DragValue::new(&mut self.steps.hits).range(1..=64));
            });
            if ui.button(format!("Spread {hits} hits evenly")).on_hover_text("The classic rhythms come out of this: try 3 in 8, 5 in 16").clicked() {
                act = Some(2);
            }
            if ui.button("Clear").clicked() {
                act = Some(3);
            }
        });
        if let Some(a) = act {
            let velocity = 0.8;
            if let Some((notes, length)) = self.step_notes_mut(sel) {
                match a {
                    0 => steps::shift_row(notes, length, &row, -1),
                    1 => steps::shift_row(notes, length, &row, 1),
                    2 => steps::fill_row(notes, length, &row, &steps::euclid(hits, row.steps as usize), velocity),
                    _ => steps::clear_row(notes, &row),
                }
            }
            self.piano.selected.clear();
        }
    }

    /// The steps a hand on a row of values is going over, with their values,
    /// drawn from the bottom of the row (0) to its top (1).
    fn drawn_steps(&mut self, key: u64, resp: &egui::Response, rect: Rect, step_w: f32, steps: usize) -> Vec<(usize, f32)> {
        if !resp.is_pointer_button_down_on() {
            return Vec::new();
        }
        let Some(at) = resp.interact_pointer_pos() else { return Vec::new() };
        let k = (((at.x - rect.left()) / step_w).floor().max(0.0) as usize).min(steps.saturating_sub(1));
        let v = ((rect.bottom() - 2.0 - at.y) / (rect.height() - 4.0)).clamp(0.0, 1.0);
        let (from, was) = self.steps.last_draw.filter(|l| l.0 == key).map_or((k, v), |l| (l.1, l.2));
        self.steps.last_draw = Some((key, k, v));
        between(from, was, k, v)
    }

    /// An opened row: one thing about its steps.
    #[allow(clippy::too_many_arguments)]
    fn step_sub_cells(&mut self, ui: &mut Ui, rect: Rect, sel: ClipSel, row: StepRow, edit: Edit, which: usize, step_w: f32, playhead: Option<f64>) {
        let color = self.clip_ref(sel).map_or(accent_now(), |c| Color32::from_rgb(c.color[0], c.color[1], c.color[2]));
        let notes: Vec<Note> = match self.clip_ref(sel).map(|c| &c.content) {
            Some(ClipContent::Midi { notes }) => notes.iter().filter(|n| row.owns(n.pitch)).cloned().collect(),
            _ => Vec::new(),
        };
        let now = playhead.map(|b| row.step_at(b));
        let p = ui.painter_at(rect);
        for k in 0..row.steps as usize {
            let r = Rect::from_min_size(rect.min + vec2(k as f32 * step_w, 0.0), vec2(step_w, rect.height())).shrink(1.0);
            p.rect_filled(r, 1.0, Color32::from_gray(0x2e));
            if let Some(n) = steps::step_note(&notes, &row, k) {
                let v = edit.value(n, &row, k as f64 * row.step_len()).clamp(0.0, 1.0);
                if edit == Edit::Offset {
                    let y = r.bottom() - v * r.height();
                    p.rect_filled(Rect::from_x_y_ranges(r.x_range(), y.min(r.center().y)..=y.max(r.center().y)), 0.0, color);
                    p.line_segment([r.left_center(), r.right_center()], Stroke::new(1.0_f32, Color32::from_gray(0x70)));
                } else {
                    let h = v.max(0.05) * r.height();
                    p.rect_filled(Rect::from_min_max(pos2(r.left(), r.bottom() - h), r.max), 1.0, color);
                }
                if let Some(text) = edit.label(n).filter(|_| r.width() >= 22.0 || edit != Edit::Note) {
                    p.text(r.center(), Align2::CENTER_CENTER, text, small(), Color32::WHITE);
                }
            }
            if now == Some(k) {
                p.rect_stroke(r, 1.0, Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Inside);
            }
        }
        let resp = ui.interact(rect, Id::new(("steps_sub", row.pitch, which)), Sense::click_and_drag());
        let key = row.pitch as u64 * 16 + which as u64;
        let hits = self.drawn_steps(key, &resp, rect, step_w, row.steps as usize);
        if !hits.is_empty() {
            // A melody's notes stay in the song's key.
            let scale = self.project.scale;
            let snap = matches!(edit, Edit::Note | Edit::Octave);
            if let Some((all, length)) = self.step_notes_mut(sel) {
                for (k, v) in hits {
                    steps::edit_step(all, length, &row, k, |n, at| {
                        edit.apply(n, &row, at, v);
                        if snap {
                            n.pitch = scale.snap(n.pitch);
                        }
                    });
                }
            }
            self.piano.selected.clear();
        }
        if let Some(at) = resp.hover_pos() {
            let k = (((at.x - rect.left()) / step_w).floor().max(0.0) as usize).min(row.steps.max(1) as usize - 1);
            if let Some(n) = steps::step_note(&notes, &row, k) {
                let text = edit.tell(n, &row, k as f64 * row.step_len());
                resp.on_hover_text_at_pointer(text);
            }
        }
    }

    /// A lane's steps as the clip has them.
    fn lane_now(&self, sel: ClipSel, lane: &StepLane) -> Vec<f32> {
        let base = self.env_base(sel.track(), lane.target).unwrap_or(0.0);
        let env = self.clip_ref(sel).and_then(|c| c.envelopes.iter().find(|e| e.target == lane.target && !e.points.is_empty()));
        steps::lane_values(env, lane, base)
    }

    /// Writes a lane's steps into the clip's line for it.
    fn lane_write(&mut self, sel: ClipSel, lane: &StepLane, values: &[f32]) {
        let Some(clip) = self.clip_mut(sel) else { return };
        let length = clip.length;
        let k = match clip.envelopes.iter().position(|e| e.target == lane.target) {
            Some(k) => k,
            None => {
                clip.envelopes.push(Envelope { target: lane.target, points: Vec::new() });
                clip.envelopes.len() - 1
            }
        };
        steps::write_lane(&mut clip.envelopes[k], lane, length, values);
    }

    /// A lane's settings changed: remembered in the clip, and its line follows.
    fn lane_change(&mut self, sel: ClipSel, old: StepLane, new: StepLane) {
        let was = self.lane_now(sel, &old);
        let values: Vec<f32> = (0..new.steps.max(1) as usize).map(|k| was[k % was.len()]).collect();
        if let Some(clip) = self.clip_mut(sel) {
            if let Some(l) = clip.step_lanes.iter_mut().find(|l| l.target == old.target) {
                *l = new;
            }
        }
        self.lane_write(sel, &new, &values);
    }

    fn lane_head(&mut self, ui: &mut Ui, r: Rect, sel: ClipSel, lane: StepLane, info: &crate::ui::envelope_view::EnvTarget, index: usize) {
        rect_fill(ui.painter(), r.shrink2(vec2(0.0, 1.0)), 0.0, PANEL2);
        let name_r = Rect::from_min_size(r.min, vec2(72.0, r.height()));
        ui.painter().with_clip_rect(name_r).text(name_r.left_center() + vec2(5.0, 0.0), Align2::LEFT_CENTER, &info.name, small(), TEXT);
        ui.interact(name_r, Id::new(("steps_lane_name", index)), Sense::hover()).on_hover_text(format!("{}: {}", info.group, info.name));
        let tools = Rect::from_min_max(pos2(name_r.right(), r.top() + 4.0), pos2(r.right() - 2.0, r.bottom() - 4.0));
        let mut sub = ui.new_child(egui::UiBuilder::new().max_rect(tools).layout(egui::Layout::left_to_right(egui::Align::Center)));
        sub.spacing_mut().item_spacing = vec2(2.0, 0.0);
        let length = self.clip_ref(sel).map_or(4.0, |c| c.length);
        let most = steps::most_steps(lane.rate, length);
        let mut new = lane;
        egui::ComboBox::from_id_salt(("lane_len", index)).width(32.0).selected_text(format!("{}", lane.steps)).show_ui(&mut sub, |ui| {
            for n in LENGTHS.into_iter().filter(|n| *n < most).chain(std::iter::once(most)) {
                ui.selectable_value(&mut new.steps, n, format!("{n}"));
            }
        });
        egui::ComboBox::from_id_salt(("lane_rate", index)).width(36.0).selected_text(RATES[(lane.rate as usize).min(RATES.len() - 1)].0).show_ui(&mut sub, |ui| {
            for (k, x) in RATES.iter().enumerate() {
                ui.selectable_value(&mut new.rate, k as u8, x.0);
            }
        });
        let mut act = None;
        sub.menu_button("...", |ui| {
            let mut smooth = lane.smooth;
            if ui.checkbox(&mut smooth, "Slide between steps").changed() {
                act = Some(0);
                ui.close();
            }
            if ui.button("Remove lane").on_hover_text("Takes its line out of the clip too").clicked() {
                act = Some(1);
                ui.close();
            }
        });
        if new != lane {
            // A lane that ran the whole clip still does at its new rate.
            let new = if new.rate != lane.rate && lane.row().fills(length) { StepLane { smooth: lane.smooth, ..StepLane::new(lane.target, new.rate, length) } } else { StepLane { steps: new.row().fit(length).steps, ..new } };
            self.lane_change(sel, lane, new);
        }
        match act {
            Some(0) => self.lane_change(sel, lane, StepLane { smooth: !lane.smooth, ..lane }),
            Some(_) => {
                if let Some(clip) = self.clip_mut(sel) {
                    clip.step_lanes.retain(|l| l.target != lane.target);
                    clip.envelopes.retain(|e| e.target != lane.target);
                }
            }
            None => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn lane_cells(&mut self, ui: &mut Ui, rect: Rect, sel: ClipSel, lane: StepLane, info: &crate::ui::envelope_view::EnvTarget, index: usize, step_w: f32, playhead: Option<f64>) {
        let color = self.clip_ref(sel).map_or(accent_now(), |c| Color32::from_rgb(c.color[0], c.color[1], c.color[2]));
        let row = lane.row();
        let mut values = self.lane_now(sel, &lane);
        let now = playhead.map(|b| row.step_at(b));
        let p = ui.painter_at(rect);
        for (k, v) in values.iter().enumerate() {
            let r = Rect::from_min_size(rect.min + vec2(k as f32 * step_w, 0.0), vec2(step_w, rect.height())).shrink(1.0);
            p.rect_filled(r, 1.0, Color32::from_gray(if (k as f64 * row.step_len()).floor() as i64 % 2 == 0 { 0x34 } else { 0x2a }));
            let h = v.clamp(0.0, 1.0).max(0.03) * r.height();
            p.rect_filled(Rect::from_min_max(pos2(r.left(), r.bottom() - h), r.max), 1.0, color.gamma_multiply(0.8));
            if now == Some(k) {
                p.rect_stroke(r, 1.0, Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Inside);
            }
        }
        let resp = ui.interact(rect, Id::new(("steps_lane", index)), Sense::click_and_drag());
        let hits = self.drawn_steps(1 << 32 | index as u64, &resp, rect, step_w, values.len());
        if !hits.is_empty() {
            for (k, v) in hits {
                values[k] = v;
            }
            self.lane_write(sel, &lane, &values);
        }
        if let Some(at) = resp.hover_pos() {
            let k = (((at.x - rect.left()) / step_w).floor().max(0.0) as usize).min(values.len().saturating_sub(1));
            let text = info.format(values[k]);
            resp.on_hover_text_at_pointer(text);
        }
    }

    fn step_cells(&mut self, ui: &mut Ui, rect: Rect, sel: ClipSel, row: StepRow, index: usize, step_w: f32, playhead: Option<f64>) {
        let edit = self.steps.edit;
        let color = self.clip_ref(sel).map_or(accent_now(), |c| Color32::from_rgb(c.color[0], c.color[1], c.color[2]));
        let notes: Vec<Note> = match self.clip_ref(sel).map(|c| &c.content) {
            Some(ClipContent::Midi { notes }) => notes.iter().filter(|n| row.owns(n.pitch)).cloned().collect(),
            _ => Vec::new(),
        };
        let now = playhead.map(|b| row.step_at(b));
        let p = ui.painter_at(rect);
        for k in 0..row.steps as usize {
            let r = Rect::from_min_size(rect.min + vec2(k as f32 * step_w, 0.0), vec2(step_w, rect.height())).shrink(1.0);
            let beat_group = (k as f64 * row.step_len() / 1.0).floor() as i64 % 2 == 0;
            let base = if beat_group { Color32::from_gray(0x46) } else { Color32::from_gray(0x34) };
            p.rect_filled(r, 2.0, base);
            if let Some(n) = steps::step_note(&notes, &row, k) {
                let at = k as f64 * row.step_len();
                let v = edit.value(n, &row, at);
                p.rect_filled(r, 2.0, color.gamma_multiply(0.35));
                match edit {
                    Edit::Gate => {
                        let w = (v * 2.0).min(1.0) * r.width();
                        p.rect_filled(Rect::from_min_size(r.min, vec2(w, r.height())), 2.0, color);
                        if v > 0.5 {
                            p.line_segment([r.right_center(), r.right_center() + vec2(step_w * 0.5, 0.0)], Stroke::new(3.0_f32, color));
                        }
                    }
                    Edit::Offset => {
                        p.rect_filled(r, 2.0, color.gamma_multiply(0.7));
                        let x = r.left() + v * r.width();
                        p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], Stroke::new(2.0_f32, Color32::WHITE));
                    }
                    Edit::Ratchet => {
                        p.rect_filled(r, 2.0, color);
                        let hits = n.repeats.max(1) as usize;
                        if hits > 1 {
                            for h in 1..hits {
                                let x = r.left() + r.width() * h as f32 / hits as f32;
                                p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], Stroke::new(1.0_f32, Color32::from_gray(0x10)));
                            }
                        }
                    }
                    Edit::Every => {
                        p.rect_filled(r, 2.0, color.gamma_multiply(1.0 / n.recur_len.max(1) as f32));
                    }
                    _ => {
                        let h = v.clamp(0.04, 1.0) * r.height();
                        p.rect_filled(Rect::from_min_max(pos2(r.left(), r.bottom() - h), r.max), 2.0, color);
                    }
                }
                if let Some(text) = edit.label(n) {
                    p.text(r.center(), Align2::CENTER_CENTER, text, small(), Color32::WHITE);
                } else if row.any && r.width() >= 24.0 {
                    p.text(r.center(), Align2::CENTER_CENTER, note_name(n.pitch), small(), Color32::WHITE);
                }
            }
            if now == Some(k) {
                p.rect_stroke(r, 2.0, Stroke::new(1.5_f32, Color32::WHITE), egui::StrokeKind::Inside);
            }
        }
        // The hand.
        let resp = ui.interact(rect, Id::new(("steps_cells", index)), Sense::click_and_drag());
        let step_of = |x: f32| (((x - rect.left()) / step_w).floor().max(0.0) as usize).min(row.steps.max(1) as usize - 1);
        if resp.drag_started() {
            if let Some(at) = resp.interact_pointer_pos() {
                let k = step_of(at.x);
                let on = steps::step_note(&notes, &row, k);
                let value = on.map_or(0.0, |n| edit.value(n, &row, k as f64 * row.step_len()));
                self.steps.drag = Some(Stroke_ { paint: None, row: index, step: k, start_y: at.y, start_value: value });
            }
        }
        if resp.dragged() {
            if let (Some(mut d), Some(at)) = (self.steps.drag.filter(|d| d.row == index), resp.interact_pointer_pos()) {
                let k = step_of(at.x);
                let was_on = steps::step_note(&notes, &row, d.step).is_some();
                // Across: painting; up and down on a step that is on: its value.
                if d.paint.is_none() && (k != d.step || !was_on) {
                    d.paint = Some(!was_on);
                    if let Some((all, length)) = self.step_notes_mut(sel) {
                        steps::set_step(all, length, &row, d.step, !was_on, 0.8);
                    }
                }
                match d.paint {
                    Some(on) => {
                        if steps::step_note(&notes, &row, k).is_some() != on {
                            if let Some((all, length)) = self.step_notes_mut(sel) {
                                steps::set_step(all, length, &row, k, on, 0.8);
                            }
                        }
                    }
                    None => {
                        let v = d.start_value + (d.start_y - at.y) / 80.0;
                        if let Some((all, length)) = self.step_notes_mut(sel) {
                            steps::edit_step(all, length, &row, d.step, |n, place| edit.apply(n, &row, place, v));
                        }
                    }
                }
                self.steps.drag = Some(d);
                self.piano.selected.clear();
            }
        }
        if resp.drag_stopped() {
            self.steps.drag = None;
        }
        if resp.clicked() {
            if let Some(at) = resp.interact_pointer_pos() {
                let k = step_of(at.x);
                let on = steps::step_note(&notes, &row, k).is_none();
                if let Some((all, length)) = self.step_notes_mut(sel) {
                    steps::set_step(all, length, &row, k, on, 0.8);
                }
                self.piano.selected.clear();
                if on && self.steps.audition {
                    self.step_audition(sel, row.pitch, 0.8);
                }
            }
        }
    }
}

/// The steps from one place to another with the values along the way.
fn between(from: usize, was: f32, to: usize, v: f32) -> Vec<(usize, f32)> {
    if from == to {
        return vec![(to, v)];
    }
    let (a, b) = (from.min(to), from.max(to));
    (a..=b).map(|s| (s, was + (v - was) * (s as f32 - from as f32) / (to as f32 - from as f32))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fast_hand_leaves_no_steps_out() {
        assert_eq!(between(2, 0.5, 2, 0.7), [(2, 0.7)]);
        assert_eq!(between(0, 0.0, 2, 1.0), [(0, 0.0), (1, 0.5), (2, 1.0)]);
        assert_eq!(between(2, 1.0, 0, 0.0), [(0, 0.0), (1, 0.5), (2, 1.0)], "going back too");
    }

    #[test]
    fn a_step_tells_what_it_holds() {
        let r = row();
        let mut n = Note::new(36, 1.0, 0.25, 1.0);
        assert_eq!(Edit::Velocity.tell(&n, &r, 1.0), "Velocity 127");
        assert_eq!(Edit::Gate.tell(&n, &r, 1.0), "Gate 1.00 steps");
        assert_eq!(Edit::Every.tell(&n, &r, 1.0), "Every time");
        n.recur_len = 3;
        assert_eq!(Edit::Every.tell(&n, &r, 1.0), "Every 3");
        n.start = 1.05;
        assert_eq!(Edit::Offset.tell(&n, &r, 1.0), "Offset +20 %");
    }

    fn row() -> StepRow {
        StepRow { pitch: 36, steps: 16, rate: 2, any: false }
    }

    #[test]
    fn a_melody_step_moves_in_its_octave_and_by_octaves() {
        let m = StepRow::melody(48, 2, 4.0);
        let mut n = Note::new(48, 0.0, 0.2, 0.8);
        Edit::Note.apply(&mut n, &m, 0.0, 7.0 / 11.0);
        assert_eq!(n.pitch, 55);
        assert!((Edit::Note.value(&n, &m, 0.0) - 7.0 / 11.0).abs() < 1e-6);
        Edit::Octave.apply(&mut n, &m, 0.0, 0.75);
        assert_eq!(n.pitch, 67, "an octave up, the same note");
        assert_eq!(Edit::Octave.tell(&n, &m, 0.0), "Octave +1");
        Edit::Octave.apply(&mut n, &m, 0.0, 0.0);
        assert_eq!(n.pitch, 31, "two down");
        assert_eq!(Edit::Octave.value(&n, &m, 0.0), 0.0);
        Edit::Note.apply(&mut n, &m, 0.0, 0.0);
        assert_eq!(n.pitch, 24, "the note moves within its octave");
    }

    #[test]
    fn what_is_set_on_a_step_is_read_back() {
        let r = row();
        let at = 1.0;
        for (edit, v) in [(Edit::Velocity, 0.6), (Edit::Gate, 0.25), (Edit::Chance, 0.3), (Edit::Offset, 0.75)] {
            let mut n = Note::new(36, at, 0.2, 0.8);
            edit.apply(&mut n, &r, at, v);
            assert!((edit.value(&n, &r, at) - v).abs() < 0.02, "{edit:?}: {}", edit.value(&n, &r, at));
        }
    }

    #[test]
    fn ratchets_and_every_go_in_whole_numbers() {
        let r = row();
        let mut n = Note::new(36, 0.0, 0.2, 0.8);
        Edit::Ratchet.apply(&mut n, &r, 0.0, 3.0 / 7.0);
        assert_eq!(n.repeats, 4);
        assert_eq!(Edit::Ratchet.label(&n).as_deref(), Some("4"));
        Edit::Every.apply(&mut n, &r, 0.0, 1.0 / 7.0);
        assert_eq!((n.recur_len, n.recur_mask), (2, 1));
        assert_eq!(Edit::Every.label(&n).as_deref(), Some("1/2"));
        Edit::Velocity.apply(&mut n, &r, 0.0, 0.0);
        assert!(n.velocity > 0.0, "a step that is on still sounds");
    }

    #[test]
    fn a_long_gate_ties_into_the_next_step_and_an_offset_stays_on_its_step() {
        let r = row();
        let mut n = Note::new(36, 1.0, 0.2, 0.8);
        Edit::Gate.apply(&mut n, &r, 1.0, 1.0);
        assert!((n.length - 0.5).abs() < 1e-9, "two steps: {}", n.length);
        Edit::Offset.apply(&mut n, &r, 1.0, 1.0);
        assert!(n.start > 1.0 && n.start < 1.0 + r.step_len() * 0.5, "{}", n.start);
        let notes = vec![n];
        assert!(steps::step_note(&notes, &r, 4).is_some(), "still on its own step");
    }
}
