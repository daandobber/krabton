//! Floor plans for the Map: where everything of an orchestra or a band
//! usually stands, drawn on the floor to go by. Only a drawing: nothing has to
//! stand there, and *Place tracks* puts the tracks there by their names.

/// A part of a floor plan, in the map's own terms: across -1 to 1, depth 0 at
/// the back wall (the listener sits at 0.6 unless moved).
pub(crate) struct Zone {
    pub name: &'static str,
    pub across: f32,
    pub depth: f32,
    pub half_w: f32,
    pub half_h: f32,
    /// Which family: parts of it share a tint.
    pub family: usize,
    /// What a track's name or instrument has when it stands here (lower case).
    pub words: &'static [&'static str],
}

pub(crate) struct Plan {
    pub name: &'static str,
    pub zones: &'static [Zone],
}

const fn zone(name: &'static str, across: f32, depth: f32, half_w: f32, half_h: f32, family: usize, words: &'static [&'static str]) -> Zone {
    Zone { name, across, depth, half_w, half_h, family, words }
}

// The first zone that has a word wins, so the narrow ones come first
// ("bassoon" before "bass", "second violin" before "violin").
pub(crate) const PLANS: [Plan; 6] = [
    Plan {
        name: "Orchestra",
        zones: &[
            zone("Percussion", -0.36, 0.06, 0.2, 0.04, 3, &["perc", "drum", "glock", "xylo", "marimba", "bell", "cymbal", "triangle"]),
            zone("Timpani", 0.06, 0.06, 0.18, 0.04, 3, &["timpani"]),
            zone("Piano", -0.78, 0.06, 0.2, 0.04, 4, &["piano", "organ", "celesta", "keys"]),
            zone("Horns", -0.56, 0.16, 0.18, 0.04, 2, &["horn"]),
            zone("Trumpets", -0.16, 0.16, 0.2, 0.04, 2, &["trumpet"]),
            zone("Trombones", 0.28, 0.16, 0.22, 0.04, 2, &["trombone"]),
            zone("Tuba", 0.66, 0.16, 0.14, 0.04, 2, &["tuba"]),
            zone("Clarinets", -0.21, 0.26, 0.2, 0.04, 1, &["clarinet", "sax"]),
            zone("Bassoons", 0.21, 0.26, 0.2, 0.04, 1, &["bassoon"]),
            zone("Flutes", -0.21, 0.35, 0.2, 0.04, 1, &["flute", "piccolo", "fluit"]),
            zone("Oboes", 0.21, 0.35, 0.2, 0.04, 1, &["oboe"]),
            zone("Harp", -0.76, 0.29, 0.16, 0.08, 4, &["harp", "lyre"]),
            zone("Basses", 0.76, 0.29, 0.16, 0.08, 0, &["bass"]),
            zone("2nd Violins", -0.2, 0.46, 0.2, 0.055, 0, &["second vio", "2nd vio", "violin 2", "violins 2"]),
            zone("Violas", 0.2, 0.46, 0.2, 0.055, 0, &["viola", "altviool"]),
            zone("1st Violins", -0.68, 0.46, 0.25, 0.055, 0, &["violin", "viool", "strings", "erhu"]),
            zone("Cellos", 0.68, 0.46, 0.25, 0.055, 0, &["cello"]),
            zone("Conductor", 0.0, 0.54, 0.12, 0.03, 4, &[]),
        ],
    },
    Plan {
        name: "Band",
        zones: &[
            zone("Drums", 0.0, 0.1, 0.28, 0.08, 3, &["drum", "kick", "snare", "hat", "beat", "perc"]),
            zone("Bass", -0.52, 0.2, 0.2, 0.07, 0, &["bass", "808", "sub"]),
            zone("Keys", 0.52, 0.2, 0.2, 0.07, 4, &["keys", "piano", "organ", "synth", "chord", "pad", "arp", "rhodes"]),
            zone("Guitar 2", 0.6, 0.38, 0.22, 0.07, 2, &["guitar 2", "rhythm guitar", "second guitar", "acoustic", "gitaar 2"]),
            zone("Guitar", -0.6, 0.38, 0.22, 0.07, 2, &["guitar", "gitaar"]),
            zone("Backing Vocals", 0.0, 0.29, 0.28, 0.05, 1, &["backing", "backup", "choir", "harmony", "koor"]),
            zone("Vocals", 0.0, 0.43, 0.2, 0.06, 1, &["vocal", "vox", "voice", "sing", "rap", "lead", "zang"]),
        ],
    },
    Plan {
        name: "Jazz Combo",
        zones: &[
            zone("Drums", 0.42, 0.1, 0.24, 0.08, 3, &["drum", "kick", "snare", "hat", "beat", "perc"]),
            zone("Double Bass", -0.06, 0.1, 0.22, 0.07, 0, &["bass", "808", "sub"]),
            zone("Piano", -0.62, 0.18, 0.26, 0.1, 4, &["piano", "keys", "organ", "rhodes", "chord"]),
            zone("Guitar", 0.66, 0.28, 0.2, 0.06, 2, &["guitar", "gitaar"]),
            zone("Trumpet", -0.3, 0.38, 0.16, 0.05, 1, &["trumpet"]),
            zone("Sax", 0.0, 0.38, 0.12, 0.05, 1, &["sax", "clarinet", "flute", "melodica"]),
            zone("Trombone", 0.3, 0.38, 0.16, 0.05, 1, &["trombone", "horn"]),
            zone("Vocals", 0.0, 0.5, 0.14, 0.05, 2, &["vocal", "vox", "voice", "sing", "zang"]),
        ],
    },
    Plan {
        name: "Choir",
        zones: &[
            zone("Tenors", -0.28, 0.17, 0.26, 0.08, 1, &["tenor"]),
            zone("Basses", 0.28, 0.17, 0.26, 0.08, 0, &["bass", "bariton"]),
            zone("Sopranos", -0.46, 0.35, 0.38, 0.09, 2, &["soprano", "sopraan"]),
            zone("Altos", 0.46, 0.35, 0.38, 0.09, 3, &["alto", "alt "]),
            zone("Conductor", 0.0, 0.5, 0.12, 0.04, 4, &[]),
        ],
    },
    Plan {
        name: "String Quartet",
        zones: &[
            zone("2nd Violin", -0.18, 0.2, 0.18, 0.07, 1, &["second vio", "2nd vio", "violin 2"]),
            zone("Viola", 0.18, 0.2, 0.16, 0.07, 2, &["viola", "altviool"]),
            zone("1st Violin", -0.4, 0.36, 0.2, 0.07, 0, &["violin", "viool", "strings"]),
            zone("Cello", 0.4, 0.36, 0.18, 0.07, 3, &["cello", "bass"]),
        ],
    },
    Plan {
        name: "Club Mix",
        zones: &[
            zone("Hats", 0.54, 0.18, 0.18, 0.06, 3, &["hat", "cymbal", "shaker", "ride"]),
            zone("Percussion", -0.54, 0.18, 0.2, 0.06, 3, &["perc", "conga", "bongo", "tom"]),
            zone("Snare & Clap", 0.0, 0.29, 0.2, 0.05, 3, &["snare", "clap"]),
            zone("Kick & Sub", 0.0, 0.14, 0.2, 0.07, 0, &["kick", "sub", "808", "bass", "drum", "beat"]),
            zone("FX & Risers", 0.76, 0.07, 0.22, 0.05, 4, &["fx", "riser", "noise", "sweep"]),
            zone("Chords & Pads", -0.62, 0.37, 0.26, 0.06, 2, &["chord", "pad", "keys", "piano", "strings", "organ"]),
            zone("Arps", 0.62, 0.37, 0.2, 0.06, 2, &["arp", "pluck", "seq"]),
            zone("Lead", -0.18, 0.47, 0.15, 0.05, 1, &["lead", "synth", "melod"]),
            zone("Vocals", 0.18, 0.47, 0.15, 0.05, 1, &["vocal", "vox", "voice", "sing", "rap", "zang"]),
        ],
    },
];

/// The plan a map is set to (0: none).
pub(crate) fn plan(choice: u8) -> Option<&'static Plan> {
    PLANS.get((choice as usize).checked_sub(1)?)
}

impl Plan {
    /// Which zone a track belongs in, by what it is called (lower case).
    pub(crate) fn zone_for(&self, name: &str) -> Option<usize> {
        self.zones.iter().position(|z| z.words.iter().any(|w| name.contains(w)))
    }
}

impl Zone {
    /// Where the nth track in it stands (across, depth): out from the middle.
    pub(crate) fn spot(&self, nth: usize) -> (f32, f32) {
        let side = (nth + 1) / 2;
        let way = if nth % 2 == 1 { -1.0 } else { 1.0 };
        let across = self.across + way * side as f32 * self.half_w * 0.8;
        // Every other pair a little forward, so their name tags do not cover each other.
        let depth = self.depth + if side % 2 == 1 { self.half_h * 0.6 } else { 0.0 };
        (across.clamp(self.across - self.half_w, self.across + self.half_w), depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_zone_lies_on_the_floor_before_the_listener() {
        for p in &PLANS {
            for z in p.zones {
                assert!(z.across.abs() + z.half_w <= 1.0, "{} {}", p.name, z.name);
                assert!(z.depth - z.half_h >= 0.0 && z.depth + z.half_h <= 0.6, "{} {}", p.name, z.name);
                assert!(z.family < 5, "{}", z.name);
                assert!(z.words.iter().all(|w| *w == w.to_lowercase()), "{}", z.name);
            }
        }
    }

    #[test]
    fn no_two_zones_of_a_plan_lie_over_each_other() {
        // Metres in the room a new set has: a unit across, a unit of depth.
        let (mx, my) = (5.3, 15.4);
        for p in &PLANS {
            for (i, a) in p.zones.iter().enumerate() {
                for b in &p.zones[i + 1..] {
                    for k in 0..64 {
                        let t = k as f32 / 64.0 * std::f32::consts::TAU;
                        let x = a.across + a.half_w * t.cos() - b.across;
                        let y = a.depth + a.half_h * t.sin() - b.depth;
                        let inside = (x * mx / (b.half_w * mx)).powi(2) + (y * my / (b.half_h * my)).powi(2);
                        assert!(inside >= 0.98, "{}: {} on {}", p.name, a.name, b.name);
                    }
                }
            }
        }
    }

    #[test]
    fn tracks_find_their_place_by_their_name() {
        let at = |plan: usize, name: &str| PLANS[plan].zone_for(name).map(|z| PLANS[plan].zones[z].name);
        // The orchestra's own instruments.
        assert_eq!(at(0, "violins"), Some("1st Violins"));
        assert_eq!(at(0, "second violins"), Some("2nd Violins"));
        assert_eq!(at(0, "violas"), Some("Violas"));
        assert_eq!(at(0, "bassoon"), Some("Bassoons"));
        assert_eq!(at(0, "double bass"), Some("Basses"));
        assert_eq!(at(0, "french horn"), Some("Horns"));
        assert_eq!(at(0, "tubular bells"), Some("Percussion"));
        assert_eq!(at(0, "fat lead"), None);
        // A band out of what a new set has.
        assert_eq!(at(1, "drum rack 808"), Some("Drums"));
        assert_eq!(at(1, "bass fat bass"), Some("Bass"));
        assert_eq!(at(1, "fm keys"), Some("Keys"));
        assert_eq!(at(1, "electric guitar"), Some("Guitar"));
        assert_eq!(at(1, "guitar 2"), Some("Guitar 2"));
        assert_eq!(at(1, "backing vocals"), Some("Backing Vocals"));
        assert_eq!(at(5, "kick"), Some("Kick & Sub"));
        assert_eq!(at(5, "hi hat"), Some("Hats"));
    }

    #[test]
    fn tracks_in_one_zone_stand_side_by_side_inside_it() {
        let z = &PLANS[0].zones[0];
        let spots: Vec<f32> = (0..3).map(|n| z.spot(n).0).collect();
        assert_eq!(spots[0], z.across);
        assert!(spots[1] < spots[0] && spots[2] > spots[0], "{spots:?}");
        for n in 0..40 {
            assert!((z.spot(n).0 - z.across).abs() <= z.half_w + 1e-5);
        }
        assert!(plan(0).is_none());
        assert_eq!(plan(1).unwrap().name, "Orchestra");
        assert!(plan(7).is_none(), "an unknown one shows nothing");
    }
}
