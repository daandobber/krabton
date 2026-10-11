//! The Rack view: every track's devices at once, MIDI effects and
//! sequencers, instrument and audio effects, a track per row with its
//! devices side by side, or (with the axes swapped) a track per column with
//! its devices under each other. Returns and the master come last. Click a
//! track's header to select it.

use crate::app::{App, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, RichText, Sense, Ui};

impl App {
    pub(crate) fn ui_device_rack(&mut self, ui: &mut Ui) {
        let columns = self.prefs.layout.rack_columns;
        let h = crate::prefs::Layout::or(self.prefs.layout.rack_h, 230.0);
        // The bar on top: the axes and the height of a row of devices.
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            ui.label(RichText::new("Rack").strong().color(TEXT));
            ui.add_space(8.0);
            let label = if columns { "Tracks in columns" } else { "Tracks in rows" };
            if ui.add(egui::Button::new(RichText::new(label).color(TEXT)).fill(PANEL3)).on_hover_text("Swap the axes: a track per row with its devices side by side, or a track per column with its devices under each other").clicked() {
                self.prefs.layout.rack_columns = !columns;
                self.layout_dirty = true;
            }
            ui.add_space(8.0);
            let shown = self.prefs.layout.rack_detail;
            let label = if shown { "Hide clip view" } else { "Show clip view" };
            if ui.add(egui::Button::new(RichText::new(label).color(TEXT)).fill(PANEL3)).on_hover_text("The clip editor, devices, modulation and grid at the bottom (Ctrl+Alt+L)").clicked() {
                self.toggle_detail();
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Device height").small().color(TEXT_DIM)).on_hover_text("For devices you did not resize: drag the grip at the bottom of a device for its own height");
            let mut hh = h;
            if ui.add(egui::DragValue::new(&mut hh).range(140.0..=600.0).speed(1.0).suffix(" px")).changed() {
                self.prefs.layout.rack_h = hh;
                self.layout_dirty = true;
            }
        });
        let mut sels: Vec<TrackSel> = (0..self.project.tracks.len()).map(TrackSel::Track).collect();
        sels.extend((0..self.project.returns.len()).map(TrackSel::Return));
        sels.push(TrackSel::Master);
        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            if columns {
                ui.horizontal_top(|ui| {
                    for sel in sels {
                        ui.vertical(|ui| {
                            self.rack_head(ui, sel, false, h);
                            self.device_chain(ui, sel, h, true);
                        });
                        ui.add_space(10.0);
                    }
                });
            } else {
                ui.vertical(|ui| {
                    for sel in sels {
                        ui.horizontal_top(|ui| {
                            let row = self.rack_row_height(sel, h);
                            self.rack_head(ui, sel, true, row);
                            self.device_chain(ui, sel, h, true);
                        });
                        ui.add_space(6.0);
                    }
                });
            }
        });
    }

    /// The height of a track's row: its tallest device.
    fn rack_row_height(&self, sel: TrackSel, h: f32) -> f32 {
        let mut keys: Vec<u64> = Vec::new();
        match sel {
            TrackSel::Track(i) => {
                let t = &self.project.tracks[i];
                keys.extend(t.midi_devices.iter().map(|d| d.id));
                if t.instrument.is_some() {
                    keys.push(t.id);
                }
                keys.extend(t.devices.iter().map(|d| d.id));
            }
            TrackSel::Return(j) => keys.extend(self.project.returns[j].devices.iter().map(|d| d.id)),
            TrackSel::Master => keys.extend(self.project.master.devices.iter().map(|d| d.id)),
        }
        keys.iter().map(|k| self.rack_height(*k, h)).fold(h, f32::max)
    }

    /// A track's name tag: at the left of its row, or on top of its column.
    fn rack_head(&mut self, ui: &mut Ui, sel: TrackSel, beside: bool, h: f32) {
        let (name, color) = match sel {
            TrackSel::Track(i) => (self.project.tracks[i].name.clone(), rgb(self.project.tracks[i].color)),
            TrackSel::Return(j) => (self.project.returns[j].name.clone(), rgb(self.project.returns[j].color)),
            TrackSel::Master => ("Master".to_string(), ACCENT),
        };
        let size = if beside { vec2(96.0, h + 30.0) } else { vec2(180.0, 26.0) };
        let (r, resp) = ui.allocate_exact_size(size, Sense::click());
        let selected = self.sel_track == sel;
        let p = ui.painter();
        rect_fill(p, r, 4.0, if selected { shade(color, 0.45) } else if resp.hovered() { PANEL3 } else { PANEL2 });
        if beside {
            rect_fill(p, egui::Rect::from_min_size(r.min, vec2(4.0, r.height())), 2.0, color);
            p.text(pos2(r.left() + 10.0, r.top() + 8.0), Align2::LEFT_TOP, &name, FontId::proportional(14.0), if selected { Color32::WHITE } else { TEXT });
        } else {
            rect_fill(p, egui::Rect::from_min_size(r.min, vec2(r.width(), 3.0)), 1.0, color);
            p.text(pos2(r.left() + 8.0, r.center().y + 1.0), Align2::LEFT_CENTER, &name, FontId::proportional(14.0), if selected { Color32::WHITE } else { TEXT });
        }
        if resp.on_hover_text("Select this track").clicked() {
            self.sel_track = sel;
        }
    }
}
