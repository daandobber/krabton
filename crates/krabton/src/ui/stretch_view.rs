//! The Stretch: its sample drawn large, Position as a handle on it (click
//! or drag anywhere on the sample to scrub, while keys hold), each voice a
//! light where it reads, and over it the stretched spectrum as it sounds,
//! glowing bands from low to high. Beside it the window sizes and Freeze as
//! buttons, and the rest on faders, a page at a time. A sample dropped on it
//! (from the browser or from outside) is what it stretches.

use super::inst_screen::{InstLive, SCREEN_BG};
use super::modulation::{slider_overlay, ModView};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, RichText, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::stretch::{self as st, *};
use krabton_engine::sample::{SampleData, SampleRef};

const WAVE: Color32 = Color32::from_rgb(0x3c, 0x4a, 0x58);
const GLOW: Color32 = Color32::from_rgb(0x8f, 0xd6, 0xff);
const VOICE: Color32 = Color32::from_rgb(0xff, 0xd2, 0x7a);
const ICE: Color32 = Color32::from_rgb(0xc8, 0xf0, 0xff);

/// The pages of faders: a name and the parameters on it.
const PAGES: &[(&str, &[usize])] = &[
    ("Stretch", &[STRETCH, POSITION, ONSET, SHIMMER]),
    ("Pitch", &[PITCH, ROOT]),
    ("Amp", &[ATTACK, RELEASE, VELOCITY, VOICES]),
    ("Space", &[TONE, SPACE, VOLUME]),
];

/// The Stretch's panel. Returns a message for the player (a sample that
/// would not load, say).
#[allow(clippy::too_many_arguments)]
pub(crate) fn stretch_panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut Vec<f32>, sample: &mut Option<SampleRef>, live: &InstLive, height: f32, mut mv: Option<&mut ModView>) -> Option<String> {
    if params.len() < SPECS.len() {
        return None;
    }
    st::warm();
    let mut said = None;
    let mut load = |path: &std::path::Path, sample: &mut Option<SampleRef>| match SampleRef::load(path) {
        Ok(s) => *sample = Some(s),
        Err(e) => said = Some(format!("Could not load the sample: {e}")),
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new("Preset").small().color(crate::skin::ink_dim()));
        egui::ComboBox::from_id_salt(id.with("preset")).width(130.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
            for name in preset_names() {
                if ui.selectable_label(preset == name, *name).clicked() {
                    *preset = name.to_string();
                    *params = st::preset(name);
                }
            }
        });
        if ui.button("Load sample...").on_hover_text("A WAV, FLAC, MP3 or AIFF file to stretch (or drop one on the sample)").clicked() {
            if let Some(path) = crate::import::audio_file_dialog().pick_file() {
                load(&path, sample);
            }
        }
        match sample.as_ref() {
            Some(s) => {
                ui.label(RichText::new(format!("{}  {:.1} s", s.name, s.duration())).color(accent_now()));
                if ui.small_button("x").on_hover_text("Back to its own sound").clicked() {
                    *sample = None;
                }
            }
            None => {
                ui.label(RichText::new("its own chord: drop a sample on it").small().color(crate::skin::ink_dim()));
            }
        }
    });
    let playing = sample.clone();
    let src = st::source(playing.as_ref());
    let h = (height - 30.0).max(120.0);
    ui.horizontal_top(|ui| {
        let key = id.with("screen_w");
        let mut w = ui.data_mut(|d| *d.get_persisted_mut_or(key, 400.0f32));
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click_and_drag());
        let before = w;
        width_grip(ui, id.with("screen_grip"), rect, &mut w, 260.0, 1400.0);
        if w != before {
            ui.data_mut(|d| d.insert_persisted(key, w));
        }
        screen(ui, rect, &resp, params, src, live);
        if let Some(item) = drop_payload::<crate::app::BrowserItem>(&resp) {
            if let crate::app::BrowserItem::Sample(path) = &*item {
                load(path, sample);
            }
        }
        let here = crate::import::file_drop_pos(ui.ctx()).is_none_or(|p| rect.contains(p));
        let files_over = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
        if resp.dnd_hover_payload::<crate::app::BrowserItem>().is_some() || (files_over && here) {
            rect_line(ui.painter(), rect, 6.0, 2.0, accent_now());
        }
        let dropped: Vec<std::path::PathBuf> = ui.ctx().input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if let Some(path) = dropped.iter().find(|p| krabton_engine::sample::is_audio_file(p)).filter(|_| here) {
            load(path, sample);
            crate::import::take_file_drop(ui.ctx());
        }
        ui.add_space(8.0);
        switches(ui, id, params, h, live);
        ui.add_space(8.0);
        faders(ui, id, params, h, mv.as_deref_mut());
    });
    if live.vals[0] > 0.0 {
        ui.ctx().request_repaint();
    }
    said
}

/// The sample, Position, the voices and the spectrum. A click or a drag
/// moves Position (the voices with it).
fn screen(ui: &mut Ui, rect: Rect, resp: &egui::Response, p: &mut [f32], src: &SampleData, live: &InstLive) {
    let inner = rect.shrink2(vec2(10.0, 16.0));
    let x_of = |f: f32| inner.left() + f.clamp(0.0, 1.0) * inner.width();
    if let Some(q) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        p[POSITION] = ((q.x - inner.left()) / inner.width()).clamp(0.0, 1.0);
    }
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    let painter = ui.painter_at(rect);
    rect_fill(&painter, rect, 6.0, SCREEN_BG);
    // The spectrum as it sounds, behind the sample: a glowing band per range.
    let bands = &live.vals[BANDS_AT..BANDS_AT + BANDS];
    let bw = inner.width() / BANDS as f32;
    for (k, v) in bands.iter().enumerate() {
        if *v <= 0.01 {
            continue;
        }
        let x = inner.left() + k as f32 * bw;
        let top = inner.bottom() - v * inner.height();
        let r = Rect::from_min_max(pos2(x + 1.0, top), pos2(x + bw - 1.0, inner.bottom()));
        painter.rect_filled(r, 2.0, GLOW.gamma_multiply(0.10 + 0.25 * v));
        painter.line_segment([pos2(x + 1.0, top), pos2(x + bw - 1.0, top)], Stroke::new(1.5_f32, GLOW.gamma_multiply(0.3 + 0.6 * v)));
    }
    // The sample.
    let cols = inner.width().max(1.0) as usize;
    let peaks = &src.peaks;
    let mid = inner.center().y;
    let amp = inner.height() * 0.42;
    if !peaks.is_empty() {
        for c in 0..cols {
            let a = c * peaks.len() / cols;
            let b = ((c + 1) * peaks.len() / cols).max(a + 1).min(peaks.len());
            let (lo, hi) = peaks[a..b].iter().fold((0.0f32, 0.0f32), |(lo, hi), &(mn, mx)| (lo.min(mn), hi.max(mx)));
            let x = inner.left() + c as f32 + 0.5;
            painter.line_segment([pos2(x, mid - hi * amp), pos2(x, mid - lo * amp + 1.0)], Stroke::new(1.0_f32, WAVE));
        }
    }
    // Where each voice reads (its window, as wide as it is).
    let frames = live.vals[1].max(1.0);
    let win = SIZES[(p[WINDOW].round().max(0.0) as usize).min(SIZES.len() - 1)] as f32;
    for k in 0..MAX_VOICES {
        let at = live.vals[2 + k];
        if at < 0.0 {
            continue;
        }
        let a = at / frames;
        let b = (at + win) / frames;
        let r = Rect::from_min_max(pos2(x_of(a), inner.top()), pos2(x_of(b).max(x_of(a) + 2.0), inner.bottom()));
        painter.rect_filled(r, 0.0, VOICE.gamma_multiply(0.16));
        let c = pos2(x_of((a + b) * 0.5), inner.top() + 4.0);
        painter.circle_filled(c, 4.0, VOICE);
    }
    // Position: a line with a handle on top.
    let px = x_of(p[POSITION]);
    let frozen = p[FREEZE] >= 0.5 || live.vals[6] > 0.5;
    let line = if frozen { ICE } else { accent_now() };
    painter.line_segment([pos2(px, rect.top() + 4.0), pos2(px, inner.bottom())], Stroke::new(2.0_f32, line));
    painter.add(Shape::convex_polygon(vec![pos2(px - 7.0, rect.top() + 2.0), pos2(px + 7.0, rect.top() + 2.0), pos2(px, rect.top() + 12.0)], line, Stroke::NONE));
    let font = FontId::proportional(11.0);
    text(&painter, pos2(rect.left() + 8.0, rect.top() + 7.0), Align2::LEFT_CENTER, format!("{}  {:.1} s", src.name, src.duration()), font.clone(), Color32::from_gray(170));
    let n = live.vals[0].round() as usize;
    let right = if frozen {
        "frozen".to_string()
    } else if n > 0 {
        format!("{n} voice{}  x{:.0}", if n == 1 { "" } else { "s" }, p[STRETCH])
    } else {
        format!("x{:.0}: play a key", p[STRETCH])
    };
    text(&painter, pos2(rect.right() - 8.0, rect.top() + 7.0), Align2::RIGHT_CENTER, right, font.clone(), if frozen { ICE } else { Color32::from_gray(170) });
    text(&painter, pos2(rect.left() + 8.0, rect.bottom() - 8.0), Align2::LEFT_CENTER, format!("Position {:.0} %   click or drag to scrub", p[POSITION] * 100.0), font, Color32::from_gray(120));
}

/// The window sizes, Freeze and Keys, as buttons.
fn switches(ui: &mut Ui, id: Id, p: &mut [f32], h: f32, live: &InstLive) {
    let w = 176.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let painter = ui.painter().clone();
    let ink = crate::skin::ink_dim();
    let accent = accent_now();
    let row_h = ((h - 2.0 * 13.0 - 16.0) / 4.0).clamp(16.0, 34.0);
    let mut y = rect.top();
    text(&painter, pos2(rect.left(), y + 6.0), Align2::LEFT_CENTER, "Window", small(), ink);
    y += 13.0;
    // The windows, a row of five: longer is smoother and smearier.
    let bw = w / WINDOWS.len() as f32;
    for (k, name) in WINDOWS.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + k as f32 * bw, y), vec2(bw - 3.0, row_h));
        let on = p[WINDOW].round() as usize == k;
        let short = name.trim_end_matches(" s");
        if button(ui, r, id.with(("window", k)), short, on, accent).on_hover_text(format!("Windows of {name}: shorter keeps more of the sound's movement, longer smears it smoother")).clicked() {
            p[WINDOW] = k as f32;
        }
    }
    y += row_h + 8.0;
    // Freeze, big: the place stands still (the sustain pedal does it too).
    let pedal = live.vals[6] > 0.5;
    let r = Rect::from_min_size(pos2(rect.left(), y), vec2(w, row_h * 2.0));
    let on = p[FREEZE] >= 0.5;
    if button(ui, r, id.with("freeze"), if pedal && !on { "Frozen (pedal)" } else { "Freeze" }, on || pedal, ICE).on_hover_text("Hold this moment: the place stops moving and the spectrum stays (the sustain pedal holds it too)").clicked() {
        p[FREEZE] = if on { 0.0 } else { 1.0 };
    }
    y += row_h * 2.0 + 8.0;
    text(&painter, pos2(rect.left(), y + 6.0), Align2::LEFT_CENTER, "Keys", small(), ink);
    y += 13.0;
    let half = w / 2.0;
    for (k, (name, tip)) in [("Pitch", "Keys play the stretch at their pitch (Root plays it as it is)"), ("Fixed", "Every key plays it at its own pitch")].into_iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + k as f32 * half, y), vec2(half - 3.0, row_h));
        let on = (p[KEYS] >= 0.5) == (k == 0);
        if button(ui, r, id.with(("keys", k)), name, on, accent).on_hover_text(tip).clicked() {
            p[KEYS] = if k == 0 { 1.0 } else { 0.0 };
        }
    }
}

/// The faders of one page, the pages picked from a column beside them.
fn faders(ui: &mut Ui, id: Id, p: &mut [f32], h: f32, mut mv: Option<&mut ModView>) {
    let tab_key = id.with("page");
    let mut page = ui.data(|d| d.get_temp::<usize>(tab_key).unwrap_or(0)).min(PAGES.len() - 1);
    let fw = 46.0;
    let most = PAGES.iter().map(|pg| pg.1.len()).max().unwrap_or(1);
    let tab_w = 64.0;
    let (rect, _) = ui.allocate_exact_size(vec2(tab_w + most as f32 * fw, h), Sense::hover());
    let tab_h = ((h - 3.0 * (PAGES.len() - 1) as f32) / PAGES.len() as f32).min(30.0);
    let accent = accent_now();
    for (k, (name, _)) in PAGES.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left(), rect.top() + k as f32 * (tab_h + 3.0)), vec2(tab_w - 8.0, tab_h));
        if button(ui, r, id.with(("page", k)), name, k == page, accent).clicked() {
            page = k;
            ui.data_mut(|d| d.insert_temp(tab_key, k));
        }
    }
    for (k, &pi) in PAGES[page].1.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + tab_w + k as f32 * fw, rect.top()), vec2(fw - 4.0, h));
        let spec = &SPECS[pi];
        let mut v = p[pi];
        let (changed, track) = param_slider(ui, r, id.with(("fader", pi)), spec, &mut v, accent);
        if changed {
            p[pi] = v;
        }
        if let Some(mv) = mv.as_deref_mut() {
            slider_overlay(ui, r, track, id.with(("mod", pi)), spec, p[pi], pi, mv);
        }
    }
}
