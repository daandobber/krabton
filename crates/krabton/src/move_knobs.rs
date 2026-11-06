//! What the Move's eight knobs can reach, as Push does it: a first set with
//! the knobs that matter most, then every parameter, set by set, by its group.

use krabton_engine::params::{ParamKind, ParamSpec};
use krabton_engine::Instrument;

/// Eight knobs at a time.
pub const SIZE: usize = 8;

/// A set of knobs: its name and the parameters (by index).
#[derive(Clone, Debug, PartialEq)]
pub struct Bank {
    pub name: String,
    pub knobs: Vec<usize>,
}

/// What to look for in a device without a list of its own, most telling first.
const TELLING: [&str; 20] = ["cutoff", "freq", "reso", "timbre", "bright", "morph", "tone", "color", "drive", "mix", "amount", "depth", "time", "rate", "feedback", "decay", "attack", "release", "level", "volume"];

/// Declares each instrument's first set. A name is the first knob called that;
/// "Group::Name" where an instrument has it twice.
macro_rules! main_knobs {
    ($($variant:ident, $specs:path, [$($name:expr),* $(,)?];)*) => {
        /// The knobs of an instrument's first set, by name.
        pub fn main_names(inst: &Instrument) -> &'static [&'static str] {
            match inst {
                $(Instrument::$variant { .. } => &[$($name),*],)*
                _ => &[],
            }
        }

        #[cfg(test)]
        fn table() -> Vec<(&'static str, &'static [ParamSpec], &'static [&'static str])> {
            vec![$((stringify!($variant), $specs, &[$($name),*] as &'static [&'static str]),)*]
        }
    };
}

main_knobs! {
    Synth, krabton_engine::dsp::synth::SPECS, ["Cutoff", "Resonance", "Env Amt", "Drive", "Amp Env::Attack", "Amp Env::Decay", "Amp Env::Sustain", "Amp Env::Release"];
    Analog, krabton_engine::dsp::analog::SPECS, ["Cutoff", "Resonance", "Env Amount", "Detune 2", "Attack", "Decay", "Sustain", "Release"];
    Waves, krabton_engine::dsp::waves::SPECS, ["Position 1", "Position 2", "Cutoff", "Resonance", "Env Amount", "Attack", "Decay", "Release"];
    Silicon, krabton_engine::dsp::silicon::SPECS, ["Cutoff", "Resonance", "Env Amount", "Env Decay", "Attack", "Decay", "Sustain", "Release"];
    Crimson, krabton_engine::dsp::crimson::SPECS, ["Cutoff", "Resonance", "Env Amount", "Filter Env::Decay", "Amp Env::Attack", "Amp Env::Decay", "Amp Env::Sustain", "Amp Env::Release"];
    Hyper, krabton_engine::hyper::SPECS, ["Cutoff", "Resonance", "Env Amount", "Osc Mix", "Attack", "Decay", "Sustain", "Release"];
    Hybrid, krabton_engine::dsp::hybrid::SPECS, ["Cutoff 1", "Reso 1", "Env 1", "Shape 1", "Shape 2", "Attack", "Decay", "Release"];
    Quartet, krabton_engine::dsp::quartet::SPECS, ["Cutoff A", "Reso A", "Env A", "Cutoff B", "Reso B", "Env B", "Delay", "Reverb"];
    Bassline, krabton_engine::dsp::bassline::SPECS, ["Cutoff", "Resonance", "Env Mod", "Decay", "Accent", "Slide Time", "Drive", "Volume"];
    Mono101, krabton_engine::dsp::mono101::SPECS, ["Freq", "Res", "Env", "VCF::Mod", "Attack", "Decay", "Sustain", "Release"];
    Munich, krabton_engine::dsp::munich::SPECS, ["Cutoff", "Emphasis", "Contour", "Filter Contour::Decay", "Loudness::Decay", "Drive", "Glide", "Mix"];
    Macro, krabton_engine::dsp::macro_osc::SPECS, ["Harmonics", "Timbre", "Morph", "Out / Aux", "Decay", "Timbre Env", "Morph Env", "LPG Colour"];
    Braids, krabton_engine::dsp::braids_synth::SPECS, ["Timbre", "Color", "Env Attack", "Env Decay", "Timbre Env", "Color Env", "Decay", "Release"];
    Elements, krabton_engine::dsp::elements_synth::SPECS, ["Geometry", "Brightness", "Damping", "Position", "Contour", "Bow", "Blow", "Strike"];
    Rings, krabton_engine::dsp::rings_synth::SPECS, ["Model", "Structure", "Brightness", "Damping", "Position", "Voices", "Width", "Volume"];
    Fm, krabton_engine::dsp::fm::SPECS, ["Brightness", "Cutoff", "Resonance", "Attack", "Release", "Detune", "Glide", "Volume"];
    Coast, krabton_engine::dsp::coast::SPECS, ["Timbre", "Symmetry", "Ratio", "FM", "Attack", "Decay", "Env Timbre", "Brightness"];
    Micro, krabton_engine::dsp::micro::SPECS, ["Cutoff", "Resonance", "Env Cutoff", "PWM", "PWM Rate", "Vibrato", "Filter Mode", "Arp Speed"];
    Console, krabton_engine::dsp::console::SPECS, ["Duty 1", "Duty 2", "Pulse 1", "Pulse 2", "Noise", "Attack", "Decay", "Release"];
    Handheld, krabton_engine::dsp::handheld::SPECS, ["Duty 1", "Duty 2", "Pulse 1", "Pulse 2", "Wave", "Noise", "Decay", "Treble"];
    Glide, krabton_engine::dsp::glide::SPECS, ["Decay", "Glide", "Drop", "Click", "Drive", "Overtone", "Tone", "Volume"];
    StringMachine, krabton_engine::dsp::string_machine::SPECS, ["Cello", "Viola", "Violin", "Attack", "Release", "Ensemble", "Tone", "Bite"];
    Tines, krabton_engine::dsp::tines::SPECS, ["Bark", "Hammer", "Bell", "Decay", "Release", "Tremolo", "Rate", "Tone"];
    Sweeps, krabton_engine::dsp::sweeps::SPECS, ["Kind", "Length", "Noise", "Resonance", "Curve", "Width", "Drive", "Volume"];
    Calculator, krabton_engine::dsp::calculator::SPECS, ["Sound", "Vibrato", "Tremolo", "Attack", "Decay", "Release", "Rhythm", "Volume"];
    ChipFm, krabton_engine::dsp::chipfm::SPECS, ["Algorithm", "Feedback", "Op1 Level", "LFO Rate", "AM Depth", "PM Depth", "Pitch Drop", "Volume"];
    Warp, krabton_engine::dsp::warp::SPECS, ["Warp 1", "Warp 2", "Level 1", "Level 2", "Detune", "Vib Depth", "Chorus", "Volume"];
    Oidos, krabton_engine::dsp::oidos::SPECS, ["Modes", "Fat", "Width", "Overtones", "Sharpness", "Harmonicity", "Filter Low", "Filter High"];
    Drone, krabton_engine::dsp::drone::SPECS, ["Tanpura", "Pad", "Swell", "Air", "Harmonics", "Evolve", "Tone", "Space"];
    Sampler, krabton_engine::dsp::sampler::SPECS, ["Cutoff", "Resonance", "Attack", "Decay", "Sustain", "Release", "Transpose", "Start"];
    Sf2, krabton_engine::dsp::sf2::SPECS, ["Cutoff", "Resonance", "Attack", "Decay", "Sustain", "Release", "Reverb", "Chorus"];
    Granulator, krabton_engine::dsp::granulator::SPECS, ["Position", "Spray", "Size", "Density", "Pitch", "Tone", "Smear", "Cloud"];
    Stretch, krabton_engine::dsp::stretch::SPECS, ["Stretch", "Position", "Shimmer", "Tone", "Space", "Pitch", "Attack", "Release"];
    BeadsSynth, krabton_engine::dsp::beads_synth::SPECS, ["Wave", "Size", "Shape", "Density", "Wave Random", "Size Random", "Grains", "Reverb"];
    Keytone, krabton_engine::dsp::keytone::SPECS, ["Sound", "Envelope", "Vib Depth", "Vib Rate", "Rate", "Bits", "Tone", "Volume"];
    Princess, krabton_engine::dsp::princess_machine::SPECS, ["Swing", "Deep Fry", "Fry Drive", "Pump", "Stutter", "Tape Stop", "808 Distortion", "Volume"];
    Turntable, krabton_engine::dsp::turntable::SPECS, ["Pitch", "Filter", "Resonance", "Fader", "Crackle", "Wow", "Noise", "Volume"];
    Guitar, krabton_engine::dsp::guitar::SPECS, ["Brightness", "Sustain", "Palm Mute", "Pick Position", "Strum", "Tone", "Body", "Hardness"];
    Piano, krabton_engine::piano::SPECS, ["Dynamics", "Una Corda", "Sustain", "Lid", "Tone", "Room", "Width", "Volume"];
    Organ, krabton_engine::dsp::organ::SPECS, ["16'", "8'", "4'", "2'", "Percussion", "Rotary", "Key Click", "Drive"];
    Voice, krabton_engine::dsp::stk_voice::SPECS, ["Morph", "Formant", "Brightness", "Breath", "Vib Depth", "Attack", "Release", "Singers"];
    Singer, krabton_engine::singer::SPECS, ["Gender", "Breath", "Vibrato", "Vib Rate", "Portamento", "Consonant", "Release", "Volume"];
    Monk, krabton_engine::dsp::monk::SPECS, ["Vowel", "Head", "Breath", "Vibrato", "Attack", "Release", "Unison", "Echo"];
    Mallets, krabton_engine::dsp::stk_mallets::SPECS, ["Model", "Control 1", "Control 2", "Control 3", "Control 4", "Control 5", "Control 6", "Control 7"];
    Strings, krabton_engine::dsp::stk_strings::SPECS, ["Model", "Control 1", "Control 2", "Control 3", "Control 4", "Control 5", "Control 6", "Control 7"];
    Winds, krabton_engine::dsp::stk_winds::SPECS, ["Model", "Control 1", "Control 2", "Control 3", "Control 4", "Control 5", "Control 6", "Control 7"];
    PanFlute, krabton_engine::dsp::pan_flute::SPECS, ["Breath", "Chiff", "Tone", "Vibrato", "Attack", "Release", "Space", "Volume"];
    Melodica, krabton_engine::dsp::melodica::SPECS, ["Breath", "Tone", "Beat", "Vibrato", "Attack", "Release", "Echo", "Drive"];
    Harmonica, krabton_engine::dsp::harmonica::SPECS, ["Mouth", "Bends", "Hands", "Wah", "Breath", "Tone", "Vibrato Rate", "Space"];
    Theremin, krabton_engine::dsp::theremin::SPECS, ["Waveform", "Brightness", "Formant", "Vibrato", "Rate", "Range", "Glide", "Volume"];
    Shakers, krabton_engine::dsp::shaker_synth::SPECS, ["Objects", "Grain", "Ring", "Shakes", "Rate", "Brightness", "Drive", "Volume"];
    StreetOrgan, krabton_engine::dsp::street_organ::SPECS, ["Bourdon", "Flute", "Violin", "Piccolo", "Pressure", "Tremulant", "Space", "Volume"];
    Orchestra, krabton_engine::orchestra::SPECS, ["Dynamics", "Velocity", "Attack", "Release", "Legato", "Legato Time", "Width", "Volume"];
    Modular, krabton_engine::modular::SPECS, ["Macro 1", "Macro 2", "Macro 3", "Macro 4", "Macro 5", "Macro 6", "Macro 7", "Macro 8"];
    Birds, krabton_engine::dsp::birds::SPECS, ["Busy", "Variety", "Tempo", "Pitch", "Distance", "Width", "Space", "Volume"];
    Nature, krabton_engine::dsp::nature::SPECS, ["Wind", "Rain", "Thunder", "Stream", "Sea", "Crickets", "Fire", "Space"];
    WindChimes, krabton_engine::dsp::wind_chimes::SPECS, ["Wind", "Gusts", "Tubes", "Material", "Clapper", "Ring", "Space", "Volume"];
}

/// Where a knob of a list is.
fn find(specs: &[ParamSpec], name: &str) -> Option<usize> {
    let (group, name) = match name.split_once("::") {
        Some((g, n)) => (Some(g), n),
        None => (None, name),
    };
    specs.iter().position(|s| s.shown() && s.name == name && group.is_none_or(|g| s.group == g))
}

/// The first set: the instrument's list, topped up with what tells most.
fn main_bank(specs: &[ParamSpec], list: &[&str]) -> Vec<usize> {
    let mut knobs: Vec<usize> = Vec::with_capacity(SIZE);
    for name in list {
        if let Some(i) = find(specs, name).filter(|i| !knobs.contains(i)) {
            knobs.push(i);
        }
    }
    let listed = knobs.len();
    for word in TELLING {
        let smooth = |s: &ParamSpec| s.shown() && matches!(s.kind, ParamKind::Linear | ParamKind::Log);
        let found = (0..specs.len()).find(|&i| !knobs.contains(&i) && smooth(&specs[i]) && specs[i].name.to_lowercase().contains(word));
        if let (Some(i), true) = (found, knobs.len() < SIZE) {
            knobs.push(i);
        }
    }
    for i in 0..specs.len() {
        if knobs.len() < SIZE && specs[i].shown() && !knobs.contains(&i) {
            knobs.push(i);
        }
    }
    // Found by looking: in the order the device has them.
    if listed == 0 {
        knobs.sort_unstable();
    }
    knobs
}

/// All the sets of a device. Up to eight parameters are one set; more get the
/// first set, then one for each group, a long group cut up (Effects A, B) and a
/// group of one or two sharing with the one before.
pub fn banks(specs: &[ParamSpec], list: &[&str]) -> Vec<Bank> {
    let shown: Vec<usize> = (0..specs.len()).filter(|&i| specs[i].shown()).collect();
    if shown.len() <= SIZE {
        return vec![Bank { name: String::new(), knobs: shown }];
    }
    let mut groups: Vec<(&str, Vec<usize>)> = Vec::new();
    for &i in &shown {
        match groups.iter_mut().find(|g| g.0 == specs[i].group) {
            Some(g) => g.1.push(i),
            None => groups.push((specs[i].group, vec![i])),
        }
    }
    let mut out = vec![Bank { name: "Main".to_string(), knobs: main_bank(specs, list) }];
    for (group, knobs) in groups {
        let group = if group.is_empty() { "More" } else { group };
        let many = knobs.len() > SIZE;
        for (k, part) in knobs.chunks(SIZE).enumerate() {
            let name = if many { format!("{group} {}", (b'A' + k as u8) as char) } else { group.to_string() };
            let last = out.len() - 1;
            let shares = last > 0 && out[last].knobs.len() + part.len() <= SIZE && (part.len() <= 2 || out[last].knobs.len() <= 2);
            if shares {
                out[last].name = format!("{} + {name}", out[last].name);
                out[last].knobs.extend_from_slice(part);
            } else {
                out.push(Bank { name, knobs: part.to_vec() });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_list_names_knobs_that_are_there() {
        for (variant, specs, list) in table() {
            assert_eq!(list.len(), SIZE, "{variant}: eight knobs");
            let mut found: Vec<usize> = list.iter().map(|n| find(specs, n).unwrap_or_else(|| panic!("{variant} has no {n}"))).collect();
            found.sort_unstable();
            found.dedup();
            assert_eq!(found.len(), SIZE, "{variant}: a knob twice in {list:?}");
        }
    }

    #[test]
    fn all_parameters_can_be_reached_after_the_first_set() {
        for (variant, specs, list) in table() {
            let sets = banks(specs, list);
            assert!(sets.iter().all(|b| b.knobs.len() <= SIZE && !b.knobs.is_empty()), "{variant}: {sets:?}");
            let shown = specs.iter().filter(|s| s.shown()).count();
            if shown > SIZE {
                assert_eq!(sets[0].name, "Main", "{variant}");
                assert_eq!(sets[0].knobs.iter().map(|&i| specs[i].name).count(), SIZE, "{variant}");
            }
            let mut all: Vec<usize> = sets.iter().skip(usize::from(shown > SIZE)).flat_map(|b| b.knobs.clone()).collect();
            all.sort_unstable();
            all.dedup();
            assert_eq!(all.len(), shown, "{variant}: not all there in {sets:?}");
        }
    }

    #[test]
    fn the_first_set_is_what_matters_and_the_rest_goes_by_group() {
        let specs = krabton_engine::dsp::analog::SPECS;
        let sets = banks(specs, main_names(&Instrument::analog("Init")));
        let names: Vec<&str> = sets[0].knobs.iter().map(|&i| specs[i].name).collect();
        assert_eq!(names, ["Cutoff", "Resonance", "Env Amount", "Detune 2", "Attack", "Decay", "Sustain", "Release"]);
        assert!(sets.iter().any(|b| b.name == "Filter" && b.knobs.iter().any(|&i| specs[i].name == "Cutoff")), "{sets:?}");
        // The same name twice: the list says which.
        let munich = krabton_engine::dsp::munich::SPECS;
        assert_eq!(munich[find(munich, "Loudness::Decay").unwrap()].group, "Loudness");
        assert_eq!(munich[find(munich, "Filter Contour::Decay").unwrap()].group, "Filter Contour");
        // A long group is cut up, all in one piece of the same name.
        let silicon = krabton_engine::dsp::silicon::SPECS;
        let sets = banks(silicon, &[]);
        assert!(sets.iter().any(|b| b.name == "Effects A") && sets.iter().any(|b| b.name.contains("Effects B")), "{sets:?}");
        // Without a list it looks for what tells most, in the device's own order.
        assert!(sets[0].knobs.windows(2).all(|w| w[0] < w[1]));
        assert!(sets[0].knobs.iter().any(|&i| silicon[i].name == "Cutoff"));
    }

    #[test]
    fn a_small_device_is_one_set_without_a_name() {
        let specs = krabton_engine::dsp::grit::SPECS;
        let sets = banks(specs, &[]);
        assert_eq!(sets.len(), 1);
        assert!(sets[0].name.is_empty());
        assert_eq!(sets[0].knobs.len(), specs.iter().filter(|s| s.shown()).count());
    }
}
