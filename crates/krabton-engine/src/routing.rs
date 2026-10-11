//! Track audio routing: where a track's audio comes from ("Audio From"),
//! whether you hear it ("Monitor") and where the track goes ("Audio To").
//!
//! Tracks are processed so that every track comes after the tracks it takes
//! audio from, which makes track-to-track routing sample exact. Routing
//! that would loop back is refused by [`Project::can_route`].

use crate::project::{Project, TrackKind};
use serde::{Deserialize, Serialize};

/// Where an audio track's input comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AudioInput {
    #[default]
    None,
    /// Inputs of the audio interface: `channel` (0-based) alone, or it and
    /// the next one as a stereo pair.
    Ext { channel: u8, stereo: bool },
    /// Another track, taken at a point of its chain.
    Track { id: u64, tap: Tap },
    /// The master output, one block late.
    Resampling,
}

/// Where in a track's chain another track listens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Tap {
    /// Clips or instrument, before the effects.
    PreFx,
    /// After the effects, before volume and pan.
    #[default]
    PostFx,
    /// After volume and pan.
    PostMixer,
}

impl Tap {
    pub const ALL: [Tap; 3] = [Tap::PreFx, Tap::PostFx, Tap::PostMixer];

    pub fn name(self) -> &'static str {
        match self {
            Tap::PreFx => "Pre FX",
            Tap::PostFx => "Post FX",
            Tap::PostMixer => "Post Mixer",
        }
    }
}

/// When an audio track plays its input instead of its clips.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Monitor {
    /// Always: the track is a live channel and its clips are silent.
    In,
    /// While the track is armed and no session clip plays on it.
    #[default]
    Auto,
    /// Never; the input can still be recorded.
    Off,
}

impl Monitor {
    pub const ALL: [Monitor; 3] = [Monitor::In, Monitor::Auto, Monitor::Off];

    pub fn name(self) -> &'static str {
        match self {
            Monitor::In => "In",
            Monitor::Auto => "Auto",
            Monitor::Off => "Off",
        }
    }
}

/// Which live notes a MIDI track takes ("MIDI From").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MidiInput {
    /// The computer keyboard, the grid and MIDI hardware.
    #[default]
    AllIns,
    /// Only the computer keyboard (and the grid).
    Keyboard,
    /// Only the connected MIDI device.
    Device,
    None,
    /// Another MIDI track: the notes it plays (after its MIDI effects, so a
    /// sequencer's notes too).
    Track(u64),
}

impl MidiInput {
    /// Whether notes from this source reach the track.
    pub fn accepts(self, from_device: bool) -> bool {
        match self {
            MidiInput::AllIns => true,
            MidiInput::Keyboard => !from_device,
            MidiInput::Device => from_device,
            MidiInput::None | MidiInput::Track(_) => false,
        }
    }
}

/// Where a track's output goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AudioOutput {
    #[default]
    Master,
    /// Into the input of an audio track.
    Track(u64),
}

impl Project {
    /// Indices of the tracks whose audio track `ti` receives directly, with
    /// the tap: its own input and every track sent to it.
    pub fn audio_sources(&self, ti: usize) -> Vec<(usize, Tap)> {
        let Some(t) = self.tracks.get(ti) else { return Vec::new() };
        if t.kind != TrackKind::Audio {
            return Vec::new();
        }
        let mut out = Vec::new();
        if let AudioInput::Track { id, tap } = t.input {
            if let Some(si) = self.track_index(id).filter(|&si| si != ti) {
                out.push((si, tap));
            }
        }
        for (si, s) in self.tracks.iter().enumerate() {
            if si != ti && s.output == AudioOutput::Track(t.id) {
                out.push((si, Tap::PostMixer));
            }
        }
        out
    }

    /// Whether the audio of track `from` reaches track `to`, directly or
    /// through other tracks.
    pub fn feeds(&self, from: usize, to: usize) -> bool {
        let mut seen = vec![false; self.tracks.len()];
        let mut stack = vec![to];
        while let Some(t) = stack.pop() {
            if t == from {
                return true;
            }
            if std::mem::replace(&mut seen[t], true) {
                continue;
            }
            stack.extend(self.audio_sources(t).into_iter().map(|(s, _)| s));
        }
        false
    }

    /// Whether track `from` can be routed into audio track `to` without
    /// making a loop.
    pub fn can_route(&self, from: usize, to: usize) -> bool {
        from != to && from < self.tracks.len() && self.tracks.get(to).is_some_and(|t| t.kind == TrackKind::Audio) && !self.feeds(to, from)
    }

    /// The track whose notes MIDI track `ti` takes ("MIDI From" a track).
    pub fn midi_source(&self, ti: usize) -> Option<usize> {
        let t = self.tracks.get(ti)?;
        let MidiInput::Track(id) = t.midi_in else { return None };
        if t.kind != TrackKind::Midi {
            return None;
        }
        self.track_index(id).filter(|&si| si != ti && self.tracks[si].kind == TrackKind::Midi)
    }

    /// Whether MIDI track `to` can take the notes of `from` without making
    /// a loop.
    pub fn can_route_midi(&self, from: usize, to: usize) -> bool {
        let midi = |i: usize| self.tracks.get(i).is_some_and(|t| t.kind == TrackKind::Midi);
        if from == to || !midi(from) || !midi(to) {
            return false;
        }
        // Follow where `from` takes its notes from: it must not come back to `to`.
        let mut at = from;
        for _ in 0..self.tracks.len() {
            match self.midi_source(at) {
                Some(s) if s == to => return false,
                Some(s) => at = s,
                None => return true,
            }
        }
        false
    }

    /// Track indices in processing order: every track after the tracks it
    /// takes audio (or notes) from. Tracks caught in a loop keep their place at the end
    /// (they hear the previous block of each other).
    pub fn routing_order(&self) -> Vec<usize> {
        let n = self.tracks.len();
        let sources: Vec<Vec<usize>> = (0..n).map(|ti| self.audio_sources(ti).into_iter().map(|(s, _)| s).chain(self.midi_source(ti)).collect()).collect();
        let mut done = vec![false; n];
        let mut order = Vec::with_capacity(n);
        loop {
            let before = order.len();
            for ti in 0..n {
                if !done[ti] && sources[ti].iter().all(|&s| done[s]) {
                    done[ti] = true;
                    order.push(ti);
                }
            }
            if order.len() == before {
                break;
            }
        }
        order.extend((0..n).filter(|&ti| !done[ti]));
        order
    }

    /// The MIDI tracks that hear a live note, like Live: the track's MIDI
    /// From and channel must match the source; Monitor In always hears it,
    /// Auto when armed (or, with nothing armed, when selected), Off never.
    /// `channel` is `None` for the computer keyboard and the grid.
    pub fn live_note_targets(&self, selected: Option<usize>, channel: Option<u8>) -> Vec<usize> {
        let from_device = channel.is_some();
        let hears = |t: &crate::project::Track| {
            t.kind == TrackKind::Midi && t.midi_in.accepts(from_device) && (t.midi_channel.is_none() || !from_device || t.midi_channel == channel)
        };
        let any_armed = self.tracks.iter().any(|t| t.arm && t.monitor == Monitor::Auto && hears(t));
        self.tracks
            .iter()
            .enumerate()
            .filter(|(i, t)| {
                hears(t)
                    && match t.monitor {
                        Monitor::In => true,
                        Monitor::Auto => t.arm || (!any_armed && selected == Some(*i)),
                        Monitor::Off => false,
                    }
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Drops routing to tracks that no longer exist (or to themselves).
    pub fn fix_routing(&mut self) {
        let ids: Vec<(u64, TrackKind)> = self.tracks.iter().map(|t| (t.id, t.kind)).collect();
        for t in &mut self.tracks {
            let own = t.id;
            if let AudioInput::Track { id, .. } = t.input {
                if id == own || !ids.iter().any(|(i, _)| *i == id) {
                    t.input = AudioInput::None;
                }
            }
            if let AudioOutput::Track(id) = t.output {
                if id == own || !ids.iter().any(|(i, k)| *i == id && *k == TrackKind::Audio) {
                    t.output = AudioOutput::Master;
                }
            }
            if t.kind == TrackKind::Midi {
                t.input = AudioInput::None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_with_audio(n: usize) -> Project {
        let mut p = Project::blank();
        for _ in 0..n {
            p.add_audio_track();
        }
        p
    }

    #[test]
    fn order_puts_sources_first_and_loops_are_refused() {
        let mut p = project_with_audio(4);
        let ids: Vec<u64> = p.tracks.iter().map(|t| t.id).collect();
        // 0 listens to 2, 2 listens to 3; 1 sends its output to 0.
        p.tracks[0].input = AudioInput::Track { id: ids[2], tap: Tap::PostFx };
        p.tracks[2].input = AudioInput::Track { id: ids[3], tap: Tap::PreFx };
        p.tracks[1].output = AudioOutput::Track(ids[0]);
        let order = p.routing_order();
        let pos = |t: usize| order.iter().position(|&x| x == t).unwrap();
        assert!(pos(3) < pos(2) && pos(2) < pos(0) && pos(1) < pos(0), "{order:?}");
        assert!(p.feeds(3, 0) && p.feeds(1, 0) && !p.feeds(0, 3));
        // 3 taking from 0 would close a loop; 3 from 1 is fine.
        assert!(!p.can_route(0, 3));
        assert!(p.can_route(1, 3));
        assert!(!p.can_route(2, 2));
        // A loop written by hand still yields every track once.
        p.tracks[3].input = AudioInput::Track { id: ids[0], tap: Tap::PostFx };
        let mut order = p.routing_order();
        order.sort();
        assert_eq!(order, vec![0, 1, 2, 3]);
    }

    #[test]
    fn live_notes_follow_midi_from_monitor_and_arm() {
        let mut p = Project::blank();
        for _ in 0..3 {
            p.add_midi_track(None);
        }
        // Nothing armed: the selected track plays, from any source.
        assert_eq!(p.live_note_targets(Some(1), None), vec![1]);
        assert_eq!(p.live_note_targets(Some(1), Some(0)), vec![1]);
        // Armed tracks take over from the selection.
        p.tracks[2].arm = true;
        assert_eq!(p.live_note_targets(Some(1), None), vec![2]);
        // Monitor In always hears, Off never.
        p.tracks[0].monitor = Monitor::In;
        p.tracks[2].monitor = Monitor::Off;
        assert_eq!(p.live_note_targets(Some(1), None), vec![0, 1]);
        // MIDI From: keyboard only ignores the device, a channel filters it.
        p.tracks[0].midi_in = MidiInput::Keyboard;
        assert_eq!(p.live_note_targets(None, Some(3)), Vec::<usize>::new());
        p.tracks[0].midi_in = MidiInput::Device;
        p.tracks[0].midi_channel = Some(3);
        assert_eq!(p.live_note_targets(None, Some(3)), vec![0]);
        assert_eq!(p.live_note_targets(None, Some(4)), Vec::<usize>::new());
        assert_eq!(p.live_note_targets(None, None), Vec::<usize>::new(), "the keyboard is not the device");
        p.tracks[0].midi_in = MidiInput::None;
        assert!(!p.live_note_targets(Some(0), None).contains(&0));
    }

    #[test]
    fn routing_to_deleted_tracks_is_dropped() {
        let mut p = project_with_audio(2);
        let gone = p.tracks[1].id;
        p.tracks[0].input = AudioInput::Track { id: gone, tap: Tap::PostFx };
        p.tracks[0].output = AudioOutput::Track(gone);
        p.tracks.remove(1);
        p.fix_routing();
        assert_eq!(p.tracks[0].input, AudioInput::None);
        assert_eq!(p.tracks[0].output, AudioOutput::Master);
    }
}
