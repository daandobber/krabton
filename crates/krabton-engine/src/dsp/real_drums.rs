//! Sampled drums: kits made of recordings, in packs that are downloaded on
//! first use into Documents/Krabton/Library (each from its own free
//! source, CC0 or Creative Commons with credit). Every pad has velocity
//! layers (soft to loud) with round robins where the source has them, so
//! no two hits sound the same. Until a pack is here, each pad plays a
//! stand-in voice of Krabton's own (see `acoustic`).
//!
//! The first pack: an acoustic kit and hand percussion from the Versilian
//! Community Sample Library (VCSL, CC0, by Versilian Studios).

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
    /// The folder in the pack's source ("" when the files carry their path).
    pub dir: &'static str,
    /// Velocity layers, soft to loud, each with its round robins.
    pub layers: &'static [&'static [&'static str]],
    /// Recorded with several microphones, one file each: then every entry
    /// of `layers` is a hit's path without its ending, and these are the
    /// endings with their pan (-1..1) and gain (dB). The hits are mixed to
    /// stereo once, after the download.
    pub mics: &'static [(&'static str, f32, f32)],
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
    RealPad { name, note, fallback, level_db, tune: 0.0, choke: 0, dir, layers, mics: &[] }
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


// ---------------------------------------------------------------- machines and more

const OPEN_DRUMS: &str = "https://raw.githubusercontent.com/fluid-music/open-drums/475cc3314fe06f6d1af02e9790ad9707c1f2b26b/";
const FISCHER: &str = "https://raw.githubusercontent.com/tidalcycles/sounds-tr808-fischer/85fbecf1bec32553395625ea659e2a56dfd7c0e1/";
const SONIC_PI: &str = "https://raw.githubusercontent.com/sonic-pi-net/sonic-pi/a0e3699a82a6ae236a759559eae842131d05a485/";
const TR707: &str = "tr-707/TR707WAV";
const SP: &str = "etc/samples";

/// A pad of one recording.
const fn one(name: &'static str, note: u8, fallback: DrumVoice, level_db: f32, dir: &'static str, file: &'static [&'static [&'static str]]) -> RealPad {
    rp(name, note, fallback, level_db, dir, file)
}

pub static KIT_707: &[RealPad] = &[
    one("Kick 2", 35, DrumVoice::AcKick, 0.0, TR707, &[&["BassDrum2.wav"]]),
    one("Kick", 36, DrumVoice::AcKick, 0.0, TR707, &[&["BassDrum1.wav"]]),
    one("Rim Shot", 37, DrumVoice::AcStick, -6.0, TR707, &[&["RimShot.wav"]]),
    one("Snare", 38, DrumVoice::AcSnare, -1.0, TR707, &[&["Snare1.wav"]]),
    one("Clap", 39, DrumVoice::Clap, -3.0, TR707, &[&["HandClap.wav"]]),
    one("Snare 2", 40, DrumVoice::AcSnare, -1.0, TR707, &[&["Snare2.wav"]]),
    RealPad { choke: 1, ..one("Closed Hat", 42, DrumVoice::AcHatClosed, -6.0, TR707, &[&["HhC.wav"]]) },
    one("Low Tom", 43, DrumVoice::AcTomFloor, -3.0, TR707, &[&["LowTom.wav"]]),
    RealPad { choke: 1, ..one("Open Hat", 46, DrumVoice::AcHatOpen, -7.0, TR707, &[&["HhO.wav"]]) },
    one("Mid Tom", 47, DrumVoice::AcTomMid, -3.0, TR707, &[&["MedTom.wav"]]),
    one("Crash", 49, DrumVoice::AcCrash, -9.0, TR707, &[&["Crash.wav"]]),
    one("High Tom", 50, DrumVoice::AcTomHigh, -3.0, TR707, &[&["HiTom.wav"]]),
    one("Ride", 51, DrumVoice::AcRide, -9.0, TR707, &[&["Ride.wav"]]),
    one("Tambourine", 54, DrumVoice::Tambourine, -9.0, TR707, &[&["Tamb.wav"]]),
    one("Cowbell", 56, DrumVoice::Cowbell, -9.0, TR707, &[&["CowBell.wav"]]),
];

pub static KIT_808: &[RealPad] = &[
    one("Boom", 35, DrumVoice::M808Kick, -1.0, "", &[&["bd8/BD2510.WAV"]]),
    one("Kick", 36, DrumVoice::M808Kick, 0.0, "", &[&["bd8/BD5025.WAV"]]),
    one("Rim Shot", 37, DrumVoice::M808Rim, -7.0, "", &[&["rs8/RS.WAV"]]),
    one("Snare", 38, DrumVoice::M808Snare, -1.0, "", &[&["sd8/SD5075.WAV"]]),
    one("Clap", 39, DrumVoice::M808Clap, -3.0, "", &[&["cp8/CP.WAV"]]),
    RealPad { choke: 1, ..one("Closed Hat", 42, DrumVoice::M808ClosedHat, -6.0, "", &[&["ch8/CH.WAV"]]) },
    one("Low Tom", 43, DrumVoice::M808Tom, -3.0, "", &[&["lt8/LT50.WAV"]]),
    RealPad { choke: 1, ..one("Open Hat", 46, DrumVoice::M808OpenHat, -7.0, "", &[&["oh8/OH50.WAV"]]) },
    one("Mid Tom", 47, DrumVoice::M808Tom, -3.0, "", &[&["mt8/MT50.WAV"]]),
    one("Cymbal", 49, DrumVoice::M808Cymbal, -9.0, "", &[&["cy8/CY5075.WAV"]]),
    one("High Tom", 50, DrumVoice::M808Tom, -3.0, "", &[&["ht8/HT50.WAV"]]),
    one("Cowbell", 56, DrumVoice::M808Cowbell, -8.0, "", &[&["cb8/CB.WAV"]]),
    one("High Conga", 63, DrumVoice::M808Conga, -4.0, "", &[&["hc8/HC50.WAV"]]),
    one("Low Conga", 64, DrumVoice::M808Conga, -4.0, "", &[&["lc8/LC50.WAV"]]),
    one("Maracas", 70, DrumVoice::M808Maracas, -8.0, "", &[&["ma8/MA.WAV"]]),
    one("Claves", 75, DrumVoice::M808Claves, -9.0, "", &[&["cl8/CL.WAV"]]),
];

pub static KIT_ELECTRO: &[RealPad] = &[
    one("Hollow Kick", 35, DrumVoice::M808Kick, 0.0, SP, &[&["elec_hollow_kick.flac"]]),
    one("Kick", 36, DrumVoice::M808Kick, 0.0, SP, &[&["elec_soft_kick.flac"]]),
    one("Wood", 37, DrumVoice::M808Rim, -6.0, SP, &[&["elec_wood.flac"]]),
    one("Snare", 38, DrumVoice::M808Snare, -2.0, SP, &[&["elec_snare.flac"]]),
    one("Pop", 39, DrumVoice::M808Clap, -4.0, SP, &[&["elec_pop.flac"]]),
    one("High Snare", 40, DrumVoice::M808Snare, -2.0, SP, &[&["elec_hi_snare.flac"]]),
    RealPad { choke: 1, ..one("Tick", 42, DrumVoice::M808ClosedHat, -6.0, SP, &[&["elec_tick.flac"]]) },
    one("Fuzz Tom", 45, DrumVoice::M808Tom, -3.0, SP, &[&["elec_fuzz_tom.flac"]]),
    RealPad { choke: 1, ..one("Cymbal", 46, DrumVoice::M808OpenHat, -8.0, SP, &[&["elec_cymbal.flac"]]) },
    one("Bong", 56, DrumVoice::M808Cowbell, -7.0, SP, &[&["elec_bong.flac"]]),
    one("Low Snare", 60, DrumVoice::M808Snare, -2.0, SP, &[&["elec_lo_snare.flac"]]),
    one("Filter Snare", 61, DrumVoice::M808Snare, -4.0, SP, &[&["elec_filt_snare.flac"]]),
    one("Bell", 67, DrumVoice::M808Cowbell, -9.0, SP, &[&["elec_bell.flac"]]),
    one("Blip", 71, DrumVoice::M808Claves, -10.0, SP, &[&["elec_blip.flac"]]),
    one("Triangle", 81, DrumVoice::M808Claves, -10.0, SP, &[&["elec_triangle.flac"]]),
    one("Chime", 84, DrumVoice::M808Cymbal, -11.0, SP, &[&["elec_chime.flac"]]),
];

pub static KIT_CLUB: &[RealPad] = &[
    one("Boom Kick", 35, DrumVoice::M808Kick, -2.0, SP, &[&["bd_boom.flac"]]),
    one("Kick", 36, DrumVoice::M808Kick, 0.0, SP, &[&["bd_haus.flac"]]),
    one("Tap", 37, DrumVoice::M808Rim, -6.0, SP, &[&["tbd_perc_tap_1.flac"]]),
    one("Snare", 38, DrumVoice::M808Snare, -1.0, SP, &[&["sn_dolf.flac"]]),
    one("Snap", 39, DrumVoice::Clap, -4.0, SP, &[&["perc_snap.flac"]]),
    one("Snare 2", 40, DrumVoice::M808Snare, -1.0, SP, &[&["sn_zome.flac"]]),
    RealPad { choke: 1, ..one("Closed Hat", 42, DrumVoice::M808ClosedHat, -6.0, SP, &[&["hat_tap.flac"]]) },
    RealPad { choke: 1, ..one("Accent Hat", 44, DrumVoice::M808ClosedHat, -6.0, SP, &[&["hat_snap.flac"]]) },
    RealPad { choke: 1, ..one("Open Hat", 46, DrumVoice::M808OpenHat, -7.0, SP, &[&["hat_gump.flac"]]) },
    one("Ride", 51, DrumVoice::AcRide, -10.0, SP, &[&["ride_tri.flac"]]),
    one("Klub Kick", 60, DrumVoice::M808Kick, 0.0, SP, &[&["bd_klub.flac"]]),
    one("Tek Kick", 61, DrumVoice::M808Kick, 0.0, SP, &[&["bd_tek.flac"]]),
    one("808 Kick", 65, DrumVoice::M808Kick, -1.0, SP, &[&["bd_808.flac"]]),
    one("Dub Snare", 66, DrumVoice::M808Snare, -2.0, SP, &[&["sn_dub.flac"]]),
    one("Noise Hat", 67, DrumVoice::M808OpenHat, -8.0, SP, &[&["hat_noiz.flac"]]),
    one("Metal Hat", 68, DrumVoice::M808OpenHat, -9.0, SP, &[&["hat_metal.flac"]]),
];

pub static KIT_GLITCH: &[RealPad] = &[
    one("Chip Kick", 35, DrumVoice::M808Kick, 0.0, SP, &[&["bd_chip.flac"]]),
    one("Kick", 36, DrumVoice::M808Kick, 0.0, SP, &[&["bd_mehackit.flac"]]),
    one("Blip", 37, DrumVoice::M808Rim, -6.0, SP, &[&["tbd_perc_blip.flac"]]),
    one("Snare", 38, DrumVoice::M808Snare, -2.0, SP, &[&["sn_generic.flac"]]),
    one("Glitch 1", 39, DrumVoice::M808Clap, -6.0, SP, &[&["glitch_perc1.flac"]]),
    one("Glitch 2", 40, DrumVoice::M808Snare, -6.0, SP, &[&["glitch_perc2.flac"]]),
    RealPad { choke: 1, ..one("Tick", 42, DrumVoice::M808ClosedHat, -7.0, SP, &[&["tbd_perc_hat.flac"]]) },
    one("Impact", 45, DrumVoice::M808Tom, -4.0, SP, &[&["perc_impact1.flac"]]),
    RealPad { choke: 1, ..one("Swash", 46, DrumVoice::M808OpenHat, -8.0, SP, &[&["perc_swash.flac"]]) },
    one("Swoosh", 49, DrumVoice::M808Cymbal, -9.0, SP, &[&["perc_swoosh.flac"]]),
    one("Glitch 3", 60, DrumVoice::M808Clap, -6.0, SP, &[&["glitch_perc3.flac"]]),
    one("Glitch 4", 61, DrumVoice::M808Clap, -6.0, SP, &[&["glitch_perc4.flac"]]),
    one("Robot", 65, DrumVoice::M808Cowbell, -7.0, SP, &[&["glitch_robot1.flac"]]),
    one("Glitch Bass", 67, DrumVoice::M808Tom, -6.0, SP, &[&["glitch_bass_g.flac"]]),
    one("Elec Blip", 71, DrumVoice::M808Claves, -10.0, SP, &[&["elec_blip.flac"]]),
    one("Twip", 72, DrumVoice::M808Claves, -10.0, SP, &[&["elec_twip.flac"]]),
];

pub static KIT_TABLA: &[RealPad] = &[
    RealPad { choke: 3, ..one("Ghe Long", 35, DrumVoice::M808Tom, 0.0, SP, &[&["tabla_ghe2.flac", "tabla_ghe3.flac", "tabla_ghe7.flac"]]) },
    RealPad { choke: 3, ..one("Ghe", 36, DrumVoice::M808Tom, 0.0, SP, &[&["tabla_ghe1.flac", "tabla_ghe8.flac", "tabla_ghe4.flac"]]) },
    RealPad { choke: 3, ..one("Ke", 37, DrumVoice::AcStick, -4.0, SP, &[&["tabla_ke1.flac", "tabla_ke2.flac", "tabla_ke3.flac"]]) },
    RealPad { choke: 2, ..one("Na", 38, DrumVoice::M808Conga, -2.0, SP, &[&["tabla_na.flac"]]) },
    RealPad { choke: 2, ..one("Ta", 39, DrumVoice::M808Conga, -2.0, SP, &[&["tabla_tas1.flac", "tabla_tas2.flac", "tabla_tas3.flac"]]) },
    RealPad { choke: 2, ..one("Na Open", 40, DrumVoice::M808Conga, -2.0, SP, &[&["tabla_na_o.flac"]]) },
    RealPad { choke: 2, ..one("Te", 42, DrumVoice::Cr78Bongo, -4.0, SP, &[&["tabla_te1.flac", "tabla_te2.flac"]]) },
    RealPad { choke: 3, ..one("Ghe Boom", 43, DrumVoice::M808Tom, 0.0, SP, &[&["tabla_ghe5.flac", "tabla_ghe6.flac"]]) },
    RealPad { choke: 2, ..one("Re", 44, DrumVoice::Cr78Bongo, -4.0, SP, &[&["tabla_re.flac"]]) },
    RealPad { choke: 2, ..one("Tun", 46, DrumVoice::M808Conga, -3.0, SP, &[&["tabla_tun1.flac", "tabla_tun2.flac", "tabla_tun3.flac"]]) },
    one("Dhe", 47, DrumVoice::M808Conga, 0.0, SP, &[&["tabla_dhec.flac"]]),
    RealPad { choke: 2, ..one("Na Short", 50, DrumVoice::M808Conga, -2.0, SP, &[&["tabla_na_s.flac"]]) },
    RealPad { choke: 2, ..one("Te Ne", 60, DrumVoice::Cr78Bongo, -4.0, SP, &[&["tabla_te_ne.flac"]]) },
    RealPad { choke: 2, ..one("Te M", 61, DrumVoice::Cr78Bongo, -4.0, SP, &[&["tabla_te_m.flac"]]) },
];

pub static KIT_DRY: &[RealPad] = &[
    one("Heavy Kick", 35, DrumVoice::AcKick, 0.0, SP, &[&["drum_heavy_kick.flac"]]),
    one("Kick", 36, DrumVoice::AcKick, 0.0, SP, &[&["drum_bass_soft.flac"], &["drum_bass_hard.flac"]]),
    one("Snare", 38, DrumVoice::AcSnare, -1.0, SP, &[&["drum_snare_soft.flac"], &["drum_snare_hard.flac"]]),
    RealPad { choke: 1, ..one("Closed Hat", 42, DrumVoice::AcHatClosed, -6.0, SP, &[&["drum_cymbal_closed.flac"]]) },
    one("Floor Tom", 43, DrumVoice::AcTomFloor, -3.0, SP, &[&["drum_tom_lo_soft.flac"], &["drum_tom_lo_hard.flac"]]),
    RealPad { choke: 1, ..one("Pedal Hat", 44, DrumVoice::AcHatPedal, -6.0, SP, &[&["drum_cymbal_pedal.flac"]]) },
    RealPad { choke: 1, ..one("Open Hat", 46, DrumVoice::AcHatOpen, -7.0, SP, &[&["drum_cymbal_open.flac"]]) },
    one("Mid Tom", 47, DrumVoice::AcTomMid, -3.0, SP, &[&["drum_tom_mid_soft.flac"], &["drum_tom_mid_hard.flac"]]),
    one("Crash", 49, DrumVoice::AcCrash, -8.0, SP, &[&["drum_cymbal_soft.flac"], &["drum_cymbal_hard.flac"]]),
    one("High Tom", 50, DrumVoice::AcTomHigh, -3.0, SP, &[&["drum_tom_hi_soft.flac"], &["drum_tom_hi_hard.flac"]]),
    one("Ride", 51, DrumVoice::AcRide, -9.0, SP, &[&["ride_via.flac"]]),
    one("Splash", 55, DrumVoice::AcCrash, -9.0, SP, &[&["drum_splash_soft.flac"], &["drum_splash_hard.flac"]]),
    one("Cowbell", 56, DrumVoice::Cowbell, -10.0, SP, &[&["drum_cowbell.flac"]]),
];

// ---------------------------------------------------------------- acoustic kits of many kinds

/// Rusty Kit.
pub static KIT_RUSTY: &[RealPad] = &[
    rp("Kick", 36, DrumVoice::AcKick, 0.0, "", &[&["Samples/kick_24/kick/kick/k_vl3_rr1.flac", "Samples/kick_24/kick/kick/k_vl3_rr2.flac"], &["Samples/kick_24/kick/kick/k_vl7_rr1.flac", "Samples/kick_24/kick/kick/k_vl7_rr2.flac"], &["Samples/kick_24/kick/kick/k_vl10_rr1.flac", "Samples/kick_24/kick/kick/k_vl10_rr2.flac"], &["Samples/kick_24/kick/kick/k_vl14_rr1.flac", "Samples/kick_24/kick/kick/k_vl14_rr2.flac"]]),
    rp("Side Stick", 37, DrumVoice::AcStick, -5.0, "", &[&["Samples/snare_14/sidestick/oh/sn_ss_vl1_rr1.flac", "Samples/snare_14/sidestick/oh/sn_ss_vl1_rr2.flac"], &["Samples/snare_14/sidestick/oh/sn_ss_vl2_rr1.flac", "Samples/snare_14/sidestick/oh/sn_ss_vl2_rr2.flac"], &["Samples/snare_14/sidestick/oh/sn_ss_vl3_rr1.flac", "Samples/snare_14/sidestick/oh/sn_ss_vl3_rr2.flac"], &["Samples/snare_14/sidestick/oh/sn_ss_vl4_rr1.flac", "Samples/snare_14/sidestick/oh/sn_ss_vl4_rr2.flac"]]),
    rp("Snare", 38, DrumVoice::AcSnare, -1.0, "", &[&["Samples/snare_14/center/oh/sn_center_vl2_rr1.flac", "Samples/snare_14/center/oh/sn_center_vl2_rr2.flac"], &["Samples/snare_14/center/oh/sn_center_vl5_rr1.flac", "Samples/snare_14/center/oh/sn_center_vl5_rr2.flac"], &["Samples/snare_14/center/oh/sn_center_vl7_rr1.flac", "Samples/snare_14/center/oh/sn_center_vl7_rr2.flac"], &["Samples/snare_14/center/oh/sn_center_vl10_rr1.flac", "Samples/snare_14/center/oh/sn_center_vl10_rr2.flac"]]),
    rp("Rimshot", 40, DrumVoice::AcSnare, -2.0, "", &[&["Samples/snare_14/rimshot/oh/sn_rims_vl1_rr1.flac", "Samples/snare_14/rimshot/oh/sn_rims_vl1_rr2.flac"], &["Samples/snare_14/rimshot/oh/sn_rims_vl3_rr1.flac", "Samples/snare_14/rimshot/oh/sn_rims_vl3_rr2.flac"], &["Samples/snare_14/rimshot/oh/sn_rims_vl4_rr1.flac", "Samples/snare_14/rimshot/oh/sn_rims_vl4_rr2.flac"], &["Samples/snare_14/rimshot/oh/sn_rims_vl6_rr1.flac", "Samples/snare_14/rimshot/oh/sn_rims_vl6_rr2.flac"]]),
    rp("Low Floor Tom 22in", 41, DrumVoice::AcTomFloor, -3.0, "", &[&["Samples/tom_22/center/oh/t22_vl2_rr1.flac", "Samples/tom_22/center/oh/t22_vl2_rr2.flac"], &["Samples/tom_22/center/oh/t22_vl4_rr1.flac", "Samples/tom_22/center/oh/t22_vl4_rr2.flac"], &["Samples/tom_22/center/oh/t22_vl5_rr1.flac", "Samples/tom_22/center/oh/t22_vl5_rr2.flac"], &["Samples/tom_22/center/oh/t22_vl7_rr1.flac", "Samples/tom_22/center/oh/t22_vl7_rr2.flac"]]),
    RealPad { choke: 1, ..rp("Closed Hat", 42, DrumVoice::AcHatClosed, -7.0, "", &[&["Samples/hihat_14/tc/oh/ht_tc_vl2_rr1.flac", "Samples/hihat_14/tc/oh/ht_tc_vl2_rr2.flac"], &["Samples/hihat_14/tc/oh/ht_tc_vl4_rr1.flac", "Samples/hihat_14/tc/oh/ht_tc_vl4_rr2.flac"], &["Samples/hihat_14/tc/oh/ht_tc_vl6_rr1.flac", "Samples/hihat_14/tc/oh/ht_tc_vl6_rr2.flac"], &["Samples/hihat_14/tc/oh/ht_tc_vl8_rr1.flac", "Samples/hihat_14/tc/oh/ht_tc_vl8_rr2.flac"]]) },
    rp("Floor Tom 18in", 43, DrumVoice::AcTomFloor, -3.0, "", &[&["Samples/tom_18/center/oh/t18_vl2_rr1.flac", "Samples/tom_18/center/oh/t18_vl2_rr2.flac"], &["Samples/tom_18/center/oh/t18_vl4_rr1.flac", "Samples/tom_18/center/oh/t18_vl4_rr2.flac"], &["Samples/tom_18/center/oh/t18_vl6_rr1.flac", "Samples/tom_18/center/oh/t18_vl6_rr2.flac"], &["Samples/tom_18/center/oh/t18_vl8_rr1.flac", "Samples/tom_18/center/oh/t18_vl8_rr2.flac"]]),
    RealPad { choke: 1, ..rp("Pedal Hat", 44, DrumVoice::AcHatPedal, -9.0, "", &[&["Samples/hihat_14/chik/oh/ht_chik_vl1_rr1.flac", "Samples/hihat_14/chik/oh/ht_chik_vl1_rr2.flac"], &["Samples/hihat_14/chik/oh/ht_chik_vl2_rr1.flac", "Samples/hihat_14/chik/oh/ht_chik_vl2_rr2.flac"], &["Samples/hihat_14/chik/oh/ht_chik_vl4_rr1.flac", "Samples/hihat_14/chik/oh/ht_chik_vl4_rr2.flac"], &["Samples/hihat_14/chik/oh/ht_chik_vl5_rr1.flac", "Samples/hihat_14/chik/oh/ht_chik_vl5_rr2.flac"]]) },
    RealPad { choke: 1, ..rp("Open Hat", 46, DrumVoice::AcHatOpen, -7.0, "", &[&["Samples/hihat_14/open/oh/ht_open_vl1_rr1.flac", "Samples/hihat_14/open/oh/ht_open_vl1_rr2.flac"], &["Samples/hihat_14/open/oh/ht_open_vl3_rr1.flac", "Samples/hihat_14/open/oh/ht_open_vl3_rr2.flac"], &["Samples/hihat_14/open/oh/ht_open_vl4_rr1.flac", "Samples/hihat_14/open/oh/ht_open_vl4_rr2.flac"], &["Samples/hihat_14/open/oh/ht_open_vl6_rr1.flac", "Samples/hihat_14/open/oh/ht_open_vl6_rr2.flac"]]) },
    rp("Mid Tom 15in", 47, DrumVoice::AcTomMid, -3.0, "", &[&["Samples/tom_15/center/oh/t15_vl2_rr1.flac", "Samples/tom_15/center/oh/t15_vl2_rr2.flac"], &["Samples/tom_15/center/oh/t15_vl4_rr1.flac", "Samples/tom_15/center/oh/t15_vl4_rr2.flac"], &["Samples/tom_15/center/oh/t15_vl5_rr1.flac", "Samples/tom_15/center/oh/t15_vl5_rr2.flac"], &["Samples/tom_15/center/oh/t15_vl7_rr1.flac", "Samples/tom_15/center/oh/t15_vl7_rr2.flac"]]),
    rp("Crash", 49, DrumVoice::AcCrash, -6.0, "", &[&["Samples/crash_17/cr/oh/cr_vl1_rr1.flac", "Samples/crash_17/cr/oh/cr_vl1_rr2.flac"], &["Samples/crash_17/cr/oh/cr_vl2_rr1.flac", "Samples/crash_17/cr/oh/cr_vl2_rr2.flac"], &["Samples/crash_17/cr/oh/cr_vl4_rr1.flac", "Samples/crash_17/cr/oh/cr_vl4_rr2.flac"], &["Samples/crash_17/cr/oh/cr_vl5_rr1.flac", "Samples/crash_17/cr/oh/cr_vl5_rr2.flac"]]),
    rp("High Tom 14in", 50, DrumVoice::AcTomHigh, -3.0, "", &[&["Samples/tom_14/center/oh/t14_vl1_rr1.flac", "Samples/tom_14/center/oh/t14_vl1_rr2.flac"], &["Samples/tom_14/center/oh/t14_vl3_rr1.flac", "Samples/tom_14/center/oh/t14_vl3_rr2.flac"], &["Samples/tom_14/center/oh/t14_vl4_rr1.flac", "Samples/tom_14/center/oh/t14_vl4_rr2.flac"], &["Samples/tom_14/center/oh/t14_vl6_rr1.flac", "Samples/tom_14/center/oh/t14_vl6_rr2.flac"]]),
    rp("Ride", 51, DrumVoice::AcRide, -8.0, "", &[&["Samples/ride_22/rd/oh/rd_vl2_rr1.flac", "Samples/ride_22/rd/oh/rd_vl2_rr2.flac"], &["Samples/ride_22/rd/oh/rd_vl5_rr1.flac", "Samples/ride_22/rd/oh/rd_vl5_rr2.flac"], &["Samples/ride_22/rd/oh/rd_vl7_rr1.flac", "Samples/ride_22/rd/oh/rd_vl7_rr2.flac"], &["Samples/ride_22/rd/oh/rd_vl10_rr1.flac", "Samples/ride_22/rd/oh/rd_vl10_rr2.flac"]]),
    rp("China", 52, DrumVoice::AcChina, -7.0, "", &[&["Samples/china_18/cn/oh/cn_vl1_rr1.flac", "Samples/china_18/cn/oh/cn_vl1_rr2.flac"], &["Samples/china_18/cn/oh/cn_vl2_rr1.flac", "Samples/china_18/cn/oh/cn_vl2_rr2.flac"], &["Samples/china_18/cn/oh/cn_vl4_rr1.flac", "Samples/china_18/cn/oh/cn_vl4_rr2.flac"], &["Samples/china_18/cn/oh/cn_vl5_rr1.flac", "Samples/china_18/cn/oh/cn_vl5_rr2.flac"]]),
    rp("Ride Bell", 53, DrumVoice::AcRideBell, -7.0, "", &[&["Samples/ride_22/bl/oh/rd_bl_vl1_rr1.flac", "Samples/ride_22/bl/oh/rd_bl_vl1_rr2.flac"], &["Samples/ride_22/bl/oh/rd_bl_vl2_rr1.flac", "Samples/ride_22/bl/oh/rd_bl_vl2_rr2.flac"], &["Samples/ride_22/bl/oh/rd_bl_vl3_rr1.flac", "Samples/ride_22/bl/oh/rd_bl_vl3_rr2.flac"], &["Samples/ride_22/bl/oh/rd_bl_vl4_rr1.flac", "Samples/ride_22/bl/oh/rd_bl_vl4_rr2.flac"]]),
    rp("Sizzle Crash", 57, DrumVoice::AcCrash, -6.0, "", &[&["Samples/crash_sizzle_17/cr/oh/crs_vl1_rr1.flac", "Samples/crash_sizzle_17/cr/oh/crs_vl1_rr2.flac"], &["Samples/crash_sizzle_17/cr/oh/crs_vl3_rr1.flac", "Samples/crash_sizzle_17/cr/oh/crs_vl3_rr2.flac"], &["Samples/crash_sizzle_17/cr/oh/crs_vl4_rr1.flac", "Samples/crash_sizzle_17/cr/oh/crs_vl4_rr2.flac"], &["Samples/crash_sizzle_17/cr/oh/crs_vl6_rr1.flac", "Samples/crash_sizzle_17/cr/oh/crs_vl6_rr2.flac"]]),
];

/// Swirly Brush Kit.
pub static KIT_SWIRLY: &[RealPad] = &[
    rp("Kick", 36, DrumVoice::AcKick, 0.0, "", &[&["Samples/marching_kick/marching_kick_vl4_rr1_reso.wav", "Samples/marching_kick/marching_kick_vl4_rr2_reso.wav"], &["Samples/marching_kick/marching_kick_vl10_rr1_reso.wav", "Samples/marching_kick/marching_kick_vl10_rr2_reso.wav"], &["Samples/marching_kick/marching_kick_vl16_rr1_reso.wav", "Samples/marching_kick/marching_kick_vl16_rr2_reso.wav"], &["Samples/marching_kick/marching_kick_vl22_rr1_reso.wav", "Samples/marching_kick/marching_kick_vl22_rr2_reso.wav"]]),
    rp("Brush Dig", 37, DrumVoice::AcBrush, -5.0, "", &[&["Samples/snare_dig/snare_dig_vl1_rr1_top.wav", "Samples/snare_dig/snare_dig_vl1_rr2_top.wav"], &["Samples/snare_dig/snare_dig_vl3_rr1_top.wav", "Samples/snare_dig/snare_dig_vl3_rr2_top.wav"], &["Samples/snare_dig/snare_dig_vl4_rr1_top.wav", "Samples/snare_dig/snare_dig_vl4_rr2_top.wav"], &["Samples/snare_dig/snare_dig_vl6_rr1_top.wav", "Samples/snare_dig/snare_dig_vl6_rr2_top.wav"]]),
    rp("Snare Brush", 38, DrumVoice::AcBrush, -1.0, "", &[&["Samples/snare_main/snare_hit_vl2_rr1_top.wav", "Samples/snare_main/snare_hit_vl2_rr2_top.wav"], &["Samples/snare_main/snare_hit_vl5_rr1_top.wav", "Samples/snare_main/snare_hit_vl5_rr2_top.wav"], &["Samples/snare_main/snare_hit_vl9_rr1_top.wav", "Samples/snare_main/snare_hit_vl9_rr2_top.wav"], &["Samples/snare_main/snare_hit_vl12_rr1_top.wav", "Samples/snare_main/snare_hit_vl12_rr2_top.wav"]]),
    rp("Brush Swirl", 39, DrumVoice::AcBrush, -3.0, "", &[&["Samples/snare_stir/stir_dl1_skin_rr1.wav", "Samples/snare_stir/stir_dl1_skin_rr2.wav"], &["Samples/snare_stir/stir_dl3_skin_rr1.wav", "Samples/snare_stir/stir_dl3_skin_rr2.wav"]]),
    rp("Snare Edge", 40, DrumVoice::AcBrush, -2.0, "", &[&["Samples/snare_edge/snare_edge_vl2_rr1_top.wav", "Samples/snare_edge/snare_edge_vl2_rr2_top.wav"], &["Samples/snare_edge/snare_edge_vl5_rr1_top.wav", "Samples/snare_edge/snare_edge_vl5_rr2_top.wav"], &["Samples/snare_edge/snare_edge_vl7_rr1_top.wav", "Samples/snare_edge/snare_edge_vl7_rr2_top.wav"], &["Samples/snare_edge/snare_edge_vl10_rr1_top.wav", "Samples/snare_edge/snare_edge_vl10_rr2_top.wav"]]),
    RealPad { choke: 1, ..rp("Closed Hat", 42, DrumVoice::AcHatClosed, -7.0, "", &[&["Samples/hat_tight/hh_tight_vl3_rr1.wav", "Samples/hat_tight/hh_tight_vl3_rr2.wav"], &["Samples/hat_tight/hh_tight_vl7_rr1.wav", "Samples/hat_tight/hh_tight_vl7_rr2.wav"], &["Samples/hat_tight/hh_tight_vl11_rr1.wav", "Samples/hat_tight/hh_tight_vl11_rr2.wav"], &["Samples/hat_tight/hh_tight_vl15_rr1.wav", "Samples/hat_tight/hh_tight_vl15_rr2.wav"]]) },
    rp("Floor Tom 16in", 43, DrumVoice::AcTomFloor, -3.0, "", &[&["Samples/tom_floor/tom_floor_vl2_rr1.wav", "Samples/tom_floor/tom_floor_vl2_rr2.wav"], &["Samples/tom_floor/tom_floor_vl5_rr1.wav", "Samples/tom_floor/tom_floor_vl5_rr2.wav"], &["Samples/tom_floor/tom_floor_vl9_rr1.wav", "Samples/tom_floor/tom_floor_vl9_rr2.wav"], &["Samples/tom_floor/tom_floor_vl12_rr1.wav", "Samples/tom_floor/tom_floor_vl12_rr2.wav"]]),
    RealPad { choke: 1, ..rp("Pedal Hat", 44, DrumVoice::AcHatPedal, -9.0, "", &[&["Samples/hat_foot/hh_foot_vl1_rr1.wav", "Samples/hat_foot/hh_foot_vl1_rr2.wav"], &["Samples/hat_foot/hh_foot_vl3_rr1.wav", "Samples/hat_foot/hh_foot_vl3_rr2.wav"], &["Samples/hat_foot/hh_foot_vl4_rr1.wav", "Samples/hat_foot/hh_foot_vl4_rr2.wav"], &["Samples/hat_foot/hh_foot_vl6_rr1.wav", "Samples/hat_foot/hh_foot_vl6_rr2.wav"]]) },
    RealPad { choke: 1, ..rp("Open Hat", 46, DrumVoice::AcHatOpen, -7.0, "", &[&["Samples/hat_open/hh_open_vl3_rr1.wav", "Samples/hat_open/hh_open_vl3_rr2.wav"], &["Samples/hat_open/hh_open_vl7_rr1.wav", "Samples/hat_open/hh_open_vl7_rr2.wav"], &["Samples/hat_open/hh_open_vl10_rr1.wav", "Samples/hat_open/hh_open_vl10_rr2.wav"], &["Samples/hat_open/hh_open_vl14_rr1.wav", "Samples/hat_open/hh_open_vl14_rr2.wav"]]) },
    rp("Mid Tom 12in", 47, DrumVoice::AcTomMid, -3.0, "", &[&["Samples/tom_xmed/tom_xmed_vl2_rr1.wav", "Samples/tom_xmed/tom_xmed_vl2_rr2.wav"], &["Samples/tom_xmed/tom_xmed_vl5_rr1.wav", "Samples/tom_xmed/tom_xmed_vl5_rr2.wav"], &["Samples/tom_xmed/tom_xmed_vl7_rr1.wav", "Samples/tom_xmed/tom_xmed_vl7_rr2.wav"], &["Samples/tom_xmed/tom_xmed_vl10_rr1.wav", "Samples/tom_xmed/tom_xmed_vl10_rr2.wav"]]),
    rp("Crash", 49, DrumVoice::AcCrash, -6.0, "", &[&["Samples/crash/crash_vl2_rr1.wav", "Samples/crash/crash_vl2_rr2.wav"], &["Samples/crash/crash_vl5_rr1.wav", "Samples/crash/crash_vl5_rr2.wav"], &["Samples/crash/crash_vl9_rr1.wav", "Samples/crash/crash_vl9_rr2.wav"], &["Samples/crash/crash_vl12_rr1.wav", "Samples/crash/crash_vl12_rr2.wav"]]),
    rp("High Tom 10in", 50, DrumVoice::AcTomHigh, -3.0, "", &[&["Samples/tom_mhi/tom_mhi_vl2_rr1.wav", "Samples/tom_mhi/tom_mhi_vl2_rr2.wav"], &["Samples/tom_mhi/tom_mhi_vl5_rr1.wav", "Samples/tom_mhi/tom_mhi_vl5_rr2.wav"], &["Samples/tom_mhi/tom_mhi_vl7_rr1.wav", "Samples/tom_mhi/tom_mhi_vl7_rr2.wav"], &["Samples/tom_mhi/tom_mhi_vl10_rr1.wav", "Samples/tom_mhi/tom_mhi_vl10_rr2.wav"]]),
    rp("Ride", 51, DrumVoice::AcRide, -8.0, "", &[&["Samples/ride/ride_vl3_rr1.wav", "Samples/ride/ride_vl3_rr2.wav"], &["Samples/ride/ride_vl7_rr1.wav", "Samples/ride/ride_vl7_rr2.wav"], &["Samples/ride/ride_vl11_rr1.wav", "Samples/ride/ride_vl11_rr2.wav"], &["Samples/ride/ride_vl15_rr1.wav", "Samples/ride/ride_vl15_rr2.wav"]]),
    rp("China", 52, DrumVoice::AcChina, -7.0, "", &[&["Samples/china/china_vl2_rr1.wav", "Samples/china/china_vl2_rr2.wav"], &["Samples/china/china_vl5_rr1.wav", "Samples/china/china_vl5_rr2.wav"], &["Samples/china/china_vl9_rr1.wav", "Samples/china/china_vl9_rr2.wav"], &["Samples/china/china_vl12_rr1.wav", "Samples/china/china_vl12_rr2.wav"]]),
    rp("Splash", 55, DrumVoice::AcCrash, -7.0, "", &[&["Samples/splash/splash_vl2_rr1.wav", "Samples/splash/splash_vl2_rr2.wav"], &["Samples/splash/splash_vl5_rr1.wav", "Samples/splash/splash_vl5_rr2.wav"], &["Samples/splash/splash_vl9_rr1.wav", "Samples/splash/splash_vl9_rr2.wav"], &["Samples/splash/splash_vl12_rr1.wav", "Samples/splash/splash_vl12_rr2.wav"]]),
    rp("Broken Crash", 57, DrumVoice::AcCrash, -6.0, "", &[&["Samples/crash_broken/crash_vl2_rr1.wav", "Samples/crash_broken/crash_vl2_rr2.wav"], &["Samples/crash_broken/crash_vl5_rr1.wav", "Samples/crash_broken/crash_vl5_rr2.wav"], &["Samples/crash_broken/crash_vl9_rr1.wav", "Samples/crash_broken/crash_vl9_rr2.wav"], &["Samples/crash_broken/crash_vl12_rr1.wav", "Samples/crash_broken/crash_vl12_rr2.wav"]]),
];

/// Rattle Kit.
pub static KIT_RATTLE: &[RealPad] = &[
    rp("Kick 20in", 36, DrumVoice::AcKick, 0.0, "", &[&["Samples/k20/out/rr1/kick_clean_vl2_rr1.flac", "Samples/k20/out/rr2/kick_clean_vl2_rr2.flac"], &["Samples/k20/out/rr1/kick_clean_vl4_rr1.flac", "Samples/k20/out/rr2/kick_clean_vl4_rr2.flac"], &["Samples/k20/out/rr1/kick_clean_vl6_rr1.flac", "Samples/k20/out/rr2/kick_clean_vl6_rr2.flac"], &["Samples/k20/out/rr1/kick_clean_vl8_rr1.flac", "Samples/k20/out/rr2/kick_clean_vl8_rr2.flac"]]),
    rp("Side Stick", 37, DrumVoice::AcStick, -5.0, "", &[&["Samples/s14/top/rr1/s14_sstick_vl1_rr1.flac", "Samples/s14/top/rr2/s14_sstick_vl1_rr2.flac"], &["Samples/s14/top/rr1/s14_sstick_vl3_rr1.flac", "Samples/s14/top/rr2/s14_sstick_vl3_rr2.flac"], &["Samples/s14/top/rr1/s14_sstick_vl4_rr1.flac", "Samples/s14/top/rr2/s14_sstick_vl4_rr2.flac"], &["Samples/s14/top/rr1/s14_sstick_vl6_rr1.flac", "Samples/s14/top/rr2/s14_sstick_vl6_rr2.flac"]]),
    rp("Snare 14in", 38, DrumVoice::AcSnare, -1.0, "", &[&["Samples/s14/top/rr1/s14_center_vl2_rr1.flac", "Samples/s14/top/rr2/s14_center_vl2_rr2.flac"], &["Samples/s14/top/rr1/s14_center_vl4_rr1.flac", "Samples/s14/top/rr2/s14_center_vl4_rr2.flac"], &["Samples/s14/top/rr1/s14_center_vl6_rr1.flac", "Samples/s14/top/rr2/s14_center_vl6_rr2.flac"], &["Samples/s14/top/rr1/s14_center_vl8_rr1.flac", "Samples/s14/top/rr2/s14_center_vl8_rr2.flac"]]),
    rp("Snare 22in", 40, DrumVoice::AcSnare, -2.0, "", &[&["Samples/s22/top/rr1/s22_center_vl2_rr1.flac", "Samples/s22/top/rr2/s22_center_vl2_rr2.flac"], &["Samples/s22/top/rr1/s22_center_vl4_rr1.flac", "Samples/s22/top/rr2/s22_center_vl4_rr2.flac"], &["Samples/s22/top/rr1/s22_center_vl6_rr1.flac", "Samples/s22/top/rr2/s22_center_vl6_rr2.flac"], &["Samples/s22/top/rr1/s22_center_vl8_rr1.flac", "Samples/s22/top/rr2/s22_center_vl8_rr2.flac"]]),
    RealPad { choke: 1, ..rp("Closed Hat", 42, DrumVoice::AcHatClosed, -7.0, "", &[&["Samples/h14/cl/rr1/hh_tight_tip_vl1_rr1.flac", "Samples/h14/cl/rr2/hh_tight_tip_vl1_rr2.flac"], &["Samples/h14/cl/rr1/hh_tight_tip_vl3_rr1.flac", "Samples/h14/cl/rr2/hh_tight_tip_vl3_rr2.flac"], &["Samples/h14/cl/rr1/hh_tight_tip_vl4_rr1.flac", "Samples/h14/cl/rr2/hh_tight_tip_vl4_rr2.flac"], &["Samples/h14/cl/rr1/hh_tight_tip_vl6_rr1.flac", "Samples/h14/cl/rr2/hh_tight_tip_vl6_rr2.flac"]]) },
    rp("Floor Tom 22in", 43, DrumVoice::AcTomFloor, -3.0, "", &[&["Samples/s22/top/rr1/s22_tom_clean_vl2_rr1.flac", "Samples/s22/top/rr2/s22_tom_clean_vl2_rr2.flac"], &["Samples/s22/top/rr1/s22_tom_clean_vl4_rr1.flac", "Samples/s22/top/rr2/s22_tom_clean_vl4_rr2.flac"], &["Samples/s22/top/rr1/s22_tom_clean_vl5_rr1.flac", "Samples/s22/top/rr2/s22_tom_clean_vl5_rr2.flac"], &["Samples/s22/top/rr1/s22_tom_clean_vl7_rr1.flac", "Samples/s22/top/rr2/s22_tom_clean_vl7_rr2.flac"]]),
    RealPad { choke: 1, ..rp("Pedal Hat", 44, DrumVoice::AcHatPedal, -9.0, "", &[&["Samples/h14/cl/rr1/hh_footchik_vl1_rr1.flac", "Samples/h14/cl/rr2/hh_footchik_vl1_rr2.flac"], &["Samples/h14/cl/rr1/hh_footchik_vl3_rr1.flac", "Samples/h14/cl/rr2/hh_footchik_vl3_rr2.flac"], &["Samples/h14/cl/rr1/hh_footchik_vl4_rr1.flac", "Samples/h14/cl/rr2/hh_footchik_vl4_rr2.flac"], &["Samples/h14/cl/rr1/hh_footchik_vl6_rr1.flac", "Samples/h14/cl/rr2/hh_footchik_vl6_rr2.flac"]]) },
    RealPad { choke: 1, ..rp("Open Hat", 46, DrumVoice::AcHatOpen, -7.0, "", &[&["Samples/h14/cl/rr1/hh_open_tip_vl2_rr1.flac", "Samples/h14/cl/rr2/hh_open_tip_vl2_rr2.flac"], &["Samples/h14/cl/rr1/hh_open_tip_vl4_rr1.flac", "Samples/h14/cl/rr2/hh_open_tip_vl4_rr2.flac"], &["Samples/h14/cl/rr1/hh_open_tip_vl5_rr1.flac", "Samples/h14/cl/rr2/hh_open_tip_vl5_rr2.flac"], &["Samples/h14/cl/rr1/hh_open_tip_vl7_rr1.flac", "Samples/h14/cl/rr2/hh_open_tip_vl7_rr2.flac"]]) },
    rp("Mid Tom 14in", 47, DrumVoice::AcTomMid, -3.0, "", &[&["Samples/s14/top/rr1/s14_tom_clean_vl1_rr1.flac", "Samples/s14/top/rr2/s14_tom_clean_vl1_rr2.flac"], &["Samples/s14/top/rr1/s14_tom_clean_vl3_rr1.flac", "Samples/s14/top/rr2/s14_tom_clean_vl3_rr2.flac"], &["Samples/s14/top/rr1/s14_tom_clean_vl4_rr1.flac", "Samples/s14/top/rr2/s14_tom_clean_vl4_rr2.flac"], &["Samples/s14/top/rr1/s14_tom_clean_vl6_rr1.flac", "Samples/s14/top/rr2/s14_tom_clean_vl6_rr2.flac"]]),
    rp("Crash", 49, DrumVoice::AcCrash, -6.0, "", &[&["Samples/c16/oh/rr1/cr_edge_vl1_rr1.flac", "Samples/c16/oh/rr2/cr_edge_vl1_rr2.flac"], &["Samples/c16/oh/rr1/cr_edge_vl2_rr1.flac", "Samples/c16/oh/rr2/cr_edge_vl2_rr2.flac"], &["Samples/c16/oh/rr1/cr_edge_vl4_rr1.flac", "Samples/c16/oh/rr2/cr_edge_vl4_rr2.flac"], &["Samples/c16/oh/rr1/cr_edge_vl5_rr1.flac", "Samples/c16/oh/rr2/cr_edge_vl5_rr2.flac"]]),
    rp("High Tom 13in", 50, DrumVoice::AcTomHigh, -3.0, "", &[&["Samples/s13/top/rr1/s13_tom_clean_vl2_rr1.flac", "Samples/s13/top/rr2/s13_tom_clean_vl2_rr2.flac"], &["Samples/s13/top/rr1/s13_tom_clean_vl4_rr1.flac", "Samples/s13/top/rr2/s13_tom_clean_vl4_rr2.flac"], &["Samples/s13/top/rr1/s13_tom_clean_vl5_rr1.flac", "Samples/s13/top/rr2/s13_tom_clean_vl5_rr2.flac"], &["Samples/s13/top/rr1/s13_tom_clean_vl7_rr1.flac", "Samples/s13/top/rr2/s13_tom_clean_vl7_rr2.flac"]]),
    rp("Ride", 51, DrumVoice::AcRide, -8.0, "", &[&["Samples/r20/oh/rr1/ride_bow_vl2_rr1.flac", "Samples/r20/oh/rr2/ride_bow_vl2_rr2.flac"], &["Samples/r20/oh/rr1/ride_bow_vl4_rr1.flac", "Samples/r20/oh/rr2/ride_bow_vl4_rr2.flac"], &["Samples/r20/oh/rr1/ride_bow_vl6_rr1.flac", "Samples/r20/oh/rr2/ride_bow_vl6_rr2.flac"], &["Samples/r20/oh/rr1/ride_bow_vl8_rr1.flac", "Samples/r20/oh/rr2/ride_bow_vl8_rr2.flac"]]),
    rp("Ride Bell", 53, DrumVoice::AcRideBell, -7.0, "", &[&["Samples/r20/oh/rr1/ride_bell_vl1_rr1.flac", "Samples/r20/oh/rr2/ride_bell_vl1_rr2.flac"], &["Samples/r20/oh/rr1/ride_bell_vl3_rr1.flac", "Samples/r20/oh/rr2/ride_bell_vl3_rr2.flac"], &["Samples/r20/oh/rr1/ride_bell_vl4_rr1.flac", "Samples/r20/oh/rr2/ride_bell_vl4_rr2.flac"], &["Samples/r20/oh/rr1/ride_bell_vl6_rr1.flac", "Samples/r20/oh/rr2/ride_bell_vl6_rr2.flac"]]),
    rp("Ride Crash", 57, DrumVoice::AcCrash, -6.0, "", &[&["Samples/r20/oh/rr1/ride_edge_vl1_rr1.flac", "Samples/r20/oh/rr2/ride_edge_vl1_rr2.flac"], &["Samples/r20/oh/rr1/ride_edge_vl3_rr1.flac", "Samples/r20/oh/rr2/ride_edge_vl3_rr2.flac"], &["Samples/r20/oh/rr1/ride_edge_vl4_rr1.flac", "Samples/r20/oh/rr2/ride_edge_vl4_rr2.flac"], &["Samples/r20/oh/rr1/ride_edge_vl6_rr1.flac", "Samples/r20/oh/rr2/ride_edge_vl6_rr2.flac"]]),
    rp("Woodblock Hi", 76, DrumVoice::AcStick, -4.0, "", &[&["Samples/wb/cl/rr1/woodblock_hi_vl1_rr1.flac", "Samples/wb/cl/rr2/woodblock_hi_vl1_rr2.flac"], &["Samples/wb/cl/rr1/woodblock_hi_vl3_rr1.flac", "Samples/wb/cl/rr2/woodblock_hi_vl3_rr2.flac"], &["Samples/wb/cl/rr1/woodblock_hi_vl4_rr1.flac", "Samples/wb/cl/rr2/woodblock_hi_vl4_rr2.flac"], &["Samples/wb/cl/rr1/woodblock_hi_vl6_rr1.flac", "Samples/wb/cl/rr2/woodblock_hi_vl6_rr2.flac"]]),
    rp("Woodblock Lo", 77, DrumVoice::AcStick, -4.0, "", &[&["Samples/wb/cl/rr1/woodblock_lo_vl1_rr1.flac", "Samples/wb/cl/rr2/woodblock_lo_vl1_rr2.flac"], &["Samples/wb/cl/rr1/woodblock_lo_vl3_rr1.flac", "Samples/wb/cl/rr2/woodblock_lo_vl3_rr2.flac"], &["Samples/wb/cl/rr1/woodblock_lo_vl4_rr1.flac", "Samples/wb/cl/rr2/woodblock_lo_vl4_rr2.flac"], &["Samples/wb/cl/rr1/woodblock_lo_vl6_rr1.flac", "Samples/wb/cl/rr2/woodblock_lo_vl6_rr2.flac"]]),
];

/// Jazz Club Kit.
pub static KIT_JAZZ_CLUB: &[RealPad] = &[
    rp("Kick", 36, DrumVoice::AcKick, 0.0, "", &[&["Samples/kickmic/kick/kickmic_kick_snon_vl1_rr1.flac", "Samples/kickmic/kick/kickmic_kick_snon_vl1_rr2.flac"], &["Samples/kickmic/kick/kickmic_kick_snon_vl2_rr1.flac", "Samples/kickmic/kick/kickmic_kick_snon_vl2_rr2.flac"], &["Samples/kickmic/kick/kickmic_kick_snon_vl3_rr1.flac", "Samples/kickmic/kick/kickmic_kick_snon_vl3_rr2.flac"], &["Samples/kickmic/kick/kickmic_kick_snon_vl4_rr1.flac", "Samples/kickmic/kick/kickmic_kick_snon_vl4_rr2.flac"]]),
    rp("Cross Stick", 37, DrumVoice::AcStick, -5.0, "", &[&["Samples/oh/snare/oh_snare_crossstick_vl3.flac", "Samples/oh/snare/oh_snare_crossstick_vl4.flac"], &["Samples/oh/snare/oh_snare_crossstick_vl7.flac", "Samples/oh/snare/oh_snare_crossstick_vl8.flac"], &["Samples/oh/snare/oh_snare_crossstick_vl11.flac", "Samples/oh/snare/oh_snare_crossstick_vl12.flac"], &["Samples/oh/snare/oh_snare_crossstick_vl15.flac", "Samples/oh/snare/oh_snare_crossstick_vl16.flac"]]),
    rp("Snare", 38, DrumVoice::AcSnare, -1.0, "", &[&["Samples/oh/snare/oh_snare_center_vl6.flac", "Samples/oh/snare/oh_snare_center_vl7.flac"], &["Samples/oh/snare/oh_snare_center_vl16.flac", "Samples/oh/snare/oh_snare_center_vl17.flac"], &["Samples/oh/snare/oh_snare_center_vl25.flac", "Samples/oh/snare/oh_snare_center_vl26.flac"], &["Samples/oh/snare/oh_snare_center_vl35.flac", "Samples/oh/snare/oh_snare_center_vl36.flac"]]),
    rp("Rimshot", 40, DrumVoice::AcSnare, -2.0, "", &[&["Samples/oh/snare/oh_snare_rimshot_vl2.flac", "Samples/oh/snare/oh_snare_rimshot_vl3.flac"], &["Samples/oh/snare/oh_snare_rimshot_vl5.flac", "Samples/oh/snare/oh_snare_rimshot_vl6.flac"], &["Samples/oh/snare/oh_snare_rimshot_vl8.flac", "Samples/oh/snare/oh_snare_rimshot_vl9.flac"], &["Samples/oh/snare/oh_snare_rimshot_vl11.flac", "Samples/oh/snare/oh_snare_rimshot_vl12.flac"]]),
    RealPad { choke: 1, ..rp("Closed Hat", 42, DrumVoice::AcHatClosed, -7.0, "", &[&["Samples/oh/hh/oh_hh_closed_vl1_rr1.flac", "Samples/oh/hh/oh_hh_closed_vl1_rr2.flac"], &["Samples/oh/hh/oh_hh_closed_vl2_rr1.flac", "Samples/oh/hh/oh_hh_closed_vl2_rr2.flac"], &["Samples/oh/hh/oh_hh_closed_vl3_rr1.flac", "Samples/oh/hh/oh_hh_closed_vl3_rr2.flac"], &["Samples/oh/hh/oh_hh_closed_vl4_rr1.flac", "Samples/oh/hh/oh_hh_closed_vl4_rr2.flac"]]) },
    rp("Floor Tom", 43, DrumVoice::AcTomFloor, -3.0, "", &[&["Samples/oh/ltom/oh_ltom_center_vl3.flac", "Samples/oh/ltom/oh_ltom_center_vl4.flac"], &["Samples/oh/ltom/oh_ltom_center_vl7.flac", "Samples/oh/ltom/oh_ltom_center_vl8.flac"], &["Samples/oh/ltom/oh_ltom_center_vl11.flac", "Samples/oh/ltom/oh_ltom_center_vl12.flac"], &["Samples/oh/ltom/oh_ltom_center_vl15.flac", "Samples/oh/ltom/oh_ltom_center_vl16.flac"]]),
    RealPad { choke: 1, ..rp("Pedal Hat", 44, DrumVoice::AcHatPedal, -9.0, "", &[&["Samples/oh/hh/oh_hh_pedal_vl1_rr1.flac", "Samples/oh/hh/oh_hh_pedal_vl1_rr2.flac"], &["Samples/oh/hh/oh_hh_pedal_vl2_rr1.flac", "Samples/oh/hh/oh_hh_pedal_vl2_rr2.flac"], &["Samples/oh/hh/oh_hh_pedal_vl3_rr1.flac", "Samples/oh/hh/oh_hh_pedal_vl3_rr2.flac"]]) },
    RealPad { choke: 1, ..rp("Open Hat", 46, DrumVoice::AcHatOpen, -7.0, "", &[&["Samples/oh/hh/oh_hh_open_vl1_rr1.flac", "Samples/oh/hh/oh_hh_open_vl1_rr2.flac"], &["Samples/oh/hh/oh_hh_open_vl2_rr1.flac", "Samples/oh/hh/oh_hh_open_vl2_rr2.flac"], &["Samples/oh/hh/oh_hh_open_vl3_rr1.flac", "Samples/oh/hh/oh_hh_open_vl3_rr2.flac"], &["Samples/oh/hh/oh_hh_open_vl4_rr1.flac", "Samples/oh/hh/oh_hh_open_vl4_rr2.flac"]]) },
    rp("Crash", 49, DrumVoice::AcCrash, -6.0, "", &[&["Samples/oh/crash/oh_crash_crash_vl1_rr1.flac", "Samples/oh/crash/oh_crash_crash_vl1_rr2.flac"], &["Samples/oh/crash/oh_crash_crash_vl2_rr1.flac", "Samples/oh/crash/oh_crash_crash_vl2_rr2.flac"], &["Samples/oh/crash/oh_crash_crash_vl3_rr1.flac", "Samples/oh/crash/oh_crash_crash_vl3_rr2.flac"]]),
    rp("High Tom", 50, DrumVoice::AcTomHigh, -3.0, "", &[&["Samples/oh/htom/oh_htom_center_vl3.flac", "Samples/oh/htom/oh_htom_center_vl4.flac"], &["Samples/oh/htom/oh_htom_center_vl7.flac", "Samples/oh/htom/oh_htom_center_vl8.flac"], &["Samples/oh/htom/oh_htom_center_vl11.flac", "Samples/oh/htom/oh_htom_center_vl12.flac"], &["Samples/oh/htom/oh_htom_center_vl15.flac", "Samples/oh/htom/oh_htom_center_vl16.flac"]]),
    rp("Ride", 51, DrumVoice::AcRide, -8.0, "", &[&["Samples/oh/ride/oh_ride_ride_vl1_rr1.flac", "Samples/oh/ride/oh_ride_ride_vl1_rr2.flac"], &["Samples/oh/ride/oh_ride_ride_vl2_rr1.flac", "Samples/oh/ride/oh_ride_ride_vl2_rr2.flac"], &["Samples/oh/ride/oh_ride_ride_vl3_rr1.flac", "Samples/oh/ride/oh_ride_ride_vl3_rr2.flac"]]),
    rp("Ride Bell", 53, DrumVoice::AcRideBell, -7.0, "", &[&["Samples/oh/ride/oh_ride_bell_vl1_rr1.flac", "Samples/oh/ride/oh_ride_bell_vl1_rr2.flac"], &["Samples/oh/ride/oh_ride_bell_vl2_rr1.flac", "Samples/oh/ride/oh_ride_bell_vl2_rr2.flac"], &["Samples/oh/ride/oh_ride_bell_vl3_rr1.flac", "Samples/oh/ride/oh_ride_bell_vl3_rr2.flac"]]),
    rp("Tambourine", 54, DrumVoice::Tambourine, -8.0, "", &[&["Samples/perc/oh/tambourine/Tamb1_Shake_rr1_Overhead.wav", "Samples/perc/oh/tambourine/Tamb1_Shake_rr2_Overhead.wav"]]),
    rp("Cowbell", 56, DrumVoice::Cowbell, -9.0, "", &[&["Samples/perc/oh/cowbell/Cowbell1_Normal_v1_rr1_Overhead.wav", "Samples/perc/oh/cowbell/Cowbell1_Normal_v1_rr2_Overhead.wav"], &["Samples/perc/oh/cowbell/Cowbell1_Normal_v2_rr1_Overhead.wav", "Samples/perc/oh/cowbell/Cowbell1_Normal_v2_rr2_Overhead.wav"], &["Samples/perc/oh/cowbell/Cowbell1_Normal_v3_rr1_Overhead.wav", "Samples/perc/oh/cowbell/Cowbell1_Normal_v3_rr2_Overhead.wav"]]),
    rp("Sizzle Crash", 57, DrumVoice::AcCrash, -6.0, "", &[&["Samples/oh/crash/oh_crash_sizzle_vl1_rr1.flac", "Samples/oh/crash/oh_crash_sizzle_vl1_rr2.flac"], &["Samples/oh/crash/oh_crash_sizzle_vl2_rr1.flac", "Samples/oh/crash/oh_crash_sizzle_vl2_rr2.flac"], &["Samples/oh/crash/oh_crash_sizzle_vl3_rr1.flac", "Samples/oh/crash/oh_crash_sizzle_vl3_rr2.flac"]]),
    rp("Flat Ride", 59, DrumVoice::AcRide, -8.0, "", &[&["Samples/oh/flatride/oh_flatride_ride_vl1_rr1.flac", "Samples/oh/flatride/oh_flatride_ride_vl1_rr2.flac"], &["Samples/oh/flatride/oh_flatride_ride_vl2_rr1.flac", "Samples/oh/flatride/oh_flatride_ride_vl2_rr2.flac"], &["Samples/oh/flatride/oh_flatride_ride_vl3_rr1.flac", "Samples/oh/flatride/oh_flatride_ride_vl3_rr2.flac"]]),
];

/// 70s Kit.
pub static KIT_SEVENTIES: &[RealPad] = &[
    rp("Kick", 36, DrumVoice::AcKick, 0.0, "", &[&["Samples/retro_mic/ks_vl1_rr1.wav", "Samples/retro_mic/ks_vl1_rr2.wav"], &["Samples/retro_mic/ks_vl3_rr1.wav", "Samples/retro_mic/ks_vl3_rr2.wav"], &["Samples/retro_mic/ks_vl4_rr1.wav", "Samples/retro_mic/ks_vl4_rr2.wav"], &["Samples/retro_mic/ks_vl6_rr1.wav", "Samples/retro_mic/ks_vl6_rr2.wav"]]),
    rp("Side Stick", 37, DrumVoice::AcStick, -5.0, "", &[&["Samples/retro_mic/ss_vl1_rr1.wav", "Samples/retro_mic/ss_vl1_rr2.wav"], &["Samples/retro_mic/ss_vl2_rr1.wav", "Samples/retro_mic/ss_vl2_rr2.wav"], &["Samples/retro_mic/ss_vl3_rr1.wav", "Samples/retro_mic/ss_vl3_rr2.wav"], &["Samples/retro_mic/ss_vl4_rr1.wav", "Samples/retro_mic/ss_vl4_rr2.wav"]]),
    rp("Snare", 38, DrumVoice::AcSnare, -1.0, "", &[&["Samples/retro_mic/sc_vl1_rr1.wav", "Samples/retro_mic/sc_vl1_rr2.wav"], &["Samples/retro_mic/sc_vl3_rr1.wav", "Samples/retro_mic/sc_vl3_rr2.wav"], &["Samples/retro_mic/sc_vl4_rr1.wav", "Samples/retro_mic/sc_vl4_rr2.wav"], &["Samples/retro_mic/sc_vl6_rr1.wav", "Samples/retro_mic/sc_vl6_rr2.wav"]]),
    rp("Snare Edge", 40, DrumVoice::AcSnare, -2.0, "", &[&["Samples/retro_mic/se_vl1_rr1.wav", "Samples/retro_mic/se_vl1_rr2.wav"], &["Samples/retro_mic/se_vl2_rr1.wav", "Samples/retro_mic/se_vl2_rr2.wav"], &["Samples/retro_mic/se_vl4_rr1.wav", "Samples/retro_mic/se_vl4_rr2.wav"], &["Samples/retro_mic/se_vl5_rr1.wav", "Samples/retro_mic/se_vl5_rr2.wav"]]),
    RealPad { choke: 1, ..rp("Closed Hat", 42, DrumVoice::AcHatClosed, -7.0, "", &[&["Samples/retro_mic/ht_vl1_rr1.wav", "Samples/retro_mic/ht_vl1_rr2.wav"], &["Samples/retro_mic/ht_vl3_rr1.wav", "Samples/retro_mic/ht_vl3_rr2.wav"], &["Samples/retro_mic/ht_vl4_rr1.wav", "Samples/retro_mic/ht_vl4_rr2.wav"], &["Samples/retro_mic/ht_vl6_rr1.wav", "Samples/retro_mic/ht_vl6_rr2.wav"]]) },
    rp("Floor Tom", 43, DrumVoice::AcTomFloor, -3.0, "", &[&["Samples/retro_mic/tl_vl1_rr1.wav", "Samples/retro_mic/tl_vl1_rr2.wav"], &["Samples/retro_mic/tl_vl2_rr1.wav", "Samples/retro_mic/tl_vl2_rr2.wav"], &["Samples/retro_mic/tl_vl4_rr1.wav", "Samples/retro_mic/tl_vl4_rr2.wav"], &["Samples/retro_mic/tl_vl5_rr1.wav", "Samples/retro_mic/tl_vl5_rr2.wav"]]),
    RealPad { choke: 1, ..rp("Pedal Hat", 44, DrumVoice::AcHatPedal, -9.0, "", &[&["Samples/retro_mic/hf_vl1_rr1.wav", "Samples/retro_mic/hf_vl1_rr2.wav"], &["Samples/retro_mic/hf_vl2_rr1.wav", "Samples/retro_mic/hf_vl2_rr2.wav"], &["Samples/retro_mic/hf_vl3_rr1.wav", "Samples/retro_mic/hf_vl3_rr2.wav"]]) },
    RealPad { choke: 1, ..rp("Open Hat", 46, DrumVoice::AcHatOpen, -7.0, "", &[&["Samples/retro_mic/ho_vl1_rr1.wav", "Samples/retro_mic/ho_vl1_rr2.wav"], &["Samples/retro_mic/ho_vl2_rr1.wav", "Samples/retro_mic/ho_vl2_rr2.wav"], &["Samples/retro_mic/ho_vl4_rr1.wav", "Samples/retro_mic/ho_vl4_rr2.wav"], &["Samples/retro_mic/ho_vl5_rr1.wav", "Samples/retro_mic/ho_vl5_rr2.wav"]]) },
    rp("Mid Tom", 47, DrumVoice::AcTomMid, -3.0, "", &[&["Samples/retro_mic/tm_vl1_rr1.wav", "Samples/retro_mic/tm_vl1_rr2.wav"], &["Samples/retro_mic/tm_vl2_rr1.wav", "Samples/retro_mic/tm_vl2_rr2.wav"], &["Samples/retro_mic/tm_vl4_rr1.wav", "Samples/retro_mic/tm_vl4_rr2.wav"], &["Samples/retro_mic/tm_vl5_rr1.wav", "Samples/retro_mic/tm_vl5_rr2.wav"]]),
    rp("High Tom", 50, DrumVoice::AcTomHigh, -3.0, "", &[&["Samples/retro_mic/th_vl1_rr1.wav", "Samples/retro_mic/th_vl1_rr2.wav"], &["Samples/retro_mic/th_vl2_rr1.wav", "Samples/retro_mic/th_vl2_rr2.wav"], &["Samples/retro_mic/th_vl4_rr1.wav", "Samples/retro_mic/th_vl4_rr2.wav"], &["Samples/retro_mic/th_vl5_rr1.wav", "Samples/retro_mic/th_vl5_rr2.wav"]]),
];

/// Sonor Rock Kit.
pub static KIT_SONOR: &[RealPad] = &[
    rp("Kick", 36, DrumVoice::AcKick, 0.0, "Samples", &[&["KickSamples.flac", "KickSamples#1.flac"], &["KickSamples#2.flac", "KickSamples#3.flac"], &["KickSamples#4.flac", "KickSamples#5.flac"], &["KickSamples#6.flac", "KickSamples#7.flac"]]),
    rp("Snare", 38, DrumVoice::AcSnare, -1.0, "Samples", &[&["SnareLeftHand.flac", "SnareRightHand.flac"], &["SnareLeftHand#3.flac", "SnareRightHand#3.flac"], &["SnareLeftHand#6.flac", "SnareRightHand#7.flac"], &["SnareLeftHand#9.flac", "SnareRightHand#8.flac"]]),
    rp("Rimshot", 40, DrumVoice::AcSnare, -2.0, "Samples", &[&["SnareSamples#9.flac"]]),
    RealPad { choke: 1, ..rp("Closed Hat", 42, DrumVoice::AcHatClosed, -7.0, "Samples", &[&["HHClosed#1.flac", "HHClosed.flac"], &["HHClosed#3.flac", "HHClosed#2.flac"], &["HHClosed#7.flac", "HHClosed#5.flac"], &["HHClosed#6.flac", "HHClosed#8.flac"]]) },
    rp("Floor Tom", 43, DrumVoice::AcTomFloor, -3.0, "Samples", &[&["TomLow.flac"], &["TomLow#2.flac"], &["TomLow#4.flac"], &["TomLow#6.flac"]]),
    RealPad { choke: 1, ..rp("Pedal Hat", 44, DrumVoice::AcHatPedal, -9.0, "Samples", &[&["HHPedal.flac"], &["HHPedal#2.flac"], &["HHPedal#3.flac"], &["HHPedal#5.flac"]]) },
    RealPad { choke: 1, ..rp("Open Hat", 46, DrumVoice::AcHatOpen, -7.0, "Samples", &[&["HHOpenClang#3.flac"], &["HHOpenClang#4.flac"], &["HHOpenClang.flac"], &["HHOpenClang#1.flac"]]) },
    rp("Crash", 49, DrumVoice::AcCrash, -6.0, "Samples", &[&["CrashClang.flac"], &["CrashClang#1.flac"], &["CrashClang#3.flac"], &["CrashClang#4.flac"]]),
    rp("High Tom", 50, DrumVoice::AcTomHigh, -3.0, "Samples", &[&["TomHigh.flac"], &["TomHigh#1.flac"], &["TomHigh#3.flac"], &["TomHigh#4.flac"]]),
    rp("Ride", 51, DrumVoice::AcRide, -8.0, "Samples", &[&["Ride.flac"], &["Ride#1.flac"], &["Ride#3.flac"], &["Ride#4.flac"]]),
    rp("Ride Bell", 53, DrumVoice::AcRideBell, -7.0, "Samples", &[&["RideBell.flac"], &["RideBell#1.flac"], &["RideBell#2.flac"], &["RideBell#3.flac"]]),
    rp("Crash 2", 57, DrumVoice::AcCrash, -6.0, "Samples", &[&["CrashTip.flac"], &["CrashTip#1.flac"], &["CrashTip#2.flac"], &["CrashTip#3.flac"]]),
];

/// Naked Studio Kit.
pub static KIT_NAKED: &[RealPad] = &[
    rp("Kick", 36, DrumVoice::AcKick, 0.0, "Wilkinson Audio/Naked Drums/Samples", &[&["-Bottom_Black_D2_20_one-II9X.flac", "-Bottom_Black_D2_20_two-N3VW.flac"], &["-Bottom_Black_D2_30_one-NLNF.flac", "-Bottom_Black_D2_30_two-VYTH.flac"], &["-Bottom_Black_D2_40_one-HLYB.flac", "-Bottom_Black_D2_40_two-G626.flac"], &["-Bottom_Black_D2_50_one-V4WI.flac", "-Bottom_Black_D2_50_two-LN2C.flac"]]),
    rp("Side Stick", 37, DrumVoice::AcStick, -5.0, "Wilkinson Audio/Naked Drums/Samples", &[&["OH_Black_D2_50_one-GMS9.flac", "OH_Black_D2_50_two-D5FN.flac"]]),
    rp("Snare", 38, DrumVoice::AcSnare, -1.0, "Wilkinson Audio/Naked Drums/Samples", &[&["-OH_Black_D2_10_one-UD4F.flac", "-OH_Black_D2_10_two-D1EG.flac"], &["-OH_Black_D2_20_one-ZUBL.flac", "-OH_Black_D2_20_two-1NDI.flac"], &["-OH_Black_D2_40_one-HF7Z.flac", "-OH_Black_D2_40_two-QHJY.flac"], &["-OH_Black_D2_50_one-QGEQ.flac", "-OH_Black_D2_50_two-J3TL.flac"]]),
    rp("Snare 2", 40, DrumVoice::AcSnare, -2.0, "Wilkinson Audio/Naked Drums/Samples", &[&["-OH_Black_D2_10_one-6TJD.flac", "-OH_Black_D2_10_two-EU4B.flac"], &["-OH_Black_D2_20_one-CEG2.flac", "-OH_Black_D2_20_two-DBU0.flac"], &["-OH_Black_D2_40_one-18HN.flac", "-OH_Black_D2_40_two-4FRE.flac"], &["-OH_Black_D2_50_one-8VZB.flac", "-OH_Black_D2_50_two-PJ9B.flac"]]),
    RealPad { choke: 1, ..rp("Closed Hat", 42, DrumVoice::AcHatClosed, -7.0, "Wilkinson Audio/Naked Drums/Samples", &[&["OH_A2_10_one-CWG6.flac", "OH_A2_10_two-X129.flac"], &["OH_A2_20_one-4I2D.flac", "OH_A2_20_two-88YK.flac"], &["OH_A2_30_one-PF2U.flac", "OH_A2_30_two-LX20.flac"]]) },
    rp("Floor Tom", 43, DrumVoice::AcTomFloor, -3.0, "Wilkinson Audio/Naked Drums/Samples", &[&["-OH_D3_10_one-38X2.flac", "-OH_D3_10_two-03VR.flac"], &["-OH_D3_20_one-1KRF.flac", "-OH_D3_20_two-MTCO.flac"], &["-OH_D3_30_one-ZWA5.flac", "-OH_D3_30_two-7Q5A.flac"]]),
    RealPad { choke: 1, ..rp("Pedal Hat", 44, DrumVoice::AcHatPedal, -9.0, "Wilkinson Audio/Naked Drums/Samples", &[&["OH_E2_10_one-TCLW.flac", "OH_E2_10_two-8NXR.flac"]]) },
    RealPad { choke: 1, ..rp("Open Hat", 46, DrumVoice::AcHatOpen, -7.0, "Wilkinson Audio/Naked Drums/Samples", &[&["OH_F2_10_one-UFYV.flac", "OH_F2_10_two-1Y6T.flac"], &["OH_F2_20_one-GZYD.flac", "OH_F2_20_two-J722.flac"], &["OH_F2_30_one-KL3V.flac", "OH_F2_30_two-JQDP.flac"]]) },
    rp("Mid Tom", 47, DrumVoice::AcTomMid, -3.0, "Wilkinson Audio/Naked Drums/Samples", &[&["-OH_D3_10_one-C75T.flac", "-OH_D3_10_two-FS99.flac"], &["-OH_D3_20_one-6XRU.flac", "-OH_D3_20_two-Q5N1.flac"], &["-OH_D3_30_one-G8NW.flac", "-OH_D3_30_two-SU67.flac"]]),
    rp("Crash", 49, DrumVoice::AcCrash, -6.0, "Wilkinson Audio/Naked Drums/Samples", &[&["OH_D3_10_one-H2DR.flac", "OH_D3_10_two-AMP1.flac"], &["OH_D3_20_one-CX1I.flac", "OH_D3_20_two-6MEQ.flac"], &["OH_D3_30_one-W5TF.flac", "OH_D3_30_two-5R9X.flac"]]),
    rp("High Tom", 50, DrumVoice::AcTomHigh, -3.0, "Wilkinson Audio/Naked Drums/Samples", &[&["-OH_D3_10_one-K58E.flac", "-OH_D3_10_two-13GW.flac"], &["-OH_D3_20_one-SCPM.flac", "-OH_D3_20_two-L6OO.flac"], &["-OH_D3_30_one-E1OL.flac", "-OH_D3_30_two-QZVT.flac"]]),
    rp("Ride", 51, DrumVoice::AcRide, -8.0, "Wilkinson Audio/Naked Drums/Samples", &[&["OH_D3_10_one-KXMA.flac", "OH_D3_10_two-76VA.flac"], &["OH_D3_20_one-NF8L.flac", "OH_D3_20_two-XGZR.flac"], &["OH_D3_30_one-2HUY.flac", "OH_D3_30_two-LZL0.flac"]]),
    rp("China", 52, DrumVoice::AcChina, -7.0, "Wilkinson Audio/Naked Drums/Samples", &[&["OH_D3_10_one-KI3W.flac", "OH_D3_10_two-MPG9.flac"], &["OH_D3_20_one-ZI80.flac", "OH_D3_20_two-Y466.flac"], &["OH_D3_30_one-DYXB.flac", "OH_D3_30_two-XBCZ.flac"]]),
    rp("Ride Bell", 53, DrumVoice::AcRideBell, -7.0, "Wilkinson Audio/Naked Drums/Samples", &[&["OH_E3_10_one-DJ89.flac", "OH_E3_10_two-W99L.flac"]]),
    rp("Crash 2", 57, DrumVoice::AcCrash, -6.0, "Wilkinson Audio/Naked Drums/Samples", &[&["OH_D3_10_one-6447.flac", "OH_D3_10_two-HPJV.flac"], &["OH_D3_20_one-T1YU.flac", "OH_D3_20_two-NLZQ.flac"], &["OH_D3_30_one-2TAZ.flac", "OH_D3_30_two-VL4B.flac"]]),
];

/// Metal Kit.
pub static KIT_METAL: &[RealPad] = &[
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-KdrumL.flac", 0.0, 0.0)], ..rp("Kick 2", 35, DrumVoice::AcKick, 0.0, "DrumGizmo/MuldjordKit/Samples/KdrumL", &[&["2-KdrumL", "3-KdrumL"], &["8-KdrumL", "9-KdrumL"], &["13-KdrumL", "14-KdrumL"], &["24-KdrumL", "25-KdrumL"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-KdrumR.flac", 0.0, 0.0)], ..rp("Kick", 36, DrumVoice::AcKick, 0.0, "DrumGizmo/MuldjordKit/Samples/KdrumR", &[&["2-KdrumR", "3-KdrumR"], &["6-KdrumR", "7-KdrumR"], &["13-KdrumR", "14-KdrumR"], &["26-KdrumR", "27-KdrumR"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-Snare_top.flac", 0.0, 0.0), ("-Snare_bottom.flac", 0.0, -6.0)], ..rp("Side Stick", 37, DrumVoice::AcStick, -5.0, "DrumGizmo/MuldjordKit/Samples/SnareRest", &[&["1-SnareRest"], &["2-SnareRest", "3-SnareRest"], &["4-SnareRest", "5-SnareRest"], &["6-SnareRest", "7-SnareRest"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-Snare_top.flac", 0.0, 0.0), ("-Snare_bottom.flac", 0.0, -6.0)], ..rp("Snare", 38, DrumVoice::AcSnare, -1.0, "DrumGizmo/MuldjordKit/Samples/Snare", &[&["1-Snare", "2-Snare"], &["14-Snare", "15-Snare"], &["41-Snare", "42-Snare"], &["55-Snare", "56-Snare"]]) },
    RealPad { choke: 1, mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-Hihat.flac", -0.4, 0.0)], ..rp("Closed Hat", 42, DrumVoice::AcHatClosed, -7.0, "DrumGizmo/MuldjordKit/Samples/HihatClosed", &[&["1-HihatClosed", "2-HihatClosed"], &["7-HihatClosed", "8-HihatClosed"], &["16-HihatClosed", "17-HihatClosed"], &["21-HihatClosed", "22-HihatClosed"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-Tom4.flac", 0.6, 0.0)], ..rp("Floor Tom", 43, DrumVoice::AcTomFloor, -3.0, "DrumGizmo/MuldjordKit/Samples/Tom4", &[&["1-Tom4", "2-Tom4"], &["3-Tom4", "4-Tom4"], &["6-Tom4", "7-Tom4"], &["10-Tom4", "11-Tom4"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-Tom3.flac", 0.2, 0.0)], ..rp("Low Tom", 45, DrumVoice::AcTomMid, -3.0, "DrumGizmo/MuldjordKit/Samples/Tom3", &[&["1-Tom3", "2-Tom3"], &["3-Tom3", "4-Tom3"], &["5-Tom3", "6-Tom3"], &["9-Tom3", "10-Tom3"]]) },
    RealPad { choke: 1, mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-Hihat.flac", -0.4, 0.0)], ..rp("Open Hat", 46, DrumVoice::AcHatOpen, -7.0, "DrumGizmo/MuldjordKit/Samples/HihatOpen", &[&["2-HihatOpen", "3-HihatOpen"], &["5-HihatOpen", "6-HihatOpen"], &["11-HihatOpen", "12-HihatOpen"], &["17-HihatOpen", "18-HihatOpen"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-Tom2.flac", -0.2, 0.0)], ..rp("Mid Tom", 47, DrumVoice::AcTomMid, -3.0, "DrumGizmo/MuldjordKit/Samples/Tom2", &[&["1-Tom2"], &["2-Tom2", "3-Tom2"], &["5-Tom2", "6-Tom2"], &["9-Tom2", "10-Tom2"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0)], ..rp("Crash", 49, DrumVoice::AcCrash, -6.0, "DrumGizmo/MuldjordKit/Samples/CrashL", &[&["1-CrashL", "2-CrashL"], &["3-CrashL", "4-CrashL"], &["5-CrashL", "6-CrashL"], &["8-CrashL", "9-CrashL"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-Tom1.flac", -0.6, 0.0)], ..rp("High Tom", 50, DrumVoice::AcTomHigh, -3.0, "DrumGizmo/MuldjordKit/Samples/Tom1", &[&["1-Tom1", "2-Tom1"], &["3-Tom1", "4-Tom1"], &["5-Tom1", "6-Tom1"], &["8-Tom1", "9-Tom1"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-RideR.flac", 0.5, 0.0)], ..rp("Ride", 51, DrumVoice::AcRide, -8.0, "DrumGizmo/MuldjordKit/Samples/RideR", &[&["1-RideR"], &["2-RideR", "3-RideR"], &["5-RideR", "6-RideR"], &["8-RideR", "9-RideR"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0)], ..rp("China", 52, DrumVoice::AcChina, -7.0, "DrumGizmo/MuldjordKit/Samples/China", &[&["1-China", "2-China"], &["3-China", "4-China"], &["5-China", "6-China"], &["9-China", "10-China"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-RideR.flac", 0.5, 0.0)], ..rp("Ride Bell", 53, DrumVoice::AcRideBell, -7.0, "DrumGizmo/MuldjordKit/Samples/RideRBell", &[&["1-RideRBell", "2-RideRBell"], &["3-RideRBell", "4-RideRBell"], &["5-RideRBell", "6-RideRBell"], &["7-RideRBell", "8-RideRBell"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0)], ..rp("Crash 2", 57, DrumVoice::AcCrash, -6.0, "DrumGizmo/MuldjordKit/Samples/CrashR", &[&["1-CrashR", "2-CrashR"], &["3-CrashR", "4-CrashR"], &["6-CrashR", "7-CrashR"], &["9-CrashR", "10-CrashR"]]) },
];

/// Airy Kit.
pub static KIT_AIRY: &[RealPad] = &[
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-AmbL.flac", -1.0, 0.0), ("-AmbR.flac", 1.0, 0.0), ("-Kdrum_back.flac", 0.0, 0.0), ("-Kdrum_front.flac", 0.0, 0.0)], ..rp("Kick", 36, DrumVoice::AcKick, 0.0, "DrumGizmo/DRSKit/Samples/Kdrum_with_contact", &[&["2-Kdrum_with_contact", "3-Kdrum_with_contact"], &["8-Kdrum_with_contact", "9-Kdrum_with_contact"], &["12-Kdrum_with_contact", "13-Kdrum_with_contact"], &["21-Kdrum_with_contact", "22-Kdrum_with_contact"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-AmbL.flac", -1.0, 0.0), ("-AmbR.flac", 1.0, 0.0), ("-Snare_top.flac", 0.0, 0.0), ("-Snare_bottom.flac", 0.0, -6.0)], ..rp("Side Stick", 37, DrumVoice::AcStick, -5.0, "DrumGizmo/DRSKit/Samples/Snare_rest", &[&["1-Snare_rest", "2-Snare_rest"], &["3-Snare_rest", "4-Snare_rest"], &["5-Snare_rest", "6-Snare_rest"], &["8-Snare_rest", "9-Snare_rest"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-AmbL.flac", -1.0, 0.0), ("-AmbR.flac", 1.0, 0.0), ("-Snare_top.flac", 0.0, 0.0), ("-Snare_bottom.flac", 0.0, -6.0)], ..rp("Snare", 38, DrumVoice::AcSnare, -1.0, "DrumGizmo/DRSKit/Samples/Snare", &[&["1-Snare", "2-Snare"], &["17-Snare", "18-Snare"], &["32-Snare", "33-Snare"], &["48-Snare", "49-Snare"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-AmbL.flac", -1.0, 0.0), ("-AmbR.flac", 1.0, 0.0), ("-Snare_top.flac", 0.0, 0.0), ("-Snare_bottom.flac", 0.0, -6.0)], ..rp("Brush Snare", 40, DrumVoice::AcBrush, -2.0, "DrumGizmo/DRSKit/Samples/Snare_whisker", &[&["1-Snare_whisker", "2-Snare_whisker"], &["6-Snare_whisker", "7-Snare_whisker"], &["13-Snare_whisker", "14-Snare_whisker"], &["15-Snare_whisker"]]) },
    RealPad { choke: 1, mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-AmbL.flac", -1.0, 0.0), ("-AmbR.flac", 1.0, 0.0), ("-Hihat.flac", -0.4, -14.0)], ..rp("Closed Hat", 42, DrumVoice::AcHatClosed, -7.0, "DrumGizmo/DRSKit/Samples/Hihat_closed", &[&["1-Hihat_closed", "2-Hihat_closed"], &["7-Hihat_closed", "8-Hihat_closed"], &["10-Hihat_closed", "11-Hihat_closed"], &["17-Hihat_closed", "18-Hihat_closed"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-AmbL.flac", -1.0, 0.0), ("-AmbR.flac", 1.0, 0.0), ("-Tom3.flac", 0.6, 0.0)], ..rp("Floor Tom", 43, DrumVoice::AcTomFloor, -3.0, "DrumGizmo/DRSKit/Samples/Tom3", &[&["1-Tom3", "2-Tom3"], &["7-Tom3", "8-Tom3"], &["10-Tom3", "11-Tom3"], &["17-Tom3", "18-Tom3"]]) },
    RealPad { choke: 1, mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-AmbL.flac", -1.0, 0.0), ("-AmbR.flac", 1.0, 0.0), ("-Hihat.flac", -0.4, -14.0)], ..rp("Pedal Hat", 44, DrumVoice::AcHatPedal, -9.0, "DrumGizmo/DRSKit/Samples/Hihat_foot", &[&["1-Hihat_foot", "2-Hihat_foot"], &["3-Hihat_foot"], &["6-Hihat_foot", "7-Hihat_foot"], &["8-Hihat_foot"]]) },
    RealPad { choke: 1, mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-AmbL.flac", -1.0, 0.0), ("-AmbR.flac", 1.0, 0.0), ("-Hihat.flac", -0.4, -14.0)], ..rp("Open Hat", 46, DrumVoice::AcHatOpen, -7.0, "DrumGizmo/DRSKit/Samples/Hihat_open", &[&["1-Hihat_open", "2-Hihat_open"], &["3-Hihat_open", "4-Hihat_open"], &["9-Hihat_open", "10-Hihat_open"], &["13-Hihat_open", "14-Hihat_open"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-AmbL.flac", -1.0, 0.0), ("-AmbR.flac", 1.0, 0.0), ("-Tom2.flac", 0.2, 0.0)], ..rp("Mid Tom", 47, DrumVoice::AcTomMid, -3.0, "DrumGizmo/DRSKit/Samples/Tom2", &[&["1-Tom2", "2-Tom2"], &["7-Tom2", "8-Tom2"], &["10-Tom2", "11-Tom2"], &["17-Tom2", "18-Tom2"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-AmbL.flac", -1.0, 0.0), ("-AmbR.flac", 1.0, 0.0)], ..rp("Crash", 49, DrumVoice::AcCrash, -6.0, "DrumGizmo/DRSKit/Samples/Crash_left_shank", &[&["1-Crash_left_shank", "2-Crash_left_shank"], &["4-Crash_left_shank", "5-Crash_left_shank"], &["7-Crash_left_shank", "8-Crash_left_shank"], &["10-Crash_left_shank", "11-Crash_left_shank"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-AmbL.flac", -1.0, 0.0), ("-AmbR.flac", 1.0, 0.0), ("-Tom1.flac", -0.2, 0.0)], ..rp("High Tom", 50, DrumVoice::AcTomHigh, -3.0, "DrumGizmo/DRSKit/Samples/Tom1", &[&["1-Tom1", "2-Tom1"], &["10-Tom1", "11-Tom1"], &["14-Tom1", "15-Tom1"], &["23-Tom1", "24-Tom1"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-AmbL.flac", -1.0, 0.0), ("-AmbR.flac", 1.0, 0.0), ("-Ride.flac", 0.4, -14.0)], ..rp("Ride", 51, DrumVoice::AcRide, -8.0, "DrumGizmo/DRSKit/Samples/Ride_tip", &[&["1-Ride_tip"], &["2-Ride_tip", "3-Ride_tip"], &["6-Ride_tip", "7-Ride_tip"], &["10-Ride_tip", "11-Ride_tip"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-AmbL.flac", -1.0, 0.0), ("-AmbR.flac", 1.0, 0.0), ("-Ride.flac", 0.4, -14.0)], ..rp("Ride Bell", 53, DrumVoice::AcRideBell, -7.0, "DrumGizmo/DRSKit/Samples/Ride_shank_bell", &[&["1-Ride_shank_bell", "2-Ride_shank_bell"], &["3-Ride_shank_bell", "4-Ride_shank_bell"], &["7-Ride_shank_bell", "8-Ride_shank_bell"], &["9-Ride_shank_bell", "10-Ride_shank_bell"]]) },
    RealPad { mics: &[("-OHL.flac", -1.0, 0.0), ("-OHR.flac", 1.0, 0.0), ("-AmbL.flac", -1.0, 0.0), ("-AmbR.flac", 1.0, 0.0)], ..rp("Crash 2", 57, DrumVoice::AcCrash, -6.0, "DrumGizmo/DRSKit/Samples/Crash_right_shank", &[&["1-Crash_right_shank", "2-Crash_right_shank"], &["3-Crash_right_shank", "4-Crash_right_shank"], &["7-Crash_right_shank", "8-Crash_right_shank"], &["9-Crash_right_shank", "10-Crash_right_shank"]]) },
];

/// Body Percussion.
pub static KIT_BODY: &[RealPad] = &[
    rp("Half Stomp", 35, DrumVoice::AcKick, 0.0, "Samples/body", &[&["halfstomp_r_vl1_rr1.wav", "halfstomp_r_vl1_rr2.wav"], &["halfstomp_r_vl2_rr1.wav", "halfstomp_r_vl2_rr2.wav"]]),
    rp("Stomp", 36, DrumVoice::AcKick, 0.0, "Samples/body", &[&["stomp_r_vl1_rr1.wav", "stomp_r_vl1_rr2.wav"], &["stomp_r_vl2_rr1.wav", "stomp_r_vl2_rr2.wav"]]),
    rp("Snap", 37, DrumVoice::AcStick, -3.0, "Samples/body", &[&["snap_l_rr1.wav", "snap_l_rr2.wav"]]),
    rp("Chest", 38, DrumVoice::AcSnare, -3.0, "Samples/body", &[&["chestside_r_vl1_rr1.wav", "chestside_r_vl1_rr2.wav"], &["chestside_r_vl2_rr1.wav", "chestside_r_vl2_rr2.wav"]]),
    rp("Clap", 39, DrumVoice::Clap, -3.0, "Samples/body", &[&["handclap_vl1_rr1.wav", "handclap_vl1_rr2.wav"], &["handclap_vl2_rr1.wav", "handclap_vl2_rr2.wav"]]),
    rp("Finger Clap", 40, DrumVoice::Clap, -3.0, "Samples/body", &[&["fingerclap_vl1_rr1.wav", "fingerclap_vl1_rr2.wav"], &["fingerclap_vl2_rr1.wav", "fingerclap_vl2_rr2.wav"]]),
    rp("Heel", 41, DrumVoice::AcTomFloor, -3.0, "Samples/body", &[&["heel_r_vl1_rr1.wav", "heel_r_vl1_rr2.wav"], &["heel_r_vl2_rr1.wav", "heel_r_vl2_rr2.wav"]]),
    rp("Snap 2", 42, DrumVoice::AcHatClosed, -3.0, "Samples/body", &[&["snap_r_rr1.wav", "snap_r_rr2.wav"]]),
    rp("Belly", 43, DrumVoice::AcTomFloor, -3.0, "Samples/body", &[&["bellycenter_r_vl1_rr1.wav", "bellycenter_r_vl1_rr2.wav"], &["bellycenter_r_vl2_rr1.wav", "bellycenter_r_vl2_rr2.wav"]]),
    rp("Pinky Snap", 44, DrumVoice::AcHatPedal, -3.0, "Samples/body", &[&["pinkysnap_r_rr1.wav", "pinkysnap_r_rr2.wav"]]),
    rp("Belly Side", 45, DrumVoice::AcTomMid, -3.0, "Samples/body", &[&["bellyside_r_vl1_rr1.wav", "bellyside_r_vl1_rr2.wav"], &["bellyside_r_vl2_rr1.wav", "bellyside_r_vl2_rr2.wav"]]),
    rp("Chest Thump", 47, DrumVoice::AcTomMid, -3.0, "Samples/body", &[&["chestthump_l_vl1_rr1.wav", "chestthump_l_vl1_rr2.wav"], &["chestthump_l_vl2_rr1.wav", "chestthump_l_vl2_rr2.wav"]]),
    rp("Shoulder", 48, DrumVoice::AcTomHigh, -3.0, "Samples/body", &[&["shoulderout_r_vl1_rr1.wav", "shoulderout_r_vl1_rr2.wav"], &["shoulderout_r_vl2_rr1.wav", "shoulderout_r_vl2_rr2.wav"]]),
    rp("Head", 50, DrumVoice::AcTomHigh, -3.0, "Samples/body", &[&["head_vl1_rr1.wav", "head_vl1_rr2.wav"], &["head_vl2_rr1.wav", "head_vl2_rr2.wav"]]),
    rp("Cheek Pop", 56, DrumVoice::Cowbell, -3.0, "Samples/body", &[&["cheek_rout_rr1.wav", "cheek_rout_rr2.wav"]]),
];

/// A downloadable set of kits from one source.
pub struct SamplePack {
    /// Its folder in the Library.
    pub folder: &'static str,
    /// Where the files are (ends with '/').
    pub base: &'static str,
    /// Who made the recordings, and under which license.
    pub credit: &'static str,
    /// About how big the download is.
    pub mb: u32,
    pub kits: &'static [(&'static str, &'static [RealPad])],
    /// Bring every pad's loudest hit to the same level (the recordings were
    /// not levelled, so the pads' levels are set for levelled hits).
    pub normalize: bool,
}

const fn karoryfer(folder: &'static str, base: &'static str, credit: &'static str, mb: u32, kits: &'static [(&'static str, &'static [RealPad])]) -> SamplePack {
    SamplePack { folder, base, credit, mb, kits, normalize: true }
}

pub static PACKS: &[SamplePack] = &[
    SamplePack { folder: FOLDER, base: BASE_URL, credit: CREDIT, mb: 62, kits: &[("Real Kit", REAL_KIT), ("Real Percussion", REAL_PERC)], normalize: false },
    SamplePack {
        folder: "707 Samples (Francois Dion)",
        base: OPEN_DRUMS,
        credit: "TR-707 samples by Francois Dion, public domain (\"All samples are public domain, so use em however ya want\"), via fluid-music/open-drums: https://github.com/fluid-music/open-drums",
        mb: 1,
        kits: &[("707 Samples", KIT_707)],
        normalize: false,
    },
    SamplePack {
        folder: "808 Samples (Michael Fischer)",
        base: FISCHER,
        credit: "TR-808 samples by Michael Fischer / Technopolis (1994, \"absolutely free\"), CC0 via TidalCycles: https://github.com/tidalcycles/sounds-tr808-fischer",
        mb: 2,
        kits: &[("808 Samples", KIT_808)],
        normalize: false,
    },
    SamplePack {
        folder: "Sonic Pi Samples",
        base: SONIC_PI,
        credit: "Samples from Sonic Pi, CC0 (from freesound.org, and donated by The Black Dog): https://github.com/sonic-pi-net/sonic-pi/tree/dev/etc/samples",
        mb: 4,
        kits: &[("Electro Kit", KIT_ELECTRO), ("Club Kit", KIT_CLUB), ("Glitch Kit", KIT_GLITCH), ("Tabla", KIT_TABLA), ("Dry Kit", KIT_DRY)],
        normalize: false,
    },
    karoryfer(
        "Rusty Kit (Karoryfer)",
        "https://raw.githubusercontent.com/sfzinstruments/karoryfer.big-rusty-drums/f07ce00df34a46b6b08375be56fe116cf15782bc/",
        "Big Rusty Drums by Karoryfer Samples, CC0 1.0: https://github.com/sfzinstruments/karoryfer.big-rusty-drums",
        32,
        &[("Rusty Kit", KIT_RUSTY)],
    ),
    karoryfer(
        "Lounge Kit (Karoryfer)",
        "https://raw.githubusercontent.com/sfzinstruments/karoryfer.swirly-drums/c40dafe0011cb2e54c0c220ff0fa308a11fc60f5/",
        "Swirly Drums by Karoryfer Samples, CC0 1.0: https://github.com/sfzinstruments/karoryfer.swirly-drums",
        35,
        &[("Lounge Kit", KIT_SWIRLY)],
    ),
    karoryfer(
        "Garage Kit (Karoryfer)",
        "https://raw.githubusercontent.com/sfzinstruments/karoryfer.unruly-drums/9bf75c2a1392f190cd1c264645653629f0b3a097/",
        "Unruly Drums by Karoryfer Samples, CC0 1.0: https://github.com/sfzinstruments/karoryfer.unruly-drums",
        26,
        &[("Garage Kit", KIT_RATTLE)],
    ),
    karoryfer(
        "Jazz Club Kit (Virtuosity)",
        "https://raw.githubusercontent.com/sfzinstruments/virtuosity_drums/9f04cf9a734527edfbb0a4eee1f674e45bbf71bc/",
        "Virtuosity Drums: Austin McMahon's kit at Virtuosity Musical Instruments, sampled by Versilian Studios and mapped by Karoryfer Samples, CC0 1.0: https://github.com/sfzinstruments/virtuosity_drums",
        57,
        &[("Jazz Club Kit", KIT_JAZZ_CLUB)],
    ),
    karoryfer(
        "70s Kit (Karoryfer)",
        "https://raw.githubusercontent.com/sfzinstruments/karoryfer.gogodze-phu-vol-ii/69a0274cdc39c99c3098cde1bc3789690de86d62/",
        "Gogodze Phu Vol. II by Karoryfer Samples, CC0 1.0: https://github.com/sfzinstruments/karoryfer.gogodze-phu-vol-ii",
        27,
        &[("70s Kit", KIT_SEVENTIES)],
    ),
    karoryfer(
        "Classic Rock Kit (Sam Greene)",
        "https://raw.githubusercontent.com/sfzinstruments/SamsSonor/ef3e32058924ed1c3335e86d03094d3f310a9104/",
        "A drum kit recorded by Sam Greene, SFZ edition by sfzinstruments, CC BY-SA 4.0 (https://creativecommons.org/licenses/by-sa/4.0/): https://github.com/sfzinstruments/SamsSonor",
        24,
        &[("Classic Rock Kit", KIT_SONOR)],
    ),
    karoryfer(
        "Naked Kit (Wilkinson Audio)",
        "https://raw.githubusercontent.com/sfzinstruments/WilkinsonAudio.NakedDrums/407732a548606ac715ad8f3fbac63d9f4df77c0d/",
        "Naked Drums by Wilkinson Audio, CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/): https://github.com/sfzinstruments/WilkinsonAudio.NakedDrums",
        36,
        &[("Naked Kit", KIT_NAKED)],
    ),
    karoryfer(
        "Metal Kit (DrumGizmo)",
        "https://raw.githubusercontent.com/sfzinstruments/DrumGizmo.MuldjordKit/fc165714974aa843125ce88597f341b31376e1d5/",
        "MuldjordKit from DrumGizmo (drumgizmo.org), CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/); the microphones mixed to stereo by Krabton: https://github.com/sfzinstruments/DrumGizmo.MuldjordKit",
        40,
        &[("Metal Kit", KIT_METAL)],
    ),
    karoryfer(
        "Room Kit (DrumGizmo)",
        "https://raw.githubusercontent.com/sfzinstruments/DrumGizmo.DRSKit/b01448990a31f722542f1812e56159a97a6a2a82/",
        "DRSKit from DrumGizmo (drumgizmo.org), CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/); the microphones mixed to stereo by Krabton: https://github.com/sfzinstruments/DrumGizmo.DRSKit",
        76,
        &[("Room Kit", KIT_AIRY)],
    ),
    karoryfer(
        "Body Percussion",
        "https://raw.githubusercontent.com/sfzinstruments/body_percussion/4ac9d8966679c648b62fa10a188179e186b97f24/",
        "Body percussion samples from sfzinstruments, CC0 1.0: https://github.com/sfzinstruments/body_percussion",
        17,
        &[("Body Percussion", KIT_BODY)],
    ),
];

/// The pack a kit comes from.
pub fn pack_of(kit: &str) -> Option<&'static SamplePack> {
    PACKS.iter().find(|p| p.kits.iter().any(|k| k.0 == kit))
}

/// The pads of a kit.
pub fn table(kit: &str) -> &'static [RealPad] {
    PACKS.iter().flat_map(|p| p.kits.iter()).find(|k| k.0 == kit).map(|k| k.1).unwrap_or(&[])
}

/// Where a pack's samples live: Documents/Krabton/Library.
pub fn dir(pack: &SamplePack) -> PathBuf {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default();
    home.join("Documents").join("Krabton").join("Library").join(pack.folder)
}

/// A path for a URL: spaces and the like escaped.
fn url_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            ' ' => out.push_str("%20"),
            ',' => out.push_str("%2C"),
            '#' => out.push_str("%23"),
            '%' => out.push_str("%25"),
            '+' => out.push_str("%2B"),
            '?' => out.push_str("%3F"),
            '\'' => out.push_str("%27"),
            c => out.push(c),
        }
    }
    out
}

/// The path of a pad's file (or hit) in the source.
fn src_path(p: &RealPad, f: &str) -> String {
    if p.dir.is_empty() {
        f.to_string()
    } else {
        format!("{}/{}", p.dir, f)
    }
}

/// Every file of a pack to download (each once): its path in the source.
/// A hit recorded with several microphones has a file per microphone.
fn sources(pack: &SamplePack) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (_, pads) in pack.kits {
        for p in pads.iter() {
            for f in p.layers.iter().flat_map(|l| l.iter()) {
                let path = src_path(p, f);
                let paths: Vec<String> = if p.mics.is_empty() { vec![path] } else { p.mics.iter().map(|m| format!("{path}{}", m.0)).collect() };
                for path in paths {
                    if !out.contains(&path) {
                        out.push(path);
                    }
                }
            }
        }
    }
    out
}

/// The stereo mix of a hit recorded with several microphones.
fn mix_name(stem: &str) -> String {
    format!("{}.mix.wav", stem.replace(['/', '\\'], "_"))
}

/// The name a file of the pack gets on this computer: its own name, or
/// its path made flat where two folders hold files of the same name.
fn local_name(all: &[String], path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    if all.iter().filter(|p| p.rsplit('/').next() == Some(name)).count() > 1 {
        path.replace(['/', '\\'], "_")
    } else {
        name.to_string()
    }
}

/// Every file to download: (url, file name here).
pub fn files(pack: &SamplePack) -> Vec<(String, String)> {
    let all = sources(pack);
    all.iter().map(|p| (format!("{}{}", pack.base, url_path(p)), local_name(&all, p))).collect()
}

/// The files a pack needs here to play: its recordings, and the mixes of
/// the hits recorded with several microphones.
fn needed(pack: &SamplePack) -> Vec<String> {
    let all = sources(pack);
    let mut out = Vec::new();
    for (_, pads) in pack.kits {
        for p in pads.iter() {
            for f in p.layers.iter().flat_map(|l| l.iter()) {
                let path = src_path(p, f);
                out.push(if p.mics.is_empty() { local_name(&all, &path) } else { mix_name(&path) });
            }
        }
    }
    out
}

/// True when every sample of the pack is on this computer.
pub fn installed(pack: &SamplePack) -> bool {
    let d = dir(pack);
    needed(pack).iter().all(|n| d.join(n).is_file())
}

/// Mixes the hits recorded with several microphones to stereo (each
/// microphone panned and set as the kit's own mix has it; every pad's
/// loudest hit at -3 dBFS), then removes the microphones' files.
pub fn build_mixes(pack: &SamplePack) -> Result<(), String> {
    build_mixes_in(pack, &dir(pack))
}

fn build_mixes_in(pack: &SamplePack, d: &std::path::Path) -> Result<(), String> {
    use crate::sample::SampleData;
    let all = sources(pack);
    let mut used: Vec<std::path::PathBuf> = Vec::new();
    for (_, pads) in pack.kits {
        for p in pads.iter().filter(|p| !p.mics.is_empty()) {
            let stems: Vec<String> = p.layers.iter().flat_map(|l| l.iter()).map(|f| src_path(p, f)).collect();
            for stem in &stems {
                for m in p.mics {
                    used.push(d.join(local_name(&all, &format!("{stem}{}", m.0))));
                }
            }
            if stems.iter().all(|s| d.join(mix_name(s)).is_file()) {
                continue;
            }
            let mut hits: Vec<(u32, Vec<f32>, Vec<f32>)> = Vec::new();
            for stem in &stems {
                let (mut l, mut r, mut rate) = (Vec::<f32>::new(), Vec::<f32>::new(), 44100);
                for &(suffix, pan, gain_db) in p.mics {
                    let path = d.join(local_name(&all, &format!("{stem}{suffix}")));
                    let s = SampleData::load(&path)?;
                    rate = s.sample_rate;
                    let g = 10f32.powf(gain_db / 20.0);
                    // Equal power: the middle at full level on both sides.
                    let a = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
                    let (gl, gr) = (a.cos() * std::f32::consts::SQRT_2 * g, a.sin() * std::f32::consts::SQRT_2 * g);
                    if l.len() < s.frames() {
                        l.resize(s.frames(), 0.0);
                        r.resize(s.frames(), 0.0);
                    }
                    for i in 0..s.frames() {
                        // A mono microphone fills both sides of its own pan.
                        let (x, y) = (s.left[i], s.right[i]);
                        l[i] += x * gl;
                        r[i] += y * gr;
                    }
                }
                hits.push((rate, l, r));
            }
            let peak = hits.iter().flat_map(|h| h.1.iter().chain(h.2.iter())).fold(0.0f32, |m, v| m.max(v.abs()));
            let scale = if peak > 1e-6 { 0.708 / peak } else { 1.0 };
            for (stem, (rate, l, r)) in stems.iter().zip(hits) {
                let spec = hound::WavSpec { channels: 2, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
                let out = d.join(mix_name(stem));
                let part = out.with_extension("part");
                let mut w = hound::WavWriter::create(&part, spec).map_err(|e| format!("{}: {e}", part.display()))?;
                for (a, b) in l.iter().zip(r.iter()) {
                    for v in [a, b] {
                        w.write_sample((v * scale * 32767.0).round().clamp(-32768.0, 32767.0) as i16).map_err(|e| e.to_string())?;
                    }
                }
                w.finalize().map_err(|e| e.to_string())?;
                std::fs::rename(&part, &out).map_err(|e| e.to_string())?;
            }
        }
    }
    for f in used {
        let _ = std::fs::remove_file(f);
    }
    Ok(())
}

/// The pads of a kit: its recordings when they are here, else the stand-ins.
pub fn kit(name: &str) -> Vec<DrumPad> {
    let Some(pack) = pack_of(name) else { return Vec::new() };
    let d = dir(pack);
    let all = sources(pack);
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
                .map(|l| {
                    l.iter()
                        .filter_map(|f| {
                            let path = src_path(rp, f);
                            let here = if rp.mics.is_empty() { local_name(&all, &path) } else { mix_name(&path) };
                            SampleRef::load(&d.join(here)).ok()
                        })
                        .collect::<Vec<_>>()
                })
                .filter(|l: &Vec<SampleRef>| !l.is_empty())
                .collect();
            if pack.normalize {
                let peak = p.layers.iter().flatten().flat_map(|s| s.0.left.iter().chain(s.0.right.iter())).map(|v| v.abs()).fold(0.0f32, f32::max);
                if peak > 1e-4 {
                    p.trim_db = (20.0 * (0.708 / peak).log10()).clamp(-12.0, 20.0);
                }
            }
            p
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kit_is_complete_and_its_files_listed_once() {
        for pack in PACKS {
            assert!(pack.base.ends_with('/') && pack.mb > 0, "{}", pack.folder);
            for (name, pads) in pack.kits {
                assert!((8..=16).contains(&pads.len()), "{name}: {} pads", pads.len());
                let mut notes: Vec<u8> = pads.iter().map(|p| p.note).collect();
                notes.sort();
                notes.dedup();
                assert_eq!(notes.len(), pads.len(), "{name}: one pad per note");
                assert!(pads.iter().all(|p| !p.layers.is_empty() && p.layers.iter().all(|l| !l.is_empty())), "{name}");
                assert_eq!(pack_of(name).map(|p| p.folder), Some(pack.folder));
            }
            let f = files(pack);
            let mut names: Vec<&String> = f.iter().map(|x| &x.1).collect();
            names.sort();
            names.dedup();
            assert_eq!(names.len(), f.len(), "{}: local names clash", pack.folder);
            assert!(f.iter().all(|(u, _)| u.starts_with(pack.base) && !u.contains(' ') && !u.contains(',')), "{}", pack.folder);
        }
    }

    #[test]
    fn hits_of_several_microphones_are_mixed_to_stereo_once() {
        // A pad of two hits, each a left and a right microphone.
        static PADS: &[RealPad] = &[RealPad { mics: &[("-L.wav", -1.0, 0.0), ("-R.wav", 1.0, 0.0)], ..rp("Snare", 38, DrumVoice::AcSnare, 0.0, "s", &[&["1"], &["2"]]) }];
        static PACK: SamplePack = SamplePack { folder: "test", base: "https://example.org/", credit: "", mb: 1, kits: &[("Test", PADS)], normalize: false };
        let d = std::env::temp_dir().join(format!("krabton_mix_test_{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let write = |name: &str, v: f32| {
            let spec = hound::WavSpec { channels: 1, sample_rate: 48000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
            let mut w = hound::WavWriter::create(d.join(name), spec).unwrap();
            for _ in 0..100 {
                w.write_sample((v * 32767.0) as i16).unwrap();
            }
            w.finalize().unwrap();
        };
        let all = sources(&PACK);
        assert_eq!(all, vec!["s/1-L.wav", "s/1-R.wav", "s/2-L.wav", "s/2-R.wav"]);
        // The soft hit: left louder than right; the loud hit twice as loud.
        for (path, v) in [("s/1-L.wav", 0.2), ("s/1-R.wav", 0.1), ("s/2-L.wav", 0.4), ("s/2-R.wav", 0.2)] {
            write(&local_name(&all, path), v);
        }
        build_mixes_in(&PACK, &d).unwrap();
        let soft = crate::sample::SampleData::load(&d.join(mix_name("s/1"))).unwrap();
        let loud = crate::sample::SampleData::load(&d.join(mix_name("s/2"))).unwrap();
        // Panned hard: the left mic only on the left; the loudest hit at -3 dBFS.
        assert!((loud.left[10] - 0.708).abs() < 0.01, "{}", loud.left[10]);
        assert!((loud.right[10] - 0.354).abs() < 0.01, "{}", loud.right[10]);
        assert!((soft.left[10] - 0.354).abs() < 0.01, "{}", soft.left[10]);
        // The microphones' files are gone, the mixes stay.
        assert!(!d.join(local_name(&all, "s/1-L.wav")).exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn files_of_the_same_name_in_two_folders_stay_apart() {
        let all = vec!["a/hit.wav".to_string(), "b/hit.wav".to_string(), "b/other.wav".to_string()];
        assert_eq!(local_name(&all, "a/hit.wav"), "a_hit.wav");
        assert_eq!(local_name(&all, "b/other.wav"), "other.wav");
        assert_eq!(url_path("Snare Drum, Modern 1/x#1.wav"), "Snare%20Drum%2C%20Modern%201/x%231.wav");
    }

    #[test]
    fn without_the_samples_the_stand_ins_play() {
        for pack in PACKS {
            if installed(pack) {
                // Here: every pad has its recordings, and they are not silent.
                for (name, pads) in pack.kits {
                    let k = kit(name);
                    for (p, rp) in k.iter().zip(pads.iter()) {
                        assert_eq!(p.layers.len(), rp.layers.len(), "{name}: {}", p.name);
                        let peak = p.layers.iter().flatten().map(|s| s.left.iter().fold(0.0f32, |m, v| m.max(v.abs()))).fold(0.0f32, f32::max);
                        assert!(peak > 0.01, "{name}: {} is silent", p.name);
                    }
                }
                continue;
            }
            for (name, pads) in pack.kits {
                let k = kit(name);
                assert_eq!(k.len(), pads.len());
                assert!(k.iter().all(|p| p.layers.is_empty()), "{name}");
            }
        }
    }
}
