//! Clip envelopes, as in Live: a parameter (of the mixer, the instrument,
//! a MIDI effect or an audio effect on the clip's track) that follows a
//! line while the clip plays. Pick what it moves, then click to add points,
//! drag them, double-click one to remove it, or draw with Draw. With
//! Automation Arm on, turning a knob while the clip plays records it here.

use crate::app::{App, ClipSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui, UiBuilder};
use krabton_engine::modulation::{instrument_params, instrument_specs, ModOwner, ModTarget};
use krabton_engine::params::ParamSpec;
use krabton_engine::project::{db_to_fader_pos, fader_pos_to_db, Envelope};

/// How far (of a parameter's range) a thinned recording may stray.
const SIMPLIFY: f32 = 0.004;

/// Something a clip on this track can move: where it lives, its name, and
/// how its values read.
#[derive(Clone)]
pub(crate) struct EnvTarget {
    pub group: String,
    pub name: String,
    pub target: ModTarget,
    pub spec: Option<&'static ParamSpec>,
}

impl EnvTarget {
    /// The value at `norm` (0..1), as text.
    pub fn format(&self, norm: f32) -> String {
        match (self.target, self.spec) {
            (ModTarget::Volume(_), _) => {
                let db = fader_pos_to_db(norm);
                if db <= krabton_engine::project::FADER_MIN_DB + 0.1 {
                    "-inf dB".into()
                } else {
                    format!("{:.1} dB", db + 0.0)
                }
            }
            (ModTarget::Pan(_), _) => crate::ui::session::pan_text(norm * 2.0 - 1.0),
            (ModTarget::Send { .. }, _) => format!("{:.0} %", norm * 100.0),
            (_, Some(spec)) => spec.format(spec.from_norm(norm)),
            _ => format!("{norm:.2}"),
        }
    }
}

impl App {
    /// Everything the clip on track `t` can move.
    pub(crate) fn env_targets(&self, t: usize) -> Vec<EnvTarget> {
        let Some(tr) = self.project.tracks.get(t) else { return Vec::new() };
        let mut out = vec![
            EnvTarget { group: "Mixer".into(), name: "Volume".into(), target: ModTarget::Volume(tr.id), spec: None },
            EnvTarget { group: "Mixer".into(), name: "Pan".into(), target: ModTarget::Pan(tr.id), spec: None },
        ];
        for (j, r) in self.project.returns.iter().enumerate().take(8) {
            out.push(EnvTarget { group: "Mixer".into(), name: format!("Send {}", r.name), target: ModTarget::Send { track: tr.id, send: j as u16 }, spec: None });
        }
        if let Some(specs) = tr.instrument.as_ref().and_then(instrument_specs) {
            let name = tr.instrument.as_ref().map(|i| i.kind_name()).unwrap_or("Instrument");
            for (k, s) in specs.iter().enumerate() {
                out.push(EnvTarget { group: name.into(), name: s.name.into(), target: ModTarget::Param { owner: ModOwner::Instrument(tr.id), param: k as u16 }, spec: Some(s) });
            }
        }
        for d in &tr.midi_devices {
            for (k, s) in d.kind.specs().iter().enumerate() {
                out.push(EnvTarget { group: d.kind.name().into(), name: s.name.into(), target: ModTarget::Param { owner: ModOwner::MidiDevice(d.id), param: k as u16 }, spec: Some(s) });
            }
        }
        for d in &tr.devices {
            for (k, s) in d.kind.specs().iter().enumerate() {
                out.push(EnvTarget { group: d.name().into(), name: s.name.into(), target: ModTarget::Param { owner: ModOwner::Device(d.id), param: k as u16 }, spec: Some(s) });
            }
        }
        out
    }

    /// The value a target has now in the project (0..1), without envelopes.
    pub(crate) fn env_base(&self, t: usize, target: ModTarget) -> Option<f32> {
        let tr = self.project.tracks.get(t)?;
        match target {
            ModTarget::Volume(_) => Some(db_to_fader_pos(tr.mixer.volume_db)),
            ModTarget::Pan(_) => Some((tr.mixer.pan + 1.0) * 0.5),
            ModTarget::Send { send, .. } => tr.mixer.sends.get(send as usize).copied(),
            ModTarget::Param { owner, param } => {
                let k = param as usize;
                let (specs, values): (&[ParamSpec], &[f32]) = match owner {
                    ModOwner::Instrument(_) => (tr.instrument.as_ref().and_then(instrument_specs)?, tr.instrument.as_ref().and_then(instrument_params)?),
                    ModOwner::MidiDevice(id) => {
                        let d = tr.midi_devices.iter().find(|d| d.id == id)?;
                        (d.kind.specs(), &d.params)
                    }
                    ModOwner::Device(id) => {
                        let d = tr.devices.iter().find(|d| d.id == id)?;
                        (d.kind.specs(), &d.params)
                    }
                    ModOwner::Modulator(_) => return None,
                };
                Some(specs.get(k)?.to_norm(*values.get(k)?))
            }
        }
    }

    /// The clip's envelope editor in `rect`.
    pub(crate) fn ui_envelopes(&mut self, ui: &mut Ui, rect: Rect, sel: ClipSel) {
        let t = sel.track();
        let targets = self.env_targets(t);
        let Some(clip) = self.clip_ref(sel).cloned() else { return };
        let has = |tg: ModTarget| clip.envelopes.iter().any(|e| e.target == tg && !e.points.is_empty());
        // What to show: the one picked, else the first with an envelope, else Volume.
        let current = self.env_target.filter(|tg| targets.iter().any(|x| x.target == *tg)).or_else(|| clip.envelopes.first().map(|e| e.target)).unwrap_or(targets[0].target);
        self.env_target = Some(current);
        let info = targets.iter().find(|x| x.target == current).cloned().unwrap_or_else(|| targets[0].clone());
        let bar = Rect::from_min_size(rect.min, vec2(rect.width(), 26.0));
        let canvas = Rect::from_min_max(pos2(rect.left() + 46.0, bar.bottom() + 6.0), pos2(rect.right() - 10.0, rect.bottom() - 18.0));
        rect_fill(ui.painter(), rect, 0.0, KNOB);

        // The picker: device, then parameter; red with a ~ where there is a line.
        let mut bui = ui.new_child(UiBuilder::new().max_rect(bar.shrink2(vec2(6.0, 2.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
        bui.set_clip_rect(bar);
        let mut groups: Vec<String> = Vec::new();
        for x in &targets {
            if !groups.contains(&x.group) {
                groups.push(x.group.clone());
            }
        }
        let mut pick = None;
        let dot = |on: bool, s: &str| if on { RichText::new(format!("~ {s}")).color(REC) } else { RichText::new(s) };
        egui::ComboBox::from_id_salt(("env_group", t)).width(150.0).selected_text(dot(targets.iter().any(|x| x.group == info.group && has(x.target)), &info.group)).show_ui(&mut bui, |ui| {
            for g in &groups {
                let any = targets.iter().any(|x| &x.group == g && has(x.target));
                if ui.selectable_label(*g == info.group, dot(any, g)).clicked() {
                    // The group's first parameter with a line, else its first.
                    pick = targets.iter().find(|x| &x.group == g && has(x.target)).or_else(|| targets.iter().find(|x| &x.group == g)).map(|x| x.target);
                }
            }
        });
        egui::ComboBox::from_id_salt(("env_param", t)).width(170.0).selected_text(dot(has(current), &info.name)).show_ui(&mut bui, |ui| {
            for x in targets.iter().filter(|x| x.group == info.group) {
                if ui.selectable_label(x.target == current, dot(has(x.target), &x.name)).clicked() {
                    pick = Some(x.target);
                }
            }
        });
        bui.separator();
        let draw_on = self.env_draw;
        if bui.add(egui::Button::new(RichText::new("Draw").color(if draw_on { TEXT_DARK } else { TEXT })).fill(if draw_on { accent_now() } else { PANEL3 })).on_hover_text("Draw the line by dragging (on the grid)").clicked() {
            self.env_draw = !draw_on;
        }
        let mut clear = false;
        if bui.add_enabled(has(current), egui::Button::new("Clear")).on_hover_text("Take this parameter's line out of the clip").clicked() {
            clear = true;
        }
        bui.label(RichText::new("Click: add a point · drag: move · double-click: remove").small().color(TEXT_DIM));
        if let Some(p) = pick {
            self.env_target = Some(p);
        }

        // The canvas.
        let painter = ui.painter_at(rect);
        let len = clip.length.max(0.25);
        let x_of = |b: f64| canvas.left() + (b / len) as f32 * canvas.width();
        let b_of = |x: f32| (((x - canvas.left()) / canvas.width()).clamp(0.0, 1.0) as f64) * len;
        let y_of = |v: f32| canvas.bottom() - v.clamp(0.0, 1.0) * canvas.height();
        let v_of = |y: f32| ((canvas.bottom() - y) / canvas.height()).clamp(0.0, 1.0);
        rect_fill(&painter, canvas, 2.0, PANEL2);
        // Beats and bars.
        let mut b = 0.0;
        while b <= len + 1e-9 {
            let bar_line = (b % 4.0).abs() < 1e-6;
            painter.line_segment([pos2(x_of(b), canvas.top()), pos2(x_of(b), canvas.bottom())], Stroke::new(1.0_f32, if bar_line { LINE } else { LINE_SOFT }));
            if bar_line {
                text(&painter, pos2(x_of(b) + 3.0, canvas.top() + 2.0), Align2::LEFT_TOP, format!("{}", (b / 4.0) as i64 + 1), small(), TEXT_DIM);
            }
            b += 1.0;
        }
        for v in [0.25f32, 0.5, 0.75] {
            painter.line_segment([pos2(canvas.left(), y_of(v)), pos2(canvas.right(), y_of(v))], Stroke::new(1.0_f32, LINE_SOFT));
        }
        text(&painter, pos2(canvas.left() - 4.0, canvas.top()), Align2::RIGHT_TOP, info.format(1.0), small(), TEXT_DIM);
        text(&painter, pos2(canvas.left() - 4.0, canvas.bottom()), Align2::RIGHT_BOTTOM, info.format(0.0), small(), TEXT_DIM);
        let env = clip.envelopes.iter().find(|e| e.target == current).cloned();
        let base = self.env_base(t, current).unwrap_or(0.5);
        let color = rgb(self.project.tracks.get(t).map(|tr| tr.color).unwrap_or([255, 160, 50]));
        match &env {
            Some(e) if !e.points.is_empty() => {
                let mut pts: Vec<Pos2> = Vec::with_capacity(e.points.len() + 2);
                pts.push(pos2(canvas.left(), y_of(e.points[0].1)));
                pts.extend(e.points.iter().map(|p| pos2(x_of(p.0), y_of(p.1))));
                pts.push(pos2(canvas.right(), y_of(e.points[e.points.len() - 1].1)));
                painter.add(Shape::line(pts, Stroke::new(2.0_f32, color)));
                for p in &e.points {
                    painter.circle_filled(pos2(x_of(p.0), y_of(p.1)), 4.0, color);
                }
            }
            _ => {
                // No line yet: the knob's value, dashed.
                let y = y_of(base);
                let mut x = canvas.left();
                while x < canvas.right() {
                    painter.line_segment([pos2(x, y), pos2((x + 6.0).min(canvas.right()), y)], Stroke::new(1.0_f32, TEXT_DIM));
                    x += 11.0;
                }
                text(&painter, pos2(canvas.center().x, y - 4.0), Align2::CENTER_BOTTOM, format!("{} is at {} — click to start a line", info.name, info.format(base)), small(), TEXT_DIM);
            }
        }
        // Where the clip is now.
        if let Some(pos) = self.clip_play_pos(sel) {
            let x = x_of(pos);
            painter.line_segment([pos2(x, canvas.top()), pos2(x, canvas.bottom())], Stroke::new(1.5_f32, Color32::WHITE.gamma_multiply(0.7)));
            if let Some(v) = env.as_ref().and_then(|e| e.value_at(pos)) {
                painter.circle_stroke(pos2(x, y_of(v)), 5.0, Stroke::new(1.5_f32, Color32::WHITE));
            }
            ui.ctx().request_repaint();
        }

        // Editing.
        let resp = ui.interact(canvas, Id::new(("env_canvas", t)), Sense::click_and_drag());
        let grid = 0.25f64;
        let snap = |b: f64| if ui.input(|i| i.modifiers.alt) { b } else { (b / grid).round() * grid };
        let near = |pos: Pos2, e: &Envelope| e.points.iter().position(|p| pos2(x_of(p.0), y_of(p.1)).distance(pos) < 7.0);
        let drag_id = Id::new(("env_drag", t));
        let mut edited: Option<Envelope> = None;
        let mut start = env.clone().unwrap_or(Envelope { target: current, points: Vec::new() });
        if let Some(pos) = resp.interact_pointer_pos() {
            if let Some(h) = resp.hover_pos().filter(|_| !resp.dragged()) {
                if let Some(i) = near(h, &start) {
                    let p = start.points[i];
                    painter.circle_stroke(pos2(x_of(p.0), y_of(p.1)), 7.0, Stroke::new(1.0_f32, Color32::WHITE));
                    text(&painter, pos2(x_of(p.0), y_of(p.1) - 10.0), Align2::CENTER_BOTTOM, info.format(p.1), small(), TEXT);
                }
            }
            // Two quick clicks in different places are two points, not a double-click.
            let click_id = Id::new(("env_click", t));
            let last_click = ui.ctx().memory(|m| m.data.get_temp::<Pos2>(click_id));
            if resp.clicked() {
                ui.ctx().memory_mut(|m| m.data.insert_temp(click_id, pos));
            }
            let double = resp.double_clicked() && last_click.is_some_and(|q| q.distance(pos) < 8.0);
            if double {
                if let Some(i) = near(pos, &start) {
                    start.points.remove(i);
                    edited = Some(start.clone());
                }
            } else if self.env_draw && (resp.dragged() || resp.clicked()) {
                // Draw: each grid step under the pointer takes its height.
                start.set(snap(b_of(pos.x)).min(len), v_of(pos.y), grid * 0.5);
                edited = Some(start.clone());
            } else if resp.drag_started() {
                let i = match near(pos, &start) {
                    Some(i) => i,
                    None => {
                        start.set(snap(b_of(pos.x)).min(len), v_of(pos.y), 1e-6);
                        start.points.iter().position(|p| (p.0 - snap(b_of(pos.x)).min(len)).abs() < 1e-9).unwrap_or(0)
                    }
                };
                ui.ctx().memory_mut(|m| m.data.insert_temp(drag_id, i));
                edited = Some(start.clone());
            } else if resp.dragged() {
                if let Some(i) = ui.ctx().memory(|m| m.data.get_temp::<usize>(drag_id)).filter(|i| *i < start.points.len()) {
                    // A point moves between its neighbours.
                    let lo = if i > 0 { start.points[i - 1].0 } else { 0.0 };
                    let hi = start.points.get(i + 1).map(|p| p.0).unwrap_or(len);
                    start.points[i] = (snap(b_of(pos.x)).clamp(lo, hi), v_of(pos.y));
                    edited = Some(start.clone());
                }
            } else if resp.clicked() && near(pos, &start).is_none() {
                start.set(snap(b_of(pos.x)).min(len), v_of(pos.y), 1e-6);
                edited = Some(start.clone());
            }
        }
        if let Some(e) = edited {
            if let Some(c) = self.clip_mut(sel) {
                c.envelopes.retain(|x| x.target != current);
                if !e.points.is_empty() {
                    c.envelopes.push(e);
                }
            }
        }
        if clear {
            if let Some(c) = self.clip_mut(sel) {
                c.envelopes.retain(|x| x.target != current);
            }
        }
        resp.on_hover_text("Click: add a point (on the grid; Alt: anywhere) · drag: move · double-click: remove");
    }

    /// What the envelope of the clip playing on track `t` gives `target`
    /// now (0..1), so knobs can show what is heard.
    pub(crate) fn envelope_now(&self, t: usize, target: ModTarget) -> Option<f32> {
        use std::sync::atomic::Ordering::Relaxed;
        let ts = self.engine.status.tracks.get(t)?;
        let pos = ts.clip_pos.load();
        if pos < 0.0 {
            return None;
        }
        let tr = self.project.tracks.get(t)?;
        let slot = ts.playing_slot.load(Relaxed);
        let clip = if slot >= 0 {
            tr.slots.get(slot as usize)?.as_ref()?
        } else {
            let id = ts.arr_clip.load(Relaxed);
            &tr.arrangement.iter().find(|a| a.id == id)?.clip
        };
        clip.envelopes.iter().find(|e| e.target == target)?.value_at(pos)
    }

    /// Automation Arm: while the song plays, knobs turned on a track whose
    /// clip plays are written into that clip as envelope points.
    pub(crate) fn record_automation(&mut self) {
        if !self.auto_arm || !self.is_playing() {
            self.auto_prev.clear();
            self.finish_moves(true);
            return;
        }
        let frame = self.frame;
        self.finish_moves(false);
        let status = self.status();
        for t in 0..self.project.tracks.len() {
            let Some(ts) = status.tracks.get(t) else { continue };
            let slot = ts.playing_slot.load(std::sync::atomic::Ordering::Relaxed);
            let arr = ts.arr_clip.load(std::sync::atomic::Ordering::Relaxed);
            let pos = ts.clip_pos.load();
            if pos < 0.0 {
                continue;
            }
            let sel = if slot >= 0 { ClipSel::Slot { track: t, scene: slot as usize } } else if arr != 0 { ClipSel::Arr { track: t, id: arr } } else { continue };
            for x in self.env_targets(t) {
                let Some(now) = self.env_base(t, x.target) else { continue };
                let key = (t, x.target);
                let before = self.auto_prev.insert(key, now);
                let Some(before) = before else { continue };
                if (before - now).abs() < 1e-6 {
                    continue;
                }
                // Turned: write it at the clip's position.
                let going = self.auto_last.get(&key).copied().filter(|m| m.0 == sel);
                let Some(c) = self.clip_mut(sel) else { continue };
                let len = c.length.max(1e-6);
                let pos = pos.min(len);
                let e = match c.envelopes.iter_mut().find(|e| e.target == x.target) {
                    Some(e) => e,
                    None => {
                        c.envelopes.push(Envelope { target: x.target, points: vec![(0.0, before)] });
                        c.envelopes.last_mut().unwrap()
                    }
                };
                let start = match going {
                    // Still turning: what the turn passed over since the
                    // last frame is overwritten.
                    Some((_, start, last, _)) if pos >= last => {
                        e.points.retain(|p| !(p.0 > last && p.0 < pos));
                        start
                    }
                    other => {
                        // A new move (or the clip came round): the earlier
                        // part is tidied, and the line holds as it was up to here.
                        if let Some((_, start, last, _)) = other {
                            e.simplify(start, last, SIMPLIFY);
                        }
                        let hold = pos - 1.0 / 32.0;
                        if hold > 0.0 {
                            let held = e.value_at(hold).unwrap_or(before);
                            e.set(hold, held, 1e-6);
                        }
                        hold.max(0.0)
                    }
                };
                e.set(pos, now, 1.0 / 64.0);
                self.auto_last.insert(key, (sel, start, pos, frame));
            }
        }
    }

    /// Recorded moves that ended (no turn for half a second, or recording
    /// stopped) are thinned out to the points their shape needs.
    fn finish_moves(&mut self, all: bool) {
        if self.auto_last.is_empty() {
            return;
        }
        let frame = self.frame;
        let done: Vec<_> = self.auto_last.iter().filter(|(_, m)| all || frame.saturating_sub(m.3) >= 30).map(|(k, m)| (*k, *m)).collect();
        for (key, (sel, start, last, _)) in done {
            self.auto_last.remove(&key);
            if let Some(e) = self.clip_mut(sel).and_then(|c| c.envelopes.iter_mut().find(|e| e.target == key.1)) {
                e.simplify(start, last, SIMPLIFY);
            }
        }
    }
}
