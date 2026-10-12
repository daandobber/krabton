//! Real Drums: an acoustic drum kit and a hand percussion kit made of
//! recordings, from the Versilian Community Sample Library (VCSL, CC0, by
//! Versilian Studios). Every pad has velocity layers (soft to loud) with
//! round robins, so no two hits sound the same. The samples are downloaded
//! on first use into Documents/Krabton/Library; until then each pad plays a
//! modelled stand-in (see `acoustic`).

use super::drums::DrumVoice;
use crate::project::DrumPad;
use crate::sample::SampleRef;
use std::path::PathBuf;

pub const FOLDER: &str = "Real Drums (VCSL)";
pub const BASE_URL: &str = "https://raw.githubusercontent.com/sgossner/VCSL/master/";
pub const CREDIT: &str = "Samples from the Versilian Community Sample Library (VCSL) by Versilian Studios, CC0 1.0: https://github.com/sgossner/VCSL";

pub struct RealPad {
    pub name: &'static str,
    pub note: u8,
    /// What plays until the samples are there.
    pub fallback: DrumVoice,
    pub level_db: f32,
    pub tune: f32,
    pub choke: u8,
    /// The folder in VCSL.
    pub dir: &'static str,
    /// Velocity layers, soft to loud, each with its round robins.
    pub layers: &'static [&'static [&'static str]],
}

const KICK: &str = "Membranophones/Struck Membranophones/Bass Drum 1";
const SNARE1: &str = "Membranophones/Struck Membranophones/Snare Drum, Modern 1";
const SNARE2: &str = "Membranophones/Struck Membranophones/Snare Drum, Modern 2";
const TOM_HI: &str = "Membranophones/Struck Membranophones/Tom 1/Stick";
const TOM_LO: &str = "Membranophones/Struck Membranophones/Tom 2/Stick";
const BONGOS: &str = "Membranophones/Struck Membranophones/Bongos";
const CONGA: &str = "Membranophones/Struck Membranophones/Conga";
const HATS: &str = "Idiophones/Struck Idiophones/Hi-Hat Cymbal";
const CYM1: &str = "Idiophones/Struck Idiophones/Suspended Cymbal 1";
const CYM2: &str = "Idiophones/Struck Idiophones/Suspended Cymbal 2";
const CLAPS: &str = "Idiophones/Struck Idiophones/Claps";
const TAMB: &str = "Idiophones/Struck Idiophones/Tambourine 1";
const COWBELLS: &str = "Idiophones/Struck Idiophones/Cowbells";
const CAJON: &str = "Idiophones/Struck Idiophones/Cajon";
const SHAKER: &str = "Idiophones/Struck Idiophones/Shaker, Small";
const CLAVES: &str = "Idiophones/Struck Idiophones/Claves";
const WOOD: &str = "Idiophones/Struck Idiophones/Woodblock";

const fn rp(name: &'static str, note: u8, fallback: DrumVoice, level_db: f32, dir: &'static str, layers: &'static [&'static [&'static str]]) -> RealPad {
    RealPad { name, note, fallback, level_db, tune: 0.0, choke: 0, dir, layers }
}

pub static REAL_KIT: &[RealPad] = &[
    rp("Kick", 36, DrumVoice::AcKick, 0.0, KICK, &[
        &["BDrumNew_hit_v2_rr1_Sum.wav", "BDrumNew_hit_v2_rr2_Sum.wav"],
        &["BDrumNew_hit_v3_rr1_Sum.wav", "BDrumNew_hit_v3_rr2_Sum.wav"],
        &["BDrumNew_hit_v5_rr1_Sum.wav", "BDrumNew_hit_v5_rr2_Sum.wav"],
        &["BDrumNew_hit_v7_rr1_Sum.wav", "BDrumNew_hit_v7_rr2_Sum.wav"],
    ]),
    rp("Side Stick", 37, DrumVoice::AcStick, -4.0, SNARE2, &[&["Snare3M_Xstick_v2_rr1_Mid.wav", "Snare3M_Xstick_v2_rr2_Mid.wav"]]),
    rp("Snare", 38, DrumVoice::AcSnare, -1.0, SNARE1, &[
        &["Snare2_HitSN_v3_rr1_Mid.wav", "Snare2_HitSN_v3_rr2_Mid.wav"],
        &["Snare2_HitSN_v5_rr1_Mid.wav", "Snare2_HitSN_v5_rr2_Mid.wav"],
        &["Snare2_HitSN_v6_rr1_Mid.wav", "Snare2_HitSN_v6_rr2_Mid.wav"],
        &["Snare2_HitSN_v7_rr1_Mid.wav", "Snare2_HitSN_v7_rr2_Mid.wav"],
        &["Snare2_HitSN_v9_rr1_Mid.wav", "Snare2_HitSN_v9_rr2_Mid.wav"],
    ]),
    rp("Claps", 39, DrumVoice::Clap, -4.0, CLAPS, &[&["Clap_rr1.wav", "Clap_rr2.wav", "Clap_rr3.wav", "Clap_rr4.wav", "Clap_rr5.wav", "Clap_rr6.wav"]]),
    RealPad { choke: 1, ..rp("Hi-Hat", 42, DrumVoice::AcHatClosed, -5.0, HATS, &[
        &["HiHat_HitC_v1_rr1_Mid.wav", "HiHat_HitC_v1_rr2_Mid.wav"],
        &["HiHat_HitC_v2_rr1_Mid.wav", "HiHat_HitC_v2_rr2_Mid.wav"],
        &["HiHat_HitC_v3_rr1_Mid.wav", "HiHat_HitC_v3_rr2_Mid.wav"],
        &["HiHat_HitC_v4_rr1_Mid.wav", "HiHat_HitC_v4_rr2_Mid.wav"],
    ]) },
    RealPad { choke: 1, ..rp("Pedal Hat", 44, DrumVoice::AcHatPedal, -5.0, HATS, &[&["HiHat_Close_rr1_Mid.wav", "HiHat_Close_rr2_Mid.wav"]]) },
    RealPad { choke: 1, ..rp("Open Hat", 46, DrumVoice::AcHatOpen, -5.0, HATS, &[&["HiHat_HitLoose_rr1_Mid.wav", "HiHat_HitLoose_rr2_Mid.wav"], &["HiHat_HitO_rr1_Mid.wav", "HiHat_HitO_rr2_Mid.wav"]]) },
    rp("Floor Tom", 43, DrumVoice::AcTomFloor, -2.0, TOM_LO, &[
        &["TomL_HitS_v2_rr1_Mid.wav", "TomL_HitS_v2_rr2_Mid.wav"],
        &["TomL_HitS_v3_rr1_Mid.wav", "TomL_HitS_v3_rr2_Mid.wav"],
        &["TomL_HitS_v4_rr1_Mid.wav", "TomL_HitS_v4_rr2_Mid.wav"],
    ]),
    RealPad { tune: -3.0, ..rp("Mid Tom", 47, DrumVoice::AcTomMid, -2.0, TOM_HI, &[
        &["TomH_HitS_v2_rr1_Mid.wav", "TomH_HitS_v2_rr2_Mid.wav"],
        &["TomH_HitS_v3_rr1_Mid.wav", "TomH_HitS_v3_rr2_Mid.wav"],
        &["TomH_HitS_v4_rr1_Mid.wav", "TomH_HitS_v4_rr2_Mid.wav"],
    ]) },
    rp("High Tom", 50, DrumVoice::AcTomHigh, -2.0, TOM_HI, &[
        &["TomH_HitS_v2_rr1_Mid.wav", "TomH_HitS_v2_rr2_Mid.wav"],
        &["TomH_HitS_v3_rr1_Mid.wav", "TomH_HitS_v3_rr2_Mid.wav"],
        &["TomH_HitS_v4_rr1_Mid.wav", "TomH_HitS_v4_rr2_Mid.wav"],
    ]),
    rp("Crash", 49, DrumVoice::AcCrash, -6.0, CYM1, &[&["susCymb1_hit_pp1.wav"], &["susCymb1_hit_mp1.wav"], &["susCymb1_hit_f1.wav"], &["susCymb1_hit_fff1.wav"]]),
    rp("Ride", 51, DrumVoice::AcRide, -6.0, CYM2, &[&["susCymb2_hit_stick_pp1.wav"], &["susCymb2_hit_stick_mp1.wav"], &["susCymb2_hit_stick_mf1.wav"]]),
    rp("Ride Bell", 53, DrumVoice::AcRideBell, -6.0, CYM2, &[&["susCymb2_hit_bell_p1.wav"], &["susCymb2_hit_bell_f1.wav"]]),
    rp("Tambourine", 54, DrumVoice::Tambourine, -8.0, TAMB, &[&["Tamb1_Hit_v1_rr1_Mid.wav"], &["Tamb1_Hit_v2_rr1_Mid.wav"]]),
    rp("Cowbell", 56, DrumVoice::Cowbell, -10.0, COWBELLS, &[&["Cowbell1_Hit_v2_rr1_Mid.wav"], &["Cowbell1_Hit_v3_rr1_Mid.wav"], &["Cowbell1_Hit_v4_rr1_Mid.wav"]]),
    rp("Crash 2", 57, DrumVoice::AcCrash, -7.0, CYM2, &[&["susCymb2_hit_pp1.wav"], &["susCymb2_hit_mp1.wav"], &["susCymb2_hit_mf1.wav"], &["susCymb2_hit_f1.wav"], &["susCymb2_hit_fff1.wav"]]),
];

pub static REAL_PERC: &[RealPad] = &[
    rp("Cajon Bass", 36, DrumVoice::AcKick, 0.0, CAJON, &[&["Cajon_hit1_mp_rr1.wav", "Cajon_hit1_mp_rr2.wav"], &["Cajon_hit1_f_rr1.wav", "Cajon_hit1_f_rr2.wav"], &["Cajon_hit1_fff_rr1.wav", "Cajon_hit1_fff_rr2.wav"]]),
    rp("Cajon Tone", 37, DrumVoice::AcStick, -2.0, CAJON, &[&["Cajon_hit3_p_rr1.wav", "Cajon_hit3_p_rr2.wav"], &["Cajon_hit3_mp_rr1.wav", "Cajon_hit3_mp_rr2.wav"], &["Cajon_hit3_f_rr1.wav", "Cajon_hit3_f_rr2.wav"]]),
    rp("Cajon Slap", 38, DrumVoice::AcSnare, -1.0, CAJON, &[&["Cajon_hit2_pp_rr1.wav", "Cajon_hit2_pp_rr2.wav"], &["Cajon_hit2_mp_rr1.wav", "Cajon_hit2_mp_rr2.wav"], &["Cajon_hit2_f_rr1.wav", "Cajon_hit2_f_rr2.wav"]]),
    rp("Hand Clap", 39, DrumVoice::Clap, -4.0, CLAPS, &[&["SoloClap_vl1.wav"], &["SoloClap_vl2.wav"], &["SoloClap_vl3.wav"], &["SoloClap_vl4.wav"]]),
    rp("Tambourine", 54, DrumVoice::Tambourine, -6.0, TAMB, &[&["Tamb1_Shake_rr1_Mid.wav", "Tamb1_Shake_rr2_Mid.wav"]]),
    rp("Cowbell", 56, DrumVoice::Cowbell, -8.0, COWBELLS, &[&["Cowbell2_Normal_v2_rr1_Mid.wav"], &["Cowbell2_Normal_v3_rr1_Mid.wav"]]),
    rp("High Bongo", 60, DrumVoice::Cr78Bongo, -3.0, BONGOS, &[
        &["BongoH_Hit1_v1_rr1_Mid.wav", "BongoH_Hit1_v1_rr2_Mid.wav"],
        &["BongoH_Hit1_v2_rr1_Mid.wav", "BongoH_Hit1_v2_rr2_Mid.wav"],
        &["BongoH_Hit1_v3_rr1_Mid.wav", "BongoH_Hit1_v3_rr2_Mid.wav"],
    ]),
    RealPad { tune: -5.0, ..rp("Low Bongo", 61, DrumVoice::Cr78Bongo, -3.0, BONGOS, &[
        &["BongoL_Hit1_v1_rr1_Mid.wav", "BongoL_Hit1_v1_rr2_Mid.wav"],
        &["BongoL_Hit1_v2_rr1_Mid.wav", "BongoL_Hit1_v2_rr2_Mid.wav"],
        &["BongoL_Hit1_v3_rr1_Mid.wav", "BongoL_Hit1_v3_rr2_Mid.wav"],
    ]) },
    rp("Muted Conga", 62, DrumVoice::M808Conga, -3.0, CONGA, &[&["Conga_HitFM_v1_rr1_Sum.wav", "Conga_HitFM_v1_rr2_Sum.wav"], &["Conga_HitFM_v2_rr1_Sum.wav", "Conga_HitFM_v2_rr2_Sum.wav"]]),
    rp("Conga", 63, DrumVoice::M808Conga, -3.0, CONGA, &[
        &["Conga_HitN_v1_rr1_Sum.wav", "Conga_HitN_v1_rr2_Sum.wav"],
        &["Conga_HitN_v2_rr1_Sum.wav", "Conga_HitN_v2_rr2_Sum.wav"],
        &["Conga_HitN_v3_rr1_Sum.wav", "Conga_HitN_v3_rr2_Sum.wav"],
    ]),
    RealPad { tune: -5.0, ..rp("Tumba", 64, DrumVoice::M808Conga, -3.0, CONGA, &[
        &["Tumba_HitN_v1_rr1_Sum.wav", "Tumba_HitN_v1_rr2_Sum.wav"],
        &["Tumba_HitN_v2_rr1_Sum.wav", "Tumba_HitN_v2_rr2_Sum.wav"],
        &["Tumba_HitN_v3_rr1_Sum.wav", "Tumba_HitN_v3_rr2_Sum.wav"],
        &["Tumba_HitN_v4_rr1_Sum.wav", "Tumba_HitN_v4_rr2_Sum.wav"],
    ]) },
    RealPad { tune: 4.0, ..rp("Quinto", 65, DrumVoice::M808Conga, -3.0, CONGA, &[
        &["Quinto_HitN_v1_rr1_Sum.wav", "Quinto_HitN_v1_rr2_Sum.wav"],
        &["Quinto_HitN_v2_rr1_Sum.wav", "Quinto_HitN_v2_rr2_Sum.wav"],
        &["Quinto_HitN_v3_rr1_Sum.wav", "Quinto_HitN_v3_rr2_Sum.wav"],
    ]) },
    rp("Shaker Up", 69, DrumVoice::Maraca, -6.0, SHAKER, &[&["Mid_ShakerHighFaster_Up_rr1.wav", "Mid_ShakerHighFaster_Up_rr2.wav"]]),
    rp("Shaker", 70, DrumVoice::Maraca, -6.0, SHAKER, &[&["Mid_Shaker_Slap_rr1.wav", "Mid_Shaker_Slap_rr2.wav"]]),
    rp("Claves", 75, DrumVoice::M808Claves, -8.0, CLAVES, &[&["Claves1_Hit_v1_rr1_Mid.wav"], &["Claves1_Hit_v2_rr1_Mid.wav"], &["Claves1_Hit_v3_rr1_Mid.wav"]]),
    RealPad { tune: -5.0, ..rp("Woodblock", 76, DrumVoice::M808Claves, -8.0, WOOD, &[
        &["wood_click_pp_rr1.wav", "wood_click_pp_rr2.wav", "wood_click_pp_rr3.wav"],
        &["wood_click_mp.wav"],
        &["wood_click_f_rr1.wav", "wood_click_f_rr2.wav"],
        &["wood_click_ff.wav"],
    ]) },
];

/// The pads of a kit (`Real Kit` or `Real Percussion`).
pub fn table(kit: &str) -> &'static [RealPad] {
    if kit == "Real Percussion" {
        REAL_PERC
    } else {
        REAL_KIT
    }
}

/// Where the samples live: Documents/Krabton/Library.
pub fn dir() -> PathBuf {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default();
    home.join("Documents").join("Krabton").join("Library").join(FOLDER)
}

/// Every file to download: (url, file name). Each file once.
pub fn files() -> Vec<(String, &'static str)> {
    let mut out: Vec<(String, &'static str)> = Vec::new();
    for p in REAL_KIT.iter().chain(REAL_PERC.iter()) {
        for name in p.layers.iter().flat_map(|l| l.iter()) {
            if !out.iter().any(|(_, n)| n == name) {
                let path = format!("{}/{}", p.dir, name).replace(' ', "%20").replace(',', "%2C");
                out.push((format!("{BASE_URL}{path}"), name));
            }
        }
    }
    out
}

/// True when every sample is on this computer.
pub fn installed() -> bool {
    let d = dir();
    files().iter().all(|(_, n)| d.join(n).is_file())
}

/// The pads of a kit: its recordings when they are here, else the stand-ins.
pub fn kit(name: &str) -> Vec<DrumPad> {
    let d = dir();
    table(name)
        .iter()
        .map(|rp| {
            let mut p = DrumPad::new(rp.fallback);
            p.name = rp.name.to_string();
            p.note = rp.note;
            p.level_db = rp.level_db;
            p.tune = rp.tune;
            p.choke = rp.choke;
            p.layers = rp
                .layers
                .iter()
                .map(|l| l.iter().filter_map(|n| SampleRef::load(&d.join(n)).ok()).collect::<Vec<_>>())
                .filter(|l: &Vec<SampleRef>| !l.is_empty())
                .collect();
            p
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kits_are_complete_and_their_files_listed_once() {
        for t in [REAL_KIT, REAL_PERC] {
            assert_eq!(t.len(), 16);
            let mut notes: Vec<u8> = t.iter().map(|p| p.note).collect();
            notes.sort();
            notes.dedup();
            assert_eq!(notes.len(), 16, "one pad per note");
            assert!(t.iter().all(|p| !p.layers.is_empty() && p.layers.iter().all(|l| !l.is_empty())));
        }
        let f = files();
        let mut names: Vec<&str> = f.iter().map(|x| x.1).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), f.len());
        assert!(f.iter().all(|(u, _)| u.starts_with(BASE_URL) && !u.contains(' ') && !u.contains(',')));
    }

    #[test]
    fn without_the_samples_the_stand_ins_play() {
        // A folder that is not there: every pad has its modelled voice and no layers.
        let pads = kit("Real Kit");
        assert_eq!(pads.len(), 16);
        if !installed() {
            assert!(pads.iter().all(|p| p.layers.is_empty()));
        }
    }
}
