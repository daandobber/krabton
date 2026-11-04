//! MIDI Learn: click a knob on screen, move one on a controller, and from then
//! on it turns it, as a drag would. It picks parameters the way a modulator is
//! mapped (see `ui::modulation`). The knobs are saved with the set. A scene's
//! tempo also waits here until the scene really starts.

use crate::app::App;
use krabton_engine::modulation::{MidiMap, ModTarget};

/// Stands in for a modulator while parameters are picked.
pub const SOURCE: u64 = u64::MAX - 7;

impl App {
    pub(crate) fn toggle_midi_learn(&mut self) {
        self.midi_learn = !self.midi_learn;
        self.midi_learn_target = None;
        if self.midi_learn {
            self.mod_learn = None;
            self.toast("MIDI Learn: click a knob here, then move one on your controller. Esc when done");
        }
    }

    /// A parameter was clicked while learning.
    pub(crate) fn midi_learn_pick(&mut self, target: ModTarget) {
        self.midi_learn_target = Some(target);
        let (a, b) = self.project.target_label(target);
        self.toast(format!("{a} {b}: now move a knob on your controller"));
    }

    /// A controller moved.
    pub(crate) fn midi_cc(&mut self, channel: u8, cc: u8, value: f32) {
        if self.midi_learn {
            if let Some(target) = self.midi_learn_target.take() {
                // A knob does one thing, and a parameter hears one knob.
                self.project.midi_maps.retain(|m| !(m.channel == channel && m.cc == cc) && m.target != target);
                self.project.midi_maps.push(MidiMap { channel, cc, target });
                let (a, b) = self.project.target_label(target);
                self.toast(format!("CC {cc} now turns {a} {b}"));
                return;
            }
        }
        let targets: Vec<ModTarget> = self.project.midi_maps.iter().filter(|m| m.channel == channel && m.cc == cc).map(|m| m.target).collect();
        for target in targets {
            set_target(&mut self.project, target, value);
        }
    }

    /// A scene with a tempo: the song goes to it when the scene starts.
    pub(crate) fn poll_scene_tempo(&mut self) {
        let Some((scene, bpm)) = self.scene_tempo else { return };
        let status = self.status();
        let started = !self.is_playing() || status.tracks.iter().any(|t| t.playing_slot.load(std::sync::atomic::Ordering::Relaxed) == scene as i32);
        if started {
            self.scene_tempo = None;
            self.project.bpm = bpm;
        }
    }
}

/// Puts a parameter where a knob at `value` (0..1) says, on its own scale.
fn set_target(project: &mut krabton_engine::project::Project, target: ModTarget, value: f32) {
    let ModTarget::Param { owner, param } = target else { return };
    let Some(spec) = project.owner_specs(owner).and_then(|s| s.get(param as usize)) else { return };
    let v = spec.from_norm(value.clamp(0.0, 1.0));
    if let Some(slot) = project.owner_params_mut(owner).and_then(|p| p.get_mut(param as usize)) {
        *slot = v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use krabton_engine::modulation::ModOwner;
    use krabton_engine::project::{Instrument, Project};

    #[test]
    fn a_knob_sets_a_parameter_on_its_own_scale() {
        let mut p = Project::blank();
        let t = p.add_midi_track(Some(Instrument::munich("Init")));
        let owner = ModOwner::Instrument(p.tracks[t].id);
        let cutoff = krabton_engine::dsp::munich::CUTOFF;
        let spec = &krabton_engine::dsp::munich::SPECS[cutoff];
        set_target(&mut p, ModTarget::Param { owner, param: cutoff as u16 }, 1.0);
        assert_eq!(p.owner_params_mut(owner).unwrap()[cutoff], spec.max);
        set_target(&mut p, ModTarget::Param { owner, param: cutoff as u16 }, 0.0);
        assert_eq!(p.owner_params_mut(owner).unwrap()[cutoff], spec.min);
        // Halfway on a logarithmic knob is the middle of its octaves, not of its hertz.
        set_target(&mut p, ModTarget::Param { owner, param: cutoff as u16 }, 0.5);
        let mid = p.owner_params_mut(owner).unwrap()[cutoff];
        assert!(mid < (spec.min + spec.max) * 0.25, "{mid}");
    }
}
