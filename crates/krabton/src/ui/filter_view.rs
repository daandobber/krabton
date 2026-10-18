//! VA Filter's display: the model's response over the audible range, the
//! cutoff marked where it is set; drag sideways for the cutoff, up and down
//! for the resonance, click for the next model (right-click: back). Its
//! knobs beside it.

use super::devices::param_grid_mod;
use super::modulation::ModView;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Pos2, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::fx::vafilter::{self as vf, CUTOFF, MODEL, MODELS, RESONANCE};

pub(crate) fn va_filter_panel(ui: &mut Ui, id: Id, params: &mut [f32], accent: Color32, h: f32, mv: Option<&mut ModView>, width: &mut f32) -> bool {
    ui.horizontal_top(|ui| {
        let mut changed = false;
        let (rect, resp) = ui.allocate_exact_size(vec2(*width, (h - 30.0).max(80.0)), Sense::click_and_drag());
        let p = ui.painter_at(rect);
        rect_fill(&p, rect, 3.0, Color32::from_rgb(0x14, 0x16, 0x1b));
        let model = (params[MODEL].round().max(0.0) as usize).min(MODELS.len() - 1);
        let cutoff = params[CUTOFF].clamp(20.0, 20000.0);
        let res = params[RESONANCE].clamp(0.0, 1.0);
        let x_of = |f: f32| rect.left() + (f / 20.0).log(1000.0).clamp(0.0, 1.0) * rect.width();
        let y_of = |db: f32| rect.center().y - (db / 30.0).clamp(-0.5, 0.48) * rect.height();
        // Octave lines and 0 dB.
        for f in [50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0] {
            let x = x_of(f);
            p.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(1.0_f32, Color32::from_rgb(0x22, 0x25, 0x2c)));
        }
        p.line_segment([pos2(rect.left(), y_of(0.0)), pos2(rect.right(), y_of(0.0))], Stroke::new(1.0_f32, Color32::from_rgb(0x2c, 0x30, 0x38)));
        let pts: Vec<Pos2> = (0..=160)
            .map(|i| {
                let t = i as f32 / 160.0;
                let f = 20.0 * 1000f32.powf(t);
                let db = 20.0 * vf::response(model, cutoff, res, f).max(1e-4).log10();
                pos2(rect.left() + t * rect.width(), y_of(db))
            })
            .collect();
        let mut fill = pts.clone();
        fill.push(rect.right_bottom());
        fill.push(rect.left_bottom());
        p.add(Shape::convex_polygon(fill, accent.gamma_multiply(0.12), Stroke::NONE));
        p.add(Shape::line(pts, Stroke::new(2.0_f32, accent)));
        let cx = x_of(cutoff);
        p.line_segment([pos2(cx, rect.top() + 18.0), pos2(cx, rect.bottom())], Stroke::new(1.0_f32, accent.gamma_multiply(0.5)));
        let label = if cutoff >= 1000.0 { format!("{:.1} kHz", cutoff / 1000.0) } else { format!("{:.0} Hz", cutoff) };
        p.text(rect.left_top() + vec2(8.0, 6.0), Align2::LEFT_TOP, MODELS[model], FontId::proportional(14.0), TEXT);
        p.text(rect.right_top() + vec2(-8.0, 8.0), Align2::RIGHT_TOP, format!("{label}  ·  res {:>3.0}%", res * 100.0), FontId::monospace(10.0), TEXT_DIM);
        if resp.dragged() {
            let d = resp.drag_delta();
            let f = (cutoff * 2f32.powf(d.x / rect.width() * 10.0)).clamp(20.0, 20000.0);
            if f != params[CUTOFF] {
                params[CUTOFF] = f;
                changed = true;
            }
            let r = (res - d.y / rect.height()).clamp(0.0, 1.0);
            if r != params[RESONANCE] {
                params[RESONANCE] = r;
                changed = true;
            }
        }
        if resp.clicked() {
            params[MODEL] = ((model + 1) % MODELS.len()) as f32;
            changed = true;
        } else if resp.secondary_clicked() {
            params[MODEL] = ((model + MODELS.len() - 1) % MODELS.len()) as f32;
            changed = true;
        }
        resp.on_hover_text("Drag sideways for the cutoff, up and down for the resonance; click for the next model (right-click: back). Drag the right edge to make it wider.");
        width_grip(ui, id.with("display_width"), rect, width, 200.0, 1400.0);
        ui.add_space(6.0);
        changed | param_grid_mod(ui, id, vf::SPECS, params, accent, h - 24.0, mv)
    })
    .inner
}
