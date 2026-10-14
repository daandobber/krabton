//! The Key drawer: move the whole song by pressing a chord on a map.
//!
//! The map is two rows of hexagons round the circle of fifths: major chords
//! above, minor chords below and half a step to the right, so every minor
//! sits between the two majors it shares the most with. Next to each other
//! means close. The chords of the song's key make one block in the middle
//! (in C: F C G over Dm Am Em), lit and numbered (IV I V, ii vi iii).
//!
//! A press on a chord of the key moves the song onto that chord and keeps
//! the key's notes (the IV of C is F Lydian: C E G becomes F A C): it
//! sounds like the chord changing. A press outside the block moves the song
//! into that key. The song's own chord brings it home.
//!
//! *Sequence*: up to eight steps the song moves through while it plays, a
//! step every 1, 2 or 4 bars, changing on the bar. Click a step, then press
//! chords to fill it; or start from a progression. Drums stay; what you
//! play sounds as you play it.

use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui};
use krabton_engine::{Scale, ScaleKind};
use std::sync::atomic::Ordering;

const NAMES: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];

/// Progressions by degrees of the song's key.
const PROGRESSIONS: &[&[usize]] = &[&[0, 4, 5, 3], &[5, 3, 0, 4], &[0, 5, 3, 4], &[0, 3, 4, 3], &[0, 3, 0, 4], &[1, 4, 0, 0], &[0, 5, 1, 4], &[0, 6, 5, 4], &[0, 2, 3, 3], &[0, 0, 3, 3, 4, 4, 0, 0]];

/// The hexagon's radius (centre to corner).
const R: f32 = 25.0;

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

/// Whether a key is a minor one (by its third).
fn is_minor(k: &Scale) -> bool {
    k.intervals().get(2) == Some(&3)
}

fn hexagon(c: Pos2, r: f32) -> Vec<Pos2> {
    (0..6).map(|i| {
        let a = std::f32::consts::FRAC_PI_3 * i as f32 + std::f32::consts::FRAC_PI_6;
        pos2(c.x + r * a.cos(), c.y + r * a.sin())
    }).collect()
}

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

    /// The key a chord on the map moves the song to: on a chord of the
    /// song's key, that chord in the key's notes; elsewhere, its key. None:
    /// the song's own.
    fn chord_key(&self, root: u8, minor: bool) -> Option<Scale> {
        let song = self.project.scale;
        let degree = (0..song.len().min(7)).find(|&d| {
            let k = song.degree_key(d);
            k.root % 12 == root && is_minor(&k) == minor && k.kind != ScaleKind::Locrian
        });
        match degree {
            Some(0) => None,
            Some(d) => Some(song.degree_key(d)),
            None => Some(Scale::new(root, if minor { ScaleKind::Minor } else { ScaleKind::Major })),
        }
    }

    pub(crate) fn ui_transpose_drawer(&mut self, ui: &mut Ui) {
        let song = self.project.scale;
        let now_key = self.current_play_key();
        let now = now_key.unwrap_or(song);
        let moved = now_key.filter(|k| *k != song).is_some();
        let running = self.project.key_seq_on && self.is_playing();
        let mut pick: Option<Option<Scale>> = None;

        // The head: what plays, and what can be done.
        ui.horizontal(|ui| {
            ui.label(RichText::new("Key").strong().size(16.0).color(TEXT));
            ui.add_space(8.0);
            let waiting = self.status().key_waiting.load(Ordering::Relaxed);
            let txt = if waiting {
                format!("{}  from the next bar", short(&self.project.sounding_key()))
            } else if moved {
                format!("{}  now on  {}", song.name(), short(&now))
            } else {
                format!("{}, at home", song.name())
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
                let on_bar = self.project.play_key_on_bar;
                if ui.selectable_label(on_bar, "On the bar").on_hover_text("On: a press takes effect on the first beat of the next bar while the song plays. Off: at once").clicked() {
                    self.project.play_key_on_bar = !on_bar;
                }
                if self.project.play_key.is_some() {
                    if ui.button("Make it the key").on_hover_text("Write it into the set: every clip's notes move and it becomes the song's key. It sounds the same").clicked() {
                        self.project.bake_play_key();
                        self.toast(format!("The song is in {} now", self.project.scale.name()));
                    }
                    if ui.button("Home").on_hover_text(format!("Back to {}", song.name())).clicked() {
                        self.project.play_key = None;
                    }
                }
                ui.label(RichText::new("Chords next to each other are close. The lit ones are the song's key").small().color(TEXT_DIM));
            });
        });
        ui.add_space(4.0);

        ui.horizontal_top(|ui| {
            // The map: majors above in fifths, minors below between them,
            // turned so the song's key is in the middle.
            let w = 3f32.sqrt() * R;
            let row = 1.5 * R;
            let (rect, _) = ui.allocate_exact_size(vec2(12.5 * w + 4.0, 2.0 * R + row + 6.0), Sense::hover());
            let home_major = if is_minor(&song) { (song.root + 3) % 12 } else { song.root % 12 };
            let p = ui.painter().clone();
            let song_mask = song.mask();
            let seq = self.project.key_seq.clone();
            let pointer = ui.input(|i| i.pointer.hover_pos());
            for row_i in 0..2 {
                let minor = row_i == 1;
                for k in 0..12usize {
                    // Column k: k - 5 fifths from the home major.
                    let maj = (home_major as i32 + 7 * (k as i32 - 5)).rem_euclid(12) as u8;
                    let root = if minor { (maj + 9) % 12 } else { maj };
                    let c = pos2(rect.left() + w * 0.5 + 2.0 + k as f32 * w + if minor { w * 0.5 } else { 0.0 }, rect.top() + R + 2.0 + row_i as f32 * row);
                    let target = self.chord_key(root, minor);
                    let key_of = target.unwrap_or(song);
                    let degree = (0..song.len().min(7)).find(|&d| song.degree_key(d) == key_of);
                    let in_key = degree.is_some();
                    let on = now == key_of;
                    let hovered = pointer.is_some_and(|pt| pt.distance(c) < R * 0.9);
                    let shared = (Scale::new(root, if minor { ScaleKind::Minor } else { ScaleKind::Major }).mask() & song_mask).count_ones() as f32;
                    let glow = ((shared - 3.0) / 4.0).clamp(0.0, 1.0);
                    let fill = if on {
                        ACCENT
                    } else if in_key {
                        mix(PANEL3, Color32::from_gray(110), 0.45)
                    } else {
                        mix(PANEL2, PANEL3, glow)
                    };
                    let fill = if hovered && !on { fill.gamma_multiply(1.35) } else { fill };
                    p.add(Shape::convex_polygon(hexagon(c, R - 1.5), fill, Stroke::new(1.0_f32, if in_key { Color32::from_gray(140) } else { BG })));
                    if target.is_none() {
                        p.add(Shape::closed_line(hexagon(c, R - 3.0), Stroke::new(2.0_f32, if on { TEXT_DARK } else { ACCENT })));
                    }
                    let ink = if on { TEXT_DARK } else if in_key { TEXT } else { TEXT_DIM };
                    let name = format!("{}{}", NAMES[root as usize], if minor { "m" } else { "" });
                    let num = degree.map(|d| song.numeral(d)).unwrap_or_default();
                    if num.is_empty() {
                        text(&p, c, Align2::CENTER_CENTER, &name, FontId::proportional(13.0), ink);
                    } else {
                        text(&p, pos2(c.x, c.y - 5.0), Align2::CENTER_CENTER, &name, FontId::proportional(13.0), ink);
                        text(&p, pos2(c.x, c.y + 9.0), Align2::CENTER_CENTER, &num, FontId::proportional(10.0), if on { TEXT_DARK } else { TEXT_DIM });
                    }
                    // The sequence's steps on this chord.
                    let steps: Vec<String> = seq.iter().enumerate().filter(|(_, s)| s.unwrap_or(song) == key_of).map(|(i, _)| (i + 1).to_string()).collect();
                    if !steps.is_empty() {
                        text(&p, pos2(c.x, c.y - R * 0.62), Align2::CENTER_CENTER, &steps.join(","), FontId::proportional(9.0), if on { TEXT_DARK } else { ACCENT });
                    }
                    let resp = ui.interact(Rect::from_center_size(c, vec2(w * 0.9, R * 1.6)), Id::new(("key_hex", row_i, k)), Sense::click());
                    if resp.clicked() && pointer.is_some_and(|pt| pt.distance(c) < R * 0.95) {
                        pick = Some(target);
                    }
                    let what = if target.is_none() {
                        format!("{}: home", song.name())
                    } else if in_key {
                        format!("{name} ({num}): the song moves onto this chord and keeps the notes of {}", song.name())
                    } else {
                        format!("{name}: the song moves into {}", key_of.name())
                    };
                    resp.on_hover_text(what);
                }
            }
            ui.add_space(16.0);

            // The sequence.
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Sequence").small().color(TEXT_DIM));
                    let run = self.project.key_seq_on;
                    let rb = egui::Button::new(RichText::new("Run").small().color(if run { TEXT_DARK } else { TEXT })).fill(if run { ACCENT } else { PANEL3 });
                    if ui.add(rb).on_hover_text("While the song plays it goes through the steps, a step every so many bars").clicked() {
                        self.project.key_seq_on = !run;
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
                            let chords = prog.iter().map(|&d| short(&song.degree_key(d))).collect::<Vec<_>>().join(" ");
                            if ui.button(format!("{name}    {chords}")).clicked() {
                                self.project.key_seq = prog.iter().map(|&d| if d == 0 { None } else { Some(song.degree_key(d)) }).collect();
                                self.key_seq_sel = None;
                                ui.close();
                            }
                        }
                    });
                });
                if self.project.key_seq.is_empty() {
                    self.project.key_seq = vec![None; 4];
                }
                let len = self.project.key_seq.len();
                let step_now = if running { self.project.key_seq_step(self.status().session_pos.load()) } else { None };
                let (sw, sh) = (52.0, 44.0);
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
                    let k_of = key.unwrap_or(song);
                    let degree = (0..song.len().min(7)).find(|&d| k_of == song.degree_key(d)).map(|d| song.numeral(d)).unwrap_or_default();
                    let ink = if playing { TEXT_DARK } else { TEXT };
                    text(p, pos2(r.center().x, r.top() + 15.0), Align2::CENTER_CENTER, &short(&k_of), FontId::proportional(13.0), ink);
                    text(p, pos2(r.center().x, r.top() + 32.0), Align2::CENTER_CENTER, &degree, FontId::proportional(10.0), if playing { TEXT_DARK } else { TEXT_DIM });
                    if resp.clicked() {
                        self.key_seq_sel = if sel { None } else { Some(k) };
                    }
                    if resp.secondary_clicked() {
                        self.project.key_seq[k] = None;
                    }
                    resp.on_hover_text("Click, then press a chord on the map to put it here; right-click: home");
                }
                let hint = if self.key_seq_sel.is_some() { "Press a chord for the chosen step" } else { "Click a step, then a chord" };
                ui.label(RichText::new(hint).small().color(if self.key_seq_sel.is_some() { ACCENT } else { TEXT_DIM }));
            });
        });

        // A press: into the chosen step of the sequence, or the key now.
        if let Some(k) = pick {
            match self.key_seq_sel {
                Some(s) if s < self.project.key_seq.len() => {
                    self.project.key_seq[s] = k;
                    let next = s + 1;
                    self.key_seq_sel = (next < self.project.key_seq.len()).then_some(next);
                }
                _ => self.project.play_key = k,
            }
        }
    }
}
