//! The Beads device: the recording as a string, now on the right and older
//! to the left, with every grain a bead where it reads: bigger the louder,
//! higher the higher it plays, another colour when it plays backwards. In
//! delay mode the string is the delay; with the wavetables it is the wave
//! playing. Drag: left and right for TIME, up and down for PITCH.

use super::inst_screen::{caption, SCREEN_BG};
use super::modulation::ModView;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Pos2, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::fx::beads::*;
use krabton_engine::EngineStatus;

const DIM: Color32 = Color32::from_rgb(0x4a, 0x4e, 0x58);
const FAINT: Color32 = Color32::from_rgb(0x1e, 0x21, 0x28);
const BACKWARDS: Color32 = Color32::from_rgb(0x7f, 0xb8, 0xff);
const ICE: Color32 = Color32::from_rgb(0x9f, 0xd8, 0xff);

/// The wavetables, read once.
fn waves() -> &'static [i16] {
    static W: std::sync::OnceLock<Vec<i16>> = std::sync::OnceLock::new();
    W.get_or_init(krabton_beads::synth::waves)
}

#[allow(clippy::too_many_arguments)]
pub fn beads_panel(ui: &mut Ui, id: Id, dev_id: u64, p: &mut Vec<f32>, accent: Color32, h: f32, mv: Option<&mut ModView>, status: &EngineStatus, width: &mut f32) -> bool {
    if p.len() < SPECS.len() {
        return false;
    }
    let m = status.meters.read(dev_id).unwrap_or([0.0; krabton_engine::scope::METER_VALUES]);
    let mut changed = false;
    ui.horizontal_top(|ui| {
        let (rect, resp) = ui.allocate_exact_size(vec2(*width, (h - 30.0).max(110.0)), Sense::click_and_drag());
        width_grip(ui, id.with("display_width"), rect, width, 240.0, 1200.0);
        let inner = rect.shrink2(vec2(14.0, 24.0));
        if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            let time = (((inner.right() - pos.x) / inner.width()).clamp(0.0, 1.0) * 100.0).round() / 100.0;
            let pitch = ((inner.center().y - pos.y) / (inner.height() * 0.5) * 24.0).clamp(-24.0, 24.0).round();
            if p[TIME] != time || p[PITCH] != pitch {
                p[TIME] = time;
                p[PITCH] = pitch;
                changed = true;
            }
        }
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        draw(ui, rect, inner, p, &m, accent);
        resp.on_hover_text("Every grain is a bead where it reads: bigger the louder, higher the higher it plays, blue when backwards. Drag: left and right for Time (right is now), up and down for Pitch");
        ui.add_space(6.0);
        changed |= crate::ui::devices::param_grid_mod(ui, id, SPECS, p, accent, h - 24.0, mv);
    });
    changed
}

fn draw(ui: &Ui, rect: Rect, inner: Rect, p: &[f32], m: &[f32], accent: Color32) {
    let painter = ui.painter_at(rect);
    rect_fill(&painter, rect, 6.0, SCREEN_BG);
    let t = ui.input(|i| i.time) as f32;
    let wavetables = m[4] > 0.5;
    let delay = m[1] > 0.5;
    let frozen = m[5] > 0.5 || p[FREEZE] >= 0.5;
    let cy = inner.center().y;
    let x_of = |pos: f32| inner.right() - pos.clamp(0.0, 1.0) * inner.width();
    let y_of = |pitch: f32| cy - (pitch / 24.0).clamp(-1.0, 1.0) * inner.height() * 0.5;
    // Octave lines.
    for st in [-24.0f32, -12.0, 12.0, 24.0] {
        let y = y_of(st);
        painter.line_segment([pos2(inner.left(), y), pos2(inner.right(), y)], Stroke::new(1.0_f32, FAINT));
    }
    // The string: the recording (or the wave).
    let string_color = if frozen { ICE } else { mix(DIM, accent, 0.35) };
    if wavetables {
        let w = waves();
        let bank = (m[3].max(0.0) as usize).min(krabton_beads::synth::NUM_BANKS - 1);
        let wave = (p[TIME].clamp(0.0, 1.0) * 15.0).round() as usize;
        let start = (bank * 16 + wave.min(15)) * (256 + 4);
        let diffs: Vec<f32> = (0..256).map(|i| w[start + i + 2] as f32 - w[start + i + 1] as f32).collect();
        let peak = diffs.iter().fold(1.0f32, |a, v| a.max(v.abs()));
        let pts: Vec<Pos2> = diffs.iter().enumerate().map(|(i, v)| pos2(inner.left() + i as f32 / 255.0 * inner.width(), cy - v / peak * inner.height() * 0.35)).collect();
        painter.add(Shape::line(pts, Stroke::new(1.5_f32, string_color)));
    } else {
        // The part recorded so far, brighter; a slow sway keeps it alive.
        let filled = m[0].clamp(0.0, 1.0);
        let n = 64;
        let pts: Vec<Pos2> = (0..=n)
            .map(|k| {
                let u = k as f32 / n as f32;
                let x = inner.left() + u * inner.width();
                pos2(x, cy + (u * 9.0 - t * 1.3).sin() * 1.5)
            })
            .collect();
        let split = ((1.0 - filled) * n as f32) as usize;
        if split > 0 {
            painter.add(Shape::line(pts[..=split.min(n)].to_vec(), Stroke::new(1.0_f32, FAINT)));
        }
        if split < n {
            painter.add(Shape::line(pts[split..].to_vec(), Stroke::new(2.0_f32, string_color)));
        }
        // Where TIME points.
        let tx = x_of(p[TIME]);
        painter.line_segment([pos2(tx, inner.top()), pos2(tx, inner.bottom())], Stroke::new(1.0_f32, accent.gamma_multiply(0.35)));
    }
    // The beads.
    for i in 0..SHOWN_GRAINS {
        let o = 6 + 3 * i;
        let level = m[o + 1];
        if level <= 0.001 {
            continue;
        }
        let raw = m[o + 2];
        let (pitch, back) = if raw > 50.0 { (raw - 100.0, true) } else { (raw, false) };
        let x = if wavetables { inner.left() + m[o].clamp(0.0, 1.0) * inner.width() } else { x_of(m[o]) };
        let y = y_of(pitch) + (t * 2.0 + i as f32).sin() * 1.0;
        let r = 3.0 + 9.0 * level.min(1.0).sqrt();
        let c = if back { BACKWARDS } else { accent };
        painter.line_segment([pos2(x, cy), pos2(x, y)], Stroke::new(1.0_f32, c.gamma_multiply(0.25)));
        painter.circle_filled(pos2(x, y), r + 3.0, c.gamma_multiply(0.12));
        painter.circle_filled(pos2(x, y), r, c.gamma_multiply(0.55 + 0.45 * level.min(1.0)));
        painter.circle_filled(pos2(x - r * 0.35, y - r * 0.35), (r * 0.28).max(1.0), Color32::from_white_alpha(140));
    }
    // Captions and lights.
    let q = QUALITIES[(p[QUALITY].round().max(0.0) as usize).min(QUALITIES.len() - 1)];
    let what = if wavetables {
        "wavetables"
    } else if delay {
        "delay"
    } else {
        "grains"
    };
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, &format!("{q} · {what}"), accent);
    let seed_on = m[2] > 0.5;
    let led = pos2(rect.right() - 16.0, rect.top() + 12.0);
    painter.circle_filled(led, 4.0, if seed_on { accent } else { FAINT });
    if seed_on {
        painter.circle_filled(led, 8.0, accent.gamma_multiply(0.18));
    }
    caption(&painter, led - vec2(10.0, 0.0), Align2::RIGHT_CENTER, "seed", if seed_on { accent } else { DIM });
    if frozen {
        caption(&painter, pos2(rect.right() - 10.0, rect.bottom() - 6.0), Align2::RIGHT_BOTTOM, "frozen", ICE);
    }
    if !wavetables {
        caption(&painter, pos2(rect.left() + 10.0, rect.bottom() - 6.0), Align2::LEFT_BOTTOM, "older", DIM);
        caption(&painter, pos2(rect.right() - if frozen { 60.0 } else { 10.0 }, rect.bottom() - 6.0), Align2::RIGHT_BOTTOM, "now", DIM);
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_beads_screen_draws() {
        let ctx = egui::Context::default();
        let status = krabton_engine::EngineStatus::new();
        for source in [0.0, 1.0] {
            let mut p = krabton_engine::params::defaults(SPECS);
            p[SOURCE] = source;
            let mut width = 400.0;
            for _ in 0..3 {
                let _ = ctx.run(egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() }, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        beads_panel(ui, Id::new("b"), 1, &mut p, accent_now(), 220.0, None, &status, &mut width);
                    });
                });
            }
        }
    }
}
