//! Airway's panel: the preset, how it plays (Vowels, Words, Creature), the
//! body and the voices along the top; for Words the lyric, its syllables
//! as chips (the one sung last lit) and the pronouncing dictionary; for a
//! creature its calls; then every setting as a knob.

use super::inst_screen::InstLive;
use super::modulation::ModView;
use crate::theme::*;
use egui::{Id, RichText, Ui};
use krabton_engine::dsp::airway::{self as aw, PlansRef};
use std::sync::{Arc, Mutex};

/// The dictionary's download, when one is going.
type Fetch = Arc<Mutex<Option<Result<(), String>>>>;

/// A choice's name in the panel's ink (readable on its paper when not picked).
fn ink(name: &str) -> RichText {
    RichText::new(name).color(crate::skin::ink())
}

/// Airway's panel. Returns a message for the player.
#[allow(clippy::too_many_arguments)]
pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut Vec<f32>, text: &mut String, plans: &mut PlansRef, live: &InstLive, height: f32, mv: Option<&mut ModView>) -> Option<String> {
    if params.len() < aw::SPECS.len() {
        return None;
    }
    let mut said = None;
    let body_was = params[aw::BODY].round() as u8;
    let accent = accent_now();
    ui.horizontal(|ui| {
        ui.label(RichText::new("Preset").small().color(crate::skin::ink_dim()));
        egui::ComboBox::from_id_salt(id.with("preset")).width(130.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
            for name in aw::preset_names() {
                if ui.selectable_label(preset == name, *name).clicked() {
                    *preset = name.to_string();
                    *params = aw::preset(name);
                }
            }
        });
        for (k, name) in aw::MODES.iter().enumerate() {
            let tip = ["Each key sings a vowel at its pitch, the mouth set by the knobs", "Each key sings the next syllable of the lyric", "Each key a creature's call at its pitch"][k];
            if ui.selectable_label(params[aw::MODE].round() as usize == k, ink(name)).on_hover_text(tip).clicked() {
                params[aw::MODE] = k as f32;
            }
        }
        ui.separator();
        for (k, name) in aw::BODIES.iter().enumerate() {
            if ui.selectable_label(params[aw::BODY].round() as usize == k, ink(name)).on_hover_text("The body: its size, its airway and its pitch (a change builds a new airway)").clicked() {
                params[aw::BODY] = k as f32;
            }
        }
        ui.separator();
        ui.label(RichText::new("Voices").small().color(crate::skin::ink_dim()));
        for (k, name) in aw::VOICE_COUNTS.iter().enumerate() {
            if ui.selectable_label(params[aw::VOICES].round() as usize == k, ink(name)).on_hover_text("How many keys sing at once: each voice is a whole simulated airway, and costs much (start with one)").clicked() {
                params[aw::VOICES] = k as f32;
            }
        }
    });
    let mode = params[aw::MODE].round() as usize;
    if mode == 1 {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Lyric").small().color(crate::skin::ink_dim()));
            let edit = ui.add(egui::TextEdit::singleline(text).desired_width(320.0).hint_text("words to sing, a syllable a key (ti-ger splits by hand, - holds a vowel on)"));
            if edit.lost_focus() || (edit.changed() && !edit.has_focus()) {
                *plans = aw::prepare(text, params[aw::BODY].round() as u8);
            }
            // The dictionary: for English as it is spoken (without it, from the spelling).
            let key = id.with("dict_fetch");
            let fetch: Option<Fetch> = ui.data(|d| d.get_temp(key));
            match fetch.as_ref().and_then(|f| f.lock().ok().and_then(|g| g.clone())) {
                Some(Ok(())) => {
                    ui.data_mut(|d| d.remove::<Fetch>(key));
                    aw::reload_dictionary();
                    *plans = aw::prepare(text, params[aw::BODY].round() as u8);
                    said = Some("The pronouncing dictionary is in: the lyric is sung as English is spoken".to_string());
                }
                Some(Err(e)) => {
                    ui.data_mut(|d| d.remove::<Fetch>(key));
                    said = Some(format!("Could not get the dictionary: {e}"));
                }
                None if fetch.is_some() => {
                    ui.spinner();
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
                }
                None => {
                    if !aw::has_dictionary() && ui.button("Get dictionary").on_hover_text("The CMU pronouncing dictionary (3.6 MB, BSD licence) into Documents/Krabton/Library/CMUdict: words sung as English speaks them, not guessed from their spelling").clicked() {
                        let f: Fetch = Arc::new(Mutex::new(None));
                        let g = f.clone();
                        std::thread::spawn(move || {
                            let to = aw::dict_path();
                            let r = to.parent().map_or(Ok(()), std::fs::create_dir_all).map_err(|e| e.to_string()).and_then(|_| crate::piano_lib::fetch(aw::DICT_URL, &to));
                            if let Ok(mut x) = g.lock() {
                                *x = Some(r);
                            }
                        });
                        ui.data_mut(|d| d.insert_temp(key, f));
                    }
                }
            }
        });
        // The syllables, the one sung last lit.
        let sung = live.vals[4].round() as usize;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            for (k, s) in plans.syls.iter().enumerate().take(64) {
                let on = k == sung && live.vals[5..9].iter().any(|v| *v >= 0.0);
                let t = RichText::new(&s.text).small().color(if on { text_on(accent) } else { crate::skin::well().1 });
                ui.add(egui::Button::new(t).fill(if on { accent } else { crate::skin::well().0 }).corner_radius(8.0));
            }
            if plans.syls.is_empty() {
                ui.label(RichText::new("No lyric yet: type words above").small().color(crate::skin::ink_dim()));
            }
        });
    } else if mode == 2 {
        let c = (params[aw::CREATURE].round().max(0.0) as usize).min(aw::CREATURES.len() - 1);
        ui.horizontal(|ui| {
            for (k, name) in aw::CREATURES.iter().enumerate() {
                if ui.selectable_label(k == c, ink(name)).clicked() {
                    params[aw::CREATURE] = k as f32;
                }
            }
            ui.separator();
            if let Some(def) = aw::creature_def(c) {
                for (k, call) in def.calls.iter().enumerate().take(8) {
                    if ui.selectable_label(params[aw::CALL].round() as usize == k + 1, ink(call.name)).on_hover_text(call.hint).clicked() {
                        params[aw::CALL] = (k + 1) as f32;
                    }
                }
            }
        });
    }
    if params[aw::BODY].round() as u8 != body_was && !text.is_empty() {
        *plans = aw::prepare(text, params[aw::BODY].round() as u8);
    }
    // The rest as knobs (the choices above are on the screen only).
    let rows = if mode == 1 { 70.0 } else { 36.0 };
    super::devices::param_grid_mod(ui, id.with("knobs"), aw::SPECS, params, accent, height - rows, mv);
    if live.vals[..4].iter().any(|v| *v > 0.0) {
        ui.ctx().request_repaint();
    }
    said
}
