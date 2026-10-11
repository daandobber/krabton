//! The detail panel at the bottom: clip editor or device chain.

use crate::app::{App, ClipSel, DetailTab, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, RichText, Sense, Stroke, Ui, UiBuilder};
use krabton_engine::{ClipContent, Note};

const SIDEBAR_W: f32 = 212.0;
const TABS_H: f32 = 24.0;
const TOOLS_H: f32 = 28.0;

impl App {
    pub(crate) fn ui_detail(&mut self, ui: &mut Ui) {
        let full = ui.available_rect_before_wrap();
        ui.allocate_rect(full, Sense::hover());
        let tabs = Rect::from_min_size(full.min, vec2(full.width(), TABS_H));
        let content = Rect::from_min_max(pos2(full.left(), tabs.bottom()), full.max);
        rect_fill(ui.painter(), tabs, 0.0, PANEL2);
        ui.painter().line_segment([pos2(full.left(), full.top()), pos2(full.right(), full.top())], Stroke::new(1.0_f32, LINE));

        let mut x = tabs.left() + 6.0;
        for (tab, label) in [(DetailTab::Clip, "Clip"), (DetailTab::Devices, "Devices"), (DetailTab::Mod, "Mod"), (DetailTab::Grid, "Grid")] {
            let r = Rect::from_min_size(pos2(x, tabs.top() + 3.0), vec2(70.0, TABS_H - 6.0));
            if button(ui, r, Id::new(("detail_tab", label)), label, self.detail == tab, ACCENT).on_hover_text("Shift+Tab toggles").clicked() {
                self.detail = tab;
            }
            x += 74.0;
        }
        let title = match self.detail {
            DetailTab::Clip => self.sel_clip.and_then(|s| self.clip_ref(s)).map(|c| c.name.clone()).unwrap_or_default(),
            DetailTab::Devices => match self.sel_track {
                TrackSel::Track(i) => self.project.tracks.get(i).map(|t| t.name.clone()).unwrap_or_default(),
                TrackSel::Return(j) => self.project.returns.get(j).map(|t| t.name.clone()).unwrap_or_default(),
                TrackSel::Master => "Master".into(),
            },
            DetailTab::Mod => format!("Modulation matrix · {} modulators · {} connections", self.project.modulators.len(), self.project.mod_routes.len()),
            DetailTab::Grid => match self.grid.monome.as_ref().and_then(|m| m.grid_id()) {
                Some(id) => format!("monome grid {id}"),
                None => "Virtual grid (plug in a monome grid)".into(),
            },
        };
        if self.mod_learn.is_some() {
            self.mod_banner(ui, tabs);
        } else {
            text(ui.painter(), pos2(x + 8.0, tabs.center().y), Align2::LEFT_CENTER, title, body(), TEXT_DIM);
        }
        // Fold the panel away (the strip at the bottom brings it back).
        let fold = Rect::from_center_size(pos2(tabs.right() - 16.0, tabs.center().y), vec2(22.0, TABS_H - 6.0));
        let fresp = ui.interact(fold, Id::new("detail_fold"), Sense::click()).on_hover_text("Hide the clip and device view (Ctrl+Alt+L)");
        let c = fold.center();
        let col = if fresp.hovered() { TEXT } else { TEXT_DIM };
        ui.painter().add(egui::Shape::convex_polygon(vec![c + vec2(-5.0, -2.0), c + vec2(5.0, -2.0), c + vec2(0.0, 3.0)], col, Stroke::NONE));
        if fresp.clicked() {
            self.toggle_detail();
        }

        match self.detail {
            DetailTab::Clip => self.ui_clip_detail(ui, content),
            // In the Rack every track's devices are already above.
            DetailTab::Devices if self.view == crate::app::View::Rack => {
                ui.painter().text(content.center(), Align2::CENTER_CENTER, "The devices are in the Rack above", body(), TEXT_DIM);
            }
            DetailTab::Devices => self.ui_devices(ui, content),
            DetailTab::Grid => self.ui_grid_panel(ui, content),
            DetailTab::Mod => self.ui_mod_panel(ui, content),
        }
    }

    fn ui_clip_detail(&mut self, ui: &mut Ui, body: Rect) {
        let Some(sel) = self.sel_clip else {
            ui.painter().text(
                body.center(),
                Align2::CENTER_CENTER,
                "Select a clip. Double-click an empty slot on a MIDI track to create one.",
                body_font(),
                TEXT_DIM,
            );
            return;
        };
        let Some(clip) = self.clip_ref(sel).cloned() else { return };
        let side_w = crate::prefs::Layout::or(self.prefs.layout.clip_side_w, SIDEBAR_W).clamp(150.0, 480.0);
        let sidebar = Rect::from_min_size(body.min, vec2(side_w, body.height()));
        let main = Rect::from_min_max(pos2(sidebar.right() + 1.0, body.top()), body.max);
        rect_fill(ui.painter(), sidebar, 0.0, PANEL);
        ui.painter().line_segment([pos2(sidebar.right(), body.top()), pos2(sidebar.right(), body.bottom())], Stroke::new(1.0_f32, LINE));

        let mut sui = ui.new_child(UiBuilder::new().max_rect(sidebar.shrink(6.0)).layout(egui::Layout::top_down(egui::Align::Min)));
        sui.set_clip_rect(sidebar);
        self.clip_sidebar(&mut sui, sel, &clip);
        let split = Rect::from_min_max(pos2(sidebar.right() - 4.0, sidebar.top()), pos2(sidebar.right() + 2.0, sidebar.bottom()));
        let resp = crate::widgets::splitter(ui, Id::new("clip_side_split"), split, true);
        if resp.dragged() {
            self.prefs.layout.clip_side_w = (side_w + resp.drag_delta().x).clamp(150.0, 480.0);
            self.layout_dirty = true;
        }

        match &clip.content {
            ClipContent::Midi { .. } => {
                let bar = Rect::from_min_size(main.min, vec2(main.width(), TOOLS_H));
                let roll = Rect::from_min_max(pos2(main.left(), bar.bottom()), main.max);
                rect_fill(ui.painter(), bar, 0.0, PANEL);
                let mut tui = ui.new_child(UiBuilder::new().max_rect(bar.shrink2(vec2(6.0, 2.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
                tui.set_clip_rect(bar);
                self.midi_tools_bar(&mut tui);
                self.ui_piano_roll(ui, roll, sel);
            }
            ClipContent::Audio(_) => self.ui_audio_clip(ui, main, sel),
        }
    }

    /// Scale chip and scale-aware MIDI tools above the piano roll.
    fn midi_tools_bar(&mut self, ui: &mut Ui) {
        use crate::ui::piano_roll::NoteTool;
        use krabton_engine::midi_tools::ArpStyle;
        ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
        let on = self.project.scale_mode;
        if ui.add(egui::Button::new(RichText::new("Scale").color(if on { TEXT_DARK } else { TEXT })).fill(if on { ACCENT } else { PANEL3 })).on_hover_text("Scale mode: highlight the project scale").clicked() {
            self.project.scale_mode = !on;
        }
        if ui.button(self.project.scale.name()).on_hover_text("Change the key and scale of the project").clicked() {
            self.open_scale_dialog();
        }
        let fold = self.piano.fold;
        if ui.add_enabled(on, egui::Button::new(RichText::new("Fold").color(if fold && on { TEXT_DARK } else { TEXT })).fill(if fold && on { ACCENT } else { PANEL3 })).on_hover_text("Only show the notes of the scale").clicked() {
            self.piano.fold = !fold;
        }
        let gh = self.piano.ghosts;
        if ui.add(egui::Button::new(RichText::new("Ghosts").color(if gh { TEXT_DARK } else { TEXT })).fill(if gh { ACCENT } else { PANEL3 })).on_hover_text("Show the notes of the other tracks that play alongside, faintly").clicked() {
            self.piano.ghosts = !gh;
        }
        ui.separator();
        let sel_txt = if self.piano.selected.is_empty() { "all notes" } else { "selected notes" };
        let tip = |what: &str| format!("{what} ({sel_txt})");
        if ui.button("Snap").on_hover_text(tip("Move notes outside the scale to the nearest scale note")).clicked() {
            self.piano_tool(NoteTool::SnapToScale);
        }
        ui.label(RichText::new("Step").small().color(TEXT_DIM));
        if ui.small_button("-").on_hover_text(tip("One scale step down")).clicked() {
            self.piano_tool(NoteTool::Degrees(-1));
        }
        if ui.small_button("+").on_hover_text(tip("One scale step up")).clicked() {
            self.piano_tool(NoteTool::Degrees(1));
        }
        ui.separator();
        if ui.button("Chords").on_hover_text(tip("Stack diatonic triads on top of each note")).clicked() {
            self.piano_tool(NoteTool::Chords(3));
        }
        if ui.button("7ths").on_hover_text(tip("Stack diatonic seventh chords")).clicked() {
            self.piano_tool(NoteTool::Chords(4));
        }
        ui.separator();
        if ui.button("Arp").on_hover_text(tip("Turn chords into an arpeggio")).clicked() {
            self.piano_tool(NoteTool::Arpeggiate);
        }
        let rates = [(0.5, "1/8"), (0.25, "1/16"), (0.125, "1/32"), (1.0 / 6.0, "1/16T")];
        let cur = rates.iter().find(|r| (r.0 - self.piano.arp_rate).abs() < 1e-9).map(|r| r.1).unwrap_or("1/16");
        egui::ComboBox::from_id_salt("arp_rate").width(48.0).selected_text(cur).show_ui(ui, |ui| {
            for (r, name) in rates {
                ui.selectable_value(&mut self.piano.arp_rate, r, name);
            }
        });
        egui::ComboBox::from_id_salt("arp_style").width(64.0).selected_text(self.piano.arp_style.name()).show_ui(ui, |ui| {
            for st in ArpStyle::ALL {
                ui.selectable_value(&mut self.piano.arp_style, st, st.name());
            }
        });
        if ui.button("Strum").on_hover_text(tip("Offset chord notes like a strummed guitar")).clicked() {
            self.piano_tool(NoteTool::Strum);
        }
        if ui.button("Legato").on_hover_text(tip("Extend notes until the next one starts")).clicked() {
            self.piano_tool(NoteTool::Legato);
        }
        if ui.button("Humanize").on_hover_text(tip("Small random timing and velocity changes")).clicked() {
            self.piano_tool(NoteTool::Humanize);
        }
        ui.separator();
        if ui.add(egui::Button::new(RichText::new("Seed").color(TEXT_DARK)).fill(shade(ACCENT, 0.85))).on_hover_text("Generate a random melody in the scale (click again for another one)").clicked() {
            self.piano_tool(NoteTool::Seed);
        }
    }

    /// Bitwig-style note operators of the selected notes: what changes only
    /// touches that setting of every selected note.
    fn note_operators(&mut self, ui: &mut Ui, sel: ClipSel, picked: &[usize], first: &krabton_engine::Note) {
        ui.separator();
        ui.label(RichText::new("Note operators").strong()).on_hover_text("How the selected notes play each time round: a chance, velocity that wanders, quick repeats, and on which loops");
        let (mut chance, mut spread, mut reps, mut rlen, mut mask) = (first.chance, first.vel_spread, first.repeats.max(1), first.recur_len, first.recur_mask);
        #[derive(PartialEq)]
        enum Set {
            None,
            Chance,
            Spread,
            Repeats,
            Recur,
        }
        let mut set = Set::None;
        let pct = |v: f64, _: std::ops::RangeInclusive<usize>| format!("{:.0}%", v * 100.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Chance").small().color(TEXT_DIM));
            if ui.add(egui::Slider::new(&mut chance, 0.0..=1.0).custom_formatter(pct)).on_hover_text("How often the note plays").changed() {
                set = Set::Chance;
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Vel spread").small().color(TEXT_DIM));
            if ui.add(egui::Slider::new(&mut spread, 0.0..=1.0).custom_formatter(pct)).on_hover_text("How far the velocity wanders each time").changed() {
                set = Set::Spread;
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Repeats").small().color(TEXT_DIM));
            if ui.add(egui::DragValue::new(&mut reps).range(1..=16)).on_hover_text("Play the note as this many quick hits (ratchets)").changed() {
                set = Set::Repeats;
            }
            ui.label(RichText::new("Recur").small().color(TEXT_DIM));
            let label = |n: u8| if n < 2 { "Off".to_string() } else { format!("{n} loops") };
            egui::ComboBox::from_id_salt("note_recur").width(64.0).selected_text(label(rlen)).show_ui(ui, |ui| {
                for n in [0u8, 2, 3, 4, 5, 6, 7, 8] {
                    if ui.selectable_label(rlen == n, label(n)).clicked() {
                        rlen = n;
                        if n >= 2 && mask.count_ones() >= 8 {
                            mask = 1;
                        }
                        set = Set::Recur;
                    }
                }
            })
            .response
            .on_hover_text("Recurrence: over a cycle of loops, play only on the chosen ones");
        });
        if rlen >= 2 {
            ui.horizontal(|ui| {
                ui.label(RichText::new("on loop").small().color(TEXT_DIM));
                for i in 0..rlen {
                    let on = (mask >> i) & 1 == 1;
                    if ui.selectable_label(on, format!("{}", i + 1)).clicked() {
                        mask ^= 1 << i;
                        set = Set::Recur;
                    }
                }
            });
        }
        if set == Set::None {
            return;
        }
        if let Some(v) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
            for &i in picked {
                let Some(n) = v.get_mut(i) else { continue };
                match set {
                    Set::Chance => n.chance = chance,
                    Set::Spread => n.vel_spread = spread,
                    Set::Repeats => n.repeats = reps,
                    Set::Recur => {
                        n.recur_len = rlen;
                        n.recur_mask = mask;
                    }
                    Set::None => {}
                }
            }
        }
    }

    fn clip_sidebar(&mut self, ui: &mut Ui, sel: ClipSel, clip: &krabton_engine::Clip) {
        let bpb = self.project.beats_per_bar_f();
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        let mut name = clip.name.clone();
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(12.0, 18.0), Sense::hover());
            ui.painter().rect_filled(r, 2.0, rgb(clip.color));
            if ui.add(egui::TextEdit::singleline(&mut name).desired_width(170.0)).changed() {
                if let Some(c) = self.clip_mut(sel) {
                    c.name = name.clone();
                }
            }
        });

        if let ClipSel::Arr { track, id } = sel {
            if let Some(ac) = self.project.tracks.get_mut(track).and_then(|t| t.arrangement.iter_mut().find(|a| a.id == id)) {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Position").small().color(TEXT_DIM));
                    let mut bar = ac.start / bpb + 1.0;
                    if ui.add(egui::DragValue::new(&mut bar).range(1.0..=9999.0).speed(0.05).fixed_decimals(2)).changed() {
                        ac.start = (bar - 1.0).max(0.0) * bpb;
                    }
                    let mut len = ac.length / bpb;
                    ui.label(RichText::new("Len").small().color(TEXT_DIM));
                    if ui.add(egui::DragValue::new(&mut len).range(0.0625..=999.0).speed(0.05).fixed_decimals(2)).changed() {
                        ac.length = len.max(0.0625) * bpb;
                    }
                });
            }
        }

        let mut length_bars = clip.length / bpb;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Loop length").small().color(TEXT_DIM));
            if ui.add(egui::DragValue::new(&mut length_bars).range(0.0625..=256.0).speed(0.02).fixed_decimals(2).suffix(" bars")).changed() {
                if let Some(c) = self.clip_mut(sel) {
                    c.length = (length_bars * bpb).max(0.0625);
                }
            }
        });

        match &clip.content {
            ClipContent::Midi { notes } => {
                ui.horizontal(|ui| {
                    if ui.button("x2").on_hover_text("Duplicate loop: doubles the length and copies the notes").clicked() {
                        let len = clip.length;
                        if let Some(c) = self.clip_mut(sel) {
                            if let Some(v) = c.notes_mut() {
                                let copies: Vec<Note> = v.iter().filter(|n| n.start < len).map(|n| Note { start: n.start + len, ..n.clone() }).collect();
                                v.extend(copies);
                            }
                            c.length = len * 2.0;
                        }
                    }
                    if ui.button("/2").on_hover_text("Halve the loop length").clicked() {
                        if let Some(c) = self.clip_mut(sel) {
                            c.length = (c.length / 2.0).max(0.0625);
                        }
                    }
                    if ui.button("Quantize").on_hover_text("Ctrl+U").clicked() {
                        self.piano_quantize();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Grid").small().color(TEXT_DIM));
                    let grids = [(1.0, "1/4"), (0.5, "1/8"), (0.25, "1/16"), (0.125, "1/32"), (1.0 / 3.0, "1/8T"), (1.0 / 6.0, "1/16T")];
                    let cur = grids.iter().find(|g| (g.0 - self.piano.grid).abs() < 1e-9).map(|g| g.1).unwrap_or("1/16");
                    egui::ComboBox::from_id_salt("piano_grid").width(56.0).selected_text(cur).show_ui(ui, |ui| {
                        for (g, name) in grids {
                            ui.selectable_value(&mut self.piano.grid, g, name);
                        }
                    });
                    let dm = self.piano.draw_mode;
                    if ui.add(egui::Button::new(RichText::new("Draw").color(if dm { TEXT_DARK } else { TEXT })).fill(if dm { ACCENT } else { PANEL3 })).on_hover_text("Draw mode (B): click to add/remove notes").clicked() {
                        self.piano.draw_mode = !dm;
                    }
                    let pv = self.prefs.note_preview;
                    if ui.add(egui::Button::new(RichText::new("Preview").color(if pv { TEXT_DARK } else { TEXT })).fill(if pv { ACCENT } else { PANEL3 })).on_hover_text("Hear notes while placing and moving them").clicked() {
                        self.prefs.note_preview = !pv;
                        self.prefs.save();
                    }

                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Transpose").small().color(TEXT_DIM));
                    for (label, s) in [("-12", -12), ("-1", -1), ("+1", 1), ("+12", 12)] {
                        if ui.small_button(label).clicked() {
                            self.piano_transpose(s);
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Select All").clicked() {
                        self.piano_select_all();
                    }
                    if ui.button("Clear").on_hover_text("Remove all notes").clicked() {
                        if let Some(v) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
                            v.clear();
                        }
                        self.piano.selected.clear();
                    }
                });
                ui.label(RichText::new(format!("{} notes, {} selected", notes.len(), self.piano.selected.len())).small().color(TEXT_DIM));
                let picked: Vec<usize> = self.piano.selected.iter().copied().filter(|&i| i < notes.len()).collect();
                match picked.first().map(|&i| notes[i].clone()) {
                    Some(first) => self.note_operators(ui, sel, &picked, &first),
                    None => {
                        ui.label(RichText::new("Double-click: add/remove note\nDrag: move, edge: resize\nCtrl+drag: copy, Alt: no snap\nCtrl+C/X/V/D: copy, cut, paste, duplicate\nArrows: move, Shift: octave / length\nSelect notes for their operators").small().color(TEXT_DIM));
                    }
                }
            }
            ClipContent::Audio(a) => {
                let mut bpm = a.bpm;
                let mut gain = a.gain_db;
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Gain").small().color(TEXT_DIM));
                    if ui.add(egui::DragValue::new(&mut gain).range(-48.0..=24.0).speed(0.1).fixed_decimals(1).suffix(" dB")).changed() {
                        self.set_audio(sel, |a| a.gain_db = gain);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Original BPM").small().color(TEXT_DIM));
                    if ui.add(egui::DragValue::new(&mut bpm).range(20.0..=999.0).speed(0.1).fixed_decimals(2)).changed() {
                        self.set_audio(sel, |a| a.bpm = bpm);
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("x2").on_hover_text("Double the original tempo").clicked() {
                        self.set_audio(sel, |a| a.bpm = (a.bpm * 2.0).min(999.0));
                    }
                    if ui.button("/2").on_hover_text("Halve the original tempo").clicked() {
                        self.set_audio(sel, |a| a.bpm = (a.bpm / 2.0).max(20.0));
                    }
                    if ui.button("Fit length").on_hover_text("Set the loop to the whole sample").clicked() {
                        if let Some(c) = self.clip_mut(sel) {
                            if let ClipContent::Audio(a) = &c.content {
                                c.length = a.sample_beats().max(0.0625);
                            }
                        }
                    }
                });
                ui.label(
                    RichText::new(format!(
                        "{}\n{} Hz, {:.2} s\nWarp: Re-Pitch (follows tempo)",
                        a.sample.name,
                        a.sample.sample_rate,
                        a.sample.duration()
                    ))
                    .small()
                    .color(TEXT_DIM),
                );
            }
        }
    }

    fn set_audio(&mut self, sel: ClipSel, f: impl FnOnce(&mut krabton_engine::AudioClip)) {
        if let Some(c) = self.clip_mut(sel) {
            if let ClipContent::Audio(a) = &mut c.content {
                f(a);
            }
        }
    }

    fn ui_audio_clip(&mut self, ui: &mut Ui, area: Rect, sel: ClipSel) {
        let Some(clip) = self.clip_ref(sel) else { return };
        let ClipContent::Audio(a) = &clip.content else { return };
        let p = ui.painter().with_clip_rect(area);
        rect_fill(&p, area, 0.0, BG);
        let r = area.shrink2(vec2(8.0, 10.0));
        let total_beats = a.sample_beats().max(clip.length).max(0.25);
        let ppb = r.width() as f64 / total_beats;
        let base = rgb(clip.color);
        let s = &a.sample;
        let src_fpb = s.sample_rate as f64 * 60.0 / a.bpm.max(1.0);
        let bpb = self.project.beats_per_bar_f();
        // Loop region.
        let loop_r = Rect::from_min_max(r.min, pos2(r.left() + (clip.length * ppb) as f32, r.bottom()));
        p.rect_filled(loop_r, 0.0, shade(PANEL2, 1.1));
        // Beat grid.
        let mut b = 0.0;
        while b <= total_beats {
            let x = r.left() + (b * ppb) as f32;
            let bar = (b / bpb).fract().abs() < 1e-9;
            p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], Stroke::new(1.0_f32, if bar { LINE } else { LINE_SOFT }));
            if bar {
                p.text(pos2(x + 2.0, r.top() + 2.0), Align2::LEFT_TOP, format!("{}", (b / bpb) as i64 + 1), small(), TEXT_DIM);
            }
            b += 1.0;
        }
        // Waveform.
        let mid = r.center().y;
        let half = r.height() * 0.45;
        let frames_per_px = src_fpb / ppb;
        let step = ((frames_per_px as usize) / krabton_engine::sample::PEAK_BLOCK).max(1);
        for x in 0..r.width() as i32 {
            let frame = (x as f64 * frames_per_px) as usize;
            let idx = frame / krabton_engine::sample::PEAK_BLOCK;
            if idx >= s.peaks.len() {
                break;
            }
            let mut mn = 0.0f32;
            let mut mx = 0.0f32;
            for j in idx..(idx + step).min(s.peaks.len()) {
                mn = mn.min(s.peaks[j].0);
                mx = mx.max(s.peaks[j].1);
            }
            let xx = r.left() + x as f32;
            let c = if (x as f64) < clip.length * ppb { base } else { shade(base, 0.4) };
            p.line_segment([pos2(xx, mid - mx * half), pos2(xx, mid - mn * half + 0.5)], Stroke::new(1.0_f32, c));
        }
        p.line_segment([pos2(loop_r.right(), r.top()), pos2(loop_r.right(), r.bottom())], Stroke::new(2.0_f32, ACCENT));
        if let Some(pos) = self.clip_play_pos(sel) {
            let x = r.left() + (pos * ppb) as f32;
            p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], Stroke::new(1.5_f32, Color32::WHITE));
        }
        // Drag the loop end marker.
        let handle = Rect::from_center_size(pos2(loop_r.right(), r.center().y), vec2(10.0, r.height()));
        let resp = ui.interact(handle, Id::new("audio_loop_end"), Sense::drag());
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if resp.dragged() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let beats = ((pos.x - r.left()) as f64 / ppb).max(0.25);
                let beats = if ui.input(|i| i.modifiers.alt) { beats } else { (beats * 4.0).round() / 4.0 };
                if let Some(c) = self.clip_mut(sel) {
                    c.length = beats.max(0.25);
                }
            }
        }
    }
}

fn body_font() -> egui::FontId {
    crate::theme::body()
}
