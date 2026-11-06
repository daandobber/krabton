//! The Step Gate's panel: its steps as bars. A click opens or closes a step, a
//! drag up and down sets how far, and the step sounding is lit.

use super::devices::param_grid_mod;
use crate::ui::modulation::ModView;
use egui::{pos2, vec2, Color32, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use krabton_engine::dsp::fx::step_gate as sg;
use krabton_engine::engine::EngineStatus;
use std::sync::atomic::Ordering;

const BAR_W: f32 = 18.0;

/// What a click does to a step: open it unless it is, then close it.
pub(crate) fn toggled(level: f32) -> f32 {
    if level >= 0.5 {
        0.0
    } else {
        1.0
    }
}

pub(crate) fn step_gate_panel(ui: &mut Ui, id: Id, p: &mut Vec<f32>, accent: Color32, h: f32, mv: Option<&mut ModView>, status: &EngineStatus) -> bool {
    if p.len() < sg::SPECS.len() {
        return false;
    }
    let mut changed = false;
    let height = (h - 24.0).clamp(90.0, 190.0);
    ui.horizontal_top(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(BAR_W * sg::MAX_STEPS as f32, height), Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 3.0, Color32::from_gray(0x1c));
        let steps = (p[sg::STEPS].round().max(1.0) as usize).min(sg::MAX_STEPS);
        let now = status.playing.load(Ordering::Relaxed).then(|| sg::step_at(p, status.session_pos.load()));
        for k in 0..sg::MAX_STEPS {
            let cell = Rect::from_min_size(pos2(rect.left() + k as f32 * BAR_W, rect.top()), vec2(BAR_W, height)).shrink2(vec2(1.5, 3.0));
            let used = k < steps;
            painter.rect_filled(cell, 2.0, Color32::from_gray(if !used { 0x20 } else if (k / 4) % 2 == 0 { 0x3c } else { 0x30 }));
            let level = p[sg::FIRST_STEP + k].clamp(0.0, 1.0);
            if used && level > 0.0 {
                let top = cell.bottom() - cell.height() * level;
                painter.rect_filled(Rect::from_min_max(pos2(cell.left(), top), cell.max), 2.0, accent);
            }
            if now == Some(k) {
                painter.rect_stroke(cell, 2.0, Stroke::new(1.5_f32, Color32::WHITE), StrokeKind::Inside);
            }
            if !used {
                continue;
            }
            let resp = ui.interact(cell, id.with(("step_gate_bar", k)), Sense::click_and_drag());
            if resp.clicked() {
                p[sg::FIRST_STEP + k] = toggled(level);
                changed = true;
            } else if resp.dragged() {
                if let Some(at) = resp.interact_pointer_pos() {
                    p[sg::FIRST_STEP + k] = ((cell.bottom() - at.y) / cell.height()).clamp(0.0, 1.0);
                    changed = true;
                }
            }
        }
        ui.add_space(8.0);
        // The first five are its knobs; the steps are the bars.
        let head = sg::FIRST_STEP;
        changed |= param_grid_mod(ui, id.with("step_gate_knobs"), &sg::SPECS[..head], &mut p[..head], accent, height, mv);
    });
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_opens_a_closed_step_and_closes_an_open_one() {
        assert_eq!(toggled(0.0), 1.0);
        assert_eq!(toggled(0.3), 1.0);
        assert_eq!(toggled(1.0), 0.0);
        assert_eq!(toggled(0.6), 0.0);
    }
}
