//! The band: every device that plays the chord progression (Progression,
//! Guitar Chords and the players) with Band on plays the same chords. Change
//! them on one and the others follow; one that joins takes the band's.
//! With parts (Verse, Chorus...; see `krabton_engine::band`) they show and
//! edit the part playing, and move to the next part with the song.

use crate::app::App;
use krabton_engine::band::BandStatus;
use krabton_engine::sequencers::players::{band_param, band_part, set_band_part};
use std::collections::HashMap;

#[derive(Default)]
pub struct Band {
    /// What the band plays (see `band_part`).
    part: Option<Vec<u8>>,
    /// Each member's part as it was last frame, by device id.
    seen: HashMap<u64, Vec<u8>>,
    /// The song's part the members showed last frame.
    shown: Option<usize>,
    /// The Band window is open, and the part it edits.
    pub(crate) window: bool,
    pub(crate) editing: usize,
}

impl App {
    /// Where the band is now, as the engine last said.
    pub(crate) fn band_status(&self) -> BandStatus {
        BandStatus::unpack(self.status().band.load(std::sync::atomic::Ordering::Relaxed))
    }

    /// With parts, what a member's editor shows: the part playing (see
    /// `band_frame`), for a chord player with Band on.
    pub(crate) fn band_shown_part(&self, kind: krabton_engine::midi_fx::MidiFxKind, params: &[f32]) -> Option<Vec<u8>> {
        let on = band_param(kind).and_then(|b| params.get(b)).is_some_and(|v| *v >= 0.5);
        let parts = &self.project.band.parts;
        parts.get(self.band.shown.unwrap_or(0).min(parts.len().saturating_sub(1))).filter(|_| on).map(|p| p.part())
    }

    /// Once a frame, before the project goes to the engine.
    pub(crate) fn band_frame(&mut self) {
        let mut members = Vec::new();
        for (t, tr) in self.project.tracks.iter().enumerate() {
            for (d, dev) in tr.midi_devices.iter().enumerate() {
                if band_param(dev.kind).and_then(|b| dev.params.get(b)).is_some_and(|v| *v >= 0.5) {
                    members.push((t, d));
                }
            }
        }
        let parts = self.project.band.parts.len();
        if members.is_empty() {
            self.band.part = None;
            self.band.seen.clear();
            self.band.shown = None;
            return;
        }
        // One that changed since the last frame leads; one that joined follows.
        let mut lead = None;
        let mut first = None;
        for &(t, d) in &members {
            let dev = &self.project.tracks[t].midi_devices[d];
            let part = band_part(&dev.params, &dev.data);
            if self.band.seen.get(&dev.id).is_some_and(|s| *s != part) {
                lead = Some(part);
                break;
            }
            first.get_or_insert(part);
        }
        if parts > 0 {
            // With parts the engine plays them; an edit on a member (its
            // editor shows the part playing) goes into that part.
            if let (Some(lead), Some(p)) = (&lead, self.band.shown.and_then(|s| self.project.band.parts.get_mut(s))) {
                p.set_part(lead);
            }
            let st = self.band_status();
            let now = if st.parts { (st.part as usize).min(parts - 1) } else { self.band.shown.unwrap_or(0).min(parts - 1) };
            self.band.shown = Some(now);
            self.band.part = Some(self.project.band.parts[now].part());
            let tracks = &self.project.tracks;
            self.band.seen = members.iter().map(|&(t, d)| &tracks[t].midi_devices[d]).map(|dev| (dev.id, band_part(&dev.params, &dev.data))).collect();
            return;
        }
        self.band.shown = None;
        let Some(part) = lead.or_else(|| self.band.part.clone()).or(first) else { return };
        let mut seen = HashMap::new();
        for &(t, d) in &members {
            let dev = &mut self.project.tracks[t].midi_devices[d];
            if band_part(&dev.params, &dev.data) != part {
                set_band_part(&mut dev.params, &mut dev.data, &part);
            }
            seen.insert(dev.id, part.clone());
        }
        self.band.seen = seen;
        self.band.part = Some(part);
    }
}
