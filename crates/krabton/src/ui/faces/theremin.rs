//! The Theremin as a cabinet with its field in the middle: play it there
//! with the mouse or a finger, the pitch rod on the right (nearer is
//! higher, the notes of the song's key marked across) and the volume loop
//! below (nearer is softer). Around it, in sliders: how it plays (octave,
//! range, glide, snap), its tone (with a screen of the wave), the player's
//! hand (vibrato and wobble) and the volume's swell.

use super::{draw, screen_box, Ctl, Ctx, Face, Presets};
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::theremin;

/// What the hand on the field did this frame.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Hand {
    /// Over the field: its pitch (MIDI) and its height (0..1).
    At(f32, f32),
    /// Taken away.
    Gone,
}

pub(crate) fn face() -> Face {
    Face::new(59.1, 12.0)
        .section(0.0, 0.0, 10.0, 12.0, "Play")
        .preset(0.3, 0.4, 9.4, 1.4)
        .named(0.3, 2.1, 2.5, 9.0, "Octave", Ctl::Buttons, "Octave")
        .named(2.9, 2.1, 2.5, 5.4, "Range", Ctl::Buttons, "Octaves")
        .named(5.6, 2.0, 2.1, 9.2, "Glide", Ctl::Fader, "Glide")
        .named(7.7, 2.0, 2.1, 9.2, "Snap", Ctl::Fader, "Snap")
        .screen(10.2, 0.2, 21.0, 11.6, "field")
        .section(31.4, 0.0, 7.6, 12.0, "Tone")
        .screen(31.6, 0.4, 7.2, 3.6, "wave")
        .named(31.6, 4.3, 2.4, 6.8, "Waveform", Ctl::Fader, "Wave")
        .named(34.0, 4.3, 2.4, 6.8, "Brightness", Ctl::Fader, "Bright")
        .named(36.4, 4.3, 2.4, 6.8, "Formant", Ctl::Fader, "Formant")
        .section(39.2, 0.0, 10.9, 12.0, "Hand")
        .named(39.4, 0.3, 2.1, 10.8, "Vibrato", Ctl::Fader, "Vibrato")
        .named(41.5, 0.3, 2.1, 10.8, "Rate", Ctl::Fader, "Rate")
        .named(43.6, 0.3, 2.1, 10.8, "Delay", Ctl::Fader, "Delay")
        .named(45.7, 0.3, 2.1, 10.8, "Irregular", Ctl::Fader, "Loose")
        .named(47.8, 0.3, 2.1, 10.8, "Wobble", Ctl::Fader, "Wobble")
        .section(50.3, 0.0, 8.8, 12.0, "Volume")
        .named(50.5, 0.3, 2.1, 10.8, "Attack", Ctl::Fader, "Swell")
        .named(52.6, 0.3, 2.1, 10.8, "Release", Ctl::Fader, "Fade")
        .named(54.7, 0.3, 2.1, 10.8, "Velocity", Ctl::Fader, "Vel")
        .named(56.8, 0.3, 2.1, 10.8, "Volume", Ctl::Fader, "Vol")
}

/// The Theremin's face, its field played and its screens live. `key` is the
/// song's key (a bit per pitch class) and its root. Returns whether a value
/// changed and what the hand did.
#[allow(clippy::too_many_arguments)]
pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut [f32], live: &InstLive, key: (u16, u8), height: f32, mv: Option<&mut ModView>) -> (bool, Option<Hand>) {
    let face = face();
    let mut hand = None;
    let mut cx = Ctx { id, keys: theremin::SPECS, specs: theremin::SPECS, params, mv, presets: Some(Presets { current: preset, names: theremin::preset_names(), load: theremin::preset }) };
    let changed = draw(ui, &face, height, &mut cx, &mut |ui, which, rect, p| {
        match which {
            "field" => hand = field(ui, rect, id.with("field"), p, live, key),
            _ => wave(ui, rect, p),
        }
        false
    });
    (changed, hand)
}

/// The lowest and highest pitch across the field (MIDI): from the C of the
/// Octave (at 0 the C below middle C), Range octaves up.
pub(crate) fn span(params: &[f32]) -> (f32, f32) {
    let get = |i: usize, d: f32| params.get(i).copied().unwrap_or(d);
    let low = 48.0 + 12.0 * get(theremin::OCTAVE, 0.0).round();
    (low, low + 12.0 * get(theremin::RANGE, 3.0).round().clamp(1.0, 6.0))
}

/// The field to play: X is the pitch (the pitch rod on the right), Y the
/// height over the volume loop (below). Press and move to play; let go and
/// it fades. Draws the notes of the key, the hand, and what is heard.
pub(crate) fn field(ui: &mut Ui, r: Rect, id: Id, params: &[f32], live: &InstLive, key: (u16, u8)) -> Option<Hand> {
    let painter = ui.painter().clone();
    let u = (r.height() / 11.6).max(8.0);
    screen_box(&painter, r, u);
    let glow = accent_now();
    // The antennas take the right and bottom edges; the hand plays in between.
    let rod_w = (u * 0.45).clamp(4.0, 9.0);
    let play = Rect::from_min_max(pos2(r.left() + u * 0.5, r.top() + u * 1.5), pos2(r.right() - rod_w - u * 1.2, r.bottom() - u * 1.6));
    let (low, high) = span(params);
    let x_of = |pitch: f32| play.left() + (pitch - low) / (high - low) * play.width();
    let y_of = |height: f32| play.bottom() - height.clamp(0.0, 1.0) * play.height();
    // Louder upwards: the field brightens away from the loop.
    for k in 0..12 {
        let f = k as f32 / 12.0;
        let band = Rect::from_min_max(pos2(play.left(), y_of(f + 1.0 / 12.0)), pos2(play.right(), y_of(f)));
        painter.rect_filled(band, 0.0, Color32::from_rgba_unmultiplied(glow.r(), glow.g(), glow.b(), (f * f * 18.0) as u8));
    }
    // The notes of the key, the roots stronger; named along the top where
    // there is room, the roots first.
    let (mask, root) = key;
    let font = FontId::proportional((u * 0.62).clamp(8.0, 11.0));
    let notes: Vec<(f32, u8, bool)> = (low as i32..=high as i32)
        .filter(|n| mask >> n.rem_euclid(12) & 1 == 1)
        .map(|n| (x_of(n as f32), n.clamp(0, 127) as u8, n.rem_euclid(12) as u8 == root % 12))
        .collect();
    let mut taken: Vec<(f32, f32)> = Vec::new();
    for pass in [true, false] {
        for &(x, n, is_root) in notes.iter().filter(|nt| nt.2 == pass) {
            let line = Color32::from_white_alpha(if is_root { 70 } else { 28 });
            painter.line_segment([pos2(x, play.top()), pos2(x, play.bottom())], Stroke::new(1.0_f32, line));
            let name = krabton_engine::note_name(n);
            let half = painter.layout_no_wrap(name.clone(), font.clone(), Color32::WHITE).size().x * 0.5 + 2.0;
            if taken.iter().any(|&(a, b)| x + half > a && x - half < b) {
                continue;
            }
            taken.push((x - half, x + half));
            let c = if is_root { Color32::from_white_alpha(170) } else { Color32::from_white_alpha(80) };
            painter.text(pos2(x, play.top() - 2.0), Align2::CENTER_BOTTOM, name, font.clone(), c);
        }
    }
    // The pitch rod, standing on the right.
    let rod = Rect::from_min_max(pos2(r.right() - rod_w - u * 0.5, r.top() + u * 0.5), pos2(r.right() - u * 0.5, r.bottom() - u * 0.4));
    let steel = Color32::from_rgb(0x9a, 0xa1, 0xa8);
    painter.rect_filled(rod, rod_w * 0.5, steel);
    painter.rect_filled(Rect::from_min_size(rod.min + vec2(rod_w * 0.2, 2.0), vec2(rod_w * 0.25, rod.height() - 4.0)), rod_w * 0.2, Color32::from_rgb(0xe8, 0xec, 0xf0));
    // The volume loop, lying along the bottom on the left.
    let loop_c = pos2(play.left() + play.width() * 0.16, r.bottom() - u * 0.75);
    let (rx, ry) = (play.width() * 0.14, u * 0.42);
    let ring: Vec<_> = (0..=48).map(|k| k as f32 / 48.0 * std::f32::consts::TAU).map(|a| pos2(loop_c.x + rx * a.cos(), loop_c.y + ry * a.sin())).collect();
    painter.add(Shape::line(ring, Stroke::new((u * 0.22).clamp(1.5, 3.0), steel)));
    painter.line_segment([pos2(r.left() + 2.0, loop_c.y), pos2(loop_c.x - rx, loop_c.y)], Stroke::new((u * 0.22).clamp(1.5, 3.0), steel));

    // The hand.
    let resp = ui.interact(play.expand2(vec2(u * 0.5, u * 0.5)), id, Sense::click_and_drag());
    let held = resp.is_pointer_button_down_on();
    let was = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(false));
    ui.data_mut(|d| d.insert_temp(id, held));
    let mut out = None;
    let mut at = None;
    if held {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            let fx = ((pos.x - play.left()) / play.width()).clamp(0.0, 1.0);
            let fy = ((play.bottom() - pos.y) / play.height()).clamp(0.0, 1.0);
            let pitch = low + fx * (high - low);
            out = Some(Hand::At(pitch, fy));
            at = Some(pos2(x_of(pitch), y_of(fy)));
        }
        // Sent again every frame while held, so it never sticks.
        ui.ctx().request_repaint();
    } else if was {
        out = Some(Hand::Gone);
    }
    if let Some(h) = at {
        // Its reach to the rod and down to the loop.
        let faint = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(glow.r(), glow.g(), glow.b(), 70));
        painter.line_segment([h, pos2(rod.left(), h.y)], faint);
        painter.line_segment([h, pos2(h.x, play.bottom())], faint);
        for (k, a) in [(3.0, 25u8), (2.0, 45), (1.4, 80)] {
            painter.circle_filled(h, u * 0.5 * k, Color32::from_rgba_unmultiplied(glow.r(), glow.g(), glow.b(), a));
        }
        painter.circle_stroke(h, u * 0.5, Stroke::new(1.5_f32, Color32::WHITE));
    }
    // What is heard: the pitch (snapped, with its vibrato) and how loud.
    let sounding = live.vals[0] > 0.0;
    if sounding {
        let (heard, level) = (live.vals[1], live.vals[2].clamp(0.0, 1.0));
        let x = x_of(heard).clamp(play.left(), play.right());
        let y = y_of(level.sqrt());
        painter.line_segment([pos2(x, play.top()), pos2(x, play.bottom())], Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(glow.r(), glow.g(), glow.b(), (60.0 + 160.0 * level) as u8)));
        painter.circle_filled(pos2(x, y), u * 0.32, glow);
        // The note it is nearest, and how far off.
        let near = heard.round();
        let cents = ((heard - near) * 100.0).round() as i32;
        let text = format!("{}  {cents:+} ct", krabton_engine::note_name(near.clamp(0.0, 127.0) as u8));
        painter.text(pos2(play.right(), r.bottom() - u * 0.8), Align2::RIGHT_CENTER, text, FontId::monospace((u * 0.75).clamp(9.0, 13.0)), Color32::from_white_alpha(200));
        ui.ctx().request_repaint();
    } else {
        painter.text(pos2(play.right(), r.bottom() - u * 0.8), Align2::RIGHT_CENTER, "press and move to play", FontId::proportional((u * 0.7).clamp(9.0, 12.0)), Color32::from_white_alpha(110));
    }
    out
}

/// The wave as Waveform makes it: from a sine to the leaning, reedy beat.
fn wave(ui: &mut Ui, r: Rect, p: &[f32]) {
    let painter = ui.painter().clone();
    let u = r.height() / 3.6;
    screen_box(&painter, r, u);
    let inner = r.shrink(u * 0.4);
    let w = p.get(theremin::WAVEFORM).copied().unwrap_or(0.0);
    let pts: Vec<_> = (0..=96)
        .map(|k| {
            let f = k as f32 / 96.0;
            pos2(inner.left() + f * inner.width(), inner.center().y - theremin::wave_at(w, (f * 2.0).fract()) * inner.height() * 0.45)
        })
        .collect();
    painter.add(Shape::line(pts, Stroke::new(1.5_f32, accent_now())));
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_control_of_the_theremin_is_on_its_face() {
        super::super::check(&super::face(), krabton_engine::dsp::theremin::SPECS);
    }

    #[test]
    fn the_field_spans_its_range_from_the_octaves_c() {
        let mut p = krabton_engine::dsp::theremin::preset("Classic");
        assert_eq!(super::span(&p), (48.0, 84.0));
        (p[krabton_engine::dsp::theremin::OCTAVE], p[krabton_engine::dsp::theremin::RANGE]) = (-1.0, 2.0);
        assert_eq!(super::span(&p), (36.0, 60.0));
    }
}
