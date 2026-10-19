//! The Street Organ as a street organ: its painted front with the pipes
//! lighting up as they speak and the drums at its foot, the registers as
//! stops to pull, then the divisions, the wind and the pipes.

use super::{draw, Ctl, Ctx, Face, Presets};
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use egui::{pos2, vec2, Color32, Id, Rect, Shape, Stroke, Ui};
use krabton_engine::dsp::street_organ;

pub(crate) fn face() -> Face {
    Face::new(42.8, 12.0)
        .screen(0.3, 0.3, 12.0, 10.6, "facade")
        .section(12.6, 0.0, 9.6, 12.0, "Registers")
        .row(12.7, 0.3, 1.88, 10.8, &["Bourdon", "Flute", "Violin", "Celeste", "Piccolo"], Ctl::Fader)
        .section(22.4, 0.0, 6.6, 12.0, "Divisions")
        .knob(22.5, 0.3, 3.1, 3.6, "Bass")
        .knob(22.5, 3.9, 3.1, 3.6, "Accomp")
        .knob(22.5, 7.5, 3.1, 3.6, "Drums")
        .ctl(25.6, 0.3, 3.1, 5.3, "Divisions", Ctl::Switch)
        .knob(25.6, 5.8, 3.1, 5.3, "Transpose")
        .section(29.2, 0.0, 6.6, 12.0, "Wind")
        .knob(29.3, 0.3, 3.1, 3.6, "Pressure")
        .knob(32.4, 0.3, 3.1, 3.6, "Wind")
        .knob(29.3, 3.9, 3.1, 3.6, "Chiff")
        .knob(32.4, 3.9, 3.1, 3.6, "Tremulant")
        .knob(29.3, 7.5, 3.1, 3.6, "Trem Rate")
        .knob(32.4, 7.5, 3.1, 3.6, "Beat")
        .section(36.0, 0.0, 6.6, 12.0, "Pipes")
        .knob(36.1, 0.3, 3.1, 3.6, "Out of Tune")
        .knob(39.2, 0.3, 3.1, 3.6, "Attack")
        .knob(36.1, 3.9, 3.1, 3.6, "Release")
        .knob(39.2, 3.9, 3.1, 3.6, "Space")
        .knob(36.1, 7.5, 3.1, 3.6, "Width")
        .knob(39.2, 7.5, 3.1, 3.6, "Volume")
}

pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut [f32], live: &InstLive, height: f32, mv: Option<&mut ModView>) -> bool {
    let face = face();
    let mut cx = Ctx { id, keys: street_organ::SPECS, specs: street_organ::SPECS, params, mv, presets: Some(Presets { current: preset, names: street_organ::preset_names(), load: street_organ::preset }) };
    let changed = draw(ui, &face, height, &mut cx, &mut |ui, _, rect, _| {
        facade(ui, rect, live);
        false
    });
    if live.vals[0] > 0.0 {
        ui.ctx().request_repaint();
    }
    changed
}

/// How many pipes stand in the front, and the notes they answer for.
const PIPES: usize = 17;
const LOW: f32 = 36.0;
const HIGH: f32 = 96.0;

/// The organ's front: an arched top, three towers of pipes (the tallest in
/// the middle) lighting with the notes they play, and the drums below.
fn facade(ui: &mut Ui, r: Rect, live: &InstLive) {
    let p = ui.painter().clone();
    let u = r.height() / 10.6;
    let gold = Color32::from_rgb(0xe8, 0xc3, 0x5a);
    let paint = Color32::from_rgb(0x2a, 0x10, 0x14);
    let glow = accent_now();
    // The cabinet and its painted arch.
    p.rect_filled(r, 4.0, paint);
    let arch_h = u * 1.4;
    let arch = Rect::from_min_size(r.min, vec2(r.width(), arch_h));
    let mut pts = Vec::new();
    for k in 0..=24 {
        let t = k as f32 / 24.0;
        let x = arch.left() + t * arch.width();
        let y = arch.bottom() - (t * std::f32::consts::PI).sin() * arch_h * 0.85;
        pts.push(pos2(x, y));
    }
    p.add(Shape::line(pts, Stroke::new((u * 0.18).max(1.5), gold)));
    // How loud each pipe is now (from the notes sounding).
    let mut lit = [0.0f32; PIPES];
    for i in 0..(krabton_engine::engine::INST_VALS - 8) / 2 {
        let note = live.vals[8 + 2 * i];
        let level = live.vals[9 + 2 * i];
        if level > 0.001 {
            let k = (((note - LOW) / (HIGH - LOW)).clamp(0.0, 0.999) * PIPES as f32) as usize;
            lit[k] = lit[k].max(level.min(1.0));
        }
    }
    // Pipes from the outside in grow taller towards the middle; low notes left.
    let area = Rect::from_min_max(pos2(r.left() + u * 0.4, arch.bottom() + u * 0.2), pos2(r.right() - u * 0.4, r.bottom() - u * 3.0));
    let pw = area.width() / PIPES as f32;
    let shake = live.vals[1] * 0.06;
    for k in 0..PIPES {
        let mid = (PIPES as f32 - 1.0) / 2.0;
        let d = (k as f32 - mid).abs() / mid;
        let h = area.height() * (1.0 - 0.45 * d) * (1.0 + shake * lit[k]);
        let x = area.left() + k as f32 * pw + pw * 0.15;
        let body = Rect::from_min_size(pos2(x, area.bottom() - h), vec2(pw * 0.7, h));
        let metal = Color32::from_rgb(0xb9, 0xbd, 0xc4);
        let shine = mix(metal, glow, lit[k] * 0.85);
        p.rect_filled(body, pw * 0.2, shine);
        p.rect_filled(Rect::from_min_size(body.min + vec2(body.width() * 0.15, 0.0), vec2(body.width() * 0.18, h)), 0.0, Color32::from_white_alpha(40));
        // The mouth.
        let my = body.bottom() - h * 0.22;
        p.rect_filled(Rect::from_min_size(pos2(body.left() + body.width() * 0.2, my), vec2(body.width() * 0.6, (u * 0.3).max(2.0))), 1.0, Color32::from_black_alpha(170));
        if lit[k] > 0.02 {
            p.circle_filled(pos2(body.center().x, my), pw * 0.9 * lit[k], Color32::from_rgba_unmultiplied(glow.r(), glow.g(), glow.b(), (70.0 * lit[k]) as u8));
        }
    }
    // The foot: a gold rail, then the drums (bass drum with its cymbal, the snare, the woodblock).
    let rail_y = r.bottom() - u * 2.8;
    p.line_segment([pos2(r.left() + u * 0.3, rail_y), pos2(r.right() - u * 0.3, rail_y)], Stroke::new((u * 0.15).max(1.0), gold));
    let cy = r.bottom() - u * 1.35;
    let flash = |v: f32, base: Color32| mix(base, glow, v.clamp(0.0, 1.0));
    let kick = pos2(r.left() + r.width() * 0.3, cy);
    p.circle_filled(kick, u * 1.05, flash(live.vals[2], Color32::from_rgb(0x7a, 0x1d, 0x22)));
    p.circle_stroke(kick, u * 1.05, Stroke::new((u * 0.12).max(1.0), gold));
    p.circle_filled(kick, u * 0.6, flash(live.vals[2], Color32::from_rgb(0xf0, 0xe6, 0xd0)));
    let cym = pos2(kick.x + u * 1.3, cy - u * 0.8);
    p.add(Shape::convex_polygon(vec![cym + vec2(-u * 0.8, 0.0), cym + vec2(0.0, -u * 0.18), cym + vec2(u * 0.8, 0.0), cym + vec2(0.0, u * 0.12)], flash(live.vals[4], Color32::from_rgb(0xc9, 0xa2, 0x3c)), Stroke::NONE));
    let snare = pos2(r.left() + r.width() * 0.66, cy + u * 0.2);
    p.rect_filled(Rect::from_center_size(snare, vec2(u * 1.6, u * 0.9)), 2.0, flash(live.vals[3], Color32::from_rgb(0xd8, 0xd8, 0xd8)));
    p.rect_stroke(Rect::from_center_size(snare, vec2(u * 1.6, u * 0.9)), 2.0, Stroke::new(1.0_f32, gold), egui::StrokeKind::Inside);
    let wood = pos2(r.left() + r.width() * 0.86, cy + u * 0.3);
    p.rect_filled(Rect::from_center_size(wood, vec2(u * 0.9, u * 0.5)), 2.0, flash(live.vals[5], Color32::from_rgb(0x9a, 0x5a, 0x2a)));
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_knob_of_the_street_organ_is_on_its_face() {
        super::super::check(&super::face(), krabton_engine::dsp::street_organ::SPECS);
    }
}
