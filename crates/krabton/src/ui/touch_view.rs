//! The Touch view: the selected track played with touch plates, made to play
//! with (see `sequencers::touch`). The plates run along the bottom, in the
//! song's key: higher up a plate presses harder, dragging across them slides
//! from one to the next, and the letter keys press them too. Above them the
//! pulser and chance, the five steps, and the patch bay: cables from what
//! Touch makes (pressure, pulse, steps, random, drift, where the hand is) to
//! six of the instrument's knobs, each cable with its amount.
//!
//! It works on the track's Touch device (put first in its MIDI chain when the
//! view first shows the track) and on Touch modulators routed to the
//! instrument, so what is played and patched here is saved, automatable and
//! seen in the Mod tab as well.

use crate::app::{App, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::epaint::CubicBezierShape;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Id, Mesh, Pos2, Rect, Sense, Stroke, Ui};
use krabton_engine::engine::Command;
use krabton_engine::midi_fx::MidiFxKind;
use krabton_engine::modulation::{instrument_specs, ModKind, ModOwner, ModTarget, MAX_MODS};
use krabton_engine::params::{ParamKind, ParamSpec};
use krabton_engine::project::TrackKind;
use krabton_engine::sequencers::touch::{self, N_PATCH, N_STEPS, OUTPUTS};

/// The colours of the six outputs, and of their cables.
pub(crate) const OUT_COLORS: [Color32; 6] = [
    Color32::from_rgb(0xff, 0x9a, 0x3c),
    Color32::from_rgb(0xff, 0x5a, 0x7a),
    Color32::from_rgb(0xf5, 0xd0, 0x4a),
    Color32::from_rgb(0x58, 0xd0, 0x8a),
    Color32::from_rgb(0x4f, 0xc8, 0xf0),
    Color32::from_rgb(0xb0, 0x8c, 0xff),
];

/// What the outputs are, for their tooltips.
const OUT_HELP: [&str; 6] = [
    "How hard the plates are pressed (higher up a plate: harder)",
    "The pulse: an envelope on every pulse (or every touch), as long as Decay",
    "The steps' values (the lower row of the steps)",
    "A random value, new on every pulse (or touch)",
    "A random value drifting slowly from one to the next",
    "Where along the plates the hand is, low to high",
];

/// What the view keeps between frames.
#[derive(Default)]
pub struct TouchViewState {
    /// The plates held (by the pointer or fingers): their notes and the track.
    held: Vec<(u8, u64)>,
    /// Fingers on the plates (touch screens): id, where, how hard if the screen says.
    fingers: Vec<(u64, Pos2, Option<f32>)>,
    /// How hard and where, as last sent to the track.
    sent: Option<(u64, f32, f32)>,
    /// A cable being drawn, from an output (true, which) or a patch point (false, which).
    drawing: Option<(bool, usize)>,
    /// The pulses counted last, and when the count changed (the pulse light).
    pulses: u32,
    flash: f64,
}

/// A track's Touch modulator for one output.
fn touch_mod(app: &App, track: u64, out: usize) -> Option<u64> {
    app.project.modulators.iter().find(|m| m.kind == ModKind::Touch && m.track == Some(track) && m.params.first().map(|v| v.round() as usize) == Some(out)).map(|m| m.id)
}

fn continuous(s: &ParamSpec) -> bool {
    matches!(s.kind, ParamKind::Linear | ParamKind::Log | ParamKind::Int) && s.group != krabton_engine::params::SCREEN_ONLY && s.max > s.min
}

/// The six knobs of an instrument worth patching first: the filter or
/// timbre, the shape or resonance, the decay, a drive or modulation, a rate,
/// and something of the space; then whatever is left.
pub(crate) fn touch_picks(specs: &[ParamSpec]) -> Vec<usize> {
    const WANT: [&[&str]; 6] = [
        &["cutoff", "filter", "bright", "timbre", "tone", "colo"],
        &["reso", "harmonic", "morph", "shape", "wave", "fold"],
        &["decay", "release", "damp", "length"],
        &["drive", "fm", "mod", "detune", "spread", "noise"],
        &["vibrato", "lfo", "rate", "speed"],
        &["attack", "pan", "space", "reverb", "delay", "chorus"],
    ];
    let mut picks: Vec<usize> = Vec::new();
    for words in WANT {
        if let Some(i) = specs.iter().enumerate().position(|(i, s)| continuous(s) && !picks.contains(&i) && words.iter().any(|w| s.name.to_lowercase().contains(w))) {
            picks.push(i);
        }
    }
    for (i, s) in specs.iter().enumerate() {
        if picks.len() >= N_PATCH {
            break;
        }
        if continuous(s) && !picks.contains(&i) && !s.name.to_lowercase().contains("volume") {
            picks.push(i);
        }
    }
    picks.truncate(N_PATCH);
    picks
}

/// A vertical gradient, top to bottom.
fn gradient(p: &egui::Painter, r: Rect, top: Color32, bottom: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(r.left_top(), top);
    m.colored_vertex(r.right_top(), top);
    m.colored_vertex(r.right_bottom(), bottom);
    m.colored_vertex(r.left_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(m));
}

fn section(p: &egui::Painter, r: Rect, title: &str) -> Rect {
    p.rect_filled(r, 6.0, PANEL);
    p.rect_stroke(r, 6.0, Stroke::new(1.0_f32, LINE_SOFT), egui::StrokeKind::Inside);
    p.text(r.left_top() + vec2(10.0, 8.0), Align2::LEFT_TOP, title, FontId::proportional(11.0), TEXT_DIM);
    Rect::from_min_max(r.min + vec2(10.0, 26.0), r.max - vec2(10.0, 8.0))
}

/// A knob for one of Touch's parameters, its name above and value below.
fn knob_param(ui: &mut Ui, r: Rect, id: Id, spec: &ParamSpec, value: &mut f32, color: Color32) -> bool {
    let p = ui.painter().clone();
    p.text(pos2(r.center().x, r.top()), Align2::CENTER_TOP, spec.name, FontId::proportional(10.5), TEXT_DIM);
    let side = (r.height() - 30.0).min(r.width()).max(20.0);
    let k = Rect::from_center_size(pos2(r.center().x, r.top() + 14.0 + side * 0.5), vec2(side, side));
    let (resp, new) = knob(ui, k, id, spec.to_norm(*value), spec.to_norm(spec.default), false, color);
    let mut changed = false;
    if let Some(n) = new {
        *value = spec.from_norm(n);
        changed = true;
    }
    resp.on_hover_text(format!("{}: {}", spec.name, spec.format(*value)));
    p.text(pos2(r.center().x, k.bottom() + 2.0), Align2::CENTER_TOP, spec.format(*value), FontId::proportional(10.0), TEXT);
    changed
}

/// A lit button that switches a parameter on and off.
fn toggle(ui: &mut Ui, r: Rect, id: Id, label: &str, on: bool, lamp: Option<f32>, tip: &str) -> bool {
    let resp = ui.interact(r, id, Sense::click()).on_hover_text(tip);
    let p = ui.painter();
    let fill = if on { mix(PANEL3, accent_now(), 0.35) } else { PANEL3 };
    p.rect_filled(r, 5.0, if resp.hovered() { mix(fill, TEXT, 0.08) } else { fill });
    p.rect_stroke(r, 5.0, Stroke::new(1.0_f32, if on { accent_now() } else { LINE }), egui::StrokeKind::Inside);
    let mut tx = r.center().x;
    if let Some(l) = lamp {
        let c = pos2(r.left() + 12.0, r.center().y);
        p.circle_filled(c, 4.5, mix(shade(REC, 0.35), REC, l.clamp(0.0, 1.0)));
        tx += 6.0;
    }
    p.text(pos2(tx, r.center().y), Align2::CENTER_CENTER, label, FontId::proportional(12.0), if on { TEXT } else { TEXT_DIM });
    resp.clicked()
}

/// A choice shown as its value; a click opens the list.
fn choice(ui: &mut Ui, r: Rect, id: Id, spec: &ParamSpec, value: &mut f32) -> bool {
    let ParamKind::Choice(items) = spec.kind else { return false };
    let resp = ui.interact(r, id, Sense::click()).on_hover_text(spec.name);
    let p = ui.painter();
    p.rect_filled(r, 4.0, if resp.hovered() { PANEL3 } else { SLOT });
    let i = (value.round().max(0.0) as usize).min(items.len() - 1);
    p.text(pos2(r.left() + 8.0, r.center().y), Align2::LEFT_CENTER, format!("{}  {}", spec.name, items[i]), FontId::proportional(11.5), TEXT);
    down_arrow(&p, pos2(r.right() - 11.0, r.center().y));
    let mut changed = false;
    egui::Popup::menu(&resp).show(|ui| {
        for (k, it) in items.iter().enumerate() {
            if ui.selectable_label(k == i, *it).clicked() {
                *value = k as f32;
                changed = true;
                ui.close();
            }
        }
    });
    changed
}

/// The little arrow of a list that opens.
fn down_arrow(p: &egui::Painter, c: Pos2) {
    p.add(egui::Shape::convex_polygon(vec![c + vec2(-4.0, -2.0), c + vec2(4.0, -2.0), c + vec2(0.0, 3.0)], TEXT_DIM, Stroke::NONE));
}

fn bezier(a: Pos2, b: Pos2) -> [Pos2; 4] {
    let dx = (b.x - a.x).abs();
    let sag = 18.0 + dx * 0.12 + (b.y - a.y).abs() * 0.05;
    [a, pos2(a.x + dx * 0.4, a.y + sag), pos2(b.x - dx * 0.4, b.y + sag), b]
}

fn bezier_mid(c: &[Pos2; 4]) -> Pos2 {
    let w = [0.125, 0.375, 0.375, 0.125];
    pos2(c.iter().zip(w).map(|(p, w)| p.x * w).sum(), c.iter().zip(w).map(|(p, w)| p.y * w).sum())
}

impl App {
    /// The track the Touch view plays: the selected one, if it has an instrument.
    pub(crate) fn touch_track(&self) -> Option<usize> {
        let TrackSel::Track(t) = self.sel_track else { return None };
        let tr = self.project.tracks.get(t)?;
        (tr.kind == TrackKind::Midi && tr.instrument.is_some()).then_some(t)
    }

    /// The track's Touch device; the first time, it is put at the front of
    /// its MIDI chain, with six of the instrument's knobs on its patch points
    /// and the pressure patched to the first.
    pub(crate) fn touch_device(&mut self, ti: usize) -> usize {
        if let Some(i) = self.project.tracks[ti].midi_devices.iter().position(|d| d.kind == MidiFxKind::Touch) {
            return i;
        }
        let tid = self.project.tracks[ti].id;
        let specs = self.project.tracks[ti].instrument.as_ref().and_then(instrument_specs).unwrap_or(&[]);
        let picks = touch_picks(specs);
        let mut dev = self.project.make_midi_device(MidiFxKind::Touch);
        for (k, &param) in picks.iter().enumerate() {
            touch::set_patch(&mut dev.data, k, Some(param as u16));
        }
        self.project.tracks[ti].midi_devices.insert(0, dev);
        if let Some(&first) = picks.first() {
            self.touch_connect(tid, 0, first as u16, 0.5);
        }
        0
    }

    /// A cable from output `out` to the instrument's parameter `param`.
    fn touch_connect(&mut self, track: u64, out: usize, param: u16, amount: f32) {
        let id = match touch_mod(self, track, out) {
            Some(id) => id,
            None => {
                if self.project.modulators.len() >= MAX_MODS {
                    self.toast(format!("At most {MAX_MODS} modulators: take one away in the Mod tab"));
                    return;
                }
                let mut m = self.project.make_modulator(ModKind::Touch);
                m.params[0] = out as f32;
                m.track = Some(track);
                let tname = self.project.tracks.iter().find(|t| t.id == track).map(|t| t.name.clone()).unwrap_or_default();
                m.name = format!("Touch {} ({tname})", OUTPUTS[out]);
                let id = m.id;
                self.project.modulators.push(m);
                id
            }
        };
        self.project.add_route(id, ModTarget::Param { owner: ModOwner::Instrument(track), param }, amount);
    }

    /// Takes a cable away (and its modulator, when nothing else is patched from it).
    fn touch_unpatch(&mut self, route: u64) {
        let source = self.project.mod_routes.iter().find(|r| r.id == route).map(|r| r.source);
        self.project.mod_routes.retain(|r| r.id != route);
        if let Some(src) = source {
            let used = self.project.mod_routes.iter().any(|r| r.source == src || r.target == ModTarget::Param { owner: ModOwner::Modulator(src), param: 0 });
            if !used && self.project.modulators.iter().any(|m| m.id == src && m.kind == ModKind::Touch) {
                self.project.modulators.retain(|m| m.id != src);
                self.project.prune_mod_routes();
            }
        }
    }

    /// The cables of a track's Touch: (output, patch point, route, amount).
    fn touch_cables(&self, track: u64, data: &[u8]) -> Vec<(usize, usize, u64, f32)> {
        let mut out = Vec::new();
        for k in 0..OUTPUTS.len() {
            let Some(src) = touch_mod(self, track, k) else { continue };
            for r in self.project.mod_routes.iter().filter(|r| r.source == src) {
                if let ModTarget::Param { owner: ModOwner::Instrument(t), param } = r.target {
                    if let Some(slot) = (0..N_PATCH).find(|&s| t == track && touch::patch(data, s) == Some(param)) {
                        out.push((k, slot, r.id, r.amount));
                    }
                }
            }
        }
        out
    }

    /// A plate pressed from the keys (`step` scale steps above the base note).
    pub(crate) fn touch_key(&mut self, step: i32) -> Option<(u8, Vec<u64>)> {
        let ti = self.touch_track()?;
        let di = self.touch_device(ti);
        let base = self.project.tracks[ti].midi_devices[di].params.get(touch::BASE).copied().unwrap_or(48.0);
        let key = self.project.sounding_key();
        let (d0, _) = key.locate(base.round() as i32);
        let pitch = key.pitch_at(d0 + step).clamp(0, 127) as u8;
        let id = self.project.tracks[ti].id;
        let vel = self.kb_velocity;
        self.cmd(Command::Touch { track: id, pressure: vel, position: 0.5 });
        self.cmd(Command::NoteOn { track: id, pitch, velocity: vel });
        Some((pitch, vec![id]))
    }

    pub(crate) fn ui_touch(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, BG);
        let Some(ti) = self.touch_track() else {
            self.touch_pick(ui, full);
            return;
        };
        let di = self.touch_device(ti);
        let tid = self.project.tracks[ti].id;
        let now = ui.input(|i| i.time);
        let status = self.status();
        let vis = status.seqs.read(self.project.tracks[ti].midi_devices[di].id);
        let (step, _, pulses, _, outs) = vis.map(touch::read_visual).unwrap_or((0, 5, 0, 0, [0.0; 6]));
        if pulses != self.touch_view.pulses {
            self.touch_view.pulses = pulses;
            self.touch_view.flash = now;
        }
        let lamp = (1.0 - ((now - self.touch_view.flash) / 0.12) as f32).clamp(0.0, 1.0);
        let accent = accent_now();
        let specs = touch::TOUCH;
        let inst_specs = self.project.tracks[ti].instrument.as_ref().and_then(instrument_specs).unwrap_or(&[]);
        let held_notes = status.tracks.get(ti).map(|t| t.inst_notes()).unwrap_or(0);
        let key = self.project.sounding_key();
        let base_note = self.project.tracks[ti].midi_devices[di].params[touch::BASE].round() as i32;

        let r = full.shrink2(vec2(12.0, 10.0));
        let header = Rect::from_min_size(r.min, vec2(r.width(), 28.0));
        let body = Rect::from_min_max(pos2(r.left(), header.bottom() + 8.0), r.max);
        let plates_h = (body.height() * 0.42).clamp(90.0, 320.0);
        let plates = Rect::from_min_max(pos2(body.left(), body.bottom() - plates_h), body.max);
        let top = Rect::from_min_max(body.min, pos2(body.right(), plates.top() - 10.0));
        let w = top.width();
        let pulser = Rect::from_min_size(top.min, vec2((w * 0.24).max(200.0), top.height()));
        let steps_r = Rect::from_min_size(pos2(pulser.right() + 10.0, top.top()), vec2(w * 0.34 - 10.0, top.height()));
        let patch_r = Rect::from_min_max(pos2(steps_r.right() + 10.0, top.top()), top.max);
        let p = ui.painter().clone();

        // ---------------------------------------------------------------- header
        let tr = &self.project.tracks[ti];
        let tcol = Color32::from_rgb(tr.color[0], tr.color[1], tr.color[2]);
        p.text(header.left_center(), Align2::LEFT_CENTER, "TOUCH", FontId::proportional(18.0), accent);
        let chip = Rect::from_min_size(pos2(header.left() + 78.0, header.top() + 2.0), vec2(250.0, 24.0));
        let chip_resp = ui.interact(chip, Id::new("touch_track"), Sense::click()).on_hover_text("The track Touch plays: click for another");
        p.rect_filled(chip, 5.0, if chip_resp.hovered() { PANEL3 } else { PANEL2 });
        p.rect_filled(Rect::from_min_size(chip.min, vec2(5.0, chip.height())), 2.0, tcol);
        let inst_name = tr.instrument.as_ref().map(|i| i.track_name()).unwrap_or_default();
        p.text(pos2(chip.left() + 12.0, chip.center().y), Align2::LEFT_CENTER, format!("{}  ·  {inst_name}", tr.name), FontId::proportional(12.5), TEXT);
        down_arrow(&p, pos2(chip.right() - 11.0, chip.center().y));
        let mut pick_track = None;
        egui::Popup::menu(&chip_resp).show(|ui| {
            for (i, t) in self.project.tracks.iter().enumerate() {
                if t.kind == TrackKind::Midi && t.instrument.is_some() && ui.selectable_label(i == ti, &t.name).clicked() {
                    pick_track = Some(i);
                    ui.close();
                }
            }
        });
        // Its presets (the steps and the settings; the patch stays).
        let preset_r = Rect::from_min_size(pos2(chip.right() + 10.0, chip.top()), vec2(170.0, 24.0));
        let preset_resp = ui.interact(preset_r, Id::new("touch_preset"), Sense::click()).on_hover_text("Touch's presets: the pulser, chance and steps (the patch stays)");
        let preset_name = self.project.tracks[ti].midi_devices[di].preset.clone();
        p.rect_filled(preset_r, 5.0, if preset_resp.hovered() { PANEL3 } else { PANEL2 });
        p.text(pos2(preset_r.left() + 10.0, preset_r.center().y), Align2::LEFT_CENTER, if preset_name.is_empty() { "Presets" } else { preset_name.as_str() }, FontId::proportional(12.0), TEXT);
        down_arrow(&p, pos2(preset_r.right() - 11.0, preset_r.center().y));
        let mut pick_preset = None;
        egui::Popup::menu(&preset_resp).show(|ui| {
            for (i, pr) in krabton_engine::presets::midi(MidiFxKind::Touch).iter().enumerate() {
                if ui.selectable_label(pr.name == preset_name, pr.name).clicked() {
                    pick_preset = Some(i);
                    ui.close();
                }
            }
        });
        // How to play it, where there is room.
        let hint = "higher up a plate: harder  ·  drag across: slide  ·  keys A–L and W–P play the plates";
        let hw = p.layout_no_wrap(hint.into(), FontId::proportional(11.0), TEXT_DIM).size().x;
        if header.right() - hw > preset_r.right() + 20.0 {
            p.text(header.right_center(), Align2::RIGHT_CENTER, hint, FontId::proportional(11.0), TEXT_DIM);
        }

        let dev = &mut self.project.tracks[ti].midi_devices[di];
        let mut changed = false;

        // ---------------------------------------------------------------- pulser and chance
        let inner = section(&p, pulser, "PULSER  ·  CHANCE");
        let half = (inner.width() - 8.0) / 2.0;
        let pulse_on = dev.params[touch::PULSE] >= 0.5;
        let latch_on = dev.params[touch::LATCH] >= 0.5;
        let row = Rect::from_min_size(inner.min, vec2(half, 28.0));
        if toggle(ui, row, Id::new(("touch_pulse", tid)), "Pulse", pulse_on, Some(if pulse_on { lamp } else { 0.0 }), "The pulser: plays the held (or latched) plate again in time, also with the song stopped") {
            dev.params[touch::PULSE] = if pulse_on { 0.0 } else { 1.0 };
            changed = true;
        }
        let row2 = Rect::from_min_size(pos2(row.right() + 8.0, row.top()), vec2(half, 28.0));
        if toggle(ui, row2, Id::new(("touch_latch", tid)), "Latch", latch_on, None, "Keeps the last plate going after you let go; the next touch takes over") {
            dev.params[touch::LATCH] = if latch_on { 0.0 } else { 1.0 };
            changed = true;
        }
        let rate_r = Rect::from_min_size(pos2(inner.left(), row.bottom() + 6.0), vec2(inner.width(), 22.0));
        changed |= choice(ui, rate_r, Id::new(("touch_rate", tid)), &specs[touch::RATE], &mut dev.params[touch::RATE]);
        let kn = Rect::from_min_max(pos2(inner.left(), rate_r.bottom() + 8.0), inner.max);
        let kw = kn.width() / 2.0;
        let kh = kn.height() / 2.0;
        for (n, k) in [touch::DECAY, touch::GATE, touch::CHANCE, touch::WANDER].into_iter().enumerate() {
            let cell = Rect::from_min_size(pos2(kn.left() + (n % 2) as f32 * kw, kn.top() + (n / 2) as f32 * kh), vec2(kw, kh)).shrink(2.0);
            let color = if n < 2 { OUT_COLORS[1] } else { OUT_COLORS[3] };
            changed |= knob_param(ui, cell, Id::new(("touch_knob", tid, k)), &specs[k], &mut dev.params[k], color);
        }

        // ---------------------------------------------------------------- steps
        let inner = section(&p, steps_r, "STEPS");
        let ctl = Rect::from_min_size(inner.min, vec2(inner.width(), 22.0));
        let cw = (ctl.width() - 12.0) / 3.0;
        let move_on = dev.params[touch::MOVE] >= 0.5;
        if toggle(ui, Rect::from_min_size(ctl.min, vec2(cw, 22.0)), Id::new(("touch_move", tid)), "Move Notes", move_on, None, "The upper row moves the notes through the key (each touch, or each pulse, a step further)") {
            dev.params[touch::MOVE] = if move_on { 0.0 } else { 1.0 };
            changed = true;
        }
        changed |= choice(ui, Rect::from_min_size(pos2(ctl.left() + cw + 6.0, ctl.top()), vec2(cw, 22.0)), Id::new(("touch_order", tid)), &specs[touch::ORDER], &mut dev.params[touch::ORDER]);
        // How many steps: click the numbers below.
        let n_on = (dev.params[touch::STEPS].round() as usize).clamp(1, N_STEPS);
        p.text(pos2(ctl.right(), ctl.center().y), Align2::RIGHT_CENTER, format!("{n_on} steps"), FontId::proportional(11.5), TEXT_DIM);
        let grid = Rect::from_min_max(pos2(inner.left(), ctl.bottom() + 10.0), inner.max);
        let col_w = grid.width() / N_STEPS as f32;
        let note_h = (grid.height() - 26.0) * 0.55;
        let val_h = grid.height() - 26.0 - note_h - 8.0;
        for i in 0..N_STEPS {
            let col = Rect::from_min_size(pos2(grid.left() + i as f32 * col_w, grid.top()), vec2(col_w, grid.height())).shrink2(vec2(5.0, 0.0));
            let active = i < n_on;
            let here = i == step && active;
            let (_, b) = touch::step(&dev.data, i);
            if here {
                p.rect_filled(col.expand(3.0), 5.0, shade(accent, 0.18));
            }
            // The note: up and down from the middle, in scale steps.
            let nr = Rect::from_min_size(col.min, vec2(col.width(), note_h));
            let resp = ui.interact(nr, Id::new(("touch_note", tid, i)), Sense::click_and_drag()).on_hover_text("This step's note: up and down through the key (double-click: back to the middle)");
            if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
                let v = (((nr.center().y - pt.y) / (nr.height() * 0.5)) * 7.0).round() as i32;
                touch::set_step(&mut dev.data, i, v, b);
                changed = true;
            }
            if resp.double_clicked() {
                touch::set_step(&mut dev.data, i, 0, b);
                changed = true;
            }
            let (a, _) = touch::step(&dev.data, i);
            p.rect_filled(nr, 4.0, SLOT);
            p.line_segment([pos2(nr.left() + 4.0, nr.center().y), pos2(nr.right() - 4.0, nr.center().y)], Stroke::new(1.0_f32, LINE));
            let y = nr.center().y - a as f32 / 7.0 * nr.height() * 0.5;
            let c = if active { OUT_COLORS[2] } else { TEXT_DIM };
            let bar = Rect::from_two_pos(pos2(nr.left() + 6.0, nr.center().y), pos2(nr.right() - 6.0, y));
            p.rect_filled(bar, 2.0, shade(c, if active { 0.75 } else { 0.35 }));
            p.line_segment([pos2(nr.left() + 4.0, y), pos2(nr.right() - 4.0, y)], Stroke::new(2.0_f32, c));
            let (d0, rest) = key.locate(base_note);
            let name = krabton_engine::note_name((key.pitch_at(d0 + a) + rest).clamp(0, 127) as u8);
            p.text(pos2(nr.center().x, nr.top() + 3.0), Align2::CENTER_TOP, format!("{a:+}"), FontId::proportional(10.5), if active { TEXT } else { TEXT_DIM });
            p.text(pos2(nr.center().x, nr.bottom() - 3.0), Align2::CENTER_BOTTOM, name, FontId::proportional(9.5), TEXT_DIM);
            // The value: patched from the Steps output.
            let vr = Rect::from_min_size(pos2(col.left(), nr.bottom() + 8.0), vec2(col.width(), val_h));
            let resp = ui.interact(vr, Id::new(("touch_val", tid, i)), Sense::click_and_drag()).on_hover_text("This step's value, for the Steps output (patch it to a knob)");
            if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
                touch::set_step(&mut dev.data, i, a, (vr.bottom() - pt.y) / vr.height());
                changed = true;
            }
            let (_, b) = touch::step(&dev.data, i);
            p.rect_filled(vr, 4.0, SLOT);
            let fill = Rect::from_min_max(pos2(vr.left() + 6.0, vr.bottom() - b * vr.height()), pos2(vr.right() - 6.0, vr.bottom()));
            p.rect_filled(fill, 2.0, shade(c, if active { 0.6 } else { 0.25 }));
            // Its number: a click sets how many steps there are.
            let nb = Rect::from_min_size(pos2(col.left(), grid.bottom() - 18.0), vec2(col.width(), 18.0));
            let resp = ui.interact(nb, Id::new(("touch_n", tid, i)), Sense::click()).on_hover_text("Click: this many steps");
            if resp.clicked() {
                dev.params[touch::STEPS] = (i + 1) as f32;
                changed = true;
            }
            p.circle_filled(pos2(nb.left() + 10.0, nb.center().y), 4.0, if here { mix(c, Color32::WHITE, lamp * 0.6) } else if active { shade(c, 0.4) } else { PANEL3 });
            p.text(pos2(nb.center().x + 6.0, nb.center().y), Align2::CENTER_CENTER, format!("{}", i + 1), FontId::proportional(11.0), if active { TEXT } else { TEXT_DIM });
        }

        // ---------------------------------------------------------------- patch bay
        let inner = section(&p, patch_r, "PATCH");
        let data = self.project.tracks[ti].midi_devices[di].data.clone();
        let cables = self.touch_cables(tid, &data);
        let row_h = (inner.height() / OUTPUTS.len() as f32).min(34.0);
        let jack_r = 7.0;
        let src_x = inner.left() + 78.0;
        let dst_x = inner.right() - (inner.width() * 0.42).clamp(120.0, 210.0);
        let src_at = |k: usize| pos2(src_x, inner.top() + row_h * (k as f32 + 0.5));
        let dst_at = |k: usize| pos2(dst_x, inner.top() + row_h * (k as f32 + 0.5));
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let released = ui.input(|i| i.pointer.any_released());
        let mut connect: Option<(usize, usize)> = None;
        let mut unpatch: Option<u64> = None;
        let mut amounts: Vec<(u64, f32)> = Vec::new();
        let mut set_slot: Option<(usize, Option<u16>)> = None;
        // The outputs.
        for k in 0..OUTPUTS.len() {
            let c = src_at(k);
            let jr = Rect::from_center_size(c, vec2(jack_r * 2.6, jack_r * 2.6));
            let resp = ui.interact(jr, Id::new(("touch_src", tid, k)), Sense::drag()).on_hover_text(format!("{}: {}. Drag to a knob on the right", OUTPUTS[k], OUT_HELP[k]));
            if resp.drag_started() {
                self.touch_view.drawing = Some((true, k));
            }
            if resp.hovered() || resp.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
            let v = outs[k];
            p.circle_filled(c, jack_r + 3.0, mix(PANEL2, OUT_COLORS[k], 0.15 + 0.6 * v));
            p.circle_filled(c, jack_r, Color32::from_rgb(0x10, 0x10, 0x12));
            p.circle_stroke(c, jack_r, Stroke::new(2.0_f32, OUT_COLORS[k]));
            p.text(pos2(c.x - jack_r - 8.0, c.y), Align2::RIGHT_CENTER, OUTPUTS[k], FontId::proportional(11.5), TEXT);
            // A meter of what it gives now.
            let m = Rect::from_min_size(pos2(c.x + jack_r + 6.0, c.y - 2.0), vec2(28.0, 4.0));
            p.rect_filled(m, 2.0, SLOT);
            p.rect_filled(Rect::from_min_size(m.min, vec2(m.width() * v, m.height())), 2.0, OUT_COLORS[k]);
        }
        // The patch points: the instrument's knobs.
        for s in 0..N_PATCH {
            let c = dst_at(s);
            let param = touch::patch(&data, s);
            let jr = Rect::from_center_size(c, vec2(jack_r * 2.6, jack_r * 2.6));
            let resp = ui.interact(jr, Id::new(("touch_dst", tid, s)), Sense::drag()).on_hover_text("Drag to an output on the left to patch it here");
            if resp.drag_started() && param.is_some() {
                self.touch_view.drawing = Some((false, s));
            }
            // How far the cables move it now.
            let moved: f32 = cables.iter().filter(|c| c.1 == s).map(|c| c.3 * outs[c.0]).sum();
            p.circle_filled(c, jack_r + 3.0, mix(PANEL2, accent, (moved.abs() * 0.8).min(0.8)));
            p.circle_filled(c, jack_r, Color32::from_rgb(0x10, 0x10, 0x12));
            p.circle_stroke(c, jack_r, Stroke::new(2.0_f32, if param.is_some() { TEXT } else { LINE }));
            let label = Rect::from_min_max(pos2(c.x + jack_r + 8.0, c.y - 11.0), pos2(inner.right(), c.y + 11.0));
            let lresp = ui.interact(label, Id::new(("touch_dst_pick", tid, s)), Sense::click()).on_hover_text("The knob this point moves: click for another");
            p.rect_filled(label, 4.0, if lresp.hovered() { PANEL3 } else { SLOT });
            let name = param.and_then(|i| inst_specs.get(i as usize)).map(|sp| sp.name).unwrap_or("—");
            p.text(pos2(label.left() + 8.0, label.center().y), Align2::LEFT_CENTER, name, FontId::proportional(11.5), if param.is_some() { TEXT } else { TEXT_DIM });
            // The pull, as a little bar along the bottom.
            if moved.abs() > 0.002 {
                let mid = label.center().x;
                let wbar = (moved.clamp(-1.0, 1.0)) * label.width() * 0.45;
                p.rect_filled(Rect::from_two_pos(pos2(mid, label.bottom() - 3.0), pos2(mid + wbar, label.bottom() - 1.0)), 1.0, accent);
            }
            egui::Popup::menu(&lresp).show(|ui| {
                if ui.selectable_label(param.is_none(), "—  (nothing)").clicked() {
                    set_slot = Some((s, None));
                    ui.close();
                }
                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    let mut group = "";
                    for (i, sp) in inst_specs.iter().enumerate() {
                        if !continuous(sp) {
                            continue;
                        }
                        if sp.group != group {
                            group = sp.group;
                            ui.label(egui::RichText::new(group).small().color(TEXT_DIM));
                        }
                        if ui.selectable_label(param == Some(i as u16), sp.name).clicked() {
                            set_slot = Some((s, Some(i as u16)));
                            ui.close();
                        }
                    }
                });
            });
        }
        // The cables, and their amounts.
        for &(k, s, rid, amount) in &cables {
            let pts = bezier(src_at(k), dst_at(s));
            let glow = 0.45 + 0.55 * outs[k];
            p.add(CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, Stroke::new(4.0_f32, shade(OUT_COLORS[k], 0.3))));
            p.add(CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, Stroke::new(2.2_f32, shade(OUT_COLORS[k], glow))));
            let mid = bezier_mid(&pts);
            let kr = Rect::from_center_size(mid, vec2(20.0, 20.0));
            let pname = touch::patch(&data, s).and_then(|i| inst_specs.get(i as usize)).map(|sp| sp.name).unwrap_or("?");
            let resp = ui
                .interact(kr, Id::new(("touch_amt", rid)), Sense::click_and_drag())
                .on_hover_text(format!("{} → {}: {:+.0}%. Drag up or down; double-click takes the cable away", OUTPUTS[k], pname, amount * 100.0));
            let mut a = amount;
            if resp.dragged() {
                let fine = ui.input(|i| i.modifiers.shift);
                a = (a - resp.drag_delta().y * if fine { 0.002 } else { 0.008 }).clamp(-1.0, 1.0);
                amounts.push((rid, a));
            }
            if resp.double_clicked() || resp.secondary_clicked() {
                unpatch = Some(rid);
            }
            p.circle_filled(mid, 9.0, Color32::from_rgb(0x14, 0x14, 0x16));
            p.circle_stroke(mid, 9.0, Stroke::new(1.0_f32, shade(OUT_COLORS[k], 0.6)));
            let start = -std::f32::consts::FRAC_PI_2;
            let arc: Vec<Pos2> = (0..=16).map(|j| {
                let t = start + a * std::f32::consts::PI * j as f32 / 16.0;
                mid + vec2(t.cos(), t.sin()) * 6.0
            }).collect();
            p.add(egui::Shape::line(arc, Stroke::new(2.0_f32, OUT_COLORS[k])));
        }
        // A cable being drawn.
        if let (Some((from_src, k)), Some(pt)) = (self.touch_view.drawing, pointer) {
            let (a, col) = if from_src { (src_at(k), OUT_COLORS[k]) } else { (dst_at(k), TEXT) };
            let pts = if from_src { bezier(a, pt) } else { bezier(pt, a) };
            p.add(CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, Stroke::new(2.2_f32, col)));
            if released {
                let near = |c: Pos2| pt.distance(c) < jack_r * 2.2;
                if from_src {
                    if let Some(s) = (0..N_PATCH).find(|&s| near(dst_at(s)) && touch::patch(&data, s).is_some()) {
                        connect = Some((k, s));
                    }
                } else if let Some(o) = (0..OUTPUTS.len()).find(|&o| near(src_at(o))) {
                    connect = Some((o, k));
                }
                self.touch_view.drawing = None;
            }
        }
        if cables.is_empty() && self.touch_view.drawing.is_none() {
            p.text(pos2((src_x + dst_x) * 0.5, inner.bottom() - 4.0), Align2::CENTER_BOTTOM, "drag from an output to a knob", FontId::proportional(11.0), TEXT_DIM);
        }

        // ---------------------------------------------------------------- the plates
        let (d0, _) = key.locate(base_note);
        let per_octave = (key.locate(base_note + 12).0 - d0).max(1);
        let n = ((per_octave * 2 + 1) as usize).clamp(8, 16);
        let pitches: Vec<u8> = (0..n).map(|i| key.pitch_at(d0 + i as i32).clamp(0, 127) as u8).collect();
        let label_h = 18.0;
        let area = Rect::from_min_max(plates.min, pos2(plates.right(), plates.bottom() - label_h));
        let gap = 6.0;
        let pw = (area.width() - gap * (n as f32 - 1.0)) / n as f32;
        let plate_at = |i: usize| Rect::from_min_size(pos2(area.left() + i as f32 * (pw + gap), area.top()), vec2(pw, area.height()));
        let resp = ui.interact(area, Id::new(("touch_plates", tid)), Sense::click_and_drag());
        // What presses: each finger on a touch screen (a chord, its force as
        // the pressure where the screen measures it), or else the pointer.
        let plate_of = |pt: Pos2| (((pt.x - area.left()) / (pw + gap)).floor().max(0.0) as usize).min(n - 1);
        let press_of = |pt: Pos2| (1.0 - (pt.y - area.top()) / area.height()).clamp(0.05, 1.0);
        let along = |pt: Pos2| ((pt.x - area.left()) / area.width()).clamp(0.0, 1.0);
        for ev in ui.input(|i| i.events.clone()) {
            if let egui::Event::Touch { id, phase, pos, force, .. } = ev {
                let fid = id.0.wrapping_add(1);
                let fingers = &mut self.touch_view.fingers;
                match phase {
                    egui::TouchPhase::Start if area.contains(pos) => fingers.push((fid, pos, force)),
                    egui::TouchPhase::Move => {
                        if let Some(f) = fingers.iter_mut().find(|f| f.0 == fid) {
                            (f.1, f.2) = (pos, force);
                        }
                    }
                    egui::TouchPhase::End | egui::TouchPhase::Cancel => fingers.retain(|f| f.0 != fid),
                    _ => {}
                }
            }
        }
        let mut want: Vec<(u8, f32, f32)> = self.touch_view.fingers.iter().map(|&(_, pt, force)| (pitches[plate_of(pt)], force.filter(|f| *f > 0.0).map(|f| f.clamp(0.05, 1.0)).unwrap_or_else(|| press_of(pt)), along(pt))).collect();
        let touching = !self.touch_view.fingers.is_empty() || ui.input(|i| i.any_touches());
        if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.is_pointer_button_down_on() && !touching) {
            want.push((pitches[plate_of(pt)], press_of(pt), along(pt)));
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        // Keys held for this track press too.
        let keys_down = self.held_keys.values().any(|(_, ids)| ids.contains(&tid));
        for (i, &pitch) in pitches.iter().enumerate() {
            let pr = plate_at(i);
            let sounding = held_notes >> pitch & 1 == 1;
            let held = want.iter().any(|w| w.0 == pitch) || self.held_keys.values().any(|(pp, ids)| *pp == pitch && ids.contains(&tid));
            let root = key.locate(pitch as i32).0.rem_euclid(per_octave) == 0;
            let (t, b) = (Color32::from_rgb(0x9a, 0x6a, 0x46), Color32::from_rgb(0x3a, 0x26, 0x1a));
            gradient(&p, pr, if held { mix(t, accent, 0.4) } else { t }, if held { mix(b, accent, 0.3) } else { b });
            if sounding {
                p.rect_filled(pr, 4.0, Color32::from_rgba_unmultiplied(tcol.r(), tcol.g(), tcol.b(), 70));
                p.rect_stroke(pr, 4.0, Stroke::new(2.0_f32, mix(tcol, Color32::WHITE, 0.3)), egui::StrokeKind::Inside);
            } else {
                p.rect_stroke(pr, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(0x1a, 0x12, 0x0c)), egui::StrokeKind::Inside);
            }
            // The ridges of the plate.
            for j in 1..4 {
                let x = pr.left() + pr.width() * j as f32 / 4.0;
                p.line_segment([pos2(x, pr.top() + 6.0), pos2(x, pr.bottom() - 6.0)], Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0, 0, 0, 40)));
            }
            for &(_, pressure, _) in want.iter().filter(|w| w.0 == pitch) {
                let y = pr.bottom() - pressure * pr.height();
                p.line_segment([pos2(pr.left() + 3.0, y), pos2(pr.right() - 3.0, y)], Stroke::new(2.0_f32, Color32::WHITE));
            }
            if root {
                p.circle_filled(pos2(pr.center().x, pr.top() + 9.0), 3.0, shade(accent, 0.9));
            }
            p.text(pos2(pr.center().x, pr.bottom() + 3.0), Align2::CENTER_TOP, krabton_engine::note_name(pitch), FontId::proportional(10.5), if sounding || held { TEXT } else { TEXT_DIM });
        }
        // Base note: an octave lower or higher.
        for (dir, glyph) in [(-1, "◀"), (1, "▶")] {
            let x = if dir < 0 { plates.left() + 8.0 } else { plates.right() - 8.0 };
            let br = Rect::from_center_size(pos2(x, plates.bottom() - label_h * 0.5 + 1.0), vec2(16.0, 16.0));
            let r2 = ui.interact(br, Id::new(("touch_oct", tid, dir)), Sense::click()).on_hover_text(if dir < 0 { "The plates an octave lower" } else { "The plates an octave higher" });
            p.text(br.center(), Align2::CENTER_CENTER, glyph, FontId::proportional(12.0), if r2.hovered() { TEXT } else { TEXT_DIM });
            if r2.clicked() {
                let d = &mut self.project.tracks[ti].midi_devices[di];
                d.params[touch::BASE] = (d.params[touch::BASE] + 12.0 * dir as f32).clamp(24.0, 84.0);
                changed = true;
            }
        }

        // ---------------------------------------------------------------- what was asked
        // How hard (the hardest press) and where (the last finger), when it changes.
        let (pressure, position) = match want.iter().fold(None::<(f32, f32)>, |a, w| Some((a.map_or(w.1, |a| a.0.max(w.1)), w.2))) {
            Some(pp) => pp,
            None if keys_down => (self.kb_velocity, self.touch_view.sent.map(|s| s.2).unwrap_or(0.5)),
            None => (0.0, self.touch_view.sent.filter(|s| s.0 == tid).map(|s| s.2).unwrap_or(0.5)),
        };
        // The plates pressed now against before: what is let go stops, what is
        // new starts (sliding across lets one go and takes the next).
        let before = std::mem::take(&mut self.touch_view.held);
        for &(pitch, t) in &before {
            if t != tid || !want.iter().any(|w| w.0 == pitch) {
                self.cmd(Command::NoteOff { track: t, pitch });
            }
        }
        let mut now_held: Vec<(u8, u64)> = Vec::with_capacity(want.len());
        for &(pitch, pr, po) in &want {
            if now_held.iter().any(|h| h.0 == pitch) {
                continue;
            }
            if !before.iter().any(|&(bp, t)| bp == pitch && t == tid) {
                // The pressure first, so the note starts with it.
                self.cmd(Command::Touch { track: tid, pressure: pressure.max(pr), position: po });
                self.cmd(Command::NoteOn { track: tid, pitch, velocity: pr });
            }
            now_held.push((pitch, tid));
        }
        self.touch_view.held = now_held;
        let fresh = match self.touch_view.sent {
            Some((t, pr, po)) => t != tid || (pr - pressure).abs() > 0.004 || (po - position).abs() > 0.004,
            None => true,
        };
        if fresh {
            self.cmd(Command::Touch { track: tid, pressure, position });
            self.touch_view.sent = Some((tid, pressure, position));
        }
        if let Some((s, param)) = set_slot {
            // The cables into it follow it (or go, with nothing there).
            let routes: Vec<u64> = cables.iter().filter(|c| c.1 == s).map(|c| c.2).collect();
            match param {
                Some(np) => {
                    for r in self.project.mod_routes.iter_mut().filter(|r| routes.contains(&r.id)) {
                        r.target = ModTarget::Param { owner: ModOwner::Instrument(tid), param: np };
                    }
                }
                None => {
                    for rid in routes {
                        self.touch_unpatch(rid);
                    }
                }
            }
            touch::set_patch(&mut self.project.tracks[ti].midi_devices[di].data, s, param);
        }
        for (rid, a) in amounts {
            if let Some(r) = self.project.mod_routes.iter_mut().find(|r| r.id == rid) {
                r.amount = a;
            }
        }
        if let Some(rid) = unpatch {
            self.touch_unpatch(rid);
        }
        if let Some((k, s)) = connect {
            if let Some(param) = touch::patch(&data, s) {
                if !cables.iter().any(|c| c.0 == k && c.1 == s) {
                    self.touch_connect(tid, k, param, 0.5);
                }
            }
        }
        if let Some(i) = pick_preset {
            let pr = &krabton_engine::presets::midi(MidiFxKind::Touch)[i];
            let d = &mut self.project.tracks[ti].midi_devices[di];
            d.params = krabton_engine::presets::apply_to(touch::TOUCH, pr, &d.params);
            if let Some(f) = pr.data {
                // The steps from the preset, the patch as it was.
                let fresh = f();
                let keep = d.data.clone();
                d.data = fresh;
                for s in 0..N_PATCH {
                    touch::set_patch(&mut d.data, s, touch::patch(&keep, s));
                }
            }
            d.preset = pr.name.to_string();
        }
        if changed {
            self.project.tracks[ti].midi_devices[di].preset.clear();
        }
        if let Some(i) = pick_track {
            self.touch_release_all();
            self.sel_track = TrackSel::Track(i);
        }
    }

    /// Whether the Touch view holds a plate or a pressure.
    pub(crate) fn touch_view_holds(&self) -> bool {
        !self.touch_view.held.is_empty() || self.touch_view.sent.is_some_and(|s| s.1 > 0.0)
    }

    /// Lets go of the plate held (leaving the view, another track).
    pub(crate) fn touch_release_all(&mut self) {
        for (pitch, t) in std::mem::take(&mut self.touch_view.held) {
            self.cmd(Command::NoteOff { track: t, pitch });
        }
        self.touch_view.fingers.clear();
        if let Some((t, _, po)) = self.touch_view.sent.take() {
            self.cmd(Command::Touch { track: t, pressure: 0.0, position: po });
        }
    }

    /// No track to play: the ones that could be.
    fn touch_pick(&mut self, ui: &mut Ui, r: Rect) {
        let p = ui.painter().clone();
        let c = r.center();
        p.text(pos2(c.x, c.y - 70.0), Align2::CENTER_CENTER, "TOUCH", FontId::proportional(26.0), accent_now());
        p.text(pos2(c.x, c.y - 38.0), Align2::CENTER_CENTER, "Pick a track with an instrument to play with plates, a pulser, steps and cables", FontId::proportional(13.0), TEXT_DIM);
        let tracks: Vec<(usize, String, Color32)> = self.project.tracks.iter().enumerate().filter(|(_, t)| t.kind == TrackKind::Midi && t.instrument.is_some()).map(|(i, t)| (i, t.name.clone(), Color32::from_rgb(t.color[0], t.color[1], t.color[2]))).collect();
        if tracks.is_empty() {
            p.text(pos2(c.x, c.y), Align2::CENTER_CENTER, "There is none yet: add an instrument from the browser", FontId::proportional(13.0), TEXT);
            return;
        }
        let w = 220.0;
        let cols = ((r.width() - 40.0) / (w + 10.0)).floor().max(1.0) as usize;
        for (n, (i, name, col)) in tracks.into_iter().enumerate() {
            let x = c.x + ((n % cols) as f32 - (cols as f32 - 1.0) * 0.5) * (w + 10.0);
            let b = Rect::from_center_size(pos2(x, c.y + (n / cols) as f32 * 36.0), vec2(w, 30.0));
            let resp = ui.interact(b, Id::new(("touch_pick", i)), Sense::click());
            p.rect_filled(b, 5.0, if resp.hovered() { PANEL3 } else { PANEL2 });
            p.rect_filled(Rect::from_min_size(b.min, vec2(5.0, b.height())), 2.0, col);
            p.text(pos2(b.left() + 14.0, b.center().y), Align2::LEFT_CENTER, name, FontId::proportional(12.5), TEXT);
            if resp.clicked() {
                self.sel_track = TrackSel::Track(i);
            }
        }
    }
}

/// Touch in the track's MIDI chain (the Devices tab): its five steps (drag
/// them) and what it gives out now; it is played in the Touch view.
pub(crate) fn device_editor(ui: &mut Ui, id: Id, params: &[f32], data: &mut Vec<u8>, live: super::seq_view::Live, height: f32) -> bool {
    let w = 280.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let (step, _, _, _, outs) = live.vis.map(touch::read_visual).unwrap_or((0, 5, 0, 0, [0.0; 6]));
    let n_on = (params.get(touch::STEPS).copied().unwrap_or(5.0).round() as usize).clamp(1, N_STEPS);
    let mut changed = false;
    let inner = rect.shrink(6.0);
    let foot = 16.0;
    let steps = Rect::from_min_max(inner.min, pos2(inner.left() + inner.width() * 0.62, inner.bottom() - foot));
    let col_w = steps.width() / N_STEPS as f32;
    for i in 0..N_STEPS {
        let col = Rect::from_min_size(pos2(steps.left() + i as f32 * col_w, steps.top()), vec2(col_w, steps.height())).shrink2(vec2(3.0, 0.0));
        let active = i < n_on;
        let c = if active { OUT_COLORS[2] } else { TEXT_DIM };
        let (a, b) = touch::step(data, i);
        let nr = Rect::from_min_size(col.min, vec2(col.width(), col.height() * 0.6));
        let vr = Rect::from_min_max(pos2(col.left(), nr.bottom() + 4.0), col.max);
        let resp = ui.interact(nr, id.with(("note", i)), Sense::click_and_drag()).on_hover_text("This step's note, in scale steps (Move Notes)");
        if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            touch::set_step(data, i, (((nr.center().y - pt.y) / (nr.height() * 0.5)) * 7.0).round() as i32, b);
            changed = true;
        }
        let resp = ui.interact(vr, id.with(("val", i)), Sense::click_and_drag()).on_hover_text("This step's value, for the Steps output");
        if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            touch::set_step(data, i, a, (vr.bottom() - pt.y) / vr.height());
            changed = true;
        }
        let (a, b) = touch::step(data, i);
        if i == step && active {
            p.rect_filled(col.expand(2.0), 3.0, shade(accent_now(), 0.2));
        }
        p.rect_filled(nr, 3.0, SLOT);
        let y = nr.center().y - a as f32 / 7.0 * nr.height() * 0.5;
        p.rect_filled(Rect::from_two_pos(pos2(nr.left() + 3.0, nr.center().y), pos2(nr.right() - 3.0, y)), 1.0, shade(c, 0.7));
        p.rect_filled(vr, 3.0, SLOT);
        p.rect_filled(Rect::from_min_max(pos2(vr.left() + 3.0, vr.bottom() - b * vr.height()), pos2(vr.right() - 3.0, vr.bottom())), 1.0, shade(c, 0.55));
    }
    // What it gives out now.
    let meters = Rect::from_min_max(pos2(steps.right() + 10.0, inner.top()), pos2(inner.right(), inner.bottom() - foot));
    let row = meters.height() / OUTPUTS.len() as f32;
    for (k, name) in OUTPUTS.iter().enumerate() {
        let y = meters.top() + row * (k as f32 + 0.5);
        p.text(pos2(meters.left(), y), Align2::LEFT_CENTER, *name, FontId::proportional(9.5), TEXT_DIM);
        let m = Rect::from_min_size(pos2(meters.left() + 46.0, y - 2.0), vec2(meters.width() - 46.0, 4.0));
        p.rect_filled(m, 2.0, SLOT);
        p.rect_filled(Rect::from_min_size(m.min, vec2(m.width() * outs[k], m.height())), 2.0, OUT_COLORS[k]);
    }
    p.text(pos2(inner.left(), inner.bottom()), Align2::LEFT_BOTTOM, "Played in the Touch view (top bar): plates, pulser, cables", FontId::proportional(10.0), TEXT_DIM);
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_patch_points_go_to_the_playable_knobs_first() {
        let picks = touch_picks(krabton_engine::dsp::synth::SPECS);
        assert_eq!(picks.len(), N_PATCH);
        let names: Vec<&str> = picks.iter().map(|&i| krabton_engine::dsp::synth::SPECS[i].name).collect();
        assert!(names[0].to_lowercase().contains("cutoff") || names[0].to_lowercase().contains("filter"), "{names:?}");
        assert!(picks.iter().all(|&i| continuous(&krabton_engine::dsp::synth::SPECS[i])));
        let mut seen = std::collections::HashSet::new();
        assert!(picks.iter().all(|i| seen.insert(*i)), "each knob once");
        assert!(touch_picks(&[]).is_empty());
    }
}
