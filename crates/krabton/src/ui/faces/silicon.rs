//! Silicon as a board with the chip on it: a screen of the DSP56300 at work
//! (how many million instructions a second it runs, the voices sounding, and
//! Krabton's own firmware going by), the two oscillators, the filters, both
//! envelopes in faders, the LFO, the voices and the effects.

use super::{draw, screen_box, Ctl, Ctx, Face, Presets};
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use egui::{pos2, Align2, FontId, Id, Rect, Ui};
use krabton_engine::dsp::silicon;

pub(crate) fn face() -> Face {
    Face::new(89.2, 12.0)
        .section(0.0, 0.0, 8.4, 12.0, "Chip")
        .preset(0.3, 0.4, 7.8, 1.4)
        .screen(0.3, 2.2, 7.8, 8.9, "chip")
        .section(8.6, 0.0, 6.4, 12.0, "Osc 1")
        .knob(8.8, 0.3, 3.0, 5.4, "Shape")
        .named(11.8, 0.3, 3.0, 5.4, "Pulse Width", Ctl::Knob, "PW")
        .knob(8.8, 5.9, 3.0, 5.3, "Sub")
        .knob(11.8, 5.9, 3.0, 5.3, "Noise")
        .section(15.2, 0.0, 12.4, 12.0, "Osc 2")
        .named(15.4, 0.3, 3.0, 5.4, "Shape 2", Ctl::Knob, "Shape")
        .named(18.4, 0.3, 3.0, 5.4, "Pulse Width 2", Ctl::Knob, "PW")
        .named(21.4, 0.3, 3.0, 5.4, "Osc 2", Ctl::Knob, "Level")
        .named(24.4, 0.3, 3.0, 5.4, "Semitone", Ctl::Knob, "Semi")
        .knob(15.4, 5.9, 3.0, 5.3, "Detune")
        .knob(18.4, 5.9, 3.0, 5.3, "FM")
        .knob(21.4, 5.9, 3.0, 5.3, "Ring")
        .ctl(24.4, 5.9, 3.0, 5.3, "Sync", Ctl::Switch)
        .section(27.8, 0.0, 12.4, 12.0, "Filter")
        .named(28.0, 0.3, 3.0, 3.55, "Filter", Ctl::Selector, "Mode")
        .knob(31.0, 0.3, 3.0, 3.55, "Cutoff")
        .named(34.0, 0.3, 3.0, 3.55, "Resonance", Ctl::Knob, "Res")
        .named(37.0, 0.3, 3.0, 3.55, "Env Amount", Ctl::Knob, "Env")
        .named(28.0, 3.95, 3.0, 3.55, "Key Track", Ctl::Knob, "Key")
        .named(31.0, 3.95, 3.0, 3.55, "Vel > Cutoff", Ctl::Knob, "Vel")
        .knob(34.0, 3.95, 3.0, 3.55, "Drive")
        .named(28.0, 7.6, 3.0, 3.55, "Routing", Ctl::Selector, "Filter 2")
        .named(31.0, 7.6, 3.0, 3.55, "Filter 2", Ctl::Selector, "Mode 2")
        .named(34.0, 7.6, 3.0, 3.55, "Cutoff 2", Ctl::Knob, "Cutoff 2")
        .named(37.0, 7.6, 3.0, 3.55, "Resonance 2", Ctl::Knob, "Res 2")
        .section(40.4, 0.0, 6.6, 12.0, "Filter Env")
        .named(40.6, 0.3, 1.6, 10.9, "Env Attack", Ctl::Fader, "A")
        .named(42.2, 0.3, 1.6, 10.9, "Env Decay", Ctl::Fader, "D")
        .named(43.8, 0.3, 1.6, 10.9, "Env Sustain", Ctl::Fader, "S")
        .named(45.4, 0.3, 1.6, 10.9, "Env Release", Ctl::Fader, "R")
        .section(47.2, 0.0, 6.6, 12.0, "Amp")
        .named(47.4, 0.3, 1.6, 10.9, "Attack", Ctl::Fader, "A")
        .named(49.0, 0.3, 1.6, 10.9, "Decay", Ctl::Fader, "D")
        .named(50.6, 0.3, 1.6, 10.9, "Sustain", Ctl::Fader, "S")
        .named(52.2, 0.3, 1.6, 10.9, "Release", Ctl::Fader, "R")
        .section(54.0, 0.0, 9.4, 12.0, "LFO")
        .named(54.2, 0.3, 3.0, 5.4, "LFO Shape", Ctl::Selector, "Shape")
        .named(57.2, 0.3, 3.0, 5.4, "LFO Rate", Ctl::Knob, "Rate")
        .named(60.2, 0.3, 3.0, 5.4, "LFO Pitch", Ctl::Knob, "Pitch")
        .named(54.2, 5.9, 3.0, 5.3, "LFO Cutoff", Ctl::Knob, "Cutoff")
        .named(57.2, 5.9, 3.0, 5.3, "LFO PW", Ctl::Knob, "PW")
        .named(60.2, 5.9, 3.0, 5.3, "LFO Amp", Ctl::Knob, "Level")
        .section(63.6, 0.0, 9.4, 12.0, "Voices")
        .knob(63.8, 0.3, 3.0, 5.4, "Voices")
        .knob(66.8, 0.3, 3.0, 5.4, "Unison")
        .named(69.8, 0.3, 3.0, 5.4, "Unison Detune", Ctl::Knob, "Detune")
        .knob(63.8, 5.9, 3.0, 5.3, "Spread")
        .knob(66.8, 5.9, 3.0, 5.3, "Glide")
        .knob(69.8, 5.9, 3.0, 5.3, "Octave")
        .section(73.2, 0.0, 12.4, 12.0, "Effects")
        .knob(73.4, 0.3, 3.0, 5.4, "Chorus")
        .named(76.4, 0.3, 3.0, 5.4, "Chorus Depth", Ctl::Knob, "Depth")
        .knob(79.4, 0.3, 3.0, 5.4, "Delay")
        .named(82.4, 0.3, 3.0, 5.4, "Delay Time", Ctl::Knob, "Time")
        .knob(73.4, 5.9, 3.0, 5.3, "Feedback")
        .knob(76.4, 5.9, 3.0, 5.3, "Reverb")
        .named(79.4, 5.9, 3.0, 5.3, "Reverb Size", Ctl::Knob, "Size")
        .named(82.4, 5.9, 3.0, 5.3, "Reverb Damp", Ctl::Knob, "Damp")
        .section(85.8, 0.0, 3.4, 12.0, "Output")
        .knob(86.0, 0.3, 3.0, 5.4, "Velocity")
        .knob(86.0, 5.9, 3.0, 5.3, "Volume")
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
