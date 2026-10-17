//! Small live screens for Big Picture's Rack: a device's own screen from the
//! desktop view (an instrument's, a sequencer's editor, a scope), drawn on
//! a layer of its own that is scaled down into the card and does not take
//! input; or, for a device without one, the sound it makes and the notes it
//! plays.

use super::devices::Link;
use crate::app::App;
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use egui::emath::TSTransform;
use egui::{pos2, vec2, Color32, Id, Painter, Pos2, Rect, Shape, Stroke};
use krabton_engine::dsp::stk_synth::Family;
use krabton_engine::Instrument;

/// The height the desktop screens are drawn at (before scaling).
const NATURAL_H: f32 = 170.0;

impl App {
    /// Whether this device has a desktop screen to show.
    fn has_screen(&self, t: usize, link: Link) -> bool {
        let track = &self.project.tracks[t];
        match link {
            Link::Instrument => matches!(
                track.instrument,
                Some(
                    Instrument::Bassline { .. }
                        | Instrument::Drone { .. }
                        | Instrument::Waves { .. }
                        | Instrument::WindChimes { .. }
                        | Instrument::BeadsSynth { .. }
                        | Instrument::Birds { .. }
                        | Instrument::Nature { .. }
                        | Instrument::PanFlute { .. }
                        | Instrument::Melodica { .. }
                        | Instrument::Harmonica { .. }
                        | Instrument::Calculator { .. }
                        | Instrument::Console { .. }
                        | Instrument::Handheld { .. }
                        | Instrument::Micro { .. }
                        | Instrument::Warp { .. }
                        | Instrument::Hybrid { .. }
                        | Instrument::Oidos { .. }
                        | Instrument::Quartet { .. }
                        | Instrument::Guitar { .. }
                        | Instrument::Monk { .. }
                        | Instrument::Rings { .. }
                        | Instrument::Organ { .. }
                        | Instrument::Voice { .. }
                        | Instrument::Elements { .. }
                        | Instrument::Mallets { .. }
                        | Instrument::Strings { .. }
                        | Instrument::Winds { .. }
                        | Instrument::Facet { .. }
                )
            ),
            Link::Midi(k) => track.midi_devices.get(k).is_some_and(|d| d.kind.is_sequencer()),
            Link::Fx(k) => track.devices.get(k).is_some_and(|d| self.status().scopes.find(d.id).is_some()),
        }
    }

    /// Draws device `link` of track `t` live into `target`.
    pub(super) fn rack_screen(&mut self, ctx: &egui::Context, p: &Painter, key: (usize, usize), t: usize, link: Link, target: Rect, accent: Color32) {
        rect_fill_screen(p, target);
        if !self.has_screen(t, link) {
            self.fallback_screen(p, t, link, target, accent);
            return;
        }
        let status = self.status();
        let area = egui::Area::new(Id::new(("big_rack_screen", key))).order(egui::Order::Middle).fixed_pos(Pos2::ZERO).interactable(false).constrain(false);
        let track = self.project.tracks[t].clone();
        let scale = self.project.scale;
        let playing = self.is_playing();
        let shown = area.show(ctx, |ui| {
            let id = Id::new(("big_rack_inner", key));
            match link {
                Link::Instrument => {
                    let live = InstLive::read(&status, t);
                    let Some(inst) = track.instrument.as_ref() else { return };
                    let mut params = krabton_engine::modulation::instrument_params(inst).map(|p| p.to_vec()).unwrap_or_default();
                    let p = &mut params[..];
                    let h = NATURAL_H;
                    use crate::ui::{beads_view, engine_screens as es, inst_screen as is, nature_view as nv};
                    match inst {
                        Instrument::Bassline { .. } => {
                            es::bassline_screen(ui, id, p, &live, h);
                        }
                        Instrument::Drone { .. } => es::drone_screen(ui, id, p, &live, h),
                        Instrument::Waves { .. } => {
                            es::waves_screen(ui, id, p, &live, h);
                        }
                        Instrument::WindChimes { .. } => {
                            es::wind_chimes_screen(ui, id, p, &live, h);
                        }
                        Instrument::BeadsSynth { .. } => {
                            beads_view::beads_synth_screen(ui, id, p, &live, h);
                        }
                        Instrument::Birds { .. } => {
                            nv::birds_screen(ui, id, p, &live, h);
                        }
                        Instrument::Nature { .. } => {
                            nv::nature_screen(ui, id, p, &live, h);
                        }
                        Instrument::PanFlute { .. } => {
                            nv::pan_flute_screen(ui, id, p, &live, h);
                        }
                        Instrument::Melodica { .. } => {
                            crate::ui::reed_view::melodica_screen(ui, id, p, &live, h);
                        }
                        Instrument::Harmonica { .. } => {
                            crate::ui::reed_view::harmonica_screen(ui, id, p, &live, h);
                        }
                        Instrument::Calculator { .. } => {
                            crate::ui::chip_view::calculator_screen(ui, id, p, &live, h);
                        }
                        Instrument::Console { .. } => {
                            crate::ui::chip_view::chip_screen(ui, id, crate::ui::chip_view::Chip::Console, p, &live, h);
                        }
                        Instrument::Handheld { .. } => {
                            crate::ui::chip_view::chip_screen(ui, id, crate::ui::chip_view::Chip::Handheld, p, &live, h);
                        }
                        Instrument::Micro { .. } => {
                            crate::ui::chip_view::chip_screen(ui, id, crate::ui::chip_view::Chip::Micro, p, &live, h);
                        }
                        Instrument::Warp { .. } => {
                            crate::ui::warp_view::warp_screen(ui, id, p, &live, h);
                        }
                        Instrument::Hybrid { .. } => {
                            crate::ui::warp_view::hybrid_screen(ui, id, p, &live, h);
                        }
                        Instrument::Oidos { .. } => {
                            crate::ui::oidos_view::oidos_screen(ui, id, p, &live, h);
                        }
                        Instrument::Quartet { .. } => {
                            crate::ui::warp_view::quartet_screen(ui, id, p, &live, h);
                        }
                        Instrument::Guitar { .. } => {
                            es::guitar_screen(ui, id, p, &live, h);
                        }
                        Instrument::Monk { .. } => {
                            is::monk_screen(ui, id, p, &live, h);
                        }
                        Instrument::Rings { .. } => is::rings_screen(ui, p, &live, h),
                        Instrument::Organ { .. } => {
                            is::organ_screen(ui, id, p, &live, h);
                        }
                        Instrument::Voice { .. } => is::voice_screen(ui, id, p, &live, h),
                        Instrument::Elements { .. } => is::elements_screen(ui, id, p, &live, h),
                        Instrument::Mallets { .. } => is::stk_screen(ui, id, Family::Mallets, p, &live, h),
                        Instrument::Strings { .. } => is::stk_screen(ui, id, Family::Strings, p, &live, h),
                        Instrument::Winds { .. } => is::stk_screen(ui, id, Family::Winds, p, &live, h),
                        Instrument::Facet { .. } => {
                            is::facet_screen(ui, id, p, &live, h);
                        }
                        _ => {}
                    }
                }
                Link::Midi(k) => {
                    let Some(dev) = track.midi_devices.get(k) else { return };
                    let vis = if dev.enabled { status.seqs.read(dev.id) } else { None };
                    let kit = track.instrument.as_ref().map(|i| i.drum_notes()).unwrap_or(0);
                    let live = crate::ui::seq_view::Live { vis, playing, scale, kit };
                    let (mut params, mut data, mut lane) = (dev.params.clone(), dev.data.clone(), 0usize);
                    crate::ui::seq_view::editor(ui, id, dev.kind, &mut params, &mut data, live, &mut lane, rgb(track.color), NATURAL_H);
                }
                Link::Fx(k) => {
                    let Some(dev) = track.devices.get(k) else { return };
                    let scope = krabton_engine::dsp::effects::EffectKind::Scope;
                    let params = if dev.kind == scope { dev.params.clone() } else { krabton_engine::params::defaults(scope.specs()) };
                    let opts = crate::ui::scope::ScopeOpts::from_params(&params);
                    let (rect, _) = ui.allocate_exact_size(vec2(300.0, NATURAL_H), egui::Sense::hover());
                    crate::ui::scope::scope_view(ui, rect, id, status.scopes.find(dev.id), status.sample_rate.load(), &opts);
                }
            }
        });
        // Scale the screen into the card, in the middle.
        let nat = shown.response.rect;
        if nat.width() < 1.0 || nat.height() < 1.0 {
            return;
        }
        let s = (target.width() / nat.width()).min(target.height() / nat.height()).min(1.5);
        let size = nat.size() * s;
        let pos = target.center() - size * 0.5;
        ctx.set_transform_layer(shown.response.layer_id, TSTransform::new(pos.to_vec2() - nat.min.to_vec2() * s, s));
    }

    /// No screen of its own: the sound of the track and the notes it plays.
    fn fallback_screen(&self, p: &Painter, t: usize, link: Link, r: Rect, accent: Color32) {
        let status = self.status();
        let live = InstLive::read(&status, t);
        let wave = &live.wave[live.wave.len().saturating_sub(512)..];
        let gain = if matches!(link, Link::Fx(_)) { 1.0 } else { 1.0 / live.level().max(0.05).sqrt() };
        if wave.len() >= 2 {
            let pts: Vec<Pos2> = wave.iter().enumerate().map(|(i, v)| pos2(r.left() + i as f32 / (wave.len() - 1) as f32 * r.width(), r.center().y - (v * gain).clamp(-1.0, 1.0) * r.height() * 0.42)).collect();
            p.add(Shape::line(pts, Stroke::new(1.5_f32, accent)));
        }
        // The notes sounding, as dots across the keys.
        for n in (0..128u8).filter(|n| live.notes >> n & 1 == 1) {
            let x = r.left() + (n as f32 - 24.0).clamp(0.0, 84.0) / 84.0 * r.width();
            p.circle_filled(pos2(x, r.bottom() - 5.0), 3.0, Color32::WHITE);
        }
        // An effect: the track's level too.
        if let Link::Fx(_) = link {
            let (l, rr) = self.meters.get(t).copied().unwrap_or((0.0, 0.0));
            let h = (l.max(rr)).clamp(0.0, 1.0).sqrt() * r.height();
            p.rect_filled(Rect::from_min_max(pos2(r.right() - 6.0, r.bottom() - h), r.max), 2.0, shade(accent, 0.8));
        }
    }
}

fn rect_fill_screen(p: &Painter, r: Rect) {
    p.rect_filled(r, 6.0, Color32::from_rgb(0x0a, 0x0b, 0x0e));
}
