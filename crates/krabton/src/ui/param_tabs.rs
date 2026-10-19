//! Tabs for the devices with many knobs: their parameter groups split over
//! two or three pages of what belongs together (the sound, how it moves,
//! its effects, ...), picked from a narrow column beside the knobs.

use krabton_engine::dsp::{analog, chipfm, handheld, hybrid, micro, quartet, sf2, v2, waves};
use krabton_engine::params::ParamSpec;

/// A page: its name and the groups on it.
pub(crate) type Tab = (&'static str, &'static [&'static str]);

const V2: &[Tab] = &[
    ("Sound", &["Voice", "Osc 1", "Osc 2", "Osc 3", "VCF 1", "VCF 2", "Filters", "Voice Dist", "Play"]),
    ("Motion", &["Amp EG", "EG 2", "LFO 1", "LFO 2", "Matrix"]),
    ("Effects", &["Channel", "Ch Dist", "Chorus", "Compressor", "Reverb", "Delay", "Output"]),
];
const QUARTET: &[Tab] = &[
    ("Sound", &["Osc 1", "Osc 2", "Osc 3", "Osc 4", "Play"]),
    ("Shape", &["Filter A", "Filter B", "Amp", "Env 2", "Env 3", "Env 4", "Env 5", "LFOs", "Effects"]),
    ("Matrix", &["Matrix"]),
];
const CHIPFM: &[Tab] = &[("Chip", &["Chip", "LFO", "Play"]), ("Op 1 · 2", &["Op 1", "Op 2"]), ("Op 3 · 4", &["Op 3", "Op 4"])];
const MICRO: &[Tab] = &[("Voices", &["Voice 1", "Voice 2", "Voice 3"]), ("Play", &["Play", "Filter", "Output"])];
const WAVES: &[Tab] = &[("Oscillators", &["Osc 1", "Osc 2", "Sub + Noise"]), ("Shape", &["Filter", "Filter Env", "Amp Env", "LFO", "Voice", "Output"])];
const ANALOG: &[Tab] = &[("Sound", &["Osc 1", "Osc 2", "Mixer", "Filter"]), ("Motion", &["Filter Env", "Amp Env", "LFO", "Voice", "Output"])];
const HYBRID: &[Tab] = &[("Sound", &["Osc 1", "Osc 2", "Filter 1", "Filter 2"]), ("Motion", &["Amp", "Mod Env", "LFO", "Play"])];
const SF2: &[Tab] = &[("Sound", &["Channel", "Play"]), ("Shape", &["Envelope", "Filter", "Filter Env", "LFO"])];
const HANDHELD: &[Tab] = &[("Channels", &["Pulse 1", "Pulse 2", "Wave", "Noise"]), ("Play", &["Play", "Envelope", "Output"])];

/// The tabs of a device's parameters, if it has so many it gets them.
pub(crate) fn layout(specs: &[ParamSpec]) -> Option<&'static [Tab]> {
    let is = |s: &[ParamSpec]| std::ptr::eq(specs.as_ptr(), s.as_ptr()) && specs.len() == s.len();
    let pairs: [(&[ParamSpec], &'static [Tab]); 9] = [(v2::SPECS, V2), (quartet::SPECS, QUARTET), (chipfm::SPECS, CHIPFM), (micro::SPECS, MICRO), (waves::SPECS, WAVES), (analog::SPECS, ANALOG), (hybrid::SPECS, HYBRID), (handheld::SPECS, HANDHELD), (sf2::SPECS, SF2)];
    pairs.iter().find(|(s, _)| is(s)).map(|(_, t)| *t)
}

/// Whether `group` is on page `tab` (a group no page names goes on the first).
pub(crate) fn on_tab(tabs: &[Tab], tab: usize, group: &str) -> bool {
    match tabs.iter().position(|t| t.1.contains(&group)) {
        Some(t) => t == tab,
        None => tab == 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_group_of_a_tabbed_device_is_on_one_page() {
        for specs in [v2::SPECS, quartet::SPECS, chipfm::SPECS, micro::SPECS, waves::SPECS, analog::SPECS, hybrid::SPECS, handheld::SPECS, sf2::SPECS] {
            let tabs = layout(specs).expect("tabbed");
            assert!(tabs.len() >= 2 && tabs.len() <= 3);
            let mut groups: Vec<&str> = specs.iter().filter(|s| s.shown()).map(|s| s.group).collect();
            groups.dedup();
            for g in &groups {
                let named = tabs.iter().filter(|t| t.1.contains(g)).count();
                assert_eq!(named, 1, "{g} is on {named} pages");
            }
            // And every page has something on it.
            for (t, tab) in tabs.iter().enumerate() {
                assert!(groups.iter().any(|g| on_tab(tabs, t, g)), "{} is empty", tab.0);
            }
        }
        assert!(layout(krabton_engine::dsp::oidos::SPECS).is_none());
    }
}
