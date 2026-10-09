//! Panels of the Lens devices: an EQ with a big interactive curve and a
//! spectrum analyzer, and a compressor and limiter with scrolling level
//! displays, in the spirit of the well-known visual mixing plug-ins.

use crate::theme::*;
use crate::ui::modulation::{cell_overlay, ModView};
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Mesh, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui, UiBuilder, Vec2};
use krabton_engine::dsp::fft::Fft;
use krabton_engine::dsp::{lens_comp as comp, lens_eq as eq, lens_limiter as lim};
use krabton_engine::engine::EngineStatus;
use krabton_engine::params::ParamSpec;
use krabton_engine::scope::PRE_KEY;
use std::cell::RefCell;
use std::collections::VecDeque;

const SCREEN: Color32 = Color32::from_rgb(0x11, 0x14, 0x1a);
const GRID: Color32 = Color32::from_rgb(0x22, 0x27, 0x30);
const GRID_STRONG: Color32 = Color32::from_rgb(0x33, 0x3a, 0x46);
const LABEL: Color32 = Color32::from_rgb(0x6c, 0x74, 0x82);
const CURVE: Color32 = Color32::from_rgb(0xf2, 0xd4, 0x7c);
const SPECTRUM_POST: Color32 = Color32::from_rgb(0x7f, 0xb6, 0xe8);
const SPECTRUM_PRE: Color32 = Color32::from_rgb(0x5a, 0x60, 0x6c);
const GR_RED: Color32 = Color32::from_rgb(0xf0, 0x55, 0x4a);

const F_MIN: f64 = 10.0;
const F_MAX: f64 = 30000.0;
const FFT_N: usize = 4096;

thread_local! {
    static FFT: RefCell<Fft> = RefCell::new(Fft::new(FFT_N));
}

fn band_color(b: usize) -> Color32 {
    Color32::from(egui::ecolor::Hsva::new(b as f32 / eq::BANDS as f32, 0.55, 0.95, 1.0))
}

/// A knob for parameter `pi`, with the modulation overlay when mapping.
#[allow(clippy::too_many_arguments)]
fn knob(ui: &mut Ui, rect: Rect, id: Id, specs: &[ParamSpec], p: &mut [f32], pi: usize, accent: Color32, mv: &mut Option<&mut ModView>) -> bool {
    let changed = param_cell(ui, rect, id.with(pi), &specs[pi], &mut p[pi], accent);
    if let Some(mv) = mv.as_deref_mut() {
        cell_overlay(ui, rect, id.with(("mod", pi)), &specs[pi], p[pi], pi, mv);
    }
    changed
}

/// A combo box for a choice parameter, filling `rect`.
fn choice(ui: &mut Ui, rect: Rect, id: Id, options: &[&str], value: &mut f32) -> bool {
    let mut c = ui.new_child(UiBuilder::new().max_rect(rect));
    let cur = (value.round().max(0.0) as usize).min(options.len() - 1);
    let mut pick = cur;
    egui::ComboBox::from_id_salt(id).width(rect.width()).selected_text(RichText::new(options[cur]).small()).show_ui(&mut c, |ui| {
        for (k, o) in options.iter().enumerate() {
            ui.selectable_value(&mut pick, k, *o);
        }
    });
    if pick != cur {
        *value = pick as f32;
        true
    } else {
        false
    }
}

fn mesh_area(painter: &egui::Painter, top: &[Pos2], base: f32, fill: Color32) {
    if top.len() < 2 {
        return;
    }
    let mut m = Mesh::default();
    for (k, p) in top.iter().enumerate() {
        m.colored_vertex(*p, fill);
        m.colored_vertex(pos2(p.x, base), fill);
        if k > 0 {
            let i = (k * 2) as u32;
            m.add_triangle(i - 2, i - 1, i);
            m.add_triangle(i - 1, i, i + 1);
        }
    }
    painter.add(Shape::mesh(m));
}

fn fmt_freq(f: f64) -> String {
    if f >= 1000.0 {
        format!("{:.2} kHz", f / 1000.0)
    } else {
        format!("{f:.0} Hz")
    }
}

fn note_of(f: f64) -> String {
    let midi = 69.0 + 12.0 * (f / 440.0).log2();
    let n = midi.round().clamp(0.0, 127.0);
    format!("{} {:+.0}c", krabton_engine::note_name(n as u8), (midi - n) * 100.0)
}

// ---------------------------------------------------------------- EQ

#[derive(Clone, Default)]
struct EqView {
    selected: Option<usize>,
    /// Band being dragged.
    drag: Option<usize>,
    /// From where the band was grabbed to its dot, so it does not jump.
    grab: Vec2,
    menu: Option<usize>,
    pre: Vec<f32>,
    post: Vec<f32>,
}

struct Mapping {
    rect: Rect,
    range: f32,
}

impl Mapping {
    fn x(&self, f: f64) -> f32 {
        self.rect.left() + self.rect.width() * ((f / F_MIN).ln() / (F_MAX / F_MIN).ln()) as f32
    }
    fn freq(&self, x: f32) -> f64 {
        F_MIN * (F_MAX / F_MIN).powf(((x - self.rect.left()) / self.rect.width()).clamp(0.0, 1.0) as f64)
    }
    fn y(&self, db: f32) -> f32 {
        self.rect.center().y - db / self.range * (self.rect.height() * 0.5 - 12.0)
    }
    fn db(&self, y: f32) -> f32 {
        (self.rect.center().y - y) / (self.rect.height() * 0.5 - 12.0) * self.range
    }
    /// Analyzer scale: -90..+6 dB over the height.
    fn spec_y(&self, db: f32) -> f32 {
        self.rect.bottom() - ((db + 90.0) / 96.0).clamp(0.0, 1.0) * self.rect.height()
    }
}

/// Where band `b`'s dot sits.
fn node_pos(m: &Mapping, p: &[f32], b: usize) -> Option<Pos2> {
    let shape = eq::Shape::from_param(p[eq::idx(b, eq::SHAPE)]);
    if shape == eq::Shape::Off {
        return None;
    }
    let f = p[eq::idx(b, eq::FREQ)] as f64;
    let g = if shape.has_gain() { p[eq::idx(b, eq::GAIN)] * p[eq::GAIN_SCALE] } else { 0.0 };
    Some(pos2(m.x(f), m.y(g.clamp(-m.range, m.range))))
}

/// How close to a dot a press grabs it.
const GRAB: f32 = 12.0;

/// The band whose dot is nearest to `pt`, if one is within reach.
fn node_at(m: &Mapping, p: &[f32], pt: Pos2) -> Option<usize> {
    (0..eq::BANDS)
        .filter_map(|b| node_pos(m, p, b).map(|n| (b, n.distance(pt))))
        .filter(|(_, d)| *d < GRAB)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(b, _)| b)
}

/// Smoothed analyzer levels of the tap `key`: rise at once, fall slowly.
fn analyze(status: &EngineStatus, key: u64, smooth: &mut Vec<f32>) {
    let Some(buf) = status.scopes.find(key) else {
        smooth.clear();
        return;
    };
    let (mut l, mut r) = (Vec::new(), Vec::new());
    buf.read(FFT_N, &mut l, &mut r);
    let mid: Vec<f32> = l.iter().zip(&r).map(|(a, b)| 0.5 * (a + b)).collect();
    let mut fresh = Vec::new();
    FFT.with(|f| f.borrow_mut().spectrum_db(&mid, &mut fresh));
    if smooth.len() != fresh.len() {
        *smooth = fresh;
        return;
    }
    for (s, n) in smooth.iter_mut().zip(&fresh) {
        *s = if *n > *s { *n } else { (*s - 0.8).max(*n) };
    }
}

/// The analyzer curve as points on the display, with a 4.5 dB/oct tilt so
/// pink noise looks flat.
fn spectrum_points(m: &Mapping, bins: &[f32], sr: f64) -> Vec<Pos2> {
    if bins.is_empty() {
        return Vec::new();
    }
    let per_bin = sr / FFT_N as f64;
    let mut pts = Vec::new();
    let mut x = m.rect.left();
    while x <= m.rect.right() {
        let (f0, f1) = (m.freq(x), m.freq(x + 2.0));
        let (k0, k1) = (f0 / per_bin, f1 / per_bin);
        let db = if k1 - k0 < 1.0 {
            let k = (0.5 * (k0 + k1)).max(0.0);
            let i = (k as usize).min(bins.len() - 2);
            let t = (k - i as f64) as f32;
            bins[i] + (bins[i + 1] - bins[i]) * t
        } else {
            let (a, b) = ((k0 as usize).min(bins.len() - 1), (k1.ceil() as usize).min(bins.len()));
            bins[a..b.max(a + 1)].iter().fold(f32::MIN, |m, v| m.max(*v))
        };
        let tilt = 4.5 * ((0.5 * (f0 + f1)) / 1000.0).log2() as f32;
        pts.push(pos2(x, m.spec_y(db + tilt)));
        x += 2.0;
    }
    pts
}

fn eq_display(ui: &mut Ui, rect: Rect, id: Id, dev_id: u64, p: &mut [f32], status: &EngineStatus, view: &mut EqView) -> bool {
    let sr = status.sample_rate.load() as f64;
    let range = eq::DISPLAY_DB[(p[eq::DISPLAY].round().max(0.0) as usize).min(3)];
    let m = Mapping { rect, range };
    let resp = ui.interact(rect, id.with("display"), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, SCREEN);
    let mut changed = false;

    // Grid.
    let mut f = 10.0;
    while f <= 20000.0 {
        for k in 1..10 {
            let ff = f * k as f64;
            if !(20.0..=20000.0).contains(&ff) {
                continue;
            }
            let x = m.x(ff);
            let strong = k == 1;
            painter.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(1.0_f32, if strong { GRID_STRONG } else { GRID }));
            if k == 1 || k == 2 || k == 5 {
                let label = if ff >= 1000.0 { format!("{}k", ff / 1000.0) } else { format!("{ff}") };
                painter.text(pos2(x + 2.0, rect.bottom() - 2.0), Align2::LEFT_BOTTOM, label, small(), LABEL);
            }
        }
        f *= 10.0;
    }
    let step = if range <= 3.0 { 1.0 } else if range <= 6.0 { 2.0 } else if range <= 12.0 { 3.0 } else { 10.0 };
    let mut g = -range;
    while g <= range + 0.01 {
        let y = m.y(g);
        painter.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(1.0_f32, if g.abs() < 0.01 { GRID_STRONG } else { GRID }));
        painter.text(pos2(rect.right() - 3.0, y - 1.0), Align2::RIGHT_BOTTOM, format!("{g:+.0}"), small(), LABEL);
        g += step;
    }

    // Analyzer.
    let mode = p[eq::ANALYZER].round() as i32;
    if mode == 1 || mode == 3 {
        analyze(status, dev_id | PRE_KEY, &mut view.pre);
        let pts = spectrum_points(&m, &view.pre, sr);
        mesh_area(&painter, &pts, rect.bottom(), SPECTRUM_PRE.gamma_multiply(0.35));
    }
    if mode == 2 || mode == 3 {
        analyze(status, dev_id, &mut view.post);
        let pts = spectrum_points(&m, &view.post, sr);
        mesh_area(&painter, &pts, rect.bottom(), SPECTRUM_POST.gamma_multiply(0.18));
        painter.add(Shape::line(pts, Stroke::new(1.0_f32, SPECTRUM_POST.gamma_multiply(0.6))));
    }

    // Curves: each band designed once, evaluated along the display.
    let dyn_db = status.meters.read(dev_id).unwrap_or([0.0; krabton_engine::scope::METER_VALUES]);
    let designs: Vec<Option<krabton_engine::dsp::biquad::Cascade>> = (0..eq::BANDS)
        .map(|b| {
            eq::band_settings(p, b).map(|(shape, f, g, q, slope, _)| eq::design(shape, f, g + if shape.has_gain() { dyn_db[b] as f64 } else { 0.0 }, q, slope, sr))
        })
        .collect();
    let cols: Vec<(f32, f64)> = {
        let mut v = Vec::new();
        let mut x = rect.left();
        while x <= rect.right() {
            v.push((x, 2.0 * std::f64::consts::PI * m.freq(x).min(0.5 * sr) / sr));
            x += 2.0;
        }
        v
    };
    let hovered_node = resp.hover_pos().and_then(|hp| node_at(&m, p, hp));
    let focus = view.drag.or(hovered_node).or(view.selected);
    if let Some(b) = focus {
        if let Some(Some(d)) = designs.get(b) {
            let pts: Vec<Pos2> = cols.iter().map(|(x, w)| pos2(*x, m.y(d.mag_db(*w) as f32))).collect();
            let c = band_color(b);
            mesh_area(&painter, &pts, m.y(0.0), c.gamma_multiply(0.12));
            painter.add(Shape::line(pts, Stroke::new(1.0_f32, c.gamma_multiply(0.7))));
        }
    }
    let total: Vec<Pos2> = cols.iter().map(|(x, w)| pos2(*x, m.y(designs.iter().flatten().map(|d| d.mag_db(*w)).sum::<f64>() as f32 + 0.0))).collect();
    mesh_area(&painter, &total, m.y(0.0), CURVE.gamma_multiply(0.10));
    painter.add(Shape::line(total, Stroke::new(2.0_f32, CURVE)));

    // Band dots.
    for b in 0..eq::BANDS {
        let Some(pos) = node_pos(&m, p, b) else { continue };
        let c = band_color(b);
        let active = p[eq::idx(b, eq::ACTIVE)] >= 0.5;
        let shape = eq::Shape::from_param(p[eq::idx(b, eq::SHAPE)]);
        let range_db = p[eq::idx(b, eq::DYN_RANGE)];
        if shape.has_gain() && range_db.abs() > 0.01 {
            // Dynamic: how far the band may move, and where it is now.
            let y1 = m.y((p[eq::idx(b, eq::GAIN)] * p[eq::GAIN_SCALE] + range_db).clamp(-range, range));
            painter.line_segment([pos, pos2(pos.x, y1)], Stroke::new(4.0_f32, c.gamma_multiply(0.35)));
            let now = m.y((p[eq::idx(b, eq::GAIN)] * p[eq::GAIN_SCALE] + dyn_db[b]).clamp(-range, range));
            painter.circle_filled(pos2(pos.x, now), 3.5, c);
        }
        let fill = if active { c } else { shade(c, 0.35) };
        let r = if focus == Some(b) { 8.5 } else { 7.0 };
        painter.circle(pos, r, fill, Stroke::new(1.5_f32, if view.selected == Some(b) { Color32::WHITE } else { shade(c, 0.6) }));
        painter.text(pos, Align2::CENTER_CENTER, format!("{}", b + 1), small(), TEXT_DARK);
    }

    // Interaction.
    let fine = ui.input(|i| i.modifiers.shift);
    let alt = ui.input(|i| i.modifiers.alt);
    if resp.drag_started() {
        // egui reports the drag once the pointer has moved a little, so the
        // band is the one where the button went down (a quick drag would
        // otherwise leave a dot grabbed off its centre behind).
        let origin = ui.input(|i| i.pointer.press_origin()).or(resp.hover_pos());
        view.drag = origin.and_then(|o| node_at(&m, p, o));
        if let (Some(b), Some(o)) = (view.drag, origin) {
            view.selected = Some(b);
            view.grab = node_pos(&m, p, b).map(|n| n - o).unwrap_or(Vec2::ZERO);
        }
    }
    if let (Some(b), true) = (view.drag, resp.dragged()) {
        let shape = eq::Shape::from_param(p[eq::idx(b, eq::SHAPE)]);
        let d = resp.drag_delta();
        let (fi, gi, qi) = (eq::idx(b, eq::FREQ), eq::idx(b, eq::GAIN), eq::idx(b, eq::Q));
        if fine {
            p[fi] = (p[fi] as f64 * (F_MAX / F_MIN).powf(d.x as f64 * 0.15 / rect.width() as f64)) as f32;
        } else if let Some(hp) = resp.interact_pointer_pos() {
            p[fi] = m.freq(hp.x + view.grab.x) as f32;
        }
        p[fi] = p[fi].clamp(10.0, 30000.0);
        if shape.has_gain() {
            let scale = p[eq::GAIN_SCALE].max(0.01);
            if fine {
                p[gi] -= d.y / (rect.height() * 0.5) * range * 0.15 / scale;
            } else if let Some(hp) = resp.interact_pointer_pos() {
                p[gi] = m.db(hp.y + view.grab.y) / scale;
            }
            p[gi] = p[gi].clamp(-30.0, 30.0);
        } else {
            p[qi] = (p[qi] * (-d.y * if fine { 0.002 } else { 0.012 }).exp()).clamp(0.025, 40.0);
        }
        changed = true;
    }
    if resp.drag_stopped() {
        view.drag = None;
    }
    if resp.double_clicked() {
        if let Some(b) = hovered_node {
            // Double-click a dot: remove the band.
            let specs = eq::SPECS;
            for k in 0..eq::PER_BAND {
                p[eq::idx(b, k)] = specs[eq::idx(b, k)].default;
            }
            view.selected = None;
        } else if let (Some(hp), Some(b)) = (resp.interact_pointer_pos(), (0..eq::BANDS).find(|&b| eq::Shape::from_param(p[eq::idx(b, eq::SHAPE)]) == eq::Shape::Off)) {
            // Double-click the curve: a new band there.
            let f = m.freq(hp.x);
            let shape = if f < 30.0 {
                eq::Shape::LowCut
            } else if f > 15000.0 {
                eq::Shape::HighCut
            } else {
                eq::Shape::Bell
            };
            let gain = if shape == eq::Shape::Bell { m.db(hp.y) / p[eq::GAIN_SCALE].max(0.01) } else { 0.0 };
            let q = if shape == eq::Shape::Bell { 1.0 } else { 0.7071 };
            p[eq::idx(b, eq::SHAPE)] = shape as i32 as f32;
            p[eq::idx(b, eq::FREQ)] = f as f32;
            p[eq::idx(b, eq::GAIN)] = gain.clamp(-30.0, 30.0);
            p[eq::idx(b, eq::Q)] = q;
            p[eq::idx(b, eq::ACTIVE)] = 1.0;
            view.selected = Some(b);
        }
        changed = true;
    } else if resp.clicked() {
        match hovered_node {
            Some(b) if alt => {
                let a = eq::idx(b, eq::ACTIVE);
                p[a] = if p[a] >= 0.5 { 0.0 } else { 1.0 };
                changed = true;
            }
            Some(b) => view.selected = Some(b),
            None => view.selected = None,
        }
    }
    // Wheel over a dot (or with a band selected) changes its Q.
    if resp.hovered() {
        let wheel = ui.input(|i| i.raw_scroll_delta.y);
        if let Some(b) = hovered_node.or(view.selected) {
            if wheel.abs() > 0.0 {
                let qi = eq::idx(b, eq::Q);
                p[qi] = (p[qi] * 1.12f32.powf(wheel / 50.0)).clamp(0.025, 40.0);
                changed = true;
            }
        }
    }
    if resp.secondary_clicked() {
        view.menu = hovered_node;
    }
    let menu_band = view.menu;
    resp.context_menu(|ui| {
        match menu_band {
            Some(b) => {
                ui.label(RichText::new(format!("Band {}", b + 1)).color(band_color(b)));
                ui.menu_button("Shape", |ui| {
                    for (k, s) in eq::SHAPES.iter().enumerate().skip(1) {
                        if ui.selectable_label(p[eq::idx(b, eq::SHAPE)].round() as usize == k, *s).clicked() {
                            p[eq::idx(b, eq::SHAPE)] = k as f32;
                            changed = true;
                            ui.close();
                        }
                    }
                });
                if eq::Shape::from_param(p[eq::idx(b, eq::SHAPE)]).has_slope() {
                    ui.menu_button("Slope", |ui| {
                        for (k, s) in eq::SLOPES.iter().enumerate() {
                            if ui.selectable_label(p[eq::idx(b, eq::SLOPE)].round() as usize == k, *s).clicked() {
                                p[eq::idx(b, eq::SLOPE)] = k as f32;
                                changed = true;
                                ui.close();
                            }
                        }
                    });
                }
                ui.menu_button("Stereo", |ui| {
                    for (k, s) in eq::STEREO_MODES.iter().enumerate() {
                        if ui.selectable_label(p[eq::idx(b, eq::STEREO)].round() as usize == k, *s).clicked() {
                            p[eq::idx(b, eq::STEREO)] = k as f32;
                            changed = true;
                            ui.close();
                        }
                    }
                });
                let dynamic = p[eq::idx(b, eq::DYN_RANGE)].abs() > 0.01;
                if ui.button(if dynamic { "Make static" } else { "Make dynamic" }).clicked() {
                    p[eq::idx(b, eq::DYN_RANGE)] = if dynamic { 0.0 } else { -6.0 };
                    changed = true;
                    ui.close();
                }
                let active = p[eq::idx(b, eq::ACTIVE)] >= 0.5;
                if ui.button(if active { "Bypass" } else { "Enable" }).clicked() {
                    p[eq::idx(b, eq::ACTIVE)] = if active { 0.0 } else { 1.0 };
                    changed = true;
                    ui.close();
                }
                if ui.button("Delete").clicked() {
                    for k in 0..eq::PER_BAND {
                        p[eq::idx(b, k)] = eq::SPECS[eq::idx(b, k)].default;
                    }
                    changed = true;
                    ui.close();
                }
            }
            None => {
                ui.label("Double-click to add a band");
                if ui.button("Reset all bands").clicked() {
                    for (i, v) in p.iter_mut().enumerate().take(eq::BANDS * eq::PER_BAND) {
                        *v = eq::SPECS[i].default;
                    }
                    changed = true;
                    ui.close();
                }
            }
        }
    });

    // Readout under the pointer, or of the band.
    if let Some(hp) = resp.hover_pos() {
        let text = match hovered_node {
            Some(b) => {
                let shape = eq::Shape::from_param(p[eq::idx(b, eq::SHAPE)]);
                let mut t = format!("{} {}  {}", b + 1, eq::SHAPES[shape as usize], fmt_freq(p[eq::idx(b, eq::FREQ)] as f64));
                if shape.has_gain() {
                    t += &format!("  {:+.1} dB", p[eq::idx(b, eq::GAIN)]);
                }
                t + &format!("  Q {:.2}", p[eq::idx(b, eq::Q)])
            }
            None => format!("{}  {}  {:+.1} dB", fmt_freq(m.freq(hp.x)), note_of(m.freq(hp.x)), m.db(hp.y)),
        };
        painter.text(rect.left_top() + vec2(8.0, 6.0), Align2::LEFT_TOP, text, small(), TEXT);
    } else if view.selected.is_none() && !designs.iter().any(|d| d.is_some()) {
        painter.text(rect.center(), Align2::CENTER_CENTER, "Double-click to add a band", body(), LABEL);
    }
    changed
}

/// Lens EQ: the display and a strip with the selected band and the global
/// settings.
#[allow(clippy::too_many_arguments)]
pub fn eq_panel(ui: &mut Ui, id: Id, dev_id: u64, p: &mut Vec<f32>, accent: Color32, h: f32, mut mv: Option<&mut ModView>, status: &EngineStatus) -> bool {
    if p.len() < eq::SPECS.len() {
        return false;
    }
    let view_id = id.with("eqview");
    let mut view: EqView = ui.ctx().memory_mut(|m| m.data.get_temp::<EqView>(view_id)).unwrap_or_default();
    let mut changed = false;
    ui.horizontal_top(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(620.0, (h - 30.0).max(120.0)), Sense::hover());
        changed |= eq_display(ui, rect, id, dev_id, p, status, &mut view);
        ui.add_space(6.0);
        // Band strip: the selected band on top, the global settings below.
        let (strip, _) = ui.allocate_exact_size(vec2(4.0 * CELL_W, rect.height()), Sense::hover());
        let painter = ui.painter_at(strip);
        let cell = |y: f32, k: usize| Rect::from_min_size(pos2(strip.left() + k as f32 * CELL_W, y), vec2(CELL_W - 2.0, CELL_H - 2.0));
        let quarter = |y: f32, k: usize, n: usize| Rect::from_min_size(pos2(strip.left() + k as f32 * (strip.width() / 4.0), y), vec2(n as f32 * strip.width() / 4.0 - 3.0, 18.0));
        let mut y = strip.top();
        match view.selected {
            Some(b) => {
                let c = band_color(b);
                painter.circle_filled(pos2(strip.left() + 7.0, y + 8.0), 5.0, c);
                painter.text(pos2(strip.left() + 16.0, y + 8.0), Align2::LEFT_CENTER, format!("Band {}", b + 1), small(), TEXT);
                let shape = eq::Shape::from_param(p[eq::idx(b, eq::SHAPE)]);
                changed |= choice(ui, quarter(y, 2, 2), id.with(("stereo", b)), eq::STEREO_MODES, &mut p[eq::idx(b, eq::STEREO)]);
                y += 20.0;
                changed |= choice(ui, quarter(y, 0, 2), id.with(("shape", b)), eq::SHAPES, &mut p[eq::idx(b, eq::SHAPE)]);
                if shape.has_slope() {
                    changed |= choice(ui, quarter(y, 2, 2), id.with(("slope", b)), eq::SLOPES, &mut p[eq::idx(b, eq::SLOPE)]);
                }
                y += 22.0;
                changed |= knob(ui, cell(y, 0), id, eq::SPECS, p, eq::idx(b, eq::FREQ), c, &mut mv);
                if shape.has_gain() {
                    changed |= knob(ui, cell(y, 1), id, eq::SPECS, p, eq::idx(b, eq::GAIN), c, &mut mv);
                }
                changed |= knob(ui, cell(y, 2), id, eq::SPECS, p, eq::idx(b, eq::Q), c, &mut mv);
                changed |= knob(ui, cell(y, 3), id, eq::SPECS, p, eq::idx(b, eq::ACTIVE), c, &mut mv);
                y += CELL_H;
                if shape.has_gain() {
                    changed |= knob(ui, cell(y, 0), id, eq::SPECS, p, eq::idx(b, eq::DYN_RANGE), c, &mut mv);
                    changed |= knob(ui, cell(y, 1), id, eq::SPECS, p, eq::idx(b, eq::DYN_THRESH), c, &mut mv);
                }
            }
            None => {
                painter.text(pos2(strip.left(), y + 8.0), Align2::LEFT_CENTER, "Click a band to edit it,", small(), TEXT_DIM);
                painter.text(pos2(strip.left(), y + 24.0), Align2::LEFT_CENTER, "double-click the curve to add one", small(), TEXT_DIM);
                y += 42.0 + CELL_H;
            }
        }
        // Global: output and gain scale share the row, the views go below.
        changed |= knob(ui, cell(y, 2), id, eq::SPECS, p, eq::OUTPUT, accent, &mut mv);
        changed |= knob(ui, cell(y, 3), id, eq::SPECS, p, eq::GAIN_SCALE, accent, &mut mv);
        y += CELL_H + 2.0;
        painter.text(pos2(strip.left(), y + 9.0), Align2::LEFT_CENTER, "Analyzer", small(), TEXT_DIM);
        changed |= choice(ui, quarter(y, 1, 1), id.with("analyzer"), eq::ANALYZER_MODES, &mut p[eq::ANALYZER]);
        painter.text(pos2(strip.left() + strip.width() * 0.5 + 2.0, y + 9.0), Align2::LEFT_CENTER, "Range", small(), TEXT_DIM);
        changed |= choice(ui, quarter(y, 3, 1), id.with("display"), eq::DISPLAY_RANGES, &mut p[eq::DISPLAY]);
    });
    ui.ctx().memory_mut(|m| m.data.insert_temp(view_id, view));
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    changed
}

// ---------------------------------------------------------------- level history

/// One column of a scrolling level display: input and output peak and gain
/// reduction, in dB.
#[derive(Clone, Default)]
struct History {
    cols: VecDeque<[f32; 3]>,
    pre_seen: u64,
    post_seen: u64,
}

const HISTORY_LEN: usize = 600;

fn db(x: f32) -> f32 {
    20.0 * x.max(1e-6).log10()
}

/// Adds a column for what was played since the last frame.
fn update_history(ui: &Ui, key: Id, status: &EngineStatus, dev_id: u64, gr: f32) -> History {
    let mut h: History = ui.ctx().memory_mut(|m| m.data.get_temp::<History>(key)).unwrap_or_default();
    let peak_since = |buf: Option<&krabton_engine::scope::ScopeBuffer>, seen: &mut u64| {
        let Some(buf) = buf else { return -120.0 };
        let now = buf.written();
        let n = (now.saturating_sub(*seen) as usize).min(buf.len() / 2);
        *seen = now;
        let (mut l, mut r) = (Vec::new(), Vec::new());
        buf.read(n, &mut l, &mut r);
        db(l.iter().chain(&r).fold(0.0f32, |m, v| m.max(v.abs())))
    };
    let pre = peak_since(status.scopes.find(dev_id | PRE_KEY), &mut h.pre_seen);
    let post = peak_since(status.scopes.find(dev_id), &mut h.post_seen);
    h.cols.push_back([pre, post, gr]);
    while h.cols.len() > HISTORY_LEN {
        h.cols.pop_front();
    }
    ui.ctx().memory_mut(|m| m.data.insert_temp(key, h.clone()));
    h
}

/// Draws a history: input (light) and output levels rising from the bottom
/// on a `floor`..0 dB scale, gain reduction hanging from the top.
fn draw_history(painter: &egui::Painter, rect: Rect, h: &History, floor: f32, out_color: Color32) {
    let y = |d: f32| rect.bottom() - ((d - floor) / -floor).clamp(0.0, 1.0) * rect.height();
    let n = h.cols.len();
    let w = rect.width() / HISTORY_LEN as f32;
    let x0 = rect.right() - n as f32 * w;
    let pts = |k: usize| -> Vec<Pos2> { h.cols.iter().enumerate().map(|(i, c)| pos2(x0 + i as f32 * w, y(c[k]))).collect() };
    mesh_area(painter, &pts(0), rect.bottom(), Color32::from_rgb(0x55, 0x5c, 0x68).gamma_multiply(0.7));
    mesh_area(painter, &pts(1), rect.bottom(), out_color.gamma_multiply(0.55));
    let gr: Vec<Pos2> = h.cols.iter().enumerate().map(|(i, c)| pos2(x0 + i as f32 * w, rect.top() + (-c[2] / -floor).clamp(0.0, 1.0) * rect.height())).collect();
    painter.add(Shape::line(gr, Stroke::new(1.5_f32, GR_RED)));
}

fn level_grid(painter: &egui::Painter, rect: Rect, floor: f32) {
    let mut d = 0.0;
    while d >= floor {
        let yy = rect.bottom() - ((d - floor) / -floor) * rect.height();
        painter.line_segment([pos2(rect.left(), yy), pos2(rect.right(), yy)], Stroke::new(1.0_f32, GRID));
        painter.text(pos2(rect.right() - 3.0, yy + 1.0), Align2::RIGHT_TOP, format!("{d:.0}"), small(), LABEL);
        d -= 6.0;
    }
}

/// A vertical gain reduction meter hanging from the top.
fn gr_meter(painter: &egui::Painter, rect: Rect, gr: f32, floor: f32) {
    painter.rect_filled(rect, 2.0, SCREEN);
    let h = (-gr / -floor).clamp(0.0, 1.0) * rect.height();
    painter.rect_filled(Rect::from_min_size(rect.min, vec2(rect.width(), h)), 2.0, GR_RED);
    painter.text(pos2(rect.center().x, rect.bottom() - 2.0), Align2::CENTER_BOTTOM, format!("{gr:.1}"), small(), TEXT);
}

// ---------------------------------------------------------------- compressor

/// Lens Comp: level display with gain reduction, the transfer curve, and
/// the knobs.
#[allow(clippy::too_many_arguments)]
pub fn comp_panel(ui: &mut Ui, id: Id, dev_id: u64, p: &mut Vec<f32>, accent: Color32, h: f32, mv: Option<&mut ModView>, status: &EngineStatus) -> bool {
    if p.len() < comp::SPECS.len() {
        return false;
    }
    let meters = status.meters.read(dev_id).unwrap_or([0.0; krabton_engine::scope::METER_VALUES]);
    let gr = meters[comp::M_GR];
    let mut changed = false;
    ui.horizontal_top(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(420.0, (h - 30.0).max(120.0)), Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, SCREEN);
        let floor = -60.0;
        let hist = update_history(ui, id.with("history"), status, dev_id, gr);
        let levels = Rect::from_min_max(rect.min, pos2(rect.right() - 26.0, rect.bottom()));
        level_grid(&painter, levels, floor);
        draw_history(&painter, levels, &hist, floor, SPECTRUM_POST);
        // Threshold.
        let ty = levels.bottom() - ((p[comp::THRESHOLD] - floor) / -floor) * levels.height();
        painter.extend(Shape::dashed_line(&[pos2(levels.left(), ty), pos2(levels.right(), ty)], Stroke::new(1.0_f32, CURVE.gamma_multiply(0.8)), 6.0, 4.0));
        // Transfer curve inset.
        let side = (rect.height() * 0.42).min(110.0);
        let inset = Rect::from_min_size(rect.left_top() + vec2(8.0, 8.0), vec2(side, side));
        painter.rect_filled(inset, 3.0, SCREEN.gamma_multiply(0.9));
        painter.rect_stroke(inset, 3.0, Stroke::new(1.0_f32, GRID_STRONG), egui::StrokeKind::Inside);
        let style = (p[comp::STYLE].round().max(0.0) as usize).min(comp::STYLES.len() - 1);
        let knee = p[comp::KNEE].max([0.0, 3.0, 12.0, 18.0, 6.0, 6.0, 3.0, 0.0][style]);
        let map = |d: f32| ((d - floor) / -floor).clamp(0.0, 1.0);
        let curve: Vec<Pos2> = (0..=60)
            .map(|k| {
                let x = floor + k as f32;
                let yv = x + comp::gain_computer(x, p[comp::THRESHOLD], p[comp::RATIO], knee).max(-p[comp::RANGE]);
                pos2(inset.left() + map(x) * inset.width(), inset.bottom() - map(yv) * inset.height())
            })
            .collect();
        painter.line_segment([inset.left_bottom(), inset.right_top()], Stroke::new(1.0_f32, GRID_STRONG));
        painter.add(Shape::line(curve, Stroke::new(1.5_f32, CURVE)));
        if let Some(c) = hist.cols.back() {
            let x = c[0];
            let yv = x + c[2];
            painter.circle_filled(pos2(inset.left() + map(x) * inset.width(), inset.bottom() - map(yv) * inset.height()), 3.0, Color32::WHITE);
        }
        gr_meter(&painter, Rect::from_min_max(pos2(rect.right() - 22.0, rect.top() + 4.0), pos2(rect.right() - 4.0, rect.bottom() - 4.0)), gr, -30.0);
        ui.add_space(6.0);
        changed = crate::ui::devices::param_grid_mod(ui, id, comp::SPECS, p, accent, h - 24.0, mv);
    });
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    changed
}

// ---------------------------------------------------------------- limiter

/// Lens Limiter: level display with gain reduction, loudness readouts and
/// the knobs.
#[allow(clippy::too_many_arguments)]
pub fn limiter_panel(ui: &mut Ui, id: Id, dev_id: u64, p: &mut Vec<f32>, accent: Color32, h: f32, mv: Option<&mut ModView>, status: &EngineStatus) -> bool {
    if p.len() < lim::SPECS.len() {
        return false;
    }
    let meters = status.meters.read(dev_id).unwrap_or([0.0; krabton_engine::scope::METER_VALUES]);
    let gr = meters[lim::M_GR];
    let mut changed = false;
    ui.horizontal_top(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(400.0, (h - 30.0).max(120.0)), Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, SCREEN);
        let floor = -36.0;
        let hist = update_history(ui, id.with("history"), status, dev_id, gr);
        let levels = Rect::from_min_max(rect.min, pos2(rect.right() - 26.0, rect.bottom()));
        level_grid(&painter, levels, floor);
        draw_history(&painter, levels, &hist, floor, Color32::from_rgb(0x6f, 0xc7, 0xb4));
        let cy = levels.bottom() - ((p[lim::CEILING] - floor) / -floor) * levels.height();
        painter.extend(Shape::dashed_line(&[pos2(levels.left(), cy), pos2(levels.right(), cy)], Stroke::new(1.0_f32, CURVE.gamma_multiply(0.8)), 6.0, 4.0));
        gr_meter(&painter, Rect::from_min_max(pos2(rect.right() - 22.0, rect.top() + 4.0), pos2(rect.right() - 4.0, rect.bottom() - 4.0)), gr, -18.0);
        ui.add_space(6.0);
        // Loudness readouts.
        let (lr, _) = ui.allocate_exact_size(vec2(150.0, rect.height()), Sense::hover());
        let lp = ui.painter_at(lr);
        lp.rect_filled(lr, 4.0, SCREEN);
        let show = |v: f32| if v <= -119.0 { "-inf".to_string() } else { format!("{v:.1}") };
        let rows = [
            ("Momentary", show(meters[lim::M_MOMENTARY]), "LUFS"),
            ("Short-term", show(meters[lim::M_SHORT]), "LUFS"),
            ("Integrated", show(meters[lim::M_INTEGRATED]), "LUFS"),
            ("True Peak", show(meters[lim::M_TRUE_PEAK]), "dBTP"),
            ("Reduction", format!("{gr:.1}"), "dB"),
        ];
        let row_h = ((lr.height() - 26.0) / rows.len() as f32).min(34.0);
        for (k, (name, value, unit)) in rows.iter().enumerate() {
            let y = lr.top() + 6.0 + k as f32 * row_h;
            lp.text(pos2(lr.left() + 8.0, y), Align2::LEFT_TOP, *name, small(), LABEL);
            lp.text(pos2(lr.right() - 8.0, y + row_h - 6.0), Align2::RIGHT_BOTTOM, format!("{value} {unit}"), body(), if k == 2 { CURVE } else { TEXT });
        }
        let b = Rect::from_min_size(pos2(lr.left() + 8.0, lr.bottom() - 22.0), vec2(lr.width() - 16.0, 17.0));
        if button(ui, b, id.with("reset_lufs"), "Reset loudness", false, accent).clicked() {
            p[lim::RESET] += 1.0;
            changed = true;
        }
        ui.add_space(6.0);
        // The reset counter is not a knob.
        changed |= crate::ui::devices::param_grid_mod(ui, id, &lim::SPECS[..lim::RESET], &mut p[..lim::RESET], accent, h - 24.0, mv);
    });
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, RawInput};

    /// The EQ panel in a headless egui, fed input frame by frame, with its
    /// parameters written back only when it reports a change (as the device
    /// view does).
    struct Rig {
        ctx: egui::Context,
        status: EngineStatus,
        p: Vec<f32>,
        origin: Pos2,
    }

    impl Rig {
        fn new() -> Self {
            let mut r = Rig { ctx: egui::Context::default(), status: EngineStatus::new(), p: krabton_engine::params::defaults(&eq::SPECS), origin: Pos2::ZERO };
            r.idle(2);
            r
        }

        fn frame(&mut self, events: Vec<Event>) {
            let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 600.0))), events, ..Default::default() };
            let (p, status) = (&mut self.p, &self.status);
            let mut origin = self.origin;
            let _ = self.ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    origin = ui.cursor().min;
                    let mut v = p.clone();
                    if eq_panel(ui, Id::new(("device", 7u64)), 7, &mut v, ACCENT, 290.0, None, status) {
                        *p = v;
                    }
                });
            });
            self.origin = origin;
        }

        fn idle(&mut self, n: usize) {
            for _ in 0..n {
                self.frame(Vec::new());
            }
        }

        fn mapping(&self) -> Mapping {
            Mapping { rect: Rect::from_min_size(self.origin, vec2(620.0, 260.0)), range: 12.0 }
        }

        /// A point on the display: frequency and gain.
        fn at(&self, f: f64, db: f32) -> Pos2 {
            let m = self.mapping();
            pos2(m.x(f), m.y(db))
        }

        fn node(&self, b: usize) -> Option<Pos2> {
            node_pos(&self.mapping(), &self.p, b)
        }

        fn button(pos: Pos2, pressed: bool) -> Event {
            Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
        }

        fn double_click(&mut self, pos: Pos2) {
            self.frame(vec![Event::PointerMoved(pos)]);
            for _ in 0..2 {
                self.frame(vec![Self::button(pos, true)]);
                self.frame(vec![Self::button(pos, false)]);
            }
            self.idle(60);
        }

        fn click(&mut self, pos: Pos2) {
            self.frame(vec![Event::PointerMoved(pos)]);
            self.frame(vec![Self::button(pos, true)]);
            self.frame(vec![Self::button(pos, false)]);
            self.idle(60);
        }

        fn drag(&mut self, from: Pos2, to: Pos2) {
            self.frame(vec![Event::PointerMoved(from)]);
            self.frame(vec![Self::button(from, true)]);
            for k in 1..=10 {
                self.frame(vec![Event::PointerMoved(from + (to - from) * (k as f32 / 10.0))]);
            }
            self.frame(vec![Self::button(to, false)]);
            self.idle(60);
        }
    }

    /// A quick drag: two big moves, as a hand does.
    fn flick(r: &mut Rig, from: Pos2, to: Pos2) {
        r.frame(vec![Event::PointerMoved(from)]);
        r.frame(vec![Rig::button(from, true)]);
        r.frame(vec![Event::PointerMoved(from + (to - from) * 0.5)]);
        r.frame(vec![Event::PointerMoved(to)]);
        r.frame(vec![Rig::button(to, false)]);
        r.idle(60);
    }

    #[test]
    fn a_band_grabbed_off_centre_follows_a_quick_drag() {
        let mut r = Rig::new();
        r.double_click(r.at(300.0, 0.0));
        r.click(r.at(5000.0, 0.0));
        // Press 6 px beside the dot and move away fast.
        let start = r.node(0).unwrap() + vec2(6.0, 0.0);
        let to = r.at(2000.0, 6.0) + vec2(6.0, 0.0);
        flick(&mut r, start, to);
        assert!(close(r.node(0), r.at(2000.0, 6.0)), "band 1 followed: {:?} want {:?}", r.node(0), r.at(2000.0, 6.0));
    }

    fn close(a: Option<Pos2>, b: Pos2) -> bool {
        a.is_some_and(|a| a.distance(b) < 2.0)
    }

    #[test]
    fn every_band_can_be_dragged_again_later() {
        let mut r = Rig::new();
        r.double_click(r.at(100.0, 6.0));
        assert!(close(r.node(0), r.at(100.0, 6.0)), "band 1 made: {:?}", r.node(0));
        r.double_click(r.at(2000.0, -6.0));
        assert!(close(r.node(1), r.at(2000.0, -6.0)), "band 2 made: {:?}", r.node(1));
        r.drag(r.at(2000.0, -6.0), r.at(4000.0, -3.0));
        assert!(close(r.node(1), r.at(4000.0, -3.0)), "band 2 dragged: {:?}", r.node(1));
        // Something else: click the empty curve, then drag the first band.
        r.click(r.at(500.0, 0.0));
        r.drag(r.at(100.0, 6.0), r.at(60.0, 3.0));
        assert!(close(r.node(0), r.at(60.0, 3.0)), "band 1 dragged again: {:?}", r.node(0));
        r.drag(r.at(4000.0, -3.0), r.at(8000.0, 0.0));
        assert!(close(r.node(1), r.at(8000.0, 0.0)), "band 2 dragged again: {:?}", r.node(1));
    }
}
