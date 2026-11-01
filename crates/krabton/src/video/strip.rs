//! The frames of a film along the Video's timeline: small pictures made
//! from its frames in the background (unpacking a frame takes a few
//! milliseconds, too long for a whole strip in one frame of the window),
//! the newest asked for first, and kept.

use super::media::Media;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// How wide each small picture is (pixels).
pub const THUMB_W: usize = 96;
/// Kept at most; past that they are let go and made again as needed.
const MAX_KEPT: usize = 600;

/// A film's small pictures, by frame.
pub struct Strip {
    media: Arc<Media>,
    /// Asked for and not made yet (the last first).
    wanted: Arc<Mutex<Vec<usize>>>,
    made: Arc<Mutex<Vec<(usize, egui::ColorImage)>>>,
    busy: Arc<AtomicBool>,
    asked: HashSet<usize>,
    tex: HashMap<usize, egui::TextureHandle>,
}

impl Strip {
    pub fn new(media: Arc<Media>) -> Self {
        Self { media, wanted: Default::default(), made: Default::default(), busy: Default::default(), asked: HashSet::new(), tex: HashMap::new() }
    }

    /// The small picture of frame `i`, or None while it is being made (it is asked for).
    pub fn get(&mut self, i: usize) -> Option<&egui::TextureHandle> {
        if !self.tex.contains_key(&i) && self.asked.insert(i) {
            if let Ok(mut w) = self.wanted.lock() {
                w.push(i);
                // Asked for long ago (the timeline moved on): forgotten, asked again if seen again.
                if w.len() > 200 {
                    let old = w.remove(0);
                    self.asked.remove(&old);
                }
            }
        }
        self.tex.get(&i)
    }

    /// Puts up what was made, and sets the worker going on what is asked for.
    pub fn frame(&mut self, ctx: &egui::Context) {
        let made: Vec<(usize, egui::ColorImage)> = self.made.lock().map(|mut m| std::mem::take(&mut *m)).unwrap_or_default();
        if !made.is_empty() {
            if self.tex.len() + made.len() > MAX_KEPT {
                self.tex.clear();
                self.asked.clear();
            }
            for (i, img) in made {
                let t = ctx.load_texture(format!("video_strip_{}_{i}", self.media.path.display()), img, egui::TextureOptions::LINEAR);
                self.tex.insert(i, t);
            }
            ctx.request_repaint();
        }
        let waiting = self.wanted.lock().map(|w| !w.is_empty()).unwrap_or(false);
        if waiting && !self.busy.swap(true, Ordering::AcqRel) {
            let (media, wanted, made, busy) = (self.media.clone(), self.wanted.clone(), self.made.clone(), self.busy.clone());
            std::thread::spawn(move || {
                loop {
                    let next = wanted.lock().ok().and_then(|mut w| w.pop());
                    let Some(i) = next else { break };
                    if let Some(img) = media.thumbnail_of(i, THUMB_W) {
                        if let Ok(mut m) = made.lock() {
                            m.push((i, img));
                        }
                    }
                }
                busy.store(false, Ordering::Release);
            });
        }
        if waiting || self.busy.load(Ordering::Acquire) {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }
    }
}
