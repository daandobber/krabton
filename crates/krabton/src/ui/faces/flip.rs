//! Flipside, late at night: glass panels on a starry indigo sky, its name
//! on a cassette's label. The sample's wave glows with its chops over it,
//! numbered with the keys they are on; a chop's line is dragged to move it
//! (it snaps to the hits and to where the wave crosses zero; Alt for free,
//! Shift for fine), a double-click puts a new line, a right-click on a line
//! (or dragging it off the top or bottom) takes it away, and the lines keep
//! to their places between their neighbours. The wheel scrolls a wave
//! zoomed in with Ctrl and the wheel. Pressing a chop plays it and chooses
//! it for the panel beside, where its own sound is: pitch, fine, gain, pan,
//! trim, fades, tone, and click for its direction, way of playing, choke
//! group and stretch. Under the wave the ways to chop it (again), the order
//! shuffled or the sound thrown about, an auto-flip into a clip and the
//! chops sent to a Drum Rack or Analog Princess. Then the flip's own
//! sections: its pitch, the envelope, the filter, the voices, the LFO, the
//! tape and crusher, the fry. A sample dropped on it is flipped.

use super::{draw, Ctl, Ctx, Face, Presets};
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Mesh, Pos2, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::princess_flip::{self as pf, AutoStyle, ChopBy, Flip, FlipChop, Grid, Play};
use krabton_engine::sample::SampleRef;
use std::cell::Cell;

/// The section that holds the wave (units), and where the rest start.
const WAVE_W: f32 = 40.0;
const CHOP_X: f32 = 40.2;
const CHOP_W: f32 = 24.0;

pub(crate) fn face() -> Face {
    let x = CHOP_X + CHOP_W + 0.2;
    Face::new(x + 79.8, 12.0)
        .screened(&["Chop"])
        .screen(0.0, 0.0, WAVE_W, 12.0, "glass:Flipside")
        .screen(0.3, 0.3, WAVE_W - 0.6, 1.5, "label")
        .preset(2.6, 0.45, 15.0, 1.2)
        .screen(0.3, 1.95, WAVE_W - 0.6, 6.45, "wave")
        .screen(0.3, 8.5, WAVE_W - 0.6, 2.6, "chopctl")
        .screen(CHOP_X, 0.0, CHOP_W, 12.0, "chop")
        // Its pitch and how it plays.
        .screen(x, 0.0, 10.0, 12.0, "glass:Flip")
        .named(x + 0.2, 0.3, 2.4, 10.8, "Pitch", Ctl::Fader, "Pitch")
        .named(x + 2.7, 0.3, 7.1, 3.5, "Slowed + Reverb", Ctl::Switch, "Slowed")
        .named(x + 2.7, 3.9, 3.5, 3.6, "Song Key", Ctl::Switch, "Key")
        .named(x + 6.3, 3.9, 3.5, 3.6, "Half-Time", Ctl::Switch, "Half")
        .named(x + 2.7, 7.6, 7.1, 3.5, "Mode", Ctl::Buttons, "Mode")
        // The envelope and what velocity does to the level.
        .screen(x + 10.2, 0.0, 11.9, 12.0, "glass:Envelope")
        .row(x + 10.4, 0.3, 2.3, 10.8, &["Attack", "Decay", "Sustain", "Release", "Vel > Level"], Ctl::Fader)
        // The filter.
        .screen(x + 22.3, 0.0, 12.9, 12.0, "glass:Filter")
        .named(x + 22.5, 0.3, 3.4, 10.8, "Filter Type", Ctl::Buttons, "Type")
        .named(x + 25.9, 0.3, 2.3, 10.8, "Low Pass", Ctl::Fader, "Cutoff")
        .named(x + 28.2, 0.3, 2.3, 10.8, "Resonance", Ctl::Fader, "Res")
        .named(x + 30.5, 0.3, 2.3, 10.8, "Env Amount", Ctl::Fader, "Env")
        .named(x + 32.8, 0.3, 2.3, 10.8, "Vel > Filter", Ctl::Fader, "Vel")
        // The voices.
        .screen(x + 35.4, 0.0, 14.6, 12.0, "glass:Voices")
        .named(x + 35.6, 0.3, 3.3, 5.6, "Voicing", Ctl::Buttons, "Voicing")
        .named(x + 35.6, 6.0, 3.3, 2.5, "Voices", Ctl::Lcd, "Voices")
        .named(x + 35.6, 8.6, 3.3, 2.5, "Choke", Ctl::Switch, "Choke")
        .named(x + 39.0, 0.3, 2.3, 10.8, "Glide", Ctl::Fader, "Glide")
        .named(x + 41.3, 0.3, 2.3, 10.8, "Vel > Pitch", Ctl::Fader, "Vel Pitch")
        .named(x + 43.7, 0.3, 3.0, 10.8, "Stutter", Ctl::Buttons, "Stutter")
        .named(x + 46.8, 0.3, 3.0, 10.8, "Hiss", Ctl::Fader, "Hiss")
        // The LFO in the song's time.
        .screen(x + 50.2, 0.0, 10.6, 12.0, "glass:LFO")
        .named(x + 50.4, 0.3, 3.2, 2.6, "LFO Rate", Ctl::Lcd, "Rate")
        .named(x + 50.4, 3.0, 3.2, 8.1, "LFO Shape", Ctl::Buttons, "Shape")
        .named(x + 53.7, 0.3, 2.3, 10.8, "LFO > Pitch", Ctl::Fader, "Pitch")
        .named(x + 56.0, 0.3, 2.3, 10.8, "LFO > Filter", Ctl::Fader, "Filter")
        .named(x + 58.3, 0.3, 2.3, 10.8, "LFO > Pan", Ctl::Fader, "Pan")
        // The tape and the crusher.
        .screen(x + 61.0, 0.0, 9.0, 12.0, "glass:Tape")
        .named(x + 61.2, 0.3, 2.3, 10.8, "Wobble", Ctl::Fader, "Wobble")
        .named(x + 63.5, 0.3, 2.8, 10.8, "Wobble Type", Ctl::Buttons, "Kind")
        .named(x + 66.4, 0.3, 1.8, 10.8, "Bits", Ctl::Fader, "Bits")
        .named(x + 68.2, 0.3, 1.8, 10.8, "Crush Rate", Ctl::Fader, "Rate")
        // The fry, the room, how wide, how loud.
        .screen(x + 70.2, 0.0, 9.6, 12.0, "glass:Fry")
        .row(x + 70.4, 0.3, 2.3, 10.8, &["Fry", "Reverb", "Width", "Volume"], Ctl::Fader)
}

/// The night: its sky, its pastels.
const SKY_TOP: Color32 = Color32::from_rgb(0x2c, 0x22, 0x66);
const SKY_BOTTOM: Color32 = Color32::from_rgb(0x10, 0x0d, 0x30);
const PINK: Color32 = Color32::from_rgb(0xff, 0xa6, 0xd9);
const CYAN: Color32 = Color32::from_rgb(0x9e, 0xe8, 0xff);
const LILAC: Color32 = Color32::from_rgb(0xc9, 0xb0, 0xff);
const BUTTER: Color32 = Color32::from_rgb(0xff, 0xf0, 0xa8);
const LABEL: Color32 = Color32::from_rgb(0xff, 0xe3, 0xf1);
const LABEL_INK: Color32 = Color32::from_rgb(0x2a, 0x1f, 0x5c);
/// The chops' colours without the skin.
const PADS: [Color32; 4] = [PINK, CYAN, LILAC, BUTTER];

/// What the face does this frame: notes (None: let go), a message, an
/// auto-flip or an export for the app to make.
pub(crate) struct Played {
    pub notes: Vec<(u8, Option<f32>)>,
    pub said: Option<String>,
    pub auto: Option<AutoStyle>,
    pub export: Option<Export>,
}

/// Where the chops are sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Export {
    /// A new track with a Drum Rack of them.
    DrumRack,
    /// A new Analog Princess, a chop on each of its tracks.
    Princess,
}

/// What the face keeps between frames: the auto-flip's style.
#[derive(Clone, Copy, Default)]
struct ChopView {
    style: usize,
}

/// How much of the wave shows: zoomed in so far, from this far into it (0..1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct View {
    pub zoom: f32,
    pub from: f64,
}

impl Default for View {
    fn default() -> Self {
        Self { zoom: 1.0, from: 0.0 }
    }
}

thread_local! {
    /// A view the debug script asks for, taken by the next wave drawn.
    static SCRIPT_VIEW: Cell<Option<View>> = const { Cell::new(None) };
}

/// The debug script's zoom: the next wave drawn shows this much.
pub(crate) fn script_view(v: View) {
    SCRIPT_VIEW.with(|c| c.set(Some(v)));
}

/// What a press on the wave is doing.
#[derive(Clone, Copy, Debug, Default)]
enum Hold {
    #[default]
    Nothing,
    /// Playing this note.
    Note(u8),
    /// Moving chop `k`'s line, from `from` (frames) where the press was at
    /// `x`; `off` while it is dragged off the wave (let go: it goes).
    Line { k: usize, from: u64, x: f32, off: bool },
}

/// A sample flipped: listened to and chopped as the flip was.
pub(crate) fn flip_onto(sample: &mut Option<SampleRef>, flip: &mut Flip, s: SampleRef) {
    *flip = pf::listen_as(&s, flip.chopping);
    *sample = Some(s);
}

/// Chopped again as its `chopping` says (keeping the hand's lines unless `all`).
pub(crate) fn rechop(sample: Option<&SampleRef>, flip: &mut Flip, all: bool) {
    if let Some(s) = sample {
        flip.rechop(s, all);
    }
}

/// The chosen chop (0 up), as the Chop parameter says.
pub(crate) fn chosen(p: &[f32], flip: &Flip) -> usize {
    (p.get(pf::CHOP).copied().unwrap_or(1).round().max(1.0) as usize - 1).min(flip.chops.len().saturating_sub(1))
}

/// Flipside's panel.
#[allow(clippy::too_many_arguments)]
pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut Vec<f32>, sample: &mut Option<SampleRef>, flip: &mut Flip, live: &InstLive, height: f32, mv: Option<&mut ModView>) -> Played {
    let mut out = Played { notes: Vec::new(), said: None, auto: None, export: None };
    if params.len() < pf::SPECS.len() {
        return out;
    }
    let h = height.max(120.0);
    let face = face();
    let vid = id.with("chopview");
    let mut cv: ChopView = ui.ctx().data(|d| d.get_temp(vid)).unwrap_or_default();
    let mut notes = Vec::new();
    let mut said = None;
    let mut load: Option<std::path::PathBuf> = None;
    let mut acts = Acts::default();
    let filter_was = params[pf::FILTER_TYPE];
    let resp = {
        let (smp, fl) = (&*sample, &mut *flip);
        let mut cx = Ctx { id, keys: pf::SPECS, specs: pf::SPECS, params, mv, presets: Some(Presets { current: preset, names: pf::preset_names(), load: pf::preset }) };
        ui.scope(|ui| {
            draw(ui, &face, h, &mut cx, &mut |ui, which, rect, p| match which {
                "label" => {
                    label(ui, rect, p, smp.as_ref(), fl, live);
                    false
                }
                "wave" => wave(ui, id.with("wave"), rect, p, smp.as_ref(), fl, live, &mut notes),
                "chopctl" => chop_buttons(ui, id.with("chopctl"), rect, smp.is_some(), fl, &mut cv, &mut load, &mut acts),
                "chop" => chop_panel(ui, id.with("chop"), rect, p, smp.as_ref(), fl, live),
                g => {
                    if let Some(title) = g.strip_prefix("glass:") {
                        glass(ui.painter(), rect, title);
                    }
                    false
                }
            });
        })
        .response
    };
    // A filter turned to high or band pass starts somewhere it is heard.
    if params[pf::FILTER_TYPE] != filter_was && params[pf::FILTER_TYPE] >= 0.5 && params[pf::LOW_PASS] >= 19_000.0 {
        params[pf::LOW_PASS] = if params[pf::FILTER_TYPE] < 1.5 { 300.0 } else { 1000.0 };
    }
    if let Some(all) = acts.rechop {
        rechop(sample.as_ref(), flip, all);
    }
    out.auto = acts.auto;
    out.export = acts.export;
    // A sample from the browser or from outside.
    if let Some(item) = drop_payload::<crate::app::BrowserItem>(&resp) {
        if let crate::app::BrowserItem::Sample(path) = &*item {
            load = Some(path.clone());
        }
    }
    let here = crate::import::file_drop_pos(ui.ctx()).is_none_or(|p| resp.rect.contains(p));
    let files: Vec<std::path::PathBuf> = ui.ctx().input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
    if let Some(path) = files.iter().find(|p| krabton_engine::sample::is_audio_file(p)).filter(|_| here) {
        load = Some(path.clone());
        crate::import::take_file_drop(ui.ctx());
    }
    if let Some(path) = load {
        match SampleRef::load(&path) {
            Ok(s) => flip_onto(sample, flip, s),
            Err(e) => said = Some(format!("Could not load the sample: {e}")),
        }
    }
    ui.ctx().data_mut(|d| d.insert_temp(vid, cv));
    if live.vals[pf::VIS_VOICES] > 0.0 || live.vals[..pf::MAX_CHOPS].iter().any(|v| *v > 0.01) {
        ui.ctx().request_repaint();
    }
    out.notes = notes;
    out.said = said.or(acts.said);
    out
}

/// What the buttons under the wave asked for.
#[derive(Default)]
struct Acts {
    /// Chop again (all of it: the hand's lines too).
    rechop: Option<bool>,
    auto: Option<AutoStyle>,
    export: Option<Export>,
    said: Option<String>,
}

/// A rounded pane of glass, its name along the bottom.
fn glass(p: &egui::Painter, r: Rect, title: &str) {
    let r = r.shrink(1.0);
    let th = 13.0f32.max(r.height() * 0.07);
    let body = Rect::from_min_max(r.min, pos2(r.max.x, r.max.y - th));
    p.rect_filled(body.translate(vec2(0.0, 2.0)), 10.0, Color32::from_black_alpha(50));
    let mut mesh = Mesh::default();
    let (top, bottom) = (Color32::from_rgba_unmultiplied(0xc9, 0xb0, 0xff, 26), Color32::from_rgba_unmultiplied(0x9e, 0xe8, 0xff, 10));
    mesh.colored_vertex(body.left_top(), top);
    mesh.colored_vertex(body.right_top(), top);
    mesh.colored_vertex(body.right_bottom(), bottom);
    mesh.colored_vertex(body.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.rect_filled(body, 10.0, Color32::from_rgba_unmultiplied(0x18, 0x12, 0x40, 120));
    p.add(Shape::mesh(mesh));
    // A shine along its top edge.
    p.line_segment([pos2(body.left() + 10.0, body.top() + 1.0), pos2(body.right() - 10.0, body.top() + 1.0)], Stroke::new(1.0_f32, Color32::from_white_alpha(46)));
    p.rect_stroke(body, 10.0, Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0xff, 0xd6, 0xee, 52)), egui::StrokeKind::Inside);
    let t = title.to_lowercase();
    let font = super::fit(p, &t, crate::skin::Font::Retro.id((th * 0.82).clamp(9.0, 14.0)), r.width() - 10.0);
    p.text(pos2(r.center().x, r.max.y - th * 0.5), Align2::CENTER_CENTER, t, font, mix(PINK, Color32::WHITE, 0.35));
}

/// A star of four points (a sparkle).
fn sparkle(p: &egui::Painter, c: Pos2, r: f32, col: Color32) {
    let s = Stroke::new(1.0_f32, col);
    p.line_segment([c - vec2(r, 0.0), c + vec2(r, 0.0)], s);
    p.line_segment([c - vec2(0.0, r), c + vec2(0.0, r)], s);
    p.circle_filled(c, (r * 0.28).max(0.8), col);
}

fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

/// The cassette's label across the top: the preset sits in it, beside the
/// sample's name, and what was heard in it (key, tempo, how far it moves).
fn label(ui: &mut Ui, r: Rect, p: &[f32], sample: Option<&SampleRef>, flip: &Flip, live: &InstLive) {
    let pt = ui.painter();
    pt.rect_filled(r.translate(vec2(0.0, 1.5)), 6.0, Color32::from_black_alpha(70));
    pt.rect_filled(r, 6.0, LABEL);
    // The label's stripes and the two reels' windows.
    let band = Rect::from_min_max(pos2(r.left() + 6.0, r.bottom() - 4.0), pos2(r.right() - 6.0, r.bottom() - 2.5));
    pt.rect_filled(band, 1.0, PINK);
    pt.rect_filled(band.translate(vec2(0.0, -2.5)), 1.0, CYAN);
    let reel = r.height() * 0.28;
    for x in [r.left() + r.height() * 0.62, r.right() - r.height() * 0.62] {
        let c = pos2(x, r.center().y - 1.0);
        pt.circle_filled(c, reel, LABEL_INK);
        pt.circle_stroke(c, reel * 0.55, Stroke::new(1.0_f32, LABEL));
    }
    let side = Rect::from_min_max(pos2(r.left() + r.width() * 0.44, r.top()), pos2(r.right() - r.height() * 1.3, r.bottom() - 4.0));
    let name = sample.map_or("side A · drop a sample".to_string(), |s| format!("side A · {}", s.name));
    let f = super::fit(pt, &name, crate::skin::Font::Retro.id((r.height() * 0.42).clamp(9.0, 13.0)), side.width() * 0.55);
    pt.text(pos2(side.left(), side.center().y), Align2::LEFT_CENTER, name, f, LABEL_INK);
    let shift = live.vals[pf::VIS_SHIFT].round() as i32;
    let key = flip.key_name().unwrap_or_else(|| "no key".into());
    let bpm = flip.bpm.map_or("no tempo".to_string(), |b| format!("{b:.1} BPM"));
    let text = if p[pf::SONG_KEY] >= 0.5 && flip.key.is_some() { format!("{key} · {bpm} · {shift:+} st") } else { format!("{key} · {bpm}") };
    let f = super::fit(pt, &text, crate::skin::Font::Lcd.id((r.height() * 0.6).clamp(10.0, 17.0)), side.width() * 0.44);
    pt.text(pos2(side.right(), side.center().y), Align2::RIGHT_CENTER, text, f, LABEL_INK);
}

/// Where the wave is drawn: frames to x and back.
struct Map {
    left: f32,
    width: f32,
    v0: f64,
    len: f64,
}

impl Map {
    fn x(&self, frame: f64) -> f32 {
        self.left + ((frame - self.v0) / self.len.max(1.0)) as f32 * self.width
    }

    fn frame(&self, x: f32) -> f64 {
        self.v0 + ((x - self.left) / self.width.max(1.0)) as f64 * self.len
    }
}

/// The wave and its chops: press a chop to play it (and choose it), drag a
/// line to move it, double-click for a new one, right-click a line to take
/// it away (or a chop to turn it round), the wheel to zoom and scroll.
#[allow(clippy::too_many_arguments)]
fn wave(ui: &mut Ui, id: Id, r: Rect, p: &mut [f32], sample: Option<&SampleRef>, flip: &mut Flip, live: &InstLive, notes: &mut Vec<(u8, Option<f32>)>) -> bool {
    let painter = ui.painter().clone();
    // The sky, deep at the bottom, with its stars.
    painter.rect_filled(r.expand(1.5), 9.0, Color32::from_black_alpha(120));
    let mut sky = Mesh::default();
    sky.colored_vertex(r.left_top(), SKY_TOP);
    sky.colored_vertex(r.right_top(), SKY_TOP);
    sky.colored_vertex(r.right_bottom(), SKY_BOTTOM);
    sky.colored_vertex(r.left_bottom(), SKY_BOTTOM);
    sky.add_triangle(0, 1, 2);
    sky.add_triangle(0, 2, 3);
    painter.rect_filled(r, 8.0, SKY_BOTTOM);
    painter.with_clip_rect(r.shrink(3.0)).add(Shape::mesh(sky));
    for k in 0..(r.width() * r.height() / 1400.0) as u32 {
        let h = hash(k.wrapping_mul(2_654_435_761) ^ 0xF11D);
        let at = pos2(r.left() + (h % 1000) as f32 / 1000.0 * r.width(), r.top() + (h >> 10 & 1023) as f32 / 1023.0 * r.height());
        let a = 40 + (h >> 20 & 63) as u8;
        if h >> 27 == 0 {
            sparkle(&painter, at, 3.0, Color32::from_white_alpha(a + 40));
        } else {
            painter.circle_filled(at, 0.7, Color32::from_white_alpha(a));
        }
    }
    painter.rect_stroke(r, 8.0, Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0xff, 0xd6, 0xee, 60)), egui::StrokeKind::Inside);
    let pads = crate::skin::current().map(|s| s.pads).filter(|p| p.len() >= 4).unwrap_or(&PADS);
    let Some(s) = sample else {
        let f = crate::skin::Font::Retro.id((r.height() * 0.12).clamp(11.0, 18.0));
        painter.text(r.center() - vec2(0.0, r.height() * 0.06), Align2::CENTER_CENTER, "drop a sample to flip it", f, PINK);
        painter.text(r.center() + vec2(0.0, r.height() * 0.12), Align2::CENTER_CENTER, "soul, R&B, anime, a game, a video's sound: its key and tempo are heard, it is chopped over the keys from C3", FontId::proportional((r.height() * 0.065).clamp(8.0, 11.0)), mix(LILAC, Color32::WHITE, 0.3));
        return false;
    };
    let n = s.frames().max(1) as f64;
    let inner = r.shrink(5.0);
    let handle_h = (inner.height() * 0.13).clamp(12.0, 17.0);
    let bar_h = 4.0;
    let wave = Rect::from_min_max(pos2(inner.left(), inner.top() + handle_h + 2.0), pos2(inner.right(), inner.bottom() - bar_h - 2.0));
    let mid = wave.center().y;
    // What shows: the script's, or as it was, zoomed and scrolled by the wheel.
    let vkey = id.with("view");
    let mut view: View = SCRIPT_VIEW.with(|c| c.take()).unwrap_or_else(|| ui.ctx().data(|d| d.get_temp(vkey)).unwrap_or_default());
    let resp = ui.interact(r, id, Sense::click_and_drag());
    if resp.hovered() {
        let (zoom, scroll, ctrl) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta, i.modifiers.ctrl || i.modifiers.command));
        let pointer = resp.hover_pos().map_or(0.5, |pt| ((pt.x - wave.left()) / wave.width().max(1.0)).clamp(0.0, 1.0) as f64);
        let mut z = zoom;
        if z == 1.0 && ctrl && scroll.y != 0.0 {
            z = (scroll.y * 0.01).exp();
        }
        if z != 1.0 {
            let at = view.from + pointer / view.zoom as f64;
            view.zoom = (view.zoom * z).clamp(1.0, 256.0);
            view.from = at - pointer / view.zoom as f64;
        } else if (scroll.x != 0.0 || scroll.y != 0.0) && view.zoom > 1.0 {
            view.from -= (scroll.x + scroll.y) as f64 / wave.width().max(1.0) as f64 / view.zoom as f64;
        }
        if z != 1.0 || view.zoom > 1.0 && scroll != egui::Vec2::ZERO {
            ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
        }
    }
    view.from = view.from.clamp(0.0, 1.0 - 1.0 / view.zoom as f64);
    ui.ctx().data_mut(|d| d.insert_temp(vkey, view));
    let map = Map { left: wave.left(), width: wave.width(), v0: view.from * n, len: n / view.zoom as f64 };
    let clip = painter.with_clip_rect(inner);
    // The chops' bands under the wave, every other one a little lighter.
    let sel = chosen(p, flip);
    let lit = |k: usize| live.vals[k.min(pf::MAX_CHOPS - 1)].clamp(0.0, 1.0);
    for (k, c) in flip.chops.iter().enumerate() {
        let (x0, x1) = (map.x(c.start as f64), map.x(c.end as f64));
        if x1 < wave.left() || x0 > wave.right() {
            continue;
        }
        let band = Rect::from_min_max(pos2(x0, wave.top()), pos2(x1.max(x0 + 1.0), wave.bottom()));
        let col = pads[k % 4];
        clip.rect_filled(band, 0.0, Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), (10.0 + 18.0 * (k % 2) as f32 + 90.0 * lit(k)) as u8));
        if k == sel {
            clip.rect_stroke(band, 3.0, Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 170)), egui::StrokeKind::Inside);
        }
    }
    // The wave, glowing: from its overview, or its samples zoomed in close.
    let cols = wave.width().max(1.0) as usize;
    let scale = s.peak().max(1e-4).recip().min(4.0) * wave.height() * 0.46;
    let per_peak = n / s.peaks.len().max(1) as f64;
    let mut tops = Vec::with_capacity(cols);
    for c in 0..cols {
        let (f0, f1) = (map.frame(wave.left() + c as f32), map.frame(wave.left() + c as f32 + 1.0));
        let (lo, hi) = if f1 - f0 >= per_peak && !s.peaks.is_empty() {
            let a = ((f0 / per_peak) as usize).min(s.peaks.len() - 1);
            let b = ((f1 / per_peak).ceil() as usize).clamp(a + 1, s.peaks.len());
            s.peaks[a..b].iter().fold((0.0f32, 0.0f32), |(l, h), &(mn, mx)| (l.min(mn), h.max(mx)))
        } else {
            let (a, b) = ((f0.max(0.0) as usize).min(s.frames()), (f1.ceil().max(0.0) as usize).min(s.frames()));
            (a..b.max(a + 1).min(s.frames())).fold((0.0f32, 0.0f32), |(l, h), i| {
                let v = 0.5 * (s.left[i] + s.right[i]);
                (l.min(v), h.max(v))
            })
        };
        tops.push((lo, hi));
    }
    for (c, &(lo, hi)) in tops.iter().enumerate() {
        let x = wave.left() + c as f32 + 0.5;
        let (y0, y1) = (mid - hi * scale, mid - lo * scale);
        clip.line_segment([pos2(x, y0 - 1.5), pos2(x, y1 + 1.5)], Stroke::new(3.0_f32, Color32::from_rgba_unmultiplied(0x9e, 0xe8, 0xff, 26)));
        clip.line_segment([pos2(x, y0), pos2(x, y1.max(y0 + 0.5))], Stroke::new(1.0_f32, mix(CYAN, Color32::WHITE, 0.45)));
    }
    // The hits it heard, as small marks at the bottom (where lines snap).
    if let Some(an) = s.analysis_ready() {
        for &t in &an.transients {
            let x = map.x(t as f64);
            if x >= wave.left() && x <= wave.right() {
                clip.line_segment([pos2(x, wave.bottom() - 4.0), pos2(x, wave.bottom())], Stroke::new(1.0_f32, Color32::from_white_alpha(70)));
            }
        }
    }
    // The press: a line held, a note played.
    let hkey = id.with("hold");
    let mut hold: Hold = ui.ctx().data(|d| d.get_temp(hkey)).unwrap_or_default();
    let pt = resp.interact_pointer_pos().or(resp.hover_pos()).or_else(|| ui.input(|i| i.pointer.latest_pos()));
    let near_line = |pt: Pos2, flip: &Flip| -> Option<usize> {
        if !inner.expand(4.0).contains(pt) {
            return None;
        }
        flip.chops.iter().enumerate().map(|(k, c)| (k, (map.x(c.start as f64) - pt.x).abs())).filter(|&(_, d)| d <= 6.0).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(k, _)| k)
    };
    let hover_line = match hold {
        Hold::Line { k, .. } => Some(k),
        _ => pt.and_then(|pt| near_line(pt, flip)),
    };
    let chop_at = |x: f32, flip: &Flip| {
        let f = map.frame(x);
        flip.chops.iter().enumerate().filter(|(_, c)| f >= c.start as f64 && f < c.end as f64).min_by_key(|(_, c)| c.end - c.start).map(|(k, _)| k)
    };
    let hover_chop = pt.filter(|pt| wave.expand2(vec2(0.0, handle_h + 2.0)).contains(*pt)).and_then(|pt| chop_at(pt.x, flip));
    let (primary, alt, shift) = ui.input(|i| (i.pointer.primary_down(), i.modifiers.alt, i.modifiers.shift));
    let pressed = resp.is_pointer_button_down_on() && primary;
    let mut changed = false;
    let frames = s.frames();
    let fpp = map.len / map.width.max(1.0) as f64;
    match hold {
        Hold::Nothing if pressed => {
            if let Some(k) = pt.and_then(|pt| near_line(pt, flip)) {
                hold = Hold::Line { k, from: flip.chops[k].start, x: pt.map_or(0.0, |p| p.x), off: false };
                p[pf::CHOP] = (k + 1) as f32;
                changed = true;
            } else if let Some(k) = hover_chop {
                let note = pf::FIRST_NOTE + k as u8;
                notes.push((note, Some(0.9)));
                hold = Hold::Note(note);
                p[pf::CHOP] = (k + 1) as f32;
                changed = true;
            }
        }
        Hold::Line { k, from, x, .. } if primary => {
            if let Some(pt) = pt {
                // Fine: a fifth as far; free with Alt, else onto a hit or a zero crossing.
                let to = if shift { from as f64 + (pt.x - x) as f64 * fpp * 0.15 } else { map.frame(pt.x) };
                let to = to.clamp(0.0, frames as f64) as u64;
                let to = if alt { to } else { pf::snap(s, to, (8.0 * fpp) as u64) };
                if flip.chops.get(k).is_some_and(|c| c.start != to) {
                    flip.move_line(k, to, frames);
                    changed = true;
                }
                let off = pt.y < r.top() - 22.0 || pt.y > r.bottom() + 22.0;
                hold = Hold::Line { k, from, x, off };
            }
        }
        Hold::Line { k, off, .. } => {
            if off && flip.delete_line(k) {
                p[pf::CHOP] = (k.max(1)) as f32;
                changed = true;
            }
            hold = Hold::Nothing;
        }
        Hold::Note(note) if !primary => {
            notes.push((note, None));
            hold = Hold::Nothing;
        }
        _ => {}
    }
    // A double-click puts a line there (snapped as a drag is).
    if resp.double_clicked() && hover_line.is_none() {
        if let Some(pt) = resp.interact_pointer_pos() {
            let at = map.frame(pt.x).clamp(0.0, frames as f64) as u64;
            let at = if alt { at } else { pf::snap(s, at, (8.0 * fpp) as u64) };
            if let Some(k) = flip.add_line(at, frames) {
                p[pf::CHOP] = (k + 1) as f32;
                changed = true;
            }
        }
    }
    if resp.secondary_clicked() {
        if let Some(k) = hover_line {
            changed |= flip.delete_line(k);
        } else if let Some(c) = hover_chop.and_then(|k| flip.chops.get_mut(k)) {
            c.reverse = !c.reverse;
            changed = true;
        }
    }
    ui.ctx().data_mut(|d| d.insert_temp(hkey, hold));
    // The lines, with their handles: the chop's number and its key.
    let hfont = FontId::proportional((handle_h * 0.66).clamp(8.0, 12.0));
    let mut order = flip.lines();
    // The chosen and the held over the others.
    order.sort_by_key(|&(k, _)| (k == sel || Some(k) == hover_line) as u8);
    let dragging_off = matches!(hold, Hold::Line { off: true, .. });
    for &(k, start) in &order {
        let x = map.x(start as f64);
        if x < inner.left() - 1.0 || x > inner.right() + 1.0 {
            continue;
        }
        let col = pads[k % 4];
        let hot = Some(k) == hover_line;
        let gone = hot && dragging_off;
        let c = flip.chops[k];
        let line_col = if gone { Color32::from_rgb(0xff, 0x6b, 0x6b) } else if hot { Color32::WHITE } else { col };
        if hot {
            clip.line_segment([pos2(x, wave.top()), pos2(x, wave.bottom())], Stroke::new(5.0_f32, Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 60)));
        }
        clip.line_segment([pos2(x, inner.top() + handle_h), pos2(x, wave.bottom())], Stroke::new(if hot { 2.0_f32 } else { 1.0 }, line_col));
        let note = krabton_engine::note_name(pf::FIRST_NOTE + k.min(pf::MAX_CHOPS - 1) as u8);
        let room = flip.lines().iter().map(|&(_, f)| map.x(f as f64)).filter(|&o| o > x + 1.0).fold(inner.right(), f32::min) - x;
        let arrow = if c.reverse { "\u{2190}" } else { "" };
        let text = if room > 46.0 || hot || k == sel { format!("{}{arrow} {note}", k + 1) } else { format!("{}{arrow}", k + 1) };
        let w = painter.layout_no_wrap(text.clone(), hfont.clone(), Color32::WHITE).size().x + 8.0;
        let tab = Rect::from_min_size(pos2(x - 1.0, inner.top()), vec2(w, handle_h));
        let filled = k == sel || hot;
        clip.rect_filled(tab, egui::CornerRadius { nw: 0, ne: 6, sw: 0, se: 6 }, if filled { line_col } else { Color32::from_rgba_unmultiplied(0x12, 0x0e, 0x34, 210) });
        clip.rect_stroke(tab, egui::CornerRadius { nw: 0, ne: 6, sw: 0, se: 6 }, Stroke::new(1.0_f32, line_col), egui::StrokeKind::Inside);
        clip.text(pos2(tab.left() + 4.0, tab.center().y), Align2::LEFT_CENTER, text, hfont.clone(), if filled { LABEL_INK } else { line_col });
        if gone {
            clip.text(pos2(x, mid), Align2::CENTER_CENTER, "\u{00d7}", FontId::proportional(18.0), line_col);
        }
    }
    // Where it plays.
    let at = live.vals[pf::VIS_AT];
    if at >= 0.0 {
        let x = map.x(at.clamp(0.0, 1.0) as f64 * n);
        if x >= wave.left() && x <= wave.right() {
            clip.line_segment([pos2(x, wave.top()), pos2(x, wave.bottom())], Stroke::new(1.5_f32, Color32::WHITE));
            sparkle(&clip, pos2(x, wave.top() + 3.0), 4.0, Color32::WHITE);
        }
    }
    // How far in it is zoomed, along the bottom.
    let bar = Rect::from_min_max(pos2(inner.left(), inner.bottom() - bar_h), inner.max);
    if view.zoom > 1.0 {
        painter.rect_filled(bar, 2.0, Color32::from_white_alpha(18));
        let a = bar.left() + view.from as f32 * bar.width();
        painter.rect_filled(Rect::from_min_max(pos2(a, bar.top()), pos2(a + bar.width() / view.zoom, bar.bottom())), 2.0, mix(PINK, LILAC, 0.5));
    }
    let tip = match (hover_line, hover_chop) {
        (Some(k), _) => format!("Chop {}'s line: drag to move it (it snaps to the hits and zero crossings; Alt: free, Shift: fine), right-click or drag it off the top or bottom to take it away", k + 1),
        (None, Some(k)) => format!("Chop {} on {}: press to play it and choose it (the keys below C3 play it at their pitch), right-click to turn it round, double-click for a new line here. Ctrl and the wheel zoom, the wheel scrolls", k + 1, krabton_engine::note_name(pf::FIRST_NOTE + k.min(pf::MAX_CHOPS - 1) as u8)),
        _ => "Double-click for a new line; drop a sample here to flip it. Ctrl and the wheel zoom, the wheel scrolls".into(),
    };
    if !matches!(hold, Hold::Line { .. }) {
        resp.on_hover_text(tip);
    }
    changed
}

/// The settings of the chosen chop: faders for its pitch, fine tune, gain,
/// pan, trim, fades and tone, and click for its direction, way of playing,
/// choke group and stretch.
fn chop_panel(ui: &mut Ui, id: Id, r: Rect, p: &mut [f32], sample: Option<&SampleRef>, flip: &mut Flip, live: &InstLive) -> bool {
    glass(ui.painter(), r, "Chop");
    let painter = ui.painter().clone();
    let u = r.height() / 12.0;
    let inner = Rect::from_min_max(r.min + vec2(u * 0.3, u * 0.3), pos2(r.right() - u * 0.3, r.bottom() - u * 1.0));
    let k = chosen(p, flip);
    let Some(c) = flip.chops.get(k).copied() else {
        painter.text(inner.center(), Align2::CENTER_CENTER, "no chops yet", crate::skin::Font::Retro.id(13.0), LILAC);
        return false;
    };
    let col = crate::skin::current().map(|s| s.pads).filter(|p| p.len() >= 4).unwrap_or(&PADS)[k % 4];
    // Its name: number, key, how long, where.
    let head = Rect::from_min_size(inner.min, vec2(inner.width(), u * 1.4));
    let lit = live.vals[k.min(pf::MAX_CHOPS - 1)].clamp(0.0, 1.0);
    painter.circle_filled(pos2(head.left() + u * 0.5, head.center().y), u * 0.32, mix(col, Color32::WHITE, lit * 0.6));
    let title = format!("chop {} · {}", k + 1, krabton_engine::note_name(pf::FIRST_NOTE + k.min(pf::MAX_CHOPS - 1) as u8));
    painter.text(pos2(head.left() + u * 1.1, head.center().y), Align2::LEFT_CENTER, title, crate::skin::Font::Retro.id((u * 0.8).clamp(10.0, 16.0)), col);
    let sr = sample.map_or(44_100.0, |s| s.sample_rate as f64);
    let info = format!("{:.2} s at {:.2} s", (c.end - c.start) as f64 / sr, c.start as f64 / sr);
    painter.text(pos2(head.right(), head.center().y), Align2::RIGHT_CENTER, info, FontId::proportional((u * 0.5).clamp(8.0, 11.0)), crate::skin::ink_dim());
    // The faders.
    let mut changed = false;
    let row = Rect::from_min_max(pos2(inner.left(), head.bottom() + u * 0.2), pos2(inner.right(), inner.bottom() - u * 2.1));
    let n = pf::CHOP_SPECS.len();
    let w = row.width() / n as f32;
    let names = ["Pitch", "Fine", "Gain", "Pan", "Trim In", "Trim Out", "Fade In", "Fade Out", "Tone"];
    let mut chop = c;
    for (i, spec) in pf::CHOP_SPECS.iter().enumerate() {
        let cell = Rect::from_min_size(pos2(row.left() + i as f32 * w, row.top()), vec2(w, row.height())).shrink2(vec2(1.0, 0.0));
        let Some(v) = chop.setting(i) else { continue };
        let shown = krabton_engine::params::ParamSpec { name: names[i], ..*spec };
        let mut val = *v;
        let (ch, _) = param_slider(ui, cell, id.with(("set", i)), &shown, &mut val, accent_now());
        if ch {
            *v = spec.clamp(val);
            changed = true;
        }
    }
    // The switches: click for the next, right-click for the one before.
    let brow = Rect::from_min_max(pos2(inner.left(), row.bottom() + u * 0.3), inner.max);
    let bw = brow.width() / 5.0;
    let cell = |i: f32, span: f32| Rect::from_min_size(pos2(brow.left() + i * bw, brow.top()), vec2(bw * span, brow.height())).shrink(2.0);
    let on = accent_now();
    if button(ui, cell(0.0, 1.0), id.with("rev"), "REVERSE", chop.reverse, on).on_hover_text("Play it backwards").clicked() {
        chop.reverse = !chop.reverse;
        changed = true;
    }
    let pi = Play::ALL.iter().position(|m| *m == chop.play).unwrap_or(0);
    let resp = button(ui, cell(1.0, 1.0), id.with("play"), &chop.play.name().to_uppercase(), chop.play != Play::Flip, on).on_hover_text("How it plays: as the flip's Mode, one-shot (to its end), gate (while held), loop or ping-pong (round, or to its end and back, while held). Click for the next, right-click for the one before");
    if resp.clicked() || resp.secondary_clicked() {
        let d = if resp.clicked() { 1 } else { Play::ALL.len() - 1 };
        chop.play = Play::ALL[(pi + d) % Play::ALL.len()];
        changed = true;
    }
    let choke = if chop.choke == 0 { "CHOKE \u{2013}".to_string() } else { format!("CHOKE {}", chop.choke) };
    let resp = button(ui, cell(2.0, 1.0), id.with("choke"), &choke, chop.choke > 0, on).on_hover_text("Its choke group: chops of a group cut each other (–: as the flip's Choke). Click for the next, right-click for the one before");
    if resp.clicked() {
        chop.choke = (chop.choke + 1) % 5;
        changed = true;
    }
    if resp.secondary_clicked() {
        chop.choke = (chop.choke + 4) % 5;
        changed = true;
    }
    if button(ui, cell(3.0, 1.0), id.with("stretch"), "STRETCH", chop.stretch, on).on_hover_text("Stretched: its pitch moves without its speed (read in grains); off, it plays faster or slower as a record does").clicked() {
        chop.stretch = !chop.stretch;
        changed = true;
    }
    if button(ui, cell(4.0, 1.0), id.with("plain"), "PLAIN", false, on).on_hover_text("Its own sound back to nothing (where it is stays)").clicked() {
        chop = FlipChop { start: chop.start, end: chop.end, manual: chop.manual, ..FlipChop::default() };
        changed = true;
    }
    if changed {
        if let Some(c) = flip.chops.get_mut(k) {
            *c = chop;
        }
    }
    changed
}

/// The buttons under the wave: how to chop it (at its hits, in bars, in
/// equal parts or only by hand), how many, how readily or on what grid,
/// again; then the order shuffled or sorted, the sound thrown about, an
/// auto-flip into a clip, the chops sent elsewhere, load.
#[allow(clippy::too_many_arguments)]
fn chop_buttons(ui: &mut Ui, id: Id, r: Rect, has: bool, flip: &mut Flip, cv: &mut ChopView, load: &mut Option<std::path::PathBuf>, acts: &mut Acts) -> bool {
    let on = accent_now();
    let rows = 2.0;
    let rh = r.height() / rows;
    let cells = 14.0;
    let bw = r.width() / cells;
    let cell = |row: f32, k: f32, w: f32| Rect::from_min_size(pos2(r.left() + k * bw, r.top() + row * rh), vec2(w * bw, rh)).shrink(2.0);
    let mut changed = false;
    let ch = &mut flip.chopping;
    let mut again = None;
    // Row one: chop by, how many, how readily or on what grid, again.
    for (i, by) in ChopBy::ALL.iter().enumerate() {
        let tip = match by {
            ChopBy::Hits => "Chop at its hits (the strongest, as many as the count; the sensitivity says how soft a hit still counts)",
            ChopBy::Bars => "Chop in bars or beats of its tempo (the grid; Auto: bars, halved till there are enough)",
            ChopBy::Equal => "Chop it in equal parts",
            ChopBy::Manual => "Only the lines put by hand (double-click the wave for one)",
        };
        if button(ui, cell(0.0, i as f32 * 1.6, 1.6), id.with(("by", i)), &by.name().to_uppercase(), ch.by == *by, on).on_hover_text(tip).clicked() {
            ch.by = *by;
            again = Some(false);
        }
    }
    let count = format!("{} CHOPS", ch.count);
    let resp = button(ui, cell(0.0, 6.4, 1.8), id.with("count"), &count, false, on).on_hover_text("How many chops (the most, at its hits): click for more, right-click for fewer (4 to 32)");
    const COUNTS: [usize; 7] = [4, 8, 12, 16, 20, 24, 32];
    let ci = COUNTS.iter().position(|c| *c >= ch.count).unwrap_or(2);
    if resp.clicked() {
        ch.count = COUNTS[(ci + 1) % COUNTS.len()];
        again = Some(false);
    }
    if resp.secondary_clicked() {
        ch.count = COUNTS[(ci + COUNTS.len() - 1) % COUNTS.len()];
        again = Some(false);
    }
    match ch.by {
        ChopBy::Hits => {
            // The sensitivity: a small fader across, chopped again when let go.
            let sr = cell(0.0, 8.2, 2.8);
            let spec = krabton_engine::params::ParamSpec::lin("Sensitivity", "Chop", 0.0, 1.0, 0.5, krabton_engine::params::Unit::Percent);
            let resp = ui.interact(sr, id.with("sens"), Sense::click_and_drag());
            if resp.dragged() || resp.clicked() {
                if let Some(pt) = resp.interact_pointer_pos() {
                    ch.sensitivity = ((pt.x - sr.left() - 4.0) / (sr.width() - 8.0)).clamp(0.0, 1.0);
                }
            }
            if resp.double_clicked() {
                ch.sensitivity = spec.default;
            }
            if resp.drag_stopped() || resp.clicked() || resp.double_clicked() {
                again = Some(false);
            }
            let p = ui.painter();
            rect_fill(p, sr, 3.0, crate::skin::switch_off());
            let fill = Rect::from_min_max(sr.min, pos2(sr.left() + 4.0 + ch.sensitivity * (sr.width() - 8.0), sr.bottom()));
            rect_fill(p, fill, 3.0, shade(on, 0.55));
            p.text(sr.center(), Align2::CENTER_CENTER, format!("SENSITIVITY {}", spec.format(ch.sensitivity)), small(), crate::skin::well().1);
            resp.on_hover_text("How soft a hit still counts (drag; let go and it is chopped again; double-click: 50 %)");
        }
        ChopBy::Bars => {
            let gi = Grid::ALL.iter().position(|g| *g == ch.grid).unwrap_or(0);
            let resp = button(ui, cell(0.0, 8.2, 2.8), id.with("grid"), &format!("GRID {}", ch.grid.name().to_uppercase()), false, on).on_hover_text("The grid: Auto (bars, halved till there are enough), a bar, half a bar, a beat, an eighth, a sixteenth. Click for the next, right-click for the one before");
            if resp.clicked() || resp.secondary_clicked() {
                let d = if resp.clicked() { 1 } else { Grid::ALL.len() - 1 };
                ch.grid = Grid::ALL[(gi + d) % Grid::ALL.len()];
                again = Some(false);
            }
        }
        _ => {}
    }
    let resp = button(ui, cell(0.0, 11.0, 1.6), id.with("again"), "RE-CHOP", false, on).on_hover_text("Chop it again: the lines put by hand stay. Right-click: all of it again (the hand's lines go too)");
    if resp.clicked() {
        again = Some(false);
    }
    if resp.secondary_clicked() {
        again = Some(true);
    }
    if button(ui, cell(0.0, 12.6, 1.4), id.with("load"), "LOAD", false, on).on_hover_text("A sample to flip: WAV, FLAC, MP3, AIFF (or drop one here)").clicked() {
        if let Some(path) = crate::import::audio_file_dialog().pick_file() {
            *load = Some(path);
        }
    }
    if has {
        acts.rechop = again;
    }
    // Row two: the order, the sound, an auto-flip, export.
    let seed = ui.input(|i| (i.time * 1000.0) as u32);
    if button(ui, cell(1.0, 0.0, 1.6), id.with("shuffle"), "SHUFFLE", false, on).on_hover_text("The chops on the keys in a new order (undo brings it back)").clicked() {
        flip.shuffle(seed);
        changed = true;
    }
    if button(ui, cell(1.0, 1.6, 1.6), id.with("sort"), "SORT", false, on).on_hover_text("The chops on the keys in the order they are in the sample").clicked() {
        flip.sort();
        changed = true;
    }
    if button(ui, cell(1.0, 3.2, 1.6), id.with("random"), "RANDOM", false, on).on_hover_text("Their sound thrown about: some an octave or a fifth away, some backwards, louder or softer, left or right (undo brings it back)").clicked() {
        flip.randomize(seed);
        changed = true;
    }
    let style = AutoStyle::ALL[cv.style % AutoStyle::ALL.len()];
    let resp = button(ui, cell(1.0, 5.2, 2.0), id.with("style"), &format!("{} \u{25b8}", style.name().to_uppercase()), false, on).on_hover_text("The auto-flip's feel: straight (the chops one after the other), stutter (some played over on sixteenths) or the pluggnb bounce (3, 3 and 2). Click for the next");
    if resp.clicked() {
        cv.style = (cv.style + 1) % AutoStyle::ALL.len();
    }
    if resp.secondary_clicked() {
        cv.style = (cv.style + AutoStyle::ALL.len() - 1) % AutoStyle::ALL.len();
    }
    if button(ui, cell(1.0, 7.2, 2.2), id.with("auto"), "AUTO-FLIP", false, on).on_hover_text("Four bars that play the chops in a new order, in time with the song, as a clip in the track's selected slot (each click a new one)").clicked() {
        if flip.chops.is_empty() {
            acts.said = Some("Flip a sample first: drop one on Flipside".into());
        } else {
            acts.auto = Some(style);
        }
    }
    if button(ui, cell(1.0, 9.6, 2.2), id.with("rack"), "TO DRUM RACK", false, on).on_hover_text("The chops (the first sixteen) as the pads of a Drum Rack on a new track, from C1 up").clicked() {
        acts.export = Some(Export::DrumRack);
    }
    if button(ui, cell(1.0, 11.8, 2.2), id.with("princess"), "TO PRINCESS", false, on).on_hover_text("The chops (the first twelve) on the tracks of a new Analog Princess, as its Sample voice").clicked() {
        acts.export = Some(Export::Princess);
    }
    changed || again.is_some()
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_face_holds_every_parameter() {
        super::super::check(&super::face(), krabton_engine::dsp::princess_flip::SPECS);
    }

    #[test]
    fn the_chop_panel_holds_every_setting() {
        let mut c = krabton_engine::dsp::princess_flip::FlipChop::default();
        for k in 0..krabton_engine::dsp::princess_flip::CHOP_SPECS.len() {
            assert!(c.setting(k).is_some(), "setting {k}");
        }
        assert!(c.setting(krabton_engine::dsp::princess_flip::CHOP_SPECS.len()).is_none());
    }
}
