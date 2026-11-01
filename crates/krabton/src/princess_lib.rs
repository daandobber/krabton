//! The Princess Sounds on this computer: made in the background the first
//! time they are wanted (see `krabton_engine::princess_sounds`), and the
//! debug script's actions for them, Princess Flip, the Fryer and Analog
//! Princess's Sample voice.

use crate::app::{App, BrowserItem, DropTarget, TrackSel};
use krabton_engine::princess_sounds as ps;
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

impl App {
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
            // flip=N: a new track with Princess Flip preset N.
            "flip" => {
                let names = krabton_engine::dsp::princess_flip::preset_names();
                let p = names[nums.first().copied().unwrap_or(0).min(names.len() - 1)];
                self.apply_item(&BrowserItem::Flip(p), DropTarget::NewTrack);
                self.detail = crate::app::DetailTab::Devices;
            }
            // flipchop=0|1,COUNT: the selected Flip chopped again at its hits (0) or in bars (1).
            "flipchop" => {
                let bars = nums.first().copied().unwrap_or(0) == 1;
                let count = nums.get(1).copied().unwrap_or(12);
                if let Some(Instrument::Flip { sample, flip, .. }) = self.selected_instrument() {
                    if let Some(s) = sample.clone() {
                        crate::ui::faces::flip::flip_onto(sample, flip, s, bars, count);
                    }
                }
            }
            // fliprev=K: chop K (1 up) of the selected Flip turned round.
            "fliprev" => {
                let k = nums.first().copied().unwrap_or(1).max(1) - 1;
                if let Some(Instrument::Flip { flip, .. }) = self.selected_instrument() {
                    if let Some(c) = flip.chops.get_mut(k) {
                        c.reverse = !c.reverse;
                    }
                }
            }
            // flipinfo: prints what the selected Flip heard.
            "flipinfo" => {
                if let Some(Instrument::Flip { sample, flip, .. }) = self.selected_instrument() {
                    println!("flip: {} key {:?} bpm {:?} chops {:?}", sample.as_ref().map_or("-".to_string(), |s| s.name.clone()), flip.key_name(), flip.bpm, flip.chops.iter().map(|c| (c.start, c.end, c.reverse)).collect::<Vec<_>>());
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
