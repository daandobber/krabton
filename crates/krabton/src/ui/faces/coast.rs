//! Coast as a west-coast instrument panel: how it plays, the modulation
//! oscillator, the principal oscillator with its folder (a screen draws the
//! wave folded as Timbre and Symmetry fold it), the contour that strikes or
//! opens the gate, the low pass gate itself (a screen shows the light, and
//! the afterglow Response leaves) and the output. With sliders on (the
//! Sliders pref) the same in long sliders, as on a west-coast panel, and a
//! screen of the contour's shape.

use super::{draw, screen_box, Ctl, Ctx, Face, Presets};
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use egui::{pos2, Id, Rect, Shape, Stroke, Ui};
use krabton_engine::dsp::coast;

pub(crate) fn face() -> Face {
    Face::new(53.0, 12.0)
        .section(0.0, 0.0, 7.4, 12.0, "Play")
        .preset(0.3, 0.4, 6.8, 1.4)
        .knob(0.3, 2.1, 3.4, 4.6, "Glide")
        .knob(3.7, 2.1, 3.4, 4.6, "Velocity")
        .knob(0.3, 7.0, 2.25, 4.6, "Octave")
        .knob(2.55, 7.0, 2.25, 4.6, "Voices")
        .ctl(4.8, 7.0, 2.3, 4.6, "Keyed", Ctl::Switch)
        .section(7.6, 0.0, 10.2, 12.0, "Modulation")
        .named(7.8, 0.3, 3.4, 5.4, "Mod Wave", Ctl::Selector, "Wave")
        .knob(11.2, 0.3, 3.3, 5.4, "Ratio")
        .named(14.5, 0.3, 3.1, 5.4, "Mod Freq", Ctl::Knob, "Free Hz")
        .named(7.8, 5.9, 5.0, 5.3, "FM", Ctl::Knob, "FM Index")
        .named(12.8, 5.9, 4.8, 5.3, "AM", Ctl::Knob, "AM Depth")
        .section(18.0, 0.0, 12.6, 12.0, "Oscillator")
        .named(18.2, 0.3, 3.4, 5.4, "Wave", Ctl::Selector, "Wave")
        .knob(21.6, 0.3, 4.6, 5.4, "Timbre")
        .screen(26.4, 0.6, 4.0, 4.8, "fold")
        .knob(18.2, 5.9, 4.1, 5.3, "Symmetry")
        .knob(22.3, 5.9, 4.1, 5.3, "Noise")
        .screen(26.4, 6.2, 4.0, 4.8, "wave")
        .section(30.8, 0.0, 9.2, 12.0, "Contour")
        .named(31.0, 0.3, 3.6, 5.4, "Contour", Ctl::Selector, "Mode")
        .knob(34.6, 0.3, 2.7, 5.4, "Attack")
        .knob(37.3, 0.3, 2.5, 5.4, "Decay")
        .named(31.0, 5.9, 4.4, 5.3, "Env Timbre", Ctl::Knob, "Timbre")
        .named(35.4, 5.9, 4.4, 5.3, "Env FM", Ctl::Knob, "FM")
        .section(40.2, 0.0, 8.6, 12.0, "Low Pass Gate")
        .named(40.4, 0.3, 3.4, 5.4, "Gate", Ctl::Selector, "Mode")
        .knob(43.8, 0.3, 4.8, 5.4, "Response")
        .knob(40.4, 5.9, 3.6, 5.3, "Brightness")
        .screen(44.2, 6.2, 4.4, 4.8, "gate")
        .section(49.0, 0.0, 4.0, 12.0, "Output")
        .knob(49.2, 0.3, 3.6, 5.4, "Volume")
        .knob(49.2, 5.9, 3.6, 5.3, "Width")
}

/// Coast in sliders: the selectors and switches stay, the rest slides.
pub(crate) fn face_sliders() -> Face {
    Face::new(53.0, 12.0)
        .section(0.0, 0.0, 7.4, 12.0, "Play")
        .preset(0.3, 0.4, 6.8, 1.4)
        .named(0.3, 2.0, 1.7, 9.2, "Glide", Ctl::Fader, "Glide")
        .named(2.0, 2.0, 1.7, 9.2, "Velocity", Ctl::Fader, "Vel")
        .named(3.7, 2.0, 1.7, 9.2, "Octave", Ctl::Fader, "Oct")
        .named(5.4, 2.0, 1.7, 9.2, "Voices", Ctl::Fader, "Voices")
        .section(7.6, 0.0, 10.2, 12.0, "Modulation")
        .named(7.8, 0.3, 3.4, 5.4, "Mod Wave", Ctl::Selector, "Wave")
        .ctl(7.8, 5.9, 3.4, 5.3, "Keyed", Ctl::Switch)
        .named(11.3, 0.3, 1.6, 10.8, "Ratio", Ctl::Fader, "Ratio")
        .named(12.9, 0.3, 1.6, 10.8, "Mod Freq", Ctl::Fader, "Free")
        .named(14.5, 0.3, 1.6, 10.8, "FM", Ctl::Fader, "FM")
        .named(16.1, 0.3, 1.6, 10.8, "AM", Ctl::Fader, "AM")
        .section(18.0, 0.0, 12.6, 12.0, "Oscillator")
        .named(18.2, 0.3, 3.4, 5.4, "Wave", Ctl::Selector, "Wave")
        .screen(18.2, 6.2, 3.4, 4.8, "wave")
        .named(21.8, 0.3, 1.6, 10.8, "Timbre", Ctl::Fader, "Timbre")
        .named(23.4, 0.3, 1.6, 10.8, "Symmetry", Ctl::Fader, "Sym")
        .named(25.0, 0.3, 1.6, 10.8, "Noise", Ctl::Fader, "Noise")
        .screen(26.8, 0.6, 3.6, 10.4, "fold")
        .section(30.8, 0.0, 10.4, 12.0, "Contour")
        .named(31.0, 0.3, 3.4, 5.4, "Contour", Ctl::Selector, "Mode")
        .screen(31.0, 6.2, 3.4, 4.8, "contour")
        .named(35.0, 0.3, 1.55, 10.8, "Attack", Ctl::Fader, "Rise")
        .named(36.55, 0.3, 1.55, 10.8, "Decay", Ctl::Fader, "Fall")
        .named(38.1, 0.3, 1.55, 10.8, "Env Timbre", Ctl::Fader, "Timbre")
        .named(39.65, 0.3, 1.5, 10.8, "Env FM", Ctl::Fader, "FM")
        .section(41.4, 0.0, 7.6, 12.0, "Low Pass Gate")
        .named(41.6, 0.3, 3.2, 5.4, "Gate", Ctl::Selector, "Mode")
        .screen(41.6, 6.2, 3.2, 4.8, "gate")
        .named(45.0, 0.3, 1.8, 10.8, "Response", Ctl::Fader, "Resp")
        .named(46.8, 0.3, 1.8, 10.8, "Brightness", Ctl::Fader, "Bright")
        .section(49.2, 0.0, 3.8, 12.0, "Output")
        .named(49.4, 0.3, 1.6, 10.8, "Volume", Ctl::Fader, "Vol")
        .named(51.0, 0.3, 1.6, 10.8, "Width", Ctl::Fader, "Width")
}

/// Coast's face (with knobs, or with sliders), its screens live. Returns
/// whether a value changed.
pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut [f32], live: &InstLive, height: f32, mv: Option<&mut ModView>) -> bool {
    let face = if crate::skin::sliders() { face_sliders() } else { face() };
    let mut cx = Ctx { id, keys: coast::SPECS, specs: coast::SPECS, params, mv, presets: Some(Presets { current: preset, names: coast::preset_names(), load: coast::preset }) };
    let changed = draw(ui, &face, height, &mut cx, &mut |ui, which, rect, p| {
        screen(ui, rect, which, p, live);
        false
    });
    if live.vals[0] > 0.0 {
        ui.ctx().request_repaint();
    }
    changed
}

/// The screens: the wave folded (and as it is folded now), its shape before
/// folding, and the gate's light with the afterglow it leaves.
fn screen(ui: &mut Ui, r: Rect, which: &str, p: &[f32], live: &InstLive) {
    let painter = ui.painter().clone();
    let u = r.height() / 10.6;
    screen_box(&painter, r, u);
    let ink = accent_now();
    let dim = shade(ink, 0.35);
    let inner = r.shrink(u * 0.7);
    let get = |i: usize| p.get(i).copied().unwrap_or(0.0);
    let base_wave = |kind: usize, ph: f32| match kind {
        0 => (std::f32::consts::TAU * ph).sin(),
        1 => 1.0 - 4.0 * (ph - 0.5).abs(),
        2 => 2.0 * ph - 1.0,
        _ => {
            if ph < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
    };
    let folded = |x: f32, t: f32| (0.5 * std::f32::consts::PI * (x * (1.0 + t * 7.0) + get(coast::SYMMETRY) * 0.8)).sin();
    let plot = |vals: &dyn Fn(f32) -> f32, color| {
        let pts: Vec<_> = (0..=96)
            .map(|k| {
                let f = k as f32 / 96.0;
                pos2(inner.left() + f * inner.width(), inner.center().y - vals(f) * inner.height() * 0.45)
            })
            .collect();
        painter.add(Shape::line(pts, Stroke::new(1.5_f32, color)));
    };
    let kind = (get(coast::WAVE).round().max(0.0) as usize).min(3);
    match which {
        "fold" => {
            // The fold as set, faint; as it is now (the contour opening it), bright.
            plot(&|f| folded(base_wave(kind, f), get(coast::TIMBRE)), dim);
            let now = if live.vals[0] > 0.0 { live.vals[2] } else { get(coast::TIMBRE) };
            plot(&|f| folded(base_wave(kind, f), now), ink);
        }
        "wave" => plot(&|f| base_wave(kind, f) * 0.9, ink),
        "contour" => {
            // Struck: at once and down; Transient: up and down; Sustained: up, held, down.
            let mode = get(coast::CONTOUR).round().max(0.0) as usize;
            let (a, d) = (get(coast::ATTACK).max(0.001), get(coast::DECAY).max(0.01));
            let hold = if mode == 2 { (a + d) * 0.6 } else { 0.0 };
            let a = if mode == 0 { 0.0 } else { a };
            let total = a + hold + d;
            let level = |t: f32| {
                if t < a {
                    t / a
                } else if t < a + hold {
                    1.0
                } else {
                    (-4.6 * (t - a - hold) / d).exp()
                }
            };
            let pts: Vec<_> = (0..=64)
                .map(|k| {
                    let t = k as f32 / 64.0 * total;
                    pos2(inner.left() + k as f32 / 64.0 * inner.width(), inner.bottom() - level(t) * inner.height())
                })
                .collect();
            painter.add(Shape::line(pts, Stroke::new(1.5_f32, ink)));
        }
        _ => {
            // The light: how far the gate is open now, and the afterglow it leaves.
            let open = live.vals[1].clamp(0.0, 1.0);
            let resp = get(coast::RESPONSE).max(0.01);
            let pts: Vec<_> = (0..=64)
                .map(|k| {
                    let t = k as f32 / 64.0 * 2.0;
                    let level = (-4.6 * t / resp).exp();
                    pos2(inner.left() + k as f32 / 64.0 * inner.width(), inner.bottom() - level * inner.height())
                })
                .collect();
            painter.add(Shape::line(pts, Stroke::new(1.5_f32, dim)));
            let bar = Rect::from_min_max(pos2(inner.right() - inner.width() * 0.12, inner.bottom() - open * inner.height()), inner.right_bottom());
            painter.rect_filled(bar, 1.0, ink);
            painter.circle_filled(pos2(inner.left() + u * 0.8, inner.top() + u * 0.8), u * 0.5, mix(dim, ink, open));
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_knob_of_coast_is_on_its_face() {
        super::super::check(&super::face(), krabton_engine::dsp::coast::SPECS);
        super::super::check(&super::face_sliders(), krabton_engine::dsp::coast::SPECS);
    }
}
