//! The tape on the UI side: arming a track (its memory made, a copy kept for
//! undo), the edits (lift a bar or the loop out, drop it back somewhere,
//! erase it, undo and redo), telling the engine which track is played, and
//! the tape saved as WAV files next to the set and loaded with it.

use crate::app::{App, TrackSel};
use krabton_engine::engine::Command;
use krabton_engine::sample::SampleData;
use krabton_engine::tape::{Reel, TAPE_TRACKS};
use std::path::PathBuf;

impl App {
    /// Arms track `k` to record (or disarms it when it is armed already).
    pub fn tape_arm(&mut self, k: usize) {
        if k >= TAPE_TRACKS {
            return;
        }
        if self.project.tape.armed == Some(k) {
            self.project.tape.armed = None;
            return;
        }
        let st = self.status();
        st.reel.spool_or_make(k);
        st.reel.snapshot(k);
        self.project.tape.armed = Some(k);
    }

    /// Before the transport starts: a copy of the armed track, so the take can be undone.
    pub(crate) fn tape_before_play(&mut self) {
        if let Some(k) = self.project.tape.armed {
            self.status().reel.snapshot(k);
        }
    }

    /// Frames of the bar the tape is in, or of the loop when it is on.
    pub fn tape_region(&self) -> (usize, usize) {
        let st = self.status();
        let sr = st.reel.rate() as f64;
        let t = &self.project.tape;
        if t.loop_on && t.loop_out > t.loop_in + 0.01 {
            return ((t.loop_in * sr) as usize, (t.loop_out * sr) as usize);
        }
        let bar = self.project.beats_per_bar_f() * 60.0 / self.project.bpm.max(1.0) * sr;
        let pos = st.reel.pos.load();
        let a = (pos / bar).floor() * bar;
        (a as usize, (a + bar) as usize)
    }

    /// Takes the bar (or the loop) out of track `k`, to drop elsewhere.
    pub fn tape_lift(&mut self, k: usize) {
        let st = self.status();
        let Some(s) = st.reel.spool(k) else { return };
        let (a, b) = self.tape_region();
        st.reel.snapshot(k);
        self.tape_clip = Some(s.copy_raw(a, b));
        s.erase(a, b);
        self.toast(format!("Lifted {:.1} s from track {}", (b - a) as f32 / st.reel.rate(), k + 1));
    }

    /// Puts what was lifted at the start of the bar (or loop) on track `k`.
    pub fn tape_drop(&mut self, k: usize) {
        let Some(clip) = self.tape_clip.clone() else {
            self.toast("Nothing lifted yet");
            return;
        };
        let st = self.status();
        let s = st.reel.spool_or_make(k);
        let (a, _) = self.tape_region();
        st.reel.snapshot(k);
        s.paste_raw(a, &clip);
        self.toast(format!("Dropped on track {}", k + 1));
    }

    pub fn tape_erase(&mut self, k: usize) {
        let st = self.status();
        let Some(s) = st.reel.spool(k) else { return };
        let (a, b) = self.tape_region();
        st.reel.snapshot(k);
        s.erase(a, b);
    }

    /// Undoes the last take or edit on the tape; again, redoes it.
    pub fn tape_undo(&mut self) {
        match self.status().reel.swap_undo() {
            Some(k) => self.toast(format!("Tape track {}: undone (again: redone)", k + 1)),
            None => self.toast("Nothing to undo on the tape"),
        }
    }

    /// Per frame: the engine hears which track is played (it records that).
    pub(crate) fn tape_frame(&mut self) {
        let played = match self.sel_track {
            TrackSel::Track(t) => self.project.tracks.get(t).map(|t| t.id),
            _ => None,
        };
        if self.tape_follow_sent != Some(played) {
            self.tape_follow_sent = Some(played);
            self.cmd(Command::TapeFollow(played));
        }
    }

    /// Writes the tracks with something on them next to the set (`dir`, the
    /// set named `name`) and notes where in the project.
    pub(crate) fn tape_save(&mut self, dir: &std::path::Path, name: &str) {
        let st = self.status();
        let sr = st.reel.rate() as u32;
        for k in 0..TAPE_TRACKS {
            let Some(s) = st.reel.spool(k) else { continue };
            if s.end() == 0 {
                self.project.tape.tracks[k].file = None;
                continue;
            }
            let path = dir.join(format!("{name} Tape {}.wav", k + 1));
            let fresh = self.project.tape.tracks[k].file.as_deref() == Some(path.as_path()) && path.exists();
            if fresh && !s.dirty.load(std::sync::atomic::Ordering::Relaxed) {
                continue;
            }
            match krabton_engine::render::write_wav(&path, sr, &s.interleaved()) {
                Ok(()) => {
                    s.dirty.store(false, std::sync::atomic::Ordering::Relaxed);
                    self.project.tape.tracks[k].file = Some(path);
                }
                Err(e) => self.toast(format!("Tape track {} not saved: {e}", k + 1)),
            }
        }
    }

    /// After a set is opened (or a new one made): the tape as the set has it.
    pub(crate) fn tape_load(&mut self) {
        let st = self.status();
        st.reel.clear();
        self.tape_clip = None;
        let files: Vec<(usize, PathBuf)> = self.project.tape.tracks.iter().enumerate().filter_map(|(k, t)| t.file.clone().map(|f| (k, f))).collect();
        for (k, path) in files {
            match SampleData::load(&path) {
                Ok(d) => load_track(&st.reel, k, &d),
                Err(e) => self.toast(format!("Tape track {}: {e}", k + 1)),
            }
        }
    }
}

/// Fills tape track `k` from a sample, at the tape's rate.
fn load_track(reel: &Reel, k: usize, d: &SampleData) {
    let rate = reel.rate() as f64;
    let ratio = d.sample_rate as f64 / rate;
    let frames = ((d.frames() as f64 / ratio) as usize).min(reel.frames());
    let mut inter = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let (l, r) = d.read(i as f64 * ratio);
        inter.push(l);
        inter.push(r);
    }
    let s = reel.spool_or_make(k);
    s.load_interleaved(&inter);
}
