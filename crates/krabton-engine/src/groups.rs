//! Groups, as in Live: tracks gathered under a group track. The group track is
//! an audio track that hears its members (their output goes into it), so its
//! effects and its fader work on all of them at once; it folds them away in the
//! views, and a solo anywhere in it is heard through it. Groups can hold groups.

use crate::project::{Project, TrackKind};
use crate::routing::{AudioInput, AudioOutput, Monitor};

/// How deep groups can go in each other (and the end of a chain gone round).
const DEPTH: usize = 16;

impl Project {
    /// The group track a track is in, by index.
    pub fn group_of(&self, ti: usize) -> Option<usize> {
        let id = self.tracks.get(ti)?.group?;
        self.tracks.iter().position(|t| t.id == id && t.is_group)
    }

    /// Whether track `ti` is in group `gi`, directly or in a group in it.
    pub fn in_group(&self, ti: usize, gi: usize) -> bool {
        let mut at = ti;
        for _ in 0..DEPTH {
            match self.group_of(at) {
                Some(g) if g == gi => return true,
                Some(g) => at = g,
                None => return false,
            }
        }
        false
    }

    /// The tracks in a group, groups in it and theirs too, in order.
    pub fn members(&self, gi: usize) -> Vec<usize> {
        (0..self.tracks.len()).filter(|&ti| self.in_group(ti, gi)).collect()
    }

    /// Folded away: a group it is in is folded.
    pub fn hidden(&self, ti: usize) -> bool {
        let mut at = ti;
        for _ in 0..DEPTH {
            match self.group_of(at) {
                Some(g) if self.tracks[g].folded => return true,
                Some(g) => at = g,
                None => return false,
            }
        }
        false
    }

    /// How many groups deep a track is.
    pub fn depth(&self, ti: usize) -> usize {
        let mut at = ti;
        let mut d = 0;
        while let Some(g) = self.group_of(at) {
            d += 1;
            at = g;
            if d >= DEPTH {
                break;
            }
        }
        d
    }

    /// Whether a track is heard while something is soloed: it is soloed, or a
    /// group it is in is, or something in it is (its sound goes through it).
    pub fn heard_in_solo(&self, ti: usize) -> bool {
        let Some(t) = self.tracks.get(ti) else { return false };
        if t.mixer.solo {
            return true;
        }
        let mut at = ti;
        for _ in 0..DEPTH {
            match self.group_of(at) {
                Some(g) if self.tracks[g].mixer.solo => return true,
                Some(g) => at = g,
                None => break,
            }
        }
        t.is_group && (0..self.tracks.len()).any(|j| self.tracks[j].mixer.solo && self.in_group(j, ti))
    }

    /// Gathers tracks in a new group, put where the first of them was, its
    /// members (and what is in them) right after it. Returns its index.
    pub fn group_tracks(&mut self, picked: &[usize]) -> Option<usize> {
        let mut picked: Vec<usize> = picked.iter().copied().filter(|&i| i < self.tracks.len()).collect();
        picked.sort_unstable();
        picked.dedup();
        let first = *picked.first()?;
        // In the group the first one was in, if any.
        let parent = self.tracks[first].group;
        let color = self.tracks[first].color;
        let mut group = self.new_group_track(parent);
        group.color = color;
        let gid = group.id;
        // The block that moves: the picked ones and whatever is in them.
        let ids: Vec<u64> = (0..self.tracks.len()).filter(|&ti| picked.iter().any(|&p| p == ti || self.in_group(ti, p))).map(|ti| self.tracks[ti].id).collect();
        for t in self.tracks.iter_mut() {
            // The ones picked go in it; what is in them stays where it is.
            if ids.contains(&t.id) && !ids.iter().any(|i| Some(*i) == t.group) {
                t.group = Some(gid);
            }
        }
        let at = first.min(self.tracks.len());
        let mut moved = Vec::new();
        let mut i = 0;
        while i < self.tracks.len() {
            if ids.contains(&self.tracks[i].id) {
                moved.push(self.tracks.remove(i));
            } else {
                i += 1;
            }
        }
        let at = at.min(self.tracks.len());
        self.tracks.insert(at, group);
        for (k, t) in moved.into_iter().enumerate() {
            self.tracks.insert(at + 1 + k, t);
        }
        self.fix_groups();
        Some(at)
    }

    fn new_group_track(&mut self, parent: Option<u64>) -> crate::project::Track {
        let n = self.tracks.iter().filter(|t| t.is_group).count() + 1;
        let mut t = self.new_track(TrackKind::Audio, if n == 1 { "Group".to_string() } else { format!("Group {n}") }, None);
        t.is_group = true;
        t.group = parent;
        t.input = AudioInput::None;
        t.monitor = Monitor::In;
        t
    }

    /// Lets a group's tracks go (into the group it was in) and takes the group away.
    pub fn ungroup(&mut self, gi: usize) {
        let Some(g) = self.tracks.get(gi).filter(|t| t.is_group) else { return };
        let (gid, parent) = (g.id, g.group);
        for t in self.tracks.iter_mut() {
            if t.group == Some(gid) {
                t.group = parent;
                t.output = AudioOutput::Master;
            }
        }
        self.tracks.remove(gi);
        self.fix_groups();
    }

    /// Takes one track out of its group (into the one around it, if any).
    pub fn leave_group(&mut self, ti: usize) {
        let parent = self.group_of(ti).and_then(|g| self.tracks[g].group);
        if let Some(t) = self.tracks.get_mut(ti) {
            if t.group.is_some() {
                t.group = parent;
                t.output = AudioOutput::Master;
            }
        }
        self.fix_groups();
    }

    /// Keeps groups whole: a group that is gone lets its tracks go, every
    /// member plays into its group, a group track hears them.
    pub fn fix_groups(&mut self) {
        let groups: Vec<u64> = self.tracks.iter().filter(|t| t.is_group).map(|t| t.id).collect();
        for t in self.tracks.iter_mut() {
            if t.group.is_some_and(|g| g == t.id || !groups.contains(&g)) {
                t.group = None;
                if let AudioOutput::Track(_) = t.output {
                    t.output = AudioOutput::Master;
                }
            }
            if t.is_group {
                t.kind = TrackKind::Audio;
                t.input = AudioInput::None;
                t.monitor = Monitor::In;
            }
        }
        // A chain that goes round on itself is cut.
        for ti in 0..self.tracks.len() {
            if self.depth(ti) >= DEPTH {
                self.tracks[ti].group = None;
            }
        }
        for ti in 0..self.tracks.len() {
            if let Some(g) = self.group_of(ti) {
                let id = self.tracks[g].id;
                self.tracks[ti].output = AudioOutput::Track(id);
            }
        }
    }

    /// The clips a group's slot in a scene launches: its tracks' in that scene.
    pub fn group_clips(&self, gi: usize, scene: usize) -> Vec<u64> {
        self.members(gi).into_iter().filter(|&ti| !self.tracks[ti].is_group && self.tracks[ti].slots.get(scene).is_some_and(|s| s.is_some())).map(|ti| self.tracks[ti].id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Project {
        let mut p = Project::new_default();
        while p.tracks.len() < 5 {
            p.add_midi_track(None);
        }
        p
    }

    #[test]
    fn grouping_puts_a_group_track_first_and_its_tracks_play_into_it() {
        let mut p = project();
        let (a, c) = (p.tracks[1].id, p.tracks[3].id);
        let gi = p.group_tracks(&[3, 1]).unwrap();
        assert_eq!(gi, 1);
        let g = &p.tracks[gi];
        assert!(g.is_group && g.kind == TrackKind::Audio && g.monitor == Monitor::In);
        assert_eq!(p.tracks[2].id, a, "its tracks right after it, in order");
        assert_eq!(p.tracks[3].id, c);
        for ti in [2, 3] {
            assert_eq!(p.tracks[ti].output, AudioOutput::Track(g.id));
            assert_eq!(p.group_of(ti), Some(gi));
        }
        assert_eq!(p.members(gi), [2, 3]);
        assert_eq!(p.group_of(4), None);
        // The engine hears them through it.
        assert_eq!(p.audio_sources(gi).len(), 2);
        assert!(p.routing_order().iter().position(|&x| x == gi) > p.routing_order().iter().position(|&x| x == 2));
    }

    #[test]
    fn folding_hides_what_is_in_it_and_groups_go_in_groups() {
        let mut p = project();
        let inner = p.group_tracks(&[2, 3]).unwrap();
        let outer = p.group_tracks(&[inner, 1]).unwrap();
        // outer, track 1, inner, its two.
        assert_eq!(p.depth(outer + 3), 2);
        assert!(p.in_group(outer + 3, outer));
        assert_eq!(p.members(outer).len(), 4);
        p.tracks[outer].folded = true;
        assert!(p.hidden(outer + 1) && p.hidden(outer + 3));
        assert!(!p.hidden(outer));
        p.tracks[outer].folded = false;
        let inner = p.tracks.iter().position(|t| t.is_group && t.group.is_some()).unwrap();
        p.tracks[inner].folded = true;
        assert!(p.hidden(inner + 1) && !p.hidden(outer + 1));
        // The inner group plays into the outer.
        assert_eq!(p.tracks[inner].output, AudioOutput::Track(p.tracks[outer].id));
    }

    #[test]
    fn a_solo_in_a_group_is_heard_through_it() {
        let mut p = project();
        let gi = p.group_tracks(&[1, 2]).unwrap();
        p.tracks[gi + 1].mixer.solo = true;
        assert!(p.heard_in_solo(gi + 1));
        assert!(p.heard_in_solo(gi), "the group it plays through");
        assert!(!p.heard_in_solo(gi + 2), "its neighbour");
        assert!(!p.heard_in_solo(0));
        p.tracks[gi + 1].mixer.solo = false;
        p.tracks[gi].mixer.solo = true;
        assert!(p.heard_in_solo(gi + 1) && p.heard_in_solo(gi + 2), "a soloed group: all of it");
    }

    #[test]
    fn ungrouping_lets_them_go_and_a_lost_group_lets_go_too() {
        let mut p = project();
        let n = p.tracks.len();
        let gi = p.group_tracks(&[0, 1]).unwrap();
        p.ungroup(gi);
        assert_eq!(p.tracks.len(), n);
        assert!(p.tracks.iter().all(|t| t.group.is_none() && t.output == AudioOutput::Master));
        let gi = p.group_tracks(&[0, 1]).unwrap();
        p.leave_group(gi + 1);
        assert_eq!(p.group_of(gi + 1), None);
        assert_eq!(p.tracks[gi + 1].output, AudioOutput::Master);
        p.tracks.remove(gi);
        p.fix_groups();
        assert!(p.tracks.iter().all(|t| t.group.is_none() && t.output == AudioOutput::Master));
    }

    #[test]
    fn a_group_slot_launches_its_tracks_clips_in_that_scene() {
        let mut p = project();
        let gi = p.group_tracks(&[1, 2]).unwrap();
        let id = p.alloc_id();
        p.tracks[gi + 2].slots[0] = Some(crate::project::Clip::new_midi(id, "c", [1, 2, 3], 4.0));
        assert_eq!(p.group_clips(gi, 0), [p.tracks[gi + 2].id]);
        assert!(p.group_clips(gi, 1).is_empty());
    }

    #[test]
    fn a_group_survives_a_save() {
        let mut p = project();
        let gi = p.group_tracks(&[1, 2]).unwrap();
        p.tracks[gi].folded = true;
        let back: Project = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert!(back.tracks[gi].is_group && back.tracks[gi].folded);
        assert_eq!(back.group_of(gi + 1), Some(gi));
    }
}
