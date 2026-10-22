//! Crimson as a red virtual analog: its patch, the two oscillators (screens
//! draw their waves; the wavetables in displays), the mixer in faders, the
//! filters with a screen of their curve (and where the envelope has the
//! cutoff now), both envelopes in faders, the LFOs, unison, how it plays,
//! and the effects. Choices of many kinds sit in displays.

use super::{draw, screen_box, Ctl, Ctx, Face, Presets};
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use egui::{pos2, Id, Rect, Shape, Stroke, Ui};
use krabton_engine::dsp::crimson;

pub(crate) fn face() -> Face {
    Face::new(80.2, 12.0)
        .section(0.0, 0.0, 6.2, 12.0, "Patch")
        .preset(0.3, 0.4, 5.6, 1.4)
        .knob(0.3, 2.3, 2.8, 4.4, "Voices")
        .knob(3.1, 2.3, 2.8, 4.4, "Octave")
        .knob(0.3, 6.9, 5.6, 4.3, "Volume")
        .section(6.4, 0.0, 8.2, 12.0, "Osc 1")
        .named(6.6, 0.3, 2.6, 3.55, "Osc 1:Mode", Ctl::Switch, "Mode")
        .named(9.2, 0.3, 2.6, 3.55, "Osc 1:Shape", Ctl::Knob, "Shape")
        .named(11.8, 0.3, 2.6, 3.55, "Osc 1:Pulse Width", Ctl::Knob, "PW")
        .knob(6.6, 3.95, 2.6, 3.55, "Density")
        .named(9.2, 3.95, 2.6, 3.55, "Osc 1:Spread", Ctl::Knob, "Spread")
        .screen(11.9, 4.15, 2.4, 3.2, "osc1")
        .named(6.6, 7.6, 7.8, 3.55, "Osc 1:Table", Ctl::Lcd, "Table")
        .section(14.8, 0.0, 10.6, 12.0, "Osc 2")
        .named(15.0, 0.3, 2.6, 3.55, "Osc 2:Mode", Ctl::Switch, "Mode")
        .named(17.6, 0.3, 2.6, 3.55, "Osc 2:Shape", Ctl::Knob, "Shape")
        .named(20.2, 0.3, 2.6, 3.55, "Osc 2:Pulse Width", Ctl::Knob, "PW")
        .named(22.8, 0.3, 2.6, 3.55, "Semitone", Ctl::Knob, "Semi")
        .named(15.0, 3.95, 2.6, 3.55, "Osc 2:Detune", Ctl::Knob, "Detune")
        .knob(17.6, 3.95, 2.6, 3.55, "FM")
        .ctl(20.2, 3.95, 2.6, 3.55, "Sync", Ctl::Switch)
        .named(22.8, 3.95, 2.6, 3.55, "Env > Pitch", Ctl::Knob, "Env Pitch")
        .named(15.0, 7.6, 7.8, 3.55, "Osc 2:Table", Ctl::Lcd, "Table")
        .screen(22.9, 7.8, 2.4, 3.2, "osc2")
        .section(25.6, 0.0, 6.4, 12.0, "Mixer")
        .named(25.8, 0.3, 1.5, 10.9, "Balance", Ctl::Fader, "1 / 2")
        .named(27.3, 0.3, 1.5, 10.9, "Sub", Ctl::Fader, "Sub")
        .named(28.8, 0.3, 1.5, 10.9, "Noise", Ctl::Fader, "Noise")
        .named(30.3, 0.3, 1.5, 10.9, "Ring", Ctl::Fader, "Ring")
        .section(32.2, 0.0, 13.4, 12.0, "Filter")
        .named(32.4, 0.3, 5.2, 3.55, "Filter:Mode", Ctl::Lcd, "Mode")
        .named(37.6, 0.3, 5.2, 3.55, "Routing", Ctl::Lcd, "Routing")
        .named(42.8, 0.3, 2.6, 3.55, "Mode 2", Ctl::Switch, "Mode 2")
        .knob(32.4, 3.95, 2.6, 3.55, "Cutoff")
        .named(35.0, 3.95, 2.6, 3.55, "Resonance", Ctl::Knob, "Res")
        .named(37.6, 3.95, 2.6, 3.55, "Env Amount", Ctl::Knob, "Env")
        .named(40.2, 3.95, 2.6, 3.55, "Key Track", Ctl::Knob, "Key")
        .named(42.8, 3.95, 2.6, 3.55, "Cutoff 2", Ctl::Knob, "Cutoff 2")
        .named(32.4, 7.6, 5.2, 3.55, "Saturation", Ctl::Lcd, "Saturation")
        .knob(37.6, 7.6, 2.6, 3.55, "Drive")
        .screen(40.3, 7.8, 5.0, 3.3, "filter")
        .section(45.8, 0.0, 6.4, 12.0, "Filter Env")
        .named(46.0, 0.3, 1.5, 10.9, "Filter Env:Attack", Ctl::Fader, "A")
        .named(47.5, 0.3, 1.5, 10.9, "Filter Env:Decay", Ctl::Fader, "D")
        .named(49.0, 0.3, 1.5, 10.9, "Filter Env:Sustain", Ctl::Fader, "S")
        .named(50.5, 0.3, 1.5, 10.9, "Filter Env:Release", Ctl::Fader, "R")
        .section(52.4, 0.0, 6.4, 12.0, "Amp Env")
        .named(52.6, 0.3, 1.5, 10.9, "Amp Env:Attack", Ctl::Fader, "A")
        .named(54.1, 0.3, 1.5, 10.9, "Amp Env:Decay", Ctl::Fader, "D")
        .named(55.6, 0.3, 1.5, 10.9, "Amp Env:Sustain", Ctl::Fader, "S")
        .named(57.1, 0.3, 1.5, 10.9, "Amp Env:Release", Ctl::Fader, "R")
        .section(59.0, 0.0, 5.6, 12.0, "LFO 1")
        .named(59.2, 0.3, 5.2, 3.55, "LFO 1:Shape", Ctl::Lcd, "Shape")
        .named(59.2, 3.95, 2.6, 3.55, "LFO 1:Rate", Ctl::Knob, "Rate")
        .named(61.8, 3.95, 2.6, 3.55, "To Pitch", Ctl::Knob, "Pitch")
        .named(59.2, 7.6, 2.6, 3.55, "To Cutoff", Ctl::Knob, "Cutoff")
        .named(61.8, 7.6, 2.6, 3.55, "To Shape", Ctl::Knob, "Shape")
        .section(64.8, 0.0, 3.0, 12.0, "LFO 2")
        .named(65.0, 0.3, 2.6, 3.55, "LFO 2:Rate", Ctl::Knob, "Rate")
        .named(65.0, 3.95, 2.6, 3.55, "To Pan", Ctl::Knob, "Pan")
        .named(65.0, 7.6, 2.6, 3.55, "To Amp", Ctl::Knob, "Amp")
        .section(68.0, 0.0, 3.0, 12.0, "Unison")
        .knob(68.2, 0.3, 2.6, 3.55, "Unison")
        .named(68.2, 3.95, 2.6, 3.55, "Unison:Detune", Ctl::Knob, "Detune")
        .named(68.2, 7.6, 2.6, 3.55, "Unison:Spread", Ctl::Knob, "Spread")
        .section(71.2, 0.0, 3.0, 12.0, "Play")
        .knob(71.4, 0.3, 2.6, 3.55, "Glide")
        .knob(71.4, 3.95, 2.6, 3.55, "Velocity")
        .named(71.4, 7.6, 2.6, 3.55, "Vel > Cutoff", Ctl::Knob, "Vel Cut")
        .section(74.4, 0.0, 5.6, 12.0, "Effects")
        .knob(74.6, 0.3, 2.6, 3.55, "Chorus")
        .knob(77.2, 0.3, 2.6, 3.55, "Reverb")
        .knob(74.6, 3.95, 2.6, 3.55, "Delay")
        .named(77.2, 3.95, 2.6, 3.55, "Delay Time", Ctl::Knob, "Time")
        .named(74.6, 7.6, 2.6, 3.55, "Feedback", Ctl::Knob, "Feedback")
        .knob(77.2, 7.6, 2.6, 3.55, "Boost")
}

/// Crimson's face, its screens live. Returns whether a value changed.
pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut [f32], live: &InstLive, height: f32, mv: Option<&mut ModView>) -> bool {
    let face = face();
    let mut cx = Ctx { id, keys: crimson::SPECS, specs: crimson::SPECS, params, mv, presets: Some(Presets { current: preset, names: crimson::preset_names(), load: crimson::preset }) };
    let changed = draw(ui, &face, height, &mut cx, &mut |ui, which, rect, p| {
        screen(ui, rect, which, p, live);
        false
    });
    if live.vals[0] > 0.0 {
        ui.ctx().request_repaint();
    }
    changed
}

/// How loud a filter of `mode` passes `x` times its cutoff (a two-pole with
/// damping `k`), as the engine's state variable filter does.
fn response(mode: usize, x: f32, k: f32) -> f32 {
    let x2 = x * x;
    let den = ((1.0 - x2) * (1.0 - x2) + (k * x) * (k * x)).sqrt().max(1e-6);
    match mode {
        2 => x2 / den,
        3 => k * x / den,
        4 => (1.0 - x2).abs() / den,
        _ => 1.0 / den,
    }
}

/// The screens: each oscillator's wave, and the filter's curve.
fn screen(ui: &mut Ui, r: Rect, which: &str, p: &[f32], live: &InstLive) {
    let painter = ui.painter().clone();
    let u = r.height() / 10.6;
    screen_box(&painter, r, u);
    let ink = accent_now();
    let dim = shade(ink, 0.35);
    let inner = r.shrink((u * 0.7).max(2.0));
    let get = |i: usize| p.get(i).copied().unwrap_or(0.0);
    let plot = |vals: &dyn Fn(f32) -> f32, color, n: usize| {
        let pts: Vec<_> = (0..=n)
            .map(|k| {
                let f = k as f32 / n as f32;
                pos2(inner.left() + f * inner.width(), inner.center().y - vals(f).clamp(-1.2, 1.2) * inner.height() * 0.42)
            })
            .collect();
        painter.add(Shape::line(pts, Stroke::new(1.5_f32, color)));
    };
    match which {
        "osc1" | "osc2" => {
            let osc = if which == "osc1" { 1 } else { 2 };
            plot(&|f| crimson::wave_at(p, osc, f), ink, 96);
        }
        _ => {
            // The filter's curve over ten octaves: as set, and as the envelope has it now.
            let mode = get(crimson::FILTER_MODE).round().max(0.0) as usize;
            let k = 2.0 - 1.97 * get(crimson::RESONANCE).clamp(0.0, 1.0);
            let curve = |cutoff: f32| {
                move |f: f32| {
                    let hz = 20.0 * 2f32.powf(f * 10.0);
                    let x = hz / cutoff.max(20.0);
                    let mut g = response(mode, x, k);
                    if mode == 0 {
                        g *= response(0, x, 2.0 - 1.97 * get(crimson::RESONANCE).clamp(0.0, 1.0) * 0.3);
                    }
                    let db = 20.0 * g.max(1e-5).log10();
                    ((db + 36.0) / 54.0).clamp(0.0, 1.0) * 2.0 - 1.0
                }
            };
            plot(&curve(get(crimson::CUTOFF)), dim, 80);
            if live.vals[0] > 0.0 {
                let now = 20.0 * 2f32.powf(live.vals[2] * 10.0);
                plot(&curve(now), ink, 80);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_knob_of_crimson_is_on_its_face() {
        super::super::check(&super::face(), krabton_engine::dsp::crimson::SPECS);
    }
}
