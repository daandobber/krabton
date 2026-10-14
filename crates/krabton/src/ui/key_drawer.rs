//! The Key drawer: the whole song in another key at a press, and keys in
//! sequence. Its parts, from the most musical to the widest:
//!
//! - *In the key*: the chords of the song's key (I ii iii IV V vi vii°).
//!   A press moves the song onto that chord without leaving the key's
//!   notes (the IV of C is F Lydian), so it sounds like a chord change.
//! - *Moves*: the usual ways out: the relative and parallel key, a semitone
//!   or a whole tone up (the key lift at the end of a song), a fourth or a
//!   fifth.
//! - A keyboard: the root, the scale kept.
//! - *All keys*: chord buttons round the circle of fifths as on an
//!   omnichord, brighter the more notes they share with the song's key.
//! - *Sequence*: up to eight steps the song moves through, one every 1, 2
//!   or 4 bars while it plays (Run). Click a step, then press keys to fill
//!   it; progressions to start from.
//!
//! Drums stay; what you play sounds as you play it.

use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, RichText, Sense, Ui};
use krabton_engine::{Scale, ScaleKind};
use std::sync::atomic::Ordering;

const NAMES: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];

/// Progressions by degrees of the song's key.
const PROGRESSIONS: &[&[usize]] = &[&[0, 4, 5, 3], &[5, 3, 0, 4], &[0, 5, 3, 4], &[0, 3, 4, 3], &[0, 3, 0, 4], &[1, 4, 0, 0], &[0, 5, 1, 4], &[0, 6, 5, 4], &[0, 2, 3, 3], &[0, 0, 3, 3, 4, 4, 0, 0]];

/// A key's short name, as the chord on its root: D (major and Lydian), Dm
/// (minor, Dorian, Phrygian), D7 (Mixolydian), D° (Locrian), or the scale's name.
fn short(k: &Scale) -> String {
    let root = NAMES[k.root as usize % 12];
    match k.kind {
        ScaleKind::Major | ScaleKind::Lydian => root.to_string(),
        ScaleKind::Minor | ScaleKind::Dorian | ScaleKind::Phrygian => format!("{root}m"),
        ScaleKind::Mixolydian => format!("{root}7"),
        ScaleKind::Locrian => format!("{root}°"),
        other => format!("{root} {}", other.name()),
    }
}

/// What a press asks for: a key (None: the song's own).
type Pick = Option<Option<Scale>>;

impl App {
    /// The key the music is moved into now: the sequence's step while it
    /// runs, else the drawer's.
    pub(crate) fn current_play_key(&self) -> Option<Scale> {
        let p = &self.project;
        if p.key_seq_on && self.is_playing() {
            let beat = self.status().session_pos.load();
            if let Some(k) = p.key_seq_step(beat) {
                return p.key_seq[k];
            }
        }
        p.play_key
    }

    /// A button of the drawer: (pressed this frame, let go this frame).
    #[allow(clippy::too_many_arguments)]
    fn key_button(&self, ui: &mut Ui, r: Rect, id: Id, label: &str, sub: &str, on: bool, own: bool, glow: f32) -> (bool, bool) {
        let resp = ui.interact(r, id, Sense::click_and_drag());
        let p = ui.painter();
        let base = mix(PANEL2, PANEL3.gamma_multiply(1.5), glow.clamp(0.0, 1.0));
        p.rect_filled(r, 5.0, if on { ACCENT } else if resp.hovered() { base.gamma_multiply(1.3) } else { base });
        if own {
            p.rect_stroke(r, 5.0, egui::Stroke::new(1.5_f32, ACCENT), egui::StrokeKind::Inside);
        }
        let ink = if on { TEXT_DARK } else { TEXT };
        if sub.is_empty() {
            text(p, r.center(), Align2::CENTER_CENTER, label, FontId::proportional(13.0), ink);
        } else {
            text(p, pos2(r.center().x, r.top() + r.height() * 0.38), Align2::CENTER_CENTER, label, FontId::proportional(15.0), ink);
            text(p, pos2(r.center().x, r.top() + r.height() * 0.76), Align2::CENTER_CENTER, sub, FontId::proportional(11.0), if on { TEXT_DARK } else { TEXT_DIM });
        }
        let pressed = ui.input(|i| i.pointer.primary_pressed()) && resp.hovered();
        (pressed, resp.drag_stopped() || resp.clicked())
    }

    pub(crate) fn ui_transpose_drawer(&mut self, ui: &mut Ui) {
        let song = self.project.scale;
        let now_key = self.current_play_key();
        let now = now_key.unwrap_or(song);
        let moved = now_key.filter(|k| *k != song).is_some();
        let running = self.project.key_seq_on && self.is_playing();
        let mut pick: Pick = None;
        let mut released = false;

        // The head: what plays, and what can be done.
        ui.horizontal(|ui| {
            ui.label(RichText::new("Key").strong().size(16.0).color(TEXT));
            ui.add_space(8.0);
            let waiting = self.status().key_waiting.load(Ordering::Relaxed);
            let txt = if waiting {
                format!("{}  from the next bar", self.project.sounding_key().name())
            } else if moved {
                format!("{}  plays in  {}", song.name(), now.name())
            } else {
                format!("{}, the song's own", song.name())
            };
            let col = if waiting && !super::session::blink() { TEXT_DIM } else if moved || waiting { ACCENT } else { TEXT_DIM };
            ui.label(RichText::new(txt).size(14.0).color(col));
            if waiting || running {
                ui.ctx().request_repaint();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("x").on_hover_text("Close").clicked() {
                    self.transpose_open = false;
                }
                let all = self.key_all;
                if ui.selectable_label(all, "All keys").on_hover_text("Chord buttons for every key, round the circle of fifths").clicked() {
                    self.key_all = !all;
                }
                let on_bar = self.project.play_key_on_bar;
                if ui.selectable_label(on_bar, "Next bar").on_hover_text("On: a new key starts on the first beat of the next bar while the song plays. Off: at once").clicked() {
                    self.project.play_key_on_bar = !on_bar;
                }
                let latch = self.key_latch;
                if ui.selectable_label(latch, "Hold").on_hover_text("On: a press keeps the key. Off: the key lasts while you hold it, like a chord button").clicked() {
                    self.key_latch = !latch;
                }
                if self.project.play_key.is_some() {
                    if ui.button("Make it the key").on_hover_text("Write it into the set: every clip's notes move into this key and it becomes the song's key. It sounds the same").clicked() {
                        self.project.bake_play_key();
                        self.toast(format!("The song is in {} now", self.project.scale.name()));
                    }
                    if ui.button(format!("Back to {}", song.name())).clicked() {
                        self.project.play_key = None;
                    }
                }
            });
        });
        ui.add_space(6.0);

        ui.horizontal_top(|ui| {
            // The keyboard: a key is the root, the scale stays.
            let kw = 30.0;
            let kh = 92.0;
            let (rect, resp) = ui.allocate_exact_size(vec2(7.0 * kw, kh), Sense::click_and_drag());
            let p = ui.painter().clone();
            let whites = [0u8, 2, 4, 5, 7, 9, 11];
            let blacks = [(1u8, 0.0f32), (3, 1.0), (6, 3.0), (8, 4.0), (10, 5.0)];
            let white_r = |k: usize| Rect::from_min_size(pos2(rect.left() + k as f32 * kw, rect.top()), vec2(kw - 2.0, kh));
            let black_r = |after: f32| Rect::from_min_size(pos2(rect.left() + (after + 1.0) * kw - kw * 0.32, rect.top()), vec2(kw * 0.62, kh * 0.6));
            let hit = resp.interact_pointer_pos().or(resp.hover_pos()).and_then(|pt| {
                blacks.iter().find(|(_, a)| black_r(*a).contains(pt)).map(|b| b.0).or_else(|| (0..7).find(|&k| white_r(k).contains(pt)).map(|k| whites[k]))
            });
            for (k, &pc) in whites.iter().enumerate() {
                let r = white_r(k);
                let lit = now.root % 12 == pc;
                p.rect_filled(r, 3.0, if lit { ACCENT } else if hit == Some(pc) { Color32::from_gray(225) } else { Color32::from_gray(200) });
                text(&p, pos2(r.center().x, r.bottom() - 12.0), Align2::CENTER_CENTER, NAMES[pc as usize], FontId::proportional(12.0), TEXT_DARK);
                if song.root % 12 == pc {
                    p.circle_filled(pos2(r.center().x, r.bottom() - 27.0), 3.5, if lit { TEXT_DARK } else { ACCENT });
                }
            }
            for &(pc, after) in &blacks {
                let r = black_r(after);
                let lit = now.root % 12 == pc;
                p.rect_filled(r, 3.0, if lit { ACCENT } else if hit == Some(pc) { Color32::from_gray(70) } else { Color32::from_gray(35) });
                if song.root % 12 == pc {
                    p.circle_filled(pos2(r.center().x, r.bottom() - 10.0), 3.0, ACCENT);
                }
            }
            if ui.input(|i| i.pointer.primary_pressed()) && resp.hovered() {
                if let Some(pc) = hit {
                    pick = Some(Some(Scale::new(pc, now.kind)));
                }
            }
            released |= resp.drag_stopped() || resp.clicked();
            resp.on_hover_text("Press a key: the song plays from that root, in the same scale. The dot is the song's own key");
            ui.add_space(14.0);

            ui.vertical(|ui| {
                // In the key: its chords, staying in its notes.
                ui.label(RichText::new(format!("In {}", song.name())).small().color(TEXT_DIM));
                let n = song.len();
                let (bw, bh) = (62.0, 40.0);
                let (row, _) = ui.allocate_exact_size(vec2(bw * n.min(7) as f32, bh), Sense::hover());
                for d in 0..n.min(7) {
                    let k = song.degree_key(d);
                    let r = Rect::from_min_size(pos2(row.left() + d as f32 * bw, row.top()), vec2(bw - 4.0, bh));
                    let on = now == k;
                    let (pr, rl) = self.key_button(ui, r, Id::new(("key_deg", d)), &song.chord_name(d), &song.numeral(d), on, d == 0, 1.0);
                    if pr {
                        pick = Some(if d == 0 { None } else { Some(k) });
                    }
                    released |= rl;
                }
                ui.add_space(4.0);
                // The usual ways out of a key.
                let base = self.key_seq_sel.and_then(|s| s.checked_sub(1)).and_then(|s| self.project.key_seq.get(s).copied().flatten()).unwrap_or(now);
                let minorish = base.intervals().get(2) == Some(&3);
                let relative = if minorish { Scale::new((base.root + 3) % 12, ScaleKind::Major) } else { Scale::new((base.root + 9) % 12, ScaleKind::Minor) };
                let parallel = Scale::new(base.root, if minorish { ScaleKind::Major } else { ScaleKind::Minor });
                let up = |s: u8| Scale::new((base.root + s) % 12, base.kind);
                let moves: [(&str, &str, Scale); 7] = [
                    ("Relative", "", relative),
                    ("Parallel", "", parallel),
                    ("-1", "a semitone down", up(11)),
                    ("+1", "a semitone up", up(1)),
                    ("+2", "a whole tone up: the key lift", up(2)),
                    ("4th", "a fourth up", up(5)),
                    ("5th", "a fifth up", up(7)),
                ];
                let (mrow, _) = ui.allocate_exact_size(vec2(bw * 7.0, 26.0), Sense::hover());
                for (j, (label, tip, k)) in moves.iter().enumerate() {
                    let r = Rect::from_min_size(pos2(mrow.left() + j as f32 * bw, mrow.top()), vec2(bw - 4.0, 26.0));
                    let (pr, rl) = self.key_button(ui, r, Id::new(("key_move", j)), label, "", false, false, 0.4);
                    if pr {
                        pick = Some(if *k == song { None } else { Some(*k) });
                    }
                    released |= rl;
                    let tipr = ui.interact(r, Id::new(("key_move_tip", j)), Sense::hover());
                    tipr.on_hover_text(format!("{} ({})", if tip.is_empty() { label } else { tip }, k.name()));
                }
            });
            ui.add_space(14.0);

            // The sequence.
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Sequence").small().color(TEXT_DIM));
                    let run = self.project.key_seq_on;
                    let rb = egui::Button::new(RichText::new("Run").small().color(if run { TEXT_DARK } else { TEXT })).fill(if run { ACCENT } else { PANEL3 });
                    if ui.add(rb).on_hover_text("While the song plays it moves through the steps, a step every so many bars").clicked() {
                        self.project.key_seq_on = !run;
                        if self.project.key_seq.is_empty() {
                            self.project.key_seq = vec![None; 4];
                        }
                    }
                    let bars = self.project.key_seq_bars.max(1);
                    if ui.small_button(format!("{bars} bar{}", if bars > 1 { "s" } else { "" })).on_hover_text("How long a step lasts").clicked() {
                        self.project.key_seq_bars = match bars {
                            1 => 2,
                            2 => 4,
                            _ => 1,
                        };
                    }
                    let len = self.project.key_seq.len();
                    if ui.small_button("-").on_hover_text("One step fewer").clicked() && len > 1 {
                        self.project.key_seq.pop();
                    }
                    if ui.small_button("+").on_hover_text("One step more").clicked() && len < 8 {
                        self.project.key_seq.push(None);
                    }
                    let menu = ui.small_button("Progressions");
                    egui::Popup::menu(&menu).show(|ui| {
                        for prog in PROGRESSIONS {
                            let name = prog.iter().map(|&d| song.numeral(d)).collect::<Vec<_>>().join(" - ");
                            let chords = prog.iter().map(|&d| song.chord_name(d)).collect::<Vec<_>>().join(" ");
                            if ui.button(format!("{name}    {chords}")).clicked() {
                                self.project.key_seq = prog.iter().map(|&d| if d == 0 { None } else { Some(song.degree_key(d)) }).collect();
                                self.key_seq_sel = None;
                                ui.close();
                            }
                        }
                    });
                    if ui.small_button("Clear").clicked() {
                        self.project.key_seq.iter_mut().for_each(|s| *s = None);
                    }
                });
                if self.project.key_seq.is_empty() {
                    self.project.key_seq = vec![None; 4];
                }
                let len = self.project.key_seq.len();
                let step_now = if running { self.project.key_seq_step(self.status().session_pos.load()) } else { None };
                let (sw, sh) = (56.0, 48.0);
                let (row, _) = ui.allocate_exact_size(vec2(sw * len as f32, sh), Sense::hover());
                for k in 0..len {
                    let r = Rect::from_min_size(pos2(row.left() + k as f32 * sw, row.top()), vec2(sw - 4.0, sh));
                    let key = self.project.key_seq[k];
                    let resp = ui.interact(r, Id::new(("key_seq", k)), Sense::click());
                    let p = ui.painter();
                    let sel = self.key_seq_sel == Some(k);
                    let playing = step_now == Some(k);
                    p.rect_filled(r, 5.0, if playing { ACCENT } else if sel { PANEL3.gamma_multiply(1.6) } else { PANEL3 });
                    if sel {
                        p.rect_stroke(r, 5.0, egui::Stroke::new(1.5_f32, ACCENT), egui::StrokeKind::Inside);
                    }
                    let name = key.map(|k| short(&k)).unwrap_or_else(|| short(&song));
                    let degree = (0..song.len().min(7)).find(|&d| key.unwrap_or(song) == song.degree_key(d)).map(|d| song.numeral(d)).unwrap_or_default();
                    let ink = if playing { TEXT_DARK } else { TEXT };
                    text(p, pos2(r.center().x, r.top() + 17.0), Align2::CENTER_CENTER, &name, FontId::proportional(13.0), ink);
                    text(p, pos2(r.center().x, r.top() + 35.0), Align2::CENTER_CENTER, &degree, FontId::proportional(11.0), if playing { TEXT_DARK } else { TEXT_DIM });
                    if resp.clicked() {
                        self.key_seq_sel = if sel { None } else { Some(k) };
                    }
                    if resp.secondary_clicked() {
                        self.project.key_seq[k] = None;
                    }
                    resp.on_hover_text("Click to fill this step with the next key you press; right-click: the song's own key");
                }
                if self.key_seq_sel.is_some() {
                    ui.label(RichText::new("Press a key or a chord for the chosen step").small().color(ACCENT));
                }
            });
        });

        // All keys: chord buttons round the circle of fifths.
        if self.key_all {
            ui.add_space(6.0);
            let order: [u8; 12] = [1, 8, 3, 10, 5, 0, 7, 2, 9, 4, 11, 6];
            let rows: [(&str, ScaleKind); 3] = [("", ScaleKind::Major), ("m", ScaleKind::Minor), ("7", ScaleKind::Mixolydian)];
            let (bw, bh) = (44.0, 26.0);
            let (grid, _) = ui.allocate_exact_size(vec2(12.0 * bw, 3.0 * (bh + 4.0)), Sense::hover());
            let song_mask = song.mask();
            for (r, (suffix, kind)) in rows.iter().enumerate() {
                for (c, &pc) in order.iter().enumerate() {
                    let rr = Rect::from_min_size(pos2(grid.left() + c as f32 * bw, grid.top() + r as f32 * (bh + 4.0)), vec2(bw - 4.0, bh));
                    let key = Scale::new(pc, *kind);
                    // Brighter the more notes it shares with the song's key.
                    let shared = (key.mask() & song_mask).count_ones() as f32;
                    let glow = ((shared - 3.0) / 4.0).clamp(0.0, 1.0);
                    let (pr, rl) = self.key_button(ui, rr, Id::new(("key_all", r, c)), &format!("{}{}", NAMES[pc as usize], suffix), "", now == key, song == key, glow);
                    if pr {
                        pick = Some(if key == song { None } else { Some(key) });
                    }
                    released |= rl;
                }
            }
        }

        // A press: into the chosen step of the sequence, or the key now.
        if let Some(k) = pick {
            match self.key_seq_sel {
                Some(s) if s < self.project.key_seq.len() => {
                    self.project.key_seq[s] = k;
                    let next = s + 1;
                    self.key_seq_sel = (next < self.project.key_seq.len()).then_some(next);
                    if !running {
                        self.project.play_key = k;
                    }
                }
                _ => {
                    self.project.play_key = k;
                    self.key_held = !self.key_latch;
                }
            }
        }
        // Without Hold, letting go goes back to the song's key.
        if released && self.key_held && !self.key_latch {
            self.project.play_key = None;
            self.key_held = false;
        }
    }
}
