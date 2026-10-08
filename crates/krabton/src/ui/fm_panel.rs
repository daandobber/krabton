//! Device view for Krab FM: voice/bank selection, algorithm diagram,
//! operator table and the engine's macro controls.

use crate::app::{App, FmBank};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Pos2, Rect, RichText, Sense, Stroke, Ui};
use krabton_engine::dsp::dx7::{self, Dx7Voice, ALGORITHMS};
use krabton_engine::dsp::fm;

/// Grid positions (column, depth) of the six operators of an algorithm.
/// Carriers sit at depth 0; modulators stack above what they modulate.
pub fn algorithm_layout(alg: usize) -> [(f32, f32); 6] {
    let a = &ALGORITHMS[alg.min(31)];
    let targets = |k: usize| (0..6).filter(move |&t| a.mods[t] & (1 << k) != 0);
    let mut depth = [0usize; 6];
    for k in 0..6 {
        depth[k] = targets(k).map(|t| depth[t] + 1).max().unwrap_or(0);
    }
    let mut x = [0.0f32; 6];
    let mut col = 0.0;
    for k in 0..6 {
        if depth[k] == 0 {
            x[k] = col;
            col += 1.0;
        }
    }
    let max_depth = depth.iter().copied().max().unwrap_or(0);
    for d in 1..=max_depth {
        let mut ops: Vec<usize> = (0..6).filter(|&k| depth[k] == d).collect();
        for &k in &ops {
            let xs: Vec<f32> = targets(k).map(|t| x[t]).collect();
            x[k] = xs.iter().sum::<f32>() / xs.len().max(1) as f32;
        }
        ops.sort_by(|a, b| x[*a].total_cmp(&x[*b]).then(a.cmp(b)));
        // Siblings that share a spot are spread around it.
        let mut i = 0;
        while i < ops.len() {
            let mut j = i;
            while j + 1 < ops.len() && (x[ops[j + 1]] - x[ops[i]]).abs() < 1e-6 {
                j += 1;
            }
            let n = j - i + 1;
            if n > 1 {
                let base = x[ops[i]];
                for (m, &k) in ops[i..=j].iter().enumerate() {
                    x[k] = base + m as f32 - (n - 1) as f32 / 2.0;
                }
            }
            i = j + 1;
        }
        ops.sort_by(|a, b| x[*a].total_cmp(&x[*b]));
        for w in 1..ops.len() {
            if x[ops[w]] < x[ops[w - 1]] + 1.0 {
                x[ops[w]] = x[ops[w - 1]] + 1.0;
            }
        }
    }
    let min_x = x.iter().copied().fold(f32::INFINITY, f32::min);
    let mut out = [(0.0, 0.0); 6];
    for k in 0..6 {
        out[k] = (x[k] - min_x, depth[k] as f32);
    }
    out
}

/// Draws the operator routing of an algorithm.
fn draw_algorithm(ui: &Ui, rect: Rect, alg: usize, op_mask: u8) {
    let p = ui.painter();
    rect_fill(p, rect, 3.0, BG);
    let a = &ALGORITHMS[alg.min(31)];
    let layout = algorithm_layout(alg);
    let cols = layout.iter().map(|l| l.0).fold(0.0, f32::max) + 1.0;
    let rows = layout.iter().map(|l| l.1).fold(0.0, f32::max) + 1.0;
    let area = Rect::from_min_max(rect.min + vec2(6.0, 6.0), rect.max - vec2(6.0, 14.0));
    let cw = area.width() / cols;
    let ch = area.height() / rows;
    let size = (cw.min(ch) * 0.62).clamp(10.0, 22.0);
    let center = |k: usize| -> Pos2 {
        let (x, d) = layout[k];
        pos2(area.left() + (x + 0.5) * cw, area.bottom() - (d + 0.5) * ch)
    };
    let line = Stroke::new(1.5_f32, TEXT_DIM);
    // Modulation connections.
    for t in 0..6 {
        for k in 0..6 {
            if a.mods[t] & (1 << k) != 0 {
                p.line_segment([center(k) + vec2(0.0, size * 0.5), center(t) - vec2(0.0, size * 0.5)], line);
            }
        }
    }
    // Carriers to the output bus.
    let bus_y = rect.bottom() - 8.0;
    let carriers: Vec<usize> = (0..6).filter(|&k| a.carriers & (1 << k) != 0).collect();
    if let (Some(&first), Some(&last)) = (carriers.first(), carriers.last()) {
        p.line_segment([pos2(center(first).x, bus_y), pos2(center(last).x, bus_y)], Stroke::new(2.0_f32, ACCENT));
    }
    for &k in &carriers {
        p.line_segment([center(k) + vec2(0.0, size * 0.5), pos2(center(k).x, bus_y)], Stroke::new(1.5_f32, ACCENT));
    }
    // Feedback loop.
    let (from, to) = (center(a.fb_from), center(a.fb_to));
    let rx = from.x.max(to.x) + size * 0.5 + 5.0;
    let top = to.y - size * 0.5 - 5.0;
    let pts = vec![
        from + vec2(size * 0.5, 0.0),
        pos2(rx, from.y),
        pos2(rx, top),
        pos2(to.x, top),
        to - vec2(0.0, size * 0.5),
    ];
    p.add(egui::Shape::line(pts, Stroke::new(1.2_f32, shade(ACCENT, 0.7))));
    // Operator boxes.
    for k in 0..6 {
        let r = Rect::from_center_size(center(k), vec2(size, size));
        let on = op_mask & (1 << k) != 0;
        let carrier = a.carriers & (1 << k) != 0;
        let fill = match (on, carrier) {
            (false, _) => PANEL2,
            (true, true) => ACCENT,
            (true, false) => PANEL3,
        };
        rect_fill(p, r, 2.0, fill);
        rect_line(p, r, 2.0, 1.0, if on { ACCENT } else { LINE });
        p.text(r.center(), Align2::CENTER_CENTER, k + 1, small(), if carrier && on { TEXT_DARK } else { TEXT });
    }
    p.text(pos2(rect.left() + 4.0, rect.top() + 2.0), Align2::LEFT_TOP, format!("Alg {}", alg + 1), small(), TEXT_DIM);
}

/// A tiny drawing of an operator's 4-stage envelope.
fn draw_envelope(ui: &Ui, rect: Rect, voice: &Dx7Voice, op: usize, color: Color32) {
    let p = ui.painter();
    rect_fill(p, rect, 2.0, BG);
    let r = |i: usize| voice.op(op, dx7::R1 + i) as f32;
    let l = |i: usize| voice.op(op, dx7::L1 + i) as f32 / 99.0;
    let seg = |rate: f32| 0.15 + (99.0 - rate) / 99.0;
    let widths = [seg(r(0)), seg(r(1)), seg(r(2)), 0.6, seg(r(3))];
    let total: f32 = widths.iter().sum();
    let sx = rect.width() / total;
    let y = |lv: f32| rect.bottom() - 1.0 - lv * (rect.height() - 2.0);
    let mut x = rect.left();
    let mut pts = vec![pos2(x, y(l(3)))];
    for (i, lv) in [l(0), l(1), l(2), l(2), l(3)].into_iter().enumerate() {
        x += widths[i] * sx;
        pts.push(pos2(x, y(lv)));
    }
    p.add(egui::Shape::line(pts, Stroke::new(1.2_f32, color)));
}

fn ratio_text(voice: &Dx7Voice, op: usize) -> String {
    let coarse = voice.op(op, dx7::PC) as f32;
    let fine = voice.op(op, dx7::PF) as f32;
    if voice.op(op, dx7::PM) == 1 {
        let hz = 10f32.powf((voice.op(op, dx7::PC) & 3) as f32 + fine / 100.0);
        if hz >= 1000.0 {
            format!("{:.2}kHz", hz / 1000.0)
        } else {
            format!("{hz:.1}Hz")
        }
    } else {
        let ratio = if coarse == 0.0 { 0.5 } else { coarse } * (1.0 + fine / 100.0);
        format!("x{ratio:.2}")
    }
}

/// A small drag control for a DX7 byte parameter. Returns the new value if changed.
fn byte_drag(ui: &mut Ui, value: u8, range: std::ops::RangeInclusive<i32>, fmt: impl Fn(i32) -> String + 'static) -> Option<u8> {
    let mut v = value as i32;
    let resp = ui.add(egui::DragValue::new(&mut v).range(range).speed(0.15).custom_formatter(move |x, _| fmt(x as i32)));
    (resp.changed() && v != value as i32).then_some(v as u8)
}

impl App {
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn fm_panel(&mut self, ui: &mut Ui, track_id: u64, voice_bytes: &mut Vec<u8>, op_mask: &mut u8, bank: &mut String, params: &mut Vec<f32>, h: f32, mv: Option<&mut crate::ui::modulation::ModView>) {
        let mut voice = Dx7Voice::from_slice(voice_bytes).unwrap_or_default();
        let before = voice.clone();
        ui.horizontal_top(|ui| {
            // Voice and bank.
            ui.vertical(|ui| {
                ui.set_width(210.0);
                ui.label(RichText::new(voice.name()).strong().size(17.0).color(ACCENT));
                ui.label(RichText::new("6-op FM, DX7 voice compatible").small().color(TEXT_DIM));
                let bank_names: Vec<String> = self.fm_banks.iter().map(|b| b.name.clone()).collect();
                egui::ComboBox::from_id_salt(("fm_bank", track_id)).width(190.0).selected_text(bank.as_str()).show_ui(ui, |ui| {
                    for name in &bank_names {
                        if ui.selectable_label(bank == name, name).clicked() {
                            *bank = name.clone();
                        }
                    }
                });
                let voices: Vec<Dx7Voice> = self.fm_banks.iter().find(|b| &b.name == bank).map(|b| b.voices.clone()).unwrap_or_default();
                let current = voices.iter().position(|v| *v == voice);
                ui.horizontal(|ui| {
                    if ui.small_button("<").on_hover_text("Previous voice").clicked() && !voices.is_empty() {
                        let i = current.map(|i| (i + voices.len() - 1) % voices.len()).unwrap_or(0);
                        voice = voices[i].clone();
                    }
                    let label = current.map(|i| format!("{:02} {}", i + 1, voices[i].name())).unwrap_or_else(|| "(edited)".into());
                    egui::ComboBox::from_id_salt(("fm_voice", track_id)).width(140.0).selected_text(label).height(400.0).show_ui(ui, |ui| {
                        for (i, v) in voices.iter().enumerate() {
                            if ui.selectable_label(current == Some(i), format!("{:02} {}", i + 1, v.name())).clicked() {
                                voice = v.clone();
                            }
                        }
                    });
                    if ui.small_button(">").on_hover_text("Next voice").clicked() && !voices.is_empty() {
                        let i = current.map(|i| (i + 1) % voices.len()).unwrap_or(0);
                        voice = voices[i].clone();
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Load .syx...").on_hover_text("Load a DX7 bank or voice (Dexed and MiniDexed banks work)").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("DX7 sysex", &["syx", "SYX"]).pick_file() {
                            match self.load_fm_bank(&path) {
                                Ok(i) => {
                                    *bank = self.fm_banks[i].name.clone();
                                    if let Some(v) = self.fm_banks[i].voices.first() {
                                        voice = v.clone();
                                    }
                                }
                                Err(e) => self.toast(e),
                            }
                        }
                    }
                    if ui.button("Save...").on_hover_text("Export this voice as a DX7 single-voice .syx").clicked() {
                        let name = format!("{}.syx", voice.name().trim().replace(' ', "_"));
                        if let Some(path) = rfd::FileDialog::new().add_filter("DX7 sysex", &["syx"]).set_file_name(name).save_file() {
                            match std::fs::write(&path, dx7::single_voice_sysex(&voice)) {
                                Ok(()) => self.toast(format!("Saved {}", path.display())),
                                Err(e) => self.toast(format!("Could not save: {e}")),
                            }
                        }
                    }
                });
                let mut name = voice.name();
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Name").small().color(TEXT_DIM));
                    if ui.add(egui::TextEdit::singleline(&mut name).char_limit(10).desired_width(120.0)).changed() {
                        voice.set_name(&name);
                    }
                });
                egui::Grid::new(("fm_global", track_id)).num_columns(4).spacing(vec2(6.0, 3.0)).show(ui, |ui| {
                    ui.label(RichText::new("Algorithm").small().color(TEXT_DIM));
                    if let Some(v) = byte_drag(ui, voice.get(dx7::ALG), 0..=31, |x| format!("{}", x + 1)) {
                        voice.set(dx7::ALG, v);
                    }
                    ui.label(RichText::new("Feedback").small().color(TEXT_DIM));
                    if let Some(v) = byte_drag(ui, voice.get(dx7::FB), 0..=7, |x| format!("{x}")) {
                        voice.set(dx7::FB, v);
                    }
                    ui.end_row();
                    ui.label(RichText::new("LFO speed").small().color(TEXT_DIM));
                    if let Some(v) = byte_drag(ui, voice.get(dx7::LFS), 0..=99, |x| format!("{x}")) {
                        voice.set(dx7::LFS, v);
                    }
                    ui.label(RichText::new("Vibrato").small().color(TEXT_DIM));
                    if let Some(v) = byte_drag(ui, voice.get(dx7::LPMD), 0..=99, |x| format!("{x}")) {
                        voice.set(dx7::LPMD, v);
                    }
                    ui.end_row();
                });
            });

            // Algorithm diagram.
            let (r, _) = ui.allocate_exact_size(vec2(170.0, (h - 40.0).clamp(110.0, 190.0)), Sense::hover());
            draw_algorithm(ui, r, voice.algorithm(), *op_mask);

            // Operator table.
            ui.vertical(|ui| {
                egui::Grid::new(("fm_ops", track_id)).num_columns(9).spacing(vec2(5.0, 2.0)).show(ui, |ui| {
                    for head in ["", "Level", "Ratio", "Coarse", "Fine", "Detune", "Fixed", "Vel", "Envelope"] {
                        ui.label(RichText::new(head).small().color(TEXT_DIM));
                    }
                    ui.end_row();
                    let carriers = ALGORITHMS[voice.algorithm()].carriers;
                    for op in 0..6 {
                        let on = *op_mask & (1 << op) != 0;
                        let carrier = carriers & (1 << op) != 0;
                        let label = RichText::new(format!("OP{}", op + 1)).color(if !on {
                            TEXT_DIM
                        } else if carrier {
                            ACCENT
                        } else {
                            TEXT
                        });
                        if ui.add(egui::Button::new(label).fill(if on { PANEL3 } else { BG }).min_size(vec2(40.0, 18.0))).on_hover_text("Operator on/off").clicked() {
                            *op_mask ^= 1 << op;
                        }
                        if let Some(v) = byte_drag(ui, voice.op(op, dx7::OL), 0..=99, |x| format!("{x}")) {
                            voice.set_op(op, dx7::OL, v);
                        }
                        ui.label(RichText::new(ratio_text(&voice, op)).monospace().small());
                        if let Some(v) = byte_drag(ui, voice.op(op, dx7::PC), 0..=31, |x| format!("{x}")) {
                            voice.set_op(op, dx7::PC, v);
                        }
                        if let Some(v) = byte_drag(ui, voice.op(op, dx7::PF), 0..=99, |x| format!("{x}")) {
                            voice.set_op(op, dx7::PF, v);
                        }
                        if let Some(v) = byte_drag(ui, voice.op(op, dx7::PD), 0..=14, |x| format!("{:+}", x - 7)) {
                            voice.set_op(op, dx7::PD, v);
                        }
                        let mut fixed = voice.op(op, dx7::PM) == 1;
                        if ui.checkbox(&mut fixed, "").on_hover_text("Fixed frequency instead of a ratio").changed() {
                            voice.set_op(op, dx7::PM, fixed as u8);
                        }
                        if let Some(v) = byte_drag(ui, voice.op(op, dx7::KVS), 0..=7, |x| format!("{x}")) {
                            voice.set_op(op, dx7::KVS, v);
                        }
                        let (er, _) = ui.allocate_exact_size(vec2(64.0, 16.0), Sense::hover());
                        draw_envelope(ui, er, &voice, op, if carrier { ACCENT } else { TEXT_DIM });
                        ui.end_row();
                    }
                });
            });

            // Engine macros (MiniDexed-style tone generator settings).
            ui.vertical(|ui| {
                ui.label(RichText::new("Krab FM").small().color(TEXT_DIM));
                crate::ui::devices::param_grid_mod(ui, Id::new(("fm_params", track_id)), fm::SPECS, params, ACCENT, h - 40.0, mv);
            });
        });
        if voice != before {
            *voice_bytes = voice.0.to_vec();
        }
    }
}

impl FmBank {
    pub fn factory() -> Self {
        FmBank { name: "Krab Factory".into(), path: None, voices: dx7::factory_bank() }
    }
}
