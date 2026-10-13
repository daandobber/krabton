//! Big Picture: Krabton for a controller, a joystick or a touch screen (a
//! Steam Deck, a ClockworkPi, a tablet). The same project and engine as the
//! desktop, with a few big pages instead of panels: Clips (launch them),
//! Play (pads), Devices (every knob as a row), Mixer and Browse (add
//! sounds), a menu for the rest. Everything moves with the D-pad (or the
//! left stick, the arrows), A chooses, B goes back, the shoulder buttons
//! turn the pages and the triggers (or the right stick) turn a value;
//! everything can be touched too.

pub mod browse;
pub mod devices;
pub mod input;
pub mod mixer;
pub mod play;

use crate::app::{App, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Painter, Pos2, Rect, Sense, Ui};
use input::{Frame, Nav};
use krabton_engine::engine::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Clips,
    Play,
    Devices,
    Mixer,
    Browse,
}

impl Page {
    pub const ALL: [Page; 5] = [Page::Clips, Page::Play, Page::Devices, Page::Mixer, Page::Browse];

    pub fn name(self) -> &'static str {
        match self {
            Page::Clips => "Clips",
            Page::Play => "Play",
            Page::Devices => "Devices",
            Page::Mixer => "Mixer",
            Page::Browse => "Browse",
        }
    }
}

pub struct BigPicture {
    pub on: bool,
    pub page: Page,
    pub pads: input::Pads,
    /// The menu, open at this row.
    pub menu: Option<usize>,
    /// Clips: the cursor (track, scene) and the first track and scene shown.
    pub clip_cursor: (usize, usize),
    pub clip_first: (usize, usize),
    pub play: play::PlayState,
    pub dev: devices::DevState,
    pub mix: mixer::MixState,
    pub browse: browse::BrowseState,
    /// Presses a script feeds in, one per frame.
    pub script: std::collections::VecDeque<Nav>,
    /// The clip actions, open at this row.
    pub clip_menu: Option<usize>,
    pub fullscreen: bool,
}

impl BigPicture {
    pub fn new(on: bool) -> Self {
        Self {
            on,
            page: Page::Clips,
            pads: input::Pads::new(),
            menu: None,
            clip_cursor: (0, 0),
            clip_first: (0, 0),
            play: play::PlayState::default(),
            dev: devices::DevState::default(),
            mix: mixer::MixState::default(),
            browse: browse::BrowseState::default(),
            script: Default::default(),
            clip_menu: None,
            fullscreen: false,
        }
    }
}

/// Whether Krabton should start in Big Picture: the setting, or
/// `--big-picture` / `KRABTON_BIG_PICTURE=1` (for a Steam launch option).
pub fn start_in_big_picture(pref: bool) -> bool {
    if std::env::var_os("KRABTON_TEST_WINDOW").is_some() {
        return std::env::var_os("KRABTON_BIG_PICTURE").is_some();
    }
    pref || std::env::args().any(|a| a == "--big-picture") || std::env::var("KRABTON_BIG_PICTURE").is_ok_and(|v| v != "0")
}

// ---------------------------------------------------------------- drawing

/// Sizes from the screen: one unit is a row of a list.
#[derive(Clone, Copy, Debug)]
pub struct Sz {
    pub u: f32,
    pub font: f32,
    pub small: f32,
}

impl Sz {
    pub fn of(r: Rect) -> Self {
        let u = (r.height() / 18.0).clamp(22.0, 52.0);
        Self { u, font: (u * 0.5).max(14.0), small: (u * 0.38).max(11.5) }
    }

    pub fn f(&self) -> FontId {
        FontId::proportional(self.font)
    }

    pub fn s(&self) -> FontId {
        FontId::proportional(self.small)
    }

    pub fn big(&self) -> FontId {
        FontId::proportional(self.font * 1.35)
    }
}

pub const FOCUS: Color32 = Color32::from_rgb(0xff, 0xd8, 0x6b);
pub const TILE: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2f);
pub const TILE_HI: Color32 = Color32::from_rgb(0x36, 0x36, 0x3c);

/// A tile, with the focus ring when it has the focus.
pub fn tile(p: &Painter, r: Rect, fill: Color32, focus: bool) {
    rect_fill(p, r, 8.0, fill);
    if focus {
        rect_line(p, r.expand(5.0), 12.0, 3.0, FOCUS.gamma_multiply(0.22));
        rect_line(p, r.expand(2.0), 10.0, 3.0, FOCUS);
    }
}

/// The face of a controller button (or the D-pad, or the triggers) for the hints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Glyph {
    Btn(Nav),
    Dpad,
    Triggers,
}

pub fn glyph(p: &Painter, c: Pos2, r: f32, g: Glyph) {
    let font = FontId::proportional(r * 1.05);
    let round = |label: &str, col: Color32| {
        p.circle_filled(c, r, col);
        p.text(c, Align2::CENTER_CENTER, label, font.clone(), TEXT_DARK);
    };
    let pill = |label: &str, w: f32| {
        let rr = Rect::from_center_size(c, vec2(r * w, r * 1.6));
        rect_fill(p, rr, r * 0.5, TEXT_DIM);
        p.text(c, Align2::CENTER_CENTER, label, FontId::proportional(r * 0.85), TEXT_DARK);
    };
    match g {
        Glyph::Btn(Nav::A) => round("A", Color32::from_rgb(0x6b, 0xd1, 0x6b)),
        Glyph::Btn(Nav::B) => round("B", Color32::from_rgb(0xf0, 0x6a, 0x5c)),
        Glyph::Btn(Nav::X) => round("X", Color32::from_rgb(0x5c, 0xa8, 0xf0)),
        Glyph::Btn(Nav::Y) => round("Y", Color32::from_rgb(0xf2, 0xd0, 0x4f)),
        Glyph::Btn(Nav::PrevPage) => pill("LB", 2.3),
        Glyph::Btn(Nav::NextPage) => pill("RB", 2.3),
        Glyph::Btn(Nav::Start) => pill("Start", 3.6),
        Glyph::Btn(Nav::Select) => pill("Menu", 3.6),
        Glyph::Btn(Nav::Record) => pill("R3", 2.3),
        Glyph::Triggers => pill("LT RT", 3.6),
        Glyph::Dpad | Glyph::Btn(_) => {
            let w = r * 0.62;
            let col = TEXT_DIM;
            p.rect_filled(Rect::from_center_size(c, vec2(w, r * 2.0)), 2.0, col);
            p.rect_filled(Rect::from_center_size(c, vec2(r * 2.0, w)), 2.0, col);
        }
    }
}

/// How wide a glyph is drawn at radius `r`.
fn glyph_w(g: Glyph, r: f32) -> f32 {
    match g {
        Glyph::Btn(Nav::Start | Nav::Select) | Glyph::Triggers => r * 3.6,
        Glyph::Btn(Nav::PrevPage | Nav::NextPage | Nav::Record) => r * 2.3,
        _ => r * 2.0,
    }
}

/// Text that shrinks to fit `w`.
pub fn fit_text(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, w: f32, c: Color32) {
    let mut font = FontId::proportional(size);
    let width = p.layout_no_wrap(s.to_string(), font.clone(), c).size().x;
    if width > w && w > 0.0 {
        font.size = (size * w / width).max(size * 0.6);
    }
    p.text(pos, anchor, s, font, c);
}

/// Whole rows of `row_h` that a finger (dragging `resp`) or the mouse wheel
/// over `r` scrolled this frame (positive: towards the end of the list).
pub fn scrolled_rows(ui: &Ui, resp: &egui::Response, r: Rect, row_h: f32, id: Id) -> i64 {
    let mut dy = if resp.dragged() { resp.drag_delta().y } else { 0.0 };
    if ui.rect_contains_pointer(r) {
        dy += ui.input(|i| i.smooth_scroll_delta.y);
    }
    if dy == 0.0 {
        return 0;
    }
    let acc = ui.ctx().memory(|m| m.data.get_temp::<f32>(id).unwrap_or(0.0)) + dy;
    let k = (acc / row_h).trunc();
    ui.ctx().memory_mut(|m| m.data.insert_temp(id, acc - k * row_h));
    -(k as i64)
}

/// A value bar (0..1) in a row: filled to the value, its text on it.
pub fn value_bar(p: &Painter, r: Rect, v: f32, bipolar: bool, label: &str, s: &Sz, accent: Color32) {
    rect_fill(p, r, 6.0, BG);
    let v = v.clamp(0.0, 1.0);
    let (a, b) = if bipolar { (r.left() + r.width() * 0.5, r.left() + r.width() * v) } else { (r.left(), r.left() + r.width() * v) };
    let fill = Rect::from_min_max(pos2(a.min(b), r.top()), pos2(a.max(b).max(a.min(b) + 2.0), r.bottom()));
    rect_fill(p, fill, 6.0, shade(accent, 0.75));
    fit_text(p, r.center(), Align2::CENTER_CENTER, label, s.font, r.width() - 8.0, TEXT);
}

// ---------------------------------------------------------------- the menu

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuItem {
    Resume,
    Play,
    Record,
    Tempo,
    Metronome,
    Key,
    Scale,
    ScaleMode,
    Quantize,
    Undo,
    Redo,
    New,
    Open,
    Save,
    SaveAs,
    Demo,
    Ambient,
    StartHere,
    FullScreen,
    Desktop,
    Quit,
}

const MENU: [MenuItem; 21] = [
    MenuItem::Resume,
    MenuItem::Play,
    MenuItem::Record,
    MenuItem::Tempo,
    MenuItem::Metronome,
    MenuItem::Key,
    MenuItem::Scale,
    MenuItem::ScaleMode,
    MenuItem::Quantize,
    MenuItem::Undo,
    MenuItem::Redo,
    MenuItem::New,
    MenuItem::Open,
    MenuItem::Save,
    MenuItem::SaveAs,
    MenuItem::Demo,
    MenuItem::Ambient,
    MenuItem::StartHere,
    MenuItem::FullScreen,
    MenuItem::Desktop,
    MenuItem::Quit,
];

impl App {
    /// Switches to Big Picture.
    pub fn enter_big_picture(&mut self) {
        self.big.on = true;
        self.big.menu = None;
        if let TrackSel::Track(t) = self.sel_track {
            self.big.clip_cursor = (t, self.sel_scene);
        }
    }

    pub fn leave_big_picture(&mut self) {
        self.big.on = false;
        self.big.menu = None;
        self.big.clip_menu = None;
        self.big_release_notes();
    }

    /// Big Picture, the whole window.
    pub fn ui_big(&mut self, ctx: &egui::Context, mut inp: Frame) {
        if self.project.tracks.is_empty() {
            self.add_midi_track(Some(krabton_engine::Instrument::synth("Soft Pad")));
        }
        let fs = ctx.input(|i| i.viewport().fullscreen).unwrap_or(false);
        if fs != self.big.fullscreen {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.big.fullscreen));
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG)).show(ctx, |ui| {
            let full = ui.max_rect();
            let s = Sz::of(full);
            let top = Rect::from_min_size(full.min, vec2(full.width(), s.u * 1.6));
            let hints = Rect::from_min_max(pos2(full.left(), full.bottom() - s.u * 1.15), full.max);
            let body = Rect::from_min_max(pos2(full.left() + s.u * 0.4, top.bottom() + s.u * 0.35), pos2(full.right() - s.u * 0.4, hints.top() - s.u * 0.25));
            // Buttons that work everywhere.
            if self.big.menu.is_none() {
                if inp.take(Nav::PrevPage) {
                    self.big_turn_page(-1);
                }
                if inp.take(Nav::NextPage) {
                    self.big_turn_page(1);
                }
            }
            if inp.take(Nav::Start) {
                self.toggle_play();
            }
            if inp.take(Nav::Record) {
                self.toggle_record();
            }
            if inp.take(Nav::Select) {
                self.big.menu = if self.big.menu.is_some() { None } else { Some(0) };
            }
            self.big_top_bar(ui, top, &s);
            let popup = self.big.menu.is_some() || self.big.clip_menu.is_some();
            let page_inp = if popup { Frame { dt: inp.dt, ..Default::default() } } else { inp.clone() };
            let hint_list = match self.big.page {
                Page::Clips => self.big_clips(ui, body, &s, page_inp),
                Page::Play => self.big_play(ui, body, &s, page_inp),
                Page::Devices => self.big_devices(ui, body, &s, page_inp),
                Page::Mixer => self.big_mixer(ui, body, &s, page_inp),
                Page::Browse => self.big_browse(ui, body, &s, page_inp),
            };
            if self.big.clip_menu.is_some() && self.big.menu.is_none() {
                // A popup opened this frame does not see the press that opened it.
                let mut popup_inp = if popup { inp.clone() } else { Frame { dt: inp.dt, ..Default::default() } };
                self.big_clip_menu(ui, full, &s, &mut popup_inp);
                big_hints(ui, hints, &s, &[(Glyph::Dpad, "Move"), (Glyph::Btn(Nav::A), "Choose"), (Glyph::Btn(Nav::B), "Close")]);
            } else if self.big.menu.is_some() {
                self.big_menu(ui, full, &s, &mut inp);
                big_hints(ui, hints, &s, &[(Glyph::Dpad, "Move"), (Glyph::Btn(Nav::A), "Choose"), (Glyph::Btn(Nav::B), "Close")]);
            } else {
                let mut all = hint_list;
                all.push((Glyph::Btn(Nav::Start), if self.is_playing() { "Stop" } else { "Play" }));
                all.push((Glyph::Btn(Nav::Select), "Menu"));
                big_hints(ui, hints, &s, &all);
            }
            // Messages (a set loaded, something went wrong) above the hints.
            if let Some((msg, at)) = &self.message {
                if at.elapsed().as_secs_f32() < 4.0 {
                    let p = ui.painter();
                    let galley = p.layout_no_wrap(msg.clone(), s.f(), TEXT);
                    let size = galley.size() + vec2(s.u, s.u * 0.4);
                    let mr = Rect::from_center_size(pos2(full.center().x, hints.top() - s.u * 0.6), size);
                    rect_fill(p, mr, 10.0, PANEL3);
                    p.galley(mr.min + vec2(s.u * 0.5, s.u * 0.2), galley, TEXT);
                }
            }
        });
    }

    fn big_turn_page(&mut self, by: i32) {
        let i = Page::ALL.iter().position(|p| *p == self.big.page).unwrap_or(0) as i32;
        let n = Page::ALL.len() as i32;
        self.big.page = Page::ALL[(i + by).rem_euclid(n) as usize];
        self.big_release_notes();
    }

    /// The bar on top: the pages, the transport, the menu and the way back.
    fn big_top_bar(&mut self, ui: &mut Ui, r: Rect, s: &Sz) {
        let p = ui.painter().clone();
        p.rect_filled(r, 0.0, PANEL);
        let h = r.height() - s.u * 0.4;
        let y = r.center().y;
        let mut x = r.left() + s.u * 0.4;
        // The pages, between the shoulder buttons.
        glyph(&p, pos2(x + s.u * 0.45, y), s.u * 0.32, Glyph::Btn(Nav::PrevPage));
        x += s.u * 1.0;
        let accent = accent_now();
        let tab_w = (s.u * 3.0).min((r.width() * 0.46) / Page::ALL.len() as f32);
        for page in Page::ALL {
            let tr = Rect::from_min_size(pos2(x, y - h * 0.5), vec2(tab_w - s.u * 0.15, h));
            let on = self.big.page == page;
            let resp = ui.interact(tr, Id::new(("big_tab", page.name())), Sense::click());
            rect_fill(&p, tr, 8.0, if on { accent } else if resp.hovered() { TILE_HI } else { TILE });
            fit_text(&p, tr.center(), Align2::CENTER_CENTER, page.name(), s.font, tr.width() - 6.0, if on { TEXT_DARK } else { TEXT });
            if resp.clicked() {
                self.big.page = page;
                self.big.menu = None;
                self.big_release_notes();
            }
            x += tab_w;
        }
        glyph(&p, pos2(x + s.u * 0.4, y), s.u * 0.32, Glyph::Btn(Nav::NextPage));
        x += s.u * 1.1;
        // Transport.
        let playing = self.is_playing();
        let btn = |ui: &mut Ui, x: f32, w: f32, id: &str| {
            let br = Rect::from_min_size(pos2(x, y - h * 0.5), vec2(w, h));
            let resp = ui.interact(br, Id::new(("big_btn", id)), Sense::click());
            (br, resp)
        };
        let (pr, presp) = btn(ui, x, h * 1.3, "play");
        rect_fill(&p, pr, 8.0, if playing { PLAY } else { TILE });
        if playing {
            stop_icon(&p, pr.center(), h * 0.38, TEXT_DARK);
        } else {
            play_icon(&p, pr.center(), h * 0.5, TEXT);
        }
        if presp.clicked() {
            self.toggle_play();
        }
        x += h * 1.3 + s.u * 0.2;
        let (rr, rresp) = btn(ui, x, h * 1.3, "rec");
        let rec = self.recording;
        rect_fill(&p, rr, 8.0, if rec { REC } else { TILE });
        rec_icon(&p, rr.center(), h * 0.42, if rec { TEXT } else { REC });
        if rresp.clicked() {
            self.toggle_record();
        }
        x += h * 1.3 + s.u * 0.3;
        // Where the song is, and the tempo.
        let st = self.status();
        let pos = st.session_pos.load().max(0.0);
        let bpb = self.project.beats_per_bar_f();
        let bar = (pos / bpb).floor() as i64 + 1;
        let beat = (pos % bpb).floor() as i64 + 1;
        let right_w = s.u * 5.6;
        let info_w = (r.right() - right_w - x).max(0.0);
        if info_w > s.u * 2.0 {
            let info = format!("{bar}.{beat}   {:.0} BPM   {}", self.project.bpm, self.project.scale.name());
            fit_text(&p, pos2(x, y), Align2::LEFT_CENTER, &info, s.font, info_w - s.u * 0.2, TEXT_DIM);
        }
        // The menu and the way back to the desktop.
        let mr = Rect::from_min_size(pos2(r.right() - right_w, y - h * 0.5), vec2(s.u * 2.3, h));
        let mresp = ui.interact(mr, Id::new("big_menu_btn"), Sense::click());
        rect_fill(&p, mr, 8.0, if self.big.menu.is_some() { accent } else { TILE });
        fit_text(&p, mr.center(), Align2::CENTER_CENTER, "Menu", s.font, mr.width() - 6.0, if self.big.menu.is_some() { TEXT_DARK } else { TEXT });
        if mresp.clicked() {
            self.big.menu = if self.big.menu.is_some() { None } else { Some(0) };
        }
        let dr = Rect::from_min_size(pos2(mr.right() + s.u * 0.2, y - h * 0.5), vec2(r.right() - mr.right() - s.u * 0.6, h));
        let dresp = ui.interact(dr, Id::new("big_desktop_btn"), Sense::click());
        rect_fill(&p, dr, 8.0, if dresp.hovered() { TILE_HI } else { TILE });
        fit_text(&p, dr.center(), Align2::CENTER_CENTER, "Desktop", s.font, dr.width() - 6.0, TEXT);
        if dresp.on_hover_text("Back to the desktop view").clicked() {
            self.leave_big_picture();
        }
    }

    fn big_menu(&mut self, ui: &mut Ui, full: Rect, s: &Sz, inp: &mut Frame) {
        let p = ui.painter().clone();
        p.rect_filled(full, 0.0, Color32::from_black_alpha(170));
        let n = MENU.len();
        let mut cur = self.big.menu.unwrap_or(0).min(n - 1);
        let row_h = s.u * 1.05;
        let w = (s.u * 13.0).min(full.width() - s.u);
        let shown = (((full.height() - s.u * 3.0) / row_h).floor() as usize).clamp(3, n);
        let panel = Rect::from_center_size(full.center(), vec2(w, shown as f32 * row_h + s.u * 1.6));
        rect_fill(&p, panel, 12.0, PANEL2);
        text(&p, pos2(panel.left() + s.u * 0.5, panel.top() + s.u * 0.6), Align2::LEFT_CENTER, "Krabton", s.big(), accent_now());
        let pad = self.big.pads.name.clone().unwrap_or_else(|| "No controller".into());
        fit_text(&p, pos2(panel.right() - s.u * 0.5, panel.top() + s.u * 0.6), Align2::RIGHT_CENTER, &pad, s.small, panel.width() * 0.5, TEXT_DIM);
        // Moving and changing.
        let mut activate = false;
        let mut step = 0i32;
        for nav in inp.pressed.drain(..) {
            match nav {
                Nav::Up => cur = (cur + n - 1) % n,
                Nav::Down => cur = (cur + 1) % n,
                Nav::Left => step -= 1,
                Nav::Right => step += 1,
                Nav::A => activate = true,
                Nav::B => {
                    self.big.menu = None;
                    return;
                }
                _ => {}
            }
        }
        let first = cur.saturating_sub(shown / 2).min(n - shown);
        let list_top = panel.top() + s.u * 1.2;
        for (k, i) in (first..first + shown).enumerate() {
            let item = MENU[i];
            let rr = Rect::from_min_size(pos2(panel.left() + s.u * 0.3, list_top + k as f32 * row_h), vec2(panel.width() - s.u * 0.6, row_h - s.u * 0.12));
            let resp = ui.interact(rr, Id::new(("big_menu", i)), Sense::click());
            if resp.clicked() {
                cur = i;
                activate = true;
            }
            tile(&p, rr, if i == cur { TILE_HI } else { TILE }, i == cur);
            let (label, value) = self.menu_label(item);
            text(&p, pos2(rr.left() + s.u * 0.4, rr.center().y), Align2::LEFT_CENTER, label, s.f(), TEXT);
            if let Some(v) = value {
                let col = if matches!(item, MenuItem::Tempo | MenuItem::Key | MenuItem::Scale | MenuItem::Quantize) { accent_now() } else { TEXT_DIM };
                let vr = Rect::from_min_max(pos2(rr.center().x, rr.top()), rr.max);
                fit_text(&p, pos2(rr.right() - s.u * 0.4, rr.center().y), Align2::RIGHT_CENTER, &v, s.font, vr.width() - s.u * 0.5, col);
            }
        }
        self.big.menu = Some(cur);
        let item = MENU[cur];
        // The tempo turns with the triggers too.
        if item == MenuItem::Tempo && inp.turn.abs() > 0.01 {
            self.project.bpm = (self.project.bpm + inp.turn as f64 * inp.dt as f64 * 40.0).clamp(20.0, 999.0);
        }
        if step != 0 {
            self.menu_step(item, step);
        }
        if activate {
            self.menu_activate(item);
        }
    }

    fn menu_label(&self, item: MenuItem) -> (&'static str, Option<String>) {
        let on_off = |b: bool| Some(if b { "On".to_string() } else { "Off".to_string() });
        match item {
            MenuItem::Resume => ("Back", None),
            MenuItem::Play => (if self.is_playing() { "Stop" } else { "Play" }, None),
            MenuItem::Record => ("Record", on_off(self.recording)),
            MenuItem::Tempo => ("Tempo", Some(format!("< {:.1} BPM >", self.project.bpm))),
            MenuItem::Metronome => ("Metronome", on_off(self.project.metronome)),
            MenuItem::Key => ("Key", Some(format!("< {} >", krabton_engine::scale::NOTE_NAMES[self.project.scale.root as usize % 12]))),
            MenuItem::Scale => ("Scale", Some(format!("< {} >", self.project.scale.kind.name()))),
            MenuItem::ScaleMode => ("Notes in key", on_off(self.project.scale_mode)),
            MenuItem::Quantize => ("Launch on", Some(format!("< {} >", self.project.launch_quantize.name()))),
            MenuItem::Undo => ("Undo", None),
            MenuItem::Redo => ("Redo", None),
            MenuItem::New => ("New set", None),
            MenuItem::Open => ("Open a set...", None),
            MenuItem::Save => ("Save", None),
            MenuItem::SaveAs => ("Save as...", None),
            MenuItem::Demo => ("Load the demo set", None),
            MenuItem::Ambient => ("Load the ambient set", None),
            MenuItem::StartHere => ("Start in Big Picture", on_off(self.prefs.big_picture)),
            MenuItem::FullScreen => ("Full screen", on_off(self.big.fullscreen)),
            MenuItem::Desktop => ("Desktop view", None),
            MenuItem::Quit => ("Quit Krabton", None),
        }
    }

    fn menu_step(&mut self, item: MenuItem, step: i32) {
        use krabton_engine::{Quantize, Scale, ScaleKind};
        match item {
            MenuItem::Tempo => self.project.bpm = (self.project.bpm.round() + step as f64).clamp(20.0, 999.0),
            MenuItem::Key => self.project.scale = Scale::new(((self.project.scale.root as i32 + step).rem_euclid(12)) as u8, self.project.scale.kind),
            MenuItem::Scale => {
                let all = ScaleKind::ALL;
                let i = all.iter().position(|k| *k == self.project.scale.kind).unwrap_or(0) as i32;
                self.project.scale = Scale::new(self.project.scale.root, all[(i + step).rem_euclid(all.len() as i32) as usize]);
            }
            MenuItem::Quantize => {
                let all = Quantize::ALL;
                let i = all.iter().position(|q| *q == self.project.launch_quantize).unwrap_or(0) as i32;
                self.project.launch_quantize = all[(i + step).clamp(0, all.len() as i32 - 1) as usize];
            }
            MenuItem::Metronome | MenuItem::ScaleMode | MenuItem::StartHere | MenuItem::Record | MenuItem::FullScreen => self.menu_activate(item),
            _ => {}
        }
    }

    fn menu_activate(&mut self, item: MenuItem) {
        match item {
            MenuItem::Resume => self.big.menu = None,
            MenuItem::Play => self.toggle_play(),
            MenuItem::Record => self.toggle_record(),
            MenuItem::Tempo | MenuItem::Key | MenuItem::Scale | MenuItem::Quantize => {}
            MenuItem::Metronome => self.project.metronome = !self.project.metronome,
            MenuItem::ScaleMode => self.project.scale_mode = !self.project.scale_mode,
            MenuItem::Undo => self.undo(),
            MenuItem::Redo => self.redo(),
            MenuItem::New => {
                self.new_project();
                self.big.menu = None;
            }
            MenuItem::Open => {
                self.open();
                self.big.menu = None;
            }
            MenuItem::Save => self.save(false),
            MenuItem::SaveAs => self.save(true),
            MenuItem::Demo => {
                self.load_demo();
                self.big.menu = None;
            }
            MenuItem::Ambient => {
                self.load_ambient();
                self.big.menu = None;
            }
            MenuItem::StartHere => {
                self.prefs.big_picture = !self.prefs.big_picture;
                self.prefs.save();
            }
            MenuItem::FullScreen => self.big.fullscreen = !self.big.fullscreen,
            MenuItem::Desktop => self.leave_big_picture(),
            MenuItem::Quit => self.quit_requested = true,
        }
    }

    // ---------------------------------------------------------------- clips

    fn big_clips(&mut self, ui: &mut Ui, r: Rect, s: &Sz, mut inp: Frame) -> Vec<(Glyph, &'static str)> {
        let n_tracks = self.project.tracks.len().max(1);
        let n_scenes = self.project.scenes.len().max(1);
        // The column after the last track launches scenes.
        let (mut col, mut row) = self.big.clip_cursor;
        col = col.min(n_tracks);
        row = row.min(n_scenes - 1);
        let mut launch = false;
        for nav in inp.pressed.drain(..) {
            match nav {
                Nav::Left => col = col.saturating_sub(1),
                Nav::Right => col = (col + 1).min(n_tracks),
                Nav::Up => row = row.saturating_sub(1),
                Nav::Down => row = (row + 1).min(n_scenes - 1),
                Nav::A if col == n_tracks => self.cmd(Command::LaunchScene(row)),
                Nav::A => launch = true,
                Nav::X => {
                    if let Some(t) = self.project.tracks.get(col) {
                        let id = t.id;
                        self.cmd(Command::StopTrack { track: id });
                    }
                }
                Nav::Y if col < n_tracks => self.big.clip_menu = Some(0),
                _ => {}
            }
        }
        let on_scenes = col == n_tracks;
        let track_col = col.min(n_tracks - 1);
        let p = ui.painter().clone();
        let scene_w = s.u * 1.6;
        let head_h = s.u * 1.5;
        let gap = s.u * 0.18;
        let col_w_min = s.u * 4.6;
        let cols = (((r.width() - scene_w) / col_w_min).floor() as usize).clamp(1, n_tracks);
        let col_w = (r.width() - scene_w) / cols as f32;
        let cell_h = s.u * 1.45;
        let rows = (((r.height() - head_h) / cell_h).floor() as usize).clamp(1, n_scenes);
        // Keep the cursor in view.
        let (mut fc, mut fr) = self.big.clip_first;
        if track_col < fc {
            fc = track_col;
        } else if track_col >= fc + cols {
            fc = track_col + 1 - cols;
        }
        if row < fr {
            fr = row;
        } else if row >= fr + rows {
            fr = row + 1 - rows;
        }
        fc = fc.min(n_tracks.saturating_sub(cols));
        fr = fr.min(n_scenes.saturating_sub(rows));
        let st = self.status();
        let t_now = ui.input(|i| i.time);
        let grid = Rect::from_min_size(pos2(r.left(), r.top() + head_h), vec2(cols as f32 * col_w, rows as f32 * cell_h));
        for (k, ti) in (fc..(fc + cols).min(self.project.tracks.len())).enumerate() {
            let t = &self.project.tracks[ti];
            let x = r.left() + k as f32 * col_w;
            let color = rgb(t.color);
            // The track's head.
            let hr = Rect::from_min_size(pos2(x + gap * 0.5, r.top()), vec2(col_w - gap, head_h - gap));
            rect_fill(&p, hr, 8.0, TILE);
            rect_fill(&p, Rect::from_min_size(hr.min, vec2(hr.width(), s.u * 0.16)), 4.0, color);
            fit_text(&p, pos2(hr.left() + s.u * 0.3, hr.top() + head_h * 0.36), Align2::LEFT_CENTER, &t.name, s.font, hr.width() - s.u * 0.6, if ti == track_col { TEXT } else { TEXT_DIM });
            let inst = t.instrument.as_ref().map(|i| i.display_name()).unwrap_or_else(|| if t.kind == krabton_engine::TrackKind::Audio { "Audio".into() } else { String::new() });
            fit_text(&p, pos2(hr.left() + s.u * 0.3, hr.top() + head_h * 0.7), Align2::LEFT_CENTER, &inst, s.small, hr.width() - s.u * 1.0, TEXT_DIM);
            if t.arm {
                rec_icon(&p, pos2(hr.right() - s.u * 0.4, hr.top() + head_h * 0.7), s.u * 0.3, REC);
            }
            let ts = st.tracks.get(ti);
            let playing_slot = ts.map(|ts| ts.playing_slot.load(std::sync::atomic::Ordering::Relaxed)).unwrap_or(-1);
            let queued = ts.map(|ts| ts.queued_slot.load(std::sync::atomic::Ordering::Relaxed)).unwrap_or(-1);
            let clip_pos = ts.map(|ts| ts.clip_pos.load()).unwrap_or(-1.0);
            for (j, si) in (fr..(fr + rows).min(n_scenes)).enumerate() {
                let cr = Rect::from_min_size(pos2(x + gap * 0.5, grid.top() + j as f32 * cell_h + gap * 0.5), vec2(col_w - gap, cell_h - gap));
                let focus = !on_scenes && ti == col && si == row;
                match t.slots.get(si).and_then(|c| c.as_ref()) {
                    Some(clip) => {
                        let cc = rgb(clip.color);
                        let playing = playing_slot == si as i32;
                        tile(&p, cr, if playing { cc } else { shade(cc, 0.55) }, focus);
                        let tc = if playing { text_on(cc) } else { TEXT };
                        let icon = pos2(cr.left() + s.u * 0.45, cr.center().y);
                        if playing {
                            play_icon(&p, icon, s.u * 0.45, tc);
                            let frac = (clip_pos / clip.length.max(1e-6)).clamp(0.0, 1.0) as f32;
                            let bar = Rect::from_min_size(pos2(cr.left() + 4.0, cr.bottom() - 6.0), vec2((cr.width() - 8.0) * frac, 3.0));
                            p.rect_filled(bar, 1.0, tc);
                        } else if queued == si as i32 && (t_now * 4.0) as i64 % 2 == 0 {
                            play_icon(&p, icon, s.u * 0.45, FOCUS);
                        } else {
                            play_icon(&p, icon, s.u * 0.4, shade(tc, 0.7));
                        }
                        fit_text(&p, pos2(cr.left() + s.u * 0.9, cr.center().y), Align2::LEFT_CENTER, &clip.name, s.font, cr.width() - s.u * 1.1, tc);
                    }
                    None => {
                        tile(&p, cr, if focus { TILE_HI } else { Color32::from_rgb(0x20, 0x20, 0x24) }, focus);
                        stop_icon(&p, cr.center(), s.u * 0.22, if focus { TEXT_DIM } else { LINE });
                    }
                }
            }
        }
        // Scene launchers on the right.
        for (j, si) in (fr..(fr + rows).min(n_scenes)).enumerate() {
            let sr = Rect::from_min_size(pos2(grid.right() + gap, grid.top() + j as f32 * cell_h + gap * 0.5), vec2(scene_w - gap * 1.5, cell_h - gap));
            let resp = ui.interact(sr, Id::new(("big_scene", si)), Sense::click());
            tile(&p, sr, if resp.hovered() { TILE_HI } else { TILE }, on_scenes && si == row);
            play_icon(&p, sr.center(), s.u * 0.5, if si == row { FOCUS } else { TEXT_DIM });
            if resp.clicked() {
                self.cmd(Command::LaunchScene(si));
                row = si;
            }
        }
        // Touch: a tap launches, a drag scrolls.
        let resp = ui.interact(grid, Id::new("big_clip_grid"), Sense::click_and_drag());
        if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.clicked()) {
            let k = ((pt.x - grid.left()) / col_w).floor() as usize;
            let j = ((pt.y - grid.top()) / cell_h).floor() as usize;
            if fc + k < n_tracks && fr + j < n_scenes {
                col = fc + k;
                row = fr + j;
                launch = true;
            }
        }
        if resp.dragged() {
            let d = resp.drag_delta();
            let acc_id = Id::new("big_clip_drag");
            let mut acc: egui::Vec2 = ui.ctx().memory(|m| m.data.get_temp(acc_id).unwrap_or_default());
            acc += d;
            let dx = (acc.x / col_w).trunc();
            let dy = (acc.y / cell_h).trunc();
            acc.x -= dx * col_w;
            acc.y -= dy * cell_h;
            ui.ctx().memory_mut(|m| m.data.insert_temp(acc_id, acc));
            fc = (fc as i64 - dx as i64).clamp(0, n_tracks.saturating_sub(cols) as i64) as usize;
            fr = (fr as i64 - dy as i64).clamp(0, n_scenes.saturating_sub(rows) as i64) as usize;
            if !on_scenes {
                col = col.clamp(fc, fc + cols - 1);
            }
            row = row.clamp(fr, fr + rows - 1);
        }
        // Heads: a tap picks the track.
        let heads = Rect::from_min_size(r.min, vec2(cols as f32 * col_w, head_h));
        let hresp = ui.interact(heads, Id::new("big_clip_heads"), Sense::click());
        if let Some(pt) = hresp.interact_pointer_pos().filter(|_| hresp.clicked()) {
            let k = ((pt.x - heads.left()) / col_w).floor() as usize;
            if fc + k < n_tracks {
                col = fc + k;
            }
        }
        if launch && col < n_tracks {
            if let Some(t) = self.project.tracks.get(col) {
                let id = t.id;
                if t.slots.get(row).is_some_and(|c| c.is_some()) {
                    self.cmd(Command::LaunchClip { track: id, slot: row });
                } else {
                    self.cmd(Command::StopTrack { track: id });
                }
            }
        }
        self.big.clip_cursor = (col, row);
        self.big.clip_first = (fc, fr);
        let tc = col.min(n_tracks - 1);
        if self.sel_track != TrackSel::Track(tc) || self.sel_scene != row {
            self.sel_track = TrackSel::Track(tc);
            self.sel_scene = row;
            self.sel_clip = Some(crate::app::ClipSel::Slot { track: tc, scene: row });
        }
        if on_scenes {
            vec![(Glyph::Dpad, "Move"), (Glyph::Btn(Nav::A), "Launch scene"), (Glyph::Btn(Nav::PrevPage), "Pages")]
        } else {
            vec![(Glyph::Dpad, "Move"), (Glyph::Btn(Nav::A), "Launch"), (Glyph::Btn(Nav::X), "Stop track"), (Glyph::Btn(Nav::Y), "Clip"), (Glyph::Btn(Nav::PrevPage), "Pages")]
        }
    }

    /// What can be done with the slot under the cursor.
    fn big_clip_menu(&mut self, ui: &mut Ui, full: Rect, s: &Sz, inp: &mut Frame) {
        let (t, scene) = self.big.clip_cursor;
        let Some(track) = self.project.tracks.get(t) else {
            self.big.clip_menu = None;
            return;
        };
        let has_clip = track.slots.get(scene).is_some_and(|c| c.is_some());
        let midi = track.kind == krabton_engine::TrackKind::Midi;
        let mut items: Vec<(&str, u8)> = Vec::new();
        if midi {
            items.push(("Record here", 0));
        }
        if midi && !has_clip {
            items.push(("New empty clip", 1));
        }
        if has_clip {
            items.push(("Duplicate", 2));
            items.push(("Delete", 3));
        }
        items.push(("Stop all clips", 4));
        items.push(("Close", 5));
        let n = items.len();
        let mut cur = self.big.clip_menu.unwrap_or(0).min(n - 1);
        let mut activate = false;
        for nav in inp.pressed.drain(..) {
            match nav {
                Nav::Up => cur = (cur + n - 1) % n,
                Nav::Down => cur = (cur + 1) % n,
                Nav::A => activate = true,
                Nav::B | Nav::Y => {
                    self.big.clip_menu = None;
                    return;
                }
                _ => {}
            }
        }
        let p = ui.painter().clone();
        p.rect_filled(full, 0.0, Color32::from_black_alpha(150));
        let row_h = s.u * 1.1;
        let panel = Rect::from_center_size(full.center(), vec2((s.u * 10.0).min(full.width() - s.u), n as f32 * row_h + s.u * 1.5));
        rect_fill(&p, panel, 12.0, PANEL2);
        let title = format!("{} · {}", track.name, self.project.scenes.get(scene).map(|sc| sc.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| format!("Scene {}", scene + 1)));
        fit_text(&p, pos2(panel.left() + s.u * 0.5, panel.top() + s.u * 0.6), Align2::LEFT_CENTER, &title, s.font, panel.width() - s.u, accent_now());
        for (k, (label, _)) in items.iter().enumerate() {
            let rr = Rect::from_min_size(pos2(panel.left() + s.u * 0.3, panel.top() + s.u * 1.2 + k as f32 * row_h), vec2(panel.width() - s.u * 0.6, row_h - s.u * 0.12));
            let resp = ui.interact(rr, Id::new(("big_clip_menu", k)), Sense::click());
            if resp.clicked() {
                cur = k;
                activate = true;
            }
            tile(&p, rr, if k == cur { TILE_HI } else { TILE }, k == cur);
            text(&p, pos2(rr.left() + s.u * 0.4, rr.center().y), Align2::LEFT_CENTER, *label, s.f(), TEXT);
        }
        self.big.clip_menu = Some(cur);
        if activate {
            let sel = crate::app::ClipSel::Slot { track: t, scene };
            self.big.clip_menu = None;
            match items[cur].1 {
                0 => {
                    // Session record into the first empty slot from here.
                    self.sel_track = TrackSel::Track(t);
                    self.sel_scene = scene;
                    for (i, tr) in self.project.tracks.iter_mut().enumerate() {
                        tr.arm = i == t;
                    }
                    if !self.recording {
                        self.toggle_record();
                    }
                    if !self.is_playing() {
                        self.toggle_play();
                    }
                }
                1 => self.create_midi_clip(t, scene),
                2 => self.duplicate_clip(sel),
                3 => self.delete_clip(sel),
                4 => self.cmd(Command::StopAllClips),
                _ => {}
            }
        }
    }

    /// Lets go of every note the pages hold.
    pub fn big_release_notes(&mut self) {
        let held = std::mem::take(&mut self.big.play.held);
        for (_, (pitch, ids)) in held {
            self.note_off(pitch, &ids);
        }
    }
}

/// The bottom row: what the buttons do here.
fn big_hints(ui: &Ui, r: Rect, s: &Sz, hints: &[(Glyph, &str)]) {
    let p = ui.painter();
    p.rect_filled(r, 0.0, PANEL);
    let gr = s.u * 0.3;
    let mut x = r.left() + s.u * 0.5;
    for (g, label) in hints {
        let w = glyph_w(*g, gr);
        if x + w + s.u * 2.0 > r.right() {
            break;
        }
        glyph(p, pos2(x + w * 0.5, r.center().y), gr, *g);
        x += w + s.u * 0.2;
        let t = text(p, pos2(x, r.center().y), Align2::LEFT_CENTER, *label, s.s(), TEXT_DIM);
        x = t.right() + s.u * 0.6;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_follow_the_screen() {
        let deck = Sz::of(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0)));
        let devterm = Sz::of(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 480.0)));
        assert!(deck.u > devterm.u);
        assert!(devterm.font >= 14.0);
    }
}
