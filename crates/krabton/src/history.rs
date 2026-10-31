//! Names for the steps in the undo history (as in Reaper and Bitwig): what
//! changed between two versions of the project.

use krabton_engine::{ClipContent, Project, Track};

/// A short description of what changed from `a` to `b`.
pub fn describe(a: &Project, b: &Project) -> String {
    if a.tracks.len() < b.tracks.len() {
        return format!("Add track {}", b.tracks.last().map(|t| t.name.as_str()).unwrap_or(""));
    }
    if a.tracks.len() > b.tracks.len() {
        return "Delete track".into();
    }
    if a.bpm != b.bpm {
        return format!("Tempo {:.1}", b.bpm);
    }
    if a.sections != b.sections {
        return describe_sections(a, b);
    }
    if a.scenes != b.scenes {
        return "Scenes".into();
    }
    // Several tracks' arrangements at once: a section move, Scenes to Arrangement...
    if a.tracks.iter().zip(&b.tracks).filter(|(x, y)| x.arrangement != y.arrangement).count() > 1 {
        return "Arrangement".into();
    }
    for (ta, tb) in a.tracks.iter().zip(&b.tracks) {
        if ta != tb {
            return describe_track(ta, tb);
        }
    }
    if a.returns != b.returns {
        return "Return tracks".into();
    }
    if a.master != b.master {
        return "Master".into();
    }
    if a.modulators != b.modulators || a.mod_routes != b.mod_routes {
        return "Modulation".into();
    }
    if (a.loop_enabled, a.loop_start, a.loop_end) != (b.loop_enabled, b.loop_start, b.loop_end) {
        return "Loop".into();
    }
    if a.scale != b.scale || a.scale_mode != b.scale_mode {
        return "Key and scale".into();
    }
    if a.launch_quantize != b.launch_quantize || a.metronome != b.metronome || a.beats_per_bar != b.beats_per_bar {
        return "Transport settings".into();
    }
    if a.video != b.video {
        return "Video".into();
    }
    "Edit".into()
}

fn describe_sections(a: &Project, b: &Project) -> String {
    let name = |p: &Project, id: u64| p.sections.iter().find(|s| s.id == id).map(|s| s.name.clone()).unwrap_or_default();
    if b.sections.len() > a.sections.len() {
        let new = b.sections.iter().find(|s| !a.sections.iter().any(|x| x.id == s.id));
        return match new {
            Some(s) if b.tracks.iter().zip(&a.tracks).any(|(x, y)| x.arrangement != y.arrangement) => format!("Copy section {}", s.name),
            Some(s) => format!("Add section {}", s.name),
            None => "Add section".into(),
        };
    }
    if b.sections.len() < a.sections.len() {
        let gone = a.sections.iter().find(|s| !b.sections.iter().any(|x| x.id == s.id)).map(|s| s.name.clone()).unwrap_or_default();
        return format!("Delete section {gone}");
    }
    for (x, y) in a.sections.iter().filter_map(|x| b.sections.iter().find(|y| y.id == x.id).map(|y| (x, y))) {
        if x.name != y.name {
            return format!("Rename section to {}", y.name);
        }
        if x.start != y.start && x.length == y.length && b.tracks.iter().zip(&a.tracks).any(|(p, q)| p.arrangement != q.arrangement) {
            // Which one was dragged cannot be told apart from a swap; the new order can.
            let order: Vec<String> = b.sections.iter().take(5).map(|s| name(b, s.id)).collect();
            return format!("Reorder sections: {}", order.join(", "));
        }
        if x.length != y.length {
            return format!("Resize section {}", y.name);
        }
        if x.color != y.color {
            return format!("Colour section {}", y.name);
        }
    }
    "Arranger sections".into()
}

fn describe_track(a: &Track, b: &Track) -> String {
    let t = &b.name;
    if a.name != b.name {
        return format!("Rename track to {t}");
    }
    if a.instrument != b.instrument {
        return match (&a.instrument, &b.instrument) {
            (Some(x), Some(y)) if x.kind_name() == y.kind_name() => format!("{t}: {}", y.kind_name()),
            (_, Some(y)) => format!("{t}: load {}", y.kind_name()),
            _ => format!("{t}: remove instrument"),
        };
    }
    if a.devices != b.devices {
        if a.devices.len() < b.devices.len() {
            return format!("{t}: add {}", b.devices.last().map(|d| d.name()).unwrap_or("effect"));
        }
        if a.devices.len() > b.devices.len() {
            return format!("{t}: remove effect");
        }
        if let Some(d) = a.devices.iter().zip(&b.devices).find(|(x, y)| x != y).map(|(_, y)| y) {
            return format!("{t}: {}", d.name());
        }
    }
    if a.midi_devices != b.midi_devices {
        return format!("{t}: MIDI effects");
    }
    if a.slots != b.slots {
        for (x, y) in a.slots.iter().zip(&b.slots) {
            match (x, y) {
                (None, Some(c)) => return format!("{t}: new clip {}", c.name),
                (Some(_), None) => return format!("{t}: delete clip"),
                (Some(cx), Some(cy)) if cx != cy => {
                    let notes = |c: &krabton_engine::Clip| match &c.content {
                        ClipContent::Midi { notes } => Some(notes.clone()),
                        _ => None,
                    };
                    return if notes(cx) != notes(cy) {
                        format!("Edit notes in {}", cy.name)
                    } else if cx.envelopes != cy.envelopes {
                        format!("Envelopes in {}", cy.name)
                    } else if cx.sound.is_some() != cy.sound.is_some() {
                        if cy.sound.is_some() { format!("Keep the sound in {}", cy.name) } else { format!("Forget the sound of {}", cy.name) }
                    } else {
                        format!("{t}: clip {}", cy.name)
                    };
                }
                _ => {}
            }
        }
        return format!("{t}: clips");
    }
    if a.arrangement != b.arrangement {
        return format!("{t}: arrangement");
    }
    if a.mixer != b.mixer {
        return format!("{t}: mixer");
    }
    if a.arm != b.arm {
        return format!("{t}: arm");
    }
    format!("{t}: settings")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_are_named_after_what_changed() {
        let a = krabton_engine::demo::demo_project();
        let mut b = a.clone();
        b.bpm = 128.0;
        assert_eq!(describe(&a, &b), "Tempo 128.0");
        let mut b = a.clone();
        b.tracks[1].mixer.volume_db -= 3.0;
        assert_eq!(describe(&a, &b), format!("{}: mixer", a.tracks[1].name));
        let mut b = a.clone();
        if let Some(Some(c)) = b.tracks[1].slots.iter_mut().find(|s| s.is_some()) {
            if let Some(v) = c.notes_mut() {
                v.pop();
            }
        }
        assert!(describe(&a, &b).starts_with("Edit notes in"), "{}", describe(&a, &b));
    }

    #[test]
    fn section_steps_say_what_happened() {
        let mut a = krabton_engine::demo::demo_project();
        a.scenes_to_arrangement();
        let mut b = a.clone();
        let intro = b.add_section(0.0, 16.0);
        assert_eq!(describe(&a, &b), "Add section Intro");
        let mut c = b.clone();
        let verse = c.add_section(16.0, 16.0);
        let mut d = c.clone();
        d.move_section(verse, 0.0);
        assert_eq!(describe(&c, &d), "Reorder sections: Verse, Intro");
        let mut e = d.clone();
        e.copy_section(intro, None);
        assert_eq!(describe(&d, &e), "Copy section Intro");
        let mut f = e.clone();
        f.delete_section(intro, true);
        assert_eq!(describe(&e, &f), "Delete section Intro");
    }

}
