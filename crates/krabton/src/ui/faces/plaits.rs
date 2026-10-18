//! Plaits laid out as the module: the models in three banks of lit
//! buttons, the big harmonics knob with timbre and morph beside it (named
//! for what they do in the model), the small envelope amounts below them,
//! and a row of sockets along the bottom; the voice beside it.

use super::{draw, Ctl, Ctx, Face, Presets};
use crate::theme::*;
use crate::ui::modulation::ModView;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Ui};
use krabton_engine::dsp::macro_osc::{self, HARMONICS, MODEL};

pub(crate) fn face() -> Face {
    Face::new(36.0, 12.0)
        .section(0.0, 0.0, 10.0, 12.0, "Model")
        .preset(0.3, 0.4, 9.4, 1.4)
        .screen(0.3, 2.0, 9.4, 9.1, "models")
        .screened(&["Model"])
        .section(10.2, 0.0, 14.4, 12.0, "Oscillator")
        .named(10.4, 0.2, 5.4, 6.4, "Harmonics", Ctl::Knob, "Harmonics")
        .named(15.9, 0.4, 4.2, 5.2, "Timbre", Ctl::Knob, "Timbre")
        .named(20.1, 0.4, 4.2, 5.2, "Morph", Ctl::Knob, "Morph")
        .row(10.4, 6.8, 2.8, 2.5, &["Timbre Env", "Morph Env", "Pitch Env", "Decay", "LPG Colour"], Ctl::Knob)
        .jacks(10.4, 9.3, 1.75, 1.8, &["Model", "Timbre", "FM", "Harmo", "Morph", "Trig", "Level", "V/Oct"])
        .section(24.8, 0.0, 11.2, 12.0, "Voice")
        .ctl(25.0, 0.3, 2.8, 5.2, "Amp", Ctl::Switch)
        .row(27.8, 0.3, 2.0, 2.6, &["Attack", "Amp Decay", "Sustain", "Release"], Ctl::Knob)
        .row(27.8, 2.9, 2.0, 2.6, &["Velocity", "Voices", "Glide", "Transpose"], Ctl::Knob)
        .named(25.0, 5.8, 3.2, 5.3, "Out / Aux", Ctl::Knob, "Out · Aux")
        .knob(28.2, 5.8, 3.2, 5.3, "Spread")
        .knob(31.4, 5.6, 4.4, 5.5, "Volume")
}

/// The three banks of models, a lit button each.
const BANKS: [(&str, Color32); 3] = [("Synth", Color32::from_rgb(0x2f, 0xa8, 0x4f)), ("Classic", Color32::from_rgb(0xe0, 0x45, 0x34)), ("Noise & Drums", Color32::from_rgb(0xe8, 0xa0, 0x20))];

fn models(ui: &mut Ui, id: Id, rect: Rect, p: &mut [f32]) -> bool {
    let current = p[MODEL].round() as usize;
    let mut changed = false;
    let bw = rect.width() / 3.0;
    let bh = (rect.height() - 16.0) / 8.0;
    let painter = ui.painter().clone();
    let (well, well_ink) = crate::skin::well();
    for (b, (title, color)) in BANKS.iter().enumerate() {
        let x = rect.left() + b as f32 * bw;
        painter.text(pos2(x + bw * 0.5, rect.top() + 6.0), Align2::CENTER_CENTER, title.to_lowercase(), FontId::proportional(10.5), crate::skin::ink());
        for k in 0..8 {
            let m = b * 8 + k;
            let r = Rect::from_min_size(pos2(x + 2.0, rect.top() + 14.0 + k as f32 * bh), vec2(bw - 4.0, bh - 2.0));
            let resp = ui.interact(r, id.with(m), Sense::click());
            if resp.clicked() && current != m {
                p[MODEL] = m as f32;
                changed = true;
            }
            let on = p[MODEL].round() as usize == m;
            painter.rect_filled(r, 2.0, if resp.hovered() { mix(well, Color32::WHITE, 0.12) } else { well });
            let led = pos2(r.left() + 6.0, r.center().y);
            if on {
                painter.circle_filled(led, 5.0, color.gamma_multiply(0.35));
            }
            painter.circle_filled(led, 2.6, if on { *color } else { Color32::from_black_alpha(150) });
            let name = macro_osc::MODELS[m];
            let font = FontId::proportional((bh * 0.55).clamp(8.0, 11.0));
            painter.with_clip_rect(r).text(pos2(led.x + 6.0, r.center().y), Align2::LEFT_CENTER, name, font, if on { well_ink } else { mix(well_ink, well, 0.35) });
            resp.on_hover_text(name);
        }
    }
    changed
}

pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut [f32], height: f32, mv: Option<&mut ModView>) -> bool {
    let face = face();
    // The three macro knobs are named after what they do in this model.
    let model = (params[MODEL].round().max(0.0) as usize).min(macro_osc::MODEL_KNOBS.len() - 1);
    let mut specs = macro_osc::SPECS.to_vec();
    for k in 0..3 {
        specs[HARMONICS + k].name = macro_osc::MODEL_KNOBS[model][k];
    }
    let mut cx = Ctx { id, keys: macro_osc::SPECS, specs: &specs, params, mv, presets: Some(Presets { current: preset, names: macro_osc::preset_names(), load: macro_osc::preset }) };
    draw(ui, &face, height, &mut cx, &mut |ui, _, rect, p| models(ui, id.with("models"), rect, p))
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_knob_of_plaits_is_on_its_face() {
        super::super::check(&super::face(), krabton_engine::dsp::macro_osc::SPECS);
    }
}
