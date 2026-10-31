//! MIDI note editor.

use crate::app::{App, ClipSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, Id, Pos2, Rect, Sense, Stroke, Ui};
use krabton_engine::midi_tools::{self, ArpStyle};
use krabton_engine::note_edit;
use krabton_engine::{note_name, Instrument, Note};
use std::collections::HashSet;

const KEYS_W: f32 = 56.0;
const DRUM_KEYS_W: f32 = 96.0;
const RULER_H: f32 = 16.0;
const VEL_H: f32 = 46.0;
const ROW_H: f32 = 12.0;
const FOLD_ROW_H: f32 = 16.0;

/// Scale-aware note operations offered in the clip view.
#[derive(Clone, Copy, Debug)]
pub enum NoteTool {
    SnapToScale,
    Degrees(i32),
    Chords(usize),
    Arpeggiate,
    Strum,
    Legato,
    Humanize,
    Seed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragKind {
    Move,
    Resize,
    Marquee,
}

struct NoteDrag {
    kind: DragKind,
    origin: Pos2,
    originals: Vec<(usize, Note)>,
    /// Alt started a copy: it doesn't also free the notes from the grid.
    alt_copy: bool,
}

pub struct PianoState {
    key: Option<ClipSel>,
    pub selected: HashSet<usize>,
    drag: Option<NoteDrag>,
    pub draw_mode: bool,
    /// Show the notes of other clips playing alongside, faintly.
    pub ghosts: bool,
    pub grid: f64,
    scroll_y: f32,
    pub zoom: f32,
    scroll_x: f32,
    /// Where the notes were drawn last (the grid, pixels per beat, the
    /// scroll), for the envelope's line over them.
    pub(crate) last_view: Option<(Rect, f32, f32)>,
    preview: Option<(u8, Vec<u64>)>,
    marquee: Option<Rect>,
    vel_drag: Option<Vec<usize>>,
    /// Show only the rows of notes in the project scale.
    pub fold: bool,
    last_rows: usize,
    pub arp_rate: f64,
    pub arp_style: ArpStyle,
    pub seed: u64,
    /// Where Ctrl+V puts notes (set by clicking an empty spot).
    pub paste_at: Option<f64>,
    /// Live's time selection around the selected notes, when a box drew
    /// it (or a paste or Ctrl+D made it): what Ctrl+D steps by.
    pub time_sel: Option<(f64, f64)>,
    /// The clip view was clicked last, so the keys edit notes.
    pub focused: bool,
}

impl Default for PianoState {
    fn default() -> Self {
        Self {
            key: None,
            selected: HashSet::new(),
            drag: None,
            draw_mode: false,
            ghosts: true,
            grid: 0.25,
            scroll_y: 0.0,
            zoom: 1.0,
            scroll_x: 0.0,
            last_view: None,
            preview: None,
            marquee: None,
            vel_drag: None,
            fold: false,
            last_rows: 0,
            arp_rate: 0.25,
            arp_style: ArpStyle::Up,
            seed: 1,
            paste_at: None,
            time_sel: None,
            focused: false,
        }
    }
}

/// True when a button was pressed this frame in `rect`, on `ui`'s own layer
/// (not on a menu or window above it).
pub(crate) fn pressed_in(ui: &Ui, rect: Rect) -> bool {
    let Some(pos) = ui.input(|i| i.pointer.any_pressed().then(|| i.pointer.press_origin()).flatten()) else { return false };
    rect.contains(pos) && ui.ctx().layer_id_at(pos) == Some(ui.layer_id())
}

fn is_black(pitch: u8) -> bool {
    matches!(pitch % 12, 1 | 3 | 6 | 8 | 10)
}

impl App {
    /// Ctrl+A: every note, the whole loop as the time selection (when they
    /// all start in it), and the keys to the note editor.
    pub(crate) fn piano_select_all(&mut self) {
        if let Some(sel) = self.sel_clip {
            let Some(c) = self.clip_ref(sel) else { return };
            let len = c.length;
            let inside = c.notes().iter().all(|n| n.start < len);
            let (count, midi) = (c.notes().len(), c.is_midi());
            self.piano.selected = (0..count).collect();
            self.piano.time_sel = inside.then_some((0.0, len));
            self.piano.focused = midi;
        }
    }

    /// Delete: the selected notes go; the insert marker stays where they began.
    pub(crate) fn piano_delete_selected(&mut self) {
        let Some(sel) = self.sel_clip else { return };
        if let Some((from, _)) = self.notes_range(sel) {
            self.piano.paste_at = Some(from);
        }
        let mut selected: Vec<usize> = std::mem::take(&mut self.piano.selected).into_iter().collect();
        selected.sort_unstable();
        self.piano.time_sel = None;
        if let Some(notes) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
            note_edit::delete(notes, &selected);
        }
    }

    /// Quantizes the selected notes (or all notes) to the current grid.
    pub(crate) fn piano_quantize(&mut self) {
        let Some(sel) = self.sel_clip else { return };
        let grid = self.piano.grid;
        let selected = self.piano.selected.clone();
        if let Some(c) = self.clip_mut(sel) {
            let len = c.length;
            if let Some(notes) = c.notes_mut() {
                for (i, n) in notes.iter_mut().enumerate() {
                    if selected.is_empty() || selected.contains(&i) {
                        // A note at the loop's very end goes round to its start; past the loop it stays there.
                        let q = (n.start / grid).round() * grid;
                        n.start = if n.start < len { q.rem_euclid(len.max(grid)) } else { q.max(0.0) };
                    }
                }
            }
            self.toast("Quantized");
        }
    }

    pub(crate) fn piano_transpose(&mut self, semis: i32) {
        let Some(sel) = self.sel_clip else { return };
        let selected = self.piano.selected.clone();
        if let Some(notes) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
            for (i, n) in notes.iter_mut().enumerate() {
                if selected.is_empty() || selected.contains(&i) {
                    n.pitch = (n.pitch as i32 + semis).clamp(0, 127) as u8;
                }
            }
        }
    }

    /// Runs a scale-aware MIDI tool on the selected notes (or all notes).
    pub(crate) fn piano_tool(&mut self, tool: NoteTool) {
        let Some(sel) = self.sel_clip else { return };
        let scale = self.project.scale;
        let mut selected: Vec<usize> = self.piano.selected.iter().copied().collect();
        selected.sort_unstable();
        let (rate, style, seed) = (self.piano.arp_rate, self.piano.arp_style, self.piano.seed);
        self.piano.seed = self.piano.seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let grid = self.piano.grid;
        let Some(clip) = self.clip_mut(sel) else { return };
        let len = clip.length;
        let Some(notes) = clip.notes_mut() else { return };
        let mut new_sel: Option<Vec<usize>> = None;
        let msg = match tool {
            NoteTool::SnapToScale => format!("Snapped {} note(s) to {}", midi_tools::snap_to_scale(notes, &selected, &scale), scale.name()),
            NoteTool::Degrees(d) => {
                midi_tools::transpose_degrees(notes, &selected, &scale, d);
                format!("Moved {d:+} scale step(s) in {}", scale.name())
            }
            NoteTool::Chords(size) => {
                new_sel = Some(midi_tools::chordify(notes, &selected, &scale, size));
                format!("Built {} chords in {}", if size >= 4 { "seventh" } else { "triad" }, scale.name())
            }
            NoteTool::Arpeggiate => {
                new_sel = Some(midi_tools::arpeggiate(notes, &selected, rate, style, seed));
                format!("Arpeggiated ({})", style.name())
            }
            NoteTool::Strum => {
                midi_tools::strum(notes, &selected, grid * 0.25);
                "Strummed".to_string()
            }
            NoteTool::Legato => {
                midi_tools::legato(notes, &selected, len);
                "Legato".to_string()
            }
            NoteTool::Humanize => {
                midi_tools::humanize(notes, &selected, 0.015, 0.15, seed);
                "Humanized timing and velocity".to_string()
            }
            NoteTool::Seed => {
                let (low, high) = if notes.is_empty() {
                    (scale.root + 57, scale.root + 76)
                } else {
                    let lo = notes.iter().map(|n| n.pitch).min().unwrap_or(60);
                    let hi = notes.iter().map(|n| n.pitch).max().unwrap_or(72);
                    (lo.min(hi.saturating_sub(12)), hi.max(lo.saturating_add(12)))
                };
                let generated = midi_tools::seed_melody(&scale, len, grid.max(0.125), 0.55, low, high, seed);
                let start = notes.len();
                notes.extend(generated);
                new_sel = Some((start..notes.len()).collect());
                format!("Generated a melody in {}", scale.name())
            }
        };
        if let Some(ns) = new_sel {
            self.piano.selected = ns.into_iter().collect();
        }
        self.toast(msg);
    }

    /// Rows from top to bottom: all 128 pitches, the scale notes when folded,
    /// or just the drum pads.
    pub(crate) fn piano_rows(&self, track: usize) -> (Vec<u8>, Vec<String>, bool) {
        if let Some(Instrument::Drums { pads, .. }) = self.project.tracks.get(track).and_then(|t| t.instrument.as_ref()) {
            let mut v: Vec<(u8, String)> = pads.iter().map(|p| (p.note, p.name.clone())).collect();
            // A kit with an 808 that takes the keys below the drums: those keys too.
            if let Some(k) = krabton_engine::dsp::drums::keys_pad(pads) {
                let name = &pads[k].name;
                v.extend(krabton_engine::dsp::drums::KEYS_808.filter(|n| pads.iter().all(|p| p.note != *n)).map(|n| (n, format!("{name} {}", note_name(n)))));
            }
            v.sort_by(|a, b| b.0.cmp(&a.0));
            v.dedup_by_key(|x| x.0);
            let (notes, names) = v.into_iter().unzip();
            return (notes, names, true);
        }
        // Analog Princess: its tracks, and the keys below them that play its 808.
        if let Some(Instrument::Princess { .. }) = self.project.tracks.get(track).and_then(|t| t.instrument.as_ref()) {
            use krabton_engine::dsp::princess_machine as pm;
            let mut v: Vec<(u8, String)> = pm::NOTES.iter().zip(pm::TRACK_NAMES).map(|(&n, name)| (n, name.to_string())).collect();
            v.extend(pm::KEYS_808.map(|n| (n, format!("808 {}", note_name(n)))));
            v.sort_by(|a, b| b.0.cmp(&a.0));
            let (notes, names) = v.into_iter().unzip();
            return (notes, names, true);
        }
        // Grit: its sixteen pads.
        if let Some(Instrument::Grit { pads, .. }) = self.project.tracks.get(track).and_then(|t| t.instrument.as_ref()) {
            use krabton_engine::dsp::grit as gr;
            let mut v: Vec<(u8, String)> = (0..gr::PADS).map(|k| (gr::FIRST_NOTE + k as u8, pads.get(k).map(|p| p.label()).filter(|l| !l.is_empty()).map_or(format!("Pad {}", k + 1), |l| format!("{} {l}", k + 1)))).collect();
            v.sort_by(|a, b| b.0.cmp(&a.0));
            let (notes, names) = v.into_iter().unzip();
            return (notes, names, true);
        }
        // Turntable: the notes that play the deck.
        if let Some(Instrument::Turntable { .. }) = self.project.tracks.get(track).and_then(|t| t.instrument.as_ref()) {
            let mut v = krabton_engine::dsp::turntable::note_names();
            v.sort_by(|a, b| b.0.cmp(&a.0));
            let (notes, names) = v.into_iter().unzip();
            return (notes, names, true);
        }
        let scale = self.project.scale;
        let fold = self.piano.fold && self.project.scale_mode;
        let notes: Vec<u8> = (0..=127u8).rev().filter(|&p| !fold || scale.contains(p)).collect();
        let names = notes.iter().map(|&n| note_name(n)).collect();
        (notes, names, false)
    }

    /// Notes of other melodic tracks that sound with clip `sel`, in its own
    /// time (0..`len`): the same scene in the session, overlapping clips in
    /// the arrangement. As (pitch, start, length, colour).
    fn ghost_notes(&self, sel: ClipSel, len: f64) -> Vec<(u8, f64, f64, Color32)> {
        let mut out = Vec::new();
        let melodic = |t: &krabton_engine::Track| t.kind == krabton_engine::TrackKind::Midi && !matches!(t.instrument, Some(krabton_engine::Instrument::Drums { .. }));
        match sel {
            ClipSel::Slot { track, scene } => {
                for (i, t) in self.project.tracks.iter().enumerate() {
                    if i == track || !melodic(t) {
                        continue;
                    }
                    let Some(Some(c)) = t.slots.get(scene) else { continue };
                    let glen = c.length.max(0.0625);
                    for k in 0..((len / glen).ceil() as usize).min(64) {
                        for n in c.notes() {
                            let s = n.start + k as f64 * glen;
                            if n.start < glen && s < len {
                                out.push((n.pitch, s, n.length.min(len - s), rgb(t.color)));
                            }
                        }
                    }
                }
            }
            ClipSel::Arr { track, id } => {
                let Some(me) = self.project.tracks.get(track).and_then(|t| t.arrangement.iter().find(|a| a.id == id)) else { return out };
                for (i, t) in self.project.tracks.iter().enumerate() {
                    if i == track || !melodic(t) {
                        continue;
                    }
                    for o in t.arrangement.iter().filter(|o| o.start < me.end() && o.end() > me.start) {
                        let glen = o.clip.length.max(0.0625);
                        let k0 = (o.offset / glen).floor() as i64;
                        let k1 = ((o.offset + o.length) / glen).ceil() as i64;
                        for k in k0..k1.min(k0 + 256) {
                            for n in o.clip.notes() {
                                // Song time of this note, then this clip's own time.
                                let song = o.start - o.offset + k as f64 * glen + n.start;
                                if song < o.start || song >= o.end() || song < me.start || song >= me.end() {
                                    continue;
                                }
                                let local = song - me.start + me.offset;
                                if (0.0..len.max(me.length)).contains(&local) {
                                    out.push((n.pitch, local, n.length, rgb(t.color)));
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }

    pub(crate) fn ui_piano_roll(&mut self, ui: &mut Ui, area: Rect, sel: ClipSel) {
        let Some(clip) = self.clip_ref(sel) else { return };
        let len = clip.length.max(0.0625);
        let base = rgb(clip.color);
        let notes: Vec<Note> = clip.notes().to_vec();
        let track = sel.track();
        let (rows, names, drums) = self.piano_rows(track);
        if rows.is_empty() {
            return;
        }
        let keys_w = if drums { DRUM_KEYS_W } else { KEYS_W };
        let ruler = Rect::from_min_max(pos2(area.left() + keys_w, area.top()), pos2(area.right(), area.top() + RULER_H));
        let grid_rect = Rect::from_min_max(pos2(area.left() + keys_w, ruler.bottom()), pos2(area.right(), area.bottom() - VEL_H));
        let keys = Rect::from_min_max(pos2(area.left(), ruler.bottom()), pos2(area.left() + keys_w, grid_rect.bottom()));
        let vel = Rect::from_min_max(pos2(grid_rect.left(), grid_rect.bottom() + 2.0), area.max);
        if grid_rect.height() < 20.0 || grid_rect.width() < 40.0 {
            return;
        }
        let scale = self.project.scale;
        let scale_on = self.project.scale_mode && !drums;
        let folded = scale_on && self.piano.fold;
        let row_h = if drums {
            (grid_rect.height() / rows.len() as f32).clamp(14.0, 28.0)
        } else if folded {
            FOLD_ROW_H
        } else {
            ROW_H
        };

        // New clip shown (or rows folded/unfolded): center on the notes.
        let new_clip = self.piano.key != Some(sel);
        if new_clip {
            self.piano.key = Some(sel);
            self.piano.selected.clear();
            self.piano.drag = None;
            self.piano.paste_at = None;
            self.piano.time_sel = None;
            self.piano.zoom = 1.0;
            self.piano.scroll_x = 0.0;
        }
        if new_clip || self.piano.last_rows != rows.len() {
            self.piano.last_rows = rows.len();
            if !drums {
                let center = if notes.is_empty() { 60.0 } else { notes.iter().map(|n| n.pitch as f32).sum::<f32>() / notes.len() as f32 };
                let center = center.round().clamp(0.0, 127.0) as u8;
                let idx = rows.iter().position(|&r| r <= center).unwrap_or(rows.len() / 2) as f32;
                self.piano.scroll_y = idx * row_h - grid_rect.height() * 0.5;
            } else {
                self.piano.scroll_y = 0.0;
            }
        }

        // Scroll / zoom.
        if ui.rect_contains_pointer(grid_rect.union(keys).union(vel)) {
            let (d, shift, zoom) = ui.input(|i| (i.smooth_scroll_delta, i.modifiers.shift, i.zoom_delta()));
            if zoom != 1.0 {
                self.piano.zoom = (self.piano.zoom * zoom).clamp(1.0, 32.0);
            } else if shift {
                self.piano.scroll_x -= if d.x != 0.0 { d.x } else { d.y };
            } else {
                self.piano.scroll_y -= d.y;
                self.piano.scroll_x -= d.x;
            }
        }
        let content_h = rows.len() as f32 * row_h;
        self.piano.scroll_y = self.piano.scroll_y.clamp(0.0, (content_h - grid_rect.height()).max(0.0));
        let bpb = self.project.beats_per_bar_f();
        // Notes past the loop's end stay (as in Live): the view reaches to
        // the bar they end in, the part past the loop dark.
        let past = notes.iter().map(|n| n.end()).fold(len, f64::max);
        let view_len = if past > len + 1e-9 { (past / bpb - 1e-9).ceil() * bpb } else { len };
        let ppb = grid_rect.width() / view_len as f32 * self.piano.zoom;
        let content_w = view_len as f32 * ppb;
        self.piano.scroll_x = self.piano.scroll_x.clamp(0.0, (content_w - grid_rect.width()).max(0.0));
        let (sx, sy) = (self.piano.scroll_x, self.piano.scroll_y);
        self.piano.last_view = Some((grid_rect, ppb, sx));
        let bx = |b: f64| grid_rect.left() + b as f32 * ppb - sx;
        let xb = |x: f32| ((x - grid_rect.left() + sx) / ppb) as f64;
        let row_of = |pitch: u8| rows.iter().position(|&r| r == pitch);
        let ry = |row: usize| grid_rect.top() + row as f32 * row_h - sy;
        let row_at = |y: f32| -> Option<usize> {
            let r = ((y - grid_rect.top() + sy) / row_h).floor();
            (r >= 0.0 && (r as usize) < rows.len()).then_some(r as usize)
        };
        let grid = self.piano.grid;

        let p = ui.painter().with_clip_rect(area);
        let gp = ui.painter().with_clip_rect(grid_rect);
        rect_fill(&p, area, 0.0, BG);

        // Row backgrounds.
        for (ri, &pitch) in rows.iter().enumerate() {
            let y = ry(ri);
            if y + row_h < grid_rect.top() || y > grid_rect.bottom() {
                continue;
            }
            let r = Rect::from_min_size(pos2(grid_rect.left(), y), vec2(grid_rect.width(), row_h));
            let fill = if drums {
                if ri % 2 == 0 {
                    PANEL2
                } else {
                    shade(PANEL2, 0.85)
                }
            } else if scale_on {
                // Scale rows lit, the root brighter, everything else dark.
                if scale.is_root(pitch) {
                    mix(PANEL2, accent_now(), 0.22)
                } else if scale.contains(pitch) {
                    mix(PANEL2, accent_now(), 0.07)
                } else {
                    shade(PANEL2, 0.62)
                }
            } else if is_black(pitch) {
                shade(PANEL2, 0.72)
            } else {
                PANEL2
            };
            gp.rect_filled(r, 0.0, fill);
            let octave_line = if scale_on { scale.is_root(pitch) } else { pitch % 12 == 0 };
            if !drums && octave_line {
                gp.line_segment([pos2(r.left(), r.bottom()), pos2(r.right(), r.bottom())], Stroke::new(1.0_f32, LINE));
            }
        }
        // Beyond the loop end.
        let end_x = bx(len);
        if end_x < grid_rect.right() {
            gp.rect_filled(Rect::from_min_max(pos2(end_x, grid_rect.top()), grid_rect.max), 0.0, Color32::from_black_alpha(140));
        }
        // Vertical grid lines.
        let mut b = 0.0;
        while b <= view_len + 1e-9 {
            let x = bx(b);
            let is_bar = (b / bpb).fract().abs() < 1e-9;
            let is_beat = b.fract().abs() < 1e-9;
            let c = if is_bar {
                LINE
            } else if is_beat {
                shade(LINE, 0.75)
            } else {
                shade(LINE_SOFT, 0.85)
            };
            if ppb * grid as f32 >= 4.0 || is_beat {
                gp.line_segment([pos2(x, grid_rect.top()), pos2(x, grid_rect.bottom())], Stroke::new(1.0_f32, c));
            }
            if is_beat {
                let label = format!("{}.{}", (b / bpb).floor() as i64 + 1, (b % bpb) as i64 + 1);
                if ppb > 18.0 || is_bar {
                    p.with_clip_rect(ruler).text(pos2(x + 2.0, ruler.center().y), Align2::LEFT_CENTER, label, small(), TEXT_DIM);
                }
            }
            b += grid.max(1.0 / 64.0);
        }

        // Ghost notes: other tracks' clips that play alongside this one.
        if self.piano.ghosts && !drums {
            for (pitch, start, length, color) in self.ghost_notes(sel, len) {
                let Some(row) = row_of(pitch) else { continue };
                let y = ry(row);
                let r = Rect::from_min_max(pos2(bx(start) + 0.5, y + 2.0), pos2(bx(start + length).max(bx(start) + 3.0) - 0.5, y + row_h - 2.0));
                if r.intersects(grid_rect) {
                    gp.rect_filled(r, 2.0, color.gamma_multiply(0.22));
                }
            }
        }

        // Notes.
        let note_rect = |n: &Note| -> Option<Rect> {
            let row = row_of(n.pitch)?;
            let y = ry(row);
            Some(Rect::from_min_max(pos2(bx(n.start) + 0.5, y + 1.0), pos2(bx(n.start + n.length).max(bx(n.start) + 3.0) - 0.5, y + row_h - 1.0)))
        };
        for (idx, n) in notes.iter().enumerate() {
            let Some(r) = note_rect(n) else { continue };
            if !r.intersects(grid_rect) {
                continue;
            }
            let selected = self.piano.selected.contains(&idx);
            let fill = if selected { mix(base, Color32::WHITE, 0.55) } else { shade(base, 0.45 + 0.55 * n.velocity) };
            gp.rect_filled(r, 2.0, fill);
            // Operators: chance dims the note, repeats divide it.
            if n.chance < 1.0 {
                gp.rect_filled(r, 2.0, Color32::from_black_alpha(((1.0 - n.chance) * 150.0) as u8));
            }
            let reps = n.repeats.max(1);
            for i in 1..reps {
                let x = r.left() + r.width() * i as f32 / reps as f32;
                gp.line_segment([pos2(x, r.top() + 2.0), pos2(x, r.bottom() - 2.0)], Stroke::new(1.0_f32, Color32::from_black_alpha(140)));
            }
            gp.rect_stroke(r, 2.0, Stroke::new(1.0_f32, if selected { Color32::WHITE } else { shade(base, 0.35) }), egui::StrokeKind::Inside);
            if n.has_operators() && r.width() > 30.0 && row_h >= 11.0 {
                let mut tag = Vec::new();
                if n.chance < 1.0 {
                    tag.push(format!("{:.0}%", n.chance * 100.0));
                }
                if n.recur_len >= 2 {
                    tag.push(format!("{}/{}", n.recur_mask.count_ones().min(n.recur_len as u32), n.recur_len));
                }
                gp.with_clip_rect(r).text(pos2(r.left() + 3.0, r.center().y), Align2::LEFT_CENTER, tag.join(" "), egui::FontId::proportional((row_h - 3.0).clamp(7.0, 10.0)), TEXT_DARK);
            }
            if n.start >= len {
                gp.rect_filled(r, 2.0, Color32::from_black_alpha(120));
            }
            // Its lyric, just above it.
            if !n.lyric.is_empty() {
                let lyric: String = n.lyric.chars().filter(|c| !krabton_engine::singer::kana::invisible(*c)).collect();
                gp.text(pos2(r.left() + 1.0, r.top() - 1.0), Align2::LEFT_BOTTOM, lyric, egui::FontId::proportional(11.0), if selected { Color32::WHITE } else { TEXT });
            }
        }
        if let Some(m) = self.piano.marquee {
            gp.rect_filled(m, 0.0, Color32::from_rgba_unmultiplied(160, 216, 255, 30));
            gp.rect_stroke(m, 0.0, Stroke::new(1.0_f32, SELECT), egui::StrokeKind::Inside);
        }
        // The time selection over the ruler (Ctrl+V and Ctrl+D go right after
        // it), or with nothing selected the insert marker: where Ctrl+V puts notes.
        let time_sel = if self.piano.selected.is_empty() { None } else { self.notes_range(sel) };
        if let Some((a, b)) = time_sel {
            let r = Rect::from_min_max(pos2(bx(a), ruler.top()), pos2(bx(b), ruler.bottom()));
            p.with_clip_rect(ruler).rect_filled(r, 0.0, accent_now().gamma_multiply(0.35));
            gp.rect_filled(Rect::from_min_max(pos2(r.left(), grid_rect.top()), pos2(r.right(), grid_rect.bottom())), 0.0, Color32::from_white_alpha(8));
        } else if let Some(at) = self.piano.paste_at {
            let x = bx(at);
            gp.line_segment([pos2(x, grid_rect.top()), pos2(x, grid_rect.bottom())], Stroke::new(1.0_f32, accent_now()));
            p.with_clip_rect(ruler).add(egui::Shape::convex_polygon(
                vec![pos2(x - 4.0, ruler.top()), pos2(x + 4.0, ruler.top()), pos2(x, ruler.top() + 6.0)],
                accent_now(),
                Stroke::NONE,
            ));
        }
        if let Some(pos) = self.clip_play_pos(sel) {
            let x = bx(pos);
            gp.line_segment([pos2(x, grid_rect.top()), pos2(x, grid_rect.bottom())], Stroke::new(1.5_f32, Color32::WHITE));
            p.with_clip_rect(ruler).line_segment([pos2(x, ruler.top()), pos2(x, ruler.bottom())], Stroke::new(1.5_f32, Color32::WHITE));
        }

        // Keyboard strip.
        let kp = ui.painter().with_clip_rect(keys);
        for (ri, &pitch) in rows.iter().enumerate() {
            let y = ry(ri);
            if y + row_h < keys.top() || y > keys.bottom() {
                continue;
            }
            let r = Rect::from_min_size(pos2(keys.left(), y), vec2(keys_w - 1.0, row_h));
            let black = !drums && is_black(pitch);
            let held = self.piano.preview.as_ref().map(|p| p.0 == pitch).unwrap_or(false);
            let fill = if held {
                accent_now()
            } else if drums {
                PANEL3
            } else if black {
                Color32::from_gray(30)
            } else {
                Color32::from_gray(200)
            };
            if black {
                // White key body behind, short black key on top.
                kp.rect_filled(r.shrink2(vec2(0.0, 0.5)), 0.0, Color32::from_gray(200));
                kp.rect_filled(Rect::from_min_size(r.min, vec2(keys_w * 0.6, row_h)), 1.0, fill);
            } else {
                kp.rect_filled(r.shrink2(vec2(0.0, 0.5)), 1.0, fill);
                kp.line_segment([pos2(r.left(), r.bottom()), pos2(r.right(), r.bottom())], Stroke::new(1.0_f32, Color32::from_gray(150)));
            }
            if scale_on && scale.contains(pitch) {
                // Scale notes get a dot on the keyboard, the root in full accent.
                let c = if scale.is_root(pitch) { accent_now() } else { shade(accent_now(), 0.55) };
                kp.circle_filled(pos2(r.left() + 6.0, r.center().y), 2.5, c);
            }
            let label = drums || folded || pitch % 12 == 0 || (scale_on && scale.is_root(pitch));
            if label {
                let tc = if drums || black { TEXT } else { TEXT_DARK };
                kp.text(pos2(r.right() - 4.0, r.center().y), Align2::RIGHT_CENTER, &names[ri], small(), tc);
            }
        }
        let kresp = ui.interact(keys, Id::new("piano_keys"), Sense::click_and_drag());
        let down = kresp.is_pointer_button_down_on();
        let key_pitch = kresp.interact_pointer_pos().and_then(|p| row_at(p.y)).map(|r| rows[r]);
        match (&self.piano.preview, down, key_pitch) {
            (None, true, Some(pitch)) => {
                let ids = self.preview_note_on(track, pitch, 0.8);
                self.piano.preview = Some((pitch, ids));
            }
            (Some((cur, _)), true, Some(pitch)) if *cur != pitch => {
                if let Some((old, ids)) = self.piano.preview.take() {
                    self.note_off(old, &ids);
                }
                let ids = self.preview_note_on(track, pitch, 0.8);
                self.piano.preview = Some((pitch, ids));
            }
            (Some(_), false, _) => {
                if let Some((old, ids)) = self.piano.preview.take() {
                    self.note_off(old, &ids);
                }
            }
            _ => {}
        }

        // Grid interaction.
        let resp = ui.interact(grid_rect, Id::new("piano_grid"), Sense::click_and_drag());
        let free = ui.input(|i| i.modifiers.alt);
        let shift = ui.input(|i| i.modifiers.shift);
        let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
        let hit = |pos: Pos2| -> Option<(usize, bool)> {
            for (idx, n) in notes.iter().enumerate().rev() {
                if let Some(r) = note_rect(n) {
                    if r.expand(1.0).contains(pos) {
                        let resize = pos.x > r.right() - 6.0_f32.min(r.width() * 0.4);
                        return Some((idx, resize));
                    }
                }
            }
            None
        };
        if let Some(h) = resp.hover_pos() {
            match hit(h) {
                Some((_, true)) => ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal),
                Some((_, false)) => ui.ctx().set_cursor_icon(CursorIcon::Grab),
                None if self.piano.draw_mode => ui.ctx().set_cursor_icon(CursorIcon::Crosshair),
                None => {}
            }
        }
        let snap = |b: f64| if free { b } else { (b / grid).round() * grid };
        let snap_floor = |b: f64| if free { b } else { (b / grid).floor() * grid };

        if resp.drag_started() {
            let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or_default();
            match hit(origin) {
                Some((idx, resize)) => {
                    if !self.piano.selected.contains(&idx) {
                        if !shift {
                            self.piano.selected.clear();
                        }
                        self.piano.selected.insert(idx);
                        self.piano.time_sel = None;
                    }
                    let mut sel_idx: Vec<usize> = self.piano.selected.iter().copied().filter(|&i| i < notes.len()).collect();
                    sel_idx.sort_unstable();
                    // Ctrl+drag (Live 11) or Alt+drag (Live 12) drags a copy of
                    // the selection and leaves the notes where they were.
                    let copy = (ctrl || free) && !resize;
                    if copy {
                        if let Some(v) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
                            let start = v.len();
                            for &i in &sel_idx {
                                let n = v[i].clone();
                                v.push(n);
                            }
                            sel_idx = (start..v.len()).collect();
                        }
                        self.piano.selected = sel_idx.iter().copied().collect();
                    }
                    let originals = {
                        let all = self.clip_ref(sel).map(|c| c.notes().to_vec()).unwrap_or_default();
                        sel_idx.iter().filter_map(|&i| all.get(i).cloned().map(|n| (i, n))).collect()
                    };
                    let kind = if resize { DragKind::Resize } else { DragKind::Move };
                    self.piano.drag = Some(NoteDrag { kind, origin, originals, alt_copy: copy && free });
                }
                None if self.piano.draw_mode => {
                    if let Some(row) = row_at(origin.y) {
                        let start = snap_floor(xb(origin.x)).max(0.0);
                        let n = Note::new(rows[row], start, grid, 0.8);
                        if let Some(v) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
                            v.push(n.clone());
                            let idx = v.len() - 1;
                            self.piano.selected = [idx].into_iter().collect();
                            self.piano.time_sel = None;
                            self.piano.drag = Some(NoteDrag { kind: DragKind::Resize, origin, originals: vec![(idx, n)], alt_copy: false });
                        }
                    }
                }
                None => {
                    if !shift {
                        self.piano.selected.clear();
                    }
                    self.piano.drag = Some(NoteDrag { kind: DragKind::Marquee, origin, originals: Vec::new(), alt_copy: false });
                }
            }
        }
        if resp.dragged() {
            if let (Some(drag), Some(pos)) = (&self.piano.drag, resp.interact_pointer_pos()) {
                let dx = (pos.x - drag.origin.x) as f64 / ppb as f64;
                let dy = ((pos.y - drag.origin.y) / row_h).round() as i32;
                let kind = drag.kind;
                let originals = drag.originals.clone();
                let origin = drag.origin;
                let loose = free && !drag.alt_copy;
                match kind {
                    DragKind::Move => {
                        let d = if loose { dx } else { (dx / grid).round() * grid };
                        if let Some(v) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
                            for (i, o) in &originals {
                                if let Some(n) = v.get_mut(*i) {
                                    n.start = (o.start + d).clamp(0.0, view_len - 1e-6);
                                    let row = rows.iter().position(|&r| r == o.pitch).unwrap_or(0) as i32;
                                    let nr = (row + dy).clamp(0, rows.len() as i32 - 1) as usize;
                                    n.pitch = rows[nr];
                                }
                            }
                        }
                    }
                    DragKind::Resize => {
                        if let Some(v) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
                            for (i, o) in &originals {
                                if let Some(n) = v.get_mut(*i) {
                                    let end = snap(o.start + o.length + dx);
                                    n.length = (end - o.start).max(if free { 1.0 / 64.0 } else { grid.min(0.25) * 0.5 });
                                }
                            }
                        }
                    }
                    DragKind::Marquee => {
                        let m = Rect::from_two_pos(origin, pos);
                        self.piano.marquee = Some(m);
                        let mut s: HashSet<usize> = if shift { self.piano.selected.clone() } else { HashSet::new() };
                        for (idx, n) in notes.iter().enumerate() {
                            if note_rect(n).map(|r| r.intersects(m)).unwrap_or(false) {
                                s.insert(idx);
                            }
                        }
                        self.piano.selected = s;
                        // The box is the time selection (from grid line to grid line), as in Live.
                        let (a, b) = (xb(m.left()).max(0.0), xb(m.right()).max(0.0));
                        let (a, b) = (note_edit::grid_floor(a, grid), note_edit::grid_ceil(b, grid).max(a + grid));
                        self.piano.time_sel = if shift { None } else { Some((a, b)) };
                        if self.piano.selected.is_empty() {
                            self.piano.paste_at = Some(a);
                        }
                    }
                }
            }
        }
        if resp.drag_stopped() || (!ui.input(|i| i.pointer.any_down()) && self.piano.drag.is_some()) {
            // Dropped notes win over the ones they land on (as in Live).
            if self.piano.drag.take().is_some_and(|d| d.kind == DragKind::Move) {
                self.notes_make_room(sel);
                self.piano.time_sel = None;
            }
            self.piano.marquee = None;
        }
        if resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                match hit(pos) {
                    Some((idx, _)) => {
                        self.piano.time_sel = None;
                        if resp.double_clicked() || self.piano.draw_mode {
                            self.piano.selected.clear();
                            if let Some(v) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
                                if idx < v.len() {
                                    v.remove(idx);
                                }
                            }
                        } else if shift || ctrl {
                            if !self.piano.selected.remove(&idx) {
                                self.piano.selected.insert(idx);
                            }
                        } else {
                            self.piano.selected = [idx].into_iter().collect();
                        }
                    }
                    None => {
                        if resp.double_clicked() || self.piano.draw_mode {
                            if let Some(row) = row_at(pos.y) {
                                let start = snap_floor(xb(pos.x)).max(0.0);
                                if start < len {
                                    let pitch = rows[row];
                                    let n = Note::new(pitch, start, grid, 0.8);
                                    if let Some(v) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
                                        v.push(n);
                                        let idx = v.len() - 1;
                                        self.piano.selected = [idx].into_iter().collect();
                                        self.piano.time_sel = None;
                                    }
                                    if self.prefs.note_preview {
                                        let ids = self.preview_note_on(track, pitch, 0.8);
                                        self.note_off(pitch, &ids);
                                    }
                                }
                            }
                        } else if !shift {
                            self.piano.selected.clear();
                            self.piano.time_sel = None;
                            self.piano.paste_at = Some(snap_floor(xb(pos.x)).max(0.0));
                        }
                    }
                }
            }
        }

        // Right-click: Live's menu. A note not in the selection becomes it;
        // on an empty spot (nothing selected) the insert marker goes there.
        if resp.secondary_clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                match hit(pos) {
                    Some((idx, _)) if !self.piano.selected.contains(&idx) => {
                        self.piano.selected = [idx].into_iter().collect();
                        self.piano.time_sel = None;
                    }
                    None if self.piano.selected.is_empty() => self.piano.paste_at = Some(snap_floor(xb(pos.x)).max(0.0)),
                    _ => {}
                }
            }
            self.piano.focused = true;
        }
        let any = !self.piano.selected.is_empty();
        let can_paste = self.note_clipboard.is_some();
        let mut action = None;
        resp.context_menu(|ui| {
            let items = [("Copy", "Ctrl+C", any), ("Cut", "Ctrl+X", any), ("Paste", "Ctrl+V", can_paste), ("Duplicate", "Ctrl+D", any), ("Delete", "Del", any), ("Select All", "Ctrl+A", !notes.is_empty())];
            for (label, keys, enabled) in items {
                if ui.add_enabled(enabled, egui::Button::new(label).shortcut_text(keys)).clicked() {
                    action = Some(label);
                    ui.close();
                }
            }
        });
        if let Some(what) = action {
            self.piano.focused = true;
            match what {
                "Copy" => {
                    self.notes_copy();
                }
                "Cut" => self.notes_cut(),
                "Paste" => {
                    if let Some(clip) = self.note_clipboard.clone() {
                        self.notes_paste(clip);
                    }
                }
                "Duplicate" => self.notes_duplicate(),
                "Delete" => self.piano_delete_selected(),
                _ => self.piano_select_all(),
            }
        }

        // Velocity lane.
        let vp = ui.painter().with_clip_rect(vel);
        vp.rect_filled(vel, 0.0, PANEL);
        vp.line_segment([pos2(vel.left(), vel.top()), pos2(vel.right(), vel.top())], Stroke::new(1.0_f32, LINE));
        let notes_now: Vec<Note> = self.clip_ref(sel).map(|c| c.notes().to_vec()).unwrap_or_default();
        for (idx, n) in notes_now.iter().enumerate() {
            let x = bx(n.start);
            let h = (vel.height() - 6.0) * n.velocity;
            let selected = self.piano.selected.contains(&idx);
            let c = if selected { mix(base, Color32::WHITE, 0.5) } else { shade(base, 0.8) };
            vp.line_segment([pos2(x + 1.0, vel.bottom() - 2.0), pos2(x + 1.0, vel.bottom() - 2.0 - h)], Stroke::new(2.0_f32, c));
            vp.circle_filled(pos2(x + 1.0, vel.bottom() - 2.0 - h), 2.5, c);
        }
        let vresp = ui.interact(vel, Id::new("piano_vel"), Sense::click_and_drag());
        if vresp.drag_started() || vresp.clicked() {
            if let Some(pos) = vresp.interact_pointer_pos() {
                let near = notes_now
                    .iter()
                    .enumerate()
                    .map(|(i, n)| (i, (bx(n.start) - pos.x).abs()))
                    .filter(|(_, d)| *d < 6.0)
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(i, _)| i);
                self.piano.vel_drag = near.map(|i| if self.piano.selected.contains(&i) { self.piano.selected.iter().copied().collect() } else { vec![i] });
            }
        }
        if vresp.dragged() || vresp.clicked() {
            if let (Some(targets), Some(pos)) = (self.piano.vel_drag.clone(), vresp.interact_pointer_pos()) {
                let v = (1.0 - (pos.y - vel.top() - 3.0) / (vel.height() - 6.0)).clamp(0.01, 1.0);
                if let Some(notes) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
                    for i in targets {
                        if let Some(n) = notes.get_mut(i) {
                            n.velocity = v;
                        }
                    }
                }
                vresp.clone().on_hover_text(format!("Velocity {}", (v * 127.0).round()));
            }
        }
        if vresp.drag_stopped() {
            self.piano.vel_drag = None;
        }
        text(&vp, pos2(vel.left() + 4.0, vel.top() + 8.0), Align2::LEFT_CENTER, "Velocity", small(), TEXT_DIM);
        let _ = rect_fill;
    }
}
