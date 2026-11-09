//! Breaks: the break drawn large with its cuts (the one playing lit), and
//! under it the bar as it plays now, step by step: which cut each step
//! plays (as a bar as high as where it is in the break), its rolls as
//! ticks, backwards as a blue step, pitched as a dot up or down, a half as
//! one long step. Beside it its choices as buttons and the rest on faders,
//! a page at a time. A break dropped on it (from the browser or outside) is
//! what it plays; without one, its own.

use super::inst_screen::{InstLive, SCREEN_BG};
use super::modulation::{slider_overlay, ModView};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, RichText, Sense, Stroke, Ui};
use krabton_engine::dsp::breaks::{self as br, *};
use krabton_engine::sample::{SampleData, SampleRef};

const WAVE: Color32 = Color32::from_rgb(0x4a, 0x4f, 0x5c);
const LIT: Color32 = Color32::from_rgb(0xff, 0xc2, 0x4a);
const BACKWARD: Color32 = Color32::from_rgb(0x6f, 0xb4, 0xff);
const UP: Color32 = Color32::from_rgb(0xff, 0x6f, 0x8a);

/// The pages of faders: a name and the parameters on it.
/// (The LFO page, empty here, has its own controls.)
const PAGES: &[(&str, &[usize])] = &[
    ("Cut", &[SHUFFLE, REPEAT, REVERSE, HALF]),
    ("Roll", &[RATCHET, RAMP, SWING, GATE]),
    ("Pitch", &[PITCH, JUMPS, JUMP_RANGE]),
    ("Sound", &[TONE, LEVEL]),
    ("Phrase", &[VARY]),
    ("LFO", &[]),
];
const LFO_PAGE: usize = 5;

/// The settings as they are now: as the LFOs move them while it plays, else as set.
fn moved(p: &[f32], live: &InstLive) -> Vec<f32> {
    if live.vals[4] > 0.0 && live.vals.len() >= VIS_PARAMS + SPECS.len() {
        live.vals[VIS_PARAMS..VIS_PARAMS + SPECS.len()].to_vec()
    } else {
        p.to_vec()
    }
}

/// Breaks' panel. Returns a message for the player (a break that would not load, say).
#[allow(clippy::too_many_arguments)]
pub(crate) fn breaks_panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut Vec<f32>, sample: &mut Option<SampleRef>, live: &InstLive, height: f32, mut mv: Option<&mut ModView>) -> Option<String> {
    if params.len() < SPECS.len() {
        return None;
    }
    br::warm();
    let mut said = None;
    let mut load = |path: &std::path::Path, sample: &mut Option<SampleRef>| match SampleRef::load(path) {
        Ok(s) => {
            let _ = s.analysis();
            *sample = Some(s);
        }
        Err(e) => said = Some(format!("Could not load the break: {e}")),
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new("Preset").small().color(crate::skin::ink_dim()));
        egui::ComboBox::from_id_salt(id.with("preset")).width(120.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
            for name in preset_names() {
                if ui.selectable_label(preset == name, *name).clicked() {
                    *preset = name.to_string();
                    let seed = params[SEED];
                    *params = br::preset(name);
                    params[SEED] = seed;
                }
            }
        });
        if ui.button("Load break...").on_hover_text("A drum loop (WAV, FLAC, MP3 or AIFF) to cut up, or drop one on the break").clicked() {
            if let Some(path) = crate::import::audio_file_dialog().pick_file() {
                load(&path, sample);
            }
        }
        if let Some(k) = library(ui, id, sample) {
            fetch(ui, id, k);
        }
        if let Some(s) = sample.as_ref() {
            ui.label(RichText::new(format!("{:.2} s", s.duration())).small().color(crate::skin::ink_dim()));
        }
        // The dice: another throw (right-click: the one before).
        let seed = params[SEED].round() as i32;
        let resp = ui.button(format!("Dice #{seed}")).on_hover_text("Throw the dice again: another way to cut it (right-click: the one before)");
        if resp.clicked() {
            params[SEED] = ((seed + 1) % 1000) as f32;
        } else if resp.secondary_clicked() {
            params[SEED] = ((seed + 999) % 1000) as f32;
        }
    });
    // A break from the library arriving: it plays, cut on its own bars.
    let mut lib_err = None;
    let key = id.with("lib_fetch");
    if let Some((k, state)) = ui.data(|d| d.get_temp::<(usize, crate::breaks_lib::Fetch)>(key)) {
        let now = state.lock().map(|s| s.clone()).unwrap_or(crate::breaks_lib::State::Downloading);
        match now {
            crate::breaks_lib::State::Downloading => ui.ctx().request_repaint_after(std::time::Duration::from_millis(100)),
            crate::breaks_lib::State::Ready => {
                ui.data_mut(|d| d.remove::<(usize, crate::breaks_lib::Fetch)>(key));
                let b = &crate::breaks_lib::BREAKS[k];
                load(&b.path(), sample);
                if sample.as_ref().is_some_and(|s| s.path.as_deref() == Some(b.path().as_path())) {
                    params[BARS] = b.bars_setting();
                }
            }
            crate::breaks_lib::State::Failed(e) => {
                ui.data_mut(|d| d.remove::<(usize, crate::breaks_lib::Fetch)>(key));
                lib_err = Some(format!("Could not get the break: {e}"));
            }
        }
    }
    let playing = sample.clone();
    let src: &SampleData = match playing.as_ref() {
        Some(s) => s,
        None => br::builtin(),
    };
    let h = (height - 30.0).max(120.0);
    ui.horizontal_top(|ui| {
        let key = id.with("screen_w");
        let mut w = ui.data_mut(|d| *d.get_persisted_mut_or(key, 300.0f32));
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
        let before = w;
        width_grip(ui, id.with("screen_grip"), rect, &mut w, 240.0, 1400.0);
        if w != before {
            ui.data_mut(|d| d.insert_persisted(key, w));
        }
        screen(ui, rect, params, src, live);
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
        switches(ui, id, params, h);
        ui.add_space(8.0);
        faders(ui, id, params, live, h, mv.as_deref_mut());
    });
    if live.vals[4] > 0.0 {
        ui.ctx().request_repaint();
    }
    said.or(lib_err)
}

/// The break and its cuts above, the bar as it plays below.
fn screen(ui: &mut Ui, rect: Rect, p: &[f32], src: &SampleData, live: &InstLive) {
    let painter = ui.painter_at(rect);
    rect_fill(&painter, rect, 6.0, SCREEN_BG);
    let frames = src.frames().max(2);
    let n = steps_per_bar(p);
    let bars = bars_of(p, src);
    let cuts = n * bars;
    let bar = live.vals[2] as i64;
    let now = live.vals[0];
    let now_p = moved(p, live);
    let plan = plan(&now_p, bars, bar);
    let ink = crate::skin::ink_dim();
    let inner = rect.shrink(8.0);
    let wave_r = Rect::from_min_max(inner.min, pos2(inner.right(), inner.top() + inner.height() * 0.42));
    let grid_r = Rect::from_min_max(pos2(inner.left(), wave_r.bottom() + 18.0), inner.max);
    // The cut playing.
    let playing_cut = (now >= 0.0).then(|| plan.get(now as usize).map(|s| s.cut)).flatten();
    if let Some(c) = playing_cut {
        let (a, b) = cut_bounds(src, c, cuts, frames);
        let x0 = wave_r.left() + a as f32 / frames as f32 * wave_r.width();
        let x1 = wave_r.left() + b as f32 / frames as f32 * wave_r.width();
        rect_fill(&painter, Rect::from_x_y_ranges(x0..=x1.max(x0 + 2.0), wave_r.y_range()), 2.0, LIT.gamma_multiply(0.25));
    }
    // The wave.
    let cols = wave_r.width() as usize;
    let mid = wave_r.center().y;
    let peaks = &src.peaks;
    for x in 0..cols {
        let a = x * peaks.len() / cols.max(1);
        let b = ((x + 1) * peaks.len() / cols.max(1)).max(a + 1).min(peaks.len());
        let (lo, hi) = peaks[a.min(peaks.len().saturating_sub(1))..b].iter().fold((0.0f32, 0.0f32), |(lo, hi), (l, h)| (lo.min(*l), hi.max(*h)));
        let px = wave_r.left() + x as f32 + 0.5;
        let hh = wave_r.height() * 0.5;
        painter.line_segment([pos2(px, mid - hi.min(1.0) * hh), pos2(px, mid - lo.max(-1.0) * hh)], Stroke::new(1.0_f32, WAVE));
    }
    // The cuts: each a line where it starts (on the hit nearest its place), bars brighter.
    for k in 0..cuts {
        let (a, _) = cut_bounds(src, k, cuts, frames);
        let x = wave_r.left() + a as f32 / frames as f32 * wave_r.width();
        let strong = k % n == 0;
        let c = if strong { ink } else { ink.gamma_multiply(0.35) };
        painter.line_segment([pos2(x, wave_r.top()), pos2(x, wave_r.bottom())], Stroke::new(if strong { 1.2_f32 } else { 0.6 }, c));
    }
    let phrase = match p[PHRASE].round() as usize {
        0 => bars,
        k => [1, 1, 2, 4, 8, 16][k.min(5)],
    };
    let label = if live.vals[4] > 0.0 { format!("bar {} of {}  ({}-bar break, {} cuts)", bar.rem_euclid(phrase as i64) + 1, phrase, bars, cuts) } else { format!("a {}-bar break, {} cuts: plays while the song plays", bars, cuts) };
    text(&painter, pos2(inner.left(), wave_r.bottom() + 9.0), Align2::LEFT_CENTER, label, small(), ink);
    // The bar: a column a step.
    let sw = grid_r.width() / n as f32;
    let accent = accent_now();
    for (k, s) in plan.iter().enumerate() {
        let x0 = grid_r.left() + k as f32 * sw;
        let col = Rect::from_min_size(pos2(x0, grid_r.top()), vec2(sw - 1.5, grid_r.height()));
        let on = now >= 0.0 && now as usize == k;
        rect_fill(&painter, col, 2.0, if k % 4 == 0 { Color32::from_white_alpha(14) } else { Color32::from_white_alpha(6) });
        if !s.play {
            continue;
        }
        // As high as where its cut is in the break; a half spans two steps.
        let w = if s.half { sw * 2.0 - 1.5 } else { sw - 1.5 };
        let frac = (s.cut % cuts.max(1)) as f32 / cuts.max(1) as f32;
        let bh = (grid_r.height() - 16.0) * (0.12 + 0.88 * (1.0 - frac));
        let br_r = Rect::from_min_size(pos2(x0, grid_r.bottom() - bh), vec2(w, bh));
        let natural = s.cut == (bar.rem_euclid(bars as i64) as usize * n + k) % cuts;
        let base = if s.reverse { BACKWARD } else if natural { accent.gamma_multiply(0.55) } else { accent };
        rect_fill(&painter, br_r, 2.0, if on { LIT } else { base });
        // Rolls: a tick a hit.
        if s.ratchet > 1 {
            for j in 1..s.ratchet {
                let y = br_r.top() + br_r.height() * j as f32 / s.ratchet as f32;
                painter.line_segment([pos2(br_r.left() + 1.0, y), pos2(br_r.right() - 1.0, y)], Stroke::new(1.0_f32, SCREEN_BG));
            }
            if sw >= 16.0 {
                text(&painter, pos2(br_r.center().x, br_r.top() - 6.0), Align2::CENTER_CENTER, format!("{}", s.ratchet), small(), ink);
            }
        }
        // Pitched: a dot above (up) or below (down) the middle of the top row.
        if s.semis != 0.0 {
            let y = grid_r.top() + 4.0 + if s.semis > 0.0 { 0.0 } else { 6.0 };
            painter.circle_filled(pos2(br_r.center().x, y), 2.5, if s.semis > 0.0 { UP } else { BACKWARD });
        }
    }
    let _ = ui;
}

/// The choices: how many bars, the grid, the most hits a roll, the fill, how often it mutates, and the dice.
fn switches(ui: &mut Ui, id: Id, p: &mut [f32], h: f32) {
    let w = 160.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let painter = ui.painter().clone();
    let ink = crate::skin::ink_dim();
    let accent = accent_now();
    let row_h = ((h - 6.0 * 13.0 - 6.0 * 4.0) / 6.0).clamp(13.0, 26.0);
    let mut y = rect.top();
    let row = |ui: &mut Ui, p: &mut [f32], y: &mut f32, title: &str, pi: usize, names: &[&str], tips: &[&str]| {
        text(&painter, pos2(rect.left(), *y + 6.0), Align2::LEFT_CENTER, title, small(), ink);
        *y += 13.0;
        let bw = w / names.len() as f32;
        for (k, name) in names.iter().enumerate() {
            let r = Rect::from_min_size(pos2(rect.left() + k as f32 * bw, *y), vec2(bw - 3.0, row_h));
            let resp = button(ui, r, id.with((pi, k)), name, p[pi].round() as usize == k, accent);
            let resp = match tips.get(k) {
                Some(t) => resp.on_hover_text(*t),
                None => resp,
            };
            if resp.clicked() {
                p[pi] = k as f32;
            }
        }
        *y += row_h + 4.0;
    };
    row(ui, p, &mut y, "Bars", BARS, BAR_NAMES, &["As long as the break is at the song's tempo"]);
    // The grid, coarse to fine (1/8 is the last choice of the three).
    text(&painter, pos2(rect.left(), y + 6.0), Align2::LEFT_CENTER, "Grid", small(), ink);
    y += 13.0;
    let bw = w / 3.0;
    for (k, (v, name, tip)) in [(2usize, "1/8", "Cut on eighths: eight cuts a bar"), (0, "1/16", "Cut on sixteenths"), (1, "1/32", "Cut on thirty-seconds")].into_iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + k as f32 * bw, y), vec2(bw - 3.0, row_h));
        if button(ui, r, id.with(("grid", v)), name, p[GRID].round() as usize == v, accent).on_hover_text(tip).clicked() {
            p[GRID] = v as f32;
        }
    }
    y += row_h + 4.0;
    row(ui, p, &mut y, "Most hits a roll", RATCHET_MAX, RATCHET_NAMES, &[]);
    row(ui, p, &mut y, "Fill", FILL, &["Off", "2", "4", "8"], &["No fill", "A roll to close every 2 bars", "Every 4 bars", "Every 8 bars"]);
    row(ui, p, &mut y, "Mutate", MUTATE, &["Stay", "1", "2", "4"], &["The cut-up stays as it is (the faders say how much)", "Cut it up anew every bar", "Every 2 bars", "Every 4 bars"]);
    row(ui, p, &mut y, "Phrase", PHRASE, &["Brk", "1", "2", "4", "8", "16"], &["The pattern as long as the break", "A pattern of 1 bar, over and over", "2 bars, then round again", "4 bars", "8 bars", "16 bars"]);
    let _ = y;
}

/// The faders of one page, the pages picked from a column beside them.
fn faders(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, h: f32, mut mv: Option<&mut ModView>) {
    let tab_key = id.with("page");
    let mut page = ui.data(|d| d.get_temp::<usize>(tab_key).unwrap_or(0)).min(PAGES.len() - 1);
    let fw = 44.0;
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
    let now_p = moved(p, live);
    // A fader an LFO moves shows where it is now, a line across it.
    let show_moved = |ui: &Ui, track: Rect, set: f32, now: f32| {
        if (set - now).abs() > 1e-3 {
            let y = track.bottom() - now * track.height();
            ui.painter().line_segment([pos2(track.left() - 5.0, y), pos2(track.right() + 5.0, y)], Stroke::new(2.0_f32, LIT));
        }
    };
    if page == LFO_PAGE {
        lfos(ui, id, p, Rect::from_min_max(pos2(rect.left() + tab_w, rect.top()), rect.max), mv.as_deref_mut());
        return;
    }
    for (k, &pi) in PAGES[page].1.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + tab_w + k as f32 * fw, rect.top()), vec2(fw - 4.0, h));
        let spec = &SPECS[pi];
        let mut v = p[pi];
        let (changed, track) = param_slider(ui, r, id.with(("fader", pi)), spec, &mut v, accent);
        if changed {
            p[pi] = v;
        }
        show_moved(ui, track, SPECS[pi].to_norm(p[pi]), SPECS[pi].to_norm(now_p[pi]));
        if let Some(mv) = mv.as_deref_mut() {
            slider_overlay(ui, r, track, id.with(("mod", pi)), spec, p[pi], pi, mv);
        }
    }
}

/// The two LFOs side by side: what each moves, how slowly, its shape, and how far (a fader; below the middle it moves the other way).
fn lfos(ui: &mut Ui, id: Id, p: &mut [f32], rect: Rect, mut mv: Option<&mut ModView>) {
    let accent = accent_now();
    let cw = rect.width() / 2.0;
    for (j, first) in LFOS.into_iter().enumerate() {
        let col = Rect::from_min_size(pos2(rect.left() + j as f32 * cw, rect.top()), vec2(cw - 6.0, rect.height()));
        let mut combos_bottom = col.top();
        ui.scope_builder(egui::UiBuilder::new().max_rect(col).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            let on = p[first].round() as usize > 0;
            ui.label(RichText::new(format!("LFO {}", j + 1)).small().color(if on { accent } else { crate::skin::ink_dim() }));
            for (k, names, tip) in [
                (first, LFO_TARGETS, "What it moves"),
                (first + 1, LFO_RATES, "How long one sweep takes, on the song's beat"),
                (first + 2, LFO_SHAPES, "Its shape (Random: a new value every sweep)"),
            ] {
                let cur = (p[k].round().max(0.0) as usize).min(names.len() - 1);
                egui::ComboBox::from_id_salt(id.with(("lfo", k))).width(col.width() - 8.0).selected_text(names[cur]).show_ui(ui, |ui| {
                    for (i, name) in names.iter().enumerate() {
                        if ui.selectable_label(i == cur, *name).clicked() {
                            p[k] = i as f32;
                        }
                    }
                })
                .response
                .on_hover_text(tip);
            }
            combos_bottom = ui.min_rect().bottom();
        });
        let pi = first + 3;
        let r = Rect::from_min_max(pos2(col.left(), combos_bottom + 6.0), pos2(col.left() + 40.0, col.bottom()));
        let spec = &SPECS[pi];
        let mut v = p[pi];
        let (changed, track) = param_slider(ui, r, id.with(("fader", pi)), spec, &mut v, accent);
        if changed {
            p[pi] = v;
        }
        if let Some(mv) = mv.as_deref_mut() {
            slider_overlay(ui, r, track, id.with(("mod", pi)), spec, p[pi], pi, mv);
        }
        // Its shape, drawn beside the fader.
        let shape_r = Rect::from_min_max(pos2(r.right() + 6.0, r.top() + 14.0), pos2(col.right() - 2.0, (r.top() + 60.0).min(col.bottom())));
        if shape_r.width() > 10.0 && shape_r.height() > 10.0 {
            let shape = p[first + 2].round().max(0.0) as usize;
            let pts: Vec<_> = (0..=40)
                .map(|i| {
                    let t = i as f64 / 40.0;
                    let cyc = (t * 2.0).floor() as i64;
                    let v = lfo_shape(shape, (t * 2.0).fract(), cyc) * p[pi].signum().max(-1.0) * if p[pi] == 0.0 { 0.0 } else { 1.0 };
                    pos2(shape_r.left() + t as f32 * shape_r.width(), shape_r.center().y - v * shape_r.height() * 0.45)
                })
                .collect();
            let c = if p[first].round() as usize > 0 { accent } else { crate::skin::ink_dim() };
            ui.painter().add(egui::Shape::line(pts, Stroke::new(1.5_f32, c)));
        }
    }
}

/// The library break `sample` is, if it is one.
fn lib_index(sample: &Option<SampleRef>) -> Option<usize> {
    let path = sample.as_ref()?.path.clone()?;
    crate::breaks_lib::BREAKS.iter().position(|b| b.path() == path)
}

/// Starts getting library break `k` (it loads when it is here).
fn fetch(ui: &Ui, id: Id, k: usize) {
    ui.data_mut(|d| d.insert_temp(id.with("lib_fetch"), (k, crate::breaks_lib::start(k))));
}

/// The breaks to pick from: its own, and the library's (those not here yet
/// are downloaded when picked), with a step back and on through them.
/// Returns the library break picked; Own sets `sample` itself.
fn library(ui: &mut Ui, id: Id, sample: &mut Option<SampleRef>) -> Option<usize> {
    use crate::breaks_lib::{Group, BREAKS};
    let now = lib_index(sample);
    let getting = ui.data(|d| d.get_temp::<(usize, crate::breaks_lib::Fetch)>(id.with("lib_fetch"))).map(|(k, _)| k);
    let mut picked = None;
    let shown = match (getting, now, sample.as_ref()) {
        (Some(k), _, _) => format!("{}...", BREAKS[k].name),
        (None, Some(k), _) => BREAKS[k].name.to_string(),
        (None, None, Some(s)) => s.name.clone(),
        (None, None, None) => "Own break".to_string(),
    };
    let at = getting.or(now);
    if ui.small_button("<").on_hover_text("The break before, from the library").clicked() {
        picked = Some(at.map_or(BREAKS.len() - 1, |k| (k + BREAKS.len() - 1) % BREAKS.len()));
    }
    egui::ComboBox::from_id_salt(id.with("library")).width(150.0).selected_text(shown).height(520.0).show_ui(ui, |ui| {
        if ui.selectable_label(sample.is_none() && getting.is_none(), "Own break").on_hover_text("Its own break: two bars at 170").clicked() {
            *sample = None;
            ui.data_mut(|d| d.remove::<(usize, crate::breaks_lib::Fetch)>(id.with("lib_fetch")));
        }
        for g in Group::ALL {
            ui.separator();
            ui.label(RichText::new(g.name()).small().color(crate::skin::ink_dim()));
            for (k, b) in BREAKS.iter().enumerate().filter(|(_, b)| b.group == g) {
                let label = format!("{}{}  {}", if b.have() { "" } else { "\u{2193} " }, b.name, if b.bars > 0 { format!("{} \u{00b7} {:.0}", b.bars, b.bpm) } else { format!("{:.0}", b.bpm) });
                if ui.selectable_label(now == Some(k), label).on_hover_text(b.credit()).clicked() {
                    picked = Some(k);
                }
            }
        }
    })
    .response
    .on_hover_text("A break to cut up: its own, or a free one from the library (an arrow: downloaded when picked, into Documents/Krabton/Library/Breaks)");
    if ui.small_button(">").on_hover_text("The next break, from the library").clicked() {
        picked = Some(at.map_or(0, |k| (k + 1) % BREAKS.len()));
    }
    picked
}
