//! Gabber's face: one kick drawn as it sounds now, big, acid green on
//! black, the clip's walls in red: turn Drive up and watch it square up.
//! Beside it how hard it is hit now, the rolls as buttons, and every
//! setting as a knob.

use super::inst_screen::InstLive;
use super::modulation::ModView;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, RichText, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::gabber::{self as gb, *};

const ACID: Color32 = Color32::from_rgb(0x7c, 0xff, 0x2e);
const WARN: Color32 = Color32::from_rgb(0xff, 0x2a, 0x1f);
const BG: Color32 = Color32::from_rgb(0x03, 0x05, 0x03);

pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut Vec<f32>, live: &InstLive, height: f32, mv: Option<&mut ModView>) {
    if params.len() < SPECS.len() {
        return;
    }
    ui.horizontal(|ui| {
        ui.label(RichText::new("Preset").small().color(crate::skin::ink_dim()));
        egui::ComboBox::from_id_salt(id.with("preset")).width(140.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
            for name in preset_names() {
                if ui.selectable_label(preset == name, *name).clicked() {
                    *preset = name.to_string();
                    *params = gb::preset(name);
                }
            }
        });
        ui.add_space(10.0);
        ui.label(RichText::new("Roll").small().color(crate::skin::ink_dim())).on_hover_text("Hold a key and it rolls on the song's grid");
        for (k, name) in ROLLS.iter().enumerate() {
            let on = params[ROLL].round() as usize == k;
            if ui.selectable_label(on, RichText::new(*name).color(if on { ACID } else { crate::skin::ink() })).on_hover_text(if k == 0 { "Held, it strikes once" } else { "Held, it strikes again on this grid while the song plays" }).clicked() {
                params[ROLL] = k as f32;
            }
        }
    });
    let h = (height - 28.0).max(140.0);
    ui.horizontal_top(|ui| {
        // The kick, drawn.
        let w = (h * 1.6).min(ui.available_width() * 0.45).max(240.0);
        let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let p = ui.painter_at(rect);
        rect_fill(&p, rect, 4.0, BG);
        let key = id.with("preview");
        let sig: u64 = params.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, v| (h ^ v.to_bits() as u64).wrapping_mul(0x100_0000_01b3));
        let wave: Vec<f32> = match ui.data(|d| d.get_temp::<(u64, std::sync::Arc<Vec<f32>>)>(key)) {
            Some((s, w)) if s == sig => (*w).clone(),
            _ => {
                let w = std::sync::Arc::new(gb::preview(params, 36, 0.6, 360));
                ui.data_mut(|d| d.insert_temp(key, (sig, w.clone())));
                (*w).clone()
            }
        };
        let inner = rect.shrink(8.0);
        let mid = inner.center().y;
        let peak = wave.iter().fold(0.05f32, |m, v| m.max(v.abs()));
        let scale = inner.height() * 0.45 / peak;
        // The clip's walls.
        for y in [mid - 0.85 * scale * peak.min(1.0), mid + 0.85 * scale * peak.min(1.0)] {
            p.line_segment([pos2(inner.left(), y), pos2(inner.right(), y)], Stroke::new(1.0_f32, WARN.gamma_multiply(0.45)));
        }
        p.line_segment([pos2(inner.left(), mid), pos2(inner.right(), mid)], Stroke::new(1.0_f32, Color32::from_white_alpha(20)));
        let n = wave.len().max(2);
        let pts: Vec<egui::Pos2> = wave.iter().enumerate().map(|(k, v)| pos2(inner.left() + inner.width() * k as f32 / (n - 1) as f32, mid - v * scale)).collect();
        for q in &pts {
            p.line_segment([pos2(q.x, mid), *q], Stroke::new(1.0_f32, ACID.gamma_multiply(0.35)));
        }
        p.add(Shape::line(pts.clone(), Stroke::new(3.0_f32, ACID.gamma_multiply(0.25))));
        p.add(Shape::line(pts, Stroke::new(1.4_f32, ACID)));
        text(&p, inner.left_top(), Align2::LEFT_TOP, "ONE KICK  ·  600 MS  ·  C2", FontId::monospace(10.0), crate::skin::ink_dim());
        text(&p, inner.right_top(), Align2::RIGHT_TOP, format!("DRIVE {:+.0} dB", params[DRIVE]), FontId::monospace(10.0), ACID);
        // How hard it hits now.
        let m = live.vals[0].clamp(0.0, 1.5) / 1.5;
        let bar = Rect::from_min_max(pos2(inner.right() - 10.0, inner.top() + 16.0), pos2(inner.right() - 4.0, inner.bottom()));
        p.rect_filled(bar, 2.0, Color32::from_rgb(0x10, 0x16, 0x10));
        let lit = Rect::from_min_max(pos2(bar.left(), bar.bottom() - bar.height() * m), bar.max);
        p.rect_filled(lit, 2.0, if m > 0.8 { WARN } else { ACID });
        if m > 0.0 {
            ui.ctx().request_repaint();
        }
        ui.add_space(8.0);
        super::devices::param_grid_mod(ui, id.with("knobs"), SPECS, params, accent_now(), h, mv);
    });
}
