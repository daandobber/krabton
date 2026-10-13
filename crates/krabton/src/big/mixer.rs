//! Big Picture's Mixer: a strip per track (and the master): volume with its
//! meter, pan, the sends, mute, solo and arm. Left and right move between
//! the strips, up and down between their controls; the triggers (or the
//! right stick) turn the one with the focus, A switches (or resets a fader).
//! Faders can be dragged with a finger.

use super::input::{Frame, Nav};
use super::{fit_text, tile, Glyph, Sz, TILE, TILE_HI};
use crate::app::{App, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Ui};

#[derive(Default)]
pub struct MixState {
    pub strip: usize,
    pub control: usize,
    pub first: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ctl {
    Volume,
    Pan,
    Send(usize),
    Mute,
    Solo,
    Arm,
}

impl App {
    /// The controls of strip `i` (the master is the last).
    fn strip_controls(&self, i: usize) -> Vec<Ctl> {
        if i >= self.project.tracks.len() {
            return vec![Ctl::Volume];
        }
        let mut v = vec![Ctl::Volume, Ctl::Pan];
        v.extend((0..self.project.returns.len().min(4)).map(Ctl::Send));
        v.extend([Ctl::Mute, Ctl::Solo, Ctl::Arm]);
        v
    }

    fn volume_db(&mut self, i: usize) -> &mut f32 {
        match self.project.tracks.get_mut(i) {
            Some(t) => &mut t.mixer.volume_db,
            None => &mut self.project.master.volume_db,
        }
    }

    fn turn_control(&mut self, i: usize, c: Ctl, by: f32) {
        match c {
            Ctl::Volume => {
                let v = self.volume_db(i);
                *v = fader_to_db((db_to_fader(*v) + by).clamp(0.0, 1.0));
            }
            Ctl::Pan => {
                if let Some(t) = self.project.tracks.get_mut(i) {
                    t.mixer.pan = (t.mixer.pan + by * 2.0).clamp(-1.0, 1.0);
                }
            }
            Ctl::Send(k) => {
                if let Some(t) = self.project.tracks.get_mut(i) {
                    if t.mixer.sends.len() <= k {
                        t.mixer.sends.resize(k + 1, 0.0);
                    }
                    t.mixer.sends[k] = (t.mixer.sends[k] + by).clamp(0.0, 1.0);
                }
            }
            _ => {}
        }
    }

    fn press_control(&mut self, i: usize, c: Ctl) {
        match c {
            Ctl::Volume => *self.volume_db(i) = 0.0,
            Ctl::Pan => {
                if let Some(t) = self.project.tracks.get_mut(i) {
                    t.mixer.pan = 0.0;
                }
            }
            Ctl::Send(k) => {
                if let Some(s) = self.project.tracks.get_mut(i).and_then(|t| t.mixer.sends.get_mut(k)) {
                    *s = 0.0;
                }
            }
            Ctl::Mute => {
                if let Some(t) = self.project.tracks.get_mut(i) {
                    t.mixer.mute = !t.mixer.mute;
                }
            }
            Ctl::Solo => {
                if let Some(t) = self.project.tracks.get_mut(i) {
                    t.mixer.solo = !t.mixer.solo;
                }
            }
            Ctl::Arm => {
                if let Some(t) = self.project.tracks.get_mut(i) {
                    t.arm = !t.arm;
                }
            }
        }
    }

    pub(crate) fn big_mixer(&mut self, ui: &mut Ui, r: Rect, s: &Sz, mut inp: Frame) -> Vec<(Glyph, &'static str)> {
        let n = self.project.tracks.len() + 1;
        let mut strip = self.big.mix.strip.min(n - 1);
        let mut ctl_i = self.big.mix.control;
        for nav in inp.pressed.drain(..) {
            let ctls = self.strip_controls(strip);
            ctl_i = ctl_i.min(ctls.len() - 1);
            match nav {
                Nav::Left => strip = strip.saturating_sub(1),
                Nav::Right => strip = (strip + 1).min(n - 1),
                Nav::Up => ctl_i = ctl_i.saturating_sub(1),
                Nav::Down => ctl_i = (ctl_i + 1).min(ctls.len() - 1),
                Nav::A => self.press_control(strip, ctls[ctl_i]),
                _ => {}
            }
        }
        let ctls = self.strip_controls(strip);
        ctl_i = ctl_i.min(ctls.len() - 1);
        if inp.turn.abs() > 0.01 {
            self.turn_control(strip, ctls[ctl_i], inp.turn * inp.dt * 0.6);
        }
        let p = ui.painter().clone();
        let strip_w_min = s.u * 3.6;
        let shown = ((r.width() / strip_w_min).floor() as usize).clamp(1, n);
        let sw = r.width() / shown as f32;
        let mut first = self.big.mix.first;
        if strip < first {
            first = strip;
        } else if strip >= first + shown {
            first = strip + 1 - shown;
        }
        first = first.min(n - shown);
        let sends = self.project.returns.len().min(4);
        let head_h = s.u * 1.1;
        let pan_h = s.u * 0.9;
        let send_h = s.u * 0.75;
        let btn_h = s.u * 1.1;
        let fader_h = (r.height() - head_h - pan_h - send_h * sends as f32 - btn_h - s.u * 0.6).max(s.u * 2.0);
        for (k, i) in (first..first + shown).enumerate() {
            let x = r.left() + k as f32 * sw;
            let col = Rect::from_min_size(pos2(x + s.u * 0.1, r.top()), vec2(sw - s.u * 0.2, r.height()));
            let master = i >= self.project.tracks.len();
            let (name, color, vol, pan, mute, solo, arm, track_sends, meter) = if master {
                ("Master".to_string(), ACCENT, self.project.master.volume_db, 0.0, false, false, false, Vec::new(), self.master_meter)
            } else {
                let t = &self.project.tracks[i];
                (t.name.clone(), rgb(t.color), t.mixer.volume_db, t.mixer.pan, t.mixer.mute, t.mixer.solo, t.arm, t.mixer.sends.clone(), self.meters.get(i).copied().unwrap_or((0.0, 0.0)))
            };
            let focus_ctl = if i == strip { Some(ctls[ctl_i]) } else { None };
            rect_fill(&p, col, 10.0, Color32::from_rgb(0x1f, 0x1f, 0x23));
            // The head.
            let hr = Rect::from_min_size(col.min, vec2(col.width(), head_h - s.u * 0.1));
            rect_fill(&p, hr, 8.0, shade(color, 0.45));
            fit_text(&p, hr.center(), Align2::CENTER_CENTER, &name, s.font, hr.width() - s.u * 0.3, TEXT);
            let hresp = ui.interact(hr, Id::new(("big_mix_head", i)), Sense::click());
            if hresp.clicked() {
                strip = i;
                if !master {
                    self.sel_track = TrackSel::Track(i);
                }
            }
            // The fader with its meter.
            let fr = Rect::from_min_size(pos2(col.left() + s.u * 0.25, hr.bottom() + s.u * 0.2), vec2(col.width() - s.u * 0.5, fader_h));
            let focus = focus_ctl == Some(Ctl::Volume);
            tile(&p, fr, TILE, focus);
            let track_r = Rect::from_min_max(pos2(fr.center().x - fr.width() * 0.12 - (fr.width() * 0.16).min(s.u * 0.45), fr.top() + s.u * 0.75), pos2(fr.center().x - fr.width() * 0.12 + (fr.width() * 0.16).min(s.u * 0.45), fr.bottom() - s.u * 0.25));
            rect_fill(&p, track_r, 6.0, BG);
            let pos = db_to_fader(vol);
            let fill = Rect::from_min_max(pos2(track_r.left(), track_r.bottom() - pos * track_r.height()), track_r.max);
            rect_fill(&p, fill, 6.0, shade(color, 0.85));
            let thumb_y = track_r.bottom() - pos * track_r.height();
            rect_fill(&p, Rect::from_center_size(pos2(track_r.center().x, thumb_y), vec2(track_r.width() + s.u * 0.4, s.u * 0.28)), 4.0, TEXT);
            let meter_r = Rect::from_min_size(pos2(track_r.right() + s.u * 0.35, track_r.top()), vec2(s.u * 0.35, track_r.height()));
            meter_bars(&p, meter_r, meter);
            text(&p, pos2(fr.center().x, fr.top() + s.u * 0.38), Align2::CENTER_CENTER, format!("{vol:+.1} dB"), s.s(), if focus { TEXT } else { TEXT_DIM });
            let vresp = ui.interact(fr, Id::new(("big_mix_vol", i)), Sense::click_and_drag());
            if let Some(pt) = vresp.interact_pointer_pos().filter(|_| vresp.dragged()) {
                let v = ((track_r.bottom() - pt.y) / track_r.height()).clamp(0.0, 1.0);
                *self.volume_db(i) = fader_to_db(v);
            }
            if vresp.double_clicked() {
                *self.volume_db(i) = 0.0;
            }
            if vresp.clicked() || vresp.dragged() {
                strip = i;
                ctl_i = 0;
            }
            if master {
                continue;
            }
            // Pan.
            let mut y = fr.bottom() + s.u * 0.15;
            let pr = Rect::from_min_size(pos2(col.left() + s.u * 0.25, y), vec2(col.width() - s.u * 0.5, pan_h - s.u * 0.15));
            tile(&p, pr, BG, focus_ctl == Some(Ctl::Pan));
            let cx = pr.center().x;
            let px = cx + pan * pr.width() * 0.5;
            rect_fill(&p, Rect::from_min_max(pos2(cx.min(px), pr.top() + 3.0), pos2(cx.max(px).max(cx.min(px) + 2.0), pr.bottom() - 3.0)), 4.0, shade(color, 0.8));
            let pan_text = if pan.abs() < 0.01 { "C".to_string() } else if pan < 0.0 { format!("{:.0}L", -pan * 50.0) } else { format!("{:.0}R", pan * 50.0) };
            text(&p, pr.center(), Align2::CENTER_CENTER, pan_text, s.s(), TEXT);
            let presp = ui.interact(pr, Id::new(("big_mix_pan", i)), Sense::click_and_drag());
            if let Some(pt) = presp.interact_pointer_pos().filter(|_| presp.dragged()) {
                self.project.tracks[i].mixer.pan = ((pt.x - cx) / (pr.width() * 0.5)).clamp(-1.0, 1.0);
            }
            if presp.double_clicked() {
                self.project.tracks[i].mixer.pan = 0.0;
            }
            if presp.clicked() || presp.dragged() {
                strip = i;
                ctl_i = 1;
            }
            y += pan_h;
            // Sends.
            for k in 0..sends {
                let sr = Rect::from_min_size(pos2(col.left() + s.u * 0.25, y), vec2(col.width() - s.u * 0.5, send_h - s.u * 0.12));
                let v = track_sends.get(k).copied().unwrap_or(0.0);
                tile(&p, sr, BG, focus_ctl == Some(Ctl::Send(k)));
                rect_fill(&p, Rect::from_min_size(sr.min, vec2(sr.width() * v.clamp(0.0, 1.0), sr.height())), 4.0, shade(color, 0.6));
                let label = format!("{} {:.0}%", (b'A' + k as u8) as char, v * 100.0);
                text(&p, sr.center(), Align2::CENTER_CENTER, label, s.s(), TEXT);
                let sresp = ui.interact(sr, Id::new(("big_mix_send", i, k)), Sense::click_and_drag());
                if let Some(pt) = sresp.interact_pointer_pos().filter(|_| sresp.dragged()) {
                    let m = &mut self.project.tracks[i].mixer;
                    if m.sends.len() <= k {
                        m.sends.resize(k + 1, 0.0);
                    }
                    m.sends[k] = ((pt.x - sr.left()) / sr.width()).clamp(0.0, 1.0);
                }
                if sresp.clicked() || sresp.dragged() {
                    strip = i;
                    ctl_i = 2 + k;
                }
                y += send_h;
            }
            // Mute, solo, arm.
            let bw = (col.width() - s.u * 0.5 - s.u * 0.3) / 3.0;
            for (k, (ctl, label, on, on_col)) in [(Ctl::Mute, "M", mute, Color32::from_rgb(0xf2, 0xa0, 0x3c)), (Ctl::Solo, "S", solo, SOLO), (Ctl::Arm, "R", arm, REC)].into_iter().enumerate() {
                let br = Rect::from_min_size(pos2(col.left() + s.u * 0.25 + k as f32 * (bw + s.u * 0.15), y + s.u * 0.1), vec2(bw, btn_h - s.u * 0.2));
                tile(&p, br, if on { on_col } else { TILE_HI }, focus_ctl == Some(ctl));
                text(&p, br.center(), Align2::CENTER_CENTER, label, s.f(), if on { TEXT_DARK } else { TEXT });
                let bresp = ui.interact(br, Id::new(("big_mix_btn", i, k)), Sense::click());
                if bresp.clicked() {
                    self.press_control(i, ctl);
                    strip = i;
                    ctl_i = 2 + sends + k;
                }
            }
        }
        self.big.mix = MixState { strip, control: ctl_i, first };
        vec![(Glyph::Dpad, "Move"), (Glyph::Triggers, "Turn"), (Glyph::Btn(Nav::A), "Switch, reset")]
    }
}

/// A stereo meter (peaks 0..1 and a little over).
fn meter_bars(p: &egui::Painter, r: Rect, (l, rr): (f32, f32)) {
    let half = r.width() * 0.5 - 1.0;
    for (k, v) in [l, rr].into_iter().enumerate() {
        let b = Rect::from_min_size(pos2(r.left() + k as f32 * (half + 2.0), r.top()), vec2(half, r.height()));
        rect_fill(p, b, 2.0, BG);
        let h = v.clamp(0.0, 1.0).sqrt() * b.height();
        rect_fill(p, Rect::from_min_max(pos2(b.left(), b.bottom() - h), b.max), 2.0, if v >= 0.99 { REC } else { PLAY });
    }
}
