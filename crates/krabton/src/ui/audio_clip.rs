//! The clip view of an audio clip, as in Live: the waveform with its Start
//! and End markers, the loop brace, the warp markers and the transients on
//! a bars-and-beats ruler, and beside it the clip's warp and sound settings.
//!
//! On the ruler drag the flags and the brace (Alt: off the grid); drag the
//! ruler itself sideways to scroll and up or down to zoom, double-click it
//! to see everything. Double-click the strip under the ruler (or the
//! waveform) for a warp marker and drag it to put that moment on a beat.
//! Right-click the waveform to put the markers or the loop there, or to
//! crop the clip; right-click a warp marker to remove it or warp straight
//! from there. The wheel scrolls, Ctrl+wheel zooms.

use crate::app::{App, ClipSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, Id, Rect, RichText, Sense, Stroke, Ui};
use krabton_engine::{AudioClip, ClipContent, WarpMode};

const RULER_H: f32 = 28.0;
const STRIP_H: f32 = 14.0;
/// How near (points) a drag snaps to a transient or a warp marker.
const SNAP_PX: f64 = 7.0;
const MARKER: Color32 = Color32::from_rgb(0xF2, 0xC9, 0x4C);

/// What the waveform view keeps between frames.
#[derive(Default)]
pub struct WaveState {
    /// Points per clip beat, and the clip beat at the left edge.
    ppb: f32,
    left: f64,
    /// The clip the view was fitted to (another one is fitted afresh).
    fitted: Option<ClipSel>,
    drag: Option<WaveDrag>,
    /// Where the waveform was right-clicked, in clip beats.
    menu_beat: f64,
}

#[derive(Clone, Copy)]
enum WaveDrag {
    Start,
    End,
    LoopStart,
    LoopEnd,
    LoopMove { grab: f64 },
    Marker(usize),
    /// Dragging the ruler: sideways scrolls, up and down zooms.
    Ruler { x: f32, y: f32, ppb: f32, left: f64, anchor: f64 },
}

/// A beat as Live writes a position: bars.beats.sixteenths from 1.1.1.
pub fn bbs(beat: f64, bpb: f64) -> String {
    let per_bar = (bpb * 4.0).round().max(1.0) as i64;
    let q = (beat * 4.0).round() as i64;
    let bar = q.div_euclid(per_bar);
    let rest = q.rem_euclid(per_bar);
    format!("{}.{}.{}", bar + 1, rest / 4 + 1, rest % 4 + 1)
}

/// A length as Live writes it: bars.beats.sixteenths from 0.0.0.
pub fn bbs_len(beats: f64, bpb: f64) -> String {
    let per_bar = (bpb * 4.0).round().max(1.0) as i64;
    let q = (beats * 4.0).round().max(0.0) as i64;
    format!("{}.{}.{}", q / per_bar, (q % per_bar) / 4, q % 4)
}

/// Reads what [`bbs`] (or, for a length, [`bbs_len`]) writes; "3" or "3.2"
/// will do too.
pub fn parse_bbs(s: &str, bpb: f64, length: bool) -> Option<f64> {
    let parts: Vec<f64> = s.trim().split(['.', ':', ' ']).filter(|p| !p.is_empty()).map(|p| p.parse::<f64>()).collect::<Result<_, _>>().ok()?;
    let base = if length { 0.0 } else { 1.0 };
    let get = |i: usize| parts.get(i).map_or(0.0, |v| v - base);
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    Some(get(0) * bpb + if parts.len() > 1 { get(1) } else { 0.0 } + if parts.len() > 2 { get(2) * 0.25 } else { 0.0 })
}

/// The grid a drag snaps to at this zoom (points per beat).
fn grid_step(ppb: f32, bpb: f64) -> f64 {
    match ppb {
        p if p >= 120.0 => 0.25,
        p if p >= 50.0 => 0.5,
        p if p >= 14.0 => 1.0,
        p if p >= 3.0 => bpb,
        _ => bpb * 4.0,
    }
}

/// The clip beats of the sample's transients that lie between `lo` and `hi`.
fn transient_beats(a: &AudioClip, lo: f64, hi: f64) -> Vec<f64> {
    let src = a.source();
    let Some(an) = src.analysis_ready() else { return Vec::new() };
    let sr = src.sample_rate as f64;
    let (s0, s1) = (a.secs_at(lo) * sr, a.secs_at(hi) * sr);
    let i0 = an.transients.partition_point(|&t| (t as f64) < s0);
    an.transients[i0..].iter().take_while(|&&t| (t as f64) <= s1).map(|&t| a.beat_at(t as f64 / sr)).collect()
}

/// `b` moved onto a warp marker or transient nearby, else onto the grid
/// (`free`: left as it is).
fn snap(a: &AudioClip, b: f64, ppb: f32, bpb: f64, free: bool) -> f64 {
    if free {
        return b;
    }
    let tol = SNAP_PX / ppb.max(1e-3) as f64;
    let near = a.markers.iter().map(|m| m.0).chain(transient_beats(a, b - tol, b + tol)).min_by(|x, y| (x - b).abs().total_cmp(&(y - b).abs()));
    if let Some(n) = near.filter(|n| (n - b).abs() <= tol) {
        return n;
    }
    let g = grid_step(ppb, bpb);
    (b / g).round() * g
}

impl App {
    /// The clip beat the selected audio clip is playing, if it plays.
    fn audio_play_beat(&self, sel: ClipSel) -> Option<f64> {
        self.clip_play_pos(sel)?;
        let v = self.engine.status.tracks.get(sel.track())?.audio_pos.load();
        v.is_finite().then_some(v)
    }

    /// Changes the selected audio clip (and gives the clip its length).
    pub(crate) fn edit_audio(&mut self, sel: ClipSel, f: impl FnOnce(&mut AudioClip, &mut f64)) {
        if let Some(c) = self.clip_mut(sel) {
            if let ClipContent::Audio(a) = &mut c.content {
                f(a, &mut c.length);
            }
        }
    }

    /// The waveform, its ruler and its markers.
    pub(crate) fn ui_audio_clip(&mut self, ui: &mut Ui, area: Rect, sel: ClipSel) {
        let Some(clip) = self.clip_ref(sel) else { return };
        let ClipContent::Audio(a) = &clip.content else { return };
        let a = a.clone();
        let len = clip.length.max(1.0 / 64.0);
        let base = rgb(clip.color);
        let bpb = self.project.beats_per_bar_f();
        let free = ui.input(|i| i.modifiers.alt);
        let p = ui.painter().with_clip_rect(area);
        rect_fill(&p, area, 0.0, BG);
        let inner = area.shrink2(vec2(8.0, 4.0));
        let ruler = Rect::from_min_size(inner.min, vec2(inner.width(), RULER_H));
        let strip = Rect::from_min_size(pos2(inner.left(), ruler.bottom()), vec2(inner.width(), STRIP_H));
        let wave = Rect::from_min_max(pos2(inner.left(), strip.bottom() + 2.0), inner.max);
        let (s_lo, s_hi) = (a.sample_start_beat(), a.sample_end_beat());
        let loop_end = a.loop_start + len;
        let end = a.end_beat();

        // A new clip is shown whole.
        if self.wave.fitted != Some(sel) || self.wave.ppb <= 0.0 {
            let lo = s_lo.min(a.start).min(if a.looping { a.loop_start } else { a.start });
            let hi = s_hi.max(if a.looping { loop_end } else { end });
            let span = (hi - lo).max(0.25);
            self.wave.ppb = (inner.width() as f64 / (span * 1.04)) as f32;
            self.wave.left = lo - span * 0.02;
            self.wave.fitted = Some(sel);
        }
        let (ppb, left) = (self.wave.ppb, self.wave.left);
        let x_of = |b: f64| inner.left() + ((b - left) * ppb as f64) as f32;
        let b_of = |x: f32| left + (x - inner.left()) as f64 / ppb as f64;
        let first = b_of(inner.left());
        let last = b_of(inner.right());

        // What plays: the loop (and the way in from the Start marker), or Start to End.
        let (on0, on1) = if a.looping { (a.start.min(a.loop_start), loop_end) } else { (a.start, end) };
        if a.looping {
            let lr = Rect::from_min_max(pos2(x_of(a.loop_start), wave.top()), pos2(x_of(loop_end), wave.bottom()));
            p.rect_filled(lr.intersect(wave), 0.0, shade(PANEL2, 1.1));
        } else {
            let lr = Rect::from_min_max(pos2(x_of(a.start), wave.top()), pos2(x_of(end), wave.bottom()));
            p.rect_filled(lr.intersect(wave), 0.0, shade(PANEL2, 1.05));
        }

        // The ruler: bars, beats and (zoomed in) sixteenths.
        rect_fill(&p, ruler, 0.0, PANEL);
        let step = grid_step(ppb, bpb).min(1.0).max(if ppb < 14.0 { bpb } else { 0.0 });
        let label_every = if ppb * bpb as f32 >= 46.0 { bpb } else if ppb * bpb as f32 >= 12.0 { bpb * 4.0 } else { bpb * 16.0 };
        let mut b = (first / step).floor() * step;
        let mut guard = 0;
        while b <= last && guard < 4000 {
            let x = x_of(b);
            let bar = (b / bpb).fract().abs() < 1e-9 || ((b / bpb).fract() - 1.0).abs() < 1e-9;
            let col = if bar { LINE } else { LINE_SOFT };
            p.line_segment([pos2(x, wave.top()), pos2(x, wave.bottom())], Stroke::new(1.0_f32, col));
            p.line_segment([pos2(x, ruler.bottom() - if bar { 8.0 } else { 4.0 }), pos2(x, ruler.bottom())], Stroke::new(1.0_f32, TEXT_DIM));
            if ((b / label_every).round() * label_every - b).abs() < 1e-9 {
                p.text(pos2(x + 2.0, ruler.top() + 7.0), Align2::LEFT_TOP, bbs(b, bpb).trim_end_matches(".1.1").to_string(), small(), TEXT_DIM);
            } else if !bar && ppb >= 60.0 && (b.fract()).abs() < 1e-9 {
                p.text(pos2(x + 2.0, ruler.top() + 7.0), Align2::LEFT_TOP, bbs(b, bpb).trim_end_matches(".1").to_string(), small(), shade(TEXT_DIM, 0.8));
            }
            b += step;
            guard += 1;
        }

        // The waveform, warped: each column reads the stretch of sample its beats hold.
        let src = a.source();
        let ssr = src.sample_rate as f64;
        let peaks = &src.peaks;
        let mid = wave.center().y;
        let half = wave.height() * 0.46;
        p.line_segment([pos2(wave.left(), mid), pos2(wave.right(), mid)], Stroke::new(1.0_f32, LINE_SOFT));
        for xi in 0..wave.width() as i32 {
            let x = wave.left() + xi as f32;
            let (b0, b1) = (b_of(x), b_of(x + 1.0));
            let (f0, f1) = (a.secs_at(b0) * ssr, a.secs_at(b1) * ssr);
            if f1 < 0.0 || f0 >= src.frames() as f64 {
                continue;
            }
            let i0 = (f0.max(0.0) as usize) / krabton_engine::sample::PEAK_BLOCK;
            let i1 = ((f1.max(0.0) as usize) / krabton_engine::sample::PEAK_BLOCK).max(i0 + 1).min(peaks.len());
            let (mut mn, mut mx) = (0.0f32, 0.0f32);
            for pk in peaks.get(i0..i1).unwrap_or(&[]) {
                mn = mn.min(pk.0);
                mx = mx.max(pk.1);
            }
            let inside = b0 >= on0 && b0 < on1;
            let c = if inside { base } else { shade(base, 0.38) };
            p.line_segment([pos2(x, mid - mx * half), pos2(x, mid - mn * half + 0.5)], Stroke::new(1.0_f32, c));
        }
        let gain = 10f32.powf(a.gain_db / 20.0);
        if gain > 1.0 && gain.is_finite() {
            p.text(pos2(wave.right() - 4.0, wave.top() + 2.0), Align2::RIGHT_TOP, format!("{:+.1} dB", a.gain_db), small(), TEXT_DIM);
        }

        // The transients (ticks) and the warp markers in the strip.
        rect_fill(&p, strip, 0.0, shade(PANEL, 0.85));
        for t in transient_beats(&a, first, last) {
            let x = x_of(t);
            p.line_segment([pos2(x, strip.bottom() - 5.0), pos2(x, strip.bottom())], Stroke::new(1.0_f32, TEXT_DIM));
        }
        let warp_col = if a.warp { MARKER } else { shade(MARKER, 0.45) };
        let mut marker_hit = false;
        for (i, m) in a.markers.iter().enumerate() {
            let x = x_of(m.0);
            if x < inner.left() - 6.0 || x > inner.right() + 6.0 {
                continue;
            }
            p.line_segment([pos2(x, strip.top()), pos2(x, wave.bottom())], Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0xF2, 0xC9, 0x4C, 90)));
            let h = Rect::from_center_size(pos2(x, strip.center().y), vec2(9.0, STRIP_H - 2.0));
            p.add(egui::Shape::convex_polygon(vec![pos2(x - 4.5, h.top()), pos2(x + 4.5, h.top()), pos2(x + 4.5, h.bottom() - 4.0), pos2(x, h.bottom()), pos2(x - 4.5, h.bottom() - 4.0)], warp_col, Stroke::NONE));
            let resp = ui.interact(h, Id::new(("warp_marker", i)), Sense::click_and_drag()).on_hover_text(format!("Warp marker at {} (sample {:.3} s): drag it onto its beat, double-click to remove, right-click for more", bbs(m.0, bpb), m.1));
            if resp.hovered() || resp.dragged() {
                marker_hit = true;
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            }
            if resp.drag_started() {
                self.wave.drag = Some(WaveDrag::Marker(i));
            }
            if resp.double_clicked() {
                self.edit_audio(sel, |a, _| {
                    if i < a.markers.len() {
                        a.markers.remove(i);
                    }
                });
            }
            let project_bpm = self.project.bpm;
            let found = a.sample.analysis_ready().and_then(|an| an.tempo).map(|t| t.bpm);
            resp.context_menu(|ui| {
                if ui.button("Delete Warp Marker").clicked() {
                    self.edit_audio(sel, |a, _| {
                        if i < a.markers.len() {
                            a.markers.remove(i);
                        }
                    });
                    ui.close();
                }
                if let Some(bpm) = found {
                    if ui.button(format!("Warp From Here (Straight, {bpm:.2} BPM)")).on_hover_text("The markers after this one go; the rest runs at the tempo found in the sample").clicked() {
                        self.edit_audio(sel, |a, _| a.warp_from(i, bpm));
                        ui.close();
                    }
                }
                if ui.button(format!("Warp From Here (Start at {project_bpm:.2} BPM)")).on_hover_text("The markers after this one go; the rest plays at its own speed at the song's tempo").clicked() {
                    self.edit_audio(sel, |a, _| a.warp_from(i, project_bpm));
                    ui.close();
                }
                ui.separator();
                if ui.button("Clear All Warp Markers").clicked() {
                    self.edit_audio(sel, |a, _| a.markers.clear());
                    ui.close();
                }
            });
        }

        // The loop brace on the ruler, and the Start and End flags under it.
        let brace = Rect::from_min_max(pos2(x_of(a.loop_start), ruler.top() + 1.0), pos2(x_of(loop_end), ruler.top() + 6.0));
        if a.looping {
            p.rect_filled(brace, 1.5, accent_now());
        } else {
            p.rect_filled(brace, 1.5, shade(accent_now(), 0.35));
        }
        let flag = |p: &egui::Painter, x: f32, right: bool, c: Color32| {
            let (y0, y1) = (ruler.bottom() - 11.0, ruler.bottom());
            let pts = if right { vec![pos2(x, y0), pos2(x + 8.0, y0 + 5.5), pos2(x, y1)] } else { vec![pos2(x, y0), pos2(x, y1), pos2(x - 8.0, y0 + 5.5)] };
            p.add(egui::Shape::convex_polygon(pts, c, Stroke::NONE));
            p.line_segment([pos2(x, ruler.bottom()), pos2(x, wave.bottom())], Stroke::new(1.0_f32, c));
        };
        flag(&p, x_of(a.start), true, Color32::from_rgb(0x8E, 0xE0, 0x6A));
        if !a.looping {
            flag(&p, x_of(end), false, Color32::from_rgb(0xE0, 0x6A, 0x6A));
        }

        // Interaction: the flags and the brace first, then the ruler itself.
        let hot = |x: f32, r: Rect| Rect::from_min_max(pos2(x - 6.0, r.top()), pos2(x + 6.0, r.bottom()));
        let start_r = Rect::from_min_max(pos2(x_of(a.start) - 2.0, ruler.bottom() - 12.0), pos2(x_of(a.start) + 9.0, ruler.bottom()));
        let end_r = Rect::from_min_max(pos2(x_of(end) - 9.0, ruler.bottom() - 12.0), pos2(x_of(end) + 2.0, ruler.bottom()));
        let brace_hot = brace.expand2(vec2(0.0, 2.0));
        let ruler_resp = ui.interact(ruler, Id::new("wave_ruler"), Sense::click_and_drag());
        let parts = [
            (WaveDrag::Start, start_r, "Start marker: where playing begins (drag; Alt: off the grid)"),
            (WaveDrag::End, end_r, "End marker: where it stops (when it does not loop)"),
            (WaveDrag::LoopStart, hot(brace.left(), brace_hot), "The loop's start"),
            (WaveDrag::LoopEnd, hot(brace.right(), brace_hot), "The loop's end"),
            (WaveDrag::LoopMove { grab: 0.0 }, brace_hot.shrink2(vec2(6.0, 0.0)), "The loop brace: drag to move it, its ends to resize it"),
        ];
        for (k, (kind, r, tip)) in parts.into_iter().enumerate() {
            if matches!(kind, WaveDrag::End) && a.looping {
                continue;
            }
            if r.width() <= 0.0 {
                continue;
            }
            let resp = ui.interact(r.intersect(ruler), Id::new(("wave_part", k)), Sense::drag()).on_hover_text(tip);
            if resp.hovered() || resp.dragged() {
                ui.ctx().set_cursor_icon(if matches!(kind, WaveDrag::LoopMove { .. }) { CursorIcon::Grab } else { CursorIcon::ResizeHorizontal });
            }
            if resp.drag_started() {
                let at = resp.interact_pointer_pos().map_or(a.loop_start, |q| b_of(q.x));
                self.wave.drag = Some(match kind {
                    WaveDrag::LoopMove { .. } => WaveDrag::LoopMove { grab: at - a.loop_start },
                    k => k,
                });
            }
        }
        if ruler_resp.drag_started() && self.wave.drag.is_none() {
            if let Some(q) = ruler_resp.interact_pointer_pos() {
                self.wave.drag = Some(WaveDrag::Ruler { x: q.x, y: q.y, ppb, left, anchor: b_of(q.x) });
            }
        }
        if ruler_resp.double_clicked() {
            self.wave.fitted = None;
        }
        if ruler_resp.hovered() && self.wave.drag.is_none() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
        }

        // The strip and the waveform: double-click for a warp marker, right-click for the markers.
        let wresp = ui.interact(Rect::from_min_max(strip.min, wave.max), Id::new("wave_body"), Sense::click_and_drag());
        if wresp.double_clicked() && !marker_hit {
            if let Some(q) = wresp.interact_pointer_pos() {
                let at = snap(&a, b_of(q.x), ppb, bpb, true);
                let tol = SNAP_PX / ppb as f64;
                let at = transient_beats(&a, at - tol, at + tol).into_iter().min_by(|x, y| (x - at).abs().total_cmp(&(y - at).abs())).unwrap_or(at);
                self.edit_audio(sel, |a, _| {
                    a.add_marker(at);
                });
            }
        }
        if wresp.drag_started() && self.wave.drag.is_none() {
            if let Some(q) = wresp.interact_pointer_pos() {
                self.wave.drag = Some(WaveDrag::Ruler { x: q.x, y: q.y, ppb, left, anchor: b_of(q.x) });
            }
        }
        if wresp.secondary_clicked() {
            if let Some(q) = wresp.interact_pointer_pos() {
                self.wave.menu_beat = snap(&a, b_of(q.x), ppb, bpb, free);
            }
        }
        let at = self.wave.menu_beat;
        wresp.context_menu(|ui| {
            ui.label(RichText::new(bbs(at, bpb)).small().color(TEXT_DIM));
            if ui.button("Set Start Marker Here").clicked() {
                self.edit_audio(sel, |a, len| {
                    a.start = at;
                    if !a.looping && a.end_beat() <= at + 1.0 / 16.0 {
                        a.end = None;
                    }
                    if !a.looping {
                        *len = (a.end_beat() - a.start).max(1.0 / 16.0);
                    }
                });
                ui.close();
            }
            if ui.button("Set End Marker Here").on_hover_text("Plays once from the Start marker to here (loop off)").clicked() {
                self.edit_audio(sel, |a, len| {
                    if at > a.start + 1.0 / 16.0 {
                        a.end = Some(at);
                        a.looping = false;
                        *len = at - a.start;
                    }
                });
                ui.close();
            }
            if ui.button("Set Loop Here").on_hover_text("The loop starts here (its length stays) and plays from here").clicked() {
                self.edit_audio(sel, |a, _| {
                    a.loop_start = at;
                    a.start = at;
                    a.looping = true;
                });
                ui.close();
            }
            if a.warp && ui.button("Add Warp Marker Here").clicked() {
                self.edit_audio(sel, |a, _| {
                    a.add_marker(at);
                });
                ui.close();
            }
            ui.separator();
            if ui.button("Crop Clip").on_hover_text("Keep only the audio that plays: a new file beside the sample, without what lies outside the markers").clicked() {
                self.crop_audio_clip(sel);
                ui.close();
            }
            if ui.button("Slice to New MIDI Track...").on_hover_text("Cut the clip at its hits or on the beat: a Drum Rack of the slices and a clip that plays them, on a new track").clicked() {
                self.open_slice_dialog(sel);
                ui.close();
            }
            self.convert_menu(ui, sel);
        });
        if wresp.hovered() {
            let (scroll, ctrl) = ui.input(|i| (i.smooth_scroll_delta, i.modifiers.ctrl || i.modifiers.command));
            if ctrl && scroll.y != 0.0 {
                if let Some(h) = wresp.hover_pos() {
                    let anchor = b_of(h.x);
                    let np = (ppb * (scroll.y * 0.005).exp()).clamp(0.5, 4000.0);
                    self.wave.ppb = np;
                    self.wave.left = anchor - (h.x - inner.left()) as f64 / np as f64;
                }
            } else if scroll.x != 0.0 || scroll.y != 0.0 {
                let d = if scroll.x != 0.0 { scroll.x } else { scroll.y };
                self.wave.left -= d as f64 / ppb as f64;
            }
        }

        // A drag in progress.
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let down = ui.input(|i| i.pointer.primary_down());
        if let (Some(d), Some(q)) = (self.wave.drag, pointer) {
            let at = snap(&a, b_of(q.x), ppb, bpb, free);
            let min = 1.0 / 16.0;
            match d {
                WaveDrag::Start => self.edit_audio(sel, |a, len| {
                    let lim = if a.looping { a.loop_start + *len - min } else { a.end_beat() - min };
                    a.start = at.min(lim);
                    if !a.looping {
                        *len = a.end_beat() - a.start;
                    }
                }),
                WaveDrag::End => self.edit_audio(sel, |a, len| {
                    a.end = Some(at.max(a.start + min));
                    *len = a.end_beat() - a.start;
                }),
                WaveDrag::LoopStart => self.edit_audio(sel, |a, len| {
                    let le = a.loop_start + *len;
                    let ns = at.min(le - min);
                    if (a.start - a.loop_start).abs() < 1e-9 {
                        a.start = ns;
                    }
                    a.loop_start = ns;
                    *len = le - ns;
                }),
                WaveDrag::LoopEnd => self.edit_audio(sel, |a, len| *len = (at - a.loop_start).max(min)),
                WaveDrag::LoopMove { grab } => self.edit_audio(sel, |a, _| {
                    let ns = snap(a, b_of(q.x) - grab, ppb, bpb, free);
                    if (a.start - a.loop_start).abs() < 1e-9 {
                        a.start = ns;
                    }
                    a.loop_start = ns;
                }),
                WaveDrag::Marker(i) => self.edit_audio(sel, |a, _| a.move_marker(i, at)),
                WaveDrag::Ruler { x, y, ppb: p0, left: l0, anchor } => {
                    let np = (p0 * ((y - q.y) * 0.01).exp()).clamp(0.5, 4000.0);
                    self.wave.ppb = np;
                    self.wave.left = anchor - (x - inner.left()) as f64 / np as f64 - (q.x - x) as f64 / np as f64;
                    let _ = l0;
                }
            }
            if !down {
                self.wave.drag = None;
            }
        } else if !down {
            self.wave.drag = None;
        }

        // The playhead.
        if let Some(pos) = self.audio_play_beat(sel) {
            let x = x_of(pos);
            if x >= inner.left() && x <= inner.right() {
                p.line_segment([pos2(x, ruler.top()), pos2(x, wave.bottom())], Stroke::new(1.5_f32, Color32::WHITE));
            }
        }
        if !a.warp {
            p.text(pos2(wave.left() + 4.0, wave.bottom() - 3.0), Align2::LEFT_BOTTOM, "Warp off: plays at its own speed", small(), TEXT_DIM);
        }
    }

    /// Keeps only the audio of an audio clip that plays: a new WAV beside
    /// the sample, the clip on it with its beats where they were (Live's
    /// Crop Sample).
    pub(crate) fn crop_audio_clip(&mut self, sel: ClipSel) {
        let Some(clip) = self.clip_ref(sel) else { return };
        let ClipContent::Audio(a) = &clip.content else { return };
        if a.reverse {
            self.toast("Crop a clip that plays forwards (switch Reverse off first)");
            return;
        }
        let len = clip.length;
        let (lo, hi) = if a.looping { (a.start.min(a.loop_start), a.loop_start + len) } else { (a.start, a.end_beat()) };
        let s = &a.sample;
        let sr = s.sample_rate as f64;
        let f0 = (a.secs_at(lo) * sr).floor().max(0.0) as usize;
        let f1 = ((a.secs_at(hi) * sr).ceil() as usize).min(s.frames());
        if f1 <= f0 + 16 {
            self.toast("Nothing of the sample plays between the markers");
            return;
        }
        let dir = s.path.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf())).or_else(|| self.file_path.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf()))).unwrap_or_else(std::env::temp_dir);
        let stem = s.path.as_ref().and_then(|p| p.file_stem()).map(|x| x.to_string_lossy().to_string()).unwrap_or_else(|| s.name.clone());
        let mut path = dir.join(format!("{stem} Crop.wav"));
        let mut k = 2;
        while path.exists() {
            path = dir.join(format!("{stem} Crop {k}.wav"));
            k += 1;
        }
        let audio: Vec<f32> = (f0..f1).flat_map(|i| [s.left[i], s.right[i]]).collect();
        if let Err(e) = krabton_engine::render::write_wav(&path, s.sample_rate, &audio) {
            self.toast(format!("Crop failed: {e}"));
            return;
        }
        let new = match krabton_engine::SampleRef::load(&path) {
            Ok(n) => n,
            Err(e) => {
                self.toast(format!("Crop failed: {e}"));
                return;
            }
        };
        new.analysis();
        let cut = f0 as f64 / sr;
        self.edit_audio(sel, |a, _| {
            // The beats stay where they were: the seconds shift by what was cut.
            if a.markers.is_empty() {
                a.markers.push((a.beat_at(cut), cut));
            }
            for m in &mut a.markers {
                m.1 -= cut;
            }
            a.sample = new;
        });
        self.toast(format!("Cropped to {}", path.display()));
    }

    /// The clip view's settings for an audio clip: its markers and loop,
    /// its warping, and how it sounds.
    pub(crate) fn audio_sidebar(&mut self, ui: &mut Ui, sel: ClipSel, a: &AudioClip, len: f64) {
        let bpb = self.project.beats_per_bar_f();
        let small_label = |ui: &mut Ui, s: &str| {
            ui.label(RichText::new(s).small().color(TEXT_DIM));
        };
        let toggle = |ui: &mut Ui, label: &str, on: bool, tip: &str| -> bool {
            ui.add(egui::Button::new(RichText::new(label).color(if on { TEXT_DARK } else { TEXT })).fill(if on { accent_now() } else { PANEL3 })).on_hover_text(tip).clicked()
        };
        // A field in bars.beats.sixteenths.
        let field = |ui: &mut Ui, v: &mut f64, length: bool| -> bool {
            let mut x = *v;
            let r = ui.add(
                egui::DragValue::new(&mut x)
                    .speed(0.05)
                    .custom_formatter(move |v, _| if length { bbs_len(v, bpb) } else { bbs(v, bpb) })
                    .custom_parser(move |s| parse_bbs(s, bpb, length)),
            );
            if r.changed() {
                let free = ui.input(|i| i.modifiers.alt);
                *v = if free { x } else { (x * 16.0).round() / 16.0 };
                return true;
            }
            false
        };

        ui.label(RichText::new(format!("{} · {} Hz · {:.2} s", a.sample.name, a.sample.sample_rate, a.sample.duration())).small().color(TEXT_DIM));

        // Start, End and the loop.
        ui.horizontal(|ui| {
            if toggle(ui, "Loop", a.looping, "Loop: round and round the brace; off, it plays once from the Start to the End marker") {
                self.edit_audio(sel, |a, len| {
                    a.looping = !a.looping;
                    if !a.looping && a.end.is_none() {
                        a.end = Some((a.loop_start + *len).min(a.sample_end_beat()));
                    }
                });
            }
            small_label(ui, "Start");
            let mut s = a.start;
            if field(ui, &mut s, false) {
                self.edit_audio(sel, |a, len| {
                    a.start = s;
                    if !a.looping {
                        *len = (a.end_beat() - a.start).max(1.0 / 16.0);
                    }
                });
            }
        });
        ui.horizontal(|ui| {
            if a.looping {
                small_label(ui, "Position");
                let mut s = a.loop_start;
                if field(ui, &mut s, false) {
                    self.edit_audio(sel, |a, _| {
                        if (a.start - a.loop_start).abs() < 1e-9 {
                            a.start = s;
                        }
                        a.loop_start = s;
                    });
                }
                small_label(ui, "Length");
                let mut l = len;
                if field(ui, &mut l, true) {
                    self.edit_audio(sel, |_, len| *len = l.max(1.0 / 16.0));
                }
            } else {
                small_label(ui, "End");
                let mut e = a.end_beat();
                if field(ui, &mut e, false) {
                    self.edit_audio(sel, |a, len| {
                        a.end = Some(e.max(a.start + 1.0 / 16.0));
                        *len = a.end_beat() - a.start;
                    });
                }
            }
        });

        // Warping.
        ui.separator();
        ui.horizontal(|ui| {
            if toggle(ui, "Warp", a.warp, "Warp: the clip stays on the beat at any tempo. Off: it plays at its own speed") {
                self.edit_audio(sel, |a, _| a.warp = !a.warp);
            }
            let r = ui.add_enabled(a.warp, egui::Button::new(a.mode.name()).fill(PANEL3)).on_hover_text(format!("{} (click for the next mode, right-click for the one before)", a.mode.about()));
            if r.clicked() {
                self.edit_audio(sel, |a, _| a.mode = a.mode.step(1));
            }
            if r.secondary_clicked() {
                self.edit_audio(sel, |a, _| a.mode = a.mode.step(-1));
            }
        });
        if a.warp {
            ui.horizontal(|ui| match a.mode {
                WarpMode::Beats => {
                    let r = ui.button(a.preserve.name()).on_hover_text("Preserve: cut at the transients, or on a grid (click round, right-click back)");
                    if r.clicked() {
                        self.edit_audio(sel, |a, _| a.preserve = a.preserve.step(1));
                    }
                    if r.secondary_clicked() {
                        self.edit_audio(sel, |a, _| a.preserve = a.preserve.step(-1));
                    }
                    let r = ui.button(a.transient_loop.name().trim_start_matches("Loop ")).on_hover_text("When a piece ends before the next (a slower tempo): it stops, or its tail loops forwards or back and forth");
                    if r.clicked() {
                        self.edit_audio(sel, |a, _| a.transient_loop = a.transient_loop.step(1));
                    }
                    if r.secondary_clicked() {
                        self.edit_audio(sel, |a, _| a.transient_loop = a.transient_loop.step(-1));
                    }
                    let mut env = a.transient_env * 100.0;
                    if ui.add(egui::DragValue::new(&mut env).range(0.0..=100.0).speed(0.5).fixed_decimals(0)).on_hover_text("Transient envelope: 100 lets each piece ring, lower fades it out sooner").changed() {
                        self.edit_audio(sel, |a, _| a.transient_env = env / 100.0);
                    }
                }
                WarpMode::Tones | WarpMode::Texture => {
                    small_label(ui, "Grain");
                    let mut g = a.grain * 100.0;
                    if ui.add(egui::DragValue::new(&mut g).range(0.0..=100.0).speed(0.5).fixed_decimals(0)).on_hover_text("Grain size: small for high, quick sounds, big for low ones and pads").changed() {
                        self.edit_audio(sel, |a, _| a.grain = g / 100.0);
                    }
                    if a.mode == WarpMode::Texture {
                        small_label(ui, "Flux");
                        let mut f = a.flux * 100.0;
                        if ui.add(egui::DragValue::new(&mut f).range(0.0..=100.0).speed(0.5).fixed_decimals(0)).on_hover_text("How far the grains wander").changed() {
                            self.edit_audio(sel, |a, _| a.flux = f / 100.0);
                        }
                    }
                }
                WarpMode::Complex => small_label(ui, "Grains between the hits, for songs"),
                WarpMode::RePitch => small_label(ui, "Speed and pitch follow the tempo"),
            });
        }
        ui.horizontal(|ui| {
            small_label(ui, "Seg. BPM");
            let start = a.start;
            let mut bpm = a.bpm_at(start);
            let segmented = a.markers.len() >= 2 && (bpm - a.bpm).abs() > 1e-6;
            if ui.add_enabled(!segmented, egui::DragValue::new(&mut bpm).range(20.0..=999.0).speed(0.05).fixed_decimals(2)).on_hover_text("The tempo of the material (between warp markers it follows from them)").changed() {
                self.edit_audio(sel, |a, _| a.bpm = bpm);
            }
            if ui.small_button("x2").on_hover_text("Twice the tempo: as many beats again").clicked() {
                self.edit_audio(sel, |a, _| a.scale_tempo(2.0));
            }
            if ui.small_button("/2").on_hover_text("Half the tempo").clicked() {
                self.edit_audio(sel, |a, _| a.scale_tempo(0.5));
            }
        });
        ui.horizontal(|ui| {
            let tempo = a.sample.analysis_ready().and_then(|an| an.tempo);
            let tip = match tempo {
                Some(t) => format!("Warp it to the tempo found in it ({:.2} BPM), a marker on its first downbeat", t.bpm),
                None => "No steady beat was found in it".to_string(),
            };
            if ui.add_enabled(tempo.is_some(), egui::Button::new("Auto-Warp")).on_hover_text(tip).clicked() {
                if let Some(t) = tempo {
                    self.edit_audio(sel, |a, _| {
                        a.bpm = t.bpm;
                        a.markers = vec![(0.0, if a.reverse { a.sample.duration() - t.downbeat } else { t.downbeat })];
                        a.warp = true;
                    });
                }
            }
            if ui.button("Fit length").on_hover_text("The loop (or Start to End) over the whole sample").clicked() {
                self.edit_audio(sel, |a, len| {
                    let (lo, hi) = (a.sample_start_beat(), a.sample_end_beat());
                    a.start = lo;
                    a.loop_start = lo;
                    a.end = None;
                    *len = (hi - lo).max(1.0 / 16.0);
                });
            }
        });

        // How it sounds.
        ui.separator();
        ui.horizontal(|ui| {
            let on = a.pitch_applies();
            small_label(ui, "Transpose");
            let mut t = a.transpose;
            if ui.add_enabled(on, egui::DragValue::new(&mut t).range(-48.0..=48.0).speed(0.1).fixed_decimals(0).suffix(" st")).on_hover_text("Pitch in semitones: warped it keeps the tempo; unwarped it plays faster or slower (not in Re-Pitch)").changed() {
                self.edit_audio(sel, |a, _| a.transpose = t.round());
            }
            let mut d = a.detune;
            if ui.add_enabled(on, egui::DragValue::new(&mut d).range(-50.0..=50.0).speed(0.2).fixed_decimals(0).suffix(" ct")).on_hover_text("Detune in cents").changed() {
                self.edit_audio(sel, |a, _| a.detune = d);
            }
        });
        ui.horizontal(|ui| {
            small_label(ui, "Gain");
            let mut g = a.gain_db;
            if ui.add(egui::DragValue::new(&mut g).range(-48.0..=24.0).speed(0.1).fixed_decimals(1).suffix(" dB")).changed() {
                self.edit_audio(sel, |a, _| a.gain_db = g);
            }
            if ui.small_button("Normalize").on_hover_text("The gain that brings its loudest peak to 0 dB").clicked() {
                let peak = a.sample.peak();
                if peak > 1e-6 {
                    self.edit_audio(sel, |a, _| a.gain_db = (-20.0 * peak.log10()).clamp(-48.0, 24.0));
                }
            }
        });
        ui.horizontal(|ui| {
            small_label(ui, "Pan");
            let mut p = a.pan * 50.0;
            let fmt = |v: f64, _: std::ops::RangeInclusive<usize>| match v.round() as i64 {
                0 => "C".to_string(),
                n if n < 0 => format!("{}L", -n),
                n => format!("{n}R"),
            };
            if ui.add(egui::DragValue::new(&mut p).range(-50.0..=50.0).speed(0.3).custom_formatter(fmt)).on_hover_text("Left or right (double-click to type; 0 is the centre)").changed() {
                self.edit_audio(sel, |a, _| a.pan = (p / 50.0).clamp(-1.0, 1.0));
            }
            if toggle(ui, "Reverse", a.reverse, "Play it backwards (the markers and the loop are mirrored)") {
                let on = !a.reverse;
                self.edit_audio(sel, |a, len| a.set_reverse(on, *len));
            }
        });
        // Fades: an arrangement clip's own, or a session clip's from its start and to its end.
        let bpm = self.project.bpm.max(1.0);
        let to_ms = |beats: f64| beats * 60_000.0 / bpm;
        let to_beats = |ms: f64| ms * bpm / 60_000.0;
        let (fi, fo) = match sel {
            ClipSel::Arr { track, id } => self.project.tracks.get(track).and_then(|t| t.arrangement.iter().find(|x| x.id == id)).map_or((0.0, 0.0), |x| (x.fade_in, x.fade_out)),
            ClipSel::Slot { .. } => (a.fade_in, a.fade_out),
        };
        ui.horizontal(|ui| {
            small_label(ui, "Fade");
            let (mut i, mut o) = (to_ms(fi), to_ms(fo));
            let ci = ui.add(egui::DragValue::new(&mut i).range(0.0..=60_000.0).speed(2.0).fixed_decimals(0).suffix(" ms")).on_hover_text("Fade in").changed();
            let co = ui.add(egui::DragValue::new(&mut o).range(0.0..=60_000.0).speed(2.0).fixed_decimals(0).suffix(" ms")).on_hover_text(if matches!(sel, ClipSel::Slot { .. }) { "Fade out before the End marker (when it does not loop)" } else { "Fade out" }).changed();
            if ci || co {
                let (bi, bo) = (to_beats(i), to_beats(o));
                match sel {
                    ClipSel::Arr { track, id } => {
                        if let Some(x) = self.project.tracks.get_mut(track).and_then(|t| t.arrangement.iter_mut().find(|x| x.id == id)) {
                            x.fade_in = bi.min(x.length);
                            x.fade_out = bo.min((x.length - x.fade_in).max(0.0));
                        }
                    }
                    ClipSel::Slot { .. } => self.edit_audio(sel, |a, _| {
                        a.fade_in = bi;
                        a.fade_out = bo;
                    }),
                }
            }
            let mut c = a.fade_curve * 100.0;
            if ui.add(egui::DragValue::new(&mut c).range(-100.0..=100.0).speed(0.5).fixed_decimals(0)).on_hover_text("The fades' curve: 0 equal power, up to 100 quicker, down to -100 slower").changed() {
                self.edit_audio(sel, |a, _| a.fade_curve = c / 100.0);
            }
        });
        ui.horizontal(|ui| {
            if toggle(ui, "Clip Fade", a.declick, "A millisecond fade wherever the audio is cut, so it never clicks") {
                self.edit_audio(sel, |a, _| a.declick = !a.declick);
            }
            small_label(ui, "Loop xfade");
            let mut x = a.loop_xfade;
            if ui.add_enabled(a.looping, egui::DragValue::new(&mut x).range(0.0..=500.0).speed(0.5).fixed_decimals(0).suffix(" ms")).on_hover_text("Where the loop goes round, its end rings on into its start for this long").changed() {
                self.edit_audio(sel, |a, _| a.loop_xfade = x);
            }
        });
        ui.label(
            RichText::new("Drag the flags and the loop brace (Alt: no snap). Double-click under the ruler for a warp marker; right-click the waveform or a marker for more.")
                .small()
                .color(TEXT_DIM),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_read_and_write_as_bars_beats_sixteenths() {
        assert_eq!(bbs(0.0, 4.0), "1.1.1");
        assert_eq!(bbs(5.25, 4.0), "2.2.2");
        assert_eq!(bbs(-4.0, 4.0), "0.1.1");
        assert_eq!(bbs_len(6.5, 4.0), "1.2.2");
        assert_eq!(parse_bbs("2.2.2", 4.0, false), Some(5.25));
        assert_eq!(parse_bbs("3", 4.0, false), Some(8.0));
        assert_eq!(parse_bbs("1.2.2", 4.0, true), Some(6.5));
        assert_eq!(parse_bbs("x", 4.0, true), None);
        for b in [0.0, 1.75, 13.5, -2.0] {
            assert_eq!(parse_bbs(&bbs(b, 4.0), 4.0, false), Some(b));
        }
    }
}
