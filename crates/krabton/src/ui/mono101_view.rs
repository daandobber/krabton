//! Mono 101's face: its sections as the other devices draw their groups
//! (Modulator, VCO, Source Mixer, VCF, VCA, Env, Portamento, Output), with
//! the panel's vertical sliders as Krabton's faders and its switches as
//! Krabton's buttons. Modulation and envelopes show on the sliders as on
//! the knobs; the cutoff shows where the envelope and the LFO take it.

use super::inst_screen::InstLive;
use super::modulation::{slider_overlay, ModView};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Id, Rect, Sense, Stroke, Ui};
use krabton_engine::dsp::mono101 as m;
use krabton_engine::params::{ParamKind, ParamSpec};

const SLIDER_W: f32 = 38.0;
const SWITCH_W: f32 = 62.0;

enum Ctl {
    Slider(usize),
    Switch(usize),
}

fn sections() -> Vec<(&'static str, Vec<Ctl>)> {
    use Ctl::*;
    vec![
        ("Modulator", vec![Slider(m::LFO_RATE), Switch(m::LFO_WAVE), Switch(m::LFO_SYNC)]),
        ("VCO", vec![Switch(m::RANGE), Slider(m::VCO_LFO), Slider(m::PW), Switch(m::PWM_SOURCE), Slider(m::FINE)]),
        ("Source Mixer", vec![Slider(m::PULSE), Slider(m::SAW), Slider(m::SUB), Slider(m::NOISE), Switch(m::SUB_MODE)]),
        ("VCF", vec![Slider(m::CUTOFF), Slider(m::RES), Slider(m::ENV_AMT), Slider(m::VCF_LFO), Slider(m::KYBD)]),
        ("VCA", vec![Switch(m::VCA_MODE)]),
        ("Env", vec![Slider(m::ATTACK), Slider(m::DECAY), Slider(m::SUSTAIN), Slider(m::RELEASE), Switch(m::TRIGGER)]),
        ("Portamento", vec![Switch(m::PORTA_MODE), Slider(m::PORTA_TIME)]),
        ("Output", vec![Slider(m::VELOCITY), Slider(m::VINTAGE), Slider(m::VOLUME)]),
    ]
}

fn width_of(c: &Ctl) -> f32 {
    if matches!(c, Ctl::Slider(_)) {
        SLIDER_W
    } else {
        SWITCH_W
    }
}

/// A switch as a column of Krabton's buttons, its name on top like a
/// parameter cell. Returns true when changed.
fn switch(ui: &mut Ui, r: Rect, id: Id, spec: &ParamSpec, value: &mut f32, accent: egui::Color32) -> bool {
    let p = ui.painter().clone();
    let toggle = !matches!(spec.kind, ParamKind::Choice(_));
    let opts: Vec<&str> = match spec.kind {
        ParamKind::Choice(o) => o.to_vec(),
        _ => vec!["On", "Off"],
    };
    let label = Rect::from_min_size(r.min, vec2(r.width(), 12.0));
    p.with_clip_rect(label).text(label.center(), Align2::CENTER_CENTER, spec.name, small(), TEXT_DIM);
    let area = Rect::from_min_max(pos2(r.left() + 3.0, label.bottom() + 4.0), pos2(r.right() - 3.0, r.bottom() - 4.0));
    let n = opts.len().max(1);
    let h = (area.height() / n as f32).min(20.0);
    let top = area.top() + (area.height() - h * n as f32) * 0.3;
    let cur = if toggle { usize::from(*value < 0.5) } else { value.round().max(0.0) as usize };
    let mut changed = false;
    for (k, label) in opts.iter().enumerate() {
        let b = Rect::from_min_size(pos2(area.left(), top + k as f32 * h), vec2(area.width(), h - 2.0));
        if button(ui, b, id.with(k), label, k == cur, accent).clicked() && k != cur {
            *value = if toggle { if k == 0 { 1.0 } else { 0.0 } } else { k as f32 };
            changed = true;
        }
    }
    changed
}

/// The panel. `live` is what the engine reports (envelope, LFO, cutoff).
pub(crate) fn mono101_panel(ui: &mut Ui, id: Id, params: &mut [f32], live: &InstLive, height: f32, mut mv: Option<&mut ModView>) -> bool {
    if params.len() < m::SPECS.len() {
        return false;
    }
    let accent = accent_now();
    let secs = sections();
    let pad = 10.0;
    let width: f32 = secs.iter().map(|(_, c)| c.iter().map(width_of).sum::<f32>() + pad).sum();
    let h = height.clamp(120.0, 260.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let p = ui.painter().clone();
    let mut changed = false;
    let mut x = rect.left();
    let playing = live.vals[3] > 0.5 || live.vals[0] > 0.001;
    for (gi, (title, ctls)) in secs.into_iter().enumerate() {
        // A group as the other devices draw theirs: its name, a line between.
        p.text(pos2(x + 5.0, rect.top() + 6.0), Align2::LEFT_CENTER, title, small(), shade(accent, 0.85));
        if gi > 0 {
            p.line_segment([pos2(x, rect.top() + 2.0), pos2(x, rect.bottom() - 2.0)], Stroke::new(1.0_f32, LINE_SOFT));
        }
        let mut cx = x + 5.0;
        for c in ctls {
            let cell = Rect::from_min_size(pos2(cx, rect.top() + 14.0), vec2(width_of(&c) - 2.0, h - 16.0));
            match c {
                Ctl::Slider(k) => {
                    let (c, track) = param_slider(ui, cell, id.with(("s", k)), &m::SPECS[k], &mut params[k], accent);
                    changed |= c;
                    if k == m::CUTOFF && playing {
                        // Where the envelope and the LFO take the filter now.
                        let y = track.bottom() - live.vals[2].clamp(0.0, 1.0) * track.height();
                        p.line_segment([pos2(track.left() - 5.0, y), pos2(track.right() + 5.0, y)], Stroke::new(1.5_f32, accent));
                    }
                    if let Some(mv) = mv.as_deref_mut() {
                        slider_overlay(ui, cell, track, id.with(("sm", k)), &m::SPECS[k], params[k], k, mv);
                    }
                }
                Ctl::Switch(k) => {
                    changed |= switch(ui, cell, id.with(("w", k)), &m::SPECS[k], &mut params[k], accent);
                }
            }
            cx += width_of(&c);
        }
        x = cx - 5.0 + pad;
    }
    if playing {
        ui.ctx().request_repaint();
    }
    changed
}
