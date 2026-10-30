//! The Granulator: its sample drawn large, with where the grains come from
//! (Position, a handle on top) and how far they spray about it (the lit
//! band); every grain a light where it reads in the sample, as high as its
//! pitch, amber forwards and blue backwards, leaving a short trail. Drag in
//! it: left and right moves Position, up and down widens or narrows the
//! spray (or grab the band's edges). Beside it the windows and scan modes as
//! buttons, and the rest on faders, a page at a time. A sample dropped on it
//! (from the browser or from outside) is what it plays.

use super::inst_screen::{InstLive, SCREEN_BG};
use super::modulation::{slider_overlay, ModView};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, RichText, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::granulator::{self as gr, *};
use krabton_engine::sample::{SampleData, SampleRef};

const FORWARD: Color32 = Color32::from_rgb(0xff, 0xb3, 0x47);
const BACKWARD: Color32 = Color32::from_rgb(0x6f, 0xb4, 0xff);
const WAVE: Color32 = Color32::from_rgb(0x3a, 0x44, 0x5a);
const BAND: Color32 = Color32::from_rgb(0x2a, 0x55, 0x70);
const HEAD: Color32 = Color32::from_rgb(0x9f, 0xd8, 0xff);

/// The pages of faders: a name and the parameters on it.
const PAGES: &[(&str, &[usize])] = &[
    ("Grains", &[SIZE, DENSITY, JITTER, REVERSE, SPRAY, SPEED]),
    ("Pitch", &[PITCH, FINE, PITCH_SPRAY, OCTAVES, ROOT]),
    ("Amp", &[ATTACK, DECAY, SUSTAIN, RELEASE, VELOCITY, VEL_DENSITY]),
    ("Space", &[TONE, SMEAR, CLOUD, SPREAD, VOICES, VOLUME]),
];

/// The Granulator's panel. Returns a message for the player (a sample that
/// would not load, say).
#[allow(clippy::too_many_arguments)]
pub(crate) fn granulator_panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut Vec<f32>, sample: &mut Option<SampleRef>, live: &InstLive, height: f32, mut mv: Option<&mut ModView>) -> Option<String> {
    if params.len() < SPECS.len() {
        return None;
    }
    gr::warm();
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
                    *params = gr::preset(name);
                }
            }
        });
        if ui.button("Load sample...").on_hover_text("A WAV or FLAC file to play as grains (or drop one on the sample)").clicked() {
            if let Some(path) = rfd::FileDialog::new().add_filter("Samples", &["wav", "flac", "WAV", "FLAC"]).pick_file() {
                load(&path, sample);
            }
        }
        // Its own sounds, or the sample loaded (a click on one of them goes back to it).
        ui.label(RichText::new("Own").small().color(crate::skin::ink_dim()));
        for (k, name) in SOURCES.iter().enumerate() {
            let on = sample.is_none() && params[SOURCE].round() as usize == k;
            if ui.selectable_label(on, *name).on_hover_text("Play the Granulator's own sound").clicked() {
                params[SOURCE] = k as f32;
                *sample = None;
            }
        }
        if let Some(s) = sample.as_ref() {
            ui.label(RichText::new(format!("{}  {:.1} s", s.name, s.duration())).color(accent_now()));
        }
    });
    // What it plays now (a sample dropped below replaces it from the next frame).
    let playing = sample.clone();
    let Some(src) = gr::source(params, playing.as_ref()) else { return said };
    let h = (height - 30.0).max(120.0);
    ui.horizontal_top(|ui| {
        let key = id.with("screen_w");
        let mut w = ui.data_mut(|d| *d.get_persisted_mut_or(key, 380.0f32));
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click_and_drag());
        let before = w;
        width_grip(ui, id.with("screen_grip"), rect, &mut w, 260.0, 1400.0);
        if w != before {
            ui.data_mut(|d| d.insert_persisted(key, w));
        }
        screen(ui, id, rect, &resp, params, src, live);
        // A sample from the browser, or a file from outside.
        if let Some(item) = drop_payload::<crate::app::BrowserItem>(&resp) {
            if let crate::app::BrowserItem::Sample(path) = &*item {
                load(path, sample);
            }
        }
        if resp.dnd_hover_payload::<crate::app::BrowserItem>().is_some() {
            rect_line(ui.painter(), rect, 6.0, 2.0, accent_now());
        }
        let dropped: Vec<std::path::PathBuf> = ui.ctx().input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if let Some(path) = dropped.iter().find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("wav") || e.eq_ignore_ascii_case("flac"))) {
            load(path, sample);
        }
        ui.add_space(8.0);
        switches(ui, id, params, h);
        ui.add_space(8.0);
        faders(ui, id, params, h, mv.as_deref_mut());
    });
    if live.vals[0] > 0.0 {
        ui.ctx().request_repaint();
    }
    said
}

/// A grain as the engine tells of it: its place in the sample, its pitch,
/// how loud it is now, and whether it plays backwards.
struct Seen {
    place: f32,
    semis: f32,
    level: f32,
    back: bool,
}

fn seen(vals: &[f32]) -> Vec<Seen> {
    (0..SHOWN)
        .filter_map(|k| {
            let v = vals[SHOWN_AT + 2 * k];
            if v < 0.0 {
                return None;
            }
            let whole = v.floor();
            Some(Seen { place: v - whole, semis: vals[SHOWN_AT + 2 * k + 1], level: (whole % 16.0) / 15.0, back: whole >= 16.0 })
        })
        .collect()
}

/// The screen: the sample, the band the grains come from, the voices'
/// places, the grains and their trails. Dragging moves Position and Spray.
fn screen(ui: &mut Ui, id: Id, rect: Rect, resp: &egui::Response, p: &mut [f32], src: &SampleData, live: &InstLive) {
    let inner = rect.shrink2(vec2(10.0, 16.0));
    let x_of = |f: f32| inner.left() + f.clamp(0.0, 1.0) * inner.width();
    let spray = p[SPRAY].clamp(0.0, 1.0) * 0.5;
    // Dragging: the band's edges change Spray; anywhere else left and right
    // is Position and up and down Spray.
    let edge_key = id.with("edge_drag");
    if resp.drag_started() {
        let at = resp.interact_pointer_pos().map(|q| q.x).unwrap_or(0.0);
        let c = x_of(p[POSITION]);
        let half = spray * inner.width();
        let edge = spray > 0.0 && ((at - (c - half)).abs() < 8.0 || (at - (c + half)).abs() < 8.0);
        ui.data_mut(|d| d.insert_temp(edge_key, edge));
    }
    if let Some(q) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        let edge = resp.dragged() && ui.data(|d| d.get_temp::<bool>(edge_key).unwrap_or(false));
        if edge {
            p[SPRAY] = ((q.x - x_of(p[POSITION])).abs() / inner.width() * 2.0).clamp(0.0, 1.0);
        } else {
            p[POSITION] = ((q.x - inner.left()) / inner.width()).clamp(0.0, 1.0);
            if resp.dragged() {
                p[SPRAY] = (p[SPRAY] - resp.drag_delta().y / inner.height()).clamp(0.0, 1.0);
            }
        }
    }
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    let painter = ui.painter_at(rect);
    rect_fill(&painter, rect, 6.0, SCREEN_BG);
    // The sample.
    let cols = inner.width().max(1.0) as usize;
    let peaks = &src.peaks;
    let mid = inner.center().y;
    let amp = inner.height() * 0.45;
    if !peaks.is_empty() {
        for c in 0..cols {
            let a = c * peaks.len() / cols;
            let b = ((c + 1) * peaks.len() / cols).max(a + 1).min(peaks.len());
            let (lo, hi) = peaks[a..b].iter().fold((0.0f32, 0.0f32), |(lo, hi), &(mn, mx)| (lo.min(mn), hi.max(mx)));
            let x = inner.left() + c as f32 + 0.5;
            painter.line_segment([pos2(x, mid - hi * amp), pos2(x, mid - lo * amp + 1.0)], Stroke::new(1.0_f32, WAVE));
        }
    }
    // Where each voice's grains come from: the place the scan has reached.
    let mut places: Vec<f32> = (0..8).filter(|&k| live.vals[4 + k] >= 0.0).map(|k| gr::place(p, live.vals[1], live.vals[4 + k])).collect();
    let playing = !places.is_empty();
    if !playing {
        places.push(gr::place(p, live.vals[1], 0.0));
    }
    // The spray band about each, folded back at the ends as the grains are.
    for &c in &places {
        let (a, b) = (c - spray, c + spray);
        let band = |a: f32, b: f32| {
            if b > a {
                painter.rect_filled(Rect::from_min_max(pos2(x_of(a), inner.top()), pos2(x_of(b), inner.bottom())), 0.0, BAND.gamma_multiply(if playing { 0.55 } else { 0.4 }));
            }
        };
        band(a.max(0.0), b.min(1.0));
        if a < 0.0 {
            band(0.0, -a);
        }
        if b > 1.0 {
            band(2.0 - b, 1.0);
        }
        painter.line_segment([pos2(x_of(c), inner.top()), pos2(x_of(c), inner.bottom())], Stroke::new(1.0_f32, HEAD.gamma_multiply(0.6)));
    }
    // How long a grain is, at its own pitch: a bar from the place on.
    let len = (p[SIZE] * src.sample_rate as f32 / src.frames().max(1) as f32).min(1.0);
    let c = places[0];
    let bar = Rect::from_min_max(pos2(x_of(c), inner.top() - 6.0), pos2(x_of(c + len).max(x_of(c) + 2.0), inner.top() - 3.0));
    painter.rect_filled(bar, 1.0, HEAD.gamma_multiply(0.7));
    // The band's edges, to grab.
    let pc = p[POSITION].clamp(0.0, 1.0);
    for e in [pc - spray, pc + spray] {
        if spray > 0.0 && (0.0..=1.0).contains(&e) {
            painter.line_segment([pos2(x_of(e), inner.top() + 6.0), pos2(x_of(e), inner.bottom() - 6.0)], Stroke::new(1.5_f32, BAND.gamma_multiply(1.6)));
        }
    }
    // Position: a line with a handle on top.
    let px = x_of(pc);
    painter.line_segment([pos2(px, rect.top() + 4.0), pos2(px, inner.bottom())], Stroke::new(2.0_f32, accent_now()));
    painter.add(Shape::convex_polygon(vec![pos2(px - 7.0, rect.top() + 2.0), pos2(px + 7.0, rect.top() + 2.0), pos2(px, rect.top() + 12.0)], accent_now(), Stroke::NONE));
    // Pitch lines: the grains' own pitch in the middle, octaves above and below.
    let y_of = |semis: f32| mid - (semis / 24.0).clamp(-1.0, 1.0) * (inner.height() * 0.5 - 4.0);
    for (s, w) in [(12.0, 0.25), (-12.0, 0.25), (0.0, 0.4)] {
        let y = y_of(s);
        painter.line_segment([pos2(inner.left(), y), pos2(inner.right(), y)], Stroke::new(1.0_f32, Color32::from_white_alpha((w * 80.0) as u8)));
    }
    // The grains, and the trails they leave.
    let trail_key = id.with("trail");
    let dt = ui.input(|i| i.stable_dt).min(0.1);
    let mut trail: Vec<(f32, f32, bool, f32)> = ui.data(|d| d.get_temp(trail_key).unwrap_or_default());
    for t in trail.iter_mut() {
        t.3 -= dt * 2.5;
    }
    trail.retain(|t| t.3 > 0.0);
    let now = seen(&live.vals);
    for g in &now {
        if trail.len() < 400 {
            trail.push((g.place, g.semis, g.back, 0.6 * g.level.max(0.2)));
        }
    }
    for &(place, semis, back, life) in &trail {
        let c = if back { BACKWARD } else { FORWARD };
        painter.circle_filled(pos2(x_of(place), y_of(semis)), 1.5, c.gamma_multiply(life.min(1.0) * 0.6));
    }
    for g in &now {
        let c = if g.back { BACKWARD } else { FORWARD };
        let at = pos2(x_of(g.place), y_of(g.semis));
        let r = 2.5 + g.level * 4.5;
        painter.circle_filled(at, r + 3.0, c.gamma_multiply(0.15 + g.level * 0.2));
        painter.circle_filled(at, r, c.gamma_multiply(0.5 + g.level * 0.5));
    }
    ui.data_mut(|d| d.insert_temp(trail_key, trail));
    // What it is playing, and what dragging does.
    let font = FontId::proportional(11.0);
    text(&painter, pos2(rect.left() + 8.0, rect.top() + 7.0), Align2::LEFT_CENTER, format!("{}  {:.1} s", src.name, src.duration()), font.clone(), Color32::from_gray(170));
    let n = live.vals[0].round() as usize;
    let right = if n > 0 { format!("{n} grains") } else { "play a key".to_string() };
    text(&painter, pos2(rect.right() - 8.0, rect.top() + 7.0), Align2::RIGHT_CENTER, right, font.clone(), Color32::from_gray(170));
    text(&painter, pos2(rect.left() + 8.0, rect.bottom() - 8.0), Align2::LEFT_CENTER, format!("Position {:.0} %   Spray {:.0} %   drag: sideways Position, up and down Spray", p[POSITION] * 100.0, p[SPRAY] * 100.0), font, Color32::from_gray(120));
}

/// The windows, the scan modes and the switches, as buttons.
fn switches(ui: &mut Ui, id: Id, p: &mut [f32], h: f32) {
    let w = 176.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let painter = ui.painter().clone();
    let ink = crate::skin::ink_dim();
    let accent = accent_now();
    // Five rows of buttons and three labels.
    let row_h = ((h - 3.0 * 13.0 - 28.0) / 5.0).clamp(16.0, 30.0);
    let mut y = rect.top();
    let label = |y: &mut f32, s: &str| {
        text(&painter, pos2(rect.left(), *y + 6.0), Align2::LEFT_CENTER, s, small(), ink);
        *y += 13.0;
    };
    // The grain windows, each drawn on its button.
    label(&mut y, "Window");
    let bw = w / SHAPES.len() as f32;
    for (k, name) in SHAPES.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + k as f32 * bw, y), vec2(bw - 3.0, row_h + 6.0));
        let on = p[SHAPE].round() as usize == k;
        let resp = button(ui, r, id.with(("shape", k)), "", on, accent);
        let c = if on { text_on(accent) } else { crate::skin::well().1 };
        let inner = r.shrink2(vec2(4.0, 4.0));
        let pts: Vec<_> = (0..=24).map(|i| i as f32 / 24.0).map(|t| pos2(inner.left() + t * inner.width(), inner.bottom() - gr::window(k, t) * inner.height())).collect();
        ui.painter().add(Shape::line(pts, Stroke::new(1.5_f32, c)));
        if resp.on_hover_text(*name).clicked() {
            p[SHAPE] = k as f32;
        }
    }
    y += row_h + 10.0;
    // How the place moves.
    label(&mut y, "Scan");
    let names = ["Off", "Fwd", "Back", "Ping", "Held"];
    let tips = ["Stays at Position", "Moves forward through the sample", "Moves backward through the sample", "To and fro between the ends", "From Position on while a key is held: the sample stretched in time"];
    for (k, name) in names.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + k as f32 * bw, y), vec2(bw - 3.0, row_h));
        let on = p[SCAN].round() as usize == k;
        if button(ui, r, id.with(("scan", k)), name, on, accent).on_hover_text(tips[k]).clicked() {
            p[SCAN] = k as f32;
        }
    }
    y += row_h + 4.0;
    // Freeze and Keys, big.
    let half = w / 2.0;
    for (k, (pi, name, tip)) in [(FREEZE, "Freeze", "The place stands still"), (KEYS, "Keys", "Keys play the grains at their pitch (off: every key at the sample's own)")].into_iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + k as f32 * half, y), vec2(half - 3.0, row_h));
        let on = p[pi] >= 0.5;
        if button(ui, r, id.with(("switch", pi)), name, on, accent).on_hover_text(tip).clicked() {
            p[pi] = if on { 0.0 } else { 1.0 };
        }
    }
    y += row_h + 4.0;
    // Octave jumps: which way.
    label(&mut y, "Octaves go");
    let third = w / OCTAVE_WAYS.len() as f32;
    for (k, name) in OCTAVE_WAYS.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + k as f32 * third, y), vec2(third - 3.0, row_h));
        if button(ui, r, id.with(("octway", k)), name, p[OCTAVE_WAY].round() as usize == k, accent).clicked() {
            p[OCTAVE_WAY] = k as f32;
        }
    }
    y += row_h + 4.0;
    // Grains on the beat (or free): a click for the next division, a right click for the one before.
    text(&painter, pos2(rect.left(), y + row_h * 0.5), Align2::LEFT_CENTER, "Sync", small(), ink);
    let r = Rect::from_min_size(pos2(rect.left() + 40.0, y), vec2(w - 43.0, row_h));
    let k = (p[SYNC].round().max(0.0) as usize).min(SYNCS.len() - 1);
    let resp = button(ui, r, id.with("sync"), SYNCS[k], k > 0, accent).on_hover_text("Grains on the song's beat: click for the next division, right-click for the one before (Free: Density)");
    if resp.clicked() {
        p[SYNC] = ((k + 1) % SYNCS.len()) as f32;
    } else if resp.secondary_clicked() {
        p[SYNC] = ((k + SYNCS.len() - 1) % SYNCS.len()) as f32;
    }
}

/// The faders of one page, the pages picked from a column beside them.
fn faders(ui: &mut Ui, id: Id, p: &mut [f32], h: f32, mut mv: Option<&mut ModView>) {
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
