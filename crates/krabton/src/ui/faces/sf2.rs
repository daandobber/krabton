//! The SF2 Player as a sound module: the bank's display with its preset
//! (step through them, or pick one from the list, a General MIDI bank's by
//! family), beside the channel's knobs and the playing's.

use super::{draw, fit, screen_box, Ctx, Face};
use crate::app::BrowserItem;
use crate::theme::*;
use crate::ui::modulation::ModView;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, RichText, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::sf2::{self, Sf2Preset, Sf2Ref};

/// A free General MIDI bank to get (its author asks for links to his page).
pub(crate) const GM_BANK_URL: &str = "https://schristiancollins.com/generaluser.php";

pub(crate) fn face() -> Face {
    Face::new(31.2, 12.0)
        .screen(0.3, 0.3, 14.4, 10.6, "bank")
        .section(15.0, 0.0, 8.0, 12.0, "Channel")
        .knob(15.1, 0.3, 3.9, 5.3, "Volume")
        .knob(19.0, 0.3, 3.9, 5.3, "Pan")
        .knob(15.1, 5.8, 3.9, 5.3, "Reverb")
        .knob(19.0, 5.8, 3.9, 5.3, "Chorus")
        .section(23.2, 0.0, 8.0, 12.0, "Play")
        .knob(23.3, 0.3, 3.9, 5.3, "Vibrato")
        .knob(27.2, 0.3, 3.9, 5.3, "Velocity")
        .knob(23.3, 5.8, 3.9, 5.3, "Bend Range")
        .knob(27.2, 5.8, 3.9, 5.3, "Transpose")
}

/// What the bank's display asks for.
#[derive(Default)]
pub(crate) struct Asked {
    /// Open a bank from disk.
    pub open: bool,
    /// Play this preset (bank, program).
    pub pick: Option<(u16, u8)>,
    /// Something dropped on it from the browser.
    pub dropped: Option<BrowserItem>,
    /// Make it (or no longer) the bank MIDI files play with.
    pub midi_files: bool,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn panel(ui: &mut Ui, id: Id, font: Option<&Sf2Ref>, bank: u16, patch: u8, params: &mut [f32], for_midi: bool, height: f32, mv: Option<&mut ModView>, asked: &mut Asked) -> bool {
    let face = face();
    let mut cx = Ctx { id, keys: sf2::SPECS, specs: sf2::SPECS, params, mv, presets: None };
    draw(ui, &face, height, &mut cx, &mut |ui, _, r, _| {
        display(ui, id.with("bank"), r, font, bank, patch, for_midi, asked);
        false
    })
}

/// The display: the bank, the preset big between arrows, where it sits, and
/// buttons to open a bank and to play MIDI files with it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn display(ui: &mut Ui, id: Id, r: Rect, font: Option<&Sf2Ref>, bank: u16, patch: u8, for_midi: bool, asked: &mut Asked) {
    // A bank or a preset from the browser lands here.
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
    let pad = u * 0.5;
    let inner = r.shrink(pad);
    let presets: &[Sf2Preset] = font.map(|f| f.presets.as_slice()).unwrap_or(&[]);
    // The top line: the bank and how many presets it has.
    let top = inner.top() + u * 0.5;
    let bank_name = match font {
        Some(f) if f.is_missing() => format!("MISSING: {}", f.name.to_uppercase()),
        Some(f) => f.name.to_uppercase(),
        None => "NO BANK".into(),
    };
    let count = if presets.is_empty() { String::new() } else { format!("{} PRESETS", presets.len()) };
    let cw = if count.is_empty() { 0.0 } else { p.layout_no_wrap(count.clone(), lcd(u * 0.55), dim).size().x + u * 0.5 };
    let f = fit(&p, &bank_name, lcd(u * 0.6), inner.width() - cw);
    p.text(pos2(inner.left(), top), Align2::LEFT_CENTER, &bank_name, f, dim);
    tabular_text(&p, pos2(inner.right(), top), Align2::RIGHT_CENTER, &count, lcd(u * 0.55), dim);

    // The preset, big, between the arrows.
    let row = Rect::from_min_size(pos2(inner.left(), inner.top() + u * 1.3), vec2(inner.width(), u * 3.6));
    let at = presets.iter().position(|q| q.bank == bank && q.patch == patch);
    if presets.is_empty() {
        let msg = if font.is_some_and(|f| f.is_missing()) { "OPEN IT AGAIN" } else { "DROP A .SF2 BANK" };
        let f = fit(&p, msg, lcd(u * 1.3), row.width());
        p.text(row.center(), Align2::CENTER_CENTER, msg, f, ink);
    } else {
        let aw = u * 1.3;
        let left = Rect::from_min_size(row.min, vec2(aw, row.height()));
        let right = Rect::from_min_size(pos2(row.right() - aw, row.top()), vec2(aw, row.height()));
        for (k, (b, dir)) in [(left, -1i32), (right, 1)].into_iter().enumerate() {
            let resp = ui.interact(b, id.with(("step", k)), Sense::click()).on_hover_text(if dir < 0 { "Previous preset" } else { "Next preset" });
            if resp.clicked() {
                let n = presets.len() as i32;
                let i = match at {
                    Some(i) => (i as i32 + dir).rem_euclid(n),
                    None => 0,
                } as usize;
                asked.pick = Some((presets[i].bank, presets[i].patch));
            }
            let c = b.center();
            let s = u * 0.32;
            let pts = if dir < 0 { vec![c + vec2(s, -s * 1.4), c + vec2(-s, 0.0), c + vec2(s, s * 1.4)] } else { vec![c + vec2(-s, -s * 1.4), c + vec2(s, 0.0), c + vec2(-s, s * 1.4)] };
            p.add(Shape::convex_polygon(pts, if resp.hovered() { ink } else { dim }, Stroke::NONE));
        }
        let mid = Rect::from_min_max(pos2(left.right() + 2.0, row.top()), pos2(right.left() - 2.0, row.bottom()));
        let resp = ui.interact(mid, id.with("name"), Sense::click()).on_hover_text("Click for the bank's presets");
        let name = at.map(|i| presets[i].name.to_uppercase()).unwrap_or_else(|| "PICK A PRESET".into());
        let f = fit(&p, &name, lcd(u * 1.45), mid.width());
        p.text(mid.center(), Align2::CENTER_CENTER, name, f, if resp.hovered() { mix(ink, Color32::WHITE, 0.25) } else { ink });
        if let Some(font) = font {
            egui::Popup::menu(&resp).show(|ui| preset_menu(ui, font, bank, patch, &mut asked.pick));
        }
        // Where it sits: bank and program, and its family in General MIDI.
        let y = row.bottom() + u * 0.75;
        tabular_text(&p, pos2(inner.left(), y), Align2::LEFT_CENTER, &format!("BANK {bank:03}  PROG {:03}", patch as u32 + 1), lcd(u * 0.6), dim);
        let family = if bank >= sf2::DRUM_BANK {
            "DRUM KIT"
        } else if font.is_some_and(|f| f.is_gm()) && bank == 0 {
            sf2::GM_FAMILIES[(patch as usize / 8).min(15)]
        } else {
            ""
        };
        let family = family.to_uppercase();
        let f = fit(&p, &family, lcd(u * 0.6), inner.width() * 0.45);
        p.text(pos2(inner.right(), y), Align2::RIGHT_CENTER, family, f, dim);
    }

    // Along the bottom: open a bank, play MIDI files with this one.
    let bh = u * 1.7;
    let by = inner.bottom() - bh;
    let open_r = Rect::from_min_size(pos2(inner.left(), by), vec2(u * 4.2, bh));
    if lcd_button(ui, &p, open_r, id.with("open"), "OPEN", false, u).on_hover_text("Open a SoundFont 2 bank (.sf2)").clicked() {
        asked.open = true;
    }
    match font {
        Some(f) if f.is_gm() => {
            let mr = Rect::from_min_size(pos2(inner.right() - u * 6.6, by), vec2(u * 6.6, bh));
            let tip = if for_midi { "MIDI files you open play with this bank (click to stop)" } else { "Play the MIDI files you open with this bank: each part on its General MIDI instrument" };
            if lcd_button(ui, &p, mr, id.with("midi"), "MIDI FILES", for_midi, u).on_hover_text(tip).clicked() {
                asked.midi_files = true;
            }
        }
        None => {
            let lr = Rect::from_min_max(pos2(open_r.right() + u * 0.5, by), pos2(inner.right(), by + bh));
            ui.put(lr, egui::Hyperlink::from_label_and_url(RichText::new("Get GeneralUser GS (free GM bank)").size((u * 0.6).clamp(9.0, 12.0)), GM_BANK_URL));
        }
        _ => {}
    }
}

/// A button printed on the display; lit when `on`.
fn lcd_button(ui: &mut Ui, p: &egui::Painter, r: Rect, id: Id, text: &str, on: bool, u: f32) -> egui::Response {
    let resp = ui.interact(r, id, Sense::click());
    let ink = accent_now();
    let fill = if on { shade(ink, 0.35) } else if resp.hovered() { Color32::from_white_alpha(18) } else { Color32::TRANSPARENT };
    p.rect_filled(r, 2.0, fill);
    p.rect_stroke(r, 2.0, Stroke::new(1.0_f32, shade(ink, if on { 0.9 } else { 0.5 })), egui::StrokeKind::Inside);
    let f = fit(p, text, crate::skin::Font::Lcd.id((u * 0.62).max(8.0)), r.width() - 4.0);
    p.text(r.center(), Align2::CENTER_CENTER, text, f, if on { ink } else { shade(ink, 0.8) });
    resp
}

/// The bank's presets to pick from: a short list as it is, a long one by
/// bank, a General MIDI bank's first bank by family.
pub(crate) fn preset_menu(ui: &mut Ui, font: &Sf2Ref, bank: u16, patch: u8, pick: &mut Option<(u16, u8)>) {
    fn item(ui: &mut Ui, pr: &Sf2Preset, bank: u16, patch: u8, pick: &mut Option<(u16, u8)>) {
        if ui.selectable_label(pr.bank == bank && pr.patch == patch, format!("{:03}  {}", pr.patch as u32 + 1, pr.name)).clicked() {
            *pick = Some((pr.bank, pr.patch));
            ui.close();
        }
    }
    let presets = &font.presets;
    if presets.is_empty() {
        ui.label(RichText::new("No presets").color(TEXT_DIM));
        return;
    }
    if presets.len() <= 32 {
        egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
            for pr in presets {
                item(ui, pr, bank, patch, pick);
            }
        });
        return;
    }
    let mut banks: Vec<u16> = presets.iter().map(|p| p.bank).collect();
    banks.dedup();
    for b in banks {
        let in_bank: Vec<&Sf2Preset> = presets.iter().filter(|p| p.bank == b).collect();
        if b == 0 && font.is_gm() {
            for (k, family) in sf2::GM_FAMILIES.iter().enumerate() {
                let kids: Vec<&&Sf2Preset> = in_bank.iter().filter(|p| p.patch as usize / 8 == k).collect();
                if kids.is_empty() {
                    continue;
                }
                let here = bank == 0 && patch as usize / 8 == k;
                ui.menu_button(if here { format!("{family}  ●") } else { family.to_string() }, |ui| {
                    for pr in kids {
                        item(ui, pr, bank, patch, pick);
                    }
                });
            }
            ui.separator();
        } else {
            let title = match b {
                sf2::DRUM_BANK => "Drum Kits".to_string(),
                b if b > sf2::DRUM_BANK => format!("Drum Kits {}", b - sf2::DRUM_BANK),
                b => format!("Bank {b}"),
            };
            ui.menu_button(if b == bank { format!("{title}  ●") } else { title }, |ui| {
                egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    for pr in in_bank {
                        item(ui, pr, bank, patch, pick);
                    }
                });
            });
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_knob_of_the_sf2_player_is_on_its_face() {
        super::super::check(&super::face(), krabton_engine::dsp::sf2::SPECS);
    }
}
