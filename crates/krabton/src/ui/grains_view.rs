//! The Grains device: its buffer as a timeline (now on the right, older to
//! the left) with every grain as a little block where it reads, as long as
//! it is, higher for higher pitch and as bright as its envelope. Drag in it:
//! left and right for Time, up and down for Size.

use super::inst_screen::SCREEN_BG;
use super::modulation::ModView;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::fx::grains::*;
use krabton_engine::EngineStatus;

const ICE: Color32 = Color32::from_rgb(0x9f, 0xd8, 0xff);
const GRAIN_UP: Color32 = Color32::from_rgb(0xff, 0xd4, 0x3b);
const GRAIN_DOWN: Color32 = Color32::from_rgb(0x4d, 0x8d, 0xff);
const GRAIN: Color32 = Color32::from_rgb(0x30, 0xd9, 0x8f);

#[allow(clippy::too_many_arguments)]
pub fn grains_panel(ui: &mut Ui, id: Id, dev_id: u64, p: &mut Vec<f32>, accent: Color32, h: f32, mv: Option<&mut ModView>, status: &EngineStatus, width: &mut f32) -> bool {
    if p.len() < SPECS.len() {
        return false;
    }
    let m = status.meters.read(dev_id).unwrap_or([0.0; krabton_engine::scope::METER_VALUES]);
    let mut changed = false;
    ui.horizontal_top(|ui| {
        let (rect, resp) = ui.allocate_exact_size(vec2(*width, (h - 30.0).max(110.0)), Sense::click_and_drag());
        width_grip(ui, id.with("display_width"), rect, width, 220.0, 1400.0);
        let inner = rect.shrink2(vec2(8.0, 18.0));
        if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            let time = ((inner.right() - pos.x) / inner.width()).clamp(0.0, 1.0);
            let size = ((inner.bottom() - pos.y) / inner.height()).clamp(0.0, 1.0);
            let (time, size) = ((time * 100.0).round() / 100.0, (size * 100.0).round() / 100.0);
            if p[TIME] != time || p[SIZE] != size {
                p[TIME] = time;
                p[SIZE] = size;
                changed = true;
            }
        }
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        let painter = ui.painter_at(rect);
        rect_fill(&painter, rect, 6.0, SCREEN_BG);
        let frozen = m[1] > 0.5;
        // The part of the buffer not recorded yet.
        let filled = m[2].clamp(0.0, 1.0);
        if filled < 1.0 {
            rect_fill(&painter, Rect::from_min_max(inner.min, pos2(inner.right() - filled * inner.width(), inner.bottom())), 0.0, Color32::from_black_alpha(90));
        }
        // What comes out, faint behind.
        if let Some(buf) = status.scopes.find(dev_id) {
            let (mut l, mut r) = (Vec::new(), Vec::new());
            buf.read(4096, &mut l, &mut r);
            if l.len() > 2 {
                let pts: Vec<_> = (0..200)
                    .map(|i| {
                        let k = i * l.len() / 200;
                        pos2(inner.left() + i as f32 / 199.0 * inner.width(), inner.center().y - (l[k] + r.get(k).copied().unwrap_or(0.0)) * 0.5 * inner.height() * 0.45)
                    })
                    .collect();
                painter.add(Shape::line(pts, Stroke::new(1.0_f32, Color32::from_white_alpha(28))));
            }
        }
        // Pitch lines: unison, octaves.
        let y_of = |semis: f32| inner.center().y - semis / 24.0 * inner.height() * 0.45;
        for s in [-24.0, -12.0, 0.0, 12.0, 24.0] {
            painter.line_segment([pos2(inner.left(), y_of(s)), pos2(inner.right(), y_of(s))], Stroke::new(1.0_f32, Color32::from_white_alpha(if s == 0.0 { 20 } else { 10 })));
        }
        // Where grains start (Time, and its random range).
        let tx = inner.right() - p[TIME].clamp(0.0, 1.0) * inner.width();
        let spread = p[TIME_RAND].clamp(0.0, 1.0) * 0.5 * inner.width();
        rect_fill(&painter, Rect::from_min_max(pos2((tx - spread).max(inner.left()), inner.top()), pos2((tx + spread).min(inner.right()), inner.bottom())), 0.0, accent.gamma_multiply(0.06));
        painter.extend(Shape::dashed_line(&[pos2(tx, inner.top()), pos2(tx, inner.bottom())], Stroke::new(1.0_f32, accent.gamma_multiply(0.6)), 4.0, 3.0));
        // The grains.
        let count = (m[0].round() as usize).min(SHOWN);
        for k in 0..count {
            let back = m[3 + k * 3];
            let len = m[4 + k * 3];
            let packed = m[5 + k * 3];
            let semis = packed.floor() - 48.0;
            let env = (packed.fract() / 0.99).clamp(0.0, 1.0);
            let x = inner.right() - back * inner.width();
            let w = (len * inner.width()).max(3.0);
            let c = if semis > 0.5 {
                GRAIN_UP
            } else if semis < -0.5 {
                GRAIN_DOWN
            } else {
                GRAIN
            };
            let r = Rect::from_min_size(pos2(x, y_of(semis) - 5.0), vec2(w, 10.0));
            rect_fill(&painter, r, 3.0, c.gamma_multiply(0.25 + 0.75 * env));
        }
        if m[30] > 0.0 {
            // Delay mode: the read head.
            let x = inner.right() - m[30] * inner.width();
            painter.line_segment([pos2(x, inner.top()), pos2(x, inner.bottom())], Stroke::new(2.0_f32, GRAIN));
        }
        // Now: the write head.
        painter.line_segment([pos2(inner.right(), inner.top()), pos2(inner.right(), inner.bottom())], Stroke::new(2.0_f32, if frozen { ICE } else { Color32::from_rgb(0xff, 0x4e, 0x62) }));
        if frozen {
            rect_line(&painter, rect.shrink(1.0), 6.0, 2.0, ICE);
        }
        // What it does, in words.
        let d = p[DENSITY];
        let what = if p[SIZE] >= 0.98 {
            "delay".to_string()
        } else if d.abs() <= 0.02 {
            "no grains".to_string()
        } else {
            let rate = 0.5 * 160f32.powf(d.abs());
            if d > 0.0 {
                format!("random {rate:.0}/s")
            } else if p[SYNC] >= 0.5 {
                let div = ["1 bar", "1/2", "1/4", "1/8", "1/16", "1/32", "1/64", "1/128"][((d.abs() * 7.0).round() as usize).min(7)];
                format!("regular {div}")
            } else {
                format!("regular {rate:.0}/s")
            }
        };
        let q = QUALITIES[(p[QUALITY].round().max(0.0) as usize).min(QUALITIES.len() - 1)];
        painter.text(rect.left_top() + vec2(8.0, 4.0), Align2::LEFT_TOP, format!("{what} · {q}{}", if frozen { " · frozen" } else { "" }).to_uppercase(), FontId::monospace(10.0), accent);
        painter.text(pos2(inner.left(), rect.bottom() - 4.0), Align2::LEFT_BOTTOM, "older", FontId::monospace(9.0), TEXT_DIM);
        painter.text(pos2(inner.right(), rect.bottom() - 4.0), Align2::RIGHT_BOTTOM, "now", FontId::monospace(9.0), TEXT_DIM);
        if count > 0 {
            ui.ctx().request_repaint();
        }
        resp.on_hover_text("Drag: left and right for Time (how far back), up and down for Size. Drag the right edge to make it wider.");
        ui.add_space(6.0);
        changed |= crate::ui::devices::param_grid_mod(ui, id, SPECS, p, accent, h - 24.0, mv);
    });
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_display_draws_and_drags() {
        let ctx = egui::Context::default();
        let status = EngineStatus::new();
        let mut p = krabton_engine::params::defaults(SPECS);
        let mut width = 360.0;
        let mut origin = egui::Pos2::ZERO;
        let frames: Vec<Vec<egui::Event>> = vec![
            vec![],
            vec![egui::Event::PointerMoved(pos2(60.0, 40.0)), egui::Event::PointerButton { pos: pos2(60.0, 40.0), button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() }],
            vec![egui::Event::PointerButton { pos: pos2(60.0, 40.0), button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }],
        ];
        for (i, evs) in frames.into_iter().enumerate() {
            let evs: Vec<egui::Event> = evs
                .into_iter()
                .map(|e| match e {
                    egui::Event::PointerMoved(q) => egui::Event::PointerMoved(origin + q.to_vec2()),
                    egui::Event::PointerButton { pos, button, pressed, modifiers } => egui::Event::PointerButton { pos: origin + pos.to_vec2(), button, pressed, modifiers },
                    e => e,
                })
                .collect();
            let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1200.0, 800.0))), events: evs, ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    if i == 0 {
                        origin = ui.cursor().min;
                    }
                    grains_panel(ui, Id::new("g"), 1, &mut p, accent_now(), 200.0, None, &status, &mut width);
                });
            });
        }
        // A click towards the left and top: far back, big grains.
        assert!(p[TIME] > 0.7, "time {}", p[TIME]);
        assert!(p[SIZE] > 0.6, "size {}", p[SIZE]);
    }
}
