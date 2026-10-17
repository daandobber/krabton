//! Oidos's screen: the partials of the note playing (or of middle C), each
//! a line at its pitch as tall as it is loud, lit where the filter lets it
//! through (dim where it shuts it out), brighter the longer it rings; the
//! filter's edges drawn over them. Click for the next seed (right-click:
//! the one before): the same settings, another sound. Drag sideways for
//! Overtones (how high the modes reach), up and down for Harmonicity.

use super::inst_screen::{caption, InstLive, SCREEN_BG};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Stroke, Ui};
use krabton_engine::dsp::oidos::{self, FILTER_HIGH, FILTER_LOW, HARMONICITY, OVERTONES, SEED, SLOPE_HIGH, SLOPE_LOW, TRANSPOSE};

const DIM: Color32 = Color32::from_rgb(0x4a, 0x4e, 0x58);
const FAINT: Color32 = Color32::from_rgb(0x1e, 0x21, 0x28);

fn get(p: &[f32], i: usize) -> f32 {
    p.get(i).copied().unwrap_or(oidos::SPECS[i].default)
}

fn set(p: &mut [f32], i: usize, v: f32) -> bool {
    if p.get(i).is_some_and(|x| *x != v) {
        p[i] = v;
        true
    } else {
        false
    }
}

pub(crate) fn oidos_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    let w = 420.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let painter = ui.painter_at(rect);
    rect_fill(&painter, rect, 6.0, SCREEN_BG);
    rect_line(&painter, rect, 6.0, 1.0, Color32::from_rgb(0x26, 0x29, 0x31));
    let accent = accent_now();
    let mut changed = false;
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, "Oidos", accent);
    let playing = live.vals[6] > 0.0;
    let note = if playing { live.vals[2] } else { 60.0 } + get(p, TRANSPOSE).round();
    let parts = oidos::partials(p, note, 48000.0, true);
    let info = if playing {
        format!("SEED {}  ·  {} VOICES  ·  {} PARTIALS", get(p, SEED).round(), live.vals[0].round(), live.vals[1].round())
    } else {
        format!("SEED {}  ·  {} PARTIALS A NOTE", get(p, SEED).round(), parts.len())
    };
    painter.text(rect.right_top() + vec2(-10.0, 7.0), Align2::RIGHT_TOP, info, FontId::monospace(9.0), TEXT_DIM);
    // The spectrum: from two octaves below the note to the top of hearing.
    let sr = Rect::from_min_max(pos2(rect.left() + 10.0, rect.top() + 26.0), pos2(rect.right() - 10.0, rect.bottom() - 22.0));
    painter.rect_filled(sr, 3.0, FAINT);
    let (lo, hi) = (note - 24.0, 136.0f32.max(note + 24.0));
    let x_of = |tone: f32| sr.left() + ((tone - lo) / (hi - lo)).clamp(0.0, 1.0) * sr.width();
    // The octaves of the note.
    for k in -2..9 {
        let t = note + 12.0 * k as f32;
        if t > lo && t < hi {
            let x = x_of(t);
            painter.line_segment([pos2(x, sr.top()), pos2(x, sr.bottom())], Stroke::new(1.0_f32, if k == 0 { DIM } else { Color32::from_rgb(0x2a, 0x2e, 0x36) }));
        }
    }
    let peak = parts.iter().map(|q| q.amp.abs()).fold(1e-9, f64::max);
    let nyquist = 69.0 + 12.0 * (0.45 * 48000.0f32 / 440.0).log2();
    for q in &parts {
        let db = 20.0 * (q.amp.abs() / peak).max(1e-4).log10();
        let h = ((db + 60.0) / 60.0).clamp(0.0, 1.0) as f32 * sr.height();
        if h < 1.0 {
            continue;
        }
        let open = q.low.min(q.high).clamp(0.0, 1.0) as f32;
        // How long it rings (half its level in a tenth of a second: dark; in ten or never: bright).
        let half = if q.keep >= 1.0 { f64::INFINITY } else { 0.5f64.ln() / q.keep.ln() / 48000.0 };
        let ring = ((half.log10() + 1.0) / 2.0).clamp(0.0, 1.0) as f32;
        let color = if q.tone >= nyquist || open <= 0.0 { DIM } else { accent.gamma_multiply(0.35 + 0.65 * open * (0.4 + 0.6 * ring)) };
        let x = x_of(q.tone);
        painter.line_segment([pos2(x, sr.bottom()), pos2(x, sr.bottom() - h)], Stroke::new(1.0_f32, color));
    }
    // The filter's edges (where they start to let through, and where fully).
    let edge = |limit: usize, slope: usize, low_side: bool| {
        let f = (get(p, limit) * 2.0 - 1.0) * 120.0 + note;
        let s = (1.0 - get(p, slope)).powi(3).max(1e-3);
        let full = if low_side { f + 1.0 / s } else { f - 1.0 / s };
        (f, full)
    };
    for (low_side, (start, full)) in [(true, edge(FILTER_LOW, SLOPE_LOW, true)), (false, edge(FILTER_HIGH, SLOPE_HIGH, false))] {
        if (low_side && start <= lo) || (!low_side && start >= hi) {
            continue;
        }
        let (a, b) = (x_of(start), x_of(full));
        let shade = if low_side { Rect::from_min_max(pos2(sr.left(), sr.top()), pos2(a, sr.bottom())) } else { Rect::from_min_max(pos2(a, sr.top()), pos2(sr.right(), sr.bottom())) };
        painter.rect_filled(shade, 0.0, Color32::from_black_alpha(90));
        painter.line_segment([pos2(a, sr.bottom()), pos2(b, sr.top())], Stroke::new(1.5_f32, YELLOW));
    }
    // Under it: what the drags do.
    painter.text(pos2(sr.left(), rect.bottom() - 6.0), Align2::LEFT_BOTTOM, format!("OVERTONES {}  ·  HARMONICITY {:+.2}", get(p, OVERTONES).round(), get(p, HARMONICITY)), FontId::monospace(9.0), TEXT_DIM);
    painter.text(pos2(sr.right(), rect.bottom() - 6.0), Align2::RIGHT_BOTTOM, "click: next seed", FontId::proportional(9.5), TEXT_DIM);
    let resp = ui.interact(sr, id.with("spectrum"), Sense::click_and_drag());
    if resp.clicked() {
        changed |= set(p, SEED, (get(p, SEED).round() + 1.0).rem_euclid(101.0));
    } else if resp.secondary_clicked() {
        changed |= set(p, SEED, (get(p, SEED).round() - 1.0).rem_euclid(101.0));
    }
    if resp.dragged() {
        let d = resp.drag_delta();
        let o = (get(p, OVERTONES) + d.x * 0.25).clamp(0.0, 100.0);
        changed |= set(p, OVERTONES, o);
        let h = (get(p, HARMONICITY) - d.y / sr.height() * 2.0).clamp(-1.0, 1.0);
        changed |= set(p, HARMONICITY, h);
    }
    if playing {
        ui.ctx().request_repaint();
    }
    changed
}

const YELLOW: Color32 = Color32::from_rgb(0xff, 0xd4, 0x3b);

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, RawInput};

    #[test]
    fn the_oidos_screen_draws_every_preset() {
        let ctx = egui::Context::default();
        for playing in [false, true] {
            let mut vals = [0.0; krabton_engine::engine::INST_VALS];
            if playing {
                vals[0] = 2.0;
                vals[1] = 700.0;
                vals[2] = 64.0;
                vals[6] = 1.0;
            }
            let live = InstLive { wave: vec![0.0; 64], notes: 1 << 64, vals };
            for name in oidos::preset_names() {
                let mut p = oidos::preset(name);
                let _ = ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() }, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        oidos_screen(ui, Id::new("o"), &mut p, &live, 200.0);
                    });
                });
            }
        }
    }
}
