//! The Video in the app: the clips read into memory, the reckoning of
//! every frame (`director`: the ears on the music, the clip that plays and
//! where its film is), and the picture with its effects handed to the
//! stage. The window and the screen are in `ui::video_view`; what is saved
//! with the set is `krabton_engine::video::VideoSet`; `export` renders it
//! to a film with the song's sound.

pub mod director;
pub mod export;
pub mod ffmpeg;
pub mod gif;
pub mod media;
pub mod record;
pub mod stage;
pub mod strip;
pub mod ytdl;

use crate::app::App;
use director::{Asks, Director, Shot};
use krabton_engine::project::Project;
use krabton_engine::video::{self as kv, Effect, Look, Transition, VideoClip, VideoLook, VideoSet};
use media::Media;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// The Video while Krabton runs.
pub struct VideoRuntime {
    /// Clips read (or being read), by path.
    pub media: HashMap<PathBuf, Arc<Media>>,
    /// Small pictures of them for the bank.
    pub thumbs: HashMap<PathBuf, egui::TextureHandle>,
    /// The frames of each film along the timeline, made as they are needed.
    pub strips: HashMap<PathBuf, strip::Strip>,
    /// The ears, the clips and where they are.
    pub dir: Director,
    /// What the last frame showed.
    pub shot: Shot,
    /// The picture shown, and which frame of which clip it is (and the
    /// same for the clip it is changing from).
    pub texture: Option<egui::TextureHandle>,
    shown: Option<(PathBuf, usize)>,
    pub old_texture: Option<egui::TextureHandle>,
    old_shown: Option<(PathBuf, usize)>,
    pub stage: stage::Stage,
    pub params: stage::Params,
    /// Where the film that plays is (seconds).
    pub at: f64,
    /// The second window (for a projector) and whether it fills its screen.
    pub screen_open: bool,
    pub fullscreen: bool,
    /// The full screen the screen's window was last told.
    pub full_sent: Option<bool>,
    /// ffmpeg being fetched.
    pub fetch: Option<Arc<Mutex<ffmpeg::Fetch>>>,
    /// A film being fetched from a link, the link typed, whether the box
    /// for it is open and whether its sound goes on a track too.
    pub grab: Option<Arc<Mutex<ytdl::Grab>>>,
    pub link: String,
    pub link_open: bool,
    pub link_sound: bool,
    last: Option<Instant>,
    /// The timeline: the film stands still, a part's start marked (it plays
    /// whole until its end is marked), the parts picked, one part played
    /// alone, a place to put the film (seconds of it), and what of the
    /// film shows (from, how long; 0: all of it).
    pub paused: bool,
    pub mark_in: Option<f32>,
    pub picked: Vec<usize>,
    pub solo: Option<usize>,
    pub seek: Option<f64>,
    pub view: (f32, f32),
    /// The clip the timeline shows (the parts picked and the marks are its own).
    pub editing: Option<usize>,
    audition_was: (bool, Option<usize>),
    /// Where the window was last frame (its keys work while the pointer is on it).
    pub window_rect: Option<egui::Rect>,
    /// Export Video: its box, and the film being made.
    pub export_open: bool,
    pub export_opts: export::Opts,
    pub export: Option<export::Job>,
    /// Record Video: the recording going on, and one being put together.
    pub rec: Option<record::Recorder>,
    pub rec_done: Option<std::thread::JoinHandle<Result<PathBuf, String>>>,
    /// The side panel scrolled here once (by script, to look at it).
    pub side_scroll: Option<f32>,
}

impl Default for VideoRuntime {
    fn default() -> Self {
        Self {
            media: HashMap::new(),
            thumbs: HashMap::new(),
            strips: HashMap::new(),
            dir: Director::default(),
            shot: Shot::default(),
            texture: None,
            shown: None,
            old_texture: None,
            old_shown: None,
            stage: stage::Stage::default(),
            params: stage::Params::default(),
            at: 0.0,
            screen_open: false,
            fullscreen: false,
            full_sent: None,
            fetch: None,
            grab: None,
            link: String::new(),
            link_open: false,
            link_sound: false,
            last: None,
            paused: false,
            mark_in: None,
            picked: Vec::new(),
            solo: None,
            seek: None,
            view: (0.0, 0.0),
            editing: None,
            audition_was: (false, None),
            window_rect: None,
            export_open: false,
            export_opts: export::Opts::default(),
            export: None,
            rec: None,
            rec_done: None,
            side_scroll: None,
        }
    }
}

impl VideoRuntime {
    /// The clip that plays now (in the bank).
    pub fn playing(&self) -> usize {
        self.dir.switch.clip
    }

    /// The clip that plays now, if it is read (or being read).
    pub fn playing_media(&self, set: &VideoSet) -> Option<Arc<Media>> {
        let clip = set.clips.get(self.playing().min(set.clips.len().saturating_sub(1)))?;
        self.media.get(&clip.path).cloned()
    }

    /// Reads a clip again (ffmpeg came).
    pub fn reload_films(&mut self) {
        self.media.retain(|_, m| !m.info().needs_ffmpeg);
        self.shown = None;
        self.old_shown = None;
    }

    /// Plays clip `i` at once, from its start (a set opened, a clip cut).
    pub fn play_now(&mut self, i: usize) {
        self.dir.switch = kv::Switch { clip: i, ..Default::default() };
    }

    /// The clip as it plays for now: the whole film while a part is being
    /// marked, one part while it is played alone (only the clip edited, while it plays).
    pub fn audition(&self, set: &VideoSet) -> Option<VideoClip> {
        let i = self.playing();
        if self.editing != Some(i) || set.current != i {
            return None;
        }
        let c = set.clips.get(i)?;
        if self.mark_in.is_some() && c.is_cut() {
            return Some(VideoClip { parts: Vec::new(), ..c.clone() });
        }
        let part = *c.parts.get(self.solo?)?;
        Some(VideoClip { parts: vec![part], ..c.clone() })
    }
}

/// A smooth wobble between -1 and 1, a new place every unit of `t`.
fn wobble(t: f64, seed: f32) -> f32 {
    let h = |n: f64| ((n.sin() * 43758.5453).fract() * 2.0 - 1.0) as f32;
    let i = t.floor();
    let f = (t - i) as f32;
    let u = f * f * (3.0 - 2.0 * f);
    let (a, b) = (h(i * 12.9898 + seed as f64), h((i + 1.0) * 12.9898 + seed as f64));
    a + (b - a) * u
}

/// How a clip sits on the stage, as its frame says.
fn framing(set: &VideoSet, clip: Option<&VideoClip>) -> stage::Framing {
    let f = clip.map(|c| c.frame).unwrap_or_default();
    stage::Framing { fill: f.fit.unwrap_or(set.fit) == kv::Fit::Fill, opacity: f.opacity, zoom: f.zoom, pos: [f.x, f.y], turn: f.turn, crop: f.crop }
}

fn rgb(c: [u8; 3]) -> [f32; 3] {
    c.map(|v| v as f32 / 255.0)
}

/// What the effects of Look `def` make of a frame (`look` what its links
/// made of it now), for the stage (and an export): `sizes` are the
/// pictures' (the clip that plays, the one it is changing from; None: no
/// picture yet), `noise` three numbers by chance for this frame.
#[allow(clippy::too_many_arguments)]
pub fn stage_params(set: &VideoSet, project: &Project, def: &VideoLook, look: &Look, shot: &Shot, sizes: (Option<[f32; 2]>, Option<[f32; 2]>), noise: [f32; 3]) -> stage::Params {
    let mut p = stage::Params { fx: look.fx, ..Default::default() };
    for e in Effect::ALL {
        p.p[e.index()] = def.setting(e).params4(e);
    }
    let fx = |e: Effect| look.get(e);
    let set_of = |e: Effect| def.setting(e);
    let secs = shot.beat * 60.0 / project.bpm.max(1.0);
    // Spin: as far round as it is up, its angle always, and how far its Speed turned it.
    let sp = p.p[Effect::Spin.index()];
    p.spin = fx(Effect::Spin) * sp[0] + sp[1] + look.turned;
    // Shake: every frame somewhere else, or wobbling at its rate.
    let sh = p.p[Effect::Shake.index()];
    let size = fx(Effect::Shake) * sh[0];
    let (x, y) = if sh[1] >= 59.5 { ((noise[0] - 0.5) * 2.0, (noise[1] - 0.5) * 2.0) } else { (wobble(secs * sh[1] as f64, 1.7), wobble(secs * sh[1] as f64, 9.3)) };
    p.shake = [x * size * sh[2], y * size * sh[3]];
    // The glitch's strips move every frame, or at its rate; the grain every frame.
    let rate = p.p[Effect::Glitch.index()][2];
    p.seed = if rate >= 59.5 { noise[2] * 100.0 } else { ((secs * rate as f64).floor() * 7.31).rem_euclid(100.0) as f32 };
    p.grain = (noise[2] * 977.0).fract() * 100.0;
    // Tint: its own colour, or the track's of the strongest Tint link.
    p.tint = match (set_of(Effect::Tint).color, look.tint_link.and_then(|i| def.links.get(i))) {
        (Some(c), _) => rgb(c),
        (None, Some(l)) => rgb(l.from.and_then(|id| project.tracks.iter().find(|t| t.id == id)).map(|t| t.color).unwrap_or([0xff, 0x40, 0xa0])),
        (None, None) => rgb([0xff, 0x40, 0xa0]),
    };
    for (e, slot) in [(Effect::Flash, &mut p.flash), (Effect::Strobe, &mut p.strobe), (Effect::Edges, &mut p.edges)] {
        *slot = rgb(set_of(e).color.or(e.color()).unwrap_or([0xff; 3]));
    }
    let st = p.p[Effect::Strobe.index()];
    p.fx[Effect::Strobe.index()] = kv::strobe_light(fx(Effect::Strobe), shot.beat, 1.0 / director::length_beats(st[0]), st[1]);
    p.transition = match (shot.from, set.transition) {
        (None, _) | (_, Transition::Cut) => 0.0,
        (_, Transition::Fade) => 1.0,
        (_, Transition::Flash) => 2.0,
        (_, Transition::Glitch) => 3.0,
    };
    p.mix = shot.from.map_or(1.0, |f| f.2);
    p.video_size = sizes.0.unwrap_or([0.0, 0.0]);
    p.old_size = sizes.1.unwrap_or([0.0, 0.0]);
    p.frame = framing(set, set.clips.get(shot.clip));
    p.old_frame = framing(set, shot.from.and_then(|f| set.clips.get(f.0)));
    p
}

/// What a frame shows, as the director left it: the Look that plays, or
/// while it morphs, the last one turning into it.
pub fn frame_params(set: &VideoSet, project: &Project, dir: &mut Director, shot: &Shot, sizes: (Option<[f32; 2]>, Option<[f32; 2]>)) -> stage::Params {
    let noise = [dir.dice.unit(), dir.dice.unit(), dir.dice.unit()];
    let none = VideoLook::default();
    let def = |i: usize| set.looks.get(i).unwrap_or(&none);
    let b = stage_params(set, project, def(shot.look), &dir.look, shot, sizes, noise);
    match &dir.look_from {
        Some((from, a, m)) => stage::Params::morph(&stage_params(set, project, def(*from), a, shot, sizes, noise), &b, *m),
        None => b,
    }
}
/// Each clip's film length (seconds; 0 for a picture or one not read yet).
pub fn durations(set: &VideoSet, media: &HashMap<PathBuf, Arc<Media>>) -> Vec<f32> {
    set.clips.iter().map(|c| media.get(&c.path).map_or(0.0, |m| m.info().duration)).collect()
}

/// Puts frame `i` of `m` on `tex` (made the first time), unless it is up already.
fn show_frame(ctx: &egui::Context, tex: &mut Option<egui::TextureHandle>, shown: &mut Option<(PathBuf, usize)>, name: &str, m: &Media, i: usize) {
    let key = (m.path.clone(), i);
    if shown.as_ref() == Some(&key) {
        return;
    }
    if let Some(img) = m.image(i) {
        match tex {
            Some(t) => t.set(img, egui::TextureOptions::LINEAR),
            None => *tex = Some(ctx.load_texture(name, img, egui::TextureOptions::LINEAR)),
        }
        *shown = Some(key);
    }
}

impl App {
    /// Whether the Video shows anywhere (it only listens and reads then).
    pub(crate) fn video_on(&self) -> bool {
        self.video_window || self.video.screen_open || self.video.rec.is_some()
    }

    /// Record Video starts, into `path` (or a new film in Documents/Krabton/Recorded).
    pub(crate) fn video_record_start(&mut self, path: Option<PathBuf>) {
        if self.video.rec.is_some() {
            return;
        }
        let Some(exe) = ffmpeg::find() else {
            self.toast("Record Video needs ffmpeg: the Video's Get ffmpeg fetches it");
            return;
        };
        let out = path.unwrap_or_else(|| record::free_name(&record::recorded_dir(), &self.project.name));
        let o = self.video.export_opts;
        let status = self.status();
        match record::Recorder::start(&status, &exe, out, o.width, o.height, o.fps) {
            Ok(r) => {
                self.video.stage.want_pixels(true);
                println!("video record: started {}", r.out.display());
                self.video.rec = Some(r);
            }
            Err(e) => self.toast(e),
        }
    }

    /// Record Video stops: the film is put together in the background (a toast says where).
    pub(crate) fn video_record_stop(&mut self) {
        if let Some(r) = self.video.rec.take() {
            self.video.stage.want_pixels(false);
            let status = self.status();
            self.video.rec_done = Some(r.stop(&status));
        }
    }

    /// A recording put together: says where (or why not).
    fn poll_video_record(&mut self) {
        if !self.video.rec_done.as_ref().is_some_and(|h| h.is_finished()) {
            return;
        }
        match self.video.rec_done.take().map(|h| h.join()) {
            Some(Ok(Ok(p))) => {
                println!("video record: done {}", p.display());
                self.toast(format!("Recorded {}", p.display()));
            }
            Some(Ok(Err(e))) => {
                println!("video record: {e}");
                self.toast(e);
            }
            _ => self.toast("The recording was lost"),
        }
    }

    /// Adds clips to the bank (as many as fit), the first of them playing.
    pub(crate) fn video_add(&mut self, paths: &[PathBuf]) {
        // The first clip: the links listen to the drums for the kicks.
        if self.project.video.clips.is_empty() {
            self.video_fit_looks();
        }
        let set = &mut self.project.video;
        let mut first = None;
        for p in paths {
            if set.clips.len() >= kv::MAX_CLIPS {
                self.toast(format!("The Video's bank holds {} clips", kv::MAX_CLIPS));
                break;
            }
            set.clips.push(kv::VideoClip::new(p.clone()));
            first.get_or_insert(set.clips.len() - 1);
        }
        if let Some(i) = first {
            self.video_pick(i);
        }
    }

    /// Kicks are heard best on the drums alone (on the master a bass note
    /// after a rest is a hit in the low end too): links on the kick that
    /// listen to the master listen to the first drum track instead, when
    /// there is one.
    pub(crate) fn video_fit_links(&self, links: &mut [kv::VideoLink]) {
        let drums = self.project.tracks.iter().find(|t| t.instrument.as_ref().is_some_and(|i| i.is_drums())).map(|t| t.id);
        for l in links.iter_mut().filter(|l| l.source == kv::Source::Kick && l.from.is_none()) {
            l.from = drums;
        }
    }

    /// Every Look's kicks from the drums (see `video_fit_links`).
    pub(crate) fn video_fit_looks(&mut self) {
        let mut looks = std::mem::take(&mut self.project.video.looks);
        for l in &mut looks {
            self.video_fit_links(&mut l.links);
        }
        self.project.video.looks = looks;
    }

    /// Puts a preset in the Look edited (its kicks from the drums, see `video_fit_links`).
    pub(crate) fn video_preset(&mut self, p: &kv::Preset) {
        let mut set = self.project.video.clone();
        set.apply_preset(p);
        let mut links = std::mem::take(&mut set.edited_look_mut().links);
        self.video_fit_links(&mut links);
        set.edited_look_mut().links = links;
        self.project.video = set;
    }

    /// Look pad `i` clicked: it is the one edited, and the set's own now or
    /// on the beat its *Launch* waits for.
    pub(crate) fn video_launch_look(&mut self, i: usize) {
        if i >= self.project.video.looks.len() {
            return;
        }
        self.project.video.look = i;
        let st = self.status();
        let (beat, playing) = (st.song_pos.load(), st.playing.load(Ordering::Relaxed));
        let set = self.project.video.clone();
        self.video.dir.launch_look(&set, i, beat, playing, self.project.beats_per_bar);
    }

    /// Clip `i` of the bank: it is the one edited, and it plays at once.
    pub(crate) fn video_pick(&mut self, i: usize) {
        if i < self.project.video.clips.len() {
            self.project.video.current = i;
            self.video.play_now(i);
        }
    }

    /// Clip `i` clicked in the bank: it is the one edited, and it plays now
    /// or on the beat the set's *Launch* waits for (with its transition).
    pub(crate) fn video_launch(&mut self, i: usize) {
        if i >= self.project.video.clips.len() {
            return;
        }
        self.project.video.current = i;
        let st = self.status();
        let (beat, playing) = (st.song_pos.load(), st.playing.load(Ordering::Relaxed));
        let set = self.project.video.clone();
        self.video.dir.launch(&set, i, beat, playing, self.project.bpm, self.project.beats_per_bar);
    }

    /// One frame of the Video: reads what is needed, listens, moves the film
    /// and puts the picture up. Does nothing (and stops listening) while it
    /// shows nowhere.
    pub(crate) fn video_frame(&mut self, ctx: &egui::Context) {
        let status = self.status();
        self.poll_video_record();
        if !self.video_on() {
            status.listening.store(false, Ordering::Relaxed);
            self.video.last = None;
            return;
        }
        let now = Instant::now();
        let dt = self.video.last.map_or(0.0, |l| now.duration_since(l).as_secs_f32()).min(0.25);
        self.video.last = Some(now);
        let set = self.project.video.clone();
        let v = &mut self.video;

        // The clips: read the new ones, let go of those taken out.
        for c in &set.clips {
            v.media.entry(c.path.clone()).or_insert_with(|| Media::open(&c.path));
        }
        v.media.retain(|p, m| {
            let keep = set.clips.iter().any(|c| &c.path == p);
            if !keep {
                m.cancel();
            }
            keep
        });
        v.thumbs.retain(|p, _| set.clips.iter().any(|c| &c.path == p));
        v.strips.retain(|p, _| set.clips.iter().any(|c| &c.path == p));
        let durations = durations(&set, &v.media);

        // The clip edited: its marks and picked parts are its own.
        if v.editing != Some(set.current) {
            v.editing = Some(set.current);
            v.mark_in = None;
            v.picked.clear();
            v.solo = None;
            v.view = (0.0, 0.0);
        }
        let audition = v.audition(&set);
        let playing_clip = audition.as_ref().or(set.clips.get(v.playing()));
        let dur = durations.get(v.playing()).copied().unwrap_or(0.0);
        // A place in the film asked for (or, when what plays changes, the
        // place it was): into its parts (a gap: the next part's start).
        let aud_key = (v.mark_in.is_some(), v.solo);
        let mut seek_film = v.seek.take();
        if aud_key != v.audition_was && seek_film.is_none() {
            seek_film = Some(v.at);
        }
        v.audition_was = aud_key;
        let seek = match (seek_film, playing_clip) {
            (Some(t), Some(c)) if c.is_cued() && c.play.mode == kv::PlayMode::Slices => Some(t),
            (Some(t), Some(c)) => {
                let spans = c.spans(dur);
                Some(kv::joined_time(&spans, t).unwrap_or_else(|| {
                    // The next part's start (round to the first after the last).
                    let mut acc = 0.0;
                    for &(a, b) in &spans {
                        if t < a as f64 {
                            return acc;
                        }
                        acc += (b - a) as f64;
                    }
                    0.0
                }))
            }
            _ => None,
        };
        let asks = Asks { dt, durations: &durations, seek, hold: v.paused, audition: audition.as_ref() };
        let shot = v.dir.frame(&set, &self.project, &status, &asks);
        v.shot = shot;
        v.at = shot.at;

        // The pictures: a new frame goes up when it changes.
        let media_of = |i: usize| set.clips.get(i).and_then(|c| v.media.get(&c.path).cloned());
        let new = media_of(shot.clip).and_then(|m| m.frame_at(shot.at).map(|i| (m, i)));
        match new {
            Some((m, i)) => show_frame(ctx, &mut v.texture, &mut v.shown, "video_frame", &m, i),
            None => v.shown = None,
        }
        let old = shot.from.and_then(|(c, at, _)| media_of(c).and_then(|m| m.frame_at(at).map(|i| (m, i))));
        match old {
            Some((m, i)) => show_frame(ctx, &mut v.old_texture, &mut v.old_shown, "video_frame_old", &m, i),
            None => v.old_shown = None,
        }
        for (p, m) in &v.media {
            if !v.thumbs.contains_key(p) && m.frame_count() > 0 {
                if let Some(img) = m.thumbnail(160) {
                    v.thumbs.insert(p.clone(), ctx.load_texture(format!("video_thumb_{}", p.display()), img, egui::TextureOptions::LINEAR));
                }
            }
        }

        // The effects for the stage.
        let size = |i: usize| media_of(i).map(|m| m.info()).map(|i| [i.width as f32, i.height as f32]);
        let sizes = (v.shown.as_ref().and(size(shot.clip)), shot.from.and_then(|f| v.old_shown.as_ref().and(size(f.0))));
        v.params = frame_params(&set, &self.project, &mut v.dir, &shot, sizes);
        // Recording: the sound that came and the stage drawn last frame.
        if let Some(r) = &mut v.rec {
            let px = v.stage.take_pixels().map(|p| p.1);
            r.frame(&status, px);
            ctx.request_repaint();
        }
    }
}
