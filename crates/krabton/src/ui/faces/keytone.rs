//! Keytone as a grey toy keyboard: its sounds as big numbered buttons, a
//! display with the sound's 8-bit wave in its steps and a light for every
//! voice, the speaker's grille, the envelope's buttons and switches,
//! vibrato and portamento, the chip's rate and bits, and the keys on
//! faders. A sample loaded (or dropped on it) goes in the user slot.

use super::{draw, screen_box, Ctl, Ctx, Face, Presets};
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, RichText, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::keytone as kt;
use krabton_engine::sample::SampleRef;

pub(crate) fn face() -> Face {
    Face::new(62.5, 12.0)
        .screened(&["Sound"])
        .section(0.0, 0.0, 10.0, 12.0, "Keytone")
        .preset(0.3, 0.4, 9.4, 1.4)
        .screen(0.3, 2.2, 9.4, 6.0, "lcd")
        .screen(0.3, 8.6, 9.4, 2.5, "grille")
        .section(10.2, 0.0, 21.6, 12.0, "Sounds")
        .screen(10.4, 0.3, 21.2, 10.9, "sounds")
        .section(32.0, 0.0, 7.0, 12.0, "Envelope")
        .ctl(32.2, 0.3, 3.6, 10.9, "Envelope", Ctl::Buttons)
        .ctl(35.8, 0.3, 3.0, 5.3, "Sustain", Ctl::Switch)
        .ctl(35.8, 5.9, 3.0, 5.3, "Velocity", Ctl::Switch)
        .section(39.2, 0.0, 10.1, 12.0, "Effects")
        .ctl(39.4, 0.3, 2.8, 5.3, "Vibrato", Ctl::Switch)
        .ctl(39.4, 5.9, 2.8, 5.3, "Portamento", Ctl::Switch)
        .named(42.2, 0.3, 2.3, 10.9, "Vib Depth", Ctl::Fader, "Depth")
        .named(44.5, 0.3, 2.3, 10.9, "Vib Rate", Ctl::Fader, "Speed")
        .named(46.8, 0.3, 2.3, 10.9, "Glide", Ctl::Fader, "Glide")
        .section(49.5, 0.0, 5.5, 12.0, "Chip")
        .ctl(49.7, 0.3, 2.8, 5.3, "Rate", Ctl::Switch)
        .ctl(49.7, 5.9, 2.8, 5.3, "Bits", Ctl::Switch)
        .ctl(52.5, 0.3, 2.3, 10.9, "Tone", Ctl::Fader)
        .section(55.2, 0.0, 7.3, 12.0, "Keys")
        .named(55.4, 0.3, 2.3, 10.9, "Octave", Ctl::Fader, "Oct")
        .ctl(57.7, 0.3, 2.3, 10.9, "Tune", Ctl::Fader)
        .named(60.0, 0.3, 2.3, 10.9, "Volume", Ctl::Fader, "Vol")
}

/// Keytone's panel: a row to load a sample into the user slot, then its
/// face (or, in the plain look, its presets and a grid). Returns a message
/// for the player (a sample that would not load, say).
#[allow(clippy::too_many_arguments)]
pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut Vec<f32>, sample: &mut Option<SampleRef>, live: &InstLive, height: f32, mv: Option<&mut ModView>) -> Option<String> {
    if params.len() < kt::SPECS.len() {
        return None;
    }
    kt::warm();
    let mut said = None;
    let mut load = |path: &std::path::Path, sample: &mut Option<SampleRef>, params: &mut Vec<f32>| match SampleRef::load(path) {
        Ok(s) => {
            *sample = Some(s);
            params[kt::SOUND] = kt::SAMPLE_SLOT as f32;
        }
        Err(e) => said = Some(format!("Could not load the sample: {e}")),
    };
    let skinned = crate::skin::current().is_some();
    ui.horizontal(|ui| {
        if !skinned {
            ui.label(RichText::new("Preset").small().color(crate::skin::ink_dim()));
            egui::ComboBox::from_id_salt(id.with("preset")).width(130.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
                for name in kt::preset_names() {
                    if ui.selectable_label(preset == name, *name).clicked() {
                        *preset = name.to_string();
                        *params = kt::preset(name);
                    }
                }
            });
        }
        if ui.button("Load sample...").on_hover_text("A short WAV, FLAC, MP3 or AIFF for the user slot (Sound: Sample), crushed to 8 bits at the toy's rate as it plays (or drop one on the keyboard)").clicked() {
            if let Some(path) = crate::import::audio_file_dialog().pick_file() {
                load(&path, sample, params);
            }
        }
        match sample.as_ref() {
            Some(s) => {
                ui.label(RichText::new(format!("{}  {:.1} s", s.name, s.duration())).color(accent_now()));
                if ui.small_button("x").on_hover_text("Empty the user slot").clicked() {
                    *sample = None;
                }
            }
            None => {
                ui.label(RichText::new("the user slot is empty").small().color(crate::skin::ink_dim()));
            }
        }
    });
    let h = (height - 28.0).max(120.0);
    let resp = if skinned {
        let face = face();
        let name = sample.as_ref().map(|s| s.name.clone());
        let mut cx = Ctx { id, keys: kt::SPECS, specs: kt::SPECS, params, mv, presets: Some(Presets { current: preset, names: kt::preset_names(), load: kt::preset }) };
        ui.scope(|ui| {
            draw(ui, &face, h, &mut cx, &mut |ui, which, rect, p| screen(ui, id, which, rect, p, live, name.as_deref()));
        })
        .response
    } else {
        ui.scope(|ui| {
            crate::ui::devices::param_grid_mod(ui, id.with("grid"), kt::SPECS, params, accent_now(), h, mv);
        })
        .response
    };
    // A sample from the browser goes in the user slot.
    if let Some(item) = drop_payload::<crate::app::BrowserItem>(&resp) {
        if let crate::app::BrowserItem::Sample(path) = &*item {
            load(path, sample, params);
        }
    }
    if live.vals[0] > 0.0 {
        ui.ctx().request_repaint();
    }
    said
}

/// The screens: the sounds' buttons, the display, the speaker's grille.
#[allow(clippy::too_many_arguments)]
fn screen(ui: &mut Ui, id: Id, which: &str, r: Rect, p: &mut [f32], live: &InstLive, sample: Option<&str>) -> bool {
    let painter = ui.painter().clone();
    let u = r.height() / 10.6;
    let sound = (p[kt::SOUND].round().max(0.0) as usize).min(kt::SOUNDS.len() - 1);
    match which {
        "sounds" => {
            // Three rows of buttons, the number over the name.
            let per_row = 6;
            let rows = kt::SOUNDS.len().div_ceil(per_row);
            let (bw, bh) = (r.width() / per_row as f32, r.height() / rows as f32);
            let (well, well_ink) = crate::skin::well();
            let mut changed = false;
            for (k, name) in kt::SOUNDS.iter().enumerate() {
                let cell = Rect::from_min_size(pos2(r.left() + (k % per_row) as f32 * bw, r.top() + (k / per_row) as f32 * bh), vec2(bw, bh)).shrink(3.0);
                let resp = ui.interact(cell, id.with(("sound", k)), Sense::click());
                let on = k == sound;
                let fill = if on {
                    accent_now()
                } else if resp.hovered() {
                    mix(well, Color32::WHITE, 0.14)
                } else {
                    well
                };
                painter.rect_filled(cell.translate(vec2(0.0, 2.0)), 5.0, Color32::from_black_alpha(110));
                painter.rect_filled(cell, 5.0, fill);
                painter.rect_filled(Rect::from_min_size(cell.min + vec2(3.0, 2.0), vec2(cell.width() - 6.0, cell.height() * 0.18)), 3.0, Color32::from_white_alpha(18));
                let ink = if on { text_on(fill) } else { well_ink };
                let label = if k == kt::SAMPLE_SLOT { sample.unwrap_or("Sample") } else { name };
                let num_font = FontId::proportional((cell.height() * 0.3).clamp(9.0, 18.0));
                painter.text(pos2(cell.left() + 6.0, cell.top() + 4.0), Align2::LEFT_TOP, format!("{:02}", k + 1), num_font, if on { ink } else { mix(ink, fill, 0.35) });
                let text = crate::skin::label_text(label);
                let f = super::fit(&painter, &text, FontId::proportional((cell.height() * 0.24).clamp(8.0, 14.0)), cell.width() - 10.0);
                painter.text(pos2(cell.center().x, cell.bottom() - cell.height() * 0.3), Align2::CENTER_CENTER, text, f, ink);
                let tip = if k == kt::SAMPLE_SLOT && sample.is_none() { "The user slot: load a sample first".to_string() } else { format!("Sound {}: {label}", k + 1) };
                if resp.on_hover_text(tip).clicked() && !on {
                    p[kt::SOUND] = k as f32;
                    changed = true;
                }
            }
            changed
        }
        "lcd" => {
            screen_box(&painter, r, u);
            let ink = accent_now();
            let inner = r.shrink((u * 0.7).max(3.0));
            let title = format!("{:02} {}", sound + 1, if sound == kt::SAMPLE_SLOT { sample.unwrap_or("SAMPLE") } else { kt::SOUNDS[sound] }).to_uppercase();
            let f = super::fit(&painter, &title, crate::skin::Font::Lcd.id((inner.height() * 0.17).clamp(9.0, 16.0)), inner.width());
            painter.text(inner.left_top(), Align2::LEFT_TOP, title, f, ink);
            let rate = kt::RATES[(p[kt::RATE].round().max(0.0) as usize).min(kt::RATES.len() - 1)];
            let bits = kt::BIT_DEPTHS[(p[kt::BITS].round().max(0.0) as usize).min(kt::BIT_DEPTHS.len() - 1)];
            let small_font = FontId::proportional((inner.height() * 0.11).clamp(8.0, 11.0));
            painter.text(inner.right_top() + vec2(0.0, inner.height() * 0.2), Align2::RIGHT_TOP, format!("{rate}  {bits} bit"), small_font, shade(ink, 0.6));
            // The wave as the chip holds it: a step for every sample.
            let plot = Rect::from_min_max(pos2(inner.left(), inner.top() + inner.height() * 0.36), pos2(inner.right(), inner.bottom() - inner.height() * 0.18));
            if let Some(w) = kt::wave(sound).filter(|_| sound != kt::SAMPLE_SLOT) {
                let n = w.len().max(1);
                let mut pts = Vec::with_capacity(2 * n + 1);
                for (i, &b) in w.iter().enumerate() {
                    let y = plot.center().y - b as f32 / 127.0 * plot.height() * 0.5;
                    pts.push(pos2(plot.left() + i as f32 / n as f32 * plot.width(), y));
                    pts.push(pos2(plot.left() + (i + 1) as f32 / n as f32 * plot.width(), y));
                }
                painter.add(Shape::line(pts, Stroke::new(1.5_f32, ink)));
            }
            // A light for each of the eight voices.
            let lights = Rect::from_min_max(pos2(inner.left(), inner.bottom() - inner.height() * 0.12), inner.max);
            let d = lights.width() / kt::VOICES as f32;
            for k in 0..kt::VOICES {
                let on = live.vals[2 + k] >= 0.0 && live.vals[0] > 0.0;
                let c = pos2(lights.left() + (k as f32 + 0.5) * d, lights.center().y);
                painter.circle_filled(c, (lights.height() * 0.4).clamp(2.0, 4.0), if on { ink } else { shade(ink, 0.25) });
            }
            false
        }
        _ => {
            // The speaker: slots that glow a little as it plays.
            let level = live.vals[1].clamp(0.0, 1.0);
            let slots = 5;
            let sh = r.height() / (slots as f32 * 1.7);
            for k in 0..slots {
                let y = r.top() + r.height() * (k as f32 + 0.5) / slots as f32;
                let slot = Rect::from_center_size(pos2(r.center().x, y), vec2(r.width() * 0.9, sh));
                painter.rect_filled(slot, sh * 0.5, mix(Color32::from_rgb(0x1a, 0x1b, 0x1d), accent_now(), level * 0.5));
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_control_of_keytone_is_on_its_face() {
        super::super::check(&super::face(), krabton_engine::dsp::keytone::SPECS);
    }
}
