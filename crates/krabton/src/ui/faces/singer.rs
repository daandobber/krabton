//! The Singer as a vocal booth: the voicebank's display with the syllable
//! sung big (in kana when the voicebank is Japanese), the note and the voice's
//! shape under it, then the voice, its vibrato and the output.

use super::sf2::{lcd_button, OfferState, OfferView};
use super::{draw, fit, screen_box, Ctl, Ctx, Face};
use crate::app::BrowserItem;
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Id, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::singer::{self, VoicebankRef};

pub(crate) fn face() -> Face {
    Face::new(35.0, 12.0)
        .screen(0.3, 0.3, 14.0, 10.6, "voice")
        .section(14.6, 0.0, 6.6, 12.0, "Voice")
        .knob(14.7, 0.3, 3.1, 5.3, "Gender")
        .knob(17.8, 0.3, 3.1, 5.3, "Breath")
        .knob(14.7, 5.8, 3.1, 5.3, "Consonant")
        .knob(17.8, 5.8, 3.1, 5.3, "Portamento")
        .section(21.4, 0.0, 6.6, 12.0, "Vibrato")
        .named(21.5, 0.3, 6.4, 7.2, "Vibrato", Ctl::Knob, "Depth")
        .named(21.5, 7.5, 3.2, 3.6, "Vib Rate", Ctl::Knob, "Rate")
        .named(24.7, 7.5, 3.2, 3.6, "Vib Delay", Ctl::Knob, "Delay")
        .section(28.2, 0.0, 6.6, 12.0, "Output")
        .knob(28.3, 0.3, 3.1, 5.3, "Volume")
        .knob(31.4, 0.3, 3.1, 5.3, "Release")
        .knob(28.3, 5.8, 3.1, 5.3, "Transpose")
        .named(31.4, 5.8, 3.1, 5.3, "Default", Ctl::Selector, "No Lyric")
}

/// What the display asks for.
#[derive(Default)]
pub(crate) struct Asked {
    /// Open a voicebank's folder.
    pub open: bool,
    /// Sing with its own voice (no voicebank).
    pub own: bool,
    /// A voicebank dropped on it from the browser.
    pub dropped: Option<BrowserItem>,
    /// Get (or, when it is here, sing with) offered voicebank number `n`.
    pub get: Option<usize>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn panel(ui: &mut Ui, id: Id, bank: Option<&VoicebankRef>, params: &mut [f32], live: &InstLive, offers: &[OfferView], height: f32, mv: Option<&mut ModView>, asked: &mut Asked) -> bool {
    let face = face();
    let mut cx = Ctx { id, keys: singer::SPECS, specs: singer::SPECS, params, mv, presets: None };
    let changed = draw(ui, &face, height, &mut cx, &mut |ui, _, r, _| {
        display(ui, id.with("voice"), r, bank, live, offers, asked);
        false
    });
    let fetching = offers.iter().any(|o| matches!(o.state, OfferState::Downloading(_)));
    if live.vals[2] > 0.0 || fetching || bank.is_some_and(|b| b.ready() < 1.0) {
        ui.ctx().request_repaint();
    }
    changed
}

/// The display: the voicebank and what kind it is, the syllable being sung
/// big with its note, the voice's wave, and buttons to open a voicebank or
/// sing without one; without a voicebank, the free ones Krabton can get.
pub(crate) fn display(ui: &mut Ui, id: Id, r: Rect, bank: Option<&VoicebankRef>, live: &InstLive, offers: &[OfferView], asked: &mut Asked) {
    // A voicebank from the browser lands here.
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

    // The top line: the voicebank, and what kind it is (or how far it has listened).
    let top = inner.top() + u * 0.5;
    let name = match bank {
        Some(b) if b.is_missing() => format!("MISSING: {}", b.name.to_uppercase()),
        Some(b) => b.name.to_uppercase(),
        None => "OWN VOICE".into(),
    };
    let kind = match bank {
        Some(b) if b.is_missing() => String::new(),
        Some(b) if b.ready() < 1.0 => format!("LISTENING {:.0}%", b.ready() * 100.0),
        Some(b) => format!("{} · {}", if b.vcv { "VCV" } else { "CV" }, b.entries.len()),
        None => "FORMANTS".into(),
    };
    let kw = p.layout_no_wrap(kind.clone(), lcd(u * 0.55), dim).size().x + u * 0.5;
    let f = fit(&p, &name, lcd(u * 0.6), inner.width() - kw);
    p.text(pos2(inner.left(), top), Align2::LEFT_CENTER, &name, f, dim);
    tabular_text(&p, pos2(inner.right(), top), Align2::RIGHT_CENTER, &kind, lcd(u * 0.55), dim);
    if let Some(b) = bank.filter(|b| b.ready() < 1.0) {
        let bar = Rect::from_min_size(pos2(inner.left(), top + u * 0.45), vec2(inner.width() * b.ready().clamp(0.0, 1.0), u * 0.12));
        p.rect_filled(bar, 0.0, shade(ink, 0.6));
    }

    // The syllable, big, and the note it is on; or, with no voicebank and
    // nothing sung, the voices to get.
    let sounding = live.vals[2] > 0.001;
    if bank.is_none() && !sounding && !offers.is_empty() {
        let head = Rect::from_min_size(pos2(inner.left(), inner.top() + u * 1.1), vec2(inner.width(), u * 1.2));
        p.text(head.center(), Align2::CENTER_CENTER, "GET A VOICE", fit(&p, "GET A VOICE", lcd(u * 0.9), head.width()), ink);
        let rows = offers.len() as f32;
        let room = (inner.bottom() - u * 2.1) - (head.bottom() + u * 0.3);
        let step = (room / rows).min(u * 1.75);
        for (k, o) in offers.iter().enumerate() {
            let b = Rect::from_min_size(pos2(inner.left(), head.bottom() + u * 0.3 + k as f32 * step), vec2(inner.width(), step - u * 0.25));
            let (text, on) = match o.state {
                OfferState::Get => (format!("{}  ·  {} MB", o.name.to_uppercase(), o.mb), false),
                OfferState::Here => (o.name.to_uppercase(), false),
                OfferState::Downloading(f) if f >= 1.0 => (format!("{}  UNPACKING", o.name.to_uppercase()), true),
                OfferState::Downloading(f) => (format!("{}  {:.0}%", o.name.to_uppercase(), f * 100.0), true),
                OfferState::Failed => (format!("{}: FAILED, AGAIN?", o.name.to_uppercase()), false),
            };
            if let OfferState::Downloading(f) = o.state {
                p.rect_filled(Rect::from_min_size(b.min, vec2(b.width() * f.clamp(0.0, 1.0), b.height())), 2.0, shade(ink, 0.25));
            }
            let offer = &crate::singer_lib::OFFERS[k];
            let tip = format!("{}.\n{}. Downloaded from its mirror when you click, with its own terms beside it.", offer.about, offer.terms);
            if lcd_button(ui, &p, b, id.with(("get", k)), &text, on, u).on_hover_text(tip).clicked() && !matches!(o.state, OfferState::Downloading(_)) {
                asked.get = Some(k);
            }
        }
    } else {
        sung_view(&p, inner, u, bank, live, sounding);
    }

    // Along the bottom: open a voicebank, or sing without one.
    let bh = u * 1.7;
    let by = inner.bottom() - bh;
    let open_r = Rect::from_min_size(pos2(inner.left(), by), vec2(u * 4.6, bh));
    if lcd_button(ui, &p, open_r, id.with("open"), "OPEN", false, u).on_hover_text("Open a UTAU voicebank's folder (the one with its oto.ini)").clicked() {
        asked.open = true;
    }
    if bank.is_some() {
        let own_r = Rect::from_min_size(pos2(inner.right() - u * 6.0, by), vec2(u * 6.0, bh));
        if lcd_button(ui, &p, own_r, id.with("own"), "OWN VOICE", false, u).on_hover_text("Sing with the Singer's own voice (vowels from formants), without the voicebank").clicked() {
            asked.own = true;
        }
    }
}

/// What is sung: the syllable big, its note, where it comes from, the wave.
fn sung_view(p: &egui::Painter, inner: Rect, u: f32, bank: Option<&VoicebankRef>, live: &InstLive, sounding: bool) {
    let ink = accent_now();
    let dim = shade(ink, 0.55);
    let lcd = |size: f32| crate::skin::Font::Lcd.id(size.max(8.0));
    let sung = singer::sung_of(&live.vals);
    let big = Rect::from_min_size(pos2(inner.left(), inner.top() + u * 1.2), vec2(inner.width(), u * 4.4));
    let (text, color) = if sounding && !sung.is_empty() {
        (sung, ink)
    } else if bank.is_some_and(|b| b.is_missing()) {
        ("OPEN IT AGAIN".to_string(), dim)
    } else {
        ("· · ·".to_string(), dim)
    };
    // Kana come from the system's font, which the dot matrix does not have.
    let font = if text.is_ascii() { lcd(u * 3.0) } else { egui::FontId::proportional(u * 2.6) };
    let f = fit(&p, &text, font, big.width());
    p.text(big.center(), Align2::CENTER_CENTER, &text, f, color);
    let y = big.bottom() + u * 0.5;
    let note = if sounding { krabton_engine::note_name(live.vals[1].round().clamp(0.0, 127.0) as u8) } else { "--".into() };
    tabular_text(&p, pos2(inner.left(), y), Align2::LEFT_CENTER, &note, lcd(u * 0.8), if sounding { ink } else { dim });
    let src = if !sounding {
        ""
    } else if live.vals[3] > 0.5 {
        "FROM THE VOICEBANK"
    } else {
        "OWN VOICE"
    };
    tabular_text(&p, pos2(inner.right(), y), Align2::RIGHT_CENTER, src, lcd(u * 0.55), dim);

    // The voice's wave.
    let wave = Rect::from_min_max(pos2(inner.left(), y + u * 0.6), pos2(inner.right(), inner.bottom() - u * 2.1));
    if wave.height() > 4.0 {
        p.line_segment([pos2(wave.left(), wave.center().y), pos2(wave.right(), wave.center().y)], Stroke::new(1.0_f32, shade(ink, 0.2)));
        let w = &live.wave[live.wave.len().saturating_sub(1024)..];
        if w.len() > 1 && sounding {
            let peak = w.iter().fold(0.05f32, |m, x| m.max(x.abs()));
            let n = w.len();
            let pts: Vec<_> = (0..wave.width() as usize)
                .map(|k| {
                    let x = wave.left() + k as f32;
                    let s = w[(k * n / wave.width().max(1.0) as usize).min(n - 1)] / peak;
                    pos2(x, wave.center().y - s * wave.height() * 0.45)
                })
                .collect();
            p.add(Shape::line(pts, Stroke::new(1.2_f32, ink)));
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_knob_of_the_singer_is_on_its_face() {
        super::super::check(&super::face(), krabton_engine::singer::SPECS);
    }
}
