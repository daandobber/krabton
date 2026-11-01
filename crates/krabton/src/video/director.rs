//! The Video's reckoning, frame by frame: the ears on the music (the
//! master's sound and a few tracks'), the notes, the beat, the links, which
//! clip plays (and the one it is changing from) and where in their films
//! they are. It reads only what the engine says (`EngineStatus`), so the
//! window and an export (an engine of its own, run offline) play the Video
//! alike.

use krabton_engine::engine::{EngineStatus, LISTEN_LEN, LISTEN_TRACKS};
use krabton_engine::project::Project;
use krabton_engine::video::{self as kv, BeatClock, Dice, Effect, Listener, Look, NoteFollower, Playhead, Reading, Source, Switch, VideoClip, VideoSet};
use std::collections::HashMap;
use std::sync::atomic::Ordering;

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

/// What the window asks of a frame besides the music.
#[derive(Clone, Copy, Debug, Default)]
pub struct Asks<'a> {
    /// Seconds since the last frame.
    pub dt: f32,
    /// Each clip's film length (seconds; 0 for a picture or one not read yet), by its place in the bank.
    pub durations: &'a [f32],
    /// The film that plays put here (seconds into its parts), as the timeline is dragged.
    pub seek: Option<f64>,
    /// Paused: the film stands.
    pub hold: bool,
    /// The clip that plays, as it is to play for now (the whole film while
    /// a part is being marked, one part only while it is played alone).
    pub audition: Option<&'a VideoClip>,
}

/// What a frame shows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shot {
    /// The clip that plays and where in its film (seconds).
    pub clip: usize,
    pub at: f64,
    /// The clip it is changing from (where in its film), and how far the change is (0..1).
    pub from: Option<(usize, f64, f32)>,
    /// The song's beat (it runs on while the song is stopped, for the strobe).
    pub beat: f64,
}

/// The Video playing.
pub struct Director {
    sample_rate: f32,
    master: Option<Ear>,
    /// The tracks listened to, by slot of `EngineStatus::listen` (track id, its ears).
    tracks: [Option<(u64, Ear)>; LISTEN_TRACKS],
    /// Notes of a track (by id), or of all of them (None).
    notes: HashMap<Option<u64>, NoteFollower>,
    pub clock: BeatClock,
    /// Where the clip that plays is, and the one it is changing from.
    pub playhead: Playhead,
    pub from_head: Playhead,
    pub look: Look,
    /// Each link's source now (for its meter), and whether a continuous one
    /// is up (it hits when it rises past the middle).
    pub readings: Vec<f32>,
    up: Vec<bool>,
    /// Which clip plays (Next Clip, the scenes, the notes and the bank change it, not the set).
    pub switch: Switch,
    /// The clip the playhead is on (it starts again when another plays).
    on: Option<usize>,
    last_scene: Option<usize>,
    /// A beat that runs on while the song is stopped (for the strobe).
    free_beat: f64,
    pub dice: Dice,
    scratch: (Vec<f32>, Vec<f32>),
}

impl Default for Director {
    fn default() -> Self {
        Self {
            sample_rate: 0.0,
            master: None,
            tracks: Default::default(),
            notes: HashMap::new(),
            clock: BeatClock::default(),
            playhead: Playhead::default(),
            from_head: Playhead::default(),
            look: Look::default(),
            readings: Vec::new(),
            up: Vec::new(),
            switch: Switch::default(),
            on: None,
            last_scene: None,
            free_beat: 0.0,
            dice: Dice::new(0x5EED),
            scratch: (Vec::new(), Vec::new()),
        }
    }
}

impl Director {
    /// The ears on the master's sound (for the window's meters).
    pub fn master_value(&self, s: Source) -> f32 {
        match s {
            Source::Beat | Source::Bar | Source::Ramp => self.clock.value(s),
            Source::Notes | Source::Pitch => self.notes.get(&None).map_or(0.0, |n| n.value(s)),
            _ => self.master.as_ref().map_or(0.0, |e| e.listener.value(s)),
        }
    }

    /// The bank's clips put in another order: clip `i` is now `place(i)`
    /// (the one playing goes on as it was).
    pub fn renumber(&mut self, place: impl Fn(usize) -> usize) {
        let sw = &mut self.switch;
        sw.clip = place(sw.clip);
        if let Some(p) = &mut sw.pending {
            p.0 = place(p.0);
        }
        if let Some(f) = &mut sw.fade {
            f.from = place(f.from);
        }
        self.on = self.on.map(&place);
    }

    /// Clip `i` plays now (changing as the set says), or on the beat the set's
    /// *Launch* waits for, while the song plays at `beat`.
    pub fn launch(&mut self, set: &VideoSet, i: usize, beat: f64, playing: bool, bpm: f64, beats_per_bar: u32) {
        let at = kv::launch_beat(set.launch, beat, playing, beats_per_bar);
        self.switch.ask(i, beat, at, set.transition_secs(bpm));
    }

    /// One frame: listens, follows the notes and the beat, reads the links,
    /// changes clips and moves the films.
    pub fn frame(&mut self, set: &VideoSet, project: &Project, status: &EngineStatus, asks: &Asks) -> Shot {
        let dt = asks.dt;
        status.listening.store(true, Ordering::Relaxed);
        // Ears: the master, and the tracks the links listen to.
        let sr = status.sample_rate.load();
        if sr != self.sample_rate || self.master.is_none() {
            self.sample_rate = sr;
            self.master = Some(Ear::new(sr, status.listen_master.written()));
            self.tracks = Default::default();
        }
        let mut wanted: Vec<u64> = Vec::new();
        for l in &set.links {
            if let (true, Some(id)) = (l.source.listens(), l.from) {
                if !wanted.contains(&id) && wanted.len() < LISTEN_TRACKS {
                    wanted.push(id);
                }
            }
        }
        for (k, slot) in self.tracks.iter_mut().enumerate() {
            let want = wanted.get(k).copied();
            if slot.as_ref().map(|s| s.0) != want {
                *slot = want.map(|id| (id, Ear::new(sr, status.listen[k].written())));
            }
            status.listen_ids[k].store(want.unwrap_or(0), Ordering::Relaxed);
        }
        let before = self.master.as_ref().map_or(0, |e| e.read_to);
        let moved = status.listen_master.written().saturating_sub(before).min(LISTEN_LEN as u64) as usize;
        if let Some(ear) = self.master.as_mut() {
            ear.hear(&status.listen_master, 0, &mut self.scratch);
        }
        for (k, slot) in self.tracks.iter_mut().enumerate() {
            if let Some((_, ear)) = slot {
                ear.hear(&status.listen[k], moved, &mut self.scratch);
            }
        }

        // Notes: what started on every track since the last frame.
        let mut all = (0u128, 0.0f32);
        let mut struck: HashMap<u64, (u128, f32)> = HashMap::new();
        for (i, t) in project.tracks.iter().enumerate() {
            if let Some(ts) = status.tracks.get(i) {
                let (mask, vel) = ts.take_struck();
                if mask != 0 {
                    struck.insert(t.id, (mask, vel));
                    all.0 |= mask;
                    all.1 = all.1.max(vel);
                }
            }
        }
        let mut followed: Vec<Option<u64>> = vec![None];
        followed.extend(set.clips.iter().map(|c| c.play.slice_track));
        followed.extend(set.clips.iter().filter_map(|c| c.cues.as_ref()).map(|c| c.track));
        followed.extend(set.links.iter().filter(|l| l.source.follows_notes()).map(|l| l.from));
        if let Some(id) = set.clip_notes {
            followed.push(Some(id));
        }
        followed.sort();
        followed.dedup();
        self.notes.retain(|k, _| followed.contains(k));
        for k in followed {
            let (mask, vel) = match k {
                Some(id) => struck.get(&id).copied().unwrap_or((0, 0.0)),
                None => all,
            };
            self.notes.entry(k).or_default().frame(mask, vel, dt);
        }

        // The beat.
        let playing = status.playing.load(Ordering::Relaxed);
        let beat = status.song_pos.load();
        let bpm = project.bpm;
        self.clock.frame(beat, playing, project.beats_per_bar, dt);
        self.free_beat += dt as f64 * bpm / 60.0;

        // The links.
        self.up.resize(set.links.len(), false);
        self.readings.resize(set.links.len(), 0.0);
        let mut readings = Vec::with_capacity(set.links.len());
        for (i, l) in set.links.iter().enumerate() {
            let mut r = match l.source {
                s if s.listens() => {
                    let ear = l.from.and_then(|id| self.tracks.iter().flatten().find(|t| t.0 == id).map(|t| &t.1)).or(self.master.as_ref());
                    ear.map(|e| e.reading(s)).unwrap_or_default()
                }
                Source::Notes | Source::Pitch => self.notes.get(&l.from).map(|n| Reading { value: n.value(l.source), hit: n.hit }).unwrap_or_default(),
                Source::Ramp => Reading { value: self.clock.value(Source::Ramp), hit: self.clock.bar_hit },
                s => Reading { value: self.clock.value(s), hit: self.clock.hit(s) },
            };
            // A level or a band hits when it rises past the middle.
            if matches!(l.source, Source::Level | Source::Bass | Source::Mids | Source::Highs) {
                r.hit = !self.up[i] && r.value > 0.6;
                if r.value > 0.6 {
                    self.up[i] = true;
                } else if r.value < 0.4 {
                    self.up[i] = false;
                }
            }
            self.readings[i] = r.value;
            readings.push(r);
        }
        let mut k = 0;
        self.look = kv::look(&set.links, set.amount, &mut self.dice, |_| {
            k += 1;
            readings[k - 1]
        });

        // Which clip: a clip waiting for its beat, a scene that starts, a
        // note on the clips' track, Next Clip (these three at once, they
        // come in time).
        let n = set.clips.len();
        let secs = set.transition_secs(bpm);
        self.switch.frame(beat, playing, dt as f64, secs);
        if set.follow_scenes {
            let scene = kv::scene_clip(status.tracks.iter().take(project.tracks.len()).map(|t| t.playing_slot.load(Ordering::Relaxed)), n);
            if let (Some(s), true) = (scene, scene != self.last_scene) {
                self.switch.go(s, secs);
            }
            self.last_scene = scene;
        }
        if let Some(note) = set.clip_notes.and_then(|id| self.notes.get(&Some(id))).filter(|f| f.hit) {
            if n > 0 {
                self.switch.go(kv::slice_for_note(note.pitch, n as u32), secs);
            }
        }
        if self.look.next_clip && n > 1 {
            self.switch.go((self.switch.clip + 1) % n, secs);
        }
        let last = n.saturating_sub(1);
        self.switch.clip = self.switch.clip.min(last);
        if let Some(f) = &mut self.switch.fade {
            if f.from > last {
                self.switch.fade = None;
            }
        }
        let now = self.switch.clip;
        if self.on != Some(now) {
            // The last clip keeps running while the new one comes in; the new one starts from its beginning.
            self.from_head = std::mem::take(&mut self.playhead);
            self.playhead.dice = Dice::new(self.from_head.dice.next());
            self.on = Some(now);
        }

        // Where the films are. A film cut by Slice to Drum Rack follows the
        // notes of the track it was sliced to, and Gate the notes it holds.
        let input = |clip: &VideoClip, i: usize, notes: &HashMap<Option<u64>, NoteFollower>, look: &Look| {
            let cues = clip.cues.as_ref().filter(|c| !c.is_empty());
            let by = cues.map_or(clip.play.slice_track, |c| c.track);
            let slice_note = notes.get(&by).filter(|n| n.hit).map(|n| n.pitch);
            let held = match (cues, by) {
                (None, _) => 0,
                (Some(_), Some(id)) => project.tracks.iter().position(|t| t.id == id).and_then(|i| status.tracks.get(i)).map_or(0, |ts| ts.inst_notes()),
                (Some(_), None) => status.tracks.iter().take(project.tracks.len()).fold(0, |m, ts| m | ts.inst_notes()),
            };
            kv::PlayInput {
                dt,
                duration: asks.durations.get(i).copied().unwrap_or(0.0),
                beat,
                playing,
                bpm,
                beats_per_bar: project.beats_per_bar,
                speed: look.get(Effect::Speed),
                freeze: look.get(Effect::Freeze),
                scratch: look.get(Effect::Scratch),
                jump: look.jump,
                stutter: look.stutter,
                note: if clip.play.mode == kv::PlayMode::Slices { slice_note } else { None },
                held,
                seek: None,
                hold: false,
            }
        };
        let mut shot = Shot { clip: now, beat: if playing { beat } else { self.free_beat }, ..Default::default() };
        if let Some(clip) = asks.audition.or(set.clips.get(now)) {
            let mut inp = input(clip, now, &self.notes, &self.look);
            inp.seek = asks.seek;
            inp.hold = asks.hold;
            shot.at = self.playhead.advance(clip, &inp);
        }
        if let Some(f) = self.switch.fade {
            if let Some(clip) = set.clips.get(f.from) {
                // It goes on as it went, without the hits (they are the new one's).
                let mut inp = input(clip, f.from, &self.notes, &Look::default());
                inp.note = None;
                shot.from = Some((f.from, self.from_head.advance(clip, &inp), f.mix()));
            }
        }
        shot
    }
}
