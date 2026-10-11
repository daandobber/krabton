//! The face of a guitar pedal: its enclosure and name, the LED, and the
//! footswitch that turns it on and off. The Wah has its treadle instead of
//! a name (drag it to rock the pedal) and the Amp is a little head with its
//! grille.

use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::effects::EffectKind;
use krabton_engine::dsp::fx::pedals;

/// Enclosure and lettering colours.
fn colors(kind: EffectKind) -> (Color32, Color32) {
    match kind {
        EffectKind::Overdrive => (Color32::from_rgb(0x2f, 0x9a, 0x5a), Color32::from_rgb(0xf0, 0xf4, 0xe8)),
        EffectKind::DistortionPedal => (Color32::from_rgb(0x1c, 0x1c, 0x1e), Color32::from_rgb(0xf2, 0xf2, 0xf2)),
        EffectKind::Fuzz => (Color32::from_rgb(0xc0, 0x3a, 0x2b), Color32::from_rgb(0xf6, 0xe6, 0xc8)),
        EffectKind::Octaver => (Color32::from_rgb(0x5b, 0x3f, 0xa8), Color32::from_rgb(0xf0, 0xe8, 0xff)),
        EffectKind::Squash => (Color32::from_rgb(0x2a, 0x6c, 0xc4), Color32::from_rgb(0xff, 0xff, 0xff)),
        EffectKind::Wah => (Color32::from_rgb(0x22, 0x22, 0x24), Color32::from_rgb(0xd8, 0xd8, 0xd8)),
        EffectKind::Tremolo => (Color32::from_rgb(0xe8, 0xb8, 0x2a), Color32::from_rgb(0x2a, 0x22, 0x10)),
        EffectKind::Vibe => (Color32::from_rgb(0xe9, 0xe1, 0xcf), Color32::from_rgb(0x4a, 0x2c, 0x60)),
        _ => (Color32::from_rgb(0x26, 0x24, 0x22), Color32::from_rgb(0xe0, 0xc0, 0x70)),
    }
}

/// Draws the pedal. Returns (params changed, footswitch pressed).
pub(crate) fn pedal_face(ui: &mut Ui, id: Id, kind: EffectKind, params: &mut [f32], on: bool, height: f32) -> (bool, bool) {
    let w = 112.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let (body, ink) = colors(kind);
    let mut changed = false;
    let p = ui.painter_at(rect.expand(1.0));
    let r = rect.shrink(2.0);
    rect_fill(&p, r.translate(vec2(0.0, 2.0)), 8.0, Color32::from_black_alpha(80));
    rect_fill(&p, r, 8.0, body);
    rect_line(&p, r, 8.0, 1.0, shade(body, 1.4));
    // The LED.
    let led = pos2(r.center().x, r.top() + 12.0);
    if on {
        p.circle_filled(led, 9.0, Color32::from_rgba_unmultiplied(0xff, 0x30, 0x30, 40));
    }
    p.circle_filled(led, 4.0, if on { Color32::from_rgb(0xff, 0x3a, 0x3a) } else { Color32::from_rgb(0x50, 0x14, 0x14) });
    let foot = pos2(r.center().x, r.bottom() - 22.0);
    let middle = Rect::from_min_max(pos2(r.left() + 6.0, led.y + 10.0), pos2(r.right() - 6.0, foot.y - 18.0));
    match kind {
        EffectKind::Wah => {
            // The treadle, tilted as far as the pedal is pressed.
            let resp = ui.interact(middle, id.with("treadle"), Sense::click_and_drag());
            if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
                let v = (((middle.bottom() - pos.y) / middle.height()).clamp(0.0, 1.0) * 100.0).round() / 100.0;
                if params[pedals::wah::PEDAL] != v {
                    params[pedals::wah::PEDAL] = v;
                    changed = true;
                }
            }
            if resp.hovered() || resp.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }
            resp.on_hover_text("Drag up (toe down) or down (heel down): rock the wah");
            let t = params[pedals::wah::PEDAL].clamp(0.0, 1.0);
            let c = middle.center();
            let (hw, hh) = (middle.width() * 0.42, middle.height() * 0.5);
            // Seen from the side-ish: the toe end rises and narrows.
            let toe = hh * (0.55 + 0.45 * t);
            let heel = hh * (1.0 - 0.35 * t);
            let pts = vec![pos2(c.x - hw * 0.8, c.y - toe), pos2(c.x + hw * 0.8, c.y - toe), pos2(c.x + hw, c.y + heel), pos2(c.x - hw, c.y + heel)];
            p.add(Shape::convex_polygon(pts.clone(), Color32::from_rgb(0x3a, 0x3a, 0x3e), Stroke::new(1.0_f32, Color32::from_rgb(0x60, 0x60, 0x66))));
            for k in 1..6 {
                let f = k as f32 / 6.0;
                let y = c.y - toe + f * (toe + heel);
                let half = hw * (0.8 + 0.2 * f);
                p.line_segment([pos2(c.x - half + 4.0, y), pos2(c.x + half - 4.0, y)], Stroke::new(1.0_f32, Color32::from_rgb(0x2a, 0x2a, 0x2e)));
            }
            text(&p, pos2(c.x, middle.bottom() + 2.0), Align2::CENTER_TOP, "WAH", FontId::proportional(13.0), ink);
        }
        EffectKind::Amp => {
            // A little head: the panel and the grille below it.
            let grille = Rect::from_min_max(pos2(middle.left() + 2.0, middle.center().y - 4.0), pos2(middle.right() - 2.0, middle.bottom() + 10.0));
            rect_fill(&p, grille, 3.0, Color32::from_rgb(0x4a, 0x40, 0x34));
            let mut y = grille.top() + 3.0;
            while y < grille.bottom() - 2.0 {
                p.line_segment([pos2(grille.left() + 3.0, y), pos2(grille.right() - 3.0, y)], Stroke::new(1.0_f32, Color32::from_rgb(0x5e, 0x52, 0x42)));
                y += 3.0;
            }
            let kind_i = (params[pedals::amp::TYPE].round().max(0.0) as usize).min(pedals::amp::TYPES.len() - 1);
            text(&p, pos2(middle.center().x, middle.top() + 2.0), Align2::CENTER_TOP, "AMP", FontId::proportional(18.0), ink);
            text(&p, pos2(middle.center().x, middle.top() + 22.0), Align2::CENTER_TOP, pedals::amp::TYPES[kind_i].to_uppercase(), FontId::monospace(10.0), shade(ink, 0.8));
        }
        _ => {
            let name = kind.name().to_uppercase();
            let size = if name.len() > 8 { 14.0 } else { 17.0 };
            text(&p, middle.center(), Align2::CENTER_CENTER, &name, FontId::proportional(size), ink);
        }
    }
    // The footswitch.
    let fr = Rect::from_center_size(foot, vec2(30.0, 30.0));
    let resp = ui.interact(fr, id.with("foot"), Sense::click());
    let pressed = resp.is_pointer_button_down_on();
    p.circle_filled(foot, 13.0, Color32::from_rgb(0x9a, 0x9c, 0xa2));
    p.circle_filled(foot, 10.0, if pressed { Color32::from_rgb(0xa8, 0xaa, 0xb0) } else { Color32::from_rgb(0xc8, 0xca, 0xd0) });
    p.circle_stroke(foot, 10.0, Stroke::new(1.0_f32, Color32::from_rgb(0x70, 0x72, 0x78)));
    let stomp = resp.on_hover_text("Stomp: turn it on or off").clicked();
    (changed, stomp)
}
