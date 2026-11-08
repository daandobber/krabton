//! The Brain's face: its brain as a field of blocks (across by how bright,
//! up by how loud, coloured by the sound they come from), the block playing
//! lit with its synapses and the last ones it played trailing; under it the
//! sounds in the brain (drop audio from the browser, or add files or a
//! folder). The brain is made away from the UI (`App::poll_brains`) and
//! handed to the engine on the device.

use super::devices::param_grid_mod;
use super::modulation::ModView;
use crate::app::{App, BrowserItem};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, RichText, Sense, Stroke, Ui};
use krabton_engine::dsp::effects::EffectKind;
use krabton_engine::dsp::fx::brain::{self as kb, BrainData};
use krabton_engine::project::{BrainRef, Device};
use krabton_engine::sample::SampleRef;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

/// A brain being made.
pub(crate) struct BrainJob {
    key: u64,
    cancel: Arc<AtomicBool>,
    done: mpsc::Receiver<Option<BrainData>>,
}

/// What the face asks for.
pub(crate) enum BrainAct {
    Add(Vec<PathBuf>),
    Remove(usize),
    Clear,
}

/// The audio files in a folder (and the folders in it), up to `most`.
pub(crate) fn audio_in(dir: &std::path::Path, most: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                todo.push(p);
            } else if p.extension().and_then(|e| e.to_str()).is_some_and(|e| ["wav", "flac", "mp3", "aif", "aiff", "ogg"].contains(&e.to_ascii_lowercase().as_str())) {
                out.push(p);
                if out.len() >= most {
                    return out;
                }
            }
        }
    }
    out
}

/// The colour of sound `k` of the brain.
fn sound_colour(k: usize) -> Color32 {
    let h = (k as f32 * 0.618_034 + 0.08).fract();
    egui::ecolor::Hsva::new(h, 0.6, 0.95, 1.0).into()
}

/// The Brain's face. Whether a setting changed, and what it asks for.
#[allow(clippy::too_many_arguments)]
pub(crate) fn brain_panel(ui: &mut Ui, id: Id, dev_id: u64, params: &mut [f32], sounds: &[SampleRef], brain: Option<&Arc<BrainData>>, making: bool, accent: Color32, h: f32, mv: Option<&mut ModView>, status: &krabton_engine::EngineStatus, width: &mut f32) -> (bool, Option<BrainAct>) {
    let mut act = None;
    let changed = ui
        .horizontal_top(|ui| {
            ui.vertical(|ui| {
                let list_h = 44.0;
                let (rect, resp) = ui.allocate_exact_size(vec2(*width, (h - 30.0 - list_h).max(80.0)), Sense::click());
                let p = ui.painter().with_clip_rect(rect);
                rect_fill(&p, rect, 4.0, Color32::from_rgb(0x0c, 0x0d, 0x14));
                let field = rect.shrink2(vec2(10.0, 10.0));
                let at = |b: &kb::BrainBlock| pos2(field.left() + b.x * field.width(), field.top() + b.y * field.height());
                let meters = status.meters.read(dev_id);
                match brain.filter(|b| !b.blocks.is_empty()) {
                    Some(b) => {
                        // Every block, a dot in its sound's colour (a big brain: some of them).
                        let step = (b.blocks.len() / 3000).max(1);
                        for blk in b.blocks.iter().step_by(step) {
                            p.circle_filled(at(blk), 1.2, sound_colour(blk.sound as usize).gamma_multiply(0.45));
                        }
                        if let Some(m) = meters {
                            let now = m[0];
                            if now >= 0.0 && (now as usize) < b.blocks.len() {
                                let i = now as usize;
                                let here = at(&b.blocks[i]);
                                // Its synapses.
                                let k = (params[kb::SYNAPSES].round() as usize).clamp(1, kb::MAX_SYNAPSES);
                                for &s in &b.synapses(i)[..k] {
                                    if let Some(o) = b.blocks.get(s as usize) {
                                        p.line_segment([here, at(o)], Stroke::new(1.0_f32, accent.gamma_multiply(0.25)));
                                    }
                                }
                                // The last ones it played, fading.
                                let mut last = here;
                                for (n, v) in m[4..4 + kb::SHOWN].iter().enumerate().skip(1) {
                                    if *v < 0.0 || *v as usize >= b.blocks.len() {
                                        break;
                                    }
                                    let q = at(&b.blocks[*v as usize]);
                                    let fade = 1.0 - n as f32 / kb::SHOWN as f32;
                                    p.line_segment([last, q], Stroke::new(1.5_f32, sound_colour(b.blocks[*v as usize].sound as usize).gamma_multiply(0.7 * fade)));
                                    last = q;
                                }
                                let c = sound_colour(b.blocks[i].sound as usize);
                                p.circle_filled(here, 5.0 + 4.0 * m[2], c);
                                p.circle_stroke(here, 9.0 + 6.0 * m[3].min(1.0) * 4.0, Stroke::new(1.5_f32, c.gamma_multiply(0.6)));
                                let name = b.sounds.get(b.blocks[i].sound as usize).map_or("", |s| s.name.as_str());
                                text(&p, pos2(rect.left() + 8.0, rect.bottom() - 10.0), Align2::LEFT_CENTER, format!("{name} · {:.0} % alike", m[2] * 100.0), small(), TEXT);
                            }
                        }
                        text(&p, pos2(rect.right() - 8.0, rect.top() + 10.0), Align2::RIGHT_CENTER, format!("{} blocks", b.blocks.len()), small(), TEXT_DIM);
                    }
                    None => {
                        let say = if making { "Making the brain..." } else if sounds.is_empty() { "Drop sounds here (or add files or a folder): the brain the track is played out of" } else { "No brain yet" };
                        text(&p, rect.center(), Align2::CENTER_CENTER, say, small(), TEXT_DIM);
                    }
                }
                if making && brain.is_some() {
                    text(&p, pos2(rect.left() + 8.0, rect.top() + 10.0), Align2::LEFT_CENTER, "Making it again...", small(), TEXT_DIM);
                }
                // Sounds dropped from the browser.
                if let Some(item) = drop_payload::<BrowserItem>(&resp) {
                    if let BrowserItem::Sample(path) = &*item {
                        act = Some(BrainAct::Add(vec![path.clone()]));
                    }
                }
                if resp.dnd_hover_payload::<BrowserItem>().is_some() {
                    rect_line(ui.painter(), rect, 2.0, 2.0, accent_now());
                }
                resp.on_hover_text("The brain: each dot a block of its sounds (across: how bright, up: how loud). The block playing is lit, with its synapses and the ones played before it. Drop audio from the browser on it to add it. Drag the right edge to make it wider");
                width_grip(ui, id.with("brain_width"), rect, width, 260.0, 1200.0);
                // The sounds in it.
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if ui.small_button("Add sounds...").on_hover_text("Audio files for the brain (WAV, FLAC, MP3, AIFF, OGG)").clicked() {
                        if let Some(paths) = crate::import::audio_file_dialog().pick_files() {
                            act = Some(BrainAct::Add(paths));
                        }
                    }
                    if ui.small_button("Add folder...").on_hover_text("Every audio file in a folder (and the folders in it), up to 200").clicked() {
                        if let Some(dir) = rfd::FileDialog::new().set_title("A folder of sounds for the brain").pick_folder() {
                            act = Some(BrainAct::Add(audio_in(&dir, 200)));
                        }
                    }
                    if !sounds.is_empty() && ui.small_button("Clear").on_hover_text("Empty the brain").clicked() {
                        act = Some(BrainAct::Clear);
                    }
                    ui.label(RichText::new(format!("{} sound{}", sounds.len(), if sounds.len() == 1 { "" } else { "s" })).small().color(TEXT_DIM));
                });
                egui::ScrollArea::horizontal().id_salt(id.with("brain_sounds")).max_width(*width).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        for (k, s) in sounds.iter().enumerate() {
                            let chip = egui::Button::new(RichText::new(format!("{}  x", s.name)).small().color(TEXT)).fill(sound_colour(k).gamma_multiply(0.3)).corner_radius(8.0);
                            if ui.add(chip).on_hover_text("Take it out of the brain").clicked() {
                                act = Some(BrainAct::Remove(k));
                            }
                        }
                    });
                });
            });
            ui.add_space(6.0);
            param_grid_mod(ui, id, EffectKind::Brain.specs(), params, accent, h - 24.0, mv)
        })
        .inner;
    (changed, act)
}

impl App {
    /// Every Brain in the set, by its device: made (again) when its sounds or
    /// its block change, away from the UI; handed over when it is done.
    pub(crate) fn poll_brains(&mut self) {
        let sr = self.status().sample_rate.load();
        if sr <= 0.0 {
            return;
        }
        // (device id, what its brain must be made of)
        let mut wanted: Vec<(u64, u64, Vec<SampleRef>, usize)> = Vec::new();
        let mut look = |d: &Device| {
            if d.kind == EffectKind::Brain {
                let block = kb::BLOCKS[(d.params.get(kb::BLOCK).copied().unwrap_or(2.0).round().max(0.0) as usize).min(kb::BLOCKS.len() - 1)];
                let key = kb::key_of(&d.sounds, block, sr);
                let has = d.brain.0.as_ref().map(|b| b.key);
                if has != Some(key) {
                    wanted.push((d.id, key, d.sounds.clone(), block));
                }
            }
        };
        for t in &self.project.tracks {
            t.devices.iter().for_each(&mut look);
        }
        self.project.master.devices.iter().for_each(&mut look);
        let alive: Vec<u64> = wanted.iter().map(|w| w.0).collect();
        // Jobs no longer wanted stop.
        self.brain_jobs.retain(|id, job| {
            let keep = alive.contains(id);
            if !keep {
                job.cancel.store(true, Ordering::Relaxed);
            }
            keep
        });
        for (id, key, sounds, block) in wanted {
            // No sounds: no brain.
            if sounds.is_empty() {
                self.set_brain(id, BrainRef(None));
                continue;
            }
            if let Some(job) = self.brain_jobs.get(&id) {
                if job.key == key {
                    match job.done.try_recv() {
                        Ok(Some(b)) => {
                            self.brain_jobs.remove(&id);
                            self.set_brain(id, BrainRef(Some(Arc::new(b))));
                        }
                        Ok(None) | Err(mpsc::TryRecvError::Disconnected) => {
                            self.brain_jobs.remove(&id);
                        }
                        Err(mpsc::TryRecvError::Empty) => {}
                    }
                    continue;
                }
                job.cancel.store(true, Ordering::Relaxed);
            }
            let cancel = Arc::new(AtomicBool::new(false));
            let (tx, rx) = mpsc::channel();
            let stop = cancel.clone();
            std::thread::spawn(move || {
                let _ = tx.send(kb::build(&sounds, block, sr, &stop));
            });
            self.brain_jobs.insert(id, BrainJob { key, cancel, done: rx });
        }
    }

    /// Whether the brain of device `id` is being made.
    pub(crate) fn brain_making(&self, id: u64) -> bool {
        self.brain_jobs.contains_key(&id)
    }

    fn set_brain(&mut self, id: u64, brain: BrainRef) {
        let set = |d: &mut Device| {
            if d.id == id && d.brain != brain {
                d.brain = brain.clone();
            }
        };
        for t in &mut self.project.tracks {
            t.devices.iter_mut().for_each(set);
        }
        self.project.master.devices.iter_mut().for_each(set);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_gives_its_audio_files_and_those_in_its_folders() {
        let dir = std::env::temp_dir().join(format!("krabton-brain-test-{}", std::process::id()));
        let sub = dir.join("more");
        std::fs::create_dir_all(&sub).unwrap();
        for f in [dir.join("a.wav"), dir.join("notes.txt"), sub.join("b.FLAC"), sub.join("c.mp3")] {
            std::fs::write(f, b"x").unwrap();
        }
        let mut got: Vec<String> = audio_in(&dir, 10).iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect();
        got.sort();
        assert_eq!(got, ["a.wav", "b.FLAC", "c.mp3"]);
        assert_eq!(audio_in(&dir, 2).len(), 2, "no more than asked");
        let _ = std::fs::remove_dir_all(&dir);
        assert_ne!(sound_colour(0), sound_colour(1));
    }
}
