//! The Video in the app: the clips read into memory, the ears on the music
//! (the master's sound and a few tracks', the notes, the beat), where the
//! film is, and the picture with its effects handed to the stage. The
//! window and the screen are in `ui::video_view`; what is saved with the set
//! is `krabton_engine::video::VideoSet`.

pub mod ffmpeg;
pub mod gif;
pub mod media;
pub mod stage;
pub mod ytdl;

use crate::app::App;
use krabton_engine::engine::{LISTEN_LEN, LISTEN_TRACKS};
use krabton_engine::video::{self as kv, BeatClock, Dice, Effect, Listener, Look, NoteFollower, Playhead, Reading, Source};
use media::Media;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// One sound listened to: its ears, how far it was read, and its hits this frame.
struct Ear {
    listener: Listener,
    read_to: u64,
    kick: bool,
    hit: bool,
}

impl Ear {
    fn new(sr: f32, read_to: u64) -> Self {
        Self { listener: Listener::new(sr), read_to, kick: false, hit: false }
    }

    /// Takes in what came since the last frame (or, when nothing came, as
    /// much silence as the master moved on: an idle track).
    fn hear(&mut self, buf: &krabton_engine::scope::ScopeBuffer, moved: usize, scratch: &mut (Vec<f32>, Vec<f32>)) {
        let new = buf.written().saturating_sub(self.read_to).min(LISTEN_LEN as u64) as usize;
        if new > 0 {
            // The newest `new` samples, up to where the buffer is now.
            self.read_to = buf.read(new, &mut scratch.0, &mut scratch.1);
            self.listener.feed(&scratch.0);
        } else if moved > 0 {
            scratch.0.clear();
            scratch.0.resize(moved, 0.0);
            self.listener.feed(&scratch.0);
        }
        (self.kick, self.hit) = self.listener.take_hits();
    }

    fn reading(&self, s: Source) -> Reading {
        let hit = match s {
            Source::Kick => self.kick,
            Source::Hits => self.hit,
            _ => false,
        };
        Reading { value: self.listener.value(s), hit }
    }
}

/// The Video while Krabton runs.
pub struct VideoRuntime {
    /// Clips read (or being read), by path.
    pub media: HashMap<PathBuf, Arc<Media>>,
    /// Small pictures of them for the bank.
    pub thumbs: HashMap<PathBuf, egui::TextureHandle>,
    sample_rate: f32,
    master: Option<Ear>,
    /// The tracks listened to, by slot of `EngineStatus::listen` (track id, its ears).
    tracks: [Option<(u64, Ear)>; LISTEN_TRACKS],
    /// Notes of a track (by id), or of all of them (None).
    notes: HashMap<Option<u64>, NoteFollower>,
    pub clock: BeatClock,
    pub playhead: Playhead,
    pub look: Look,
    /// Each link's source now (for its meter), and whether a continuous one
    /// is up (it hits when it rises past the middle).
    pub readings: Vec<f32>,
    up: Vec<bool>,
    /// The clip playing now (Next Clip and the scenes change it, not the set).
    pub playing: usize,
    playing_path: Option<PathBuf>,
    last_scene: Option<usize>,
    /// The picture shown, and which frame of which clip it is.
    pub texture: Option<egui::TextureHandle>,
    shown: Option<(PathBuf, usize)>,
    pub stage: stage::Stage,
    pub params: stage::Params,
    /// Where the film is (seconds) and how long it is, for the window.
    pub at: f64,
    /// The second window (for a projector) and whether it fills its screen.
    pub screen_open: bool,
    pub fullscreen: bool,
    /// ffmpeg being fetched.
    pub fetch: Option<Arc<Mutex<ffmpeg::Fetch>>>,
    /// A film being fetched from a link, the link typed, whether the box
    /// for it is open and whether its sound goes on a track too.
    pub grab: Option<Arc<Mutex<ytdl::Grab>>>,
    pub link: String,
    pub link_open: bool,
    pub link_sound: bool,
    last: Option<Instant>,
    /// A beat that runs on while the song is stopped (for the strobe).
    free_beat: f64,
    dice: Dice,
    scratch: (Vec<f32>, Vec<f32>),
}

impl Default for VideoRuntime {
    fn default() -> Self {
        Self {
            media: HashMap::new(),
            thumbs: HashMap::new(),
            sample_rate: 0.0,
            master: None,
            tracks: Default::default(),
            notes: HashMap::new(),
            clock: BeatClock::default(),
            playhead: Playhead::default(),
            look: Look::default(),
            readings: Vec::new(),
            up: Vec::new(),
            playing: 0,
            playing_path: None,
            last_scene: None,
            texture: None,
            shown: None,
            stage: stage::Stage::default(),
            params: stage::Params::default(),
            at: 0.0,
            screen_open: false,
            fullscreen: false,
            fetch: None,
            grab: None,
            link: String::new(),
            link_open: false,
            link_sound: false,
            last: None,
            free_beat: 0.0,
            dice: Dice::new(0x5EED),
            scratch: (Vec::new(), Vec::new()),
        }
    }
}

impl VideoRuntime {
    /// The ears on the master's sound (for the window's meters).
    pub fn master_value(&self, s: Source) -> f32 {
        match s {
            Source::Beat | Source::Bar | Source::Ramp => self.clock.value(s),
            Source::Notes | Source::Pitch => self.notes.get(&None).map_or(0.0, |n| n.value(s)),
            _ => self.master.as_ref().map_or(0.0, |e| e.listener.value(s)),
        }
    }

    /// The clip that plays now, if it is read (or being read).
    pub fn playing_media(&self, set: &kv::VideoSet) -> Option<Arc<Media>> {
        let clip = set.clips.get(self.playing.min(set.clips.len().saturating_sub(1)))?;
        self.media.get(&clip.path).cloned()
    }

    /// Reads a clip again (ffmpeg came).
    pub fn reload_films(&mut self) {
        self.media.retain(|_, m| !m.info().needs_ffmpeg);
        self.shown = None;
    }
}

impl App {
    /// Whether the Video shows anywhere (it only listens and reads then).
    pub(crate) fn video_on(&self) -> bool {
        self.video_window || self.video.screen_open
    }

    /// Adds clips to the bank (as many as fit), the first of them playing.
    pub(crate) fn video_add(&mut self, paths: &[PathBuf]) {
        // The first clip: the links listen to the drums for the kicks.
        if self.project.video.clips.is_empty() {
            let mut links = std::mem::take(&mut self.project.video.links);
            self.video_fit_links(&mut links);
            self.project.video.links = links;
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
        for l in links.iter_mut().filter(|l| l.source == Source::Kick && l.from.is_none()) {
            l.from = drums;
        }
    }

    /// Puts a preset in place (its kicks from the drums, see `video_fit_links`).
    pub(crate) fn video_preset(&mut self, p: &kv::Preset) {
        let mut set = self.project.video.clone();
        set.apply_preset(p);
        self.video_fit_links(&mut set.links);
        self.project.video = set;
    }

    /// Plays clip `i` of the bank (and makes it the set's).
    pub(crate) fn video_pick(&mut self, i: usize) {
        if i < self.project.video.clips.len() {
            self.project.video.current = i;
            self.video.playing = i;
        }
    }

    /// One frame of the Video: reads what is needed, listens, moves the film
    /// and puts the picture up. Does nothing (and stops listening) while it
    /// shows nowhere.
    pub(crate) fn video_frame(&mut self, ctx: &egui::Context) {
        let status = self.status();
        if !self.video_on() {
            status.listening.store(false, Ordering::Relaxed);
            self.video.last = None;
            return;
        }
        status.listening.store(true, Ordering::Relaxed);
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

        // Ears: the master, and the tracks the links listen to.
        let sr = status.sample_rate.load();
        if sr != v.sample_rate || v.master.is_none() {
            v.sample_rate = sr;
            v.master = Some(Ear::new(sr, status.listen_master.written()));
            v.tracks = Default::default();
        }
        let mut wanted: Vec<u64> = Vec::new();
        for l in &set.links {
            if let (true, Some(id)) = (l.source.listens(), l.from) {
                if !wanted.contains(&id) && wanted.len() < LISTEN_TRACKS {
                    wanted.push(id);
                }
            }
        }
        for (k, slot) in v.tracks.iter_mut().enumerate() {
            let want = wanted.get(k).copied();
            if slot.as_ref().map(|s| s.0) != want {
                *slot = want.map(|id| (id, Ear::new(sr, status.listen[k].written())));
            }
            status.listen_ids[k].store(want.unwrap_or(0), Ordering::Relaxed);
        }
        let before = v.master.as_ref().map_or(0, |e| e.read_to);
        let moved = status.listen_master.written().saturating_sub(before).min(LISTEN_LEN as u64) as usize;
        if let Some(ear) = v.master.as_mut() {
            ear.hear(&status.listen_master, 0, &mut v.scratch);
        }
        for (k, slot) in v.tracks.iter_mut().enumerate() {
            if let Some((_, ear)) = slot {
                ear.hear(&status.listen[k], moved, &mut v.scratch);
            }
        }

        // Notes: what started on every track since the last frame.
        let mut all = (0u128, 0.0f32);
        let mut struck: HashMap<u64, (u128, f32)> = HashMap::new();
        for (i, t) in self.project.tracks.iter().enumerate() {
            if let Some(ts) = status.tracks.get(i) {
                let (mask, vel) = ts.take_struck();
                if mask != 0 {
                    struck.insert(t.id, (mask, vel));
                    all.0 |= mask;
                    all.1 = all.1.max(vel);
                }
            }
        }
        let mut followed: Vec<Option<u64>> = vec![None, set.slice_track];
        followed.extend(set.clips.iter().filter_map(|c| c.cues.as_ref()).map(|c| c.track));
        followed.extend(set.links.iter().filter(|l| l.source.follows_notes()).map(|l| l.from));
        followed.sort();
        followed.dedup();
        v.notes.retain(|k, _| followed.contains(k));
        for k in followed {
            let (mask, vel) = match k {
                Some(id) => struck.get(&id).copied().unwrap_or((0, 0.0)),
                None => all,
            };
            v.notes.entry(k).or_default().frame(mask, vel, dt);
        }

        // The beat.
        let playing = status.playing.load(Ordering::Relaxed);
        let beat = status.song_pos.load();
        let bpm = self.project.bpm;
        v.clock.frame(beat, playing, self.project.beats_per_bar, dt);
        v.free_beat += dt as f64 * bpm / 60.0;

        // The links.
        v.up.resize(set.links.len(), false);
        v.readings.resize(set.links.len(), 0.0);
        let mut readings = Vec::with_capacity(set.links.len());
        for (i, l) in set.links.iter().enumerate() {
            let mut r = match l.source {
                s if s.listens() => {
                    let ear = l.from.and_then(|id| v.tracks.iter().flatten().find(|t| t.0 == id).map(|t| &t.1)).or(v.master.as_ref());
                    ear.map(|e| e.reading(s)).unwrap_or_default()
                }
                Source::Notes | Source::Pitch => v.notes.get(&l.from).map(|n| Reading { value: n.value(l.source), hit: n.hit }).unwrap_or_default(),
                Source::Ramp => Reading { value: v.clock.value(Source::Ramp), hit: v.clock.bar_hit },
                s => Reading { value: v.clock.value(s), hit: v.clock.hit(s) },
            };
            // A level or a band hits when it rises past the middle.
            if matches!(l.source, Source::Level | Source::Bass | Source::Mids | Source::Highs) {
                r.hit = !v.up[i] && r.value > 0.6;
                if r.value > 0.6 {
                    v.up[i] = true;
                } else if r.value < 0.4 {
                    v.up[i] = false;
                }
            }
            v.readings[i] = r.value;
            readings.push(r);
        }
        let mut k = 0;
        v.look = kv::look(&set.links, set.amount, &mut v.dice, |_| {
            k += 1;
            readings[k - 1]
        });

        // Which clip: a scene that starts picks one, Next Clip moves on.
        let n = set.clips.len();
        if set.follow_scenes {
            let scene = kv::scene_clip(status.tracks.iter().take(self.project.tracks.len()).map(|t| t.playing_slot.load(Ordering::Relaxed)), n);
            if scene.is_some() && scene != v.last_scene {
                v.playing = scene.unwrap_or(0);
            }
            v.last_scene = scene;
        }
        if v.look.next_clip && n > 1 {
            v.playing = (v.playing + 1) % n;
        }
        v.playing = v.playing.min(n.saturating_sub(1));
        let clip = set.clips.get(v.playing);
        let media = clip.and_then(|c| v.media.get(&c.path).cloned());
        let path = clip.map(|c| c.path.clone());
        if path != v.playing_path {
            v.playing_path = path.clone();
            v.playhead.reset();
        }

        // Where the film is. A film cut by Slice to Drum Rack follows the
        // notes of the track it was sliced to, and Gate the notes it holds.
        let info = media.as_ref().map(|m| m.info()).unwrap_or_default();
        let cues = clip.and_then(|c| c.cues.as_ref()).filter(|c| !c.is_empty());
        let by = cues.map_or(set.slice_track, |c| c.track);
        let slice_note = v.notes.get(&by).filter(|n| n.hit).map(|n| n.pitch);
        let held = match (cues, by) {
            (None, _) => 0,
            (Some(_), Some(id)) => self.project.tracks.iter().position(|t| t.id == id).and_then(|i| status.tracks.get(i)).map_or(0, |ts| ts.inst_notes()),
            (Some(_), None) => status.tracks.iter().take(self.project.tracks.len()).fold(0, |m, ts| m | ts.inst_notes()),
        };
        let input = kv::PlayInput {
            dt,
            duration: info.duration,
            beat,
            playing,
            bpm,
            beats_per_bar: self.project.beats_per_bar,
            speed: v.look.get(Effect::Speed),
            freeze: v.look.get(Effect::Freeze),
            scratch: v.look.get(Effect::Scratch),
            jump: v.look.jump,
            stutter: v.look.stutter,
            note: if set.play == kv::PlayMode::Slices { slice_note } else { None },
            clip: v.playing,
            held,
        };
        v.at = v.playhead.advance(&set, &input);

        // The picture: a new frame goes up when it changes.
        let frame = media.as_ref().and_then(|m| m.frame_at(v.at).map(|i| (m.clone(), i)));
        match (frame, &path) {
            (Some((m, i)), Some(p)) => {
                let key = (p.clone(), i);
                if v.shown.as_ref() != Some(&key) {
                    if let Some(img) = m.image(i) {
                        match &mut v.texture {
                            Some(t) => t.set(img, egui::TextureOptions::LINEAR),
                            None => v.texture = Some(ctx.load_texture("video_frame", img, egui::TextureOptions::LINEAR)),
                        }
                        v.shown = Some(key);
                    }
                }
            }
            _ => v.shown = None,
        }
        for (p, m) in &v.media {
            if !v.thumbs.contains_key(p) && m.frame_count() > 0 {
                if let Some(img) = m.thumbnail(160) {
                    v.thumbs.insert(p.clone(), ctx.load_texture(format!("video_thumb_{}", p.display()), img, egui::TextureOptions::LINEAR));
                }
            }
        }

        // The effects for the stage.
        let lk = v.look;
        let fx = |e: Effect| lk.get(e);
        let shake = fx(Effect::Shake) * 0.05;
        let tint = match lk.tint_link.and_then(|i| set.links.get(i)) {
            Some(l) => {
                let c = l.from.and_then(|id| self.project.tracks.iter().find(|t| t.id == id)).map(|t| t.color).unwrap_or([0xff, 0x40, 0xa0]);
                [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, fx(Effect::Tint)]
            }
            None => [0.0; 4],
        };
        let shown = v.shown.is_some();
        v.params = stage::Params {
            video_size: if shown { [info.width as f32, info.height as f32] } else { [0.0, 0.0] },
            fill: set.fit == kv::Fit::Fill,
            zoom: fx(Effect::Zoom),
            spin: fx(Effect::Spin),
            shake: [(v.dice.unit() - 0.5) * 2.0 * shake, (v.dice.unit() - 0.5) * 2.0 * shake],
            mirror: fx(Effect::Mirror),
            kaleido: fx(Effect::Kaleido),
            pixels: fx(Effect::Pixels),
            split: fx(Effect::Split),
            glitch: fx(Effect::Glitch),
            seed: v.dice.unit() * 100.0,
            hue: fx(Effect::Hue),
            poster: fx(Effect::Poster),
            flash: fx(Effect::Flash),
            invert: fx(Effect::Invert),
            strobe: kv::strobe_light(fx(Effect::Strobe), if playing { beat } else { v.free_beat }),
            gate: fx(Effect::Gate),
            trails: fx(Effect::Trails),
            tint,
        };
    }
}
