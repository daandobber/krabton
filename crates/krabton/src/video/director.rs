//! The Video's reckoning, frame by frame: the ears on the music (the
//! master's sound and a few tracks'), the notes, the beat, the hits heard,
//! the Look that plays (and the one it morphs from) with its links, which
//! clip plays (and the one it is changing from) and where in their films
//! they are. It reads only what the engine says (`EngineStatus`), so the
//! window and an export (an engine of its own, run offline) play the Video
//! alike.

use krabton_engine::engine::{EngineStatus, LISTEN_LEN, LISTEN_TRACKS};
use krabton_engine::project::Project;
use krabton_engine::video::{self as kv, BandListener, BeatClock, Dice, Effect, HitDetector, HitTrigger, Listener, Look, NoteFollower, Playhead, Reading, Rises, Source, Step, Switch, VideoClip, VideoSet};
use std::collections::HashMap;
use std::sync::atomic::Ordering;

/// One sound listened to: its ears (and the bands of its own the links
/// ask for), how far it was read, and its hits this frame.
struct Ear {
    listener: Listener,
    bands: Vec<BandListener>,
    read_to: u64,
    kick: bool,
    hit: bool,
    rises: Rises,
}

impl Ear {
    fn new(sr: f32, read_to: u64) -> Self {
        Self { listener: Listener::new(sr), bands: Vec::new(), read_to, kick: false, hit: false, rises: Rises::default() }
    }

    /// Takes in what came since the last frame (or, when nothing came, as
    /// much silence as the master moved on: an idle track).
    fn hear(&mut self, buf: &krabton_engine::scope::ScopeBuffer, moved: usize, scratch: &mut (Vec<f32>, Vec<f32>)) {
        let new = buf.written().saturating_sub(self.read_to).min(LISTEN_LEN as u64) as usize;
        if new > 0 {
            // The newest `new` samples, up to where the buffer is now.
            self.read_to = buf.read(new, &mut scratch.0, &mut scratch.1);
        } else if moved > 0 {
            scratch.0.clear();
            scratch.0.resize(moved, 0.0);
        } else {
            scratch.0.clear();
        }
        if !scratch.0.is_empty() {
            self.listener.feed(&scratch.0);
            for b in &mut self.bands {
                b.feed(&scratch.0);
            }
        }
        (self.kick, self.hit) = self.listener.take_hits();
        self.rises = self.listener.take_rises();
    }

    fn reading(&self, s: Source) -> Reading {
        let hit = match s {
            Source::Kick => self.kick,
            Source::Hits => self.hit,
            _ => false,
        };
        Reading { value: self.listener.value(s), hit }
    }

    /// A band of its own (made the first time it is asked for).
    fn band(&mut self, sr: f32, band: (f32, f32)) -> f32 {
        match self.bands.iter().find(|b| b.band == band) {
            Some(b) => b.value(),
            None => {
                self.bands.push(BandListener::new(sr, band));
                0.0
            }
        }
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
    /// The Look that plays.
    pub look: usize,
}

/// What a hit trigger drives (each keeps its own gap and grid).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum HitUse {
    Slices(usize),
    Parts(usize),
    Clips,
    Looks,
}

/// A link's state between frames: its smoothed value, whether a level is
/// up (it hits when it rises past the middle), its LFO's cycle.
#[derive(Clone, Copy, Debug, Default)]
struct LinkState {
    value: f32,
    up: bool,
    cycle: Option<i64>,
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
    /// What the Look that plays makes of the picture, and the Look it morphs
    /// from (which, as it moves now, and how far the morph is, 0..1).
    pub look: Look,
    pub look_from: Option<(usize, Look, f32)>,
    /// Each link of the Look edited, its source now (for its meter).
    pub readings: Vec<f32>,
    links: HashMap<(usize, usize), LinkState>,
    /// How far each Look's Spin has turned by itself.
    turned: HashMap<usize, f32>,
    /// Which clip plays (Next Clip, the scenes, the notes, the hits and the bank change it, not the set).
    pub switch: Switch,
    /// The set's own Look (a pad clicked, the notes, the hits, the scenes),
    /// and the Look that plays (the part's, the clip's, the sequence's or
    /// the set's), morphing from the last.
    pub pads: Switch,
    pub looks: Switch,
    /// The clip the playhead is on (it starts again when another plays).
    on: Option<usize>,
    last_scene: Option<usize>,
    last_look_scene: Option<usize>,
    /// The part the film was in last frame (for its Look).
    part_now: Option<usize>,
    /// The set's own Look taken as it starts (the first frame).
    started: bool,
    hits: HashMap<HitUse, HitDetector>,
    /// A beat that runs on while the song is stopped (for the strobe and the LFOs).
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
            look_from: None,
            readings: Vec::new(),
            links: HashMap::new(),
            turned: HashMap::new(),
            switch: Switch::default(),
            pads: Switch::default(),
            looks: Switch::default(),
            on: None,
            last_scene: None,
            last_look_scene: None,
            part_now: None,
            started: false,
            hits: HashMap::new(),
            free_beat: 0.0,
            dice: Dice::new(0x5EED),
            scratch: (Vec::new(), Vec::new()),
        }
    }
}

/// The note lengths of a length setting (`1/4`, `1/8`, `1/16`, `1/32`), in beats.
pub fn length_beats(choice: f32) -> f32 {
    [1.0, 0.5, 0.25, 0.125][(choice.round().max(0.0) as usize).min(3)]
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

    /// The Look that plays now.
    pub fn look_playing(&self) -> usize {
        self.looks.clip
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

    /// Look pad `i` is the set's own now, or on the beat its *Launch* waits for.
    pub fn launch_look(&mut self, set: &VideoSet, i: usize, beat: f64, playing: bool, beats_per_bar: u32) {
        let at = kv::launch_beat(set.look_launch, beat, playing, beats_per_bar);
        self.pads.ask(i, beat, at, 0.0);
    }

    /// The ears on track `id` (None: the master).
    fn ear(&mut self, id: Option<u64>) -> Option<&mut Ear> {
        match id {
            Some(id) if self.tracks.iter().flatten().any(|t| t.0 == id) => self.tracks.iter_mut().flatten().find(|t| t.0 == id).map(|t| &mut t.1),
            _ => self.master.as_mut(),
        }
    }

    /// A hit trigger this frame: the band of a hit that fires now.
    fn hit(&mut self, used: HitUse, t: &HitTrigger, dt: f32, beat: f64, playing: bool, bpm: f64) -> Option<usize> {
        let rises = self.ear(t.track).map(|e| e.rises).unwrap_or_default();
        self.hits.entry(used).or_default().frame(t, rises, dt, beat, playing, bpm)
    }

    /// The links of Look `li` read this frame (smoothed and shaped as each says).
    fn read_look(&mut self, set: &VideoSet, li: usize, dt: f32, lfo_beat: f64) -> Vec<Reading> {
        let Some(look) = set.looks.get(li) else { return Vec::new() };
        let sr = self.sample_rate;
        let mut out = Vec::with_capacity(look.links.len());
        for (k, l) in look.links.iter().enumerate() {
            let mut st = self.links.get(&(li, k)).copied().unwrap_or_default();
            let mut r = match l.source {
                Source::Band => Reading { value: self.ear(l.from).map_or(0.0, |e| e.band(sr, l.band)), hit: false },
                s if s.listens() => self.ear(l.from).map(|e| e.reading(s)).unwrap_or_default(),
                Source::Notes | Source::Pitch => self.notes.get(&l.from).map(|n| Reading { value: n.value(l.source), hit: n.hit }).unwrap_or_default(),
                Source::Ramp => Reading { value: self.clock.value(Source::Ramp), hit: self.clock.bar_hit },
                Source::Fixed => Reading { value: 1.0, hit: false },
                Source::Lfo => {
                    let (v, cycle) = l.lfo.at(lfo_beat);
                    let hit = st.cycle.is_some_and(|c| c != cycle);
                    st.cycle = Some(cycle);
                    Reading { value: v, hit }
                }
                s => Reading { value: self.clock.value(s), hit: self.clock.hit(s) },
            };
            // A level or a band hits when it rises past the middle.
            if matches!(l.source, Source::Level | Source::Bass | Source::Mids | Source::Highs | Source::Band) {
                r.hit = !st.up && r.value > 0.6;
                if r.value > 0.6 {
                    st.up = true;
                } else if r.value < 0.4 {
                    st.up = false;
                }
            }
            // As the link answers: bent, over its threshold, smoothed.
            st.value = l.response.smooth(st.value, l.response.shape(r.value), dt);
            r.value = st.value;
            self.links.insert((li, k), st);
            out.push(r);
        }
        out
    }

    /// What Look `li` makes of the picture this frame.
    fn make_look(&mut self, set: &VideoSet, li: usize, readings: &[Reading], dt: f32, bpm: f64, beats_per_bar: u32) -> Look {
        let Some(def) = set.looks.get(li) else { return Look::default() };
        let mut k = 0;
        let mut look = kv::look(&def.links, &def.fx, set.amount, &mut self.dice, |_| {
            k += 1;
            readings.get(k - 1).copied().unwrap_or_default()
        });
        // Spin's Speed turns it on by itself while it is up.
        let speed = def.setting(Effect::Spin).param(Effect::Spin, 2);
        let turned = self.turned.entry(li).or_insert(0.0);
        *turned = (*turned + dt * speed * look.get(Effect::Spin) * (bpm / 60.0 / beats_per_bar.max(1) as f64) as f32).rem_euclid(1.0);
        look.turned = *turned;
        look
    }

    /// One frame: listens, follows the notes and the beat, picks the Look and
    /// reads its links, changes clips and moves the films.
    pub fn frame(&mut self, set: &VideoSet, project: &Project, status: &EngineStatus, asks: &Asks) -> Shot {
        let dt = asks.dt;
        status.listening.store(true, Ordering::Relaxed);
        let n_looks = set.looks.len().max(1);
        if !self.started {
            // The set's own Look from the start, without a morph.
            self.started = true;
            self.pads.clip = set.look.min(n_looks - 1);
            self.looks.clip = self.pads.clip;
        }
        // The Looks that may be read this frame: the one playing, the one it morphs from, the one edited.
        let mut in_use = vec![self.looks.clip.min(n_looks - 1), set.look.min(n_looks - 1)];
        in_use.extend(self.looks.fade.map(|f| f.from));
        // Ears: the master, and the tracks the links and the hit triggers listen to.
        let sr = status.sample_rate.load();
        if sr != self.sample_rate || self.master.is_none() {
            self.sample_rate = sr;
            self.master = Some(Ear::new(sr, status.listen_master.written()));
            self.tracks = Default::default();
        }
        let mut wanted: Vec<u64> = Vec::new();
        let mut want = |id: Option<u64>| {
            if let Some(id) = id {
                if !wanted.contains(&id) && wanted.len() < LISTEN_TRACKS {
                    wanted.push(id);
                }
            }
        };
        for li in &in_use {
            for l in set.looks.get(*li).map_or(&[][..], |l| &l.links) {
                if l.source.listens() {
                    want(l.from);
                }
            }
        }
        let playing_clip = set.clips.get(self.switch.clip);
        for t in [set.look_hits.as_ref(), set.clip_hits.as_ref(), playing_clip.and_then(|c| c.play.slice_hits.as_ref()), playing_clip.filter(|c| c.play.seq.on && c.play.seq.order == kv::PartOrder::Hits).map(|c| &c.play.seq.hits)].into_iter().flatten() {
            want(t.track);
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
        // The bands of their own nobody asks for any more go.
        let bands: Vec<(f32, f32)> = in_use.iter().filter_map(|li| set.looks.get(*li)).flat_map(|l| l.links.iter()).filter(|l| l.source == Source::Band).map(|l| l.band).collect();
        for ear in self.tracks.iter_mut().flatten().map(|t| &mut t.1).chain(self.master.as_mut()) {
            ear.bands.retain(|b| bands.contains(&b.band));
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
        followed.extend(set.clips.iter().filter(|c| c.play.seq.on && c.play.seq.order == kv::PartOrder::Notes).map(|c| c.play.seq.track));
        followed.extend(in_use.iter().filter_map(|li| set.looks.get(*li)).flat_map(|l| l.links.iter()).filter(|l| l.source.follows_notes()).map(|l| l.from));
        for id in [set.clip_notes, set.look_notes].into_iter().flatten() {
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
        let bpb = project.beats_per_bar;
        self.clock.frame(beat, playing, bpb, dt);
        self.free_beat += dt as f64 * bpm / 60.0;
        let lfo_beat = if playing { beat } else { self.free_beat };

        // The Look: the set's own (a pad clicked and waiting for its beat,
        // the scenes, a track's notes, hits heard), then whoever has one of
        // their own: the part that plays, else its clip, else the sequence.
        self.pads.frame(beat, playing, dt as f64, 0.0);
        if set.look_scenes {
            let scene = kv::scene_clip(status.tracks.iter().take(project.tracks.len()).map(|t| t.playing_slot.load(Ordering::Relaxed)), n_looks);
            if let (Some(s), true) = (scene, scene != self.last_look_scene) {
                self.pads.go(s, 0.0);
            }
            self.last_look_scene = scene;
        }
        if let Some(note) = set.look_notes.and_then(|id| self.notes.get(&Some(id))).filter(|f| f.hit) {
            self.pads.go(kv::slice_for_note(note.pitch, n_looks as u32), 0.0);
        }
        if let Some(t) = set.look_hits {
            if let Some(b) = self.hit(HitUse::Looks, &t, dt, beat, playing, bpm) {
                let to = Step::of_hit(t.to, b).from(self.pads.clip, n_looks, &mut self.dice);
                self.pads.go(to, 0.0);
            }
        }
        self.pads.clip = self.pads.clip.min(n_looks - 1);
        let seq = set.look_seq.look_at(beat, bpb, n_looks).filter(|_| playing);
        let clip_look = playing_clip.and_then(|c| c.look);
        let part_look = playing_clip.zip(self.part_now).and_then(|(c, k)| c.part_look(k));
        let target = kv::look_for(part_look, clip_look, seq, self.pads.clip, n_looks);
        let morph = set.morph_beats.max(0.0) as f64 * 60.0 / bpm.max(1.0);
        self.looks.frame(beat, playing, dt as f64, morph);
        self.looks.go(target, morph);
        self.looks.clip = self.looks.clip.min(n_looks - 1);
        if self.looks.fade.is_some_and(|f| f.from >= n_looks) {
            self.looks.fade = None;
        }

        // The links of the Looks in use, and what they make of the picture.
        let now = self.looks.clip;
        let mut in_use = vec![now, set.look.min(n_looks - 1)];
        in_use.extend(self.looks.fade.map(|f| f.from));
        self.links.retain(|k, _| in_use.contains(&k.0));
        self.turned.retain(|k, _| in_use.contains(k));
        let readings = self.read_look(set, now, dt, lfo_beat);
        self.look = self.make_look(set, now, &readings, dt, bpm, bpb);
        self.look_from = None;
        if let Some(f) = self.looks.fade {
            let r = self.read_look(set, f.from, dt, lfo_beat);
            let a = self.make_look(set, f.from, &r, dt, bpm, bpb);
            self.look_from = Some((f.from, a, f.mix()));
        }
        let edited = set.look.min(n_looks - 1);
        // The meters show the Look edited (read once a frame, as it plays or not).
        self.readings = if edited == now {
            readings.iter().map(|r| r.value).collect()
        } else {
            let read_already = self.look_from.is_some_and(|f| f.0 == edited);
            self.read_look(set, edited, if read_already { 0.0 } else { dt }, lfo_beat).iter().map(|r| r.value).collect()
        };
        // Where the film is moves with the Look as it morphs.
        let moving = match &self.look_from {
            Some((_, a, m)) => Look::morph(a, &self.look, *m),
            None => self.look,
        };
        let def = set.looks.get(now).cloned().unwrap_or_default();

        // Which clip: a clip waiting for its beat, a scene that starts, a
        // note on the clips' track, a hit heard, Next Clip (these at once,
        // they come in time).
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
        if let Some(t) = set.clip_hits {
            if let (Some(b), true) = (self.hit(HitUse::Clips, &t, dt, beat, playing, bpm), n > 0) {
                let to = Step::of_hit(t.to, b).from(self.switch.clip, n, &mut self.dice);
                self.switch.go(to, secs);
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
        let now_clip = self.switch.clip;
        if self.on != Some(now_clip) {
            // The last clip keeps running while the new one comes in; the new one starts from its beginning.
            self.from_head = std::mem::take(&mut self.playhead);
            self.playhead.dice = Dice::new(self.from_head.dice.next());
            self.on = Some(now_clip);
        }

        // The hits heard that move the slices or the parts of the clip that plays.
        let (mut slice_step, mut part_step) = (None, None);
        if let Some(c) = set.clips.get(now_clip) {
            if let (kv::PlayMode::Slices, Some(t)) = (c.play.mode, c.play.slice_hits) {
                slice_step = self.hit(HitUse::Slices(now_clip), &t, dt, beat, playing, bpm).map(|b| Step::of_hit(t.to, b));
            }
            if c.play.seq.on && c.play.seq.order == kv::PartOrder::Hits {
                let t = c.play.seq.hits;
                part_step = self.hit(HitUse::Parts(now_clip), &t, dt, beat, playing, bpm).map(|b| Step::of_hit(t.to, b));
            }
        }
        self.hits.retain(|k, _| match k {
            HitUse::Slices(i) | HitUse::Parts(i) => *i == now_clip,
            HitUse::Clips => set.clip_hits.is_some(),
            HitUse::Looks => set.look_hits.is_some(),
        });

        // Where the films are. A film cut by Slice to Drum Rack follows the
        // notes of the track it was sliced to, and Gate the notes it holds.
        let fx = |e: Effect, k: usize| def.setting(e).param(e, k);
        let speed_top = fx(Effect::Speed, 0);
        let reach = fx(Effect::Scratch, 0);
        let stutter_beats = length_beats(fx(Effect::Stutter, 0));
        let input = |clip: &VideoClip, i: usize, notes: &HashMap<Option<u64>, NoteFollower>, look: &Look| {
            let cues = clip.cues.as_ref().filter(|c| !c.is_empty());
            let by = cues.map_or(clip.play.slice_track, |c| c.track);
            let mut slice_note = notes.get(&by).filter(|n| n.hit).map(|n| n.pitch);
            // Notes from C3 pick the parts; on the slices' own track they are not slices.
            let seq = &clip.play.seq;
            let part_note = if seq.on && seq.order == kv::PartOrder::Notes { notes.get(&seq.track).filter(|n| n.hit && n.pitch >= kv::PART_NOTE).map(|n| n.pitch) } else { None };
            if part_note.is_some() && seq.track == by {
                slice_note = None;
            }
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
                beats_per_bar: bpb,
                speed: look.get(Effect::Speed) * (speed_top - 1.0),
                freeze: look.get(Effect::Freeze),
                scratch: look.get(Effect::Scratch) * reach,
                jump: look.jump,
                stutter: look.stutter,
                stutter_beats,
                note: if clip.play.mode == kv::PlayMode::Slices && clip.play.slice_hits.is_none() { slice_note } else { None },
                slice_step: None,
                part_note,
                part_step: None,
                held,
                seek: None,
                hold: false,
            }
        };
        let mut shot = Shot { clip: now_clip, beat: if playing { beat } else { self.free_beat }, look: now, ..Default::default() };
        if let Some(clip) = asks.audition.or(set.clips.get(now_clip)) {
            let mut inp = input(clip, now_clip, &self.notes, &moving);
            inp.seek = asks.seek;
            inp.hold = asks.hold;
            inp.slice_step = slice_step;
            inp.part_step = part_step;
            shot.at = self.playhead.advance(clip, &inp);
            // The part it is in now (its Look, next frame).
            self.part_now = if clip.is_cut() { self.playhead.part.or_else(|| set.clips.get(now_clip).and_then(|c| c.part_at(shot.at as f32))) } else { None };
        }
        if let Some(f) = self.switch.fade {
            if let Some(clip) = set.clips.get(f.from) {
                // It goes on as it went, without the hits (they are the new one's).
                let mut inp = input(clip, f.from, &self.notes, &Look::default());
                inp.note = None;
                inp.part_note = None;
                shot.from = Some((f.from, self.from_head.advance(clip, &inp), f.mix()));
            }
        }
        shot
    }
}
