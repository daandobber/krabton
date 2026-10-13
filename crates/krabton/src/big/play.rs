//! Big Picture's Play page: pads for the selected track. A melodic track
//! gets eight by four pads in the key (each row a fourth up, the roots lit),
//! a drum track a pad per drum with its name. Touch them (with as many
//! fingers as you like), or move over them with the D-pad and hold A; X and
//! Y move an octave, the triggers change the track. The pads light up with
//! every note the track plays, also from its clips and sequencers.

use super::input::{Frame, Nav};
use super::{fit_text, tile, Glyph, Sz, TILE};
use crate::app::{App, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Ui};
use std::collections::HashMap;

/// Who holds a note: the pad cursor, the mouse, or a finger (by touch id).
pub const HOLD_CURSOR: u64 = u64::MAX;
pub const HOLD_MOUSE: u64 = u64::MAX - 1;

#[derive(Default)]
pub struct PlayState {
    pub cursor: (i32, i32),
    pub octave: i32,
    /// Notes held, by who holds them: (the pitch played, the tracks).
    pub held: HashMap<u64, (u8, Vec<u64>)>,
    /// Which pad each holder is on.
    pub on_pad: HashMap<u64, (i32, i32)>,
    /// The triggers' last value (they change the track when pulled).
    pub last_turn: f32,
    /// A touch screen was used: its fingers, not the mouse it imitates.
    pub touch: bool,
}

/// The pads of a track: (columns, rows, pitch of each pad, name of each).
fn pads(app: &App, t: usize, octave: i32) -> (i32, i32, Vec<u8>, Vec<String>) {
    let track = &app.project.tracks[t];
    if let Some(krabton_engine::Instrument::Drums { pads, .. }) = &track.instrument {
        let n = pads.len().clamp(1, 16);
        let cols = 4;
        let rows = (n as i32 + cols - 1) / cols;
        // The first pad bottom left, as on a drum machine.
        let mut pitches = vec![0u8; (cols * rows) as usize];
        let mut names = vec![String::new(); (cols * rows) as usize];
        for (i, pad) in pads.iter().take(16).enumerate() {
            let (c, r) = (i as i32 % cols, rows - 1 - i as i32 / cols);
            pitches[(r * cols + c) as usize] = pad.note;
            names[(r * cols + c) as usize] = pad.name.clone();
        }
        return (cols, rows, pitches, names);
    }
    let (cols, rows) = (8, 4);
    let scale = app.project.scale;
    let base = (4 + octave) * 12 + scale.root as i32;
    let (abs, _) = scale.locate(base);
    let mut pitches = Vec::new();
    let mut names = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let degree = c + (rows - 1 - r) * 3;
            let p = scale.pitch_at(abs + degree).clamp(0, 127) as u8;
            pitches.push(p);
            names.push(krabton_engine::note_name(p));
        }
    }
    (cols, rows, pitches, names)
}

impl App {
    fn play_track(&mut self) -> usize {
        let n = self.project.tracks.len();
        let t = match self.sel_track {
            TrackSel::Track(t) if t < n => t,
            _ => 0,
        };
        // The armed track is the one that hears the pads: when another is
        // armed, the arm moves here.
        if self.project.tracks.iter().enumerate().any(|(i, tr)| tr.arm && i != t) {
            for (i, tr) in self.project.tracks.iter_mut().enumerate() {
                tr.arm = i == t;
            }
        }
        t
    }

    fn pad_on(&mut self, who: u64, pad: (i32, i32), pitch: u8) {
        self.pad_off(who);
        let (played, ids) = self.note_on(pitch, 0.85);
        self.big.play.held.insert(who, (played, ids));
        self.big.play.on_pad.insert(who, pad);
    }

    fn pad_off(&mut self, who: u64) {
        if let Some((p, ids)) = self.big.play.held.remove(&who) {
            self.note_off(p, &ids);
        }
        self.big.play.on_pad.remove(&who);
    }

    pub(crate) fn big_play(&mut self, ui: &mut Ui, r: Rect, s: &Sz, mut inp: Frame) -> Vec<(Glyph, &'static str)> {
        if self.project.tracks.is_empty() {
            return Vec::new();
        }
        let mut t = self.play_track();
        // The triggers change the track.
        let turn = inp.turn;
        let last = self.big.play.last_turn;
        self.big.play.last_turn = turn;
        let n_tracks = self.project.tracks.len();
        let mut new_t = t;
        if turn > 0.6 && last <= 0.6 {
            new_t = (t + 1) % n_tracks;
        } else if turn < -0.6 && last >= -0.6 {
            new_t = (t + n_tracks - 1) % n_tracks;
        }
        let p = ui.painter().clone();
        let accent = rgb(self.project.tracks[t].color);
        // The head: the track (arrows to change it), the octave, recording.
        let head_h = s.u * 1.4;
        let head = Rect::from_min_size(r.min, vec2(r.width(), head_h));
        let arrow_w = s.u * 1.5;
        let left = Rect::from_min_size(head.min, vec2(arrow_w, head_h - s.u * 0.2));
        let right = Rect::from_min_size(pos2(left.right() + s.u * 7.6, head.top()), vec2(arrow_w, head_h - s.u * 0.2));
        for (rr, by, label) in [(left, -1i32, "<"), (right, 1, ">")] {
            let resp = ui.interact(rr, Id::new(("big_play_track", by)), Sense::click());
            rect_fill(&p, rr, 8.0, if resp.hovered() { super::TILE_HI } else { TILE });
            text(&p, rr.center(), Align2::CENTER_CENTER, label, s.big(), TEXT);
            if resp.clicked() {
                new_t = ((t as i32 + by).rem_euclid(n_tracks as i32)) as usize;
            }
        }
        if new_t != t {
            self.big_release_notes();
            self.sel_track = TrackSel::Track(new_t);
            t = self.play_track();
        }
        let track = &self.project.tracks[t];
        let name_r = Rect::from_min_max(pos2(left.right() + s.u * 0.3, head.top()), pos2(right.left() - s.u * 0.3, head.bottom() - s.u * 0.2));
        rect_fill(&p, name_r, 8.0, shade(accent, 0.35));
        let inst = track.instrument.as_ref().map(|i| format!("{} · {}", i.kind_name(), i.display_name())).unwrap_or_default();
        fit_text(&p, pos2(name_r.left() + s.u * 0.3, name_r.top() + name_r.height() * 0.34), Align2::LEFT_CENTER, &track.name, s.font, name_r.width() - s.u * 0.6, TEXT);
        fit_text(&p, pos2(name_r.left() + s.u * 0.3, name_r.top() + name_r.height() * 0.74), Align2::LEFT_CENTER, &inst, s.small, name_r.width() - s.u * 0.6, TEXT_DIM);
        let drums = matches!(track.instrument, Some(krabton_engine::Instrument::Drums { .. }));
        let mut x = right.right() + s.u * 0.5;
        if !drums {
            for (by, label) in [(-1i32, "Oct -"), (1, "Oct +")] {
                let br = Rect::from_min_size(pos2(x, head.top()), vec2(s.u * 2.6, head_h - s.u * 0.2));
                let resp = ui.interact(br, Id::new(("big_oct", by)), Sense::click());
                rect_fill(&p, br, 8.0, if resp.hovered() { super::TILE_HI } else { TILE });
                text(&p, br.center(), Align2::CENTER_CENTER, label, s.f(), TEXT);
                if resp.clicked() {
                    self.big.play.octave = (self.big.play.octave + by).clamp(-3, 4);
                }
                x += s.u * 2.8;
            }
            text(&p, pos2(x + s.u * 0.2, head.top() + head_h * 0.45), Align2::LEFT_CENTER, format!("C{}", 4 + self.big.play.octave - 1), s.f(), TEXT_DIM);
        }
        let rec_r = Rect::from_min_size(pos2(head.right() - s.u * 3.2, head.top()), vec2(s.u * 3.2, head_h - s.u * 0.2));
        let rresp = ui.interact(rec_r, Id::new("big_play_rec"), Sense::click());
        rect_fill(&p, rec_r, 8.0, if self.recording { REC } else { TILE });
        rec_icon(&p, pos2(rec_r.left() + s.u * 0.6, rec_r.center().y), s.u * 0.45, if self.recording { TEXT } else { REC });
        text(&p, pos2(rec_r.left() + s.u * 1.1, rec_r.center().y), Align2::LEFT_CENTER, "Rec", s.f(), TEXT);
        if rresp.on_hover_text("Record what you play into a new clip").clicked() {
            self.toggle_record();
        }

        // The pads.
        let (cols, rows, pitches, names) = pads(self, t, self.big.play.octave);
        let area = Rect::from_min_max(pos2(r.left(), head.bottom() + s.u * 0.2), r.max);
        let gap = s.u * 0.18;
        let pw = (area.width() - gap * (cols - 1) as f32) / cols as f32;
        let ph = (area.height() - gap * (rows - 1) as f32) / rows as f32;
        let side = if drums { pw.min(ph) } else { pw };
        let ph = if drums { side } else { ph };
        let grid_w = side * cols as f32 + gap * (cols - 1) as f32;
        let x0 = area.left() + (area.width() - grid_w) * 0.5;
        let pad_rect = |c: i32, rr: i32| Rect::from_min_size(pos2(x0 + c as f32 * (side + gap), area.top() + rr as f32 * (ph + gap)), vec2(side, ph));
        let pad_at = |pt: egui::Pos2| -> Option<(i32, i32)> {
            (0..rows).flat_map(|rr| (0..cols).map(move |c| (c, rr))).find(|&(c, rr)| pad_rect(c, rr).contains(pt))
        };
        // The cursor and A.
        let (mut cx, mut cy) = self.big.play.cursor;
        cx = cx.clamp(0, cols - 1);
        cy = cy.clamp(0, rows - 1);
        let mut a_down = None;
        for nav in inp.pressed.drain(..) {
            match nav {
                Nav::Left => cx = (cx - 1).max(0),
                Nav::Right => cx = (cx + 1).min(cols - 1),
                Nav::Up => cy = (cy - 1).max(0),
                Nav::Down => cy = (cy + 1).min(rows - 1),
                Nav::A => a_down = Some(true),
                Nav::X if !drums => self.big.play.octave = (self.big.play.octave - 1).max(-3),
                Nav::Y if !drums => self.big.play.octave = (self.big.play.octave + 1).min(4),
                _ => {}
            }
        }
        if inp.released.contains(&Nav::A) {
            a_down = Some(false);
        }
        let pitch_of = |c: i32, rr: i32| pitches.get((rr * cols + c) as usize).copied().unwrap_or(60);
        match a_down {
            Some(true) => self.pad_on(HOLD_CURSOR, (cx, cy), pitch_of(cx, cy)),
            Some(false) => self.pad_off(HOLD_CURSOR),
            None => {
                // Held A slides along with the cursor.
                if self.big.play.on_pad.get(&HOLD_CURSOR).is_some_and(|&pad| pad != (cx, cy)) {
                    self.pad_on(HOLD_CURSOR, (cx, cy), pitch_of(cx, cy));
                }
            }
        }
        self.big.play.cursor = (cx, cy);
        // Fingers.
        let touches: Vec<(u64, egui::TouchPhase, egui::Pos2)> = ui.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Touch { id, phase, pos, .. } => Some((id.0, *phase, *pos)),
                    _ => None,
                })
                .collect()
        });
        if !touches.is_empty() {
            self.big.play.touch = true;
        }
        for (id, phase, pos) in touches {
            match phase {
                egui::TouchPhase::Start | egui::TouchPhase::Move => match pad_at(pos) {
                    Some(pad) if self.big.play.on_pad.get(&id) != Some(&pad) => self.pad_on(id, pad, pitch_of(pad.0, pad.1)),
                    Some(_) => {}
                    None => self.pad_off(id),
                },
                _ => self.pad_off(id),
            }
        }
        // The mouse (when there is no touch screen).
        if !self.big.play.touch {
            let (down, pos) = ui.input(|i| (i.pointer.primary_down(), i.pointer.interact_pos()));
            match (down, pos.and_then(pad_at)) {
                (true, Some(pad)) if self.big.play.on_pad.get(&HOLD_MOUSE) != Some(&pad) => self.pad_on(HOLD_MOUSE, pad, pitch_of(pad.0, pad.1)),
                (true, Some(_)) => {}
                _ => self.pad_off(HOLD_MOUSE),
            }
        }
        // What sounds now (the track's notes, from anywhere).
        let st = self.status();
        let sounding = st.tracks.get(t).map(|ts| [ts.inst_held[0].load(std::sync::atomic::Ordering::Relaxed), ts.inst_held[1].load(std::sync::atomic::Ordering::Relaxed)]).unwrap_or([0, 0]);
        let sounds = |pitch: u8| (sounding[(pitch / 64) as usize] >> (pitch % 64)) & 1 == 1;
        let held: Vec<(i32, i32)> = self.big.play.on_pad.values().copied().collect();
        let scale = self.project.scale;
        for rr in 0..rows {
            for c in 0..cols {
                let pr = pad_rect(c, rr);
                let pitch = pitch_of(c, rr);
                let name = &names[(rr * cols + c) as usize];
                let pressed = held.contains(&(c, rr));
                let lit = pressed || sounds(pitch);
                let root = !drums && scale.is_root(pitch);
                let base = if drums {
                    if name.is_empty() {
                        Color32::from_rgb(0x1c, 0x1c, 0x20)
                    } else {
                        shade(accent, 0.45)
                    }
                } else if root {
                    shade(accent, 0.7)
                } else {
                    TILE
                };
                let fill = if lit { mix(accent, Color32::WHITE, 0.35) } else { base };
                tile(&p, pr, fill, (c, rr) == (cx, cy));
                fit_text(&p, pr.center(), Align2::CENTER_CENTER, name, s.font, pr.width() - 8.0, if lit { TEXT_DARK } else { TEXT });
            }
        }
        if drums {
            vec![(Glyph::Dpad, "Move"), (Glyph::Btn(Nav::A), "Play"), (Glyph::Triggers, "Track")]
        } else {
            vec![(Glyph::Dpad, "Move"), (Glyph::Btn(Nav::A), "Play"), (Glyph::Btn(Nav::X), "Oct -"), (Glyph::Btn(Nav::Y), "Oct +"), (Glyph::Triggers, "Track")]
        }
    }
}
