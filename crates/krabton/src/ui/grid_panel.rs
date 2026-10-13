//! The Grid tab: an on-screen monome grid that runs the same logic as a real
//! one, plus mode switches, connected devices and the arc mapping.

use crate::app::{App, TrackSel};
use crate::grid::GridMode;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, RichText, Sense, Stroke, Ui, UiBuilder};

const SIDEBAR_W: f32 = 236.0;
/// Warm white, like varibright grid LEDs.
const LED: Color32 = Color32::from_rgb(0xff, 0xd8, 0x9c);

impl App {
    pub(crate) fn ui_grid_panel(&mut self, ui: &mut Ui, body: Rect) {
        let sidebar = Rect::from_min_size(body.min, vec2(SIDEBAR_W, body.height()));
        let main = Rect::from_min_max(pos2(sidebar.right() + 1.0, body.top()), body.max);
        rect_fill(ui.painter(), sidebar, 0.0, PANEL);
        ui.painter().line_segment([pos2(sidebar.right(), body.top()), pos2(sidebar.right(), body.bottom())], Stroke::new(1.0_f32, LINE));

        let mut sui = ui.new_child(UiBuilder::new().max_rect(sidebar.shrink(6.0)).layout(egui::Layout::top_down(egui::Align::Min)));
        sui.set_clip_rect(sidebar);
        egui::ScrollArea::vertical().id_salt("grid_sidebar").auto_shrink([false, false]).show(&mut sui, |ui| self.grid_sidebar(ui));

        self.virtual_grid(ui, main);
    }

    fn grid_sidebar(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.horizontal(|ui| {
            for m in GridMode::ALL {
                if ui.selectable_label(self.grid.mode == m, m.name()).clicked() {
                    self.grid.mode = m;
                }
            }
        });
        let help = match self.grid.mode {
            GridMode::Session => "Columns are tracks, rows are scenes. Press to launch; the bottom row stops a track. The right column launches scenes (bottom: stop all).",
            GridMode::Keys => "An in-key keyboard: every pad is in the project scale, rows are a fourth apart. Bright pads are roots. Plays the armed or selected track.",
            GridMode::Sequencer => "Edits the first sequencer on the selected track.",
        };
        ui.label(RichText::new(help).small().color(TEXT_DIM));

        match self.grid.mode {
            GridMode::Session => {
                let nt = self.project.tracks.len();
                let ns = self.project.scenes.len();
                let cols = (if self.grid.width >= 16 { self.grid.width - 1 } else { self.grid.width }) as usize;
                let rows = (self.grid.height - 1) as usize;
                ui.horizontal(|ui| {
                    if ui.small_button("<").clicked() {
                        self.grid.track_offset = self.grid.track_offset.saturating_sub(1);
                    }
                    if ui.small_button(">").clicked() && self.grid.track_offset + 1 < nt {
                        self.grid.track_offset += 1;
                    }
                    let last = (self.grid.track_offset + cols).min(nt);
                    ui.label(RichText::new(format!("Tracks {}-{last}", self.grid.track_offset + 1)).small());
                });
                ui.horizontal(|ui| {
                    if ui.small_button("^").clicked() {
                        self.grid.scene_offset = self.grid.scene_offset.saturating_sub(1);
                    }
                    if ui.small_button("v").clicked() && self.grid.scene_offset + 1 < ns {
                        self.grid.scene_offset += 1;
                    }
                    let last = (self.grid.scene_offset + rows).min(ns);
                    ui.label(RichText::new(format!("Scenes {}-{last}", self.grid.scene_offset + 1)).small());
                });
            }
            GridMode::Keys => {
                let s = self.project.scale;
                ui.label(RichText::new(format!("{} · octave {} (Z/X)", s.name(), self.kb_octave)).small());
            }
            GridMode::Sequencer => match self.grid_sequencer() {
                Some((t, _, kind)) => {
                    let name = &self.project.tracks[t].name;
                    ui.label(RichText::new(format!("{} on {name}", kind.name())).small().color(PLAY));
                    let how = match kind {
                        krabton_engine::midi_fx::MidiFxKind::Life => "Press pads to draw the seed. Bright = alive now, dim = seed.",
                        krabton_engine::midi_fx::MidiFxKind::Polymeter => "Bottom row: pick a lane (trig, note, octave, length, prob). Above: set steps.",
                        krabton_engine::midi_fx::MidiFxKind::Euclid => "Rows 1-4 show the rings. Rows 5-8: tap to set the pulses of each ring.",
                        krabton_engine::midi_fx::MidiFxKind::Turing => "Top row: the register. Row 7: loop length. Row 8: how much it changes.",
                        krabton_engine::midi_fx::MidiFxKind::Bounce => "Two columns per ball: the left sets how high it drops from, the right its note. Bottom row: number of balls.",
                        krabton_engine::midi_fx::MidiFxKind::Orbits => "A row per planet: tap a column to set how often it comes round.",
                        krabton_engine::midi_fx::MidiFxKind::Chaos => "The road to chaos, left to right: tap a column to go there.",
                        krabton_engine::midi_fx::MidiFxKind::Automaton => "The generations scroll up. Bottom row, left half: the rule's eight bits.",
                        krabton_engine::midi_fx::MidiFxKind::Ant => "The ant's board: watch it build its highway.",
                        krabton_engine::midi_fx::MidiFxKind::Pong => "The box, with the balls bouncing in it.",
                        krabton_engine::midi_fx::MidiFxKind::Markov => "The melody's place in its range. Bottom row: repeat, step, leap and rest chances.",
                        krabton_engine::midi_fx::MidiFxKind::Rain => "The last eight drops, falling down the rows.",
                        krabton_engine::midi_fx::MidiFxKind::Words => "The letter that plays. Bottom row: how far through the text.",
                        krabton_engine::midi_fx::MidiFxKind::Lorenz => "The butterfly's wings, seen side on.",
                        krabton_engine::midi_fx::MidiFxKind::Loops => "A row per loop, its tape running left to right: bright as it plays.",
                        krabton_engine::midi_fx::MidiFxKind::Bloom => "Press a pad to drop a note: higher rows are higher notes. In Loop your notes stay (dim): press one to take it away.",
                        krabton_engine::midi_fx::MidiFxKind::Chimes => "The tubes, lighting up as they strike. Bottom row: the wind.",
                        krabton_engine::midi_fx::MidiFxKind::DriftChords => "A row per voice, from the bottom: where it sits in the range.",
                        krabton_engine::midi_fx::MidiFxKind::Tintinnabuli => "The melody (bright) and its triad voice (dim). Bottom row: the held tonic.",
                        krabton_engine::midi_fx::MidiFxKind::GuitarChords => "Rows 1-6: the chord on the neck (high string on top). Row 7: the chords. Row 8: the strokes (press to cycle rest, down, up).",
                        krabton_engine::midi_fx::MidiFxKind::Pedal => "The held note: the tonic in the middle. Bottom row: the bars until it may move.",
                        krabton_engine::midi_fx::MidiFxKind::Step => "Columns are steps, rows are scale degrees (top = high). Bottom row: step on/off.",
                        krabton_engine::midi_fx::MidiFxKind::Acid => "Rows 1-6: notes (octave, b7, 5, 4, b3, root). Row 7: accent. Row 8: slide.",
                        krabton_engine::midi_fx::MidiFxKind::DrumGrid => "Rows are drum lanes: tap to cycle off, hit, accent.",
                        krabton_engine::midi_fx::MidiFxKind::Progression => "Columns are chords; rows pick the degree (top = VII). Bottom row: chord type.",
                        krabton_engine::midi_fx::MidiFxKind::DrumMap => "Rows 1-3: kick, snare and hats of the bar that plays (dim: steps the fill leaves out). Lower left: the map of grooves, tap to move. Right: the three fills and chaos.",
                        krabton_engine::midi_fx::MidiFxKind::Marbles => "Rows 1-3: T1, T2 and T3, the last sixteen ticks. Row 4: the X values. Rows 5-7: déjà vu, loop length, bias. Bottom row: the model (left), jitter (right).",
                        krabton_engine::midi_fx::MidiFxKind::Drunk => "The walk wanders left and right. Bottom row: how often it rests.",
                        krabton_engine::midi_fx::MidiFxKind::Chance => "Rows are the lanes: tap to cycle off, hit, accent. Dim steps have a chance or a condition.",
                        krabton_engine::midi_fx::MidiFxKind::Bits => "The formula's bits (bit 7 on top), bright where a drum plays. Four columns per drum: tap a row to give it that bit.",
                        krabton_engine::midi_fx::MidiFxKind::Percussion => "A row per player and the lead at the bottom. Tap a row to bring the ensemble up to it; the lead's row turns it on or off.",
                        _ => "",
                    };
                    ui.label(RichText::new(how).small().color(TEXT_DIM));
                }
                None => {
                    ui.label(RichText::new("No sequencer on the selected track. Drop one from the browser (Sequencers).").small().color(TEXT_DIM));
                }
            },
        }

        ui.separator();
        ui.label(RichText::new("monome").strong());
        let (grid_id, arc_id, devices, err, port) = match &self.grid.monome {
            Some(m) => (m.grid_id().map(str::to_string), m.arc_id().map(str::to_string), m.devices.clone(), None, m.grid_port().map(str::to_string)),
            None => (None, None, Vec::new(), self.grid.error.clone(), None),
        };
        if let Some(e) = err {
            ui.label(RichText::new(e).small().color(REC));
        }
        match &grid_id {
            Some(id) => {
                let via = port.as_ref().map(|p| format!(" on {p}")).unwrap_or_else(|| " via serialosc".into());
                ui.label(RichText::new(format!("grid {id}{via} ({}x{})", self.grid.width, self.grid.height)).small().color(PLAY))
            }
            None => ui.label(RichText::new("No grid: the one on the right works with the mouse.").small().color(TEXT_DIM)),
        };
        if let Some(id) = &arc_id {
            ui.label(RichText::new(format!("arc {id}")).small().color(PLAY));
        }
        for d in &devices {
            if Some(&d.id) != grid_id.as_ref() && Some(&d.id) != arc_id.as_ref() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{} {}", d.kind, d.id)).small());
                    if ui.small_button("Use").clicked() {
                        if let Some(m) = self.grid.monome.as_mut() {
                            m.connect(&d.id);
                        }
                    }
                });
            }
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new("Rotation").small().color(TEXT_DIM));
            // A 16x8 grid only turns half way; square grids every quarter.
            let square = self.grid.monome.as_ref().map(|m| m.grid_size.0 == m.grid_size.1).unwrap_or(false);
            let cur = self.prefs.grid_rotation;
            for deg in [0, 90, 180, 270] {
                if deg % 180 == 90 && !square && cur != deg {
                    continue;
                }
                if ui.selectable_label(cur == deg, format!("{deg}°")).on_hover_text("Turn the grid, e.g. when its cable points the other way").clicked() {
                    self.set_grid_rotation(deg);
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Rescan").on_hover_text("Look for grids and arcs again (serialosc and USB serial ports)").clicked() {
                if let Some(m) = self.grid.monome.as_mut() {
                    m.rescan();
                }
                self.toast("Looking for monome devices...");
            }
            if grid_id.is_none() {
                let sizes = [(16, 8), (8, 8), (16, 16)];
                let cur = (self.grid.width, self.grid.height);
                let i = sizes.iter().position(|s| *s == cur).unwrap_or(0);
                if ui.button(format!("{}x{}", cur.0, cur.1)).on_hover_text("Size of the virtual grid").clicked() {
                    let (w, h) = sizes[(i + 1) % sizes.len()];
                    self.grid.width = w;
                    self.grid.height = h;
                }
            }
        });

        ui.separator();
        let targets = self.arc_targets();
        ui.label(RichText::new("Arc rings").strong());
        if targets.is_empty() {
            ui.label(RichText::new("Select a track to map the arc.").small().color(TEXT_DIM));
        }
        for (i, (name, v)) in targets.iter().enumerate() {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(60.0, 10.0), Sense::hover());
                rect_fill(ui.painter(), r, 2.0, KNOB);
                rect_fill(ui.painter(), Rect::from_min_size(r.min, vec2(r.width() * v.clamp(0.0, 1.0), r.height())), 2.0, shade(ACCENT, 0.8));
                ui.label(RichText::new(format!("{} {name}", i + 1)).small());
            });
        }
    }

    fn virtual_grid(&mut self, ui: &mut Ui, area: Rect) {
        let (w, h) = (self.grid.width, self.grid.height);
        let pad = 14.0;
        let cell = ((area.width() - 2.0 * pad) / w as f32).min((area.height() - 2.0 * pad) / h as f32).clamp(8.0, 36.0);
        let size = vec2(cell * w as f32, cell * h as f32);
        let grid = Rect::from_center_size(area.center(), size);
        let p = ui.painter().clone();
        rect_fill(&p, grid.expand(8.0), 6.0, Color32::from_rgb(0x1c, 0x1c, 0x1e));
        rect_line(&p, grid.expand(8.0), 6.0, 1.0, LINE_SOFT);
        let mode_color = match self.grid.mode {
            GridMode::Session => ACCENT,
            GridMode::Keys => SELECT,
            GridMode::Sequencer => PLAY,
        };
        for y in 0..h {
            for x in 0..w {
                let level = self.grid.leds.get((y * w + x) as usize).copied().unwrap_or(0);
                let r = Rect::from_min_size(pos2(grid.left() + x as f32 * cell, grid.top() + y as f32 * cell), vec2(cell, cell)).shrink(cell * 0.09);
                let fill = if level == 0 { PANEL2 } else { mix(shade(LED, 0.18), LED, level as f32 / 15.0) };
                rect_fill(&p, r, cell * 0.14, fill);
                if self.grid.virtual_down == Some((x, y)) {
                    rect_line(&p, r, cell * 0.14, 1.5, mode_color);
                }
            }
        }
        // Quad seam, like the 8x8 sections of real grids.
        if w > 8 {
            let x = grid.left() + 8.0 * cell;
            p.line_segment([pos2(x, grid.top() + 2.0), pos2(x, grid.bottom() - 2.0)], Stroke::new(1.0_f32, shade(LINE_SOFT, 0.8)));
        }
        text(&p, pos2(grid.left(), grid.top() - 12.0), Align2::LEFT_CENTER, format!("{} mode", self.grid.mode.name()), small(), mode_color);
        if let TrackSel::Track(t) = self.sel_track {
            if let Some(tr) = self.project.tracks.get(t) {
                text(&p, pos2(grid.right(), grid.top() - 12.0), Align2::RIGHT_CENTER, &tr.name, small(), TEXT_DIM);
            }
        }

        let resp = ui.interact(grid, Id::new("virtual_grid"), Sense::click_and_drag());
        let cell_at = |pt: egui::Pos2| -> Option<(u32, u32)> {
            let x = ((pt.x - grid.left()) / cell).floor();
            let y = ((pt.y - grid.top()) / cell).floor();
            (x >= 0.0 && y >= 0.0 && (x as u32) < w && (y as u32) < h).then_some((x as u32, y as u32))
        };
        let cur = if resp.is_pointer_button_down_on() { resp.interact_pointer_pos().and_then(cell_at) } else { None };
        if cur != self.grid.virtual_down {
            if let Some((x, y)) = self.grid.virtual_down {
                self.grid_key(x, y, false);
            }
            if let Some((x, y)) = cur {
                self.grid_key(x, y, true);
            }
            self.grid.virtual_down = cur;
        }
    }
}
