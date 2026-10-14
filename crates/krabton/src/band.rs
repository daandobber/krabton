//! The band: every device that plays the chord progression (Progression,
//! Guitar Chords and the players) with Band on plays the same chords. Change
//! them on one and the others follow; one that joins takes the band's.

use crate::app::App;
use krabton_engine::sequencers::players::{band_param, band_part, set_band_part};
use std::collections::HashMap;

#[derive(Default)]
pub struct Band {
    /// What the band plays (see `band_part`).
    part: Option<Vec<u8>>,
    /// Each member's part as it was last frame, by device id.
    seen: HashMap<u64, Vec<u8>>,
}

impl App {
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
        if members.is_empty() {
            self.band = Band::default();
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
