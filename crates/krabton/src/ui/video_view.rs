//! The Video's window: the bank of clips along the top, the picture with
//! what the music does under it, and beside it how the clip plays, a few
//! presets and the links (a source moving an effect). *Screen* opens the
//! picture in a window of its own, for a second screen or a projector
//! (double-click it to fill the screen).

use super::windows::tool_window;
use crate::app::App;
use crate::theme::*;
use crate::video::{ffmpeg, media, ytdl};
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Context, Id, Rect, RichText, Sense, Ui, UiBuilder};
use krabton_engine::video::{self as kv, Effect, PlayMode, Source, VideoLink};
use std::path::PathBuf;

/// A choice that changes with a click (lit when `on`).
fn choice(ui: &mut Ui, label: &str, on: bool, tip: &str) -> egui::Response {
    let b = egui::Button::new(RichText::new(label).color(if on { TEXT_DARK } else { TEXT })).fill(if on { ACCENT } else { PANEL3 });
    ui.add(b).on_hover_text(tip)
}

/// The sixteen-by-nine rect in `r`, in its middle.
fn sixteen_nine(r: Rect) -> Rect {
    let w = r.width().min(r.height() * 16.0 / 9.0);
    Rect::from_center_size(r.center(), vec2(w, w * 9.0 / 16.0))
}

/// m:ss.s
fn clock(secs: f64) -> String {
    let s = secs.max(0.0);
    format!("{}:{:04.1}", (s / 60.0).floor() as u64, s % 60.0)
}

impl App {
    /// The Video: one frame of it, its window and its screen.
    pub(crate) fn ui_video(&mut self, ctx: &Context) {
        self.video_frame(ctx);
        self.poll_ffmpeg_fetch();
        self.poll_link_grab(ctx);
        if self.video_window {
            let (keep, _) = tool_window(ctx, "video", "Video", vec2(1120.0, 700.0), |ui| self.video_body(ui));
            if !keep {
                self.video_window = false;
            }
        }
        if self.video.screen_open {
            self.video_screen(ctx);
        }
    }

    fn poll_ffmpeg_fetch(&mut self) {
        let state = self.video.fetch.as_ref().and_then(|f| f.lock().ok().map(|s| s.clone()));
        match state {
            Some(ffmpeg::Fetch::Ready(_)) => {
                self.video.fetch = None;
                self.video.reload_films();
                self.toast("ffmpeg is here: films play now");
            }
            Some(ffmpeg::Fetch::Failed(e)) => {
                self.video.fetch = None;
                self.toast(e);
            }
            _ => {}
        }
    }

    /// A film from a link arrived: into the bank, its sound onto a new
    /// audio track when asked.
    fn poll_link_grab(&mut self, ctx: &Context) {
        let state = self.video.grab.as_ref().and_then(|g| g.lock().ok().map(|s| s.clone()));
        match state {
            Some(ytdl::Grab::Ready(path)) => {
                self.video.grab = None;
                let title = ytdl::title_of(&path);
                self.toast(format!("{title} is in the Video"));
                let before = self.project.video.clips.len();
                self.video_add(std::slice::from_ref(&path));
                if let Some(c) = self.project.video.clips.get_mut(before) {
                    c.name = title;
                }
                if self.video.link_sound {
                    self.import_files(vec![path], crate::import::FileTarget::NewTrack { beat: None });
                }
            }
            Some(ytdl::Grab::Failed(e)) => {
                self.video.grab = None;
                self.toast(e);
            }
            Some(_) => ctx.request_repaint_after(std::time::Duration::from_millis(250)),
            None => {}
        }
    }

    /// Fetches the film behind the link typed.
    pub(crate) fn video_fetch_link(&mut self) {
        let link = self.video.link.trim().to_string();
        if !ytdl::is_link(&link) {
            self.toast("That is not a link (it starts with https://)");
            return;
        }
        self.video.grab = Some(ytdl::start(link));
        self.video.link_open = false;
        self.video.link.clear();
    }

    fn video_body(&mut self, ui: &mut Ui) {
        // Films and pictures dropped on the Video (or anywhere, when the
        // system does not say where the pointer is) come into its bank, and
        // nowhere else.
        let full = ui.available_rect_before_wrap();
        let here = crate::import::file_drop_pos(ui.ctx()).is_none_or(|p| full.contains(p));
        let dropped: Vec<PathBuf> = ui.ctx().input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).filter(|p| media::takes(p)).collect());
        if here && !dropped.is_empty() {
            self.video_add(&dropped);
            crate::import::take_file_drop(ui.ctx());
        }
        ui.allocate_rect(full, Sense::hover());
        let bank_h = 96.0;
        let side_w = 340.0f32.min(full.width() * 0.42);
        let bank = Rect::from_min_size(full.min, vec2(full.width(), bank_h));
        let below = Rect::from_min_max(pos2(full.left(), bank.bottom() + 8.0), full.max);
        let left = Rect::from_min_max(below.min, pos2(below.right() - side_w - 10.0, below.bottom()));
        let side = Rect::from_min_max(pos2(left.right() + 10.0, below.top()), below.max);

        let mut bank_ui = ui.new_child(UiBuilder::new().max_rect(bank));
        self.video_bank(&mut bank_ui);
        let mut left_ui = ui.new_child(UiBuilder::new().max_rect(left));
        self.video_preview(&mut left_ui);
        let mut side_ui = ui.new_child(UiBuilder::new().max_rect(side));
        egui::ScrollArea::vertical().id_salt("video_side").auto_shrink([false, false]).show(&mut side_ui, |ui| self.video_controls(ui));
    }

    /// The clips, as pads: click one to play it, right-click for more.
    fn video_bank(&mut self, ui: &mut Ui) {
        let r = ui.max_rect();
        let (pw, ph) = (136.0, 76.0);
        let clips = self.project.video.clips.clone();
        let mut pick = None;
        let mut remove = None;
        let mut replace = None;
        let mut slice = None;
        for (i, c) in clips.iter().enumerate() {
            let pad = Rect::from_min_size(pos2(r.left() + i as f32 * (pw + 8.0), r.top()), vec2(pw, ph));
            let resp = ui.interact(pad, Id::new(("video_pad", i)), Sense::click());
            let p = ui.painter();
            rect_fill(p, pad, 4.0, if resp.hovered() { PANEL3 } else { PANEL2 });
            let m = self.video.media.get(&c.path).cloned();
            let info = m.as_ref().map(|m| m.info()).unwrap_or_default();
            if let Some(t) = self.video.thumbs.get(&c.path) {
                let [tw, th] = t.size();
                let area = pad.shrink(3.0);
                let s = (area.width() / tw as f32).min(area.height() / th as f32);
                let img = Rect::from_center_size(area.center(), vec2(tw as f32 * s, th as f32 * s));
                p.image(t.id(), img, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            // Its name on a dark band at the bottom.
            let band = Rect::from_min_max(pos2(pad.left(), pad.bottom() - 16.0), pad.max);
            p.rect_filled(band, egui::CornerRadius { nw: 0, ne: 0, sw: 4, se: 4 }, Color32::from_black_alpha(170));
            p.with_clip_rect(band.shrink2(vec2(4.0, 0.0))).text(pos2(band.left() + 5.0, band.center().y), Align2::LEFT_CENTER, &c.name, small(), TEXT);
            let status = if info.needs_ffmpeg {
                Some(("needs ffmpeg", ACCENT))
            } else if info.error.is_some() {
                Some(("cannot read", REC))
            } else {
                None
            };
            if let Some((s, col)) = status {
                p.text(pos2(pad.center().x, pad.top() + 26.0), Align2::CENTER_CENTER, s, small(), col);
            }
            // Cut by Slice to Drum Rack: how many cues, in a corner.
            if let Some(q) = &c.cues {
                let g = p.text(pad.min + vec2(6.0, 5.0), Align2::LEFT_TOP, format!("{} cues", q.len()), small(), TEXT);
                p.rect_filled(g.expand(2.0), 2.0, Color32::from_black_alpha(170));
                p.text(pad.min + vec2(6.0, 5.0), Align2::LEFT_TOP, format!("{} cues", q.len()), small(), TEXT);
            }
            // Still being read: how far.
            if let Some(m) = &m {
                if !m.is_done() && info.expected > 0 {
                    let f = (m.frame_count() as f32 / info.expected as f32).clamp(0.0, 1.0);
                    let bar = Rect::from_min_size(pos2(pad.left() + 4.0, band.top() - 5.0), vec2((pad.width() - 8.0) * f, 3.0));
                    p.rect_filled(bar, 1.0, ACCENT);
                } else if !m.is_done() {
                    p.text(pos2(pad.center().x, pad.top() + 26.0), Align2::CENTER_CENTER, "reading...", small(), TEXT_DIM);
                }
            }
            let on = self.video.playing == i;
            rect_line(p, pad, 4.0, if on { 2.0 } else { 1.0 }, if on { ACCENT } else { LINE });
            let tip = match (&info.error, info.note.is_empty()) {
                (Some(e), _) => format!("{}\n{e}", c.path.display()),
                (None, false) => format!("{}\n{}", c.path.display(), info.note),
                (None, true) => c.path.display().to_string(),
            };
            let film = kv::is_film(&c.path);
            let more = if film { "slice it to a Drum Rack, replace or remove it" } else { "replace or remove it" };
            let resp = resp.on_hover_text(format!("{tip}\nClick to play it; right-click to {more}"));
            if resp.clicked() {
                pick = Some(i);
            }
            resp.context_menu(|ui| {
                if film && ui.button("Slice to Drum Rack...").on_hover_text("Its sound cut at its hits or on the beat onto the pads of a new Drum Rack track, and its picture cut at the same places: the track's notes play both, in sync").clicked() {
                    slice = Some(c.path.clone());
                    ui.close();
                }
                if ui.button("Replace...").clicked() {
                    replace = Some(i);
                    ui.close();
                }
                if ui.button("Remove").clicked() {
                    remove = Some(i);
                    ui.close();
                }
            });
        }
        // The empty pad: load more.
        if clips.len() < kv::MAX_CLIPS {
            let pad = Rect::from_min_size(pos2(r.left() + clips.len() as f32 * (pw + 8.0), r.top()), vec2(pw, ph));
            let resp = ui.interact(pad, Id::new("video_pad_add"), Sense::click());
            let p = ui.painter();
            let col = if resp.hovered() { TEXT } else { TEXT_DIM };
            rect_line(p, pad, 4.0, 1.0, if resp.hovered() { ACCENT } else { LINE });
            p.text(pad.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, "+", egui::FontId::proportional(22.0), col);
            p.text(pad.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "Film, picture or GIF", small(), col);
            if resp.on_hover_text("Load films (needs ffmpeg), pictures (PNG, JPEG) or animated GIFs. Or drop them on this window").clicked() {
                if let Some(paths) = rfd::FileDialog::new().set_title("Films, pictures or GIFs for the Video").add_filter("Films, pictures and GIFs", &media::EXTENSIONS).pick_files() {
                    self.video_add(&paths);
                }
            }
        }
        // The pad for a link (YouTube and the like), or how its film is coming.
        if clips.len() + 1 < kv::MAX_CLIPS {
            let pad = Rect::from_min_size(pos2(r.left() + (clips.len() + 1) as f32 * (pw + 8.0), r.top()), vec2(pw, ph));
            let grab = self.video.grab.as_ref().and_then(|g| g.lock().ok().map(|s| s.clone()));
            let resp = ui.interact(pad, Id::new("video_pad_link"), Sense::click());
            let p = ui.painter();
            let busy = grab.is_some();
            let col = if resp.hovered() || busy { TEXT } else { TEXT_DIM };
            rect_line(p, pad, 4.0, 1.0, if resp.hovered() || self.video.link_open || busy { ACCENT } else { LINE });
            match &grab {
                Some(g) => {
                    let (what, f) = match g {
                        ytdl::Grab::Tool => ("Getting yt-dlp...".to_string(), 0.0),
                        ytdl::Grab::Film(f) if *f >= 0.999 => ("Almost there...".to_string(), 1.0),
                        ytdl::Grab::Film(f) => (format!("Fetching {:.0} %", f * 100.0), *f),
                        _ => (String::new(), 1.0),
                    };
                    p.text(pad.center() - vec2(0.0, 6.0), Align2::CENTER_CENTER, what, small(), col);
                    let bar = Rect::from_min_size(pos2(pad.left() + 8.0, pad.center().y + 12.0), vec2((pad.width() - 16.0) * f, 3.0));
                    p.rect_filled(bar, 1.0, ACCENT);
                }
                None => {
                    p.text(pad.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, "Link", egui::FontId::proportional(16.0), col);
                    p.text(pad.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "YouTube and more", small(), col);
                }
            }
            let tip = if busy { "A film is being fetched from a link" } else { "A film from a link (YouTube and the many sites yt-dlp knows): paste it, and it comes into the bank. yt-dlp is fetched the first time" };
            if resp.on_hover_text(tip).clicked() && !busy {
                self.video.link_open = !self.video.link_open;
            }
            if self.video.link_open && !busy {
                self.video_link_box(ui.ctx(), pos2(pad.left(), pad.bottom() + 4.0));
            }
        }
        if let Some(i) = pick {
            self.video_pick(i);
        }
        if let Some(path) = slice {
            self.open_film_slice_dialog(&path);
        }
        if let Some(i) = replace {
            if let Some(path) = rfd::FileDialog::new().set_title("A film, picture or GIF instead").add_filter("Films, pictures and GIFs", &media::EXTENSIONS).pick_file() {
                if let Some(c) = self.project.video.clips.get_mut(i) {
                    *c = kv::VideoClip::new(path);
                    self.video_pick(i);
                }
            }
        }
        if let Some(i) = remove {
            let set = &mut self.project.video;
            set.clips.remove(i);
            set.current = set.current.min(set.clips.len().saturating_sub(1));
            self.video.playing = set.current;
        }
    }

    /// The box to paste a link in, under its pad.
    fn video_link_box(&mut self, ctx: &Context, at: egui::Pos2) {
        let mut get = false;
        let mut close = false;
        egui::Area::new(Id::new("video_link_box")).order(egui::Order::Foreground).fixed_pos(at).show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(420.0);
                ui.label(RichText::new("A film from a link").strong().color(TEXT));
                let edit = ui.add(egui::TextEdit::singleline(&mut self.video.link).hint_text("https://www.youtube.com/watch?v=...").desired_width(f32::INFINITY));
                edit.request_focus();
                if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    get = true;
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
                ui.checkbox(&mut self.video.link_sound, "Its sound on a new audio track too");
                ui.horizontal(|ui| {
                    if ui.add_enabled(ytdl::is_link(&self.video.link), egui::Button::new("Get it")).clicked() {
                        get = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                    if ui.button("Paste").on_hover_text("The link on the clipboard").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                    }
                });
                ui.label(RichText::new("Fetched with yt-dlp (it comes the first time), up to 720p, into Documents/Krabton/Library/Links. Only take what you may use").small().color(TEXT_DIM));
            });
        });
        if get {
            self.video_fetch_link();
        } else if close {
            self.video.link_open = false;
        }
    }

    /// The picture, and what the music does under it.
    fn video_preview(&mut self, ui: &mut Ui) {
        let r = ui.max_rect();
        let ears_h = 54.0;
        let trim_h = 22.0;
        let pic_area = Rect::from_min_max(r.min, pos2(r.right(), r.bottom() - ears_h - trim_h - 8.0));
        // At the top of its place, in the middle across.
        let fitted = sixteen_nine(pic_area);
        let pic = Rect::from_min_size(pos2(fitted.left(), pic_area.top()), fitted.size());
        let p = ui.painter().clone();
        p.rect_filled(pic, 2.0, Color32::BLACK);
        let set = self.project.video.clone();
        let m = self.video.playing_media(&set);
        let info = m.as_ref().map(|m| m.info()).unwrap_or_default();
        self.draw_stage(&p, pic);
        // What to do when there is nothing to see.
        let mut msg_ui = ui.new_child(UiBuilder::new().max_rect(pic.shrink(24.0)).layout(egui::Layout::top_down(egui::Align::Center)));
        if set.clips.is_empty() {
            msg_ui.add_space(pic.height() * 0.25);
            msg_ui.label(RichText::new("Drop a film, a picture or a GIF here, or click + above").color(TEXT).size(15.0));
            msg_ui.add_space(6.0);
            msg_ui.label(RichText::new("It plays along with the music: the kicks, the bass, the notes and the beat move it, as the links on the right say. Screen puts it on a second screen or a projector.").color(TEXT_DIM));
        } else if info.needs_ffmpeg {
            self.ffmpeg_offer(&mut msg_ui, pic.height());
        } else if let Some(e) = &info.error {
            msg_ui.add_space(pic.height() * 0.35);
            msg_ui.label(RichText::new(e).color(REC));
        }
        // How it is doing, under the picture.
        let line_y = pic.bottom() + 4.0;
        if pic.bottom() + 18.0 < r.bottom() - ears_h {
            let left = if info.duration > 0.0 { format!("{} / {}", clock(self.video.at), clock(info.duration as f64)) } else { String::new() };
            p.text(pos2(pic.left(), line_y), Align2::LEFT_TOP, left, mono(), TEXT_DIM);
            p.text(pos2(pic.right(), line_y), Align2::RIGHT_TOP, &info.note, small(), TEXT_DIM);
        }
        // The part that plays, under that.
        let strip = Rect::from_min_size(pos2(pic.left(), pic.bottom() + 22.0), vec2(pic.width(), 14.0));
        if info.duration > 0.0 && strip.bottom() < r.bottom() - ears_h {
            self.video_trim(ui, strip, info.duration);
        }
        // The meters right under it (at the bottom when the picture fills the height).
        let ears_top = (pic.bottom() + 24.0 + trim_h).min(r.bottom() - ears_h);
        let ears = Rect::from_min_size(pos2(pic.left(), ears_top), vec2(pic.width(), ears_h));
        self.video_ears(ui, ears);
    }

    /// The part of the playing clip that plays, on a strip as long as the
    /// film: drag its ends (or right-click for the playhead's place);
    /// double-click for the whole film again.
    fn video_trim(&mut self, ui: &mut Ui, strip: Rect, duration: f32) {
        let i = self.video.playing;
        let Some(clip) = self.project.video.clips.get(i) else { return };
        let cued = clip.cues.as_ref().is_some_and(|c| !c.is_empty());
        let (from, to) = clip.part(duration);
        let x_of = |t: f32| strip.left() + strip.width() * (t / duration).clamp(0.0, 1.0);
        let t_of = |x: f32| ((x - strip.left()) / strip.width()).clamp(0.0, 1.0) * duration;
        let p = ui.painter();
        p.rect_filled(strip, 3.0, PANEL3);
        let kept = Rect::from_min_max(pos2(x_of(from), strip.top()), pos2(x_of(to), strip.bottom()));
        p.rect_filled(kept, 3.0, if cued { LINE } else { ACCENT.gamma_multiply(0.45) });
        let at = x_of(self.video.at as f32);
        p.line_segment([pos2(at, strip.top() - 2.0), pos2(at, strip.bottom() + 2.0)], egui::Stroke::new(1.5_f32, TEXT));
        if cued {
            ui.interact(strip, Id::new("video_trim"), Sense::hover()).on_hover_text("Cut by Slice to Drum Rack: the whole film plays, by its cues");
            return;
        }
        for x in [kept.left(), kept.right()] {
            p.rect_filled(Rect::from_center_size(pos2(x, strip.center().y), vec2(5.0, strip.height() + 6.0)), 2.0, ACCENT);
        }
        let resp = ui.interact(strip.expand2(vec2(4.0, 4.0)), Id::new("video_trim"), Sense::click_and_drag());
        let which = Id::new("video_trim_end");
        if resp.drag_started() {
            if let Some(pt) = resp.interact_pointer_pos() {
                // The nearer end.
                let end = (pt.x - kept.right()).abs() < (pt.x - kept.left()).abs();
                ui.ctx().data_mut(|d| d.insert_temp(which, end));
            }
        }
        let mut new = None;
        if resp.dragged() {
            if let Some(pt) = resp.interact_pointer_pos() {
                let end = ui.ctx().data(|d| d.get_temp::<bool>(which)).unwrap_or(false);
                let t = t_of(pt.x);
                new = Some(if end { (from, t.max(from + kv::MIN_PART)) } else { (t.min(to - kv::MIN_PART), to) });
            }
        }
        if resp.double_clicked() {
            new = Some((0.0, duration));
        }
        let here = self.video.at as f32;
        resp.context_menu(|ui| {
            if ui.button("Start here").clicked() {
                new = Some((here.min(to - kv::MIN_PART), to));
                ui.close();
            }
            if ui.button("End here").clicked() {
                new = Some((from, here.max(from + kv::MIN_PART)));
                ui.close();
            }
            if ui.button("Whole film").clicked() {
                new = Some((0.0, duration));
                ui.close();
            }
        });
        let resp = resp.on_hover_text(format!("Plays {} to {} ({:.1} s): drag the ends; right-click for the playhead's place; double-click for the whole film", clock(from as f64), clock(to as f64), to - from));
        let _ = resp;
        if let Some((a, b)) = new {
            if let Some(c) = self.project.video.clips.get_mut(i) {
                c.start = a.max(0.0);
                c.end = (b < duration - 1e-3).then_some(b);
            }
        }
    }

    /// The stage in `rect` (or, without shaders, the plain picture).
    fn draw_stage(&self, p: &egui::Painter, rect: Rect) {
        if self.video.stage.failed().is_some() {
            if let (Some(t), true) = (&self.video.texture, self.video.params.video_size[0] > 0.0) {
                let [w, h] = t.size();
                let s = (rect.width() / w as f32).min(rect.height() / h as f32);
                p.image(t.id(), Rect::from_center_size(rect.center(), vec2(w as f32 * s, h as f32 * s)), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            return;
        }
        let tex = self.video.texture.as_ref().map(|t| t.id()).filter(|_| self.video.params.video_size[0] > 0.0);
        self.video.stage.paint(p, rect, tex, self.video.params, self.frame);
    }

    /// Films need ffmpeg: what it is, a button to fetch it, how far that is.
    fn ffmpeg_offer(&mut self, ui: &mut Ui, h: f32) {
        ui.add_space((h * 0.18).max(4.0));
        ui.label(RichText::new("This clip is a film, and films need ffmpeg").color(TEXT).size(15.0));
        ui.add_space(4.0);
        ui.label(RichText::new("ffmpeg is a free program that reads nearly every film. Krabton can fetch it into its library for you.").color(TEXT_DIM));
        ui.add_space(8.0);
        let state = self.video.fetch.as_ref().and_then(|f| f.lock().ok().map(|s| s.clone()));
        match (state, ffmpeg::download()) {
            (Some(ffmpeg::Fetch::Downloading { bytes }), Some((_, total))) => {
                ui.add(egui::ProgressBar::new((bytes as f32 / total as f32).min(1.0)).desired_width(280.0).text(format!("Fetching ffmpeg: {} of about {} MB", bytes >> 20, total >> 20)));
            }
            (Some(ffmpeg::Fetch::Unpacking), _) => {
                ui.label(RichText::new("Unpacking ffmpeg...").color(TEXT));
            }
            (_, Some((_, total))) => {
                if ui.add(egui::Button::new(RichText::new(format!("Get ffmpeg ({} MB)", total / 1_000_000)).color(TEXT_DARK)).fill(ACCENT)).on_hover_text(format!("Downloads a ready-made ffmpeg into {}", ffmpeg::library_dir().display())).clicked() {
                    self.video.fetch = Some(ffmpeg::start_fetch());
                }
                ui.add_space(4.0);
                ui.label(RichText::new(ffmpeg::install_hint()).small().color(TEXT_DIM));
            }
            (_, None) => {
                ui.label(RichText::new(format!("Install it ({}), then start Krabton again", ffmpeg::install_hint().trim_start_matches("or install it yourself ").trim_matches(['(', ')']))).color(TEXT));
            }
        }
        ui.add_space(6.0);
        ui.label(RichText::new("Pictures and GIFs play without it.").small().color(TEXT_DIM));
        if ui.small_button("Look again").on_hover_text("Installed it just now? Krabton looks for it again").clicked() {
            ffmpeg::forget();
            self.video.reload_films();
        }
    }

    /// What the music does now, as small meters (the master's and the song's).
    fn video_ears(&self, ui: &mut Ui, r: Rect) {
        let p = ui.painter();
        rect_fill(p, r, 4.0, PANEL2);
        let sources = [Source::Kick, Source::Hits, Source::Bass, Source::Mids, Source::Highs, Source::Level, Source::Notes, Source::Beat, Source::Bar];
        let w = (r.width() - 16.0) / sources.len() as f32;
        for (k, s) in sources.iter().enumerate() {
            let cell = Rect::from_min_size(pos2(r.left() + 8.0 + k as f32 * w, r.top() + 6.0), vec2(w - 6.0, r.height() - 12.0));
            let v = self.video.master_value(*s).clamp(0.0, 1.0);
            let bar = Rect::from_min_max(pos2(cell.left(), cell.bottom() - 6.0), pos2(cell.right(), cell.bottom()));
            p.rect_filled(bar, 2.0, PANEL3);
            p.rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * v, bar.height())), 2.0, mix(shade(ACCENT, 0.6), ACCENT, v));
            p.text(pos2(cell.left(), cell.top() + 4.0), Align2::LEFT_CENTER, s.name(), small(), if v > 0.5 { TEXT } else { TEXT_DIM });
            let _ = ui.interact(cell, Id::new(("video_ear", k)), Sense::hover()).on_hover_text(s.about());
        }
    }

    /// How it plays, the presets, the links, the screen.
    fn video_controls(&mut self, ui: &mut Ui) {
        let mut set = self.project.video.clone();
        let playing = self.video.playing.min(set.clips.len().saturating_sub(1));
        let cued = set.play == PlayMode::Slices && set.clips.get(playing).is_some_and(|c| c.cues.is_some());
        let tracks: Vec<(u64, String)> = self.project.tracks.iter().map(|t| (t.id, t.name.clone())).collect();
        let track_name = |id: Option<u64>, none: &str| id.and_then(|id| tracks.iter().find(|t| t.0 == id)).map_or(none.to_string(), |t| t.1.clone());
        // The next track in the list (None first: the master, or all).
        let next_track = |id: Option<u64>| -> Option<u64> {
            let at = id.and_then(|id| tracks.iter().position(|t| t.0 == id));
            match at {
                None => tracks.first().map(|t| t.0),
                Some(i) => tracks.get(i + 1).map(|t| t.0),
            }
        };

        // The screen: the picture alone in a window of its own.
        let screen = self.video.screen_open;
        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new(RichText::new(if screen { "Screen is open" } else { "Screen" }).color(if screen { TEXT_DARK } else { TEXT }).size(14.0)).fill(if screen { ACCENT } else { PANEL3 })).on_hover_text("The picture alone in a window of its own: drag it to a second screen or a projector and double-click it to fill that screen (Esc leaves). Click again to close it").clicked() {
            self.video.screen_open = !screen;
            self.video.fullscreen = false;
        }
        ui.add_space(8.0);

        ui.label(RichText::new("Play").strong());
        ui.horizontal_wrapped(|ui| {
            if choice(ui, set.play.name(), true, &format!("{}. Click for the next way", set.play.about())).clicked() {
                set.play = set.play.next();
            }
            if choice(ui, "Reverse", set.reverse, "Plays backwards").clicked() {
                set.reverse = !set.reverse;
            }
            if choice(ui, set.fit.name(), false, "Fill: the whole screen, cutting off what sticks out. Fit: the whole picture, with black bars. Click to change").clicked() {
                set.fit = set.fit.next();
            }
            if choice(ui, "Scenes", set.follow_scenes, "The scene that plays picks the clip: scene 1 the first clip, scene 2 the second, and on").clicked() {
                set.follow_scenes = !set.follow_scenes;
            }
        });
        match set.play {
            PlayMode::Tempo => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("The clip lasts").small().color(TEXT_DIM));
                    let label = format!("{} bar{}", set.bars, if set.bars == 1 { "" } else { "s" });
                    if choice(ui, &label, false, "How many bars the whole clip takes, in time with the song. Click for more").clicked() {
                        set.bars = match set.bars {
                            1 => 2,
                            2 => 4,
                            4 => 8,
                            8 => 16,
                            _ => 1,
                        };
                    }
                });
            }
            // Cues play as filmed, as their sound does.
            PlayMode::Slices if cued => {}
            PlayMode::Loop | PlayMode::Slices => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Speed").small().color(TEXT_DIM));
                    ui.spacing_mut().slider_width = 150.0;
                    ui.add(egui::Slider::new(&mut set.speed, 0.0..=4.0).fixed_decimals(2).suffix("×")).on_hover_text("How fast it plays (1: as filmed). Double-click to type");
                });
            }
        }
        if let (true, Some(cues)) = (cued, set.clips.get_mut(playing).and_then(|c| c.cues.as_mut())) {
            // Cut where Slice to Drum Rack cut its sound.
            let mut even = false;
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Cues").small().color(TEXT_DIM));
                ui.label(RichText::new(cues.len().to_string()).color(TEXT)).on_hover_text("Where the film is cut: the same places as its sound's slices on the Drum Rack's pads, C1 the first");
                ui.label(RichText::new("played by").small().color(TEXT_DIM));
                if choice(ui, &track_name(cues.track, "every track"), false, "Whose notes show the cues (the track it was sliced to). Click for the next track").clicked() {
                    cues.track = next_track(cues.track);
                }
                let (name, tip) = if cues.gate { ("Gate", "Gate: the picture stands still where its note ends, as the pad's sound stops. Click for One-Shot") } else { ("One-Shot", "One-Shot: each piece plays on to its end, as the pad's sound does. Click for Gate") };
                if choice(ui, name, false, tip).clicked() {
                    cues.gate = !cues.gate;
                }
                if choice(ui, "Even", false, "Forget the cues: back to equal slices").clicked() {
                    even = true;
                }
            });
            ui.label(RichText::new("Set by Slice to Drum Rack: each note shows the film from its slice's start, as filmed, for as long as its sound plays").small().color(TEXT_DIM));
            if even {
                set.clips[playing].cues = None;
            }
        } else if set.play == PlayMode::Slices {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Slices").small().color(TEXT_DIM));
                if choice(ui, &set.slices.to_string(), false, "In how many slices the clip is cut (C1 plays the first). Click for more").clicked() {
                    set.slices = match set.slices {
                        4 => 8,
                        8 => 16,
                        16 => 32,
                        _ => 4,
                    };
                }
                ui.label(RichText::new("played by").small().color(TEXT_DIM));
                if choice(ui, &track_name(set.slice_track, "every track"), false, "Whose notes pick the slices. Click for the next track").clicked() {
                    set.slice_track = next_track(set.slice_track);
                }
            });
        } else {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Slices").small().color(TEXT_DIM));
                if choice(ui, &set.slices.to_string(), false, "Where Jump can cut to: the clip in this many equal slices. Click for more").clicked() {
                    set.slices = match set.slices {
                        4 => 8,
                        8 => 16,
                        16 => 32,
                        _ => 4,
                    };
                }
            });
        }
        ui.add_space(8.0);

        ui.label(RichText::new("Music").strong()).on_hover_text("How much the music moves the picture, all the links at once");
        ui.horizontal(|ui| {
            ui.spacing_mut().slider_width = ui.available_width() - 50.0;
            ui.add(egui::Slider::new(&mut set.amount, 0.0..=1.0).show_value(false)).on_hover_text("How much the music moves the picture, all the links at once (0: the clip as it is)");
            ui.label(RichText::new(format!("{:.0} %", set.amount * 100.0)).small().color(TEXT_DIM));
        });
        ui.add_space(8.0);

        ui.label(RichText::new("Presets").strong());
        ui.horizontal_wrapped(|ui| {
            for name in kv::PRESET_NAMES {
                if let Some(pr) = kv::preset(name) {
                    // Lit while its links are as it set them (whichever track they hear).
                    let same = |a: &[VideoLink], b: &[VideoLink]| a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x.source, x.effect, x.amount) == (y.source, y.effect, y.amount));
                    let on = same(&set.links, &pr.links);
                    if choice(ui, name, on, pr.about).clicked() {
                        set.apply_preset(&pr);
                        self.video_fit_links(&mut set.links);
                    }
                }
            }
        });
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Links").strong()).on_hover_text("Each link: a source in the music moves an effect, by its amount");
            ui.label(RichText::new("source > effect, amount").small().color(TEXT_DIM));
        });
        let mut remove = None;
        for (i, l) in set.links.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                let src = ui.add_sized([58.0, 22.0], egui::Button::new(RichText::new(l.source.name()).color(TEXT)).fill(PANEL3)).on_hover_text(format!("{}.\nClick for the next source, right-click for all", l.source.about()));
                if src.clicked() {
                    l.source = l.source.next();
                }
                src.context_menu(|ui| {
                    for s in Source::ALL {
                        if ui.selectable_label(l.source == s, s.name()).on_hover_text(s.about()).clicked() {
                            l.source = s;
                            ui.close();
                        }
                    }
                });
                if l.source.has_track() {
                    let none = if l.source.listens() { "Master" } else { "All" };
                    let name = track_name(l.from, none);
                    let short: String = if name.chars().count() > 9 { format!("{}…", name.chars().take(8).collect::<String>()) } else { name.clone() };
                    let t = ui.add_sized([62.0, 22.0], egui::Button::new(RichText::new(short).small().color(TEXT_DIM)).fill(PANEL2)).on_hover_text(format!("Of {name}. Click for the next track ({none}: {})", if l.source.listens() { "the whole song" } else { "every track" }));
                    if t.clicked() {
                        l.from = next_track(l.from);
                    }
                } else {
                    // The song's own: no track to pick, the effect stays in its column.
                    ui.add_sized([62.0, 22.0], egui::Label::new(RichText::new("Song").small().color(TEXT_DIM)));
                }
                ui.label(RichText::new(">").color(TEXT_DIM));
                let eff = ui.add_sized([76.0, 22.0], egui::Button::new(RichText::new(l.effect.name()).color(TEXT)).fill(PANEL3)).on_hover_text(format!("{}.\nClick for the next effect, right-click for all", l.effect.about()));
                if eff.clicked() {
                    l.effect = l.effect.next();
                }
                eff.context_menu(|ui| {
                    for e in Effect::ALL {
                        if ui.selectable_label(l.effect == e, e.name()).on_hover_text(e.about()).clicked() {
                            l.effect = e;
                            ui.close();
                        }
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("x").on_hover_text("Take this link away").clicked() {
                        remove = Some(i);
                    }
                    ui.spacing_mut().slider_width = ui.available_width().clamp(40.0, 110.0);
                    let tip = if l.effect.on_hits() { "How often a hit does it" } else { "How much" };
                    ui.add(egui::Slider::new(&mut l.amount, 0.0..=1.0).show_value(false)).on_hover_text(tip);
                });
            });
            // The source now, as a thin line under its row.
            let v = self.video.readings.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            let y = ui.cursor().top() - 1.0;
            let line = Rect::from_min_max(pos2(ui.max_rect().left(), y - 2.0), pos2(ui.max_rect().right(), y));
            let p = ui.painter();
            p.rect_filled(line, 1.0, PANEL2);
            p.rect_filled(Rect::from_min_size(line.min, vec2(line.width() * v * l.amount.max(0.15), line.height())), 1.0, ACCENT);
            ui.add_space(3.0);
        }
        if let Some(i) = remove {
            set.links.remove(i);
        }
        if set.links.len() < kv::MAX_LINKS && ui.button("+ Link").on_hover_text("A new link: the kick zooming in, to start from").clicked() {
            let mut link = [VideoLink::new(Source::Kick, Effect::Zoom, 0.4)];
            self.video_fit_links(&mut link);
            set.links.extend(link);
        }
        if set != self.project.video {
            self.project.video = set;
        }
        ui.add_space(8.0);
        ui.label(RichText::new("Pictures and GIFs play as they are; films need ffmpeg. Drop files on this window to add them.").small().color(TEXT_DIM));
    }

    /// The picture alone, in a window of its own (for a second screen).
    fn video_screen(&mut self, ctx: &Context) {
        let id = egui::ViewportId::from_hash_of("krabton_video_screen");
        let builder = egui::ViewportBuilder::default().with_title("Krabton Video").with_inner_size([960.0, 540.0]).with_min_inner_size([160.0, 90.0]);
        let mut close = false;
        ctx.show_viewport_immediate(id, builder, |ctx, class| {
            let draw = |ui: &mut Ui, app: &mut App| {
                let r = ui.max_rect();
                ui.painter().rect_filled(r, 0.0, Color32::BLACK);
                let pic = sixteen_nine(r);
                app.draw_stage(ui.painter(), pic);
                let resp = ui.interact(r, Id::new("video_screen_area"), Sense::click());
                // A hint while the mouse is over it (out of the picture when it is still).
                if resp.hovered() && ui.input(|i| i.pointer.time_since_last_movement() < 2.0) {
                    let hint = if app.video.fullscreen { "Double-click or Esc: back to a window" } else { "Double-click: fill the screen" };
                    let g = ui.painter().text(r.center_bottom() - vec2(0.0, 20.0), Align2::CENTER_CENTER, hint, body(), TEXT);
                    ui.painter().rect_filled(g.expand(6.0), 4.0, Color32::from_black_alpha(160));
                    ui.painter().text(r.center_bottom() - vec2(0.0, 20.0), Align2::CENTER_CENTER, hint, body(), TEXT);
                }
                resp.double_clicked()
            };
            let toggle = if class == egui::ViewportClass::Embedded {
                // No windows of its own here: inside the main one.
                let mut open = true;
                let t = egui::Window::new("Video screen").open(&mut open).default_size([480.0, 270.0]).show(ctx, |ui| draw(ui, self)).and_then(|r| r.inner).unwrap_or(false);
                close |= !open;
                t
            } else {
                egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::BLACK)).show(ctx, |ui| draw(ui, self)).inner
            };
            let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
            let f11 = ctx.input(|i| i.key_pressed(egui::Key::F11));
            if toggle || f11 || (esc && self.video.fullscreen) {
                self.video.fullscreen = !self.video.fullscreen;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.video.fullscreen));
            }
            if ctx.input(|i| i.viewport().close_requested()) {
                close = true;
            }
        });
        if close {
            self.video.screen_open = false;
            self.video.fullscreen = false;
        }
    }
}
