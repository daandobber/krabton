//! Princess Flip in Analog Princess's colours: a screen with the sample's
//! wave and its chops numbered over it (click one to play it and choose it
//! for the keys below, right-click it to turn it round), the key and tempo
//! heard in it and how far the chops move into the song's key, the buttons
//! that chop it again (at its hits or in bars, 8, 12 or 16 chops), then the
//! flip's own sliders: pitch, slowed, half-time, the chops' mode, stutter,
//! the tape's wobble, low pass and hiss, the fry, the room, the volume. A
//! sample dropped on it is flipped.

use super::{draw, Ctl, Ctx, Face, Presets};
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Stroke, Ui};
use krabton_engine::dsp::princess_flip::{self as pf, ChopBy, Flip};
use krabton_engine::sample::SampleRef;

pub(crate) fn face() -> Face {
    Face::new(64.0, 12.0)
        .screened(&["Chop"])
        .section(0.0, 0.0, 24.0, 12.0, "Princess Flip")
        .preset(0.3, 0.3, 23.4, 1.3)
        .screen(0.3, 1.8, 23.4, 6.6, "wave")
        .screen(0.3, 8.6, 23.4, 2.5, "chopctl")
        .section(24.2, 0.0, 10.0, 12.0, "Flip")
        .named(24.4, 0.3, 2.4, 10.8, "Pitch", Ctl::Fader, "Pitch")
        .named(26.9, 0.3, 7.1, 3.5, "Slowed + Reverb", Ctl::Switch, "Slowed")
        .named(26.9, 3.9, 3.5, 3.6, "Song Key", Ctl::Switch, "Key")
        .named(30.5, 3.9, 3.5, 3.6, "Half-Time", Ctl::Switch, "Half")
        .named(26.9, 7.6, 7.1, 3.5, "Mode", Ctl::Buttons, "Mode")
        .section(34.4, 0.0, 11.0, 12.0, "Chops")
        .named(34.6, 0.3, 3.6, 10.8, "Stutter", Ctl::Buttons, "Stutter")
        .named(38.3, 0.3, 2.3, 10.8, "Attack", Ctl::Fader, "Att")
        .named(40.6, 0.3, 2.3, 10.8, "Release", Ctl::Fader, "Rel")
        .named(42.9, 0.3, 2.3, 10.8, "Choke", Ctl::Switch, "Choke")
        .section(45.6, 0.0, 7.6, 12.0, "Tape")
        .named(45.8, 0.3, 2.4, 10.8, "Wobble", Ctl::Fader, "Wobble")
        .named(48.2, 0.3, 2.5, 10.8, "Low Pass", Ctl::Fader, "LP")
        .named(50.7, 0.3, 2.4, 10.8, "Hiss", Ctl::Fader, "Hiss")
        .section(53.4, 0.0, 10.6, 12.0, "Fry")
        .named(53.6, 0.3, 3.4, 10.8, "Fry", Ctl::Fader, "Fry")
        .named(57.1, 0.3, 3.4, 10.8, "Reverb", Ctl::Fader, "Room")
        .named(60.5, 0.3, 3.3, 10.8, "Volume", Ctl::Fader, "Vol")
}

/// The chops' colours without Analog Princess's look.
const PADS: [Color32; 4] = [Color32::from_rgb(0xff, 0x4f, 0xb0), Color32::from_rgb(0xb0, 0x4f, 0xff), Color32::from_rgb(0x4f, 0xe0, 0xff), Color32::from_rgb(0xff, 0xe1, 0x4f)];

/// What the face does this frame: notes (None: let go), a message.
pub(crate) struct Played {
    pub notes: Vec<(u8, Option<f32>)>,
    pub said: Option<String>,
}

/// How the face chops a sample next (kept between frames).
#[derive(Clone, Copy)]
struct ChopView {
    bars: bool,
    count: usize,
}

impl Default for ChopView {
    fn default() -> Self {
        Self { bars: false, count: 12 }
    }
}

/// A sample flipped: listened to and chopped as the face says.
pub(crate) fn flip_onto(sample: &mut Option<SampleRef>, flip: &mut Flip, s: SampleRef, bars: bool, count: usize) {
    *flip = pf::listen(&s, if bars { ChopBy::Bars } else { ChopBy::Hits }, count);
    *sample = Some(s);
}

/// Princess Flip's panel.
#[allow(clippy::too_many_arguments)]
pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut Vec<f32>, sample: &mut Option<SampleRef>, flip: &mut Flip, live: &InstLive, height: f32, mv: Option<&mut ModView>) -> Played {
    let mut out = Played { notes: Vec::new(), said: None };
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
    let mut rechop = false;
    let resp = {
        let (smp, fl) = (&*sample, &mut *flip);
        let mut cx = Ctx { id, keys: pf::SPECS, specs: pf::SPECS, params, mv, presets: Some(Presets { current: preset, names: pf::preset_names(), load: pf::preset }) };
        ui.scope(|ui| {
            draw(ui, &face, h, &mut cx, &mut |ui, which, rect, p| match which {
                "wave" => wave(ui, id.with("wave"), rect, p, smp.as_ref(), fl, live, &mut notes),
                "chopctl" => chop_buttons(ui, id.with("chopctl"), rect, p, smp.is_some(), fl, &mut cv, live, &mut load, &mut rechop),
                _ => false,
            });
        })
        .response
    };
    if rechop {
        if let Some(s) = sample.clone() {
            flip_onto(sample, flip, s, cv.bars, cv.count);
        }
    }
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
            Ok(s) => flip_onto(sample, flip, s, cv.bars, cv.count),
            Err(e) => said = Some(format!("Could not load the sample: {e}")),
        }
    }
    ui.ctx().data_mut(|d| d.insert_temp(vid, cv));
    if live.vals[18] > 0.0 || live.vals[..pf::MAX_CHOPS].iter().any(|v| *v > 0.01) {
        ui.ctx().request_repaint();
    }
    out.notes = notes;
    out.said = said;
    out
}

/// Where chop `k` is drawn, in the wave's rect.
fn chop_x(r: Rect, flip_len: f64, frame: u64) -> f32 {
    r.left() + (frame as f64 / flip_len.max(1.0)) as f32 * r.width()
}

/// The wave and its chops: press a chop to play it (and choose it for the
/// keys below), right-click to turn it round.
#[allow(clippy::too_many_arguments)]
fn wave(ui: &mut Ui, id: Id, r: Rect, p: &mut [f32], sample: Option<&SampleRef>, flip: &mut Flip, live: &InstLive, notes: &mut Vec<(u8, Option<f32>)>) -> bool {
    let painter = ui.painter().clone();
    super::screen_box(&painter, r, 12.0);
    let accent = accent_now();
    let pads = crate::skin::current().map(|s| s.pads).filter(|p| p.len() >= 4).unwrap_or(&PADS);
    let Some(s) = sample else {
        let f = FontId::proportional((r.height() * 0.11).clamp(10.0, 16.0));
        painter.text(r.center(), Align2::CENTER_CENTER, "DROP A SAMPLE TO FLIP IT", f, shade(accent, 0.8));
        painter.text(r.center() + vec2(0.0, r.height() * 0.16), Align2::CENTER_CENTER, "soul, R&B, anime, a game, a video's sound: its key and tempo are heard, it is chopped over the keys from C3", FontId::proportional((r.height() * 0.065).clamp(8.0, 11.0)), crate::skin::ink_dim());
        return false;
    };
    let n = s.frames() as f64;
    let inner = r.shrink(4.0);
    let wave = Rect::from_min_max(pos2(inner.left(), inner.top() + inner.height() * 0.16), inner.max);
    let mid = wave.center().y;
    // The chops' bands under the wave, every other one a little lighter.
    let sel = (p[pf::CHOP].round().max(1.0) as usize - 1).min(flip.chops.len().saturating_sub(1));
    for (k, c) in flip.chops.iter().enumerate() {
        let (x0, x1) = (chop_x(wave, n, c.start), chop_x(wave, n, c.end));
        let band = Rect::from_min_max(pos2(x0, wave.top()), pos2(x1.max(x0 + 1.0), wave.bottom()));
        let col = pads[k % 4];
        let lit = live.vals[k.min(pf::MAX_CHOPS - 1)].clamp(0.0, 1.0);
        painter.rect_filled(band, 0.0, Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), (14.0 + 26.0 * (k % 2) as f32 + 90.0 * lit) as u8));
        if k == sel {
            painter.rect_stroke(band, 1.0, Stroke::new(1.5_f32, accent), egui::StrokeKind::Inside);
        }
    }
    // The wave, from its overview.
    let peaks = &s.peaks;
    if !peaks.is_empty() {
        let cols = wave.width().max(1.0) as usize;
        let scale = s.peak().max(1e-4).recip().min(4.0);
        for c in 0..cols {
            let a = c * peaks.len() / cols;
            let b = ((c + 1) * peaks.len() / cols).max(a + 1).min(peaks.len());
            let (lo, hi) = peaks[a..b].iter().fold((0.0f32, 0.0f32), |(l, h), &(mn, mx)| (l.min(mn), h.max(mx)));
            let x = wave.left() + c as f32 + 0.5;
            painter.line_segment([pos2(x, mid - hi * scale * wave.height() * 0.46), pos2(x, mid - lo * scale * wave.height() * 0.46)], Stroke::new(1.0_f32, mix(accent, Color32::WHITE, 0.25)));
        }
    }
    // The chops' marks and numbers (an arrow for one turned round).
    let f = FontId::proportional((inner.height() * 0.11).clamp(8.0, 13.0));
    for (k, c) in flip.chops.iter().enumerate() {
        let x0 = chop_x(wave, n, c.start);
        painter.line_segment([pos2(x0, inner.top()), pos2(x0, wave.bottom())], Stroke::new(1.0_f32, pads[k % 4]));
        let label = if c.reverse { format!("{}\u{2190}", k + 1) } else { format!("{}", k + 1) };
        painter.text(pos2(x0 + 2.0, inner.top()), Align2::LEFT_TOP, label, f.clone(), if k == sel { Color32::WHITE } else { pads[k % 4] });
    }
    // Where it plays.
    let at = live.vals[16];
    if at >= 0.0 {
        let x = wave.left() + at.clamp(0.0, 1.0) * wave.width();
        painter.line_segment([pos2(x, wave.top()), pos2(x, wave.bottom())], Stroke::new(1.5_f32, Color32::WHITE));
    }
    // Pressing a chop plays it.
    let resp = ui.interact(r, id, Sense::click_and_drag());
    let key = id.with("down");
    let was: Option<u8> = ui.ctx().memory(|m| m.data.get_temp::<Option<u8>>(key)).flatten();
    let down = resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down());
    let hover_chop = resp.hover_pos().or(resp.interact_pointer_pos()).and_then(|pt| flip.chops.iter().position(|c| pt.x >= chop_x(wave, n, c.start) && pt.x < chop_x(wave, n, c.end)));
    let mut changed = false;
    match (down, was) {
        (true, None) => {
            if let Some(k) = hover_chop {
                let note = pf::FIRST_NOTE + k as u8;
                notes.push((note, Some(0.9)));
                ui.ctx().memory_mut(|m| m.data.insert_temp(key, Some(note)));
                p[pf::CHOP] = (k + 1) as f32;
                changed = true;
            }
        }
        (false, Some(note)) => {
            notes.push((note, None));
            ui.ctx().memory_mut(|m| m.data.insert_temp::<Option<u8>>(key, None));
        }
        _ => {}
    }
    if resp.secondary_clicked() {
        if let Some(c) = hover_chop.and_then(|k| flip.chops.get_mut(k)) {
            c.reverse = !c.reverse;
            changed = true;
        }
    }
    resp.on_hover_text(match hover_chop {
        Some(k) => format!("Chop {} on {}: press to play it (it becomes the chop the keys below C3 play), right-click to turn it round", k + 1, krabton_engine::note_name(pf::FIRST_NOTE + k as u8)),
        None => "Drop a sample here to flip it".into(),
    });
    changed
}

/// The buttons under the wave: chop at the hits or in bars, how many, the
/// chosen chop backwards, load; and what was heard in the sample.
#[allow(clippy::too_many_arguments)]
fn chop_buttons(ui: &mut Ui, id: Id, r: Rect, p: &mut [f32], has: bool, flip: &mut Flip, cv: &mut ChopView, live: &InstLive, load: &mut Option<std::path::PathBuf>, rechop: &mut bool) -> bool {
    let on = accent_now();
    let cells = 7.0;
    let bw = r.width() * 0.62 / cells;
    let cell = |k: f32, w: f32| Rect::from_min_size(pos2(r.left() + k * bw, r.top()), vec2(w * bw, r.height() * 0.55)).shrink(2.0);
    let mut changed = false;
    if button(ui, cell(0.0, 1.0), id.with("hits"), "HITS", !cv.bars, on).on_hover_text("Chop at its hits (the strongest)").clicked() {
        cv.bars = false;
        *rechop = true;
    }
    if button(ui, cell(1.0, 1.0), id.with("bars"), "BARS", cv.bars, on).on_hover_text("Chop in bars of its tempo (halves or beats of them for a short sample)").clicked() {
        cv.bars = true;
        *rechop = true;
    }
    let count = format!("{} CHOPS", cv.count);
    let resp = button(ui, cell(2.0, 1.5), id.with("count"), &count, false, on).on_hover_text("How many chops: click for more, right-click for fewer (8, 12 or 16)");
    if resp.clicked() {
        cv.count = match cv.count {
            8 => 12,
            12 => 16,
            _ => 8,
        };
        *rechop = true;
    }
    if resp.secondary_clicked() {
        cv.count = match cv.count {
            16 => 12,
            12 => 8,
            _ => 16,
        };
        *rechop = true;
    }
    let sel = (p[pf::CHOP].round().max(1.0) as usize - 1).min(flip.chops.len().saturating_sub(1));
    let rev = flip.chops.get(sel).is_some_and(|c| c.reverse);
    if button(ui, cell(3.5, 1.5), id.with("rev"), "REVERSE", rev, on).on_hover_text("Play the chosen chop backwards (or right-click it on the wave)").clicked() {
        if let Some(c) = flip.chops.get_mut(sel) {
            c.reverse = !c.reverse;
            changed = true;
        }
    }
    if button(ui, cell(5.0, 2.0), id.with("load"), "LOAD", false, on).on_hover_text("A sample to flip: WAV, FLAC, MP3, AIFF (or drop one here)").clicked() {
        if let Some(path) = crate::import::audio_file_dialog().pick_file() {
            *load = Some(path);
        }
    }
    if !has {
        *rechop = false;
    }
    // What was heard.
    let painter = ui.painter().clone();
    let info = Rect::from_min_max(pos2(r.left() + r.width() * 0.63, r.top()), pos2(r.right(), r.top() + r.height() * 0.55)).shrink(2.0);
    super::screen_box(&painter, info, 12.0);
    let shift = live.vals[17].round() as i32;
    let key = flip.key_name().unwrap_or_else(|| "no key".into());
    let bpm = flip.bpm.map_or("no tempo".to_string(), |b| format!("{b:.1} BPM"));
    let text = if p[pf::SONG_KEY] >= 0.5 && flip.key.is_some() { format!("{} · {bpm} · {shift:+} st", key.to_uppercase()) } else { format!("{} · {bpm}", key.to_uppercase()) };
    let f = super::fit(&painter, &text, crate::skin::Font::Lcd.id((info.height() * 0.5).clamp(8.0, 14.0)), info.width() - 6.0);
    painter.text(info.center(), Align2::CENTER_CENTER, text, f, on);
    // Below: where the chops are on the keys.
    let line = Rect::from_min_max(pos2(r.left(), r.top() + r.height() * 0.6), r.max);
    let chops = flip.chops.len();
    let text = if chops == 0 {
        "Chops go on the keys from C3 up; the keys below play the chosen chop at their pitch".to_string()
    } else {
        format!("{chops} chops on C3 to {} · C1 to B2 play chop {} at their pitch (C2 as it is)", krabton_engine::note_name(pf::FIRST_NOTE + chops as u8 - 1), sel + 1)
    };
    let f = super::fit(&painter, &text, FontId::proportional((line.height() * 0.6).clamp(8.0, 12.0)), line.width());
    painter.text(line.left_center(), Align2::LEFT_CENTER, text, f, crate::skin::ink_dim());
    changed
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_face_holds_every_parameter() {
        super::super::check(&super::face(), krabton_engine::dsp::princess_flip::SPECS);
    }
}
