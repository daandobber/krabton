//! Silicon laid out as the big desktop virtual analogs are: a screen of the
//! DSP56300 at work (how many million instructions a second it runs, the
//! voices sounding, and Krabton's own firmware going by), the three
//! oscillators with the sub, the noise and the ring, both filters with their
//! routing and saturation in displays, both envelopes in faders with their
//! sustain's slope, the two LFOs, the voices, and the effects.

use super::{draw, screen_box, Ctl, Ctx, Face, Presets};
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use egui::{pos2, Align2, FontId, Id, Rect, Ui};
use krabton_engine::dsp::silicon;

pub(crate) fn face() -> Face {
    Face::new(106.8, 12.0)
        .section(0.0, 0.0, 8.2, 12.0, "Chip")
        .preset(0.3, 0.4, 7.6, 1.4)
        .screen(0.3, 2.2, 7.6, 8.9, "chip")
        .section(8.4, 0.0, 8.2, 12.0, "Osc 1")
        .named(8.6, 0.3, 2.6, 3.55, "Osc 1 Mode", Ctl::Buttons, "Mode")
        .knob(11.2, 0.3, 2.6, 3.55, "Shape")
        .named(13.8, 0.3, 2.6, 3.55, "Pulse Width", Ctl::Knob, "PW")
        .knob(8.6, 3.95, 2.6, 3.55, "Density")
        .named(11.2, 3.95, 2.6, 3.55, "Saw Spread", Ctl::Knob, "Spread")
        .named(13.8, 3.95, 2.6, 3.55, "Wave 1", Ctl::Knob, "Wave")
        .named(8.6, 7.6, 7.8, 3.55, "Table 1", Ctl::Lcd, "Table")
        .section(16.8, 0.0, 10.8, 12.0, "Osc 2")
        .named(17.0, 0.3, 2.6, 3.55, "Osc 2 Mode", Ctl::Buttons, "Mode")
        .named(19.6, 0.3, 2.6, 3.55, "Shape 2", Ctl::Knob, "Shape")
        .named(22.2, 0.3, 2.6, 3.55, "Pulse Width 2", Ctl::Knob, "PW")
        .named(24.8, 0.3, 2.6, 3.55, "Semitone", Ctl::Knob, "Semi")
        .knob(17.0, 3.95, 2.6, 3.55, "Detune")
        .knob(19.6, 3.95, 2.6, 3.55, "FM")
        .named(22.2, 3.95, 2.6, 3.55, "Env > FM", Ctl::Knob, "Env FM")
        .named(24.8, 3.95, 2.6, 3.55, "Env > Osc 2", Ctl::Knob, "Env Pitch")
        .named(17.0, 7.6, 5.2, 3.55, "Table 2", Ctl::Lcd, "Table")
        .named(22.2, 7.6, 2.6, 3.55, "Wave 2", Ctl::Knob, "Wave")
        .ctl(24.8, 7.6, 2.6, 3.55, "Sync", Ctl::Switch)
        .section(27.8, 0.0, 8.2, 12.0, "Osc 3 & Mix")
        .named(28.0, 0.3, 2.6, 3.55, "Osc 3", Ctl::Knob, "Osc 3")
        .named(30.6, 0.3, 2.6, 3.55, "Semitone 3", Ctl::Knob, "Semi")
        .named(33.2, 0.3, 2.6, 3.55, "Shape 3", Ctl::Knob, "Shape")
        .knob(28.0, 3.95, 2.6, 3.55, "Sub")
        .named(30.6, 3.95, 2.6, 3.55, "Sub Wave", Ctl::Switch, "Wave")
        .knob(33.2, 3.95, 2.6, 3.55, "Ring")
        .knob(28.0, 7.6, 2.6, 3.55, "Noise")
        .named(30.6, 7.6, 2.6, 3.55, "Noise Color", Ctl::Knob, "Color")
        .named(33.2, 7.6, 2.6, 3.55, "Osc 2", Ctl::Knob, "Osc 2")
        .section(36.2, 0.0, 16.0, 12.0, "Filters")
        .named(36.4, 0.3, 5.2, 3.55, "Filter", Ctl::Lcd, "Filter 1")
        .knob(41.6, 0.3, 2.6, 3.55, "Cutoff")
        .named(44.2, 0.3, 2.6, 3.55, "Resonance", Ctl::Knob, "Res")
        .named(46.8, 0.3, 2.6, 3.55, "Env Amount", Ctl::Knob, "Env")
        .named(49.4, 0.3, 2.6, 3.55, "Key Track", Ctl::Knob, "Key")
        .named(36.4, 3.95, 5.2, 3.55, "Routing", Ctl::Lcd, "Routing")
        .knob(41.6, 3.95, 2.6, 3.55, "Cutoff 2")
        .named(44.2, 3.95, 2.6, 3.55, "Resonance 2", Ctl::Knob, "Res 2")
        .named(46.8, 3.95, 2.6, 3.55, "Filter Balance", Ctl::Knob, "Balance")
        .named(49.4, 3.95, 2.6, 3.55, "Vel > Cutoff", Ctl::Knob, "Vel")
        .named(36.4, 7.6, 2.6, 3.55, "Filter 2", Ctl::Selector, "Filter 2")
        .knob(39.0, 7.6, 2.6, 3.55, "Drive")
        .named(41.6, 7.6, 5.2, 3.55, "Saturation", Ctl::Lcd, "Saturation")
        .named(46.8, 7.6, 2.6, 3.55, "Sat Amount", Ctl::Knob, "Amount")
        .section(52.4, 0.0, 7.9, 12.0, "Filter Env")
        .named(52.6, 0.3, 1.5, 10.9, "Env Attack", Ctl::Fader, "A")
        .named(54.1, 0.3, 1.5, 10.9, "Env Decay", Ctl::Fader, "D")
        .named(55.6, 0.3, 1.5, 10.9, "Env Sustain", Ctl::Fader, "S")
        .named(57.1, 0.3, 1.5, 10.9, "Env Slope", Ctl::Fader, "Slope")
        .named(58.6, 0.3, 1.5, 10.9, "Env Release", Ctl::Fader, "R")
        .section(60.5, 0.0, 7.9, 12.0, "Amp Env")
        .named(60.7, 0.3, 1.5, 10.9, "Attack", Ctl::Fader, "A")
        .named(62.2, 0.3, 1.5, 10.9, "Decay", Ctl::Fader, "D")
        .named(63.7, 0.3, 1.5, 10.9, "Sustain", Ctl::Fader, "S")
        .named(65.2, 0.3, 1.5, 10.9, "Amp Slope", Ctl::Fader, "Slope")
        .named(66.7, 0.3, 1.5, 10.9, "Release", Ctl::Fader, "R")
        .section(68.6, 0.0, 13.4, 12.0, "LFOs")
        .named(68.8, 0.3, 5.2, 3.55, "LFO Shape", Ctl::Lcd, "LFO 1")
        .named(74.0, 0.3, 5.2, 3.55, "LFO 2 Shape", Ctl::Lcd, "LFO 2")
        .named(68.8, 3.95, 2.6, 3.55, "LFO Rate", Ctl::Knob, "Rate")
        .named(71.4, 3.95, 2.6, 3.55, "LFO Pitch", Ctl::Knob, "Pitch")
        .named(74.0, 3.95, 2.6, 3.55, "LFO Cutoff", Ctl::Knob, "Cutoff")
        .named(76.6, 3.95, 2.6, 3.55, "LFO PW", Ctl::Knob, "PW")
        .named(79.2, 3.95, 2.6, 3.55, "LFO Amp", Ctl::Knob, "Amp")
        .named(68.8, 7.6, 2.6, 3.55, "LFO 2 Rate", Ctl::Knob, "Rate")
        .named(71.4, 7.6, 2.6, 3.55, "LFO 2 Cutoff 2", Ctl::Knob, "Cutoff 2")
        .named(74.0, 7.6, 2.6, 3.55, "LFO 2 Pan", Ctl::Knob, "Pan")
        .named(76.6, 7.6, 2.6, 3.55, "LFO 2 FM", Ctl::Knob, "FM")
        .named(79.2, 7.6, 2.6, 3.55, "LFO 2 Wave", Ctl::Knob, "Wave")
        .section(82.2, 0.0, 8.2, 12.0, "Voices")
        .knob(82.4, 0.3, 2.6, 3.55, "Voices")
        .knob(85.0, 0.3, 2.6, 3.55, "Unison")
        .named(87.6, 0.3, 2.6, 3.55, "Unison Detune", Ctl::Knob, "Detune")
        .knob(82.4, 3.95, 2.6, 3.55, "Spread")
        .knob(85.0, 3.95, 2.6, 3.55, "Glide")
        .knob(87.6, 3.95, 2.6, 3.55, "Octave")
        .knob(82.4, 7.6, 2.6, 3.55, "Velocity")
        .knob(85.0, 7.6, 2.6, 3.55, "Volume")
        .section(90.6, 0.0, 16.0, 12.0, "Effects")
        .named(90.8, 0.3, 5.2, 3.55, "Distortion", Ctl::Lcd, "Distortion")
        .named(96.0, 0.3, 2.6, 3.55, "Dist Amount", Ctl::Knob, "Amount")
        .named(98.6, 0.3, 2.6, 3.55, "Chorus Mode", Ctl::Buttons, "Mode")
        .knob(101.2, 0.3, 2.6, 3.55, "Chorus")
        .named(103.8, 0.3, 2.6, 3.55, "Chorus Depth", Ctl::Knob, "Depth")
        .knob(90.8, 3.95, 2.6, 3.55, "Phaser")
        .named(93.4, 3.95, 2.6, 3.55, "Phaser Rate", Ctl::Knob, "Rate")
        .named(96.0, 3.95, 2.6, 3.55, "Phaser Feedback", Ctl::Knob, "Fdbk")
        .knob(98.6, 3.95, 2.6, 3.55, "Delay")
        .named(101.2, 3.95, 2.6, 3.55, "Delay Time", Ctl::Knob, "Time")
        .named(103.8, 3.95, 2.6, 3.55, "Feedback", Ctl::Knob, "Repeats")
        .knob(90.8, 7.6, 2.6, 3.55, "Reverb")
        .named(93.4, 7.6, 2.6, 3.55, "Reverb Size", Ctl::Knob, "Size")
        .named(96.0, 7.6, 2.6, 3.55, "Reverb Damp", Ctl::Knob, "Damp")
        .knob(98.6, 7.6, 2.6, 3.55, "Boost")
        .named(101.2, 7.6, 2.6, 3.55, "Boost Freq", Ctl::Knob, "Freq")
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
