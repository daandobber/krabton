//! Faces: a machine laid out as the machine it is, not as a grid of knobs.
//! Each face places its controls on a panel in units (a unit grows with the
//! height there is), in sections with their names, with the kind of control
//! each parameter has on the real thing: a big knob for what matters, lever
//! switches, rotary selectors with their positions printed around them, LED
//! buttons, drawbars, a display for the preset. Faces are drawn in the
//! machines' own look; the plain look keeps the grids.

pub mod analog;
pub mod bassline;
pub mod drums;
pub mod effects;
pub mod modules;
pub mod organ;
pub mod piano;
pub mod plaits;
pub mod sf2;
pub mod singer;
pub mod hyper;
pub mod coast;
pub mod street_organ;
pub mod synth;

use super::modulation::{knob_overlay, slider_overlay, ModView};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::params::{ParamKind, ParamSpec};

/// How a parameter is played.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ctl {
    Knob,
    /// A vertical slider.
    Fader,
    /// A lever with two to four positions (a toggle or a short choice).
    Switch,
    /// A rotary selector with its positions printed around it.
    Selector,
    /// A row (or column) of buttons with lights.
    Buttons,
    /// A small display: click for the list, arrows to step.
    #[allow(dead_code)]
    Lcd,
    /// An organ drawbar (pulled out downwards), its handle in this colour.
    Drawbar(Color32),
}

/// What sits on a face.
#[derive(Clone, Debug)]
pub enum What {
    Param { key: &'static str, ctl: Ctl, label: Option<&'static str> },
    /// A framed part of the panel, its name along the bottom.
    Section(&'static str),
    Text(&'static str),
    /// A screen, drawn by the machine.
    Screen(&'static str),
    /// The preset display.
    Preset,
    /// A socket (only to look at, as on the real panel).
    Jack(&'static str),
}

#[derive(Clone, Debug)]
pub struct Item {
    /// Where, in units.
    pub at: Rect,
    pub what: What,
}

/// A machine's panel: its size in units and what is on it.
#[derive(Clone, Debug, Default)]
pub struct Face {
    pub w: f32,
    pub h: f32,
    pub items: Vec<Item>,
    /// Parameters its screens play (not controls of their own).
    pub screened: Vec<&'static str>,
}

fn at(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(pos2(x, y), vec2(w, h))
}

impl Face {
    pub fn new(w: f32, h: f32) -> Self {
        Self { w, h, items: Vec::new(), screened: Vec::new() }
    }

    /// Parameters a screen of the face plays.
    pub fn screened(mut self, keys: &[&'static str]) -> Self {
        self.screened.extend_from_slice(keys);
        self
    }

    fn put(mut self, r: Rect, what: What) -> Self {
        self.items.push(Item { at: r, what });
        self
    }

    pub fn section(self, x: f32, y: f32, w: f32, h: f32, title: &'static str) -> Self {
        self.put(at(x, y, w, h), What::Section(title))
    }

    pub fn ctl(self, x: f32, y: f32, w: f32, h: f32, key: &'static str, ctl: Ctl) -> Self {
        self.put(at(x, y, w, h), What::Param { key, ctl, label: None })
    }

    /// A control with a label of its own (as the real panel prints it).
    pub fn named(self, x: f32, y: f32, w: f32, h: f32, key: &'static str, ctl: Ctl, label: &'static str) -> Self {
        self.put(at(x, y, w, h), What::Param { key, ctl, label: Some(label) })
    }

    pub fn knob(self, x: f32, y: f32, w: f32, h: f32, key: &'static str) -> Self {
        self.ctl(x, y, w, h, key, Ctl::Knob)
    }

    /// Controls side by side, each `w` wide.
    pub fn row(mut self, x: f32, y: f32, w: f32, h: f32, keys: &[&'static str], ctl: Ctl) -> Self {
        for (k, key) in keys.iter().enumerate() {
            self = self.ctl(x + k as f32 * w, y, w, h, key, ctl);
        }
        self
    }

    /// Controls one under the other, each `h` high.
    pub fn col(mut self, x: f32, y: f32, w: f32, h: f32, keys: &[&'static str], ctl: Ctl) -> Self {
        for (k, key) in keys.iter().enumerate() {
            self = self.ctl(x, y + k as f32 * h, w, h, key, ctl);
        }
        self
    }

    pub fn text(self, x: f32, y: f32, w: f32, h: f32, text: &'static str) -> Self {
        self.put(at(x, y, w, h), What::Text(text))
    }

    pub fn screen(self, x: f32, y: f32, w: f32, h: f32, name: &'static str) -> Self {
        self.put(at(x, y, w, h), What::Screen(name))
    }

    pub fn preset(self, x: f32, y: f32, w: f32, h: f32) -> Self {
        self.put(at(x, y, w, h), What::Preset)
    }

    pub fn jacks(mut self, x: f32, y: f32, w: f32, h: f32, labels: &[&'static str]) -> Self {
        for (k, l) in labels.iter().enumerate() {
            self = self.put(at(x + k as f32 * w, y, w, h), What::Jack(l));
        }
        self
    }

    /// The parameters it shows (their keys).
    #[cfg(test)]
    pub fn keys(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.items.iter().filter_map(|i| match i.what {
            What::Param { key, .. } => Some(key),
            _ => None,
        })
    }
}

/// A parameter by its key: its name, or "Group:Name" where names repeat.
pub fn find(specs: &[ParamSpec], key: &str) -> Option<usize> {
    match key.split_once(':') {
        Some((g, n)) => specs.iter().position(|s| s.group == g && s.name == n),
        None => specs.iter().position(|s| s.name == key),
    }
}

/// A machine's presets, shown in its display.
pub struct Presets<'a> {
    pub current: &'a mut String,
    pub names: &'static [&'static str],
    pub load: fn(&str) -> Vec<f32>,
}

/// What a face is drawn with.
pub struct Ctx<'a> {
    pub id: Id,
    /// To find the parameters (the machine's table).
    pub keys: &'static [ParamSpec],
    /// Their names and formats as shown (a copy may rename some).
    pub specs: &'a [ParamSpec],
    pub params: &'a mut [f32],
    pub mv: Option<&'a mut ModView>,
    pub presets: Option<Presets<'a>>,
}

fn label_font(u: f32) -> FontId {
    FontId::proportional((u * 0.5).clamp(9.0, 12.5))
}

fn value_font(u: f32) -> FontId {
    FontId::proportional((u * 0.46).clamp(8.5, 11.5))
}

/// Text that fits `w`: smaller (down to 7 points) if it has to.
pub(super) fn fit(p: &egui::Painter, text: &str, mut font: FontId, w: f32) -> FontId {
    let width = p.layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE).size().x;
    if width > w && width > 0.0 {
        font.size = (font.size * w / width).max(7.0);
    }
    font
}

/// Draws a face in `height` (its width follows). `screen` paints the
/// machine's screens (and may change values). Returns whether a value changed.
pub fn draw(ui: &mut Ui, face: &Face, height: f32, cx: &mut Ctx, screen: &mut dyn FnMut(&mut Ui, &str, Rect, &mut [f32]) -> bool) -> bool {
    let u = (height / face.h).clamp(10.0, 26.0);
    let (rect, _) = ui.allocate_exact_size(vec2(face.w * u, face.h * u), Sense::hover());
    let px = |r: Rect| Rect::from_min_size(rect.min + r.min.to_vec2() * u, r.size() * u);
    let p = ui.painter().clone();
    // Sections and print first, the controls on top.
    for item in &face.items {
        let r = px(item.at);
        match &item.what {
            What::Section(title) => section(&p, r, title, u),
            What::Text(t) => {
                let t = crate::skin::label_text(t);
                let font = fit(&p, &t, label_font(u), r.width());
                p.text(r.center(), Align2::CENTER_CENTER, t, font, crate::skin::ink_dim());
            }
            What::Jack(l) => jack(&p, r, l, u),
            _ => {}
        }
    }
    let mut changed = false;
    for item in &face.items {
        let r = px(item.at);
        match &item.what {
            What::Param { key, ctl, label } => {
                let Some(pi) = find(cx.keys, key) else { continue };
                let (Some(spec), Some(v)) = (cx.specs.get(pi), cx.params.get_mut(pi)) else { continue };
                let id = cx.id.with(pi);
                let label = label.unwrap_or(spec.name);
                let mut val = *v;
                let c = match ctl {
                    Ctl::Knob => knob(ui, r, id, spec, &mut val, label, u),
                    Ctl::Fader => {
                        let shown = ParamSpec { name: label, ..*spec };
                        let (c, track) = param_slider(ui, r, id, &shown, &mut val, accent_now());
                        if let Some(mv) = cx.mv.as_deref_mut() {
                            slider_overlay(ui, r, track, id.with("mod"), spec, val, pi, mv);
                        }
                        *v = val;
                        changed |= c;
                        continue;
                    }
                    Ctl::Switch => switch(ui, r, id, spec, &mut val, label, u),
                    Ctl::Selector => selector(ui, r, id, spec, &mut val, label, u),
                    Ctl::Buttons => buttons(ui, r, id, spec, &mut val, label, u),
                    Ctl::Lcd => lcd(ui, r, id, spec, &mut val, label, u),
                    Ctl::Drawbar(color) => drawbar(ui, r, id, spec, &mut val, label, u, *color),
                };
                if c {
                    *v = val;
                    changed = true;
                }
                if let Some(mv) = cx.mv.as_deref_mut() {
                    let k = if *ctl == Ctl::Knob { knob_rect(r, u) } else { r };
                    knob_overlay(ui, r, k, id.with("mod"), spec, *v, pi, mv);
                }
            }
            What::Screen(name) => changed |= screen(ui, name, r, cx.params),
            What::Preset => {
                if let Some(pr) = cx.presets.as_mut() {
                    if let Some(new) = preset(ui, r, cx.id.with("preset"), pr, u) {
                        if new.len() == cx.params.len() {
                            cx.params.copy_from_slice(&new);
                        }
                        changed = true;
                    }
                }
            }
            _ => {}
        }
    }
    changed
}

/// A section: a frame, its name centred along the bottom.
fn section(p: &egui::Painter, r: Rect, title: &str, u: f32) {
    let ink = crate::skin::ink();
    let line = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), 70));
    let th = (u * 0.85).max(12.0);
    let body = Rect::from_min_max(r.min, pos2(r.max.x, r.max.y - th));
    p.rect_stroke(body.shrink(1.0), 4.0, line, egui::StrokeKind::Inside);
    let title = title.to_uppercase();
    let font = fit(p, &title, FontId::proportional((u * 0.52).clamp(9.0, 13.0)), r.width() - 8.0);
    // Spaced out, as printed on panels.
    let spaced: String = title.chars().flat_map(|c| [c, '\u{2009}']).collect();
    let font = fit(p, &spaced, font, r.width() - 8.0);
    p.text(pos2(r.center().x, r.max.y - th * 0.5), Align2::CENTER_CENTER, spaced.trim_end(), font, ink);
}

/// A socket, with its name above.
fn jack(p: &egui::Painter, r: Rect, label: &str, u: f32) {
    let c = pos2(r.center().x, r.bottom() - r.width().min(r.height()) * 0.38);
    let rr = (r.width().min(r.height()) * 0.28).min(u * 0.6);
    p.circle_filled(c + vec2(0.0, 1.0), rr + 1.0, Color32::from_black_alpha(90));
    p.circle_filled(c, rr, Color32::from_rgb(0xb8, 0xbb, 0xbf));
    p.circle_filled(c, rr * 0.62, Color32::from_rgb(0x1a, 0x1a, 0x1c));
    let font = fit(p, label, FontId::proportional((u * 0.42).clamp(7.5, 10.0)), r.width());
    p.text(pos2(r.center().x, c.y - rr - 3.0), Align2::CENTER_BOTTOM, crate::skin::label_text(label), font, crate::skin::ink_dim());
}

/// The label over a control (fitted), and where the rest goes.
fn top_label(p: &egui::Painter, r: Rect, label: &str, u: f32) -> Rect {
    let label = crate::skin::label_text(label);
    let font = fit(p, &label, label_font(u), r.width());
    let h = font.size + 3.0;
    p.text(pos2(r.center().x, r.top() + h * 0.5), Align2::CENTER_CENTER, label, font, crate::skin::ink());
    Rect::from_min_max(pos2(r.left(), r.top() + h), r.max)
}

fn value_text(p: &egui::Painter, r: Rect, text: &str, u: f32) -> Rect {
    let font = fit(p, text, value_font(u), r.width());
    let h = font.size + 3.0;
    tabular_text(p, pos2(r.center().x, r.bottom() - h * 0.5), Align2::CENTER_CENTER, text, font, crate::skin::ink_dim());
    Rect::from_min_max(r.min, pos2(r.right(), r.bottom() - h))
}

/// Where the knob sits in a knob's place, and whether its value is printed
/// under it (where there is no room, the knob keeps it for its tooltip).
fn knob_layout(r: Rect, u: f32) -> (Rect, bool) {
    let lh = label_font(u).size + 3.0;
    let mut vh = value_font(u).size + 3.0;
    let mut side = (r.height() - lh - vh).min(r.width() - 2.0);
    let with_value = side >= u * 1.1;
    if !with_value {
        vh = 0.0;
        side = (r.height() - lh).min(r.width() - 2.0);
    }
    let side = side.max(8.0);
    (Rect::from_center_size(pos2(r.center().x, r.top() + lh + (r.height() - lh - vh) * 0.5), vec2(side, side)), with_value)
}

fn knob_rect(r: Rect, u: f32) -> Rect {
    knob_layout(r, u).0
}

pub(crate) fn knob(ui: &mut Ui, r: Rect, id: Id, spec: &ParamSpec, v: &mut f32, label: &str, u: f32) -> bool {
    let p = ui.painter().clone();
    top_label(&p, r, label, u);
    let (k, with_value) = knob_layout(r, u);
    if with_value {
        value_text(&p, r, &spec.format(*v), u);
    }
    let bipolar = spec.min < 0.0 && spec.max > 0.0;
    let steps = if spec.is_discrete() { (spec.max - spec.min).round().max(1.0) } else { 0.0 };
    let speed = if steps > 0.0 { (167.0 / (steps * 24.0)).max(1.0) } else { 1.0 } * (36.0 / k.width()).clamp(0.5, 1.0);
    let (resp, new) = knob_with_speed(ui, k, id, spec.to_norm(*v), spec.to_norm(spec.default), bipolar, accent_now(), speed);
    resp.on_hover_text(format!("{}: {}\nDrag to change, Shift for fine, double-click to reset", spec.name, spec.format(*v)));
    match new.map(|n| spec.from_norm(n)) {
        Some(nv) if nv != *v => {
            *v = nv;
            true
        }
        _ => false,
    }
}

/// The options of a choice or toggle (a toggle's on comes first).
fn options(spec: &ParamSpec) -> Vec<String> {
    match spec.kind {
        ParamKind::Choice(o) => o.iter().map(|s| s.to_string()).collect(),
        ParamKind::Toggle => vec!["On".into(), "Off".into()],
        _ => {
            let n = (spec.max - spec.min).round().max(0.0) as usize + 1;
            (0..n).map(|k| spec.format(spec.min + k as f32)).collect()
        }
    }
}

/// The option shown first and the value of option `k`.
fn option_value(spec: &ParamSpec, k: usize) -> f32 {
    match spec.kind {
        ParamKind::Toggle => {
            if k == 0 {
                1.0
            } else {
                0.0
            }
        }
        ParamKind::Choice(_) => k as f32,
        _ => spec.min + k as f32,
    }
}

fn option_of(spec: &ParamSpec, v: f32, n: usize) -> usize {
    match spec.kind {
        ParamKind::Toggle => {
            if v >= 0.5 {
                0
            } else {
                1
            }
        }
        ParamKind::Choice(_) => (v.round().max(0.0) as usize).min(n.saturating_sub(1)),
        _ => ((v - spec.min).round().max(0.0) as usize).min(n.saturating_sub(1)),
    }
}

/// Click, drag and double-click over options: the option now (or None).
fn pick(ui: &Ui, resp: &egui::Response, id: Id, current: usize, n: usize, step_px: f32, vertical: bool, at: impl Fn(egui::Pos2) -> Option<usize>, default: usize) -> Option<usize> {
    let acc_id = id.with("acc");
    if resp.drag_started() {
        ui.ctx().memory_mut(|m| m.data.insert_temp(acc_id, 0.0f32));
    }
    if resp.dragged() {
        let d = if vertical { resp.drag_delta().y } else { -resp.drag_delta().x };
        let acc = ui.ctx().memory(|m| m.data.get_temp::<f32>(acc_id).unwrap_or(0.0)) + d;
        let k = (acc / step_px).trunc();
        ui.ctx().memory_mut(|m| m.data.insert_temp(acc_id, acc - k * step_px));
        if k != 0.0 {
            let next = (current as f32 + k).clamp(0.0, (n - 1) as f32) as usize;
            if next != current {
                return Some(next);
            }
        }
    }
    if resp.double_clicked() {
        return Some(default);
    }
    if resp.clicked() {
        let chosen = resp.interact_pointer_pos().and_then(&at).unwrap_or((current + 1) % n);
        if chosen != current {
            return Some(chosen);
        }
    }
    None
}

fn set_option(spec: &ParamSpec, v: &mut f32, k: Option<usize>) -> bool {
    match k.map(|k| option_value(spec, k)) {
        Some(nv) if nv != *v => {
            *v = nv;
            true
        }
        _ => false,
    }
}

/// A lever switch: the positions down a slot, their names beside it.
fn switch(ui: &mut Ui, r: Rect, id: Id, spec: &ParamSpec, v: &mut f32, label: &str, u: f32) -> bool {
    let p = ui.painter().clone();
    let area = top_label(&p, r, label, u);
    let opts = options(spec);
    let n = opts.len().max(2);
    let cur = option_of(spec, *v, n);
    let slot_w = (u * 0.42).clamp(5.0, 11.0);
    let names_w = opts.iter().map(|o| p.layout_no_wrap(crate::skin::label_text(o), value_font(u), Color32::WHITE).size().x).fold(0.0f32, f32::max);
    let total = slot_w + 5.0 + names_w.min(area.width() - slot_w - 5.0);
    let x0 = area.center().x - total * 0.5;
    let slot = Rect::from_min_max(pos2(x0, area.top() + 3.0), pos2(x0 + slot_w, area.bottom() - 3.0));
    let step = slot.height() / n as f32;
    let y_of = |k: usize| slot.top() + step * (k as f32 + 0.5);
    let resp = ui.interact(area, id, Sense::click_and_drag());
    let default = option_of(spec, spec.default, n);
    let k = pick(ui, &resp, id, cur, n, (step * 0.8).max(10.0), true, |pos| Some((((pos.y - slot.top()) / step).floor().max(0.0) as usize).min(n - 1)), default);
    let changed = set_option(spec, v, k);
    let cur = option_of(spec, *v, n);
    let (well, _) = crate::skin::well();
    p.rect_filled(slot, slot_w * 0.5, well);
    p.rect_stroke(slot, slot_w * 0.5, Stroke::new(1.0_f32, Color32::from_black_alpha(120)), egui::StrokeKind::Inside);
    for (k, o) in opts.iter().enumerate() {
        let on = k == cur;
        let font = fit(&p, &crate::skin::label_text(o), value_font(u), (area.right() - slot.right() - 5.0).max(10.0));
        p.text(pos2(slot.right() + 5.0, y_of(k)), Align2::LEFT_CENTER, crate::skin::label_text(o), font, if on { crate::skin::ink() } else { crate::skin::ink_dim() });
    }
    let hot = resp.hovered() || resp.dragged();
    let lever = Rect::from_center_size(pos2(slot.center().x, y_of(cur)), vec2(slot_w * 1.9, (step * 0.62).clamp(6.0, u * 0.9)));
    p.rect_filled(lever.translate(vec2(0.0, 1.5)), 2.5, Color32::from_black_alpha(110));
    p.add(Shape::rect_filled(lever, 2.5, if hot { Color32::WHITE } else { Color32::from_rgb(0xd8, 0xdb, 0xde) }));
    p.line_segment([pos2(lever.left() + 2.0, lever.center().y), pos2(lever.right() - 2.0, lever.center().y)], Stroke::new(1.5_f32, accent_now()));
    resp.on_hover_text(format!("{}: {}\nClick a position, or drag", spec.name, opts.get(cur).cloned().unwrap_or_default()));
    changed
}

/// A rotary selector: a knob pointing at one of the positions printed round it.
fn selector(ui: &mut Ui, r: Rect, id: Id, spec: &ParamSpec, v: &mut f32, label: &str, u: f32) -> bool {
    let p = ui.painter().clone();
    let area = top_label(&p, r, label, u);
    let opts = options(spec);
    // The positions of a toggle go off, on (left to right).
    let n = opts.len().max(2);
    let cur = option_of(spec, *v, n);
    let font = value_font(u);
    let side = area.width().min(area.height());
    let kr = side * 0.27;
    let c = area.center();
    let ring = kr + side * 0.08;
    let ang = |k: usize| crate::widgets::KNOB_START + crate::widgets::KNOB_RANGE * k as f32 / (n - 1) as f32;
    let resp = ui.interact(area, id, Sense::click_and_drag());
    let default = option_of(spec, spec.default, n);
    let k = pick(
        ui,
        &resp,
        id,
        cur,
        n,
        16.0,
        true,
        |pos| {
            let d = pos - c;
            if d.length() < kr * 0.8 {
                return None;
            }
            (0..n).min_by(|&a, &b| {
                let da = (pos - (c + vec2(ang(a).cos(), ang(a).sin()) * ring)).length();
                let db = (pos - (c + vec2(ang(b).cos(), ang(b).sin()) * ring)).length();
                da.total_cmp(&db)
            })
        },
        default,
    );
    let changed = set_option(spec, v, k);
    let cur = option_of(spec, *v, n);
    for k in 0..n {
        let d = vec2(ang(k).cos(), ang(k).sin());
        let on = k == cur;
        let col = if on { crate::skin::ink() } else { crate::skin::ink_dim() };
        p.line_segment([c + d * (kr + 1.0), c + d * (kr + side * 0.05)], Stroke::new(1.0_f32, if on { accent_now() } else { crate::skin::ink_dim() }));
        // Waves as their shapes, as panels print them; the rest as words that keep inside.
        if let Some(shape) = wave_icon(&opts[k]) {
            let w = (u * 0.95).min(side * 0.3);
            let pos = c + d * (ring + w * 0.55);
            let pts: Vec<egui::Pos2> = shape.iter().map(|(x, y)| pos + vec2((x - 0.5) * w, -y * w * 0.32)).collect();
            p.add(Shape::line(pts, Stroke::new(if on { 1.8_f32 } else { 1.2 }, col)));
            continue;
        }
        let name = crate::skin::label_text(&opts[k]);
        let pos = c + d * (ring + font.size * 0.55);
        let (align, room) = if d.x < -0.3 {
            (Align2::RIGHT_CENTER, pos.x - r.left())
        } else if d.x > 0.3 {
            (Align2::LEFT_CENTER, r.right() - pos.x)
        } else {
            (Align2::CENTER_CENTER, r.width())
        };
        let f = fit(&p, &name, font.clone(), room.max(8.0));
        p.text(pos, align, name, f, col);
    }
    let knob = Rect::from_center_size(c, vec2(kr * 2.0, kr * 2.0));
    crate::widgets::ring_face(&p, knob, cur as f32 / (n - 1) as f32, false, accent_now(), resp.hovered() || resp.dragged());
    resp.on_hover_text(format!("{}: {}\nClick a position, or drag", spec.name, opts.get(cur).cloned().unwrap_or_default()));
    changed
}

/// The shape of a wave by its name (x 0..1, y -1..1), if it is one.
fn wave_icon(name: &str) -> Option<Vec<(f32, f32)>> {
    let n = name.to_lowercase();
    let pts = match n.as_str() {
        "saw" | "ramp" | "saw down" | "sawtooth" => vec![(0.0, -1.0), (0.0, 1.0), (0.5, -1.0), (0.5, 1.0), (1.0, -1.0)],
        "saw up" | "ramp up" => vec![(0.0, -1.0), (0.5, 1.0), (0.5, -1.0), (1.0, 1.0), (1.0, -1.0)],
        "pulse" | "square" | "sqr" => vec![(0.0, -1.0), (0.0, 1.0), (0.5, 1.0), (0.5, -1.0), (1.0, -1.0), (1.0, 1.0)],
        "triangle" | "tri" => vec![(0.0, 0.0), (0.25, 1.0), (0.75, -1.0), (1.0, 0.0)],
        "sine" | "sin" => (0..=16).map(|k| (k as f32 / 16.0, (k as f32 / 16.0 * std::f32::consts::TAU).sin())).collect(),
        "s&h" | "random" | "sample & hold" => vec![(0.0, 0.3), (0.2, 0.3), (0.2, -0.8), (0.4, -0.8), (0.4, 1.0), (0.6, 1.0), (0.6, -0.3), (0.8, -0.3), (0.8, 0.6), (1.0, 0.6)],
        "noise" => (0..=10).map(|k| (k as f32 / 10.0, if k % 2 == 0 { 0.8 } else { -0.7 } * (1.0 - (k % 3) as f32 * 0.3))).collect(),
        _ => return None,
    };
    Some(pts)
}

/// Buttons with lights, one per option, in a row (or a column if taller).
fn buttons(ui: &mut Ui, r: Rect, id: Id, spec: &ParamSpec, v: &mut f32, label: &str, u: f32) -> bool {
    let p = ui.painter().clone();
    let area = top_label(&p, r, label, u);
    let opts = options(spec);
    let n = opts.len().max(1);
    let cur = option_of(spec, *v, n);
    let column = area.height() > area.width();
    let cell = |k: usize| {
        if column {
            let h = area.height() / n as f32;
            Rect::from_min_size(pos2(area.left(), area.top() + k as f32 * h), vec2(area.width(), h)).shrink(1.5)
        } else {
            let w = area.width() / n as f32;
            Rect::from_min_size(pos2(area.left() + k as f32 * w, area.top()), vec2(w, area.height())).shrink(1.5)
        }
    };
    let mut changed = false;
    let (well, well_ink) = crate::skin::well();
    for (k, o) in opts.iter().enumerate() {
        let b = cell(k);
        let resp = ui.interact(b, id.with(k), Sense::click());
        if resp.clicked() {
            changed |= set_option(spec, v, Some(k));
        }
        if resp.double_clicked() {
            changed |= set_option(spec, v, Some(option_of(spec, spec.default, n)));
        }
        let on = option_of(spec, *v, n) == k;
        p.rect_filled(b.translate(vec2(0.0, 1.0)), 3.0, Color32::from_black_alpha(90));
        p.rect_filled(b, 3.0, if resp.hovered() { mix(well, Color32::WHITE, 0.12) } else { well });
        let led = pos2(b.left() + (u * 0.3).max(5.0), b.center().y);
        p.circle_filled(led, (u * 0.14).clamp(2.0, 3.5), if on { accent_now() } else { Color32::from_black_alpha(140) });
        if on {
            p.circle_filled(led, (u * 0.26).clamp(3.5, 6.0), Color32::from_rgba_unmultiplied(accent_now().r(), accent_now().g(), accent_now().b(), 50));
        }
        let text = crate::skin::label_text(o);
        let tr = Rect::from_min_max(pos2(led.x + 5.0, b.top()), b.max);
        let f = fit(&p, &text, value_font(u), tr.width() - 2.0);
        p.text(tr.left_center(), Align2::LEFT_CENTER, text, f, if on { well_ink } else { mix(well_ink, well, 0.35) });
        resp.on_hover_text(format!("{}: {o}", spec.name));
    }
    let _ = cur;
    changed
}

/// A display: the value in its lettering, arrows to step, click for the list.
fn lcd(ui: &mut Ui, r: Rect, id: Id, spec: &ParamSpec, v: &mut f32, label: &str, u: f32) -> bool {
    let p = ui.painter().clone();
    let area = top_label(&p, r, label, u);
    let opts = options(spec);
    let n = opts.len().max(1);
    let cur = option_of(spec, *v, n);
    let d = Rect::from_center_size(area.center(), vec2(area.width(), area.height().min(u * 1.6)));
    let mut changed = false;
    let arrow_w = d.height().min(u * 1.1);
    let left = Rect::from_min_size(d.min, vec2(arrow_w, d.height()));
    let right = Rect::from_min_size(pos2(d.right() - arrow_w, d.top()), vec2(arrow_w, d.height()));
    let mid = Rect::from_min_max(pos2(left.right(), d.top()), pos2(right.left(), d.bottom()));
    for (k, (b, dir)) in [(left, -1i32), (right, 1)].into_iter().enumerate() {
        let resp = ui.interact(b, id.with(("step", k)), Sense::click());
        if resp.clicked() {
            let next = (cur as i32 + dir).rem_euclid(n as i32) as usize;
            changed |= set_option(spec, v, Some(next));
        }
        let col = if resp.hovered() { accent_now() } else { crate::skin::ink_dim() };
        let c = b.center();
        let s = b.height() * 0.18;
        let pts = if dir < 0 { vec![c + vec2(s, -s * 1.4), c + vec2(-s, 0.0), c + vec2(s, s * 1.4)] } else { vec![c + vec2(-s, -s * 1.4), c + vec2(s, 0.0), c + vec2(-s, s * 1.4)] };
        p.add(Shape::convex_polygon(pts, col, Stroke::NONE));
    }
    let resp = ui.interact(mid, id, Sense::click());
    screen_box(&p, mid, u);
    let cur = option_of(spec, *v, n);
    let text = opts.get(cur).cloned().unwrap_or_default();
    let f = fit(&p, &text, crate::skin::Font::Lcd.id((mid.height() * 0.62).max(10.0)), mid.width() - 6.0);
    p.text(mid.center(), Align2::CENTER_CENTER, &text, f, accent_now());
    egui::Popup::menu(&resp).show(|ui| {
        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
            for (k, o) in opts.iter().enumerate() {
                if ui.selectable_label(k == cur, o).clicked() {
                    changed |= set_option(spec, v, Some(k));
                    ui.close();
                }
            }
        });
    });
    changed
}

/// A dark glass, as displays sit in panels.
pub(super) fn screen_box(p: &egui::Painter, r: Rect, u: f32) {
    let _ = u;
    p.rect_filled(r.expand(1.5), 3.0, Color32::from_black_alpha(140));
    p.rect_filled(r, 2.0, super::inst_screen::SCREEN_BG);
    p.line_segment([pos2(r.left() + 2.0, r.top() + 1.0), pos2(r.right() - 2.0, r.top() + 1.0)], Stroke::new(1.0_f32, Color32::from_white_alpha(14)));
}

/// An organ drawbar: pulled out downwards, its shaft marked 1 to 8.
#[allow(clippy::too_many_arguments)]
fn drawbar(ui: &mut Ui, r: Rect, id: Id, spec: &ParamSpec, v: &mut f32, label: &str, u: f32, color: Color32) -> bool {
    let p = ui.painter().clone();
    let area = top_label(&p, r, label, u);
    let max = spec.max.max(1.0);
    let w = (area.width() * 0.62).min(u * 1.5);
    let slot_top = area.top() + 2.0;
    let handle_h = (u * 1.2).max(12.0);
    let travel = area.height() - handle_h - 6.0;
    let resp = ui.interact(area, id, Sense::click_and_drag());
    let mut changed = false;
    let fine = ui.input(|i| i.modifiers.shift);
    if let Some(n) = drag_position(ui, &resp, id, *v / max, resp.drag_delta().y / travel.max(1.0) * if fine { 0.3 } else { 1.0 }) {
        let nv = (n * max).round().clamp(0.0, max);
        if nv != *v {
            *v = nv;
            changed = true;
        }
    }
    if resp.double_clicked() {
        *v = spec.default;
        changed = true;
    }
    let pulled = (*v / max).clamp(0.0, 1.0) * travel;
    let cx = area.center().x;
    // The slot in the panel, then the shaft out of it with its marks.
    p.rect_filled(Rect::from_min_size(pos2(cx - w * 0.5 - 2.0, slot_top - 2.0), vec2(w + 4.0, 5.0)), 2.0, Color32::from_black_alpha(200));
    if pulled > 0.5 {
        let shaft = Rect::from_min_size(pos2(cx - w * 0.32, slot_top), vec2(w * 0.64, pulled));
        p.rect_filled(shaft, 1.0, Color32::from_rgb(0xe9, 0xe4, 0xd6));
        let step = travel / max;
        for k in 1..=(*v as i32) {
            let y = slot_top + k as f32 * step;
            if y > slot_top + pulled + 0.5 {
                break;
            }
            p.text(pos2(cx, y - step * 0.5), Align2::CENTER_CENTER, k.to_string(), FontId::proportional((step * 0.7).clamp(6.0, 11.0)), Color32::from_rgb(0x30, 0x2c, 0x26));
        }
    }
    let handle = Rect::from_min_size(pos2(cx - w * 0.5, slot_top + pulled), vec2(w, handle_h));
    p.rect_filled(handle.translate(vec2(0.0, 2.0)), 3.0, Color32::from_black_alpha(120));
    p.rect_filled(handle, 3.0, if resp.hovered() || resp.dragged() { mix(color, Color32::WHITE, 0.2) } else { color });
    p.rect_filled(Rect::from_min_size(handle.min + vec2(2.0, 2.0), vec2(w - 4.0, handle_h * 0.3)), 2.0, Color32::from_white_alpha(40));
    let ink = text_on(color);
    p.text(handle.center() + vec2(0.0, handle_h * 0.08), Align2::CENTER_CENTER, format!("{}", *v as i32), FontId::proportional((handle_h * 0.55).clamp(8.0, 13.0)), ink);
    resp.on_hover_text(format!("{}: {}\nDrag down to pull it out, double-click to reset", spec.name, *v as i32));
    changed
}

/// The preset display: its name, arrows to step, click for the list.
/// Returns the values of a preset picked.
fn preset(ui: &mut Ui, r: Rect, id: Id, pr: &mut Presets, u: f32) -> Option<Vec<f32>> {
    let i = names_display(ui, r, id, pr.names, pr.current, u)?;
    *pr.current = pr.names[i].to_string();
    Some((pr.load)(pr.names[i]))
}

/// A display of one of `names` (the one called `current`), with arrows to
/// step and the list on a click. Returns the one picked.
pub(crate) fn names_display(ui: &mut Ui, r: Rect, id: Id, names: &[&str], current: &str, u: f32) -> Option<usize> {
    let p = ui.painter().clone();
    let n = names.len();
    if n == 0 {
        return None;
    }
    let at = names.iter().position(|x| *x == current);
    let mut picked: Option<usize> = None;
    let arrow_w = r.height().min(u * 1.2);
    let left = Rect::from_min_size(r.min, vec2(arrow_w, r.height()));
    let right = Rect::from_min_size(pos2(r.right() - arrow_w, r.top()), vec2(arrow_w, r.height()));
    let mid = Rect::from_min_max(pos2(left.right() + 2.0, r.top()), pos2(right.left() - 2.0, r.bottom()));
    for (k, (b, dir)) in [(left, -1i32), (right, 1)].into_iter().enumerate() {
        let resp = ui.interact(b, id.with(("step", k)), Sense::click()).on_hover_text(if dir < 0 { "Previous" } else { "Next" });
        if resp.clicked() {
            picked = Some(match at {
                Some(i) => (i as i32 + dir).rem_euclid(n as i32) as usize,
                None if dir > 0 => 0,
                None => n - 1,
            });
        }
        let (well, well_ink) = crate::skin::well();
        p.rect_filled(b.shrink(1.0), 3.0, if resp.hovered() { mix(well, Color32::WHITE, 0.15) } else { well });
        let c = b.center();
        let s = b.height().min(b.width()) * 0.16;
        let pts = if dir < 0 { vec![c + vec2(s, -s * 1.4), c + vec2(-s, 0.0), c + vec2(s, s * 1.4)] } else { vec![c + vec2(-s, -s * 1.4), c + vec2(s, 0.0), c + vec2(-s, s * 1.4)] };
        p.add(Shape::convex_polygon(pts, well_ink, Stroke::NONE));
    }
    let resp = ui.interact(mid, id, Sense::click()).on_hover_text("Click for the list");
    screen_box(&p, mid, u);
    let name = if current.is_empty() { "PRESETS".to_string() } else { current.to_uppercase() };
    let f = fit(&p, &name, crate::skin::Font::Lcd.id((mid.height() * 0.6).max(10.0)), mid.width() - 8.0);
    p.text(mid.center(), Align2::CENTER_CENTER, name, f, accent_now());
    egui::Popup::menu(&resp).show(|ui| {
        egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
            for (i, name) in names.iter().enumerate() {
                if ui.selectable_label(at == Some(i), *name).clicked() {
                    picked = Some(i);
                    ui.close();
                }
            }
        });
    });
    picked
}

#[cfg(test)]
pub(crate) fn check(face: &Face, specs: &[ParamSpec]) {
    // Every key finds its parameter (only one), and every shown parameter is on the face.
    for key in face.keys() {
        assert!(find(specs, key).is_some(), "{key} is not a parameter");
        if !key.contains(':') {
            assert_eq!(specs.iter().filter(|s| s.name == key).count(), 1, "{key} is not one parameter: say its group");
        }
    }
    for (i, s) in specs.iter().enumerate().filter(|(_, s)| s.shown()) {
        assert!(face.keys().chain(face.screened.iter().copied()).any(|k| find(specs, k) == Some(i)), "{}:{} is not on the face", s.group, s.name);
    }
    // Nothing sticks out of the panel.
    for item in &face.items {
        assert!(item.at.min.x >= 0.0 && item.at.min.y >= 0.0 && item.at.max.x <= face.w + 1e-3 && item.at.max.y <= face.h + 1e-3, "{:?} sticks out", item.what);
    }
}
