//! The Princess Sounds on this computer: made in the background the first
//! time they are wanted (see `krabton_engine::princess_sounds`), and the
//! debug script's actions for them, Flipside, the Fryer and Analog
//! Princess's Sample voice.

use crate::app::{App, BrowserItem, DropTarget, TrackSel};
use crate::ui::faces::flip::Export;
use krabton_engine::dsp::princess_flip as pf;
use krabton_engine::princess_sounds as ps;
use krabton_engine::project::{Clip, ClipContent};
use krabton_engine::Instrument;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Making { done: usize, total: usize },
    Ready,
    Failed(String),
}

pub type Install = Arc<Mutex<State>>;

/// Starts making the sounds not made yet; the state tells how far it is.
pub fn start() -> Install {
    let total = ps::sounds().len();
    let state: Install = Arc::new(Mutex::new(State::Making { done: 0, total }));
    let s = state.clone();
    std::thread::spawn(move || {
        let dir = ps::dir();
        let progress = |done: usize, total: usize| {
            if let Ok(mut st) = s.lock() {
                *st = State::Making { done, total };
            }
        };
        let result = ps::install(&dir, &progress);
        if let Ok(mut st) = s.lock() {
            *st = match result {
                Ok(()) => State::Ready,
                Err(e) => State::Failed(e),
            };
        }
    });
    state
}

/// Two numbers of a script's argument (`A,B`).
fn two(arg: &str) -> (f64, f64) {
    let v: Vec<f64> = arg.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    (v.first().copied().unwrap_or(0.0), v.get(1).copied().unwrap_or(0.0))
}

impl App {
    /// An auto-flip: four bars that play track `t`'s Flipside chops in a new
    /// order, as `style` feels, a clip in the selected scene's slot (what was
    /// there goes; undo brings it back). Returns what to say.
    pub(crate) fn flip_auto_clip(&mut self, t: usize, style: pf::AutoStyle) -> Option<String> {
        let bpb = self.project.beats_per_bar_f();
        let bpm = self.project.bpm;
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(7, |d| d.subsec_nanos()) ^ self.frame as u32;
        let notes = match self.project.tracks.get(t).and_then(|tr| tr.instrument.as_ref()) {
            Some(Instrument::Flip { sample: Some(s), flip, .. }) if !flip.chops.is_empty() => pf::auto_flip(flip, s.sample_rate, bpm, bpb, style, 4, seed),
            Some(Instrument::Flip { .. }) => return Some("Flip a sample first: drop one on Flipside".into()),
            _ => return None,
        };
        let slot = self.sel_scene;
        while slot >= self.project.scenes.len() {
            let n = self.project.scenes.len();
            self.project.add_scene(n);
        }
        let id = self.project.alloc_id();
        let color = self.project.tracks[t].color;
        let mut clip = Clip::new_midi(id, format!("Flip {}", style.name()), color, 4.0 * bpb);
        clip.content = ClipContent::Midi { notes };
        self.project.tracks[t].slots[slot] = Some(clip);
        self.sel_clip = Some(crate::app::ClipSel::Slot { track: t, scene: slot });
        Some(format!("Auto-flip ({}): four bars in scene {}'s slot", style.name().to_lowercase(), slot + 1))
    }

    /// Track `t`'s Flipside chops on a new track after it: the pads of a
    /// Drum Rack (in the song's key as they play, sixteen at most) or the
    /// tracks of an Analog Princess as its Sample voice (twelve at most).
    pub(crate) fn flip_export(&mut self, t: usize, to: Export) -> Option<String> {
        let scale = self.project.scale;
        let tr = self.project.tracks.get(t)?;
        let color = tr.color;
        let Some(Instrument::Flip { params, sample: Some(s), flip, .. }) = tr.instrument.as_ref() else {
            return Some("Flip a sample first: drop one on Flipside".into());
        };
        if flip.chops.is_empty() {
            return Some("There are no chops to send".into());
        }
        let shift = match flip.key {
            Some(k) if params.get(pf::SONG_KEY).is_some_and(|v| *v >= 0.5) && scale.kind != krabton_engine::scale::ScaleKind::Chromatic => pf::key_shift(k, scale),
            _ => 0,
        };
        let gate = params.get(pf::MODE).is_some_and(|v| *v >= 0.5);
        let name = s.name.clone();
        let (inst, count, what) = match to {
            Export::DrumRack => {
                let pads = pf::drum_pads(flip, s, shift, gate);
                let n = pads.len();
                (Instrument::Drums { kit: format!("{name} Chops"), pads, engine: krabton_engine::dsp::drums::DrumEngine::Rack }, n, "a Drum Rack")
            }
            Export::Princess => {
                use krabton_engine::dsp::princess_machine as pm;
                let Instrument::Princess { preset, mut params, mut machine } = Instrument::princess("Init") else { return None };
                let n = flip.chops.len().min(pm::TRACKS);
                for (k, c) in flip.chops.iter().take(n).enumerate() {
                    crate::ui::faces::princess::put_on(&mut params, &mut machine, k, pf::chop_sample(s, c, k));
                    params[pm::tp(k, pm::T_TUNE)] = (c.pitch + c.fine / 100.0 + shift as f32).clamp(-24.0, 24.0);
                    params[pm::tp(k, pm::T_PAN)] = c.pan;
                    params[pm::tp(k, pm::T_LEVEL)] = c.gain.clamp(-40.0, 6.0);
                    params[pm::tp(k, pm::T_CHOKE)] = c.choke.min(4) as f32;
                    params[pm::tp(k, pm::T_B)] = if c.reverse { 1.0 } else { 0.0 };
                }
                (Instrument::Princess { preset, params, machine }, n, "Analog Princess")
            }
        };
        let at = self.project.insert_midi_track(t + 1, format!("{name} Chops"), Some(inst));
        self.project.tracks[at].color = color;
        self.sel_track = TrackSel::Track(at);
        self.detail = crate::app::DetailTab::Devices;
        Some(format!("{count} chops on {what} on a new track"))
    }

    /// The selected track's instrument.
    fn selected_instrument(&mut self) -> Option<&mut Instrument> {
        match self.sel_track {
            TrackSel::Track(t) => self.project.tracks.get_mut(t).and_then(|tr| tr.instrument.as_mut()),
            _ => None,
        }
    }

    /// Script actions for the Princess tools (see `run_script`); whether
    /// `name` was one.
    pub(crate) fn princess_script(&mut self, name: &str, arg: &str) -> bool {
        let nums: Vec<usize> = arg.split(',').filter_map(|v| v.trim().parse().ok()).collect();
        match name {
            // flip=N (or flipside=N): a new track with Flipside preset N.
            "flip" | "flipside" => {
                let names = pf::preset_names();
                let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                self.apply_item(&BrowserItem::Flip(p), DropTarget::NewTrack);
                self.detail = crate::app::DetailTab::Devices;
            }
            // flipchop=BY,COUNT: the selected Flipside chopped again, all of it, at
            // its hits (0), in bars (1), in equal parts (2) or by hand only (3).
            "flipchop" => {
                let by = pf::ChopBy::ALL[nums.first().copied().unwrap_or(0).min(3)];
                let count = nums.get(1).copied().unwrap_or(12);
                if let Some(Instrument::Flip { sample, flip, .. }) = self.selected_instrument() {
                    flip.chopping.by = by;
                    flip.chopping.count = count.clamp(1, pf::MAX_CHOPS);
                    crate::ui::faces::flip::rechop(sample.as_ref(), flip, true);
                }
            }
            // flipgrid=G: the grid of chopping in bars (0 Auto, 1 a bar ... 5 a
            // sixteenth); flipsens=S: how readily a hit counts (0..1). Neither
            // chops again.
            "flipgrid" => {
                if let Some(Instrument::Flip { flip, .. }) = self.selected_instrument() {
                    flip.chopping.grid = pf::Grid::ALL[nums.first().copied().unwrap_or(0).min(pf::Grid::ALL.len() - 1)];
                }
            }
            "flipsens" => {
                if let Some(Instrument::Flip { flip, .. }) = self.selected_instrument() {
                    flip.chopping.sensitivity = arg.trim().parse::<f32>().unwrap_or(0.5).clamp(0.0, 1.0);
                }
            }
            // fliprev=K: chop K (1 up) of the selected Flipside turned round.
            "fliprev" => {
                let k = nums.first().copied().unwrap_or(1).max(1) - 1;
                if let Some(Instrument::Flip { flip, .. }) = self.selected_instrument() {
                    if let Some(c) = flip.chops.get_mut(k) {
                        c.reverse = !c.reverse;
                    }
                }
            }
            // flipsel=K: chop K (1 up) chosen (for the chop panel and the keys below).
            "flipsel" => {
                let k = nums.first().copied().unwrap_or(1).clamp(1, pf::MAX_CHOPS);
                if let Some(Instrument::Flip { params, .. }) = self.selected_instrument() {
                    if let Some(v) = params.get_mut(pf::CHOP) {
                        *v = k as f32;
                    }
                }
            }
            // flipmove=K,AT: chop K's line moved to AT (0..1 of the sample) as a
            // drag lets it go (snapped; flipmovefree=K,AT: not snapped).
            "flipmove" | "flipmovefree" => {
                let (k, at) = two(arg);
                let free = name == "flipmovefree";
                if let Some(Instrument::Flip { sample: Some(s), flip, .. }) = self.selected_instrument() {
                    let n = s.frames();
                    let to = (at.clamp(0.0, 1.0) * n as f64) as u64;
                    let to = if free { to } else { pf::snap(s, to, (0.004 * s.sample_rate as f64) as u64) };
                    let went = flip.move_line((k as usize).max(1) - 1, to, n);
                    println!("flipmove: chop {k} at {went} of {n}");
                }
            }
            // flipadd=AT: a new line at AT (0..1 of the sample).
            "flipadd" => {
                let at = arg.trim().parse::<f64>().unwrap_or(0.5);
                if let Some(Instrument::Flip { sample: Some(s), flip, params, .. }) = self.selected_instrument() {
                    let n = s.frames();
                    if let Some(k) = flip.add_line((at.clamp(0.0, 1.0) * n as f64) as u64, n) {
                        params[pf::CHOP] = (k + 1) as f32;
                        println!("flipadd: chop {}", k + 1);
                    }
                }
            }
            // flipdel=K: chop K's line taken away.
            "flipdel" => {
                let k = nums.first().copied().unwrap_or(1).max(1) - 1;
                if let Some(Instrument::Flip { flip, .. }) = self.selected_instrument() {
                    flip.delete_line(k);
                }
            }
            // flipset=K,SETTING,VALUE: chop K's setting (0 pitch ... 8 tone, as
            // the chop panel's faders; 9 its way of playing 0..4, 10 its choke
            // group, 11 stretch).
            "flipset" => {
                let v: Vec<f32> = arg.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                let (k, which, val) = (v.first().copied().unwrap_or(1.0) as usize, v.get(1).copied().unwrap_or(0.0) as usize, v.get(2).copied().unwrap_or(0.0));
                if let Some(Instrument::Flip { flip, .. }) = self.selected_instrument() {
                    if let Some(c) = flip.chops.get_mut(k.max(1) - 1) {
                        match which {
                            9 => c.play = pf::Play::ALL[(val as usize).min(4)],
                            10 => c.choke = (val as u8).min(4),
                            11 => c.stretch = val >= 0.5,
                            w => {
                                if let Some(spec) = pf::CHOP_SPECS.get(w) {
                                    if let Some(x) = c.setting(w) {
                                        *x = spec.clamp(val);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // flipzoom=Z,FROM: the wave zoomed in Z times, from FROM (0..1) of it.
            "flipzoom" => {
                let (zoom, from) = two(arg);
                crate::ui::faces::flip::script_view(crate::ui::faces::flip::View { zoom: (zoom as f32).clamp(1.0, 256.0), from });
            }
            // flipshuffle, flipsort, fliprandom: the chops' order or their sound.
            "flipshuffle" | "flipsort" | "fliprandom" => {
                if let Some(Instrument::Flip { flip, .. }) = self.selected_instrument() {
                    match name {
                        "flipshuffle" => flip.shuffle(17),
                        "flipsort" => flip.sort(),
                        _ => flip.randomize(17),
                    }
                }
            }
            // flipauto=S: an auto-flip (0 straight, 1 stutter, 2 bounce) in the selected slot.
            "flipauto" => {
                if let TrackSel::Track(t) = self.sel_track {
                    let style = pf::AutoStyle::ALL[nums.first().copied().unwrap_or(0).min(2)];
                    if let Some(msg) = self.flip_auto_clip(t, style) {
                        println!("flipauto: {msg}");
                    }
                }
            }
            // flipexport=rack|princess: the chops onto a new Drum Rack or Analog Princess.
            "flipexport" => {
                if let TrackSel::Track(t) = self.sel_track {
                    let to = if arg.trim() == "princess" { Export::Princess } else { Export::DrumRack };
                    if let Some(msg) = self.flip_export(t, to) {
                        println!("flipexport: {msg}");
                    }
                }
            }
            // flipinfo: prints what the selected Flipside heard and its chops.
            "flipinfo" => {
                if let Some(Instrument::Flip { sample, flip, .. }) = self.selected_instrument() {
                    println!("flip: {} key {:?} bpm {:?} {:?} chops {:?}", sample.as_ref().map_or("-".to_string(), |s| s.name.clone()), flip.key_name(), flip.bpm, flip.chopping, flip.chops.iter().map(|c| (c.start, c.end, c.reverse, c.manual)).collect::<Vec<_>>());
                }
            }
            // fryer: a Fryer on the selected chain; fryerhold=stutter|tape|surf|none.
            "fryer" => {
                let dev = self.project.make_device(krabton_engine::dsp::effects::EffectKind::Fryer);
                if let Some(chain) = self.chain_mut(self.sel_track) {
                    chain.push(dev);
                }
                self.detail = crate::app::DetailTab::Devices;
            }
            "fryerhold" => {
                use krabton_engine::dsp::fx::fryer as fr;
                let want = match arg.trim() {
                    "stutter" => Some(fr::STUTTER_HOLD),
                    "tape" => Some(fr::TAPE_HOLD),
                    "surf" => Some(fr::SURF_HOLD),
                    _ => None,
                };
                if let Some(d) = self.chain_mut(self.sel_track).and_then(|c| c.iter_mut().find(|d| d.kind == krabton_engine::dsp::effects::EffectKind::Fryer)) {
                    for k in [fr::STUTTER_HOLD, fr::TAPE_HOLD, fr::SURF_HOLD] {
                        if let Some(v) = d.params.get_mut(k) {
                            *v = if want == Some(k) { 1.0 } else { 0.0 };
                        }
                    }
                }
            }
            // princesstrack=T: the selected Analog Princess's track T (0 to 11) chosen.
            "princesstrack" => self.sel_pad = nums.first().copied().unwrap_or(0).min(krabton_engine::dsp::princess_machine::TRACKS - 1),
            // princesssample=T,PATH: that sample on track T of the selected Analog Princess.
            "princesssample" => {
                let (t, path) = arg.split_once(',').unwrap_or(("0", arg));
                let t = t.trim().parse().unwrap_or(0);
                match krabton_engine::SampleRef::load(&PathBuf::from(path.trim())) {
                    Ok(s) => {
                        if let Some(Instrument::Princess { params, machine, .. }) = self.selected_instrument() {
                            crate::ui::faces::princess::put_on(params, machine, t, s);
                        }
                        self.sel_pad = t;
                    }
                    Err(e) => eprintln!("princesssample: {e}"),
                }
            }
            // princesssound=T,N: Princess Sound N (of the one-shots) on track T.
            "princesssound" => {
                let (t, n) = (nums.first().copied().unwrap_or(0), nums.get(1).copied().unwrap_or(0));
                let shots = ps::one_shots();
                if let Some(s) = shots.get(n.min(shots.len() - 1)) {
                    match ps::ensure(&ps::dir(), s).and_then(|p| krabton_engine::SampleRef::load(&p)) {
                        Ok(smp) => {
                            if let Some(Instrument::Princess { params, machine, .. }) = self.selected_instrument() {
                                crate::ui::faces::princess::put_on(params, machine, t, smp);
                            }
                            self.sel_pad = t;
                        }
                        Err(e) => eprintln!("princesssound: {e}"),
                    }
                }
            }
            // princesssounds: makes the library now (waits), then prints where.
            "princesssounds" => {
                let dir = ps::dir();
                match ps::install(&dir, &|_, _| {}) {
                    Ok(()) => println!("princess sounds: {} in {}", ps::sounds().len(), dir.display()),
                    Err(e) => eprintln!("princess sounds: {e}"),
                }
            }
            // browsersamples=FOLDER: the browser on Samples, Princess Sounds open (and that folder of it).
            "browsersamples" => {
                use crate::ui::browser::{Category, Section};
                self.browser.visible = true;
                self.browser.section = Section::Category(Category::Samples);
                let root = format!("{:?}/{}", Section::Category(Category::Samples), ps::FOLDER);
                self.browser.open.insert(root.clone());
                if !arg.trim().is_empty() {
                    self.browser.open.insert(format!("{root}/{}", arg.trim()));
                }
            }
            _ => return false,
        }
        true
    }
}
