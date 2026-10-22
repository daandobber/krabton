//! Silicon as a board with the chip on it: a screen of the DSP56300 at work
//! (how many million instructions a second it runs, the voices sounding, and
//! Krabton's own firmware going by), the oscillators, the filter, the
//! amplifier's envelope in faders, the LFO and how it plays.

use super::{draw, screen_box, Ctl, Ctx, Face, Presets};
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use egui::{pos2, Align2, FontId, Id, Rect, Ui};
use krabton_engine::dsp::silicon;

pub(crate) fn face() -> Face {
    Face::new(49.6, 12.0)
        .section(0.0, 0.0, 10.0, 12.0, "Chip")
        .preset(0.3, 0.4, 9.4, 1.4)
        .screen(0.3, 2.2, 9.4, 8.9, "chip")
        .section(10.2, 0.0, 9.4, 12.0, "Oscillators")
        .knob(10.4, 0.3, 3.0, 5.4, "Shape")
        .knob(13.4, 0.3, 3.0, 5.4, "Osc 2")
        .knob(16.4, 0.3, 3.0, 5.4, "Sub")
        .knob(10.4, 5.9, 3.0, 5.3, "Detune")
        .named(13.4, 5.9, 3.0, 5.3, "Semitone", Ctl::Knob, "Semi")
        .section(19.8, 0.0, 9.4, 12.0, "Filter")
        .named(20.0, 0.3, 3.0, 5.4, "Filter", Ctl::Selector, "Mode")
        .knob(23.0, 0.3, 3.0, 5.4, "Cutoff")
        .named(26.0, 0.3, 3.0, 5.4, "Resonance", Ctl::Knob, "Res")
        .named(20.0, 5.9, 3.0, 5.3, "Env Amount", Ctl::Knob, "Env")
        .named(23.0, 5.9, 3.0, 5.3, "Env Decay", Ctl::Knob, "Decay")
        .named(26.0, 5.9, 3.0, 5.3, "Key Track", Ctl::Knob, "Key")
        .section(29.4, 0.0, 6.8, 12.0, "Amp")
        .named(29.6, 0.3, 1.6, 10.9, "Attack", Ctl::Fader, "A")
        .named(31.2, 0.3, 1.6, 10.9, "Decay", Ctl::Fader, "D")
        .named(32.8, 0.3, 1.6, 10.9, "Sustain", Ctl::Fader, "S")
        .named(34.4, 0.3, 1.6, 10.9, "Release", Ctl::Fader, "R")
        .section(36.4, 0.0, 6.4, 12.0, "LFO")
        .named(36.6, 0.3, 6.0, 5.4, "LFO Rate", Ctl::Knob, "Rate")
        .named(36.6, 5.9, 3.0, 5.3, "LFO Pitch", Ctl::Knob, "Pitch")
        .named(39.6, 5.9, 3.0, 5.3, "LFO Cutoff", Ctl::Knob, "Cutoff")
        .section(43.0, 0.0, 6.6, 12.0, "Play")
        .knob(43.2, 0.3, 3.1, 5.4, "Glide")
        .knob(46.3, 0.3, 3.1, 5.4, "Voices")
        .knob(43.2, 5.9, 3.1, 5.3, "Velocity")
        .knob(46.3, 5.9, 3.1, 5.3, "Volume")
}

/// Silicon's face, its screen live. Returns whether a value changed.
pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut [f32], live: &InstLive, height: f32, mv: Option<&mut ModView>) -> bool {
    let face = face();
    let mut cx = Ctx { id, keys: silicon::SPECS, specs: silicon::SPECS, params, mv, presets: Some(Presets { current: preset, names: silicon::preset_names(), load: silicon::preset }) };
    let changed = draw(ui, &face, height, &mut cx, &mut |ui, _which, rect, _p| {
        screen(ui, rect, live);
        false
    });
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
    changed
}

/// The firmware's instructions (comments and blank lines left out).
fn listing() -> &'static [String] {
    static LINES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    LINES.get_or_init(|| {
        silicon::FIRMWARE
            .lines()
            .map(|l| l.split(';').next().unwrap_or("").trim_end().to_string())
            .filter(|l| !l.trim().is_empty() && !l.contains(" equ "))
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect()
    })
}

/// The chip at work: its speed, its voices, its firmware going by.
fn screen(ui: &mut Ui, r: Rect, live: &InstLive) {
    let p = ui.painter().clone();
    let u = r.height() / 10.6;
    screen_box(&p, r, u);
    let ink = accent_now();
    let dim = shade(ink, 0.45);
    let inner = r.shrink(u * 0.6);
    let row = (inner.height() / 9.0).max(8.0);
    // As big as the widest line lets it be (a monospace letter is about 0.6 of its size wide).
    let title = "DSP56300 · KRABTON FIRMWARE";
    let mono = FontId::monospace((row * 0.78).min(inner.width() / (title.chars().count() as f32 * 0.62)).clamp(6.0, 13.0));
    let p = p.with_clip_rect(inner);
    p.text(inner.left_top(), Align2::LEFT_TOP, title, mono.clone(), ink);
    let status = if live.vals[2] < 0.5 {
        "NO CHIP".to_string()
    } else {
        format!("{:>5.1} MIPS   {} VOICES", live.vals[1], live.vals[0] as u32)
    };
    p.text(pos2(inner.left(), inner.top() + row), Align2::LEFT_TOP, status, mono.clone(), if live.vals[0] > 0.0 { ink } else { dim });
    let lines = listing();
    if lines.is_empty() {
        return;
    }
    // The program going by, faster while it plays.
    let t = ui.input(|i| i.time);
    let speed = if live.vals[0] > 0.0 { 6.0 } else { 1.0 };
    let first = (t * speed) as usize;
    let shown = 6;
    for k in 0..shown {
        let line = &lines[(first + k) % lines.len()];
        let y = inner.top() + row * (2.6 + k as f32);
        let c = if k == shown / 2 { ink } else { dim };
        p.text(pos2(inner.left(), y), Align2::LEFT_TOP, line, mono.clone(), c);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_knob_of_silicon_is_on_its_face() {
        super::super::check(&super::face(), krabton_engine::dsp::silicon::SPECS);
    }

    #[test]
    fn the_listing_is_the_firmware() {
        let l = super::listing();
        assert!(l.iter().any(|x| x.starts_with("render_block")) && l.iter().any(|x| x.contains("mac")));
    }
}
