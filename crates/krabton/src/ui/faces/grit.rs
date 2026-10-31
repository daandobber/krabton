//! Grit as a pad sampler of the old kind: sixteen rubbery pads (pad 1 at
//! the bottom left), a small green display with the pad's name, its wave
//! and what the machine is doing, the pad's own sliders (tune, level, pan,
//! decay, filter) and its buttons (reverse, gate, choke, load, chop), the
//! machine's character and drive, note repeat with its swing and the 16
//! Levels. A sample dropped on a pad goes on that pad.

use super::{draw, Ctl, Ctx, Face, Presets};
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Stroke, Ui};
use krabton_engine::dsp::grit::{self as gr, GritPad};
use krabton_engine::params::{ParamSpec, Unit};
use krabton_engine::sample::SampleRef;

pub(crate) fn face() -> Face {
    Face::new(59.8, 12.0)
        .screened(&["Levels Pad"])
        .section(0.0, 0.0, 14.0, 12.0, "Grit")
        .preset(0.3, 0.3, 13.4, 1.3)
        .screen(0.3, 1.8, 13.4, 4.4, "lcd")
        .screen(0.3, 6.5, 13.4, 4.7, "padctl")
        .section(14.2, 0.0, 12.0, 12.0, "Pad")
        .screen(14.4, 0.3, 11.6, 10.9, "padfaders")
        .section(26.4, 0.0, 15.0, 12.0, "Pads")
        .screen(26.6, 0.3, 14.6, 10.9, "pads")
        .section(41.6, 0.0, 8.4, 12.0, "Machine")
        .ctl(41.8, 0.3, 3.8, 10.9, "Character", Ctl::Buttons)
        .ctl(45.6, 0.3, 2.2, 10.9, "Drive", Ctl::Fader)
        .named(47.8, 0.3, 2.2, 10.9, "Volume", Ctl::Fader, "Vol")
        .section(50.2, 0.0, 9.6, 12.0, "Repeat")
        .ctl(50.4, 0.3, 3.0, 10.9, "Repeat", Ctl::Buttons)
        .ctl(53.4, 0.3, 2.2, 10.9, "Swing", Ctl::Fader)
        .named(55.6, 0.3, 4.0, 10.9, "16 Levels", Ctl::Buttons, "16 Levels")
}

/// What the face does this frame: notes for the pads (None: let go), the
/// pad chosen, a message for the player.
pub(crate) struct Played {
    pub notes: Vec<(u8, Option<f32>)>,
    pub said: Option<String>,
}

static TUNE: ParamSpec = ParamSpec::lin("Tune", "Pad", -24.0, 24.0, 0.0, Unit::Semi);
static LEVEL: ParamSpec = ParamSpec::lin("Level", "Pad", -30.0, 6.0, 0.0, Unit::Db);
static PAN: ParamSpec = ParamSpec::lin("Pan", "Pad", -1.0, 1.0, 0.0, Unit::Pan);
static DECAY: ParamSpec = ParamSpec::lin("Decay", "Pad", 0.0, 1.0, 1.0, Unit::Percent);
static FILTER: ParamSpec = ParamSpec::lin("Filter", "Pad", 0.0, 1.0, 1.0, Unit::Percent);

/// Grit's panel: the face (its preset display also sets the pads that have
/// no sample of their own).
#[allow(clippy::too_many_arguments)]
pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut Vec<f32>, pads: &mut Vec<GritPad>, sel: &mut usize, live: &InstLive, height: f32, mv: Option<&mut ModView>) -> Played {
    let mut out = Played { notes: Vec::new(), said: None };
    if params.len() < gr::SPECS.len() {
        return out;
    }
    gr::warm();
    gr::fix_pads(pads);
    *sel = (*sel).min(gr::PADS - 1);
    let h = height.max(120.0);
    let face = face();
    let before = preset.clone();
    let mut notes = Vec::new();
    let mut said = None;
    let mut dropped_on: Option<(usize, std::path::PathBuf)> = None;
    let resp = {
        let pads_ref = &mut *pads;
        let sel_ref = &mut *sel;
        let mut cx = Ctx { id, keys: gr::SPECS, specs: gr::SPECS, params, mv, presets: Some(Presets { current: preset, names: gr::preset_names(), load: gr::preset }) };
        ui.scope(|ui| {
            draw(ui, &face, h, &mut cx, &mut |ui, which, rect, p| match which {
                "lcd" => {
                    lcd(ui, rect, p, pads_ref, *sel_ref, live);
                    false
                }
                "padctl" => pad_buttons(ui, id.with("padctl"), rect, p, pads_ref, *sel_ref, &mut said),
                "padfaders" => pad_faders(ui, id.with("padfaders"), rect, &mut pads_ref[*sel_ref]),
                "pads" => {
                    pad_grid(ui, id.with("pads"), rect, pads_ref, sel_ref, live, &mut notes, &mut dropped_on);
                    false
                }
                _ => false,
            });
        })
        .response
    };
    // A new preset: its kit on the pads that have no sample of their own.
    if *preset != before {
        for (pad, new) in pads.iter_mut().zip(gr::preset_pads(preset)) {
            if pad.sample.is_none() {
                *pad = new;
            }
        }
    }
    // A sample from the browser or from outside: on the pad it was dropped
    // on, or the chosen one.
    if let Some(item) = drop_payload::<crate::app::BrowserItem>(&resp) {
        if let crate::app::BrowserItem::Sample(path) = &*item {
            dropped_on.get_or_insert((*sel, path.clone()));
        }
    }
    let here = crate::import::file_drop_pos(ui.ctx()).is_none_or(|p| resp.rect.contains(p));
    let files: Vec<std::path::PathBuf> = ui.ctx().input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
    if let Some(path) = files.iter().find(|p| krabton_engine::sample::is_audio_file(p)).filter(|_| here) {
        let at = crate::import::file_drop_pos(ui.ctx()).and_then(|p| pad_at(ui, id.with("pads"), p)).unwrap_or(*sel);
        dropped_on = Some((at, path.clone()));
        crate::import::take_file_drop(ui.ctx());
    }
    if let Some((k, path)) = dropped_on {
        match SampleRef::load(&path) {
            Ok(s) => {
                put_on(&mut pads[k], s);
                *sel = k;
            }
            Err(e) => said = Some(format!("Could not load the sample: {e}")),
        }
    }
    if live.vals[0] > 0.0 || live.vals[2..2 + gr::PADS].iter().any(|&v| v > 0.01) {
        ui.ctx().request_repaint();
    }
    out.notes = notes;
    out.said = said;
    out
}

/// A sample on a pad: named after it, the pad's other settings kept.
pub(crate) fn put_on(pad: &mut GritPad, s: SampleRef) {
    pad.name = s.name.clone();
    pad.sample = Some(s);
    pad.sound = None;
}

/// Where the pads were drawn last (kept for a drop from outside).
fn pad_at(ui: &Ui, id: Id, p: egui::Pos2) -> Option<usize> {
    let r = ui.ctx().memory(|m| m.data.get_temp::<Rect>(id.with("rect")))?;
    r.contains(p).then(|| {
        let (c, row) = (((p.x - r.left()) / r.width() * 4.0) as usize, ((p.y - r.top()) / r.height() * 4.0) as usize);
        (3 - row.min(3)) * 4 + c.min(3)
    })
}

fn pad_rect(r: Rect, k: usize) -> Rect {
    let (w, h) = (r.width() / 4.0, r.height() / 4.0);
    let (c, row) = (k % 4, 3 - k / 4);
    Rect::from_min_size(pos2(r.left() + c as f32 * w, r.top() + row as f32 * h), vec2(w, h)).shrink((w.min(h) * 0.07).max(2.0))
}

/// The sixteen pads: pressed, they play (and are chosen); a sample dropped on one goes there.
#[allow(clippy::too_many_arguments)]
fn pad_grid(ui: &mut Ui, id: Id, r: Rect, pads: &[GritPad], sel: &mut usize, live: &InstLive, notes: &mut Vec<(u8, Option<f32>)>, dropped: &mut Option<(usize, std::path::PathBuf)>) {
    ui.ctx().memory_mut(|m| m.data.insert_temp(id.with("rect"), r));
    let p = ui.painter().clone();
    let (well, ink) = crate::skin::well();
    let rubber = Color32::from_rgb(0x5a, 0x5c, 0x5f);
    for k in 0..gr::PADS {
        let b = pad_rect(r, k);
        let resp = ui.interact(b, id.with(k), Sense::click_and_drag());
        let key = id.with(("down", k));
        let down = resp.is_pointer_button_down_on();
        let was = ui.ctx().memory(|m| m.data.get_temp::<bool>(key)).unwrap_or(false);
        if down && !was {
            // Harder towards the pad's top, as a pad without the full-level button.
            let vel = resp.interact_pointer_pos().map_or(0.85, |pt| (1.0 - (pt.y - b.top()) / b.height() * 0.6).clamp(0.3, 1.0));
            notes.push((gr::FIRST_NOTE + k as u8, Some(vel)));
            *sel = k;
        } else if was && !down {
            notes.push((gr::FIRST_NOTE + k as u8, None));
        }
        if down != was {
            ui.ctx().memory_mut(|m| m.data.insert_temp(key, down));
        }
        if let Some(item) = drop_payload::<crate::app::BrowserItem>(&resp) {
            if let crate::app::BrowserItem::Sample(path) = &*item {
                *dropped = Some((k, path.clone()));
            }
        }
        let lit = live.vals[2 + k].clamp(0.0, 1.0);
        let pad = &pads[k];
        let base = if pad.is_empty() { mix(rubber, Color32::BLACK, 0.35) } else { rubber };
        let fill = mix(if down { mix(base, Color32::BLACK, 0.2) } else { base }, accent_now(), lit * 0.75);
        p.rect_filled(b.translate(vec2(0.0, 2.5)), 7.0, Color32::from_black_alpha(130));
        p.rect_filled(b, 7.0, fill);
        // The rubber's soft top.
        p.rect_filled(Rect::from_min_size(b.min + vec2(4.0, 3.0), vec2(b.width() - 8.0, b.height() * 0.35)), 5.0, Color32::from_white_alpha(10));
        if k == *sel {
            p.rect_stroke(b.expand(1.5), 8.0, Stroke::new(2.0_f32, accent_now()), egui::StrokeKind::Outside);
        }
        if resp.dnd_hover_payload::<crate::app::BrowserItem>().is_some() {
            p.rect_stroke(b, 7.0, Stroke::new(2.0_f32, Color32::WHITE), egui::StrokeKind::Inside);
        }
        let f = FontId::proportional((b.height() * 0.2).clamp(8.0, 12.0));
        p.text(b.left_top() + vec2(5.0, 4.0), Align2::LEFT_TOP, format!("{}", k + 1), f.clone(), mix(ink, well, 0.3));
        let name = pad.label();
        let nf = super::fit(&p, name, f, b.width() - 8.0);
        p.text(pos2(b.center().x, b.bottom() - 4.0), Align2::CENTER_BOTTOM, name, nf, ink);
        resp.on_hover_text(format!("Pad {} ({}): {}\nPress to play (higher on the pad is harder); drop a sample on it", k + 1, note_name(gr::FIRST_NOTE + k as u8), if name.is_empty() { "empty" } else { name }));
    }
}

fn note_name(n: u8) -> String {
    const N: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    format!("{}{}", N[n as usize % 12], n as i32 / 12 - 2)
}

/// The display: the pad, its wave, the machine, the repeat and the levels.
fn lcd(ui: &mut Ui, r: Rect, p: &[f32], pads: &[GritPad], sel: usize, live: &InstLive) {
    let painter = ui.painter().clone();
    let bg = Color32::from_rgb(0x9d, 0xb8, 0x6a);
    let ink = Color32::from_rgb(0x1e, 0x2a, 0x12);
    painter.rect_filled(r.expand(2.0), 4.0, Color32::from_rgb(0x1a, 0x1b, 0x1d));
    painter.rect_filled(r, 2.0, bg);
    let lh = (r.height() / 5.0).clamp(9.0, 16.0);
    let f = FontId::monospace((lh * 0.78).clamp(8.0, 13.0));
    let pad = &pads[sel];
    let line = |k: f32, text: &str, align: Align2| {
        let y = r.top() + 3.0 + k * lh + lh * 0.5;
        let x = if align == Align2::RIGHT_CENTER { r.right() - 5.0 } else { r.left() + 5.0 };
        painter.text(pos2(x, y), align, text, f.clone(), ink);
    };
    let name = if pad.label().is_empty() { "-- empty --" } else { pad.label() };
    line(0.0, &format!("PAD {:02} {}", sel + 1, name), Align2::LEFT_CENTER);
    let ch = gr::CHARACTERS[(p[gr::CHARACTER].round().max(0.0) as usize).min(gr::CHARACTERS.len() - 1)];
    line(0.0, ch, Align2::RIGHT_CENTER);
    let rep = gr::REPEATS[(p[gr::REPEAT].round().max(0.0) as usize).min(gr::REPEATS.len() - 1)];
    let held = live.vals[18] as u32;
    let rep_text = if rep == "Off" { "REPEAT OFF".to_string() } else { format!("REPEAT {rep} SW {:.0}%{}", p[gr::SWING] * 100.0, if held != 0 { " *" } else { "" }) };
    line(1.0, &rep_text, Align2::LEFT_CENTER);
    let lv = (p[gr::LEVELS].round().max(0.0) as usize).min(gr::LEVEL_MODES.len() - 1);
    if lv > 0 {
        line(1.0, &format!("16LV {} P{:.0}", gr::LEVEL_MODES[lv].to_uppercase(), p[gr::LEVELS_PAD]), Align2::RIGHT_CENTER);
    }
    // The pad's wave, in the display's dots.
    let wave = Rect::from_min_max(pos2(r.left() + 5.0, r.top() + 3.0 + 2.0 * lh), pos2(r.right() - 5.0, r.bottom() - 4.0));
    painter.line_segment([pos2(wave.left(), wave.center().y), pos2(wave.right(), wave.center().y)], Stroke::new(1.0_f32, mix(ink, bg, 0.7)));
    let data: Option<&[f32]> = match (&pad.sample, pad.sound) {
        (Some(s), _) => Some(&s.left[..]),
        (None, Some(k)) => gr::sound(k as usize),
        _ => None,
    };
    if let Some(d) = data.filter(|d| !d.is_empty()) {
        let cols = (wave.width() / 2.0).max(1.0) as usize;
        let per = (d.len() / cols).max(1);
        for c in 0..cols {
            let a = c * d.len() / cols;
            let chunk = &d[a..(a + per).min(d.len())];
            let m = chunk.iter().fold(0.0f32, |m, v| m.max(v.abs())).min(1.0);
            let x = wave.left() + c as f32 * 2.0 + 1.0;
            let x = if pad.reverse { wave.right() - (x - wave.left()) } else { x };
            painter.line_segment([pos2(x, wave.center().y - m * wave.height() * 0.5), pos2(x, wave.center().y + m * wave.height() * 0.5)], Stroke::new(1.5_f32, ink));
        }
    }
    let lvl = live.vals[1].clamp(0.0, 1.0);
    let meter = Rect::from_min_max(pos2(wave.right() - 4.0, wave.bottom() - wave.height() * lvl), pos2(wave.right(), wave.bottom()));
    painter.rect_filled(meter, 0.0, ink);
}

/// The chosen pad's sliders.
fn pad_faders(ui: &mut Ui, id: Id, r: Rect, pad: &mut GritPad) -> bool {
    let w = r.width() / 5.0;
    let mut changed = false;
    let mut fader = |ui: &mut Ui, k: usize, spec: &ParamSpec, v: &mut f32| {
        let cell = Rect::from_min_size(pos2(r.left() + k as f32 * w, r.top()), vec2(w, r.height()));
        changed |= param_slider(ui, cell, id.with(k), spec, v, accent_now()).0;
    };
    fader(ui, 0, &TUNE, &mut pad.tune);
    fader(ui, 1, &LEVEL, &mut pad.level_db);
    fader(ui, 2, &PAN, &mut pad.pan);
    fader(ui, 3, &DECAY, &mut pad.decay);
    fader(ui, 4, &FILTER, &mut pad.filter);
    changed
}

/// The chosen pad's buttons: reverse, gate, choke; load a sample, chop it,
/// clear it, play it in 16 Levels.
fn pad_buttons(ui: &mut Ui, id: Id, r: Rect, p: &mut [f32], pads: &mut Vec<GritPad>, sel: usize, said: &mut Option<String>) -> bool {
    let rows = 2.0;
    let rh = r.height() / rows;
    let mut changed = false;
    let cell = |row: usize, k: usize, n: usize| {
        let w = r.width() / n as f32;
        Rect::from_min_size(pos2(r.left() + k as f32 * w, r.top() + row as f32 * rh), vec2(w, rh)).shrink(2.5)
    };
    let on = accent_now();
    let pad = &mut pads[sel];
    if button(ui, cell(0, 0, 4), id.with("rev"), "REVERSE", pad.reverse, on).on_hover_text("Play the pad backwards").clicked() {
        pad.reverse = !pad.reverse;
        changed = true;
    }
    if button(ui, cell(0, 1, 4), id.with("gate"), if pad.gate { "GATE" } else { "ONE SHOT" }, pad.gate, on).on_hover_text("One shot: plays to its end; gate: while held").clicked() {
        pad.gate = !pad.gate;
        changed = true;
    }
    let choke = if pad.choke == 0 { "CHOKE -".to_string() } else { format!("CHOKE {}", pad.choke) };
    if button(ui, cell(0, 2, 4), id.with("choke"), &choke, pad.choke > 0, on).on_hover_text("Click for the next choke group (1 to 4, or none): pads of a group cut each other off").clicked() {
        pad.choke = (pad.choke + 1) % 5;
        changed = true;
    }
    let lv_on = p[gr::LEVELS] >= 0.5 && p[gr::LEVELS_PAD].round() as usize == sel + 1;
    if button(ui, cell(0, 3, 4), id.with("lv"), "16 LEVELS", lv_on, on).on_hover_text("Play this pad's sample on all sixteen in steps (16 Levels on Tune, or as set)").clicked() {
        p[gr::LEVELS_PAD] = (sel + 1) as f32;
        if p[gr::LEVELS] < 0.5 {
            p[gr::LEVELS] = 1.0;
        } else if lv_on {
            p[gr::LEVELS] = 0.0;
        }
        changed = true;
    }
    if button(ui, cell(1, 0, 4), id.with("load"), "LOAD", false, on).on_hover_text("A WAV, FLAC, MP3 or AIFF for this pad (or drop one on it)").clicked() {
        if let Some(path) = crate::import::audio_file_dialog().pick_file() {
            match SampleRef::load(&path) {
                Ok(s) => put_on(&mut pads[sel], s),
                Err(e) => *said = Some(format!("Could not load the sample: {e}")),
            }
            changed = true;
        }
    }
    let loop_here = pads[sel].sample.clone();
    for (k, (label, by, tip)) in [("HITS", gr::Chop::Transients, "Chop this pad's sample (a loop, a break) to the pads at its hits (the strongest sixteen)"), ("GRID", gr::Chop::Grid, "Chop this pad's sample (a loop, a break) to the pads in sixteen equal parts")].into_iter().enumerate() {
        let resp = button(ui, cell(1, 1 + k, 4), id.with(("chop", k)), label, false, on).on_hover_text(tip);
        if resp.clicked() {
            match &loop_here {
                Some(s) => {
                    *pads = gr::chop(s, by);
                    changed = true;
                }
                None => *said = Some("Put a sample (a loop) on the pad first, then chop it".into()),
            }
        }
    }
    if button(ui, cell(1, 3, 4), id.with("clear"), "CLEAR", false, on).on_hover_text("Empty this pad").clicked() {
        pads[sel] = GritPad::default();
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_face_holds_every_parameter() {
        super::super::check(&super::face(), krabton_engine::dsp::grit::SPECS);
    }

    #[test]
    fn pad_one_is_at_the_bottom_left() {
        let r = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 400.0));
        assert!(super::pad_rect(r, 0).center().y > 300.0 && super::pad_rect(r, 0).center().x < 100.0);
        assert!(super::pad_rect(r, 15).center().y < 100.0 && super::pad_rect(r, 15).center().x > 300.0);
    }
}
