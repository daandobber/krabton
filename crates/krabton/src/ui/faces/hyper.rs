//! Hyper as a desktop synth: the display (the firmware and how far the device
//! has booted, the preset big between arrows, a click for the list by bank),
//! then its oscillators, filter, the two envelopes, the LFOs, the effects
//! and the output.

use super::sf2::lcd_button;
use super::{draw, fit, screen_box, Ctx, Face};
use crate::app::BrowserItem;
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::hyper::{self, FirmwareRef, Status};
use std::path::PathBuf;

pub(crate) fn face() -> Face {
    Face::new(61.2, 12.0)
        .screen(0.3, 0.3, 13.0, 10.6, "display")
        .section(13.6, 0.0, 9.8, 12.0, "Oscillators")
        .knob(13.7, 0.3, 3.1, 3.6, "Shape 1")
        .knob(16.8, 0.3, 3.1, 3.6, "Shape 2")
        .knob(19.9, 0.3, 3.1, 3.6, "Detune")
        .knob(13.7, 3.9, 3.1, 3.6, "Osc Mix")
        .knob(16.8, 3.9, 3.1, 3.6, "Sub")
        .knob(19.9, 3.9, 3.1, 3.6, "Noise")
        .knob(13.7, 7.5, 3.1, 3.6, "FM")
        .section(23.6, 0.0, 6.6, 12.0, "Filter")
        .knob(23.7, 0.3, 3.1, 5.3, "Cutoff")
        .knob(26.8, 0.3, 3.1, 5.3, "Resonance")
        .knob(23.7, 5.8, 3.1, 5.3, "Env Amount")
        .knob(26.8, 5.8, 3.1, 5.3, "Key Track")
        .section(30.4, 0.0, 6.6, 12.0, "Filter Env")
        .named(30.5, 0.3, 3.1, 5.3, "F Attack", super::Ctl::Knob, "Attack")
        .named(33.6, 0.3, 3.1, 5.3, "F Decay", super::Ctl::Knob, "Decay")
        .named(30.5, 5.8, 3.1, 5.3, "F Sustain", super::Ctl::Knob, "Sustain")
        .named(33.6, 5.8, 3.1, 5.3, "F Release", super::Ctl::Knob, "Release")
        .section(37.2, 0.0, 6.6, 12.0, "Amp Env")
        .knob(37.3, 0.3, 3.1, 5.3, "Attack")
        .knob(40.4, 0.3, 3.1, 5.3, "Decay")
        .knob(37.3, 5.8, 3.1, 5.3, "Sustain")
        .knob(40.4, 5.8, 3.1, 5.3, "Release")
        .section(44.0, 0.0, 6.6, 12.0, "LFO")
        .named(44.1, 0.3, 3.1, 5.3, "LFO 1 Rate", super::Ctl::Knob, "Rate 1")
        .knob(47.2, 0.3, 3.1, 5.3, "Vibrato")
        .named(44.1, 5.8, 3.1, 5.3, "LFO 2 Rate", super::Ctl::Knob, "Rate 2")
        .named(47.2, 5.8, 3.1, 5.3, "LFO > Cutoff", super::Ctl::Knob, "> Cutoff")
        .section(50.8, 0.0, 6.6, 12.0, "Effects")
        .knob(50.9, 0.3, 3.1, 5.3, "Chorus")
        .knob(54.0, 0.3, 3.1, 5.3, "Delay")
        .named(50.9, 5.8, 3.1, 5.3, "Delay Time", super::Ctl::Knob, "Time")
        .knob(54.0, 5.8, 3.1, 5.3, "Feedback")
        .section(57.6, 0.0, 3.4, 12.0, "Output")
        .knob(57.7, 0.3, 3.1, 5.3, "Volume")
        .knob(57.7, 5.8, 3.1, 5.3, "Pan")
}

/// What the display asks for.
#[derive(Default)]
pub(crate) struct Asked {
    /// Open a firmware file.
    pub open: bool,
    /// Look through a folder for one.
    pub search: bool,
    /// Play this factory preset (bank, program).
    pub pick: Option<(u16, u16)>,
    /// Something dropped on it from the browser.
    pub dropped: Option<BrowserItem>,
    /// A firmware found on this computer.
    pub firmware: Option<PathBuf>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn panel(ui: &mut Ui, id: Id, fw: Option<&FirmwareRef>, status: &Status, preset: (u16, u16), found: &[PathBuf], params: &mut [f32], live: &InstLive, height: f32, mv: Option<&mut ModView>, asked: &mut Asked) -> bool {
    let face = face();
    let mut cx = Ctx { id, keys: hyper::SPECS, specs: hyper::SPECS, params, mv, presets: None };
    let changed = draw(ui, &face, height, &mut cx, &mut |ui, _, r, _| {
        display(ui, id.with("display"), r, fw, status, preset, found, live, asked);
        false
    });
    if matches!(status, Status::Booting) || live.vals[1] > 0.0005 {
        ui.ctx().request_repaint();
    }
    changed
}

/// The display: the firmware and the device's state, the preset big between
/// the arrows (a click: the list), where it sits, and the button to open a
/// firmware; without one, the firmware files found on this computer.
#[allow(clippy::too_many_arguments)]
pub(crate) fn display(ui: &mut Ui, id: Id, r: Rect, fw: Option<&FirmwareRef>, status: &Status, preset: (u16, u16), found: &[PathBuf], live: &InstLive, asked: &mut Asked) {
    let drop = ui.interact(r, id.with("drop"), Sense::hover());
    if let Some(item) = drop_payload::<BrowserItem>(&drop) {
        asked.dropped = Some((*item).clone());
    }
    let p = ui.painter().clone();
    let u = r.height() / 10.6;
    screen_box(&p, r, u);
    if drop.dnd_hover_payload::<BrowserItem>().is_some() {
        p.rect_stroke(r, 2.0, Stroke::new(1.5_f32, accent_now()), egui::StrokeKind::Inside);
    }
    let ink = accent_now();
    let dim = shade(ink, 0.55);
    let lcd = |size: f32| crate::skin::Font::Lcd.id(size.max(8.0));
    let inner = r.shrink(u * 0.5);

    // The top line: the firmware, and the device.
    let top = inner.top() + u * 0.5;
    let (left, right) = match (fw, status) {
        (None, _) => ("NO FIRMWARE".to_string(), String::new()),
        (Some(f), _) if f.is_missing() => (format!("MISSING: {}", f.name.to_uppercase()), String::new()),
        (Some(f), Status::Booting) => (f.model.to_uppercase(), "BOOTING".to_string()),
        (Some(f), Status::Failed(_)) => (f.model.to_uppercase(), "FAILED".to_string()),
        (Some(f), Status::Ready) => (f.model.to_uppercase(), "READY".to_string()),
        (Some(f), Status::Off) => (f.model.to_uppercase(), String::new()),
    };
    let rw = p.layout_no_wrap(right.clone(), lcd(u * 0.55), dim).size().x + u * 0.5;
    let f = fit(&p, &left, lcd(u * 0.6), inner.width() - rw);
    p.text(pos2(inner.left(), top), Align2::LEFT_CENTER, &left, f, dim);
    let rc = if matches!(status, Status::Booting) && (ui.input(|i| i.time) * 2.0) as i64 % 2 == 0 { ink } else { dim };
    tabular_text(&p, pos2(inner.right(), top), Align2::RIGHT_CENTER, &right, lcd(u * 0.55), rc);

    let row = Rect::from_min_size(pos2(inner.left(), inner.top() + u * 1.3), vec2(inner.width(), u * 3.6));
    match fw.filter(|f| !f.is_missing()) {
        None => {
            // No firmware: what it needs, and the ones found on this computer.
            let msg = if fw.is_some() { "OPEN IT AGAIN" } else { "OPEN A VIRUS FIRMWARE" };
            let head = Rect::from_min_size(row.min, vec2(row.width(), u * 1.8));
            p.text(head.center(), Align2::CENTER_CENTER, msg, fit(&p, msg, lcd(u * 1.0), head.width()), ink);
            let hint = "A VIRUS TI FIRMWARE (.BIN) OR A/B/C OS";
            p.text(pos2(head.center().x, head.bottom() + u * 0.35), Align2::CENTER_CENTER, hint, fit(&p, hint, lcd(u * 0.55), head.width()), dim);
            for (k, path) in found.iter().take(3).enumerate() {
                let b = Rect::from_min_size(pos2(inner.left(), head.bottom() + u * 0.9 + k as f32 * u * 1.9), vec2(inner.width(), u * 1.6));
                let name = path.file_name().map(|n| n.to_string_lossy().to_uppercase()).unwrap_or_default();
                if lcd_button(ui, &p, b, id.with(("found", k)), &name, false, u).on_hover_text(format!("Play with {}", path.display())).clicked() {
                    asked.firmware = Some(path.clone());
                }
            }
        }
        Some(f) => {
            let presets: Vec<(u16, u16)> = f.banks.iter().enumerate().flat_map(|(b, bank)| (0..bank.len()).map(move |p| (b as u16, p as u16))).collect();
            let at = presets.iter().position(|q| *q == preset);
            let aw = u * 1.3;
            let left_r = Rect::from_min_size(row.min, vec2(aw, row.height()));
            let right_r = Rect::from_min_size(pos2(row.right() - aw, row.top()), vec2(aw, row.height()));
            for (k, (b, dir)) in [(left_r, -1i32), (right_r, 1)].into_iter().enumerate() {
                let resp = ui.interact(b, id.with(("step", k)), Sense::click()).on_hover_text(if dir < 0 { "Previous preset" } else { "Next preset" });
                if resp.clicked() && !presets.is_empty() {
                    let n = presets.len() as i32;
                    let i = at.map(|i| (i as i32 + dir).rem_euclid(n)).unwrap_or(0) as usize;
                    asked.pick = Some(presets[i]);
                }
                let c = b.center();
                let s = u * 0.32;
                let pts = if dir < 0 { vec![c + vec2(s, -s * 1.4), c + vec2(-s, 0.0), c + vec2(s, s * 1.4)] } else { vec![c + vec2(-s, -s * 1.4), c + vec2(s, 0.0), c + vec2(-s, s * 1.4)] };
                p.add(Shape::convex_polygon(pts, if resp.hovered() { ink } else { dim }, Stroke::NONE));
            }
            let mid = Rect::from_min_max(pos2(left_r.right() + 2.0, row.top()), pos2(right_r.left() - 2.0, row.bottom()));
            let resp = ui.interact(mid, id.with("name"), Sense::click()).on_hover_text("Click for the factory presets");
            let name = f.preset(preset.0, preset.1).map(|q| q.name.to_uppercase()).unwrap_or_else(|| "PICK A PRESET".into());
            let font = fit(&p, &name, lcd(u * 1.45), mid.width());
            p.text(mid.center(), Align2::CENTER_CENTER, name, font, if resp.hovered() { mix(ink, Color32::WHITE, 0.25) } else { ink });
            egui::Popup::menu(&resp).show(|ui| preset_menu(ui, f, preset, &mut asked.pick));
            let y = row.bottom() + u * 0.75;
            tabular_text(&p, pos2(inner.left(), y), Align2::LEFT_CENTER, &format!("{}  {:03}", FirmwareRef::bank_name(preset.0).to_uppercase(), preset.1 as u32 + 1), lcd(u * 0.6), dim);
            // How loud it plays.
            let meter = Rect::from_min_size(pos2(inner.right() - inner.width() * 0.4, y - u * 0.2), vec2(inner.width() * 0.4, u * 0.4));
            p.rect_filled(meter, 1.0, shade(ink, 0.15));
            let lv = (live.vals[1] * 3.0).clamp(0.0, 1.0);
            p.rect_filled(Rect::from_min_size(meter.min, vec2(meter.width() * lv, meter.height())), 1.0, shade(ink, 0.8));
            if let Status::Failed(e) = status {
                let e = e.to_uppercase();
                p.text(pos2(inner.left(), y + u * 1.0), Align2::LEFT_CENTER, &e, fit(&p, &e, lcd(u * 0.5), inner.width()), ink);
            }
        }
    }

    // Along the bottom: open a firmware, or look for one in a folder.
    let bh = u * 1.7;
    let open_r = Rect::from_min_size(pos2(inner.left(), inner.bottom() - bh), vec2(u * 6.0, bh));
    if lcd_button(ui, &p, open_r, id.with("open"), "FIRMWARE", false, u).on_hover_text("Open a Virus firmware file: a Virus TI's (Access's container, 6 to 9 MB, whatever it is called), or a Virus A, B or C operating system. It stays where it is; Krabton keeps the path").clicked() {
        asked.open = true;
    }
    let find_r = Rect::from_min_size(pos2(open_r.right() + u * 0.4, open_r.top()), vec2(u * 6.0, bh));
    if lcd_button(ui, &p, find_r, id.with("search"), "SEARCH", false, u).on_hover_text("Look through a folder (and the folders in it) for a Virus TI firmware: where Access's Virus TI software was unpacked or installed").clicked() {
        asked.search = true;
    }
}

/// The firmware's factory presets, by bank.
pub(crate) fn preset_menu(ui: &mut Ui, f: &FirmwareRef, at: (u16, u16), pick: &mut Option<(u16, u16)>) {
    if f.banks.iter().all(|b| b.is_empty()) {
        ui.label(egui::RichText::new("No presets").color(TEXT_DIM));
        return;
    }
    for (b, bank) in f.banks.iter().enumerate() {
        if bank.is_empty() {
            continue;
        }
        let title = FirmwareRef::bank_name(b as u16);
        ui.menu_button(if b as u16 == at.0 { format!("{title}  ●") } else { title }, |ui| {
            egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                for (k, pr) in bank.iter().enumerate() {
                    if ui.selectable_label((b as u16, k as u16) == at, format!("{:03}  {}", k + 1, pr.name)).clicked() {
                        *pick = Some((b as u16, k as u16));
                        ui.close();
                    }
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_knob_of_hyper_is_on_its_face() {
        super::super::check(&super::face(), krabton_engine::hyper::SPECS);
    }
}
