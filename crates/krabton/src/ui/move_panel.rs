//! The Move tab: an Ableton Move on screen that runs the same actions as a
//! real one (its pads, steps, knobs and buttons lit as the Move's are; click
//! and drag them), the Move's ports and the messages it sent that Krabton
//! does not place. The Move's own screen can't be reached over USB, so its
//! screen is drawn here, and over everything (the HUD) while a knob is touched.

use crate::app::App;
use crate::move_ctl::{Button, Knob, MoveIn, PadMode, Screen, PALETTE};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Painter, Pos2, Rect, RichText, Sense, Stroke, Ui, UiBuilder};

const SIDEBAR_W: f32 = 250.0;
/// The Move's body and its screen.
const BODY: Color32 = Color32::from_rgb(0x1b, 0x1b, 0x1d);
const SCREEN: Color32 = Color32::from_rgb(0x06, 0x06, 0x08);

/// A pad's or step's light.
fn lit(i: u8) -> Color32 {
    if i == 0 {
        PANEL2
    } else {
        rgb(PALETTE[i as usize & 127])
    }
}

/// A white light at a brightness.
fn white(v: u8) -> Color32 {
    if v == 0 {
        PANEL2
    } else {
        mix(Color32::from_rgb(0x50, 0x50, 0x54), Color32::WHITE, v as f32 / 127.0)
    }
}

fn button_light(b: Button, v: u8) -> Color32 {
    if b.coloured() {
        lit(v)
    } else {
        white(v)
    }
}

/// Buttons the on-screen Move keeps down until clicked again (a mouse
/// holds one thing at a time).
fn latches(b: Button) -> bool {
    matches!(b, Button::Shift | Button::Mute | Button::Delete | Button::Copy | Button::RecordAudio)
}

/// The Move's screen: a title and a line, and (when there is room) its
/// eight parameters, each a name, a bar and its value, the touched one lit.
pub(crate) fn draw_screen(p: &Painter, r: Rect, s: &Screen) {
    rect_fill(p, r, 5.0, SCREEN);
    rect_line(p, r, 5.0, 1.0, LINE);
    let pad = (r.height() * 0.08).clamp(4.0, 12.0);
    let inner = r.shrink(pad);
    let big = FontId::proportional((r.height() * 0.16).clamp(10.0, 20.0));
    if inner.width() < 300.0 {
        // Small: the title, the line under it and the touched one only.
        let title = fit(p, &s.title, inner.width());
        text(p, inner.left_top(), Align2::LEFT_TOP, &s.title, title, Color32::WHITE);
        let hot = s.hot.and_then(|h| s.params.get(h));
        // Three lines where they fit; else the touched one in the second's place.
        if hot.is_none() || inner.height() >= 40.0 {
            let sub = fit(p, &s.sub, inner.width());
            text(p, pos2(inner.left(), inner.top() + 14.0), Align2::LEFT_TOP, &s.sub, sub, TEXT_DIM);
        }
        if let Some((name, value, _)) = hot {
            let line = format!("{name}  {value}");
            let f = fit(p, &line, inner.width());
            text(p, pos2(inner.left(), inner.bottom()), Align2::LEFT_BOTTOM, line, f, ACCENT);
        }
        return;
    }
    text(p, inner.left_top(), Align2::LEFT_TOP, &s.title, big.clone(), Color32::WHITE);
    text(p, inner.right_top(), Align2::RIGHT_TOP, &s.sub, small(), TEXT_DIM);
    if s.params.is_empty() {
        return;
    }
    let top = inner.top() + big.size + 8.0;
    let room = Rect::from_min_max(pos2(inner.left(), top), inner.max);
    let w = room.width() / 8.0;
    for (i, (name, value, norm)) in s.params.iter().enumerate().take(8) {
        let c = Rect::from_min_size(pos2(room.left() + i as f32 * w, room.top()), vec2(w, room.height())).shrink2(vec2(3.0, 0.0));
        let hot = s.hot == Some(i);
        let col = if hot { ACCENT } else { TEXT };
        if hot {
            rect_fill(p, c.expand(2.0), 3.0, Color32::from_rgb(0x20, 0x1a, 0x10));
        }
        let fname = fit(p, name, c.width());
        text(p, pos2(c.center().x, c.top()), Align2::CENTER_TOP, name, fname, col);
        let bar = Rect::from_min_size(pos2(c.left(), c.center().y - 3.0), vec2(c.width(), 6.0));
        rect_fill(p, bar, 2.0, Color32::from_rgb(0x26, 0x26, 0x2a));
        rect_fill(p, Rect::from_min_size(bar.min, vec2(bar.width() * norm.clamp(0.0, 1.0), bar.height())), 2.0, if hot { ACCENT } else { Color32::from_rgb(0xc8, 0xc8, 0xcc) });
        let fval = fit(p, value, c.width());
        text(p, pos2(c.center().x, c.bottom()), Align2::CENTER_BOTTOM, value, fval, if hot { Color32::WHITE } else { TEXT_DIM });
    }
}

/// A small font that fits a width.
fn fit(p: &Painter, s: &str, width: f32) -> FontId {
    let mut size = 11.0;
    while size > 7.0 && p.layout_no_wrap(s.to_string(), FontId::proportional(size), Color32::WHITE).size().x > width {
        size -= 0.5;
    }
    FontId::proportional(size)
}

/// Where everything of the on-screen Move is, for a cell (a pad's size).
struct Layout {
    c: f32,
    screen: Rect,
    jog: (Pos2, f32),
    knobs: [(Pos2, f32); 8],
    volume: (Pos2, f32),
    pads: [Rect; 32],
    steps: [Rect; 16],
    buttons: Vec<(Button, Rect)>,
}

/// Where everything is, measured from a photo of the Move: screen, jog wheel,
/// Back and Menu, Play and Record down the left; the four track buttons; the
/// 32 pads (wider than tall) under the eight knobs; the steps under them; the
/// two columns of buttons and the arrows with + and - on the right.
fn layout(area: Rect) -> (Rect, Layout) {
    let c = (area.width() / 12.1).min(area.height() / 5.85).clamp(14.0, 96.0);
    let size = vec2(11.86 * c, 5.58 * c);
    let o = area.center() - size * 0.5;
    let dev = Rect::from_min_size(o, size);
    let at = |x: f32, y: f32| o + vec2(x * c, y * c);
    let round = |x: f32, y: f32, r: f32| Rect::from_center_size(at(x, y), vec2(2.0 * r * c, 2.0 * r * c));
    let screen = Rect::from_min_size(at(0.25, 0.33), vec2(1.22 * c, 0.68 * c));
    let jog = (at(0.84, 2.04), 0.49 * c);
    let knobs = std::array::from_fn(|i| (at(2.62 + i as f32 * 1.012, 0.54), 0.25 * c));
    let volume = (at(11.37, 0.59), 0.28 * c);
    // Pad 0 is the bottom left.
    let pads = std::array::from_fn(|i| {
        let (col, row) = ((i % 8) as f32, (i / 8) as f32);
        Rect::from_min_size(at(2.2 + col * 1.012, 1.33 + (3.0 - row) * 0.77), vec2(0.91 * c, 0.66 * c))
    });
    let steps = std::array::from_fn(|i| round(2.41 + i as f32 * 0.503, 4.85, 0.17));
    let mut buttons = vec![
        (Button::Back, round(0.52, 3.2, 0.27)),
        (Button::Menu, round(1.2, 3.2, 0.27)),
        (Button::Play, round(0.52, 4.85, 0.27)),
        (Button::Record, round(1.21, 4.85, 0.27)),
        (Button::Capture, round(10.63, 1.67, 0.27)),
        (Button::RecordAudio, round(11.3, 1.67, 0.27)),
        (Button::Loop, round(10.63, 2.42, 0.27)),
        (Button::Mute, round(11.3, 2.42, 0.27)),
        (Button::Delete, round(10.63, 3.17, 0.27)),
        (Button::Copy, round(11.3, 3.17, 0.27)),
        (Button::Undo, round(10.63, 3.94, 0.27)),
        (Button::Shift, round(11.3, 3.94, 0.27)),
        (Button::Left, round(10.54, 4.86, 0.16)),
        (Button::Right, round(11.4, 4.86, 0.16)),
        (Button::Up, round(10.97, 4.63, 0.16)),
        (Button::Down, round(10.97, 5.1, 0.16)),
    ];
    // The track buttons, top to bottom, beside the pad rows.
    for k in 0..4u8 {
        let y = 1.66 + k as f32 * 0.77;
        buttons.push((Button::Row(k), Rect::from_center_size(at(1.8, y), vec2(0.28 * c, 0.68 * c))));
    }
    (dev, Layout { c, screen, jog, knobs, volume, pads, steps, buttons })
}

/// A button's mark, as printed on the Move.
fn icon(p: &egui::Painter, b: Button, r: Rect, col: Color32) {
    let (m, u) = (r.center(), r.width() * 0.2);
    let line = egui::Stroke::new((u * 0.28).max(1.0), col);
    let seg = |a: (f32, f32), z: (f32, f32)| p.line_segment([m + vec2(a.0 * u, a.1 * u), m + vec2(z.0 * u, z.1 * u)], line);
    match b {
        Button::Back | Button::Left => {
            seg((0.45, -0.8), (-0.45, 0.0));
            seg((-0.45, 0.0), (0.45, 0.8));
        }
        Button::Right => {
            seg((-0.45, -0.8), (0.45, 0.0));
            seg((0.45, 0.0), (-0.45, 0.8));
        }
        Button::Menu => {
            for y in [-0.6, 0.0, 0.6] {
                seg((-0.9, y), (0.9, y));
            }
        }
        Button::Play => {
            seg((-0.6, -0.95), (0.9, 0.0));
            seg((0.9, 0.0), (-0.6, 0.95));
            seg((-0.6, 0.95), (-0.6, -0.95));
        }
        Button::Record => {
            p.circle_stroke(m, u * 0.95, line);
        }
        Button::RecordAudio => {
            p.circle_filled(m, u * 0.7, col);
        }
        Button::Capture => {
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                seg((x, y), (x * 0.4, y));
                seg((x, y), (x, y * 0.4));
            }
        }
        Button::Loop => {
            seg((-0.9, -0.5), (0.9, -0.5));
            seg((0.9, -0.5), (0.9, 0.5));
            seg((0.9, 0.5), (-0.5, 0.5));
            seg((-0.5, 0.5), (-0.1, 0.95));
            seg((-0.5, 0.5), (-0.1, 0.05));
        }
        Button::Mute => {
            seg((-0.8, 0.8), (-0.8, -0.8));
            seg((-0.8, -0.8), (0.0, 0.3));
            seg((0.0, 0.3), (0.8, -0.8));
            seg((0.8, -0.8), (0.8, 0.8));
        }
        Button::Delete => {
            seg((-0.75, -0.75), (0.75, 0.75));
            seg((0.75, -0.75), (-0.75, 0.75));
        }
        Button::Copy => {
            p.rect_stroke(Rect::from_min_size(m + vec2(-0.9 * u, -0.25 * u), vec2(1.15 * u, 1.15 * u)), 0.0, line, egui::StrokeKind::Middle);
            seg((-0.25, -0.9), (0.9, -0.9));
            seg((0.9, -0.9), (0.9, 0.25));
        }
        Button::Undo => {
            seg((-0.9, -0.3), (0.9, -0.3));
            seg((0.9, -0.3), (0.9, 0.8));
            seg((-0.9, -0.3), (-0.35, -0.85));
            seg((-0.9, -0.3), (-0.35, 0.25));
        }
        Button::Shift => {
            for x in [-0.8, 0.0, 0.8] {
                p.circle_filled(m + vec2(x * u, 0.0), u * 0.2, col);
            }
        }
        Button::Up => {
            seg((-0.7, 0.0), (0.7, 0.0));
            seg((0.0, -0.7), (0.0, 0.7));
        }
        Button::Down => {
            seg((-0.7, 0.0), (0.7, 0.0));
        }
        Button::Row(_) | Button::Jog => {}
    }
}

/// What a press on the on-screen Move landed on.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Hit {
    Pad(u8, f32),
    Step(u8),
    Button(Button),
    Knob(Knob),
}

impl Layout {
    fn hit(&self, pt: Pos2) -> Option<Hit> {
        if let Some(i) = self.pads.iter().position(|r| r.contains(pt)) {
            let r = self.pads[i];
            // Higher up a pad: harder.
            let vel = (0.3 + 0.7 * (r.bottom() - pt.y) / r.height()).clamp(0.05, 1.0);
            return Some(Hit::Pad(i as u8, vel));
        }
        if let Some(i) = self.steps.iter().position(|r| r.contains(pt)) {
            return Some(Hit::Step(i as u8));
        }
        if let Some((b, _)) = self.buttons.iter().find(|(_, r)| r.contains(pt)) {
            return Some(Hit::Button(*b));
        }
        if let Some(k) = self.knobs.iter().position(|(c, r)| c.distance(pt) <= *r) {
            return Some(Hit::Knob(Knob::Enc(k as u8)));
        }
        if self.volume.0.distance(pt) <= self.volume.1 {
            return Some(Hit::Knob(Knob::Volume));
        }
        if self.jog.0.distance(pt) <= self.jog.1 {
            return Some(Hit::Knob(Knob::Jog));
        }
        None
    }
}

impl App {
    pub(crate) fn ui_move_panel(&mut self, ui: &mut Ui, body: Rect) {
        let sidebar = Rect::from_min_size(body.min, vec2(SIDEBAR_W, body.height()));
        let main = Rect::from_min_max(pos2(sidebar.right() + 1.0, body.top()), body.max);
        rect_fill(ui.painter(), sidebar, 0.0, PANEL);
        ui.painter().line_segment([pos2(sidebar.right(), body.top()), pos2(sidebar.right(), body.bottom())], Stroke::new(1.0_f32, LINE));
        let mut sui = ui.new_child(UiBuilder::new().max_rect(sidebar.shrink(6.0)).layout(egui::Layout::top_down(egui::Align::Min)));
        sui.set_clip_rect(sidebar);
        egui::ScrollArea::vertical().id_salt("move_sidebar").auto_shrink([false, false]).show(&mut sui, |ui| self.move_sidebar(ui));
        self.virtual_move(ui, main);
    }

    fn move_sidebar(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 4.0;
        let mode = self.move_mode();
        let testing = self.move_ctl.test_page.is_some();
        ui.horizontal(|ui| {
            for (m, label) in [(PadMode::Notes, if mode == PadMode::Drums { "Drums" } else { "Notes" }), (PadMode::Session, "Session"), (PadMode::Touch, "Touch")] {
                let on = !testing && (mode == m || (m == PadMode::Notes && mode == PadMode::Drums));
                if ui.selectable_label(on, label).clicked() {
                    self.move_set_mode(m);
                }
            }
            if ui.selectable_label(testing, "Colours").on_hover_text("The colour test (Shift + Menu): the pads lit with the palette's indices").clicked() {
                self.move_ctl.test_page = if testing { None } else { Some(0) };
            }
        });
        let help = if testing {
            "The pads show palette indices (Up/Down or steps 1-4: the next 32). If a pad shows another colour than here, tell us: the table in move_ctl.rs is a guess."
        } else {
            match mode {
                PadMode::Notes => "An in-key keyboard: rows a fourth apart, roots in the track's colour. Up/Down: octave. Hold pads and tap steps to put those notes there; tap a lit step to take it away.",
                PadMode::Drums => "The kit on the left 4x4 (tap to choose and play a drum), sixteen velocities of it on the right. The steps are the chosen drum's.",
                PadMode::Session => "8 tracks by 4 scenes: press to launch, Shift to pick only, Delete to remove, Copy then another pad to copy. Row buttons and steps launch scenes; Up/Down move the scenes.",
                PadMode::Touch => "The Touch view's plates, pressed as hard as the pad. Up/Down: an octave.",
            }
        };
        ui.label(RichText::new(help).small().color(TEXT_DIM));

        ui.separator();
        ui.label(RichText::new("Ableton Move").strong());
        let status = match (&self.move_ctl.connected, self.move_ctl.hardware()) {
            (Some((input, output)), _) => {
                ui.label(RichText::new(format!("In: {input}")).small().color(PLAY));
                let out = output.clone().unwrap_or_else(|| "none (no lights)".into());
                ui.label(RichText::new(format!("Lights: {out}")).small().color(if output.is_some() { PLAY } else { REC }));
                None
            }
            (None, true) if self.prefs.move_off => Some("Not used (switched off below)."),
            (None, true) => Some("No Move found. Set it to Control Live and plug it in by USB; until then the one on the right works with the mouse."),
            (None, false) => Some("A test window: only the one on the right (KRABTON_MOVE=1 looks for a real one)."),
        };
        if let Some(s) = status {
            ui.label(RichText::new(s).small().color(TEXT_DIM));
        }
        let mut on = !self.prefs.move_off;
        if ui.checkbox(&mut on, "Use a Move as a controller").changed() {
            self.prefs.move_off = !on;
            self.prefs.save();
            self.move_ctl.set_enabled(on);
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new("Lights").small().color(TEXT_DIM));
            let now = crate::move_ctl::BRIGHTNESS[(self.prefs.move_brightness as usize).min(3)].0;
            egui::ComboBox::from_id_salt("move_brightness").width(90.0).selected_text(now).show_ui(ui, |ui| {
                for (k, (name, _)) in crate::move_ctl::BRIGHTNESS.iter().enumerate() {
                    if ui.selectable_label(self.prefs.move_brightness as usize == k, *name).clicked() {
                        self.prefs.move_brightness = k as u8;
                        self.prefs.save();
                        self.move_ctl.send_brightness(k as u8);
                    }
                }
            });
            ui.label(RichText::new("Pads").small().color(TEXT_DIM));
            let now = crate::move_ctl::PAD_CURVES[(self.prefs.move_curve as usize).min(3)].0;
            egui::ComboBox::from_id_salt("move_curve").width(90.0).selected_text(now).show_ui(ui, |ui| {
                for (k, (name, _)) in crate::move_ctl::PAD_CURVES.iter().enumerate() {
                    if ui.selectable_label(self.prefs.move_curve as usize == k, *name).on_hover_text("How hard a pad must be hit. Light: a soft hit already plays. Firm: only a hard one plays loud").clicked() {
                        self.prefs.move_curve = k as u8;
                        self.prefs.save();
                    }
                }
            });
        });
        if self.move_ctl.last_raw > 0.0 {
            let raw = self.move_ctl.last_raw;
            let played = crate::move_ctl::pad_curve(self.prefs.move_curve, raw);
            ui.label(RichText::new(format!("Last hit: {} from the pad, plays at {}", (raw * 127.0).round(), (played * 127.0).round())).small().color(TEXT_DIM));
        }
        let mut pick = (self.prefs.move_in.clone(), self.prefs.move_out.clone());
        let ins = self.move_ctl.inputs.clone();
        let outs = self.move_ctl.outputs.clone();
        let combo = |ui: &mut Ui, id: &str, label: &str, cur: &mut Option<String>, names: &[String]| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).small().color(TEXT_DIM));
                egui::ComboBox::from_id_salt(id).width(170.0).selected_text(cur.clone().unwrap_or_else(|| "Found by name".into())).show_ui(ui, |ui| {
                    ui.selectable_value(cur, None, "Found by name");
                    for n in names {
                        ui.selectable_value(cur, Some(n.clone()), n);
                    }
                });
            });
        };
        combo(ui, "move_in_port", "In", &mut pick.0, &ins);
        combo(ui, "move_out_port", "Out", &mut pick.1, &outs);
        if pick != (self.prefs.move_in.clone(), self.prefs.move_out.clone()) {
            self.prefs.move_in = pick.0.clone();
            self.prefs.move_out = pick.1.clone();
            self.prefs.save();
            self.move_ctl.set_ports(pick.0, pick.1);
        }
        if ui.button("Rescan").on_hover_text("Look for the Move again").clicked() {
            self.move_ctl.rescan();
            self.toast("Looking for an Ableton Move...");
        }

        if !self.move_ctl.log.is_empty() {
            ui.separator();
            ui.label(RichText::new("Sent but not placed").strong());
            ui.label(RichText::new("Messages from the Move Krabton does not know (the jog wheel pressed?).").small().color(TEXT_DIM));
            for m in self.move_ctl.log.iter().rev() {
                ui.label(RichText::new(m).small().monospace());
            }
        }

        ui.separator();
        egui::CollapsingHeader::new(RichText::new("What does what").strong()).id_salt("move_map").show(ui, |ui| {
            for line in [
                "Menu: next mode · Shift+Menu: colour test",
                "Shift+row 1-4: Notes/Drums, Session, Touch, colours",
                "Row buttons: tracks of the bank of four (Session: scenes)",
                "Mute+row: mute it · Sample+row: arm it",
                "Hold a row + knobs: its mixer · + Volume: its volume",
                "Knobs: the device (Left/Right: device, mixer last; Shift+Left/Right: next eight) · Shift: fine",
                "Volume: master · Shift: the selected track",
                "Jog: track · Shift+jog: scene",
                "Steps: the clip's sixteenths · Shift+step: bars long · Shift+Up/Down: page",
                "Play · Shift: to the start · Rec · Shift: Automation Arm",
                "Loop · Shift: metronome · Capture: what you just played",
                "Undo · Shift: redo · Copy, Delete: hold + pad or step",
                "Mute: selected track · Shift+Mute: solo · Sample: arm",
            ] {
                ui.label(RichText::new(line).small());
            }
        });
    }

    fn virtual_move(&mut self, ui: &mut Ui, area: Rect) {
        let (dev, lay) = layout(area.shrink(10.0));
        let c = lay.c;
        let p = ui.painter().with_clip_rect(area);
        rect_fill(&p, dev, c * 0.25, BODY);
        rect_line(&p, dev, c * 0.25, 1.0, LINE_SOFT);
        let leds = self.move_ctl.leds.clone();
        let small_ok = c >= 26.0;

        // The screen, the jog wheel and the knobs.
        let screen = self.move_screen();
        draw_screen(&p, lay.screen, &screen);
        let knob = |center: Pos2, r: f32, ring: Color32, touched: bool| {
            p.circle_filled(center, r, Color32::from_rgb(0x2a, 0x2a, 0x2e));
            p.circle_stroke(center, r, Stroke::new(if touched { 2.0_f32 } else { 1.0 }, if touched { Color32::WHITE } else { ring }));
            p.circle_filled(center, r * 0.62, Color32::from_rgb(0x3a, 0x3a, 0x3f));
        };
        knob(lay.jog.0, lay.jog.1, LINE, self.move_ctl.touched[9]);
        for (k, (center, r)) in lay.knobs.iter().enumerate() {
            let v = leds.knob(k as u8);
            let ring = if v == 0 { LINE } else { lit(v) };
            knob(*center, *r, ring, self.move_ctl.touched[k]);
            if let Some((name, _, norm)) = screen.params.get(k) {
                // Where it is, as a dot round the knob.
                let a = std::f32::consts::PI * (0.75 + 1.5 * norm.clamp(0.0, 1.0));
                p.circle_filled(*center + vec2(a.cos(), a.sin()) * *r * 0.8, (*r * 0.12).max(1.5), Color32::WHITE);
                if small_ok {
                    let f = fit(&p, name, c * 0.95);
                    text(&p, pos2(center.x, center.y - r - 2.0), Align2::CENTER_BOTTOM, name, f, TEXT_DIM);
                }
            }
        }
        knob(lay.volume.0, lay.volume.1, LINE, self.move_ctl.touched[8]);
        if small_ok {
            text(&p, pos2(lay.volume.0.x, lay.volume.0.y - lay.volume.1 - 2.0), Align2::CENTER_BOTTOM, "Volume", small(), TEXT_DIM);
        }

        // The pads, the steps and the buttons.
        let labels = self.move_pad_labels();
        for (i, r) in lay.pads.iter().enumerate() {
            let col = lit(leds.pad(i as u8));
            rect_fill(&p, *r, c * 0.08, col);
            if small_ok {
                if let Some(l) = labels.get(i).filter(|l| !l.is_empty()) {
                    let f = fit(&p, l, r.width() - 4.0);
                    text(&p, pos2(r.left() + 3.0, r.bottom() - 2.0), Align2::LEFT_BOTTOM, l, f, if leds.pad(i as u8) == 0 { TEXT_DIM } else { text_on(col) });
                }
            }
        }
        for (i, r) in lay.steps.iter().enumerate() {
            let lamp = leds.step(i as u8);
            // Never darker than the button itself.
            p.circle_filled(r.center(), r.width() * 0.5, Color32::from_rgb(0x20, 0x20, 0x23));
            if lamp != 0 {
                p.circle_filled(r.center(), r.width() * 0.5, mix(Color32::from_rgb(0x20, 0x20, 0x23), lit(lamp), 0.85));
            }
            let led = leds.step_led(i as u8);
            if led > 0 {
                let d = Rect::from_center_size(pos2(r.center().x, r.bottom() + c * 0.06), vec2(r.width() * 0.5, 2.0));
                rect_fill(&p, d, 1.0, white(led));
            }
        }
        for (b, r) in &lay.buttons {
            let v = leds.button(*b);
            let col = button_light(*b, v);
            let down = self.move_ctl.holds(*b);
            let idle = Color32::from_rgb(0x26, 0x26, 0x29);
            let shade = if v == 0 { idle } else if b.coloured() { col } else { mix(idle, col, (v as f32 / 127.0).clamp(0.25, 1.0)) };
            if matches!(b, Button::Row(_)) {
                rect_fill(&p, *r, r.width() * 0.45, shade);
                if down {
                    rect_line(&p, *r, r.width() * 0.45, 1.5, Color32::WHITE);
                }
            } else {
                p.circle_filled(r.center(), r.width() * 0.5, shade);
                p.circle_stroke(r.center(), r.width() * 0.5, egui::Stroke::new(1.0_f32, if down { Color32::WHITE } else { Color32::from_rgb(0x3a, 0x3a, 0x3f) }));
                let mark = if b.coloured() && v > 0 { text_on(col) } else { Color32::from_rgb(0xd8, 0xd8, 0xdc) };
                icon(&p, *b, *r, mark);
            }
        }
        // What a button is, on the mark's own terms.
        if let Some(pt) = ui.ctx().pointer_hover_pos() {
            if let Some((b, _)) = lay.buttons.iter().find(|(_, r)| r.contains(pt)) {
                egui::Tooltip::always_open(ui.ctx().clone(), ui.layer_id(), Id::new("move_button_tip"), egui::PopupAnchor::Pointer).show(|ui| ui.label(b.name()));
            }
        }
        let mode = if self.move_ctl.test_page.is_some() { "Colour test".to_string() } else { format!("{} mode", self.move_mode().name()) };
        text(&p, pos2(lay.screen.left(), dev.top() + 1.0), Align2::LEFT_TOP, &mode, fit(&p, &mode, lay.screen.width()), ACCENT);

        // The mouse plays it.
        let resp = ui.interact(dev, Id::new("virtual_move"), Sense::click_and_drag());
        let hover = resp.hover_pos().and_then(|pt| lay.hit(pt));
        if let Some(Hit::Knob(k)) = hover {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll.abs() > 0.5 {
                let ticks = (scroll / 8.0).round() as i32;
                let ticks = if ticks == 0 { scroll.signum() as i32 } else { ticks };
                self.move_input(MoveIn::Turn { knob: k, ticks });
            }
        }
        let drag_id = Id::new("virtual_move_drag");
        if resp.is_pointer_button_down_on() && self.move_ctl.virtual_down.is_none() {
            if let Some(hit) = resp.interact_pointer_pos().and_then(|pt| lay.hit(pt)) {
                let down = match hit {
                    Hit::Pad(pad, vel) => Some(MoveIn::Pad { pad, down: true, vel }),
                    Hit::Step(step) => Some(MoveIn::Step { step, down: true }),
                    Hit::Knob(knob) => Some(MoveIn::Touch { knob, down: true }),
                    Hit::Button(b) if latches(b) => {
                        // A latching button: one click down, the next up.
                        let held = self.move_ctl.latched.contains(&b);
                        if held {
                            self.move_ctl.latched.retain(|x| *x != b);
                        } else {
                            self.move_ctl.latched.push(b);
                        }
                        self.move_input(MoveIn::Button { button: b, down: !held });
                        None
                    }
                    Hit::Button(b) => Some(MoveIn::Button { button: b, down: true }),
                };
                // A latching button sends nothing more until let go.
                self.move_ctl.virtual_down = Some(down.unwrap_or(MoveIn::Other([0, 0, 0])));
                if let Some(d) = down {
                    self.move_input(d);
                }
                ui.ctx().data_mut(|d| d.insert_temp(drag_id, 0.0f32));
            }
        }
        if let Some(MoveIn::Touch { knob, .. }) = self.move_ctl.virtual_down {
            // Dragging up turns it clockwise.
            let dy = resp.drag_delta().y;
            let mut acc: f32 = ui.ctx().data(|d| d.get_temp(drag_id)).unwrap_or(0.0);
            acc -= dy / 3.0;
            let ticks = acc.trunc() as i32;
            if ticks != 0 {
                acc -= ticks as f32;
                self.move_input(MoveIn::Turn { knob, ticks });
            }
            ui.ctx().data_mut(|d| d.insert_temp(drag_id, acc));
        }
        if !resp.is_pointer_button_down_on() && !resp.dragged() {
            if let Some(down) = self.move_ctl.virtual_down.take() {
                let up = match down {
                    MoveIn::Pad { pad, .. } => Some(MoveIn::Pad { pad, down: false, vel: 0.0 }),
                    MoveIn::Step { step, .. } => Some(MoveIn::Step { step, down: false }),
                    MoveIn::Touch { knob, .. } => Some(MoveIn::Touch { knob, down: false }),
                    MoveIn::Button { button, .. } => Some(MoveIn::Button { button, down: false }),
                    _ => None,
                };
                if let Some(u) = up {
                    self.move_input(u);
                }
            }
        }
    }

    /// The Move's screen over everything while a knob is touched (or for a
    /// moment after one turned), as the Move's own would show it.
    pub(crate) fn move_hud(&mut self, ctx: &egui::Context) {
        if !self.move_ctl.hud_shown() || self.detail == crate::app::DetailTab::Move && self.detail_shown() {
            return;
        }
        let screen = self.move_screen();
        let full = ctx.screen_rect();
        let size = vec2((full.width() - 40.0).min(660.0), 132.0);
        let pos = pos2(full.center().x - size.x * 0.5, full.bottom() - size.y - 70.0);
        egui::Area::new(Id::new("move_hud")).order(egui::Order::Foreground).fixed_pos(pos).interactable(false).show(ctx, |ui| {
            let (r, _) = ui.allocate_exact_size(size, Sense::hover());
            draw_screen(ui.painter(), r, &screen);
        });
    }
}
