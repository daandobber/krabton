//! A drum pad's sample as a waveform, as a sampler shows it: the whole file,
//! the part the pad plays lit, its start and end to drag (or the part moved
//! whole, or a new one drawn), its fades as handles at the top, the mouse
//! wheel to zoom, and a line that runs along while the pad sounds. Under it
//! the start and the end in seconds, and Reverse.

use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, RichText, Sense, Shape, Stroke, Ui};
use krabton_engine::sample::{SampleRegion, SampleRef};
use krabton_engine::DrumPad;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

/// Files read whole, to cut pads from.
#[derive(Default)]
pub(crate) struct WaveCache {
    whole: HashMap<PathBuf, SampleRef>,
    loading: HashMap<PathBuf, Receiver<Result<SampleRef, String>>>,
}

/// What a drag on the waveform holds.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Grab {
    Start,
    End,
    FadeIn,
    FadeOut,
    /// The part moved whole, held this many frames from its start.
    Move(u64),
    /// A new part drawn from this frame.
    New(u64),
}

/// What a press at frame `at` takes, with the part `a..b` and its fades;
/// `slop` is how many frames near counts as on a line.
pub(crate) fn grab_at(at: u64, top: bool, a: u64, b: u64, fade_in: u64, fade_out: u64, slop: u64) -> Grab {
    let near = |x: u64| at.abs_diff(x) <= slop;
    if top && near(a + fade_in) {
        Grab::FadeIn
    } else if top && near(b.saturating_sub(fade_out)) {
        Grab::FadeOut
    } else if near(a) {
        Grab::Start
    } else if near(b) {
        Grab::End
    } else if at > a && at < b {
        Grab::Move(at - a)
    } else {
        Grab::New(at)
    }
}

/// The part after a drag to frame `at`: (start, end, fade in, fade out), in a
/// file of `n` frames; a part is at least `min` frames long and its fades fit in it.
pub(crate) fn dragged(grab: Grab, at: u64, part: (u64, u64, u64, u64), n: u64, min: u64) -> (u64, u64, u64, u64) {
    let (a, b, fi, fo) = part;
    let (a, b) = match grab {
        Grab::Start => (at.min(b.saturating_sub(min)), b),
        Grab::End => (a, at.max(a + min).min(n)),
        Grab::Move(off) => {
            let len = b - a;
            let a = at.saturating_sub(off).min(n.saturating_sub(len));
            (a, a + len)
        }
        Grab::New(from) => {
            let (x, y) = (from.min(at), from.max(at));
            (x, y.max(x + min).min(n))
        }
        Grab::FadeIn | Grab::FadeOut => (a, b),
    };
    let len = b - a;
    let (fi, fo) = match grab {
        Grab::FadeIn => (at.saturating_sub(a).min(len / 2), fo),
        Grab::FadeOut => (fi, b.saturating_sub(at).min(len / 2)),
        _ => (fi.min(len / 2), fo.min(len / 2)),
    };
    (a, b, fi, fo)
}

/// The view after a turn of the wheel: zoomed about frame `at` by `factor`
/// (below 1 closer), kept inside the file and at least `min` frames wide.
pub(crate) fn zoomed(view: (f64, f64), at: f64, factor: f64, n: f64, min: f64) -> (f64, f64) {
    let width = ((view.1 - view.0) * factor).clamp(min.min(n), n);
    let t = ((at - view.0) / (view.1 - view.0).max(1e-9)).clamp(0.0, 1.0);
    let a = (at - t * width).clamp(0.0, n - width);
    (a, a + width)
}

impl WaveCache {
    /// A file read whole elsewhere (for a Video Rack) is kept here too.
    pub(crate) fn keep_whole(&mut self, path: &Path, whole: &SampleRef) {
        self.whole.insert(path.to_path_buf(), whole.clone());
    }
}

impl App {
    /// The file a pad's sample was cut from, read whole (in the background the first time).
    pub(crate) fn whole_sample(&mut self, path: &Path) -> Option<SampleRef> {
        if let Some(s) = self.waves.whole.get(path) {
            return Some(s.clone());
        }
        if let Some(rx) = self.waves.loading.get(path) {
            match rx.try_recv() {
                Ok(Ok(s)) => {
                    self.waves.loading.remove(path);
                    self.waves.whole.insert(path.to_path_buf(), s.clone());
                    return Some(s);
                }
                Ok(Err(_)) => {
                    self.waves.loading.remove(path);
                }
                Err(_) => {}
            }
            return None;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let p = path.to_path_buf();
        std::thread::spawn(move || {
            let _ = tx.send(SampleRef::load(&p));
        });
        self.waves.loading.insert(path.to_path_buf(), rx);
        None
    }

    /// The pad's waveform and its part, to edit. `since`: how long ago it was struck.
    pub(crate) fn pad_wave(&mut self, ui: &mut Ui, id: Id, pad: &mut DrumPad, width: f32, height: f32, since: Option<f64>) {
        let Some(sample) = pad.sample.clone() else { return };
        let Some(path) = sample.path.clone() else { return };
        let whole = self.whole_sample(&path);
        let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click_and_drag());
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 3.0, Color32::from_gray(0x16));
        let Some(whole) = whole else {
            text(&p, rect.center(), Align2::CENTER_CENTER, "Reading the file...", small(), TEXT_DIM);
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
            return;
        };
        let n = whole.frames().max(1) as u64;
        let region = sample.region.unwrap_or(SampleRegion { start: 0, end: sample.frames() as u64, fade_in: 0, fade_out: 0 });
        let (a, b, fi, fo) = (region.start.min(n), region.end.min(n).max(region.start + 1), region.fade_in as u64, region.fade_out as u64);
        let rate = whole.sample_rate.max(1) as f64;

        // The view: zoomed by the wheel, whole at first.
        let view_id = id.with("view");
        let mut view: (f64, f64) = ui.ctx().data(|d| d.get_temp(view_id)).unwrap_or((0.0, n as f64));
        if view.1 > n as f64 || view.1 <= view.0 {
            view = (0.0, n as f64);
        }
        if let Some(pos) = resp.hover_pos() {
            let wheel = ui.input(|i| i.smooth_scroll_delta.y);
            if wheel != 0.0 {
                let at = view.0 + ((pos.x - rect.left()) / rect.width()) as f64 * (view.1 - view.0);
                view = zoomed(view, at, (-wheel as f64 * 0.004).exp(), n as f64, rate * 0.01);
            }
        }
        if resp.double_clicked() {
            let pad_len = (b - a) as f64;
            view = zoomed((a as f64 - pad_len * 0.15, b as f64 + pad_len * 0.15), (a + b) as f64 * 0.5, 1.0, n as f64, rate * 0.01);
            view = (view.0.max(0.0), view.1.min(n as f64));
        }
        let x_of = |f: f64| rect.left() + ((f - view.0) / (view.1 - view.0)) as f32 * rect.width();
        let f_of = |x: f32| (view.0 + ((x - rect.left()) / rect.width()) as f64 * (view.1 - view.0)).clamp(0.0, n as f64) as u64;

        // The waveform: a line a column, lit where the pad plays.
        let accent = accent_now();
        let mid = rect.center().y;
        let cols = rect.width() as usize;
        for c in 0..cols {
            let f0 = view.0 + (view.1 - view.0) * c as f64 / cols as f64;
            let f1 = view.0 + (view.1 - view.0) * (c + 1) as f64 / cols as f64;
            let (i0, i1) = (f0 as usize, (f1 as usize).max(f0 as usize + 1).min(n as usize));
            let step = ((i1 - i0) / 64).max(1);
            let (mut lo, mut hi) = (0.0f32, 0.0f32);
            let mut i = i0;
            while i < i1 {
                let v = 0.5 * (whole.left[i] + whole.right[i]);
                lo = lo.min(v);
                hi = hi.max(v);
                i += step;
            }
            let inside = (f0 as u64) >= a && (f0 as u64) < b;
            let x = rect.left() + c as f32 + 0.5;
            let half = rect.height() * 0.46;
            p.line_segment([pos2(x, mid - hi * half), pos2(x, mid - lo * half - 0.5)], Stroke::new(1.0_f32, if inside { accent } else { Color32::from_gray(0x55) }));
        }
        // The part: its edges, its fades, and where it plays now.
        let (xa, xb) = (x_of(a as f64), x_of(b as f64));
        p.rect_filled(Rect::from_x_y_ranges(xa..=xb, rect.y_range()), 0.0, accent.gamma_multiply(0.08));
        for x in [xa, xb] {
            p.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(2.0_f32, Color32::WHITE));
        }
        let (xfi, xfo) = (x_of((a + fi) as f64), x_of(b.saturating_sub(fo) as f64));
        p.add(Shape::line(vec![pos2(xa, rect.bottom()), pos2(xfi, rect.top() + 6.0), pos2(xfo, rect.top() + 6.0), pos2(xb, rect.bottom())], Stroke::new(1.0_f32, Color32::from_white_alpha(120))));
        for x in [xfi, xfo] {
            p.rect_filled(Rect::from_center_size(pos2(x, rect.top() + 6.0), vec2(8.0, 8.0)), 1.0, Color32::WHITE);
        }
        if let Some(s) = since {
            let played = s * rate * 2f64.powf(pad.tune as f64 / 12.0);
            if played < (b - a) as f64 {
                let f = if pad.reverse { b as f64 - played } else { a as f64 + played };
                let x = x_of(f);
                p.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(1.5_f32, Color32::from_rgb(0x5c, 0xe0, 0xff)));
                ui.ctx().request_repaint();
            }
        }

        // The hand.
        let grab_id = id.with("grab");
        let slop = ((view.1 - view.0) / rect.width() as f64 * 6.0) as u64;
        if resp.drag_started() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let g = grab_at(f_of(pos.x), pos.y < rect.top() + 14.0, a, b, fi, fo, slop);
                ui.ctx().data_mut(|d| d.insert_temp(grab_id, g));
            }
        }
        let mut part = None;
        if resp.dragged() {
            if let (Some(g), Some(pos)) = (ui.ctx().data(|d| d.get_temp::<Grab>(grab_id)), resp.interact_pointer_pos()) {
                part = Some(dragged(g, f_of(pos.x), (a, b, fi, fo), n, (rate * 0.005) as u64));
            }
        }
        if let Some(pos) = resp.hover_pos() {
            let g = grab_at(f_of(pos.x), pos.y < rect.top() + 14.0, a, b, fi, fo, slop);
            ui.ctx().set_cursor_icon(match g {
                Grab::Start | Grab::End | Grab::FadeIn | Grab::FadeOut => egui::CursorIcon::ResizeHorizontal,
                Grab::Move(_) => egui::CursorIcon::Grab,
                Grab::New(_) => egui::CursorIcon::Crosshair,
            });
        }
        resp.on_hover_text("Drag its edges for where it starts and ends, the squares for its fades, the middle to move it, elsewhere to draw a new part. Wheel: zoom, double-click: zoom to it");
        ui.ctx().data_mut(|d| d.insert_temp(view_id, view));

        // Under it: where it starts and ends, Reverse, zoom to it.
        ui.horizontal(|ui| {
            let (mut s0, mut s1) = (a as f64 / rate, b as f64 / rate);
            let total = n as f64 / rate;
            ui.label(RichText::new("Start").small().color(crate::skin::ink_dim()));
            let c0 = ui.add(egui::DragValue::new(&mut s0).range(0.0..=total).speed(0.001).suffix(" s").max_decimals(3)).changed();
            ui.label(RichText::new("End").small().color(crate::skin::ink_dim()));
            let c1 = ui.add(egui::DragValue::new(&mut s1).range(0.0..=total).speed(0.001).suffix(" s").max_decimals(3)).changed();
            if c0 || c1 {
                let (x, y) = ((s0 * rate) as u64, (s1 * rate) as u64);
                part = Some(dragged(Grab::New(x.min(y)), x.max(y), (a, b, fi, fo), n, (rate * 0.005) as u64));
            }
            ui.checkbox(&mut pad.reverse, RichText::new("Reverse").small().color(crate::skin::ink_dim())).on_hover_text("Plays from its end to its start");
            if ui.small_button("Fit").on_hover_text("Zoom to the part it plays").clicked() {
                let pad_len = (b - a) as f64;
                let v = (((a as f64) - pad_len * 0.15).max(0.0), ((b as f64) + pad_len * 0.15).min(n as f64));
                ui.ctx().data_mut(|d| d.insert_temp(view_id, v));
            }
            if ui.small_button("All").on_hover_text("The whole file").clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(view_id, (0.0, n as f64)));
            }
        });
        if let Some((a2, b2, fi2, fo2)) = part {
            if (a2, b2, fi2, fo2) != (a, b, fi, fo) {
                let r = SampleRegion { start: a2, end: b2, fade_in: fi2 as u32, fade_out: fo2 as u32 };
                pad.sample = Some(SampleRef::new(whole.region(sample.name.clone(), r)));
            }
        }
    }

    /// How long ago pad `k` of a drum track was struck, while it may still sound.
    pub(crate) fn pad_since(&self, track: u64, k: usize) -> Option<f64> {
        self.video_rack.pad_since(track, k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_takes_what_is_under_it() {
        // A part from 100 to 200, fades of 10 and 20, 3 frames of slop.
        let g = |at, top| grab_at(at, top, 100, 200, 10, 20, 3);
        assert_eq!(g(101, false), Grab::Start);
        assert_eq!(g(198, false), Grab::End);
        assert_eq!(g(110, true), Grab::FadeIn);
        assert_eq!(g(181, true), Grab::FadeOut);
        assert_eq!(g(150, false), Grab::Move(50));
        assert_eq!(g(300, false), Grab::New(300));
    }

    #[test]
    fn dragging_keeps_the_part_whole_and_in_the_file() {
        let part = (100, 200, 10, 20);
        assert_eq!(dragged(Grab::Start, 50, part, 1000, 5), (50, 200, 10, 20));
        assert_eq!(dragged(Grab::Start, 199, part, 1000, 5), (195, 200, 2, 2), "never shorter than its least, fades inside");
        assert_eq!(dragged(Grab::End, 5000, part, 1000, 5), (100, 1000, 10, 20), "not past the file");
        assert_eq!(dragged(Grab::Move(50), 980, part, 1000, 5), (900, 1000, 10, 20));
        assert_eq!(dragged(Grab::Move(50), 10, part, 1000, 5), (0, 100, 10, 20));
        assert_eq!(dragged(Grab::New(400), 300, part, 1000, 5), (300, 400, 10, 20), "drawn backwards too");
        assert_eq!(dragged(Grab::FadeIn, 130, part, 1000, 5), (100, 200, 30, 20));
        assert_eq!(dragged(Grab::FadeOut, 120, part, 1000, 5), (100, 200, 10, 50), "half the part at most");
    }

    #[test]
    fn the_wheel_zooms_about_the_pointer_and_stays_in_the_file() {
        let v = zoomed((0.0, 1000.0), 250.0, 0.5, 1000.0, 10.0);
        assert!((v.0 - 125.0).abs() < 1e-9 && (v.1 - 625.0).abs() < 1e-9, "{v:?}");
        let v = zoomed((0.0, 1000.0), 0.0, 4.0, 1000.0, 10.0);
        assert_eq!(v, (0.0, 1000.0), "never more than all of it");
        let v = zoomed((0.0, 20.0), 10.0, 0.01, 1000.0, 10.0);
        assert!((v.1 - v.0 - 10.0).abs() < 1e-9, "never closer than its least");
    }
}
